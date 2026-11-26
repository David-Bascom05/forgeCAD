use super::workspace::row;
use super::*;

impl Editor {
    pub(super) fn focus_on(&mut self, id: &str) {
        let Some(inspection) =
            sch::inspect_pin(self.variant_checks.as_ref().map_or(&self.circuit, |(_, c)| c), id)
        else {
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
            manage: None,
            id: id.into(),
            inspection,
            page,
        });
        self.inspector_open = true;
        self.fit = true;
    }
    pub(super) fn dock_tabs(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let width = (ui.available_width() - ui.spacing().item_spacing.x * 2.) / 3.;
        ui.horizontal(|ui| {
            for (tab, label) in [
                (Dock::Parts, "Parts"),
                (Dock::Power, "Power"),
                (Dock::Properties, "Properties"),
            ] {
                let active = self.dock == tab;
                let r = ui.add_sized(
                    [width, crate::shell::theme::TITLE_ROW_H],
                    egui::Button::selectable(active, label).frame_when_inactive(false),
                );
                if active {
                    ui.painter()
                        .hline(r.rect.x_range(), r.rect.bottom(), egui::Stroke::new(2., t.accent));
                }
                places.push((format!("electronics:dock:{label}"), r.rect));
                if r.clicked() && self.dock != tab {
                    self.start(Tool::Select);
                    self.dock = tab;
                }
            }
        });
        ui.separator();
    }
    pub(super) fn dock_ui(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        if self.library() || self.dock == Dock::Properties {
            ui.spacing_mut().item_spacing = vec2(crate::shell::theme::S2, crate::shell::theme::S1);
        }
        if self.library() {
            self.inspector(ui, t, places);
            return;
        }
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
                        .rsplit_once('.')
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
                    self.inspector(ui, t, places)
                }
            }
        }
    }
    pub(super) fn filter_catalog(&mut self) {
        let words: Vec<_> = self
            .catalog_search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let mut tree = LibraryBranch::default();
        for (name, terms) in &self.catalog {
            if words.iter().all(|w| terms.contains(w)) {
                let (namespace, _) = name.rsplit_once('.').unwrap_or(("", name));
                tree.at(namespace).symbols.push(name.clone());
            }
        }
        self.catalog_tree = tree;
    }
    fn parts_ui(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.catalog_search)
                    .hint_text("Search symbols, pins or part numbers"),
            )
            .changed()
        {
            self.filter_catalog();
        }
        let tree = &self.catalog_tree;
        let mut selected = None;
        if tree.symbols.is_empty() && tree.children.is_empty() {
            ui.weak("No matching symbols");
        } else {
            tree.show(
                ui,
                !self.catalog_search.trim().is_empty(),
                self.catalog.len() < 100,
                &mut |ui, name| {
                    let short = name.rsplit('.').next().unwrap_or(name);
                    let symbol = &self.doc.symbols[name];
                    let pins = format!("{} pins", symbol.pins.len());
                    if row(
                        places,
                        ui,
                        t,
                        short,
                        symbol.fields.get("MPN").unwrap_or(&pins),
                        self.tool == Tool::Place && self.symbol == name,
                        "schchip",
                    ) {
                        selected = Some(name.to_owned());
                    }
                    if let Some((key, _)) = places.last_mut() {
                        *key = format!("electronics:part:{name}");
                    }
                },
            );
        }
        if let Some(name) = selected {
            self.symbol = name;
            self.start(Tool::Place);
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
                let reference = ui.text_edit_singleline(&mut self.form.name);
                places.push(("electronics:place-reference".into(), reference.rect));
            });
            let units: std::collections::BTreeSet<_> = s.pins.values().map(|p| p.unit.as_str()).collect();
            if units.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label("Sub part");
                    egui::ComboBox::from_id_salt("place-sub-part")
                        .selected_text(&self.form.unit)
                        .show_ui(ui, |ui| {
                            for unit in units {
                                ui.selectable_value(&mut self.form.unit, unit.into(), unit);
                            }
                        });
                });
            }
            ui.label("Click the sheet to place");
        }
        ui.separator();
        if ui.button("Link library…").clicked() {
            self.start(Tool::Library);
        }
        for (name, path) in &self.doc.libraries {
            if let Some(path) = path
                && ui
                    .link(format!("Open {name}"))
                    .on_hover_text(path.display().to_string())
                    .clicked()
            {
                self.open_file = Some(path.to_string_lossy().into_owned());
            }
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

#[derive(Default)]
pub(super) struct LibraryBranch {
    children: BTreeMap<String, LibraryBranch>,
    symbols: Vec<String>,
}
impl LibraryBranch {
    fn at(&mut self, path: &str) -> &mut Self {
        let mut node = self;
        for name in path.split('.').filter(|s| !s.is_empty()) {
            node = node.children.entry(name.to_owned()).or_default();
        }
        node
    }
    fn show(&self, ui: &mut Ui, searching: bool, open: bool, row: &mut impl FnMut(&mut Ui, &str)) {
        for name in &self.symbols {
            row(ui, name);
        }
        for (name, child) in &self.children {
            let header = egui::CollapsingHeader::new(name.as_str()).default_open(open);
            let header = if searching {
                header.open(Some(true))
            } else {
                header
            };
            header.show(ui, |ui| child.show(ui, searching, open, row));
        }
    }
}
