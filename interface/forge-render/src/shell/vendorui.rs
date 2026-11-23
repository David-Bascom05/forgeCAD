//! Sheet-metal vendors on screen: a window to pick the part's vendor, material and stocked
//! thickness (written into the sheet as `vendor:`, `material:` and its `t`), to copy a built-in
//! profile into the user's own and edit it, import and export; and the DFM panel, the part
//! checked against its vendor (forge-lang's `dfm::report`), each violation listed with its rule,
//! measured value and limit, lit on the model and marked on the flat pattern when chosen.
//!
//! Self-contained until the UI session's shared components land: it draws with the shell's theme
//! tokens and the sheet windows' frame.

use super::Shell;
use super::fg::{FgPanel, From};
use super::placed::Placed;
use super::sheet::SheetAct;
use super::theme::{medium, mono, sans, semi};
use egui::{Color32, Ui, pos2, vec2};
use forge_lang::dfm::Report;
use forge_lang::error::{Literal, Op};
use forge_lang::values::Dim;
use forge_vendor::{Lib, Profiles, Vendor};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

/// A chosen violation's faces and flat marks: violet, apart from the build's red and amber.
pub const LIT: [u8; 4] = [0x9A, 0x5C, 0xF0, 150];
pub const MARK: Color32 = Color32::from_rgb(0xE0, 0x45, 0x40);
pub const CHOSEN: Color32 = Color32::from_rgb(0x9A, 0x5C, 0xF0);

/// A profile being edited: the vendor, the file it saves to, its text when edited as text.
pub struct Edit {
    pub v: Vendor,
    pub file: PathBuf,
    pub material: usize,
    /// The stocks of the material shown, as typed: t, gauge, radii, K, min flange, hole to bend,
    /// hole to edge, min hole.
    pub rows: Vec<[String; 8]>,
    pub text: Option<String>,
    pub error: Option<String>,
}

const COLS: [&str; 8] = [
    "t",
    "gauge",
    "radius",
    "K",
    "min flange",
    "hole-bend",
    "hole-edge",
    "min hole",
];

fn len_text(l: &Option<forge_vendor::Len>) -> String {
    l.as_ref().map(|x| x.text()).unwrap_or_default()
}

fn nums_text(v: &[f64]) -> String {
    v.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(", ")
}

impl Edit {
    fn new(v: Vendor, file: PathBuf) -> Edit {
        let mut e = Edit {
            v,
            file,
            material: 0,
            rows: vec![],
            text: None,
            error: None,
        };
        e.load_rows();
        e
    }

    fn load_rows(&mut self) {
        self.rows = self
            .v
            .materials
            .get(self.material)
            .map(|m| {
                m.stock
                    .iter()
                    .map(|s| {
                        [
                            format!("{}", s.t),
                            s.gauge.clone().unwrap_or_default(),
                            nums_text(&s.radius),
                            s.k.map(|k| format!("{k}")).unwrap_or_default(),
                            len_text(&s.rules.min_flange),
                            len_text(&s.rules.hole_to_bend),
                            len_text(&s.rules.hole_to_edge),
                            len_text(&s.rules.min_hole),
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    /// The rows typed back into the material's stocks.
    fn store_rows(&mut self) -> Result<(), String> {
        let Some(m) = self.v.materials.get_mut(self.material) else {
            return Ok(());
        };
        let num = |s: &str, what: &str| -> Result<Option<f64>, String> {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            s.parse::<f64>()
                .map(Some)
                .map_err(|_| format!("{what} '{s}' is not a number"))
        };
        let len = |s: &str, what: &str| -> Result<Option<forge_vendor::Len>, String> {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            forge_vendor::Len::parse(s)
                .map(Some)
                .map_err(|e| format!("{what}: {e}"))
        };
        let mut stocks = vec![];
        for (k, r) in self.rows.iter().enumerate() {
            let mut s = m.stock.get(k).cloned().unwrap_or_default();
            s.t = num(&r[0], "t")?.ok_or("every stock needs its t")?;
            s.gauge = (!r[1].trim().is_empty()).then(|| r[1].trim().to_string());
            s.radius = r[2]
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(|x| {
                    x.parse::<f64>()
                        .map_err(|_| format!("radius '{x}' is not a number"))
                })
                .collect::<Result<_, _>>()?;
            s.k = num(&r[3], "K")?;
            s.rules.min_flange = len(&r[4], "min flange")?;
            s.rules.hole_to_bend = len(&r[5], "hole to bend")?;
            s.rules.hole_to_edge = len(&r[6], "hole to edge")?;
            s.rules.min_hole = len(&r[7], "min hole")?;
            stocks.push(s);
        }
        m.stock = stocks;
        Ok(())
    }

    /// The profile as it stands, checked by reading its text back.
    fn checked(&mut self) -> Result<Vendor, String> {
        match &self.text {
            Some(t) => Vendor::parse(t).map_err(|e| e.to_string()),
            None => {
                self.store_rows()?;
                Vendor::parse(&self.v.to_text()).map_err(|e| e.to_string())
            }
        }
    }
}

#[derive(Default)]
pub struct VendorUi {
    pub open: bool,
    pub dfm_open: bool,
    profiles: Option<Arc<Profiles>>,
    /// The chosen profile (by id), material and thickness (mm).
    pick: Option<String>,
    material: Option<String>,
    thickness: Option<f64>,
    pub edit: Option<Edit>,
    import: String,
    export: String,
    msg: Option<(bool, String)>,
    pub report: Option<Arc<Report>>,
    running: Option<mpsc::Receiver<Report>>,
    /// The source last checked: the panel checks again when it changes.
    checked: Option<u64>,
    /// The chosen violation: (sheet, item) in the report.
    pub chosen: Option<(usize, usize)>,
}

/// A change to the panel's own state, made once its window is drawn.
type Later = Box<dyn FnOnce(&mut VendorUi)>;

fn hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn doc_dir(fg: &FgPanel) -> Option<PathBuf> {
    std::path::Path::new(&fg.path).parent().map(|p| p.to_path_buf())
}

/// The ops that put a vendor, material and stocked thickness into the part's first sheet: its
/// `t` by its param where a param drives it.
pub fn apply_ops(fg: &FgPanel, vendor: &str, material: &str, t: f64) -> Result<Vec<Op>, String> {
    let sheet = fg
        .feats
        .iter()
        .find(|f| f.kind == "sheet")
        .ok_or("the part has no sheet: make a Face first")?;
    let q = |s: &str| format!("{s:?}");
    let mut ops = vec![
        Op::SetArg {
            id: sheet.id.clone(),
            arg: "vendor".into(),
            text: q(vendor),
        },
        Op::SetArg {
            id: sheet.id.clone(),
            arg: "material".into(),
            text: q(material),
        },
    ];
    let driving = sheet
        .dims
        .iter()
        .find(|d| d.what == "thickness")
        .and_then(|d| match &d.from {
            From::Driving(k) => Some(k.clone()),
            _ => None,
        });
    ops.push(match driving {
        Some(param) => Op::SetParam {
            param,
            value: Literal {
                value: t,
                dim: Dim::Length,
            },
        },
        None => Op::SetArg {
            id: sheet.id.clone(),
            arg: "t".into(),
            text: format!("{t}mm"),
        },
    });
    Ok(ops)
}

impl VendorUi {
    fn profiles(&mut self, fg: &FgPanel) -> Arc<Profiles> {
        self.profiles
            .get_or_insert_with(|| Arc::new(Profiles::load(doc_dir(fg).as_deref())))
            .clone()
    }

    fn reload(&mut self) {
        self.profiles = None;
    }

    /// Checks the part again when its source has changed since the last check (off the window's
    /// thread: a check builds the part), and takes a finished check in.
    pub fn poll(&mut self, fg: &FgPanel, ctx: &egui::Context) {
        if let Some(rx) = &self.running {
            match rx.try_recv() {
                Ok(r) => {
                    self.report = Some(Arc::new(r));
                    self.running = None;
                    if self.chosen.is_some_and(|(s, i)| {
                        self.report
                            .as_ref()
                            .is_none_or(|r| r.sheets.get(s).is_none_or(|x| x.all().get(i).is_none()))
                    }) {
                        self.chosen = None;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100))
                }
                Err(mpsc::TryRecvError::Disconnected) => self.running = None,
            }
            return;
        }
        if !self.dfm_open {
            return;
        }
        let h = hash(&fg.source);
        if self.checked == Some(h) {
            return;
        }
        self.checked = Some(h);
        let (tx, rx) = mpsc::channel();
        let src = fg.source.clone();
        let dir = doc_dir(fg);
        let spawned = std::thread::Builder::new()
            .name("dfm-check".into())
            .spawn(move || {
                let _ = tx.send(forge_lang::dfm::report(&src, dir, None));
            });
        if spawned.is_ok() {
            self.running = Some(rx);
        }
    }

    /// Every violation's flat points on sheet `id`, the chosen one's last: (points, chosen).
    pub fn flat_marks(&self, id: &str) -> Vec<(Vec<[f64; 2]>, bool)> {
        let Some(r) = &self.report else { return vec![] };
        let mut out = vec![];
        for (si, s) in r.sheets.iter().enumerate().filter(|(_, s)| s.sheet == id) {
            for (ii, it) in s.all().into_iter().enumerate() {
                if it.flat.is_empty() || it.severity != "violation" {
                    continue;
                }
                out.push((
                    it.flat.iter().map(|p| [p.x, p.y]).collect(),
                    self.chosen == Some((si, ii)),
                ));
            }
        }
        out.sort_by_key(|x| x.1);
        out
    }

    /// The chosen violation's anchor (whose faces the view lights) and its flat points.
    pub fn lit(&self) -> Option<(&str, &[forge_math::Point2], &str)> {
        let (s, i) = self.chosen?;
        let r = self.report.as_ref()?;
        let sheet = r.sheets.get(s)?;
        let it = *sheet.all().get(i)?;
        Some((it.anchor.as_deref().unwrap_or(""), &it.flat, &sheet.sheet))
    }
}

/// A profile's `source`: links split at ` ; `, a bare `/path` the one before it with its last
/// part replaced (`.../materials/sheet/aluminum ; /steel`).
pub fn sources(s: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for part in s.split(';').map(str::trim).filter(|x| !x.is_empty()) {
        if let Some(rest) = part.strip_prefix('/')
            && let Some(prev) = out.last()
        {
            let base = prev.trim_end_matches('/');
            let cut = base
                .rfind('/')
                .filter(|&k| k > base.find("//").map_or(0, |d| d + 1));
            match cut {
                Some(k) => out.push(format!("{}/{rest}", &base[..k])),
                None => out.push(format!("{base}/{rest}")),
            }
        } else {
            out.push(part.to_string());
        }
    }
    out
}

/// A link as read: no scheme, no `www.`, no trailing `/`.
fn link_label(url: &str) -> &str {
    let u = url.split_once("://").map_or(url, |x| x.1);
    let u = u.strip_prefix("www.").unwrap_or(u);
    u.trim_end_matches('/')
}

/// A rule's code in words: `HoleNearBend` is "Hole near bend".
pub fn rule_words(code: &str) -> String {
    let mut out = String::new();
    for (k, c) in code.chars().enumerate() {
        if k > 0 && c.is_ascii_uppercase() {
            out.push(' ');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A number as a shop reads it: to 0.01, no trailing zeros.
pub fn round2(x: f64) -> String {
    let s = format!("{:.2}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

/// A message's long decimals (more than two places) rounded as `round2` does.
pub fn rounded_text(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut k = 0;
    while k < b.len() {
        let start = k;
        if b[k].is_ascii_digit() && (k == 0 || !(b[k - 1].is_ascii_alphanumeric() || b[k - 1] == b'_')) {
            while k < b.len() && b[k].is_ascii_digit() {
                k += 1;
            }
            if k + 1 < b.len() && b[k] == b'.' && b[k + 1].is_ascii_digit() {
                let dot = k;
                k += 1;
                while k < b.len() && b[k].is_ascii_digit() {
                    k += 1;
                }
                if k - dot - 1 > 2
                    && let Ok(x) = s[start..k].parse::<f64>()
                {
                    out.push_str(&round2(x));
                    continue;
                }
            }
            out.push_str(&s[start..k]);
            continue;
        }
        let ch = s[k..].chars().next().unwrap_or(' ');
        out.push(ch);
        k += ch.len_utf8();
    }
    out
}

fn small(t: &super::theme::Theme, s: impl Into<String>) -> egui::RichText {
    egui::RichText::new(s).font(sans(11.0)).color(t.ink2)
}

impl Shell {
    pub(super) fn vendor_ui(&mut self, ctx: &egui::Context) {
        // the panel lent out for the frame: these windows read it and change only the sheet UI
        let Some(fg) = self.fg.take() else { return };
        self.sheet.vendor.poll(&fg, ctx);
        self.vendor_window(ctx, &fg);
        self.dfm_window(ctx, &fg);
        self.fg = Some(fg);
    }

    fn vendor_window(&mut self, ctx: &egui::Context, fg: &FgPanel) {
        if !self.sheet.vendor.open {
            return;
        }
        let t = self.theme;
        let profiles = self.sheet.vendor.profiles(fg);
        let v = self.view_rect;
        let mut out: Vec<SheetAct> = vec![];
        let ui_state = &mut self.sheet.vendor;
        // the part's own choice, as its last check found it
        let current = ui_state
            .report
            .as_ref()
            .and_then(|r| r.sheets.first())
            .and_then(|s| s.vendor.as_ref())
            .map(|v| format!("{} {}", v.name, v.material));
        if ui_state.pick.is_none() {
            ui_state.pick = profiles.list.first().map(|e| e.vendor.id.clone());
        }
        let mut close = false;
        let mut todo: Option<Later> = None;
        let mut places: super::placed::Places = vec![];
        egui::Window::new("Sheet Metal Vendor")
            .id(egui::Id::unique("sheet-vendor"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.left() + 360.0, v.top() + 48.0))
            .default_width(560.0)
            .frame(super::fgasm::dialog_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                close = super::fgasm::dialog_head(
                    ui,
                    &t,
                    "folder",
                    "Sheet Metal Vendor",
                    "sheet:vendorwin",
                    places,
                );
                if let Some(c) = &current {
                    ui.label(small(&t, format!("This part: {c}")));
                }
                ui.add_space(4.0);
                ui.horizontal_top(|ui| {
                    // the profiles, by where they are kept
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        for lib in [Lib::Project, Lib::User, Lib::ForgeCad] {
                            let here: Vec<_> = profiles.list.iter().filter(|e| e.lib == lib).collect();
                            if here.is_empty() {
                                continue;
                            }
                            ui.label(egui::RichText::new(lib.label()).font(medium(11.0)).color(t.ink2));
                            for e in here {
                                let on = ui_state.pick.as_deref() == Some(e.vendor.id.as_str());
                                if ui
                                    .selectable_label(
                                        on,
                                        egui::RichText::new(&e.vendor.name).font(sans(12.0)),
                                    )
                                    .clicked()
                                {
                                    ui_state.pick = Some(e.vendor.id.clone());
                                    ui_state.material = None;
                                    ui_state.thickness = None;
                                }
                            }
                        }
                        for (f, e) in &profiles.errors {
                            ui.label(
                                egui::RichText::new(format!("unread {}: {e}", f.display()))
                                    .font(sans(10.0))
                                    .color(t.error),
                            );
                        }
                    });
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.set_width(360.0);
                        let Some(e) = ui_state.pick.as_deref().and_then(|id| profiles.find(id)) else {
                            ui.label(small(&t, "No profile chosen"));
                            return;
                        };
                        let vd = &e.vendor;
                        ui.label(egui::RichText::new(&vd.name).font(semi(12.0)).color(t.ink));
                        if let Some(s) = &vd.source {
                            let links = sources(s);
                            ui.label(small(&t, if links.len() == 1 { "Source" } else { "Sources" }));
                            for url in &links {
                                ui.add(egui::Hyperlink::from_label_and_url(
                                    egui::RichText::new(link_label(url))
                                        .font(super::theme::caption())
                                        .color(t.accent)
                                        .underline(),
                                    url,
                                ))
                                .on_hover_text(url);
                            }
                        }
                        ui.label(small(
                            &t,
                            format!("Data of {}", vd.taken.as_deref().unwrap_or("undated")),
                        ));
                        ui.label(
                            egui::RichText::new(forge_vendor::VERIFY)
                                .font(medium(11.0))
                                .color(t.focus),
                        );
                        if let Some(ops) = &vd.operations {
                            ui.add(egui::Label::new(small(&t, format!("Does: {}", ops.join(", ")))).wrap());
                        }
                        ui.add_space(4.0);
                        // material and thickness for the part
                        let names: Vec<String> = vd.materials.iter().map(|m| m.name.clone()).collect();
                        let mat = ui_state
                            .material
                            .clone()
                            .filter(|m| names.contains(m))
                            .or_else(|| names.first().cloned());
                        let fields = egui::Grid::new("vendor-fields")
                            .num_columns(2)
                            .spacing(vec2(super::theme::S2, super::theme::S1))
                            .show(ui, |ui| {
                                ui.label(small(&t, "Material"));
                                egui::ComboBox::from_id_salt("vendor-material")
                                    .width(240.0)
                                    .selected_text(mat.clone().unwrap_or_default())
                                    .show_ui(ui, |ui| {
                                        for n in &names {
                                            if ui.selectable_label(mat.as_deref() == Some(n), n).clicked() {
                                                ui_state.material = Some(n.clone());
                                                ui_state.thickness = None;
                                            }
                                        }
                                    });
                                let m = mat.as_deref().and_then(|n| vd.material(n));
                                let ts = m.map(|m| vd.thicknesses(m)).unwrap_or_default();
                                let shop = m.map(|m| vd.shop(m));
                                let label_of = |x: &(f64, Option<String>)| {
                                    let inch = x.0 / 25.4;
                                    match &x.1 {
                                        Some(g) => format!("{:.3} mm  ({:.3} in, {g})", x.0, inch),
                                        None => format!("{:.3} mm  ({:.3} in)", x.0, inch),
                                    }
                                };
                                ui.end_row();
                                let th = ui_state.thickness.or_else(|| ts.first().map(|x| x.0));
                                ui.label(small(&t, "Thickness"));
                                egui::ComboBox::from_id_salt("vendor-thickness")
                                    .width(240.0)
                                    .selected_text(
                                        ts.iter()
                                            .find(|x| Some(x.0) == th)
                                            .map(label_of)
                                            .unwrap_or_default(),
                                    )
                                    .show_ui(ui, |ui| {
                                        for x in &ts {
                                            if ui.selectable_label(Some(x.0) == th, label_of(x)).clicked() {
                                                ui_state.thickness = Some(x.0);
                                            }
                                        }
                                    });
                                ui.end_row();
                                (shop, th)
                            });
                        let (shop, th) = fields.inner;
                        if let (Some(shop), Some(th)) = (&shop, th)
                            && let Some(s) = shop.stock(th)
                        {
                            let bends = !s.not.contains(&forge_sheet::Operation::Bend)
                                && shop.offers(forge_sheet::Operation::Bend);
                            let text = if !bends {
                                "Cut only: not bent in this stock".to_string()
                            } else if s.radii.is_empty() {
                                "Bends, radius and K not stated: give the sheet its own".to_string()
                            } else {
                                format!(
                                    "Bends at R {} mm, {}",
                                    nums_text(
                                        &s.radii
                                            .iter()
                                            .map(|r| (r * 1000.0).round() / 1000.0)
                                            .collect::<Vec<_>>()
                                    ),
                                    match &s.rule {
                                        forge_sheet::BendRule::K(k) => format!("K {k}"),
                                        _ => "K by table".into(),
                                    }
                                )
                            };
                            ui.label(small(&t, text));
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let can = mat.is_some() && th.is_some();
                            if ui
                                .add_enabled(can, egui::Button::new("Use for this part"))
                                .clicked()
                            {
                                match apply_ops(fg, &vd.id, mat.as_deref().unwrap_or(""), th.unwrap_or(0.0)) {
                                    Ok(ops) => out.push(SheetAct::Apply(ops)),
                                    Err(e) => ui_state.msg = Some((false, e)),
                                }
                            }
                            if ui.button("Check (DFM)").clicked() {
                                ui_state.dfm_open = true;
                                ui_state.checked = None;
                            }
                        });
                        ui.horizontal(|ui| {
                            let editable = e.lib != Lib::ForgeCad;
                            if editable && ui.button("Edit").clicked() {
                                let file = e.file.clone().unwrap_or_default();
                                ui_state.edit = Some(Edit::new(vd.clone(), file));
                            }
                            if ui
                                .button("Duplicate to My vendors")
                                .on_hover_text("A copy in your own folder, to change")
                                .clicked()
                            {
                                let id = format!("{}-copy", vd.id);
                                match forge_vendor::user_dir() {
                                    Some(dir) => {
                                        let mut c = vd.clone();
                                        c.id = id.clone();
                                        c.name = format!("{} (my copy)", vd.name);
                                        let file = dir.join(format!("{id}.toml"));
                                        match forge_vendor::save(&file, &c) {
                                            Ok(()) => {
                                                todo = Some(Box::new(move |u: &mut VendorUi| {
                                                    u.reload();
                                                    u.pick = Some(id);
                                                    u.edit = Some(Edit::new(c, file));
                                                }));
                                            }
                                            Err(err) => ui_state.msg = Some((false, err)),
                                        }
                                    }
                                    None => {
                                        ui_state.msg =
                                            Some((false, "no config folder on this machine".into()))
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut ui_state.export)
                                    .hint_text("file.toml")
                                    .desired_width(180.0),
                            );
                            if ui.button("Export").clicked() {
                                let f = ui_state.export.trim().to_string();
                                ui_state.msg = Some(match forge_vendor::save(std::path::Path::new(&f), vd) {
                                    Ok(()) => (true, format!("wrote {f}")),
                                    Err(e) => (false, e),
                                });
                            }
                        });
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut ui_state.import)
                            .hint_text("a vendor profile .toml")
                            .desired_width(260.0),
                    );
                    if ui.button("Import").clicked() {
                        let f = ui_state.import.trim().to_string();
                        match forge_vendor::import(std::path::Path::new(&f)) {
                            Ok(to) => {
                                ui_state.msg = Some((true, format!("saved as {}", to.display())));
                                todo = Some(Box::new(|u: &mut VendorUi| u.reload()));
                            }
                            Err(e) => ui_state.msg = Some((false, e)),
                        }
                    }
                    if ui.button("Reload").clicked() {
                        todo = Some(Box::new(|u: &mut VendorUi| u.reload()));
                    }
                });
                if let Some((ok, m)) = &ui_state.msg {
                    ui.label(egui::RichText::new(m).font(sans(11.0)).color(if *ok {
                        t.ink2
                    } else {
                        t.error
                    }));
                }
                if let Some(ed) = &mut ui_state.edit {
                    ui.separator();
                    let mut saved = false;
                    let mut drop_edit = false;
                    edit_ui(ui, &t, ed, &mut saved, &mut drop_edit);
                    if saved {
                        todo = Some(Box::new(|u: &mut VendorUi| u.reload()));
                    }
                    if drop_edit {
                        ui_state.edit = None;
                    }
                }
            });
        if let Some(f) = todo {
            f(&mut self.sheet.vendor);
        }
        if close {
            self.sheet.vendor.open = false;
        }
        self.places.extend(places);
        self.sheet.out.extend(out);
    }

    fn dfm_window(&mut self, ctx: &egui::Context, _fg: &FgPanel) {
        if !self.sheet.vendor.dfm_open {
            return;
        }
        let t = self.theme;
        let v = self.view_rect;
        let u = &mut self.sheet.vendor;
        let mut close = false;
        let mut places: super::placed::Places = vec![];
        egui::Window::new("Design for Manufacture")
            .id(egui::Id::unique("sheet-dfm"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.right() - 500.0, v.top() + 48.0))
            .min_width(460.0)
            .max_width(460.0)
            .frame(super::fgasm::dialog_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                close =
                    super::fgasm::dialog_head(ui, &t, "check", "Design for Manufacture", "sheet:dfm", places);
                ui.horizontal(|ui| {
                    let r = ui.button(egui::RichText::new("Check again").font(sans(12.0)));
                    places.push(("sheet:dfm:again".into(), r.rect));
                    if r.clicked() {
                        u.checked = None;
                    }
                    if u.running.is_some() {
                        ui.spinner();
                        ui.label(small(&t, "Checking…"));
                    }
                });
                let Some(r) = u.report.clone() else {
                    return;
                };
                ui.add_space(super::theme::S1);
                for it in &r.refused {
                    pill_row(
                        ui,
                        &t,
                        "Refused",
                        t.error,
                        &rule_words(&it.rule),
                        &format!("line {}", it.line),
                    );
                    ui.label(
                        egui::RichText::new(rounded_text(&it.text))
                            .font(sans(12.0))
                            .color(t.ink),
                    );
                    ui.add_space(super::theme::S2);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height((v.height() * 0.6).max(200.0))
                    .show(ui, |ui| {
                        for (si, s) in r.sheets.iter().enumerate() {
                            dfm_sheet(ui, &t, u, si, s, places);
                        }
                    });
            });
        self.places.extend(places);
        if close {
            u.dfm_open = false;
            u.chosen = None;
        }
    }
}

/// A severity in words and colour, as the Issues window has it.
fn severity(t: &super::theme::Theme, s: &str) -> (&'static str, Color32) {
    match s {
        "violation" => ("Violation", t.error),
        "refused" => ("Refused", t.error),
        "warning" => ("Warning", super::fg::WARN),
        _ => ("Info", t.ink2),
    }
}

/// An Issues-style head: the severity's pill, then what it is, then where in secondary ink.
fn pill_row(
    ui: &mut Ui,
    t: &super::theme::Theme,
    word: &str,
    colour: Color32,
    what: &str,
    at: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = super::theme::S2;
        egui::Frame::new()
            .fill(colour.gamma_multiply(0.16))
            .corner_radius(super::theme::R1)
            .inner_margin(egui::Margin::symmetric(6, 1))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(word).font(medium(11.0)).color(colour));
            });
        ui.label(egui::RichText::new(what).font(medium(12.0)).color(t.ink));
        ui.label(
            egui::RichText::new(at)
                .font(super::theme::caption())
                .color(t.ink2),
        );
    })
    .response
}

/// One sheet's part of the DFM panel: its vendor, its brake, then a row per finding, each chosen
/// by a click (lit on the model and the flat).
fn dfm_sheet(
    ui: &mut Ui,
    t: &super::theme::Theme,
    u: &mut VendorUi,
    si: usize,
    s: &forge_lang::dfm::SheetReport,
    places: &mut super::placed::Places,
) {
    let head = match &s.vendor {
        Some(vt) => format!(
            "{} · {} {}, {} mm",
            s.sheet,
            vt.name,
            vt.material,
            round2(s.thickness)
        ),
        None if s.checked => format!("{} · the part's own shop", s.sheet),
        None => format!("{} · no vendor or shop: choose a vendor to check", s.sheet),
    };
    ui.label(egui::RichText::new(head).font(super::theme::title()).color(t.ink));
    if let Some(vt) = &s.vendor {
        ui.label(small(
            t,
            format!("Data of {}", vt.taken.as_deref().unwrap_or("undated")),
        ));
        ui.label(
            egui::RichText::new(forge_vendor::VERIFY)
                .font(medium(11.0))
                .color(t.focus),
        );
    }
    if let Some(b) = &s.brake {
        ui.label(small(
            t,
            if b.feasible {
                format!("Press brake: bends in order {}", b.sequence.join(", "))
            } else {
                "Press brake: no bend order works with this tooling".to_string()
            },
        ));
    }
    let all = s.all();
    let count = all
        .iter()
        .filter(|i| i.severity == "violation" || i.severity == "refused")
        .count();
    let (summary, ink) = if count == 0 {
        ("No violations".to_string(), t.success)
    } else if count == 1 {
        ("1 to fix".to_string(), t.error)
    } else {
        (format!("{count} to fix"), t.error)
    };
    ui.label(egui::RichText::new(summary).font(medium(12.0)).color(ink));
    for (ii, it) in all.into_iter().enumerate() {
        ui.add_space(super::theme::S1);
        ui.separator();
        let on = u.chosen == Some((si, ii));
        let (word, colour) = severity(t, &it.severity);
        let row = egui::Frame::new()
            .fill(if on {
                t.accent.gamma_multiply(0.22)
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(super::theme::R1)
            .inner_margin(egui::Margin::symmetric(4, 3))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                pill_row(
                    ui,
                    t,
                    word,
                    colour,
                    &rule_words(&it.rule),
                    &format!("{} · line {} · {}", it.feature, it.line, it.rule),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(rounded_text(&it.text))
                            .font(sans(12.0))
                            .color(t.ink),
                    )
                    .wrap(),
                );
                if let (Some(m), Some(l)) = (it.measured, it.limit) {
                    let bound = match it.sense {
                        Some("min") => "at least ",
                        Some("max") => "at most ",
                        _ => "",
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "Measured {} {u} · limit {bound}{} {u}",
                            round2(m),
                            round2(l),
                            u = it.unit
                        ))
                        .font(super::theme::code())
                        .color(t.ink2),
                    );
                }
                if !it.fixes.is_empty() {
                    ui.label(small(t, "Fixes in the Issues window"));
                }
            })
            .response;
        let r = ui.interact(
            row.rect,
            ui.scope_id().with(("dfm-row", si, ii)),
            egui::Sense::click(),
        );
        places.push((format!("sheet:dfm:{si}:{ii}"), row.rect));
        if r.hovered() && !on {
            ui.painter().rect_stroke(
                row.rect,
                super::theme::R1,
                egui::Stroke::new(1.0, t.line),
                egui::StrokeKind::Inside,
            );
        }
        if r.on_hover_text("Light it on the model and the flat").clicked() {
            u.chosen = if on { None } else { Some((si, ii)) };
        }
    }
    ui.add_space(super::theme::S4);
}

/// The editor of a user's profile: its facts, its operations, a material's stocks as a table,
/// or the whole of it as text; Save writes it to its file, checked by reading it back.
fn edit_ui(ui: &mut Ui, t: &super::theme::Theme, ed: &mut Edit, saved: &mut bool, drop_edit: &mut bool) {
    ui.label(
        egui::RichText::new(format!("Editing {}", ed.file.display()))
            .font(semi(12.0))
            .color(t.ink),
    );
    let mut as_text = ed.text.is_some();
    if ui.checkbox(&mut as_text, "Edit as text").changed() {
        if as_text {
            let _ = ed.store_rows();
            ed.text = Some(ed.v.to_text());
        } else if let Some(text) = ed.text.take() {
            match Vendor::parse(&text) {
                Ok(v) => {
                    ed.v = v;
                    ed.load_rows();
                    ed.error = None;
                }
                Err(e) => {
                    ed.error = Some(e.to_string());
                    ed.text = Some(text);
                }
            }
        }
    }
    if let Some(text) = &mut ed.text {
        ui.add(
            egui::TextEdit::multiline(text)
                .font(mono(11.0))
                .desired_rows(16)
                .desired_width(f32::INFINITY),
        );
    } else {
        let field = |ui: &mut Ui, name: &str, s: &mut String| {
            ui.horizontal(|ui| {
                ui.add_sized(vec2(70.0, 20.0), egui::Label::new(small(t, name)));
                ui.add(egui::TextEdit::singleline(s).desired_width(300.0));
            });
        };
        field(ui, "Name", &mut ed.v.name);
        let mut src = ed.v.source.clone().unwrap_or_default();
        field(ui, "Source", &mut src);
        ed.v.source = (!src.trim().is_empty()).then_some(src);
        let mut taken = ed.v.taken.clone().unwrap_or_default();
        field(ui, "Data of", &mut taken);
        ed.v.taken = (!taken.trim().is_empty()).then_some(taken);
        let mut note = ed.v.note.clone().unwrap_or_default();
        field(ui, "Note", &mut note);
        ed.v.note = (!note.trim().is_empty()).then_some(note);
        ui.horizontal_wrapped(|ui| {
            ui.label(small(t, "Does"));
            let mut ops = ed.v.operations.clone().unwrap_or_default();
            for op in forge_sheet::Operation::ALL {
                let mut on = ops.iter().any(|o| o == op.name());
                if ui.checkbox(&mut on, op.name()).changed() {
                    if on {
                        ops.push(op.name().into());
                    } else {
                        ops.retain(|o| o != op.name());
                    }
                }
            }
            ed.v.operations = Some(ops);
        });
        let names: Vec<String> = ed.v.materials.iter().map(|m| m.name.clone()).collect();
        ui.horizontal(|ui| {
            let mut pick = ed.material;
            egui::ComboBox::from_id_salt("vendor-edit-material")
                .selected_text(names.get(pick).cloned().unwrap_or_else(|| "no material".into()))
                .show_ui(ui, |ui| {
                    for (k, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut pick, k, n);
                    }
                });
            if pick != ed.material {
                if let Err(e) = ed.store_rows() {
                    ed.error = Some(e);
                } else {
                    ed.material = pick;
                    ed.load_rows();
                }
            }
            if ui.button("Add material").clicked() {
                let _ = ed.store_rows();
                ed.v.materials.push(forge_vendor::MaterialRow {
                    name: format!("material {}", ed.v.materials.len() + 1),
                    ..Default::default()
                });
                ed.material = ed.v.materials.len() - 1;
                ed.load_rows();
            }
        });
        if let Some(m) = ed.v.materials.get_mut(ed.material) {
            ui.horizontal(|ui| {
                ui.add_sized(vec2(70.0, 20.0), egui::Label::new(small(t, "Material")));
                ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(200.0));
            });
        }
        ui.label(small(
            t,
            format!(
                "Lengths in {}; rules a number or like 2.5t + r",
                ed.v.units.word()
            ),
        ));
        egui::ScrollArea::horizontal()
            .id_salt("vendor-stock-scroll")
            .show(ui, |ui| {
                egui::Grid::new("vendor-stocks")
                    .spacing(vec2(4.0, 2.0))
                    .show(ui, |ui| {
                        for c in COLS {
                            ui.label(egui::RichText::new(c).font(medium(10.0)).color(t.ink2));
                        }
                        ui.end_row();
                        let mut remove = None;
                        for (k, row) in ed.rows.iter_mut().enumerate() {
                            for cell in row.iter_mut() {
                                ui.add(
                                    egui::TextEdit::singleline(cell)
                                        .font(mono(11.0))
                                        .desired_width(58.0),
                                );
                            }
                            if ui.small_button("x").clicked() {
                                remove = Some(k);
                            }
                            ui.end_row();
                        }
                        if let Some(k) = remove {
                            ed.rows.remove(k);
                            if let Some(m) = ed.v.materials.get_mut(ed.material)
                                && k < m.stock.len()
                            {
                                m.stock.remove(k);
                            }
                        }
                    });
            });
        if ui.button("Add stock").clicked() {
            ed.rows.push(Default::default());
        }
    }
    if let Some(e) = &ed.error {
        ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
    }
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            match ed.checked() {
                Ok(v) => match forge_vendor::save(&ed.file, &v) {
                    Ok(()) => {
                        ed.v = v;
                        ed.error = None;
                        *saved = true;
                    }
                    Err(e) => ed.error = Some(e),
                },
                Err(e) => ed.error = Some(e),
            }
        }
        if ui.button("Close editor").clicked() {
            *drop_edit = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sources_bare_paths_take_the_link_before() {
        let s = "https://www.fabworks.com/resources/guidelines/bending ; https://www.fabworks.com/resources/materials/sheet/aluminum ; /steel ; /stainless-steel";
        assert_eq!(
            sources(s),
            [
                "https://www.fabworks.com/resources/guidelines/bending",
                "https://www.fabworks.com/resources/materials/sheet/aluminum",
                "https://www.fabworks.com/resources/materials/sheet/steel",
                "https://www.fabworks.com/resources/materials/sheet/stainless-steel",
            ]
        );
        assert_eq!(
            link_label("https://sendcutsend.com/guidelines/"),
            "sendcutsend.com/guidelines"
        );
        assert_eq!(sources("a book, p. 3"), ["a book, p. 3"]);
    }

    #[test]
    fn numbers_read_as_a_shop_gives_them() {
        assert_eq!(
            rounded_text("the hole is 1.736872506509 from the bend, at least 2.5 (t0.063, 5052)"),
            "the hole is 1.74 from the bend, at least 2.5 (t0.063, 5052)"
        );
        assert_eq!(round2(3.0), "3");
        assert_eq!(round2(1.6002), "1.6");
        assert_eq!(rule_words("HoleNearBend"), "Hole near bend");
    }
}
