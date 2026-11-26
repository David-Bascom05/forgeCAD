use super::*;
use crate::shell::{self, theme, ux};
use egui::{Align2, ScrollArea, Sense, Stroke, UiBuilder};
impl Tool {
    pub(in crate::shell) fn shortcut(self) -> (egui::Key, &'static str) {
        use Tool::*;
        match self {
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
                "Rotate the selected symbol or gate by a quarter turn.",
            ),
            Mirror => (egui::Key::Y, "Reflect the selected symbol or gate."),
            Delete => (
                egui::Key::Delete,
                "Remove the selection after checking its references.",
            ),
            Sheet => (egui::Key::N, "Create a sheet definition."),
            Subsheet => (egui::Key::H, "Place an occurrence of a sheet definition."),
            Gate => (egui::Key::G, "Place another unit of an existing component."),
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
            Grid => (egui::Key::Q, "Toggle the drawing grid and placement snapping."),
            Properties => (egui::Key::I, "Inspect and edit the selected object."),
            Intent => (egui::Key::U, "Assign a typed provider to a consumer pin."),
            Bus => (egui::Key::D, "Pick connections for a named bus."),
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
            Text => (egui::Key::T, "Place graphic text in the active symbol unit."),
            Role => (egui::Key::K, "Declare a typed role for symbol pins."),
        }
    }
    pub(super) fn button(self) -> (&'static str, &'static str) {
        use Tool::*;
        match self {
            Select => ("Select", "select"),
            Power => ("Power", "schpower"),
            Place => ("Place Symbol", "schchip"),
            Wire => ("Wire", "schwire"),
            Junction => ("Junction", "schjunction"),
            Label => ("Net Label", "text"),
            Port => ("Port", "schpin"),
            NoConnect => ("No Connect", "close"),
            Note => ("Note", "markup"),
            Move => ("Move", "move"),
            Rotate => ("Rotate", "rotate"),
            Mirror => ("Mirror", "mirror"),
            Delete => ("Delete", "delete"),
            Sheet => ("New Sheet", "sheet"),
            Subsheet => ("Sheet Instance", "subasm"),
            Gate => ("Place Gate", "schchip"),
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
            Bus => ("Bus", "schbus"),
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
                    on: self.tool == *t || (*t == Grid && self.grid),
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
                group("Connections", &[Intent], &[NoConnect, Bus, Flow]),
                group("Modify", &[Move], &[Rotate, Mirror, Delete]),
                group("Sheets", &[Sheet], &[Subsheet, Gate, Note]),
                group("Library", &[Library], &[]),
            ],
            (0, true) => vec![
                group("Symbol", &[NewSymbol, Pin], &[Role]),
                group("Graphics", &[Line, Rect, Circle], &[Arc, Text]),
                group("Modify", &[Properties], &[Move, Delete]),
            ],
            (1, false) => vec![group("Inspect", &[Properties, Checks, Bom], &[])],
            (1, true) => vec![group("Inspect", &[Properties, Checks], &[])],
            _ => vec![group("Navigate", &[Fit, Grid, Select], &[])],
        }
    }
    fn browser(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let section = |ui: &mut Ui, s: &str| ux::Sheet::new(t, ux::System::of_doc("mm"), "").section(ui, s);
        if self.library() {
            section(ui, "Symbols");
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Find symbol"));
            for id in self.doc.symbols.keys().cloned().collect::<Vec<_>>() {
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
                    egui::ComboBox::from_label("Unit")
                        .selected_text(&self.unit)
                        .show_ui(ui, |ui| {
                            for unit in units {
                                ui.selectable_value(&mut self.unit, unit.clone(), unit);
                            }
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
            section(ui, "Sheets");
            let sheets = self.circuit.occurrences.keys().cloned().collect::<Vec<_>>();
            for id in sheets {
                if row(places, ui, t, &id, "sheet", self.subject == id, "sheet") {
                    self.subject = id;
                    self.fit = true;
                    self.selected.clear();
                    self.pin.clear();
                    self.start(Tool::Select);
                    self.refresh();
                }
            }
            section(ui, "Components");
            let rows = self
                .doc
                .rows
                .iter()
                .filter(|r| r.sheet == self.sheet())
                .map(|r| {
                    (
                        r.id.clone(),
                        match &r.item {
                            Item::Place { symbol, .. } => Some(symbol.clone()),
                            Item::Gate { of, unit, .. } => Some(format!("{of}.{unit}")),
                            _ => None,
                        },
                    )
                })
                .collect::<Vec<_>>();
            for (id, kind) in &rows {
                if let Some(k) = kind
                    && row(
                        places,
                        ui,
                        t,
                        id,
                        k.rsplit('.').next().unwrap_or(k),
                        self.selected == *id,
                        "schchip",
                    )
                {
                    self.select(id, "");
                }
            }
            egui::CollapsingHeader::new("Connections and notes").show(ui, |ui| {
                for (id, k) in &rows {
                    if k.is_none() && row(places, ui, t, id, "", self.selected == *id, "schwire") {
                        self.select(id, "");
                    }
                }
            });
            section(ui, "Symbol libraries");
            for path in self.dependencies() {
                if row(
                    places,
                    ui,
                    t,
                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("Library"),
                    "open",
                    false,
                    "folder",
                ) {
                    self.open_file = Some(path.to_string_lossy().into());
                }
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
fn row(
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
                    Tool::Checks,
                    Tool::Bom,
                    Tool::Intent,
                    Tool::Bus,
                    Tool::Flow,
                ][..]
            };
            tools.extend_from_slice(specific);
            tools.extend([Tool::Move, Tool::Delete, Tool::Fit, Tool::Grid, Tool::Properties]);
            for cmd in tools {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, cmd.shortcut().0)) {
                    self.out.push(Action::Electronics(cmd));
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
                editor.inspector_open = true;
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
            (s.width() * 0.25).clamp(240., 344.).min(s.width() * 0.8)
        };
        let view = Rect::from_min_max(pos2(left, top), pos2(s.width() - right, bottom));
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
        editor.canvas(ui, view, &t);
        if right > 0. {
            let r = Rect::from_min_max(pos2(view.right(), top), pos2(s.width(), bottom));
            let mut b = pane(ui, "electronics-inspector", r);
            ui.painter().vline(r.left(), r.y_range(), Stroke::new(1., t.line));
            ScrollArea::vertical().show(&mut b, |ui| {
                if narrow && ui.button("Back to sheet").clicked() {
                    editor.inspector_open = false;
                }
                if narrow {
                    ui.collapsing("Document browser", |ui| editor.browser(ui, &t, &mut self.places));
                }
                editor.inspector(ui, &t);
            });
        }
        if narrow && right == 0. {
            let r = Rect::from_min_size(view.right_top() + vec2(-100., 5.), vec2(95., 25.));
            if ui.put(r, egui::Button::new("Properties")).clicked() {
                editor.inspector_open = true;
            }
        }
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
