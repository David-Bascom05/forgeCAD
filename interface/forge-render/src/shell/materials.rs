//! The Material Browser, Inventor's: the libraries (the project's, mine, ForgeCAD's read-only),
//! their materials, and an editor whose tabs other tools add to (`EXTRA_TABS`). ForgeCAD's
//! materials are duplicated to be changed; a changed value says where it came from (cited, user,
//! stand-in). Saved as forge-material's TOML in `~/.config/forgecad/materials/mine.toml` or the
//! project's `materials/project.toml`; Assign to Part sets the part's material.

use super::Shell;
use super::fg::FgAct;
use super::fgasm::dialog_frame;
use super::theme::{Theme, medium, mono, sans};
use egui::{Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
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
pub const EXTRA_TABS: &[ExtraTab] = &[];

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

/// The core values: key, words, unit.
const ROWS: [(&str, &str, &str); 6] = [
    ("density", "Density", "kg/m³"),
    ("youngs", "Young's modulus", "GPa"),
    ("poisson", "Poisson's ratio", ""),
    ("yield", "Yield strength", "MPa"),
    ("uts", "Tensile strength", "MPa"),
    ("cte", "Thermal expansion", "µm/m·K"),
];

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
}

fn hex(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A material's least sure value: what the list's Source column says.
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

fn kind_colour(t: &Theme, k: Kind) -> Color32 {
    match k {
        Kind::Cited => t.ink2,
        Kind::User => Color32::from_rgb(0xF0, 0xB9, 0x5A),
        Kind::StandIn => Color32::from_rgb(0xE0, 0x7A, 0x3A),
    }
}

impl Browser {
    pub fn open(dir: Option<PathBuf>, chosen: Option<String>) -> Browser {
        let libs = Libraries::load(dir.as_deref());
        let chosen = chosen.and_then(|n| libs.find(&n).map(|(l, m)| (l, m.name.clone())));
        Browser {
            libs: Some(Arc::new(libs)),
            dir,
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

    fn save(&mut self) {
        let Some(e) = &self.editing else { return };
        let Some(file) = self.file(e.lib) else {
            self.status = "No folder for that library".into();
            return;
        };
        match forge_material::save(&file, &e.material, e.old.as_deref()) {
            Ok(()) => {
                self.status = format!("Saved to {}", file.display());
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
        let r = egui::Window::new("Material Browser")
            .id(egui::Id::unique("material-browser"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(pos2(v.left() + 20.0, v.top() + 20.0))
            .fixed_size(vec2(1060.0, 600.0))
            .frame(dialog_frame(&t))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Material Browser")
                            .font(medium(15.0))
                            .color(t.ink),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let x =
                            ui.add(egui::Button::new(egui::RichText::new("×").font(sans(16.0))).frame(false));
                        places.push(("materials:close".into(), x.rect));
                        close |= x.clicked();
                        let s = ui.add(
                            egui::TextEdit::singleline(&mut b.search)
                                .desired_width(220.0)
                                .hint_text("Search"),
                        );
                        places.push(("materials:search".into(), s.rect));
                    });
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(220.0);
                        ui.set_min_height(540.0);
                        Self::browser_libraries(ui, &t, &mut b, &mut places)
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(290.0);
                        ui.set_min_height(540.0);
                        Self::browser_list(ui, &t, &mut b, &mut places)
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(500.0);
                        ui.set_min_height(540.0);
                        Self::browser_editor(ui, &t, &mut b, part, &mut places, &mut assign, &mut close)
                    });
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

    fn browser_libraries(ui: &mut Ui, t: &Theme, b: &mut Browser, places: &mut Vec<(String, Rect)>) {
        let Some(libs) = b.libs.clone() else { return };
        ui.label(egui::RichText::new("LIBRARIES").font(sans(11.0)).color(t.ink2));
        let count = |lib: Lib| libs.all().filter(|(s, _)| s.lib == lib).count();
        let mut row = |ui: &mut Ui, on: bool, text: String, n: usize, indent: f32, key: String| -> bool {
            let (r, resp) = ui.allocate_exact_size(vec2(210.0, 24.0), Sense::click());
            if on {
                ui.painter().rect_filled(r, 2.0, t.on.gamma_multiply(0.5));
                ui.painter()
                    .vline(r.left(), r.y_range(), Stroke::new(2.0, t.focus));
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 2.0, t.hover);
            }
            ui.painter().text(
                pos2(r.left() + 10.0 + indent, r.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                sans(12.5),
                if indent > 0.0 { t.ink2 } else { t.ink },
            );
            ui.painter().text(
                pos2(r.right() - 8.0, r.center().y),
                egui::Align2::RIGHT_CENTER,
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
            "All".into(),
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
            .map_or("Project".to_string(), |n| {
                format!("Project · {}", n.to_string_lossy())
            });
        for (lib, name) in [(Lib::Project, project), (Lib::User, "My materials".to_string())] {
            if row(
                ui,
                b.lib == Some(lib) && b.family.is_none(),
                name,
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
            "ForgeCAD · read-only".into(),
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
            let word = format!("{}{}", f[..1].to_uppercase(), &f[1..]);
            if row(ui, on, word, n, 14.0, format!("materials:family:{f}")) {
                b.lib = Some(Lib::ForgeCad);
                b.family = Some(f.to_string());
            }
        }
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let chosen = b.chosen.as_ref().and_then(|(l, n)| {
                libs.all()
                    .find(|(s, m)| s.lib == *l && m.name == *n)
                    .map(|x| x.1.clone())
            });
            let r = ui.button(egui::RichText::new("New").font(sans(12.0)));
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
            }
            let r = ui.add_enabled(
                chosen.is_some(),
                egui::Button::new(egui::RichText::new("Duplicate").font(sans(12.0))),
            );
            places.push(("materials:duplicate".into(), r.rect));
            if r.clicked()
                && let Some(m) = chosen.clone()
            {
                let mut d = m.clone();
                d.from = Some(m.name.clone());
                d.name = format!("{} copy", m.name);
                d.aliases.clear();
                b.editing = Some(Editing {
                    material: d,
                    lib: Lib::User,
                    old: None,
                    changed: true,
                });
                b.tab = 0;
            }
            for (import, word) in [(true, "Import"), (false, "Export")] {
                let r = ui.add_enabled(
                    import || chosen.is_some(),
                    egui::Button::new(egui::RichText::new(word).font(sans(12.0))),
                );
                places.push((format!("materials:{}", word.to_lowercase()), r.rect));
                if r.clicked() {
                    let home = std::env::var("HOME").unwrap_or_default();
                    let name = chosen
                        .as_ref()
                        .map_or("materials".into(), |m| forge_material::norm(&m.name));
                    b.path = Some((import, format!("{home}/{name}.toml")));
                }
            }
        });
        if let Some((import, path)) = &mut b.path {
            let import = *import;
            ui.add(egui::TextEdit::singleline(path).desired_width(200.0));
            let go = ui.button(if import {
                "Import into My materials"
            } else {
                "Export"
            });
            places.push(("materials:path:go".into(), go.rect));
            if go.clicked() {
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
                    let m = b.chosen.as_ref().and_then(|(l, n)| {
                        libs.all()
                            .find(|(s, m)| s.lib == *l && m.name == *n)
                            .map(|x| x.1.clone())
                    });
                    match m.map(|m| forge_material::write(&file, &[m])) {
                        Some(Ok(())) => format!("Exported to {}", file.display()),
                        Some(Err(e)) => e,
                        None => "Choose a material".into(),
                    }
                };
                b.path = None;
                b.reload();
            }
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Searched: project, then mine, then ForgeCAD")
                .font(sans(11.0))
                .color(t.ink2),
        );
    }

    fn browser_list(ui: &mut Ui, t: &Theme, b: &mut Browser, places: &mut Vec<(String, Rect)>) {
        let Some(libs) = b.libs.clone() else { return };
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            ui.label(egui::RichText::new("NAME").font(sans(11.0)).color(t.ink2));
            ui.add_space(170.0);
            ui.label(egui::RichText::new("SOURCE").font(sans(11.0)).color(t.ink2));
        });
        let q = forge_material::norm(&b.search);
        egui::ScrollArea::vertical()
            .id_salt("material-list")
            .max_height(520.0)
            .show(ui, |ui| {
                for (s, m) in libs.all() {
                    if b.lib.is_some_and(|l| l != s.lib) || b.family.as_ref().is_some_and(|f| *f != m.family)
                    {
                        continue;
                    }
                    if !q.is_empty()
                        && !std::iter::once(&m.name)
                            .chain(&m.aliases)
                            .any(|n| forge_material::norm(n).contains(&q))
                    {
                        continue;
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(280.0, 26.0), Sense::click());
                    let on = b.chosen.as_ref().is_some_and(|c| c.0 == s.lib && c.1 == m.name);
                    if on {
                        ui.painter().rect_filled(r, 2.0, t.on.gamma_multiply(0.5));
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 2.0, t.hover);
                    }
                    let sw = Rect::from_center_size(pos2(r.left() + 13.0, r.center().y), vec2(14.0, 14.0));
                    ui.painter().rect_filled(sw, 2.0, hex(m.colour));
                    ui.painter()
                        .rect_stroke(sw, 2.0, Stroke::new(1.0, t.line), egui::StrokeKind::Inside);
                    let mut name = m.name.clone();
                    if s.lib != Lib::ForgeCad
                        && libs
                            .all()
                            .any(|(o, x)| o.lib == Lib::ForgeCad && x.name == m.name)
                    {
                        name.push_str(" (overrides ForgeCAD)");
                    }
                    ui.painter().text(
                        pos2(r.left() + 28.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        name,
                        sans(12.5),
                        t.ink,
                    );
                    let k = kind_of(m);
                    ui.painter().text(
                        pos2(r.right() - 8.0, r.center().y),
                        egui::Align2::RIGHT_CENTER,
                        k.word(),
                        sans(11.0),
                        kind_colour(t, k),
                    );
                    places.push((format!("materials:item:{}", m.name), r));
                    if resp.clicked() {
                        b.chosen = Some((s.lib, m.name.clone()));
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

    #[allow(clippy::too_many_arguments)]
    fn browser_editor(
        ui: &mut Ui,
        t: &Theme,
        b: &mut Browser,
        part: bool,
        places: &mut Vec<(String, Rect)>,
        assign: &mut Option<String>,
        close: &mut bool,
    ) {
        let Some(libs) = b.libs.clone() else { return };
        // what's shown: the material being changed, else the one chosen (read-only)
        let shown = match &b.editing {
            Some(e) => Some((e.lib, e.material.clone(), true)),
            None => b
                .chosen
                .as_ref()
                .and_then(|(l, n)| libs.all().find(|(s, m)| s.lib == *l && m.name == *n))
                .map(|(s, m)| (s.lib, m.clone(), s.lib != Lib::ForgeCad)),
        };
        let Some((lib, mut m, editable)) = shown else {
            ui.label(
                egui::RichText::new("Choose a material, or make a new one")
                    .font(sans(12.0))
                    .color(t.ink2),
            );
            return;
        };
        let before = m.clone();
        ui.label(egui::RichText::new(&m.name).font(medium(16.0)).color(t.ink));
        let changed_n = m.kinds.iter().filter(|k| k.1 != Kind::Cited).count();
        let mut sub = lib.label().to_string();
        if let Some(f) = &m.from {
            sub.push_str(&format!(" · made from {f}"));
        }
        if changed_n > 0 {
            sub.push_str(&format!(" · {changed_n} values not cited"));
        }
        ui.label(egui::RichText::new(sub).font(sans(12.0)).color(t.ink2));
        let extra: Vec<&ExtraTab> = EXTRA_TABS.iter().filter(|x| (x.applies)(&m)).collect();
        let mut tabs: Vec<&str> = vec!["Identity", "Physical", "Appearance"];
        tabs.extend(extra.iter().map(|x| x.title));
        tabs.push("Sources");
        b.tab = b.tab.min(tabs.len() - 1);
        ui.horizontal(|ui| {
            for (k, name) in tabs.iter().enumerate() {
                let r = ui.selectable_label(b.tab == k, egui::RichText::new(*name).font(sans(12.5)));
                places.push((format!("materials:tab:{name}"), r.rect));
                if r.clicked() {
                    b.tab = k;
                }
            }
        });
        ui.separator();
        let mut edited = false;
        ui.add_enabled_ui(editable, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("material-editor")
                .max_height(380.0)
                .show(ui, |ui| {
                    match b.tab {
                        0 => {
                            egui::Grid::new("mb-identity")
                                .num_columns(2)
                                .spacing(vec2(14.0, 6.0))
                                .show(ui, |ui| {
                                    ui.label("Name");
                                    edited |= ui
                                        .add(egui::TextEdit::singleline(&mut m.name).desired_width(300.0))
                                        .changed();
                                    ui.end_row();
                                    ui.label("Aliases");
                                    let mut a = m.aliases.join(", ");
                                    if ui
                                        .add(egui::TextEdit::singleline(&mut a).desired_width(300.0))
                                        .changed()
                                    {
                                        m.aliases = a
                                            .split(',')
                                            .map(|x| x.trim().to_string())
                                            .filter(|x| !x.is_empty())
                                            .collect();
                                        edited = true;
                                    }
                                    ui.end_row();
                                    ui.label("Family");
                                    egui::ComboBox::from_id_salt("mb-family")
                                        .selected_text(&m.family)
                                        .show_ui(ui, |ui| {
                                            for f in FAMILIES {
                                                edited |= ui
                                                    .selectable_value(&mut m.family, f.to_string(), f)
                                                    .changed();
                                            }
                                        });
                                    ui.end_row();
                                    ui.label("Made from");
                                    ui.label(m.from.clone().unwrap_or_else(|| "—".into()));
                                    ui.end_row();
                                    ui.label("Note");
                                    let mut n = m.note.clone().unwrap_or_default();
                                    if ui
                                        .add(
                                            egui::TextEdit::multiline(&mut n)
                                                .desired_width(300.0)
                                                .desired_rows(4),
                                        )
                                        .changed()
                                    {
                                        m.note = (!n.is_empty()).then_some(n);
                                        edited = true;
                                    }
                                    ui.end_row();
                                });
                        }
                        1 => {
                            egui::Grid::new("mb-physical")
                                .num_columns(4)
                                .spacing(vec2(12.0, 6.0))
                                .show(ui, |ui| {
                                    for h in ["PROPERTY", "VALUE", "", "SOURCE"] {
                                        ui.label(egui::RichText::new(h).font(sans(11.0)).color(t.ink2));
                                    }
                                    ui.end_row();
                                    for (key, words, unit) in ROWS {
                                        ui.label(egui::RichText::new(words).font(sans(12.5)).color(t.ink2));
                                        let mut v = m.value(key).unwrap_or(0.0);
                                        let had = m.value(key).is_some();
                                        let r = ui.add(
                                            egui::DragValue::new(&mut v).speed(0.1).max_decimals(4).suffix(
                                                if unit.is_empty() {
                                                    String::new()
                                                } else {
                                                    format!(" {unit}")
                                                },
                                            ),
                                        );
                                        places.push((format!("materials:value:{key}"), r.rect));
                                        if r.changed() {
                                            m.set_value(key, Some(v));
                                            let src = m.source_of(key).to_string();
                                            let src = if m.kind_of(key) == Kind::Cited {
                                                "user".to_string()
                                            } else {
                                                src
                                            };
                                            m.set_source(key, &src, Kind::User);
                                            edited = true;
                                        }
                                        if !had {
                                            ui.label(
                                                egui::RichText::new("not given")
                                                    .font(sans(11.0))
                                                    .color(t.ink2),
                                            );
                                        } else {
                                            ui.label("");
                                        }
                                        let k = m.kind_of(key);
                                        let mut src = m.source_of(key).to_string();
                                        ui.horizontal(|ui| {
                                            let mut kind = k;
                                            egui::ComboBox::from_id_salt(("mb-kind", key))
                                                .selected_text(
                                                    egui::RichText::new(k.word()).color(kind_colour(t, k)),
                                                )
                                                .width(78.0)
                                                .show_ui(ui, |ui| {
                                                    for x in [Kind::Cited, Kind::User, Kind::StandIn] {
                                                        ui.selectable_value(&mut kind, x, x.word());
                                                    }
                                                });
                                            let r = ui.add(
                                                egui::TextEdit::singleline(&mut src)
                                                    .desired_width(200.0)
                                                    .font(sans(11.5)),
                                            );
                                            if kind != k || r.changed() {
                                                m.set_source(key, &src, kind);
                                                edited = true;
                                            }
                                        });
                                        ui.end_row();
                                    }
                                });
                        }
                        2 => {
                            ui.horizontal(|ui| {
                                ui.label("Colour");
                                let mut c = m.colour;
                                if ui.color_edit_button_srgb(&mut c).changed() {
                                    m.colour = c;
                                    edited = true;
                                }
                            });
                            let (r, _) = ui.allocate_exact_size(vec2(220.0, 120.0), Sense::hover());
                            // a lit swatch: the colour, lighter towards the light
                            let c = hex(m.colour);
                            ui.painter().rect_filled(r, 6.0, c);
                            ui.painter().rect_filled(
                                Rect::from_min_max(r.min, pos2(r.max.x, r.center().y)),
                                6.0,
                                Color32::from_white_alpha(28),
                            );
                            ui.label(
                                egui::RichText::new("The model's tint for parts of this material")
                                    .font(sans(11.0))
                                    .color(t.ink2),
                            );
                        }
                        k if k < 3 + extra.len() => {
                            edited |= (extra[k - 3].ui)(ui, t, &mut m);
                        }
                        _ => {
                            ui.label(egui::RichText::new("Record").font(medium(12.0)).color(t.ink));
                            edited |= ui
                                .add(
                                    egui::TextEdit::multiline(&mut m.source)
                                        .desired_width(460.0)
                                        .desired_rows(3),
                                )
                                .changed();
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("By value").font(medium(12.0)).color(t.ink));
                            egui::Grid::new("mb-sources")
                                .num_columns(3)
                                .spacing(vec2(12.0, 4.0))
                                .show(ui, |ui| {
                                    let keys: Vec<String> = ROWS
                                        .iter()
                                        .map(|r| r.0.to_string())
                                        .filter(|k| m.value(k).is_some())
                                        .chain(m.extra.iter().map(|x| x.0.clone()))
                                        .collect();
                                    for k in keys {
                                        let kind = m.kind_of(&k);
                                        ui.label(egui::RichText::new(&k).font(mono(11.5)).color(t.ink2));
                                        ui.label(
                                            egui::RichText::new(kind.word())
                                                .font(sans(11.5))
                                                .color(kind_colour(t, kind)),
                                        );
                                        ui.label(
                                            egui::RichText::new(m.source_of(&k))
                                                .font(sans(11.5))
                                                .color(t.ink),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    }
                });
        });
        if edited && let Some(e) = &mut b.editing {
            e.material = m.clone();
            e.changed = true;
        }
        let _ = before;
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            let note = match &b.editing {
                Some(e) if e.changed => format!(
                    "Unsaved · saves to {}",
                    b.file(e.lib).map_or("—".into(), |f| f.display().to_string())
                ),
                _ if !editable => "ForgeCAD's library is read-only: Duplicate to change it".into(),
                _ => b.status.clone(),
            };
            ui.label(egui::RichText::new(note).font(sans(11.0)).color(t.ink2));
        });
        ui.horizontal(|ui| {
            if let Some(e) = &mut b.editing {
                ui.label(egui::RichText::new("Save to").font(sans(12.0)).color(t.ink2));
                egui::ComboBox::from_id_salt("mb-save-to")
                    .selected_text(e.lib.label())
                    .show_ui(ui, |ui| {
                        for l in [Lib::User, Lib::Project] {
                            ui.selectable_value(&mut e.lib, l, l.label());
                        }
                    });
                let r = ui.button(egui::RichText::new("Save").font(sans(12.5)));
                places.push(("materials:save".into(), r.rect));
                if r.clicked() {
                    b.save();
                }
            }
            let r = ui.button(egui::RichText::new("Close").font(sans(12.5)));
            places.push(("materials:cancel".into(), r.rect));
            *close |= r.clicked();
            let ok = part && !b.editing.as_ref().is_some_and(|e| e.changed);
            let r = ui.add_enabled(
                ok,
                egui::Button::new(
                    egui::RichText::new("Assign to Part")
                        .font(medium(12.5))
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0x2F, 0x6F, 0x4F)),
            );
            places.push(("materials:assign".into(), r.rect));
            if r.clicked() {
                *assign = Some(m.name.clone());
            }
        });
    }
}
