//! The Robot workspace's panel (research/export/urdf-editor.md §6): a robot document's links
//! and joints as a tree from the root, a slider per moving joint (held within its limits; a
//! mimic follows its leader), the selected joint's or link's values with each field marked
//! linked or overridden, and what the export says (its refusal with fixes, warnings, notes).
//! The host (gui_robot.rs) reads `q` and moves the parts; nothing here edits a document yet.

use super::Shell;
use super::placed::Placed;
use super::theme::{medium, mono, sans};
use egui::{Color32, RichText, Ui, pos2, vec2};
use forge_lang::urdf::{Exported, RobotView, UrdfError, ViewJoint};
use std::collections::BTreeMap;

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

    fn overridden(&self, what: &str, field: &str) -> Option<&str> {
        self.overrides
            .iter()
            .find(|o| o.1 == what && o.2 == field)
            .map(|o| o.3.as_str())
    }
}

fn deg(x: f64) -> String {
    format!("{:.2}°", x.to_degrees())
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
                        Self::robot_detail(ui, &t, &p, &view);
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

    fn robot_detail(ui: &mut Ui, t: &super::theme::Theme, p: &RobotPanel, view: &RobotView) {
        let row = |ui: &mut Ui, k: &str, v: String, linked: Option<&str>| {
            ui.label(RichText::new(k).font(sans(11.0)).color(t.ink2));
            ui.horizontal(|ui| {
                ui.label(RichText::new(v).font(mono(11.0)).color(t.ink));
                if let Some(l) = linked {
                    let was = if l.is_empty() {
                        "robot".to_string()
                    } else {
                        format!("source: {l}")
                    };
                    ui.label(
                        RichText::new(format!("overridden ({was})"))
                            .font(sans(10.0))
                            .color(t.accent),
                    );
                }
            });
            ui.end_row();
        };
        match &p.sel {
            Some(Sel::Joint(n)) => {
                let Some(j) = view.joints.iter().find(|j| j.name == *n) else {
                    return;
                };
                let what = format!("joint({})", j.name);
                ui.label(
                    RichText::new(format!("Joint {}", j.name))
                        .font(medium(12.0))
                        .color(t.ink),
                );
                egui::Grid::new("robot-joint")
                    .num_columns(2)
                    .spacing(vec2(10.0, 3.0))
                    .show(ui, |ui| {
                        row(ui, "type", j.kind.to_string(), p.overridden(&what, "type"));
                        row(ui, "parent, child", format!("{}, {}", j.parent, j.child), None);
                        let o = j.origin.origin;
                        row(
                            ui,
                            "origin (mm)",
                            format!("{}, {}, {}", mm(o.x), mm(o.y), mm(o.z)),
                            None,
                        );
                        if let Some(a) = j.axis {
                            row(
                                ui,
                                "axis",
                                format!("{:.4}, {:.4}, {:.4}", a[0], a[1], a[2]),
                                p.overridden(&what, "axis"),
                            );
                        }
                        if let Some((lo, hi)) = j.limit {
                            let s = if j.kind == "prismatic" {
                                format!("{} … {} mm", mm(lo * 1000.0), mm(hi * 1000.0))
                            } else {
                                format!("{} … {}", deg(lo), deg(hi))
                            };
                            row(ui, "limits", s, p.overridden(&what, "limit"));
                        }
                        if let Some((l, m, off)) = &j.mimic {
                            row(ui, "mimics", format!("{l} × {m} + {off}"), None);
                        }
                        for f in ["effort", "velocity", "damping", "friction"] {
                            if let Some(o) = p.overrides.iter().find(|o| o.1 == what && o.2 == f) {
                                row(ui, f, o.4.clone(), Some(""));
                            }
                        }
                        row(ui, "source joint", j.of.clone(), None);
                    });
            }
            Some(Sel::Link(n)) => {
                let Some(l) = view.links.iter().find(|l| l.name == *n) else {
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
                        row(ui, "bodies", l.bodies.join(" "), None);
                        row(
                            ui,
                            "mass",
                            l.mass.map_or("none (massless)".into(), |m| format!("{m:.6} kg")),
                            p.overridden(&what, "mass"),
                        );
                        let c = l.com;
                        row(
                            ui,
                            "centre of mass (mm)",
                            format!("{}, {}, {}", mm(c[0]), mm(c[1]), mm(c[2])),
                            p.overridden(&what, "com"),
                        );
                        row(ui, "visuals", format!("{}", l.visuals.len()), None);
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
                        row(ui, "collision", shapes.join(", "), None);
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
        let _ = Color32::WHITE;
    }
}
