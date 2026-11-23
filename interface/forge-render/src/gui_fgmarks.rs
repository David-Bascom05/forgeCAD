//! What a Forge part's browser shows on the model: the faces of the row chosen lit, each feature
//! the build flags tinted (a lint's when its row is chosen), the chosen feature's number as an
//! arrow to drag (`shell::fgdim`), and a selection window's edges taken by an edges selection.

use super::Gui;
use crate::render::{Entity, FaceColour};
use crate::shell::fg::{FgPanel, Severity, Status};

/// A warning's tint over a face (straight alpha), and an error's.
const AMBER: [u8; 4] = [0xE0, 0xA0, 0x30, 120];
const RED: [u8; 4] = [0xE0, 0x45, 0x40, 130];

/// Is feature `id` the row `key`, or under it (a loop's instance, a call's feature)?
fn under(id: &str, key: &str) -> bool {
    id == key
        || id
            .strip_prefix(key)
            .is_some_and(|r| r.starts_with('.') || r.starts_with('['))
}

/// The worst the build says of feature `id` itself: failed, an error, a warning or shop rule, a
/// lint.
fn worst(p: &FgPanel, id: &str) -> Option<Severity> {
    let failed = p.feats.iter().any(|f| f.id == id && f.status == Status::Failed);
    let mut w: Option<Severity> = failed.then_some(Severity::Error);
    let rank = |s: Severity| match s {
        Severity::Error => 3,
        Severity::Warning | Severity::Violation => 2,
        Severity::Lint => 1,
    };
    for i in p.issues.iter().filter(|i| i.feature.as_deref() == Some(id)) {
        if crate::shell::fg::is_note(i) {
            continue;
        }
        if w.is_none_or(|x| rank(i.severity) > rank(x)) {
            w = Some(i.severity);
        }
    }
    w
}

/// Is edge `k` of the part shown straight: every point of its polyline on the line through its
/// ends?
pub(super) fn straight(mesh: Option<&crate::mesh::PartMesh>, k: u32) -> bool {
    let Some(l) = mesh.and_then(|m| m.lods.first()) else {
        return false;
    };
    let Some(r) = l.edges.iter().find(|r| r.edge == k) else {
        return false;
    };
    let (a, b) = (r.start_at, r.end_at);
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len2 = d.iter().map(|x| x * x).sum::<f32>();
    if len2 < 1e-12 {
        return false;
    }
    let f = |b: &[u8], k: usize| f32::from_le_bytes([b[k], b[k + 1], b[k + 2], b[k + 3]]);
    (r.first..r.first + r.count).all(|i| {
        let at = i as usize * crate::mesh::SEGMENT;
        let Some(seg) = l.segments.get(at..at + crate::mesh::SEGMENT) else {
            return false;
        };
        [0usize, 16].iter().all(|&o| {
            let p = [f(seg, o), f(seg, o + 4), f(seg, o + 8)];
            let w = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
            let t = (w[0] * d[0] + w[1] * d[1] + w[2] * d[2]) / len2;
            let off: f32 = (0..3).map(|j| (w[j] - t * d[j]).powi(2)).sum();
            off < 1e-6 * len2.max(1.0)
        })
    })
}

impl Gui {
    /// A Forge part (not an assembly) with nothing over its browser's choice: no tool, no sketch.
    fn fg_part_quiet(&self) -> Option<&FgPanel> {
        let p = self.shell.fg.as_ref()?;
        (p.asm.is_none() && !p.assembly && p.tool.is_none() && !p.flat && self.shell.sketch.is_none())
            .then_some(p)
    }

    /// The faces the browser's chosen row made: its feature's, or every feature's under a group.
    pub fn fg_lit_faces(&self) -> Vec<u32> {
        let (Some(p), Some(a)) = (self.fg_part_quiet(), self.fg_anchors.as_ref()) else {
            return vec![];
        };
        let Some(key) = p.selected.as_deref() else {
            return vec![];
        };
        let mut out: Vec<u32> = p
            .feats
            .iter()
            .filter(|f| under(&f.id, key))
            .filter_map(|f| a.role_faces.get(&format!("{}.faces", f.id)))
            .flatten()
            .copied()
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The faces of each feature the build flags, in its colour: an error's red, a warning's
    /// amber; a lint's amber only while its row is chosen (every loose number would tint).
    pub fn fg_flag_colours(&self) -> Vec<FaceColour> {
        let (Some(p), Some(a)) = (self.fg_part_quiet(), self.fg_anchors.as_ref()) else {
            return vec![];
        };
        let chosen = p.selected.as_deref();
        let mut out = Vec::new();
        for f in &p.feats {
            let rgba = match worst(p, &f.id) {
                Some(Severity::Error) => RED,
                Some(Severity::Warning | Severity::Violation) => AMBER,
                Some(Severity::Lint) if chosen.is_some_and(|k| under(&f.id, k)) => AMBER,
                _ => continue,
            };
            for &face in a.role_faces.get(&format!("{}.faces", f.id)).into_iter().flatten() {
                out.push(FaceColour {
                    placement: 0,
                    face,
                    rgba,
                });
            }
        }
        // the DFM panel's chosen violation, over the rest
        if let Some((anchor, _, _)) = self.shell.sheet.vendor.lit() {
            for &face in a.role_faces.get(anchor).into_iter().flatten() {
                out.retain(|c| c.face != face);
                out.push(FaceColour {
                    placement: 0,
                    face,
                    rgba: crate::shell::vendorui::LIT,
                });
            }
        }
        out
    }

    /// What the browser lights and tints changed since the view was drawn: draw it again.
    pub(super) fn fg_marks_changed(&mut self) -> bool {
        let now = (self.fg_lit_faces(), self.fg_flag_colours());
        if self.fg_marks_was != now {
            self.fg_marks_was = now;
            return true;
        }
        false
    }

    /// The chosen row's number as an arrow on the model, found again for each build; kept as it
    /// is while it's dragged or a preview of it is on screen.
    pub(super) fn fg_dim_sync(&mut self) {
        let busy = self.shell.fg_dim.as_ref().is_some_and(|d| d.dragging)
            || self.fg.as_ref().is_some_and(|s| s.previewing);
        if busy {
            return;
        }
        let want = self.fg_part_quiet().and_then(|p| p.selected.clone());
        let (Some(id), Some(s), Some(a), Some(mesh)) = (
            want,
            self.fg.as_ref(),
            self.fg_anchors.as_ref(),
            self.fg_folded.as_ref(),
        ) else {
            self.shell.fg_dim = None;
            return;
        };
        // the build its arrow stands on: the whole mesh's and the anchors'
        let key = s.whole ^ (std::sync::Arc::as_ptr(a) as u64).rotate_left(17);
        if self
            .shell
            .fg_dim
            .as_ref()
            .is_some_and(|d| d.feat == id && d.key == key)
        {
            return;
        }
        let Some(p) = self.shell.fg.as_ref() else { return };
        self.shell.fg_dim = p
            .feats
            .iter()
            .find(|f| f.id == id)
            .and_then(|f| crate::shell::fgdim::handle(f, &s.src, &p.issues, &p.params, a, mesh, key));
    }

    /// A selection window let go while a tool's edges selection takes picks: each edge it holds
    /// that Forge names, taken; the rest said.
    pub(super) fn fg_window_edges(&mut self, edges: Vec<Entity>) -> bool {
        use crate::shell::fgtool::SlotKind;
        let Some(tool) = self.shell.fg.as_mut().and_then(|p| p.tool.as_mut()) else {
            return false;
        };
        if tool
            .slots
            .get(tool.active)
            .is_none_or(|s| s.kind != SlotKind::Edges)
        {
            return false;
        }
        let names = self.fg_anchors.as_ref().map(|a| &a.edges);
        let (mut took, mut unnamed) = (0, 0);
        for e in edges {
            let Entity::Edge(k) = e else { continue };
            match names.and_then(|n| n.get(k as usize).cloned().flatten()) {
                Some(t) => {
                    let slot = &tool.slots[tool.active];
                    if !slot.picks.contains(&t) {
                        tool.take(Ok(t));
                        took += 1;
                    }
                }
                None => unnamed += 1,
            }
        }
        self.shell.status = match unnamed {
            0 => format!("Window: {took} edges taken"),
            n => format!("Window: {took} edges taken; {n} with no stable Forge name left out"),
        };
        true
    }

    /// A hole placed from two edges: where its centre is on the face now (those distances in
    /// from each edge, on the face's side), in the face plane's coordinates, for the sketch the
    /// tool writes to start its solve from.
    pub(super) fn fg_hole_guess(&mut self) {
        let Some(t) = self.shell.fg.as_mut().and_then(|p| p.tool.as_mut()) else {
            return;
        };
        if t.kind != "hole" || t.choice("Placement") != "Linear" {
            return;
        }
        let one = |l: &str| {
            t.slots
                .iter()
                .find(|s| s.label == l)
                .and_then(|s| s.picks.first().cloned())
        };
        let (Some(face), Some(e1), Some(e2)) = (one("Face"), one("Edge 1"), one("Edge 2")) else {
            return;
        };
        let num = |n: &str| {
            let p = t.params.iter().find(|p| p.name == n)?;
            crate::shell::fg::parse_in(&p.text, p.dim, t.units).map(|l| l.value)
        };
        let (Some(d1), Some(d2)) = (num("d1"), num("d2")) else {
            return;
        };
        let (Some(a), Some(l)) = (
            self.fg_anchors.as_ref(),
            self.fg_folded.as_ref().and_then(|m| m.lods.first()),
        ) else {
            return;
        };
        let index =
            |list: &[Option<String>], text: &str| list.iter().position(|x| x.as_deref() == Some(text));
        let ends = |text: &str| {
            let k = index(&a.edges, text)? as u32;
            let r = l.edges.iter().find(|r| r.edge == k)?;
            Some((r.start_at.map(f64::from), r.end_at.map(f64::from)))
        };
        let (Some(fk), Some(a1), Some(a2)) = (index(&a.faces, &face), ends(&e1), ends(&e2)) else {
            return;
        };
        let (Some((on, n)), Some(plane)) = (
            crate::shell::fgdim::face_point(l, fk as u32),
            a.planes.get(&format!("{face}.plane")),
        ) else {
            return;
        };
        let Some(at) = linear_place(on, n, [a1, a2], (d1, d2)) else {
            return;
        };
        let o = plane.origin;
        let w = [at[0] - o.x, at[1] - o.y, at[2] - o.z];
        let (x, y) = (plane.x().get(), plane.y().get());
        let g = (
            w[0] * x.x + w[1] * x.y + w[2] * x.z,
            w[0] * y.x + w[1] * y.y + w[2] * y.z,
        );
        t.at_guess = Some(g);
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-12);
    a.map(|v| v / l)
}

/// Where on the face (a point `on` it, its normal `n`) a place is that's `d.0` in from the first
/// edge and `d.1` from the second (each by its ends), on the side of each the face's point is:
/// three planes met. None if the edges are parallel.
pub fn linear_place(
    on: [f64; 3],
    n: [f64; 3],
    edges: [([f64; 3], [f64; 3]); 2],
    d: (f64, f64),
) -> Option<[f64; 3]> {
    let inward = |a: [f64; 3], b: [f64; 3]| {
        let v = unit(cross(n, sub(b, a)));
        if dot(v, sub(on, a)) >= 0.0 {
            v
        } else {
            v.map(|x| -x)
        }
    };
    let [(a1, b1), (a2, b2)] = edges;
    let (v1, v2) = (inward(a1, b1), inward(a2, b2));
    let (r1, r2, r3) = (dot(n, on), dot(v1, a1) + d.0, dot(v2, a2) + d.1);
    let (c23, c31, c12) = (cross(v1, v2), cross(v2, n), cross(n, v1));
    let det = dot(n, c23);
    if det.abs() < 1e-9 {
        return None;
    }
    Some([0, 1, 2].map(|k| (c23[k] * r1 + c31[k] * r2 + c12[k] * r3) / det))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_linear_hole_is_its_distances_in_from_each_edge_on_the_face() {
        // the top of a 60 square at z 20, from its front edge (y 0) and its left (x 0)
        let at = linear_place(
            [30.0, 30.0, 20.0],
            [0.0, 0.0, 1.0],
            [
                ([0.0, 0.0, 20.0], [60.0, 0.0, 20.0]),
                ([0.0, 60.0, 20.0], [0.0, 0.0, 20.0]),
            ],
            (10.0, 15.0),
        )
        .unwrap();
        assert!(
            sub(at, [15.0, 10.0, 20.0]).iter().all(|v| v.abs() < 1e-9),
            "{at:?}"
        );
        // the edges parallel: nowhere
        assert!(
            linear_place(
                [30.0, 30.0, 20.0],
                [0.0, 0.0, 1.0],
                [
                    ([0.0, 0.0, 20.0], [60.0, 0.0, 20.0]),
                    ([0.0, 60.0, 20.0], [60.0, 60.0, 20.0])
                ],
                (10.0, 15.0)
            )
            .is_none()
        );
    }
}
