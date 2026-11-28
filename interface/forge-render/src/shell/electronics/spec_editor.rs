//! Manufacturer characteristics use the same property controls as the symbol inspector.
use super::{
    Ui,
    inspector::{choice, field},
    sch::characteristics::*,
};

fn new_reading() -> Reading {
    Reading {
        kind: "pin_voltage".into(),
        unit: "V".into(),
        ..Default::default()
    }
}
pub(super) type FetContext<'a> = (&'a str, &'a super::sch::Symbol);

pub(super) fn new_fet_reading(kind: &str, (part, symbol): FetContext<'_>) -> Reading {
    Reading {
        kind: kind.into(),
        unit: fet::property(kind)
            .map(|p| p.unit)
            .or_else(|| units(kind).map(|us| us[0]))
            .unwrap_or("")
            .into(),
        pins: super::part_properties::terminals(symbol, part, kind),
        ..Default::default()
    }
}

fn matches_search(name: &str, needle: &str) -> bool {
    let normalize = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    normalize(name).contains(&normalize(needle))
}

// A condition can also use ordinary quantities such as the fixture's forward current.
fn fet_picker(ui: &mut Ui, part: &str, conditions: bool) -> Option<&'static str> {
    let id = ui.scope_id().with("fet_filter");
    let mut query = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_default();
    ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search name or datasheet symbol"));
    let mut selected = None;
    let mut count = 0;
    egui::ScrollArea::vertical().max_height(300.).show(ui, |ui| {
        let mut group = |title: &str, entries: Vec<(&'static str, String)>| {
            let entries: Vec<_> = entries
                .into_iter()
                .filter(|(_, label)| matches_search(label, &query))
                .collect();
            if entries.is_empty() {
                return;
            }
            count += entries.len();
            let rows = |ui: &mut Ui| {
                for (kind, label) in entries {
                    if ui.selectable_label(false, label).clicked() {
                        selected = Some(kind);
                        ui.close();
                    }
                }
            };
            if query.trim().is_empty() {
                ui.collapsing(title, rows);
            } else {
                ui.weak(title);
                rows(ui);
            }
        };
        for g in fet::groups(part) {
            group(
                g.title,
                g.properties.iter().map(|p| (p.kind, p.label())).collect(),
            );
        }
        if conditions {
            group(
                "General characteristics",
                FAMILIES
                    .iter()
                    .flat_map(|(ks, _)| ks.iter().copied())
                    .chain(STATES.iter().map(|(k, _)| *k))
                    .filter(|k| fet::property(k).is_none())
                    .map(|k| (k, k.replace('_', " ")))
                    .collect(),
            );
        }
        if count == 0 {
            ui.weak("No matching characteristics");
        }
        if ui.button("Custom quantity…").clicked() {
            selected = Some("custom.quantity");
            ui.close();
        }
    });
    if selected.is_some() {
        query.clear();
    }
    ui.data_mut(|d| d.insert_temp(id, query));
    selected
}

pub(super) fn reading(
    ui: &mut Ui,
    r: &mut Reading,
    pins: &[String],
    allowed: Option<&[&str]>,
    fet: Option<FetContext<'_>>,
) {
    ui.horizontal(|ui| {
        ui.label("Quantity");
        egui::ComboBox::from_id_salt("quantity")
            .close_behavior(if fet.is_some() {
                egui::PopupCloseBehavior::CloseOnClickOutside
            } else {
                egui::PopupCloseBehavior::CloseOnClick
            })
            .selected_text(r.kind.replace('_', " "))
            .show_ui(ui, |ui| {
                if let Some(context) = fet {
                    if let Some(kind) = fet_picker(ui, context.0, allowed.is_none())
                        && kind != r.kind
                    {
                        *r = new_fet_reading(kind, context);
                    }
                    return;
                }
                for kind in FAMILIES
                    .iter()
                    .flat_map(|(ks, _)| ks.iter().copied())
                    .chain(STATES.iter().map(|(k, _)| *k))
                    .chain(["custom.quantity"])
                {
                    if kind == "capacitance_retained"
                        || allowed.is_some_and(|kinds| kind != "custom.quantity" && !kinds.contains(&kind))
                    {
                        continue;
                    }
                    if ui
                        .selectable_label(r.kind == kind, kind.replace('_', " "))
                        .clicked()
                        && r.kind != kind
                    {
                        *r = Reading {
                            kind: kind.into(),
                            unit: units(kind).map_or("", |us| us[0]).into(),
                            state: states(kind).and_then(|s| s.first()).copied().unwrap_or("").into(),
                            pins: std::mem::take(&mut r.pins),
                            ..Default::default()
                        };
                    }
                }
            });
    });
    let custom = r.kind.starts_with("custom.");
    if custom {
        let mut name = r.kind.trim_start_matches("custom.").to_string();
        field(ui, "Custom quantity", &mut name);
        r.kind = format!("custom.{name}");
    }
    if let Some(us) = units(&r.kind) {
        let mut selected = r.unit.clone();
        choice(
            ui,
            "Unit",
            &mut selected,
            us.iter()
                .filter(|u| **u != "dBm" || r.converted(u).is_some())
                .map(ToString::to_string)
                .collect(),
        );
        if selected != r.unit {
            super::part_properties::change_unit(r, selected);
        }
    } else if custom {
        field(ui, "Unit", &mut r.unit);
    }
    if let Some(vs) = states(&r.kind) {
        if vs.is_empty() {
            field(ui, "State", &mut r.state);
        } else {
            choice(
                ui,
                "State",
                &mut r.state,
                vs.iter()
                    .copied()
                    .filter(|v| match (r.kind.as_str(), fet) {
                        ("device_family", Some(("mosfet", _))) => v.ends_with("_mosfet"),
                        ("device_family", Some(("jfet", _))) => v.ends_with("_jfet"),
                        ("device_family", Some(_)) => {
                            v.ends_with("_fet") || v.ends_with("_mosfet") || v.ends_with("_jfet")
                        }
                        _ => true,
                    })
                    .map(ToString::to_string)
                    .collect(),
            );
        }
    } else {
        if custom {
            field(ui, "State", &mut r.state);
        }
        for (label, bound) in [
            ("Minimum", &mut r.min),
            ("Typical", &mut r.typ),
            ("Maximum", &mut r.max),
        ] {
            ui.horizontal(|ui| {
                let mut supplied = bound.is_some();
                if ui.checkbox(&mut supplied, label).changed() {
                    *bound = supplied.then_some(0.0);
                }
                if let Some(v) = bound {
                    ui.add(egui::DragValue::new(v).speed(0.01).max_decimals(14));
                }
            });
        }
    }
    terminals(ui, &mut r.pins, pins);
}
pub(super) fn terminals(ui: &mut Ui, selected: &mut Vec<String>, pins: &[String]) {
    if !pins.is_empty() {
        let mut remove = None;
        for (i, name) in selected.iter_mut().enumerate() {
            ui.push_id(("terminal", i), |ui| {
                ui.horizontal(|ui| {
                    choice(ui, &format!("Terminal {}", i + 1), name, pins.to_vec());
                    if ui.small_button("−").on_hover_text("Remove terminal").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            selected.remove(i);
        }
        if let Some(next) = pins.iter().find(|p| !selected.contains(p))
            && ui
                .small_button("+ Terminal")
                .on_hover_text("Add a terminal in datasheet order")
                .clicked()
        {
            selected.push(next.clone());
        }
    }
}
pub(super) fn edit(
    ui: &mut Ui,
    specs: &mut Vec<Specification>,
    pins: &[String],
    electrical: &str,
    fet: Option<FetContext<'_>>,
) {
    if electrical != "no_connect" {
        super::part_properties::pin_voltage(ui, specs);
    }
    ui.collapsing("Advanced properties", |ui| {
        details(ui, specs, pins, None, fet, |_| false)
    });
}
pub(super) fn details(
    ui: &mut Ui,
    specs: &mut Vec<Specification>,
    pins: &[String],
    allowed: Option<&[&str]>,
    fet: Option<FetContext<'_>>,
    skip: impl Fn(&Specification) -> bool,
) {
    let mut remove = None;
    let mut duplicate = None;
    if let Some(context) = fet {
        super::fet_curves::show(ui, specs, context, pins, true);
    }
    for (i, spec) in specs.iter_mut().enumerate() {
        if skip(spec) || fet.is_some() && !spec.curve.is_empty() {
            continue;
        }
        ui.push_id(i, |ui| {
            let heading = if fet.is_some() {
                let v = &spec.value;
                let name = fet::property(&v.kind)
                    .map(|p| p.notation)
                    .filter(|s| !s.is_empty());
                let mut caption = name
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.kind.replace('_', " "));
                if let Some((label, value)) = [("typ", v.typ), ("max", v.max), ("min", v.min)]
                    .into_iter()
                    .find_map(|(k, v)| v.map(|n| (k, n)))
                {
                    caption.push_str(&format!(" · {label} {value} {}", v.unit));
                }
                if let Some(c) = spec.conditions.first()
                    && let Some(n) = c.typ
                {
                    caption.push_str(&format!(" @ {n} {}", c.unit));
                }
                caption
            } else {
                spec.value.kind.replace('_', " ")
            };
            egui::CollapsingHeader::new(heading)
                .id_salt("spec")
                .show(ui, |ui| {
                    choice(
                        ui,
                        "Rating",
                        &mut spec.class,
                        CLASSES.iter().map(ToString::to_string).collect(),
                    );
                    reading(ui, &mut spec.value, pins, allowed, fet);
                    egui::CollapsingHeader::new(format!("Conditions ({})", spec.conditions.len()))
                        .id_salt("conditions")
                        .show(ui, |ui| {
                            let mut remove = None;
                            for (i, c) in spec.conditions.iter_mut().enumerate() {
                                ui.push_id(i, |ui| {
                                    reading(ui, c, pins, None, fet);
                                    if ui.small_button("Remove condition").clicked() {
                                        remove = Some(i);
                                    }
                                });
                            }
                            if let Some(i) = remove {
                                spec.conditions.remove(i);
                            }
                            if ui.small_button("+ Condition").clicked() {
                                spec.conditions.push(new_reading());
                            }
                        });
                    field(ui, "Datasheet reference", &mut spec.source);
                    field(ui, "Note", &mut spec.note);
                    if fet.is_some() && ui.small_button("Duplicate operating point").clicked() {
                        duplicate = Some(i);
                    }
                    if ui.small_button("Remove characteristic").clicked() {
                        remove = Some(i);
                    }
                })
                .header_response
                .on_hover_text(spec.describe());
        });
    }
    if let Some(i) = remove {
        specs.remove(i);
    } else if let Some(i) = duplicate {
        specs.insert(i + 1, specs[i].clone());
    }
    if allowed.is_some() || fet.is_some() {
        let add = ui.button("Add characteristic");
        egui::Popup::menu(&add)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                if let Some(context) = fet {
                    if let Some(kind) = fet_picker(ui, context.0, allowed.is_none()) {
                        specs.push(Specification {
                            value: new_fet_reading(kind, context),
                            class: "characteristic".into(),
                            ..Default::default()
                        });
                    }
                    return;
                }
                for &kind in allowed
                    .unwrap_or_default()
                    .iter()
                    .chain(std::iter::once(&"custom.quantity"))
                {
                    if ui.button(kind.replace('_', " ")).clicked() {
                        specs.push(Specification {
                            value: Reading {
                                kind: kind.into(),
                                unit: units(kind).map_or("", |us| us[0]).into(),
                                ..Default::default()
                            },
                            class: "characteristic".into(),
                            ..Default::default()
                        });
                        ui.close();
                    }
                }
            });
    } else if ui.button("+ Characteristic").clicked() {
        specs.push(Specification {
            value: new_reading(),
            class: "characteristic".into(),
            ..Default::default()
        });
    }
}

pub(super) fn summary(r: &Reading) -> String {
    let mut parts = Vec::new();
    if !r.pins.is_empty() {
        parts.push(r.pins.join(" → "));
    }
    if !r.state.is_empty() {
        parts.push(r.state.replace('_', " "));
    }
    for (key, value) in [("min", r.min), ("typ", r.typ), ("max", r.max)] {
        if let Some(v) = value {
            parts.push(format!("{key} {v} {}", r.unit));
        }
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{electronics::Editor, theme};
    use egui::{Event, PointerButton, Rect, epaint::Shape, pos2, vec2};

    #[test]
    fn native_picker_searches_notation_applies_roles_and_saves_the_operating_point() {
        let src = include_str!("../../../../../examples/electronics/datasheet-characteristics.fg");
        let mut editor = Editor::open(None, src.into()).unwrap();
        editor.subject = "Mosfet".into();
        editor.pin.clear();
        editor.inspect();
        let symbol = &editor.doc.symbols["Mosfet"];
        let ctx = egui::Context::default();
        ctx.set_fonts(theme::fonts());
        let mut clock = 0.;
        let mut added = vec![];
        let mut frame = |events| {
            clock += 0.25;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(700., 640.))),
                    time: Some(clock),
                    events,
                    ..Default::default()
                },
                |ui| {
                    details(ui, &mut added, &[], Some(&[]), Some(("mosfet", symbol)), |_| {
                        false
                    })
                },
            );
            out.textures_delta.clear();
            out.shapes
                .into_iter()
                .filter_map(|s| match s.shape {
                    Shape::Text(t) => Some((t.galley.text().to_owned(), t.visual_bounding_rect().center())),
                    _ => None,
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let click = |pos| {
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ]
        };
        let initial = frame(vec![]);
        frame(click(initial["Add characteristic"]));
        let menu = frame(vec![]);
        frame(click(menu["Search name or datasheet symbol"]));
        frame(vec![Event::Text("qGD".into())]);
        let results = frame(vec![]);
        let row = "gate drain charge (Qgd (Miller))";
        assert!(results.contains_key(row), "{results:?}");
        assert!(!results.contains_key("gate source charge (Qgs)"));
        frame(click(results[row]));
        let record = added.last_mut().expect("selected a characteristic");
        assert_eq!(record.value.pins, ["G", "D"]);
        assert_eq!(record.value.unit, "nC");
        record.value.typ = Some(6.7);
        let mut condition = new_fet_reading("drain_source_voltage", ("mosfet", symbol));
        condition.typ = Some(30.);
        record.conditions.push(condition);
        record.source = "SLPS488B §5.1".into();
        editor.form.symbol_specs.push(record.clone());
        editor.save_properties().unwrap();
        let reopened = Editor::open(None, editor.src.clone()).unwrap();
        assert_eq!(reopened.doc.symbols["Mosfet"].specs.last(), Some(&*record));
    }
}
