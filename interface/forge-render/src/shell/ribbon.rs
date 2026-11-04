//! The ribbon's tabs, groups and buttons. A button with no action is drawn disabled: there are no
//! buttons that pretend.

use super::sketching::SkTool;
use super::theme::sans;
use super::{Action, FgCmd, RIBBON_H, Shell, TABS_H, TITLE_H, icons};
use crate::render::LineMode;
use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};

pub(super) struct Tool {
    pub(super) name: &'static str,
    pub(super) icon: &'static str,
    pub(super) action: Option<Action>,
    pub(super) on: bool,
}

pub(super) struct Group {
    pub(super) name: &'static str,
    pub(super) big: Vec<Tool>,
    pub(super) small: Vec<Tool>,
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

/// An assembly's Assemble and Inspect tabs, Inventor's: the commands this app has.
fn asm_groups(s: &Shell) -> Option<Vec<Group>> {
    let g = |name, big, small| Group { name, big, small };
    let a = s.fg.as_ref()?.asm.as_ref();
    let fg = |name, icon, c: FgCmd, on: bool| Tool {
        name,
        icon,
        action: a.map(|_| Action::Fg(c)),
        on,
    };
    let params = s.fg.as_ref().is_some_and(|f| f.params_open);
    let tool = a.and_then(|a| a.tool.as_ref());
    let up = |k: &str| match tool {
        Some(super::asmtools::AsmTool::Place(_)) => k == "place",
        Some(super::asmtools::AsmTool::Pattern(_)) => k == "pattern",
        Some(super::asmtools::AsmTool::MirrorCopy(m)) => k == if m.mirror { "mirror" } else { "copy" },
        Some(super::asmtools::AsmTool::Free(r)) => k == if *r { "rotate" } else { "move" },
        Some(super::asmtools::AsmTool::Link(f)) => k == if f.joint { "joint" } else { "constrain" },
        None => false,
    };
    match s.tab {
        0 => Some(vec![
            g(
                "Component",
                vec![fg("Place", "part", FgCmd::Place, up("place"))],
                vec![
                    fg("Pattern", "rect", FgCmd::Pattern, up("pattern")),
                    fg("Mirror", "mirror", FgCmd::Mirror, up("mirror")),
                    fg("Copy", "subasm", FgCmd::Copy, up("copy")),
                ],
            ),
            g(
                "Position",
                vec![
                    fg("Free Move", "move", FgCmd::FreeMove, up("move")),
                    fg("Free Rotate", "revolve", FgCmd::FreeRotate, up("rotate")),
                ],
                vec![],
            ),
            g(
                "Relationships",
                vec![
                    fg("Joint", "jrevolute", FgCmd::Joint, up("joint")),
                    fg("Constrain", "jmate", FgCmd::Constrain, up("constrain")),
                ],
                vec![],
            ),
            g(
                "Manage",
                vec![
                    fg(
                        "Bill\u{a0}of Materials",
                        "rule",
                        FgCmd::Bom,
                        a.is_some_and(|a| a.bom_open),
                    ),
                    go("Parameters", "params", Action::Fg(FgCmd::Parameters), params),
                ],
                vec![],
            ),
            g(
                "Productivity",
                vec![fg(
                    "Explode",
                    "combine",
                    FgCmd::Explode,
                    a.is_some_and(|a| a.explode_open),
                )],
                vec![],
            ),
        ]),
        2 => Some(
            vec![g(
                "Interference",
                vec![fg(
                    "Analyze Interference",
                    "check",
                    FgCmd::Interference,
                    a.is_some_and(|a| a.interference),
                )],
                vec![],
            )]
            .into_iter()
            .chain(inspect_groups(s))
            .collect(),
        ),
        _ => None,
    }
}

/// The Inspect tab's Measure, Analysis and Properties: a Forge document's.
fn inspect_groups(s: &Shell) -> Vec<Group> {
    use super::inspect::InspectCmd as C;
    let i = &s.inspect;
    let t = |name, icon, c: C, on: bool| Tool {
        name,
        icon,
        action: s.fg.as_ref().map(|_| Action::Inspect(c)),
        on,
    };
    let measuring = i.measure.as_ref().map(|m| m.wall);
    let props = i.props.as_ref().map(|p| p.tab);
    vec![
        Group {
            name: "Measure",
            big: vec![t("Measure", "measure", C::Measure, measuring == Some(false))],
            small: vec![
                t("Wall", "measure", C::Wall, measuring == Some(true)),
                t("Mass", "mass", C::Mass, props == Some(5)),
                t("CG Marker", "mass", C::Cg, i.cg),
            ],
        },
        Group {
            name: "Analysis",
            big: vec![
                t("Section", "section", C::Section, i.cut.is_some()),
                off("Check", "check"),
            ],
            small: vec![],
        },
        Group {
            name: "Properties",
            big: vec![
                t(
                    "iProperties",
                    "params",
                    C::Properties,
                    props.is_some_and(|p| p != 5),
                ),
                t("Materials", "mass", C::Materials, i.browser.is_some()),
            ],
            small: vec![],
        },
    ]
}

fn groups(s: &Shell) -> Vec<Group> {
    let g = |name, big, small| Group { name, big, small };
    let v = s.view;
    if s.is_asm()
        && let Some(gs) = asm_groups(s)
    {
        return gs;
    }
    let asm_explode = s.fg.as_ref().and_then(|f| f.asm.as_ref()).map(|a| a.explode_open);
    let fg_open = s.fg.is_some();
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
                    new("Sweep", "sweep", "sweep"),
                    new("Loft", "loft", "loft"),
                    new("Coil", "coil", "coil"),
                    new("Rib", "rib", "rib"),
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
                    new("Shell", "shell", "shell"),
                    new("Draft", "draft", "draft"),
                    new("Thread", "thread", "thread"),
                    new("Combine", "combine", "combine"),
                ],
            ),
            g(
                "Work Features",
                vec![new("Plane", "plane", "plane")],
                vec![new("Axis", "axis", "axis"), new("Point", "point", "point")],
            ),
            g(
                "Pattern",
                vec![new("Linear", "rect", "pattern")],
                vec![off("Circular", "circular"), off("Mirror", "mirror")],
            ),
        ]
        .into_iter()
        // a Forge part has sheet metal: its own tab, as Inventor's Convert to Sheet Metal gives
        .chain((s.fg.is_some() && !s.sheet.env).then(|| {
            g(
                "Convert",
                vec![go(
                    "Convert to Sheet Metal",
                    "smface",
                    Action::Sheet(super::sheet::SheetCmd::Convert),
                    false,
                )],
                vec![],
            )
        }))
        .collect(),
        super::SHEET_TAB => super::sheetui::sheet_groups(s),
        super::FLAT_TAB => super::sheetui::flat_groups(s),
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
                        t("Project Geometry", "project", SkTool::Project),
                    ],
                    vec![
                        t("Spline", "spline", SkTool::Spline),
                        t("Point", "point", SkTool::Point),
                    ],
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
        2 => inspect_groups(s),
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
                    go("Pastel Parts", "sun", Action::PartColours(!v.pastel), v.pastel),
                    Tool {
                        name: "Explode",
                        icon: "combine",
                        action: asm_explode.map(|_| Action::Fg(FgCmd::Explode)),
                        on: asm_explode == Some(true),
                    },
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
            g(
                "Parameters",
                vec![Tool {
                    name: "Parameters",
                    icon: "params",
                    action: fg_open.then_some(Action::Fg(FgCmd::Parameters)),
                    on: s.fg.as_ref().is_some_and(|f| f.params_open),
                }],
                vec![],
            ),
            g(
                "Document",
                vec![
                    Tool {
                        name: "Save",
                        icon: "save",
                        action: fg_open.then_some(Action::Fg(FgCmd::Save)),
                        on: false,
                    },
                    go("Open", "open", Action::Open, false),
                ],
                vec![],
            ),
            g(
                "Units",
                forge_doc::units::DOC_LENGTHS
                    .into_iter()
                    .map(|u| go(u, "measure", Action::SetUnits(u), s.units == u))
                    .collect(),
                vec![],
            ),
        ],
    }
}

const BIG_W: f32 = 54.0;
const BIG_PAD: f32 = 12.0;
const SMALL_W: f32 = 92.0;
const SMALL_H: f32 = 27.0;

/// A big button's label in two lines at most, split where the longer is shortest.
fn big_label(name: &str) -> String {
    let spaces: Vec<usize> = name.match_indices(' ').map(|m| m.0).collect();
    match spaces.iter().min_by_key(|&&k| k.max(name.len() - k - 1)) {
        Some(&k) => format!("{}\n{}", &name[..k], &name[k + 1..]),
        None => name.to_string(),
    }
}

/// A big button is as wide as its label.
fn big_width(ui: &mut Ui, tool: &Tool) -> f32 {
    let text = ui
        .fonts_mut(|f| f.layout_no_wrap(big_label(tool.name), sans(12.0), egui::Color32::WHITE))
        .size()
        .x;
    (text + BIG_PAD).max(BIG_W)
}

/// A column of small buttons is as wide as its longest label.
fn small_width(ui: &mut Ui, tools: &[Tool]) -> f32 {
    tools
        .iter()
        .map(|t| {
            ui.fonts_mut(|f| f.layout_no_wrap(t.name.to_string(), sans(12.0), egui::Color32::WHITE))
                .size()
                .x
                + 34.0
        })
        .fold(SMALL_W, f32::max)
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
            let bigs: Vec<f32> = g.big.iter().map(|tool| big_width(ui, tool)).collect();
            let cols: Vec<f32> = g.small.chunks(3).map(|c| small_width(ui, c)).collect();
            let width = 12.0 + bigs.iter().sum::<f32>() + cols.iter().sum::<f32>();
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
                let left: f32 = cols[..k / 3].iter().sum();
                self.small_button(
                    ui,
                    tool,
                    x + left,
                    y + 5.0 + (k % 3) as f32 * SMALL_H,
                    cols[k / 3],
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
            .on_hover_text(if tool.action.is_some() {
                tool.name.to_string()
            } else {
                format!("{}: coming", tool.name)
            });
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
        let g = ui.fonts_mut(|f| f.layout_no_wrap(big_label(tool.name), sans(12.0), ink));
        let p = ui.painter();
        icons::draw(p, pos2(r.center().x - 14.0, y + 6.0), 28.0, tool.icon, 1.25, ink);
        p.galley(pos2(r.center().x - g.size().x * 0.5, y + 36.0), g, ink);
    }

    fn small_button(&mut self, ui: &mut Ui, tool: &Tool, x: f32, y: f32, w: f32) {
        let r = Rect::from_min_size(pos2(x, y), vec2(w - 2.0, 26.0));
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
