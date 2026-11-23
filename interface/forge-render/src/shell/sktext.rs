//! Text in a sketch (`label = text("AB-1", at: (x, y), size: s, font: "Open Sans")`), as the
//! sketcher holds it: not solver geometry (the glyphs are fixed where they're set), so a text is
//! an item of its own beside the rows. Opened from a block it remembers the params its numbers
//! read (a change is a SetParam) and where its statement is; a new one is written with params of
//! the sketch for its numbers, as every number the sketcher writes is.

use super::fgtool::ToolError;
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::syntax::{Arg, Expr, ExprKind};
use forge_lang::values::Dim;
use std::collections::{HashMap, HashSet};

/// How a text's lines line up on its start.
pub const ALIGNS: [&str; 3] = ["left", "center", "right"];

#[derive(Clone, Debug, PartialEq)]
pub struct SkText {
    /// Its statement's name, once it has one in the file.
    pub name: Option<String>,
    /// Its lines, one to a line.
    pub text: String,
    /// Where the first line's baseline starts, in the sketch's own coordinates (mm).
    pub x: f64,
    pub y: f64,
    /// The em, mm.
    pub size: f64,
    /// Degrees, counter-clockwise.
    pub angle: f64,
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    pub align: String,
    /// As it was opened: its values, and the params its numbers are (by field).
    pub was: Option<Box<Was>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Was {
    pub values: [f64; 4],
    pub words: (String, String, bool, bool, String),
    /// `x`, `y`, `size`, `angle`: the param each is written as, where it's one.
    pub params: Vec<(&'static str, ParamKey, String)>,
}

const FIELDS: [&str; 4] = ["x", "y", "size", "angle"];

impl SkText {
    /// A new text where the click put it.
    pub fn new(x: f64, y: f64, size: f64) -> SkText {
        SkText {
            name: None,
            text: "TEXT".into(),
            x,
            y,
            size,
            angle: 0.0,
            font: forge_lang::fonts::DEFAULT.into(),
            bold: false,
            italic: false,
            align: "left".into(),
            was: None,
        }
    }

    fn values(&self) -> [f64; 4] {
        [self.x, self.y, self.size, self.angle]
    }

    fn words(&self) -> (String, String, bool, bool, String) {
        (
            self.text.clone(),
            self.font.clone(),
            self.bold,
            self.italic,
            self.align.clone(),
        )
    }

    /// Whether it differs from what was opened (a new one always does).
    pub fn changed(&self) -> bool {
        match &self.was {
            None => true,
            Some(w) => w.values != self.values() || w.words != self.words(),
        }
    }

    pub fn lines(&self) -> Vec<&str> {
        self.text.lines().collect()
    }

    /// What's wrong with it, typed: nothing to draw, a line Forge can't write, a size or
    /// alignment that isn't one.
    pub fn check(&self) -> Result<(), ToolError> {
        let no = |why: String| ToolError::NoForgeForm { tool: "Text", why };
        if self.lines().iter().all(|l| l.trim().is_empty()) {
            return Err(no("there is no text to draw".into()));
        }
        if self.text.contains('"') {
            return Err(no("a Forge string holds no double quote (\"): take it out".into()));
        }
        if self.font.contains('"') || self.font.trim().is_empty() {
            return Err(no(format!("{:?} is no font", self.font)));
        }
        if !(self.size.is_finite() && self.size > 0.0) {
            return Err(ToolError::BadNumber {
                param: "size",
                text: forge_lang::values::fmt(self.size),
            });
        }
        if !ALIGNS.contains(&self.align.as_str()) {
            return Err(no(format!(
                "{:?} is no alignment: left, center or right",
                self.align
            )));
        }
        Ok(())
    }

    /// The call, its numbers written by `num` (field, value, dimension).
    pub fn call(&self, num: &mut dyn FnMut(&'static str, f64, Dim) -> String) -> String {
        let lines = self.lines();
        let text = if lines.len() == 1 {
            format!("\"{}\"", lines[0])
        } else {
            let each: Vec<String> = lines.iter().map(|l| format!("\"{l}\"")).collect();
            format!("[{}]", each.join(", "))
        };
        let mut s = format!(
            "text({text}, at: ({}, {}), size: {}",
            num("x", self.x, Dim::Length),
            num("y", self.y, Dim::Length),
            num("size", self.size, Dim::Length)
        );
        if !self.font.eq_ignore_ascii_case(forge_lang::fonts::DEFAULT) {
            s.push_str(&format!(", font: \"{}\"", self.font));
        }
        if self.bold {
            s.push_str(", bold: true");
        }
        if self.italic {
            s.push_str(", italic: true");
        }
        if self.align != "left" {
            s.push_str(&format!(", align: \"{}\"", self.align));
        }
        if self.angle != 0.0
            || self
                .was
                .as_ref()
                .is_some_and(|w| w.params.iter().any(|p| p.0 == "angle"))
        {
            s.push_str(&format!(", angle: {}", num("angle", self.angle, Dim::Angle)));
        }
        s.push(')');
        s
    }

    /// Read from a `text(..)` call in a block: its numbers by the params they read (`env`: each
    /// param's value as text, angles in degrees) or as written.
    pub fn read(
        name: &str,
        args: &[Arg],
        params: &HashMap<String, (ParamKey, Dim)>,
        env: &[(String, String)],
    ) -> Result<SkText, String> {
        let arg = |k: usize, key: &str| -> Option<&Expr> {
            args.iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.0 == key))
                .or_else(|| args.iter().filter(|a| a.name.is_none()).nth(k))
                .map(|a| &a.value)
        };
        let mut bound: Vec<(&'static str, ParamKey, String)> = vec![];
        let mut number = |field: &'static str, e: &Expr| -> Result<f64, String> {
            match &e.kind {
                ExprKind::Num(v, _) => Ok(*v),
                ExprKind::Neg(x) => match &x.kind {
                    ExprKind::Num(v, _) => Ok(-*v),
                    _ => Err(format!("a text's {field} the sketcher can't read")),
                },
                ExprKind::Name(n) => {
                    let v = env
                        .iter()
                        .find(|p| p.0 == *n)
                        .and_then(|p| p.1.parse::<f64>().ok())
                        .ok_or_else(|| format!("a text's {field} read from {n}, no number here"))?;
                    if let Some((k, _)) = params.get(n) {
                        bound.push((field, k.clone(), n.clone()));
                    }
                    Ok(v)
                }
                _ => Err(format!("a text's {field} the sketcher can't read")),
            }
        };
        let text = match arg(0, "text").map(|e| &e.kind) {
            Some(ExprKind::Str(s)) => s.clone(),
            Some(ExprKind::List(xs)) => {
                let mut lines = vec![];
                for x in xs {
                    match &x.kind {
                        ExprKind::Str(s) => lines.push(s.clone()),
                        _ => return Err("a text line that isn't a string".into()),
                    }
                }
                lines.join("\n")
            }
            _ => return Err("a text the sketcher can't read".into()),
        };
        let (x, y) = match arg(99, "at").map(|e| &e.kind) {
            None => (0.0, 0.0),
            Some(ExprKind::Tuple(t)) if t.len() == 2 => (number("x", &t[0])?, number("y", &t[1])?),
            Some(_) => return Err("a text's start the sketcher can't read".into()),
        };
        let size = number("size", arg(99, "size").ok_or("a text with no size")?)?;
        let angle = match arg(99, "angle") {
            Some(e) => number("angle", e)?,
            None => 0.0,
        };
        let word = |key: &str| -> Result<Option<String>, String> {
            match arg(99, key).map(|e| &e.kind) {
                None => Ok(None),
                Some(ExprKind::Str(s)) => Ok(Some(s.clone())),
                Some(_) => Err(format!("a text's {key} the sketcher can't read")),
            }
        };
        let flag = |key: &str| -> Result<bool, String> {
            match arg(99, key).map(|e| &e.kind) {
                None => Ok(false),
                Some(ExprKind::Bool(b)) => Ok(*b),
                Some(_) => Err(format!("a text's {key} the sketcher can't read")),
            }
        };
        let mut t = SkText {
            name: Some(name.to_string()),
            text,
            x,
            y,
            size,
            angle,
            font: word("font")?.unwrap_or_else(|| forge_lang::fonts::DEFAULT.into()),
            bold: flag("bold")?,
            italic: flag("italic")?,
            align: word("align")?.unwrap_or_else(|| "left".into()),
            was: None,
        };
        t.was = Some(Box::new(Was {
            values: t.values(),
            words: t.words(),
            params: bound,
        }));
        Ok(t)
    }
}

/// Closed polylines, in ems.
pub type Outlines = std::sync::Arc<Vec<Vec<(f64, f64)>>>;

/// A text's glyphs in its own font, flattened, in ems (`forge_lang::sketch_text::preview`),
/// kept by what decides them: laid out once per change, not once a frame.
pub fn outlines(dir: Option<&std::path::Path>, t: &SkText) -> Result<Outlines, String> {
    use std::sync::{Arc, Mutex, OnceLock};
    type Key = (Option<std::path::PathBuf>, String, bool, bool, String, String);
    type Got = Result<Outlines, String>;
    static CACHE: OnceLock<Mutex<HashMap<Key, Got>>> = OnceLock::new();
    let key: Key = (
        dir.map(|d| d.to_path_buf()),
        t.font.clone(),
        t.bold,
        t.italic,
        t.align.clone(),
        t.text.clone(),
    );
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(got) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return got.clone();
    }
    let got: Got =
        forge_lang::sketch_text::preview(dir, &t.font, t.bold, t.italic, &t.text, &t.align).map(Arc::new);
    let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
    // a bounded cache: typing makes a key a keystroke
    if c.len() > 256 {
        c.clear();
    }
    c.insert(key, got.clone());
    got
}

/// The fonts a sketch text may take: the shipped families, then .ttf and .otf files beside the
/// part.
pub fn fonts(dir: Option<&std::path::Path>) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for (family, _) in forge_lang::fonts::faces() {
        if !out.iter().any(|f| f == family) {
            out.push(family.to_string());
        }
    }
    let mut beside: Vec<String> = dir
        .and_then(|d| forge_dirs::fs::read_dir(d).ok())
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| {
            n.rsplit_once('.')
                .is_some_and(|(_, x)| matches!(x.to_ascii_lowercase().as_str(), "ttf" | "otf"))
        })
        .collect();
    beside.sort();
    out.extend(beside);
    out
}

/// Where a statement named `name` is in `block`: its first byte to just past its last, and the
/// line it's on (indent to line break) for taking it out.
fn statement(block: &str, name: &str) -> Option<((usize, usize), (usize, usize))> {
    let head = format!("{name} = text(");
    let lo = block.match_indices(&head).map(|m| m.0).find(|&k| {
        block[..k]
            .chars()
            .next_back()
            .is_none_or(|c| c == ' ' || c == '\n' || c == '\t')
    })?;
    let b = block.as_bytes();
    let (mut depth, mut i, mut quoted) = (0i32, lo + head.len() - 1, false);
    while i < b.len() {
        match b[i] {
            b'"' => quoted = !quoted,
            b'(' if !quoted => depth += 1,
            b')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    let hi = i + 1;
                    let line_lo = block[..lo].rfind('\n').map_or(0, |n| n + 1);
                    let line_hi = block[hi..].find('\n').map_or(block.len(), |n| hi + n + 1);
                    return Some(((lo, hi), (line_lo, line_hi)));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The block (a sketch's whole statement, its params above it) with its texts written: new ones
/// added above its closing brace with params of the sketch for their numbers, changed ones
/// rewritten in place (a number that reads a param sets the param: the ops), and `gone` ones
/// taken out.
pub fn write(
    block: &str,
    sketch: &str,
    texts: &[SkText],
    gone: &[String],
    taken: &HashSet<String>,
) -> Result<(String, Vec<Op>), ToolError> {
    let mut out = block.to_string();
    let mut ops: Vec<Op> = vec![];
    let mut taken = taken.clone();
    let lit = |v: f64, dim: Dim| Literal { value: v, dim }.text();
    for name in gone {
        if let Some((_, line)) = statement(&out, name) {
            out.replace_range(line.0..line.1, "");
        }
    }
    for t in texts.iter().filter(|t| t.name.is_some() && t.changed()) {
        t.check()?;
        let name = t.name.as_deref().expect("a named text");
        let Some((span, _)) = statement(&out, name) else {
            return Err(ToolError::NoForgeForm {
                tool: "Text",
                why: format!("{name} isn't in the sketch any more"),
            });
        };
        let was = t.was.as_ref().expect("an opened text");
        let call = t.call(
            &mut |field, v, dim| match was.params.iter().find(|p| p.0 == field) {
                Some((_, key, bare)) => {
                    let k = FIELDS.iter().position(|f| *f == field).expect("a field");
                    if was.values[k] != v {
                        ops.retain(|o| !matches!(o, Op::SetParam { param, .. } if param == key));
                        ops.push(Op::SetParam {
                            param: key.clone(),
                            value: Literal { value: v, dim },
                        });
                    }
                    bare.clone()
                }
                None => lit(v, dim),
            },
        );
        out.replace_range(span.0..span.1, &format!("{name} = {call}"));
    }
    let new: Vec<&SkText> = texts.iter().filter(|t| t.name.is_none()).collect();
    if new.is_empty() {
        return Ok((out, ops));
    }
    let mut params = String::new();
    let mut stmts = String::new();
    for t in new {
        t.check()?;
        let name = (1..)
            .map(|k| format!("t{k}"))
            .find(|n| !taken.contains(n))
            .expect("a free name");
        taken.insert(name.clone());
        let call = t.call(&mut |field, v, dim| {
            let p = format!("{name}_{field}");
            params.push_str(&format!("param {sketch}.{p} = {}\n", lit(v, dim)));
            p
        });
        stmts.push_str(&format!("  {name} = {call}\n"));
    }
    // above the block's closing brace, and its params above the block
    let close = out.rfind('}').ok_or(ToolError::NoForgeForm {
        tool: "Text",
        why: "no sketch block to write it in".into(),
    })?;
    let line_start = out[..close].rfind('\n').map_or(0, |n| n + 1);
    if out[line_start..close].trim().is_empty() {
        out.insert_str(line_start, &stmts);
    } else {
        out.insert_str(close, &format!("\n{stmts}"));
    }
    let head = out
        .match_indices("sketch ")
        .map(|m| m.0)
        .find(|&k| k == 0 || out.as_bytes()[k - 1] == b'\n')
        .unwrap_or(0);
    out.insert_str(head, &params);
    Ok((out, ops))
}

impl super::Shell {
    /// The sketch's texts drawn where they'll be (a preview in the sketch's colour: the glyphs
    /// themselves come from the build at Finish), and the open one's dialog.
    pub(super) fn sktext_ui(&mut self, ctx: &egui::Context) {
        use super::theme::{self, S1, S2};
        use egui::{Rect, pos2, vec2};
        use forge_math::dmath::{cos, sin};
        let t = self.theme;
        let Some((camera, size)) = self.camera else { return };
        let Some(m) = self.sketch.as_mut() else { return };
        let ppp = ctx.pixels_per_point();
        let view = self.view_rect;
        let plane = m.plane;
        let screen = |p: (f64, f64)| {
            let w = plane.plane_point(forge_math::Point2::new(p.0, p.1));
            camera
                .pixel_of([w.x as f32, w.y as f32, w.z as f32], size.0, size.1)
                .map(|px| view.min + vec2(px.0, px.1) / ppp)
        };
        let font_dir = self
            .fg
            .as_ref()
            .and_then(|f| std::path::Path::new(&f.path).parent().map(|d| d.to_path_buf()));
        let painter = ctx
            .layer_painter(egui::LayerId::background())
            .with_clip_rect(view);
        for (k, s) in m.more.sktexts.iter().enumerate() {
            let a = s.angle.to_radians();
            let (Some(o), Some(e), Some(u)) = (
                screen((s.x, s.y)),
                screen((s.x + cos(a) * s.size, s.y + sin(a) * s.size)),
                screen((s.x - sin(a) * s.size, s.y + cos(a) * s.size)),
            ) else {
                continue;
            };
            let em = (u - o).length();
            if !(2.0..=400.0).contains(&em) {
                continue;
            }
            let along = (e - o).normalized();
            let up = (u - o).normalized();
            let font = if s.bold { theme::semi(em) } else { theme::sans(em) };
            let open = m.more.sktext_open == Some(k);
            let ink = if open { t.focus } else { t.sketch };
            // the glyphs in the chosen font, as the build will set them
            if let Ok(lines) = outlines(font_dir.as_deref(), s) {
                let stroke = egui::Stroke::new(1.25, ink);
                let (sa, ca) = (sin(a), cos(a));
                for l in lines.iter() {
                    let pts: Vec<egui::Pos2> = l
                        .iter()
                        .filter_map(|&(ex, ey)| {
                            let (x, y) = (s.size * ex, s.size * ey);
                            screen((s.x + x * ca - y * sa, s.y + x * sa + y * ca))
                        })
                        .collect();
                    if pts.len() == l.len() {
                        painter.add(egui::Shape::closed_line(pts, stroke));
                    }
                }
                painter.circle_filled(o, 3.0, ink);
                self.places.push((
                    format!("sktext:at:{k}"),
                    Rect::from_center_size(o, vec2(10.0, 10.0)),
                ));
                continue;
            }
            for (line, words) in s.lines().iter().enumerate() {
                let g = ctx.fonts_mut(|f| f.layout_no_wrap(words.to_string(), font.clone(), ink));
                let w = g.size().x;
                let shift = match s.align.as_str() {
                    "center" => -w * 0.5,
                    "right" => -w,
                    _ => 0.0,
                };
                // the line's baseline start, then the galley's top left above it
                let base = o + along * shift - up * (line as f32 * em * 1.36);
                let top_left = base + up * (em * 0.93);
                let shape = egui::epaint::TextShape::new(top_left, g, ink).with_angle(along.angle());
                painter.add(shape);
            }
            painter.circle_filled(o, 3.0, ink);
            self.places.push((
                format!("sktext:at:{k}"),
                Rect::from_center_size(o, vec2(10.0, 10.0)),
            ));
        }
        let Some(k) = m.more.sktext_open.filter(|k| *k < m.more.sktexts.len()) else {
            return;
        };
        let dir = self
            .fg
            .as_ref()
            .and_then(|f| std::path::Path::new(&f.path).parent().map(|d| d.to_path_buf()));
        let fonts = fonts(dir.as_deref());
        let m = self.sketch.as_mut().expect("the sketch");
        let mut places: Vec<(String, Rect)> = vec![];
        let (mut done, mut delete, mut close) = (false, false, false);
        let s = &mut m.more.sktexts[k];
        let shown = egui::Window::new("Text")
            .id(egui::Id::unique("sketch-text-dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(pos2(view.left() + 12.0, view.top() + 56.0))
            .fixed_size(vec2(super::dialog::DIALOG_W, 0.0))
            .frame(theme::panel_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                close = super::fgasm::dialog_head(ui, &t, "sktext", "Text", "sktext", &mut places);
                let r = ui.add(
                    egui::TextEdit::multiline(&mut s.text)
                        .desired_rows(3)
                        .desired_width(super::dialog::DIALOG_W)
                        .font(theme::body())
                        .hint_text("One line per line"),
                );
                places.push(("sktext:text".into(), r.rect));
                let row =
                    |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui) -> egui::Response| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(
                                vec2(super::dialog::FIELD_AT - S2, theme::CONTROL_H),
                                egui::Sense::hover(),
                            );
                            ui.painter().text(
                                pos2(r.left(), r.center().y),
                                egui::Align2::LEFT_CENTER,
                                label,
                                theme::body(),
                                t.ink,
                            );
                            add(ui)
                        })
                        .inner
                    };
                let r = row(ui, "Font", &mut |ui| {
                    egui::ComboBox::from_id_salt("sktext-font")
                        .selected_text(s.font.clone())
                        .width(super::dialog::DIALOG_W - super::dialog::FIELD_AT - S1)
                        .show_ui(ui, |ui| {
                            for f in &fonts {
                                ui.selectable_value(&mut s.font, f.clone(), f.as_str());
                            }
                        })
                        .response
                });
                places.push(("sktext:font".into(), r.rect));
                let r = row(ui, "Size", &mut |ui| {
                    ui.add(
                        egui::DragValue::new(&mut s.size)
                            .speed(0.1)
                            .range(0.1..=10_000.0)
                            .suffix(" mm"),
                    )
                });
                places.push(("sktext:size".into(), r.rect));
                let r = row(ui, "Angle", &mut |ui| {
                    ui.add(
                        egui::DragValue::new(&mut s.angle)
                            .speed(1.0)
                            .range(-360.0..=360.0)
                            .suffix("°"),
                    )
                });
                places.push(("sktext:angle".into(), r.rect));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = S2 * 2.0;
                    let r = ui.checkbox(&mut s.bold, egui::RichText::new("Bold").font(theme::body()));
                    places.push(("sktext:bold".into(), r.rect));
                    let r = ui.checkbox(&mut s.italic, egui::RichText::new("Italic").font(theme::body()));
                    places.push(("sktext:italic".into(), r.rect));
                });
                let items: Vec<(&str, bool)> = ALIGNS.iter().map(|a| (*a, s.align == *a)).collect();
                let words = [
                    ("Left", items[0].1),
                    ("Center", items[1].1),
                    ("Right", items[2].1),
                ];
                if let Some(c) = super::dialog::text_toggles(ui, &t, &words, "sktext:align", &mut places) {
                    s.align = ALIGNS[c].to_string();
                }
                if let Err(e) = s.check() {
                    ui.label(egui::RichText::new(e.text()).font(theme::small()).color(t.error));
                } else if let Err(why) = outlines(dir.as_deref(), s) {
                    ui.label(egui::RichText::new(why).font(theme::small()).color(t.error));
                }
                ui.add_space(S1);
                ui.horizontal(|ui| {
                    let b = ui.button(egui::RichText::new("Delete").font(theme::small()));
                    places.push(("sktext:delete".into(), b.rect));
                    delete = b.clicked();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Done")
                                    .font(theme::medium(13.0))
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(t.accent)
                            .min_size(vec2(74.0, theme::CONTROL_H)),
                        );
                        places.push(("sktext:done".into(), b.rect));
                        done = b.clicked();
                    });
                });
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
            places.push(("sktext:dialog".into(), r.response.rect));
        }
        let m = self.sketch.as_mut().expect("the sketch");
        if delete {
            let gone = m.more.sktexts.remove(k);
            if let Some(n) = gone.name {
                m.more.sktexts_gone.push(n);
            }
            m.more.sktext_open = None;
        } else if done || close {
            m.more.sktext_open = None;
        }
        self.places.extend(places);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "param sk1.x1 = 0\nsketch sk1(on: xy) {\n  l1 = line(near(x1, 0), near(10, 0))\n}";

    #[test]
    fn a_new_text_is_written_with_params_for_its_numbers() {
        let mut t = SkText::new(10.0, 20.0, 5.0);
        t.text = "AB-1\nLINE 2".into();
        t.bold = true;
        t.align = "center".into();
        let (out, ops) = write(BLOCK, "sk1", &[t], &[], &HashSet::new()).unwrap();
        assert!(ops.is_empty());
        assert_eq!(
            out,
            "param sk1.x1 = 0\nparam sk1.t1_x = 10\nparam sk1.t1_y = 20\nparam sk1.t1_size = 5\n\
             sketch sk1(on: xy) {\n  l1 = line(near(x1, 0), near(10, 0))\n  \
             t1 = text([\"AB-1\", \"LINE 2\"], at: (t1_x, t1_y), size: t1_size, bold: true, align: \"center\")\n}"
        );
    }

    #[test]
    fn an_opened_text_sets_its_params_and_rewrites_its_words() {
        let src = "param sk1.t1_x = 10\nparam sk1.t1_size = 5\nsketch sk1(on: xy) {\n  t1 = text(\"A\", at: (t1_x, 2), size: t1_size, font: \"Source Sans 3\")\n}";
        let prog = forge_lang::syntax::parse(src).unwrap();
        // the call's arguments, as the block reads them
        let args = format!("{prog:?}");
        assert!(args.contains("Source Sans 3"));
        let key = |n: &str| ParamKey {
            owner: vec![],
            name: n.into(),
        };
        let params: HashMap<String, (ParamKey, Dim)> = [
            ("t1_x".to_string(), (key("t1_x"), Dim::Length)),
            ("t1_size".to_string(), (key("t1_size"), Dim::Length)),
        ]
        .into();
        let env = vec![
            ("t1_x".to_string(), "10".to_string()),
            ("t1_size".to_string(), "5".to_string()),
        ];
        let call = |e: &Expr| -> Vec<Arg> {
            match &e.kind {
                ExprKind::Call(_, a) => a.clone(),
                _ => panic!(),
            }
        };
        let stmt = match &prog[2].kind {
            forge_lang::syntax::StmtKind::Block { body, .. } => body[0].clone(),
            k => panic!("{k:?}"),
        };
        let args = match &stmt.kind {
            forge_lang::syntax::StmtKind::Assign { value, .. } => call(value),
            _ => panic!(),
        };
        let mut t = SkText::read("t1", &args, &params, &env).unwrap();
        assert_eq!(
            (t.x, t.y, t.size, t.font.as_str()),
            (10.0, 2.0, 5.0, "Source Sans 3")
        );
        assert!(!t.changed());
        t.x = 12.0;
        t.y = 3.0;
        t.text = "B".into();
        t.italic = true;
        let (out, ops) = write(src, "sk1", &[t], &[], &HashSet::new()).unwrap();
        assert_eq!(
            ops,
            vec![Op::SetParam {
                param: key("t1_x"),
                value: Literal {
                    value: 12.0,
                    dim: Dim::Length
                }
            }]
        );
        assert!(
            out.contains(
                "t1 = text(\"B\", at: (t1_x, 3), size: t1_size, font: \"Source Sans 3\", italic: true)"
            ),
            "{out}"
        );
        // and taken out
        let (out, _) = write(src, "sk1", &[], &["t1".into()], &HashSet::new()).unwrap();
        assert!(!out.contains("text("), "{out}");
    }

    #[test]
    fn the_preview_is_the_chosen_fonts_glyphs() {
        let mut t = SkText::new(0.0, 0.0, 5.0);
        t.text = "AB".into();
        let plain = outlines(None, &t).expect("Open Sans lays out");
        // A has an outside and a counter; B an outside and two counters
        assert_eq!(plain.len(), 5, "{}", plain.len());
        t.bold = true;
        let bold = outlines(None, &t).unwrap();
        assert_ne!(plain, bold, "bold is its own face");
        t.font = "Source Sans 3".into();
        t.bold = false;
        let other = outlines(None, &t).unwrap();
        assert_ne!(plain, other, "another family draws otherwise");
        t.font = "No Such Font".into();
        assert!(outlines(None, &t).is_err());
    }

    #[test]
    fn a_quote_in_the_text_is_refused_typed() {
        let mut t = SkText::new(0.0, 0.0, 5.0);
        t.text = "say \"hi\"".into();
        assert_eq!(t.check().map_err(|e| e.kind()), Err("NoForgeForm"));
        t.text = "   ".into();
        assert!(t.check().is_err());
    }
}
