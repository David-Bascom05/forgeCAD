//! The Sketch tab's tools beyond line, circle, arc, rectangle and spline, as Inventor has them:
//! shapes from clicked points (three-point circle and arc, tangent arc, centre and three-point
//! rectangle, slots, polygon, ellipse), corner fillet and chamfer, text, the Modify panel (move,
//! copy, rotate, scale, mirror, offset, patterns, trim, extend, split) and every constraint. Each
//! writes sketch rows; the solver holds them.

use super::{Draft, Glyph, SkTool, SketchMode, tidy};
use forge_doc::sketch as sk;
use forge_doc::text::tokens;
use forge_math::dmath;
use forge_sketch::Entity;

type P = (f64, f64);

/// The Modify and Format state a sketch keeps between clicks.
#[derive(Clone, Debug, Default)]
pub struct More {
    /// The geometry chosen for a Modify tool.
    pub sel: Vec<String>,
    /// The choosing's done (Enter, or a click on nothing): points or a line come next.
    pub placing: bool,
    /// Constraint marks hidden.
    pub hide_glyphs: bool,
    /// Each entity's freedom shown.
    pub show_dof: bool,
    /// New lines are construction lines.
    pub construction: bool,
    /// Text placed: the point it stands on and what it says, for Emboss after the sketch.
    pub texts: Vec<(String, String)>,
    /// How many copies a pattern makes, sides a polygon has (unset: Inventor's default).
    pub count: usize,
    /// A corner fillet's radius or chamfer's leg (unset: a fifth of the shorter side).
    pub size: f64,
}

fn n(v: f64) -> String {
    forge_agent::short(tidy(v))
}

fn pt(p: P) -> String {
    format!("{} {}", n(p.0), n(p.1))
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

fn len(a: P) -> f64 {
    (a.0 * a.0 + a.1 * a.1).sqrt()
}

fn dist(a: P, b: P) -> f64 {
    len(sub(a, b))
}

/// The circle through three points: centre and radius, none if they're in a line.
pub fn circumcircle(a: P, b: P, c: P) -> Option<(P, f64)> {
    let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
    if d.abs() < 1e-9 {
        return None;
    }
    let s = |p: P| p.0 * p.0 + p.1 * p.1;
    let x = (s(a) * (b.1 - c.1) + s(b) * (c.1 - a.1) + s(c) * (a.1 - b.1)) / d;
    let y = (s(a) * (c.0 - b.0) + s(b) * (a.0 - c.0) + s(c) * (b.0 - a.0)) / d;
    Some(((x, y), dist((x, y), a)))
}

/// Where two infinite lines (point, direction) cross.
fn cross_lines(p: P, u: P, q: P, v: P) -> Option<P> {
    let den = u.0 * v.1 - u.1 * v.0;
    if den.abs() < 1e-12 {
        return None;
    }
    let w = sub(q, p);
    let t = (w.0 * v.1 - w.1 * v.0) / den;
    Some((p.0 + u.0 * t, p.1 + u.1 * t))
}

/// How many points a shape tool takes.
fn points_wanted(t: SkTool) -> Option<usize> {
    Some(match t {
        SkTool::Circle3 | SkTool::Arc3 | SkTool::Rect3 | SkTool::Ellipse => 3,
        SkTool::SlotCenter | SkTool::SlotOverall | SkTool::SlotCC => 3,
        SkTool::ArcTan | SkTool::RectCenter | SkTool::Polygon => 2,
        _ => return None,
    })
}

/// A shape's preview and rows from its points.
struct Shape {
    rows: Vec<String>,
    lines: Vec<(P, P)>,
    circle: Option<(P, f64)>,
    arcs: Vec<(P, f64, f64, f64)>,
}

impl Shape {
    fn new() -> Shape {
        Shape {
            rows: vec![],
            lines: vec![],
            circle: None,
            arcs: vec![],
        }
    }
}

/// An arc's preview from its centre, start, end and way.
fn arc_of(c: P, s: P, e: P, ccw: bool) -> (P, f64, f64, f64) {
    let r = dist(c, s);
    let (mut t0, mut t1) = (
        dmath::atan2(s.1 - c.1, s.0 - c.0),
        dmath::atan2(e.1 - c.1, e.0 - c.0),
    );
    if !ccw {
        std::mem::swap(&mut t0, &mut t1);
    }
    if t1 <= t0 {
        t1 += core::f64::consts::TAU;
    }
    (c, r, t0, t1)
}

impl SketchMode {
    /// The next free name of a kind, `k` after the first.
    fn name_after(&self, kind: char, k: usize) -> String {
        format!("{kind}{}", self.next_name(kind) + k)
    }

    /// A point's word: the sketch's point there if one's within reach, else its numbers.
    fn word(&self, at: P, reach: f64) -> (String, P) {
        self.word_at(at, reach)
    }

    /// A shape tool's rows and preview from its points (the pointer's last).
    fn shape(&self, tool: SkTool, p: &[P], words: &[String]) -> Option<Shape> {
        let mut s = Shape::new();
        let w = |k: usize| words.get(k).cloned().unwrap_or_else(|| pt(p[k]));
        match tool {
            SkTool::Circle3 => {
                let (c, r) = circumcircle(p[0], p[1], p[2])?;
                s.rows.push(format!("circle {} {}", pt(c), n(r)));
                s.circle = Some((c, r));
            }
            SkTool::Arc3 => {
                // start, end, then a point it passes through
                let (c, _) = circumcircle(p[0], p[1], p[2])?;
                let side = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
                let ccw = side < 0.0;
                s.rows.push(format!(
                    "arc {} {} {}{}",
                    pt(c),
                    w(0),
                    w(1),
                    if ccw { "" } else { " cw" }
                ));
                s.arcs.push(arc_of(c, p[0], p[1], ccw));
            }
            SkTool::ArcTan => {
                // from a line's or arc's end, leaving along it
                let from = words.first()?;
                let way = self.leaving(from)?;
                let normal = (-way.1, way.0);
                let chord = sub(p[1], p[0]);
                let mid = ((p[0].0 + p[1].0) * 0.5, (p[0].1 + p[1].1) * 0.5);
                let c = cross_lines(p[0], normal, mid, (-chord.1, chord.0))?;
                let ccw = way.0 * chord.1 - way.1 * chord.0 > 0.0;
                let arc = self.name_after('a', 0);
                s.rows.push(format!(
                    "arc {} {from} {}{}",
                    pt(c),
                    pt(p[1]),
                    if ccw { "" } else { " cw" }
                ));
                s.rows.push(format!("tan {} {arc}", from.split('.').next()?));
                s.arcs.push(arc_of(c, p[0], p[1], ccw));
            }
            SkTool::RectCenter => {
                let (c, k) = (p[0], p[1]);
                let (hw, hh) = ((k.0 - c.0).abs(), (k.1 - c.1).abs());
                if hw < 1e-9 || hh < 1e-9 {
                    return None;
                }
                s.rows.push(format!(
                    "rect {} {} {} {}",
                    n(c.0 - hw),
                    n(c.1 - hh),
                    n(2.0 * hw),
                    n(2.0 * hh)
                ));
                let q = [
                    (c.0 - hw, c.1 - hh),
                    (c.0 + hw, c.1 - hh),
                    (c.0 + hw, c.1 + hh),
                    (c.0 - hw, c.1 + hh),
                ];
                for i in 0..4 {
                    s.lines.push((q[i], q[(i + 1) % 4]));
                }
            }
            SkTool::Rect3 => {
                let u = sub(p[1], p[0]);
                let l = len(u);
                if l < 1e-9 {
                    return None;
                }
                let nrm = (-u.1 / l, u.0 / l);
                let h = (p[2].0 - p[1].0) * nrm.0 + (p[2].1 - p[1].1) * nrm.1;
                if h.abs() < 1e-9 {
                    return None;
                }
                let q2 = (p[1].0 + nrm.0 * h, p[1].1 + nrm.1 * h);
                let q3 = (p[0].0 + nrm.0 * h, p[0].1 + nrm.1 * h);
                let l0 = self.next_name('l');
                s.rows
                    .push(format!("poly {} {} {} {}", w(0), w(1), pt(q2), pt(q3)));
                s.rows.push(format!("perp l{l0} l{}", l0 + 1));
                s.rows.push(format!("par l{l0} l{}", l0 + 2));
                s.rows.push(format!("par l{} l{}", l0 + 1, l0 + 3));
                let q = [p[0], p[1], q2, q3];
                for i in 0..4 {
                    s.lines.push((q[i], q[(i + 1) % 4]));
                }
            }
            SkTool::SlotCC | SkTool::SlotOverall | SkTool::SlotCenter => {
                let (a, b) = match tool {
                    SkTool::SlotCC => (p[0], p[1]),
                    SkTool::SlotCenter => ((2.0 * p[0].0 - p[1].0, 2.0 * p[0].1 - p[1].1), p[1]),
                    _ => (p[0], p[1]),
                };
                let u = sub(b, a);
                let l = len(u);
                if l < 1e-9 {
                    return None;
                }
                let d = (u.0 / l, u.1 / l);
                let nrm = (-d.1, d.0);
                let r = ((p[2].0 - a.0) * nrm.0 + (p[2].1 - a.1) * nrm.1).abs();
                if r < 1e-9 {
                    return None;
                }
                // overall: the clicks are its ends; the centres stand r inside
                let (a, b) = if tool == SkTool::SlotOverall {
                    if l <= 2.0 * r {
                        return None;
                    }
                    ((a.0 + d.0 * r, a.1 + d.1 * r), (b.0 - d.0 * r, b.1 - d.1 * r))
                } else {
                    (a, b)
                };
                let off = |c: P, k: f64| (c.0 + nrm.0 * r * k, c.1 + nrm.1 * r * k);
                let (l1, l2) = (self.name_after('l', 0), self.name_after('l', 1));
                let (a1, a2) = (self.name_after('a', 0), self.name_after('a', 1));
                s.rows
                    .push(format!("line {} {}", pt(off(a, 1.0)), pt(off(b, 1.0))));
                s.rows
                    .push(format!("line {} {}", pt(off(b, -1.0)), pt(off(a, -1.0))));
                // round the ends, clockwise from the left side to the right
                s.rows.push(format!("arc {} {l1}.b {l2}.a cw", pt(b)));
                s.rows.push(format!("arc {} {l2}.b {l1}.a cw", pt(a)));
                s.rows.extend([
                    format!("tan {l1} {a1}"),
                    format!("tan {l2} {a1}"),
                    format!("tan {l2} {a2}"),
                    format!("tan {l1} {a2}"),
                ]);
                s.lines.push((off(a, 1.0), off(b, 1.0)));
                s.lines.push((off(b, -1.0), off(a, -1.0)));
                s.arcs.push(arc_of(b, off(b, -1.0), off(b, 1.0), true));
                s.arcs.push(arc_of(a, off(a, 1.0), off(a, -1.0), true));
            }
            SkTool::Polygon => {
                let sides = if self.more.count == 0 {
                    6
                } else {
                    self.more.count.max(3)
                };
                let (c, v) = (p[0], p[1]);
                let r = dist(c, v);
                if r < 1e-9 {
                    return None;
                }
                let t0 = dmath::atan2(v.1 - c.1, v.0 - c.0);
                let q: Vec<P> = (0..sides)
                    .map(|k| {
                        let t = t0 + core::f64::consts::TAU * k as f64 / sides as f64;
                        (c.0 + r * dmath::cos(t), c.1 + r * dmath::sin(t))
                    })
                    .collect();
                let l0 = self.next_name('l');
                s.rows.push(format!(
                    "poly {}",
                    q.iter().map(|x| pt(*x)).collect::<Vec<_>>().join(" ")
                ));
                let rest: Vec<String> = (1..sides).map(|k| format!("l{}", l0 + k)).collect();
                s.rows.push(format!("eq l{l0} {}", rest.join(" ")));
                for i in 0..sides {
                    s.lines.push((q[i], q[(i + 1) % sides]));
                }
            }
            SkTool::Ellipse => {
                let (c, m) = (p[0], p[1]);
                let u = sub(m, c);
                let a = len(u);
                if a < 1e-9 {
                    return None;
                }
                let b = ((p[2].0 - c.0) * -u.1 / a + (p[2].1 - c.1) * u.0 / a).abs();
                if b < 1e-9 {
                    return None;
                }
                s.rows.push(format!("ellipse {} {} {}", pt(c), pt(m), n(b)));
                let t = dmath::atan2(u.1, u.0);
                let pts: Vec<P> = (0..=64)
                    .map(|k| {
                        let v = core::f64::consts::TAU * k as f64 / 64.0;
                        let (x, y) = (a * dmath::cos(v), b * dmath::sin(v));
                        (
                            c.0 + x * dmath::cos(t) - y * dmath::sin(t),
                            c.1 + x * dmath::sin(t) + y * dmath::cos(t),
                        )
                    })
                    .collect();
                for k in 0..64 {
                    s.lines.push((pts[k], pts[k + 1]));
                }
            }
            _ => return None,
        }
        Some(s)
    }

    /// The direction a line or arc leaves its end by, for a tangent arc started there.
    fn leaving(&self, end: &str) -> Option<P> {
        let (e, which) = end.split_once('.')?;
        let s = self.solved.as_ref()?;
        let (_, id) = s.names.iter().find(|x| x.0 == e)?;
        let g = &s.solved;
        match g.entity(*id)? {
            Entity::Line { a, b } => {
                let (pa, pb) = (g.point(a), g.point(b));
                let u = if which == "b" {
                    (pb.x - pa.x, pb.y - pa.y)
                } else {
                    (pa.x - pb.x, pa.y - pb.y)
                };
                let l = len(u);
                (l > 1e-12).then(|| (u.0 / l, u.1 / l))
            }
            Entity::Arc {
                center,
                start,
                end: stop,
                ccw,
            } => {
                let c = g.point(center);
                let at = g.point(if which == "b" { stop } else { start });
                let r = (at.x - c.x, at.y - c.y);
                // along the arc's way at its end, back against it at its start
                let k = if (which == "b") == ccw { 1.0 } else { -1.0 };
                let l = len(r).max(1e-12);
                Some((-r.1 / l * k, r.0 / l * k))
            }
            _ => None,
        }
    }

    /// What a tool beyond the first ones would make with the pointer here.
    pub(super) fn more_draft(&self, tool: SkTool, at: P, reach: f64, d: &mut Draft) {
        if let Some(want) = points_wanted(tool) {
            let mut p = self.pick_at.clone();
            let mut words = self.picks.clone();
            let (w, q) = self.word(at, reach);
            if q != at {
                d.glyphs.push((Glyph::Coincident, q));
            }
            p.push(q);
            words.push(w);
            if p.len() == want {
                if let Some(s) = self.shape(tool, &p, &words) {
                    d.rows = s.rows;
                    d.lines = s.lines;
                    d.circle = s.circle;
                    d.arc = s.arcs.first().copied();
                    d.arcs = s.arcs.iter().skip(1).copied().collect();
                }
            } else {
                for w in p.windows(2) {
                    d.lines.push((w[0], w[1]));
                }
            }
            return;
        }
        // a modify tool shows where its copies or moved geometry would go
        if let Some(rows) = self.modify_rows(tool, Some(at), reach) {
            d.lines = self.outline_of_rows(&rows);
        }
    }

    /// The outlines of geometry rows, as segments, for a preview.
    fn outline_of_rows(&self, rows: &[String]) -> Vec<(P, P)> {
        let mut all = self.rows.clone();
        all.extend(rows.iter().cloned());
        let Ok(s) = self.build(&all) else {
            return Vec::new();
        };
        let before: Vec<&String> = self
            .solved
            .as_ref()
            .map_or(vec![], |x| x.names.iter().map(|n| &n.0).collect());
        let mut out = vec![];
        let shown = SketchMode {
            solved: Some(s.clone()),
            ..self.clone()
        };
        for (name, _) in &s.names {
            if before.contains(&name) && !self.more.sel.contains(name) {
                continue;
            }
            let pts = shown.outline_of(name);
            for w in pts.windows(2) {
                out.push((w[0], w[1]));
            }
        }
        out
    }

    /// A click for a tool beyond the first ones. True when it was one of them.
    pub(super) fn more_click(&mut self, tool: SkTool, at: P, reach: f64) -> bool {
        if let Some(want) = points_wanted(tool) {
            let (w, q) = self.word(at, reach);
            if tool == SkTool::ArcTan && self.picks.is_empty() && self.leaving(&w).is_none() {
                self.error = Some("a tangent arc starts at the end of a line or an arc".into());
                return true;
            }
            self.picks.push(w);
            self.pick_at.push(q);
            if self.pick_at.len() == want {
                let p = std::mem::take(&mut self.pick_at);
                let words = std::mem::take(&mut self.picks);
                match self.shape(tool, &p, &words) {
                    Some(s) => {
                        self.add_all(s.rows);
                    }
                    None => self.error = Some("those points make no shape".into()),
                }
            }
            return true;
        }
        match tool {
            SkTool::Horizontal | SkTool::Vertical => {
                let word = if tool == SkTool::Horizontal { "hor" } else { "ver" };
                if self.picks.is_empty()
                    && let Some(l) = self.entity_near(at, reach, "l")
                {
                    self.add(format!("{word} {l}"));
                    return true;
                }
                self.two_points(word, at, reach);
            }
            SkTool::Collinear => self.two_of("colin", "l", at, reach),
            SkTool::Concentric => self.two_of("conc", "ca", at, reach),
            SkTool::Equal => {
                let kinds = match self.picks.first().and_then(|p| p.chars().next()) {
                    Some('l') => "l",
                    Some(_) => "ca",
                    None => "lca",
                };
                self.two_of("eq", kinds, at, reach);
            }
            SkTool::Fix => {
                let (w, _) = self.word(at, reach);
                let pick = if w.contains(' ') {
                    self.entity_near(at, reach, "lcase")
                } else {
                    Some(w)
                };
                if let Some(x) = pick {
                    self.add(format!("fix {x}"));
                }
            }
            SkTool::Symmetric => {
                if self.picks.len() < 2 {
                    let (w, _) = self.word(at, reach);
                    let pick = if w.contains(' ') {
                        self.entity_near(at, reach, "lca")
                    } else {
                        Some(w)
                    };
                    if let Some(x) = pick {
                        self.picks.push(x);
                    }
                } else if let Some(l) = self.entity_near(at, reach, "l") {
                    let (a, b) = (self.picks[0].clone(), self.picks[1].clone());
                    self.picks.clear();
                    let rows =
                        if a.starts_with('l') && !a.contains('.') && b.starts_with('l') && !b.contains('.') {
                            vec![format!("sym {a}.a {b}.a {l}"), format!("sym {a}.b {b}.b {l}")]
                        } else {
                            vec![format!("sym {a} {b} {l}")]
                        };
                    self.add_all(rows);
                }
            }
            SkTool::Text => {
                let (w, q) = self.word(at, reach);
                let p = if w.contains(' ') {
                    let name = self.name_after('p', 0);
                    if !self.add(format!("pt {}", pt(q))) {
                        return true;
                    }
                    name
                } else {
                    w
                };
                self.more.texts.push((p, "TEXT".into()));
            }
            SkTool::SkFillet | SkTool::SkChamfer => self.corner(tool, at, reach),
            SkTool::Trim | SkTool::Extend | SkTool::Split => self.cut_line(tool, at, reach),
            _ => {
                if !self.more.placing {
                    match self.entity_near(at, reach, "lcasp") {
                        Some(e) => {
                            if let Some(k) = self.more.sel.iter().position(|x| *x == e) {
                                self.more.sel.remove(k);
                            } else {
                                self.more.sel.push(e);
                            }
                            return true;
                        }
                        // a click on nothing ends the choosing; for point tools it's the first point
                        None if !self.more.sel.is_empty() => {
                            self.more.placing = true;
                            if tool == SkTool::Mirror {
                                return true;
                            }
                        }
                        None => return true,
                    }
                }
                self.place_modify(tool, at, reach);
            }
        }
        true
    }

    fn two_points(&mut self, word: &str, at: P, reach: f64) {
        let (w, _) = self.word(at, reach);
        if w.contains(' ') {
            return;
        }
        self.picks.push(w);
        if self.picks.len() == 2 {
            let row = format!("{word} {} {}", self.picks[0], self.picks[1]);
            self.picks.clear();
            self.add(row);
        }
    }

    fn two_of(&mut self, word: &str, kinds: &str, at: P, reach: f64) {
        let Some(e) = self.entity_near(at, reach, kinds) else {
            return;
        };
        if self.picks.contains(&e) {
            return;
        }
        self.picks.push(e);
        if self.picks.len() == 2 {
            let row = format!("{word} {} {}", self.picks[0], self.picks[1]);
            self.picks.clear();
            self.add(row);
        }
    }

    /// Adds rows if the sketch still solves with them all; none if not.
    pub(super) fn add_all(&mut self, rows: Vec<String>) -> bool {
        let rows: Vec<String> = rows
            .into_iter()
            .map(|r| {
                if self.more.construction && r.starts_with("line ") {
                    format!("{r} cons")
                } else {
                    r
                }
            })
            .collect();
        let mut next = self.rows.clone();
        next.extend(rows);
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

    /// The row that makes a named entity, by counting names as the rows make them.
    pub(super) fn row_of(&self, name: &str) -> Option<usize> {
        let kind = name.chars().next()?;
        let k: usize = name[1..].parse().ok()?;
        let mut seen = 0usize;
        for (i, r) in self.rows.iter().enumerate() {
            let t = tokens(r);
            let made = match (t.first()?.as_str(), kind) {
                ("line", 'l') | ("circle", 'c') | ("arc", 'a') | ("pt", 'p') => 1,
                ("spline" | "bspline", 's') | ("ellipse" | "earc", 'e') => 1,
                ("rect", 'l') => 4,
                ("poly", 'l') => {
                    let open = t.last().is_some_and(|w| w == "open");
                    let (mut j, mut pts) = (1, 0);
                    while j < t.len() - usize::from(open) {
                        j += if t[j].parse::<f64>().is_ok() { 2 } else { 1 };
                        pts += 1;
                    }
                    pts - usize::from(open)
                }
                _ => 0,
            };
            if seen + made >= k && made > 0 {
                return Some(i);
            }
            seen += made;
        }
        None
    }

    /// Fresh rows for chosen geometry as it stands, each point moved by `f`; points the
    /// originals share are shared by the copies. With `tie`, each copied point is tied to its
    /// original by that row word and line (a mirror's `sym`).
    fn copy_rows(&self, f: &dyn Fn(P) -> P, flip: bool, tie: Option<&str>) -> Vec<String> {
        let Some(s) = &self.solved else { return vec![] };
        let g = &s.solved;
        let mut rows = vec![];
        // each copied point's word, by where its original is
        let mut made: Vec<(P, String)> = vec![];
        let mut counts: std::collections::HashMap<char, usize> = Default::default();
        let mut name = |k: char| {
            let c = counts.entry(k).or_default();
            *c += 1;
            self.name_after(k, *c - 1)
        };
        let mut ties = vec![];
        for e in &self.more.sel {
            let Some((_, id)) = s.names.iter().find(|x| x.0 == *e) else {
                continue;
            };
            let p = |q: forge_math::Point2| (q.x, q.y);
            match g.entity(*id) {
                Some(Entity::Line { a, b }) => {
                    let me = name('l');
                    let (pa, pb) = (p(g.point(a)), p(g.point(b)));
                    rows.push(format!("line {} {}", pt(f(pa)), pt(f(pb))));
                    for (q, end) in [(pa, "a"), (pb, "b")] {
                        if let Some(w) = made.iter().find(|m| dist(m.0, q) < 1e-9) {
                            ties.push(format!("co {me}.{end} {}", w.1));
                        } else {
                            made.push((q, format!("{me}.{end}")));
                        }
                    }
                    if let Some(t) = tie {
                        ties.push(format!("sym {e}.a {me}.a {t}"));
                        ties.push(format!("sym {e}.b {me}.b {t}"));
                    }
                }
                Some(Entity::Circle { center, .. }) => {
                    let me = name('c');
                    rows.push(format!(
                        "circle {} {}",
                        pt(f(p(g.point(center)))),
                        n(g.radius(*id))
                    ));
                    if let Some(t) = tie {
                        ties.push(format!("sym {e} {me} {t}"));
                    }
                }
                Some(Entity::Arc {
                    center,
                    start,
                    end,
                    ccw,
                }) => {
                    let me = name('a');
                    let (c, a0, b0) = (p(g.point(center)), p(g.point(start)), p(g.point(end)));
                    // mirrored, an arc runs the other way
                    let (a0, b0) = if flip { (b0, a0) } else { (a0, b0) };
                    rows.push(format!(
                        "arc {} {} {}{}",
                        pt(f(c)),
                        pt(f(a0)),
                        pt(f(b0)),
                        if ccw { "" } else { " cw" }
                    ));
                    for (q, end) in [(a0, "a"), (b0, "b")] {
                        if let Some(w) = made.iter().find(|m| dist(m.0, q) < 1e-9) {
                            ties.push(format!("co {me}.{end} {}", w.1));
                        } else {
                            made.push((q, format!("{me}.{end}")));
                        }
                    }
                    if let Some(t) = tie {
                        ties.push(format!("sym {e} {me} {t}"));
                    }
                }
                Some(Entity::Point { .. }) => {
                    let me = name('p');
                    rows.push(format!("pt {}", pt(f(p(g.point(*id))))));
                    if let Some(t) = tie {
                        ties.push(format!("sym {e} {me} {t}"));
                    }
                }
                Some(Entity::Spline { .. }) => {
                    name('s');
                    if let Some(pts) = g.spline_points(*id) {
                        let ws: Vec<String> = pts.iter().map(|q| pt(f(p(g.point(*q))))).collect();
                        rows.push(format!("spline {} open", ws.join(" ")));
                    }
                }
                _ => {}
            }
        }
        rows.extend(ties);
        rows
    }

    /// The rows a Modify tool makes, with the pointer at `at` for its last point.
    fn modify_rows(&self, tool: SkTool, at: Option<P>, reach: f64) -> Option<Vec<String>> {
        if self.more.sel.is_empty() || !self.more.placing {
            return None;
        }
        let mut p = self.pick_at.clone();
        if let Some(a) = at {
            p.push(self.word(a, reach).1);
        }
        let count = if self.more.count == 0 {
            3
        } else {
            self.more.count.max(2)
        };
        Some(match (tool, p.len()) {
            (SkTool::Move | SkTool::Copy, 2) => {
                let v = sub(p[1], p[0]);
                let mv = move |q: P| (q.0 + v.0, q.1 + v.1);
                if tool == SkTool::Move {
                    return Some(self.moved_rows(&mv, 1.0));
                }
                self.copy_rows(&mv, false, None)
            }
            (SkTool::Rotate, 3) => {
                let (c, a, b) = (p[0], p[1], p[2]);
                let t = dmath::atan2(b.1 - c.1, b.0 - c.0) - dmath::atan2(a.1 - c.1, a.0 - c.0);
                let (sn, cs) = (dmath::sin(t), dmath::cos(t));
                let rot = move |q: P| {
                    let (x, y) = (q.0 - c.0, q.1 - c.1);
                    (c.0 + x * cs - y * sn, c.1 + x * sn + y * cs)
                };
                return Some(self.moved_rows(&rot, 1.0));
            }
            (SkTool::Scale, 3) => {
                let (c, a, b) = (p[0], p[1], p[2]);
                let k = dist(c, b) / dist(c, a).max(1e-12);
                let sc = move |q: P| (c.0 + (q.0 - c.0) * k, c.1 + (q.1 - c.1) * k);
                return Some(self.moved_rows(&sc, k));
            }
            (SkTool::Offset, 1) => {
                let (dst, side) = self.offset_at(p[0])?;
                return Some(self.offset_rows(dst * side));
            }
            (SkTool::RectPattern, 2) => {
                let v = sub(p[1], p[0]);
                let mut rows = vec![];
                let mut shown = self.clone();
                for k in 1..count {
                    let k = k as f64;
                    let mv = move |q: P| (q.0 + v.0 * k, q.1 + v.1 * k);
                    let r = shown.copy_rows(&mv, false, None);
                    shown.rows.extend(r.iter().cloned());
                    shown.solved = shown.build(&shown.rows).ok();
                    rows.extend(r);
                }
                rows
            }
            (SkTool::CircPattern, 1) => {
                let c = p[0];
                let mut rows = vec![];
                let mut shown = self.clone();
                for k in 1..count {
                    let t = core::f64::consts::TAU * k as f64 / count as f64;
                    let (sn, cs) = (dmath::sin(t), dmath::cos(t));
                    let rot = move |q: P| {
                        let (x, y) = (q.0 - c.0, q.1 - c.1);
                        (c.0 + x * cs - y * sn, c.1 + x * sn + y * cs)
                    };
                    let r = shown.copy_rows(&rot, false, None);
                    shown.rows.extend(r.iter().cloned());
                    shown.solved = shown.build(&shown.rows).ok();
                    rows.extend(r);
                }
                rows
            }
            _ => return None,
        })
    }

    /// The rows as they'd be with the chosen geometry's own numbers moved by `f`: what follows
    /// it by name moves with it. Returns the whole new set of rows.
    fn moved_rows(&self, f: &dyn Fn(P) -> P, scale: f64) -> Vec<String> {
        let mut rows = self.rows.clone();
        let mut done = vec![];
        for e in &self.more.sel {
            let Some(k) = self.row_of(e) else { continue };
            if done.contains(&k) {
                continue;
            }
            done.push(k);
            let mut t = tokens(&rows[k]);
            let num = |s: &str| s.parse::<f64>().ok();
            let kind = t[0].clone();
            let mut i = 1;
            // every number pair that is a point is moved; a circle's radius is scaled
            while i < t.len() {
                if let (Some(x), Some(y)) = (num(&t[i]), t.get(i + 1).and_then(|s| num(s))) {
                    if kind == "rect" {
                        let q = f((x, y));
                        t[i] = n(q.0);
                        t[i + 1] = n(q.1);
                        if let (Some(w), Some(h)) =
                            (t.get(3).and_then(|s| num(s)), t.get(4).and_then(|s| num(s)))
                        {
                            t[3] = n(w * scale);
                            t[4] = n(h * scale);
                        }
                        break;
                    }
                    let q = f((x, y));
                    t[i] = n(q.0);
                    t[i + 1] = n(q.1);
                    i += 2;
                    if kind == "circle"
                        && let Some(r) = t.get(i).and_then(|s| num(s))
                    {
                        t[i] = n(r * scale);
                        i += 1;
                    }
                } else {
                    // a named point: one token; the circle's radius after one too
                    i += 1;
                    if kind == "circle"
                        && i == 2
                        && let Some(r) = t.get(i).and_then(|s| num(s))
                    {
                        t[i] = n(r * scale);
                        i += 1;
                    }
                }
            }
            rows[k] = t.join(" ");
        }
        rows
    }

    /// How far the pointer is from the first chosen line or circle, and which side.
    fn offset_at(&self, at: P) -> Option<(f64, f64)> {
        let s = self.solved.as_ref()?;
        let e = self.more.sel.first()?;
        let (_, id) = s.names.iter().find(|x| x.0 == *e)?;
        let g = &s.solved;
        match g.entity(*id)? {
            Entity::Line { a, b } => {
                let (pa, pb) = (g.point(a), g.point(b));
                let u = (pb.x - pa.x, pb.y - pa.y);
                let l = len(u).max(1e-12);
                let d = ((at.0 - pa.x) * -u.1 + (at.1 - pa.y) * u.0) / l;
                Some((tidy(d.abs()), d.signum()))
            }
            Entity::Circle { center, .. } | Entity::Arc { center, .. } => {
                let c = g.point(center);
                let d = dist(at, (c.x, c.y)) - g.radius(*id);
                Some((tidy(d.abs()), d.signum()))
            }
            _ => None,
        }
    }

    /// Chosen lines shifted `d` to their left (a chain's corners met again), circles and arcs
    /// grown by `d`, each tied by a distance to its original.
    fn offset_rows(&self, d: f64) -> Vec<String> {
        let Some(s) = &self.solved else { return vec![] };
        let g = &s.solved;
        let mut rows = vec![];
        let mut ls: Vec<(P, P)> = vec![];
        for e in &self.more.sel {
            let Some((_, id)) = s.names.iter().find(|x| x.0 == *e) else {
                continue;
            };
            match g.entity(*id) {
                Some(Entity::Line { a, b }) => {
                    let (pa, pb) = (g.point(a), g.point(b));
                    let u = (pb.x - pa.x, pb.y - pa.y);
                    let l = len(u).max(1e-12);
                    let o = (-u.1 / l * d, u.0 / l * d);
                    ls.push(((pa.x + o.0, pa.y + o.1), (pb.x + o.0, pb.y + o.1)));
                }
                Some(Entity::Circle { center, .. }) => {
                    let c = g.point(center);
                    let r = g.radius(*id) + d;
                    if r > 1e-9 {
                        let me = self.name_after(
                            'c',
                            rows.iter().filter(|r: &&String| r.starts_with("circle")).count(),
                        );
                        rows.push(format!("circle {} {}", pt((c.x, c.y)), n(r)));
                        rows.push(format!("conc {e} {me}"));
                    }
                }
                _ => {}
            }
        }
        // a chain's neighbours meet where their offsets cross
        let k = ls.len();
        for i in 0..k {
            let j = (i + 1) % k;
            if j == i || (j == 0 && k < 3) {
                continue;
            }
            let (a, b) = (ls[i], ls[j]);
            if dist(a.1, b.0) > 1e-6
                && let Some(x) = cross_lines(a.0, sub(a.1, a.0), b.0, sub(b.1, b.0))
            {
                ls[i].1 = x;
                ls[j].0 = x;
            }
        }
        let l0 = self.next_name('l');
        for (i, (a, b)) in ls.iter().enumerate() {
            rows.push(format!("line {} {}", pt(*a), pt(*b)));
            rows.push(format!(
                "par l{} {}",
                l0 + i,
                self.more
                    .sel
                    .iter()
                    .filter(|e| e.starts_with('l'))
                    .nth(i)
                    .cloned()
                    .unwrap_or_default()
            ));
        }
        for i in 1..ls.len() {
            if dist(ls[i - 1].1, ls[i].0) < 1e-6 {
                rows.push(format!("co l{}.b l{}.a", l0 + i - 1, l0 + i));
            }
        }
        rows
    }

    /// A point for a Modify tool once its geometry's chosen; the last applies it.
    fn place_modify(&mut self, tool: SkTool, at: P, reach: f64) {
        if tool == SkTool::Mirror {
            let Some(line) = self.entity_near(at, reach, "l") else {
                return;
            };
            let Some(s) = &self.solved else { return };
            let Ok((a, b)) = sk::line(s, &line) else { return };
            let (a, b) = ((a.x, a.y), (b.x, b.y));
            let u = sub(b, a);
            let l2 = (u.0 * u.0 + u.1 * u.1).max(1e-12);
            let refl = move |q: P| {
                let t = ((q.0 - a.0) * u.0 + (q.1 - a.1) * u.1) / l2;
                let f = (a.0 + u.0 * t, a.1 + u.1 * t);
                (2.0 * f.0 - q.0, 2.0 * f.1 - q.1)
            };
            let rows = self.copy_rows(&refl, true, Some(&line));
            self.add_all(rows);
            self.done_modify();
            return;
        }
        let want = match tool {
            SkTool::Rotate | SkTool::Scale => 3,
            SkTool::Offset | SkTool::CircPattern => 1,
            _ => 2,
        };
        if self.pick_at.len() + 1 < want {
            let (w, q) = self.word(at, reach);
            self.picks.push(w);
            self.pick_at.push(q);
            return;
        }
        let Some(rows) = self.modify_rows(tool, Some(at), reach) else {
            return;
        };
        let moved = matches!(tool, SkTool::Move | SkTool::Rotate | SkTool::Scale);
        if moved {
            match self.build(&rows) {
                Ok(s) => {
                    self.rows = rows;
                    self.solved = Some(s);
                    self.error = None;
                }
                Err(e) => self.error = Some(e),
            }
        } else {
            self.add_all(rows);
        }
        self.done_modify();
    }

    fn done_modify(&mut self) {
        self.more.sel.clear();
        self.more.placing = false;
        self.picks.clear();
        self.pick_at.clear();
    }

    /// Enter for a Modify tool: the choosing's done.
    pub(super) fn more_enter(&mut self) -> bool {
        let modify = self.tool.is_some_and(|t| t.modifies());
        if modify && !self.more.sel.is_empty() && !self.more.placing {
            self.more.placing = true;
            return true;
        }
        false
    }

    /// A corner of two lines rounded or cut: the click is near the corner; the lines are cut
    /// back and an arc (tangent to both) or a line put between.
    fn corner(&mut self, tool: SkTool, at: P, reach: f64) {
        let Some(s) = &self.solved else { return };
        let ls: Vec<String> = s
            .names
            .iter()
            .filter(|x| x.0.starts_with('l'))
            .map(|x| x.0.clone())
            .collect();
        // the two lines with ends at the corner nearest the click
        let mut ends: Vec<(String, &str, P)> = vec![];
        for l in &ls {
            if let Ok((a, b)) = sk::line(s, l) {
                ends.push((l.clone(), "a", (a.x, a.y)));
                ends.push((l.clone(), "b", (b.x, b.y)));
            }
        }
        let Some(c) = ends
            .iter()
            .map(|e| e.2)
            .filter(|p| dist(*p, at) < reach * 4.0)
            .min_by(|x, y| dist(*x, at).total_cmp(&dist(*y, at)))
        else {
            self.error = Some("click near a corner where two lines meet".into());
            return;
        };
        let at_c: Vec<&(String, &str, P)> = ends.iter().filter(|e| dist(e.2, c) < 1e-6).collect();
        let [(l1, e1, _), (l2, e2, _)] = at_c[..] else {
            self.error = Some("a corner of exactly two lines".into());
            return;
        };
        let (Ok(g1), Ok(g2)) = (sk::line(s, l1), sk::line(s, l2)) else {
            return;
        };
        let other = |g: (forge_math::Point2, forge_math::Point2), e: &str| {
            if e == "a" { (g.1.x, g.1.y) } else { (g.0.x, g.0.y) }
        };
        let (o1, o2) = (other(g1, e1), other(g2, e2));
        let (u1, u2) = (sub(o1, c), sub(o2, c));
        let (d1, d2) = (len(u1), len(u2));
        let (u1, u2) = ((u1.0 / d1, u1.1 / d1), (u2.0 / d2, u2.1 / d2));
        let half = dmath::acos((u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0)) * 0.5;
        // a fillet's radius or a chamfer's leg: what's set, else a fifth of the shorter side
        let size = if self.more.size > 0.0 {
            self.more.size
        } else {
            tidy(d1.min(d2) * 0.2)
        };
        let back = if tool == SkTool::SkFillet {
            size * dmath::cos(half) / dmath::sin(half).max(1e-9)
        } else {
            size
        };
        if back >= d1.min(d2) || half < 1e-6 {
            self.error = Some("the corner's sides are too short for that".into());
            return;
        }
        let t1 = (c.0 + u1.0 * back, c.1 + u1.1 * back);
        let t2 = (c.0 + u2.0 * back, c.1 + u2.1 * back);
        let (Some(k1), Some(k2)) = (self.row_of(l1), self.row_of(l2)) else {
            return;
        };
        let mut rows = self.rows.clone();
        // each line's own row ends at its cut end; a row that's a rect is refused
        for (k, l, e, t) in [(k1, l1, e1, t1), (k2, l2, e2, t2)] {
            let toks = tokens(&rows[k]);
            if toks[0] != "line" {
                self.error = Some(format!(
                    "{l} is part of a {}: draw it with lines to round its corners",
                    toks[0]
                ));
                return;
            }
            let mut out = vec!["line".to_string()];
            let mut i = 1;
            for end in ["a", "b"] {
                let numeric = toks.get(i).is_some_and(|x| x.parse::<f64>().is_ok());
                let tok = if numeric {
                    let v = format!("{} {}", toks[i], toks[i + 1]);
                    i += 2;
                    v
                } else {
                    i += 1;
                    toks[i - 1].clone()
                };
                out.push(if end == *e { pt(t) } else { tok });
            }
            out.extend(toks[i..].iter().cloned());
            rows[k] = out.join(" ");
        }
        let a = self.name_after('a', 0);
        if tool == SkTool::SkFillet {
            let bis = (u1.0 + u2.0, u1.1 + u2.1);
            let bl = len(bis).max(1e-12);
            let h = size / dmath::sin(half).max(1e-9);
            let center = (c.0 + bis.0 / bl * h, c.1 + bis.1 / bl * h);
            let ccw = u1.0 * u2.1 - u1.1 * u2.0 < 0.0;
            rows.push(format!(
                "arc {} {l1}.{e1} {l2}.{e2}{}",
                pt(center),
                if ccw { "" } else { " cw" }
            ));
            rows.push(format!("tan {l1} {a}"));
            rows.push(format!("tan {l2} {a}"));
        } else {
            rows.push(format!("line {l1}.{e1} {l2}.{e2}"));
        }
        match self.build(&rows) {
            Ok(s) => {
                self.rows = rows;
                self.solved = Some(s);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Where a line meets the others: its parameters along it (0 at a, 1 at b), sorted.
    fn crossings(&self, line: &str) -> Vec<f64> {
        let Some(s) = &self.solved else { return vec![] };
        let Ok((a, b)) = sk::line(s, line) else {
            return vec![];
        };
        let (a, b) = ((a.x, a.y), (b.x, b.y));
        let u = sub(b, a);
        let uu = (u.0 * u.0 + u.1 * u.1).max(1e-12);
        let g = &s.solved;
        let mut ts = vec![];
        for (name, id) in &s.names {
            if name == line {
                continue;
            }
            match g.entity(*id) {
                Some(Entity::Line { a: p, b: q }) => {
                    let (p, q) = ((g.point(p).x, g.point(p).y), (g.point(q).x, g.point(q).y));
                    let v = sub(q, p);
                    let den = u.0 * v.1 - u.1 * v.0;
                    if den.abs() < 1e-12 {
                        continue;
                    }
                    let w = sub(p, a);
                    let t = (w.0 * v.1 - w.1 * v.0) / den;
                    let s2 = (w.0 * u.1 - w.1 * u.0) / den;
                    if (-1e-9..=1.0 + 1e-9).contains(&s2) {
                        ts.push(t);
                    }
                }
                Some(Entity::Circle { center, .. } | Entity::Arc { center, .. }) => {
                    let c = (g.point(center).x, g.point(center).y);
                    let r = g.radius(*id);
                    let w = sub(a, c);
                    let (bq, cq) = (2.0 * (w.0 * u.0 + w.1 * u.1), w.0 * w.0 + w.1 * w.1 - r * r);
                    let disc = bq * bq - 4.0 * uu * cq;
                    if disc < 0.0 {
                        continue;
                    }
                    for sgn in [-1.0, 1.0] {
                        let t = (-bq + sgn * disc.sqrt()) / (2.0 * uu);
                        let q = (a.0 + u.0 * t, a.1 + u.1 * t);
                        let on_arc = match g.entity(*id) {
                            Some(Entity::Arc { .. }) => {
                                let pts = self.outline_of(name);
                                pts.iter().any(|x| dist(*x, q) < r * 0.07 + 1e-6)
                            }
                            _ => true,
                        };
                        if on_arc {
                            ts.push(t);
                        }
                    }
                }
                _ => {}
            }
        }
        ts.sort_by(f64::total_cmp);
        ts
    }

    /// Trim the piece of a line clicked back to where it meets others; extend its nearer end to
    /// the next it meets; split it there.
    fn cut_line(&mut self, tool: SkTool, at: P, reach: f64) {
        let Some(l) = self.entity_near(at, reach, "l") else {
            self.error = Some(format!("{} works on lines: click one", tool.word()));
            return;
        };
        let Some(s) = &self.solved else { return };
        let Ok((a, b)) = sk::line(s, &l) else { return };
        let (a, b) = ((a.x, a.y), (b.x, b.y));
        let u = sub(b, a);
        let uu = (u.0 * u.0 + u.1 * u.1).max(1e-12);
        let t_at = ((at.0 - a.0) * u.0 + (at.1 - a.1) * u.1) / uu;
        let ts = self.crossings(&l);
        let on = |t: f64| (a.0 + u.0 * t, a.1 + u.1 * t);
        let Some(k) = self.row_of(&l) else { return };
        if tokens(&self.rows[k])[0] != "line" {
            self.error = Some(format!(
                "{l} is part of a shape: {} works on lines drawn as lines",
                tool.word()
            ));
            return;
        }
        let inside: Vec<f64> = ts
            .iter()
            .copied()
            .filter(|t| *t > 1e-6 && *t < 1.0 - 1e-6)
            .collect();
        let before = inside
            .iter()
            .copied()
            .filter(|t| *t < t_at)
            .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))));
        let after = inside
            .iter()
            .copied()
            .filter(|t| *t > t_at)
            .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.min(t))));
        let mut rows = self.rows.clone();
        let tail: Vec<String> = {
            let t = tokens(&rows[k]);
            t.iter()
                .skip_while(|_| false)
                .filter(|w| *w == "cons")
                .cloned()
                .collect()
        };
        let tail = if tail.is_empty() {
            String::new()
        } else {
            " cons".into()
        };
        match tool {
            SkTool::Trim => match (before, after) {
                (None, None) => {
                    self.error = Some(format!("{l} meets nothing to trim back to"));
                    return;
                }
                (Some(t0), None) => rows[k] = format!("line {} {}{tail}", pt(a), pt(on(t0))),
                (None, Some(t1)) => rows[k] = format!("line {} {}{tail}", pt(on(t1)), pt(b)),
                (Some(t0), Some(t1)) => {
                    rows[k] = format!("line {} {}{tail}", pt(a), pt(on(t0)));
                    rows.push(format!("line {} {}{tail}", pt(on(t1)), pt(b)));
                }
            },
            SkTool::Split => {
                let Some(t) = inside
                    .iter()
                    .copied()
                    .min_by(|x, y| (x - t_at).abs().total_cmp(&(y - t_at).abs()))
                else {
                    self.error = Some(format!("{l} meets nothing to split it at"));
                    return;
                };
                rows[k] = format!("line {} {}{tail}", pt(a), pt(on(t)));
                let me = self.name_after('l', 0);
                rows.push(format!("line {l}.b {}{tail}", pt(b)));
                // sharing the point, parallel is collinear
                rows.push(format!("par {l} {me}"));
            }
            _ => {
                // the end nearer the click goes on to the next thing beyond it
                if t_at > 0.5 {
                    let Some(t) = ts.iter().copied().find(|t| *t > 1.0 + 1e-6) else {
                        self.error = Some(format!("nothing beyond {l}'s end to extend it to"));
                        return;
                    };
                    rows[k] = format!("line {} {}{tail}", pt(a), pt(on(t)));
                } else {
                    let Some(t) = ts.iter().copied().rev().find(|t| *t < -1e-6) else {
                        self.error = Some(format!("nothing beyond {l}'s start to extend it to"));
                        return;
                    };
                    rows[k] = format!("line {} {}{tail}", pt(on(t)), pt(b));
                }
            }
        }
        match self.build(&rows) {
            Ok(s) => {
                self.rows = rows;
                self.solved = Some(s);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }
}

impl SkTool {
    /// It works on geometry chosen first.
    pub fn modifies(self) -> bool {
        matches!(
            self,
            SkTool::Move
                | SkTool::Copy
                | SkTool::Rotate
                | SkTool::Scale
                | SkTool::Mirror
                | SkTool::Offset
                | SkTool::RectPattern
                | SkTool::CircPattern
        )
    }

    /// Its name, in words.
    pub fn word(self) -> &'static str {
        match self {
            SkTool::Trim => "Trim",
            SkTool::Extend => "Extend",
            SkTool::Split => "Split",
            _ => "That tool",
        }
    }

    /// What the status bar asks for, given how far it's got.
    pub fn prompt(self, picked: usize, chosen: usize, placing: bool) -> Option<&'static str> {
        Some(match self {
            SkTool::Circle3 => [
                "Click the first point on the circle",
                "Click the second point",
                "Click the third point",
            ][picked.min(2)],
            SkTool::Arc3 => [
                "Click the arc's start",
                "Click its end",
                "Click a point it passes through",
            ][picked.min(2)],
            SkTool::ArcTan => [
                "Click the end of a line or arc to leave tangent from",
                "Click the arc's end",
            ][picked.min(1)],
            SkTool::RectCenter => ["Click the rectangle's centre", "Click a corner"][picked.min(1)],
            SkTool::Rect3 => [
                "Click the first corner",
                "Click the second corner: the first side",
                "Click to set the width",
            ][picked.min(2)],
            SkTool::SlotCC => [
                "Click the first arc's centre",
                "Click the second arc's centre",
                "Click to set the width",
            ][picked.min(2)],
            SkTool::SlotOverall => {
                ["Click one end", "Click the other end", "Click to set the width"][picked.min(2)]
            }
            SkTool::SlotCenter => [
                "Click the slot's centre",
                "Click an arc's centre",
                "Click to set the width",
            ][picked.min(2)],
            SkTool::Polygon => ["Click the polygon's centre", "Click a corner"][picked.min(1)],
            SkTool::Ellipse => [
                "Click the centre",
                "Click the end of the major axis",
                "Click to set the minor axis",
            ][picked.min(2)],
            SkTool::SkFillet => "Click near a corner where two lines meet",
            SkTool::SkChamfer => "Click near a corner where two lines meet",
            SkTool::Text => "Click where the text starts; Emboss takes it after the sketch",
            SkTool::Trim => "Click the piece of a line to trim away",
            SkTool::Extend => "Click near the end of a line to extend",
            SkTool::Split => "Click a line where it should split",
            SkTool::Horizontal | SkTool::Vertical => "Click a line, or two points",
            SkTool::Collinear => "Click two lines",
            SkTool::Concentric => "Click two circles or arcs",
            SkTool::Equal => "Click two lines, or two circles or arcs",
            SkTool::Fix => "Click a point or a curve to fix",
            SkTool::Symmetric => {
                if picked < 2 {
                    "Click two points, lines or circles"
                } else {
                    "Click the symmetry line"
                }
            }
            t if t.modifies() => {
                if !placing {
                    if chosen == 0 {
                        "Click the geometry to change"
                    } else {
                        "Click more geometry, or Enter (or a click on nothing) when done"
                    }
                } else {
                    match (t, picked) {
                        (SkTool::Mirror, _) => "Click the mirror line",
                        (SkTool::Move | SkTool::Copy, 0) => "Click the base point",
                        (SkTool::Move | SkTool::Copy, _) => "Click where it goes",
                        (SkTool::Rotate, 0) => "Click the centre of rotation",
                        (SkTool::Rotate, 1) => "Click a point to turn from",
                        (SkTool::Rotate, _) => "Click where that point turns to",
                        (SkTool::Scale, 0) => "Click the base point",
                        (SkTool::Scale, 1) => "Click a point to scale from",
                        (SkTool::Scale, _) => "Click where that point goes",
                        (SkTool::Offset, _) => "Click the side and distance",
                        (SkTool::RectPattern, 0) => "Click the start of the step",
                        (SkTool::RectPattern, _) => "Click the step's end: the next copy's place",
                        (SkTool::CircPattern, _) => "Click the centre to turn copies about",
                        _ => return None,
                    }
                }
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::SketchFor;
    use super::*;
    use forge_math::Frame;

    fn mode() -> SketchMode {
        SketchMode::new(SketchFor::New, "xy".into(), Frame::default(), Vec::new(), None)
    }

    fn click_all(m: &mut SketchMode, pts: &[P]) {
        for p in pts {
            m.click(*p, 0.5);
        }
    }

    fn solves(m: &SketchMode) {
        assert!(m.error.is_none(), "{:?}\n{:?}", m.error, m.rows);
        assert!(m.solved.is_some());
    }

    #[test]
    fn three_points_make_a_circle_and_an_arc() {
        let mut m = mode();
        m.set_tool(Some(SkTool::Circle3));
        click_all(&mut m, &[(10.0, 0.0), (0.0, 10.0), (-10.0, 0.0)]);
        assert_eq!(m.rows, ["circle 0 0 10"]);
        m.set_tool(Some(SkTool::Arc3));
        click_all(&mut m, &[(30.0, 0.0), (10.0, 0.0), (20.0, 10.0)]);
        assert_eq!(m.rows[1], "arc 20 0 30 0 10 0");
        solves(&m);
    }

    #[test]
    fn a_tangent_arc_leaves_a_line_end_along_it() {
        let mut m = mode();
        m.add("line 0 0 10 0".into());
        m.set_tool(Some(SkTool::ArcTan));
        click_all(&mut m, &[(10.0, 0.0), (10.0, 10.0)]);
        assert_eq!(m.rows[1], "arc 10 5 l1.b 10 10");
        assert_eq!(m.rows[2], "tan l1 a1");
        solves(&m);
    }

    #[test]
    fn rectangles_by_centre_and_by_three_points() {
        let mut m = mode();
        m.set_tool(Some(SkTool::RectCenter));
        click_all(&mut m, &[(0.0, 0.0), (5.0, 3.0)]);
        assert_eq!(m.rows[0], "rect -5 -3 10 6");
        m.set_tool(Some(SkTool::Rect3));
        click_all(&mut m, &[(20.0, 0.0), (30.0, 10.0), (25.0, 20.0)]);
        assert!(m.rows[1].starts_with("poly 20 0 30 10 "), "{:?}", m.rows);
        assert_eq!(m.rows[2], "perp l5 l6");
        solves(&m);
    }

    #[test]
    fn every_slot_is_two_lines_and_two_arcs_tangent() {
        for t in [SkTool::SlotCC, SkTool::SlotOverall, SkTool::SlotCenter] {
            let mut m = mode();
            m.set_tool(Some(t));
            click_all(&mut m, &[(0.0, 0.0), (20.0, 0.0), (10.0, 4.0)]);
            solves(&m);
            assert_eq!(m.rows.len(), 8, "{t:?} {:?}", m.rows);
            let s = m.solved.as_ref().unwrap();
            let p = sk::profile(s, None).expect("one closed loop");
            assert_eq!(p.loops.len(), 1, "{t:?}");
            // its area: the ends round outward
            let a = p.area().abs();
            let r = 4.0;
            let len = if t == SkTool::SlotOverall {
                20.0 - 2.0 * r
            } else if t == SkTool::SlotCenter {
                40.0
            } else {
                20.0
            };
            assert!(
                (a - (2.0 * r * len + core::f64::consts::PI * r * r)).abs() < 1e-6,
                "{t:?} {a}"
            );
        }
    }

    #[test]
    fn a_polygon_has_equal_sides() {
        let mut m = mode();
        m.more.count = 6;
        m.set_tool(Some(SkTool::Polygon));
        click_all(&mut m, &[(0.0, 0.0), (10.0, 0.0)]);
        assert!(m.rows[0].starts_with("poly 10 0 5 8.66"), "{:?}", m.rows);
        assert_eq!(m.rows[1], "eq l1 l2 l3 l4 l5 l6");
        solves(&m);
    }

    #[test]
    fn every_constraint_tool_writes_its_row() {
        let mut m = mode();
        for r in [
            "line 0 0 10 0",
            "line 20 0 30 0",
            "circle 50 0 5",
            "circle 51 1 3",
            "line 0 10 10 12",
            "line 0 -20 0 20",
        ] {
            m.add(r.into());
        }
        m.set_tool(Some(SkTool::Horizontal));
        m.click((5.0, 0.2), 0.5);
        m.set_tool(Some(SkTool::Collinear));
        click_all(&mut m, &[(5.0, 0.0), (25.0, 0.2)]);
        m.set_tool(Some(SkTool::Concentric));
        click_all(&mut m, &[(55.0, 0.0), (54.0, 1.0)]);
        m.set_tool(Some(SkTool::Equal));
        click_all(&mut m, &[(5.0, 0.0), (5.0, 11.0)]);
        m.set_tool(Some(SkTool::Fix));
        m.click((0.0, 0.0), 0.5);
        m.set_tool(Some(SkTool::Vertical));
        m.click((0.0, 5.0), 0.5);
        solves(&m);
        assert_eq!(
            &m.rows[6..],
            [
                "hor l1",
                "colin l1 l2",
                "conc c1 c2",
                "eq l1 l3",
                "fix l1.a",
                "ver l4"
            ]
        );
        m.set_tool(Some(SkTool::Symmetric));
        // the two lines, then the line between
        click_all(&mut m, &[(25.0, 0.0), (5.0, 11.0), (0.0, 5.0)]);
        solves(&m);
        assert_eq!(m.rows.last().unwrap(), "sym l2.b l3.b l4");
    }

    #[test]
    fn move_copy_rotate_scale_and_mirror_chosen_geometry() {
        let mut m = mode();
        m.add("line 0 0 10 0".into());
        m.add("line l1.b 10 10".into());
        m.add("line 0 -20 0 20".into());
        m.set_tool(Some(SkTool::Copy));
        click_all(&mut m, &[(5.0, 0.0), (10.0, 5.0)]);
        assert!(m.more_enter());
        click_all(&mut m, &[(0.0, 0.0), (20.0, 0.0)]);
        solves(&m);
        assert_eq!(m.rows[3], "line 20 0 30 0");
        assert_eq!(m.rows[4], "line 30 0 30 10");
        assert_eq!(m.rows[5], "co l5.a l4.b");
        m.set_tool(Some(SkTool::Move));
        click_all(&mut m, &[(25.0, 0.0)]);
        m.more_enter();
        click_all(&mut m, &[(0.0, 50.0), (0.0, 55.0)]);
        assert_eq!(m.rows[3], "line 20 5 30 5");
        m.set_tool(Some(SkTool::Mirror));
        click_all(&mut m, &[(5.0, 0.0)]);
        m.more_enter();
        click_all(&mut m, &[(0.0, 10.0)]);
        solves(&m);
        assert!(
            m.rows
                .iter()
                .any(|r| r == "line -0 0 -10 0" || r == "line 0 0 -10 0"),
            "{:?}",
            m.rows
        );
        assert!(m.rows.iter().any(|r| r == "sym l1.a l6.a l3"), "{:?}", m.rows);
        m.set_tool(Some(SkTool::Rotate));
        click_all(&mut m, &[(10.0, 5.0)]);
        m.more_enter();
        click_all(&mut m, &[(10.0, 0.0), (20.0, 0.0), (10.0, 10.0)]);
        solves(&m);
        assert_eq!(m.rows[1], "line l1.b 0 0");
    }

    #[test]
    fn offset_and_patterns_copy_what_is_chosen() {
        let mut m = mode();
        m.add("circle 0 0 5".into());
        m.set_tool(Some(SkTool::Offset));
        m.click((5.0, 0.0), 0.5);
        m.more_enter();
        m.click((8.0, 0.0), 0.5);
        solves(&m);
        assert_eq!(&m.rows[1..], ["circle 0 0 8", "conc c1 c2"]);
        m.more.count = 4;
        m.set_tool(Some(SkTool::CircPattern));
        m.add("circle 20 0 2".into());
        m.click((22.0, 0.0), 0.5);
        m.more_enter();
        m.click((0.0, 0.0), 0.5);
        solves(&m);
        assert_eq!(
            m.rows.iter().filter(|r| r.starts_with("circle")).count(),
            6,
            "{:?}",
            m.rows
        );
        m.more.count = 3;
        m.set_tool(Some(SkTool::RectPattern));
        m.click((22.0, 0.0), 0.5);
        m.more_enter();
        click_all(&mut m, &[(40.0, 40.0), (50.0, 40.0)]);
        solves(&m);
        assert!(m.rows.contains(&"circle 30 0 2".to_string()), "{:?}", m.rows);
        assert!(m.rows.contains(&"circle 40 0 2".to_string()), "{:?}", m.rows);
    }

    #[test]
    fn trim_extend_split_and_corners() {
        let mut m = mode();
        m.add("line 0 0 20 0".into());
        m.add("line 10 -5 10 5".into());
        m.add("line 30 -5 30 5".into());
        m.set_tool(Some(SkTool::Extend));
        m.click((19.0, 0.0), 0.5);
        assert_eq!(m.rows[0], "line 0 0 30 0");
        m.set_tool(Some(SkTool::Split));
        m.click((11.0, 0.0), 0.5);
        solves(&m);
        assert_eq!(m.rows[0], "line 0 0 10 0");
        assert_eq!(m.rows[3], "line l1.b 30 0");
        m.set_tool(Some(SkTool::Trim));
        m.click((10.0, 4.0), 0.5);
        solves(&m);
        assert_eq!(m.rows[1], "line 10 -5 10 0");
        let mut m = mode();
        m.add("line 0 0 20 0".into());
        m.add("line l1.b 20 20".into());
        m.more.size = 3.0;
        m.set_tool(Some(SkTool::SkFillet));
        m.click((19.5, 0.5), 0.5);
        solves(&m);
        assert_eq!(m.rows[0], "line 0 0 17 0");
        assert!(m.rows[2].starts_with("arc 17 3 l1.b l2.a"), "{:?}", m.rows);
    }
}
