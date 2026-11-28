//! Attribute names, types and unit declarations belong to the document, including GUI-created types.
mod grid;
mod tables;
use super::*;
use sch::{AttributeType, AttributeUnit, StrapKind};

pub(super) fn key(label: &str) -> String {
    let mut id = label
        .trim()
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if id.is_empty() {
        id = "attribute".into();
    }
    if id.as_bytes()[0].is_ascii_digit() {
        id.insert_str(0, "attr_");
    }
    id
}

struct UnitRow {
    radix: Option<[String; 3]>,
    name: String,
    factor: String,
    min: String,
    max: String,
}
impl UnitRow {
    fn from_unit(u: &AttributeUnit) -> Self {
        Self {
            radix: u
                .radix
                .as_ref()
                .map(|r| [r.base.to_string(), r.prefix.clone(), r.digits.to_string()]),
            name: u.name.clone(),
            factor: u.factor.to_string(),
            min: u.min.map(|v| v.to_string()).unwrap_or_default(),
            max: u.max.map(|v| v.to_string()).unwrap_or_default(),
        }
    }
    fn blank(base: bool) -> Self {
        Self {
            radix: None,
            name: String::new(),
            factor: if base { "1".into() } else { String::new() },
            min: String::new(),
            max: String::new(),
        }
    }
    fn parse(&self) -> Result<AttributeUnit> {
        let number = |text: &str| {
            text.trim()
                .parse::<f64>()
                .map_err(|_| format!("Enter a number for {}.", self.name))
        };
        let bound = |text: &str| (!text.trim().is_empty()).then(|| number(text)).transpose();
        let radix = self
            .radix
            .as_ref()
            .map(|r| -> Result<sch::Radix> {
                Ok(sch::Radix {
                    base: r[0].parse().map_err(|_| "Enter an integer base.")?,
                    prefix: r[1].clone(),
                    digits: r[2].parse().map_err(|_| "Enter a whole number of digits.")?,
                })
            })
            .transpose()?;
        Ok(AttributeUnit {
            radix,
            name: self.name.trim().into(),
            factor: number(&self.factor)?,
            min: bound(&self.min)?,
            max: bound(&self.max)?,
        })
    }
}
pub(super) struct UnitDraft {
    lookup: bool,
    tables: Option<Vec<sch::ConversionTable>>,
    pub declaring: bool,
    new_type: bool,
    id: String,
    label: String,
    rows: Vec<UnitRow>,
}
impl UnitDraft {
    pub(super) fn existing(
        id: &str,
        units: &[AttributeUnit],
        graph: &sch::ConversionGraph,
        lookup: bool,
    ) -> Self {
        Self {
            declaring: false,
            lookup,
            tables: Some(graph.tables().to_vec()),
            new_type: false,
            id: id.into(),
            label: String::new(),
            rows: units.iter().map(UnitRow::from_unit).collect(),
        }
    }
    fn custom() -> Self {
        Self {
            declaring: true,
            lookup: false,
            tables: None,
            new_type: true,
            id: String::new(),
            label: String::new(),
            rows: vec![UnitRow::blank(true)],
        }
    }
    fn units(&self) -> Result<Vec<AttributeUnit>> {
        let units = self.rows.iter().map(UnitRow::parse).collect::<Result<Vec<_>>>()?;
        sch::validate_units(&units)?;
        Ok(units)
    }
    fn kind(&self) -> StrapKind {
        if self.lookup {
            return StrapKind::Lookup;
        }
        if self.rows.first().is_some_and(|r| r.radix.is_some()) {
            StrapKind::Integer
        } else {
            StrapKind::Quantity
        }
    }
    fn table(&mut self, ui: &mut Ui, t: &Theme, a: &Attribute, format: &str) {
        if self.declaring {
            let mut mode = self.kind();
            ui.horizontal_wrapped(|ui| {
                ui.label("Conversion");
                for (kind, label) in [
                    (StrapKind::Quantity, "Scaled quantity"),
                    (StrapKind::Integer, "Integer encoding"),
                    (StrapKind::Lookup, "Value pairs"),
                ] {
                    ui.selectable_value(&mut mode, kind, label);
                }
            });
            if mode != self.kind() {
                self.lookup = mode == StrapKind::Lookup;
                self.tables = None;
                self.rows = match mode {
                    StrapKind::Integer => StrapKind::Address7
                        .standard_units()
                        .iter()
                        .map(|u| {
                            let mut r = UnitRow::from_unit(u);
                            r.min = "0".into();
                            r.max.clear();
                            if let Some(c) = &mut r.radix {
                                c[2] = "0".into();
                            }
                            r
                        })
                        .collect(),
                    StrapKind::Lookup => ["Source", "Destination"]
                        .into_iter()
                        .map(|name| {
                            let mut r = UnitRow::blank(true);
                            r.name = name.into();
                            r
                        })
                        .collect(),
                    _ => vec![UnitRow::blank(true)],
                };
                if self.lookup {
                    self.tables = Some(vec![]);
                }
            }
        }
        let kind = self.kind();
        let before: Vec<_> = self.rows.iter().map(|u| u.name.clone()).collect();
        grid::show(ui, t, &mut self.rows, true, kind, a, format);
        let after: Vec<_> = self.rows.iter().map(|u| u.name.clone()).collect();
        if let Some(list) = &mut self.tables
            && before.len() == after.len()
        {
            for t in list {
                for s in [&mut t.from, &mut t.to] {
                    if let Some(i) = before.iter().position(|n| n == s) {
                        s.clone_from(&after[i]);
                    }
                }
            }
        }
        if let Some(list) = &mut self.tables {
            tables::edit(ui, list, &after, self.lookup);
        } else {
            let mut list = self
                .units()
                .map(|units| sch::ConversionGraph::defaults(&units))
                .unwrap_or_default();
            if tables::edit(ui, &mut list, &after, self.lookup) {
                self.tables = Some(list);
            }
        }
    }
}
impl TableWindow {
    fn select_type(&mut self, id: Option<String>, t: &AttributeType) {
        self.previous_type_editor = None;
        self.type_editor = id
            .as_ref()
            .filter(|k| {
                !k.contains('.')
                    && matches!(
                        t.kind,
                        StrapKind::Quantity | StrapKind::Integer | StrapKind::Lookup
                    )
            })
            .map(|k| UnitDraft::existing(k, &t.units, &t.conversions, t.kind == StrapKind::Lookup));
        self.draft.kind = t.kind.word().into();
        self.draft.source_type = id;
        self.draft.modes = t.modes.clone();
        self.draft.range = t.range.clone();
        self.draft.units = t.units.clone();
        self.draft.conversions = t.conversions.clone();
        if t.kind != StrapKind::Address7 {
            self.draft.bus = None;
        }
        self.format = t.units[0].name.clone();
        self.draft.unit = Some(self.format.clone());
        self.error.clear();
    }

    fn declare(&mut self, doc: &Document) -> Result<()> {
        let d = self.type_editor.as_mut().ok_or("Choose Custom… first.")?;
        if d.label.trim().is_empty() {
            return Err("Name the custom type before declaring it.".into());
        }
        let id = key(&d.label);
        if doc.attribute_types.contains_key(&id)
            || doc.symbols.contains_key(&id)
            || doc.role_names.contains_key(&id)
            || doc.libraries.contains_key(&id)
        {
            return Err(format!("The name {id} is already in use."));
        }
        let units = d.units()?;
        self.draft.conversions = tables::graph(d.kind(), &units, d.tables.as_deref())?;
        self.format = units[0].name.clone();
        self.draft.units = units;
        self.draft.unit = Some(self.format.clone());
        self.draft.source_type = Some(id.clone());
        self.draft.kind = d.kind().word().into();
        self.draft.range = None;
        self.draft.modes.clear();
        self.draft.bus = None;
        d.id = id;
        d.declaring = false;
        self.previous_type_editor = None;
        self.error.clear();
        Ok(())
    }
    pub(super) fn type_op(&mut self, doc: &Document, before: &str) -> Result<Vec<Op>> {
        let Some(d) = &self.type_editor else {
            return Ok(vec![]);
        };
        if d.declaring {
            return Err("Declare the custom type and its units first.".into());
        }
        let units = d.units()?;
        if !units.iter().any(|u| u.name == self.format) {
            return Err("Choose one of the declared units.".into());
        }
        self.draft.units = units.clone();
        let graph = tables::graph(d.kind(), &units, d.tables.as_deref())?;
        self.draft.conversions = graph.clone();
        let edges = tables::source(graph.tables());
        let rows = units
            .iter()
            .map(|u| {
                let mut fields = vec![format!("name: {}", quote(&u.name))];
                if let Some(r) = &u.radix {
                    fields.extend([
                        format!("radix: {}", r.base),
                        format!("prefix: {}", quote(&r.prefix)),
                        format!("digits: {}", r.digits),
                    ]);
                } else if !d.lookup {
                    fields.push(format!("factor: {}", u.factor));
                }
                for (key, value) in [("min", u.min), ("max", u.max)] {
                    if let Some(n) = value {
                        fields.push(format!("{key}: {n}"));
                    }
                }
                format!("{{{}}}", fields.join(", "))
            })
            .collect::<Vec<_>>()
            .join(", ");
        let list = format!("[{rows}]");
        if d.new_type {
            return Ok(vec![Op::AddFeature {
                before: Some(before.into()),
                text: format!(
                    "{} = attribute_type(label: {}, kind: {}, units: {list}, conversions: {edges})",
                    d.id,
                    quote(d.label.trim()),
                    quote(d.kind().word())
                ),
            }]);
        }
        let existing = doc.attribute_types.get(&d.id).ok_or("Missing local type")?;
        let mut edits = Vec::new();
        if existing.units != units {
            edits.push(arg(&d.id, "units", list));
        }
        if existing.conversions != graph {
            edits.push(arg(&d.id, "conversions", edges));
        }
        Ok(edits)
    }
    pub(super) fn header(&mut self, ui: &mut Ui, doc: &Document, t: &Theme) {
        let kind = StrapKind::parse(&self.draft.kind).unwrap();
        let label = self
            .type_editor
            .as_ref()
            .filter(|d| d.new_type)
            .map(|d| d.label.as_str())
            .or_else(|| {
                self.draft
                    .source_type
                    .as_ref()
                    .and_then(|id| doc.attribute_types.get(id))
                    .map(|t| t.label.as_str())
            })
            .or_else(|| {
                sch::standard_attribute_types()
                    .values()
                    .find(|t| t.kind == kind)
                    .map(|t| t.label.as_str())
            })
            .unwrap_or("Custom")
            .to_owned();
        let mut selected = None;
        let mut custom = false;
        let mut cancel_new = false;
        ui.add_space(5.);
        egui::Grid::new("attribute-identity")
            .num_columns(2)
            .spacing(vec2(18., 7.))
            .show(ui, |ui| {
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.draft.label)
                        .desired_width(250.)
                        .hint_text("Attribute name"),
                );
                ui.end_row();
                ui.label("Type");
                if let Some(d) = self.type_editor.as_mut().filter(|d| d.declaring) {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut d.label)
                                .desired_width(185.)
                                .hint_text("New type name"),
                        );
                        cancel_new = ui.small_button("Cancel").clicked();
                    });
                } else {
                    egui::ComboBox::from_id_salt("attribute-type")
                        .width(195.)
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for (id, t, lib) in types(doc) {
                                if ui
                                    .selectable_label(
                                        self.draft.kind == t.kind.word()
                                            && self.draft.source_type.as_ref() == lib.then_some(id),
                                        if lib {
                                            format!("{} · {id}", t.label)
                                        } else {
                                            t.label.clone()
                                        },
                                    )
                                    .clicked()
                                {
                                    selected = Some((lib.then(|| id.clone()), t));
                                }
                            }
                            ui.separator();
                            custom = ui.selectable_label(false, "Custom…").clicked();
                        });
                }
                ui.end_row();
                ui.label("Unit");
                if let Some(d) = &self.type_editor
                    && d.declaring
                {
                    ui.weak("Declare units below");
                } else {
                    let mut next = self.format.clone();
                    let current = self.type_editor.as_ref().and_then(|d| d.units().ok());
                    let units = current.as_deref().unwrap_or(&self.draft.units);
                    selector(
                        ui,
                        "attribute-unit",
                        &mut next,
                        units.iter().map(|u| u.name.clone()),
                    );
                    if next != self.format {
                        if let (Some(units), Some(d)) = (current, &self.type_editor) {
                            match tables::graph(d.kind(), &units, d.tables.as_deref()) {
                                Ok(graph) => {
                                    self.draft.conversions = graph;
                                    self.draft.units = units;
                                    self.change_format(kind, next);
                                }
                                Err(e) => self.error = e,
                            }
                        } else {
                            self.change_format(kind, next);
                        }
                    }
                }
                ui.end_row();
            });
        if let Some((id, t)) = selected {
            self.select_type(id, t);
        }
        if custom {
            self.previous_type_editor = self.type_editor.take();
            let mut draft = UnitDraft::custom();
            if matches!(
                kind,
                StrapKind::Quantity | StrapKind::Integer | StrapKind::Address7 | StrapKind::Lookup
            ) {
                draft.rows = self.draft.units.iter().map(UnitRow::from_unit).collect();
                draft.lookup = kind == StrapKind::Lookup;
                draft.tables = Some(self.draft.conversions.tables().to_vec());
            }
            self.type_editor = Some(draft);
            self.error.clear();
        }
        if cancel_new {
            self.type_editor = self.previous_type_editor.take();
            self.error.clear();
        }
        let declaring = self.type_editor.as_ref().is_some_and(|d| d.declaring);
        let mut declare = false;
        egui::CollapsingHeader::new("Units")
            .id_salt(("attribute-units", declaring))
            .open(declaring.then_some(true))
            .show(ui, |ui| {
                if let Some(d) = &mut self.type_editor {
                    d.table(ui, t, &self.draft, &self.format);
                    if d.declaring {
                        declare = ui.button("Declare type").clicked();
                    }
                } else {
                    let mut rows = self.draft.units.iter().map(UnitRow::from_unit).collect();
                    grid::show(ui, t, &mut rows, false, kind, &self.draft, &self.format);
                    tables::inspect(ui, self.draft.conversions.tables());
                }
            });
        if declare && let Err(e) = self.declare(doc) {
            self.error = e;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declare_before_use_convert_save_reopen_and_undo_as_one_edit() {
        let source = "symbol_library\nChip=symbol(pins:{})";
        let mut editor = Editor::open(None, source.into()).unwrap();
        editor.inspect();
        let draft = Attribute {
            name: "f".into(),
            label: "Switching frequency".into(),
            kind: "text".into(),
            ..Default::default()
        };
        let mut win = TableWindow::new(None, draft);
        win.type_editor = Some(UnitDraft::custom());
        assert!(win.type_op(&editor.doc, "Chip").is_err());
        let d = win.type_editor.as_mut().unwrap();
        d.label = "Frequency".into();
        assert!(d.units().is_err());
        d.rows[0] = UnitRow {
            radix: None,
            name: "Hz".into(),
            factor: "1".into(),
            min: "0".into(),
            max: "2000000".into(),
        };
        d.rows.push(UnitRow {
            radix: None,
            name: "kHz".into(),
            factor: "1000".into(),
            min: "0".into(),
            max: "2000".into(),
        });
        win.declare(&editor.doc).unwrap();
        win.draft.default = Some("125000 Hz".into());
        win.change_format(StrapKind::Quantity, "kHz".into());
        assert_eq!(win.draft.default.as_deref(), Some("125 kHz"));
        let op = win.type_op(&editor.doc, "Chip").unwrap();
        editor.form.straps.push(win.draft);
        editor.save_properties_with(op).unwrap();
        assert!(editor.src.find("frequency = attribute_type").unwrap() < editor.src.find("Chip=").unwrap());
        let reopened = Editor::open(None, editor.src.clone()).unwrap();
        let a = load(&reopened.doc.symbols["Chip"]).remove(0);
        assert_eq!(a.label, "Switching frequency");
        assert_eq!(a.unit.as_deref(), Some("kHz"));
        assert_eq!(a.units[1].factor, 1000.);
        assert_eq!(a.units[1].max, Some(2000.));
        editor.step(false);
        assert_eq!(editor.src, source);
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    #[test]
    fn a_locally_declared_integer_keeps_its_codec_on_reopen() {
        let mut e = Editor::open(None, "symbol_library\nChip=symbol(pins:{})".into()).unwrap();
        e.inspect();
        let mut w = TableWindow::new(
            None,
            Attribute {
                name: "code".into(),
                label: "Register code".into(),
                kind: "text".into(),
                ..Default::default()
            },
        );
        let mut local = UnitDraft::custom();
        local.label = "Flags".into();
        local.rows = StrapKind::Address7
            .standard_units()
            .iter()
            .map(UnitRow::from_unit)
            .collect();
        w.type_editor = Some(local);
        w.declare(&e.doc).unwrap();
        w.draft.default = Some("0x48".into());
        w.change_format(StrapKind::Integer, "Binary".into());
        assert_eq!(w.draft.default.as_deref(), Some("0b1001000"));
        let op = w.type_op(&e.doc, "Chip").unwrap();
        e.form.straps.push(w.draft);
        e.save_properties_with(op).unwrap();
        let doc = Editor::open(None, e.src).unwrap().doc;
        let t = &doc.symbols["Chip"].strap_tables["code"];
        assert_eq!(t.kind, StrapKind::Integer);
        assert_eq!(t.units[0].radix.as_ref().unwrap().base, 16);
        assert_eq!(t.unit.as_deref(), Some("Binary"));
    }
}

#[cfg(test)]
mod conversion_table_tests {
    use super::*;
    #[test]
    fn saved_attribute_uses_two_library_tables_after_reopen() {
        let src = include_str!("../../../../../../examples/electronics/conversion-chain.fg");
        let mut editor = Editor::open(None, src.into()).unwrap();
        editor.inspect();
        let mut win = TableWindow::new(Some(0), editor.form.straps[0].clone());
        assert!(win.type_editor.as_ref().unwrap().lookup);
        win.change_format(StrapKind::Lookup, "Binary".into());
        assert_eq!(win.draft.default.as_deref(), Some("01001000"));
        assert_eq!(win.draft.rows[2].1, "01001010");
        let d = win.type_editor.as_mut().unwrap();
        d.tables.as_mut().unwrap()[1].name = "Binary codes".into();
        let changes = win.type_op(&editor.doc, "Device").unwrap();
        editor.form.straps[0] = win.draft;
        editor.save_properties_with(changes).unwrap();
        let reopened = Editor::open(None, editor.src.clone()).unwrap();
        let a = load(&reopened.doc.symbols["Device"]).remove(0);
        assert_eq!(a.conversions.tables()[1].name, "Binary codes");
        let mut again = TableWindow::new(Some(0), a);
        again.change_format(StrapKind::Lookup, "Hex".into());
        assert_eq!(again.draft.default.as_deref(), Some("0x48"));
        editor.step(false);
        assert_eq!(editor.src, src);
    }
}
