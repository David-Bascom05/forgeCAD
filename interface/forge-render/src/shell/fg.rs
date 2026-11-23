//! A Forge (.fg) part's chrome: the browser of its feature ids (its menus and double click open
//! the tools that edit them), the Parameters dialog, the Issues window with each issue's fixes. Like the rest of the shell it changes nothing itself: every
//! edit leaves as a typed op in `out`, which the host applies through forge-lang.

use super::theme::{medium, mono, sans, semi};
use super::{Shell, icons, panel};
use crate::shell::placed::Placed;
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use forge_lang::error::{Literal, Op, ParamKey, Scope, parse_path, path_text};
use forge_lang::syntax::Seg;
use forge_lang::values::Dim;
use std::collections::HashSet;

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

/// A browser row's name typed in its place, all of it selected at first: Enter or a click away
/// renames it everywhere, Esc leaves it.
fn rename_edit(ui: &mut Ui, fg: &mut FgPanel, at: Rect, places: &mut Vec<(String, Rect)>) {
    let Some((from, text)) = &mut fg.renaming else {
        return;
    };
    let id = egui::Id::unique(("fg-rename", from.as_str()));
    let resp = ui.put(
        at,
        egui::TextEdit::singleline(text)
            .id(id)
            .font(mono(12.0))
            .margin(vec2(4.0, 2.0)),
    );
    places.push(("fg:renametext".into(), at));
    if fg.rename_fresh {
        fg.rename_fresh = false;
        resp.request_focus();
        if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), id) {
            let n = text.chars().count();
            st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(n),
            )));
            st.store(ui.ctx(), id);
        }
        return;
    }
    if resp.lost_focus() {
        let (from, to) = (from.clone(), text.trim().to_string());
        fg.renaming = None;
        if !to.is_empty() && to != from {
            fg.out.push(FgAct::Apply(Op::Rename { from, to }));
        }
    }
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

/// What the browser and the dialogs ask of the host.
#[derive(Clone, Debug, PartialEq)]
pub enum FgAct {
    Apply(Op),
    /// A file in a tab of its own: an assembly's part.
    OpenFile(String),
    Save,
    /// A tool's statement: applied, and the tool closed if it was.
    Tool(Op),
    /// What a Forge part can't take: said, typed.
    Refuse(super::fgtool::ToolError),
    /// An assembly's view changed (selection, hidden, isolated, highlighted): redraw its parts.
    AsmShow,
    /// The view turned to a place: its middle and how far round it to show (mm).
    Locate([f64; 3], f64),
    /// Run the assembly's overlap check.
    AsmCheck,
    /// Where the interferences are and what faces they touch.
    AsmLocate,
    /// The view fitted to a located clash, its faces marked.
    AsmZoom(usize),
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
    /// A sketch block, in the Sketch tab with its geometry, constraints and dimensions.
    EditSketch(String),
    /// A feature's tool dialog, filled from its statement, to edit it.
    EditFeature(String),
    /// Move a component to its own file, bring one in, or make bodies a component.
    Refactor(forge_lang::refactor::Refactor),
    /// A number dragged on the model: these ops built in the file's place, the file kept.
    DimPreview(Vec<Op>),
    /// The number let go or typed: these ops applied, one undo step.
    DimCommit(Vec<Op>),
    /// The drag given up: the file's own part again.
    DimCancel,
    /// The end of part moved: builds stop after this statement (None: the whole part).
    Rollback(Option<String>),
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
    /// The file as opened, for what's beside it (its folder, its project's materials).
    pub path: String,
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
    /// Every issue listed, not the first few.
    pub all_issues: bool,
    /// A browser row being renamed in place: its statement and the name typed so far.
    pub renaming: Option<(String, String)>,
    pub rename_fresh: bool,
    /// The last op the host refused, and why.
    pub refused: Option<String>,
    pub collapsed: HashSet<String>,
    pub sheets: Vec<SheetRow>,
    /// The part's thermoforms, and whether their wall thickness is mapped on the view.
    pub thermos: Vec<super::thermo::ThermoRow>,
    /// The Thermoform tab's dialogs and analyses, and the materials Convert offers.
    pub thermo: super::thermo_ui::ThermoUi,
    /// The Harness tab's dialog (harness_ui.rs).
    pub harness: super::harness_ui::HarnessUi,
    pub thermo_materials: Vec<super::thermo_ui::MaterialRow>,
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
    /// The weldments' cut list, as the last build had it; its panel up.
    pub cutlist: super::weld_ui::CutList,
    pub cutlist_open: bool,
    /// The Issues window is up, over the status bar.
    pub issues_list: bool,
    /// Edit Occurrence: a placed component call's args.
    pub occ_edit: Option<super::occedit::OccEdit>,
    /// The End of Part marker: builds stop after this statement (None: at the end).
    pub rollback: Option<String>,
}

/// A statement `Suppress {id}` took out of the build, kept as comments: its id, the line it
/// starts on, how many lines are under its tag, and its kind as its line writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Suppressed {
    pub id: String,
    pub line: usize,
    pub lines: usize,
    pub kind: String,
}

/// The statements a text holds suppressed (`#~ID LINE`), in order.
pub fn suppressed(src: &str) -> Vec<Suppressed> {
    let mut out: Vec<Suppressed> = Vec::new();
    for (k, l) in src.lines().enumerate() {
        let Some(rest) = l.strip_prefix("#~") else {
            continue;
        };
        let Some((id, line)) = rest.split_once(' ') else {
            continue;
        };
        // `#~ID<`, `#~ID>`: a statement Suppress put on a line of its own
        let id = id.trim_end_matches(['<', '>']);
        let at = match out.iter().position(|x| x.id == id) {
            Some(i) => i,
            None => {
                out.push(Suppressed {
                    id: id.to_string(),
                    line: k + 1,
                    lines: 0,
                    kind: String::new(),
                });
                out.len() - 1
            }
        };
        out[at].lines += 1;
        // the statement making it says its kind: `sketch id(`, or `id = call(`
        let line = line.trim_start();
        if out[at].kind.is_empty() {
            if line.starts_with(&format!("sketch {id}(")) {
                out[at].kind = "sketch".into();
            } else if let Some(call) = line
                .strip_prefix(id)
                .and_then(|r| r.trim_start().strip_prefix('='))
                .and_then(|r| r.trim_start().split('(').next())
            {
                out[at].kind = match call.trim() {
                    "cut" | "intersect" => "extrude".into(),
                    c => c.to_string(),
                };
            }
        }
    }
    out
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
/// A fix's button: what it does where the issue says, then its op.
pub fn fix_label(kind: &str, op: &Op) -> String {
    let what = match (kind, op) {
        // a taper whose walls meet before its depth: the depth to where they meet, or the
        // largest taper that still reaches the depth (either leaves the part with no lint)
        ("TaperCloses", Op::SetParam { value, .. }) if value.dim == Dim::Length => {
            "Depth to where it closes: "
        }
        ("TaperCloses", Op::SetParam { value, .. }) if value.dim == Dim::Angle => {
            "Largest taper that reaches the depth: "
        }
        _ => "",
    };
    format!("{what}{}", op_label(op))
}

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
        Op::Respell { to, .. } => format!("Write {to}"),
        Op::Suppress { id } => format!("Suppress {id}"),
        Op::Unsuppress { id } => format!("Unsuppress {id}"),
    }
}

/// A typed value: a number, its unit optional (`mm`, `deg`); else the param's own.
pub fn parse_value(text: &str, dim: Dim) -> Option<Literal> {
    parse_in(text, dim, "mm")
}

/// A typed value in a part shown in `doc` (`# units in`): a bare length is in it, a number may be a
/// fraction (`1/8`), and any length or angle unit may follow it (`3 in`, `0.5rad`); what comes
/// back is mm and degrees, as the text holds them.
pub fn parse_in(text: &str, dim: Dim, doc: &str) -> Option<Literal> {
    let t = text.trim();
    let (num, unit) = match t.find(|c: char| c.is_ascii_alphabetic() || c == '°') {
        Some(k) => (t[..k].trim(), t[k..].trim()),
        None => (t, ""),
    };
    let num = num.trim_start_matches('(').trim_end_matches(')');
    let value: f64 = match num.split_once('/') {
        Some((a, b)) => a.trim().parse::<f64>().ok()? / b.trim().parse::<f64>().ok()?,
        None => num.parse().ok()?,
    };
    if !value.is_finite() {
        return None;
    }
    let scaled = |word: &str| {
        let (d, top, bottom) = forge_lang::units::lookup(word)?;
        let dim = match d {
            forge_lang::units::LENGTH => Dim::Length,
            forge_lang::units::ANGLE => Dim::Angle,
            _ => return None,
        };
        Some(Literal {
            value: value * top / bottom,
            dim,
        })
    };
    match unit {
        "" if dim == Dim::Length => scaled(doc),
        "" => Some(Literal { value, dim }),
        "°" => scaled("deg"),
        w => scaled(w),
    }
}

/// The unit a part's text says it's shown in (`# units in`), if it says.
pub fn units_of(src: &str) -> Option<&'static str> {
    let says = src.lines().any(|l| {
        l.trim()
            .strip_prefix('#')
            .is_some_and(|n| n.trim().starts_with(forge_lang::units::LINE))
    });
    says.then(|| forge_lang::units::of_text(src))
}

/// The text with its units line saying `unit`: the line there changed, else one put first.
pub fn with_units(src: &str, unit: &str) -> String {
    let line = format!("# {}{unit}", forge_lang::units::LINE);
    let mut done = false;
    let mut out: Vec<String> = src
        .split_inclusive('\n')
        .map(|l| {
            let is = l
                .trim()
                .strip_prefix('#')
                .is_some_and(|n| n.trim().starts_with(forge_lang::units::LINE));
            if is && !done {
                done = true;
                let end = if l.ends_with('\n') { "\n" } else { "" };
                format!("{line}{end}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !done {
        out.insert(0, format!("{line}\n"));
    }
    out.concat()
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

    /// The row is one instance of a loop or call: the params it shares, each of which
    /// SplitInstance would give it alone.
    pub fn splittable(&self, key: &str) -> Vec<(Vec<Seg>, ParamKey)> {
        let Some(segs) = parse_path(key) else {
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

/// A suppressed statement's row: its icon and name greyed, struck through, and its menu's
/// Unsuppress.
fn suppressed_row(
    ui: &mut Ui,
    t: &super::theme::Theme,
    w: f32,
    sp: &Suppressed,
    fg: &mut FgPanel,
    menu_places: &mut Vec<(String, Rect)>,
    places: &mut Vec<(String, Rect)>,
) {
    let (r, resp) = ui.allocate_exact_size(vec2(w, super::ROW_H), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 0.0, t.hover);
    }
    let ink = t.ink2.gamma_multiply(0.7);
    let x = r.left() + 8.0;
    icons::draw(
        ui.painter(),
        pos2(x + 12.0, r.top() + 5.0),
        16.0,
        panel::icon(&sp.kind),
        1.5,
        ink,
    );
    let g = ui.fonts_mut(|f| f.layout_no_wrap(sp.id.clone(), sans(13.0), ink));
    let at = pos2(x + 34.0, r.center().y - g.size().y * 0.5);
    let wide = g.size().x;
    ui.painter().galley(at, g, ink);
    ui.painter().line_segment(
        [
            pos2(at.x - 1.0, r.center().y),
            pos2(at.x + wide + 1.0, r.center().y),
        ],
        Stroke::new(1.0, ink),
    );
    ui.painter().text(
        pos2(r.right() - 10.0, r.center().y),
        Align2::RIGHT_CENTER,
        "suppressed",
        sans(11.0),
        ink,
    );
    let resp = resp.on_hover_text(format!(
        "{} suppressed: {} line{} out of the build, kept as comments",
        sp.id,
        sp.lines,
        if sp.lines == 1 { "" } else { "s" }
    ));
    places.push((format!("fg:suppressed:{}", sp.id), r));
    let id = sp.id.clone();
    if resp.double_clicked() {
        fg.out.push(FgAct::Apply(Op::Unsuppress { id: id.clone() }));
    }
    resp.context_menu(|ui| {
        let b = ui.button(egui::RichText::new("Unsuppress Features").font(sans(12.0)));
        menu_places.push((format!("fg:menu:unsuppress:{id}"), b.rect));
        if b.clicked() {
            fg.out.push(FgAct::Apply(Op::Unsuppress { id: id.clone() }));
            ui.close();
        }
    });
}

/// The End of Part row: a rule across the browser; its menu (or a double click) takes it to the
/// end when it's rolled back.
fn end_of_part_row(
    ui: &mut Ui,
    t: &super::theme::Theme,
    w: f32,
    fg: &mut FgPanel,
    menu_places: &mut Vec<(String, Rect)>,
    places: &mut Vec<(String, Rect)>,
) {
    let (r, resp) = ui.allocate_exact_size(vec2(w, super::ROW_H), Sense::click());
    let rolled = fg.rollback.is_some();
    let ink = if rolled { WARN } else { t.ink2 };
    if resp.hovered() {
        ui.painter().rect_filled(r, 0.0, t.hover);
    }
    icons::draw(
        ui.painter(),
        pos2(r.left() + 20.0, r.top() + 5.0),
        16.0,
        "end",
        1.5,
        ink,
    );
    ui.painter().text(
        pos2(r.left() + 42.0, r.center().y),
        Align2::LEFT_CENTER,
        "End of Part",
        medium(13.0),
        ink,
    );
    ui.painter().line_segment(
        [
            pos2(r.left() + 140.0, r.center().y),
            pos2(r.right() - 10.0, r.center().y),
        ],
        Stroke::new(1.0, ink.gamma_multiply(0.6)),
    );
    places.push(("fg:row:endofpart".into(), r));
    if resp.double_clicked() && rolled {
        fg.out.push(FgAct::Rollback(None));
    }
    resp.context_menu(|ui| {
        if rolled {
            let b = ui.button(egui::RichText::new("Move End of Part to End").font(sans(12.0)));
            menu_places.push(("fg:menu:rollforward".into(), b.rect));
            if b.clicked() {
                fg.out.push(FgAct::Rollback(None));
                ui.close();
            }
        } else {
            ui.label(egui::RichText::new("At the end: every feature built").font(sans(12.0)));
        }
    });
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
        let sheet = !fg.sheets.is_empty();
        let mut flat_row = false;
        let stmts = statements(&fg.feats);
        let mut menu_places: Vec<(String, Rect)> = Vec::new();
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
                    // a sheet metal part's browser: Folded Model, and its Flat Pattern below
                    ui.painter().text(
                        pos2(r.left() + 42.0, r.center().y),
                        Align2::LEFT_CENTER,
                        if sheet { "Folded Model" } else { "Solid" },
                        medium(13.0),
                        ink,
                    );
                    if row_eye(ui, &t, r, "fg-eye-solid", !fg.body_hidden) {
                        eye_flip = Some(None);
                    }
                    places.push(("fg:eye:solid".into(), eye_rect(r)));
                    if sheet {
                        let (r, resp) =
                            ui.allocate_exact_size(vec2(area.width(), super::ROW_H), Sense::click());
                        let on = fg.flat;
                        if on {
                            ui.painter().rect_filled(r, 0.0, t.on);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, 0.0, t.hover);
                        }
                        icons::draw(
                            ui.painter(),
                            pos2(r.left() + 20.0, r.top() + 5.0),
                            16.0,
                            "flatpattern",
                            1.5,
                            if on { t.on_ink } else { t.ink },
                        );
                        ui.painter().text(
                            pos2(r.left() + 42.0, r.center().y),
                            Align2::LEFT_CENTER,
                            "Flat Pattern",
                            medium(13.0),
                            if on { t.on_ink } else { t.ink },
                        );
                        places.push(("fg:row:flat".into(), r));
                        if resp.double_clicked() || resp.clicked() {
                            flat_row = true;
                        }
                    }
                }
                let mut hidden: Option<String> = None;
                // the statements suppressed, each at its place among the rows; the End of Part
                // after the statement it's at (or last)
                let sups = suppressed(&fg.source);
                let mut sup_next = 0;
                let mut past_rollback = false;
                let mut eop_drawn = false;
                let first_line = |fg: &FgPanel, key: &str| {
                    fg.feats
                        .iter()
                        .filter(|f| {
                            f.id == key
                                || f.id.starts_with(&format!("{key}."))
                                || f.id.starts_with(&format!("{key}["))
                        })
                        .map(|f| f.line)
                        .min()
                        .unwrap_or(usize::MAX)
                };
                for row in &rows {
                    if row.depth == 0 {
                        let line = first_line(&fg, &row.key);
                        while sup_next < sups.len() && sups[sup_next].line < line {
                            suppressed_row(
                                ui,
                                &t,
                                area.width(),
                                &sups[sup_next],
                                &mut fg,
                                &mut menu_places,
                                &mut places,
                            );
                            sup_next += 1;
                        }
                        if past_rollback && !eop_drawn {
                            end_of_part_row(ui, &t, area.width(), &mut fg, &mut menu_places, &mut places);
                            eop_drawn = true;
                        }
                    }
                    if let Some(h) = &hidden {
                        if row.key.starts_with(h.as_str()) && row.key != *h {
                            continue;
                        }
                        hidden = None;
                    }
                    if fg.rollback.as_deref() == Some(row.key.as_str()) {
                        past_rollback = true;
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
                    let kind_of = row.feat.map(|i| fg.feats[i].kind.clone());
                    let kind = kind_of.as_deref();
                    let icon = match kind {
                        Some(k) => panel::icon(k),
                        None => "pattern",
                    };
                    let flag = fg.flag(&row.key);
                    let tint = match flag {
                        Some(Severity::Error) => Some(t.error),
                        Some(Severity::Lint | Severity::Warning | Severity::Violation) => Some(WARN),
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
                    let renaming = fg.renaming.as_ref().is_some_and(|n| n.0 == row.key);
                    if renaming {
                        let at = Rect::from_min_max(
                            pos2(x + 30.0, r.top() + 2.0),
                            pos2(r.right() - 60.0, r.bottom() - 2.0),
                        );
                        rename_edit(ui, &mut fg, at, &mut places);
                    } else {
                        p.text(
                            pos2(x + 34.0, r.center().y),
                            Align2::LEFT_CENTER,
                            &row.label,
                            font,
                            ink,
                        );
                    }
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
                        let at = pos2(r.right() - 4.0, r.top() + 6.0);
                        p.circle_filled(at, 3.0, c);
                        // what the dot means, on its own hover
                        let why = if flag == Some(Severity::Error) {
                            "An error here: the status bar's chip lists it"
                        } else {
                            "A warning here: the status bar's chip lists it"
                        };
                        ui.interact(
                            Rect::from_center_size(at, vec2(10.0, 10.0)),
                            ui.scope_id().with(("flag", &row.key)),
                            Sense::hover(),
                        )
                        .on_hover_text(why);
                    }
                    let is_sketch = kind == Some("sketch");
                    // Start Sketch waiting for a plane: a sketch row is the one to edit
                    if resp.clicked() && is_sketch && self.sketch_pick {
                        fg.out.push(FgAct::EditSketch(row.key.clone()));
                    } else if resp.clicked() {
                        chosen = Some(row.key.clone());
                    }
                    // a feature opens its tool's dialog, a sketch its sketch, as Inventor's
                    // double click does
                    if resp.double_clicked() && !renaming {
                        if is_sketch {
                            fg.out.push(FgAct::EditSketch(row.key.clone()));
                        } else if kind.is_some() && statement_of(&row.key) == row.key {
                            fg.out.push(FgAct::EditFeature(row.key.clone()));
                        }
                    }
                    let key = row.key.clone();
                    let edit = row.feat.is_some() && statement_of(&key) == key;
                    resp.context_menu(|ui| {
                        ui.set_min_width(150.0);
                        let mut item = |ui: &mut Ui, label: &str, name: &str| {
                            let b = ui.button(egui::RichText::new(label).font(sans(12.0)));
                            menu_places.push((format!("fg:menu:{name}:{key}"), b.rect));
                            let hit = b.clicked();
                            if hit {
                                ui.close();
                            }
                            hit
                        };
                        if is_sketch {
                            if item(ui, "Edit Sketch", "edit-sketch") {
                                fg.out.push(FgAct::EditSketch(key.clone()));
                            }
                        } else if edit && item(ui, "Edit Feature", "edit") {
                            fg.out.push(FgAct::EditFeature(key.clone()));
                        }
                        let at = stmts.iter().position(|s| *s == key);
                        if let Some(k) = at {
                            if item(ui, "Rename", "rename") {
                                fg.renaming = Some((key.clone(), key.clone()));
                                fg.rename_fresh = true;
                            }
                            if k > 0 && item(ui, "Move Up", "up") {
                                fg.out.push(FgAct::Apply(Op::MoveFeature {
                                    id: key.clone(),
                                    before: Some(stmts[k - 1].clone()),
                                }));
                            }
                            if k + 1 < stmts.len() && item(ui, "Move Down", "down") {
                                fg.out.push(FgAct::Apply(Op::MoveFeature {
                                    id: key.clone(),
                                    before: stmts.get(k + 2).cloned(),
                                }));
                            }
                            ui.separator();
                            if item(ui, "Suppress Features", "suppress") {
                                fg.out.push(FgAct::Apply(Op::Suppress { id: key.clone() }));
                            }
                            // Inventor's End of Part dragged here: what follows isn't built
                            if fg.rollback.as_deref() != Some(key.as_str())
                                && item(ui, "Move End of Part Here", "rollback")
                            {
                                fg.out.push(FgAct::Rollback(Some(key.clone())));
                            }
                            if item(ui, "Delete", "delete") {
                                fg.out.push(FgAct::Apply(Op::DeleteFeature { id: key.clone() }));
                            }
                        }
                        // one instance of a loop or a call: a shared param of its own
                        for (id, param) in fg.splittable(&key) {
                            let op = Op::SplitInstance { id, param };
                            let name = match &op {
                                Op::SplitInstance { param, .. } => format!("split:{param}"),
                                _ => unreachable!("just made"),
                            };
                            if item(ui, &op_label(&op), &name) {
                                fg.out.push(FgAct::Apply(op));
                            }
                        }
                    });
                    places.push((format!("fg:row:{}", row.key), r));
                    if kind == Some("thermoform") {
                        super::thermo_ui::feature_row(&resp, &mut fg, &mut places, &row.key);
                        let x0 = x - r.left();
                        let body = super::thermo_ui::tree_children(
                            ui,
                            &t,
                            &mut fg,
                            area.width(),
                            &row.key,
                            x0,
                            &mut places,
                        );
                        // a tool waiting for a body takes the one clicked
                        if body.is_some() && fg.tool.is_some() {
                            chosen = body;
                        }
                    }
                    if row.group && !open {
                        hidden = Some(row.key.clone());
                    }
                }
                for sp in &sups[sup_next.min(sups.len())..] {
                    suppressed_row(ui, &t, area.width(), sp, &mut fg, &mut menu_places, &mut places);
                }
                if !eop_drawn && !fg.feats.is_empty() {
                    end_of_part_row(ui, &t, area.width(), &mut fg, &mut menu_places, &mut places);
                }
            });
        if let Some(k) = toggle
            && !fg.collapsed.remove(&k)
        {
            fg.collapsed.insert(k);
        }
        if flat_row {
            self.out.push(super::Action::Sheet(if fg.flat {
                super::sheet::SheetCmd::Folded
            } else {
                super::sheet::SheetCmd::FlatPattern
            }));
        }
        places.extend(menu_places);
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
            let work = fg
                .feats
                .iter()
                .find(|f| f.id == k)
                .map_or("", |f| f.kind.as_str());
            let made = |call: &str| work == "work" && fg.source.contains(&format!("{k} = {call}("));
            let pick = match kind {
                SlotKind::Profile | SlotKind::Profiles | SlotKind::Plane | SlotKind::UpTo if is_sketch => {
                    Ok(k.clone())
                }
                SlotKind::Plane | SlotKind::UpTo if made("workplane") => Ok(k.clone()),
                // a sketch for a hole: every point it has
                SlotKind::Points if is_sketch => Ok(format!("{k}.points")),
                SlotKind::RevAxis if made("workaxis") => Ok(k.clone()),
                SlotKind::Feature | SlotKind::Features if !is_sketch => Ok(k.clone()),
                // a weldment's member, by its own row (`frame[2]`)
                SlotKind::Member | SlotKind::Members if work == "member" => Ok(k.clone()),
                // an end trimmed to a sketch's or work plane's plane, a member or a body
                SlotKind::TrimTo => Ok(k.clone()),
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
        let units = self.units;
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
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                close = super::fgasm::dialog_head(ui, &t, "params", "Parameters", "fg:params", places);
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
                                    Self::param_row(ui, &t, &mut fg, p, units, &mut out, places);
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
        units: &str,
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
                    match parse_in(&typed, dim, units) {
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
                    // a length shows in mm, as the file has it, in a part shown in another unit
                    let shown = if dim == Dim::Length && units != "mm" {
                        format!("{} mm", short(value))
                    } else {
                        short(value)
                    };
                    fg.editing = Some((p.key.clone(), shown));
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

    /// The document's issues, over the status bar: each with its fixes (a shop rule's too), and an
    /// assembly's with a Show.
    pub(super) fn fg_issues_window(&mut self, ctx: &egui::Context) {
        let Some(mut fg) = self.fg.take() else { return };
        if !fg.issues_list {
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
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                close = super::fgasm::dialog_head(ui, &t, "warning", "Issues", "fg:issueswin", places);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height((v.height() * 0.45).max(160.0))
                    .show(ui, |ui| {
                        Self::fg_issues(ui, &t, &mut fg, &mut out, places);
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

    fn fg_issues(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let (errors, warnings) = issue_counts(&fg.issues);
        let head = format!("{} · {}", plural(errors, "error"), plural(warnings, "warning"));
        let ink = if errors > 0 {
            t.error
        } else if warnings > 0 {
            WARN
        } else {
            t.ink2
        };
        let r = ui.label(egui::RichText::new(head).font(medium(12.0)).color(ink));
        places.push(("fg:issues".into(), r.rect));
        ui.add_space(super::theme::S1);
        let issues = std::mem::take(&mut fg.issues);
        // a long list shows its first few until asked
        let cap = if fg.all_issues { 300 } else { 8 };
        // an assembly's notes (intended contacts) are counted in Analyze Interference, not here
        let note = is_note;
        for (k, (i, is)) in issues
            .iter()
            .enumerate()
            .filter(|x| !note(x.1))
            .take(cap)
            .enumerate()
        {
            let colour = match is.severity {
                Severity::Error => t.error,
                _ => WARN,
            };
            let word = match is.severity {
                Severity::Error => "Error",
                Severity::Lint => "Lint",
                Severity::Warning => "Warning",
                Severity::Violation => "Violation",
            };
            // each issue a row of its own: a rule between, its severity in words as well as colour
            if k > 0 {
                ui.add_space(super::theme::S1);
                ui.separator();
            }
            ui.add_space(super::theme::S1);
            let r = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = super::theme::S2;
                    egui::Frame::new()
                        .fill(colour.gamma_multiply(0.16))
                        .corner_radius(super::theme::R1)
                        .inner_margin(egui::Margin::symmetric(6, 1))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(word).font(medium(11.0)).color(colour));
                        });
                    // where it is, when a statement says it (an assembly's solve says none)
                    let at = if is.line == 0 {
                        is.kind.clone()
                    } else {
                        format!("{} · line {}:{}", is.kind, is.line, is.col)
                    };
                    ui.label(
                        egui::RichText::new(at)
                            .font(super::theme::caption())
                            .color(t.ink2),
                    );
                })
                .response;
            places.push((format!("fg:issue:{i}"), r.rect));
            ui.add_space(2.0);
            ui.label(egui::RichText::new(&is.text).font(sans(12.0)).color(t.ink));
            if !is.fixes.is_empty() || is.asm.is_some() {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(super::theme::S1 + 2.0, super::theme::S1);
                    for (j, op) in is.fixes.iter().enumerate() {
                        if Self::fg_button(ui, &fix_label(&is.kind, op), format!("fg:fix:{i}:{j}"), places) {
                            out.push(FgAct::Apply(op.clone()));
                        }
                    }
                    if let Some(ai) = &is.asm {
                        Self::asm_show_button(ui, &mut fg.asm, ai, i, out, places);
                    }
                });
            }
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
}

/// Is `word` in `line` as a name of its own (not part of a longer one)?
fn names(line: &str, word: &str) -> bool {
    let id = |c: char| c.is_ascii_alphanumeric() || c == '_';
    line.match_indices(word).any(|(k, _)| {
        let before = line[..k].chars().next_back();
        let after = line[k + word.len()..].chars().next();
        // `base.side(..)` is a member, not the name
        !before.is_some_and(|c| id(c) || c == '.') && !after.is_some_and(id)
    })
}

/// The sketches nothing else in the text reads: what a new extrude can take.
pub fn free_sketches(src: &str, sketches: &[forge_lang::anchors::SketchRefs]) -> Vec<String> {
    let lines: Vec<&str> = src.lines().collect();
    sketches
        .iter()
        .map(|s| s.id.as_str())
        .filter(|id| !id.contains(['.', '[']))
        .filter(|id| {
            // its own block: from its head to the brace closing it, which are its
            let head = format!("sketch {id}(");
            let Some(lo) = lines.iter().position(|l| l.trim_start().starts_with(&head)) else {
                return false;
            };
            let hi = (lo..lines.len())
                .find(|&k| lines[k].starts_with('}'))
                .unwrap_or(lo);
            !lines
                .iter()
                .enumerate()
                .filter(|(k, _)| *k < lo || *k > hi)
                .any(|(_, l)| {
                    let l = l.trim_start();
                    // its own params are its, not readers of it
                    let own =
                        l.starts_with(&format!("param {id}.")) || l.starts_with(&format!("param {id}["));
                    !l.starts_with('#') && !own && names(l, id)
                })
        })
        .map(String::from)
        .collect()
}

/// A Forge file is built and shown posed, as an assembly: it holds components or joints (an
/// `assembly` first line is optional; a text that does not parse goes by that line). The browser
/// asks it too.
pub fn is_assembly(src: &str) -> bool {
    match forge_lang::syntax::parse(src) {
        Ok(p) => forge_lang::asm::has_components(&p),
        Err(_) => src
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .is_some_and(|l| l == "assembly"),
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
    fn a_sketch_nothing_reads_is_free() {
        let refs = |ids: &[&str]| -> Vec<forge_lang::anchors::SketchRefs> {
            ids.iter()
                .map(|i| forge_lang::anchors::SketchRefs {
                    id: i.to_string(),
                    ..Default::default()
                })
                .collect()
        };
        let src = "sketch plan(on: xy) {\n  r = rect(corner: (0, 0), size: (4, 4))\n}\nbase = extrude(plan, 2)\nparam top.r = 1\nsketch top(on: base.end) {\n  c = circle((1, 1), r)\n}\nsketch topper(on: base.end) {\n}\n";
        assert_eq!(
            free_sketches(src, &refs(&["plan", "top", "topper"])),
            ["top", "topper"]
        );
        // a member of the same word reads no sketch
        let src = format!(
            "sketch side(on: xy) {{\n}}\n{src}e = extrude(top.c, 1)\ns = shell(1, open: e.side(top.c))\n"
        );
        assert_eq!(free_sketches(&src, &refs(&["side", "top"])), ["side"]);
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
        assert_eq!(parse_value("3 in", Dim::Length).map(|l| l.value), Some(76.2));
        assert_eq!(parse_in("1/8", Dim::Length, "in").map(|l| l.value), Some(3.175));
        assert_eq!(parse_in("2 mm", Dim::Length, "in").map(|l| l.value), Some(2.0));
        assert_eq!(
            parse_in("30", Dim::Angle, "in").map(|l| (l.value, l.dim)),
            Some((30.0, Dim::Angle))
        );
        assert_eq!(parse_value("3 lb", Dim::Length), None);
    }

    #[test]
    fn a_parts_unit_is_one_line_of_its_text() {
        let src = "shared t = 6\n";
        assert_eq!(units_of(src), None);
        let inch = with_units(src, "in");
        assert_eq!(inch, "# units in\nshared t = 6\n");
        assert_eq!(units_of(&inch), Some("in"));
        assert_eq!(with_units(&inch, "cm"), "# units cm\nshared t = 6\n");
        assert_eq!(with_units(&inch, "in"), inch);
        // nothing it builds changes
        let run = |t: &str| forge_lang::run(t, forge_lang::Options::default(), Default::default()).0;
        assert!(run(&inch).errors.is_empty());
    }
}
