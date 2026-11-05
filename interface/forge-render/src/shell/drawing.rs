//! The Drawing environment, Inventor's: the sheet with its views and annotations, the Place Views
//! and Annotate tabs, the sheet browser, the tools' dialogs and picks. Every edit is a typed op on
//! the drawing file (`forge_lang::drawing::ops`), checked before it is written; the sheet is made
//! again from the file in the background, so the window never waits on the kernel.

use super::theme::{mono, sans, semi};
use super::{Action, Shell, icons};
use egui::{Align2, Color32, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_draw::page::text_width;
use forge_draw::{Align, Geom2, Pen};
use forge_lang::drawing::doc::{self, AnnotKind, ItemKind};
use forge_lang::drawing::ops::{DrawOp, apply_checked};
use forge_lang::drawing::regen::{self, Drawing};
use forge_math::{Point2, Vec2};
use std::sync::mpsc::{Receiver, channel};

/// The Drawing environment's tabs.
pub const TABS: [&str; 4] = ["Place Views", "Annotate", "Sketch", "Manage"];

/// A drawing ribbon command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DwgCmd {
    BaseView,
    Projected,
    Auxiliary,
    Section,
    Detail,
    Break,
    BreakOut,
    Crop,
    Dimension,
    Baseline,
    Ordinate,
    Chain,
    HoleNote,
    CenterMark,
    CenterMarks,
    Centerline,
    Bisector,
    Text,
    LeaderText,
    Datum,
    DatumTarget,
    Fcf,
    Surface,
    Weld,
    Edge,
    Balloon,
    AutoBalloon,
    PartsList,
    ExportPdf,
    ExportDxf,
    Fit,
    Update,
    NewSheet,
}

impl DwgCmd {
    pub fn name(self) -> &'static str {
        match self {
            DwgCmd::BaseView => "Base",
            DwgCmd::Projected => "Projected",
            DwgCmd::Auxiliary => "Auxiliary",
            DwgCmd::Section => "Section",
            DwgCmd::Detail => "Detail",
            DwgCmd::Break => "Break",
            DwgCmd::BreakOut => "Break Out",
            DwgCmd::Crop => "Crop",
            DwgCmd::Dimension => "Dimension",
            DwgCmd::Baseline => "Baseline",
            DwgCmd::Ordinate => "Ordinate",
            DwgCmd::Chain => "Chain",
            DwgCmd::HoleNote => "Hole and Thread",
            DwgCmd::CenterMark => "Center Mark",
            DwgCmd::CenterMarks => "Automated Centerlines",
            DwgCmd::Centerline => "Centerline",
            DwgCmd::Bisector => "Bisector",
            DwgCmd::Text => "Text",
            DwgCmd::LeaderText => "Leader Text",
            DwgCmd::Datum => "Datum Identifier",
            DwgCmd::DatumTarget => "Datum Target",
            DwgCmd::Fcf => "Feature Control Frame",
            DwgCmd::Surface => "Surface",
            DwgCmd::Weld => "Welding",
            DwgCmd::Edge => "Edge",
            DwgCmd::Balloon => "Balloon",
            DwgCmd::AutoBalloon => "Auto Balloon",
            DwgCmd::PartsList => "Parts List",
            DwgCmd::ExportPdf => "Export PDF",
            DwgCmd::ExportDxf => "Export DXF",
            DwgCmd::Fit => "Zoom All",
            DwgCmd::Update => "Update",
            DwgCmd::NewSheet => "New Sheet",
        }
    }

    /// What the tool asks for next, by how much has been picked.
    fn prompt(self, step: usize) -> &'static str {
        use DwgCmd::*;
        match (self, step) {
            (BaseView, _) => "Click on the sheet to place the base view",
            (Projected | Section | Detail | Auxiliary | Crop | Break | AutoBalloon | CenterMarks, 0) => {
                "Select a view"
            }
            (Projected, _) => "Move off the parent view and click to place a projected view; Esc to finish",
            (Section, 1) => "Click the cutting line's first point",
            (Section, 2) => "Click the cutting line's second point",
            (Detail, 1) => "Click the detail's centre",
            (Detail, 2) => "Click to set the fence's radius",
            (Auxiliary, 1) => "Select a straight edge to look square to",
            (Section | Detail | Auxiliary, _) => "Click to place the view",
            (Crop, 1) => "Click a corner of the crop boundary",
            (Crop, _) => "Click the opposite corner",
            (Break, 1) => "Click where the break starts",
            (Break, _) => "Click where the break ends",
            (Dimension, 0) => "Select an edge, circle or point",
            (Dimension, 1) => "Select a second edge, or click to place",
            (Dimension, _) => "Click to place the dimension",
            (Baseline | Ordinate | Chain, 0) => "Select the base edge",
            (Baseline | Ordinate | Chain, _) => {
                "Select the features to dimension; Enter or Done, then click to place"
            }
            (HoleNote | CenterMark, 0) => "Select a hole's circle",
            (Centerline, 0 | 1) => "Select two circles",
            (Bisector, 0 | 1) => "Select two edges",
            (Text, _) => "Click to place the text",
            (Balloon, 0) => "Select a part's edge",
            (PartsList, 0) => "Select a view of the assembly",
            (PartsList, _) => "Click the parts list's lower right corner",
            (_, 0) => "Select an edge or a dimension",
            _ => "Click to place",
        }
    }
}

/// What was picked on the sheet: a model reference on a view, or an annotation.
#[derive(Clone, Debug, PartialEq)]
struct Pick {
    view: String,
    /// The reference as the document writes it, or an item's id.
    text: String,
    kind: &'static str,
    /// Where it was clicked (sheet mm).
    at: Point2,
    /// The occurrence it is in (assemblies).
    occ: String,
    /// What to light (sheet mm).
    geom: Option<Geom2>,
}

/// A dialog that is up.
#[derive(Clone, Debug, PartialEq)]
enum Dlg {
    Base {
        dir: usize,
        scale: usize,
        hidden: bool,
        tangent: bool,
        label: String,
    },
    Text {
        text: String,
        /// The op the text completes.
        then: TextFor,
    },
    Fcf(FcfForm),
    Surface {
        ra: String,
        process: usize,
        lay: String,
    },
    Weld {
        kind: usize,
        side: usize,
        size: String,
        length: String,
        all_around: bool,
        field: bool,
        tail: String,
    },
    Datum {
        letter: String,
    },
    EditDim {
        id: String,
        text: String,
        tol: usize,
        a: String,
        b: String,
        places: String,
    },
    EditView {
        id: String,
        scale: String,
        hidden: bool,
        label: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum TextFor {
    Note(Point2),
    Leader(Pick, Point2),
    Edit(String),
}

#[derive(Clone, Debug, PartialEq)]
struct FcfForm {
    ch: usize,
    dia: bool,
    tol: String,
    mods: [bool; 7],
    datums: [String; 3],
    all_around: bool,
}

impl Default for FcfForm {
    fn default() -> FcfForm {
        FcfForm {
            ch: 9,
            dia: true,
            tol: "0.1".into(),
            mods: [false; 7],
            datums: ["A".into(), "B".into(), "C".into()],
            all_around: false,
        }
    }
}

const MODS: [&str; 7] = ["M", "L", "P", "F", "T", "U", "ST"];
const DIRS: [(&str, &str); 8] = [
    ("Front", "front"),
    ("Top", "top"),
    ("Right", "right"),
    ("Back", "back"),
    ("Bottom", "bottom"),
    ("Left", "left"),
    ("Iso Top Right", "iso"),
    ("Iso Top Left", "iso_left"),
];
const SCALES: [(&str, f64); 10] = [
    ("1:1", 1.0),
    ("1:2", 0.5),
    ("1:3", 1.0 / 3.0),
    ("1:4", 0.25),
    ("1:5", 0.2),
    ("1:8", 0.125),
    ("1:10", 0.1),
    ("1:20", 0.05),
    ("2:1", 2.0),
    ("4:1", 4.0),
];
const TOLS: [&str; 7] = [
    "Default",
    "Symmetric",
    "Deviation",
    "Limits",
    "Basic",
    "Reference",
    "Fit",
];

/// A drag under way: what moves, from where (sheet mm), and by how far.
#[derive(Clone, Debug)]
struct Drag {
    id: String,
    from: Point2,
    by: Vec2,
}

/// The drawing file, made, and the environment's state.
/// What the maker sends: the sheets so far, then the drawing.
enum Made {
    Part(regen::Shown),
    Done(Drawing),
}

pub struct DrawingEnv {
    pub path: String,
    pub src: String,
    pub made: Option<Drawing>,
    pending: Option<Receiver<Made>>,
    /// While making: the views made of how many, and whether what is shown is only a preview.
    pub progress: Option<(usize, usize)>,
    started: std::time::Instant,
    previewing: bool,
    /// The files its last make read (its models and theirs), for the host's watch.
    pub deps: std::sync::Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>,
    pub sheet: usize,
    /// Screen pixels per sheet millimetre; where the sheet's origin (lower left) is on screen.
    zoom: f32,
    origin: Pos2,
    fitted: bool,
    pub selected: Option<String>,
    hover: Option<Pick>,
    pub tool: Option<DwgCmd>,
    picks: Vec<Pick>,
    pts: Vec<Point2>,
    dlg: Option<Dlg>,
    /// A dialog's values after its OK, while its tool picks and places.
    kept: Option<Dlg>,
    drag: Option<Drag>,
    /// The base view dialog's last choices, and the frame's.
    base_dir: usize,
    base_scale: usize,
    fcf: FcfForm,
    /// Each stroke of the sheet shown, as points in sheet mm, made once per sheet made.
    lines: Vec<(Pen, Vec<Point2>)>,
    lines_for: Option<(usize, usize)>,
    made_n: usize,
    pub message: String,
    pub busy: bool,
    pointer: Option<Point2>,
    /// The file as it was before each edit, and after each undone one.
    undo: Vec<String>,
    redo: Vec<String>,
    /// Steps written from outside (the Agent tab): the undo depth each made, its label and key.
    tags: Vec<(usize, String, u64)>,
}

fn name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

/// A number as a drawing file writes it.
fn num(x: f64) -> String {
    forge_draw::annot::number(x, 3)
}

fn pt(p: Point2) -> String {
    format!("({}, {})", num(p.x), num(p.y))
}

impl DrawingEnv {
    pub fn open(path: &str) -> Result<DrawingEnv, String> {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let mut e = DrawingEnv {
            path: path.to_string(),
            src,
            made: None,
            pending: None,
            progress: None,
            started: std::time::Instant::now(),
            previewing: false,
            deps: Default::default(),
            sheet: 0,
            zoom: 2.0,
            origin: pos2(0.0, 0.0),
            fitted: false,
            selected: None,
            hover: None,
            tool: None,
            picks: vec![],
            pts: vec![],
            dlg: None,
            kept: None,
            drag: None,
            base_dir: 0,
            base_scale: 0,
            fcf: FcfForm::default(),
            lines: vec![],
            lines_for: None,
            made_n: 0,
            message: String::new(),
            busy: false,
            pointer: None,
            undo: vec![],
            redo: vec![],
            tags: vec![],
        };
        e.regen();
        Ok(e)
    }

    pub fn name(&self) -> String {
        name_of(&self.path)
    }

    /// Makes the sheet again from the file, in the background.
    /// The drawing's text as it is on disk now, or its model changed: made again.
    pub fn reload(&mut self, src: String) {
        self.src = src;
        self.regen();
    }

    fn regen(&mut self) {
        let (send, recv) = channel();
        let src = self.src.clone();
        let dir = std::path::Path::new(&self.path)
            .canonicalize()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        let file = dir.join(std::path::Path::new(&self.path).file_name().unwrap_or_default());
        let deps = self.deps.clone();
        std::thread::spawn(move || {
            let watch = forge_lang::deps::watch();
            let d = regen::shown(&src, &file, &mut |s| {
                let _ = send.send(Made::Part(s));
            });
            let read: Vec<std::path::PathBuf> = watch
                .done()
                .0
                .into_iter()
                .filter(|(_, seen)| !matches!(seen, forge_lang::deps::Seen::There))
                .map(|(p, _)| p)
                .collect();
            if !read.is_empty() {
                *deps.lock().unwrap_or_else(|e| e.into_inner()) = read;
            }
            let _ = send.send(Made::Done(d));
        });
        self.pending = Some(recv);
        self.busy = true;
        self.progress = None;
        self.started = std::time::Instant::now();
    }

    /// Takes a finished make, if there is one: true when the sheet changed.
    pub fn poll(&mut self) -> bool {
        let Some(r) = &self.pending else { return false };
        let mut changed = false;
        loop {
            match r.try_recv() {
                Ok(Made::Part(s)) => {
                    self.progress = Some((s.done, s.total));
                    // the sheets as they come, only until a whole drawing is on screen
                    if self.made.is_none() || self.previewing {
                        self.made = Some(s.drawing);
                        self.made_n += 1;
                        self.previewing = true;
                        changed = true;
                    }
                }
                Ok(Made::Done(d)) => {
                    self.made = Some(d);
                    self.made_n += 1;
                    self.pending = None;
                    self.busy = false;
                    self.progress = None;
                    self.previewing = false;
                    if let Some(d) = &self.made {
                        self.sheet = self.sheet.min(d.pages.len().saturating_sub(1));
                    }
                    return true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return changed,
                Err(_) => {
                    self.pending = None;
                    self.busy = false;
                    self.progress = None;
                    self.message = "The drawing could not be made".into();
                    return true;
                }
            }
        }
    }

    /// Applies typed ops to the file, all or none, writes it and makes the sheet again.
    fn apply(&mut self, ops: Vec<DrawOp>) {
        let mut src = self.src.clone();
        for op in &ops {
            match apply_checked(&src, op) {
                Ok(s) => src = s,
                Err(e) => {
                    self.message = e.text();
                    return;
                }
            }
        }
        if let Err(e) = std::fs::write(&self.path, &src) {
            self.message = format!("{}: {e}", self.path);
            return;
        }
        self.undo.push(std::mem::replace(&mut self.src, src));
        self.redo.clear();
        self.tags.retain(|t| t.0 < self.undo.len());
        self.message.clear();
        self.regen();
    }

    /// The file's text from outside (the Agent tab) as one labelled undo step, a later text under
    /// the same key joining it; written unless the file holds it already.
    pub fn external(&mut self, text: String, label: &str, key: u64, on_disk: bool) {
        if text == self.src {
            return;
        }
        if !on_disk && let Err(e) = std::fs::write(&self.path, &text) {
            self.message = format!("{}: {e}", self.path);
            return;
        }
        let joins = self
            .tags
            .last()
            .is_some_and(|t| t.2 == key && t.0 == self.undo.len() && self.redo.is_empty());
        if joins {
            self.src = text;
        } else {
            self.undo.push(std::mem::replace(&mut self.src, text));
            self.redo.clear();
            self.tags.retain(|t| t.0 < self.undo.len());
            self.tags.push((self.undo.len(), label.to_string(), key));
        }
        self.message.clear();
        self.regen();
    }

    /// The label and key of the step Undo would take back, if it came from outside.
    pub fn undo_tag(&self) -> Option<(&str, u64)> {
        let t = self.tags.iter().rev().find(|t| t.0 == self.undo.len())?;
        Some((&t.1, t.2))
    }

    /// Undo (or redo) the last edit: the file goes back as it was.
    pub fn undo(&mut self, redo: bool) {
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        let Some(src) = from.pop() else { return };
        to.push(std::mem::replace(&mut self.src, src));
        if let Err(e) = std::fs::write(&self.path, &self.src) {
            self.message = format!("{}: {e}", self.path);
        }
        self.regen();
    }

    /// A fresh item id: `prefix` and the first number free.
    fn fresh(&self, prefix: &str) -> String {
        let d = doc::read(&self.src);
        (1..)
            .map(|k| format!("{prefix}{k}"))
            .find(|n| d.item(n).is_none())
            .unwrap_or_else(|| prefix.to_string())
    }

    fn add(&mut self, prefix: &str, call: String) {
        let id = self.fresh(prefix);
        self.apply(vec![DrawOp::AddItem {
            text: format!("{id} = {call}"),
        }]);
        self.selected = Some(id);
    }

    fn sheet_id(&self) -> String {
        self.made
            .as_ref()
            .and_then(|d| d.sheets.get(self.sheet).cloned())
            .unwrap_or_else(|| "sheet".into())
    }

    fn model_id(&self) -> Option<String> {
        doc::read(&self.src).items.iter().find_map(|i| match i.kind {
            ItemKind::Model { .. } => Some(i.id.clone()),
            _ => None,
        })
    }

    fn to_sheet(&self, p: Pos2) -> Point2 {
        Point2::new(
            ((p.x - self.origin.x) / self.zoom) as f64,
            ((self.origin.y - p.y) / self.zoom) as f64,
        )
    }

    fn to_screen(&self, q: Point2) -> Pos2 {
        pos2(
            self.origin.x + q.x as f32 * self.zoom,
            self.origin.y - q.y as f32 * self.zoom,
        )
    }

    fn fit(&mut self, canvas: Rect) {
        let Some(page) = self.made.as_ref().and_then(|d| d.pages.get(self.sheet)) else {
            return;
        };
        let z = ((canvas.width() - 40.0) / page.w as f32).min((canvas.height() - 40.0) / page.h as f32);
        self.zoom = z.max(0.05);
        let size = vec2(page.w as f32 * z, page.h as f32 * z);
        let lo = canvas.center() - size * 0.5;
        self.origin = pos2(lo.x, lo.y + size.y);
        self.fitted = true;
    }

    /// The views on the shown sheet.
    fn views(&self) -> Vec<&regen::ViewOut> {
        self.made.as_ref().map_or(vec![], |d| {
            d.views.iter().filter(|v| v.sheet == self.sheet).collect()
        })
    }

    fn view_at(&self, q: Point2) -> Option<String> {
        self.views()
            .into_iter()
            .find(|v| {
                q.x >= v.lo.x - 3.0 && q.x <= v.hi.x + 3.0 && q.y >= v.lo.y - 3.0 && q.y <= v.hi.y + 3.0
            })
            .map(|v| v.id.clone())
    }

    /// The model edge (or face outline) nearest `q` within `tol` sheet mm, on any view of the sheet.
    fn pick_edge(&self, q: Point2, tol: f64) -> Option<Pick> {
        let d = self.made.as_ref()?;
        let mut best: Option<(f64, Pick)> = None;
        for v in self.views() {
            let mq = v.place.inverse_point(q);
            let model = &d.models[v.model].model;
            for p in &v.pieces {
                let pts = p.geom.points(0.05);
                let dist = pts
                    .windows(2)
                    .map(|w| {
                        let ab = w[1] - w[0];
                        let t = ((mq - w[0]).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
                        mq.distance(w[0] + ab * t)
                    })
                    .fold(f64::INFINITY, f64::min)
                    * v.scale;
                // a click inside a hole picks its circle, if no edge is nearer
                let dist = match p.geom.circle() {
                    Some((c, r)) if mq.distance(c) < r => dist.min(0.9 * tol),
                    _ => dist,
                };
                if dist > tol || best.as_ref().is_some_and(|b| b.0 <= dist) {
                    continue;
                }
                let occ_i = v.insts[p.inst].2;
                let occ = if model.paths.len() > 1 {
                    model.paths[occ_i].clone()
                } else {
                    String::new()
                };
                let (face, n) = match p.source {
                    forge_draw::Source::Edge(n) => (false, n),
                    forge_draw::Source::Outline(n) => (true, n),
                };
                let r = doc::Ref {
                    occ: occ.clone(),
                    name: n,
                    face,
                    at: None,
                    span: forge_lang::syntax::Span::new(0, 0),
                };
                let kind = if face {
                    "outline"
                } else if p.geom.circle().is_some() {
                    "circle"
                } else if matches!(p.geom, Geom2::Line { .. }) {
                    "line"
                } else {
                    "curve"
                };
                best = Some((
                    dist,
                    Pick {
                        view: v.id.clone(),
                        text: r.text(),
                        kind,
                        at: q,
                        occ,
                        geom: Some(p.geom.placed(&v.place)),
                    },
                ));
            }
        }
        best.map(|b| b.1)
    }

    /// The annotation (not a view) whose box holds `q`.
    fn item_at(&self, q: Point2) -> Option<String> {
        let d = self.made.as_ref()?;
        d.items
            .iter()
            .filter(|i| {
                i.sheet == self.sheet
                    && !matches!(i.kind, "base" | "projected" | "section" | "detail" | "flat")
            })
            .filter(|i| {
                q.x >= i.lo.x - 1.0 && q.x <= i.hi.x + 1.0 && q.y >= i.lo.y - 1.0 && q.y <= i.hi.y + 1.0
            })
            .min_by(|a, b| {
                let area = |i: &&regen::ItemOut| (i.hi.x - i.lo.x) * (i.hi.y - i.lo.y);
                area(a).total_cmp(&area(b))
            })
            .map(|i| i.id.clone())
    }

    /// Where `q` (sheet mm) is in a view's own coordinates.
    fn in_view(&self, view: &str, q: Point2) -> Point2 {
        self.made
            .as_ref()
            .and_then(|d| d.view(view))
            .map_or(q, |v| v.unmap(q))
    }

    /// The next datum letter no datum symbol on the drawing has.
    fn free_letter(&self) -> String {
        let d = doc::read(&self.src);
        let taken: Vec<String> = d
            .items
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::Annot(a) => match &a.kind {
                    AnnotKind::Datum { letter, .. } => Some(letter.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        ('A'..='Z')
            .filter(|c| !"IOQ".contains(*c))
            .map(|c| c.to_string())
            .find(|c| !taken.contains(c))
            .unwrap_or_else(|| "Z".into())
    }

    pub fn start(&mut self, c: DwgCmd) {
        self.picks.clear();
        self.pts.clear();
        self.message.clear();
        match c {
            DwgCmd::ExportPdf | DwgCmd::ExportDxf => {
                self.export(c == DwgCmd::ExportPdf);
                return;
            }
            DwgCmd::Update => {
                if let Ok(s) = std::fs::read_to_string(&self.path) {
                    self.src = s;
                }
                self.regen();
                return;
            }
            DwgCmd::Fit => {
                self.fitted = false;
                return;
            }
            DwgCmd::NewSheet => {
                let n = self.made.as_ref().map_or(1, |d| d.sheets.len() + 1);
                let id = self.fresh("sheet");
                self.apply(vec![DrawOp::AddItem {
                    text: format!("{id} = sheet(\"B\", title: \"SHEET {n}\")"),
                }]);
                self.sheet = n - 1;
                return;
            }
            _ => {}
        }
        self.tool = Some(c);
        match c {
            DwgCmd::BaseView => {
                self.dlg = Some(Dlg::Base {
                    dir: self.base_dir,
                    scale: self.base_scale,
                    hidden: true,
                    tangent: false,
                    label: String::new(),
                });
                self.tool = None;
            }
            DwgCmd::Fcf => self.dlg = Some(Dlg::Fcf(self.fcf.clone())),
            DwgCmd::Surface => {
                self.dlg = Some(Dlg::Surface {
                    ra: "1.6".into(),
                    process: 1,
                    lay: String::new(),
                })
            }
            DwgCmd::Weld => {
                self.dlg = Some(Dlg::Weld {
                    kind: 0,
                    side: 0,
                    size: "5".into(),
                    length: String::new(),
                    all_around: false,
                    field: false,
                    tail: String::new(),
                })
            }
            DwgCmd::Datum => {
                self.dlg = Some(Dlg::Datum {
                    letter: self.free_letter(),
                })
            }
            _ => {}
        }
    }

    fn export(&mut self, pdf: bool) {
        let Some(d) = &self.made else { return };
        let stem = self.path.strip_suffix(".fg").unwrap_or(&self.path).to_string();
        let res = if pdf {
            let out = format!("{stem}.pdf");
            std::fs::write(&out, forge_draw::pdf::pdf(&d.pages)).map(|_| out)
        } else {
            let mut last = Ok(String::new());
            for (k, p) in d.pages.iter().enumerate() {
                let out = if d.pages.len() > 1 {
                    format!("{stem}-{}.dxf", k + 1)
                } else {
                    format!("{stem}.dxf")
                };
                last = std::fs::write(&out, forge_draw::dxf::dxf(p)).map(|_| out);
            }
            last
        };
        self.message = match res {
            Ok(out) => format!("Wrote {}", name_of(&out)),
            Err(e) => format!("Export failed: {e}"),
        };
    }

    fn cancel(&mut self) {
        self.tool = None;
        self.picks.clear();
        self.pts.clear();
        self.dlg = None;
        self.kept = None;
    }

    fn step(&self) -> usize {
        self.picks.len() + self.pts.len()
    }

    /// A click on the sheet at `q` while a tool is on.
    fn click(&mut self, q: Point2) {
        use DwgCmd::*;
        let Some(tool) = self.tool else { return };
        let view = self.view_at(q);
        let edge = self.pick_edge(q, 2.5 / self.zoom.max(0.1) as f64 * 3.0);
        let item = self.item_at(q);
        let sheet = self.sheet_id();
        match tool {
            BaseView => {
                let Some(model) = self.model_id() else {
                    self.message = "The drawing has no model: File > New Drawing from a part".into();
                    return;
                };
                let (dir, scale) = (DIRS[self.base_dir].1, SCALES[self.base_scale].1);
                let (hidden, tangent, label) = match &self.dlg {
                    Some(Dlg::Base {
                        hidden,
                        tangent,
                        label,
                        ..
                    }) => (*hidden, *tangent, label.clone()),
                    _ => (true, false, String::new()),
                };
                let mut call = format!(
                    "base_view({model}, sheet: {sheet}, dir: \"{dir}\", scale: {}, at: {}",
                    num(scale),
                    pt(q)
                );
                if !hidden {
                    call.push_str(", hidden: false");
                }
                if tangent {
                    call.push_str(", tangent: true");
                }
                if !label.is_empty() {
                    call.push_str(&format!(", label: \"{label}\""));
                }
                call.push(')');
                self.add("view", call);
                self.cancel();
            }
            Projected | AutoBalloon | CenterMarks | PartsList if self.picks.is_empty() => {
                let Some(v) = view else { return };
                match tool {
                    AutoBalloon => {
                        self.add("balloons", format!("auto_balloons({v})"));
                        self.cancel();
                    }
                    CenterMarks => {
                        self.add("marks", format!("center_marks({v})"));
                        self.add("axes", format!("centerlines({v})"));
                        self.cancel();
                    }
                    _ => self.picks.push(Pick {
                        view: v.clone(),
                        text: v,
                        kind: "view",
                        at: q,
                        occ: String::new(),
                        geom: None,
                    }),
                }
            }
            Projected => {
                let parent = self.picks[0].view.clone();
                self.add("view", format!("projected({parent}, at: {})", pt(q)));
                // more from the same parent until Esc, as Inventor does
            }
            PartsList => {
                let v = self.picks[0].view.clone();
                self.add("parts", format!("parts_list({v}, sheet: {sheet}, at: {})", pt(q)));
                self.cancel();
            }
            Section | Detail | Auxiliary | Crop | Break if self.picks.is_empty() => {
                if let Some(v) = view {
                    self.picks.push(Pick {
                        view: v.clone(),
                        text: v,
                        kind: "view",
                        at: q,
                        occ: String::new(),
                        geom: None,
                    });
                }
            }
            Section => {
                let parent = self.picks[0].view.clone();
                if self.pts.len() < 2 {
                    self.pts.push(self.in_view(&parent, q));
                } else {
                    let (a, b) = (self.pts[0], self.pts[1]);
                    self.add(
                        "section",
                        format!("section({parent}, [{}, {}], at: {})", pt(a), pt(b), pt(q)),
                    );
                    self.cancel();
                }
            }
            Detail => {
                let parent = self.picks[0].view.clone();
                match self.pts.len() {
                    0 => self.pts.push(self.in_view(&parent, q)),
                    1 => self.pts.push(self.in_view(&parent, q)),
                    _ => {
                        let r = self.pts[0].distance(self.pts[1]);
                        self.add(
                            "detail",
                            format!(
                                "detail({parent}, {}, r: {}, scale: 2, at: {})",
                                pt(self.pts[0]),
                                num(r),
                                pt(q)
                            ),
                        );
                        self.cancel();
                    }
                }
            }
            Auxiliary => {
                let parent = self.picks[0].view.clone();
                if self.picks.len() == 1 {
                    if let Some(e) = edge.filter(|e| e.kind == "line" && e.view == parent) {
                        self.picks.push(e);
                    }
                    return;
                }
                let Some((pv, edge)) = self
                    .made
                    .as_ref()
                    .and_then(|d| d.view(&parent))
                    .zip(self.picks[1].geom.clone())
                else {
                    return;
                };
                let (Geom2::Line { a, b }, Some(model)) = (edge, self.model_id()) else {
                    return;
                };
                let along = (b - a).normalize().unwrap_or(Vec2::X);
                let mut n = along.perp();
                if n.dot(q - a) < 0.0 {
                    n = n * -1.0;
                }
                let toward = pv.view.unproject_vec(n);
                let up = pv.view.toward.get();
                let up = if (up - toward * up.dot(toward)).length() < 1e-9 {
                    pv.view.unproject_vec(along)
                } else {
                    up
                };
                let letter = self.free_view_letter();
                self.add(
                    "aux",
                    format!(
                        "base_view({model}, sheet: {sheet}, dir: ({}, {}, {}), up: ({}, {}, {}), scale: {}, at: {}, label: \"VIEW {letter}\")",
                        num(toward.x),
                        num(toward.y),
                        num(toward.z),
                        num(up.x),
                        num(up.y),
                        num(up.z),
                        num(pv.scale),
                        pt(q)
                    ),
                );
                self.cancel();
            }
            Crop | Break => {
                let v = self.picks[0].view.clone();
                let p = self.in_view(&v, q);
                if self.pts.is_empty() {
                    self.pts.push(p);
                    return;
                }
                let a = self.pts[0];
                let op = if tool == Crop {
                    DrawOp::SetArg {
                        item: v,
                        arg: "crop".into(),
                        value: format!(
                            "[{}, {}]",
                            pt(Point2::new(a.x.min(p.x), a.y.min(p.y))),
                            pt(Point2::new(a.x.max(p.x), a.y.max(p.y)))
                        ),
                    }
                } else {
                    let horizontal = (p.x - a.x).abs() >= (p.y - a.y).abs();
                    let (lo, hi) = if horizontal {
                        (a.x.min(p.x), a.x.max(p.x))
                    } else {
                        (a.y.min(p.y), a.y.max(p.y))
                    };
                    DrawOp::SetArg {
                        item: v,
                        arg: "break".into(),
                        value: format!(
                            "(\"{}\", {}, {})",
                            if horizontal { "x" } else { "y" },
                            num(lo),
                            num(hi)
                        ),
                    }
                };
                self.apply(vec![op]);
                self.cancel();
            }
            Dimension => {
                if self.picks.len() < 2
                    && let Some(e) = edge.clone()
                    && (self.picks.is_empty()
                        || (self.picks[0].view == e.view && self.picks[0].text != e.text))
                {
                    self.picks.push(e);
                    return;
                }
                let Some(first) = self.picks.first().cloned() else {
                    return;
                };
                let at = self.in_view(&first.view, q);
                let call = match (self.picks.len(), first.kind) {
                    (1, "circle") => format!("diameter({}, {}, at: {})", first.view, first.text, pt(at)),
                    (1, _) => format!("dim({}, {}, at: {})", first.view, first.text, pt(at)),
                    _ => format!(
                        "dim({}, {}, {}, at: {})",
                        first.view,
                        first.text,
                        self.picks[1].text,
                        pt(at)
                    ),
                };
                self.add("d", call);
                self.picks.clear();
            }
            Baseline | Ordinate | Chain => {
                if let Some(e) = edge.clone()
                    && (self.picks.is_empty() || self.picks[0].view == e.view)
                    && self.pts.is_empty()
                {
                    if !self.picks.iter().any(|p| p.text == e.text) {
                        self.picks.push(e);
                    }
                    return;
                }
                self.finish_set(q);
            }
            HoleNote | CenterMark | Balloon | LeaderText | DatumTarget | Edge if self.picks.is_empty() => {
                if let Some(e) = edge.filter(|e| !matches!(tool, HoleNote | CenterMark) || e.kind == "circle")
                {
                    if tool == CenterMark {
                        self.add("mark", format!("center_mark({}, {})", e.view, e.text));
                        return;
                    }
                    self.picks.push(e);
                }
            }
            HoleNote | Balloon | DatumTarget | Edge => {
                let e = self.picks[0].clone();
                let at = pt(self.in_view(&e.view, q));
                let call = match tool {
                    HoleNote => format!("hole_note({}, {}, at: {at})", e.view, e.text),
                    Balloon => {
                        if e.occ.is_empty() {
                            self.message = "Balloons number an assembly's parts".into();
                            self.cancel();
                            return;
                        }
                        format!("balloon({}, \"{}\", at: {at})", e.view, e.occ)
                    }
                    DatumTarget => format!(
                        "datum_target({}, {}, \"{}1\", at: {at})",
                        e.view,
                        e.text,
                        self.free_letter()
                    ),
                    _ => format!(
                        "edge({}, {}, \"+0.3\", lower: \"-0.1\", at: {at})",
                        e.view, e.text
                    ),
                };
                let prefix = match tool {
                    HoleNote => "hole",
                    Balloon => "balloon",
                    DatumTarget => "target",
                    _ => "edge",
                };
                self.add(prefix, call);
                self.picks.clear();
            }
            LeaderText => {
                let e = self.picks[0].clone();
                self.dlg = Some(Dlg::Text {
                    text: String::new(),
                    then: TextFor::Leader(e, q),
                });
            }
            Centerline | Bisector => {
                if let Some(e) = edge.filter(|e| {
                    if tool == Centerline {
                        e.kind == "circle"
                    } else {
                        e.kind == "line"
                    }
                }) {
                    self.picks.push(e);
                }
                if self.picks.len() == 2 {
                    let (a, b) = (self.picks[0].clone(), self.picks[1].clone());
                    self.add("axis", format!("centerline({}, {}, {})", a.view, a.text, b.text));
                    self.cancel();
                }
            }
            Text => {
                self.dlg = Some(Dlg::Text {
                    text: String::new(),
                    then: TextFor::Note(q),
                });
            }
            Datum | Fcf | Surface | Weld => {
                if self.picks.is_empty() {
                    // a datum or frame on a dimension hangs from it: no place to click
                    if let Some(id) = item.filter(|_| matches!(tool, Datum | Fcf)) {
                        let v = self.item_view(&id).unwrap_or_default();
                        self.place_gdt(tool, &v, &id, None);
                        return;
                    }
                    if let Some(e) = edge {
                        self.picks.push(e);
                    }
                    return;
                }
                let e = self.picks[0].clone();
                let at = self.in_view(&e.view, q);
                self.place_gdt(tool, &e.view, &e.text, Some(at));
            }
            BreakOut => {}
            _ => {}
        }
    }

    fn item_view(&self, id: &str) -> Option<String> {
        self.made
            .as_ref()?
            .items
            .iter()
            .find(|i| i.id == id)?
            .view
            .clone()
    }

    fn free_view_letter(&self) -> String {
        let used = self.made.as_ref().map_or(0, |d| {
            d.views
                .iter()
                .filter(|v| matches!(v.kind, "section" | "detail"))
                .count()
        });
        ('A'..='Z')
            .filter(|c| !"IOQ".contains(*c))
            .nth(used + 4)
            .map_or("X".into(), |c| c.to_string())
    }

    fn finish_set(&mut self, q: Point2) {
        let Some(tool) = self.tool else { return };
        if self.picks.len() < 2 {
            return;
        }
        let base = self.picks[0].clone();
        let at = self.in_view(&base.view, q);
        let refs: Vec<String> = self.picks[1..].iter().map(|p| p.text.clone()).collect();
        let (xs, ys): (Vec<f64>, Vec<f64>) = self.picks.iter().map(|p| (p.at.x, p.at.y)).unzip();
        let spread = |v: &[f64]| {
            v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                - v.iter().cloned().fold(f64::INFINITY, f64::min)
        };
        let dir = if spread(&xs) >= spread(&ys) {
            "horizontal"
        } else {
            "vertical"
        };
        let func = match tool {
            DwgCmd::Baseline => "baseline",
            DwgCmd::Chain => "chain",
            _ => "ordinate",
        };
        self.add(
            func,
            format!(
                "{func}({}, {}, [{}], dir: \"{dir}\", at: {})",
                base.view,
                base.text,
                refs.join(", "),
                pt(at)
            ),
        );
        self.picks.clear();
        self.pts.clear();
    }

    fn place_gdt(&mut self, tool: DwgCmd, view: &str, to: &str, at: Option<Point2>) {
        let at_arg = at.map_or(String::new(), |p| format!(", at: {}", pt(p)));
        match (tool, self.kept.clone().or(self.dlg.clone())) {
            (DwgCmd::Datum, Some(Dlg::Datum { letter })) => {
                self.add("datum", format!("datum({view}, {to}, \"{letter}\"{at_arg})"));
                self.kept = Some(Dlg::Datum {
                    letter: self.free_letter(),
                });
            }
            (DwgCmd::Fcf, _) => {
                let f = self.fcf.clone();
                let ch = forge_draw::gdt::CHARS[f.ch].0;
                let mut cell = vec![];
                if f.dia {
                    cell.push("dia".to_string());
                }
                cell.push(f.tol.trim().to_string());
                for (k, on) in f.mods.iter().enumerate() {
                    if *on {
                        cell.push(MODS[k].to_string());
                    }
                }
                let datums: Vec<String> = f
                    .datums
                    .iter()
                    .filter(|d| !d.trim().is_empty())
                    .map(|d| format!("\"{}\"", d.trim()))
                    .collect();
                let mut call = format!(
                    "fcf({view}, {to}, \"{ch}\", \"{}\", [{}]",
                    cell.join(" "),
                    datums.join(", ")
                );
                if f.all_around {
                    call.push_str(", all_around: true");
                }
                call.push_str(&at_arg);
                call.push(')');
                self.add("fcf", call);
            }
            (DwgCmd::Surface, Some(Dlg::Surface { ra, process, lay })) => {
                let process = ["any", "required", "forbidden"][process];
                let lay = if lay.is_empty() {
                    String::new()
                } else {
                    format!(", lay: \"{lay}\"")
                };
                self.add(
                    "finish",
                    format!(
                        "surface({view}, {to}, ra: {}, process: \"{process}\"{lay}{at_arg})",
                        ra.trim()
                    ),
                );
            }
            (
                DwgCmd::Weld,
                Some(Dlg::Weld {
                    kind,
                    side,
                    size,
                    length,
                    all_around,
                    field,
                    tail,
                }),
            ) => {
                let kind = ["fillet", "square", "v", "bevel", "plug"][kind];
                let side = ["arrow", "other", "both"][side];
                let mut call = format!("weld({view}, {to}, \"{kind}\", side: \"{side}\", size: \"{size}\"");
                if !length.is_empty() {
                    call.push_str(&format!(", length: \"{length}\""));
                }
                if all_around {
                    call.push_str(", all_around: true");
                }
                if field {
                    call.push_str(", field: true");
                }
                if !tail.is_empty() {
                    call.push_str(&format!(", tail: \"{tail}\""));
                }
                call.push_str(&at_arg);
                call.push(')');
                self.add("weld", call);
            }
            _ => {}
        }
        self.picks.clear();
    }

    /// The dialog to edit an item, by its kind.
    fn edit(&mut self, id: &str) {
        let d = doc::read(&self.src);
        let Some(item) = d.item(id) else { return };
        match &item.kind {
            ItemKind::View(v) => {
                self.dlg = Some(Dlg::EditView {
                    id: id.into(),
                    scale: v.scale.map_or(String::new(), num),
                    hidden: v.hidden.unwrap_or(true),
                    label: v.label.clone().unwrap_or_default(),
                })
            }
            ItemKind::Note { text, .. } => {
                self.dlg = Some(Dlg::Text {
                    text: text.clone(),
                    then: TextFor::Edit(id.into()),
                })
            }
            ItemKind::Annot(a) => match &a.kind {
                AnnotKind::Linear { .. }
                | AnnotKind::Radius { .. }
                | AnnotKind::Diameter { .. }
                | AnnotKind::Angle { .. } => {
                    let (tol, x, y) = match &a.tol {
                        None => (0, String::new(), String::new()),
                        Some(doc::TolSpec::Symmetric(t)) => (1, num(*t), String::new()),
                        Some(doc::TolSpec::Deviation(u, l)) => (2, num(*u), num(*l)),
                        Some(doc::TolSpec::Limits(u, l)) => (3, num(*u), num(*l)),
                        Some(doc::TolSpec::Basic) => (4, String::new(), String::new()),
                        Some(doc::TolSpec::Reference) => (5, String::new(), String::new()),
                        Some(doc::TolSpec::Fit(c)) => (6, c.clone(), String::new()),
                        Some(_) => (0, String::new(), String::new()),
                    };
                    self.dlg = Some(Dlg::EditDim {
                        id: id.into(),
                        text: a.text.clone().unwrap_or_default(),
                        tol,
                        a: x,
                        b: y,
                        places: a.places.map_or(String::new(), |p| p.to_string()),
                    });
                }
                AnnotKind::Leader { .. } => {
                    self.dlg = Some(Dlg::Text {
                        text: a.text.clone().unwrap_or_default(),
                        then: TextFor::Edit(id.into()),
                    })
                }
                AnnotKind::Datum { letter, .. } => {
                    self.dlg = Some(Dlg::Datum {
                        letter: letter.clone(),
                    })
                }
                _ => self.message = format!("{id}: edit it in the file, or delete and place it again"),
            },
            _ => {}
        }
    }

    /// The sheet's strokes as points, made again when the sheet is.
    fn cache(&mut self) {
        let key = (self.made_n, self.sheet);
        if self.lines_for == Some(key) {
            return;
        }
        self.lines.clear();
        if let Some(page) = self.made.as_ref().and_then(|d| d.pages.get(self.sheet)) {
            for s in &page.strokes {
                self.lines.push((s.pen, s.geom.points(0.05)));
            }
        }
        self.lines_for = Some(key);
    }
}

fn pen_colour(pen: Pen) -> Color32 {
    match pen {
        Pen::Sick => Color32::from_rgb(0xD9, 0x00, 0xBF),
        Pen::Hatch => Color32::from_rgb(0x50, 0x50, 0x50),
        Pen::Thin | Pen::Center | Pen::Phantom => Color32::from_rgb(0x20, 0x20, 0x20),
        _ => Color32::BLACK,
    }
}

/// A ribbon group: its name, its big buttons and its small ones, each a command and its icon.
pub type RibbonGroup = (
    &'static str,
    Vec<(DwgCmd, &'static str)>,
    Vec<(DwgCmd, &'static str)>,
);

const SELECT: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xD9);
const PRESELECT: Color32 = Color32::from_rgb(0xFF, 0x9A, 0x1F);

impl Shell {
    /// The Drawing environment's ribbon groups: Inventor's Place Views and Annotate.
    pub(super) fn drawing_groups(&self) -> Vec<RibbonGroup> {
        use DwgCmd::*;
        match self.tab {
            0 => vec![
                (
                    "Create",
                    vec![
                        (BaseView, "dwgbase"),
                        (Projected, "dwgproj"),
                        (Auxiliary, "dwgaux"),
                        (Section, "dwgsect"),
                        (Detail, "dwgdetail"),
                    ],
                    vec![],
                ),
                (
                    "Modify",
                    vec![(Break, "dwgbreak"), (Crop, "dwgcrop")],
                    vec![(BreakOut, "dwgbreakout")],
                ),
                ("Sheets", vec![(NewSheet, "sheet")], vec![]),
            ],
            1 => vec![
                (
                    "Dimension",
                    vec![(Dimension, "dim")],
                    vec![(Baseline, "baseline"), (Ordinate, "ordinate"), (Chain, "chain")],
                ),
                ("Feature Notes", vec![(HoleNote, "hole")], vec![]),
                (
                    "Symbols",
                    vec![(Fcf, "fcf"), (Datum, "datum")],
                    vec![
                        (Surface, "surface"),
                        (Weld, "weld"),
                        (DatumTarget, "dtarget"),
                        (Edge, "edgesym"),
                    ],
                ),
                ("Text", vec![(Text, "text"), (LeaderText, "leadertext")], vec![]),
                (
                    "Center Marks",
                    vec![(CenterMarks, "centermark")],
                    vec![
                        (CenterMark, "centermark"),
                        (Centerline, "centerline"),
                        (Bisector, "bisector"),
                    ],
                ),
                (
                    "Table",
                    vec![(PartsList, "partslist"), (Balloon, "balloon")],
                    vec![(AutoBalloon, "autoballoon")],
                ),
            ],
            2 => vec![("Sketch", vec![(Text, "text"), (LeaderText, "leadertext")], vec![])],
            _ => vec![
                (
                    "Export",
                    vec![(ExportPdf, "export"), (ExportDxf, "export")],
                    vec![],
                ),
                ("Update", vec![(Update, "check"), (Fit, "fit")], vec![]),
            ],
        }
    }

    /// The whole window in the Drawing environment.
    pub(super) fn drawing_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let s = ctx.content_rect();
        let top = super::TITLE_H + super::TABS_H + super::RIBBON_H;
        let bottom = s.height() - super::DOCS_H - super::STATUS_H;
        let side_w = self.side_px.unwrap_or(super::SIDE_W + 30.0);
        let side = Rect::from_min_max(pos2(0.0, top), pos2(side_w, bottom));
        let canvas = Rect::from_min_max(pos2(side.right() + 1.0, top), pos2(s.width(), bottom));
        self.view_rect = canvas;
        if let Some(env) = &mut self.drawing
            && env.poll()
        {
            ctx.request_repaint();
        }
        if self.drawing.as_ref().is_some_and(|d| d.busy) {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        self.title_bar(ui, s.width());
        self.escape_one_level(&ctx);
        self.tabs_row(ui, s.width());
        self.ribbon(ui, s.width());
        // the ribbon's commands are the environment's own
        let mine: Vec<DwgCmd> = self
            .out
            .iter()
            .filter_map(|a| match a {
                Action::Dwg(c) => Some(*c),
                _ => None,
            })
            .collect();
        self.out.retain(|a| !matches!(a, Action::Dwg(_)));
        if let Some(env) = &mut self.drawing {
            for c in mine {
                env.start(c);
            }
        }
        self.drawing_canvas(ui, canvas);
        self.drawing_browser(ui, side);
        self.side_splitter(ui, side, s.width());
        self.docs_and_status(ui, s, bottom);
        self.drawing_dialogs(&ctx);
        self.drawing_mode_bar(&ctx, canvas);
        self.resize_edges(ui, s);
        self.menus(&ctx);
        if let Some(env) = &self.drawing {
            self.status = match (&env.tool, env.busy) {
                _ if !env.message.is_empty() => env.message.clone(),
                (Some(tool), _) => tool.prompt(env.step()).to_string(),
                (None, true) => match env.progress {
                    Some((done, total)) if total > 0 && done > 0 => {
                        let spent = env.started.elapsed().as_secs_f64();
                        let left = spent / done as f64 * (total - done) as f64;
                        format!("Making the drawing: {done} of {total} views, about {left:.0} s left")
                    }
                    Some((_, total)) if total > 0 => format!("Making the drawing: 0 of {total} views"),
                    _ => "Updating the drawing".into(),
                },
                (None, false) => {
                    let n = env.made.as_ref().map_or(0, |d| d.errors.len());
                    if n > 1 {
                        format!("{n} problems: see the browser")
                    } else if n == 1 {
                        "1 problem: see the browser".into()
                    } else {
                        "Ready".into()
                    }
                }
            };
        }
    }

    fn drawing_canvas(&mut self, ui: &mut Ui, canvas: Rect) {
        let t = self.theme;
        let Some(env) = &mut self.drawing else { return };
        let p = ui.painter_at(canvas);
        p.rect_filled(
            canvas,
            0.0,
            if t.dark {
                Color32::from_rgb(0x3A, 0x40, 0x48)
            } else {
                Color32::from_rgb(0xB8, 0xBE, 0xC6)
            },
        );
        if !env.fitted && env.made.is_some() {
            env.fit(canvas);
        }
        let resp = ui.interact(canvas, ui.scope_id().with("dwg-canvas"), Sense::click_and_drag());
        // zoom about the pointer, pan with the middle button
        if let Some(hp) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let k = (scroll * 0.002).exp();
                env.origin = hp + (env.origin - hp) * k;
                env.zoom *= k;
            }
            env.pointer = Some(env.to_sheet(hp));
        } else {
            env.pointer = None;
        }
        if ui.input(|i| i.pointer.middle_down()) && resp.hovered() {
            env.origin += ui.input(|i| i.pointer.delta());
        }
        env.cache();
        let z = env.zoom;
        let q = env.pointer;
        let hovering_item = q.and_then(|q| env.item_at(q));
        env.hover = match (env.tool, q) {
            (Some(_), Some(q)) => env.pick_edge(q, 8.0 / z as f64),
            _ => None,
        };
        let Some(made) = &env.made else {
            p.text(
                canvas.center(),
                Align2::CENTER_CENTER,
                if env.busy { "Making the drawing..." } else { "" },
                sans(16.0),
                t.ink,
            );
            return;
        };
        let Some(page) = made.pages.get(env.sheet) else {
            return;
        };
        // the paper, a shadow under it
        let paper = Rect::from_min_max(
            env.to_screen(Point2::new(0.0, page.h)),
            env.to_screen(Point2::new(page.w, 0.0)),
        );
        p.rect_filled(
            paper.translate(vec2(3.0, 3.0)),
            0.0,
            Color32::from_black_alpha(60),
        );
        p.rect_filled(paper, 0.0, Color32::WHITE);
        let drag_by = env.drag.as_ref().map(|d| (d.id.clone(), d.by));
        for (pen, pts) in &env.lines {
            let pts: Vec<Pos2> = pts.iter().map(|q| env.to_screen(*q)).collect();
            let w = (pen.width() as f32 * z * 0.9).clamp(0.6, 4.0);
            let stroke = Stroke::new(w, pen_colour(*pen));
            let dash = pen.dashes();
            if dash.is_empty() {
                p.add(Shape::line(pts, stroke));
            } else {
                let (on, off) = (dash[0] as f32 * z, dash[1] as f32 * z);
                p.extend(Shape::dashed_line(&pts, stroke, on.max(2.0), off.max(1.5)));
            }
        }
        for (pen, poly) in &page.fills {
            let pts: Vec<Pos2> = poly.iter().map(|q| env.to_screen(*q)).collect();
            p.add(Shape::convex_polygon(pts, pen_colour(*pen), Stroke::NONE));
        }
        for txt in &page.texts {
            let size = (txt.h / 0.718) as f32 * z;
            if size < 2.0 {
                continue;
            }
            // the app's type has no ⌀: Ø stands for it, as Inventor's does
            let shown = txt.text.replace('\u{2300}', "\u{d8}");
            let g = ui.fonts_mut(|f| f.layout_no_wrap(shown, sans(size), pen_colour(txt.pen)));
            let w = g.size().x;
            let ang = -(txt.angle as f32).to_radians();
            let dir = vec2(ang.cos(), ang.sin());
            let up = vec2(dir.y, -dir.x);
            let base = env.to_screen(txt.at);
            let shift = match txt.align {
                Align::Left => 0.0,
                Align::Centre => -0.5 * w,
                Align::Right => -w,
            };
            // egui sets type from its top: the baseline's a cap height and a bit below it
            let topleft = base + dir * shift + up * (size * 0.93);
            p.add(Shape::Text(
                egui::epaint::TextShape::new(topleft, g, pen_colour(txt.pen)).with_angle(ang),
            ));
        }
        let _ = text_width;
        if let Some(h) = &env.hover
            && let Some(g) = &h.geom
        {
            let pts: Vec<Pos2> = g.points(0.05).iter().map(|q| env.to_screen(*q)).collect();
            p.add(Shape::line(pts, Stroke::new(3.0, PRESELECT)));
        }
        for pk in &env.picks {
            if let Some(g) = &pk.geom {
                let pts: Vec<Pos2> = g.points(0.05).iter().map(|q| env.to_screen(*q)).collect();
                p.add(Shape::line(pts, Stroke::new(3.0, SELECT)));
            } else if let Some(v) = made.view(&pk.view) {
                p.rect_stroke(
                    Rect::from_min_max(
                        env.to_screen(Point2::new(v.lo.x, v.hi.y)),
                        env.to_screen(Point2::new(v.hi.x, v.lo.y)),
                    )
                    .expand(4.0),
                    2.0,
                    Stroke::new(2.0, SELECT),
                    StrokeKind::Outside,
                );
            }
        }
        // the tool's rubber band: a section line, a detail's fence, a crop's box
        if let (Some(tool), Some(q)) = (env.tool, q)
            && let Some(first) = env.picks.first()
            && let Some(v) = made.view(&first.view)
        {
            let sp = |m: Point2| env.to_screen(v.place.point(m));
            let k = Stroke::new(1.5, SELECT);
            match (tool, env.pts.len()) {
                (DwgCmd::Section, 1) => {
                    p.line_segment([sp(env.pts[0]), env.to_screen(q)], k);
                }
                (DwgCmd::Section, 2) => {
                    p.line_segment([sp(env.pts[0]), sp(env.pts[1])], k);
                }
                (DwgCmd::Detail, 1) => {
                    let c = sp(env.pts[0]);
                    p.circle_stroke(c, c.distance(env.to_screen(q)), k);
                }
                (DwgCmd::Detail, 2) => {
                    let c = sp(env.pts[0]);
                    p.circle_stroke(c, c.distance(sp(env.pts[1])), k);
                }
                (DwgCmd::Crop, 1) => {
                    p.rect_stroke(
                        Rect::from_two_pos(sp(env.pts[0]), env.to_screen(q)),
                        0.0,
                        k,
                        StrokeKind::Middle,
                    );
                }
                (DwgCmd::Break, 1) => {
                    let a = sp(env.pts[0]);
                    p.line_segment([a + vec2(0.0, -30.0), a + vec2(0.0, 30.0)], k);
                    let b = env.to_screen(q);
                    p.line_segment([b + vec2(0.0, -30.0), b + vec2(0.0, 30.0)], k);
                }
                _ => {}
            }
            if matches!(tool, DwgCmd::Projected)
                || (matches!(tool, DwgCmd::Section | DwgCmd::Detail | DwgCmd::Auxiliary) && env.step() >= 3)
            {
                let c = env.to_screen(q);
                p.rect_stroke(
                    Rect::from_center_size(c, vec2(60.0, 40.0)),
                    2.0,
                    Stroke::new(1.0, SELECT),
                    StrokeKind::Middle,
                );
            }
        }
        // views and annotations: hover and selection boxes, places for scripts
        for v in made.views.iter().filter(|v| v.sheet == env.sheet) {
            let r = Rect::from_min_max(
                env.to_screen(Point2::new(v.lo.x, v.hi.y)),
                env.to_screen(Point2::new(v.hi.x, v.lo.y)),
            )
            .expand(6.0);
            self.places.push((format!("dwgview:{}", v.id), r));
            let sel = env.selected.as_deref() == Some(v.id.as_str());
            let hov = env.tool.is_none()
                && q.is_some_and(|q| env.view_at(q).as_deref() == Some(v.id.as_str()))
                && hovering_item.is_none();
            if sel || hov {
                let r = drag_by
                    .as_ref()
                    .filter(|d| d.0 == v.id)
                    .map_or(r, |d| r.translate(vec2(d.1.x as f32, -d.1.y as f32) * z));
                p.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(if sel { 1.5 } else { 1.0 }, if sel { SELECT } else { PRESELECT }),
                    StrokeKind::Middle,
                );
            }
        }
        for i in made.items.iter().filter(|i| {
            i.sheet == env.sheet && !matches!(i.kind, "base" | "projected" | "section" | "detail" | "flat")
        }) {
            let r = Rect::from_min_max(
                env.to_screen(Point2::new(i.lo.x, i.hi.y)),
                env.to_screen(Point2::new(i.hi.x, i.lo.y)),
            )
            .expand(3.0);
            self.places.push((format!("dwgitem:{}", i.id), r));
            let sel = env.selected.as_deref() == Some(i.id.as_str());
            if sel || (env.tool.is_none() && hovering_item.as_deref() == Some(i.id.as_str())) {
                let r = drag_by
                    .as_ref()
                    .filter(|d| d.0 == i.id)
                    .map_or(r, |d| r.translate(vec2(d.1.x as f32, -d.1.y as f32) * z));
                p.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.5, if sel { SELECT } else { PRESELECT }),
                    StrokeKind::Middle,
                );
            }
            if i.sick {
                p.circle_filled(r.right_top(), 5.0, Color32::from_rgb(0xD9, 0x00, 0xBF));
            }
        }
        // the sheet tabs
        let names = made.sheets.clone();
        let mut x = canvas.left() + 8.0;
        for (k, n) in names.iter().enumerate() {
            let w = 14.0 + n.len() as f32 * 8.0;
            let r = Rect::from_min_size(pos2(x, canvas.bottom() - 26.0), vec2(w, 22.0));
            let on = k == env.sheet;
            p.rect_filled(r, 3.0, if on { t.raised } else { t.surface });
            p.rect_stroke(r, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, n, sans(12.0), t.ink);
            self.places.push((format!("dwgsheet:{n}"), r));
            if ui
                .interact(r, ui.scope_id().with(("sheettab", k)), Sense::click())
                .clicked()
            {
                env.sheet = k;
                env.fitted = false;
            }
            x += w + 4.0;
        }
        // clicks, drags, keys
        let keys = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Delete),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::F),
            )
        });
        let typing = ui.ctx().egui_wants_keyboard_input();
        if keys.0 {
            env.cancel();
        }
        if keys.2
            && let Some(q) = q
        {
            env.finish_set(q);
        }
        if keys.3 && !typing && resp.hovered() {
            env.fitted = false;
        }
        if keys.1
            && !typing
            && env.tool.is_none()
            && let Some(id) = env.selected.clone()
        {
            env.apply(vec![DrawOp::DeleteItem { item: id }]);
            env.selected = None;
        }
        if resp.clicked()
            && let Some(q) = resp.interact_pointer_pos().map(|p| env.to_sheet(p))
        {
            if env.tool.is_some() {
                env.click(q);
            } else {
                env.selected = env.item_at(q).or_else(|| env.view_at(q));
            }
        }
        if resp.double_clicked()
            && env.tool.is_none()
            && let Some(id) = env.selected.clone()
        {
            env.edit(&id);
        }
        if resp.secondary_clicked() {
            if matches!(
                env.tool,
                Some(DwgCmd::Baseline | DwgCmd::Ordinate | DwgCmd::Chain)
            ) {
                if let Some(q) = q {
                    env.finish_set(q);
                }
            } else if env.tool.is_some() {
                env.cancel();
            } else if let Some(q) = q {
                env.selected = env.item_at(q).or_else(|| env.view_at(q));
            }
        }
        // the right-click menu: edit, delete, and what the problems with it offer as fixes
        if env.tool.is_none()
            && let Some(id) = env.selected.clone()
        {
            let fixes: Vec<DrawOp> = env
                .made
                .as_ref()
                .map(|m| {
                    m.errors
                        .iter()
                        .filter(|e| problem_item(&e.problem) == Some(id.as_str()))
                        .flat_map(|e| e.fixes.clone())
                        .collect()
                })
                .unwrap_or_default();
            let mut chosen: Option<DrawOp> = None;
            let mut edit = false;
            resp.context_menu(|ui| {
                ui.set_min_width(220.0);
                ui.label(egui::RichText::new(&id).font(semi(12.5)));
                if ui.button("Edit...").clicked() {
                    edit = true;
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    chosen = Some(DrawOp::DeleteItem { item: id.clone() });
                    ui.close();
                }
                for f in &fixes {
                    let label = match f {
                        DrawOp::SetArg { value, .. } => format!("Reattach to {value}"),
                        DrawOp::DeleteItem { .. } => continue,
                        DrawOp::AddItem { text } => format!("Add {text}"),
                    };
                    if ui.button(label).clicked() {
                        chosen = Some(f.clone());
                        ui.close();
                    }
                }
            });
            if edit {
                env.edit(&id);
            }
            if let Some(op) = chosen {
                env.apply(vec![op]);
            }
        }
        if env.tool.is_none()
            && resp.drag_started_by(egui::PointerButton::Primary)
            && let Some(q) = resp.interact_pointer_pos().map(|p| env.to_sheet(p))
            && let Some(id) = env.item_at(q).or_else(|| env.view_at(q))
        {
            env.selected = Some(id.clone());
            env.drag = Some(Drag {
                id,
                from: q,
                by: Vec2::ZERO,
            });
        }
        if let (Some(d), Some(q)) = (&mut env.drag, q) {
            d.by = q - d.from;
        }
        if resp.drag_stopped()
            && let Some(d) = env.drag.take()
            && d.by.length() > 0.3
        {
            moved(env, &d);
        }
    }

    fn drawing_browser(&mut self, ui: &mut Ui, side: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(side, 0.0, t.surface);
        p.vline(side.right(), side.top()..=side.bottom(), Stroke::new(1.0, t.line));
        self.side_tabs(ui, side.top(), side.width(), "");
        if self.agent.tab == super::agent::SideTab::Agent {
            let area = Rect::from_min_max(pos2(side.left(), side.top() + super::HEAD_H), side.max);
            self.agent_ui(ui, area);
            return;
        }
        let Some(env) = &mut self.drawing else { return };
        let Some(made) = &env.made else { return };
        let d = &made.doc;
        // rows: the drawing, its models, each sheet, its views by parentage, each view's annotations
        let mut rows: Vec<(usize, String, String, &'static str, bool)> =
            vec![(0, String::new(), env.name(), "file", false)];
        for m in &made.models {
            rows.push((
                1,
                m.id.clone(),
                name_of(&m.file),
                if m.model.paths.len() > 1 { "subasm" } else { "part" },
                false,
            ));
        }
        let sick: Vec<&str> = made
            .items
            .iter()
            .filter(|i| i.sick)
            .map(|i| i.id.as_str())
            .collect();
        let errored: Vec<String> = made
            .errors
            .iter()
            .filter(|e| !e.warning)
            .filter_map(|e| {
                d.items
                    .iter()
                    .find(|i| i.span.lo <= e.span.lo && e.span.hi <= i.span.hi.max(e.span.hi))
                    .map(|i| i.id.clone())
            })
            .collect();
        for (k, sheet) in made.sheets.iter().enumerate() {
            rows.push((
                1,
                sheet.clone(),
                format!("Sheet:{} ({sheet})", k + 1),
                "sheet",
                false,
            ));
            fn add_view(
                rows: &mut Vec<(usize, String, String, &'static str, bool)>,
                made: &Drawing,
                id: &str,
                depth: usize,
                sick: &[&str],
            ) {
                let Some(v) = made.view(id) else { return };
                let label = match v.kind {
                    "section" => format!("{id}  SECTION {0}-{0}", v.label),
                    "detail" => format!("{id}  DETAIL {}", v.label),
                    k => format!("{id}  {}", k),
                };
                rows.push((depth, id.to_string(), label, "dwgbase", false));
                for i in made
                    .items
                    .iter()
                    .filter(|i| i.view.as_deref() == Some(id) && i.id != id)
                {
                    rows.push((
                        depth + 1,
                        i.id.clone(),
                        format!("{}  {}", i.id, i.kind),
                        "dim",
                        sick.contains(&i.id.as_str()),
                    ));
                }
                for c in made.views.iter().filter(|c| c.parent.as_deref() == Some(id)) {
                    add_view(rows, made, &c.id, depth + 1, sick);
                }
            }
            for v in made.views.iter().filter(|v| v.sheet == k && v.parent.is_none()) {
                add_view(&mut rows, made, &v.id, 2, &sick);
            }
            for i in made.items.iter().filter(|i| i.sheet == k && i.view.is_none()) {
                rows.push((2, i.id.clone(), format!("{}  {}", i.id, i.kind), "text", false));
            }
        }
        let mut y = side.top() + 30.0;
        let mut chosen = None;
        let mut edit = None;
        for (depth, id, label, icon, is_sick) in rows {
            if y > side.bottom() - 20.0 {
                break;
            }
            let r = Rect::from_min_size(pos2(side.left(), y), vec2(side.width() - 1.0, 24.0));
            let resp = ui.interact(r, ui.scope_id().with(("dwgrow", &id, depth)), Sense::click());
            let on = !id.is_empty() && env.selected.as_deref() == Some(id.as_str());
            if on {
                p.rect_filled(r, 0.0, t.selection);
            } else if resp.hovered() {
                p.rect_filled(r, 0.0, t.hover);
            }
            let x = side.left() + 10.0 + depth as f32 * 14.0;
            let bad = is_sick || errored.contains(&id);
            let ink = if bad {
                Color32::from_rgb(0xE0, 0x40, 0xC8)
            } else {
                t.ink
            };
            icons::draw(
                &p,
                pos2(x, y + 4.0),
                16.0,
                if bad { "warning" } else { icon },
                1.3,
                ink,
            );
            p.text(
                pos2(x + 22.0, y + 12.0),
                Align2::LEFT_CENTER,
                &label,
                if depth == 0 { semi(12.5) } else { sans(12.5) },
                ink,
            );
            self.places.push((format!("dwgrow:{id}"), r));
            if resp.clicked() && !id.is_empty() {
                chosen = Some(id.clone());
            }
            if resp.double_clicked() && !id.is_empty() {
                edit = Some(id.clone());
            }
            y += 24.0;
        }
        // what's wrong, under the tree
        let problems: Vec<String> = made
            .errors
            .iter()
            .map(|e| format!("{} {}", if e.warning { "!" } else { "x" }, e.problem.text()))
            .collect();
        if !problems.is_empty() && y < side.bottom() - 40.0 {
            p.hline(side.left()..=side.right(), y + 6.0, Stroke::new(1.0, t.line));
            y += 14.0;
            for pr in problems {
                if y > side.bottom() - 16.0 {
                    break;
                }
                let g = ui.fonts_mut(|f| f.layout(pr, sans(11.5), t.ink2, side.width() - 20.0));
                let h = g.size().y;
                p.galley(pos2(side.left() + 10.0, y), g, t.ink2);
                y += h + 4.0;
            }
        }
        if let Some(id) = chosen {
            env.selected = Some(id);
        }
        if let Some(id) = edit {
            env.edit(&id);
        }
    }

    fn drawing_mode_bar(&mut self, ctx: &egui::Context, canvas: Rect) {
        let t = self.theme;
        let Some(env) = &mut self.drawing else { return };
        let Some(tool) = env.tool else { return };
        if env.dlg.is_some() {
            return;
        }
        let mut done = false;
        let mut cancel = false;
        egui::Area::new(egui::Id::unique("dwg-mode"))
            .fixed_pos(pos2(canvas.center().x - 200.0, canvas.top() + 10.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(t.raised)
                    .stroke(Stroke::new(1.0, t.line))
                    .corner_radius(6.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(tool.name()).font(semi(13.0)).color(t.ink));
                            ui.separator();
                            ui.label(
                                egui::RichText::new(tool.prompt(env.step()))
                                    .font(sans(12.5))
                                    .color(t.ink2),
                            );
                            if matches!(tool, DwgCmd::Baseline | DwgCmd::Ordinate | DwgCmd::Chain)
                                && ui.button("Done").clicked()
                            {
                                done = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel = true;
                            }
                        });
                    });
            });
        if done {
            // Done: the set is placed where the pointer next clicks
            env.pts.push(Point2::ORIGIN);
        }
        if cancel {
            env.cancel();
        }
    }

    fn drawing_dialogs(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some(env) = &mut self.drawing else { return };
        let Some(dlg) = env.dlg.clone() else { return };
        let mut ok_rect = None;
        let mut ok = false;
        let mut cancel = false;
        let mut next = dlg.clone();
        let title = match &dlg {
            Dlg::Base { .. } => "Drawing View",
            Dlg::Text { .. } => "Format Text",
            Dlg::Fcf(_) => "Feature Control Frame",
            Dlg::Surface { .. } => "Surface Texture",
            Dlg::Weld { .. } => "Welding Symbol",
            Dlg::Datum { .. } => "Datum Identifier",
            Dlg::EditDim { .. } => "Edit Dimension",
            Dlg::EditView { .. } => "Edit View",
        };
        egui::Window::new(title)
            .id(egui::Id::unique("dwg-dialog"))
            .collapsible(false)
            .resizable(false)
            .frame(super::fgasm::dialog_frame(&t))
            .default_pos(pos2(420.0, 200.0))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = vec2(8.0, 8.0);
                match &mut next {
                    Dlg::Base {
                        dir,
                        scale,
                        hidden,
                        tangent,
                        label,
                    } => {
                        ui.label(egui::RichText::new("Orientation").font(semi(12.5)));
                        egui::Grid::new("dirs").spacing(vec2(6.0, 6.0)).show(ui, |ui| {
                            for (k, (n, _)) in DIRS.iter().enumerate() {
                                if ui.selectable_label(*dir == k, *n).clicked() {
                                    *dir = k;
                                }
                                if k % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Scale");
                            egui::ComboBox::from_id_salt("scale")
                                .selected_text(SCALES[*scale].0)
                                .show_ui(ui, |ui| {
                                    for (k, (n, _)) in SCALES.iter().enumerate() {
                                        ui.selectable_value(scale, k, *n);
                                    }
                                });
                            ui.label("Label");
                            ui.text_edit_singleline(label);
                        });
                        ui.label(egui::RichText::new("Style").font(semi(12.5)));
                        ui.horizontal(|ui| {
                            ui.selectable_value(hidden, true, "Hidden Line");
                            ui.selectable_value(hidden, false, "Hidden Line Removed");
                        });
                        ui.checkbox(tangent, "Tangent Edges");
                    }
                    Dlg::Text { text, .. } => {
                        ui.add(
                            egui::TextEdit::multiline(text)
                                .desired_rows(4)
                                .desired_width(320.0)
                                .font(mono(13.0)),
                        );
                    }
                    Dlg::Datum { letter } => {
                        ui.horizontal(|ui| {
                            ui.label("Datum letter");
                            ui.add(egui::TextEdit::singleline(letter).desired_width(40.0));
                        });
                        ui.label(
                            egui::RichText::new("Pick an edge, or a dimension for a feature of size")
                                .color(t.ink2),
                        );
                    }
                    Dlg::Fcf(f) => fcf_form(ui, f, t),
                    Dlg::Surface { ra, process, lay } => {
                        ui.horizontal(|ui| {
                            ui.selectable_value(process, 0, "Basic");
                            ui.selectable_value(process, 1, "Material Removal Required");
                            ui.selectable_value(process, 2, "Material Removal Prohibited");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Ra (µm)");
                            ui.add(egui::TextEdit::singleline(ra).desired_width(60.0));
                            ui.label("Lay");
                            egui::ComboBox::from_id_salt("lay")
                                .selected_text(if lay.is_empty() { "none" } else { lay.as_str() })
                                .show_ui(ui, |ui| {
                                    for l in ["", "=", "\u{22a5}", "X", "M", "C", "R", "P"] {
                                        ui.selectable_value(
                                            lay,
                                            l.to_string(),
                                            if l.is_empty() { "none" } else { l },
                                        );
                                    }
                                });
                        });
                    }
                    Dlg::Weld {
                        kind,
                        side,
                        size,
                        length,
                        all_around,
                        field,
                        tail,
                    } => {
                        ui.horizontal(|ui| {
                            for (k, n) in ["Fillet", "Square", "V", "Bevel", "Plug"].iter().enumerate() {
                                ui.selectable_value(kind, k, *n);
                            }
                        });
                        ui.horizontal(|ui| {
                            for (k, n) in ["Arrow Side", "Other Side", "Both Sides"].iter().enumerate() {
                                ui.selectable_value(side, k, *n);
                            }
                        });
                        egui::Grid::new("weld").show(ui, |ui| {
                            ui.label("Size");
                            ui.text_edit_singleline(size);
                            ui.end_row();
                            ui.label("Length-Pitch");
                            ui.text_edit_singleline(length);
                            ui.end_row();
                            ui.label("Tail");
                            ui.text_edit_singleline(tail);
                            ui.end_row();
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(all_around, "All Around");
                            ui.checkbox(field, "Field Weld");
                        });
                    }
                    Dlg::EditDim {
                        text,
                        tol,
                        a,
                        b,
                        places,
                        ..
                    } => {
                        ui.horizontal(|ui| {
                            ui.label("Text override");
                            ui.text_edit_singleline(text);
                        });
                        ui.horizontal_wrapped(|ui| {
                            for (k, n) in TOLS.iter().enumerate() {
                                ui.selectable_value(tol, k, *n);
                            }
                        });
                        match *tol {
                            1 => {
                                ui.horizontal(|ui| {
                                    ui.label("\u{b1}");
                                    ui.text_edit_singleline(a);
                                });
                            }
                            2 | 3 => {
                                ui.horizontal(|ui| {
                                    ui.label("Upper");
                                    ui.add(egui::TextEdit::singleline(a).desired_width(70.0));
                                    ui.label("Lower");
                                    ui.add(egui::TextEdit::singleline(b).desired_width(70.0));
                                });
                            }
                            6 => {
                                ui.horizontal(|ui| {
                                    ui.label("Class (ISO 286)");
                                    egui::ComboBox::from_id_salt("fit")
                                        .selected_text(a.as_str())
                                        .show_ui(ui, |ui| {
                                            for c in [
                                                "H7", "H8", "H9", "H11", "G7", "F8", "K7", "N7", "P7", "h6",
                                                "h7", "g6", "f7", "k6", "m6", "n6", "p6", "s6", "js6",
                                            ] {
                                                ui.selectable_value(a, c.to_string(), c);
                                            }
                                        });
                                });
                            }
                            _ => {}
                        }
                        ui.horizontal(|ui| {
                            ui.label("Precision");
                            ui.add(
                                egui::TextEdit::singleline(places)
                                    .desired_width(40.0)
                                    .hint_text("2"),
                            );
                        });
                    }
                    Dlg::EditView {
                        scale, hidden, label, ..
                    } => {
                        egui::Grid::new("view").show(ui, |ui| {
                            ui.label("Scale");
                            ui.text_edit_singleline(scale);
                            ui.end_row();
                            ui.label("Label");
                            ui.text_edit_singleline(label);
                            ui.end_row();
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(hidden, true, "Hidden Line");
                            ui.selectable_value(hidden, false, "Hidden Line Removed");
                        });
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let r = ui.button(egui::RichText::new("   OK   ").font(semi(13.0)));
                    ok_rect = Some(r.rect);
                    if r.clicked() {
                        ok = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        env.dlg = Some(next.clone());
        if let Some(r) = ok_rect {
            self.places.push(("dwgdlg:OK".into(), r));
        }
        let Some(env) = &mut self.drawing else { return };
        if cancel {
            env.cancel();
            return;
        }
        if !ok {
            return;
        }
        match next {
            Dlg::Base { dir, scale, .. } => {
                env.base_dir = dir;
                env.base_scale = scale;
                env.tool = Some(DwgCmd::BaseView);
                // the dialog's values stay with the tool till the view is placed
            }
            Dlg::Text { text, then } => {
                let text = text.replace('"', "'").replace('\n', "\\n");
                env.dlg = None;
                match then {
                    TextFor::Note(q) => {
                        let sheet = env.sheet_id();
                        env.add("note", format!("note(\"{text}\", sheet: {sheet}, at: {})", pt(q)));
                        env.tool = None;
                    }
                    TextFor::Leader(e, q) => {
                        let at = env.in_view(&e.view, q);
                        env.add(
                            "leader",
                            format!("leader({}, {}, text: \"{text}\", at: {})", e.view, e.text, pt(at)),
                        );
                        env.picks.clear();
                    }
                    TextFor::Edit(id) => {
                        let arg = if matches!(
                            doc::read(&env.src).item(&id).map(|i| &i.kind),
                            Some(ItemKind::Note { .. })
                        ) {
                            "0"
                        } else {
                            "text"
                        };
                        env.apply(vec![DrawOp::SetArg {
                            item: id,
                            arg: arg.into(),
                            value: format!("\"{text}\""),
                        }]);
                    }
                }
            }
            Dlg::Fcf(f) => {
                env.fcf = f;
                env.dlg = None;
                env.tool = Some(DwgCmd::Fcf);
            }
            Dlg::Surface { .. } | Dlg::Weld { .. } | Dlg::Datum { .. } => {
                // the values stay with the tool, which picks and places
                env.kept = env.dlg.take();
            }
            Dlg::EditDim {
                id,
                text,
                tol,
                a,
                b,
                places,
            } => {
                env.dlg = None;
                let mut ops = vec![];
                // the tolerance arguments go, then the one chosen comes back
                let mut src = env.src.clone();
                for arg in ["tol", "limits", "fit", "basic", "reference"] {
                    src = strip_arg(&src, &id, arg);
                }
                env.src = src;
                if !text.is_empty() {
                    ops.push(DrawOp::SetArg {
                        item: id.clone(),
                        arg: "text".into(),
                        value: format!("\"{text}\""),
                    });
                }
                let set = |arg: &str, value: String| DrawOp::SetArg {
                    item: id.clone(),
                    arg: arg.into(),
                    value,
                };
                match tol {
                    1 => ops.push(set("tol", a.trim().to_string())),
                    2 => ops.push(set("tol", format!("({}, {})", a.trim(), b.trim()))),
                    3 => ops.push(set("limits", format!("({}, {})", a.trim(), b.trim()))),
                    4 => ops.push(set("basic", "true".into())),
                    5 => ops.push(set("reference", "true".into())),
                    6 => ops.push(set("fit", format!("\"{}\"", a.trim()))),
                    _ => {}
                }
                if !places.trim().is_empty() {
                    ops.push(set("places", places.trim().to_string()));
                }
                env.apply(ops);
            }
            Dlg::EditView {
                id,
                scale,
                hidden,
                label,
            } => {
                env.dlg = None;
                let mut ops = vec![];
                if !scale.trim().is_empty() {
                    ops.push(DrawOp::SetArg {
                        item: id.clone(),
                        arg: "scale".into(),
                        value: scale.trim().into(),
                    });
                }
                ops.push(DrawOp::SetArg {
                    item: id.clone(),
                    arg: "hidden".into(),
                    value: hidden.to_string(),
                });
                if !label.is_empty() {
                    ops.push(DrawOp::SetArg {
                        item: id,
                        arg: "label".into(),
                        value: format!("\"{label}\""),
                    });
                }
                env.apply(ops);
            }
        }
    }
}

/// The source with one keyword argument of an item's call taken out (a tolerance being changed).
fn strip_arg(src: &str, item: &str, arg: &str) -> String {
    let Ok(prog) = forge_lang::syntax::parse(src) else {
        return src.to_string();
    };
    for s in &prog {
        if let forge_lang::syntax::StmtKind::Assign { name, value } = &s.kind
            && name == item
            && let forge_lang::syntax::ExprKind::Call(_, args) = &value.kind
            && let Some(k) = args
                .iter()
                .position(|a| a.name.as_ref().is_some_and(|n| n.0 == arg))
        {
            let a = &args[k];
            let lo = a.name.as_ref().map_or(a.value.span.lo, |n| n.1.lo) as usize;
            let hi = a.value.span.hi as usize;
            // with the comma before it
            let before = src[..lo]
                .rfind(',')
                .filter(|&c| src[c + 1..lo].trim().is_empty())
                .unwrap_or(lo);
            return format!("{}{}", &src[..before], &src[hi..]);
        }
    }
    src.to_string()
}

/// The item a problem is about, where it names one.
fn problem_item(p: &doc::Problem) -> Option<&str> {
    match p {
        doc::Problem::Sick { item, .. }
        | doc::Problem::NotInView { item, .. }
        | doc::Problem::HiddenOnly { item, .. }
        | doc::Problem::NotHere { item, .. }
        | doc::Problem::Gdt { item, .. } => Some(item),
        _ => None,
    }
}

/// A drag ended: the view or annotation moves there, written back as its `at`.
fn moved(env: &mut DrawingEnv, d: &Drag) {
    let Some(made) = &env.made else { return };
    let doc = doc::read(&env.src);
    let Some(item) = doc.item(&d.id) else { return };
    let op = match &item.kind {
        ItemKind::View(v) => DrawOp::SetArg {
            item: d.id.clone(),
            arg: "at".into(),
            value: pt(v.at + d.by),
        },
        ItemKind::Note { at, .. } => DrawOp::SetArg {
            item: d.id.clone(),
            arg: "at".into(),
            value: pt(*at + d.by),
        },
        ItemKind::PartsList { at, .. } => {
            let base = at.unwrap_or_else(|| {
                made.items
                    .iter()
                    .find(|i| i.id == d.id)
                    .map_or(Point2::ORIGIN, |i| Point2::new(i.hi.x, i.lo.y))
            });
            DrawOp::SetArg {
                item: d.id.clone(),
                arg: "at".into(),
                value: pt(base + d.by),
            }
        }
        ItemKind::Annot(a) => {
            let Some(v) = made.view(&a.view) else { return };
            let from = a.at.unwrap_or_else(|| v.place.inverse_point(d.from));
            DrawOp::SetArg {
                item: d.id.clone(),
                arg: "at".into(),
                value: pt(from + d.by * (1.0 / v.scale)),
            }
        }
        _ => return,
    };
    env.apply(vec![op]);
}

fn fcf_form(ui: &mut Ui, f: &mut FcfForm, t: super::theme::Theme) {
    ui.label(egui::RichText::new("Characteristic").font(semi(12.5)));
    egui::Grid::new("chars").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
        for (k, (word, _)) in forge_draw::gdt::CHARS.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(vec2(36.0, 30.0), Sense::click());
            let on = f.ch == k;
            let p = ui.painter();
            p.rect_filled(
                r,
                3.0,
                if on {
                    t.on
                } else if resp.hovered() {
                    t.hover
                } else {
                    t.field
                },
            );
            p.rect_stroke(
                r,
                3.0,
                Stroke::new(1.0, if on { t.on_ink } else { t.line }),
                StrokeKind::Inside,
            );
            // the glyph, drawn by the same strokes the sheet uses
            let mut page = forge_draw::Page::default();
            forge_draw::gdt::glyph(&mut page, forge_draw::gdt::CHARS[k].1, Point2::ORIGIN, 1.0);
            let ink = if on { t.on_ink } else { t.ink };
            for s in &page.strokes {
                let pts: Vec<Pos2> = s
                    .geom
                    .points(0.02)
                    .iter()
                    .map(|q| r.center() + vec2(q.x as f32 * 11.0, -q.y as f32 * 11.0))
                    .collect();
                p.add(Shape::line(pts, Stroke::new(1.4, ink)));
            }
            for (_, poly) in &page.fills {
                let pts: Vec<Pos2> = poly
                    .iter()
                    .map(|q| r.center() + vec2(q.x as f32 * 11.0, -q.y as f32 * 11.0))
                    .collect();
                p.add(Shape::convex_polygon(pts, ink, Stroke::NONE));
            }
            if resp.on_hover_text(word.replace('_', " ")).clicked() {
                f.ch = k;
            }
            if k % 7 == 6 {
                ui.end_row();
            }
        }
    });
    ui.label(egui::RichText::new("Tolerance").font(semi(12.5)));
    ui.horizontal(|ui| {
        ui.toggle_value(&mut f.dia, "\u{d8}");
        ui.add(egui::TextEdit::singleline(&mut f.tol).desired_width(70.0));
        for (k, m) in MODS.iter().enumerate() {
            ui.toggle_value(&mut f.mods[k], *m);
        }
    });
    ui.label(egui::RichText::new("Datums").font(semi(12.5)));
    ui.horizontal(|ui| {
        for d in f.datums.iter_mut() {
            ui.add(egui::TextEdit::singleline(d).desired_width(56.0).hint_text("—"));
        }
    });
    ui.checkbox(&mut f.all_around, "All Around");
    ui.label(egui::RichText::new("Then pick an edge, or a dimension to hang it under").color(t.ink2));
}

/// File > New Drawing: the open part's or assembly's drawing, beside it, with a base view and
/// its three projections on the sheet size chosen.
pub fn new_drawing(model: &str, size: &str, standard: &str) -> Result<String, String> {
    let path = std::path::Path::new(model);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("no file name")?;
    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    let out = (0..)
        .map(|k| {
            dir.join(if k == 0 {
                format!("{stem}_dwg.fg")
            } else {
                format!("{stem}_dwg{k}.fg")
            })
        })
        .find(|p| !p.exists())
        .ok_or("no name free")?;
    let src = std::fs::read_to_string(model).map_err(|e| format!("{model}: {e}"))?;
    let asm = forge_lang::syntax::parse(&src).is_ok_and(|p| forge_lang::asm::declared(&p));
    let (w, h) = forge_draw::sheet::size(size).ok_or("no such sheet size")?;
    // a scale that fits the model's front view in a third of the sheet
    let opts = forge_lang::Options {
        build: true,
        render: true,
        dir: Some(dir.to_path_buf()),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, forge_lang::build::Cache::default());
    let size_mm = forge_lang::drawing::Model::of(&run)
        .map(|m| {
            m.bodies
                .iter()
                .map(|(b, pose)| {
                    let bb = b.faces_box();
                    bb.diagonal() + pose.origin.to_vec().length() * 0.0
                })
                .fold(1.0, f64::max)
        })
        .unwrap_or(100.0);
    let fit = (w * 0.3 / size_mm).min(h * 0.35 / size_mm);
    let scale = SCALES
        .iter()
        .map(|s| s.1)
        .filter(|s| *s <= fit)
        .fold(0.05, f64::max);
    let kind = if asm { "assembly" } else { "part" };
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (cx, cy) = (w * 0.25, h * 0.38);
    let text = format!(
        "drawing\n# {stem} drawing: made by File > New Drawing.\n\nm = {kind}(\"{file}\")\ns1 = sheet(\"{size}\", standard: \"{standard}\", title: \"{}\")\nfront = base_view(m, sheet: s1, dir: \"front\", scale: {}, at: ({}, {}))\ntop = projected(front, at: ({}, {}))\nright = projected(front, at: ({}, {}))\niso = projected(front, at: ({}, {}))\n",
        stem.to_uppercase().replace('_', " "),
        num(scale),
        num(cx),
        num(cy),
        num(cx),
        num(h * 0.75),
        num(w * 0.55),
        num(cy),
        num(w * 0.62),
        num(h * 0.72)
    );
    std::fs::write(&out, text).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().into_owned())
}

/// The New Drawing dialog's choices.
#[derive(Clone, Debug, PartialEq)]
pub struct NewDrawing {
    pub model: String,
    pub size: usize,
    pub iso: bool,
}

impl Shell {
    /// File > New Drawing's dialog: the sheet size and standard, then the drawing opens.
    pub(super) fn new_drawing_window(&mut self, ctx: &egui::Context) {
        let Some(nd) = &mut self.new_drawing else { return };
        let mut ok = false;
        let mut cancel = false;
        egui::Window::new("New Drawing")
            .id(egui::Id::unique("new-dwg"))
            .frame(super::fgasm::dialog_frame(&self.theme))
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(500.0, 220.0))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(format!("From {}", name_of(&nd.model))).font(semi(13.0)));
                ui.label("Sheet size");
                egui::Grid::new("sizes").show(ui, |ui| {
                    for (k, (n, w, h)) in forge_draw::sheet::SIZES.iter().enumerate() {
                        if ui
                            .selectable_label(nd.size == k, format!("{n}  {w} x {h}"))
                            .clicked()
                        {
                            nd.size = k;
                        }
                        if k % 5 == 4 {
                            ui.end_row();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Standard");
                    ui.selectable_value(&mut nd.iso, false, "ANSI (ASME Y14.5-2018), third angle");
                    ui.selectable_value(&mut nd.iso, true, "ISO 1101");
                });
                ui.horizontal(|ui| {
                    if ui.button("   OK   ").clicked() {
                        ok = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if cancel {
            self.new_drawing = None;
        }
        if ok {
            let nd = self.new_drawing.take().expect("the dialog is up");
            let size = forge_draw::sheet::SIZES[nd.size].0;
            match new_drawing(&nd.model, size, if nd.iso { "ISO" } else { "ASME" }) {
                // in a tab of its own, the model's left as it was
                Ok(p) if !self.doc_tabs.is_empty() => self.open_paths.push(p),
                Ok(p) => match DrawingEnv::open(&p) {
                    Ok(env) => self.open_drawing(env),
                    Err(e) => self.status = format!("New Drawing: {e}"),
                },
                Err(e) => self.status = format!("New Drawing: {e}"),
            }
        }
    }

    /// The Drawing environment, on this drawing.
    pub fn open_drawing(&mut self, env: DrawingEnv) {
        self.name = env.name();
        self.drawing = Some(env);
        self.tab = 0;
    }
}
