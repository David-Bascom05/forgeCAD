//! Inventor's joint origins on a part: for each face, the points a joint or constraint can take
//! (the face's centre, a cylinder's axis, each edge's circle centre, midpoint and ends), each with
//! the Forge text that names it in the part, and the part's own declared anchors. The text is what
//! forge-lang evaluates (`pin.end.center`, `plane(at: edges(between: (..)).center, normal: ..)`);
//! a face no anchor names has no stable name and offers nothing but declared anchors.

use forge_lang::builtins::{edge_snap, faces_axis_middle, faces_centroid};
use forge_math::{Frame, Point3, UnitVec3};
use forge_topo::{Body, EdgeId, FaceId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    /// A planar face's centre: its plane.
    Face,
    /// A cylinder's or cone's axis, halfway along it.
    Axis,
    /// A circular edge's centre, z along the face's normal (or the cylinder's axis).
    Circle,
    /// A straight edge's midpoint.
    Mid,
    /// An edge's end.
    End,
    /// The part's own `anchor(..)`.
    Anchor,
}

impl SnapKind {
    /// What the snap is, as a constraint takes it.
    pub fn word(self) -> &'static str {
        match self {
            SnapKind::Face => "face",
            SnapKind::Axis => "axis",
            SnapKind::Circle => "circle centre",
            SnapKind::Mid => "edge midpoint",
            SnapKind::End => "edge end",
            SnapKind::Anchor => "anchor",
        }
    }

    /// It stands for a line (a revolute's axis, a concentric's).
    pub fn axial(self) -> bool {
        matches!(self, SnapKind::Axis | SnapKind::Circle)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snap {
    pub kind: SnapKind,
    /// Its text in the part: what follows `instance.` in the assembly.
    pub text: String,
    /// In the part's space: the origin, and z the normal or axis.
    pub frame: Frame,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartSnaps {
    /// Each face's snaps (by the mesh's face index), into `snaps`.
    pub faces: Vec<Vec<usize>>,
    /// Each edge's (by the mesh's edge index).
    pub edges: HashMap<u32, Vec<usize>>,
    pub snaps: Vec<Snap>,
    /// The declared anchors, into `snaps`.
    pub anchors: Vec<usize>,
}

fn same(a: &Frame, b: &Frame) -> bool {
    (a.origin - b.origin).length() < 1e-6 && a.z().get().dot(b.z().get()) > 1.0 - 1e-9
}

/// What `plane(at: P, normal: N)` makes (forge-lang's default x).
fn plane_at(p: Point3, n: UnitVec3) -> Frame {
    Frame::from_z(p, n)
}

/// A part file's snaps, from its run with anchors.
pub fn part_snaps(text: &str, dir: Option<std::path::PathBuf>) -> Result<PartSnaps, String> {
    let opts = forge_lang::Options {
        anchors: true,
        dir: dir.clone(),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(text, opts, Default::default());
    if let Some(e) = run.errors.first() {
        return Err(format!("{}: {}", e.kind(), e.text()));
    }
    let body = run.body.as_ref().ok_or("the part has no solid")?;
    // the mesh numbers the live faces and edges in order, as the compacted body does
    let body: Body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let a = run.anchors.ok_or("no anchors")?;
    let declared = forge_lang::asm::part_anchors(text, dir);
    snaps_of(&body, &a, &declared)
}

fn snaps_of(
    body: &Body,
    a: &forge_lang::anchors::Anchors,
    declared: &[(String, Frame)],
) -> Result<PartSnaps, String> {
    let mut out = PartSnaps::default();
    let fid: Vec<FaceId> = body.face_ids().collect();
    let text_of = |f: FaceId| a.faces.get(&body.face(f).name).cloned();
    // an edge's text, when it names that edge alone
    let mut by_pair: HashMap<(String, String), Vec<EdgeId>> = HashMap::new();
    let mut edge_text: HashMap<EdgeId, String> = HashMap::new();
    for e in body.edge_ids() {
        let Some(t) = a.edges.get(&body.edge(e).name) else {
            continue;
        };
        let fs = forge_agent::select::faces_of_edge(body, e);
        if let [x, y] = fs[..]
            && let (Some(tx), Some(ty)) = (text_of(x), text_of(y))
        {
            let key = if tx < ty { (tx, ty) } else { (ty, tx) };
            by_pair.entry(key).or_default().push(e);
        }
        edge_text.insert(e, t.clone());
    }
    let alone = |e: EdgeId| -> Option<&String> {
        let fs = forge_agent::select::faces_of_edge(body, e);
        let [x, y] = fs[..] else { return None };
        let (tx, ty) = (text_of(x)?, text_of(y)?);
        let key = if tx < ty { (tx, ty) } else { (ty, tx) };
        (by_pair.get(&key)?.len() == 1).then(|| edge_text.get(&e))?
    };
    let push = |out: &mut PartSnaps, s: Snap| -> usize {
        match out
            .snaps
            .iter()
            .position(|x| x.kind == s.kind && same(&x.frame, &s.frame))
        {
            Some(k) => k,
            None => {
                out.snaps.push(s);
                out.snaps.len() - 1
            }
        }
    };
    for &f in &fid {
        let mut here = vec![];
        let Some(t) = text_of(f) else {
            out.faces.push(here);
            continue;
        };
        let one = a.sizes.get(&t).is_none_or(|&n| n == 1);
        let plane = a.planes.get(&format!("{t}.plane")).copied();
        // its centre
        let normal = match plane {
            Some(p) => {
                if one && let Some(c) = faces_centroid(body, &[f], &p) {
                    here.push(push(
                        &mut out,
                        Snap {
                            kind: SnapKind::Face,
                            text: format!("{t}.center"),
                            frame: c,
                        },
                    ));
                }
                Some((p.z(), "normal"))
            }
            None => match faces_axis_middle(body, &[f]) {
                Some((o, d)) => {
                    if one {
                        here.push(push(
                            &mut out,
                            Snap {
                                kind: SnapKind::Axis,
                                text: format!("{t}.center"),
                                frame: Frame::from_z(o, d),
                            },
                        ));
                    }
                    Some((d, "axis"))
                }
                None => None,
            },
        };
        // its edges' centres, midpoints and ends, z out of the planar face beside the edge (a
        // rim's z out of its face, whichever side it's picked from), else along the axis
        if let Some((n, word)) = normal {
            for e in forge_agent::select::edges_of_face(body, f) {
                let Some(et) = alone(e) else { continue };
                let from = here.len();
                let flat = forge_agent::select::faces_of_edge(body, e)
                    .into_iter()
                    .find_map(|g| {
                        let tg = text_of(g)?;
                        let p = a.planes.get(&format!("{tg}.plane"))?;
                        Some((p.z(), tg))
                    });
                let (n, nt) = match flat {
                    Some((z, tg)) => (z, format!("{tg}.normal")),
                    None => (n, format!("{t}.{word}")),
                };
                let at = |what: &str| format!("plane(at: {et}.{what}, normal: {nt})");
                match edge_snap(body, e, "center") {
                    Err((o, _)) => here.push(push(
                        &mut out,
                        Snap {
                            kind: SnapKind::Circle,
                            text: at("center"),
                            frame: plane_at(o, n),
                        },
                    )),
                    Ok(p) => {
                        here.push(push(
                            &mut out,
                            Snap {
                                kind: SnapKind::Mid,
                                text: at("center"),
                                frame: plane_at(p, n),
                            },
                        ));
                        for w in ["start", "end"] {
                            let Ok(q) = edge_snap(body, e, w) else { continue };
                            here.push(push(
                                &mut out,
                                Snap {
                                    kind: SnapKind::End,
                                    text: at(w),
                                    frame: plane_at(q, n),
                                },
                            ));
                        }
                    }
                }
                let mine = out.edges.entry(e.index() as u32).or_default();
                for &k in &here[from..] {
                    if !mine.contains(&k) {
                        mine.push(k);
                    }
                }
            }
        }
        out.faces.push(here);
    }
    // the part's own anchors; a snap where one already is takes its name
    for (name, fr) in declared {
        if let Some(s) = out.snaps.iter_mut().find(|s| same(&s.frame, fr)) {
            s.text = name.clone();
        }
        out.snaps.push(Snap {
            kind: SnapKind::Anchor,
            text: name.clone(),
            frame: *fr,
        });
        out.anchors.push(out.snaps.len() - 1);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "shared r = 5
shared h = 20
shared w = 30
sketch s(on: xy) {
  c = circle((0, 0), r)
}
pin = extrude(s, h)
sketch b(on: pin.end) {
  o = rect(center: (0, 0), size: (w, w))
}
cap = extrude(b, r)
top = anchor(cap.end.center)
";

    #[test]
    fn every_snap_is_where_forge_lang_puts_its_text() {
        let got = part_snaps(PIN, None).unwrap();
        assert!(got.snaps.iter().any(|s| s.kind == SnapKind::Circle));
        assert!(got.snaps.iter().any(|s| s.kind == SnapKind::Mid));
        assert!(got.snaps.iter().any(|s| s.kind == SnapKind::Axis));
        // the cap's top centre is the declared anchor, by its name
        assert!(
            got.snaps
                .iter()
                .any(|s| s.kind == SnapKind::Face && s.text == "top")
        );
        // each text, as an anchor, evaluates to the frame the snap was drawn at
        let mut text = PIN.to_string();
        let geo: Vec<&Snap> = got.snaps.iter().filter(|s| s.kind != SnapKind::Anchor).collect();
        for (k, s) in geo.iter().enumerate() {
            if s.text != "top" {
                text.push_str(&format!("snap_{k} = anchor({})\n", s.text));
            }
        }
        let frames = forge_lang::asm::part_anchors(&text, None);
        for (k, s) in geo.iter().enumerate() {
            let name = if s.text == "top" {
                "top".to_string()
            } else {
                format!("snap_{k}")
            };
            let f = frames
                .iter()
                .find(|x| x.0 == name)
                .unwrap_or_else(|| panic!("{}", s.text))
                .1;
            assert!(
                (f.origin - s.frame.origin).length() < 1e-9,
                "{} {f:?} {:?}",
                s.text,
                s.frame
            );
            assert!(f.z().get().dot(s.frame.z().get()) > 1.0 - 1e-12, "{}", s.text);
        }
    }
}
