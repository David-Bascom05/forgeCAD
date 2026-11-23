//! An assembly's placements: where, and how often, each solid of a file stands.
//!
//! A part's geometry lives in its representations: a SHAPE_REPRESENTATION and the
//! ADVANCED_BREP_SHAPE_REPRESENTATION it is related to (a plain SHAPE_REPRESENTATION_RELATIONSHIP:
//! one frame). A part is placed in its parent in one of two ways (ISO 10303-214 and the CAx-IF's
//! recommended practices for assemblies):
//!
//! - a CONTEXT_DEPENDENT_SHAPE_REPRESENTATION: a relationship between the child's and the parent's
//!   representations, with an ITEM_DEFINED_TRANSFORMATION from a placement in the child to one in
//!   the parent, for the NEXT_ASSEMBLY_USAGE_OCCURRENCE (parent and child product definitions) its
//!   product definition shape names;
//! - a MAPPED_ITEM among the parent's items: a REPRESENTATION_MAP of the child's representation
//!   from its origin onto a target placement.
//!
//! Each is an edge, parent to child, with a rigid frame. Walking the edges from the top (the
//! representations nothing places) gives every solid its placements in the world. An edge that
//! can't be followed is never guessed at: the solids under it are refused, by name.

use crate::part21::{File, Value};
use forge_math::{Frame, Point3, Vec3};
use std::collections::{BTreeMap, BTreeSet};

/// How many placements one solid may get: past this a file is refused, not read in part.
pub const MAX_INSTANCES: usize = 100_000;
/// How deep an assembly may nest.
const MAX_DEPTH: usize = 64;

/// Where each solid stands: its placements, in order (the file's own order of the edges walked).
pub struct Placements {
    /// Solids with placements; a solid not here stands once where it is.
    pub of: BTreeMap<u32, Vec<Frame>>,
    /// Solids that can't be placed, and why.
    pub refused: BTreeMap<u32, (u32, String)>,
    /// Placements given and not applied: none, unless something of the structure is not
    /// followed (an occurrence with no placement, a placement no solid is under).
    pub unresolved: usize,
}

struct Edge {
    parent: usize,
    child: usize,
    frame: Option<Frame>,
    /// the entity that placed it, for a refusal
    by: u32,
    why: &'static str,
}

fn find(up: &mut [usize], mut x: usize) -> usize {
    while up[x] != x {
        up[x] = up[up[x]];
        x = up[x];
    }
    x
}

fn refs(v: Option<&Value>) -> Option<u32> {
    match v {
        Some(Value::Ref(r)) => Some(*r),
        _ => None,
    }
}

/// The rigid frame taking `from`'s local coordinates to `to`'s, as a frame in `to`'s space:
/// a point at local `p` of `from` lands at local `p` of `to`.
fn mapping(from: &Frame, to: &Frame) -> Frame {
    to.compose(&from.inverse())
}

/// Near enough to the identity that applying it would only add rounding: Onshape wraps every
/// part in a placement onto itself.
pub fn is_identity(f: &Frame) -> bool {
    let near = |x: Vec3, y: Vec3| x.dot(y) >= 1.0 - 1e-12;
    f.origin.distance(Point3::ORIGIN) <= 1e-9
        && near(f.x().into(), Vec3::X)
        && near(f.y().into(), Vec3::Y)
        && near(f.z().into(), Vec3::Z)
}

/// The placements of every solid in `solids`. `placement` reads an AXIS2_PLACEMENT_3D.
pub fn placements(f: &File<'_>, solids: &[u32], placement: impl Fn(u32) -> Option<Frame>) -> Placements {
    let mut out = Placements {
        of: BTreeMap::new(),
        refused: BTreeMap::new(),
        unresolved: 0,
    };
    // representations, and what each lists
    let mut rep_index: BTreeMap<u32, usize> = BTreeMap::new();
    let mut reps: Vec<(u32, Vec<u32>)> = Vec::new();
    let (mut cdsr, mut mapped, mut plain, mut nauo, mut pds, mut sdr) =
        (vec![], vec![], vec![], vec![], vec![], vec![]);
    for &(id, at, n) in &f.ids {
        let recs = &f.records[at as usize..(at + n) as usize];
        let names: Vec<&str> = recs.iter().map(|r| f.name(r)).collect();
        let has = |k: &str| names.contains(&k);
        for r in recs {
            let name = f.name(r);
            let a = f.args(r);
            if (name.ends_with("_REPRESENTATION") || name == "SHAPE_REPRESENTATION")
                && !name.ends_with("_SHAPE_REPRESENTATION_RELATIONSHIP")
                && let Some(Value::List(l, ln)) = a.get(1)
            {
                let items = f.run(*l, *ln).iter().filter_map(|v| refs(Some(v))).collect();
                rep_index.entry(id).or_insert_with(|| {
                    reps.push((id, items));
                    reps.len() - 1
                });
            }
            match name {
                "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" => cdsr.push((id, refs(a.first()), refs(a.get(1)))),
                "MAPPED_ITEM" => mapped.push((id, refs(a.get(1)), refs(a.get(2)))),
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => nauo.push((id, refs(a.get(3)), refs(a.get(4)))),
                "PRODUCT_DEFINITION_SHAPE" => pds.push((id, refs(a.get(2)))),
                "SHAPE_DEFINITION_REPRESENTATION" => sdr.push((refs(a.first()), refs(a.get(1)))),
                _ => {}
            }
        }
        // a relationship with no transformation: the two representations share a frame
        let rel = has("REPRESENTATION_RELATIONSHIP") || has("SHAPE_REPRESENTATION_RELATIONSHIP");
        if rel
            && !has("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
            && let Some(r) = recs.iter().find(|r| f.args(r).len() >= 4)
        {
            let a = f.args(r);
            plain.push((refs(a.get(2)), refs(a.get(3))));
        }
    }
    if cdsr.is_empty() && mapped.is_empty() && nauo.is_empty() {
        return out;
    }
    // one frame: representations related plainly
    let mut up: Vec<usize> = (0..reps.len()).collect();
    for (a, b) in plain {
        if let (Some(a), Some(b)) = (
            a.and_then(|x| rep_index.get(&x)),
            b.and_then(|x| rep_index.get(&x)),
        ) {
            let (x, y) = (find(&mut up, *a), find(&mut up, *b));
            if x != y {
                up[x.max(y)] = x.min(y);
            }
        }
    }
    let class = |up: &mut Vec<usize>, r: u32| rep_index.get(&r).map(|&i| find(up, i));
    // a product definition's representation, through its shape
    let pds_of: BTreeMap<u32, Option<u32>> = pds.iter().copied().collect();
    let mut pd_class: BTreeMap<u32, usize> = BTreeMap::new();
    for (s, r) in sdr {
        if let (Some(Some(pd)), Some(r)) = (s.and_then(|s| pds_of.get(&s)), r)
            && let Some(c) = class(&mut up, r)
        {
            pd_class.entry(*pd).or_insert(c);
        }
    }
    let nauo_of: BTreeMap<u32, (Option<u32>, Option<u32>)> =
        nauo.iter().map(|&(id, a, b)| (id, (a, b))).collect();
    let mut placed_nauo = BTreeSet::new();
    let mut edges: Vec<Edge> = Vec::new();
    for (id, rel, shape) in cdsr {
        // the relationship: its two representations and its transformation
        let recs = rel.and_then(|r| f.records_of(r)).unwrap_or(&[]);
        let (mut r1, mut r2, mut op) = (None, None, None);
        for r in recs {
            let a = f.args(r);
            match f.name(r) {
                "REPRESENTATION_RELATIONSHIP" => (r1, r2) = (refs(a.get(2)), refs(a.get(3))),
                "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION" => op = refs(a.first()),
                _ => {}
            }
        }
        let occurrence = shape.and_then(|s| pds_of.get(&s).copied().flatten());
        if let Some(o) = occurrence {
            placed_nauo.insert(o);
        }
        let (c1, c2) = (
            r1.and_then(|r| class(&mut up, r)),
            r2.and_then(|r| class(&mut up, r)),
        );
        let (Some(c1), Some(c2)) = (c1, c2) else {
            out.unresolved += 1;
            continue;
        };
        // which side is the child: the occurrence's related product, else the standard's rep_1
        let child_pd = occurrence.and_then(|o| nauo_of.get(&o)).and_then(|x| x.1);
        let swapped = child_pd
            .and_then(|pd| pd_class.get(&pd))
            .is_some_and(|&c| c == c2 && c != c1);
        let (parent, child) = if swapped { (c1, c2) } else { (c2, c1) };
        let t = op.and_then(|op| {
            let recs = f.records_of(op)?;
            let r = recs.iter().find(|r| f.name(r) == "ITEM_DEFINED_TRANSFORMATION")?;
            let a = f.args(r);
            let (i1, i2) = (placement(refs(a.get(2))?)?, placement(refs(a.get(3))?)?);
            // item 1 is in rep_1, item 2 in rep_2: child to parent
            Some(if swapped {
                mapping(&i2, &i1)
            } else {
                mapping(&i1, &i2)
            })
        });
        edges.push(Edge {
            parent,
            child,
            frame: t,
            by: id,
            why: "an assembly placement whose transformation is not an ITEM_DEFINED_TRANSFORMATION of two placements read",
        });
    }
    // which representation lists each mapped item
    let mut lister: BTreeMap<u32, usize> = BTreeMap::new();
    for (i, (_, items)) in reps.iter().enumerate() {
        for &x in items {
            lister.entry(x).or_insert(i);
        }
    }
    for (id, source, target) in mapped {
        let parent = lister.get(&id).map(|&i| find(&mut up, i));
        let map = source
            .and_then(|s| f.records_of(s))
            .and_then(|r| r.iter().find(|r| f.name(r) == "REPRESENTATION_MAP"));
        let (origin, child) = match map {
            Some(r) => {
                let a = f.args(r);
                (refs(a.first()), refs(a.get(1)).and_then(|c| class(&mut up, c)))
            }
            None => (None, None),
        };
        // a mapped item no representation lists places nothing
        let Some(parent) = parent else { continue };
        let Some(child) = child else {
            out.unresolved += 1;
            continue;
        };
        let t = (|| Some(mapping(&placement(origin?)?, &placement(target?)?)))();
        edges.push(Edge {
            parent,
            child,
            frame: t,
            by: id,
            why: "a mapped item whose origin or target is not a placement read",
        });
    }
    // an occurrence nothing places, whose child nothing else places either: where it stands
    // is not known
    let reached: BTreeSet<usize> = edges.iter().map(|e| e.child).collect();
    for (id, _, child) in &nauo {
        if !placed_nauo.contains(id) {
            if child
                .and_then(|pd| pd_class.get(&pd))
                .is_some_and(|c| reached.contains(c))
            {
                continue;
            }
            out.unresolved += 1;
            if let Some(&c) = child.and_then(|pd| pd_class.get(&pd)) {
                edges.push(Edge {
                    parent: usize::MAX,
                    child: c,
                    frame: None,
                    by: *id,
                    why: "an assembly occurrence with no placement",
                });
            }
        }
    }
    // walk from the top: every class nothing places
    let n = reps.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut placed = vec![false; n];
    let mut bad: Vec<Option<(u32, &'static str)>> = vec![None; n];
    for (k, e) in edges.iter().enumerate() {
        placed[e.child] = true;
        match (e.frame, e.parent) {
            (Some(_), p) if p != usize::MAX => children[p].push(k),
            _ => {
                bad[e.child].get_or_insert((e.by, e.why));
            }
        }
    }
    let mut at: Vec<Vec<Frame>> = vec![Vec::new(); n];
    let mut over = None;
    let mut stack: Vec<(usize, Frame, usize)> = (0..n)
        .filter(|&c| find(&mut up, c) == c && !placed[c])
        .map(|c| (c, Frame::WORLD, 0))
        .collect();
    stack.reverse();
    while let Some((c, w, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            over = Some("an assembly nested past 64 levels (or placing itself)");
            break;
        }
        at[c].push(w);
        if at[c].len() > MAX_INSTANCES {
            over = Some("an assembly of more than 100 000 placements of one part");
            break;
        }
        for &k in children[c].iter().rev() {
            let e = &edges[k];
            stack.push((e.child, w.compose(&e.frame.expect("placed")), depth + 1));
        }
    }
    // a placement that leads nowhere: a part placed in a loop of its own
    let solid_class: Vec<(u32, BTreeSet<usize>)> = solids
        .iter()
        .map(|&s| {
            let mut cs = BTreeSet::new();
            for (i, (_, items)) in reps.iter().enumerate() {
                if items.contains(&s) {
                    cs.insert(find(&mut up, i));
                }
            }
            (s, cs)
        })
        .collect();
    for (s, cs) in solid_class {
        if let Some(why) = over {
            out.refused.insert(s, (0, why.into()));
            continue;
        }
        if cs.is_empty() {
            continue;
        }
        if let Some(&(by, why)) = cs.iter().find_map(|&c| bad[c].as_ref()) {
            out.refused.insert(s, (by, why.into()));
            continue;
        }
        // a solid stands where the structure places it; one it doesn't reach stands once, where
        // it is, however many representations list it (a second view of the part, a
        // tessellated twin, as some systems write)
        let reached: Vec<usize> = cs.iter().copied().filter(|&c| placed[c]).collect();
        if reached.is_empty() {
            continue;
        }
        let frames: Vec<Frame> = reached.iter().flat_map(|&c| at[c].iter().copied()).collect();
        if frames.is_empty() {
            out.refused
                .insert(s, (0, "a part placed only in a loop of placements".into()));
            continue;
        }
        if frames.len() == 1 && is_identity(&frames[0]) {
            continue;
        }
        out.of.insert(s, frames);
    }
    out
}

/// `b` placed by the rigid frame `f`: every point, curve and surface moved, the parameters kept
/// (a rigid frame keeps them). The `k`th placement of a part past the first has its names salted
/// with `k`, so no two bodies of a file share one.
pub fn placed(b: &forge_topo::Body, f: &Frame, k: usize) -> forge_topo::Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
    let mut out = b.clone();
    let rename = |n: forge_topo::Name| if k == 0 { n } else { n.with(k as u64) };
    for v in &mut out.vertices {
        v.point = f.point_to_world(v.point);
        v.name = rename(v.name);
    }
    for e in &mut out.edges {
        e.curve = e.curve.transformed(f);
        e.name = rename(e.name);
        e.bbox = Default::default();
    }
    for face in &mut out.faces {
        face.surface = face.surface.transformed(f);
        face.name = rename(face.name);
        face.bbox = Default::default();
        face.query = Default::default();
    }
    // a file's edge may lie off its faces by just its tolerance: moved, the rounding of the move
    // must not take it past (a few ulps of the coordinates)
    let reach = out
        .vertices
        .iter()
        .map(|v| v.point.to_vec().abs_max())
        .fold(0.0, f64::max);
    let slack = 1e-12 * (1.0 + reach);
    for e in &mut out.edges {
        if e.tol > forge_math::tol::LINEAR {
            e.tol += slack;
        }
    }
    for v in &mut out.vertices {
        if v.tol > forge_math::tol::LINEAR {
            v.tol += slack;
        }
    }
    out.hulls = Default::default();
    out.joins = out.joins.renamed(rename);
    out.split = out.split.iter().map(|&n| rename(n)).collect();
    out
}

/// A solid's mass properties, placed by `f`.
pub fn placed_mass(m: &forge_topo::mass::MassProps, f: &Frame) -> forge_topo::mass::MassProps {
    // I' = R I R^T, R's columns the frame's axes: entry (i, j) is axis_i' I axis_j', where
    // axis_i' is row i of R
    let (x, y, z) = (Vec3::from(f.x()), Vec3::from(f.y()), Vec3::from(f.z()));
    let row = |i: usize| Vec3::new([x.x, x.y, x.z][i], [y.x, y.y, y.z][i], [z.x, z.y, z.z][i]);
    let times = |v: Vec3| {
        let c = m.inertia.map(|r| r[0] * v.x + r[1] * v.y + r[2] * v.z);
        Vec3::new(c[0], c[1], c[2])
    };
    let inertia = [0, 1, 2].map(|i| [0, 1, 2].map(|j| row(i).dot(times(row(j)))));
    forge_topo::mass::MassProps {
        centroid: f.point_to_world(m.centroid),
        inertia,
        axes: m.axes.map(|a| f.vec_to_world(a)),
        ..m.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::{Point2, UnitVec3};

    /// A body placed by a turned, moved frame measures as its moved integrals say.
    #[test]
    fn a_placed_body_and_its_placed_mass_agree() {
        let p = forge_ops::Profile {
            plane: Frame::WORLD,
            loops: vec![forge_ops::Profile::rectangle(Point2::new(1.0, 2.0), 4.0, 2.0, 0)],
        };
        let b = forge_ops::extrude(&p, 7.0, 1).unwrap();
        let z = UnitVec3::new_unchecked(Vec3::new(1.0, 2.0, 2.0) / 3.0);
        let f = Frame::from_zx(Point3::new(10.0, -3.0, 5.0), z, Vec3::new(0.0, 1.0, -1.0)).unwrap();
        let m = forge_topo::mass::mass(&b).unwrap();
        let moved = placed(&b, &f, 1);
        assert!(forge_topo::check(&moved).is_empty());
        let want = forge_topo::mass::mass(&moved).unwrap();
        let got = placed_mass(&m, &f);
        assert!((got.volume - want.volume).abs() < 1e-9 * want.volume);
        assert!(got.centroid.distance(want.centroid) < 1e-9);
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (got.inertia[i][j] - want.inertia[i][j]).abs() < 1e-7,
                    "{:?} {:?}",
                    got.inertia,
                    want.inertia
                );
            }
        }
        assert!(moved.faces.iter().zip(&b.faces).all(|(x, y)| x.name != y.name));
    }
}
