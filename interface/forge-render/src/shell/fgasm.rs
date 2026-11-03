//! An assembly (.fg) in the app: its instance tree by occurrence path, the instance chosen (hide,
//! isolate, its part file and anchors), the joints with a slider for each drive (a SetParam), the
//! mobility, the expects, the BOM, and the overlap check on request. Typed results (redundant,
//! conflicts, interferences, intended contacts) are the panel's issues, each with a Show that
//! highlights what it names.

use super::Shell;
use super::fg::{FeatRow, FgAct, FgPanel, From, Status, TreeRow, short, tree_rows};
use super::theme::{medium, mono, sans};
use egui::{Align2, Rect, Sense, Ui, pos2, vec2};
use forge_lang::asm::{ExpectOut, Issue as AsmIssue, JointOut};
use forge_lang::error::{Literal, Op};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct OccRow {
    pub path: String,
    pub file: String,
    /// It has a solid to draw.
    pub drawn: bool,
}

#[derive(Default)]
pub struct AsmPanel {
    pub occ: Vec<OccRow>,
    pub joints: Vec<JointOut>,
    pub mobility: usize,
    pub bom: Vec<(String, usize)>,
    pub expects: Vec<ExpectOut>,
    /// The overlap check's counts once it ran: interferences, intended contacts.
    pub overlap: Option<(usize, usize)>,
    /// The overlap check is running.
    pub checking: bool,
    pub timings: String,
    /// The tree row chosen: an occurrence's path, or a subassembly's (all under it).
    pub selected: Option<String>,
    pub hidden: HashSet<String>,
    /// Only this row (and what's under it) shown.
    pub isolated: Option<String>,
    /// What an issue's Show picked out: occurrences, and joints or constraints by id.
    pub highlight: Vec<String>,
    pub highlight_items: Vec<String>,
    /// The chosen part's file and its anchors (or why not), when asked.
    pub anchors: Option<(String, Result<Vec<String>, String>)>,
    pub collapsed: HashSet<String>,
    /// A drive's slider value while it's moved, before it's let go.
    pub dragging: Option<(String, f64)>,
}

/// Whether `path` is the row `key` or under it.
pub fn under(path: &str, key: &str) -> bool {
    path == key || path.starts_with(&format!("{key}.")) || path.starts_with(&format!("{key}["))
}

impl AsmPanel {
    pub fn tree(&self) -> Vec<TreeRow> {
        let feats: Vec<FeatRow> = self
            .occ
            .iter()
            .map(|o| FeatRow {
                id: o.path.clone(),
                kind: part_word(&o.file),
                line: 0,
                status: Status::Ok,
                dims: vec![],
                reads: vec![],
            })
            .collect();
        tree_rows(&feats)
    }

    /// Whether the occurrence at `path` is drawn: not hidden, and inside what's isolated.
    pub fn shown(&self, path: &str) -> bool {
        !self.hidden.iter().any(|h| under(path, h)) && self.isolated.as_ref().is_none_or(|k| under(path, k))
    }

    pub fn picked(&self, path: &str) -> bool {
        self.selected.as_ref().is_some_and(|k| under(path, k))
    }

    pub fn flagged(&self, path: &str) -> bool {
        self.highlight.iter().any(|h| under(path, h))
    }
}

/// A part file's name as the tree shows it: its stem.
pub fn part_word(file: &str) -> String {
    let n = file.rsplit('/').next().unwrap_or(file);
    n.trim_end_matches(".fg").to_string()
}

/// What an assembly issue names, to highlight: occurrences and items.
pub fn named(issue: &AsmIssue) -> (Vec<String>, Vec<String>) {
    match issue {
        AsmIssue::Interference { a, b, .. } | AsmIssue::ShallowEngagement { a, b, .. } => {
            (vec![a.clone(), b.clone()], vec![])
        }
        AsmIssue::IntendedContact { a, b, by } => (vec![a.clone(), b.clone()], vec![by.clone()]),
        AsmIssue::Conflict { items, .. } => (vec![], items.iter().map(|i| i.id.clone()).collect()),
        AsmIssue::Redundant { item, implied_by, .. } => (
            vec![],
            std::iter::once(item.id.clone())
                .chain(implied_by.iter().map(|i| i.id.clone()))
                .collect(),
        ),
        AsmIssue::NotConverged { items, .. } => (vec![], items.iter().map(|i| i.id.clone()).collect()),
        AsmIssue::BrokenAnchor { item, .. }
        | AsmIssue::WrongReference { item, .. }
        | AsmIssue::SelfLink { item } => (vec![], vec![item.clone()]),
        AsmIssue::LimitViolated { joint, .. } | AsmIssue::BadFreedom { joint, .. } => {
            (vec![], vec![joint.clone()])
        }
        AsmIssue::Inside { path, issue } => {
            let (occ, items) = named(issue);
            (occ.into_iter().map(|o| format!("{path}.{o}")).collect(), items)
        }
        _ => (vec![], vec![]),
    }
}

impl Shell {
    /// The instance tree, in the side panel's place.
    pub(super) fn asm_tree(&mut self, ui: &mut Ui, area: Rect) {
        let Some(mut fg) = self.fg.take() else { return };
        let Some(a) = &mut fg.asm else {
            self.fg = Some(fg);
            return;
        };
        let t = self.theme;
        let rows = a.tree();
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut chosen = None;
        let mut toggle = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
        egui::ScrollArea::vertical()
            .id_salt("fg-asm-tree")
            .auto_shrink([false, false])
            .show_rows(
                &mut child,
                super::ROW_H,
                visible(&rows, &a.collapsed).len(),
                |ui, range| {
                    let shown = visible(&rows, &a.collapsed);
                    for &k in &shown[range] {
                        let row = &rows[k];
                        let (r, resp) =
                            ui.allocate_exact_size(vec2(area.width(), super::ROW_H), Sense::click());
                        let on = a.selected.as_deref() == Some(row.key.as_str());
                        let p = ui.painter();
                        if on {
                            p.rect_filled(r, 0.0, t.on);
                        } else if resp.hovered() {
                            p.rect_filled(r, 0.0, t.hover);
                        }
                        let x = r.left() + 8.0 + row.depth as f32 * 14.0;
                        if row.group {
                            let arrow = Rect::from_min_size(pos2(x - 2.0, r.top() + 5.0), vec2(14.0, 16.0));
                            let ar = ui.interact(
                                arrow,
                                ui.scope_id().with(("asm-open", &row.key)),
                                Sense::click(),
                            );
                            let open = !a.collapsed.contains(&row.key);
                            ui.painter().text(
                                arrow.center(),
                                Align2::CENTER_CENTER,
                                if open { "▾" } else { "▸" },
                                sans(12.0),
                                t.ink2,
                            );
                            if ar.clicked() {
                                toggle = Some(row.key.clone());
                            }
                        }
                        let ink = if !a.shown(&row.key) {
                            t.ink2.gamma_multiply(0.6)
                        } else if on {
                            t.on_ink
                        } else if a.flagged(&row.key) {
                            t.error
                        } else {
                            t.ink
                        };
                        let p = ui.painter();
                        p.text(
                            pos2(x + 16.0, r.center().y),
                            Align2::LEFT_CENTER,
                            &row.label,
                            if on { medium(13.0) } else { sans(13.0) },
                            ink,
                        );
                        if let Some(i) = row.feat {
                            p.text(
                                pos2(r.right() - 8.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                part_word(&a.occ[i].file),
                                sans(10.0),
                                t.ink2,
                            );
                        }
                        if resp.clicked() {
                            chosen = Some(row.key.clone());
                        }
                        places.push((format!("asm:row:{}", row.key), r));
                    }
                },
            );
        if let Some(k) = toggle
            && !a.collapsed.remove(&k)
        {
            a.collapsed.insert(k);
        }
        if let Some(k) = chosen {
            a.selected = if a.selected.as_deref() == Some(k.as_str()) {
                None
            } else {
                Some(k)
            };
            a.anchors = None;
            fg.out.push(FgAct::AsmShow);
        }
        self.places.extend(places);
        self.fg = Some(fg);
    }

    /// The assembly's sections of the panel: the instance chosen, joints and drives, DOF, expects,
    /// the BOM, the overlap check.
    pub(super) fn asm_panel(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let feats = fg.feats.clone();
        let Some(a) = &mut fg.asm else { return };
        let summary = format!(
            "{} occurrences · {} parts · DOF {} · {}",
            a.occ.len(),
            a.bom.len(),
            a.mobility,
            a.timings
        );
        ui.label(egui::RichText::new(summary).font(sans(11.0)).color(t.ink2));
        // the instance chosen
        if let Some(sel) = a.selected.clone() {
            Self::fg_heading(ui, t, &sel);
            let files: Vec<&OccRow> = a.occ.iter().filter(|o| under(&o.path, &sel)).collect();
            let one = files.len() == 1;
            if let Some(o) = files.first().filter(|_| one) {
                let r = ui.label(egui::RichText::new(&o.file).font(mono(11.0)).color(t.ink));
                places.push(("asm:file".into(), r.rect));
            } else {
                ui.label(
                    egui::RichText::new(format!("{} occurrences", files.len()))
                        .font(sans(11.0))
                        .color(t.ink2),
                );
            }
            ui.horizontal_wrapped(|ui| {
                let hidden = a.hidden.contains(&sel);
                if Self::fg_button(
                    ui,
                    if hidden { "Show" } else { "Hide" },
                    "asm:hide".into(),
                    places,
                ) {
                    if hidden {
                        a.hidden.remove(&sel);
                    } else {
                        a.hidden.insert(sel.clone());
                    }
                    out.push(FgAct::AsmShow);
                }
                let iso = a.isolated.as_deref() == Some(sel.as_str());
                if Self::fg_button(
                    ui,
                    if iso { "Show all" } else { "Isolate" },
                    "asm:isolate".into(),
                    places,
                ) {
                    a.isolated = if iso { None } else { Some(sel.clone()) };
                    out.push(FgAct::AsmShow);
                }
                if one && Self::fg_button(ui, "Anchors", "asm:anchors".into(), places) {
                    out.push(FgAct::AsmAnchors(files[0].file.clone()));
                }
            });
            if let Some((file, got)) = &a.anchors {
                match got {
                    Ok(list) => {
                        let r = ui.label(
                            egui::RichText::new(format!("{} anchors of {}", list.len(), part_word(file)))
                                .font(medium(11.0))
                                .color(t.ink),
                        );
                        places.push(("asm:anchorlist".into(), r.rect));
                        for x in list.iter().take(200) {
                            ui.label(egui::RichText::new(x).font(mono(10.0)).color(t.ink2));
                        }
                    }
                    Err(why) => {
                        ui.label(egui::RichText::new(why).font(sans(11.0)).color(t.error));
                    }
                }
            }
        } else if (a.isolated.is_some() || !a.hidden.is_empty())
            && Self::fg_button(ui, "Show all", "asm:showall".into(), places)
        {
            a.isolated = None;
            a.hidden.clear();
            out.push(FgAct::AsmShow);
        }
        // joints, each drive a slider
        Self::fg_heading(
            ui,
            t,
            &format!("Joints ({}), mobility {}", a.joints.len(), a.mobility),
        );
        for j in &a.joints {
            let flagged = a.highlight_items.contains(&j.id);
            let vals: Vec<String> = j
                .values
                .iter()
                .map(|(w, v, ang)| {
                    if *ang {
                        format!("{w} {}°", short(v.to_degrees()))
                    } else {
                        format!("{w} {}", short(*v))
                    }
                })
                .collect();
            let r = ui.label(
                egui::RichText::new(format!(
                    "{} {} · {} · free {}",
                    j.id,
                    j.kind,
                    vals.join(", "),
                    j.free
                ))
                .font(mono(11.0))
                .color(if flagged { t.error } else { t.ink }),
            );
            places.push((format!("asm:joint:{}", j.id), r.rect));
            // a drive: the joint's dimension a param drives
            let drives: Vec<(String, forge_lang::error::ParamKey, f64, forge_lang::values::Dim)> = feats
                .iter()
                .filter(|f| f.id == j.id)
                .flat_map(|f| &f.dims)
                .filter_map(|d| match &d.from {
                    From::Driving(k) => Some((d.what.clone(), k.clone(), d.value, d.dim)),
                    _ => None,
                })
                .collect();
            for (what, key, value, dim) in drives {
                let angle = dim == forge_lang::values::Dim::Angle;
                let (lo, hi) = j
                    .limits
                    .iter()
                    .find(|l| l.0 == what)
                    .map(|l| {
                        if angle {
                            (l.1.to_degrees(), l.2.to_degrees())
                        } else {
                            (l.1, l.2)
                        }
                    })
                    .unwrap_or(if angle {
                        (-180.0, 180.0)
                    } else {
                        (
                            value - 100.0f64.max(value.abs()),
                            value + 100.0f64.max(value.abs()),
                        )
                    });
                let name = format!("{}:{what}", j.id);
                let mut v = match &a.dragging {
                    Some((n, x)) if *n == name => *x,
                    _ => value,
                };
                let resp = ui.add(
                    egui::Slider::new(&mut v, lo.min(value)..=hi.max(value))
                        .text(format!("{key}"))
                        .suffix(if angle { "°" } else { " mm" }),
                );
                places.push((format!("asm:drive:{name}"), resp.rect));
                if resp.dragged() {
                    a.dragging = Some((name.clone(), v));
                }
                let let_go = resp.drag_stopped() || (resp.changed() && !resp.dragged());
                if let_go {
                    a.dragging = None;
                    if v != value {
                        out.push(FgAct::Apply(Op::SetParam {
                            param: key.clone(),
                            value: Literal { value: v, dim },
                        }));
                    }
                }
            }
        }
        // expects
        if a.expects.is_empty() && a.overlap.is_none() {
            Self::fg_heading(ui, t, "Expects");
            ui.label(
                egui::RichText::new("measured with the checks (Run checks below)")
                    .font(sans(11.0))
                    .color(t.ink2),
            );
        }
        if !a.expects.is_empty() {
            let fails = a.expects.iter().filter(|e| !e.ok).count();
            Self::fg_heading(
                ui,
                t,
                &format!("Expects: {} of {} pass", a.expects.len() - fails, a.expects.len()),
            );
            for (k, e) in a.expects.iter().enumerate() {
                let m = e.measured.map_or("-".to_string(), short);
                let l = e.limit.map_or("-".to_string(), short);
                let r = ui.label(
                    egui::RichText::new(format!(
                        "{} line {}: {} {m} (limit {l})",
                        if e.ok { "pass" } else { "FAIL" },
                        e.line,
                        e.metric
                    ))
                    .font(mono(10.0))
                    .color(if e.ok { t.ink2 } else { t.error }),
                );
                places.push((format!("asm:expect:{k}"), r.rect));
            }
        }
        // the overlap check and the expects, on request (slow on a big assembly)
        Self::fg_heading(ui, t, "Interference and expects");
        match (a.overlap, a.checking) {
            (_, true) => {
                ui.label(egui::RichText::new("checking…").font(sans(11.0)).color(t.ink2));
            }
            (Some((n, c)), false) => {
                ui.label(
                    egui::RichText::new(format!("{n} interferences, {c} intended contacts"))
                        .font(sans(11.0))
                        .color(if n > 0 { t.error } else { t.ink }),
                );
            }
            (None, false) => {
                if Self::fg_button(ui, "Run checks", "asm:check".into(), places) {
                    a.checking = true;
                    out.push(FgAct::AsmCheck);
                }
            }
        }
        // the BOM
        Self::fg_heading(
            ui,
            t,
            &format!("BOM ({} parts)", a.bom.iter().map(|b| b.1).sum::<usize>()),
        );
        for (file, n) in &a.bom {
            let r = ui.label(
                egui::RichText::new(format!("{n:>4} × {}", part_word(file)))
                    .font(mono(11.0))
                    .color(t.ink),
            );
            places.push((format!("asm:bom:{}", part_word(file)), r.rect));
        }
    }

    /// An assembly issue's Show: highlights what it names.
    pub(super) fn asm_show_button(
        ui: &mut Ui,
        fg_asm: &mut Option<AsmPanel>,
        issue: &AsmIssue,
        k: usize,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let Some(a) = fg_asm else { return };
        let (occ, items) = named(issue);
        if occ.is_empty() && items.is_empty() {
            return;
        }
        let on = a.highlight == occ && a.highlight_items == items;
        if Self::fg_button(
            ui,
            if on { "Hide highlight" } else { "Show" },
            format!("asm:show:{k}"),
            places,
        ) {
            if on {
                a.highlight.clear();
                a.highlight_items.clear();
            } else {
                a.highlight = occ;
                a.highlight_items = items;
            }
            out.push(FgAct::AsmShow);
        }
    }
}

/// The rows a collapsed group leaves showing, by index.
fn visible(rows: &[TreeRow], collapsed: &HashSet<String>) -> Vec<usize> {
    let mut out = vec![];
    let mut hidden: Option<&str> = None;
    for (k, r) in rows.iter().enumerate() {
        if let Some(h) = hidden {
            if under(&r.key, h) && r.key != h {
                continue;
            }
            hidden = None;
        }
        out.push(k);
        if r.group && collapsed.contains(&r.key) {
            hidden = Some(&r.key);
        }
    }
    out
}
