//! PCB document state. Geometry, DRC and the fabrication service are compiled from
//! the same source the CLI consumes. Navigation never rebuilds the board.
mod canvas;
mod placement;
mod workspace;
use egui::Vec2;
use forge_lang::{
    edit,
    error::Op,
    pcb::{self, Board, PlotLayer, Point, Side},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub const NEW_BOARD: &str = r#"pcb
perimeter = edge(points: [(0,0),(50,0),(50,30),(0,30)])
top = copper(thickness: 0.035mm, weight_oz_ft2: 1)
core = dielectric(material: "FR-4", thickness: 1.53mm)
bottom = copper(thickness: 0.035mm, weight_oz_ft2: 1)
stack = stackup(layers: [top,core,bottom])
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Select,
    Place,
    Move,
    Rotate,
    Flip,
    Route,
    Checks,
    Stack,
    Fit,
    Grid,
    Layers,
    Properties,
}
#[derive(Clone, Copy, PartialEq)]
enum Inspector {
    Properties,
    Checks,
    Stack,
    Layers,
}
struct Revision {
    board: Board,
    scene: canvas::Scene,
    checks: Vec<(String, String, String)>,
    deps: Vec<PathBuf>,
    services: Vec<(String, String, String)>,
}
impl Revision {
    fn build(src: &str, path: Option<&str>) -> Result<Self, String> {
        let dir = path
            .and_then(|p| Path::new(p).parent())
            .filter(|p| !p.as_os_str().is_empty());
        let root = path.and_then(crate::gui::gui_docs::root_of);
        let watch = forge_lang::deps::watch();
        let board =
            pcb::compile(src, dir, root.as_deref()).map_err(|e| format!("{} · {}", e.kind, e.message))?;
        let scene = canvas::Scene::new(&board)?;
        let checks = board
            .check()
            .findings
            .into_iter()
            .map(|f| {
                use pcb::Witness;
                let why = match f.witness {
                    Witness::Library { ref message, .. } => message.clone(),
                    Witness::Pair { other, layer } => format!("{other} on {layer}"),
                    Witness::Disconnected { other, net } => format!("{other} on {net}"),
                    Witness::Spacing {
                        other,
                        layer,
                        minimum_nm,
                        measured_floor_nm,
                        ..
                    } => format!(
                        "{other} on {layer} · {:.4} mm / {:.4} mm",
                        mm(measured_floor_nm),
                        mm(minimum_nm)
                    ),
                    Witness::BelowMinimum {
                        twice_measured_nm,
                        minimum_nm,
                    } => format!("{:.4} mm / {:.4} mm", mm(twice_measured_nm) / 2., mm(minimum_nm)),
                    Witness::Outside { .. } => "Outside board profile".into(),
                };
                (
                    f.rule.name().into(),
                    f.object.into(),
                    format!("{} · {why}", board.severity(f.rule)),
                )
            })
            .collect();
        let profiles = pcb::profiles(dir, root.as_deref());
        let mut services = vec![];
        for id in profiles.ids() {
            let v = &profiles.find(id).unwrap().vendor;
            for s in &v.pcb {
                services.push((v.id.clone(), s.id.clone(), format!("{} · {}", v.name, s.id)));
            }
        }
        let deps = watch.done().0.into_keys().collect();
        Ok(Self {
            board,
            scene,
            checks,
            deps,
            services,
        })
    }
}
pub struct Editor {
    pub path: Option<String>,
    place: placement::Form,
    src: String,
    saved: String,
    undo: Vec<String>,
    redo: Vec<String>,
    rev: Revision,
    active: PlotLayer,
    hidden: BTreeSet<PlotLayer>,
    selected: String,
    net: String,
    mode: Command,
    panel: Inspector,
    compact_panel: bool,
    find_layer: String,
    pitch: f64,
    grid: bool,
    snap: bool,
    zoom: f32,
    center: Vec2,
    pub fit: bool,
    draft: Vec<Point>,
    width: f64,
    net_name: String,
    message: String,
    cursor: Option<Point>,
}
fn mm(n: i64) -> f64 {
    n as f64 / 1e6
}
fn coord(p: Point) -> String {
    format!("({:.6}mm, {:.6}mm)", mm(p[0]), mm(p[1]))
}
fn field(id: &str, arg: &str, text: String) -> Op {
    Op::SetArg {
        id: id.into(),
        arg: arg.into(),
        text,
    }
}
impl Editor {
    pub fn open(path: Option<&str>, src: String) -> Result<Self, String> {
        let rev = Revision::build(&src, path)?;
        let active = rev.scene.layers[0].layer.clone();
        let hidden = rev
            .scene
            .layers
            .iter()
            .filter(|s| !s.default_visible)
            .map(|s| s.layer.clone())
            .collect();
        let net = rev.board.nets.keys().next().cloned().unwrap_or_default();
        let (undo, redo) = path
            .and_then(|p| forge_lang::undo::history(Path::new(p), &src))
            .unwrap_or_default();
        Ok(Self {
            path: path.map(str::to_owned),
            place: Default::default(),
            saved: src.clone(),
            src,
            rev,
            active,
            hidden,
            selected: String::new(),
            net,
            undo,
            redo,
            mode: Command::Select,
            panel: Inspector::Properties,
            compact_panel: false,
            find_layer: String::new(),
            pitch: 0.5,
            grid: true,
            snap: true,
            zoom: 1.,
            center: Vec2::ZERO,
            fit: true,
            draft: vec![],
            width: 0.25,
            net_name: String::new(),
            message: String::new(),
            cursor: None,
        })
    }
    pub fn dirty(&self) -> bool {
        self.saved != self.src
    }
    pub fn dependencies(&self) -> Vec<PathBuf> {
        self.rev.deps.clone()
    }
    fn install(&mut self, src: String) -> Result<(), String> {
        let new = Revision::build(&src, self.path.as_deref())?;
        if !new.scene.layers.iter().any(|s| s.layer == self.active) {
            self.active = new.scene.layers[0].layer.clone();
        }
        if !new.board.nets.contains_key(&self.net) {
            self.net = new.board.nets.keys().next().cloned().unwrap_or_default();
        }
        self.rev = new;
        self.src = src;
        self.draft.clear();
        self.message.clear();
        Ok(())
    }
    fn edit(&mut self, ops: Vec<Op>) -> bool {
        let mut next = self.src.clone();
        for op in ops {
            match edit::apply(&next, &op) {
                Ok(s) => next = s,
                Err(e) => {
                    self.message = e.text();
                    return false;
                }
            }
        }
        if next == self.src {
            return true;
        }
        let prev = self.src.clone();
        match self.install(next) {
            Ok(()) => {
                self.undo.push(prev);
                self.redo.clear();
                true
            }
            Err(e) => {
                self.message = e;
                false
            }
        }
    }
    pub fn step(&mut self, redo: bool) {
        let Some(next) = (if redo { &self.redo } else { &self.undo }).last().cloned() else {
            return;
        };
        let prev = self.src.clone();
        match self.install(next) {
            Ok(()) => {
                if redo {
                    self.redo.pop();
                    self.undo.push(prev)
                } else {
                    self.undo.pop();
                    self.redo.push(prev)
                }
            }
            Err(e) => self.message = e,
        }
    }
    pub fn save(&mut self, path: &str) -> Result<(), String> {
        if Path::new(path).extension().and_then(|e| e.to_str()) != Some("fg") {
            return Err("Choose a .fg filename".into());
        }
        if self.path.as_deref() == Some(path)
            && let Ok(src) = forge_dirs::fs::read_to_string(path)
            && src != self.saved
            && src != self.src
        {
            return Err("File changed on disk. Use Save As to keep your edits.".into());
        }
        let rev = Revision::build(&self.src, Some(path))?;
        forge_dirs::fs::write(path, &self.src).map_err(|e| e.to_string())?;
        self.rev = rev;
        self.path = Some(path.into());
        self.saved = self.src.clone();
        forge_lang::undo::keep(Path::new(path), &self.undo, &self.src, &self.redo).map_err(|e| e.to_string())
    }
    pub fn reload(&mut self, own: bool, text: Option<&str>) {
        if own && text == Some(&self.saved) {
            return;
        }
        if own && self.dirty() {
            self.message = "File changed on disk. Use Save As to keep your edits.".into();
            return;
        }
        let next = if own {
            let Some(s) = text else { return };
            s.to_owned()
        } else {
            self.src.clone()
        };
        match self.install(next) {
            Ok(()) if own => {
                self.saved = self.src.clone();
                (self.undo, self.redo) = self
                    .path
                    .as_ref()
                    .and_then(|p| forge_lang::undo::history(Path::new(p), &self.src))
                    .unwrap_or_default()
            }
            Err(e) => self.message = e,
            _ => {}
        }
    }
    fn next_id(&self, prefix: &str) -> String {
        let names: BTreeSet<_> = forge_lang::syntax::parse(&self.src)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| match s.kind {
                forge_lang::syntax::StmtKind::Assign { name, .. }
                | forge_lang::syntax::StmtKind::Shared { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        (1..)
            .map(|i| format!("{prefix}{i}"))
            .find(|s| !names.contains(s))
            .unwrap()
    }
    fn commit_route(&mut self) {
        let PlotLayer::Copper(layer) = &self.active else {
            self.message = "Select a copper layer to route".into();
            return;
        };
        if self.draft.len() < 2 || self.net.is_empty() {
            self.message = "Choose a net and at least two distinct points".into();
            return;
        }
        let mut ops = Vec::new();
        let mut net = self.net.clone();
        // An automatically bound schematic net can lack a board identifier. Declare
        // the first public alias and its track together so later reroutes use it too.
        if net.starts_with('@') {
            let b = &self.rev.board;
            let terminal = b.schematic.as_ref().and_then(|s| {
                s.circuit
                    .net_of
                    .iter()
                    .find(|(pin, n)| **n == b.nets[&net].name && s.circuit.terminals.contains_key(*pin))
                    .map(|(pin, _)| pin)
            });
            let Some(terminal) = terminal else {
                self.message = "No schematic terminal is available for this net".into();
                return;
            };
            net = self.next_id("net");
            ops.push(Op::AddFeature {
                before: None,
                text: format!("{net} = net(terminal: {})", forge_lang::props::quote(terminal)),
            });
        }
        let id = self.next_id("track");
        let text = format!(
            "{id} = track(net: {}, layer: {layer}, width: {:.6}mm, points: [{}])",
            net,
            self.width,
            self.draft
                .iter()
                .copied()
                .map(coord)
                .collect::<Vec<_>>()
                .join(", ")
        );
        ops.push(Op::AddFeature { before: None, text });
        if self.edit(ops) {
            self.net = net;
            self.selected = id;
            self.mode = Command::Select
        }
    }
    fn command(&mut self, cmd: Command) {
        self.message.clear();
        match cmd {
            Command::Fit => self.fit = true,
            Command::Grid => self.grid = !self.grid,
            Command::Rotate | Command::Flip => {
                let Some(p) = self.rev.board.layout.placements.get(&self.selected) else {
                    self.message = "Select a footprint first".into();
                    return;
                };
                let op = if cmd == Command::Rotate {
                    field(
                        &self.selected,
                        "angle",
                        format!("{}deg", (p.pose.angle + 90.) % 360.),
                    )
                } else {
                    field(
                        &self.selected,
                        "side",
                        format!("\"{}\"", if p.pose.side == Side::Top { "bottom" } else { "top" }),
                    )
                };
                self.edit(vec![op]);
            }
            Command::Checks | Command::Stack | Command::Properties | Command::Layers => {
                self.panel = match cmd {
                    Command::Checks => Inspector::Checks,
                    Command::Stack => Inspector::Stack,
                    Command::Layers => Inspector::Layers,
                    _ => Inspector::Properties,
                };
                self.compact_panel = true;
            }
            Command::Move if !self.rev.board.layout.placements.contains_key(&self.selected) => {
                self.message = "Select a footprint first".into()
            }
            Command::Route if !matches!(self.active, PlotLayer::Copper(_)) => {
                self.message = "Select a copper layer to route".into()
            }
            _ => {
                if cmd == Command::Route {
                    self.hidden.remove(&self.active);
                }
                self.draft.clear();
                if matches!(cmd, Command::Place | Command::Route) {
                    self.compact_panel = true;
                }
                self.mode = cmd;
                self.panel = Inspector::Properties;
            }
        }
    }
}
