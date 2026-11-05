//! Place from Content Center, as Inventor has it: the standard parts library's categories on the
//! left, the chosen category's families as pictures, and the chosen family's table (size, length,
//! thread, grade) with the part's designation; Place writes `NAME = std(..)` into the assembly.

use crate::shell::placed::Placed;
use super::fgasm::{dialog_frame, dialog_head};
use super::theme::{Theme, medium, mono, sans, semi};
use super::{Shell, icons};
use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_lang::error::Op;
use forge_lang::stdparts::{Family, Spec, families, generate, key};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};

const W: f32 = 860.0;
const TREE_W: f32 = 200.0;
const TILE: f32 = 112.0;
const PICTURE_PX: usize = 160;

#[derive(Default)]
pub struct ContentCenter {
    pub open: bool,
    /// The category chosen in the tree: a path prefix, "" for all.
    cat: String,
    /// The family chosen, by its file stem.
    fam: Option<&'static str>,
    size: Option<String>,
    length: Option<f64>,
    thread: usize,
    grade: usize,
    /// Scroll the size and length lists to their choices on the next frame.
    jump: bool,
    /// Scroll the length list to its choice on the next frame: the size changed under it.
    jump_length: bool,
    /// By the standard part's key: each family's tile, and each choice shown large.
    pictures: HashMap<String, Picture>,
    made: Option<(Sender<Made>, Receiver<Made>)>,
}

/// A family's picture as its thread finished it.
type Made = (String, Option<egui::ColorImage>);

enum Picture {
    Making,
    Ready(egui::TextureHandle),
    None,
}

/// A family's tile: its middle size at a short length, so the head shows at a tile's size.
fn tile_spec(f: &Family) -> Spec {
    let row = &f.rows[f.rows.len() / 2];
    let ls = f.lengths_of(row);
    Spec {
        family: f.name.clone(),
        size: row[0].clone(),
        length: ls
            .iter()
            .copied()
            .find(|l| *l >= 2.5 * thread_d(f, row))
            .or(ls.last().copied()),
        options: Default::default(),
    }
}

/// A part's picture, built and drawn as the CLI draws, from below the head (Inventor's tiles
/// show the head).
fn picture(spec: &Spec) -> Option<egui::ColorImage> {
    let text = generate(spec).ok()?.text;
    let run = forge_lang::run(&text, forge_lang::Options::default(), Default::default()).0;
    let body = run.body?;
    let mut cache = forge_agent::facemesh::Cache::default();
    let mesh = forge_agent::facemesh::join(&cache.faces(&body));
    let p = forge_agent::render::picture(&body, &mesh, "under", PICTURE_PX, &[]).ok()?;
    let mut rgba = Vec::with_capacity(p.w * p.h * 4);
    let bg = p.px.first().copied();
    for &i in &p.px {
        let [r, g, b] = p.palette[i as usize];
        // the background clear, so the tile's own colour shows
        rgba.extend_from_slice(&[r, g, b, if Some(i) == bg { 0 } else { 255 }]);
    }
    Some(egui::ColorImage::from_rgba_unmultiplied([p.w, p.h], &rgba))
}

/// A size's diameter as its table's unit counts it, for a length in proportion.
fn thread_d(f: &Family, row: &[String]) -> f64 {
    let s = f.col("thread").map_or(row[0].as_str(), |c| row[c].as_str());
    s.trim_start_matches('M')
        .split(['x', '-'])
        .next()
        .and_then(|d| d.parse().ok())
        .unwrap_or(0.0)
}

/// The tree's nodes: each category path and every prefix of it, sorted.
fn categories() -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for f in families().iter().filter(|f| f.placeable()) {
        let parts: Vec<&str> = f.category().split('/').collect();
        for k in 1..=parts.len() {
            let p = parts[..k].join("/");
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn in_cat(f: &Family, cat: &str) -> bool {
    cat.is_empty() || f.category() == cat || f.category().starts_with(&format!("{cat}/"))
}

/// A name for the new instance not yet in the document: `iso4762_m6x20`, `iso4762_m6x20_2` ..
fn fresh_name(source: &str, f: &Family, size: &str, length: Option<f64>) -> String {
    let mut base = format!("{}_{}", f.id, size.to_ascii_lowercase());
    if let Some(l) = length {
        base += &format!("x{}", forge_lang::values::fmt(l));
    }
    let base: String = base
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let taken = |n: &str| {
        source.lines().any(|l| {
            l.trim_start()
                .strip_prefix(n)
                .is_some_and(|r| r.trim_start().starts_with('='))
        })
    };
    let mut name = base.clone();
    let mut k = 2;
    while taken(&name) {
        name = format!("{base}_{k}");
        k += 1;
    }
    name
}

impl ContentCenter {
    fn family(&self) -> Option<&'static Family> {
        families().iter().find(|f| Some(f.id) == self.fam)
    }

    fn choose(&mut self, f: &'static Family) {
        self.fam = Some(f.id);
        let row = &f.rows[f.rows.len() / 2];
        self.size = Some(row[0].clone());
        self.pick_length(f);
        self.thread = 0;
        self.grade = 0;
        self.jump = true;
    }

    /// The length kept if the size has it, else the nearest it has.
    fn pick_length(&mut self, f: &Family) {
        let Some(row) = self.size.as_ref().and_then(|s| f.row(s)) else {
            return;
        };
        let ls = f.lengths_of(row);
        self.length = match self.length {
            _ if ls.is_empty() => None,
            Some(l) if ls.contains(&l) => Some(l),
            Some(l) => ls
                .iter()
                .copied()
                .min_by(|a, b| (a - l).abs().total_cmp(&(b - l).abs())),
            None => ls.get(ls.len() / 2).copied(),
        };
    }

    fn spec(&self, f: &Family) -> Option<Spec> {
        let mut options = std::collections::BTreeMap::new();
        for (name, k) in [("thread", self.thread), ("grade", self.grade)] {
            if let Some((_, vals)) = f.options.iter().find(|o| o.0 == name)
                && k > 0
            {
                options.insert(name.to_string(), vals[k].clone());
            }
        }
        Some(Spec {
            family: f.name.clone(),
            size: self.size.clone()?,
            length: self.length,
            options,
        })
    }

    /// The statement Place writes.
    pub fn statement(&self, source: &str) -> Option<String> {
        let f = self.family()?;
        let s = self.spec(f)?;
        let mut call = format!("std({:?}, size: {:?}", f.name, s.size);
        if let Some(l) = s.length {
            call += &format!(", length: {}", forge_lang::values::fmt(l));
        }
        for (k, v) in &s.options {
            call += &format!(", {k}: {v:?}");
        }
        Some(format!("{} = {call})", fresh_name(source, f, &s.size, s.length)))
    }

    /// Pictures finished since the last frame, as textures; the next ones started.
    fn pictures(&mut self, ctx: &egui::Context, want: Vec<Spec>) {
        let (tx, rx) = self.made.get_or_insert_with(channel);
        while let Ok((id, img)) = rx.try_recv() {
            let p = match img {
                Some(i) => {
                    Picture::Ready(ctx.load_texture(format!("cc:{id}"), i, egui::TextureOptions::LINEAR))
                }
                None => Picture::None,
            };
            self.pictures.insert(id, p);
        }
        for s in want {
            let k = key(&s);
            if self.pictures.contains_key(&k) {
                continue;
            }
            self.pictures.insert(k.clone(), Picture::Making);
            let (tx, ctx) = (tx.clone(), ctx.clone());
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let _ = tx.send((k, picture(&s)));
                ctx.request_repaint();
            });
            #[cfg(target_arch = "wasm32")]
            {
                let _ = tx.send((k, picture(&s)));
                ctx.request_repaint();
            }
        }
        if self.pictures.values().any(|p| matches!(p, Picture::Making)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn picture_in(ui: &mut Ui, t: &Theme, p: Option<&Picture>, r: Rect) {
    match p {
        Some(Picture::Ready(tex)) => {
            let s = tex.size_vec2();
            let k = (r.width() / s.x).min(r.height() / s.y);
            let at = Rect::from_center_size(r.center(), s * k);
            ui.painter().image(
                tex.id(),
                at,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        Some(Picture::Making) => {
            ui.painter()
                .text(r.center(), egui::Align2::CENTER_CENTER, "...", sans(12.0), t.ink2);
        }
        _ => {
            icons::draw(
                ui.painter(),
                r.center() - vec2(14.0, 14.0),
                28.0,
                "part",
                1.5,
                t.ink2,
            );
        }
    }
}

/// A list of choices, one per row, the chosen one marked: the index clicked.
#[allow(clippy::too_many_arguments)]
fn pick_list(
    ui: &mut Ui,
    t: &Theme,
    id: &str,
    items: &[String],
    chosen: Option<usize>,
    jump: bool,
    h: f32,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let mut out = None;
    // whole rows: the list's height, and the chosen row near the middle with the top row whole
    let pitch = 20.0 + ui.spacing().item_spacing.y;
    let shown = ((h + ui.spacing().item_spacing.y) / pitch).floor().max(1.0) as usize;
    let mut area = egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(shown as f32 * pitch - ui.spacing().item_spacing.y)
        .auto_shrink([false, false]);
    if let Some(k) = chosen.filter(|_| jump) {
        let top = k.saturating_sub(shown / 2).min(items.len().saturating_sub(shown));
        area = area.vertical_scroll_offset(top as f32 * pitch);
    }
    area.show(ui, |ui| {
        for (k, it) in items.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click());
            let on = chosen == Some(k);
            if on {
                ui.painter().rect_filled(r, 3.0, t.selection);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 3.0, t.hover);
            }
            ui.painter().text(
                pos2(r.left() + 8.0, r.center().y),
                egui::Align2::LEFT_CENTER,
                it,
                mono(12.0),
                t.ink,
            );
            places.push((format!("cc:{id}:{it}"), r));
            if resp.clicked() {
                out = Some(k);
            }
        }
    });
    out
}

impl Shell {
    /// The Place from Content Center dialog, when open.
    pub(super) fn content_window(&mut self, ctx: &egui::Context) {
        if !self.content.open {
            return;
        }
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = vec![];
        let mut cc = std::mem::take(&mut self.content);
        let source = self.fg.as_ref().map(|f| f.source.clone()).unwrap_or_default();
        let in_asm = self.is_asm();
        let mut place = None;
        let mut close = false;
        let at = pos2(v.center().x - W / 2.0, v.top() + 40.0);
        let shown = egui::Window::new("Place from Content Center")
            .id(egui::Id::unique("content-center"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(at)
            .min_width(W)
            .max_width(W)
            .frame(dialog_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                close = dialog_head(ui, &t, "part", "Place from Content Center", "cc", places);
                ui.add_space(4.0);
                let fams: Vec<&'static Family> = families()
                    .iter()
                    .filter(|f| f.placeable() && in_cat(f, &cc.cat))
                    .collect();
                let mut want: Vec<Spec> = fams.iter().map(|f| tile_spec(f)).collect();
                if let Some(s) = cc.family().and_then(|f| cc.spec(f)) {
                    want.push(s);
                }
                cc.pictures(ctx, want);
                ui.horizontal_top(|ui| {
                    // category view
                    ui.vertical(|ui| {
                        ui.set_width(TREE_W);
                        ui.label(
                            egui::RichText::new("Category View")
                                .font(medium(11.0))
                                .color(t.ink2),
                        );
                        let mut all = vec![String::new()];
                        all.extend(categories());
                        egui::ScrollArea::vertical()
                            .id_salt("cc-tree")
                            .max_height(420.0)
                            .show(ui, |ui| {
                                for c in &all {
                                    let depth = if c.is_empty() {
                                        0
                                    } else {
                                        c.matches('/').count() + 1
                                    };
                                    let name = if c.is_empty() {
                                        "Content Center"
                                    } else {
                                        c.rsplit('/').next().unwrap_or(c)
                                    };
                                    let (r, resp) =
                                        ui.allocate_exact_size(vec2(TREE_W - 8.0, 22.0), Sense::click());
                                    let on = cc.cat == *c;
                                    if on {
                                        ui.painter().rect_filled(r, 3.0, t.selection);
                                    } else if resp.hovered() {
                                        ui.painter().rect_filled(r, 3.0, t.hover);
                                    }
                                    let x = r.left() + 4.0 + 14.0 * depth as f32;
                                    icons::draw(
                                        ui.painter(),
                                        pos2(x, r.top() + 3.0),
                                        16.0,
                                        "folder",
                                        1.3,
                                        t.ink2,
                                    );
                                    ui.painter().text(
                                        pos2(x + 22.0, r.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        name,
                                        sans(12.0),
                                        t.ink,
                                    );
                                    places.push((format!("cc:cat:{name}"), r));
                                    if resp.clicked() {
                                        cc.cat = c.clone();
                                    }
                                }
                            });
                    });
                    let sep = ui.available_rect_before_wrap();
                    ui.painter().vline(
                        sep.left(),
                        sep.top()..=sep.top() + 440.0,
                        Stroke::new(1.0, t.line),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{} families", fams.len()))
                                .font(medium(11.0))
                                .color(t.ink2),
                        );
                        // families as pictures
                        egui::ScrollArea::vertical()
                            .id_salt("cc-fams")
                            .max_height(2.0 * (TILE + 44.0) + 10.0)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                                    for f in &fams {
                                        let (r, resp) =
                                            ui.allocate_exact_size(vec2(TILE, TILE + 44.0), Sense::click());
                                        let on = cc.fam == Some(f.id);
                                        ui.painter().rect(
                                            r,
                                            4.0,
                                            if on {
                                                t.selection
                                            } else if resp.hovered() {
                                                t.hover
                                            } else {
                                                t.raised
                                            },
                                            Stroke::new(1.0, if on { t.accent } else { t.line }),
                                            StrokeKind::Inside,
                                        );
                                        let pic = Rect::from_min_size(
                                            r.min + vec2(8.0, 6.0),
                                            vec2(TILE - 16.0, TILE - 12.0),
                                        );
                                        picture_in(ui, &t, cc.pictures.get(&key(&tile_spec(f))), pic);
                                        ui.painter().text(
                                            pos2(r.center().x, r.bottom() - 32.0),
                                            egui::Align2::CENTER_CENTER,
                                            &f.name,
                                            semi(11.0),
                                            t.ink,
                                        );
                                        let g = ui.painter().layout(
                                            f.item().to_string(),
                                            sans(9.5),
                                            t.ink2,
                                            TILE - 8.0,
                                        );
                                        let at = pos2(r.center().x - g.size().x / 2.0, r.bottom() - 22.0);
                                        ui.painter().galley(at, g, t.ink2);
                                        places.push((format!("cc:fam:{}", f.name), r));
                                        let resp = resp.on_hover_text(format!("{}\n{}", f.title, f.standard));
                                        if resp.clicked() {
                                            cc.choose(f);
                                        }
                                    }
                                });
                            });
                        ui.add_space(6.0);
                        let r = ui.available_rect_before_wrap();
                        ui.painter()
                            .hline(r.left()..=r.right(), r.top(), Stroke::new(1.0, t.line));
                        ui.add_space(6.0);
                        let Some(f) = cc.family() else {
                            ui.label(
                                egui::RichText::new("Choose a family")
                                    .font(sans(12.0))
                                    .color(t.ink2),
                            );
                            return;
                        };
                        // the family's table
                        let jump = std::mem::take(&mut cc.jump);
                        let jump_length = std::mem::take(&mut cc.jump_length) || jump;
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(150.0);
                                let (r, _) = ui.allocate_exact_size(vec2(150.0, 130.0), Sense::hover());
                                ui.painter().rect_filled(r, 4.0, t.raised);
                                let shown = cc.spec(f).map(|s| key(&s)).and_then(|k| cc.pictures.get(&k));
                                picture_in(ui, &t, shown, r.shrink(6.0));
                                ui.label(egui::RichText::new(&f.title).font(semi(12.0)).color(t.ink));
                                ui.label(egui::RichText::new(&f.standard).font(sans(11.0)).color(t.ink2));
                                ui.label(
                                    egui::RichText::new(if f.inch { "inch" } else { "metric" })
                                        .font(sans(11.0))
                                        .color(t.ink2),
                                );
                            });
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.set_width(110.0);
                                ui.label(egui::RichText::new("Size").font(medium(11.0)).color(t.ink2));
                                let sizes: Vec<String> = f.sizes().iter().map(|s| s.to_string()).collect();
                                let k = cc.size.as_ref().and_then(|s| sizes.iter().position(|x| x == s));
                                if let Some(k) =
                                    pick_list(ui, &t, "size", &sizes, k, jump, 190.0, places)
                                {
                                    cc.size = Some(sizes[k].clone());
                                    cc.pick_length(f);
                                    cc.jump_length = true;
                                }
                            });
                            if f.has_lengths() {
                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.set_width(110.0);
                                    ui.label(
                                        egui::RichText::new(if f.inch {
                                            "Length (in)"
                                        } else {
                                            "Length (mm)"
                                        })
                                        .font(medium(11.0))
                                        .color(t.ink2),
                                    );
                                    let row = cc.size.as_ref().and_then(|s| f.row(s));
                                    let ls = row.map(|r| f.lengths_of(r)).unwrap_or_default();
                                    let words: Vec<String> =
                                        ls.iter().map(|l| forge_lang::values::fmt(*l)).collect();
                                    let k = cc.length.and_then(|l| ls.iter().position(|x| *x == l));
                                    if let Some(k) = pick_list(
                                        ui,
                                        &t,
                                        "length",
                                        &words,
                                        k,
                                        jump_length,
                                        190.0,
                                        places,
                                    ) {
                                        cc.length = Some(ls[k]);
                                    }
                                });
                            }
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                for (name, label) in [("thread", "Thread"), ("grade", "Grade")] {
                                    let Some((_, vals)) = f.options.iter().find(|o| o.0 == name) else {
                                        continue;
                                    };
                                    ui.label(egui::RichText::new(label).font(medium(11.0)).color(t.ink2));
                                    let k = if name == "thread" {
                                        &mut cc.thread
                                    } else {
                                        &mut cc.grade
                                    };
                                    ui.horizontal_wrapped(|ui| {
                                        for (j, v) in vals.iter().enumerate() {
                                            let r = ui.selectable_label(
                                                *k == j,
                                                egui::RichText::new(v).font(sans(12.0)),
                                            );
                                            places.push((format!("cc:{name}:{v}"), r.rect));
                                            if r.clicked() {
                                                *k = j;
                                            }
                                        }
                                    });
                                    ui.add_space(6.0);
                                }
                                if let Some(s) = cc.spec(f)
                                    && let Ok(g) = generate(&s)
                                {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new("Designation")
                                            .font(medium(11.0))
                                            .color(t.ink2),
                                    );
                                    ui.label(
                                        egui::RichText::new(&g.designation).font(semi(13.0)).color(t.ink),
                                    );
                                    for src in f.all("src").take(1) {
                                        let short: String = src.chars().take(80).collect();
                                        ui.label(
                                            egui::RichText::new(format!("Source: {short}"))
                                                .font(sans(10.0))
                                                .color(t.ink2),
                                        );
                                    }
                                }
                            });
                        });
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if !in_asm {
                        ui.label(
                            egui::RichText::new("Open an assembly to place parts")
                                .font(sans(11.0))
                                .color(t.ink2),
                        );
                    }
                    ui.add_space((ui.available_width() - 64.0 - 8.0 - 64.0).max(0.0));
                    let c = ui.add_sized(vec2(64.0, 28.0), egui::Button::new("Cancel"));
                    places.push(("cc:cancel".into(), c.rect));
                    if c.clicked() {
                        close = true;
                    }
                    let ok = cc.statement(&source).filter(|_| in_asm);
                    let label = egui::RichText::new("Place")
                        .font(semi(13.0))
                        .color(Color32::WHITE);
                    let r = ui.add_enabled(
                        ok.is_some(),
                        egui::Button::new(label).fill(t.accent).min_size(vec2(64.0, 28.0)),
                    );
                    places.push(("cc:place".into(), r.rect));
                    if r.clicked() {
                        place = ok;
                    }
                });
            });
        if let Some(text) = place
            && let Some(fg) = &mut self.fg
        {
            fg.out
                .push(super::fg::FgAct::Apply(Op::AddFeature { before: None, text }));
            close = true;
        }
        if close {
            cc.open = false;
        }
        if let Some(s) = shown {
            self.view_blocks.push(s.response.rect);
        }
        self.places.extend(places);
        self.content = cc;
    }
}
