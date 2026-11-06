//! A DWG document's window, laid out as AutoCAD's in ForgeCAD's colours: the ribbon's Home,
//! Insert, Annotate, View, Manage and Output tabs; the Properties palette on the left and the
//! Layers palette on the right; the canvas; the command line docked under it; the Model and
//! layout tabs with the cursor's coordinates and the drafting toggles; the document tabs and the
//! status bar.

use super::dwgdoc::{Cmd, DwgDoc, Press, Run, TOGGLES, View};
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

fn to_world(v: &View, canvas: Rect, p: Pos2) -> [f64; 2] {
    let c = canvas.center();
    [
        v.center[0] + (p.x - c.x) as f64 / v.scale,
        v.center[1] - (p.y - c.y) as f64 / v.scale,
    ]
}

fn to_screen(v: &View, canvas: Rect, w: [f64; 2]) -> Pos2 {
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
                Cmd::Run(name) => self.dwg_run(name),
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
        {
            let Some(d) = &mut self.dwgdoc else { return };
            d.space_pan = space_held && d.input.is_empty();
            for e in &events {
                match e {
                    egui::Event::Text(s) if s != " " && !s.chars().any(char::is_control) => {
                        d.input.push_str(s);
                        d.recall = None;
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
                            K::Num1 if modifiers.command => {
                                d.show_props = !d.show_props;
                            }
                            K::Enter => submit = Some(std::mem::take(&mut d.input)),
                            K::Space if !d.input.is_empty() => submit = Some(std::mem::take(&mut d.input)),
                            K::Backspace => {
                                d.input.pop();
                            }
                            K::Escape => escape = true,
                            K::Delete if d.input.is_empty() && !d.selected.is_empty() => {
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
            }
        }
        if save {
            self.dwg_run("QSAVE");
        }
        for name in toggles {
            self.dwg_toggle(name);
        }
        if let Some(line) = submit {
            if let Some(d) = &mut self.dwgdoc {
                if !line.trim().is_empty() {
                    d.typed.push(line.trim().to_string());
                }
                d.recall = None;
            }
            self.dwg_run(&line);
        }
    }

    fn dwg_toggle(&mut self, name: &str) {
        let Some(d) = &mut self.dwgdoc else { return };
        if name == "LWT" {
            d.lwt = !d.lwt;
            d.say(format!("<Lineweight {}>", if d.lwt { "on" } else { "off" }));
        } else {
            d.say(format!(
                "<{name}> not yet: drafting aids come with the editing commands"
            ));
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
        let (layers, flags) = d.words();
        let px_per_mm = 96.0 / 25.4;
        p.add(egui_wgpu::Callback::new_paint_callback(
            canvas,
            Paint {
                geom: d.geom.clone(),
                look: Look {
                    center: v.center,
                    scale: v.scale,
                    lw_per_mm: if d.lwt { px_per_mm } else { 0.0 },
                    paper,
                    selected: if paper { SELECT_PAPER } else { SELECT },
                    hovered: HOVER,
                },
                rect: canvas,
                layers,
                flags,
            },
        ));

        // what's under the pointer, the press and the window
        let ink = if paper {
            Color32::from_gray(235)
        } else {
            Color32::from_gray(225)
        };
        let hover = resp.hover_pos().filter(|_| !panning);
        d.cursor = resp.hover_pos().map(|hp| to_world(&v, canvas, hp));
        let tol = PICKBOX as f64 / v.scale;
        let shift = ui.input(|i| i.modifiers.shift);
        let geom = d.geom.clone();
        let pick = |d: &DwgDoc, at: Pos2| {
            let shown = |l: usize| d.layer_shown(l);
            geom.pick(to_world(&v, canvas, at), tol, &shown)
        };
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
            if (now - pr.at).length() > DRAG_PT {
                window_now = Some((pr.at, now));
            }
            if resp.drag_stopped_by(egui::PointerButton::Primary) || !ui.input(|i| i.pointer.primary_down()) {
                d.press = None;
                if let Some((a, b)) = window_now {
                    let crossing = b.x < a.x;
                    let shown = |l: usize| d.layer_shown(l);
                    let got =
                        geom.select_box(to_world(&v, canvas, a), to_world(&v, canvas, b), crossing, &shown);
                    d.select(&got, !shift);
                    window_now = None;
                }
            }
        }
        if resp.clicked_by(egui::PointerButton::Primary)
            && !d.space_pan
            && let Some(at) = resp.interact_pointer_pos()
        {
            match pick(d, at) {
                Some(e) => {
                    let chosen = d.selected.contains(&e);
                    d.select(&[e], !(shift && chosen));
                }
                None if !shift => d.clear_selection(),
                None => {}
            }
        }
        let over = match (hover, window_now) {
            (Some(hp), None) => pick(d, hp),
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

        // the UCS icon, lower left
        let o = pos2(canvas.left() + 22.0, canvas.bottom() - 22.0);
        let ucs = if paper {
            Color32::from_gray(30)
        } else {
            Color32::from_gray(200)
        };
        let ucs_ink = if paper { Color32::from_gray(235) } else { ucs };
        let _ = ucs_ink;
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

        // AutoCAD's crosshairs and pickbox in place of the pointer
        if let Some(hp) = resp.hover_pos() {
            if !panning {
                ctx.set_cursor_icon(egui::CursorIcon::None);
                let ink = if paper { Color32::from_gray(25) } else { ink };
                let st = Stroke::new(1.0, ink);
                let at = pos2(hp.x.round() + 0.5, hp.y.round() + 0.5);
                p.line_segment([at - vec2(CROSSHAIR, 0.0), at - vec2(PICKBOX, 0.0)], st);
                p.line_segment([at + vec2(PICKBOX, 0.0), at + vec2(CROSSHAIR, 0.0)], st);
                p.line_segment([at - vec2(0.0, CROSSHAIR), at - vec2(0.0, PICKBOX)], st);
                p.line_segment([at + vec2(0.0, PICKBOX), at + vec2(0.0, CROSSHAIR)], st);
                p.rect_stroke(
                    Rect::from_center_size(at, vec2(PICKBOX * 2.0, PICKBOX * 2.0)),
                    0.0,
                    st,
                    StrokeKind::Middle,
                );
            } else {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        if !d.geom.scene.items.is_empty() || paper {
            // nothing to say
        } else {
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
    /// geometry, read only for now.
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
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dwg-props")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let w = body.width();
                    let mut group = "";
                    for (g, k, v) in &rows {
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
                        let (rr, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
                        let pp = ui.painter();
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
                            t.ink2,
                        );
                        pp.vline(rr.left() + split, rr.y_range(), Stroke::new(1.0, t.line));
                        pp.with_clip_rect(val_r).text(
                            pos2(val_r.left(), rr.center().y),
                            Align2::LEFT_CENTER,
                            v,
                            mono(12.0),
                            t.ink,
                        );
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
    }

    /// The Layers palette: each layer's on and freeze switches (shown here only, not written),
    /// its lock, colour, line type and lineweight.
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
        // the columns' heads
        let cols = Rect::from_min_size(
            pos2(r.left() + 1.0, r.top() + super::HEAD_H + 1.0),
            vec2(r.width() - 1.0, 22.0),
        );
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
        let mut flips = vec![];
        let mut placed = vec![];
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dwg-layers")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let w = body.width();
                    for row in &rows {
                        let (rr, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
                        let pp = ui.painter().clone();
                        if resp.hovered() {
                            pp.rect_filled(rr, 0.0, t.hover);
                        }
                        let dim = !(row.on && !row.frozen);
                        let ink = if dim { t.ink2 } else { t.ink };
                        let mut switch =
                            |k: usize, icon: &str, tip: &str, name: &str, click: Option<bool>| {
                                let b = Rect::from_min_size(
                                    pos2(rr.left() + 6.0 + k as f32 * 24.0, rr.top() + 2.0),
                                    vec2(20.0, 20.0),
                                );
                                let sense = if click.is_some() {
                                    Sense::click()
                                } else {
                                    Sense::hover()
                                };
                                let br = ui
                                    .interact(b, ui.scope_id().with((name, row.index)), sense)
                                    .on_hover_text(tip);
                                if br.hovered() && click.is_some() {
                                    pp.rect_filled(b, 3.0, t.raised);
                                }
                                icons::draw(&pp, pos2(b.left() + 2.0, b.top() + 2.0), 16.0, icon, 1.4, ink);
                                placed.push((format!("dwg:layer:{name}:{}", row.name), b));
                                if br.clicked()
                                    && let Some(f) = click
                                {
                                    flips.push((row.index, f));
                                }
                            };
                        switch(
                            0,
                            if row.on { "eye" } else { "hide" },
                            if row.on {
                                "On: click to turn off"
                            } else {
                                "Off: click to turn on"
                            },
                            "on",
                            Some(false),
                        );
                        switch(
                            1,
                            if row.frozen { "freeze" } else { "sun" },
                            if row.frozen {
                                "Frozen: click to thaw"
                            } else {
                                "Thawed: click to freeze"
                            },
                            "freeze",
                            Some(true),
                        );
                        switch(
                            2,
                            if row.locked { "lock" } else { "unlock" },
                            if row.locked { "Locked" } else { "Unlocked" },
                            "lock",
                            None,
                        );
                        let sw = Rect::from_min_size(
                            pos2(rr.left() + 78.0, rr.center().y - 6.0),
                            vec2(12.0, 12.0),
                        );
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
                        pp.with_clip_rect(name_r).text(
                            pos2(name_r.left(), rr.center().y),
                            Align2::LEFT_CENTER,
                            &row.name,
                            sans(12.0),
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
                        resp.on_hover_text(format!(
                            "{}\nLinetype {}\nLineweight {}\n{}",
                            row.name,
                            row.ltype,
                            row.weight,
                            if row.plot { "Plots" } else { "Doesn't plot" }
                        ));
                    }
                });
        });
        self.places.extend(placed);
        if let Some(d) = &mut self.dwgdoc {
            for (i, frozen) in flips {
                d.flip_layer(i, frozen);
            }
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
        let prompt = match d.pending {
            Some(c) => format!("{c} "),
            None => String::new(),
        };
        let shown = if d.input.is_empty() && d.pending.is_none() {
            None
        } else {
            Some(format!("{prompt}{}", d.input))
        };
        let tx = input.left() + 26.0;
        match shown {
            Some(s) => {
                let g = ui.fonts_mut(|f| f.layout_no_wrap(s, mono(12.0), t.ink));
                let w = g.size().x;
                p.with_clip_rect(input)
                    .galley(pos2(tx, input.center().y - g.size().y * 0.5), g, t.ink);
                p.vline(tx + w + 1.0, input.y_range().shrink(4.0), Stroke::new(1.0, t.ink));
            }
            None => {
                p.text(
                    pos2(tx, input.center().y),
                    Align2::LEFT_CENTER,
                    "Type a command",
                    mono(12.0),
                    t.ink2.gamma_multiply(0.7),
                );
            }
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
        let lwt = d.lwt;
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
        for (name, tip) in TOGGLES.iter().rev() {
            let w = ui
                .fonts_mut(|f| f.layout_no_wrap(name.to_string(), semi(11.0), t.ink))
                .size()
                .x
                + 14.0;
            let b = Rect::from_min_max(pos2(rx - w, r.top() + 4.0), pos2(rx, r.bottom() - 4.0));
            let works = *name == "LWT";
            let on = works && lwt;
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
        let words = format!(
            "{} · {} items",
            if n == 0 {
                "all objects decoded".to_string()
            } else {
                format!("{n} kept as read")
            },
            d.geom.scene.items.len()
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
