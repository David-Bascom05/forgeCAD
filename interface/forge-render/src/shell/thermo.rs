//! A thermoform as the app shows it: its numbers, and over the view, while the Thickness
//! analysis is on, the wall-thickness map's banded legend (thickest at the top, as analysis tools
//! stack it) with the model and the law it read, MIN, MAX and probes pinned on the part, and the
//! forming's steps on a scrubber at the view's foot.
use super::Shell;
use super::theme::{mono, sans, semi};
use super::thermo_ui::Analysis;
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

/// One thermoform as the app shows it (made from forge-lang's report on the host's side).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ThermoRow {
    pub id: String,
    pub facts: Vec<(String, String)>,
    /// The thinnest and thickest wall, mm, and where (world).
    pub min: (f64, [f32; 3]),
    pub max: (f64, [f32; 3]),
    /// The thinnest the material allows, mm.
    pub limit: f64,
    /// What the numbers are: the model and the law it read.
    pub model: String,
    pub material: String,
    pub female: bool,
    pub from: String,
    pub pull: String,
    pub t: f64,
    pub shrink: f64,
    /// The material's least draft for the mould, degrees, and each face of the mould's surface's
    /// least draft.
    pub draft_min: f32,
    pub face_drafts: Vec<f32>,
    /// A section through the middle along x: the mould's top, the formed sheet's, and the base.
    pub section: Vec<[f32; 2]>,
    pub section_part: Vec<[f32; 2]>,
    pub base: f32,
    /// The forming's steps, by stage.
    pub stages: Vec<&'static str>,
    /// The sheet's points on the part (world) and the wall there, mm, at the last step.
    pub points: Vec<([f32; 3], f32)>,
}

/// The wall-thickness map's bands, thickest first.
pub const BANDS: [[f32; 3]; 10] = [
    [0.17, 0.35, 0.63],
    [0.18, 0.53, 0.84],
    [0.18, 0.72, 0.79],
    [0.24, 0.79, 0.54],
    [0.55, 0.81, 0.24],
    [0.85, 0.82, 0.23],
    [0.95, 0.66, 0.23],
    [0.94, 0.42, 0.20],
    [0.85, 0.20, 0.18],
    [0.55, 0.12, 0.25],
];

/// The draft analysis's colours: OK, low, very low, undercut, flat.
pub const DRAFT: [[f32; 3]; 5] = [
    [0.24, 0.68, 0.42],
    [0.89, 0.75, 0.29],
    [0.96, 0.52, 0.49],
    [0.79, 0.42, 0.85],
    [0.55, 0.59, 0.64],
];

/// The band of a thickness `f` of the way from the thinnest (0) to the thickest (1).
pub fn band(f: f32) -> [f32; 3] {
    let k = ((1.0 - f.clamp(0.0, 1.0)) * 10.0) as usize;
    BANDS[k.min(9)]
}

pub fn c32(c: [f32; 3]) -> Color32 {
    Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8)
}

/// A flag pinned on the part: a dot where it is, a line to its label.
fn flag(p: &egui::Painter, at: Pos2, text: String, fill: Color32, ink: Color32, toward: egui::Vec2) {
    let g = p.layout_no_wrap(text, mono(12.0), ink);
    let tip = at + toward;
    let r = Rect::from_min_size(
        tip - vec2(
            if toward.x < 0.0 { g.size().x + 10.0 } else { 0.0 },
            g.size().y + 8.0,
        ),
        g.size() + vec2(10.0, 8.0),
    );
    p.line_segment([at, tip], Stroke::new(1.2, Color32::WHITE));
    p.circle_filled(at, 4.5, Color32::WHITE);
    p.circle_filled(at, 3.0, Color32::from_rgb(20, 24, 30));
    p.rect_filled(r, 2.0, fill);
    p.rect_stroke(r, 2.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
    p.galley(r.min + vec2(5.0, 4.0), g, ink);
}

impl Shell {
    /// The thickness map's legend and flags, and the step scrubber, over the view.
    pub(super) fn thermo_probes(&mut self, ctx: &egui::Context) {
        let Some(fg) = &mut self.fg else { return };
        if fg.thermo.analysis != Analysis::Thickness || fg.thermos.is_empty() {
            return;
        }
        let Some((camera, size)) = self.camera else { return };
        let (t, view, ppp) = (self.theme, self.view_rect, ctx.pixels_per_point());
        let r = fg.thermos[0].clone();
        let n = r.stages.len().max(1);
        // the steps play on their own
        if fg.thermo.playing {
            let now = ctx.input(|i| i.time);
            let k = ((now * 8.0) as usize) % n;
            fg.thermo.step = Some(k);
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let step = fg.thermo.step.unwrap_or(n - 1).min(n - 1);
        let playing = fg.thermo.playing;
        let mut places = vec![];
        let mut scrub = None;
        let mut play = false;
        let mut pin = None;
        let probe = fg.thermo.probe;
        let probes = fg.thermo.probes.clone();
        egui::Area::new(egui::Id::unique("thermo-thickness"))
            .fixed_pos(view.min)
            .order(egui::Order::Middle)
            .interactable(true)
            .constrain(false)
            .show(ctx, |ui| {
                let p = ui.painter().clone();
                let (lo, hi) = (r.min.0, r.max.0);
                // the legend: title, the model's step and sheet, the bands
                let (x, y0) = (view.min.x + 24.0, view.min.y + 56.0);
                p.rect_filled(
                    Rect::from_min_size(pos2(x - 12.0, y0 - 12.0), vec2(260.0, 380.0)),
                    6.0,
                    t.bg.gamma_multiply(0.88),
                );
                p.text(pos2(x, y0), Align2::LEFT_TOP, "Wall thickness", semi(14.0), t.ink);
                p.text(
                    pos2(x, y0 + 22.0),
                    Align2::LEFT_TOP,
                    format!("Unit: mm · step {} / {n}", step + 1),
                    sans(11.0),
                    t.ink2,
                );
                p.text(
                    pos2(x, y0 + 38.0),
                    Align2::LEFT_TOP,
                    format!("sheet {:.2} · {}", r.t, r.material),
                    sans(11.0),
                    t.ink2,
                );
                let (bh, by) = (22.0, y0 + 62.0);
                for (k, c) in BANDS.iter().enumerate() {
                    let sw = Rect::from_min_size(pos2(x, by + k as f32 * bh), vec2(26.0, bh));
                    p.rect_filled(sw, 0.0, c32(*c));
                    let v = hi - (hi - lo) * k as f64 / 9.0;
                    let tag = match k {
                        0 => " Max",
                        9 => " Min",
                        _ => "",
                    };
                    p.text(
                        pos2(x + 34.0, sw.center().y),
                        Align2::LEFT_CENTER,
                        format!("{v:.2}{tag}"),
                        mono(12.0),
                        t.ink,
                    );
                }
                let lim_y = by + 10.0 * bh + 16.0;
                if r.limit > 0.0 {
                    let red = Color32::from_rgb(0xf0, 0x6a, 0x5c);
                    p.text(
                        pos2(x, lim_y),
                        Align2::LEFT_CENTER,
                        format!("— limit {:.2} (min_wall of t)", r.limit),
                        sans(11.0),
                        red,
                    );
                }
                let g = p.layout(r.model.clone(), sans(11.0), t.ink2, 230.0);
                p.galley(pos2(x, lim_y + 14.0), g, t.ink2);
                // the extremes and the probes, pinned where they are
                let on = |q: [f32; 3]| {
                    camera
                        .pixel_of(q, size.0, size.1)
                        .map(|px| view.min + vec2(px.0, px.1) / ppp)
                };
                if let Some(s) = on(r.max.1) {
                    flag(
                        &p,
                        s,
                        format!("MAX {:.2} mm", r.max.0),
                        c32(BANDS[0]),
                        Color32::WHITE,
                        vec2(-40.0, 48.0),
                    );
                }
                if let Some(s) = on(r.min.1) {
                    flag(
                        &p,
                        s,
                        format!("MIN {:.2} mm", r.min.0),
                        c32(BANDS[9]),
                        Color32::WHITE,
                        vec2(46.0, 40.0),
                    );
                }
                for (k, (q, v)) in probes.iter().enumerate() {
                    if let Some(s) = on(*q) {
                        flag(
                            &p,
                            s,
                            format!("Probe {} · {v:.2} mm", k + 1),
                            t.raised,
                            t.ink,
                            vec2(-30.0, -46.0),
                        );
                    }
                }
                // probing: the nearest point of the sheet under the pointer
                if probe
                    && let Some(at) = ctx.input(|i| i.pointer.hover_pos())
                    && view.contains(at)
                {
                    let near = r
                        .points
                        .iter()
                        .filter_map(|(q, v)| on(*q).map(|s| (s, *q, *v)))
                        .min_by(|a, b| (a.0 - at).length().total_cmp(&(b.0 - at).length()));
                    if let Some((s, q, v)) = near
                        && (s - at).length() < 24.0
                    {
                        flag(&p, s, format!("{v:.2} mm"), t.raised, t.ink, vec2(16.0, -30.0));
                        if ctx.input(|i| i.pointer.primary_clicked()) {
                            pin = Some((q, v as f64));
                        }
                    }
                }
                // the scrubber: play, the step, the stages
                let bar = Rect::from_min_max(
                    pos2(view.min.x + 16.0, view.max.y - 44.0),
                    pos2(view.max.x - 16.0, view.max.y - 12.0),
                );
                p.rect_filled(bar, 4.0, t.surface.gamma_multiply(0.92));
                let pb = Rect::from_min_size(bar.min + vec2(8.0, 4.0), vec2(34.0, 24.0));
                let resp = ui.interact(pb, ui.scope_id().with("thermo-play"), Sense::click());
                p.rect_stroke(pb, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                let c = pb.center();
                if playing {
                    for dx in [-4.0, 2.0] {
                        p.rect_filled(
                            Rect::from_min_size(c + vec2(dx, -6.0), vec2(3.0, 12.0)),
                            0.0,
                            t.ink,
                        );
                    }
                } else {
                    p.add(egui::Shape::convex_polygon(
                        vec![c + vec2(-4.0, -7.0), c + vec2(7.0, 0.0), c + vec2(-4.0, 7.0)],
                        t.ink,
                        Stroke::NONE,
                    ));
                }
                places.push(("thermo:play".to_string(), pb));
                if resp.clicked() {
                    play = true;
                }
                p.text(
                    pos2(pb.right() + 14.0, bar.center().y),
                    Align2::LEFT_CENTER,
                    "Step",
                    sans(12.0),
                    t.ink2,
                );
                let stages = stage_text(&r.stages);
                let sg = p.layout_no_wrap(format!("{} / {n} · {stages}", step + 1), mono(12.0), t.ink);
                let track = Rect::from_min_max(
                    pos2(pb.right() + 56.0, bar.center().y - 3.0),
                    pos2(bar.right() - sg.size().x - 24.0, bar.center().y + 3.0),
                );
                p.rect_filled(track, 3.0, t.line);
                let f = if n > 1 { step as f32 / (n - 1) as f32 } else { 1.0 };
                let knob = pos2(track.left() + track.width() * f, track.center().y);
                let blue = Color32::from_rgb(0x2f, 0x86, 0xd6);
                p.rect_filled(
                    Rect::from_min_max(track.min, pos2(knob.x, track.max.y)),
                    3.0,
                    blue,
                );
                p.circle_filled(knob, 8.0, blue);
                p.circle_stroke(knob, 8.0, Stroke::new(1.5, Color32::WHITE));
                let hit = track.expand2(vec2(8.0, 10.0));
                let drag = ui.interact(hit, ui.scope_id().with("thermo-scrub"), Sense::click_and_drag());
                places.push(("thermo:scrub".to_string(), hit));
                if (drag.dragged() || drag.clicked())
                    && let Some(at) = drag.interact_pointer_pos()
                {
                    let f = ((at.x - track.left()) / track.width()).clamp(0.0, 1.0);
                    scrub = Some((f * (n - 1) as f32).round() as usize);
                }
                p.galley(
                    pos2(
                        bar.right() - sg.size().x - 12.0,
                        bar.center().y - sg.size().y / 2.0,
                    ),
                    sg,
                    t.ink,
                );
            });
        let fg = self.fg.as_mut().expect("taken above");
        if play {
            fg.thermo.playing = !fg.thermo.playing;
        }
        if let Some(k) = scrub {
            fg.thermo.playing = false;
            fg.thermo.step = Some(k);
        }
        if let Some(q) = pin {
            fg.thermo.probes.push(q);
        }
        self.places.extend(places);
    }
}

/// The stages as step ranges: "drape 2–7 · vacuum 8–17 · creep 18–21".
fn stage_text(stages: &[&'static str]) -> String {
    let mut out: Vec<String> = vec![];
    let mut k = 0;
    while k < stages.len() {
        let s = stages[k];
        let start = k;
        while k < stages.len() && stages[k] == s {
            k += 1;
        }
        if s != "sheet" {
            out.push(format!("{s} {}–{}", start + 1, k));
        }
    }
    out.join(" · ")
}
