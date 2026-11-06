//! A DWG file in a tab of its own: a generic 2D drawing (a floor plan, a site plan), not one of
//! ForgeCAD's parts, seen AutoCAD's way. The file as read (`forge_dwg::file::Drawing`), the
//! space shown as a scene on the GPU (`crate::dwgview`), its layers' switches, the selection,
//! the command line and its history. Every command, from a ribbon button or typed, goes through
//! `run_command`; the editing ones say "not yet" for now. The window's layout is in
//! `dwgdoc_ui.rs`.

use crate::dwgview::Geom;
use crate::dwgview::gpu::Words;
use forge_dwg::file::Drawing;
use forge_dwg::io::{Color, Href};
use forge_dwg::scene::{BoxFont, Options, Scene, Space};
use forge_dwg::types::Data;
use forge_dwg::version::Version;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// The ribbon's tabs, AutoCAD's.
pub const TABS: [&str; 6] = ["Home", "Insert", "Annotate", "View", "Manage", "Output"];

/// What the ribbon and the window ask of the DWG document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// A command by its name, as the command line takes it.
    Run(&'static str),
}

/// A command: its name, its aliases, and whether it does anything yet.
pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub works: bool,
}

const fn c(name: &'static str, aliases: &'static [&'static str], works: bool) -> Command {
    Command { name, aliases, works }
}

/// AutoCAD's commands and their aliases, as acad.pgp has them.
pub const COMMANDS: &[Command] = &[
    c("LINE", &["L"], false),
    c("PLINE", &["PL"], false),
    c("CIRCLE", &["C"], false),
    c("ARC", &["A"], false),
    c("RECTANG", &["REC", "RECTANGLE"], false),
    c("POLYGON", &["POL"], false),
    c("ELLIPSE", &["EL"], false),
    c("SPLINE", &["SPL"], false),
    c("HATCH", &["H", "BH"], false),
    c("TEXT", &["T", "DT", "DTEXT"], false),
    c("MTEXT", &["MT"], false),
    c("DIM", &[], false),
    c("DIMLINEAR", &["DLI"], false),
    c("DIMALIGNED", &["DAL"], false),
    c("DIMANGULAR", &["DAN"], false),
    c("DIMRADIUS", &["DRA"], false),
    c("DIMDIAMETER", &["DDI"], false),
    c("LEADER", &["LE", "MLEADER", "MLD"], false),
    c("TABLE", &["TB"], false),
    c("MOVE", &["M"], false),
    c("COPY", &["CO", "CP"], false),
    c("ROTATE", &["RO"], false),
    c("MIRROR", &["MI"], false),
    c("SCALE", &["SC"], false),
    c("STRETCH", &["S"], false),
    c("TRIM", &["TR"], false),
    c("EXTEND", &["EX"], false),
    c("OFFSET", &["O"], false),
    c("FILLET", &["F"], false),
    c("CHAMFER", &["CHA"], false),
    c("ARRAY", &["AR"], false),
    c("ERASE", &["E"], false),
    c("EXPLODE", &["X"], false),
    c("MATCHPROP", &["MA"], false),
    c("INSERT", &["I"], false),
    c("BLOCK", &["B"], false),
    c("BEDIT", &["BE"], false),
    c("XATTACH", &["XA"], false),
    c("CLIP", &[], false),
    c("MEASUREGEOM", &["MEA", "DIST", "DI"], false),
    c("PURGE", &["PU"], false),
    c("PLOT", &["PRINT"], false),
    c("EXPORTPDF", &[], false),
    c("LAYER", &["LA"], true),
    c("PROPERTIES", &["PR", "CH", "MO"], true),
    c("ZOOM", &["Z"], true),
    c("PAN", &["P"], true),
    c("REGEN", &["RE"], true),
    c("LWDISPLAY", &["LWT"], true),
    c("QSAVE", &["SAVE"], true),
    c("SAVEAS", &[], true),
    c("OPEN", &[], true),
    c("AUDIT", &[], true),
];

/// The command a typed word names, by its name or an alias.
pub fn command_of(word: &str) -> Option<&'static Command> {
    let w = word.trim().trim_start_matches(['_', '.']).to_ascii_uppercase();
    COMMANDS
        .iter()
        .find(|c| c.name == w || c.aliases.contains(&w.as_str()))
}

/// The status bar's toggles, AutoCAD's: only LWT works yet.
pub const TOGGLES: [(&str, &str); 6] = [
    ("SNAP", "Snap mode (F9)"),
    ("GRID", "Grid display (F7)"),
    ("ORTHO", "Ortho mode (F8)"),
    ("POLAR", "Polar tracking (F10)"),
    ("OSNAP", "Object snap (F3)"),
    ("LWT", "Show/hide lineweight"),
];

/// A layout's tab: its name, its LAYOUT object, its sheet (lo, hi) in paper units if it has one.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutTab {
    pub name: String,
    pub handle: u32,
    pub paper: Option<([f64; 2], [f64; 2])>,
}

/// Where the canvas looks: the world point at its centre and points a unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub center: [f64; 2],
    pub scale: f64,
}

/// The pointer's press on the canvas, for a click or a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Press {
    pub at: egui::Pos2,
    /// It moved far enough to be a window.
    pub window: bool,
}

pub struct DwgDoc {
    /// Its file; none for one opened from bytes with no path (the browser's).
    pub path: Option<String>,
    pub name: String,
    pub drawing: Drawing,
    /// Model first, then each layout in tab order.
    pub layouts: Vec<LayoutTab>,
    /// Which: 0 Model, k the layout `layouts[k - 1]`.
    pub space: usize,
    pub geom: Arc<Geom>,
    /// How each space was last seen.
    views: HashMap<usize, View>,
    pub view: Option<View>,
    /// A layer's on and frozen, by its handle: the viewer's own, not written to the file.
    pub layer_state: HashMap<u32, (bool, bool)>,
    layer_words: Words,
    pub selected: BTreeSet<u32>,
    pub hover: Option<u32>,
    flags: Vec<u32>,
    flags_words: Words,
    /// LWDISPLAY.
    pub lwt: bool,
    /// The command line: what it said, what's typed, what was typed before.
    pub history: Vec<String>,
    pub input: String,
    pub typed: Vec<String>,
    pub recall: Option<usize>,
    /// A command waiting for its option (ZOOM's).
    pub pending: Option<&'static str>,
    pub last_command: Option<&'static str>,
    /// The pointer on the canvas, in world units.
    pub cursor: Option<[f64; 2]>,
    pub press: Option<Press>,
    /// Space held: the left button pans.
    pub space_pan: bool,
    pub show_layers: bool,
    pub show_props: bool,
    pub show_warnings: bool,
    /// The saved file's bytes for the host to hand over (the browser's download).
    pub download: Option<(String, Vec<u8>)>,
    /// A rebuild of the scene is wanted (a space switched, REGEN).
    pub fit_next: bool,
}

/// The file name from a path.
#[cfg(not(target_arch = "wasm32"))]
fn base_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn version_name(v: Version) -> &'static str {
    match v {
        Version::R2000 => "AutoCAD 2000 (AC1015)",
        Version::R2004 => "AutoCAD 2004 (AC1018)",
        Version::R2007 => "AutoCAD 2007 (AC1021)",
        Version::R2010 => "AutoCAD 2010 (AC1024)",
        Version::R2013 => "AutoCAD 2013 (AC1027)",
        Version::R2018 => "AutoCAD 2018 (AC1032)",
    }
}

/// The layouts, by tab order, the model's apart.
fn layouts_of(d: &Drawing) -> Vec<LayoutTab> {
    let ms = d.header.handle("BLOCK_RECORD_MODEL_SPACE");
    let mut tabs: Vec<(i32, LayoutTab)> = vec![];
    for (h, o) in &d.objects {
        let Data::Layout(l) = &o.data else { continue };
        let name = l.name.text();
        if Some(l.block.handle) == ms || name.eq_ignore_ascii_case("Model") {
            continue;
        }
        let ok = |lo: [f64; 2], hi: [f64; 2]| {
            (lo.iter().chain(&hi).all(|v| v.is_finite()) && hi[0] > lo[0] && hi[1] > lo[1])
                .then_some((lo, hi))
        };
        // the limits, else the plot's paper less nothing, turned as it plots
        let paper = ok(l.limmin, l.limmax).or_else(|| {
            let [w, h] = l.plot.paper;
            let (w, h) = if l.plot.rotation % 2 == 1 { (h, w) } else { (w, h) };
            let k = if l.plot.paper_units == 0 { 1.0 / 25.4 } else { 1.0 };
            ok([0.0, 0.0], [w * k, h * k])
        });
        tabs.push((
            l.tab_order,
            LayoutTab {
                name,
                handle: *h,
                paper,
            },
        ));
    }
    tabs.sort_by_key(|t| t.0);
    tabs.into_iter().map(|t| t.1).collect()
}

/// A table entry's name: a layer's, a line type's, a block's, a text style's.
pub fn entry_name(d: &Drawing, h: Href) -> Option<String> {
    Some(match &d.objects.get(&h.handle)?.data {
        Data::Layer(l) => l.entry.name.text(),
        Data::Ltype(l) => l.entry.name.text(),
        Data::BlockHeader(b) => b.entry.name.text(),
        Data::Style(s) => s.entry.name.text(),
        _ => return None,
    })
}

/// AutoCAD's names for colours 1..7.
pub fn colour_name(c: &Color) -> String {
    if let Some(rgb) = c.rgb.filter(|v| v >> 24 == 0xC2) {
        return format!("{},{},{}", (rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255);
    }
    match c.index {
        256 => "ByLayer".into(),
        0 => "ByBlock".into(),
        1 => "Red".into(),
        2 => "Yellow".into(),
        3 => "Green".into(),
        4 => "Cyan".into(),
        5 => "Blue".into(),
        6 => "Magenta".into(),
        7 => "White".into(),
        i => format!("{}", i.abs()),
    }
}

/// A lineweight index as stored (0..23, 29 bylayer, 30 byblock, 31 default) in words.
pub fn lineweight_name(i: u8) -> String {
    const MM100: [u16; 24] = [
        0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158, 200, 211,
    ];
    match i {
        29 => "ByLayer".into(),
        30 => "ByBlock".into(),
        31 => "Default".into(),
        i => MM100
            .get(i as usize)
            .map_or_else(|| format!("{i}"), |w| format!("{:.2} mm", *w as f64 / 100.0)),
    }
}

/// A number as AutoCAD's Properties shows it.
fn num(v: f64) -> String {
    format!("{v:.4}")
}

fn deg(rad: f64) -> String {
    let mut d = rad.to_degrees() % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    format!("{d:.0}\u{b0}").replace("360\u{b0}", "0\u{b0}")
}

/// An entity's type as AutoCAD names it.
pub fn type_name(data: &Data) -> String {
    match data {
        Data::Line(_) => "Line".into(),
        Data::Circle(_) => "Circle".into(),
        Data::Arc(_) => "Arc".into(),
        Data::LwPolyline(_) => "Polyline".into(),
        Data::Polyline2d(_) => "2D Polyline".into(),
        Data::Polyline3d(_) => "3D Polyline".into(),
        Data::Text(_) => "Text".into(),
        Data::MText(_) => "MText".into(),
        Data::Insert(_) => "Block Reference".into(),
        Data::Hatch(_) => "Hatch".into(),
        Data::Solid(_) => "2D Solid".into(),
        Data::Point(_) => "Point".into(),
        Data::Ellipse(_) => "Ellipse".into(),
        Data::Spline(_) => "Spline".into(),
        Data::Viewport(_) => "Viewport".into(),
        Data::DimLinear(_) => "Rotated Dimension".into(),
        Data::DimAligned(_) => "Aligned Dimension".into(),
        Data::DimRadius(_) => "Radial Dimension".into(),
        Data::DimDiameter(_) => "Diametric Dimension".into(),
        Data::DimOrdinate(_) => "Ordinate Dimension".into(),
        Data::DimAng2Ln(_) | Data::DimAng3Pt(_) => "Angular Dimension".into(),
        Data::Leader(_) => "Leader".into(),
        Data::MLeader(_) => "Multileader".into(),
        Data::Attrib(_) => "Attribute".into(),
        Data::AttDef(_) => "Attribute Definition".into(),
        Data::Raw(t) => format!("Object {t}"),
        d => d.name(),
    }
}

/// An object's own fields as Debug shows them, the top level only, scalars and short lists: the
/// fallback for a type with no hand-made rows.
fn debug_rows(data: &Data) -> Vec<(String, String)> {
    let text = format!("{data:#?}");
    let mut rows = vec![];
    for line in text.lines().skip(2) {
        // `        field: value,` at the struct's own depth
        let Some(rest) = line.strip_prefix("        ") else {
            continue;
        };
        if rest.starts_with(' ') {
            continue;
        }
        let Some((k, v)) = rest.split_once(": ") else {
            continue;
        };
        let v = v.trim_end_matches(',');
        if v.ends_with('{') || v.ends_with('[') || v.ends_with('(') {
            continue;
        }
        rows.push((k.replace('_', " "), v.chars().take(60).collect()));
    }
    rows
}

fn xyz(p: [f64; 3]) -> [(String, String); 3] {
    [
        ("X".into(), num(p[0])),
        ("Y".into(), num(p[1])),
        ("Z".into(), num(p[2])),
    ]
}

/// One property row: its group, its name, its value.
pub type Prop = (&'static str, String, String);

impl DwgDoc {
    /// A DWG file's bytes as a document; `path` where it's saved back to.
    pub fn from_bytes(path: Option<String>, name: &str, bytes: &[u8]) -> Result<DwgDoc, String> {
        let t0 = crate::viewer::now_ms();
        let drawing = forge_dwg::file::read(bytes).map_err(|e| format!("{name}: {}", e.0))?;
        let layouts = layouts_of(&drawing);
        let scene = Arc::new(Scene::default());
        let mut d = DwgDoc {
            path,
            name: name.to_string(),
            drawing,
            layouts,
            space: 0,
            geom: Arc::new(Geom::new(scene)),
            views: HashMap::new(),
            view: None,
            layer_state: HashMap::new(),
            layer_words: Words {
                words: Arc::new(vec![]),
                version: 0,
                since: None,
            },
            selected: BTreeSet::new(),
            hover: None,
            flags: vec![],
            flags_words: Words {
                words: Arc::new(vec![]),
                version: 0,
                since: None,
            },
            lwt: false,
            history: vec![],
            input: String::new(),
            typed: vec![],
            recall: None,
            pending: None,
            last_command: None,
            cursor: None,
            press: None,
            space_pan: false,
            show_layers: true,
            show_props: true,
            show_warnings: false,
            download: None,
            fit_next: true,
        };
        d.rebuild();
        let v = d.drawing.ver();
        let items = d.geom.n_items();
        let ms = crate::viewer::now_ms() - t0;
        d.say(format!(
            "Opened {name}: {}, {items} items in {ms:.0} ms",
            version_name(v)
        ));
        let n = d.drawing.warnings.len();
        if n > 0 {
            d.say(format!(
                "{n} object{} kept as read (not decoded)",
                if n == 1 { "" } else { "s" }
            ));
        }
        Ok(d)
    }

    /// A DWG file from disk.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(path: &str) -> Result<DwgDoc, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        DwgDoc::from_bytes(Some(path.to_string()), &base_name(path), &bytes)
    }

    /// The space's tab names: Model, then the layouts.
    pub fn tab_names(&self) -> Vec<String> {
        std::iter::once("Model".to_string())
            .chain(self.layouts.iter().map(|l| l.name.clone()))
            .collect()
    }

    pub fn space_of(&self, k: usize) -> Space {
        match k.checked_sub(1).and_then(|i| self.layouts.get(i)) {
            Some(l) => Space::Layout(l.handle),
            None => Space::Model,
        }
    }

    pub fn is_paper(&self) -> bool {
        self.space > 0
    }

    pub fn paper(&self) -> Option<([f64; 2], [f64; 2])> {
        self.layouts.get(self.space.checked_sub(1)?)?.paper
    }

    /// The scene of the space shown made again, and drawn from it.
    pub fn rebuild(&mut self) {
        let scene = forge_dwg::scene::build(
            &self.drawing,
            self.space_of(self.space),
            &BoxFont,
            &Options::default(),
        );
        // a layer met for the first time takes the file's switches
        for l in &scene.layers {
            self.layer_state.entry(l.handle).or_insert((l.on, l.frozen));
        }
        self.geom = Arc::new(Geom::new(Arc::new(scene)));
        self.flags = vec![0; self.geom.n_items()];
        self.hover = None;
        self.selected.retain(|e| self.geom.by_entity.contains_key(e));
        for e in self.selected.clone() {
            self.mark(e, 1, true);
        }
        self.flags_words = Words {
            words: Arc::new(self.flags.clone()),
            version: self.flags_words.version + 1,
            since: None,
        };
        self.layers_changed();
    }

    /// Another space: its scene, and its view as left (fitted the first time).
    pub fn switch_space(&mut self, k: usize) {
        if k == self.space || k > self.layouts.len() {
            return;
        }
        if let Some(v) = self.view {
            self.views.insert(self.space, v);
        }
        self.space = k;
        self.selected.clear();
        self.rebuild();
        self.view = self.views.get(&k).copied();
        self.fit_next = self.view.is_none();
        let name = self.tab_names()[k].clone();
        self.say(format!("Regenerating {name}."));
    }

    /// Is the scene's layer `i` shown: on and not frozen?
    pub fn layer_shown(&self, i: usize) -> bool {
        self.geom
            .scene
            .layers
            .get(i)
            .and_then(|l| self.layer_state.get(&l.handle))
            .is_none_or(|&(on, frozen)| on && !frozen)
    }

    fn layers_changed(&mut self) {
        let words: Vec<u32> = (0..self.geom.scene.layers.len().max(1))
            .map(|i| self.layer_shown(i) as u32)
            .collect();
        self.layer_words = Words {
            words: Arc::new(words),
            version: self.layer_words.version + 1,
            since: None,
        };
        // what's on a layer turned off can't stay chosen
        let gone: Vec<u32> = self
            .selected
            .iter()
            .copied()
            .filter(|e| {
                self.geom.by_entity[e]
                    .iter()
                    .all(|&i| !self.layer_shown(self.geom.scene.items[i as usize].layer))
            })
            .collect();
        for e in gone {
            self.selected.remove(&e);
            self.mark(e, 1, false);
        }
        self.flags_changed(0, self.flags.len() as u32);
    }

    /// A layer's on or frozen switch flipped, by the scene's index.
    pub fn flip_layer(&mut self, i: usize, frozen: bool) {
        let Some(l) = self.geom.scene.layers.get(i) else {
            return;
        };
        let s = self.layer_state.entry(l.handle).or_insert((l.on, l.frozen));
        if frozen {
            s.1 = !s.1;
        } else {
            s.0 = !s.0;
        }
        self.layers_changed();
    }

    fn mark(&mut self, entity: u32, bit: u32, on: bool) -> Option<(u32, u32)> {
        let items = self.geom.by_entity.get(&entity)?;
        let (mut lo, mut hi) = (u32::MAX, 0);
        for &i in items {
            let f = &mut self.flags[i as usize];
            *f = if on { *f | bit } else { *f & !bit };
            lo = lo.min(i);
            hi = hi.max(i + 1);
        }
        Some((lo, hi))
    }

    /// The items' words from lo to hi changed: a new version for the GPU, with the range that
    /// differs from the version it last drew.
    fn flags_changed(&mut self, lo: u32, hi: u32) {
        let since = match self.flags_words.since {
            Some((from, a, b)) => (from, a.min(lo), b.max(hi)),
            None => (self.flags_words.version, lo, hi),
        };
        self.flags_words = Words {
            words: Arc::new(self.flags.clone()),
            version: self.flags_words.version + 1,
            since: Some(since),
        };
    }

    /// What to draw: the layers' and the items' words, the range since the last draw's.
    pub fn words(&mut self) -> (Words, Words) {
        let out = (self.layer_words.clone(), self.flags_words.clone());
        // the next change is from this version
        self.flags_words.since = None;
        out
    }

    fn set_flags(&mut self, entity: u32, bit: u32, on: bool) {
        if let Some((lo, hi)) = self.mark(entity, bit, on) {
            self.flags_changed(lo, hi);
        }
    }

    pub fn set_hover(&mut self, e: Option<u32>) {
        if e == self.hover {
            return;
        }
        if let Some(old) = self.hover.take() {
            self.set_flags(old, 2, false);
        }
        if let Some(new) = e {
            self.set_flags(new, 2, true);
        }
        self.hover = e;
    }

    /// Chosen, or not.
    pub fn select(&mut self, entities: &[u32], add: bool) {
        for &e in entities {
            let changed = if add {
                self.selected.insert(e)
            } else {
                self.selected.remove(&e)
            };
            if changed {
                self.set_flags(e, 1, add);
            }
        }
    }

    pub fn clear_selection(&mut self) {
        let all: Vec<u32> = self.selected.iter().copied().collect();
        self.select(&all, false);
    }

    /// Everything shown in view: the space's extents, a little in from the canvas's edges.
    pub fn fit(&mut self, canvas: egui::Vec2) {
        let shown = |i: usize| self.layer_shown(i);
        let ext = self.geom.shown_extents(&shown);
        let ext = match (self.paper(), ext) {
            (Some((plo, phi)), Some((lo, hi))) => Some((
                [plo[0].min(lo[0]), plo[1].min(lo[1])],
                [phi[0].max(hi[0]), phi[1].max(hi[1])],
            )),
            (Some(p), None) => Some(p),
            (None, e) => e,
        };
        let Some((lo, hi)) = ext else {
            self.view = Some(View {
                center: [0.0, 0.0],
                scale: 1.0,
            });
            return;
        };
        let w = (hi[0] - lo[0]).max(1e-9);
        let h = (hi[1] - lo[1]).max(1e-9);
        let scale = ((canvas.x as f64 * 0.94) / w).min((canvas.y as f64 * 0.94) / h);
        self.view = Some(View {
            center: [0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1])],
            scale: if scale.is_finite() && scale > 0.0 {
                scale
            } else {
                1.0
            },
        });
    }

    /// The command line's echo, kept to the last hundred lines.
    pub fn say(&mut self, s: impl Into<String>) {
        self.history.push(s.into());
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    /// The file's bytes in its own version, or why not.
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        forge_dwg::file::write(&self.drawing).map_err(|e| e.0)
    }

    /// Writes the file where it came from (natively), or hands the host its bytes to save
    /// (the browser).
    pub fn save(&mut self) {
        let v = version_name(self.drawing.ver());
        let bytes = match self.bytes() {
            Ok(b) => b,
            Err(e) => {
                self.say(format!("Can't save {}: {e}", self.name));
                return;
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(p) = self.path.clone() {
            match std::fs::write(&p, &bytes) {
                Ok(()) => self.say(format!("Saved {} as {v}", self.name)),
                Err(e) => self.say(format!("Can't save {p}: {e}")),
            }
            return;
        }
        self.download = Some((self.name.clone(), bytes));
        self.say(format!("Saved {} as {v}", self.name));
    }

    /// Writes it to a new path, which it's saved to from now on.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as(&mut self, path: &str) {
        let path = if path.to_ascii_lowercase().ends_with(".dwg") {
            path.to_string()
        } else {
            format!("{path}.dwg")
        };
        self.path = Some(path.clone());
        self.name = base_name(&path);
        self.save();
    }

    /// What the Properties palette shows for the selection: one entity's rows, or how many of
    /// each type.
    pub fn properties(&self) -> Vec<Prop> {
        match self.selected.len() {
            0 => self.drawing_props(),
            1 => self.entity_props(*self.selected.first().expect("one")),
            n => {
                let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
                for h in &self.selected {
                    if let Some(o) = self.drawing.objects.get(h) {
                        *kinds.entry(type_name(&o.data)).or_default() += 1;
                    }
                }
                let mut rows = vec![("General", "All".to_string(), format!("{n}"))];
                rows.extend(kinds.into_iter().map(|(k, n)| ("General", k, n.to_string())));
                rows
            }
        }
    }

    fn drawing_props(&self) -> Vec<Prop> {
        let d = &self.drawing;
        let s = &self.geom.scene;
        let mut rows = vec![
            ("Drawing", "File".into(), self.name.clone()),
            ("Drawing", "Version".into(), version_name(d.ver()).to_string()),
            ("Drawing", "Objects".into(), d.objects.len().to_string()),
            ("Drawing", "Layers".into(), s.layers.len().to_string()),
            ("Drawing", "Space".into(), self.tab_names()[self.space].clone()),
            ("Drawing", "Items drawn".into(), s.items.len().to_string()),
        ];
        if let Some((lo, hi)) = s.extents {
            rows.push((
                "Drawing",
                "Extents min".into(),
                format!("{}, {}", num(lo[0]), num(lo[1])),
            ));
            rows.push((
                "Drawing",
                "Extents max".into(),
                format!("{}, {}", num(hi[0]), num(hi[1])),
            ));
        }
        rows.push(("Drawing", "Not decoded".into(), d.warnings.len().to_string()));
        rows
    }

    /// An entity's rows: AutoCAD's General group, then its geometry.
    pub fn entity_props(&self, h: u32) -> Vec<Prop> {
        let d = &self.drawing;
        let Some(o) = d.objects.get(&h) else {
            return vec![("General", "Handle".into(), format!("{h:X}"))];
        };
        let mut rows: Vec<Prop> = vec![
            ("General", "Type".into(), type_name(&o.data)),
            ("General", "Handle".into(), format!("{h:X}")),
        ];
        if let Some(e) = &o.ent {
            rows.push(("General", "Color".into(), colour_name(&e.color)));
            rows.push((
                "General",
                "Layer".into(),
                entry_name(d, e.layer).unwrap_or_else(|| "0".into()),
            ));
            let lt = match e.ltype_flags {
                0 => "ByLayer".into(),
                1 => "ByBlock".into(),
                2 => "Continuous".into(),
                _ => entry_name(d, e.ltype).unwrap_or_else(|| "?".into()),
            };
            rows.push(("General", "Linetype".into(), lt));
            rows.push(("General", "Linetype scale".into(), num(e.ltype_scale)));
            rows.push(("General", "Lineweight".into(), lineweight_name(e.lineweight)));
            if e.invisible != 0 {
                rows.push(("General", "Visible".into(), "No".into()));
            }
        }
        let mut geo = |k: &str, v: String| rows.push(("Geometry", k.to_string(), v));
        match &o.data {
            Data::Line(l) => {
                for ((_, v), w) in xyz(l.start).into_iter().zip(["Start X", "Start Y", "Start Z"]) {
                    geo(w, v);
                }
                for ((_, v), w) in xyz(l.end).into_iter().zip(["End X", "End Y", "End Z"]) {
                    geo(w, v);
                }
                let (dx, dy) = (l.end[0] - l.start[0], l.end[1] - l.start[1]);
                geo("Delta X", num(dx));
                geo("Delta Y", num(dy));
                geo("Length", num((dx * dx + dy * dy).sqrt()));
                geo("Angle", deg(forge_math::dmath::atan2(dy, dx)));
            }
            Data::Circle(c) => {
                for ((_, v), w) in xyz(c.center)
                    .into_iter()
                    .zip(["Center X", "Center Y", "Center Z"])
                {
                    geo(w, v);
                }
                geo("Radius", num(c.radius));
                geo("Diameter", num(2.0 * c.radius));
                geo("Circumference", num(std::f64::consts::TAU * c.radius));
                geo("Area", num(std::f64::consts::PI * c.radius * c.radius));
            }
            Data::Arc(a) => {
                for ((_, v), w) in xyz(a.circle.center)
                    .into_iter()
                    .zip(["Center X", "Center Y", "Center Z"])
                {
                    geo(w, v);
                }
                geo("Radius", num(a.circle.radius));
                geo("Start angle", deg(a.start));
                geo("End angle", deg(a.end));
                let mut sweep = a.end - a.start;
                while sweep <= 0.0 {
                    sweep += std::f64::consts::TAU;
                }
                geo("Total angle", format!("{:.0}\u{b0}", sweep.to_degrees()));
                geo("Arc length", num(sweep * a.circle.radius));
            }
            Data::LwPolyline(p) => {
                geo("Vertices", p.points.len().to_string());
                if let Some(q) = p.points.first() {
                    geo("Vertex X", num(q[0]));
                    geo("Vertex Y", num(q[1]));
                }
                geo("Global width", num(p.const_width));
                geo("Elevation", num(p.elevation));
                geo("Closed", if p.flag & 512 != 0 { "Yes" } else { "No" }.into());
                let n = p.points.len();
                let segs = if p.flag & 512 != 0 { n } else { n.saturating_sub(1) };
                let len: f64 = (0..segs)
                    .map(|k| {
                        let (a, b) = (p.points[k], p.points[(k + 1) % n]);
                        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
                    })
                    .sum();
                geo("Length (chords)", num(len));
            }
            Data::Text(t) => {
                geo("Contents", t.text.text());
                geo(
                    "Style",
                    entry_name(d, t.style).unwrap_or_else(|| "Standard".into()),
                );
                geo("Height", num(t.height));
                geo("Rotation", deg(t.rotation));
                geo("Width factor", num(t.width));
                geo("Position X", num(t.insert[0]));
                geo("Position Y", num(t.insert[1]));
            }
            Data::MText(t) => {
                geo("Contents", t.text.text().chars().take(120).collect());
                geo(
                    "Style",
                    entry_name(d, t.style).unwrap_or_else(|| "Standard".into()),
                );
                geo("Text height", num(t.text_height));
                geo("Defined width", num(t.rect_width));
                for ((_, v), w) in xyz(t.insert)
                    .into_iter()
                    .zip(["Position X", "Position Y", "Position Z"])
                {
                    geo(w, v);
                }
            }
            Data::Insert(i) => {
                geo("Name", entry_name(d, i.block).unwrap_or_else(|| "?".into()));
                for ((_, v), w) in xyz(i.at)
                    .into_iter()
                    .zip(["Position X", "Position Y", "Position Z"])
                {
                    geo(w, v);
                }
                geo("Scale X", num(i.scale[0]));
                geo("Scale Y", num(i.scale[1]));
                geo("Scale Z", num(i.scale[2]));
                geo("Rotation", deg(i.rotation));
            }
            data => {
                for (k, v) in debug_rows(data) {
                    geo(&k, v);
                }
            }
        }
        rows
    }

    /// The layers panel's rows: name, colour, on, frozen, locked, line type, lineweight.
    pub fn layer_rows(&self) -> Vec<LayerRow> {
        let d = &self.drawing;
        self.geom
            .scene
            .layers
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let (on, frozen) = self
                    .layer_state
                    .get(&l.handle)
                    .copied()
                    .unwrap_or((l.on, l.frozen));
                let (ltype, weight) = match d.objects.get(&l.handle).map(|o| &o.data) {
                    Some(Data::Layer(x)) => (
                        entry_name(d, x.ltype).unwrap_or_else(|| "Continuous".into()),
                        lineweight_name(((x.flags as u16 >> 5) & 31) as u8),
                    ),
                    _ => ("Continuous".into(), "Default".into()),
                };
                LayerRow {
                    index: i,
                    name: l.name.clone(),
                    rgb: l.rgb,
                    on,
                    frozen,
                    locked: l.locked,
                    plot: l.plot,
                    ltype,
                    weight,
                }
            })
            .collect()
    }

    /// Runs a command by its name or alias, as typed or as a button asks. Editing commands are
    /// not here yet: they say so.
    pub fn run_command(&mut self, word: &str) -> Run {
        let word = word.trim();
        if word.is_empty() {
            // Enter on an empty line: the last command again
            return match self.last_command {
                Some(c) => self.run_command(c),
                None => Run::Done,
            };
        }
        if let Some(p) = self.pending.take() {
            return self.option(p, word);
        }
        let Some(c) = command_of(word) else {
            self.say(format!("Command: {word}"));
            self.say(format!(
                "Unknown command \"{}\".  Press F1 for help.",
                word.to_ascii_uppercase()
            ));
            return Run::Done;
        };
        self.say(format!("Command: {}", c.name));
        self.last_command = Some(c.name);
        if !c.works {
            self.say(format!(
                "{}: not yet. Editing commands come in a later version of this viewer.",
                c.name
            ));
            return Run::Done;
        }
        match c.name {
            "LAYER" => {
                self.show_layers = !self.show_layers;
                Run::Done
            }
            "PROPERTIES" => {
                self.show_props = !self.show_props;
                Run::Done
            }
            "ZOOM" => {
                self.pending = Some("ZOOM");
                self.say("Specify corner of window, enter a scale factor (nX or nXP), or [All/Center/Dynamic/Extents/Previous/Scale/Window/Object] <real time>:");
                Run::Done
            }
            "PAN" => {
                self.say("Press the middle button (or hold Space) and drag to pan. Press Esc to exit.");
                Run::Done
            }
            "REGEN" => {
                self.rebuild();
                self.say("Regenerating model.");
                Run::Done
            }
            "LWDISPLAY" => {
                self.lwt = !self.lwt;
                self.say(format!("LWDISPLAY is {}", if self.lwt { "ON" } else { "OFF" }));
                Run::Done
            }
            "QSAVE" => {
                self.save();
                Run::Done
            }
            "SAVEAS" => Run::SaveAs,
            "OPEN" => Run::Open,
            "AUDIT" => {
                let n = self.drawing.warnings.len();
                self.say(format!(
                    "{} objects, {n} kept as read",
                    self.drawing.objects.len()
                ));
                for w in self.drawing.warnings.iter().take(20).cloned().collect::<Vec<_>>() {
                    self.say(format!("  {w}"));
                }
                Run::Done
            }
            _ => Run::Done,
        }
    }

    /// A command's option, typed after its prompt.
    fn option(&mut self, cmd: &'static str, word: &str) -> Run {
        self.say(word.to_string());
        match (cmd, word.to_ascii_uppercase().as_str()) {
            ("ZOOM", "E" | "EXTENTS" | "A" | "ALL") => Run::ZoomExtents,
            ("ZOOM", w) => {
                // a scale factor: 2, 2X
                match w.trim_end_matches('X').parse::<f64>() {
                    Ok(f) if f > 0.0 => {
                        if let Some(v) = &mut self.view {
                            v.scale *= f;
                        }
                    }
                    _ => self.say("Requires a scale factor or an option keyword."),
                }
                Run::Done
            }
            _ => Run::Done,
        }
    }

    /// Esc: the command waiting called off, else the selection cleared.
    pub fn escape(&mut self) {
        if self.pending.take().is_some() || !self.input.is_empty() {
            self.input.clear();
            self.say("*Cancel*");
        } else {
            self.clear_selection();
        }
        self.press = None;
    }
}

/// What a command leaves the window to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    Done,
    ZoomExtents,
    SaveAs,
    Open,
}

/// A row of the Layers panel.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerRow {
    pub index: usize,
    pub name: String,
    pub rgb: [u8; 3],
    pub on: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plot: bool,
    pub ltype: String,
    pub weight: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_find_their_commands() {
        for (typed, name) in [
            ("l", "LINE"),
            ("PL", "PLINE"),
            ("c", "CIRCLE"),
            ("A", "ARC"),
            ("rec", "RECTANG"),
            ("H", "HATCH"),
            ("T", "TEXT"),
            ("DIM", "DIM"),
            ("M", "MOVE"),
            ("CO", "COPY"),
            ("RO", "ROTATE"),
            ("MI", "MIRROR"),
            ("SC", "SCALE"),
            ("TR", "TRIM"),
            ("EX", "EXTEND"),
            ("O", "OFFSET"),
            ("F", "FILLET"),
            ("E", "ERASE"),
            ("X", "EXPLODE"),
            ("LA", "LAYER"),
            ("Z", "ZOOM"),
            ("P", "PAN"),
            ("_line", "LINE"),
        ] {
            assert_eq!(command_of(typed).map(|c| c.name), Some(name), "{typed}");
        }
        assert!(command_of("FROB").is_none());
    }

    #[test]
    fn lineweights_and_colours() {
        assert_eq!(lineweight_name(7), "0.25 mm");
        assert_eq!(lineweight_name(29), "ByLayer");
        assert_eq!(colour_name(&Color::index(256)), "ByLayer");
        assert_eq!(colour_name(&Color::index(1)), "Red");
    }
}
