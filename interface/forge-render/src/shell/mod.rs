//! The app's chrome: title bar, ribbon, tree, panel, tabs, status bar. It knows nothing of the
//! window or the kernel's worker: it reads a document and says what was asked in `Action`s, so the
//! desktop app and the browser share it. Buttons never change the document themselves: the host
//! turns an action into a tool call.

pub mod agent;
pub mod agent_settings;
pub mod asmdrag;
pub mod asmglyph;
pub mod asmpart;
pub mod asmtools;
pub mod content;
pub mod dialog;
pub mod dim;
pub mod docs;
pub mod drawing;
pub mod dwgcmd;
#[cfg(test)]
mod dwgcmd_tests;
pub mod dwgcmds;
pub mod dwgdoc;
mod dwgdoc_edit_ui;
mod dwgdoc_ui;
pub mod dwgprops;
pub mod fg;
pub mod fgasm;
pub mod fgdialog;
mod fgedit;
pub mod fgsketch;
pub mod fgtool;
pub mod harness_look;
pub mod harness_ui;
pub mod icons;
pub mod inspect;
pub mod markup;
pub mod materials;
mod measure_ui;
pub mod occedit;
pub mod panel;
pub(crate) mod placed;
mod ribbon;
pub mod settings;
pub mod sheet;
mod sheetui;
mod sketch3d;
pub mod sketching;
pub mod theme;
pub mod thermo;
pub mod thermo_material;
pub mod thermo_ui;
mod tree;
mod visual_style;

pub use crate::nav::NavTool;
use crate::render::VisualStyle;
use crate::shell::placed::Placed;
use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_doc::{Body, Document, ItemId};
use theme::{Theme, medium, mono, sans, semi};

pub const TABS: [&str; 5] = ["3D Model", "Sketch", "Inspect", "View", "Manage"];
/// A sheet metal part's own tabs: shown first, Inventor's way; the flat pattern's in its place.
pub const SHEET_TAB: usize = 5;
pub const FLAT_TAB: usize = 6;
/// A Forge part's Thermoform tab: after Sketch.
pub const THERMO_TAB: usize = 7;
/// Assemble: a Forge file's, beside 3D Model, whatever the file holds (any file may hold
/// components and joints).
pub const ASM_TAB: usize = 8;
/// An assembly's Harness tab: after Assemble.
pub const HARNESS_TAB: usize = 9;

const TITLE_H: f32 = 36.0;
const TABS_H: f32 = 34.0;
/// How long a check goes on before the status bar says it's going on, in ms.
const CHECK_SHOWS_MS: f64 = 500.0;
/// A marking menu's dead zone, the ring its wedges light to, how far from it the pointer still
/// chooses, and how far a flick must go.
const DEAD_PT: f32 = 10.0;
const RING_PT: f32 = 42.0;
const WEDGE_FAR_PT: f32 = 260.0;
const FLICK_PT: f32 = 16.0;
/// One of a marking menu's: its word, its icon, what it does if it can.
pub type Mark = (&'static str, &'static str, Option<Action>);
pub type Marks = [Mark; 8];
const RIBBON_H: f32 = 104.0;
const SIDE_W: f32 = 270.0;
/// An assembly's browser: wider, for what each relationship ties.
const ASM_SIDE_W: f32 = 340.0;
const SIDE_MIN_W: f32 = 180.0;
const HEAD_H: f32 = 30.0;
const ROW_H: f32 = 26.0;
const DOCS_H: f32 = 28.0;
const STATUS_H: f32 = 24.0;

/// Where a dialog may stand: the window less its title bar, the document tabs and status bar.
pub(crate) fn dialog_area(ctx: &egui::Context) -> egui::Rect {
    let s = ctx.content_rect();
    egui::Rect::from_min_max(
        egui::pos2(s.left(), s.top() + TITLE_H),
        egui::pos2(s.right(), s.bottom() - DOCS_H - STATUS_H),
    )
}
const NAVBAR_W: f32 = 32.0;
/// Below the view cube's widget.
const NAVBAR_TOP: f32 = 176.0;
const RESIZE_EDGE: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

/// What the chrome was asked for. The host does it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// An untitled Forge part, in a tab.
    New,
    /// An .fc part: the host asks where, and it opens in a tab.
    NewFc,
    Open,
    Undo,
    Redo,
    FitAll,
    /// Square on to the sketch being drawn or the one chosen.
    LookAt,
    /// A sketch's eye.
    SketchVisibility(ItemId),
    /// The part reads in this length unit, from now on.
    SetUnits(&'static str),
    /// Back out of picking a plane.
    CancelPick,
    /// Nothing chosen.
    Deselect,
    Home,
    SetHome,
    /// Home is this part of the cube's view.
    SetHomeAt([i8; 3]),
    /// This face of the cube takes this name.
    SetFaceAs([i8; 3], [i8; 3]),
    ResetOrientation,
    VisualStyle(VisualStyle),
    Section,
    /// Inventor grey or a pastel per part.
    PartColours(bool),
    Tool(NavTool),
    Parallel(bool),
    Minimize,
    Maximize,
    Close,
    DragWindow,
    ResizeWindow(Edge),
    /// Start Sketch: on what's chosen, or wait for a plane or a planar face to be picked.
    StartSketch,
    SketchTool(sketching::SkTool),
    /// A feature's inline profile, in sketch mode.
    EditInline(ItemId),
    FinishSketch,
    CancelSketch,
    /// A ribbon button: a dialog for a new feature of this kind.
    NewFeature(&'static str),
    /// A dialog ended: OK, or Cancel.
    DialogOk,
    DialogCancel,
    /// What the tree and the marking menu ask of a feature.
    Edit(ItemId),
    Delete(ItemId),
    Suppress(ItemId),
    Unsuppress(ItemId),
    /// Move it up to before this one, or to the end.
    Reorder(ItemId, Option<ItemId>),
    /// The end-of-part marker after this feature, or at the end.
    EndAt(Option<ItemId>),
    StartRename(ItemId),
    /// A dragged number's new value; the drag ending there; the drag or edit called off.
    DimDrag(f64),
    DimCommit(f64),
    DimCancel,
    /// A Forge file's ribbon command.
    Fg(FgCmd),
    /// Measure, Section, iProperties and the centre of gravity.
    Inspect(inspect::InspectCmd),
    /// A sheet metal command, from the Sheet Metal or Flat Pattern tab.
    Sheet(sheet::SheetCmd),
    /// A drawing's ribbon command.
    Dwg(drawing::DwgCmd),
    /// A DWG document's command, from its ribbon.
    DwgDoc(dwgdoc::Cmd),
    /// File > New Drawing, from the open part or assembly; File > Close Drawing.
    NewDrawing,
    CloseDrawing,
    /// A shortcut's command (`settings::KeyCmd`), its key as set in Settings.
    Key(settings::KeyCmd),
    /// Manage > Settings.
    Settings,
    /// The document tabs' asks.
    Doc(docs::DocCmd),
    /// Where an untitled document is to be saved: the host's file dialog, then `Gui::save_as`.
    SaveAs,
    /// A ground or shadow switch of the View tab, flipped.
    Look(crate::look::Toggle),
}

/// What the ribbon asks of a Forge file: Inventor's dialogs and commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FgCmd {
    /// The Thermoform tab's.
    Thermo(thermo_ui::Cmd),
    /// The Harness tab's.
    Harness(harness_ui::Cmd),
    Parameters,
    Interference,
    Bom,
    Explode,
    Save,
    /// The assembly's authoring tools (asmtools.rs).
    Place,
    Joint,
    Constrain,
    Pattern,
    Mirror,
    Copy,
    FreeMove,
    FreeRotate,
    /// A drag's transient pose written to the file, or dropped (asmdrag.rs).
    KeepPosition,
    ReturnToDesign,
    ContentCenter,
    /// Show All of the joints' and constraints' glyphs in the view (else only those called for).
    Relationships,
}

/// One row of the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: ItemId,
    pub name: String,
    pub kind: String,
    pub icon: &'static str,
    /// As the tree reads: the kind's word and its count, "Extrude2".
    pub title: String,
    /// Its line in the part file, label and all.
    pub line: String,
    /// Out of the build, kept as a comment.
    pub suppressed: bool,
    /// What it's made from: its sketch, shown under it.
    pub sketch: Option<SketchChild>,
    /// A sketch that feature rows carry: not shown at the top.
    pub tucked: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SketchChild {
    /// A `sketch` line, with its title.
    Line(ItemId, String),
    /// The profile's in the feature's own line.
    Inline(panel::InlineProfile, String),
}

/// What the view shows, for the buttons that reflect it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    pub style: VisualStyle,
    pub section: bool,
    pub parallel: bool,
    pub tool: NavTool,
    pub maximized: bool,
    /// Parts in a pastel each, not Inventor's grey.
    pub pastel: bool,
}

impl Default for ViewState {
    fn default() -> ViewState {
        ViewState {
            style: VisualStyle::ShadedEdges,
            section: false,
            parallel: true,
            tool: NavTool::Select,
            maximized: false,
            pastel: false,
        }
    }
}

/// What's drawn over a Forge file's model besides it: each switched in the view's toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overlays {
    /// The origin's axes and centre point.
    pub axes: bool,
    /// A grid on the ground (xy).
    pub grid: bool,
    /// The origin's planes, sketch planes and work planes.
    pub planes: bool,
    /// Declared anchors: a part's own, or the chosen occurrence's in an assembly.
    pub anchors: bool,
}

pub const PART_OVERLAYS: Overlays = Overlays {
    axes: true,
    grid: true,
    planes: false,
    anchors: false,
};
pub const ASM_OVERLAYS: Overlays = Overlays {
    axes: false,
    grid: false,
    planes: false,
    anchors: false,
};

/// The toolbar's switches: their names (for places and tests) and words.
pub const OVERLAY_SWITCHES: [(&str, &str); 4] = [
    ("axes", "Axes"),
    ("grid", "Grid"),
    ("planes", "Planes"),
    ("anchors", "Anchors"),
];

impl Overlays {
    pub fn get(&self, name: &str) -> bool {
        match name {
            "axes" => self.axes,
            "grid" => self.grid,
            "planes" => self.planes,
            _ => self.anchors,
        }
    }

    pub fn flip(&mut self, name: &str) {
        let v = match name {
            "axes" => &mut self.axes,
            "grid" => &mut self.grid,
            "planes" => &mut self.planes,
            _ => &mut self.anchors,
        };
        *v = !*v;
    }
}

pub struct Shell {
    pub theme: Theme,
    pub tab: usize,
    pub name: String,
    pub doc: Option<Document>,
    pub rows: Vec<Row>,
    pub failed: Option<(ItemId, String)>,
    pub selected: Option<ItemId>,
    pub version: u64,
    pub status: String,
    pub view: ViewState,
    pub out: Vec<Action>,
    /// Where the viewport is, in logical pixels, snapped to device pixels.
    pub view_rect: Rect,
    /// Where each thing was drawn, last frame, by name: tests and scripts click by it.
    pub places: Vec<(String, Rect)>,
    /// The view cube's right-click menu, at this point.
    pub cube_menu: Option<egui::Pos2>,
    /// The press that opened it is still to come through the frame: it isn't a press outside it.
    pub cube_menu_fresh: bool,
    /// What part of the cube it's for.
    pub cube_region: Option<[i8; 3]>,
    /// The part's picture, drawn by the renderer this same frame.
    pub view_tex: Option<egui::TextureId>,
    /// The view cube's words, in the view's device pixels: real type over its picture.
    pub cube_labels: Vec<crate::viewcube::Label>,
    pub cube_place: Option<crate::viewcube::Place>,
    /// The view's camera and its size in device pixels, for what's drawn on the model.
    pub camera: Option<(crate::camera::Camera, (u32, u32))>,
    /// The Origin folder's open; which of its items are always shown; which is hovered or selected.
    pub origin_open: bool,
    pub origin_shown: [bool; 7],
    pub origin_hover: Option<crate::origin::Item>,
    origin_tree_hover: Option<crate::origin::Item>,
    pub origin_selected: Option<crate::origin::Item>,
    /// The feature whose faces the pointer's over in the tree.
    pub tree_hover: Option<ItemId>,
    /// The end-of-part marker: after this feature, or at the end.
    pub end_at: Option<ItemId>,
    /// A row's name being typed; a name typed, for the tool.
    pub renaming: Option<(ItemId, String)>,
    pub renamed: Option<(ItemId, String)>,
    /// The tree's right-click menu: for this row, at this point.
    /// What the model's sketches are, in 3D.
    pub sketch_shapes: Vec<forge_agent::handles::SketchShape>,
    /// Sketches told to show or hide, by name. Others show until something reads them.
    pub sketch_eye: std::collections::HashMap<String, bool>,
    /// The sketch under the pointer in the view.
    pub sketch_hover: Option<ItemId>,
    /// It built but couldn't all be drawn.
    pub unshown: bool,
    /// What the check found of the part: amber in the tree, counted in the status bar.
    pub flags: Vec<forge_agent::live::Flag>,
    /// A check's been going on since then.
    pub checking_since: Option<f64>,
    /// The length unit the part reads in.
    pub units: &'static str,
    /// The part's middle: which way a hole faces in.
    pub part_middle: [f32; 3],
    /// The marking menu's up, centred here.
    pub marking: Option<egui::Pos2>,
    /// It's for this row of the tree.
    pub marking_row: Option<ItemId>,
    /// A selection window being dragged, on screen.
    pub window_box: Option<(egui::Pos2, egui::Pos2)>,
    /// Sketch mode.
    pub sketch: Option<sketching::SketchMode>,
    /// Waiting for a plane or a face to sketch on.
    pub sketch_pick: bool,
    /// Feature rows opened to show their sketch.
    pub open_rows: std::collections::HashSet<ItemId>,
    /// A new feature waiting for the sketch being drawn.
    pub then_feature: Option<&'static str>,
    /// The feature dialog that's up.
    pub dialog: Option<dialog::Dialog>,
    /// The selected feature's number, on the model.
    pub dim: Option<dim::DimEdit>,
    /// What's on the model and takes the pointer from the view.
    pub view_blocks: Vec<Rect>,
    /// The browser's width as dragged; None for the mode's default.
    pub side_px: Option<f32>,
    /// Place from Content Center: the standard parts library's dialog.
    pub content: content::ContentCenter,
    /// What was typed for it, committed with Enter.
    pub dim_typed: Option<String>,
    /// A Forge (.fg) part is open: its browser and dialogs instead of the .fc ones.
    pub fg: Option<fg::FgPanel>,
    /// The view's switches for parts and for assemblies: kept for the session, each kind its own.
    pub overlays: [Overlays; 2],
    pub file_menu: bool,
    /// The sheet metal environment: its tabs, dialogs and flat pattern.
    pub sheet: sheet::SheetUi,
    style_set: Option<bool>,
    /// Measure, the section views, iProperties (`inspect.rs`).
    pub inspect: inspect::Inspect,
    /// A drawing is open: the Drawing environment instead of the model's.
    pub drawing: Option<drawing::DrawingEnv>,
    /// File > New Drawing's dialog.
    pub new_drawing: Option<drawing::NewDrawing>,
    /// A DWG file is open: AutoCAD's layout instead of the model's (`dwgdoc.rs`).
    pub dwgdoc: Option<dwgdoc::DwgDoc>,
    /// What's set in Settings, and its window.
    pub settings: settings::Settings,
    pub prefs: settings::Window,
    /// The left panel's Agent tab (`agent.rs`).
    pub agent: agent::AgentTab,
    /// The open documents' tabs, and which is in the window.
    pub doc_tabs: Vec<docs::DocTab>,
    pub active_doc: usize,
    /// The tab whose changes aren't saved, being closed: Save, Don't Save or Cancel.
    pub close_ask: Option<usize>,
    /// Files to open in tabs of their own: a new drawing, a part from its assembly.
    pub open_paths: Vec<String>,
    /// Drawing blocks of .fg files to open in tabs of their own (file, block).
    pub open_blocks: Vec<(String, String)>,
    /// How far the strip is scrolled; the tab being dragged.
    doc_scroll: f32,
    doc_drag: Option<usize>,
}

impl Default for Shell {
    fn default() -> Shell {
        Shell {
            theme: theme::DARK,
            tab: 0,
            name: String::new(),
            doc: None,
            rows: Vec::new(),
            failed: None,
            selected: None,
            version: 0,
            status: String::new(),
            view: ViewState::default(),
            out: Vec::new(),
            view_rect: Rect::ZERO,
            places: Vec::new(),
            cube_menu: None,
            cube_menu_fresh: false,
            cube_region: None,
            view_tex: None,
            cube_labels: Vec::new(),
            cube_place: None,
            camera: None,
            sketch: None,
            sketch_pick: false,
            open_rows: Default::default(),
            then_feature: None,
            tree_hover: None,
            end_at: None,
            renaming: None,
            renamed: None,
            sketch_shapes: Vec::new(),
            sketch_eye: Default::default(),
            sketch_hover: None,
            unshown: false,
            flags: Vec::new(),
            checking_since: None,
            units: "mm",
            part_middle: [0.0; 3],
            marking: None,
            marking_row: None,
            window_box: None,
            origin_open: false,
            origin_shown: [false; 7],
            origin_hover: None,
            origin_tree_hover: None,
            origin_selected: None,
            dialog: None,
            dim: None,
            view_blocks: Vec::new(),
            side_px: None,
            content: Default::default(),
            dim_typed: None,
            fg: None,
            overlays: [PART_OVERLAYS, ASM_OVERLAYS],
            file_menu: false,
            sheet: sheet::SheetUi::new(),
            style_set: None,
            inspect: Default::default(),
            drawing: None,
            new_drawing: None,
            dwgdoc: None,
            settings: Default::default(),
            prefs: Default::default(),
            agent: Default::default(),
            doc_tabs: Vec::new(),
            active_doc: 0,
            close_ask: None,
            open_paths: Vec::new(),
            open_blocks: Vec::new(),
            doc_scroll: 0.0,
            doc_drag: None,
        }
    }
}

impl Shell {
    /// The document is what this text says now.
    pub fn set_doc(&mut self, src: &str, version: u64, failed: Option<(u64, String)>) {
        let doc = Document::parse(src);
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let sketch_names: Vec<(ItemId, String)> = doc
            .items
            .iter()
            .filter(|i| matches!(i.body, Body::Sketch { .. }))
            .filter_map(|i| Some((i.id, doc.name(i.id)?)))
            .collect();
        // who uses each sketch
        let uses = |it: &forge_doc::Item, id: ItemId, name: &str| {
            it.id != id && it.args().iter().any(|a| a.to == Some(id) || a.text == name)
        };
        self.rows = doc
            .items
            .iter()
            .filter_map(|it| {
                let note = it.note.as_deref();
                let (kind, text, suppressed) = match &it.body {
                    Body::Feature { kind, .. } => {
                        (kind.clone(), doc.text_of(it.id).unwrap_or_default(), false)
                    }
                    Body::Sketch { .. } => (
                        "sketch".to_string(),
                        doc.text_of(it.id).unwrap_or_default(),
                        false,
                    ),
                    Body::Blank => {
                        let (_, line) = note.and_then(forge_agent::tools::suppressed)?;
                        let words = forge_doc::text::tokens(line);
                        let at = usize::from(words.first().is_some_and(|w| w.ends_with(':')));
                        (words.get(at)?.clone(), line.to_string(), true)
                    }
                    _ => return None,
                };
                let word = panel::kind_word(&kind);
                let count = counts.entry(word.clone()).or_default();
                *count += 1;
                let title = forge_agent::tools::display_name(note)
                    .map_or_else(|| format!("{word}{count}"), str::to_string);
                let n_users = |id: ItemId, name: &str| {
                    doc.items
                        .iter()
                        .filter(|o| matches!(o.body, Body::Feature { .. }) && uses(o, id, name))
                        .count()
                };
                let tucked = matches!(it.body, Body::Sketch { .. })
                    && sketch_names
                        .iter()
                        .any(|(id, n)| *id == it.id && n_users(*id, n) == 1);
                let used = sketch_names.iter().find(|(id, n)| uses(it, *id, n)).cloned();
                let inline = match &it.body {
                    Body::Feature { kind, args } => {
                        let words: Vec<&str> = args.iter().map(|a| a.text.as_str()).collect();
                        panel::inline_profile(kind, &words)
                    }
                    _ => None,
                };
                Some(Row {
                    id: it.id,
                    title,
                    line: text.lines().next().unwrap_or("").to_string(),
                    name: doc.name(it.id).unwrap_or_else(|| kind.clone()),
                    icon: panel::icon(&kind),
                    kind,
                    suppressed,
                    sketch: match (used, inline) {
                        (Some((id, _)), _) => Some(SketchChild::Line(id, String::new())),
                        (None, Some(p)) => Some(SketchChild::Inline(p, String::new())),
                        _ => None,
                    },
                    tucked,
                })
            })
            .collect();
        // a child's title is its sketch's, in the count the tree shows
        let mut inline_count = self.rows.iter().filter(|r| r.kind == "sketch").count();
        let titles: Vec<(ItemId, String)> = self.rows.iter().map(|r| (r.id, r.title.clone())).collect();
        for r in &mut self.rows {
            match &mut r.sketch {
                Some(SketchChild::Line(id, title)) => {
                    *title = titles
                        .iter()
                        .find(|t| t.0 == *id)
                        .map_or(String::new(), |t| t.1.clone());
                }
                Some(SketchChild::Inline(_, title)) => {
                    inline_count += 1;
                    *title = format!("Sketch{inline_count}");
                }
                None => {}
            }
        }
        self.failed = failed.map(|(id, why)| (ItemId(id), why));
        if self
            .selected
            .is_some_and(|s| !self.rows.iter().any(|r| r.id == s))
        {
            self.selected = None;
        }
        self.units = forge_doc::units::of_text(src);
        self.doc = Some(doc);
        self.version = version;
    }

    /// A Forge assembly is open (known from its text, before its first solve).
    pub fn is_asm(&self) -> bool {
        self.fg.as_ref().is_some_and(|f| f.assembly)
    }

    pub fn side_w(&self) -> f32 {
        self.side_px
            .unwrap_or(if self.is_asm() { ASM_SIDE_W } else { SIDE_W })
    }

    /// A ribbon command for a Forge file: its dialog up or down.
    pub fn fg_command(&mut self, c: FgCmd) {
        if c == FgCmd::ContentCenter {
            self.content.open = !self.content.open;
            return;
        }
        let Some(fg) = &mut self.fg else { return };
        match c {
            FgCmd::Thermo(c) => fg.thermo_command(c),
            FgCmd::Harness(c) => fg.harness_command(c),
            FgCmd::Parameters => fg.params_open = !fg.params_open,
            FgCmd::Interference => {
                if let Some(a) = &mut fg.asm {
                    a.interference = !a.interference;
                }
            }
            FgCmd::Bom => {
                if let Some(a) = &mut fg.asm {
                    a.bom_open = !a.bom_open;
                }
            }
            FgCmd::Relationships => {
                if let Some(a) = &mut fg.asm {
                    a.glyphs.all = !a.glyphs.all;
                }
            }
            FgCmd::Explode => {
                if let Some(a) = &mut fg.asm {
                    a.explode_open = !a.explode_open;
                    a.explode = if a.explode_open { 1.0 } else { 0.0 };
                }
            }
            FgCmd::Save => fg.out.push(fg::FgAct::Save),
            FgCmd::KeepPosition => self.status = asmdrag::command(fg, true),
            FgCmd::ReturnToDesign => self.status = asmdrag::command(fg, false),
            FgCmd::ContentCenter => {}
            c => self.status = asmtools::command(fg, c),
        }
    }

    /// The switches for what's open.
    pub fn overlays(&self) -> Overlays {
        self.overlays[usize::from(self.is_asm())]
    }

    pub fn overlays_mut(&mut self) -> &mut Overlays {
        let k = usize::from(self.is_asm());
        &mut self.overlays[k]
    }

    /// The view's switches, top left over a Forge file's view: on ones filled.
    fn view_toolbar(&mut self, ctx: &egui::Context) {
        if self.fg.is_none() || self.view_rect.width() < 200.0 {
            return;
        }
        let t = self.theme;
        let at = self.view_rect.min + vec2(10.0, 10.0);
        let o = self.overlays();
        let mut flip = None;
        let mut places = Vec::new();
        let r = egui::Area::new(egui::Id::unique("view-toolbar"))
            .fixed_pos(at)
            .order(egui::Order::Middle)
            .show_placed(ctx, &mut places, |ui, places| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (name, word) in OVERLAY_SWITCHES {
                        let on = o.get(name);
                        let g = ui.fonts_mut(|f| f.layout_no_wrap(word.to_string(), sans(12.0), t.ink));
                        let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 18.0, 24.0), Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, on, word));
                        let p = ui.painter();
                        let fill = if on {
                            t.on
                        } else if resp.hovered() {
                            t.hover
                        } else {
                            t.surface.gamma_multiply(0.85)
                        };
                        p.rect_filled(r, 4.0, fill);
                        p.rect_stroke(r, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                        p.text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            word,
                            sans(12.0),
                            if on { t.on_ink } else { t.ink2 },
                        );
                        places.push((format!("view:{name}"), r));
                        if resp.clicked() {
                            flip = Some(name);
                        }
                    }
                });
            });
        self.view_blocks.push(r.response.rect);
        self.places.extend(places);
        if let Some(name) = flip {
            self.overlays_mut().flip(name);
        }
    }

    pub fn place(&self, name: &str) -> Option<Rect> {
        self.places.iter().find(|p| p.0 == name).map(|p| p.1)
    }

    fn note(&mut self, name: impl Into<String>, r: Rect) {
        self.places.push((name.into(), r));
    }

    /// A menu's up: a press anywhere else only closes it.
    pub fn menu_open(&self) -> bool {
        self.file_menu || self.cube_menu.is_some() || self.marking.is_some()
    }

    /// The marking menu for where things stand: eight places clockwise from the top, and a list
    /// under them for the rest.
    pub fn marking_items(&self) -> (Marks, Vec<Mark>) {
        const NONE: Mark = ("", "", None);
        let row = |id: ItemId| -> (Marks, Vec<Mark>) {
            let off = self.rows.iter().any(|r| r.id == id && r.suppressed);
            let sketch = self.rows.iter().any(|r| r.id == id && r.kind == "sketch");
            let mut more: Vec<Mark> = vec![(
                "Move End of Part Here",
                "end",
                (!off).then_some(Action::EndAt(Some(id))),
            )];
            if sketch {
                let word = if self.sketch_shown(id) { "Hide" } else { "Show" };
                more.push(("Visibility", "eye", Some(Action::SketchVisibility(id))));
                let _ = word;
            }
            (
                [
                    ("Edit Feature", "edit", (!off).then_some(Action::Edit(id))),
                    NONE,
                    ("Rename", "rename", (!off).then_some(Action::StartRename(id))),
                    NONE,
                    ("Delete", "delete", (!off).then_some(Action::Delete(id))),
                    NONE,
                    (
                        if off { "Unsuppress" } else { "Suppress" },
                        "hide",
                        Some(if off {
                            Action::Unsuppress(id)
                        } else {
                            Action::Suppress(id)
                        }),
                    ),
                    NONE,
                ],
                more,
            )
        };
        if let Some(id) = self.marking_row {
            return row(id);
        }
        // a Forge tool's dialog, as Inventor's: OK and Cancel under the right button too
        if self.dialog.is_some() || self.fg.as_ref().is_some_and(|p| p.tool.is_some()) {
            return (
                [
                    ("OK", "finish", Some(Action::DialogOk)),
                    NONE,
                    NONE,
                    NONE,
                    ("Cancel", "close", Some(Action::DialogCancel)),
                    NONE,
                    NONE,
                    NONE,
                ],
                Vec::new(),
            );
        }
        if let Some(m) = &self.sketch {
            let _ = m;
            let tool = |t| self.sketch_tool(t);
            use sketching::SkTool as K;
            return (
                [
                    ("Finish Sketch", "finish", Some(Action::FinishSketch)),
                    ("Line", "line", tool(K::Line)),
                    ("Rectangle", "rectangle", tool(K::Rectangle)),
                    ("Circle", "circle", tool(K::Circle)),
                    ("Cancel", "close", Some(Action::CancelSketch)),
                    ("Arc", "arc", tool(K::Arc)),
                    ("Dimension", "dim", tool(K::Dimension)),
                    ("Look At", "lookat", Some(Action::LookAt)),
                ],
                vec![
                    ("Undo", "undo", Some(Action::Undo)),
                    ("Fit All", "fit", Some(Action::FitAll)),
                ],
            );
        }
        // a dragged pose, Inventor's: kept or returned from the view's menu
        if self
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .is_some_and(|a| !a.posing.is_empty())
        {
            return (
                [
                    ("Keep Position", "pin", Some(Action::Fg(FgCmd::KeepPosition))),
                    ("Undo", "undo", Some(Action::Undo)),
                    NONE,
                    ("Fit All", "fit", Some(Action::FitAll)),
                    (
                        "Return to Design",
                        "home",
                        Some(Action::Fg(FgCmd::ReturnToDesign)),
                    ),
                    NONE,
                    ("Home", "home", Some(Action::Home)),
                    NONE,
                ],
                Vec::new(),
            );
        }
        if let Some(id) = self.selected.filter(|s| self.rows.iter().any(|r| r.id == *s)) {
            let (mut m, more) = row(id);
            m[1] = ("Undo", "undo", Some(Action::Undo));
            m[3] = ("Fit All", "fit", Some(Action::FitAll));
            return (m, more);
        }
        (
            [
                ("Undo", "undo", Some(Action::Undo)),
                ("Start Sketch", "sketch", Some(Action::StartSketch)),
                ("Extrude", "extrude", Some(Action::NewFeature("extrude"))),
                ("Fillet", "fillet", Some(Action::NewFeature("fillet"))),
                ("Redo", "redo", Some(Action::Redo)),
                ("Hole", "hole", Some(Action::NewFeature("hole"))),
                ("Home", "home", Some(Action::Home)),
                ("Fit All", "fit", Some(Action::FitAll)),
            ],
            vec![
                ("Revolve", "revolve", Some(Action::NewFeature("revolve"))),
                ("Chamfer", "chamfer", Some(Action::NewFeature("chamfer"))),
            ],
        )
    }

    /// A flick of the right button from `from` to `to`: which of the eight, if it went far enough.
    pub fn marking_flick(from: (f32, f32), to: (f32, f32), scale: f32) -> Option<usize> {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        if (dx * dx + dy * dy).sqrt() < FLICK_PT * scale {
            return None;
        }
        let a = dx.atan2(-dy).rem_euclid(core::f32::consts::TAU);
        Some(((a / core::f32::consts::FRAC_PI_4).round() as usize) % 8)
    }

    /// The whole window.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let t = self.theme;
        if self.style_set != Some(t.dark) {
            theme::apply(&ctx, &t);
            self.style_set = Some(t.dark);
        }
        // the tooltip's delay as Settings has it
        if ctx.global_style().interaction.tooltip_delay != self.settings.tip_delay {
            let d = self.settings.tip_delay;
            ctx.all_styles_mut(|s| s.interaction.tooltip_delay = d);
        }
        self.places.clear();
        self.view_blocks.clear();
        self.doc_keys(ui);
        let s = ctx.content_rect();
        let p = ui.painter().clone();
        p.rect_filled(s, 0.0, t.bg);
        if self.dwgdoc.is_some() {
            self.dwgdoc_ui(ui);
            self.settings_window(&ctx);
            return;
        }
        if self.drawing.is_some() {
            self.shortcuts(&ctx);
            self.drawing_ui(ui);
            self.new_drawing_window(&ctx);
            self.settings_window(&ctx);
            self.markup_ui(ui);
            return;
        }

        let top = TITLE_H + TABS_H + RIBBON_H;
        let bottom = s.height() - DOCS_H - STATUS_H;
        let side_w = self.side_w();
        let side = Rect::from_min_max(pos2(0.0, top), pos2(side_w, bottom));
        let ppp = ctx.pixels_per_point();
        let snap = |v: f32| (v * ppp).round() / ppp;
        self.view_rect = Rect::from_min_max(
            pos2(snap(side_w + 1.0), snap(top)),
            pos2(snap(s.width()), snap(bottom)),
        );

        p.rect_filled(
            self.view_rect,
            0.0,
            egui::Color32::from_rgb((t.view >> 16) as u8, (t.view >> 8) as u8, t.view as u8),
        );
        if let Some(tex) = self.view_tex {
            let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            p.image(tex, self.view_rect, whole, Color32::WHITE);
        }
        self.cube_words(ui, ppp);
        self.title_bar(ui, s.width());
        self.shortcuts(&ctx);
        self.escape_one_level(&ctx);
        self.tabs_row(ui, s.width());
        self.ribbon(ui, s.width());
        self.side(ui, side);
        self.side_splitter(ui, side, s.width());
        self.docs_and_status(ui, s, bottom);
        self.navbar(&ctx);
        self.sketches_ui(&ctx);
        self.window_ui(&ctx);
        self.dim_ui(&ctx);
        self.thermo_probes(&ctx);
        self.thermo_ui(&ctx);
        self.harness_ui(&ctx);
        self.dialog_ui(&ctx);
        self.fg_dialog(&ctx);
        self.occ_dialog(&ctx);
        self.asm_windows(&ctx);
        self.inspect_ui(&ctx);
        self.content_window(&ctx);
        self.fg_params_window(&ctx);
        self.fg_issues_window(&ctx);
        self.sheet_ui(&ctx);
        self.view_toolbar(&ctx);
        self.sketch_ui(&ctx);
        self.mode_bar(&ctx);
        self.resize_edges(ui, s);
        self.menus(&ctx);
        self.settings_window(&ctx);
        self.new_drawing_window(&ctx);
        self.markup_ui(ui);
    }

    /// The cube's words, painted on its faces: each glyph's corners go through the face's own
    /// projection, so a word foreshortens with its face.
    fn cube_words(&self, ui: &mut Ui, ppp: f32) {
        let t = self.theme;
        let (Some((camera, _)), Some(place)) = (self.camera, self.cube_place) else {
            return;
        };
        // type sharper than it shows, as the atlas holds it; the word's em is fixed on the face
        let em_pt = crate::viewcube::LABEL_EM * crate::viewcube::half_side_px(&place) / ppp;
        let size = (em_pt * 2.0).round().max(8.0);
        let per = crate::viewcube::LABEL_EM / size;
        let atlas = ui.fonts_mut(|f| f.font_image_size());
        let (uw, uh) = (atlas[0] as f32, atlas[1] as f32);
        for l in &self.cube_labels {
            if l.alpha <= 0.0 {
                continue;
            }
            let ink = if l.hovered { t.on_ink } else { t.ink };
            let g = ui.fonts_mut(|f| f.layout_no_wrap(l.text.to_string(), medium(size), ink));
            let mid = g.mesh_bounds.center();
            let colour = ink.gamma_multiply(l.alpha);
            let mut mesh = egui::Mesh::with_texture(egui::TextureId::default());
            for placed in &g.rows {
                let m = &placed.row.visuals.mesh;
                let base = mesh.vertices.len() as u32;
                for v in &m.vertices {
                    let p = v.pos + placed.pos.to_vec2();
                    let px = crate::viewcube::on_face(
                        &camera,
                        &place,
                        l,
                        (p.x - mid.x) * per,
                        (mid.y - p.y) * per,
                    );
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: self.view_rect.min + vec2(px[0], px[1]) / ppp,
                        uv: pos2(v.uv.x / uw, v.uv.y / uh),
                        color: colour,
                    });
                }
                mesh.indices.extend(m.indices.iter().map(|i| i + base));
            }
            ui.painter().add(egui::Shape::mesh(mesh));
        }
    }

    /// Draws a flat clickable square with an icon, in the title bar's or the bar's colours.
    fn icon_button(
        &mut self,
        ui: &mut Ui,
        name: &str,
        r: Rect,
        icon: &str,
        tip: &str,
        ink: Color32,
    ) -> egui::Response {
        let resp = ui
            .interact(r, ui.scope_id().with(name), Sense::click())
            .on_hover_text(tip);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
        if resp.hovered() || resp.has_focus() {
            ui.painter().rect_filled(r, 4.0, ink.gamma_multiply(0.14));
        }
        if resp.has_focus() {
            ui.painter()
                .rect_stroke(r, 4.0, Stroke::new(2.0, self.theme.focus), StrokeKind::Inside);
        }
        icons::draw(
            ui.painter(),
            pos2(r.center().x - 8.0, r.center().y - 8.0),
            16.0,
            icon,
            1.5,
            ink,
        );
        self.note(name, r);
        resp
    }

    fn title_bar(&mut self, ui: &mut Ui, w: f32) {
        let t = self.theme;
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(w, TITLE_H));
        let p = ui.painter().clone();
        p.rect_filled(bar, 0.0, t.bar);

        // the bar itself moves the window; a double click maximizes
        let drag = ui.interact(bar, ui.scope_id().with("title-drag"), Sense::click_and_drag());
        if drag.double_clicked() {
            self.out.push(Action::Maximize);
        } else if drag.drag_started() {
            self.out.push(Action::DragWindow);
        }

        p.text(
            pos2(12.0, TITLE_H * 0.5),
            Align2::LEFT_CENTER,
            "ForgeCAD",
            semi(13.0),
            Color32::WHITE,
        );
        let quick = [
            ("new", "new", "New", Action::New),
            ("open", "open", "Open", Action::Open),
            ("undo", "undo", "Undo", Action::Undo),
            ("redo", "redo", "Redo", Action::Redo),
        ];
        for (k, (name, icon, tip, action)) in quick.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(84.0 + k as f32 * 30.0, 3.0), vec2(30.0, 30.0));
            if self.icon_button(ui, name, r, icon, tip, Color32::WHITE).clicked() {
                self.out.push(action);
            }
        }
        // a Forge file with edits not yet written says so, as File > Save clears it
        let dirty = self.drawing.is_none() && self.fg.as_ref().is_some_and(|f| f.dirty);
        let name = if dirty {
            format!("{} (unsaved)", self.name)
        } else {
            self.name.clone()
        };
        let r = p.text(
            pos2(w * 0.5, TITLE_H * 0.5),
            Align2::CENTER_CENTER,
            name,
            mono(12.0),
            Color32::WHITE,
        );
        self.note("title:name", r);

        let wins = [
            ("minimize", "minimize", "Minimize", Action::Minimize),
            ("maximize", "maximize", "Maximize", Action::Maximize),
            ("close", "close", "Close", Action::Close),
        ];
        for (k, (name, icon, tip, action)) in wins.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(w - 110.0 + k as f32 * 36.0, 3.0), vec2(34.0, 30.0));
            if self.icon_button(ui, name, r, icon, tip, Color32::WHITE).clicked() {
                self.out.push(action);
            }
        }
    }

    fn tabs_row(&mut self, ui: &mut Ui, w: f32) {
        let t = self.theme;
        let p = ui.painter().clone();
        let y = TITLE_H;
        p.rect_filled(Rect::from_min_size(pos2(0.0, y), vec2(w, TABS_H)), 0.0, t.surface);
        let up = CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        };
        let file = Rect::from_min_size(pos2(8.0, y + 4.0), vec2(52.0, 30.0));
        let resp = ui.interact(file, ui.scope_id().with("file"), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "File"));
        p.rect_filled(
            file,
            up,
            if resp.hovered() {
                t.accent.gamma_multiply(1.15)
            } else {
                t.accent
            },
        );
        p.text(
            file.center(),
            Align2::CENTER_CENTER,
            "File",
            semi(13.0),
            Color32::WHITE,
        );
        self.note("file", file);
        if resp.clicked() {
            self.file_menu = !self.file_menu;
        }
        let mut x = 66.0;
        let drawing = self.drawing.is_some();
        let dwg = self.dwgdoc.is_some();
        let order: Vec<usize> = if dwg {
            (0..dwgdoc::TABS.len()).collect()
        } else if drawing {
            (0..drawing::TABS.len()).collect()
        } else {
            self.tab_order()
        };
        for k in order {
            // the Model tab in an assembly too (one document); Thermoform where there is one
            if k == THERMO_TAB && !self.thermo_tab() {
                continue;
            }
            if k == HARNESS_TAB && !self.harness_tab() {
                continue;
            }
            let name = match k {
                k if dwg => &dwgdoc::TABS[k],
                k if drawing => &drawing::TABS[k],
                SHEET_TAB => &"Sheet Metal",
                THERMO_TAB => &"Thermoform",
                FLAT_TAB => &"Flat Pattern",
                ASM_TAB => &"Assemble",
                HARNESS_TAB => &"Harness",
                k => &TABS[k],
            };
            let on = k == self.tab;
            let w = ui
                .fonts_mut(|f| f.layout_no_wrap(name.to_string(), sans(13.0), t.ink))
                .size()
                .x
                + 28.0;
            let r = Rect::from_min_size(pos2(x, y + 4.0), vec2(w, 30.0));
            let resp = ui.interact(r, ui.scope_id().with(("tab", k)), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, *name));
            if resp.clicked() {
                self.tab = k;
                // Assemble's tools on a file with no components yet: placing one makes it an assembly
                if k == ASM_TAB
                    && let Some(f) = &mut self.fg
                {
                    f.asm.get_or_insert_with(Default::default);
                }
            }
            if on {
                p.rect_filled(r, up, t.raised);
                p.rect_stroke(r, up, Stroke::new(1.0, t.line), StrokeKind::Inside);
                p.rect_filled(Rect::from_min_size(r.min, vec2(w, 2.0)), 0.0, t.focus);
            } else if resp.hovered() {
                p.rect_filled(r, up, t.hover);
            }
            let (font, ink) = if on {
                (semi(13.0), t.ink)
            } else {
                (sans(13.0), if resp.hovered() { t.ink } else { t.ink2 })
            };
            p.text(r.center(), Align2::CENTER_CENTER, *name, font, ink);
            self.note(format!("tab:{name}"), r);
            x += w + 2.0;
        }
    }

    /// The tabs shown, in order: a sheet metal part's Sheet Metal first, the flat pattern's tab
    /// alone with Inspect, View and Manage.
    pub fn tab_order(&self) -> Vec<usize> {
        let fg = self.fg.is_some() && !self.is_asm();
        if fg && self.sheet.flat_env {
            vec![FLAT_TAB, 2, 3, 4]
        } else if fg && self.sheet.env {
            vec![SHEET_TAB, 0, 1, ASM_TAB, 2, 3, 4]
        } else if self.fg.is_some() {
            vec![0, 1, ASM_TAB, HARNESS_TAB, THERMO_TAB, 2, 3, 4]
        } else {
            vec![0, 1, THERMO_TAB, 2, 3, 4]
        }
    }

    fn side(&mut self, ui: &mut Ui, side: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(side, 0.0, t.surface);
        p.vline(side.right(), side.y_range(), Stroke::new(1.0, t.line));
        let name = self
            .name
            .trim_end_matches(".fc")
            .trim_end_matches(".fg")
            .to_string();
        // an assembly's name is its browser's top row
        let name = if self.is_asm() { String::new() } else { name };
        self.side_tabs(ui, side.top(), side.width(), &name);
        let tree = Rect::from_min_max(pos2(0.0, side.top() + HEAD_H), pos2(side.right(), side.bottom()));
        if self.agent.tab == agent::SideTab::Agent {
            self.agent_ui(ui, tree);
        } else if self.is_asm() && self.fg.as_ref().is_some_and(|f| f.asm.is_some()) {
            self.asm_tree(ui, tree);
        } else if self.fg.is_some() {
            self.fg_tree(ui, tree);
        } else {
            self.tree(ui, tree);
        }
        self.agent_drag(ui);
    }

    /// A feature's fields, from the document as it stands.
    pub fn fields_of(&self, id: ItemId) -> Vec<(String, String)> {
        let Some(item) = self
            .doc
            .as_ref()
            .and_then(|d| d.items.iter().find(|i| i.id == id))
        else {
            return Vec::new();
        };
        match &item.body {
            Body::Feature { kind, args } => {
                let words: Vec<&str> = args.iter().map(|a| a.text.as_str()).collect();
                panel::fields(kind, &words)
            }
            Body::Sketch { rows, .. } => rows
                .iter()
                .filter(|r| r.args.first().is_none_or(|w| w != "ids"))
                .filter_map(|r| panel::sketch_row(&r.args))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn docs_and_status(&mut self, ui: &mut Ui, s: Rect, bottom: f32) {
        self.close_prompt(&ui.ctx().clone());
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(
            Rect::from_min_size(pos2(0.0, bottom), vec2(s.width(), DOCS_H)),
            0.0,
            t.surface,
        );
        p.hline(0.0..=s.width(), bottom, Stroke::new(1.0, t.line));
        if !self.doc_tabs.is_empty() {
            self.doc_strip(
                ui,
                Rect::from_min_size(pos2(0.0, bottom + 1.0), vec2(s.width(), DOCS_H - 1.0)),
            );
        } else if !self.name.is_empty() {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(self.name.clone(), mono(12.0), t.ink));
            let d = Rect::from_min_size(pos2(8.0, bottom + 3.0), vec2(g.size().x + 24.0, 22.0));
            p.rect_filled(d, 3.0, t.raised);
            p.rect_stroke(d, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            p.galley(pos2(d.left() + 12.0, d.center().y - g.size().y * 0.5), g, t.ink);
        }
        let st = Rect::from_min_size(pos2(0.0, bottom + DOCS_H), vec2(s.width(), STATUS_H));
        p.rect_filled(st, 0.0, t.bar);
        let said = p.text(
            pos2(10.0, st.center().y),
            Align2::LEFT_CENTER,
            if self.status.is_empty() && self.unshown {
                "This part built but some of it can't be drawn: fix or delete what the tree marks"
            } else {
                &self.status
            },
            sans(12.0),
            Color32::WHITE,
        );
        // markup for the agent, on or off
        let mr = Rect::from_min_size(
            pos2(s.width() - 330.0, st.top() + 2.0),
            vec2(78.0, STATUS_H - 4.0),
        );
        let m_resp = ui
            .interact(mr, ui.scope_id().with("status-markup"), Sense::click())
            .on_hover_text("Draw on the window for the agent (K)");
        m_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Markup"));
        if self.agent.markup.on || m_resp.hovered() {
            p.rect_filled(
                mr,
                3.0,
                Color32::from_white_alpha(if self.agent.markup.on { 48 } else { 28 }),
            );
        }
        icons::draw(
            &p,
            pos2(mr.left() + 4.0, mr.center().y - 7.0),
            14.0,
            "markup",
            1.4,
            Color32::WHITE,
        );
        p.text(
            pos2(mr.left() + 22.0, mr.center().y),
            Align2::LEFT_CENTER,
            "Markup",
            sans(12.0),
            Color32::WHITE,
        );
        self.note("status:markup", mr);
        if m_resp.clicked() {
            self.agent.markup.toggle();
        }
        // a Forge file's issues a click away (an assembly's timings on hover): its checks, a
        // shop's rules, each with its fixes, in the Issues window
        let asm = self.is_asm();
        if let Some(fg) = &mut self.fg {
            if let Some(a) = &fg.asm
                && !a.timings.is_empty()
            {
                ui.interact(said, ui.scope_id().with("status-times"), Sense::hover())
                    .on_hover_text(&a.timings);
            }
            let (errors, warnings) = fg::issue_counts(&fg.issues);
            let text = format!(
                "{} · {}",
                fg::plural(errors, "error"),
                fg::plural(warnings, "warning")
            );
            let g = ui.fonts_mut(|f| f.layout_no_wrap(text.clone(), sans(12.0), Color32::WHITE));
            let right = s.width()
                - 10.0
                - ui.fonts_mut(|f| f.layout_no_wrap(format!("| {}", self.units), mono(12.0), Color32::WHITE))
                    .size()
                    .x
                - 12.0;
            let r = Rect::from_min_max(
                pos2(right - g.size().x - 22.0, st.top() + 2.0),
                pos2(right, st.bottom() - 2.0),
            );
            let resp = ui.interact(r, ui.scope_id().with("status-issues"), Sense::click());
            if resp.hovered() || fg.issues_list {
                p.rect_filled(r, 3.0, Color32::from_white_alpha(28));
            }
            let ink = if errors > 0 {
                t.error
            } else if warnings > 0 {
                fg::WARN
            } else {
                Color32::WHITE
            };
            icons::draw(
                &p,
                pos2(r.left() + 3.0, r.center().y - 7.0),
                14.0,
                "warning",
                1.5,
                ink,
            );
            p.galley(
                pos2(r.left() + 20.0, r.center().y - g.size().y * 0.5),
                g,
                Color32::WHITE,
            );
            let name = if asm { "asm:issues" } else { "fg:issues:status" };
            self.places.push((name.into(), r));
            if resp.clicked() {
                fg.issues_list = !fg.issues_list;
            }
            p.text(
                pos2(s.width() - 10.0, st.center().y),
                Align2::RIGHT_CENTER,
                format!("| {}", self.units),
                mono(12.0),
                Color32::WHITE,
            );
            return;
        }
        // the check's word only if it's taking long: a quick one shouldn't flicker
        let slow = self
            .checking_since
            .is_some_and(|t| crate::viewer::now_ms() - t > CHECK_SHOWS_MS);
        let state = if self.failed.is_some() {
            "error".to_string()
        } else if !self.flags.is_empty() {
            let n = self
                .flags
                .iter()
                .map(|f| f.item)
                .collect::<std::collections::HashSet<_>>()
                .len();
            format!("{n} warning{}", if n == 1 { "" } else { "s" })
        } else if slow {
            "checking".to_string()
        } else {
            "valid".to_string()
        };
        let right = format!("{state} | {}", self.units);
        p.text(
            pos2(s.width() - 10.0, st.center().y),
            Align2::RIGHT_CENTER,
            right,
            mono(12.0),
            Color32::WHITE,
        );
    }

    /// Esc backs out one level, wherever one is: never two, and never none.
    fn escape_one_level(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        if self.markup_escape(ctx) || self.agent.takes_escape(ctx) {
            return;
        }
        // these take it themselves, knowing what they had
        if self.dim.as_ref().is_some_and(|d| d.dragging) || self.dim_field_focused(ctx) {
            return;
        }
        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if ribbon::close_drop(ctx) {
            // a ribbon dropdown shuts, and that's all
        } else if self.menu_open() {
            self.file_menu = false;
            self.cube_menu = None;
            self.marking = None;
            self.marking_row = None;
        } else if self.renaming.is_some() {
            self.renaming = None;
        } else if let Some(p) = self.fg.as_mut().filter(|p| p.renaming.is_some()) {
            p.renaming = None;
        } else if self.dialog.is_some() {
            self.out.push(Action::DialogCancel);
        } else if let Some(p) = self.fg.as_mut().filter(|p| p.tool.is_some()) {
            p.tool = None;
            self.status.clear();
        } else if let Some(p) = self
            .fg
            .as_mut()
            .filter(|p| p.asm.as_ref().is_some_and(|a| !a.posing.is_empty()))
        {
            // a dragged pose: Return to Design
            self.status = asmdrag::command(p, false);
        } else if let Some(m) = &mut self.inspect.measure {
            // what's picked goes, then Measure
            if m.picks.is_empty() {
                self.inspect.measure = None;
            } else {
                m.restart();
            }
        } else if let Some(m) = &mut self.sketch {
            if m.has_tool() {
                m.escape();
            } else if m.has_changes() {
                self.out.push(Action::FinishSketch);
            } else {
                self.out.push(Action::CancelSketch);
            }
        } else if self.sketch_pick {
            self.out.push(Action::CancelPick);
        } else if self.selected.is_some() || self.origin_selected.is_some() {
            self.out.push(Action::Deselect);
        }
    }

    fn dim_field_focused(&self, ctx: &egui::Context) -> bool {
        self.sketch.is_none()
            && self.dialog.is_none()
            && self.renaming.is_none()
            && ctx.memory(|m| m.focused().is_some())
    }

    /// While a mode's waiting on something: what, and the way out, over the view.
    fn mode_bar(&mut self, ctx: &egui::Context) {
        let (text, finish, cancel) = if self.sketch_pick {
            (
                "Pick a plane, a planar face, or a sketch to edit".to_string(),
                None,
                Action::CancelPick,
            )
        } else if let Some(m) = &self.sketch {
            let what = if m.is_new() {
                "New sketch"
            } else {
                "Editing sketch"
            };
            (what.to_string(), Some(Action::FinishSketch), Action::CancelSketch)
        } else {
            return;
        };
        let t = self.theme;
        let v = self.view_rect;
        let mut out = None;
        let placed_before = self.places.len();
        let (_, sizing) = egui::Area::new(egui::Id::unique("mode-bar"))
            .pivot(Align2::CENTER_TOP)
            .fixed_pos(pos2(v.center().x, v.top() + 10.0))
            .order(egui::Order::Foreground)
            .show_sizing(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.surface)
                    .stroke(Stroke::new(1.0, t.line))
                    .corner_radius(4.0)
                    .inner_margin(vec2(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(text).font(sans(13.0)).color(t.ink));
                            ui.add_space(8.0);
                            if let Some(f) = finish {
                                let r = ui.button(egui::RichText::new("Finish").font(sans(13.0)));
                                self.places.push(("mode:finish".into(), r.rect));
                                if r.clicked() {
                                    out = Some(f);
                                }
                            }
                            let r = ui.button(egui::RichText::new("Cancel").font(sans(13.0)));
                            self.places.push(("mode:cancel".into(), r.rect));
                            if r.clicked() {
                                out = Some(cancel);
                            }
                        });
                    });
            });
        // what it named unseen, sizing itself, is off
        if sizing {
            self.places.truncate(placed_before);
        }
        self.out.extend(out);
    }

    fn navbar(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let v = self.view_rect;
        let at = pos2(v.right() - NAVBAR_W - 12.0, v.top() + NAVBAR_TOP);
        let items = [
            (
                "nav:select",
                "select",
                "Select",
                Action::Tool(NavTool::Select),
                Some(NavTool::Select),
            ),
            (
                "nav:orbit",
                "orbit",
                "Orbit",
                Action::Tool(NavTool::Orbit),
                Some(NavTool::Orbit),
            ),
            (
                "nav:pan",
                "pan",
                "Pan",
                Action::Tool(NavTool::Pan),
                Some(NavTool::Pan),
            ),
            (
                "nav:zoom",
                "zoom",
                "Zoom",
                Action::Tool(NavTool::Zoom),
                Some(NavTool::Zoom),
            ),
            ("nav:fit", "fit", "Fit All", Action::FitAll, None),
        ];
        let tool = self.view.tool;
        egui::Area::new(egui::Id::unique("navbar"))
            .fixed_pos(at)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let (bar, _) = ui.allocate_exact_size(vec2(NAVBAR_W, 5.0 * 30.0 + 2.0), Sense::hover());
                ui.painter().rect_filled(bar, 4.0, t.surface);
                ui.painter()
                    .rect_stroke(bar, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                for (k, (name, icon, tip, action, is)) in items.into_iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(bar.left() + 1.0, bar.top() + 1.0 + k as f32 * 30.0),
                        vec2(30.0, 30.0),
                    );
                    let on = is == Some(tool);
                    if on {
                        ui.painter().rect_filled(r.shrink(2.0), 3.0, t.on);
                    }
                    let ink = if on { t.on_ink } else { t.ink };
                    let resp = self.icon_button(ui, name, r, icon, tip, ink);
                    if resp.clicked() {
                        self.out.push(action);
                    }
                }
            });
    }

    /// With no title bar from the system, the window's edges resize it.
    /// The browser's right edge: drag to resize it, double-click for its default width.
    pub(super) fn side_splitter(&mut self, ui: &mut Ui, side: Rect, window_w: f32) {
        let r = Rect::from_min_max(
            pos2(side.right() - 3.0, side.top()),
            pos2(side.right() + 3.0, side.bottom()),
        );
        let resp = ui.interact(
            r,
            egui::Id::unique("side-splitter"),
            egui::Sense::click_and_drag(),
        );
        let live = resp.hovered() || resp.dragged();
        if live {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            ui.painter()
                .vline(side.right(), side.y_range(), Stroke::new(2.0, self.theme.accent));
        }
        if resp.dragged() {
            let max = (window_w * 0.6).max(SIDE_MIN_W);
            self.side_px = Some((side.width() + resp.drag_delta().x).clamp(SIDE_MIN_W, max));
        }
        if resp.double_clicked() {
            self.side_px = None;
        }
        self.view_blocks.push(r);
    }

    fn resize_edges(&mut self, ui: &mut Ui, s: Rect) {
        if self.view.maximized {
            return;
        }
        let Some(at) = ui.ctx().input(|i| i.pointer.hover_pos()) else {
            return;
        };
        let (l, r) = (at.x < RESIZE_EDGE, at.x > s.width() - RESIZE_EDGE);
        let (top, b) = (at.y < RESIZE_EDGE, at.y > s.height() - RESIZE_EDGE);
        let edge = match (top, b, l, r) {
            (true, _, true, _) => Edge::NW,
            (true, _, _, true) => Edge::NE,
            (_, true, true, _) => Edge::SW,
            (_, true, _, true) => Edge::SE,
            (true, ..) => Edge::N,
            (_, true, ..) => Edge::S,
            (_, _, true, _) => Edge::W,
            (_, _, _, true) => Edge::E,
            _ => return,
        };
        let icon = match edge {
            Edge::N | Edge::S => egui::CursorIcon::ResizeVertical,
            Edge::E | Edge::W => egui::CursorIcon::ResizeHorizontal,
            Edge::NE | Edge::SW => egui::CursorIcon::ResizeNeSw,
            Edge::NW | Edge::SE => egui::CursorIcon::ResizeNwSe,
        };
        ui.ctx().set_cursor_icon(icon);
        if ui.ctx().input(|i| i.pointer.primary_pressed()) {
            self.out.push(Action::ResizeWindow(edge));
        }
    }

    fn menus(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        self.marking_menu(ctx);
        if self.file_menu {
            let at = pos2(8.0, TITLE_H + TABS_H);
            let model_open = self.fg.is_some() && self.drawing.is_none();
            let mut items = vec![
                ("New", Some(Action::New)),
                ("New .fc Part...", Some(Action::NewFc)),
                ("New Sheet Metal", Some(Action::Sheet(sheet::SheetCmd::NewPart))),
                ("Open...", Some(Action::Open)),
            ];
            // a Forge file is written when asked (Ctrl+S too); an .fc part with every act
            items.push(("Save", model_open.then_some(Action::Fg(FgCmd::Save))));
            items.push(("New Drawing...", model_open.then_some(Action::NewDrawing)));
            if self.drawing.is_some() {
                items.push(("Close Drawing", Some(Action::CloseDrawing)));
            }
            if self.dwgdoc.is_some() {
                items.push(("Save", Some(Action::DwgDoc(dwgdoc::Cmd::Run("QSAVE")))));
                items.push(("Save As...", Some(Action::DwgDoc(dwgdoc::Cmd::Run("SAVEAS")))));
            }
            match menu_at(ctx, "file-menu", at, &t, &items, &mut self.places) {
                MenuResult::Chose(a) => {
                    self.out.push(a);
                    self.file_menu = false;
                }
                MenuResult::Close => self.file_menu = false,
                MenuResult::Open => {}
            }
        }
        if let Some(at) = self.cube_menu {
            let parallel = self.view.parallel;
            let region = self.cube_region;
            let mut items: Vec<(&str, Option<Action>)> = vec![("Go Home", Some(Action::Home))];
            if let Some(r) = region {
                items.push(("Set as Home", Some(Action::SetHomeAt(r))));
            }
            items.push(("Set Current View as Home", Some(Action::SetHome)));
            // a face can take a name
            if let Some(r) = region.filter(|r| r.iter().filter(|v| **v != 0).count() == 1) {
                const WORDS: [&str; 6] = [
                    "Set This Face As Front",
                    "Set This Face As Back",
                    "Set This Face As Top",
                    "Set This Face As Bottom",
                    "Set This Face As Left",
                    "Set This Face As Right",
                ];
                for (word, (_, n)) in WORDS.iter().zip(crate::viewcube::FACE_NAMES) {
                    items.push((word, Some(Action::SetFaceAs(r, n))));
                }
            }
            items.push(("Reset Orientation", Some(Action::ResetOrientation)));
            items.push((
                if parallel {
                    "\u{2713} Orthographic"
                } else {
                    "Orthographic"
                },
                Some(Action::Parallel(true)),
            ));
            items.push((
                if parallel {
                    "Perspective"
                } else {
                    "\u{2713} Perspective"
                },
                Some(Action::Parallel(false)),
            ));
            let fresh = std::mem::take(&mut self.cube_menu_fresh);
            match menu_at(ctx, "cube-menu", at, &t, &items, &mut self.places) {
                MenuResult::Chose(a) => {
                    self.out.push(a);
                    self.cube_menu = None;
                }
                MenuResult::Close if !fresh => self.cube_menu = None,
                MenuResult::Close | MenuResult::Open => {}
            }
        }
    }
}

impl Shell {
    pub fn start_rename(&mut self, id: ItemId) {
        let title = self.rows.iter().find(|r| r.id == id).map(|r| r.title.clone());
        if let Some(t) = title {
            self.renaming = Some((id, t));
        }
    }

    /// The marking menu: eight wedges round the place it opened, each its word and icon on a chip,
    /// chosen by direction; a small dead zone in the middle; what's left over in a list below.
    fn marking_menu(&mut self, ctx: &egui::Context) {
        let Some(c) = self.marking else { return };
        let t = self.theme;
        let (items, more) = self.marking_items();
        let mut chose = None;
        let mut closing = false;
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let wedge = pointer.and_then(|p| {
            let d = p - c;
            (d.length() >= DEAD_PT && d.length() <= WEDGE_FAR_PT).then(|| {
                let a = d.x.atan2(-d.y).rem_euclid(core::f32::consts::TAU);
                ((a / core::f32::consts::FRAC_PI_4).round() as usize) % 8
            })
        });
        let wedge = wedge.filter(|k| items[*k].2.is_some());
        let placed_before = self.places.len();
        let (_, sizing) = egui::Area::new(egui::Id::unique("marking"))
            .fixed_pos(pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show_sizing(ctx, |ui| {
                let p = ui.painter();
                let around = |a: f32, r: f32| c + vec2(a.sin() * r, -a.cos() * r);
                // the ring, and the wedge the pointer's in
                let q = core::f32::consts::FRAC_PI_8;
                if let Some(k) = wedge {
                    let mid = k as f32 * core::f32::consts::FRAC_PI_4;
                    let mut mesh = egui::Mesh::default();
                    let ink = t.focus.gamma_multiply(0.75);
                    for s in 0..8 {
                        let (a0, a1) = (
                            mid - q + 2.0 * q * s as f32 / 8.0,
                            mid - q + 2.0 * q * (s + 1) as f32 / 8.0,
                        );
                        let base = mesh.vertices.len() as u32;
                        for pt in [
                            around(a0, DEAD_PT),
                            around(a1, DEAD_PT),
                            around(a1, RING_PT),
                            around(a0, RING_PT),
                        ] {
                            mesh.colored_vertex(pt, ink);
                        }
                        mesh.add_triangle(base, base + 1, base + 2);
                        mesh.add_triangle(base, base + 2, base + 3);
                    }
                    p.add(mesh);
                }
                p.circle_stroke(c, RING_PT, Stroke::new(1.0, t.ink2.gamma_multiply(0.5)));
                p.circle_filled(c, DEAD_PT, t.surface);
                p.circle_stroke(c, DEAD_PT, Stroke::new(1.5, t.ink2));
                for (k, (label, icon, action)) in items.iter().enumerate() {
                    if label.is_empty() {
                        continue;
                    }
                    let a = k as f32 * core::f32::consts::FRAC_PI_4;
                    let g = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), sans(12.0), t.ink));
                    let w = g.size().x + 34.0;
                    let at = around(a, RING_PT + 8.0);
                    // standing out from the ring on its own side
                    let (sx, sy) = (a.sin(), -a.cos());
                    // top and bottom stand clear of their neighbours
                    let lift = if sx.abs() < 0.1 { 24.0 } else { 8.0 };
                    let mid = at + vec2(sx * w * 0.5, sy * lift);
                    let r = Rect::from_center_size(mid, vec2(w, 28.0));
                    let on = wedge == Some(k);
                    let p = ui.painter();
                    p.rect_filled(r, 4.0, if on { t.on } else { t.surface });
                    p.rect_stroke(
                        r,
                        4.0,
                        Stroke::new(1.0, if on { t.focus } else { t.line }),
                        StrokeKind::Inside,
                    );
                    let ink = if action.is_none() {
                        t.ink2.gamma_multiply(0.55)
                    } else if on {
                        t.on_ink
                    } else {
                        t.ink
                    };
                    icons::draw(p, r.left_top() + vec2(7.0, 6.0), 16.0, icon, 1.5, ink);
                    p.text(
                        pos2(r.left() + 28.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        sans(12.0),
                        ink,
                    );
                    self.places.push((format!("mark:{label}"), r));
                }
                // the rest
                let mut y = c.y + RING_PT + 60.0;
                for (label, icon, action) in &more {
                    let r = Rect::from_min_size(pos2(c.x - 80.0, y), vec2(160.0, 26.0));
                    let resp = ui.interact(r, ui.scope_id().with(*label), Sense::click());
                    let p = ui.painter();
                    p.rect_filled(r, 0.0, if resp.hovered() { t.on } else { t.surface });
                    p.rect_stroke(r, 0.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                    let ink = if action.is_none() { t.ink2 } else { t.ink };
                    icons::draw(p, r.left_top() + vec2(7.0, 5.0), 16.0, icon, 1.5, ink);
                    p.text(
                        pos2(r.left() + 28.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        sans(12.0),
                        ink,
                    );
                    self.places.push((format!("mark:{label}"), r));
                    if resp.clicked() {
                        chose = *action;
                        closing = true;
                    }
                    y += 26.0;
                }
            });
        // what it named unseen, sizing itself, is off
        if sizing {
            self.places.truncate(placed_before);
        }
        let clicked = ctx.input(|i| i.pointer.primary_clicked());
        if clicked && !closing {
            match wedge {
                Some(k) => chose = items[k].2,
                None => closing = true,
            }
        }
        if let Some(a) = chose {
            self.out.push(a);
            closing = true;
        }
        if closing {
            self.marking = None;
            self.marking_row = None;
        }
    }

    /// A selection window: solid when it takes what's wholly in it, dashed when what it crosses too.
    fn window_ui(&self, ctx: &egui::Context) {
        let Some((a, b)) = self.window_box else { return };
        let t = self.theme;
        let p = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::unique("window"),
            ))
            .with_clip_rect(self.view_rect);
        let r = Rect::from_two_pos(a, b);
        p.rect_filled(r, 0.0, t.focus.gamma_multiply(0.12));
        if b.x >= a.x {
            p.rect_stroke(r, 0.0, Stroke::new(1.5, t.focus), StrokeKind::Inside);
        } else {
            let corners = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            p.extend(egui::Shape::dashed_line(
                &corners,
                Stroke::new(1.5, t.focus),
                6.0,
                4.0,
            ));
        }
    }
}

enum MenuResult {
    Open,
    Close,
    Chose(Action),
}

/// A flat menu at a point; closes on Esc or a press outside it.
fn menu_at(
    ctx: &egui::Context,
    id: &str,
    at: egui::Pos2,
    t: &Theme,
    items: &[(&str, Option<Action>)],
    places: &mut Vec<(String, Rect)>,
) -> MenuResult {
    let mut out = MenuResult::Open;
    let w = 220.0;
    let inner = egui::Area::new(egui::Id::unique(id))
        .fixed_pos(at)
        .order(egui::Order::Foreground)
        .show_placed(ctx, places, |ui, places| {
            let (panel, _) =
                ui.allocate_exact_size(vec2(w, 8.0 + items.len() as f32 * ROW_H), Sense::click());
            ui.painter().rect_filled(panel, 4.0, t.surface);
            ui.painter()
                .rect_stroke(panel, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            for (k, (label, action)) in items.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(panel.left() + 1.0, panel.top() + 4.0 + k as f32 * ROW_H),
                    vec2(w - 2.0, ROW_H),
                );
                let sense = if action.is_some() {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let resp = ui.interact(r, ui.scope_id().with(k), sense);
                places.push((format!("{id}:{label}"), r));
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, action.is_some(), *label));
                if (resp.hovered() || resp.has_focus()) && action.is_some() {
                    ui.painter().rect_filled(r, 0.0, t.on);
                }
                let ink = if action.is_none() {
                    t.ink2
                } else if resp.hovered() || resp.has_focus() {
                    t.on_ink
                } else {
                    t.ink
                };
                ui.painter().text(
                    pos2(r.left() + 12.0, r.center().y),
                    Align2::LEFT_CENTER,
                    *label,
                    sans(13.0),
                    ink,
                );
                if resp.clicked()
                    && let Some(a) = action
                {
                    out = MenuResult::Chose(*a);
                }
            }
        });
    let outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !inner.response.rect.contains(p))
    });
    if matches!(out, MenuResult::Open) && (outside || ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
        out = MenuResult::Close;
    }
    out
}
