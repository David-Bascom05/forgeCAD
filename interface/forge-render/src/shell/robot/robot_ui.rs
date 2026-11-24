//! The Robot workspace's panel (research/export/urdf-editor.md §6): a robot document's links
//! and joints as a tree from the root, a slider per moving joint (held within its limits; a
//! mimic follows its leader), the selected joint's or link's values with each field marked
//! linked or overridden, and what the export says (its refusal with fixes, warnings, notes).
//! The host (gui_robot.rs) reads `q` and moves the parts; nothing here edits a document yet.

use crate::shell::Shell;
use crate::shell::icons;
use crate::shell::theme::{Theme, medium, mono, sans};
use crate::shell::ux;
use egui::{Align2, RichText, Sense, Stroke, Ui, pos2, vec2};
use forge_urdf::robot::Edit;
use forge_urdf::{Conflict, Exported, RobotView, UrdfError, ViewJoint};
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

/// A change to the robot's tree, on the selected link or joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeOp {
    MakeRoot,
    Drop,
    Merge,
    Freeze,
    Frame,
    Hull,
}

impl TreeOp {
    pub fn label(self) -> &'static str {
        match self {
            TreeOp::MakeRoot => "Make root",
            TreeOp::Drop => "Drop",
            TreeOp::Merge => "Merge into parent",
            TreeOp::Freeze => "Freeze here",
            TreeOp::Frame => "Add frame",
            TreeOp::Hull => "Collision: hull",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            TreeOp::MakeRoot => "groundplane",
            TreeOp::Drop => "delete",
            TreeOp::Merge => "combine",
            TreeOp::Freeze => "jfastened",
            TreeOp::Frame => "ucs",
            TreeOp::Hull => "solids",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            TreeOp::MakeRoot => "the tree hung from this link",
            TreeOp::Drop => "out of the robot: no visual, collision or mass",
            TreeOp::Merge => "one link with its parent (across a fixed joint)",
            TreeOp::Freeze => "a fixed joint in the robot, at its value now",
            TreeOp::Frame => "a tool or sensor frame on this link",
            TreeOp::Hull => "its collision the convex hull of its meshes",
        }
    }
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
    /// The environment's browser drew the tree this frame.
    pub side_drawn: bool,
    /// The 3D view's overlays: joint frames and axes, centres of mass.
    pub show_axes: bool,
    pub show_com: bool,
    pub open: bool,
    /// The view was fitted to the robot since the environment was entered.
    pub framed: bool,
    /// Each joint's name over the view this frame: (name, where in points from the view's
    /// corner, selected); the host puts them at the axis arrows' tips.
    pub labels: Vec<(String, [f32; 2], bool)>,
}

impl RobotPanel {
    pub fn new(file: &str) -> RobotPanel {
        RobotPanel {
            file: file.to_string(),
            open: true,
            show_axes: true,
            show_com: true,
            // the export runs when the host first sees it
            stale: true,
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

    /// The edit a tree operation is on the selection, or why it can't be.
    pub fn tree_edit(&self, op: TreeOp) -> Result<Edit, String> {
        let view = self.view.as_ref().ok_or("the robot is not built yet")?;
        if op == TreeOp::Freeze {
            let Some(Sel::Joint(n)) = &self.sel else {
                return Err("select a joint".into());
            };
            let j = view.joints.iter().find(|j| j.name == *n).ok_or("no such joint")?;
            if j.position.len() != 1 || j.mimic.is_some() {
                return Err("a moving joint of one value".into());
            }
            if j.of != j.name || self.source.is_none() {
                return Err("a source joint of one freedom".into());
            }
            let v = self.value(j);
            let at = if j.kind == "prismatic" {
                format!("{}", (v * 1000.0 * 1e6).round() / 1e6)
            } else {
                format!("{}deg", (v.to_degrees() * 1e6).round() / 1e6)
            };
            return Ok(Edit::Add {
                text: format!("freeze(src.{}, at: {at})", j.of),
            });
        }
        let Some(Sel::Link(n)) = &self.sel else {
            return Err("select a link".into());
        };
        let l = view.links.iter().find(|l| l.name == *n).ok_or("no such link")?;
        let target = self.link_target(l);
        let up = view.joints.iter().find(|j| j.child == l.name);
        let has_kids = view.joints.iter().any(|j| j.parent == l.name);
        Ok(match op {
            TreeOp::MakeRoot => {
                if self.source.is_none() || l.bodies.is_empty() {
                    return Err("a link of the source".into());
                }
                if view.root == l.name {
                    return Err("it is the root".into());
                }
                Edit::Put {
                    call: "root".into(),
                    text: format!("root({target})"),
                }
            }
            TreeOp::Drop => {
                if view.root == l.name || has_kids {
                    return Err("a leaf, not the root".into());
                }
                Edit::Add {
                    text: format!("drop({target})"),
                }
            }
            TreeOp::Merge => {
                let j = up.ok_or("the root has no parent")?;
                if j.kind != "fixed" {
                    return Err("the joint above moves: freeze it first".into());
                }
                let parent = view
                    .links
                    .iter()
                    .find(|x| x.name == j.parent)
                    .ok_or("no parent")?;
                Edit::Add {
                    text: format!("merge({}, {target})", self.link_target(parent)),
                }
            }
            TreeOp::Hull => Edit::Add {
                text: format!("collision({target}, hull)"),
            },
            TreeOp::Frame => {
                let name = (0..)
                    .map(|k| {
                        if k == 0 {
                            "tool".to_string()
                        } else {
                            format!("tool{k}")
                        }
                    })
                    .find(|n| !view.links.iter().any(|x| x.name == *n))
                    .unwrap_or_default();
                Edit::Add {
                    text: format!("{name} = frame(on: {target}, at: pose())"),
                }
            }
            TreeOp::Freeze => unreachable!(),
        })
    }

    /// How the document names a link: `src.B` by a source instance it holds, else its name.
    pub fn link_target(&self, l: &forge_urdf::ViewLink) -> String {
        match l.bodies.first() {
            Some(b) if self.source.is_some() => format!("src.{b}"),
            _ => format!("\"{}\"", l.name),
        }
    }

    fn overridden(&self, what: &str, field: &str) -> Option<&str> {
        self.overrides
            .iter()
            .find(|o| o.1 == what && o.2 == field)
            .map(|o| o.3.as_str())
    }
}

/// Where a value shown came from (docs/ux/STANDARD.md §6).
#[derive(Clone, Debug, PartialEq)]
enum Prov {
    /// Linked: the source assembly's (its mass from its materials).
    Linked,
    /// The robot document's own; with the source's value it replaced, if it had one.
    Yours(Option<String>),
    /// Not given anywhere.
    Unknown,
}

/// `from assembly`'s colour.
const BLUE: egui::Color32 = egui::Color32::from_rgb(59, 130, 246);

/// One badge, one column wide, the hover its detail: the shared widget (`shell::ux::badge`) for
/// the robot's own values and unknown ones; `from assembly` (which it has no word for) drawn
/// the same size, blue.
fn badge(ui: &mut Ui, t: &Theme, b: &Prov) {
    match b {
        Prov::Yours(Some(was)) => {
            ux::badge(
                ui,
                t,
                &ux::Source::User(format!("The robot's override; the assembly has {}.", pretty(was))),
            );
        }
        Prov::Yours(None) => {
            ux::badge(
                ui,
                t,
                &ux::Source::User("The robot's own value: the assembly has none.".into()),
            );
        }
        Prov::Unknown => {
            ux::badge(ui, t, &ux::Source::Unknown);
        }
        Prov::Linked => {
            let (r, resp) = ui.allocate_exact_size(vec2(ux::BADGE_W + 26.0, 16.0), Sense::hover());
            let g = ui
                .painter()
                .layout_no_wrap("from assembly".into(), crate::shell::theme::caption(), BLUE);
            let pill = egui::Rect::from_min_size(r.min + vec2(0.0, 0.5), vec2(g.size().x + 10.0, 15.0));
            ui.painter().rect_filled(pill, 7.5, BLUE.gamma_multiply(0.14));
            ui.painter().galley(
                pos2(pill.left() + 5.0, pill.center().y - g.size().y / 2.0),
                g,
                BLUE,
            );
            resp.on_hover_text("Linked: the source assembly's value.");
        }
    }
}

/// A value in the document's words made readable: `(-135deg, 135deg)` → `−135…135°`.
fn pretty(text: &str) -> String {
    let t = text.trim();
    let inner = t.strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(t);
    let parts: Vec<String> = inner
        .split(',')
        .map(|x| {
            let x = x.trim();
            let (n, deg) = match x.strip_suffix("deg") {
                Some(n) => (n.trim(), true),
                None => (x, false),
            };
            let n = n.replacen('-', "−", 1);
            if deg { format!("{n}°") } else { n }
        })
        .collect();
    match &parts[..] {
        [a, b] => match (a.strip_suffix('°'), b.strip_suffix('°')) {
            (Some(a), Some(_)) => format!("{a}…{b}"),
            _ => format!("{a}…{b}"),
        },
        _ => parts.join(", "),
    }
}

/// A number to the precision it carries, at most `dec` places, at least one.
fn num(x: f64, dec: usize) -> String {
    let s = format!("{x:.dec$}");
    let s = if s.contains('.') {
        let s = s.trim_end_matches('0');
        if s.ends_with('.') {
            format!("{s}0")
        } else {
            s.to_string()
        }
    } else {
        s
    };
    let s = if s == "-0.0" { "0.0".to_string() } else { s };
    s.replacen('-', "−", 1)
}

/// A length given in mm (the view's frames are the source's, in mm), in the user's unit.
fn len(mm: f64, inch: bool) -> String {
    ux::show(mm, ux::Qty::Length, sys(inch))
}

/// The user's unit system (the status bar's mm / in).
fn sys(inch: bool) -> ux::System {
    if inch {
        ux::System::Inch
    } else {
        ux::System::Metric
    }
}

/// An axis as a direction: `+Z` along a frame axis, else its components.
fn axis_text(a: [f64; 3]) -> String {
    for (k, n) in ["X", "Y", "Z"].iter().enumerate() {
        let other = (0..3).filter(|&i| i != k).all(|i| a[i].abs() < 1e-9);
        if other && (a[k].abs() - 1.0).abs() < 1e-9 {
            return format!("{}{n}", if a[k] > 0.0 { "+" } else { "−" });
        }
    }
    format!("({}, {}, {})", num(a[0], 4), num(a[1], 4), num(a[2], 4))
}

/// A joint type's human name.
fn kind_name(k: &str) -> &'static str {
    match k {
        "revolute" => "Revolute",
        "continuous" => "Continuous",
        "prismatic" => "Prismatic",
        "fixed" => "Fixed",
        "floating" => "Floating",
        "planar" => "Planar",
        _ => "Joint",
    }
}

/// A joint type's icon.
fn kind_icon(k: &str) -> &'static str {
    match k {
        "revolute" | "continuous" => "jrevolute",
        "prismatic" => "jslider",
        "fixed" => "jfastened",
        "planar" => "jmate",
        _ => "ball",
    }
}

/// Upper case first.
fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// A titled section of the sheet.
fn section(ui: &mut Ui, t: &Theme, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).font(medium(12.0)).color(t.ink));
    ui.separator();
}

/// A sheet's row: label (its raw key on hover), the value right-aligned in a fixed column, a
/// badge. In a three-column grid.
fn row(ui: &mut Ui, t: &Theme, label: &str, key: &str, value: &str, prov: Option<&Prov>) {
    let l = ui.label(RichText::new(label).font(sans(12.0)).color(t.ink2));
    if !key.is_empty() {
        l.on_hover_text(RichText::new(key).font(mono(11.0)));
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.set_min_width(VALUE_W);
        ui.label(RichText::new(value).font(sans(12.0)).color(t.ink));
    });
    match prov {
        Some(b) => badge(ui, t, b),
        None => {
            ui.label("");
        }
    }
    ui.end_row();
}

/// The sheet's value column.
const VALUE_W: f32 = 120.0;
/// The browser's rows.
const ROW_H: f32 = 22.0;
const INDENT: f32 = 16.0;

/// One of the browser's rows.
struct TreeRow {
    depth: usize,
    sel: Sel,
    name: String,
    icon: &'static str,
    /// At the right edge: a link's mass, a joint's type.
    right: String,
    badge: Option<&'static str>,
    /// Whether a sibling follows it (its tree line goes on down).
    more: bool,
}

impl Shell {
    /// The environment's browser: the robot's links and joints as a tree from the root, with tree
    /// lines, an icon per kind and each link's mass at the right.
    pub(in crate::shell) fn robot_browser(&mut self, ui: &mut Ui, r: egui::Rect) -> bool {
        let Some(mut p) = self.robot.take() else {
            return false;
        };
        let t = self.theme;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        let width = r.width();
        let (head, _) = child.allocate_exact_size(vec2(width, ROW_H + 6.0), Sense::hover());
        icons::draw(
            child.painter(),
            pos2(head.left() + 10.0, head.top() + 6.0),
            16.0,
            "jrevolute",
            1.5,
            t.ink,
        );
        let title = if p.name.is_empty() {
            "Robot".to_string()
        } else {
            p.name.clone()
        };
        child.painter().text(
            pos2(head.left() + 32.0, head.center().y),
            Align2::LEFT_CENTER,
            title,
            medium(13.0),
            t.ink,
        );
        if let Some(v) = &p.view {
            let moving = v.joints.iter().filter(|j| j.position.len() == 1).count();
            child.painter().text(
                pos2(head.right() - 10.0, head.center().y),
                Align2::RIGHT_CENTER,
                format!(
                    "{} links · {moving} moving",
                    v.links.iter().filter(|l| !l.dummy).count()
                ),
                sans(11.0),
                t.ink2,
            );
        }
        match p.view.clone() {
            Some(view) => {
                let rows = tree_rows(&view);
                egui::ScrollArea::vertical()
                    .id_salt("robot-side")
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| {
                        // the tree lines still open at each depth
                        let mut open: Vec<bool> = vec![];
                        for row in &rows {
                            // open[d]: rows at depth d still to come, their line in column d - 1
                            open.truncate(row.depth);
                            open.resize(row.depth, false);
                            let (rr, resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
                            let on = p.sel.as_ref() == Some(&row.sel);
                            let pt = ui.painter();
                            if on {
                                pt.rect_filled(rr, 0.0, t.selection);
                            } else if resp.hovered() {
                                pt.rect_filled(rr, 0.0, t.hover);
                            }
                            let x0 = rr.left() + 12.0;
                            let line = Stroke::new(1.0, t.line);
                            for (d, &o) in open.iter().enumerate().skip(1) {
                                if o {
                                    let x = x0 + (d - 1) as f32 * INDENT + 4.0;
                                    pt.vline(x, rr.y_range(), line);
                                }
                            }
                            if row.depth > 0 {
                                let x = x0 + (row.depth - 1) as f32 * INDENT + 4.0;
                                let bottom = if row.more { rr.bottom() } else { rr.center().y };
                                pt.vline(x, rr.top()..=bottom, line);
                                pt.hline(x..=x + INDENT - 6.0, rr.center().y, line);
                            }
                            let ix = x0 + row.depth as f32 * INDENT;
                            icons::draw(pt, pos2(ix, rr.top() + 3.0), 16.0, row.icon, 1.3, t.ink);
                            let mut right_x = rr.right() - 10.0;
                            if !row.right.is_empty() {
                                let g = pt.layout_no_wrap(row.right.clone(), sans(11.0), t.ink2);
                                pt.galley(
                                    pos2(right_x - g.size().x, rr.center().y - g.size().y / 2.0),
                                    g.clone(),
                                    t.ink2,
                                );
                                right_x -= g.size().x + 8.0;
                            }
                            if let Some(b) = row.badge {
                                let g = pt.layout_no_wrap(b.to_string(), sans(10.0), t.ink2);
                                let pill = egui::Rect::from_min_size(
                                    pos2(right_x - g.size().x - 10.0, rr.center().y - 8.0),
                                    vec2(g.size().x + 10.0, 16.0),
                                );
                                pt.rect_filled(pill, 8.0, t.ink2.gamma_multiply(0.14));
                                pt.galley(
                                    pos2(pill.left() + 5.0, pill.center().y - g.size().y / 2.0),
                                    g,
                                    t.ink2,
                                );
                                right_x = pill.left() - 6.0;
                            }
                            let name_x = ix + 22.0;
                            let room = (right_x - name_x).max(20.0);
                            let job = egui::text::LayoutJob {
                                wrap: egui::text::TextWrapping::truncate_at_width(room),
                                ..egui::text::LayoutJob::simple_singleline(
                                    row.name.clone(),
                                    sans(12.0),
                                    t.ink,
                                )
                            };
                            let g = ui.fonts_mut(|f| f.layout_job(job));
                            let cut = g.elided;
                            ui.painter()
                                .galley(pos2(name_x, rr.center().y - g.size().y / 2.0), g, t.ink);
                            let resp = if cut { resp.on_hover_text(&row.name) } else { resp };
                            if resp.clicked() {
                                p.sel = if on { None } else { Some(row.sel.clone()) };
                            }
                            self.places.push((
                                match &row.sel {
                                    Sel::Link(n) => format!("robot:link:{n}"),
                                    Sel::Joint(n) => format!("robot:joint:{n}"),
                                },
                                rr,
                            ));
                            open.push(row.more);
                        }
                    });
            }
            None => {
                child.add_space(8.0);
                child.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.spinner();
                    ui.label(
                        RichText::new("Building the robot…")
                            .font(sans(12.0))
                            .color(t.ink2),
                    );
                });
            }
        }
        p.side_drawn = true;
        self.robot = Some(p);
        true
    }

    /// The environment's property sheet, docked at the right edge, full height, scrolling: the
    /// robot and its link, the selected link's or joint's sheet, the drive, the checks.
    pub(in crate::shell) fn robot_properties(&mut self, ui: &mut Ui, r: egui::Rect) {
        let Some(mut p) = self.robot.take() else { return };
        let t = self.theme;
        let inch = self.units == "in";
        let pt = ui.painter().clone();
        pt.rect_filled(r, 0.0, t.surface);
        pt.vline(r.left(), r.y_range(), Stroke::new(1.0, t.line));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(12.0, 8.0))));
        child.set_clip_rect(r);
        egui::ScrollArea::vertical()
            .id_salt("robot-dock")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                // the robot: its name, what it is linked to
                let title = if p.name.is_empty() {
                    "Robot".to_string()
                } else {
                    p.name.clone()
                };
                ui.label(RichText::new(title).font(medium(15.0)).color(t.ink));
                let doc = std::path::Path::new(&p.file)
                    .file_name()
                    .map_or_else(|| p.file.clone(), |n| n.to_string_lossy().into_owned());
                match &p.source {
                    Some(s) => ui.label(
                        RichText::new(format!("Linked to {s} · {doc}"))
                            .font(sans(11.0))
                            .color(t.ink2),
                    ),
                    None => ui.label(
                        RichText::new(format!("Unlinked · {doc}"))
                            .font(sans(11.0))
                            .color(t.ink2),
                    ),
                };
                if !p.said.is_empty() {
                    ui.label(RichText::new(&p.said).font(sans(11.0)).color(t.ink2));
                }
                Self::robot_problems(ui, &t, &mut p);
                match p.view.clone() {
                    None => {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new("Building the robot…")
                                    .font(sans(12.0))
                                    .color(t.ink2),
                            );
                        });
                    }
                    Some(view) => {
                        Self::robot_detail(ui, &t, &mut p, &view, inch);
                        Self::robot_sliders(ui, &t, &mut p, &view, inch);
                        if !p.warnings.is_empty() || !p.notes.is_empty() {
                            section(ui, &t, "Checks");
                            for w in &p.warnings {
                                ui.label(RichText::new(w).font(sans(11.0)).color(t.warn));
                            }
                            for n in &p.notes {
                                ui.label(RichText::new(n).font(sans(11.0)).color(t.ink2));
                            }
                        }
                    }
                }
            });
        self.places.push(("robot:sheet".into(), r));
        p.side_drawn = false;
        self.robot = Some(p);
    }

    /// Its properties panel's width (the Environment trait's `properties_width`): docked right
    /// while the environment is up.
    pub fn robot_properties_width(&self) -> Option<f32> {
        self.robot_on().then_some(crate::shell::ROBOT_PROPS_W)
    }

    /// Its line in the status bar (the trait's `status`): what you are in, and the robot.
    pub fn robot_status(&self) -> Option<String> {
        let p = self.robot.as_ref().filter(|p| p.open)?;
        let name = if p.name.is_empty() {
            "robot"
        } else {
            p.name.as_str()
        };
        let what = match &p.view {
            Some(v) => {
                let links = v.links.iter().filter(|l| !l.dummy).count();
                let moving = v.joints.iter().filter(|j| j.position.len() == 1).count();
                format!("{links} links, {moving} moving joints")
            }
            None => "building".into(),
        };
        let state = if !p.conflicts.is_empty() {
            format!(" · {} changed in the assembly", p.conflicts.len())
        } else if p.error.is_some() {
            " · not exportable: see the sheet".into()
        } else {
            String::new()
        };
        let src = p
            .source
            .as_ref()
            .map_or_else(|| " · unlinked".to_string(), |s| format!(" · linked to {s}"));
        Some(format!(
            "Robot · {name} · {what}{src}{state} · Finish Robot returns to the assembly"
        ))
    }

    /// What it draws over the view (the Environment trait's `draw`): the joints' names at their
    /// axis arrows' tips. It takes none of the view's pointer.
    pub(in crate::shell) fn robot_draw(&mut self, ui: &mut Ui, view: egui::Rect) -> bool {
        let Some(p) = &self.robot else { return false };
        if !p.open || p.labels.is_empty() {
            return false;
        }
        let t = self.theme;
        let v = view;
        let pt = ui.painter().with_clip_rect(v);
        for (name, at, chosen) in &p.labels {
            let a = v.min + vec2(at[0], at[1]);
            let ink = if *chosen { t.accent } else { t.ink };
            let g = pt.layout_no_wrap(name.clone(), sans(11.0), ink);
            let r = egui::Rect::from_min_size(a + vec2(6.0, -g.size().y - 6.0), g.size() + vec2(8.0, 4.0));
            pt.rect_filled(r, 3.0, t.surface.gamma_multiply(0.85));
            pt.galley(r.min + vec2(4.0, 2.0), g, ink);
        }
        false
    }

    /// The export's refusal, inline: conflicts as sentences each with its two answers, any other
    /// refusal in its words with its fixes.
    fn robot_problems(ui: &mut Ui, t: &Theme, p: &mut RobotPanel) {
        let src = p.source.clone().unwrap_or_else(|| "the source".into());
        if !p.conflicts.is_empty() {
            section(ui, t, "Changed in the assembly");
            for c in p.conflicts.clone() {
                let name = c
                    .what
                    .split_once('(')
                    .map_or(c.what.as_str(), |(_, x)| x.trim_end_matches(')'));
                let sentence = format!(
                    "{} {} changed in {src} from {} to {}. The robot overrides it with {}.",
                    cap(name),
                    c.field,
                    pretty(&c.was),
                    pretty(&c.now),
                    pretty(&c.set)
                );
                egui::Frame::new()
                    .fill(t.warn.gamma_multiply(0.12))
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(sentence).font(sans(12.0)).color(t.ink))
                            .on_hover_text(format!(
                                "line {} of the robot document: {} {}",
                                c.line, c.what, c.field
                            ));
                        ui.horizontal(|ui| {
                            if ui
                                .button("Keep override")
                                .on_hover_text(format!(
                                    "keep {} over the assembly's new {}",
                                    pretty(&c.set),
                                    pretty(&c.now)
                                ))
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
                                .button("Use source")
                                .on_hover_text(format!("drop the override: {} from {src}", pretty(&c.now)))
                                .clicked()
                            {
                                p.out.push(RobotAct::Edit(Edit::Reset {
                                    line: c.line,
                                    field: c.field.clone(),
                                }));
                            }
                        });
                    });
            }
        } else if let Some((kind, text, fixes)) = p.error.clone() {
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(t.error.gamma_multiply(0.12))
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(text).font(sans(12.0)).color(t.ink))
                        .on_hover_text(RichText::new(kind).font(mono(11.0)));
                    for f in fixes {
                        ui.label(RichText::new(format!("Fix: {f}")).font(sans(11.0)).color(t.ink2));
                    }
                });
        }
    }

    /// The drive: a slider per moving joint (a mimic follows its leader), in degrees or the
    /// user's length unit.
    fn robot_sliders(ui: &mut Ui, t: &Theme, p: &mut RobotPanel, view: &RobotView, inch: bool) {
        let moving: Vec<&ViewJoint> = view.joints.iter().filter(|j| j.position.len() == 1).collect();
        if moving.is_empty() {
            return;
        }
        section(ui, t, "Drive");
        egui::Grid::new("robot-drive")
            .num_columns(2)
            .spacing(vec2(8.0, 4.0))
            .show(ui, |ui| {
                for j in moving {
                    let angular = j.kind != "prismatic";
                    let (lo, hi) = j.limit.unwrap_or(if angular {
                        (-std::f64::consts::PI, std::f64::consts::PI)
                    } else {
                        (-0.1, 0.1)
                    });
                    let k = if angular {
                        180.0 / std::f64::consts::PI
                    } else if inch {
                        1.0 / 0.0254
                    } else {
                        1000.0
                    };
                    let label = if j.mimic.is_some() {
                        format!("{} (mimic)", j.name)
                    } else {
                        j.name.clone()
                    };
                    let l = ui.label(RichText::new(label).font(sans(12.0)).color(t.ink));
                    if let Some((leader, m, off)) = &j.mimic {
                        l.on_hover_text(format!("follows {leader} × {} + {}", num(*m, 4), num(*off, 4)));
                    }
                    let mut shown = p.value(j) * k;
                    let s = egui::Slider::new(&mut shown, lo * k..=hi * k)
                        .suffix(match (angular, inch) {
                            (true, _) => "°",
                            (false, true) => " in",
                            _ => " mm",
                        })
                        .fixed_decimals(1);
                    let r = ui.add_enabled(j.mimic.is_none(), s);
                    // (a slider says changed when it only shows the value again: moved only
                    // when the value is another)
                    let was = p.value(j);
                    if r.changed() && (shown / k - was).abs() > 1e-12 * (1.0 + was.abs()) {
                        p.set(&j.name, shown / k);
                    }
                    ui.end_row();
                }
            });
    }

    /// The selected joint's or link's sheet: titled sections, every value with its unit and where
    /// it came from.
    fn robot_detail(ui: &mut Ui, t: &Theme, p: &mut RobotPanel, view: &RobotView, inch: bool) {
        match p.sel.clone() {
            Some(Sel::Joint(n)) => {
                if let Some(j) = view.joints.iter().find(|j| j.name == n).cloned() {
                    Self::joint_sheet(ui, t, p, view, &j, inch);
                }
            }
            Some(Sel::Link(n)) => {
                if let Some(l) = view.links.iter().find(|l| l.name == n).cloned() {
                    Self::link_sheet(ui, t, p, view, &l, inch);
                }
            }
            None => {
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Select a link or a joint in the browser for its properties.")
                        .font(sans(12.0))
                        .color(t.ink2),
                );
            }
        }
    }

    fn joint_sheet(ui: &mut Ui, t: &Theme, p: &mut RobotPanel, _view: &RobotView, j: &ViewJoint, inch: bool) {
        let what = format!("joint({})", j.name);
        let line = p.line_of(&what);
        let key = |f: &str| match line {
            Some(l) => format!("{what}.{f} · line {l}"),
            None => format!("{what}.{f}"),
        };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            icons::draw(
                ui.painter(),
                ui.cursor().min + vec2(0.0, 1.0),
                18.0,
                kind_icon(j.kind),
                1.4,
                t.ink,
            );
            ui.add_space(24.0);
            ui.label(RichText::new(&j.name).font(medium(14.0)).color(t.ink));
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("{} joint", kind_name(j.kind)))
                    .font(sans(12.0))
                    .color(t.ink2),
            );
        });
        let angular = j.kind != "prismatic";
        section(ui, t, "Joint");
        egui::Grid::new("robot-joint")
            .num_columns(3)
            .spacing(vec2(10.0, 4.0))
            .show(ui, |ui| {
                row(
                    ui,
                    t,
                    "Type",
                    &key("type"),
                    kind_name(j.kind),
                    Some(&Prov::Linked),
                );
                row(ui, t, "Parent link", "parent", &j.parent, None);
                row(ui, t, "Child link", "child", &j.child, None);
                if !j.of.is_empty() && j.of != j.name {
                    row(ui, t, "Source joint", "of", &j.of, None);
                }
                if let Some((l, m, off)) = &j.mimic {
                    row(
                        ui,
                        t,
                        "Follows",
                        "mimic",
                        &format!("{l} × {} + {}", num(*m, 4), num(*off, 4)),
                        Some(&Prov::Linked),
                    );
                }
            });
        section(ui, t, "Placement");
        egui::Grid::new("robot-joint-at")
            .num_columns(3)
            .spacing(vec2(10.0, 4.0))
            .show(ui, |ui| {
                let o = j.origin.origin;
                for (a, v) in [("X", o.x), ("Y", o.y), ("Z", o.z)] {
                    row(
                        ui,
                        t,
                        &format!("Origin {a}"),
                        "origin xyz (in the parent link)",
                        &len(v, inch),
                        None,
                    );
                }
                if let Some(a) = j.axis {
                    row(ui, t, "Axis", "axis (in the joint's frame)", &axis_text(a), None);
                }
            });
        // limits: shown in degrees or the user's unit, sent in the document's
        if matches!(j.kind, "revolute" | "prismatic" | "continuous") {
            let k = if angular {
                180.0 / std::f64::consts::PI
            } else if inch {
                1.0 / 0.0254
            } else {
                1000.0
            };
            let to_doc = |x: f64| {
                if angular {
                    format!("{}deg", (x * 1e6).round() / 1e6)
                } else {
                    // the document's lengths are mm
                    let mm = if inch { x * 25.4 } else { x };
                    format!("{}", (mm * 1e6).round() / 1e6)
                }
            };
            let txt = |lo: f64, hi: f64| format!("({}, {})", to_doc(lo), to_doc(hi));
            let unit = if angular {
                "°"
            } else if inch {
                " in"
            } else {
                " mm"
            };
            let (lo0, hi0) = j.limit.map_or((-180.0, 180.0), |(a, b)| (a * k, b * k));
            // a limit as shown: an angle by the shared units, a length in the unit it is in
            let lim = |x: f64| {
                if angular {
                    ux::show(x, ux::Qty::Angle, sys(inch))
                } else {
                    format!("{}{unit}", num(x, 3))
                }
            };
            let over = p.overridden(&what, "limit").map(str::to_string);
            let prov = match &over {
                Some(w) if w.is_empty() => Prov::Yours(None),
                Some(w) => Prov::Yours(Some(w.clone())),
                None if j.limit.is_some() => Prov::Linked,
                None => Prov::Unknown,
            };
            section(ui, t, "Limits");
            egui::Grid::new("robot-joint-lim")
                .num_columns(3)
                .spacing(vec2(10.0, 4.0))
                .show(ui, |ui| match j.limit {
                    Some(_) => {
                        row(ui, t, "Lower", &key("limit"), &lim(lo0), Some(&prov));
                        row(ui, t, "Upper", &key("limit"), &lim(hi0), Some(&prov));
                    }
                    None => row(ui, t, "Range", &key("limit"), "none (turns freely)", Some(&prov)),
                });
            let lo_key = format!("{}.lo", j.name);
            let hi_key = format!("{}.hi", j.name);
            let mut lo = *p.draft.get(&lo_key).unwrap_or(&lo0);
            let mut hi = *p.draft.get(&hi_key).unwrap_or(&hi0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::DragValue::new(&mut lo).speed(0.5).suffix(unit))
                    .changed()
                {
                    p.draft.insert(lo_key.clone(), lo);
                }
                ui.label(RichText::new("to").font(sans(12.0)).color(t.ink2));
                if ui
                    .add(egui::DragValue::new(&mut hi).speed(0.5).suffix(unit))
                    .changed()
                {
                    p.draft.insert(hi_key.clone(), hi);
                }
                let ok = lo <= hi;
                if ui
                    .add_enabled(ok, egui::Button::new("Override"))
                    .on_hover_text("the robot's own limits; the assembly keeps its")
                    .on_disabled_hover_text("lower above upper")
                    .clicked()
                {
                    let was = j.limit.map(|_| txt(lo0, hi0));
                    p.set_joint(j, "limit", txt(lo, hi), was);
                    p.draft.remove(&lo_key);
                    p.draft.remove(&hi_key);
                }
            });
            if over.is_some()
                && let Some(l) = line
            {
                ui.horizontal(|ui| {
                    if ui
                        .button("Use source")
                        .on_hover_text("drop the robot's limits")
                        .clicked()
                    {
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
                            .on_hover_text(
                                "write these limits in the source assembly, and drop the robot's override",
                            )
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
        if j.position.len() == 1 {
            section(ui, t, "Actuation");
            let (eu, vu, du, fu) = if angular {
                ("N·m", "rad/s", "N·m·s/rad", "N·m")
            } else {
                ("N", "m/s", "N·s/m", "N")
            };
            egui::Grid::new("robot-act")
                .num_columns(3)
                .spacing(vec2(10.0, 4.0))
                .show(ui, |ui| {
                    for (field, label, unit) in [
                        ("effort", "Effort", eu),
                        ("velocity", "Velocity", vu),
                        ("damping", "Damping", du),
                        ("friction", "Friction", fu),
                    ] {
                        let own = p.overrides.iter().any(|o| o.1 == what && o.2 == field);
                        let written = match field {
                            "effort" => j.effort,
                            "velocity" => j.velocity,
                            "damping" => j.dynamics.map(|d| d.0),
                            _ => j.dynamics.map(|d| d.1),
                        };
                        let prov = match (own, written) {
                            (true, _) => Prov::Yours(None),
                            (false, Some(_)) => Prov::Yours(None),
                            (false, None) => Prov::Unknown,
                        };
                        let k = format!("{what}.{field}");
                        let l = ui.label(RichText::new(label).font(sans(12.0)).color(t.ink2));
                        l.on_hover_text(
                            RichText::new(if own {
                                k.clone()
                            } else {
                                format!("{k} · actuation(..)")
                            })
                            .font(mono(11.0)),
                        );
                        let dk = format!("{}.{field}", j.name);
                        let mut v = *p.draft.get(&dk).unwrap_or(&written.unwrap_or(0.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.set_min_width(VALUE_W);
                            if ui
                                .add(
                                    egui::DragValue::new(&mut v)
                                        .speed(0.1)
                                        .range(0.0..=f64::MAX)
                                        .suffix(format!(" {unit}")),
                                )
                                .changed()
                            {
                                p.draft.insert(dk.clone(), v);
                            }
                        });
                        ui.horizontal(|ui| {
                            badge(ui, t, &prov);
                            let dirty = p.draft.contains_key(&dk);
                            if dirty
                                && ui
                                    .small_button("Set")
                                    .on_hover_text("the robot's own value")
                                    .clicked()
                            {
                                p.set_joint(j, field, format!("{v}"), None);
                                p.draft.remove(&dk);
                            }
                            if own
                                && !dirty
                                && let Some(l) = line
                                && ui
                                    .small_button("Clear")
                                    .on_hover_text("drop the joint's own value")
                                    .clicked()
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
        // fixed at its value now: a freeze, the robot's own (merging across it needs one)
        if let Ok(e) = p.tree_edit(TreeOp::Freeze) {
            ui.add_space(6.0);
            if ui
                .button(TreeOp::Freeze.label())
                .on_hover_text(TreeOp::Freeze.tip())
                .clicked()
            {
                p.out.push(RobotAct::Edit(e));
            }
        }
    }

    fn link_sheet(
        ui: &mut Ui,
        t: &Theme,
        p: &mut RobotPanel,
        view: &RobotView,
        l: &forge_urdf::ViewLink,
        inch: bool,
    ) {
        let what = format!("inertial({})", l.name);
        let line = p.line_of(&what);
        let key = |f: &str| match line {
            Some(n) => format!("{what}.{f} · line {n}"),
            None => format!("{what}.{f}"),
        };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            icons::draw(
                ui.painter(),
                ui.cursor().min + vec2(0.0, 1.0),
                18.0,
                "part",
                1.4,
                t.ink,
            );
            ui.add_space(24.0);
            ui.label(RichText::new(&l.name).font(medium(14.0)).color(t.ink));
            ui.add_space(6.0);
            let kind = if view.root == l.name {
                "Root link"
            } else if l.dummy {
                "Massless link"
            } else {
                "Link"
            };
            ui.label(RichText::new(kind).font(sans(12.0)).color(t.ink2));
        });
        if !l.bodies.is_empty() {
            section(ui, t, "Bodies");
            for b in &l.bodies {
                ui.label(RichText::new(b).font(sans(12.0)).color(t.ink));
            }
        }
        section(ui, t, "Mass properties");
        let over = p.overridden(&what, "mass").map(str::to_string);
        let prov = match &over {
            Some(w) if w.is_empty() => Prov::Yours(None),
            Some(w) => Prov::Yours(Some(format!("{w} kg"))),
            None if l.dummy => Prov::Linked,
            None if l.mass.is_some() => Prov::Linked,
            None => Prov::Unknown,
        };
        egui::Grid::new("robot-link")
            .num_columns(3)
            .spacing(vec2(10.0, 4.0))
            .show(ui, |ui| {
                let m = match l.mass {
                    Some(m) => ux::show(m, ux::Qty::Mass, sys(inch)),
                    None if l.dummy => "none (massless)".into(),
                    None => "unknown".into(),
                };
                row(ui, t, "Mass", &key("mass"), &m, Some(&prov));
                if !l.dummy {
                    let c = l.com;
                    for (a, v) in [("X", c[0]), ("Y", c[1]), ("Z", c[2])] {
                        row(
                            ui,
                            t,
                            &format!("Centre of mass {a}"),
                            "inertial origin (in the link)",
                            &len(v, inch),
                            Some(&prov),
                        );
                    }
                }
            });
        if !l.dummy {
            let dk = format!("{}.mass", l.name);
            let m0 = l.mass.unwrap_or(0.0);
            let mut m = *p.draft.get(&dk).unwrap_or(&m0);
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
                    p.draft.insert(dk.clone(), m);
                }
                if ui
                    .button("Override")
                    .on_hover_text("the robot's own mass; the assembly keeps its")
                    .clicked()
                {
                    p.out.push(RobotAct::Edit(Edit::Set {
                        line,
                        call: "inertial".into(),
                        target: p.link_target(l),
                        field: "mass".into(),
                        text: format!("{m}"),
                        was: if over.is_some() {
                            None
                        } else {
                            Some(format!("{m0}"))
                        },
                    }));
                    p.draft.remove(&dk);
                }
                if over.is_some()
                    && let Some(n) = line
                    && ui
                        .button("Use source")
                        .on_hover_text("the mass from the parts' materials")
                        .clicked()
                {
                    p.out.push(RobotAct::Edit(Edit::Reset {
                        line: n,
                        field: "mass".into(),
                    }));
                }
            });
        }
        section(ui, t, "Geometry");
        egui::Grid::new("robot-link-geo")
            .num_columns(3)
            .spacing(vec2(10.0, 4.0))
            .show(ui, |ui| {
                row(ui, t, "Visuals", "visual", &format!("{}", l.visuals.len()), None);
                let shapes: Vec<String> = l
                    .collisions
                    .iter()
                    .map(|c| match &c.shape {
                        forge_urdf::Shape::Mesh { file, .. } => {
                            if file.contains("hull") {
                                "convex hull".to_string()
                            } else {
                                "mesh".to_string()
                            }
                        }
                        forge_urdf::Shape::Box(_) => "box".into(),
                        forge_urdf::Shape::Cylinder { .. } => "cylinder".into(),
                        forge_urdf::Shape::Sphere(_) => "sphere".into(),
                    })
                    .collect();
                let c = if shapes.is_empty() {
                    "none".to_string()
                } else {
                    shapes.join(", ")
                };
                row(ui, t, "Collision", "collision", &c, None);
            });
        section(ui, t, "Structure");
        ui.horizontal_wrapped(|ui| {
            for op in [
                TreeOp::MakeRoot,
                TreeOp::Drop,
                TreeOp::Merge,
                TreeOp::Hull,
                TreeOp::Frame,
            ] {
                let e = p.tree_edit(op);
                let r = ui.add_enabled(e.is_ok(), egui::Button::new(op.label()));
                let r = match &e {
                    Ok(_) => r.on_hover_text(op.tip()),
                    Err(why) => r.on_disabled_hover_text(why.as_str()),
                };
                if r.clicked()
                    && let Ok(e) = e
                {
                    p.out.push(RobotAct::Edit(e));
                }
            }
        });
    }
}

/// The browser's rows: depth first from the root, each joint then the link it carries, in
/// document order.
fn tree_rows(view: &RobotView) -> Vec<TreeRow> {
    let mut rows = vec![];
    // (link, depth, more siblings after it)
    fn link(view: &RobotView, name: &str, depth: usize, more: bool, rows: &mut Vec<TreeRow>) {
        let l = view.links.iter().find(|l| l.name == name);
        let (right, badge) = match l {
            Some(l) if l.dummy => (String::new(), Some("massless")),
            Some(l) => (
                l.mass.map_or(String::new(), |m| format!("{} kg", num(m, 3))),
                None,
            ),
            None => (String::new(), None),
        };
        let badge = if view.root == name { Some("root") } else { badge };
        rows.push(TreeRow {
            depth,
            sel: Sel::Link(name.to_string()),
            name: name.to_string(),
            icon: "part",
            right,
            badge,
            more,
        });
        let kids: Vec<&ViewJoint> = view.joints.iter().filter(|j| j.parent == name).collect();
        for (i, j) in kids.iter().enumerate() {
            let last = i + 1 == kids.len();
            rows.push(TreeRow {
                depth: depth + 1,
                sel: Sel::Joint(j.name.clone()),
                name: j.name.clone(),
                icon: kind_icon(j.kind),
                right: if j.mimic.is_some() {
                    format!("{} · mimic", j.kind)
                } else {
                    j.kind.to_string()
                },
                badge: None,
                more: !last,
            });
            link(view, &j.child, depth + 2, false, rows);
        }
    }
    link(view, &view.root, 0, false, &mut rows);
    rows
}
