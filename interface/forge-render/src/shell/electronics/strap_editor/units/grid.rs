//! The unit catalog shares the truth table's surface and typography.
use super::*;
use crate::shell::theme;
use egui::{Align2, Sense, Stroke, StrokeKind};

fn number(text: &str) -> String {
    let Ok(n) = text.parse::<i64>() else {
        return text.into();
    };
    let mut s = String::new();
    for (i, c) in n.unsigned_abs().to_string().chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            s.push(',');
        }
        s.push(c);
    }
    if n < 0 {
        s.push('-');
    }
    s.chars().rev().collect()
}
fn bounds(row: &UnitRow) -> String {
    match (row.min.as_str(), row.max.as_str()) {
        ("", "") => "Unbounded".into(),
        (lo, "") => format!("≥ {}", number(lo)),
        ("", hi) => format!("≤ {}", number(hi)),
        (lo, hi) => format!("{} – {}", number(lo), number(hi)),
    }
}
fn edit(ui: &mut Ui, label: &str, value: &mut String, width: f32) {
    ui.label(label);
    ui.add_sized(vec2(width, 25.), egui::TextEdit::singleline(value));
    ui.end_row();
}
fn cell_editor(ui: &mut Ui, row: &mut UnitRow, col: usize, base: bool) {
    ui.spacing_mut().item_spacing = vec2(8., 6.);
    ui.set_min_width(190.);
    egui::Grid::new("unit-cell-editor")
        .spacing(vec2(12., 6.))
        .show(ui, |ui| match col {
            0 => edit(ui, "Name", &mut row.name, 112.),
            1 => {
                if let Some(r) = &mut row.radix {
                    edit(ui, "Base", &mut r[0], 74.);
                    edit(ui, "Prefix", &mut r[1], 74.);
                    edit(ui, "Minimum digits", &mut r[2], 74.);
                } else if base {
                    ui.label("Base unit");
                    ui.label("× 1");
                    ui.end_row();
                } else {
                    edit(ui, "Factor to base", &mut row.factor, 112.);
                }
            }
            _ => {
                edit(ui, "Minimum", &mut row.min, 100.);
                edit(ui, "Maximum", &mut row.max, 100.);
            }
        });
    if ui.button("Done").clicked() {
        ui.close();
    }
}

pub(super) fn show(
    ui: &mut Ui,
    t: &Theme,
    rows: &mut Vec<UnitRow>,
    editable: bool,
    kind: StrapKind,
    a: &Attribute,
    format: &str,
) {
    let encoded = rows.first().is_some_and(|r| r.radix.is_some());
    let lookup = kind == StrapKind::Lookup;
    let categorical = matches!(kind, StrapKind::Text | StrapKind::Choice | StrapKind::BootMode);
    let defs = rows
        .iter()
        .map(UnitRow::parse)
        .collect::<Result<Vec<_>>>()
        .ok()
        .filter(|u| sch::validate_units(u).is_ok());
    let sample = a
        .default
        .as_deref()
        .or_else(|| a.rows.iter().map(|(_, v)| v.as_str()).find(|v| !v.is_empty()));
    let base = rows.first().map(|r| r.name.as_str()).unwrap_or("");
    let conversion = if encoded {
        "Encoding".into()
    } else if !lookup && !categorical && !base.is_empty() {
        format!("To {base}")
    } else {
        "Conversion".into()
    };
    let headers = [
        if encoded || lookup { "Format" } else { "Unit" },
        &conversion,
        "Example",
        if categorical || lookup {
            "Valid values"
        } else if encoded {
            "Range (decimal)"
        } else {
            "Valid range"
        },
    ];
    let width = ui.available_width();
    let h = 34.;
    let (grid, _) = ui.allocate_exact_size(vec2(width, h * (1 + rows.len()) as f32), Sense::hover());
    let cell = |r: usize, c: usize| {
        let x = [0., 0.20, 0.44, 0.70, 1.];
        Rect::from_min_size(
            grid.min + vec2(width * x[c], h * r as f32),
            vec2(width * (x[c + 1] - x[c]), h),
        )
    };
    ui.painter().rect_filled(grid, 5., t.bg);
    ui.painter()
        .rect_filled(Rect::from_min_size(grid.min, vec2(width, h)), 4., t.raised);
    ui.painter()
        .rect_stroke(grid, 5., Stroke::new(1., t.line), StrokeKind::Inside);
    for (i, title) in headers.iter().enumerate() {
        let r = cell(0, i);
        ui.painter().text(
            r.left_center() + vec2(10., 0.),
            Align2::LEFT_CENTER,
            title,
            theme::semi(13.),
            t.ink2,
        );
    }
    let mut remove = None;
    for (index, row) in rows.iter_mut().enumerate() {
        let conversion = if let Some(codec) = &row.radix {
            format!("Base {}", codec[0])
        } else if kind == StrapKind::Lookup {
            "Table lookup".into()
        } else if categorical {
            "Identity".into()
        } else {
            format!("× {}", number(&row.factor))
        };
        let example = defs
            .as_ref()
            .and_then(|u| a.conversions.convert(kind, sample?, u, Some(format), &row.name))
            .unwrap_or_else(|| "—".into());
        let range = if kind == StrapKind::Lookup {
            format!("{} values", a.conversions.value_count())
        } else if categorical {
            if a.modes.is_empty() {
                "Any text".into()
            } else {
                a.modes.join(", ")
            }
        } else if encoded {
            let lo = if row.min.is_empty() { "0" } else { &row.min };
            let hi = if row.max.is_empty() {
                "4294967295"
            } else {
                &row.max
            };
            format!("{} – {}", number(lo), number(hi))
        } else {
            bounds(row)
        };
        let values = [
            if row.name.is_empty() {
                "Name unit…".into()
            } else {
                row.name.clone()
            },
            conversion,
            example,
            range,
        ];
        let y = cell(index + 1, 0).top();
        ui.painter().line_segment(
            [egui::pos2(grid.left(), y), egui::pos2(grid.right(), y)],
            Stroke::new(1., t.line),
        );
        for (col, value) in values.iter().enumerate() {
            let r = cell(index + 1, col).shrink(1.);
            let active = editable && col != 2 && (kind != StrapKind::Lookup || col == 0);
            let response = ui.interact(
                r,
                ui.scope_id().with(("unit", index, col)),
                if active { Sense::click() } else { Sense::hover() },
            );
            if active && response.hovered() {
                ui.painter().rect_filled(r, 2., t.hover);
            }
            let ink = if col == 0 {
                t.on_ink
            } else if col == 1 {
                t.ink2
            } else {
                t.ink
            };
            let font = if col == 0 {
                theme::semi(14.)
            } else if col == 2 {
                theme::mono(13.)
            } else {
                theme::medium(13.)
            };
            let paint = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
            let text = paint.layout_no_wrap(value.clone(), font, ink);
            let clipped = text.size().x > r.width() - 18.;
            paint.galley(r.left_center() + vec2(9., -text.size().y / 2.), text, ink);
            if clipped {
                response.clone().on_hover_text(value);
            }
            if active {
                egui::Popup::menu(&response)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| cell_editor(ui, row, col, index == 0));
                if index > 0 {
                    egui::Popup::context_menu(&response)
                        .id(response.id.with("actions"))
                        .show(|ui| {
                            if ui.button("Remove unit").clicked() {
                                remove = Some(index);
                                ui.close();
                            }
                        });
                }
            }
        }
    }
    if let Some(i) = remove {
        rows.remove(i);
    }
    if editable
        && ui
            .small_button(if encoded || lookup {
                "+  Add format"
            } else {
                "+  Add unit"
            })
            .clicked()
    {
        let mut new = UnitRow::blank(kind == StrapKind::Lookup);
        if encoded {
            new.factor = "1".into();
            new.radix = Some(["16".into(), "0x".into(), "0".into()]);
        }
        rows.push(new);
    }
}
