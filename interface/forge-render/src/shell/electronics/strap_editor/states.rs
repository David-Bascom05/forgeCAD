//! Electrical input states share no value editor with the attribute result.
use super::*;
use sch::{StrapCondition as Condition, VoltageRange};

impl Input {
    pub(super) fn display(&self, key: &str) -> String {
        match self.states.iter().find(|(id, _)| id == key).map(|(_, c)| c) {
            Some(Condition::Voltage(v)) => v.to_string(),
            Some(Condition::Floating) => "Floating".into(),
            _ if key.eq_ignore_ascii_case("high") => "High".into(),
            _ if key.eq_ignore_ascii_case("low") => "Low".into(),
            _ if key.is_empty() => "Select state".into(),
            _ => key.into(),
        }
    }
    fn reference(&self, level: &str, pins: &[String]) -> Option<String> {
        if let Some((_, Condition::Reference(pin))) =
            self.states.iter().find(|(id, _)| id.eq_ignore_ascii_case(level))
        {
            return Some(pin.clone());
        }
        let names: &[&str] = if level == "high" {
            &["VDD", "VCC"]
        } else {
            &["VSS", "GND"]
        };
        names.iter().find_map(|name| {
            pins.iter()
                .find(|p| **p != self.pin && p.eq_ignore_ascii_case(name))
                .cloned()
        })
    }
}

pub(super) struct Selection {
    pub row: usize,
    pub col: usize,
    pub key: String,
    pub condition: Condition,
}

impl Selection {
    pub(super) fn apply(self, a: &mut Attribute) {
        let input = &mut a.inputs[self.col];
        let previous = std::mem::take(&mut a.rows[self.row].0[self.col]);
        if !a.rows.iter().any(|(cells, _)| cells[self.col] == previous) {
            input
                .states
                .retain(|(key, c)| key != &previous || !matches!(c, Condition::Voltage(_)));
        }
        let id = if let Some((id, _)) = input.states.iter().find(|(_, v)| *v == self.condition) {
            id.clone()
        } else {
            let mut id = self.key.clone();
            let mut i = 2;
            while input.states.iter().any(|(s, _)| *s == id) {
                id = format!("{}_{i}", self.key);
                i += 1;
            }
            input.states.push((id.clone(), self.condition));
            id
        };
        a.rows[self.row].0[self.col] = id;
    }
}

enum Page {
    Choices,
    Voltage([String; 2]),
    Reference(&'static str),
}
pub(super) struct StateMenu {
    cell: (usize, usize),
    page: Page,
}
impl StateMenu {
    pub(super) fn new(row: usize, col: usize) -> Self {
        Self {
            cell: (row, col),
            page: Page::Choices,
        }
    }
    pub(super) fn show(
        &mut self,
        ui: &mut Ui,
        row: usize,
        col: usize,
        input: &Input,
        key: &str,
        pins: &[String],
    ) -> Option<Selection> {
        if self.cell != (row, col) {
            *self = Self::new(row, col);
        }
        let mut choice = None;
        let current = input.states.iter().find(|(s, _)| s == key).map(|(_, v)| v);
        ui.set_min_width(165.);
        match &mut self.page {
            Page::Choices => {
                for (id, label) in [("high", "High"), ("low", "Low")] {
                    if ui.selectable_label(key.eq_ignore_ascii_case(id), label).clicked() {
                        if let Some(pin) = input.reference(id, pins) {
                            choice = Some((id.into(), Condition::Reference(pin)));
                        } else {
                            self.page = Page::Reference(id);
                        }
                    }
                }
                if ui
                    .selectable_label(matches!(current, Some(Condition::Floating)), "Floating")
                    .clicked()
                {
                    choice = Some(("floating".into(), Condition::Floating));
                }
                if ui
                    .selectable_label(matches!(current, Some(Condition::Voltage(_))), "Voltage range…")
                    .clicked()
                {
                    let limits = match current {
                        Some(Condition::Voltage(v)) => [v.min.to_string(), v.max.to_string()],
                        _ => Default::default(),
                    };
                    self.page = Page::Voltage(limits);
                }
                for (id, condition) in &input.states {
                    if matches!(condition, Condition::Reference(_))
                        && !id.eq_ignore_ascii_case("high")
                        && !id.eq_ignore_ascii_case("low")
                        && ui.selectable_label(id == key, id).clicked()
                    {
                        choice = Some((id.clone(), condition.clone()));
                    }
                }
            }
            Page::Reference(level) => {
                ui.strong(format!("Reference pin for {level}"));
                for pin in pins.iter().filter(|id| **id != input.pin) {
                    if ui.button(pin).clicked() {
                        choice = Some(((*level).into(), Condition::Reference(pin.clone())));
                    }
                }
                if ui.small_button("Back").clicked() {
                    self.page = Page::Choices;
                }
            }
            Page::Voltage(bounds) => {
                ui.set_min_width(205.);
                ui.spacing_mut().item_spacing = vec2(8., 6.);
                ui.spacing_mut().button_padding = vec2(8., 4.);
                ui.add_space(3.);
                ui.strong("Voltage range");
                egui::Grid::new(("voltage-limits", row, col))
                    .spacing(vec2(10., 6.))
                    .show(ui, |ui| {
                        for (label, text) in ["Minimum", "Maximum"].into_iter().zip(bounds.iter_mut()) {
                            ui.label(label);
                            ui.add_sized(
                                vec2(78., 24.),
                                egui::TextEdit::singleline(text).desired_width(78.),
                            );
                            ui.label("V");
                            ui.end_row();
                        }
                    });
                let values = bounds[0]
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .zip(bounds[1].trim().parse::<f64>().ok())
                    .filter(|(lo, hi)| lo.is_finite() && hi.is_finite() && lo <= hi);
                if values.is_none() && bounds.iter().all(|s| !s.trim().is_empty()) {
                    ui.label("Enter finite limits in ascending order.");
                }
                let back = ui
                    .horizontal(|ui| {
                        let back = ui.small_button("Back").clicked();
                        if ui
                            .add_enabled(values.is_some(), egui::Button::new("Use range"))
                            .clicked()
                            && let Some((min, max)) = values
                        {
                            choice = Some(("window".into(), Condition::Voltage(VoltageRange { min, max })));
                        }
                        back
                    })
                    .inner;
                if back {
                    self.page = Page::Choices;
                }
            }
        }
        choice.map(|(key, condition)| {
            ui.close();
            Selection {
                row,
                col,
                key,
                condition,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_one_window_preserves_other_cells_and_round_trips() {
        let src = r#"symbol_library
Chip=symbol(pins:{A:pin(number:"1",kind:"input",at:(0,0)),B:pin(number:"2",kind:"input",at:(2,0))},
 strap_tables:{mode:strap_table(kind:"text",inputs:{A:{window:[1.7,1.9]},B:{off:"open",on:A}},
 rows:[[{A:window,B:off},"Normal"],[{A:window,B:on},"Boot"]])})"#;
        let mut editor = Editor::open(None, src.into()).unwrap();
        editor.inspect();
        let a = &mut editor.form.straps[0];
        Selection {
            row: 0,
            col: 0,
            key: "window".into(),
            condition: Condition::Voltage(VoltageRange { min: 0.5, max: 1. }),
        }
        .apply(a);
        assert_ne!(a.rows[0].0[0], a.rows[1].0[0]);
        assert_eq!(a.inputs[0].display(&a.rows[1].0[0]), "1.7–1.9 V");
        assert_eq!(a.inputs[1].display("off"), "Floating");
        editor.save_properties().unwrap();
        let reopened = Editor::open(None, editor.src.clone()).unwrap();
        let a = &load(&reopened.doc.symbols["Chip"])[0];
        assert_eq!(a.inputs[0].display(&a.rows[0].0[0]), "0.5–1 V");
        assert_eq!(a.rows[1].1, "Boot");
        editor.step(false);
        assert_eq!(editor.src, src);
    }
}
