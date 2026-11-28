//! Inspect and edit the typed assignments behind a pin's checklist.
use super::*;
use crate::shell::theme;
use egui::{Align, Layout, RichText};

enum Action {
    Visit(String),
    Assign(String, bool),
    Manage(sch::RoleId, bool),
    Remove(String),
    SetRole {
        role: sch::RoleId,
        want: bool,
        enabled: bool,
    },
    NewRole,
}

impl Editor {
    pub(super) fn set_placement_role(
        &mut self,
        terminal: &str,
        role: &sch::RoleId,
        want: bool,
        enabled: bool,
    ) -> Result<()> {
        let (owner, pin) = terminal
            .rsplit_once('/')
            .and_then(|(_, name)| name.split_once('.'))
            .ok_or("Select a placed pin.")?;
        let Some(Item::Place { symbol, pins, .. }) =
            self.doc.rows.iter().find(|r| r.id == owner).map(|r| &r.item)
        else {
            return Err("Select a component placement.".into());
        };
        let wants = &pins.wants;
        let gives = &pins.gives;
        let field = if want { "wants" } else { "gives" };
        let mut roles = (if want { wants } else { gives })
            .get(pin)
            .cloned()
            .unwrap_or_default();
        if enabled {
            roles.insert(role.clone());
        } else {
            roles.remove(role);
        }
        let mut names = Vec::new();
        for role in roles {
            let (name, _) = self
                .doc
                .role_names
                .iter()
                .find(|(_, id)| **id == role)
                .ok_or("Unknown role")?;
            names.push(name.as_str());
        }
        let list = format!("[{}]", names.join(", "));
        let tree = syntax::parse(&self.src).map_err(|e| e.text())?;
        let value = if let Some(expr) = source_arg(&tree, owner, field) {
            let ExprKind::Map(members) = &expr.kind else {
                return Err("Expected a pin map.".into());
            };
            let start = expr.span.lo as usize;
            let mut text = self.src[start..expr.span.hi as usize].to_owned();
            let aliases = &self.doc.symbols[symbol].pins[pin].aliases;
            if let Some((_, old)) = members
                .iter()
                .find(|(name, _)| name == pin || aliases.contains(name))
            {
                text.replace_range(old.span.lo as usize - start..old.span.hi as usize - start, &list);
            } else {
                let separator = if members.is_empty() { "" } else { ", " };
                // Inserting at the front also accepts a trailing comma on the original map.
                text.insert_str(1, &format!("{pin}: {list}{separator}"));
            }
            text
        } else {
            format!("{{{pin}: {list}}}")
        };
        self.apply(vec![arg(owner, field, value)])
    }

    pub(super) fn checklist(&mut self, ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
        self.variant_picker(ui);
        let Some(focus) = &self.focus_pin else { return };
        let checked = self.variant_checks.as_ref().map_or(&self.circuit, |(_, c)| c);
        let pin = &checked.terminals[&focus.id];
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
        let symbol = &self.doc.symbols[&pin.symbol];
        let rows = definition
            .specs
            .iter()
            .chain(
                pin.function
                    .as_ref()
                    .and_then(|k| definition.functions.get(k))
                    .into_iter()
                    .flat_map(|f| &f.specs),
            )
            .chain(symbol.specs.iter().filter(|s| s.value.pins.contains(&pin.pin)));
        if rows.clone().next().is_some() {
            ui.collapsing("Datasheet characteristics", |ui| {
                for spec in rows {
                    ui.strong(spec.value.kind.replace('_', " "));
                    ui.label(super::spec_editor::summary(&spec.value));
                    ui.weak(spec.class.replace('_', " "));
                    for c in &spec.conditions {
                        ui.label(format!(
                            "When {} · {}",
                            c.kind.replace('_', " "),
                            super::spec_editor::summary(c)
                        ));
                    }
                    if !spec.source.is_empty() {
                        ui.small(&spec.source);
                    }
                    if !spec.note.is_empty() {
                        ui.small(&spec.note);
                    }
                }
            });
        }
        ui.label(RichText::new("Pin checklist").font(theme::semi(16.)).color(t.ink));
        ui.separator();
        let mut action = None;
        for (need, group) in [(true, "Wants"), (false, "Provides")] {
            let checks = || focus.inspection.checks.iter().filter(|c| c.need == need);
            let total = checks().count();
            let satisfied = checks()
                .filter(|c| {
                    c.peers.iter().any(|(_, s)| *s == sch::Evidence::Verified)
                        && !c.peers.iter().any(|(_, s)| *s == sch::Evidence::Invalid)
                })
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
                    Some(sch::Rating::VoltageRange(range)) => title.push_str(&format!(" · {range}")),
                    Some(sch::Rating::VoltageMv(mv)) => title.push_str(&format!(" · {} V", mv / 1000.)),
                    Some(sch::Rating::CapacitanceNf(nf)) => title.push_str(&format!(" · {nf} nF")),
                    None => {}
                }
                let voltage = need
                    .then(|| checked.voltages.get(&focus.id)?.get(&check.role))
                    .flatten();
                let connection = need
                    .then(|| checked.connections.get(&focus.id)?.get(&check.role))
                    .flatten();
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
                        let mut marked = verified && !invalid;
                        let toggle = ui.add_enabled(
                            !check.measured,
                            egui::Checkbox::new(&mut marked, RichText::new(title).font(theme::title()))
                                .indeterminate(invalid),
                        );
                        places.push((
                            format!("electronics:check:{}:{}", focus.id, check.role.0),
                            toggle.rect,
                        ));
                        if toggle
                            .on_hover_text(if connection.is_some() {
                                "Checked from the actual wiring and component value"
                            } else if check.measured {
                                "Checked from the voltages provided on this net"
                            } else if invalid || verified {
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
                            if let Some(path) = connection {
                                let r = ui.label(&path.detail);
                                places.push((
                                    format!(
                                        "electronics:connection:{}:{}:{}",
                                        focus.id, check.role.0, path.satisfied
                                    ),
                                    r.rect,
                                ));
                            }
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
                                    if let Some(check) = voltage {
                                        for source in
                                            check.providers.iter().filter(|source| &source.pin == peer)
                                        {
                                            let offered = source.range;
                                            let fits = check.required.accepts(offered);
                                            let text = if fits { "in range" } else { "out of range" };
                                            let label = ui.colored_label(
                                                if fits { t.success } else { t.error },
                                                format!("{offered} · {text}"),
                                            );
                                            places.push((
                                                format!("electronics:voltage:{}:{peer}:{fits}", focus.id),
                                                label.rect,
                                            ));
                                        }
                                    } else {
                                        ui.colored_label(color, caption);
                                    }
                                });
                            }
                            if check.peers.is_empty() && connection.is_none() {
                                let text = match (need, spec.kind) {
                                    _ if voltage.is_some() => "No connected pin provides a known voltage",
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
        let placement = ui.collapsing("Placement roles", |ui| {
            if self
                .circuit
                .occurrences
                .values()
                .filter(|s| s.as_str() == self.sheet())
                .count()
                > 1
            {
                ui.weak("Applies to this placement in every instance of this sheet.");
            }
            egui::Grid::new("placement-pin-roles").show(ui, |ui| {
                ui.strong("Role");
                ui.strong("Wants");
                ui.strong("Provides");
                ui.end_row();
                let mut seen = std::collections::BTreeSet::new();
                for (label, role) in &self.doc.role_names {
                    if !seen.insert(role) {
                        continue;
                    }
                    ui.label(label);
                    for (want, defaults, effective) in [
                        (true, &definition.wants, &pin.wants),
                        (false, &definition.gives, &pin.gives),
                    ] {
                        let inherited = defaults.contains(role);
                        let mut enabled = effective.contains(role);
                        let response = ui
                            .add_enabled(
                                !inherited && (want || self.doc.roles[role].connection.is_none()),
                                egui::Checkbox::without_text(&mut enabled),
                            )
                            .on_hover_text(if inherited {
                                "Defined on the library pin"
                            } else {
                                "Set on this placement"
                            });
                        places.push((
                            format!("electronics:placement-role:{want}:{label}"),
                            response.rect,
                        ));
                        if response.changed() {
                            action = Some(Action::SetRole {
                                role: role.clone(),
                                want,
                                enabled,
                            });
                        }
                    }
                    ui.end_row();
                }
            });
            if ui.button("New role…").clicked() {
                action = Some(Action::NewRole);
            }
        });
        places.push((
            "electronics:placement-roles".into(),
            placement.header_response.rect,
        ));
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
            Some(Action::SetRole { role, want, enabled }) => {
                let terminal = focus.id.clone();
                if let Err(e) = self.set_placement_role(&terminal, &role, want, enabled) {
                    self.message = e;
                }
            }
            Some(Action::NewRole) => self.start(Tool::Role),
            None => {}
        }
    }
}
