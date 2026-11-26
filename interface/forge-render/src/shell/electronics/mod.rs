//! Native electronic documents. The canvas, browser and inspector edit a single .fg source.
mod annotation;
mod canvas;
mod inspector;
mod palette;
mod workspace;
use super::{
    Action, Shell, Theme,
    ribbon::{Group, Tool as RibbonTool},
};
use egui::{Pos2, Rect, Ui, pos2, vec2};
use forge_lang::{
    edit,
    error::Op,
    props::quote,
    schematic::{self as sch, Circuit, Document, Item, Point},
    syntax::{self, ExprKind, StmtKind},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Place,
    Power,
    Wire,
    Junction,
    Label,
    Port,
    NoConnect,
    Note,
    Move,
    Rotate,
    Mirror,
    Delete,
    Sheet,
    Subsheet,
    Gate,
    Library,
    Checks,
    Bom,
    Fit,
    Grid,
    NewSymbol,
    Pin,
    Line,
    Rect,
    Circle,
    Arc,
    Text,
    Role,
    Properties,
    Intent,
    Bus,
    Flow,
    Directions,
    Dots,
}
pub fn tab_name(library: bool, tab: usize) -> &'static str {
    match tab {
        0 if library => "Symbol",
        0 => "Schematic",
        1 => "Inspect",
        3 if !library => "Draw",
        _ => "View",
    }
}

#[derive(Default)]
struct Inputs {
    description: String,
    power_symbol: Option<String>,
    line_width: f64,
    text_size: f64,
    bold: bool,
    name: String,
    text: String,
    number: String,
    electrical: String,
    unit: String,
    value: String,
    ref_offset: Option<Point>,
    value_offset: Option<Point>,
    footprint: String,
    datasheet: String,
    fields: Vec<(String, String)>,
    populated: bool,
    in_bom: bool,
    global: bool,
    angle: i32,
    geometry: Vec<Point>,
    radius: f64,
    wants: Vec<String>,
    gives: Vec<String>,
    return_from: String,
    return_to: String,
    preferred_distance: String,
    role_kind: String,
    role: String,
    group: String,
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
enum Dock {
    Parts,
    Power,
    #[default]
    Properties,
}

#[derive(Clone, Copy, Default)]
struct SignalMarkers {
    arrows: bool,
    dots: bool,
}
impl SignalMarkers {
    fn active(self) -> bool {
        self.arrows || self.dots
    }
}

struct PinFocus {
    id: String,
    inspection: sch::PinInspection,
    page: forge_draw::Page,
}

pub struct Editor {
    pub path: Option<String>,
    src: String,
    saved: String,
    undo: Vec<String>,
    redo: Vec<String>,
    doc: Document,
    circuit: Circuit,
    page: forge_draw::Page,
    subject: String,
    unit: String,
    selected: String,
    pin: String,
    graphic: Option<usize>,
    symbol: String,
    search: String,
    dock: Dock,
    catalog_search: String,
    preview: Option<(String, String, forge_draw::Page)>,
    power_search: String,
    power_aliases: Vec<String>,
    catalog: Vec<(String, String)>,
    collapsed: std::collections::BTreeSet<String>,
    focus_pin: Option<PinFocus>,
    markers: SignalMarkers,
    directions: BTreeMap<String, sch::Direction>,
    tool: Tool,
    form: Inputs,
    first: Option<String>,
    points: Vec<Point>,
    pub fit: bool,
    origin: Pos2,
    zoom: f32,
    grid: bool,
    snap: bool,
    pitch: f64,
    inspector_open: bool,
    message: String,
    pub open_file: Option<String>,
    pub attach_requested: bool,
}
fn compile(src: &str, path: Option<&str>) -> Result<(Document, Circuit)> {
    let dir = path
        .and_then(|s| Path::new(s).parent())
        .filter(|p| !p.as_os_str().is_empty());
    sch::compile(src, dir, None).map_err(|e| format!("{} · {}", e.kind, e.message))
}
fn arg(id: &str, name: &str, value: String) -> Op {
    Op::SetArg {
        id: id.into(),
        arg: name.into(),
        text: value,
    }
}
fn source_arg<'a>(statements: &'a [syntax::Stmt], id: &str, field: &str) -> Option<&'a syntax::Expr> {
    statements.iter().find_map(|s| {
        if let StmtKind::Assign { name, value } = &s.kind
            && name == id
            && let ExprKind::Call(_, args) = &value.kind
        {
            args.iter()
                .find(|a| a.name.as_ref().is_some_and(|(k, _)| k == field))
                .map(|a| &a.value)
        } else {
            None
        }
    })
}
fn power_aliases(src: &str) -> Vec<String> {
    let tree = syntax::parse(src).unwrap_or_default();
    tree.iter()
        .filter_map(|s| {
            let StmtKind::Assign { name, value } = &s.kind else {
                return None;
            };
            let ExprKind::Call(function, args) = &value.kind else {
                return None;
            };
            let is_power = matches!(&function.kind, ExprKind::Name(n) if n == "library")
                && args.iter().any(|arg| {
                    arg.name.as_ref().is_some_and(|(k, _)| k == "file")
                        && matches!(&arg.value.kind, ExprKind::Str(path) if path == "forge:power")
                });
            is_power.then(|| name.clone())
        })
        .collect()
}
fn coord(p: Point) -> String {
    format!("({}mm, {}mm)", p[0], p[1])
}
fn field_text(fields: &[(String, String)]) -> Result<String> {
    let mut pairs = BTreeMap::new();
    for (k, v) in fields {
        if !identifier(k) || pairs.insert(k, v).is_some() {
            return Err("Each field needs a unique identifier.".into());
        }
    }
    Ok(format!(
        "{{{}}}",
        pairs
            .into_iter()
            .map(|(k, v)| format!("{k}: {}", quote(v)))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
fn identifier(s: &str) -> bool {
    s.starts_with(|c: char| c == '_' || c.is_ascii_alphabetic())
        && s.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric())
        && !syntax::KEYWORDS.contains(&s)
}
impl Editor {
    pub fn open(path: Option<&str>, src: String) -> Result<Self> {
        let (doc, circuit) = compile(&src, path)?;
        let (undo, redo) = path
            .and_then(|p| forge_lang::undo::history(Path::new(p), &src))
            .unwrap_or_default();
        let mut e = Self {
            path: path.map(str::to_owned),
            saved: src.clone(),
            src,
            undo,
            redo,
            doc,
            circuit,
            page: Default::default(),
            subject: String::new(),
            unit: "A".into(),
            selected: String::new(),
            pin: String::new(),
            graphic: None,
            symbol: String::new(),
            search: String::new(),
            dock: Dock::Properties,
            catalog_search: String::new(),
            preview: None,
            power_search: String::new(),
            power_aliases: vec![],
            catalog: vec![],
            collapsed: Default::default(),
            focus_pin: None,
            markers: Default::default(),
            directions: Default::default(),
            tool: Tool::Select,
            form: Default::default(),
            first: None,
            points: Vec::new(),
            fit: true,
            origin: Pos2::ZERO,
            zoom: 10.,
            grid: true,
            snap: true,
            pitch: 0.5,
            inspector_open: false,
            message: String::new(),
            open_file: None,
            attach_requested: false,
        };
        e.refresh();
        e.inspect();
        Ok(e)
    }
    pub fn library(&self) -> bool {
        self.doc.library
    }
    pub fn dirty(&self) -> bool {
        self.src != self.saved
    }
    fn refresh(&mut self) {
        self.preview = None;
        let focused = self.focus_pin.take().map(|f| f.id);
        if self.doc.library {
            if !self.doc.symbols.contains_key(&self.subject) {
                self.subject = self.doc.symbols.keys().next().cloned().unwrap_or_default();
            }
            if let Some(s) = self.doc.symbols.get(&self.subject)
                && !s.pins.values().any(|p| p.unit == self.unit)
                && let Some(p) = s.pins.values().next()
            {
                self.unit = p.unit.clone();
            }
            if !self
                .doc
                .symbols
                .get(&self.subject)
                .is_some_and(|s| s.pins.contains_key(&self.pin))
            {
                self.pin.clear();
            }
            if self.graphic.is_some_and(|i| {
                self.doc
                    .symbols
                    .get(&self.subject)
                    .is_none_or(|s| i >= s.graphics.len())
            }) {
                self.graphic = None;
            }
            self.page = sch::symbol_page(&self.doc, &self.subject, &self.unit).unwrap_or_default();
        } else {
            if !self.circuit.occurrences.contains_key(&self.subject) {
                self.subject = self
                    .circuit
                    .occurrences
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_default();
            }
            self.power_aliases = power_aliases(&self.src);
            if self
                .doc
                .symbols
                .get(&self.symbol)
                .is_none_or(|s| s.power_net.is_some())
            {
                self.symbol = self
                    .doc
                    .symbols
                    .iter()
                    .find(|(_, s)| s.power_net.is_none())
                    .map(|(id, _)| id.clone())
                    .unwrap_or_default();
            }
            if !self
                .doc
                .rows
                .iter()
                .any(|r| r.id == self.selected && r.sheet == self.sheet())
            {
                self.selected.clear();
                self.pin.clear();
            }
            self.page = sch::sheet_page(&self.doc, &self.circuit, &self.subject).unwrap_or_default();
            self.catalog = self
                .doc
                .symbols
                .iter()
                .filter(|(_, s)| s.power_net.is_none())
                .map(|(name, s)| {
                    let mut terms = vec![name.as_str()];
                    terms.extend(s.fields.values().map(String::as_str));
                    terms.extend(
                        s.pins
                            .iter()
                            .flat_map(|(n, p)| [n.as_str(), p.description.as_str()]),
                    );
                    (name.clone(), terms.join(" ").to_lowercase())
                })
                .collect();
            if self.markers.active() {
                self.directions = sch::wire_directions(&self.circuit);
            }
            if let Some(id) = focused {
                self.focus_on(&id);
            }
        }
    }
    fn install(&mut self, src: String) -> Result<()> {
        let (d, c) = compile(&src, self.path.as_deref())?;
        self.doc = d;
        self.circuit = c;
        self.src = src;
        self.refresh();
        Ok(())
    }
    fn apply(&mut self, ops: Vec<Op>) -> Result<()> {
        let before = self.src.clone();
        let mut next = before.clone();
        for op in ops {
            next = edit::apply(&next, &op).map_err(|e| e.text())?;
        }
        if next != before {
            self.install(next)?;
            self.undo.push(before);
            self.redo.clear();
        }
        Ok(())
    }
    pub fn step(&mut self, redo: bool) {
        let stack = if redo { &self.redo } else { &self.undo };
        if let Some(next) = stack.last().cloned() {
            let prev = self.src.clone();
            match self.install(next) {
                Ok(()) => {
                    if redo {
                        self.redo.pop();
                        self.undo.push(prev);
                    } else {
                        self.undo.pop();
                        self.redo.push(prev);
                    }
                    self.start(Tool::Select);
                }
                Err(e) => self.message = e,
            }
        }
    }
    pub fn save(&mut self, path: &str) -> Result<()> {
        if Path::new(path).extension().and_then(|e| e.to_str()) != Some("fg") {
            return Err("Choose a .fg filename.".into());
        }
        if self.path.as_deref() == Some(path)
            && let Ok(on_disk) = forge_dirs::fs::read_to_string(path)
            && on_disk != self.saved
            && on_disk != self.src
        {
            return Err("The file changed on disk. Save As to keep your edits.".into());
        }
        let (doc, circuit) = compile(&self.src, Some(path))?;
        if let Some(old) = self.path.as_deref().filter(|old| *old != path) {
            let symbols = self
                .doc
                .symbols
                .iter()
                .filter(|(id, _)| !id.contains('.'))
                .filter_map(|(_, s)| s.datasheet.as_deref());
            let placements = self.doc.rows.iter().filter_map(|r| match &r.item {
                Item::Place { datasheet, .. } => datasheet.as_deref(),
                _ => None,
            });
            for link in symbols
                .chain(placements)
                .collect::<std::collections::BTreeSet<_>>()
            {
                sch::datasheet::copy_managed(Path::new(old), Path::new(path), link)
                    .map_err(|e| e.to_string())?;
            }
        }
        forge_dirs::fs::write(path, &self.src).map_err(|e| e.to_string())?;
        self.path = Some(path.into());
        self.saved = self.src.clone();
        self.doc = doc;
        self.circuit = circuit;
        self.refresh();
        forge_lang::undo::keep(Path::new(path), &self.undo, &self.src, &self.redo)
            .map_err(|e| format!("Document saved; could not save undo history: {e}"))
    }
    pub fn dependencies(&self) -> Vec<PathBuf> {
        self.doc.library_paths().map(Path::to_owned).collect()
    }
    pub fn reload(&mut self, own: bool, text: Option<&str>) {
        if own && self.dirty() {
            if text != Some(&self.saved) {
                self.message = "File changed on disk. Save As to preserve your edits.".into();
            }
            return;
        }
        let next = if own {
            let Some(s) = text else { return };
            s.to_owned()
        } else {
            self.src.clone()
        };
        match self.install(next) {
            Ok(()) => {
                if own {
                    self.saved = self.src.clone();
                    (self.undo, self.redo) = self
                        .path
                        .as_ref()
                        .and_then(|p| forge_lang::undo::history(Path::new(p), &self.src))
                        .unwrap_or_default();
                }
                self.inspect();
            }
            Err(e) => self.message = e,
        }
    }
    fn sheet(&self) -> &str {
        self.circuit
            .occurrences
            .get(&self.subject)
            .map(String::as_str)
            .unwrap_or("")
    }
    fn fresh(&self, prefix: &str) -> String {
        let names: std::collections::BTreeSet<_> = syntax::parse(&self.src)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| match s.kind {
                StmtKind::Assign { name, .. } | StmtKind::Shared { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        (1..)
            .map(|i| format!("{prefix}{i}"))
            .find(|id| !names.contains(id))
            .unwrap()
    }
    fn add(&mut self, id: &str, body: String) -> Result<()> {
        if !identifier(id) {
            return Err("Enter a valid, unique identifier.".into());
        }
        let before = if self.tool == Tool::Role {
            syntax::parse(&self.src)
                .map_err(|e| e.text())?
                .into_iter()
                .find_map(|stmt| match stmt.kind {
                    StmtKind::Assign { name, .. } if self.doc.symbols.contains_key(&name) => Some(name),
                    _ => None,
                })
        } else {
            None
        };
        self.apply(vec![Op::AddFeature {
            before,
            text: format!("{id} = {body}"),
        }])
    }
    fn start(&mut self, t: Tool) {
        if matches!(t, Tool::Directions | Tool::Dots) {
            let was_active = self.markers.active();
            let setting = if t == Tool::Dots {
                &mut self.markers.dots
            } else {
                &mut self.markers.arrows
            };
            *setting = !*setting;
            if !was_active {
                self.directions = sch::wire_directions(&self.circuit);
            } else if !self.markers.active() {
                self.directions.clear();
            }
            return;
        }
        if self.focus_pin.take().is_some() {
            self.fit = true;
        }
        self.dock = match t {
            Tool::Place => Dock::Parts,
            Tool::Power => Dock::Power,
            _ => Dock::Properties,
        };

        self.first = None;
        self.points.clear();
        self.message.clear();
        self.tool = t;
        if !matches!(t, Tool::Delete | Tool::Properties | Tool::Move) {
            self.graphic = None;
        }
        match t {
            Tool::Fit => {
                self.fit = true;
                self.tool = Tool::Select;
            }
            Tool::Grid => {
                self.grid = !self.grid;
                self.tool = Tool::Select;
            }
            Tool::Select | Tool::Properties | Tool::Move => self.inspect(),
            Tool::Rotate | Tool::Mirror | Tool::Delete => {
                let result = self.modify(t);
                self.tool = Tool::Select;
                if let Err(e) = result {
                    self.message = e;
                }
                self.inspect();
            }
            _ => {
                self.form = Inputs {
                    text_size: 1.55,
                    line_width: 0.25,
                    electrical: "passive".into(),
                    unit: self.unit.clone(),
                    role_kind: "bypass".into(),
                    number: "1".into(),
                    populated: true,
                    in_bom: true,
                    ..Default::default()
                };
                self.form.name = self.fresh(match t {
                    Tool::Place => "U",
                    Tool::Pin => "pin",
                    Tool::Library => "parts",
                    Tool::Role => "role",
                    Tool::Sheet => "sheet",
                    Tool::Subsheet => "channel",
                    Tool::NewSymbol => "Symbol",
                    _ => "item",
                });
                if t == Tool::Pin
                    && let Some(s) = self.doc.symbols.get(&self.subject)
                {
                    self.form.number = (1..)
                        .map(|n| n.to_string())
                        .find(|n| !s.pins.values().any(|p| &p.number == n))
                        .unwrap();
                }
                if t == Tool::Power {
                    self.form.text = "GND".into();
                }
                if t == Tool::Label {
                    self.form.text = "VCC".into();
                }
                if t == Tool::Intent {
                    self.form.role = self.doc.role_names.keys().next().cloned().unwrap_or_default();
                }
                if t == Tool::Subsheet {
                    self.form.group = self
                        .doc
                        .sheets
                        .keys()
                        .find(|s| s.as_str() != self.sheet())
                        .cloned()
                        .unwrap_or_default();
                }
            }
        }
    }
    /// A terminal is named for its electrical component, but can be drawn on another gate row.
    fn geometric_selection(&self) -> &str {
        if !self.pin.is_empty() {
            let key = format!("{}/{}.{}", self.subject, self.selected, self.pin);
            if let Some(span) = self.circuit.origins.get(&key)
                && let Some(row) = self
                    .doc
                    .rows
                    .iter()
                    .find(|r| r.span == *span && r.sheet == self.sheet())
            {
                return &row.id;
            }
        }
        &self.selected
    }

    fn modify(&mut self, t: Tool) -> Result<()> {
        if self.library() {
            if t == Tool::Delete {
                if !self.pin.is_empty() {
                    let k = self.pin.clone();
                    self.change_member("pins", Some(&k), None, None)?;
                    self.pin.clear();
                    return Ok(());
                }
                if let Some(i) = self.graphic {
                    return self.change_member("graphics", None, Some(i), None);
                }
                let id = self.subject.clone();
                return self.apply(vec![Op::DeleteFeature { id }]);
            }
            return Err("Select a placement in a schematic.".into());
        }
        let id = if matches!(t, Tool::Rotate | Tool::Mirror) {
            self.geometric_selection().to_owned()
        } else {
            self.selected.clone()
        };
        let Some(row) = self.doc.rows.iter().find(|r| r.id == id) else {
            return Err("Select an object first.".into());
        };
        let op = match (t, &row.item) {
            (Tool::Delete, _) => Op::DeleteFeature { id },
            (Tool::Rotate, Item::Place { angle, .. } | Item::Gate { angle, .. }) => {
                arg(&id, "angle", format!("{}deg", (angle + 90.) % 360.))
            }
            (Tool::Mirror, Item::Place { mirror, .. } | Item::Gate { mirror, .. }) => {
                arg(&id, "mirror", (!mirror).to_string())
            }
            _ => return Err("Select a placed symbol or gate.".into()),
        };
        self.apply(vec![op])
    }
    // Edit one pin or graphic in the existing argument. Keep every other member's expression.
    fn change_member(
        &mut self,
        field: &str,
        key: Option<&str>,
        index: Option<usize>,
        new: Option<String>,
    ) -> Result<()> {
        let tree = syntax::parse(&self.src).map_err(|e| e.text())?;
        let old = source_arg(&tree, &self.subject, field);
        let part = |e: &syntax::Expr| self.src[e.span.lo as usize..e.span.hi as usize].to_owned();
        let mut vals = Vec::new();
        let mut found = false;
        match old.map(|e| &e.kind) {
            Some(ExprKind::Map(m)) => {
                for (k, e) in m {
                    if key == Some(k) {
                        found = true;
                        if let Some(v) = &new {
                            vals.push(format!("{k}: {v}"));
                        }
                    } else {
                        vals.push(format!("{k}: {}", part(e)));
                    }
                }
            }
            Some(ExprKind::List(l)) => {
                for (i, e) in l.iter().enumerate() {
                    if index == Some(i) {
                        found = true;
                        if let Some(v) = &new {
                            vals.push(v.clone());
                        }
                    } else {
                        vals.push(part(e));
                    }
                }
            }
            _ => {}
        }
        if !found && let Some(v) = new {
            vals.push(if let Some(k) = key {
                if !identifier(k) {
                    return Err("Enter a pin identifier.".into());
                }
                format!("{k}: {v}")
            } else {
                v
            });
        }
        let inner = vals.join(", ");
        let value = if field == "pins" {
            format!("{{{inner}}}")
        } else {
            format!("[{inner}]")
        };
        self.apply(vec![arg(&self.subject, field, value)])
    }
}

impl Editor {
    pub fn attachment_target(&self) -> Option<(String, String)> {
        let id = if self.library() {
            &self.subject
        } else {
            &self.selected
        };
        Some((self.path.clone()?, id.clone()))
    }
    pub fn attach(&mut self, path: &str, id: &str, bytes: &[u8]) -> Result<()> {
        if self.attachment_target() != Some((path.into(), id.into())) {
            return Err("The attachment target changed. Select the part and attach again.".into());
        }
        let link = sch::datasheet::store_pdf(Path::new(path), bytes).map_err(|e| e.to_string())?;
        self.apply(vec![arg(id, "datasheet", quote(&link))])?;
        self.form.datasheet = link;
        self.message = "Datasheet attached".into();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_connect_toggles_existing_markers_in_one_edit() {
        let src = r#"schematic
S = symbol(pins: {p: pin(number: "1", kind: "input", at: (0,0))})
main = sheet()
chip = place(sheet: main, symbol: S, at: (0,0))
"#;
        let mut e = Editor::open(None, src.into()).unwrap();
        e.start(Tool::NoConnect);
        e.click([0., 0.], Some("chip.p"), None).unwrap();
        let marked = e.src.clone();
        assert!(marked.contains("no_connect("));
        e.click([0., 0.], Some("chip.p"), None).unwrap();
        assert!(!e.src.contains("no_connect("));
        e.step(false);
        assert_eq!(e.src, marked);
    }

    #[test]
    fn new_roles_precede_every_local_symbol() {
        let mut e = Editor::open(
            None,
            "symbol_library\nZ = symbol(pins: {})\nA = symbol(pins: {})\n".into(),
        )
        .unwrap();
        e.start(Tool::Role);
        e.form.name = "rail".into();
        e.form.role_kind = "power".into();
        e.create_record().unwrap();
        assert!(e.src.find("rail =").unwrap() < e.src.find("Z =").unwrap());
    }

    #[test]
    fn a_unit_pin_edits_the_displayed_gate_and_keeps_its_component_identity() {
        let src = r#"schematic
S = symbol(pins: {a: pin(unit: "A", number: "1", kind: "input", at: (2,0)), b: pin(unit: "B", number: "2", kind: "output", at: (1,0))})
top = sheet()
chip = place(sheet: top, symbol: S, at: (0,0), angle: 90deg)
second = gate(sheet: top, of: chip, unit: "B", at: (10,0))
"#;
        let mut e = Editor::open(None, src.into()).unwrap();
        e.start(Tool::Move);
        e.click([11., 0.], Some("chip.b"), None).unwrap();
        assert_eq!(e.selected, "chip");
        assert_eq!(e.geometric_selection(), "second");
        e.click([20., 0.], None, None).unwrap();
        assert_eq!(e.circuit.locations["top/chip.b"], [21., 0.]);
        e.modify(Tool::Rotate).unwrap();
        assert_eq!(e.circuit.locations["top/chip.b"], [20., 1.]);
        e.modify(Tool::Mirror).unwrap();
        assert_eq!(e.circuit.locations["top/chip.b"], [20., -1.]);
        assert_eq!(e.circuit.locations["top/chip.a"], [0., 2.]);
    }

    #[test]
    fn power_placement_snaps_connects_and_reuses_its_library() {
        let src = "schematic
main = sheet()
a = port(sheet:main, at:(3,2))
";
        let mut e = Editor::open(None, src.into()).unwrap();
        e.start(Tool::Power);
        e.click([2.9, 2.1], Some("a"), None).unwrap();
        let first = e.src.clone();
        assert_eq!(e.circuit.locations["main/PWR1.net"], [3., 2.]);
        assert!(e.circuit.is_attached("main/PWR1.net"));
        e.start(Tool::Power);
        e.click([10., 2.], None, None).unwrap();
        assert_eq!(e.src.matches("forge:power").count(), 1);
        assert_eq!(
            e.circuit.net_of["main/PWR1.net"],
            e.circuit.net_of["main/PWR2.net"]
        );
        assert!(!e.circuit.is_attached("main/PWR2.net"));
        e.step(false);
        assert_eq!(e.src, first);
        e.step(false);
        assert_eq!(e.src, src);
    }

    #[test]
    fn custom_power_uses_its_library_pin_and_unit() {
        let mut e = Editor::open(
            None,
            r#"schematic
main=sheet()
VEE=symbol(power_net:"VEE",pins:{return_pin:pin(number:"1",unit:"B",kind:"passive",at:(2,3))})
j=junction(sheet:main,at:(5,10))
"#
            .into(),
        )
        .unwrap();
        e.start(Tool::Power);
        e.form.power_symbol = Some("VEE".into());
        e.click([5., 10.], Some("j"), None).unwrap();
        assert_eq!(e.circuit.locations["main/PWR1.return_pin"], [5., 10.]);
        assert!(e.circuit.is_attached("main/PWR1.return_pin"));
        assert!(!e.src.contains("forge:power"));
    }

    #[test]
    fn branching_on_a_wire_connects_in_one_undoable_edit() {
        let src = "schematic
main = sheet()
a = port(sheet: main, at: (0,0))
b = port(sheet: main, at: (10,5))
c = port(sheet: main, at: (4,3))
trace = wire(sheet: main, from: a, to: b, via: [(10,0)])
";
        let mut e = Editor::open(None, src.into()).unwrap();
        e.start(Tool::Wire);
        e.click([4., 3.], Some("c"), None).unwrap();
        e.click([4., 0.], None, Some("trace")).unwrap();
        assert_eq!(e.circuit.wires.len(), 3);
        assert_eq!(e.circuit.net_of["main/c"], e.circuit.net_of["main/b"]);
        assert_eq!(e.circuit.locations["main/j1"], [4., 0.]);
        e.step(false);
        assert_eq!(e.src, src);
    }

    #[test]
    fn create_library_and_circuit_save_reopen_and_undo() {
        let dir = forge_dirs::temp_dir().join(format!("fg-electronics-editor-{}", forge_dirs::process_id()));
        forge_dirs::fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("my-symbols.fg");
        let mut e = Editor::open(None, "symbol_library\n".into()).unwrap();
        e.start(Tool::NewSymbol);
        e.form.name = "Sensor".into();
        e.create_record().unwrap();
        e.start(Tool::Role);
        e.form.name = "rail".into();
        e.form.role_kind = "power".into();
        e.create_record().unwrap();
        e.start(Tool::Pin);
        e.form.name = "supply".into();
        e.form.number = "1".into();
        e.form.wants = vec!["rail".into()];
        e.click([2., 0.], None, None).unwrap();
        e.start(Tool::Rect);
        e.click([-2., -2.], None, None).unwrap();
        e.click([2., 2.], None, None).unwrap();
        e.pin.clear();
        e.start(Tool::Properties);
        e.form.fields = vec![("MPN".into(), "DEMO".into())];
        e.save_properties().unwrap();
        let lname = lib.to_str().unwrap();
        e.save(lname).unwrap();
        e.attach(lname, "Sensor", b"%PDF-1.4\nexample\n%%EOF").unwrap();
        e.save(lname).unwrap();
        let link = e.doc.symbols["Sensor"].datasheet.as_ref().unwrap();
        assert_eq!(
            forge_dirs::fs::read(dir.join(link)).unwrap(),
            b"%PDF-1.4\nexample\n%%EOF"
        );
        let file = dir.join("test.fg");
        let path = file.to_str().unwrap();
        let mut circuit = Editor::open(None, "schematic\nmain = sheet()\n".into()).unwrap();
        circuit.save(path).unwrap();
        circuit.start(Tool::Library);
        circuit.form.name = "lib".into();
        circuit.form.text = "my-symbols.fg".into();
        circuit.create_record().unwrap();
        for (id, x) in [("U1", 0.), ("U2", 15.)] {
            circuit.start(Tool::Place);
            circuit.symbol = "lib.Sensor".into();
            circuit.form.name = id.into();
            circuit.click([x, 0.], None, None).unwrap();
        }
        circuit.start(Tool::Wire);
        circuit.click([2., 0.], Some("U1.supply"), None).unwrap();
        circuit.click([8., 3.], None, None).unwrap();
        circuit.click([17., 0.], Some("U2.supply"), None).unwrap();
        circuit.save(path).unwrap();
        let saved = forge_dirs::fs::read_to_string(path).unwrap();
        let mut back = Editor::open(Some(path), saved.clone()).unwrap();
        assert_eq!(back.circuit.nets.len(), 1);
        assert_eq!(sch::bom(&back.doc, &back.circuit)[0].references.len(), 2);
        back.step(false);
        assert_eq!(back.circuit.nets.len(), 2);
        back.step(true);
        assert_eq!(back.src, saved);
        back.select("U2", "");
        back.form.populated = false;
        back.save_properties().unwrap();
        assert_eq!(sch::bom(&back.doc, &back.circuit)[0].references, ["main/U1"]);
        forge_dirs::fs::remove_dir_all(dir).unwrap();
    }
}
