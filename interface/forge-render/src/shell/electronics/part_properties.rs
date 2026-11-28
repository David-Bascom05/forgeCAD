//! Ordinary properties and type-specific detail share the symbol's datasheet records.
use super::{inspector::property_row, sch, spec_editor};
use egui::Ui;
use sch::characteristics::{Reading, Specification, fet, units};

fn voltage_kind(part: &str) -> &'static str {
    match part {
        "resistor" | "capacitor" | "inductor" | "relay" | "switch" | "fuse" | "connector" | "transformer"
        | "crystal" => "working_voltage",
        "fet" | "mosfet" | "jfet" => "drain_source_voltage",
        "bjt" => "collector_emitter_voltage",
        "diode" => "forward_voltage",
        _ => "supply_voltage",
    }
}
fn normal_quantity(part: &str) -> Option<(&'static str, &'static str)> {
    match part {
        "resistor" => Some(("Resistance", "resistance")),
        "capacitor" => Some(("Capacitance", "capacitance")),
        "fet" | "mosfet" | "jfet" => Some(("On-resistance", "on_resistance")),
        _ => None,
    }
}
fn quantities(part: &str) -> Vec<&'static str> {
    let mut kinds = vec![voltage_kind(part)];
    if let Some((_, kind)) = normal_quantity(part) {
        kinds.push(kind);
    }
    kinds.extend_from_slice(match part {
        "resistor" => &["tolerance", "power_dissipation"][..],
        "capacitor" => &["tolerance", "series_resistance", "leakage_current"],
        "bjt" => &["collector_current", "dc_current_gain", "power_dissipation"],
        _ => &[],
    });
    kinds
}
pub(super) fn terminals(symbol: &sch::Symbol, part: &str, kind: &str) -> Vec<String> {
    if kind == "working_voltage" && matches!(part, "resistor" | "capacitor") && symbol.pins.len() == 2 {
        return symbol.pins.keys().cloned().collect();
    }
    if kind == "supply_voltage" {
        let mut supplies = symbol
            .pins
            .iter()
            .filter(|(_, p)| p.electrical == sch::Electrical::Supply);
        if let Some((id, _)) = supplies.next()
            && supplies.next().is_none()
        {
            return vec![id.clone()];
        }
    }
    let pair = if fet::is_fet(part) {
        let Some(pair) = fet::property(kind).and_then(|p| p.terminals) else {
            return vec![];
        };
        pair
    } else {
        match kind {
            "drain_source_voltage" | "on_resistance" => ["drain", "source"],
            "collector_emitter_voltage" => ["collector", "emitter"],
            "forward_voltage" => ["anode", "cathode"],
            _ => return vec![],
        }
    };
    let mut pins = Vec::new();
    for role in pair {
        let mut matches = symbol.pins.iter().filter(|(_, p)| {
            p.specs
                .iter()
                .any(|s| s.value.kind == "terminal_role" && s.value.state == role && s.conditions.is_empty())
        });
        let Some((name, _)) = matches.next() else {
            return vec![];
        };
        if matches.next().is_some() || pins.contains(name) {
            return vec![];
        }
        pins.push(name.clone());
    }
    pins
}

#[derive(Clone, Copy)]
enum Bound {
    Typical,
    Maximum,
}
impl Bound {
    fn value(self, v: &mut Reading) -> &mut Option<f64> {
        match self {
            Self::Typical => &mut v.typ,
            Self::Maximum => &mut v.max,
        }
    }
}
struct Property<'a> {
    label: &'a str,
    kind: &'a str,
    class: &'a str,
    bound: Bound,
}

pub(super) fn change_unit(r: &mut Reading, unit: String) {
    if let Some(converted) = r.converted(&unit) {
        *r = converted;
    }
}

impl Property<'_> {
    fn row(&self, ui: &mut Ui, specs: &mut Vec<Specification>, pins: &[String], editable: bool) {
        let mut candidates = specs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.value.kind == self.kind && s.class == self.class && s.conditions.is_empty());
        let index = candidates.next().map(|(i, _)| i);
        if candidates.next().is_some() {
            property_row(ui, self.label, |ui, _| {
                ui.label("Multiple ratings · see Advanced properties")
            });
            return;
        }
        if index.is_none() {
            let conditioned = specs
                .iter()
                .filter(|s| s.value.kind == self.kind && s.class == self.class && !s.conditions.is_empty())
                .count();
            if conditioned > 0 {
                property_row(ui, self.label, |ui, caption| {
                    ui.label(format!(
                        "{conditioned} measured operating point{}",
                        if conditioned == 1 { "" } else { "s" }
                    ))
                    .labelled_by(caption.id)
                    .on_hover_ui(|ui| {
                        for s in specs
                            .iter()
                            .filter(|s| s.value.kind == self.kind && s.class == self.class)
                        {
                            ui.label(s.describe());
                        }
                    })
                });
                return;
            }
        }
        let unit = units(self.kind).expect("numeric property")[0];
        ui.push_id((self.kind, self.class, self.label), |ui| {
            property_row(ui, self.label, |ui, caption| {
                let number = index.and_then(|i| *self.bound.value(&mut specs[i].value));
                if !editable {
                    let value = number
                        .zip(index)
                        .map_or_else(|| "—".to_owned(), |(n, i)| format!("{n} {}", specs[i].value.unit));
                    ui.label(value).labelled_by(caption.id);
                } else if let (Some(mut n), Some(i)) = (number, index) {
                    let r = &mut specs[i].value;
                    if ui
                        .add(egui::DragValue::new(&mut n).max_decimals(12).speed(0.05))
                        .labelled_by(caption.id)
                        .changed()
                    {
                        *self.bound.value(r) = Some(n);
                    }
                    let mut selected = r.unit.clone();
                    egui::ComboBox::from_id_salt("unit")
                        .width(44.)
                        .selected_text(&selected)
                        .show_ui(ui, |ui| {
                            for &u in units(self.kind).unwrap() {
                                ui.selectable_value(&mut selected, u.to_owned(), u);
                            }
                        });
                    if selected != r.unit {
                        change_unit(r, selected);
                    }
                    if ui.small_button("×").on_hover_text("Clear this value").clicked() {
                        *self.bound.value(r) = None;
                        if r.min.is_none() && r.typ.is_none() && r.max.is_none() {
                            specs.remove(i);
                        }
                    }
                } else if ui
                    .small_button(format!("Set…  {unit}"))
                    .labelled_by(caption.id)
                    .clicked()
                {
                    let i = index.unwrap_or_else(|| {
                        specs.push(Specification {
                            class: self.class.into(),
                            value: Reading {
                                kind: self.kind.into(),
                                unit: unit.into(),
                                pins: pins.to_vec(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        specs.len() - 1
                    });
                    *self.bound.value(&mut specs[i].value) = Some(0.);
                }
            });
        });
    }
}

fn curve_point(s: &Specification) -> bool {
    s.class == "characteristic"
        && s.value.kind == "capacitance_retained"
        && s.value.pins.is_empty()
        && s.value.min.is_none()
        && s.value.max.is_none()
        && s.value.typ.is_some()
        && matches!(&s.conditions[..],[bias] if bias.kind=="working_voltage" && bias.pins.is_empty() && bias.typ.is_some() && bias.min.is_none() && bias.max.is_none())
}
fn bias_points(specs: &[Specification]) -> Vec<(f64, f64)> {
    let mut points: Vec<_> = specs
        .iter()
        .filter(|s| curve_point(s))
        .map(|s| {
            let bias = &s.conditions[0];
            (bias.typ.unwrap() * bias.unit_scale(), s.value.typ.unwrap())
        })
        .collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    points
}
fn plot_curve(ui: &mut Ui, points: &[(f64, f64)]) {
    if points.len() < 2 {
        return;
    }
    let span = points.last().unwrap().0;
    if span <= 0. {
        return;
    }
    let (area, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 78.), egui::Sense::hover());
    let rect = area.shrink2(egui::vec2(4., 5.));
    let painter = ui.painter();
    let grid = ui.visuals().widgets.noninteractive.bg_stroke;
    let color = ui.visuals().hyperlink_color;
    for k in [0., 0.5, 1.] {
        let y = rect.bottom() - rect.height() * k;
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], grid);
    }
    let mut previous = None;
    for &(v, pct) in points {
        let pos = egui::pos2(
            rect.left() + rect.width() * (v / span) as f32,
            rect.bottom() - rect.height() * (pct / 100.) as f32,
        );
        if let Some(last) = previous {
            painter.line_segment([last, pos], egui::Stroke::new(1.5, color));
        }
        painter.circle_filled(pos, 2.5, color);
        previous = Some(pos);
    }
    ui.weak(format!("0–{span} V DC bias · 0–100% retained"));
}
fn curve(ui: &mut Ui, specs: &mut Vec<Specification>, editable: bool) {
    ui.strong("Capacitance vs. DC bias");
    let points = bias_points(specs);
    plot_curve(ui, &points);
    let mut remove = None;
    egui::Grid::new("dc-bias-points").striped(true).show(ui, |ui| {
        ui.label("DC bias (V)");
        ui.label("Capacitance retained (%)");
        ui.end_row();
        for (i, s) in specs.iter_mut().enumerate().filter(|(_, s)| curve_point(s)) {
            let cond = &mut s.conditions[0];
            let scale = cond.unit_scale();
            let mut volts = cond.typ.unwrap() * scale;
            if editable {
                if ui
                    .add(egui::DragValue::new(&mut volts).range(0.0..=1e12).speed(0.1))
                    .changed()
                    && !points.iter().any(|&(v, _)| v == volts)
                {
                    cond.typ = Some(volts / scale);
                }
                ui.add(
                    egui::DragValue::new(s.value.typ.as_mut().unwrap())
                        .range(0.0..=100.)
                        .speed(0.1),
                );
                if ui.small_button("×").on_hover_text("Remove point").clicked() {
                    remove = Some(i);
                }
            } else {
                ui.label(volts.to_string());
                ui.label(s.value.typ.unwrap().to_string());
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        specs.remove(i);
    }
    if editable {
        let id = ui.scope_id().with("new_dc_point");
        let (mut volts, mut percent) = ui
            .data_mut(|d| d.get_temp::<(String, String)>(id))
            .unwrap_or_default();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut volts)
                    .hint_text("V")
                    .desired_width(48.),
            );
            ui.add(
                egui::TextEdit::singleline(&mut percent)
                    .hint_text("%")
                    .desired_width(48.),
            );
            let point = volts
                .trim()
                .parse::<f64>()
                .ok()
                .zip(percent.trim().parse::<f64>().ok())
                .filter(|(v, p)| {
                    (0.0..=1e12).contains(v)
                        && !points.iter().any(|(x, _)| x == v)
                        && (0.0..=100.).contains(p)
                });
            if ui
                .add_enabled(point.is_some(), egui::Button::new("Add point"))
                .clicked()
            {
                let (v, p) = point.unwrap();
                specs.push(Specification {
                    class: "characteristic".into(),
                    value: Reading {
                        kind: "capacitance_retained".into(),
                        unit: "%".into(),
                        typ: Some(p),
                        ..Default::default()
                    },
                    conditions: vec![Reading {
                        kind: "working_voltage".into(),
                        unit: "V".into(),
                        typ: Some(v),
                        ..Default::default()
                    }],
                    ..Default::default()
                });
                volts.clear();
                percent.clear();
            }
        });
        ui.data_mut(|d| d.insert_temp(id, (volts, percent)));
    }
}

pub(super) fn pin_voltage(ui: &mut Ui, specs: &mut Vec<Specification>) {
    voltage_rows(ui, "pin_voltage", specs, &[], true);
}
fn voltage_rows(ui: &mut Ui, voltage: &str, specs: &mut Vec<Specification>, pins: &[String], editable: bool) {
    for (label, class, bound) in [
        ("Nominal voltage", "operating", Bound::Typical),
        ("Max operating voltage", "operating", Bound::Maximum),
        ("Absolute max voltage", "absolute_max", Bound::Maximum),
    ] {
        Property {
            label,
            kind: voltage,
            class,
            bound,
        }
        .row(ui, specs, pins, editable);
    }
}
pub(super) fn normal(
    ui: &mut Ui,
    symbol: &sch::Symbol,
    part: &str,
    specs: &mut Vec<Specification>,
    editable: bool,
) {
    let voltage = voltage_kind(part);
    let pins = terminals(symbol, part, voltage);
    voltage_rows(ui, voltage, specs, &pins, editable);
    if let Some((label, kind)) = normal_quantity(part) {
        Property {
            label,
            kind,
            class: "characteristic",
            bound: Bound::Typical,
        }
        .row(ui, specs, &terminals(symbol, part, kind), editable);
    }
}
pub(super) fn advanced(
    ui: &mut Ui,
    part: &str,
    specs: &mut Vec<Specification>,
    symbol: &sch::Symbol,
    editable: bool,
) {
    egui::CollapsingHeader::new("Advanced properties")
        .id_salt("advanced_properties")
        .show(ui, |ui| {
            if part == "capacitor" {
                curve(ui, specs, editable);
            }
            if editable {
                spec_editor::details(
                    ui,
                    specs,
                    &symbol.pins.keys().cloned().collect::<Vec<_>>(),
                    Some(&quantities(part)),
                    fet::is_fet(part).then_some((part, symbol)),
                    |s| part == "capacitor" && curve_point(s),
                );
            } else {
                if fet::is_fet(part) {
                    super::fet_curves::show(
                        ui,
                        specs,
                        (part, symbol),
                        &symbol.pins.keys().cloned().collect::<Vec<_>>(),
                        false,
                    );
                }
                for s in specs.iter().filter(|s| {
                    (part != "capacitor" || !curve_point(s)) && (!fet::is_fet(part) || s.curve.is_empty())
                }) {
                    ui.label(s.describe());
                    if !s.source.is_empty() {
                        ui.weak(&s.source);
                    }
                }
            }
        });
}
pub(super) fn requirements(ui: &mut Ui, part: &str, specs: &mut Vec<Specification>, symbol: &sch::Symbol) {
    ui.collapsing("Datasheet requirements", |ui| {
        spec_editor::details(
            ui,
            specs,
            &symbol.pins.keys().cloned().collect::<Vec<_>>(),
            Some(&quantities(part)),
            fet::is_fet(part).then_some((part, symbol)),
            |_| false,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::theme;
    use egui::{Event, PointerButton, Rect, epaint::Shape, pos2, vec2};

    #[test]
    fn native_property_rows_and_expanded_panel_follow_the_selected_type() {
        let symbol = sch::read("symbol_library\nP=symbol(pins:{})", None, None)
            .unwrap()
            .symbols
            .remove("P")
            .unwrap();
        for (kind, label) in [
            ("resistor", "Resistance"),
            ("capacitor", "Capacitance"),
            ("mosfet", "On-resistance"),
        ] {
            let ctx = egui::Context::default();
            ctx.set_fonts(theme::fonts());
            let mut specs = vec![];
            let mut time = 0.;
            let mut frame = |events| {
                time += 0.25;
                let mut frame = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(550., 620.))),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        normal(ui, &symbol, kind, &mut specs, true);
                        advanced(ui, kind, &mut specs, &symbol, true);
                    },
                );
                frame.textures_delta.clear();
                frame
                    .shapes
                    .into_iter()
                    .filter_map(|s| match s.shape {
                        Shape::Text(t) => {
                            Some((t.galley.text().to_owned(), t.visual_bounding_rect().center()))
                        }
                        _ => None,
                    })
                    .collect::<std::collections::BTreeMap<_, _>>()
            };
            let before = frame(vec![]);
            for name in [
                "Nominal voltage",
                "Max operating voltage",
                "Absolute max voltage",
                label,
            ] {
                assert!(before.contains_key(name), "{kind}, {name}, {before:?}");
            }
            assert!(!before.contains_key("Capacitance vs. DC bias"));
            let pos = before["Advanced properties"];
            frame(vec![Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                frame(vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                }]);
            }
            let after = frame(vec![]);
            assert!(after.contains_key("Add characteristic"));
            assert_eq!(after.contains_key("Add point"), kind == "capacitor");
            assert!(specs.is_empty(), "opening the panel must not create ratings");
        }
    }

    #[test]
    fn editing_a_local_nominal_updates_shared_symbol_without_freezing_placements() {
        use super::super::Editor;
        let src = "schematic\nR=symbol(part_kind:\"resistor\",pins:{},specs:[{kind:\"resistance\",unit:\"kΩ\",typ:10}])\nmain=sheet()\nr=place(sheet:main,symbol:R,at:(0,0))\ns=place(sheet:main,symbol:R,at:(30,0))";
        let mut e = Editor::open(None, src.into()).unwrap();
        e.subject = "main".into();
        e.selected = "r".into();
        e.inspect();
        e.form.inherited_specs[0].value.typ = Some(22.);
        e.save_properties().unwrap();
        let reopened = Editor::open(None, e.src.clone()).unwrap();
        for r in &reopened.doc.rows {
            if let sch::Item::Place { value, .. } = &r.item {
                assert_eq!(value, "22 kΩ");
            }
        }
        assert!(!e.src.contains("value: \"10 kΩ\""));
        e.step(false);
        assert_eq!(e.src, src);
    }
}
