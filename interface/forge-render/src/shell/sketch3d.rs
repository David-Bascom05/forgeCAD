//! The model's sketches in the view: drawn on their planes, chosen by pointing, their closed regions
//! picked for a feature that reads one.

use super::Shell;
use egui::{Color32, Pos2};
use forge_doc::ItemId;

/// How near a curve the pointer chooses it, in points.
const NEAR_PT: f32 = 6.0;

fn to_line_gap(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let (ab, ap) = (b - a, p - a);
    let t = (ap.dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// Is this place inside these loops, holes being out?
fn inside(loops: &[Vec<Pos2>], p: Pos2) -> bool {
    let mut odd = false;
    for l in loops {
        for k in 0..l.len() {
            let (a, b) = (l[k], l[(k + 1) % l.len()]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                odd = !odd;
            }
        }
    }
    odd
}

fn area(l: &[Pos2]) -> f32 {
    let mut s = 0.0;
    for k in 0..l.len() {
        let (a, b) = (l[k], l[(k + 1) % l.len()]);
        s += a.x * b.y - b.x * a.y;
    }
    s.abs() * 0.5
}

/// A flat fill of any outline: a span of each row of points between its crossings.
fn fill(p: &egui::Painter, loops: &[Vec<Pos2>], ink: Color32) {
    let mut mesh = egui::Mesh::default();
    let (mut top, mut bottom) = (f32::MAX, f32::MIN);
    for q in loops.iter().flatten() {
        top = top.min(q.y);
        bottom = bottom.max(q.y);
    }
    let mut y = top.max(p.clip_rect().top());
    let end = bottom.min(p.clip_rect().bottom());
    let mut cuts = Vec::new();
    while y < end {
        cuts.clear();
        let mid = y + 0.5;
        for l in loops {
            for k in 0..l.len() {
                let (a, b) = (l[k], l[(k + 1) % l.len()]);
                if (a.y > mid) != (b.y > mid) {
                    cuts.push(a.x + (mid - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        cuts.sort_by(f32::total_cmp);
        for c in cuts.as_chunks::<2>().0 {
            mesh.add_colored_rect(
                egui::Rect::from_min_max(egui::pos2(c[0], y), egui::pos2(c[1], y + 1.0)),
                ink,
            );
        }
        y += 1.0;
    }
    p.add(mesh);
}

impl Shell {
    /// Does it show? As told, else until something reads it.
    pub fn sketch_shown(&self, id: ItemId) -> bool {
        let told = self
            .rows
            .iter()
            .find(|r| r.id == id)
            .and_then(|r| self.sketch_eye.get(&r.name));
        told.copied().unwrap_or_else(|| {
            let Some(doc) = &self.doc else { return true };
            let name = doc.name(id);
            !doc.items.iter().any(|i| {
                i.id != id
                    && i.args()
                        .iter()
                        .any(|a| a.to == Some(id) || Some(&a.text) == name.as_ref())
            })
        })
    }

    /// A sketch's eye turned: what it now is.
    pub fn toggle_sketch(&mut self, id: ItemId) {
        let now = !self.sketch_shown(id);
        if let Some(r) = self.rows.iter().find(|r| r.id == id) {
            self.sketch_eye.insert(r.name.clone(), now);
        }
    }

    /// The sketches to draw in the view, each with whether it's lit: not the one being edited, which
    /// the sketch mode draws on top.
    pub fn sketches_drawn(&self) -> Vec<(usize, bool)> {
        let drawing = match self.sketch.as_ref().map(|m| &m.edit) {
            Some(super::sketching::SketchFor::Line(id)) => Some(*id),
            _ => None,
        };
        self.sketch_shapes
            .iter()
            .enumerate()
            .filter_map(|(k, s)| {
                let id = s.item?;
                let lit =
                    self.selected == Some(id) || self.tree_hover == Some(id) || self.sketch_hover == Some(id);
                ((self.sketch_shown(id) || lit) && drawing != Some(id)).then_some((k, lit))
            })
            .collect()
    }

    /// Does the open dialog take a sketch's region for its profile?
    fn picks_region(&self) -> bool {
        self.dialog
            .as_ref()
            .is_some_and(|d| matches!(d.target, super::dialog::DialogTarget::New("extrude" | "revolve")))
    }

    fn picks_points(&self) -> bool {
        self.dialog
            .as_ref()
            .is_some_and(|d| matches!(d.target, super::dialog::DialogTarget::New("hole")))
    }

    /// Placing a hole: every place of every sketch shows, chosen ones lit, and a click takes one.
    fn points_ui(
        &mut self,
        ctx: &egui::Context,
        shapes: &[forge_agent::handles::SketchShape],
        pointer: Option<Pos2>,
        screen: &impl Fn([f64; 3]) -> Option<Pos2>,
    ) {
        let (t, view) = (self.theme, self.view_rect);
        let p = ctx
            .layer_painter(egui::LayerId::background())
            .with_clip_rect(view);
        let chosen = self
            .dialog
            .as_ref()
            .and_then(|d| d.fields.iter().find(|f| f.0 == "Points"))
            .map_or(String::new(), |f| f.1.clone());
        let mut near: Option<(f32, usize, usize, Pos2)> = None;
        for (i, s) in shapes.iter().enumerate() {
            let Some(name) = s.item.and_then(|id| self.doc.as_ref().and_then(|d| d.name(id))) else {
                continue;
            };
            let words: Vec<&str> = chosen.split_whitespace().collect();
            for (j, (point, w)) in s.named.iter().enumerate() {
                let Some(u) = screen(*w) else { continue };
                let on = words.get(1) == Some(&name.as_str()) && words[3..].contains(&point.as_str());
                p.circle_filled(u, if on { 6.0 } else { 4.0 }, if on { t.focus } else { t.sketch });
                p.circle_stroke(u, if on { 6.0 } else { 4.0 }, egui::Stroke::new(1.0, t.surface));
                if let Some(at) = pointer {
                    let gap = at.distance(u);
                    if gap < 10.0 && near.is_none_or(|n| gap < n.0) {
                        near = Some((gap, i, j, u));
                    }
                }
            }
        }
        let Some((_, i, j, u)) = near else { return };
        p.circle_stroke(u, 8.0, egui::Stroke::new(2.0, t.focus));
        if ctx.input(|e| e.pointer.primary_clicked()) {
            let s = &shapes[i];
            let name = s.item.and_then(|id| self.doc.as_ref().and_then(|d| d.name(id)));
            // the way in: `on` drills along the sketch's normal, `-on` against it
            let z: forge_math::Vec3 = s.plane.z().into();
            let o = s.plane.origin;
            let m = self.part_middle;
            let toward = (m[0] as f64 - o.x) * z.x + (m[1] as f64 - o.y) * z.y + (m[2] as f64 - o.z) * z.z;
            let on = if toward > 0.0 { "on" } else { "-on" };
            if let (Some(d), Some(name)) = (&mut self.dialog, name) {
                d.toggle_point(&name, &s.named[j].0, on);
            }
        }
    }

    pub(super) fn sketches_ui(&mut self, ctx: &egui::Context) {
        // a press stays with what it was on: its release is what chooses
        let was = self.sketch_hover.filter(|_| ctx.input(|i| i.pointer.any_down()));
        self.sketch_hover = was;
        let Some((camera, size)) = self.camera else { return };
        let (view, ppp, t) = (self.view_rect, ctx.pixels_per_point(), self.theme);
        let screen = |w: [f64; 3]| {
            camera
                .pixel_of([w[0] as f32, w[1] as f32, w[2] as f32], size.0, size.1)
                .map(|px| view.min + egui::vec2(px.0, px.1) / ppp)
        };
        let drawing = match self.sketch.as_ref().map(|m| &m.edit) {
            Some(super::sketching::SketchFor::Line(id)) => Some(*id),
            _ => None,
        };
        let free_pointer = self.sketch.is_none() && !self.menu_open();
        let pointer = ctx
            .pointer_hover_pos()
            .filter(|p| {
                free_pointer
                    && view.contains(*p)
                    && !self.view_blocks.iter().any(|r| r.contains(*p))
                    && ctx
                        .layer_id_at(*p)
                        .is_none_or(|l| l.order == egui::Order::Background)
            })
            .filter(|_| !ctx.input(|i| i.pointer.any_down()));
        let shapes = std::mem::take(&mut self.sketch_shapes);
        if self.picks_points() {
            self.points_ui(ctx, &shapes, pointer, &screen);
        }
        let picking = self.picks_region();
        let p = ctx
            .layer_painter(egui::LayerId::background())
            .with_clip_rect(view);

        // what's chosen in the dialog, and the region under the pointer
        let chosen: Option<Vec<String>> = self.dialog.as_ref().and_then(|d| {
            let f = d.fields.iter().find(|f| f.0 == "Profile")?;
            Some(f.1.split_whitespace().map(str::to_string).collect())
        });
        let mut under: Option<(f32, ItemId, String, Vec<Vec<Pos2>>)> = None;
        let mut near: Option<(f32, ItemId)> = None;
        let up = |l: &Vec<[f64; 3]>| -> Option<Vec<Pos2>> { l.iter().map(|w| screen(*w)).collect() };
        for s in &shapes {
            let Some(id) = s.item else { continue };
            let lit = self.selected == Some(id) || self.tree_hover == Some(id);
            if !(self.sketch_shown(id) || lit) || drawing == Some(id) {
                continue;
            }
            let name = self.doc.as_ref().and_then(|d| d.name(id)).unwrap_or_default();
            if picking {
                for r in &s.regions {
                    let Some(loops) = r.loops.iter().map(&up).collect::<Option<Vec<_>>>() else {
                        continue;
                    };
                    let is_chosen = chosen.as_ref().is_some_and(|c| {
                        c.first().is_some_and(|w| *w == name) && c.get(1).is_none_or(|w| *w == r.through)
                    });
                    if is_chosen {
                        fill(&p, &loops, t.focus.gamma_multiply(0.28));
                    }
                    if let Some(at) = pointer
                        && inside(&loops, at)
                    {
                        let size = loops.first().map_or(0.0, |l| area(l));
                        if under.as_ref().is_none_or(|u| size < u.0) {
                            under = Some((size, id, r.through.clone(), loops));
                        }
                    }
                }
            }
            for (c, _) in &s.curves {
                let Some(pts) = up(c) else { continue };
                if let Some(at) = pointer {
                    for w in pts.windows(2) {
                        let gap = to_line_gap(at, w[0], w[1]);
                        if gap < NEAR_PT && near.is_none_or(|n| gap < n.0) {
                            near = Some((gap, id));
                        }
                    }
                }
            }
        }
        if !picking && was.is_none() {
            self.sketch_hover = near.map(|n| n.1);
        }
        if let Some((_, id, through, loops)) = &under {
            fill(&p, loops, t.focus.gamma_multiply(0.4));
            if ctx.input(|i| i.pointer.primary_clicked()) {
                let name = self.doc.as_ref().and_then(|d| d.name(*id));
                let many = shapes
                    .iter()
                    .find(|s| s.item == Some(*id))
                    .is_some_and(|s| s.regions.len() > 1);
                if let (Some(d), Some(name)) = (&mut self.dialog, name) {
                    d.on_sketch(&if many { format!("{name} {through}") } else { name });
                }
            }
        }
        self.sketch_shapes = shapes;
    }
}
