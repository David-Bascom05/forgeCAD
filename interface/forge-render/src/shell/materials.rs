//! The Material Browser, Inventor's: the libraries (the project's, mine, ForgeCAD's read-only),
//! their materials, and a detail view whose tabs other tools add to (`EXTRA_TABS`). ForgeCAD's
//! materials are duplicated to be changed; a changed value says where it came from (cited, user,
//! stand-in). Saved as forge-material's TOML in `~/.config/forgecad/materials/mine.toml` or the
//! project's `materials/project.toml`; Assign to Part sets the part's material.

use super::Shell;
use super::fg::FgAct;
use super::fgasm::dialog_frame;
use super::icons;
use super::theme::{Theme, medium, mono, sans, semi};
use egui::{Align2, Color32, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_lang::error::Op;
use forge_material::{Kind, Lib, Libraries, Material};
use std::path::PathBuf;
use std::sync::Arc;

/// An editor tab another tool adds: its title, whether a material has anything for it, and its
/// contents, which may change the material (true when they did).
pub struct ExtraTab {
    pub title: &'static str,
    pub applies: fn(&Material) -> bool,
    pub ui: fn(&mut egui::Ui, &Theme, &mut Material) -> bool,
}

/// The tabs after Identity, Physical and Appearance, before Sources: thermoforming adds its own
/// here (its values are `thermo.*` keys of the material).
pub const EXTRA_TABS: &[ExtraTab] = &[ExtraTab {
    title: "Thermoform",
    applies: super::thermo_material::applies,
    ui: super::thermo_material::tab,
}];

const FAMILIES: [&str; 9] = [
    "steel",
    "stainless",
    "aluminium",
    "copper",
    "iron",
    "titanium",
    "plastic",
    "rubber",
    "wood",
];

/// The core values: key, words, unit, the group they're shown in.
const ROWS: [(&str, &str, &str, &str); 6] = [
    ("density", "Density", "kg/m³", "Mechanical"),
    ("youngs", "Young's modulus", "GPa", "Mechanical"),
    ("poisson", "Poisson's ratio", "", "Mechanical"),
    ("yield", "Yield strength", "MPa", "Strength"),
    ("uts", "Tensile strength", "MPa", "Strength"),
    ("cte", "Thermal expansion", "µm/m·K", "Thermal"),
];

/// Words for the thermoforming keys, for the Sources tab.
const THERMO_WORDS: [(&str, &str); 15] = [
    ("window", "Forming window"),
    ("normal", "Normal temp"),
    ("shrink", "Shrink"),
    ("draft_male", "Draft male"),
    ("draft_female", "Draft female"),
    ("min_wall", "Min wall"),
    ("max_adr", "Max areal draw ratio"),
    ("max_hd", "Max H:D"),
    ("min_radius", "Min radius"),
    ("min_inside_radius", "Min inside radius"),
    ("max_vent", "Max vent"),
    ("law", "Stretch law"),
    ("mu", "Stretch law moduli"),
    ("alpha", "Stretch law exponents"),
    ("law_temp", "Stretch law temperature"),
];

const W: f32 = 1160.0;
/// The tallest it gets: on a short screen, what's left.
const H: f32 = 660.0;
const HEAD_H: f32 = 48.0;
const LIBS_W: f32 = 214.0;
const LIST_W: f32 = 300.0;
const FOOT_H: f32 = 56.0;

/// A material being changed: where it'll be saved, the name it was saved under.
#[derive(Clone, Debug)]
pub struct Editing {
    pub material: Material,
    pub lib: Lib,
    pub old: Option<String>,
    pub changed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Browser {
    pub search: String,
    /// The library chosen at the left (none: all), and a ForgeCAD family under it.
    pub lib: Option<Lib>,
    pub family: Option<String>,
    /// The material chosen in the list: its library and name.
    pub chosen: Option<(Lib, String)>,
    pub editing: Option<Editing>,
    pub tab: usize,
    /// The file Import reads or Export writes, being typed.
    pub path: Option<(bool, String)>,
    pub status: String,
    pub libs: Option<Arc<Libraries>>,
    pub dir: Option<PathBuf>,
    /// The part's material, by the library's name.
    pub assigned: Option<String>,
}

fn hex(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn lerp(a: Color32, b: Color32, f: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// The user's colour: amber, darker on a light page.
pub(super) fn amber(t: &Theme) -> Color32 {
    if t.dark {
        Color32::from_rgb(0xE0, 0xA0, 0x30)
    } else {
        Color32::from_rgb(0x9A, 0x62, 0x00)
    }
}

fn kind_colour(t: &Theme, k: Kind) -> Color32 {
    match k {
        Kind::Cited => t.ink2,
        Kind::User => amber(t),
        Kind::StandIn if t.dark => Color32::from_rgb(0xE8, 0x80, 0x48),
        Kind::StandIn => Color32::from_rgb(0xB0, 0x48, 0x14),
    }
}

/// A material's least sure value: what the list marks.
fn kind_of(m: &Material) -> Kind {
    let mut k = Kind::Cited;
    for (_, x) in &m.kinds {
        if *x == Kind::StandIn {
            return Kind::StandIn;
        }
        k = *x;
    }
    k
}

pub(super) fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

/// A number as a person writes it: no trailing zeros, at most four decimals.
pub(super) fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// A source's text and its link, apart.
fn split_url(s: &str) -> (String, Option<String>) {
    let Some(at) = s.find("http") else {
        return (s.trim().to_string(), None);
    };
    let end = s[at..].find(char::is_whitespace).map_or(s.len(), |e| at + e);
    let url = s[at..end].trim_end_matches([',', ')', '.', ';']).to_string();
    let cut = [' ', ',', ';', ':'];
    let before = s[..at].trim_end_matches(cut);
    let after = s[end..].trim_start_matches(cut);
    let text = match (before.is_empty(), after.is_empty()) {
        (false, false) => format!("{before}; {after}"),
        _ => format!("{before}{after}"),
    };
    (text, Some(url))
}

/// A source in a few words: up to its first comma, with the year it gives.
fn cite(s: &str) -> String {
    let (text, _) = split_url(s);
    let head = text.split([',', ';', '(', '/']).next().unwrap_or(&text).trim();
    let year = s
        .as_bytes()
        .windows(4)
        .enumerate()
        .find(|(k, w)| {
            w.iter().all(u8::is_ascii_digit)
                && matches!((w[0], w[1]), (b'1', b'9') | (b'2', b'0'))
                && !s.as_bytes().get(k + 4).is_some_and(u8::is_ascii_digit)
                && (*k == 0 || !s.as_bytes()[k - 1].is_ascii_digit())
        })
        .map(|(_, w)| String::from_utf8_lossy(w).into_owned());
    let mut out: String = head.chars().take(44).collect();
    if head.chars().count() > 44 {
        out.push('…');
    }
    match year {
        Some(y) if !out.contains(&y) => format!("{out} {y}"),
        _ => out,
    }
}

/// A key in words: the core values', thermoforming's, else the key tidied.
fn key_words(key: &str) -> String {
    if let Some(r) = ROWS.iter().find(|r| r.0 == key) {
        return r.1.into();
    }
    if let Some(k) = key.strip_prefix("thermo.") {
        if let Some(w) = THERMO_WORDS.iter().find(|w| w.0 == k) {
            return w.1.into();
        }
        return capital(&k.replace('_', " "));
    }
    capital(&key.replace(['_', '.'], " "))
}

/// A lit ball of a colour: what a material looks like.
pub(super) fn ball(p: &Painter, c: Pos2, r: f32, col: Color32) {
    let dark = lerp(col, Color32::BLACK, 0.5);
    let light = lerp(col, Color32::WHITE, 0.35);
    p.circle_filled(c, r, dark);
    let n = (r * 0.9).clamp(8.0, 48.0) as usize;
    for i in 1..=n {
        let f = i as f32 / n as f32;
        let k = if f < 0.55 {
            lerp(dark, col, f / 0.55)
        } else {
            lerp(col, light, (f - 0.55) / 0.45)
        };
        p.circle_filled(c + vec2(-0.30, -0.34) * r * f, r * (1.0 - 0.78 * f), k);
    }
    p.circle_filled(
        c + vec2(-0.36, -0.40) * r,
        r * 0.10,
        Color32::from_white_alpha(60),
    );
    p.circle_stroke(c, r, Stroke::new(1.0, Color32::from_black_alpha(70)));
}

/// A small rounded label: its rect, from its left edge (or its right, `right`).
pub(super) fn pill(p: &Painter, at: Pos2, right: bool, text: &str, ink: Color32, fill: Color32) -> Rect {
    let g = p.layout_no_wrap(text.to_string(), medium(10.5), ink);
    let size = vec2(g.size().x + 14.0, 18.0);
    let min = if right {
        at - vec2(size.x, size.y / 2.0)
    } else {
        at - vec2(0.0, size.y / 2.0)
    };
    let r = Rect::from_min_size(min, size);
    p.rect_filled(r, 9.0, fill);
    p.rect_stroke(
        r,
        9.0,
        Stroke::new(1.0, ink.gamma_multiply(0.45)),
        StrokeKind::Inside,
    );
    p.galley(pos2(r.left() + 7.0, r.center().y - g.size().y / 2.0), g, ink);
    r
}

/// Text cut to a width with an ellipsis, all of it on hover.
pub(super) fn clipped(ui: &mut Ui, r: Rect, text: &str, font: egui::FontId, ink: Color32, id: &str) {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, ink);
    job.wrap = egui::text::TextWrapping::truncate_at_width(r.width());
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let cut = g.elided;
    ui.painter()
        .galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, ink);
    if cut {
        ui.interact(r, ui.scope_id().with(("clipped", id)), Sense::hover())
            .on_hover_text(text);
    }
}

/// A section's heading: its words and a rule to the right edge.
pub(super) fn section(ui: &mut Ui, t: &Theme, text: &str) {
    ui.add_space(10.0);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_uppercase(), semi(10.5), t.ink2));
    let right = r.left() + g.size().x + 10.0;
    ui.painter()
        .galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, t.ink2);
    ui.painter()
        .hline(right..=r.right(), r.center().y, Stroke::new(1.0, t.line));
    ui.add_space(4.0);
}

/// Tabs along a rule, the chosen one underlined; true when one was clicked.
pub(super) fn tab_strip(
    ui: &mut Ui,
    t: &Theme,
    names: &[&str],
    on: &mut usize,
    key: &str,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let w = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
    ui.painter()
        .hline(row.x_range(), row.bottom() - 0.5, Stroke::new(1.0, t.line));
    let mut x = row.left();
    let mut clicked = false;
    for (k, name) in names.iter().enumerate() {
        let g = ui.fonts_mut(|f| f.layout_no_wrap(name.to_string(), medium(12.5), t.ink));
        let r = Rect::from_min_size(pos2(x, row.top()), vec2(g.size().x + 26.0, row.height()));
        x = r.right() + 2.0;
        let resp = ui.interact(r, ui.scope_id().with((key, k)), Sense::click());
        let sel = *on == k;
        if sel {
            ui.painter().rect_filled(
                r.shrink2(vec2(0.0, 1.0)),
                egui::CornerRadius {
                    nw: 4,
                    ne: 4,
                    sw: 0,
                    se: 0,
                },
                t.on,
            );
            ui.painter()
                .hline(r.x_range(), r.bottom() - 1.0, Stroke::new(2.0, t.on_ink));
        } else if resp.hovered() {
            ui.painter().rect_filled(
                r.shrink2(vec2(0.0, 1.0)),
                egui::CornerRadius {
                    nw: 4,
                    ne: 4,
                    sw: 0,
                    se: 0,
                },
                t.hover,
            );
        }
        let ink = if sel { t.ink } else { t.ink2 };
        ui.painter()
            .text(r.center(), Align2::CENTER_CENTER, *name, medium(12.5), ink);
        places.push((format!("{key}:{name}"), r));
        if resp.clicked() {
            *on = k;
            clicked = true;
        }
    }
    clicked
}

/// A button of a size at a place; `primary` is the dialog's main action.
pub(super) fn button_at(
    ui: &mut Ui,
    t: &Theme,
    r: Rect,
    text: &str,
    primary: bool,
    enabled: bool,
) -> Response {
    let words = if primary {
        egui::RichText::new(text).font(medium(12.5)).color(Color32::WHITE)
    } else {
        egui::RichText::new(text).font(sans(12.5))
    };
    let mut b = egui::Button::new(words).min_size(r.size());
    if primary {
        b = b.fill(t.accent);
    }
    ui.put(r, |ui: &mut Ui| ui.add_enabled(enabled, b))
}

impl Browser {
    pub fn open(dir: Option<PathBuf>, chosen: Option<String>) -> Browser {
        let libs = Libraries::load(dir.as_deref());
        let chosen = chosen.and_then(|n| libs.find(&n).map(|(l, m)| (l, m.name.clone())));
        Browser {
            libs: Some(Arc::new(libs)),
            dir,
            assigned: chosen.as_ref().map(|c| c.1.clone()),
            chosen,
            ..Browser::default()
        }
    }

    fn reload(&mut self) {
        self.libs = Some(Arc::new(Libraries::load(self.dir.as_deref())));
    }

    fn file(&self, lib: Lib) -> Option<PathBuf> {
        match lib {
            Lib::User => forge_material::user_dir().map(|d| d.join("mine.toml")),
            Lib::Project => self
                .dir
                .as_ref()
                .map(|d| forge_material::project_dir(d).join("project.toml")),
            Lib::ForgeCad => None,
        }
    }

    /// A file as the footer says it: home as `~`.
    fn short(&self, f: &std::path::Path) -> String {
        let s = f.display().to_string();
        match std::env::var("HOME") {
            Ok(h) if !h.is_empty() && s.starts_with(&h) => format!("~{}", &s[h.len()..]),
            _ => s,
        }
    }

    fn save(&mut self) {
        let Some(e) = &self.editing else { return };
        let Some(file) = self.file(e.lib) else {
            self.status = "No folder for that library".into();
            return;
        };
        match forge_material::save(&file, &e.material, e.old.as_deref()) {
            Ok(()) => {
                self.status = format!("Saved to {}", self.short(&file));
                self.chosen = Some((e.lib, e.material.name.clone()));
                if let Some(e) = &mut self.editing {
                    e.old = Some(e.material.name.clone());
                    e.changed = false;
                }
                self.reload();
            }
            Err(why) => self.status = why,
        }
    }

    fn chosen_material(&self) -> Option<Material> {
        let libs = self.libs.as_ref()?;
        let (l, n) = self.chosen.as_ref()?;
        libs.all()
            .find(|(s, m)| s.lib == *l && m.name == *n)
            .map(|x| x.1.clone())
    }

    fn duplicate(&mut self, m: &Material) {
        let mut d = m.clone();
        d.from = Some(m.name.clone());
        d.name = format!("{} copy", m.name);
        d.aliases.clear();
        self.editing = Some(Editing {
            material: d,
            lib: Lib::User,
            old: None,
            changed: true,
        });
        self.status.clear();
    }
}

impl Shell {
    /// The Material Browser, when it's up.
    pub(super) fn materials_ui(&mut self, ctx: &egui::Context) {
        let Some(mut b) = self.inspect.browser.take() else {
            return;
        };
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = vec![];
        let mut close = false;
        let mut assign: Option<String> = None;
        let part = !self.inspect.assembly;
        // placed where it fits whole: egui lays a window out where it was put, then moves it
        let area = super::dialog_area(ctx);
        let h = H.min(area.height() - 16.0).max(420.0);
        let r = egui::Window::new("Material Browser")
            .id(egui::Id::unique("material-browser"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(pos2(
                (v.left() + 20.0).min(area.right() - W - 8.0).max(area.left()),
                (v.top() + 20.0).min(area.bottom() - h - 8.0).max(area.top()),
            ))
            .fixed_size(vec2(W, h))
            .frame(dialog_frame(&t).inner_margin(0))
            .constrain_to(area)
            .show(ctx, |ui| {
                let all = Rect::from_min_size(ui.max_rect().min, vec2(W, h));
                ui.allocate_rect(all, Sense::hover());
                let head = Rect::from_min_size(all.min, vec2(W, HEAD_H));
                let body = Rect::from_min_max(pos2(all.left(), head.bottom()), all.max);
                let libs = Rect::from_min_size(body.min, vec2(LIBS_W, body.height()));
                let list = Rect::from_min_size(pos2(libs.right(), body.top()), vec2(LIST_W, body.height()));
                let detail = Rect::from_min_max(pos2(list.right(), body.top()), body.max);
                let p = ui.painter().clone();
                p.rect_filled(
                    libs,
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 6,
                        se: 0,
                    },
                    t.raised,
                );
                p.hline(all.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.line));
                p.vline(libs.right() - 0.5, body.y_range(), Stroke::new(1.0, t.line));
                p.vline(list.right() - 0.5, body.y_range(), Stroke::new(1.0, t.line));
                close |= Self::browser_head(ui, &t, head, &mut b, &mut places);
                let region = |r: Rect, m: egui::Vec2| egui::UiBuilder::new().max_rect(r.shrink2(m));
                ui.scope_builder(region(libs, vec2(10.0, 10.0)), |ui| {
                    Self::browser_libraries(ui, &t, &mut b, &mut places)
                });
                ui.scope_builder(region(list, vec2(0.0, 0.0)), |ui| {
                    Self::browser_list(ui, &t, &mut b, &mut places)
                });
                ui.scope_builder(region(detail, vec2(0.0, 0.0)), |ui| {
                    Self::browser_detail(ui, &t, &mut b, part, &mut places, &mut assign, &mut close)
                });
            });
        if let Some(r) = r {
            self.view_blocks.push(r.response.rect);
        }
        self.places.extend(places);
        if let Some(name) = assign
            && let Some(fg) = &mut self.fg
        {
            fg.out.push(FgAct::Apply(Op::SetProperty {
                key: "material".into(),
                text: Some(forge_lang::props::quote(&name)),
            }));
            self.inspect.want_physical = true;
        }
        if !close {
            self.inspect.browser = Some(b);
        }
    }

    /// The title, the search and the close box; true when closed.
    fn browser_head(
        ui: &mut Ui,
        t: &Theme,
        head: Rect,
        b: &mut Browser,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        ui.painter().text(
            pos2(head.left() + 16.0, head.center().y),
            Align2::LEFT_CENTER,
            "Material Browser",
            semi(15.0),
            t.ink,
        );
        let x = Rect::from_center_size(pos2(head.right() - 22.0, head.center().y), vec2(22.0, 22.0));
        let r = ui.interact(x, ui.scope_id().with("mb-close"), Sense::click());
        icons::draw(
            ui.painter(),
            x.left_top() + vec2(3.0, 3.0),
            16.0,
            "close",
            1.5,
            if r.hovered() { t.ink } else { t.ink2 },
        );
        places.push(("materials:close".into(), x));
        let field = Rect::from_min_size(
            pos2(x.left() - 12.0 - 260.0, head.center().y - 14.0),
            vec2(260.0, 28.0),
        );
        let s = ui.put(
            field,
            egui::TextEdit::singleline(&mut b.search)
                .font(sans(12.5))
                .margin(vec2(8.0, 6.0))
                .hint_text("Search names and aliases"),
        );
        places.push(("materials:search".into(), s.rect));
        r.clicked()
    }

    fn browser_libraries(ui: &mut Ui, t: &Theme, b: &mut Browser, places: &mut Vec<(String, Rect)>) {
        let Some(libs) = b.libs.clone() else { return };
        let area = ui.max_rect();
        let w = area.width();
        ui.painter().text(
            pos2(area.left() + 6.0, area.top() + 8.0),
            Align2::LEFT_CENTER,
            "LIBRARIES",
            semi(10.5),
            t.ink2,
        );
        ui.add_space(20.0);
        let count = |lib: Lib| libs.all().filter(|(s, _)| s.lib == lib).count();
        let mut row =
            |ui: &mut Ui, on: bool, text: &str, tag: &str, n: usize, indent: f32, key: String| -> bool {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click());
                if on {
                    ui.painter().rect_filled(r, 4.0, t.on);
                    ui.painter().vline(
                        r.left() + 1.0,
                        r.y_range().shrink(4.0),
                        Stroke::new(2.0, t.on_ink),
                    );
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 4.0, t.hover);
                }
                let ink = if indent > 0.0 && !on { t.ink2 } else { t.ink };
                let font = if indent > 0.0 { sans(12.5) } else { medium(12.5) };
                let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, ink));
                let gw = g.size().x;
                ui.painter().galley(
                    pos2(r.left() + 10.0 + indent, r.center().y - g.size().y / 2.0),
                    g,
                    ink,
                );
                if !tag.is_empty() {
                    ui.painter().text(
                        pos2(r.left() + 16.0 + indent + gw, r.center().y + 0.5),
                        Align2::LEFT_CENTER,
                        tag,
                        sans(10.5),
                        t.ink2,
                    );
                }
                ui.painter().text(
                    pos2(r.right() - 8.0, r.center().y),
                    Align2::RIGHT_CENTER,
                    n.to_string(),
                    sans(11.0),
                    t.ink2,
                );
                places.push((key, r));
                resp.clicked()
            };
        if row(
            ui,
            b.lib.is_none(),
            "All",
            "",
            libs.all().count(),
            0.0,
            "materials:lib:all".into(),
        ) {
            b.lib = None;
            b.family = None;
        }
        let project = b
            .dir
            .as_ref()
            .and_then(|d| d.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (lib, name, tag) in [
            (Lib::Project, "Project", project.as_str()),
            (Lib::User, "My materials", ""),
        ] {
            if row(
                ui,
                b.lib == Some(lib) && b.family.is_none(),
                name,
                tag,
                count(lib),
                0.0,
                format!("materials:lib:{}", lib.label()),
            ) {
                b.lib = Some(lib);
                b.family = None;
            }
        }
        let forge = b.lib == Some(Lib::ForgeCad);
        if row(
            ui,
            forge && b.family.is_none(),
            "ForgeCAD",
            "read-only",
            count(Lib::ForgeCad),
            0.0,
            "materials:lib:forgecad".into(),
        ) {
            b.lib = Some(Lib::ForgeCad);
            b.family = None;
        }
        for f in FAMILIES {
            let n = libs
                .all()
                .filter(|(s, m)| s.lib == Lib::ForgeCad && m.family == f)
                .count();
            if n == 0 {
                continue;
            }
            let on = forge && b.family.as_deref() == Some(f);
            if row(ui, on, &capital(f), "", n, 14.0, format!("materials:family:{f}")) {
                b.lib = Some(Lib::ForgeCad);
                b.family = Some(f.to_string());
            }
        }
        // the library's tools, at the foot of the column
        let chosen = b.chosen_material();
        let bw = (w - 6.0) / 2.0;
        let foot = area.bottom() - 2.0 * 28.0 - 6.0;
        let path_h = if b.path.is_some() { 72.0 } else { 0.0 };
        if let Some((import, path)) = &mut b.path {
            let import = *import;
            let top = foot - path_h;
            let f = Rect::from_min_size(pos2(area.left(), top), vec2(w, 28.0));
            ui.put(
                f,
                egui::TextEdit::singleline(path)
                    .font(mono(11.0))
                    .margin(vec2(6.0, 6.0)),
            );
            let go = Rect::from_min_size(pos2(area.left(), top + 34.0), vec2(w, 28.0));
            let r = button_at(
                ui,
                t,
                go,
                if import {
                    "Import into My materials"
                } else {
                    "Export"
                },
                false,
                true,
            );
            places.push(("materials:path:go".into(), r.rect));
            if r.clicked() {
                let file = PathBuf::from(path.as_str());
                b.status = if import {
                    match std::fs::read_to_string(&file)
                        .map_err(|e| e.to_string())
                        .and_then(|s| {
                            forge_material::parse(&s).map_err(|e| format!("line {}: {}", e.line, e.why))
                        }) {
                        Ok(list) => {
                            let mine = b.file(Lib::User);
                            let n = list.len();
                            let r = mine.ok_or("no user folder".to_string()).and_then(|f| {
                                list.iter().try_for_each(|m| forge_material::save(&f, m, None))
                            });
                            match r {
                                Ok(()) => format!("Imported {n} into My materials"),
                                Err(e) => e,
                            }
                        }
                        Err(e) => e,
                    }
                } else {
                    match chosen
                        .as_ref()
                        .map(|m| forge_material::write(&file, std::slice::from_ref(m)))
                    {
                        Some(Ok(())) => format!("Exported to {}", b.short(&file)),
                        Some(Err(e)) => e,
                        None => "Choose a material".into(),
                    }
                };
                b.path = None;
                b.reload();
            }
        }
        let cell = |k: usize| {
            Rect::from_min_size(
                pos2(
                    area.left() + (k % 2) as f32 * (bw + 6.0),
                    foot + (k / 2) as f32 * 34.0,
                ),
                vec2(bw, 28.0),
            )
        };
        let r = button_at(ui, t, cell(0), "New", false, true);
        places.push(("materials:new".into(), r.rect));
        if r.clicked() {
            let mut m = forge_material::find("1008 CR")
                .cloned()
                .unwrap_or_else(|| libs.all().next().expect("a library").1.clone());
            m.name = "New material".into();
            m.aliases.clear();
            m.from = None;
            m.source = "user".into();
            m.sources.clear();
            m.kinds = forge_material::VALUES
                .iter()
                .map(|k| (k.to_string(), Kind::StandIn))
                .collect();
            m.extra.clear();
            m.note = None;
            b.editing = Some(Editing {
                material: m,
                lib: Lib::User,
                old: None,
                changed: true,
            });
            b.tab = 0;
            b.status.clear();
        }
        let r = button_at(ui, t, cell(1), "Duplicate", false, chosen.is_some());
        places.push(("materials:duplicate".into(), r.rect));
        if r.clicked()
            && let Some(m) = &chosen
        {
            b.duplicate(m);
            b.tab = 0;
        }
        for (k, import, word) in [(2, true, "Import"), (3, false, "Export")] {
            let r = button_at(ui, t, cell(k), word, false, import || chosen.is_some());
            places.push((format!("materials:{}", word.to_lowercase()), r.rect));
            if r.clicked() {
                let home = std::env::var("HOME").unwrap_or_default();
                let name = chosen
                    .as_ref()
                    .map_or("materials".into(), |m| forge_material::norm(&m.name));
                b.path = match &b.path {
                    Some((i, _)) if *i == import => None,
                    _ => Some((import, format!("{home}/{name}.toml"))),
                };
            }
        }
    }

    fn browser_list(ui: &mut Ui, t: &Theme, b: &mut Browser, places: &mut Vec<(String, Rect)>) {
        let Some(libs) = b.libs.clone() else { return };
        let area = ui.max_rect();
        let q = forge_material::norm(&b.search);
        let shown: Vec<_> = libs
            .all()
            .filter(|(s, m)| {
                b.lib.is_none_or(|l| l == s.lib) && b.family.as_ref().is_none_or(|f| *f == m.family)
            })
            .filter(|(_, m)| {
                q.is_empty()
                    || std::iter::once(&m.name)
                        .chain(&m.aliases)
                        .any(|n| forge_material::norm(n).contains(&q))
            })
            .collect();
        let (hr, _) = ui.allocate_exact_size(vec2(area.width(), 34.0), Sense::hover());
        let what = if q.is_empty() {
            format!(
                "{} material{}",
                shown.len(),
                if shown.len() == 1 { "" } else { "s" }
            )
        } else {
            format!("{} found", shown.len())
        };
        ui.painter().text(
            pos2(hr.left() + 14.0, hr.center().y),
            Align2::LEFT_CENTER,
            what.to_uppercase(),
            semi(10.5),
            t.ink2,
        );
        ui.painter()
            .hline(hr.x_range(), hr.bottom() - 0.5, Stroke::new(1.0, t.line));
        // the part's material: the first by the search order
        let on_part = b
            .assigned
            .as_ref()
            .and_then(|n| libs.find(n))
            .map(|(l, m)| (l, m.name.clone()));
        let overrides = |m: &Material| {
            libs.all()
                .any(|(o, x)| o.lib == Lib::ForgeCad && x.name == m.name)
        };
        egui::ScrollArea::vertical()
            .id_salt("material-list")
            .auto_shrink([false, false])
            .max_height(area.height() - 34.0)
            .show(ui, |ui| {
                let w = ui.available_width();
                let mut group = String::new();
                for (s, m) in shown {
                    // headers: a library, or ForgeCAD's family
                    let g = if s.lib == Lib::ForgeCad {
                        if b.lib.is_none() {
                            format!("ForgeCAD · {}", capital(&m.family))
                        } else {
                            capital(&m.family)
                        }
                    } else {
                        s.lib.label().to_string()
                    };
                    if g != group && b.family.is_none() && b.lib.is_none_or(|l| l == Lib::ForgeCad) {
                        let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
                        ui.painter().text(
                            pos2(r.left() + 14.0, r.bottom() - 8.0),
                            Align2::LEFT_CENTER,
                            &g,
                            medium(11.0),
                            t.ink2,
                        );
                        group = g;
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
                    let on = b.chosen.as_ref().is_some_and(|c| c.0 == s.lib && c.1 == m.name);
                    if on {
                        ui.painter().rect_filled(r, 0.0, t.on);
                        ui.painter()
                            .vline(r.left() + 1.0, r.y_range(), Stroke::new(2.0, t.on_ink));
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 0.0, t.hover);
                    }
                    ball(
                        ui.painter(),
                        pos2(r.left() + 26.0, r.center().y),
                        11.0,
                        hex(m.colour),
                    );
                    let x = r.left() + 46.0;
                    let mut right = r.right() - 12.0;
                    let k = kind_of(m);
                    if k != Kind::Cited {
                        let c = kind_colour(t, k);
                        right = pill(
                            ui.painter(),
                            pos2(right, r.center().y),
                            true,
                            k.word(),
                            c,
                            c.gamma_multiply(0.12),
                        )
                        .left()
                            - 6.0;
                    }
                    if on_part.as_ref().is_some_and(|(l, n)| *l == s.lib && *n == m.name) {
                        right = pill(
                            ui.painter(),
                            pos2(right, r.center().y),
                            true,
                            "on part",
                            t.on_ink,
                            t.on_ink.gamma_multiply(0.12),
                        )
                        .left()
                            - 6.0;
                    }
                    let name_r = Rect::from_min_max(pos2(x, r.top() + 4.0), pos2(right, r.center().y + 1.0));
                    clipped(ui, name_r, &m.name, sans(13.0), t.ink, &m.name);
                    let mut sub = format!("{} · {} kg/m³", capital(&m.family), num(m.density));
                    if s.lib != Lib::ForgeCad && overrides(m) {
                        sub.push_str(" · overrides ForgeCAD");
                    }
                    let sub_r =
                        Rect::from_min_max(pos2(x, r.center().y + 1.0), pos2(right, r.bottom() - 4.0));
                    clipped(ui, sub_r, &sub, sans(11.0), t.ink2, &format!("{}-sub", m.name));
                    places.push((format!("materials:item:{}", m.name), r));
                    if resp.clicked() {
                        b.chosen = Some((s.lib, m.name.clone()));
                        b.status.clear();
                        b.editing = (s.lib != Lib::ForgeCad).then(|| Editing {
                            material: m.clone(),
                            lib: s.lib,
                            old: Some(m.name.clone()),
                            changed: false,
                        });
                    }
                }
            });
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn browser_detail(
        ui: &mut Ui,
        t: &Theme,
        b: &mut Browser,
        part: bool,
        places: &mut Vec<(String, Rect)>,
        assign: &mut Option<String>,
        close: &mut bool,
    ) {
        let Some(libs) = b.libs.clone() else { return };
        let area = ui.max_rect();
        // what's shown: the material being changed, else the one chosen (read-only)
        let shown = match &b.editing {
            Some(e) => Some((e.lib, e.material.clone(), true)),
            None => b
                .chosen
                .as_ref()
                .and_then(|(l, n)| libs.all().find(|(s, m)| s.lib == *l && m.name == *n))
                .map(|(s, m)| (s.lib, m.clone(), s.lib != Lib::ForgeCad)),
        };
        let foot = Rect::from_min_max(pos2(area.left(), area.bottom() - FOOT_H), area.max);
        let Some((lib, mut m, editable)) = shown else {
            ui.painter().text(
                area.center() - vec2(0.0, FOOT_H / 2.0),
                Align2::CENTER_CENTER,
                "Choose a material",
                sans(13.0),
                t.ink2,
            );
            Self::browser_foot(ui, t, b, foot, None, part, places, assign, close);
            return;
        };
        let unsaved = b.editing.as_ref().is_some_and(|e| e.changed);
        let pad = 20.0;
        let inner = area.shrink2(vec2(pad, 0.0));
        let p = ui.painter().clone();
        // the head: a ball of its colour, its name, where it's kept
        let top = area.top() + 18.0;
        ball(&p, pos2(inner.left() + 26.0, top + 26.0), 26.0, hex(m.colour));
        let tx = inner.left() + 68.0;
        let mut right = inner.right();
        let dup = Rect::from_min_size(pos2(right - 92.0, top + 2.0), vec2(92.0, 28.0));
        let r = button_at(ui, t, dup, "Duplicate", false, true);
        places.push(("materials:detail:duplicate".into(), r.rect));
        if r.clicked() {
            b.duplicate(&m);
            b.tab = 0;
            return;
        }
        right = dup.left() - 10.0;
        let (badge, ink) = if !editable {
            ("Read-only", t.ink2)
        } else if unsaved {
            ("Unsaved", amber(t))
        } else {
            (lib.label(), t.on_ink)
        };
        let br = pill(
            &p,
            pos2(right, dup.center().y),
            true,
            badge,
            ink,
            ink.gamma_multiply(0.10),
        );
        places.push(("materials:badge".into(), br));
        let name_r = Rect::from_min_max(pos2(tx, top), pos2(br.left() - 10.0, top + 30.0));
        clipped(ui, name_r, &m.name, semi(19.0), t.ink, "mb-name");
        let mut sub = vec![capital(&m.family)];
        sub.push(match lib {
            Lib::ForgeCad => "ForgeCAD library".into(),
            l => l.label().to_string(),
        });
        if let Some(f) = &m.from {
            sub.push(format!("made from {f}"));
        }
        let off = m.kinds.iter().filter(|k| k.1 != Kind::Cited).count();
        if off > 0 {
            sub.push(format!(
                "{off} value{} not cited",
                if off == 1 { "" } else { "s" }
            ));
        }
        let sub_r = Rect::from_min_max(pos2(tx, top + 32.0), pos2(inner.right(), top + 50.0));
        clipped(ui, sub_r, &sub.join(" · "), sans(12.0), t.ink2, "mb-sub");
        // the values a choice is made on, at a glance
        let tiles_top = top + 70.0;
        let tw = (inner.width() - 3.0 * 8.0) / 4.0;
        for (k, (key, words, unit)) in [
            ("density", "Density", "kg/m³"),
            ("youngs", "Young's modulus", "GPa"),
            ("yield", "Yield strength", "MPa"),
            ("uts", "Tensile strength", "MPa"),
        ]
        .iter()
        .enumerate()
        {
            let r = Rect::from_min_size(
                pos2(inner.left() + k as f32 * (tw + 8.0), tiles_top),
                vec2(tw, 50.0),
            );
            p.rect_filled(r, 4.0, t.raised);
            p.rect_stroke(r, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            p.text(
                r.left_top() + vec2(10.0, 14.0),
                Align2::LEFT_CENTER,
                *words,
                sans(11.0),
                t.ink2,
            );
            match m.value(key) {
                Some(x) => {
                    let g = p.layout_no_wrap(num(x), medium(16.0), t.ink);
                    let gw = g.size().x;
                    p.galley(pos2(r.left() + 10.0, r.bottom() - 10.0 - g.size().y), g, t.ink);
                    p.text(
                        pos2(r.left() + 14.0 + gw, r.bottom() - 13.0),
                        Align2::LEFT_BOTTOM,
                        *unit,
                        sans(11.0),
                        t.ink2,
                    );
                }
                None => {
                    p.text(
                        pos2(r.left() + 10.0, r.bottom() - 10.0),
                        Align2::LEFT_BOTTOM,
                        "—",
                        medium(16.0),
                        t.ink2,
                    );
                }
            }
        }
        let extra: Vec<&ExtraTab> = EXTRA_TABS.iter().filter(|x| (x.applies)(&m)).collect();
        let mut tabs: Vec<&str> = vec!["Identity", "Physical", "Appearance"];
        tabs.extend(extra.iter().map(|x| x.title));
        tabs.push("Sources");
        b.tab = b.tab.min(tabs.len() - 1);
        let tabs_r = Rect::from_min_max(
            pos2(inner.left(), tiles_top + 66.0),
            pos2(inner.right(), tiles_top + 98.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(tabs_r), |ui| {
            tab_strip(ui, t, &tabs, &mut b.tab, "materials:tab", places)
        });
        let body = Rect::from_min_max(
            pos2(inner.left(), tabs_r.bottom() + 6.0),
            pos2(area.right(), foot.top()),
        );
        let mut edited = false;
        let tab = b.tab;
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .id_salt(("material-detail", tab))
                .auto_shrink([false, false])
                .max_height(body.height())
                .show(ui, |ui| {
                    ui.set_width(inner.width());
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    match tab {
                        0 => {
                            edited |= Self::identity_tab(
                                ui,
                                t,
                                &mut m,
                                lib,
                                b.file(lib).map(|f| b.short(&f)),
                                editable,
                                places,
                            )
                        }
                        1 => edited |= Self::values_tab(ui, t, &mut m, editable, places),
                        2 => edited |= Self::appearance_tab(ui, t, &mut m, editable, places),
                        k if k < 3 + extra.len() => {
                            ui.add_space(8.0);
                            ui.scope(|ui| {
                                if !editable {
                                    ui.visuals_mut().disabled_alpha = 0.85;
                                    ui.disable();
                                }
                                edited |= (extra[k - 3].ui)(ui, t, &mut m);
                            });
                        }
                        _ => edited |= Self::sources_tab(ui, t, &mut m, editable),
                    }
                    ui.add_space(12.0);
                });
        });
        if edited && let Some(e) = &mut b.editing {
            e.material = m.clone();
            e.changed = true;
        }
        Self::browser_foot(ui, t, b, foot, Some(&m), part, places, assign, close);
    }

    #[allow(clippy::too_many_arguments)]
    fn browser_foot(
        ui: &mut Ui,
        t: &Theme,
        b: &mut Browser,
        foot: Rect,
        m: Option<&Material>,
        part: bool,
        places: &mut Vec<(String, Rect)>,
        assign: &mut Option<String>,
        close: &mut bool,
    ) {
        ui.painter()
            .hline(foot.x_range(), foot.top() + 0.5, Stroke::new(1.0, t.line));
        let y = foot.center().y;
        let mut x = foot.right() - 20.0;
        let mut at = |w: f32| {
            let r = Rect::from_min_size(pos2(x - w, y - 15.0), vec2(w, 30.0));
            x = r.left() - 8.0;
            r
        };
        let unsaved = b.editing.as_ref().is_some_and(|e| e.changed);
        let name = m.map(|m| m.name.clone());
        let on = name.is_some() && name == b.assigned;
        let r = button_at(
            ui,
            t,
            at(124.0),
            if on && part { "Assigned" } else { "Assign to Part" },
            true,
            part && name.is_some() && !unsaved && !on,
        );
        places.push(("materials:assign".into(), r.rect));
        if r.clicked()
            && let Some(n) = &name
        {
            *assign = Some(n.clone());
            b.assigned = Some(n.clone());
            b.status = format!("{n} assigned to the part");
        }
        let r = button_at(ui, t, at(76.0), "Close", false, true);
        places.push(("materials:cancel".into(), r.rect));
        *close |= r.clicked();
        let mut left = foot.left() + 20.0;
        let right = x;
        if b.editing.is_some() {
            let g = ui.fonts_mut(|f| f.layout_no_wrap("Save to".into(), sans(12.0), t.ink2));
            let gw = g.size().x;
            ui.painter().galley(pos2(left, y - g.size().y / 2.0), g, t.ink2);
            let combo = Rect::from_min_size(pos2(left + gw + 8.0, y - 13.0), vec2(130.0, 26.0));
            if let Some(e) = &mut b.editing {
                ui.scope_builder(egui::UiBuilder::new().max_rect(combo), |ui| {
                    egui::ComboBox::from_id_salt("mb-save-to")
                        .selected_text(e.lib.label())
                        .width(combo.width())
                        .show_ui(ui, |ui| {
                            for l in [Lib::User, Lib::Project] {
                                ui.selectable_value(&mut e.lib, l, l.label());
                            }
                        });
                });
            }
            let s = Rect::from_min_size(pos2(combo.right() + 8.0, y - 15.0), vec2(64.0, 30.0));
            let r = button_at(ui, t, s, "Save", false, unsaved);
            places.push(("materials:save".into(), r.rect));
            if r.clicked() {
                b.save();
            }
            left = s.right() + 12.0;
        }
        if !b.status.is_empty() {
            let r = Rect::from_min_max(pos2(left, y - 10.0), pos2(right, y + 10.0));
            clipped(ui, r, &b.status.clone(), sans(12.0), t.ink2, "mb-note");
        }
    }

    /// A labelled row of the Identity tab: the words, then what `f` puts at the right.
    fn field_row<R>(ui: &mut Ui, t: &Theme, words: &str, h: f32, f: impl FnOnce(&mut Ui, Rect) -> R) -> R {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        ui.painter().text(
            pos2(r.left(), r.top() + 15.0),
            Align2::LEFT_CENTER,
            words,
            sans(12.5),
            t.ink2,
        );
        let value = Rect::from_min_max(
            pos2(r.left() + 140.0, r.top() + 2.0),
            pos2(r.right() - 4.0, r.bottom() - 2.0),
        );
        f(ui, value)
    }

    fn identity_tab(
        ui: &mut Ui,
        t: &Theme,
        m: &mut Material,
        lib: Lib,
        file: Option<String>,
        editable: bool,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let mut edited = false;
        ui.add_space(8.0);
        let rh = if editable { 36.0 } else { 32.0 };
        let text = |ui: &mut Ui, r: Rect, s: &str, font: egui::FontId| {
            ui.painter().text(
                pos2(r.left(), r.top() + 13.0),
                Align2::LEFT_CENTER,
                s,
                font,
                t.ink,
            );
        };
        let field = |ui: &mut Ui, r: Rect, s: &mut String, hint: &str| {
            ui.put(
                Rect::from_min_size(r.min, vec2(r.width(), 26.0)),
                egui::TextEdit::singleline(s)
                    .font(sans(12.5))
                    .margin(vec2(8.0, 5.0))
                    .hint_text(hint),
            )
        };
        Self::field_row(ui, t, "Name", rh, |ui, r| {
            if editable {
                let resp = field(ui, r, &mut m.name, "");
                places.push(("materials:name".into(), resp.rect));
                edited |= resp.changed();
            } else {
                text(ui, r, &m.name, medium(13.0));
            }
        });
        if editable || !m.aliases.is_empty() {
            let h = if editable {
                rh
            } else {
                Self::chips_height(ui, &m.aliases)
            };
            Self::field_row(ui, t, "Aliases", h, |ui, r| {
                if editable {
                    let mut a = m.aliases.join(", ");
                    if field(ui, r, &mut a, "Other names, comma-separated").changed() {
                        m.aliases = a
                            .split(',')
                            .map(|x| x.trim().to_string())
                            .filter(|x| !x.is_empty())
                            .collect();
                        edited = true;
                    }
                } else {
                    Self::chips(ui, t, r, &m.aliases);
                }
            });
        }
        Self::field_row(ui, t, "Family", rh, |ui, r| {
            if editable {
                ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
                    egui::ComboBox::from_id_salt("mb-family")
                        .selected_text(capital(&m.family))
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for f in FAMILIES {
                                edited |= ui
                                    .selectable_value(&mut m.family, f.to_string(), capital(f))
                                    .changed();
                            }
                        });
                });
            } else {
                text(ui, r, &capital(&m.family), sans(13.0));
            }
        });
        if let Some(f) = m.from.clone() {
            Self::field_row(ui, t, "Made from", 32.0, |ui, r| text(ui, r, &f, sans(13.0)));
        }
        section(ui, t, "Library");
        Self::field_row(ui, t, "Kept in", 32.0, |ui, r| {
            text(
                ui,
                r,
                match lib {
                    Lib::ForgeCad => "ForgeCAD, built in",
                    l => l.label(),
                },
                sans(13.0),
            )
        });
        if let Some(s) = file {
            Self::field_row(ui, t, "File", 32.0, |ui, r| {
                clipped(
                    ui,
                    Rect::from_min_size(r.min, vec2(r.width(), 26.0)),
                    &s,
                    mono(11.5),
                    t.ink,
                    "mb-file",
                )
            });
        }
        edited
    }

    fn chips_layout(ui: &Ui, names: &[String], w: f32) -> Vec<(Rect, Arc<egui::Galley>)> {
        let mut out = vec![];
        let (mut x, mut y) = (0.0, 2.0);
        for n in names {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(n.clone(), sans(12.0), Color32::PLACEHOLDER));
            let cw = g.size().x + 16.0;
            if x > 0.0 && w > 0.0 && x + cw > w {
                x = 0.0;
                y += 28.0;
            }
            out.push((Rect::from_min_size(pos2(x, y), vec2(cw, 22.0)), g));
            x += cw + 6.0;
        }
        out
    }

    fn chips_height(ui: &Ui, names: &[String]) -> f32 {
        let w = ui.available_width() - 144.0;
        let rows = Self::chips_layout(ui, names, w).last().map_or(0.0, |c| c.0.top());
        rows + 22.0 + 10.0
    }

    /// Names as rounded chips, wrapped in a rect.
    fn chips(ui: &mut Ui, t: &Theme, r: Rect, names: &[String]) {
        let p = ui.painter();
        for (c, g) in Self::chips_layout(ui, names, r.width()) {
            let c = c.translate(r.min.to_vec2());
            p.rect_filled(c, 11.0, t.raised);
            p.rect_stroke(c, 11.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            p.galley(pos2(c.left() + 8.0, c.center().y - g.size().y / 2.0), g, t.ink);
        }
    }

    /// The Physical tab: each value with its unit and source.
    fn values_tab(
        ui: &mut Ui,
        t: &Theme,
        m: &mut Material,
        editable: bool,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let mut edited = false;
        let mut group = "";
        for (key, words, unit, g) in ROWS {
            if g != group {
                section(ui, t, g);
                group = g;
            }
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
            let p = ui.painter().clone();
            let y = r.center().y;
            p.text(pos2(r.left(), y), Align2::LEFT_CENTER, words, sans(13.0), t.ink);
            let field =
                Rect::from_min_max(pos2(r.left() + 168.0, y - 13.0), pos2(r.left() + 280.0, y + 13.0));
            let v = m.value(key);
            let k = m.kind_of(key);
            if editable {
                let ek = egui::Id::unique(("mb-value", key));
                let mut s: String = ui
                    .ctx()
                    .data_mut(|d| d.get_temp(ek))
                    .unwrap_or_else(|| v.map(num).unwrap_or_default());
                let resp = ui.put(
                    field,
                    egui::TextEdit::singleline(&mut s)
                        .font(mono(12.5))
                        .margin(vec2(8.0, 5.0))
                        .align(Align2::RIGHT_CENTER)
                        .hint_text("—"),
                );
                if k != Kind::Cited {
                    p.rect_stroke(
                        field,
                        4.0,
                        Stroke::new(1.0, kind_colour(t, k).gamma_multiply(0.8)),
                        StrokeKind::Inside,
                    );
                }
                places.push((format!("materials:value:{key}"), resp.rect));
                if resp.changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(ek, s.clone()));
                }
                if resp.lost_focus() {
                    let got = s.trim().replace(',', ".").parse::<f64>().ok();
                    let ok = got.is_some() || (s.trim().is_empty() && key != "density");
                    if ok && got != v {
                        m.set_value(key, got);
                        m.set_source(key, "user", Kind::User);
                        edited = true;
                    }
                    ui.ctx().data_mut(|d| d.remove::<String>(ek));
                }
            } else {
                let (text, ink) = match v {
                    Some(x) => (num(x), t.ink),
                    None => ("—".to_string(), t.ink2),
                };
                p.text(
                    pos2(field.right() - 8.0, y),
                    Align2::RIGHT_CENTER,
                    text,
                    mono(13.0),
                    ink,
                );
                places.push((format!("materials:value:{key}"), field));
            }
            p.text(
                pos2(field.right() + 8.0, y),
                Align2::LEFT_CENTER,
                unit,
                sans(12.0),
                t.ink2,
            );
            if v.is_some() {
                let src = Rect::from_min_max(pos2(r.left() + 352.0, y - 9.0), pos2(r.right() - 4.0, y + 9.0));
                let c = kind_colour(t, k);
                p.circle_filled(pos2(src.left() + 3.0, y), 3.0, c);
                let words = match k {
                    Kind::Cited => cite(m.source_of(key)),
                    Kind::User => format!("user · {}", cite(m.source_of(key))),
                    Kind::StandIn => format!("stand-in · {}", cite(m.source_of(key))),
                };
                let words = words.trim_end_matches(" · user").to_string();
                let full = m.source_of(key).to_string();
                clipped(
                    ui,
                    Rect::from_min_max(pos2(src.left() + 12.0, src.top()), src.max),
                    &words,
                    sans(12.0),
                    if k == Kind::Cited { t.ink2 } else { c },
                    key,
                );
                ui.interact(src, ui.scope_id().with(("mb-src", key)), Sense::hover())
                    .on_hover_text(full);
            }
        }
        edited
    }

    fn appearance_tab(
        ui: &mut Ui,
        t: &Theme,
        m: &mut Material,
        editable: bool,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let mut edited = false;
        section(ui, t, "Colour");
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 170.0), Sense::hover());
        let p = ui.painter().clone();
        let well = Rect::from_min_size(r.min + vec2(0.0, 6.0), vec2(170.0, 160.0));
        p.rect_filled(well, 6.0, if t.dark { t.bg } else { t.raised });
        p.rect_stroke(well, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        ball(&p, well.center(), 62.0, hex(m.colour));
        let x = well.right() + 24.0;
        let c = m.colour;
        let rows: [(&str, String); 2] = [
            ("Hex", format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])),
            ("RGB", format!("{} {} {}", c[0], c[1], c[2])),
        ];
        let sw = Rect::from_min_size(pos2(x + 90.0, well.top() + 4.0), vec2(44.0, 26.0));
        p.text(
            pos2(x, sw.center().y),
            Align2::LEFT_CENTER,
            "Model tint",
            sans(12.5),
            t.ink2,
        );
        if editable {
            let mut cc = m.colour;
            let resp = ui.put(sw, |ui: &mut Ui| ui.color_edit_button_srgb(&mut cc));
            places.push(("materials:colour".into(), resp.rect));
            if cc != m.colour {
                m.colour = cc;
                edited = true;
            }
        } else {
            p.rect_filled(sw, 4.0, hex(m.colour));
            p.rect_stroke(sw, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        }
        for (k, (words, val)) in rows.iter().enumerate() {
            let y = sw.center().y + 34.0 * (k + 1) as f32;
            p.text(pos2(x, y), Align2::LEFT_CENTER, *words, sans(12.5), t.ink2);
            p.text(pos2(x + 90.0, y), Align2::LEFT_CENTER, val, mono(13.0), t.ink);
        }
        edited
    }

    /// Every value that has a source of its own, and the record's.
    fn sources_tab(ui: &mut Ui, t: &Theme, m: &mut Material, editable: bool) -> bool {
        let mut edited = false;
        section(ui, t, "Record");
        let w = ui.available_width() - 4.0;
        if editable {
            edited |= ui
                .add(
                    egui::TextEdit::multiline(&mut m.source)
                        .font(sans(12.5))
                        .margin(vec2(8.0, 6.0))
                        .desired_width(w)
                        .desired_rows(3)
                        .hint_text("Where the values come from: a datasheet, a handbook, a test"),
                )
                .changed();
        } else {
            Self::citation(ui, t, &m.source, w, t.ink);
        }
        let keys: Vec<String> = ROWS
            .iter()
            .map(|r| r.0.to_string())
            .filter(|k| m.value(k).is_some())
            .chain(m.extra.iter().map(|x| x.0.clone()))
            .filter(|k| editable || m.source_of(k) != m.source || m.kind_of(k) != Kind::Cited)
            .collect();
        if keys.is_empty() {
            return edited;
        }
        section(ui, t, "Values");
        for k in keys {
            let kind = m.kind_of(&k);
            let c = kind_colour(t, kind);
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(170.0, 26.0), Sense::hover());
                ui.painter().text(
                    pos2(r.left(), r.center().y),
                    Align2::LEFT_CENTER,
                    key_words(&k),
                    sans(12.5),
                    t.ink,
                );
                let (r, _) = ui.allocate_exact_size(vec2(84.0, 26.0), Sense::hover());
                if editable {
                    let mut nk = kind;
                    ui.scope_builder(egui::UiBuilder::new().max_rect(r.shrink2(vec2(0.0, 1.0))), |ui| {
                        egui::ComboBox::from_id_salt(("mb-kind", &k))
                            .selected_text(egui::RichText::new(kind.word()).color(c))
                            .width(76.0)
                            .show_ui(ui, |ui| {
                                for x in [Kind::Cited, Kind::User, Kind::StandIn] {
                                    ui.selectable_value(&mut nk, x, x.word());
                                }
                            });
                    });
                    let mut src = m.source_of(&k).to_string();
                    let rw = ui.available_width() - 4.0;
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut src)
                            .font(sans(12.0))
                            .margin(vec2(6.0, 5.0))
                            .desired_width(rw),
                    );
                    if nk != kind || resp.changed() {
                        m.set_source(&k, &src, nk);
                        edited = true;
                    }
                } else {
                    pill(
                        ui.painter(),
                        pos2(r.left(), r.center().y),
                        false,
                        kind.word(),
                        c,
                        c.gamma_multiply(0.12),
                    );
                    let rw = ui.available_width() - 4.0;
                    let s = if m.source_of(&k) == m.source {
                        "the record's source".to_string()
                    } else {
                        m.source_of(&k).to_string()
                    };
                    ui.vertical(|ui| {
                        ui.add_space(5.0);
                        Self::citation(ui, t, &s, rw, t.ink2);
                    });
                }
            });
            ui.add_space(6.0);
        }
        edited
    }

    /// Words wrapped to the width left, from the left, lines a little apart.
    fn paragraph(ui: &mut Ui, s: &str, font: egui::FontId, ink: Color32) {
        let w = ui.available_width() - 4.0;
        let mut job = egui::text::LayoutJob::single_section(
            s.to_string(),
            egui::TextFormat {
                font_id: font,
                color: ink,
                line_height: Some(18.0),
                ..Default::default()
            },
        );
        job.wrap.max_width = w;
        let g = ui.fonts_mut(|f| f.layout_job(job));
        let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
        ui.painter().galley(r.min, g, ink);
    }

    /// A citation: its words wrapped, its link under them.
    fn citation(ui: &mut Ui, t: &Theme, s: &str, w: f32, ink: Color32) {
        let (text, url) = split_url(s);
        if !text.is_empty() {
            ui.scope(|ui| {
                ui.set_max_width(w);
                Self::paragraph(ui, &text, sans(12.5), ink);
            });
        }
        if let Some(u) = url {
            ui.add_space(4.0);
            let shown = u
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .to_string();
            let mut job = egui::text::LayoutJob::simple_singleline(shown, sans(12.0), t.on_ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(w);
            ui.scope(|ui| {
                ui.visuals_mut().hyperlink_color = t.on_ink;
                ui.add(egui::Hyperlink::from_label_and_url(job, &u))
                    .on_hover_text(&u);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_reads_in_a_few_words() {
        let s = "ASM Aerospace Specification Metals / MatWeb, 'Aluminum 6061-T6; 6061-T651', https://www.aerospacemetals.com/wp-content/uploads/2023/06/Aluminum-6061-T6-6061-T651.pdf";
        assert_eq!(cite(s), "ASM Aerospace Specification Metals 2023");
        let (text, url) = split_url(s);
        assert!(url.unwrap().ends_with(".pdf"));
        assert!(!text.contains("http"));
        assert_eq!(cite("user"), "user");
        assert_eq!(cite("tensile 2500 psi"), "tensile 2500 psi");
        assert_eq!(key_words("thermo.max_adr"), "Max areal draw ratio");
        assert_eq!(key_words("uts"), "Tensile strength");
        assert_eq!(num(68.9), "68.9");
        assert_eq!(num(2700.0), "2700");
    }
}
