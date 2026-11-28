//! Schematic and board checks edit policy declarations in the open document.
use forge_lang::{
    error::Op,
    props::quote,
    syntax::{self, ExprKind, StmtKind},
};

pub(super) fn menu<'a>(
    ui: &mut egui::Ui,
    rules: impl Iterator<Item = (&'static str, &'a str)>,
) -> Option<(&'static str, Option<&'static str>)> {
    let mut change = None;
    ui.menu_button("Rule severities", |ui| {
        egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
            for (id, level) in rules {
                ui.menu_button(format!("{id}  ·  {level}"), |ui| {
                    for value in ["error", "warning", "info"] {
                        if ui.selectable_label(level == value, value).clicked() {
                            change = Some((id, Some(value)));
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    }
                    ui.separator();
                    if ui.button("Use inherited severity").clicked() {
                        change = Some((id, None));
                        ui.close_kind(egui::UiKind::Menu);
                    }
                });
            }
        });
    });
    change
}

pub(super) fn edit(
    src: &str,
    head: &str,
    rule: &str,
    level: Option<&str>,
    fresh: impl FnOnce() -> String,
) -> Result<Option<Op>, String> {
    for stmt in syntax::parse(src).map_err(|e| e.text())? {
        if let StmtKind::Assign { name, value } = stmt.kind
            && let ExprKind::Call(func, args) = value.kind
            && matches!(&func.kind,ExprKind::Name(n) if n==head)
            && args.iter().any(|a| {
                a.name.as_ref().is_some_and(|(k, _)| k == "rule")
                    && matches!(&a.value.kind,ExprKind::Str(v) if v==rule)
            })
        {
            return Ok(Some(match level {
                Some(value) => Op::SetArg {
                    id: name,
                    arg: "severity".into(),
                    text: quote(value),
                },
                None => Op::DeleteFeature { id: name },
            }));
        }
    }
    Ok(level.map(|value| Op::AddFeature {
        before: None,
        text: format!(
            "{} = {head}(rule: {}, severity: {})",
            fresh(),
            quote(rule),
            quote(value)
        ),
    }))
}
