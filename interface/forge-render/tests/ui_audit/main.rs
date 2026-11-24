//! The button auditor: every command the app's own ribbon registry lists (`Shell::ribbon_live`),
//! on every tab and environment of a set of documents, clicked headless. Each click is a
//! before/after: egui's layers, the shell's placed names, the text drawn, the status line, the
//! pixels. It is classed (a dialog or popup, a panel, an environment, a mode, an action with
//! feedback, the system's file picker, or nothing visible, a panic, error text) and what it
//! opened is judged (`judge.rs`, docs/ux/STANDARD.md).
//!
//! `cargo test --release --test ui_audit` runs the smoke set (CI): it fails on a panic, a
//! command with nothing visible, or a judge finding not in `known.txt`.
//! `UI_AUDIT=full UI_AUDIT_OUT=DIR` crawls everything and writes `DIR/records.tsv` and
//! the shots; tools/ui-audit/crawl.sh runs that and research/ui-audit/report.py's report.

mod judge;
mod snap;

use egui::{Rect, pos2, vec2};
use forge_render::gui::Headless;
use forge_render::shell::Action;
use forge_render::shell::env::EnvId;
use judge::Finding;
use snap::Snap;
use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

thread_local! {
    /// What the frames since it was last taken asked of the window's host (Open, Save As..).
    static HOST: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

thread_local! {
    /// The tooltip a menu item showed on hover before its click.
    static TIP: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    /// The window as that tooltip showed it (PNG).
    static TIP_SHOT: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

/// Rests the pointer on `r` until a tooltip shows (or a while): its words.
fn hover_tip(h: &mut Headless, r: Rect) -> String {
    let before = Snap::take(h);
    h.move_to(r.center().x, r.center().y);
    for _ in 0..60 {
        step(h);
        let tips: Vec<Rect> = h.gui.ctx.memory(|m| {
            m.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|l| l.order == egui::Order::Tooltip)
                .filter_map(|l| m.area_rect(l.id))
                .collect()
        });
        if !tips.is_empty() {
            for _ in 0..3 {
                step(h);
            }
            let s = Snap::take(h);
            let words: Vec<String> = judge::new_texts_in(&s, &before, &tips)
                .iter()
                .map(|t| t.s.trim().to_string())
                .collect();
            if !words.is_empty() {
                TIP_SHOT.with(|t| *t.borrow_mut() = s.png());
                return words.join(" / ");
            }
        }
    }
    // no tooltip: a reason may be written under the item itself, small and muted
    let s = Snap::take(h);
    let under: Vec<String> = judge::on_top_in(
        &s,
        &[Rect::from_min_max(
            pos2(r.left() - 4.0, r.bottom() - 2.0),
            pos2(r.right(), r.bottom() + 20.0),
        )],
    )
    .iter()
    // painted after the item's own label (the menu's layer), not a panel's text beneath
    .filter(|t| {
        let item = s
            .texts
            .iter()
            .filter(|x| r.contains(x.rect.center()))
            .map(|x| x.idx)
            .max();
        t.size <= 11.5 && t.rect.top() >= r.bottom() - 2.0 && item.is_some_and(|i| t.idx > i)
    })
    .map(|t| t.s.trim().to_string())
    .collect();
    if !under.is_empty() {
        TIP_SHOT.with(|t| *t.borrow_mut() = s.png());
        return under.join(" / ");
    }
    String::new()
}

fn step(h: &mut Headless) {
    let out = h.step();
    // the window's own asks: its file dialogs, its frame (the rest of `host` is the app's)
    let window: Vec<String> = out
        .host
        .iter()
        .map(|a| format!("{a:?}"))
        .filter(|w| {
            [
                "Open",
                "SaveAs",
                "ExportMesh",
                "Minimize",
                "Maximize",
                "Close",
                "DragWindow",
                "ResizeWindow",
                "Sheet(NewPart)",
            ]
            .iter()
            .any(|k| w == k || w.starts_with(&format!("{k}(")))
        })
        .collect();
    if !window.is_empty() {
        HOST.with(|x| x.borrow_mut().extend(window));
    }
}

fn step_host(h: &mut Headless) -> Vec<String> {
    step(h);
    HOST.with(|x| std::mem::take(&mut *x.borrow_mut()))
}

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");

const PLATE: &str = "shared t = 6\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (60, 40))\n}\n\
plate = extrude(plan, t)\n";

/// A document to crawl in, and how to get it to the state the crawl starts from.
#[derive(Clone, Copy)]
pub struct Fixture {
    pub name: &'static str,
    /// An example (relative to examples/), `PLATE` written out, or none (the start screen).
    pub file: Option<&'static str>,
    pub setup: Setup,
    /// Tabs crawled here; empty: every tab shown. Tabs the same in every document (View,
    /// Inspect, Manage) are crawled in a few fixtures only.
    pub tabs: &'static [&'static str],
}

#[derive(Clone, Copy, PartialEq)]
pub enum Setup {
    None,
    /// Into a sketch's editing (the feature's id).
    EditSketch(&'static str),
    /// File > New Drawing from the part.
    NewDrawing,
}

const ALL: &[&str] = &[];
const DESIGN: &[&str] = &[
    "3D Model",
    "Sketch",
    "Assemble",
    "Weldments",
    "Harness",
    "Environments",
];

pub const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "start",
        file: None,
        setup: Setup::None,
        tabs: ALL,
    },
    Fixture {
        name: "plate",
        file: Some("PLATE"),
        setup: Setup::None,
        tabs: ALL,
    },
    Fixture {
        name: "part",
        file: Some("demo/part_tools.fg"),
        setup: Setup::None,
        tabs: &["3D Model", "Environments"],
    },
    Fixture {
        name: "sketch",
        file: Some("PLATE"),
        setup: Setup::EditSketch("plan"),
        tabs: ALL,
    },
    Fixture {
        name: "assembly",
        file: Some("sheetmetal/tray_asm.fg"),
        setup: Setup::None,
        tabs: ALL,
    },
    Fixture {
        name: "sheet",
        file: Some("sheetmetal/load_tray.fg"),
        setup: Setup::None,
        tabs: &["Sheet Metal", "Environments"],
    },
    Fixture {
        name: "cam",
        file: Some("machined/bracket_cam.fg"),
        setup: Setup::None,
        tabs: &["Manufacture"],
    },
    Fixture {
        name: "thermo",
        file: Some("thermoform/cover.fg"),
        setup: Setup::None,
        tabs: &["Environments"],
    },
    Fixture {
        name: "weld",
        file: Some("weldments/table_frame.fg"),
        setup: Setup::None,
        tabs: &["Weldments"],
    },
    Fixture {
        name: "harness",
        file: Some("harness/loom.fg"),
        setup: Setup::None,
        tabs: &["Harness"],
    },
    Fixture {
        name: "robot",
        file: Some("robot/arm2_robot.fg"),
        setup: Setup::None,
        tabs: DESIGN,
    },
    Fixture {
        name: "drawing",
        file: Some("PLATE"),
        setup: Setup::NewDrawing,
        tabs: ALL,
    },
];

/// What a click did, as a person would say it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Crash,
    Nothing,
    ErrorText,
    Dialog,
    Panel,
    Workspace,
    Mode,
    Action,
    Picker,
    Host,
    Disabled,
}

impl Class {
    fn word(self) -> &'static str {
        match self {
            Class::Crash => "CRASH",
            Class::Nothing => "NOTHING VISIBLE",
            Class::ErrorText => "ERROR TEXT",
            Class::Dialog => "DIALOG",
            Class::Panel => "PANEL",
            Class::Workspace => "WORKSPACE",
            Class::Mode => "MODE",
            Class::Action => "ACTION",
            Class::Picker => "FILE PICKER",
            Class::Host => "HOST",
            Class::Disabled => "DISABLED",
        }
    }
}

/// Commands that rightly answer a click with a mode and no popup: the sketcher's draw and
/// constrain tools (the next click on the sketch is the input; Inventor's are the same), the
/// view's navigation tools (the next drag is the input) and toggles drawn as switched on.
/// A mode outside this list is a finding: the click needs a prompt or a dialog.
pub fn mode_ok(f: &str, tab: &str, group: &str, tool: &str) -> bool {
    let sketching = f == "sketch" && (tab == "Sketch" || tab == "3D Model");
    sketching && matches!(group, "Create" | "Constrain" | "Modify" | "Pattern" | "Format")
        || matches!(
            group,
            "Navigate" | "Appearance" | "Windows" | "Visibility" | "Display" | "Show"
        )
        || matches!(tool, "Orbit" | "Pan" | "Zoom" | "Look At" | "Free Orbit")
}

pub struct Record {
    pub fixture: &'static str,
    pub tab: String,
    pub env: String,
    pub group: String,
    pub tool: String,
    pub class: Class,
    pub detail: String,
    pub findings: Vec<Finding>,
    pub shot: Option<String>,
    pub thumb: Option<String>,
    pub ms: u128,
}

impl Record {
    pub fn key(&self) -> String {
        format!("{}/{}{}/{}", self.fixture, self.env, self.tab, self.tool)
    }
}

struct Crawl {
    out: Option<PathBuf>,
    size: (f32, f32),
    records: Vec<Record>,
    /// Tabs already crawled (fixture-independent ones once).
    seen: BTreeSet<String>,
    budget: Option<usize>,
    noisy: bool,
}

fn copy_fixture(f: &Fixture, tag: &str) -> Option<PathBuf> {
    let file = f.file?;
    // each in a folder of its own inside one of the run's: a dialog that lists the files around
    // a part (Place) sees the example's neighbours, not other runs' folders
    let dir = std::env::temp_dir()
        .join(format!("fg-ui-audit-{}", std::process::id()))
        .join(f.name)
        .join(tag);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    if file == "PLATE" {
        let p = dir.join("plate.fg");
        std::fs::write(&p, PLATE).unwrap();
        return Some(p);
    }
    let src = Path::new(EXAMPLES).join(file);
    copy_dir(src.parent().unwrap(), &dir);
    let p = dir.join(src.file_name().unwrap());
    let _ = std::fs::remove_file(forge_lang::undo::log_path(&p));
    Some(p)
}

fn copy_dir(from: &Path, to: &Path) {
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_file() {
            let _ = std::fs::copy(&p, to.join(e.file_name()));
        } else if p.is_dir() {
            let d = to.join(e.file_name());
            std::fs::create_dir_all(&d).unwrap();
            copy_dir(&p, &d);
        }
    }
}

fn settled(h: &mut Headless, secs: f64) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs_f64(secs);
    while std::time::Instant::now() < end {
        step(h);
        let busy = h.gui.loading()
            || h.gui.fg.as_ref().is_some_and(|s| s.done != s.seq)
            || h.gui.view.as_ref().is_some_and(|v| v.nav.moving())
            || h.gui.shell.drawing.as_ref().is_some_and(|d| d.busy);
        if !busy {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    // past a window's fade-in
    for _ in 0..10 {
        step(h);
    }
}

/// A fresh window on the fixture, in the state its crawl starts from.
fn start(f: &Fixture, size: (f32, f32)) -> Option<Headless> {
    let mut h = match Headless::new(size, 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.gui.audit_shapes = Some(Vec::new());
    step(&mut h);
    if let Some(p) = copy_fixture(f, "run") {
        h.gui.open(p.to_str().unwrap());
        let _ = h.until(|g| {
            g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.seq > 0 && s.done == s.seq)
                || g.shell.status.contains("rror")
        });
    }
    match f.setup {
        Setup::None => {}
        Setup::EditSketch(id) => {
            h.gui.fg_edit_feature(id);
        }
        Setup::NewDrawing => {
            h.gui.shell.out.push(Action::NewDrawing);
            settled(&mut h, 10.0);
            // its dialog's OK (the dialog places no names: found by its words)
            let ok = Snap::take(&mut h)
                .texts
                .iter()
                .find(|t| t.s.trim() == "OK")
                .map(|t| t.rect.center());
            if let Some(c) = ok {
                h.click_at(c.x, c.y);
            }
            let _ = h.until(|g| g.shell.drawing.as_ref().is_some_and(|d| !d.busy));
        }
    }
    settled(&mut h, 60.0);
    park(&mut h);
    Some(h)
}

/// The pointer somewhere it hovers nothing: the title bar's empty stretch.
fn park(h: &mut Headless) {
    let w = h.gui.shell.view_rect.right().max(400.0);
    h.move_to(w * 0.3, 18.0);
    step(h);
    step(h);
}

fn tab_names(h: &mut Headless) -> Vec<String> {
    // those behind » too: its menu opened to read them, and closed
    if h.gui.shell.place("tabs:more").is_some() && click_place(h, "tabs:more") {
        step(h);
        let names = tab_names_placed(h);
        h.key(egui::Key::Escape);
        step(h);
        step(h);
        return names;
    }
    tab_names_placed(h)
}

fn tab_names_placed(h: &Headless) -> Vec<String> {
    let mut v: Vec<(f32, String)> = h
        .gui
        .shell
        .places
        .iter()
        .filter_map(|(k, r)| k.strip_prefix("tab:").map(|n| (r.left(), n.to_string())))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v.dedup_by(|a, b| a.1 == b.1);
    v.into_iter().map(|x| x.1).collect()
}

fn click_place(h: &mut Headless, name: &str) -> bool {
    let Some(r) = h.placed(name) else { return false };
    h.move_to(r.center().x, r.center().y);
    step(h);
    h.button(true);
    step(h);
    h.button(false);
    step(h);
    true
}

fn go_tab(h: &mut Headless, tab: &str) -> bool {
    let name = format!("tab:{tab}");
    // a narrow window keeps some tabs behind » (shell/mod.rs tabs_row)
    if h.gui.shell.place(&name).is_none() && click_place(h, "tabs:more") {
        step(h);
    }
    if !click_place(h, &name) {
        return false;
    }
    settled(h, 10.0);
    park(h);
    true
}

/// How a command is reached: a ribbon tool, or an item of a menu opened by a click (or a right
/// click) on a placed name.
#[derive(Clone)]
enum Click {
    Tool,
    /// A button the shell placed by name, in a panel already open.
    Place(String),
    Menu {
        opener: String,
        right: bool,
        item: String,
    },
}

fn right_click_place(h: &mut Headless, name: &str) -> bool {
    let Some(r) = h.placed(name) else { return false };
    h.move_to(r.center().x, r.center().y);
    step(h);
    h.right_click_here();
    step(h);
    true
}

fn open_menu(h: &mut Headless, opener: &str, right: bool) -> bool {
    let ok = if right {
        right_click_place(h, opener)
    } else {
        click_place(h, opener)
    };
    for _ in 0..3 {
        step(h);
    }
    ok
}

fn do_click(h: &mut Headless, how: &Click, group: &str, tool: &str) -> bool {
    match how {
        Click::Tool => click_tool(h, group, tool),
        Click::Place(name) => {
            let tip = h
                .gui
                .shell
                .place(name)
                .map(|r| hover_tip(h, r))
                .unwrap_or_default();
            TIP.with(|t| *t.borrow_mut() = tip);
            click_place(h, name)
        }
        Click::Menu { opener, right, item } => {
            if !open_menu(h, opener, *right) {
                return false;
            }
            // what the item's hover says first: an item drawn off says why there, and its click
            // then rightly does nothing
            let tip = h
                .gui
                .shell
                .place(item)
                .map(|r| hover_tip(h, r))
                .unwrap_or_default();
            TIP.with(|t| *t.borrow_mut() = tip);
            click_place(h, item)
        }
    }
}

/// Clicks a ribbon tool, through its folded group's menu when the window is too narrow for it.
fn click_tool(h: &mut Headless, group: &str, tool: &str) -> bool {
    if h.gui.shell.place(&format!("tool:{tool}")).is_some() && click_place(h, &format!("tool:{tool}")) {
        return true;
    }
    // folded: its group's menu, opened (a click may close one left open, so twice at most)
    for _ in 0..2 {
        if !click_place(h, &format!("ribbon:group:{group}")) {
            return false;
        }
        step(h);
        if h.gui.shell.place(&format!("tool:{tool}")).is_some() {
            return click_place(h, &format!("tool:{tool}"));
        }
    }
    false
}

impl Crawl {
    fn write(&self, name: &str, bytes: &[u8]) -> Option<String> {
        let dir = self.out.as_ref()?;
        let p = dir.join("shots").join(name);
        std::fs::create_dir_all(p.parent().unwrap()).ok()?;
        std::fs::write(&p, bytes).ok()?;
        Some(format!("shots/{name}"))
    }

    fn fixture(&mut self, f: &Fixture, only_tab: Option<&str>) {
        let Some(mut h) = start(f, self.size) else { return };
        let tabs: Vec<String> = tab_names(&mut h)
            .into_iter()
            .filter(|t| f.tabs.is_empty() || f.tabs.contains(&t.as_str()))
            .filter(|t| only_tab.is_none_or(|o| o == t))
            .collect();
        if let Some(dir) = &self.out {
            let _ = std::fs::create_dir_all(dir.join("shots"));
            if let Some(png) = Snap::take(&mut h).png() {
                self.write(&format!("{}--base.png", f.name), &png);
            }
        }
        if only_tab.is_none() || only_tab == Some("menus") {
            h = self.menus(f, h);
        }
        if only_tab.is_none() || only_tab == Some("side") {
            h = self.side_panels(f, h);
        }
        for tab in tabs {
            // View, Inspect and Manage are the same everywhere: once per kind of document
            let common = matches!(tab.as_str(), "View" | "Inspect" | "Manage");
            let key = if common {
                format!("{}:{}", kind_of(f), tab)
            } else {
                format!("{}:{}", f.name, tab)
            };
            if !self.seen.insert(key) {
                continue;
            }
            if tab == "Environments" {
                h = self.environments(f, h);
            } else {
                h = self.tab(f, h, &tab, "");
            }
        }
    }

    /// Every tool of a tab, each from the tab's own starting state.
    fn tab(&mut self, f: &Fixture, mut h: Headless, tab: &str, env: &str) -> Headless {
        if !go_tab(&mut h, tab) {
            eprintln!("{}: no tab {tab}", f.name);
            return h;
        }
        let tools = h.gui.shell.ribbon_live();
        let base_env = h.gui.shell.env;
        // a view that changes by itself (a render converging, a simulation playing) can't say
        // whether a click changed it
        let a = Snap::take(&mut h);
        for _ in 0..20 {
            step(&mut h);
        }
        let b = Snap::take(&mut h);
        let view = Rect::from_min_max(pos2(0.0, 150.0), pos2(self.size.0, self.size.1 - 28.0));
        self.noisy = b.changed(&a, view).0 > 200;
        for (group, tool, live) in tools {
            if self.budget == Some(0) {
                break;
            }
            if let Some(b) = &mut self.budget {
                *b -= 1;
            }
            let (rec, fresh) = self.probe(f, h, tab, env, group, tool, live, base_env, &Click::Tool);
            eprintln!(
                "  {:<14} {:<30} {:<16} {}",
                f.name,
                format!("{tab}/{tool}"),
                rec.class.word(),
                rec.detail
            );
            self.records.push(rec);
            h = match fresh {
                Some(h) => h,
                None => self
                    .reenter(f, tab, base_env)
                    .unwrap_or_else(|| panic!("{}: can't start again", f.name)),
            };
        }
        h
    }

    /// The File menu and the browser rows' right-click menus (a sketch's and a feature's): each
    /// item as its menu places it.
    fn menus(&mut self, f: &Fixture, mut h: Headless) -> Headless {
        let mut openers: Vec<(String, String, bool, &'static str)> = Vec::new();
        if self.seen.insert(format!("{}:File", kind_of(f))) {
            openers.push(("File".into(), "file".into(), false, "file-menu:"));
        }
        let rows: Vec<String> = h
            .gui
            .shell
            .places
            .iter()
            .filter_map(|(k, _)| k.strip_prefix("fg:row:").map(|x| x.to_string()))
            .collect();
        let kinds = |h: &Headless, key: &str| {
            h.gui
                .shell
                .fg
                .as_ref()
                .and_then(|p| p.feats.iter().find(|x| x.id == key).map(|x| x.kind.clone()))
        };
        let mut took = BTreeSet::new();
        for key in rows {
            let k = kinds(&h, &key).unwrap_or_else(|| "row".into());
            if took.insert(k.clone()) && took.len() <= 4 {
                openers.push((
                    format!("Browser: {k} row"),
                    format!("fg:row:{key}"),
                    true,
                    "fg:menu:",
                ));
            }
        }
        for (menu, opener, right, prefix) in openers {
            let before = Snap::take(&mut h);
            if !open_menu(&mut h, &opener, right) {
                continue;
            }
            let items: Vec<String> = Snap::take(&mut h)
                .new_places(&before)
                .into_iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, _)| k)
                .collect();
            h.key(egui::Key::Escape);
            settled(&mut h, 2.0);
            park(&mut h);
            h = restore(h, &before, h_env(&before), "", false)
                .unwrap_or_else(|| start(f, self.size).unwrap());
            let base_env = before.env;
            for item in items {
                let label = item
                    .trim_start_matches(prefix)
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .to_string();
                let how = Click::Menu {
                    opener: opener.clone(),
                    right,
                    item: item.clone(),
                };
                let (rec, fresh) = self.probe(
                    f,
                    h,
                    &format!("menu {menu}"),
                    "",
                    &menu,
                    &label,
                    true,
                    base_env,
                    &how,
                );
                eprintln!(
                    "  {:<14} {:<30} {:<16} {}",
                    f.name,
                    format!("{menu}/{label}"),
                    rec.class.word(),
                    rec.detail
                );
                self.records.push(rec);
                h = match fresh {
                    Some(h) => h,
                    None => start(f, self.size).unwrap(),
                };
            }
        }
        h
    }

    /// The side panel's tabs (Model, Agent, Versions): each opened, then every button it places
    /// that the Model tab's browser doesn't (its rows are the menus' business), clicked from it.
    fn side_panels(&mut self, f: &Fixture, mut h: Headless) -> Headless {
        let side = |h: &Headless| h.gui.shell.view_rect.left();
        let tabs: Vec<String> = h
            .gui
            .shell
            .places
            .iter()
            .filter_map(|(k, _)| k.strip_prefix("side:tab:").map(|x| x.to_string()))
            .collect();
        let base = Snap::take(&mut h);
        for tab in tabs {
            if tab == "Model" || !self.seen.insert(format!("{}:side:{tab}", kind_of(f))) {
                continue;
            }
            let opener = format!("side:tab:{tab}");
            let open = |h: &mut Headless| {
                click_place(h, &opener);
                settled(h, 5.0);
                park(h);
            };
            open(&mut h);
            let now = Snap::take(&mut h);
            let edge = side(&h);
            let items: Vec<String> = now
                .places
                .iter()
                .filter(|(k, r)| {
                    r.center().x < edge
                        && r.area() < 40_000.0
                        && !base.places.contains_key(*k)
                        && !["fg:", "side:tab:", "doc:"].iter().any(|p| k.starts_with(p))
                })
                .map(|(k, _)| k.clone())
                .collect();
            eprintln!("  side {tab}: {} buttons", items.len());
            for item in items {
                let label = item.clone();
                let how = Click::Place(item);
                let (rec, fresh) = self.probe(
                    f,
                    h,
                    &format!("side {tab}"),
                    "",
                    &tab,
                    &label,
                    true,
                    base.env,
                    &how,
                );
                eprintln!(
                    "  {:<14} {:<30} {:<16} {}",
                    f.name,
                    format!("{tab}/{label}"),
                    rec.class.word(),
                    rec.detail
                );
                self.records.push(rec);
                h = match fresh {
                    Some(h) => h,
                    None => {
                        let mut h = start(f, self.size).unwrap();
                        open(&mut h);
                        h
                    }
                };
            }
            click_place(&mut h, "side:tab:Model");
            settled(&mut h, 5.0);
        }
        h
    }

    /// A fresh window on the fixture, back in the environment and tab a probe started from.
    fn reenter(&self, f: &Fixture, tab: &str, env: Option<EnvId>) -> Option<Headless> {
        let mut h = start(f, self.size)?;
        if let Some(e) = env {
            h.gui.shell.out.push(Action::EnterEnv(e));
            settled(&mut h, 30.0);
        }
        go_tab(&mut h, tab);
        Some(h)
    }

    /// Each environment's tile, then each of its tabs' tools, then its Finish.
    fn environments(&mut self, f: &Fixture, mut h: Headless) -> Headless {
        h = self.tab(f, h, "Environments", "");
        for id in EnvId::ALL {
            if id.available(&h.gui.shell).is_err() {
                continue;
            }
            h.gui.shell.out.push(Action::EnterEnv(id));
            settled(&mut h, 30.0);
            park(&mut h);
            if h.gui.shell.env != Some(id) {
                continue;
            }
            let key = format!("env:{}", id.name());
            if !self.seen.insert(key) {
                h.gui.shell.out.push(Action::ExitEnv);
                settled(&mut h, 10.0);
                continue;
            }
            for tab in tab_names(&mut h) {
                if matches!(tab.as_str(), "View" | "Inspect" | "Manage") {
                    continue;
                }
                h = self.tab(f, h, &tab, &format!("{}: ", id.name()));
                if h.gui.shell.env != Some(id) {
                    h.gui.shell.out.push(Action::EnterEnv(id));
                    settled(&mut h, 30.0);
                }
            }
            h.gui.shell.out.push(Action::ExitEnv);
            settled(&mut h, 10.0);
            go_tab(&mut h, "Environments");
        }
        h
    }

    /// One click: the record, and the window to go on with (None: start again).
    #[allow(clippy::too_many_arguments)]
    fn probe(
        &mut self,
        f: &Fixture,
        mut h: Headless,
        tab: &str,
        env: &str,
        group: &str,
        tool: &str,
        live: bool,
        base_env: Option<EnvId>,
        how: &Click,
    ) -> (Record, Option<Headless>) {
        let t0 = std::time::Instant::now();
        let mut rec = Record {
            fixture: f.name,
            tab: tab.to_string(),
            env: env.to_string(),
            group: group.to_string(),
            tool: tool.to_string(),
            class: Class::Nothing,
            detail: String::new(),
            findings: Vec::new(),
            shot: None,
            thumb: None,
            ms: 0,
        };
        let slug = slug(&format!("{}-{}{}-{}", f.name, env, tab, tool));
        rec.findings
            .extend(judge::label(tool, matches!(how, Click::Tool), true));
        if !live {
            rec.class = Class::Disabled;
            TIP_SHOT.with(|t| t.borrow_mut().take());
            let (why, tip) = disabled_tip(&mut h, group, tool);
            if tip && let Some(png) = TIP_SHOT.with(|t| t.borrow_mut().take()) {
                rec.thumb = self.write(&format!("{slug}.hover.png"), &png);
            }
            rec.detail = why.clone();
            if !tip {
                rec.findings.push(judge::finding(
                    "no-reason",
                    true,
                    format!("off, and its tooltip says no reason ({why})"),
                ));
            }
            park(&mut h);
            rec.ms = t0.elapsed().as_millis();
            return (rec, Some(h));
        }
        // the status line cleared first: a command that says what the one before said (Mirror
        // then Copy, both refused alike) is seen to speak
        h.gui.shell.status.clear();
        for _ in 0..2 {
            step(&mut h);
        }
        let before = Snap::take(&mut h);
        let pickers = forge_render::plat::PICKERS_ASKED.load(std::sync::atomic::Ordering::Relaxed);
        HOST.with(|x| x.borrow_mut().clear());
        TIP.with(|t| t.borrow_mut().clear());
        let clicked = catch_unwind(AssertUnwindSafe(|| {
            let hit = do_click(&mut h, how, group, tool);
            let mut host = Vec::new();
            for _ in 0..3 {
                host.extend(step_host(&mut h));
            }
            settled(&mut h, 20.0);
            park(&mut h);
            (hit, host, h)
        }));
        let (hit, host, mut h) = match clicked {
            Ok(x) => x,
            Err(e) => {
                rec.class = Class::Crash;
                rec.detail = panic_words(&e);
                rec.ms = t0.elapsed().as_millis();
                return (rec, None);
            }
        };
        if !hit {
            rec.class = Class::Nothing;
            rec.detail = "not placed on screen: nothing to click".into();
            rec.ms = t0.elapsed().as_millis();
            return (rec, Some(h));
        }
        let after = Snap::take(&mut h);
        if let Click::Menu { item, .. } = how
            && after.places.contains_key(item)
        {
            rec.class = Class::Disabled;
            let tip = TIP.with(|t| std::mem::take(&mut *t.borrow_mut()));
            rec.detail = if tip.is_empty() {
                rec.findings.push(judge::finding(
                    "no-reason",
                    true,
                    "off, and its hover says no reason",
                ));
                "off: the menu stays open, no reason on hover".into()
            } else {
                if let Some(png) = TIP_SHOT.with(|t| t.borrow_mut().take()) {
                    rec.thumb = self.write(&format!("{slug}.hover.png"), &png);
                }
                format!("off: {}", judge::short(&tip))
            };
            rec.ms = t0.elapsed().as_millis();
            let fresh = restore(h, &before, base_env, tab, self.noisy);
            return (rec, fresh);
        }
        let picked = forge_render::plat::PICKERS_ASKED.load(std::sync::atomic::Ordering::Relaxed) > pickers;
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(self.size.0, self.size.1));
        classify(f, &mut rec, &before, &after, &host, picked, screen, self.noisy);
        let tip = TIP.with(|t| std::mem::take(&mut *t.borrow_mut()));
        let tip_shot = TIP_SHOT.with(|t| t.borrow_mut().take());
        // an item that said why it's off, and whose click changed nothing that matters (no
        // popup, the document and environment as they were): drawn off, as STANDARD §7 asks
        let still = after.new_layers(&before).is_empty()
            && after.seq == before.seq
            && after.dirty == before.dirty
            && after.env == before.env
            && after.docs == before.docs;
        // a reason, not just a name: "Off: …", "… first", "isn't …" (a live button whose tooltip
        // only names it, and whose click does nothing, is dead)
        let reason = {
            let l = tip.to_lowercase();
            [
                "off", "first", "isn't", "can't", "cannot", "not ", "no ", "only", "needs", "select",
                "used by", "finish",
            ]
            .iter()
            .any(|w| l.contains(w))
        };
        if !tip.is_empty()
            && reason
            && still
            && matches!(rec.class, Class::Nothing | Class::Action | Class::Mode)
        {
            rec.class = Class::Disabled;
            rec.detail = format!("off: {}", judge::short(&tip));
            rec.findings
                .retain(|f| !matches!(f.rule, "nothing" | "silent-mode"));
        }
        if !tip.is_empty()
            && let Some(png) = tip_shot
        {
            let hover = self.write(&format!("{slug}.hover.png"), &png);
            if rec.thumb.is_none() {
                rec.thumb = hover;
            }
        }
        // the popup itself, judged; then whether Esc takes it away
        let popup: Vec<Rect> = popup_region(&before, &after);
        if matches!(rec.class, Class::Dialog | Class::Panel | Class::ErrorText) && !popup.is_empty() {
            rec.findings.extend(judge::popup(&after, &before, &popup, screen));
            let bound = popup.iter().copied().reduce(|a, b| a.union(b)).unwrap();
            if let Some(png) = after.crop_png(bound.expand(6.0), 520) {
                rec.thumb = self.write(&format!("{slug}.thumb.png"), &png);
            }
            let toast = popup.iter().all(|r| r.height() < 60.0);
            if toast {
                rec.detail = format!(
                    "a note: \"{}\"",
                    judge::new_texts_in(&after, &before, &popup)
                        .iter()
                        .map(|t| t.s.trim())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            if rec.class == Class::Dialog && !toast {
                if after.focus.is_none() || after.focus == before.focus {
                    rec.findings.push(judge::finding(
                        "no-focus",
                        false,
                        "nothing has the keyboard when it opens",
                    ));
                }
                let words: Vec<String> = judge::new_texts_in(&after, &before, &popup)
                    .iter()
                    .map(|t| t.s.trim().to_string())
                    .collect();
                let close_icon = after
                    .new_places(&before)
                    .iter()
                    .any(|(k, _)| k.ends_with(":close") || k.ends_with(":cancel"));
                let exits = close_icon
                    || words.iter().any(|w| {
                        matches!(
                            w.as_str(),
                            "Cancel" | "Close" | "×" | "✕" | "Done" | "OK" | "Finish"
                        )
                    });
                h.key(egui::Key::Escape);
                for _ in 0..8 {
                    step(&mut h);
                }
                let esc = Snap::take(&mut h);
                let still = popup_region(&before, &esc);
                if !still.is_empty() {
                    rec.findings.push(judge::finding(
                        if exits { "esc" } else { "no-exit" },
                        !exits,
                        if exits {
                            "Esc leaves it open"
                        } else {
                            "no Cancel or Close, and Esc leaves it open"
                        },
                    ));
                }
                order_words(&mut rec, &words);
            }
        }
        if let Some(png) = after.png() {
            rec.shot = self.write(&format!("{slug}.png"), &png);
        }
        rec.ms = t0.elapsed().as_millis();
        // back where it began, or start again
        let fresh = restore(h, &before, base_env, tab, self.noisy);
        (rec, fresh)
    }
}

fn h_env(s: &Snap) -> Option<EnvId> {
    s.env
}

fn kind_of(f: &Fixture) -> &'static str {
    match f.name {
        "start" => "start",
        "assembly" => "assembly",
        "cam" => "cam",
        "drawing" => "drawing",
        "sketch" => "sketch",
        _ => "part",
    }
}

fn order_words(rec: &mut Record, words: &[String]) {
    let has = |w: &str| words.iter().any(|x| x == w);
    if has("OK") && has("Cancel") {
        rec.detail.push_str(" [OK, Cancel]");
    }
}

fn slug(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            o.push(c.to_ascii_lowercase());
        } else if !o.ends_with('-') {
            o.push('-');
        }
    }
    o.trim_matches('-').to_string()
}

fn panic_words(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "panicked".into())
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

const ERROR_WORDS: &[&str] = &[
    "error:",
    "error!",
    "failed",
    "panic",
    "can't",
    "cannot",
    "unable",
    "refused",
    "invalid",
    "not found",
    "couldn't",
];

fn errorish(s: &str) -> bool {
    let l = s.to_lowercase();
    ERROR_WORDS.iter().any(|w| l.contains(w))
}

/// The new layers, and the panels and windows the shell placed by name, that the click opened.
fn popup_region(before: &Snap, after: &Snap) -> Vec<Rect> {
    let mut v: Vec<Rect> = after.new_layers(before).into_iter().map(|x| x.1).collect();
    for (k, r) in after.new_places(before) {
        if (k.starts_with("dialog")
            || k.starts_with("panel")
            || k.starts_with("window")
            || k.contains(":dialog")
            || k.contains(":panel"))
            && r.area() > 2000.0
            && !v.iter().any(|x| x.contains_rect(r))
        {
            v.push(r);
        }
    }
    v
}

#[allow(clippy::too_many_arguments)]
fn classify(
    f: &Fixture,
    rec: &mut Record,
    before: &Snap,
    after: &Snap,
    host: &[String],
    picked: bool,
    screen: Rect,
    noisy: bool,
) {
    let layers = after.new_layers(before);
    let places = after.new_places(before);
    let status_new = after.status != before.status && !after.status.trim().is_empty();
    let new_text: Vec<String> = after
        .texts
        .iter()
        .filter(|t| !before.texts.iter().any(|b| b.s == t.s))
        .map(|t| t.s.clone())
        .collect();
    let view = Rect::from_min_max(pos2(0.0, 150.0), pos2(screen.right(), screen.bottom() - 28.0));
    let ribbon = Rect::from_min_max(pos2(0.0, 0.0), pos2(screen.right(), 150.0));
    let (n_view, bb_view) = after.changed(before, view);
    let (n_rib, _) = after.changed(before, ribbon);
    let errs: Vec<&String> = new_text.iter().filter(|t| errorish(t)).collect();
    let status = if status_new {
        format!("status: \"{}\"", judge::short(&after.status))
    } else {
        String::new()
    };
    if picked {
        rec.class = Class::Picker;
        rec.detail = "the system's file picker".into();
    } else if !host.is_empty() {
        rec.class = Class::Host;
        rec.detail = format!("the window's host is asked: {}", host.join(", "));
    } else if after.env != before.env {
        rec.class = Class::Workspace;
        rec.detail = format!(
            "into {}",
            after.env.map_or("the design".into(), |e| e.name().to_string())
        );
    } else if after.docs != before.docs {
        rec.class = Class::Workspace;
        rec.detail = "a new document tab".into();
    } else if !layers.is_empty() {
        rec.class = Class::Dialog;
        let r = layers.iter().map(|x| x.1).reduce(|a, b| a.union(b)).unwrap();
        rec.detail = format!("{} popup(s), {:.0}x{:.0}", layers.len(), r.width(), r.height());
    } else if after.tab != before.tab {
        rec.class = Class::Panel;
        rec.detail = format!("switches to tab {}", after.tab);
    } else if places.iter().any(|(_, r)| r.area() > 4000.0)
        || (n_view > 20_000
            && bb_view.is_some_and(|b| {
                b.width() < screen.width() * 0.6
                    && b.height() > 300.0
                    && (b.left() < 400.0 || b.right() > screen.right() - 400.0)
            }))
    {
        rec.class = Class::Panel;
        let names: Vec<&str> = places
            .iter()
            .filter(|x| x.1.area() > 4000.0)
            .map(|x| x.0.as_str())
            .take(3)
            .collect();
        rec.detail = format!("panel {}", names.join(", "));
    } else if after.seq != before.seq || after.dirty != before.dirty {
        rec.class = Class::Action;
        rec.detail = format!("the document changed {status}");
    } else if status_new {
        rec.class = Class::Action;
        rec.detail = status.clone();
    } else if n_view > 200 && !noisy {
        rec.class = Class::Action;
        rec.detail = format!("the view changed ({n_view} px)");
    } else if n_view > 200 {
        rec.class = Class::Action;
        rec.detail = format!("the view changed ({n_view} px), but it changes by itself too");
        rec.findings.push(judge::finding(
            "unsure",
            false,
            "only the view changed, and the view animates by itself: no other sign the click was taken",
        ));
    } else if n_rib > 30 {
        rec.class = Class::Mode;
        rec.detail = "the button stays on; no prompt".into();
    } else {
        rec.class = Class::Nothing;
        rec.detail = "no popup, panel, status line or change in the view".into();
    }
    if !errs.is_empty() || (status_new && errorish(&after.status)) {
        let words = errs.first().map_or(after.status.clone(), |s| s.to_string());
        if matches!(rec.class, Class::Dialog | Class::Panel) {
            rec.findings
                .push(judge::finding("error-text", false, judge::short(&words)));
        } else {
            rec.class = Class::ErrorText;
            rec.detail = format!("\"{}\"", judge::short(&words));
        }
    }
    // what the bar draws is what's judged (it may say a path by its file's name)
    if status_new && let Some(why) = judge::debug_text(&after.status_drawn) {
        rec.findings.push(judge::finding(
            "raw-text",
            true,
            format!("status \"{}\" ({why})", judge::short(&after.status_drawn)),
        ));
    }
    if rec.class == Class::Mode {
        if status_new {
            rec.detail = format!("a mode, prompted: {status}");
        } else if !mode_ok(f.name, &rec.tab, &rec.group, &rec.tool) {
            rec.findings.push(judge::finding(
                "silent-mode",
                true,
                "a mode with no prompt, outside the allowed list",
            ));
        }
    }
    if rec.class == Class::Nothing {
        rec.findings.push(judge::finding(
            "nothing",
            true,
            "nothing a person can see happens",
        ));
    }
}

/// Hovers an off tool until its tooltip shows: whether it says why (more than its own name).
fn disabled_tip(h: &mut Headless, group: &str, tool: &str) -> (String, bool) {
    let name = format!("tool:{tool}");
    let Some(r) = h.gui.shell.place(&name) else {
        let _ = group;
        return ("not placed".into(), false);
    };
    let before = Snap::take(h);
    h.move_to(r.center().x, r.center().y);
    let mut words = Vec::new();
    for _ in 0..90 {
        step(h);
        let tips: Vec<(egui::LayerId, Rect)> = h.gui.ctx.memory(|m| {
            m.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|l| l.order == egui::Order::Tooltip)
                .filter_map(|l| m.area_rect(l.id).map(|r| (l, r)))
                .collect()
        });
        if !tips.is_empty() {
            // past the tooltip's fade-in
            for _ in 0..10 {
                step(h);
            }
            let s = Snap::take(h);
            TIP_SHOT.with(|t| *t.borrow_mut() = s.png());
            let rects: Vec<Rect> = tips.iter().map(|x| x.1).collect();
            words = judge::new_texts_in(&s, &before, &rects)
                .iter()
                .map(|t| t.s.trim().to_string())
                .collect();
            if words.len() > 1 || words.iter().any(|w| w != tool) {
                break;
            }
        }
    }
    let said: Vec<&String> = words.iter().filter(|w| *w != tool && !w.is_empty()).collect();
    let why = said
        .iter()
        .map(|s| judge::short(s))
        .collect::<Vec<_>>()
        .join(" / ");
    (
        if why.is_empty() { "no tooltip".into() } else { why },
        !said.is_empty(),
    )
}

/// Esc, Cancel, out of an environment, the tab again: back to where the probe began, or None
/// when it isn't (a fresh window is then made).
fn restore(
    mut h: Headless,
    before: &Snap,
    base_env: Option<EnvId>,
    tab: &str,
    noisy: bool,
) -> Option<Headless> {
    let sig = before.signature();
    // and drawn the same: a toggle left on (Section, Shadows) is caught here
    let same = |now: &Snap| {
        let whole = Rect::from_min_size(pos2(0.0, 0.0), vec2(now.w as f32, now.h as f32));
        let view = Rect::from_min_max(pos2(0.0, 150.0), pos2(now.w as f32, now.h as f32 - 28.0));
        let n = if noisy {
            now.changed(before, whole).0 - now.changed(before, view).0
        } else {
            now.changed(before, whole).0
        };
        now.signature() == sig && n < 150
    };
    let ok = catch_unwind(AssertUnwindSafe(|| {
        for round in 0..3 {
            let now = Snap::take(&mut h);
            if same(&now) {
                return Some(h);
            }
            if now.seq != before.seq || now.docs != before.docs || now.dirty != before.dirty {
                return None;
            }
            match round {
                0 => {
                    h.key(egui::Key::Escape);
                    h.gui.shell.out.push(Action::DialogCancel);
                    h.gui.shell.file_menu = false;
                }
                1 => {
                    if h.gui.shell.env != base_env {
                        h.gui.shell.out.push(Action::ExitEnv);
                        step(&mut h);
                        if let Some(e) = base_env {
                            h.gui.shell.out.push(Action::EnterEnv(e));
                        }
                    }
                }
                _ => {
                    if !tab.is_empty() && !tab.starts_with("menu ") {
                        let _ = go_tab(&mut h, tab);
                    }
                }
            }
            for _ in 0..6 {
                step(&mut h);
            }
            park(&mut h);
        }
        let now = Snap::take(&mut h);
        same(&now).then_some(h)
    }));
    ok.ok().flatten()
}

fn tsv(s: &str) -> String {
    s.replace(['\t', '\n'], " ")
}

fn write_records(dir: &Path, recs: &[Record]) {
    let mut o = String::from("fixture\tenv\ttab\tgroup\ttool\tclass\tdetail\tfindings\tshot\tthumb\tms\n");
    for r in recs {
        let finds: Vec<String> = r
            .findings
            .iter()
            .map(|f| format!("{}{}: {}", if f.hard { "!" } else { "?" }, f.rule, f.what))
            .collect();
        o.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.fixture,
            tsv(r.env.trim_end_matches(": ")),
            tsv(&r.tab),
            tsv(&r.group),
            tsv(&r.tool),
            r.class.word(),
            tsv(r.detail.trim()),
            tsv(&finds.join(" | ")),
            r.shot.clone().unwrap_or_default(),
            r.thumb.clone().unwrap_or_default(),
            r.ms
        ));
    }
    std::fs::write(dir.join("records.tsv"), o).unwrap();
}

/// Findings already known (and on their way to being fixed): `KEY<TAB>RULE` lines; a finding
/// not in it fails the smoke test.
fn known() -> BTreeSet<String> {
    include_str!("known.txt")
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// On a thread with room: a debug build's frames are deep.
fn run(
    fixtures: &[&Fixture],
    only_tab: Option<&str>,
    size: (f32, f32),
    budget: Option<usize>,
) -> Vec<Record> {
    let fixtures: Vec<Fixture> = fixtures.iter().map(|f| **f).collect();
    let only_tab = only_tab.map(|s| s.to_string());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let fs: Vec<&Fixture> = fixtures.iter().collect();
            run_here(&fs, only_tab.as_deref(), size, budget)
        })
        .unwrap()
        .join()
        .unwrap()
}

fn run_here(
    fixtures: &[&Fixture],
    only_tab: Option<&str>,
    size: (f32, f32),
    budget: Option<usize>,
) -> Vec<Record> {
    forge_render::plat::NO_PICKER.store(true, std::sync::atomic::Ordering::Relaxed);
    let out = std::env::var_os("UI_AUDIT_OUT").map(PathBuf::from);
    let mut c = Crawl {
        out: out.clone(),
        size,
        records: Vec::new(),
        seen: BTreeSet::new(),
        budget,
        noisy: false,
    };
    for f in fixtures {
        eprintln!("== {}", f.name);
        // a panic the crawl can't step past (starting again into one) ends this document's
        // crawl, not the run: what it found so far is kept
        if let Err(e) = catch_unwind(AssertUnwindSafe(|| c.fixture(f, only_tab))) {
            eprintln!("{}: the crawl stopped: {}", f.name, panic_words(&e));
        }
        // kept as it goes: a crawl killed later loses only the document it was in
        if let Some(dir) = &out {
            std::fs::create_dir_all(dir).unwrap();
            write_records(dir, &c.records);
        }
    }
    if let Some(dir) = &out {
        std::fs::create_dir_all(dir).unwrap();
        write_records(dir, &c.records);
    }
    c.records
}

fn verdict(recs: &[Record]) -> Vec<String> {
    let known = known();
    let mut bad = Vec::new();
    for r in recs {
        for f in &r.findings {
            if !f.hard {
                continue;
            }
            let line = format!("{}\t{}", r.key(), f.rule);
            if !known.contains(&line) {
                bad.push(format!("{line}\t{}", f.what));
            }
        }
        if matches!(r.class, Class::Crash) {
            bad.push(format!("{}\tcrash\t{}", r.key(), r.detail));
        }
    }
    bad
}

/// The CI set: the plate's 3D Model tab (every tool, its dialogs judged) and the start screen.
#[test]
fn smoke_every_3d_model_tool_shows_something() {
    if std::env::var("UI_AUDIT").is_ok_and(|v| v == "full") {
        return;
    }
    let f = FIXTURES.iter().find(|f| f.name == "plate").unwrap();
    let recs = run(&[f], Some("3D Model"), (1400.0, 860.0), None);
    if recs.is_empty() {
        return; // no GPU: skipped
    }
    let bad = verdict(&recs);
    assert!(
        bad.is_empty(),
        "new UI audit failures (add to tests/ui_audit/known.txt only with a fix under way):\n{}",
        bad.join("\n")
    );
}

/// The whole crawl: `UI_AUDIT=full UI_AUDIT_OUT=DIR` (tools/ui-audit/crawl.sh). `UI_AUDIT_ONLY`
/// names fixtures, comma-separated; `UI_AUDIT_SIZE=WxH` another window size.
#[test]
fn full_crawl() {
    if std::env::var("UI_AUDIT").is_ok_and(|v| v == "full") {
        let only: Option<Vec<String>> = std::env::var("UI_AUDIT_ONLY")
            .ok()
            .map(|s| s.split(',').map(|x| x.to_string()).collect());
        let fs: Vec<&Fixture> = FIXTURES
            .iter()
            .filter(|f| only.as_ref().is_none_or(|o| o.iter().any(|n| n == f.name)))
            .collect();
        let size = std::env::var("UI_AUDIT_SIZE")
            .ok()
            .and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?))
            })
            .unwrap_or((1400.0, 860.0));
        let budget = std::env::var("UI_AUDIT_BUDGET").ok().and_then(|s| s.parse().ok());
        let recs = run(&fs, std::env::var("UI_AUDIT_TAB").ok().as_deref(), size, budget);
        let mut counts = std::collections::BTreeMap::new();
        for r in &recs {
            *counts.entry(r.class.word()).or_insert(0) += 1;
        }
        eprintln!("{counts:?}");
    }
}
