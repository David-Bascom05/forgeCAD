//! The Manufacture workspace's widgets: the operations browser and the selected op's results in
//! the side panel, the ribbon's groups, and the dialogs (setup, op, post, tool library, checks,
//! text). Every change is a `Cmd` or an edit of the document (mod.rs); nothing here keeps a
//! value the text does not.

use super::schema::{self, Kind, Tab};
use super::{Ask, CamWs, Cmd, Locate, OpDlg, SetupDlg};
use crate::shell::placed::{Host, Placed};
use crate::shell::theme::{Theme, medium, mono, sans};
use crate::shell::thermo_ui::{button, foot, frame, head, heading, note, row, tabs};
use crate::shell::{Action, Shell, icons};
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};

type Places = Vec<(String, Rect)>;
/// A ribbon button: its name, icon, action (None: disabled) and whether it is on.
pub type Button = (&'static str, &'static str, Option<Action>, bool);

const DLG_W: f32 = 420.0;
const ROW_H: f32 = 26.0;

/// An op's state: its word and colour (its mark is painted: `mark`).
pub fn status_look(t: &Theme, status: &str) -> (&'static str, &'static str, Color32) {
    match status {
        "built" => ("", "certified", t.success),
        "refused" => ("", "refused", t.error),
        "unposted" => ("", "not posted", t.warn),
        "stale" => ("", "needs generating", t.ink2),
        "generating" => ("", "generating", t.accent),
        "suppressed" => ("", "suppressed", t.ink2.gamma_multiply(0.6)),
        _ => ("", "not generated", t.error),
    }
}

/// The Checks tab's lines (CAM-SIM's panel shows them): each op's name and kind with its
/// verdict's words and colour, and the run's warnings.
pub fn check_lines(w: &CamWs, t: &Theme) -> (Vec<(String, String, Color32)>, Vec<String>) {
    let Some(run) = &w.run else {
        return (vec![], vec![]);
    };
    let ops = run
        .ops
        .iter()
        .map(|o| {
            let (_, words, c) = status_look(t, &o.status);
            (format!("{} \u{b7} {}", o.name, o.kind), words.to_string(), c)
        })
        .collect();
    // what is left over is in the tab's issues already, box by box
    (ops, run.warnings.clone())
}

/// A state's mark, painted (the UI's font has no check or cross): a disc with a tick, a cross,
/// a bar; a ring for stale, a ring with a dot for generating, a slashed ring for suppressed.
pub fn mark(p: &egui::Painter, c: egui::Pos2, t: &Theme, status: &str) {
    let (_, _, col) = status_look(t, status);
    let r = 6.0;
    let w = Stroke::new(1.6, Color32::WHITE);
    match status {
        "built" => {
            p.circle_filled(c, r, col);
            p.line_segment([c + vec2(-3.0, 0.2), c + vec2(-0.8, 2.6)], w);
            p.line_segment([c + vec2(-0.8, 2.6), c + vec2(3.2, -2.4)], w);
        }
        "refused" | "error" | "fail" => {
            p.circle_filled(c, r, col);
            p.line_segment([c + vec2(-2.6, -2.6), c + vec2(2.6, 2.6)], w);
            p.line_segment([c + vec2(-2.6, 2.6), c + vec2(2.6, -2.6)], w);
        }
        "unposted" | "undecided" => {
            p.circle_filled(c, r, col);
            p.line_segment([c + vec2(0.0, -3.2), c + vec2(0.0, 0.8)], w);
            p.circle_filled(c + vec2(0.0, 3.0), 0.9, Color32::WHITE);
        }
        "generating" => {
            p.circle_stroke(c, r - 0.5, Stroke::new(1.5, col));
            p.circle_filled(c, 2.2, col);
        }
        "suppressed" => {
            p.circle_stroke(c, r - 0.5, Stroke::new(1.5, col));
            p.line_segment([c + vec2(-3.5, 3.5), c + vec2(3.5, -3.5)], Stroke::new(1.5, col));
        }
        _ => {
            p.circle_stroke(c, r - 0.5, Stroke::new(1.5, col));
        }
    }
}

/// A fold arrow, painted: right (folded) or down.
fn fold_arrow(p: &egui::Painter, c: egui::Pos2, open: bool, col: Color32) {
    let pts = if open {
        vec![c + vec2(-4.0, -2.0), c + vec2(4.0, -2.0), c + vec2(0.0, 3.0)]
    } else {
        vec![c + vec2(-2.0, -4.0), c + vec2(3.0, 0.0), c + vec2(-2.0, 4.0)]
    };
    p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
}

fn icon_of(func: &str) -> &'static str {
    schema::spec(func).map_or("cam_clear", |s| s.icon)
}

/// The ribbon's groups on the Manufacture tab.
pub fn groups(s: &Shell) -> Vec<(&'static str, Vec<Button>, Vec<Button>)> {
    let Some(w) = &s.cam else { return vec![] };
    let a = |c: Ask| Some(Action::Cam(c));
    let has_setup = !w.setups().is_empty();
    let op = |sp: &'static schema::OpSpec| -> Button {
        (
            sp.label,
            sp.icon,
            has_setup.then_some(Action::Cam(Ask::NewOp(sp.func))),
            false,
        )
    };
    let of = |g: &str| -> Vec<Button> {
        schema::OPS
            .iter()
            .chain(schema::TURN_OPS)
            .filter(|o| o.group == g)
            .map(op)
            .collect()
    };
    let generating = w.running.is_some();
    // a document is milling or turning (one dispatch per document): its own setup and ops;
    // an empty one offers both setups
    let turning = w.turning();
    let milling = !turning && has_setup;
    let mut setup = vec![];
    if !turning {
        setup.push(("New Setup", "cam_setup", a(Ask::NewSetup), false));
    }
    if !milling {
        setup.push(("New Lathe Setup", "cam_lathe", a(Ask::NewLatheSetup), false));
    }
    let mut out = vec![("Setup", setup, vec![])];
    if turning {
        let t = of("Turning");
        let (big, small) = t.split_at(3.min(t.len()));
        out.push(("Turning", big.to_vec(), small.to_vec()));
        out.push((
            "View",
            vec![("Section View", "cam_lathe", a(Ask::SectionView), w.turnview.open)],
            vec![],
        ));
    } else {
        let two_d = of("2D");
        let (big2, small2) = two_d.split_at(3.min(two_d.len()));
        out.push(("2D", big2.to_vec(), small2.to_vec()));
        out.push(("Drilling", of("Drilling"), vec![]));
        // 3D surfacing: rough and parallel big, the rest small
        let three = of("3D");
        let (big3, small3) = three.split_at(2.min(three.len()));
        out.push(("3D", big3.to_vec(), small3.to_vec()));
    }
    out.extend(vec![
        (
            "Actions",
            vec![
                (
                    "Generate",
                    "cam_generate",
                    (!generating).then_some(Action::Cam(Ask::Generate)),
                    generating,
                ),
                (
                    "Post Process",
                    "cam_post",
                    a(Ask::PostProcess),
                    w.post_dlg.is_some(),
                ),
                ("NC Programs", "cam_sheet", a(Ask::Programs), w.programs_open),
                (
                    "Quality",
                    "cam_check",
                    // enabled once there is a run: the window says when the measure is still
                    // on its way (or that a turning run is not measured)
                    w.run.is_some().then_some(Action::Cam(Ask::Quality)),
                    w.quality_open,
                ),
            ],
            vec![
                (
                    "Setup Sheet",
                    "cam_sheet",
                    w.run.is_some().then_some(Action::Cam(Ask::SetupSheet)),
                    false,
                ),
                (
                    "Export NC",
                    "cam_post",
                    w.run.is_some().then_some(Action::Cam(Ask::ExportNc)),
                    false,
                ),
                ("Checks", "cam_check", a(Ask::Checks), w.checks_open),
                (
                    "Check Now",
                    "cam_check",
                    (!generating && w.run.is_some()).then_some(Action::Cam(Ask::CheckNow)),
                    false,
                ),
            ],
        ),
        (
            "Manage",
            vec![("Tool Library", "cam_tools", a(Ask::ToolLibrary), w.tools_open)],
            vec![("Document Text", "cam_post", a(Ask::ToggleText), w.text_open)],
        ),
    ]);
    out
}

/// A one-line number: up to 3 decimals, no trailing zeros.
pub fn num(x: f64) -> String {
    if !x.is_finite() {
        return "—".into();
    }
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// What a cycle time's two numbers are (shown on hover wherever they are).
pub const CYCLE_TIP: &str = "\u{2265}: the least time the posted program can take on this machine, from its feeds, rapid rates and tool changes (a guaranteed lower bound).\nest.: an estimate with stated assumptions, not a prediction: a jerk-limited motion planner with 1000 mm/s\u{b2} acceleration (the machine record's where it gives one), 20,000 mm/s\u{b3} jerk and corners blended within 0.02 mm. A real machine can differ by tens of per cent.";

/// A cycle time: the lower bound and the estimate, together.
pub fn cycle_words((lo, est): (f64, f64)) -> String {
    format!("\u{2265} {} \u{b7} est. {}", time(lo), time(est))
}

/// As `cycle_words`, short (m:ss), for the browser's rows.
fn cycle_short((lo, est): (f64, f64)) -> String {
    format!("\u{2265} {} \u{b7} est. {}", short_time(lo), short_time(est))
}

/// A choice's words for a person (the Forge word on hover): `passes: "one_way"` reads "One
/// direction".
fn choice_words(arg: &str, word: &str) -> String {
    match (arg, word) {
        ("passes", "one_way") => "One direction".into(),
        ("passes", "two_way") => "Both directions".into(),
        _ => {
            let w = word.replace('_', " ");
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        }
    }
}

/// A default's value, where its words give one: a leading number ("0 (walls)", "60"), or a
/// share of the tool's diameter ("half the tool's diameter", "0.4 × the tool's diameter").
fn default_value(words: &str, tool_d: Option<f64>) -> Option<f64> {
    let lead: String = words
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if !lead.is_empty() && !words.contains("tool's diameter") {
        return lead.parse().ok();
    }
    let d = tool_d?;
    if words.starts_with("half the tool's diameter") {
        return Some(0.5 * d);
    }
    let (k, rest) = words.split_once(" \u{d7} ")?;
    rest.starts_with("the tool's diameter").then_some(())?;
    Some(k.trim().parse::<f64>().ok()? * d)
}

/// A time as m:ss.
fn short_time(s: f64) -> String {
    // whole seconds, rounded as `time` rounds them
    let s = s.round();
    format!("{}:{:02}", (s / 60.0).floor(), (s % 60.0) as u32)
}

/// A work offset's word: G54 for the first.
fn wcs_word(k: u32) -> String {
    if k < 6 {
        format!("G{}", 54 + k)
    } else {
        format!("G54.1 P{}", k - 5)
    }
}

fn time(s: f64) -> String {
    // whole seconds (a 59.6 s rounds to 1:00, not 0:60)
    let s = s.round();
    if s >= 60.0 {
        format!("{}:{:02} min", (s / 60.0).floor(), (s % 60.0) as u32)
    } else {
        format!("{} s", num(s))
    }
}

impl Shell {
    /// The side panel: the operations browser over the selected statement's results.
    pub(in crate::shell) fn cam_side(&mut self, ui: &mut Ui, area: Rect) {
        let Some(mut w) = self.cam.take() else { return };
        let t = self.theme;
        let mut places: Places = vec![];
        let mut cmds: Vec<Cmd> = vec![];
        let sel = w.selected.clone();
        // the op the simulation is playing (CAM-SIM's cursor), marked in the browser
        let playing = self.camsim.as_ref().and_then(|s| s.cursor()).map(|c| c.op);
        let details_h = if sel.is_some() {
            (area.height() * 0.48).max(220.0)
        } else {
            0.0
        };
        let tree = Rect::from_min_max(area.min, pos2(area.right(), area.bottom() - details_h));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(tree));
        egui::ScrollArea::vertical()
            .id_salt("cam-tree")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                let width = area.width();
                // the document: its name and how it stands
                let (r, _) = ui.allocate_exact_size(vec2(width, ROW_H + 4.0), Sense::hover());
                icons::draw(
                    ui.painter(),
                    pos2(r.left() + 10.0, r.top() + 6.0),
                    16.0,
                    "cam_setup",
                    1.5,
                    t.ink,
                );
                let cert = match &w.run {
                    _ if w.running.is_some() => ("generating…".to_string(), t.accent),
                    Some(run) => match cert_of(run.ok, &run.certification) {
                        Cert::Certified => ("certified".into(), t.success),
                        // not a failure: a rule this run does not decide (G4's leftover not
                        // computed; G5's travel by span only until a setup is placed)
                        Cert::Unchecked(g) => (format!("certified · {}", unchecked_chip(g)), t.ink2),
                        Cert::Refused => ("refused".into(), t.error),
                    },
                    None => ("not generated".into(), t.ink2),
                };
                if let Some(run) = &w.run
                    && let Cert::Unchecked(g) = cert_of(run.ok, &run.certification)
                {
                    ui.interact(r, ui.scope_id().with("cam-cert"), Sense::hover())
                        .on_hover_text(unchecked_words(g));
                }
                ui.painter().text(
                    pos2(r.left() + 32.0, r.center().y),
                    Align2::LEFT_CENTER,
                    "Setups",
                    medium(13.0),
                    t.ink,
                );
                ui.painter().text(
                    pos2(r.right() - 10.0, r.center().y),
                    Align2::RIGHT_CENTER,
                    &cert.0,
                    sans(12.0),
                    cert.1,
                );
                places.push(("cam:certification".into(), r));
                // a setting, so a checkbox (the ribbon holds commands)
                ui.horizontal(|ui| {
                    ui.add_space(30.0);
                    let mut auto = w.auto;
                    let r = ui
                        .checkbox(&mut auto, egui::RichText::new("Generate as I edit").font(sans(12.0)).color(t.ink2))
                        .on_hover_text("Each edit generates again: the ops it changed, and those after them in their setup.");
                    places.push(("cam:auto".into(), r.rect));
                    if auto != w.auto {
                        cmds.push(Cmd::AutoGenerate);
                    }
                });
                if w.setups().is_empty() {
                    note(
                        ui,
                        "No setups yet: Setup > New Setup places the stock and the work offset.",
                        width,
                        t.ink2,
                    );
                }
                let setups: Vec<String> = w.setups().iter().map(|s| s.name.clone()).collect();
                for su in setups {
                    let folded = w.folded.contains(&su);
                    let suppressed = w.stmt(&su).is_some_and(|s| s.suppressed);
                    let ops: Vec<(String, String)> = w
                        .ops_of(&su)
                        .iter()
                        .map(|s| (s.name.clone(), s.func.clone()))
                        .collect();
                    let worst = ops
                        .iter()
                        .map(|o| w.status(&o.0))
                        .fold("built", |a, b| match (a, b) {
                            ("refused" | "error", _) => a,
                            (_, "refused" | "error") => b,
                            ("stale" | "generating", _) => a,
                            (_, "stale" | "generating") => b,
                            ("unposted", _) => a,
                            (_, "unposted") => b,
                            _ => a,
                        });
                    let lathe = w.stmt(&su).is_some_and(|s| s.func == "lathe_setup");
                    let label = match w.run.as_ref().and_then(|r| r.setup(&su)) {
                        Some(_) if lathe => format!("{su}  ·  lathe"),
                        Some(s) => {
                            let tsum: f64 = w.run.as_ref().map_or(0.0, |r| r.ops.iter().filter(|o| o.setup == su).map(|o| o.time_s).sum());
                            if let Some(c) = w.run.as_ref().and_then(|r| r.setup_cycle(&su)) {
                                format!("{su}  ·  {}  ·  {}", wcs_word(s.offset), cycle_short(c))
                            } else if tsum > 0.0 {
                                format!("{su}  ·  {}  ·  {}", wcs_word(s.offset), short_time(tsum))
                            } else {
                                format!("{su}  ·  {}", wcs_word(s.offset))
                            }
                        }
                        None => su.clone(),
                    };
                    let resp = tree_row(
                        ui,
                        &t,
                        width,
                        0,
                        Some(!folded),
                        "cam_setup",
                        &label,
                        if suppressed {
                            "suppressed"
                        } else if ops.is_empty() {
                            ""
                        } else {
                            worst
                        },
                        sel.as_deref() == Some(su.as_str()),
                        &format!("cam:row:{su}"),
                        &mut places,
                    );
                    if resp.0 {
                        if folded {
                            w.folded.retain(|x| x != &su);
                        } else {
                            w.folded.push(su.clone());
                        }
                    }
                    row_actions(&resp.1, &su, &w, &mut cmds, &mut places);
                    if folded {
                        continue;
                    }
                    for (op, func) in &ops {
                        let st = w.status(op).to_string();
                        let tool = w
                            .run
                            .as_ref()
                            .and_then(|r| r.op(op))
                            .and_then(|o| o.tool_t.map(|n| format!("T{n}")))
                            .unwrap_or_default();
                        let label = if tool.is_empty() {
                            op.clone()
                        } else {
                            format!("{op}  ·  {tool}")
                        };
                        let resp = tree_row(
                            ui,
                            &t,
                            width,
                            1,
                            None,
                            icon_of(func),
                            &label,
                            &st,
                            sel.as_deref() == Some(op.as_str()),
                            &format!("cam:row:{op}"),
                            &mut places,
                        );
                        if playing.as_deref() == Some(op.as_str()) {
                            // the simulation's playhead: a small arrow at the row's left
                            let r = resp.1.rect;
                            let c = egui::pos2(r.left() + 6.0, r.center().y);
                            ui.painter().add(egui::Shape::convex_polygon(
                                vec![c + vec2(-3.0, -5.0), c + vec2(4.0, 0.0), c + vec2(-3.0, 5.0)],
                                t.accent,
                                Stroke::NONE,
                            ));
                            places.push((format!("cam:playing:{op}"), r));
                        }
                        row_actions(&resp.1, op, &w, &mut cmds, &mut places);
                    }
                    // its program(s)
                    let progs: Vec<String> = w
                        .stmts
                        .iter()
                        .filter(|s| {
                            s.role == super::doc::Role::Program && s.setup.as_deref() == Some(su.as_str())
                        })
                        .map(|s| s.name.clone())
                        .collect();
                    for p in progs {
                        let st = match w
                            .run
                            .as_ref()
                            .and_then(|r| r.program(&p))
                        {
                            Some(x) if x.error.is_none() && x.text.is_some() => "built",
                            Some(_) => "refused",
                            None => "stale",
                        };
                        let resp = tree_row(
                            ui,
                            &t,
                            width,
                            1,
                            None,
                            "cam_post",
                            &format!("{p}  ·  NC program"),
                            st,
                            sel.as_deref() == Some(p.as_str()),
                            &format!("cam:row:{p}"),
                            &mut places,
                        );
                        row_actions(&resp.1, &p, &w, &mut cmds, &mut places);
                    }
                }
                // the tools the document names
                ui.add_space(6.0);
                let (r, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
                ui.painter().text(
                    pos2(r.left() + 12.0, r.center().y),
                    Align2::LEFT_CENTER,
                    "Tools",
                    medium(13.0),
                    t.ink,
                );
                for s in w.tools() {
                    let label = tool_words(&w, s);
                    let resp = tree_row(
                        ui,
                        &t,
                        width,
                        1,
                        None,
                        "cam_tools",
                        &label,
                        "",
                        false,
                        &format!("cam:tool:{}", s.name),
                        &mut places,
                    );
                    if resp.1.double_clicked() {
                        cmds.push(Cmd::ToolLibrary);
                    }
                }
                if let Some(run) = &w.run {
                    for e in &run.errors {
                        note(ui, e, width, t.error);
                    }
                }
                if let Some(s) = &w.said {
                    note(ui, s, width, t.warn);
                }
            });
        if let Some(id) = sel {
            let r = Rect::from_min_max(pos2(area.left(), area.bottom() - details_h), area.max);
            ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, t.line));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(0.0, 2.0))));
            egui::ScrollArea::vertical()
                .id_salt("cam-details")
                .auto_shrink([false, false])
                .show(&mut child, |ui| {
                    details(ui, &t, &w, &id, r.width(), &mut cmds, &mut places);
                    // the last row clear of the panel's edge when scrolled to the end
                    ui.add_space(16.0);
                });
        }
        for c in cmds {
            w.command(c);
        }
        self.places.extend(places);
        self.cam = Some(w);
    }

    /// The workspace's windows.
    pub(in crate::shell) fn cam_ui(&mut self, ctx: &egui::Context) {
        let Some(mut w) = self.cam.take() else { return };
        w.screen_w = ctx.content_rect().width();
        // docked this frame only if the environment's properties column said so this frame
        let dock = w.dock.take();
        w.dock = dock;
        let t = self.theme;
        // the view less what the simulation's panel and timeline cover: the windows keep inside
        let v = self.camsim_free(self.view_rect);
        let mut places: Places = vec![];
        let mut blocks: Vec<Rect> = vec![];
        if !w.sheet_hidden
            && let Some(mut d) = w.op_dlg.take()
        {
            let (keep, r) = op_dialog(ctx, &t, v, &mut w, &mut d, &mut places);
            blocks.extend(r);
            if keep {
                w.op_dlg = Some(d);
            }
        }
        if !w.sheet_hidden
            && let Some(mut d) = w.setup_dlg.take()
        {
            let (keep, r) = setup_dialog(ctx, &t, v, &mut w, &mut d, &mut places);
            blocks.extend(r);
            if keep {
                w.setup_dlg = Some(d);
            }
        }
        if w.post_dlg.is_some() {
            blocks.extend(post_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.quality_open {
            blocks.extend(quality_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.programs_open {
            blocks.extend(programs_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.tools_open {
            // milling: a page in place of the view and the properties column; turning: a window
            if w.turning() {
                blocks.extend(tools_window(ctx, &t, v, &mut w, &mut places));
            } else {
                blocks.extend(tools_page(ctx, &t, self.view_rect, &mut w, &mut places));
            }
        }
        if w.checks_open {
            blocks.extend(checks_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.text_open {
            blocks.extend(text_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.turning() && w.turnview.open {
            blocks.extend(section_window(ctx, &t, v, &mut w, &mut places));
        }
        // docked only in a frame whose properties column set it (it draws before this)
        w.dock = None;
        w.dock_aux = None;
        w.sheet_hidden = false;
        self.view_blocks.extend(blocks);
        self.places.extend(places);
        self.cam = Some(w);
    }
}

/// One browser row: indent, a fold arrow (Some(open)), icon, label, status glyph. Returns
/// (the arrow clicked, the row's response).
#[allow(clippy::too_many_arguments)]
fn tree_row(
    ui: &mut Ui,
    t: &Theme,
    width: f32,
    depth: usize,
    fold: Option<bool>,
    icon: &str,
    label: &str,
    status: &str,
    on: bool,
    key: &str,
    places: &mut Places,
) -> (bool, egui::Response) {
    let (r, resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r, 0.0, t.on);
    } else if resp.hovered() {
        p.rect_filled(r, 0.0, t.hover);
    }
    let ink = if on { t.on_ink } else { t.ink };
    let x0 = r.left() + 8.0 + depth as f32 * 18.0;
    let mut arrow = false;
    if let Some(open) = fold {
        let a = Rect::from_center_size(pos2(x0 + 5.0, r.center().y), vec2(14.0, 14.0));
        fold_arrow(p, a.center(), open, t.ink2);
        let ar = ui.interact(a, ui.scope_id().with((key, "fold")), Sense::click());
        places.push((format!("{key}:fold"), a));
        arrow = ar.clicked();
    }
    let x = x0 + 14.0;
    let faded = status == "suppressed";
    icons::draw(
        p,
        pos2(x, r.top() + 5.0),
        16.0,
        icon,
        1.5,
        if faded { ink.gamma_multiply(0.5) } else { ink },
    );
    p.text(
        pos2(x + 22.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        if faded { ink.gamma_multiply(0.5) } else { ink },
    );
    if !status.is_empty() {
        mark(p, pos2(r.right() - 16.0, r.center().y), t, status);
        places.push((format!("{key}:status:{status}"), r));
    }
    places.push((key.to_string(), r));
    (arrow, resp)
}

/// A row's clicks: select, open (double click), and its menu.
fn row_actions(resp: &egui::Response, id: &str, w: &CamWs, cmds: &mut Vec<Cmd>, places: &mut Places) {
    if resp.clicked() {
        cmds.push(Cmd::Select(id.to_string()));
    }
    if resp.double_clicked() {
        cmds.push(Cmd::Edit(id.to_string()));
    }
    let is_op = w.stmt(id).is_some_and(|s| s.is_op());
    let suppressed = w.stmt(id).is_some_and(|s| s.suppressed);
    resp.context_menu(|ui| {
        let mut item = |ui: &mut Ui, text: &str, key: &str, c: Cmd| {
            let r = ui.button(text);
            places.push((format!("cam:menu:{key}"), r.rect));
            if r.clicked() {
                cmds.push(c);
                ui.close();
            }
        };
        item(ui, "Edit…", "edit", Cmd::Edit(id.into()));
        item(ui, "Generate", "generate", Cmd::Generate);
        item(ui, "Check Now", "check", Cmd::CheckNow);
        if is_op {
            item(ui, "Move Up", "up", Cmd::Move(id.into(), true));
            item(ui, "Move Down", "down", Cmd::Move(id.into(), false));
            item(ui, "Show Stock After", "stock", Cmd::StockAt(Some(id.into())));
        }
        item(
            ui,
            if suppressed { "Unsuppress" } else { "Suppress" },
            "suppress",
            Cmd::Suppress(id.into()),
        );
        ui.separator();
        item(ui, "Delete", "delete", Cmd::Delete(id.into()));
    });
}

/// The selected statement's results: an op's tool, feeds, path, certificate and checks; a
/// setup's stock and work offset; a program's lines.
fn details(
    ui: &mut Ui,
    t: &Theme,
    w: &CamWs,
    id: &str,
    width: f32,
    cmds: &mut Vec<Cmd>,
    places: &mut Places,
) {
    let Some(s) = w.stmt(id) else { return };
    let run = w.run.as_ref();
    heading(ui, t, &format!("{id} · {}", s.func), width);
    match &s.role {
        super::doc::Role::Setup if s.func == "lathe_setup" => {
            let v = |k: &str| s.arg(k).map(str::to_string).unwrap_or_else(|| "—".into());
            fact(
                ui,
                t,
                "Lathe",
                &s.arg_str("machine").unwrap_or_else(|| "none".into()),
                width,
                t.ink,
            );
            fact(
                ui,
                t,
                "Bar",
                &format!("D {} mm, front at z {}", v("bar"), v("front")),
                width,
                t.ink,
            );
            fact(ui, t, "Chuck", &v("chuck"), width, t.ink);
            match s.arg("at") {
                Some(a) => fact(ui, t, "Z0 at machine Z", a, width, t.ink),
                None => fact(ui, t, "Placed", "no (G5 spans only)", width, t.ink2),
            }
            fact(
                ui,
                t,
                "Tolerance",
                s.arg("tol").unwrap_or("0.002 (default)"),
                width,
                t.ink,
            );
            for l in run.map(|r| r.leftover.clone()).unwrap_or_default() {
                note(
                    ui,
                    &format!(
                        "Left over: r {}…{}, z {}…{} ({})",
                        num(l.boxes[0][0]),
                        num(l.boxes[0][2]),
                        num(l.z[0]),
                        num(l.z[1]),
                        l.why
                    ),
                    width,
                    t.ink2,
                );
            }
        }
        super::doc::Role::Setup => {
            let Some(su) = run.and_then(|r| r.setup(id)) else {
                note(ui, "Generate to see the stock and work offset.", width, t.ink2);
                return;
            };
            fact(ui, t, "Machine", &machine_words(&su.machine), width, t.ink);
            fact(ui, t, "Work offset", &wcs_word(su.offset), width, t.ink);
            if let Some(r) = run {
                let ops: Vec<&super::run::OpRes> = r.ops.iter().filter(|o| o.setup == id).collect();
                let total: f64 = ops.iter().map(|o| o.time_s).sum();
                if let Some(c) = r.setup_cycle(id) {
                    heading(ui, t, "Cycle time", width);
                    fact_tip(ui, t, "On the machine", &cycle_words(c), CYCLE_TIP, width, t.ink);
                    for o in &ops {
                        if let Some(oc) = r.cycle([o.name.as_str()]) {
                            fact_tip(
                                ui,
                                t,
                                &format!("  {} · {}", o.name, o.kind),
                                &cycle_words(oc),
                                CYCLE_TIP,
                                width,
                                t.ink2,
                            );
                        }
                    }
                } else {
                    heading(ui, t, &format!("Feed time {}", time(total)), width);
                    for o in &ops {
                        fact(
                            ui,
                            t,
                            &format!("  {} · {}", o.name, o.kind),
                            &time(o.time_s),
                            width,
                            t.ink2,
                        );
                    }
                    note(
                        ui,
                        "Feeds and rapids at their programmed rates, tool changes not counted: not a cycle time (that comes from the machine's timeline once the program is timed).",
                        width,
                        t.ink2,
                    );
                }
            }
            fact(
                ui,
                t,
                "Origin (part frame)",
                &format!(
                    "{}, {}, {}",
                    num(su.origin[0]),
                    num(su.origin[1]),
                    num(su.origin[2])
                ),
                width,
                t.ink,
            );
            let sz = [0, 1, 2].map(|k| su.stock_max[k] - su.stock_min[k]);
            fact(
                ui,
                t,
                "Stock",
                &format!("{} × {} × {} mm", num(sz[0]), num(sz[1]), num(sz[2])),
                width,
                t.ink,
            );
            fact(ui, t, "Tolerance", &format!("{} mm", num(su.tol)), width, t.ink);
            fact(
                ui,
                t,
                "Clearance",
                &format!("{} mm", num(su.clearance)),
                width,
                t.ink,
            );
            fact(
                ui,
                t,
                "Clear · retract",
                &format!("{} · {} mm", num(su.clear_h), num(su.retract_h)),
                width,
                t.ink,
            );
            fact(
                ui,
                t,
                "Fixture",
                su.fixture.as_deref().unwrap_or("none"),
                width,
                t.ink,
            );
            match su.at {
                Some(a) => fact(
                    ui,
                    t,
                    "Placed at",
                    &format!("{}, {}, {}", num(a[0]), num(a[1]), num(a[2])),
                    width,
                    t.ink,
                ),
                None => fact(ui, t, "Placed at", "not placed (G5 spans only)", width, t.warn),
            }
            for l in run
                .map(|r| r.leftover.iter().filter(|l| l.setup == id).collect::<Vec<_>>())
                .unwrap_or_default()
            {
                note(
                    ui,
                    &format!(
                        "Left over z {}…{}: {} region(s), {}",
                        num(l.z[0]),
                        num(l.z[1]),
                        l.boxes.len(),
                        l.why
                    ),
                    width,
                    t.warn,
                );
            }
        }
        super::doc::Role::Program => {
            let Some(p) = run.and_then(|r| r.program(id)) else {
                note(ui, "Generate to post it.", width, t.ink2);
                return;
            };
            fact(ui, t, "Post", &p.post, width, t.ink);
            fact(ui, t, "Lines", &p.lines.to_string(), width, t.ink);
            fact(
                ui,
                t,
                "Round trip",
                if p.round_trip { "exact" } else { "no" },
                width,
                if p.round_trip { t.success } else { t.error },
            );
            // the side panel is narrow: the word, its reason as a note below
            let (word, why) = p.certified.split_once(" (").unwrap_or((p.certified.as_str(), ""));
            let ink = if p.certified == "full" { t.success } else { t.warn };
            fact(ui, t, "Certified", word, width, ink);
            if !why.is_empty() {
                note(ui, why.trim_end_matches(')'), width, t.ink2);
            }
            if let Some(e) = &p.error {
                note(ui, e, width, t.error);
            }
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if button(ui, t, "Post Process…", true, "cam:details:post".into(), places) {
                    cmds.push(Cmd::PostProcess);
                }
            });
        }
        super::doc::Role::Op(_) => {
            let st = w.status(id);
            let (_, word, c) = status_look(t, st);
            fact(ui, t, "Status", word, width, c);
            if let Some(o) = run.and_then(|r| r.op(id)) {
                fact(ui, t, "Assurance", o.assurance.words(), width, t.ink2);
            }
            if st == "stale" || st == "error" {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if button(ui, t, "Generate", true, "cam:details:generate".into(), places) {
                        cmds.push(Cmd::Generate);
                    }
                });
            }
            let Some(o) = run.and_then(|r| r.op(id)) else {
                return;
            };
            if let Some(tool) = &o.tool {
                fact(
                    ui,
                    t,
                    "Tool",
                    &match o.tool_d {
                        Some(d) => format!("T{} {tool}, D {}", o.tool_t.unwrap_or(0), num(d)),
                        None => format!("T{} {tool}", o.tool_t.unwrap_or(0)),
                    },
                    width,
                    t.ink,
                );
            }
            for p in &o.plan {
                fact(ui, t, "Plan", p, width, t.ink);
            }
            if let (Some(rpm), Some(f)) = (o.rpm, o.feed) {
                fact(
                    ui,
                    t,
                    "Spindle",
                    &format!("{} rpm", num(rpm.round())),
                    width,
                    t.ink,
                );
                fact(ui, t, "Feed", &format!("{} mm/min", num(f.round())), width, t.ink);
            }
            if let (Some(vc), Some(fz), Some(hex)) = (o.vc, o.fz, o.hex) {
                fact(ui, t, "vc", &format!("{} m/min", num(vc.round())), width, t.ink);
                fact(
                    ui,
                    t,
                    "fz · chip (thinned)",
                    &format!("{} · {} mm", num(fz), num(hex)),
                    width,
                    t.ink,
                );
            }
            if let Some(p) = o.power_kw {
                fact(ui, t, "Power", &format!("{} kW", num(p)), width, t.ink);
            }
            fact(ui, t, "Moves", &o.moves.to_string(), width, t.ink);
            // lengths and time where the run gives them (turning's reply has none yet)
            if o.length_cut > 0.0 || o.time_s > 0.0 {
                fact(
                    ui,
                    t,
                    "Cutting · rapid",
                    &format!(
                        "{} · {} mm",
                        num(o.length_cut.round()),
                        num(o.length_rapid.round())
                    ),
                    width,
                    t.ink,
                );
                let own = run.and_then(|r| r.cycle([o.name.as_str()]));
                match own {
                    Some(c) => fact_tip(ui, t, "Cycle time", &cycle_words(c), CYCLE_TIP, width, t.ink),
                    None => fact(ui, t, "Feed time", &time(o.time_s), width, t.ink),
                }
                // adaptive: what clear (at its own defaults) would take on the same setup, tool
                // and heights, so the faster can be chosen (TOOLPATH-QUALITY)
                if o.kind == "adaptive" {
                    let tip = "Clear on the same setup, tool, heights and stock to leave, at clear's own defaults, measured on the machine's timeline as this op is. A tuned clear (fewer, deeper levels) may be faster still.";
                    let ready = w
                        .alt
                        .as_ref()
                        .filter(|a| a.0 == o.name && run.is_some_and(|r| r.text == a.1));
                    match (ready, own) {
                        (Some((_, _, Some(c))), Some(me)) => {
                            let pct = (c.1 / me.1 - 1.0) * 100.0;
                            let words = format!(
                                "{} ({}{:.0}%)",
                                cycle_short(*c),
                                if pct >= 0.0 { "+" } else { "\u{2212}" },
                                pct.abs()
                            );
                            fact_tip(
                                ui,
                                t,
                                "Clear (its defaults)",
                                &words,
                                tip,
                                width,
                                if c.1 < me.1 { t.warn } else { t.ink2 },
                            );
                        }
                        (Some((_, _, Some(c))), None) => fact_tip(
                            ui,
                            t,
                            "Clear (its defaults)",
                            &cycle_short(*c),
                            tip,
                            width,
                            t.ink2,
                        ),
                        (Some((_, _, None)), _) => fact_tip(
                            ui,
                            t,
                            "Clear (its defaults)",
                            "does not build here",
                            tip,
                            width,
                            t.ink2,
                        ),
                        (None, _) => fact_tip(
                            ui,
                            t,
                            "Clear (its defaults)",
                            "measuring\u{2026}",
                            tip,
                            width,
                            t.ink2,
                        ),
                    }
                }
            }
            if let Some(l) = o.least {
                let tol = run.and_then(|r| r.setup(&o.setup)).map_or(0.0, |s| s.tol);
                // a finishing cut touches the wall it finishes by design: a gouge is below −tol;
                // one line, the reason on hover
                let (words, tip) = if l.abs() <= tol + 1e-9 {
                    (
                        format!("{} mm · on its wall", num(l)),
                        format!(
                            "The cutter touches the wall it finishes, as a finishing pass is meant to; a gouge would be under −{} mm (the setup's tolerance).",
                            num(tol)
                        ),
                    )
                } else {
                    (
                        format!("{} mm", num(l)),
                        "The least distance the certificate found between the cutter and the part over every posted move.".to_string(),
                    )
                };
                fact_tip(ui, t, "Least cutter distance", &words, &tip, width, t.ink);
            }
            if o.pairs > 0 {
                fact(ui, t, "Certified pairs", &o.pairs.to_string(), width, t.ink);
            }
            if let Some(q) = run.and_then(|r| r.quality(id)) {
                quality_facts(ui, t, q, width);
            }
            for r in &o.removals {
                note(ui, &format!("Declared removal: {r}"), width, t.ink2);
            }
            if let Some(r) = &o.refusal {
                note(
                    ui,
                    r,
                    width,
                    if o.status == "unposted" { t.warn } else { t.error },
                );
            }
            if let Some(f) = &o.fix {
                note(ui, &format!("Fix: {f}"), width, t.accent);
            }
            if !o.checks.is_empty() {
                heading(ui, t, "Checks", width);
            }
            for (k, c) in o.checks.iter().enumerate() {
                if check_row(ui, t, c, width, &format!("cam:check:{k}"), places) {
                    cmds.push(Cmd::Locate(Locate {
                        op: id.to_string(),
                        mov: c.mov,
                        at: c.at,
                        rule: c.rule.clone(),
                    }));
                }
            }
        }
        _ => {}
    }
}

/// A percentage of a share in [0, 1].
fn pct(x: f64) -> String {
    if x.is_finite() {
        format!("{:.0}%", 100.0 * x)
    } else {
        "—".into()
    }
}

/// An op's toolpath quality (forge-cam-quality): what its path is like to run.
fn quality_facts(ui: &mut Ui, t: &Theme, q: &forge_cam_quality::OpQ, width: f32) {
    heading(ui, t, "Toolpath quality", width);
    let cut_air = q.t_cut + q.t_air;
    if q.t_min > 0.0 {
        fact_tip(
            ui,
            t,
            "Cycle time",
            &cycle_words((q.t_min, q.t_scurve.max(q.t_min))),
            CYCLE_TIP,
            width,
            t.ink,
        );
    }
    if cut_air > 0.0 {
        let air = q.t_air / cut_air;
        fact(
            ui,
            t,
            "Feed time · in air",
            &format!("{} · {}", time(cut_air), pct(air)),
            width,
            if air > 0.3 { t.warn } else { t.ink },
        );
    }
    fact(
        ui,
        t,
        "Retracts · plunges · ramps",
        &format!("{} · {} · {}", q.retracts, q.plunges, q.ramps),
        width,
        if q.plunges > 0 { t.warn } else { t.ink },
    );
    if q.side {
        fact(
            ui,
            t,
            "Engagement max · mean",
            &format!("{}° · {}°", num(q.phi_max.round()), num(q.phi_mean.round())),
            width,
            if q.phi_max > 170.0 { t.warn } else { t.ink },
        );
        fact(
            ui,
            t,
            "Climb · conv. · slotting",
            &format!("{} · {} · {}", pct(q.climb), pct(q.conventional), pct(q.slotting)),
            width,
            if q.slotting > 0.05 { t.warn } else { t.ink },
        );
        fact(
            ui,
            t,
            "Chip (thinned) mean · max",
            &format!("{} · {} mm", num(q.hex_mean), num(q.hex_max)),
            width,
            t.ink,
        );
        fact(
            ui,
            t,
            "Feed per tooth target",
            &format!("{} mm", num(q.fz_target)),
            width,
            t.ink2,
        );
        fact(
            ui,
            t,
            "Rubbing · overloaded",
            &format!("{} · {}", pct(q.thin), pct(q.thick)),
            width,
            if q.thin > 0.1 || q.thick > 0.1 {
                t.warn
            } else {
                t.ink
            },
        );
    }
    if q.mrr > 0.0 {
        fact(
            ui,
            t,
            "Material removal",
            &format!("{} cm3/min", num(q.mrr / 1000.0)),
            width,
            t.ink,
        );
    }
    if q.p_max > 0.0 {
        let over = q.p_avail > 0.0 && q.p_max > q.p_avail;
        let avail = if q.p_avail > 0.0 {
            format!(" of {} kW", num(q.p_avail))
        } else {
            String::new()
        };
        fact(
            ui,
            t,
            "Spindle power peak",
            &format!("{} kW{avail}", num(q.p_max)),
            width,
            if over { t.error } else { t.ink },
        );
    }
    if q.rapid_cuts > 0 {
        note(
            ui,
            &format!(
                "{} rapid move(s) meet material in the measure model",
                q.rapid_cuts
            ),
            width,
            t.warn,
        );
    }
}

/// A failed or undecided check: click to locate its witness. True when clicked.
fn check_row(
    ui: &mut Ui,
    t: &Theme,
    c: &super::run::Check,
    width: f32,
    key: &str,
    places: &mut Places,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 0.0, t.hover);
    }
    let ink = if c.verdict == "fail" { t.error } else { t.warn };
    mark(
        ui.painter(),
        pos2(r.left() + 20.0, r.top() + 10.0),
        t,
        if c.verdict == "fail" { "fail" } else { "undecided" },
    );
    ui.painter().text(
        pos2(r.left() + 32.0, r.top() + 10.0),
        Align2::LEFT_CENTER,
        format!("{} · {} vs {} mm", c.rule, num(c.value), num(c.limit)),
        medium(12.0),
        ink,
    );
    ui.painter().text(
        pos2(r.left() + 14.0, r.top() + 27.0),
        Align2::LEFT_CENTER,
        format!(
            "{} vs {} · {} move {} at ({}, {}, {})",
            c.tool_part,
            c.other,
            c.phase,
            c.mov,
            num(c.at[0]),
            num(c.at[1]),
            num(c.at[2])
        ),
        sans(11.0),
        t.ink2,
    );
    ui.painter().text(
        pos2(r.right() - 12.0, r.center().y),
        Align2::RIGHT_CENTER,
        "Locate",
        sans(11.0),
        t.accent,
    );
    places.push((key.to_string(), r));
    resp.clicked()
}

/// A text field for argument `key`: its words, the text as written, a unit and the default.
#[allow(clippy::too_many_arguments)]
fn arg_field(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    v: &mut String,
    hint: &str,
    unit: &str,
    key: &str,
    places: &mut Places,
) {
    arg_field_tip(ui, t, label, v, hint, "", unit, key, places);
}

/// As `arg_field`, with `tip` on hover (a default's rule).
#[allow(clippy::too_many_arguments)]
fn arg_field_tip(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    v: &mut String,
    hint: &str,
    tip: &str,
    unit: &str,
    key: &str,
    places: &mut Places,
) {
    row(ui, t, label, DLG_W, 30.0, |ui| {
        let r = ui.add(
            egui::TextEdit::singleline(v)
                .font(sans(13.0))
                .hint_text(hint)
                .desired_width(ui.available_width() - 40.0)
                .margin(vec2(7.0, 5.0)),
        );
        if !tip.is_empty() {
            r.clone().on_hover_text(tip);
        }
        places.push((key.to_string(), r.rect));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(unit).font(sans(12.0)).color(t.ink2));
    });
}

/// A choice among `items` (value, words); `now` the value as written.
fn arg_choice(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    v: &mut String,
    items: &[(String, String)],
    key: &str,
    places: &mut Places,
) {
    row(ui, t, label, DLG_W, 30.0, |ui| {
        let shown = items
            .iter()
            .find(|x| x.0 == *v)
            .map_or_else(|| v.clone(), |x| x.1.clone());
        let r = egui::ComboBox::from_id_salt(("cam-choice", key))
            .selected_text(if shown.is_empty() {
                "(default)".to_string()
            } else {
                shown
            })
            .width(ui.available_width() - 6.0)
            .show_ui(ui, |ui| {
                for (k, (val, words)) in items.iter().enumerate() {
                    let r = ui.selectable_label(*v == *val, words.as_str());
                    places.push((format!("{key}:{k}"), r.rect));
                    if r.clicked() {
                        *v = val.clone();
                    }
                }
            });
        // the Forge word it writes, on hover (an op's keyword choices)
        if let Some(arg) = key.strip_prefix("cam:op:arg:") {
            let words = if v.is_empty() {
                "not written (the default)".to_string()
            } else {
                format!("{arg}: {v}")
            };
            r.response.clone().on_hover_text(format!("Forge text: {words}"));
        }
        places.push((key.to_string(), r.response.rect));
    });
}

fn op_dialog(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    d: &mut OpDlg,
    places: &mut Places,
) -> (bool, Option<Rect>) {
    let Some(spec) = schema::spec(&d.func) else {
        w.text_open = true;
        return (false, None);
    };
    let tabs_of = schema::tabs(spec);
    let (mut ok, mut cancel, mut apply, mut close) = (false, false, false, false);
    let tools: Vec<(String, String)> = w
        .tools()
        .iter()
        .map(|s| (s.name.clone(), tool_words(w, s)))
        .collect();
    let part = w.parts.first().cloned().unwrap_or_default();
    let holes: Vec<(String, String)> = part
        .holes
        .iter()
        .map(|h| {
            let depth = h
                .depth
                .map_or("through".to_string(), |x| format!("{} deep", num(x)));
            let thr = h
                .thread
                .map(|(m, p)| format!(" · M{}×{}", num(m), num(p)))
                .unwrap_or_default();
            (
                format!("{}.{}", part.name, h.name),
                format!(
                    "{} · D {} {depth}{thr} · {} place(s)",
                    h.name,
                    num(h.d),
                    h.places.len()
                ),
            )
        })
        .collect();
    let sketches: Vec<(String, String)> = part
        .sketches
        .iter()
        .map(|s| (format!("{}.{s}", part.name), s.clone()))
        .collect();
    let new = d.id.is_none();
    let docked = w.dock.is_some();
    let r = dialog_host(w.dock, "cam-op-dialog", t, egui::Window::new("cam-op")
        .id(egui::Id::unique("cam-op-dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(DLG_W, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v)))
        .show_placed(ctx, places, |ui, places| {
            ui.spacing_mut().item_spacing.y = 4.0;
            close = head(
                ui,
                t,
                spec.icon,
                &format!("{} · {}", spec.label, if new { "new".to_string() } else { d.name.clone() }),
                DLG_W,
                places,
                "cam:op",
            );
            let names: Vec<&str> = tabs_of.iter().map(|x| x.label()).collect();
            tabs(ui, t, &names, &mut d.tab, DLG_W, places, "cam:op:tab");
            let tab = tabs_of.get(d.tab).copied().unwrap_or(Tab::Tool);
            ui.add_space(4.0);
            if tab == Tab::Tool || (tab == tabs_of[0] && tab != Tab::Tool) {
                // the op's name and setup sit with its first tab
                let mut name = d.name.clone();
                arg_field(ui, t, "Name", &mut name, "", "", "cam:op:name", places);
                d.name = name;
                fact(ui, t, "Setup", &d.setup, DLG_W, t.ink);
            }
            // the chosen tool's diameter: the defaults that follow from it, as numbers
            let tool_d = w
                .lib
                .tools
                .iter()
                .find(|x| w.stmt(d.get("tool")).and_then(|s| s.pos(0)).map(super::doc::unquote).as_deref() == Some(x.id.as_str()))
                .map(|x| x.dc);
            for f in spec.fields.iter().filter(|f| f.tab == tab) {
                let key = format!("cam:op:arg:{}", f.arg);
                let mut val = d.get(f.arg).to_string();
                // a default: its value where it is a number (or the tool's diameter gives one),
                // "default" beside it, the rule on hover
                let (hint, tip) = if f.required {
                    ("required".to_string(), String::new())
                } else {
                    match default_value(f.default, tool_d) {
                        Some(v) => (format!("{}  \u{b7} default", num(v)), format!("Default: {}", f.default)),
                        None => (format!("default: {}", f.default), String::new()),
                    }
                };
                match f.kind {
                    Kind::Len => arg_field_tip(ui, t, f.label, &mut val, &hint, &tip, "mm", &key, places),
                    Kind::Deg => arg_field_tip(ui, t, f.label, &mut val, &hint, &tip, "\u{b0}", &key, places),
                    Kind::Z => arg_field(ui, t, f.label, &mut val, &hint, "mm z", &key, places),
                    Kind::Count => arg_field(ui, t, f.label, &mut val, &hint, "", &key, places),
                    Kind::Text => arg_field(ui, t, f.label, &mut val, &hint, "", &key, places),
                    Kind::Choice(words) => {
                        let mut items: Vec<(String, String)> = vec![];
                        if !f.required {
                            items.push((String::new(), format!("{} (default)", choice_words(f.arg, f.default))));
                        }
                        items.extend(words.iter().map(|w| (format!("\"{w}\""), choice_words(f.arg, w))));
                        arg_choice(ui, t, f.label, &mut val, &items, &key, places);
                    }
                    Kind::Tool => {
                        arg_choice(ui, t, f.label, &mut val, &tools, &key, places);
                        if let Some(tool) = w.lib.tools.iter().find(|x| {
                            w.stmt(&val).and_then(|s| s.pos(0)).map(super::doc::unquote).as_deref() == Some(x.id.as_str())
                        }) {
                            fact(ui, t, "Type", kind_words(tool.kind), DLG_W, t.ink2);
                            fact(ui, t, "Diameter", &format!("{} mm", num(tool.dc)), DLG_W, t.ink2);
                            fact(ui, t, "Flutes", &tool.nof.to_string(), DLG_W, t.ink2);
                            fact(ui, t, "Flute length", &format!("{} mm", num(tool.lcf)), DLG_W, t.ink2);
                        }
                        ui.horizontal(|ui| {
                            ui.add_space(146.0);
                            if button(ui, t, "Tool Library…", false, "cam:op:library".into(), places) {
                                w.tools_open = true;
                            }
                        });
                        if let Some(o) = d.id.as_ref().and_then(|id| w.run.as_ref().and_then(|r| r.op(id))) {
                            heading(ui, t, "Feeds and speeds (from cutting data)", DLG_W);
                            if let (Some(rpm), Some(fd)) = (o.rpm, o.feed) {
                                fact(ui, t, "Spindle", &format!("{} rpm", num(rpm.round())), DLG_W, t.ink);
                                fact(ui, t, "Feed", &format!("{} mm/min", num(fd.round())), DLG_W, t.ink);
                            }
                            if let (Some(vc), Some(fz)) = (o.vc, o.fz) {
                                fact(ui, t, "Cutting speed · per tooth", &format!("{} m/min · {} mm", num(vc.round()), num(fz)), DLG_W, t.ink);
                            }
                            if let Some(h) = o.hex {
                                fact(ui, t, "Chip thickness (thinned)", &format!("{} mm", num(h)), DLG_W, t.ink);
                            }
                        }
                    }
                    Kind::Hole => {
                        let mut items = vec![];
                        if !f.required {
                            items.push((String::new(), format!("none ({})", f.default)));
                        }
                        items.extend(holes.iter().cloned());
                        arg_choice(ui, t, f.label, &mut val, &items, &key, places);
                        if holes.is_empty() {
                            note(ui, "The part has no hole() features.", DLG_W, t.ink2);
                        } else {
                            note(ui, "Or pick a face of the hole in the view: the hole's own feature is taken, with its diameter, depth and thread.", DLG_W, t.ink2);
                        }
                    }
                    Kind::Sketch => {
                        arg_choice(ui, t, f.label, &mut val, &sketches, &key, places);
                        note(ui, "Or pick the sketch's face in the view.", DLG_W, t.ink2);
                    }
                }
                d.set(f.arg, val);
            }
            if tab == Tab::Linking {
                if let Some(su) = w.run.as_ref().and_then(|r| r.setup(&d.setup)) {
                    heading(ui, t, "Retract and rapids (the setup's)", DLG_W);
                    fact(ui, t, "Clearance height", &format!("{} mm", num(su.clear_h)), DLG_W, t.ink);
                    fact(ui, t, "Retract height", &format!("{} mm", num(su.retract_h)), DLG_W, t.ink);
                    fact(ui, t, "Clearance kept by rapids", &format!("{} mm", num(su.clearance)), DLG_W, t.ink);
                }
                note(
                    ui,
                    "Passes link by retract to the retract height, between levels and between regions; every rapid is certified clear of the part and the in-process stock. Lead-in arcs, stay-down links and high-feed linking are not in the language yet.",
                    DLG_W,
                    t.ink2,
                );
                ui.horizontal(|ui| {
                    ui.add_space(146.0);
                    if button(ui, t, "Setup Heights…", false, "cam:op:setup-heights".into(), places) {
                        let id = d.setup.clone();
                        w.command(Cmd::Edit(id));
                        if let Some(sd) = &mut w.setup_dlg {
                            sd.tab = 2;
                        }
                    }
                });
            }
            if tab == Tab::Heights
                && let Some(su) = w.run.as_ref().and_then(|r| r.setup(&d.setup))
            {
                fact(ui, t, "Clearance height (setup)", &format!("{} mm", num(su.clear_h)), DLG_W, t.ink2);
                fact(ui, t, "Retract height (setup)", &format!("{} mm", num(su.retract_h)), DLG_W, t.ink2);
                fact(ui, t, "Stock top", &format!("{} mm", num(su.stock_max[2])), DLG_W, t.ink2);
            }
            ui.add_space(2.0);
            note(ui, spec.what, DLG_W, t.ink2);
            forge_text(ui, t, &CamWs::op_text(d), DLG_W);
            if let Some(e) = &d.error {
                note(ui, e, DLG_W, t.error);
            }
            if !docked {
                (ok, cancel, apply) = foot(ui, t, "cam:op", true, places);
            }
        });
    let rect = r.map(|r| r.response.rect);
    // docked: the commit buttons pinned under the sheet's body, always in sight
    if let Some(dock) = w.dock {
        (ok, cancel, apply) = docked_foot(ctx, t, dock, "cam:op", true, places);
    }
    // Enter is OK (Esc cancels: the shell's one level back, CamWs::escape; Tab moves on)
    ok |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if ok || apply {
        let missing: Vec<&str> = spec
            .fields
            .iter()
            .filter(|f| f.required && d.get(f.arg).trim().is_empty())
            .map(|f| f.label)
            .collect();
        if !missing.is_empty() {
            d.error = Some(format!("needed: {}", missing.join(", ")));
            return (true, rect);
        }
        match w.op_ok(d) {
            Ok(()) => {
                d.error = None;
                if apply {
                    // the dialog now edits what it made
                    d.id = Some(d.name.clone());
                    d.was = d.args.clone();
                }
            }
            Err(e) => {
                d.error = Some(e);
                return (true, rect);
            }
        }
    }
    (!(ok || cancel || close), rect)
}

fn setup_dialog(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    d: &mut SetupDlg,
    places: &mut Places,
) -> (bool, Option<Rect>) {
    if d.lathe {
        return lathe_dialog(ctx, t, v, w, d, places);
    }
    let (mut ok, mut cancel, mut close) = (false, false, false);
    let get = |d: &SetupDlg, k: &str| {
        d.args
            .iter()
            .find(|a| a.0 == k)
            .map(|a| a.1.clone())
            .unwrap_or_default()
    };
    let set = |d: &mut SetupDlg, k: &str, v: String| match d.args.iter_mut().find(|a| a.0 == k) {
        Some(a) => a.1 = v,
        None => d.args.push((k.to_string(), v)),
    };
    // the document's machines and fixtures by name, then the library's
    let mut machines: Vec<(String, String)> = w
        .stmts
        .iter()
        .filter(|s| s.role == super::doc::Role::Machine)
        .map(|s| {
            (
                s.name.clone(),
                machine_words(&s.pos(0).map(super::doc::unquote).unwrap_or_default()),
            )
        })
        .collect();
    machines.extend(w.lib.machines.iter().map(|m| {
        (
            format!("machine(\"{}\")", m.name),
            format!("{} · {} control", machine_words(&m.name), m.control),
        )
    }));
    let mut fixtures: Vec<(String, String)> = vec![(String::new(), "none".into())];
    fixtures.extend(
        w.stmts
            .iter()
            .filter(|s| s.role == super::doc::Role::Fixture)
            .map(|s| (s.name.clone(), format!("{} · {}", s.name, s.func))),
    );
    fixtures.extend(w.lib.vises.iter().flat_map(|vi| {
        ["x", "y"].map(|ax| {
            (
                format!("vise(\"{}\", across: \"{ax}\")", vi.name),
                format!("{} across {ax} (opens {} mm)", vi.name, num(vi.max_opening)),
            )
        })
    }));
    let mut froms: Vec<(String, String)> = vec![(String::new(), "none (new stock)".into())];
    froms.extend(
        w.setups()
            .iter()
            .filter(|s| Some(s.name.as_str()) != d.id.as_deref())
            .map(|s| (s.name.clone(), format!("{} (turned over)", s.name))),
    );
    let new = d.id.is_none();
    let docked = w.dock.is_some();
    let r = dialog_host(w.dock, "cam-setup-dialog", t, egui::Window::new("cam-setup")
        .id(egui::Id::unique("cam-setup-dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(DLG_W, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v)))
        .show_placed(ctx, places, |ui, places| {
            ui.spacing_mut().item_spacing.y = 4.0;
            close = head(
                ui,
                t,
                "cam_setup",
                &if new { "New Setup".to_string() } else { format!("Setup · {}", d.name) },
                DLG_W,
                places,
                "cam:setup",
            );
            tabs(ui, t, &["Setup", "Stock", "Heights", "Post"], &mut d.tab, DLG_W, places, "cam:setup:tab");
            ui.add_space(4.0);
            match d.tab {
                0 => {
                    let mut name = d.name.clone();
                    arg_field(ui, t, "Name", &mut name, "", "", "cam:setup:name", places);
                    d.name = name;
                    let mut m = get(d, "machine");
                    arg_choice(ui, t, "Machine", &mut m, &machines, "cam:setup:machine", places);
                    set(d, "machine", m);
                    heading(ui, t, "Work coordinate system", DLG_W);
                    let mut o = get(d, "origin");
                    let picks = [
                        ("\"top-center\"", "Stock top, centre"),
                        ("\"top-corner\"", "Stock top, corner (x−, y−)"),
                    ];
                    let mut items: Vec<(String, String)> = picks.iter().map(|(a, b)| ((*a).into(), (*b).into())).collect();
                    if !o.is_empty() && !items.iter().any(|x| x.0 == o) {
                        items.push((o.clone(), format!("Point {o} (part frame)")));
                    }
                    items.push(("(0, 0, 0)".into(), "Part origin".into()));
                    arg_choice(ui, t, "Origin", &mut o, &items, "cam:setup:origin", places);
                    // a point of its own: its numbers (the named origins need none)
                    if o.trim_start().starts_with('(') {
                        arg_field(ui, t, "Origin point (x, y, z)", &mut o, "(0, 0, 0)", "mm", "cam:setup:origin:text", places);
                    }
                    set(d, "origin", o);
                    let mut wcs = get(d, "wcs");
                    let offsets: Vec<(String, String)> = std::iter::once((String::new(), "G54 (default)".to_string()))
                        .chain((1..6).map(|k| (k.to_string(), format!("G{}", 54 + k))))
                        .collect();
                    arg_choice(ui, t, "Work offset", &mut wcs, &offsets, "cam:setup:wcs", places);
                    set(d, "wcs", wcs);
                    let mut at = get(d, "at");
                    arg_field(ui, t, "Placed at (machine)", &mut at, "unplaced: G5 spans only", "", "cam:setup:at", places);
                    set(d, "at", at);
                    heading(ui, t, "Workholding", DLG_W);
                    let mut fx = get(d, "fixture");
                    arg_choice(ui, t, "Fixture", &mut fx, &fixtures, "cam:setup:fixture", places);
                    set(d, "fixture", fx);
                    let mut fr = get(d, "from");
                    arg_choice(ui, t, "Continue from", &mut fr, &froms, "cam:setup:from", places);
                    set(d, "from", fr);
                }
                1 => {
                    let mut k = get(d, "stock_kind");
                    let kinds = [
                        ("margin".to_string(), "Box: the part's box plus offsets".to_string()),
                        ("box".to_string(), "Box: fixed size (min, max)".to_string()),
                        ("cylinder".to_string(), "Cylinder: a bar along z".to_string()),
                        ("tube".to_string(), "Tube: a bar with a bore".to_string()),
                        ("solid".to_string(), "From a solid: its own part file".to_string()),
                        ("from".to_string(), "From the earlier setup".to_string()),
                    ];
                    arg_choice(ui, t, "Mode", &mut k, &kinds, "cam:setup:stock", places);
                    set(d, "stock_kind", k.clone());
                    match k.as_str() {
                        "box" => {
                            for (key, label) in [("min", "Min (x, y, z)"), ("max", "Max (x, y, z)")] {
                                let mut s = get(d, key);
                                arg_field(ui, t, label, &mut s, "(x, y, z)", "mm", &format!("cam:setup:{key}"), places);
                                set(d, key, s);
                            }
                        }
                        "from" => note(ui, "The stock that setup provably left, turned over about x.", DLG_W, t.ink2),
                        "cylinder" | "tube" => {
                            let mut rows = vec![("d", "Diameter", "the part's reach + the radial margin")];
                            if k == "tube" {
                                rows.push(("id", "Bore diameter", "required"));
                            }
                            rows.push(("margin", "Margin (radial, above)", "(2, 1)"));
                            rows.push(("below", "Below the part", "0"));
                            for (key, label, hint) in rows {
                                let mut s = get(d, key);
                                arg_field(ui, t, label, &mut s, hint, "mm", &format!("cam:setup:{key}"), places);
                                set(d, key, s);
                            }
                            note(ui, "Along z about the part's centre; it must hold the part, band by band, or the setup is refused.", DLG_W, t.ink2);
                        }
                        "solid" => {
                            let mut s = get(d, "file");
                            arg_field(ui, t, "Stock part file", &mut s, "blank.fg", "", "cam:setup:file", places);
                            set(d, "file", s);
                            note(ui, "A prism along z in the part's frame, beside the document.", DLG_W, t.ink2);
                        }
                        _ => {
                            let mut s = get(d, "margin");
                            arg_field(ui, t, "Offsets (x, y, z)", &mut s, "(3, 3, 2)", "mm", "cam:setup:margin", places);
                            set(d, "margin", s);
                            let mut s = get(d, "below");
                            arg_field(ui, t, "Below the part", &mut s, "0", "mm", "cam:setup:below", places);
                            set(d, "below", s);
                        }
                    }
                }
                2 => {
                    for (key, label, hint) in [
                        ("tol", "Tolerance", "default 0.01"),
                        ("clearance", "Clearance", "default 1"),
                        ("clear", "Clearance height", "above the stock top"),
                        ("retract", "Retract height", "above the stock top"),
                    ] {
                        let mut s = get(d, key);
                        arg_field(ui, t, label, &mut s, hint, "mm", &format!("cam:setup:{key}"), places);
                        set(d, key, s);
                    }
                }
                _ => {
                    note(ui, "Each setup is posted by its program(..) statement: the Post Process dialog picks the dialect and number.", DLG_W, t.ink2);
                }
            }
            if d.tab != 3 {
                forge_text(ui, t, &w.setup_text(d), DLG_W);
            }
            if let Some(e) = &d.error {
                note(ui, e, DLG_W, t.error);
            }
            if !docked {
                (ok, cancel, _) = foot(ui, t, "cam:setup", false, places);
            }
        });
    let rect = r.map(|r| r.response.rect);
    // docked: the commit buttons pinned under the sheet's body, always in sight
    if let Some(dock) = w.dock {
        (ok, cancel, _) = docked_foot(ctx, t, dock, "cam:setup", false, places);
    }
    // Enter is OK (Esc cancels: the shell's one level back, CamWs::escape; Tab moves on)
    ok |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if ok && let Err(e) = w.setup_ok(d) {
        d.error = Some(e);
        return (true, rect);
    }
    (!(ok || cancel || close), rect)
}

/// A lathe setup (`lathe_setup`, docs/cam-turning.md): the bar, the chuck, the lathe and where
/// the work sits on it; the tolerances.
fn lathe_dialog(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    d: &mut SetupDlg,
    places: &mut Places,
) -> (bool, Option<Rect>) {
    let (mut ok, mut cancel, mut close) = (false, false, false);
    let get = |d: &SetupDlg, k: &str| {
        d.args
            .iter()
            .find(|a| a.0 == k)
            .map(|a| a.1.clone())
            .unwrap_or_default()
    };
    let set = |d: &mut SetupDlg, k: &str, v: String| match d.args.iter_mut().find(|a| a.0 == k) {
        Some(a) => a.1 = v,
        None => d.args.push((k.to_string(), v)),
    };
    let mut machines: Vec<(String, String)> = vec![(String::new(), "none (G5 not checked)".into())];
    machines.extend(super::lathes().into_iter().map(|m| (format!("\"{m}\""), m)));
    let mut chucks: Vec<(String, String)> = w
        .stmts
        .iter()
        .filter(|s| s.func == "chuck")
        .map(|s| {
            (
                s.name.clone(),
                format!(
                    "{} · {}",
                    s.name,
                    s.args
                        .iter()
                        .map(|a| a.key.clone().unwrap_or_default() + ": " + &a.text)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })
        .collect();
    chucks.push((
        "chuck(face: -70, grip: 30, jaw: 15)".into(),
        "a new chuck (face −70, grip 30, jaw 15)".into(),
    ));
    let new = d.id.is_none();
    let docked = w.dock.is_some();
    let r = dialog_host(w.dock, "cam-lathe-dialog", t, egui::Window::new("cam-lathe")
        .id(egui::Id::unique("cam-lathe-dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(DLG_W, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v)))
        .show_placed(ctx, places, |ui, places| {
            ui.spacing_mut().item_spacing.y = 4.0;
            close = head(
                ui,
                t,
                "cam_lathe",
                &if new { "New Lathe Setup".to_string() } else { format!("Lathe Setup · {}", d.name) },
                DLG_W,
                places,
                "cam:setup",
            );
            tabs(ui, t, &["Setup", "Stock", "Tolerances"], &mut d.tab, DLG_W, places, "cam:setup:tab");
            ui.add_space(4.0);
            match d.tab {
                0 => {
                    let mut name = d.name.clone();
                    arg_field(ui, t, "Name", &mut name, "", "", "cam:setup:name", places);
                    d.name = name;
                    let mut m = get(d, "machine");
                    arg_choice(ui, t, "Lathe", &mut m, &machines, "cam:setup:machine", places);
                    set(d, "machine", m);
                    heading(ui, t, "Work and tools on the machine", DLG_W);
                    let mut at = get(d, "at");
                    arg_field(ui, t, "Z0 at machine Z", &mut at, "unplaced: G5 spans only", "mm", "cam:setup:at", places);
                    set(d, "at", at);
                    let mut off = get(d, "offsets");
                    arg_field(ui, t, "Tool offsets", &mut off, "((station, x, z), ...)", "", "cam:setup:offsets", places);
                    set(d, "offsets", off);
                    heading(ui, t, "Workholding", DLG_W);
                    let mut c = get(d, "chuck");
                    arg_choice(ui, t, "Chuck", &mut c, &chucks, "cam:setup:chuck", places);
                    set(d, "chuck", c);
                    note(ui, "The chuck's face (z), grip length and jaw height: its envelope as it turns is what G2 keeps every tool body clear of.", DLG_W, t.ink2);
                }
                1 => {
                    for (key, label, hint, unit) in [
                        ("bar", "Bar diameter", "required", "mm"),
                        ("front", "Front face at", "the bar's front face, z", "mm z"),
                    ] {
                        let mut x = get(d, key);
                        arg_field(ui, t, label, &mut x, hint, unit, &format!("cam:setup:{key}"), places);
                        set(d, key, x);
                    }
                    note(ui, "Z is the spindle axis, Z0 where the part puts it; the bar is the stock, its front face the first face cut.", DLG_W, t.ink2);
                }
                _ => {
                    for (key, label, hint) in [
                        ("tol", "Tolerance", "default 0.002"),
                        ("clearance", "Fixture clearance", "default 1"),
                    ] {
                        let mut x = get(d, key);
                        arg_field(ui, t, label, &mut x, hint, "mm", &format!("cam:setup:{key}"), places);
                        set(d, key, x);
                    }
                }
            }
            forge_text(ui, t, &w.setup_text(d), DLG_W);
            if let Some(e) = &d.error {
                note(ui, e, DLG_W, t.error);
            }
            if !docked {
                (ok, cancel, _) = foot(ui, t, "cam:setup", false, places);
            }
        });
    let rect = r.map(|r| r.response.rect);
    // docked: the commit buttons pinned under the sheet's body, always in sight
    if let Some(dock) = w.dock {
        (ok, cancel, _) = docked_foot(ctx, t, dock, "cam:setup", false, places);
    }
    // Enter is OK (Esc cancels: the shell's one level back, CamWs::escape; Tab moves on)
    ok |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if ok && let Err(e) = w.setup_ok(d) {
        d.error = Some(e);
        return (true, rect);
    }
    (!(ok || cancel || close), rect)
}

fn post_window(ctx: &egui::Context, t: &Theme, v: Rect, w: &mut CamWs, places: &mut Places) -> Option<Rect> {
    let mut close = false;
    let progs: Vec<String> = w
        .stmts
        .iter()
        .filter(|s| s.role == super::doc::Role::Program)
        .map(|s| s.name.clone())
        .collect();
    // within the view's free part (a small screen narrows it)
    let width = 620.0_f32.min(v.width() - 32.0).max(360.0);
    let mut edits: Vec<super::doc::Edit> = vec![];
    let mut write: Option<(String, String)> = None;
    let (mut post_it, mut export_it): (Option<String>, Option<String>) = (None, None);
    let r = egui::Window::new("cam-post")
        .id(egui::Id::unique("cam-post-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.right() - width - 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v))
        .show_placed(ctx, places, |ui, places| {
            close = head(ui, t, "cam_post", "Post Process", width, places, "cam:post");
            if progs.is_empty() {
                note(
                    ui,
                    "No program(..) in the document: a new setup makes one.",
                    width,
                    t.ink2,
                );
                return;
            }
            let d = w.post_dlg.as_mut().expect("open");
            d.program = d.program.min(progs.len() - 1);
            let mut k = d.program;
            let names: Vec<&str> = progs.iter().map(String::as_str).collect();
            tabs(ui, t, &names, &mut k, width, places, "cam:post:prog");
            d.program = k;
            let id = progs[k].clone();
            let s = w.stmt(&id).cloned().expect("listed");
            let mut post = s.arg("post").unwrap_or("").to_string();
            // a lathe's program posts with the lathe dialects (forge-turn's)
            let dialects: Vec<(String, String)> = if s.func == "lathe_program" {
                ["fanuc-lathe", "haas-lathe", "linuxcnc-lathe"]
                    .iter()
                    .map(|n| (format!("\"{n}\""), (*n).to_string()))
                    .collect()
            } else {
                forge_post::DIALECTS
                    .iter()
                    .map(|(n, _)| (format!("\"{n}\""), (*n).to_string()))
                    .collect()
            };
            let was = post.clone();
            arg_choice(ui, t, "Dialect", &mut post, &dialects, "cam:post:dialect", places);
            if post != was {
                edits.push(super::doc::Edit::Set {
                    id: id.clone(),
                    arg: "post".into(),
                    text: post,
                });
            }
            let mut number = s.arg("number").unwrap_or("").to_string();
            let was = number.clone();
            arg_field(
                ui,
                t,
                "Program number",
                &mut number,
                "",
                "",
                "cam:post:number",
                places,
            );
            if number != was && number.trim().parse::<u32>().is_ok() {
                edits.push(super::doc::Edit::Set {
                    id: id.clone(),
                    arg: "number".into(),
                    text: number.trim().into(),
                });
            }
            let res = w.run.as_ref().and_then(|r| r.program(&id));
            match &res {
                Some(p) => {
                    fact(
                        ui,
                        t,
                        "Lines · round trip",
                        &format!("{} · {}", p.lines, if p.round_trip { "exact" } else { "no" }),
                        width,
                        t.ink,
                    );
                    fact(
                        ui,
                        t,
                        "Certified",
                        &p.certified,
                        width,
                        if p.certified == "full" { t.success } else { t.warn },
                    );
                    if let Some(e) = &p.error {
                        note(ui, e, width, t.error);
                    }
                }
                None => note(ui, "Not generated yet.", width, t.ink2),
            }
            if w.stale
                .iter()
                .any(|o| w.stmt(o).and_then(|s| s.setup.clone()) == s.setup)
            {
                note(
                    ui,
                    "Some of its ops need generating: the text below is from the last run.",
                    width,
                    t.warn,
                );
            }
            let text = res.as_ref().and_then(|p| p.text.clone()).unwrap_or_default();
            let (r, _) = ui.allocate_exact_size(vec2(width, 360.0), Sense::hover());
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(14.0, 4.0))));
            child.painter().rect_filled(child.max_rect(), 4.0, t.field);
            places.push(("cam:post:text".into(), r));
            egui::ScrollArea::vertical()
                .id_salt("cam-post-text")
                .auto_shrink([false, false])
                .show(&mut child, |ui| {
                    if text.is_empty() {
                        ui.label(
                            egui::RichText::new("(no program: an op of it is refused, or not generated)")
                                .font(mono(12.0))
                                .color(t.ink2),
                        );
                    }
                    for (n, l) in text.lines().enumerate() {
                        ui.label(
                            egui::RichText::new(format!("{:>4}  {l}", n + 1))
                                .font(mono(12.0))
                                .color(t.ink),
                        );
                    }
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                // posted as the NC Programs window posts it: named by the statement, refused
                // when not certified in full unless allowed
                if !text.is_empty() && button(ui, t, "Post", true, "cam:post:write".into(), places) {
                    post_it = Some(id.clone());
                }
                if !text.is_empty() && button(ui, t, "Export…", false, "cam:post:export".into(), places) {
                    export_it = Some(id.clone());
                }
                if button(ui, t, "Setup Sheet", false, "cam:post:sheet".into(), places)
                    && let Some(r) = &w.run
                {
                    let p = w.path.trim_end_matches(".fg").to_string() + "-sheet.html";
                    write = Some((p, r.sheet.clone()));
                }
            });
            ui.add_space(10.0);
        });
    for e in &edits {
        w.edit(e);
    }
    if let Some(x) = write {
        w.want_write.push(x);
    }
    if let Some(n) = post_it {
        w.post_programs(Some(&n));
    }
    if let Some(n) = export_it {
        w.export_programs(Some(&n));
    }
    if close {
        w.post_dlg = None;
    }
    r.map(|r| r.response.rect)
}

fn tools_window(ctx: &egui::Context, t: &Theme, v: Rect, w: &mut CamWs, places: &mut Places) -> Option<Rect> {
    let mut close = false;
    // within the view (a side panel takes its right): its left, as the other windows open
    // two panes (the list, the tool shown) where the view has room, else one over the other
    let width = if w.turning() { 560.0_f32 } else { 860.0_f32 }
        .min(v.width() - 48.0)
        .max(380.0);
    let mut add: Option<String> = None;
    let r = egui::Window::new("cam-tools")
        .id(egui::Id::unique("cam-tools-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v))
        .show_placed(ctx, places, |ui, places| {
            close = head(ui, t, "cam_tools", "Tool Library", width, places, "cam:tools");
            let mut f = w.tools_filter.clone();
            row(ui, t, "Filter", width, 30.0, |ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut f)
                        .font(sans(13.0))
                        .hint_text("type, id or diameter")
                        .desired_width(ui.available_width() - 6.0),
                );
                places.push(("cam:tools:filter".into(), r.rect));
            });
            w.tools_filter = f.clone();
            let f = f.to_lowercase();
            let used: Vec<String> = w
                .tools()
                .iter()
                .filter_map(|s| s.pos(0).map(super::doc::unquote))
                .collect();
            if w.turning() {
                lathe_tools(ui, t, width, &f, &used, &mut add, places);
                ui.add_space(8.0);
                return;
            }
            ui.add_space(8.0);
        });
    if let Some(id) = add.clone().filter(|_| w.turning()) {
        // a lathe tool at the next free station
        let tn = (1..)
            .find(|k| !w.tools().iter().any(|s| s.arg("t") == Some(&k.to_string())))
            .unwrap_or(1);
        let name = super::doc::free_name(&w.stmts, &format!("t{tn}"));
        let before = w
            .tools()
            .last()
            .and_then(|s| w.stmts.iter().position(|x| x.name == s.name))
            .and_then(|k| w.stmts.get(k + 1))
            .or_else(|| w.stmts.iter().find(|s| s.role == super::doc::Role::Setup))
            .map(|s| s.name.clone());
        w.edit(&super::doc::Edit::Add {
            before,
            text: format!("{name} = lathe_tool(\"{id}\", t: {tn})"),
        });
        add = None;
    }
    if let Some(id) = add {
        add_mill_tool(w, id);
    }
    if close {
        w.tools_open = false;
    }
    r.map(|r| r.response.rect)
}

/// A library tool into the document: `tool("id", t: N, stickout: .., holder: ..)` at the next
/// free number, after the document's last tool (else before its first setup); it is recent.
fn add_mill_tool(w: &mut CamWs, id: String) {
    super::toollib::touch(&mut w.tools_recent, &id);
    let n = w.tools().len() + 1;
    let tn = (1..)
        .find(|k| !w.tools().iter().any(|s| s.arg("t") == Some(&k.to_string())))
        .unwrap_or(n);
    let name = super::doc::free_name(&w.stmts, &format!("t{tn}"));
    let holder = w
        .lib
        .holders
        .first()
        .map(|h| format!(", holder: \"{}\"", h.id))
        .unwrap_or_default();
    let stick = w
        .lib
        .tools
        .iter()
        .find(|x| x.id == id)
        .map_or(30.0, |x| (x.lcf + 10.0).round());
    // after the last tool, else before the first setup
    let before = w
        .tools()
        .last()
        .and_then(|s| w.stmts.iter().position(|x| x.name == s.name))
        .and_then(|k| w.stmts.get(k + 1))
        .or_else(|| w.stmts.iter().find(|s| s.role == super::doc::Role::Setup))
        .map(|s| s.name.clone());
    w.edit(&super::doc::Edit::Add {
        before,
        text: format!(
            "{name} = tool(\"{id}\", t: {tn}, stickout: {}{holder})",
            num(stick)
        ),
    });
}

/// The milling tool library as a page in place of the view and the properties column (UX-LEAD:
/// browsing tools is a task of its own, the part need not be in view): a header with the way
/// back, the finder on the left (search, type, diameter, material; the document's tools, the
/// recent ones, the library), the tool shown on the right. Esc goes back too.
fn tools_page(ctx: &egui::Context, t: &Theme, v: Rect, w: &mut CamWs, places: &mut Places) -> Option<Rect> {
    let page = Rect::from_min_max(v.min, pos2(ctx.content_rect().right(), v.bottom()));
    let mut add: Option<String> = None;
    let mut back = false;
    // the document's tools: library id to its statement's number and name
    let in_doc: Vec<(String, String, String)> = w
        .tools()
        .iter()
        .filter_map(|s| {
            Some((
                s.pos(0).map(super::doc::unquote)?,
                s.arg("t").unwrap_or("?").to_string(),
                s.name.clone(),
            ))
        })
        .collect();
    if w.tools_sel.is_none() {
        w.tools_sel = in_doc
            .first()
            .map(|d| d.0.clone())
            .or_else(|| w.lib.tools.first().map(|x| x.id.clone()));
    }
    egui::Area::new(egui::Id::unique("cam-tools-page"))
        .fixed_pos(page.min)
        .order(egui::Order::Foreground)
        .constrain(false)
        .default_size(page.size())
        .fade_in(false)
        .show(ctx, |ui| {
            ui.set_min_size(page.size());
            ui.set_max_size(page.size());
            ui.set_clip_rect(page);
            // the page takes the pointer: nothing under it (the view, the column) is reached
            let (bg, _) = ui.allocate_exact_size(page.size(), Sense::click_and_drag());
            ui.painter().rect_filled(bg, 0.0, t.bg);
            places.push(("cam:tools:page".into(), bg));
            // the header
            let head = Rect::from_min_size(page.min, vec2(page.width(), 48.0));
            ui.painter().rect_filled(head, 0.0, t.surface);
            ui.painter()
                .hline(head.x_range(), head.bottom(), Stroke::new(1.0, t.line));
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(head.shrink2(vec2(12.0, 8.0))),
                |ui| {
                    ui.horizontal_centered(|ui| {
                        if button(
                            ui,
                            t,
                            "Back to Manufacture",
                            false,
                            "cam:tools:back".into(),
                            places,
                        ) {
                            back = true;
                        }
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("Tool Library")
                                .font(medium(16.0))
                                .color(t.ink),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!("{} tools", w.lib.tools.len()))
                                .font(sans(12.0))
                                .color(t.ink2),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            for (aux, word, key) in [
                                (2u8, "Holders", "cam:tools:holders"),
                                (1, "Import Fusion library\u{2026}", "cam:tools:import-open"),
                            ] {
                                if button(ui, t, word, false, key.into(), places) {
                                    w.tools_aux = if w.tools_aux == aux { 0 } else { aux };
                                    w.tools_form = None;
                                }
                            }
                            if button(ui, t, "New tool\u{2026}", false, "cam:tools:new".into(), places) {
                                w.tools_aux = 0;
                                w.tools_form =
                                    Some(super::toollib::ToolForm::blank(&w.lib, forge_tooling::Kind::Flat));
                            }
                        });
                    });
                },
            );
            let body = Rect::from_min_max(pos2(page.left(), head.bottom() + 1.0), page.max);
            let list_w = 360.0_f32.min(body.width() * 0.45);
            let left = Rect::from_min_max(body.min, pos2(body.left() + list_w, body.bottom()));
            let right = Rect::from_min_max(pos2(left.right() + 1.0, body.top()), body.max);
            ui.painter()
                .vline(left.right(), body.y_range(), Stroke::new(1.0, t.line));
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(left.shrink2(vec2(0.0, 8.0))),
                |ui| {
                    finder(ui, t, w, left.width(), &in_doc, &mut add, places);
                },
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                detail_pane(ui, t, w, right, &in_doc, &mut add, places);
            });
        });
    if let Some(id) = add {
        add_mill_tool(w, id);
    }
    if back {
        w.tools_open = false;
        w.tools_form = None;
        w.tools_aux = 0;
    }
    Some(page)
}

/// The finder: the search, the type chips (one row, scrolling), the diameter range and the
/// material; then the document's tools (their numbers), the recent ones and the library.
fn finder(
    ui: &mut Ui,
    t: &Theme,
    w: &mut CamWs,
    width: f32,
    in_doc: &[(String, String, String)],
    add: &mut Option<String>,
    places: &mut Places,
) {
    use super::toollib::{self, KIND_CHIPS};
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut w.tools_find.text)
                .font(sans(13.0))
                .hint_text("Search: id, type, \u{d8} or maker")
                .desired_width(width - 24.0)
                .margin(vec2(7.0, 5.0)),
        );
        places.push(("cam:tools:search".into(), r.rect));
    });
    ui.add_space(4.0);
    // the type chips: one row, scrolled sideways where it does not fit
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        egui::ScrollArea::horizontal()
            .id_salt("cam-tools-chips")
            .max_width(width - 24.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let all = ui.selectable_label(w.tools_find.kind.is_none(), "All");
                    places.push(("cam:tools:kind:all".into(), all.rect));
                    if all.clicked() {
                        w.tools_find.kind = None;
                    }
                    for (k, (name, _)) in KIND_CHIPS.iter().enumerate() {
                        let on = w.tools_find.kind == Some(k);
                        let r = ui.selectable_label(on, *name);
                        places.push((format!("cam:tools:kind:{}", name.to_lowercase()), r.rect));
                        if r.clicked() {
                            w.tools_find.kind = if on { None } else { Some(k) };
                        }
                    }
                });
            });
    });
    ui.add_space(4.0);
    // "Ø from [ ] to [ ] mm", the material
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let word = |ui: &mut Ui, s: &str| ui.label(egui::RichText::new(s).font(sans(12.0)).color(t.ink2));
        word(ui, "\u{d8} from");
        ui.add_space(6.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut w.tools_find.dmin)
                .font(sans(12.0))
                .desired_width(40.0),
        );
        places.push(("cam:tools:dmin".into(), r.rect));
        ui.add_space(6.0);
        word(ui, "to");
        ui.add_space(6.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut w.tools_find.dmax)
                .font(sans(12.0))
                .desired_width(40.0),
        );
        places.push(("cam:tools:dmax".into(), r.rect));
        ui.add_space(6.0);
        word(ui, "mm");
        ui.add_space(6.0);
        let mats = toollib::materials(&w.lib);
        let shown = if w.tools_find.material.is_empty() {
            "Any material".to_string()
        } else {
            w.tools_find.material.clone()
        };
        let r = egui::ComboBox::from_id_salt("cam-tools-material")
            .selected_text(shown)
            .width(104.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut w.tools_find.material, String::new(), "Any material");
                for m in mats {
                    ui.selectable_value(&mut w.tools_find.material, m.clone(), m);
                }
            });
        places.push(("cam:tools:material".into(), r.response.rect));
    });
    if !w.tools_find.is_open() {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            if ui.small_button("Clear the search").clicked() {
                w.tools_find = toollib::Find::default();
            }
        });
    }
    ui.add_space(4.0);
    let mut pick: Option<String> = None;
    egui::ScrollArea::vertical()
        .id_salt("cam-tools-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let shown: Vec<&forge_tooling::Tool> = toollib::sorted(&w.lib.tools)
                .into_iter()
                .filter(|x| toollib::matches(&w.tools_find, x))
                .collect();
            let doc: Vec<&forge_tooling::Tool> = shown
                .iter()
                .copied()
                .filter(|x| in_doc.iter().any(|d| d.0 == x.id))
                .collect();
            let recent: Vec<&forge_tooling::Tool> = w
                .tools_recent
                .iter()
                .filter_map(|id| shown.iter().copied().find(|x| &x.id == id))
                .filter(|x| !in_doc.iter().any(|d| d.0 == x.id))
                .collect();
            let rest: Vec<&forge_tooling::Tool> = shown
                .iter()
                .copied()
                .filter(|x| !in_doc.iter().any(|d| d.0 == x.id) && !w.tools_recent.contains(&x.id))
                .collect();
            if shown.is_empty() {
                note(
                    ui,
                    "No tool matches: clear the search, or make one with New tool\u{2026}",
                    width,
                    t.ink2,
                );
            }
            for (title, list) in [("In this document", doc), ("Recent", recent), ("Library", rest)] {
                if list.is_empty() {
                    continue;
                }
                heading(ui, t, &format!("{title} \u{b7} {}", list.len()), width - 12.0);
                for x in list {
                    let badge = in_doc.iter().find(|d| d.0 == x.id).map(|d| format!("T{}", d.1));
                    let sel = w.tools_sel.as_deref() == Some(x.id.as_str());
                    let resp = tool_row(ui, t, x, width - 12.0, sel, badge.as_deref());
                    places.push((format!("cam:tools:row:{}", x.id), resp.rect));
                    if resp.clicked() {
                        pick = Some(x.id.clone());
                    }
                    if resp.double_clicked() && badge.is_none() {
                        *add = Some(x.id.clone());
                    }
                }
            }
        });
    if let Some(id) = pick {
        if w.tools_sel.as_deref() != Some(id.as_str()) {
            w.tools_form = None;
        }
        w.tools_aux = 0;
        w.tools_sel = Some(id);
    }
}

/// One tool in the list: its glyph, "Ø10 · Flat end mill", the id and what else matters under
/// it, its number in the document on the right.
fn tool_row(
    ui: &mut Ui,
    t: &Theme,
    x: &forge_tooling::Tool,
    w: f32,
    sel: bool,
    badge: Option<&str>,
) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
    let p = ui.painter();
    if sel {
        p.rect_filled(r, 4.0, t.on.gamma_multiply(0.35));
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, t.hover);
    }
    tool_glyph(
        p,
        Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), vec2(12.0, 24.0)),
        x.kind,
        t.ink2,
    );
    p.text(
        pos2(r.left() + 40.0, r.top() + 10.0),
        Align2::LEFT_CENTER,
        format!(
            "\u{d8}{} \u{b7} {}",
            num(x.dc),
            super::toollib::kind_words(x.kind)
        ),
        sans(13.0),
        t.ink,
    );
    let mut more = vec![x.id.clone()];
    if x.nof > 0 {
        more.push(format!("{} fl", x.nof));
    }
    more.push(format!("{} mm flute", num(x.lcf)));
    if !x.material.is_empty() {
        more.push(x.material.clone());
    }
    p.text(
        pos2(r.left() + 40.0, r.top() + 26.0),
        Align2::LEFT_CENTER,
        more.join(" \u{b7} "),
        sans(11.0),
        t.ink2,
    );
    if let Some(b) = badge {
        let g = p.layout_no_wrap(b.to_string(), medium(11.0), t.on_ink);
        let br = Rect::from_center_size(
            pos2(r.right() - 10.0 - g.size().x / 2.0 - 5.0, r.center().y),
            g.size() + vec2(10.0, 4.0),
        );
        p.rect_filled(br, 4.0, t.on);
        p.galley(br.min + vec2(5.0, 2.0), g, t.on_ink);
    }
    resp
}

/// The page's right side: the tool shown (its cross-section large on top, its facts in a sheet,
/// its words in the document), the new tool being made, or the holders / the Fusion import;
/// what to do with it in a footer pinned at the bottom (UX-LEAD: the commit buttons always seen).
#[allow(clippy::too_many_arguments)]
fn detail_pane(
    ui: &mut Ui,
    t: &Theme,
    w: &mut CamWs,
    r: Rect,
    in_doc: &[(String, String, String)],
    add: &mut Option<String>,
    places: &mut Places,
) {
    let foot = Rect::from_min_max(pos2(r.left(), r.bottom() - FOOT_H - 8.0), r.max);
    let body = Rect::from_min_max(r.min, pos2(r.right(), foot.top()));
    let x = w
        .tools_sel
        .as_ref()
        .and_then(|id| w.lib.tools.iter().find(|x| &x.id == id))
        .cloned();
    let doc_name = x
        .as_ref()
        .and_then(|x| in_doc.iter().find(|d| d.0 == x.id))
        .map(|d| d.2.clone());
    let inst_id = |name: &str| egui::Id::unique(("cam-tool-inst", name.to_string()));
    // the footer first (its buttons are read by the body below, this same frame)
    ui.painter().rect_filled(foot, 0.0, t.surface);
    ui.painter()
        .hline(foot.x_range(), foot.top(), Stroke::new(1.0, t.line));
    let mut apply = false;
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(foot.shrink2(vec2(14.0, 8.0))),
        |ui| {
            ui.horizontal_centered(|ui| {
                if w.tools_aux != 0 {
                    if button(ui, t, "Close", false, "cam:tools:aux-close".into(), places) {
                        w.tools_aux = 0;
                    }
                } else if let Some(f) = w.tools_form.clone() {
                    let preview = f.toml(&w.lib);
                    if button(
                        ui,
                        t,
                        "Save to project",
                        preview.is_ok(),
                        "cam:tools:save".into(),
                        places,
                    ) {
                        match preview.and_then(|text| w.save_tool(&text, f.id.trim())) {
                            Ok(m) => {
                                w.said = Some(m);
                                w.tools_sel = Some(f.id.trim().to_string());
                                w.tools_form = None;
                            }
                            Err(e) => w.said = Some(e),
                        }
                    }
                    if button(ui, t, "Cancel", false, "cam:tools:cancel".into(), places) {
                        w.tools_form = None;
                    }
                } else if let Some(x) = &x {
                    match &doc_name {
                        Some(name) => {
                            let changed =
                                ui.data(|d| d.get_temp::<(String, String, String)>(inst_id(name)).is_some());
                            apply =
                                button(ui, t, "Apply", changed, "cam:tools:apply".into(), places) && changed;
                        }
                        None => {
                            if button(ui, t, "Add to document", true, "cam:tools:add".into(), places) {
                                *add = Some(x.id.clone());
                            }
                        }
                    }
                    if button(
                        ui,
                        t,
                        "Derive from this tool\u{2026}",
                        false,
                        "cam:tools:derive".into(),
                        places,
                    ) {
                        w.tools_form = Some(super::toollib::ToolForm::derived(&w.lib, x));
                    }
                }
                if let Some(s) = &w.said {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(s).font(sans(12.0)).color(t.ink2));
                }
            });
        },
    );
    let width = (body.width() - 8.0).min(640.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical().id_salt("cam-tools-detail").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(body.width() - 8.0);
            if w.tools_aux == 1 {
                return import_pane(ui, t, w, width, places);
            }
            if w.tools_aux == 2 {
                heading(ui, t, &format!("Holders \u{b7} {}", w.lib.holders.len()), width);
                for h in &w.lib.holders {
                    fact(ui, t, &h.id, &format!("{} \u{b7} gauge {} mm \u{b7} max {} rpm", h.interface, num(h.gauge_length), grouped(h.max_rpm)), width, t.ink);
                }
                return;
            }
            if w.tools_form.is_some() {
                return form_body(ui, t, w, width, body.height(), places);
            }
            let Some(x) = &x else {
                note(ui, "Choose a tool on the left.", width, t.ink2);
                return;
            };
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new(&x.id).font(medium(17.0)).color(t.ink));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(super::toollib::kind_words(x.kind)).font(sans(14.0)).color(t.ink2));
                if let Some(name) = &doc_name {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(format!("in this document as {name}")).font(sans(12.0)).color(t.ink2));
                }
            });
            // where its numbers come from
            let (origin, tip) = if x.real {
                (format!("{} {} \u{b7} the maker's values, each cited", x.vendor, x.product_id), "Every value is cited to the maker's published data.")
            } else if x.source.starts_with("derived from") || x.source == "made in the tool library" {
                (format!("Project tool \u{b7} {} \u{b7} cam/tools.toml", x.source), "The project's own record, beside the document.")
            } else {
                (
                    "Built in: generic, our own numbers \u{b7} verify with the vendor".to_string(),
                    "Generic tools and conservative holders, our own numbers: verify with the vendor. A project adds its own in cam/*.toml beside the document.",
                )
            };
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new(origin).font(sans(12.0)).color(t.ink2)).on_hover_text(tip);
            });
            // the cross-section, large
            let ph = (body.height() * 0.42).clamp(180.0, 320.0);
            let (pr, _) = ui.allocate_exact_size(vec2(width, ph), Sense::hover());
            places.push(("cam:tools:profile".into(), pr));
            tool_profile(ui.painter(), pr.shrink2(vec2(24.0, 8.0)), x, t);
            // its facts, in a sheet
            let mut facts = vec![
                ("Diameter", format!("{} mm", num(x.dc))),
                ("Flutes", x.nof.to_string()),
                ("Flute length", format!("{} mm", num(x.lcf))),
                ("Shank", format!("\u{d8}{} mm", num(x.dcon))),
                ("Overall length", format!("{} mm", num(x.oal))),
            ];
            use forge_tooling::Kind as K;
            match x.kind {
                K::Bull => facts.push(("Corner radius", format!("{} mm", num(x.re)))),
                K::Drill => facts.push(("Point angle", format!("{}\u{b0}", num(x.angle)))),
                K::Chamfer | K::Engrave | K::Spot | K::Countersink => {
                    facts.push(("Included angle", format!("{}\u{b0}", num(x.angle))));
                    if x.tip_d > 0.0 {
                        facts.push(("Tip diameter", format!("{} mm", num(x.tip_d))));
                    }
                }
                K::Tap | K::ThreadMill => facts.push(("Pitch", format!("{} mm", num(x.tp)))),
                _ => {}
            }
            if !x.material.is_empty() {
                facts.push(("Material", x.material.clone()));
            }
            // in the document: its number, stick-out and holder (Apply in the footer)
            if let Some(name) = &doc_name {
                heading(ui, t, &format!("{name} in this document"), width);
                tool_instance(ui, t, w, name, inst_id(name), apply, places);
            }
            heading(ui, t, "The tool", width);
            sheet(ui, t, width, |ui, iw| {
                for (k, val) in &facts {
                    fact(ui, t, k, val, iw, t.ink);
                }
            });
            ui.add_space(12.0);
        });
    });
}

/// A boxed group of facts (UX's sheet): `body` gets the inner width.
fn sheet(ui: &mut Ui, t: &Theme, width: f32, body: impl FnOnce(&mut Ui, f32)) {
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        egui::Frame::new()
            .fill(t.surface)
            .stroke(Stroke::new(1.0, t.line))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(0, 6))
            .show(ui, |ui| {
                ui.set_width(width - 28.0);
                ui.vertical(|ui| body(ui, width - 28.0));
            });
    });
}

/// A document tool's own words: its number, stick-out and holder, set in its statement when
/// `apply` (the footer's Apply); the edits kept meanwhile under `id`.
fn tool_instance(
    ui: &mut Ui,
    t: &Theme,
    w: &mut CamWs,
    name: &str,
    id: egui::Id,
    apply: bool,
    places: &mut Places,
) {
    let Some(st) = w.stmt(name).cloned() else { return };
    let now = (
        st.arg("t").unwrap_or("").to_string(),
        st.arg("stickout").unwrap_or("").to_string(),
        st.arg("holder").map(super::doc::unquote).unwrap_or_default(),
    );
    let mut ed: (String, String, String) = ui.data_mut(|d| d.get_temp(id)).unwrap_or_else(|| now.clone());
    arg_field(
        ui,
        t,
        "Tool number",
        &mut ed.0,
        "",
        "",
        &format!("cam:tools:inst:{name}:t"),
        places,
    );
    arg_field(
        ui,
        t,
        "Stick-out",
        &mut ed.1,
        "",
        "mm",
        &format!("cam:tools:inst:{name}:stickout"),
        places,
    );
    let holders: Vec<(String, String)> = std::iter::once((String::new(), "none".to_string()))
        .chain(
            w.lib
                .holders
                .iter()
                .map(|h| (h.id.clone(), format!("{} \u{b7} {}", h.id, h.interface))),
        )
        .collect();
    arg_choice(
        ui,
        t,
        "Holder",
        &mut ed.2,
        &holders,
        &format!("cam:tools:inst:{name}:holder"),
        places,
    );
    let mut changed = ed != now;
    if apply && changed {
        let mut edits = vec![];
        for (arg, new, old, quote) in [
            ("t", &ed.0, &now.0, false),
            ("stickout", &ed.1, &now.1, false),
            ("holder", &ed.2, &now.2, true),
        ] {
            if new == old {
                continue;
            }
            edits.push(if new.trim().is_empty() {
                super::doc::Edit::Unset {
                    id: name.to_string(),
                    arg: arg.into(),
                }
            } else {
                super::doc::Edit::Set {
                    id: name.to_string(),
                    arg: arg.into(),
                    text: if quote {
                        format!("\"{}\"", new.trim())
                    } else {
                        new.trim().to_string()
                    },
                }
            });
        }
        match w.edits(&edits) {
            Ok(()) => changed = false,
            Err(e) => w.said = Some(e),
        }
    }
    ui.data_mut(|d| {
        if changed {
            d.insert_temp(id, ed);
        } else {
            d.remove::<(String, String, String)>(id);
        }
    });
}

/// A new tool: from nothing (its type chosen) or derived (the source's type), its cross-section
/// drawn as typed; Save and Cancel are the footer's.
fn form_body(ui: &mut Ui, t: &Theme, w: &mut CamWs, width: f32, h: f32, places: &mut Places) {
    use super::toollib;
    let Some(mut f) = w.tools_form.take() else { return };
    ui.add_space(10.0);
    let title = match &f.from {
        Some(src) => format!("Derived from {src}"),
        None => "New tool".to_string(),
    };
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(egui::RichText::new(title).font(medium(17.0)).color(t.ink));
    });
    note(
        ui,
        if f.from.is_some() {
            "Its values, under a new id: change what differs. What you change is cited as yours; the rest keeps the source's cites. Saved to cam/tools.toml beside the document."
        } else {
            "A tool of your own, saved to cam/tools.toml beside the document."
        },
        width,
        t.ink2,
    );
    // what it would be, drawn as typed (when it reads)
    let preview = f.toml(&w.lib);
    let ph = (h * 0.32).clamp(150.0, 240.0);
    let (pr, _) = ui.allocate_exact_size(vec2(width, ph), Sense::hover());
    if let Ok(text) = &preview {
        let mut l = w.lib.clone();
        if l.add("preview", text).is_ok()
            && let Some(x) = l.tools.iter().find(|x| x.id == f.id.trim())
        {
            tool_profile(ui.painter(), pr.shrink2(vec2(24.0, 8.0)), x, t);
        }
    }
    if f.from.is_none() {
        let kinds: Vec<(String, String)> = toollib::KIND_CHIPS
            .iter()
            .flat_map(|c| c.1.iter())
            .map(|k| (k.name().to_string(), toollib::kind_words(*k).to_string()))
            .collect();
        let mut kn = f.kind.name().to_string();
        arg_choice(ui, t, "Type", &mut kn, &kinds, "cam:tools:form:type", places);
        if let Some(k) = forge_tooling::Kind::parse(&kn)
            && k != f.kind
        {
            f.set_kind(k);
        }
    }
    let mut id = f.id.clone();
    arg_field(ui, t, "Id", &mut id, "", "", "cam:tools:form:id", places);
    f.id = id;
    for (k, label, unit) in toollib::form_fields(f.kind) {
        let mut val = f.get(k).to_string();
        arg_field(
            ui,
            t,
            label,
            &mut val,
            "",
            unit,
            &format!("cam:tools:form:{k}"),
            places,
        );
        f.set(k, val);
    }
    if let Err(e) = &preview {
        note(ui, e, width, t.warn);
    }
    ui.add_space(12.0);
    w.tools_form = Some(f);
}

/// The Fusion import, on the page's right side.
fn import_pane(ui: &mut Ui, t: &Theme, w: &mut CamWs, width: f32, places: &mut Places) {
    heading(ui, t, "Import a Fusion tool library", width);
    note(
        ui,
        "A Fusion 360 tool library (its .json export) beside the document, converted into the project's cam/<name>.toml.",
        width,
        t.ink2,
    );
    let mut ip = w.import_path.clone();
    let mut go = false;
    row(ui, t, "JSON file", width, 30.0, |ui| {
        let r = ui.add(
            egui::TextEdit::singleline(&mut ip)
                .font(sans(13.0))
                .hint_text("beside the document, e.g. shop.json")
                .desired_width(ui.available_width() - 80.0),
        );
        places.push(("cam:tools:import:path".into(), r.rect));
        go = button(ui, t, "Import", true, "cam:tools:import".into(), places);
    });
    w.import_path = ip;
    if go {
        let f = w.import_path.clone();
        w.said = Some(match w.import_fusion(&f) {
            Ok(m) => m,
            Err(e) => format!("import: {e}"),
        });
    }
}

/// A tool's side view to scale: shank, flutes (helix lines), and its end as its type makes it
/// (square, ball, corner radius, point or chamfer, thread), with its diameter, flute length and
/// overall length marked.
fn tool_profile(p: &egui::Painter, r: Rect, x: &forge_tooling::Tool, t: &Theme) {
    use forge_tooling::Kind as K;
    let oal = x.oal.max(x.lcf).max(1e-3);
    let wmax = x.dc.max(x.dcon).max(1e-3);
    // tip at the bottom; room on the right for the marks
    let s = ((r.height() - 22.0) / oal as f32).min((r.width() * 0.3) / wmax as f32);
    let cx = r.center().x;
    let tip = r.bottom() - 14.0;
    let y = |mm: f64| tip - mm as f32 * s;
    let hw = |d: f64| d as f32 * s / 2.0;
    let (rd, rs) = (hw(x.dc), hw(x.dcon));
    let fill = t.ink2.gamma_multiply(0.18);
    let st = Stroke::new(1.3, t.ink);
    // the shank
    let shank = Rect::from_min_max(pos2(cx - rs, y(oal)), pos2(cx + rs, y(x.lcf)));
    p.rect_filled(shank, 0.0, fill);
    p.rect_stroke(shank, 0.0, st, egui::StrokeKind::Middle);
    // the end: its outline from the left at flute top, down round the tip, up the right
    let lcf = y(x.lcf);
    let mut pts = vec![pos2(cx - rd, lcf)];
    let arc = |pts: &mut Vec<egui::Pos2>, c: egui::Pos2, rad: f32, a0: f32, a1: f32| {
        for k in 0..=12 {
            let a = a0 + (a1 - a0) * k as f32 / 12.0;
            pts.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    };
    let point = |ang: f64, tip_half: f32| -> f32 {
        // the height of a cone of this included angle from radius rd to tip_half
        let half = (ang.max(1.0) / 2.0).to_radians() as f32;
        (rd - tip_half).max(0.0) / half.tan()
    };
    match x.kind {
        K::Ball => {
            pts.push(pos2(cx - rd, tip - rd));
            arc(&mut pts, pos2(cx, tip - rd), rd, std::f32::consts::PI, 0.0);
            // egui y grows down: the arc above went the wrong way; mirror it below the centre
            let n = pts.len();
            for q in pts[n - 13..].iter_mut() {
                q.y = 2.0 * (tip - rd) - q.y;
            }
        }
        K::Bull => {
            let re = (x.re as f32 * s).min(rd);
            pts.push(pos2(cx - rd, tip - re));
            let n0 = pts.len();
            arc(
                &mut pts,
                pos2(cx - rd + re, tip - re),
                re,
                std::f32::consts::PI,
                std::f32::consts::FRAC_PI_2,
            );
            arc(
                &mut pts,
                pos2(cx + rd - re, tip - re),
                re,
                std::f32::consts::FRAC_PI_2,
                0.0,
            );
            let _ = n0;
        }
        K::Drill | K::Spot | K::Countersink | K::Chamfer | K::Engrave => {
            let th = if matches!(x.kind, K::Chamfer | K::Engrave) {
                hw(x.tip_d)
            } else {
                0.0
            };
            let h = point(x.angle, th);
            pts.push(pos2(cx - rd, tip - h));
            pts.push(pos2(cx - th, tip));
            pts.push(pos2(cx + th, tip));
        }
        _ => {
            pts.push(pos2(cx - rd, tip));
            pts.push(pos2(cx + rd, tip));
        }
    }
    match x.kind {
        K::Drill | K::Spot | K::Countersink | K::Chamfer | K::Engrave => {
            let th = if matches!(x.kind, K::Chamfer | K::Engrave) {
                hw(x.tip_d)
            } else {
                0.0
            };
            pts.push(pos2(cx + rd, tip - point(x.angle, th)));
        }
        K::Ball | K::Bull => pts.push(pos2(
            cx + rd,
            tip - if x.kind == K::Ball {
                rd
            } else {
                (x.re as f32 * s).min(rd)
            },
        )),
        _ => {}
    }
    pts.push(pos2(cx + rd, lcf));
    p.add(egui::Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
    p.add(egui::Shape::closed_line(pts, st));
    // the flutes: helix lines across the fluted length (threads: teeth)
    let lines = ((x.lcf as f32 * s) / 9.0).clamp(2.0, 14.0) as usize;
    for k in 1..lines {
        let yy = lcf + (tip - lcf) * k as f32 / lines as f32;
        if yy > tip - rd * 0.9
            && matches!(
                x.kind,
                K::Ball | K::Drill | K::Spot | K::Chamfer | K::Countersink | K::Engrave
            )
        {
            break;
        }
        let tilt = if matches!(x.kind, K::Tap | K::ThreadMill) {
            0.0
        } else {
            rd * 0.5
        };
        p.line_segment(
            [pos2(cx - rd, yy + tilt), pos2(cx + rd, yy - tilt)],
            Stroke::new(1.0, t.ink2.gamma_multiply(0.6)),
        );
    }
    // the marks: diameter under the tip, flute length and overall length on the right
    let mark = Stroke::new(1.0, t.ink2);
    let font = mono(11.0);
    p.text(
        pos2(cx, tip + 8.0),
        Align2::CENTER_CENTER,
        format!("\u{d8}{}", num(x.dc)),
        font.clone(),
        t.ink2,
    );
    // flute length on the right, overall length on the left: the marks stay inside `r`
    let side = rd.max(rs) + 12.0;
    for (from, to, label, right) in [(tip, lcf, num(x.lcf), true), (tip, y(oal), num(x.oal), false)] {
        let xx = if right { cx + side } else { cx - side };
        p.line_segment([pos2(xx, from), pos2(xx, to)], mark);
        p.line_segment([pos2(xx - 4.0, from), pos2(xx + 4.0, from)], mark);
        p.line_segment([pos2(xx - 4.0, to), pos2(xx + 4.0, to)], mark);
        let (at, al) = if right {
            (pos2(xx + 6.0, (from + to) / 2.0), Align2::LEFT_CENTER)
        } else {
            (pos2(xx - 6.0, (from + to) / 2.0), Align2::RIGHT_CENTER)
        };
        p.text(at, al, label, font.clone(), t.ink2);
    }
    if (x.dcon - x.dc).abs() > 1e-6 {
        p.text(
            pos2(cx, y(oal) - 8.0),
            Align2::CENTER_CENTER,
            format!("shank \u{d8}{}", num(x.dcon)),
            font,
            t.ink2,
        );
    }
}

/// The lathe tool library (forge-turn's built-in records): insert, holder or bar, nose radius,
/// cutting data; Add puts a `lathe_tool(..)` at the next free station.
#[allow(clippy::too_many_arguments)]
fn lathe_tools(
    ui: &mut Ui,
    t: &Theme,
    width: f32,
    f: &str,
    used: &[String],
    add: &mut Option<String>,
    places: &mut Places,
) {
    heading(ui, t, "Lathe tools", width);
    let lib = forge_turn::insert::library(forge_turn::insert::BUILTIN).unwrap_or_default();
    let (r, _) = ui.allocate_exact_size(vec2(width, 320.0), Sense::hover());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
    egui::ScrollArea::vertical()
        .id_salt("cam-lathe-tools")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            for tool in &lib {
                let line = format!(
                    "{}  ·  {:?}  ·  {}  ·  r {}  ·  vc {}  ·  f {}",
                    tool.id,
                    tool.kind,
                    tool.code,
                    num(tool.re),
                    num(tool.vc),
                    num(tool.f)
                );
                if !f.is_empty() && !line.to_lowercase().contains(f) {
                    continue;
                }
                let (r, resp) = ui.allocate_exact_size(vec2(width - 12.0, 24.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, 0.0, t.hover);
                }
                let inu = used.contains(&tool.id);
                ui.painter().text(
                    pos2(r.left() + 14.0, r.center().y),
                    Align2::LEFT_CENTER,
                    &line,
                    sans(12.0),
                    t.ink,
                );
                ui.painter().text(
                    pos2(r.right() - 10.0, r.center().y),
                    Align2::RIGHT_CENTER,
                    if inu { "in document" } else { "Add" },
                    sans(12.0),
                    if inu { t.ink2 } else { t.accent },
                );
                places.push((format!("cam:tools:row:{}", tool.id), r));
                if resp.clicked() && !inu {
                    *add = Some(tool.id.clone());
                }
            }
        });
    note(
        ui,
        "Inserts and holders decoded from their ISO codes (kernel/forge-turn/data/turning.toml); holder heads are generic: verify with the vendor.",
        width,
        t.ink2,
    );
}

/// The turning view's window (turnview.rs), at the view's bottom right.
fn section_window(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    places: &mut Places,
) -> Option<Rect> {
    let mut close = false;
    // half the view at most, and beside a floating window (620 wide at the view's left) when one
    // is open
    let floating = w.programs_open
        || w.quality_open
        || w.tools_open
        || w.checks_open
        || w.text_open
        || w.post_dlg.is_some();
    let room = if floating {
        v.width() - 620.0 - 64.0
    } else {
        v.width() * 0.5
    };
    let width = 560.0_f32.min(room).max(320.0);
    let r = egui::Window::new("cam-section")
        .id(egui::Id::unique("cam-section-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        // docked at the view's bottom right: the floating windows open at its top left
        .pivot(Align2::RIGHT_BOTTOM)
        .fixed_pos(pos2(v.right() - 16.0, v.bottom() - 16.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v))
        .show_placed(ctx, places, |ui, places| {
            close = head(
                ui,
                t,
                "cam_lathe",
                "Section · radius up, z along the spindle",
                width,
                places,
                "cam:turn",
            );
            super::turnview::body(ui, t, w, width, places);
            ui.add_space(8.0);
        });
    if close {
        w.turnview.open = false;
    }
    r.map(|r| r.response.rect)
}

/// The NC programs, Fusion's NC Program manager: every program, the setups it posts, its
/// dialect and number, what it made, its file (name, folder, extension, header) and whether that
/// file is posted and current; Post (into the document's folder), Export… (the host's Save As),
/// Re-post Stale, the setup sheet and tool list. Every setting is an argument of the program's
/// statement, saved with the document (forge_cam_lang::export).
fn programs_window(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    places: &mut Places,
) -> Option<Rect> {
    use super::nc::Posted;
    let mut close = false;
    // within the view's free part (a small screen narrows it)
    let width = match w.dock_aux {
        Some(d) => d.width() - 16.0,
        None => 620.0_f32.min(v.width() - 32.0).max(360.0),
    };
    let mut edits: Vec<super::doc::Edit> = vec![];
    let mut cmds: Vec<Cmd> = vec![];
    let progs: Vec<super::doc::Stmt> = w
        .stmts
        .iter()
        .filter(|s| s.role == super::doc::Role::Program)
        .cloned()
        .collect();
    let setups: Vec<String> = w.setups().iter().map(|s| s.name.clone()).collect();
    let lathe = w.turning();
    let posted = w.posted();
    let ops_changed = !w.stale.is_empty();
    let mut field = w.prog_field.take();
    let mut allow = w.allow_uncertified;
    let win = egui::Window::new("cam-programs")
        .id(egui::Id::unique("cam-programs-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v));
    // a tab of the properties column (docked) where the environment gives one
    let r = dialog_host(aux_rect(w.dock_aux), "programs", t, win)
        .show_placed(ctx, places, |ui, places| {
            // wrapped rows wrap at the column's width (a scroll area's content grows otherwise)
            ui.set_max_width(width);
            close = head(ui, t, "cam_sheet", "NC Programs", width, places, "cam:programs");
            if progs.is_empty() {
                note(ui, "No program yet: a new setup comes with one, or add one below.", width, t.ink2);
            }
            for (k, p) in progs.iter().enumerate() {
                program_row(ui, t, w, p, k, &setups, &posted, ops_changed, width, &mut field, &mut edits, &mut cmds, places);
            }
            // ops no program posts: never on the machine, so not certified
            let unposted: Vec<String> = w
                .run
                .as_ref()
                .map(|r| r.ops.iter().filter(|o| o.status == "unposted").map(|o| o.name.clone()).collect())
                .unwrap_or_default();
            if !unposted.is_empty() {
                note(
                    ui,
                    &format!(
                        "Posted by no program: {}. Tick it in a program's Ops, or add a New Program for it.",
                        unposted.join(", ")
                    ),
                    width,
                    t.warn,
                );
            }
            // every program: a footer of their own, under a rule
            ui.add_space(6.0);
            let (rr, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
            ui.painter().hline(rr.x_range(), rr.center().y, Stroke::new(1.0, t.line));
            heading(ui, t, "All programs", width);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                let r = ui.checkbox(&mut allow, "Allow uncertified").on_hover_text("Post a program that is not certified in full: its header says what was not checked");
                places.push(("cam:programs:allow".into(), r.rect));
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.add_space(14.0);
                if !setups.is_empty() && button(ui, t, "New Program", false, "cam:programs:new".into(), places) {
                    let n = super::doc::free_name(&w.stmts, &format!("prog{}", progs.len() + 1));
                    let num = progs.len() + if lathe { 1 } else { 1001 };
                    let (f, post) = if lathe { ("lathe_program", "fanuc-lathe") } else { ("program", "fanuc") };
                    edits.push(super::doc::Edit::Add {
                        before: None,
                        text: format!("{n} = {f}({}, post: \"{post}\", number: {num})", setups[0]),
                    });
                }
                let any = w.run.is_some() && !progs.is_empty();
                if any && button(ui, t, "Post All", true, "cam:programs:write".into(), places) {
                    cmds.push(Cmd::PostNc(None));
                }
                if posted.iter().any(|p| p.1 == Posted::Stale)
                    && button(ui, t, "Re-post Stale", false, "cam:programs:repost".into(), places)
                {
                    cmds.push(Cmd::RepostStale);
                }
                if any && button(ui, t, "Export All…", false, "cam:programs:export".into(), places) {
                    cmds.push(Cmd::ExportNc(None));
                }
                // the column's width: the exports on a line of their own
                if width < 520.0 {
                    ui.end_row();
                    ui.add_space(14.0);
                }
                if w.run.is_some() && button(ui, t, "Setup Sheet & Tools…", false, "cam:programs:sheet".into(), places) {
                    cmds.push(Cmd::ExportSheet);
                }
            });
            if let Some(s) = &w.said {
                note(ui, s, width, t.ink2);
            }
            // one line; the rest on hover
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new("Post writes beside the document; Export\u{2026} saves a copy.").font(sans(12.0)).color(t.ink2))
                    .on_hover_text("Post writes a program's file beside the document (its File, Folder and Extension); Export\u{2026} saves a copy where you choose (several in a folder, or a zip in the browser).");
            });
            ui.add_space(8.0);
        });
    w.prog_field = field;
    w.allow_uncertified = allow;
    if !edits.is_empty()
        && let Err(e) = w.edits(&edits)
    {
        w.said = Some(e);
    }
    for c in cmds {
        w.command(c);
    }
    if close {
        w.programs_open = false;
    }
    r.map(|r| r.response.rect)
}

/// One program in the NC Programs window.
#[allow(clippy::too_many_arguments)]
fn program_row(
    ui: &mut Ui,
    t: &Theme,
    w: &CamWs,
    p: &super::doc::Stmt,
    k: usize,
    setups: &[String],
    posted: &[super::nc::PostedRow],
    ops_changed: bool,
    width: f32,
    field: &mut Option<(String, String)>,
    edits: &mut Vec<super::doc::Edit>,
    cmds: &mut Vec<Cmd>,
    places: &mut Places,
) {
    use super::nc::Posted;
    // the program's parts: itself, or one per setup when split (`NAME/SETUP`)
    let mine = |n: &str| n == p.name || n.split_once('/').is_some_and(|(x, _)| x == p.name);
    let res = w.run.as_ref().and_then(|r| r.program(&p.name));
    let states: Vec<&super::nc::PostedRow> = posted.iter().filter(|x| mine(&x.0)).collect();
    // the worst part's state stands for the program
    let rank = |s: Posted| match s {
        Posted::Refused => 0,
        Posted::Stale => 1,
        Posted::No => 2,
        Posted::Current => 3,
    };
    let state = states.iter().min_by_key(|x| rank(x.1)).copied();
    // the name, and its file's state as a painted mark
    let (r, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
    let ink = match state.map(|s| s.1) {
        Some(Posted::Current) => t.success,
        Some(Posted::Stale) => t.warn,
        Some(Posted::Refused) => t.error,
        _ => t.ink2,
    };
    ui.painter()
        .circle_filled(pos2(r.left() + 19.0, r.center().y), 4.0, ink);
    ui.painter().text(
        pos2(r.left() + 30.0, r.center().y),
        Align2::LEFT_CENTER,
        &p.name,
        medium(14.0),
        t.ink,
    );
    let words = match state {
        Some((_, s, _)) if states.len() > 1 => format!("{} files: {}", states.len(), s.words()),
        Some((_, s, path)) if !path.is_empty() => format!("{path}: {}", s.words()),
        Some((_, s, _)) => s.words().to_string(),
        None => "not generated".into(),
    };
    ui.painter().text(
        pos2(r.right() - 14.0, r.center().y),
        Align2::RIGHT_CENTER,
        words,
        sans(12.0),
        ink,
    );
    places.push((format!("cam:programs:{}:state", p.name), r));
    // its setups: a check each (the first positional argument: one name or a list)
    let now = p.pos(0).unwrap_or("").to_string();
    let had: Vec<String> = now
        .trim_matches(|c| c == '[' || c == ']' || c == '(' || c == ')')
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();
    let mut want = had.clone();
    ui.horizontal_wrapped(|ui| {
        ui.add_space(14.0);
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(egui::RichText::new("Setups").font(sans(13.0)).color(t.ink2));
        for su in setups {
            let mut on = want.contains(su);
            let r = ui.checkbox(&mut on, su.as_str());
            places.push((format!("cam:programs:{}:{su}", p.name), r.rect));
            if on && !want.contains(su) {
                want.push(su.clone());
            }
            if !on {
                want.retain(|x| x != su);
            }
        }
    });
    if want != had && !want.is_empty() {
        let ordered: Vec<String> = setups.iter().filter(|s| want.contains(s)).cloned().collect();
        let text = if ordered.len() == 1 {
            ordered[0].clone()
        } else {
            format!("[{}]", ordered.join(", "))
        };
        // a program reads its setups: below the last of them first
        let at = |n: &str| w.stmts.iter().position(|x| x.name == n);
        if ordered.iter().any(|su| at(su) > at(&p.name)) {
            edits.push(super::doc::Edit::Move {
                id: p.name.clone(),
                before: None,
            });
        }
        edits.push(super::doc::Edit::Set {
            id: p.name.clone(),
            arg: "#0".into(),
            text,
        });
    }
    // its ops: every op of its setups unless `ops:` names some (posted in the document's order)
    let ops: Vec<&super::doc::Stmt> = w
        .stmts
        .iter()
        .filter(|o| o.is_op() && o.setup.as_ref().is_some_and(|su| had.contains(su)))
        .collect();
    if ops.len() > 1 {
        let named: Option<Vec<String>> = p.arg("ops").map(|t| {
            t.trim_matches(|c| c == '[' || c == ']' || c == '(' || c == ')')
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        });
        let on_now = |n: &str| named.as_ref().is_none_or(|v| v.iter().any(|x| x == n));
        let mut pick: Vec<String> = ops
            .iter()
            .filter(|o| on_now(&o.name))
            .map(|o| o.name.clone())
            .collect();
        ui.horizontal_wrapped(|ui| {
            ui.add_space(14.0);
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.label(egui::RichText::new("Ops").font(sans(13.0)).color(t.ink2));
            for o in &ops {
                let mut on = pick.contains(&o.name);
                let r = ui.checkbox(&mut on, o.name.as_str());
                places.push((format!("cam:programs:{}:op:{}", p.name, o.name), r.rect));
                if on != pick.contains(&o.name) {
                    if on {
                        pick.push(o.name.clone());
                    } else {
                        pick.retain(|x| x != &o.name);
                    }
                }
            }
        });
        let was: Vec<String> = ops
            .iter()
            .filter(|o| on_now(&o.name))
            .map(|o| o.name.clone())
            .collect();
        if pick.len() != was.len() && !pick.is_empty() {
            let ordered: Vec<String> = ops
                .iter()
                .filter(|o| pick.contains(&o.name))
                .map(|o| o.name.clone())
                .collect();
            if ordered.len() == ops.len() {
                edits.push(super::doc::Edit::Unset {
                    id: p.name.clone(),
                    arg: "ops".into(),
                });
            } else {
                // a program reads its ops: below the last of them first
                let at = |n: &str| w.stmts.iter().position(|x| x.name == n);
                if ordered.iter().any(|o| at(o) > at(&p.name)) {
                    edits.push(super::doc::Edit::Move {
                        id: p.name.clone(),
                        before: None,
                    });
                }
                edits.push(super::doc::Edit::Set {
                    id: p.name.clone(),
                    arg: "ops".into(),
                    text: format!("[{}]", ordered.join(", ")),
                });
            }
        }
    }
    let line = format!(
        "{} · O{} · {}",
        p.arg_str("post").unwrap_or_else(|| "the machine's".into()),
        p.arg("number").unwrap_or("?"),
        match &res {
            Some(r) if r.text.is_some() => format!("{} lines, certified {}", r.lines, r.certified),
            Some(r) => format!("not released: {}", r.error.clone().unwrap_or_default()),
            None => "not generated".into(),
        }
    );
    note(
        ui,
        &line,
        width,
        if res.as_ref().is_some_and(|r| r.text.is_some()) {
            t.ink
        } else {
            t.warn
        },
    );
    if ops_changed && state.is_some_and(|s| s.1 == Posted::Current) {
        note(
            ui,
            "Ops changed since the run: generate, then re-post.",
            width,
            t.warn,
        );
    }
    // its file: name, folder, extension, header
    let ext_default = forge_cam_lang::export::ext_of(&p.arg_str("post").unwrap_or_default()).to_string();
    // its file and posting, one field a row (UX-LEAD's sheet pattern)
    prog_field(ui, t, "File", width, p, "file", &p.name, field, edits, places);
    prog_field(
        ui,
        t,
        "Folder",
        width,
        p,
        "out",
        "beside the document",
        field,
        edits,
        places,
    );
    prog_field(
        ui,
        t,
        "Extension",
        width,
        p,
        "ext",
        &ext_default,
        field,
        edits,
        places,
    );
    prog_field(
        ui,
        t,
        "Comment",
        width,
        p,
        "comment",
        "a header line",
        field,
        edits,
        places,
    );
    prog_field(
        ui,
        t,
        "Decimals",
        width,
        p,
        "decimals",
        "the post's",
        field,
        edits,
        places,
    );
    prog_field(
        ui,
        t,
        "Feed decimals",
        width,
        p,
        "feed_decimals",
        "the post's",
        field,
        edits,
        places,
    );
    let now = p.arg_str("coolant").unwrap_or_default();
    let mut pick = now.clone();
    row(ui, t, "Coolant", width, 30.0, |ui| {
        let r = egui::ComboBox::from_id_salt(format!("cam-programs-{}-coolant", p.name))
            .width(ui.available_width() - 6.0)
            .selected_text(if pick.is_empty() {
                "the post's"
            } else {
                pick.as_str()
            })
            .show_ui(ui, |ui| {
                for c in ["", "flood", "mist", "through", "off"] {
                    ui.selectable_value(
                        &mut pick,
                        c.to_string(),
                        if c.is_empty() { "the post's" } else { c },
                    );
                }
            });
        places.push((format!("cam:programs:{}:coolant", p.name), r.response.rect));
    });
    if pick != now {
        edits.push(if pick.is_empty() {
            super::doc::Edit::Unset {
                id: p.name.clone(),
                arg: "coolant".into(),
            }
        } else {
            super::doc::Edit::Set {
                id: p.name.clone(),
                arg: "coolant".into(),
                text: format!("\"{pick}\""),
            }
        });
    }
    // the checks, a row each
    for (arg, words, on_text) in [
        ("split", "One file per setup", "\"setup\""),
        ("tools", "Tool list in the header", "\"list\""),
    ] {
        let had = match arg {
            "split" => p.arg_str("split").as_deref() == Some("setup"),
            _ => p.arg_str("tools").as_deref() == Some("list"),
        };
        let mut on = had;
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            let r = ui.checkbox(&mut on, words);
            places.push((format!("cam:programs:{}:{arg}", p.name), r.rect));
        });
        if on != had {
            edits.push(if on {
                super::doc::Edit::Set {
                    id: p.name.clone(),
                    arg: arg.into(),
                    text: on_text.into(),
                }
            } else {
                super::doc::Edit::Unset {
                    id: p.name.clone(),
                    arg: arg.into(),
                }
            });
        }
    }
    // the program's own actions, together
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(14.0);
        let ok = res.as_ref().is_some_and(|r| r.text.is_some());
        if ok
            && button(
                ui,
                t,
                "Post",
                true,
                format!("cam:programs:{}:post", p.name),
                places,
            )
        {
            cmds.push(Cmd::PostNc(Some(p.name.clone())));
        }
        if ok
            && button(
                ui,
                t,
                "Export\u{2026}",
                false,
                format!("cam:programs:{}:export", p.name),
                places,
            )
        {
            cmds.push(Cmd::ExportNc(Some(p.name.clone())));
        }
        if button(
            ui,
            t,
            "Post Process",
            false,
            format!("cam:programs:{}:open", p.name),
            places,
        ) {
            cmds.push(Cmd::PostAt(k));
        }
    });
    ui.add_space(6.0);
}

/// The program arguments that are numbers (written unquoted).
const NUMBER_ARGS: &[&str] = &["decimals", "feed_decimals"];

/// A program's export setting `arg` as a short field: the statement's value, written when the
/// field is left (empty: the default, the argument taken out).
#[allow(clippy::too_many_arguments)]
fn prog_field(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    width: f32,
    p: &super::doc::Stmt,
    arg: &str,
    hint: &str,
    field: &mut Option<(String, String)>,
    edits: &mut Vec<super::doc::Edit>,
    places: &mut Places,
) {
    let key = format!("cam:programs:{}:{arg}", p.name);
    let now = p.arg_str(arg).unwrap_or_default();
    let mut v = match field {
        Some((k, b)) if *k == key => b.clone(),
        _ => now.clone(),
    };
    // a sheet row: the label in its column, the field filling the rest
    let r = row(ui, t, label, width, 30.0, |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut v)
                .font(sans(13.0))
                .hint_text(hint)
                .desired_width(ui.available_width() - 6.0)
                .margin(vec2(7.0, 5.0)),
        )
    });
    places.push((key.clone(), r.rect));
    if r.changed() {
        *field = Some((key.clone(), v.clone()));
    }
    if r.lost_focus() && field.as_ref().is_some_and(|f| f.0 == key) {
        *field = None;
        let v = v.trim().replace(['"', '\\'], "");
        if v != now {
            edits.push(if v.is_empty() {
                super::doc::Edit::Unset {
                    id: p.name.clone(),
                    arg: arg.into(),
                }
            } else {
                super::doc::Edit::Set {
                    id: p.name.clone(),
                    arg: arg.into(),
                    // counts are numbers; names and folders are strings
                    text: if NUMBER_ARGS.contains(&arg) {
                        v.clone()
                    } else {
                        format!("\"{v}\"")
                    },
                }
            });
        }
    }
}

/// Every op's toolpath quality in one table, each setup's totals, and the programs' time on the
/// machine's timeline (forge-cam-quality).
fn quality_window(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    places: &mut Places,
) -> Option<Rect> {
    let mut close = false;
    // within the view's free part (a small screen narrows it)
    let width = match w.dock_aux {
        Some(d) => d.width() - 16.0,
        None => 720.0_f32.min(v.width() - 32.0).max(360.0),
    };
    let mut select: Option<String> = None;
    let mut colour = w.focus.colour_by;
    let win = egui::Window::new("cam-quality")
        .id(egui::Id::unique("cam-quality-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v));
    let r = dialog_host(aux_rect(w.dock_aux), "quality", t, win)
        .show_placed(ctx, places, |ui, places| {
            ui.set_max_width(width);
            close = head(ui, t, "cam_check", "Toolpath Quality", width, places, "cam:quality");
            // the toolpath's colouring in the view (the simulation view draws it)
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new("Colour toolpath by").font(sans(13.0)).color(t.ink2));
                use super::ColourBy;
                // chip load and engagement come from the quality measure: none to colour by
                // while a milling run's measure is on its way
                let measuring = w.run.as_ref().is_some_and(|r| r.cam.is_some() && r.quality.is_none());
                for (label, c, key) in [
                    ("Op", None, "op"),
                    ("Chip load", Some(ColourBy::ChipLoad), "chip"),
                    ("Engagement", Some(ColourBy::Engagement), "engagement"),
                ] {
                    let live = c.is_none() || !measuring;
                    let r = ui
                        .add_enabled(live, egui::Button::selectable(colour == c, label))
                        .on_disabled_hover_text("Measuring quality");
                    places.push((format!("cam:quality:colour:{key}"), r.rect));
                    if r.clicked() {
                        colour = c;
                    }
                }
            });
            let Some(run) = &w.run else { return };
            let Some(job) = &run.quality else {
                if run.cam.is_some() {
                    // a milling run is measured after it is shown (the worker hands the
                    // measure over when it lands, and wakes the frame)
                    ui.add_space(6.0);
                    let r = ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        crate::shell::ux::loading(ui, t, "Measuring quality");
                    });
                    places.push(("cam:quality:measuring".into(), r.response.rect));
                    ui.add_space(6.0);
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                } else {
                    let top = ui.cursor().min;
                    note(ui, "Not measured (turning runs are not yet).", width, t.ink2);
                    places.push((
                        "cam:quality:not-measured".into(),
                        Rect::from_min_max(top, pos2(top.x + width, ui.cursor().min.y)),
                    ));
                }
                return;
            };
            let cols: [(&str, f32); 10] = [
                ("op", 110.0),
                ("feed time", 70.0),
                ("air", 50.0),
                ("plunges", 56.0),
                ("eng. max", 60.0),
                ("slot", 48.0),
                ("chip mean", 70.0),
                ("rub / over", 80.0),
                ("MRR cm3/min", 86.0),
                ("kW", 50.0),
            ];
            // the table at its own width: sideways in a scroll where the column is narrower
            let tw = width.max(28.0 + cols.iter().map(|c| c.1).sum::<f32>());
            let header = |ui: &mut Ui| {
                let (r, _) = ui.allocate_exact_size(vec2(tw, 22.0), Sense::hover());
                let mut x = r.left() + 14.0;
                for (c, wd) in cols {
                    ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, c, sans(11.0), t.ink2);
                    x += wd;
                }
            };
            egui::ScrollArea::horizontal().id_salt("cam-quality-table").show(ui, |ui| {
            for su in w.setups() {
                let ops: Vec<&forge_cam_quality::OpQ> = job
                    .ops
                    .iter()
                    .filter(|q| run.op(&q.name).is_some_and(|o| o.setup == su.name))
                    .collect();
                if ops.is_empty() {
                    continue;
                }
                let feed: f64 = ops.iter().map(|q| q.t_cut + q.t_air).sum();
                let air: f64 = ops.iter().map(|q| q.t_air).sum();
                heading(
                    ui,
                    t,
                    &format!("{} · feed time {} · {} in air", su.name, time(feed), if feed > 0.0 { pct(air / feed) } else { "—".into() }),
                    width,
                );
                header(ui);
                for q in ops {
                    let (r, resp) = ui.allocate_exact_size(vec2(tw, 22.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 0.0, t.hover);
                    }
                    if resp.clicked() {
                        select = Some(q.name.clone());
                    }
                    places.push((format!("cam:quality:row:{}", q.name), r));
                    let side = |s: String| if q.side { s } else { "—".into() };
                    let cells = [
                        q.name.clone(),
                        time(q.t_cut + q.t_air),
                        if q.t_cut + q.t_air > 0.0 { pct(q.t_air / (q.t_cut + q.t_air)) } else { "—".into() },
                        q.plunges.to_string(),
                        side(format!("{}°", num(q.phi_max.round()))),
                        side(pct(q.slotting)),
                        side(num(q.hex_mean)),
                        side(format!("{} / {}", pct(q.thin), pct(q.thick))),
                        if q.mrr > 0.0 { num((q.mrr / 1000.0 * 10.0).round() / 10.0) } else { "—".into() },
                        if q.p_max > 0.0 { num((q.p_max * 100.0).round() / 100.0) } else { "—".into() },
                    ];
                    let warn = [
                        false,
                        false,
                        q.t_cut + q.t_air > 0.0 && q.t_air / (q.t_cut + q.t_air) > 0.3,
                        q.plunges > 0,
                        q.side && q.phi_max > 170.0,
                        q.side && q.slotting > 0.05,
                        false,
                        q.side && (q.thin > 0.1 || q.thick > 0.1),
                        false,
                        q.p_avail > 0.0 && q.p_max > q.p_avail,
                    ];
                    let mut x = r.left() + 14.0;
                    for (ci, ((c, (_, wd)), wn)) in cells.iter().zip(cols).zip(warn).enumerate() {
                        // the op's name as the tree shows it; the numbers in mono
                        let font = if ci == 0 { sans(12.0) } else { mono(11.0) };
                        ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, c, font, if wn { t.warn } else { t.ink });
                        x += wd;
                    }
                }
            }
            });
            if !job.progs.is_empty() {
                heading(ui, t, "Programs on the machine's timeline", width);
                for p in &job.progs {
                    let words = match &p.error {
                        Some(e) => format!("not timed: {e}"),
                        None => format!(
                            "{} · {} tool changes{}",
                            cycle_words((p.t_min, p.t_scurve.max(p.t_min))),
                            p.tool_changes,
                            if p.accel_missing { " · the machine gives no accelerations" } else { "" }
                        ),
                    };
                    fact_tip(ui, t, &format!("{} on {}", p.name, p.machine), &words, CYCLE_TIP, width, if p.error.is_some() { t.warn } else { t.ink });
                }
            }
            note(
                ui,
                &format!(
                    "Measured on a {} mm height map: rubbing is a chip under half the feed per tooth, overloaded over 1.25 times it. Amber: worth a look.",
                    num(job.cell)
                ),
                width,
                t.ink2,
            );
            ui.add_space(8.0);
        });
    if let Some(op) = select {
        w.command(Cmd::Select(op));
    }
    w.focus.colour_by = colour;
    if close {
        w.quality_open = false;
    }
    r.map(|r| r.response.rect)
}

fn checks_window(
    ctx: &egui::Context,
    t: &Theme,
    v: Rect,
    w: &mut CamWs,
    places: &mut Places,
) -> Option<Rect> {
    let mut close = false;
    // within the view's free part (a small screen narrows it)
    let width = 460.0_f32.min(v.width() - 32.0).max(360.0);
    let mut locate: Option<Locate> = None;
    let r = egui::Window::new("cam-checks")
        .id(egui::Id::unique("cam-checks-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.right() - width - 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v))
        .show_placed(ctx, places, |ui, places| {
            close = head(ui, t, "cam_check", "Checks", width, places, "cam:checks");
            let Some(run) = &w.run else {
                note(ui, "Generate first.", width, t.ink2);
                return;
            };
            let (word, c) = match cert_of(run.ok, &run.certification) {
                Cert::Certified => ("Every op certified, every program released".to_string(), t.success),
                Cert::Unchecked(g) => (unchecked_words(g), t.ink2),
                Cert::Refused => ("Refused: an op or program below".to_string(), t.error),
            };
            note(ui, &word, width, c);
            note(ui, "Rules: gouge, holder_collision, rapid_clear, fixture_collision, axis_limit (G5), leftover (G4), on the posted path.", width, t.ink2);
            let mut k = 0;
            for o in &run.ops {
                let (_, wd, c) = status_look(t, &o.status);
                fact(ui, t, &format!("{} · {}", o.name, o.kind), wd, width, c);
                for ch in &o.checks {
                    if check_row(ui, t, ch, width, &format!("cam:checks:{k}"), places) {
                        locate = Some(Locate { op: o.name.clone(), mov: ch.mov, at: ch.at, rule: ch.rule.clone() });
                    }
                    k += 1;
                }
            }
            if !run.leftover.is_empty() {
                heading(ui, t, "Left over (G4)", width);
                for l in &run.leftover {
                    fact(ui, t, &format!("{} z {}…{}", l.setup, num(l.z[0]), num(l.z[1])), &format!("{} · {}", l.boxes.len(), l.why), width, t.warn);
                }
            }
            for wn in &run.warnings {
                note(ui, wn, width, t.warn);
            }
            ui.add_space(8.0);
        });
    if let Some(l) = locate {
        w.command(Cmd::Locate(l));
    }
    if close {
        w.checks_open = false;
    }
    r.map(|r| r.response.rect)
}

fn text_window(ctx: &egui::Context, t: &Theme, v: Rect, w: &mut CamWs, places: &mut Places) -> Option<Rect> {
    let mut close = false;
    let mut apply = false;
    // a panel across the bottom of the view, above the simulation's timeline (UX-LEAD)
    let ph = text_panel_h(v.height());
    let panel = Rect::from_min_max(
        pos2(v.left() + 8.0, v.bottom() - ph),
        pos2(v.right() - 8.0, v.bottom() - 4.0),
    );
    let width = panel.width() - 16.0;
    let win = egui::Window::new("cam-text")
        .id(egui::Id::unique("cam-text-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .fade_in(false)
        .constrain_to(below_chips(v));
    let r = dialog_host(aux_rect(Some(panel)), "text", t, win).show_placed(ctx, places, |ui, places| {
        ui.set_max_width(width);
        close = head(
            ui,
            t,
            "cam_post",
            &format!("{} · text", w.path.rsplit('/').next().unwrap_or("")),
            width,
            places,
            "cam:text",
        );
        if w.text_edit != w.text && !ui.memory(|m| m.has_focus(egui::Id::unique("cam-text-edit"))) {
            w.text_edit = w.text.clone();
        }
        egui::ScrollArea::vertical()
            .id_salt("cam-text-scroll")
            .max_height(ph - 110.0)
            .show(ui, |ui| {
                let r = ui.add(
                    egui::TextEdit::multiline(&mut w.text_edit)
                        .id(egui::Id::unique("cam-text-edit"))
                        .font(mono(12.0))
                        .code_editor()
                        .desired_width(width - 28.0)
                        .desired_rows(8),
                );
                // what is seen of it (a long text runs past the scroll's viewport, where a
                // click is clipped): a click by name lands on the field
                places.push(("cam:text:edit".into(), r.rect.intersect(ui.clip_rect())));
            });
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            apply = button(ui, t, "Apply", true, "cam:text:apply".into(), places);
        });
        ui.add_space(8.0);
    });
    if apply {
        let s = w.text_edit.clone();
        w.command(Cmd::Text(s));
    }
    if close {
        w.text_open = false;
    }
    r.map(|r| r.response.rect)
}

/// The status bar's words for the workspace.
pub fn status_line(w: &CamWs) -> String {
    let n = w.stmts.iter().filter(|s| s.is_op() && !s.suppressed).count();
    let built = w
        .stmts
        .iter()
        .filter(|s| s.is_op() && w.status(&s.name) == "built")
        .count();
    let time: f64 = w
        .run
        .as_ref()
        .map_or(0.0, |r| r.ops.iter().map(|o| o.time_s).sum());
    let cycle = w
        .run
        .as_ref()
        .and_then(|r| r.cycle(r.ops.iter().map(|o| o.name.as_str())));
    let machining = if let Some(c) = cycle {
        format!(" · {} machining", cycle_words(c))
    } else if time > 0.0 {
        format!(" · {} feed time", self::time(time))
    } else {
        String::new()
    };
    // a milling run's quality is measured after it is shown
    let measuring = w
        .run
        .as_ref()
        .is_some_and(|r| r.cam.is_some() && r.quality.is_none());
    format!(
        "{} · {built}/{n} ops certified{machining}{}{}",
        w.path.rsplit('/').next().unwrap_or(""),
        if w.dirty() { " · unsaved" } else { "" },
        if measuring { " · measuring quality" } else { "" }
    )
}

/// A tool kind in words.
fn kind_words(k: forge_tooling::Kind) -> &'static str {
    use forge_tooling::Kind::*;
    match k {
        Flat => "Flat end mill",
        Ball => "Ball end mill",
        Bull => "Bull nose end mill",
        Chamfer => "Chamfer mill",
        Spot => "Spot drill",
        Drill => "Drill",
        Tap => "Tap",
        ThreadMill => "Thread mill",
        Reamer => "Reamer",
        Bore => "Boring bar",
        Counterbore => "Counterbore",
        Countersink => "Countersink",
        Slot => "Slot cutter",
        Engrave => "Engraver",
    }
}

/// A whole number with thin groups of three (15 000).
fn grouped(v: f64) -> String {
    let n = v.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push('\u{2009}');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// A tool's side silhouette in `r` (shank up, its tip down): flat, ball, point or V.
fn tool_glyph(p: &egui::Painter, r: Rect, k: forge_tooling::Kind, ink: Color32) {
    use forge_tooling::Kind::*;
    let st = Stroke::new(1.2, ink);
    let (l, rt, top, bot) = (r.left(), r.right(), r.top(), r.bottom());
    let tip = bot - r.width() * 0.5;
    // the shank and the flutes' sides
    p.line_segment([pos2(l, top), pos2(l, tip)], st);
    p.line_segment([pos2(rt, top), pos2(rt, tip)], st);
    p.line_segment([pos2(l, top), pos2(rt, top)], st);
    match k {
        Ball => {
            let c = pos2(r.center().x, tip);
            let n = 8;
            let pts: Vec<egui::Pos2> = (0..=n)
                .map(|i| {
                    let a = std::f32::consts::PI * i as f32 / n as f32;
                    pos2(c.x - a.cos() * r.width() * 0.5, c.y + a.sin() * r.width() * 0.5)
                })
                .collect();
            p.add(egui::Shape::line(pts, st));
        }
        Drill | Spot | Chamfer | Countersink | Engrave => {
            p.line_segment([pos2(l, tip), pos2(r.center().x, bot)], st);
            p.line_segment([pos2(rt, tip), pos2(r.center().x, bot)], st);
        }
        _ => {
            p.line_segment([pos2(l, tip), pos2(l, bot)], st);
            p.line_segment([pos2(rt, tip), pos2(rt, bot)], st);
            p.line_segment([pos2(l, bot), pos2(rt, bot)], st);
        }
    }
    // flutes
    p.line_segment(
        [pos2(l, (top + tip) * 0.5), pos2(rt, (top + tip) * 0.5 + 3.0)],
        Stroke::new(0.8, ink),
    );
}

/// A document tool as a machinist reads it: "T1 · Flat end mill Ø10" (the library's kind and
/// diameter; the id when the library has no such tool).
fn tool_words(w: &CamWs, s: &super::doc::Stmt) -> String {
    let id = s.pos(0).map(super::doc::unquote).unwrap_or_default();
    let t = s.arg("t").unwrap_or("?");
    match w.lib.tools.iter().find(|x| x.id == id) {
        Some(x) => format!("T{t} · {} Ø{}", kind_words(x.kind), num(x.dc)),
        None => format!("T{t} · {id}"),
    }
}

/// A machine's id in words: "generic-vmc3" → "Generic 3-axis VMC".
fn machine_words(id: &str) -> String {
    let words: Vec<String> = id
        .split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| match w {
            "vmc3" => "3-axis VMC".to_string(),
            "vmc4" => "4-axis VMC".to_string(),
            "vmc5" => "5-axis VMC".to_string(),
            "hmc" | "vmc" | "cnc" => w.to_uppercase(),
            w => {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            }
        })
        .collect();
    if words.is_empty() {
        id.to_string()
    } else {
        words.join(" ")
    }
}

/// A label and its value on one row, the value right-aligned; when the two do not fit side by
/// side the value takes its own row under the label, right-aligned and wrapped: a value never
/// draws over its label (docs/ux/STANDARD.md §5).
fn fact(ui: &mut Ui, t: &Theme, k: &str, v: &str, w: f32, ink: Color32) {
    fact_r(ui, t, k, v, w, ink);
}

/// `fact`, its response (for a hover).
fn fact_r(ui: &mut Ui, t: &Theme, k: &str, v: &str, w: f32, ink: Color32) -> egui::Response {
    let pad = 14.0;
    let kg = ui.painter().layout_no_wrap(k.to_string(), sans(12.0), t.ink2);
    let vg = ui.painter().layout_no_wrap(v.to_string(), mono(12.0), ink);
    if kg.size().x + vg.size().x + 16.0 <= w - 2.0 * pad {
        let (r, resp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
        ui.painter()
            .galley(pos2(r.left() + pad, r.center().y - kg.size().y * 0.5), kg, t.ink2);
        ui.painter().galley(
            pos2(r.right() - pad - vg.size().x, r.center().y - vg.size().y * 0.5),
            vg,
            ink,
        );
        return resp;
    }
    // the label, then the value under it (wrapped to the row)
    let vg = ui
        .painter()
        .layout(v.to_string(), mono(12.0), ink, w - 2.0 * pad - 12.0);
    let h = 20.0 + vg.size().y + 2.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().galley(
        pos2(r.left() + pad, r.top() + 10.0 - kg.size().y * 0.5),
        kg,
        t.ink2,
    );
    ui.painter()
        .galley(pos2(r.right() - pad - vg.size().x, r.top() + 20.0), vg, ink);
    resp
}

/// A fact whose words need a sentence: the row, and the sentence on hover.
fn fact_tip(ui: &mut Ui, t: &Theme, k: &str, v: &str, tip: &str, w: f32, ink: Color32) {
    fact_r(ui, t, k, v, w, ink).on_hover_text(tip);
}

/// The statement a dialog will write, as the document holds it: a muted "Forge text" label and
/// the line in the code font (the text is the source of truth; this is it, not a caption).
fn forge_text(ui: &mut Ui, t: &Theme, text: &str, w: f32) {
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 14.0, r.center().y),
        Align2::LEFT_CENTER,
        "Forge text",
        sans(11.0),
        t.ink2,
    );
    let g = ui.painter().layout(text.to_string(), mono(12.0), t.ink, w - 36.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 10.0), Sense::hover());
    let field = egui::Rect::from_min_max(pos2(r.left() + 12.0, r.top()), pos2(r.right() - 12.0, r.bottom()));
    ui.painter().rect_filled(field, 4.0, t.field);
    ui.painter()
        .galley(pos2(field.left() + 6.0, field.top() + 5.0), g, t.ink);
}

/// The part of the view a floating window keeps inside: below the view's chip row (Axes, Grid,
/// Planes, Anchors), 4 px in from the other edges.
fn below_chips(v: Rect) -> Rect {
    Rect::from_min_max(
        pos2(v.left() + 4.0, v.top() + 40.0),
        pos2(v.right() - 4.0, v.bottom() - 4.0),
    )
}

/// An op or setup dialog's host: docked in the Manufacture environment's properties column when
/// it has one (`CamWs::dock`, set by the environment each frame), else floating over the view.
fn dialog_host<'a>(dock: Option<Rect>, id: &str, t: &Theme, win: egui::Window<'a>) -> Host<'a> {
    match dock {
        Some(r) => Host::Dock(
            egui::Id::unique(("cam-dock", id)),
            Rect::from_min_max(r.min, pos2(r.right(), r.bottom() - FOOT_H)),
            frame(t),
        ),
        None => Host::Float(Box::new(win)),
    }
}

/// The Document Text panel's height in a view (above the timeline) `free_h` tall: the view is
/// re-framed into what it leaves (`Gui::cam_reframe`).
pub fn text_panel_h(free_h: f32) -> f32 {
    (free_h * 0.38).clamp(220.0, 380.0)
}

/// A column tab's rect as `dialog_host` takes it (it keeps a sheet's footer height free at the
/// bottom; a tab has no footer).
fn aux_rect(r: Option<Rect>) -> Option<Rect> {
    r.map(|r| Rect::from_min_max(r.min, pos2(r.right(), r.bottom() + FOOT_H)))
}

/// A docked sheet's footer height: its OK, Cancel and Apply.
const FOOT_H: f32 = 48.0;

/// A docked sheet's commit buttons, pinned at the bottom of its rect (outside the body's scroll).
fn docked_foot(
    ctx: &egui::Context,
    t: &Theme,
    dock: Rect,
    key: &str,
    apply: bool,
    places: &mut Places,
) -> (bool, bool, bool) {
    let r = Rect::from_min_max(pos2(dock.left(), dock.bottom() - FOOT_H), dock.max);
    egui::Area::new(egui::Id::unique(("cam-dock-foot", key)))
        .fixed_pos(r.min)
        .order(egui::Order::Middle)
        .fade_in(false)
        .show(ctx, |ui| {
            ui.set_width(r.width());
            ui.painter().rect_filled(r, 0.0, t.surface);
            ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, t.line));
            ui.add_space(2.0);
            foot(ui, t, key, apply, places)
        })
        .inner
}

/// A run's certification as the Setups header and the Checks panel read it: refused only when
/// the run is not ok (an op, a program or a statement refused); otherwise certified, naming a
/// rule the run leaves undecided (forge-cam-lang's `Cam::certification`: "full", "except G5",
/// "full except G4 (leftover not computed)", "except G4 and G5 (..)").
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Cert {
    Certified,
    Unchecked(&'static str),
    Refused,
}

pub(crate) fn cert_of(ok: bool, certification: &str) -> Cert {
    if !ok || certification == "refused" {
        return Cert::Refused;
    }
    match certification {
        "full" => Cert::Certified,
        "except G5" => Cert::Unchecked("G5"),
        c if c.starts_with("full except G4") => Cert::Unchecked("G4"),
        c if c.starts_with("except G4 and G5") => Cert::Unchecked("G4 and G5"),
        // words this build does not know: not called a refusal, not called full
        _ => Cert::Unchecked("a rule"),
    }
}

/// A certified run's words as the simulation's header says them ("Certified", or "Certified ·
/// leftover not checked" with the hover's explanation), the same as the Setups chip.
pub fn certified_words(certification: &str) -> (String, Option<String>) {
    match cert_of(true, certification) {
        Cert::Unchecked(g) => (
            format!("Certified \u{b7} {}", unchecked_chip(g)),
            Some(unchecked_words(g)),
        ),
        _ => ("Certified".to_string(), None),
    }
}

/// The header chip's words for an unchecked rule: what it means, never the rule's code (UX-LEAD).
fn unchecked_chip(g: &str) -> &'static str {
    match g {
        "G4" => "leftover not checked",
        "G5" => "travel checked by span",
        "G4 and G5" => "2 checks partial",
        _ => "a check partial",
    }
}

/// What an unchecked rule means, for the hover and the Checks panel: the rule's code first.
fn unchecked_words(g: &str) -> String {
    let g4 = "G4: the leftover (the stock left against the part) is not computed in this run.";
    let g5 = "G5: machine travel is checked against each axis's span only, because this setup isn't \
              placed on the machine. Set its position (at:) in the setup dialog to check positions too.";
    match g {
        "G4" => format!("{g4}\nEvery op is certified and every program released."),
        "G5" => format!("{g5}\nEvery op is certified."),
        "G4 and G5" => format!("{g4}\n{g5}\nEvery op is certified."),
        _ => "A check is not decided in this run. Every op is certified.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Cert, cert_of, choice_words, default_value, unchecked_chip, unchecked_words};

    /// A run whose ops and programs all certify reads certified, naming what it leaves
    /// unchecked (mold_core_cam: "full except G4 (leftover not computed)" read "refused"); only a
    /// run that is not ok reads refused.
    #[test]
    fn a_certified_run_with_a_rule_unchecked_is_not_refused() {
        assert_eq!(cert_of(true, "full"), Cert::Certified);
        assert_eq!(
            cert_of(true, "full except G4 (leftover not computed)"),
            Cert::Unchecked("G4")
        );
        assert_eq!(cert_of(true, "except G5"), Cert::Unchecked("G5"));
        assert_eq!(
            cert_of(true, "except G4 and G5 (leftover not computed; no `at:`)"),
            Cert::Unchecked("G4 and G5")
        );
        assert_eq!(cert_of(false, "refused"), Cert::Refused);
        assert_eq!(cert_of(false, "full"), Cert::Refused);
        assert_eq!(cert_of(true, "refused"), Cert::Refused);
        // the chip in words, the code first on the hover
        assert_eq!(unchecked_chip("G4"), "leftover not checked");
        assert_eq!(unchecked_chip("G5"), "travel checked by span");
        assert_eq!(unchecked_chip("G4 and G5"), "2 checks partial");
        assert!(unchecked_words("G4").starts_with("G4: "));
        assert!(unchecked_words("G5").starts_with("G5: machine travel is checked"));
    }

    /// The simulation's header says a certified run as the chip does: no raw certification
    /// string ("full except G4 (leftover not computed)"), the code on the hover's first line.
    #[test]
    fn the_simulations_header_reads_as_the_chip() {
        assert_eq!(super::certified_words("full"), ("Certified".to_string(), None));
        let (w, h) = super::certified_words("full except G4 (leftover not computed)");
        assert_eq!(w, "Certified \u{b7} leftover not checked");
        assert!(h.is_some_and(|h| h.starts_with("G4:")));
    }

    /// mold_core_cam end to end: its five ops and its program certify, G4 is not computed: the
    /// header reads "certified · G4 unchecked".
    #[test]
    fn mold_core_reads_certified_with_g4_unchecked() {
        let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
        let text = std::fs::read_to_string(dir.join("mold_core_cam.fg")).expect("the example");
        let r = crate::shell::cam::run::run(&text, &dir);
        assert!(r.ok, "{:?}", r.errors);
        assert!(r.ops.iter().all(|o| o.status == "built"));
        assert_eq!(
            cert_of(r.ok, &r.certification),
            Cert::Unchecked("G4"),
            "{}",
            r.certification
        );
    }

    #[test]
    fn defaults_read_as_their_values() {
        assert_eq!(default_value("half the tool's diameter", Some(10.0)), Some(5.0));
        assert_eq!(default_value("0.4 × the tool's diameter", Some(10.0)), Some(4.0));
        assert_eq!(default_value("half the tool's diameter", None), None);
        assert_eq!(default_value("0 (walls)", None), Some(0.0));
        assert_eq!(default_value("0 (from +x)", None), Some(0.0));
        assert_eq!(default_value("60", None), Some(60.0));
        assert_eq!(default_value("the stock's top", Some(10.0)), None);
        assert_eq!(choice_words("passes", "two_way"), "Both directions");
        assert_eq!(choice_words("side", "outside"), "Outside");
    }
}
