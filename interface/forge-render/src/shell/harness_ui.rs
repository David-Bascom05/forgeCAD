//! Wire harnesses in the app, Inventor's Cable and Harness way: a Harness tab in an assembly
//! (Create Harness, Add Connector or Pin, Route Segment, Add Wire, Route Wires, Import and Export
//! Wire List), each a dialog that writes the harness's .fg lines. The browser's Harness branch and
//! the status bar's line are fed from the run (fgasm.rs, gui.rs).
use super::fg::{FgAct, FgPanel, Issue};
use super::theme::{Theme, sans};
use super::thermo_ui::{Button, DLG_W, foot, frame, head, heading, note, row, tabs};
use super::{Action, FgCmd, Shell};
use crate::shell::placed::Placed;
use egui::{Rect, pos2, vec2};
use forge_lang::error::Op;
use forge_lang::harness::{Diff, HarnessOut, HarnessProblem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Create,
    Connector,
    Segment,
    Wire,
    Route,
    Import,
    Export,
    WireType,
    Library,
}

#[derive(Clone, Debug, Default)]
pub struct HarnessUi {
    pub dialog: Option<Dialog>,
    /// The selected row the status bar last spoke for.
    pub said: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Dialog {
    Create {
        name: String,
        slack: String,
        round_up: String,
    },
    Connector {
        h: usize,
        /// 0: a connector (a part's face and pins), 1: a pin (one anchor, a one-pin connector).
        tab: usize,
        name: String,
        inst: String,
        face: String,
        pins: String,
        out: String,
    },
    Segment {
        h: usize,
        from: String,
        to: String,
        via: String,
        /// A node made with it: its name, reference, offset and axis ("" none).
        node: String,
        at: String,
        off: String,
        along: String,
    },
    Wire {
        h: usize,
        name: String,
        from: String,
        from_pin: String,
        to: String,
        to_pin: String,
        ty: String,
        /// Its colour code, base and stripes (empty: the type's).
        colour: [String; 3],
    },
    /// A wire type, new or edited: every field by its key (`SPEC`).
    WireType {
        h: usize,
        name: String,
        fields: Vec<(String, String)>,
        editing: bool,
        /// The type being edited as it was worked out: its name and values (a param's).
        was: Option<forge_lang::harness::TypeOut>,
    },
    /// The wire library: its rows, a search, the conductor and insulation chips on, and the
    /// column it is sorted by (and whether rising).
    Library {
        h: usize,
        search: String,
        chips: Vec<String>,
        sort: (usize, bool),
        rows: Vec<forge_lang::harness::LibRow>,
        problem: Option<String>,
        picked: Option<usize>,
    },
    Route {
        h: usize,
    },
    Import {
        h: usize,
        path: String,
        preview: Option<Result<(Vec<Op>, Diff), String>>,
    },
    Export {
        h: usize,
        path: String,
        done: Option<Result<String, String>>,
    },
}

impl Shell {
    /// An assembly has the Harness tab.
    pub fn harness_tab(&self) -> bool {
        self.is_asm()
    }
}

fn harnesses(fg: &FgPanel) -> &[HarnessOut] {
    fg.asm.as_ref().map_or(&[], |a| a.harnesses.as_slice())
}

/// The ribbon's Harness tab.
pub fn groups(s: &Shell) -> Vec<(&'static str, Vec<Button>, Vec<Button>)> {
    let fg = s.fg.as_ref();
    let has = fg.is_some_and(|f| !harnesses(f).is_empty());
    let up = |c: Cmd| {
        fg.and_then(|f| f.harness.dialog.as_ref()).is_some_and(|d| {
            matches!(
                (d, c),
                (Dialog::Create { .. }, Cmd::Create)
                    | (Dialog::Connector { .. }, Cmd::Connector)
                    | (Dialog::Segment { .. }, Cmd::Segment)
                    | (Dialog::Wire { .. }, Cmd::Wire)
                    | (Dialog::Route { .. }, Cmd::Route)
                    | (Dialog::Import { .. }, Cmd::Import)
                    | (Dialog::Export { .. }, Cmd::Export)
                    | (Dialog::WireType { .. }, Cmd::WireType)
                    | (Dialog::Library { .. }, Cmd::Library)
            )
        })
    };
    let go = |c: Cmd, ok: bool| ok.then_some(Action::Fg(FgCmd::Harness(c)));
    vec![
        (
            "Create",
            vec![
                (
                    "Create Harness",
                    "harness",
                    go(Cmd::Create, fg.is_some()),
                    up(Cmd::Create),
                ),
                (
                    "Add Connector or\u{a0}Pin",
                    "connector",
                    go(Cmd::Connector, has),
                    up(Cmd::Connector),
                ),
            ],
            vec![],
        ),
        (
            "Route",
            vec![
                ("Route Segment", "sweep", go(Cmd::Segment, has), up(Cmd::Segment)),
                ("Add Wire", "wire", go(Cmd::Wire, has), up(Cmd::Wire)),
                ("Route Wires", "harness", go(Cmd::Route, has), up(Cmd::Route)),
            ],
            vec![],
        ),
        (
            "Wires",
            vec![
                ("Wire Type", "wire", go(Cmd::WireType, has), up(Cmd::WireType)),
                ("Wire Library", "folder", go(Cmd::Library, has), up(Cmd::Library)),
            ],
            vec![],
        ),
        (
            "Manage",
            vec![],
            vec![
                ("Import Wire List", "open", go(Cmd::Import, has), up(Cmd::Import)),
                ("Export Wire List", "save", go(Cmd::Export, has), up(Cmd::Export)),
            ],
        ),
    ]
}

/// The status bar's words for the harnesses: wires routed, and the bend and route warnings.
pub fn status(hs: &[HarnessOut], issues: &[Issue]) -> String {
    if hs.is_empty() {
        return String::new();
    }
    let wires: usize = hs.iter().map(|h| h.wires.len()).sum();
    let routed: usize = hs
        .iter()
        .map(|h| h.wires.iter().filter(|w| w.routed.is_some()).count())
        .sum();
    let (mut bends, mut routes) = (0, 0);
    for i in issues {
        if let Some(forge_lang::asm::Issue::Harness(p)) = &i.asm {
            match p {
                HarnessProblem::BendTooTight { .. } => bends += 1,
                HarnessProblem::KinkAtNode { .. }
                | HarnessProblem::NoPath { .. }
                | HarnessProblem::PathBroken { .. } => routes += 1,
                _ => {}
            }
        }
    }
    let mut s = if routed == wires {
        format!(" · {wires} wires routed")
    } else {
        format!(" · {routed} of {wires} wires routed")
    };
    if bends > 0 {
        s.push_str(&format!(" · {}", super::fg::plural(bends, "tight bend")));
    }
    if routes > 0 {
        s.push_str(&format!(" · {}", super::fg::plural(routes, "route warning")));
    }
    s
}

/// The harness's items by kind, as named in it.
fn names(h: &HarnessOut) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let conns = h.connectors.iter().map(|c| c.name.clone()).collect();
    let nodes = h.nodes.iter().map(|n| n.name.clone()).collect();
    let segs = h.segments.iter().map(|s| s.name.clone()).collect();
    let mut types: Vec<String> = h.wires.iter().map(|w| w.ty.clone()).collect();
    types.sort();
    types.dedup();
    (conns, nodes, segs, types)
}

/// The first name `stem1`, `stem2`, .. no item has.
fn next_name(stem: &str, taken: &[String]) -> String {
    (1..)
        .map(|k| format!("{stem}{k}"))
        .find(|n| !taken.contains(n))
        .expect("a free name")
}

/// Where an op inside harness `h` goes: its block's end (`loom.`).
fn at_end(h: &HarnessOut) -> Option<String> {
    h.file.is_none().then(|| format!("{}.", h.id))
}

impl FgPanel {
    pub fn harness_command(&mut self, c: Cmd) {
        let hs = harnesses(self);
        let h = 0;
        let first = hs.first();
        let taken: Vec<String> = first
            .map(|h| {
                let (c, n, s, _) = names(h);
                c.into_iter()
                    .chain(n)
                    .chain(s)
                    .chain(h.wires.iter().map(|w| w.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let open = match c {
            Cmd::Create => Dialog::Create {
                name: next_name("harness", &hs.iter().map(|h| h.id.clone()).collect::<Vec<_>>()),
                slack: "0.03".into(),
                round_up: "5".into(),
            },
            Cmd::Connector => Dialog::Connector {
                h,
                tab: 0,
                name: next_name("J", &taken),
                inst: self
                    .asm
                    .as_ref()
                    .and_then(|a| a.insts.iter().find(|i| !i.path.contains('.')))
                    .map(|i| i.path.clone())
                    .unwrap_or_default(),
                face: "face".into(),
                pins: "pin".into(),
                out: "10".into(),
            },
            Cmd::Segment => {
                let nodes = first.map(|h| names(h).1).unwrap_or_default();
                Dialog::Segment {
                    h,
                    from: nodes.first().cloned().unwrap_or_default(),
                    to: nodes.get(1).cloned().unwrap_or_default(),
                    via: String::new(),
                    node: String::new(),
                    at: String::new(),
                    off: String::new(),
                    along: String::new(),
                }
            }
            Cmd::Wire => {
                let (conns, _, _, types) = first.map(names).unwrap_or_default();
                Dialog::Wire {
                    h,
                    name: next_name("W", &taken),
                    from: conns.first().cloned().unwrap_or_default(),
                    from_pin: "1".into(),
                    to: conns.get(1).cloned().unwrap_or_default(),
                    to_pin: "1".into(),
                    ty: types.first().cloned().unwrap_or_default(),
                    colour: Default::default(),
                }
            }
            Cmd::WireType => {
                // the selected wire's type to edit, else a new one
                let sel = self
                    .asm
                    .as_ref()
                    .and_then(|a| a.selected.clone())
                    .unwrap_or_default();
                let edit = first.and_then(|h| {
                    let key = sel.strip_prefix(&format!("h:{}:", h.id))?;
                    let ty = match key.split(':').collect::<Vec<_>>().as_slice() {
                        ["t", t] => t.to_string(),
                        ["w", w] => h.wires.iter().find(|x| x.name == *w)?.ty.clone(),
                        _ => return None,
                    };
                    h.types.iter().find(|t| t.name == ty).cloned()
                });
                match edit {
                    Some(t) => Dialog::WireType {
                        h,
                        name: t.name.clone(),
                        // as written (a param stays the param), else as worked out
                        fields: first
                            .filter(|h| h.file.is_none())
                            .and_then(|h| forge_lang::harness::call_args(&self.source, &h.id, &t.name))
                            .unwrap_or_else(|| t.spec.clone()),
                        editing: true,
                        was: Some(t.clone()),
                    },
                    None => Dialog::WireType {
                        h,
                        name: next_name(
                            "wt",
                            &first
                                .map(|h| h.types.iter().map(|t| t.name.clone()).collect::<Vec<_>>())
                                .unwrap_or_default(),
                        ),
                        fields: vec![("od".into(), String::new()), ("source".into(), String::new())],
                        editing: false,
                        was: None,
                    },
                }
            }
            Cmd::Library => {
                let dir = std::path::Path::new(&self.path).parent().map(|d| d.to_path_buf());
                let (rows, bad) = forge_lang::harness::library(dir.as_deref());
                Dialog::Library {
                    h,
                    search: String::new(),
                    chips: vec![],
                    sort: (1, true),
                    rows,
                    problem: bad.map(|p| p.text()),
                    picked: None,
                }
            }
            Cmd::Route => Dialog::Route { h },
            Cmd::Import => Dialog::Import {
                h,
                path: "wires.csv".into(),
                preview: None,
            },
            Cmd::Export => Dialog::Export {
                h,
                path: format!("{}-wires.csv", first.map_or("harness", |h| h.id.as_str())),
                done: None,
            },
        };
        // asked again: the dialog goes
        let same = self
            .harness
            .dialog
            .as_ref()
            .is_some_and(|d| std::mem::discriminant(d) == std::mem::discriminant(&open));
        self.harness.dialog = if same { None } else { Some(open) };
    }

    /// A file named in a dialog, beside the document.
    fn beside(&self, name: &str) -> std::path::PathBuf {
        let p = std::path::Path::new(name);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        std::path::Path::new(&self.path)
            .parent()
            .map_or_else(|| p.to_path_buf(), |d| d.join(p))
    }
}

fn line_edit(
    ui: &mut egui::Ui,
    t: &Theme,
    label: &str,
    s: &mut String,
    hint: &str,
    key: &str,
    places: &mut Vec<(String, Rect)>,
) {
    row(ui, t, label, DLG_W, 28.0, |ui| {
        let r = ui.add(
            egui::TextEdit::singleline(s)
                .font(sans(13.0))
                .hint_text(hint)
                .desired_width(ui.available_width() - 6.0)
                .margin(vec2(7.0, 5.0)),
        );
        places.push((format!("harness:{key}"), r.rect));
    });
}

#[allow(clippy::too_many_arguments)]
fn pick(
    ui: &mut egui::Ui,
    t: &Theme,
    label: &str,
    now: &mut String,
    from: &[String],
    key: &str,
    places: &mut Vec<(String, Rect)>,
) {
    row(ui, t, label, DLG_W, 30.0, |ui| {
        let r = egui::ComboBox::from_id_salt(("harness-pick", key))
            .selected_text(now.as_str())
            .width(ui.available_width() - 6.0)
            .show_ui(ui, |ui| {
                for x in from {
                    ui.selectable_value(now, x.clone(), x.as_str());
                }
            });
        places.push((format!("harness:{key}"), r.response.rect));
    });
}

/// `12`, or a name (a param): what an argument's text may be.
fn arg_text(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let ok = s.parse::<f64>().is_ok()
        || s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    ok.then(|| s.to_string())
}

/// What a dialog writes, by its fields; Err: what is missing.
pub fn dialog_ops(d: &Dialog, hs: &[HarnessOut]) -> Result<Vec<Op>, String> {
    let h = |k: usize| hs.get(k).ok_or_else(|| "no harness".to_string());
    let add = |h: &HarnessOut, text: String| Op::AddFeature {
        before: at_end(h),
        text,
    };
    let need = |s: &str, what: &str| {
        if s.trim().is_empty() {
            Err(format!("{what} is needed"))
        } else {
            Ok(s.trim().to_string())
        }
    };
    match d {
        Dialog::Create {
            name,
            slack,
            round_up,
        } => {
            let name = need(name, "a name")?;
            let mut args = vec![];
            if let Some(s) = arg_text(slack) {
                args.push(format!("slack: ratio({s})"));
            }
            if let Some(r) = arg_text(round_up) {
                args.push(format!("round_up: {r}"));
            }
            Ok(vec![Op::AddFeature {
                before: None,
                text: format!("harness {name}({}) {{\n}}", args.join(", ")),
            }])
        }
        Dialog::Connector {
            h: k,
            tab,
            name,
            inst,
            face,
            pins,
            out,
        } => {
            let h = h(*k)?;
            let name = need(name, "a name")?;
            let inst = need(inst, "a component")?;
            let face = need(face, "an anchor")?;
            let line = if *tab == 0 {
                format!(
                    "{name} = connector({inst}.{face}, pins: {inst}.{})",
                    need(pins, "the pins' name")?
                )
            } else {
                format!("{name} = connector({inst}.{face}, pins: {{p: {inst}.{face}}})")
            };
            let node = format!(
                "n_{} = node({name}, out: {})",
                name.to_lowercase(),
                arg_text(out).unwrap_or_else(|| "0".into())
            );
            Ok(vec![add(h, line), add(h, node)])
        }
        Dialog::Segment {
            h: k,
            from,
            to,
            via,
            node,
            at,
            off,
            along,
        } => {
            let h = h(*k)?;
            let mut ops = vec![];
            if !node.trim().is_empty() {
                let mut args = vec![need(at, "the new node's place")?];
                if let Some(o) = arg_text(off) {
                    args.push(format!("off: {o}"));
                }
                if ["x", "y", "z"].contains(&along.trim()) {
                    args.push(format!("along: \"{}\"", along.trim()));
                }
                ops.push(add(h, format!("{} = node({})", node.trim(), args.join(", "))));
            }
            let (a, b) = (need(from, "a start node")?, need(to, "an end node")?);
            let vias: Vec<&str> = via.split(',').map(str::trim).filter(|x| !x.is_empty()).collect();
            let seg = next_name(
                "s",
                &h.segments.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
            );
            let line = if vias.is_empty() {
                format!("{seg} = segment({a}, {b})")
            } else {
                format!("{seg} = segment({a}, {b}, via: [{}])", vias.join(", "))
            };
            ops.push(add(h, line));
            Ok(ops)
        }
        Dialog::Wire {
            h: k,
            name,
            from,
            from_pin,
            to,
            to_pin,
            ty,
            colour,
        } => {
            let h = h(*k)?;
            let pin = |c: &str, p: &str| -> Result<String, String> {
                let (c, p) = (need(c, "a connector")?, need(p, "a pin")?);
                Ok(if p.chars().all(|x| x.is_ascii_digit()) {
                    format!("{c}[{p}]")
                } else {
                    format!("{c}.{p}")
                })
            };
            let code = colour_text(colour);
            let tail = if code.is_empty() {
                String::new()
            } else {
                format!(", colour: \"{code}\"")
            };
            Ok(vec![add(
                h,
                format!(
                    "{} = wire({}, {}, type: {}{tail})",
                    need(name, "a name")?,
                    pin(from, from_pin)?,
                    pin(to, to_pin)?,
                    need(ty, "a wire type")?
                ),
            )])
        }
        Dialog::WireType {
            h: k,
            name,
            fields,
            editing,
            was,
        } => {
            let h = h(*k)?;
            let name = need(name, "a name")?;
            let line = format!("{name} = {}", wire_type_call(fields)?);
            let old = was.as_ref().map_or(name.as_str(), |w| w.name.as_str());
            Ok(vec![if *editing {
                Op::ReplaceFeature {
                    id: format!("{}{old}", at_end(h).unwrap_or_default()),
                    text: line,
                }
            } else {
                add(h, line)
            }])
        }
        Dialog::Library {
            h: k,
            rows,
            picked: Some(i),
            ..
        } => {
            let h = h(*k)?;
            let row = rows.get(*i).ok_or("no row picked")?;
            let taken: Vec<String> = h.types.iter().map(|t| t.name.clone()).collect();
            let stem: String = row
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .collect();
            let stem = if stem.starts_with(|c: char| c.is_ascii_alphabetic()) {
                stem
            } else {
                format!("w_{stem}")
            };
            let name = if taken.contains(&stem) {
                next_name(&stem, &taken)
            } else {
                stem
            };
            Ok(vec![add(
                h,
                format!("{name} = wire_type(\"{}\")", row.name.replace('"', "")),
            )])
        }
        _ => Ok(vec![]),
    }
}

impl Shell {
    /// The Harness tab's dialogs.
    pub(super) fn harness_ui(&mut self, ctx: &egui::Context) {
        if self.tab == super::HARNESS_TAB && !self.harness_tab() {
            self.tab = 0;
        }
        let Some(mut fg) = self.fg.take() else { return };
        // a selected harness row's properties in the status bar, once each time it changes
        let sel = fg.asm.as_ref().and_then(|a| a.selected.clone());
        if sel != fg.harness.said {
            if let Some(words) = fg
                .asm
                .as_ref()
                .zip(sel.as_deref())
                .and_then(|(a, k)| selection_status(&a.harnesses, k))
            {
                self.status = words;
            }
            fg.harness.said = sel;
        }
        let Some(mut d) = fg.harness.dialog.take() else {
            self.fg = Some(fg);
            return;
        };
        let t = self.theme;
        let v = self.view_rect;
        let mut places = vec![];
        let (mut ok, mut close, mut apply) = (false, false, false);
        let hs: Vec<HarnessOut> = harnesses(&fg).to_vec();
        let insts: Vec<String> = fg
            .asm
            .as_ref()
            .map(|a| {
                a.insts
                    .iter()
                    .filter(|i| !i.path.contains('.'))
                    .map(|i| i.path.clone())
                    .collect()
            })
            .unwrap_or_default();
        let source = fg.source.clone();
        let (title, icon) = match &d {
            Dialog::Create { .. } => ("Create Harness", "harness"),
            Dialog::Connector { .. } => ("Add Connector or Pin", "connector"),
            Dialog::Segment { .. } => ("Route Segment", "sweep"),
            Dialog::Wire { .. } => ("Add Wire", "wire"),
            Dialog::Route { .. } => ("Route Wires", "harness"),
            Dialog::Import { .. } => ("Import Wire List", "open"),
            Dialog::Export { .. } => ("Export Wire List", "save"),
            Dialog::WireType { editing: true, .. } => ("Edit Wire Type", "wire"),
            Dialog::WireType { .. } => ("Wire Type", "wire"),
            Dialog::Library { .. } => ("Wire Library", "folder"),
        };
        let mut problem: Option<String> = None;
        let wide = matches!(d, Dialog::WireType { .. } | Dialog::Library { .. } | Dialog::Wire { .. });
        if wide {
            let (o, x, r) = super::harness_look::wide(ctx, &t, v, title, icon, &mut d, &hs, &mut places);
            ok |= o;
            close |= x;
            if let Some(r) = r {
                self.view_blocks.push(r);
            }
        }
        if !wide {
        egui::Window::new(title)
            .id(egui::Id::unique(title))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
            .fixed_size(vec2(DLG_W, 0.0))
            .frame(frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                ui.spacing_mut().item_spacing.y = 6.0;
                close = head(ui, &t, icon, title, DLG_W, places, "harness:dialog");
                let h_of = |k: usize| hs.get(k);
                let in_file = |k: usize| h_of(k).and_then(|h| h.file.clone());
                match &mut d {
                    Dialog::Create {
                        name,
                        slack,
                        round_up,
                    } => {
                        line_edit(ui, &t, "Name", name, "loom", "name", places);
                        line_edit(ui, &t, "Slack", slack, "0.03", "slack", places);
                        line_edit(ui, &t, "Round up to (mm)", round_up, "5", "round", places);
                    }
                    Dialog::Connector {
                        h,
                        tab,
                        name,
                        inst,
                        face,
                        pins,
                        out,
                    } => {
                        harness_pick(ui, &t, h, &hs, places);
                        tabs(ui, &t, &["Connector", "Pin"], tab, DLG_W, places, "harness:conn");
                        line_edit(ui, &t, "Name", name, "J1", "conn-name", places);
                        pick(ui, &t, "Component", inst, &insts, "conn-inst", places);
                        line_edit(ui, &t, "Anchor", face, "face", "conn-face", places);
                        if *tab == 0 {
                            line_edit(ui, &t, "Pins (anchors)", pins, "pin", "conn-pins", places);
                        }
                        line_edit(ui, &t, "Exit (mm out)", out, "10", "conn-out", places);
                        if let Some(f) = in_file(*h) {
                            note(ui, &format!("written in {f}"), DLG_W, t.ink2);
                        }
                    }
                    Dialog::Segment {
                        h,
                        from,
                        to,
                        via,
                        node,
                        at,
                        off,
                        along,
                    } => {
                        harness_pick(ui, &t, h, &hs, places);
                        let mut nodes = h_of(*h).map(|x| names(x).1).unwrap_or_default();
                        if !node.trim().is_empty() {
                            nodes.push(node.trim().to_string());
                        }
                        pick(ui, &t, "From node", from, &nodes, "seg-from", places);
                        pick(ui, &t, "To node", to, &nodes, "seg-to", places);
                        line_edit(
                            ui,
                            &t,
                            "Via (anchors)",
                            via,
                            "frame.corner_a, frame.corner_b",
                            "seg-via",
                            places,
                        );
                        heading(ui, &t, "New node", DLG_W);
                        line_edit(ui, &t, "Name", node, "clip1", "node-name", places);
                        line_edit(ui, &t, "At (anchor)", at, "plate.clip", "node-at", places);
                        line_edit(ui, &t, "Offset (mm)", off, "0", "node-off", places);
                        let axes = vec![String::new(), "x".into(), "y".into(), "z".into()];
                        pick(ui, &t, "Along", along, &axes, "node-along", places);
                    }
                    Dialog::Wire { .. } | Dialog::WireType { .. } | Dialog::Library { .. } => {}
                    Dialog::Route { h } => {
                        harness_pick(ui, &t, h, &hs, places);
                        if let Some(ho) = h_of(*h) {
                            for w in &ho.wires {
                                let path = if w.routed.is_some() {
                                    w.path.join(" › ")
                                } else {
                                    "no path".into()
                                };
                                let mark = if w.given { "written" } else { "shortest" };
                                note(ui, &format!("{}  {}  ({mark})", w.name, path), DLG_W, t.ink);
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            if super::thermo_ui::button(
                                ui,
                                &t,
                                "Write Paths",
                                false,
                                "harness:route:pin".into(),
                                places,
                            ) {
                                apply = true;
                            }
                            if super::thermo_ui::button(
                                ui,
                                &t,
                                "Clear Paths",
                                false,
                                "harness:route:clear".into(),
                                places,
                            ) {
                                ok = true;
                            }
                        });
                    }
                    Dialog::Import { h, path, preview } => {
                        harness_pick(ui, &t, h, &hs, places);
                        line_edit(
                            ui,
                            &t,
                            "Wire list (CSV)",
                            path,
                            "wires.csv",
                            "import-path",
                            places,
                        );
                        if super::thermo_ui::button(
                            ui,
                            &t,
                            "Preview",
                            false,
                            "harness:import:preview".into(),
                            places,
                        ) {
                            *preview = Some(import_preview(&fg, &source, &hs, *h, path));
                        }
                        match preview {
                            Some(Ok((ops, diff))) if ops.is_empty() => {
                                let _ = diff;
                                note(ui, "the list is what the harness holds", DLG_W, t.ink2)
                            }
                            Some(Ok((_, diff))) => {
                                for l in diff.text().lines() {
                                    note(ui, l, DLG_W, t.ink);
                                }
                            }
                            Some(Err(e)) => note(ui, e, DLG_W, t.error),
                            None => {}
                        }
                    }
                    Dialog::Export { h, path, done } => {
                        harness_pick(ui, &t, h, &hs, places);
                        line_edit(ui, &t, "File (CSV)", path, "wires.csv", "export-path", places);
                        match done {
                            Some(Ok(p)) => note(ui, &format!("written: {p}"), DLG_W, t.ink2),
                            Some(Err(e)) => note(ui, e, DLG_W, t.error),
                            None => {}
                        }
                    }
                }
                if let Some(p) = &problem {
                    note(ui, p, DLG_W, t.error);
                }
                if !matches!(d, Dialog::Route { .. }) {
                    let (o, x, _) = foot(ui, &t, "harness:dialog", false, places);
                    ok |= o;
                    close |= x;
                }
            });
        }
        let keep = match (&mut d, ok || apply) {
            (_, false) => !close,
            (Dialog::Route { h }, true) => {
                match hs
                    .get(*h)
                    .map(|ho| forge_lang::harness::route_ops(&source, ho, apply))
                {
                    Some(Ok(ops)) if !ops.is_empty() => fg.out.push(FgAct::AsmOps(ops)),
                    Some(Err(e)) => problem = Some(e),
                    _ => {}
                }
                true
            }
            (Dialog::Import { h, path, preview }, true) => {
                let p = preview
                    .take()
                    .unwrap_or_else(|| import_preview(&fg, &source, &hs, *h, path));
                match (p, hs.get(*h).and_then(|x| x.file.clone())) {
                    (Ok((ops, _)), None) => {
                        if !ops.is_empty() {
                            fg.out.push(FgAct::AsmOps(ops));
                        }
                        false
                    }
                    // a harness file: the ops written there (the assembly reads it again)
                    (Ok((ops, _)), Some(f)) => {
                        let file = fg.beside(&f);
                        let text = std::fs::read_to_string(&file).unwrap_or_default();
                        match forge_lang::edit::apply_all_checked(&text, &ops) {
                            Ok(new) => {
                                let _ = std::fs::write(&file, new);
                                false
                            }
                            Err(e) => {
                                *preview = Some(Err(e.text()));
                                true
                            }
                        }
                    }
                    (Err(e), _) => {
                        *preview = Some(Err(e));
                        true
                    }
                }
            }
            (Dialog::Export { h, path, done }, true) => {
                let file = fg.beside(path);
                *done = Some(match hs.get(*h) {
                    Some(ho) => std::fs::write(&file, forge_lang::harness::export_csv(ho))
                        .map(|_| file.to_string_lossy().to_string())
                        .map_err(|e| e.to_string()),
                    None => Err("no harness".into()),
                });
                matches!(done, Some(Err(_)))
            }
            (d, true) => match dialog_ops(d, &hs) {
                Ok(ops) => {
                    if !ops.is_empty() {
                        fg.out.push(FgAct::AsmOps(ops));
                    }
                    false
                }
                Err(e) => {
                    problem = Some(e);
                    true
                }
            },
        };
        if let Some(p) = problem {
            if let Dialog::Import { preview, .. } = &mut d {
                *preview = Some(Err(p));
            } else {
                self.status = p;
            }
        }
        if keep {
            fg.harness.dialog = Some(d);
        }
        self.places.extend(places);
        self.fg = Some(fg);
    }
}

fn harness_pick(
    ui: &mut egui::Ui,
    t: &Theme,
    h: &mut usize,
    hs: &[HarnessOut],
    places: &mut Vec<(String, Rect)>,
) {
    if hs.len() < 2 {
        return;
    }
    row(ui, t, "Harness", DLG_W, 30.0, |ui| {
        let r = egui::ComboBox::from_id_salt("harness-which")
            .selected_text(hs.get(*h).map_or("", |x| x.id.as_str()))
            .width(ui.available_width() - 6.0)
            .show_ui(ui, |ui| {
                for (k, x) in hs.iter().enumerate() {
                    ui.selectable_value(h, k, x.id.as_str());
                }
            });
        places.push(("harness:which".into(), r.response.rect));
    });
}

/// A wire list's import into harness `h`: the ops and the diff, against the text it is in.
fn import_preview(
    fg: &FgPanel,
    source: &str,
    hs: &[HarnessOut],
    h: usize,
    path: &str,
) -> Result<(Vec<Op>, Diff), String> {
    let ho = hs.get(h).ok_or("no harness")?;
    let csv = std::fs::read_to_string(fg.beside(path)).map_err(|e| format!("{path}: {e}"))?;
    let text = match &ho.file {
        Some(f) => std::fs::read_to_string(fg.beside(f)).map_err(|e| format!("{f}: {e}"))?,
        None => source.to_string(),
    };
    forge_lang::harness::import(&text, &ho.id, &csv, Some(ho))
}

/// A colour code from its pickers: base, then stripes (`WH/BU`).
fn colour_text(c: &[String; 3]) -> String {
    c.iter()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// A field's words in a dialog or a report.
pub fn label_of(k: &str) -> String {
    match k {
        "awg" => "AWG".into(),
        "mm2" => "Cross-section (mm²)".into(),
        "strand_d" => "Strand (mm)".into(),
        "conductor_d" => "Conductor (mm)".into(),
        "ohm_km" => "Resistance (ohm/km)".into(),
        "wall" => "Wall (mm)".into(),
        "od" => "OD (mm)".into(),
        "temp" => "Temperature (°C)".into(),
        "volts" => "Voltage (V)".into(),
        "min_bend" => "Min bend (mm)".into(),
        "spec_tag" => "Spec tag".into(),
        "mpn" => "MPN".into(),
        "part_number" => "Our part number".into(),
        k => {
            let mut c = k.chars();
            c.next().map_or(String::new(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        }
    }
}

/// `wire_type(..)` from the fields: words quoted, numbers as they are; the OD needed.
pub fn wire_type_call(fields: &[(String, String)]) -> Result<String, String> {
    let mut args = vec![];
    for &(k, _, num) in forge_lang::harness::SPEC {
        let Some(v) = fields
            .iter()
            .find(|x| x.0 == k)
            .map(|x| x.1.trim())
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        // a number, a param's name or an expression as written; words quoted unless they are
        let name = v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') && !v.starts_with(|c: char| c.is_ascii_digit());
        if num {
            if v.parse::<f64>().is_err() && !name && !v.contains(['(', '+', '*', '/', '-']) {
                return Err(format!("{} is not a number", label_of(k)));
            }
            args.push(format!("{k}: {v}"));
        } else if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
            args.push(format!("{k}: {v}"));
        } else {
            args.push(format!("{k}: \"{}\"", v.replace('"', "")));
        }
    }
    let has = |k: &str| fields.iter().any(|x| x.0 == k && !x.1.trim().is_empty());
    if !has("od") && !has("awg") {
        return Err("an OD (or a gauge) is needed".into());
    }
    if !has("source") {
        return Err("a source is needed".into());
    }
    Ok(format!("wire_type({})", args.join(", ")))
}

/// Where a harness problem is: the row it names, and the point (if it has one).
pub fn locate(p: &HarnessProblem) -> Option<(String, Option<[f64; 3]>)> {
    let pt = |q: &forge_math::Point3| Some([q.x, q.y, q.z]);
    match p {
        HarnessProblem::WireClash { harness, wire, at, .. } => Some((format!("h:{harness}:w:{wire}"), pt(at))),
        HarnessProblem::BendTooTight { harness, segment, at, .. } => {
            Some((format!("h:{harness}:s:{segment}"), pt(at)))
        }
        HarnessProblem::KinkAtNode { harness, wire, .. }
        | HarnessProblem::NoPath { harness, wire, .. }
        | HarnessProblem::PathBroken { harness, wire, .. }
        | HarnessProblem::BreakoutTooTight { harness, wire, .. } => Some((format!("h:{harness}:w:{wire}"), None)),
        HarnessProblem::SegmentEmpty { harness, segment } => Some((format!("h:{harness}:s:{segment}"), None)),
        _ => None,
    }
}

/// A harness item's properties, for the browser's properties window: its title and rows.
pub fn item_props(hs: &[HarnessOut], key: &str) -> Option<(String, Vec<(String, String)>)> {
    let rest = key.strip_prefix("h:")?;
    for h in hs {
        let Some(k) = rest.strip_prefix(&format!("{}:", h.id)) else {
            continue;
        };
        let parts: Vec<&str> = k.split(':').collect();
        let type_rows = |ty: &forge_lang::harness::TypeOut, out: &mut Vec<(String, String)>| {
            for (k, v) in &ty.spec {
                let l = if k == "colour" {
                    "Type colour".to_string()
                } else {
                    label_of(k)
                };
                out.push((l, v.clone()));
            }
            if let Some(r) = ty.ohm_km.filter(|_| ty.ohm_worked) {
                out.push((
                    "Resistance (ohm/km)".into(),
                    format!(
                        "{}{}",
                        super::fg::short(r),
                        if ty.ohm_worked {
                            " (from its cross-section)"
                        } else {
                            ""
                        }
                    ),
                ));
            }
        };
        match parts.as_slice() {
            ["w", w] | ["s", _, w] => {
                let wo = h.wires.iter().find(|x| x.name == *w)?;
                let mut out = vec![
                    ("From".into(), wo.from.clone()),
                    ("To".into(), wo.to.clone()),
                    ("Colour".into(), wo.colour.clone()),
                    ("Type".into(), wo.ty.clone()),
                    ("Path".into(), wo.path.join(" › ")),
                ];
                if let Some(o) = &wo.routed {
                    out.push(("Cut length (mm)".into(), super::fg::short(o.length)));
                    out.push((
                        "Length terms".into(),
                        format!(
                            "route {} + loops {} + slack {} + insertion {}",
                            super::fg::short(o.route),
                            super::fg::short(o.loops),
                            super::fg::short(o.slack),
                            super::fg::short(o.insertion)
                        ),
                    ));
                }
                if let Some(r) = wo.ohms {
                    out.push(("Resistance (ohm)".into(), super::fg::short(r)));
                }
                if let (Some(i), Some(v)) = (wo.current, wo.vdrop) {
                    out.push(("Current (A)".into(), super::fg::short(i)));
                    out.push(("Voltage drop (V)".into(), super::fg::short(v)));
                }
                if let Some(ty) = h.types.iter().find(|t| t.name == wo.ty) {
                    type_rows(ty, &mut out);
                }
                return Some((format!("{}.{}", h.id, wo.name), out));
            }
            ["t", t] => {
                let ty = h.types.iter().find(|x| x.name == *t)?;
                let mut out = vec![];
                type_rows(ty, &mut out);
                return Some((format!("{}.{}", h.id, ty.name), out));
            }
            ["s", sg] => {
                let s = h.segments.iter().find(|x| x.name == *sg)?;
                let mut out = vec![
                    ("From node".into(), s.a.clone()),
                    ("To node".into(), s.b.clone()),
                    ("Length (mm)".into(), super::fg::short(s.length)),
                    ("Bundle Ø (mm)".into(), super::fg::short(s.d)),
                    ("Packed Ø (mm)".into(), super::fg::short(s.d_pack)),
                    ("Min bend (mm)".into(), super::fg::short(s.r_min)),
                    ("Wires".into(), s.wires.join(", ")),
                ];
                if let Some(c) = &s.covering {
                    out.push(("Covering".into(), c.clone()));
                }
                return Some((format!("{}.{}", h.id, s.name), out));
            }
            _ => {}
        }
    }
    None
}

/// The status bar's words for a selected wire: its colour, type and spec, length, resistance.
pub fn selection_status(hs: &[HarnessOut], key: &str) -> Option<String> {
    let (title, rows) = item_props(hs, key)?;
    let get = |k: &str| {
        rows.iter()
            .find(|r| r.0 == k)
            .map(|r| r.1.clone())
            .unwrap_or_default()
    };
    let mut words = vec![title];
    for k in [
        "Colour",
        "Type",
        "AWG",
        "Cross-section (mm²)",
        "Conductor",
        "Insulation",
        "Spec",
        "Cut length (mm)",
        "Resistance (ohm)",
        "Voltage drop (V)",
    ] {
        let v = get(k);
        if !v.is_empty() {
            words.push(match k {
                "AWG" => format!("{v} AWG"),
                "Cross-section (mm²)" => format!("{v} mm²"),
                "Cut length (mm)" => format!("{v} mm"),
                "Resistance (ohm)" => format!("{v} ohm"),
                "Voltage drop (V)" => format!("{v} V drop"),
                _ => v,
            });
        }
    }
    Some(words.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loom() -> HarnessOut {
        HarnessOut {
            id: "loom".into(),
            ..HarnessOut::default()
        }
    }

    #[test]
    fn the_dialogs_write_harness_lines_at_its_end() {
        let hs = [loom()];
        let c = Dialog::Connector {
            h: 0,
            tab: 0,
            name: "J4".into(),
            inst: "m3".into(),
            face: "face".into(),
            pins: "pin".into(),
            out: "out".into(),
        };
        assert_eq!(
            dialog_ops(&c, &hs).unwrap(),
            vec![
                Op::AddFeature {
                    before: Some("loom.".into()),
                    text: "J4 = connector(m3.face, pins: m3.pin)".into()
                },
                Op::AddFeature {
                    before: Some("loom.".into()),
                    text: "n_j4 = node(J4, out: out)".into()
                },
            ]
        );
        let w = Dialog::Wire {
            h: 0,
            name: "W9".into(),
            from: "J1".into(),
            from_pin: "8".into(),
            to: "J2".into(),
            to_pin: "a".into(),
            ty: "w22".into(),
            colour: ["WH".into(), "BU".into(), String::new()],
        };
        assert_eq!(
            dialog_ops(&w, &hs).unwrap()[0],
            Op::AddFeature {
                before: Some("loom.".into()),
                text: "W9 = wire(J1[8], J2.a, type: w22, colour: \"WH/BU\")".into()
            }
        );
        let s = Dialog::Segment {
            h: 0,
            from: "n1".into(),
            to: "clip".into(),
            via: "".into(),
            node: "clip".into(),
            at: "plate.clip".into(),
            off: "rise".into(),
            along: "x".into(),
        };
        let ops = dialog_ops(&s, &hs).unwrap();
        assert_eq!(
            ops.iter()
                .map(|o| match o {
                    Op::AddFeature { text, .. } => text.as_str(),
                    _ => "",
                })
                .collect::<Vec<_>>(),
            [
                "clip = node(plate.clip, off: rise, along: \"x\")",
                "s1 = segment(n1, clip)"
            ]
        );
        let none = Dialog::Wire {
            h: 0,
            name: "W1".into(),
            from: "".into(),
            from_pin: "1".into(),
            to: "J2".into(),
            to_pin: "1".into(),
            ty: "w22".into(),
            colour: Default::default(),
        };
        assert_eq!(dialog_ops(&none, &hs), Err("a connector is needed".into()));
    }

    #[test]
    fn create_harness_writes_an_empty_block() {
        let d = Dialog::Create {
            name: "loom".into(),
            slack: "0.03".into(),
            round_up: "5".into(),
        };
        assert_eq!(
            dialog_ops(&d, &[]).unwrap(),
            vec![Op::AddFeature {
                before: None,
                text: "harness loom(slack: ratio(0.03), round_up: 5) {\n}".into()
            }]
        );
    }

    #[test]
    fn an_edited_wire_type_keeps_its_params_and_words() {
        let f = |k: &str, v: &str| (k.to_string(), v.to_string());
        let fields = vec![
            f("awg", "g22"),
            f("od", "od22"),
            f("conductor", "\"Cu\""),
            f("insulation", "PVC"),
            f("source", "\"example\""),
        ];
        assert_eq!(
            wire_type_call(&fields).unwrap(),
            "wire_type(conductor: \"Cu\", awg: g22, insulation: \"PVC\", od: od22, source: \"example\")"
        );
        assert!(wire_type_call(&[f("od", "1.6")]).is_err(), "a source is needed");
    }
}
