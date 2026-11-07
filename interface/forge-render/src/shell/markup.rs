//! Markup for the agent: pen, arrow, ellipse, rectangle and text drawn over the window, sent with
//! the next message as a picture and as what each mark is on, by the names the agent's tools take.
//! This keeps the marks and draws them; the host (`gui_markup.rs`) names what's under them.

use super::theme::{medium, semi};
use super::{DOCS_H, HEAD_H, STATUS_H, Shell, TABS_H, TITLE_H, agent::Entry, agent::SideTab, icons};
use egui::{
    Align2, Color32, Id, LayerId, Order, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Pen,
    Arrow,
    Ellipse,
    Rect,
    Text,
}

impl Tool {
    pub const ALL: [(Tool, &'static str, &'static str); 5] = [
        (Tool::Pen, "pen", "Pen"),
        (Tool::Arrow, "arrowmark", "Arrow"),
        (Tool::Ellipse, "ellipse", "Ellipse"),
        (Tool::Rect, "rectangle", "Rectangle"),
        (Tool::Text, "text", "Text"),
    ];

    fn word(self) -> &'static str {
        match self {
            Tool::Pen => "pen stroke",
            Tool::Arrow => "arrow",
            Tool::Ellipse => "ellipse",
            Tool::Rect => "rectangle",
            Tool::Text => "text",
        }
    }
}

pub const INKS: [(&str, Color32); 4] = [
    ("red", Color32::from_rgb(0xE5, 0x48, 0x4D)),
    ("amber", Color32::from_rgb(0xF5, 0xA5, 0x24)),
    ("green", Color32::from_rgb(0x30, 0xA4, 0x6C)),
    ("blue", Color32::from_rgb(0x3E, 0x8E, 0xF7)),
];

const WIDE: f32 = 3.0;
const TEXT_PT: f32 = 15.0;

/// One mark, in window points: a pen's points, an arrow's tail and head, a shape's or a label's
/// two corners.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub tool: Tool,
    pub ink: usize,
    pub pts: Vec<Pos2>,
    pub text: String,
}

impl Mark {
    pub fn bounds(&self) -> Rect {
        let r = Rect::from_points(&self.pts);
        if self.tool == Tool::Text {
            r
        } else {
            r.expand(WIDE * 0.5)
        }
    }

    /// A pen stroke that comes back near where it began: a loop round something.
    pub fn closed(&self) -> bool {
        let (Some(a), Some(b)) = (self.pts.first(), self.pts.last()) else {
            return false;
        };
        let r = Rect::from_points(&self.pts);
        self.tool == Tool::Pen
            && self.pts.len() > 8
            && r.size().min_elem() > 12.0
            && a.distance(*b) < 0.25 * r.size().max_elem()
    }

    /// A shape that holds a region: an ellipse, a rectangle, a loop.
    pub fn region(&self) -> bool {
        matches!(self.tool, Tool::Ellipse | Tool::Rect) || self.closed()
    }

    /// Is the point in the region the mark holds?
    pub fn inside(&self, p: Pos2) -> bool {
        let r = Rect::from_points(&self.pts);
        match self.tool {
            Tool::Rect => r.contains(p),
            Tool::Ellipse => {
                let (c, h) = (r.center(), r.size() * 0.5);
                if h.x < 1.0 || h.y < 1.0 {
                    return false;
                }
                let d = (p - c) / h;
                d.x * d.x + d.y * d.y <= 1.0
            }
            Tool::Pen if self.closed() => {
                // even-odd, the stroke closed back to its start
                let n = self.pts.len();
                let mut inside = false;
                for k in 0..n {
                    let (a, b) = (self.pts[k], self.pts[(k + 1) % n]);
                    if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                        inside = !inside;
                    }
                }
                inside
            }
            _ => false,
        }
    }

    /// Points along a pen stroke, about `step` apart.
    pub fn samples(&self, step: f32) -> Vec<Pos2> {
        let mut out: Vec<Pos2> = self.pts.first().copied().into_iter().collect();
        for w in self.pts.windows(2) {
            let n = (w[0].distance(w[1]) / step).ceil().max(1.0) as usize;
            out.extend((1..=n).map(|k| w[0].lerp(w[1], k as f32 / n as f32)));
        }
        out
    }
}

/// A small RGBA picture, for the chat's thumbnail.
#[derive(Clone, PartialEq)]
pub struct Thumb {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Thumb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Thumb({}×{})", self.w, self.h)
    }
}

/// A message's markup as it was sent: the marks, the camera they were drawn over, a thumbnail of
/// the picture, and what the agent was told of them.
#[derive(Clone, Debug, PartialEq)]
pub struct Sent {
    pub marks: Vec<Mark>,
    pub camera: Option<crate::camera::Camera>,
    pub thumb: Arc<Thumb>,
    pub said: String,
}

#[derive(Default)]
pub struct Markup {
    pub on: bool,
    pub tool: Tool,
    pub ink: usize,
    pub marks: Vec<Mark>,
    drawing: Option<Mark>,
    /// A label being typed: where, and what so far.
    pub typing: Option<(Pos2, String)>,
    /// A sent message's marks shown again: its entry.
    pub shown: Option<usize>,
    /// The frame that's the picture sent: no tool strip in it.
    pub sending: bool,
}

impl Markup {
    pub fn toggle(&mut self) {
        self.on = !self.on;
        self.shown = None;
        if !self.on {
            self.drawing = None;
            self.commit_text();
        }
    }

    pub fn commit_text(&mut self) {
        if let Some((at, text)) = self.typing.take()
            && !text.trim().is_empty()
        {
            // the label's box, as it's drawn: a glyph of this size is about 0.55 em wide
            let w = text.trim().chars().count() as f32 * TEXT_PT * 0.55 + 10.0;
            self.marks.push(Mark {
                tool: Tool::Text,
                ink: self.ink,
                pts: vec![at, at + vec2(w, TEXT_PT + 8.0)],
                text: text.trim().to_string(),
            });
        }
    }
}

/// What's under the marks, as the host can tell: names the agent's tools take, each with its kind
/// (`face \`extrude1.end\``).
pub trait Under {
    /// What's at a point in window points, or the nearest thing within `reach` points.
    fn at(&self, p: Pos2, reach: f32) -> Option<String>;
    /// What a region mark holds: each thing's name, its pixels inside, its pixels shown at all, and
    /// whether it's a whole (an occurrence, the solid) rather than a face or edge.
    fn within(&self, m: &Mark) -> Vec<(String, u32, u32, bool)>;
    /// Where a point is on a drawing's sheet, in mm, if it's on one.
    fn sheet(&self, _p: Pos2) -> Option<(f64, f64)> {
        None
    }
}

fn list(names: &[String], most: usize) -> String {
    let mut s = names.iter().take(most).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > most {
        s.push_str(&format!(" and {} more", names.len() - most));
    }
    s
}

/// The marks in words, one line each: kind, colour, where in the picture (`px` takes a window
/// point to its pixel), and what in the model it's on.
pub fn describe(marks: &[Mark], under: &dyn Under, px: &dyn Fn(Pos2) -> (i32, i32)) -> String {
    let mut out = String::from("Marked on the screen (the picture shows the marks; x, y are its pixels):");
    for (k, m) in marks.iter().enumerate() {
        let b = m.bounds();
        let (lo, hi) = (px(b.min), px(b.max));
        let ink = INKS[m.ink.min(INKS.len() - 1)].0;
        let head = match m.tool {
            Tool::Text => format!(
                "text \"{}\", {ink}, x {}–{}, y {}–{}",
                m.text, lo.0, hi.0, lo.1, hi.1
            ),
            Tool::Arrow => {
                let (a, z) = (px(m.pts[0]), px(*m.pts.last().expect("two points")));
                format!("arrow, {ink}, from x {} y {} to x {} y {}", a.0, a.1, z.0, z.1)
            }
            t => {
                let word = if m.closed() { "loop drawn round" } else { t.word() };
                format!("{word}, {ink}, x {}–{}, y {}–{}", lo.0, hi.0, lo.1, hi.1)
            }
        };
        // on a sheet, where it points or sits in the sheet's own mm
        let key = match m.tool {
            Tool::Arrow => *m.pts.last().expect("two points"),
            _ => b.center(),
        };
        let head = match under.sheet(key) {
            Some((x, y)) => format!("{head} (sheet x {x:.1}, y {y:.1} mm)"),
            None => head,
        };
        let what = if m.region() {
            let mut got = under.within(m);
            got.sort_by_key(|g| (std::cmp::Reverse(g.3), std::cmp::Reverse(g.1)));
            let mut around: Vec<String> = got
                .iter()
                .filter(|g| g.1 >= 4 && g.1 * 2 >= g.2)
                .map(|g| g.0.clone())
                .collect();
            // the faces held say it: their edges are noise
            let leaf = |n: &str| n.rsplit("› ").next().unwrap_or(n).to_string();
            if around.iter().any(|n| leaf(n).starts_with("face ")) {
                around.retain(|n| !leaf(n).starts_with("edge "));
            }
            if !around.is_empty() {
                format!("around {}", list(&around, 10))
            } else {
                got.sort_by_key(|g| std::cmp::Reverse(g.1));
                let over: Vec<String> = got.iter().filter(|g| g.1 > 0).map(|g| g.0.clone()).collect();
                if over.is_empty() {
                    "on nothing of the model".into()
                } else {
                    format!("over part of {}", list(&over, 4))
                }
            }
        } else {
            match m.tool {
                Tool::Arrow => {
                    let from = under.at(m.pts[0], 12.0);
                    let to = under.at(*m.pts.last().expect("two points"), 12.0);
                    match (from, to) {
                        (Some(f), Some(t)) if f != t => format!("from {f} pointing to {t}"),
                        (_, Some(t)) => format!("pointing to {t}"),
                        (Some(f), None) => format!("from {f}, pointing at nothing of the model"),
                        (None, None) => "pointing at nothing of the model".into(),
                    }
                }
                Tool::Text => {
                    let mut s = match under.at(b.center(), 0.0) {
                        Some(on) => format!("on {on}"),
                        None => match under.at(b.center(), 40.0) {
                            Some(n) => format!("near {n}"),
                            None => "on nothing of the model".into(),
                        },
                    };
                    for (j, o) in marks
                        .iter()
                        .enumerate()
                        .filter(|(j, o)| *j != k && o.tool != Tool::Text)
                    {
                        let near = |p: Pos2| b.expand(30.0).contains(p);
                        if o.tool == Tool::Arrow && near(o.pts[0]) {
                            s.push_str(&format!("; at the tail of mark {}", j + 1));
                        } else if o.bounds().expand(20.0).intersects(b) {
                            s.push_str(&format!("; beside mark {}", j + 1));
                        }
                    }
                    s
                }
                _ => {
                    let mut seen: Vec<String> = Vec::new();
                    for p in m.samples(4.0) {
                        if let Some(n) = under.at(p, 4.0)
                            && seen.last() != Some(&n)
                            && !seen.contains(&n)
                        {
                            seen.push(n);
                        }
                    }
                    if seen.is_empty() {
                        "on nothing of the model".into()
                    } else {
                        format!("across {}", list(&seen, 8))
                    }
                }
            }
        };
        out.push_str(&format!("\n{}. {head}: {what}", k + 1));
    }
    out
}

/// The marks drawn, `dim` for a sent message's shown again.
pub fn paint(p: &egui::Painter, marks: &[Mark], dim: bool, t: &super::theme::Theme) {
    for m in marks {
        let ink = INKS[m.ink.min(INKS.len() - 1)].1;
        let ink = if dim { ink.gamma_multiply(0.7) } else { ink };
        let s = Stroke::new(WIDE, ink);
        match m.tool {
            Tool::Pen => {
                if m.pts.len() == 1 {
                    p.circle_filled(m.pts[0], WIDE * 0.5, ink);
                } else {
                    p.add(Shape::line(m.pts.clone(), s));
                }
            }
            Tool::Arrow => {
                let (a, z) = (m.pts[0], *m.pts.last().expect("two points"));
                p.line_segment([a, z], s);
                let d = (z - a).normalized();
                if d.is_finite() {
                    let n = vec2(-d.y, d.x);
                    let back = z - d * 14.0;
                    p.add(Shape::convex_polygon(
                        vec![z + d * 2.0, back + n * 7.0, back - n * 7.0],
                        ink,
                        Stroke::NONE,
                    ));
                }
            }
            Tool::Ellipse => {
                let r = Rect::from_points(&m.pts);
                let (c, h) = (r.center(), r.size() * 0.5);
                let pts: Vec<Pos2> = (0..72)
                    .map(|k| {
                        let a = k as f32 / 72.0 * std::f32::consts::TAU;
                        c + vec2(h.x * a.cos(), h.y * a.sin())
                    })
                    .collect();
                p.add(Shape::closed_line(pts, s));
            }
            Tool::Rect => {
                p.rect_stroke(Rect::from_points(&m.pts), 2.0, s, StrokeKind::Middle);
            }
            Tool::Text => {
                let r = m.bounds();
                p.rect_filled(r, 4.0, t.surface.gamma_multiply(0.85));
                p.rect_stroke(r, 4.0, Stroke::new(1.5, ink), StrokeKind::Inside);
                p.text(
                    pos2(r.left() + 5.0, r.center().y),
                    Align2::LEFT_CENTER,
                    &m.text,
                    semi(TEXT_PT),
                    ink,
                );
            }
        }
    }
}

impl Shell {
    /// Where a drag draws: the ribbon and the view (or the sheet), and the browser on the Model tab.
    fn markup_regions(&self, ctx: &egui::Context) -> Vec<Rect> {
        let s = ctx.content_rect();
        let bottom = s.height() - DOCS_H - STATUS_H;
        let left = self.view_rect.left();
        let mut out = vec![Rect::from_min_max(
            pos2(left, TITLE_H + TABS_H),
            pos2(s.width(), bottom),
        )];
        if self.agent.tab == SideTab::Model {
            out.push(Rect::from_min_max(
                pos2(0.0, self.view_rect.top() + HEAD_H),
                pos2(left - 2.0, bottom),
            ));
        }
        out
    }

    /// The markup over everything: its surface, its marks, its tool strip. Last in the frame, so
    /// a drag on the surface is its own and not what's under it.
    pub(super) fn markup_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let t = self.theme;
        let ink_layer = ctx.layer_painter(LayerId::new(Order::Middle, Id::unique("markup-ink")));
        if let Some(k) = self.agent.markup.shown {
            match self.agent.entries.get(k) {
                Some(Entry::User { markup: Some(s), .. }) => paint(&ink_layer, &s.marks, true, &t),
                _ => self.agent.markup.shown = None,
            }
        }
        if !self.agent.markup.on {
            return;
        }
        for (k, r) in self.markup_regions(&ctx).into_iter().enumerate() {
            let resp = ui.interact(r, Id::unique(("markup-surface", k)), Sense::click_and_drag());
            if resp.hovered() || resp.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            let m = &mut self.agent.markup;
            let at = resp.interact_pointer_pos();
            if resp.drag_started()
                && m.tool != Tool::Text
                && let Some(o) = ctx.input(|i| i.pointer.press_origin())
            {
                m.commit_text();
                m.drawing = Some(Mark {
                    tool: m.tool,
                    ink: m.ink,
                    pts: vec![o],
                    text: String::new(),
                });
            }
            if resp.dragged()
                && let (Some(d), Some(at)) = (&mut m.drawing, at)
            {
                if d.tool == Tool::Pen {
                    if d.pts.last().is_none_or(|l| l.distance(at) >= 2.0) {
                        d.pts.push(at);
                    }
                } else {
                    d.pts.truncate(1);
                    d.pts.push(at);
                }
            }
            if resp.drag_stopped()
                && let Some(d) = m.drawing.take()
                && d.pts.len() > 1
                && Rect::from_points(&d.pts).size().max_elem() > 4.0
            {
                m.marks.push(d);
            }
            if resp.clicked()
                && m.tool == Tool::Text
                && let Some(at) = at
            {
                m.commit_text();
                m.typing = Some((at - vec2(5.0, (TEXT_PT + 8.0) * 0.5), String::new()));
            }
            self.places.push((format!("markup:surface:{k}"), r));
        }
        let m = &self.agent.markup;
        paint(&ink_layer, &m.marks, false, &t);
        if let Some(d) = &m.drawing {
            paint(&ink_layer, std::slice::from_ref(d), false, &t);
        }
        self.markup_text(&ctx);
        if !self.agent.markup.sending {
            self.markup_strip(&ctx);
        }
    }

    /// The label being typed: a field where it goes; Enter or a click away keeps it, Esc drops it.
    fn markup_text(&mut self, ctx: &egui::Context) {
        let Some((at, _)) = self.agent.markup.typing else {
            return;
        };
        let ink = INKS[self.agent.markup.ink].1;
        let t = self.theme;
        let mut done = None;
        egui::Area::new(Id::unique("markup-text"))
            .order(Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                let Some((_, text)) = &mut self.agent.markup.typing else {
                    return;
                };
                egui::Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(1.5, ink))
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        let r = ui.add(
                            egui::TextEdit::singleline(text)
                                .id(Id::unique("markup-text-field"))
                                .font(semi(TEXT_PT))
                                .text_color(ink)
                                .frame(egui::Frame::NONE)
                                .desired_width(180.0),
                        );
                        if !r.has_focus() && !r.lost_focus() {
                            r.request_focus();
                        }
                        if r.lost_focus() {
                            done = Some(!ui.input(|i| i.key_pressed(egui::Key::Escape)));
                        }
                    });
            });
        match done {
            Some(true) => self.agent.markup.commit_text(),
            Some(false) => self.agent.markup.typing = None,
            None => {}
        }
    }

    /// The tool strip at the top of the view: the tools, the colours, undo, clear, done.
    fn markup_strip(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        const B: f32 = 28.0;
        let w = 5.0 * B + 8.0 + 4.0 * 22.0 + 8.0 + 2.0 * B + 8.0 + 52.0 + 12.0;
        let at = pos2(self.view_rect.center().x - w * 0.5, self.view_rect.top() + 8.0);
        let mut places: Vec<(String, Rect)> = Vec::new();
        let m = &mut self.agent.markup;
        egui::Area::new(Id::unique("markup-strip"))
            .order(Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(w, B + 8.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(r, 6.0, t.raised);
                p.rect_stroke(r, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                let x = std::cell::Cell::new(r.left() + 6.0);
                let gap = || x.set(x.get() + 8.0);
                let y = r.top() + 4.0;
                let mut button = |ui: &mut Ui, name: String, wd: f32, tip: &str, on: bool| {
                    let b = Rect::from_min_size(pos2(x.get(), y), vec2(wd, B));
                    let resp = ui
                        .interact(b, Id::unique(("markup-b", &name)), Sense::click())
                        .on_hover_text(tip);
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
                    if on {
                        ui.painter().rect_filled(b, 4.0, t.on);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(b, 4.0, t.hover);
                    }
                    places.push((name, b));
                    x.set(x.get() + wd);
                    (b, resp.clicked())
                };
                for (tool, icon, tip) in Tool::ALL {
                    let (b, hit) = button(ui, format!("markup:{icon}"), B, tip, m.tool == tool);
                    icons::draw(ui.painter(), b.center() - vec2(8.0, 8.0), 16.0, icon, 1.5, t.ink);
                    if hit {
                        m.commit_text();
                        m.tool = tool;
                    }
                }
                gap();
                for (k, (name, c)) in INKS.iter().enumerate() {
                    let (b, hit) = button(ui, format!("markup:ink:{name}"), 22.0, name, false);
                    ui.painter().circle_filled(b.center(), 7.0, *c);
                    if m.ink == k {
                        ui.painter()
                            .circle_stroke(b.center(), 9.5, Stroke::new(1.5, t.ink));
                    }
                    if hit {
                        m.ink = k;
                    }
                }
                gap();
                let (b, hit) = button(ui, "markup:undo".into(), B, "Undo the last mark", false);
                icons::draw(
                    ui.painter(),
                    b.center() - vec2(8.0, 8.0),
                    16.0,
                    "undo",
                    1.5,
                    t.ink,
                );
                if hit {
                    m.typing = None;
                    m.marks.pop();
                }
                let (b, hit) = button(ui, "markup:clear".into(), B, "Clear the marks", false);
                icons::draw(
                    ui.painter(),
                    b.center() - vec2(8.0, 8.0),
                    16.0,
                    "delete",
                    1.5,
                    t.ink,
                );
                if hit {
                    m.typing = None;
                    m.marks.clear();
                }
                gap();
                let (b, hit) = button(ui, "markup:done".into(), 52.0, "Leave markup (Esc)", false);
                ui.painter()
                    .text(b.center(), Align2::CENTER_CENTER, "Done", medium(12.0), t.ink);
                if hit {
                    m.toggle();
                }
            });
        self.places.extend(places);
    }

    /// Esc while marking: a label being typed is the field's; else markup ends, or a sent
    /// message's marks shown again go.
    pub(super) fn markup_escape(&mut self, ctx: &egui::Context) -> bool {
        let m = &mut self.agent.markup;
        if m.typing.is_some() {
            return true;
        }
        if !m.on && m.shown.is_none() {
            return false;
        }
        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if m.on {
            m.toggle();
        } else {
            m.shown = None;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Under for Fake {
        fn at(&self, p: Pos2, reach: f32) -> Option<String> {
            // a face left of x = 100, an edge along x = 200
            if p.x < 100.0 {
                Some("face `extrude1.end`".into())
            } else if (p.x - 200.0).abs() <= reach.max(1.0) {
                Some("edge `e3`".into())
            } else {
                None
            }
        }

        fn within(&self, m: &Mark) -> Vec<(String, u32, u32, bool)> {
            let n = (0..100).filter(|k| m.inside(pos2(*k as f32 * 3.0, 50.0))).count() as u32;
            vec![
                ("occurrence `bracket:2`".into(), n, 60, true),
                ("face `F12`".into(), n / 2, 10, false),
                ("face `F13`".into(), 1, 50, false),
            ]
        }
    }

    fn mark(tool: Tool, pts: &[(f32, f32)], text: &str) -> Mark {
        Mark {
            tool,
            ink: 0,
            pts: pts.iter().map(|p| pos2(p.0, p.1)).collect(),
            text: text.into(),
        }
    }

    #[test]
    fn an_ellipse_holds_its_inside_and_a_loop_is_a_region() {
        let e = mark(Tool::Ellipse, &[(0.0, 0.0), (100.0, 50.0)], "");
        assert!(e.inside(pos2(50.0, 25.0)) && !e.inside(pos2(2.0, 2.0)));
        let ring: Vec<(f32, f32)> = (0..20)
            .map(|k| {
                let a = k as f32 / 19.0 * 6.2;
                (50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
            })
            .collect();
        let l = mark(Tool::Pen, &ring, "");
        assert!(l.closed() && l.region() && l.inside(pos2(50.0, 50.0)) && !l.inside(pos2(5.0, 5.0)));
        let open = mark(Tool::Pen, &[(0.0, 0.0), (50.0, 0.0), (100.0, 3.0)], "");
        assert!(!open.closed() && open.samples(4.0).len() > 20);
    }

    #[test]
    fn marks_read_as_what_they_are_on() {
        let marks = vec![
            mark(Tool::Ellipse, &[(0.0, 0.0), (300.0, 100.0)], ""),
            mark(Tool::Arrow, &[(50.0, 10.0), (200.0, 10.0)], ""),
            mark(Tool::Text, &[(30.0, 0.0), (90.0, 20.0)], "thicker"),
            mark(Tool::Pen, &[(10.0, 300.0), (250.0, 300.0)], ""),
        ];
        let said = describe(&marks, &Fake, &|p| (p.x as i32 * 2, p.y as i32 * 2));
        let lines: Vec<&str> = said.lines().collect();
        assert_eq!(lines.len(), 5, "{said}");
        assert!(
            lines[1].starts_with("1. ellipse, red, x -2–602")
                && lines[1].ends_with("around occurrence `bracket:2`, face `F12`"),
            "{said}"
        );
        assert!(
            lines[2].ends_with("from face `extrude1.end` pointing to edge `e3`"),
            "{said}"
        );
        assert!(
            lines[3].starts_with("3. text \"thicker\"")
                && lines[3].contains("on face `extrude1.end`")
                && lines[3].contains("at the tail of mark 2"),
            "{said}"
        );
        assert!(
            lines[4].ends_with("across face `extrude1.end`, edge `e3`"),
            "{said}"
        );
    }
}
