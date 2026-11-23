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
    let of = |g: &str| -> Vec<Button> { schema::OPS.iter().filter(|o| o.group == g).map(op).collect() };
    let two_d = of("2D");
    let (big2, small2) = two_d.split_at(3.min(two_d.len()));
    let generating = w.running.is_some();
    vec![
        (
            "Setup",
            vec![("New Setup", "cam_setup", a(Ask::NewSetup), false)],
            vec![],
        ),
        ("2D", big2.to_vec(), small2.to_vec()),
        ("Drilling", of("Drilling"), vec![]),
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
            ],
            vec![
                (
                    "Setup Sheet",
                    "cam_sheet",
                    w.run.is_some().then_some(Action::Cam(Ask::SetupSheet)),
                    false,
                ),
                ("Checks", "cam_check", a(Ask::Checks), w.checks_open),
                ("Auto Generate", "cam_generate", a(Ask::AutoGenerate), w.auto),
            ],
        ),
        (
            "Manage",
            vec![("Tool Library", "cam_tools", a(Ask::ToolLibrary), w.tools_open)],
            vec![("Document Text", "cam_post", a(Ask::ToggleText), w.text_open)],
        ),
    ]
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

/// A work offset's word: G54 for the first.
fn wcs_word(k: u32) -> String {
    if k < 6 {
        format!("G{}", 54 + k)
    } else {
        format!("G54.1 P{}", k - 5)
    }
}

fn time(s: f64) -> String {
    if s >= 60.0 {
        format!("{}:{:02} min", (s / 60.0).floor(), (s % 60.0).round() as u32)
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
                    Some(run) if run.certification == "except G5" => ("certified except G5".into(), t.warn),
                    Some(_) => ("refused".into(), t.error),
                    None => ("not generated".into(), t.ink2),
                };
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
                    let label = match w.run.as_ref().and_then(|r| r.setup(&su)) {
                        Some(s) => format!("{su}  ·  {}", wcs_word(s.offset)),
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
                            .and_then(|r| r.programs.iter().find(|x| x.name == p))
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
        if w.tools_open {
            blocks.extend(tools_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.checks_open {
            blocks.extend(checks_window(ctx, &t, v, &mut w, &mut places));
        }
        if w.text_open {
            blocks.extend(text_window(ctx, &t, v, &mut w, &mut places));
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
        super::doc::Role::Setup => {
            let Some(su) = run.and_then(|r| r.setup(id)) else {
                note(ui, "Generate to see the stock and work offset.", width, t.ink2);
                return;
            };
            fact(ui, t, "Machine", &su.machine, width, t.ink);
            fact(ui, t, "Work offset", &wcs_word(su.offset), width, t.ink);
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
            let Some(p) = run.and_then(|r| r.programs.iter().find(|p| p.name == id)) else {
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
                    &format!(
                        "T{} {tool}, D {}",
                        o.tool_t.unwrap_or(0),
                        num(o.tool_d.unwrap_or(0.0))
                    ),
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
            if let Some(l) = o.least {
                fact(
                    ui,
                    t,
                    "Least cutter distance",
                    &format!("{} mm", num(l)),
                    width,
                    t.ink,
                );
            }
            if o.pairs > 0 {
                fact(ui, t, "Certified pairs", &o.pairs.to_string(), width, t.ink);
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
                note(
                    ui,
                    "Linking is by retract to the setup's retract height; rapids are certified clear of the part and the in-process stock.",
                    DLG_W,
                    t.ink2,
                );
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
                        _ => {
                            let mut s = get(d, "margin");
                            arg_field(ui, t, "Offsets (x, y, z)", &mut s, "(3, 3, 2)", "mm", "cam:setup:margin", places);
                            set(d, "margin", s);
                        }
                    }
                    note(ui, "Cylinder and tube stock and stock from a solid are not in this phase's language yet.", DLG_W, t.ink2);
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
            let dialects: Vec<(String, String)> = forge_post::DIALECTS
                .iter()
                .map(|(n, _)| (format!("\"{n}\""), (*n).to_string()))
                .collect();
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
                .and_then(|r| r.programs.iter().find(|p| p.name == id))
                .cloned();
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
                if !text.is_empty() && button(ui, t, "Write NC File", true, "cam:post:write".into(), places) {
                    let p = w.dir.join(format!("{id}.nc"));
                    write = Some((p.display().to_string(), text.clone()));
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
            heading(ui, t, "Holders", width);
            for h in &w.lib.holders {
                fact(ui, t, &h.id, &format!("{} · gauge {} mm · {} rpm", h.interface, num(h.gauge_length), num(h.max_rpm)), width, t.ink);
            }
            note(ui, "Built in: generic tools and conservative holders, our own numbers; verify with the vendor. A project adds its own in cam/*.toml beside the document.", width, t.ink2);
            ui.add_space(8.0);
        });
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
                "except G5" => ("Certified except G5: a setup is not placed (at:)", t.warn),
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
    format!(
        "{} · {built}/{n} ops certified · {} machining{}",
        w.path.rsplit('/').next().unwrap_or(""),
        self::time(time),
        if w.dirty() { " · unsaved" } else { "" }
    )
}
