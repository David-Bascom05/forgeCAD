//! A DWG document's window, laid out as AutoCAD's in ForgeCAD's colours: the ribbon's Home,
//! Insert, Annotate, View, Manage and Output tabs; the Properties palette on the left and the
//! Layers palette on the right; the canvas; the command line docked under it; the Model and
//! layout tabs with the cursor's coordinates and the drafting toggles; the document tabs and the
//! status bar. The canvas feeds the drafting commands (`dwgcmd.rs`) its clicks, windows and the
//! cursor; the palettes and the ribbon's layer dropdown edit the drawing (`dwgprops.rs`); what
//! the commands draw over the canvas is in `dwgdoc_edit_ui.rs`.

use super::dwgcmd::{Inp, want};
use super::dwgdoc::{Cmd, DwgDoc, Press, Run, TOGGLES, View};
use super::dwgprops::Switch;
use forge_dwg::draft::V2;
use super::theme::{mono, sans, semi};
use super::{Action, Shell, icons};
use crate::dwgview::gpu::{Look, Paint};
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// The command line: its history lines and its input line.
const CMD_LINES: usize = 3;
const CMD_LINE_H: f32 = 17.0;
const CMD_H: f32 = CMD_LINES as f32 * CMD_LINE_H + 36.0;
/// The Model and layout tabs' row, with the coordinates and toggles.
const MODEL_H: f32 = 28.0;
const PROPS_W: f32 = 300.0;
const LAYERS_W: f32 = 320.0;
/// AutoCAD's pickbox, half its side, in points; and the crosshair's arms.
const PICKBOX: f32 = 5.0;
const CROSSHAIR: f32 = 48.0;
/// A press that moves this far is a window, not a click.
const DRAG_PT: f32 = 4.0;

/// The model space's background: AutoCAD's dark, in ForgeCAD's blue-grey.
const MODEL_BG: Color32 = Color32::from_rgb(0x1B, 0x21, 0x29);
/// A layout's backdrop round its sheet.
const LAYOUT_BG: Color32 = Color32::from_rgb(0x5D, 0x65, 0x70);
const SELECT: [u8; 3] = [0x4A, 0x9B, 0xFF];
const SELECT_PAPER: [u8; 3] = [0x1E, 0x6F, 0xD9];
const HOVER: [u8; 3] = [0xFF, 0x9A, 0x1F];
const WINDOW: Color32 = Color32::from_rgb(0x3C, 0x8C, 0xFF);
const CROSSING: Color32 = Color32::from_rgb(0x4C, 0xC2, 0x6A);

type Tools = Vec<(&'static str, &'static str, &'static str)>;

/// The ribbon's groups for the tab: each a name, its big buttons and its small ones, a button
/// (word, icon, command).
pub(super) fn groups(tab: usize) -> Vec<(&'static str, Tools, Tools)> {
    match tab {
        0 => vec![
            (
                "Draw",
                vec![
                    ("Line", "line", "LINE"),
                    ("Polyline", "polyline", "PLINE"),
                    ("Circle", "circle", "CIRCLE"),
                    ("Arc", "arc", "ARC"),
                ],
                vec![
                    ("Rectangle", "rectangle", "RECTANG"),
                    ("Hatch", "hatch", "HATCH"),
                    ("Ellipse", "ellipse", "ELLIPSE"),
                    ("Polygon", "polygon", "POLYGON"),
                    ("Spline", "spline", "SPLINE"),
                ],
            ),
            (
                "Modify",
                vec![],
                vec![
                    ("Move", "move", "MOVE"),
                    ("Copy", "copy", "COPY"),
                    ("Stretch", "stretch", "STRETCH"),
                    ("Rotate", "rotate", "ROTATE"),
                    ("Mirror", "mirror", "MIRROR"),
                    ("Scale", "scale", "SCALE"),
                    ("Trim", "trim", "TRIM"),
                    ("Fillet", "fillet", "FILLET"),
                    ("Array", "rect", "ARRAY"),
                    ("Erase", "delete", "ERASE"),
                    ("Explode", "explode", "EXPLODE"),
                    ("Offset", "offset", "OFFSET"),
                    ("Extend", "extend", "EXTEND"),
                    ("Chamfer", "chamfer", "CHAMFER"),
                ],
            ),
            (
                "Annotation",
                vec![("Text", "text", "TEXT"), ("Dimension", "dim", "DIM")],
                vec![
                    ("Leader", "leadertext", "LEADER"),
                    ("Table", "partslist", "TABLE"),
                ],
            ),
            ("Layers", vec![("Layer Properties", "layers", "LAYER")], vec![]),
            (
                "Properties",
                vec![("Properties", "params", "PROPERTIES")],
                vec![("Match Properties", "matchprop", "MATCHPROP")],
            ),
            ("Utilities", vec![("Measure", "measure", "MEASUREGEOM")], vec![]),
        ],
        1 => vec![
            (
                "Block",
                vec![
                    ("Insert", "block", "INSERT"),
                    ("Create", "sketchnode", "BLOCK"),
                    ("Edit", "edit", "BEDIT"),
                ],
                vec![],
            ),
            (
                "Reference",
                vec![("Attach", "import", "XATTACH")],
                vec![("Clip", "dwgcrop", "CLIP")],
            ),
        ],
        2 => vec![
            (
                "Text",
                vec![("Multiline Text", "text", "MTEXT")],
                vec![("Single Line", "text", "TEXT")],
            ),
            (
                "Dimensions",
                vec![("Dimension", "dim", "DIM")],
                vec![
                    ("Linear", "dim", "DIMLINEAR"),
                    ("Aligned", "dim", "DIMALIGNED"),
                    ("Angular", "angle", "DIMANGULAR"),
                    ("Radius", "circle", "DIMRADIUS"),
                    ("Diameter", "circle", "DIMDIAMETER"),
                ],
            ),
            ("Leaders", vec![("Multileader", "leadertext", "LEADER")], vec![]),
            ("Tables", vec![("Table", "partslist", "TABLE")], vec![]),
        ],
        3 => vec![
            (
                "Navigate",
                vec![("Zoom Extents", "fit", "ZOOMEXTENTS"), ("Pan", "pan", "PAN")],
                vec![("Regen", "check", "REGEN")],
            ),
            (
                "Palettes",
                vec![
                    ("Layer Properties", "layers", "LAYER"),
                    ("Properties", "params", "PROPERTIES"),
                ],
                vec![],
            ),
            ("Display", vec![("Lineweight", "lines", "LWDISPLAY")], vec![]),
        ],
        4 => vec![(
            "Cleanup",
            vec![("Audit", "check", "AUDIT"), ("Purge", "delete", "PURGE")],
            vec![],
        )],
        _ => vec![
            (
                "Plot",
                vec![("Plot", "plot", "PLOT"), ("Export PDF", "export", "EXPORTPDF")],
                vec![],
            ),
            (
                "Save",
                vec![("Save", "save", "QSAVE"), ("Save As", "save", "SAVEAS")],
                vec![],
            ),
        ],
    }
}

/// Is a ribbon button's command on (a palette open, LWDISPLAY)?
pub(super) fn tool_on(d: &DwgDoc, cmd: &str) -> bool {
    match cmd {
        "LAYER" => d.show_layers,
        "PROPERTIES" => d.show_props,
        "LWDISPLAY" => d.lwt,
        _ => false,
    }
}

pub(super) fn to_world(v: &View, canvas: Rect, p: Pos2) -> [f64; 2] {
    let c = canvas.center();
    [
        v.center[0] + (p.x - c.x) as f64 / v.scale,
        v.center[1] - (p.y - c.y) as f64 / v.scale,
    ]
}

pub(super) fn to_screen(v: &View, canvas: Rect, w: [f64; 2]) -> Pos2 {
    let c = canvas.center();
    pos2(
        c.x + ((w[0] - v.center[0]) * v.scale) as f32,
        c.y - ((w[1] - v.center[1]) * v.scale) as f32,
    )
}

/// A dashed rectangle's sides.
fn dashed_rect(p: &egui::Painter, r: Rect, stroke: Stroke) {
    let pts = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for w in pts.windows(2) {
        p.extend(egui::Shape::dashed_line(&[w[0], w[1]], stroke, 6.0, 4.0));
    }
}

impl Shell {
    /// The whole window for a DWG document.
    pub(super) fn dwgdoc_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let s = ctx.content_rect();
        let t = self.theme;
        let Some(d) = &self.dwgdoc else { return };
        let (props, layers) = (d.show_props, d.show_layers);
        let top = super::TITLE_H + super::TABS_H + super::RIBBON_H;
        let bars = s.height() - super::DOCS_H - super::STATUS_H;
        let model_y = bars - MODEL_H;
        let cmd_y = model_y - CMD_H;
        let left = if props { PROPS_W } else { 0.0 };
        let right = s.width() - if layers { LAYERS_W } else { 0.0 };
        let ppp = ctx.pixels_per_point();
        let snap = |v: f32| (v * ppp).round() / ppp;
        let canvas = Rect::from_min_max(
            pos2(snap(left + 1.0), snap(top)),
            pos2(snap(right - 1.0), snap(cmd_y)),
        );
        self.view_rect = canvas;
        self.title_bar(ui, s.width());
        self.tabs_row(ui, s.width());
        self.dwg_keys(&ctx);
        self.ribbon(ui, s.width());
        self.dwg_layer_dropdown(ui);
        self.dwg_canvas(ui, canvas);
        if props {
            self.dwg_props(ui, Rect::from_min_max(pos2(0.0, top), pos2(left, model_y)));
        }
        if layers {
            self.dwg_layers(ui, Rect::from_min_max(pos2(right, top), pos2(s.width(), model_y)));
        }
        self.dwg_command_line(ui, Rect::from_min_max(pos2(left, cmd_y), pos2(right, model_y)));
        self.dwg_model_row(
            ui,
            Rect::from_min_size(pos2(0.0, model_y), vec2(s.width(), MODEL_H)),
        );
        self.dwg_bottom(ui, s, bars);
        self.dwg_warnings(&ctx);
        self.dwg_osnap_window(&ctx);
        self.dwg_mtext_window(&ctx);
        self.resize_edges(ui, s);
        self.menus(&ctx);
        // the ribbon's and the File menu's commands are the document's own
        let asked: Vec<Cmd> = self
            .out
            .iter()
            .filter_map(|a| match a {
                Action::DwgDoc(c) => Some(*c),
                _ => None,
            })
            .collect();
        self.out.retain(|a| !matches!(a, Action::DwgDoc(_)));
        for c in asked {
            match c {
                Cmd::Run(name) => self.dwg_button(name),
            }
        }
        let _ = t;
    }

    /// A command by name, from a button or typed: what it leaves to the window, done.
    pub(super) fn dwg_run(&mut self, name: &str) {
        let canvas = self.view_rect;
        let Some(d) = &mut self.dwgdoc else { return };
        if name == "ZOOMEXTENTS" {
            d.say("Command: ZOOM");
            d.say("Extents");
            d.fit(canvas.size());
            return;
        }
        match d.run_command(name) {
            Run::Done => {}
            Run::ZoomExtents => d.fit(canvas.size()),
            Run::SaveAs => self.out.push(Action::SaveAs),
            Run::Open => self.out.push(Action::Open),
        }
    }

    /// A ribbon button's command: the one running called off first.
    pub(super) fn dwg_button(&mut self, name: &str) {
        if let Some(d) = &mut self.dwgdoc
            && d.ed.active()
        {
            d.cancel_command();
        }
        self.dwg_run(name);
    }

    /// The keyboard goes to the command line whenever nothing else holds it, as AutoCAD's does.
    fn dwg_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let space_held = ctx.input(|i| i.key_down(egui::Key::Space));
        let mut submit = None;
        let mut toggles = vec![];
        let mut escape = false;
        let mut save = false;
        let mut undo = None;
        {
            let Some(d) = &mut self.dwgdoc else { return };
            let active = d.ed.active();
            let wants_text = d.ed.prompt.as_ref().is_some_and(|p| p.wants(want::TEXT));
            // Space pans (held, the left button dragging) only with no command running
            d.space_pan = space_held && d.input.is_empty() && !active;
            for e in &events {
                match e {
                    egui::Event::Text(s) if s == " " => {
                        if wants_text {
                            d.input.push(' ');
                        }
                    }
                    egui::Event::Text(s) if !s.chars().any(char::is_control) => {
                        d.input.push_str(s);
                        d.recall = None;
                    }
                    egui::Event::Key {
                        key: egui::Key::Space,
                        pressed: false,
                        ..
                    } => {
                        // Space let go with no pan, no command: the last command again
                        if !active && !wants_text && d.input.is_empty() && !d.space_panned {
                            submit = Some(String::new());
                        }
                        d.space_panned = false;
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        use egui::Key as K;
                        match key {
                            K::S if modifiers.command => save = true,
                            K::Z if modifiers.command => undo = Some(true),
                            K::Y if modifiers.command => undo = Some(false),
                            K::Num1 if modifiers.command => {
                                d.show_props = !d.show_props;
                            }
                            K::Enter => submit = Some(std::mem::take(&mut d.input)),
                            // Space is Enter for a command (but a space in its text)
                            K::Space if !wants_text && (active || !d.input.is_empty()) => {
                                submit = Some(std::mem::take(&mut d.input))
                            }
                            K::Backspace => {
                                d.input.pop();
                            }
                            K::Escape => escape = true,
                            K::Delete if d.input.is_empty() && !d.selected.is_empty() && !active => {
                                submit = Some("ERASE".into());
                            }
                            K::ArrowUp | K::ArrowDown if !d.typed.is_empty() => {
                                let n = d.typed.len();
                                let k = match (d.recall, *key == K::ArrowUp) {
                                    (None, true) => n - 1,
                                    (None, false) => continue,
                                    (Some(k), true) => k.saturating_sub(1),
                                    (Some(k), false) => (k + 1).min(n - 1),
                                };
                                d.recall = Some(k);
                                d.input = d.typed[k].clone();
                            }
                            K::F3 => toggles.push("OSNAP"),
                            K::F7 => toggles.push("GRID"),
                            K::F8 => toggles.push("ORTHO"),
                            K::F9 => toggles.push("SNAP"),
                            K::F10 => toggles.push("POLAR"),
                            K::F12 => toggles.push("DYN"),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        if escape {
            if self.file_menu {
                self.file_menu = false;
            } else if let Some(d) = &mut self.dwgdoc {
                d.escape();
                d.refresh_grips();
            }
        }
        if save {
            self.dwg_run("QSAVE");
        }
        if let (Some(u), Some(d)) = (undo, &mut self.dwgdoc) {
            d.say(if u { "Command: U" } else { "Command: REDO" });
            if u { d.undo() } else { d.redo() }
        }
        for name in toggles {
            self.dwg_toggle(name);
        }
        if let Some(line) = submit {
            if let Some(d) = &mut self.dwgdoc {
                if !line.trim().is_empty() && !d.ed.active() {
                    d.typed.push(line.trim().to_string());
                }
                d.recall = None;
            }
            self.dwg_run(&line);
        }
    }

    fn dwg_toggle(&mut self, name: &str) {
        let Some(d) = &mut self.dwgdoc else { return };
        let st = &mut d.ed.set;
        let (word, on) = match name {
            "LWT" => {
                d.lwt = !d.lwt;
                ("Lineweight", d.lwt)
            }
            "SNAP" => {
                st.snap = !st.snap;
                ("Snap", st.snap)
            }
            "GRID" => {
                st.grid = !st.grid;
                ("Grid", st.grid)
            }
            "ORTHO" => {
                st.ortho = !st.ortho;
                if st.ortho {
                    st.polar = false;
                }
                ("Ortho", st.ortho)
            }
            "POLAR" => {
                st.polar = !st.polar;
                if st.polar {
                    st.ortho = false;
                }
                ("Polar", st.polar)
            }
            "OSNAP" => {
                st.osnap = !st.osnap;
                ("Osnap", st.osnap)
            }
            "DYN" => {
                st.dyn_input = !st.dyn_input;
                ("Dynamic Input", st.dyn_input)
            }
            _ => return,
        };
        d.say(format!("<{word} {}>", if on { "on" } else { "off" }));
    }

    /// Is a status toggle on?
    fn dwg_toggle_on(d: &DwgDoc, name: &str) -> bool {
        let st = &d.ed.set;
        match name {
            "LWT" => d.lwt,
            "SNAP" => st.snap,
            "GRID" => st.grid,
            "ORTHO" => st.ortho,
            "POLAR" => st.polar,
            "OSNAP" => st.osnap,
            "DYN" => st.dyn_input,
            _ => false,
        }
    }

    fn dwg_canvas(&mut self, ui: &mut Ui, canvas: Rect) {
        let t = self.theme;
        let Some(d) = &mut self.dwgdoc else { return };
        let ctx = ui.ctx().clone();
        if d.fit_next || d.view.is_none() {
            d.fit(canvas.size());
            d.fit_next = false;
        }
        let paper = d.is_paper();
        let p = ui.painter_at(canvas);
        p.rect_filled(canvas, 0.0, if paper { LAYOUT_BG } else { MODEL_BG });
        let resp = ui.interact(canvas, ui.scope_id().with("dwg-canvas"), Sense::click_and_drag());
        let mut v = d.view.expect("fitted");
        // the wheel zooms about the pointer
        if let Some(hp) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let w = to_world(&v, canvas, hp);
                v.scale *= forge_math::dmath::exp(scroll as f64 * 0.0025);
                v.scale = v.scale.clamp(1e-12, 1e12);
                let c = canvas.center();
                v.center = [
                    w[0] - (hp.x - c.x) as f64 / v.scale,
                    w[1] + (hp.y - c.y) as f64 / v.scale,
                ];
            }
        }
        // the middle button pans, or the left with Space held; a middle double click fits
        let panning = resp.dragged_by(egui::PointerButton::Middle)
            || (d.space_pan && resp.dragged_by(egui::PointerButton::Primary));
        if panning {
            let dl = resp.drag_delta();
            v.center[0] -= dl.x as f64 / v.scale;
            v.center[1] += dl.y as f64 / v.scale;
            if d.space_pan {
                d.space_panned = true;
            }
        }
        d.view = Some(v);
        if resp.double_clicked_by(egui::PointerButton::Middle) {
            d.fit(canvas.size());
            d.say("Command: ZOOM");
            d.say("Extents");
        }
        let v = d.view.expect("fitted");

        // a layout's sheet, under its drawing
        if paper && let Some((lo, hi)) = d.paper() {
            let r = Rect::from_two_pos(to_screen(&v, canvas, lo), to_screen(&v, canvas, hi));
            p.rect_filled(r.translate(vec2(4.0, 4.0)), 0.0, Color32::from_black_alpha(90));
            p.rect_filled(r, 0.0, Color32::WHITE);
        }
        let px_per_mm = 96.0 / 25.4;
        let look = Look {
            center: v.center,
            scale: v.scale,
            lw_per_mm: if d.lwt { px_per_mm } else { 0.0 },
            paper,
            selected: if paper { SELECT_PAPER } else { SELECT },
            hovered: HOVER,
        };
        let (layers, flags) = d.words();
        p.add(egui_wgpu::Callback::new_paint_callback(
            canvas,
            Paint {
                geom: d.geom.clone(),
                look,
                rect: canvas,
                layers,
                flags,
            },
        ));
        // what edits changed, over the rest
        if d.delta.n_items() > 0 {
            let (layers, flags) = d.delta_words();
            p.add(egui_wgpu::Callback::new_paint_callback(
                canvas,
                Paint {
                    geom: d.delta.clone(),
                    look,
                    rect: canvas,
                    layers,
                    flags,
                },
            ));
        }

        // what's under the pointer, the press and the window
        let ink = if paper {
            Color32::from_gray(235)
        } else {
            Color32::from_gray(225)
        };
        let hover = resp.hover_pos().filter(|_| !panning);
        d.cursor = resp.hover_pos().map(|hp| to_world(&v, canvas, hp));
        let shift = ui.input(|i| i.modifiers.shift);
        d.ed.shift = shift;
        if let Some(hp) = hover {
            let w = V2::from(to_world(&v, canvas, hp));
            if d.ed.raw != Some(w) || d.ed.px != v.scale {
                d.hover(w, v.scale);
            }
        }
        let tol = PICKBOX as f64 / v.scale;
        let prompt = d.ed.prompt.clone();
        let selecting = prompt.as_ref().is_none_or(|p| p.wants(want::SEL) || p.wants(want::PICK));
        let pick = |d: &DwgDoc, at: Pos2| d.viewq().pick(V2::from(to_world(&v, canvas, at)), tol);
        if resp.drag_started_by(egui::PointerButton::Primary)
            && !d.space_pan
            && let Some(at) = resp.interact_pointer_pos()
        {
            // where the press began, not where the drag was seen
            let from = ui.input(|i| i.pointer.press_origin()).unwrap_or(at);
            d.press = Some(Press {
                at: from,
                window: true,
            });
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut window_now = None;
        if let (Some(pr), Some(now)) = (d.press, pointer) {
            if (now - pr.at).length() > DRAG_PT && selecting {
                window_now = Some((pr.at, now));
            }
            if resp.drag_stopped_by(egui::PointerButton::Primary) || !ui.input(|i| i.pointer.primary_down()) {
                d.press = None;
                if let Some((a, b)) = window_now {
                    let crossing = b.x < a.x;
                    let (wa, wb) = (V2::from(to_world(&v, canvas, a)), V2::from(to_world(&v, canvas, b)));
                    if d.ed.active() {
                        d.window(wa, wb, crossing);
                    } else {
                        let got: Vec<u32> = d
                            .viewq()
                            .select_box(wa, wb, crossing)
                            .into_iter()
                            .filter(|h| !d.locked(*h))
                            .collect();
                        d.choose(&got, !shift);
                    }
                    window_now = None;
                }
            }
        }
        if resp.clicked_by(egui::PointerButton::Primary)
            && !d.space_pan
            && let Some(at) = resp.interact_pointer_pos()
        {
            let w = V2::from(to_world(&v, canvas, at));
            if d.ed.active() {
                d.click(w, shift);
            } else if !d.click(w, shift) {
                match pick(d, at).filter(|h| !d.locked(*h)) {
                    Some(e) => {
                        let chosen = d.selected.contains(&e);
                        d.choose(&[e], !(shift && chosen));
                    }
                    None if !shift => {
                        d.clear_selection();
                        d.refresh_grips();
                    }
                    None => {}
                }
            }
        }
        // a right click: Enter, as AutoCAD's (with no shortcut menu here)
        if resp.clicked_by(egui::PointerButton::Secondary) && d.ed.active() {
            d.enter();
        }
        let over = match (hover, window_now) {
            (Some(hp), None) if selecting => pick(d, hp),
            _ => None,
        };
        d.set_hover(over);
        if let Some((a, b)) = window_now {
            let r = Rect::from_two_pos(a, b);
            if b.x >= a.x {
                p.rect_filled(r, 0.0, WINDOW.gamma_multiply(0.18));
                p.rect_stroke(r, 0.0, Stroke::new(1.0, WINDOW), StrokeKind::Middle);
            } else {
                p.rect_filled(r, 0.0, CROSSING.gamma_multiply(0.18));
                dashed_rect(&p, r, Stroke::new(1.0, CROSSING));
            }
        }
        let wants_point = prompt.as_ref().is_some_and(|p| p.wants(want::POINT));
        let pick_only = prompt.as_ref().is_some_and(|p| !p.wants(want::POINT));

        // the UCS icon, lower left
        let o = pos2(canvas.left() + 22.0, canvas.bottom() - 22.0);
        p.line_segment(
            [o, o + vec2(34.0, 0.0)],
            Stroke::new(1.5, Color32::from_rgb(0xE0, 0x55, 0x4A)),
        );
        p.line_segment(
            [o, o - vec2(0.0, 34.0)],
            Stroke::new(1.5, Color32::from_rgb(0x4C, 0xC2, 0x6A)),
        );
        p.text(
            o + vec2(38.0, 0.0),
            Align2::LEFT_CENTER,
            "X",
            sans(11.0),
            Color32::from_gray(210),
        );
        p.text(
            o - vec2(0.0, 38.0),
            Align2::CENTER_BOTTOM,
            "Y",
            sans(11.0),
            Color32::from_gray(210),
        );
        p.rect_stroke(
            Rect::from_center_size(o, vec2(6.0, 6.0)),
            0.0,
            Stroke::new(1.0, Color32::from_gray(210)),
            StrokeKind::Middle,
        );

        // the drafting overlay: rubber band, grips, snaps, tracking, dynamic input
        let has_doc = self.dwgdoc.is_some();
        if has_doc {
            self.dwg_overlay(&p, canvas, &v, paper, hover);
        }
        let Some(d) = &mut self.dwgdoc else { return };

        // AutoCAD's crosshairs and pickbox in place of the pointer: the crosshairs where a
        // point is wanted, the pickbox alone where an object is
        if let Some(hp) = resp.hover_pos() {
            if !panning {
                ctx.set_cursor_icon(egui::CursorIcon::None);
                let ink = if paper { Color32::from_gray(25) } else { ink };
                let st = Stroke::new(1.0, ink);
                // the crosshairs on the snapped point
                let at = match (wants_point, d.ed.cursor) {
                    (true, Some(c)) if d.ed.track.is_some() => to_screen(&v, canvas, c.arr()),
                    _ => hp,
                };
                let at = pos2(at.x.round() + 0.5, at.y.round() + 0.5);
                let gap = if wants_point { 0.0 } else { PICKBOX };
                if !pick_only {
                    p.line_segment([at - vec2(CROSSHAIR, 0.0), at - vec2(gap, 0.0)], st);
                    p.line_segment([at + vec2(gap, 0.0), at + vec2(CROSSHAIR, 0.0)], st);
                    p.line_segment([at - vec2(0.0, CROSSHAIR), at - vec2(0.0, gap)], st);
                    p.line_segment([at + vec2(0.0, gap), at + vec2(0.0, CROSSHAIR)], st);
                }
                if !wants_point || pick_only {
                    p.rect_stroke(
                        Rect::from_center_size(at, vec2(PICKBOX * 2.0, PICKBOX * 2.0)),
                        0.0,
                        st,
                        StrokeKind::Middle,
                    );
                }
            } else {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        if d.geom.scene.items.is_empty() && d.delta.n_items() == 0 && !paper {
            p.text(
                canvas.center(),
                Align2::CENTER_CENTER,
                "Nothing drawn in this space",
                sans(13.0),
                t.ink2,
            );
        }
        self.places.push(("dwg:canvas".into(), canvas));
    }

    /// A palette's head: its title, in ForgeCAD's panel style.
    fn dwg_palette_head(&mut self, ui: &mut Ui, r: Rect, title: &str, close: &str) -> bool {
        let t = self.theme;
        let p = ui.painter().clone();
        let head = Rect::from_min_size(r.min, vec2(r.width(), super::HEAD_H));
        p.rect_filled(head, 0.0, t.raised);
        p.hline(r.x_range(), head.bottom(), Stroke::new(1.0, t.line));
        p.text(
            pos2(r.left() + 12.0, head.center().y),
            Align2::LEFT_CENTER,
            title,
            semi(12.0),
            t.ink,
        );
        let x = Rect::from_center_size(pos2(r.right() - 16.0, head.center().y), vec2(20.0, 20.0));
        let resp = ui
            .interact(x, ui.scope_id().with(("palette-close", close)), Sense::click())
            .on_hover_text("Close");
        if resp.hovered() {
            p.rect_filled(x, 3.0, t.hover);
        }
        icons::draw(
            &p,
            pos2(x.left() + 2.0, x.top() + 2.0),
            16.0,
            "close",
            1.5,
            t.ink2,
        );
        self.places.push((format!("dwg:close:{close}"), x));
        resp.clicked()
    }

    /// The Properties palette: the selection's type, layer, colour, line type, lineweight and
    /// geometry; a field that can change is clicked to type it, or picked from its list.
    fn dwg_props(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, t.surface);
        p.vline(r.right(), r.y_range(), Stroke::new(1.0, t.line));
        if self.dwg_palette_head(ui, r, "PROPERTIES", "props")
            && let Some(d) = &mut self.dwgdoc
        {
            d.show_props = false;
        }
        let Some(d) = &self.dwgdoc else { return };
        let rows = d.properties();
        let editable: Vec<bool> = rows.iter().map(|(_, k, _)| d.prop_editable(k)).collect();
        let choices: Vec<Option<Vec<String>>> = rows
            .iter()
            .zip(&editable)
            .map(|((_, k, _), e)| if *e { d.prop_choices(k) } else { None })
            .collect();
        let mut editing = d.prop_edit.clone();
        let what = match d.selected.len() {
            0 => "No selection".to_string(),
            1 => rows.first().map_or_else(String::new, |r| r.2.clone()),
            n => format!("All ({n})"),
        };
        // the selection's kind, as AutoCAD's dropdown shows it
        let bar = Rect::from_min_size(
            pos2(r.left() + 8.0, r.top() + super::HEAD_H + 6.0),
            vec2(r.width() - 16.0, 24.0),
        );
        p.rect_filled(bar, 3.0, t.field);
        p.rect_stroke(
            bar,
            3.0,
            Stroke::new(1.0, super::theme::edge(&t)),
            StrokeKind::Inside,
        );
        p.text(
            pos2(bar.left() + 8.0, bar.center().y),
            Align2::LEFT_CENTER,
            &what,
            sans(12.0),
            t.ink,
        );
        self.places.push(("dwg:props:what".into(), bar));
        let body = Rect::from_min_max(pos2(r.left(), bar.bottom() + 6.0), r.max);
        let split = r.width() * 0.4;
        let mut placed = vec![];
        let mut commit: Option<(String, String)> = None;
        let mut start: Option<(String, String)> = None;
        let mut cancel = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dwg-props")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let w = body.width();
                    let mut group = "";
                    for (i, (g, k, v)) in rows.iter().enumerate() {
                        if *g != group {
                            group = g;
                            let (rr, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
                            ui.painter().rect_filled(rr, 0.0, t.raised);
                            ui.painter().text(
                                pos2(rr.left() + 10.0, rr.center().y),
                                Align2::LEFT_CENTER,
                                *g,
                                semi(12.0),
                                t.ink,
                            );
                        }
                        let can = editable[i];
                        let sense = if can { Sense::click() } else { Sense::hover() };
                        let (rr, resp) = ui.allocate_exact_size(vec2(w, 22.0), sense);
                        let pp = ui.painter().clone();
                        if resp.hovered() {
                            pp.rect_filled(rr, 0.0, t.hover);
                        }
                        let name_r = Rect::from_min_max(
                            rr.min + vec2(14.0, 0.0),
                            pos2(rr.left() + split - 4.0, rr.bottom()),
                        );
                        let val_r = Rect::from_min_max(
                            pos2(rr.left() + split + 6.0, rr.top()),
                            rr.max - vec2(8.0, 0.0),
                        );
                        pp.with_clip_rect(name_r).text(
                            pos2(name_r.left(), rr.center().y),
                            Align2::LEFT_CENTER,
                            k,
                            sans(12.0),
                            if can { t.ink } else { t.ink2 },
                        );
                        pp.vline(rr.left() + split, rr.y_range(), Stroke::new(1.0, t.line));
                        let typing = editing.as_ref().is_some_and(|e| e.0 == *k);
                        if typing {
                            let buf = &mut editing.as_mut().expect("typing").1;
                            let te = ui.put(
                                val_r.shrink2(vec2(0.0, 1.0)),
                                egui::TextEdit::singleline(buf).font(mono(12.0)),
                            );
                            if !te.has_focus() && !te.lost_focus() {
                                te.request_focus();
                            }
                            if te.lost_focus() {
                                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    commit = Some((k.clone(), buf.clone()));
                                } else {
                                    cancel = true;
                                }
                            }
                            placed.push((format!("dwg:propedit:{k}"), val_r));
                        } else if let Some(cs) = &choices[i] {
                            ui.scope_builder(egui::UiBuilder::new().max_rect(val_r), |ui| {
                                egui::ComboBox::from_id_salt(("dwg-prop", k.as_str()))
                                    .width(val_r.width() - 4.0)
                                    .selected_text(egui::RichText::new(v).font(mono(12.0)))
                                    .show_ui(ui, |ui| {
                                        for c in cs {
                                            let it = ui.selectable_label(c == v, c);
                                            placed.push((format!("dwg:propchoice:{k}:{c}"), it.rect));
                                            if it.clicked() {
                                                commit = Some((k.clone(), c.clone()));
                                            }
                                        }
                                    });
                            });
                        } else {
                            pp.with_clip_rect(val_r).text(
                                pos2(val_r.left(), rr.center().y),
                                Align2::LEFT_CENTER,
                                v,
                                mono(12.0),
                                t.ink,
                            );
                            if can && resp.clicked() {
                                start = Some((k.clone(), v.trim_end_matches('\u{b0}').to_string()));
                            }
                        }
                        pp.hline(
                            rr.x_range(),
                            rr.bottom() - 0.5,
                            Stroke::new(1.0, t.line.gamma_multiply(0.5)),
                        );
                        if v.chars().count() > 24 {
                            resp.on_hover_text(v);
                        }
                        placed.push((format!("dwg:prop:{k}"), rr));
                    }
                });
        });
        self.places.extend(placed);
        let Some(d) = &mut self.dwgdoc else { return };
        if let Some((k, v)) = commit {
            d.prop_edit = None;
            if let Err(e) = d.set_prop(&k, &v) {
                d.say(format!("Can't set {k}: {e}"));
            }
        } else if cancel {
            d.prop_edit = None;
        } else if let Some(s) = start {
            d.prop_edit = Some(s);
        } else if editing.is_some() {
            d.prop_edit = editing;
        }
    }

    /// The Layers palette: each layer's on, freeze and lock switches and its colour (each a
    /// change to the drawing, undoable), its line type; New, Delete and Set current; a double
    /// click makes a layer current.
    fn dwg_layers(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, t.surface);
        p.vline(r.left(), r.y_range(), Stroke::new(1.0, t.line));
        if self.dwg_palette_head(ui, r, "LAYERS", "layers")
            && let Some(d) = &mut self.dwgdoc
        {
            d.show_layers = false;
        }
        let Some(d) = &self.dwgdoc else { return };
        let rows = d.layer_rows();
        let chosen = d.layer_sel;
        // the tools: new, delete, current
        let tools_r = Rect::from_min_size(
            pos2(r.left() + 1.0, r.top() + super::HEAD_H + 1.0),
            vec2(r.width() - 1.0, 28.0),
        );
        let mut act: Option<&str> = None;
        let mut placed = vec![];
        let mut x = tools_r.left() + 6.0;
        for (word, icon, key) in [
            ("New layer", "new", "new"),
            ("Delete layer", "delete", "delete"),
            ("Set current", "check", "current"),
        ] {
            let b = Rect::from_min_size(pos2(x, tools_r.top() + 4.0), vec2(22.0, 22.0));
            let resp = ui
                .interact(b, ui.scope_id().with(("dwg-layer-tool", key)), Sense::click())
                .on_hover_text(word);
            if resp.hovered() {
                p.rect_filled(b, 3.0, t.hover);
            }
            icons::draw(&p, pos2(b.left() + 3.0, b.top() + 3.0), 16.0, icon, 1.4, t.ink);
            placed.push((format!("dwg:layertool:{key}"), b));
            if resp.clicked() {
                act = Some(key);
            }
            x += 26.0;
        }
        // the columns' heads
        let cols = Rect::from_min_size(pos2(r.left() + 1.0, tools_r.bottom()), vec2(r.width() - 1.0, 22.0));
        p.rect_filled(cols, 0.0, t.raised);
        let x0 = cols.left();
        for (x, word) in [
            (10.0, "On"),
            (34.0, "Frz"),
            (58.0, "Lck"),
            (84.0, "Name"),
            (r.width() - 96.0, "Linetype"),
        ] {
            p.text(
                pos2(x0 + x, cols.center().y),
                Align2::LEFT_CENTER,
                word,
                sans(11.0),
                t.ink2,
            );
        }
        let body = Rect::from_min_max(pos2(r.left() + 1.0, cols.bottom()), r.max);
        let mut flips: Vec<(u32, Switch)> = vec![];
        let mut colour: Option<(u32, i16)> = None;
        let mut pick_row = None;
        let mut make_current = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dwg-layers")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let w = body.width();
                    for row in &rows {
                        let (rr, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                        let pp = ui.painter().clone();
                        if chosen == Some(row.handle) {
                            pp.rect_filled(rr, 0.0, t.on.gamma_multiply(0.35));
                        } else if resp.hovered() {
                            pp.rect_filled(rr, 0.0, t.hover);
                        }
                        if resp.double_clicked() {
                            make_current = Some(row.handle);
                        } else if resp.clicked() {
                            pick_row = Some(row.handle);
                        }
                        let dim = !(row.on && !row.frozen);
                        let ink = if dim { t.ink2 } else { t.ink };
                        let mut switch = |k: usize, icon: &str, tip: &str, name: &str, which: Switch| {
                            let b = Rect::from_min_size(
                                pos2(rr.left() + 6.0 + k as f32 * 24.0, rr.top() + 2.0),
                                vec2(20.0, 20.0),
                            );
                            let br = ui
                                .interact(b, ui.scope_id().with((name, row.handle)), Sense::click())
                                .on_hover_text(tip);
                            if br.hovered() {
                                pp.rect_filled(b, 3.0, t.raised);
                            }
                            icons::draw(&pp, pos2(b.left() + 2.0, b.top() + 2.0), 16.0, icon, 1.4, ink);
                            placed.push((format!("dwg:layer:{name}:{}", row.name), b));
                            if br.clicked() {
                                flips.push((row.handle, which));
                            }
                        };
                        switch(
                            0,
                            if row.on { "eye" } else { "hide" },
                            if row.on { "On: click to turn off" } else { "Off: click to turn on" },
                            "on",
                            Switch::On,
                        );
                        switch(
                            1,
                            if row.frozen { "freeze" } else { "sun" },
                            if row.frozen { "Frozen: click to thaw" } else { "Thawed: click to freeze" },
                            "freeze",
                            Switch::Freeze,
                        );
                        switch(
                            2,
                            if row.locked { "lock" } else { "unlock" },
                            if row.locked { "Locked: click to unlock" } else { "Unlocked: click to lock" },
                            "lock",
                            Switch::Lock,
                        );
                        // the colour: a swatch that picks another
                        let sw = Rect::from_min_size(pos2(rr.left() + 78.0, rr.center().y - 6.0), vec2(12.0, 12.0));
                        {
                            let id = ui.make_persistent_id(("dwg-layer-colour", row.handle));
                            let sr = ui
                                .interact(sw, id.with("swatch"), Sense::click())
                                .on_hover_text(format!("Color {}: click to change", row.aci));
                            egui::Popup::menu(&sr).id(id).show(|ui| {
                                ui.label(egui::RichText::new("Layer colour").font(sans(11.0)));
                                for line in [[1i16, 2, 3, 4, 5, 6, 7], [8, 9, 250, 251, 252, 253, 254]] {
                                    ui.horizontal(|ui| {
                                        for c in line {
                                            let rgb = forge_dwg::scene::aci_rgb(c);
                                            let (cr, cresp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                                            ui.painter().rect_filled(cr, 2.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                                            if cresp.hovered() {
                                                ui.painter().rect_stroke(cr, 2.0, Stroke::new(1.5, t.ink), StrokeKind::Outside);
                                            }
                                            placed.push((format!("dwg:layercolour:{}:{c}", row.name), cr));
                                            if cresp.on_hover_text(format!("{c}")).clicked() {
                                                colour = Some((row.handle, c));
                                            }
                                        }
                                    });
                                }
                            });
                            placed.push((format!("dwg:layer:colour:{}", row.name), sw));
                        }
                        pp.rect_filled(sw, 2.0, Color32::from_rgb(row.rgb[0], row.rgb[1], row.rgb[2]));
                        pp.rect_stroke(
                            sw,
                            2.0,
                            Stroke::new(1.0, super::theme::edge(&t)),
                            StrokeKind::Outside,
                        );
                        let name_r = Rect::from_min_max(
                            pos2(sw.right() + 8.0, rr.top()),
                            pos2(rr.right() - 100.0, rr.bottom()),
                        );
                        let label = if row.current { format!("{}  (current)", row.name) } else { row.name.clone() };
                        pp.with_clip_rect(name_r).text(
                            pos2(name_r.left(), rr.center().y),
                            Align2::LEFT_CENTER,
                            label,
                            if row.current { semi(12.0) } else { sans(12.0) },
                            ink,
                        );
                        pp.with_clip_rect(Rect::from_min_max(pos2(rr.right() - 96.0, rr.top()), rr.max))
                            .text(
                                pos2(rr.right() - 96.0, rr.center().y),
                                Align2::LEFT_CENTER,
                                &row.ltype,
                                sans(11.0),
                                t.ink2,
                            );
                        placed.push((format!("dwg:layerrow:{}", row.name), rr));
                        resp.on_hover_text(format!(
                            "{}\nLinetype {}\nLineweight {}\n{}\nDouble-click to make it current",
                            row.name,
                            row.ltype,
                            row.weight,
                            if row.plot { "Plots" } else { "Doesn't plot" }
                        ));
                    }
                });
        });
        self.places.extend(placed);
        let Some(d) = &mut self.dwgdoc else { return };
        for (h, which) in flips {
            let _ = d.layer_switch(h, which);
        }
        if let Some((h, c)) = colour {
            let _ = d.layer_colour(h, c);
        }
        if let Some(h) = pick_row {
            d.layer_sel = Some(h);
        }
        if let Some(h) = make_current {
            d.layer_sel = Some(h);
            let _ = d.set_current_layer(h);
        }
        match act {
            Some("new") => {
                if let Ok(h) = d.new_layer() {
                    d.layer_sel = Some(h);
                }
            }
            Some("delete") => {
                if let Some(h) = d.layer_sel
                    && d.delete_layer(h).is_ok()
                {
                    d.layer_sel = None;
                }
            }
            Some("current") => {
                if let Some(h) = d.layer_sel {
                    let _ = d.set_current_layer(h);
                }
            }
            _ => {}
        }
    }

    /// The command line, docked under the canvas: its last lines, and the line being typed.
    fn dwg_command_line(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, t.surface);
        p.hline(r.x_range(), r.top(), Stroke::new(1.0, t.line));
        let Some(d) = &self.dwgdoc else { return };
        let n = d.history.len();
        for (k, line) in d.history[n.saturating_sub(CMD_LINES)..].iter().enumerate() {
            p.with_clip_rect(r).text(
                pos2(
                    r.left() + 12.0,
                    r.top() + 4.0 + k as f32 * CMD_LINE_H + CMD_LINE_H * 0.5,
                ),
                Align2::LEFT_CENTER,
                line,
                mono(12.0),
                t.ink2,
            );
        }
        let input = Rect::from_min_max(
            pos2(r.left() + 6.0, r.bottom() - 26.0),
            pos2(r.right() - 6.0, r.bottom() - 4.0),
        );
        p.rect_filled(input, 3.0, t.field);
        p.rect_stroke(
            input,
            3.0,
            Stroke::new(1.0, super::theme::edge(&t)),
            StrokeKind::Inside,
        );
        icons::draw(
            &p,
            pos2(input.left() + 5.0, input.top() + 3.0),
            16.0,
            "chevron",
            1.5,
            t.ink2,
        );
        let tx = input.left() + 26.0;
        let mut x = tx;
        let cy = input.center().y;
        let mut clicked: Option<&'static str> = None;
        let mut pieces: Vec<(String, Option<&'static str>)> = vec![];
        if let Some(pr) = &d.ed.prompt {
            pieces.push((format!("{} {}", d.ed.name, pr.text), None));
            if !pr.opts.is_empty() {
                pieces.push((" [".into(), None));
                for (k, o) in pr.opts.iter().enumerate() {
                    if k > 0 {
                        pieces.push(("/".into(), None));
                    }
                    pieces.push((o.to_string(), Some(*o)));
                }
                pieces.push(("]".into(), None));
            }
            if let Some(df) = &pr.dflt {
                pieces.push((format!(" <{df}>"), None));
            }
            pieces.push((": ".into(), None));
        } else if let Some(c) = d.pending {
            pieces.push((format!("{c} "), None));
        }
        let mut placed = vec![];
        for (text, opt) in &pieces {
            let col = if opt.is_some() { t.focus } else { t.ink2 };
            let g = ui.fonts_mut(|f| f.layout_no_wrap(text.clone(), mono(12.0), col));
            let w = g.size().x;
            let rr = Rect::from_min_size(pos2(x, cy - g.size().y * 0.5), g.size());
            if let Some(o) = opt {
                let resp = ui.interact(rr, ui.scope_id().with(("dwg-opt", *o)), Sense::click());
                if resp.hovered() {
                    p.rect_filled(rr.expand(1.0), 2.0, t.hover);
                }
                if resp.clicked() {
                    clicked = Some(*o);
                }
                placed.push((format!("dwg:option:{o}"), rr));
            }
            p.with_clip_rect(input).galley(rr.min, g, col);
            x += w;
        }
        if pieces.is_empty() && d.input.is_empty() {
            p.text(
                pos2(tx, cy),
                Align2::LEFT_CENTER,
                "Type a command",
                mono(12.0),
                t.ink2.gamma_multiply(0.7),
            );
        } else {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(d.input.clone(), mono(12.0), t.ink));
            let w = g.size().x;
            p.with_clip_rect(input).galley(pos2(x, cy - g.size().y * 0.5), g, t.ink);
            p.vline(x + w + 1.0, input.y_range().shrink(4.0), Stroke::new(1.0, t.ink));
        }
        self.places.extend(placed);
        if let (Some(o), Some(d)) = (clicked, &mut self.dwgdoc) {
            d.say(o.to_string());
            d.feed(Inp::Key(o));
        }
        self.places.push(("dwg:command".into(), input));
    }

    /// The Model and layout tabs, the cursor's coordinates and the drafting toggles.
    fn dwg_model_row(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, t.raised);
        p.hline(r.x_range(), r.top(), Stroke::new(1.0, t.line));
        let Some(d) = &self.dwgdoc else { return };
        let names = d.tab_names();
        let space = d.space;
        let paper = d.is_paper();
        let cursor = d.cursor;
        let mut x = r.left() + 8.0;
        let mut go = None;
        let down = CornerRadius {
            nw: 0,
            ne: 0,
            sw: 4,
            se: 4,
        };
        for (k, name) in names.iter().enumerate() {
            let w = ui
                .fonts_mut(|f| f.layout_no_wrap(name.clone(), sans(12.0), t.ink))
                .size()
                .x
                + 24.0;
            let tab = Rect::from_min_size(pos2(x, r.top()), vec2(w, r.height() - 3.0));
            let resp = ui.interact(tab, ui.scope_id().with(("dwg-space", k)), Sense::click());
            let on = k == space;
            if on {
                p.rect_filled(tab, down, t.surface);
                p.rect_stroke(tab, down, Stroke::new(1.0, t.line), StrokeKind::Inside);
                p.rect_filled(
                    Rect::from_min_size(pos2(tab.left(), tab.bottom() - 2.0), vec2(w, 2.0)),
                    0.0,
                    t.focus,
                );
            } else if resp.hovered() {
                p.rect_filled(tab, down, t.hover);
            }
            p.text(
                tab.center(),
                Align2::CENTER_CENTER,
                name,
                if on { semi(12.0) } else { sans(12.0) },
                if on { t.ink } else { t.ink2 },
            );
            self.places.push((format!("dwg:space:{name}"), tab));
            if resp.clicked() {
                go = Some(k);
            }
            x += w + 2.0;
        }
        // the toggles, right to left, and the coordinates before them
        let mut rx = r.right() - 8.0;
        let mut flip = None;
        let mut show_osnap = false;
        for (name, tip) in TOGGLES.iter().rev() {
            let w = ui
                .fonts_mut(|f| f.layout_no_wrap(name.to_string(), semi(11.0), t.ink))
                .size()
                .x
                + 14.0;
            let b = Rect::from_min_max(pos2(rx - w, r.top() + 4.0), pos2(rx, r.bottom() - 4.0));
            let works = true;
            let on = self.dwgdoc.as_ref().is_some_and(|d| Self::dwg_toggle_on(d, name));
            let resp = ui
                .interact(b, ui.scope_id().with(("dwg-toggle", *name)), Sense::click())
                .on_hover_text(if works {
                    tip.to_string()
                } else {
                    format!("{tip}: not yet")
                });
            if on {
                p.rect_filled(b, 3.0, t.on);
            } else if resp.hovered() {
                p.rect_filled(b, 3.0, t.hover);
            }
            p.text(
                b.center(),
                Align2::CENTER_CENTER,
                *name,
                semi(11.0),
                if on {
                    t.on_ink
                } else if works {
                    t.ink
                } else {
                    t.ink2.gamma_multiply(0.7)
                },
            );
            self.places.push((format!("dwg:toggle:{name}"), b));
            if resp.clicked() {
                flip = Some(*name);
            }
            if resp.secondary_clicked() && *name == "OSNAP" {
                show_osnap = true;
            }
            rx -= w + 2.0;
        }
        let space_word = if paper { "PAPER" } else { "MODEL" };
        let mw = ui
            .fonts_mut(|f| f.layout_no_wrap(space_word.into(), semi(11.0), t.ink))
            .size()
            .x
            + 14.0;
        let mb = Rect::from_min_max(
            pos2(rx - mw - 6.0, r.top() + 4.0),
            pos2(rx - 6.0, r.bottom() - 4.0),
        );
        p.rect_stroke(mb, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        p.text(mb.center(), Align2::CENTER_CENTER, space_word, semi(11.0), t.ink);
        let coords = match cursor {
            Some(c) => format!("{:.4}, {:.4}, 0.0000", c[0], c[1]),
            None => String::new(),
        };
        let cr = p.text(
            pos2(mb.left() - 14.0, r.center().y),
            Align2::RIGHT_CENTER,
            coords,
            mono(12.0),
            t.ink,
        );
        self.places.push(("dwg:coords".into(), cr));
        if let Some(k) = go
            && let Some(d) = &mut self.dwgdoc
        {
            d.switch_space(k);
        }
        if let Some(name) = flip {
            self.dwg_toggle(name);
        }
        if show_osnap && let Some(d) = &mut self.dwgdoc {
            d.ed.show_osnap = true;
        }
    }

    /// The document tabs and the status bar, as every document has them.
    fn dwg_bottom(&mut self, ui: &mut Ui, s: Rect, bottom: f32) {
        self.close_prompt(&ui.ctx().clone());
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(
            Rect::from_min_size(pos2(0.0, bottom), vec2(s.width(), super::DOCS_H)),
            0.0,
            t.surface,
        );
        p.hline(0.0..=s.width(), bottom, Stroke::new(1.0, t.line));
        if !self.doc_tabs.is_empty() {
            self.doc_strip(
                ui,
                Rect::from_min_size(pos2(0.0, bottom + 1.0), vec2(s.width(), super::DOCS_H - 1.0)),
            );
        }
        let st = Rect::from_min_size(
            pos2(0.0, bottom + super::DOCS_H),
            vec2(s.width(), super::STATUS_H),
        );
        p.rect_filled(st, 0.0, t.bar);
        let Some(d) = &self.dwgdoc else { return };
        let said = d.history.last().cloned().unwrap_or_default();
        let status = if self.status.is_empty() {
            said
        } else {
            self.status.clone()
        };
        p.text(
            pos2(10.0, st.center().y),
            Align2::LEFT_CENTER,
            status,
            sans(12.0),
            Color32::WHITE,
        );
        let n = d.drawing.warnings.len();
        let edit = match d.ed.times.last() {
            Some((what, ms)) => format!(" · {what} {ms:.1} ms"),
            None => String::new(),
        };
        let words = format!(
            "{}{edit} · {} items",
            if n == 0 {
                "all objects decoded".to_string()
            } else {
                format!("{n} kept as read")
            },
            d.geom.scene.items.len() + d.delta.n_items()
        );
        let g = ui.fonts_mut(|f| f.layout_no_wrap(words.clone(), sans(12.0), Color32::WHITE));
        let r = Rect::from_min_max(
            pos2(s.width() - 10.0 - g.size().x - 24.0, st.top() + 2.0),
            pos2(s.width() - 6.0, st.bottom() - 2.0),
        );
        let resp = ui
            .interact(r, ui.scope_id().with("dwg-warnings"), Sense::click())
            .on_hover_text("Objects this reader keeps as they were read, not decoded: click for the list");
        if resp.hovered() || d.show_warnings {
            p.rect_filled(r, 3.0, Color32::from_white_alpha(28));
        }
        let ink = if n > 0 { super::fg::WARN } else { Color32::WHITE };
        icons::draw(
            &p,
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
        self.places.push(("dwg:warnings".into(), r));
        if resp.clicked()
            && let Some(d) = &mut self.dwgdoc
        {
            d.show_warnings = !d.show_warnings;
        }
    }

    /// The objects kept raw, listed.
    fn dwg_warnings(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.dwgdoc else { return };
        if !d.show_warnings {
            return;
        }
        let area = super::dialog_area(ctx);
        let mut open = true;
        egui::Window::new("Objects kept as read")
            .id(egui::Id::unique("dwg-warnings-window"))
            .frame(super::fgasm::dialog_frame(&self.theme))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(520.0, 320.0))
            .default_pos(pos2(area.right() - 560.0, area.bottom() - 380.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                if d.drawing.warnings.is_empty() {
                    ui.label("Every object in this file was decoded.");
                    return;
                }
                ui.label(
                    egui::RichText::new(
                        "These objects are kept byte for byte as they were read and written back unchanged on Save.",
                    )
                    .font(sans(12.0)),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for w in &d.drawing.warnings {
                        ui.label(egui::RichText::new(w).font(mono(11.0)));
                    }
                });
            });
        if !open {
            d.show_warnings = false;
        }
    }

    /// A DWG document comes up in the window.
    pub fn open_dwg(&mut self, d: DwgDoc) {
        self.name = d.name.clone();
        self.dwgdoc = Some(d);
        self.tab = 0;
    }
}
