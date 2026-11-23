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

use Kind::{Choice, Deg, Hole, Len, Sketch};
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
            f("leave", "Stock to leave", Passes, Len, "0"),
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
            f("side", "Side", Geometry, Choice(&["outside", "inside"]), "outside"),
            f("from", "Top height", Heights, Len, "the part's top"),
            f("to", "Bottom height", Heights, Len, "the part's bottom"),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("leave", "Stock to leave", Passes, Len, "0"),
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
            f("#1", "Hole feature", Geometry, Hole, "every pocket between the heights"),
            f("from", "Top height", Heights, Len, "(needed without a hole)"),
            f("to", "Bottom height", Heights, Len, "(needed without a hole)"),
            f("step", "Max stepdown", Passes, Len, "half the tool's diameter"),
            f("stepover", "Stepover", Passes, Len, "0.4 × the tool's diameter"),
            f("leave", "Stock to leave", Passes, Len, "0"),
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
            f("length", "Thread length", Passes, Len, "the depth less two pitches"),
        ],
    },
];

pub fn spec(func: &str) -> Option<&'static OpSpec> {
    OPS.iter().find(|s| s.func == func)
}

/// The tabs `s` has fields on, in order.
pub fn tabs(s: &OpSpec) -> Vec<Tab> {
    Tab::ALL
        .into_iter()
        .filter(|t| s.fields.iter().any(|f| f.tab == *t))
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
