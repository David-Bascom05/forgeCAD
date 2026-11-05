//! Measure on screen, as Inventor's: a compact floating panel (the selection priority, what's
//! picked, every value with a button to add it up and one to copy it, the running totals) and
//! the results drawn in the model as dimensions (arrowed lines, angle arcs, tinted faces with
//! their areas, circles with their diameters), all in the world, so they stay put as it turns.

use super::Shell;
use super::fgasm::dialog_head;
use super::inspect::{Filter, MPick, shown};
use super::theme::{Theme, medium, mono, sans};
use egui::{Color32, Pos2, Rect, Stroke, Ui, pos2, vec2};
use forge_agent::measure::{Quantity, Shown};
use forge_math::{Point3, Vec3, dmath};

/// The dimensions' ink, a face's tint, and the labels' ground.
const INK: Color32 = Color32::from_rgb(0xFF, 0xD0, 0x40);
const TINT: Color32 = Color32::from_rgba_premultiplied(0x30, 0x70, 0xC0, 0x50);
const GROUND: Color32 = Color32::from_rgba_premultiplied(0x10, 0x12, 0x16, 0xE0);
const ARROW: f32 = 9.0;

/// A kind of value added up together: angles, lengths, areas, volumes.
fn kind_word(power: u8) -> &'static str {
    match power {
        0 => "Angles",
        2 => "Areas",
        3 => "Volumes",
        _ => "Lengths",
    }
}

/// What's added up: the values Inventor's Accumulate takes (not coordinates, not a mass).
fn adds_up(q: Quantity) -> bool {
    !matches!(
        q,
        Quantity::X | Quantity::Y | Quantity::Z | Quantity::Mass | Quantity::Dx | Quantity::Dy | Quantity::Dz
    )
}

impl Shell {
    pub(super) fn measure_window(
        &mut self,
        ctx: &egui::Context,
        t: &Theme,
        v: Rect,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let mut close = false;
        let at = pos2(v.left() + 16.0, v.top() + 56.0);
        let m = self.inspect.measure.as_mut()?;
        let title = if m.wall { "Wall Thickness" } else { "Measure" };
        let units = (m.unit, m.angle);
        let n = m.precision;
        let r = Self::inspect_window(title, "measure", t, at, 290.0).show(ctx, |ui| {
            close = dialog_head(ui, t, "measure", title, "measure:close", places);
            if m.wall {
                wall_rows(ui, t, m, places);
                return;
            }
            // the selection priority: what a click takes
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for f in Filter::ALL {
                    let on = m.filter == f;
                    let r = ui.add(
                        egui::Button::new(egui::RichText::new(f.word()).font(sans(11.0)).color(if on {
                            t.on_ink
                        } else {
                            t.ink
                        }))
                        .fill(if on { t.on } else { t.surface })
                        .small(),
                    );
                    places.push((format!("measure:filter:{}", f.word()), r.rect));
                    if r.clicked() && !on {
                        // as Inventor's: a new priority starts the measuring again
                        m.filter = f;
                        m.restart();
                    }
                }
            });
            ui.add_space(2.0);
            for k in 0..2 {
                let name = m.names.get(k).cloned().unwrap_or_else(|| match (k, m.picks.len()) {
                    (0, _) => "Pick anything: a face, edge, point, plane, body".into(),
                    (1, 0) => String::new(),
                    _ => "A second, to measure between".into(),
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(70.0, 16.0),
                        egui::Label::new(
                            egui::RichText::new(format!("Selection {}", k + 1))
                                .font(medium(11.0))
                                .color(t.ink),
                        ),
                    );
                    let r = ui.label(egui::RichText::new(name).font(sans(11.0)).color(t.ink2));
                    places.push((format!("measure:pick{}", k + 1), r.rect));
                });
            }
            ui.add_space(4.0);
            let mut add = None;
            match &m.reading {
                Some(Ok(r)) => {
                    for (q, x) in &r.values {
                        let text = shown(*q, *x, units, n);
                        if value_row(ui, t, q.label(), &text, adds_up(*q), places) {
                            add = Some((*q, *x));
                        }
                    }
                    if r.values.is_empty() {
                        ui.label(egui::RichText::new("Nothing to measure there").font(sans(11.0)).color(t.ink2));
                    }
                }
                Some(Err(e)) => {
                    ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
                }
                None if m.busy => {
                    ui.label(egui::RichText::new("Measuring...").font(sans(11.0)).color(t.ink2));
                }
                None => {}
            }
            if let Some(a) = add {
                m.accum.push(a);
            }
            // the running totals, a line a kind
            if !m.accum.is_empty() {
                ui.add_space(4.0);
                ui.separator();
                ui.label(egui::RichText::new("Accumulated").font(medium(11.0)).color(t.ink));
                for power in [1, 2, 3, 0] {
                    let of: Vec<f64> = m.accum.iter().filter(|a| a.0.power() == power).map(|a| a.1).collect();
                    if of.is_empty() {
                        continue;
                    }
                    let q = match power {
                        0 => Quantity::Angle,
                        2 => Quantity::Area,
                        3 => Quantity::Volume,
                        _ => Quantity::Length,
                    };
                    let total = shown(q, of.iter().sum(), units, n);
                    let words = format!("{} ({})", kind_word(power), of.len());
                    let r = ui
                        .horizontal(|ui| {
                            ui.add_sized(
                                vec2(118.0, 16.0),
                                egui::Label::new(egui::RichText::new(&words).font(sans(11.0)).color(t.ink2)),
                            );
                            ui.label(egui::RichText::new(&total).font(mono(12.0)).color(t.ink))
                        })
                        .inner;
                    places.push((format!("measure:accum:{}", kind_word(power)), r.rect));
                }
                let c = ui.small_button(egui::RichText::new("Clear totals").font(sans(10.0)));
                places.push(("measure:clear".into(), c.rect));
                if c.clicked() {
                    m.accum.clear();
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let all = ui.small_button(egui::RichText::new("Copy all").font(sans(11.0)));
                places.push(("measure:copyall".into(), all.rect));
                if all.clicked()
                    && let Some(Ok(r)) = &m.reading
                {
                    let text: Vec<String> = r
                        .values
                        .iter()
                        .map(|(q, x)| format!("{}\t{}", q.label(), shown(*q, *x, units, n)))
                        .collect();
                    ui.ctx().copy_text(text.join("\n"));
                }
                let r = ui.small_button(egui::RichText::new("Restart").font(sans(11.0)));
                places.push(("measure:restart".into(), r.rect));
                if r.clicked() {
                    m.restart();
                }
                let r = egui::ComboBox::from_id_salt("measure-precision")
                    .selected_text(format!("{} dp", m.precision))
                    .width(48.0)
                    .show_ui(ui, |ui| {
                        for p in 0..=6 {
                            ui.selectable_value(&mut m.precision, p, format!("{p}"));
                        }
                    });
                places.push(("measure:precision".into(), r.response.rect));
                let r = egui::ComboBox::from_id_salt("measure-unit")
                    .selected_text(m.unit)
                    .width(40.0)
                    .show_ui(ui, |ui| {
                        for u in forge_doc::units::DOC_LENGTHS {
                            ui.selectable_value(&mut m.unit, u, u);
                        }
                    });
                places.push(("measure:unit".into(), r.response.rect));
            });
            ui.label(
                egui::RichText::new("Esc clears, Enter starts again")
                    .font(sans(10.0))
                    .color(t.ink2),
            );
        });
        if close {
            self.inspect.measure = None;
        }
        r.map(|r| r.response.rect)
    }

    /// The reading drawn in the model: each dimension where it measures, in the world, so it
    /// stays on what it measures as the view turns.
    pub(super) fn measure_marks(&mut self, ctx: &egui::Context) {
        let Some((camera, size)) = self.camera else { return };
        let Some(m) = &self.inspect.measure else { return };
        let v = self.view_rect;
        let ppp = ctx.pixels_per_point();
        let to_screen = |p: Point3| {
            camera
                .pixel_of([p.x as f32, p.y as f32, p.z as f32], size.0, size.1)
                .map(|q| v.min + vec2(q.0, q.1) / ppp)
        };
        // on the view's own layer: over the picture, under every panel
        let painter = ctx.layer_painter(egui::LayerId::background()).with_clip_rect(v);
        // what's picked that's no body's: drawn over, as a body's faces and edges are marked
        let reach = self.inspect.bounds.1 as f64;
        for p in m.picks.iter().chain(m.hover.iter()) {
            if let MPick::Free(f, _) = p {
                free_shape(&painter, &to_screen, f, reach);
            }
        }
        let Some(Ok(r)) = &m.reading else { return };
        let units = (m.unit, m.angle);
        let n = m.precision;
        let value = |q: Quantity| r.get(q).map(|x| shown(q, x, units, n));
        let mut labels = Labels {
            painter: &painter,
            taken: vec![],
            places: vec![],
        };
        let line = Stroke::new(1.5, INK);
        for s in &r.shown {
            match s {
                Shown::Face(tris, c) => {
                    for tri in tris {
                        let pts: Option<Vec<Pos2>> = tri.iter().map(|p| to_screen(*p)).collect();
                        if let Some(pts) = pts {
                            painter.add(egui::Shape::convex_polygon(pts, TINT, Stroke::NONE));
                        }
                    }
                    if let Some(at) = to_screen(*c) {
                        let mut text = vec![];
                        if let Some(a) = value(Quantity::Area) {
                            text.push(format!("A {a}"));
                        }
                        if let Some(p) = value(Quantity::Perimeter) {
                            text.push(format!("P {p}"));
                        }
                        labels.put(at, at + vec2(28.0, -28.0), text.join("\n"), "Area", line);
                    }
                }
                // two that touch: nothing between them to draw (the angle says it)
                Shown::Line(a, b, _) if a.distance(*b) < 1e-9 => {}
                Shown::Line(a, b, q) => {
                    let (Some(pa), Some(pb)) = (to_screen(*a), to_screen(*b)) else {
                        continue;
                    };
                    let stroke = if *q == Quantity::Distance {
                        line
                    } else {
                        Stroke::new(1.0, INK.gamma_multiply(0.7))
                    };
                    dimension(&painter, pa, pb, stroke);
                    let text = value(*q).unwrap_or_else(|| shown(*q, a.distance(*b), units, n));
                    let mid = pa + (pb - pa) * 0.5;
                    // beside the line, square to it on screen
                    let along = (pb - pa).normalized();
                    let side = if along.length() > 0.0 {
                        vec2(-along.y, along.x)
                    } else {
                        vec2(0.0, -1.0)
                    };
                    let off = if *q == Quantity::Distance { 16.0 } else { -16.0 };
                    let words = if *q == Quantity::Distance {
                        text
                    } else {
                        format!("{} {text}", q.label())
                    };
                    labels.put(mid, mid + side * off, words, q.label(), Stroke::NONE);
                }
                Shown::Point(p) => {
                    if let Some(q) = to_screen(*p) {
                        painter.circle_stroke(q, 5.0, line);
                        painter.line_segment([q - vec2(9.0, 0.0), q + vec2(9.0, 0.0)], Stroke::new(1.0, INK));
                        painter.line_segment([q - vec2(0.0, 9.0), q + vec2(0.0, 9.0)], Stroke::new(1.0, INK));
                        if r.get(Quantity::Area).is_none()
                            && let (Some(x), Some(y), Some(z)) =
                                (r.get(Quantity::X), r.get(Quantity::Y), r.get(Quantity::Z))
                        {
                            let k = |v: f64| shown(Quantity::Length, v, units, n);
                            let text = format!("X {}\nY {}\nZ {}", k(x), k(y), k(z));
                            labels.put(q, q + vec2(26.0, -26.0), text, "X", line);
                        }
                    }
                }
                Shown::Circle(c, z, rad) => {
                    let (u, w) = across(*z);
                    let ring: Vec<Pos2> = (0..=64)
                        .filter_map(|k| {
                            let a = k as f64 / 64.0 * std::f64::consts::TAU;
                            to_screen(*c + (u * dmath::cos(a) + w * dmath::sin(a)) * *rad)
                        })
                        .collect();
                    painter.add(egui::Shape::line(ring, line));
                    // a leader from the rim, out to the diameter
                    let rim = *c + (u + w) * (*rad / 2f64.sqrt());
                    if let (Some(cs), Some(rs)) = (to_screen(*c), to_screen(rim)) {
                        painter.circle_filled(cs, 2.5, INK);
                        let out = (rs - cs).normalized();
                        let out = if out.length() > 0.0 { out } else { vec2(0.7, -0.7) };
                        arrow(&painter, rs, rs - out * ARROW, INK);
                        let text = format!("Ø {}", shown(Quantity::Diameter, 2.0 * rad, units, n));
                        labels.put(rs, rs + out * 34.0, text, "Diameter", line);
                    }
                }
                Shown::Curve(pts, q) => {
                    let ps: Vec<Pos2> = pts.iter().filter_map(|p| to_screen(*p)).collect();
                    painter.add(egui::Shape::line(ps.clone(), Stroke::new(3.0, INK)));
                    if let Some(mid) = ps.get(ps.len() / 2).copied()
                        && let Some(text) = value(*q)
                    {
                        labels.put(mid, mid + vec2(24.0, -24.0), format!("L {text}"), "Length", line);
                    }
                }
                Shown::Angle(vtx, u, w, deg) => {
                    let Some(vs) = to_screen(*vtx) else { continue };
                    // the arc 48 points out on screen, whatever the zoom
                    let per_mm = to_screen(*vtx + *u)
                        .map_or(1.0, |q| (q - vs).length().max(1e-3) as f64);
                    let reach = 48.0 / per_mm;
                    let w2 = (*w - *u * u.dot(*w)).normalize().map_or(*w, |x| x.get());
                    let sweep = deg.to_radians();
                    let at = |t: f64| to_screen(*vtx + (*u * dmath::cos(t) + w2 * dmath::sin(t)) * reach);
                    let arc: Vec<Pos2> = (0..=32).filter_map(|k| at(sweep * k as f64 / 32.0)).collect();
                    if arc.len() < 2 {
                        continue;
                    }
                    for arm in [*u, *w] {
                        if let Some(e) = to_screen(*vtx + arm * reach * 1.25) {
                            painter.line_segment([vs, e], Stroke::new(1.0, INK.gamma_multiply(0.7)));
                        }
                    }
                    painter.add(egui::Shape::line(arc.clone(), line));
                    arrow(&painter, arc[0], arc[1], INK);
                    arrow(&painter, arc[arc.len() - 1], arc[arc.len() - 2], INK);
                    if let Some(mid) = at(sweep * 0.5) {
                        let out = (mid - vs).normalized();
                        let text = shown(Quantity::Angle, *deg, units, n);
                        labels.put(mid, mid + out * 22.0, text, "Angle", Stroke::NONE);
                    }
                }
                Shown::Box(lo, hi) => {
                    let c = |i: usize| {
                        Point3::new(
                            if i & 1 == 0 { lo.x } else { hi.x },
                            if i & 2 == 0 { lo.y } else { hi.y },
                            if i & 4 == 0 { lo.z } else { hi.z },
                        )
                    };
                    for i in 0..8usize {
                        for bit in [1, 2, 4] {
                            if i & bit == 0
                                && let (Some(a), Some(b)) = (to_screen(c(i)), to_screen(c(i | bit)))
                            {
                                dashed(&painter, a, b, Stroke::new(1.0, INK.gamma_multiply(0.8)));
                            }
                        }
                    }
                    if let (Some(x), Some(y), Some(z), Some(top)) = (
                        value(Quantity::SizeX),
                        value(Quantity::SizeY),
                        value(Quantity::SizeZ),
                        to_screen(c(7)),
                    ) {
                        labels.put(top, top + vec2(30.0, -30.0), format!("{x} × {y} × {z}"), "Box", line);
                    }
                }
            }
        }
        let places = labels.places;
        self.places.extend(places);
    }
}

/// The thinnest wall's rows.
fn wall_rows(ui: &mut Ui, t: &Theme, m: &super::inspect::MeasurePanel, places: &mut Vec<(String, Rect)>) {
    ui.label(
        egui::RichText::new("The thinnest wall of the part picked, or of the part")
            .font(sans(11.0))
            .color(t.ink2),
    );
    match &m.wall_at {
        Some(Ok((w, p))) => {
            let text = shown(Quantity::Length, *w, (m.unit, m.angle), m.precision);
            value_row(ui, t, "Thinnest wall", &text, false, places);
            value_row(ui, t, "At", &format!("{:.1}, {:.1}, {:.1}", p.x, p.y, p.z), false, places);
        }
        Some(Err(e)) => {
            ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
        }
        None => {
            ui.label(egui::RichText::new("Working it out...").font(sans(11.0)).color(t.ink2));
        }
    }
}

/// A value: its words, the value, a button to add it to the totals (if it adds up) and one to
/// copy it. True when it's to be added.
fn value_row(ui: &mut Ui, t: &Theme, what: &str, value: &str, adds: bool, places: &mut Vec<(String, Rect)>) -> bool {
    let mut added = false;
    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(112.0, 16.0),
            egui::Label::new(egui::RichText::new(what).font(sans(11.0)).color(t.ink2)),
        );
        let r = ui.label(egui::RichText::new(value).font(mono(12.0)).color(t.ink));
        places.push((format!("measure:value:{what}"), r.rect));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let c = ui
                .add(egui::Button::new(egui::RichText::new("Copy").font(sans(10.0))).small())
                .on_hover_text("Copy the value");
            places.push((format!("measure:copy:{what}"), c.rect));
            if c.clicked() {
                ui.ctx().copy_text(value.to_string());
            }
            if adds {
                let a = ui
                    .add(egui::Button::new(egui::RichText::new("+").font(sans(11.0))).small())
                    .on_hover_text("Add to the totals");
                places.push((format!("measure:add:{what}"), a.rect));
                added = a.clicked();
            }
        });
    });
    added
}

/// Labels set in the view, each where it's free of those before, with a leader to what it says.
struct Labels<'a> {
    painter: &'a egui::Painter,
    taken: Vec<Rect>,
    places: Vec<(String, Rect)>,
}

impl Labels<'_> {
    fn put(&mut self, anchor: Pos2, want: Pos2, text: String, key: &str, leader: Stroke) {
        let g = self.painter.layout_no_wrap(text, mono(12.0), INK);
        let mut r = Rect::from_center_size(want, g.size() + vec2(10.0, 6.0));
        // down out of the way of the ones already set
        for _ in 0..12 {
            match self.taken.iter().find(|o| o.expand(2.0).intersects(r)) {
                Some(o) => r = r.translate(vec2(0.0, o.bottom() - r.top() + 4.0)),
                None => break,
            }
        }
        if leader != Stroke::NONE {
            let to = pos2(anchor.x.clamp(r.left(), r.right()), anchor.y.clamp(r.top(), r.bottom()));
            self.painter.line_segment([anchor, to], leader);
        }
        self.painter.rect_filled(r, 3.0, GROUND);
        self.painter
            .rect_stroke(r, 3.0, Stroke::new(1.0, INK.gamma_multiply(0.6)), egui::StrokeKind::Inside);
        self.painter.galley(r.min + vec2(5.0, 3.0), g, INK);
        self.taken.push(r);
        self.places.push((format!("measure:mark:{key}"), r));
    }
}

/// A dimension line with an arrowhead at each end, pointing out to what it measures.
fn dimension(p: &egui::Painter, a: Pos2, b: Pos2, s: Stroke) {
    p.line_segment([a, b], s);
    if (b - a).length() > 2.5 * ARROW {
        arrow(p, a, b, s.color);
        arrow(p, b, a, s.color);
    } else {
        p.circle_filled(a, 2.5, s.color);
        p.circle_filled(b, 2.5, s.color);
    }
}

/// A filled arrowhead at `tip`, coming from `from`.
fn arrow(p: &egui::Painter, tip: Pos2, from: Pos2, ink: Color32) {
    let way = (tip - from).normalized();
    if way.length() == 0.0 {
        return;
    }
    let side = vec2(-way.y, way.x);
    p.add(egui::Shape::convex_polygon(
        vec![tip, tip - way * ARROW + side * 3.5, tip - way * ARROW - side * 3.5],
        ink,
        Stroke::NONE,
    ));
}

fn dashed(p: &egui::Painter, a: Pos2, b: Pos2, s: Stroke) {
    let len = (b - a).length();
    let n = (len / 8.0).ceil().max(1.0) as usize;
    for k in (0..n).step_by(2) {
        let (t0, t1) = (k as f32 / n as f32, ((k + 1) as f32 / n as f32).min(1.0));
        p.line_segment([a + (b - a) * t0, a + (b - a) * t1], s);
    }
}

/// Two ways square to `z` and to each other.
fn across(z: Vec3) -> (Vec3, Vec3) {
    let z = z.normalize().map_or(Vec3::Z, |u| u.get());
    let x = if z.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = z.cross(x).normalize().map_or(x, |u| u.get());
    (u, z.cross(u))
}

/// Free geometry picked, drawn over the model: a point's cross, a line, a circle, a curve, an
/// axis as a long dashed line, a plane as a square about its origin.
fn free_shape(p: &egui::Painter, to_screen: &dyn Fn(Point3) -> Option<Pos2>, f: &forge_agent::measure::Free, reach: f64) {
    use forge_agent::measure::Free;
    let s = Stroke::new(2.5, Color32::from_rgb(0x40, 0xC0, 0xF0));
    let poly = |pts: Vec<Point3>| {
        let ps: Vec<Pos2> = pts.into_iter().filter_map(to_screen).collect();
        p.add(egui::Shape::line(ps, s));
    };
    let reach = reach.max(10.0);
    match f {
        Free::Point(q) => {
            if let Some(c) = to_screen(*q) {
                p.circle_stroke(c, 6.0, s);
            }
        }
        Free::Segment(a, b) => poly(vec![*a, *b]),
        Free::Circle(c, z, r) => {
            let (u, w) = across(*z);
            poly(
                (0..=64)
                    .map(|k| {
                        let t = k as f64 / 64.0 * std::f64::consts::TAU;
                        *c + (u * dmath::cos(t) + w * dmath::sin(t)) * *r
                    })
                    .collect(),
            );
        }
        Free::Arc(_, _, _, pts) | Free::Curve(pts) => poly(pts.clone()),
        Free::Axis(o, u) => {
            if let (Some(a), Some(b)) = (to_screen(*o - *u * reach), to_screen(*o + *u * reach)) {
                dashed(p, a, b, s);
            }
        }
        Free::Plane(o, n) => {
            let (u, w) = across(*n);
            let k = reach * 0.6;
            poly(vec![
                *o + (u + w) * k,
                *o + (u - w) * k,
                *o - (u + w) * k,
                *o - (u - w) * k,
                *o + (u + w) * k,
            ]);
        }
    }
}
