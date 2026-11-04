//! A thermoform's report in the part panel, and its wall-thickness map over the view as an
//! analysis tool shows one: a colour bar with its range in the view's corner, MIN and MAX pinned on
//! the part where they are.
use super::fg::FgPanel;
use super::theme::{mono, sans};
use super::{Shell, theme::Theme};
use egui::{Color32, Pos2, Rect, Stroke, Ui, vec2};

/// One thermoform as the panel shows it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ThermoRow {
    pub id: String,
    pub facts: Vec<(String, String)>,
    /// The thinnest and thickest wall, mm, and where (world).
    pub min: (f64, [f32; 3]),
    pub max: (f64, [f32; 3]),
    /// What the numbers are: the model and the law it read.
    pub model: String,
}

/// The map's colours, blue (thinnest) through cyan, green and yellow to red (thickest).
pub fn colour(f: f32) -> [f32; 3] {
    const STOPS: [[f32; 3]; 5] = [
        [0.10, 0.25, 0.95],
        [0.05, 0.80, 0.95],
        [0.15, 0.85, 0.20],
        [0.98, 0.88, 0.10],
        [0.95, 0.15, 0.10],
    ];
    let x = f.clamp(0.0, 1.0) * 4.0;
    let i = (x as usize).min(3);
    let u = x - i as f32;
    [0, 1, 2].map(|k| STOPS[i][k] + (STOPS[i + 1][k] - STOPS[i][k]) * u)
}

fn c32(c: [f32; 3]) -> Color32 {
    Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8)
}

/// The Convert to Thermoform form: what the body is, the mould, the sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct FormChoice {
    /// 0: a solid, its outside; 1: a thin wall; 2: a solid, its inside.
    pub from: usize,
    pub female: bool,
    pub t: f64,
    pub material: String,
}

impl Default for FormChoice {
    fn default() -> Self {
        FormChoice {
            from: 0,
            female: false,
            t: 2.0,
            material: "ABS".into(),
        }
    }
}

const FROM: [(&str, &str); 3] = [
    ("outer", "Solid: the part's outside"),
    ("wall", "Thin wall: the part as shelled"),
    ("inner", "Solid: the part's inside"),
];
const MATERIALS: [&str; 6] = ["ABS", "HIPS", "PETG", "PC", "PMMA", "HDPE"];

/// The statement that converts body `of`, named so as not to take a name the part has.
pub fn convert_text(fg: &FgPanel, of: &str) -> String {
    let taken = |n: &str| {
        fg.feats
            .iter()
            .any(|f| f.id == n || f.id.starts_with(&format!("{n}[")))
    };
    let name = (1..)
        .map(|k| {
            if k == 1 {
                "vacuum_part".to_string()
            } else {
                format!("vacuum_part{k}")
            }
        })
        .find(|n| !taken(n))
        .expect("a free name");
    let c = &fg.form;
    let (from, _) = FROM[c.from.min(2)];
    let t = if from == "wall" {
        String::new()
    } else {
        format!(", t: {}", c.t)
    };
    let tool = if c.female { ", tool: \"female\"" } else { "" };
    format!(
        "{name} = thermoform({of}, from: \"{from}\"{tool}{t}, material: \"{}\")\n",
        c.material
    )
}

/// The last feature that made or changed a solid: what Convert to Thermoform takes.
fn last_solid(fg: &FgPanel) -> Option<String> {
    fg.feats
        .iter()
        .rev()
        .find(|f| {
            !matches!(
                f.kind.as_str(),
                "sketch" | "plane" | "axis" | "point" | "param" | "thermoform"
            )
        })
        .map(|f| f.id.clone())
}

impl Shell {
    pub(super) fn fg_thermos(
        ui: &mut Ui,
        t: &Theme,
        fg: &mut FgPanel,
        out: &mut Vec<super::fg::FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        if fg.thermos.is_empty() {
            if fg.assembly || !fg.sheets.is_empty() {
                return;
            }
            let Some(of) = last_solid(fg) else { return };
            Self::fg_heading(ui, t, "Thermoform");
            egui::ComboBox::from_id_salt("thermo-from")
                .selected_text(FROM[fg.form.from.min(2)].1)
                .show_ui(ui, |ui| {
                    for (k, (_, label)) in FROM.iter().enumerate() {
                        ui.selectable_value(&mut fg.form.from, k, *label);
                    }
                });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut fg.form.female, false, "Male mould");
                ui.selectable_value(&mut fg.form.female, true, "Female mould");
            });
            ui.horizontal(|ui| {
                if fg.form.from != 1 {
                    ui.label(egui::RichText::new("Sheet").font(sans(12.0)).color(t.ink2));
                    ui.add(
                        egui::DragValue::new(&mut fg.form.t)
                            .range(0.1..=20.0)
                            .speed(0.05)
                            .suffix(" mm"),
                    );
                }
                egui::ComboBox::from_id_salt("thermo-material")
                    .selected_text(fg.form.material.clone())
                    .show_ui(ui, |ui| {
                        for m in MATERIALS {
                            ui.selectable_value(&mut fg.form.material, m.to_string(), m);
                        }
                    });
            });
            ui.label(
                egui::RichText::new(format!("the part: {of}"))
                    .font(sans(11.0))
                    .color(t.ink2),
            );
            if Self::fg_button(ui, "Convert to Thermoform", "fg:thermoform".into(), places) {
                out.push(super::fg::FgAct::Apply(forge_lang::error::Op::AddFeature {
                    before: None,
                    text: convert_text(fg, &of),
                }));
            }
            return;
        }
        Self::fg_heading(ui, t, "Thermoform");
        let label = if fg.walls {
            "Hide wall thickness"
        } else {
            "Show wall thickness"
        };
        if Self::fg_button(ui, label, "fg:walls".into(), places) {
            fg.walls = !fg.walls;
        }
        for r in &fg.thermos {
            ui.label(egui::RichText::new(&r.id).font(sans(12.0)).color(t.ink));
            if r.max.0 > 0.0 {
                ui.label(
                    egui::RichText::new(format!("  wall {:.2} to {:.2} mm", r.min.0, r.max.0))
                        .font(mono(11.0))
                        .color(t.ink),
                );
            }
            for (k, v) in &r.facts {
                ui.label(
                    egui::RichText::new(format!("  {k}: {v}"))
                        .font(mono(11.0))
                        .color(t.ink2),
                );
            }
            if !r.model.is_empty() {
                ui.label(
                    egui::RichText::new(format!("  {}", r.model))
                        .font(sans(10.0))
                        .color(t.ink2),
                );
            }
        }
    }

    /// The map's legend in the view's corner and its extremes pinned on the part.
    pub(super) fn thermo_probes(&mut self, ctx: &egui::Context) {
        let Some(fg) = &self.fg else { return };
        if !fg.walls || fg.thermos.is_empty() {
            return;
        }
        let Some((camera, size)) = self.camera else { return };
        let (t, view, ppp) = (self.theme, self.view_rect, ctx.pixels_per_point());
        let rows = fg.thermos.clone();
        egui::Area::new(egui::Id::unique("thermo-probes"))
            .fixed_pos(view.min)
            .order(egui::Order::Middle)
            .interactable(false)
            .constrain(false)
            .show(ctx, |ui| {
                let p = ui.painter();
                let (lo, hi) = rows.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| {
                    (a.min(r.min.0), b.max(r.max.0))
                });
                // the colour bar: thickest at the top, as the analysis tools stack it
                let (x, y0, w, h) = (view.min.x + 14.0, view.min.y + 76.0, 16.0, 180.0);
                p.text(
                    Pos2::new(x, y0 - 18.0),
                    egui::Align2::LEFT_BOTTOM,
                    "Wall thickness (mm)",
                    sans(12.0),
                    t.ink,
                );
                let n = 24;
                for k in 0..n {
                    let (a, b) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
                    let r = Rect::from_min_max(Pos2::new(x, y0 + h * a), Pos2::new(x + w, y0 + h * b));
                    p.rect_filled(r, 0.0, c32(colour(1.0 - (a + b) / 2.0)));
                }
                p.rect_stroke(
                    Rect::from_min_size(Pos2::new(x, y0), vec2(w, h)),
                    0.0,
                    Stroke::new(1.0, t.ink2),
                    egui::StrokeKind::Outside,
                );
                for k in 0..=5 {
                    let f = k as f64 / 5.0;
                    let v = hi - (hi - lo) * f;
                    let label = match k {
                        0 => format!("{v:.3} Max"),
                        5 => format!("{v:.3} Min"),
                        _ => format!("{v:.3}"),
                    };
                    p.text(
                        Pos2::new(x + w + 6.0, y0 + h * f as f32),
                        egui::Align2::LEFT_CENTER,
                        label,
                        mono(11.0),
                        t.ink,
                    );
                }
                // the extremes, pinned where they are
                for r in &rows {
                    for (tag, (v, at)) in [("MIN", r.min), ("MAX", r.max)] {
                        let Some(px) = camera.pixel_of(at, size.0, size.1) else {
                            continue;
                        };
                        let s = view.min + vec2(px.0, px.1) / ppp;
                        let f = if hi > lo {
                            ((v - lo) / (hi - lo)) as f32
                        } else {
                            0.5
                        };
                        let text = format!("{tag} {v:.3} mm");
                        let tip = s + vec2(14.0, -18.0);
                        p.line_segment([s, tip], Stroke::new(1.5, t.ink));
                        p.circle_filled(s, 3.5, c32(colour(f)));
                        p.circle_stroke(s, 3.5, Stroke::new(1.0, t.ink));
                        let g = p.layout_no_wrap(text, mono(11.0), t.ink);
                        let r = Rect::from_min_size(tip + vec2(0.0, -g.size().y), g.size()).expand(3.0);
                        p.rect_filled(r, 3.0, t.raised);
                        p.rect_stroke(r, 3.0, Stroke::new(1.0, c32(colour(f))), egui::StrokeKind::Inside);
                        p.galley(r.min + vec2(3.0, 3.0), g, t.ink);
                    }
                }
            });
    }
}
