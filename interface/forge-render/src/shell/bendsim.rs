//! The Bend Simulation environment's state and panels (kernel/forge-brake, `query F bendsim`):
//! the press and tooling from the library (real makers' records, every value cited) or a
//! stand-in set, the bend order (the planner's, or edited), and the simulation through every
//! step's stages — the view draws it in 3D (gui_bendsim.rs); these panels choose, order, scrub
//! and report.

use forge_brake as fb;
use forge_brake::catalog::{Catalog, Status};
use forge_brake::scene::{STAGES, Stage, stage_at};
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
    pub const MUTED: Color32 = Color32::from_rgb(0x90, 0x94, 0x9A);
}

/// Seconds a step takes when played.
pub const STEP_SECONDS: f64 = 4.0;

/// What is simulated: a library press (its tools all, or those named) or a stand-in set, and
/// the order (None: the planner's).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Choice {
    pub press: Option<String>,
    pub punches: Option<Vec<String>>,
    pub dies: Option<Vec<String>>,
    /// Bend names in order.
    pub sequence: Option<Vec<String>>,
    /// Keep the last step's tools where they pass (a ranked program applied).
    pub keep_tools: bool,
}

/// What the environment holds: the simulation, the step shown, where in its motion, and play.
pub struct BendSimUi {
    pub sim: Result<Sim, String>,
    pub step: usize,
    pub t: f64,
    pub playing: bool,
    /// A stand-in set's id, used when no press is chosen.
    pub tooling: String,
    pub choice: Choice,
    /// The sheet's material (its look), and the press's table height above the floor.
    pub material: Option<String>,
    pub table_height: Option<f64>,
    /// The part's text and file it was simulated from.
    pub source: String,
    pub path: String,
    /// The library as last read (built in, the part's folder's, the user's).
    pub catalog: Option<Result<Catalog, String>>,
    /// The planner found no feasible order: the part's own order is shown, as given.
    pub as_given: bool,
    /// The step and instant last asked to be drawn.
    pub painted: Option<(usize, u64)>,
}

fn dir_of(path: &str) -> Option<std::path::PathBuf> {
    std::path::Path::new(path)
        .parent()
        .map(std::path::Path::to_path_buf)
}

impl BendSimUi {
    /// Simulates the part's text on a stand-in set (its files beside `path`).
    pub fn open(source: &str, path: &str, tooling: &str) -> BendSimUi {
        BendSimUi::open_with(source, path, tooling, Choice::default())
    }

    /// Simulates the part's text as `choice` says.
    pub fn open_with(source: &str, path: &str, tooling: &str, choice: Choice) -> BendSimUi {
        let dir = dir_of(path);
        let ask = Ask {
            tooling: choice.press.is_none().then(|| tooling.to_string()),
            press: choice.press.clone(),
            punches: choice.punches.clone(),
            dies: choice.dies.clone(),
            sequence: choice.sequence.as_ref().map(|s| s.join(",")),
            keep_tools: choice.keep_tools,
            // the ranked alternatives, offered beside the order shown (never applied unasked)
            programs: Some(5),
            ..Ask::default()
        };
        let mut sim = simulate(source, dir.clone(), None, &ask).map_err(|a| a.text);
        // no feasible order: the bends in the part's own order, simulated as given, so what
        // blocks them is seen in the view
        let mut as_given = false;
        if let Ok(s) = &sim
            && !s.plan.feasible
            && s.plan.steps.is_empty()
            && choice.sequence.is_none()
            && !s.names.is_empty()
        {
            let again = Ask {
                sequence: Some(s.names.join(",")),
                programs: None,
                ..ask.clone()
            };
            if let Ok(s2) = simulate(source, dir.clone(), None, &again) {
                sim = Ok(s2);
                as_given = true;
            }
        }
        let material = sim.as_ref().ok().and_then(|s| {
            s.answer
                .json
                .get("material")
                .and_then(|m| m.str().map(str::to_string))
        });
        let mut b = BendSimUi {
            sim,
            step: 0,
            t: 0.0,
            playing: false,
            tooling: tooling.into(),
            choice,
            material,
            table_height: None,
            source: source.into(),
            path: path.into(),
            catalog: None,
            as_given,
            painted: None,
        };
        if let Some(p) = b.choice.press.clone() {
            b.table_height = b.catalog().and_then(|c| c.press(&p)).and_then(|r| r.table_height);
        }
        b
    }

    /// The library, read once.
    pub fn catalog(&mut self) -> Option<&Catalog> {
        if self.catalog.is_none() {
            let dir = dir_of(&self.path);
            self.catalog = Some(forge_lang::bendsim::catalog(dir.as_deref()).map_err(|a| a.text));
        }
        self.catalog.as_ref().and_then(|c| c.as_ref().ok())
    }

    /// Simulated again with a changed choice; the step kept where it still is one.
    pub fn redo(&mut self, choice: Choice, tooling: &str) {
        let cat = self.catalog.take();
        let mut fresh = BendSimUi::open_with(&self.source.clone(), &self.path.clone(), tooling, choice);
        if fresh.catalog.is_none() {
            fresh.catalog = cat;
        }
        fresh.resume(self);
        *self = fresh;
    }

    /// The step and instant of an earlier simulation, where it still has that step.
    pub fn resume(&mut self, was: &BendSimUi) {
        let n = self.steps();
        if was.step < n {
            self.step = was.step;
            self.t = was.t;
        }
    }

    pub fn steps(&self) -> usize {
        self.sim.as_ref().map_or(0, |s| s.plan.steps.len())
    }

    /// To step `k`, at its start.
    pub fn go(&mut self, k: usize) {
        self.step = k;
        self.t = 0.0;
    }

    /// Play or pause; from the start again once played through.
    pub fn play(&mut self) {
        let n = self.steps();
        self.playing = !self.playing;
        if self.playing && self.t >= 1.0 && self.step + 1 >= n {
            self.go(0);
        }
    }

    /// The whole run's position: steps done plus the instant in the step.
    pub fn position(&self) -> f64 {
        self.step as f64 + self.t
    }

    pub fn set_position(&mut self, x: f64) {
        let n = self.steps();
        if n == 0 {
            return;
        }
        let x = x.clamp(0.0, n as f64);
        let k = (x.floor() as usize).min(n - 1);
        self.step = k;
        self.t = (x - k as f64).clamp(0.0, 1.0);
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

    /// The instant shown: its stage, the bend's turn, the punch tip.
    pub fn pose(&self) -> Option<fb::scene::Pose> {
        let s = self.sim.as_ref().ok()?;
        if self.step >= s.plan.steps.len() {
            return None;
        }
        Some(fb::scene::Scene::new(&s.model, &s.tooling, &s.plan).pose(self.step, self.t))
    }

    /// Moves play on by `dt` seconds; past a step's end, on to the next.
    pub fn tick(&mut self, dt: f64) {
        if !self.playing {
            return;
        }
        let n = self.steps();
        self.t += dt / STEP_SECONDS;
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
        st.failures().iter().map(|c| (c.tool.clone(), c.at)).collect()
    }

    /// The machine's words: a library press's maker and model, or the stand-in set's.
    pub fn machine_words(&mut self) -> String {
        match self.choice.press.clone() {
            Some(p) => match self.catalog().and_then(|c| c.press(&p)) {
                Some(r) => format!("{} {}", r.prov.maker, r.model),
                None => p,
            },
            None => format!("{} (stand-in: forgecad's own numbers)", self.tooling),
        }
    }
}

fn status_mark(s: Option<Status>) -> (&'static str, egui::Color32) {
    match s {
        Some(Status::Published) => ("P", look::PASS),
        Some(Status::Estimate) => ("E", look::WARN),
        Some(Status::Conflict) => ("C", look::WARN),
        Some(Status::Unknown) | None => ("U", look::MUTED),
    }
}

/// One value with its provenance mark and the cite on hover.
fn cited(ui: &mut egui::Ui, label: &str, value: String, prov: &fb::catalog::Prov, path: &str) {
    let (m, c) = status_mark(prov.status(path));
    let r = ui.horizontal(|ui| {
        ui.colored_label(c, m);
        ui.label(format!("{label}: {value}"));
    });
    let cites = prov.of(path);
    if !cites.is_empty() {
        r.response.on_hover_ui(|ui| {
            for c in cites {
                ui.label(format!(
                    "{} — {}",
                    c.status.word(),
                    c.value.as_deref().unwrap_or("")
                ));
                if let Some(s) = &c.source {
                    ui.small(format!("{s} (read {})", c.taken.as_deref().unwrap_or("?")));
                }
                if let Some(b) = &c.basis {
                    ui.small(format!("basis: {b}"));
                }
            }
        });
    }
}

/// The Setup panel: the machine from the library (or a stand-in), its specs with their
/// provenance, and the tools it takes.
pub fn setup_ui(ui: &mut egui::Ui, st: &mut BendSimUi) {
    let mut redo: Option<(Choice, String)> = None;
    let words = st.machine_words();
    let tooling = st.tooling.clone();
    let choice = st.choice.clone();
    let Some(cat) = st.catalog().cloned() else {
        if let Some(Err(e)) = &st.catalog {
            ui.colored_label(look::FAIL, format!("library: {e}"));
        }
        return;
    };
    ui.strong("Machine");
    egui::ComboBox::from_id_salt("bendsim-press")
        .selected_text(words)
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            for id in fb::library::IDS {
                if ui
                    .selectable_label(
                        choice.press.is_none() && tooling == id,
                        format!("{id} (stand-in)"),
                    )
                    .clicked()
                {
                    redo = Some((
                        Choice {
                            press: None,
                            ..Default::default()
                        },
                        id.to_string(),
                    ));
                }
            }
            ui.separator();
            for r in &cat.presses {
                let txt = format!(
                    "{} {}{}{}",
                    r.prov.maker,
                    r.model,
                    r.press
                        .as_ref()
                        .ok()
                        .and_then(|p| p.max_force)
                        .map_or(String::new(), |f| format!(" · {:.0} kN", f)),
                    match &r.press {
                        Ok(p) => format!(" · {:.0} mm", p.bed_length),
                        Err(f) => format!(" · UNKNOWN {}", f.join(", ")),
                    }
                );
                let ok = r.press.is_ok();
                let resp = ui.add_enabled(
                    ok,
                    egui::Button::selectable(choice.press.as_deref() == Some(&r.id), txt),
                );
                if resp.clicked() {
                    redo = Some((
                        Choice {
                            press: Some(r.id.clone()),
                            ..Default::default()
                        },
                        tooling.clone(),
                    ));
                }
            }
        });
    if let Some(p) = choice.press.as_deref().and_then(|p| cat.press(p)) {
        egui::CollapsingHeader::new("Specs (P published, E estimate, C conflict, U unknown)")
            .id_salt("bendsim-specs")
            .show(ui, |ui| {
                let pv = &p.prov;
                if let Ok(pr) = &p.press {
                    cited(
                        ui,
                        "force",
                        pr.max_force.map_or("UNKNOWN".into(), |f| format!("{f:.0} kN")),
                        pv,
                        "force",
                    );
                    cited(
                        ui,
                        "bending length",
                        format!("{:.0} mm", pr.bed_length),
                        pv,
                        "bed_length",
                    );
                    cited(
                        ui,
                        "between housings",
                        format!("{:.0} mm", pr.housing_gap),
                        pv,
                        "housing_gap",
                    );
                    cited(
                        ui,
                        "open height",
                        format!("{:.0} mm", pr.open_height),
                        pv,
                        "open_height",
                    );
                    cited(ui, "stroke", format!("{:.0} mm", pr.stroke), pv, "stroke");
                    cited(
                        ui,
                        "throat",
                        pr.frame
                            .map_or("UNKNOWN".into(), |f| format!("{:.0} mm", f.throat)),
                        pv,
                        "throat",
                    );
                    if !pr.unchecked.is_empty() {
                        ui.colored_label(look::WARN, format!("not checked: {}", pr.unchecked.join(", ")));
                    }
                }
                for c in pv.of("record") {
                    ui.small(c.value.clone().unwrap_or_default());
                }
            });
        // the tools of its clamping system
        let fits = |c: &str| p.clamp == "unknown" || c == p.clamp;
        egui::CollapsingHeader::new("Tooling")
            .id_salt("bendsim-tools")
            .default_open(false)
            .show(ui, |ui| {
                let mut ch = choice.clone();
                let mut changed = false;
                for (kind, list) in [("Punches", true), ("Dies", false)] {
                    ui.strong(kind);
                    let rows: Vec<(String, String, Option<String>)> = if list {
                        cat.punches
                            .iter()
                            .filter(|r| fits(&r.clamp))
                            .map(|r| {
                                (
                                    r.id.clone(),
                                    r.prov.name.clone(),
                                    r.punch.as_ref().err().map(|f| f.join(", ")),
                                )
                            })
                            .collect()
                    } else {
                        cat.dies
                            .iter()
                            .filter(|r| fits(&r.clamp))
                            .map(|r| {
                                (
                                    r.id.clone(),
                                    r.prov.name.clone(),
                                    r.die.as_ref().err().map(|f| f.join(", ")),
                                )
                            })
                            .collect()
                    };
                    let sel = if list { &mut ch.punches } else { &mut ch.dies };
                    for (id, name, unknown) in rows {
                        let defaults: Vec<&str> = cat.default_tools(p, list);
                        let mut on = unknown.is_none()
                            && match sel.as_ref() {
                                Some(s) => s.contains(&id),
                                None => defaults.contains(&id.as_str()),
                            };
                        let label = match &unknown {
                            Some(f) => format!("{id} {name} — UNKNOWN: {f}"),
                            None => format!("{id} {name}"),
                        };
                        if ui
                            .add_enabled(unknown.is_none(), egui::Checkbox::new(&mut on, label))
                            .changed()
                        {
                            let all: Vec<String> = defaults.iter().map(|x| x.to_string()).collect();
                            let mut s = sel.clone().unwrap_or(all);
                            if on {
                                s.push(id.clone());
                            } else {
                                s.retain(|x| x != &id);
                            }
                            *sel = Some(s);
                            changed = true;
                        }
                    }
                }
                if changed {
                    redo = Some((ch, tooling.clone()));
                }
            });
    }
    if let Some((c, t)) = redo {
        st.redo(c, &t);
    }
}

/// The Sequence panel: the order simulated (the planner's or edited), moved with the arrows.
pub fn sequence_ui(ui: &mut egui::Ui, st: &mut BendSimUi) {
    let Ok(sim) = &st.sim else { return };
    let names: Vec<String> = sim.plan.steps.iter().map(|s| sim.names[s.bend].clone()).collect();
    let passes: Vec<bool> = sim.plan.steps.iter().map(fb::Step::pass).collect();
    let progs: Vec<(Vec<String>, [usize; 4])> = sim
        .programs
        .iter()
        .map(|p| {
            (
                p.plan.sequence.iter().map(|b| sim.names[*b].clone()).collect(),
                [p.flips, p.turns, p.setups, p.changes],
            )
        })
        .collect();
    let mut order = names.clone();
    let mut moved = None;
    let mut go = None;
    ui.horizontal(|ui| {
        ui.strong("Sequence");
        ui.label(if st.as_given {
            "(no feasible order: the part's own, as given)"
        } else if st.choice.sequence.is_some() {
            "(edited)"
        } else {
            "(planned)"
        });
    });
    for (i, n) in names.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.small_button("up").clicked() && i > 0 {
                moved = Some((i, i - 1));
            }
            if ui.small_button("dn").clicked() && i + 1 < names.len() {
                moved = Some((i, i + 1));
            }
            let c = if passes[i] { look::PASS } else { look::FAIL };
            if ui
                .selectable_label(
                    st.step == i,
                    egui::RichText::new(format!("{}. {n}", i + 1)).color(c),
                )
                .clicked()
            {
                go = Some(i);
            }
        });
    }
    if let Some(i) = go {
        st.go(i);
    }
    let planned = st.choice.sequence.is_some() && ui.button("Plan the order").clicked();
    // the planner's ranked programs: offered, applied only by a click
    let mut apply: Option<Vec<String>> = None;
    if !progs.is_empty() {
        egui::CollapsingHeader::new(format!("Suggested programs ({})", progs.len()))
            .id_salt("bendsim-programs")
            .default_open(false)
            .show(ui, |ui| {
                ui.small("ranked: fewest flips and turns, then stations, then tool changes");
                for (i, (order, k)) in progs.iter().enumerate() {
                    let shown = *order == names;
                    ui.horizontal(|ui| {
                        let txt = format!(
                            "{}. {} · flips {} · turns {} · stations {} · changes {}",
                            i + 1,
                            order.join(" > "),
                            k[0],
                            k[1],
                            k[2],
                            k[3]
                        );
                        if shown {
                            ui.colored_label(look::PASS, txt);
                        } else {
                            ui.label(txt);
                            if ui.small_button("use").clicked() {
                                apply = Some(order.clone());
                            }
                        }
                    });
                }
            });
    }
    if let Some(order) = apply {
        let mut c = st.choice.clone();
        c.sequence = Some(order);
        c.keep_tools = true;
        let t = st.tooling.clone();
        st.redo(c, &t);
    } else if let Some((a, b)) = moved {
        order.swap(a, b);
        let mut c = st.choice.clone();
        c.sequence = Some(order);
        let t = st.tooling.clone();
        st.redo(c, &t);
    } else if planned {
        let mut c = st.choice.clone();
        c.sequence = None;
        let t = st.tooling.clone();
        st.redo(c, &t);
    }
}

/// The step's results: tooling, gauge, force, springback and every check. Returns the bend's
/// name when its "flat pattern" link is clicked.
pub fn results_ui(ui: &mut egui::Ui, st: &BendSimUi) -> Option<String> {
    let mut flat = None;
    let sim = match &st.sim {
        Ok(s) => s,
        Err(e) => {
            ui.colored_label(look::FAIL, e);
            return None;
        }
    };
    let plan = &sim.plan;
    ui.colored_label(
        if plan.feasible { look::PASS } else { look::FAIL },
        if plan.feasible {
            "Feasible"
        } else {
            "No feasible sequence"
        },
    );
    let names = &sim.names;
    if let Some(s) = plan.steps.get(st.step) {
        ui.label(format!(
            "{}: {:.1}° turn, overbend {:.1}° (springs back to {:.1}°)",
            names[s.bend],
            s.angle.to_degrees(),
            s.overbend.to_degrees(),
            s.angle.to_degrees()
        ));
        ui.label(format!(
            "punch {} · die {}{}{}",
            s.punch,
            s.die,
            if s.flip { " · flipped" } else { "" },
            if s.setup.rotate { " · end for end" } else { "" }
        ));
        if ui.link("this bend on the flat pattern").clicked() {
            flat = Some(names[s.bend].clone());
        }
        if let (Some(pl), Some(dl)) = (s.punch_length, s.die_length) {
            ui.label(format!("stacks: punch {pl:.0} mm, die {dl:.0} mm"));
        }
        if let Some(g) = &s.gauge {
            ui.label(format!(
                "backgauge X {:.1} R {:.1} · {} finger(s)",
                g.x,
                g.r,
                g.fingers.len()
            ));
        }
        ui.label(format!(
            "ram travel {:.1} mm{}",
            s.travel,
            s.force.map_or(String::new(), |f| format!(" · force {f:.1} kN"))
        ));
        egui::Grid::new("bendsim-checks").striped(true).show(ui, |ui| {
            for c in &s.checks {
                let (col, verdict) = if c.pass {
                    (look::PASS, "pass")
                } else if c.unchecked.is_some() {
                    (look::MUTED, "not checked")
                } else if c.severity == fb::Severity::Warning {
                    (look::WARN, "warning")
                } else if c.undecided {
                    (look::FAIL, "undecided")
                } else {
                    (look::FAIL, "fail")
                };
                ui.colored_label(col, c.rule.word());
                ui.colored_label(col, verdict);
                let ang = matches!(c.rule, fb::Rule::DieAngle | fb::Rule::PunchAngle);
                let v = |x: f64| {
                    if ang {
                        format!("{:.2}°", x.to_degrees())
                    } else {
                        format!("{x:.2}")
                    }
                };
                match &c.unchecked {
                    Some(u) => ui.label(format!("unknown: {u}")),
                    None => ui.label(format!(
                        "{} {} {}",
                        v(c.value),
                        if c.at_most { "≤" } else { "≥" },
                        v(c.limit)
                    )),
                };
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
    flat
}

/// The timeline: every step a block, its stages marked; drag to scrub.
pub fn timeline_ui(ui: &mut egui::Ui, st: &mut BendSimUi) {
    let n = st.steps();
    ui.horizontal(|ui| {
        if ui.button("|<").clicked() {
            st.go(0);
        }
        if ui.button("<").clicked() {
            st.go(st.step.saturating_sub(1));
        }
        if ui.button(if st.playing { "Pause" } else { "Play" }).clicked() {
            st.play();
        }
        if ui.button(">").clicked() && n > 0 {
            st.go((st.step + 1).min(n - 1));
        }
        let (stage, _) = stage_at(st.t);
        let pose = st.pose();
        ui.label(format!(
            "step {}/{} · {}{}",
            st.step + 1,
            n,
            stage.word(),
            pose.map_or(String::new(), |p| format!(" · phi {:.1}°", p.phi.to_degrees()))
        ));
    });
    let w = ui.available_width().max(100.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 34.0), egui::Sense::click_and_drag());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0, egui::Color32::from_gray(30));
    if n == 0 {
        return;
    }
    let bw = rect.width() / n as f32;
    let fails: Vec<bool> = st
        .sim
        .as_ref()
        .map(|s| s.plan.steps.iter().map(|x| !x.pass()).collect())
        .unwrap_or_default();
    for k in 0..n {
        let x0 = rect.left() + bw * k as f32;
        for (s, a, b) in STAGES {
            let shade = match s {
                Stage::Handle => 52,
                Stage::Gauge => 62,
                Stage::Approach => 72,
                Stage::Bend => 96,
                Stage::Springback => 84,
                Stage::Return => 66,
            };
            let r = egui::Rect::from_min_max(
                egui::pos2(x0 + bw * a as f32, rect.top() + 4.0),
                egui::pos2(x0 + bw * b as f32, rect.bottom() - 4.0),
            );
            p.rect_filled(r, 0.0, egui::Color32::from_gray(shade));
        }
        let c = if fails.get(k).copied().unwrap_or(false) {
            look::FAIL
        } else {
            look::PASS
        };
        p.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x0, rect.bottom() - 4.0),
                egui::pos2(x0 + bw, rect.bottom()),
            ),
            0.0,
            c,
        );
        p.line_segment(
            [egui::pos2(x0, rect.top()), egui::pos2(x0, rect.bottom())],
            egui::Stroke::new(1.0, egui::Color32::from_gray(15)),
        );
    }
    let x = rect.left() + (st.position() / n as f64) as f32 * rect.width();
    p.line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(2.0, egui::Color32::WHITE),
    );
    if (resp.dragged() || resp.clicked())
        && let Some(pt) = resp.interact_pointer_pos()
    {
        let f = ((pt.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
        st.set_position(f * n as f64);
        st.playing = false;
    }
}

/// Words over the view: the machine, the step and stage, and what fails.
pub fn overlay(ui: &egui::Ui, rect: egui::Rect, st: &mut BendSimUi) {
    let p = ui.painter_at(rect);
    let words = st.machine_words();
    let Ok(sim) = &st.sim else {
        if let Err(e) = &st.sim {
            p.text(
                rect.left_top() + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                e,
                egui::FontId::proportional(14.0),
                look::FAIL,
            );
        }
        return;
    };
    let mut lines = vec![(words, egui::Color32::from_gray(225))];
    if let Some(s) = sim.plan.steps.get(st.step) {
        let (stage, _) = stage_at(st.t);
        lines.push((
            format!(
                "{}. {} · {} · {} on {}",
                st.step + 1,
                sim.names[s.bend],
                stage.word(),
                s.punch,
                s.die
            ),
            egui::Color32::from_gray(200),
        ));
        if stage == Stage::Handle {
            let h = s.checks.iter().find(|c| c.rule == fb::Rule::Handling);
            lines.push(match h {
                Some(c) if c.pass => (
                    format!(
                        "operator handling: lift, pull out, turn, push in, set down; proven clear ({:.2} mm)",
                        c.value
                    ),
                    look::PASS,
                ),
                Some(c) => (
                    format!(
                        "operator handling: not proven clear{}",
                        c.tool.as_deref().map_or(String::new(), |t| format!(" of {t}"))
                    ),
                    look::WARN,
                ),
                None => ("operator handling: the first step, none".into(), look::MUTED),
            });
        }
        for c in s.failures() {
            lines.push((
                format!(
                    "x {}{}{}",
                    c.rule.word(),
                    c.tool.as_deref().map_or(String::new(), |t| format!(" · {t}")),
                    c.phi
                        .map_or(String::new(), |f| format!(" at phi {:.1}°", f.to_degrees()))
                ),
                look::FAIL,
            ));
        }
    }
    let mut y = rect.top() + 44.0;
    for (l, c) in lines {
        p.text(
            egui::pos2(rect.left() + 12.0, y),
            egui::Align2::LEFT_TOP,
            l,
            egui::FontId::proportional(13.0),
            c,
        );
        y += 18.0;
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
        ui.playing = true;
        ui.tick(0.3 * STEP_SECONDS);
        let a = ui.pose().unwrap();
        ui.tick(0.3 * STEP_SECONDS);
        let b = ui.pose().unwrap();
        assert!(b.tip < a.tip, "{a:?} {b:?}");
        ui.tick(10.0 * STEP_SECONDS);
        assert!(!ui.playing && ui.t == 1.0);
        let bad = BendSimUi::open(L, "l.fg", "nope");
        assert!(bad.sim.is_err());
    }

    #[test]
    fn a_library_press_simulates() {
        let c = Choice {
            press: Some("trumpf-trubend-5130".into()),
            ..Default::default()
        };
        let ui = BendSimUi::open_with(L, "l.fg", "std-eu", c);
        let sim = ui.sim.as_ref().expect("simulated");
        assert_eq!(sim.tooling.press.id, "trumpf-trubend-5130");
        assert!(sim.plan.feasible, "{:?}", sim.plan.blocking);
        assert!(ui.table_height.is_some());
    }
}
