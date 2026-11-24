//! The Manufacture workspace's widgets: the operations browser and the selected op's results in
//! the side panel, the ribbon's groups, and the dialogs (setup, op, post, tool library, checks,
//! text). Every change is a `Cmd` or an edit of the document (mod.rs); nothing here keeps a
//! value the text does not.

use super::schema::{self, Kind, Tab};
use super::{Ask, CamWs, Cmd, Locate, OpDlg, SetupDlg};
use crate::shell::placed::Placed;
use crate::shell::theme::{Theme, medium, mono, sans};
use crate::shell::thermo_ui::{button, fact, foot, frame, head, heading, note, row, tabs};
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
                    w.run
                        .as_ref()
                        .is_some_and(|r| r.quality.is_some())
                        .then_some(Action::Cam(Ask::Quality)),
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
                ("Auto Generate", "cam_generate", a(Ask::AutoGenerate), w.auto),
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

/// A time as m:ss.
fn short_time(s: f64) -> String {
    format!("{}:{:02}", (s / 60.0).floor(), (s % 60.0).floor() as u32)
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
                    Some(run) if run.certification == "full" => ("certified".into(), t.success),
                    // not a failure: travel (G5) is checked by span only until a setup is placed
                    Some(run) if run.certification == "except G5" => ("certified · G5 unchecked".into(), t.ink2),
                    Some(_) => ("refused".into(), t.error),
                    None => ("not generated".into(), t.ink2),
                };
                if w.run.as_ref().is_some_and(|r| r.certification == "except G5") {
                    ui.interact(r, ui.scope_id().with("cam-cert"), Sense::hover()).on_hover_text(
                        "Every op is certified. Machine travel (G5) is checked against each axis's span only: \
                         a setup is not placed on the machine (`at:`, the work offset in machine coordinates). \
                         Place it in the setup dialog to check the positions too.",
                    );
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
                            if tsum > 0.0 {
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
                    let label = format!(
                        "{}  ·  {}  T{}",
                        s.name,
                        s.pos(0).map(super::doc::unquote).unwrap_or_default(),
                        s.arg("t").unwrap_or("?")
                    );
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
                    details(ui, &t, &w, &id, r.width(), &mut cmds, &mut places)
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
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Places = vec![];
        let mut blocks: Vec<Rect> = vec![];
        if let Some(mut d) = w.op_dlg.take() {
            let (keep, r) = op_dialog(ctx, &t, v, &mut w, &mut d, &mut places);
            blocks.extend(r);
            if keep {
                w.op_dlg = Some(d);
            }
        }
        if let Some(mut d) = w.setup_dlg.take() {
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
            blocks.extend(tools_window(ctx, &t, v, &mut w, &mut places));
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
                        "Left over: ρ {}…{}, z {}…{} ({})",
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
            fact(ui, t, "Machine", &su.machine, width, t.ink);
            fact(ui, t, "Work offset", &wcs_word(su.offset), width, t.ink);
            if let Some(r) = run {
                let ops: Vec<&super::run::OpRes> = r.ops.iter().filter(|o| o.setup == id).collect();
                let total: f64 = ops.iter().map(|o| o.time_s).sum();
                heading(ui, t, &format!("Cycle time {}", time(total)), width);
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
                    "Feeds and rapids at their programmed rates, tool changes not counted; the machine-timeline time comes with the quality measure.",
                    width,
                    t.ink2,
                );
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
                note(ui, &format!("Assurance: {}", o.assurance.words()), width, t.ink2);
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
                fact(ui, t, "Time", &time(o.time_s), width, t.ink);
            }
            if let Some(l) = o.least {
                fact(
                    ui,
                    t,
                    "Least cutter distance",
                    &format!("{} mm", num(l)),
                    width,
                    t.ink,
                );
                let tol = run.and_then(|r| r.setup(&o.setup)).map_or(0.0, |s| s.tol);
                if l.abs() <= tol + 1e-9 {
                    // a finishing cut touches the wall it finishes by design: a gouge is below −tol
                    note(
                        ui,
                        &format!(
                            "0 is the cutter on the wall it finishes, as a finishing pass is meant to be; a gouge would be under −{} mm (the setup's tol).",
                            num(tol)
                        ),
                        width,
                        t.ink2,
                    );
                }
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
    if q.t_stop > 0.0 {
        fact(ui, t, "Cycle (machine timeline)", &time(q.t_stop), width, t.ink);
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
    row(ui, t, label, DLG_W, 30.0, |ui| {
        let r = ui.add(
            egui::TextEdit::singleline(v)
                .font(sans(13.0))
                .hint_text(hint)
                .desired_width(ui.available_width() - 40.0)
                .margin(vec2(7.0, 5.0)),
        );
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
        .map(|s| {
            let id = s.pos(0).map(super::doc::unquote).unwrap_or_default();
            (
                s.name.clone(),
                format!("{} · T{} · {id}", s.name, s.arg("t").unwrap_or("?")),
            )
        })
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
    let r = egui::Window::new("cam-op")
        .id(egui::Id::unique("cam-op-dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(DLG_W, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
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
            for f in spec.fields.iter().filter(|f| f.tab == tab) {
                let key = format!("cam:op:arg:{}", f.arg);
                let mut val = d.get(f.arg).to_string();
                let hint = if f.required { "required".to_string() } else { format!("default: {}", f.default) };
                match f.kind {
                    Kind::Len => arg_field(ui, t, f.label, &mut val, &hint, "mm", &key, places),
                    Kind::Deg => arg_field(ui, t, f.label, &mut val, &hint, "deg", &key, places),
                    Kind::Z => arg_field(ui, t, f.label, &mut val, &hint, "mm z", &key, places),
                    Kind::Count => arg_field(ui, t, f.label, &mut val, &hint, "", &key, places),
                    Kind::Choice(words) => {
                        let mut items: Vec<(String, String)> = vec![];
                        if !f.required {
                            items.push((String::new(), format!("default ({})", f.default)));
                        }
                        items.extend(words.iter().map(|w| (format!("\"{w}\""), (*w).to_string())));
                        arg_choice(ui, t, f.label, &mut val, &items, &key, places);
                    }
                    Kind::Tool => {
                        arg_choice(ui, t, f.label, &mut val, &tools, &key, places);
                        if let Some(tool) = w.lib.tools.iter().find(|x| {
                            w.stmt(&val).and_then(|s| s.pos(0)).map(super::doc::unquote).as_deref() == Some(x.id.as_str())
                        }) {
                            fact(ui, t, "Type", tool.kind.name(), DLG_W, t.ink2);
                            fact(
                                ui,
                                t,
                                "Diameter · flutes · flute length",
                                &format!("{} · {} · {} mm", num(tool.dc), tool.nof, num(tool.lcf)),
                                DLG_W,
                                t.ink2,
                            );
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
            note(ui, &CamWs::op_text(d), DLG_W, t.ink2);
            if let Some(e) = &d.error {
                note(ui, e, DLG_W, t.error);
            }
            (ok, cancel, apply) = foot(ui, t, "cam:op", true, places);
        });
    let rect = r.map(|r| r.response.rect);
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
                format!(
                    "{} · {}",
                    s.name,
                    s.pos(0).map(super::doc::unquote).unwrap_or_default()
                ),
            )
        })
        .collect();
    machines.extend(w.lib.machines.iter().map(|m| {
        (
            format!("machine(\"{}\")", m.name),
            format!("{} ({}, {})", m.name, m.kind, m.control),
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
    let r = egui::Window::new("cam-setup")
        .id(egui::Id::unique("cam-setup-dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(DLG_W, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
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
                    arg_field(ui, t, "Origin point", &mut o, "\"top-center\" or (x, y, z)", "", "cam:setup:origin:text", places);
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
                note(ui, &w.setup_text(d), DLG_W, t.ink2);
            }
            if let Some(e) = &d.error {
                note(ui, e, DLG_W, t.error);
            }
            (ok, cancel, _) = foot(ui, t, "cam:setup", false, places);
        });
    let rect = r.map(|r| r.response.rect);
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
    let r = egui::Window::new("cam-lathe")
        .id(egui::Id::unique("cam-lathe-dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(DLG_W, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
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
            note(ui, &w.setup_text(d), DLG_W, t.ink2);
            if let Some(e) = &d.error {
                note(ui, e, DLG_W, t.error);
            }
            (ok, cancel, _) = foot(ui, t, "cam:setup", false, places);
        });
    let rect = r.map(|r| r.response.rect);
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
    let width = 620.0;
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
        .constrain_to(crate::shell::dialog_area(ctx))
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
            let res = w
                .run
                .as_ref()
                .and_then(|r| r.program(&id));
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
    let width = 560.0;
    let mut add: Option<String> = None;
    let r = egui::Window::new("cam-tools")
        .id(egui::Id::unique("cam-tools-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.center().x - width / 2.0, v.top() + 60.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
        .show_placed(ctx, places, |ui, places| {
            close = head(ui, t, "cam_tools", "Tool Library", width, places, "cam:tools");
            let mut f = w.tools_filter.clone();
            row(ui, t, "Filter", width, 30.0, |ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut f).font(sans(13.0)).hint_text("type, id or diameter").desired_width(ui.available_width() - 6.0));
                places.push(("cam:tools:filter".into(), r.rect));
            });
            w.tools_filter = f.clone();
            let f = f.to_lowercase();
            let used: Vec<String> = w.tools().iter().filter_map(|s| s.pos(0).map(super::doc::unquote)).collect();
            if w.turning() {
                lathe_tools(ui, t, width, &f, &used, &mut add, places);
                ui.add_space(8.0);
                return;
            }
            heading(ui, t, "Cutters", width);
            let (r, _) = ui.allocate_exact_size(vec2(width, 300.0), Sense::hover());
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
            egui::ScrollArea::vertical().id_salt("cam-tools-list").auto_shrink([false, false]).show(&mut child, |ui| {
                for tool in &w.lib.tools {
                    let line = format!(
                        "{}  ·  {}  ·  D {}  ·  {} fl  ·  flute {}",
                        tool.id,
                        tool.kind.name(),
                        num(tool.dc),
                        tool.nof,
                        num(tool.lcf)
                    );
                    if !f.is_empty() && !line.to_lowercase().contains(&f) {
                        continue;
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(width - 12.0, 24.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 0.0, t.hover);
                    }
                    let inu = used.contains(&tool.id);
                    ui.painter().text(pos2(r.left() + 14.0, r.center().y), Align2::LEFT_CENTER, &line, sans(12.0), t.ink);
                    ui.painter().text(
                        pos2(r.right() - 10.0, r.center().y),
                        Align2::RIGHT_CENTER,
                        if inu { "in document" } else { "Add" },
                        sans(12.0),
                        if inu { t.ink2 } else { t.accent },
                    );
                    places.push((format!("cam:tools:row:{}", tool.id), r));
                    if resp.clicked() && !inu {
                        add = Some(tool.id.clone());
                    }
                }
            });
            heading(ui, t, "Import a Fusion tool library", width);
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
            if let Some(s) = &w.said {
                note(ui, s, width, t.ink2);
            }
            heading(ui, t, "Holders", width);
            for h in &w.lib.holders {
                fact(ui, t, &h.id, &format!("{} · gauge {} mm · {} rpm", h.interface, num(h.gauge_length), num(h.max_rpm)), width, t.ink);
            }
            note(ui, "Built in: generic tools and conservative holders, our own numbers; verify with the vendor. A project adds its own in cam/*.toml beside the document.", width, t.ink2);
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
    if close {
        w.tools_open = false;
    }
    r.map(|r| r.response.rect)
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
    let width = 560.0;
    let r = egui::Window::new("cam-section")
        .id(egui::Id::unique("cam-section-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
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
    let width = 620.0;
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
    let r = egui::Window::new("cam-programs")
        .id(egui::Id::unique("cam-programs-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
        .show_placed(ctx, places, |ui, places| {
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
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                let r = ui.checkbox(&mut allow, "Allow uncertified (their headers say what was not checked)");
                places.push(("cam:programs:allow".into(), r.rect));
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
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
                if w.run.is_some() && button(ui, t, "Setup Sheet & Tools…", false, "cam:programs:sheet".into(), places) {
                    cmds.push(Cmd::ExportSheet);
                }
            });
            if let Some(s) = &w.said {
                note(ui, s, width, t.ink2);
            }
            note(
                ui,
                "Post writes a program's file beside the document (File, Folder, Ext.); Export… saves a copy where you choose (several in a folder, or a zip in the browser).",
                width,
                t.ink2,
            );
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
    ui.painter().circle_filled(pos2(r.left() + 19.0, r.center().y), 4.0, ink);
    ui.painter().text(pos2(r.left() + 30.0, r.center().y), Align2::LEFT_CENTER, &p.name, medium(14.0), t.ink);
    let words = match state {
        Some((_, s, _)) if states.len() > 1 => format!("{} files: {}", states.len(), s.words()),
        Some((_, s, path)) if !path.is_empty() => format!("{path}: {}", s.words()),
        Some((_, s, _)) => s.words().to_string(),
        None => "not generated".into(),
    };
    ui.painter().text(pos2(r.right() - 14.0, r.center().y), Align2::RIGHT_CENTER, words, sans(12.0), ink);
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
    ui.horizontal(|ui| {
        ui.add_space(14.0);
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
        let text = if ordered.len() == 1 { ordered[0].clone() } else { format!("[{}]", ordered.join(", ")) };
        // a program reads its setups: below the last of them first
        let at = |n: &str| w.stmts.iter().position(|x| x.name == n);
        if ordered.iter().any(|su| at(su) > at(&p.name)) {
            edits.push(super::doc::Edit::Move { id: p.name.clone(), before: None });
        }
        edits.push(super::doc::Edit::Set { id: p.name.clone(), arg: "#0".into(), text });
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
        let mut pick: Vec<String> = ops.iter().filter(|o| on_now(&o.name)).map(|o| o.name.clone()).collect();
        ui.horizontal_wrapped(|ui| {
            ui.add_space(14.0);
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
        let was: Vec<String> = ops.iter().filter(|o| on_now(&o.name)).map(|o| o.name.clone()).collect();
        if pick.len() != was.len() && !pick.is_empty() {
            let ordered: Vec<String> = ops.iter().filter(|o| pick.contains(&o.name)).map(|o| o.name.clone()).collect();
            if ordered.len() == ops.len() {
                edits.push(super::doc::Edit::Unset { id: p.name.clone(), arg: "ops".into() });
            } else {
                // a program reads its ops: below the last of them first
                let at = |n: &str| w.stmts.iter().position(|x| x.name == n);
                if ordered.iter().any(|o| at(o) > at(&p.name)) {
                    edits.push(super::doc::Edit::Move { id: p.name.clone(), before: None });
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
    note(ui, &line, width, if res.as_ref().is_some_and(|r| r.text.is_some()) { t.ink } else { t.warn });
    if ops_changed && state.is_some_and(|s| s.1 == Posted::Current) {
        note(ui, "Ops changed since the run: generate, then re-post.", width, t.warn);
    }
    // its file: name, folder, extension, header
    let ext_default = forge_cam_lang::export::ext_of(&p.arg_str("post").unwrap_or_default()).to_string();
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        prog_field(ui, t, "File", 110.0, p, "file", &p.name, field, edits, places);
        prog_field(ui, t, "Folder", 90.0, p, "out", "beside it", field, edits, places);
        prog_field(ui, t, "Ext.", 44.0, p, "ext", &ext_default, field, edits, places);
        prog_field(ui, t, "Comment", 140.0, p, "comment", "header line", field, edits, places);
    });
    // posting: one file per setup, decimals, coolant (forge-post's options, certified as posted)
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        let had = p.arg_str("split").as_deref() == Some("setup");
        let mut on = had;
        let r = ui.checkbox(&mut on, "One file per setup");
        places.push((format!("cam:programs:{}:split", p.name), r.rect));
        if on != had {
            edits.push(if on {
                super::doc::Edit::Set { id: p.name.clone(), arg: "split".into(), text: "\"setup\"".into() }
            } else {
                super::doc::Edit::Unset { id: p.name.clone(), arg: "split".into() }
            });
        }
        prog_field(ui, t, "Decimals", 34.0, p, "decimals", "", field, edits, places);
        prog_field(ui, t, "Feed dec.", 34.0, p, "feed_decimals", "", field, edits, places);
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Coolant").font(sans(12.0)).color(t.ink2));
        let now = p.arg_str("coolant").unwrap_or_default();
        let mut pick = now.clone();
        let r = egui::ComboBox::from_id_salt(format!("cam-programs-{}-coolant", p.name))
            .width(70.0)
            .selected_text(if pick.is_empty() { "post's" } else { pick.as_str() })
            .show_ui(ui, |ui| {
                for c in ["", "flood", "mist", "air", "off"] {
                    ui.selectable_value(&mut pick, c.to_string(), if c.is_empty() { "post's" } else { c });
                }
            });
        places.push((format!("cam:programs:{}:coolant", p.name), r.response.rect));
        if pick != now {
            edits.push(if pick.is_empty() {
                super::doc::Edit::Unset { id: p.name.clone(), arg: "coolant".into() }
            } else {
                super::doc::Edit::Set { id: p.name.clone(), arg: "coolant".into(), text: format!("\"{pick}\"") }
            });
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        let had = p.arg_str("tools").as_deref() == Some("list");
        let mut on = had;
        let r = ui.checkbox(&mut on, "Tool list in header");
        places.push((format!("cam:programs:{}:tools", p.name), r.rect));
        if on != had {
            edits.push(if on {
                super::doc::Edit::Set { id: p.name.clone(), arg: "tools".into(), text: "\"list\"".into() }
            } else {
                super::doc::Edit::Unset { id: p.name.clone(), arg: "tools".into() }
            });
        }
        ui.add_space(12.0);
        let ok = res.as_ref().is_some_and(|r| r.text.is_some());
        if ok && button(ui, t, "Post", true, format!("cam:programs:{}:post", p.name), places) {
            cmds.push(Cmd::PostNc(Some(p.name.clone())));
        }
        if ok && button(ui, t, "Export…", false, format!("cam:programs:{}:export", p.name), places) {
            cmds.push(Cmd::ExportNc(Some(p.name.clone())));
        }
        if button(ui, t, "Post Process", false, format!("cam:programs:{}:open", p.name), places) {
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
    wide: f32,
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
    ui.add_space(6.0);
    ui.label(egui::RichText::new(label).font(sans(12.0)).color(t.ink2));
    let r = ui.add(
        egui::TextEdit::singleline(&mut v)
            .font(sans(12.0))
            .hint_text(hint)
            .desired_width(wide)
            .margin(vec2(5.0, 3.0)),
    );
    places.push((key.clone(), r.rect));
    if r.changed() {
        *field = Some((key.clone(), v.clone()));
    }
    if r.lost_focus() && field.as_ref().is_some_and(|f| f.0 == key) {
        *field = None;
        let v = v.trim().replace(['"', '\\'], "");
        if v != now {
            edits.push(if v.is_empty() {
                super::doc::Edit::Unset { id: p.name.clone(), arg: arg.into() }
            } else {
                super::doc::Edit::Set {
                    id: p.name.clone(),
                    arg: arg.into(),
                    // counts are numbers; names and folders are strings
                    text: if NUMBER_ARGS.contains(&arg) { v.clone() } else { format!("\"{v}\"") },
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
    let width = 720.0;
    let mut select: Option<String> = None;
    let mut colour = w.focus.colour_by;
    let r = egui::Window::new("cam-quality")
        .id(egui::Id::unique("cam-quality-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
        .show_placed(ctx, places, |ui, places| {
            close = head(ui, t, "cam_check", "Toolpath Quality", width, places, "cam:quality");
            // the toolpath's colouring in the view (the simulation view draws it)
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new("Colour toolpath by").font(sans(13.0)).color(t.ink2));
                use super::ColourBy;
                for (label, c, key) in [
                    ("Op", None, "op"),
                    ("Chip load", Some(ColourBy::ChipLoad), "chip"),
                    ("Engagement", Some(ColourBy::Engagement), "engagement"),
                ] {
                    let r = ui.selectable_label(colour == c, label);
                    places.push((format!("cam:quality:colour:{key}"), r.rect));
                    if r.clicked() {
                        colour = c;
                    }
                }
            });
            let Some(run) = &w.run else { return };
            let Some(job) = &run.quality else {
                note(ui, "Not measured (turning runs are not yet).", width, t.ink2);
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
            let header = |ui: &mut Ui| {
                let (r, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
                let mut x = r.left() + 14.0;
                for (c, wd) in cols {
                    ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, c, sans(11.0), t.ink2);
                    x += wd;
                }
            };
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
                    let (r, resp) = ui.allocate_exact_size(vec2(width, 22.0), Sense::click());
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
                    for ((c, (_, wd)), wn) in cells.iter().zip(cols).zip(warn) {
                        ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, c, mono(11.0), if wn { t.warn } else { t.ink });
                        x += wd;
                    }
                }
            }
            if !job.progs.is_empty() {
                heading(ui, t, "Programs on the machine's timeline", width);
                for p in &job.progs {
                    let words = match &p.error {
                        Some(e) => format!("not timed: {e}"),
                        None => format!(
                            "{} with stops ({} at least) · {} tool changes{}",
                            time(p.t_stop),
                            time(p.t_min),
                            p.tool_changes,
                            if p.accel_missing { " · the machine gives no accelerations" } else { "" }
                        ),
                    };
                    fact(ui, t, &format!("{} on {}", p.name, p.machine), &words, width, if p.error.is_some() { t.warn } else { t.ink });
                }
            }
            note(
                ui,
                &format!(
                    "Measured on a height map of {} mm cells (forge-cam-quality): rubbing is a chip under half the feed per tooth, overloaded over 1.25 times it. Amber: worth a look.",
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
    let width = 460.0;
    let mut locate: Option<Locate> = None;
    let r = egui::Window::new("cam-checks")
        .id(egui::Id::unique("cam-checks-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.right() - width - 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
        .show_placed(ctx, places, |ui, places| {
            close = head(ui, t, "cam_check", "Checks", width, places, "cam:checks");
            let Some(run) = &w.run else {
                note(ui, "Generate first.", width, t.ink2);
                return;
            };
            let (word, c) = match run.certification.as_str() {
                "full" => ("Every op certified, every program released", t.success),
                "except G5" => (
                    "Every op certified; machine travel (G5) checked by span only: a setup is not placed (at:)",
                    t.ink2,
                ),
                _ => ("Refused: an op or program below", t.error),
            };
            note(ui, word, width, c);
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
    let width = 640.0;
    let r = egui::Window::new("cam-text")
        .id(egui::Id::unique("cam-text-window"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(pos2(v.left() + 24.0, v.top() + 44.0))
        .fixed_size(vec2(width, 0.0))
        .frame(frame(t))
        .constrain_to(crate::shell::dialog_area(ctx))
        .show_placed(ctx, places, |ui, places| {
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
                .max_height(420.0)
                .show(ui, |ui| {
                    let r = ui.add(
                        egui::TextEdit::multiline(&mut w.text_edit)
                            .id(egui::Id::unique("cam-text-edit"))
                            .font(mono(12.0))
                            .code_editor()
                            .desired_width(width - 28.0)
                            .desired_rows(20),
                    );
                    places.push(("cam:text:edit".into(), r.rect));
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
    let machining = if time > 0.0 {
        format!(" · {} machining", self::time(time))
    } else {
        String::new()
    };
    format!(
        "{} · {built}/{n} ops certified{machining}{}",
        w.path.rsplit('/').next().unwrap_or(""),
        if w.dirty() { " · unsaved" } else { "" }
    )
}
