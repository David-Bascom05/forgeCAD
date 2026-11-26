use super::*;
use crate::shell::{
    self, Action, Shell, Theme,
    ribbon::{Group, Tool},
    ux,
};
use egui::{Rect, ScrollArea, Stroke, Ui, UiBuilder, pos2, vec2};

impl Command {
    fn button(self) -> (&'static str, &'static str) {
        match self {
            Self::Place => ("Place Footprint", "schchip"),
            Self::Select => ("Select", "select"),
            Self::Move => ("Move", "move"),
            Self::Rotate => ("Rotate", "rotate"),
            Self::Flip => ("Flip Side", "mirror"),
            Self::Route => ("Route Track", "schwire"),
            Self::Checks => ("Design Rules", "check"),
            Self::Stack => ("Stackup", "partslist"),
            Self::Fit => ("Fit All", "fit"),
            Self::Grid => ("Grid", "grid"),
            Self::Layers => ("Layers", "sheet"),
            Self::Properties => ("Properties", "params"),
        }
    }
    pub(in crate::shell) fn tip(self) -> &'static str {
        match self {
            Self::Place => "Choose a footprint and its pin mapping, then click its position on the board.",
            Self::Select => "Pick an object on the board. Escape cancels the active tool.",
            Self::Move => "Select a footprint, then click its new position on the grid.",
            Self::Rotate => "Rotate the selected footprint 90 degrees.",
            Self::Flip => "Move the selected footprint to the other face.",
            Self::Route => "Choose a copper layer and net. Click route points and press Enter to finish.",
            Self::Checks => "Inspect physical connectivity and fabrication rule findings.",
            Self::Stack => "Inspect the laminate and select a fabrication service.",
            Self::Fit => "Fit the board profile to the window.",
            Self::Grid => "Toggle the board grid.",
            Self::Layers => "Show and activate board layers.",
            Self::Properties => "Inspect the selected board object.",
        }
    }
}
impl Editor {
    pub(in crate::shell) fn groups(&self, tab: usize) -> Vec<Group> {
        use Command::*;
        let group = |name, commands: &[Command]| Group {
            name,
            big: commands
                .iter()
                .map(|&c| {
                    let (name, icon) = c.button();
                    Tool {
                        name,
                        icon,
                        action: Some(Action::Pcb(c)),
                        on: self.mode == c || (c == Grid && self.grid),
                    }
                })
                .collect(),
            small: vec![],
            icons: vec![],
        };
        match tab {
            1 => vec![group("Inspect", &[Checks, Stack, Properties])],
            2 => vec![group("Navigate", &[Fit, Grid, Layers])],
            _ => vec![
                group("Layout", &[Select, Route]),
                group("Placement", &[Place, Move, Rotate, Flip]),
                group("Board", &[Layers, Stack, Checks]),
            ],
        }
    }
    fn layers_ui(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        ui.add(egui::TextEdit::singleline(&mut self.find_layer).hint_text("Find layer"));
        let search = self.find_layer.trim().to_ascii_lowercase();
        for group in [
            "Copper",
            "Surface",
            "Special processes",
            "Machining",
            "Constraints",
            "Assembly",
            "Connectivity",
            "Documentation",
        ] {
            let matches = |l: &&canvas::SceneLayer| {
                l.layer.group() == group
                    && (search.is_empty()
                        || l.layer
                            .title(&self.rev.board)
                            .to_ascii_lowercase()
                            .contains(&search))
            };
            if !self.rev.scene.layers.iter().any(|l| matches(&l)) {
                continue;
            }
            let header = egui::CollapsingHeader::new(group)
                .default_open(matches!(group, "Copper" | "Surface" | "Machining"));
            let header = if search.is_empty() {
                header
            } else {
                header.open(Some(true))
            };
            header.show(ui, |ui| {
                for l in self.rev.scene.layers.iter().filter(matches) {
                    let key = &l.layer;
                    let name = key.title(&self.rev.board);
                    ui.horizontal(|ui| {
                        let mut visible = !self.hidden.contains(key);
                        let resp = ui
                            .checkbox(&mut visible, "")
                            .on_hover_text(format!("Show {name}"));
                        places.push((format!("pcb:visible:{}", key.id()), resp.rect));
                        if resp.changed() {
                            if visible {
                                self.hidden.remove(key);
                            } else {
                                self.hidden.insert(key.clone());
                            }
                        }
                        let (rect, _) = ui.allocate_exact_size(vec2(11., 11.), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 2., canvas::color(key, &self.rev.board, t));
                        let resp = ui
                            .selectable_label(&self.active == key, &name)
                            .on_hover_text(format!("{} artwork primitives", l.count()));
                        places.push((format!("pcb:layer:{}", key.id()), resp.rect));
                        if resp.clicked() {
                            self.active = key.clone();
                            self.hidden.remove(key);
                            self.draft.clear();
                            self.mode = Command::Select;
                        }
                    });
                }
            });
        }
    }
    pub(super) fn add_net_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.net_name)
                    .desired_width(95.)
                    .hint_text("New net"),
            );
            if ui.button("Add").clicked() && !self.net_name.trim().is_empty() {
                let id = self.next_id("net");
                let name = forge_lang::props::quote(self.net_name.trim());
                if self.edit(vec![Op::AddFeature {
                    before: None,
                    text: format!("{id} = net(name: {name})"),
                }]) {
                    self.net = id;
                    self.net_name.clear();
                }
            }
        });
    }
    fn inspector(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let sheet = ux::Sheet::new(t, ux::System::of_doc("mm"), "");
        match self.panel {
            Inspector::Layers => self.layers_ui(ui, t, places),
            Inspector::Checks => {
                sheet.section(ui, "Design rules");
                if self.rev.checks.is_empty() {
                    ui.colored_label(t.success, "No findings");
                }
                for (rule, id, detail) in &self.rev.checks {
                    if ui
                        .selectable_label(self.selected == *id, format!("{rule} · {id}"))
                        .clicked()
                    {
                        self.selected = id.clone();
                    }
                    ui.small(detail);
                }
            }
            Inspector::Stack => {
                sheet.section(ui, "Physical stackup");
                egui::Grid::new("pcb-stack").striped(true).show(ui, |ui| {
                    ui.strong("Layer");
                    ui.strong("Thickness");
                    ui.end_row();
                    for l in &self.rev.board.layers {
                        ui.label(&l.id);
                        ui.label(format!("{:.3} mm", mm(l.thickness_nm)));
                        ui.end_row();
                    }
                });
                sheet.section(ui, "Fabrication service");
                let process = self.rev.board.process.as_ref();
                let sel = process.and_then(|p| p.selected.as_ref());
                let mut change = None;
                egui::ComboBox::from_id_salt("pcb-vendor")
                    .selected_text(
                        sel.map(|s| format!("{} · {}", s.vendor, s.service.id))
                            .unwrap_or_else(|| "Board rules".into()),
                    )
                    .show_ui(ui, |ui| {
                        for (v, s, name) in &self.rev.services {
                            if ui
                                .selectable_label(
                                    sel.is_some_and(|p| p.vendor == *v && p.service.id == *s),
                                    name,
                                )
                                .clicked()
                            {
                                change = Some((v.clone(), s.clone()));
                            }
                        }
                    });
                if let Some(p) = process {
                    for (k, n) in [
                        ("min_width", p.min_width_nm),
                        ("min_spacing", p.min_spacing_nm),
                        ("min_drill", p.min_drill_nm),
                        ("min_annular_ring", p.min_annular_ring_nm),
                    ] {
                        sheet.text(ui, k, &format!("{} mm", mm(n)), None);
                    }
                    if let Some(s) = sel {
                        ui.hyperlink_to("Service specifications", &s.service.source);
                    }
                }
                if let Some((v, s)) = change {
                    let v = forge_lang::props::quote(&v);
                    let s = forge_lang::props::quote(&s);
                    let ops = match process {
                        Some(p) => vec![field(&p.id, "vendor", v), field(&p.id, "service", s)],
                        None => vec![Op::AddFeature {
                            before: None,
                            text: format!("{} = process(vendor: {v}, service: {s})", self.next_id("process")),
                        }],
                    };
                    self.edit(ops);
                }
            }
            Inspector::Properties => {
                if self.mode == Command::Place {
                    sheet.section(ui, "Place footprint");
                    self.placement_form(ui);
                }
                if self.mode == Command::Route {
                    sheet.section(ui, "Route track");
                    ui.horizontal(|ui| {
                        ui.label("Net");
                        egui::ComboBox::from_id_salt("pcb-net")
                            .selected_text(
                                self.rev
                                    .board
                                    .nets
                                    .get(&self.net)
                                    .map(|n| n.name.as_str())
                                    .unwrap_or("Choose net"),
                            )
                            .show_ui(ui, |ui| {
                                for (id, n) in &self.rev.board.nets {
                                    ui.selectable_value(&mut self.net, id.clone(), &n.name);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        ui.add(
                            egui::DragValue::new(&mut self.width)
                                .range(0.001..=50.)
                                .speed(0.025)
                                .suffix(" mm"),
                        );
                    });
                    self.add_net_ui(ui);
                    if ui
                        .add_enabled(self.draft.len() >= 2, egui::Button::new("Finish track"))
                        .clicked()
                    {
                        self.commit_route();
                    }
                }
                sheet.section(ui, "Board");
                let board = &self.rev.board;
                let bounds = self.rev.scene.bounds;
                sheet.text(
                    ui,
                    "size",
                    &format!(
                        "{} × {} mm",
                        mm(bounds[1][0] - bounds[0][0]),
                        mm(bounds[1][1] - bounds[0][1])
                    ),
                    None,
                );
                sheet.text(
                    ui,
                    "thickness",
                    &format!("{:.3} mm", mm(board.layers.iter().map(|l| l.thickness_nm).sum())),
                    None,
                );
                sheet.text(ui, "nets", &board.nets.len().to_string(), None);
                sheet.text(
                    ui,
                    "service",
                    &board
                        .process
                        .as_ref()
                        .and_then(|p| p.selected.as_ref())
                        .map(|s| format!("{} · {}", s.vendor, s.service.id))
                        .unwrap_or_else(|| "Board rules".into()),
                    None,
                );
                if let Some(p) = board.layout.placements.get(&self.selected) {
                    sheet.section(ui, &p.reference);
                    sheet.text(ui, "footprint", &p.footprint, None);
                    sheet.text(ui, "side", p.pose.side.name(), None);
                    sheet.text(
                        ui,
                        "position",
                        &format!("{}, {} mm", mm(p.pose.at[0]), mm(p.pose.at[1])),
                        None,
                    );
                    sheet.text(ui, "rotation", &format!("{}°", p.pose.angle), None);
                    for (id, pad) in board
                        .layout
                        .pads
                        .iter()
                        .filter(|(_, p)| p.placement == self.selected)
                    {
                        sheet.text(ui, id, &board.nets[&pad.net].name, None);
                    }
                } else if !self.selected.is_empty() {
                    sheet.section(ui, &self.selected);
                    if let Some(track) = board.tracks.get(&self.selected) {
                        sheet.text(ui, "net", &board.nets[&track.net].name, None);
                        sheet.text(ui, "layer", &track.layer, None);
                        sheet.text(ui, "width", &format!("{} mm", mm(track.width_nm)), None);
                    }
                    if let Some(v) = board.vias.get(&self.selected) {
                        sheet.text(ui, "net", &board.nets[&v.net].name, None);
                        sheet.text(ui, "drill", &format!("{} mm", mm(v.drill_nm)), None);
                    }
                }
                egui::CollapsingHeader::new("Footprints")
                    .default_open(self.selected.is_empty())
                    .show(ui, |ui| {
                        for (id, p) in &board.layout.placements {
                            let resp = ui.selectable_label(
                                self.selected == *id,
                                format!("{}   {}", p.reference, p.footprint),
                            );
                            places.push((format!("pcb:component:{id}"), resp.rect));
                            if resp.clicked() {
                                self.selected = id.clone();
                            }
                        }
                    });
            }
        }
    }
}
impl Shell {
    pub(in crate::shell) fn pcb_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let r = ctx.content_rect();
        let t = self.theme;
        if !ctx.egui_wants_keyboard_input() {
            self.shortcuts(&ctx);
        }
        self.title_bar(ui, r.width());
        self.tabs_row(ui, r.width());
        self.ribbon(ui, r.width());
        let mut e = self.pcb.take().unwrap();
        self.out.retain(|a| {
            if let Action::Pcb(c) = a {
                e.command(*c);
                false
            } else {
                true
            }
        });
        if !ctx.egui_wants_keyboard_input() {
            for (k, c) in [
                (egui::Key::Escape, Command::Select),
                (egui::Key::M, Command::Move),
                (egui::Key::R, Command::Rotate),
                (egui::Key::X, Command::Route),
                (egui::Key::F, Command::Fit),
            ] {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k)) {
                    e.command(c);
                }
            }
            if e.mode == Command::Route
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                e.commit_route();
            }
        }
        let top = shell::TITLE_H + shell::TABS_H + shell::RIBBON_H;
        let bottom = (r.height() - shell::DOCS_H - shell::STATUS_H).max(top);
        let narrow = r.width() < 850.;
        let left = if narrow {
            0.
        } else {
            self.side_px.unwrap_or(222.).min(r.width() * 0.24)
        };
        let right = if narrow && !e.compact_panel {
            0.
        } else {
            (r.width() * 0.24).clamp(260., 340.).min(r.width() * 0.78)
        };
        let pane = |ui: &mut Ui, id: &str, r: Rect| {
            ui.painter().rect_filled(r, 0., t.surface);
            let rect = r.shrink2(vec2(10., 6.));
            let mut u = ui.new_child(UiBuilder::new().id_salt(id).max_rect(rect));
            u.set_clip_rect(rect);
            u
        };
        if left > 0. {
            let area = Rect::from_min_max(pos2(0., top), pos2(left, bottom));
            let mut u = pane(ui, "pcb-browser", area);
            ux::Sheet::new(&t, ux::System::of_doc("mm"), "").section(&mut u, "Board layers");
            ScrollArea::vertical()
                .id_salt("pcb-layer-list")
                .show(&mut u, |ui| e.layers_ui(ui, &t, &mut self.places));
            self.side_splitter(ui, area, r.width());
        }
        let view = Rect::from_min_max(pos2(left, top), pos2(r.width() - right, (bottom - 32.).max(top)));
        self.view_rect = view;
        self.view_owned = true;
        e.canvas(ui, view, &t, &mut self.places);
        let mut strip = pane(
            ui,
            "pcb-grid",
            Rect::from_min_max(view.left_bottom(), pos2(view.right(), bottom)),
        );
        strip.horizontal(|ui| {
            ui.label("Grid");
            egui::ComboBox::from_id_salt("pcb-pitch")
                .selected_text(format!("{} mm", e.pitch))
                .width(76.)
                .show_ui(ui, |ui| {
                    for pitch in [0.05, 0.1, 0.25, 0.5, 1., 2.54] {
                        ui.selectable_value(&mut e.pitch, pitch, format!("{pitch} mm"));
                    }
                });
            ui.checkbox(&mut e.snap, "Snap");
            if view.width() > 320. {
                ui.separator();
                ui.label(e.active.title(&e.rev.board));
            }
            if view.width() > 520.
                && let Some(pt) = e.cursor
            {
                ui.separator();
                ui.weak(format!("{:.2}, {:.2} mm", mm(pt[0]), mm(pt[1])));
            }
        });
        if right > 0. {
            let area = Rect::from_min_max(pos2(view.right(), top), pos2(r.width(), bottom));
            let mut u = pane(ui, "pcb-inspector", area);
            ui.painter()
                .vline(area.left(), area.y_range(), Stroke::new(1., t.line));
            u.horizontal_wrapped(|ui| {
                for (name, panel) in [
                    ("Properties", Inspector::Properties),
                    ("Stackup", Inspector::Stack),
                    ("DRC", Inspector::Checks),
                ] {
                    let resp = ui.selectable_label(e.panel == panel, name);
                    self.places.push((format!("pcb:panel:{name}"), resp.rect));
                    if resp.clicked() {
                        e.panel = panel
                    }
                }
                if narrow && ui.button("Close").clicked() {
                    e.compact_panel = false;
                }
            });
            ScrollArea::vertical()
                .id_salt("pcb-properties")
                .show(&mut u, |ui| e.inspector(ui, &t, &mut self.places));
        }
        let status = if !e.message.is_empty() {
            e.message.clone()
        } else if e.mode == Command::Move {
            "Click the new footprint position".into()
        } else if e.mode == Command::Route {
            format!("Route track · {} points · Enter to finish", e.draft.len())
        } else {
            format!(
                "{} nets   ·   {} footprints   ·   {} DRC findings",
                e.rev.board.nets.len(),
                e.rev.board.layout.placements.len(),
                e.rev.checks.len()
            )
        };
        self.state_status(&ctx, status);
        self.pcb = Some(e);
        self.docs_and_status(ui, r, bottom);
        self.resize_edges(ui, r);
        self.menus(&ctx);
        self.settings_window(&ctx);
    }
}
