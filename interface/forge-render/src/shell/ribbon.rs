//! The ribbon's tabs, groups and buttons. A button with no action is drawn disabled: there are no
//! buttons that pretend.

use super::sketching::SkTool;
use super::theme::sans;
use super::{Action, RIBBON_H, Shell, TABS_H, TITLE_H, icons};
use crate::render::LineMode;
use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};

struct Tool {
    name: &'static str,
    icon: &'static str,
    action: Option<Action>,
    on: bool,
}

struct Group {
    name: &'static str,
    big: Vec<Tool>,
    small: Vec<Tool>,
}

const fn off(name: &'static str, icon: &'static str) -> Tool {
    Tool {
        name,
        icon,
        action: None,
        on: false,
    }
}

const fn go(name: &'static str, icon: &'static str, action: Action, on: bool) -> Tool {
    Tool {
        name,
        icon,
        action: Some(action),
        on,
    }
}

const fn new(name: &'static str, icon: &'static str, kind: &'static str) -> Tool {
    go(name, icon, Action::NewFeature(kind), false)
}

fn groups(s: &Shell) -> Vec<Group> {
    let g = |name, big, small| Group { name, big, small };
    let v = s.view;
    match s.tab {
        0 => vec![
            g(
                "Sketch",
                vec![go("Start Sketch", "sketch", Action::StartSketch, s.sketch_pick)],
                vec![],
            ),
            g(
                "Create",
                vec![
                    new("Extrude", "extrude", "extrude"),
                    new("Revolve", "revolve", "revolve"),
                ],
                // off till each starts from what's picked and has its test in tests/tools.rs
                vec![
                    off("Sweep", "sweep"),
                    off("Loft", "loft"),
                    off("Coil", "coil"),
                    off("Rib", "rib"),
                ],
            ),
            g(
                "Modify",
                vec![
                    new("Hole", "hole", "hole"),
                    new("Fillet", "fillet", "fillet"),
                    new("Chamfer", "chamfer", "chamfer"),
                ],
                vec![
                    off("Shell", "shell"),
                    new("Draft", "draft", "draft"),
                    new("Thread", "thread", "thread"),
                    off("Combine", "combine"),
                ],
            ),
            g(
                "Work Features",
                vec![off("Plane", "plane")],
                vec![off("Axis", "axis"), off("Point", "point")],
            ),
            g(
                "Pattern",
                vec![off("Linear", "rect")],
                vec![off("Circular", "circular"), off("Mirror", "mirror")],
            ),
        ],
        1 => {
            let on = s.sketch.as_ref().and_then(|m| m.tool);
            let t = |name, icon, tool: SkTool| Tool {
                name,
                icon,
                action: s.sketch_tool(tool),
                on: on == Some(tool),
            };
            vec![
                g(
                    "Create",
                    vec![
                        t("Line", "line", SkTool::Line),
                        t("Circle", "circle", SkTool::Circle),
                        t("Arc", "arc", SkTool::Arc),
                        t("Rectangle", "rectangle", SkTool::Rectangle),
                    ],
                    vec![off("Spline", "spline"), t("Point", "point", SkTool::Point)],
                ),
                g(
                    "Modify",
                    vec![],
                    vec![
                        off("Trim", "trim"),
                        off("Offset", "offset"),
                        off("Mirror", "mirror"),
                    ],
                ),
                g(
                    "Constrain",
                    vec![t("Dimension", "dim", SkTool::Dimension)],
                    vec![
                        t("Coincident", "coincident", SkTool::Coincident),
                        t("Parallel", "parallel", SkTool::Parallel),
                        t("Perpendicular", "perp", SkTool::Perpendicular),
                        t("Tangent", "tangent", SkTool::Tangent),
                    ],
                ),
                g(
                    "View",
                    vec![Tool {
                        name: "Look At",
                        icon: "lookat",
                        action: s.sketch.as_ref().map(|_| Action::LookAt),
                        on: false,
                    }],
                    vec![],
                ),
                g(
                    "Exit",
                    vec![Tool {
                        name: "Finish Sketch",
                        icon: "finish",
                        action: s.sketch.as_ref().map(|_| Action::FinishSketch),
                        on: false,
                    }],
                    vec![Tool {
                        name: "Cancel",
                        icon: "close",
                        action: s.sketch.as_ref().map(|_| Action::CancelSketch),
                        on: false,
                    }],
                ),
            ]
        }
        2 => vec![
            g(
                "Measure",
                vec![off("Distance", "measure")],
                vec![off("Wall", "measure"), off("Mass", "mass")],
            ),
            g(
                "Analysis",
                vec![off("Section", "section"), off("Check", "check")],
                vec![],
            ),
        ],
        3 => vec![
            g(
                "Lines",
                vec![
                    go(
                        "Near",
                        "lines",
                        Action::Lines(LineMode::Near),
                        v.lines == LineMode::Near,
                    ),
                    go(
                        "Far",
                        "lines",
                        Action::Lines(LineMode::Far),
                        v.lines == LineMode::Far,
                    ),
                    go(
                        "None",
                        "lines",
                        Action::Lines(LineMode::None),
                        v.lines == LineMode::None,
                    ),
                ],
                vec![],
            ),
            g(
                "Appearance",
                vec![
                    go("Section", "section", Action::Section, v.section),
                    off("Explode", "combine"),
                ],
                vec![],
            ),
            g(
                "Navigate",
                vec![go(
                    "Orbit",
                    "orbit",
                    Action::Tool(super::NavTool::Orbit),
                    v.tool == super::NavTool::Orbit,
                )],
                vec![
                    go(
                        "Pan",
                        "pan",
                        Action::Tool(super::NavTool::Pan),
                        v.tool == super::NavTool::Pan,
                    ),
                    go(
                        "Zoom",
                        "zoom",
                        Action::Tool(super::NavTool::Zoom),
                        v.tool == super::NavTool::Zoom,
                    ),
                    go("Fit All", "fit", Action::FitAll, false),
                ],
            ),
        ],
        _ => vec![
            g("Parameters", vec![off("Parameters", "params")], vec![]),
            g(
                "Document",
                vec![off("Save", "save"), go("Open", "open", Action::Open, false)],
                vec![],
            ),
        ],
    }
}

const BIG_W: f32 = 54.0;
const BIG_PAD: f32 = 12.0;
const SMALL_W: f32 = 92.0;
const SMALL_H: f32 = 27.0;

/// A big button is as wide as its label, its words a line each.
fn big_width(ui: &mut Ui, tool: &Tool) -> f32 {
    let text = ui
        .fonts_mut(|f| f.layout_no_wrap(tool.name.replace(' ', "\n"), sans(12.0), egui::Color32::WHITE))
        .size()
        .x;
    (text + BIG_PAD).max(BIG_W)
}

impl Shell {
    pub(super) fn ribbon(&mut self, ui: &mut Ui, w: f32) {
        let t = self.theme;
        let y = TITLE_H + TABS_H;
        let p = ui.painter().clone();
        p.rect_filled(
            Rect::from_min_size(pos2(0.0, y), vec2(w, RIBBON_H)),
            0.0,
            t.raised,
        );
        p.hline(0.0..=w, y, Stroke::new(1.0, t.line));
        p.hline(0.0..=w, y + RIBBON_H - 0.5, Stroke::new(1.0, t.line));
        let mut gx = 0.0;
        for g in groups(self) {
            let cols = g.small.len().div_ceil(3);
            let bigs: Vec<f32> = g.big.iter().map(|tool| big_width(ui, tool)).collect();
            let width = 12.0 + bigs.iter().sum::<f32>() + cols as f32 * SMALL_W;
            let width = width.max(
                ui.fonts_mut(|f| f.layout_no_wrap(g.name.to_string(), sans(11.0), t.ink2))
                    .size()
                    .x
                    + 16.0,
            );
            let mut x = gx + 6.0;
            for (tool, w) in g.big.iter().zip(&bigs) {
                self.big_button(ui, tool, x, y + 6.0, *w);
                x += w;
            }
            for (k, tool) in g.small.iter().enumerate() {
                self.small_button(
                    ui,
                    tool,
                    x + (k / 3) as f32 * SMALL_W,
                    y + 5.0 + (k % 3) as f32 * SMALL_H,
                );
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
    }

    fn tool_response(&mut self, ui: &mut Ui, tool: &Tool, r: Rect) -> (egui::Color32, egui::Response) {
        let t = self.theme;
        let sense = if tool.action.is_some() {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui
            .interact(r, ui.scope_id().with(("tool", self.tab, tool.name)), sense)
            .on_hover_text(tool.name);
        resp.widget_info(|| {
            egui::WidgetInfo::selected(egui::Role::Button, tool.action.is_some(), tool.on, tool.name)
        });
        let live = tool.action.is_some();
        let p = ui.painter();
        if tool.on {
            p.rect_filled(r, 4.0, t.on);
        } else if live && resp.hovered() {
            p.rect_filled(r, 4.0, t.hover);
        }
        if resp.has_focus() {
            p.rect_stroke(r, 4.0, Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
        }
        if resp.clicked()
            && let Some(a) = tool.action
        {
            self.out.push(a);
        }
        self.places.push((format!("tool:{}", tool.name), r));
        let ink = if tool.on {
            t.on_ink
        } else if live {
            t.ink
        } else {
            t.ink2.gamma_multiply(0.55)
        };
        (ink, resp)
    }

    fn big_button(&mut self, ui: &mut Ui, tool: &Tool, x: f32, y: f32, w: f32) {
        let r = Rect::from_min_size(pos2(x, y), vec2(w - 2.0, 72.0));
        let (ink, _) = self.tool_response(ui, tool, r);
        let g = ui.fonts_mut(|f| f.layout_no_wrap(tool.name.replace(' ', "\n"), sans(12.0), ink));
        let p = ui.painter();
        icons::draw(p, pos2(r.center().x - 14.0, y + 6.0), 28.0, tool.icon, 1.25, ink);
        p.galley(pos2(r.center().x - g.size().x * 0.5, y + 36.0), g, ink);
    }

    fn small_button(&mut self, ui: &mut Ui, tool: &Tool, x: f32, y: f32) {
        let r = Rect::from_min_size(pos2(x, y), vec2(SMALL_W - 2.0, 26.0));
        let (ink, _) = self.tool_response(ui, tool, r);
        let p = ui.painter();
        icons::draw(p, pos2(x + 4.0, y + 5.0), 16.0, tool.icon, 1.5, ink);
        p.text(
            pos2(x + 26.0, r.center().y),
            Align2::LEFT_CENTER,
            tool.name,
            sans(12.0),
            ink,
        );
    }
}
