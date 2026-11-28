//! Measured traces live in the same symbol records used by ERC and library tools.
use super::{
    inspector::{choice, field},
    sch::characteristics::{CLASSES, CURVE_BASES, CURVE_SCALES, Reading, Specification, fet, units},
    spec_editor::{self, FetContext, new_fet_reading},
};
use egui::{DragValue, Ui};
use std::collections::BTreeSet;

// Each preset chooses axes only. The author supplies every value and fixed bias.
const PRESETS: &[(&str, &str, &str)] = &[
    ("Output characteristics", "drain_source_voltage", "drain_current"),
    ("Transfer characteristics", "gate_source_voltage", "drain_current"),
    ("Gate-charge curve", "gate_charge", "gate_source_voltage"),
    (
        "On-resistance vs. gate bias",
        "gate_source_voltage",
        "on_resistance",
    ),
    (
        "On-resistance vs. temperature",
        "junction_temperature",
        "on_resistance",
    ),
    (
        "Input capacitance vs. drain bias",
        "drain_source_voltage",
        "input_capacitance",
    ),
    (
        "Output capacitance vs. drain bias",
        "drain_source_voltage",
        "output_capacitance",
    ),
    (
        "Reverse transfer capacitance vs. drain bias",
        "drain_source_voltage",
        "reverse_transfer_capacitance",
    ),
    (
        "Output charge vs. drain bias",
        "drain_source_voltage",
        "output_charge",
    ),
    (
        "Output energy vs. drain bias",
        "drain_source_voltage",
        "output_capacitance_energy",
    ),
    (
        "Safe operating area",
        "drain_source_voltage",
        "safe_operating_drain_current",
    ),
    (
        "Transient thermal impedance",
        "pulse_duration",
        "transient_thermal_impedance_junction_case",
    ),
    (
        "Transconductance vs. drain current",
        "drain_current",
        "transconductance",
    ),
    ("Noise spectrum", "frequency", "voltage_noise_density"),
    ("Forward gain vs. frequency", "frequency", "s21_magnitude"),
    ("Switching energy vs. current", "drain_current", "turn_on_energy"),
    (
        "Reverse recovery vs. current slew",
        "commutation_current_slew_rate",
        "recovery_charge",
    ),
];

fn next_name(specs: &[Specification], title: &str) -> String {
    if specs.iter().all(|s| s.curve != title) {
        return title.into();
    }
    (2..)
        .map(|i| format!("{title} {i}"))
        .find(|name| specs.iter().all(|s| &s.curve != name))
        .unwrap()
}

fn create(specs: &mut Vec<Specification>, ctx: FetContext<'_>, title: &str, x: &str, y: &str) {
    specs.push(Specification {
        curve: next_name(specs, title),
        value: new_fet_reading(y, ctx),
        conditions: vec![new_fet_reading(x, ctx)],
        class: "characteristic".into(),
        curve_scale: "linear".into(),
        ..Default::default()
    });
}

fn optional_cell(ui: &mut Ui, value: &mut Option<f64>, editable: bool) {
    if !editable {
        ui.label(value.map_or_else(|| "—".into(), |v| v.to_string()));
        return;
    }
    if let Some(n) = value {
        let r = ui.add(DragValue::new(n).speed(0.01).max_decimals(14));
        r.context_menu(|ui| {
            if ui.button("Clear value").clicked() {
                *value = None;
                ui.close();
            }
        });
    } else if ui.small_button("—").on_hover_text("Enter a value").clicked() {
        *value = Some(0.);
    }
}

fn axis_label(r: &Reading) -> String {
    let name = fet::property(&r.kind)
        .map(|p| p.notation)
        .filter(|s| !s.is_empty());
    format!(
        "{} ({})",
        name.map(str::to_owned)
            .unwrap_or_else(|| r.kind.replace('_', " ")),
        r.unit
    )
}

fn plot(ui: &mut Ui, points: &[Specification], indices: &[usize]) {
    let mut data: Vec<_> = indices
        .iter()
        .filter_map(|&i| {
            let p = &points[i];
            Some((
                p.conditions.first()?.typ?,
                [p.value.min, p.value.typ, p.value.max],
            ))
        })
        .collect();
    data.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some((&(xmin, _), &(xmax, _))) = data.first().zip(data.last()) else {
        return;
    };
    if xmax <= xmin {
        return;
    }
    let (ymin, ymax) = data
        .iter()
        .flat_map(|(_, ys)| ys.iter().flatten())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &n| {
            (lo.min(n), hi.max(n))
        });
    if !ymin.is_finite() {
        return;
    }
    let first = &points[indices[0]];
    ui.weak(format!(
        "{} vs. {}",
        axis_label(&first.value),
        axis_label(&first.conditions[0])
    ));
    let log_x = matches!(first.curve_scale.as_str(), "log_x" | "log_log");
    let log_y = matches!(first.curve_scale.as_str(), "log_y" | "log_log");
    if log_x && xmin <= 0. || log_y && ymin <= 0. {
        ui.weak("Logarithmic axes need positive samples.");
        return;
    }
    let fraction = |v: f64, lo: f64, hi: f64, log: bool| -> f32 {
        if lo == hi {
            return 0.5;
        }
        let map = |x: f64| {
            if log {
                forge_math::dmath::ln(x) / std::f64::consts::LN_10
            } else {
                x
            }
        };
        ((map(v) - map(lo)) / (map(hi) - map(lo))) as f32
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 84.), egui::Sense::hover());
    let r = rect.shrink(5.);
    let colors = [
        ui.visuals().weak_text_color(),
        ui.visuals().hyperlink_color,
        ui.visuals().text_color(),
    ];
    ui.horizontal(|ui| {
        for (i, label) in ["Minimum", "Typical", "Maximum"].iter().enumerate() {
            if data.iter().any(|(_, ys)| ys[i].is_some()) {
                ui.colored_label(colors[i], *label);
            }
        }
    });
    for (col, color) in colors.into_iter().enumerate() {
        let mut last = None;
        for &(x, ys) in &data {
            let Some(y) = ys[col] else {
                last = None;
                continue;
            };
            let u = fraction(x, xmin, xmax, log_x);
            let v = fraction(y, ymin, ymax, log_y);
            let pos = egui::pos2(r.left() + r.width() * u, r.bottom() - r.height() * v);
            if let Some(prev) = last {
                ui.painter()
                    .line_segment([prev, pos], egui::Stroke::new(1., color));
            }
            ui.painter().circle_filled(pos, 2.5, color);
            last = Some(pos);
        }
    }
    ui.horizontal(|ui| {
        ui.weak(format!("X {xmin} – {xmax} {}", first.conditions[0].unit));
        ui.separator();
        ui.weak(format!("Y {ymin} – {ymax} {}", first.value.unit));
    });
}

fn axis_settings(
    ui: &mut Ui,
    specs: &mut [Specification],
    indices: &[usize],
    horizontal: bool,
    pins: &[String],
) {
    fn axis(s: &mut Specification, horizontal: bool) -> &mut Reading {
        if horizontal {
            &mut s.conditions[0]
        } else {
            &mut s.value
        }
    }
    let current = axis(&mut specs[indices[0]], horizontal).clone();
    ui.label(current.kind.replace('_', " "));
    let mut unit = current.unit.clone();
    if let Some(us) = units(&current.kind) {
        choice(
            ui,
            "Unit",
            &mut unit,
            us.iter()
                .filter(|u| {
                    indices
                        .iter()
                        .all(|&i| axis(&mut specs[i], horizontal).converted(u).is_some())
                })
                .map(ToString::to_string)
                .collect(),
        );
    }
    let mut terminals = current.pins.clone();
    spec_editor::terminals(ui, &mut terminals, pins);
    if current.unit == unit && current.pins == terminals {
        return;
    }
    for &i in indices {
        let r = axis(&mut specs[i], horizontal);
        if unit != current.unit
            && let Some(new) = r.converted(&unit)
        {
            *r = new;
        }
        r.pins = terminals.clone();
    }
}

fn trace(
    ui: &mut Ui,
    specs: &mut Vec<Specification>,
    name: &str,
    ctx: FetContext<'_>,
    pins: &[String],
    editable: bool,
) {
    let indices: Vec<_> = specs
        .iter()
        .enumerate()
        .filter(|(_, p)| p.curve == name)
        .map(|(i, _)| i)
        .collect();
    let Some(&first) = indices.first() else { return };
    let mut title = name.to_owned();
    if editable {
        field(ui, "Trace name", &mut title);
        if !title.trim().is_empty() && title != name && specs.iter().all(|s| s.curve != title) {
            for &i in &indices {
                specs[i].curve = title.clone();
            }
        }
        ui.collapsing("Axes and fixed conditions", |ui| {
            ui.push_id("x", |ui| axis_settings(ui, specs, &indices, true, pins));
            ui.separator();
            ui.push_id("y", |ui| axis_settings(ui, specs, &indices, false, pins));
            let mut class = specs[first].class.clone();
            choice(
                ui,
                "Rating",
                &mut class,
                CLASSES.iter().map(ToString::to_string).collect(),
            );
            let mut scale = specs[first].curve_scale.clone();
            let mut basis = specs[first].curve_basis.clone();
            choice(
                ui,
                "Axis scaling",
                &mut scale,
                CURVE_SCALES.iter().map(ToString::to_string).collect(),
            );
            choice(
                ui,
                "Data source type",
                &mut basis,
                CURVE_BASES.iter().map(ToString::to_string).collect(),
            );
            let old = specs[first].conditions[1..].to_vec();
            let mut fixed = old.clone();
            let mut remove = None;
            for (j, c) in fixed.iter_mut().enumerate() {
                ui.push_id(j, |ui| {
                    spec_editor::reading(ui, c, pins, None, Some(ctx));
                    if ui.small_button("Remove condition").clicked() {
                        remove = Some(j);
                    }
                });
            }
            if let Some(j) = remove {
                fixed.remove(j);
            }
            if ui.small_button("Add fixed condition").clicked() {
                fixed.push(new_fet_reading("junction_temperature", ctx));
            }
            for &i in &indices {
                specs[i].class = class.clone();
                specs[i].curve_scale = scale.clone();
                specs[i].curve_basis = basis.clone();
                if old != fixed {
                    specs[i].conditions.truncate(1);
                    specs[i].conditions.extend_from_slice(&fixed);
                }
            }
        });
    }
    for c in &specs[first].conditions[1..] {
        ui.weak(c.describe());
    }
    plot(ui, specs, &indices);
    let x = axis_label(&specs[first].conditions[0]);
    let y = specs[first].value.unit.clone();
    let mut removed = None;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        egui::Grid::new("samples").striped(true).show(ui, |ui| {
            for head in [
                x,
                format!("Min ({y})"),
                format!("Typ ({y})"),
                format!("Max ({y})"),
            ] {
                ui.label(head);
            }
            ui.end_row();
            for &i in &indices {
                let p = &mut specs[i];
                for cell in [
                    &mut p.conditions[0].typ,
                    &mut p.value.min,
                    &mut p.value.typ,
                    &mut p.value.max,
                ] {
                    optional_cell(ui, cell, editable);
                }
                if editable && ui.small_button("×").on_hover_text("Remove sample").clicked() {
                    removed = Some(i);
                }
                ui.end_row();
            }
        })
    });
    ui.collapsing("Datasheet references", |ui| {
        for &i in &indices {
            let p = &mut specs[i];
            ui.push_id(i, |ui| {
                ui.weak(p.conditions[0].describe());
                if editable {
                    field(ui, "Source", &mut p.source);
                    field(ui, "Note", &mut p.note);
                } else {
                    ui.label(&p.source);
                    if !p.note.is_empty() {
                        ui.weak(&p.note);
                    }
                }
            });
        }
    });
    if editable {
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Add sample").clicked() {
                let mut point = specs[*indices.last().unwrap()].clone();
                point.conditions[0].typ = None;
                point.value.min = None;
                point.value.typ = None;
                point.value.max = None;
                specs.push(point);
            }
            if ui.small_button("New trace at other conditions").clicked() {
                let next = next_name(specs, &specs[first].curve);
                for &i in &indices {
                    let mut point = specs[i].clone();
                    point.curve = next.clone();
                    point.value.min = None;
                    point.value.typ = None;
                    point.value.max = None;
                    specs.push(point);
                }
            }
        });
        if let Some(i) = removed {
            specs.remove(i);
        }
    }
}

pub(super) fn show(
    ui: &mut Ui,
    specs: &mut Vec<Specification>,
    ctx: FetContext<'_>,
    pins: &[String],
    editable: bool,
) {
    ui.collapsing("Datasheet curves", |ui| {
        let names: BTreeSet<_> = specs
            .iter()
            .filter(|s| !s.curve.is_empty())
            .map(|s| s.curve.clone())
            .collect();
        for name in names {
            let i = specs.iter().position(|p| p.curve == name).unwrap();
            egui::CollapsingHeader::new(&name)
                .id_salt(("trace", i))
                .show(ui, |ui| trace(ui, specs, &name, ctx, pins, editable));
        }
        if !editable {
            return;
        }
        let add = ui.button("Add curve");
        egui::Popup::menu(&add)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
                    let permitted =
                        |k: &str| fet::groups(ctx.0).any(|g| g.properties.iter().any(|p| p.kind == k));
                    for &(title, x, y) in PRESETS {
                        if permitted(x) && permitted(y) && ui.button(title).clicked() {
                            create(specs, ctx, title, x, y);
                            ui.close();
                        }
                    }
                    ui.collapsing("Choose other axes", |ui| {
                        let id = ui.scope_id().with("curve_axes");
                        let (mut x, mut y) = ui
                            .data_mut(|d| d.get_temp::<(String, String)>(id))
                            .unwrap_or_default();
                        let kinds: Vec<_> = fet::groups(ctx.0)
                            .flat_map(|g| g.properties)
                            .filter(|p| !p.unit.is_empty())
                            .map(|p| p.kind.to_owned())
                            .collect();
                        choice(ui, "Sweep", &mut x, kinds.clone());
                        choice(ui, "Measurement", &mut y, kinds);
                        if ui
                            .add_enabled(!x.is_empty() && !y.is_empty(), egui::Button::new("Create curve"))
                            .clicked()
                        {
                            create(
                                specs,
                                ctx,
                                &format!("{} vs. {}", y.replace('_', " "), x.replace('_', " ")),
                                &x,
                                &y,
                            );
                            ui.close();
                        }
                        ui.data_mut(|d| d.insert_temp(id, (x, y)));
                    });
                });
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{electronics::Editor, theme};
    use egui::{Event, PointerButton, Rect, epaint::Shape, pos2, vec2};

    #[test]
    fn fork_a_real_trace_reuses_coordinates_without_copying_measurements() {
        let src = include_str!("../../../../../examples/electronics/datasheet-characteristics.fg");
        let mut e = Editor::open(None, src.into()).unwrap();
        e.subject = "MatchedPair".into();
        e.pin.clear();
        e.inspect();
        let symbol = &e.doc.symbols["MatchedPair"];
        let name = "Noise at 2 mA";
        let ctx = egui::Context::default();
        ctx.set_fonts(theme::fonts());
        let mut clock = 0.;
        let mut frame = |events| {
            clock += 0.25;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(700., 850.))),
                    time: Some(clock),
                    events,
                    ..Default::default()
                },
                |ui| trace(ui, &mut e.form.symbol_specs, name, ("jfet", symbol), &[], true),
            );
            output.textures_delta.clear();
            output
                .shapes
                .into_iter()
                .filter_map(|s| match s.shape {
                    Shape::Text(t) => Some((t.galley.text().to_owned(), t.visual_bounding_rect().center())),
                    _ => None,
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let rows = frame(vec![]);
        let pos = rows["New trace at other conditions"];
        frame(vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }]);
        }
        let fork: Vec<_> = e
            .form
            .symbol_specs
            .iter_mut()
            .filter(|s| s.curve == "Noise at 2 mA 2")
            .collect();
        assert_eq!(fork.len(), 2);
        for (p, (frequency, measurement)) in fork.into_iter().zip([(10., 4.), (1000., 2.)]) {
            assert_eq!(p.conditions[0].typ, Some(frequency));
            assert_eq!(p.value.typ, None);
            p.curve = "Bench noise at 1 mA".into();
            p.curve_basis = "bench_measurement".into();
            p.conditions[1].typ = Some(1.);
            p.value.typ = Some(measurement);
            p.source = "Test fixture A".into();
        }
        e.save_properties().unwrap();
        let reopened = Editor::open(None, e.src.clone()).unwrap();
        let traces = &reopened.doc.symbols["MatchedPair"].specs;
        assert_eq!(traces.iter().filter(|p| p.curve == name).count(), 2);
        assert_eq!(
            traces
                .iter()
                .filter(|p| p.curve == "Bench noise at 1 mA" && p.source == "Test fixture A")
                .count(),
            2
        );
    }
}
