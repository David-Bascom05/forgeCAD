//! Pairwise conversion tables are named library declarations, editable in their owning document.
use super::*;
use sch::{ConversionGraph, ConversionTable};
pub(super) fn source(tables: &[ConversionTable]) -> String {
    let entries = tables
        .iter()
        .map(|t| {
            let mut fields = vec![
                format!("name: {}", quote(&t.name)),
                format!("from: {}", quote(&t.from)),
                format!("to: {}", quote(&t.to)),
            ];
            if !t.rows.is_empty() {
                fields.push(format!(
                    "rows: [{}]",
                    t.rows
                        .iter()
                        .map(|[a, b]| format!("[{}, {}]", quote(a), quote(b)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            format!("{{{}}}", fields.join(", "))
        })
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(", "))
}
fn text_cell(ui: &mut Ui, text: &mut String, width: f32) -> bool {
    let r = ui.add_sized(
        vec2(width, 24.),
        egui::Button::new(if text.is_empty() {
            "Enter value…"
        } else {
            text.as_str()
        })
        .frame(false),
    );
    let mut changed = false;
    egui::Popup::menu(&r)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            changed = ui
                .add(egui::TextEdit::singleline(text).desired_width(145.))
                .changed();
            if ui.button("Done").clicked() {
                ui.close();
            }
        });
    changed
}
fn heading(t: &ConversionTable) -> String {
    if t.name == format!("{} ↔ {}", t.from, t.to) {
        format!("{} – {}", t.from, t.to)
    } else {
        t.name.clone()
    }
}
pub(super) fn inspect(ui: &mut Ui, tables: &[ConversionTable]) {
    if tables.is_empty() {
        return;
    }
    ui.add_space(5.);
    ui.strong("Conversion tables");
    for (i, t) in tables.iter().enumerate() {
        egui::CollapsingHeader::new(heading(t))
            .id_salt(("conversion-pair", i))
            .show(ui, |ui| {
                if t.rows.is_empty() {
                    ui.weak(format!(
                        "Covers values within the declared ranges of {} and {}.",
                        t.from, t.to
                    ));
                } else {
                    ui.horizontal(|ui| {
                        ui.add_sized(vec2(110., 20.), egui::Label::new(&t.from));
                        ui.label(&t.to);
                    });
                    egui::ScrollArea::vertical().max_height(200.).show_rows(
                        ui,
                        24.,
                        t.rows.len(),
                        |ui, rows| {
                            for i in rows {
                                ui.horizontal(|ui| {
                                    ui.add_sized(vec2(110., 24.), egui::Label::new(&t.rows[i][0]));
                                    ui.monospace(&t.rows[i][1]);
                                });
                            }
                        },
                    );
                }
            });
    }
}
pub(super) fn edit(ui: &mut Ui, tables: &mut Vec<ConversionTable>, units: &[String], lookup: bool) -> bool {
    let mut changed = false;
    ui.add_space(5.);
    ui.strong("Conversion tables");
    let mut remove = None;
    for (i, t) in tables.iter_mut().enumerate() {
        egui::CollapsingHeader::new(heading(t))
            .id_salt(("conversion-pair", i))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    changed |= text_cell(ui, &mut t.name, 135.);
                });
                let endpoints = (t.from.clone(), t.to.clone());
                ui.horizontal_wrapped(|ui| {
                    selector(ui, ("from", i), &mut t.from, units.iter().cloned());
                    ui.label("and");
                    selector(ui, ("to", i), &mut t.to, units.iter().cloned());
                });
                changed |= endpoints.0 != t.from || endpoints.1 != t.to;
                if lookup {
                    let mut del = None;
                    ui.horizontal(|ui| {
                        for name in [&t.from, &t.to] {
                            ui.add_sized(
                                vec2(105., 20.),
                                egui::Label::new(egui::RichText::new(name).strong()),
                            );
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt(("conversion-values", i))
                        .max_height(215.)
                        .show_rows(ui, 24., t.rows.len(), |ui, visible| {
                            for j in visible {
                                ui.push_id(j, |ui| {
                                    ui.horizontal(|ui| {
                                        let [a, b] = &mut t.rows[j];
                                        changed |= text_cell(ui, a, 105.);
                                        changed |= text_cell(ui, b, 105.);
                                        if ui.small_button("×").on_hover_text("Remove pair").clicked() {
                                            del = Some(j);
                                        }
                                    })
                                });
                            }
                        });
                    if let Some(j) = del {
                        t.rows.remove(j);
                        changed = true;
                    }
                    if ui.small_button("+  Add pair").clicked() {
                        t.rows.push(Default::default());
                        changed = true;
                    }
                } else {
                    ui.weak("Covers values within both declared ranges.");
                }
                if ui.small_button("Remove table").clicked() {
                    remove = Some(i);
                }
            });
    }
    if let Some(i) = remove {
        tables.remove(i);
        changed = true;
    }
    if ui
        .add_enabled(units.len() >= 2, egui::Button::new("+  Add conversion table"))
        .clicked()
    {
        let (from, to) = units
            .iter()
            .enumerate()
            .flat_map(|(i, a)| units.iter().skip(i + 1).map(move |b| (a, b)))
            .find(|(a, b)| {
                !tables
                    .iter()
                    .any(|t| (&t.from == *a && &t.to == *b) || (&t.from == *b && &t.to == *a))
            })
            .unwrap_or((&units[0], &units[1]));
        let mut i = tables.len() + 1;
        while tables.iter().any(|t| t.name == format!("Table {i}")) {
            i += 1;
        }
        tables.push(ConversionTable {
            name: format!("Table {i}"),
            from: from.clone(),
            to: to.clone(),
            rows: if lookup { vec![Default::default()] } else { vec![] },
        });
        changed = true;
    }
    changed
}
pub(super) fn graph(
    kind: StrapKind,
    units: &[AttributeUnit],
    tables: Option<&[ConversionTable]>,
) -> Result<ConversionGraph> {
    ConversionGraph::build(
        kind,
        units,
        tables
            .map(|t| t.to_vec())
            .unwrap_or_else(|| ConversionGraph::defaults(units)),
    )
}
