//! What a UI shows on a finished part: which feature made each face, and each feature's main number
//! with the real axis it moves along, so a drag there is in millimetres on the model.

use crate::model::Model;
use crate::tools::Target;
use forge_doc::{Body, ItemId};
use forge_math::{Point3, Vec3};
use forge_model::{Extent, Feature, Hole};
use forge_ops::{ChamferSize, Radius};
use std::collections::HashMap;

/// A number on the model and how the mouse moves it.
#[derive(Clone, Debug, PartialEq)]
pub struct Handle {
    pub item: ItemId,
    pub label: &'static str,
    /// As written: a number, or an expression, which a drag can't change.
    pub text: String,
    pub value: f64,
    pub at: Target,
    pub draggable: bool,
    /// Where its arrow starts, and which way it points (unit), on the model.
    pub origin: [f64; 3],
    pub dir: [f64; 3],
    /// How much the value changes per millimetre along `dir`.
    pub per_mm: f64,
}

/// One state of the part, for a UI that doesn't hold it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// By the body's face index: the item that made the face.
    pub face_item: Vec<Option<ItemId>>,
    pub handles: Vec<Handle>,
    /// By the body's face and edge index: how a part file names each (`@f..`, `@e..`).
    pub face_refs: Vec<String>,
    pub edge_refs: Vec<String>,
    pub vertex_refs: Vec<String>,
    /// By the body's edge index: points along it.
    pub edge_points: Vec<Vec<[f32; 3]>>,
    /// By the body's face index: its edges, by the body's edge index.
    pub face_edges: Vec<Vec<u32>>,
    /// By the body's face index: its radius, if it's a cylinder.
    pub face_radius: Vec<Option<f64>>,
    /// By the body's face index: does it face its own axis, as a bore's wall does?
    pub face_bore: Vec<bool>,
    pub sketches: Vec<SketchShape>,
}

/// A sketch line as it stands in the model, in 3D.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SketchShape {
    pub item: Option<ItemId>,
    pub plane: forge_math::Frame,
    /// Each curve as points, and whether it's construction.
    pub curves: Vec<(Vec<[f64; 3]>, bool)>,
    pub points: Vec<[f64; 3]>,
    /// Each of `curves`' names.
    pub curve_names: Vec<String>,
    /// The places a line reading it can name, as it would name them.
    pub named: Vec<(String, [f64; 3])>,
    pub regions: Vec<RegionShape>,
}

/// A closed region of a sketch, in 3D.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegionShape {
    /// What a line reading the sketch names to mean this one.
    pub through: String,
    pub loops: Vec<Vec<[f64; 3]>>,
}

fn sketch_shapes(model: &Model, n: usize) -> Vec<SketchShape> {
    (0..n.min(model.features.len()))
        .filter_map(|i| {
            let forge_model::Feature::Sketch(s) = &model.features[i] else {
                return None;
            };
            let up = |p: forge_math::Point2| {
                let w = s.plane.plane_point(p);
                [w.x, w.y, w.z]
            };
            let (curves, points) = forge_doc::sketch::drawing(s);
            Some(SketchShape {
                item: Some(ItemId(model.lines[i].id)),
                plane: s.plane,
                curves: curves
                    .into_iter()
                    .map(|(c, faint)| (c.into_iter().map(up).collect(), faint))
                    .collect(),
                points: points.into_iter().map(up).collect(),
                curve_names: forge_doc::sketch::curve_names(s),
                named: forge_doc::sketch::named_points(s)
                    .into_iter()
                    .map(|(n, p)| (n, up(p)))
                    .collect(),
                regions: forge_doc::sketch::regions(s)
                    .into_iter()
                    .map(|r| RegionShape {
                        through: r.through,
                        loops: r
                            .loops
                            .into_iter()
                            .map(|l| l.into_iter().map(up).collect())
                            .collect(),
                    })
                    .collect(),
            })
        })
        .collect()
}

impl Facts {
    pub fn handle(&self, item: ItemId) -> Option<&Handle> {
        self.handles.iter().find(|h| h.item == item)
    }

    pub fn faces_of(&self, item: ItemId) -> Vec<u32> {
        (0..self.face_item.len())
            .filter(|&k| self.face_item[k] == Some(item))
            .map(|k| k as u32)
            .collect()
    }
}

/// A number's word and value, its arrow's start and way, its value per mm.
type Found = (&'static str, f64, [f64; 3], [f64; 3], f64);

#[derive(Clone, Copy)]
struct Patch {
    centre: [f64; 3],
    normal: [f64; 3],
    area: f64,
    /// Its normals spread apart moving along it: a bulge, not a hollow. Zero if flat.
    bulge: f64,
}

fn sub(a: Point3, b: Point3) -> [f64; 3] {
    [a.x - b.x, a.y - b.y, a.z - b.z]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 0.0 { a.map(|v| v / l) } else { [0.0; 3] }
}

fn vec3(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn point(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

fn plus(a: [f64; 3], b: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

fn patches(model: &mut Model) -> HashMap<u32, Patch> {
    let mut out = HashMap::new();
    for (f, mesh) in model.faces() {
        let Some(m) = mesh else { continue };
        let tm = &m.mesh;
        let (mut area, mut centre, mut normal) = (0.0, [0.0; 3], [0.0; 3]);
        let mut tris: Vec<([f64; 3], [f64; 3])> = Vec::with_capacity(tm.triangles.len());
        for t in &tm.triangles {
            let [a, b, c] = t.map(|i| tm.positions[i as usize]);
            let n = cross(sub(b, a), sub(c, a));
            let twice = dot(n, n).sqrt();
            let mid = [
                (a.x + b.x + c.x) / 3.0,
                (a.y + b.y + c.y) / 3.0,
                (a.z + b.z + c.z) / 3.0,
            ];
            area += twice;
            centre = plus(centre, mid, twice);
            normal = plus(normal, n, 1.0);
            tris.push((mid, unit(n)));
        }
        if area <= 0.0 {
            continue;
        }
        let mut bulge = 0.0;
        // a sample of pairs is plenty
        let step = (tris.len() / 16).max(1);
        for p in tris.iter().step_by(step) {
            for q in tris.iter().step_by(step) {
                let dn = plus(p.1, q.1, -1.0);
                bulge += dot(dn, plus(p.0, q.0, -1.0));
            }
        }
        out.insert(
            f.index() as u32,
            Patch {
                centre: centre.map(|v| v / area),
                normal: unit(normal),
                area,
                bulge,
            },
        );
    }
    out
}

/// The main number's word in the feature's line, as written.
fn main_arg(args: &[forge_doc::Arg]) -> Option<usize> {
    args.iter().position(|a| {
        let t = a.text.as_str();
        !matches!(t, "add" | "cut" | "keep" | "new")
    })
}

/// A number: what changes it, whether a drag may, and what's written.
fn target_of(model: &Model, item: ItemId, k: usize) -> Option<(Target, bool, String)> {
    let doc = &model.doc;
    let Body::Feature { args, .. } = &doc.items.iter().find(|i| i.id == item)?.body else {
        return None;
    };
    let text = args.get(k)?.text.clone();
    if text.parse::<f64>().is_ok() {
        return Some((Target::Arg(item, k), true, text));
    }
    if let Some(p) = doc.param(&text)
        && let Some(Body::Param { expr, .. }) = doc.items.iter().find(|i| i.id == p).map(|i| &i.body)
    {
        return Some((Target::Param(p), expr.parse::<f64>().is_ok(), expr.clone()));
    }
    Some((Target::Arg(item, k), false, text))
}

/// Everything the UI needs after a commit.
pub fn facts(model: &mut Model) -> Facts {
    let makers = model.face_makers();
    let patches = patches(model);
    let built = model.built().min(model.features.len());
    let item_of = |i: usize, m: &Model| ItemId(m.lines[i].id);
    let face_item: Vec<Option<ItemId>> = makers
        .iter()
        .map(|m| m.filter(|&i| i < built).map(|i| item_of(i, model)))
        .collect();
    let mut handles = Vec::new();
    for i in 0..built {
        let item = item_of(i, model);
        let own: Vec<Patch> = face_item
            .iter()
            .enumerate()
            .filter(|(_, o)| **o == Some(item))
            .filter_map(|(k, _)| patches.get(&(k as u32)).copied())
            .collect();
        let biggest = own.iter().copied().max_by(|a, b| a.area.total_cmp(&b.area));
        let along_faces = |dir: [f64; 3], fallback: [f64; 3]| {
            own.iter()
                .filter(|p| dot(p.normal, dir).abs() > 0.9)
                .map(|p| p.centre)
                .max_by(|a, b| dot(*a, dir).total_cmp(&dot(*b, dir)))
                .unwrap_or(fallback)
        };
        // the surface moves with its number: in along the normal for what cuts into a bulge
        let on_surface = |label, value: f64, grows_out: Option<bool>| {
            let p = biggest?;
            let out = grows_out.unwrap_or(p.bulge < 0.0);
            Some((label, value, p.centre, p.normal, if out { 1.0 } else { -1.0 }))
        };
        let found: Option<Found> = match &model.features[i] {
            Feature::Extrude { profile, extent, .. } => {
                let z = vec3(profile.plane.z().into());
                let o = point(profile.plane.origin);
                match extent {
                    Extent::Distance(d) => {
                        let s = if *d < 0.0 { -1.0 } else { 1.0 };
                        let dir = z.map(|v| v * s);
                        Some(("Distance", *d, along_faces(dir, plus(o, dir, d.abs())), dir, s))
                    }
                    Extent::Symmetric(d) => {
                        Some(("Distance", *d, along_faces(z, plus(o, z, d / 2.0)), z, 2.0))
                    }
                    Extent::TwoSided { forward, .. } => {
                        Some(("Distance", *forward, along_faces(z, plus(o, z, *forward)), z, 1.0))
                    }
                    _ => None,
                }
            }
            Feature::Ball { frame, radius, .. } => {
                let x = vec3(frame.x().into());
                Some(("Radius", *radius, plus(point(frame.origin), x, *radius), x, 1.0))
            }
            Feature::Hole(h) => {
                let Hole { at, diameter, .. } = &**h;
                let x = vec3(at.x().into());
                Some((
                    "Diameter",
                    *diameter,
                    plus(point(at.origin), x, diameter / 2.0),
                    x,
                    2.0,
                ))
            }
            Feature::Fillet { edges, .. } => match edges.first() {
                Some((_, Radius::Constant(r))) => on_surface("Radius", *r, None),
                _ => None,
            },
            Feature::FaceFillet { radius, .. } => on_surface("Radius", *radius, None),
            Feature::Chamfer {
                size: ChamferSize::Equal(d),
                ..
            } => on_surface("Distance", *d, Some(false)),
            Feature::Shell { thickness, .. } => on_surface("Thickness", *thickness, Some(true)),
            Feature::Edit(forge_model::Edit::Offset { distance, .. }) => {
                on_surface("Distance", *distance, Some(true))
            }
            Feature::Edit(forge_model::Edit::Thicken { thickness, .. }) => {
                on_surface("Thickness", *thickness, Some(true))
            }
            _ => None,
        };
        let Some((label, value, origin, dir, per_mm)) = found else {
            continue;
        };
        let k = match model.doc.items.iter().find(|it| it.id == item).map(|it| &it.body) {
            Some(Body::Feature { args, .. }) => main_arg(args),
            _ => None,
        };
        let Some((at, draggable, text)) = k.and_then(|k| target_of(model, item, k)) else {
            continue;
        };
        handles.push(Handle {
            item,
            label,
            text,
            value,
            at,
            draggable,
            origin,
            dir,
            per_mm,
        });
    }
    let (face_refs, edge_refs) = refs(model.body());
    Facts {
        face_item,
        handles,
        face_refs,
        edge_refs,
        vertex_refs: vertex_refs(model.body()),
        edge_points: edge_points(model.body()),
        face_edges: face_edges(model.body()),
        face_radius: radii(model.body()),
        face_bore: bores(model.body()),
        sketches: sketch_shapes(model, model.features.len()),
    }
}

fn vertex_refs(body: Option<&forge_topo::Body>) -> Vec<String> {
    let Some(b) = body else { return Vec::new() };
    let names = crate::select::handles(b);
    b.vertex_ids()
        .map(|v| names.get(&b.vertex(v).name).cloned().unwrap_or_default())
        .collect()
}

fn edge_points(body: Option<&forge_topo::Body>) -> Vec<Vec<[f32; 3]>> {
    let Some(b) = body else { return Vec::new() };
    b.edge_ids()
        .map(|e| {
            let g = b.edge(e);
            let straight = matches!(g.curve, forge_geom::Curve::Line(_));
            let steps = if straight { 1 } else { 16 };
            (0..=steps)
                .map(|k| {
                    let t = g.range.lo + (g.range.hi - g.range.lo) * k as f64 / steps as f64;
                    let p = forge_geom::CurveGeom::eval(&g.curve, t);
                    [p.x as f32, p.y as f32, p.z as f32]
                })
                .collect()
        })
        .collect()
}

fn face_edges(body: Option<&forge_topo::Body>) -> Vec<Vec<u32>> {
    let Some(b) = body else { return Vec::new() };
    let at: std::collections::HashMap<_, u32> =
        b.edge_ids().enumerate().map(|(k, e)| (e, k as u32)).collect();
    b.face_ids()
        .map(|f| {
            crate::select::edges_of_face(b, f)
                .into_iter()
                .filter_map(|e| at.get(&e).copied())
                .collect()
        })
        .collect()
}

fn bores(body: Option<&forge_topo::Body>) -> Vec<bool> {
    let Some(b) = body else { return Vec::new() };
    b.face_ids().map(|f| !b.face(f).same_sense).collect()
}

fn radii(body: Option<&forge_topo::Body>) -> Vec<Option<f64>> {
    let Some(b) = body else { return Vec::new() };
    b.face_ids()
        .map(|f| match &b.face(f).surface {
            forge_geom::Surface::Cylinder(c) => Some(c.radius),
            _ => None,
        })
        .collect()
}

fn refs(body: Option<&forge_topo::Body>) -> (Vec<String>, Vec<String>) {
    let Some(b) = body else {
        return (Vec::new(), Vec::new());
    };
    let names = crate::select::handles(b);
    let of = |n: forge_topo::Name| names.get(&n).cloned().unwrap_or_default();
    (
        b.face_ids().map(|f| of(b.face(f).name)).collect(),
        b.edge_ids().map(|e| of(b.edge(e).name)).collect(),
    )
}

/// What's known of the part rolled back to `n` lines: its faces' makers, and no numbers on it.
pub fn facts_at(model: &Model, n: usize) -> Facts {
    let n = n.min(model.built());
    let face_item = model
        .face_makers_at(n)
        .iter()
        .map(|m| m.filter(|&i| i < n).map(|i| ItemId(model.lines[i].id)))
        .collect();
    let (face_refs, edge_refs) = refs(model.body_after(n));
    Facts {
        face_item,
        handles: Vec::new(),
        face_refs,
        edge_refs,
        vertex_refs: vertex_refs(model.body_after(n)),
        edge_points: edge_points(model.body_after(n)),
        face_edges: face_edges(model.body_after(n)),
        face_radius: radii(model.body_after(n)),
        face_bore: bores(model.body_after(n)),
        sketches: sketch_shapes(model, n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(f: &Facts, m: &Model, name: &str) -> Handle {
        let id = m.doc.find(name).unwrap();
        f.handle(id).unwrap().clone()
    }

    #[test]
    fn an_extrudes_arrow_stands_on_its_end_along_its_normal() {
        let mut m = Model::new("base: extrude 6 rect -30 -20 60 40\n");
        let f = facts(&mut m);
        let h = at(&f, &m, "base");
        assert_eq!((h.label, h.value, h.per_mm), ("Distance", 6.0, 1.0));
        assert_eq!(h.dir, [0.0, 0.0, 1.0]);
        assert!(
            (h.origin[2] - 6.0).abs() < 1e-9 && h.origin[0].abs() < 1e-9 && h.origin[1].abs() < 1e-9,
            "{:?}",
            h.origin
        );
        assert!(h.draggable);
        // each of its faces is its
        let id = m.doc.find("base").unwrap();
        assert_eq!(f.faces_of(id).len(), 6);
    }

    #[test]
    fn a_hole_widens_at_twice_the_drag_and_its_faces_are_its_own() {
        let mut m = Model::new("base: extrude 20 rect -50 -50 100 100\nh: hole 8 -xy at 0 0 20 thru\n");
        let f = facts(&mut m);
        let h = at(&f, &m, "h");
        assert_eq!((h.label, h.value, h.per_mm), ("Diameter", 8.0, 2.0));
        let r = (h.origin[0].powi(2) + h.origin[1].powi(2)).sqrt();
        assert!((r - 4.0).abs() < 1e-9, "{r}");
        assert!((h.origin[2] - 20.0).abs() < 1e-9);
        let id = m.doc.find("h").unwrap();
        assert_eq!(f.faces_of(id).len(), 1);
    }

    #[test]
    fn a_fillets_surface_goes_in_as_a_convex_radius_grows_and_out_for_a_concave_one() {
        let mut m = Model::new("base: extrude 20 rect 0 0 40 40\nf: fillet 3 (edges top along +x)\n");
        let f = facts(&mut m);
        assert!(m.error.is_none());
        let h = at(&f, &m, "f");
        assert_eq!(h.label, "Radius");
        assert_eq!(h.per_mm, -1.0, "the rounded corner bulges");
        let mut m = Model::new(
            "base: extrude 5 rect 0 0 40 40\nwall: extrude 20 rect 0 0 5 40\nf: fillet 2 (edges between base wall concave)\n",
        );
        let f = facts(&mut m);
        assert!(m.error.is_none(), "{:?}", m.error);
        assert_eq!(at(&f, &m, "f").per_mm, 1.0, "the hollow fills out");
    }

    #[test]
    fn a_parameter_is_what_a_drag_changes_and_an_expression_is_not_draggable() {
        let mut m =
            Model::new("t = 6\nbase: extrude t rect 0 0 40 40\nb2: extrude (t + 1) rect 50 0 40 40\n");
        let f = facts(&mut m);
        let h = at(&f, &m, "base");
        assert!(matches!(h.at, Target::Param(_)) && h.draggable);
        assert_eq!((h.value, h.text.as_str()), (6.0, "6"));
        let e = at(&f, &m, "b2");
        assert!(!e.draggable);
        assert_eq!(e.value, 7.0);
    }

    #[test]
    fn a_cut_points_along_its_own_normal_on_its_floor() {
        let mut m =
            Model::new("base: extrude 10 rect 0 0 40 40\nc: extrude cut 2 -xy at 20 20 10 circle 0 0 5\n");
        let f = facts(&mut m);
        let h = at(&f, &m, "c");
        assert_eq!(h.dir, [0.0, 0.0, -1.0]);
        assert!((h.origin[2] - 8.0).abs() < 1e-9, "{:?}", h.origin);
        assert_eq!(h.per_mm, 1.0);
    }
}
