use super::*;
use crate::shell::{theme, ux};
use egui::{ComboBox, RichText};
fn property_row<R>(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui, egui::Response) -> R) -> R {
    ui.horizontal(|ui| {
        let width = (ui.available_width() * 0.35).min(116.);
        let caption = ui
            .allocate_ui_with_layout(
                vec2(width, theme::CONTROL_H),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(width);
                    ui.label(RichText::new(label).font(theme::small()))
                },
            )
            .inner;
        control(ui, caption)
    })
    .inner
}
fn field(ui: &mut Ui, label: &str, value: &mut String) {
    text_control(ui, label, value);
}
fn text_control(ui: &mut Ui, label: &str, value: &mut String) -> egui::Response {
    property_row(ui, label, |ui, caption| {
        ui.add_sized(
            [ui.available_width(), theme::CONTROL_H],
            egui::TextEdit::singleline(value),
        )
        .labelled_by(caption.id)
        .on_hover_text(value.as_str())
    })
}

fn checkbox(ui: &mut Ui, label: &str, value: &mut bool) -> egui::Response {
    property_row(ui, label, |ui, caption| {
        let caption = caption.interact(egui::Sense::click());
        if caption.clicked() {
            *value = !*value;
        }
        let mut response = ui
            .add(egui::Checkbox::without_text(value))
            .labelled_by(caption.id);
        if caption.clicked() {
            response.mark_changed();
        }
        response
    })
}

fn choice(ui: &mut Ui, label: &str, value: &mut String, items: Vec<String>) {
    property_row(ui, label, |ui, caption| {
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
            })
            .response
            .labelled_by(caption.id);
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
                self.form.ref_prefix = sym.ref_prefix.clone();
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
                    self.form.description = p.description.clone();
                    self.form.electrical = p.electrical.word().into();
                    self.form.unit = p.unit.clone();
                    self.form.angle = p.angle.to_string();
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
                Item::Note { at, text, size, bold } => {
                    self.form.text = text.clone();
                    self.form.text_size = *size;
                    self.form.bold = *bold;
                    self.form.geometry = vec![*at];
                }
                Item::Drawing { shape, width } => {
                    self.form.line_width = *width;
                    self.form.geometry = annotation::control_points(shape);
                    if let sch::Shape::Circle(_, r) = shape {
                        self.form.radius = *r;
                    }
                }
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
        let old_name = (self.tool != Tool::Pin).then_some(self.pin.as_str());
        let pins = &self.doc.symbols[&self.subject].pins;
        if pins
            .iter()
            .any(|(id, p)| Some(id.as_str()) != old_name && (id == &f.name || p.aliases.contains(&f.name)))
        {
            return Err("This pin name already belongs to another pin.".into());
        }
        let deg = pin_degrees(&f.angle).ok_or("Enter a finite direction in degrees (for example, -22.5).")?;
        let old = old_name.and_then(|k| pins.get(k));
        let mut aliases = old.map(|p| p.aliases.clone()).unwrap_or_default();
        if let Some(previous) = old_name
            && previous != f.name
        {
            aliases.remove(&f.name);
            aliases.insert(previous.into());
        }
        let alias_arg = if aliases.is_empty() {
            String::new()
        } else {
            format!(
                ", aliases: [{}]",
                aliases.into_iter().collect::<Vec<_>>().join(", ")
            )
        };
        let tree = syntax::parse(&self.src).map_err(|e| e.text())?;
        let raw_args = source_arg(&tree, &self.subject, "pins").and_then(|map| {
            let ExprKind::Map(m) = &map.kind else { return None };
            let (_, expr) = m.iter().find(|(k, _)| Some(k.as_str()) == old_name)?;
            let ExprKind::Call(_, args) = &expr.kind else {
                return None;
            };
            Some(args)
        });
        // Keeping the original coordinate and angle expressions preserves shared parameters on rename.
        let original = |key: &str| {
            raw_args?
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|(k, _)| k == key))
                .map(|a| self.src[a.value.span.lo as usize..a.value.span.hi as usize].to_string())
        };
        let position = old
            .filter(|p| p.at == at)
            .and_then(|_| original("at"))
            .unwrap_or_else(|| coord(at));
        let direction = old
            .filter(|p| p.angle == deg.rem_euclid(360.))
            .and_then(|_| original("angle"))
            .unwrap_or_else(|| format!("{deg}deg"));
        let body = format!(
            "pin(number: {}, kind: {}, at: {position}, unit: {}, angle: {direction}, wants: [{}], gives: [{}], description: {}{alias_arg})",
            quote(&f.number),
            quote(&f.electrical),
            quote(&f.unit),
            f.wants.join(", "),
            f.gives.join(", "),
            quote(&f.description)
        );
        let name = f.name.clone();
        let gate = f.unit.clone();
        let key = old_name.unwrap_or(&name).to_string();
        self.change_member("pins", Some((&key, &name)), None, Some(body))?;
        self.pin = name;
        if self.unit != gate {
            self.unit = gate;
            self.refresh();
        }
        self.inspect();
        Ok(())
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
    fn pin_form(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let f = &mut self.form;
        let name = text_control(ui, "Pin name", &mut f.name);
        places.push(("electronics:pin-name".into(), name.rect));
        field(ui, "Pin number", &mut f.number);
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
        let direction = text_control(ui, "Direction (°)", &mut f.angle)
            .on_hover_text("Pin orientation in degrees, counterclockwise from the right.");
        places.push(("electronics:pin-direction".into(), direction.rect));
        let description = ui.label("Description");
        ui.add(
            egui::TextEdit::multiline(&mut f.description)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        )
        .labelled_by(description.id);
        let sheet = ux::Sheet::new(t, ux::System::Metric, "");
        sheet.section(ui, "Pin roles");
        if self.doc.role_names.is_empty() {
            ui.weak("No roles declared");
        } else {
            egui::Grid::new("symbol-pin-roles")
                .num_columns(3)
                .spacing(vec2(theme::S3, theme::S1))
                .min_col_width((ui.available_width() - 2. * theme::S3) / 3.)
                .striped(true)
                .min_row_height(theme::CONTROL_H)
                .show(ui, |ui| {
                    ui.strong("Role");
                    ui.strong("Wants");
                    ui.strong("Provides");
                    ui.end_row();
                    let mut seen = std::collections::BTreeSet::new();
                    for (name, id) in &self.doc.role_names {
                        if !seen.insert(id) {
                            continue;
                        }
                        ui.label(name).on_hover_text(self.doc.roles[id].kind.word());
                        for (direction, set) in [("Wants", &mut f.wants), ("Provides", &mut f.gives)] {
                            let mut on = set.contains(name);
                            let r = ui.add(egui::Checkbox::without_text(&mut on));
                            r.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::Role::CheckBox,
                                    true,
                                    on,
                                    format!("{direction} {name}"),
                                )
                            });
                            if r.on_hover_text(format!("{direction} {name}")).changed() {
                                if on {
                                    set.push(name.clone());
                                } else {
                                    set.retain(|n| n != name);
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
        }
        ui.collapsing("Sub parts", |ui| {
            field(ui, "Sub part", &mut f.unit);
            ui.weak("For a dual op-amp, A and B are separate sections of the same chip.");
        });
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
        if self.library()
            && let Some(symbol) = self.doc.symbols.get(id)
            && symbol.ref_prefix != f.ref_prefix
        {
            ops.push(arg(id, "ref_prefix", quote(&f.ref_prefix)));
        }
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
                Item::Drawing { shape, width } => {
                    ops.extend(annotation::geometry_args(id, shape, &f.geometry, f.radius));
                    if *width != f.line_width {
                        ops.push(arg(id, "width", format!("{}mm", f.line_width)));
                    }
                }
                Item::Note { at, size, bold, .. } => {
                    if *at != f.geometry[0] {
                        ops.push(arg(id, "at", coord(f.geometry[0])));
                    }
                    if *size != f.text_size {
                        ops.push(arg(id, "size", format!("{}mm", f.text_size)));
                    }
                    if *bold != f.bold {
                        ops.push(arg(id, "bold", f.bold.to_string()));
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
            .any(|r| &r.id == id && matches!(r.item, Item::Label { .. } | Item::Note{..}))
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
            Tool::NewSymbol => format!(
                "symbol(ref_prefix: {}, pins: {{}}, graphics: [])",
                quote(&f.ref_prefix)
            ),
            Tool::Library => format!("library(file: {})", quote(&f.text)),
            Tool::Role => {
                let rating = match f.role_kind.as_str() {
                    "power" if !f.voltage_min.is_empty() || !f.voltage_max.is_empty() => {
                        if f.voltage_min.trim().is_empty()
                            || f.voltage_max.trim().is_empty()
                            || !f.value.trim().is_empty()
                        {
                            return Err("For a voltage range, enter both limits in volts and clear the single voltage.".into());
                        }
                        format!(", voltage_range_v: [{}, {}]", f.voltage_min, f.voltage_max)
                    }
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
    fn selected_pin_detail(&mut self, ui: &mut Ui) {
        let id = format!("{}/{}.{}", self.subject, self.selected, self.pin);
        if let Some(p) = self.circuit.terminals.get(&id) {
            ui.strong(format!(
                "{} · pin {} · {}",
                self.pin,
                p.number,
                p.electrical.word()
            ));
            if ui.link("Inspect pin relationships").clicked() {
                self.focus_on(&id);
            }
        }
    }
    pub(super) fn inspector(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let path = self.path.clone().unwrap_or_default();
        let sheet = ux::Sheet::new(t, ux::System::of_doc("mm"), path.trim());
        if matches!(self.tool, Tool::Select | Tool::Properties) {
            let title = if self.library() {
                &self.subject
            } else {
                &self.selected
            };
            if !title.is_empty() {
                ui.add_space(theme::S2);
                ui.label(RichText::new(title).font(theme::heading()).color(t.ink));
            }
            sheet.section(
                ui,
                if self.library() && !self.pin.is_empty() {
                    "Pin properties"
                } else if !self.selected.is_empty() {
                    "Object properties"
                } else {
                    "Properties"
                },
            );
        } else {
            sheet.section(ui, self.tool.button().0);
        }
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
                    "{collisions} readability warning{}",
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
                    self.pin_form(ui, t, places);
                    let apply = ui.button("Apply pin changes");
                    places.push(("electronics:apply-pin".into(), apply.rect));
                    if apply.clicked() {
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
                    field(ui, "Sub part (blank for all)", &mut self.form.unit);
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
                    if self.library() {
                        let r = text_control(ui, "Reference prefix", &mut self.form.ref_prefix)
                            .on_hover_text("Letters used for new references, e.g. R1, C1 or TP1.");
                        places.push(("electronics:ref-prefix".into(), r.rect));
                    }
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
                            property_row(ui, "Symbol", |ui, _| ui.label(symbol));
                            field(ui, "Value", &mut self.form.value);
                            ui.collapsing("Field positions", |ui| {
                                for (label, offset) in [
                                    ("Reference", &mut self.form.ref_offset),
                                    ("Value", &mut self.form.value_offset),
                                ] {
                                    let mut manual = offset.is_some();
                                    if checkbox(ui, &format!("Manual {label} position"), &mut manual)
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
                            sheet.section(ui, "Assembly");
                            field(ui, "Footprint", &mut self.form.footprint);
                            checkbox(ui, "Populated", &mut self.form.populated);
                            checkbox(ui, "Include in BOM", &mut self.form.in_bom);
                        }
                        if let Some(Item::Place { symbol, .. }) = part {
                            let base = &self.doc.symbols[symbol];
                            if !base.fields.is_empty() || base.datasheet.is_some() {
                                egui::CollapsingHeader::new("Library defaults")
                                    .default_open(false)
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
                        }
                        sheet.section(ui, "Documentation");
                        field(ui, "Datasheet", &mut self.form.datasheet);
                        if ui
                            .add_enabled(self.path.is_some(), egui::Button::new("Attach PDF…"))
                            .on_hover_text("Copy a PDF beside the saved document")
                            .clicked()
                        {
                            self.attach_requested = true;
                        }
                        sheet.section(ui, "Custom fields");
                        custom(ui, &mut self.form.fields);
                    } else if let Some(Item::Drawing { shape, .. }) = part {
                        annotation::controls(ui, &mut self.form, Some(shape));
                    } else if matches!(part, Some(Item::Note { .. } | Item::Label { .. })) {
                        if matches!(part, Some(Item::Note { .. })) {
                            annotation::text_controls(ui, &mut self.form);
                            annotation::point_fields(ui, &mut self.form.geometry);
                        }

                        field(ui, "Text", &mut self.form.text);
                        if matches!(part, Some(Item::Label { .. })) {
                            label_scope(ui, &mut self.form.global);
                        }
                    }
                    let apply = ui.button("Apply changes");
                    places.push(("electronics:apply-properties".into(), apply.rect));
                    if apply.clicked() {
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
            Pin => self.pin_form(ui, t, places),
            Power => choice(
                ui,
                "Power net",
                &mut self.form.text,
                sch::power_symbols().keys().cloned().collect(),
            ),
            Place => {
                let previous = self.symbol.clone();
                choice(
                    ui,
                    "Symbol",
                    &mut self.symbol,
                    self.doc.symbols.keys().cloned().collect(),
                );
                if self.symbol != previous {
                    self.form.name = self.fresh(&self.doc.symbols[&self.symbol].ref_prefix);
                }
                field(ui, "Reference", &mut self.form.name);
                field(ui, "Sub part", &mut self.form.unit);
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
                field(ui, "Sub part", &mut self.form.unit);
            }
            Subsheet => {
                choice(
                    ui,
                    "Sheet definition",
                    &mut self.form.group,
                    self.doc.sheets.keys().cloned().collect(),
                );
            }
            Line | Rect | Circle | Arc if !self.library() => annotation::controls(ui, &mut self.form, None),
            Note | Text | Label => {
                if self.tool == Note {
                    annotation::text_controls(ui, &mut self.form);
                }
                field(ui, "Text", &mut self.form.text);
                if self.tool == Label {
                    label_scope(ui, &mut self.form.global);
                }
            }
            Port => field(ui, "Port name", &mut self.form.name),
            Intent => {
                if let Some((pin, need)) = &self.form.checklist_pin {
                    ui.label(RichText::new(pin.rsplit('/').next().unwrap_or(pin)).font(theme::heading()));
                    ui.label(format!(
                        "{} · {}",
                        if *need { "Wants" } else { "Provides" },
                        self.form.role
                    ));
                } else {
                    choice(
                        ui,
                        "Role",
                        &mut self.form.role,
                        self.doc.role_names.keys().cloned().collect(),
                    );
                }
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
                if let Some((pin, need)) = &self.form.checklist_pin {
                    let mut chosen = None;
                    let role = &self.doc.role_names[&self.form.role];
                    sheet.section(
                        ui,
                        if *need {
                            "Connected providers"
                        } else {
                            "Connected consumers"
                        },
                    );
                    let mut any = false;
                    for key in &self.circuit.nets[&self.circuit.net_of[pin]] {
                        let Some((scope, short)) = key.rsplit_once('/') else {
                            continue;
                        };
                        if scope == self.subject
                            && key != pin
                            && self.circuit.terminals.get(key).is_some_and(|peer| {
                                (if *need { &peer.gives } else { &peer.wants }).contains(role)
                            })
                        {
                            any = true;
                            let r = ui.button(format!("Assign {short}"));
                            places.push((format!("electronics:assign:{key}"), r.rect));
                            if r.clicked() {
                                chosen = Some(short.to_owned());
                            }
                        }
                    }
                    if !any {
                        ui.weak("Connect a pin with the matching role on this sheet.");
                    }
                    if let Some(short) = chosen {
                        result = self.click([0., 0.], Some(&short), None);
                    }
                }
            }
            Sheet | Library | Role | NewSymbol | Bus => {
                field(ui, "Name", &mut self.form.name);
                match self.tool {
                    Sheet => field(ui, "Sheet title", &mut self.form.text),
                    NewSymbol => field(ui, "Reference prefix", &mut self.form.ref_prefix),
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
                                field(ui, "Voltage · mV (optional)", &mut self.form.value);
                                if self.form.role_kind == "power" {
                                    field(ui, "Minimum (V)", &mut self.form.voltage_min);
                                    field(ui, "Maximum (V)", &mut self.form.voltage_max);
                                }
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
