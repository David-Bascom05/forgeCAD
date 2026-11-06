//! What the drafting commands draw over a DWG's canvas, and their windows: the rubber band
//! (`Ghost`s), the selection's grips, AutoSnap's marker and tooltip, POLAR's and ORTHO's
//! tracking line, the grid, dynamic input by the cursor; the Drafting Settings window (running
//! object snaps, polar angle, snap and grid), MTEXT's editor and the ribbon's layer dropdown.
//! All drawn with the egui painter, in screen space: nothing here touches the scene.

use super::Shell;
use super::dwgcmd::{Ghost, Inp, Track, grid_step, want};
use super::dwgdoc::View;
use super::dwgdoc_ui::{to_screen, to_world};
use super::theme::{mono, sans};
use egui::{Align2, Color32, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};
use forge_dwg::draft::snap::SnapKind;
use forge_dwg::draft::{Curve, V2, v2};

/// AutoSnap's marker colour, and the tracking line's.
const MARKER: Color32 = Color32::from_rgb(0xF2, 0xC8, 0x1E);
const TRACK: Color32 = Color32::from_rgb(0x6F, 0xC8, 0x6F);
const GRIP: Color32 = Color32::from_rgb(0x2F, 0x6B, 0xFF);
const GRIP_HOT: Color32 = Color32::from_rgb(0xFF, 0x30, 0x30);
const GRIP_OVER: Color32 = Color32::from_rgb(0x30, 0xE0, 0x60);
/// The most segments a rubber band draws (a big selection dragged draws its box past this).
const MAX_GHOST_SEGS: usize = 20_000;

/// AutoCAD's names for the snaps, as AutoSnap's tooltip says them.
pub fn snap_name(k: SnapKind) -> &'static str {
    match k {
        SnapKind::Endpoint => "Endpoint",
        SnapKind::Intersection => "Intersection",
        SnapKind::Midpoint => "Midpoint",
        SnapKind::Center => "Center",
        SnapKind::Quadrant => "Quadrant",
        SnapKind::Node => "Node",
        SnapKind::Insertion => "Insertion",
        SnapKind::Perpendicular => "Perpendicular",
        SnapKind::Tangent => "Tangent",
        SnapKind::ApparentIntersection => "Apparent Intersection",
        SnapKind::Extension => "Extension",
        SnapKind::Parallel => "Parallel",
        SnapKind::Nearest => "Nearest",
    }
}

/// The marker AutoSnap draws for a kind of snap, at a screen point.
pub fn snap_marker(p: &egui::Painter, k: SnapKind, at: Pos2) {
    let s = 6.0;
    let st = Stroke::new(2.0, MARKER);
    let sq = |r: f32| Rect::from_center_size(at, vec2(r * 2.0, r * 2.0));
    match k {
        SnapKind::Endpoint => {
            p.rect_stroke(sq(s), 0.0, st, StrokeKind::Middle);
        }
        SnapKind::Midpoint => {
            let pts = vec![at + vec2(0.0, -s), at + vec2(s, s * 0.8), at + vec2(-s, s * 0.8)];
            p.add(Shape::closed_line(pts, st));
        }
        SnapKind::Center => {
            p.circle_stroke(at, s, st);
        }
        SnapKind::Quadrant => {
            let pts = vec![at + vec2(0.0, -s), at + vec2(s, 0.0), at + vec2(0.0, s), at + vec2(-s, 0.0)];
            p.add(Shape::closed_line(pts, st));
        }
        SnapKind::Intersection | SnapKind::ApparentIntersection => {
            p.line_segment([at + vec2(-s, -s), at + vec2(s, s)], st);
            p.line_segment([at + vec2(-s, s), at + vec2(s, -s)], st);
            if k == SnapKind::ApparentIntersection {
                p.rect_stroke(sq(s), 0.0, st, StrokeKind::Middle);
            }
        }
        SnapKind::Node => {
            p.circle_stroke(at, s, st);
            p.line_segment([at + vec2(-s * 0.7, -s * 0.7), at + vec2(s * 0.7, s * 0.7)], st);
            p.line_segment([at + vec2(-s * 0.7, s * 0.7), at + vec2(s * 0.7, -s * 0.7)], st);
        }
        SnapKind::Insertion => {
            p.rect_stroke(Rect::from_min_size(at + vec2(-s, -s), vec2(s, s)), 0.0, st, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_size(at, vec2(s, s)), 0.0, st, StrokeKind::Middle);
        }
        SnapKind::Perpendicular => {
            p.line_segment([at + vec2(-s, s), at + vec2(s, s)], st);
            p.line_segment([at + vec2(-s, s), at + vec2(-s, -s)], st);
            p.line_segment([at + vec2(-s, 0.0), at + vec2(0.0, 0.0)], st);
            p.line_segment([at + vec2(0.0, 0.0), at + vec2(0.0, s)], st);
        }
        SnapKind::Tangent => {
            p.circle_stroke(at, s * 0.8, st);
            p.line_segment([at + vec2(-s, -s * 0.8), at + vec2(s, -s * 0.8)], st);
        }
        SnapKind::Nearest => {
            let pts = vec![
                at + vec2(-s, -s),
                at + vec2(s, -s),
                at + vec2(-s, s),
                at + vec2(s, s),
                at + vec2(-s, -s),
            ];
            p.add(Shape::line(pts, st));
        }
        SnapKind::Extension => {
            p.line_segment([at + vec2(-s, 0.0), at + vec2(s, 0.0)], st);
            p.line_segment([at + vec2(0.0, -s), at + vec2(0.0, s)], st);
        }
        SnapKind::Parallel => {
            p.line_segment([at + vec2(-s, s), at + vec2(-s * 0.2, -s)], st);
            p.line_segment([at + vec2(s * 0.2, s), at + vec2(s, -s)], st);
        }
    }
}

/// A tooltip box by the cursor, as AutoSnap's and dynamic input's.
fn tip(p: &egui::Painter, at: Pos2, text: &str, fg: Color32, bg: Color32) -> Rect {
    let g = p.layout_no_wrap(text.to_string(), sans(12.0), fg);
    let r = Rect::from_min_size(at, g.size() + vec2(10.0, 6.0));
    p.rect_filled(r, 3.0, bg);
    p.rect_stroke(r, 3.0, Stroke::new(1.0, fg.gamma_multiply(0.5)), StrokeKind::Inside);
    p.galley(r.min + vec2(5.0, 3.0), g, fg);
    r
}

/// A curve's points for drawing at a zoom (points a unit).
fn curve_pts(c: &Curve, scale: f64) -> Vec<V2> {
    let (t0, t1) = c.domain();
    if !t0.is_finite() || !t1.is_finite() {
        return vec![];
    }
    let n = match c {
        Curve::Line { .. } => 1,
        Curve::Poly(p) => {
            let mut out = vec![];
            for k in 0..p.nseg() {
                let sg = p.seg(k);
                let m = if sg.bulge() == 0.0 { 1 } else { 24 };
                for i in 0..m {
                    out.push(sg.point(i as f64 / m as f64));
                }
            }
            if let Some(last) = p.nseg().checked_sub(1) {
                out.push(p.seg(last).point(1.0));
            }
            return out;
        }
        _ => {
            let len_px = c.bbox().size() * scale;
            (len_px / 4.0).clamp(24.0, 512.0) as usize
        }
    };
    (0..=n).map(|k| c.point(t0 + (t1 - t0) * k as f64 / n as f64)).collect()
}

impl Shell {
    /// The drafting overlay on the canvas.
    pub(super) fn dwg_overlay(&mut self, p: &egui::Painter, canvas: Rect, v: &View, paper: bool, hover: Option<Pos2>) {
        let Some(d) = &self.dwgdoc else { return };
        let ink = if paper { Color32::from_gray(30) } else { Color32::from_gray(235) };
        let scr = |w: V2| to_screen(v, canvas, w.arr());
        // the grid
        if d.ed.set.grid {
            let mut step = grid_step(v.scale);
            let lo = to_world(v, canvas, canvas.left_bottom());
            let hi = to_world(v, canvas, canvas.right_top());
            while ((hi[0] - lo[0]) / step) * ((hi[1] - lo[1]) / step) > 40_000.0 {
                step *= 2.0;
            }
            let dot = if paper { Color32::from_gray(160) } else { Color32::from_gray(80) };
            let (x0, y0) = ((lo[0] / step).floor() as i64, (lo[1] / step).floor() as i64);
            let (x1, y1) = ((hi[0] / step).ceil() as i64, (hi[1] / step).ceil() as i64);
            for i in x0..=x1 {
                for j in y0..=y1 {
                    let q = scr(v2(i as f64 * step, j as f64 * step));
                    p.rect_filled(Rect::from_center_size(q, vec2(1.5, 1.5)), 0.0, dot);
                }
            }
        }
        // the rubber band
        let band = Stroke::new(1.0, ink);
        let q = d.viewq();
        for g in d.ghosts() {
            match g {
                Ghost::Curve(c) => {
                    let pts: Vec<Pos2> = curve_pts(&c, v.scale).into_iter().map(scr).collect();
                    if pts.len() >= 2 {
                        p.add(Shape::line(pts, band));
                    }
                }
                Ghost::Dashed(a, b) => {
                    p.extend(Shape::dashed_line(&[scr(a), scr(b)], Stroke::new(1.0, ink.gamma_multiply(0.7)), 6.0, 4.0));
                }
                Ghost::Text(at, h, s) => {
                    let size = (h * v.scale).clamp(6.0, 200.0) as f32;
                    p.text(scr(at), Align2::LEFT_BOTTOM, s, sans(size), ink);
                }
                Ghost::Ents(hs, m) => {
                    let mut segs = vec![];
                    for h in &hs {
                        q.segments(*h, MAX_GHOST_SEGS + 1, &mut segs);
                        if segs.len() > MAX_GHOST_SEGS {
                            break;
                        }
                    }
                    if segs.len() > MAX_GHOST_SEGS {
                        // too many to drag: their box
                        let mut b = forge_dwg::draft::Bbox::EMPTY;
                        for h in &hs {
                            if let Some((lo, hi)) = q.geom_of(*h).entity_box(*h) {
                                b.add(V2::from(lo));
                                b.add(V2::from(hi));
                            }
                        }
                        if !b.is_empty() {
                            let c = [b.min, v2(b.max.x, b.min.y), b.max, v2(b.min.x, b.max.y)];
                            p.add(Shape::closed_line(c.iter().map(|w| scr(m.apply(*w))).collect(), band));
                        }
                    } else {
                        for (a, b) in segs {
                            let (a, b) = (m.apply(V2::from(a)), m.apply(V2::from(b)));
                            p.line_segment([scr(a), scr(b)], band);
                        }
                    }
                }
            }
        }
        // grips
        let hover_w = hover.map(|h| V2::from(to_world(v, canvas, h)));
        for (k, (_, g)) in d.ed.grips.iter().enumerate() {
            let at = scr(g.at);
            if !canvas.contains(at) {
                continue;
            }
            let over = hover_w.is_some_and(|w| (scr(w) - at).length() <= 5.0);
            let col = if d.ed.grip_hot == Some(k) {
                GRIP_HOT
            } else if over {
                GRIP_OVER
            } else {
                GRIP
            };
            let r = Rect::from_center_size(at, vec2(8.0, 8.0));
            p.rect_filled(r, 0.0, col);
            p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Outside);
        }
        // AutoSnap's marker and tooltip, or the tracking line
        let tip_bg = Color32::from_rgba_unmultiplied(30, 34, 40, 235);
        match &d.ed.track {
            Some(Track::Snap(s)) => {
                let at = scr(s.point);
                snap_marker(p, s.kind, at);
                if s.kind == SnapKind::Extension
                    && let Some(&h) = s.handles.first()
                    && let Some(c) = d.drawing.objects.get(&h).and_then(|_| forge_dwg::draft::entity::curve_of(&d.drawing, h).ok())
                {
                    let e = if c.curve.start().dist(s.point) < c.curve.end().dist(s.point) {
                        c.curve.start()
                    } else {
                        c.curve.end()
                    };
                    p.extend(Shape::dotted_line(&[scr(e), at], TRACK, 6.0, 1.0));
                }
                tip(p, at + vec2(14.0, 12.0), snap_name(s.kind), Color32::WHITE, tip_bg);
            }
            Some(Track::Polar { base, angle }) => {
                if let Some(c) = d.ed.cursor {
                    let far = *base + V2::polar(*angle, (canvas.width() + canvas.height()) as f64 * 2.0 / v.scale);
                    p.extend(Shape::dotted_line(&[scr(*base), scr(far)], TRACK, 6.0, 1.0));
                    let what = if d.ed.set.ortho { "Ortho" } else { "Polar" };
                    let a = angle.to_degrees().rem_euclid(360.0);
                    let txt = format!("{what}: {:.4} < {a:.0}\u{b0}", base.dist(c));
                    tip(p, scr(c) + vec2(14.0, 12.0), &txt, Color32::WHITE, tip_bg);
                }
            }
            _ => {}
        }
        // dynamic input: the prompt and what's typed by the cursor, and the rubber band's length
        // and angle
        if d.ed.set.dyn_input
            && let (Some(pr), Some(hp)) = (&d.ed.prompt, hover)
        {
            let line = pr.text.clone();
            let typed = if d.input.is_empty() { String::new() } else { format!("  {}", d.input) };
            let r = tip(p, hp + vec2(18.0, 26.0), &format!("{line}{typed}"), Color32::WHITE, tip_bg);
            if let (Some(b), Some(c)) = (pr.base, d.ed.cursor)
                && pr.wants(want::POINT)
                && b.dist(c) > 0.0
            {
                let a = (c - b).angle().to_degrees().rem_euclid(360.0);
                tip(p, r.left_bottom() + vec2(0.0, 4.0), &format!("{:.4}   {a:.0}\u{b0}", b.dist(c)), Color32::from_gray(220), tip_bg);
            }
        }
    }

    /// The Drafting Settings window: running object snaps, polar tracking's angle, SNAP's and
    /// GRID's switches.
    pub(super) fn dwg_osnap_window(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.dwgdoc else { return };
        if !d.ed.show_osnap {
            return;
        }
        let area = super::dialog_area(ctx);
        let mut open = true;
        let mut place = vec![];
        egui::Window::new("Drafting Settings")
            .id(egui::Id::unique("dwg-osnap-window"))
            .frame(super::fgasm::dialog_frame(&self.theme))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(area.center().x - 200.0, area.top() + 80.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                let s = &mut d.ed.set;
                ui.checkbox(&mut s.osnap, "Object Snap On (F3)");
                ui.add_space(4.0);
                egui::Grid::new("osnap-kinds").num_columns(2).show(ui, |ui| {
                    for (i, k) in SnapKind::ALL.iter().enumerate() {
                        let mut on = s.osmode & k.bit() != 0;
                        let r = ui.checkbox(&mut on, snap_name(*k));
                        place.push((format!("dwg:osnap:{}", snap_name(*k)), r.rect));
                        if r.changed() {
                            s.osmode ^= k.bit();
                        }
                        if i % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        s.osmode = SnapKind::ALL.iter().map(|k| k.bit()).sum();
                    }
                    if ui.button("Clear All").clicked() {
                        s.osmode = 0;
                    }
                });
                ui.separator();
                ui.checkbox(&mut s.polar, "Polar Tracking On (F10)");
                ui.horizontal(|ui| {
                    ui.label("Increment angle");
                    let cur = s.polar_inc.to_degrees();
                    egui::ComboBox::from_id_salt("polar-inc")
                        .selected_text(format!("{cur}"))
                        .show_ui(ui, |ui| {
                            for a in [90.0, 45.0, 30.0, 22.5, 18.0, 15.0, 10.0, 5.0] {
                                if ui.selectable_label((cur - a).abs() < 1e-9, format!("{a}")).clicked() {
                                    s.polar_inc = f64::to_radians(a);
                                }
                            }
                        });
                });
                ui.checkbox(&mut s.ortho, "Ortho (F8)");
                ui.separator();
                ui.checkbox(&mut s.snap, "Snap On (F9)");
                ui.horizontal(|ui| {
                    ui.label("Snap spacing (0: from the zoom)");
                    ui.add(egui::DragValue::new(&mut s.snap_unit).range(0.0..=1e9).speed(0.1));
                });
                ui.checkbox(&mut s.grid, "Grid On (F7)");
                ui.checkbox(&mut s.dyn_input, "Dynamic Input (F12)");
            });
        self.places.extend(place);
        if !open && let Some(d) = &mut self.dwgdoc {
            d.ed.show_osnap = false;
        }
    }

    /// MTEXT's editor: a box to type the text in, OK to place it.
    pub(super) fn dwg_mtext_window(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.dwgdoc else { return };
        let on = d.ed.prompt.as_ref().is_some_and(|p| p.wants(want::LINES));
        if !on {
            return;
        }
        let area = super::dialog_area(ctx);
        let mut ok = false;
        let mut cancel = false;
        let mut place = vec![];
        egui::Window::new("Text Editor")
            .id(egui::Id::unique("dwg-mtext-window"))
            .frame(super::fgasm::dialog_frame(&self.theme))
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(420.0, 200.0))
            .default_pos(pos2(area.center().x - 210.0, area.top() + 120.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                let r = ui.add(
                    egui::TextEdit::multiline(&mut d.mtext)
                        .font(mono(13.0))
                        .desired_rows(6)
                        .desired_width(f32::INFINITY),
                );
                place.push(("dwg:mtext:edit".to_string(), r.rect));
                if !r.has_focus() && d.mtext.is_empty() {
                    r.request_focus();
                }
                ui.horizontal(|ui| {
                    let b = ui.button("OK");
                    place.push(("dwg:mtext:ok".to_string(), b.rect));
                    ok = b.clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
                    cancel = ui.button("Cancel").clicked();
                });
            });
        self.places.extend(place);
        if let Some(d) = &mut self.dwgdoc {
            if ok {
                let text = std::mem::take(&mut d.mtext);
                d.feed(Inp::Text(text));
            } else if cancel {
                d.mtext.clear();
                d.cancel_command();
            }
        }
    }

    /// The ribbon's layer dropdown: the current layer; a pick moves what's chosen to it, or
    /// makes it current.
    pub(super) fn dwg_layer_dropdown(&mut self, ui: &mut egui::Ui) {
        let Some(slot) = self.places.iter().rev().find(|(k, _)| k == "dwg:ribbon:layer").map(|x| x.1) else {
            return;
        };
        let t = self.theme;
        let Some(d) = &mut self.dwgdoc else { return };
        let rows = d.layer_rows();
        let cur = rows.iter().find(|r| r.current).map(|r| r.name.clone()).unwrap_or_else(|| "0".into());
        let shown = if d.selected.is_empty() {
            cur
        } else {
            let names: Vec<String> = d
                .selected
                .iter()
                .filter_map(|h| d.drawing.objects.get(h).and_then(|o| o.ent.as_ref()))
                .filter_map(|e| super::dwgdoc::entry_name(&d.drawing, e.layer))
                .collect();
            if names.windows(2).all(|w| w[0] == w[1]) {
                names.first().cloned().unwrap_or(cur)
            } else {
                "*VARIES*".into()
            }
        };
        let mut picked = None;
        let mut place = vec![];
        ui.scope_builder(egui::UiBuilder::new().max_rect(slot), |ui| {
            let r = egui::ComboBox::from_id_salt("dwg-ribbon-layer")
                .width(slot.width() - 8.0)
                .selected_text(egui::RichText::new(&shown).font(sans(12.0)).color(t.ink))
                .show_ui(ui, |ui| {
                    for row in &rows {
                        let resp = ui.horizontal(|ui| {
                            let (sw, _) = ui.allocate_exact_size(vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(sw, 2.0, Color32::from_rgb(row.rgb[0], row.rgb[1], row.rgb[2]));
                            ui.selectable_label(row.name == shown, &row.name)
                        });
                        place.push((format!("dwg:ribbon:layer:{}", row.name), resp.inner.rect));
                        if resp.inner.clicked() {
                            picked = Some(row.handle);
                        }
                    }
                });
            place.push(("dwg:ribbon:layer-combo".to_string(), r.response.rect));
        });
        self.places.extend(place);
        if let (Some(h), Some(d)) = (picked, &mut self.dwgdoc)
            && let Err(e) = d.layer_picked(h)
        {
            d.say(e);
        }
    }
}
