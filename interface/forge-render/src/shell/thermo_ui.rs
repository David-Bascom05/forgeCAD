//! The Thermoform tab: Convert to Thermoform, the Form dialog, the analyses (draft and undercut,
//! wall thickness) with their legends over the view, the mould's settings and its exports, and the
//! issues a click on the status bar opens. No panel stays open: each is a dialog or an overlay the
//! tab's buttons bring up and put away, as Inventor's are.
use super::fg::{FgAct, FgPanel, Severity};
use super::theme::{Theme, medium, mono, sans, semi};
use super::thermo::{ThermoRow, band, c32};
use super::{Action, FgCmd, Shell};
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_lang::error::Op;

/// A Thermoform tab command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Convert,
    Form,
    Trim,
    Plug,
    Vents,
    Draft,
    Undercut,
    Thickness,
    Probe,
    Animate,
    Mould,
    Export,
    ExportCsv,
    Materials,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Analysis {
    #[default]
    None,
    Draft,
    Thickness,
}

/// The tab's state: which dialog is up, which analysis shows, the step watched, the probes.
#[derive(Clone, Debug, Default)]
pub struct ThermoUi {
    pub convert: Option<Convert>,
    pub form: Option<Form>,
    pub mould: Option<Mould>,
    pub analysis: Analysis,
    /// The draft analysis's least draft, degrees (None: the material's for the mould).
    pub draft_min: Option<f32>,
    /// The step of the thickness model shown (None: the last), and whether it plays.
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Export {
    MouldStep,
    MouldStl,
    PartStep,
    TrimDxf,
    ProcessPdf,
    ThicknessCsv,
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

/// The Form dialog, on a thermoform: its arguments as written, edited.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub id: String,
    pub tab: usize,
    pub args: Vec<(String, String)>,
    pub was: Vec<(String, String)>,
}

/// The mould's settings, on a thermoform.
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

/// A ribbon button: its word, its icon, what it does (none: drawn disabled), whether it's on.
pub type Button = (&'static str, &'static str, Option<Action>, bool);

/// The ribbon's Thermoform tab: its groups, each with big and small buttons.
pub fn groups(s: &Shell) -> Vec<(&'static str, Vec<Button>, Vec<Button>)> {
    let fg = s.fg.as_ref();
    let has = fg.is_some_and(|f| !f.thermos.is_empty());
    let solid = fg.is_some_and(|f| last_solid(f).is_some());
    let ui = fg.map(|f| &f.thermo);
    let go = |c: Cmd, ok: bool| ok.then_some(Action::Fg(FgCmd::Thermo(c)));
    let on = |a: Analysis| ui.is_some_and(|u| u.analysis == a);
    vec![
        (
            "Thermoform",
            vec![
                (
                    "Convert",
                    "newsolid",
                    go(Cmd::Convert, solid),
                    ui.is_some_and(|u| u.convert.is_some()),
                ),
                (
                    "Form",
                    "shell",
                    go(Cmd::Form, has),
                    ui.is_some_and(|u| u.form.is_some()),
                ),
                ("Materials", "params", None, false),
            ],
            vec![],
        ),
        (
            "Modify",
            vec![],
            vec![
                ("Trim", "trim", go(Cmd::Trim, has), false),
                ("Plug", "extrude", go(Cmd::Plug, has), false),
                ("Cutout", "hole", None, false),
                ("Vents", "hole", go(Cmd::Vents, has), false),
            ],
        ),
        (
            "Analyze",
            vec![
                ("Draft", "draft", go(Cmd::Draft, has), on(Analysis::Draft)),
                (
                    "Thickness",
                    "measure",
                    go(Cmd::Thickness, has),
                    on(Analysis::Thickness),
                ),
            ],
            vec![
                ("Undercut", "draft", go(Cmd::Undercut, has), on(Analysis::Draft)),
                (
                    "Probe",
                    "measure",
                    go(Cmd::Probe, has),
                    ui.is_some_and(|u| u.probe),
                ),
                (
                    "Animate",
                    "orbit",
                    go(Cmd::Animate, has),
                    ui.is_some_and(|u| u.playing),
                ),
                ("Export CSV", "save", go(Cmd::ExportCsv, has), false),
            ],
        ),
        (
            "Mould",
            vec![
                (
                    "Mould",
                    "extrude",
                    go(Cmd::Mould, has),
                    ui.is_some_and(|u| u.mould.is_some()),
                ),
                ("Export", "save", go(Cmd::Export, has), false),
            ],
            vec![],
        ),
    ]
}

impl FgPanel {
    /// The first thermoform's row, the one the tab works on.
    pub fn thermo_row(&self) -> Option<&ThermoRow> {
        self.thermos.first()
    }

    pub fn thermo_command(&mut self, c: Cmd) {
        let first = self.thermos.first().map(|r| r.id.clone());
        let u = &mut self.thermo;
        match c {
            Cmd::Convert => {
                if u.convert.is_some() {
                    u.convert = None;
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
            Cmd::Form | Cmd::Trim | Cmd::Plug => {
                let Some(id) = first else { return };
                let tab = match c {
                    Cmd::Trim => 1,
                    Cmd::Plug => 2,
                    _ => 0,
                };
                match &mut self.thermo.form {
                    Some(f) if f.tab == tab || c == Cmd::Form => self.thermo.form = None,
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
            Cmd::Mould | Cmd::Vents | Cmd::Export => {
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
            Cmd::Draft | Cmd::Undercut => {
                u.analysis = if u.analysis == Analysis::Draft {
                    Analysis::None
                } else {
                    Analysis::Draft
                };
            }
            Cmd::Thickness => {
                u.analysis = if u.analysis == Analysis::Thickness {
                    Analysis::None
                } else {
                    Analysis::Thickness
                };
                u.step = None;
            }
            Cmd::Probe => {
                u.probe = !u.probe;
                u.analysis = Analysis::Thickness;
            }
            Cmd::Animate => {
                u.analysis = Analysis::Thickness;
                u.playing = !u.playing;
                if u.playing {
                    u.step = Some(0);
                }
            }
            Cmd::ExportCsv => {
                if let Some(id) = first {
                    u.want_export = Some((id, Export::ThicknessCsv));
                }
            }
            // the Material Browser is the material library's (shell/materials.rs)
            Cmd::Materials => {}
        }
    }
}

// ---- the look: a dialog's frame, rows, buttons ----

fn frame(t: &Theme) -> egui::Frame {
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
fn row<R>(ui: &mut Ui, t: &Theme, label: &str, w: f32, h: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
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
fn button(
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

const DLG_W: f32 = 446.0;

impl Shell {
    /// The Thermoform tab's dialogs, overlays and the issues opened from the status bar.
    pub(super) fn thermo_ui(&mut self, ctx: &egui::Context) {
        let Some(mut fg) = self.fg.take() else { return };
        let t = self.theme;
        let mut places = vec![];
        self.convert_dialog(ctx, &t, &mut fg, &mut places);
        self.form_dialog(ctx, &t, &mut fg, &mut places);
        self.mould_dialog(ctx, &t, &mut fg, &mut places);
        self.thermo_issues(ctx, &t, &mut fg, &mut places);
        self.draft_legend(ctx, &t, &mut fg, &mut places);
        material_window(ctx, &t, &mut fg, &mut places);
        self.places.extend(places);
        self.fg = Some(fg);
    }

    fn convert_dialog(
        &self,
        ctx: &egui::Context,
        t: &Theme,
        fg: &mut FgPanel,
        places: &mut Vec<(String, Rect)>,
    ) {
        let Some(mut c) = fg.thermo.convert.take() else {
            return;
        };
        let v = self.view_rect;
        let mut close = false;
        let mut ok = false;
        let mut open_material: Option<String> = None;
        egui::Window::new("Convert to Thermoform")
            .id(egui::Id::unique("thermo-convert"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.right() - DLG_W - 140.0, v.top() + 60.0))
            .fixed_size(vec2(DLG_W, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                close = title(ui, t, "Convert to Thermoform", DLG_W, places, "thermo:convert");
                ui.add_space(4.0);
                let (r, _) = ui.allocate_exact_size(vec2(DLG_W, 20.0), Sense::hover());
                ui.painter().text(
                    pos2(r.left() + 14.0, r.center().y),
                    Align2::LEFT_CENTER,
                    "What did you model?",
                    sans(12.0),
                    t.ink2,
                );
                cards(ui, t, &mut c.from, places);
                row(ui, t, "Part", DLG_W, 32.0, |ui| {
                    ui.label(egui::RichText::new(&c.of).font(mono(13.0)).color(t.ink));
                });
                row(ui, t, "Open face", DLG_W, 32.0, |ui| {
                    let mut s = c.open.clone();
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut s)
                            .font(mono(13.0))
                            .hint_text("the shape's foot (auto)")
                            .desired_width(ui.available_width())
                            .margin(vec2(7.0, 5.0)),
                    );
                    places.push(("thermo:convert:open".into(), r.rect));
                    c.open = s;
                });
                row(ui, t, "Mould", DLG_W, 34.0, |ui| {
                    let mut k = usize::from(c.female);
                    toggles(
                        ui,
                        t,
                        &mut k,
                        &["Male (inside on tool)", "Female"],
                        "thermo:convert:tool",
                        places,
                    );
                    c.female = k == 1;
                });
                row(ui, t, "Material", DLG_W, 32.0, |ui| {
                    let label = fg
                        .thermo_materials
                        .iter()
                        .find(|m| m.name == c.material)
                        .map_or(c.material.clone(), |m| format!("{} · {}", m.name, short(&m.law)));
                    let r = egui::ComboBox::from_id_salt("thermo-material")
                        .selected_text(label)
                        .width(ui.available_width() - 6.0)
                        .show_ui(ui, |ui| {
                            for m in &fg.thermo_materials {
                                if ui
                                    .selectable_label(
                                        c.material == m.name,
                                        format!("{} · {}", m.name, short(&m.law)),
                                    )
                                    .clicked()
                                {
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
                    row(ui, t, "Gauge t", DLG_W, 32.0, |ui| {
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
                }
                let m = fg.thermo_materials.iter().find(|m| m.name == c.material).cloned();
                row(ui, t, "Sheet temp", DLG_W, 32.0, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut c.temp)
                            .range(60.0..=300.0)
                            .speed(1.0)
                            .suffix(" °C"),
                    );
                    if let Some(m) = &m {
                        let fit = m
                            .fitted
                            .map_or(String::new(), |f| format!(" · law fitted at {} °C", fmt(f)));
                        ui.label(
                            egui::RichText::new(format!(
                                "window {}–{} °C{fit}",
                                fmt(m.window.0),
                                fmt(m.window.1)
                            ))
                            .font(sans(11.0))
                            .color(t.ink2),
                        );
                    }
                });
                // the statement it adds, as it will read
                let text = convert_text(fg, &c);
                ui.add_space(4.0);
                let (r, _) = ui.allocate_exact_size(vec2(DLG_W, 52.0), Sense::hover());
                let b =
                    Rect::from_min_max(pos2(r.left() + 14.0, r.top()), pos2(r.right() - 14.0, r.bottom()));
                ui.painter().rect_filled(b, 3.0, t.field);
                ui.painter()
                    .rect_stroke(b, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                let g = ui
                    .painter()
                    .layout(text.trim().to_string(), mono(12.0), t.ink, b.width() - 16.0);
                ui.painter().galley(b.min + vec2(8.0, 6.0), g, t.ink);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(DLG_W - 14.0 - 72.0 * 2.0 - 10.0);
                    if button(ui, t, "Cancel", false, "thermo:convert:cancel".into(), places) {
                        close = true;
                    }
                    if button(ui, t, "OK", true, "thermo:convert:ok".into(), places) {
                        ok = true;
                    }
                });
                ui.add_space(12.0);
            });
        if let Some(name) = open_material {
            let libs = forge_material::Libraries::cached(None);
            fg.thermo.material = libs.find(&name).map(|x| x.1.clone());
        }
        if ok {
            fg.out.push(FgAct::Apply(Op::AddFeature {
                before: None,
                text: convert_text(fg, &c),
            }));
            fg.thermo.analysis = Analysis::Thickness;
        } else if !close {
            fg.thermo.convert = Some(c);
        }
    }

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
        let (mut close, mut ok) = (false, false);
        let row_data = fg.thermos.iter().find(|r| r.id == f.id).cloned();
        let issues: Vec<(Severity, String, String)> = fg
            .issues
            .iter()
            .filter(|i| i.feature.as_deref() == Some(f.id.as_str()))
            .map(|i| (i.severity, i.kind.clone(), i.text.clone()))
            .collect();
        egui::Window::new("Form")
            .id(egui::Id::unique("thermo-form"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.left() + 24.0, v.top() + 24.0))
            .fixed_size(vec2(DLG_W - 64.0, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                let w = DLG_W - 64.0;
                ui.spacing_mut().item_spacing.y = 6.0;
                close = title(ui, t, &format!("Form · {}", f.id), w, places, "thermo:form");
                // the tabs
                let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
                let mut x = r.left() + 6.0;
                for (k, name) in ["Shape", "Trim", "Plug", "Mould"].iter().enumerate() {
                    let g = ui.painter().layout_no_wrap(name.to_string(), sans(13.0), t.ink);
                    let tab = Rect::from_min_size(pos2(x, r.top()), vec2(g.size().x + 26.0, 32.0));
                    let resp = ui.interact(tab, ui.scope_id().with(("thermo-tab", k)), Sense::click());
                    if f.tab == k {
                        ui.painter().rect_filled(tab, 0.0, t.raised);
                        ui.painter().hline(tab.left()..=tab.right(), tab.bottom() - 1.0, Stroke::new(2.0, t.focus));
                    }
                    ui.painter().galley(pos2(tab.left() + 13.0, tab.center().y - g.size().y / 2.0), g, if f.tab == k { t.ink } else { t.ink2 });
                    places.push((format!("thermo:form:tab:{k}"), tab));
                    if resp.clicked() {
                        f.tab = k;
                    }
                    x = tab.right();
                }
                ui.painter().hline(r.left()..=r.right(), r.bottom(), Stroke::new(1.0, t.line));
                ui.add_space(6.0);
                let a = &mut f.args;
                match f.tab {
                    0 => {
                        row(ui, t, "Shape", w, 32.0, |ui| {
                            ui.label(egui::RichText::new(arg(a, "shape").unwrap_or("")).font(mono(13.0)).color(t.ink));
                        });
                        row(ui, t, "Modelled as", w, 32.0, |ui| {
                            let now = arg(a, "from").unwrap_or("\"outer\"").trim_matches('"').to_string();
                            let mut k = FROM.iter().position(|x| x.0 == now).unwrap_or(0);
                            let r = egui::ComboBox::from_id_salt("thermo-form-from")
                                .selected_text(FROM[k].1)
                                .width(ui.available_width() - 6.0)
                                .show_ui(ui, |ui| {
                                    for (j, (_, name, _)) in FROM.iter().enumerate() {
                                        ui.selectable_value(&mut k, j, *name);
                                    }
                                });
                            places.push(("thermo:form:from".into(), r.response.rect));
                            set(a, "from", format!("\"{}\"", FROM[k].0));
                        });
                        row(ui, t, "Mould", w, 34.0, |ui| {
                            let mut k = usize::from(arg(a, "tool") == Some("\"female\""));
                            toggles(ui, t, &mut k, &["Male", "Female"], "thermo:form:tool", places);
                            set(a, "tool", if k == 1 { "\"female\"".into() } else { "\"male\"".into() });
                        });
                        for (label, key, hint) in [("Open face", "open", "the shape's foot"), ("Pull", "pull", "Z"), ("Sheet t", "t", "read from the wall"), ("Material", "material", "\"ABS\""), ("Sheet temp", "temperature", "the material's normal")] {
                            row(ui, t, label, w, 32.0, |ui| {
                                let mut s = arg(a, key).unwrap_or("").to_string();
                                let r = ui.add(egui::TextEdit::singleline(&mut s).font(mono(13.0)).hint_text(hint).desired_width(ui.available_width()).margin(vec2(7.0, 5.0)));
                                places.push((format!("thermo:form:{key}"), r.rect));
                                set(a, key, s);
                            });
                        }
                    }
                    1 => {
                        for (label, key, hint) in [("Flange", "flange", "10"), ("Trim profile", "trim", "the shape's box + flange"), ("Sheet blank", "sheet", "(W, L): the trim and the clamp"), ("Pull", "pull", "Z")] {
                            row(ui, t, label, w, 32.0, |ui| {
                                let mut s = arg(a, key).unwrap_or("").to_string();
                                let r = ui.add(egui::TextEdit::singleline(&mut s).font(mono(13.0)).hint_text(hint).desired_width(ui.available_width()).margin(vec2(7.0, 5.0)));
                                places.push((format!("thermo:form:{key}"), r.rect));
                                set(a, key, s);
                            });
                        }
                    }
                    2 => {
                        row(ui, t, "Plug assist", w, 32.0, |ui| {
                            ui.add_enabled(false, egui::Button::new(egui::RichText::new("Pick a plug body").font(sans(13.0))));
                        });
                        let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
                        let g = ui.painter().layout("The thickness model has no plug yet: it forms by drape and vacuum. A plug is the next step of the model (a rigid tool the sheet sticks to).".into(), sans(11.0), t.ink2, w - 28.0);
                        ui.painter().galley(r.min + vec2(14.0, 0.0), g, t.ink2);
                    }
                    _ => {
                        for (label, key, hint) in [("Shrink", "shrink", "the material's"), ("Base height", "base", "10"), ("Base margin", "margin", "5"), ("Vent Ø", "vent", "the material's (≤ 0.5 t)"), ("Vent pitch", "vent_pitch", "25")] {
                            row(ui, t, label, w, 32.0, |ui| {
                                let mut s = arg(a, key).unwrap_or("").to_string();
                                let r = ui.add(egui::TextEdit::singleline(&mut s).font(mono(13.0)).hint_text(hint).desired_width(ui.available_width()).margin(vec2(7.0, 5.0)));
                                places.push((format!("thermo:form:{key}"), r.rect));
                                set(a, key, s);
                            });
                        }
                    }
                }
                // the checks as the part stands
                ui.add_space(4.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                ui.painter().text(pos2(r.left() + 14.0, r.center().y), Align2::LEFT_CENTER, "Live checks", sans(12.0), t.ink2);
                let green = Color32::from_rgb(0x5c, 0xc0, 0x7a);
                let mut lines: Vec<(Color32, &str, String)> = vec![];
                for k in ["DraftTooLow", "Undercut", "RadiusTooTight", "DrawTooDeep", "Webbing", "ThinWall"] {
                    match issues.iter().find(|i| i.1 == k || (k == "RadiusTooTight" && i.1 == "SharpEdge")) {
                        Some((sev, _, text)) => lines.push((
                            if *sev == Severity::Error { t.error } else { super::fg::WARN },
                            "warning",
                            text.clone(),
                        )),
                        None => lines.push((green, "check", ok_word(k, row_data.as_ref()))),
                    }
                }
                for (c, icon, s) in lines {
                    let g = ui.painter().layout(s, sans(11.0), c, w - 46.0);
                    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 3.0), Sense::hover());
                    super::icons::draw(ui.painter(), r.min + vec2(14.0, 0.0), 13.0, icon, 1.4, c);
                    ui.painter().galley(r.min + vec2(32.0, 0.0), g, c);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(w - 14.0 - 72.0 * 2.0 - 10.0);
                    if button(ui, t, "Cancel", false, "thermo:form:cancel".into(), places) {
                        close = true;
                    }
                    if button(ui, t, "OK", true, "thermo:form:ok".into(), places) {
                        ok = true;
                    }
                });
                ui.add_space(12.0);
            });
        // the section preview beside the dialog
        if let Some(r) = &row_data {
            section(
                ctx,
                t,
                self.view_rect,
                r,
                f.args.iter().find(|a| a.0 == "flange").map(|a| a.1.as_str()),
            );
        }
        if ok {
            for op in changes(&f.id, &f.was, &f.args) {
                fg.out.push(FgAct::Apply(op));
            }
        } else if !close {
            fg.thermo.form = Some(f);
        }
    }

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
        let w = 360.0;
        let (mut close, mut apply) = (false, false);
        let mut export = None;
        let facts = fg
            .thermos
            .iter()
            .find(|r| r.id == m.id)
            .map(|r| r.facts.clone())
            .unwrap_or_default();
        egui::Window::new("Mould")
            .id(egui::Id::unique("thermo-mould"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.right() - w - 20.0, v.top() + 20.0))
            .fixed_size(vec2(w, 0.0))
            .frame(frame(t))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                close = title(ui, t, "Mould", w, places, "thermo:mould");
                for (label, key, hint) in [
                    ("Shrink", "shrink", "the material's"),
                    ("Base height", "base", "10"),
                    ("Base margin", "margin", "5 (≥ clamp)"),
                    ("Vent Ø", "vent", "≤ 0.5 t"),
                    ("Vent pitch", "vent_pitch", "25"),
                ] {
                    row(ui, t, label, w, 32.0, |ui| {
                        let mut s = arg(&m.args, key).unwrap_or("").to_string();
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut s)
                                .font(mono(13.0))
                                .hint_text(hint)
                                .desired_width(ui.available_width())
                                .margin(vec2(7.0, 5.0)),
                        );
                        places.push((format!("thermo:mould:{key}"), r.rect));
                        set(&mut m.args, key, s);
                    });
                }
                row(ui, t, "Vent placing", w, 32.0, |ui| {
                    let auto = arg(&m.args, "vents").is_none_or(str::is_empty);
                    ui.label(
                        egui::RichText::new(if auto {
                            "Auto: last contact + pitch"
                        } else {
                            "Given points"
                        })
                        .font(sans(13.0))
                        .color(t.ink),
                    );
                });
                row(ui, t, "Made by", w, 32.0, |ui| {
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
                for (k, v) in facts.iter().filter(|f| f.0 == "vents" || f.0 == "mould scale") {
                    let k = if k == "vents" { "Vents" } else { "Mould scale" };
                    row(ui, t, k, w, 24.0, |ui| {
                        ui.label(egui::RichText::new(v).font(mono(12.0)).color(t.ink));
                    });
                }
                ui.horizontal(|ui| {
                    ui.add_space(w - 14.0 - 72.0);
                    if button(ui, t, "Apply", true, "thermo:mould:apply".into(), places) {
                        apply = true;
                    }
                });
                ui.add_space(6.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
                ui.painter()
                    .hline(r.left()..=r.right(), r.top(), Stroke::new(1.0, t.line));
                ui.painter().text(
                    pos2(r.left() + 14.0, r.center().y + 2.0),
                    Align2::LEFT_CENTER,
                    "Export",
                    semi(14.0),
                    t.ink,
                );
                for (label, e) in [
                    ("Mould: STEP (for CAM)", Export::MouldStep),
                    ("Mould: STL (print)", Export::MouldStl),
                    ("Formed part: STEP", Export::PartStep),
                    ("Trim line: DXF", Export::TrimDxf),
                    (
                        "Process sheet: PDF (material, gauge, temp, blank, vents, thickness)",
                        Export::ProcessPdf,
                    ),
                ] {
                    if wide(ui, t, label, w, false, format!("thermo:export:{e:?}"), places) {
                        export = Some(e);
                    }
                }
                if let Some(s) = &fg.thermo.said {
                    let g = ui.painter().layout(s.clone(), sans(11.0), t.ink2, w - 28.0);
                    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 4.0), Sense::hover());
                    ui.painter().galley(r.min + vec2(14.0, 2.0), g, t.ink2);
                }
                ui.add_space(12.0);
            });
        if let Some(e) = export {
            fg.thermo.want_export = Some((m.id.clone(), e));
            fg.thermo.said = Some(format!("{}: {}", m.id, MADE_BY[m.made_by]));
        }
        if apply {
            for op in changes(&m.id, &m.was, &m.args) {
                fg.out.push(FgAct::Apply(op));
            }
            m.was = m.args.clone();
        }
        if !close {
            fg.thermo.mould = Some(m);
        }
    }

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
            .show(ctx, |ui| {
                frame(t).show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    close = title(
                        ui,
                        t,
                        "Issues · opened from the status bar",
                        w,
                        places,
                        "thermo:issues",
                    );
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
            .show(ctx, |ui| {
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

    /// The status bar's word on a part with a thermoform: its issues, a click away.
    pub(super) fn thermo_status(&mut self, ui: &mut Ui, right: f32, st: Rect) {
        let Some(fg) = &mut self.fg else { return };
        if fg.thermos.is_empty() {
            return;
        }
        let ids: Vec<&String> = fg.thermos.iter().map(|r| &r.id).collect();
        let (mut e, mut w) = (0, 0);
        for i in fg
            .issues
            .iter()
            .filter(|i| i.feature.as_ref().is_some_and(|f| ids.contains(&f)))
        {
            match i.severity {
                Severity::Error => e += 1,
                Severity::Lint => {}
                _ => w += 1,
            }
        }
        let text = format!(
            "Thermoform: {} · {}",
            super::fg::plural(e, "error"),
            super::fg::plural(w, "warning")
        );
        let g = ui.fonts_mut(|f| f.layout_no_wrap(text, sans(12.0), Color32::WHITE));
        let r = Rect::from_min_max(
            pos2(right - g.size().x - 22.0, st.top() + 2.0),
            pos2(right, st.bottom() - 2.0),
        );
        let resp = ui.interact(r, ui.scope_id().with("thermo-status"), Sense::click());
        let p = ui.painter();
        if resp.hovered() || fg.thermo.issues {
            p.rect_filled(r, 3.0, Color32::from_white_alpha(28));
        }
        let ink = if e > 0 {
            self.theme.error
        } else if w > 0 {
            super::fg::WARN
        } else {
            Color32::WHITE
        };
        super::icons::draw(
            p,
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
        self.places.push(("thermo:status".into(), r));
        if resp.clicked() {
            fg.thermo.issues = !fg.thermo.issues;
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

/// What a check says when it passes.
fn ok_word(kind: &str, r: Option<&ThermoRow>) -> String {
    let mat = r.map_or("", |r| r.material.as_str());
    match kind {
        "DraftTooLow" => format!(
            "Draft at least {:.1}° on every wall ({mat})",
            r.map_or(0.0, |r| r.draft_min)
        ),
        "Undercut" => format!("No undercuts along {}", r.map_or("the pull", |r| r.pull.as_str())),
        "RadiusTooTight" | "SharpEdge" => "Every edge rounded enough for the sheet".into(),
        "DrawTooDeep" => format!("Draw ratio within {mat}'s"),
        "Webbing" => "No webbing between raised parts".into(),
        _ => "Wall above the material's least".into(),
    }
}

/// The section through the thermoform beside the Form dialog: the mould, the formed sheet, the
/// clamp and the pull, from the model's own numbers.
fn section(ctx: &egui::Context, t: &Theme, v: Rect, r: &ThermoRow, flange: Option<&str>) {
    if r.section.is_empty() {
        return;
    }
    let area = Rect::from_min_max(
        pos2(v.left() + DLG_W + 40.0, v.top() + 40.0),
        pos2(v.right() - 40.0, v.top() + 360.0),
    );
    if area.width() < 200.0 {
        return;
    }
    egui::Area::new(egui::Id::unique("thermo-section"))
        .fixed_pos(area.min)
        .order(egui::Order::Middle)
        .interactable(false)
        .show(ctx, |ui| {
            let p = ui.painter();
            p.rect_filled(area.expand(16.0), 6.0, t.bg.gamma_multiply(0.96));
            let (lo, hi) = r
                .section
                .iter()
                .fold(([f32::MAX; 2], [f32::MIN; 2]), |(a, b), q| {
                    (
                        [a[0].min(q[0]), a[1].min(q[1] - r.base)],
                        [b[0].max(q[0]), b[1].max(q[1] + r.t as f32)],
                    )
                });
            let s = ((area.width() - 80.0) / (hi[0] - lo[0])).min((area.height() - 90.0) / (hi[1] - lo[1]));
            let at = |q: [f32; 2]| {
                pos2(
                    area.center().x + (q[0] - (lo[0] + hi[0]) / 2.0) * s,
                    area.bottom() - 40.0 - (q[1] - lo[1]) * s,
                )
            };
            // the mould: its profile down to the base
            let mut mould: Vec<Pos2> = r.section.iter().map(|q| at(*q)).collect();
            let (a, b) = (r.section[0], r.section[r.section.len() - 1]);
            mould.push(at([b[0], lo[1]]));
            mould.push(at([a[0], lo[1]]));
            let grey = Color32::from_rgb(0x7d, 0x87, 0x94);
            p.add(egui::Shape::Path(egui::epaint::PathShape {
                points: mould,
                closed: true,
                fill: grey,
                stroke: Stroke::new(1.0, Color32::from_rgb(0x9a, 0xa4, 0xb0)).into(),
            }));
            let base = Rect::from_min_max(at([a[0], lo[1] + r.base * 0.999]), at([b[0], lo[1]]));
            p.text(
                base.center(),
                Align2::CENTER_CENTER,
                "Mould base (generated)",
                sans(12.0),
                t.ink,
            );
            // the formed sheet, t thick, off the mould
            let gold = Color32::from_rgb(0xe0, 0xa8, 0x3a);
            let sheet: Vec<Pos2> = r.section_part.iter().map(|q| at(*q)).collect();
            p.add(egui::Shape::line(
                sheet,
                Stroke::new((r.t as f32 * s).max(3.0), gold),
            ));
            p.text(
                at([hi[0], hi[1]]) + vec2(-4.0, -6.0),
                Align2::RIGHT_BOTTOM,
                format!("formed part, t = {}", fmt(r.t)),
                sans(12.0),
                gold,
            );
            let top = at([(lo[0] + hi[0]) / 2.0, hi[1]]);
            p.arrow(
                top + vec2(0.0, 6.0),
                vec2(0.0, -40.0),
                Stroke::new(1.5, Color32::from_rgb(0x7f, 0xb2, 0xec)),
            );
            p.text(
                top + vec2(12.0, -30.0),
                Align2::LEFT_CENTER,
                format!("pull {}", r.pull),
                sans(12.0),
                Color32::from_rgb(0x7f, 0xb2, 0xec),
            );
            p.text(
                at([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]),
                Align2::CENTER_CENTER,
                format!(
                    "Mould = your shape {}, + {:.2} % shrink",
                    if r.from == "outer" && !r.female {
                        "− t"
                    } else {
                        ""
                    },
                    r.shrink * 100.0
                ),
                sans(12.0),
                t.ink,
            );
            p.text(
                pos2(area.center().x, area.bottom() - 10.0),
                Align2::CENTER_CENTER,
                format!(
                    "Preview · section through {} · flange {}",
                    r.id,
                    flange.unwrap_or("10")
                ),
                sans(12.0),
                t.ink2,
            );
        });
}

/// The banded legend's colour for a thickness between the range's ends.
pub fn legend_colour(f: f32) -> Color32 {
    c32(band(f))
}

/// The browser's heading over a thermoform part's rows (none on a part without one).
pub fn tree_heading(ui: &mut Ui, t: &Theme, fg: &FgPanel, w: f32, what: &str, note: &str) {
    if fg.thermos.is_empty() {
        return;
    }
    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    let g = ui.painter().layout_no_wrap(what.into(), sans(10.5), t.ink2);
    let gw = g.size().x;
    ui.painter()
        .galley(pos2(r.left() + 14.0, r.center().y - g.size().y / 2.0), g, t.ink2);
    ui.painter().text(
        pos2(r.left() + 20.0 + gw, r.center().y),
        Align2::LEFT_CENTER,
        format!("({note})"),
        sans(10.5),
        t.ink2.gamma_multiply(0.8),
    );
}

/// What each thermoform generated, in the browser under the shape: the formed part, the mould,
/// the blank; a click opens what shows it.
pub fn tree_generated(ui: &mut Ui, t: &Theme, fg: &mut FgPanel, w: f32, places: &mut Vec<(String, Rect)>) {
    if fg.thermos.is_empty() {
        return;
    }
    ui.add_space(6.0);
    tree_heading(ui, t, fg, w, "GENERATED", "from the shape");
    let rows: Vec<(String, String, String, &'static str, Cmd)> = fg
        .thermos
        .iter()
        .flat_map(|r| {
            let fact = |k: &str| {
                r.facts
                    .iter()
                    .find(|f| f.0 == k)
                    .map_or(String::new(), |f| f.1.clone())
            };
            [
                (
                    r.id.clone(),
                    "Formed part".to_string(),
                    format!("{:.2} nominal", r.t),
                    "thicken",
                    Cmd::Thickness,
                ),
                (
                    r.id.clone(),
                    "Mould".to_string(),
                    format!("+{:.2} % · {} vents", r.shrink * 100.0, fact("vents")),
                    "extrude",
                    Cmd::Mould,
                ),
                (
                    r.id.clone(),
                    "Blank".to_string(),
                    fact("sheet used"),
                    "rect",
                    Cmd::Trim,
                ),
            ]
        })
        .collect();
    for (id, name, note, icon, cmd) in rows {
        let (r, resp) = ui.allocate_exact_size(vec2(w, super::ROW_H), Sense::click());
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(r, 0.0, t.hover);
        }
        super::icons::draw(p, pos2(r.left() + 20.0, r.top() + 5.0), 16.0, icon, 1.5, t.ink);
        let g = p.layout_no_wrap(name.clone(), sans(13.0), t.ink);
        let gw = g.size().x;
        p.galley(pos2(r.left() + 42.0, r.center().y - g.size().y / 2.0), g, t.ink);
        p.text(
            pos2(r.left() + 48.0 + gw, r.center().y),
            Align2::LEFT_CENTER,
            note,
            sans(11.0),
            t.ink2,
        );
        places.push((format!("fg:generated:{id}:{name}"), r));
        if resp.clicked() {
            fg.thermo_command(cmd);
        }
    }
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
        .show(ctx, |ui| {
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
