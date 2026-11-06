//! Thermoforming in the app, as Inventor would have it: Convert to Thermoform on the 3D Model tab,
//! then a Thermoform tab (only while the part has one) of three steps: Edit Formed Part, Check
//! Forming (one results window: the wall's map, its extremes, draft, probes, the forming played,
//! the CSV), Make Mould (its settings and the exports). The browser holds the formed part, the
//! mould and the blank, each with its eye; the formed part shows as modelled (t everywhere) or as
//! simulated (the wall the model gives), its colours on or off in either. Every edit is a dialog.
use super::fg::{FgAct, FgPanel, Severity};
use super::theme::{Theme, medium, mono, sans, semi};
use super::thermo::{Masses, ThermoRow, band, c32};
use super::{Action, FgCmd, Shell};
use crate::shell::placed::Placed;
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_lang::error::Op;

/// A thermoform command, from the ribbon, the browser or a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Convert,
    Form,
    Trim,
    Plug,
    Check,
    Mould,
    Vents,
    /// The formed part as simulated, or as modelled.
    Simulated,
    /// The wall's colours on the part.
    Colours,
    ShowPart,
    ShowMould,
    ShowBlank,
}

/// What the part's faces are coloured by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Analysis {
    #[default]
    None,
    Draft,
    Thickness,
    /// The larger principal stretch over the sheet on the part.
    Stretch,
    /// Where the sheet gathers and folds onto itself.
    Webbing,
}

/// What of the thermoform shows: its eyes in the browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Show {
    pub part: bool,
    pub mould: bool,
    pub blank: bool,
}

impl Default for Show {
    fn default() -> Self {
        Show {
            part: true,
            mould: true,
            blank: false,
        }
    }
}

/// The thermoform's dialogs and view: none of it is saved in the part.
#[derive(Clone, Debug, Default)]
pub struct ThermoUi {
    pub convert: Option<Convert>,
    pub form: Option<Form>,
    pub mould: Option<Mould>,
    pub check: bool,
    pub show: Show,
    /// The formed part as the model leaves it, its wall rebuilt at the model's thickness.
    pub simulated: bool,
    pub analysis: Analysis,
    /// The draft analysis's least draft, degrees (None: the material's for the mould).
    pub draft_min: Option<f32>,
    /// The step of the forming shown (None: the last), and whether it plays.
    pub step: Option<usize>,
    pub playing: bool,
    pub probe: bool,
    /// Probes pinned on the part: where, and the thickness there.
    pub probes: Vec<([f32; 3], f64)>,
    /// The issues opened from the status bar.
    pub issues: bool,
    /// Fixes re-simulated: the fix's op as JSON text, and the thinnest wall it gives (None: running).
    pub resim: Vec<(String, Option<f64>)>,
    /// Asked of the host: re-simulate these fixes.
    pub want_resim: Vec<Op>,
    /// Asked of the host: an export of the thermoform (its id, what).
    pub want_export: Option<(String, Export)>,
    pub said: Option<String>,
    /// A material opened from Convert's "…": its record, and where saving it writes.
    pub material: Option<forge_material::Material>,
    /// Converted, the Thermoform tab is wanted before the build has the feature.
    pub converted: bool,
    /// What the host drew the part's mesh for last (`view_key`).
    pub drawn: Option<usize>,
    /// The body looked at, chosen by its row in the browser (it shows alone, as sheet metal's
    /// Folded Model and Flat Pattern rows switch), and the one under the pointer there.
    pub picked: Option<Body>,
    pub hover: Option<Body>,
    /// What the host marked in the view last (`picked`, `hover`).
    pub marked: Option<(Option<Body>, Option<Body>)>,
    /// An assembly's part files with a thermoform, and the one whose results are up (the
    /// selected occurrence's).
    pub asm_files: Vec<String>,
    pub asm_part: Option<String>,
    /// Asked of the host: open this part file and run this there (an assembly edits its parts
    /// in their own files).
    pub want_open: Option<(String, Cmd)>,
}

/// One of a thermoform's bodies, as the browser has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Part,
    Mould,
    Blank,
}

impl ThermoUi {
    /// What the part's mesh depends on of this: its eyes and its state.
    pub fn view_key(&self) -> usize {
        usize::from(self.shows(Body::Part))
            | usize::from(self.shows(Body::Mould)) << 1
            | usize::from(self.shows(Body::Blank)) << 2
            | usize::from(self.simulated) << 3
    }

    /// Whether a body is on view: the one looked at alone, else as its eye says.
    pub fn shows(&self, b: Body) -> bool {
        match self.picked {
            Some(p) => p == b,
            None => match b {
                Body::Part => self.show.part,
                Body::Mould => self.show.mould,
                Body::Blank => self.show.blank,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Export {
    MouldStep,
    MouldStl,
    PartStep,
    PartStl,
    TrimDxf,
    ProcessPdf,
    ThicknessCsv,
    /// The flat sheet's print that forms into an artwork (asks for the artwork).
    DistortionPrint,
}

/// A material as Convert offers it: its name, its law's source, its forming window and normal
/// temperature, the temperature its law was fitted at.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MaterialRow {
    pub name: String,
    pub law: String,
    pub window: (f64, f64),
    pub normal: f64,
    pub fitted: Option<f64>,
}

const FROM: [(&str, &str, &str); 3] = [
    (
        "outer",
        "Outer shape",
        "Solid. Its outside is the part's outside; shelled inward by t.",
    ),
    ("wall", "Wall", "Already shelled. t is read from the wall."),
    (
        "inner",
        "Inner shape",
        "Solid. Its outside is the part's inside; sheet goes outward.",
    ),
];
const GAUGES: [f64; 10] = [0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0];
const PULLS: [(&str, &str); 6] = [
    ("Z", "+Z (up)"),
    ("-Z", "−Z"),
    ("X", "+X"),
    ("-X", "−X"),
    ("Y", "+Y"),
    ("-Y", "−Y"),
];

/// The Convert to Thermoform dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct Convert {
    pub of: String,
    pub from: usize,
    pub female: bool,
    /// The open face's anchor text (empty: the shape's own foot).
    pub open: String,
    pub material: String,
    pub t: f64,
    pub temp: f64,
}

/// The Edit Formed Part dialog: the thermoform's arguments as written, edited.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub id: String,
    pub tab: usize,
    pub args: Vec<(String, String)>,
    pub was: Vec<(String, String)>,
}

/// The Make Mould dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct Mould {
    pub id: String,
    pub args: Vec<(String, String)>,
    pub was: Vec<(String, String)>,
    /// What the mould is made of or by: a note for the process sheet.
    pub made_by: usize,
}

const MADE_BY: [&str; 4] = [
    "CNC from MDF / tooling board",
    "3D printed",
    "Cast aluminium",
    "CNC aluminium",
];

/// A call's arguments as written: the first positional one under "shape", the rest by name.
pub fn args_of(src: &str, id: &str) -> Vec<(String, String)> {
    let head = format!("{id} = thermoform(");
    let Some(at) = src.find(&head) else { return vec![] };
    let body = &src[at + head.len()..];
    let mut depth = 0i32;
    let (mut out, mut cur) = (vec![], String::new());
    let push = |cur: &mut String, out: &mut Vec<(String, String)>| {
        let s = cur.trim().to_string();
        cur.clear();
        if s.is_empty() {
            return;
        }
        match s.split_once(':') {
            Some((k, v)) if k.trim().chars().all(|c| c.is_alphanumeric() || c == '_') => {
                out.push((k.trim().to_string(), v.trim().to_string()))
            }
            _ => out.push(("shape".into(), s)),
        }
    };
    for c in body.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' if depth == 0 => {
                push(&mut cur, &mut out);
                break;
            }
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                push(&mut cur, &mut out);
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out
}

fn arg<'a>(args: &'a [(String, String)], k: &str) -> Option<&'a str> {
    args.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
}

fn set(args: &mut Vec<(String, String)>, k: &str, v: String) {
    match args.iter_mut().find(|a| a.0 == k) {
        Some(a) => a.1 = v,
        None => args.push((k.into(), v)),
    }
}

/// The ops that turn `was` into `now` on statement `id`.
fn changes(id: &str, was: &[(String, String)], now: &[(String, String)]) -> Vec<Op> {
    now.iter()
        .filter(|(k, v)| k != "shape" && !v.is_empty() && arg(was, k) != Some(v.as_str()))
        .map(|(k, v)| Op::SetArg {
            id: id.into(),
            arg: k.clone(),
            text: v.clone(),
        })
        .collect()
}

/// The last feature that made or changed a solid: what Convert takes.
pub fn last_solid(fg: &FgPanel) -> Option<String> {
    fg.feats
        .iter()
        .rev()
        .find(|f| {
            !matches!(
                f.kind.as_str(),
                "sketch" | "plane" | "axis" | "point" | "param" | "thermoform" | "path"
            )
        })
        .map(|f| f.id.clone())
}

/// The statement Convert adds, named so as not to take a name the part has.
pub fn convert_text(fg: &FgPanel, c: &Convert) -> String {
    let taken = |n: &str| {
        fg.feats
            .iter()
            .any(|f| f.id == n || f.id.starts_with(&format!("{n}[")))
    };
    let name = (1..)
        .map(|k| {
            if k == 1 {
                "part".to_string()
            } else {
                format!("part{k}")
            }
        })
        .find(|n| !taken(n))
        .expect("a free name");
    let (from, ..) = FROM[c.from.min(2)];
    let mut a = format!("{name} = thermoform({}, from: \"{from}\"", c.of);
    if !c.open.trim().is_empty() {
        a += &format!(", open: {}", c.open.trim());
    }
    if from != "wall" {
        a += &format!(", t: {}", fmt(c.t));
    }
    a += &format!(
        ", tool: \"{}\", material: \"{}\"",
        if c.female { "female" } else { "male" },
        c.material
    );
    let m = fg.thermo_materials.iter().find(|m| m.name == c.material);
    if m.is_none_or(|m| (m.normal - c.temp).abs() > 1e-9) {
        a += &format!(", temperature: {}", fmt(c.temp));
    }
    a + ")\n"
}

fn fmt(x: f64) -> String {
    let s = format!("{x:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The solids Convert can take: each feature that made or changed one, last first.
pub fn solid_feats(fg: &FgPanel) -> Vec<String> {
    fg.feats
        .iter()
        .rev()
        .filter(|f| {
            !matches!(
                f.kind.as_str(),
                "sketch" | "plane" | "axis" | "point" | "param" | "thermoform" | "path" | "ucs"
            ) && !f.id.contains('.')
        })
        .map(|f| f.id.clone())
        .collect()
}

/// A ribbon button: its word, its icon, what it does (none: drawn disabled), whether it's on.
pub type Button = (&'static str, &'static str, Option<Action>, bool);

/// The 3D Model tab's Convert to Thermoform.
pub fn convert_button(s: &Shell) -> Button {
    let fg = s.fg.as_ref();
    (
        "Convert to Thermoform",
        "thicken",
        fg.filter(|f| last_solid(f).is_some())
            .map(|_| Action::Fg(FgCmd::Thermo(Cmd::Convert))),
        fg.is_some_and(|f| f.thermo.convert.is_some()),
    )
}

impl Shell {
    /// The part has a thermoform (or one is on its way), or the assembly a part with one: its
    /// tab shows.
    pub fn thermo_tab(&self) -> bool {
        let Some(f) = self.fg.as_ref() else { return false };
        if self.is_asm() {
            return !f.thermo.asm_files.is_empty();
        }
        f.thermo.converted || f.feats.iter().any(|x| x.kind == "thermoform")
    }
}

/// The ribbon's Thermoform tab: three steps, then the small things and the view.
pub fn groups(s: &Shell) -> Vec<(&'static str, Vec<Button>, Vec<Button>)> {
    let fg = s.fg.as_ref();
    let has = fg.is_some_and(|f| !f.thermos.is_empty());
    // an assembly's views are its parts' own: the part's state and colours are in its file
    let asm = s.is_asm();
    let modelled = !asm
        && fg
            .and_then(|f| f.thermos.first())
            .is_some_and(|r| r.simulated.is_some());
    let ui = fg.map(|f| &f.thermo);
    let go = |c: Cmd, ok: bool| ok.then_some(Action::Fg(FgCmd::Thermo(c)));
    let on = |f: &dyn Fn(&ThermoUi) -> bool| ui.is_some_and(f);
    vec![
        (
            "Formed Part",
            vec![(
                "Edit Formed\u{a0}Part",
                "thicken",
                go(Cmd::Form, has),
                on(&|u| u.form.as_ref().is_some_and(|f| f.tab == 0)),
            )],
            vec![],
        ),
        (
            "Check",
            vec![("Check Forming", "measure", go(Cmd::Check, has), on(&|u| u.check))],
            vec![],
        ),
        (
            "Mould",
            vec![(
                "Make Mould",
                "extrude",
                go(Cmd::Mould, has),
                on(&|u| u.mould.is_some()),
            )],
            vec![],
        ),
        (
            "Modify",
            vec![],
            vec![
                (
                    "Trim",
                    "trim",
                    go(Cmd::Trim, has),
                    on(&|u| u.form.as_ref().is_some_and(|f| f.tab == 1)),
                ),
                (
                    "Plug",
                    "extrude",
                    go(Cmd::Plug, has),
                    on(&|u| u.form.as_ref().is_some_and(|f| f.tab == 2)),
                ),
                ("Vents", "hole", go(Cmd::Vents, has), false),
            ],
        ),
        (
            "Display",
            vec![],
            vec![
                (
                    "Simulated Wall",
                    "shell",
                    go(Cmd::Simulated, modelled),
                    on(&|u| u.simulated),
                ),
                (
                    "Thickness Colours",
                    "draft",
                    go(Cmd::Colours, has && !asm),
                    on(&|u| u.analysis == Analysis::Thickness),
                ),
            ],
        ),
    ]
}

impl FgPanel {
    /// The first thermoform's row, the one the tab works on.
    pub fn thermo_row(&self) -> Option<&ThermoRow> {
        self.thermos.first()
    }

    /// Edit Formed Part, on a tab: up, or onto that tab, or (asked again) away.
    fn open_form(&mut self, tab: usize) {
        let Some(id) = self.thermos.first().map(|r| r.id.clone()) else {
            return;
        };
        match &mut self.thermo.form {
            Some(f) if f.tab == tab => self.thermo.form = None,
            Some(f) => f.tab = tab,
            None => {
                let args = args_of(&self.source, &id);
                self.thermo.form = Some(Form {
                    id,
                    tab,
                    was: args.clone(),
                    args,
                });
                self.thermo.convert = None;
            }
        }
    }

    pub fn thermo_command(&mut self, c: Cmd) {
        // in an assembly a part's thermoform is edited in the part's file
        if self.asm.is_some()
            && matches!(c, Cmd::Form | Cmd::Trim | Cmd::Plug | Cmd::Mould | Cmd::Vents)
            && let Some(f) = &self.thermo.asm_part
        {
            self.thermo.want_open = Some((f.clone(), c));
            return;
        }
        let first = self.thermos.first().map(|r| r.id.clone());
        match c {
            Cmd::Convert => {
                if self.thermo.convert.is_some() {
                    self.thermo.convert = None;
                    return;
                }
                let of = last_solid(self).unwrap_or_default();
                let m = self
                    .thermo_materials
                    .iter()
                    .find(|m| m.name == "HIPS")
                    .or(self.thermo_materials.first())
                    .cloned()
                    .unwrap_or_default();
                self.thermo.form = None;
                self.thermo.convert = Some(Convert {
                    of,
                    from: 0,
                    female: false,
                    open: String::new(),
                    material: m.name.clone(),
                    t: 2.0,
                    temp: m.normal,
                });
            }
            Cmd::Form => self.open_form(0),
            Cmd::Trim => self.open_form(1),
            Cmd::Plug => self.open_form(2),
            Cmd::Check => {
                let u = &mut self.thermo;
                u.check = !u.check;
                if u.check && u.analysis == Analysis::None && self.asm.is_none() {
                    u.analysis = Analysis::Thickness;
                }
            }
            Cmd::Mould | Cmd::Vents => {
                let Some(id) = first else { return };
                if self.thermo.mould.is_some() && c == Cmd::Mould {
                    self.thermo.mould = None;
                    return;
                }
                let args = args_of(&self.source, &id);
                self.thermo.mould = Some(Mould {
                    id,
                    was: args.clone(),
                    args,
                    made_by: 0,
                });
            }
            Cmd::Simulated => self.thermo.simulated = !self.thermo.simulated,
            Cmd::Colours => {
                let u = &mut self.thermo;
                u.analysis = if u.analysis == Analysis::Thickness {
                    Analysis::None
                } else {
                    Analysis::Thickness
                };
            }
            // an eye says what shows of them all again
            Cmd::ShowPart => {
                self.thermo.picked = None;
                self.thermo.show.part = !self.thermo.show.part;
            }
            Cmd::ShowMould => {
                self.thermo.picked = None;
                self.thermo.show.mould = !self.thermo.show.mould;
            }
            Cmd::ShowBlank => {
                self.thermo.picked = None;
                self.thermo.show.blank = !self.thermo.show.blank;
            }
        }
    }
}

// ---- the look: a dialog's frame, rows, buttons ----

pub(super) fn frame(t: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(6)
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(90),
        })
        .inner_margin(0)
}

/// A dialog's title bar: its name and a close cross. True when closed.
fn title(ui: &mut Ui, t: &Theme, text: &str, w: f32, places: &mut Vec<(String, Rect)>, key: &str) -> bool {
    let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let p = ui.painter();
    p.text(
        pos2(r.left() + 14.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        semi(14.0),
        t.ink,
    );
    p.hline(r.left()..=r.right(), r.bottom(), Stroke::new(1.0, t.line));
    let x = Rect::from_center_size(pos2(r.right() - 20.0, r.center().y), vec2(22.0, 22.0));
    let resp = ui.interact(x, ui.scope_id().with((key, "close")), Sense::click());
    if resp.hovered() {
        p.rect_filled(x, 3.0, t.hover);
    }
    p.text(x.center(), Align2::CENTER_CENTER, "×", sans(15.0), t.ink2);
    places.push((format!("{key}:close"), x));
    resp.clicked()
}

/// A labelled row; the widget is laid out in the row's right part.
pub(super) fn row<R>(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    w: f32,
    h: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 14.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink2,
    );
    let field = Rect::from_min_max(
        pos2(r.left() + 146.0, r.top() + 2.0),
        pos2(r.right() - 14.0, r.bottom() - 2.0),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(field)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    add(&mut child)
}

/// A button as the dialogs have them; `strong` is the OK one.
pub(super) fn button(
    ui: &mut Ui,
    t: &Theme,
    text: &str,
    strong: bool,
    key: String,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let b = egui::Button::new(
        egui::RichText::new(text)
            .font(if strong { medium(13.0) } else { sans(13.0) })
            .color(if strong { Color32::WHITE } else { t.ink }),
    )
    .fill(if strong {
        Color32::from_rgb(0x2f, 0x6f, 0x4f)
    } else {
        t.raised
    })
    .stroke(Stroke::new(
        1.0,
        if strong {
            Color32::from_rgb(0x2f, 0x6f, 0x4f)
        } else {
            t.line
        },
    ))
    .min_size(vec2(72.0, 30.0));
    let r = ui.add(b);
    places.push((key, r.rect));
    r.clicked()
}

/// A button across the dialog, its words from the left; `first` marks the one to take.
fn wide(
    ui: &mut Ui,
    t: &Theme,
    text: &str,
    w: f32,
    first: bool,
    key: String,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let g = ui.painter().layout(text.to_string(), sans(12.5), t.ink, w - 48.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, g.size().y + 14.0), Sense::click());
    let b = Rect::from_min_max(
        pos2(r.left() + 14.0, r.top() + 2.0),
        pos2(r.right() - 14.0, r.bottom() - 2.0),
    );
    let p = ui.painter();
    p.rect_filled(b, 3.0, if resp.hovered() { t.hover } else { t.raised });
    p.rect_stroke(
        b,
        3.0,
        Stroke::new(1.0, if first { t.focus } else { t.line }),
        StrokeKind::Inside,
    );
    p.galley(pos2(b.left() + 10.0, b.center().y - g.size().y / 2.0), g, t.ink);
    places.push((key, b));
    resp.clicked()
}

/// One of two or more choices, as a row of toggles.
fn toggles(
    ui: &mut Ui,
    t: &Theme,
    now: &mut usize,
    words: &[&str],
    key: &str,
    places: &mut Vec<(String, Rect)>,
) {
    for (k, w) in words.iter().enumerate() {
        let on = *now == k;
        let b = egui::Button::new(egui::RichText::new(*w).font(sans(13.0)).color(if on {
            t.on_ink
        } else {
            t.ink
        }))
        .fill(if on { t.on } else { t.raised })
        .stroke(Stroke::new(1.0, if on { t.focus } else { t.line }));
        let r = ui.add(b);
        places.push((format!("{key}:{k}"), r.rect));
        if r.clicked() {
            *now = k;
        }
    }
}

/// The three ways in, as cards with their picture.
fn cards(ui: &mut Ui, t: &Theme, now: &mut usize, places: &mut Vec<(String, Rect)>) {
    let (r, _) = ui.allocate_exact_size(vec2(DLG_W, 150.0), Sense::hover());
    let w = (DLG_W - 28.0 - 16.0) / 3.0;
    for (k, (_, name, words)) in FROM.iter().enumerate() {
        let c = Rect::from_min_size(
            pos2(r.left() + 14.0 + k as f32 * (w + 8.0), r.top()),
            vec2(w, 146.0),
        );
        let resp = ui.interact(c, ui.scope_id().with(("thermo-card", k)), Sense::click());
        let on = *now == k;
        let p = ui.painter();
        p.rect_filled(
            c,
            4.0,
            if on {
                t.on
            } else if resp.hovered() {
                t.hover
            } else {
                t.raised
            },
        );
        p.rect_stroke(
            c,
            4.0,
            Stroke::new(1.0, if on { t.focus } else { t.line }),
            StrokeKind::Inside,
        );
        // the section of what's modelled: filled for a solid, a line for a wall; the mould's side
        let (x0, y0) = (c.left() + 18.0, c.top() + 14.0);
        let pts = [
            pos2(x0, y0 + 44.0),
            pos2(x0 + 12.0, y0 + 4.0),
            pos2(w - 48.0 + x0, y0 + 4.0),
            pos2(w - 36.0 + x0, y0 + 44.0),
        ];
        let gold = Color32::from_rgb(0xe0, 0xa8, 0x3a);
        let blue = Color32::from_rgb(0x7f, 0xb2, 0xec);
        match k {
            0 => {
                p.add(egui::Shape::convex_polygon(
                    pts.to_vec(),
                    Color32::from_rgb(0xd9, 0xe0, 0xe8),
                    Stroke::NONE,
                ));
                p.add(egui::Shape::line(pts.to_vec(), Stroke::new(3.0, gold)));
            }
            1 => {
                p.add(egui::Shape::line(
                    pts.to_vec(),
                    Stroke::new(4.0, Color32::from_rgb(0xd9, 0xe0, 0xe8)),
                ));
            }
            _ => {
                p.add(egui::Shape::convex_polygon(
                    pts.to_vec(),
                    Color32::from_rgb(0xd9, 0xe0, 0xe8),
                    Stroke::NONE,
                ));
                p.add(egui::Shape::line(pts.to_vec(), Stroke::new(3.0, blue)));
            }
        }
        p.text(
            pos2(c.left() + 9.0, c.top() + 74.0),
            Align2::LEFT_CENTER,
            *name,
            semi(13.0),
            t.ink,
        );
        let g = p.layout(words.to_string(), sans(11.0), t.ink2, w - 16.0);
        p.galley(pos2(c.left() + 9.0, c.top() + 86.0), g, t.ink2);
        places.push((format!("thermo:from:{k}"), c));
        if resp.clicked() {
            *now = k;
        }
    }
}

pub(super) const DLG_W: f32 = 446.0;

/// A dialog's head, Inventor's: the command's icon and name, a close cross. True when closed.
pub(super) fn head(
    ui: &mut Ui,
    t: &Theme,
    icon: &str,
    text: &str,
    w: f32,
    places: &mut Vec<(String, Rect)>,
    key: &str,
) -> bool {
    let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let p = ui.painter();
    super::icons::draw(
        p,
        pos2(r.left() + 12.0, r.center().y - 9.0),
        18.0,
        icon,
        1.5,
        t.ink,
    );
    p.text(
        pos2(r.left() + 38.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        semi(14.0),
        t.ink,
    );
    p.hline(r.left()..=r.right(), r.bottom(), Stroke::new(1.0, t.line));
    let x = Rect::from_center_size(pos2(r.right() - 20.0, r.center().y), vec2(22.0, 22.0));
    let resp = ui.interact(x, ui.scope_id().with((key, "close")), Sense::click());
    if resp.hovered() {
        p.rect_filled(x, 3.0, t.hover);
    }
    p.text(x.center(), Align2::CENTER_CENTER, "×", sans(15.0), t.ink2);
    places.push((format!("{key}:close"), x));
    resp.clicked()
}

/// A dialog's tabs; `now` the one up.
pub(super) fn tabs(
    ui: &mut Ui,
    t: &Theme,
    names: &[&str],
    now: &mut usize,
    w: f32,
    places: &mut Vec<(String, Rect)>,
    key: &str,
) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
    let mut x = r.left() + 6.0;
    for (k, name) in names.iter().enumerate() {
        let g = ui.painter().layout_no_wrap(name.to_string(), sans(13.0), t.ink);
        let tab = Rect::from_min_size(pos2(x, r.top()), vec2(g.size().x + 26.0, 32.0));
        let resp = ui.interact(tab, ui.scope_id().with((key, k)), Sense::click());
        if *now == k {
            ui.painter().rect_filled(tab, 0.0, t.raised);
            ui.painter().hline(
                tab.left()..=tab.right(),
                tab.bottom() - 1.0,
                Stroke::new(2.0, t.focus),
            );
        }
        let ink = if *now == k { t.ink } else { t.ink2 };
        ui.painter()
            .galley(pos2(tab.left() + 13.0, tab.center().y - g.size().y / 2.0), g, ink);
        places.push((format!("{key}:{k}"), tab));
        if resp.clicked() {
            *now = k;
        }
        x = tab.right();
    }
    ui.painter()
        .hline(r.left()..=r.right(), r.bottom(), Stroke::new(1.0, t.line));
}

/// A section's heading inside a dialog.
pub(super) fn heading(ui: &mut Ui, t: &Theme, text: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 14.0, r.bottom() - 6.0),
        Align2::LEFT_BOTTOM,
        text,
        medium(13.0),
        t.ink,
    );
}

/// A value field: its words, the argument as written (a number or a parameter), its unit, and
/// what it comes to.
#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    args: &mut Vec<(String, String)>,
    key: &str,
    hint: &str,
    unit: &str,
    w: f32,
    places: &mut Vec<(String, Rect)>,
) {
    row(ui, t, label, w, 28.0, |ui| {
        let mut s = arg(args, key).unwrap_or("").to_string();
        let r = ui.add(
            egui::TextEdit::singleline(&mut s)
                .font(sans(13.0))
                .hint_text(hint)
                .desired_width(ui.available_width() - 44.0)
                .margin(vec2(7.0, 5.0)),
        );
        places.push((format!("thermo:field:{key}"), r.rect));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(unit).font(sans(12.0)).color(t.ink2));
        set(args, key, s);
    });
}

/// A choice among words, written quoted.
#[allow(clippy::too_many_arguments)]
fn choice(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    args: &mut Vec<(String, String)>,
    key: &str,
    words: &[(&str, &str)],
    w: f32,
    places: &mut Vec<(String, Rect)>,
) {
    row(ui, t, label, w, 28.0, |ui| {
        let now = arg(args, key).unwrap_or("").trim_matches('"').to_string();
        let mut k = words.iter().position(|x| x.0 == now).unwrap_or(0);
        let r = egui::ComboBox::from_id_salt(("thermo-choice", key))
            .selected_text(words[k].1)
            .width(ui.available_width() - 6.0)
            .show_ui(ui, |ui| {
                for (j, (_, name)) in words.iter().enumerate() {
                    ui.selectable_value(&mut k, j, *name);
                }
            });
        places.push((format!("thermo:choice:{key}"), r.response.rect));
        let was = arg(args, key).is_some();
        if was || k != 0 {
            let q = key != "pull";
            set(
                args,
                key,
                if q {
                    format!("\"{}\"", words[k].0)
                } else {
                    words[k].0.to_string()
                },
            );
        }
    });
}

/// OK, Cancel and Apply, along the foot as Inventor's dialogs have them.
pub(super) fn foot(
    ui: &mut Ui,
    t: &Theme,
    key: &str,
    apply: bool,
    places: &mut Vec<(String, Rect)>,
) -> (bool, bool, bool) {
    let (mut ok, mut cancel, mut ap) = (false, false, false);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ok = button(ui, t, "OK", true, format!("{key}:ok"), places);
        cancel = button(ui, t, "Cancel", false, format!("{key}:cancel"), places);
        if apply {
            ap = button(ui, t, "Apply", false, format!("{key}:apply"), places);
        }
    });
    ui.add_space(12.0);
    (ok, cancel, ap)
}

pub(super) fn note(ui: &mut Ui, text: &str, w: f32, ink: Color32) {
    let g = ui.painter().layout(text.to_string(), sans(11.5), ink, w - 28.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 4.0), Sense::hover());
    ui.painter().galley(r.min + vec2(14.0, 2.0), g, ink);
}

/// A row of words and a value at the right, in the results.
pub(super) fn fact(ui: &mut Ui, t: &Theme, k: &str, v: &str, w: f32, ink: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 14.0, r.center().y),
        Align2::LEFT_CENTER,
        k,
        sans(12.0),
        t.ink2,
    );
    ui.painter().text(
        pos2(r.right() - 14.0, r.center().y),
        Align2::RIGHT_CENTER,
        v,
        mono(12.0),
        ink,
    );
}

const FORM_W: f32 = 382.0;
const CHECK_W: f32 = 340.0;

impl Shell {
    /// The thermoform's dialogs and windows, and the issues opened from the status bar.
    pub(super) fn thermo_ui(&mut self, ctx: &egui::Context) {
        if self.tab == super::THERMO_TAB && !self.thermo_tab() {
            self.tab = 0;
        }
        let Some(mut fg) = self.fg.take() else { return };
        if fg.feats.iter().any(|f| f.kind == "thermoform") {
            fg.thermo.converted = false;
        }
        let t = self.theme;
        let mut places = vec![];
        if self.convert_dialog(ctx, &t, &mut fg, &mut places) {
            self.tab = super::THERMO_TAB;
        }
        self.form_dialog(ctx, &t, &mut fg, &mut places);
        self.mould_dialog(ctx, &t, &mut fg, &mut places);
        self.check_window(ctx, &t, &mut fg, &mut places);
        if fg.selected.is_some() {
            fg.thermo.picked = None;
        }
        self.thermo_issues(ctx, &t, &mut fg, &mut places);
        self.draft_legend(ctx, &t, &mut fg, &mut places);
        material_window(ctx, &t, &mut fg, &mut places);
        self.places.extend(places);
        self.fg = Some(fg);
    }

    /// Convert to Thermoform: the shape, what it is, the mould's side, the sheet. True on OK.
    fn convert_dialog(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let Some(mut c) = fg.thermo.convert.take() else {
            return false;
        };
        let v = self.view_rect;
        let (mut ok, mut close) = (false, false);
        let mut open_material: Option<String> = None;
        let solids = solid_feats(fg);
        egui::Window::new("Convert to Thermoform")
            .id(egui::Id::unique("thermo-convert"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
            .fixed_size(vec2(DLG_W, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, places, |ui, places| {
                ui.spacing_mut().item_spacing.y = 6.0;
                close = head(
                    ui,
                    t,
                    "thicken",
                    "Convert to Thermoform",
                    DLG_W,
                    places,
                    "thermo:convert",
                );
                heading(ui, t, "Shape", DLG_W);
                row(ui, t, "Solid", DLG_W, 32.0, |ui| {
                    let r = egui::ComboBox::from_id_salt("thermo-convert-solid")
                        .selected_text(c.of.as_str())
                        .width(ui.available_width() - 6.0)
                        .show_ui(ui, |ui| {
                            for s in &solids {
                                ui.selectable_value(&mut c.of, s.clone(), s.as_str());
                            }
                        });
                    places.push(("thermo:convert:solid".into(), r.response.rect));
                });
                cards(ui, t, &mut c.from, places);
                heading(ui, t, "Mould", DLG_W);
                row(ui, t, "Mould", DLG_W, 34.0, |ui| {
                    let mut k = usize::from(c.female);
                    toggles(
                        ui,
                        t,
                        &mut k,
                        &["Male (part over it)", "Female (part in it)"],
                        "thermo:convert:tool",
                        places,
                    );
                    c.female = k == 1;
                });
                heading(ui, t, "Sheet", DLG_W);
                row(ui, t, "Material", DLG_W, 32.0, |ui| {
                    let label = fg
                        .thermo_materials
                        .iter()
                        .find(|m| m.name == c.material)
                        .map_or(c.material.clone(), |m| format!("{} · {}", m.name, short(&m.law)));
                    let r = egui::ComboBox::from_id_salt("thermo-material")
                        .selected_text(label)
                        .width(ui.available_width() - 40.0)
                        .show_ui(ui, |ui| {
                            for m in &fg.thermo_materials {
                                let text = format!("{} · {}", m.name, short(&m.law));
                                if ui.selectable_label(c.material == m.name, text).clicked() {
                                    c.material = m.name.clone();
                                    c.temp = m.normal;
                                }
                            }
                        });
                    places.push(("thermo:convert:material".into(), r.response.rect));
                    let more = ui.add(
                        egui::Button::new(egui::RichText::new("…").font(sans(13.0)))
                            .min_size(vec2(30.0, 26.0)),
                    );
                    places.push(("thermo:convert:more".into(), more.rect));
                    if more.clicked() {
                        open_material = Some(c.material.clone());
                    }
                });
                if c.from != 1 {
                    row(ui, t, "Thickness", DLG_W, 32.0, |ui| {
                        let r = egui::ComboBox::from_id_salt("thermo-gauge")
                            .selected_text(format!("{} mm", fmt(c.t)))
                            .width(ui.available_width() - 6.0)
                            .show_ui(ui, |ui| {
                                for g in GAUGES {
                                    ui.selectable_value(&mut c.t, g, format!("{} mm", fmt(g)));
                                }
                            });
                        places.push(("thermo:convert:gauge".into(), r.response.rect));
                    });
                } else {
                    row(ui, t, "Thickness", DLG_W, 28.0, |ui| {
                        ui.label(
                            egui::RichText::new("the wall's own")
                                .font(sans(13.0))
                                .color(t.ink2),
                        );
                    });
                }
                let m = fg.thermo_materials.iter().find(|m| m.name == c.material).cloned();
                row(ui, t, "Sheet temperature", DLG_W, 32.0, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut c.temp)
                            .range(60.0..=300.0)
                            .speed(1.0)
                            .suffix(" °C"),
                    );
                    if let Some(m) = &m {
                        ui.label(
                            egui::RichText::new(format!("forms {}–{} °C", fmt(m.window.0), fmt(m.window.1)))
                                .font(sans(11.0))
                                .color(t.ink2),
                        );
                    }
                });
                let (o, x, _) = foot(ui, t, "thermo:convert", false, places);
                ok = o;
                close |= x;
            });
        if let Some(name) = open_material {
            let libs = forge_material::Libraries::cached(None);
            fg.thermo.material = libs.find(&name).map(|x| x.1.clone());
        }
        if ok && !c.of.is_empty() {
            fg.out.push(FgAct::Apply(Op::AddFeature {
                before: None,
                text: convert_text(fg, &c),
            }));
            fg.thermo.converted = true;
            return true;
        }
        if !close {
            fg.thermo.convert = Some(c);
        }
        false
    }

    /// Edit Formed Part: the thermoform's shape, sheet and trim, prefilled as written.
    fn form_dialog(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) {
        let Some(mut f) = fg.thermo.form.take() else {
            return;
        };
        let v = self.view_rect;
        let w = FORM_W;
        let (mut ok, mut close, mut apply) = (false, false, false);
        let row_data = fg.thermos.iter().find(|r| r.id == f.id).cloned();
        let materials: Vec<(String, String)> = fg
            .thermo_materials
            .iter()
            .map(|m| (m.name.clone(), m.name.clone()))
            .collect();
        let left = if fg.thermo.analysis == Analysis::None {
            24.0
        } else {
            300.0
        };
        egui::Window::new("Edit Formed Part")
            .id(egui::Id::unique("thermo-form"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.left() + left, v.top() + 44.0))
            .fixed_size(vec2(w, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, places, |ui, places| {
                ui.spacing_mut().item_spacing.y = 6.0;
                close = head(ui, t, "thicken", &format!("Formed Part · {}", f.id), w, places, "thermo:form");
                tabs(ui, t, &["Shape", "Trim", "Plug", "Advanced"], &mut f.tab, w, places, "thermo:form:tab");
                ui.add_space(4.0);
                let a = &mut f.args;
                match f.tab {
                    0 => {
                        row(ui, t, "Shape", w, 30.0, |ui| {
                            ui.label(egui::RichText::new(arg(a, "shape").unwrap_or("")).font(sans(13.0)).color(t.ink));
                        });
                        let from: Vec<(&str, &str)> = FROM.iter().map(|x| (x.0, x.1)).collect();
                        choice(ui, t, "Modelled as", a, "from", &from, w, places);
                        row(ui, t, "Mould", w, 34.0, |ui| {
                            let mut k = usize::from(arg(a, "tool") == Some("\"female\""));
                            toggles(ui, t, &mut k, &["Male", "Female"], "thermo:form:tool", places);
                            if k == 1 || arg(a, "tool").is_some() {
                                set(a, "tool", if k == 1 { "\"female\"".into() } else { "\"male\"".into() });
                            }
                        });
                        heading(ui, t, "Sheet", w);
                        let m: Vec<(&str, &str)> = materials.iter().map(|x| (x.0.as_str(), x.1.as_str())).collect();
                        if !m.is_empty() {
                            choice(ui, t, "Material", a, "material", &m, w, places);
                        }
                        // a parameter's name says what it comes to
                        let tn = row_data.as_ref().map_or(String::new(), |r| format!("now {}", fmt(r.t)));
                        let unit = match (arg(a, "t"), &row_data) {
                            (Some(x), Some(r)) if x.trim().parse::<f64>().is_err() => format!("= {}", fmt(r.t)),
                            _ => "mm".to_string(),
                        };
                        field(ui, t, "Thickness", a, "t", &tn, &unit, w, places);
                        field(ui, t, "Temperature", a, "temperature", "the material's", "°C", w, places);
                        choice(ui, t, "Pull", a, "pull", &PULLS, w, places);
                    }
                    1 => {
                        field(ui, t, "Flange", a, "flange", "10", "mm", w, places);
                        row(ui, t, "Trim outline", w, 28.0, |ui| {
                            let text = if arg(a, "trim").is_some() { "a profile" } else { "the shape's box + flange" };
                            ui.label(egui::RichText::new(text).font(sans(13.0)).color(t.ink2));
                        });
                        heading(ui, t, "Blank", w);
                        let mut auto = arg(a, "sheet").is_none_or(str::is_empty);
                        row(ui, t, "Size", w, 30.0, |ui| {
                            let r = ui.checkbox(&mut auto, egui::RichText::new("The trim and the clamp's").font(sans(13.0)));
                            places.push(("thermo:form:blank-auto".into(), r.rect));
                        });
                        if auto {
                            set(a, "sheet", String::new());
                        } else {
                            let now = arg(a, "sheet").unwrap_or("").trim_matches(['(', ')']).to_string();
                            let (mut bw, mut bl) = now.split_once(',').map_or((String::new(), String::new()), |(x, y)| (x.trim().into(), y.trim().into()));
                            if bw.is_empty() {
                                let (x, y) = row_data.as_ref().and_then(|r| r.facts.iter().find(|f| f.0 == "sheet used")).and_then(|f| {
                                    let mut it = f.1.split('×').map(str::trim);
                                    Some((it.next()?.to_string(), it.next()?.to_string()))
                                }).unwrap_or(("200".into(), "150".into()));
                                (bw, bl) = (x, y);
                            }
                            let mut pair = vec![("w".to_string(), bw), ("l".to_string(), bl)];
                            field(ui, t, "Width", &mut pair, "w", "", "mm", w, places);
                            field(ui, t, "Length", &mut pair, "l", "", "mm", w, places);
                            set(a, "sheet", format!("({}, {})", pair[0].1, pair[1].1));
                        }
                    }
                    2 => {
                        let modes = ["", "\"dry\"", "\"lubricated\""];
                        row(ui, t, "Plug assist", w, 34.0, |ui| {
                            let mut k = modes.iter().position(|m| arg(a, "plug").unwrap_or("") == *m).unwrap_or(0);
                            toggles(ui, t, &mut k, &["None", "Dry", "Lubricated"], "thermo:form:plug", places);
                            set(a, "plug", modes[k].to_string());
                        });
                        if arg(a, "plug").is_some_and(|p| !p.is_empty()) {
                            field(ui, t, "Stroke", a, "plug_stroke", "below the sheet", "mm", w, places);
                            field(ui, t, "Diameter", a, "plug_diameter", "42", "mm", w, places);
                            field(ui, t, "Nose radius", a, "plug_nose", "90", "mm", w, places);
                            field(ui, t, "Edge radius", a, "plug_edge", "10", "mm", w, places);
                            field(ui, t, "Starts above", a, "plug_start", "20", "mm", w, places);
                            note(
                                ui,
                                "A round plug at the trim's middle, driven in before the vacuum (a female mould's). Dry: the sheet sticks where it touches it; lubricated: it slides on it.",
                                w,
                                t.ink2,
                            );
                        } else {
                            note(ui, "The sheet forms by the vacuum alone.", w, t.ink2);
                        }
                    }
                    _ => {
                        heading(ui, t, "Bending", w);
                        row(ui, t, "Sheet stiffness", w, 34.0, |ui| {
                            let mut k = usize::from(arg(a, "bending") == Some("true"));
                            toggles(ui, t, &mut k, &["Membrane", "Plate"], "thermo:form:bending", places);
                            set(a, "bending", if k == 1 { "true".into() } else { String::new() });
                        });
                        note(
                            ui,
                            "Plate: the sheet also bends as a plate of its thickness (closer at tight rounds; forming takes about 5-15 % longer).",
                            w,
                            t.ink2,
                        );
                        heading(ui, t, "Uneven heating", w);
                        field(ui, t, "Edge temperature", a, "heat_edge", "the same as the middle", "°C", w, places);
                        note(
                            ui,
                            "The sheet at the temperature above in its middle and this at the window's edge, each part as stiff as the material's temperature shift says (ABS, HIPS).",
                            w,
                            t.ink2,
                        );
                    }
                }
                let (o, x, ap) = foot(ui, t, "thermo:form", true, places);
                ok = o;
                close |= x;
                apply = ap;
            });
        if ok || apply {
            for op in changes(&f.id, &f.was, &f.args) {
                fg.out.push(FgAct::Apply(op));
            }
            f.was = f.args.clone();
        }
        if !(ok || close) {
            fg.thermo.form = Some(f);
        }
    }

    /// Make Mould: what the mould is grown by, stands on, is vented with, and its exports.
    fn mould_dialog(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) {
        let Some(mut m) = fg.thermo.mould.take() else {
            return;
        };
        let v = self.view_rect;
        let w = FORM_W;
        let (mut ok, mut close, mut apply) = (false, false, false);
        let mut export = None;
        let r = fg.thermos.iter().find(|r| r.id == m.id).cloned();
        let fact_of = |k: &str| {
            r.as_ref()
                .and_then(|r| r.facts.iter().find(|f| f.0 == k))
                .map_or(String::new(), |f| f.1.clone())
        };
        let left = if fg.thermo.analysis == Analysis::None {
            24.0
        } else {
            300.0
        };
        egui::Window::new("Make Mould")
            .id(egui::Id::unique("thermo-mould"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.left() + left, v.top() + 44.0))
            .fixed_size(vec2(w, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, places, |ui, places| {
                ui.spacing_mut().item_spacing.y = 3.0;
                close = head(
                    ui,
                    t,
                    "extrude",
                    &format!("Make Mould · {}", m.id),
                    w,
                    places,
                    "thermo:mould",
                );
                heading(ui, t, "Tool", w);
                let shrink = r
                    .as_ref()
                    .map_or(String::new(), |r| format!("{}'s, {:.4}", r.material, r.shrink));
                field(ui, t, "Shrink", &mut m.args, "shrink", &shrink, "", w, places);
                field(ui, t, "Base height", &mut m.args, "base", "10", "mm", w, places);
                field(ui, t, "Base margin", &mut m.args, "margin", "5", "mm", w, places);
                row(ui, t, "Made by", w, 28.0, |ui| {
                    let r = egui::ComboBox::from_id_salt("thermo-made-by")
                        .selected_text(MADE_BY[m.made_by])
                        .width(ui.available_width() - 6.0)
                        .show_ui(ui, |ui| {
                            for (k, s) in MADE_BY.iter().enumerate() {
                                ui.selectable_value(&mut m.made_by, k, *s);
                            }
                        });
                    places.push(("thermo:mould:madeby".into(), r.response.rect));
                });
                heading(ui, t, "Vents", w);
                let mut on = arg(&m.args, "vent") != Some("false");
                row(ui, t, "Vents", w, 30.0, |ui| {
                    let r = ui.checkbox(
                        &mut on,
                        egui::RichText::new("Where the sheet arrives last").font(sans(13.0)),
                    );
                    places.push(("thermo:mould:vents".into(), r.rect));
                });
                if !on {
                    set(&mut m.args, "vent", "false".into());
                } else {
                    if arg(&m.args, "vent") == Some("false") {
                        set(&mut m.args, "vent", String::new());
                    }
                    field(
                        ui,
                        t,
                        "Diameter",
                        &mut m.args,
                        "vent",
                        "the material's",
                        "mm",
                        w,
                        places,
                    );
                    field(ui, t, "Pitch", &mut m.args, "vent_pitch", "25", "mm", w, places);
                }
                fact(ui, t, "Vents", &fact_of("vents"), w, t.ink);
                fact(ui, t, "Mould scale", &fact_of("mould scale"), w, t.ink);
                heading(ui, t, "Export", w);
                for pair in [
                    [
                        ("Mould · STEP", Export::MouldStep),
                        ("Mould · STL", Export::MouldStl),
                    ],
                    [
                        ("Part · STEP (modelled)", Export::PartStep),
                        ("Part · STL (simulated)", Export::PartStl),
                    ],
                    [
                        ("Trim line · DXF", Export::TrimDxf),
                        ("Process sheet · PDF", Export::ProcessPdf),
                    ],
                    [
                        ("Distortion print…", Export::DistortionPrint),
                        ("Wall · CSV", Export::ThicknessCsv),
                    ],
                ] {
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        for (label, e) in pair {
                            let b =
                                egui::Button::new(egui::RichText::new(label).font(sans(12.5)).color(t.ink))
                                    .fill(t.raised)
                                    .stroke(Stroke::new(1.0, t.line))
                                    .min_size(vec2((w - 34.0) / 2.0, 26.0));
                            let r = ui.add(b);
                            places.push((format!("thermo:export:{e:?}"), r.rect));
                            if r.clicked() {
                                export = Some(e);
                            }
                        }
                    });
                }
                if let Some(s) = &fg.thermo.said {
                    note(ui, s, w, t.ink2);
                }
                let (o, x, ap) = foot(ui, t, "thermo:mould", true, places);
                ok = o;
                close |= x;
                apply = ap;
            });
        if let Some(e) = export {
            fg.thermo.want_export = Some((m.id.clone(), e));
        }
        if ok || apply {
            for op in changes(&m.id, &m.was, &m.args) {
                fg.out.push(FgAct::Apply(op));
            }
            m.was = m.args.clone();
        }
        if !(ok || close) {
            fg.thermo.mould = Some(m);
        }
    }

    /// Check Forming: the one results window. The verdict, the wall's extremes, what colours the
    /// part, probes, the forming played, the two masses, the checks, the CSV.
    fn check_window(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) {
        if !fg.thermo.check {
            return;
        }
        let Some(r) = fg.thermos.first().cloned() else {
            return;
        };
        let v = self.view_rect;
        let w = CHECK_W;
        // the panel is out of the shell while it draws: its own word on being an assembly
        let asm = fg.asm.is_some();
        let mut close = false;
        let issues: Vec<(Severity, String, String)> = fg
            .issues
            .iter()
            .filter(|i| i.feature.as_deref() == Some(r.id.as_str()))
            .map(|i| (i.severity, i.kind.clone(), i.text.clone()))
            .collect();
        let green = Color32::from_rgb(0x5c, 0xc0, 0x7a);
        let n = r.stages.len().max(1);
        if fg.thermo.playing {
            let k = ((ctx.input(|i| i.time) * 8.0) as usize) % n;
            fg.thermo.step = Some(k);
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let u = &mut fg.thermo;
        egui::Window::new("Check Forming")
            .id(egui::Id::unique("thermo-check"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.right() - w - 170.0, v.top() + 8.0))
            .fixed_size(vec2(w, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, places, |ui, places| {
                ui.spacing_mut().item_spacing.y = 2.0;
                close = head(
                    ui,
                    t,
                    "measure",
                    &format!("Check Forming · {}", r.id),
                    w,
                    places,
                    "thermo:check",
                );
                let (ink, verdict) = match r.passes {
                    Some(true) => (
                        green,
                        format!(
                            "Forms: thinnest wall {:.2} mm, over the {:.2} mm limit",
                            r.min.0, r.limit
                        ),
                    ),
                    Some(false) if r.unformed > 0 => (
                        super::fg::WARN,
                        format!(
                            "The sheet bridges at {} points: it never reaches the mould there",
                            r.unformed
                        ),
                    ),
                    Some(false) => (
                        super::fg::WARN,
                        format!("Too thin: {:.2} mm, under the {:.2} mm limit", r.min.0, r.limit),
                    ),
                    None => (t.ink2, "No thickness model (walls: false)".to_string()),
                };
                ui.add_space(4.0);
                note(ui, &verdict, w, ink);
                heading(ui, t, "Wall thickness", w);
                fact(ui, t, "Thinnest (MIN)", &format!("{:.2} mm", r.min.0), w, t.ink);
                fact(ui, t, "Thickest (MAX)", &format!("{:.2} mm", r.max.0), w, t.ink);
                if let Some(m) = r.facts.iter().find(|f| f.0 == "mean wall") {
                    fact(ui, t, "Mean (t / draw ratio)", &m.1, w, t.ink);
                }
                fact(
                    ui,
                    t,
                    &format!("Limit ({})", r.material),
                    &format!("{:.2} mm", r.limit),
                    w,
                    t.ink,
                );
                if let Some([l1, area, uneven]) = r.stretch {
                    heading(ui, t, "Stretch", w);
                    fact(ui, t, "Most stretched (principal)", &format!("{l1:.2} ×"), w, t.ink);
                    fact(ui, t, "Most stretched (area)", &format!("{area:.2} ×"), w, t.ink);
                    fact(ui, t, "Most uneven (l1 / l2)", &format!("{uneven:.2}"), w, t.ink);
                }
                if let Some((gathered, folds)) = r.web {
                    heading(ui, t, "Webbing", w);
                    let ink = if folds > 0 { super::fg::WARN } else { t.ink };
                    fact(ui, t, "Gathered (flat sheet)", &format!("{gathered:.0} mm²"), w, ink);
                    fact(ui, t, "Folds onto itself", &format!("{folds}"), w, ink);
                    if folds > 0 {
                        note(
                            ui,
                            &format!("The sheet folds onto itself at {folds} points: webs likely (Webbing shows them)"),
                            w,
                            super::fg::WARN,
                        );
                    }
                }
                heading(ui, t, "Shrink", w);
                fact(ui, t, "Mould scale (vendor)", &format!("{:.2} %", 100.0 * r.shrink), w, t.ink);
                if !r.shrink_source.is_empty() {
                    note(ui, &r.shrink_source, w, t.ink2);
                }
                if let Some((a, b)) = r.shrink_band {
                    fact(
                        ui,
                        t,
                        "Free contraction",
                        &format!("{:.2}-{:.2} %", 100.0 * a, 100.0 * b),
                        w,
                        t.ink,
                    );
                    note(ui, &r.shrink_band_from, w, t.ink2);
                }
                if !asm {
                    heading(ui, t, "Show on the part", w);
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        let layers = [
                            Analysis::Thickness,
                            Analysis::Stretch,
                            Analysis::Webbing,
                            Analysis::Draft,
                            Analysis::None,
                        ];
                        let mut k = layers.iter().position(|a| *a == u.analysis).unwrap_or(4);
                        toggles(
                            ui,
                            t,
                            &mut k,
                            &["Thickness", "Stretch", "Webbing", "Draft", "Nothing"],
                            "thermo:check:layer",
                            places,
                        );
                        u.analysis = layers[k];
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        let mut k = usize::from(u.simulated);
                        let can = r.simulated.is_some();
                        ui.add_enabled_ui(can, |ui| {
                            toggles(
                                ui,
                                t,
                                &mut k,
                                &["Modelled", "Simulated wall"],
                                "thermo:check:state",
                                places,
                            );
                        });
                        u.simulated = can && k == 1;
                    });
                    heading(ui, t, "Probe", w);
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        let mut k = usize::from(u.probe);
                        let was = k;
                        toggles(
                            ui,
                            t,
                            &mut k,
                            &["Off", "Click the part"],
                            "thermo:check:probe",
                            places,
                        );
                        u.probe = k == 1;
                        if u.probe && was == 0 {
                            u.analysis = Analysis::Thickness;
                        }
                        if !u.probes.is_empty()
                            && button(ui, t, "Clear", false, "thermo:check:clear".into(), places)
                        {
                            u.probes.clear();
                        }
                    });
                    for (k, (_, h)) in u.probes.iter().enumerate() {
                        fact(
                            ui,
                            t,
                            &format!("Probe {}", k + 1),
                            &format!("{h:.2} mm"),
                            w,
                            t.ink,
                        );
                    }
                    heading(ui, t, "Forming", w);
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        if button(
                            ui,
                            t,
                            if u.playing { "Pause" } else { "Play" },
                            false,
                            "thermo:check:play".into(),
                            places,
                        ) {
                            u.playing = !u.playing;
                            u.analysis = Analysis::Thickness;
                        }
                        let mut k = u.step.unwrap_or(n - 1).min(n - 1);
                        ui.style_mut().visuals.selection.bg_fill = Color32::from_rgb(0x2f, 0x86, 0xd6);
                        ui.spacing_mut().slider_width = 120.0;
                        let s = ui.add(
                            egui::Slider::new(&mut k, 0..=n - 1)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        ui.add_space(6.0);
                        places.push(("thermo:check:step".into(), s.rect));
                        if s.changed() {
                            u.playing = false;
                            u.analysis = Analysis::Thickness;
                            u.step = (k + 1 < n).then_some(k);
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} / {n} · {}",
                                k + 1,
                                r.stages.get(k).copied().unwrap_or("")
                            ))
                            .font(mono(11.5))
                            .color(t.ink2),
                        );
                    });
                }
                if let (Some(a), Some(b)) = (&r.modelled, &r.simulated) {
                    heading(ui, t, "Mass", w);
                    let kg = |m: &Masses| {
                        m.mass.map_or(format!("{:.0} mm³", m.volume), |x| {
                            format!("{:.1} g", x * 1000.0)
                        })
                    };
                    fact(ui, t, "Modelled (t everywhere)", &kg(a), w, t.ink);
                    fact(ui, t, "Simulated (the model's wall)", &kg(b), w, t.ink);
                    fact(
                        ui,
                        t,
                        "Simulated / modelled",
                        &format!("{:.0} %", 100.0 * b.volume / a.volume),
                        w,
                        t.ink,
                    );
                }
                // an assembly has the part's numbers; its checks are the part's file's
                if !asm {
                    heading(ui, t, "Checks", w);
                    let kinds = [
                        "DraftTooLow",
                        "Undercut",
                        "RadiusTooTight",
                        "DrawTooDeep",
                        "Webbing",
                        "ThinWall",
                    ];
                    let found = |k: &str| {
                        issues
                            .iter()
                            .find(|i| i.1 == k || (k == "RadiusTooTight" && i.1 == "SharpEdge"))
                    };
                    let passed = kinds.iter().filter(|k| found(k).is_none()).count();
                    let mut lines: Vec<(Color32, &str, String)> = kinds
                        .iter()
                        .filter_map(|k| found(k))
                        .map(|(sev, _, text)| {
                            (
                                if *sev == Severity::Error {
                                    t.error
                                } else {
                                    super::fg::WARN
                                },
                                "warning",
                                text.clone(),
                            )
                        })
                        .collect();
                    if passed > 0 {
                        let all = if passed == kinds.len() { "All" } else { "The other" };
                        lines.push((
                            green,
                            "check",
                            format!("{all} {passed} pass: draft, undercut, radii, draw, webbing, wall"),
                        ));
                    }
                    for (c, icon, s) in lines {
                        let g = ui.painter().layout(s, sans(11.0), c, w - 46.0);
                        let (rr, _) = ui.allocate_exact_size(vec2(w, g.size().y + 3.0), Sense::hover());
                        super::icons::draw(ui.painter(), rr.min + vec2(14.0, 0.0), 13.0, icon, 1.4, c);
                        ui.painter().galley(rr.min + vec2(32.0, 0.0), g, c);
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if button(ui, t, "Export CSV…", false, "thermo:check:csv".into(), places) {
                        u.want_export = Some((r.id.clone(), Export::ThicknessCsv));
                    }
                    if !asm
                        && !issues.is_empty()
                        && button(ui, t, "Fixes…", false, "thermo:check:fixes".into(), places)
                    {
                        u.issues = !u.issues;
                    }
                    if button(ui, t, "Close", false, "thermo:check:done".into(), places) {
                        close = true;
                    }
                });
                if let Some(s) = &u.said {
                    note(ui, s, w, t.ink2);
                }
                ui.add_space(10.0);
            });
        if close {
            fg.thermo.check = false;
            fg.thermo.playing = false;
            fg.thermo.probe = false;
        }
    }
}

impl Shell {
    /// The thermoform's issues, opened from the status bar: each with its fixes, a thin wall's
    /// each re-simulated.
    fn thermo_issues(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) {
        if !fg.thermo.issues || fg.thermos.is_empty() {
            return;
        }
        let v = self.view_rect;
        let w = 344.0;
        let ids: Vec<String> = fg.thermos.iter().map(|r| r.id.clone()).collect();
        let issues: Vec<super::fg::Issue> = fg
            .issues
            .iter()
            .filter(|i| i.feature.as_ref().is_some_and(|f| ids.contains(f)))
            .cloned()
            .collect();
        let mut apply = vec![];
        let mut want = vec![];
        let mut close = false;
        egui::Area::new(egui::Id::unique("thermo-issues"))
            .pivot(Align2::RIGHT_BOTTOM)
            .fixed_pos(pos2(v.right() - 8.0, v.bottom() - 8.0))
            .order(egui::Order::Foreground)
            .show_placed(ctx, places, |ui, places| {
                frame(t).show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    close = title(ui, t, "Issues and their fixes", w, places, "thermo:issues");
                    egui::ScrollArea::vertical()
                        .max_height(v.height() - 120.0)
                        .show(ui, |ui| {
                            if issues.is_empty() {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(14.0);
                                    ui.label(
                                        egui::RichText::new("No issues: the part forms by every check.")
                                            .font(sans(12.0))
                                            .color(t.ink2),
                                    );
                                });
                            }
                            for (k, i) in issues.iter().enumerate() {
                                let ink = match i.severity {
                                    Severity::Error => t.error,
                                    Severity::Lint => t.ink2,
                                    _ => super::fg::WARN,
                                };
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(14.0);
                                    let sev = match i.severity {
                                        Severity::Error => "Error",
                                        Severity::Violation => "Shop rule",
                                        _ => "Warning",
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{sev} · {}", words(&i.kind)))
                                            .font(medium(13.0))
                                            .color(ink),
                                    );
                                });
                                let g = ui.painter().layout(i.text.clone(), sans(12.0), t.ink, w - 28.0);
                                let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
                                ui.painter().galley(r.min + vec2(14.0, 0.0), g, t.ink);
                                for (j, op) in i.fixes.iter().enumerate() {
                                    let key = op.json().to_string();
                                    let sim = fg.thermo.resim.iter().find(|r| r.0 == key);
                                    if i.kind == "ThinWall" && sim.is_none() {
                                        want.push(op.clone());
                                    }
                                    let after = match sim {
                                        Some((_, Some(min))) => {
                                            let limit = fg
                                                .thermos
                                                .iter()
                                                .find(|r| Some(&r.id) == i.feature.as_ref())
                                                .map_or(0.0, |r| r.limit);
                                            format!(
                                                "  ·  min {min:.2} mm {}",
                                                if *min >= limit { "(passes)" } else { "(still thin)" }
                                            )
                                        }
                                        Some((_, None)) => "  ·  simulating…".into(),
                                        None => String::new(),
                                    };
                                    if wide(
                                        ui,
                                        t,
                                        &format!("{}{after}", op_words(op)),
                                        w,
                                        j == 0,
                                        format!("thermo:fix:{k}:{j}"),
                                        places,
                                    ) {
                                        apply.push(op.clone());
                                    }
                                }
                            }
                            // the numbers the shop asks for
                            if let Some(r) = fg.thermos.first() {
                                ui.add_space(8.0);
                                for (k, v) in r.facts.iter().filter(|f| {
                                    ["mean wall", "areal draw ratio", "below limit", "sheet used"]
                                        .contains(&f.0.as_str())
                                }) {
                                    ui.horizontal(|ui| {
                                        ui.add_space(14.0);
                                        ui.label(egui::RichText::new(k).font(sans(12.0)).color(t.ink2));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(14.0);
                                                ui.label(
                                                    egui::RichText::new(v).font(mono(12.0)).color(t.ink),
                                                );
                                            },
                                        );
                                    });
                                }
                            }
                            ui.add_space(10.0);
                        });
                });
            });
        for op in apply {
            fg.out.push(FgAct::Apply(op));
        }
        fg.thermo.want_resim = want;
        if close {
            fg.thermo.issues = false;
        }
    }

    /// The draft analysis's legend: its classes and their counts, the least draft, the pull.
    fn draft_legend(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) {
        if fg.thermo.analysis != Analysis::Draft {
            return;
        }
        let Some(r) = fg.thermos.first().cloned() else {
            return;
        };
        let v = self.view_rect;
        let w = 256.0;
        let mut min = fg.thermo.draft_min.unwrap_or(r.draft_min);
        egui::Area::new(egui::Id::unique("thermo-draft"))
            .fixed_pos(pos2(v.left() + 16.0, v.top() + 48.0))
            .order(egui::Order::Middle)
            .show_placed(ctx, places, |ui, places| {
                frame(t).show(ui, |ui| {
                    ui.set_width(w);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("Draft analysis")
                                .font(semi(13.0))
                                .color(t.ink),
                        );
                    });
                    let counts = draft_counts(&r, min);
                    for (k, (name, n)) in [
                        (
                            format!("≥ {min:.1}° ({} min)", if r.female { "female" } else { "male" }),
                            counts[0],
                        ),
                        (format!("1.0 – {min:.1}°"), counts[1]),
                        ("< 1.0°".to_string(), counts[2]),
                        ("Undercut".to_string(), counts[3]),
                        ("Flat / clamp".to_string(), counts[4]),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            let (sw, _) = ui.allocate_exact_size(vec2(16.0, 14.0), Sense::hover());
                            ui.painter().rect_filled(sw, 2.0, c32(super::thermo::DRAFT[k]));
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(name).font(sans(12.0)).color(t.ink));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(12.0);
                                ui.label(
                                    egui::RichText::new(format!("{n} faces"))
                                        .font(sans(11.0))
                                        .color(t.ink2),
                                );
                            });
                        });
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Minimum draft (from {} · {})",
                                r.material,
                                if r.female { "female" } else { "male" }
                            ))
                            .font(sans(11.0))
                            .color(t.ink2),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.style_mut().visuals.widgets.inactive.bg_fill = t.line;
                        ui.style_mut().visuals.selection.bg_fill = Color32::from_rgb(0x2f, 0x86, 0xd6);
                        let s = ui.add(
                            egui::Slider::new(&mut min, 0.5..=8.0)
                                .suffix("°")
                                .step_by(0.1)
                                .trailing_fill(true),
                        );
                        places.push(("thermo:draft:min".into(), s.rect));
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(format!("Pull {} from {}", r.pull, r.id))
                                .font(sans(11.0))
                                .color(t.ink2),
                        );
                    });
                    ui.add_space(10.0);
                });
            });
        if (min - r.draft_min).abs() > 1e-4 || fg.thermo.draft_min.is_some() {
            fg.thermo.draft_min = Some(min);
        }
    }
}

/// Faces in each draft class at least draft `min` degrees: OK, low, very low, undercut, flat.
pub fn draft_counts(r: &ThermoRow, min: f32) -> [usize; 5] {
    let mut out = [0; 5];
    for &a in &r.face_drafts {
        out[draft_class(a, min)] += 1;
    }
    out
}

/// A draft's class (degrees): 0 at least `min`, 1 from 1 to it, 2 under 1, 3 under the pull, 4 flat.
pub fn draft_class(a: f32, min: f32) -> usize {
    if a < -0.01 {
        3
    } else if a > 80.0 {
        4
    } else if a >= min - 1e-4 {
        0
    } else if a >= 1.0 {
        1
    } else {
        2
    }
}

/// A source in a few words: its first authors and its year, "Erchiqui & Kandil 2004 fit".
fn short(s: &str) -> String {
    if s.is_empty() || s.starts_with("stand-in") {
        return "stand-in law".into();
    }
    let who = s.split(',').next().unwrap_or(s).trim();
    let year = s
        .as_bytes()
        .windows(4)
        .find(|w| w.iter().all(u8::is_ascii_digit) && (w[0] == b'1' || w[0] == b'2'))
        .map(|w| String::from_utf8_lossy(w).to_string());
    let who: String = who.chars().take(26).collect();
    match year {
        Some(y) => format!("{who} {y} fit"),
        None => format!("{who} fit"),
    }
}

/// An issue's kind in words.
fn words(kind: &str) -> String {
    let mut out = String::new();
    for (k, c) in kind.chars().enumerate() {
        if c.is_uppercase() && k > 0 {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A fix in words.
fn op_words(op: &Op) -> String {
    match op {
        Op::SetParam { param, value } => format!("Set {param} to {}", fmt(value.value)),
        Op::SetArg { id, arg, text } => format!("Set {id} {arg} to {text}"),
        Op::AddFeature { text, .. } => format!("Add {}", text.trim()),
        other => format!("{other:?}"),
    }
}

/// The banded legend's colour for a thickness between the range's ends.
pub fn legend_colour(f: f32) -> Color32 {
    c32(band(f))
}

/// A thermoform's row in the browser: Edit…, Check Forming and Make Mould a right click away.
pub fn feature_row(resp: &egui::Response, fg: &mut FgPanel, places: &mut Vec<(String, Rect)>, key: &str) {
    resp.context_menu(|ui| {
        for (word, c) in [
            ("Edit…", Cmd::Form),
            ("Check Forming", Cmd::Check),
            ("Make Mould", Cmd::Mould),
        ] {
            let b = ui.button(word);
            places.push((format!("fg:menu:{word}:{key}"), b.rect));
            if b.clicked() {
                if c == Cmd::Form {
                    fg.thermo.form = None;
                }
                fg.thermo_command(c);
                ui.close();
            }
        }
    });
}

/// Under a thermoform in the browser: the formed part, the mould and the blank, each with its eye.
/// A click shows that body alone, as sheet metal's Folded Model and Flat Pattern rows do; its
/// tools are the ribbon's and the thermoform row's. Returns the feature that made the body clicked.
pub fn tree_children(
    ui: &mut Ui,
    t: &Theme,
    fg: &mut FgPanel,
    w: f32,
    key: &str,
    x0: f32,
    places: &mut Vec<(String, Rect)>,
) -> Option<String> {
    let r = fg.thermos.iter().find(|r| r.id == key).cloned()?;
    let fact = |k: &str| {
        r.facts
            .iter()
            .find(|f| f.0 == k)
            .map_or(String::new(), |f| f.1.clone())
    };
    let u = fg.thermo.clone();
    let part_note = if u.simulated && r.simulated.is_some() {
        format!("simulated {:.2}–{:.2}", r.min.0, r.max.0)
    } else {
        format!("modelled {:.2}", r.t)
    };
    let rows = [
        (
            "Formed Part",
            "thicken",
            part_note,
            u.show.part,
            Cmd::ShowPart,
            Cmd::Form,
        ),
        (
            "Mould",
            "extrude",
            format!(
                "+{:.2} % · {} vents",
                r.shrink * 100.0,
                fact("vents").split(' ').next().unwrap_or("")
            ),
            u.show.mould,
            Cmd::ShowMould,
            Cmd::Mould,
        ),
        (
            "Blank",
            "rect",
            fact("sheet used"),
            u.show.blank,
            Cmd::ShowBlank,
            Cmd::Trim,
        ),
    ];
    let mut chosen = None;
    if fg.thermo.hover.is_some() {
        fg.thermo.hover = None;
    }
    for ((name, icon, note, shown, eye, _), body) in
        rows.into_iter().zip([Body::Part, Body::Mould, Body::Blank])
    {
        let (rr, resp) = ui.allocate_exact_size(vec2(w, super::ROW_H), Sense::click());
        let p = ui.painter();
        let on = u.picked == Some(body);
        if on {
            p.rect_filled(rr, 0.0, t.on);
            p.rect_filled(Rect::from_min_size(rr.min, vec2(2.0, super::ROW_H)), 0.0, t.focus);
        } else if resp.hovered() {
            p.rect_filled(rr, 0.0, t.hover);
        }
        if resp.hovered() {
            fg.thermo.hover = Some(body);
        }
        // a click looks at that body alone (again: at them all), and a tool waiting for a body
        // takes it
        if resp.clicked() {
            fg.thermo.picked = if on { None } else { Some(body) };
            fg.selected = None;
            chosen = Some(match body {
                Body::Mould => r.shape.clone(),
                _ => r.id.clone(),
            });
        }
        let ink = match (on, u.shows(body)) {
            (true, _) => t.on_ink,
            (_, true) => t.ink,
            _ => t.ink2.gamma_multiply(0.6),
        };
        let x = rr.left() + x0 + 16.0;
        super::icons::draw(p, pos2(x + 12.0, rr.top() + 5.0), 16.0, icon, 1.5, ink);
        let g = p.layout_no_wrap(name.to_string(), sans(13.0), ink);
        let gw = g.size().x;
        p.galley(pos2(x + 34.0, rr.center().y - g.size().y / 2.0), g, ink);
        p.text(
            pos2(x + 40.0 + gw, rr.center().y),
            Align2::LEFT_CENTER,
            note,
            sans(11.0),
            t.ink2,
        );
        if super::fg::row_eye(ui, t, rr, &format!("thermo-eye-{name}"), shown) {
            fg.thermo_command(eye);
        }
        places.push((format!("fg:eye:thermo:{name}"), super::fg::eye_rect(rr)));
        places.push((format!("fg:row:thermo:{name}"), rr));
    }
    chosen
}

/// The part's physical properties with its formed part as simulated in the modelled one's place:
/// iProperties' Simulated column, beside Modeled.
pub fn simulated_physical(
    m: &forge_lang::props::Physical,
    r: &ThermoRow,
) -> Option<forge_lang::props::Physical> {
    let (a, b) = (r.modelled.as_ref()?, r.simulated.as_ref()?);
    let mut p = m.clone();
    p.volume += b.volume - a.volume;
    p.area += b.area - a.area;
    if let Some(s) = p.solids.iter_mut().find(|s| s.id == r.id) {
        s.volume = b.volume;
        s.area = b.area;
        s.mass = b.mass;
        s.centroid = b.cog;
    }
    let (Some(ma), Some(mb), Some(mm)) = (a.mass, b.mass, m.mass) else {
        return Some(p);
    };
    let ms = mm - ma + mb;
    let c: [f64; 3] = std::array::from_fn(|i| (mm * m.centroid[i] - ma * a.cog[i] + mb * b.cog[i]) / ms);
    p.mass = Some(ms);
    p.centroid = c;
    // each body's inertia about its centre carried to the origin, the formed part's swapped
    let carried = |i: [[f64; 3]; 3], mass: f64, c: [f64; 3], sign: f64| -> [[f64; 3]; 3] {
        let cc = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
        std::array::from_fn(|r| {
            std::array::from_fn(|k| {
                let d = if r == k { cc } else { 0.0 };
                i[r][k] + sign * mass * (d - c[r] * c[k])
            })
        })
    };
    if let (Some(io), Some(ia), Some(ib)) = (m.origin_inertia, a.inertia, b.inertia) {
        let (fa, fb) = (carried(ia, ma, a.cog, 1.0), carried(ib, mb, b.cog, 1.0));
        let o: [[f64; 3]; 3] =
            std::array::from_fn(|r| std::array::from_fn(|k| io[r][k] - fa[r][k] + fb[r][k]));
        let about = carried(o, ms, c, -1.0);
        let (pm, axes) = forge_topo::mass::eigen(about);
        p.origin_inertia = Some(o);
        p.inertia = Some(about);
        p.principal = Some((pm, axes.map(|v| [v.x, v.y, v.z])));
    }
    Some(p)
}

/// A material's thermoforming values, opened from Convert: the Material Browser's Thermoform tab
/// on its own, and Save to My materials.
fn material_window(ctx: &egui::Context, t: &Theme, fg: &mut FgPanel, places: &mut Vec<(String, Rect)>) {
    let Some(mut m) = fg.thermo.material.take() else {
        return;
    };
    let mut open = true;
    let mut save = false;
    egui::Window::new(format!("Material · {} · Thermoform", m.name))
        .id(egui::Id::unique("thermo-material"))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .frame(frame(t).inner_margin(14))
        .default_pos(pos2(300.0, 190.0))
        .default_size(vec2(760.0, 560.0))
        .constrain_to(super::dialog_area(ctx))
        .show_placed(ctx, places, |ui, places| {
            ui.set_width(760.0);
            super::thermo_material::tab(ui, t, &mut m);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let dir = forge_material::user_dir().map(|d| d.join("mine.toml"));
                ui.label(
                    egui::RichText::new(match &dir {
                        Some(d) => format!("Saves to {}", d.display()),
                        None => "No user library here".into(),
                    })
                    .font(sans(11.5))
                    .color(t.ink2),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(
                        ui,
                        t,
                        "Save to My materials",
                        true,
                        "thermo:material:save".into(),
                        places,
                    ) {
                        save = true;
                    }
                });
            });
        });
    if save {
        let said = match forge_material::user_dir() {
            Some(d) => {
                let file = d.join("mine.toml");
                let _ = std::fs::create_dir_all(&d);
                match forge_material::save(&file, &m, None) {
                    Ok(()) => {
                        fg.thermo_materials.clear();
                        format!("Saved {} to {}", m.name, file.display())
                    }
                    Err(e) => format!("Not saved: {e}"),
                }
            }
            None => "No user library here".into(),
        };
        fg.thermo.said = Some(said);
    }
    if open {
        fg.thermo.material = Some(m);
    }
}
