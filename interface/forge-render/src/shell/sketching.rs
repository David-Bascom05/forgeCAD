//! Sketch mode: draw on a plane with the solver behind every row. Each tool adds a row to the block a
//! part file would hold; the rows solve here in-process, and Finish Sketch commits them as one call.

use super::theme::{mono, sans};
use super::{Action, Shell};
use egui::{Color32, Pos2, Rect, Sense, Stroke, vec2};
use forge_doc::text::{Want, tokens};
use forge_doc::{Document, ItemId, expr, sketch as sk};
use forge_math::{Frame, Point2};
use forge_sketch::{Entity, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkTool {
    Line,
    Circle,
    Arc,
    Rectangle,
    Point,
    Dimension,
    Coincident,
    Parallel,
    Perpendicular,
    Tangent,
}

/// What a dimension measures, in the plane: how it's drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Measure {
    /// Between two points, along this direction.
    Along((f64, f64), (f64, f64), (f64, f64)),
    /// A circle's or arc's centre and radius, and whether it reads as a diameter.
    Round((f64, f64), f64, bool),
}

/// Dimension lines stand this far off what they measure, then this much more each (points).
const DIM_OFF: f32 = 26.0;
const DIM_STEP: f32 = 22.0;
const ARROW_PT: f32 = 8.0;

/// How close a click must be to take a point or an entity, in points.
const SNAP_PT: f32 = 9.0;
/// A line this close to level or plumb is made so.
const SQUARE_TO: f64 = 0.05;
const CIRCLE_STEPS: usize = 64;

/// What Finish Sketch changes.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchFor {
    New,
    /// A sketch already there.
    Line(ItemId),
    /// A feature's inline profile, which becomes a named sketch line it reads, in one commit.
    Inline(ItemId, String),
}

#[derive(Clone, Debug)]
pub struct SketchMode {
    /// What to change: a sketch already there, or a new one.
    pub edit: SketchFor,
    /// After `sketch`: a frame, or `on @f..`.
    pub head: String,
    pub plane: Frame,
    pub rows: Vec<String>,
    pub tool: Option<SkTool>,
    /// What's been clicked so far for the tool, each as a row's word(s).
    picks: Vec<String>,
    /// Where those are, in the plane.
    pick_at: Vec<(f64, f64)>,
    pub solved: Option<forge_model::Sketch>,
    pub error: Option<String>,
    /// A dimension being given: the row without its value, where its field goes, the text.
    pub asking: Option<(String, Pos2, String)>,
    /// Numbers for `dist` and the like read through the part's parameters.
    params: Vec<(String, String)>,
    /// The rows it opened with: nothing changed, nothing to commit.
    opened: Vec<String>,
    /// A dimension's row being changed, when it's one already there.
    asking_row: Option<usize>,
}

/// A row added, for tests and the status bar.
fn num_of(params: &[(String, String)]) -> impl Fn(&str, Want) -> Result<f64, String> + '_ {
    move |t: &str, want: Want| {
        let env = |name: &str| {
            let (_, e) = params.iter().rev().find(|p| p.0 == name)?;
            let v: f64 = e.parse().ok().or_else(|| {
                let inner = |n: &str| {
                    params
                        .iter()
                        .rev()
                        .find(|p| p.0 == n)
                        .and_then(|p| p.1.parse::<f64>().ok())
                        .map(|v| (v, forge_model::Unit::None))
                };
                expr::eval(e, &inner).ok().map(|r| r.0)
            })?;
            Some((v, forge_model::Unit::None))
        };
        let v = match t.parse::<f64>() {
            Ok(v) => v,
            Err(_) => expr::eval(t, &env)?.0,
        };
        Ok(match want {
            Want::Angle => v * core::f64::consts::PI / 180.0,
            _ => v,
        })
    }
}

impl SketchMode {
    pub fn new(
        edit: SketchFor,
        head: String,
        plane: Frame,
        rows: Vec<String>,
        doc: Option<&Document>,
    ) -> SketchMode {
        let params = doc.map_or(Vec::new(), |d| {
            d.items
                .iter()
                .filter_map(|i| match &i.body {
                    forge_doc::Body::Param { name, expr, .. } => Some((name.clone(), expr.clone())),
                    _ => None,
                })
                .collect()
        });
        let mut m = SketchMode {
            edit,
            head,
            plane,
            opened: rows.clone(),
            asking_row: None,
            rows,
            tool: None,
            picks: Vec::new(),
            pick_at: Vec::new(),
            solved: None,
            error: None,
            asking: None,
            params,
        };
        m.solve();
        m
    }

    fn build(&self, rows: &[String]) -> Result<forge_model::Sketch, String> {
        let toks: Vec<Vec<String>> = rows.iter().map(|r| tokens(r)).collect();
        sk::build(self.plane, toks, &num_of(&self.params), self.solved.as_ref())
    }

    fn solve(&mut self) {
        match self.build(&self.rows) {
            Ok(s) => {
                self.solved = Some(s);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Adds a row if the sketch still solves; if not, says why and leaves it as it was.
    pub fn add(&mut self, row: String) -> bool {
        let mut next = self.rows.clone();
        next.push(row);
        match self.build(&next) {
            Ok(s) => {
                self.rows = next;
                self.solved = Some(s);
                self.error = None;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }

    /// Nothing's different from how it opened.
    pub fn unchanged(&self) -> bool {
        self.rows == self.opened
    }

    /// Replaces a row if the sketch still solves; if not, says why and leaves it as it was.
    pub fn replace(&mut self, k: usize, row: String) -> bool {
        let mut next = self.rows.clone();
        next[k] = row;
        // from the rows as written, not from where the last solve left them
        let was = self.solved.take();
        match self.build(&next) {
            Ok(s) => {
                self.rows = next;
                self.solved = Some(s);
                self.error = None;
                true
            }
            Err(e) => {
                self.solved = was;
                self.error = Some(e);
                false
            }
        }
    }

    /// The dimensions there are: the row's number, its words before the value, the value as written,
    /// and what it measures.
    pub fn dimensions(&self) -> Vec<(usize, String, String, Measure)> {
        let Some(s) = &self.solved else { return Vec::new() };
        let at = |w: &str| -> Option<(f64, f64)> { sk::point(s, w).ok().map(|p| (p.x, p.y)) };
        let mut out = Vec::new();
        for (k, row) in self.rows.iter().enumerate() {
            let t = tokens(row);
            let Some((value, words)) = t.split_last() else {
                continue;
            };
            let measure = match words {
                [w, a, b] if w == "hdist" => at(a).zip(at(b)).map(|(p, q)| Measure::Along(p, q, (1.0, 0.0))),
                [w, a, b] if w == "vdist" => at(a).zip(at(b)).map(|(p, q)| Measure::Along(p, q, (0.0, 1.0))),
                [w, a, b] if w == "dist" => at(a).zip(at(b)).map(|(p, q)| {
                    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
                    let l = (dx * dx + dy * dy).sqrt().max(1e-12);
                    Measure::Along(p, q, (dx / l, dy / l))
                }),
                [w, l] if w == "dist" => sk::line(s, l).ok().map(|(p, q)| {
                    let (dx, dy) = (q.x - p.x, q.y - p.y);
                    let n = (dx * dx + dy * dy).sqrt().max(1e-12);
                    Measure::Along((p.x, p.y), (q.x, q.y), (dx / n, dy / n))
                }),
                [w, c] if w == "dia" || w == "rad" => s
                    .names
                    .iter()
                    .find(|n| n.0 == *c)
                    .and_then(|(_, id)| Some(Measure::Round(at(c)?, s.solved.radius(*id), w == "dia"))),
                _ => None,
            };
            if let Some(m) = measure {
                out.push((k, words.join(" "), value.clone(), m));
            }
        }
        out
    }

    /// The middle of what's drawn, for which side a dimension stands off to.
    fn middle(&self) -> (f64, f64) {
        let pts = self.points();
        if pts.is_empty() {
            return (0.0, 0.0);
        }
        let n = pts.len() as f64;
        let (x, y) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.1.0, a.1 + p.1.1));
        (x / n, y / n)
    }

    /// Takes the last row back.
    pub fn undo(&mut self) {
        if self.rows.pop().is_some() {
            self.solved = None;
            self.solve();
        }
    }

    /// The block as a part file holds it.
    pub fn block(&self) -> String {
        let mut out = format!("sketch {}\n", self.head);
        for r in &self.rows {
            out.push_str("  ");
            out.push_str(r);
            out.push('\n');
        }
        out.push_str("end\n");
        out
    }

    pub fn set_tool(&mut self, t: Option<SkTool>) {
        self.tool = t;
        self.picks.clear();
        self.pick_at.clear();
        self.asking = None;
        self.asking_row = None;
    }

    /// Esc: what's half done goes, then the tool.
    pub fn escape(&mut self) {
        if !self.picks.is_empty() || self.asking.is_some() {
            self.picks.clear();
            self.pick_at.clear();
            self.asking = None;
            self.asking_row = None;
        } else {
            self.tool = None;
        }
    }

    /// Every point the sketch names, in the plane.
    fn points(&self) -> Vec<(String, (f64, f64))> {
        let Some(s) = &self.solved else { return Vec::new() };
        let mut out = Vec::new();
        let g = &s.solved;
        for (name, id) in &s.names {
            let mut put = |n: String, p: Point2| out.push((n, (p.x, p.y)));
            match g.entity(*id) {
                Some(Entity::Point { .. }) => put(name.clone(), g.point(*id)),
                Some(Entity::Line { a, b }) => {
                    put(format!("{name}.a"), g.point(a));
                    put(format!("{name}.b"), g.point(b));
                }
                Some(Entity::Circle { center, .. }) => put(name.clone(), g.point(center)),
                Some(Entity::Arc {
                    center, start, end, ..
                }) => {
                    put(name.clone(), g.point(center));
                    put(format!("{name}.a"), g.point(start));
                    put(format!("{name}.b"), g.point(end));
                }
                None => {}
            }
        }
        out
    }

    /// The name of what's nearest a place, of those whose name starts with one of `kinds`, within `reach`.
    fn entity_near(&self, at: (f64, f64), reach: f64, kinds: &str) -> Option<String> {
        let s = self.solved.as_ref()?;
        let g = &s.solved;
        let mut best: Option<(f64, String)> = None;
        for (name, id) in &s.names {
            if !name.starts_with(|c| kinds.contains(c)) {
                continue;
            }
            let d = match g.entity(*id) {
                Some(Entity::Line { a, b }) => {
                    let (p, q) = (g.point(a), g.point(b));
                    let (dx, dy) = (q.x - p.x, q.y - p.y);
                    let l = (dx * dx + dy * dy).max(1e-12);
                    let t = (((at.0 - p.x) * dx + (at.1 - p.y) * dy) / l).clamp(0.0, 1.0);
                    ((p.x + dx * t - at.0).powi(2) + (p.y + dy * t - at.1).powi(2)).sqrt()
                }
                Some(Entity::Circle { center, .. } | Entity::Arc { center, .. }) => {
                    let c = g.point(center);
                    let r = g.radius(*id);
                    (((c.x - at.0).powi(2) + (c.y - at.1).powi(2)).sqrt() - r).abs()
                }
                _ => continue,
            };
            if d < reach && best.as_ref().is_none_or(|b| d < b.0) {
                best = Some((d, name.clone()));
            }
        }
        best.map(|b| b.1)
    }

    /// A word for a place: a point of the sketch if one's within reach, otherwise its numbers.
    fn word_at(&self, at: (f64, f64), reach: f64) -> (String, (f64, f64)) {
        let near = self
            .points()
            .into_iter()
            .map(|(n, p)| (((p.0 - at.0).powi(2) + (p.1 - at.1).powi(2)).sqrt(), n, p))
            .filter(|c| c.0 < reach)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match near {
            Some((_, n, p)) => (n, p),
            None => {
                let tidy = |v: f64| (v * 100.0).round() / 100.0;
                let (x, y) = (tidy(at.0), tidy(at.1));
                (
                    format!("{} {}", forge_agent::short(x), forge_agent::short(y)),
                    (x, y),
                )
            }
        }
    }

    /// A click for the tool, at this place in the plane, `reach` being what's near in mm.
    pub fn click(&mut self, at: (f64, f64), reach: f64) {
        let Some(tool) = self.tool else { return };
        match tool {
            SkTool::Point => {
                let (w, _) = self.word_at(at, reach);
                self.add(format!("pt {w}"));
            }
            SkTool::Line => {
                let (w, p) = self.word_at(at, reach);
                if let (Some(from), Some(was)) = (self.picks.last().cloned(), self.pick_at.last().copied()) {
                    if !self.add(format!("line {from} {w}")) {
                        return;
                    }
                    let last = self.last_named('l');
                    // level or plumb within a hair is meant so
                    if let Some(l) = &last {
                        let (dx, dy) = (p.0 - was.0, p.1 - was.1);
                        let span = dx.abs().max(dy.abs()).max(1e-9);
                        if dy.abs() / span < SQUARE_TO {
                            self.add(format!("hor {l}"));
                        } else if dx.abs() / span < SQUARE_TO {
                            self.add(format!("ver {l}"));
                        }
                    }
                    // on from where this one ended
                    self.picks = vec![last.map_or(w, |l| format!("{l}.b"))];
                    self.pick_at = vec![p];
                } else {
                    self.picks.push(w);
                    self.pick_at.push(p);
                }
            }
            SkTool::Rectangle => {
                let (_, p) = self.word_at(at, 0.0);
                if let Some(&a) = self.pick_at.first() {
                    let (x, y) = (a.0.min(p.0), a.1.min(p.1));
                    let (w, h) = ((a.0 - p.0).abs(), (a.1 - p.1).abs());
                    if w > 0.0 && h > 0.0 {
                        let s = forge_agent::short;
                        self.add(format!("rect {} {} {} {}", s(x), s(y), s(w), s(h)));
                    }
                    self.pick_at.clear();
                } else {
                    self.pick_at.push(p);
                }
            }
            SkTool::Circle => {
                if let (Some(c), Some(ca)) = (self.picks.first().cloned(), self.pick_at.first().copied()) {
                    let r = ((at.0 - ca.0).powi(2) + (at.1 - ca.1).powi(2)).sqrt();
                    if r > 0.0 {
                        self.add(format!(
                            "circle {c} {}",
                            forge_agent::short((r * 100.0).round() / 100.0)
                        ));
                    }
                    self.picks.clear();
                    self.pick_at.clear();
                } else {
                    let (w, p) = self.word_at(at, reach);
                    self.picks.push(w);
                    self.pick_at.push(p);
                }
            }
            SkTool::Arc => {
                let (w, p) = self.word_at(at, reach);
                self.picks.push(w);
                self.pick_at.push(p);
                if self.picks.len() == 3 {
                    let row = format!("arc {} {} {}", self.picks[0], self.picks[1], self.picks[2]);
                    self.add(row);
                    self.picks.clear();
                    self.pick_at.clear();
                }
            }
            SkTool::Coincident => self.pick_two_points(at, reach),
            SkTool::Parallel | SkTool::Perpendicular | SkTool::Tangent => {
                let kinds = if tool == SkTool::Tangent { "lca" } else { "l" };
                if let Some(e) = self.entity_near(at, reach, kinds) {
                    self.picks.push(e);
                    if self.picks.len() == 2 {
                        let word = match tool {
                            SkTool::Parallel => "par",
                            SkTool::Perpendicular => "perp",
                            _ => "tan",
                        };
                        let row = format!("{word} {} {}", self.picks[0], self.picks[1]);
                        self.add(row);
                        self.picks.clear();
                    }
                }
            }
            SkTool::Dimension => {}
        }
    }

    fn pick_two_points(&mut self, at: (f64, f64), reach: f64) {
        let (w, _) = self.word_at(at, reach);
        if w.contains(' ') {
            return;
        }
        self.picks.push(w);
        if self.picks.len() == 2 {
            let row = format!("co {} {}", self.picks[0], self.picks[1]);
            self.add(row);
            self.picks.clear();
        }
    }

    /// What a dimension here would be: the row without its value, and the value as it stands.
    pub fn dimension_at(&self, at: (f64, f64), reach: f64) -> Option<(String, f64)> {
        let s = self.solved.as_ref()?;
        let e = self.entity_near(at, reach, "lca")?;
        if e.starts_with('l') {
            let (p, q) = sk::line(s, &e).ok()?;
            Some((
                format!("dist {e}"),
                ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt(),
            ))
        } else {
            let (_, id) = s.names.iter().find(|n| n.0 == e)?;
            let r = s.solved.radius(*id);
            if e.starts_with('c') {
                Some((format!("dia {e}"), r * 2.0))
            } else {
                Some((format!("rad {e}"), r))
            }
        }
    }

    fn last_named(&self, kind: char) -> Option<String> {
        let s = self.solved.as_ref()?;
        s.names
            .iter()
            .rev()
            .find(|n| n.0.starts_with(kind))
            .map(|n| n.0.clone())
    }

    /// How many degrees of freedom are left, and whether it solved.
    pub fn state(&self) -> Option<(usize, bool)> {
        let s = self.solved.as_ref()?;
        let solved = s.solved.clone().solve().status == Status::Solved;
        Some((sk::freedom(s).0, solved))
    }
}

impl Shell {
    /// Draws what's sketched, and takes clicks for its tool. Called with sketch mode on.
    pub(super) fn sketch_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some((camera, size)) = self.camera else { return };
        let Some(mut m) = self.sketch.take() else { return };
        let ppp = ctx.pixels_per_point();
        let view = self.view_rect;
        let plane = m.plane;
        let screen = |p: (f64, f64)| {
            let w = plane.plane_point(Point2::new(p.0, p.1));
            camera
                .pixel_of([w.x as f32, w.y as f32, w.z as f32], size.0, size.1)
                .map(|px| view.min + vec2(px.0, px.1) / ppp)
        };
        // the cursor on the plane
        let origin = plane.origin;
        let (z, x, y): (forge_math::Vec3, forge_math::Vec3, forge_math::Vec3) =
            (plane.z().into(), plane.x().into(), plane.y().into());
        let to_plane = |pos: Pos2| -> Option<(f64, f64)> {
            let px = ((pos.x - view.min.x) * ppp, (pos.y - view.min.y) * ppp);
            let eye = camera.eye();
            let through = camera.point_at(px, camera.dist, size.0, size.1);
            let d = [0, 1, 2].map(|k| (through[k] - eye[k]) as f64);
            let n = [z.x, z.y, z.z];
            let denom = d[0] * n[0] + d[1] * n[1] + d[2] * n[2];
            if denom.abs() < 1e-9 {
                return None;
            }
            let w = [
                origin.x - eye[0] as f64,
                origin.y - eye[1] as f64,
                origin.z - eye[2] as f64,
            ];
            let s = (w[0] * n[0] + w[1] * n[1] + w[2] * n[2]) / denom;
            let q = [
                eye[0] as f64 + d[0] * s,
                eye[1] as f64 + d[1] * s,
                eye[2] as f64 + d[2] * s,
            ];
            let v = [q[0] - origin.x, q[1] - origin.y, q[2] - origin.z];
            Some((
                v[0] * x.x + v[1] * x.y + v[2] * x.z,
                v[0] * y.x + v[1] * y.y + v[2] * y.z,
            ))
        };
        // a point's worth of millimetres here
        let mm_per_pt = {
            let (a, b) = (screen((0.0, 0.0)), screen((1.0, 0.0)));
            match (a, b) {
                (Some(a), Some(b)) => 1.0 / ((a - b).length() as f64).max(1e-6),
                _ => 1.0,
            }
        };
        let reach = SNAP_PT as f64 * mm_per_pt;

        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::unique("sketch"),
        ));
        let clip = view;
        let p = p.with_clip_rect(clip);

        // the part ghosted behind the sheet, the cube left alone
        let sheet =
            Color32::from_rgb((t.view >> 16) as u8, (t.view >> 8) as u8, t.view as u8).gamma_multiply(0.6);
        if let Some(place) = self.cube_place {
            let cube_left = view.min.x + place.at.0 / ppp;
            let cube_bottom = view.min.y + (place.at.1 + place.side) / ppp + 8.0;
            p.rect_filled(
                Rect::from_min_max(view.min, egui::pos2(cube_left, view.max.y)),
                0.0,
                sheet,
            );
            p.rect_filled(
                Rect::from_min_max(egui::pos2(cube_left, cube_bottom), view.max),
                0.0,
                sheet,
            );
        }
        // a quiet grid on the plane, its step the round number that keeps it open
        let corners: Vec<(f64, f64)> = [
            view.left_top(),
            view.right_top(),
            view.left_bottom(),
            view.right_bottom(),
        ]
        .into_iter()
        .filter_map(&to_plane)
        .collect();
        if corners.len() == 4 {
            let (x0, x1) = corners
                .iter()
                .fold((f64::MAX, f64::MIN), |a, c| (a.0.min(c.0), a.1.max(c.0)));
            let (y0, y1) = corners
                .iter()
                .fold((f64::MAX, f64::MIN), |a, c| (a.0.min(c.1), a.1.max(c.1)));
            let want = 18.0 * mm_per_pt;
            let base = 10f64.powi(want.log10().floor() as i32);
            let step = [1.0, 2.0, 5.0, 10.0]
                .into_iter()
                .map(|m| m * base)
                .find(|v| *v >= want)
                .unwrap_or(want);
            let minor = Stroke::new(1.0, t.ink2.gamma_multiply(0.07));
            let major = Stroke::new(1.0, t.ink2.gamma_multiply(0.16));
            let line = |a: (f64, f64), b: (f64, f64), s: Stroke| {
                if let (Some(u), Some(v)) = (screen(a), screen(b)) {
                    p.line_segment([u, v], s);
                }
            };
            let mut k = (x0 / step).floor() as i64;
            while (k as f64) * step <= x1 && (x1 - x0) / step < 400.0 {
                let x = k as f64 * step;
                line((x, y0), (x, y1), if k % 5 == 0 { major } else { minor });
                k += 1;
            }
            let mut k = (y0 / step).floor() as i64;
            while (k as f64) * step <= y1 && (y1 - y0) / step < 400.0 {
                let y = k as f64 * step;
                line((x0, y), (x1, y), if k % 5 == 0 { major } else { minor });
                k += 1;
            }
        }
        let ink = t.on_ink;
        let line = Stroke::new(1.5, ink);
        if let Some(s) = &m.solved {
            let g = &s.solved;
            for (name, id) in &s.names {
                match g.entity(*id) {
                    Some(Entity::Line { a, b }) => {
                        let (pa, pb) = (g.point(a), g.point(b));
                        if let (Some(u), Some(v)) = (screen((pa.x, pa.y)), screen((pb.x, pb.y))) {
                            p.line_segment([u, v], line);
                            p.circle_filled(u, 2.5, ink);
                            p.circle_filled(v, 2.5, ink);
                        }
                    }
                    Some(Entity::Circle { center, .. }) => {
                        let (c, r) = (g.point(center), g.radius(*id));
                        let pts: Vec<Pos2> = (0..=CIRCLE_STEPS)
                            .filter_map(|k| {
                                let a = k as f64 / CIRCLE_STEPS as f64 * core::f64::consts::TAU;
                                screen((
                                    c.x + r * forge_math::dmath::cos(a),
                                    c.y + r * forge_math::dmath::sin(a),
                                ))
                            })
                            .collect();
                        p.add(egui::Shape::line(pts, line));
                        if let Some(u) = screen((c.x, c.y)) {
                            p.circle_filled(u, 2.5, ink);
                        }
                    }
                    Some(Entity::Arc {
                        center,
                        start,
                        end,
                        ccw,
                    }) => {
                        let (from, to) = if ccw { (start, end) } else { (end, start) };
                        let (c, a, b) = (g.point(center), g.point(from), g.point(to));
                        let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                        let t0 = forge_math::dmath::atan2(a.y - c.y, a.x - c.x);
                        let mut t1 = forge_math::dmath::atan2(b.y - c.y, b.x - c.x);
                        if t1 <= t0 {
                            t1 += core::f64::consts::TAU;
                        }
                        let pts: Vec<Pos2> = (0..=CIRCLE_STEPS / 2)
                            .filter_map(|k| {
                                let v = t0 + (t1 - t0) * k as f64 / (CIRCLE_STEPS / 2) as f64;
                                screen((
                                    c.x + r * forge_math::dmath::cos(v),
                                    c.y + r * forge_math::dmath::sin(v),
                                ))
                            })
                            .collect();
                        p.add(egui::Shape::line(pts, line));
                    }
                    Some(Entity::Point { .. }) if name != "o" => {
                        let q = g.point(*id);
                        if let Some(u) = screen((q.x, q.y)) {
                            p.circle_filled(u, 3.0, t.focus);
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(u) = screen((0.0, 0.0)) {
            p.circle_stroke(u, 4.0, Stroke::new(1.0, t.ink2));
        }

        // the pointer's for the tool only over the view itself
        let over = ctx.input(|i| i.pointer.hover_pos()).filter(|at| {
            view.contains(*at)
                && ctx
                    .layer_id_at(*at)
                    .is_none_or(|l| l.order == egui::Order::Background)
        });
        let cursor = over.and_then(to_plane);
        if let (Some(tool), Some(at)) = (m.tool, cursor) {
            // what the next click would do
            let rubber = Stroke::new(1.0, t.focus);
            if matches!(
                tool,
                SkTool::Line | SkTool::Circle | SkTool::Rectangle | SkTool::Arc
            ) && let Some(&from) = m.pick_at.last()
                && let (Some(u), Some(v)) = (screen(from), screen(at))
            {
                p.line_segment([u, v], rubber);
            }
            let (_, snap) = m.word_at(at, reach);
            if let Some(u) = screen(snap) {
                p.circle_stroke(u, 5.0, rubber);
            }
            if ctx.input(|i| i.pointer.primary_clicked()) && self.modal_clear() {
                if tool == SkTool::Dimension {
                    if let (Some((row, value)), Some(pos)) = (m.dimension_at(at, reach), over) {
                        m.asking = Some((row, pos, forge_agent::short(value)));
                    }
                } else {
                    m.click(at, reach);
                }
            }
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        // its dimensions, standing off what they measure, as written; a click on one changes it
        let chips = m.dimensions();
        let middle = m.middle();
        let mut ask_chip = None;
        let dim_ink = t.ink2;
        let (mut left_off, mut right_off) = (0usize, 0usize);
        let arrow = |p: &egui::Painter, tip: Pos2, from: Pos2| {
            let way = (tip - from).normalized();
            let side = vec2(-way.y, way.x);
            p.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    tip - way * ARROW_PT + side * 3.0,
                    tip - way * ARROW_PT - side * 3.0,
                ],
                dim_ink,
                Stroke::NONE,
            ));
        };
        egui::Area::new(egui::Id::unique("sketch-dims"))
            .fixed_pos(view.min)
            .order(egui::Order::Middle)
            .constrain(false)
            .show(ctx, |ui| {
                for (k, words, value, what) in &chips {
                    let (pos, text) = match *what {
                        Measure::Along(a, b, d) => {
                            let span = (b.0 - a.0) * d.0 + (b.1 - a.1) * d.1;
                            if span.abs() < 1e-9 {
                                continue;
                            }
                            // to the side away from the middle, each past the last on that side
                            let n = (-d.1, d.0);
                            let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
                            let outward = (mid.0 - middle.0) * n.0 + (mid.1 - middle.1) * n.1 >= 0.0;
                            let sign = if outward { 1.0 } else { -1.0 };
                            let counter = if outward { &mut right_off } else { &mut left_off };
                            let off = (DIM_OFF + DIM_STEP * *counter as f32) as f64 * mm_per_pt;
                            *counter += 1;
                            let along = |q: (f64, f64)| q.0 * n.0 + q.1 * n.1;
                            let reach = if outward {
                                along(a).max(along(b))
                            } else {
                                along(a).min(along(b))
                            } + sign * off;
                            let foot = |q: (f64, f64)| {
                                let by = reach - along(q);
                                (q.0 + n.0 * by, q.1 + n.1 * by)
                            };
                            let (fa, fb) = (foot(a), foot(b));
                            let (Some(ua), Some(ub), Some(wa), Some(wb)) =
                                (screen(a), screen(b), screen(fa), screen(fb))
                            else {
                                continue;
                            };
                            let line = Stroke::new(1.0, dim_ink);
                            p.line_segment([ua, wa], Stroke::new(1.0, dim_ink.gamma_multiply(0.5)));
                            p.line_segment([ub, wb], Stroke::new(1.0, dim_ink.gamma_multiply(0.5)));
                            p.line_segment([wa, wb], line);
                            arrow(&p, wa, wb);
                            arrow(&p, wb, wa);
                            (wa + (wb - wa) * 0.5, value.clone())
                        }
                        Measure::Round(c, r, dia) => {
                            let on = (
                                c.0 + r * core::f64::consts::FRAC_1_SQRT_2,
                                c.1 + r * core::f64::consts::FRAC_1_SQRT_2,
                            );
                            let Some(rim) = screen(on) else { continue };
                            let out = rim + vec2(30.0, -30.0);
                            p.line_segment([rim, out], Stroke::new(1.0, dim_ink));
                            arrow(&p, rim, out);
                            (
                                out + vec2(10.0, -6.0),
                                format!("{}{value}", if dia { "\u{D8}" } else { "R" }),
                            )
                        }
                    };
                    let g = ui.fonts_mut(|f| f.layout_no_wrap(text, mono(12.0), t.ink));
                    let r = Rect::from_center_size(pos, g.size() + vec2(10.0, 6.0));
                    let resp = ui.interact(r, ui.scope_id().with(("dim", *k)), Sense::click());
                    self.view_blocks.push(r);
                    self.places.push((format!("sketch:dim:{words}"), r));
                    p.rect_filled(r, 3.0, if resp.hovered() { t.on } else { t.surface });
                    p.galley(r.min + vec2(5.0, 3.0), g, t.ink);
                    if resp.clicked() {
                        ask_chip = Some((*k, words.clone(), pos, value.clone()));
                    }
                }
            });
        if let Some((k, words, pos, value)) = ask_chip {
            m.asking = Some((words, pos - vec2(12.0, -13.0), value));
            m.asking_row = Some(k);
        }

        // a dimension's value
        let mut asked = None;
        if let Some((row, at, text)) = &mut m.asking {
            egui::Area::new(egui::Id::unique("sketch-dim"))
                .fixed_pos(*at + vec2(12.0, -13.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(100.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(r, 4.0, t.surface);
                    let f = ui.put(
                        r,
                        egui::TextEdit::singleline(text)
                            .font(mono(12.0))
                            .margin(vec2(7.0, 6.0))
                            .desired_width(86.0)
                            .id(egui::Id::unique("sketch-dim-text")),
                    );
                    f.request_focus();
                    self.places.push(("sketch:dim".to_string(), r));
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        asked = Some(format!("{row} {}", text.trim()));
                    }
                });
        }
        if let Some(row) = asked {
            match m.asking_row.take() {
                Some(k) => m.replace(k, row),
                None => m.add(row),
            };
            m.asking = None;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            m.escape();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z))
            && !ctx.egui_wants_keyboard_input()
        {
            m.undo();
        }

        // what state it's in, in the status bar; what's wrong, where it's seen
        self.status = match m.solved.as_ref().map(|s| sk::freedom(s).0) {
            Some(0) => "Fully constrained".to_string(),
            Some(n) => format!("{n} degrees of freedom"),
            None => String::new(),
        };
        if let Some(e) = &m.error {
            let g = ctx.fonts_mut(|f| f.layout(e.clone(), sans(12.0), Color32::WHITE, 420.0));
            let chip = Rect::from_min_size(view.min + vec2(12.0, 12.0), g.size() + vec2(14.0, 8.0));
            p.rect_filled(chip, 4.0, t.error);
            p.galley(chip.min + vec2(7.0, 4.0), g, Color32::WHITE);
        }
        self.sketch = Some(m);
    }

    /// No menu or dialog is in the way of a click.
    fn modal_clear(&self) -> bool {
        !self.menu_open() && self.dialog.is_none()
    }

    /// The ribbon's sketch tools, as actions.
    pub(super) fn sketch_tool(&self, t: SkTool) -> Option<Action> {
        self.sketch.as_ref().map(|_| Action::SketchTool(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode() -> SketchMode {
        SketchMode::new(SketchFor::New, "xy".into(), Frame::default(), Vec::new(), None)
    }

    #[test]
    fn a_line_chain_shares_its_points_and_a_nearly_level_one_is_made_level() {
        let mut m = mode();
        m.set_tool(Some(SkTool::Line));
        m.click((0.0, 0.0), 0.5);
        m.click((20.0, 0.3), 0.5);
        m.click((20.0, 10.0), 0.5);
        assert_eq!(m.rows[0], "line 0 0 20 0.3");
        assert!(m.rows.contains(&"hor l1".to_string()), "{:?}", m.rows);
        assert!(m.rows.contains(&"line l1.b 20 10".to_string()), "{:?}", m.rows);
        assert!(m.rows.contains(&"ver l2".to_string()), "{:?}", m.rows);
        let s = m.solved.as_ref().unwrap();
        let (a, b) = sk::line(s, "l1").unwrap();
        assert!((a.y - b.y).abs() < 1e-9, "the solver made it level: {a:?} {b:?}");
        // Esc ends the chain, then the tool
        m.escape();
        assert_eq!(m.tool, Some(SkTool::Line));
        m.escape();
        assert_eq!(m.tool, None);
    }

    #[test]
    fn shapes_make_their_rows_and_clicks_near_a_point_take_it() {
        let mut m = mode();
        m.set_tool(Some(SkTool::Rectangle));
        m.click((10.0, 10.0), 0.5);
        m.click((-10.0, -5.0), 0.5);
        assert_eq!(m.rows[0], "rect -10 -5 20 15");
        m.set_tool(Some(SkTool::Circle));
        m.click((0.0, 0.0), 0.5);
        m.click((3.0, 4.0), 0.5);
        assert_eq!(m.rows[1], "circle 0 0 5");
        m.set_tool(Some(SkTool::Point));
        // on the rectangle's first corner, within reach: its name
        m.click((-10.1, -5.1), 0.5);
        assert!(m.rows[2].starts_with("pt l"), "{}", m.rows[2]);
        assert!(m.error.is_none());
    }

    #[test]
    fn a_dimension_drives_the_geometry_and_one_that_cannot_hold_is_refused_with_why() {
        let mut m = mode();
        m.rows.push("circle 0 0 5".into());
        m.solve();
        let (row, v) = m.dimension_at((5.0, 0.0), 0.5).unwrap();
        assert_eq!((row.as_str(), v), ("dia c1", 10.0));
        assert!(m.add("dia c1 16".into()));
        let s = m.solved.as_ref().unwrap();
        let id = s.names.iter().find(|n| n.0 == "c1").unwrap().1;
        assert!((s.solved.radius(id) - 8.0).abs() < 1e-9);
        // the same again at another size can't hold
        assert!(!m.add("dia c1 20".into()));
        assert!(m.error.is_some());
        assert_eq!(m.rows.len(), 2, "the rows are as they were");
        m.undo();
        assert_eq!(m.rows.len(), 1);
    }

    #[test]
    fn a_dimension_shows_as_written_and_a_change_to_it_drives_the_sketch() {
        let params = Document::parse("w = 30\n");
        let rows = vec![
            "rect 0 0 w 20".to_string(),
            "hdist l1.a l1.b w".to_string(),
            "vdist l2.a l2.b 20".to_string(),
        ];
        let mut m = SketchMode::new(SketchFor::New, "xy".into(), Frame::default(), rows, Some(&params));
        assert!(m.error.is_none(), "{:?}", m.error);
        assert!(m.unchanged());
        let d = m.dimensions();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].2, "w", "a parameter shows as itself");
        assert!(m.replace(2, "vdist l2.a l2.b 25".into()), "{:?}", m.error);
        let (p, q) = sk::line(m.solved.as_ref().unwrap(), "l2").unwrap();
        assert!((q.y - p.y - 25.0).abs() < 1e-9);
        assert!(!m.unchanged());
        assert!(!m.replace(2, "vdist l2.a l2.b nope".into()));
        assert_eq!(m.rows[2], "vdist l2.a l2.b 25", "a bad one changes nothing");
    }

    #[test]
    fn its_block_reads_as_a_sketch_a_part_can_extrude() {
        let mut m = mode();
        m.set_tool(Some(SkTool::Rectangle));
        m.click((0.0, 0.0), 0.5);
        m.click((30.0, 20.0), 0.5);
        let src = format!("plate: {}extrude 5 plate\n", m.block());
        let p = forge_agent::tools::Part::new(&src);
        assert!(p.model.error.is_none(), "{:?}", p.model.error);
        assert!((forge_agent::model::Model::new(&src).mass().unwrap().volume - 3000.0).abs() < 1e-6);
    }
}
