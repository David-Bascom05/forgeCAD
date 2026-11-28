//! Form for named strap attributes. Pin selectors use the current symbol definition.
use super::inspector::{choice, field};
use super::*;
mod matrix;
mod states;
mod units;

#[derive(Default, Clone, PartialEq)]
pub(super) struct Attribute {
    name: String,
    label: String,
    unit: Option<String>,
    units: Vec<sch::AttributeUnit>,
    conversions: sch::ConversionGraph,
    kind: String,
    source_type: Option<String>,
    bus: Option<[String; 2]>,
    range: Option<[String; 2]>,
    modes: Vec<String>,
    default: Option<String>,
    inputs: Vec<Input>,
    rows: Vec<(Vec<String>, String)>,
}

#[derive(Default, Clone, PartialEq)]
struct Input {
    pin: String,
    states: Vec<(String, sch::StrapCondition)>,
}

pub(super) fn load(symbol: &sch::Symbol) -> Vec<Attribute> {
    symbol
        .strap_tables
        .iter()
        .map(|(name, t)| Attribute {
            name: name.clone(),
            label: t.label.clone().unwrap_or_else(|| name.clone()),
            unit: t.unit.clone(),
            units: t.units.clone(),
            conversions: t.conversions.clone(),
            kind: t.kind.word().into(),
            source_type: t.source_type.clone(),
            bus: t.bus.clone(),
            range: t.range.clone(),
            modes: t.modes.clone(),
            default: t.default.clone(),
            inputs: t
                .column_order
                .iter()
                .map(|pin| Input {
                    pin: pin.clone(),
                    states: t.inputs[pin]
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                })
                .collect(),
            rows: t
                .row_order
                .iter()
                .map(|states| {
                    (
                        t.column_order.iter().map(|pin| states[pin].clone()).collect(),
                        t.rows[states].clone(),
                    )
                })
                .collect(),
        })
        .collect()
}

pub(super) fn source(attrs: &[Attribute], doc: &Document) -> Result<String> {
    let mut tables = Vec::new();
    let key = |s: &str| {
        if identifier(s) {
            Ok(s.to_owned())
        } else {
            Err(format!("Expected a name, got ‘{s}’."))
        }
    };
    for a in attrs {
        let mut inputs = Vec::new();
        for input in &a.inputs {
            let mut states = Vec::new();
            for (state, target) in &input.states {
                states.push(format!(
                    "{}: {}",
                    key(state)?,
                    match target {
                        sch::StrapCondition::Reference(pin) => key(pin)?,
                        sch::StrapCondition::Floating => quote("open"),
                        sch::StrapCondition::Voltage(v) => format!("[{}, {}]", v.min, v.max),
                    }
                ));
            }
            inputs.push(format!("{}: {{{}}}", key(&input.pin)?, states.join(", ")));
        }
        let mut rows = Vec::new();
        for (states, v) in &a.rows {
            let mut pattern = Vec::new();
            for (input, state) in a.inputs.iter().zip(states) {
                pattern.push(format!("{}: {}", key(&input.pin)?, key(state)?));
            }
            rows.push(format!("[{{{}}}, {}]", pattern.join(", "), quote(v)));
        }
        let default = a
            .default
            .as_ref()
            .map(|s| format!(", default: {}", quote(s)))
            .unwrap_or_default();
        let mut rules = String::new();
        if !a.label.is_empty() && a.name != a.label {
            rules.push_str(&format!(", label: {}", quote(a.label.trim())));
        }
        if let Some(unit) = &a.unit {
            rules.push_str(&format!(", unit: {}", quote(unit)));
        }
        if let Some([sda, scl]) = &a.bus {
            rules.push_str(&format!(", bus: [{}, {}]", key(sda)?, key(scl)?));
        }
        let inherited = a.source_type.as_ref().and_then(|id| doc.attribute_types.get(id));
        if let Some([lo, hi]) = &a.range
            && inherited.and_then(|t| t.range.as_ref()) != a.range.as_ref()
        {
            rules.push_str(&format!(", range: [{}, {}]", quote(lo), quote(hi)));
        }
        if !a.modes.is_empty() && !inherited.is_some_and(|t| t.modes == a.modes) {
            rules.push_str(&format!(
                ", modes: [{}]",
                a.modes.iter().map(|s| quote(s)).collect::<Vec<_>>().join(", ")
            ));
        }
        tables.push(format!(
            "{}: strap_table(kind: {}, inputs: {{{}}}, rows: [{}]{}{})",
            key(&a.name)?,
            a.source_type.clone().unwrap_or_else(|| quote(&a.kind)),
            inputs.join(", "),
            rows.join(", "),
            default,
            rules
        ));
    }
    Ok(format!("{{{}}}", tables.join(", ")))
}

fn types(doc: &Document) -> impl Iterator<Item = (&String, &sch::AttributeType, bool)> {
    sch::standard_attribute_types()
        .iter()
        .filter(move |(id, _)| !doc.attribute_types.contains_key(&format!("std.properties.{id}")))
        .map(|(k, t)| (k, t, false))
        .chain(doc.attribute_types.iter().map(|(k, t)| (k, t, true)))
}
pub(super) struct TableWindow {
    matrix: matrix::Matrix,
    type_editor: Option<units::UnitDraft>,
    previous_type_editor: Option<units::UnitDraft>,
    index: Option<usize>,
    draft: Attribute,
    format: String,
    error: String,
}

impl TableWindow {
    fn new(index: Option<usize>, draft: Attribute) -> Self {
        let sample = draft
            .default
            .as_deref()
            .or_else(|| draft.rows.first().map(|(_, s)| s.as_str()))
            .unwrap_or("");
        let units = if draft.units.is_empty() {
            sch::StrapKind::parse(&draft.kind)
                .map(sch::StrapKind::standard_units)
                .unwrap_or_default()
        } else {
            &draft.units
        };
        let inferred = if sample.is_empty() {
            None
        } else if units.first().is_some_and(|u| u.radix.is_some()) {
            units
                .iter()
                .filter(|u| {
                    u.radix.as_ref().is_some_and(|r| {
                        sample
                            .get(..r.prefix.len())
                            .is_some_and(|s| s.eq_ignore_ascii_case(&r.prefix))
                    })
                })
                .max_by_key(|u| u.radix.as_ref().unwrap().prefix.len())
        } else {
            let sample = sample.replace("uA", "µA").replace("μA", "µA");
            units
                .iter()
                .filter(|u| sample.ends_with(&u.name))
                .max_by_key(|u| u.name.len())
        };
        let format = draft
            .unit
            .clone()
            .or_else(|| inferred.or_else(|| units.first()).map(|u| u.name.clone()))
            .unwrap_or_default();
        let type_editor = draft
            .source_type
            .as_ref()
            .filter(|id| {
                !id.contains('.') && matches!(draft.kind.as_str(), "quantity" | "integer" | "lookup")
            })
            .map(|id| units::UnitDraft::existing(id, units, &draft.conversions, draft.kind == "lookup"));
        Self {
            previous_type_editor: None,
            type_editor,
            matrix: Default::default(),
            index,
            draft,
            format,
            error: String::new(),
        }
    }

    fn change_format(&mut self, kind: sch::StrapKind, next: String) {
        let a = &mut self.draft;
        let mut values: Vec<_> = a
            .rows
            .iter_mut()
            .map(|(_, v)| v)
            .chain(a.default.iter_mut())
            .chain(a.range.iter_mut().flatten())
            .filter(|s| !s.trim().is_empty())
            .collect();
        let Some(converted) = values
            .iter()
            .map(|s| {
                a.conversions.convert(
                    kind,
                    s,
                    if a.units.is_empty() {
                        kind.standard_units()
                    } else {
                        &a.units
                    },
                    Some(&self.format),
                    &next,
                )
            })
            .collect::<Option<Vec<_>>>()
        else {
            self.error = "Correct the value or unit before converting.".into();
            return;
        };
        for (target, new) in values.iter_mut().zip(converted) {
            **target = new;
        }
        self.error.clear();
        a.unit = Some(next.clone());
        self.format = next;
    }
}

fn selector(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    current: &mut String,
    items: impl Iterator<Item = String>,
) {
    egui::ComboBox::from_id_salt(id)
        .width(112.)
        .selected_text(if current.is_empty() {
            "Choose…"
        } else {
            current.as_str()
        })
        .show_ui(ui, |ui| {
            for item in items {
                ui.selectable_value(current, item.clone(), item);
            }
        });
}

fn normalized(value: &str, format: &str, kind: sch::StrapKind, units: &[sch::AttributeUnit]) -> String {
    kind.format_unit(value, units, Some(format), format)
        .unwrap_or_else(|| value.trim().to_owned())
}
fn output(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    v: &mut String,
    modes: &[String],
    format: &str,
    kind: sch::StrapKind,
    units: &[sch::AttributeUnit],
) {
    if modes.is_empty() {
        if ui
            .add(
                egui::TextEdit::singleline(v)
                    .frame(egui::Frame::NONE)
                    .desired_width(134.),
            )
            .lost_focus()
            && !v.trim().is_empty()
        {
            *v = normalized(v, format, kind, units);
        }
    } else {
        selector(ui, id, v, modes.iter().cloned());
    }
}

impl Attribute {
    fn add_column(&mut self, pins: &[String]) {
        if let Some(pin) = pins.iter().find(|p| !self.inputs.iter().any(|i| &i.pin == *p)) {
            let states = [("low", "VSS"), ("high", "VDD")]
                .into_iter()
                .filter_map(|(state, target)| {
                    pins.iter()
                        .find(|p| *p != pin && p.eq_ignore_ascii_case(target))
                        .map(|p| (state.to_owned(), sch::StrapCondition::Reference(p.clone())))
                })
                .chain(std::iter::once((
                    "floating".into(),
                    sch::StrapCondition::Floating,
                )))
                .collect();
            self.inputs.push(Input {
                pin: pin.clone(),
                states,
            });
            for (states, _) in &mut self.rows {
                states.push(String::new());
            }
        }
    }
    fn remove_column(&mut self, i: usize) {
        self.inputs.remove(i);
        for (states, _) in &mut self.rows {
            states.remove(i);
        }
        if self.inputs.is_empty() {
            self.rows.clear();
        }
    }
    fn pin_states(&mut self, ui: &mut Ui, pins: &[String]) {
        for input in &mut self.inputs {
            for (state, condition) in &mut input.states {
                if let sch::StrapCondition::Reference(target) = condition {
                    ui.push_id((&input.pin, &*state), |ui| {
                        choice(
                            ui,
                            &format!("{} · {}", input.pin, state),
                            target,
                            pins.iter().filter(|id| **id != input.pin).cloned().collect(),
                        );
                    });
                }
            }
        }
    }
}

pub(super) fn list(
    ui: &mut Ui,
    attrs: &[Attribute],
    _doc: &Document,
    window: &mut Option<TableWindow>,
    places: &mut Vec<(String, Rect)>,
) {
    for (i, a) in attrs.iter().enumerate() {
        let link = ui.link(if a.label.is_empty() { &a.name } else { &a.label });
        places.push((format!("electronics:attribute:{}", a.name), link.rect));
        if link.clicked() {
            *window = Some(TableWindow::new(Some(i), a.clone()));
        }
    }
    if ui.button("Add attribute").clicked() {
        *window = Some(TableWindow::new(
            None,
            Attribute {
                kind: "text".into(),
                units: sch::StrapKind::Text.standard_units().to_vec(),
                ..Default::default()
            },
        ));
    }
}

impl Editor {
    pub(super) fn attribute_window(&mut self, ctx: &egui::Context, t: &Theme) {
        let Some(mut win) = self.form.attribute_window.take() else {
            return;
        };
        let Some(symbol) = self.doc.symbols.get(&self.subject) else {
            return;
        };
        let pins: Vec<_> = symbol.pins.keys().cloned().collect();
        let mut save = false;
        let mut cancel = false;
        let mut remove = false;
        let modal = egui::Modal::new(egui::Id::unique("symbol-attribute-table"))
            .frame(crate::shell::fgasm::dialog_frame(t))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = vec2(8., 6.);
                let area = crate::shell::dialog_area(ctx);
                let wanted = (180. * (win.draft.inputs.len() + 1) as f32 + 150.).clamp(560., 1100.);
                ui.set_width(wanted.min((area.width() - 56.).max(220.)));
                ui.heading("Derived attribute");
                egui::ScrollArea::vertical()
                    .max_height((area.height() - 155.).max(120.))
                    .show(ui, |ui| {
                        win.header(ui, &self.doc, t);
                        let kind = sch::StrapKind::parse(&win.draft.kind).unwrap();
                        if win.type_editor.as_ref().is_some_and(|d| d.declaring) {
                            return;
                        }
                        let a = &mut win.draft;
                        ui.add_space(8.);
                        win.matrix.show(
                            ui,
                            a,
                            &pins,
                            if win.format == "Unitless" { "" } else { &win.format },
                            t,
                        );
                        ui.add_space(4.);
                        ui.horizontal(|ui| {
                            let mut enabled = a.default.is_some();
                            if ui.checkbox(&mut enabled, "Default value").changed() {
                                a.default = enabled.then(String::new);
                            }
                            if let Some(v) = &mut a.default {
                                output(ui, "default", v, &a.modes, &win.format, kind, &a.units);
                            }
                        });
                        ui.separator();
                        if a.inputs.iter().any(|p| {
                            p.states
                                .iter()
                                .any(|(_, s)| matches!(s, sch::StrapCondition::Reference(_)))
                        }) {
                            ui.collapsing("Logic reference pins", |ui| a.pin_states(ui, &pins));
                        }
                        if !matches!(
                            kind,
                            sch::StrapKind::Text
                                | sch::StrapKind::Quantity
                                | sch::StrapKind::Integer
                                | sch::StrapKind::Lookup
                        ) {
                            ui.separator();
                            ui.collapsing("Electrical rules", |ui| {
                                if kind == sch::StrapKind::Address7 {
                                    let mut specified = a.bus.is_some();
                                    if ui.checkbox(&mut specified, "Specify bus pins").changed() {
                                        a.bus = specified.then(|| [String::new(), String::new()]);
                                    }
                                    if let Some([sda, scl]) = &mut a.bus {
                                        choice(ui, "SDA", sda, pins.clone());
                                        choice(ui, "SCL", scl, pins.clone());
                                    } else {
                                        ui.label("Use the active SDA and SCL pins.");
                                    }
                                } else if kind == sch::StrapKind::Current {
                                    let mut limited = a.range.is_some();
                                    if ui.checkbox(&mut limited, "Enforce operating range").changed() {
                                        a.range = limited.then(|| ["0 A".into(), "1 A".into()]);
                                    }
                                    if let Some([lo, hi]) = &mut a.range {
                                        field(ui, "Minimum", lo);
                                        field(ui, "Maximum", hi);
                                    }
                                } else {
                                    let mut del = None;
                                    for (i, mode) in a.modes.iter_mut().enumerate() {
                                        ui.horizontal(|ui| {
                                            let old = mode.clone();
                                            if ui.text_edit_singleline(mode).changed() {
                                                for v in a
                                                    .rows
                                                    .iter_mut()
                                                    .map(|(_, v)| v)
                                                    .chain(a.default.iter_mut())
                                                {
                                                    if *v == old {
                                                        v.clone_from(mode);
                                                    }
                                                }
                                            }
                                            if ui.small_button("×").on_hover_text("Remove option").clicked()
                                            {
                                                del = Some(i);
                                            }
                                        });
                                    }
                                    if let Some(i) = del {
                                        a.modes.remove(i);
                                    }
                                    if ui.button("Add option").clicked() {
                                        a.modes.push(String::new());
                                    }
                                }
                            });
                        }
                    });
                if !win.error.is_empty() {
                    ui.colored_label(t.error, &win.error);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if win.index.is_some() {
                        remove = ui.button("Remove attribute").clicked();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        save = ui
                            .add(
                                egui::Button::new(egui::RichText::new("Save").color(egui::Color32::WHITE))
                                    .fill(t.accent),
                            )
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            });
        if cancel
            || (!modal.any_popup_open
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)))
        {
            return;
        }
        if save || remove {
            let mut ops = vec![];
            if !remove {
                if win.draft.label.trim().is_empty() {
                    win.error = "Enter an attribute name.".into();
                    self.form.attribute_window = Some(win);
                    return;
                }
                if win.index.is_none() {
                    win.draft.name = units::key(&win.draft.label);
                }
                match win.type_op(&self.doc, &self.subject) {
                    Ok(changes) => ops.extend(changes),
                    Err(error) => {
                        win.error = error;
                        self.form.attribute_window = Some(win);
                        return;
                    }
                }
            }
            let old = self.form.straps.clone();
            let old_visibility = self.form.visibility.clone();
            if let Some(i) = win.index
                && (remove || old[i].name != win.draft.name)
                && let Some(on) = self.form.visibility.remove(&format!("derived.{}", old[i].name))
                && !remove
            {
                self.form
                    .visibility
                    .insert(format!("derived.{}", win.draft.name), on);
            }

            match win.index {
                Some(i) if remove => {
                    self.form.straps.remove(i);
                }
                Some(i) => self.form.straps[i] = win.draft.clone(),
                None => self.form.straps.push(win.draft.clone()),
            }
            match self.save_properties_with(ops) {
                Ok(()) => return,
                Err(e) => {
                    self.form.straps = old;
                    self.form.visibility = old_visibility;
                    win.error = e;
                }
            }
        }

        self.form.attribute_window = Some(win);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_preserves_column_and_row_order_and_converts_values_together() {
        let src = include_str!("../../../../../examples/electronics/strap-address-matrix.fg");
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/electronics/strap-address-matrix.fg"
        );
        let mut editor = Editor::open(Some(path), src.into()).unwrap();
        editor.inspect();
        let mut win = TableWindow::new(Some(0), editor.form.straps[0].clone());
        assert_eq!(win.draft.inputs[0].pin, "A1");
        assert_eq!(win.draft.rows[0].1, "0x48");
        win.change_format(sch::StrapKind::Address7, "Binary".into());
        assert_eq!(win.draft.default.as_deref(), Some("0b1001000"));
        assert_eq!(win.draft.rows[3].1, "0b1001011");
        editor.form.straps[0] = win.draft;
        editor.form.visibility.insert("derived.address".into(), false);
        editor.save_properties().unwrap();
        let reopened = load(&editor.doc.symbols["Device"]);
        assert_eq!(
            reopened
                .iter()
                .find(|a| a.name == "startup_source")
                .unwrap()
                .source_type
                .as_deref(),
            Some("firmware.startup_source")
        );
        assert_eq!(reopened[0].inputs[0].pin, "A1");
        assert_eq!(reopened[0].rows[1].0, ["low", "high"]);
        assert_eq!(reopened[0].rows[1].1, "0b1001001");
        assert_eq!(editor.doc.symbols["Device"].visibility["derived.address"], false);
        editor.step(false);
        assert_eq!(editor.src, src);
    }
    #[test]
    fn attribute_editor_saves_pinless_and_strapped_values_together_and_undoes() {
        let text = "symbol_library\nIC=symbol(pins:{A:pin(number:\"1\",kind:\"input\",at:(0,0)),VSS:pin(number:\"2\",kind:\"power_in\",at:(4,0))},fields:{vendor:\"Example\"})";
        let mut e = Editor::open(None, text.into()).unwrap();
        e.inspect();
        e.form.straps.push(Attribute {
            name: "address".into(),
            kind: "i2c_address".into(),
            default: Some("0x48".into()),
            inputs: vec![Input {
                pin: "A".into(),
                states: vec![("low".into(), sch::StrapCondition::Reference("VSS".into()))],
            }],
            rows: vec![(vec!["low".into()], "0x49".into())],
            ..Default::default()
        });
        e.form.straps.push(Attribute {
            name: "mode".into(),
            kind: "text".into(),
            default: Some("standby".into()),
            ..Default::default()
        });
        e.save_properties().unwrap();
        let saved = e.src.clone();
        let reopened = Editor::open(None, saved).unwrap();
        assert_eq!(reopened.doc.symbols["IC"].fields["vendor"], "Example");
        assert_eq!(load(&reopened.doc.symbols["IC"]).len(), 2);
        assert_eq!(
            reopened.doc.symbols["IC"].strap_tables["mode"].default.as_deref(),
            Some("standby")
        );
        e.step(false);
        assert_eq!(e.src, text);
    }
}
