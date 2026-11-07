//! How the harness's dialogs look: a wire's cross-section drawn to scale (strands in the
//! conductor, the insulation in its colour with its tracers, OD and wall called out), the header
//! a wire or type gets, sections of aligned fields with muted units, computed chips, the cut
//! length as a stacked bar, the IEC colour grid, and the wire library's table.
use super::fgasm::{dialog_frame, dialog_head};
use super::harness_ui::Dialog;
use super::materials::{button_at, clipped, pill, section};
use super::theme::{Theme, medium, mono, sans, semi};
use crate::shell::placed::Placed;
use egui::{Align2, Color32, Painter, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};
use forge_lang::harness::{HarnessOut, LibRow, SpecVal, TypeOut};
use std::collections::BTreeMap;

/// A colour code's colours as egui's.
pub fn c32(c: [f32; 3]) -> Color32 {
    Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8)
}

fn mix(a: Color32, b: Color32, f: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// What a wire's cross-section is drawn from.
#[derive(Clone, Debug, Default)]
pub struct Section {
    pub od: f64,
    pub conductor_d: Option<f64>,
    pub wall: Option<f64>,
    pub strands: Option<f64>,
    pub strand_d: Option<f64>,
    pub conductor: String,
    pub colour: String,
    pub name: String,
}

impl Section {
    /// From a type's fields by key.
    pub fn of(get: &dyn Fn(&str) -> String, colour: &str, name: &str) -> Section {
        let num = |k: &str| get(k).trim().parse::<f64>().ok().filter(|v| *v > 0.0);
        let conductor_d = num("conductor_d")
            .or_else(|| num("awg").map(forge_harness::awg_d))
            .or_else(|| num("mm2").map(|a| (4.0 * a / std::f64::consts::PI).sqrt()));
        Section {
            od: num("od").or(conductor_d).unwrap_or(1.0),
            conductor_d,
            wall: num("wall"),
            strands: num("strands"),
            strand_d: num("strand_d"),
            conductor: get("conductor"),
            colour: if colour.is_empty() { get("colour") } else { colour.to_string() },
            name: name.to_string(),
        }
    }

    /// The conductor's diameter: as given, else the OD less the wall, else most of the OD.
    fn core(&self) -> f64 {
        self.conductor_d
            .or(self.wall.map(|w| self.od - 2.0 * w))
            .unwrap_or(self.od * 0.6)
            .clamp(self.od * 0.05, self.od)
    }
}

/// The strands' centres in a conductor of unit radius, ring by ring (1, 6, 12, ..), and their
/// radius: as many as given, at their size if it is given.
fn strand_layout(n: usize, strand_d: Option<f64>, core: f64) -> (Vec<(f32, f32)>, f32) {
    let n = n.max(1);
    // rings needed for n: 1 + 6 + 12 + .. (concentric lay)
    let mut rings = 0usize;
    while 1 + 3 * rings * (rings + 1) < n {
        rings += 1;
    }
    let r = match strand_d {
        Some(d) => (d / core) as f32,
        None => 1.0 / (2 * rings + 1) as f32,
    };
    if n == 1 {
        return (vec![(0.0, 0.0)], 1.0);
    }
    let mut out = vec![(0.0, 0.0)];
    'r: for ring in 1..=rings {
        let count = 6 * ring;
        let rad = 2.0 * r * ring as f32;
        for j in 0..count {
            if out.len() >= n {
                break 'r;
            }
            let a = std::f32::consts::TAU * j as f32 / count as f32 + ring as f32 * 0.3;
            out.push((rad * a.cos(), rad * a.sin()));
        }
    }
    (out, r)
}

/// A wire's cross-section in `r`: insulation in its colour with its tracers as bands round it,
/// the conductor's strands at their scale, and the OD and wall called out.
pub fn cross_section(p: &Painter, t: &Theme, r: Rect, s: &Section, callouts: bool) {
    let codes = forge_lang::harness::colour_codes(&s.colour);
    let base = c32(forge_lang::harness::rgb(&s.colour, &s.name));
    // with call-outs: the OD under it and the wall at its right, all inside `r`
    let (c, ro) = if callouts {
        let ro = (r.height() * 0.62 / 2.0).min((r.width() - 84.0) / 2.0);
        (pos2(r.left() + 6.0 + ro, r.center().y - r.height() * 0.06), ro)
    } else {
        (r.center(), r.width().min(r.height()) * 0.92 / 2.0)
    };
    let scale = ro / (s.od as f32 / 2.0);
    let rc = (s.core() as f32 / 2.0 * scale).min(ro * 0.95);
    // insulation, lit a little from the upper left
    p.circle_filled(c, ro, mix(base, Color32::BLACK, 0.18));
    p.circle_filled(c - vec2(ro * 0.04, ro * 0.04), ro * 0.97, base);
    // tracers: bands round the insulation, opposite each other
    for (k, code) in codes.iter().skip(1).take(2).enumerate() {
        let col = c32(forge_lang::harness::rgb(code, &s.name));
        let mid = -std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::PI;
        let half = 0.42;
        let n = 16;
        let mut pts = vec![];
        for j in 0..=n {
            let a = mid - half + 2.0 * half * j as f32 / n as f32;
            pts.push(c + vec2(a.cos(), a.sin()) * ro);
        }
        for j in (0..=n).rev() {
            let a = mid - half + 2.0 * half * j as f32 / n as f32;
            pts.push(c + vec2(a.cos(), a.sin()) * (rc + (ro - rc) * 0.35));
        }
        p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
    }
    // the conductor and its strands
    let metal = match s.conductor.to_ascii_lowercase().as_str() {
        x if x.contains("tin") || x.contains("al") || x.contains("silver") => Color32::from_rgb(0xC9, 0xCC, 0xD1),
        _ => Color32::from_rgb(0xC9, 0x7A, 0x3E),
    };
    p.circle_filled(c, rc, mix(metal, Color32::BLACK, 0.45));
    let n = s.strands.map_or(1, |x| x.round().max(1.0) as usize).min(400);
    let (centres, sr) = strand_layout(n, s.strand_d, s.core());
    for (x, y) in centres {
        let at = c + vec2(x, y) * rc;
        let rr = (sr * rc).max(0.8);
        p.circle_filled(at, rr, metal);
        p.circle_filled(at - vec2(rr * 0.25, rr * 0.25), rr * 0.45, mix(metal, Color32::WHITE, 0.35));
    }
    p.circle_stroke(c, ro, Stroke::new(1.0, mix(base, Color32::BLACK, 0.45)));
    if !callouts {
        return;
    }
    // OD across under it, the wall at the right
    let ink = t.ink2;
    let y = c.y + ro + 14.0;
    let (a, b) = (pos2(c.x - ro, y), pos2(c.x + ro, y));
    let st = Stroke::new(1.0, ink);
    p.line_segment([a, b], st);
    for (q, s_) in [(a, 1.0f32), (b, -1.0)] {
        p.line_segment([q, q + vec2(5.0 * s_, -3.0)], st);
        p.line_segment([q, q + vec2(5.0 * s_, 3.0)], st);
        p.line_segment([q - vec2(0.0, 6.0), pos2(q.x, c.y + ro * 0.2)], Stroke::new(0.5, ink.gamma_multiply(0.6)));
    }
    p.text(
        pos2(c.x, y + 10.0),
        Align2::CENTER_CENTER,
        format!("Ø{}", super::fg::short(s.od)),
        mono(11.0),
        t.ink,
    );
    let wall = (s.od - s.core()) / 2.0;
    let wy = c.y - ro * 0.55;
    let wx_in = c.x + (rc * rc - (wy - c.y).powi(2)).max(0.0).sqrt();
    let wx_out = c.x + (ro * ro - (wy - c.y).powi(2)).max(0.0).sqrt();
    p.line_segment([pos2(wx_in, wy), pos2(wx_out + 14.0, wy)], Stroke::new(0.8, ink));
    p.text(
        pos2(wx_out + 17.0, wy),
        Align2::LEFT_CENTER,
        format!("wall {}", super::fg::short((wall * 100.0).round() / 100.0)),
        mono(10.5),
        ink,
    );
}

/// A dialog's header for a wire or a type: its cross-section, its name, a line of what it is and
/// its pills (spec tag, part numbers).
pub fn header(ui: &mut Ui, t: &Theme, s: &Section, title: &str, sub: &str, pills: &[String]) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 150.0), Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, t.raised);
    p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, t.line));
    let pic = Rect::from_min_size(r.min + vec2(14.0, 6.0), vec2(176.0, 138.0));
    cross_section(&p, t, pic, s, true);
    let x = pic.right() + 12.0;
    p.text(pos2(x, r.top() + 34.0), Align2::LEFT_CENTER, title, semi(18.0), t.ink);
    // the spec line wraps (two lines at most), the pills go under it
    let mut job = egui::text::LayoutJob::simple(sub.to_string(), sans(12.5), t.ink2, r.right() - 14.0 - x);
    job.wrap.max_rows = 2;
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let sub_h = g.size().y;
    p.galley(pos2(x, r.top() + 52.0), g, t.ink2);
    let mut px = x;
    let mut py = r.top() + 52.0 + sub_h + 20.0;
    for pl in pills.iter().filter(|x| !x.is_empty()) {
        let g = p.layout_no_wrap(pl.clone(), medium(10.5), t.on_ink);
        if px + g.size().x + 20.0 > r.right() - 10.0 {
            px = x;
            py += 24.0;
        }
        let rr = pill(&p, pos2(px, py), false, pl, t.on_ink, t.on_ink.gamma_multiply(0.12));
        px = rr.right() + 6.0;
    }
}

/// A label and its value's place, the label muted at the left; the value's rect.
pub fn field(ui: &mut Ui, t: &Theme, label: &str, h: f32) -> Rect {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter()
        .text(pos2(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, label, sans(12.5), t.ink2);
    Rect::from_min_max(pos2(r.left() + 168.0, r.top() + 2.0), pos2(r.right() - 16.0, r.bottom() - 2.0))
}

/// A read-only value with its unit muted after it, and where it came from as a chip.
pub fn value_row(ui: &mut Ui, t: &Theme, label: &str, value: &str, unit: &str, chip: Option<&str>) {
    let r = field(ui, t, label, 26.0);
    let p = ui.painter().clone();
    // numbers and paths (the arrow is in the mono face) in mono, words in the text face
    let font = if value.parse::<f64>().is_ok() || value.contains('›') { mono(12.0) } else { sans(12.5) };
    let g = p.layout_no_wrap(value.to_string(), font, t.ink);
    let gw = g.size().x;
    p.galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, t.ink);
    let mut x = r.left() + gw + 5.0;
    if !unit.is_empty() {
        let u = p.layout_no_wrap(unit.to_string(), sans(11.5), t.ink2);
        let uw = u.size().x;
        p.galley(pos2(x, r.center().y - u.size().y / 2.0), u, t.ink2);
        x += uw + 8.0;
    }
    if let Some(c) = chip {
        pill(&p, pos2(x, r.center().y), false, c, t.ink2, t.hover);
    }
}

/// A text field with its unit muted at its right end; its rect.
pub fn edit_row(ui: &mut Ui, t: &Theme, label: &str, v: &mut String, unit: &str, hint: &str, key: &str) -> Rect {
    let r = field(ui, t, label, 30.0);
    let box_r = Rect::from_min_max(r.min, pos2(r.right() - if unit.is_empty() { 0.0 } else { 56.0 }, r.max.y));
    ui.put(
        box_r,
        egui::TextEdit::singleline(v)
            .font(sans(13.0))
            .hint_text(hint)
            .margin(vec2(8.0, 5.0))
            .id_salt(("harness-field", key)),
    );
    if !unit.is_empty() {
        ui.painter()
            .text(pos2(r.right() - 50.0, r.center().y), Align2::LEFT_CENTER, unit, sans(11.5), t.ink2);
    }
    box_r
}

/// A choice among words or free text: a dropdown of the usual ones beside the field.
pub fn choice_row(ui: &mut Ui, t: &Theme, label: &str, v: &mut String, words: &[&str], key: &str) {
    let r = field(ui, t, label, 30.0);
    let edit_r = Rect::from_min_max(r.min, pos2(r.right() - 34.0, r.max.y));
    ui.put(
        edit_r,
        egui::TextEdit::singleline(v)
            .font(sans(13.0))
            .margin(vec2(8.0, 5.0))
            .id_salt(("harness-choice", key)),
    );
    let btn = Rect::from_min_max(pos2(r.right() - 30.0, r.top()), r.max);
    let resp = ui.put(btn, egui::Button::new(""));
    let c = btn.center();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![c + vec2(-4.5, -2.0), c + vec2(4.5, -2.0), c + vec2(0.0, 3.0)],
        t.ink2,
        Stroke::NONE,
    ));
    let pop = ui.make_persistent_id(("harness-choice-pop", key));
    egui::Popup::menu(&resp).id(pop).show(|ui| {
        for w in words {
            if ui.selectable_label(v == w, *w).clicked() {
                *v = w.to_string();
            }
        }
    });
}

/// A chip of a value worked out, not given.
pub fn computed(ui: &mut Ui, t: &Theme, label: &str, value: &str, why: &str) {
    value_row(ui, t, label, value, "", Some(why));
}

/// The cut length as a stacked bar: route, service loops, slack, insertion, each its share,
/// then the cut length rounded up, with a legend.
pub fn length_bar(ui: &mut Ui, t: &Theme, parts: &[(&str, f64)], total: f64) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 58.0), Sense::hover());
    let p = ui.painter().clone();
    let bar = Rect::from_min_max(pos2(r.left() + 16.0, r.top() + 6.0), pos2(r.right() - 16.0, r.top() + 22.0));
    p.rect_filled(bar, 4.0, t.hover);
    let sum: f64 = parts.iter().map(|x| x.1).sum::<f64>().max(total).max(1e-9);
    let cols = [t.accent, Color32::from_rgb(0x4C, 0x8B, 0xD8), Color32::from_rgb(0xE0, 0xA0, 0x30), Color32::from_rgb(0xA0, 0x6C, 0xD5)];
    let mut x = bar.left();
    for (k, (_, v)) in parts.iter().enumerate() {
        let wv = (*v / sum) as f32 * bar.width();
        if wv > 0.5 {
            let seg = Rect::from_min_max(pos2(x, bar.top()), pos2(x + wv, bar.bottom()));
            p.rect_filled(seg, 2.0, cols[k % cols.len()]);
        }
        x += wv;
    }
    p.rect_stroke(bar, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    let mut lx = bar.left();
    for (k, (name, v)) in parts.iter().enumerate() {
        if *v <= 0.0 {
            continue;
        }
        p.rect_filled(Rect::from_min_size(pos2(lx, r.top() + 33.0), vec2(9.0, 9.0)), 2.0, cols[k % cols.len()]);
        let text = format!("{name} {}", super::fg::short((v * 10.0).round() / 10.0));
        let g = p.layout_no_wrap(text, sans(11.0), t.ink2);
        let gw = g.size().x;
        p.galley(pos2(lx + 13.0, r.top() + 31.0), g, t.ink2);
        lx += gw + 30.0;
    }
    p.text(
        pos2(bar.right(), r.top() + 37.0),
        Align2::RIGHT_CENTER,
        format!("cut {} mm", super::fg::short(total)),
        medium(12.0),
        t.ink,
    );
}

/// IEC 60757's colours, in its order, with their words.
pub const IEC: [(&str, &str); 14] = [
    ("BK", "black"),
    ("BN", "brown"),
    ("RD", "red"),
    ("OG", "orange"),
    ("YE", "yellow"),
    ("GN", "green"),
    ("BU", "blue"),
    ("VT", "violet"),
    ("GY", "grey"),
    ("WH", "white"),
    ("PK", "pink"),
    ("GD", "gold"),
    ("TQ", "turquoise"),
    ("SR", "silver"),
];

/// A row of colour swatches, one chosen (or none when `none` is offered); true when changed.
pub fn colour_grid(ui: &mut Ui, t: &Theme, label: &str, now: &mut String, none: bool, key: &str) -> bool {
    let r = field(ui, t, label, 34.0);
    let words = Rect::from_min_max(pos2(r.left() - 152.0, r.top()), pos2(r.left() - 8.0, r.bottom()));
    ui.interact(words, ui.scope_id().with((key, "words")), Sense::hover())
        .on_hover_text("IEC 60757: the base colour first, then its tracers. WH/BU is white with a blue tracer.");
    let p = ui.painter().clone();
    let mut changed = false;
    let size = 22.0;
    let gap = 5.0;
    let mut x = r.left();
    let mut cells: Vec<(&str, &str)> = vec![];
    if none {
        cells.push(("", "none"));
    }
    cells.extend(IEC.iter().copied());
    for (code, word) in cells {
        let cell = Rect::from_min_size(pos2(x, r.center().y - size / 2.0), vec2(size, size));
        let resp = ui.interact(cell, ui.scope_id().with((key, code)), Sense::click()).on_hover_text(if code.is_empty() {
            "no tracer".to_string()
        } else {
            format!("{code} {word}")
        });
        if code.is_empty() {
            p.circle_stroke(cell.center(), size / 2.0 - 1.0, Stroke::new(1.0, t.ink2));
            p.line_segment([cell.left_bottom() + vec2(5.0, -5.0), cell.right_top() + vec2(-5.0, 5.0)], Stroke::new(1.0, t.ink2));
        } else {
            let c = c32(forge_lang::harness::rgb(code, ""));
            p.circle_filled(cell.center(), size / 2.0 - 1.0, c);
            p.circle_stroke(cell.center(), size / 2.0 - 1.0, Stroke::new(1.0, mix(c, Color32::BLACK, 0.35)));
        }
        if now.eq_ignore_ascii_case(code) {
            p.circle_stroke(cell.center(), size / 2.0 + 2.0, Stroke::new(2.0, t.focus));
        } else if resp.hovered() {
            p.circle_stroke(cell.center(), size / 2.0 + 1.5, Stroke::new(1.0, t.ink2));
        }
        if resp.clicked() {
            *now = code.to_string();
            changed = true;
        }
        x += size + gap;
        if x > r.right() - size {
            break;
        }
    }
    changed
}

/// A wire seen from the side, a short length of it, its tracers wound round it: Add Wire's
/// preview.
pub fn side_view(p: &Painter, r: Rect, colour: &str, name: &str) {
    let codes = forge_lang::harness::colour_codes(colour);
    let base = c32(forge_lang::harness::rgb(colour, name));
    let h = r.height() * 0.42;
    let body = Rect::from_center_size(r.center(), vec2(r.width() - 20.0, h));
    p.rect_filled(body, h / 2.0, base);
    // shading: lighter along the top
    p.rect_filled(
        Rect::from_min_max(body.min + vec2(h / 2.0, h * 0.12), pos2(body.right() - h / 2.0, body.top() + h * 0.3)),
        2.0,
        mix(base, Color32::WHITE, 0.25),
    );
    for (k, code) in codes.iter().skip(1).take(2).enumerate() {
        let col = c32(forge_lang::harness::rgb(code, name));
        let pitch = h * 3.0;
        let mut x = body.left() + h / 2.0 + k as f32 * pitch / 2.0;
        while x < body.right() - h {
            let a = pos2(x, body.bottom() - 1.0);
            let b = pos2(x + h * 0.9, body.top() + 1.0);
            p.line_segment([a, b], Stroke::new(h * 0.28, col));
            x += pitch;
        }
    }
    p.rect_stroke(body, h / 2.0, Stroke::new(1.0, mix(base, Color32::BLACK, 0.4)), StrokeKind::Inside);
}

/// A section heading, as the app's dialogs have them.
pub fn heading(ui: &mut Ui, t: &Theme, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 16.0);
            section(ui, t, text);
        });
    });
}

/// A field's text as the dialogs show it: a word's quotes off.
pub fn bare(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

fn get_of<'a>(fields: &'a [(String, String)]) -> impl Fn(&str) -> String + 'a {
    move |k: &str| {
        fields
            .iter()
            .find(|x| x.0 == k)
            .map(|x| bare(&x.1))
            .unwrap_or_default()
    }
}

/// What a type's fields work out to: its resistance from its cross-section (when not given and
/// the conductor's resistivity is a constant) and its conductor's area.
fn worked(fields: &[(String, String)]) -> (Option<f64>, Option<f64>) {
    let get = get_of(fields);
    let mut spec = BTreeMap::new();
    for (k, _, num) in forge_lang::harness::SPEC {
        let v = get(k);
        if v.is_empty() {
            continue;
        }
        let val = match v.parse::<f64>() {
            Ok(x) if *num => SpecVal::Num(x),
            _ if *num => continue,
            _ => SpecVal::Text(v),
        };
        spec.insert(k.to_string(), val);
    }
    let ohm = forge_lang::harness::ohm_km(&spec).filter(|x| x.1).map(|x| x.0);
    let num = |k: &str| get(k).parse::<f64>().ok().filter(|v| *v > 0.0);
    let area = num("mm2").or_else(|| {
        num("conductor_d")
            .or_else(|| num("awg").map(forge_harness::awg_d))
            .map(|d| std::f64::consts::PI * d * d / 4.0)
    });
    (ohm, area)
}

fn line_of(get: &dyn Fn(&str) -> String) -> String {
    let mut w = vec![];
    if !get("awg").is_empty() {
        w.push(format!("{}\u{a0}AWG", get("awg")));
    } else if !get("mm2").is_empty() {
        w.push(format!("{}\u{a0}mm²", get("mm2")));
    }
    for k in ["conductor", "insulation", "spec"] {
        if !get(k).is_empty() {
            w.push(get(k));
        }
    }
    if !get("od").is_empty() {
        w.push(format!("Ø{}\u{a0}mm", get("od")));
    }
    w.join("  ·  ")
}

const WIDE: f32 = 640.0;
const HEAD: f32 = 48.0;
const FOOT: f32 = 58.0;

pub const CONDUCTORS: &[&str] = &["Cu", "tinned Cu", "silver-plated Cu", "Al", "CCA"];
pub const INSULATIONS: &[&str] = &["PVC", "XLPE", "PTFE", "ETFE", "FEP", "silicone", "TPE", "PE"];
pub const SPECS: &[&str] = &[
    "UL 1007",
    "UL 1015",
    "UL 1569",
    "UL 3266",
    "SAE J1128 TXL",
    "SAE J1128 GXL",
    "ISO 6722",
    "MIL-W-22759/16",
    "MIL-W-22759/32",
];

/// Wire Type, Wire Library and Add Wire, laid out at their width: true for each of (done, closed).
#[allow(clippy::too_many_arguments)]
pub fn wide(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    title: &str,
    icon: &str,
    d: &mut Dialog,
    hs: &[HarnessOut],
    places: &mut Vec<(String, Rect)>,
) -> (bool, bool, Option<Rect>) {
    let area = super::dialog_area(ctx);
    let want: f32 = match d {
        Dialog::WireType { .. } => 820.0,
        Dialog::Library { rows, .. } => HEAD + FOOT + 84.0 + 30.0 + 132.0 + 32.0 * rows.len().clamp(5, 12) as f32,
        _ => 676.0,
    };
    let h = want.min(area.height() - 16.0).max(420.0);
    let (mut ok, mut close) = (false, false);
    let r = egui::Window::new(title)
        .id(egui::Id::unique(title))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .movable(true)
        .fade_in(false)
        .default_pos(pos2(
            (v.left() + 20.0).min(area.right() - WIDE - 8.0).max(area.left()),
            (v.top() + 16.0).min(area.bottom() - h - 8.0).max(area.top()),
        ))
        .fixed_size(vec2(WIDE, h))
        .frame(dialog_frame(t).inner_margin(0))
        .constrain_to(area)
        .show_placed(ctx, places, |ui, places| {
            let all = Rect::from_min_size(ui.max_rect().min, vec2(WIDE, h));
            ui.allocate_rect(all, Sense::hover());
            let head = Rect::from_min_size(all.min, vec2(WIDE, HEAD));
            let foot = Rect::from_min_max(pos2(all.left(), all.bottom() - FOOT), all.max);
            let body = Rect::from_min_max(pos2(all.left(), head.bottom()), pos2(all.right(), foot.top()));
            let p = ui.painter().clone();
            p.hline(all.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.line));
            p.hline(all.x_range(), foot.top() + 0.5, Stroke::new(1.0, t.line));
            ui.scope_builder(UiBuilder::new().max_rect(head.shrink2(vec2(16.0, 11.0))), |ui| {
                close |= dialog_head(ui, t, icon, title, "harness:dialog", places);
            });
            let mut use_now = false;
            ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
                ui.set_clip_rect(body.intersect(ui.clip_rect()));
                match d {
                    Dialog::WireType { name, fields, was, .. } => wire_type_body(ui, t, name, fields, was.as_ref(), places),
                    Dialog::Library {
                        search,
                        chips,
                        sort,
                        rows,
                        problem,
                        picked,
                        ..
                    } => use_now = library_body(ui, t, search, chips, sort, rows, problem, picked, places),
                    Dialog::Wire { .. } => wire_body(ui, t, d, hs, places),
                    _ => {}
                }
            });
            // the body fades into the footer: more below, not a row cut in half
            if !matches!(d, Dialog::Library { .. }) {
                for k in 0..24 {
                    let f = k as f32 / 24.0;
                    let y = body.bottom() - 24.0 + k as f32;
                    p.rect_filled(
                        Rect::from_min_max(pos2(body.left(), y), pos2(body.right() - 10.0, y + 1.0)),
                        0.0,
                        t.surface.gamma_multiply(f * f),
                    );
                }
            }
            let words = match d {
                Dialog::WireType { editing: true, .. } => "Save",
                Dialog::WireType { .. } => "Create",
                Dialog::Library { .. } => "Use",
                _ => "Add",
            };
            let can = !matches!(d, Dialog::Library { picked: None, .. });
            let bh = 30.0;
            let y = foot.center().y - bh / 2.0;
            let cancel = Rect::from_min_size(pos2(foot.right() - 16.0 - 88.0, y), vec2(88.0, bh));
            let go = Rect::from_min_size(pos2(cancel.left() - 8.0 - 88.0, y), vec2(88.0, bh));
            let g = button_at(ui, t, go, words, true, can);
            places.push(("harness:dialog:ok".into(), g.rect));
            let c = button_at(ui, t, cancel, "Cancel", false, true);
            places.push(("harness:dialog:cancel".into(), c.rect));
            ok |= g.clicked() || use_now;
            close |= c.clicked();
            if let Dialog::WireType { fields, .. } = d {
                let missing = ["od", "source"]
                    .iter()
                    .filter(|k| get_of(fields)(k).is_empty() && !(**k == "od" && !get_of(fields)("awg").is_empty()))
                    .map(|k| if *k == "od" { "an OD or gauge" } else { "a source" })
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    p.text(
                        pos2(foot.left() + 16.0, foot.center().y),
                        Align2::LEFT_CENTER,
                        format!("Needs {}", missing.join(" and ")),
                        sans(12.0),
                        t.ink2,
                    );
                }
            }
        });
    (ok, close, r.map(|r| r.response.rect))
}

/// The fields as values: as written where that is a number or words, else (a param, an
/// expression) as the edited type worked it out.
fn resolved(fields: &[(String, String)], was: Option<&TypeOut>) -> Vec<(String, String)> {
    forge_lang::harness::SPEC
        .iter()
        .filter_map(|&(k, _, num)| {
            let f = fields.iter().find(|x| x.0 == k).map(|x| bare(&x.1)).unwrap_or_default();
            let v = if f.is_empty() || !num || f.parse::<f64>().is_ok() {
                f
            } else {
                was.map(|w| bare(w.get(k))).unwrap_or_default()
            };
            (!v.is_empty()).then(|| (k.to_string(), v))
        })
        .collect()
}

fn wire_type_body(
    ui: &mut Ui,
    t: &Theme,
    name: &mut String,
    fields: &mut Vec<(String, String)>,
    was: Option<&TypeOut>,
    places: &mut Vec<(String, Rect)>,
) {
    let values = resolved(fields, was);
    {
        let get = get_of(&values);
        let s = Section::of(&get, "", name);
        let pills: Vec<String> = ["spec_tag", "mpn", "part_number"].iter().map(|k| get(k)).collect();
        header(ui, t, &s, name, &line_of(&get), &pills);
    }
    let (ohm, area) = worked(&values);
    let set = |fields: &mut Vec<(String, String)>, k: &str, v: String| match fields.iter_mut().find(|x| x.0 == k) {
        Some(f) => f.1 = v,
        None if !v.is_empty() => fields.push((k.to_string(), v)),
        None => {}
    };
    let val = |k: &str, fields: &Vec<(String, String)>| fields.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap_or_default();
    egui::ScrollArea::vertical()
        .id_salt("harness-wt-body")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            heading(ui, t, "Identity");
            let r = edit_row(ui, t, "Name", name, "", "wt1", "name");
            places.push(("harness:wt-name".into(), r));
            let mut group = "";
            for &(k, g, _) in forge_lang::harness::SPEC {
                if g != group {
                    heading(ui, t, g);
                    group = g;
                }
                let shown = bare(&val(k, fields));
                let mut edited = shown.clone();
                match k {
                    "conductor" => choice_row(ui, t, "Material", &mut edited, CONDUCTORS, k),
                    "insulation" => choice_row(ui, t, "Material", &mut edited, INSULATIONS, k),
                    "spec" => choice_row(ui, t, "Standard", &mut edited, SPECS, k),
                    "colour" => {
                        let mut parts: Vec<String> = forge_lang::harness::colour_codes(&shown);
                        parts.resize(3, String::new());
                        let mut c = [parts[0].clone(), parts[1].clone(), parts[2].clone()];
                        let a = colour_grid(ui, t, "Colour", &mut c[0], true, "wt-c0");
                        let b = colour_grid(ui, t, "Tracer", &mut c[1], true, "wt-c1");
                        if a || b {
                            edited = c.iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join("/");
                        }
                    }
                    _ => {
                        let (label, unit, hint) = field_words(k);
                        let r = edit_row(ui, t, label, &mut edited, unit, hint, k);
                        // a param or expression: what it is now, at the field's right
                        let now = values.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap_or_default();
                        if !edited.is_empty() && edited.parse::<f64>().is_err() && !now.is_empty() && now != edited {
                            ui.painter().text(
                                pos2(r.right() - 8.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                format!("= {now}"),
                                mono(11.5),
                                t.ink2,
                            );
                        }
                        places.push((format!("harness:wt-{k}"), r));
                    }
                }
                if edited != shown {
                    set(fields, k, edited);
                }
                if k == "min_bend" && val("min_bend", fields).trim().is_empty() {
                    let od = values.iter().find(|x| x.0 == "od").and_then(|x| x.1.parse::<f64>().ok());
                    if let Some(od) = od {
                        computed(ui, t, "", &format!("{} mm", super::fg::short(3.0 * od)), "3 × OD");
                    }
                }
                if k == "ohm_km" && val("ohm_km", fields).trim().is_empty() {
                    if let Some(o) = ohm {
                        computed(ui, t, "", &format!("{} ohm/km", super::fg::short((o * 100.0).round() / 100.0)), "from cross-section");
                    }
                    if let Some(a) = area.filter(|_| val("mm2", fields).trim().is_empty()) {
                        computed(ui, t, "", &format!("{} mm²", super::fg::short((a * 1000.0).round() / 1000.0)), "conductor area");
                    }
                }
            }
            ui.add_space(48.0);
        });
}

/// A field's label, unit and hint.
fn field_words(k: &str) -> (&'static str, &'static str, &'static str) {
    match k {
        "awg" => ("Gauge", "AWG", ""),
        "mm2" => ("Cross-section", "mm²", ""),
        "strands" => ("Strands", "", ""),
        "strand_d" => ("Strand", "mm", ""),
        "conductor_d" => ("Conductor Ø", "mm", ""),
        "ohm_km" => ("Resistance", "ohm/km", ""),
        "wall" => ("Wall", "mm", ""),
        "od" => ("Outer Ø", "mm", ""),
        "temp" => ("Temperature", "°C", ""),
        "volts" => ("Voltage", "V", ""),
        "min_bend" => ("Min bend", "mm", ""),
        "spec_tag" => ("Tag", "", ""),
        "maker" => ("Maker", "", ""),
        "mpn" => ("MPN", "", ""),
        "part_number" => ("Our part number", "", ""),
        "source" => ("Source", "", "datasheet, URL or page"),
        _ => ("", "", ""),
    }
}

#[allow(clippy::too_many_arguments)]
fn library_body(
    ui: &mut Ui,
    t: &Theme,
    search: &mut String,
    chips: &mut Vec<String>,
    sort: &mut (usize, bool),
    rows: &[LibRow],
    problem: &Option<String>,
    picked: &mut Option<usize>,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let area = ui.max_rect();
    let p = ui.painter().clone();
    // the filter bar: search, then a chip for each conductor and insulation the library has
    let bar = Rect::from_min_size(area.min, vec2(area.width(), 84.0));
    p.rect_filled(bar, 0.0, t.raised);
    p.hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, t.line));
    let sr = Rect::from_min_size(bar.min + vec2(16.0, 10.0), vec2(bar.width() - 32.0, 28.0));
    let s = ui.put(
        sr,
        egui::TextEdit::singleline(search)
            .font(sans(13.0))
            .hint_text("Search name, gauge, spec, maker")
            .margin(vec2(28.0, 5.0)),
    );
    places.push(("harness:lib-search".into(), s.rect));
    super::icons::draw(&p, sr.left_top() + vec2(7.0, 6.0), 16.0, "zoom", 1.4, t.ink2);
    let mut words: Vec<String> = vec![];
    for r in rows {
        for k in ["conductor", "insulation"] {
            let v = r.get(k);
            if !v.is_empty() && !words.contains(&v) {
                words.push(v);
            }
        }
    }
    let mut x = bar.left() + 16.0;
    let cy = bar.top() + 62.0;
    for w in &words {
        let on = chips.contains(w);
        let g = p.layout_no_wrap(w.clone(), medium(11.5), if on { t.on_ink } else { t.ink2 });
        let cr = Rect::from_min_size(pos2(x, cy - 12.0), vec2(g.size().x + 22.0, 24.0));
        let resp = ui.interact(cr, ui.scope_id().with(("chip", w)), Sense::click());
        p.rect_filled(cr, 12.0, if on { t.on } else if resp.hovered() { t.hover } else { t.surface });
        p.rect_stroke(cr, 12.0, Stroke::new(1.0, if on { t.on_ink } else { t.line }), StrokeKind::Inside);
        p.galley(pos2(cr.left() + 11.0, cr.center().y - g.size().y / 2.0), g, t.ink);
        if resp.clicked() {
            if on {
                chips.retain(|c| c != w);
            } else {
                chips.push(w.clone());
            }
        }
        x = cr.right() + 6.0;
    }
    if !chips.is_empty() {
        let cr = Rect::from_min_size(pos2(x + 4.0, cy - 12.0), vec2(48.0, 24.0));
        let resp = ui.interact(cr, ui.scope_id().with("chips-clear"), Sense::click());
        p.text(cr.center(), Align2::CENTER_CENTER, "Clear", sans(11.5), if resp.hovered() { t.ink } else { t.ink2 });
        if resp.clicked() {
            chips.clear();
        }
    }
    let q = search.trim().to_lowercase();
    let mut shown: Vec<usize> = (0..rows.len())
        .filter(|&i| {
            let r = &rows[i];
            let hit = q.is_empty() || r.name.to_lowercase().contains(&q) || r.spec.values().any(|v| v.text().to_lowercase().contains(&q));
            let chip = chips.iter().all(|c| r.get("conductor") == *c || r.get("insulation") == *c);
            hit && chip
        })
        .collect();
    // columns: swatch, name, gauge, conductor, insulation, OD, spec
    let cols: [(&str, f32); 7] = [("", 36.0), ("Name", 150.0), ("Gauge", 84.0), ("Conductor", 92.0), ("Insulation", 82.0), ("OD", 54.0), ("Spec", 0.0)];
    let gauge_key = |r: &LibRow| {
        r.get("mm2")
            .parse::<f64>()
            .ok()
            .or_else(|| r.get("awg").parse::<f64>().ok().map(|a| {
                let d = forge_harness::awg_d(a);
                std::f64::consts::PI * d * d / 4.0
            }))
            .unwrap_or(0.0)
    };
    let (col, up) = *sort;
    shown.sort_by(|&a, &b| {
        let (ra, rb) = (&rows[a], &rows[b]);
        let o = match col {
            2 => gauge_key(ra).total_cmp(&gauge_key(rb)),
            3 => ra.get("conductor").cmp(&rb.get("conductor")),
            4 => ra.get("insulation").cmp(&rb.get("insulation")),
            5 => ra.get("od").parse::<f64>().unwrap_or(0.0).total_cmp(&rb.get("od").parse::<f64>().unwrap_or(0.0)),
            6 => ra.get("spec").cmp(&rb.get("spec")),
            _ => ra.name.cmp(&rb.name),
        };
        if up { o } else { o.reverse() }
    });
    let preview_h = 132.0;
    let hr = Rect::from_min_size(pos2(area.left(), bar.bottom()), vec2(area.width(), 30.0));
    p.hline(hr.x_range(), hr.bottom() - 0.5, Stroke::new(1.0, t.line));
    let mut xs = vec![];
    let mut x = hr.left() + 8.0;
    for (k, (name, w)) in cols.iter().enumerate() {
        let w = if *w == 0.0 { hr.right() - 12.0 - x } else { *w };
        xs.push((x, w));
        if !name.is_empty() {
            let cr = Rect::from_min_size(pos2(x, hr.top()), vec2(w, hr.height()));
            let resp = ui.interact(cr, ui.scope_id().with(("col", k)), Sense::click());
            let ink = if col == k || resp.hovered() { t.ink } else { t.ink2 };
            let right = k == 5;
            let tx = if right { cr.right() - 6.0 } else { cr.left() + 6.0 };
            let at = p.text(pos2(tx, cr.center().y), if right { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }, name.to_uppercase(), semi(10.5), ink);
            if col == k {
                let ax = if right { at.left() - 9.0 } else { at.right() + 5.0 };
                let c = pos2(ax + 3.0, cr.center().y);
                let (a, b, tip) = if up {
                    (c + vec2(-3.5, 2.0), c + vec2(3.5, 2.0), c + vec2(0.0, -2.5))
                } else {
                    (c + vec2(-3.5, -2.0), c + vec2(3.5, -2.0), c + vec2(0.0, 2.5))
                };
                p.add(egui::Shape::convex_polygon(vec![a, b, tip], ink, Stroke::NONE));
            }
            if resp.clicked() {
                *sort = if col == k { (k, !up) } else { (k, true) };
            }
        }
        x += w;
    }
    let list = Rect::from_min_max(pos2(area.left(), hr.bottom()), pos2(area.right(), area.bottom() - preview_h));
    let mut use_now = false;
    ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("harness-lib-table")
            .auto_shrink([false, false])
            .max_height(list.height())
            .show(ui, |ui| {
                let w = ui.available_width();
                for (n, &i) in shown.iter().enumerate() {
                    let r = &rows[i];
                    let (rr, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
                    let p = ui.painter().clone();
                    let on = *picked == Some(i);
                    if on {
                        p.rect_filled(rr, 0.0, t.on);
                        p.vline(rr.left() + 1.0, rr.y_range(), Stroke::new(2.0, t.on_ink));
                    } else if resp.hovered() {
                        p.rect_filled(rr, 0.0, t.hover);
                    } else if n % 2 == 1 {
                        p.rect_filled(rr, 0.0, t.raised.gamma_multiply(0.5));
                    }
                    let cell = |k: usize| {
                        let (x, w) = xs[k];
                        Rect::from_min_size(pos2(x + 6.0, rr.top()), vec2(w - 12.0, rr.height()))
                    };
                    let sw = cell(0);
                    let mini = Rect::from_center_size(pos2(sw.left() + 12.0, rr.center().y), vec2(24.0, 24.0));
                    let sec = Section::of(&|k: &str| r.get(k), "", &r.name);
                    cross_section(&p, t, mini, &sec, false);
                    clipped(ui, cell(1), &r.name, medium(12.5), t.ink, &format!("lib-n{i}"));
                    let gauge = if !r.get("awg").is_empty() {
                        format!("{} AWG", r.get("awg"))
                    } else if !r.get("mm2").is_empty() {
                        format!("{} mm²", r.get("mm2"))
                    } else {
                        String::new()
                    };
                    if !gauge.is_empty() {
                        pill(ui.painter(), pos2(cell(2).left(), rr.center().y), false, &gauge, t.ink, t.hover);
                    }
                    clipped(ui, cell(3), &r.get("conductor"), sans(12.0), t.ink, &format!("lib-c{i}"));
                    clipped(ui, cell(4), &r.get("insulation"), sans(12.0), t.ink, &format!("lib-i{i}"));
                    let od = r.get("od");
                    if !od.is_empty() {
                        ui.painter().text(pos2(cell(5).right(), rr.center().y), Align2::RIGHT_CENTER, &od, mono(11.5), t.ink);
                    }
                    let spec = [r.get("spec"), r.get("maker")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join("  ·  ");
                    clipped(ui, cell(6), &spec, sans(12.0), t.ink2, &format!("lib-s{i}"));
                    places.push((format!("harness:lib:{i}"), rr));
                    if resp.clicked() {
                        *picked = Some(i);
                    }
                    if resp.double_clicked() {
                        *picked = Some(i);
                        use_now = true;
                    }
                }
                if shown.is_empty() {
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(if rows.is_empty() { "No wire library found" } else { "Nothing matches" }).font(sans(13.0)).color(t.ink2));
                        if rows.is_empty() {
                            ui.label(egui::RichText::new("wires/*.csv beside the document, or ~/.config/forgecad/wires/").font(mono(11.0)).color(t.ink2));
                        }
                    });
                }
            });
    });
    // the picked wire, its section and what it is
    let pv = Rect::from_min_max(pos2(area.left(), area.bottom() - preview_h), area.max);
    p.rect_filled(pv, 0.0, t.raised);
    p.hline(pv.x_range(), pv.top() + 0.5, Stroke::new(1.0, t.line));
    if let Some(e) = problem {
        p.text(pos2(pv.left() + 16.0, pv.top() + 16.0), Align2::LEFT_CENTER, e, sans(12.0), t.error);
    }
    match picked.and_then(|i| rows.get(i)) {
        Some(r) => {
            let get = |k: &str| r.get(k);
            let sec = Section::of(&get, "", &r.name);
            cross_section(&p, t, Rect::from_min_size(pv.min + vec2(14.0, 6.0), vec2(150.0, 122.0)), &sec, true);
            let x = pv.left() + 180.0;
            p.text(pos2(x, pv.top() + 26.0), Align2::LEFT_CENTER, &r.name, semi(16.0), t.ink);
            p.text(pos2(x, pv.top() + 50.0), Align2::LEFT_CENTER, line_of(&get), sans(12.0), t.ink2);
            let mut lines = vec![];
            let parts = [get("maker"), get("mpn")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" ");
            if !parts.is_empty() {
                lines.push(parts);
            }
            let rating = [
                (get("temp"), "°C"),
                (get("volts"), "V"),
            ]
            .into_iter()
            .filter(|x| !x.0.is_empty())
            .map(|(v, u)| format!("{v} {u}"))
            .collect::<Vec<_>>()
            .join(" · ");
            if !rating.is_empty() {
                lines.push(rating);
            }
            for (k, l) in lines.iter().enumerate() {
                p.text(pos2(x, pv.top() + 74.0 + 20.0 * k as f32), Align2::LEFT_CENTER, l, sans(12.0), t.ink);
            }
            let from = format!("{}:{}", std::path::Path::new(&r.file).file_name().map_or(String::new(), |f| f.to_string_lossy().to_string()), r.line);
            p.text(pos2(pv.right() - 16.0, pv.bottom() - 14.0), Align2::RIGHT_CENTER, from, mono(10.5), t.ink2);
        }
        None => {
            p.text(pv.center(), Align2::CENTER_CENTER, format!("{} wire types", shown.len()), sans(12.5), t.ink2);
        }
    }
    use_now
}

fn wire_body(ui: &mut Ui, t: &Theme, d: &mut Dialog, hs: &[HarnessOut], places: &mut Vec<(String, Rect)>) {
    let Dialog::Wire {
        h,
        name,
        from,
        from_pin,
        to,
        to_pin,
        ty,
        colour,
    } = d
    else {
        return;
    };
    let Some(ho) = hs.get(*h) else { return };
    let tyo: Option<&TypeOut> = ho.types.iter().find(|x| x.name == *ty);
    let code = {
        let c = colour.iter().map(|x| x.trim()).filter(|x| !x.is_empty()).collect::<Vec<_>>().join("/");
        if c.is_empty() { tyo.map(|x| bare(x.get("colour"))).unwrap_or_default() } else { c }
    };
    // the preview: the wire from the side and its section
    let area = ui.max_rect();
    let pv = Rect::from_min_size(area.min, vec2(area.width(), 118.0));
    let p = ui.painter().clone();
    p.rect_filled(pv, 0.0, t.raised);
    p.hline(pv.x_range(), pv.bottom() - 0.5, Stroke::new(1.0, t.line));
    let tget = |k: &str| tyo.map(|x| bare(x.get(k))).unwrap_or_default();
    let sec = Section::of(&tget, &code, name);
    cross_section(&p, t, Rect::from_min_size(pv.min + vec2(14.0, 9.0), vec2(100.0, 100.0)), &sec, false);
    side_view(&p, Rect::from_min_max(pos2(pv.left() + 124.0, pv.top() + 58.0), pos2(pv.right() - 14.0, pv.bottom() - 8.0)), &code, name);
    p.text(pos2(pv.left() + 134.0, pv.top() + 22.0), Align2::LEFT_CENTER, name.as_str(), semi(16.0), t.ink);
    let mut sub = vec![];
    if !code.is_empty() {
        sub.push(code.clone());
    }
    if !ty.is_empty() {
        sub.push(ty.clone());
    }
    let l = line_of(&tget);
    if !l.is_empty() {
        sub.push(l);
    }
    p.text(pos2(pv.left() + 134.0, pv.top() + 42.0), Align2::LEFT_CENTER, sub.join("  ·  "), sans(12.0), t.ink2);
    ui.allocate_rect(pv, Sense::hover());
    egui::ScrollArea::vertical()
        .id_salt("harness-wire-body")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            heading(ui, t, "Wire");
            let r = edit_row(ui, t, "Name", name, "", "W1", "wire-name");
            places.push(("harness:wire-name".into(), r));
            // the two ends side by side, each a connector and its pin
            heading(ui, t, "Ends");
            let w = ui.available_width();
            let (er, _) = ui.allocate_exact_size(vec2(w, 92.0), Sense::hover());
            let half = (er.width() - 16.0 * 2.0 - 12.0) / 2.0;
            let conns: Vec<&forge_lang::harness::ConnOut> = ho.connectors.iter().collect();
            for (k, (label, c, pin)) in [("From", &mut *from, &mut *from_pin), ("To", &mut *to, &mut *to_pin)].into_iter().enumerate() {
                let card = Rect::from_min_size(pos2(er.left() + 16.0 + k as f32 * (half + 12.0), er.top()), vec2(half, er.height()));
                let pa = ui.painter();
                pa.rect_filled(card, 6.0, t.raised);
                pa.rect_stroke(card, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                pa.text(card.left_top() + vec2(12.0, 14.0), Align2::LEFT_CENTER, label.to_uppercase(), semi(10.5), t.ink2);
                let pins: Vec<String> = conns.iter().find(|x| x.name == *c).map(|x| x.pins.clone()).unwrap_or_default();
                if !pins.is_empty() {
                    pa.text(card.right_top() + vec2(-12.0, 14.0), Align2::RIGHT_CENTER, format!("{} pins", pins.len()), sans(11.0), t.ink2);
                }
                let cr = Rect::from_min_size(card.left_top() + vec2(12.0, 30.0), vec2(card.width() - 24.0 - 96.0, 28.0));
                let mut cui = ui.new_child(UiBuilder::new().max_rect(cr));
                let rr = egui::ComboBox::from_id_salt(("harness-end", k))
                    .selected_text(egui::RichText::new(c.as_str()).font(medium(13.0)))
                    .width(cr.width())
                    .show_ui(&mut cui, |ui| {
                        for x in &conns {
                            ui.selectable_value(c, x.name.clone(), format!("{}   {}", x.name, x.face));
                        }
                    });
                places.push((format!("harness:wire-{}", if k == 0 { "from" } else { "to" }), rr.response.rect));
                let pr = Rect::from_min_size(pos2(cr.right() + 8.0, cr.top()), vec2(88.0, 28.0));
                let mut pui = ui.new_child(UiBuilder::new().max_rect(pr));
                if pins.is_empty() {
                    pui.add(egui::TextEdit::singleline(pin).font(mono(12.5)).desired_width(pr.width()).margin(vec2(7.0, 5.0)));
                } else {
                    egui::ComboBox::from_id_salt(("harness-pin", k))
                        .selected_text(egui::RichText::new(format!("pin {pin}")).font(mono(12.0)))
                .wrap_mode(egui::TextWrapMode::Extend)
                        .width(pr.width())
                        .height(300.0)
                        .show_ui(&mut pui, |ui| {
                            for x in &pins {
                                ui.selectable_value(pin, x.clone(), format!("pin {x}"));
                            }
                        });
                }
                let face = conns.iter().find(|x| x.name == *c).map(|x| x.face.clone()).unwrap_or_default();
                ui.painter().text(card.left_bottom() + vec2(12.0, -14.0), Align2::LEFT_CENTER, face, mono(10.5), t.ink2);
            }
            heading(ui, t, "Type");
            let r = field(ui, t, "Wire type", 30.0);
            let mut tui = ui.new_child(UiBuilder::new().max_rect(r));
            let shown = tyo.map(|x| format!("{}    {}", x.name, line_of(&|k: &str| bare(x.get(k))))).unwrap_or_else(|| ty.clone());
            let rr = egui::ComboBox::from_id_salt("harness-wire-type")
                .selected_text(egui::RichText::new(shown).font(sans(12.5)))
                .width(r.width())
                .show_ui(&mut tui, |ui| {
                    for x in &ho.types {
                        let (row, resp) = ui.allocate_exact_size(vec2(r.width() - 12.0, 28.0), Sense::click());
                        if *ty == x.name {
                            ui.painter().rect_filled(row, 4.0, t.on);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(row, 4.0, t.hover);
                        }
                        let s = Section::of(&|k: &str| bare(x.get(k)), "", &x.name);
                        cross_section(ui.painter(), t, Rect::from_center_size(pos2(row.left() + 16.0, row.center().y), vec2(22.0, 22.0)), &s, false);
                        ui.painter().text(pos2(row.left() + 34.0, row.center().y), Align2::LEFT_CENTER, &x.name, medium(12.5), t.ink);
                        ui.painter().text(pos2(row.left() + 90.0, row.center().y), Align2::LEFT_CENTER, line_of(&|k: &str| bare(x.get(k))), sans(11.5), t.ink2);
                        if resp.clicked() {
                            *ty = x.name.clone();
                            ui.close();
                        }
                    }
                });
            places.push(("harness:wire-type".into(), rr.response.rect));
            heading(ui, t, "Colour");
            colour_grid(ui, t, "Base", &mut colour[0], true, "wire-c0");
            colour_grid(ui, t, "Tracer", &mut colour[1], true, "wire-c1");
            colour_grid(ui, t, "Second tracer", &mut colour[2], true, "wire-c2");
            ui.add_space(48.0);
        });
}

/// A wire's or a wire type's properties, laid out; false for anything else.
pub fn props(ui: &mut Ui, t: &Theme, hs: &[HarnessOut], key: &str, max_h: f32) -> bool {
    let Some(rest) = key.strip_prefix("h:") else { return false };
    for h in hs {
        let Some(k) = rest.strip_prefix(&format!("{}:", h.id)) else { continue };
        let parts: Vec<&str> = k.split(':').collect();
        let (wire, ty) = match parts.as_slice() {
            ["w", w] | ["s", _, w] => {
                let Some(wo) = h.wires.iter().find(|x| x.name == *w) else { return false };
                (Some(wo), h.types.iter().find(|x| x.name == wo.ty))
            }
            ["t", n] => (None, h.types.iter().find(|x| x.name == *n)),
            _ => return false,
        };
        let get = |k: &str| ty.map(|x| bare(x.get(k))).unwrap_or_default();
        let colour = wire.map(|w| w.colour.clone()).unwrap_or_default();
        let name = wire.map_or_else(|| ty.map(|x| x.name.clone()).unwrap_or_default(), |w| w.name.clone());
        let mut sec = Section::of(&get, &colour, &name);
        if let Some(w) = wire {
            sec.od = w.od;
        }
        let mut sub = vec![];
        if !colour.is_empty() {
            sub.push(colour.clone());
        }
        if let Some(x) = ty.filter(|_| wire.is_some()) {
            sub.push(x.name.clone());
        }
        let l = line_of(&get);
        if !l.is_empty() {
            sub.push(l);
        }
        let pills: Vec<String> = ["spec_tag", "mpn", "part_number"].iter().map(|k| get(k)).collect();
        header(ui, t, &sec, &format!("{}.{}", h.id, name), &sub.join("  ·  "), &pills);
        let out = egui::ScrollArea::vertical()
            .id_salt("harness-props-body")
            .max_height(max_h - 150.0)
            .min_scrolled_height((props_height(ui.ctx()) - 150.0).clamp(80.0, max_h - 150.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                if let Some(w) = wire {
                    heading(ui, t, "Connection");
                    value_row(ui, t, "From", &w.from, "", None);
                    value_row(ui, t, "To", &w.to, "", None);
                    value_row(ui, t, "Path", &w.path.join(" › "), "", if w.given { None } else { Some("shortest") });
                    if let Some(o) = &w.routed {
                        heading(ui, t, "Cut length");
                        length_bar(
                            ui,
                            t,
                            &[("route", o.route), ("loops", o.loops), ("slack", o.slack), ("insertion", o.insertion)],
                            o.length,
                        );
                    }
                    if w.ohms.is_some() || w.current.is_some() {
                        heading(ui, t, "Electrical");
                        if let Some(r) = w.ohms {
                            value_row(ui, t, "Resistance", &super::fg::short((r * 10000.0).round() / 10000.0), "ohm", None);
                        }
                        if let Some(i) = w.current {
                            value_row(ui, t, "Current", &super::fg::short(i), "A", None);
                        }
                        if let Some(v) = w.vdrop {
                            value_row(ui, t, "Voltage drop", &super::fg::short((v * 1000.0).round() / 1000.0), "V", None);
                        }
                    }
                }
                if let Some(ty) = ty {
                    let mut group = "";
                    for &(k, g, _) in forge_lang::harness::SPEC {
                        let v = bare(ty.get(k));
                        let derived = k == "ohm_km" && v.is_empty() && ty.ohm_worked;
                        if v.is_empty() && !derived {
                            continue;
                        }
                        if g != group {
                            heading(ui, t, g);
                            group = g;
                        }
                        let (label, unit, _) = field_words(k);
                        let label = match k {
                            "conductor" | "insulation" => "Material",
                            "spec" => "Standard",
                            "colour" => "Colour",
                            _ => label,
                        };
                        if derived {
                            if let Some(o) = ty.ohm_km {
                                value_row(ui, t, label, &super::fg::short((o * 100.0).round() / 100.0), unit, Some("from cross-section"));
                            }
                        } else if k == "colour" {
                            let r = field(ui, t, label, 26.0);
                            let s = Section { od: 1.0, conductor_d: Some(0.5), colour: v.clone(), name: name.clone(), ..Default::default() };
                            cross_section(ui.painter(), t, Rect::from_center_size(pos2(r.left() + 9.0, r.center().y), vec2(18.0, 18.0)), &s, false);
                            ui.painter().text(pos2(r.left() + 24.0, r.center().y), Align2::LEFT_CENTER, &v, mono(12.0), t.ink);
                        } else if k == "source" {
                            let r = field(ui, t, label, 26.0);
                            clipped(ui, r, &v, sans(12.0), t.ink, "props-source");
                        } else {
                            value_row(ui, t, label, &v, unit, None);
                        }
                    }
                }
                ui.add_space(10.0);
            });
        let tall = out.content_size.y + 150.0;
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("harness-props-h"), tall));
        return true;
    }
    false
}

/// How tall the properties' contents were when last laid out.
fn props_height(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(egui::Id::unique("harness-props-h")))
        .unwrap_or(400.0)
}
