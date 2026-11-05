//! The ribbon's tabs, groups and buttons. A button with no action is drawn disabled: there are no
//! buttons that pretend.

use super::theme::sans;
use super::{Action, FgCmd, RIBBON_H, Shell, TABS_H, TITLE_H, icons};
use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};

mod sketch_tab;
pub(super) mod tips;

pub(super) use sketch_tab::close_drop;

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
    /// Icon-only buttons in a grid of three rows, as Inventor's Constrain panel has them.
    pub(super) icons: Vec<Tool>,
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
    let g = |name, big, small| Group {
        name,
        big,
        small,
        icons: vec![],
    };
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
                vec![
                    fg("Place", "part", FgCmd::Place, up("place")),
                    go(
                        "Content Center",
                        "folder",
                        Action::Fg(FgCmd::ContentCenter),
                        s.content.open,
                    ),
                ],
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
                {
                    // a drag's pose: written, or dropped
                    let posed = a.is_some_and(|a| !a.posing.is_empty());
                    let pose = |name, icon, c| Tool {
                        name,
                        icon,
                        action: posed.then_some(Action::Fg(c)),
                        on: false,
                    };
                    vec![
                        pose("Keep Position", "pin", FgCmd::KeepPosition),
                        pose("Return to Design", "home", FgCmd::ReturnToDesign),
                    ]
                },
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
                t("CG Marker", "cg", C::Cg, i.cg),
            ],
            icons: vec![],
        },
        Group {
            name: "Analysis",
            big: vec![
                t("Section", "section", C::Section, i.cut.is_some()),
                off("Check", "check"),
            ],
            small: vec![],
            icons: vec![],
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
            icons: vec![],
        },
    ]
}

fn groups(s: &Shell) -> Vec<Group> {
    let g = |name, big, small| Group {
        name,
        big,
        small,
        icons: vec![],
    };
    if let Some(env) = &s.drawing {
        // the Drawing environment's: a tool it has not got is drawn off
        let tool = |(c, icon): (super::drawing::DwgCmd, &'static str)| {
            if c == super::drawing::DwgCmd::BreakOut {
                off(c.name(), icon)
            } else {
                go(c.name(), icon, Action::Dwg(c), env.tool == Some(c))
            }
        };
        return s
            .drawing_groups()
            .into_iter()
            .map(|(name, big, small)| {
                g(
                    name,
                    big.into_iter().map(tool).collect(),
                    small.into_iter().map(tool).collect(),
                )
            })
            .collect();
    }
    let v = s.view;
    if s.is_asm()
        && let Some(gs) = asm_groups(s)
    {
        return gs;
    }
    if s.tab == super::THERMO_TAB {
        return super::thermo_ui::groups(s)
            .into_iter()
            .map(|(name, big, small)| {
                let tool = |(name, icon, action, on)| Tool {
                    name,
                    icon,
                    action,
                    on,
                };
                g(
                    name,
                    big.into_iter().map(tool).collect(),
                    small.into_iter().map(tool).collect(),
                )
            })
            .collect();
    }
    let asm_explode = s.fg.as_ref().and_then(|f| f.asm.as_ref()).map(|a| a.explode_open);
    let asm_glyphs = s.fg.as_ref().and_then(|f| f.asm.as_ref()).map(|a| a.glyphs.all);
    let fg_open = s.fg.is_some();
    // the tools only a Forge part has a form for
    let fg_new = |name, icon, kind| Tool {
        name,
        icon,
        action: fg_open.then_some(Action::NewFeature(kind)),
        on: false,
    };
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
                vec![
                    new("Loft", "loft", "loft"),
                    new("Sweep", "sweep", "sweep"),
                    new("Coil", "coil", "coil"),
                    new("Rib", "rib", "rib"),
                    fg_new("Emboss", "emboss", "emboss"),
                    fg_new("Import", "import", "import"),
                    off("Decal", "decal"),
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
                    fg_new("Split", "split", "split"),
                    new("Combine", "combine", "combine"),
                    fg_new("Thicken", "thicken", "thicken"),
                    fg_new("Direct", "move", "direct"),
                    fg_new("Delete Face", "deleteface", "deleteface"),
                    fg_new("Replace", "replaceface", "replaceface"),
                ],
            ),
            g(
                "Work Features",
                vec![new("Plane", "plane", "plane")],
                vec![
                    new("Axis", "axis", "axis"),
                    new("Point", "point", "point"),
                    fg_new("UCS", "ucs", "ucs"),
                ],
            ),
            g(
                "Pattern",
                vec![new("Rectangular", "rect", "pattern")],
                vec![
                    fg_new("Circular", "circular", "circular"),
                    fg_new("Sketch", "sketchpattern", "sketchpattern"),
                    fg_new("Mirror", "mirror", "mirror"),
                ],
            ),
            g(
                "Bodies",
                vec![],
                vec![
                    fg_new("Move", "movebody", "movebody"),
                    fg_new("Copy", "copybody", "copybody"),
                ],
            ),
        ]
        .into_iter()
        // a Forge part has sheet metal: its own tab, as Inventor's Convert to Sheet Metal gives
        .chain((s.fg.is_some() && !s.sheet.env).then(|| {
            let (name, icon, action, on) = super::thermo_ui::convert_button(s);
            g(
                "Convert",
                vec![
                    go(
                        "Convert to Sheet Metal",
                        "smface",
                        Action::Sheet(super::sheet::SheetCmd::Convert),
                        false,
                    ),
                    Tool {
                        name,
                        icon,
                        action,
                        on,
                    },
                ],
                vec![],
            )
        }))
        .collect(),
        super::SHEET_TAB => super::sheetui::sheet_groups(s),
        super::FLAT_TAB => super::sheetui::flat_groups(s),
        // drawn by sketch_tab, its split buttons and dropdowns
        1 => vec![],
        2 => inspect_groups(s),
        3 => vec![
            g(
                "Appearance",
                vec![
                    go(
                        super::visual_style::BUTTON,
                        "lines",
                        Action::VisualStyle(v.style),
                        false,
                    ),
                    go("Section", "section", Action::Section, v.section),
                    Tool {
                        name: "Explode",
                        icon: "combine",
                        action: asm_explode.map(|_| Action::Fg(FgCmd::Explode)),
                        on: asm_explode == Some(true),
                    },
                    Tool {
                        name: "Relationships",
                        icon: "relfolder",
                        action: asm_glyphs.map(|_| Action::Fg(FgCmd::Relationships)),
                        on: asm_glyphs == Some(true),
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
                "Options",
                vec![go("Settings", "options", Action::Settings, s.prefs.open)],
                vec![],
            ),
        ],
    }
}

/// Why a tool Inventor has is greyed here, said in its tooltip.
fn why_off(name: &str) -> Option<&'static str> {
    Some(match name {
        "Decal" => "not yet: the kernel has no image projection onto faces",
        "Smooth (G2)" => {
            "not yet: the sketch solver has no curvature tie at a joint (Tangent holds G1 meanwhile)"
        }
        _ => return None,
    })
}

/// The Sketch tab of a part (not a drawing's, an assembly's or another environment's).
fn s_is_sketch_tab(s: &Shell) -> bool {
    s.tab == 1 && s.drawing.is_none() && !s.is_asm()
}

const BIG_W: f32 = 54.0;
const BIG_PAD: f32 = 12.0;
const SMALL_W: f32 = 92.0;
const SMALL_H: f32 = 27.0;
const ICON_W: f32 = 30.0;

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
        if s_is_sketch_tab(self) {
            return self.sketch_ribbon(ui, w);
        }
        let mut gx = 0.0;
        let gs = groups(self);
        // too wide for the window: small buttons lose their words, as Inventor's ribbon shrinks
        let full: f32 = gs
            .iter()
            .map(|g| {
                12.0 + g.big.iter().map(|tool| big_width(ui, tool)).sum::<f32>()
                    + g.small.chunks(3).map(|c| small_width(ui, c)).sum::<f32>()
                    + g.icons.len().div_ceil(3) as f32 * ICON_W
            })
            .sum();
        let compact = full > w;
        for mut g in gs {
            if compact {
                let mut small = std::mem::take(&mut g.small);
                small.append(&mut g.icons);
                g.icons = small;
            }
            let bigs: Vec<f32> = g.big.iter().map(|tool| big_width(ui, tool)).collect();
            let cols: Vec<f32> = g.small.chunks(3).map(|c| small_width(ui, c)).collect();
            let icon_cols = g.icons.len().div_ceil(3);
            let width =
                12.0 + bigs.iter().sum::<f32>() + cols.iter().sum::<f32>() + icon_cols as f32 * ICON_W;
            let width = width.max(
                ui.fonts_mut(|f| f.layout_no_wrap(g.name.to_string(), sans(11.0), t.ink2))
                    .size()
                    .x
                    + 16.0,
            );
            let mut x = gx + 6.0;
            for (tool, w) in g.big.iter().zip(&bigs) {
                if tool.name == super::visual_style::BUTTON {
                    self.style_button(ui, tool, x, y + 6.0, *w);
                } else {
                    self.big_button(ui, tool, x, y + 6.0, *w);
                }
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
            let ix = x + cols.iter().sum::<f32>();
            for (k, tool) in g.icons.iter().enumerate() {
                self.grid_button(
                    ui,
                    tool,
                    ix + (k / 3) as f32 * ICON_W,
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

    pub(super) fn tool_response(
        &mut self,
        ui: &mut Ui,
        tool: &Tool,
        r: Rect,
    ) -> (egui::Color32, egui::Response) {
        let t = self.theme;
        let sense = if tool.action.is_some() {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui.interact(r, ui.scope_id().with(("tool", self.tab, tool.name)), sense);
        let resp = self.tool_tip(resp, tool);
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

    fn grid_button(&mut self, ui: &mut Ui, tool: &Tool, x: f32, y: f32) {
        let r = Rect::from_min_size(pos2(x, y), vec2(ICON_W - 2.0, 26.0));
        let (ink, _) = self.tool_response(ui, tool, r);
        icons::draw(ui.painter(), pos2(x + 5.0, y + 4.0), 18.0, tool.icon, 1.5, ink);
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
