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

/// A value on screen as an entity's being drawn: live until typed in, then what the entity obeys.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub label: &'static str,
    pub text: String,
    pub locked: bool,
}

/// A relation the click will make, shown by the cursor as it's found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Coincident,
    Midpoint,
    Horizontal,
    Vertical,
    Parallel,
    Perpendicular,
    Tangent,
}

/// What the next click would make: the entity as it will be, and what to write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub lines: Vec<((f64, f64), (f64, f64))>,
    pub circle: Option<((f64, f64), f64)>,
    /// Centre, radius, start and end angles, counter-clockwise.
    pub arc: Option<((f64, f64), f64, f64, f64)>,
    pub rows: Vec<String>,
    pub glyphs: Vec<(Glyph, (f64, f64))>,
    /// Each field's live value and the dimension it is.
    pub values: Vec<(f64, Measure)>,
    /// A line chain goes on from here.
    pub end: Option<(String, (f64, f64))>,
}

/// A point snapped to: its word, where it is, what it's tied by, the rows that needs.
type Snap = (String, (f64, f64), Glyph, Vec<String>);

/// How near an angle (radians, about 3 degrees) a line must be to be taken as level, plumb or
/// parallel.
const INFER: f64 = 0.052;
const FIELD_W: f32 = 74.0;

fn tidy(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// What a dimension measures, in the plane: how it's drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Measure {
    /// Between two points, along this direction.
    Along((f64, f64), (f64, f64), (f64, f64)),
    /// A circle's or arc's centre and radius, and whether it reads as a diameter.
    Round((f64, f64), f64, bool),
    /// About a centre, from one angle to another (radians), at this radius.
    Sweep((f64, f64), f64, f64, f64),
}

/// How dimensions are drawn: which way they stand off, and how far the last ones went.
struct DimPen<'a> {
    p: &'a egui::Painter,
    screen: &'a dyn Fn((f64, f64)) -> Option<Pos2>,
    middle: (f64, f64),
    mm_per_pt: f64,
    ink: Color32,
    /// How many have stood off to each side so far.
    sides: (usize, usize),
}

impl DimPen<'_> {
    fn arrow(&self, tip: Pos2, from: Pos2) {
        let way = (tip - from).normalized();
        let side = vec2(-way.y, way.x);
        self.p.add(egui::Shape::convex_polygon(
            vec![
                tip,
                tip - way * ARROW_PT + side * 3.0,
                tip - way * ARROW_PT - side * 3.0,
            ],
            self.ink,
            Stroke::NONE,
        ));
    }

    /// Draws a dimension's lines; where its value sits.
    fn draw(&mut self, what: Measure) -> Option<Pos2> {
        let (screen, ink) = (self.screen, self.ink);
        let line = Stroke::new(1.0, ink);
        match what {
            Measure::Along(a, b, d) => {
                let span = (b.0 - a.0) * d.0 + (b.1 - a.1) * d.1;
                if span.abs() < 1e-9 {
                    return None;
                }
                // to the side away from the middle, each past the last on that side
                let n = (-d.1, d.0);
                let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
                let outward = (mid.0 - self.middle.0) * n.0 + (mid.1 - self.middle.1) * n.1 >= 0.0;
                let sign = if outward { 1.0 } else { -1.0 };
                let counter = if outward {
                    &mut self.sides.1
                } else {
                    &mut self.sides.0
                };
                let off = (DIM_OFF + DIM_STEP * *counter as f32) as f64 * self.mm_per_pt;
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
                let (ua, ub, wa, wb) = (screen(a)?, screen(b)?, screen(foot(a))?, screen(foot(b))?);
                let faint = Stroke::new(1.0, ink.gamma_multiply(0.5));
                self.p.line_segment([ua, wa], faint);
                self.p.line_segment([ub, wb], faint);
                self.p.line_segment([wa, wb], line);
                self.arrow(wa, wb);
                self.arrow(wb, wa);
                Some(wa + (wb - wa) * 0.5)
            }
            Measure::Round(c, r, _) => {
                let on = (
                    c.0 + r * core::f64::consts::FRAC_1_SQRT_2,
                    c.1 + r * core::f64::consts::FRAC_1_SQRT_2,
                );
                let rim = screen(on)?;
                let out = rim + vec2(30.0, -30.0);
                self.p.line_segment([rim, out], line);
                self.arrow(rim, out);
                Some(out + vec2(10.0, -6.0))
            }
            Measure::Sweep(c, r, t0, t1) => {
                let at = |t: f64| {
                    screen((
                        c.0 + r * forge_math::dmath::cos(t),
                        c.1 + r * forge_math::dmath::sin(t),
                    ))
                };
                let steps = 32;
                let pts: Vec<Pos2> = (0..=steps)
                    .filter_map(|k| at(t0 + (t1 - t0) * k as f64 / steps as f64))
                    .collect();
                if pts.len() < 2 {
                    return None;
                }
                let faint = Stroke::new(1.0, ink.gamma_multiply(0.5));
                if let Some(u) = screen(c) {
                    self.p.line_segment([u, pts[0]], faint);
                    self.p.line_segment([u, pts[pts.len() - 1]], faint);
                }
                self.p.add(egui::Shape::line(pts.clone(), line));
                self.arrow(pts[pts.len() - 1], pts[pts.len() - 2]);
                self.arrow(pts[0], pts[1]);
                at((t0 + t1) * 0.5)
            }
        }
    }
}

/// Dimension lines stand this far off what they measure, then this much more each (points).
const DIM_OFF: f32 = 26.0;
const DIM_STEP: f32 = 22.0;
const ARROW_PT: f32 = 8.0;

/// How close a click must be to take a point or an entity, in points.
const SNAP_PT: f32 = 9.0;
/// How far from where a line would touch a rim it still reads as tangent, in reaches.
const TANGENT_REACH: f64 = 6.0;
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
    /// The length unit it's being read in.
    pub units: &'static str,
    /// The rows it opened with: nothing changed, nothing to commit.
    opened: Vec<String>,
    /// A dimension's row being changed, when it's one already there.
    asking_row: Option<usize>,
    /// The values on screen for what's being drawn.
    pub fields: Vec<Field>,
    /// Which field takes the keyboard next.
    want_focus: Option<usize>,
    /// The field that has the keyboard.
    pub active: usize,
    /// Rows a snapped first point needs written before the entity's.
    first_rows: Vec<String>,
    /// Where the pointer last was on the plane: typing leaves it, the preview stays.
    cursor: Option<(f64, f64)>,
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
            fields: Vec::new(),
            want_focus: None,
            active: 0,
            first_rows: Vec::new(),
            cursor: None,
            rows,
            tool: None,
            picks: Vec::new(),
            pick_at: Vec::new(),
            solved: None,
            error: None,
            asking: None,
            params,
            units: "mm",
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
        self.fields.clear();
        self.first_rows.clear();
    }

    pub fn has_tool(&self) -> bool {
        self.tool.is_some() || !self.picks.is_empty() || self.asking.is_some()
    }

    pub fn has_changes(&self) -> bool {
        self.rows != self.opened
    }

    pub fn is_new(&self) -> bool {
        self.edit == SketchFor::New
    }

    /// Esc: what's half done goes, then the tool.
    pub fn escape(&mut self) {
        if !self.picks.is_empty() || self.asking.is_some() {
            self.picks.clear();
            self.pick_at.clear();
            self.asking = None;
            self.asking_row = None;
            self.fields.clear();
            self.first_rows.clear();
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

    /// A field's value: what's typed if it's locked, a number or an expression over the parameters.
    fn locked(&self, k: usize, want: Want) -> Option<f64> {
        let f = self.fields.get(k).filter(|f| f.locked)?;
        num_of(&self.params)(&self.file_of(f), want).ok()
    }

    /// What's typed, as a row writes it.
    fn file_of(&self, f: &Field) -> String {
        let is_param = |w: &str| self.params.iter().any(|p| p.0 == w);
        let kind = super::panel::field_kind(f.label).unwrap_or(forge_doc::units::Kind::Length);
        forge_doc::units::field_file(&f.text, kind, self.units, &is_param)
    }

    fn next_name(&self, kind: char) -> usize {
        self.solved.as_ref().map_or(1, |s| {
            s.names.iter().filter(|n| n.0.starts_with(kind)).count() + 1
        })
    }

    /// The fields each tool shows at each stage.
    fn fields_for(tool: SkTool, picked: usize) -> Vec<Field> {
        let f = |label| Field {
            label,
            text: String::new(),
            locked: false,
        };
        match (tool, picked) {
            (SkTool::Line, 1) => vec![f("Length"), f("Angle")],
            (SkTool::Rectangle, 1) => vec![f("Width"), f("Height")],
            (SkTool::Circle, 1) => vec![f("Diameter")],
            (SkTool::Arc, 1) => vec![f("Radius")],
            (SkTool::Arc, 2) => vec![f("Sweep")],
            _ => Vec::new(),
        }
    }

    fn stage(&mut self) {
        if let Some(t) = self.tool {
            self.fields = Self::fields_for(t, self.pick_at.len());
            self.want_focus = (!self.fields.is_empty()).then_some(0);
        }
    }

    /// What's under a place: a point of the sketch, a line's middle, or nothing, with the rows it takes.
    fn snap(&self, at: (f64, f64), reach: f64) -> Option<Snap> {
        let d = |p: (f64, f64)| ((p.0 - at.0).powi(2) + (p.1 - at.1).powi(2)).sqrt();
        let near = self
            .points()
            .into_iter()
            .map(|(n, p)| (d(p), n, p))
            .filter(|c| c.0 < reach)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, n, p)) = near {
            return Some((n, p, Glyph::Coincident, Vec::new()));
        }
        let s = self.solved.as_ref()?;
        let mid = s
            .names
            .iter()
            .filter(|n| n.0.starts_with('l'))
            .filter_map(|n| {
                let (a, b) = sk::line(s, &n.0).ok()?;
                let m = ((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                (d(m) < reach).then(|| (n.0.clone(), m))
            })
            .next();
        let (l, m) = mid?;
        // the middle becomes a real point, tied to it, only when it's used
        let p = format!("p{}", self.next_name('p'));
        let rows = vec![format!("pt {} {}", m.0, m.1), format!("mid {p} {l}")];
        Some((p, m, Glyph::Midpoint, rows))
    }

    /// A round thing's rim near this place that a line from `from` can leave tangent: its name and
    /// the point where it touches.
    fn tangent_from(&self, from: (f64, f64), at: (f64, f64), reach: f64) -> Option<(String, (f64, f64))> {
        let s = self.solved.as_ref()?;
        let mut best: Option<(f64, String, (f64, f64))> = None;
        for (name, id) in &s.names {
            let (c, r, span) = match s.solved.entity(*id) {
                Some(Entity::Circle { center, .. }) => (s.solved.point(center), s.solved.radius(*id), None),
                Some(Entity::Arc {
                    center,
                    start,
                    end,
                    ccw,
                }) => {
                    let (a, b) = if ccw { (start, end) } else { (end, start) };
                    let (c, a, b) = (s.solved.point(center), s.solved.point(a), s.solved.point(b));
                    let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                    (c, r, Some((a, b)))
                }
                _ => continue,
            };
            let (vx, vy) = (c.x - from.0, c.y - from.1);
            let dist = (vx * vx + vy * vy).sqrt();
            let off_rim = (((at.0 - c.x).powi(2) + (at.1 - c.y).powi(2)).sqrt() - r).abs();
            if dist <= r + 1e-9 || off_rim > reach {
                continue;
            }
            let leg = (dist * dist - r * r).sqrt();
            let (cos, sin) = (leg / dist, r / dist);
            for side in [-1.0, 1.0] {
                let way = (
                    (vx * cos - side * vy * sin) / dist,
                    (vy * cos + side * vx * sin) / dist,
                );
                let t = (from.0 + way.0 * leg, from.1 + way.1 * leg);
                let gap = ((t.0 - at.0).powi(2) + (t.1 - at.1).powi(2)).sqrt();
                if gap > TANGENT_REACH * reach {
                    continue;
                }
                // on an arc, only where the arc is
                if let Some((a, b)) = span {
                    let turn = |u: (f64, f64)| {
                        forge_math::dmath::atan2(u.1 - c.y, u.0 - c.x).rem_euclid(core::f64::consts::TAU)
                    };
                    let t0 = turn((a.x, a.y));
                    let sweep = (turn((b.x, b.y)) - t0).rem_euclid(core::f64::consts::TAU);
                    if (turn(t) - t0).rem_euclid(core::f64::consts::TAU) > sweep {
                        continue;
                    }
                }
                if best.as_ref().is_none_or(|b| gap < b.0) {
                    best = Some((gap, name.clone(), t));
                }
            }
        }
        best.map(|b| (b.1, b.2))
    }

    /// The other lines of the sketch, with their directions.
    fn line_ways(&self) -> Vec<(String, (f64, f64))> {
        let Some(s) = &self.solved else { return Vec::new() };
        s.names
            .iter()
            .filter(|n| n.0.starts_with('l'))
            .filter_map(|n| {
                let (a, b) = sk::line(s, &n.0).ok()?;
                let (x, y) = (b.x - a.x, b.y - a.y);
                let l = (x * x + y * y).sqrt();
                (l > 1e-9).then(|| (n.0.clone(), (x / l, y / l)))
            })
            .collect()
    }

    /// What the next click would make, with the pointer here.
    pub fn draft(&self, at: (f64, f64), reach: f64) -> Draft {
        let mut d = Draft::default();
        let Some(tool) = self.tool else { return d };
        let n = |v: f64| v.to_string();
        let snapped = self.snap(at, reach);
        if let Some((_, p, g, _)) = &snapped {
            d.glyphs.push((*g, *p));
        }
        let point_of = |d: &mut Draft, free: (f64, f64)| -> (String, (f64, f64)) {
            match &snapped {
                Some((w, p, _, rows)) => {
                    d.rows.extend(rows.iter().cloned());
                    (w.clone(), *p)
                }
                None => {
                    let p = (tidy(free.0), tidy(free.1));
                    (format!("{} {}", n(p.0), n(p.1)), p)
                }
            }
        };
        match (tool, self.pick_at.len()) {
            (SkTool::Point, _) => {
                let (w, p) = point_of(&mut d, at);
                d.rows.push(format!("pt {w}"));
                d.end = Some((String::new(), p));
            }
            (SkTool::Line, 1) => {
                let a = self.pick_at[0];
                let from = self.picks[0].clone();
                let line = format!("l{}", self.next_name('l'));
                let (len_on, ang_on) = (self.locked(0, Want::Len), self.locked(1, Want::Len));
                let mut relation = Vec::new();
                let free = snapped.as_ref().map_or(at, |s| s.1);
                let (mut dx, mut dy) = (free.0 - a.0, free.1 - a.1);
                let mut length = (dx * dx + dy * dy).sqrt();
                let mut typed = false;
                let touching = if snapped.is_none() && ang_on.is_none() && len_on.is_none() {
                    self.tangent_from(a, at, reach)
                } else {
                    None
                };
                if let Some((round, t)) = &touching {
                    (dx, dy) = (t.0 - a.0, t.1 - a.1);
                    length = (dx * dx + dy * dy).sqrt();
                    typed = true;
                    d.glyphs.push((Glyph::Tangent, *t));
                    relation.push(format!("tan {line} {round}"));
                } else if let Some(deg) = ang_on {
                    let r = deg * core::f64::consts::PI / 180.0;
                    (dx, dy) = (
                        forge_math::dmath::cos(r) * length.max(1e-9),
                        forge_math::dmath::sin(r) * length.max(1e-9),
                    );
                    typed = true;
                } else if snapped.is_none() && length > 1e-9 {
                    // level, plumb, or along or square to a line that's there
                    let unit = (dx / length, dy / length);
                    if unit.1.abs() < INFER {
                        dy = 0.0;
                        d.glyphs.push((Glyph::Horizontal, (a.0 + dx, a.1)));
                        relation.push(format!("hor {line}"));
                    } else if unit.0.abs() < INFER {
                        dx = 0.0;
                        d.glyphs.push((Glyph::Vertical, (a.0, a.1 + dy)));
                        relation.push(format!("ver {line}"));
                    } else {
                        for (o, w) in self.line_ways() {
                            let cross = unit.0 * w.1 - unit.1 * w.0;
                            let dot = unit.0 * w.0 + unit.1 * w.1;
                            if cross.abs() < INFER {
                                let s = dx * w.0 + dy * w.1;
                                (dx, dy) = (w.0 * s, w.1 * s);
                                d.glyphs.push((Glyph::Parallel, (a.0 + dx, a.1 + dy)));
                                relation.push(format!("par {line} {o}"));
                                break;
                            }
                            if dot.abs() < INFER {
                                let s = -dx * w.1 + dy * w.0;
                                (dx, dy) = (-w.1 * s, w.0 * s);
                                d.glyphs.push((Glyph::Perpendicular, (a.0 + dx, a.1 + dy)));
                                relation.push(format!("perp {line} {o}"));
                                break;
                            }
                        }
                    }
                    length = (dx * dx + dy * dy).sqrt();
                }
                if let Some(v) = len_on {
                    let l = length.max(1e-9);
                    (dx, dy) = (dx / l * v, dy / l * v);
                    length = v;
                }
                // from a tangent end, within a hair: tangent
                if let Some(arc) = from
                    .strip_suffix(".b")
                    .or(from.strip_suffix(".a"))
                    .filter(|w| w.starts_with('a'))
                    && let Some(s) = &self.solved
                    && let Some((_, id)) = s.names.iter().find(|x| x.0 == arc)
                    && let Some(Entity::Arc { center, .. }) = s.solved.entity(*id)
                {
                    let c = s.solved.point(center);
                    let (rx, ry) = (a.0 - c.x, a.1 - c.y);
                    let r = (rx * rx + ry * ry).sqrt().max(1e-9);
                    if ((dx * rx + dy * ry) / (r * length.max(1e-9))).abs() < INFER {
                        d.glyphs.push((Glyph::Tangent, a));
                        relation.push(format!("tan {line} {arc}"));
                    }
                }
                let end = (a.0 + dx, a.1 + dy);
                let keeps_snap = snapped.is_some() && len_on.is_none() && !typed;
                let (word, end) = if keeps_snap {
                    point_of(&mut d, at)
                } else {
                    let e = if len_on.is_some() || typed {
                        end
                    } else {
                        (tidy(end.0), tidy(end.1))
                    };
                    (format!("{} {}", n(e.0), n(e.1)), e)
                };
                d.rows.extend(self.first_rows.iter().cloned());
                d.rows.push(format!("line {from} {word}"));
                d.rows.extend(relation);
                if let Some(f) = self.fields.first().filter(|f| f.locked) {
                    d.rows.push(format!("dist {line} {}", self.file_of(f)));
                }
                d.lines.push((a, end));
                let way = (dx / length.max(1e-9), dy / length.max(1e-9));
                d.values.push((length, Measure::Along(a, end, way)));
                let angle = forge_math::dmath::atan2(end.1 - a.1, end.0 - a.0);
                d.values.push((
                    angle * 180.0 / core::f64::consts::PI,
                    Measure::Sweep(a, length * 0.5, 0.0, angle),
                ));
                d.end = Some((format!("{line}.b"), end));
            }
            (SkTool::Rectangle, 1) => {
                let a = self.pick_at[0];
                let c = snapped.as_ref().map_or(at, |s| s.1);
                let (sx, sy) = (
                    if c.0 < a.0 { -1.0 } else { 1.0 },
                    if c.1 < a.1 { -1.0 } else { 1.0 },
                );
                let w = self.locked(0, Want::Len).unwrap_or(tidy((c.0 - a.0).abs()));
                let h = self.locked(1, Want::Len).unwrap_or(tidy((c.1 - a.1).abs()));
                let b = (a.0 + sx * w, a.1 + sy * h);
                let (x, y) = (a.0.min(b.0), a.1.min(b.1));
                let l = self.next_name('l');
                d.rows.extend(self.first_rows.iter().cloned());
                d.rows.push(format!("rect {} {} {} {}", n(x), n(y), n(w), n(h)));
                if !self.picks[0].contains(' ') {
                    // the corner it started from is the point it snapped to
                    let corner = match (sx > 0.0, sy > 0.0) {
                        (true, true) => l,
                        (false, true) => l + 1,
                        (false, false) => l + 2,
                        (true, false) => l + 3,
                    };
                    d.rows.push(format!("co l{corner}.a {}", self.picks[0]));
                }
                if let Some(f) = self.fields.first().filter(|f| f.locked) {
                    d.rows.push(format!("hdist l{l}.a l{l}.b {}", self.file_of(f)));
                }
                if let Some(f) = self.fields.get(1).filter(|f| f.locked) {
                    d.rows
                        .push(format!("vdist l{0}.a l{0}.b {1}", l + 1, self.file_of(f)));
                }
                let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
                for k in 0..4 {
                    d.lines.push((corners[k], corners[(k + 1) % 4]));
                }
                d.values.push((w, Measure::Along((x, y), (x + w, y), (1.0, 0.0))));
                d.values
                    .push((h, Measure::Along((x + w, y), (x + w, y + h), (0.0, 1.0))));
                d.end = Some((String::new(), b));
            }
            (SkTool::Circle, 1) => {
                let c = self.pick_at[0];
                let free = snapped.as_ref().map_or(at, |s| s.1);
                let r = self.locked(0, Want::Len).map_or(
                    tidy(((free.0 - c.0).powi(2) + (free.1 - c.1).powi(2)).sqrt()),
                    |v| v / 2.0,
                );
                let name = self.next_name('c');
                d.rows.extend(self.first_rows.iter().cloned());
                d.rows.push(format!("circle {} {}", self.picks[0], n(r)));
                if let Some(f) = self.fields.first().filter(|f| f.locked) {
                    d.rows.push(format!("dia c{name} {}", self.file_of(f)));
                }
                d.circle = Some((c, r));
                d.values.push((r * 2.0, Measure::Round(c, r, true)));
                d.end = Some((String::new(), c));
            }
            (SkTool::Arc, 1) => {
                let c = self.pick_at[0];
                let free = snapped.as_ref().map_or(at, |s| s.1);
                let (rx, ry) = (free.0 - c.0, free.1 - c.1);
                let r = (rx * rx + ry * ry).sqrt();
                let radius = self.locked(0, Want::Len).unwrap_or(r);
                d.circle = Some((c, radius));
                d.values.push((radius, Measure::Round(c, radius, false)));
                let s = radius / r.max(1e-9);
                d.end = Some((String::new(), (c.0 + rx * s, c.1 + ry * s)));
            }
            (SkTool::Arc, 2) => {
                let (c, st) = (self.pick_at[0], self.pick_at[1]);
                let radius = ((st.0 - c.0).powi(2) + (st.1 - c.1).powi(2)).sqrt();
                let t0 = forge_math::dmath::atan2(st.1 - c.1, st.0 - c.0);
                let free = snapped.as_ref().map_or(at, |s| s.1);
                let mut t1 = forge_math::dmath::atan2(free.1 - c.1, free.0 - c.0);
                if t1 <= t0 {
                    t1 += core::f64::consts::TAU;
                }
                let deg = core::f64::consts::PI / 180.0;
                if let Some(v) = self.locked(0, Want::Len) {
                    t1 = t0 + v * deg;
                }
                let end = (
                    c.0 + radius * forge_math::dmath::cos(t1),
                    c.1 + radius * forge_math::dmath::sin(t1),
                );
                d.arc = Some((c, radius, t0, t1));
                d.rows.extend(self.first_rows.iter().cloned());
                d.rows.push(format!(
                    "arc {} {} {} {}",
                    self.picks[0],
                    self.picks[1],
                    n(end.0),
                    n(end.1)
                ));
                d.values
                    .push(((t1 - t0) / deg, Measure::Sweep(c, radius * 1.15, t0, t1)));
                d.end = Some((String::new(), end));
            }
            _ => {}
        }
        d
    }

    /// Places what's drafted, all its rows or none.
    fn place(&mut self, d: Draft) -> bool {
        let mut next = self.rows.clone();
        next.extend(d.rows);
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

    /// Enter: what's drafted goes down where the pointer is.
    pub fn enter(&mut self, reach: f64) {
        if let Some(at) = self.cursor {
            self.click(at, reach);
        }
    }

    /// A click for the tool, at this place in the plane, `reach` being what's near in mm.
    pub fn click(&mut self, at: (f64, f64), reach: f64) {
        let Some(tool) = self.tool else { return };
        match tool {
            SkTool::Point => {
                let d = self.draft(at, reach);
                self.place(d);
            }
            SkTool::Line | SkTool::Rectangle | SkTool::Circle | SkTool::Arc => {
                let first = self.pick_at.is_empty() || (tool == SkTool::Arc && self.pick_at.len() < 2);
                if first {
                    if tool == SkTool::Arc && self.pick_at.len() == 1 {
                        // the start, on the circle the radius says
                        let d = self.draft(at, reach);
                        let Some((_, p)) = d.end else { return };
                        if let Some(f) = self.fields.first().filter(|f| f.locked) {
                            self.first_rows
                                .push(format!("rad a{} {}", self.next_name('a'), self.file_of(f)));
                        }
                        self.picks.push(format!("{} {}", p.0, p.1));
                        self.pick_at.push(p);
                    } else {
                        match self.snap(at, reach) {
                            Some((w, p, _, rows)) => {
                                self.first_rows = rows;
                                self.picks.push(w);
                                self.pick_at.push(p);
                            }
                            None => {
                                let p = (tidy(at.0), tidy(at.1));
                                self.picks.push(format!("{} {}", p.0, p.1));
                                self.pick_at.push(p);
                            }
                        }
                    }
                    self.stage();
                    return;
                }
                let d = self.draft(at, reach);
                let end = d.end.clone();
                // an arc's radius is written after its row
                let trailing: Vec<String> = if tool == SkTool::Arc {
                    std::mem::take(&mut self.first_rows)
                } else {
                    self.first_rows.clear();
                    Vec::new()
                };
                let mut d = d;
                d.rows.extend(trailing);
                if !self.place(d) {
                    return;
                }
                self.picks.clear();
                self.pick_at.clear();
                self.fields.clear();
                // a line chain goes on from where it ended
                if let (SkTool::Line, Some((w, p))) = (tool, end) {
                    self.picks.push(w);
                    self.pick_at.push(p);
                    self.stage();
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

        let under = p.clone();
        let p = ctx
            .layer_painter(egui::LayerId::background())
            .with_clip_rect(clip);
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
        let p = under;
        let ink = t.sketch;
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
        if cursor.is_some() {
            m.cursor = cursor;
        }
        let draft_at = cursor.or(m.cursor);
        if let (Some(tool), Some(at)) = (m.tool, draft_at) {
            let d = m.draft(at, reach);
            let line = Stroke::new(1.5, ink);
            for (a, b) in &d.lines {
                if let (Some(u), Some(v)) = (screen(*a), screen(*b)) {
                    p.line_segment([u, v], line);
                    p.circle_filled(u, 2.5, ink);
                    p.circle_filled(v, 2.5, ink);
                }
            }
            let round = |c: (f64, f64), r: f64, t0: f64, t1: f64| -> Vec<Pos2> {
                (0..=CIRCLE_STEPS)
                    .filter_map(|k| {
                        let v = t0 + (t1 - t0) * k as f64 / CIRCLE_STEPS as f64;
                        screen((
                            c.0 + r * forge_math::dmath::cos(v),
                            c.1 + r * forge_math::dmath::sin(v),
                        ))
                    })
                    .collect()
            };
            if let Some((c, r)) = d.circle {
                p.add(egui::Shape::line(round(c, r, 0.0, core::f64::consts::TAU), line));
                if let Some(u) = screen(c) {
                    p.circle_filled(u, 2.5, ink);
                }
            }
            if let Some((c, r, t0, t1)) = d.arc {
                p.add(egui::Shape::line(round(c, r, t0, t1), line));
            }
            // middles of lines near the pointer, quietly
            if let Some(s) = &m.solved {
                for (name, _) in s.names.iter().filter(|n| n.0.starts_with('l')) {
                    if let Ok((a, b)) = sk::line(s, name)
                        && let Some(u) = screen(((a.x + b.x) * 0.5, (a.y + b.y) * 0.5))
                        && over.is_some_and(|o| (o - u).length() < 60.0)
                    {
                        p.circle_stroke(u, 2.5, Stroke::new(1.0, t.ink2));
                    }
                }
            }
            // what the click will tie it to
            for (k, (g, at)) in d.glyphs.iter().enumerate() {
                let Some(u) = screen(*at) else { continue };
                let name = match g {
                    Glyph::Coincident => "coincident",
                    Glyph::Midpoint => "point",
                    Glyph::Horizontal => "minimize",
                    Glyph::Vertical => "axis",
                    Glyph::Parallel => "parallel",
                    Glyph::Perpendicular => "perp",
                    Glyph::Tangent => "tangent",
                };
                if matches!(g, Glyph::Coincident | Glyph::Midpoint) {
                    p.circle_stroke(u, 5.0, Stroke::new(1.5, t.focus));
                }
                let chip = Rect::from_min_size(u + vec2(10.0 + k as f32 * 20.0, 8.0), vec2(18.0, 18.0));
                p.rect_filled(chip, 3.0, t.surface);
                super::icons::draw(&p, chip.min + vec2(2.0, 2.0), 14.0, name, 1.25, t.focus);
            }
            if m.pick_at.is_empty()
                && matches!(
                    tool,
                    SkTool::Line | SkTool::Rectangle | SkTool::Circle | SkTool::Arc
                )
            {
                let (_, q) = m.word_at(at, reach);
                if let Some(u) = screen(q) {
                    p.circle_filled(u, 2.5, ink);
                }
            }

            // the values are dimensions, live until typed in, each edited where it sits
            let draft_mid = {
                let pts: Vec<(f64, f64)> = d.lines.iter().flat_map(|l| [l.0, l.1]).collect();
                if pts.is_empty() {
                    at
                } else {
                    let n = pts.len() as f64;
                    let sum = pts.iter().fold((0.0, 0.0), |a, q| (a.0 + q.0, a.1 + q.1));
                    (sum.0 / n, sum.1 / n)
                }
            };
            let mut pen = DimPen {
                p: &p,
                screen: &screen,
                middle: draft_mid,
                mm_per_pt,
                ink: t.on_ink,
                sides: (0, 0),
            };
            let values = d.values.clone();
            let n_fields = m.fields.len();
            let units = m.units;
            let self_clear = self.modal_clear();
            if let Some(k) = m.want_focus.take() {
                m.active = k;
            }
            let active = m.active.min(n_fields.saturating_sub(1));
            let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter)) && self_clear;
            for (k, f) in m.fields.iter_mut().enumerate() {
                let Some((value, what)) = values.get(k).copied() else {
                    continue;
                };
                let Some(at_px) = pen.draw(what) else { continue };
                let kind = super::panel::field_kind(f.label).unwrap_or(forge_doc::units::Kind::Length);
                if !f.locked {
                    f.text = forge_doc::units::number_in(value, kind, units);
                }
                let id = egui::Id::unique("sketch-field").with(k);
                egui::Area::new(id.with("area"))
                    .fixed_pos(at_px - vec2(FIELD_W * 0.5, 12.0))
                    .order(egui::Order::Foreground)
                    .fade_in(false)
                    .show(ctx, |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(FIELD_W, 24.0), Sense::hover());
                        ui.visuals_mut().selection.stroke.color = t.on_ink;
                        ui.visuals_mut().selection.bg_fill = t.on;
                        ui.painter().rect_filled(r, 3.0, t.surface);
                        let on = k == active;
                        let edge = if on {
                            t.focus
                        } else if f.locked {
                            t.ink2
                        } else {
                            t.line
                        };
                        // what's live goes when something's typed
                        if k == active && !f.locked {
                            let mut st =
                                egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
                            let all = egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(f.text.chars().count()),
                            );
                            st.cursor.set_char_range(Some(all));
                            st.store(ui.ctx(), id);
                        }
                        let resp = ui.put(
                            r,
                            egui::TextEdit::singleline(&mut f.text)
                                .id(id)
                                .font(mono(12.0))
                                .text_color(t.ink)
                                .align(egui::Align2::CENTER_CENTER)
                                .margin(vec2(6.0, 5.0))
                                .desired_width(FIELD_W - 12.0)
                                .lock_focus(true)
                                .frame(egui::Frame::NONE),
                        );
                        ui.painter()
                            .rect_stroke(r, 3.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
                        self.places.push((format!("sketch:field:{}", f.label), r));
                        if resp.changed() {
                            f.locked = true;
                        }
                        if on && !resp.has_focus() && !resp.lost_focus() {
                            resp.request_focus();
                        }
                    });
            }
            if n_fields > 0 {
                let back = ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
                let on = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                if on || back {
                    m.active = (active + if back { n_fields - 1 } else { 1 }) % n_fields;
                }
            }
            if enter && !m.pick_at.is_empty() {
                m.enter(reach);
            }

            if cursor.is_some() && ctx.input(|i| i.pointer.primary_clicked()) && self.modal_clear() {
                if tool == SkTool::Dimension {
                    if let (Some((row, value)), Some(pos)) = (m.dimension_at(at, reach), over) {
                        let kind = if row.starts_with("ang") {
                            forge_doc::units::Kind::Angle
                        } else {
                            forge_doc::units::Kind::Length
                        };
                        let shown = forge_doc::units::number_in(value, kind, m.units);
                        m.asking = Some((row, pos, shown));
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
        let values_of = m.params.clone();
        let doc_unit = m.units;
        let shown_value = |text: &str, kind: forge_doc::units::Kind| {
            if !forge_doc::units::is_bare(text)
                && text.parse::<f64>().is_err()
                && text.chars().any(|c| c.is_ascii_digit())
                && forge_doc::units::to_person(text) != text
            {
                return forge_doc::units::to_person(text);
            }
            match num_of(&values_of)(text, Want::Len) {
                Ok(v) => format!(
                    "{} {}",
                    forge_doc::units::number_in(v, kind, doc_unit),
                    forge_doc::units::unit_of(kind, doc_unit)
                ),
                Err(_) => text.to_string(),
            }
        };
        let mut ask_chip = None;
        let mut pen = DimPen {
            p: &p,
            screen: &screen,
            middle,
            mm_per_pt,
            ink: t.ink2,
            sides: (0, 0),
        };
        egui::Area::new(egui::Id::unique("sketch-dims"))
            .fixed_pos(view.min)
            .order(egui::Order::Middle)
            .constrain(false)
            .show(ctx, |ui| {
                for (k, words, value, what) in &chips {
                    let Some(pos) = pen.draw(*what) else { continue };
                    let text = match what {
                        Measure::Round(_, _, true) => {
                            format!("\u{D8}{}", shown_value(value, forge_doc::units::Kind::Length))
                        }
                        Measure::Round(_, _, false) => {
                            format!("R{}", shown_value(value, forge_doc::units::Kind::Length))
                        }
                        Measure::Sweep(..) => shown_value(value, forge_doc::units::Kind::Angle),
                        _ => shown_value(value, forge_doc::units::Kind::Length),
                    };
                    let g = ui.fonts_mut(|f| f.layout_no_wrap(text, mono(12.0), t.ink));
                    let r = Rect::from_center_size(pos, g.size() + vec2(10.0, 6.0));
                    let resp = ui.interact(r, ui.scope_id().with(("dim", *k)), Sense::click());
                    // the number's what's shown; what it's written as is here and in its field
                    let resp = if value.parse::<f64>().is_err() {
                        resp.on_hover_text(value)
                    } else {
                        resp
                    };
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
                        let kind = if row.starts_with("ang") {
                            forge_doc::units::Kind::Angle
                        } else {
                            forge_doc::units::Kind::Length
                        };
                        let is_param = |w: &str| values_of.iter().any(|p| p.0 == w);
                        let file = forge_doc::units::field_file(text, kind, doc_unit, &is_param);
                        asked = Some(format!("{row} {file}"));
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
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z))
            && !ctx.egui_wants_keyboard_input()
        {
            m.undo();
        }

        // what state it's in, in the status bar; what's wrong, where it's seen
        let drawing = match m.tool.filter(|_| !m.pick_at.is_empty()) {
            Some(SkTool::Line) => Some("Drawing a line"),
            Some(SkTool::Rectangle) => Some("Drawing a rectangle"),
            Some(SkTool::Circle) => Some("Drawing a circle"),
            Some(SkTool::Arc) => Some("Drawing an arc"),
            _ => None,
        };
        self.status = match m.solved.as_ref().map(|s| sk::freedom(s).0) {
            _ if drawing.is_some() => drawing.unwrap_or_default().to_string(),
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
        m.click((20.1, 10.0), 0.5);
        assert_eq!(m.rows[0], "line 0 0 20 0");
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
    fn the_draft_is_the_entity_as_it_will_be_and_typed_values_lock_it_and_become_dimensions() {
        let params = Document::parse("w = 30\n");
        let mut m = SketchMode::new(
            SketchFor::New,
            "xy".into(),
            Frame::default(),
            Vec::new(),
            Some(&params),
        );
        // a line follows the pointer, with its length and angle live
        m.set_tool(Some(SkTool::Line));
        m.click((0.0, 0.0), 0.5);
        let d = m.draft((30.0, 40.0), 0.5);
        assert_eq!(d.lines, [((0.0, 0.0), (30.0, 40.0))]);
        assert!((d.values[0].0 - 50.0).abs() < 1e-9);
        assert_eq!(m.fields.len(), 2);
        // a parameter typed for its length: the preview obeys, the row says so
        m.fields[0].text = "w".into();
        m.fields[0].locked = true;
        let d = m.draft((30.0, 40.0), 0.5);
        let (_, end) = d.lines[0];
        assert!(
            ((end.0 * end.0 + end.1 * end.1).sqrt() - 30.0).abs() < 1e-9,
            "{end:?}"
        );
        assert!(d.rows.contains(&"dist l1 w".to_string()), "{:?}", d.rows);
        m.click((30.0, 40.0), 0.5);
        assert!(m.error.is_none(), "{:?}", m.error);
        assert!(m.rows.contains(&"dist l1 w".to_string()));
        m.escape();
        m.escape();

        // a rectangle's sides, four, width and height live and typed
        m.set_tool(Some(SkTool::Rectangle));
        m.click((100.0, 100.0), 0.5);
        m.fields[1].text = "12".into();
        m.fields[1].locked = true;
        let d = m.draft((120.0, 130.0), 0.5);
        assert_eq!(d.lines.len(), 4);
        assert_eq!(d.values[1].0, 12.0);
        m.click((120.0, 130.0), 0.5);
        assert!(
            m.rows
                .iter()
                .any(|r| r.starts_with("vdist") && r.ends_with(" 12")),
            "{:?}",
            m.rows
        );
        assert!(m.error.is_none(), "{:?}", m.error);

        // a circle by its diameter
        m.set_tool(Some(SkTool::Circle));
        m.click((200.0, 0.0), 0.5);
        assert_eq!(m.draft((205.0, 0.0), 0.5).circle, Some(((200.0, 0.0), 5.0)));
        m.fields[0].text = "8".into();
        m.fields[0].locked = true;
        m.click((205.0, 0.0), 0.5);
        assert!(m.rows.contains(&"circle 200 0 4".to_string()), "{:?}", m.rows);
        assert!(m.rows.iter().any(|r| r.starts_with("dia c1 8")), "{:?}", m.rows);

        // an arc: its radius, then its sweep
        m.set_tool(Some(SkTool::Arc));
        m.click((300.0, 0.0), 0.5);
        assert_eq!(m.fields[0].label, "Radius");
        m.click((310.0, 0.0), 0.5);
        assert_eq!(m.fields[0].label, "Sweep");
        m.fields[0].text = "90".into();
        m.fields[0].locked = true;
        let d = m.draft((300.0, 50.0), 0.5);
        let (_, r, t0, t1) = d.arc.unwrap();
        assert!((r - 10.0).abs() < 1e-9 && ((t1 - t0) - core::f64::consts::FRAC_PI_2).abs() < 1e-9);
        m.click((300.0, 50.0), 0.5);
        assert!(m.error.is_none(), "{:?}", m.error);
        assert!(m.rows.iter().any(|r| r.starts_with("arc ")), "{:?}", m.rows);
    }

    #[test]
    fn it_infers_as_it_goes_and_applies_on_the_click() {
        let mut m = mode();
        m.rows.push("line 0 0 30 40".into());
        m.solve();
        m.set_tool(Some(SkTool::Line));
        // from the line's far end, over its middle: tied to it, a real point
        m.click((100.0, 0.0), 0.5);
        let d = m.draft((15.1, 20.1), 0.5);
        assert!(d.glyphs.iter().any(|g| g.0 == Glyph::Midpoint), "{:?}", d.glyphs);
        m.click((15.1, 20.1), 0.5);
        assert!(m.rows.contains(&"mid p1 l1".to_string()), "{:?}", m.rows);
        assert!(m.error.is_none(), "{:?}", m.error);
        let (a, b) = sk::line(m.solved.as_ref().unwrap(), "l2").unwrap();
        let _ = a;
        assert!((b.x - 15.0).abs() < 1e-9 && (b.y - 20.0).abs() < 1e-9, "{b:?}");
        m.escape();
        // parallel to it, from somewhere else
        m.click((200.0, 200.0), 0.5);
        let d = m.draft((230.2, 240.1), 0.5);
        assert!(d.glyphs.iter().any(|g| g.0 == Glyph::Parallel), "{:?}", d.glyphs);
        // and level
        let d = m.draft((250.0, 200.5), 0.5);
        assert!(d.glyphs.iter().any(|g| g.0 == Glyph::Horizontal));
        assert_eq!(d.lines[0].1, (250.0, 200.0));
    }

    #[test]
    fn a_line_near_a_rim_leaves_tangent_and_ends_on_it() {
        let mut m = mode();
        m.rows.push("circle 50 0 10".into());
        m.solve();
        m.set_tool(Some(SkTool::Line));
        m.click((0.0, 0.0), 0.5);
        // where it touches, seen from the origin
        let (leg, dist) = ((2400.0f64).sqrt(), 50.0);
        let t = (leg * leg / dist, leg * 10.0 / dist);
        let d = m.draft((t.0 + 0.1, t.1 + 0.1), 0.5);
        assert!(d.glyphs.iter().any(|g| g.0 == Glyph::Tangent), "{:?}", d.glyphs);
        // far from where it would touch, on the rim: nothing inferred
        let d = m.draft((60.0, 0.1), 0.5);
        assert!(!d.glyphs.iter().any(|g| g.0 == Glyph::Tangent), "{:?}", d.glyphs);
        m.click((t.0 + 0.1, t.1 + 0.1), 0.5);
        assert!(m.error.is_none(), "{:?}", m.error);
        assert!(m.rows.iter().any(|r| r.starts_with("tan ")), "{:?}", m.rows);
        let s = m.solved.as_ref().unwrap();
        let line = s
            .names
            .iter()
            .map(|n| n.0.clone())
            .find(|n| n.starts_with('l'))
            .unwrap();
        let (_, b) = sk::line(s, &line).unwrap();
        assert!((b.x - t.0).abs() < 1e-6 && (b.y - t.1).abs() < 1e-6, "{b:?}");
    }

    #[test]
    fn a_midpoint_adds_nothing_to_a_sketch_until_it_is_used() {
        let mut m = mode();
        m.rows.push("line 0 0 30 40".into());
        m.solve();
        let before = m.rows.clone();
        m.set_tool(Some(SkTool::Line));
        let d = m.draft((15.0, 20.0), 0.5);
        assert!(d.glyphs.iter().any(|g| g.0 == Glyph::Midpoint));
        assert_eq!(m.rows, before, "looking at one writes nothing");
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
