//! The Robot workspace's panel (research/export/urdf-editor.md §6): a robot document's links
//! and joints as a tree from the root, a slider per moving joint (held within its limits; a
//! mimic follows its leader), the selected joint's or link's values with each field marked
//! linked or overridden, and what the export says (its refusal with fixes, warnings, notes).
//! The host (gui_robot.rs) reads `q` and moves the parts; nothing here edits a document yet.

use super::Shell;
use super::placed::Placed;
use super::theme::{medium, mono, sans};
use egui::{RichText, Ui, pos2, vec2};
use forge_lang::robot::Edit;
use forge_lang::urdf::{Conflict, Exported, RobotView, UrdfError, ViewJoint};
use std::collections::BTreeMap;

/// What the panel asks of the host, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum RobotAct {
    /// An edit of the robot document's text.
    Edit(Edit),
    /// A joint's argument written in the source assembly (`SetArg`), then the robot's override
    /// of it reset.
    Assembly {
        joint: String,
        arg: String,
        text: String,
        then: Edit,
    },
    /// The robot document's text as it was before the last edit.
    Undo,
    /// The URDF written beside the robot document.
    Export,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Sel {
    Link(String),
    Joint(String),
}

/// A robot document open in the workspace, and the panel's state.
#[derive(Default)]
pub struct RobotPanel {
    /// The robot document and the source it shows.
    pub file: String,
    pub source: Option<String>,
    pub name: String,
    pub view: Option<RobotView>,
    /// (line, statement, field, linked, set) as the export applied them.
    pub overrides: Vec<(usize, String, String, String, String)>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
    /// The export's refusal: kind, text, fixes.
    pub error: Option<(String, String, Vec<String>)>,
    /// Each moving joint's value as the sliders have it (URDF units).
    pub q: BTreeMap<String, f64>,
    pub sel: Option<Sel>,
    /// The sliders moved: the host places the parts again.
    pub changed: bool,
    /// The source was solved again: the export runs again.
    pub stale: bool,
    /// Overrides the source moved under, each to keep or reset.
    pub conflicts: Vec<Conflict>,
    /// Asks for the host.
    pub out: Vec<RobotAct>,
    /// The document's text before each edit, for Undo.
    pub undo: Vec<String>,
    /// Values typed in the selected joint's or link's fields, not yet sent.
    pub draft: BTreeMap<String, f64>,
    /// The last thing the host did, said.
    pub said: String,
    pub open: bool,
}

impl RobotPanel {
    pub fn new(file: &str) -> RobotPanel {
        RobotPanel {
            file: file.to_string(),
            open: true,
            ..RobotPanel::default()
        }
    }

    /// The export's answer: the view and its notes, or its refusal (the last view stays).
    pub fn load(&mut self, r: Result<Exported, UrdfError>) {
        match r {
            Ok(e) => {
                self.name = e.name.clone();
                self.overrides = e.overrides.clone();
                self.warnings = e.warnings.clone();
                self.notes = e.notes.clone();
                self.error = None;
                self.conflicts.clear();
                // values kept for joints that are still there
                let keep: BTreeMap<String, f64> = self
                    .q
                    .iter()
                    .filter(|(k, _)| e.view.joints.iter().any(|j| j.name == **k))
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                self.q = keep;
                self.view = Some(e.view);
                self.changed = true;
            }
            Err(e) => {
                self.conflicts = match &e {
                    UrdfError::Conflicts(cs) => cs.clone(),
                    _ => vec![],
                };
                self.error = Some((e.kind().to_string(), e.text(), e.fixes()));
            }
        }
    }

    /// A joint's value now: the slider's, a mimic's from its leader, else its export position.
    pub fn value(&self, j: &ViewJoint) -> f64 {
        if let Some((leader, m, off)) = &j.mimic {
            let l = self
                .view
                .as_ref()
                .and_then(|v| v.joints.iter().find(|x| x.name == *leader));
            let lv = l.map_or(0.0, |l| self.value(l));
            return m * lv + off;
        }
        self.q
            .get(&j.name)
            .copied()
            .unwrap_or_else(|| j.position.first().copied().unwrap_or(0.0))
    }

    /// Every slider back to the export pose.
    pub fn home(&mut self) {
        self.q.clear();
        self.changed = true;
    }

    /// A joint moved by hand (or by a test): held within its limits.
    pub fn set(&mut self, joint: &str, v: f64) -> bool {
        let Some(j) = self
            .view
            .as_ref()
            .and_then(|w| w.joints.iter().find(|j| j.name == joint))
        else {
            return false;
        };
        if j.mimic.is_some() || j.position.len() != 1 {
            return false;
        }
        let v = j.clamp(v);
        self.q.insert(joint.to_string(), v);
        self.changed = true;
        true
    }

    /// The line of the robot's statement overriding `what` (`joint(elbow)`), if it has one.
    fn line_of(&self, what: &str) -> Option<usize> {
        self.overrides.iter().find(|o| o.1 == what).map(|o| o.0)
    }

    /// How the document names a joint: `src.J` where it is one URDF joint of source joint J,
    /// else its URDF name as a string.
    fn joint_target(&self, j: &ViewJoint) -> String {
        let one = self
            .view
            .as_ref()
            .is_some_and(|v| v.joints.iter().filter(|x| x.of == j.of).count() == 1);
        if self.source.is_some() && one && !j.of.is_empty() {
            format!("src.{}", j.of)
        } else {
            format!("\"{}\"", j.name)
        }
    }

    /// A field of a joint set as an override: on its statement if it has one, else a new one;
    /// with the source's value recorded (`was`) when the field was linked till now.
    pub fn set_joint(&mut self, j: &ViewJoint, field: &str, text: String, was: Option<String>) {
        let what = format!("joint({})", j.name);
        let overridden = self.overridden(&what, field).is_some();
        self.out.push(RobotAct::Edit(Edit::Set {
            line: self.line_of(&what),
            call: "joint".into(),
            target: self.joint_target(j),
            field: field.into(),
            text,
            was: if overridden { None } else { was },
        }));
    }

    fn overridden(&self, what: &str, field: &str) -> Option<&str> {
        self.overrides
            .iter()
            .find(|o| o.1 == what && o.2 == field)
            .map(|o| o.3.as_str())
    }
}

fn mm(x: f64) -> String {
    format!("{x:.3}")
}

impl Shell {
    pub(super) fn robot_window(&mut self, ctx: &egui::Context) {
        let Some(mut p) = self.robot.take() else { return };
        if !p.open {
            self.robot = Some(p);
            return;
        }
        let t = self.theme;
        let v = self.view_rect;
        let mut places = Vec::new();
        let mut close = false;
        let w = 340.0;
        let shown = egui::Window::new("Robot")
            .id(egui::Id::unique("robot-panel"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .movable(true)
            .fade_in(false)
            .default_pos(pos2(v.right() - w - 24.0, v.top() + 16.0))
            .min_width(w)
            .max_width(w)
            .frame(super::fgasm::dialog_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                let title = if p.name.is_empty() {
                    "Robot".to_string()
                } else {
                    format!("Robot · {}", p.name)
                };
                close = super::fgasm::dialog_head(ui, &t, "assembly", &title, "robot", places);
                if let Some(s) = &p.source {
                    ui.label(
                        RichText::new(format!("linked to {s}"))
                            .font(sans(11.0))
                            .color(t.ink2),
                    );
                } else {
                    ui.label(RichText::new("unlinked").font(sans(11.0)).color(t.ink2));
                }
                if let Some((kind, text, fixes)) = p.error.clone() {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(t.error.gamma_multiply(0.15))
                        .corner_radius(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new(kind).font(medium(12.0)).color(t.error));
                            ui.label(RichText::new(text).font(sans(11.0)).color(t.ink));
                            for f in fixes {
                                ui.label(RichText::new(format!("fix: {f}")).font(mono(11.0)).color(t.ink2));
                            }
                        });
                }
                for c in p.conflicts.clone() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("line {}: {} {}", c.line, c.what, c.field))
                                .font(sans(11.0))
                                .color(t.ink),
                        );
                        if ui
                            .small_button("Keep")
                            .on_hover_text(format!("keep {} over the source's new {}", c.set, c.now))
                            .clicked()
                        {
                            p.out.push(RobotAct::Edit(Edit::Set {
                                line: Some(c.line),
                                call: String::new(),
                                target: String::new(),
                                field: format!("{}_was", c.field),
                                text: c.now.clone(),
                                was: None,
                            }));
                        }
                        if ui
                            .small_button("Reset")
                            .on_hover_text(format!("take the source's {}", c.now))
                            .clicked()
                        {
                            p.out.push(RobotAct::Edit(Edit::Reset {
                                line: c.line,
                                field: c.field.clone(),
                            }));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!p.undo.is_empty(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        p.out.push(RobotAct::Undo);
                    }
                    if ui.button("Export URDF").clicked() {
                        p.out.push(RobotAct::Export);
                    }
                    if !p.said.is_empty() {
                        ui.label(RichText::new(&p.said).font(sans(11.0)).color(t.ink2));
                    }
                });
                let Some(view) = p.view.clone() else {
                    ui.label(RichText::new("building…").font(sans(12.0)).color(t.ink2));
                    return;
                };
                ui.add_space(4.0);
                let moving = view.joints.iter().filter(|j| j.position.len() == 1).count();
                ui.label(
                    RichText::new(format!(
                        "{} links · {} joints ({moving} moving) · root {}",
                        view.links.len(),
                        view.joints.len(),
                        view.root
                    ))
                    .font(sans(11.0))
                    .color(t.ink2),
                );
                // the window grows with what is shown, up to the view's height, then scrolls
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height((v.height() - 120.0).max(240.0))
                    .show(ui, |ui| {
                        Self::robot_tree(ui, &t, &mut p, &view);
                        ui.add_space(6.0);
                        Self::robot_sliders(ui, &t, &mut p, &view);
                        ui.add_space(6.0);
                        Self::robot_detail(ui, &t, &mut p, &view);
                        if !p.warnings.is_empty() || !p.notes.is_empty() {
                            ui.add_space(6.0);
                            ui.label(RichText::new("Checks").font(medium(12.0)).color(t.ink));
                            for w in &p.warnings {
                                ui.label(
                                    RichText::new(format!("warning: {w}"))
                                        .font(sans(11.0))
                                        .color(t.warn),
                                );
                            }
                            for n in &p.notes {
                                ui.label(RichText::new(format!("· {n}")).font(sans(11.0)).color(t.ink2));
                            }
                        }
                    });
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
        }
        if close {
            p.open = false;
        }
        self.places.extend(places);
        self.robot = Some(p);
    }

    fn robot_tree(ui: &mut Ui, t: &super::theme::Theme, p: &mut RobotPanel, view: &RobotView) {
        ui.label(RichText::new("Links and joints").font(medium(12.0)).color(t.ink));
        // depth first from the root, joints in document order
        let mut rows: Vec<(usize, Sel, String)> = vec![];
        let mut stack = vec![(view.root.clone(), 0usize)];
        while let Some((link, d)) = stack.pop() {
            let l = view.links.iter().find(|l| l.name == link);
            let tag = match l {
                Some(l) if l.dummy => " (massless)".to_string(),
                Some(l) => l.mass.map_or(String::new(), |m| format!("  {m:.3} kg")),
                None => String::new(),
            };
            rows.push((d, Sel::Link(link.clone()), format!("{link}{tag}")));
            for j in view.joints.iter().filter(|j| j.parent == link).rev() {
                stack.push((j.child.clone(), d + 2));
            }
            for j in view.joints.iter().filter(|j| j.parent == link) {
                let k = if j.mimic.is_some() {
                    format!("{} · mimic", j.kind)
                } else {
                    j.kind.to_string()
                };
                rows.push((d + 1, Sel::Joint(j.name.clone()), format!("· {} ({k})", j.name)));
            }
        }
        for (d, sel, text) in rows {
            ui.horizontal(|ui| {
                ui.add_space(d as f32 * 12.0);
                let on = p.sel.as_ref() == Some(&sel);
                let r = ui.selectable_label(on, RichText::new(text).font(sans(12.0)).color(t.ink));
                if r.clicked() {
                    p.sel = if on { None } else { Some(sel) };
                }
            });
        }
    }

    fn robot_sliders(ui: &mut Ui, t: &super::theme::Theme, p: &mut RobotPanel, view: &RobotView) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Drive").font(medium(12.0)).color(t.ink));
            if ui.small_button("Export pose").clicked() {
                p.home();
            }
        });
        egui::Grid::new("robot-drive")
            .num_columns(2)
            .spacing(vec2(8.0, 4.0))
            .show(ui, |ui| {
                for j in view.joints.iter().filter(|j| j.position.len() == 1) {
                    ui.label(RichText::new(&j.name).font(sans(12.0)).color(t.ink));
                    let angular = j.kind != "prismatic";
                    let (lo, hi) = j.limit.unwrap_or(if angular {
                        (-std::f64::consts::PI, std::f64::consts::PI)
                    } else {
                        (-0.1, 0.1)
                    });
                    let mut v = p.value(j);
                    // shown in degrees or mm, held in URDF's units
                    let k = if angular {
                        180.0 / std::f64::consts::PI
                    } else {
                        1000.0
                    };
                    let mut shown = v * k;
                    let s = egui::Slider::new(&mut shown, lo * k..=hi * k)
                        .suffix(if angular { "°" } else { " mm" })
                        .fixed_decimals(1);
                    let r = ui.add_enabled(j.mimic.is_none(), s);
                    if r.changed() {
                        v = shown / k;
                        p.set(&j.name, v);
                    }
                    ui.end_row();
                }
            });
    }

    fn robot_detail(ui: &mut Ui, t: &super::theme::Theme, p: &mut RobotPanel, view: &RobotView) {
        let mark = |ui: &mut Ui, linked: Option<&str>| {
            if let Some(l) = linked {
                let was = if l.is_empty() {
                    "robot only".to_string()
                } else {
                    format!("source: {l}")
                };
                ui.label(
                    RichText::new(format!("overridden ({was})"))
                        .font(sans(10.0))
                        .color(t.accent),
                );
            } else {
                ui.label(RichText::new("linked").font(sans(10.0)).color(t.ink2));
            }
        };
        let row = |ui: &mut Ui, k: &str, v: String| {
            ui.label(RichText::new(k).font(sans(11.0)).color(t.ink2));
            ui.label(RichText::new(v).font(mono(11.0)).color(t.ink));
            ui.end_row();
        };
        match p.sel.clone() {
            Some(Sel::Joint(n)) => {
                let Some(j) = view.joints.iter().find(|j| j.name == n).cloned() else {
                    return;
                };
                let what = format!("joint({})", j.name);
                let line = p.line_of(&what);
                ui.label(
                    RichText::new(format!("Joint {}", j.name))
                        .font(medium(12.0))
                        .color(t.ink),
                );
                egui::Grid::new("robot-joint")
                    .num_columns(2)
                    .spacing(vec2(10.0, 3.0))
                    .show(ui, |ui| {
                        row(ui, "type", j.kind.to_string());
                        row(ui, "parent, child", format!("{}, {}", j.parent, j.child));
                        let o = j.origin.origin;
                        row(
                            ui,
                            "origin (mm)",
                            format!("{}, {}, {}", mm(o.x), mm(o.y), mm(o.z)),
                        );
                        if let Some(a) = j.axis {
                            row(ui, "axis", format!("{:.4}, {:.4}, {:.4}", a[0], a[1], a[2]));
                        }
                        if let Some((l, m, off)) = &j.mimic {
                            row(ui, "mimics", format!("{l} × {m} + {off}"));
                        }
                        row(ui, "source joint", j.of.clone());
                    });
                // the limits: shown in degrees or mm, sent in the document's units
                if matches!(j.kind, "revolute" | "prismatic" | "continuous") {
                    let angular = j.kind != "prismatic";
                    let k = if angular {
                        180.0 / std::f64::consts::PI
                    } else {
                        1000.0
                    };
                    let unit = if angular { "deg" } else { "" };
                    let txt = |lo: f64, hi: f64| {
                        let f = |x: f64| format!("{}{unit}", (x * 1e6).round() / 1e6);
                        format!("({}, {})", f(lo), f(hi))
                    };
                    let (lo0, hi0) = j.limit.map_or((-180.0, 180.0), |(a, b)| (a * k, b * k));
                    let over = p.overridden(&what, "limit").map(str::to_string);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("limits").font(sans(11.0)).color(t.ink2));
                        mark(ui, over.as_deref());
                    });
                    let lo_key = format!("{n}.lo");
                    let hi_key = format!("{n}.hi");
                    let mut lo = *p.draft.get(&lo_key).unwrap_or(&lo0);
                    let mut hi = *p.draft.get(&hi_key).unwrap_or(&hi0);
                    ui.horizontal(|ui| {
                        let suffix = if angular { "°" } else { " mm" };
                        if ui
                            .add(egui::DragValue::new(&mut lo).speed(0.5).suffix(suffix))
                            .changed()
                        {
                            p.draft.insert(lo_key.clone(), lo);
                        }
                        if ui
                            .add(egui::DragValue::new(&mut hi).speed(0.5).suffix(suffix))
                            .changed()
                        {
                            p.draft.insert(hi_key.clone(), hi);
                        }
                        if ui.button("Override").clicked() && lo <= hi {
                            let was = j.limit.map(|_| txt(lo0, hi0));
                            p.set_joint(&j, "limit", txt(lo, hi), was);
                            p.draft.remove(&lo_key);
                            p.draft.remove(&hi_key);
                        }
                    });
                    if over.is_some()
                        && let Some(l) = line
                    {
                        ui.horizontal(|ui| {
                            if ui.button("Reset to linked").clicked() {
                                p.out.push(RobotAct::Edit(Edit::Reset {
                                    line: l,
                                    field: "limit".into(),
                                }));
                            }
                            // a single-freedom joint's `limit:` (a chain's freedoms have their own words)
                            if p.source.is_some()
                                && j.of == j.name
                                && ui
                                    .button("Apply to assembly")
                                    .on_hover_text("write these limits in the source assembly, and drop the robot's override")
                                    .clicked()
                            {
                                p.out.push(RobotAct::Assembly {
                                    joint: j.of.clone(),
                                    arg: "limit".into(),
                                    text: txt(lo0, hi0),
                                    then: Edit::Reset {
                                        line: l,
                                        field: "limit".into(),
                                    },
                                });
                            }
                        });
                    }
                }
                // what URDF adds: actuation and dynamics, the robot's own
                ui.add_space(4.0);
                egui::Grid::new("robot-act")
                    .num_columns(3)
                    .spacing(vec2(8.0, 3.0))
                    .show(ui, |ui| {
                        for (field, unit) in [
                            ("effort", "N or N m"),
                            ("velocity", "m/s or rad/s"),
                            ("damping", ""),
                            ("friction", ""),
                        ] {
                            if j.position.len() != 1 {
                                break;
                            }
                            let now = p
                                .overrides
                                .iter()
                                .find(|o| o.1 == what && o.2 == field)
                                .map(|o| o.4.clone());
                            // what the export writes: the joint's own, the document's actuation(..), or none
                            let written = match field {
                                "effort" => j.effort,
                                "velocity" => j.velocity,
                                "damping" => j.dynamics.map(|d| d.0),
                                _ => j.dynamics.map(|d| d.1),
                            };
                            let tip = match (&now, field) {
                                (Some(_), _) => format!("{unit}; this joint's own"),
                                (None, "effort" | "velocity") if written.is_some() => {
                                    format!("{unit}; from actuation(..)")
                                }
                                (None, "effort" | "velocity") => {
                                    format!("{unit}; not given: the export refuses (or writes a placeholder)")
                                }
                                _ => format!("{unit}; not given (0)"),
                            };
                            let ink = if now.is_some() { t.accent } else { t.ink2 };
                            ui.label(RichText::new(field).font(sans(11.0)).color(ink))
                                .on_hover_text(tip);
                            let key = format!("{n}.{field}");
                            let mut v = *p.draft.get(&key).unwrap_or(&written.unwrap_or(0.0));
                            if ui
                                .add(egui::DragValue::new(&mut v).speed(0.1).range(0.0..=f64::MAX))
                                .changed()
                            {
                                p.draft.insert(key.clone(), v);
                            }
                            ui.horizontal(|ui| {
                                if ui.small_button("Set").clicked() {
                                    p.set_joint(&j, field, format!("{v}"), None);
                                    p.draft.remove(&key);
                                }
                                if now.is_some()
                                    && let Some(l) = line
                                    && ui.small_button("Reset").clicked()
                                {
                                    p.out.push(RobotAct::Edit(Edit::Reset {
                                        line: l,
                                        field: field.into(),
                                    }));
                                }
                            });
                            ui.end_row();
                        }
                    });
            }
            Some(Sel::Link(n)) => {
                let Some(l) = view.links.iter().find(|l| l.name == n).cloned() else {
                    return;
                };
                let what = format!("inertial({})", l.name);
                ui.label(
                    RichText::new(format!("Link {}", l.name))
                        .font(medium(12.0))
                        .color(t.ink),
                );
                egui::Grid::new("robot-link")
                    .num_columns(2)
                    .spacing(vec2(10.0, 3.0))
                    .show(ui, |ui| {
                        row(ui, "bodies", l.bodies.join(" "));
                        let c = l.com;
                        row(
                            ui,
                            "centre of mass (mm)",
                            format!("{}, {}, {}", mm(c[0]), mm(c[1]), mm(c[2])),
                        );
                        row(ui, "visuals", format!("{}", l.visuals.len()));
                        let shapes: Vec<String> = l
                            .collisions
                            .iter()
                            .map(|c| match &c.shape {
                                forge_lang::urdf::Shape::Mesh { file, .. } => {
                                    file.rsplit('/').next().unwrap_or(file).to_string()
                                }
                                forge_lang::urdf::Shape::Box(_) => "box".into(),
                                forge_lang::urdf::Shape::Cylinder { .. } => "cylinder".into(),
                                forge_lang::urdf::Shape::Sphere(_) => "sphere".into(),
                            })
                            .collect();
                        row(ui, "collision", shapes.join(", "));
                    });
                let over = p.overridden(&what, "mass").map(str::to_string);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("mass").font(sans(11.0)).color(t.ink2));
                    mark(ui, over.as_deref());
                });
                let key = format!("{n}.mass");
                let m0 = l.mass.unwrap_or(0.0);
                let mut m = *p.draft.get(&key).unwrap_or(&m0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::DragValue::new(&mut m)
                                .speed(0.001)
                                .range(1e-9..=f64::MAX)
                                .suffix(" kg"),
                        )
                        .changed()
                    {
                        p.draft.insert(key.clone(), m);
                    }
                    if !l.dummy && ui.button("Override").clicked() {
                        let target = match l.bodies.first() {
                            Some(b) if p.source.is_some() => format!("src.{b}"),
                            _ => format!("\"{}\"", l.name),
                        };
                        p.out.push(RobotAct::Edit(Edit::Set {
                            line: p.line_of(&what),
                            call: "inertial".into(),
                            target,
                            field: "mass".into(),
                            text: format!("{m}"),
                            was: if over.is_some() {
                                None
                            } else {
                                Some(format!("{m0}"))
                            },
                        }));
                        p.draft.remove(&key);
                    }
                    if over.is_some()
                        && let Some(line) = p.line_of(&what)
                        && ui.button("Reset to linked").clicked()
                    {
                        p.out.push(RobotAct::Edit(Edit::Reset {
                            line,
                            field: "mass".into(),
                        }));
                    }
                });
            }
            None => {
                ui.label(
                    RichText::new("Select a link or a joint for its values.")
                        .font(sans(11.0))
                        .color(t.ink2),
                );
            }
        }
    }
}
