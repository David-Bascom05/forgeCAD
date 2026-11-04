//! A Forge (.fg) part's chrome: the tree of its feature ids, its params and dimensions, its
//! errors and lints with their fixes. Like the rest of the shell it changes nothing itself: every
//! edit leaves as a typed op in `out`, which the host applies through forge-lang.

use super::theme::{medium, mono, sans, semi};
use super::{Shell, icons, panel};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_lang::error::{Literal, Op, ParamKey, Scope, parse_path, path_text};
use forge_lang::syntax::Seg;
use forge_lang::values::Dim;
use std::collections::HashSet;

pub(super) const PANEL_W: f32 = 340.0;
pub(super) const WARN: Color32 = Color32::from_rgb(0xE0, 0xA0, 0x30);

/// Errors, and everything else (warnings, lints, a shop's rules); an assembly's notes (intended
/// contacts) are neither.
pub fn issue_counts(issues: &[Issue]) -> (usize, usize) {
    let e = issues.iter().filter(|i| i.severity == Severity::Error).count();
    (e, issues.iter().filter(|i| !is_note(i)).count() - e)
}

pub fn is_note(i: &Issue) -> bool {
    i.asm
        .as_ref()
        .is_some_and(|a| a.level() == forge_lang::asm::Level::Info)
}

pub fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

pub(super) fn eye_rect(row: Rect) -> Rect {
    Rect::from_center_size(pos2(row.right() - 16.0, row.center().y), vec2(20.0, 20.0))
}

/// An eye at a row's right, apart from the row's own click: true when it's clicked.
pub(super) fn row_eye(ui: &mut Ui, t: &super::theme::Theme, row: Rect, id: &str, shown: bool) -> bool {
    let r = eye_rect(row);
    let resp = ui
        .interact(r, ui.scope_id().with(id), Sense::click())
        .on_hover_text(if shown { "Hide" } else { "Show" });
    let ink = if resp.hovered() {
        t.ink
    } else if shown {
        t.ink2
    } else {
        t.ink2.gamma_multiply(0.5)
    };
    icons::draw(
        ui.painter(),
        pos2(r.left() + 2.0, r.top() + 2.0),
        16.0,
        if shown { "eye" } else { "hide" },
        1.25,
        ink,
    );
    resp.clicked()
}

/// A disclosure triangle, drawn (the fonts have no arrows): pointing right shut, down open.
pub(super) fn disclosure(p: &egui::Painter, c: egui::Pos2, open: bool, ink: Color32) {
    let pts = if open {
        vec![
            pos2(c.x - 4.0, c.y - 2.0),
            pos2(c.x + 4.0, c.y - 2.0),
            pos2(c.x, c.y + 3.0),
        ]
    } else {
        vec![
            pos2(c.x - 2.0, c.y - 4.0),
            pos2(c.x + 3.0, c.y),
            pos2(c.x - 2.0, c.y + 4.0),
        ]
    };
    p.add(egui::Shape::convex_polygon(pts, ink, Stroke::NONE));
}

/// A section's heading that opens and shuts it: a triangle and the words, the whole line a button.
pub(super) fn fold_head(
    ui: &mut Ui,
    t: &super::theme::Theme,
    head: &str,
    open: bool,
    ink: Color32,
) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 3.0, t.hover);
    }
    disclosure(ui.painter(), pos2(r.left() + 7.0, r.center().y), open, t.ink2);
    ui.painter().text(
        pos2(r.left() + 18.0, r.center().y),
        Align2::LEFT_CENTER,
        head,
        semi(12.0),
        ink,
    );
    resp
}
const INDENT: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Failed,
    NotBuilt,
}

/// Where a feature's number came from.
#[derive(Clone, Debug, PartialEq)]
pub enum From {
    Const,
    Driving(ParamKey),
    Driven(Vec<ParamKey>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimRow {
    pub what: String,
    /// In the units a person reads (degrees for angles).
    pub value: f64,
    pub dim: Dim,
    pub from: From,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeatRow {
    pub id: String,
    pub kind: String,
    pub line: usize,
    pub status: Status,
    pub dims: Vec<DimRow>,
    pub reads: Vec<ParamKey>,
    /// A sketch's or a work plane's plane, as built.
    pub frame: Option<forge_math::Frame>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamRow {
    pub key: ParamKey,
    /// A number's value in the units a person reads; none for a list or tuple.
    pub value: Option<(f64, Dim)>,
    /// Its right side as written, for what isn't one number.
    pub text: String,
    /// The params its definition reads: it's driven by them, with no handle of its own.
    pub links: Vec<ParamKey>,
    pub used_by: Vec<String>,
    /// Evaluation observed it: editing it re-runs.
    pub structural: bool,
    pub line: usize,
}

impl ParamRow {
    /// One number written as a literal: SetParam is its handle.
    pub fn driving(&self) -> bool {
        self.links.is_empty() && self.value.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Lint,
    Warning,
    /// A shop's rule broken (sheet metal): built, but not to be made as it is.
    Violation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub kind: String,
    pub text: String,
    pub line: usize,
    pub col: usize,
    pub fixes: Vec<Op>,
    /// The feature it's in, when one is.
    pub feature: Option<String>,
    /// An assembly's result as forge-lang types it: what to highlight (a conflict's set, the
    /// two parts that interfere).
    pub asm: Option<forge_lang::asm::Issue>,
}

/// What the panel asks of the host.
#[derive(Clone, Debug, PartialEq)]
pub enum FgAct {
    Apply(Op),
    Save,
    /// The sheet's flat pattern, as DXF beside the part's file.
    ExportDxf(String),
    /// Show the flat pattern, or the folded part.
    Flat(bool),
    /// A tool's statement: applied, and the tool closed if it was.
    Tool(Op),
    /// What only an .fc part can do, asked of a Forge part: said, typed.
    Refuse(super::fgtool::ToolError),
    /// An assembly's view changed (selection, hidden, isolated, highlighted): redraw its parts.
    AsmShow,
    /// Run the assembly's overlap check.
    AsmCheck,
    /// A part file's anchors, for the instance chosen.
    AsmAnchors(String),
    /// An assembly's edit as one undo step: a dialog's OK, a menu's Delete.
    AsmOps(Vec<Op>),
    /// A dialog's ops built in the file's place, the file kept; None: the file again.
    AsmPreview(Option<Vec<Op>>),
    /// Place a component from this file (from the assembly's folder), or another like `like`.
    AsmPlace {
        file: String,
        like: Option<String>,
    },
    /// The system's file dialog for a component to place.
    AsmBrowse,
    /// Place's standard parts (screws, nuts, washers..): the host's hook (`Gui::asm_standard_part`).
    AsmStandardPart,
    /// `count` occurrences of a top-level instance, `step` apart in the world.
    AsmPattern {
        inst: String,
        count: usize,
        step: [f64; 3],
    },
    /// Free Move's moves written as the instances' poses.
    AsmApplyMoves,
    /// Mirror (in the assembly's plane: 0 XY, 1 XZ, 2 YZ) or copy (None) a top-level part: as a
    /// new part file of this name beside it, or reusing its own (None).
    AsmMirror {
        inst: String,
        plane: Option<usize>,
        file: Option<String>,
    },
    /// A part file's snap points, for the tool wanting an origin.
    AsmSnaps(String),
}

/// A sheet metal part: its id, its bends, and why it has no flat pattern if it hasn't.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetRow {
    pub id: String,
    pub bends: usize,
    pub refused: Option<String>,
    /// Its report: thickness, radius, rule..., and lists (bends, reliefs, corners...).
    pub facts: Vec<(String, String)>,
    pub lists: Vec<(String, Vec<String>)>,
}

/// One row of the tree: a feature, or a loop's or a call's group of them.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeRow {
    /// The id it stands for: a feature's, or the prefix its group shares.
    pub key: String,
    pub label: String,
    pub depth: usize,
    pub feat: Option<usize>,
    /// It has rows under it.
    pub group: bool,
}

#[derive(Default)]
pub struct FgPanel {
    pub file: String,
    /// Its text declares an assembly.
    pub assembly: bool,
    pub source: String,
    /// What's on screen differs from the file.
    pub dirty: bool,
    pub feats: Vec<FeatRow>,
    pub params: Vec<ParamRow>,
    pub issues: Vec<Issue>,
    /// The part's measures and the rebuild's times, in a line.
    pub summary: String,
    pub selected: Option<String>,
    /// A param's value being typed.
    pub editing: Option<(ParamKey, String)>,
    pub edit_fresh: bool,
    /// A feature's statement being typed, to add above the selected one.
    pub add_text: String,
    /// Every issue listed, not the first few.
    pub all_issues: bool,
    /// The issues list opened or shut by a click; unset, it's open while there are errors.
    pub issues_shown: Option<bool>,
    /// The selected statement's new name, being typed.
    pub rename: String,
    /// The last op the host refused, and why.
    pub refused: Option<String>,
    pub collapsed: HashSet<String>,
    pub show_source: bool,
    pub sheets: Vec<SheetRow>,
    /// The flat pattern is what's shown.
    pub flat: bool,
    pub out: Vec<FgAct>,
    /// A ribbon tool's form, taking picks.
    pub tool: Option<super::fgtool::FgTool>,
    /// The part's sketches and their entities, for picks a view click can't make.
    pub sketches: Vec<forge_lang::anchors::SketchRefs>,
    /// Sketch and work plane rows shown or hidden by their eye; unset, they follow Planes.
    pub eyes: std::collections::HashMap<String, bool>,
    /// The part's solid hidden by the Solid row's eye.
    pub body_hidden: bool,
    /// An assembly's instances, joints, checks and BOM, when the document is one.
    pub asm: Option<super::fgasm::AsmPanel>,
    /// The last thing picked in the view, as Forge names it (or why nothing does).
    pub picked: Option<Result<String, super::fgtool::ToolError>>,
    /// The Parameters dialog is up.
    pub params_open: bool,
    /// The issues list is up, over the status bar (an assembly's, which has no panel).
    pub issues_list: bool,
}

/// The tree of ids: `bolt[2]` under `bolt`, `lug.hub` under `lug`, in the order they were made.
pub fn tree_rows(feats: &[FeatRow]) -> Vec<TreeRow> {
    struct Node {
        key: String,
        label: String,
        feat: Option<usize>,
        kids: Vec<usize>,
    }
    let mut nodes: Vec<Node> = Vec::new();
    let mut roots: Vec<usize> = Vec::new();
    let find =
        |nodes: &mut Vec<Node>, roots: &mut Vec<usize>, parent: Option<usize>, key: String, label: String| {
            let list = match parent {
                Some(p) => nodes[p].kids.clone(),
                None => roots.clone(),
            };
            if let Some(&i) = list.iter().find(|&&i| nodes[i].key == key) {
                return i;
            }
            nodes.push(Node {
                key,
                label,
                feat: None,
                kids: Vec::new(),
            });
            let i = nodes.len() - 1;
            match parent {
                Some(p) => nodes[p].kids.push(i),
                None => roots.push(i),
            }
            i
        };
    for (fi, f) in feats.iter().enumerate() {
        let segs = parse_path(&f.id).unwrap_or_else(|| {
            vec![Seg {
                name: f.id.clone(),
                index: vec![],
            }]
        });
        let mut parent = None;
        let mut prefix = String::new();
        for s in &segs {
            let base = if prefix.is_empty() {
                s.name.clone()
            } else {
                format!("{prefix}.{}", s.name)
            };
            let at = find(&mut nodes, &mut roots, parent, base.clone(), s.name.clone());
            parent = Some(at);
            prefix = base;
            if !s.index.is_empty() {
                let whole = path_text(std::slice::from_ref(s));
                let key = format!("{prefix}{}", &whole[s.name.len()..]);
                let at = find(&mut nodes, &mut roots, parent, key.clone(), whole);
                parent = Some(at);
                prefix = key;
            }
        }
        if let Some(p) = parent {
            nodes[p].feat = Some(fi);
        }
    }
    fn walk(nodes: &[Node], i: usize, depth: usize, out: &mut Vec<TreeRow>) {
        let n = &nodes[i];
        out.push(TreeRow {
            key: n.key.clone(),
            label: n.label.clone(),
            depth,
            feat: n.feat,
            group: !n.kids.is_empty(),
        });
        for &k in &n.kids {
            walk(nodes, k, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for &r in &roots {
        walk(&nodes, r, 0, &mut out);
    }
    out
}

/// A row's top-level statement: what Delete and Move address.
pub fn statement_of(key: &str) -> &str {
    key.split(['.', '[']).next().unwrap_or(key)
}

/// The top-level statements that make features, in order.
pub fn statements(feats: &[FeatRow]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in feats {
        let s = statement_of(&f.id);
        if !out.iter().any(|o| o == s) {
            out.push(s.to_string());
        }
    }
    out
}

/// What a fix or an op does, in a few words for its button.
pub fn op_label(op: &Op) -> String {
    match op {
        Op::SetParam { param, value } => format!("Set {param} = {}", value.text()),
        Op::AddLocalParam { feature, name, value } => {
            format!("Give {} its own {name} = {}", path_text(feature), value.text())
        }
        Op::MakeShared { param } => format!("Make {} shared", param.name),
        Op::SplitInstance { id, param } => format!("Own {} for {}", param.name, path_text(id)),
        Op::AddFeature { .. } => "Add feature".into(),
        Op::DeleteFeature { id } => format!("Delete {id}"),
        Op::ReplaceFeature { id, .. } => format!("Edit {id}"),
        Op::MoveFeature { id, before } => match before {
            Some(b) => format!("Move {id} above {b}"),
            None => format!("Move {id} to the end"),
        },
        Op::LiftLiteral { name, scope, .. } => match scope {
            Scope::Shared => format!("Lift to shared {name}"),
            Scope::Local(p) => format!("Lift to {}.{name}", path_text(p)),
        },
        Op::SetArg { id, arg, text } => format!("Set {id}'s {arg}: {text}"),
        Op::Rename { from, to } => format!("Rename {from} to {to}"),
        Op::SetProperty { key, text: Some(t) } => format!("Set {key}: {t}"),
        Op::SetProperty { key, text: None } => format!("Clear {key}"),
    }
}

/// A typed value: a number, its unit optional (`mm`, `deg`); else the param's own.
pub fn parse_value(text: &str, dim: Dim) -> Option<Literal> {
    let t = text.trim();
    let (num, unit) = match t.find(|c: char| c.is_ascii_alphabetic() || c == '°') {
        Some(k) => (t[..k].trim(), t[k..].trim()),
        None => (t, ""),
    };
    let value: f64 = num.parse().ok().filter(|v: &f64| v.is_finite())?;
    let dim = match unit {
        "" => dim,
        "mm" => Dim::Length,
        "deg" | "°" => Dim::Angle,
        _ => return None,
    };
    Some(Literal { value, dim })
}

fn unit(d: Dim) -> &'static str {
    match d {
        Dim::Length => "mm",
        Dim::Angle => "deg",
        Dim::Area => "mm²",
        Dim::Volume => "mm³",
        _ => "",
    }
}

pub fn short(v: f64) -> String {
    let r = (v * 1e4).round() / 1e4;
    if r == r.trunc() && r.abs() < 1e15 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

fn keys(ks: &[ParamKey]) -> String {
    ks.iter().map(ParamKey::to_string).collect::<Vec<_>>().join(", ")
}

impl FgPanel {
    pub fn tree(&self) -> Vec<TreeRow> {
        tree_rows(&self.feats)
    }

    pub fn issues_open(&self, errors: usize) -> bool {
        self.issues_shown.unwrap_or(errors > 0)
    }

    fn feat(&self, id: &str) -> Option<&FeatRow> {
        self.feats.iter().find(|f| f.id == id)
    }

    /// The worst issue in a feature, or in any feature under a group's key.
    fn flag(&self, key: &str) -> Option<Severity> {
        let under =
            |f: &str| f == key || f.starts_with(&format!("{key}.")) || f.starts_with(&format!("{key}["));
        let mut worst = None;
        for i in &self.issues {
            if i.feature.as_deref().is_some_and(under) {
                worst = match (worst, i.severity) {
                    (_, Severity::Error) | (Some(Severity::Error), _) => Some(Severity::Error),
                    (_, Severity::Lint) | (Some(Severity::Lint), _) => Some(Severity::Lint),
                    _ => Some(Severity::Warning),
                };
            }
        }
        if self
            .feats
            .iter()
            .any(|f| under(&f.id) && f.status == Status::Failed)
        {
            worst = Some(Severity::Error);
        }
        worst
    }

    /// The params shown: those the selected feature reads or owns, or all.
    pub fn shown_params(&self) -> Vec<&ParamRow> {
        let Some(sel) = &self.selected else {
            return self.params.iter().collect();
        };
        let top = statement_of(sel);
        let reads: Vec<&ParamKey> = self
            .feats
            .iter()
            .filter(|f| {
                f.id == *sel || f.id.starts_with(&format!("{sel}.")) || f.id.starts_with(&format!("{sel}["))
            })
            .flat_map(|f| &f.reads)
            .collect();
        self.params
            .iter()
            .filter(|p| reads.contains(&&p.key) || p.key.owner.first().is_some_and(|s| s.name == top))
            .collect()
    }

    /// The selected row is one instance of a loop or call: the params it shares, each of which
    /// SplitInstance would give it alone.
    pub fn splittable(&self) -> Vec<(Vec<Seg>, ParamKey)> {
        let Some(sel) = &self.selected else { return vec![] };
        let Some(segs) = parse_path(sel) else {
            return vec![];
        };
        if segs.iter().all(|s| s.index.is_empty()) {
            return vec![];
        }
        let shared: Vec<Seg> = segs
            .iter()
            .map(|s| Seg {
                name: s.name.clone(),
                index: vec![],
            })
            .collect();
        self.params
            .iter()
            .filter(|p| p.key.owner == shared)
            .map(|p| (segs.clone(), p.key.clone()))
            .collect()
    }
}

impl Shell {
    /// The tree, in the side panel's place.
    pub(super) fn fg_tree(&mut self, ui: &mut Ui, area: Rect) {
        let Some(mut fg) = self.fg.take() else { return };
        let t = self.theme;
        let rows = fg.tree();
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut chosen = None;
        let mut toggle = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
        let planes = self.overlays().planes;
        let mut eye_flip: Option<Option<String>> = None;
        egui::ScrollArea::vertical()
            .id_salt("fg-tree")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                // the part's solid: hidden by its eye, the rest of the tree as it is
                if !fg.feats.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(area.width(), super::ROW_H), Sense::hover());
                    let ink = if fg.body_hidden {
                        t.ink2.gamma_multiply(0.55)
                    } else {
                        t.ink
                    };
                    icons::draw(
                        ui.painter(),
                        pos2(r.left() + 20.0, r.top() + 5.0),
                        16.0,
                        "newsolid",
                        1.5,
                        ink,
                    );
                    ui.painter().text(
                        pos2(r.left() + 42.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Solid",
                        medium(13.0),
                        ink,
                    );
                    if row_eye(ui, &t, r, "fg-eye-solid", !fg.body_hidden) {
                        eye_flip = Some(None);
                    }
                    places.push(("fg:eye:solid".into(), eye_rect(r)));
                }
                let mut hidden: Option<String> = None;
                for row in &rows {
                    if let Some(h) = &hidden {
                        if row.key.starts_with(h.as_str()) && row.key != *h {
                            continue;
                        }
                        hidden = None;
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(area.width(), super::ROW_H), Sense::click());
                    let on = fg.selected.as_deref() == Some(row.key.as_str());
                    resp.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::TreeItem, true, on, &row.label)
                    });
                    let p = &ui.painter().clone();
                    if on {
                        p.rect_filled(r, 0.0, t.on);
                        p.rect_filled(Rect::from_min_size(r.min, vec2(2.0, super::ROW_H)), 0.0, t.focus);
                    } else if resp.hovered() {
                        p.rect_filled(r, 0.0, t.hover);
                    }
                    let ink = if on { t.on_ink } else { t.ink };
                    let x = r.left() + 8.0 + row.depth as f32 * INDENT;
                    let open = !fg.collapsed.contains(&row.key);
                    if row.group {
                        let arrow = Rect::from_min_size(pos2(x - 2.0, r.top() + 5.0), vec2(14.0, 16.0));
                        let a = ui.interact(arrow, ui.scope_id().with(("fg-open", &row.key)), Sense::click());
                        disclosure(p, arrow.center(), open, t.ink2);
                        if a.clicked() {
                            toggle = Some(row.key.clone());
                        }
                        places.push((format!("fg:open:{}", row.key), arrow));
                    }
                    let kind = row.feat.map(|i| fg.feats[i].kind.as_str());
                    let icon = match kind {
                        Some(k) => panel::icon(k),
                        None => "pattern",
                    };
                    let flag = fg.flag(&row.key);
                    let tint = match flag {
                        Some(Severity::Error) => Some(t.error),
                        Some(Severity::Lint | Severity::Warning | Severity::Violation) => {
                            Some(Color32::from_rgb(0xE0, 0xA0, 0x30))
                        }
                        None => None,
                    };
                    icons::draw(
                        p,
                        pos2(x + 12.0, r.top() + 5.0),
                        16.0,
                        icon,
                        1.5,
                        tint.unwrap_or(ink),
                    );
                    let font = if on { medium(13.0) } else { sans(13.0) };
                    let ink = if row.feat.is_some_and(|i| fg.feats[i].status == Status::NotBuilt) {
                        t.ink2
                    } else {
                        ink
                    };
                    p.text(
                        pos2(x + 34.0, r.center().y),
                        Align2::LEFT_CENTER,
                        &row.label,
                        font,
                        ink,
                    );
                    // a sketch's or a work plane's plane: its eye
                    let plane = row.feat.is_some_and(|i| fg.feats[i].frame.is_some());
                    if let Some(k) = kind {
                        p.text(
                            pos2(r.right() - if plane { 36.0 } else { 10.0 }, r.center().y),
                            Align2::RIGHT_CENTER,
                            k,
                            sans(11.0),
                            t.ink2,
                        );
                    }
                    if plane {
                        let on = fg.eyes.get(&row.key).copied().unwrap_or(planes);
                        if row_eye(ui, &t, r, &format!("fg-eye-{}", row.key), on) {
                            eye_flip = Some(Some(row.key.clone()));
                        }
                        places.push((format!("fg:eye:{}", row.key), eye_rect(r)));
                    }
                    if let Some(c) = tint {
                        p.circle_filled(pos2(r.right() - 4.0, r.top() + 6.0), 3.0, c);
                    }
                    if resp.clicked() {
                        chosen = Some(row.key.clone());
                    }
                    places.push((format!("fg:row:{}", row.key), r));
                    if row.group && !open {
                        hidden = Some(row.key.clone());
                    }
                }
            });
        if let Some(k) = toggle
            && !fg.collapsed.remove(&k)
        {
            fg.collapsed.insert(k);
        }
        match eye_flip {
            Some(None) => fg.body_hidden = !fg.body_hidden,
            Some(Some(k)) => {
                let on = fg.eyes.get(&k).copied().unwrap_or(planes);
                fg.eyes.insert(k, !on);
            }
            None => {}
        }
        // a tool waiting for a feature or a sketch takes the row
        if let Some(k) = chosen.clone()
            && let Some(tool) = &mut fg.tool
            && tool.slots.get(tool.active).is_some_and(|s| s.kind.from_tree())
        {
            let kind = tool.slots[tool.active].kind;
            let is_sketch = fg.feats.iter().any(|f| f.id == k && f.kind == "sketch");
            use super::fgtool::{SlotKind, ToolError};
            let pick = match kind {
                SlotKind::Profile | SlotKind::Profiles | SlotKind::Plane if is_sketch => Ok(k.clone()),
                SlotKind::Feature | SlotKind::Features if !is_sketch => Ok(k.clone()),
                _ => Err(ToolError::WrongPick {
                    slot: tool.slots[tool.active].label,
                    what: if is_sketch { "sketch" } else { "feature" },
                }),
            };
            tool.take(pick);
            self.places.extend(places);
            self.fg = Some(fg);
            return;
        }
        if let Some(k) = chosen {
            fg.selected = if fg.selected.as_deref() == Some(k.as_str()) {
                None
            } else {
                Some(k)
            };
            fg.editing = None;
        }
        self.places.extend(places);
        self.fg = Some(fg);
    }

    /// The params, the selected feature's dimensions and actions, and the issues: a panel on the
    /// view's right.
    pub(super) fn fg_panel(&mut self, ctx: &egui::Context) {
        // an assembly has no panel: what it held is in the browser, the ribbon's dialogs and the
        // status bar, as Inventor has it
        if self.is_asm() {
            return;
        }
        let Some(mut fg) = self.fg.take() else { return };
        let t = self.theme;
        let v = self.view_rect;
        let rect = Rect::from_min_max(pos2(v.right(), v.top()), pos2(v.right() + PANEL_W, v.bottom()));
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut out: Vec<FgAct> = Vec::new();
        egui::Area::new(egui::Id::unique("fg-panel"))
            .fixed_pos(rect.min)
            .constrain(false)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                // a long value (a list param, a sheet's report) is cut at the panel's edge, never
                // widening the panel over the view
                ui.set_clip_rect(rect);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.painter().rect_filled(rect, 0.0, t.surface);
                ui.painter()
                    .vline(rect.left(), rect.y_range(), Stroke::new(1.0, t.line));
                egui::ScrollArea::vertical()
                    .id_salt("fg-panel-scroll")
                    .auto_shrink([false, false])
                    .max_height(rect.height())
                    .show(ui, |ui| {
                        egui::Frame::new().inner_margin(10).show(ui, |ui| {
                            ui.set_width(PANEL_W - 20.0);
                            ui.spacing_mut().item_spacing.y = 4.0;
                            Self::fg_head(ui, &t, &mut fg, &mut out, &mut places);
                            Self::fg_tool(ui, &t, &mut fg, &mut out, &mut places);
                            Self::fg_sheets(ui, &t, &fg, &mut out, &mut places);
                            Self::fg_selection(ui, &t, &mut fg, &mut out, &mut places);
                            Self::fg_issues(ui, &t, &mut fg, &mut out, &mut places);
                            Self::fg_source(ui, &t, &mut fg);
                        });
                    });
            });
        self.view_blocks.push(rect);
        self.places.extend(places);
        fg.out.extend(out);
        self.fg = Some(fg);
    }

    /// The tool's form: its name, a row per slot with what's picked and what can be chosen, its
    /// params, OK and Cancel.
    fn fg_tool(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        use super::fgtool::SlotKind as K;
        let stmts = statements(&fg.feats);
        let Some(tool) = &mut fg.tool else {
            if let Some(p) = &fg.picked {
                let text = match p {
                    Ok(a) => format!("Picked: {a}"),
                    Err(e) => format!("Picked: {}", e.text()),
                };
                let r = ui.label(egui::RichText::new(text).font(mono(11.0)).color(t.ink2));
                places.push(("fg:picked".into(), r.rect));
            }
            return;
        };
        Self::fg_heading(ui, t, &panel::kind_word(tool.kind));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Name").font(sans(12.0)).color(t.ink2));
            let r = ui.add(
                egui::TextEdit::singleline(&mut tool.name)
                    .font(mono(12.0))
                    .desired_width(140.0),
            );
            places.push(("fg:tool:name".into(), r.rect));
        });
        for k in 0..tool.slots.len() {
            let (label, kind) = (tool.slots[k].label, tool.slots[k].kind);
            ui.horizontal_wrapped(|ui| {
                let on = tool.active == k;
                let r = ui.add(egui::Button::new(egui::RichText::new(label).font(medium(12.0))).selected(on));
                places.push((format!("fg:tool:slot:{label}"), r.rect));
                if r.clicked() {
                    tool.active = k;
                }
                let mut gone = None;
                for (i, p) in tool.slots[k].picks.iter().enumerate() {
                    let r = ui.add(egui::Button::new(
                        egui::RichText::new(format!("{p} ×")).font(mono(11.0)),
                    ));
                    places.push((format!("fg:tool:pick:{label}:{i}"), r.rect));
                    if r.clicked() {
                        gone = Some(i);
                    }
                }
                if let Some(i) = gone {
                    tool.slots[k].picks.remove(i);
                }
            });
            // what the view can't give: sketch entities, features, origin planes
            let mut choices: Vec<String> = Vec::new();
            for sk in &fg.sketches {
                match kind {
                    K::Profile | K::Profiles => {
                        choices.push(sk.id.clone());
                        choices.extend(sk.closed.iter().cloned());
                    }
                    K::Line => choices.extend(sk.lines.iter().cloned()),
                    K::Path => {
                        choices.extend(sk.lines.iter().cloned());
                        choices.extend(sk.curves.iter().cloned());
                        choices.extend(sk.closed.iter().cloned());
                    }
                    K::Points => choices.extend(sk.points.iter().cloned()),
                    K::Dir => choices.extend(sk.lines.iter().map(|l| format!("{l}.dir"))),
                    K::Plane => choices.push(sk.id.clone()),
                    _ => {}
                }
            }
            match kind {
                K::Plane => choices.extend(["xy", "xz", "yz"].map(String::from)),
                K::Feature | K::Features => {
                    let sketches: Vec<&str> = fg.sketches.iter().map(|s| s.id.as_str()).collect();
                    choices.extend(stmts.iter().filter(|s| !sketches.contains(&s.as_str())).cloned());
                }
                _ => {}
            }
            if !choices.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for c in choices {
                        let r = ui.add(egui::Button::new(egui::RichText::new(&c).font(mono(10.0))).small());
                        places.push((format!("fg:choice:{label}:{c}"), r.rect));
                        if r.clicked() {
                            tool.active = k;
                            tool.take(Ok(c));
                        }
                    }
                });
            }
        }
        for p in &mut tool.params {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(p.name).font(mono(12.0)).color(t.ink));
                let r = ui.add(
                    egui::TextEdit::singleline(&mut p.text)
                        .font(mono(12.0))
                        .desired_width(70.0),
                );
                places.push((format!("fg:tool:param:{}", p.name), r.rect));
                ui.label(egui::RichText::new(unit(p.dim)).font(sans(11.0)).color(t.ink2));
            });
        }
        for (w, text) in &mut tool.words {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(*w).font(mono(12.0)).color(t.ink));
                let r = ui.add(
                    egui::TextEdit::singleline(text)
                        .font(mono(12.0))
                        .desired_width(90.0),
                );
                places.push((format!("fg:tool:word:{w}"), r.rect));
            });
        }
        for (f, on) in &mut tool.flags {
            let r = ui.checkbox(on, *f);
            places.push((format!("fg:tool:flag:{f}"), r.rect));
        }
        if let Some(e) = &tool.error {
            ui.label(
                egui::RichText::new(format!("{}: {}", e.kind(), e.text()))
                    .font(sans(12.0))
                    .color(t.error),
            );
        }
        let mut close = false;
        ui.horizontal(|ui| {
            if Self::fg_button(ui, "OK", "fg:tool:ok".into(), places) {
                match tool.text() {
                    Ok(text) => out.push(FgAct::Tool(Op::AddFeature { before: None, text })),
                    Err(e) => tool.error = Some(e),
                }
            }
            if Self::fg_button(ui, "Cancel", "fg:tool:cancel".into(), places) {
                close = true;
            }
        });
        if close {
            fg.tool = None;
        }
    }

    pub(super) fn fg_button(
        ui: &mut Ui,
        label: &str,
        name: String,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let r = ui.add(egui::Button::new(egui::RichText::new(label).font(sans(12.0))));
        places.push((name, r.rect));
        r.clicked()
    }

    pub(super) fn fg_heading(ui: &mut Ui, t: &super::theme::Theme, text: &str) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(text).font(semi(12.0)).color(t.ink));
        let r = ui.available_rect_before_wrap();
        ui.painter()
            .hline(r.left()..=r.right(), r.top(), Stroke::new(1.0, t.line));
        ui.add_space(2.0);
    }

    fn fg_head(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        ui.horizontal(|ui| {
            let name = if fg.dirty {
                format!("{} (unsaved)", fg.file)
            } else {
                fg.file.clone()
            };
            ui.label(egui::RichText::new(name).font(semi(14.0)).color(t.ink));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if Self::fg_button(ui, "Save", "fg:save".into(), places) {
                    out.push(FgAct::Save);
                }
            });
        });
        if !fg.summary.is_empty() {
            ui.label(egui::RichText::new(&fg.summary).font(sans(11.0)).color(t.ink2));
        }
        if let Some(why) = &fg.refused {
            ui.label(
                egui::RichText::new(format!("Refused: {why}"))
                    .font(sans(12.0))
                    .color(t.error),
            );
        }
    }

    fn fg_sheets(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        if fg.sheets.is_empty() {
            return;
        }
        Self::fg_heading(ui, t, "Sheet metal");
        let label = if fg.flat { "Show folded" } else { "Show flat" };
        if Self::fg_button(ui, label, "fg:flat".into(), places) {
            out.push(FgAct::Flat(!fg.flat));
        }
        for s in &fg.sheets {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} · {} bends", s.id, s.bends))
                        .font(sans(12.0))
                        .color(t.ink),
                );
                match &s.refused {
                    None => {
                        if Self::fg_button(ui, "Export DXF", format!("fg:dxf:{}", s.id), places) {
                            out.push(FgAct::ExportDxf(s.id.clone()));
                        }
                    }
                    Some(why) => {
                        ui.label(
                            egui::RichText::new(format!("no flat: {why}"))
                                .font(sans(11.0))
                                .color(t.error),
                        );
                    }
                }
            });
            // its report, by what it holds: a field or list a later lane adds shows here as is
            for (k, v) in &s.facts {
                ui.label(
                    egui::RichText::new(format!("  {k}: {v}"))
                        .font(mono(11.0))
                        .color(t.ink2),
                );
            }
            for (k, items) in &s.lists {
                if items.is_empty() {
                    continue;
                }
                let r = ui.label(
                    egui::RichText::new(format!("  {k}"))
                        .font(medium(11.0))
                        .color(t.ink),
                );
                places.push((format!("fg:sheet:{}:{k}", s.id), r.rect));
                for it in items {
                    ui.label(
                        egui::RichText::new(format!("    {it}"))
                            .font(mono(10.0))
                            .color(t.ink2),
                    );
                }
            }
        }
        // the shop's rules broken, each with its typed fixes
        let shop: Vec<(usize, &Issue)> = fg
            .issues
            .iter()
            .enumerate()
            .filter(|(_, i)| i.severity == Severity::Violation)
            .collect();
        if !shop.is_empty() {
            ui.label(egui::RichText::new("Shop").font(semi(12.0)).color(t.ink));
            for (i, is) in shop {
                let r = ui.label(
                    egui::RichText::new(format!("{}: {}", is.kind, is.text))
                        .font(sans(11.0))
                        .color(Color32::from_rgb(0xE0, 0xA0, 0x30)),
                );
                places.push((format!("fg:shop:{i}"), r.rect));
                ui.horizontal_wrapped(|ui| {
                    for (j, op) in is.fixes.iter().enumerate() {
                        if Self::fg_button(ui, &op_label(op), format!("fg:shopfix:{i}:{j}"), places) {
                            out.push(FgAct::Apply(op.clone()));
                        }
                    }
                });
            }
        }
    }

    fn fg_selection(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let stmts = statements(&fg.feats);
        if let Some(sel) = fg.selected.clone() {
            Self::fg_heading(ui, t, &sel);
            let top = statement_of(&sel).to_string();
            let at = stmts.iter().position(|s| *s == top);
            ui.horizontal_wrapped(|ui| {
                if let Some(k) = at {
                    if k > 0 && Self::fg_button(ui, "Move up", "fg:up".into(), places) {
                        out.push(FgAct::Apply(Op::MoveFeature {
                            id: top.clone(),
                            before: Some(stmts[k - 1].clone()),
                        }));
                    }
                    if k + 1 < stmts.len() && Self::fg_button(ui, "Move down", "fg:down".into(), places) {
                        out.push(FgAct::Apply(Op::MoveFeature {
                            id: top.clone(),
                            before: stmts.get(k + 2).cloned(),
                        }));
                    }
                    if Self::fg_button(ui, &format!("Delete {top}"), "fg:delete".into(), places) {
                        out.push(FgAct::Apply(Op::DeleteFeature { id: top.clone() }));
                    }
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut fg.rename)
                            .font(mono(12.0))
                            .hint_text("new name")
                            .desired_width(90.0),
                    );
                    places.push(("fg:renametext".into(), r.rect));
                    if Self::fg_button(ui, "Rename", "fg:rename".into(), places)
                        && !fg.rename.trim().is_empty()
                    {
                        out.push(FgAct::Apply(Op::Rename {
                            from: top.clone(),
                            to: fg.rename.trim().to_string(),
                        }));
                        fg.rename.clear();
                    }
                }
            });
            for (id, param) in fg.splittable() {
                let op = Op::SplitInstance { id, param };
                let name = format!(
                    "fg:split:{}",
                    match &op {
                        Op::SplitInstance { param, .. } => param.to_string(),
                        _ => unreachable!("just made"),
                    }
                );
                if Self::fg_button(ui, &op_label(&op), name, places) {
                    out.push(FgAct::Apply(op));
                }
            }
            if let Some(f) = fg.feat(&sel).cloned() {
                for (k, d) in f.dims.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&d.what).font(sans(12.0)).color(t.ink2));
                        let value = format!("{} {}", short(d.value), unit(d.dim));
                        ui.label(egui::RichText::new(value).font(mono(12.0)).color(t.ink));
                        let from = match &d.from {
                            From::Const => "constant".to_string(),
                            From::Driving(p) => format!("= {p}"),
                            From::Driven(ps) => format!("← {}", keys(ps)),
                        };
                        let r = ui.label(egui::RichText::new(from).font(sans(11.0)).color(t.ink2));
                        places.push((format!("fg:dim:{k}"), r.rect));
                    });
                }
            }
        }
        if fg.asm.is_some() {
            return;
        }
        Self::fg_heading(ui, t, "Add feature");
        let r = ui.add(
            egui::TextEdit::singleline(&mut fg.add_text)
                .font(mono(12.0))
                .hint_text("name = extrude(sketch, t)")
                .desired_width(f32::INFINITY),
        );
        places.push(("fg:addtext".into(), r.rect));
        let before = fg.selected.as_deref().map(|s| statement_of(s).to_string());
        let label = match &before {
            Some(b) => format!("Add above {b}"),
            None => "Add at the end".into(),
        };
        if Self::fg_button(ui, &label, "fg:add".into(), places) && !fg.add_text.trim().is_empty() {
            out.push(FgAct::Apply(Op::AddFeature {
                before,
                text: fg.add_text.trim().to_string(),
            }));
        }
    }

    /// The Parameters dialog, as Inventor's fx: every param with its unit, its equation (a
    /// driving one's typed in place: a SetParam), its value, and what reads it.
    pub(super) fn fg_params_window(&mut self, ctx: &egui::Context) {
        let Some(mut fg) = self.fg.take() else { return };
        if !fg.params_open {
            self.fg = Some(fg);
            return;
        }
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut out: Vec<FgAct> = Vec::new();
        let mut close = false;
        let w = (v.width() - 200.0).clamp(420.0, 760.0);
        let shown = egui::Window::new("Parameters")
            .id(egui::Id::unique("fg-params"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(pos2(v.left() + 24.0, v.top() + 48.0))
            .min_width(w)
            .max_width(w)
            .frame(super::fgasm::dialog_frame(&t))
            .show(ctx, |ui| {
                close = super::fgasm::dialog_head(ui, &t, "params", "Parameters", "fg:params", &mut places);
                ui.label(
                    egui::RichText::new(format!(
                        "{} · a driving value is edited in place; a driven one by what it reads",
                        plural(fg.params.len(), "parameter")
                    ))
                    .font(sans(11.0))
                    .color(t.ink2),
                );
                ui.add_space(4.0);
                let params = fg.params.clone();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height((v.height() - 140.0).max(200.0))
                    .show(ui, |ui| {
                        egui::Grid::new("fg-params-grid")
                            .num_columns(5)
                            .striped(true)
                            .spacing(vec2(14.0, 4.0))
                            .min_col_width(40.0)
                            .show(ui, |ui| {
                                for h in [
                                    "Parameter Name",
                                    "Unit",
                                    "Equation",
                                    "Nominal Value",
                                    "Consumed by",
                                ] {
                                    ui.label(egui::RichText::new(h).font(medium(11.0)).color(t.ink2));
                                }
                                ui.end_row();
                                for p in &params {
                                    Self::param_row(ui, &t, &mut fg, p, &mut out, &mut places);
                                    ui.end_row();
                                }
                            });
                    });
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
        }
        if close {
            fg.params_open = false;
            fg.editing = None;
        }
        self.places.extend(places);
        fg.out.extend(out);
        self.fg = Some(fg);
    }

    fn param_row(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        p: &ParamRow,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let key = p.key.to_string();
        let cut = |s: &str, n: usize| {
            let mut c: String = s.chars().take(n).collect();
            if c.len() < s.len() {
                c.push('…');
            }
            c
        };
        // on one line, however it's written
        let eq = cut(&p.text.split_whitespace().collect::<Vec<_>>().join(" "), 40);
        ui.horizontal(|ui| {
            let (mark, r) = ui.allocate_exact_size(vec2(10.0, 18.0), Sense::hover());
            if p.structural {
                let c = mark.center();
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        pos2(c.x, c.y - 4.0),
                        pos2(c.x + 4.0, c.y),
                        pos2(c.x, c.y + 4.0),
                        pos2(c.x - 4.0, c.y),
                    ],
                    Color32::from_rgb(0x9A, 0x7B, 0xE0),
                    Stroke::NONE,
                ));
                r.on_hover_text("structural: evaluation reads it; editing re-runs");
            }
            let r = ui.label(egui::RichText::new(&key).font(mono(12.0)).color(t.ink));
            places.push((format!("fg:paramname:{key}"), r.rect));
        });
        let u = p.value.map_or("", |(_, d)| unit(d));
        ui.label(
            egui::RichText::new(if u.is_empty() { "ul" } else { u })
                .font(sans(11.0))
                .color(t.ink2),
        );
        if p.driving() {
            let (value, dim) = p.value.expect("driving is a number");
            let id = egui::Id::unique(("fg-param", &key));
            let editing = fg.editing.as_ref().is_some_and(|e| e.0 == p.key);
            if editing {
                let text = &mut fg.editing.as_mut().expect("editing").1;
                let resp = ui.add(
                    egui::TextEdit::singleline(text)
                        .id(id)
                        .font(mono(12.0))
                        .desired_width(120.0),
                );
                if fg.edit_fresh {
                    fg.edit_fresh = false;
                    resp.request_focus();
                    if let Some(mut s) = egui::TextEdit::load_state(ui.ctx(), id) {
                        let n = text.chars().count();
                        s.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(n),
                        )));
                        s.store(ui.ctx(), id);
                    }
                }
                places.push((format!("fg:param:{key}"), resp.rect));
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if esc {
                    fg.editing = None;
                } else if enter || (resp.lost_focus() && !resp.has_focus()) {
                    let typed = text.clone();
                    fg.editing = None;
                    match parse_value(&typed, dim) {
                        Some(v) if v.value == value && v.dim == dim => {}
                        Some(v) => out.push(FgAct::Apply(Op::SetParam {
                            param: p.key.clone(),
                            value: v,
                        })),
                        None => fg.refused = Some(format!("{typed:?} is not a number for {key}")),
                    }
                }
            } else {
                let resp = ui.add(
                    egui::Button::new(egui::RichText::new(&eq).font(mono(12.0))).min_size(vec2(120.0, 18.0)),
                );
                places.push((format!("fg:param:{key}"), resp.rect));
                if resp.clicked() {
                    fg.editing = Some((p.key.clone(), short(value)));
                    fg.edit_fresh = true;
                }
            }
        } else {
            // driven: its equation as written, edited through what it reads
            let r = ui.label(egui::RichText::new(&eq).font(mono(12.0)).color(t.ink2));
            places.push((format!("fg:param:{key}"), r.rect));
            if !p.links.is_empty() {
                r.on_hover_text(format!("{}\ndriven by {}", p.text, keys(&p.links)));
            }
        }
        let shown = match p.value {
            Some((v, _)) => short(v),
            None => String::new(),
        };
        ui.label(egui::RichText::new(shown).font(mono(12.0)).color(t.ink));
        let used = p.used_by.join(", ");
        let r = ui.label(egui::RichText::new(cut(&used, 40)).font(sans(11.0)).color(t.ink2));
        if !used.is_empty() {
            r.on_hover_text(used);
        }
    }

    /// An assembly's issues, over the status bar: each with its fixes and a Show.
    pub(super) fn fg_issues_window(&mut self, ctx: &egui::Context) {
        let Some(mut fg) = self.fg.take() else { return };
        if !fg.issues_list || !(fg.assembly || fg.asm.is_some()) {
            self.fg = Some(fg);
            return;
        }
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut out: Vec<FgAct> = Vec::new();
        let mut close = false;
        let w = (v.width() - 32.0).min(900.0);
        let shown = egui::Window::new("Issues")
            .id(egui::Id::unique("fg-issues"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .pivot(Align2::LEFT_BOTTOM)
            .default_pos(pos2(v.left() + 16.0, v.bottom() - 12.0))
            .min_width(w)
            .max_width(w)
            .frame(super::fgasm::dialog_frame(&t))
            .show(ctx, |ui| {
                close = super::fgasm::dialog_head(ui, &t, "warning", "Issues", "fg:issueswin", &mut places);
                fg.issues_shown = Some(true);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height((v.height() * 0.45).max(160.0))
                    .show(ui, |ui| {
                        Self::fg_issues(ui, &t, &mut fg, &mut out, &mut places);
                    });
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
        }
        if close {
            fg.issues_list = false;
        }
        self.places.extend(places);
        fg.out.extend(out);
        self.fg = Some(fg);
    }

    /// The issues: one line of counts, opened by a click; open by itself only while there are
    /// errors, until a click says otherwise.
    fn fg_issues(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let (errors, warnings) = issue_counts(&fg.issues);
        let open = fg.issues_open(errors) && !fg.issues.is_empty();
        let head = format!("{} · {}", plural(errors, "error"), plural(warnings, "warning"));
        ui.add_space(6.0);
        let ink = if errors > 0 {
            t.error
        } else if warnings > 0 {
            WARN
        } else {
            t.ink2
        };
        let r = fold_head(ui, t, &head, open, ink);
        places.push(("fg:issues".into(), r.rect));
        if r.clicked() && !fg.issues.is_empty() {
            fg.issues_shown = Some(!open);
        }
        if !open {
            return;
        }
        let issues = std::mem::take(&mut fg.issues);
        // a long list shows its first few until asked
        let cap = if fg.all_issues { 300 } else { 8 };
        // an assembly's notes (intended contacts) are counted in Analyze Interference, not here
        let note = is_note;
        for (i, is) in issues.iter().enumerate().filter(|x| !note(x.1)).take(cap) {
            let colour = match is.severity {
                Severity::Error => t.error,
                _ => WARN,
            };
            let word = match is.severity {
                Severity::Error => "error",
                Severity::Lint => "lint",
                Severity::Warning => "warning",
                Severity::Violation => "violation",
            };
            let r = ui.label(
                egui::RichText::new(format!("{word} {} · line {}:{}", is.kind, is.line, is.col))
                    .font(medium(12.0))
                    .color(colour),
            );
            places.push((format!("fg:issue:{i}"), r.rect));
            ui.label(egui::RichText::new(&is.text).font(sans(12.0)).color(t.ink));
            ui.horizontal_wrapped(|ui| {
                for (j, op) in is.fixes.iter().enumerate() {
                    if Self::fg_button(ui, &op_label(op), format!("fg:fix:{i}:{j}"), places) {
                        out.push(FgAct::Apply(op.clone()));
                    }
                }
                if let Some(ai) = &is.asm {
                    Self::asm_show_button(ui, &mut fg.asm, ai, i, out, places);
                }
            });
        }
        let listed = issues.iter().filter(|i| !note(i)).count();
        if listed > 8 {
            let label = if fg.all_issues {
                "Show fewer".to_string()
            } else {
                format!("Show all {listed}")
            };
            if Self::fg_button(ui, &label, "fg:issues:all".into(), places) {
                fg.all_issues = !fg.all_issues;
            }
        }
        fg.issues = issues;
    }

    fn fg_source(ui: &mut Ui, t: &super::theme::Theme, fg: &mut FgPanel) {
        ui.add_space(6.0);
        if fold_head(ui, t, "Source", fg.show_source, t.ink).clicked() {
            fg.show_source = !fg.show_source;
        }
        if !fg.show_source {
            return;
        }
        let marked: Vec<(usize, Severity)> = fg.issues.iter().map(|i| (i.line, i.severity)).collect();
        for (k, line) in fg.source.lines().enumerate() {
            let no = k + 1;
            let mark = marked.iter().find(|m| m.0 == no).map(|m| m.1);
            let ink = match mark {
                Some(Severity::Error) => t.error,
                Some(_) => Color32::from_rgb(0xE0, 0xA0, 0x30),
                None => t.ink,
            };
            let r = ui.label(
                egui::RichText::new(format!("{no:>4} {line}"))
                    .font(mono(11.0))
                    .color(ink),
            );
            if mark.is_some() {
                ui.painter()
                    .rect_stroke(r.rect, 2.0, Stroke::new(1.0, ink), StrokeKind::Outside);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feat(id: &str) -> FeatRow {
        FeatRow {
            id: id.into(),
            kind: "extrude".into(),
            line: 1,
            status: Status::Ok,
            dims: vec![],
            reads: vec![],
            frame: None,
        }
    }

    #[test]
    fn loop_and_call_ids_nest_under_their_names() {
        let feats: Vec<FeatRow> = [
            "base",
            "bpsk[0,0]",
            "bpocket[0,0]",
            "bpsk[0,1]",
            "lug.hub",
            "lug.bore",
        ]
        .into_iter()
        .map(feat)
        .collect();
        let rows: Vec<(String, usize, bool)> = tree_rows(&feats)
            .into_iter()
            .map(|r| (r.key, r.depth, r.feat.is_some()))
            .collect();
        let want = [
            ("base", 0, true),
            ("bpsk", 0, false),
            ("bpsk[0,0]", 1, true),
            ("bpsk[0,1]", 1, true),
            ("bpocket", 0, false),
            ("bpocket[0,0]", 1, true),
            ("lug", 0, false),
            ("lug.hub", 1, true),
            ("lug.bore", 1, true),
        ];
        let want: Vec<(String, usize, bool)> = want.iter().map(|w| (w.0.to_string(), w.1, w.2)).collect();
        assert_eq!(rows, want);
        assert_eq!(statements(&feats), ["base", "bpsk", "bpocket", "lug"]);
    }

    #[test]
    fn a_typed_value_takes_the_params_dimension_or_its_own_unit() {
        assert_eq!(
            parse_value("12.5", Dim::Length),
            Some(Literal {
                value: 12.5,
                dim: Dim::Length
            })
        );
        assert_eq!(parse_value("30deg", Dim::Length).map(|l| l.dim), Some(Dim::Angle));
        assert_eq!(parse_value("3", Dim::Count).map(|l| l.dim), Some(Dim::Count));
        assert_eq!(parse_value("x", Dim::Length), None);
        assert_eq!(parse_value("3 in", Dim::Length), None);
    }
}
