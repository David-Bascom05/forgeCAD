use super::*;
use crate::shell::ux;
use egui::{ComboBox, RichText};
fn field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
}
fn choice(ui: &mut Ui, label: &str, value: &mut String, items: Vec<String>) {
    ui.label(label);
    ComboBox::from_id_salt(label)
        .selected_text(if value.is_empty() {
            "Choose…"
        } else {
            value.as_str()
        })
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for s in items {
                ui.selectable_value(value, s.clone(), s);
            }
        });
}
fn custom(ui: &mut Ui, rows: &mut Vec<(String, String)>) {
    let mut remove = None;
    for (i, (k, v)) in rows.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            ui.horizontal(|ui| {
                let width = ui.available_width();
                ui.add(
                    egui::TextEdit::singleline(k)
                        .hint_text("Field")
                        .desired_width(width * 0.33),
                );
                ui.add(
                    egui::TextEdit::singleline(v)
                        .hint_text("Value")
                        .desired_width(width * 0.42),
                );
                if ui.small_button("×").on_hover_text("Remove field").clicked() {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        rows.remove(i);
    }
    if ui.small_button("+ Add field").clicked() {
        rows.push(Default::default());
    }
}
impl Editor {
    pub(super) fn inspect(&mut self) {
        self.form = Inputs {
            unit: self.unit.clone(),
            electrical: "passive".into(),
            populated: true,
            in_bom: true,
            ..Default::default()
        };
        if self.library() {
            if let Some(sym) = self.doc.symbols.get(&self.subject) {
                self.form.name = self.subject.clone();
                self.form.fields = sym.fields.clone().into_iter().collect();
                self.form.datasheet = sym.datasheet.clone().unwrap_or_default();
                if let Some(g) = self.graphic.and_then(|i| sym.graphics.get(i)) {
                    use sch::Shape::*;
                    self.form.unit = g.unit.clone().unwrap_or_default();
                    self.form.geometry = match &g.shape {
                        Line(a, b) | Rect(a, b) => vec![*a, *b],
                        Arc(a, m, b) => vec![*a, *m, *b],
                        Polyline(ps) => ps.clone(),
                        Circle(a, r) => {
                            self.form.radius = *r;
                            vec![*a]
                        }
                        Text(a, s) => {
                            self.form.text = s.clone();
                            vec![*a]
                        }
                    };
                }
                if let Some(p) = sym.pins.get(&self.pin) {
                    self.form.name = self.pin.clone();
                    self.form.number = p.number.clone();
                    self.form.electrical = p.electrical.word().into();
                    self.form.unit = p.unit.clone();
                    self.form.angle = p.angle as i32;
                    let names = |ids: &std::collections::BTreeSet<sch::RoleId>| {
                        ids.iter()
                            .filter_map(|id| {
                                self.doc
                                    .role_names
                                    .iter()
                                    .find(|(_, r)| *r == id)
                                    .map(|(alias, _)| alias.clone())
                            })
                            .collect()
                    };
                    self.form.wants = names(&p.wants);
                    self.form.gives = names(&p.gives);
                }
            }
        } else if let Some(row) = self.doc.rows.iter().find(|r| r.id == self.selected) {
            self.form.name = row.id.clone();
            match &row.item {
                Item::Place {
                    value,
                    footprint,
                    datasheet,
                    fields,
                    populated,
                    in_bom,
                    ref_offset,
                    value_offset,
                    ..
                } => {
                    self.form.ref_offset = *ref_offset;
                    self.form.value_offset = *value_offset;
                    self.form.value = value.clone();
                    self.form.footprint = footprint.clone().unwrap_or_default();
                    self.form.datasheet = datasheet.clone().unwrap_or_default();
                    self.form.fields = fields.clone().into_iter().collect();
                    self.form.populated = *populated;
                    self.form.in_bom = *in_bom;
                }
                Item::Note(_, s) => self.form.text = s.clone(),
                Item::Label { text, global, .. } => {
                    self.form.text = text.clone();
                    self.form.global = *global;
                }
                _ => {}
            }
        }
    }
    pub(super) fn save_pin(&mut self, at: Point) -> Result<()> {
        let f = &self.form;
        if !identifier(&f.name) {
            return Err("Enter a valid pin name.".into());
        }
        if self.tool == Tool::Pin
            && self
                .doc
                .symbols
                .get(&self.subject)
                .is_some_and(|s| s.pins.contains_key(&f.name))
        {
            return Err("This pin name already exists.".into());
        }
        let body = format!(
            "pin(number: {}, kind: {}, at: {}, unit: {}, angle: {}deg, wants: [{}], gives: [{}])",
            quote(&f.number),
            quote(&f.electrical),
            coord(at),
            quote(&f.unit),
            f.angle,
            f.wants.join(", "),
            f.gives.join(", ")
        );
        let k = f.name.clone();
        self.change_member("pins", Some(&k), None, Some(body))
    }
    pub(super) fn save_graphic(&mut self, i: usize) -> Result<()> {
        let f = &self.form;
        let ps = f.geometry.iter().copied().map(coord).collect::<Vec<_>>();
        let unit = if f.unit.is_empty() {
            String::new()
        } else {
            format!(", unit: {}", quote(&f.unit))
        };
        let g = &self.doc.symbols[&self.subject].graphics[i];
        use sch::Shape::*;
        let expr = match &g.shape {
            Line(..) | Rect(..) => format!(
                "{}(from: {}, to: {}{unit})",
                if matches!(g.shape, Line(..)) {
                    "line"
                } else {
                    "rect"
                },
                ps[0],
                ps[1]
            ),
            Arc(..) => format!("arc(from: {}, mid: {}, to: {}{unit})", ps[0], ps[1], ps[2]),
            Text(..) => format!("text(at: {}, value: {}{unit})", ps[0], quote(&f.text)),
            Circle(..) => format!("circle(at: {}, radius: {}mm{unit})", ps[0], f.radius),
            Polyline(..) => format!("polyline(points: [{}]{unit})", ps.join(", ")),
        };
        self.change_member("graphics", None, Some(i), Some(expr))
    }
    fn pin_form(&mut self, ui: &mut Ui) {
        let f = &mut self.form;
        if self.tool == Tool::Pin {
            field(ui, "Pin name", &mut f.name);
        } else {
            ui.strong(&f.name);
        }
        ui.horizontal(|ui| {
            ui.label("Pin number");
            ui.text_edit_singleline(&mut f.number);
        });
        choice(
            ui,
            "Electrical type",
            &mut f.electrical,
            [
                "passive",
                "input",
                "output",
                "bidirectional",
                "power_in",
                "power_out",
                "open_drain",
                "no_connect",
            ]
            .map(str::to_string)
            .to_vec(),
        );
        field(ui, "Unit", &mut f.unit);
        ComboBox::from_label("Direction")
            .selected_text(format!("{}°", f.angle))
            .show_ui(ui, |ui| {
                for a in [0, 90, 180, 270] {
                    ui.selectable_value(&mut f.angle, a, format!("{a}°"));
                }
            });
        for (label, set) in [("Wants", &mut f.wants), ("Gives", &mut f.gives)] {
            ui.label(label);
            if self.doc.role_names.is_empty() {
                ui.weak("No role declarations");
            }
            for (name, id) in &self.doc.role_names {
                let mut enabled = set.contains(name);
                if ui
                    .checkbox(
                        &mut enabled,
                        format!("{name} · {}", self.doc.roles[id].kind.word()),
                    )
                    .changed()
                {
                    if enabled {
                        set.push(name.clone());
                    } else {
                        set.retain(|s| s != name);
                    }
                }
            }
        }
    }
    pub(super) fn save_properties(&mut self) -> Result<()> {
        if self.library() && !self.pin.is_empty() {
            let at = self.doc.symbols[&self.subject].pins[&self.pin].at;
            return self.save_pin(at);
        }
        let f = &self.form;
        let id = if self.library() {
            &self.subject
        } else {
            &self.selected
        };
        let mut ops = vec![];
        if let Some(row) = self.doc.rows.iter().find(|r| &r.id == id) {
            match &row.item {
                Item::Place {
                    ref_offset,
                    value_offset,
                    ..
                } => {
                    for (key, old, new) in [
                        ("ref_offset", ref_offset, f.ref_offset),
                        ("value_offset", value_offset, f.value_offset),
                    ] {
                        if *old != new {
                            ops.push(arg(id, key, new.map_or_else(|| "auto".into(), coord)));
                        }
                    }
                }
                Item::Label { global, .. } if *global != f.global => {
                    ops.push(arg(id, "global", f.global.to_string()))
                }
                _ => {}
            }
        }
        if self.library()
            || self
                .doc
                .rows
                .iter()
                .any(|r| &r.id == id && matches!(&r.item, Item::Place { symbol, .. } if self.doc.symbols[symbol].power_net.is_none()))
        {
            ops.push(arg(id, "fields", field_text(&f.fields)?));
            ops.push(arg(id, "datasheet", quote(&f.datasheet)));
            if !self.library() {
                for (k, v) in [
                    ("value", quote(&f.value)),
                    ("footprint", quote(&f.footprint)),
                    ("populated", f.populated.to_string()),
                    ("in_bom", f.in_bom.to_string()),
                ] {
                    ops.push(arg(id, k, v));
                }
            }
        } else if self
            .doc
            .rows
            .iter()
            .any(|r| &r.id == id && matches!(r.item, Item::Label { .. } | Item::Note(..)))
        {
            ops.push(arg(id, "value", quote(&f.text)));
        }
        if !self.form.name.is_empty() && self.form.name != *id {
            ops.push(Op::Rename {
                from: id.clone(),
                to: self.form.name.clone(),
            });
        }
        let next_id = self.form.name.clone();
        self.apply(ops)?;
        if self.library() {
            self.subject = next_id;
        } else {
            self.selected = next_id;
        }
        self.refresh();
        Ok(())
    }
    pub(super) fn create_record(&mut self) -> Result<()> {
        let f = &self.form;
        let name = f.name.clone();
        let body = match self.tool {
            Tool::Sheet => format!("sheet(title: {})", quote(&f.text)),
            Tool::NewSymbol => "symbol(pins: {}, graphics: [])".into(),
            Tool::Library => format!("library(file: {})", quote(&f.text)),
            Tool::Role => {
                let rating = match f.role_kind.as_str() {
                    _ if f.value.trim().is_empty() => String::new(),
                    "bypass" => format!(", capacitance_nf: {}", f.value),
                    "power" | "reference" => format!(", voltage_mv: {}", f.value),
                    _ => String::new(),
                };
                format!("role(kind: {}{rating})", quote(&f.role_kind))
            }
            Tool::Bus => {
                if f.wants.is_empty() {
                    return Err("Click member pins first.".into());
                }
                let members = f
                    .wants
                    .iter()
                    .enumerate()
                    .map(|(i, k)| format!("p{}: {k}", i + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("bus(sheet: {}, members: {{{members}}})", self.sheet())
            }
            _ => return Ok(()),
        };
        self.add(&name, body)?;
        if matches!(self.tool, Tool::NewSymbol | Tool::Sheet) {
            self.subject = name;
            self.pin.clear();
            self.selected.clear();
            self.refresh();
            self.fit = true;
        }
        self.start(Tool::Select);
        Ok(())
    }
    fn follow_finding(&mut self, id: &str) {
        let Some((scope, object)) = id.rsplit_once('/') else {
            return;
        };
        let (record, field) = object.split_once(':').unwrap_or((object, ""));
        let (name, pin) = record.split_once('.').unwrap_or((record, ""));
        if self.library() && self.doc.symbols.contains_key(scope) {
            self.subject = scope.into();
            self.unit = name.into();
            self.pin = pin.into();
            self.graphic = field.strip_prefix("graphic").and_then(|i| i.parse().ok());
        } else if self.circuit.occurrences.contains_key(scope) {
            self.subject = scope.into();
            self.selected = name.into();
            self.pin = pin.into();
            self.graphic = None;
        } else {
            return;
        }
        self.refresh();
        self.start(Tool::Properties);
    }
    fn selected_pin_detail(&self, ui: &mut Ui) {
        let path = format!("{}/{}.{}", self.subject, self.selected, self.pin);
        let Some(p) = self.circuit.terminals.get(&path) else {
            return;
        };
        ui.strong(format!("{}.{} · pin {}", self.selected, self.pin, p.number));
        ui.label(format!("Electrical type · {}", p.electrical.word()));
        for (label, ids) in [("Wants", &p.wants), ("Gives", &p.gives)] {
            for id in ids {
                ui.label(format!("{label} · {}", id.0));
            }
        }
        if let Ok(json) = sch::response(&self.doc, &self.circuit, &["pin".into(), path])
            && let Some(info) = json.get("result")
        {
            for (field, key) in [("fulfillments", "id"), ("candidates", "give")] {
                if let Some(forge_agent::json::Json::Arr(v)) = info.get(field)
                    && !v.is_empty()
                {
                    ui.strong(if field == "candidates" {
                        "Candidate providers"
                    } else {
                        "Confirmed relationships"
                    });
                    for item in v {
                        let name = item.get(key).and_then(|s| s.str()).unwrap_or("");
                        let ev = item.get("evidence").and_then(|s| s.str()).unwrap_or("");
                        ui.label(format!("{name} {}", ev.replace('_', " ")));
                    }
                }
            }
        }
    }
    pub(super) fn inspector(&mut self, ui: &mut Ui, t: &Theme) {
        let sheet = ux::Sheet::new(
            t,
            ux::System::of_doc("mm"),
            self.path.as_deref().unwrap_or("").trim(),
        );
        sheet.section(
            ui,
            if matches!(self.tool, Tool::Select | Tool::Properties) {
                "Properties"
            } else {
                self.tool.button().0
            },
        );
        let mut result = Ok(());
        use Tool::*;
        let collisions = self
            .circuit
            .issues
            .iter()
            .filter(|i| matches!(i.kind, "TextBoundsOverlap" | "TextGeometryOverlap"))
            .count();
        if self.tool != Checks
            && collisions > 0
            && ui
                .link(format!(
                    "⚠ {collisions} readability warning{}",
                    if collisions == 1 { "" } else { "s" }
                ))
                .clicked()
        {
            self.tool = Checks;
        }
        match self.tool {
            Checks => {
                ui.label(format!("{} findings", self.circuit.issues.len()));
                let issues = self
                    .circuit
                    .issues
                    .iter()
                    .map(|i| (i.severity(), i.message.clone(), i.objects.clone()))
                    .collect::<Vec<_>>();
                for (severity, msg, ids) in issues {
                    ui.separator();
                    ui.colored_label(if severity == "error" { t.error } else { t.warn }, msg);
                    for id in ids {
                        if ui.link(&id).clicked() {
                            self.follow_finding(&id);
                        }
                    }
                }
            }
            Bom => {
                let rows = sch::bom(&self.doc, &self.circuit);
                ui.label(format!("{} purchasing lines", rows.len()));
                if ui.button("Copy CSV").clicked() {
                    ui.ctx().copy_text(sch::bom_csv(&rows));
                }
                for line in &rows {
                    ui.separator();
                    ui.strong(format!("{} × {}", line.references.len(), line.part.value));
                    ui.label(&line.part.symbol);
                    ui.weak(line.references.join(", "));
                    for (k, v) in &line.part.fields {
                        ui.label(format!("{k} · {v}"));
                    }
                }
            }
            Select | Properties => {
                if self.library() && !self.pin.is_empty() {
                    self.pin_form(ui);
                    if ui.button("Apply pin changes").clicked() {
                        result = self.save_properties();
                    }
                } else if self.library() && self.graphic.is_some() {
                    let i = self.graphic.unwrap();
                    ui.strong(format!("Graphic {} · {}", i + 1, self.subject));
                    for (j, p) in self.form.geometry.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("P{}", j + 1));
                            for n in p {
                                ui.add(egui::DragValue::new(n).speed(0.1).suffix(" mm"));
                            }
                        });
                    }
                    match self.doc.symbols[&self.subject].graphics[i].shape {
                        sch::Shape::Text(..) => field(ui, "Text", &mut self.form.text),
                        sch::Shape::Circle(..) => {
                            ui.label("Radius");
                            ui.add(
                                egui::DragValue::new(&mut self.form.radius)
                                    .speed(0.1)
                                    .suffix(" mm"),
                            );
                        }
                        _ => {}
                    }
                    field(ui, "Unit (blank for all units)", &mut self.form.unit);
                    if ui.button("Apply geometry").clicked() {
                        result = self.save_graphic(i);
                    }
                    if ui.button("Delete graphic").clicked() {
                        result = self.modify(Delete);
                        self.graphic = None;
                        self.inspect();
                    }
                } else if self.library() && !self.subject.is_empty() || !self.selected.is_empty() {
                    if !self.pin.is_empty() {
                        self.selected_pin_detail(ui);
                        ui.separator();
                    }
                    field(
                        ui,
                        if self.library() {
                            "Symbol name"
                        } else {
                            "Reference"
                        },
                        &mut self.form.name,
                    );
                    let part = self
                        .doc
                        .rows
                        .iter()
                        .find(|r| r.id == self.selected)
                        .map(|r| &r.item);
                    if let Some(Item::Place { symbol, .. }) = part
                        && let Some(net) = &self.doc.symbols[symbol].power_net
                    {
                        ui.label(format!("Global power net · {net}"));
                    } else if self.library() || matches!(part, Some(Item::Place { .. })) {
                        if let Some(Item::Place { symbol, .. }) = part {
                            ui.label(format!("Symbol · {symbol}"));
                            let base = &self.doc.symbols[symbol];
                            if !base.fields.is_empty() || base.datasheet.is_some() {
                                egui::CollapsingHeader::new("Library defaults")
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        if let Some(link) = &base.datasheet
                                            && ui
                                                .link("Copy datasheet reference")
                                                .on_hover_text(link)
                                                .clicked()
                                        {
                                            ui.ctx().copy_text(link.clone());
                                        }
                                        for (k, v) in &base.fields {
                                            ui.label(format!("{k} · {v}"));
                                        }
                                    });
                            }
                            field(ui, "Value", &mut self.form.value);
                            ui.collapsing("Field positions", |ui| {
                                for (label, offset) in [
                                    ("Reference", &mut self.form.ref_offset),
                                    ("Value", &mut self.form.value_offset),
                                ] {
                                    let mut manual = offset.is_some();
                                    if ui
                                        .checkbox(&mut manual, format!("Position {label} manually"))
                                        .changed()
                                    {
                                        *offset = manual.then_some([0., 0.]);
                                    }
                                    if let Some(p) = offset {
                                        ui.horizontal(|ui| {
                                            for (axis, value) in ["X", "Y"].into_iter().zip(p) {
                                                ui.add(
                                                    egui::DragValue::new(value)
                                                        .prefix(axis)
                                                        .speed(0.1)
                                                        .suffix(" mm"),
                                                );
                                            }
                                        });
                                    }
                                }
                            });
                            field(ui, "Footprint", &mut self.form.footprint);
                            ui.checkbox(&mut self.form.populated, "Populated");
                            ui.checkbox(&mut self.form.in_bom, "Include in BOM");
                        }
                        field(ui, "Datasheet URL or file", &mut self.form.datasheet);
                        if ui
                            .add_enabled(self.path.is_some(), egui::Button::new("Attach PDF…"))
                            .on_hover_text("Copy a PDF beside the saved document")
                            .clicked()
                        {
                            self.attach_requested = true;
                        }
                        sheet.section(ui, "Custom fields");
                        custom(ui, &mut self.form.fields);
                    } else if matches!(part, Some(Item::Note(..) | Item::Label { .. })) {
                        field(ui, "Text", &mut self.form.text);
                        if matches!(part, Some(Item::Label { .. })) {
                            label_scope(ui, &mut self.form.global);
                        }
                    }
                    if ui.button("Apply changes").clicked() {
                        result = self.save_properties();
                    }
                } else {
                    if self.library() {
                        ui.label("Create a symbol to add pins and graphics.");
                    } else {
                        ui.strong(
                            self.doc
                                .sheets
                                .get(self.sheet())
                                .map(String::as_str)
                                .unwrap_or("Schematic"),
                        );
                        ui.label(format!(
                            "{} pins · {} nets",
                            self.circuit.terminals.len(),
                            self.circuit.nets.len()
                        ));
                        ui.weak("Select a component or pin to inspect its properties.");
                    }
                }
            }
            Pin => self.pin_form(ui),
            Power => choice(
                ui,
                "Power net",
                &mut self.form.text,
                sch::power_symbols().keys().cloned().collect(),
            ),
            Place => {
                choice(
                    ui,
                    "Symbol",
                    &mut self.symbol,
                    self.doc.symbols.keys().cloned().collect(),
                );
                field(ui, "Reference", &mut self.form.name);
                field(ui, "Unit", &mut self.form.unit);
            }
            Gate => {
                let components = self
                    .doc
                    .rows
                    .iter()
                    .filter(|r| r.sheet == self.sheet() && matches!(r.item, Item::Place { .. }))
                    .map(|r| r.id.clone())
                    .collect();
                choice(ui, "Component", &mut self.selected, components);
                field(ui, "Unit", &mut self.form.unit);
            }
            Subsheet => {
                choice(
                    ui,
                    "Sheet definition",
                    &mut self.form.group,
                    self.doc.sheets.keys().cloned().collect(),
                );
            }
            Note | Text | Label => {
                field(ui, "Text", &mut self.form.text);
                if self.tool == Label {
                    label_scope(ui, &mut self.form.global);
                }
            }
            Port => field(ui, "Port name", &mut self.form.name),
            Intent => {
                choice(
                    ui,
                    "Role",
                    &mut self.form.role,
                    self.doc.role_names.keys().cloned().collect(),
                );
                let prefix = format!("{}/", self.subject);
                let pins = std::iter::once(String::new())
                    .chain(
                        self.circuit
                            .terminals
                            .keys()
                            .filter_map(|k| k.strip_prefix(&prefix))
                            .filter(|k| !k.contains('/'))
                            .map(str::to_string),
                    )
                    .collect::<Vec<_>>();
                choice(
                    ui,
                    "Provider return (optional)",
                    &mut self.form.return_from,
                    pins.clone(),
                );
                choice(ui, "Consumer return (optional)", &mut self.form.return_to, pins);
                field(
                    ui,
                    "Preferred separation (e.g. 3mm)",
                    &mut self.form.preferred_distance,
                );
            }
            Sheet | Library | Role | NewSymbol | Bus => {
                field(ui, "Name", &mut self.form.name);
                match self.tool {
                    Sheet => field(ui, "Sheet title", &mut self.form.text),
                    Library => field(ui, "Library file (.fg)", &mut self.form.text),
                    Role => {
                        choice(
                            ui,
                            "Role kind",
                            &mut self.form.role_kind,
                            [
                                "bypass",
                                "power",
                                "return",
                                "signal",
                                "clock",
                                "bias",
                                "termination",
                                "feedback",
                                "protection",
                                "reference",
                                "interface",
                            ]
                            .map(str::to_string)
                            .to_vec(),
                        );
                        match self.form.role_kind.as_str() {
                            "bypass" => field(ui, "Capacitance · nF (optional)", &mut self.form.value),
                            "power" | "reference" => {
                                field(ui, "Voltage · mV (optional)", &mut self.form.value)
                            }
                            _ => {}
                        }
                    }
                    Bus => {
                        ui.label(self.form.wants.join(", "));
                    }
                    _ => {}
                }
                if ui
                    .button(if self.tool == Library {
                        "Link library"
                    } else {
                        "Create"
                    })
                    .clicked()
                {
                    result = self.create_record();
                }
            }
            _ => {}
        }
        if !matches!(self.tool, Select | Properties | Bom | Checks) {
            ui.add_space(10.);
            ui.label(RichText::new(self.prompt()).color(t.ink2));
        }
        if let Err(e) = result {
            self.message = e;
        }
    }
}

fn label_scope(ui: &mut Ui, global: &mut bool) {
    ui.horizontal(|ui| {
        ui.selectable_value(global, false, "Local label");
        ui.selectable_value(global, true, "Global label");
    });
}
