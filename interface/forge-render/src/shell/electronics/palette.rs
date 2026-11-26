use super::workspace::row;
use super::*;

impl Editor {
    pub(super) fn focus_on(&mut self, id: &str) {
        let Some(inspection) = sch::inspect_pin(&self.circuit, id) else {
            return;
        };
        let Some((scope, terminal)) = id.rsplit_once('/') else {
            return;
        };
        let Some((component, pin)) = terminal.split_once('.') else {
            return;
        };
        if scope != self.subject {
            return;
        }
        self.select(component, pin);
        let page = sch::pin_inspection_page(&self.doc, &self.circuit, scope, &inspection).unwrap_or_default();
        self.focus_pin = Some(PinFocus {
            id: id.into(),
            inspection,
            page,
        });
        self.inspector_open = true;
        self.fit = true;
    }
    pub(super) fn dock_ui(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        if self.library() {
            self.inspector(ui, t);
            return;
        }
        ui.horizontal(|ui| {
            for (tab, label) in [
                (Dock::Parts, "Parts"),
                (Dock::Power, "Power"),
                (Dock::Properties, "Properties"),
            ] {
                let r = ui.selectable_label(self.dock == tab, label);
                places.push((format!("electronics:dock:{label}"), r.rect));
                if r.clicked() && self.dock != tab {
                    self.start(Tool::Select);
                    self.dock = tab;
                }
            }
        });
        ui.separator();
        match self.dock {
            Dock::Parts => self.parts_ui(ui, t, places),
            Dock::Power => {
                ui.add(egui::TextEdit::singleline(&mut self.power_search).hint_text("Find power symbol"));
                let query = self.power_search.trim().to_lowercase();
                for net in sch::power_symbols()
                    .keys()
                    .filter(|k| k.to_lowercase().contains(&query))
                {
                    if row(
                        places,
                        ui,
                        t,
                        net,
                        "forge:power",
                        self.tool == Tool::Power
                            && self.form.power_symbol.is_none()
                            && self.form.text == *net,
                        "schpower",
                    ) {
                        self.start(Tool::Power);
                        self.form.text = net.clone();
                    }
                }
                let mut project_choice = None;
                for (id, s) in &self.doc.symbols {
                    let Some(net) = &s.power_net else { continue };
                    if id
                        .split_once('.')
                        .is_some_and(|(alias, _)| self.power_aliases.iter().any(|a| a == alias))
                    {
                        continue;
                    }
                    if !net.to_lowercase().contains(&query) && !id.to_lowercase().contains(&query) {
                        continue;
                    }
                    if row(
                        places,
                        ui,
                        t,
                        net,
                        id,
                        self.form.power_symbol.as_ref() == Some(id),
                        "schpower",
                    ) {
                        project_choice = Some((id.clone(), net.clone()));
                    }
                    if let Some((key, _)) = places.last_mut() {
                        *key = format!("electronics:power:{id}");
                    }
                }
                if let Some((id, net)) = project_choice {
                    self.start(Tool::Power);
                    self.form.power_symbol = Some(id);
                    self.form.text = net;
                }
                if self.tool == Tool::Power {
                    ui.separator();
                    ui.label(format!("Click to place {}", self.form.text));
                }
            }
            Dock::Properties => {
                if self.focus_pin.is_some() {
                    self.checklist(ui, t, places)
                } else {
                    self.inspector(ui, t)
                }
            }
        }
    }
    fn parts_ui(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        ui.add(
            egui::TextEdit::singleline(&mut self.catalog_search)
                .hint_text("Search symbols, pins or part numbers"),
        );
        let words: Vec<_> = self
            .catalog_search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let mut library = None;
        let mut selected = None;
        for (name, terms) in &self.catalog {
            if !words.iter().all(|s| terms.contains(s)) {
                continue;
            }
            let (lib, short) = name.rsplit_once('.').unwrap_or(("Document", name));
            if library != Some(lib) {
                ui.add_space(6.);
                ui.strong(lib);
                library = Some(lib);
            }
            let symbol = &self.doc.symbols[name];
            let pins = format!("{} pins", symbol.pins.len());
            if row(
                places,
                ui,
                t,
                short,
                symbol.fields.get("MPN").unwrap_or(&pins),
                self.tool == Tool::Place && self.symbol == *name,
                "schchip",
            ) {
                selected = Some(name.clone());
            }
            if let Some((key, _)) = places.last_mut() {
                *key = format!("electronics:part:{name}");
            }
        }
        if library.is_none() {
            ui.weak("No matching symbols");
        }
        if let Some(name) = selected {
            self.start(Tool::Place);
            self.symbol = name;
            self.form.unit = self.doc.symbols[&self.symbol]
                .pins
                .values()
                .map(|p| p.unit.as_str())
                .min()
                .unwrap_or("A")
                .into();
        }
        if self.tool == Tool::Place
            && let Some(s) = self.doc.symbols.get(&self.symbol)
        {
            ui.separator();
            ui.strong(&self.symbol);
            if self
                .preview
                .as_ref()
                .is_none_or(|(name, unit, _)| name != &self.symbol || unit != &self.form.unit)
            {
                self.preview = sch::symbol_page(&self.doc, &self.symbol, &self.form.unit)
                    .ok()
                    .map(|page| (self.symbol.clone(), self.form.unit.clone(), page));
            }
            if let Some((_, _, page)) = &self.preview {
                symbol_preview(ui, t, page);
            }
            for (k, v) in &s.fields {
                ui.label(format!("{k} · {v}"));
            }
            if let Some(link) = &s.datasheet
                && ui.link("Copy datasheet reference").on_hover_text(link).clicked()
            {
                ui.ctx().copy_text(link.clone());
            }
            ui.horizontal(|ui| {
                ui.label("Reference");
                ui.text_edit_singleline(&mut self.form.name);
            });
            let units: std::collections::BTreeSet<_> = s.pins.values().map(|p| p.unit.as_str()).collect();
            if units.len() > 1 {
                egui::ComboBox::from_label("Unit")
                    .selected_text(&self.form.unit)
                    .show_ui(ui, |ui| {
                        for unit in units {
                            ui.selectable_value(&mut self.form.unit, unit.into(), unit);
                        }
                    });
            }
            ui.label("Click the sheet to place");
        }
        ui.separator();
        if ui.button("Link library…").clicked() {
            self.start(Tool::Library);
        }
        for path in self.dependencies() {
            if let Some(file) = path.file_name()
                && ui.link(format!("Open {}", file.to_string_lossy())).clicked()
            {
                self.open_file = Some(path.to_string_lossy().into());
            }
        }
    }
    fn checklist(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let f = self.focus_pin.as_ref().unwrap();
        let p = &self.circuit.terminals[&f.id];
        let label = f.id.rsplit('/').next().unwrap_or(&f.id);
        let heading = ui.heading("Pin checklist");
        ui.add_space(4.);
        ui.strong(label);
        places.push((format!("electronics:inspection:{}", f.id), heading.rect));
        ui.weak(format!(
            "Pin {} · {}",
            p.number,
            p.electrical.word().replace('_', " ")
        ));
        let description = &self.doc.symbols[&p.symbol].pins[&p.pin].description;
        if !description.is_empty() {
            let brief = description
                .split_once('\n')
                .map_or(description.as_str(), |(first, _)| first);
            let end = brief
                .char_indices()
                .nth(160)
                .map(|(n, _)| n)
                .unwrap_or(brief.len());
            if end < description.len() {
                ui.label(format!("{}…", brief[..end].trim_end()))
                    .on_hover_text(description);
            } else {
                ui.label(description);
            }
        }
        let mut next = None;
        for (need, title) in [(true, "Needs"), (false, "Provides")] {
            ui.separator();
            let count = f.inspection.checks.iter().filter(|r| r.need == need).count();
            let verified = f
                .inspection
                .checks
                .iter()
                .filter(|r| r.need == need && r.peers.iter().any(|(_, e)| *e == sch::Evidence::Verified))
                .count();
            if count > 0 {
                ui.strong(format!("{title}   {verified} / {count} verified"));
            } else {
                ui.strong(title);
            }
            for check in f.inspection.checks.iter().filter(|r| r.need == need) {
                let role = self
                    .doc
                    .role_names
                    .iter()
                    .find(|(_, id)| **id == check.role)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or(&check.role.0);
                let verified = check.peers.iter().any(|(_, s)| *s == sch::Evidence::Verified);
                let spec = &self.doc.roles[&check.role];
                let mut title = role.rsplit('.').next().unwrap_or(role).replace('_', " ");
                if let Some(r) = &spec.rating {
                    match r {
                        sch::Rating::VoltageMv(mv) => title.push_str(&format!(" · {} V", mv / 1000.)),
                        sch::Rating::CapacitanceNf(nf) => title.push_str(&format!(" · {nf} nF")),
                    }
                }
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(13., 13.), egui::Sense::hover());
                    let ink = if verified { t.success } else { t.ink2 };
                    ui.painter().rect_stroke(
                        rect.shrink(1.),
                        2.,
                        egui::Stroke::new(1., ink),
                        egui::StrokeKind::Inside,
                    );
                    if verified {
                        let point = |x, y| rect.min + vec2(x, y);
                        ui.painter().add(egui::Shape::line(
                            vec![point(3., 6.), point(5.5, 9.), point(10., 3.5)],
                            egui::Stroke::new(1.5, ink),
                        ));
                    }
                    ui.add_space(4.);
                    let row = ui.strong(title).on_hover_text(&check.role.0);
                    places.push((format!("electronics:check:{}:{}", f.id, check.role.0), row.rect));
                });
                if need && matches!(spec.kind, sch::RoleKind::Power | sch::RoleKind::Return) {
                    for (rail, pin) in &f.inspection.rails {
                        let row = ui.label(format!("{rail} connected"));
                        places.push((format!("electronics:rail:{pin}"), row.rect));
                    }
                }
                if check.peers.is_empty() {
                    let missing = if need && spec.kind == sch::RoleKind::Power {
                        "No power source assigned"
                    } else if need {
                        "No provider"
                    } else {
                        "No consumer"
                    };
                    ui.colored_label(t.warn, missing);
                }
                for (peer, status) in &check.peers {
                    let (sheet, name) = peer.rsplit_once('/').unwrap_or(("", peer));
                    let caption = if sheet == self.subject { name } else { peer };
                    ui.horizontal_wrapped(|ui| {
                        if ui.link(caption).clicked() {
                            next = Some(peer.clone());
                        }
                        ui.add_space(6.);
                        match status {
                            sch::Evidence::Verified => ui.colored_label(t.success, "Verified"),
                            sch::Evidence::Invalid => ui.colored_label(t.error, "Check wiring"),
                            sch::Evidence::Candidate => ui.weak("Candidate"),
                        };
                    });
                }
            }
            if count == 0 {
                ui.weak("None declared");
            }
        }
        if let Some(id) = next
            && let Some((scope, _)) = id.rsplit_once('/')
        {
            if self.subject != scope {
                self.focus_pin = None;
                self.subject = scope.into();
                self.refresh();
            }
            self.focus_on(&id);
        }
        ui.separator();
        if ui.button("Close pin inspection").clicked() {
            self.start(Tool::Select);
        }
    }
}

/// Scale the library's own geometry into its catalogue preview.
fn symbol_preview(ui: &mut Ui, t: &Theme, page: &forge_draw::Page) {
    let mut bounds = Rect::NOTHING;
    for s in &page.strokes {
        let (a, b) = s.geom.bounds();
        bounds.extend_with(pos2(a.x as f32, a.y as f32));
        bounds.extend_with(pos2(b.x as f32, b.y as f32));
    }
    for text in &page.texts {
        bounds.extend_with(pos2(text.at.x as f32, text.at.y as f32));
        bounds.extend_with(pos2(
            (text.at.x + forge_draw::page::text_width(&text.text, text.h)) as f32,
            (text.at.y + text.h) as f32,
        ));
    }
    if !bounds.is_finite() {
        return;
    }
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.), egui::Sense::hover());
    let scale = ((r.width() - 8.) / bounds.width().max(1.)).min(86. / bounds.height().max(1.));
    let to_screen = |x: f64, y: f64| {
        r.center() + vec2(x as f32 - bounds.center().x, bounds.center().y - y as f32) * scale
    };
    let painter = ui.painter_at(r);
    for stroke in &page.strokes {
        painter.add(egui::Shape::line(
            stroke
                .geom
                .points(0.3 / f64::from(scale))
                .into_iter()
                .map(|p| to_screen(p.x, p.y))
                .collect(),
            egui::Stroke::new(1.2, t.ink),
        ));
    }
    for text in &page.texts {
        // At thumbnail scale, show the symbol's inscriptions and keep pin numbers in its editor.
        if text.h >= 1. {
            painter.text(
                to_screen(text.at.x, text.at.y),
                egui::Align2::LEFT_BOTTOM,
                &text.text,
                egui::FontId::proportional((text.h as f32 * scale).max(8.)),
                t.ink,
            );
        }
    }
}
