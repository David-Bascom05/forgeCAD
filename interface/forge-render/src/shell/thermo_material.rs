//! The Material Browser's Thermoform tab: a material's `thermo.*` values, each with its source
//! (cited, the user's, or a stand-in), its stretch law as an Ogden table, and Fit from test data:
//! a law fitted to a CSV of stress against stretch, plotted against the points and the law it
//! replaces, checked for stability, and taken as the user's.
use super::theme::{Theme, mono, sans, semi};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_material::{Kind, Material, Value};
use forge_thermo::fit::{self, Model, Test};

/// Whether a material has anything to show here.
pub fn applies(m: &Material) -> bool {
    m.extra.iter().any(|(k, _)| k.starts_with("thermo."))
}

/// The rows: their words, their key, how a value reads and is written (a factor on its numbers).
const ROWS: [(&str, &str, &str, f64); 11] = [
    ("Forming window", "window", "°C", 1.0),
    ("Normal temp", "normal", "°C", 1.0),
    ("Shrink", "shrink", "%", 100.0),
    ("Draft male", "draft_male", "°", 1.0),
    ("Draft female", "draft_female", "°", 1.0),
    ("Min wall", "min_wall", "× t", 1.0),
    ("Max areal draw ratio", "max_adr", "", 1.0),
    ("Max H:D", "max_hd", "", 1.0),
    ("Min radius", "min_radius", "× t + mm", 1.0),
    ("Min inside radius", "min_inside_radius", "× t + mm", 1.0),
    ("Max vent", "max_vent", "× t + mm", 1.0),
];

fn nums(m: &Material, key: &str) -> Vec<f64> {
    match m.extra(key) {
        Some(Value::Num(v)) => vec![*v],
        Some(Value::Nums(v)) => v.clone(),
        _ => vec![],
    }
}

fn text_of(v: &[f64], f: f64) -> String {
    v.iter()
        .map(|x| {
            let s = format!("{:.4}", x * f);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        })
        .collect::<Vec<_>>()
        .join(" – ")
}

fn set_nums(m: &mut Material, key: &str, v: Vec<f64>) {
    let val = if v.len() == 1 {
        Value::Num(v[0])
    } else {
        Value::Nums(v)
    };
    match m.extra.iter_mut().find(|x| x.0 == key) {
        Some(x) => x.1 = val,
        None => m.extra.push((key.to_string(), val)),
    }
}

/// A source as the tab shows it: in a few words, and orange where it is the user's or a stand-in.
fn source_line(m: &Material, key: &str, t: &Theme) -> (String, Color32) {
    let s = m.source_of(key);
    let words = s.split(',').next().unwrap_or(s).trim();
    let year = s
        .as_bytes()
        .windows(4)
        .find(|w| w.iter().all(u8::is_ascii_digit) && (w[0] == b'1' || w[0] == b'2'))
        .map(|w| format!(" {}", String::from_utf8_lossy(w)))
        .unwrap_or_default();
    let orange = Color32::from_rgb(0xe0, 0xa0, 0x30);
    match m.kind_of(key) {
        Kind::StandIn => (
            format!("stand-in ({})", words.chars().take(40).collect::<String>()),
            orange,
        ),
        Kind::User => (
            format!("user · {}", words.chars().take(46).collect::<String>()),
            orange,
        ),
        Kind::Cited => (
            format!("{}{year}", words.chars().take(46).collect::<String>()),
            t.ink2,
        ),
    }
}

/// The fit's state, kept in egui's memory while the tab shows.
#[derive(Clone, Debug, Default)]
struct FitState {
    open: bool,
    file: String,
    points: Vec<(f64, f64)>,
    biaxial: bool,
    model: usize,
    temperature: f64,
    rate: String,
    note: String,
    got: Option<fit::Fitted>,
    said: String,
    /// Taken: written into the material on the tab's next pass.
    take: bool,
}

const MODELS: [(&str, Model); 5] = [
    ("Neo-Hookean", Model::NeoHookean),
    ("Mooney-Rivlin", Model::MooneyRivlin),
    ("Ogden, 1 term", Model::Ogden(1)),
    ("Ogden, 2 terms", Model::Ogden(2)),
    ("Ogden, 3 terms", Model::Ogden(3)),
];

/// The tab; true when it changed the material.
pub fn tab(ui: &mut Ui, t: &Theme, m: &mut Material) -> bool {
    let mut changed = false;
    let id = egui::Id::unique(("thermo-fit", m.name.clone()));
    let mut st: FitState = ui.ctx().data_mut(|d| d.get_temp(id)).unwrap_or_default();
    // a fit taken in the window last frame
    if st.take
        && let Some(f) = st.got.clone()
    {
        let src = format!(
            "fitted from {} ({} points, {} test{}, RMS {:.1} %){}",
            st.file,
            st.points.len(),
            if st.biaxial { "equibiaxial" } else { "uniaxial" },
            if st.rate.is_empty() {
                String::new()
            } else {
                format!(", {}", st.rate)
            },
            f.rms * 100.0,
            if st.note.is_empty() {
                String::new()
            } else {
                format!("; {}", st.note)
            }
        );
        set_nums(m, "thermo.mu", f.law.terms.iter().map(|x| x.0).collect());
        set_nums(m, "thermo.alpha", f.law.terms.iter().map(|x| x.1).collect());
        set_nums(m, "thermo.law_temp", vec![st.temperature]);
        match m.extra.iter_mut().find(|x| x.0 == "thermo.law") {
            Some(x) => x.1 = Value::Str("ogden".into()),
            None => m.extra.push(("thermo.law".into(), Value::Str("ogden".into()))),
        }
        for k in ["thermo.law", "thermo.mu", "thermo.alpha", "thermo.law_temp"] {
            m.set_source(k, &src, Kind::User);
        }
        st.take = false;
        st.open = false;
        changed = true;
    }
    let w = ui.available_width();
    // the values: property, value, source
    let head = |ui: &mut Ui| {
        let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
        for (x, s) in [(0.0, "PROPERTY"), (190.0, "VALUE"), (360.0, "SOURCE")] {
            ui.painter().text(
                pos2(r.left() + x, r.center().y),
                Align2::LEFT_CENTER,
                s,
                sans(10.5),
                t.ink2,
            );
        }
    };
    head(ui);
    for (label, key, unit, f) in ROWS {
        let key = format!("thermo.{key}");
        let v = nums(m, &key);
        if v.is_empty() {
            continue;
        }
        let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        ui.painter().text(
            pos2(r.left(), r.center().y),
            Align2::LEFT_CENTER,
            label,
            sans(13.0),
            t.ink,
        );
        let field = Rect::from_min_size(pos2(r.left() + 190.0, r.top() + 3.0), vec2(160.0, 28.0));
        let ek = egui::Id::unique(("thermo-edit", m.name.clone(), key.clone()));
        let mut s: String = ui
            .ctx()
            .data_mut(|d| d.get_temp(ek))
            .unwrap_or_else(|| text_of(&v, f));
        let user = m.kind_of(&key) != Kind::Cited;
        let resp = ui.put(
            field,
            egui::TextEdit::singleline(&mut s)
                .font(mono(13.0))
                .margin(vec2(7.0, 5.0))
                .text_color(t.ink)
                .background_color(if user {
                    t.field.gamma_multiply(1.2)
                } else {
                    t.field
                }),
        );
        if user {
            ui.painter().rect_stroke(
                field,
                3.0,
                Stroke::new(1.0, Color32::from_rgb(0xe0, 0xa0, 0x30)),
                StrokeKind::Inside,
            );
        }
        ui.painter().text(
            pos2(field.right() - 8.0, field.center().y),
            Align2::RIGHT_CENTER,
            unit,
            sans(11.0),
            t.ink2,
        );
        if resp.changed() {
            ui.ctx().data_mut(|d| d.insert_temp(ek, s.clone()));
        }
        if resp.lost_focus() {
            let got: Vec<f64> = s
                .split(['–', '-', ',', '/', ' '])
                .filter_map(|x| x.trim().parse::<f64>().ok())
                .map(|x| x / f)
                .collect();
            if got.len() == v.len() && got != v {
                set_nums(m, &key, got);
                m.set_source(&key, "user", Kind::User);
                changed = true;
            }
            ui.ctx().data_mut(|d| d.remove::<String>(ek));
        }
        // the source in the room left, cut with an ellipsis; all of it on hover
        let (src, ink) = source_line(m, &key, t);
        let room = Rect::from_min_max(pos2(r.left() + 360.0, r.top()), r.right_bottom());
        let mut job = egui::text::LayoutJob::simple_singleline(src, sans(12.0), ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room.width());
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter()
            .galley(pos2(room.left(), room.center().y - g.size().y / 2.0), g, ink);
        ui.interact(room, ui.scope_id().with(("thermo-src", &key)), Sense::hover())
            .on_hover_text(m.source_of(&key));
    }
    // the stretch law
    ui.add_space(10.0);
    let mu = nums(m, "thermo.mu");
    let al = nums(m, "thermo.alpha");
    let rows = 4.0 + if mu.is_empty() { 0.0 } else { 3.0 };
    let (r, _) = ui.allocate_exact_size(vec2(w, 30.0 + rows * 21.0), Sense::hover());
    let p = ui.painter();
    p.rect_stroke(r, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    p.text(
        r.left_top() + vec2(12.0, 22.0),
        Align2::LEFT_CENTER,
        "Stretch law at forming temperature",
        semi(13.0),
        t.ink,
    );
    let fit_b = Rect::from_min_size(pos2(r.right() - 150.0, r.top() + 10.0), vec2(138.0, 28.0));
    let resp = ui.interact(fit_b, ui.scope_id().with("thermo-fit-open"), Sense::click());
    let p = ui.painter();
    p.rect_filled(fit_b, 3.0, if resp.hovered() { t.hover } else { t.on });
    p.rect_stroke(fit_b, 3.0, Stroke::new(1.0, t.focus), StrokeKind::Inside);
    p.text(
        fit_b.center(),
        Align2::CENTER_CENTER,
        "Fit from test data…",
        sans(12.5),
        t.ink,
    );
    if resp.clicked() {
        st.open = true;
        if st.temperature == 0.0 {
            st.temperature = nums(m, "thermo.normal").first().copied().unwrap_or(150.0);
        }
    }
    let x0 = r.left() + 12.0;
    let mut y = r.top() + 52.0;
    let law = match m.extra("thermo.law") {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    };
    p.text(
        pos2(x0, y),
        Align2::LEFT_CENTER,
        format!(
            "{} {}-term",
            if law == "ogden" { "Ogden" } else { "Neo-Hookean" },
            mu.len()
        ),
        mono(12.0),
        t.ink2,
    );
    for (k, _) in mu.iter().enumerate() {
        p.text(
            pos2(x0 + 150.0 + k as f32 * 170.0, y),
            Align2::LEFT_CENTER,
            format!("i = {}", k + 1),
            mono(12.0),
            t.ink2,
        );
    }
    for (name, v) in [("mu (MPa)", &mu), ("alpha", &al)] {
        y += 21.0;
        p.text(pos2(x0, y), Align2::LEFT_CENTER, name, mono(12.0), t.ink2);
        for (k, x) in v.iter().enumerate() {
            p.text(
                pos2(x0 + 150.0 + k as f32 * 170.0, y),
                Align2::LEFT_CENTER,
                format!("{x:.4}"),
                mono(12.0),
                t.ink,
            );
        }
    }
    y += 24.0;
    let temp = nums(m, "thermo.law_temp").first().copied();
    let (src, ink) = source_line(m, "thermo.law", t);
    p.text(
        pos2(x0, y),
        Align2::LEFT_CENTER,
        format!(
            "{src}{}",
            temp.map_or(String::new(), |c| format!(" · at {c:.0} °C"))
        ),
        sans(11.5),
        ink,
    );
    if st.open {
        fit_window(ui.ctx(), t, m, &mut st);
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, st));
    changed
}

/// The Fit stretch law window.
fn fit_window(ctx: &egui::Context, t: &Theme, m: &Material, st: &mut FitState) {
    let frame = egui::Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(6)
        .inner_margin(14);
    let mut open = true;
    egui::Window::new(format!("Fit stretch law · {}", m.name))
        .id(egui::Id::unique(("thermo-fit-window", m.name.clone())))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .frame(frame)
        .default_size(vec2(1000.0, 560.0))
        .constrain_to(super::dialog_area(ctx))
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(320.0);
                    let row = |ui: &mut Ui, label: &str, add: &mut dyn FnMut(&mut Ui)| {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                vec2(110.0, 28.0),
                                egui::Label::new(egui::RichText::new(label).font(sans(13.0)).color(t.ink2)),
                            );
                            add(ui);
                        });
                    };
                    row(ui, "Test data", &mut |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut st.file)
                                .font(mono(12.0))
                                .desired_width(150.0)
                                .hint_text("path to a CSV"),
                        );
                        if ui.button("Load").clicked() {
                            match std::fs::read_to_string(st.file.trim()) {
                                Ok(text) => {
                                    st.points = fit::read_csv(&text);
                                    st.said = format!("{} points read", st.points.len());
                                }
                                Err(e) => st.said = format!("not read: {e}"),
                            }
                        }
                    });
                    row(ui, "Test type", &mut |ui| {
                        egui::ComboBox::from_id_salt("thermo-fit-test")
                            .selected_text(if st.biaxial {
                                "Bubble inflation (equibiaxial)"
                            } else {
                                "Uniaxial tension"
                            })
                            .width(196.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut st.biaxial, true, "Bubble inflation (equibiaxial)");
                                ui.selectable_value(&mut st.biaxial, false, "Uniaxial tension");
                            });
                    });
                    row(ui, "Columns", &mut |ui| {
                        ui.label(
                            egui::RichText::new("stretch, true stress (MPa)")
                                .font(sans(12.5))
                                .color(t.ink),
                        );
                    });
                    row(ui, "Temperature", &mut |ui| {
                        ui.add(
                            egui::DragValue::new(&mut st.temperature)
                                .range(20.0..=320.0)
                                .suffix(" °C"),
                        );
                    });
                    row(ui, "Strain rate", &mut |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut st.rate)
                                .desired_width(196.0)
                                .hint_text("0.5 /s"),
                        );
                    });
                    row(ui, "Model", &mut |ui| {
                        egui::ComboBox::from_id_salt("thermo-fit-model")
                            .selected_text(MODELS[st.model].0)
                            .width(196.0)
                            .show_ui(ui, |ui| {
                                for (k, (name, _)) in MODELS.iter().enumerate() {
                                    ui.selectable_value(&mut st.model, k, *name);
                                }
                            });
                    });
                    row(ui, "Source note", &mut |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut st.note)
                                .desired_width(196.0)
                                .hint_text("lab, rig, date"),
                        );
                    });
                    ui.add_space(6.0);
                    let b = egui::Button::new(egui::RichText::new("Fit").font(sans(13.0)))
                        .min_size(vec2(306.0, 30.0))
                        .fill(t.on)
                        .stroke(Stroke::new(1.0, t.focus));
                    if ui.add(b).clicked() {
                        let test = if st.biaxial {
                            Test::Equibiaxial
                        } else {
                            Test::Uniaxial
                        };
                        let reach = st.points.iter().map(|p| p.0).fold(1.0, f64::max).max(6.0);
                        st.got = fit::fit(&st.points, test, MODELS[st.model].1, reach);
                        if st.got.is_none() {
                            st.said = "load at least two points first".into();
                        }
                    }
                    ui.label(egui::RichText::new(&st.said).font(sans(11.5)).color(t.ink2));
                });
                ui.add_space(12.0);
                ui.vertical(|ui| plot(ui, t, m, st));
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(820.0);
                if ui
                    .button(egui::RichText::new("Cancel").font(sans(13.0)))
                    .clicked()
                {
                    st.open = false;
                }
                let ok = egui::Button::new(
                    egui::RichText::new("Use this fit")
                        .font(semi(13.0))
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0x2f, 0x6f, 0x4f));
                if ui.add_enabled(st.got.is_some(), ok).clicked() {
                    st.take = true;
                }
            });
        });
    if !open {
        st.open = false;
    }
}

/// The points, the fit and the law in use, stress against stretch; and the fit's numbers.
fn plot(ui: &mut Ui, t: &Theme, m: &Material, st: &FitState) {
    let (r, _) = ui.allocate_exact_size(vec2(640.0, 360.0), Sense::hover());
    let p = ui.painter().clone();
    let area = Rect::from_min_max(r.min + vec2(56.0, 16.0), r.max - vec2(16.0, 40.0));
    let test = if st.biaxial {
        Test::Equibiaxial
    } else {
        Test::Uniaxial
    };
    let lmax = st.points.iter().map(|q| q.0).fold(3.0, f64::max).ceil();
    let current = forge_thermo::Law {
        terms: nums(m, "thermo.mu")
            .into_iter()
            .zip(nums(m, "thermo.alpha"))
            .collect(),
    };
    let curve = |law: &forge_thermo::Law| -> Vec<(f64, f64)> {
        (0..=80)
            .map(|k| {
                let l = 1.0 + (lmax - 1.0) * k as f64 / 80.0;
                (l, fit::stress(law, test, l))
            })
            .collect()
    };
    let cur = curve(&current);
    let got = st.got.as_ref().map(|f| curve(&f.law)).unwrap_or_default();
    let smax = st
        .points
        .iter()
        .map(|q| q.1)
        .chain(cur.iter().map(|q| q.1))
        .chain(got.iter().map(|q| q.1))
        .fold(0.1, f64::max)
        * 1.1;
    let at = |l: f64, s: f64| {
        pos2(
            area.left() + ((l - 1.0) / (lmax - 1.0)) as f32 * area.width(),
            area.bottom() - (s / smax).clamp(-0.05, 1.05) as f32 * area.height(),
        )
    };
    p.line_segment(
        [area.left_bottom(), area.right_bottom()],
        Stroke::new(1.0, t.ink2),
    );
    p.line_segment([area.left_bottom(), area.left_top()], Stroke::new(1.0, t.ink2));
    for k in 0..=4 {
        let l = 1.0 + (lmax - 1.0) * k as f64 / 4.0;
        p.text(
            at(l, 0.0) + vec2(0.0, 12.0),
            Align2::CENTER_CENTER,
            format!("{l:.1}"),
            mono(11.0),
            t.ink2,
        );
        let s = smax * k as f64 / 4.0;
        p.text(
            pos2(area.left() - 8.0, at(1.0, s).y),
            Align2::RIGHT_CENTER,
            format!("{s:.2}"),
            mono(11.0),
            t.ink2,
        );
    }
    p.text(
        pos2(area.center().x, r.bottom() - 6.0),
        Align2::CENTER_BOTTOM,
        format!("stretch (1.0 – {lmax:.1})"),
        sans(11.5),
        t.ink2,
    );
    let dashed: Vec<egui::Pos2> = cur.iter().map(|q| at(q.0, q.1)).collect();
    p.extend(egui::Shape::dashed_line(
        &dashed,
        Stroke::new(1.5, t.ink2),
        6.0,
        4.0,
    ));
    let gold = Color32::from_rgb(0xe0, 0xa8, 0x3a);
    if !got.is_empty() {
        p.add(egui::Shape::line(
            got.iter().map(|q| at(q.0, q.1)).collect(),
            Stroke::new(2.0, gold),
        ));
    }
    let blue = Color32::from_rgb(0x7f, 0xb2, 0xec);
    for q in &st.points {
        p.circle_filled(at(q.0, q.1), 3.0, blue);
    }
    // the key
    let k = Rect::from_min_size(area.min + vec2(16.0, 8.0), vec2(260.0, 64.0));
    p.rect_filled(k, 3.0, t.surface);
    p.rect_stroke(k, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    p.circle_filled(k.min + vec2(12.0, 14.0), 3.0, blue);
    p.text(
        k.min + vec2(24.0, 14.0),
        Align2::LEFT_CENTER,
        format!("Test points ({})", st.points.len()),
        sans(11.5),
        t.ink,
    );
    p.line_segment(
        [k.min + vec2(6.0, 33.0), k.min + vec2(18.0, 33.0)],
        Stroke::new(2.0, gold),
    );
    p.text(
        k.min + vec2(24.0, 33.0),
        Align2::LEFT_CENTER,
        match &st.got {
            Some(f) => format!("{} fit · RMS {:.1} %", MODELS[st.model].0, f.rms * 100.0),
            None => "the fit (none yet)".into(),
        },
        sans(11.5),
        t.ink,
    );
    p.line_segment(
        [k.min + vec2(6.0, 52.0), k.min + vec2(18.0, 52.0)],
        Stroke::new(1.5, t.ink2),
    );
    p.text(
        k.min + vec2(24.0, 52.0),
        Align2::LEFT_CENTER,
        "the law in use",
        sans(11.5),
        t.ink,
    );
    // the fit's numbers in tiles
    let (r2, _) = ui.allocate_exact_size(vec2(640.0, 70.0), Sense::hover());
    let tiles: [(&str, String, Color32); 3] = match &st.got {
        Some(f) => [
            ("RMS ERROR", format!("{:.1} %", f.rms * 100.0), t.ink),
            (
                "STABLE (DRUCKER)",
                format!(
                    "{} to stretch {:.1}",
                    if f.stable_to >= lmax { "yes," } else { "only" },
                    f.stable_to
                ),
                if f.stable_to >= lmax {
                    Color32::from_rgb(0x5c, 0xc0, 0x7a)
                } else {
                    t.error
                },
            ),
            ("SHEAR MODULUS G", format!("{:.3} MPa", f.law.shear()), t.ink),
        ],
        None => [
            ("RMS ERROR", "–".into(), t.ink2),
            ("STABLE (DRUCKER)", "–".into(), t.ink2),
            ("SHEAR MODULUS", "–".into(), t.ink2),
        ],
    };
    for (k, (name, v, c)) in tiles.iter().enumerate() {
        let b = Rect::from_min_size(r2.min + vec2(k as f32 * 212.0, 6.0), vec2(204.0, 60.0));
        p.rect_filled(b, 3.0, t.raised);
        p.rect_stroke(b, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        p.text(
            b.min + vec2(10.0, 14.0),
            Align2::LEFT_CENTER,
            *name,
            sans(10.5),
            t.ink2,
        );
        p.text(b.min + vec2(10.0, 40.0), Align2::LEFT_CENTER, v, semi(16.0), *c);
    }
}
