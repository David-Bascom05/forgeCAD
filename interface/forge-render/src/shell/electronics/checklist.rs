//! Inspect and edit the typed assignments behind a pin's checklist.
use super::*;
use crate::shell::theme;
use egui::{Align, Layout, RichText};

enum Action {
    Visit(String),
    Assign(String, bool),
    Manage(sch::RoleId, bool),
    Remove(String),
}

impl Editor {
    pub(super) fn checklist(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let focus = self.focus_pin.as_ref().unwrap();
        let pin = &self.circuit.terminals[&focus.id];
        let definition = &self.doc.symbols[&pin.symbol].pins[&pin.pin];
        ui.add_space(theme::S2);
        let name = focus.id.rsplit('/').next().unwrap_or(&focus.id);
        let heading = ui.label(RichText::new(name).font(theme::heading()).color(t.ink));
        places.push((format!("electronics:inspection:{}", focus.id), heading.rect));
        ui.label(
            RichText::new(format!(
                "Pin {}  ·  {}",
                pin.number,
                pin.electrical.word().replace('_', " ")
            ))
            .font(theme::small())
            .color(t.ink2),
        );
        if let Some(line) = definition.description.lines().next() {
            ui.add_space(theme::S1);
            ui.label(line).on_hover_text(&definition.description);
        }

        ui.add_space(theme::S3);
        ui.label(RichText::new("Pin checklist").font(theme::semi(16.)).color(t.ink));
        ui.separator();
        let mut action = None;
        for (need, group) in [(true, "Wants"), (false, "Provides")] {
            let checks = || focus.inspection.checks.iter().filter(|c| c.need == need);
            let total = checks().count();
            let satisfied = checks()
                .filter(|c| c.peers.iter().any(|(_, s)| *s == sch::Evidence::Verified))
                .count();
            ui.add_space(theme::S2);
            ui.horizontal(|ui| {
                ui.label(RichText::new(group).font(theme::title()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let count = if total == 0 {
                        "None declared".into()
                    } else {
                        format!("{satisfied} of {total} verified")
                    };
                    ui.label(RichText::new(count).font(theme::small()).color(t.ink2));
                });
            });
            for check in checks() {
                let role = self
                    .doc
                    .role_names
                    .iter()
                    .find(|(_, id)| **id == check.role)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or(&check.role.0);
                let spec = &self.doc.roles[&check.role];
                let label = role.rsplit('.').next().unwrap_or(role).replace('_', " ");
                let mut letters = label.chars();
                let mut title = letters
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>())
                    .unwrap_or_default()
                    + letters.as_str();
                match spec.rating {
                    Some(sch::Rating::VoltageMv(mv)) => title.push_str(&format!(" · {} V", mv / 1000.)),
                    Some(sch::Rating::CapacitanceNf(nf)) => title.push_str(&format!(" · {nf} nF")),
                    None => {}
                }
                let verified = check.peers.iter().any(|(_, e)| *e == sch::Evidence::Verified);
                let invalid = check.peers.iter().any(|(_, e)| *e == sch::Evidence::Invalid);
                ui.add_space(theme::S1);
                egui::Frame::new()
                    .fill(t.raised)
                    .stroke(egui::Stroke::new(1., t.line))
                    .corner_radius(theme::R1)
                    .inner_margin(theme::S2 as i8)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        let mut marked = verified;
                        let toggle = ui.add(
                            egui::Checkbox::new(&mut marked, RichText::new(title).font(theme::title()))
                                .indeterminate(invalid && !verified),
                        );
                        places.push((
                            format!("electronics:check:{}:{}", focus.id, check.role.0),
                            toggle.rect,
                        ));
                        if toggle
                            .on_hover_text(if invalid || verified {
                                "Manage this assignment"
                            } else if need {
                                "Assign a provider"
                            } else {
                                "Assign a consumer"
                            })
                            .changed()
                        {
                            action = Some(if verified || invalid {
                                Action::Manage(check.role.clone(), need)
                            } else {
                                Action::Assign(role.into(), need)
                            });
                        }
                        ui.indent((&focus.id, &check.role.0, need), |ui| {
                            if need && matches!(spec.kind, sch::RoleKind::Power | sch::RoleKind::Return) {
                                for (rail, pin) in &focus.inspection.rails {
                                    let label = ui.label(format!("{rail} connected"));
                                    places.push((format!("electronics:rail:{pin}"), label.rect));
                                }
                            }
                            for (peer, evidence) in &check.peers {
                                let (scope, short) = peer.rsplit_once('/').unwrap_or(("", peer));
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .link(if scope == self.subject { short } else { peer })
                                        .clicked()
                                    {
                                        action = Some(Action::Visit(peer.clone()));
                                    }
                                    let (caption, color) = match evidence {
                                        sch::Evidence::Verified => ("Verified", t.success),
                                        sch::Evidence::Invalid => ("Check wiring", t.error),
                                        sch::Evidence::Candidate => ("Candidate", t.ink2),
                                    };
                                    ui.colored_label(color, caption);
                                });
                            }
                            if check.peers.is_empty() {
                                let text = match (need, spec.kind) {
                                    (true, sch::RoleKind::Power) => "No power source assigned",
                                    (true, _) => "No provider assigned",
                                    _ => "No consumer assigned",
                                };
                                ui.colored_label(t.warn, text);
                            }
                            if focus
                                .manage
                                .as_ref()
                                .is_some_and(|(r, n)| r == &check.role && *n == need)
                            {
                                for index in self.circuit.relations.get(&focus.id).into_iter().flatten() {
                                    let link = &self.circuit.links[*index];
                                    let endpoint = if need { &link.want } else { &link.give };
                                    if endpoint == &focus.id
                                        && link.role == check.role
                                        && let Some((scope, id)) = link.id.rsplit_once('/')
                                        && scope == self.subject
                                    {
                                        let response = ui.button(format!("Remove assignment · {id}"));
                                        places.push((
                                            format!("electronics:unassign:{}", link.id),
                                            response.rect,
                                        ));
                                        if response.clicked() {
                                            action = Some(Action::Remove(id.into()));
                                        }
                                    }
                                }
                            }
                        });
                    });
            }
        }
        ui.add_space(theme::S3);
        if ui.button("Close pin inspection").clicked() {
            self.start(Tool::Select);
            return;
        }
        match action {
            Some(Action::Visit(id)) => {
                if let Some((scope, _)) = id.rsplit_once('/')
                    && scope != self.subject
                {
                    self.focus_pin = None;
                    self.subject = scope.into();
                    self.refresh();
                }
                self.focus_on(&id);
            }
            Some(Action::Assign(role, need)) => {
                let pin = focus.id.clone();
                let anchor = name.to_owned();
                self.start(Tool::Intent);
                // Keep the highlighted peer at the same screen position for the next pick.
                self.fit = false;
                self.form.role = role;
                self.form.checklist_pin = Some((pin, need));
                self.first = Some(anchor);
            }
            Some(Action::Manage(role, need)) => {
                let selected = Some((role, need));
                let focus = self.focus_pin.as_mut().unwrap();
                focus.manage = if focus.manage == selected { None } else { selected };
            }
            Some(Action::Remove(id)) => {
                if let Err(err) = self.apply(vec![Op::DeleteFeature { id }]) {
                    self.message = err;
                }
            }
            None => {}
        }
    }
}
