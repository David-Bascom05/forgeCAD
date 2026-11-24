//! What each operation's dialog shows: its arguments, the tab each is on, and how each is
//! edited. The arguments are forge-cam-lang's own (docs/cam.md's table); a dialog writes each as
//! `SetArg` on the document and leaves out what is left at the built-in's default.

/// The dialog's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Tool,
    Geometry,
    Heights,
    Passes,
    Linking,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Tool, Tab::Geometry, Tab::Heights, Tab::Passes, Tab::Linking];
    pub fn label(self) -> &'static str {
        match self {
            Tab::Tool => "Tool",
            Tab::Geometry => "Geometry",
            Tab::Heights => "Heights",
            Tab::Passes => "Passes",
            Tab::Linking => "Linking",
        }
    }
}

/// How a field is edited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A length in mm (a number, a name or an expression).
    Len,
    /// An angle in degrees.
    Deg,
    /// One of these words (written quoted).
    Choice(&'static [&'static str]),
    /// One of the document's tools (a name).
    Tool,
    /// The part's hole feature (`f.bolts`): positional argument 1.
    Hole,
    /// The part's sketch (`f.mark`): positional argument 1.
    Sketch,
    /// A z on the spindle axis, mm (turning: the part's Z0 where the part puts it).
    Z,
    /// A count (passes).
    Count,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    /// The keyword, or `#1` for the second positional argument.
    pub arg: &'static str,
    pub label: &'static str,
    pub tab: Tab,
    pub kind: Kind,
    /// What applies when it is left out, in words.
    pub default: &'static str,
    /// Required: the built-in refuses without it.
    pub required: bool,
}

const fn f(arg: &'static str, label: &'static str, tab: Tab, kind: Kind, default: &'static str) -> Field {
    Field {
        arg,
        label,
        tab,
        kind,
        default,
        required: false,
    }
}

const fn req(arg: &'static str, label: &'static str, tab: Tab, kind: Kind) -> Field {
    Field {
        arg,
        label,
        tab,
        kind,
        default: "",
        required: true,
    }
}

/// One operation kind: its built-in, its name in the UI, its ribbon group, and its fields.
#[derive(Clone, Copy, Debug)]
pub struct OpSpec {
    pub func: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub icon: &'static str,
    pub what: &'static str,
    pub fields: &'static [Field],
}

use Kind::{Choice, Count, Deg, Hole, Len, Sketch, Z};
use Tab::{Geometry, Heights, Linking, Passes};

const TOOL: Field = req("tool", "Tool", Tab::Tool, Kind::Tool);

pub const OPS: &[OpSpec] = &[
    OpSpec {
        func: "facing",
        label: "Face",
        group: "2D",
        icon: "cam_face",
        what: "Zig-zag passes over the stock's top, down to a height",
        fields: &[
            TOOL,
            f("to", "Bottom height", Heights, Len, "the part's top"),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("leave_axial", "Axial stock to leave", Passes, Len, "0 (floors)"),
        ],
    },
    OpSpec {
        func: "clear",
        label: "2D Clear",
        group: "2D",
        icon: "cam_clear",
        what: "Offset passes clearing between the stock outline and the part's section",
        fields: &[
            TOOL,
            f("from", "Top height", Heights, Len, "the stock's top"),
            req("to", "Bottom height", Heights, Len),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("stepover", "Stepover", Passes, Len, "0.4 × the tool's diameter"),
            f("leave", "Radial stock to leave", Passes, Len, "0 (walls)"),
            f("leave_axial", "Axial stock to leave", Passes, Len, "0 (floors)"),
            f("ramp", "Helical ramp angle", Linking, Deg, "plunge"),
        ],
    },
    OpSpec {
        func: "contour",
        label: "2D Contour",
        group: "2D",
        icon: "cam_contour",
        what: "The part's outline or its holes, at the section, in levels",
        fields: &[
            TOOL,
            f(
                "side",
                "Side",
                Geometry,
                Choice(&["outside", "inside"]),
                "outside",
            ),
            f("from", "Top height", Heights, Len, "the part's top; or a face: f.boss.end + 0.5"),
            f("to", "Bottom height", Heights, Len, "the part's bottom; or a face: f.plate.end"),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("leave", "Radial stock to leave", Passes, Len, "0 (walls)"),
            f("leave_axial", "Axial stock to leave", Passes, Len, "0 (floors)"),
            f("direction", "Direction", Passes, Choice(&["climb", "conventional"]), "climb"),
            f("finish", "Finishing passes", Passes, Count, "1"),
            f("finish_step", "Finishing stepover", Passes, Len, "0.1 × the tool's diameter"),
            f("spring", "Spring passes", Passes, Count, "0"),
            f("tabs", "Tabs: count", Geometry, Count, "none (count, spacing or positions: one)"),
            f("tab_spacing", "Tabs: spacing", Geometry, Len, "none"),
            f("tab_at", "Tabs: at [(x, y), ..]", Geometry, Len, "none"),
            f("tab_width", "Tab width", Geometry, Len, "the tool's diameter"),
            f("tab_height", "Tab height", Geometry, Len, "1"),
            f("tab_shape", "Tab shape", Geometry, Choice(&["rect", "triangle"]), "rect"),
            f("lead", "Lead in / out", Linking, Choice(&["none", "arc", "line"]), "none"),
            f("lead_radius", "Lead radius", Linking, Len, "half the tool's diameter (arc)"),
            f("lead_sweep", "Lead sweep", Linking, Deg, "90 (at most 180)"),
            f("lead_length", "Lead length", Linking, Len, "0 for an arc, D/2 for a line"),
            f("entry", "Entry point (x, y)", Linking, Len, "where the path starts"),
            f("overlap", "Overlap", Linking, Len, "0"),
        ],
    },
    OpSpec {
        func: "pocket",
        label: "Pocket",
        group: "2D",
        icon: "cam_pocket",
        what: "A hole feature's circle to its depth, or every hole and slot between two heights",
        fields: &[
            TOOL,
            f(
                "#1",
                "Hole feature",
                Geometry,
                Hole,
                "every pocket between the heights",
            ),
            f("from", "Top height", Heights, Len, "(needed without a hole)"),
            f("to", "Bottom height", Heights, Len, "(needed without a hole)"),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("stepover", "Stepover", Passes, Len, "0.4 × the tool's diameter"),
            f("leave", "Radial stock to leave", Passes, Len, "0 (walls)"),
            f("leave_axial", "Axial stock to leave", Passes, Len, "0 (floors)"),
            f("ramp", "Helical ramp angle", Linking, Deg, "plunge"),
        ],
    },
    OpSpec {
        func: "edge_break",
        label: "Chamfer",
        group: "2D",
        icon: "cam_chamfer",
        what: "Break the top edges by a 45° chamfer with a 90° chamfer mill (a declared removal)",
        fields: &[
            TOOL,
            f("at", "Top height", Heights, Len, "the part's top"),
            req("size", "Chamfer size", Passes, Len),
            f("tip_offset", "Tip offset", Passes, Len, "0.2"),
        ],
    },
    OpSpec {
        func: "engrave",
        label: "Engrave",
        group: "2D",
        icon: "cam_engrave",
        what: "The tool's tip along a sketch's lines, arcs and circles",
        fields: &[
            TOOL,
            req("#1", "Sketch", Geometry, Sketch),
            f("from", "Top height", Heights, Len, "the sketch's plane"),
            req("depth", "Depth", Passes, Len),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("offset", "Sideways offset", Passes, Len, "0 (+ left, − right)"),
            f("axial_offset", "Axial offset", Passes, Len, "0"),
        ],
    },
    OpSpec {
        func: "drill",
        label: "Drill",
        group: "Drilling",
        icon: "cam_drill",
        what: "Spot, drill or peck, tap or ream: the plan from the hole feature's history",
        fields: &[
            req("#1", "Hole feature", Geometry, Hole),
            f("plan", "Plan", Passes, Choice(&["auto", "drill"]), "auto"),
        ],
    },
    OpSpec {
        func: "thread_mill",
        label: "Thread Mill",
        group: "Drilling",
        icon: "cam_thread",
        what: "A hole's recorded thread, helically (a declared cosmetic-thread removal)",
        fields: &[
            TOOL,
            req("#1", "Hole feature", Geometry, Hole),
            f(
                "length",
                "Thread length",
                Passes,
                Len,
                "the depth less two pitches",
            ),
            f("passes", "Passes", Passes, Count, "1"),
            f("spring", "Spring passes", Passes, Count, "0"),
            f("hand", "Hand", Passes, Choice(&["right", "left"]), "right"),
            f("starts", "Starts", Passes, Count, "1"),
            f("lead", "Lead in", Linking, Choice(&["line", "arc"]), "line"),
        ],
    },
    OpSpec {
        func: "bore",
        label: "Bore",
        group: "Drilling",
        icon: "cam_pocket",
        what: "A hole feature's bore, helically to its depth with a finishing pass",
        fields: &[
            TOOL,
            req("#1", "Hole feature", Geometry, Hole),
            f("pitch", "Pitch per turn", Passes, Len, "from the ramp"),
            f("finish_leave", "Finishing stock", Passes, Len, "0"),
            f("spring", "Spring passes", Passes, Count, "0"),
            f("leave", "Radial stock to leave", Passes, Len, "0 (walls)"),
            f("leave_axial", "Axial stock to leave", Passes, Len, "0 (floor)"),
            f("ramp", "Helical ramp angle", Linking, Deg, "3 (at most 30)"),
        ],
    },
];

const LTOOL: Field = req("tool", "Tool", Tab::Tool, Kind::Tool);

/// Turning (forge-turn-lang, docs/cam-turning.md): z along the spindle, diameters across it.
pub const TURN_OPS: &[OpSpec] = &[
    OpSpec {
        func: "turn_face",
        label: "Turn Face",
        group: "Turning",
        icon: "cam_face",
        what: "Face from the bar's front to Z, the nose centre to the centre line",
        fields: &[
            LTOOL,
            req("to", "Face to Z", Geometry, Z),
            f("step", "Max depth per pass", Passes, Len, "the tool's ap"),
        ],
    },
    OpSpec {
        func: "turn_rough",
        label: "Rough",
        group: "Turning",
        icon: "cam_clear",
        what: "Axial passes at every step and every diameter of the part, down to the part grown by the nose radius and the leave",
        fields: &[
            LTOOL,
            f("to", "To Z", Geometry, Z, "the part's back"),
            f("top", "From diameter", Geometry, Len, "the bar"),
            f("step", "Depth of cut", Passes, Len, "the tool's ap"),
            f("leave", "Stock to leave", Passes, Len, "0"),
        ],
    },
    OpSpec {
        func: "turn_finish",
        label: "Finish",
        group: "Turning",
        icon: "cam_contour",
        what: "The profile, the nose centre on the exact offset of the part by the nose radius (posted with the imaginary tip, G40)",
        fields: &[
            LTOOL,
            f("to", "To Z", Geometry, Z, "the part's back"),
            f("leave", "Stock to leave", Passes, Len, "0"),
        ],
    },
    OpSpec {
        func: "bore_rough",
        label: "Bore",
        group: "Turning",
        icon: "cam_pocket",
        what: "The drilled hole opened outward in axial passes to the bore grown by the nose radius and the leave; Finish with a boring bar then finishes it",
        fields: &[
            LTOOL,
            req("from", "Drilled diameter", Geometry, Len),
            f("to", "To Z", Geometry, Z, "the bore's bottom"),
            f("step", "Depth of cut", Passes, Len, "the tool's ap"),
            f("leave", "Stock to leave", Passes, Len, "0"),
        ],
    },
    OpSpec {
        func: "turn_groove",
        label: "Groove",
        group: "Turning",
        icon: "cam_pocket",
        what: "A groove between two z to a radius, by plunges",
        fields: &[
            LTOOL,
            req("from", "From Z", Geometry, Z),
            req("to", "To Z", Geometry, Z),
            req("r", "Bottom radius", Geometry, Len),
        ],
    },
    OpSpec {
        func: "turn_thread",
        label: "Thread",
        group: "Turning",
        icon: "cam_thread",
        what: "An ISO metric external thread (60°), radial infeed of equal chip area: a declared removal",
        fields: &[
            LTOOL,
            req("d", "Major diameter", Geometry, Len),
            req("pitch", "Pitch", Geometry, Len),
            req("from", "From Z", Geometry, Z),
            req("to", "To Z", Geometry, Z),
            f("passes", "Passes", Passes, Count, "8"),
        ],
    },
    OpSpec {
        func: "turn_drill",
        label: "Turn Drill",
        group: "Turning",
        icon: "cam_drill",
        what: "A drill on the centre line",
        fields: &[
            LTOOL,
            req("to", "To Z", Geometry, Z),
            f("peck", "Peck", Passes, Len, "0 (none)"),
        ],
    },
    OpSpec {
        func: "part_off",
        label: "Part Off",
        group: "Turning",
        icon: "cam_chamfer",
        what: "The blade's side on the part's back face at Z, through the centre",
        fields: &[LTOOL, req("at", "At Z", Geometry, Z)],
    },
];

pub fn spec(func: &str) -> Option<&'static OpSpec> {
    OPS.iter().chain(TURN_OPS).find(|s| s.func == func)
}

/// Whether an op built-in is turning's.
pub fn is_turning_op(func: &str) -> bool {
    TURN_OPS.iter().any(|s| s.func == func)
}

/// The tabs `s` has fields on, in order.
/// The tabs `s` shows, in order: those it has fields on, and Linking always (the setup's
/// retract and rapids apply to every op; a lathe's are its clearances to stock and chuck).
pub fn tabs(s: &OpSpec) -> Vec<Tab> {
    Tab::ALL
        .into_iter()
        .filter(|t| *t == Tab::Linking || s.fields.iter().any(|f| f.tab == *t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_op_built_in_has_a_form() {
        for f in super::super::doc::OP_FUNCS {
            assert!(spec(f).is_some(), "{f}");
        }
        for s in OPS {
            assert!(!tabs(s).is_empty());
        }
    }
}
