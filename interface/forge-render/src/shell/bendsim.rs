//! The Bend Simulation environment's state and panels (kernel/forge-brake, `query F bendsim`):
//! the press and tooling from the library (real makers' records, every value cited) or a
//! stand-in set, the bend order (the planner's, or edited), and the simulation through every
//! step's stages — the view draws it in 3D (gui_bendsim.rs); these panels choose, order, scrub
//! and report.

use super::bendsim_cards as cards;
use super::theme::Theme;
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

/// The panel's pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    /// The machine, its tools and the stations on its bed.
    Setup,
    /// The bends in order, each pictured, and the step's results.
    Program,
}

/// How the view frames the press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// The part, the punch and the die on its station, the bed's near end in view.
    Work,
    /// The whole press: frame, beams, bed and backgauge.
    Overview,
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
    /// Frames still to draw after a change of instant, for the view to settle.
    pub settle: u8,
    /// The part's bending document (`<stem>_bend.fg`), when it has one: the program's source of
    /// truth, every edit an op on its text.
    pub doc: Option<DocState>,
    /// A place the view is asked to look at (a failure located), taken by the view.
    pub locate: Option<[f64; 3]>,
    /// A framing asked for (the ribbon's View tools), taken by the view.
    pub frame: Option<Framing>,
    /// The panel's page shown.
    pub pane: Pane,
    /// The Axes switch as it was on entry (put back on Finish).
    pub axes_was: Option<bool>,
    /// Follow bend: each new step framed afresh (off: the eye is the user's).
    pub follow: bool,
    /// See-through: what stands between the eye and the bend being made drawn as a ghost, at
    /// `opacity`; with `see_press`, the press's frame too.
    pub see_through: bool,
    pub opacity: f32,
    pub see_press: bool,
    /// What is seen through, as points: their outline drawn over the view.
    pub outlines: Vec<Vec<[f32; 3]>>,
    /// The tool library shows only what fits the press and the part.
    pub fit_only: bool,
    /// The planner's ranked programs asked for (the ribbon's Suggest Order): their list opened.
    pub suggest: bool,
    /// The step's checks all shown, not only failures and those not checked.
    pub all_checks: bool,
    /// Each step's picture, for the simulation they were drawn from (its address).
    pictures: Option<(usize, Vec<cards::Picture>)>,
}

/// The bending document as last read.
#[derive(Clone, Debug, PartialEq)]
pub struct DocState {
    pub path: std::path::PathBuf,
    pub src: String,
    /// Its `bend` statements' names, in order (one per step), and its stations' names.
    pub bends: Vec<String>,
    pub stations: Vec<String>,
    /// Its own refusals (a statement it does not take, a program format, ..).
    pub errors: Vec<(String, String)>,
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

    /// Simulates the part's text: as its bending document states, if it has one, else as
    /// `choice` says.
    pub fn open_with(source: &str, path: &str, tooling: &str, choice: Choice) -> BendSimUi {
        let companion = forge_bend_lang::companion(std::path::Path::new(path));
        if let Some(src) = forge_dirs::fs::read_to_string(&companion)
            .ok()
            .filter(|s| forge_bend_lang::is_bending(s))
        {
            return BendSimUi::from_doc(source, path, tooling, companion, src);
        }
        BendSimUi::plan_with(source, path, tooling, choice)
    }

    /// Simulates the part's text as `choice` says (the planner's where it says nothing).
    pub fn plan_with(source: &str, path: &str, tooling: &str, choice: Choice) -> BendSimUi {
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
            doc: None,
            locate: None,
            frame: None,
            settle: 0,
            pane: Pane::Program,
            axes_was: None,
            follow: false,
            see_through: true,
            opacity: 0.35,
            see_press: false,
            outlines: Vec::new(),
            fit_only: true,
            all_checks: false,
            suggest: false,
            pictures: None,
        };
        if let Some(p) = b.choice.press.clone() {
            b.table_height = b.catalog().and_then(|c| c.press(&p)).and_then(|r| r.table_height);
        }
        b
    }

    /// The part simulated as its bending document states.
    fn from_doc(source: &str, path: &str, tooling: &str, file: std::path::PathBuf, src: String) -> BendSimUi {
        let dir = dir_of(path);
        let run = forge_bend_lang::run_with(&src, source, dir.as_deref());
        let d = run.doc.clone();
        let sim = run.sim.ok_or_else(|| {
            run.errors
                .iter()
                .map(|(a, e)| format!("{a}: {e}"))
                .collect::<Vec<_>>()
                .join("; ")
        });
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
            tooling: d.tooling.clone().unwrap_or_else(|| tooling.to_string()),
            choice: Choice {
                press: d.press.clone(),
                ..Choice::default()
            },
            material,
            table_height: None,
            source: source.into(),
            path: path.into(),
            catalog: None,
            as_given: false,
            painted: None,
            doc: Some(DocState {
                path: file,
                src,
                bends: d.bends.iter().map(|b| b.id.clone()).collect(),
                stations: d.stations.iter().map(|s| s.id.clone()).collect(),
                errors: run.errors,
            }),
            locate: None,
            frame: None,
            settle: 0,
            pane: Pane::Program,
            axes_was: None,
            follow: false,
            see_through: true,
            opacity: 0.35,
            see_press: false,
            outlines: Vec::new(),
            fit_only: true,
            all_checks: false,
            suggest: false,
            pictures: None,
        };
        if let Some(p) = b.choice.press.clone() {
            b.table_height = b.catalog().and_then(|c| c.press(&p)).and_then(|r| r.table_height);
        }
        b
    }

    /// The bending document's path for this part.
    pub fn companion(&self) -> std::path::PathBuf {
        forge_bend_lang::companion(std::path::Path::new(&self.path))
    }

    /// The program shown written as the part's bending document (its first edit, or a whole
    /// program applied): the file's undo log keeps what was there.
    pub fn write_doc(&mut self) -> Result<(), String> {
        let sim = self.sim.as_ref().map_err(Clone::clone)?;
        let part = std::path::Path::new(&self.path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = forge_bend_lang::document(&part, None, self.choice.press.as_deref(), &self.tooling, sim);
        self.commit_doc(text)
    }

    /// One edit op on the document (written first, from the program shown, if there is none).
    pub fn doc_op(&mut self, op: &forge_lang::error::Op) -> Result<(), String> {
        if self.doc.is_none() {
            self.write_doc()?;
        }
        let src = self.doc.as_ref().map(|d| d.src.clone()).unwrap_or_default();
        let after = forge_lang::edit::apply(&src, op).map_err(|e| format!("{e:?}"))?;
        self.commit_doc(after)
    }

    fn commit_doc(&mut self, after: String) -> Result<(), String> {
        let file = self.companion();
        let before = forge_dirs::fs::read_to_string(&file).unwrap_or_default();
        forge_lang::undo::write(&file, &before, &after).map_err(|e| format!("{}: {e}", file.display()))?;
        self.reload();
        Ok(())
    }

    /// Undo or redo the document's last edit.
    pub fn doc_step(&mut self, undo: bool) -> Result<(), String> {
        let step = if undo {
            forge_lang::undo::Step::Undo
        } else {
            forge_lang::undo::Step::Redo
        };
        forge_lang::undo::step(&self.companion(), step)?;
        self.reload();
        Ok(())
    }

    /// Read again from the document (or the choice, if the document is gone), step kept.
    fn reload(&mut self) {
        let cat = self.catalog.take();
        let (source, path, tooling, choice) = (
            self.source.clone(),
            self.path.clone(),
            self.tooling.clone(),
            self.choice.clone(),
        );
        let mut fresh = BendSimUi::open_with(&source, &path, &tooling, choice);
        if fresh.catalog.is_none() {
            fresh.catalog = cat;
        }
        fresh.resume(self);
        *self = fresh;
    }

    /// The library, read once.
    pub fn catalog(&mut self) -> Option<&Catalog> {
        if self.catalog.is_none() {
            let dir = dir_of(&self.path);
            self.catalog = Some(forge_lang::bendsim::catalog(dir.as_deref()).map_err(|a| a.text));
        }
        self.catalog.as_ref().and_then(|c| c.as_ref().ok())
    }

    /// Simulated again with a changed choice; the step kept where it still is one. With a
    /// bending document, the choice planned and written to it (one undo step).
    pub fn redo(&mut self, choice: Choice, tooling: &str) {
        if self.doc.is_some() {
            let cat = self.catalog.take();
            let mut planned = BendSimUi::plan_with(&self.source.clone(), &self.path.clone(), tooling, choice);
            planned.catalog = cat;
            planned.resume(self);
            if let Err(e) = planned.write_doc() {
                planned.sim = Err(e);
            }
            *self = planned;
            return;
        }
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
        Some(Status::User) => ("Y", egui::Color32::from_rgb(0xA0, 0x80, 0xE0)),
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

/// A section's heading, as the other environments' panels have them.
pub fn head(ui: &mut egui::Ui, t: &Theme, text: &str) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(text)
            .font(super::theme::semi(13.0))
            .color(t.ink),
    );
    ui.add_space(2.0);
}

/// The Setup panel: the machine from the library (or a stand-in) by its maker's name, its specs
/// with their provenance, and its tools as a library of cards.
pub fn setup_ui(ui: &mut egui::Ui, t: &Theme, st: &mut BendSimUi) {
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
    head(ui, t, "Machine");
    let shown_id = choice.press.clone().unwrap_or_else(|| tooling.clone());
    egui::ComboBox::from_id_salt("bendsim-press")
        .selected_text(words)
        .width(ui.available_width() - 4.0)
        .show_ui(ui, |ui| {
            ui.label(egui::RichText::new("Press brakes").small().weak());
            for r in &cat.presses {
                let txt = match &r.press {
                    Ok(p) => format!(
                        "{} · {} · {:.0} mm",
                        cards::press_title(r),
                        p.max_force
                            .map_or("force UNKNOWN".into(), |f| format!("{:.0} kN", f)),
                        p.bed_length
                    ),
                    Err(f) => format!("{} · UNKNOWN {}", cards::press_title(r), f.join(", ")),
                };
                let resp = ui
                    .add_enabled(
                        r.press.is_ok(),
                        egui::Button::selectable(choice.press.as_deref() == Some(&r.id), txt),
                    )
                    .on_hover_text(&r.id);
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
            ui.separator();
            ui.label(
                egui::RichText::new("Stand-in tooling (not a real maker's)")
                    .small()
                    .weak(),
            );
            for id in fb::library::IDS {
                let word = match id {
                    "std-eu" => "Stand-in European set",
                    "std-us" => "Stand-in American set",
                    _ => id,
                };
                if ui
                    .selectable_label(choice.press.is_none() && tooling == id, word)
                    .on_hover_text(id)
                    .clicked()
                {
                    redo = Some((Choice::default(), id.to_string()));
                }
            }
        })
        .response
        .on_hover_text(shown_id);
    if let Some(p) = choice.press.as_deref().and_then(|p| cat.press(p)) {
        egui::CollapsingHeader::new("Specs")
            .id_salt("bendsim-specs")
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "P published · E estimate · C conflict · U unknown · Y yours; the source on hover",
                    )
                    .small()
                    .color(t.ink2),
                );
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
        layout_ui(ui, t, st);
        let needs = st.sim.as_ref().ok().and_then(cards::needs);
        // the open height only where it is checked: unknown clamp heights leave it open
        let open = p
            .press
            .as_ref()
            .ok()
            .filter(|x| !x.unchecked.iter().any(|u| u == "open_height"))
            .map(|x| x.open_height);
        let mut fit_only = st.fit_only;
        if let Some(c) = tool_library(ui, t, &cat, p, &choice, needs, open, &mut fit_only) {
            redo = Some((c, tooling.clone()));
        }
        st.fit_only = fit_only;
    }
    if let Some((c, t)) = redo {
        st.redo(c, &t);
    }
}

/// A card's row: id, title, values, load, drawn section, geometry known, fits.
type Row = (
    String,
    String,
    String,
    String,
    Option<fb::scene::Mesh>,
    bool,
    bool,
);

/// The press's tools as cards (its clamping system's): each with its drawn cross-section, its
/// maker's name, angle, radius, height and rated load; a click puts it in the set the planner
/// chooses from or takes it out. Filtered, unless asked, to what fits: the clamping system, a
/// known geometry, the part's sharpest angle and smallest radius (the rules' own sides), and
/// punch and die together under the open height where it is checked. Returns a new choice.
#[allow(clippy::too_many_arguments)]
fn tool_library(
    ui: &mut egui::Ui,
    t: &Theme,
    cat: &Catalog,
    p: &fb::catalog::PressRec,
    choice: &Choice,
    needs: Option<(f64, f64)>,
    open: Option<f64>,
    fit_only: &mut bool,
) -> Option<Choice> {
    head(ui, t, "Tooling");
    ui.checkbox(fit_only, "Only tools that fit this press and part")
        .on_hover_text("clamping system, known geometry, tip angle and radius against the sharpest bend, punch and die height against the open height");
    let fits = |c: &str| p.clamp == "unknown" || c == p.clamp;
    let shortest_die = cat
        .dies
        .iter()
        .filter(|r| fits(&r.clamp))
        .filter_map(|r| r.die.as_ref().ok().map(|d| d.height))
        .fold(f64::INFINITY, f64::min);
    let mut out = None;
    for punches in [true, false] {
        let defaults: Vec<&str> = cat.default_tools(p, punches);
        let sel = if punches { &choice.punches } else { &choice.dies };
        let chosen = |id: &str| match sel {
            Some(s) => s.iter().any(|x| x == id),
            None => defaults.contains(&id),
        };
        // (id, title, values, load, mesh, usable, fits)
        let mut rows: Vec<Row> = Vec::new();
        if punches {
            for r in cat.punches.iter().filter(|r| fits(&r.clamp)) {
                let row = match &r.punch {
                    Ok(x) => {
                        let ok = needs
                            .is_none_or(|(inc, rad)| x.tip_angle <= inc + 1e-9 && x.tip_radius <= rad + 1e-9)
                            && open.is_none_or(|o| !shortest_die.is_finite() || x.height + shortest_die < o);
                        (
                            format!(
                                "{:.0}° · R{:.1} · h {:.0} mm",
                                x.tip_angle.to_degrees(),
                                x.tip_radius,
                                x.height
                            ),
                            x.max_load
                                .map_or("load not published".into(), |l| format!("{l:.0} kN/m")),
                            Some(cards::punch_mesh(x)),
                            true,
                            ok,
                        )
                    }
                    Err(f) => (
                        format!("UNKNOWN: {}", f.join(", ")),
                        String::new(),
                        None,
                        false,
                        false,
                    ),
                };
                rows.push((
                    r.id.clone(),
                    cards::punch_title(r),
                    row.0,
                    row.1,
                    row.2,
                    row.3,
                    row.4,
                ));
            }
        } else {
            for r in cat.dies.iter().filter(|r| fits(&r.clamp)) {
                let row = match &r.die {
                    Ok(x) => {
                        let ok = needs.is_none_or(|(inc, _)| x.angle <= inc + 1e-9);
                        (
                            format!(
                                "V{:.0} · {:.0}° · h {:.0} mm",
                                x.v,
                                x.angle.to_degrees(),
                                x.height
                            ),
                            x.max_load
                                .map_or("load not published".into(), |l| format!("{l:.0} kN/m")),
                            Some(cards::die_mesh(x)),
                            true,
                            ok,
                        )
                    }
                    Err(f) => (
                        format!("UNKNOWN: {}", f.join(", ")),
                        String::new(),
                        None,
                        false,
                        false,
                    ),
                };
                rows.push((
                    r.id.clone(),
                    cards::die_title(r),
                    row.0,
                    row.1,
                    row.2,
                    row.3,
                    row.4,
                ));
            }
        }
        let total = rows.len();
        let on = rows.iter().filter(|r| r.5 && chosen(&r.0)).count();
        let shown: Vec<_> = rows
            .into_iter()
            .filter(|r| !*fit_only || (r.6 || chosen(&r.0)))
            .collect();
        let word = if punches { "Punches" } else { "Dies" };
        let mut click: Option<(String, bool)> = None;
        let mut cards_of = |ui: &mut egui::Ui, which: &[&Row]| {
            // the panel's visible width: wider content would be cut off at its edge
            let w = (ui.clip_rect().right() - ui.cursor().left() - 6.0)
                .min(ui.available_width())
                .max(140.0);
            for (id, title, values, load, mesh, usable, _) in which.iter().copied() {
                let is_on = *usable && chosen(id);
                let r = cards::card(ui, t, id, title, values, load, mesh.as_ref(), is_on, w);
                if *usable && r.clicked() {
                    click = Some((id.clone(), is_on));
                }
            }
        };
        let (used, rest): (Vec<&Row>, Vec<&Row>) = shown.iter().partition(|r| r.5 && chosen(&r.0));
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("{word} in use · {on}"))
                .font(super::theme::semi(11.5))
                .color(t.ink),
        )
        .id_salt(("bendsim-used", punches))
        .default_open(true)
        .show(ui, |ui| {
            if used.is_empty() {
                ui.label(
                    egui::RichText::new("none: the planner has nothing to choose from")
                        .small()
                        .color(look::WARN),
                );
            }
            cards_of(ui, &used)
        });
        egui::CollapsingHeader::new(format!(
            "Add from the library · {} {} of {total}",
            rest.len(),
            if *fit_only { "that fit" } else { "more" }
        ))
        .id_salt(("bendsim-cards", punches))
        .default_open(false)
        .show(ui, |ui| cards_of(ui, &rest));
        if let Some((id, was_on)) = click {
            let all: Vec<String> = defaults.iter().map(|x| x.to_string()).collect();
            let mut s = sel.clone().unwrap_or(all);
            if was_on {
                s.retain(|x| x != &id);
            } else {
                s.push(id);
            }
            let mut c = choice.clone();
            if punches {
                c.punches = Some(s);
            } else {
                c.dies = Some(s);
            }
            out = Some(c);
        }
    }
    out
}

/// The tooling layout along the bed (with a bending document): the bed drawn to scale with its
/// housings, each station's punch above its die as their stacks (each segment, a gap between),
/// dragged along the bed to place it; the exact values folded away; each station's stack load
/// capacity against the most force a step at it takes. Every change is a SetArg on the
/// document's `station` statement.
pub fn layout_ui(ui: &mut egui::Ui, t: &Theme, st: &mut BendSimUi) {
    use forge_lang::error::Op;
    let (Some(doc), Ok(sim)) = (&st.doc, &st.sim) else {
        return;
    };
    let tl = &sim.tooling;
    if tl.stations.is_empty() {
        return;
    }
    let cat = st.catalog.as_ref().and_then(|c| c.as_ref().ok());
    let names = doc.stations.clone();
    let mut op: Option<Op> = None;
    head(ui, t, "Stations on the bed");
    ui.label(
        egui::RichText::new("drag a station along the bed; it snaps to 5 mm")
            .small()
            .color(t.ink2),
    );
    let pr = &tl.press;
    let bed = pr.bed_length;
    let (w, h) = (ui.available_width().max(120.0), 92.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click_and_drag());
    let p = ui.painter_at(rect);
    let x_of = |y: f64| rect.left() + ((y + 0.5 * bed) / bed) as f32 * rect.width();
    let y_of = |x: f32| f64::from((x - rect.left()) / rect.width()) * bed - 0.5 * bed;
    p.rect_filled(rect, 4.0, t.field);
    // the beam (top) and the bed (bottom), the housings at either end
    let mid = rect.center().y;
    let beam = egui::Rect::from_min_max(
        rect.left_top() + egui::vec2(0.0, 4.0),
        egui::pos2(rect.right(), rect.top() + 12.0),
    );
    let table = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - 12.0),
        rect.right_bottom() - egui::vec2(0.0, 4.0),
    );
    p.rect_filled(beam, 2.0, t.line);
    p.rect_filled(table, 2.0, t.line);
    let g = 0.5 * pr.housing_gap;
    for x in [x_of(-g), x_of(g)] {
        p.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.5, look::WARN),
        );
    }
    let dragging = ui
        .ctx()
        .memory(|m| m.data.get_temp::<usize>(egui::Id::unique("bendsim-drag")));
    let live = resp
        .interact_pointer_pos()
        .map(|q| (y_of(q.x) / 5.0).round() * 5.0);
    let mut hover_k = None;
    for (k, s) in tl.stations.iter().enumerate() {
        let y = match (dragging, live) {
            (Some(d), Some(y)) if d == k => y,
            _ => s.y,
        };
        let active = dragging == Some(k);
        let col = if active { look::GAUGE } else { look::TOOL };
        let (punch, die) = (&tl.punches[s.punch], &tl.dies[s.die]);
        // the punch stack hangs from the beam, the die stack stands on the bed
        for (len, segs, top, bot) in [
            (s.punch_length, &punch.segments, beam.bottom() + 2.0, mid - 3.0),
            (s.die_length, &die.segments, mid + 3.0, table.top() - 2.0),
        ] {
            for (a, b) in fb::scene::stack_spans(segs, y - 0.5 * len, y + 0.5 * len) {
                let r =
                    egui::Rect::from_min_max(egui::pos2(x_of(a) + 0.5, top), egui::pos2(x_of(b) - 0.5, bot));
                p.rect_filled(r, 1.0, col);
            }
        }
        let whole = egui::Rect::from_min_max(
            egui::pos2(x_of(y - 0.5 * s.punch_length.max(s.die_length)), rect.top()),
            egui::pos2(x_of(y + 0.5 * s.punch_length.max(s.die_length)), rect.bottom()),
        );
        if resp.hover_pos().is_some_and(|q| whole.contains(q)) {
            hover_k = Some(k);
        }
        p.text(
            egui::pos2(whole.center().x, mid),
            egui::Align2::CENTER_CENTER,
            names.get(k).cloned().unwrap_or_default(),
            super::theme::semi(11.0),
            t.ink,
        );
        if active {
            p.text(
                egui::pos2(whole.center().x, rect.bottom() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{y:.0} mm"),
                egui::FontId::proportional(10.0),
                t.ink,
            );
        }
    }
    if let Some(k) = hover_k
        && dragging.is_none()
    {
        let s = &tl.stations[k];
        let (pi, di) = (&tl.punches[s.punch].id, &tl.dies[s.die].id);
        resp.clone().on_hover_text(format!(
            "{}\n{}\n{}\nat {:.0} mm · punch {:.0} mm · die {:.0} mm",
            names.get(k).cloned().unwrap_or_default(),
            cards::tool_title(cat, pi, true),
            cards::tool_title(cat, di, false),
            s.y,
            s.punch_length,
            s.die_length
        ));
    }
    // drag a station along the bed: written when let go
    if resp.drag_started()
        && let Some(pt) = resp.interact_pointer_pos()
    {
        let y = y_of(pt.x);
        let hit = tl
            .stations
            .iter()
            .position(|s| (s.y - y).abs() <= 0.5 * s.punch_length.max(s.die_length));
        ui.ctx().memory_mut(|m| match hit {
            Some(k) => {
                m.data.insert_temp(egui::Id::unique("bendsim-drag"), k);
            }
            None => m.data.remove::<usize>(egui::Id::unique("bendsim-drag")),
        });
    }
    if resp.drag_stopped()
        && let (Some(k), Some(y)) = (dragging, live)
    {
        ui.ctx()
            .memory_mut(|m| m.data.remove::<usize>(egui::Id::unique("bendsim-drag")));
        if let Some(n) = names.get(k)
            && (y - tl.stations[k].y).abs() > 1e-9
        {
            op = Some(Op::SetArg {
                id: n.clone(),
                arg: "y".into(),
                text: format!("{y}"),
            });
        }
    }
    // per station: its tools by name, its load; the exact values folded away
    for (k, s) in tl.stations.iter().enumerate() {
        let Some(n) = names.get(k) else { continue };
        let (punch, die) = (&tl.punches[s.punch], &tl.dies[s.die]);
        ui.label(egui::RichText::new(n).font(super::theme::semi(11.5)).color(t.ink));
        ui.label(
            egui::RichText::new(cards::tool_title(cat, &punch.id, true))
                .small()
                .color(t.ink2),
        )
        .on_hover_text(&punch.id);
        ui.label(
            egui::RichText::new(cards::tool_title(cat, &die.id, false))
                .small()
                .color(t.ink2),
        )
        .on_hover_text(&die.id);
        // the stack's capacity: the least rated load per metre of its punch and die times its
        // length, against the most force a step at it takes (with the material's strength)
        let cap = punch
            .max_load
            .map(|l| l * s.punch_length / 1000.0)
            .into_iter()
            .chain(die.max_load.map(|l| l * s.die_length / 1000.0))
            .fold(f64::INFINITY, f64::min);
        let most = sim
            .plan
            .steps
            .iter()
            .filter(|x| (x.setup.punch, x.setup.die) == (s.punch, s.die))
            .filter_map(|x| x.force)
            .fold(None, |m: Option<f64>, f| Some(m.map_or(f, |m| m.max(f))));
        let line = match (cap.is_finite(), most) {
            (true, Some(f)) => (
                format!("load {f:.1} kN of {cap:.1} kN the stack takes"),
                if f <= cap { look::PASS } else { look::FAIL },
            ),
            (true, None) => (
                format!("stack takes {cap:.1} kN (force not known: no material strength)"),
                look::MUTED,
            ),
            (false, _) => (
                "stack load: not checked (tools' ratings unknown)".into(),
                look::MUTED,
            ),
        };
        ui.label(egui::RichText::new(line.0).small().color(line.1));
        egui::CollapsingHeader::new(egui::RichText::new("exact values").small())
            .id_salt(("bendsim-station-values", k))
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (arg, word, v0) in [
                        ("y", "at", s.y),
                        ("punch_length", "punch", s.punch_length),
                        ("die_length", "die", s.die_length),
                    ] {
                        let mut v = v0;
                        ui.label(word);
                        let r = ui.add(egui::DragValue::new(&mut v).speed(1.0).suffix(" mm"));
                        if (r.lost_focus() || r.drag_stopped()) && (v - v0).abs() > 1e-9 {
                            op = Some(Op::SetArg {
                                id: n.clone(),
                                arg: arg.into(),
                                text: format!("{}", (v * 10.0).round() / 10.0),
                            });
                        }
                    }
                });
            });
    }
    if let Some(op) = op
        && let Err(e) = st.doc_op(&op)
    {
        st.sim = Err(e);
    }
}

/// The Sequence panel: the program's steps, edited in place. With the part's bending document
/// every edit is an op on its text (reorder: MoveFeature; orientation and station: SetArg),
/// with the document's undo and redo; without one, the first edit writes it from the program
/// shown. The planner's ranked programs are offered beside it, applied only by a click.
pub fn sequence_ui(ui: &mut egui::Ui, t: &Theme, st: &mut BendSimUi) {
    use forge_lang::error::Op;
    let Ok(sim) = &st.sim else {
        if let Some(d) = &st.doc {
            ui.colored_label(look::FAIL, format!("{}:", d.path.display()));
            for (a, e) in &d.errors {
                ui.colored_label(look::FAIL, format!("{a}: {e}"));
            }
        }
        doc_buttons(ui, st);
        return;
    };
    let names: Vec<String> = sim.plan.steps.iter().map(|s| sim.names[s.bend].clone()).collect();
    let passes: Vec<bool> = sim.plan.steps.iter().map(fb::Step::pass).collect();
    let rotated: Vec<bool> = sim.plan.steps.iter().map(|s| s.setup.rotate).collect();
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
    // the station each step works at (its punch and die), and the stations there are
    let stations: Vec<String> = st.doc.as_ref().map(|d| d.stations.clone()).unwrap_or_default();
    let at: Vec<Option<usize>> = sim
        .plan
        .steps
        .iter()
        .map(|s| {
            sim.tooling
                .stations
                .iter()
                .position(|x| (x.punch, x.die) == (s.setup.punch, s.setup.die))
        })
        .collect();
    let ids: Vec<String> = match &st.doc {
        Some(d) => d.bends.clone(),
        // as the document written from this program will name them
        None => (1..=names.len()).map(|k| format!("b{k}")).collect(),
    };
    let mut op: Option<Op> = None;
    let mut go = None;
    {
        ui.label(
            egui::RichText::new("Program")
                .font(super::theme::semi(13.0))
                .color(t.ink),
        );
        let note = match (&st.doc, st.as_given) {
            (Some(d), _) => format!(
                "({})",
                d.path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            (None, true) => "(no feasible order: the part's own, as given)".into(),
            (None, false) if st.choice.sequence.is_some() => "(edited, not yet written)".into(),
            _ => "(planned)".into(),
        };
        ui.add(egui::Label::new(egui::RichText::new(note).small().color(t.ink2)).wrap());
    }
    // each step's picture, drawn once for this simulation
    let key = sim as *const Sim as usize;
    if st.pictures.as_ref().is_none_or(|p| p.0 != key) {
        st.pictures = Some((key, cards::step_pictures(sim)));
    }
    let pics = &st.pictures.as_ref().unwrap().1;
    let mut moved: Option<(usize, usize)> = None;
    let row_w = ui.available_width();
    for (i, n) in names.iter().enumerate() {
        let row = ui.horizontal(|ui| {
            // the picture is the handle: drag it onto another row to move the bend there
            let pic = ui
                .dnd_drag_source(egui::Id::unique(("bendsim-row", i)), i, |ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(54.0, 44.0), egui::Sense::hover());
                    let p = ui.painter_at(r);
                    p.rect_filled(r, 4.0, if st.step == i { t.selection } else { t.field });
                    let items: Vec<(&fb::scene::Mesh, egui::Color32)> = pics
                        .get(i)
                        .map(|v| v.iter().map(|(m, c)| (m, *c)).collect())
                        .unwrap_or_default();
                    // scaled to the part, the tools about it
                    cards::paint_end_on_around(&p, r, &items, 1, 0.15);
                })
                .response
                .on_hover_text("drag to reorder");
            if pic.clicked() {
                go = Some(i);
            }
            ui.vertical(|ui| {
                let c = if passes[i] { look::PASS } else { look::FAIL };
                if ui
                    .selectable_label(
                        st.step == i,
                        egui::RichText::new(format!("{}. {n}", i + 1))
                            .font(super::theme::semi(12.0))
                            .color(c),
                    )
                    .on_hover_text(&ids[i])
                    .clicked()
                {
                    go = Some(i);
                }
                ui.horizontal(|ui| {
                    let mut r = rotated[i];
                    if ui.checkbox(&mut r, "Turn end for end").changed() {
                        op = Some(Op::SetArg {
                            id: ids[i].clone(),
                            arg: "rotate".into(),
                            text: if r { "true".into() } else { "false".into() },
                        });
                    }
                    if !stations.is_empty() {
                        let shown = at[i]
                            .and_then(|k| stations.get(k))
                            .cloned()
                            .unwrap_or_else(|| "planner".into());
                        egui::ComboBox::from_id_salt(("bendsim-station", i))
                            .selected_text(shown)
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for s in &stations {
                                    if ui.selectable_label(false, s).clicked() {
                                        op = Some(Op::SetArg {
                                            id: ids[i].clone(),
                                            arg: "at".into(),
                                            text: s.clone(),
                                        });
                                    }
                                }
                            });
                    }
                });
            });
        });
        // a row takes a dragged one: it goes where this one is
        let zone = ui.interact(
            egui::Rect::from_min_size(
                row.response.rect.min,
                egui::vec2(row_w, row.response.rect.height()),
            ),
            egui::Id::unique(("bendsim-drop", i)),
            egui::Sense::hover(),
        );
        if let Some(from) = zone.dnd_hover_payload::<usize>()
            && *from != i
        {
            let y = if *from > i {
                zone.rect.top()
            } else {
                zone.rect.bottom()
            };
            ui.painter()
                .hline(zone.rect.x_range(), y, egui::Stroke::new(2.0, t.accent));
        }
        if let Some(from) = zone.dnd_release_payload::<usize>()
            && *from != i
        {
            moved = Some((*from, i));
        }
    }
    if let Some((from, to)) = moved {
        op = Some(Op::MoveFeature {
            id: ids[from].clone(),
            before: if from > to {
                Some(ids[to].clone())
            } else {
                ids.get(to + 1).cloned()
            },
        });
    }
    if let Some(i) = go {
        st.go(i);
    }
    doc_buttons(ui, st);
    // the planner's ranked programs: offered, applied only by a click
    let mut apply: Option<Vec<String>> = None;
    if !progs.is_empty() {
        let open = std::mem::take(&mut st.suggest).then_some(true);
        egui::CollapsingHeader::new(format!("Suggested programs ({})", progs.len()))
            .id_salt("bendsim-programs")
            .default_open(false)
            .open(open)
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
    } else if let Some(op) = op
        && let Err(e) = st.doc_op(&op)
    {
        st.sim = Err(e);
    }
}

/// The document's own buttons: write it, undo and redo its edits, plan it afresh.
fn doc_buttons(ui: &mut egui::Ui, st: &mut BendSimUi) {
    ui.horizontal(|ui| {
        if st.doc.is_none() {
            if ui.button("Write program").clicked()
                && let Err(e) = st.write_doc()
            {
                st.sim = Err(e);
            }
        } else {
            if ui.button("Undo").clicked() {
                let _ = st.doc_step(true);
            }
            if ui.button("Redo").clicked() {
                let _ = st.doc_step(false);
            }
            if ui.button("Plan again").clicked() {
                let c = Choice {
                    press: st.choice.press.clone(),
                    ..Choice::default()
                };
                let t = st.tooling.clone();
                st.redo(c, &t);
            }
        }
    });
}

/// A row of the property sheet: its label, its value (with its unit), its key on hover.
fn prop(ui: &mut egui::Ui, t: &Theme, label: &str, value: String, key: Option<&str>) {
    let r = ui.label(egui::RichText::new(label).color(t.ink2));
    if let Some(k) = key {
        r.on_hover_text(k);
    }
    ui.add(egui::Label::new(egui::RichText::new(value).color(t.ink)).wrap());
    ui.end_row();
}

/// The selected bend's properties, docked at the right: the bend, its tooling and station, the
/// press's part in it, and its checks as a table (Check, Result, Measured, Limit) with human
/// names and units; failures and checks that could not run first, the passes on asking.
/// Returns the bend's name when its "flat pattern" link is clicked.
pub fn bend_sheet(ui: &mut egui::Ui, t: &Theme, st: &mut BendSimUi) -> Option<String> {
    let mut flat = None;
    let all = st.all_checks;
    let mut toggle = false;
    let mut locate: Option<(f64, Option<[f64; 3]>)> = None;
    let sim = match &st.sim {
        Ok(s) => s,
        Err(e) => {
            ui.add(egui::Label::new(egui::RichText::new(e).color(look::FAIL)).wrap());
            return None;
        }
    };
    let plan = &sim.plan;
    let names = &sim.names;
    let cat = st.catalog.as_ref().and_then(|c| c.as_ref().ok());
    if let Some(s) = plan.steps.get(st.step) {
        let ok = s.pass();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Bend {} · {}", st.step + 1, names[s.bend]))
                    .font(super::theme::semi(14.0))
                    .color(t.ink),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let c = if ok { look::PASS } else { look::FAIL };
                egui::Frame::new()
                    .fill(c.gamma_multiply(0.18))
                    .stroke(egui::Stroke::new(1.0, c))
                    .corner_radius(9.0)
                    .inner_margin(egui::Margin::symmetric(8, 2))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(if ok { "Passes" } else { "Fails" })
                                .color(c)
                                .size(12.0),
                        );
                    });
            });
        });
        ui.label(
            egui::RichText::new(format!("step {} of {}", st.step + 1, plan.steps.len()))
                .small()
                .color(t.ink2),
        );
        head(ui, t, "Bend");
        let known = !sim.tooling.springback.is_empty();
        egui::Grid::new("bendsim-sheet-bend")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                prop(
                    ui,
                    t,
                    "Turn",
                    format!("{:.1}°", s.angle.to_degrees()),
                    Some("angle"),
                );
                prop(
                    ui,
                    t,
                    "Inside radius",
                    format!(
                        "{:.2} mm",
                        sim.model.bends[s.bend].inside_radius(s.angle + s.overbend)
                    ),
                    None,
                );
                prop(
                    ui,
                    t,
                    "Overbend",
                    format!("{:.1}°", s.overbend.to_degrees()),
                    Some("overbend"),
                );
                prop(
                    ui,
                    t,
                    "Springback",
                    if known {
                        "cited for this material and tooling".into()
                    } else {
                        "not known: no cited springback for this material, bent to the turn".into()
                    },
                    None,
                );
                prop(
                    ui,
                    t,
                    "Part",
                    format!(
                        "{}{}",
                        if s.flip { "flipped" } else { "as it lies" },
                        if s.setup.rotate { ", end for end" } else { "" }
                    ),
                    None,
                );
            });
        if ui.link("Show this bend on the flat pattern").clicked() {
            flat = Some(names[s.bend].clone());
        }
        head(ui, t, "Tooling");
        let station = sim
            .tooling
            .stations
            .iter()
            .position(|x| (x.punch, x.die) == (s.setup.punch, s.setup.die));
        egui::Grid::new("bendsim-sheet-tools")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                prop(
                    ui,
                    t,
                    "Punch",
                    cards::tool_title(cat, &s.punch, true),
                    Some(&s.punch),
                );
                prop(ui, t, "Die", cards::tool_title(cat, &s.die, false), Some(&s.die));
                if let Some(k) = station {
                    prop(
                        ui,
                        t,
                        "Station",
                        format!("{} at {:.0} mm", k + 1, sim.tooling.stations[k].y),
                        None,
                    );
                }
                if let (Some(pl), Some(dl)) = (s.punch_length, s.die_length) {
                    prop(
                        ui,
                        t,
                        "Stacks",
                        format!("punch {pl:.0} mm · die {dl:.0} mm"),
                        None,
                    );
                }
            });
        head(ui, t, "Press");
        egui::Grid::new("bendsim-sheet-press")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                prop(ui, t, "Ram travel", format!("{:.1} mm", s.travel), Some("travel"));
                prop(
                    ui,
                    t,
                    "Force",
                    s.force
                        .map_or("not known: the material's strength is not cited".into(), |f| {
                            format!("{f:.1} kN")
                        }),
                    Some("force"),
                );
                if let Some(g) = &s.gauge {
                    prop(
                        ui,
                        t,
                        "Backgauge",
                        format!(
                            "X {:.1} mm · R {:.1} mm · {} finger(s)",
                            g.x,
                            g.r,
                            g.fingers.len()
                        ),
                        None,
                    );
                }
            });
        // the checks: what failed and what could not be checked; the passes on asking
        let passing = s.checks.iter().filter(|c| c.pass).count();
        let shown: Vec<&fb::Check> = s.checks.iter().filter(|c| all || !c.pass).collect();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                egui::RichText::new("Checks")
                    .font(super::theme::semi(13.0))
                    .color(t.ink),
            );
            ui.label(
                egui::RichText::new(format!("{passing} of {} pass", s.checks.len()))
                    .small()
                    .color(t.ink2),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.link(if all { "Failures only" } else { "Show all" }).clicked() {
                    toggle = true;
                }
            });
        });
        egui::Grid::new("bendsim-checks")
            .num_columns(3)
            .striped(true)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                for h in ["Check", "Result", "Measured · limit"] {
                    ui.label(egui::RichText::new(h).small().color(t.ink2));
                }
                ui.end_row();
                for c in &shown {
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
                    // what each row is about, in words, on hover: why it could not run, or what
                    // it was against
                    let why = match (&c.unchecked, &c.tool) {
                        (Some(u), _) => format!("not checked: {u} unknown"),
                        (None, Some(x)) => format!("against {}", cards::body_words(cat, x)),
                        _ => String::new(),
                    };
                    let hover = |r: egui::Response| if why.is_empty() { r } else { r.on_hover_text(&why) };
                    hover(ui.label(egui::RichText::new(cards::rule_label(c.rule)).color(t.ink)))
                        .on_hover_text(c.rule.word());
                    // a failure with a place: its verdict locates it (the timeline to its instant,
                    // the eye on it)
                    let placed = !c.pass && c.unchecked.is_none() && (c.at.is_some() || c.phi.is_some());
                    if placed {
                        let r = ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(format!("{verdict} ›")).color(col).underline(),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(format!("{why}\nclick: go to the instant and look at it"));
                        if r.clicked() {
                            let phimax = s.angle + s.overbend;
                            let tt = match c.phase {
                                Some(fb::Phase::Static) => 0.2,
                                Some(fb::Phase::Approach) => 0.38,
                                Some(fb::Phase::Return) => 0.84,
                                _ => {
                                    0.4 + 0.35 * c.phi.map_or(1.0, |f| (f / phimax.max(1e-9)).clamp(0.0, 1.0))
                                }
                            };
                            locate = Some((tt, c.at));
                        }
                    } else {
                        hover(ui.label(egui::RichText::new(verdict).color(col)));
                    }
                    match &c.unchecked {
                        Some(u) => {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("{u} unknown"))
                                        .font(super::theme::caption())
                                        .color(t.ink2),
                                )
                                .wrap(),
                            );
                        }
                        None => {
                            hover(ui.label(format!(
                                "{} {} {}",
                                cards::rule_value(c.rule, c.value),
                                if c.at_most { "≤" } else { "≥" },
                                cards::rule_value(c.rule, c.limit)
                            )));
                        }
                    }
                    ui.end_row();
                }
            });
    }
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(if plan.feasible {
            "The program is feasible"
        } else {
            "No feasible order"
        })
        .color(if plan.feasible { look::PASS } else { look::FAIL }),
    );
    for b in &plan.blocking {
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "{} is blocked: {}",
                    names[b.bend],
                    b.checks
                        .iter()
                        .map(|c| cards::rule_label(c.rule).to_lowercase())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .color(look::FAIL),
            )
            .wrap(),
        );
    }
    if toggle {
        st.all_checks = !st.all_checks;
    }
    if let Some((t, at)) = locate {
        st.t = t;
        st.playing = false;
        st.locate = at.map(|a| {
            let oy = st
                .sim
                .as_ref()
                .ok()
                .and_then(|s| s.plan.steps.get(st.step))
                .and_then(|x| {
                    let sim = st.sim.as_ref().ok()?;
                    sim.tooling.station(x.setup.punch, x.setup.die).map(|k| k.y)
                })
                .unwrap_or(0.0);
            [a[0], a[1] + oy, a[2]]
        });
    }
    flat
}

/// A transport button: its icon, its name and key on hover.
fn transport(ui: &mut egui::Ui, icon: &str, tip: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
    let p = ui.painter_at(r);
    if resp.hovered() {
        p.rect_filled(r, 4.0, egui::Color32::from_white_alpha(28));
    }
    let ink = egui::Color32::from_gray(230);
    let at = r.center() - egui::vec2(8.0, 8.0);
    match icon {
        "play" => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    at + egui::vec2(4.0, 2.0),
                    at + egui::vec2(14.0, 8.0),
                    at + egui::vec2(4.0, 14.0),
                ],
                ink,
                egui::Stroke::NONE,
            ));
        }
        "pause" => {
            for x in [4.0, 10.0] {
                p.rect_filled(
                    egui::Rect::from_min_size(at + egui::vec2(x, 2.0), egui::vec2(3.0, 12.0)),
                    0.5,
                    ink,
                );
            }
        }
        // first, step back, step on, last: a triangle and, at an end, a bar
        _ => {
            let back = matches!(icon, "first" | "stepback");
            let tri = |x0: f32, x1: f32| {
                egui::Shape::convex_polygon(
                    vec![
                        at + egui::vec2(x0, 3.0),
                        at + egui::vec2(x1, 8.0),
                        at + egui::vec2(x0, 13.0),
                    ],
                    ink,
                    egui::Stroke::NONE,
                )
            };
            p.add(if back { tri(13.0, 5.0) } else { tri(3.0, 11.0) });
            if matches!(icon, "first" | "last") {
                let x = if back { 3.0 } else { 11.5 };
                p.rect_filled(
                    egui::Rect::from_min_size(at + egui::vec2(x, 3.0), egui::vec2(2.0, 10.0)),
                    0.5,
                    ink,
                );
            }
        }
    }
    resp.on_hover_text(tip)
}

/// Follow bend, See through and its opacity: listed right to left at the bar's end, or left to
/// right on a row of their own.
fn view_settings(ui: &mut egui::Ui, st: &mut BendSimUi, rtl: bool) {
    ui.spacing_mut().item_spacing.x = 8.0;
    let ink = egui::Color32::from_gray(230);
    let follow = |ui: &mut egui::Ui, st: &mut BendSimUi| {
        if ui
            .checkbox(&mut st.follow, egui::RichText::new("Follow bend").color(ink))
            .on_hover_text("frame each new bend; off, the view stays where you put it")
            .changed()
            && st.follow
        {
            st.frame = Some(Framing::Work);
        }
    };
    let see = |ui: &mut egui::Ui, st: &mut BendSimUi| {
        ui.checkbox(&mut st.see_through, egui::RichText::new("See through").color(ink))
            .on_hover_text(
                "what stands between you and the bend being made is drawn faint; a collision stays solid red",
            );
    };
    let opacity = |ui: &mut egui::Ui, st: &mut BendSimUi| {
        if st.see_through {
            ui.add(
                egui::Slider::new(&mut st.opacity, 0.05..=0.8)
                    .show_value(false)
                    .text(egui::RichText::new("opacity").color(egui::Color32::from_gray(200))),
            )
            .on_hover_text("how much of what is seen through still shows");
        }
    };
    if rtl {
        opacity(ui, st);
        see(ui, st);
        follow(ui, st);
    } else {
        follow(ui, st);
        see(ui, st);
        opacity(ui, st);
    }
}

/// How much room the play bar has: the view's width decides (STANDARD.md §5: nothing drawn
/// over anything).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bar {
    /// All on one row.
    Wide,
    /// The view's settings on a row of their own.
    Narrow,
    /// Icons, "1/2 · gauge", and the view's settings in a View menu.
    Compact,
}

impl Bar {
    pub fn for_width(w: f32) -> Bar {
        if w < 560.0 {
            Bar::Compact
        } else if w < 900.0 {
            Bar::Narrow
        } else {
            Bar::Wide
        }
    }
}

/// The timeline: every step a block, its stages named in it; drag to scrub.
pub fn timeline_ui(ui: &mut egui::Ui, st: &mut BendSimUi, bar: Bar) {
    let n = st.steps();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if transport(ui, "first", "First bend (Home)").clicked() {
            st.go(0);
        }
        if transport(ui, "stepback", "Previous bend (Left arrow)").clicked() {
            st.go(st.step.saturating_sub(1));
        }
        let (icon, tip) = if st.playing {
            ("pause", "Pause (Space)")
        } else {
            ("play", "Play (Space)")
        };
        if transport(ui, icon, tip).clicked() {
            st.play();
        }
        if transport(ui, "stepfwd", "Next bend (Right arrow)").clicked() && n > 0 {
            st.go((st.step + 1).min(n - 1));
        }
        if transport(ui, "last", "Last bend (End)").clicked() && n > 0 {
            st.go(n - 1);
        }
        ui.add_space(if bar == Bar::Compact { 6.0 } else { 10.0 });
        let (stage, _) = stage_at(st.t);
        let pose = st.pose();
        let words = match bar {
            Bar::Compact => format!("{}/{} · {}", st.step + 1, n, stage.word()),
            _ => format!(
                "Bend {} of {} · {}{}",
                st.step + 1,
                n,
                stage.word(),
                pose.map_or(String::new(), |p| format!(" · {:.1}°", p.phi.to_degrees()))
            ),
        };
        ui.label(egui::RichText::new(words).color(egui::Color32::from_gray(230)));
        // the view's own settings, at its foot: follow the bend, see through what hides it (on a
        // row of their own when the view is narrow; in a menu when it is narrower still)
        match bar {
            Bar::Wide => {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    view_settings(ui, st, true);
                });
            }
            Bar::Compact => {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button(
                        egui::RichText::new("View").color(egui::Color32::from_gray(230)),
                        |ui| {
                            ui.vertical(|ui| view_settings(ui, st, false));
                        },
                    );
                });
            }
            Bar::Narrow => {}
        }
    });
    if bar == Bar::Narrow {
        ui.horizontal(|ui| view_settings(ui, st, false));
    }
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
            // the stage's name inside its segment, where it fits
            let word = if s == Stage::Bend {
                format!("Bend {} · bend", k + 1)
            } else {
                s.word().to_string()
            };
            let g = p.layout_no_wrap(
                word,
                egui::FontId::proportional(10.0),
                egui::Color32::from_gray(235),
            );
            if g.size().x + 6.0 < r.width() {
                p.galley(
                    egui::pos2(r.left() + 3.0, r.center().y - 0.5 * g.size().y),
                    g,
                    egui::Color32::from_gray(235),
                );
            }
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
    let cat = st.catalog.as_ref().and_then(|c| c.as_ref().ok());
    let mut lines = vec![];
    if let Some(s) = sim.plan.steps.get(st.step) {
        let (stage, _) = stage_at(st.t);
        // the caption: which bend, what it makes, where in its motion; the tools are the sheet's
        lines.push((
            format!(
                "Bend {} · {} · {:.0}° · {}",
                st.step + 1,
                sim.names[s.bend],
                s.angle.to_degrees(),
                stage.word()
            ),
            egui::Color32::from_gray(230),
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
                        c.tool
                            .as_deref()
                            .map_or(String::new(), |t| format!(" of {}", cards::body_words(cat, t)))
                    ),
                    look::WARN,
                ),
                None => ("operator handling: the first step, none".into(), look::MUTED),
            });
        }
        for c in s.failures() {
            lines.push((
                format!(
                    "{}{}{}",
                    cards::rule_label(c.rule),
                    c.tool
                        .as_deref()
                        .map_or(String::new(), |t| format!(" · {}", cards::body_words(cat, t))),
                    c.phi.map_or(String::new(), |f| format!(
                        " at {:.1}° of the turn",
                        f.to_degrees()
                    ))
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
    fn edits_are_ops_on_the_bending_document_with_undo() {
        const C: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
            sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (100, 200))\n}\n\
            base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
            right = flange(base.edge(plan.outline[1]), height: 30)\n\
            left = flange(base.edge(plan.outline[3]), height: 30)\n";
        let d = std::env::temp_dir().join(format!("fg-bendsim-doc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let part = d.join("channel.fg");
        std::fs::write(&part, C).unwrap();
        let c = Choice {
            press: Some("trumpf-trubend-5130".into()),
            ..Default::default()
        };
        let mut ui = BendSimUi::open_with(C, part.to_str().unwrap(), "std-eu", c);
        assert!(ui.doc.is_none());
        let order = |u: &BendSimUi| -> Vec<String> {
            let s = u.sim.as_ref().unwrap();
            s.plan.sequence.iter().map(|b| s.names[*b].clone()).collect()
        };
        let before = order(&ui);
        // the first edit writes the document, then moves the second bend first
        ui.doc_op(&forge_lang::error::Op::MoveFeature {
            id: "b2".into(),
            before: Some("b1".into()),
        })
        .unwrap();
        let doc = ui.doc.as_ref().expect("a document");
        assert!(doc.path.ends_with("channel_bend.fg") && doc.src.starts_with("bending"));
        assert_eq!(order(&ui), vec![before[1].clone(), before[0].clone()]);
        // turned end for end: an argument in the text
        ui.doc_op(&forge_lang::error::Op::SetArg {
            id: "b1".into(),
            arg: "rotate".into(),
            text: "true".into(),
        })
        .unwrap();
        assert!(ui.doc.as_ref().unwrap().src.contains("rotate: true"));
        // undone twice: the order the program was written with
        ui.doc_step(true).unwrap();
        ui.doc_step(true).unwrap();
        assert_eq!(order(&ui), before);
        ui.doc_step(false).unwrap();
        assert_eq!(order(&ui), vec![before[1].clone(), before[0].clone()]);
        // opened again: the document says what is simulated
        let again = BendSimUi::open(C, part.to_str().unwrap(), "std-eu");
        assert!(again.doc.is_some());
        assert_eq!(order(&again), vec![before[1].clone(), before[0].clone()]);
        let _ = std::fs::remove_dir_all(&d);
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
