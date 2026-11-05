//! The Sketch tab as Inventor lays it out: a shape's variants under one split button (its arrow
//! drops them; the last one used becomes the face), the constraints each with its name.

use crate::shell::placed::Placed;
use super::super::sketching::SkTool;
use super::super::theme::{mono, sans};
use super::super::{Action, RIBBON_H, Shell, TABS_H, TITLE_H, icons};
use super::{BIG_PAD, BIG_W, SMALL_H, Tool, big_label, tips};
use egui::{Align2, Id, Rect, Sense, Stroke, Ui, pos2, vec2};

/// A button whose arrow drops its variants; one variant and no toggles: a plain button.
pub(super) struct Split {
    /// Names its arrow (`drop:Line`) and keeps its face.
    pub id: &'static str,
    /// The words under a big face; none: the face's own name.
    pub label: Option<&'static str>,
    pub variants: Vec<Tool>,
    /// Listed under the variants, never the face: switches (Line's Construction) and one-off
    /// commands (Automatic Dimensions).
    pub toggles: Vec<Tool>,
}

pub(super) struct SkGroup {
    pub name: &'static str,
    pub big: Vec<Split>,
    /// In columns of three.
    pub small: Vec<Split>,
}

const ARROW_W: f32 = 13.0;
const FACE_H: f32 = 56.0;
const ROW_H: f32 = 26.0;

fn one(t: Tool) -> Split {
    Split {
        id: t.name,
        label: None,
        variants: vec![t],
        toggles: vec![],
    }
}

fn split(id: &'static str, label: Option<&'static str>, variants: Vec<Tool>) -> Split {
    Split {
        id,
        label,
        variants,
        toggles: vec![],
    }
}

/// The tab's groups.
pub(super) fn groups(s: &Shell) -> Vec<SkGroup> {
    let on = s.sketch.as_ref().and_then(|m| m.tool);
    let more = s.sketch.as_ref().map(|m| m.more.clone()).unwrap_or_default();
    let t = |name, icon, tool: SkTool| Tool {
        name,
        icon,
        action: s.sketch_tool(tool),
        on: on == Some(tool),
    };
    let toggle = |name, icon, tool: SkTool, now: bool| Tool {
        name,
        icon,
        action: s.sketch_tool(tool),
        on: now,
    };
    let g = |name, big, small| SkGroup { name, big, small };
    let sketch = |name, icon, action: Action| Tool {
        name,
        icon,
        action: s.sketch.as_ref().map(|_| action),
        on: false,
    };
    vec![
        g(
            "Create",
            vec![
                Split {
                    id: "Line",
                    label: Some("Line"),
                    variants: vec![
                        t("Line", "line", SkTool::Line),
                        t("Midpoint Line", "linemid", SkTool::LineMid),
                    ],
                    toggles: vec![toggle(
                        "Construction",
                        "construction",
                        SkTool::Construction,
                        more.construction,
                    )],
                },
                split(
                    "Circle",
                    Some("Circle"),
                    vec![
                        t("Center Point Circle", "circle", SkTool::Circle),
                        t("Tangent Circle", "circletan", SkTool::CircleTan),
                        t("Three Point Circle", "circle3", SkTool::Circle3),
                    ],
                ),
                split(
                    "Arc",
                    Some("Arc"),
                    vec![
                        t("Three Point Arc", "arc3", SkTool::Arc3),
                        t("Tangent Arc", "arctan", SkTool::ArcTan),
                        t("Center Point Arc", "arc", SkTool::Arc),
                    ],
                ),
                split(
                    "Rectangle",
                    Some("Rectangle"),
                    vec![
                        t("Two Point Rectangle", "rectangle", SkTool::Rectangle),
                        t("Three Point Rectangle", "rect3", SkTool::Rect3),
                        t("Two Point Center Rectangle", "rectc", SkTool::RectCenter),
                        t("Three Point Center Rectangle", "rectc3", SkTool::RectCenter3),
                    ],
                ),
                split(
                    "Slot",
                    Some("Slot"),
                    vec![
                        t("Center to Center Slot", "slotcc", SkTool::SlotCC),
                        t("Overall Slot", "slot", SkTool::SlotOverall),
                        t("Center Point Slot", "slotc", SkTool::SlotCenter),
                        t("Three Point Arc Slot", "slotarc", SkTool::SlotArc3),
                        t("Center Point Arc Slot", "slotarcc", SkTool::SlotArcCenter),
                    ],
                ),
                one(t("Project Geometry", "project", SkTool::Project)),
            ],
            vec![
                split(
                    "Spline",
                    Some("Spline"),
                    vec![
                        t("Spline Interpolation", "spline", SkTool::Spline),
                        t("Spline Control Vertex", "splinecv", SkTool::SplineCv),
                    ],
                ),
                split(
                    "Ellipse",
                    Some("Ellipse"),
                    vec![
                        t("Center Point Ellipse", "ellipse", SkTool::Ellipse),
                        t("Axis Ellipse", "ellipseaxis", SkTool::EllipseAxis),
                        t("Bounding Box Ellipse", "ellipsebox", SkTool::EllipseBox),
                    ],
                ),
                split(
                    "Polygon",
                    Some("Polygon"),
                    vec![
                        t("Inscribed Polygon", "polygon", SkTool::Polygon),
                        t("Circumscribed Polygon", "polygonout", SkTool::PolygonOut),
                    ],
                ),
                one(t("Point", "point", SkTool::Point)),
                split(
                    "Fillet",
                    None,
                    vec![
                        t("Fillet", "skfillet", SkTool::SkFillet),
                        t("Chamfer", "skchamfer", SkTool::SkChamfer),
                    ],
                ),
                one(t("Text", "sktext", SkTool::Text)),
            ],
        ),
        // Inventor's Pattern panel in here too: the tab fits a 1600 wide window with every word
        g(
            "Modify",
            vec![],
            vec![
                one(t("Move", "move", SkTool::Move)),
                one(t("Copy", "copy", SkTool::Copy)),
                one(t("Rotate", "rotate", SkTool::Rotate)),
                one(t("Scale", "scale", SkTool::Scale)),
                one(t("Stretch", "stretch", SkTool::Stretch)),
                one(t("Offset", "offset", SkTool::Offset)),
                split(
                    "Trim",
                    None,
                    vec![
                        t("Trim", "trim", SkTool::Trim),
                        t("Extend", "extend", SkTool::Extend),
                        t("Split", "split", SkTool::Split),
                    ],
                ),
                one(t("Mirror", "mirror", SkTool::Mirror)),
                split(
                    "Pattern",
                    None,
                    vec![
                        t("Rectangular", "skrect", SkTool::RectPattern),
                        t("Circular", "skcircular", SkTool::CircPattern),
                    ],
                ),
            ],
        ),
        g(
            "Constrain",
            vec![Split {
                id: "Dimension",
                label: Some("Dimension"),
                variants: vec![t("Dimension", "dim", SkTool::Dimension)],
                toggles: vec![t("Automatic Dimensions", "autodim", SkTool::AutoDim)],
            }],
            vec![
                one(t("Coincident", "coincident", SkTool::Coincident)),
                one(t("Collinear", "collinear", SkTool::Collinear)),
                one(t("Concentric", "concentric", SkTool::Concentric)),
                one(t("Fix", "fix", SkTool::Fix)),
                one(t("Parallel", "parallel", SkTool::Parallel)),
                one(t("Perpendicular", "perp", SkTool::Perpendicular)),
                one(t("Horizontal", "horizontal", SkTool::Horizontal)),
                one(t("Vertical", "vertical", SkTool::Vertical)),
                one(t("Tangent", "tangent", SkTool::Tangent)),
                one(super::off("Smooth (G2)", "smooth")),
                one(t("Symmetric", "symmetric", SkTool::Symmetric)),
                one(t("Equal", "equal", SkTool::Equal)),
            ],
        ),
        g(
            "Format",
            vec![],
            vec![
                one(toggle(
                    "Construction",
                    "construction",
                    SkTool::Construction,
                    more.construction,
                )),
                one(toggle(
                    "Show Constraints",
                    "constrainshow",
                    SkTool::ShowConstraints,
                    !more.hide_glyphs,
                )),
                one(toggle("Show Freedom", "freedom", SkTool::ShowDof, more.show_dof)),
            ],
        ),
        g(
            "Exit",
            vec![one(sketch("Finish Sketch", "finish", Action::FinishSketch))],
            vec![
                one(sketch("Look At", "lookat", Action::LookAt)),
                one(sketch("Cancel", "close", Action::CancelSketch)),
            ],
        ),
    ]
}

fn face_key(id: &str) -> Id {
    Id::unique(("sketch-face", id))
}

fn open_key() -> Id {
    Id::unique("sketch-drop")
}

/// Shuts an open dropdown; true if one was.
pub(in crate::shell) fn close_drop(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| {
        let was = d.get_temp::<(&'static str, Rect)>(open_key()).is_some();
        d.remove::<(&'static str, Rect)>(open_key());
        was
    })
}

/// The variant on a split's face: the one in use, else the last used.
fn face(ui: &Ui, sp: &Split) -> usize {
    if let Some(k) = sp.variants.iter().position(|t| t.on) {
        ui.ctx().data_mut(|d| d.insert_temp(face_key(sp.id), k));
        return k;
    }
    ui.ctx()
        .data(|d| d.get_temp::<usize>(face_key(sp.id)))
        .unwrap_or(0)
        .min(sp.variants.len() - 1)
}

fn drops(sp: &Split) -> bool {
    sp.variants.len() > 1 || !sp.toggles.is_empty()
}

fn text_w(ui: &mut Ui, s: &str) -> f32 {
    ui.fonts_mut(|f| f.layout_no_wrap(s.to_string(), sans(12.0), egui::Color32::WHITE))
        .size()
        .x
}

fn big_w(ui: &mut Ui, sp: &Split) -> f32 {
    let label = sp.label.unwrap_or(sp.variants[0].name);
    let w = ui
        .fonts_mut(|f| f.layout_no_wrap(big_label(label), sans(12.0), egui::Color32::WHITE))
        .size()
        .x;
    (w + BIG_PAD).max(BIG_W)
}

/// A column of small buttons is as wide as its longest label, with its arrows.
fn col_w(ui: &mut Ui, col: &[Split], words: bool) -> f32 {
    col.iter()
        .map(|sp| {
            let names = sp.variants.iter().map(|t| t.name);
            let text = if words {
                sp.label
                    .map(|l| text_w(ui, l))
                    .unwrap_or_else(|| names.map(|n| text_w(ui, n)).fold(0.0, f32::max))
            } else {
                0.0
            };
            let arrow = if drops(sp) { ARROW_W } else { 0.0 };
            text + if words { 31.0 } else { 26.0 } + arrow
        })
        .fold(if words { 60.0 } else { 30.0 }, f32::max)
}

impl Shell {
    pub(super) fn sketch_ribbon(&mut self, ui: &mut Ui, w: f32) {
        let t = self.theme;
        let y = TITLE_H + TABS_H;
        let p = ui.painter().clone();
        let gs = groups(self);
        let width_of = |ui: &mut Ui, g: &SkGroup, words: bool| {
            12.0 + g.big.iter().map(|sp| big_w(ui, sp)).sum::<f32>()
                + g.small.chunks(3).map(|c| col_w(ui, c, words)).sum::<f32>()
        };
        // too wide for the window: small buttons lose their words, a group at a time, the
        // constraints' last
        let mut words = vec![true; gs.len()];
        for name in ["Modify", "Format", "Exit", "Create", "Constrain"] {
            let total: f32 = gs.iter().zip(&words).map(|(g, &k)| width_of(ui, g, k)).sum();
            if total <= w {
                break;
            }
            if let Some(k) = gs.iter().position(|g| g.name == name) {
                words[k] = false;
            }
        }
        let mut anchors: Vec<(&'static str, Rect)> = vec![];
        let mut gx = 0.0;
        for (g, &words) in gs.iter().zip(&words) {
            let label_w = ui
                .fonts_mut(|f| f.layout_no_wrap(g.name.to_string(), sans(11.0), t.ink2))
                .size()
                .x
                + 16.0;
            let width = width_of(ui, g, words).max(label_w);
            let mut x = gx + 6.0;
            for sp in &g.big {
                let bw = big_w(ui, sp);
                if let Some(a) = self.big_split(ui, sp, x, y + 6.0, bw) {
                    anchors.push((sp.id, a));
                }
                x += bw;
            }
            for col in g.small.chunks(3) {
                let cw = col_w(ui, col, words);
                for (k, sp) in col.iter().enumerate() {
                    if let Some(a) = self.small_split(ui, sp, x, y + 5.0 + k as f32 * SMALL_H, cw, words) {
                        anchors.push((sp.id, a));
                    }
                }
                x += cw;
            }
            p.text(
                pos2(gx + width * 0.5, y + RIBBON_H - 9.0),
                Align2::CENTER_CENTER,
                g.name,
                sans(11.0),
                t.ink2,
            );
            p.vline(gx + width, y..=y + RIBBON_H - 1.0, Stroke::new(1.0, t.line));
            gx += width;
        }
        let open = ui.ctx().data(|d| d.get_temp::<(&'static str, Rect)>(open_key()));
        if let Some((id, arrow)) = open
            && let Some(sp) = gs
                .iter()
                .flat_map(|g| g.big.iter().chain(&g.small))
                .find(|sp| sp.id == id)
        {
            let at = anchors.iter().find(|a| a.0 == id).map_or(arrow, |a| a.1);
            self.drop_menu(ui.ctx(), sp, at, arrow);
        }
    }

    /// The arrow that drops a split's variants.
    fn drop_arrow(&mut self, ui: &mut Ui, sp: &Split, r: Rect) {
        let t = self.theme;
        let live = sp.variants.iter().chain(&sp.toggles).any(|v| v.action.is_some());
        let sense = if live { Sense::click() } else { Sense::hover() };
        let resp = ui.interact(r, ui.scope_id().with(("drop", sp.id)), sense);
        let open = ui
            .ctx()
            .data(|d| d.get_temp::<(&'static str, Rect)>(open_key()))
            .is_some_and(|o| o.0 == sp.id);
        let p = ui.painter();
        if open {
            p.rect_filled(r, 3.0, t.on);
        } else if live && resp.hovered() {
            p.rect_filled(r, 3.0, t.hover);
        }
        let ink = if open {
            t.on_ink
        } else if live {
            t.ink
        } else {
            t.ink2.gamma_multiply(0.55)
        };
        let c = r.center();
        p.add(egui::Shape::convex_polygon(
            vec![c + vec2(-3.5, -1.5), c + vec2(3.5, -1.5), c + vec2(0.0, 2.5)],
            ink,
            Stroke::NONE,
        ));
        let resp = resp.on_hover_text(format!("More {} tools", sp.id.to_lowercase()));
        self.places.push((format!("drop:{}", sp.id), r));
        if resp.clicked() {
            ui.ctx().data_mut(|d| {
                if open {
                    d.remove::<(&'static str, Rect)>(open_key());
                } else {
                    d.insert_temp(open_key(), (sp.id, r));
                }
            });
        }
    }

    /// A big split button: its face, then its arrow under the words. Where its menu drops from.
    fn big_split(&mut self, ui: &mut Ui, sp: &Split, x: f32, y: f32, w: f32) -> Option<Rect> {
        let k = face(ui, sp);
        let tool = &sp.variants[k];
        if !drops(sp) {
            self.big_button(ui, tool, x, y, w);
            return None;
        }
        let whole = Rect::from_min_size(pos2(x, y), vec2(w - 2.0, 72.0));
        let r = Rect::from_min_size(pos2(x, y), vec2(w - 2.0, FACE_H));
        let (ink, _) = self.tool_response(ui, tool, r);
        let label = sp.label.unwrap_or(tool.name);
        let g = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), sans(12.0), ink));
        let p = ui.painter();
        icons::draw(p, pos2(r.center().x - 14.0, y + 6.0), 28.0, tool.icon, 1.25, ink);
        p.galley(pos2(r.center().x - g.size().x * 0.5, y + 36.0), g, ink);
        let arrow = Rect::from_min_size(pos2(x, y + FACE_H), vec2(w - 2.0, 72.0 - FACE_H));
        self.drop_arrow(ui, sp, arrow);
        Some(whole)
    }

    fn small_split(&mut self, ui: &mut Ui, sp: &Split, x: f32, y: f32, w: f32, words: bool) -> Option<Rect> {
        let k = face(ui, sp);
        let tool = &sp.variants[k];
        let dropping = drops(sp);
        let label = sp.label.unwrap_or(tool.name);
        // the arrow right after the words, as Inventor has it
        let face_w = match (dropping, words) {
            (false, _) => w - 2.0,
            (true, true) => (text_w(ui, label) + 30.0).min(w - 2.0 - ARROW_W),
            (true, false) => w - 2.0 - ARROW_W,
        };
        let r = Rect::from_min_size(pos2(x, y), vec2(face_w, ROW_H));
        let (ink, _) = self.tool_response(ui, tool, r);
        let p = ui.painter();
        icons::draw(p, pos2(x + 4.0, y + 5.0), 16.0, tool.icon, 1.5, ink);
        if words {
            p.text(
                pos2(x + 26.0, r.center().y),
                Align2::LEFT_CENTER,
                label,
                sans(12.0),
                ink,
            );
        }
        if !dropping {
            return None;
        }
        let arrow = Rect::from_min_size(pos2(x + face_w, y), vec2(ARROW_W, ROW_H));
        let whole = Rect::from_min_size(pos2(x, y), vec2(face_w + ARROW_W, ROW_H));
        self.drop_arrow(ui, sp, arrow);
        Some(whole)
    }

    /// A split's variants (and toggles) in a list under it; a click runs one and makes it the face.
    fn drop_menu(&mut self, ctx: &egui::Context, sp: &Split, below: Rect, arrow: Rect) {
        let t = self.theme;
        let face_now = sp.variants.iter().position(|v| v.on);
        let mut picked: Option<(usize, Action, bool)> = None;
        let placed_before = self.places.len();
        let (area, sizing) = egui::Area::new(Id::unique(("sketch-drop-menu", sp.id)))
            .fixed_pos(below.left_bottom() + vec2(0.0, 2.0))
            .order(egui::Order::Foreground)
            .fade_in(false)
            .show_sizing(ctx, |ui| {
                egui::Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(1.0, t.line))
                    .corner_radius(4)
                    .inner_margin(4)
                    .show(ui, |ui| {
                        let names = sp.variants.iter().chain(&sp.toggles).map(|v| v.name);
                        let widest = names.map(|n| text_w(ui, n)).fold(0.0, f32::max);
                        let row_w = widest + 100.0;
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let rows = sp
                            .variants
                            .iter()
                            .map(|v| (v, false))
                            .chain(sp.toggles.iter().map(|v| (v, true)));
                        for (k, (v, is_toggle)) in rows.enumerate() {
                            if is_toggle && k == sp.variants.len() {
                                let (r, _) = ui.allocate_exact_size(vec2(row_w, 7.0), Sense::hover());
                                ui.painter()
                                    .hline(r.x_range(), r.center().y, Stroke::new(1.0, t.line));
                            }
                            let sense = if v.action.is_some() {
                                Sense::click()
                            } else {
                                Sense::hover()
                            };
                            let (r, resp) = ui.allocate_exact_size(vec2(row_w, ROW_H), sense);
                            let resp = self.tool_tip(resp, v);
                            let live = v.action.is_some();
                            let on = if is_toggle { false } else { face_now == Some(k) };
                            let p = ui.painter();
                            if on {
                                p.rect_filled(r, 3.0, t.on);
                            } else if live && resp.hovered() {
                                p.rect_filled(r, 3.0, t.hover);
                            }
                            let ink = if on {
                                t.on_ink
                            } else if live {
                                t.ink
                            } else {
                                t.ink2.gamma_multiply(0.55)
                            };
                            let switch = matches!(
                                v.action,
                                Some(Action::SketchTool(
                                    SkTool::Construction | SkTool::ShowConstraints | SkTool::ShowDof
                                ))
                            );
                            if switch {
                                // a box, ticked when it's on
                                let b = Rect::from_center_size(
                                    pos2(r.left() + 13.0, r.center().y),
                                    vec2(12.0, 12.0),
                                );
                                p.rect_stroke(b, 2.0, Stroke::new(1.25, ink), egui::StrokeKind::Inside);
                                if v.on {
                                    p.line(
                                        vec![
                                            b.min + vec2(2.5, 6.0),
                                            b.min + vec2(5.0, 9.0),
                                            b.min + vec2(9.5, 3.0),
                                        ],
                                        Stroke::new(1.75, ink),
                                    );
                                }
                            } else {
                                icons::draw(p, pos2(r.left() + 5.0, r.top() + 5.0), 16.0, v.icon, 1.5, ink);
                            }
                            p.text(
                                pos2(r.left() + 28.0, r.center().y),
                                Align2::LEFT_CENTER,
                                v.name,
                                sans(12.0),
                                ink,
                            );
                            let key =
                                tips::tip(tips::scope(self), v.name).and_then(|x| tips::key_of(self, x.1));
                            if let Some(key) = key {
                                p.text(
                                    pos2(r.right() - 8.0, r.center().y),
                                    Align2::RIGHT_CENTER,
                                    key,
                                    mono(11.0),
                                    if on { t.on_ink } else { t.ink2 },
                                );
                            }
                            self.places.push((format!("tool:{}", v.name), r));
                            if resp.clicked()
                                && let Some(a) = v.action
                            {
                                picked = Some((k, a, is_toggle));
                            }
                        }
                    });
            });
        // what it named unseen, sizing itself, is off
        if sizing {
            self.places.truncate(placed_before);
        }
        let menu = area.response.rect;
        self.view_blocks.push(menu);
        if let Some((k, a, is_toggle)) = picked {
            self.out.push(a);
            if !is_toggle {
                ctx.data_mut(|d| d.insert_temp(face_key(sp.id), k));
            }
            close_drop(ctx);
            return;
        }
        // a click anywhere else shuts it
        let elsewhere = ctx.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|q| !menu.contains(q) && !arrow.contains(q))
        });
        if elsewhere {
            close_drop(ctx);
        }
    }
}
