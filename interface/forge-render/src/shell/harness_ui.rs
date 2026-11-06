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
}

#[derive(Clone, Debug, Default)]
pub struct HarnessUi {
    pub dialog: Option<Dialog>,
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
            Ok(vec![add(
                h,
                format!(
                    "{} = wire({}, {}, type: {})",
                    need(name, "a name")?,
                    pin(from, from_pin)?,
                    pin(to, to_pin)?,
                    need(ty, "a wire type")?
                ),
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
        };
        let mut problem: Option<String> = None;
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
                    Dialog::Wire {
                        h,
                        name,
                        from,
                        from_pin,
                        to,
                        to_pin,
                        ty,
                    } => {
                        harness_pick(ui, &t, h, &hs, places);
                        let (conns, _, _, types) = h_of(*h).map(names).unwrap_or_default();
                        line_edit(ui, &t, "Name", name, "W1", "wire-name", places);
                        pick(ui, &t, "From connector", from, &conns, "wire-from", places);
                        line_edit(ui, &t, "From pin", from_pin, "1", "wire-from-pin", places);
                        pick(ui, &t, "To connector", to, &conns, "wire-to", places);
                        line_edit(ui, &t, "To pin", to_pin, "1", "wire-to-pin", places);
                        pick(ui, &t, "Wire type", ty, &types, "wire-type", places);
                    }
                    Dialog::Route { h } => {
                        harness_pick(ui, &t, h, &hs, places);
                        if let Some(ho) = h_of(*h) {
                            for w in &ho.wires {
                                let path = if w.routed.is_some() {
                                    w.path.join(" → ")
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
                    fg.out.push(FgAct::AsmOps(ops));
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
        };
        assert_eq!(
            dialog_ops(&w, &hs).unwrap()[0],
            Op::AddFeature {
                before: Some("loom.".into()),
                text: "W9 = wire(J1[8], J2.a, type: w22)".into()
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
}
