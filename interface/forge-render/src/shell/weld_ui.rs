//! Weldments in the app: the cut list as its panel shows it and CSV writes it (the `cutlist`
//! query's rows: identical members on one row, then the plates), and the Weldments tab's tools.

use super::Shell;
use super::fgasm::{dialog_frame, dialog_head};
use super::theme::{self, S1, S2};
use egui::{Rect, pos2, vec2};

/// One row of the cut list: identical members (section, lengths and end cuts, either way round).
#[derive(Clone, Debug, PartialEq)]
pub struct CutLine {
    pub qty: usize,
    pub section: String,
    /// The section's family in words (`tube`, `angle`).
    pub kind: String,
    /// Along the path between the end cuts.
    pub length: f64,
    /// Long point to long point (straight members).
    pub overall: Option<f64>,
    /// An arc member's radius and the degrees it turns between its end cuts.
    pub arc: Option<(f64, f64)>,
    /// Each end in words: `square`, `45 deg`, `cope to frame[0]`.
    pub ends: [String; 2],
    pub members: Vec<String>,
}

/// An end cap or a corner plate.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateLine {
    pub id: String,
    pub kind: String,
    pub t: f64,
    pub area: f64,
    pub on: Vec<String>,
}

/// A part's cut list: none when it has no members or plates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CutList {
    pub lines: Vec<CutLine>,
    pub plates: Vec<PlateLine>,
}

fn round(x: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (x * k).round() / k
}

fn num(x: f64, places: i32) -> String {
    forge_lang::values::fmt(round(x, places))
}

impl CutList {
    /// The part's weldments as the `cutlist` query groups them.
    pub fn of(w: &forge_lang::weld::Welds) -> CutList {
        let lines = forge_lang::weld::cutlist(w)
            .into_iter()
            .map(|r| {
                let end = |s: usize| {
                    if !r.copes[s].is_empty() {
                        return format!("cope to {}", r.copes[s].join(", "));
                    }
                    let cuts: Vec<String> = r.cut.ends[s]
                        .iter()
                        .filter(|e| e.angle > 1e-9)
                        .map(|e| format!("{} deg", num(e.angle, 2)))
                        .collect();
                    if cuts.is_empty() {
                        "square".into()
                    } else {
                        cuts.join(" + ")
                    }
                };
                CutLine {
                    qty: r.members.len(),
                    section: r.section.clone(),
                    kind: r.kind.to_string(),
                    length: r.cut.length,
                    overall: r.cut.overall,
                    arc: r.cut.arc,
                    ends: [end(0), end(1)],
                    members: r.members.clone(),
                }
            })
            .collect();
        let plates = w
            .plates
            .iter()
            .map(|p| PlateLine {
                id: p.id.clone(),
                kind: p.kind.to_string(),
                t: p.t,
                area: p.area,
                on: p.on.clone(),
            })
            .collect();
        CutList { lines, plates }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.plates.is_empty()
    }

    /// As CSV: a header, a row per cut-list line, then a row per plate. Lengths in mm, angles
    /// in degrees; a field with a comma or a quote is quoted.
    pub fn csv(&self) -> String {
        let field = |s: &str| {
            if s.contains([',', '"', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let mut out = String::from(
            "item,qty,section,kind,length_mm,overall_mm,arc_radius_mm,arc_deg,start,end,members\n",
        );
        for (k, l) in self.lines.iter().enumerate() {
            let row = [
                (k + 1).to_string(),
                l.qty.to_string(),
                l.section.clone(),
                l.kind.clone(),
                num(l.length, 3),
                l.overall.map_or(String::new(), |o| num(o, 3)),
                l.arc.map_or(String::new(), |a| num(a.0, 3)),
                l.arc.map_or(String::new(), |a| num(a.1, 2)),
                l.ends[0].clone(),
                l.ends[1].clone(),
                l.members.join(" "),
            ];
            let row: Vec<String> = row.iter().map(|s| field(s)).collect();
            out.push_str(&row.join(","));
            out.push('\n');
        }
        if !self.plates.is_empty() {
            out.push_str("\nplate,kind,t_mm,area_mm2,on\n");
            for p in &self.plates {
                let row = [
                    p.id.clone(),
                    p.kind.clone(),
                    num(p.t, 3),
                    num(p.area, 3),
                    p.on.join(" "),
                ];
                let row: Vec<String> = row.iter().map(|s| field(s)).collect();
                out.push_str(&row.join(","));
                out.push('\n');
            }
        }
        out
    }
}

/// Where a part's cut list is written: beside it, `NAME-cutlist.csv`.
pub fn csv_path(part: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(part);
    let stem = p
        .file_stem()
        .map_or("part".into(), |s| s.to_string_lossy().to_string());
    p.with_file_name(format!("{stem}-cutlist.csv"))
}

const W: f32 = 760.0;
const COLS: [(&str, f32); 7] = [
    ("Qty", 40.0),
    ("Section", 120.0),
    ("Length", 80.0),
    ("Overall", 80.0),
    ("Start", 110.0),
    ("End", 110.0),
    ("Members", 0.0),
];

impl Shell {
    /// The Cut List panel: its rows grouped as the `cutlist` query has them, the plates under
    /// them, Copy CSV and Export CSV.
    pub(super) fn cutlist_window(&mut self, ctx: &egui::Context) {
        let Some(fg) = &self.fg else { return };
        if !fg.cutlist_open {
            return;
        }
        let t = self.theme;
        let list = fg.cutlist.clone();
        let lit = fg.cutlist_lit;
        let mut light: Option<Option<usize>> = None;
        let path = fg.path.clone();
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let (mut close, mut copy, mut export) = (false, false, false);
        let w = W.min(v.width() - 32.0).max(420.0);
        let shown = egui::Window::new("Cut List")
            .id(egui::Id::unique("weld-cutlist"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .default_pos(pos2(v.right() - 16.0, v.bottom() - 12.0))
            .min_width(w)
            .max_width(w)
            .frame(dialog_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .show(ctx, |ui| {
                close = dialog_head(ui, &t, "cutlist", "Cut List", "weld:cutlist", &mut places);
                if list.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No members yet: Weldments > Members makes them from a sketch's lines.",
                        )
                        .font(theme::small())
                        .color(t.ink2),
                    );
                    return;
                }
                let members: usize = list.lines.iter().map(|l| l.qty).sum();
                ui.label(
                    egui::RichText::new(format!(
                        "{} members on {} rows · {} plates",
                        members,
                        list.lines.len(),
                        list.plates.len()
                    ))
                    .font(theme::caption())
                    .color(t.ink2),
                );
                ui.add_space(S1);
                egui::ScrollArea::vertical()
                    .max_height((v.height() * 0.5).max(160.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("weld-cutlist-grid")
                            .striped(true)
                            .spacing(vec2(S2, S1))
                            .num_columns(COLS.len())
                            .show(ui, |ui| {
                                for (c, wide) in COLS {
                                    let r = egui::RichText::new(c).font(theme::medium(12.0)).color(t.ink2);
                                    if wide > 0.0 {
                                        ui.add_sized(vec2(wide, 18.0), egui::Label::new(r));
                                    } else {
                                        ui.label(r);
                                    }
                                }
                                ui.end_row();
                                for (k, l) in list.lines.iter().enumerate() {
                                    let cell = |ui: &mut egui::Ui, s: String| {
                                        ui.label(egui::RichText::new(s).font(theme::small()).color(t.ink))
                                    };
                                    // a row lights its members in the view; again, off
                                    let on = lit == Some(k);
                                    let r = ui.selectable_label(
                                        on,
                                        egui::RichText::new(l.qty.to_string())
                                            .font(theme::small())
                                            .color(t.ink),
                                    );
                                    if r.clicked() {
                                        light = Some((!on).then_some(k));
                                    }
                                    let r = r.on_hover_text("Light these members in the view");
                                    places.push((format!("weld:cutlist:row:{k}"), r.rect));
                                    cell(ui, l.section.clone());
                                    cell(ui, num(l.length, 2));
                                    cell(
                                        ui,
                                        match (l.overall, l.arc) {
                                            (Some(o), _) => num(o, 2),
                                            (None, Some((r, a))) => format!("R{} {}°", num(r, 1), num(a, 1)),
                                            _ => "–".into(),
                                        },
                                    );
                                    cell(ui, l.ends[0].clone());
                                    cell(ui, l.ends[1].clone());
                                    ui.label(
                                        egui::RichText::new(l.members.join(", "))
                                            .font(theme::code())
                                            .color(t.ink2),
                                    );
                                    ui.end_row();
                                }
                            });
                        if !list.plates.is_empty() {
                            ui.add_space(S2);
                            ui.label(
                                egui::RichText::new("Plates")
                                    .font(theme::medium(12.0))
                                    .color(t.ink2),
                            );
                            for p in &list.plates {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} ({}): t {} · area {} mm² · on {}",
                                        p.id,
                                        p.kind,
                                        num(p.t, 2),
                                        num(p.area, 1),
                                        p.on.join(", ")
                                    ))
                                    .font(theme::small())
                                    .color(t.ink),
                                );
                            }
                        }
                    });
                ui.add_space(S2);
                ui.horizontal(|ui| {
                    let b = ui.button(egui::RichText::new("Copy CSV").font(theme::small()));
                    places.push(("weld:cutlist:copy".into(), b.rect));
                    copy = b.clicked();
                    let b = ui.add_enabled(
                        !path.is_empty(),
                        egui::Button::new(egui::RichText::new("Export CSV…").font(theme::small())),
                    );
                    places.push(("weld:cutlist:export".into(), b.rect));
                    export = b.clicked();
                });
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
            places.push(("weld:cutlist".into(), r.response.rect));
        }
        self.places.extend(places);
        if copy {
            ctx.copy_text(list.csv());
            self.status = "Cut list copied as CSV".into();
        }
        if export {
            let to = csv_path(&path);
            self.status = match std::fs::write(&to, list.csv()) {
                Ok(()) => format!("Cut list written to {}", to.display()),
                Err(e) => format!("Couldn't write {}: {e}", to.display()),
            };
        }
        if let Some(fg) = &mut self.fg {
            if let Some(l) = light {
                fg.cutlist_lit = l;
            }
            if close {
                fg.cutlist_open = false;
                fg.cutlist_lit = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> CutList {
        CutList {
            lines: vec![CutLine {
                qty: 2,
                section: "SHS 40x40x3".into(),
                kind: "tube".into(),
                length: 1_119.999_999,
                overall: Some(1200.0),
                arc: None,
                ends: ["45 deg".into(), "45 deg".into()],
                members: vec!["fr[0]".into(), "fr[2]".into()],
            }],
            plates: vec![PlateLine {
                id: "foot".into(),
                kind: "end_cap".into(),
                t: 5.0,
                area: 1600.0,
                on: vec!["fr[4]".into()],
            }],
        }
    }

    #[test]
    fn the_cut_list_writes_as_csv_with_its_plates_under_it() {
        assert_eq!(
            list().csv(),
            "item,qty,section,kind,length_mm,overall_mm,arc_radius_mm,arc_deg,start,end,members\n\
             1,2,SHS 40x40x3,tube,1120,1200,,,45 deg,45 deg,fr[0] fr[2]\n\
             \n\
             plate,kind,t_mm,area_mm2,on\n\
             foot,end_cap,5,1600,fr[4]\n"
        );
    }

    #[test]
    fn a_field_with_a_comma_is_quoted() {
        let mut l = list();
        l.lines[0].ends[0] = "cope to fr[1], fr[3]".into();
        assert!(l.csv().contains(",\"cope to fr[1], fr[3]\","), "{}", l.csv());
    }

    #[test]
    fn the_csv_goes_beside_the_part() {
        assert_eq!(
            csv_path("/w/table_frame.fg"),
            std::path::PathBuf::from("/w/table_frame-cutlist.csv")
        );
    }
}
