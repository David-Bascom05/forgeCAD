use super::*;
use crate::shell::{self, theme, ux};
use egui::{Align2, ScrollArea, Sense, Stroke, UiBuilder};
impl Tool {
    pub(in crate::shell) fn shortcut(self) -> (egui::Key, &'static str) {
        use Tool::*;
        match self {
            Block => (
                egui::Key::F4,
                "Select a block, review its boundary and move or duplicate it.",
            ),
            Select => (
                egui::Key::Escape,
                "Return to selection and cancel the current tool.",
            ),
            Power => (
                egui::Key::V,
                "Place a named supply or ground. Pick a pin to attach it directly.",
            ),
            Place => (egui::Key::A, "Place a component from a linked symbol library."),
            Wire => (
                egui::Key::W,
                "Pick a terminal, click bends, then finish on a terminal or wire.",
            ),
            Junction => (
                egui::Key::J,
                "Split a wire at the picked point to make a branch connection.",
            ),
            Label => (egui::Key::L, "Name the net attached to a terminal or junction."),
            Port => (egui::Key::P, "Place a named connection for a hierarchical sheet."),
            NoConnect => (egui::Key::X, "Mark a pin as intentionally unconnected."),
            Note => (egui::Key::T, "Add text to the sheet."),
            Move => (egui::Key::M, "Pick a new position for the selected object."),
            Rotate => (
                egui::Key::R,
                "Rotate the selected symbol or sub part by a quarter turn.",
            ),
            Mirror => (egui::Key::Y, "Reflect the selected symbol or sub part."),
            Delete => (
                egui::Key::Delete,
                "Remove the selection after checking its references.",
            ),
            Sheet => (egui::Key::N, "Create a sheet definition."),
            Subsheet => (egui::Key::H, "Place an occurrence of a sheet definition."),
            Gate => (egui::Key::G, "Place another sub part of an existing component."),
            Library => (
                egui::Key::K,
                "Reference a project symbol library by its .fg path.",
            ),
            Checks => (
                egui::Key::E,
                "Inspect electrical and readability findings for the design.",
            ),
            Bom => (
                egui::Key::B,
                "Review fitted components and copy their purchasing data as CSV.",
            ),
            Fit => (egui::Key::F, "Fit the drawing to the canvas."),
            Grid => (egui::Key::Q, "Show or hide the sheet grid."),
            Properties => (egui::Key::I, "Inspect and edit the selected object."),
            Intent => (egui::Key::U, "Assign a typed provider to a consumer pin."),
            DiffPair => (
                egui::Key::F8,
                "Associate two nets with positive and negative polarity.",
            ),
            Bus => (egui::Key::D, "Pick connections for a named bus."),
            Dots => (egui::Key::F7, "Animate dots along known signal paths."),
            Directions => (
                egui::Key::F6,
                "Animate known directions along wires. Red opposing arrows mark a conflict.",
            ),
            Flow => (egui::Key::O, "Record signal flow between two terminals."),
            NewSymbol => (egui::Key::N, "Add a named symbol definition to the library."),
            Pin => (
                egui::Key::P,
                "Place a pin with a physical number, electrical type and typed roles.",
            ),
            Line => (egui::Key::L, "Draw a line between two points."),
            Rect => (egui::Key::R, "Draw a rectangle through opposite corners."),
            Circle => (egui::Key::C, "Pick a center and a point on the circle."),
            Arc => (
                egui::Key::A,
                "Pick a start point, a point on the arc, and its end.",
            ),
            Text => (egui::Key::T, "Place graphic text in the active sub part."),
            Role => (egui::Key::K, "Declare a typed role for symbol pins."),
        }
    }
    pub(super) fn button(self) -> (&'static str, &'static str) {
        use Tool::*;
        match self {
            Block => ("Edit Block", "select"),
            Select => ("Select", "select"),
            Power => ("Power", "schpower"),
            Place => ("Place Symbol", "schchip"),
            Wire => ("Wire", "schwire"),
            Junction => ("Junction", "schjunction"),
            Label => ("Net Label", "text"),
            Port => ("Port", "schpin"),
            NoConnect => ("No Connect", "close"),
            Note => ("Text", "text"),
            Move => ("Move", "move"),
            Rotate => ("Rotate", "rotate"),
            Mirror => ("Mirror", "mirror"),
            Delete => ("Delete", "delete"),
            Sheet => ("New Sheet", "sheet"),
            Subsheet => ("Sheet Instance", "subasm"),
            Gate => ("Place Sub Part", "schchip"),
            Library => ("Link Library", "folder"),
            Checks => ("Electrical Checks", "check"),
            Bom => ("Bill of Materials", "partslist"),
            Fit => ("Fit All", "fit"),
            Grid => ("Grid", "grid"),
            NewSymbol => ("New Symbol", "schchip"),
            Pin => ("Pin", "schpin"),
            Line => ("Line", "line"),
            Rect => ("Rectangle", "rect"),
            Circle => ("Circle", "circle"),
            Arc => ("Arc", "arc"),
            Text => ("Text", "text"),
            Role => ("Pin Role", "schjunction"),
            Properties => ("Properties", "params"),
            Intent => ("Assign Role", "connector"),
            DiffPair => ("Differential Pair", "schbus"),
            Bus => ("Bus", "schbus"),
            Directions => ("Direction Arrows", "arrowmark"),
            Dots => ("Moving Dots", "schdots"),
            Flow => ("Signal Flow", "schwire"),
        }
    }
}
impl Editor {
    pub(in crate::shell) fn groups(&self, tab: usize) -> Vec<Group> {
        use Tool::*;
        let group = |name, big: &[Tool], small: &[Tool]| {
            let button = |t: &Tool| {
                let (name, icon) = t.button();
                RibbonTool {
                    name,
                    icon,
                    action: Some(Action::Electronics(*t)),
                    on: self.tool == *t
                        || (*t == Grid && self.grid)
                        || (*t == Directions && self.markers.arrows)
                        || (*t == Dots && self.markers.dots),
                }
            };
            Group {
                name,
                big: big.iter().map(button).collect(),
                small: small.iter().map(button).collect(),
                icons: vec![],
            }
        };
        match (tab, self.library()) {
            (0, false) => vec![
                group("Place", &[Place, Wire, Power], &[Junction, Label, Port]),
                group("Connections", &[Intent], &[NoConnect, Bus, DiffPair, Flow]),
                group("Modify", &[Move, Block], &[Rotate, Mirror, Delete]),
                group("Sheets", &[Sheet], &[Subsheet, Gate]),
                group("Library", &[Library], &[]),
            ],
            (0, true) => vec![
                group("Symbol", &[NewSymbol, Pin], &[Role]),
                group("Libraries", &[Library], &[]),
                group("Graphics", &[Line, Rect, Circle], &[Arc, Text]),
                group("Modify", &[Properties], &[Move, Delete]),
            ],
            (3, false) => vec![
                group("Geometry", &[Line, Rect, Circle], &[Arc]),
                group("Annotation", &[Note], &[]),
                group("Modify", &[Properties, Move], &[Delete]),
            ],
            (1, false) => vec![group("Inspect", &[Properties, Checks, Bom], &[])],
            (1, true) => vec![group("Inspect", &[Properties, Checks], &[])],
            (2, false) => vec![
                group("Navigate", &[Fit, Grid, Select], &[]),
                group("Signals", &[Directions, Dots], &[]),
            ],
            _ => vec![group("Navigate", &[Fit, Grid, Select], &[])],
        }
    }
    fn browser(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let section = |ui: &mut Ui, s: &str| ux::Sheet::new(t, ux::System::of_doc("mm"), "").section(ui, s);
        if self.library() {
            ui.collapsing("Libraries", |ui| {
                for (alias, path) in &self.doc.libraries {
                    let depth = alias.matches('.').count();
                    ui.horizontal(|ui| {
                        ui.add_space(12. * depth as f32);
                        let label = alias.rsplit('.').next().unwrap_or(alias);
                        if let Some(path) = path {
                            if ui.link(label).on_hover_text(path.display().to_string()).clicked() {
                                self.open_file = Some(path.to_string_lossy().into_owned());
                            }
                        } else {
                            ui.weak(label).on_hover_text("Bundled library");
                        }
                    });
                }
            });
            if ui.button("Add child library…").clicked() {
                self.start(Tool::Library);
            }
            section(ui, "Symbols");
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Find symbol"));
            for id in self
                .doc
                .symbols
                .keys()
                .filter(|id| !id.contains('.'))
                .cloned()
                .collect::<Vec<_>>()
            {
                if id.to_lowercase().contains(&self.search.to_lowercase())
                    && row(places, ui, t, &id, "symbol", self.subject == id, "schchip")
                {
                    self.subject = id;
                    self.pin.clear();
                    self.graphic = None;
                    self.start(Tool::Select);
                    self.refresh();
                    self.fit = true;
                }
            }
            if let Some(s) = self.doc.symbols.get(&self.subject) {
                let pins = s
                    .pins
                    .iter()
                    .map(|(id, p)| (id.clone(), p.number.clone(), p.unit.clone()))
                    .collect::<Vec<_>>();
                let units = self.doc.symbols[&self.subject]
                    .pins
                    .values()
                    .map(|p| p.unit.clone())
                    .collect::<std::collections::BTreeSet<_>>();
                if units.len() > 1 {
                    let previous = self.unit.clone();
                    ui.horizontal(|ui| {
                        ui.label("Sub part");
                        egui::ComboBox::from_id_salt("section-picker")
                            .selected_text(&self.unit)
                            .show_ui(ui, |ui| {
                                for unit in units {
                                    ui.selectable_value(&mut self.unit, unit.clone(), unit);
                                }
                            });
                    });
                    if self.unit != previous {
                        self.pin.clear();
                        self.graphic = None;
                        self.refresh();
                        self.inspect();
                        self.fit = true;
                    }
                }
                section(ui, "Pins");
                for (id, n, unit) in pins {
                    if row(places, ui, t, &id, &n, self.pin == id, "schpin") {
                        self.pin = id;
                        self.unit = unit;
                        self.graphic = None;
                        self.tool = Tool::Select;
                        self.inspect();
                        self.refresh();
                    }
                }
                section(ui, "Graphics");
                let gs = self.doc.symbols[&self.subject]
                    .graphics
                    .iter()
                    .map(|g| match g.shape {
                        sch::Shape::Line(..) => "Line",
                        sch::Shape::Rect(..) => "Rectangle",
                        sch::Shape::Circle(..) => "Circle",
                        sch::Shape::Arc(..) => "Arc",
                        sch::Shape::Text(..) => "Text",
                        sch::Shape::Polyline(..) => "Polyline",
                    })
                    .collect::<Vec<_>>();
                for (i, n) in gs.iter().enumerate() {
                    if row(
                        places,
                        ui,
                        t,
                        n,
                        &(i + 1).to_string(),
                        self.graphic == Some(i),
                        "line",
                    ) {
                        self.pin.clear();
                        self.graphic = Some(i);
                        self.tool = Tool::Properties;
                        self.inspect();
                    }
                }
            }
        } else {
            section(ui, "Sheet hierarchy");
            if ui
                .add(egui::TextEdit::singleline(&mut self.search).hint_text("Find across sheets"))
                .changed()
            {
                self.hits = sch::search(&self.doc, &self.circuit, &self.search);
            }
            if !self.search.trim().is_empty() {
                let mut target = None;
                for h in &self.hits {
                    let label = h
                        .zone
                        .as_ref()
                        .map_or_else(|| h.id.clone(), |z| format!("{} · {z}", h.id));
                    if ui.link(label).on_hover_text(&h.label).clicked() {
                        target = Some(h.id.clone());
                    }
                }
                if self.hits.is_empty() {
                    ui.weak("No matches");
                }
                if let Some(id) = target {
                    if self.circuit.occurrences.contains_key(&id) {
                        self.subject = id;
                        self.selected.clear();
                        self.pin.clear();
                        self.refresh();
                    } else {
                        self.follow_finding(&id);
                    }
                }
                ui.separator();
            }
            ui.collapsing("Sheet settings", |ui| {
                let id = self.sheet().to_owned();
                let title = self.doc.sheets.get(&id).cloned().unwrap_or_default();
                ui.label(&title);
                let current = self.doc.templates.get(&id).cloned().unwrap_or_default();
                egui::ComboBox::from_id_salt("sheet-template")
                    .selected_text(if current.is_empty() {
                        "Choose template"
                    } else {
                        &current
                    })
                    .show_ui(ui, |ui| {
                        for name in self
                            .doc
                            .symbols
                            .iter()
                            .filter(|(_, s)| s.pins.is_empty() && !s.graphics.is_empty())
                            .map(|(k, _)| k.clone())
                            .collect::<Vec<_>>()
                        {
                            if ui.selectable_label(name == current, &name).clicked()
                                && let Err(e) = self.apply(vec![arg(&id, "template", name)])
                            {
                                self.message = e;
                            }
                        }
                    });
                if ui.button("Reannotate project").on_hover_text("Assign unique references across all sheet occurrences. Locked references keep their numbers.").clicked() {
                    match sch::project_annotation(&self.src,&self.doc,&self.circuit).map_err(|e|e.message).and_then(|ops|self.apply(ops)){
                        Ok(())=>self.inspect(), Err(err)=>self.message=err
                    }
                }
                if ui.button("Export project PDF").clicked() {
                    let result = self
                        .path
                        .as_ref()
                        .ok_or_else(|| "Save the schematic first.".to_owned())
                        .and_then(|file| {
                            let out = Path::new(file).with_extension("pdf");
                            let data = sch::sheet_pdf(&self.doc, &self.circuit).map_err(|e| e.message)?;
                            forge_dirs::fs::write(&out, data).map_err(|e| e.to_string())?;
                            Ok(format!("Exported {}", out.display()))
                        });
                    self.message = result.unwrap_or_else(|e| e);
                }
            });
            // Iterate subtrees explicitly: lexicographic path ordering can interleave roots
            // such as a and a_2 with descendants of a.
            let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for path in self.circuit.occurrences.keys() {
                children
                    .entry(path.rsplit_once('/').map_or("", |(parent, _)| parent))
                    .or_default()
                    .push(path);
            }
            let mut pending: Vec<_> = children
                .get("")
                .into_iter()
                .flatten()
                .rev()
                .map(|&p| (p, 0))
                .collect();
            let mut chosen = None;
            while let Some((id, depth)) = pending.pop() {
                let label = id.rsplit('/').next().unwrap_or(id);
                let has_children = children.contains_key(id);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    ui.add_space(12. * depth as f32);
                    if has_children {
                        let (rect, resp) = ui.allocate_exact_size(vec2(14., 20.), Sense::click());
                        egui::collapsing_header::paint_default_icon(
                            ui,
                            if self.collapsed.contains(id) { 0. } else { 1. },
                            &resp,
                        );
                        places.push((format!("electronics:expand:{id}"), rect));
                        if resp.clicked() && !self.collapsed.remove(id) {
                            self.collapsed.insert(id.into());
                        }
                    } else {
                        ui.add_space(16.);
                    }
                    let def = &self.circuit.occurrences[id];
                    let title = &self.doc.sheets[def];
                    if row(
                        places,
                        ui,
                        t,
                        label,
                        if title == label { "" } else { title },
                        self.subject == id,
                        "sheet",
                    ) {
                        chosen = Some(id.to_owned());
                    }
                    if let Some((key, _)) = places.last_mut() {
                        *key = format!("electronics:sheet:{id}");
                    }
                });
                if !self.collapsed.contains(id) {
                    pending.extend(
                        children
                            .get(id)
                            .into_iter()
                            .flatten()
                            .rev()
                            .map(|&p| (p, depth + 1)),
                    );
                }
            }
            if let Some(id) = chosen {
                self.subject = id;
                self.selected.clear();
                self.pin.clear();
                self.start(Tool::Select);
                self.refresh();
                self.fit = true;
            }
        }
    }
    pub(super) fn select(&mut self, id: &str, pin: &str) {
        self.selected = id.into();
        self.pin = pin.into();
        self.graphic = None;
        self.start(Tool::Select);
    }
}
pub(super) fn row(
    places: &mut Vec<(String, Rect)>,
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    detail: &str,
    on: bool,
    icon: &str,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 23.), Sense::click());
    places.push((format!("electronics:{icon}:{name}"), r));
    let p = ui.painter_at(r);
    if on || resp.hovered() {
        p.rect_filled(r, 2., if on { t.on } else { t.hover });
    }
    super::super::icons::draw(
        &p,
        pos2(r.left() + 3., r.center().y - 7.),
        14.,
        icon,
        1.25,
        if on { t.on_ink } else { t.ink2 },
    );
    let w = p.layout_no_wrap(detail.into(), theme::small(), t.ink2).size().x;
    let text_rect = Rect::from_min_max(r.min, pos2((r.right() - w - 8.).max(r.left()), r.bottom()));
    p.with_clip_rect(text_rect).text(
        pos2(r.left() + 23., r.center().y),
        Align2::LEFT_CENTER,
        name,
        theme::sans(13.),
        t.ink,
    );
    p.text(
        pos2(r.right() - 3., r.center().y),
        Align2::RIGHT_CENTER,
        detail,
        theme::small(),
        t.ink2,
    );
    resp.on_hover_text(name).clicked()
}
impl Shell {
    pub(in crate::shell) fn electronics_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let s = ctx.content_rect();
        let t = self.theme;
        if !ctx.egui_wants_keyboard_input() {
            let e = self.electronics.as_ref().unwrap();
            let mut tools = vec![Tool::Select];
            let specific = if e.library() {
                &[
                    Tool::NewSymbol,
                    Tool::Pin,
                    Tool::Line,
                    Tool::Rect,
                    Tool::Circle,
                    Tool::Arc,
                    Tool::Text,
                    Tool::Role,
                ][..]
            } else {
                &[
                    Tool::Place,
                    Tool::Power,
                    Tool::Wire,
                    Tool::Junction,
                    Tool::Label,
                    Tool::Port,
                    Tool::NoConnect,
                    Tool::Note,
                    Tool::Rotate,
                    Tool::Mirror,
                    Tool::Sheet,
                    Tool::Subsheet,
                    Tool::Gate,
                    Tool::Library,
                    Tool::Block,
                    Tool::Checks,
                    Tool::Bom,
                    Tool::Intent,
                    Tool::Bus,
                    Tool::DiffPair,
                    Tool::Flow,
                    Tool::Directions,
                    Tool::Dots,
                ][..]
            };
            tools.extend_from_slice(specific);
            tools.extend([Tool::Move, Tool::Delete, Tool::Fit, Tool::Grid, Tool::Properties]);
            if !e.library() && self.tab == 3 {
                tools.retain(|t| !matches!(t, Tool::Label | Tool::Rotate | Tool::Place));
            }
            for cmd in tools {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, cmd.shortcut().0)) {
                    self.out.push(Action::Electronics(cmd));
                }
            }
            if !e.library() && self.tab == 3 {
                for cmd in [Tool::Line, Tool::Rect, Tool::Circle, Tool::Arc] {
                    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, cmd.shortcut().0)) {
                        self.out.push(Action::Electronics(cmd));
                    }
                }
            }
            self.shortcuts(&ctx);
        }
        self.title_bar(ui, s.width());
        self.tabs_row(ui, s.width());
        self.ribbon(ui, s.width());
        let mut editor = self.electronics.take().unwrap();
        self.out.retain(|a| {
            if let Action::Electronics(cmd) = a {
                editor.start(*cmd);
                if !matches!(cmd, Tool::Directions | Tool::Dots) {
                    editor.inspector_open = true;
                }
                false
            } else {
                true
            }
        });
        let top = shell::TITLE_H + shell::TABS_H + shell::RIBBON_H;
        let bottom = (s.height() - shell::DOCS_H - shell::STATUS_H).max(top);
        let narrow = s.width() < 780.;
        let left = if narrow {
            0.
        } else {
            self.side_px
                .unwrap_or(if s.width() < 1150. { 190. } else { 260. })
                .min(s.width() * 0.25)
        };
        let right = if narrow && !editor.inspector_open {
            0.
        } else {
            (s.width() * 0.25).clamp(320., 380.).min(s.width() * 0.8)
        };
        let view = Rect::from_min_max(pos2(left, top), pos2(s.width() - right, (bottom - 29.).max(top)));
        self.view_rect = view;
        self.view_owned = true;
        let pane = |ui: &mut Ui, id: &str, r: Rect| {
            ui.painter().rect_filled(r, 0., t.surface);
            let inner = r.shrink2(vec2(10., 6.));
            let mut child = ui.new_child(UiBuilder::new().id_salt(id).max_rect(inner));
            child.set_clip_rect(inner);
            child
        };
        if left > 0. {
            let r = Rect::from_min_max(pos2(0., top), pos2(left, bottom));
            let mut b = pane(ui, "electronics-browser", r);
            ScrollArea::vertical().show(&mut b, |ui| editor.browser(ui, &t, &mut self.places));
            self.side_splitter(ui, r, s.width());
        }
        editor.canvas(ui, view, &t, &mut self.places);
        let strip = Rect::from_min_max(view.left_bottom(), pos2(view.right(), bottom));
        let mut grid_ui = pane(ui, "electronics-grid", strip);
        editor.grid_controls(&mut grid_ui, &mut self.places);
        if right > 0. {
            let r = Rect::from_min_max(pos2(view.right(), top), pos2(s.width(), bottom));
            let mut b = pane(ui, "electronics-inspector", r);
            ui.painter().vline(r.left(), r.y_range(), Stroke::new(1., t.line));
            if !editor.library() {
                editor.dock_tabs(&mut b, &t, &mut self.places);
            }
            // A long parts list or component form must not leave a newly opened checklist scrolled away.
            let scroll_id = ("properties", editor.focus_pin.as_ref().map(|f| &f.id));
            ScrollArea::vertical().id_salt(scroll_id).show(&mut b, |ui| {
                if narrow && ui.button("Back to sheet").clicked() {
                    editor.inspector_open = false;
                }
                if narrow {
                    ui.collapsing("Document browser", |ui| editor.browser(ui, &t, &mut self.places));
                }
                editor.dock_ui(ui, &t, &mut self.places);
            });
        }
        if narrow && right == 0. {
            let r = Rect::from_min_size(view.right_top() + vec2(-100., 5.), vec2(95., 25.));
            if ui.put(r, egui::Button::new("Properties")).clicked() {
                editor.inspector_open = true;
            }
        }
        editor.attribute_window(&ctx, &t);
        self.state_status(
            &ctx,
            if editor.message.is_empty() {
                format!("{} · {}", editor.subject, editor.prompt())
            } else {
                editor.message.clone()
            },
        );
        if std::mem::take(&mut editor.attach_requested) {
            self.out.push(Action::AttachDatasheet);
        }
        self.electronics = Some(editor);
        self.docs_and_status(ui, s, bottom);
        self.resize_edges(ui, s);
        self.menus(&ctx);
        self.settings_window(&ctx);
    }
}
