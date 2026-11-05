//! Measure on screen, as Inventor's: a compact floating panel (the selection priority, what's
//! picked, every value with a button to add it up and one to copy it, the running totals) and
//! the results drawn in the model as dimensions (arrowed lines, angle arcs, tinted faces with
//! their areas, circles with their diameters), all in the world, so they stay put as it turns.

use super::Shell;
use super::fgasm::dialog_head;
use super::inspect::{Added, Filter, Follow, IntoField, MPick, panel_values, shown, shown_dual};
use super::theme::{Theme, medium, mono, sans};
use egui::{Color32, Pos2, Rect, Stroke, Ui, pos2, vec2};
use forge_agent::measure::{Quantity, Shown};
use forge_math::{Point3, Vec3, dmath};

/// The dimensions' ink, a face's tint, and the labels' ground.
const INK: Color32 = Color32::from_rgb(0xFF, 0xD0, 0x40);
const GROUND: Color32 = Color32::from_rgba_premultiplied(0x10, 0x12, 0x16, 0xE0);
const ARROW: f32 = 9.0;
/// The values' words' column, till it's dragged.
const LABEL_W: f32 = 112.0;

/// A kind of value added up together: angles, lengths, areas, volumes.
fn kind_word(power: u8) -> &'static str {
    match power {
        0 => "Angles",
        2 => "Areas",
        3 => "Volumes",
        4 => "Masses",
        _ => "Lengths",
    }
}

/// The quantity a group's total is shown as.
fn group_quantity(power: u8) -> Quantity {
    match power {
        0 => Quantity::Angle,
        2 => Quantity::Area,
        3 => Quantity::Volume,
        4 => Quantity::Mass,
        _ => Quantity::Length,
    }
}

/// Selection 1's and 2's colours, as Inventor shows each set in its own.
const SET_INK: [Color32; 2] = [Color32::from_rgb(0xF0, 0x90, 0x30), Color32::from_rgb(0x40, 0xC8, 0x90)];

/// What's added up: the values Inventor's Accumulate takes (lengths, angles, areas, volumes,
/// masses; not coordinates or deltas).
fn adds_up(q: Quantity) -> bool {
    !matches!(q, Quantity::X | Quantity::Y | Quantity::Z | Quantity::Dx | Quantity::Dy | Quantity::Dz)
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
        let dialog_up = self.dialog.is_some() || self.fg.as_ref().is_some_and(|f| f.tool.is_some());
        let beside = self.inspect.measure.as_ref().is_some_and(|m| m.into.is_some()) && dialog_up;
        let at = if beside {
            pos2(v.left() + super::dialog::DIALOG_W + 56.0, v.top() + 16.0)
        } else {
            pos2(v.left() + 16.0, v.top() + 56.0)
        };
        // the frames points can be read in: the work planes' and sketches'
        let frames: Vec<(String, forge_math::Frame)> = self
            .fg
            .as_ref()
            .map(|f| f.feats.iter().filter_map(|r| r.frame.map(|fr| (r.id.clone(), fr))).collect())
            .unwrap_or_default();
        let m = self.inspect.measure.as_mut()?;
        let title = if m.wall { "Wall Thickness" } else { "Measure" };
        let units = (m.unit, m.angle);
        let n = m.precision;
        // wider with a second unit beside every value
        let width = if m.dual.is_some() { 400.0 } else { 330.0 };
        let r = Self::inspect_window(title, "measure", t, at, width).show(ctx, |ui| {
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
            for k in 0..m.picks.len().max(2) {
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
                                .color(SET_INK[k.min(1)]),
                        ),
                    );
                    let r = ui.label(egui::RichText::new(name).font(sans(11.0)).color(t.ink2));
                    places.push((format!("measure:pick{}", k + 1), r.rect));
                });
            }
            ui.add_space(4.0);
            // the words' column, as wide as it's dragged
            let col = if m.label_w > 0.0 { m.label_w } else { LABEL_W };
            let dual = m.dual;
            let text_of = |q: Quantity, x: f64| shown_dual(q, x, units, n, dual);
            let frame = m.ucs.as_ref().and_then(|id| frames.iter().find(|f| &f.0 == id)).map(|f| f.1);
            // what the reading lists, worked out before the panel changes anything
            let listed = match &m.reading {
                Some(Ok(r)) => {
                    let kinds: Vec<Follow> =
                        Follow::ALL.into_iter().filter(|f| r.get(f.quantity()).is_some()).collect();
                    Some(Ok((panel_values(r, m.follow, frame.as_ref(), m.hide_delta), kinds)))
                }
                Some(Err(e)) => Some(Err(e.clone())),
                None => None,
            };
            let mut act = None;
            match &listed {
                Some(Ok((values, kinds))) => {
                    // which distance the deltas follow, as Inventor's toggle
                    if kinds.len() > 1 {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            ui.add_sized(
                                vec2(col, 16.0),
                                egui::Label::new(egui::RichText::new("Delta follows").font(sans(11.0)).color(t.ink2)),
                            );
                            for f in kinds {
                                let on = m.follow == *f;
                                let r = ui.add(
                                    egui::Button::new(egui::RichText::new(f.word()).font(sans(10.0)).color(if on {
                                        t.on_ink
                                    } else {
                                        t.ink
                                    }))
                                    .fill(if on { t.on } else { t.surface })
                                    .small(),
                                );
                                places.push((format!("measure:follow:{}", f.word()), r.rect));
                                if r.clicked() {
                                    m.follow = *f;
                                }
                            }
                        });
                    }
                    let top = ui.cursor().top();
                    for (q, x) in values {
                        let text = text_of(*q, *x);
                        let focused = m.focus == Some(*q);
                        // into a length field a length, into an angle field an angle
                        let usable = match &m.into {
                            Some((_, forge_doc::units::Kind::Angle)) => q.power() == 0,
                            Some((_, forge_doc::units::Kind::Length)) => q.power() == 1,
                            None => false,
                        };
                        match value_row(ui, t, q.label(), &text, adds_up(*q), focused, usable, col, places) {
                            Row::Use => act = Some(Row::Use.with(*q, *x)),
                            Row::None => {}
                            Row::Add => act = Some(Row::Add.with(*q, *x)),
                            Row::Sub => act = Some(Row::Sub.with(*q, *x)),
                            Row::Focus => act = Some(Row::Focus.with(*q, *x)),
                            Row::CopyAll => act = Some(Row::CopyAll.with(*q, *x)),
                        }
                    }
                    // between the words and the values: drag it to widen or narrow the words
                    let left = ui.min_rect().left();
                    let edge = Rect::from_min_max(
                        pos2(left + col - 3.0, top),
                        pos2(left + col + 3.0, ui.cursor().top().max(top + 4.0)),
                    );
                    let h = ui.interact(edge, ui.scope_id().with("measure-column"), egui::Sense::drag());
                    places.push(("measure:column".into(), edge));
                    if h.hovered() || h.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        ui.painter().line_segment(
                            [pos2(edge.center().x, edge.top()), pos2(edge.center().x, edge.bottom())],
                            Stroke::new(1.0, t.ink2),
                        );
                    }
                    if h.dragged() {
                        m.label_w = (col + h.drag_delta().x).clamp(60.0, 220.0);
                    }
                    if values.is_empty() {
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
            let what = m.names.join(" / ");
            let mut copy_all = false;
            match act {
                Some((Row::Add, q, x)) => m.accum.push(Added { q, x, what }),
                Some((Row::Sub, q, x)) => m.accum.push(Added { q, x: -x, what }),
                Some((Row::Focus, q, _)) => {
                    m.focus = if m.focus == Some(q) { None } else { Some(q) };
                    // a distance clicked is the one the deltas follow
                    if let Some(f) = Follow::ALL.into_iter().find(|f| f.quantity() == q) {
                        m.follow = f;
                    }
                }
                Some((Row::CopyAll, _, _)) => copy_all = true,
                Some((Row::Use, _, x)) => m.used = Some(x),
                _ => {}
            }
            // the running totals, a group a kind, each with what went into it
            if !m.accum.is_empty() {
                ui.add_space(4.0);
                ui.separator();
                ui.label(egui::RichText::new("Accumulated").font(medium(11.0)).color(t.ink));
                let (mut drop_one, mut drop_kind) = (None, None);
                for power in [1, 2, 3, 4, 0] {
                    let of: Vec<usize> = (0..m.accum.len()).filter(|&i| m.accum[i].q.power() == power).collect();
                    if of.is_empty() {
                        continue;
                    }
                    let q = group_quantity(power);
                    let total = text_of(q, of.iter().map(|&i| m.accum[i].x).sum());
                    let words = format!("{} ({})", kind_word(power), of.len());
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            vec2(col, 16.0),
                            egui::Label::new(egui::RichText::new(&words).font(medium(11.0)).color(t.ink)),
                        );
                        let r = ui.label(egui::RichText::new(&total).font(mono(12.0)).color(t.ink));
                        places.push((format!("measure:accum:{}", kind_word(power)), r.rect));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let x = ui.small_button(egui::RichText::new("×").font(sans(11.0))).on_hover_text("Delete the group");
                            places.push((format!("measure:accum:drop:{}", kind_word(power)), x.rect));
                            if x.clicked() {
                                drop_kind = Some(power);
                            }
                        });
                    });
                    for &i in &of {
                        let a = &m.accum[i];
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.add_sized(
                                vec2(col - 10.0, 14.0),
                                egui::Label::new(egui::RichText::new(&a.what).font(sans(10.0)).color(t.ink2)).truncate(),
                            );
                            ui.label(egui::RichText::new(text_of(a.q, a.x)).font(mono(11.0)).color(t.ink2));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let x = ui.small_button(egui::RichText::new("×").font(sans(10.0))).on_hover_text("Delete the value");
                                places.push((format!("measure:accum:entry:{i}"), x.rect));
                                if x.clicked() {
                                    drop_one = Some(i);
                                }
                            });
                        });
                    }
                }
                if let Some(i) = drop_one {
                    m.accum.remove(i);
                }
                if let Some(p) = drop_kind {
                    m.accum.retain(|a| a.q.power() != p);
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
                copy_all |= all.clicked();
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
            if copy_all {
                let mut text: Vec<String> = match &listed {
                    Some(Ok((values, _))) => {
                        values.iter().map(|(q, x)| format!("{}\t{}", q.label(), text_of(*q, *x))).collect()
                    }
                    _ => vec![],
                };
                for power in [1, 2, 3, 4, 0] {
                    let of: Vec<f64> = m.accum.iter().filter(|a| a.q.power() == power).map(|a| a.x).collect();
                    if !of.is_empty() {
                        let total = text_of(group_quantity(power), of.iter().sum());
                        text.push(format!("{} total\t{total}", kind_word(power)));
                    }
                }
                ui.ctx().copy_text(text.join("\n"));
            }
            // Inventor's Advanced Settings: a second unit, the coordinate system, the deltas
            let words = if m.advanced { "Hide advanced settings" } else { "Advanced settings..." };
            let a = ui.add(egui::Button::new(egui::RichText::new(words).font(sans(11.0)).color(t.ink2)).frame(false));
            places.push(("measure:advanced".into(), a.rect));
            if a.clicked() {
                m.advanced = !m.advanced;
            }
            if m.advanced {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(col, 16.0),
                        egui::Label::new(egui::RichText::new("Dual unit").font(sans(11.0)).color(t.ink2)),
                    );
                    let r = egui::ComboBox::from_id_salt("measure-dual")
                        .selected_text(m.dual.unwrap_or("None"))
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut m.dual, None, "None");
                            for u in forge_doc::units::DOC_LENGTHS {
                                ui.selectable_value(&mut m.dual, Some(u), u);
                            }
                        });
                    places.push(("measure:dual".into(), r.response.rect));
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(col, 16.0),
                        egui::Label::new(egui::RichText::new("Coordinate system").font(sans(11.0)).color(t.ink2)),
                    );
                    let r = egui::ComboBox::from_id_salt("measure-ucs")
                        .selected_text(m.ucs.clone().unwrap_or("World".into()))
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut m.ucs, None, "World");
                            for (id, _) in &frames {
                                ui.selectable_value(&mut m.ucs, Some(id.clone()), id);
                            }
                        });
                    places.push(("measure:ucs".into(), r.response.rect));
                });
                let mut loops = !m.no_loops;
                let r = ui
                    .checkbox(&mut loops, egui::RichText::new("Detect sketch loops").font(sans(11.0)))
                    .on_hover_text("In a sketch, a click inside a closed region measures its area and perimeter");
                places.push(("measure:loops".into(), r.rect));
                m.no_loops = !loops;
                                let mut show = !m.hide_delta;
                let r = ui.checkbox(&mut show, egui::RichText::new("Show Delta X, Y, Z").font(sans(11.0)));
                places.push(("measure:delta".into(), r.rect));
                m.hide_delta = !show;
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Esc clears, Enter starts again. Shift on a third point: the angle at the second")
                        .font(sans(10.0))
                        .color(t.ink2),
                )
                .wrap(),
            );
        });
        // a value Use was pressed on: into the field, and Measure's done, as Inventor's
        let used = m.used.take().zip(m.into.clone());
        if let Some((x, (into, kind))) = used {
            match into {
                IntoField::Dialog(label) => {
                    if let Some(d) = &mut self.dialog
                        && let Some(f) = d.fields.iter_mut().find(|f| f.0 == label)
                    {
                        f.1 = forge_doc::units::number_in(x, kind, d.units);
                    }
                }
                // a tool's parameters are mm and degrees
                IntoField::Param(name) => {
                    if let Some(p) = self
                        .fg
                        .as_mut()
                        .and_then(|f| f.tool.as_mut())
                        .and_then(|t| t.params.iter_mut().find(|p| p.name == name))
                    {
                        p.text = plain_number(x);
                        p.expr = None;
                    }
                }
            }
            close = true;
        }
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
            segs: vec![],
            wants: vec![],
        };
        // the distance the deltas follow is the one drawn
        let follow = {
            let q = m.follow.quantity();
            let drawn = r.shown.iter().any(|s| matches!(s, Shown::Line(_, _, k) if *k == q));
            if drawn { q } else { Quantity::Distance }
        };
        let base = Stroke::new(1.5, INK);
        for s in &r.shown {
            // the value clicked in the panel forward, the rest back
            let line = match m.focus {
                Some(f) if f == quantity_of(s) => Stroke::new(2.5, INK),
                Some(_) => Stroke::new(1.0, INK.gamma_multiply(0.35)),
                None => base,
            };
            match s {
                // the face's tint is the view's own mark, drawn where the face is seen
                Shown::Face(_, c) => {
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
                Shown::Line(_, _, q) if is_distance(*q) && *q != follow => {}
                Shown::Line(a, b, q) => {
                    let (Some(pa), Some(pb)) = (to_screen(*a), to_screen(*b)) else {
                        continue;
                    };
                    labels.seg(pa, pb);
                    let stroke = if is_distance(*q) {
                        line
                    } else {
                        Stroke::new(1.0, line.color.gamma_multiply(0.7))
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
                    labels.poly(&ring);
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
                    labels.poly(&ps);
                    painter.add(egui::Shape::line(ps.clone(), Stroke::new(line.width + 1.5, line.color)));
                    // a length along its curve; a loop's perimeter is with its area
                    if let Some(mid) = ps.get(ps.len() / 2).copied()
                        && *q == Quantity::Length
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
                    labels.poly(&arc);
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
                                labels.seg(a, b);
                                dashed(&painter, a, b, Stroke::new(1.0, line.color.gamma_multiply(0.8)));
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
        let places = labels.finish();
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
            value_row(ui, t, "Thinnest wall", &text, false, false, false, LABEL_W, places);
            value_row(ui, t, "At", &format!("{:.1}, {:.1}, {:.1}", p.x, p.y, p.z), false, false, false, LABEL_W, places);
        }
        Some(Err(e)) => {
            ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
        }
        None => {
            ui.label(egui::RichText::new("Working it out...").font(sans(11.0)).color(t.ink2));
        }
    }
}

/// A number as a field takes it: to a micron (or a millionth of a degree), no trailing zeros.
fn plain_number(x: f64) -> String {
    let s = format!("{x:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

/// What a click on a value's row asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    None,
    /// Into the dialog's field Measure was opened from.
    Use,
    Add,
    Sub,
    Focus,
    CopyAll,
}

impl Row {
    fn with(self, q: Quantity, x: f64) -> (Row, Quantity, f64) {
        (self, q, x)
    }
}

/// A value: its words (a click shows it in the view), the value (right-click copies it or
/// all), + and − for the totals (if it adds up), and Copy.
#[allow(clippy::too_many_arguments)]
fn value_row(
    ui: &mut Ui,
    t: &Theme,
    what: &str,
    value: &str,
    adds: bool,
    focused: bool,
    usable: bool,
    w: f32,
    places: &mut Vec<(String, Rect)>,
) -> Row {
    let mut out = Row::None;
    ui.horizontal(|ui| {
        let words = egui::RichText::new(what).font(if focused { medium(11.0) } else { sans(11.0) });
        let w = ui.add_sized(
            vec2(w, 16.0),
            egui::Label::new(words.color(if focused { t.ink } else { t.ink2 })).sense(egui::Sense::click()),
        );
        places.push((format!("measure:label:{what}"), w.rect));
        if w.clicked() {
            out = Row::Focus;
        }
        let r = ui
            .add(egui::Label::new(egui::RichText::new(value).font(mono(12.0)).color(t.ink)).sense(egui::Sense::click()));
        places.push((format!("measure:value:{what}"), r.rect));
        if r.clicked() {
            out = Row::Focus;
        }
        r.context_menu(|ui| {
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(value.to_string());
                ui.close();
            }
            if ui.button("Copy All").clicked() {
                out = Row::CopyAll;
                ui.close();
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let c = ui
                .add(egui::Button::new(egui::RichText::new("Copy").font(sans(10.0))).small())
                .on_hover_text("Copy the value");
            places.push((format!("measure:copy:{what}"), c.rect));
            if c.clicked() {
                ui.ctx().copy_text(value.to_string());
            }
            if usable {
                let u = ui
                    .add(egui::Button::new(egui::RichText::new("Use").font(sans(10.0))).small())
                    .on_hover_text("Put the value in the field");
                places.push((format!("measure:use:{what}"), u.rect));
                if u.clicked() {
                    out = Row::Use;
                }
            }
            if adds {
                let s = ui
                    .add(egui::Button::new(egui::RichText::new("−").font(sans(11.0))).small())
                    .on_hover_text("Take from the totals");
                places.push((format!("measure:sub:{what}"), s.rect));
                if s.clicked() {
                    out = Row::Sub;
                }
                let a = ui
                    .add(egui::Button::new(egui::RichText::new("+").font(sans(11.0))).small())
                    .on_hover_text("Add to the totals");
                places.push((format!("measure:add:{what}"), a.rect));
                if a.clicked() {
                    out = Row::Add;
                }
            }
        });
    });
    out
}

/// The quantity a drawn thing stands for: what a click on a panel value brings forward.
fn quantity_of(s: &Shown) -> Quantity {
    match s {
        Shown::Line(_, _, q) | Shown::Curve(_, q) => *q,
        Shown::Face(..) => Quantity::Area,
        Shown::Circle(..) => Quantity::Diameter,
        Shown::Angle(..) => Quantity::Angle,
        Shown::Box(..) => Quantity::SizeX,
        Shown::Point(_) => Quantity::X,
    }
}

fn is_distance(q: Quantity) -> bool {
    matches!(q, Quantity::Distance | Quantity::CentreDistance | Quantity::MaxDistance)
}

/// A label asked for: what it points at, where it would like to be, what it says.
struct Want {
    anchor: Pos2,
    want: Pos2,
    text: String,
    key: String,
    leader: Stroke,
}

/// The view's labels. They're asked for as the dimensions are drawn, then set once every line
/// is known: each where it covers no line, no other label's anchor, no label set before, and
/// its leader crosses no label; then drawn over everything.
struct Labels<'a> {
    painter: &'a egui::Painter,
    /// Every line drawn, on screen.
    segs: Vec<(Pos2, Pos2)>,
    wants: Vec<Want>,
}

impl Labels<'_> {
    fn put(&mut self, anchor: Pos2, want: Pos2, text: String, key: &str, leader: Stroke) {
        self.wants.push(Want {
            anchor,
            want,
            text,
            key: key.to_string(),
            leader,
        });
    }

    fn seg(&mut self, a: Pos2, b: Pos2) {
        self.segs.push((a, b));
    }

    fn poly(&mut self, ps: &[Pos2]) {
        self.segs.extend(ps.windows(2).map(|w| (w[0], w[1])));
    }

    fn finish(self) -> Vec<(String, Rect)> {
        let anchors: Vec<Pos2> = self.wants.iter().map(|w| w.anchor).collect();
        let mut taken: Vec<Rect> = vec![];
        let mut set = vec![];
        for (k, w) in self.wants.into_iter().enumerate() {
            let g = self.painter.layout_no_wrap(w.text, mono(12.0), INK);
            let size = g.size() + vec2(10.0, 6.0);
            let r = place_label(&w.anchor, w.want, size, k, &anchors, &taken, &self.segs, w.leader != Stroke::NONE);
            taken.push(r);
            set.push((r, g, w.anchor, w.leader, w.key));
        }
        // leaders under every box
        for (r, _, anchor, leader, _) in &set {
            if *leader != Stroke::NONE {
                self.painter.line_segment([*anchor, nearest_on_rect(*r, *anchor)], *leader);
            }
        }
        let mut places = vec![];
        for (r, g, _, _, key) in set {
            self.painter.rect_filled(r, 3.0, GROUND);
            self.painter
                .rect_stroke(r, 3.0, Stroke::new(1.0, INK.gamma_multiply(0.6)), egui::StrokeKind::Inside);
            self.painter.galley(r.min + vec2(5.0, 3.0), g, INK);
            places.push((format!("measure:mark:{key}"), r));
        }
        places
    }
}

/// Where a label goes: the place it asked for if that's clear, else the first clear one round
/// its anchor, farther out ring by ring; if none is, below the labels in the way.
#[allow(clippy::too_many_arguments)]
fn place_label(
    anchor: &Pos2,
    want: Pos2,
    size: egui::Vec2,
    me: usize,
    anchors: &[Pos2],
    taken: &[Rect],
    segs: &[(Pos2, Pos2)],
    leader: bool,
) -> Rect {
    let anchor = *anchor;
    let clear = |r: Rect| {
        let near = r.expand(3.0);
        !taken.iter().any(|o| o.expand(2.0).intersects(r))
            && !segs.iter().any(|(a, b)| seg_hits_rect(*a, *b, near))
            && !anchors.iter().enumerate().any(|(k, a)| k != me && r.expand(6.0).contains(*a))
            && !(leader && {
                let end = nearest_on_rect(r, anchor);
                taken.iter().any(|o| seg_hits_rect(anchor, end, o.shrink(1.0)))
            })
    };
    let first = Rect::from_center_size(want, size);
    if clear(first) {
        return first;
    }
    let out = want - anchor;
    let reach = out.length().max(24.0);
    let start = out.angle();
    for ring in [1.0f32, 1.5, 2.2, 3.0] {
        for k in 1..24 {
            // alternately either side of the way it asked for, 15° apart
            let turn = (k / 2) as f32 * if k % 2 == 0 { -1.0 } else { 1.0 } * 15f32.to_radians();
            let dir = egui::Vec2::angled(start + turn);
            // far enough that the box's nearer edge, not its centre, sits at the reach
            let c = anchor + dir * (reach * ring + 0.5 * (size.x * dir.x.abs() + size.y * dir.y.abs()) * 0.6);
            let r = Rect::from_center_size(c, size);
            if clear(r) {
                return r;
            }
        }
    }
    let mut r = first;
    for _ in 0..12 {
        match taken.iter().find(|o| o.expand(2.0).intersects(r)) {
            Some(o) => r = r.translate(vec2(0.0, o.bottom() - r.top() + 4.0)),
            None => break,
        }
    }
    r
}

fn nearest_on_rect(r: Rect, p: Pos2) -> Pos2 {
    pos2(p.x.clamp(r.left(), r.right()), p.y.clamp(r.top(), r.bottom()))
}

/// Whether a segment on screen crosses or lies in a rectangle.
fn seg_hits_rect(a: Pos2, b: Pos2, r: Rect) -> bool {
    if r.contains(a) || r.contains(b) {
        return true;
    }
    // clipped to the rectangle's slab in x, then in y (Liang-Barsky)
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    let d = b - a;
    for (p, q) in [
        (-d.x, a.x - r.left()),
        (d.x, r.right() - a.x),
        (-d.y, a.y - r.top()),
        (d.y, r.bottom() - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
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
        Free::Loop { loops, .. } => {
            for l in loops {
                let mut ring = l.clone();
                ring.extend(l.first().copied());
                poly(ring);
            }
        }
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
