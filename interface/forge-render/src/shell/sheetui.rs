//! The sheet metal environment on screen: the Sheet Metal and Flat Pattern tabs' groups, each
//! command's dialog (Inventor's layout: picks first, a tab per concern, OK Cancel Apply), the
//! Sheet Metal Defaults and rules, the flat pattern's bend lines and extents over the view, its
//! bend table and its DXF export.

use super::fgasm::{dialog_frame, dialog_head};
use super::fgtool::SlotKind;
use super::ribbon::{Group, Tool};
use super::sheet::{
    FlatView, HW_KINDS, LIBRARY, NewRule, RULE_CORNER, RULE_RELIEF, SheetAct, SheetCmd, SheetDlg, TABS, Val,
    icon, title,
};
use super::theme::{medium, mono, sans, semi};
use super::{Action, Shell, icons};
use crate::shell::placed::Placed;
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

const DLG_W: f32 = 330.0;
const UP: Color32 = Color32::from_rgb(0x4A, 0x9C, 0xE8);
const DOWN: Color32 = Color32::from_rgb(0xE8, 0x8A, 0x3A);

fn tool(name: &'static str, cmd: SheetCmd, on: bool) -> Tool {
    Tool {
        name,
        icon: icon(cmd),
        action: Some(Action::Sheet(cmd)),
        on,
    }
}

/// Shown, not built yet: its tooltip says so.
fn coming(name: &'static str, icon: &'static str) -> Tool {
    Tool {
        name,
        icon,
        action: None,
        on: false,
    }
}

/// The Sheet Metal tab, Inventor's groups in Inventor's order.
pub(super) fn sheet_groups(s: &Shell) -> Vec<Group> {
    let g = |name, big, small| Group {
        name,
        big,
        small,
        icons: vec![],
    };
    let on = |c: SheetCmd| s.sheet.dlg.as_ref().is_some_and(|d| d.cmd == c);
    let sketch_on = s.sketch_pick;
    vec![
        g(
            "Sketch",
            vec![Tool {
                name: "Start 2D Sketch",
                icon: "sketch",
                action: Some(Action::StartSketch),
                on: sketch_on,
            }],
            vec![],
        ),
        g(
            "Create",
            vec![
                tool("Face", SheetCmd::Face, on(SheetCmd::Face)),
                tool("Contour Flange", SheetCmd::Contour, on(SheetCmd::Contour)),
                tool("Flange", SheetCmd::Flange, on(SheetCmd::Flange)),
            ],
            vec![
                coming("Contour Roll", "contourroll"),
                coming("Lofted Flange", "loftedflange"),
                tool("Hem", SheetCmd::Hem, on(SheetCmd::Hem)),
                tool("Fold", SheetCmd::Fold, on(SheetCmd::Fold)),
                tool("Jog", SheetCmd::Jog, on(SheetCmd::Jog)),
                coming("Bend", "smbend"),
            ],
        ),
        g(
            "Modify",
            vec![
                tool("Cut", SheetCmd::Cut, on(SheetCmd::Cut)),
                tool("Corner Seam", SheetCmd::CornerSeam, on(SheetCmd::CornerSeam)),
            ],
            vec![
                tool(
                    "Corner Relief",
                    SheetCmd::CornerRelief,
                    on(SheetCmd::CornerRelief),
                ),
                coming("Corner Round", "fillet"),
                coming("Corner Chamfer", "chamfer"),
                coming("Rip", "rip"),
                tool("Unfold", SheetCmd::Unfold, on(SheetCmd::Unfold)),
                coming("Refold", "refold"),
                coming("Split", "split"),
            ],
        ),
        g(
            "Punch",
            vec![
                tool("PunchTool", SheetCmd::Punch, on(SheetCmd::Punch)),
                tool("Insert Hardware", SheetCmd::Hardware, on(SheetCmd::Hardware)),
            ],
            vec![],
        ),
        g(
            "Flat Pattern",
            vec![Tool {
                name: if s.fg.as_ref().is_some_and(|f| f.sheets.is_empty()) {
                    "Create Flat Pattern"
                } else {
                    "Go to Flat Pattern"
                },
                icon: "flatpattern",
                action: s
                    .fg
                    .as_ref()
                    .is_some_and(|f| !f.sheets.is_empty())
                    .then_some(Action::Sheet(SheetCmd::FlatPattern)),
                on: false,
            }],
            vec![],
        ),
        g(
            "Setup",
            vec![tool(
                "Sheet Metal Defaults",
                SheetCmd::Defaults,
                s.sheet.defaults_open.is_some(),
            )],
            vec![
                tool("Rules", SheetCmd::Rules, s.sheet.rules_open.is_some()),
                Tool {
                    name: "Convert to Part",
                    icon: "folder",
                    action: Some(Action::Sheet(SheetCmd::ConvertBack)),
                    on: false,
                },
            ],
        ),
    ]
}

/// The Flat Pattern tab.
pub(super) fn flat_groups(s: &Shell) -> Vec<Group> {
    let g = |name, big, small| Group {
        name,
        big,
        small,
        icons: vec![],
    };
    let u = &s.sheet;
    vec![
        g(
            "Manage",
            vec![
                tool("Bend Table", SheetCmd::BendTable, u.bend_table),
                tool("Save Copy As DXF/DWG", SheetCmd::ExportDxf, u.dxf_open.is_some()),
            ],
            vec![],
        ),
        g(
            "Show",
            vec![],
            vec![
                Tool {
                    name: "Extents",
                    icon: "rectangle",
                    action: Some(Action::Sheet(SheetCmd::ShowExtents)),
                    on: u.extents,
                },
                Tool {
                    name: "Bend Lines",
                    icon: "lines",
                    action: Some(Action::Sheet(SheetCmd::ShowBends)),
                    on: u.bend_lines,
                },
                Tool {
                    name: "Tangents",
                    icon: "tangent",
                    action: Some(Action::Sheet(SheetCmd::ShowTangents)),
                    on: u.tangents,
                },
                Tool {
                    name: "Punch Centres",
                    icon: "point",
                    action: Some(Action::Sheet(SheetCmd::ShowTools)),
                    on: u.tool_marks,
                },
            ],
        ),
        g(
            "Folded Part",
            vec![tool("Go to Folded Part", SheetCmd::Folded, false)],
            vec![],
        ),
    ]
}

/// A row: its label at the left, the control after it.
fn label(ui: &mut Ui, t: &super::theme::Theme, text: &str) {
    ui.add_sized(
        vec2(118.0, 22.0),
        egui::Label::new(egui::RichText::new(text).font(sans(12.0)).color(t.ink2)).truncate(),
    );
}

fn section(ui: &mut Ui, t: &super::theme::Theme, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).font(semi(12.0)).color(t.ink));
}

/// What a slot takes, said the way Inventor's prompt does.
fn prompt(k: SlotKind) -> &'static str {
    match k {
        SlotKind::SheetEdge => "Select an edge of the sheet",
        SlotKind::Feature => "Select a face (its wall) or a feature in the browser",
        SlotKind::Profile | SlotKind::Profiles => "Select a sketch or a closed loop",
        SlotKind::Path => "Select an open sketch: lines and arcs",
        SlotKind::Points => "Select a sketch point, or type x and y",
        _ => "Select",
    }
}

impl Shell {
    /// The sheet metal environment's windows and overlays.
    pub(super) fn sheet_ui(&mut self, ctx: &egui::Context) {
        if self.fg.is_none() {
            return;
        }
        self.sheet_flat_overlay(ctx);
        self.sheet_dialog(ctx);
        self.sheet_defaults(ctx);
        self.sheet_rules(ctx);
        self.sheet_bend_table(ctx);
        self.sheet_dxf(ctx);
    }

    fn sheet_window<R>(
        &mut self,
        ctx: &egui::Context,
        id: &str,
        icon: &str,
        head: &str,
        (at, width): (egui::Pos2, f32),
        body: impl FnOnce(&mut Ui, &mut Vec<(String, Rect)>) -> R,
    ) -> (bool, Option<R>) {
        let t = self.theme;
        let mut places = vec![];
        let mut close = false;
        let mut got = None;
        let shown = egui::Window::new(head)
            .id(egui::Id::unique(("sheet-win", id)))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_pos(at)
            .min_width(width)
            .max_width(width)
            .frame(dialog_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                close = dialog_head(ui, &t, icon, head, id, places);
                ui.spacing_mut().item_spacing.y = 5.0;
                got = Some(body(ui, places));
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
        }
        self.places.extend(places);
        (close, got)
    }

    fn sheet_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.sheet.dlg.take() else {
            return;
        };
        let t = self.theme;
        let fg = self.fg.as_ref().expect("a Forge part");
        // what the part offers each slot, for picks the view can't make
        let sketches: Vec<String> = fg.sketches.iter().map(|s| s.id.clone()).collect();
        let loops: Vec<String> = fg.sketches.iter().flat_map(|s| s.closed.clone()).collect();
        let points: Vec<String> = fg.sketches.iter().flat_map(|s| s.points.clone()).collect();
        let paths: Vec<String> = fg
            .sketches
            .iter()
            .flat_map(|s| s.lines.iter().chain(&s.curves).cloned())
            .collect();
        let walls: Vec<String> = fg
            .feats
            .iter()
            .filter(|f| {
                matches!(
                    f.kind.as_str(),
                    "sheet" | "flange" | "contour" | "sketched_bend" | "jog"
                )
            })
            .map(|f| f.id.clone())
            .collect();
        let flanges: Vec<String> = fg
            .feats
            .iter()
            .filter(|f| f.kind == "flange")
            .map(|f| f.id.clone())
            .collect();
        let sheets: Vec<String> = fg
            .feats
            .iter()
            .filter(|f| matches!(f.kind.as_str(), "sheet" | "contour"))
            .map(|f| f.id.clone())
            .collect();
        let edges = d.edges.get_or_insert_with(|| self.sheet.edges.clone()).clone();
        // what the preview's build says of the features the dialog writes: OK waits on it
        let failed: Option<String> = fg
            .issues
            .iter()
            .find(|i| {
                i.severity == super::fg::Severity::Error && i.feature.as_deref().is_some_and(|f| d.writes(f))
            })
            .map(|i| format!("{}: {}", i.kind, i.text))
            .or_else(|| {
                fg.feats
                    .iter()
                    .find(|f| d.writes(&f.id) && f.status == super::fg::Status::Failed)
                    .map(|f| format!("{} doesn't build", f.id))
            })
            .or_else(|| {
                // an error the part didn't have before the dialog: its own, unnamed
                let errs: Vec<&super::fg::Issue> = fg
                    .issues
                    .iter()
                    .filter(|i| i.severity == super::fg::Severity::Error)
                    .collect();
                (d.base.is_some()
                    && d.sent.as_ref().is_some_and(|s| !s.is_empty())
                    && errs.len() > d.base_errors)
                    .then(|| errs.last().map(|i| format!("{}: {}", i.kind, i.text)))
                    .flatten()
            });
        let catalogue = self.sheet.catalogue.clone();
        let v = self.view_rect;
        let (head, _) = title(d.cmd);
        let head = match &d.edit {
            Some(id) => format!("{head} : {id}"),
            None => head.to_string(),
        };
        let mut ok = false;
        let mut cancel = false;
        let defaults = self.sheet.defaults.clone();
        let (close, _) = self.sheet_window(
            ctx,
            "sheet:dlg",
            icon(d.cmd),
            &head, (pos2(v.left() + 16.0, v.top() + 48.0), DLG_W),
            |ui, places| {
                // the tabs a command has: punch and hardware are one page
                let tabs: Vec<usize> = (0..TABS.len())
                    .filter(|k| *k == 0 || d.fields.iter().any(|f| f.tab == *k) || (*k == 1 && matches!(d.cmd, SheetCmd::Flange | SheetCmd::Face | SheetCmd::Contour)))
                    .collect();
                if tabs.len() > 1 {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for k in tabs {
                            let on = d.tab == k;
                            let g = ui.fonts_mut(|f| f.layout_no_wrap(TABS[k].to_string(), sans(12.0), t.ink));
                            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 20.0, 24.0), Sense::click());
                            let p = ui.painter();
                            if on {
                                p.rect_filled(r, egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, t.raised);
                                p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 2.0)), 0.0, t.focus);
                            } else if resp.hovered() {
                                p.rect_filled(r, 4.0, t.hover);
                            }
                            p.text(r.center(), Align2::CENTER_CENTER, TABS[k], if on { medium(12.0) } else { sans(12.0) }, if on { t.ink } else { t.ink2 });
                            places.push((format!("sheet:tab:{}", TABS[k]), r));
                            if resp.clicked() {
                                d.tab = k;
                            }
                        }
                    });
                    let r = ui.available_rect_before_wrap();
                    ui.painter().hline(r.left()..=r.right(), r.top() - 3.0, Stroke::new(1.0, t.line));
                }
                if d.tab == 0 {
                    Self::sheet_picks(ui, &t, &mut d, places, &edges, &sketches, &loops, &points, &walls, &flanges, &sheets, &paths);
                    match d.cmd {
                        SheetCmd::Punch => Self::sheet_library(ui, &t, &mut d, places),
                        SheetCmd::Hardware => Self::sheet_catalogue(ui, &t, &mut d, places, &catalogue),
                        _ => {}
                    }
                }
                if d.tab == 1 && !d.fields.iter().any(|f| f.tab == 1) {
                    let text = match d.cmd {
                        SheetCmd::Flange => "Unfold rule: the sheet's (Sheet Metal Defaults). A rule of its own for one flange is coming.",
                        _ => "Unfold rule, thickness and bend radius come from Sheet Metal Defaults.",
                    };
                    ui.label(egui::RichText::new(text).font(sans(12.0)).color(t.ink2));
                    let rule = match &defaults.rule {
                        Some(r) => format!("Rule: {r}"),
                        None => format!(
                            "By value: thickness {} mm, radius {} mm, K {}",
                            super::fg::short(defaults.t),
                            super::fg::short(defaults.r),
                            super::fg::short(defaults.k)
                        ),
                    };
                    ui.label(egui::RichText::new(rule).font(mono(11.0)).color(t.ink));
                }
                let tab = d.tab;
                Self::sheet_fields(ui, &t, &mut d, tab, places);
                ui.horizontal(|ui| {
                    label(ui, &t, "Name");
                    let r = ui.add_enabled(
                        d.edit.is_none(),
                        egui::TextEdit::singleline(&mut d.name).font(mono(12.0)).desired_width(150.0),
                    );
                    places.push(("sheet:name".into(), r.rect));
                });
                if let Some(e) = d.error.as_ref().or(failed.as_ref()) {
                    let r = ui.add(egui::Label::new(egui::RichText::new(e).font(sans(12.0)).color(t.error)).wrap());
                    places.push(("sheet:error".into(), r.rect));
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let b = |ui: &mut Ui, text: &str, name: &str, places: &mut Vec<(String, Rect)>| {
                        let r = ui.add_sized(vec2(70.0, 26.0), egui::Button::new(egui::RichText::new(text).font(sans(12.0))));
                        places.push((name.into(), r.rect));
                        r.clicked()
                    };
                    if b(ui, "OK", "sheet:ok", places) {
                        ok = true;
                    }
                    if b(ui, "Cancel", "sheet:cancel", places) {
                        cancel = true;
                    }
                    if d.edit.is_none() && b(ui, "Apply +", "sheet:apply", places) {
                        ok = true;
                        d.again = true;
                    }
                });
            },
        );
        if close || cancel {
            self.sheet.out.push(SheetAct::Cancel);
            self.status.clear();
            return;
        }
        if ok && failed.is_some() {
            d.again = false;
            self.status = format!(
                "{}: it doesn't build as it stands; change it or Cancel",
                title(d.cmd).0
            );
        } else if ok {
            d.done = true;
        }
        if let Some(s) = d.slots.get(d.active)
            && s.picks.is_empty()
        {
            self.status = format!("{}: {}", title(d.cmd).0, prompt(s.kind));
        }
        self.sheet.dlg = Some(d);
    }

    #[allow(clippy::too_many_arguments)]
    fn sheet_picks(
        ui: &mut Ui,
        t: &super::theme::Theme,
        d: &mut SheetDlg,
        places: &mut Vec<(String, Rect)>,
        edges: &[String],
        sketches: &[String],
        loops: &[String],
        points: &[String],
        walls: &[String],
        flanges: &[String],
        sheets: &[String],
        paths: &[String],
    ) {
        for k in 0..d.slots.len() {
            let (lab, kind) = (d.slots[k].label, d.slots[k].kind);
            let on = d.active == k;
            ui.horizontal(|ui| {
                // Inventor's selection button: an arrow, the slot's name, red till it has picks
                let (r, resp) = ui.allocate_exact_size(vec2(118.0, 24.0), Sense::click());
                let p = ui.painter();
                let empty = d.slots[k].picks.is_empty();
                let edge = if on { t.focus } else { t.line };
                p.rect_filled(r, 3.0, if on { t.on } else { t.raised });
                p.rect_stroke(r, 3.0, Stroke::new(1.0, edge), StrokeKind::Inside);
                icons::draw(
                    p,
                    pos2(r.left() + 4.0, r.top() + 4.0),
                    16.0,
                    "select",
                    1.5,
                    if empty { t.error } else { t.ink },
                );
                p.text(
                    pos2(r.left() + 24.0, r.center().y),
                    Align2::LEFT_CENTER,
                    lab,
                    medium(12.0),
                    t.ink,
                );
                places.push((format!("sheet:slot:{lab}"), r));
                if resp.clicked() {
                    d.active = k;
                }
                let n = d.slots[k].picks.len();
                let said = match n {
                    0 => "none".to_string(),
                    1 => d.slots[k].picks[0].clone(),
                    n => format!("{n} selected"),
                };
                ui.add(
                    egui::Label::new(egui::RichText::new(said).font(mono(11.0)).color(if n == 0 {
                        t.ink2
                    } else {
                        t.ink
                    }))
                    .truncate(),
                );
            });
            if !on {
                continue;
            }
            let choices: Vec<String> = match kind {
                SlotKind::SheetEdge => edges.to_vec(),
                SlotKind::Feature if lab.starts_with("Flange") => flanges.to_vec(),
                SlotKind::Feature if lab == "Sheet" => sheets.to_vec(),
                SlotKind::Feature => walls.to_vec(),
                SlotKind::Profile | SlotKind::Profiles => sketches.iter().chain(loops).cloned().collect(),
                SlotKind::Path => paths.to_vec(),
                SlotKind::Points => points.to_vec(),
                _ => vec![],
            };
            // what the dialog's own preview made isn't offered: the list stays still as it builds
            let own = |c: &String| {
                let base = c.split(['.', '(', '[']).next().unwrap_or("");
                d.edit.is_none() && d.writes(base)
            };
            let choices: Vec<String> = choices.into_iter().filter(|c| !own(c)).collect();
            let picked = d.slots[k].picks.clone();
            let mut take = None;
            egui::ScrollArea::vertical()
                .id_salt(("sheet-choices", lab))
                .max_height(96.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                        for c in choices {
                            let sel = picked.contains(&c);
                            let r = ui.add(
                                egui::Button::new(egui::RichText::new(&c).font(mono(10.0)))
                                    .small()
                                    .selected(sel),
                            );
                            places.push((format!("sheet:choice:{lab}:{c}"), r.rect));
                            if r.clicked() {
                                take = Some(c);
                            }
                        }
                    });
                });
            if let Some(c) = take {
                d.take(Ok(c));
            }
            if kind == SlotKind::Points && matches!(d.cmd, SheetCmd::Punch | SheetCmd::Hardware) {
                let r = ui.checkbox(
                    &mut d.typed_at,
                    egui::RichText::new("Type x, y on the face instead").font(sans(12.0)),
                );
                places.push(("sheet:typed".into(), r.rect));
                if d.typed_at {
                    d.slots[k].picks.clear();
                }
            }
            ui.label(egui::RichText::new(prompt(kind)).font(sans(11.0)).color(t.ink2));
        }
    }

    fn sheet_fields(
        ui: &mut Ui,
        t: &super::theme::Theme,
        d: &mut SheetDlg,
        tab: usize,
        places: &mut Vec<(String, Rect)>,
    ) {
        let shown: Vec<bool> = d.fields.iter().map(|f| d.shown(f)).collect();
        let mut chosen: Option<(usize, &'static str)> = None;
        let heads: &[(&str, &str)] = &[
            ("h", "Height Extents"),
            ("side", "Orientation"),
            ("measure", "Height Datum"),
            ("position", "Bend Position"),
            ("r", "Bend"),
            ("relief", "Relief"),
            ("kind", "Shape"),
            ("x0", "Bend Line"),
            ("crosses_bend", "Options"),
        ];
        for (k, f) in d.fields.iter_mut().enumerate() {
            if f.tab != tab || !shown[k] || (matches!(d.cmd, SheetCmd::Hardware) && f.arg.starts_with('#')) {
                continue;
            }
            let key = [f.param, f.arg, f.label]
                .into_iter()
                .find(|k| !k.is_empty())
                .unwrap_or("");
            if let Some(h) = heads.iter().find(|h| h.0 == key) {
                section(ui, t, h.1);
            }
            ui.horizontal(|ui| {
                label(ui, t, f.label);
                match &mut f.val {
                    Val::Num { text, dim } => {
                        let hint = if f.default.is_empty() && f.first.is_none() && text.is_empty() {
                            "default"
                        } else {
                            ""
                        };
                        let r = ui.add(
                            egui::TextEdit::singleline(text)
                                .font(mono(12.0))
                                .hint_text(hint)
                                .desired_width(86.0),
                        );
                        places.push((format!("sheet:field:{key}"), r.rect));
                        let unit = match dim {
                            forge_lang::values::Dim::Angle => "deg",
                            forge_lang::values::Dim::Ratio => "",
                            _ => "mm",
                        };
                        ui.label(egui::RichText::new(unit).font(sans(11.0)).color(t.ink2));
                    }
                    Val::Choice { opts, at } => {
                        let now = opts[*at].1;
                        let r = egui::ComboBox::from_id_salt(("sheet-choice", key))
                            .selected_text(egui::RichText::new(now).font(sans(12.0)))
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for (i, o) in opts.iter().enumerate() {
                                    let r = ui.add_enabled(
                                        o.2,
                                        egui::Button::selectable(
                                            i == *at,
                                            egui::RichText::new(o.1).font(sans(12.0)),
                                        ),
                                    );
                                    let r = if o.2 {
                                        r
                                    } else {
                                        r.on_disabled_hover_text("coming")
                                    };
                                    places.push((format!("sheet:opt:{key}:{}", o.0), r.rect));
                                    if r.clicked() {
                                        chosen = Some((k, o.0));
                                    }
                                }
                            });
                        places.push((format!("sheet:field:{key}"), r.response.rect));
                    }
                    Val::Flag(on) => {
                        let r = ui.checkbox(on, "");
                        places.push((format!("sheet:field:{key}"), r.rect));
                    }
                    Val::Text(s) => {
                        let r = ui.add(egui::TextEdit::singleline(s).font(mono(12.0)).desired_width(86.0));
                        places.push((format!("sheet:field:{key}"), r.rect));
                    }
                }
            });
        }
        if let Some((k, w)) = chosen {
            let key = if d.fields[k].param.is_empty() {
                d.fields[k].label
            } else {
                d.fields[k].param
            };
            let key = if d.fields[k].arg.is_empty() || d.fields[k].arg.starts_with('#') {
                key
            } else {
                d.fields[k].arg
            };
            d.choose(key, w);
            // a choice keyed by its arg (a hem's kind) when it has no param
            if let Some(f) = d.fields.get_mut(k)
                && let Val::Choice { opts, at } = &mut f.val
                && opts[*at].0 != w
                && let Some(i) = opts.iter().position(|o| o.0 == w && o.2)
            {
                *at = i;
            }
        }
    }

    /// The punch library: the tools, a picture of the one chosen, its sizes.
    fn sheet_library(
        ui: &mut Ui,
        t: &super::theme::Theme,
        d: &mut SheetDlg,
        places: &mut Vec<(String, Rect)>,
    ) {
        section(ui, t, "Punch Library");
        if let Some(id) = &d.shop_tool {
            ui.label(
                egui::RichText::new(format!("The shop's tool {id}"))
                    .font(mono(12.0))
                    .color(t.ink),
            );
            return;
        }
        // turret punches or forming tools, as the library's two folders
        let forms = LIBRARY.get(d.tool).is_some_and(|t| t.form);
        ui.horizontal(|ui| {
            for (word, f) in [("Punches", false), ("Forming tools", true)] {
                let r = ui.add(egui::Button::selectable(
                    forms == f,
                    egui::RichText::new(word).font(sans(12.0)),
                ));
                places.push((format!("sheet:lib:{word}"), r.rect));
                if r.clicked() && forms != f {
                    d.tool = LIBRARY.iter().position(|t| t.form == f).unwrap_or(0);
                }
            }
        });
        ui.horizontal(|ui| {
            let mut pick = None;
            egui::ScrollArea::vertical()
                .id_salt("sheet-lib")
                .auto_shrink([false, false])
                .min_scrolled_height(280.0)
                .max_height(280.0)
                .max_width(150.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(146.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (k, tl) in LIBRARY.iter().enumerate().filter(|x| x.1.form == forms) {
                            let on = d.tool == k;
                            let text = egui::RichText::new(tl.id).font(mono(11.0));
                            let r = ui.add_enabled(
                                tl.built,
                                egui::Button::selectable(on, text).min_size(vec2(146.0, 20.0)),
                            );
                            let r = if tl.built {
                                r.on_hover_text(tl.about)
                            } else {
                                r.on_disabled_hover_text(format!("{}: coming", tl.about))
                            };
                            places.push((format!("sheet:tool:{}", tl.id), r.rect));
                            if r.clicked() {
                                pick = Some(k);
                            }
                        }
                    });
                });
            if let Some(k) = pick {
                d.tool = k;
            }
            // the preview: the tool's outline, to scale in its box
            let (r, _) = ui.allocate_exact_size(vec2(150.0, 280.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, 4.0, t.bg);
            p.rect_stroke(r, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            if let Some(tl) = LIBRARY.get(d.tool) {
                tool_picture(p, r.shrink(18.0), tl, t);
                p.text(
                    r.center_bottom() - vec2(0.0, 8.0),
                    Align2::CENTER_CENTER,
                    tl.about,
                    sans(10.0),
                    t.ink2,
                );
                places.push(("sheet:preview".into(), r));
            }
        });
    }

    /// The hardware catalogue: the shop's rows, else the placeholder ones (said so).
    fn sheet_catalogue(
        ui: &mut Ui,
        t: &super::theme::Theme,
        d: &mut SheetDlg,
        places: &mut Vec<(String, Rect)>,
        rows: &[super::sheet::HwRow],
    ) {
        section(ui, t, "Catalogue");
        let kind = d.field("#0").map_or("nut", |f| f.word());
        let size = d.field("Size").map_or(String::new(), |f| f.num().to_string());
        let mut pick = None;
        egui::Grid::new("sheet-hw")
            .striped(true)
            .spacing(vec2(10.0, 2.0))
            .show(ui, |ui| {
                for h in ["Type", "Size", "Hole", "Edge", "Code"] {
                    ui.label(egui::RichText::new(h).font(medium(11.0)).color(t.ink2));
                }
                ui.end_row();
                for (k, row) in rows.iter().enumerate() {
                    let on = row.kind == kind && row.size == size;
                    let word = HW_KINDS
                        .iter()
                        .find(|o| o.0 == row.kind)
                        .map_or(row.kind.as_str(), |o| o.1);
                    let r = ui.add(egui::Button::selectable(
                        on,
                        egui::RichText::new(word).font(sans(11.0)),
                    ));
                    places.push((format!("sheet:hw:{}:{}", row.kind, row.size), r.rect));
                    if r.clicked() {
                        pick = Some(k);
                    }
                    ui.label(egui::RichText::new(&row.size).font(mono(11.0)).color(t.ink));
                    ui.label(
                        egui::RichText::new(super::fg::short(row.hole))
                            .font(mono(11.0))
                            .color(t.ink),
                    );
                    ui.label(
                        egui::RichText::new(super::fg::short(row.edge))
                            .font(mono(11.0))
                            .color(t.ink),
                    );
                    ui.label(
                        egui::RichText::new(if row.code.is_empty() { "-" } else { &row.code })
                            .font(mono(11.0))
                            .color(t.ink2),
                    );
                    ui.end_row();
                }
            });
        if rows.iter().any(|r| r.source.is_none()) {
            ui.label(
                egui::RichText::new(
                    "Placeholder rows: not a maker's table. Put a cited one in the shop's hardware.",
                )
                .font(sans(11.0))
                .color(super::fg::WARN),
            );
        }
        if let Some(k) = pick {
            let (kind, size) = (rows[k].kind.clone(), rows[k].size.clone());
            d.choose("#0", &kind);
            d.set("Size", &size);
        }
    }

    fn sheet_defaults(&mut self, ctx: &egui::Context) {
        let Some(mut dd) = self.sheet.defaults_open.take() else {
            return;
        };
        let t = self.theme;
        let rules = self
            .fg
            .as_ref()
            .map(|f| super::sheet::rules(&f.params))
            .unwrap_or_default();
        let v = self.view_rect;
        let mut ok = false;
        let mut cancel = false;
        let mut styles = false;
        let (close, _) = self.sheet_window(ctx, "sheet:defaults", "smdefaults", "Sheet Metal Defaults", (pos2(v.left() + 16.0, v.top() + 48.0), 340.0), |ui, places| {
            ui.horizontal(|ui| {
                label(ui, &t, "Sheet Metal Rule");
                let now = dd.rule.clone().unwrap_or_else(|| "By value (no shop)".into());
                egui::ComboBox::from_id_salt("sheet-rule").selected_text(egui::RichText::new(now).font(sans(12.0))).width(150.0).show_ui(ui, |ui| {
                    let r = ui.selectable_label(dd.rule.is_none(), "By value (no shop)");
                    places.push(("sheet:rule:none".into(), r.rect));
                    if r.clicked() {
                        dd.rule = None;
                    }
                    for name in &rules {
                        let r = ui.selectable_label(dd.rule.as_deref() == Some(name.as_str()), name);
                        places.push((format!("sheet:rule:{name}"), r.rect));
                        if r.clicked() {
                            dd.rule = Some(name.clone());
                        }
                    }
                });
                let r = ui.button("…").on_hover_text("Edit the rules (Style and Standard Editor)");
                places.push(("sheet:defaults:styles".into(), r.rect));
                if r.clicked() {
                    styles = true;
                }
            });
            let by_value = dd.rule.is_none();
            let mut row = |ui: &mut Ui, name: &str, v: &mut f64, unit: &str, on: bool| {
                ui.horizontal(|ui| {
                    label(ui, &t, name);
                    let r = ui.add_enabled(on, egui::DragValue::new(v).speed(0.05).range(0.0..=100.0).max_decimals(3));
                    places.push((format!("sheet:defaults:{name}"), r.rect));
                    ui.label(egui::RichText::new(unit).font(sans(11.0)).color(t.ink2));
                });
            };
            row(ui, "Thickness", &mut dd.t, "mm", true);
            section(ui, &t, "Bend");
            row(ui, "Bend radius", &mut dd.r, "mm", by_value);
            section(ui, &t, "Unfold Rule");
            row(ui, "K-factor", &mut dd.k, "", by_value);
            if !by_value {
                ui.label(egui::RichText::new("The rule's stock of this thickness sets the radius, the unfold rule and the reliefs.").font(sans(11.0)).color(t.ink2));
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let r = ui.add_sized(vec2(70.0, 26.0), egui::Button::new("OK"));
                places.push(("sheet:defaults:ok".into(), r.rect));
                ok = r.clicked();
                let r = ui.add_sized(vec2(70.0, 26.0), egui::Button::new("Cancel"));
                places.push(("sheet:defaults:cancel".into(), r.rect));
                cancel = r.clicked();
            });
        });
        if styles {
            let fg = self.fg.as_ref().expect("a part");
            let mut taken = super::sheet::taken(fg);
            taken.extend(fg.params.iter().map(|p| p.key.name.clone()));
            self.sheet.rules_open = Some(NewRule::new(&taken));
        }
        if ok {
            let params = self.fg.as_ref().map(|f| f.params.clone()).unwrap_or_default();
            let ops = dd.ops(&params);
            let mut ops = ops;
            // the part's sheets take the rule chosen
            if dd.rule != self.sheet.defaults.rule
                && let Some(f) = &self.fg
            {
                for s in f.feats.iter().filter(|x| x.kind == "sheet") {
                    if let Some(r) = &dd.rule {
                        ops.push(forge_lang::error::Op::SetArg {
                            id: s.id.clone(),
                            arg: "shop".into(),
                            text: r.clone(),
                        });
                    }
                }
            }
            self.sheet.defaults = dd;
            if !ops.is_empty() {
                self.sheet.out.push(SheetAct::Apply(ops));
            }
            return;
        }
        if !(close || cancel) {
            self.sheet.defaults_open = Some(dd);
        }
    }

    fn sheet_rules(&mut self, ctx: &egui::Context) {
        let Some(mut nr) = self.sheet.rules_open.take() else {
            return;
        };
        let t = self.theme;
        let existing: Vec<(String, String)> = self
            .fg
            .as_ref()
            .map(|f| {
                f.params
                    .iter()
                    .filter(|p| {
                        p.key.owner.is_empty()
                            && (p.text.starts_with("shop(") || p.text.starts_with("stock("))
                    })
                    .map(|p| (p.key.name.clone(), p.text.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let v = self.view_rect;
        let mut save = false;
        let mut err = None;
        let (close, _) = self.sheet_window(
            ctx,
            "sheet:rules",
            "params",
            "Style and Standard Editor: Sheet Metal Rules",
            (pos2(v.left() + 380.0, v.top() + 48.0), 420.0),
            |ui, places| {
                section(ui, &t, "In this part");
                if existing.is_empty() {
                    ui.label(
                        egui::RichText::new("No rules yet: the part's sheets go by value.")
                            .font(sans(12.0))
                            .color(t.ink2),
                    );
                }
                egui::ScrollArea::vertical()
                    .id_salt("sheet-rules")
                    .max_height(110.0)
                    .show(ui, |ui| {
                        for (n, text) in &existing {
                            ui.label(egui::RichText::new(n).font(medium(12.0)).color(t.ink));
                            ui.add(
                                egui::Label::new(egui::RichText::new(text).font(mono(10.0)).color(t.ink2))
                                    .wrap(),
                            );
                        }
                    });
                section(ui, &t, "New rule");
                let mut text_row = |ui: &mut Ui, name: &str, s: &mut String, unit: &str| {
                    ui.horizontal(|ui| {
                        label(ui, &t, name);
                        let r = ui.add(egui::TextEdit::singleline(s).font(mono(12.0)).desired_width(90.0));
                        places.push((format!("sheet:rule:{name}"), r.rect));
                        ui.label(egui::RichText::new(unit).font(sans(11.0)).color(t.ink2));
                    });
                };
                text_row(ui, "Name", &mut nr.name, "");
                text_row(ui, "Thickness", &mut nr.t, "mm");
                text_row(ui, "K-factor", &mut nr.k, "");
                text_row(ui, "Bend radius", &mut nr.radius, "mm");
                text_row(ui, "Minimum flange", &mut nr.min_flange, "mm");
                text_row(ui, "Hole to bend", &mut nr.hole_to_bend, "mm");
                text_row(ui, "Kerf", &mut nr.kerf, "mm");
                ui.horizontal(|ui| {
                    label(ui, &t, "Bend relief");
                    egui::ComboBox::from_id_salt("rule-relief")
                        .selected_text(RULE_RELIEF[nr.relief].1)
                        .show_ui(ui, |ui| {
                            for (k, o) in RULE_RELIEF.iter().enumerate() {
                                ui.selectable_value(&mut nr.relief, k, o.1);
                            }
                        });
                });
                text_row(ui, "Relief width", &mut nr.relief_w, "mm");
                text_row(ui, "Relief depth", &mut nr.relief_d, "mm");
                ui.horizontal(|ui| {
                    label(ui, &t, "Corner relief");
                    egui::ComboBox::from_id_salt("rule-corner")
                        .selected_text(RULE_CORNER[nr.corner].1)
                        .show_ui(ui, |ui| {
                            for (k, o) in RULE_CORNER.iter().enumerate() {
                                ui.selectable_value(&mut nr.corner, k, o.1);
                            }
                        });
                });
                if let Err(e) = nr.text() {
                    err = Some(e.clone());
                    ui.label(egui::RichText::new(e).font(sans(12.0)).color(t.error));
                }
                ui.horizontal(|ui| {
                    let r = ui.add_sized(vec2(120.0, 26.0), egui::Button::new("Save and use"));
                    places.push(("sheet:rule:save".into(), r.rect));
                    save = r.clicked();
                });
            },
        );
        if save && err.is_none() {
            let text = nr.text().expect("checked");
            self.sheet.defaults.rule = Some(nr.name.clone());
            let mut ops = vec![forge_lang::error::Op::AddFeature { before: None, text }];
            // added at the end: the part's sheets read it from there, so it goes above the first
            if let Some(f) = &self.fg
                && let Some(first) = f.feats.first()
            {
                ops[0] = forge_lang::error::Op::AddFeature {
                    before: Some(first.id.clone()),
                    text: nr.text().expect("checked"),
                };
                for s in f.feats.iter().filter(|x| x.kind == "sheet") {
                    ops.push(forge_lang::error::Op::SetArg {
                        id: s.id.clone(),
                        arg: "shop".into(),
                        text: nr.name.clone(),
                    });
                }
            }
            self.sheet.out.push(SheetAct::Apply(ops));
            return;
        }
        if !close {
            self.sheet.rules_open = Some(nr);
        }
    }

    fn sheet_bend_table(&mut self, ctx: &egui::Context) {
        if !self.sheet.bend_table {
            return;
        }
        let t = self.theme;
        let flats = self.sheet.flats.clone();
        let v = self.view_rect;
        let (close, _) = self.sheet_window(
            ctx,
            "sheet:bends",
            "bendtable",
            "Bend Table",
            (pos2(v.right() - 480.0, v.top() + 48.0), 460.0),
            |ui, _| {
                for f in &flats {
                    ui.label(
                        egui::RichText::new(format!("{}: {} bends", f.sheet, f.bends.len()))
                            .font(semi(12.0))
                            .color(t.ink),
                    );
                    egui::Grid::new(("bend-table", &f.sheet))
                        .striped(true)
                        .spacing(vec2(14.0, 3.0))
                        .show(ui, |ui| {
                            for h in ["Bend", "Feature", "Direction", "Angle", "Radius", "Allowance"] {
                                ui.label(egui::RichText::new(h).font(medium(11.0)).color(t.ink2));
                            }
                            ui.end_row();
                            for (k, b) in f.bends.iter().enumerate() {
                                let up = b.angle >= 0.0;
                                ui.label(
                                    egui::RichText::new(format!("{}", k + 1))
                                        .font(mono(11.0))
                                        .color(t.ink),
                                );
                                ui.label(egui::RichText::new(&b.feature).font(mono(11.0)).color(t.ink));
                                ui.label(
                                    egui::RichText::new(if up { "UP" } else { "DOWN" })
                                        .font(medium(11.0))
                                        .color(if up { UP } else { DOWN }),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}°", b.angle.abs()))
                                        .font(mono(11.0))
                                        .color(t.ink),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", b.radius))
                                        .font(mono(11.0))
                                        .color(t.ink),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", b.allowance))
                                        .font(mono(11.0))
                                        .color(t.ink),
                                );
                                ui.end_row();
                            }
                        });
                }
            },
        );
        if close {
            self.sheet.bend_table = false;
        }
    }

    fn sheet_dxf(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.sheet.dxf_open.take() else {
            return;
        };
        let t = self.theme;
        let sheets: Vec<String> = self
            .fg
            .as_ref()
            .map(|f| f.sheets.iter().map(|s| s.id.clone()).collect())
            .unwrap_or_default();
        let v = self.view_rect;
        let mut ok = false;
        let mut cancel = false;
        let (close, _) = self.sheet_window(
            ctx,
            "sheet:dxf",
            "dxf",
            "Flat Pattern DXF/DWG Export Options",
            (pos2(v.left() + 16.0, v.top() + 48.0), 480.0),
            |ui, places| {
                ui.horizontal(|ui| {
                    label(ui, &t, "Sheet");
                    for s in &sheets {
                        if ui.selectable_label(c.sheet == *s, s).clicked() {
                            c.sheet = s.clone();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    label(ui, &t, "Format");
                    // the file's extension says which
                    let dwg = c.path.to_lowercase().ends_with(".dwg");
                    for (name, want) in [("AutoCAD 2000 DXF, mm", false), ("AutoCAD 2000 DWG, mm", true)] {
                        if ui
                            .selectable_label(dwg == want, egui::RichText::new(name).font(sans(12.0)))
                            .clicked()
                            && dwg != want
                        {
                            let stem = c
                                .path
                                .rsplit_once('.')
                                .map_or(c.path.clone(), |(s, _)| s.to_string());
                            c.path = format!("{stem}.{}", if want { "dwg" } else { "dxf" });
                        }
                    }
                });
                section(ui, &t, "Layers");
                egui::Grid::new("dxf-layers")
                    .spacing(vec2(8.0, 3.0))
                    .show(ui, |ui| {
                        for (k, (what, layer, on)) in c.layers.iter_mut().enumerate() {
                            let r = ui.checkbox(on, egui::RichText::new(*what).font(sans(12.0)));
                            places.push((format!("sheet:dxf:on:{k}"), r.rect));
                            let r = ui.add_enabled(
                                *on,
                                egui::TextEdit::singleline(layer)
                                    .font(mono(11.0))
                                    .min_size(vec2(200.0, 0.0))
                                    .desired_width(200.0),
                            );
                            places.push((format!("sheet:dxf:layer:{k}"), r.rect));
                            ui.end_row();
                        }
                    });
                ui.horizontal(|ui| {
                    label(ui, &t, "File");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut c.path)
                            .font(mono(11.0))
                            .desired_width(260.0),
                    );
                    places.push(("sheet:dxf:path".into(), r.rect));
                });
                ui.horizontal(|ui| {
                    let r = ui.add_sized(vec2(70.0, 26.0), egui::Button::new("OK"));
                    places.push(("sheet:dxf:ok".into(), r.rect));
                    ok = r.clicked();
                    let r = ui.add_sized(vec2(70.0, 26.0), egui::Button::new("Cancel"));
                    places.push(("sheet:dxf:cancel".into(), r.rect));
                    cancel = r.clicked();
                });
            },
        );
        if ok {
            self.sheet.out.push(SheetAct::WriteDxf(c));
        } else if !(close || cancel) {
            self.sheet.dxf_open = Some(c);
        }
    }

    /// Over the flat: bend centre lines (up and down apart), tangent lines, punch centres and the
    /// blank's extents, drawn where the camera puts them.
    fn sheet_flat_overlay(&mut self, ctx: &egui::Context) {
        if !self.sheet.flat_env {
            return;
        }
        let Some((cam, size)) = self.camera else { return };
        let ppp = ctx.pixels_per_point();
        let v = self.view_rect;
        let t = self.theme;
        let p = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::unique("sheet-flat"),
            ))
            .with_clip_rect(v);
        let Some(f) = self.sheet.flats.first().cloned() else {
            return;
        };
        let Some(frame) = f.frame else { return };
        let at = |q: [f64; 2]| -> Option<egui::Pos2> {
            let w = frame.point_to_world(forge_math::Point3::new(q[0], q[1], 0.0));
            let px = cam.pixel_of([w.x as f32, w.y as f32, w.z as f32], size.0, size.1)?;
            Some(v.min + vec2(px.0, px.1) / ppp)
        };
        let dashed = |a: egui::Pos2, b: egui::Pos2, c: Color32, w: f32| {
            p.extend(egui::Shape::dashed_line(&[a, b], Stroke::new(w, c), 9.0, 4.0));
        };
        if self.sheet.bend_lines {
            for b in &f.bends {
                let c = if b.angle >= 0.0 { UP } else { DOWN };
                if let (Some(a), Some(e)) = (at(b.a), at(b.b)) {
                    dashed(a, e, c, 1.6);
                    let mid = a + (e - a) * 0.5;
                    p.text(
                        mid + vec2(6.0, -10.0),
                        Align2::LEFT_BOTTOM,
                        format!(
                            "{} {:.0}° R{}",
                            if b.angle >= 0.0 { "UP" } else { "DOWN" },
                            b.angle.abs(),
                            super::fg::short(b.radius)
                        ),
                        sans(11.0),
                        c,
                    );
                }
            }
        }
        if self.sheet.tangents {
            for b in &f.bends {
                let d = [b.b[0] - b.a[0], b.b[1] - b.a[1]];
                let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
                if len == 0.0 {
                    continue;
                }
                let n = [-d[1] / len * b.allowance * 0.5, d[0] / len * b.allowance * 0.5];
                for s in [-1.0, 1.0] {
                    let q = |x: [f64; 2]| [x[0] + s * n[0], x[1] + s * n[1]];
                    if let (Some(a), Some(e)) = (at(q(b.a)), at(q(b.b))) {
                        p.line_segment([a, e], Stroke::new(1.0, t.ink2));
                    }
                }
            }
        }
        if self.sheet.tool_marks {
            for (q, id) in &f.tools {
                if let Some(c) = at(*q) {
                    p.line_segment(
                        [c - vec2(6.0, 0.0), c + vec2(6.0, 0.0)],
                        Stroke::new(1.2, t.focus),
                    );
                    p.line_segment(
                        [c - vec2(0.0, 6.0), c + vec2(0.0, 6.0)],
                        Stroke::new(1.2, t.focus),
                    );
                    p.text(c + vec2(7.0, 7.0), Align2::LEFT_TOP, id, mono(10.0), t.focus);
                }
            }
        }
        let (w, h) = (f.hi[0] - f.lo[0], f.hi[1] - f.lo[1]);
        if self.sheet.extents {
            let corners = [
                [f.lo[0], f.lo[1]],
                [f.hi[0], f.lo[1]],
                [f.hi[0], f.hi[1]],
                [f.lo[0], f.hi[1]],
            ];
            let pts: Vec<egui::Pos2> = corners.iter().filter_map(|c| at(*c)).collect();
            if pts.len() == 4 {
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.extend(egui::Shape::dashed_line(
                    &ring,
                    Stroke::new(1.0, t.ink2.gamma_multiply(0.8)),
                    4.0,
                    4.0,
                ));
            }
        }
        // the blank's facts, top left of the view, as Inventor's Extents says them
        let up = f.bends.iter().filter(|b| b.angle >= 0.0).count();
        let lines = [
            format!("Flat Pattern: {}", f.sheet),
            format!(
                "Extents  {} x {} mm",
                super::fg::short((w * 100.0).round() / 100.0),
                super::fg::short((h * 100.0).round() / 100.0)
            ),
            format!(
                "Bends  {} ({} up, {} down)",
                f.bends.len(),
                up,
                f.bends.len() - up
            ),
        ];
        let at = v.min + vec2(12.0, 44.0);
        let r = Rect::from_min_size(at, vec2(230.0, 62.0));
        p.rect_filled(r, 4.0, t.surface.gamma_multiply(0.9));
        p.rect_stroke(r, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        for (k, l) in lines.iter().enumerate() {
            p.text(
                at + vec2(10.0, 8.0 + k as f32 * 17.0),
                Align2::LEFT_TOP,
                l,
                if k == 0 { semi(12.0) } else { mono(11.0) },
                t.ink,
            );
        }
        self.places.push(("sheet:flat:extents".into(), r));
    }
}

/// A tool's outline in its preview box: round, obround, square, rectangle, D, keyhole, hex,
/// cluster; a form as its die circle or its louvre's slot, shaded.
fn tool_picture(p: &egui::Painter, r: Rect, tl: &super::sheet::LibTool, t: &super::theme::Theme) {
    let s = tl.sizes;
    let ext = match tl.kind {
        "keyhole" => s[2] + s[0] * 0.5 + s[1] * 0.5,
        "cluster" => s[1].max(s[2]) * s[3].max(s[4]),
        "hex" => s[0] * 1.155,
        _ => s.iter().take(2).cloned().fold(0.0, f64::max),
    };
    let k = (r.width() as f64 / ext.max(1e-6)) as f32;
    let c = r.center();
    let ink = Stroke::new(1.6, t.ink);
    let fill = t.on;
    match tl.kind {
        "round" | "dimple" | "extruded_hole" => {
            p.circle(c, s[0] as f32 * k * 0.5, fill, ink);
            if tl.kind != "round" {
                p.circle_stroke(c, s[0] as f32 * k * 0.3, Stroke::new(1.0, t.ink2));
            }
        }
        "square" => {
            p.rect(
                Rect::from_center_size(c, vec2(s[0] as f32, s[0] as f32) * k),
                0.0,
                fill,
                ink,
                StrokeKind::Middle,
            );
        }
        "rect" => {
            p.rect(
                Rect::from_center_size(c, vec2(s[0] as f32, s[1] as f32) * k),
                0.0,
                fill,
                ink,
                StrokeKind::Middle,
            );
        }
        "obround" | "louvre" | "lance" => {
            let sz = vec2(s[0] as f32, s[1] as f32) * k;
            let rad = if tl.kind == "obround" {
                (sz.y * 0.5) as u8
            } else {
                2
            };
            p.rect(Rect::from_center_size(c, sz), rad, fill, ink, StrokeKind::Middle);
        }
        "d" => {
            let rad = s[0] as f32 * k * 0.5;
            let flat = s[1] as f32 * k * 0.5;
            let pts: Vec<egui::Pos2> = (0..=48)
                .map(|i| {
                    let a = i as f32 / 48.0 * std::f32::consts::TAU;
                    let q = c + vec2(a.cos(), a.sin()) * rad;
                    pos2(q.x.clamp(c.x - flat, c.x + flat), q.y)
                })
                .collect();
            p.add(egui::Shape::closed_line(pts, ink));
        }
        "keyhole" => {
            let big = s[0] as f32 * k * 0.5;
            let slot = vec2(s[1] as f32 * k, s[2] as f32 * k);
            let top = c - vec2(0.0, slot.y * 0.5);
            p.circle(top, big, fill, ink);
            p.rect(
                Rect::from_min_size(pos2(c.x - slot.x * 0.5, top.y), slot),
                0.0,
                fill,
                ink,
                StrokeKind::Middle,
            );
        }
        "hex" => {
            let rad = s[0] as f32 * k * 0.577;
            let pts: Vec<egui::Pos2> = (0..6)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::FRAC_PI_3;
                    c + vec2(a.cos(), a.sin()) * rad
                })
                .collect();
            p.add(egui::Shape::convex_polygon(pts, fill, ink));
        }
        "cluster" => {
            let (d, px, py, nx, ny) = (
                s[0] as f32,
                s[1] as f32,
                s[2] as f32,
                s[3] as usize,
                s[4] as usize,
            );
            for i in 0..nx {
                for j in 0..ny {
                    let q = c + vec2(
                        (i as f32 - (nx - 1) as f32 * 0.5) * px,
                        (j as f32 - (ny - 1) as f32 * 0.5) * py,
                    ) * k;
                    p.circle(q, d * k * 0.5, fill, ink);
                }
            }
        }
        _ => {}
    }
}

impl FlatView {
    pub fn extents(&self) -> (f64, f64) {
        (self.hi[0] - self.lo[0], self.hi[1] - self.lo[1])
    }
}
