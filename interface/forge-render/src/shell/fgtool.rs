//! The ribbon's tools on a Forge (.fg) part: each a form of picks and params that writes one
//! Forge statement (and its params above it), applied as an `AddFeature` op. Picks are anchor
//! text (`base.end`, `edges(between: (a, b))`, `plan.outline[1]`): what the evaluator itself
//! resolves, never a coordinate or a hash. A pick with no stable Forge name is refused, typed.
//! A tool's choices (a fillet's type, a plane's method, an extrude's extent) are Inventor's, and
//! each choice gives the form its own picks and values.

use forge_lang::error::Literal;
use forge_lang::values::Dim;

/// What a slot takes, and where from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    /// Edges of the part, picked in the view; several joined.
    Edges,
    /// Faces of the part, picked in the view; several joined.
    Faces,
    /// One face of the part.
    Face,
    /// One edge of the part.
    Edge,
    /// An edge or a face (a work axis along either).
    EdgeOrFace,
    /// A plane: a planar face, an origin plane, a sketch or a work plane.
    Plane,
    /// A direction: a planar face's normal, or a sketch line's.
    Dir,
    /// An axis: a cylinder's, a sketch line, a work axis, a revolve's or a hole's.
    Axis,
    /// A closed sketch entity or loop, or a whole sketch.
    Profile,
    Profiles,
    /// A sketch line.
    Line,
    /// Sketch lines, arcs, splines or loops, in order.
    Path,
    /// A sketch's points.
    Points,
    /// One point: a sketch point, a circle's centre, a work point.
    Point,
    /// A feature (from the tree).
    Feature,
    Features,
    /// A sheet metal edge a flange stands on.
    SheetEdge,
    /// A revolve's axis: a line of its sketch, a straight edge, a work axis, a cylinder's axis
    /// or the origin's.
    RevAxis,
    /// Where an extent ends: faces of the part, a work plane or a sketch's plane.
    UpTo,
    /// A weldment member (`frame[2]`), from the browser.
    Member,
    /// Weldment members, from the browser; several listed.
    Members,
    /// What a member's end is trimmed to: a face, a work plane or a sketch, a member or a body.
    TrimTo,
}

impl SlotKind {
    pub fn many(self) -> bool {
        matches!(
            self,
            SlotKind::Edges
                | SlotKind::Faces
                | SlotKind::Profiles
                | SlotKind::Path
                | SlotKind::Features
                | SlotKind::UpTo
                | SlotKind::Members
        )
    }

    /// It takes what's clicked in the view.
    pub fn from_view(self) -> bool {
        matches!(
            self,
            SlotKind::Edges
                | SlotKind::Faces
                | SlotKind::Face
                | SlotKind::Edge
                | SlotKind::EdgeOrFace
                | SlotKind::Plane
                | SlotKind::Dir
                | SlotKind::Axis
                | SlotKind::SheetEdge
                | SlotKind::RevAxis
                | SlotKind::UpTo
                | SlotKind::TrimTo
        )
    }

    /// It takes a feature or sketch clicked in the tree.
    pub fn from_tree(self) -> bool {
        matches!(
            self,
            SlotKind::Feature
                | SlotKind::Features
                | SlotKind::Profile
                | SlotKind::Profiles
                | SlotKind::Plane
                | SlotKind::Points
                | SlotKind::RevAxis
                | SlotKind::UpTo
                | SlotKind::Member
                | SlotKind::Members
                | SlotKind::TrimTo
        )
    }

    /// What the status bar asks for while the slot takes picks.
    pub fn prompt(self) -> &'static str {
        match self {
            SlotKind::Edges => "Select edges",
            SlotKind::Faces => "Select faces",
            SlotKind::Face => "Select a face",
            SlotKind::Edge => "Select an edge",
            SlotKind::EdgeOrFace => "Select an edge or a cylindrical face",
            SlotKind::Plane => "Select a planar face, an origin plane, a sketch or a work plane",
            SlotKind::Dir => "Select a planar face or a sketch line for the direction",
            SlotKind::Axis => "Select a cylindrical face, a sketch line or a work axis",
            SlotKind::Profile => "Select a profile",
            SlotKind::Profiles => "Select profiles, in order",
            SlotKind::Line => "Select a sketch line",
            SlotKind::Path => "Select the path: sketch lines, arcs or splines, in order",
            SlotKind::Points => "Select sketch points",
            SlotKind::Point => "Select a point",
            SlotKind::Feature => "Select a feature or body in the browser",
            SlotKind::Features => "Select features or bodies in the browser",
            SlotKind::SheetEdge => "Select a sheet metal edge",
            SlotKind::RevAxis => {
                "Select a sketch line, a straight edge, a cylindrical face, a work axis or an origin axis"
            }
            SlotKind::UpTo => "Select the faces, a work plane or a sketch to end at",
            SlotKind::Member => "Select a member in the browser",
            SlotKind::Members => "Select members in the browser",
            SlotKind::TrimTo => "Select a face, a work plane, a sketch, a member or a body to trim to",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub label: &'static str,
    pub kind: SlotKind,
    pub picks: Vec<String>,
    /// Nothing picked is allowed: the statement goes without it.
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolParam {
    pub name: &'static str,
    pub text: String,
    pub dim: Dim,
    /// What an edited feature wrote for it where that's no plain number (`t`, `w / 2`): kept
    /// as written while the field still shows it.
    pub expr: Option<String>,
}

/// A row of toggles in the dialog: its items (word, icon) and which is on. Choosing another
/// gives the form that item's picks and values.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub name: &'static str,
    pub items: &'static [(&'static str, &'static str)],
    pub on: usize,
}

impl Choice {
    pub fn word(&self) -> &'static str {
        self.items[self.on].0
    }
}

/// Why a tool can't write its statement: typed, never a guess.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolError {
    MissingPick {
        slot: &'static str,
    },
    TooFew {
        slot: &'static str,
        want: usize,
    },
    BadNumber {
        param: &'static str,
        text: String,
    },
    /// The picked entity has no anchor: no Forge text names it stably.
    NoStableName {
        what: &'static str,
    },
    /// The pick is not what the slot takes.
    WrongPick {
        slot: &'static str,
        what: &'static str,
    },
    BadName {
        name: String,
    },
    /// The tool has no Forge form (yet), or the act none in Forge's ops.
    NoForgeForm {
        tool: &'static str,
        why: String,
    },
    /// A sketch value changed where the file writes an expression: it has no handle of its own.
    Driven {
        value: String,
        expr: String,
    },
    /// A pick in the posed view on an occurrence that is not this document's own model (a
    /// component's call, a part file): changed where it is made.
    NotOwnModel {
        occurrence: String,
    },
}

impl ToolError {
    pub fn kind(&self) -> &'static str {
        match self {
            ToolError::MissingPick { .. } => "MissingPick",
            ToolError::TooFew { .. } => "TooFew",
            ToolError::BadNumber { .. } => "BadNumber",
            ToolError::NoStableName { .. } => "NoStableName",
            ToolError::WrongPick { .. } => "WrongPick",
            ToolError::BadName { .. } => "BadName",
            ToolError::NoForgeForm { .. } => "NoForgeForm",
            ToolError::Driven { .. } => "Driven",
            ToolError::NotOwnModel { .. } => "NotOwnModel",
        }
    }

    pub fn text(&self) -> String {
        match self {
            ToolError::MissingPick { slot } => format!("{slot}: nothing picked yet"),
            ToolError::TooFew { slot, want } => format!("{slot}: pick at least {want}"),
            ToolError::BadNumber { param, text } => format!("{param}: {text:?} is not a number"),
            ToolError::NoStableName { what } => format!(
                "that {what} has no stable Forge name (no anchor of the part names it); pick another, or name it in the source"
            ),
            ToolError::WrongPick { slot, what } => format!("{slot} takes no {what}"),
            ToolError::BadName { name } => format!("{name:?} is not a feature name"),
            ToolError::NoForgeForm { tool, why } => format!("{tool}: {why}"),
            ToolError::Driven { value, expr } => {
                format!("{value}: that value is `{expr}`, driven by what it reads; change those instead")
            }
            ToolError::NotOwnModel { occurrence } => {
                format!(
                    "{occurrence} is not this document's own model: open its component or file to change it"
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FgTool {
    pub kind: &'static str,
    /// The feature's id: what it's assigned to.
    pub name: String,
    pub choices: Vec<Choice>,
    pub slots: Vec<Slot>,
    pub params: Vec<ToolParam>,
    /// Words: a thread's spec and length, a text's string and font.
    pub words: Vec<(&'static str, String)>,
    pub flags: Vec<(&'static str, bool)>,
    /// What a word may be, where it's one of a list (a font beside the part, a screw size).
    pub options: Vec<(&'static str, Vec<String>)>,
    /// The slot that takes the next pick.
    pub active: usize,
    pub error: Option<ToolError>,
    /// Why what it would make doesn't check, from its preview.
    pub note: Option<String>,
    /// + was pressed: another of these after this one.
    pub again: bool,
    /// The statement it edits (Edit Feature), replaced in its place; none: a new one.
    pub edit: Option<String>,
    /// The unit a bare length typed is in: the part's (`# units in`).
    pub units: &'static str,
    /// A hole placed from two edges: where on its face it is now, in the face plane's own
    /// coordinates (the host works it out from the part): the sketch's starting guess, so the
    /// solve lands on the face's side of each edge.
    pub at_guess: Option<(f64, f64)>,
    /// A rib's path its other side was tried for, by itself, after its preview was refused.
    pub side_tried: Option<String>,
}

fn slot(label: &'static str, kind: SlotKind) -> Slot {
    Slot {
        label,
        kind,
        picks: vec![],
        optional: false,
    }
}

fn maybe(label: &'static str, kind: SlotKind) -> Slot {
    Slot {
        optional: true,
        ..slot(label, kind)
    }
}

fn param(name: &'static str, text: &str, dim: Dim) -> ToolParam {
    ToolParam {
        name,
        text: text.into(),
        dim,
        expr: None,
    }
}

/// A tool's form: its slots, params, words and flags.
pub(super) type Form = (
    Vec<Slot>,
    Vec<ToolParam>,
    Vec<(&'static str, String)>,
    Vec<(&'static str, bool)>,
);

/// The tools there are a Forge form for, by the ribbon's kind.
pub const KINDS: [&str; 41] = [
    "extrude",
    "revolve",
    "hole",
    "fillet",
    "chamfer",
    "shell",
    "draft",
    "thread",
    "combine",
    "plane",
    "axis",
    "point",
    "pattern",
    "sweep",
    "loft",
    "coil",
    "rib",
    "sheet",
    "flange",
    "cutout",
    "unfold",
    "circular",
    "mirror",
    "sketchpattern",
    "emboss",
    "import",
    "split",
    "thicken",
    "direct",
    "deleteface",
    "replaceface",
    "ucs",
    "movebody",
    "copybody",
    "facefillet",
    "offsetface",
    "moveface",
    "members",
    "endtrim",
    "endcap",
    "cornerplate",
];

/// Where a section's pierce point sits on the path (`members(.., align:)`).
pub const ALIGNS: [&str; 11] = [
    "center",
    "centroid",
    "origin",
    "top",
    "bottom",
    "left",
    "right",
    "top_left",
    "top_right",
    "bottom_left",
    "bottom_right",
];

/// How two members meet at a node.
pub const CORNERS: [&str; 2] = ["miter", "butt"];

const OUTPUT: &[(&str, &str)] = &[
    ("Join", "join"),
    ("Cut", "opcut"),
    ("Intersect", "intersect"),
    ("New Solid", "newsolid"),
];
const OUTPUT3: &[(&str, &str)] = &[("Join", "join"), ("Cut", "opcut"), ("New Solid", "newsolid")];
const EXTENTS: &[(&str, &str)] = &[
    ("Distance", "distance"),
    ("Symmetric", "symmetric"),
    ("Through All", "thruall"),
    ("To Next", "tonext"),
    ("To", "tosurface"),
];
const PATTERN_OF: &[(&str, &str)] = &[("Features", "features"), ("Solids", "solids")];

/// The choices each kind starts with, Inventor's defaults first.
pub(super) fn choices_of(kind: &str) -> Vec<Choice> {
    let c = |name, items| Choice { name, items, on: 0 };
    match kind {
        "extrude" => vec![
            c("Extents", EXTENTS),
            c("Output", OUTPUT),
            c(
                "Body",
                &[
                    ("Solid", "extrude"),
                    ("Thin", "thicken"),
                    ("Surface", "sheetbody"),
                ],
            ),
            // a thin wall's side of its chain (shown with Thin only)
            c(
                "Side",
                &[("Out", "flipout"), ("In", "flipin"), ("Both", "symmetric")],
            ),
        ],
        "revolve" => vec![
            c("Extents", &[("Angle", "angle"), ("Full", "full")]),
            c("Output", OUTPUT3),
            c("Body", &[("Solid", "revolve"), ("Surface", "sheetbody")]),
        ],
        "sweep" | "coil" | "loft" => vec![c("Output", OUTPUT)],
        "hole" => vec![
            c(
                "Placement",
                &[
                    ("From Sketch", "sketch"),
                    ("Linear", "distance"),
                    ("Concentric", "concentric"),
                ],
            ),
            c(
                "Type",
                &[
                    ("Simple", "hole"),
                    ("Clearance", "clearance"),
                    ("Tapped", "tapped"),
                ],
            ),
            c(
                "Seat",
                &[
                    ("None", "noseat"),
                    ("Counterbore", "cbore"),
                    ("Countersink", "csink"),
                ],
            ),
            Choice {
                name: "Termination",
                items: &[
                    ("Distance", "distance"),
                    ("Through All", "thruall"),
                    ("To Next", "tonext"),
                    ("To", "tosurface"),
                ],
                on: 1,
            },
        ],
        "fillet" => vec![c(
            "Type",
            &[
                ("Constant", "fillet"),
                ("Variable", "varfillet"),
                ("Face", "facefillet"),
                ("Full Round", "fullround"),
            ],
        )],
        "chamfer" => vec![
            c(
                "Type",
                &[
                    ("Equal Distance", "chamfer"),
                    ("Two Distances", "chamfer"),
                    ("Distance and Angle", "chamfer"),
                ],
            ),
            c("Corner", &[("Patch", "chamfer"), ("Point", "point")]),
        ],
        "shell" => vec![c("Direction", &[("Inside", "shellin"), ("Outside", "shellout")])],
        "combine" => vec![c(
            "Operation",
            &[("Join", "join"), ("Cut", "opcut"), ("Intersect", "intersect")],
        )],
        "plane" => vec![c(
            "Method",
            &[
                ("Offset", "planeoffset"),
                ("Angle", "planeangle"),
                ("Tangent", "planetangent"),
                ("3 Points", "plane3"),
                ("Midplane", "planemid"),
                ("Normal", "planenormal"),
                ("On", "plane"),
            ],
        )],
        "axis" => vec![c(
            "Method",
            &[
                ("Edge", "axis"),
                ("2 Points", "axis2"),
                ("2 Planes", "axisplanes"),
                ("Normal", "axisnormal"),
            ],
        )],
        "point" => vec![c(
            "Method",
            &[
                ("Midpoint", "pointmid"),
                ("On Point", "point"),
                ("Intersection", "pointmeet"),
            ],
        )],
        "ucs" => vec![c("Method", &[("On Plane", "plane"), ("3 Points", "plane3")])],
        "pattern" | "circular" | "mirror" => vec![c("Pattern", PATTERN_OF)],
        "emboss" => vec![c("Type", &[("Emboss", "emboss"), ("Engrave", "engrave")])],
        "import" => vec![c(
            "Output",
            &[("New Solid", "newsolid"), ("Join", "join"), ("Cut", "opcut")],
        )],
        "split" => vec![c(
            "Method",
            &[
                ("Split Part", "split"),
                ("Trim Solid", "trim"),
                ("Split Face", "splitface"),
            ],
        )],
        "thicken" => vec![
            c(
                "Side",
                &[("Out", "flipout"), ("In", "flipin"), ("Both", "symmetric")],
            ),
            c("Output", OUTPUT),
        ],
        "direct" => vec![c(
            "Action",
            &[
                ("Move", "movebody"),
                ("Size", "offsetface"),
                ("Rotate", "rotate"),
                ("Delete", "deleteface"),
            ],
        )],
        "movebody" | "copybody" => vec![c("Motion", &[("Slide", "movebody"), ("Spin", "rotate")])],
        "members" => vec![
            c("Section", &[("Library", "folder"), ("Sketch", "sketch")]),
            c("Corner", &[("Miter", "chamfer"), ("Butt", "combine")]),
        ],
        "endtrim" => vec![c(
            "End",
            &[("Nearer", "endtrim"), ("Start", "endtrim"), ("End", "endtrim")],
        )],
        "endcap" => vec![c("End", &[("End", "endcap"), ("Start", "endcap")])],
        _ => vec![],
    }
}

/// A kind's form under its choices (`c` gives a choice's word).
pub(super) fn form_of(kind: &str, c: &dyn Fn(&str) -> &'static str) -> Option<Form> {
    use SlotKind as K;
    Some(match kind {
        "extrude" => {
            // a sheet or a thin wall goes along a chain (open or closed); a solid fills a profile
            let solid = !matches!(c("Body"), "Thin" | "Surface");
            let mut slots = vec![if solid {
                slot("Profile", K::Profile)
            } else {
                slot("Path", K::Path)
            }];
            let mut params = vec![];
            match c("Extents") {
                "Distance" if solid => {
                    params.extend([param("d", "10", Dim::Length), param("taper", "0", Dim::Angle)])
                }
                "Distance" | "Symmetric" => params.push(param("d", "10", Dim::Length)),
                "To" => slots.push(slot("To", K::UpTo)),
                _ => {}
            }
            let words = vec![];
            if c("Body") == "Thin" {
                params.push(param("t", "2", Dim::Length));
            }
            let flags = if c("Extents") == "Distance" && solid {
                vec![("flip", false)]
            } else {
                vec![]
            };
            (slots, params, words, flags)
        }
        "revolve" => (
            vec![
                if c("Body") == "Surface" {
                    slot("Path", K::Path)
                } else {
                    slot("Profile", K::Profile)
                },
                slot("Axis", K::RevAxis),
            ],
            if c("Extents") == "Angle" {
                vec![param("ang", "360", Dim::Angle)]
            } else {
                vec![]
            },
            vec![],
            vec![],
        ),
        "hole" => {
            let mut params = vec![];
            let mut words = vec![];
            match c("Type") {
                "Simple" => params.push(param("d", "5", Dim::Length)),
                "Clearance" => words.extend([("screw", "M5".to_string()), ("fit", "normal".to_string())]),
                _ => words.push(("screw", "M5".to_string())),
            }
            if c("Type") == "Simple" {
                match c("Seat") {
                    "Counterbore" => params.extend([
                        param("cbore_d", "10", Dim::Length),
                        param("cbore_depth", "5", Dim::Length),
                    ]),
                    "Countersink" => params.extend([
                        param("csink_d", "10", Dim::Length),
                        param("csink_angle", "90", Dim::Angle),
                    ]),
                    _ => {}
                }
            }
            if c("Termination") == "Distance" {
                params.extend([
                    param("depth", "10", Dim::Length),
                    param("point", "118", Dim::Angle),
                ]);
            }
            let flags = if c("Type") == "Tapped" {
                vec![("flip", false), ("model thread", true)]
            } else {
                vec![("flip", false)]
            };
            // where it goes: a sketch's points, a face measured from two of its edges, or round
            // a circular edge
            let mut slots = match c("Placement") {
                "Linear" => {
                    params.extend([param("d1", "10", Dim::Length), param("d2", "10", Dim::Length)]);
                    vec![
                        slot("Face", K::Face),
                        slot("Edge 1", K::Edge),
                        slot("Edge 2", K::Edge),
                    ]
                }
                "Concentric" => vec![slot("Face", K::Face), slot("Edge", K::Edge)],
                _ => vec![slot("Points", K::Points)],
            };
            if c("Termination") == "To" {
                slots.push(slot("To", K::UpTo));
            }
            (slots, params, words, flags)
        }
        "fillet" => match c("Type") {
            "Variable" => (
                vec![slot("Edges", K::Edges)],
                vec![
                    param("r_start", "2", Dim::Length),
                    param("r_end", "5", Dim::Length),
                ],
                vec![],
                vec![],
            ),
            "Face" => (
                vec![slot("Face Set 1", K::Faces), slot("Face Set 2", K::Faces)],
                vec![param("r", "2", Dim::Length)],
                vec![],
                vec![],
            ),
            "Full Round" => (
                vec![
                    slot("Side Face 1", K::Face),
                    slot("Center Face", K::Face),
                    slot("Side Face 2", K::Face),
                ],
                vec![],
                vec![],
                vec![],
            ),
            _ => (
                vec![slot("Edges", K::Edges)],
                vec![param("r", "2", Dim::Length)],
                vec![],
                vec![("tangent chain", true)],
            ),
        },
        "facefillet" => (
            vec![slot("Face Set 1", K::Faces), slot("Face Set 2", K::Faces)],
            vec![param("r", "2", Dim::Length)],
            vec![],
            vec![],
        ),
        "chamfer" => match c("Type") {
            "Two Distances" => (
                vec![slot("Edges", K::Edges), slot("Face", K::Face)],
                vec![param("d", "1", Dim::Length), param("d2", "2", Dim::Length)],
                vec![],
                vec![],
            ),
            "Distance and Angle" => (
                vec![slot("Edges", K::Edges), slot("Face", K::Face)],
                vec![param("d", "1", Dim::Length), param("ang", "45", Dim::Angle)],
                vec![],
                vec![],
            ),
            _ => (
                vec![slot("Edges", K::Edges)],
                vec![param("d", "1", Dim::Length)],
                vec![],
                vec![],
            ),
        },
        "shell" => (
            vec![maybe("Remove Faces", K::Faces)],
            vec![param("t", "2", Dim::Length)],
            vec![],
            vec![],
        ),
        "draft" => (
            vec![slot("Faces", K::Faces), slot("Neutral", K::Plane)],
            vec![param("ang", "3", Dim::Angle)],
            vec![],
            vec![],
        ),
        "thread" => (
            vec![slot("Cylinder", K::Face)],
            vec![],
            vec![("spec", "M6".into()), ("length", "full".into())],
            vec![("flip", false)],
        ),
        "combine" => (
            vec![slot("Target", K::Feature), slot("Tools", K::Features)],
            vec![],
            vec![],
            vec![("keep toolbody", false)],
        ),
        "plane" => match c("Method") {
            "Angle" => (
                vec![slot("Plane", K::Plane), slot("Axis", K::Axis)],
                vec![param("ang", "45", Dim::Angle)],
                vec![],
                vec![],
            ),
            "Tangent" => (
                vec![slot("Cylinder", K::Face), slot("Parallel To", K::Plane)],
                vec![],
                vec![],
                vec![],
            ),
            "3 Points" => (vec![slot("Points", K::Points)], vec![], vec![], vec![]),
            "Midplane" => (
                vec![slot("Plane 1", K::Plane), slot("Plane 2", K::Plane)],
                vec![],
                vec![],
                vec![],
            ),
            "Normal" => (
                vec![slot("Curve", K::Edge), slot("Point", K::Point)],
                vec![],
                vec![],
                vec![],
            ),
            "On" => (vec![slot("Plane", K::Plane)], vec![], vec![], vec![]),
            _ => (
                vec![slot("Face", K::Face)],
                vec![param("d", "10", Dim::Length)],
                vec![],
                vec![],
            ),
        },
        "axis" => match c("Method") {
            "2 Points" => (vec![slot("Points", K::Points)], vec![], vec![], vec![]),
            "2 Planes" => (
                vec![slot("Plane 1", K::Plane), slot("Plane 2", K::Plane)],
                vec![],
                vec![],
                vec![],
            ),
            "Normal" => (
                vec![slot("Plane", K::Plane), slot("Point", K::Point)],
                vec![],
                vec![],
                vec![],
            ),
            _ => (vec![slot("Along", K::EdgeOrFace)], vec![], vec![], vec![]),
        },
        "point" => match c("Method") {
            "On Point" => (vec![slot("Point", K::Point)], vec![], vec![], vec![]),
            "Intersection" => (
                vec![slot("Edge", K::Edge), slot("Face", K::Plane)],
                vec![],
                vec![],
                vec![],
            ),
            _ => (vec![slot("Edge", K::Edge)], vec![], vec![], vec![]),
        },
        "ucs" => match c("Method") {
            "3 Points" => (vec![slot("Points", K::Points)], vec![], vec![], vec![]),
            _ => (vec![slot("Plane", K::Plane)], vec![], vec![], vec![]),
        },
        "pattern" => {
            let mut slots = vec![];
            if c("Pattern") == "Features" {
                slots.push(slot("Features", K::Features));
            }
            slots.extend([slot("Direction", K::Dir), maybe("Direction 2", K::Dir)]);
            (
                slots,
                vec![
                    param("n", "3", Dim::Count),
                    param("step", "20", Dim::Length),
                    param("n2", "2", Dim::Count),
                    param("step2", "20", Dim::Length),
                ],
                vec![],
                vec![],
            )
        }
        "circular" => {
            let mut slots = vec![];
            if c("Pattern") == "Features" {
                slots.push(slot("Features", K::Features));
            }
            slots.push(slot("Rotation Axis", K::Axis));
            (
                slots,
                vec![param("n", "6", Dim::Count), param("ang", "360", Dim::Angle)],
                vec![],
                vec![("incremental", false)],
            )
        }
        "mirror" => {
            let mut slots = vec![];
            if c("Pattern") == "Features" {
                slots.push(slot("Features", K::Features));
            }
            slots.push(slot("Mirror Plane", K::Plane));
            (slots, vec![], vec![], vec![])
        }
        "sketchpattern" => (
            vec![slot("Features", K::Features), slot("Points", K::Points)],
            vec![],
            vec![],
            vec![],
        ),
        "emboss" => (
            vec![slot("Face", K::Plane), maybe("Position", K::Point)],
            vec![
                param("size", "8", Dim::Length),
                param("depth", "1", Dim::Length),
                param("wrap", "10", Dim::Length),
            ],
            vec![("text", "FORGE".into()), ("font", String::new())],
            vec![("wrap to cylinder", false)],
        ),
        "import" => (vec![], vec![], vec![("file", String::new())], vec![]),
        "split" => match c("Method") {
            "Split Face" => (
                vec![slot("Faces", K::Faces), slot("Split Tool", K::Plane)],
                vec![],
                vec![],
                vec![],
            ),
            "Trim Solid" => (
                vec![slot("Split Tool", K::Plane)],
                vec![],
                vec![],
                vec![("keep other side", false)],
            ),
            _ => (vec![slot("Split Tool", K::Plane)], vec![], vec![], vec![]),
        },
        "thicken" => (
            vec![slot("Faces", K::Faces)],
            vec![param("t", "2", Dim::Length)],
            vec![],
            vec![],
        ),
        "offsetface" => (
            vec![slot("Faces", K::Faces)],
            vec![param("d", "1", Dim::Length)],
            vec![],
            vec![],
        ),
        "moveface" => (
            vec![slot("Faces", K::Faces), slot("Direction", K::Dir)],
            vec![param("d", "5", Dim::Length)],
            vec![],
            vec![],
        ),
        "direct" => match c("Action") {
            "Size" => (
                vec![slot("Faces", K::Faces)],
                vec![param("d", "1", Dim::Length)],
                vec![],
                vec![],
            ),
            "Rotate" => (
                vec![slot("Faces", K::Faces), slot("Axis", K::Axis)],
                vec![param("ang", "5", Dim::Angle)],
                vec![],
                vec![],
            ),
            "Delete" => (vec![slot("Faces", K::Faces)], vec![], vec![], vec![]),
            _ => (
                vec![slot("Faces", K::Faces), slot("Direction", K::Dir)],
                vec![param("d", "5", Dim::Length)],
                vec![],
                vec![],
            ),
        },
        "deleteface" => (vec![slot("Faces", K::Faces)], vec![], vec![], vec![]),
        "replaceface" => (
            vec![slot("Existing Face", K::Face), slot("New Face", K::Plane)],
            vec![],
            vec![],
            vec![],
        ),
        "movebody" | "copybody" => {
            let mut slots = vec![slot("Bodies", K::Features)];
            let params = if c("Motion") == "Spin" {
                slots.push(slot("Axis", K::Axis));
                vec![param("ang", "90", Dim::Angle)]
            } else {
                slots.push(slot("Direction", K::Dir));
                vec![param("d", "20", Dim::Length)]
            };
            (slots, params, vec![], vec![])
        }
        "sweep" => (
            vec![slot("Profile", K::Profile), slot("Path", K::Path)],
            vec![],
            vec![],
            vec![],
        ),
        "loft" => (
            vec![slot("Profiles", K::Profiles)],
            vec![],
            vec![],
            vec![("smooth", false)],
        ),
        "coil" => (
            vec![slot("Profile", K::Profile), slot("Axis", K::Line)],
            vec![param("pitch", "8", Dim::Length), param("turns", "4", Dim::Count)],
            vec![],
            vec![("left hand", false)],
        ),
        "rib" => (
            vec![slot("Path", K::Path)],
            vec![param("t", "2", Dim::Length)],
            vec![],
            vec![("flip", false)],
        ),
        "sheet" => (
            vec![slot("Profile", K::Profile)],
            vec![
                param("t", "2", Dim::Length),
                param("r", "2", Dim::Length),
                param("k", "0.4", Dim::Ratio),
            ],
            vec![],
            vec![],
        ),
        "cutout" => (
            vec![slot("Profile", K::Profile), slot("Wall", K::Feature)],
            vec![],
            vec![],
            vec![],
        ),
        "unfold" => (vec![slot("Sheet", K::Feature)], vec![], vec![], vec![]),
        "flange" => (
            vec![slot("Edge", K::SheetEdge)],
            vec![param("height", "20", Dim::Length), param("ang", "90", Dim::Angle)],
            vec![],
            vec![],
        ),
        "members" => {
            let mut slots = vec![slot("Layout", K::Path)];
            let mut words = vec![];
            let align = if c("Section") == "Sketch" {
                slots.push(slot("Section", K::Profile));
                "origin"
            } else {
                words.push(("section", "SHS 40x40x3".to_string()));
                "center"
            };
            words.push(("align", align.to_string()));
            // a corner of its own at the nodes picked: its kind and gap there
            slots.push(maybe("Corner Points", K::Points));
            words.push(("corner_at", "butt".to_string()));
            (
                slots,
                vec![
                    param("gap", "0", Dim::Length),
                    param("dx", "0", Dim::Length),
                    param("dy", "0", Dim::Length),
                    param("turn", "0", Dim::Angle),
                    param("corner_gap", "0", Dim::Length),
                ],
                words,
                vec![("mirror", false)],
            )
        }
        "endtrim" => (
            vec![slot("Members", K::Members), slot("To", K::TrimTo)],
            vec![param("gap", "0", Dim::Length)],
            vec![],
            vec![],
        ),
        "endcap" => (
            vec![slot("Member", K::Member)],
            vec![param("t", "5", Dim::Length)],
            vec![],
            vec![],
        ),
        "cornerplate" => (
            vec![slot("Member A", K::Member), slot("Member B", K::Member)],
            vec![
                param("size", "80", Dim::Length),
                param("t", "5", Dim::Length),
                param("corner_cut", "10", Dim::Length),
            ],
            vec![],
            vec![],
        ),
        _ => return None,
    })
}

/// What a kind's features are named: the kind, or Inventor's word for a work feature.
fn base_name(kind: &str) -> &str {
    match kind {
        "point" => "wpoint",
        "axis" => "waxis",
        "plane" => "wplane",
        "circular" | "mirror" | "sketchpattern" => "pattern",
        "emboss" => "emboss",
        "direct" | "moveface" | "offsetface" | "deleteface" | "replaceface" => "direct",
        "facefillet" => "fillet",
        "members" => "frame",
        "endtrim" => "trim",
        "endcap" => "cap",
        "cornerplate" => "plate",
        k => k,
    }
}

impl FgTool {
    /// The form for a ribbon kind, named the kind and the first free count (`fillet1`).
    pub fn new(kind: &'static str, taken: &[String]) -> Option<FgTool> {
        let choices = choices_of(kind);
        let pick = |n: &str| choices.iter().find(|c| c.name == n).map_or("", |c| c.word());
        let (slots, params, words, flags) = form_of(kind, &pick)?;
        let base = base_name(kind);
        let name = (1..)
            .map(|k| format!("{base}{k}"))
            .find(|n| !taken.contains(n))
            .expect("a free name");
        Some(FgTool {
            kind,
            name,
            choices,
            slots,
            params,
            words,
            flags,
            options: vec![],
            active: 0,
            error: None,
            note: None,
            again: false,
            edit: None,
            units: "mm",
            at_guess: None,
            side_tried: None,
        })
    }

    /// Its lengths typed in the part's unit: what it shows already (mm) says so.
    pub fn set_units(&mut self, units: &'static str) {
        self.units = units;
        if units == "mm" {
            return;
        }
        for p in &mut self.params {
            if p.dim == Dim::Length && p.text.trim().parse::<f64>().is_ok() && p.expr.is_none() {
                p.text = format!("{} mm", p.text.trim());
            }
        }
    }

    /// Its text as the op OK applies: a new statement at the end, or the edited one replaced.
    pub fn op(&self, text: String) -> forge_lang::error::Op {
        match &self.edit {
            Some(id) => forge_lang::error::Op::ReplaceFeature { id: id.clone(), text },
            None => forge_lang::error::Op::AddFeature { before: None, text },
        }
    }

    pub fn choice(&self, name: &str) -> &'static str {
        self.choices
            .iter()
            .find(|c| c.name == name)
            .map_or("", |c| c.word())
    }

    /// Another item of a choice: the form that item has, keeping every pick, value, word and
    /// flag the new form shares with the old.
    pub fn choose(&mut self, name: &str, k: usize) {
        let Some(c) = self.choices.iter_mut().find(|c| c.name == name) else {
            return;
        };
        if k >= c.items.len() || c.on == k {
            return;
        }
        let was = c.on;
        c.on = k;
        // the old form's own words, as it starts: a word still at its default takes the new
        // form's default (a sketched section's `origin`, not the library's `center`)
        let old_defaults = {
            let mut before = self.choices.clone();
            if let Some(b) = before.iter_mut().find(|b| b.name == name) {
                b.on = was;
            }
            let pick = |n: &str| before.iter().find(|c| c.name == n).map_or("", |c| c.word());
            form_of(self.kind, &pick).map(|f| f.2).unwrap_or_default()
        };
        let choices = self.choices.clone();
        let pick = |n: &str| choices.iter().find(|c| c.name == n).map_or("", |c| c.word());
        let Some((mut slots, mut params, mut words, mut flags)) = form_of(self.kind, &pick) else {
            return;
        };
        for s in &mut slots {
            if let Some(o) = self.slots.iter().find(|o| o.label == s.label && o.kind == s.kind) {
                s.picks = o.picks.clone();
            }
        }
        for p in &mut params {
            if let Some(o) = self.params.iter().find(|o| o.name == p.name && o.dim == p.dim) {
                p.text = o.text.clone();
            }
        }
        for w in &mut words {
            let untouched = |o: &(&str, String)| old_defaults.iter().any(|d| d.0 == o.0 && d.1 == o.1);
            if let Some(o) = self.words.iter().find(|o| o.0 == w.0)
                && !untouched(o)
            {
                w.1 = o.1.clone();
            }
        }
        for f in &mut flags {
            if let Some(o) = self.flags.iter().find(|o| o.0 == f.0) {
                f.1 = o.1;
            }
        }
        (self.slots, self.params, self.words, self.flags) = (slots, params, words, flags);
        self.active = self
            .slots
            .iter()
            .position(|s| s.picks.is_empty() && !s.optional)
            .unwrap_or(0);
        self.error = None;
    }

    /// A word's list, the first item its value when it has none.
    pub fn offer(&mut self, word: &'static str, items: Vec<String>) {
        if let Some(w) = self.words.iter_mut().find(|w| w.0 == word)
            && w.1.is_empty()
            && let Some(first) = items.first()
        {
            w.1 = first.clone();
        }
        self.options.retain(|o| o.0 != word);
        self.options.push((word, items));
    }

    pub fn options_of(&self, word: &str) -> Option<&[String]> {
        self.options.iter().find(|o| o.0 == word).map(|o| o.1.as_slice())
    }

    pub fn flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x.0 == f && x.1)
    }

    pub fn word(&self, w: &str) -> String {
        self.words
            .iter()
            .find(|x| x.0 == w)
            .map_or(String::new(), |x| x.1.clone())
    }

    /// What the status bar asks for: the active slot's pick, or OK.
    pub fn prompt(&self) -> String {
        match self.slots.get(self.active) {
            Some(s) if s.picks.is_empty() || s.kind.many() => format!("{}: {}", s.label, s.kind.prompt()),
            _ => "Enter or OK to finish, Esc to cancel".into(),
        }
    }

    /// A pick for the active slot: taken (a second pick of the same thing takes it back out), or
    /// refused, typed.
    pub fn take(&mut self, text: Result<String, ToolError>) {
        let Some(s) = self.slots.get_mut(self.active) else {
            return;
        };
        match text {
            Ok(t) => {
                self.error = None;
                if let Some(k) = s.picks.iter().position(|p| *p == t) {
                    s.picks.remove(k);
                } else if s.kind.many() || s.kind == SlotKind::Points {
                    s.picks.push(t);
                } else {
                    s.picks = vec![t];
                    // a one-pick slot filled: the next slot waiting takes the next pick
                    if let Some(k) = self.slots.iter().position(|x| x.picks.is_empty() && !x.optional) {
                        self.active = k;
                    }
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn the(&self, label: &'static str) -> &Slot {
        self.slots
            .iter()
            .find(|s| s.label == label)
            .expect("the tool's own slot")
    }

    fn one(&self, label: &'static str) -> Result<String, ToolError> {
        self.the(label)
            .picks
            .first()
            .cloned()
            .ok_or(ToolError::MissingPick { slot: label })
    }

    fn all(&self, label: &'static str, sep: &str) -> Result<String, ToolError> {
        let s = self.the(label);
        if s.picks.is_empty() {
            return Err(ToolError::MissingPick { slot: label });
        }
        Ok(s.picks.join(sep))
    }

    /// One pick as itself, several as a list.
    fn listed(&self, label: &'static str) -> Result<String, ToolError> {
        let s = self.the(label);
        match s.picks.len() {
            0 => Err(ToolError::MissingPick { slot: label }),
            1 => Ok(s.picks[0].clone()),
            _ => Ok(format!("[{}]", s.picks.join(", "))),
        }
    }

    /// Exactly `n` points, as a tuple: three for a plane, two for an axis.
    fn tuple(&self, label: &'static str, n: usize) -> Result<String, ToolError> {
        let s = self.the(label);
        if s.picks.len() < n {
            return Err(ToolError::TooFew { slot: label, want: n });
        }
        Ok(format!("({})", s.picks[..n].join(", ")))
    }

    /// What a pattern copies: its features, or the whole part.
    fn of(&self) -> Result<String, ToolError> {
        if self.choice("Pattern") == "Solids" {
            Ok("whole: true".into())
        } else {
            Ok(format!("of: {}", self.listed("Features")?))
        }
    }

    /// An output choice as keyword arguments: join is the default.
    fn output(&self) -> &'static str {
        match self.choice("Output") {
            "Cut" => ", cut: true",
            "Intersect" => ", keep: true",
            "New Solid" => ", new: true",
            _ => "",
        }
    }

    /// The params it writes above its statement: `param NAME.x = VALUE`.
    fn param_lines(&self, skip: &[&str]) -> Result<String, ToolError> {
        let mut out = String::new();
        for p in self.params.iter().filter(|p| !skip.contains(&p.name)) {
            if p.expr.as_deref() == Some(p.text.as_str()) {
                out.push_str(&format!("param {}.{} = {}\n", self.name, p.name, p.text));
                continue;
            }
            let v = super::fg::parse_in(&p.text, p.dim, self.units)
                .filter(|l| l.dim == p.dim)
                .ok_or(ToolError::BadNumber {
                    param: p.name,
                    text: p.text.clone(),
                })?;
            let lit = Literal {
                value: v.value,
                dim: p.dim,
            };
            out.push_str(&format!("param {}.{} = {}\n", self.name, p.name, lit.text()));
        }
        Ok(out)
    }

    fn value(&self, name: &str) -> Option<f64> {
        let p = self.params.iter().find(|p| p.name == name)?;
        // an expression kept as written: no known number, so never one a default leaves out
        if p.expr.as_deref() == Some(p.text.as_str()) {
            return Some(f64::NAN);
        }
        super::fg::parse_in(&p.text, p.dim, self.units).map(|v| v.value)
    }

    /// The Forge text it adds: its params, then its statement.
    pub fn text(&self) -> Result<String, ToolError> {
        let n = &self.name;
        let ok_name = !n.is_empty()
            && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !n.starts_with(|c: char| c.is_ascii_digit());
        if !ok_name {
            return Err(ToolError::BadName { name: n.clone() });
        }
        let mut skip: Vec<&str> = vec![];
        let mut after = String::new();
        // statements a form writes above its params and its own (a hole's placing sketch)
        let mut before = String::new();
        let stmt = match self.kind {
            "extrude" if matches!(self.choice("Body"), "Thin" | "Surface") => {
                let body = self.choice("Body");
                let no = |why: String| ToolError::NoForgeForm { tool: "Extrude", why };
                let extent = match self.choice("Extents") {
                    "Distance" => "d",
                    "Symmetric" => "sym(d)",
                    e => {
                        return Err(no(format!(
                            "a {} goes a distance or symmetric, not {e}",
                            body.to_lowercase()
                        )));
                    }
                };
                let path = self.listed("Path")?;
                if body == "Surface" {
                    if self.choice("Output") != "Join" {
                        return Err(no(
                            "a surface is a body of its own: it joins, cuts or intersects nothing".into(),
                        ));
                    }
                    format!("{n} = extrude({path}, {extent}, surface: true)")
                } else {
                    let f = match self.choice("Output") {
                        "Cut" => "cut",
                        "Intersect" => {
                            return Err(no("a thin wall joins, cuts or is new: no Intersect".into()));
                        }
                        _ => "extrude",
                    };
                    let side = match self.choice("Side") {
                        "In" => ", side: \"in\"",
                        "Both" => ", side: \"both\"",
                        _ => "",
                    };
                    let new = if self.choice("Output") == "New Solid" {
                        ", new: true"
                    } else {
                        ""
                    };
                    format!("{n} = {f}({path}, {extent}, thin: t{side}{new})")
                }
            }
            "extrude" => {
                let output = self.choice("Output");
                let f = match output {
                    "Cut" => "cut",
                    "Intersect" => "intersect",
                    _ => "extrude",
                };
                let extent = match self.choice("Extents") {
                    "Symmetric" => "sym(d)".to_string(),
                    "Through All" => "thru_all".into(),
                    "To Next" => "next".into(),
                    "To" => format!("upto({})", self.all("To", " + ")?),
                    _ if self.flag("flip") => "-d".into(),
                    _ => "d".into(),
                };
                let taper = match self.value("taper") {
                    Some(v) if v != 0.0 => ", taper: taper",
                    _ => {
                        skip.push("taper");
                        ""
                    }
                };
                let new = if output == "New Solid" { ", new: true" } else { "" };
                format!("{n} = {f}({}, {extent}{taper}{new})", self.one("Profile")?)
            }
            "revolve" if self.choice("Body") == "Surface" => {
                if self.choice("Output") != "Join" {
                    return Err(ToolError::NoForgeForm {
                        tool: "Revolve",
                        why: "a surface is a body of its own: it joins or cuts nothing".into(),
                    });
                }
                format!(
                    "{n} = revolve({}, axis: {}{}, surface: true)",
                    self.listed("Path")?,
                    self.one("Axis")?,
                    if self.choice("Extents") == "Angle" {
                        ", angle: ang"
                    } else {
                        ""
                    }
                )
            }
            "revolve" => format!(
                "{n} = revolve({}, axis: {}{}{})",
                self.one("Profile")?,
                self.one("Axis")?,
                if self.choice("Extents") == "Angle" {
                    ", angle: ang"
                } else {
                    ""
                },
                match self.choice("Output") {
                    "Cut" => ", cut: true",
                    "New Solid" => ", new: true",
                    _ => "",
                }
            ),
            "hole" => {
                let to;
                let depth = match self.choice("Termination") {
                    "Distance" => "depth",
                    "To Next" => "next",
                    "To" => {
                        to = format!("upto({})", self.all("To", " + ")?);
                        to.as_str()
                    }
                    _ => "thru",
                };
                let mut args = match self.choice("Placement") {
                    // a sketch on the face, its point held the distances from the two edges
                    // projected onto it (each bound outside the sketch, as a sketch block reads
                    // edges); the guess puts the solve on the face's side of each
                    "Linear" => {
                        skip.extend(["d1", "d2"]);
                        let (face, e1, e2) = (self.one("Face")?, self.one("Edge 1")?, self.one("Edge 2")?);
                        let lit = |v: f64| {
                            Literal {
                                value: v,
                                dim: Dim::Length,
                            }
                            .text()
                        };
                        let d = |name: &'static str| -> Result<String, ToolError> {
                            let p = self
                                .params
                                .iter()
                                .find(|p| p.name == name)
                                .expect("its own param");
                            if p.expr.as_deref() == Some(p.text.as_str()) {
                                return Ok(p.text.clone());
                            }
                            super::fg::parse_in(&p.text, p.dim, self.units)
                                .filter(|l| l.dim == Dim::Length)
                                .map(|l| lit(l.value))
                                .ok_or(ToolError::BadNumber {
                                    param: name,
                                    text: p.text.clone(),
                                })
                        };
                        let (d1, d2) = (d("d1")?, d("d2")?);
                        let (gx, gy) = self
                            .at_guess
                            .unwrap_or((self.value("d2").unwrap_or(1.0), self.value("d1").unwrap_or(1.0)));
                        let r = |v: f64| lit((v * 1e6).round() / 1e6);
                        before = format!(
                            "{n}_e1 = {e1}\n{n}_e2 = {e2}\nparam {n}_at.d1 = {d1}\nparam {n}_at.d2 = {d2}\n\
                             param {n}_at.gx = {}\nparam {n}_at.gy = {}\nsketch {n}_at(on: {face}) {{\n  \
                             e1 = project({n}_e1)\n  e2 = project({n}_e2)\n  p = near(gx, gy)\n  \
                             distance(p, e1, d1)\n  distance(p, e2, d2)\n}}\n",
                            r(gx),
                            r(gy)
                        );
                        format!("at: {n}_at.p")
                    }
                    "Concentric" => format!("at: {}.center, on: {}", self.one("Edge")?, self.one("Face")?),
                    _ => format!("at: {}", self.listed("Points")?),
                };
                match self.choice("Type") {
                    "Simple" => {
                        args.push_str(", d: d");
                        match self.choice("Seat") {
                            "Counterbore" => args.push_str(", cbore: (cbore_d, cbore_depth)"),
                            "Countersink" => args.push_str(", csink: (csink_d, csink_angle)"),
                            _ => {}
                        }
                    }
                    t => {
                        let fit = if t == "Tapped" {
                            "tap".to_string()
                        } else {
                            self.word("fit")
                        };
                        args.push_str(&format!(", screw: {:?}, fit: {fit:?}", self.word("screw")));
                        match self.choice("Seat") {
                            "Counterbore" => args.push_str(", seat: \"cbore\""),
                            "Countersink" => args.push_str(", seat: \"csink\""),
                            _ => {}
                        }
                    }
                }
                args.push_str(&format!(", depth: {depth}"));
                if depth == "depth" {
                    args.push_str(", point: point");
                }
                if self.flag("flip") {
                    args.push_str(", flip: true");
                }
                if self.choice("Type") == "Tapped" && self.flag("model thread") {
                    after = format!("{n}_thread = thread(hole: {n}, length: \"full\")\n");
                }
                format!("{n} = hole({args})")
            }
            "fillet" => match self.choice("Type") {
                "Variable" => format!(
                    "{n} = fillet({}, vary(r_start, r_end))",
                    self.all("Edges", " + ")?
                ),
                "Face" => format!(
                    "{n} = facefillet({}, {}, r)",
                    self.all("Face Set 1", " + ")?,
                    self.all("Face Set 2", " + ")?
                ),
                "Full Round" => format!(
                    "{n} = fullround(side: {}, middle: {}, other: {})",
                    self.one("Side Face 1")?,
                    self.one("Center Face")?,
                    self.one("Side Face 2")?
                ),
                _ => format!(
                    "{n} = fillet({}, r{})",
                    self.all("Edges", " + ")?,
                    if self.flag("tangent chain") {
                        ""
                    } else {
                        ", only: true"
                    }
                ),
            },
            "facefillet" => format!(
                "{n} = facefillet({}, {}, r)",
                self.all("Face Set 1", " + ")?,
                self.all("Face Set 2", " + ")?
            ),
            "chamfer" => {
                let size = match self.choice("Type") {
                    "Two Distances" => format!(", d2: d2, face: {}", self.one("Face")?),
                    "Distance and Angle" => format!(", angle: ang, face: {}", self.one("Face")?),
                    _ => String::new(),
                };
                let corner = if self.choice("Corner") == "Point" {
                    ", corner: \"point\""
                } else {
                    ""
                };
                format!("{n} = chamfer({}, d{size}{corner})", self.all("Edges", " + ")?)
            }
            "shell" => {
                let open = self.the("Remove Faces");
                let out = if self.choice("Direction") == "Outside" {
                    ", out: true"
                } else {
                    ""
                };
                if open.picks.is_empty() {
                    format!("{n} = shell(t{out})")
                } else {
                    format!("{n} = shell(t, open: {}{out})", open.picks.join(" + "))
                }
            }
            "draft" => format!(
                "{n} = draft({}, angle: ang, neutral: {})",
                self.all("Faces", " + ")?,
                self.one("Neutral")?
            ),
            "thread" => {
                let len = self.word("length");
                let len = if len.parse::<f64>().is_ok() {
                    len
                } else {
                    format!("{len:?}")
                };
                format!(
                    "{n} = thread(face: {}, spec: {:?}, length: {len}{})",
                    self.one("Cylinder")?,
                    self.word("spec"),
                    if self.flag("flip") { ", flip: true" } else { "" }
                )
            }
            "combine" => {
                let op = match self.choice("Operation") {
                    "Cut" => "cut",
                    "Intersect" => "keep",
                    _ => "add",
                };
                format!(
                    "{n} = combine({}, tools: {}, op: {op:?}{})",
                    self.one("Target")?,
                    self.listed("Tools")?,
                    if self.flag("keep toolbody") {
                        ", keep: true"
                    } else {
                        ""
                    }
                )
            }
            "plane" => match self.choice("Method") {
                "Angle" => format!(
                    "{n} = workplane(turn: {}, about: {}, by: ang)",
                    self.one("Plane")?,
                    self.one("Axis")?
                ),
                "Tangent" => format!(
                    "{n} = workplane(tangent: {}, parallel: {})",
                    self.one("Cylinder")?,
                    self.one("Parallel To")?
                ),
                "3 Points" => format!("{n} = workplane(through: {})", self.tuple("Points", 3)?),
                "Midplane" => format!(
                    "{n} = workplane(mid: ({}, {}))",
                    self.one("Plane 1")?,
                    self.one("Plane 2")?
                ),
                "Normal" => format!(
                    "{n} = workplane(normal_to: {}, at: {})",
                    self.one("Curve")?,
                    self.one("Point")?
                ),
                "On" => format!("{n} = workplane(on: {})", self.one("Plane")?),
                _ => format!("{n} = workplane(offset: {}, d: d)", self.one("Face")?),
            },
            "axis" => match self.choice("Method") {
                "2 Points" => format!("{n} = workaxis(through: {})", self.tuple("Points", 2)?),
                "2 Planes" => format!(
                    "{n} = workaxis(meet: ({}, {}))",
                    self.one("Plane 1")?,
                    self.one("Plane 2")?
                ),
                "Normal" => format!(
                    "{n} = workaxis(normal_to: {}, at: {})",
                    self.one("Plane")?,
                    self.one("Point")?
                ),
                _ => format!("{n} = workaxis(along: {})", self.one("Along")?),
            },
            "point" => match self.choice("Method") {
                "On Point" => format!("{n} = workpoint(at: {})", self.one("Point")?),
                "Intersection" => format!(
                    "{n} = workpoint(meet: ({}, {}))",
                    self.one("Edge")?,
                    self.one("Face")?
                ),
                _ => format!("{n} = workpoint(mid: {})", self.one("Edge")?),
            },
            "ucs" => match self.choice("Method") {
                "3 Points" => format!("{n} = ucs(through: {})", self.tuple("Points", 3)?),
                _ => format!("{n} = ucs(on: {})", self.one("Plane")?),
            },
            "pattern" => {
                let two = self.the("Direction 2").picks.first().cloned();
                if two.is_none() {
                    skip.extend(["n2", "step2"]);
                }
                format!(
                    "{n} = pattern({}, layout: lin(along: {}, n: n, step: step{}))",
                    self.of()?,
                    self.one("Direction")?,
                    two.map_or(String::new(), |d| format!(", along2: {d}, n2: n2, step2: step2"))
                )
            }
            "circular" => {
                let spread = if self.flag("incremental") {
                    ", each: ang"
                } else if self.value("ang") == Some(360.0) {
                    skip.push("ang");
                    ""
                } else {
                    ", over: ang"
                };
                format!(
                    "{n} = pattern({}, layout: circ(about: {}, n: n{spread}))",
                    self.of()?,
                    self.one("Rotation Axis")?
                )
            }
            "mirror" => format!(
                "{n} = pattern({}, layout: mirror(on: {}))",
                self.of()?,
                self.one("Mirror Plane")?
            ),
            "sketchpattern" => format!(
                "{n} = pattern(of: {}, layout: places(at: {}))",
                self.listed("Features")?,
                self.listed("Points")?
            ),
            "emboss" => {
                let font = self.word("font");
                if font.is_empty() {
                    return Err(ToolError::NoForgeForm {
                        tool: "Emboss",
                        why: "no font: put a .ttf or .otf file beside the part and choose it".into(),
                    });
                }
                let at = self
                    .the("Position")
                    .picks
                    .first()
                    .map_or(String::new(), |p| format!(", at: {p}"));
                let wrap = if self.flag("wrap to cylinder") {
                    ", wrap: wrap"
                } else {
                    skip.push("wrap");
                    ""
                };
                format!(
                    "{n} = text({:?}, font: {font:?}, size: size, depth: depth, on: {}{at}{}{wrap})",
                    self.word("text"),
                    self.one("Face")?,
                    if self.choice("Type") == "Engrave" {
                        ", cut: true"
                    } else {
                        ""
                    },
                )
            }
            "import" => {
                let file = self.word("file");
                if file.is_empty() {
                    return Err(ToolError::NoForgeForm {
                        tool: "Import",
                        why: "no file: put a .step, .brep or .iges file beside the part and choose it".into(),
                    });
                }
                let op = match self.choice("Output") {
                    "Join" => "add",
                    "Cut" => "cut",
                    _ => "new",
                };
                format!("{n} = import({file:?}, op: {op:?})")
            }
            "split" => match self.choice("Method") {
                "Split Face" => format!(
                    "{n} = splitface(faces: {}, by: {})",
                    self.all("Faces", " + ")?,
                    self.one("Split Tool")?
                ),
                "Trim Solid" => format!(
                    "{n} = split(by: {}, keep: {:?})",
                    self.one("Split Tool")?,
                    if self.flag("keep other side") {
                        "minus"
                    } else {
                        "plus"
                    }
                ),
                _ => format!("{n} = split(by: {}, keep: \"both\")", self.one("Split Tool")?),
            },
            "thicken" => {
                let side = match self.choice("Side") {
                    "In" => "in",
                    "Both" => "both",
                    _ => "out",
                };
                format!(
                    "{n} = thicken({}, t, side: {side:?}{})",
                    self.all("Faces", " + ")?,
                    self.output()
                )
            }
            "offsetface" => format!("{n} = offset_faces({}, d)", self.all("Faces", " + ")?),
            "moveface" => format!(
                "{n} = move_faces({}, along: {}, d: d)",
                self.all("Faces", " + ")?,
                self.one("Direction")?
            ),
            "direct" => match self.choice("Action") {
                "Size" => format!("{n} = offset_faces({}, d)", self.all("Faces", " + ")?),
                "Rotate" => format!(
                    "{n} = move_faces({}, about: {}, by: ang)",
                    self.all("Faces", " + ")?,
                    self.one("Axis")?
                ),
                "Delete" => format!("{n} = delete_faces({})", self.all("Faces", " + ")?),
                _ => format!(
                    "{n} = move_faces({}, along: {}, d: d)",
                    self.all("Faces", " + ")?,
                    self.one("Direction")?
                ),
            },
            "deleteface" => format!("{n} = delete_faces({})", self.all("Faces", " + ")?),
            "replaceface" => format!(
                "{n} = replace_face({}, with: {})",
                self.one("Existing Face")?,
                self.one("New Face")?
            ),
            "movebody" | "copybody" => {
                let motion = if self.choice("Motion") == "Spin" {
                    format!("spin(about: {}, by: ang)", self.one("Axis")?)
                } else {
                    format!("slide(along: {}, by: d)", self.one("Direction")?)
                };
                format!(
                    "{n} = {}(of: {}, moves: [{motion}])",
                    self.kind,
                    self.listed("Bodies")?
                )
            }
            "sweep" => format!(
                "{n} = sweep({}, along: {}{})",
                self.one("Profile")?,
                self.listed("Path")?,
                self.output()
            ),
            "loft" => {
                let ps = self.the("Profiles");
                if ps.picks.len() < 2 {
                    return Err(ToolError::TooFew {
                        slot: "Profiles",
                        want: 2,
                    });
                }
                format!(
                    "{n} = loft([{}]{}{})",
                    ps.picks.join(", "),
                    if self.flag("smooth") { ", smooth: true" } else { "" },
                    self.output()
                )
            }
            "coil" => format!(
                "{n} = coil({}, axis: {}, pitch: pitch, turns: turns{}{})",
                self.one("Profile")?,
                self.one("Axis")?,
                if self.flag("left hand") {
                    ", left: true"
                } else {
                    ""
                },
                self.output()
            ),
            "rib" => format!(
                "{n} = rib({}, t{})",
                self.listed("Path")?,
                if self.flag("flip") { ", flip: true" } else { "" }
            ),
            "sheet" => format!(
                "{n} = sheet({}, t: t, r: r, bend: kfactor(k))",
                self.one("Profile")?
            ),
            "flange" => format!("{n} = flange({}, height: height, angle: ang)", self.one("Edge")?),
            "cutout" => format!(
                "{n} = cutout({}, wall: {})",
                self.one("Profile")?,
                self.one("Wall")?
            ),
            "unfold" => format!("{n} = unfold({})", self.one("Sheet")?),
            "members" => {
                let zero = |p: &str| self.value(p) == Some(0.0);
                let library = self.choice("Section") != "Sketch";
                let section = if library {
                    format!("{:?}", self.word("section").trim())
                } else {
                    self.one("Section")?
                };
                let mut args = format!("{}, section: {section}", self.listed("Layout")?);
                let align = self.word("align");
                let align = align.trim();
                if !ALIGNS.contains(&align) {
                    return Err(ToolError::NoForgeForm {
                        tool: "Members",
                        why: format!("{align:?} is no alignment: one of {}", ALIGNS.join(", ")),
                    });
                }
                if align != if library { "center" } else { "origin" } {
                    args.push_str(&format!(", align: {align:?}"));
                }
                if zero("dx") && zero("dy") {
                    skip.extend(["dx", "dy"]);
                } else {
                    args.push_str(", offset: (dx, dy)");
                }
                if zero("turn") {
                    skip.push("turn");
                } else {
                    args.push_str(", turn: turn");
                }
                if self.flag("mirror") {
                    args.push_str(", mirror: true");
                }
                if self.choice("Corner") == "Butt" {
                    args.push_str(", corner: \"butt\"");
                }
                if zero("gap") {
                    skip.push("gap");
                } else {
                    args.push_str(", gap: gap");
                }
                let at = &self.the("Corner Points").picks;
                if at.is_empty() {
                    skip.push("corner_gap");
                } else {
                    let kind = self.word("corner_at");
                    let kind = kind.trim();
                    if !CORNERS.contains(&kind) {
                        return Err(ToolError::NoForgeForm {
                            tool: "Members",
                            why: format!("{kind:?} is no corner: miter or butt"),
                        });
                    }
                    let each: Vec<String> = at
                        .iter()
                        .map(|p| format!("({p}, {kind:?}, corner_gap)"))
                        .collect();
                    args.push_str(&format!(", corners: [{}]", each.join(", ")));
                }
                format!("{n} = members({args})")
            }
            "endtrim" => {
                let gap = if self.value("gap") == Some(0.0) {
                    skip.push("gap");
                    ""
                } else {
                    ", gap: gap"
                };
                let end = match self.choice("End") {
                    "Start" => ", end: \"start\"",
                    "End" => ", end: \"end\"",
                    _ => "",
                };
                format!(
                    "{n} = end_trim({}, to: {}{gap}{end})",
                    self.listed("Members")?,
                    self.one("To")?
                )
            }
            "endcap" => {
                let end = if self.choice("End") == "Start" {
                    "start"
                } else {
                    "end"
                };
                format!("{n} = end_cap({}, end: {end:?}, t: t)", self.one("Member")?)
            }
            "cornerplate" => {
                let chamfer = if self.value("corner_cut") == Some(0.0) {
                    skip.push("corner_cut");
                    ""
                } else {
                    ", chamfer: corner_cut"
                };
                format!(
                    "{n} = corner_plate({}, {}, size: size, t: t{chamfer})",
                    self.one("Member A")?,
                    self.one("Member B")?
                )
            }
            _ => {
                return Err(ToolError::NoForgeForm {
                    tool: self.kind,
                    why: "no Forge form".into(),
                });
            }
        };
        // an edit replaces its own statement only: what a new one writes after it stays as it is
        if self.edit.is_some() {
            after.clear();
        }
        Ok(format!("{before}{}{stmt}\n{after}", self.param_lines(&skip)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_surface_and_a_thin_wall_write_their_extrudes_and_read_back() {
        let mut t = FgTool::new("extrude", &[]).unwrap();
        t.choose("Body", 2);
        assert_eq!(t.slots[0].label, "Path");
        assert!(t.params.iter().all(|p| p.name != "taper") && t.flags.is_empty());
        t.take(Ok("walls.wall".into()));
        assert_eq!(
            t.text().unwrap(),
            "param extrude1.d = 10\nextrude1 = extrude(walls.wall, d, surface: true)\n"
        );
        // a surface goes a distance or symmetric, and joins nothing
        t.choose("Extents", 2);
        assert_eq!(t.text().map_err(|e| e.kind()), Err("NoForgeForm"));
        t.choose("Extents", 1);
        t.choose("Output", 1);
        assert_eq!(t.text().map_err(|e| e.kind()), Err("NoForgeForm"));
        // a thin wall, both sides, cut
        t.choose("Body", 1);
        t.choose("Side", 2);
        assert_eq!(
            t.text().unwrap(),
            "param extrude1.d = 10\nparam extrude1.t = 2\nextrude1 = cut(walls.wall, sym(d), thin: t, side: \"both\")\n"
        );
        // Edit Feature reads each back into its form
        for (text, body) in [
            ("sk = 1\ns1 = extrude(walls.wall, d, surface: true)\n", "Surface"),
            ("s1 = extrude(walls.wall, d, thin: t, side: \"in\")\n", "Thin"),
        ] {
            let e = FgTool::edit("s1", text).unwrap_or_else(|e| panic!("{text}: {}", e.text()));
            assert_eq!(e.choice("Body"), body, "{text}");
            assert_eq!(e.slots[0].picks, vec!["walls.wall".to_string()]);
        }
    }

    #[test]
    fn a_revolved_surface_writes_its_call() {
        let mut t = FgTool::new("revolve", &[]).unwrap();
        t.choose("Body", 1);
        t.take(Ok("sk.arc1".into()));
        t.active = 1;
        t.take(Ok("Z".into()));
        assert_eq!(
            t.text().unwrap(),
            "param revolve1.ang = 360deg\nrevolve1 = revolve(sk.arc1, axis: Z, angle: ang, surface: true)\n"
        );
    }

    #[test]
    fn members_write_their_section_and_only_the_options_set() {
        let mut t = FgTool::new("members", &[]).unwrap();
        assert_eq!(t.name, "frame1");
        assert_eq!(t.text(), Err(ToolError::MissingPick { slot: "Layout" }));
        t.take(Ok("lay.r".into()));
        assert_eq!(
            t.text().unwrap(),
            "frame1 = members(lay.r, section: \"SHS 40x40x3\")\n"
        );
        // a second layout piece, butt corners, a gap, a turn, mirrored, aligned to its top
        t.take(Ok("side.post".into()));
        t.choose("Corner", 1);
        t.words.iter_mut().find(|w| w.0 == "align").unwrap().1 = "top".into();
        for (p, v) in [("gap", "0.5"), ("turn", "90")] {
            t.params.iter_mut().find(|x| x.name == p).unwrap().text = v.into();
        }
        t.flags[0].1 = true;
        assert_eq!(
            t.text().unwrap(),
            "param frame1.gap = 0.5\nparam frame1.turn = 90deg\n\
             frame1 = members([lay.r, side.post], section: \"SHS 40x40x3\", align: \"top\", turn: turn, mirror: true, corner: \"butt\", gap: gap)\n"
        );
    }

    #[test]
    fn members_take_a_sketched_section_an_offset_and_a_corner_of_their_own() {
        let mut t = FgTool::new("members", &[]).unwrap();
        t.choose("Section", 1);
        assert!(t.words.iter().all(|w| w.0 != "section"));
        t.take(Ok("lay.r".into()));
        assert_eq!(t.text(), Err(ToolError::MissingPick { slot: "Section" }));
        t.active = 1;
        t.take(Ok("sec.outline".into()));
        t.params.iter_mut().find(|x| x.name == "dy").unwrap().text = "10".into();
        t.active = 2;
        t.take(Ok("lay.p0".into()));
        t.params.iter_mut().find(|x| x.name == "corner_gap").unwrap().text = "1".into();
        assert_eq!(
            t.text().unwrap(),
            "param frame1.dx = 0\nparam frame1.dy = 10\nparam frame1.corner_gap = 1\n\
             frame1 = members(lay.r, section: sec.outline, offset: (dx, dy), corners: [(lay.p0, \"butt\", corner_gap)])\n"
        );
        // an alignment that isn't one is refused, typed
        t.words.iter_mut().find(|w| w.0 == "align").unwrap().1 = "middle".into();
        assert_eq!(t.text().map_err(|e| e.kind()), Err("NoForgeForm"));
    }

    #[test]
    fn end_trim_end_cap_and_corner_plate_write_their_calls() {
        let mut t = FgTool::new("endtrim", &[]).unwrap();
        t.take(Ok("legs[0]".into()));
        t.active = 1;
        t.take(Ok("top[0]".into()));
        assert_eq!(t.text().unwrap(), "trim1 = end_trim(legs[0], to: top[0])\n");
        t.choose("End", 1);
        t.params[0].text = "2".into();
        assert_eq!(
            t.text().unwrap(),
            "param trim1.gap = 2\ntrim1 = end_trim(legs[0], to: top[0], gap: gap, end: \"start\")\n"
        );

        let mut c = FgTool::new("endcap", &[]).unwrap();
        c.take(Ok("legs[0]".into()));
        assert_eq!(
            c.text().unwrap(),
            "param cap1.t = 5\ncap1 = end_cap(legs[0], end: \"end\", t: t)\n"
        );

        let mut p = FgTool::new("cornerplate", &[]).unwrap();
        p.take(Ok("top[0]".into()));
        p.take(Ok("top[1]".into()));
        assert_eq!(
            p.text().unwrap(),
            "param plate1.size = 80\nparam plate1.t = 5\nparam plate1.corner_cut = 10\n\
             plate1 = corner_plate(top[0], top[1], size: size, t: t, chamfer: corner_cut)\n"
        );
    }

    #[test]
    fn a_fillet_writes_its_param_and_its_edges_by_anchor() {
        let mut t = FgTool::new("fillet", &["fillet1".into()]).unwrap();
        assert_eq!(t.name, "fillet2");
        assert_eq!(t.text(), Err(ToolError::MissingPick { slot: "Edges" }));
        t.take(Ok("edges(between: (a.end, a.side(s.c)))".into()));
        t.take(Ok("edges(between: (b.end, b.side(s.c)))".into()));
        assert_eq!(
            t.text().unwrap(),
            "param fillet2.r = 2\nfillet2 = fillet(edges(between: (a.end, a.side(s.c))) + edges(between: (b.end, b.side(s.c))), r)\n"
        );
        // the same pick again takes it back
        t.take(Ok("edges(between: (b.end, b.side(s.c)))".into()));
        assert_eq!(t.slots[0].picks.len(), 1);
        t.take(Err(ToolError::NoStableName { what: "edge" }));
        assert_eq!(t.error.as_ref().map(ToolError::kind), Some("NoStableName"));
        t.params[0].text = "x".into();
        assert!(matches!(t.text(), Err(ToolError::BadNumber { .. })));
    }

    #[test]
    fn every_kind_and_choice_has_a_form_and_writes_typed_params() {
        for k in KINDS {
            let base = FgTool::new(k, &[]).unwrap();
            // each item of each choice, the others at their defaults
            let mut tries = vec![base.clone()];
            for (ci, c) in base.choices.iter().enumerate() {
                for item in 1..c.items.len() {
                    let mut t = base.clone();
                    t.choose(c.name, item);
                    assert_eq!(t.choices[ci].on, item);
                    tries.push(t);
                }
            }
            for mut t in tries {
                for (w, text) in &mut t.words {
                    if text.is_empty() {
                        *text = format!("{w}.ttf");
                    }
                }
                for s in 0..t.slots.len() {
                    t.active = s;
                    let many = t.slots[s].kind.many() || t.slots[s].kind == SlotKind::Points;
                    t.take(Ok(format!("p{s}a")));
                    if many {
                        t.active = s;
                        t.take(Ok(format!("p{s}b")));
                        t.active = s;
                        t.take(Ok(format!("p{s}c")));
                    }
                }
                let choices: Vec<_> = t.choices.iter().map(|c| c.word()).collect();
                let text = t
                    .text()
                    .unwrap_or_else(|e| panic!("{k} {choices:?}: {}", e.text()));
                assert!(text.contains(&format!("{} = ", t.name)), "{k}: {text}");
                assert!(
                    forge_lang::syntax::parse(&text).is_ok(),
                    "{k} {choices:?}: {text}"
                );
            }
        }
        let r = FgTool::new("revolve", &[]).unwrap();
        assert!(
            r.param_lines(&[])
                .unwrap()
                .contains("param revolve1.ang = 360deg")
        );
        let p = FgTool::new("pattern", &[]).unwrap();
        assert!(
            p.param_lines(&[])
                .unwrap()
                .contains("param pattern1.n = count(3)")
        );
    }

    #[test]
    fn a_choice_keeps_what_the_forms_share() {
        let mut t = FgTool::new("fillet", &[]).unwrap();
        t.take(Ok("e1".into()));
        t.choose("Type", 1);
        assert_eq!(t.slots[0].picks, ["e1"]);
        assert_eq!(t.params[0].name, "r_start");
        t.choose("Type", 0);
        assert_eq!(t.slots[0].picks, ["e1"]);
        assert_eq!(t.params[0].text, "2");
        let mut h = FgTool::new("hole", &[]).unwrap();
        h.take(Ok("s.points".into()));
        h.choose("Type", 2);
        h.choose("Seat", 1);
        h.choose("Termination", 0);
        let text = h.text().unwrap();
        assert!(
            text.contains(
                "hole1 = hole(at: s.points, screw: \"M5\", fit: \"tap\", seat: \"cbore\", depth: depth, point: point)"
            ),
            "{text}"
        );
        assert!(
            text.ends_with("hole1_thread = thread(hole: hole1, length: \"full\")\n"),
            "{text}"
        );
    }
}
