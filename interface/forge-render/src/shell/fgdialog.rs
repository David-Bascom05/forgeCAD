//! A Forge part's ribbon tool as Inventor shows it: a dialog over the view with the feature's
//! icon and name, its choices as toggles, a selection row per pick (the one taking clicks lit),
//! its values with their units, OK, Cancel and +. Enter is OK and Esc Cancel; the status bar
//! says what to pick next. The part shows what OK would make while it's open.

use super::dialog::{DIALOG_W, FIELD_AT, dropdown, heading, pick_row, text_toggles, toggle_row};
use super::fg::FgAct;
use super::fgtool::{FgTool, SlotKind as K};
use super::theme::{mono, sans, semi};
use super::{Shell, panel};
use egui::{Align2, Color32, Rect, Sense, Stroke, pos2, vec2};
use forge_lang::error::Op;
use forge_lang::values::Dim;

/// What a param is called in the dialog: Inventor's word for it.
pub fn param_label(name: &str) -> &'static str {
    match name {
        "d" => "Distance",
        "r" => "Radius",
        "t" => "Thickness",
        "ang" => "Angle",
        "n" => "Count",
        "step" => "Spacing",
        "n2" => "Count 2",
        "step2" => "Spacing 2",
        "taper" => "Taper",
        "depth" => "Depth",
        "point" => "Drill Point",
        "cbore_d" => "C'bore Dia",
        "cbore_depth" => "C'bore Depth",
        "csink_d" => "C'sink Dia",
        "csink_angle" => "C'sink Angle",
        "size" => "Height",
        "wrap" => "Wrap Radius",
        "pitch" => "Pitch",
        "turns" => "Revolutions",
        "height" => "Height",
        "r_start" => "Start Radius",
        "r_end" => "End Radius",
        "k" => "K-Factor",
        _ => "Value",
    }
}

fn word_label(w: &str) -> &'static str {
    match w {
        "text" => "Text",
        "font" => "Font",
        "screw" => "Size",
        "fit" => "Fit",
        "spec" => "Thread",
        "length" => "Length",
        "file" => "File",
        _ => "Value",
    }
}

fn flag_label(f: &str) -> String {
    let mut c = f.chars();
    c.next()
        .map_or(String::new(), |x| x.to_uppercase().chain(c).collect())
}

fn unit(d: Dim) -> &'static str {
    match d {
        Dim::Length => "mm",
        Dim::Angle => "deg",
        Dim::Count => "ul",
        _ => "",
    }
}

/// What the ribbon calls a tool kind, in the dialog's title.
pub fn title(kind: &str) -> &'static str {
    match kind {
        "pattern" => "Rectangular Pattern",
        "circular" => "Circular Pattern",
        "mirror" => "Mirror",
        "sketchpattern" => "Sketch Driven Pattern",
        "emboss" => "Emboss",
        "import" => "Import",
        "split" => "Split",
        "thicken" => "Thicken/Offset",
        "direct" => "Direct Edit",
        "deleteface" => "Delete Face",
        "replaceface" => "Replace Face",
        "offsetface" => "Offset Face",
        "moveface" => "Move Face",
        "facefillet" => "Face Fillet",
        "movebody" => "Move Bodies",
        "copybody" => "Copy Bodies",
        "plane" => "Work Plane",
        "axis" => "Work Axis",
        "point" => "Work Point",
        "ucs" => "UCS",
        "extrude" => "Extrude",
        "revolve" => "Revolve",
        "hole" => "Hole",
        "fillet" => "Fillet",
        "chamfer" => "Chamfer",
        "shell" => "Shell",
        "draft" => "Face Draft",
        "thread" => "Thread",
        "combine" => "Combine",
        "sweep" => "Sweep",
        "loft" => "Loft",
        "coil" => "Coil",
        "rib" => "Rib",
        "sheet" => "Sheet",
        "flange" => "Flange",
        "cutout" => "Cutout",
        "unfold" => "Unfold",
        _ => "Feature",
    }
}

/// The icon a tool kind shows.
pub fn tool_icon(kind: &str) -> &'static str {
    match kind {
        "pattern" => "rect",
        "circular" => "circular",
        "mirror" => "mirror",
        "sketchpattern" => "sketchpattern",
        "emboss" => "emboss",
        "import" => "file",
        "direct" | "moveface" => "move",
        "deleteface" => "delete",
        "replaceface" => "replaceface",
        "offsetface" => "offset",
        "facefillet" => "fillet",
        "movebody" => "move",
        "copybody" => "copybody",
        k => panel::icon(k),
    }
}

/// A value: its word, its field, its unit.
fn value_row(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    label: &str,
    text: &mut String,
    unit: &str,
    name: String,
    places: &mut Vec<(String, Rect)>,
) {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 30.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let field = Rect::from_min_size(
        pos2(row.left() + FIELD_AT, row.top() + 2.0),
        vec2(DIALOG_W - FIELD_AT, 26.0),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(text)
            .font(mono(13.0))
            .desired_width(field.width() - 14.0)
            .margin(vec2(7.0, 5.0)),
    );
    let bare = text.trim().parse::<f64>().is_ok();
    if bare && !unit.is_empty() {
        ui.painter().text(
            pos2(field.right() - 7.0, field.center().y),
            Align2::RIGHT_CENTER,
            unit,
            mono(11.0),
            t.ink2,
        );
    }
    places.push((name, field));
}

/// A chip: a pick to take back out, or a choice to take.
fn chip(ui: &mut egui::Ui, t: &super::theme::Theme, text: &str, on: bool) -> egui::Response {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), mono(11.0), t.ink));
    let (r, resp) = ui.allocate_exact_size(g.size() + vec2(12.0, 8.0), Sense::click());
    let fill = if on {
        t.on
    } else if resp.hovered() {
        t.hover
    } else {
        t.raised
    };
    ui.painter().rect_filled(r, 3.0, fill);
    ui.painter()
        .rect_stroke(r, 3.0, Stroke::new(1.0, t.line), egui::StrokeKind::Inside);
    ui.painter()
        .galley(r.min + vec2(6.0, 4.0), g, if on { t.on_ink } else { t.ink });
    resp
}

/// What a slot can take that the view can't give: sketch entities, features, origin planes.
fn choices_for(fg: &super::fg::FgPanel, kind: K) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let of_kind = |ks: &[&str]| -> Vec<String> {
        fg.feats
            .iter()
            .filter(|f| ks.contains(&f.kind.as_str()))
            .map(|f| f.id.clone())
            .collect()
    };
    for sk in &fg.sketches {
        match kind {
            K::Profile | K::Profiles => {
                out.push(sk.id.clone());
                out.extend(sk.closed.iter().cloned());
            }
            K::Line => out.extend(sk.lines.iter().cloned()),
            K::Path => {
                out.extend(sk.lines.iter().cloned());
                out.extend(sk.curves.iter().cloned());
                out.extend(sk.closed.iter().cloned());
            }
            K::Points => out.extend(sk.points.iter().cloned()),
            K::Point => out.extend(sk.points.iter().filter(|p| !p.ends_with(".points")).cloned()),
            K::Dir => out.extend(sk.lines.iter().map(|l| format!("{l}.dir"))),
            K::Axis => out.extend(sk.lines.iter().cloned()),
            K::Plane => out.push(sk.id.clone()),
            _ => {}
        }
    }
    match kind {
        K::Plane => {
            out.extend(["xy", "xz", "yz"].map(String::from));
            out.extend(of_kind(&["workplane"]));
        }
        K::Axis => {
            out.extend(of_kind(&["workaxis"]));
            out.extend(
                of_kind(&["revolve", "hole", "coil"])
                    .into_iter()
                    .map(|f| format!("{f}.axis")),
            );
        }
        K::Dir => out.extend(of_kind(&["workaxis"])),
        K::Point | K::Points => out.extend(of_kind(&["workpoint"])),
        K::Feature | K::Features => {
            let sketches: Vec<&str> = fg.sketches.iter().map(|s| s.id.as_str()).collect();
            out.extend(
                super::fg::statements(&fg.feats)
                    .into_iter()
                    .filter(|s| !sketches.contains(&s.as_str())),
            );
        }
        _ => {}
    }
    out
}

/// What the dialog was asked to do this frame.
#[derive(Clone, Copy, PartialEq)]
enum Asked {
    Nothing,
    Ok,
    Again,
    Cancel,
}

impl Shell {
    /// The open Forge tool's dialog, over the view's top left.
    pub(super) fn fg_dialog(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some(mut fg) = self.fg.take() else { return };
        let Some(mut tool) = fg.tool.take() else {
            self.fg = Some(fg);
            return;
        };
        let frame = egui::Frame::new()
            .fill(t.surface)
            .stroke(Stroke::new(1.0, t.line))
            .corner_radius(6)
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(90),
            })
            .inner_margin(12);
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut asked = Asked::Nothing;
        let shown = egui::Window::new(title(tool.kind))
            .id(egui::Id::unique("fg-tool-dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            // over the browser, as Inventor's property panel stands, the view left clear
            .default_pos(pos2(4.0, v.top() + 4.0))
            .fixed_size(vec2(DIALOG_W, 0.0))
            .frame(frame)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                asked = Self::fg_dialog_body(ui, &t, &fg, &mut tool, &mut places);
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
            places.push(("fg:dialog".into(), r.response.rect));
        }
        // Enter is OK, wherever the keyboard is
        if asked == Asked::Nothing && ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.menu_open() {
            asked = Asked::Ok;
        }
        match asked {
            Asked::Ok | Asked::Again => match tool.text() {
                Ok(text) => {
                    tool.again = asked == Asked::Again;
                    fg.out.push(FgAct::Tool(Op::AddFeature { before: None, text }));
                }
                Err(e) => tool.error = Some(e),
            },
            Asked::Cancel => {
                self.status.clear();
                self.places.extend(places);
                self.fg = Some(fg);
                return;
            }
            Asked::Nothing => {}
        }
        self.status = match &tool.error {
            Some(e) => format!("{}: {}", e.kind(), e.text()),
            None => tool.prompt(),
        };
        fg.tool = Some(tool);
        self.places.extend(places);
        self.fg = Some(fg);
    }

    fn fg_dialog_body(
        ui: &mut egui::Ui,
        t: &super::theme::Theme,
        fg: &super::fg::FgPanel,
        tool: &mut FgTool,
        places: &mut Vec<(String, Rect)>,
    ) -> Asked {
        // the feature, by its icon and name; its id beside
        let (head, _) = ui.allocate_exact_size(vec2(DIALOG_W, 30.0), Sense::hover());
        super::icons::draw(
            ui.painter(),
            head.left_top() + vec2(0.0, 3.0),
            24.0,
            tool_icon(tool.kind),
            1.5,
            t.ink,
        );
        ui.painter().text(
            pos2(head.left() + 32.0, head.center().y),
            Align2::LEFT_CENTER,
            title(tool.kind),
            semi(15.0),
            t.ink,
        );
        let name_at = Rect::from_min_size(pos2(head.right() - 92.0, head.top() + 3.0), vec2(92.0, 24.0));
        ui.put(
            name_at,
            egui::TextEdit::singleline(&mut tool.name)
                .font(mono(11.0))
                .desired_width(80.0)
                .margin(vec2(5.0, 4.0)),
        );
        places.push(("fg:tool:name".into(), name_at));

        // the choice that shapes the form first, as Inventor's big toggles
        let mut chose = None;
        for (ci, c) in tool.choices.iter().enumerate().take(2) {
            {
                let name = format!("fg:tool:choice:{}", c.name);
                if ci > 0 {
                    heading(ui, c.name, t);
                }
                let k = if c.items.len() <= 5 {
                    let items: Vec<(&str, &str, bool)> = c
                        .items
                        .iter()
                        .enumerate()
                        .map(|(k, (w, icon))| (*icon, *w, k == c.on))
                        .collect();
                    toggle_row(ui, t, &items, &name, places)
                } else {
                    let items: Vec<(&str, bool)> = c
                        .items
                        .iter()
                        .enumerate()
                        .map(|(k, (w, _))| (*w, k == c.on))
                        .collect();
                    let (a, b) = items.split_at(items.len().div_ceil(2));
                    let first = text_toggles(ui, t, a, &name, places);
                    let second = text_toggles(ui, t, b, &name, places).map(|k| k + a.len());
                    first.or(second)
                };
                if let Some(k) = k {
                    chose = Some((c.name, k));
                }
            }
        }

        ui.add_space(2.0);
        heading(ui, "Input Geometry", t);
        let mut activate = None;
        let mut gone = None;
        let mut take = None;
        for (k, s) in tool.slots.iter().enumerate() {
            let waiting = tool.active == k;
            let label = if s.optional {
                format!("{} (optional)", s.label)
            } else {
                s.label.to_string()
            };
            let clicked = pick_row(ui, &label, s.picks.len(), waiting, s.kind.prompt(), t, places);
            if let Some(r) = places.last().map(|p| p.1) {
                places.push((format!("fg:tool:slot:{}", s.label), r));
            }
            if clicked {
                activate = Some(k);
            }
            // what's picked, each taken back out by its ×; what can be chosen, under the one
            // taking picks
            if !s.picks.is_empty() || waiting {
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width(DIALOG_W);
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    for (i, p) in s.picks.iter().enumerate() {
                        let r = chip(ui, t, &format!("{p}  ×"), true);
                        places.push((format!("fg:tool:pick:{}:{i}", s.label), r.rect));
                        if r.clicked() {
                            gone = Some((k, i));
                        }
                    }
                    if waiting {
                        for c in choices_for(fg, s.kind) {
                            if s.picks.contains(&c) {
                                continue;
                            }
                            let r = chip(ui, t, &c, false);
                            places.push((format!("fg:choice:{}:{c}", s.label), r.rect));
                            if r.clicked() {
                                take = Some((k, c));
                            }
                        }
                    }
                });
            }
        }
        if let Some(k) = activate {
            tool.active = k;
        }
        if let Some((k, i)) = gone {
            tool.slots[k].picks.remove(i);
        }
        if let Some((k, c)) = take {
            tool.active = k;
            tool.take(Ok(c));
        }

        let more = tool.choices.len() > 2;
        if !tool.params.is_empty() || !tool.words.is_empty() || !tool.flags.is_empty() || more {
            heading(ui, "Behavior", t);
        }
        for c in tool.choices.iter().skip(2) {
            let items: Vec<(&str, bool)> = c
                .items
                .iter()
                .enumerate()
                .map(|(k, (w, _))| (*w, k == c.on))
                .collect();
            ui.label(egui::RichText::new(c.name).font(sans(12.0)).color(t.ink2));
            if let Some(k) = text_toggles(ui, t, &items, &format!("fg:tool:choice:{}", c.name), places) {
                chose = Some((c.name, k));
            }
        }
        let options = tool.options.clone();
        for (w, text) in &mut tool.words {
            match options.iter().find(|o| o.0 == *w) {
                Some((_, items)) if !items.is_empty() => {
                    let rows: Vec<(String, String, bool)> =
                        items.iter().map(|i| (i.clone(), i.clone(), false)).collect();
                    let before = ui.min_rect().bottom();
                    if let Some(v) = dropdown(ui, t, word_label(w), text, &rows) {
                        *text = v;
                    }
                    let r = Rect::from_min_max(
                        pos2(ui.min_rect().left() + FIELD_AT, before),
                        pos2(ui.min_rect().left() + DIALOG_W, ui.min_rect().bottom()),
                    );
                    places.push((format!("fg:tool:word:{w}"), r));
                }
                _ => value_row(
                    ui,
                    t,
                    word_label(w),
                    text,
                    "",
                    format!("fg:tool:word:{w}"),
                    places,
                ),
            }
        }
        for p in &mut tool.params {
            value_row(
                ui,
                t,
                param_label(p.name),
                &mut p.text,
                unit(p.dim),
                format!("fg:tool:param:{}", p.name),
                places,
            );
        }
        for (f, on) in &mut tool.flags {
            let r = ui.checkbox(
                on,
                egui::RichText::new(flag_label(f)).font(sans(13.0)).color(t.ink),
            );
            places.push((format!("fg:tool:flag:{f}"), r.rect));
        }
        if let Some((name, k)) = chose {
            tool.choose(name, k);
        }

        if let Some(e) = &tool.error {
            ui.label(egui::RichText::new(e.text()).font(sans(12.0)).color(t.error));
        }
        if let Some(n) = &tool.note {
            ui.label(egui::RichText::new(n).font(sans(12.0)).color(t.error));
        }

        // OK and Cancel at the right, + (OK, then another) at the left, as Inventor has them
        ui.add_space(4.0);
        let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 28.0), Sense::hover());
        let mut asked = Asked::Nothing;
        let buttons = [
            (
                "+",
                Rect::from_min_size(row.left_top(), vec2(28.0, 28.0)),
                "fg:tool:again",
                Asked::Again,
            ),
            (
                "OK",
                Rect::from_min_size(pos2(row.right() - 156.0, row.top()), vec2(74.0, 28.0)),
                "fg:tool:ok",
                Asked::Ok,
            ),
            (
                "Cancel",
                Rect::from_min_size(pos2(row.right() - 76.0, row.top()), vec2(76.0, 28.0)),
                "fg:tool:cancel",
                Asked::Cancel,
            ),
        ];
        for (word, r, name, what) in buttons {
            let resp = ui.interact(r, ui.scope_id().with(name), Sense::click());
            let ok = what == Asked::Ok;
            let fill = if ok {
                if resp.hovered() {
                    t.on.gamma_multiply(1.2)
                } else {
                    t.on
                }
            } else if resp.hovered() {
                t.hover
            } else {
                t.raised
            };
            ui.painter().rect_filled(r, 4.0, fill);
            ui.painter()
                .rect_stroke(r, 4.0, Stroke::new(1.0, t.line), egui::StrokeKind::Inside);
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                word,
                sans(13.0),
                if ok { t.on_ink } else { t.ink },
            );
            let resp = if what == Asked::Again {
                resp.on_hover_text("OK, then another")
            } else {
                resp
            };
            places.push((name.into(), r));
            if resp.clicked() {
                asked = what;
            }
        }
        asked
    }
}

/// The lists a tool's words choose from: fonts and solids beside the part, screw sizes and fits.
pub fn offer_lists(t: &mut FgTool, dir: Option<&std::path::Path>) {
    let beside = |exts: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = dir
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| {
                n.rsplit_once('.')
                    .is_some_and(|(_, x)| exts.contains(&x.to_ascii_lowercase().as_str()))
            })
            .collect();
        v.sort();
        v
    };
    let screws: Vec<String> = forge_model::standard::SCREWS
        .iter()
        .map(|s| s.name.to_string())
        .collect();
    let words: Vec<&'static str> = t.words.iter().map(|w| w.0).collect();
    for w in words {
        match w {
            "font" => t.offer(w, beside(&["ttf", "otf"])),
            "file" => t.offer(w, beside(&["step", "stp", "brep", "igs", "iges"])),
            "screw" | "spec" => t.offer(w, screws.clone()),
            "fit" => t.offer(w, ["close", "normal", "loose"].map(String::from).to_vec()),
            _ => {}
        }
    }
}
