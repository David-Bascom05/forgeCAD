//! The bend simulator's panel (kernel/forge-brake, `query F bendsim`): the bend order and each
//! step's tooling and checks, the press section drawn with the part through the step's motion,
//! play, step and scrub, and the failing checks' tools and places in red. Self-contained: its
//! own colours until the shell's design tokens cover it.

use forge_brake as fb;
use forge_lang::bendsim::{Ask, Sim, simulate};

/// Colours of its own.
pub mod look {
    use egui::Color32;
    pub const PART: Color32 = Color32::from_rgb(0x4A, 0x9C, 0xE8);
    pub const TOOL: Color32 = Color32::from_rgb(0x8A, 0x8F, 0x98);
    pub const GAUGE: Color32 = Color32::from_rgb(0xB0, 0x8A, 0x3A);
    pub const FAIL: Color32 = Color32::from_rgb(0xE0, 0x40, 0x40);
    pub const PASS: Color32 = Color32::from_rgb(0x3A, 0xA8, 0x5A);
    pub const WARN: Color32 = Color32::from_rgb(0xE8, 0xA0, 0x30);
}

/// What the panel holds: the simulation, the step shown, where in its motion, and play.
pub struct BendSimUi {
    pub sim: Result<Sim, String>,
    pub step: usize,
    pub t: f64,
    pub playing: bool,
    /// Tooling asked for (a set id or a file beside the part).
    pub tooling: String,
}

impl BendSimUi {
    /// Simulates the part's text, its files beside `path`.
    pub fn open(source: &str, path: &str, tooling: &str) -> BendSimUi {
        let dir = std::path::Path::new(path)
            .parent()
            .map(std::path::Path::to_path_buf);
        let ask = Ask {
            tooling: Some(tooling.to_string()),
            ..Ask::default()
        };
        let sim = simulate(source, dir, None, &ask).map_err(|a| a.text);
        BendSimUi {
            sim,
            step: 0,
            t: 0.0,
            playing: false,
            tooling: tooling.into(),
        }
    }

    /// The bends done before the step shown.
    pub fn done(&self) -> Vec<bool> {
        let Ok(s) = &self.sim else { return vec![] };
        let mut d = vec![false; s.model.bends.len()];
        for st in s.plan.steps.iter().take(self.step) {
            d[st.bend] = true;
        }
        d
    }

    /// The picture now.
    pub fn frame(&self) -> Option<fb::view::Frame> {
        let s = self.sim.as_ref().ok()?;
        let st = s.plan.steps.get(self.step)?;
        Some(fb::view::frame(&s.model, &s.tooling, &self.done(), st, self.t))
    }

    /// Moves play on by `dt` seconds (a step takes three); past a step's end, on to the next.
    pub fn tick(&mut self, dt: f64) {
        if !self.playing {
            return;
        }
        let n = self.sim.as_ref().map_or(0, |s| s.plan.steps.len());
        self.t += dt / 3.0;
        if self.t > 1.0 {
            if self.step + 1 < n {
                self.step += 1;
                self.t = 0.0;
            } else {
                self.t = 1.0;
                self.playing = false;
            }
        }
    }

    /// The tool names and the place of each failing check of the step shown.
    pub fn failures(&self) -> Vec<(Option<String>, Option<[f64; 3]>)> {
        let Ok(s) = &self.sim else { return vec![] };
        let Some(st) = s.plan.steps.get(self.step) else {
            return vec![];
        };
        st.checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| (c.tool.clone(), c.at))
            .collect()
    }
}

/// The section's points fitted to `rect`, z up; the same scale both ways.
pub fn fit(outlines: &[fb::view::Outline], rect: egui::Rect) -> impl Fn([f64; 2]) -> egui::Pos2 {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    // fit to the part and the tools near it, not the beams' far ends
    for o in outlines
        .iter()
        .filter(|o| o.what == "part" || o.what == "punch" || o.what == "die")
    {
        for p in &o.points {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    if !lo[0].is_finite() {
        lo = [-1.0; 2];
        hi = [1.0; 2];
    }
    let pad = 0.06 * (hi[0] - lo[0]).max(hi[1] - lo[1]);
    let (w, h) = (hi[0] - lo[0] + 2.0 * pad, hi[1] - lo[1] + 2.0 * pad);
    let s = (rect.width() as f64 / w).min(rect.height() as f64 / h);
    let cx = 0.5 * (lo[0] + hi[0]);
    let cz = 0.5 * (lo[1] + hi[1]);
    let c = rect.center();
    move |p: [f64; 2]| egui::pos2(c.x + ((p[0] - cx) * s) as f32, c.y - ((p[1] - cz) * s) as f32)
}

/// Draws the section into `rect`: tools, part, failing tools and places in red.
pub fn paint(ui: &egui::Ui, rect: egui::Rect, ui_state: &BendSimUi) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(24));
    let Some(f) = ui_state.frame() else { return };
    let to = fit(&f.outlines, rect);
    let fails = ui_state.failures();
    let failing = |name: &str| fails.iter().any(|(t, _)| t.as_deref() == Some(name));
    for o in &f.outlines {
        let colour = match o.what {
            "part" => look::PART,
            "finger" => look::GAUGE,
            _ if failing(&o.name) => look::FAIL,
            _ => look::TOOL,
        };
        let pts: Vec<egui::Pos2> = o.points.iter().map(|p| to(*p)).collect();
        if o.what == "part" {
            // the pieces' outlines on both skins, as lines
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.2, colour)));
        } else {
            painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(1.5, colour)));
        }
    }
    for (_, at) in &fails {
        if let Some(a) = at {
            let p = to([a[0], a[2]]);
            painter.circle_stroke(p, 6.0, egui::Stroke::new(2.0, look::FAIL));
        }
    }
    let label = format!(
        "{}  turn {:.1}°  tip {:.1} mm",
        f.phase.word(),
        f.phi.to_degrees(),
        f.tip
    );
    painter.text(
        rect.left_top() + egui::vec2(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(11.0),
        egui::Color32::from_gray(200),
    );
}

/// The panel's body: the order, the steps, play and scrub, the section, the step's checks.
pub fn body(ui: &mut egui::Ui, st: &mut BendSimUi) {
    let sim = match &st.sim {
        Ok(s) => s,
        Err(e) => {
            ui.colored_label(look::FAIL, e);
            return;
        }
    };
    let plan = &sim.plan;
    let head = if plan.feasible {
        "Feasible"
    } else {
        "No feasible sequence"
    };
    ui.colored_label(
        if plan.feasible { look::PASS } else { look::FAIL },
        format!("{head} on {}", sim.tooling.id),
    );
    let n = plan.steps.len();
    ui.horizontal(|ui| {
        if ui.button("⏮").clicked() {
            st.step = 0;
            st.t = 0.0;
        }
        if ui.button("◀").clicked() && st.step > 0 {
            st.step -= 1;
            st.t = 0.0;
        }
        if ui.button(if st.playing { "⏸" } else { "▶" }).clicked() {
            st.playing = !st.playing;
            if st.playing && st.t >= 1.0 && st.step + 1 >= n {
                st.step = 0;
                st.t = 0.0;
            }
        }
        if ui.button("▶|").clicked() && st.step + 1 < n {
            st.step += 1;
            st.t = 0.0;
        }
        ui.add(egui::Slider::new(&mut st.t, 0.0..=1.0).show_value(false));
    });
    let names = &sim.names;
    egui::ScrollArea::horizontal()
        .id_salt("bendsim-steps")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, s) in plan.steps.iter().enumerate() {
                    let c = if s.pass() { look::PASS } else { look::FAIL };
                    let txt = egui::RichText::new(format!("{}. {}", i + 1, names[s.bend])).color(c);
                    if ui.selectable_label(i == st.step, txt).clicked() {
                        st.step = i;
                        st.t = 0.0;
                    }
                }
            });
        });
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 300.0), egui::Sense::hover());
    paint(ui, rect, st);
    if let Some(s) = plan.steps.get(st.step) {
        ui.label(format!(
            "{}: {:.1}° (+{:.1}°)  {} on {}{}{}",
            names[s.bend],
            s.angle.to_degrees(),
            s.overbend.to_degrees(),
            s.punch,
            s.die,
            if s.flip { ", flipped" } else { "" },
            if s.setup.rotate { ", end for end" } else { "" }
        ));
        if let Some(g) = &s.gauge {
            ui.label(format!(
                "backgauge x {:.2} r {:.2}, {} finger(s)",
                g.x,
                g.r,
                g.fingers.len()
            ));
        }
        egui::Grid::new("bendsim-checks").striped(true).show(ui, |ui| {
            for c in &s.checks {
                let col = if c.pass {
                    look::PASS
                } else if c.severity == fb::Severity::Warning {
                    look::WARN
                } else {
                    look::FAIL
                };
                ui.colored_label(col, c.rule.word());
                let ang = matches!(c.rule, fb::Rule::DieAngle | fb::Rule::PunchAngle);
                let v = |x: f64| {
                    if ang {
                        format!("{:.2}°", x.to_degrees())
                    } else {
                        format!("{x:.3}")
                    }
                };
                ui.label(format!(
                    "{} {} {}",
                    v(c.value),
                    if c.at_most { "≤" } else { "≥" },
                    v(c.limit)
                ));
                ui.label(c.tool.clone().unwrap_or_default());
                ui.end_row();
            }
        });
    }
    for b in &plan.blocking {
        ui.colored_label(
            look::FAIL,
            format!(
                "{} blocked: {}",
                names[b.bend],
                b.checks
                    .iter()
                    .map(|c| c.rule.word())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
        sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (120, 60))\n}\n\
        base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
        up = flange(base.edge(plan.outline[1]), height: 30)\n";

    #[test]
    fn plays_through_its_steps() {
        let mut ui = BendSimUi::open(L, "l.fg", "std-eu");
        assert!(ui.sim.is_ok());
        let a = ui.frame().unwrap();
        ui.playing = true;
        ui.tick(1.5);
        let b = ui.frame().unwrap();
        assert!(b.tip < a.tip);
        ui.tick(10.0);
        assert!(!ui.playing && ui.t == 1.0);
        let bad = BendSimUi::open(L, "l.fg", "nope");
        assert!(bad.sim.is_err());
    }
}
