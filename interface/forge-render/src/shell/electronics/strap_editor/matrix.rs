//! The truth table paints ordinary cells. Only the selected cell mounts an editor.
use super::*;
use crate::shell::theme;
use egui::{Align2, Sense, Stroke, StrokeKind};

const HEADER: f32 = 36.;
const ROW: f32 = 38.;
const EDGE: f32 = 21.;

#[derive(Default)]
pub(super) struct Matrix {
    editing: Option<(usize, String, bool)>,
    growth: usize,
    state_menu: Option<states::StateMenu>,
}

fn line(ui: &Ui, a: egui::Pos2, b: egui::Pos2, color: egui::Color32) {
    ui.painter().line_segment([a, b], Stroke::new(1., color));
}

fn edge(ui: &mut Ui, r: Rect, columns: bool, can_grow: usize, growth: &mut usize, t: &Theme) -> usize {
    if can_grow == 0 {
        return 0;
    }
    let response = ui
        .interact(r, ui.scope_id().with(("grow", columns)), Sense::click_and_drag())
        .on_hover_cursor(if columns {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        });
    let noun = if columns { "columns" } else { "rows" };
    if let Some(delta) = response.total_drag_delta() {
        let d = if columns { delta.x } else { delta.y };
        *growth = (1 + (d.max(0.) / if columns { 68. } else { ROW }) as usize).min(can_grow);
    }
    if response.hovered() || response.dragged() || response.has_focus() {
        ui.painter().rect_filled(r.shrink(2.), 4., t.raised);
        let label = if response.dragged() {
            format!("+{}", *growth)
        } else {
            "+".into()
        };
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            theme::semi(15.),
            t.accent,
        );
    }
    let count = if response.drag_stopped() {
        std::mem::take(growth).min(can_grow)
    } else if response.clicked() {
        1
    } else {
        0
    };
    response.on_hover_text(format!("Click to add one; drag to add more {noun}"));
    count
}

fn add_rows(a: &mut Attribute, n: usize) {
    if n == 0 {
        return;
    }
    // Generate only as far as the requested unused combinations, even for a wide table.
    let mut used: std::collections::BTreeSet<_> = a.rows.iter().map(|(v, _)| v.clone()).collect();
    let mut index = 0usize;
    for _ in 0..n {
        let next = loop {
            let mut cursor = index;
            let mut pattern = Vec::with_capacity(a.inputs.len());
            for input in a.inputs.iter().rev() {
                if input.states.is_empty() {
                    break;
                }
                let radix = input.states.len();
                pattern.push(input.states[cursor % radix].0.clone());
                cursor /= radix;
            }
            if cursor > 0 || pattern.len() != a.inputs.len() || index == usize::MAX {
                break vec![String::new(); a.inputs.len()];
            }
            index += 1;
            pattern.reverse();
            if used.insert(pattern.clone()) {
                break pattern;
            }
        };
        a.rows.push((next, String::new()));
    }
}

impl Matrix {
    pub(super) fn show(&mut self, ui: &mut Ui, a: &mut Attribute, pins: &[String], format: &str, t: &Theme) {
        let n = a.inputs.len();
        let room = ui.available_width() - EDGE;
        let mut widths: Vec<f32> = a
            .inputs
            .iter()
            .map(|input| {
                let chars = std::iter::once(input.pin.as_str())
                    .chain(input.states.iter().map(|(s, _)| s.as_str()))
                    .map(|s| s.chars().count())
                    .max()
                    .unwrap_or(0);
                (40. + 8. * chars as f32).clamp(96., 320.)
            })
            .collect();
        let longest = a.rows.iter().map(|(_, v)| v.chars().count()).max().unwrap_or(0);
        widths.push((40. + 8. * longest as f32).clamp(150., 420.));
        let spare = (room - widths.iter().sum::<f32>()).max(0.) / widths.len() as f32;
        let mut offsets = vec![0.];
        for width in &mut widths {
            *width += spare;
            offsets.push(offsets.last().unwrap() + *width);
        }
        let grid_width = offsets[n + 1];
        let height = HEADER + ROW * a.rows.len() as f32;
        let mut state_change = None;
        let mut remove_row = None;
        let mut remove_col = None;
        let mut add_r = 0;
        let mut add_c = 0;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Truth table").font(theme::semi(15.)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("Table  ···", |ui| {
                    if ui
                        .add_enabled(n < pins.len(), egui::Button::new("Add input column"))
                        .clicked()
                    {
                        add_c = 1;
                        ui.close();
                    }
                    if ui.add_enabled(n > 0, egui::Button::new("Add row")).clicked() {
                        add_r = 1;
                        ui.close();
                    }
                });
            });
        });
        egui::ScrollArea::both()
            .max_height((ui.ctx().content_rect().height() * 0.43).clamp(160., 440.))
            .id_salt("attribute-grid-x")
            .show(ui, |ui| {
                let (outer, _) =
                    ui.allocate_exact_size(vec2(grid_width + EDGE, height + EDGE), Sense::hover());
                let grid = Rect::from_min_size(outer.min, vec2(outer.width() - EDGE, height));
                ui.painter().rect_filled(grid, 5., t.bg);
                ui.painter().rect_filled(
                    Rect::from_min_size(grid.min, vec2(grid.width(), HEADER)),
                    4.,
                    t.raised,
                );
                ui.painter()
                    .rect_stroke(grid, 5., Stroke::new(1., t.line), StrokeKind::Inside);
                let cell = |col: usize, row: Option<usize>| {
                    let (y, h) = row.map_or((0., HEADER), |r| (HEADER + r as f32 * ROW, ROW));
                    Rect::from_min_size(grid.min + vec2(offsets[col], y), vec2(widths[col], h))
                };
                for col in 0..=n {
                    let r = cell(col, None);
                    if col > 0 {
                        line(ui, r.left_top(), egui::pos2(r.left(), grid.bottom()), t.line);
                    }
                    let title = if col == n { "Result" } else { &a.inputs[col].pin };
                    ui.painter().text(
                        r.left_center() + vec2(12., 0.),
                        Align2::LEFT_CENTER,
                        title,
                        theme::semi(14.),
                        t.ink,
                    );
                    if col < n {
                        let response = ui.interact(r, ui.scope_id().with(("col", col)), Sense::click());
                        if response.hovered() {
                            ui.painter().text(
                                r.right_center() - vec2(12., 0.),
                                Align2::RIGHT_CENTER,
                                "⌄",
                                theme::body(),
                                t.ink2,
                            );
                        }
                        egui::Popup::menu(&response).show(|ui| {
                            for pin in pins {
                                let taken = a
                                    .inputs
                                    .iter()
                                    .enumerate()
                                    .any(|(i, p)| i != col && p.pin == *pin);
                                if !taken && ui.selectable_label(a.inputs[col].pin == *pin, pin).clicked() {
                                    a.inputs[col].pin.clone_from(pin);
                                    ui.close();
                                }
                            }
                            ui.separator();
                            if ui.button("Remove column").clicked() {
                                remove_col = Some(col);
                                ui.close();
                            }
                        });
                    }
                }
                for (row, (values, result)) in a.rows.iter_mut().enumerate() {
                    let left = cell(0, Some(row));
                    if !ui.is_rect_visible(Rect::from_min_size(left.min, vec2(grid.width(), ROW))) {
                        continue;
                    }
                    line(ui, left.left_top(), egui::pos2(grid.right(), left.top()), t.line);
                    for (col, input) in a
                        .inputs
                        .iter()
                        .zip(values.iter())
                        .map(Some)
                        .chain(std::iter::once(None))
                        .enumerate()
                    {
                        let r = cell(col, Some(row)).shrink(1.);
                        let id = ui.scope_id().with(("cell", row, col));
                        if input.is_none() && self.editing.as_ref().is_some_and(|(i, _, _)| *i == row) {
                            let (_, buffer, first) = self.editing.as_mut().unwrap();
                            let edit = ui.put(
                                r.shrink2(vec2(8., 4.)),
                                egui::TextEdit::singleline(buffer)
                                    .id(id.with("edit"))
                                    .frame(egui::Frame::NONE),
                            );
                            if std::mem::take(first) {
                                edit.request_focus();
                            }
                            ui.painter()
                                .rect_stroke(r, 2., Stroke::new(1., t.accent), StrokeKind::Inside);
                            let escape =
                                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                            if escape || edit.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if !escape {
                                    *result = super::normalized(
                                        buffer,
                                        format,
                                        sch::StrapKind::parse(&a.kind).unwrap_or(sch::StrapKind::Text),
                                        &a.units,
                                    );
                                }
                                self.editing = None;
                            }
                            continue;
                        }
                        let response = ui.interact(r, id, Sense::click());
                        if response.hovered() || response.has_focus() {
                            ui.painter().rect_filled(r, 0., t.hover);
                        }

                        let paint = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
                        let at = r.left_center() + vec2(12., 0.);
                        if let Some((pin, state)) = input {
                            paint.text(
                                at,
                                Align2::LEFT_CENTER,
                                pin.display(state),
                                theme::medium(14.),
                                t.ink2,
                            );
                            let center = r.right_center() - vec2(14., 0.);
                            line(ui, center + vec2(-3., -1.5), center + vec2(0., 1.5), t.ink2);
                            line(ui, center + vec2(0., 1.5), center + vec2(3., -1.5), t.ink2);
                        } else {
                            paint.text(
                                at,
                                Align2::LEFT_CENTER,
                                if result.is_empty() { "—" } else { result.as_str() },
                                if format.is_empty() {
                                    theme::medium(14.)
                                } else {
                                    theme::mono(14.)
                                },
                                t.ink,
                            );
                        }
                        if input.is_some() || !a.modes.is_empty() {
                            egui::Popup::menu(&response)
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                .show(|ui| {
                                    if let Some((pin, state)) = input {
                                        if response.clicked() || self.state_menu.is_none() {
                                            self.state_menu = Some(states::StateMenu::new(row, col));
                                        }
                                        if let Some(change) = self
                                            .state_menu
                                            .as_mut()
                                            .unwrap()
                                            .show(ui, row, col, pin, state, pins)
                                        {
                                            state_change = Some(change);
                                        }
                                    } else {
                                        for option in &a.modes {
                                            if ui.selectable_label(result == option, option).clicked() {
                                                result.clone_from(option);
                                                ui.close();
                                            }
                                        }
                                    }
                                });
                        } else if response.clicked() {
                            self.editing = Some((row, result.clone(), true));
                        }
                        egui::Popup::context_menu(&response)
                            .id(id.with("row-actions"))
                            .show(|ui| {
                                if ui.button("Remove row").clicked() {
                                    remove_row = Some(row);
                                    ui.close();
                                }
                            });
                    }
                }
                let right = Rect::from_min_max(grid.right_top(), egui::pos2(outer.right(), grid.bottom()));
                let bottom = Rect::from_min_max(grid.left_bottom(), egui::pos2(grid.right(), outer.bottom()));
                add_c += edge(ui, right, true, pins.len().saturating_sub(n), &mut self.growth, t);
                if n > 0 {
                    add_r += edge(ui, bottom, false, usize::MAX, &mut self.growth, t);
                }
            });
        if let Some(change) = state_change {
            change.apply(a);
        }
        if let Some(i) = remove_row {
            a.rows.remove(i);
            self.editing = None;
        }
        if let Some(i) = remove_col {
            a.remove_column(i);
            self.editing = None;
        }
        for _ in 0..add_c {
            a.add_column(pins);
        }
        add_rows(a, add_r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, PointerButton, pos2};

    #[test]
    fn input_dropdown_opens_voltage_form_and_keeps_it_open_for_typing() {
        use egui::epaint::Shape;
        let ctx = egui::Context::default();
        ctx.set_fonts(theme::fonts());
        let mut matrix = Matrix::default();
        let mut a = Attribute {
            inputs: vec![Input {
                pin: "BOOT".into(),
                states: vec![("high".into(), sch::StrapCondition::Reference("VDD".into()))],
            }],
            rows: vec![(vec!["high".into()], "Flash".into())],
            ..Default::default()
        };
        let pins = ["BOOT".into(), "VDD".into()];
        let mut time = 0.;
        let mut frame = |events| {
            time += 0.1;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(650., 500.))),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| matrix.show(ui, &mut a, &pins, "", &theme::DARK),
            );
            out.textures_delta.clear();
            out.shapes
                .into_iter()
                .filter_map(|s| match s.shape {
                    Shape::Text(t) => Some((t.galley.text().to_owned(), t.visual_bounding_rect().center())),
                    _ => None,
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let mut click = |pos| {
            frame(vec![Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                frame(vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                }]);
            }
            frame(vec![])
        };
        // The first cell starts below the table toolbar and its pin header.
        let menu = click(pos2(50., 84.));
        for label in ["High", "Low", "Floating", "Voltage range…"] {
            assert!(
                menu.contains_key(label),
                "missing {label}, visible {:?}",
                menu.keys()
            );
        }
        let form = click(menu["Voltage range…"]);
        assert!(form.contains_key("Minimum") && form.contains_key("Maximum"));
        let field = form["Minimum"] + vec2(65., 0.);
        assert!(click(field).contains_key("Use range"));
    }
    #[test]
    fn edge_gestures_commit_once_on_release_and_clamp_to_available_pins() {
        for (columns, available, start, end, expected) in [
            (false, usize::MAX, pos2(140., 100.), pos2(140., 183.), 3),
            (true, 2, pos2(140., 100.), pos2(364., 100.), 2),
        ] {
            let ctx = egui::Context::default();
            ctx.set_fonts(theme::fonts());
            let mut growth = 0;
            let mut committed = 0;
            let mut time = 0.;
            let mut frame = |events| {
                time += 0.016;
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(640., 480.))),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        committed += edge(
                            ui,
                            Rect::from_min_size(pos2(100., 90.), vec2(100., 20.)),
                            columns,
                            available,
                            &mut growth,
                            &theme::DARK,
                        );
                    },
                );
                out.textures_delta.clear();
                committed
            };
            let button = |pos, pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            frame(vec![]);
            frame(vec![Event::PointerMoved(start)]);
            frame(vec![button(start, true)]);
            assert_eq!(frame(vec![Event::PointerMoved(end)]), 0);
            assert_eq!(frame(vec![button(end, false)]), expected);
            frame(vec![Event::PointerMoved(start)]);
            frame(vec![button(start, true)]);
            assert_eq!(frame(vec![button(start, false)]), expected + 1);
        }
    }
}
