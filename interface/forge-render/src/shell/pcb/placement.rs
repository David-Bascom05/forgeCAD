//! Place a library footprint using schematic pin mappings or explicit board net assignments.
use super::*;
use egui::Ui;
use forge_lang::props::quote;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Form {
    package: String,
    library: String,
    occurrence: String,
    reference: String,
    pads: BTreeMap<String, String>,
    bottom: bool,
}
impl Editor {
    pub(super) fn placement_form(&mut self, ui: &mut Ui) {
        if self.rev.board.layout.footprints.is_empty() {
            ui.label("Link a footprint library to place parts.");
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.place.library)
                    .desired_width(150.)
                    .hint_text("Library .fg path"),
            );
            if ui.button("Link").clicked() && !self.place.library.trim().is_empty() {
                if self.path.is_none() {
                    self.message = "Save the board before linking a relative library path".into();
                } else {
                    let id = self.next_id("lib");
                    let path = quote(self.place.library.trim());
                    if self.edit(vec![Op::AddFeature {
                        before: None,
                        text: format!("{id} = library(file: {path})"),
                    }]) {
                        self.place.library.clear();
                    }
                }
            }
        });
        if self.place.occurrence.is_empty() {
            self.add_net_ui(ui);
        }
        egui::Grid::new("pcb-place-form").show(ui, |ui| {
            ui.label("Footprint");
            egui::ComboBox::from_id_salt("pcb-package")
                .selected_text(if self.place.package.is_empty() {
                    "Choose footprint"
                } else {
                    &self.place.package
                })
                .show_ui(ui, |ui| {
                    for id in self.rev.board.layout.footprints.keys() {
                        if ui
                            .selectable_value(&mut self.place.package, id.clone(), id)
                            .changed()
                        {
                            self.place.pads.clear();
                        }
                    }
                });
            ui.end_row();
            if self.rev.board.schematic.is_some() {
                ui.label("Component");
                egui::ComboBox::from_id_salt("pcb-place-occ")
                    .selected_text(if self.place.occurrence.is_empty() {
                        "Board only"
                    } else {
                        &self.place.occurrence
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.place.occurrence, String::new(), "Board only");
                        for id in self.rev.board.unplaced() {
                            ui.selectable_value(&mut self.place.occurrence, id.clone(), id);
                        }
                    });
                ui.end_row();
            }
            if self.place.occurrence.is_empty() {
                ui.label("Reference");
                ui.add(egui::TextEdit::singleline(&mut self.place.reference).desired_width(104.));
                ui.end_row();
            }
            ui.label("Side");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.place.bottom, false, "Top");
                ui.selectable_value(&mut self.place.bottom, true, "Bottom");
            });
            ui.end_row();
            if let Some(package) = self.rev.board.layout.footprints.get(&self.place.package)
                && self.place.occurrence.is_empty()
            {
                for pad in &package.pads {
                    ui.label(format!("Pad {}", pad.number));
                    let net = self.place.pads.entry(pad.number.clone()).or_default();
                    egui::ComboBox::from_id_salt(("pcb-pad-net", &pad.number))
                        .selected_text(
                            self.rev
                                .board
                                .nets
                                .get(net)
                                .map(|n| n.name.as_str())
                                .unwrap_or("Choose net"),
                        )
                        .show_ui(ui, |ui| {
                            for (id, n) in self.rev.board.nets.iter().filter(|(id, _)| !id.starts_with('@')) {
                                ui.selectable_value(net, id.clone(), &n.name);
                            }
                        });
                    ui.end_row();
                }
            }
        });
    }
    pub(super) fn place_at(&mut self, at: Point) {
        let Some(package) = self.rev.board.layout.footprints.get(&self.place.package) else {
            self.message = "Choose a footprint first".into();
            return;
        };
        let binding = if self.place.occurrence.is_empty() {
            let reference = self.place.reference.trim();
            if reference.is_empty()
                || self
                    .rev
                    .board
                    .layout
                    .placements
                    .values()
                    .any(|p| p.reference == reference)
            {
                self.message = "Enter an unused reference designator".into();
                return;
            }
            let mut bindings = vec![];
            for pad in &package.pads {
                let Some(net) = self
                    .place
                    .pads
                    .get(&pad.number)
                    .filter(|id| self.rev.board.nets.contains_key(*id))
                else {
                    self.message = format!("Choose a net for pad {}", pad.number);
                    return;
                };
                bindings.push(format!("bind(pad: {}, net: {net})", quote(&pad.number)));
            }
            format!(
                "reference: {}, bindings: [{}]",
                quote(reference),
                bindings.join(", ")
            )
        } else {
            format!("occurrence: {}", quote(&self.place.occurrence))
        };
        let id = self.next_id("part");
        let side = if self.place.bottom { "bottom" } else { "top" };
        let text = format!(
            "{id} = place(footprint: {}, {binding}, at: {}, side: \"{side}\")",
            self.place.package,
            coord(at)
        );
        if self.edit(vec![Op::AddFeature { before: None, text }]) {
            self.selected = id;
            self.mode = Command::Select;
            self.place.reference.clear();
            self.place.occurrence.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_placement_creates_real_bound_pads_and_undo_restores_the_board() {
        let src = format!(
            "{NEW_BOARD}\nrail=net(name:\"VDD\")\npkg=footprint(pads:[smd(number:\"1\",at:(1,0),size:(1,1))])\n"
        );
        let mut e = Editor::open(None, src).unwrap();
        e.place.package = "pkg".into();
        e.place.reference = "TP1".into();
        e.place.bottom = true;
        e.place_at([5_000_000, 5_000_000]);
        assert!(!e.dirty()); // incomplete mapping is a refused edit
        e.place.pads.insert("1".into(), "rail".into());
        e.place_at([5_000_000, 5_000_000]);
        let pad = &e.rev.board.layout.pads["part1.1"];
        assert_eq!(
            (&*pad.net, pad.side, pad.at),
            ("rail", Side::Bottom, [4_000_000, 5_000_000])
        );
        e.step(false);
        assert!(e.rev.board.layout.pads.is_empty());
    }
}
