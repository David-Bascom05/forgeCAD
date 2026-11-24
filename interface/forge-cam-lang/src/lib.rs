//! The `machining` document: CAM in .fg (research/cam/build.md §6).
//!
//! A machining document is a .fg file whose first statement is the bare name `machining`. Its
//! statements are the language's own (`shared`, `param`, `NAME = call(..)`), read by the same
//! parser; the calls are CAM's built-ins:
//!
//! ```text
//! machining
//! shared tol = 0.01
//! f     = part("flange.fg", group: "N")             # the part, its ISO 513 material group
//! m     = machine("generic-vmc3")
//! t1    = tool("face-50", t: 1, stickout: 30, holder: "bt40-er32")
//! t2    = tool("flat-6", t: 2, stickout: 25, holder: "bt40-er32")
//! op1   = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: "top-center", tol: tol)
//! face  = facing(op1, tool: t1, step: 1)
//! prof  = contour(op1, tool: t2, side: "outside", from: 12, to: 0, step: 4)
//! bore  = pocket(op1, f.bore_h, tool: t2, stepover: 2.5, step: 3)
//! bolts = drill(op1, f.bolts, plan: "auto")
//! prog  = program(op1, post: "fanuc", number: 1001)
//! ```
//!
//! Every operation's path is posted, read back (the round trip), and the path the text means is
//! what is certified: the cutter against the part (gouge), the shank and holder against the part
//! and the stock, rapids against both. An op whose checks are not all `pass` is refused, with the
//! check, where, and a fix. The reply is `forgecad.cam/1`.
use forge_lang::syntax::{self, Arg, Expr, ExprKind, StmtKind};
pub mod export;
pub mod holes;
pub mod life;
pub mod sim;
pub mod toollog;
pub mod toolstore;

use forge_agent::json::Json;
use forge_cam::ops::{self, CheckOut, Heights, OpPath, PMove, Phase, Stock, ToolUse};
use forge_cam_geom::check::Face;
use forge_cam_geom::region::{Loop, P2, Region, Seg, p2};
use forge_post::{CycleKind, Rec};
use forge_tooling::{Kind, Library, feeds};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Builds the staged stock even where the checks against it cannot fail (tests/stock_skip.rs,
/// the skip's differential test). Off by default.
#[doc(hidden)]
pub static FULL_STOCK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The peck retract and chip-break lift (mm) the posted cycles are read with: the controls'
/// usual values by default; tests/peck.rs sets others to show the verdicts do not depend on them.
#[doc(hidden)]
pub static CYCLE_PARAMS: std::sync::Mutex<(f64, f64)> =
    std::sync::Mutex::new((forge_post::PECK_CLEAR, forge_post::CHIP_LIFT));

/// Whether a source is a machining document.
pub fn is_machining(src: &str) -> bool {
    match syntax::parse(src) {
        Ok(p) => {
            matches!(p.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "machining")
        }
        Err(_) => false,
    }
}

#[derive(Clone, Debug)]
enum V {
    Num(f64),
    Str(String),
    Tuple(Vec<V>),
    Part(usize),
    Machine(String),
    Tool(usize),
    Setup(usize),
    Holes(usize, String),
    /// A part's sketch, by name.
    Sketch(usize, String),
    Stock(StockSpec),
    Vise(ViseSpec),
    /// Soft jaws: their index, and the jaws as a part.
    Jaws(usize, usize),
    /// A height of the model: the plane of a horizontal face of part `part` (`f.base.end`) at
    /// `z` in its frame, plus `off` along the setup's up (`f.base.end + 0.5`); an op's `from:`
    /// or `to:` reads it in its setup's frame (a turned-over part's is h − z).
    Height {
        part: usize,
        z: f64,
        off: f64,
    },
    #[allow(dead_code)]
    Op(usize),
}

#[derive(Clone, Debug)]
struct ViseSpec {
    name: String,
    axis: usize,
    clamp: Option<f64>,
    parallel: f64,
    clearance: f64,
}

#[derive(Clone, Debug)]
enum StockSpec {
    /// Margins x, y, above, and below the part.
    Margin([f64; 3], f64),
    Box {
        min: [f64; 3],
        max: [f64; 3],
    },
    /// A bar (`cylinder`) or a tube (`id`) along z about the part's centre: its diameter (or
    /// the part's round extent plus the radial margin), its bore, the margin above, below.
    Round {
        d: Option<f64>,
        id: Option<f64>,
        radial: f64,
        above: f64,
        below: f64,
    },
    /// A stock modelled as its own part file (a prism along z, in the part's frame).
    Solid(String),
}

struct PartIn {
    /// The part's source and where it is, to read its sketches.
    src: String,
    pdir: std::path::PathBuf,
    /// The workholding a setup of this part gets unless it names one (soft jaws, held to be cut).
    hold: Option<forge_cam::fixture::Fixture>,
    /// This is part `k` turned over about x (a later setup's view of it).
    turn_of: Option<usize>,
    file: String,
    faces: Vec<Face>,
    holes: Vec<holes::HoleInfo>,
    bbox: ([f64; 3], [f64; 3]),
    group: String,
    /// The work material (forge-material name or alias), when `part(.., material:)` names it.
    material: Option<String>,
}

struct SetupIn {
    name: String,
    part: usize,
    machine: String,
    stock: Stock,
    origin: [f64; 3],
    wcs: u32,
    tol: f64,
    clearance: f64,
    heights: Heights,
    /// The work offset: the machine coordinates of the WCS origin (as G54 holds them; the
    /// tool's length comes from H). None: not placed (G5 then checks the spans only).
    at: Option<[f64; 3]>,
    /// The in-process stock as the ops so far left it.
    ip: forge_cam::stock::InProcess,
    /// The stock's kind as declared: box, cylinder, tube or solid.
    stock_kind: &'static str,
    /// The workholding, if the setup names one.
    fixture: Option<forge_cam::fixture::Fixture>,
    /// The setup this one continues, the part turned over.
    from: Option<usize>,
}

/// One operation, as made and as checked.
/// What a 3-axis finishing op derived and did, for the reply and the dialogs.
#[derive(Clone, Debug, PartialEq)]
pub struct Finish {
    /// The cusp asked for (mm), when the spacing was derived from one.
    pub cusp: Option<f64>,
    /// The cusp the spacing is proven to bound on a plane at the op's slopes (mm): the scallop
    /// module's certified enclosure, upper end (an upper bound only for a bull on a slope).
    pub cusp_bound: Option<f64>,
    /// The horizontal stepover and the step-down used (mm; 0 when the op has none).
    pub stepover: f64,
    pub stepdown: f64,
    /// Links: stayed down, lifted, retracted.
    pub links: [usize; 3],
}

pub struct OpOut {
    pub name: String,
    pub kind: &'static str,
    pub setup: usize,
    pub tool: Option<usize>,
    pub paths: Vec<OpPath>,
    pub feeds: Option<feeds::Feeds>,
    pub checks: Vec<CheckOut>,
    pub pairs: usize,
    pub cells: usize,
    pub status: &'static str,
    pub refusal: Option<String>,
    pub fix: Option<String>,
    pub moves: usize,
    pub length_cut: f64,
    pub length_rapid: f64,
    pub time_s: f64,
    pub least: f64,
    pub plan: Vec<String>,
    /// The stock it is asked to leave on the part (δ).
    pub leave: f64,
    /// The target it is certified against when not the part: P ∖ Δ for declared removals.
    target: Option<Vec<Face>>,
    /// Declared removals (Δ): what each is and where.
    pub removals: Vec<String>,
    /// The paths as the posted text means them (read back from the program, in the part's
    /// frame), one per path once a program posts the op: what the checks' `move_index` counts.
    pub posted: Vec<OpPath>,
    /// The stock this op starts from (G2, G3 are checked against it).
    stock_before: Option<forge_cam::stock::InProcess>,
    /// What the op provably clears: (region, or the whole box; its final floor).
    voids: Vec<(Option<Region>, f64)>,
    /// The lowest tool tip of its posted paths, as the text reads back (∞ until posted).
    pub deepest: f64,
    /// 3-axis finishing (forge-cam-3d's ops): the derived spacing and the links.
    pub finish: Option<Finish>,
}

/// `Cam::check`'s answer for one op: clear only if it was posted and every check passed.
#[derive(Clone, Debug)]
pub struct OpChecks {
    pub op: String,
    pub clear: bool,
    pub checks: Vec<CheckOut>,
    pub pairs: usize,
    pub cells: usize,
}

pub struct ProgramOut {
    pub name: String,
    pub setups: Vec<usize>,
    /// The ops it posts, in order (every op of its setups, or those `ops:` names).
    pub ops: Vec<usize>,
    /// With `split: "setup"`: the one setup this part of the program posts (its name is then
    /// `PROGRAM/SETUP`).
    pub setup: Option<String>,
    pub post: String,
    pub text: Option<String>,
    pub lines: usize,
    pub error: Option<String>,
    pub round_trip: bool,
}

/// Each built-in's arguments: keywords, and `#0`, `#1`.. for positionals. `named_of` refuses any
/// other (a typed error naming the nearest); CAM-UI's schema is checked against it, and
/// research/forge-lang/lean/COVERAGE.md lists it (lang_coverage). A reader of a new keyword adds
/// it here in the same change.
pub const ARGS: &[(&str, &[&str])] = &[
    // FEEDS-SPEEDS' keywords (feeds/model) are listed with each built-in that takes them
    ("part", &["#0", "group", "material"]),
    ("machine", &["#0"]),
    (
        "tool",
        &[
            "#0",
            "t",
            "stickout",
            "holder",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "setup",
        &[
            "#0",
            "machine",
            "stock",
            "origin",
            "wcs",
            "at",
            "tol",
            "clearance",
            "clear",
            "retract",
            "fixture",
            "from",
        ],
    ),
    (
        "facing",
        &[
            "#0",
            "tool",
            "to",
            "step",
            "leave_axial",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "clear",
        &[
            "#0",
            "tool",
            "from",
            "to",
            "step",
            "stepover",
            "leave",
            "leave_axial",
            "ramp",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "contour",
        &[
            "#0",
            "tool",
            "side",
            "from",
            "to",
            "step",
            "leave",
            "leave_axial",
            "direction",
            "finish",
            "finish_step",
            "spring",
            "lead",
            "lead_radius",
            "lead_sweep",
            "lead_length",
            "entry",
            "overlap",
            "tabs",
            "tab_spacing",
            "tab_at",
            "tab_width",
            "tab_height",
            "tab_shape",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "pocket",
        &[
            "#0",
            "#1",
            "tool",
            "from",
            "to",
            "step",
            "stepover",
            "leave",
            "leave_axial",
            "ramp",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "drill",
        &[
            "#0",
            "#1",
            "plan",
            "vc",
            "rpm",
            "f",
            "feed",
            "peck",
            "max_power",
            "max_torque",
            "tap_vc",
            "tap_rpm",
        ],
    ),
    (
        "engrave",
        &[
            "#0",
            "#1",
            "tool",
            "depth",
            "step",
            "from",
            "offset",
            "axial_offset",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "slot",
        &[
            "#0",
            "#1",
            "tool",
            "depth",
            "step",
            "from",
            "ramp",
            "offset",
            "axial_offset",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "thread_mill",
        &[
            "#0",
            "#1",
            "tool",
            "length",
            "passes",
            "spring",
            "hand",
            "starts",
            "lead",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "edge_break",
        &[
            "#0",
            "tool",
            "at",
            "size",
            "tip_offset",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "bore",
        &[
            "#0",
            "#1",
            "tool",
            "pitch",
            "ramp",
            "finish_leave",
            "spring",
            "leave",
            "leave_axial",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
    (
        "soft_jaw",
        &[
            "#0",
            "blank",
            "from",
            "grip",
            "fit",
            "tool",
            "relief",
            "min_contact",
        ],
    ),
    (
        "program",
        &[
            "#0",
            "post",
            "number",
            "ops",
            "split",
            "decimals",
            "feed_decimals",
            "coolant",
            // read by CAM-UI's export (cam/ui-3 export.rs)
            "file",
            "ext",
            "out",
            "comment",
            "tools",
        ],
    ),
    // the 3-axis ops (`step` the step-down, `down` its old name until the next train; parallel
    // lists `step` only to refuse it with the fix)
    (
        "parallel",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "stepover",
            "angle",
            "sample",
            "step",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
            "grid",
        ],
    ),
    (
        "waterline",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "step",
            "down",
            "grid",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
            "stepover",
        ],
    ),
    (
        "zlevel",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "step",
            "down",
            "stepover",
            "grid",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "pencil",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "kink",
            "min_len",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
            "stepover",
        ],
    ),
    (
        "scallop",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "stepover",
            "walls",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "rest",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "after",
            "after_leave",
            "stepover",
            "min",
            "margin",
            "angle",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "steep_shallow",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "threshold",
            "step",
            "down",
            "stepover",
            "overlap",
            "angle",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "spiral",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "stepover",
            "center",
            "r0",
            "r1",
            "direction",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "radial",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "stepover",
            "center",
            "r0",
            "r1",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "ramping",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "slope_min",
            "step",
            "down",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    (
        "flat",
        &[
            "#0",
            "tool",
            "leave",
            "leave_axial",
            "floor",
            "area",
            "from",
            "to",
            "grid",
            "sample",
            "cusp",
            "stay",
            "lift",
            "stepover",
            "angle",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
            "ramp",
        ],
    ),
    ("box", &["margin", "below", "min", "max"]),
    ("cylinder", &["d", "margin", "below"]),
    ("tube", &["d", "id", "margin", "below"]),
    ("solid", &["#0"]),
    ("vise", &["#0", "across", "clamp", "parallel", "clearance"]),
    // 3-axis surfacing (CAM-3AXIS): the common op keys, area/floor/grid, each op's own, the feeds
    (
        "rest",
        &[
            "#0",
            "tool",
            "step",
            "ramp",
            "leave",
            "leave_axial",
            "stepover",
            "area",
            "floor",
            "grid",
            "after",
            "after_leave",
            "min",
            "vc",
            "rpm",
            "chip",
            "fz",
            "feed",
            "ap",
            "ae",
            "max_angle",
            "tilt",
            "plunge_feed",
            "ramp_feed",
            "lead_in_feed",
            "max_power",
            "max_torque",
            "max_deflection",
            "min_feed",
            "max_feed",
        ],
    ),
];

/// The arguments `fname` takes (None: not a built-in with a list).
pub fn args_of(fname: &str) -> Option<&'static [&'static str]> {
    ARGS.iter().find(|(n, _)| *n == fname).map(|(_, a)| *a)
}

/// Every argument given is one `fname` takes, or the refusal: `UnknownArg`, with the nearest
/// (by edit distance, within a third of the name) as the fix, else the list.
fn check_args(fname: &str, m: &BTreeMap<String, V>) -> Result<(), String> {
    let Some(takes) = args_of(fname) else {
        return Ok(());
    };
    for k in m.keys() {
        if takes.contains(&k.as_str()) {
            continue;
        }
        if let Some(n) = k.strip_prefix('#') {
            let most = takes.iter().filter(|t| t.starts_with('#')).count();
            return Err(format!(
                "UnknownArg: `{fname}` takes {most} positional argument{} (argument {} is one too many)",
                if most == 1 { "" } else { "s" },
                n.parse::<usize>().unwrap_or(0) + 1
            ));
        }
        let named: Vec<&str> = takes.iter().copied().filter(|t| !t.starts_with('#')).collect();
        let near = named
            .iter()
            .map(|t| (edit_distance(k, t), *t))
            .min()
            .filter(|(d, t)| *d <= k.len().max(t.len()).div_ceil(3));
        return Err(match near {
            Some((_, t)) => {
                format!("UnknownArg: `{k}` is not an argument of `{fname}` (did you mean `{t}`?)")
            }
            None => format!(
                "UnknownArg: `{k}` is not an argument of `{fname}` (it takes: {})",
                named.join(", ")
            ),
        });
    }
    Ok(())
}

/// Levenshtein distance (bytes; the names are ASCII).
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let sub = prev[j - 1] + usize::from(a[i - 1] != b[j - 1]);
            cur[j] = sub.min(prev[j] + 1).min(cur[j - 1] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// A program's own post options (`program(.., decimals:, feed_decimals:, coolant:)`).
#[derive(Clone, Copy, Debug, Default)]
struct PostOpts {
    decimals: Option<u32>,
    feed_decimals: Option<u32>,
    coolant: Option<forge_post::Coolant>,
}

/// A leftover's boxes, why, radius and lowest tip (as `Leftover` holds them).
type Reason = (Vec<[f64; 4]>, &'static str, Option<f64>, Option<f64>);

/// G4 for one setup: in each height band, the final stock less the part grown by the
/// tolerance: what is left. Empty bands are omitted.
#[derive(Clone, Debug)]
pub struct Leftover {
    pub setup: usize,
    pub z0: f64,
    pub z1: f64,
    /// Each left-over region's box, xmin ymin xmax ymax.
    pub boxes: Vec<[f64; 4]>,
    /// Why, each a fact decided exactly (else "uncut", which claims nothing):
    /// - "held by fixture": the band is within the jaws' grip, and the region's box within the
    ///   fixture's clearance of one of its pieces (no op of this setup may reach it);
    /// - "below every tool tip": the band's top is at or under the lowest tip of every posted
    ///   move of the setup (`below`), and the setup continues none: no tool body went into it;
    /// - "tool radius": at the band's own section height, no disk of radius `r` (the smallest
    ///   tool of the chain) inside the free region (the stock's box grown, less the part's
    ///   section) meets the region: no tool of that radius or more can reach it there (inside
    ///   corners, slots narrower than the tool);
    /// - "uncut".
    pub why: &'static str,
    /// For "tool radius": the radius.
    pub r: Option<f64>,
    /// For "below every tool tip": the lowest tip.
    pub below: Option<f64>,
}

/// A setup as `Cam::setup_view` shows it.
#[derive(Clone, Copy, Debug)]
pub struct SetupView<'a> {
    pub name: &'a str,
    pub stock: Stock,
    pub machine: &'a str,
    pub group: &'a str,
    pub at: Option<[f64; 3]>,
    pub clear: f64,
    pub retract: f64,
}

pub struct Cam {
    pub leftover: Vec<Leftover>,
    pub leftover_done: bool,
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    parts: Vec<PartIn>,
    tools: Vec<ToolUse>,
    setups: Vec<SetupIn>,
    pub ops: Vec<OpOut>,
    pub programs: Vec<ProgramOut>,
    lib: Library,
    jaws: Vec<forge_cam::fixture::SoftJaws>,
}

impl PartIn {
    /// Its lowest plus its highest z: turning over about x maps z to h − z.
    fn h(&self) -> f64 {
        self.bbox.0[2] + self.bbox.1[2]
    }
}

/// Whether a posted feed F = q / 10^k is over the limit `max_feed` (mm/min), decided exactly: a
/// whole-number limit (< 2^53) times 10^k is an integer, compared with q in u128 (10^22 · 2^53
/// < 2^127), so F exactly at the limit passes; any other limit scaled in outward-rounded
/// arithmetic (its lower end), so a pass is never a rounding.
fn feed_over(q: i64, k: u32, max_feed: f64) -> bool {
    let k = k.min(22);
    if max_feed >= 0.0 && max_feed.fract() == 0.0 && max_feed < 9007199254740992.0 {
        return q > 0 && (q as u128) > (max_feed as u128) * 10u128.pow(k);
    }
    let fmax = forge_cam_geom::ival::Ival::pt(max_feed)
        .mul(forge_cam_geom::ival::Ival::pt(10f64.powi(k as i32)))
        .lo;
    (q as f64) > fmax || fmax.is_nan()
}

/// The refusal for a hole that is not drilled along −z in this setup: one that points up opens
/// on the other side (a setup that holds the part that way up drills it); any other direction
/// needs a setup of its own.
fn not_down(hn: &str, dir: [f64; 3]) -> String {
    if dir[0] == 0.0 && dir[1] == 0.0 && dir[2] > 0.0 {
        format!(
            "the hole `{hn}` opens on the other side: it is drilled from the side it opens on, in a setup that holds the part that way up"
        )
    } else {
        format!("the hole `{hn}` is not drilled along −z: it needs its own setup")
    }
}

fn dist_pts(a: forge_cam_geom::region::P2, b: forge_cam_geom::region::P2) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn num(v: &V) -> Option<f64> {
    if let V::Num(x) = v { Some(*x) } else { None }
}

fn tuple3(v: &V) -> Option<[f64; 3]> {
    match v {
        V::Tuple(t) if t.len() == 3 => Some([num(&t[0])?, num(&t[1])?, num(&t[2])?]),
        _ => None,
    }
}

impl Cam {
    /// A model reference of part i (`base.end`, `plate.start`) as a height: forge-lang evaluates
    /// it on the finished part (`Options::asks`); it must be a horizontal plane (a face facing
    /// ±z, or a plane).
    fn model_height(&self, i: usize, text: &str) -> Result<V, String> {
        let p = &self.parts[i];
        let opts = forge_lang::Options {
            dir: Some(p.pdir.clone()),
            asks: vec![text.to_string()],
            ..Default::default()
        };
        let (run, _) = forge_lang::run(&p.src, opts, Default::default());
        let frame = match run.asked.first().map(|(_, a)| a) {
            Some(forge_lang::anchors::Asked::Plane(f)) => *f,
            Some(forge_lang::anchors::Asked::Faces(_, Some(f))) => *f,
            Some(forge_lang::anchors::Asked::Faces(_, None)) => {
                return Err(format!(
                    "`{text}` is not a plane: a height is a horizontal face's"
                ));
            }
            _ => return Err(format!("`{text}` is not in the part")),
        };
        let n = frame.z().get();
        if !(n.x == 0.0 && n.y == 0.0 && n.z.abs() == 1.0) {
            return Err(format!(
                "`{text}` is not horizontal: a height is a face facing up or down"
            ));
        }
        Ok(V::Height {
            part: i,
            z: frame.origin.z,
            off: 0.0,
        })
    }

    /// A bar, tube or modelled stock for part i: its box and its section (a prism along z),
    /// shown to hold the part: in each band between the part's flat faces, the part's section
    /// less the stock's is empty (exact booleans), and the part's heights are within the
    /// stock's. A part that is not 2.5D there is refused (its section is not known).
    fn shaped_stock(&self, i: usize, spec: &StockSpec, dir: &Path) -> Result<(Stock, Region), String> {
        use forge_cam_geom::boolean::{Op, boolean};
        let p = &self.parts[i];
        let (plo, phi) = p.bbox;
        // the part's sections, one per band between its flat faces' heights
        let mut zs = vec![plo[2], phi[2]];
        for f in &p.faces {
            if let Some(forge_cam_geom::slab::Prism::Flat { z, .. }) = &f.prism
                && *z > plo[2]
                && *z < phi[2]
            {
                zs.push(*z);
            }
        }
        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // heights within 1e-6 are one (the box's ends are its faces' padded by their mu)
        zs.dedup_by(|b, a| (*b - *a).abs() <= 1e-6);
        let mut secs = vec![];
        for w in zs.windows(2) {
            let zm = 0.5 * (w[0] + w[1]);
            let sec = forge_cam::section::section(&p.faces, zm, 1e-5).map_err(|e| {
                format!("stock: the part's section at z = {zm}: {e:?} (a shaped stock needs a 2.5D part)")
            })?;
            secs.push((zm, sec));
        }
        let (st, outline) = match spec {
            StockSpec::Round {
                d,
                id,
                radial,
                above,
                below,
            } => {
                let c = p2(0.5 * (plo[0] + phi[0]), 0.5 * (plo[1] + phi[1]));
                let r = match d {
                    Some(d) => 0.5 * d,
                    None => {
                        // the part's farthest reach from the centre (an arc by its circle)
                        let mut m: f64 = 0.0;
                        for (_, sec) in &secs {
                            for l in &sec.loops {
                                for sg in &l.segs {
                                    let dist = |q: forge_cam_geom::region::P2| {
                                        ((q.x - c.x).powi(2) + (q.y - c.y).powi(2)).sqrt()
                                    };
                                    m = m.max(match *sg {
                                        Seg::Line { a, b } => dist(a).max(dist(b)),
                                        Seg::Arc { a, c: cc, .. } => dist(cc) + dist_pts(a, cc),
                                    });
                                }
                            }
                        }
                        m + radial
                    }
                };
                let mut outline = forge_cam::stock::disk(c, r);
                if let Some(id) = id {
                    let ri = 0.5 * id;
                    if ri >= r {
                        return Err(format!(
                            "tube: the bore (id {id}) is not inside the tube (d {})",
                            2.0 * r
                        ));
                    }
                    outline
                        .loops
                        .push(forge_cam::stock::disk(c, ri).reversed().loops.remove(0));
                }
                let st = Stock {
                    min: [c.x - r, c.y - r, plo[2] - below],
                    max: [c.x + r, c.y + r, phi[2] + above],
                };
                (st, outline)
            }
            StockSpec::Solid(file) => {
                let path = dir.join(file);
                let src =
                    forge_dirs::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let (run, _) = forge_lang::run(
                    &src,
                    forge_lang::Options {
                        dir: path.parent().map(Path::to_path_buf),
                        ..Default::default()
                    },
                    Default::default(),
                );
                if !run.ok() {
                    return Err(format!(
                        "the stock `{file}` does not build: {:?}",
                        run.errors.first()
                    ));
                }
                let body = run.body.ok_or("the stock has no body")?;
                let faces = forge_cam::lower::lower(&body)
                    .map_err(|e| format!("the stock `{file}` is outside this phase's CAM domain: {e:?}"))?;
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for fc in &faces {
                    for k in 0..3 {
                        lo[k] = lo[k].min(fc.bbox[k].lo + fc.mu);
                        hi[k] = hi[k].max(fc.bbox[k].hi - fc.mu);
                    }
                }
                // a prism along z: walls over its whole height, flat faces only at its ends
                let mu = 1e-6 * (1.0 + hi[2].abs().max(lo[2].abs()));
                for fc in &faces {
                    let ok = match &fc.prism {
                        Some(forge_cam_geom::slab::Prism::Wall { z0, z1, .. }) => {
                            *z0 <= lo[2] + mu && *z1 >= hi[2] - mu
                        }
                        Some(forge_cam_geom::slab::Prism::Flat { z, .. }) => {
                            (z - lo[2]).abs() <= mu || (z - hi[2]).abs() <= mu
                        }
                        None => false,
                    };
                    if !ok {
                        return Err(format!(
                            "the stock `{file}` is not a prism along z (face {}): its section must not change with height",
                            fc.id
                        ));
                    }
                }
                let zm = 0.5 * (lo[2] + hi[2]);
                let outline = forge_cam::section::section(&faces, zm, 1e-5)
                    .map_err(|e| format!("the stock `{file}`'s section: {e:?}"))?;
                (Stock { min: lo, max: hi }, outline)
            }
            _ => unreachable!("shaped_stock takes a round or modelled stock"),
        };
        // (flush within 1e-6: the boxes are the faces' padded by their mu)
        if plo[2] < st.min[2] - 1e-6 || phi[2] > st.max[2] + 1e-6 {
            return Err(format!(
                "stock: the part (z {} to {}) is not within the stock's heights ({} to {})",
                plo[2], phi[2], st.min[2], st.max[2]
            ));
        }
        for (zm, sec) in &secs {
            let out = boolean(sec, &outline, Op::Diff).map_err(|e| format!("{e:?}"))?;
            if !out.is_empty() {
                // where, by its vertices (a hint for the fix)
                let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
                for l in &forge_cam_geom::boolean::region_of(&out).loops {
                    for sg in &l.segs {
                        let q = sg.start();
                        b = [b[0].min(q.x), b[1].min(q.y), b[2].max(q.x), b[3].max(q.y)];
                    }
                }
                return Err(format!(
                    "stock: the part is outside the stock at z = {zm} (x {:.3} to {:.3}, y {:.3} to {:.3})",
                    b[0], b[2], b[1], b[3]
                ));
            }
        }
        Ok((st, outline))
    }

    /// An op's arguments with each model height (`from:`, `to:`) read in setup su's frame.
    fn heights_in(&self, su: usize, a: &BTreeMap<String, V>) -> Result<BTreeMap<String, V>, String> {
        let sp = self.setups[su].part;
        let mut out = a.clone();
        for (k, v) in out.iter_mut() {
            if let V::Height { part, z, off } = *v {
                if k != "from" && k != "to" {
                    return Err(format!("`{k}` takes a number, not a model height"));
                }
                let zs = if sp == part {
                    z
                } else if self.parts[sp].turn_of == Some(part) {
                    self.parts[part].h() - z
                } else {
                    return Err(format!("`{k}`: a height of another part than the setup's"));
                };
                *v = V::Num(zs + off);
            }
        }
        Ok(out)
    }

    /// Whether setup su holds part p: p itself, or p turned over (a hole of p is then read from
    /// the turned part, which carries p's holes in its frame).
    fn holds(&self, su: usize, p: usize) -> bool {
        let sp = self.setups[su].part;
        sp == p || self.parts[sp].turn_of == Some(p)
    }

    /// The part turned over about x, made once.
    fn turned_part(&mut self, i: usize) -> Result<usize, String> {
        if let Some(k) = self.parts.iter().position(|p| p.turn_of == Some(i)) {
            return Ok(k);
        }
        let p = &self.parts[i];
        let h = p.h();
        let faces =
            forge_cam::lower::turned(&p.faces, h).map_err(|e| format!("turning the part over: {e:?}"))?;
        let (lo, hi) = p.bbox;
        let turned = PartIn {
            src: p.src.clone(),
            pdir: p.pdir.clone(),
            hold: None,
            turn_of: Some(i),
            file: p.file.clone(),
            faces,
            // the holes in the turned frame, (x, y, z) -> (x, -y, h - z), their direction
            // turned too: one drilled from below is now drilled from above, and one from
            // above is refused here as before (a hole is drilled from the side it opens on)
            holes: p
                .holes
                .iter()
                .map(|hl| {
                    let mut t = hl.clone();
                    t.places = hl.places.iter().map(|q| [q[0], -q[1], h - q[2]]).collect();
                    t.dir = [hl.dir[0], -hl.dir[1], -hl.dir[2]];
                    t
                })
                .collect(),
            bbox: ([lo[0], -hi[1], h - hi[2]], [hi[0], -lo[1], h - lo[2]]),
            group: p.group.clone(),
            material: p.material.clone(),
        };
        self.parts.push(turned);
        Ok(self.parts.len() - 1)
    }

    fn eval(&self, e: &Expr, env: &BTreeMap<String, V>) -> Result<V, String> {
        Ok(match &e.kind {
            ExprKind::Num(x, _) => V::Num(*x),
            ExprKind::Str(s) => V::Str(s.clone()),
            ExprKind::Name(n) => env.get(n).cloned().ok_or_else(|| format!("unknown name `{n}`"))?,
            ExprKind::Tuple(xs) | ExprKind::List(xs) => {
                V::Tuple(xs.iter().map(|x| self.eval(x, env)).collect::<Result<_, _>>()?)
            }
            ExprKind::Neg(x) => V::Num(-num(&self.eval(x, env)?).ok_or("`-` of a non-number")?),
            ExprKind::Bin(op, a, b) => {
                let (va, vb) = (self.eval(a, env)?, self.eval(b, env)?);
                // a model height plus or minus a length: offset along the setup's up
                match (&va, &vb, op) {
                    (V::Height { part, z, off }, V::Num(d), syntax::BinOp::Add | syntax::BinOp::Sub) => {
                        let d = if matches!(op, syntax::BinOp::Add) { *d } else { -*d };
                        return Ok(V::Height {
                            part: *part,
                            z: *z,
                            off: off + d,
                        });
                    }
                    (V::Num(d), V::Height { part, z, off }, syntax::BinOp::Add) => {
                        return Ok(V::Height {
                            part: *part,
                            z: *z,
                            off: off + d,
                        });
                    }
                    (V::Height { .. }, _, _) | (_, V::Height { .. }, _) => {
                        return Err("a model height takes only + or − a length".into());
                    }
                    _ => {}
                }
                let (a, b) = (
                    num(&va).ok_or("arithmetic on a non-number")?,
                    num(&vb).ok_or("arithmetic on a non-number")?,
                );
                V::Num(match op {
                    syntax::BinOp::Add => a + b,
                    syntax::BinOp::Sub => a - b,
                    syntax::BinOp::Mul => a * b,
                    syntax::BinOp::Div => a / b,
                    _ => return Err("only + - * / in a machining document".into()),
                })
            }
            ExprKind::Attr(x, name) if matches!(x.kind, ExprKind::Attr(..)) => {
                // a chain below a part: a model reference, `f.base.end`
                let mut names = vec![name.clone()];
                let mut root = x.as_ref();
                while let ExprKind::Attr(y, n) = &root.kind {
                    names.push(n.clone());
                    root = y.as_ref();
                }
                names.reverse();
                match self.eval(root, env)? {
                    V::Part(i) => self.model_height(i, &names.join("."))?,
                    _ => return Err(format!("`.{name}` of something that is not a part")),
                }
            }
            ExprKind::Attr(x, name) => match self.eval(x, env)? {
                V::Part(i) => {
                    if self.parts[i].holes.iter().any(|h| &h.name == name) {
                        V::Holes(i, name.clone())
                    } else if forge_lang::sketch_view(
                        &self.parts[i].src,
                        name,
                        forge_lang::Options {
                            dir: Some(self.parts[i].pdir.clone()),
                            ..Default::default()
                        },
                    )
                    .is_some()
                    {
                        V::Sketch(i, name.clone())
                    } else if let Ok(h) = self.model_height(i, name) {
                        h
                    } else {
                        let have: Vec<&str> = self.parts[i].holes.iter().map(|h| h.name.as_str()).collect();
                        return Err(format!(
                            "the part has no hole feature `{name}` (it has: {})",
                            have.join(", ")
                        ));
                    }
                }
                _ => return Err(format!("`.{name}` of something that is not a part")),
            },
            ExprKind::Call(f, args) => {
                let ExprKind::Name(fname) = &f.kind else {
                    return Err("a call of something that is not a built-in".into());
                };
                if fname == "box" {
                    let named = self.named_of("box", args, env)?;
                    if let Some(m) = named.get("margin").and_then(tuple3) {
                        // `below:`: stock under the part too (it rests on that much)
                        let below = match named.get("below") {
                            None => 0.0,
                            Some(V::Num(b)) if *b >= 0.0 => *b,
                            Some(_) => return Err("box: below: a length ≥ 0".into()),
                        };
                        return Ok(V::Stock(StockSpec::Margin(m, below)));
                    }
                    if let (Some(lo), Some(hi)) = (
                        named.get("min").and_then(tuple3),
                        named.get("max").and_then(tuple3),
                    ) {
                        return Ok(V::Stock(StockSpec::Box { min: lo, max: hi }));
                    }
                    return Err("box(margin: (x, y, z)) or box(min: (..), max: (..))".into());
                }
                if fname == "cylinder" || fname == "tube" {
                    let a = self.named_of(fname, args, env)?;
                    let len = |k: &str| -> Result<Option<f64>, String> {
                        match a.get(k) {
                            None => Ok(None),
                            Some(V::Num(x)) if *x > 0.0 && x.is_finite() => Ok(Some(*x)),
                            Some(_) => Err(format!("{fname}: {k}: a length over 0")),
                        }
                    };
                    let (radial, above) = match a.get("margin") {
                        None => (0.0, 0.0),
                        Some(V::Tuple(t)) if t.len() == 2 => match (num(&t[0]), num(&t[1])) {
                            (Some(r), Some(z)) if r >= 0.0 && z >= 0.0 => (r, z),
                            _ => return Err(format!("{fname}: margin: (radial, above), lengths ≥ 0")),
                        },
                        Some(_) => return Err(format!("{fname}: margin: (radial, above), lengths ≥ 0")),
                    };
                    let below = match a.get("below") {
                        None => 0.0,
                        Some(V::Num(b)) if *b >= 0.0 => *b,
                        Some(_) => return Err(format!("{fname}: below: a length ≥ 0")),
                    };
                    let d = len("d")?;
                    let id = len("id")?;
                    if d.is_some() && a.contains_key("margin") && radial > 0.0 {
                        return Err(format!("{fname}: d: or a radial margin, not both"));
                    }
                    if fname == "tube" && id.is_none() {
                        return Err("tube: id: the bore's diameter".into());
                    }
                    if fname == "cylinder" && id.is_some() {
                        return Err("cylinder: no id: (a bored bar is a tube)".into());
                    }
                    return Ok(V::Stock(StockSpec::Round {
                        d,
                        id,
                        radial,
                        above,
                        below,
                    }));
                }
                if fname == "solid" {
                    let a = self.named_of("solid", args, env)?;
                    return match a.get("#0") {
                        Some(V::Str(f)) => Ok(V::Stock(StockSpec::Solid(f.clone()))),
                        _ => Err("solid(\"STOCK.fg\"): the stock as its own part file".into()),
                    };
                }
                if fname == "vise" {
                    let a = self.named_of("vise", args, env)?;
                    let name = match a.get("#0") {
                        Some(V::Str(s)) => s.clone(),
                        _ => {
                            return Err(
                                "vise(\"NAME\", across: \"x\", clamp: MM, parallel: MM, clearance: MM)"
                                    .into(),
                            );
                        }
                    };
                    self.lib.vise(&name).map_err(|e| format!("{e:?}"))?;
                    let axis = match get_str(&a, "across", Some("x"))?.as_str() {
                        "x" => 0,
                        "y" => 1,
                        o => {
                            return Err(format!(
                                "across: \"x\" or \"y\" (the jaws close along it), not `{o}`"
                            ));
                        }
                    };
                    return Ok(V::Vise(ViseSpec {
                        name,
                        axis,
                        clamp: match a.get("clamp") {
                            None => None,
                            Some(V::Num(x)) => Some(*x),
                            Some(_) => return Err("`clamp` must be a number".into()),
                        },
                        parallel: get_num(&a, "parallel", Some(6.0))?,
                        clearance: get_num(&a, "clearance", Some(1.0))?,
                    }));
                }
                return Err(format!(
                    "`{fname}` is a statement's built-in here: give it its own name"
                ));
            }
            _ => return Err("an expression a machining document does not take".into()),
        })
    }

    /// A built-in's arguments by name (`#0`, `#1`.. for positionals), each one it takes
    /// (`ARGS`), or the typed refusal naming the nearest.
    fn named_of(
        &self,
        fname: &str,
        args: &[Arg],
        env: &BTreeMap<String, V>,
    ) -> Result<BTreeMap<String, V>, String> {
        let m = self.named(args, env)?;
        check_args(fname, &m)?;
        Ok(m)
    }

    fn named(&self, args: &[Arg], env: &BTreeMap<String, V>) -> Result<BTreeMap<String, V>, String> {
        let mut m = BTreeMap::new();
        let mut k = 0;
        for a in args {
            let v = self.eval(&a.value, env)?;
            match &a.name {
                Some((n, _)) => {
                    m.insert(n.clone(), v);
                }
                None => {
                    m.insert(format!("#{k}"), v);
                    k += 1;
                }
            }
        }
        Ok(m)
    }
}

/// The 3-axis surfacing and finishing ops.
fn surf3d(kind: &str) -> bool {
    matches!(
        kind,
        "parallel"
            | "waterline"
            | "zlevel"
            | "pencil"
            | "scallop"
            | "rest"
            | "steep_shallow"
            | "spiral"
            | "radial"
            | "ramping"
            | "flat"
    )
}

/// A 3-axis op's step-down: `step:`, or its old name `down:` (both given is refused by op()).
fn step_down(a: &BTreeMap<String, V>, default: f64) -> Result<f64, String> {
    if a.contains_key("step") {
        get_num(a, "step", Some(default))
    } else {
        get_num(a, "down", Some(default))
    }
}

fn get_num(m: &BTreeMap<String, V>, k: &str, default: Option<f64>) -> Result<f64, String> {
    match m.get(k) {
        Some(V::Num(x)) => Ok(*x),
        Some(_) => Err(format!("`{k}` must be a number")),
        None => default.ok_or_else(|| format!("missing `{k}:`")),
    }
}
fn get_str(m: &BTreeMap<String, V>, k: &str, default: Option<&str>) -> Result<String, String> {
    match m.get(k) {
        Some(V::Str(x)) => Ok(x.clone()),
        Some(_) => Err(format!("`{k}` must be a string")),
        None => default
            .map(str::to_string)
            .ok_or_else(|| format!("missing `{k}:`")),
    }
}

/// Runs a machining document (its file's directory finds the part) on the built-in library.
pub fn run(src: &str, dir: &Path) -> Cam {
    run_with_library(src, dir, Library::builtin())
}

/// Runs a machining document on a given tooling library: the one the user's levels make
/// (forge-tooling-builder's `library(project, user_cam)`: the project's records over the user's
/// over the built-in), so tools, holders and inserts made in the builder reach CAM. The
/// document's `cam/*.toml` beside it are added on top, as with `run`.
pub fn run_with_library(src: &str, dir: &Path, lib: Library) -> Cam {
    let mut cam = Cam {
        leftover: vec![],
        leftover_done: false,
        ok: true,
        errors: vec![],
        warnings: vec![],
        parts: vec![],
        tools: vec![],
        setups: vec![],
        ops: vec![],
        programs: vec![],
        lib,
        jaws: vec![],
    };
    let prog = match syntax::parse(src) {
        Ok(p) => p,
        Err(e) => {
            cam.ok = false;
            cam.errors.push(format!("{e:?}"));
            return cam;
        }
    };
    if !is_machining(src) {
        cam.ok = false;
        cam.errors
            .push("not a machining document: its first statement must be `machining`".into());
        return cam;
    }
    // a project library beside the document
    let proj = dir.join("cam");
    if let Ok(rd) = forge_dirs::fs::read_dir(&proj) {
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(s) = forge_dirs::fs::read_to_string(&f)
                && let Err(e) = cam.lib.add(&f.display().to_string(), &s)
            {
                cam.errors.push(e.to_string());
                cam.ok = false;
            }
        }
    }
    let mut env: BTreeMap<String, V> = BTreeMap::new();
    let mut params: BTreeMap<String, BTreeMap<String, Expr>> = BTreeMap::new();
    for st in prog.iter().skip(1) {
        let r: Result<(), String> = (|| {
            match &st.kind {
                StmtKind::Shared { name, value } => {
                    let v = cam.eval(value, &env)?;
                    env.insert(name.clone(), v);
                }
                StmtKind::Param { owner, name, value } => {
                    // the owner's id as text (`a.b`, `a[0]`): a statement's id is its name
                    let o = forge_lang::error::path_text(owner);
                    params.entry(o).or_default().insert(name.clone(), value.clone());
                }
                StmtKind::Assign { name, value } => {
                    let ExprKind::Call(f, args) = &value.kind else {
                        let v = cam.eval(value, &env)?;
                        env.insert(name.clone(), v);
                        return Ok(());
                    };
                    let ExprKind::Name(fname) = &f.kind else {
                        return Err("a call of a non-built-in".into());
                    };
                    // the statement's own params, by their short names
                    let mut local = env.clone();
                    // (its id exactly: `param xop1.d` is not `op1`'s)
                    if let Some(ps) = params.get(name.as_str()) {
                        for (k, e) in ps {
                            let v = cam.eval(e, &env)?;
                            local.insert(k.clone(), v);
                        }
                    }
                    let a = cam.named_of(fname, args, &local)?;
                    // the kernel's clock: std's Instant panics in a browser
                    let t0 = std::env::var("FORGE_CAM_TIME")
                        .is_ok()
                        .then(forge_math::clock::Instant::now);
                    let v = cam.call(name, fname, &a, dir)?;
                    if let Some(t0) = t0 {
                        eprintln!("{name}: {:?}", t0.elapsed());
                    }
                    env.insert(name.clone(), v);
                }
                // a bare statement after the header did nothing and said nothing: every
                // statement of a machining document names what it makes
                StmtKind::Expr(e) => {
                    return Err(match &e.kind {
                        ExprKind::Call(f, _) => match &f.kind {
                            ExprKind::Name(n) => format!("an operation needs a name: `{n}1 = {n}(..)`"),
                            _ => "a statement needs a name: `NAME = ..`".into(),
                        },
                        _ => "a statement needs a name: `NAME = ..`".into(),
                    });
                }
                _ => return Err("a statement a machining document does not take".into()),
            }
            Ok(())
        })();
        if let Err(e) = r {
            cam.ok = false;
            cam.errors
                .push(format!("line {}: {e}", line_of(src, st.span.lo as usize)));
        }
    }
    // each op was certified on the stock every earlier op of its setup, and every op of the
    // setup it continues, left: so those are posted before it (an earlier program, or earlier
    // in its own), or the program is refused
    let mut seen: Vec<usize> = vec![];
    for pi in 0..cam.programs.len() {
        if !cam.programs[pi].round_trip {
            continue;
        }
        let mut missing: Option<(usize, usize)> = None;
        let mut this: Vec<usize> = vec![];
        for &oi in &cam.programs[pi].ops {
            let su = cam.ops[oi].setup;
            let before = |j: usize| {
                !cam.ops[j].paths.is_empty()
                    && ((cam.ops[j].setup == su && j < oi) || Some(cam.ops[j].setup) == cam.setups[su].from)
            };
            if let Some(j) =
                (0..cam.ops.len()).find(|&j| before(j) && !seen.contains(&j) && !this.contains(&j))
            {
                missing = Some((oi, j));
                break;
            }
            this.push(oi);
        }
        if missing.is_none() {
            seen.extend(this);
        }
        if let Some((oi, j)) = missing {
            let p = &mut cam.programs[pi];
            let e = format!(
                "program `{}` posts `{}`, certified on the stock `{}` leaves, which no program posts before it; fix: post `{}` first (an earlier program, or in this one's `ops:`)",
                p.name, cam.ops[oi].name, cam.ops[j].name, cam.ops[j].name
            );
            p.error = Some(e.clone());
            p.text = None;
            p.lines = 0;
            p.round_trip = false;
            cam.ok = false;
            cam.errors.push(e);
        }
    }
    // an op no program posted has no certificate: its checks run on the posted path only
    let posted: Vec<usize> = cam
        .programs
        .iter()
        .filter(|p| p.round_trip)
        .flat_map(|p| p.ops.clone())
        .collect();
    for (oi, o) in cam.ops.iter_mut().enumerate() {
        if o.status == "built" && !posted.contains(&oi) {
            o.status = "unposted";
            o.refusal =
                Some("not certified: no program(..) posts this op (checks run on the posted path)".into());
        }
    }
    cam.leftover();
    for s in &cam.setups {
        if s.at.is_none() {
            cam.warnings.push(format!(
                "setup `{}` is not placed: G5 checked each axis's span against the travel, not the positions; fix: add `at: (x, y, z)` to the setup, the WCS origin in machine coordinates (the work offset G{} holds)",
                s.name,
                54 + s.wcs
            ));
        }
    }
    if cam.ops.iter().any(|o| o.status != "built") || cam.programs.iter().any(|p| p.error.is_some()) {
        cam.ok = false;
    }
    cam
}

fn line_of(src: &str, at: usize) -> usize {
    src[..at.min(src.len())].bytes().filter(|b| *b == b'\n').count() + 1
}

impl Cam {
    /// G4: S_end ⊆ (P ⊕ B_{tol}) ∪ L, L listed. In each band between the stock's and the part's
    /// heights: the stock's material region less the part's section grown by tol, exactly.
    fn leftover(&mut self) {
        let mut all_ok = true;
        for (si, s) in self.setups.iter().enumerate() {
            // a setup a later one continues (the part turned over) ends in that one: its stock,
            // turned, is where the later one starts, so G4 is decided at the chain's end
            if self.setups.iter().any(|t| t.from == Some(si)) {
                continue;
            }
            let part = &self.parts[s.part];
            // G4 grows the part by δ + ε: the stock the chain's ops were asked to leave (the
            // most of them), and the tolerance
            let mut chain = vec![si];
            while let Some(k) = self.setups[*chain.last().unwrap()].from {
                chain.push(k);
            }
            let delta = self
                .ops
                .iter()
                .filter(|o| chain.contains(&o.setup))
                .map(|o| o.leave)
                .fold(0.0, f64::max);
            let grow = delta + s.tol;
            // the lowest posted tip of the setup's ops (a setup continuing none: an earlier one,
            // turned over, could have cut from the other side); none if an op with a path was
            // not posted
            let below = if chain.len() == 1 {
                let mut d = f64::INFINITY;
                let mut all = true;
                for o in self.ops.iter().filter(|o| o.setup == si && !o.paths.is_empty()) {
                    all &= o.deepest.is_finite();
                    d = d.min(o.deepest);
                }
                if all && d.is_finite() { Some(d) } else { None }
            } else {
                None
            };
            // the smallest tool of the chain: half its cutting diameter
            let rmin = self
                .ops
                .iter()
                .filter(|o| chain.contains(&o.setup))
                .flat_map(|o| o.paths.iter().map(|p| 0.5 * self.tools[p.tool].tool.dc))
                .fold(None, |m: Option<f64>, r| Some(m.map_or(r, |m| m.min(r))))
                .filter(|r| *r > 0.0);
            let mut zs = s.ip.levels();
            for f in &part.faces {
                if let Some(forge_cam_geom::slab::Prism::Flat { z, .. }) = &f.prism {
                    zs.push(*z);
                }
            }
            if let Some(f) = &s.fixture {
                zs.push(f.jaw_top);
            }
            zs.retain(|z| *z >= s.ip.min[2] && *z <= s.ip.max[2]);
            zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            // exact: a band ends at the jaws' top exactly, so none straddles it
            zs.dedup();
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                let Ok(Some(mat)) = s.ip.material(zm) else {
                    if s.ip.material(zm).is_err() {
                        all_ok = false;
                    }
                    continue;
                };
                // the part's section for this band, and how far it may be grown. Its own, by
                // δ + ε; or, where it has none (outside the part's heights) or none can be taken
                // (a band thinner than the section's resolution at a face), a section at z′ just
                // past either end, by √((δ + ε)² − dz²), dz the band's farthest height from z′: a
                // point within that of (x, y, z′) ∈ P is within δ + ε of P (P ⊕ B is
                // three-dimensional), whatever z′ is
                let e = 0.5 * grow;
                let mut tries = vec![(zm, grow)];
                for zp in [w[0] - e, w[1] + e] {
                    let dz = (zp - w[0]).abs().max((zp - w[1]).abs());
                    let g2 = grow * grow - dz * dz;
                    if g2 > 0.0 {
                        tries.push((zp, (g2.sqrt() * (1.0 - 1e-12)).max(0.0)));
                    }
                }
                let mut found = None;
                let mut failed = false;
                for (zp, g) in tries {
                    match forge_cam::section::section(&part.faces, zp, 1e-5) {
                        Ok(sec) => {
                            if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                eprintln!("G4 band {w:?}: section at {zp}, grown {g}");
                            }
                            found = Some((sec, g, zp));
                            break;
                        }
                        Err(forge_cam::section::SectionError::Empty) => {}
                        Err(e) => {
                            if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                eprintln!("G4 at z {zp}: section {e:?}");
                            }
                            failed = true;
                        }
                    }
                }
                // the section at the band's own height, for the tool-radius reason
                let own = found
                    .as_ref()
                    .and_then(|(sec, _, zp)| if *zp == zm { Some(sec.clone()) } else { None });
                let left = match found {
                    Some((sec, g, _)) => match forge_cam_geom::offset::offset(&sec, -g) {
                        Ok((grown, _)) => forge_cam_geom::boolean::boolean(
                            &mat,
                            &forge_cam_geom::boolean::region_of(&grown),
                            forge_cam_geom::boolean::Op::Diff,
                        ),
                        Err(e) => Err(e),
                    },
                    None if failed => {
                        all_ok = false;
                        continue;
                    }
                    // no part at or near this height: all the material is left over
                    None => Ok(mat
                        .loops
                        .iter()
                        .map(|l| forge_cam_geom::offset::OLoop { segs: l.segs.clone() })
                        .collect()),
                };
                if let Err(e) = &left
                    && std::env::var("FORGE_CAM_DEBUG").is_ok()
                {
                    eprintln!("G4 band {w:?}: {e:?}");
                }
                match left {
                    Ok(ls) if !ls.is_empty() => {
                        let bx = |ls: &[forge_cam_geom::offset::OLoop]| -> Vec<[f64; 4]> {
                            ls.iter()
                                .map(|l| {
                                    let mut b =
                                        [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
                                    for sg in &l.segs {
                                        for p in [sg.start(), sg.end()] {
                                            b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
                                        }
                                        // an arc may bulge past its ends: its circle's box bounds it
                                        if let Seg::Arc { a, c, .. } = *sg {
                                            let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                                            b = [
                                                b[0].min(c.x - r),
                                                b[1].min(c.y - r),
                                                b[2].max(c.x + r),
                                                b[3].max(c.y + r),
                                            ];
                                        }
                                    }
                                    b
                                })
                                .collect()
                        };
                        // held by the fixture: under the jaws' top, and within the fixture's
                        // clearance of one of its pieces (where no tool may go)
                        let held = |b: &[f64; 4]| match &s.fixture {
                            Some(f) if w[1] <= f.jaw_top => f.boxes().iter().any(|(_, lo, hi)| {
                                let c = f.clearance;
                                b[0] <= hi[0] + c
                                    && b[2] >= lo[0] - c
                                    && b[1] <= hi[1] + c
                                    && b[3] >= lo[1] - c
                                    && w[0] <= hi[2] + c
                                    && w[1] >= lo[2] - c
                            }),
                            _ => false,
                        };
                        // held, by each loop's box; the rest classified as a region
                        let (hl, ul): (Vec<_>, Vec<_>) = ls
                            .into_iter()
                            .partition(|l| held(&bx(std::slice::from_ref(l))[0]));
                        let mut out: Vec<Reason> = vec![(bx(&hl), "held by fixture", None, None)];
                        if below.is_some_and(|d| w[1] <= d) {
                            out.push((bx(&ul), "below every tool tip", None, below));
                        } else {
                            match (rmin, &own) {
                                (Some(r), Some(sec)) if !ul.is_empty() => {
                                    match tool_radius_split(&s.ip, sec, r, &ul) {
                                        Some((reach, not)) => {
                                            out.push((bx(&reach), "uncut", None, None));
                                            out.push((bx(&not), "tool radius", Some(r), None));
                                        }
                                        None => out.push((bx(&ul), "uncut", None, None)),
                                    }
                                }
                                _ => out.push((bx(&ul), "uncut", None, None)),
                            }
                        }
                        for (boxes, why, r, below) in out {
                            if !boxes.is_empty() {
                                self.leftover.push(Leftover {
                                    setup: si,
                                    z0: w[0],
                                    z1: w[1],
                                    boxes,
                                    why,
                                    r,
                                    below,
                                });
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => all_ok = false,
                }
            }
        }
        self.leftover_done = all_ok;
    }

    fn call(&mut self, name: &str, f: &str, a: &BTreeMap<String, V>, dir: &Path) -> Result<V, String> {
        match f {
            "part" => {
                let file = get_str(a, "#0", None)?;
                let path = dir.join(&file);
                let src =
                    forge_dirs::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
                let holes = holes::hole_view(&src, forge_lang::Options::default());
                if !run.ok() {
                    return Err(format!(
                        "the part `{file}` does not build: {:?}",
                        run.errors.first()
                    ));
                }
                let body = run.body.ok_or("the part has no body")?;
                // the 2.5D lowering; a part outside it (spheres, revolved faces trimmed by other
                // curves) is lowered for 3-axis surfacing, those faces as supersets
                let faces = match forge_cam::lower::lower(&body) {
                    Ok(f) => f,
                    Err(e) => forge_cam::lower3d::lower_surfacing(&body)
                        .map(|x| x.0)
                        .map_err(|_| format!("the part is outside this phase's CAM domain: {e:?}"))?,
                };
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for fc in &faces {
                    for k in 0..3 {
                        lo[k] = lo[k].min(fc.bbox[k].lo + fc.mu);
                        hi[k] = hi[k].max(fc.bbox[k].hi - fc.mu);
                    }
                }
                self.parts.push(PartIn {
                    src: src.clone(),
                    pdir: path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
                    hold: None,
                    turn_of: None,
                    file,
                    faces,
                    holes,
                    bbox: (lo, hi),
                    group: get_str(a, "group", Some("N"))?,
                    material: match a.get("material") {
                        Some(V::Str(m)) => Some(m.clone()),
                        _ => None,
                    },
                });
                Ok(V::Part(self.parts.len() - 1))
            }
            "machine" => {
                let n = get_str(a, "#0", None)?;
                self.lib.machine(&n).map_err(|e| e.to_string())?;
                Ok(V::Machine(n))
            }
            "tool" => {
                let id = get_str(a, "#0", None)?;
                let t = self.lib.tool(&id).map_err(|e| e.to_string())?.clone();
                let tn = get_num(a, "t", None)? as u32;
                let stickout = get_num(a, "stickout", Some(t.lcf + 5.0))?;
                let holder = match a.get("holder") {
                    Some(V::Str(h)) => Some(self.lib.holder(h).map_err(|e| e.to_string())?.clone()),
                    _ => None,
                };
                let shape = t
                    .shape(stickout, holder.as_ref())
                    .map_err(|e| format!("tool `{id}`: {e:?}"))?;
                self.tools.push(ToolUse {
                    t: tn,
                    tool: t,
                    holder: holder.map(|h| h.id),
                    stickout,
                    shape,
                });
                Ok(V::Tool(self.tools.len() - 1))
            }
            "setup" => {
                let mut part = match a.get("#0") {
                    Some(V::Part(i)) | Some(V::Jaws(_, i)) => *i,
                    _ => return Err("setup(PART, ..)".into()),
                };
                // a later setup: the part turned over about x, as the earlier one left it
                let from = match a.get("from") {
                    None => None,
                    Some(V::Setup(k)) => {
                        if self.setups[*k].part != part {
                            return Err("from: a setup of the same part".into());
                        }
                        Some(*k)
                    }
                    Some(_) => {
                        return Err("from: SETUP (the setup this one continues, the part turned over)".into());
                    }
                };
                if from.is_some() {
                    part = self.turned_part(part)?;
                }
                let machine = match a.get("machine") {
                    Some(V::Machine(m)) => m.clone(),
                    _ => "generic-vmc3".into(),
                };
                let (plo, phi) = self.parts[part].bbox;
                let mut shape: Option<Region> = None;
                let stock_kind = match a.get("stock") {
                    Some(V::Stock(StockSpec::Round { id: Some(_), .. })) => "tube",
                    Some(V::Stock(StockSpec::Round { .. })) => "cylinder",
                    Some(V::Stock(StockSpec::Solid(_))) => "solid",
                    _ => "box",
                };
                let stock = match a.get("stock") {
                    None if from.is_some() => {
                        let ip = self.setups[from.unwrap()].ip.turned(self.parts[part].h());
                        Stock {
                            min: ip.min,
                            max: ip.max,
                        }
                    }
                    Some(V::Stock(StockSpec::Margin(m, below))) => Stock {
                        min: [plo[0] - m[0], plo[1] - m[1], plo[2] - below],
                        max: [phi[0] + m[0], phi[1] + m[1], phi[2] + m[2]],
                    },
                    Some(V::Stock(StockSpec::Box { min, max })) => Stock { min: *min, max: *max },
                    Some(V::Stock(spec @ (StockSpec::Round { .. } | StockSpec::Solid(_)))) => {
                        let (st, outline) = self.shaped_stock(part, spec, dir)?;
                        shape = Some(outline);
                        st
                    }
                    None => Stock { min: plo, max: phi },
                    _ => return Err("stock: box(..), cylinder(..), tube(..) or solid(..)".into()),
                };
                let origin = match a.get("origin") {
                    Some(V::Str(s)) if s == "top-center" => [
                        0.5 * (stock.min[0] + stock.max[0]),
                        0.5 * (stock.min[1] + stock.max[1]),
                        stock.max[2],
                    ],
                    Some(V::Str(s)) if s == "top-corner" => [stock.min[0], stock.min[1], stock.max[2]],
                    Some(v) => tuple3(v)
                        .ok_or("origin: \"top-center\", \"top-corner\" or (x, y, z) in the part's frame")?,
                    None => [
                        0.5 * (stock.min[0] + stock.max[0]),
                        0.5 * (stock.min[1] + stock.max[1]),
                        stock.max[2],
                    ],
                };
                let fixture = match a.get("fixture") {
                    None => self.parts[part].hold.clone(),
                    Some(V::Vise(v)) => {
                        let l = self.lib.vise(&v.name).map_err(|e| format!("{e:?}"))?;
                        let size = forge_cam::fixture::ViseSize {
                            jaw_width: l.jaw_width,
                            jaw_height: l.jaw_height,
                            jaw_thickness: l.jaw_thickness,
                            max_opening: l.max_opening,
                            bed_depth: l.bed_depth,
                        };
                        let clamp = v
                            .clamp
                            .ok_or("fixture: vise(.., clamp: MM): how much of the stock the jaws grip")?;
                        Some(
                            forge_cam::fixture::vise(
                                &v.name,
                                size,
                                &stock,
                                v.axis,
                                clamp,
                                v.parallel,
                                v.clearance,
                            )
                            .map_err(|e| format!("fixture: {e}"))?,
                        )
                    }
                    Some(V::Jaws(j, _)) => {
                        if from.is_none() {
                            return Err("fixture: soft jaws hold the part as an earlier setup left it: give `from: SETUP`".into());
                        }
                        Some(self.jaws[*j].fixture.clone())
                    }
                    Some(_) => return Err("fixture: vise(\"NAME\", across:, clamp:)".into()),
                };
                let clear = stock.max[2] + get_num(a, "clear", Some(10.0))?;
                let retract = stock.max[2] + get_num(a, "retract", Some(3.0))?;
                self.setups.push(SetupIn {
                    name: name.into(),
                    part,
                    machine,
                    stock,
                    origin,
                    wcs: get_num(a, "wcs", Some(0.0))? as u32,
                    at: match a.get("at") {
                        None => None,
                        Some(v) => {
                            Some(tuple3(v).ok_or("at: (x, y, z), the WCS origin in machine coordinates")?)
                        }
                    },
                    tol: get_num(a, "tol", Some(0.01))?,
                    clearance: get_num(a, "clearance", Some(0.1))?,
                    heights: Heights { clear, retract },
                    ip: match from {
                        // what the earlier setup provably cleared, turned over, inside this stock
                        Some(k) if a.get("stock").is_none() => self.setups[k].ip.turned(self.parts[part].h()),
                        _ => match &shape {
                            Some(o) => forge_cam::stock::InProcess::shaped(stock.min, stock.max, o)?,
                            None => forge_cam::stock::InProcess::new(stock.min, stock.max),
                        },
                    },
                    stock_kind,
                    fixture,
                    from,
                });
                Ok(V::Setup(self.setups.len() - 1))
            }
            "facing" | "contour" | "pocket" | "clear" | "drill" | "engrave" | "slot" | "thread_mill"
            | "edge_break" | "bore" | "parallel" | "waterline" | "zlevel" | "pencil" | "scallop" | "rest"
            | "steep_shallow" | "spiral" | "radial" | "ramping" | "flat" => self.op(name, f, a),
            "soft_jaw" => self.soft_jaw(name, a),
            "program" => {
                let su: Vec<usize> = match a.get("#0") {
                    Some(V::Setup(i)) => vec![*i],
                    Some(V::Tuple(t)) => t
                        .iter()
                        .filter_map(|v| if let V::Setup(i) = v { Some(*i) } else { None })
                        .collect(),
                    _ => return Err("program(SETUP or [SETUPS], post: \"fanuc\", number: N)".into()),
                };
                let post = get_str(a, "post", None).or_else(|_| -> Result<String, String> {
                    Ok(self
                        .lib
                        .machine(&self.setups[su[0]].machine)
                        .map(|m| m.control.clone())
                        .unwrap_or_else(|_| "fanuc".into()))
                })?;
                let number = get_num(a, "number", Some(1000.0))? as u32;
                // `ops:`: some ops of its setups, not all (each must be one of theirs)
                let sel: Option<Vec<usize>> = match a.get("ops") {
                    None => None,
                    Some(v) => {
                        let vs: Vec<&V> = match v {
                            V::Tuple(t) => t.iter().collect(),
                            v => vec![v],
                        };
                        let mut ids = vec![];
                        for v in vs {
                            match v {
                                V::Op(i) if su.contains(&self.ops[*i].setup) => ids.push(*i),
                                V::Op(i) => {
                                    return Err(format!(
                                        "ops: `{}` is an op of setup `{}`, which this program does not post",
                                        self.ops[*i].name, self.setups[self.ops[*i].setup].name
                                    ));
                                }
                                _ => return Err("ops: an op or a list of ops of the program's setups".into()),
                            }
                        }
                        Some(ids)
                    }
                };
                let digits = |k: &str, most: f64| -> Result<Option<u32>, String> {
                    match a.get(k) {
                        None => Ok(None),
                        Some(V::Num(x)) if x.fract() == 0.0 && *x >= 0.0 && *x <= most => Ok(Some(*x as u32)),
                        Some(_) => Err(format!("{k}: a whole number from 0 to {most}")),
                    }
                };
                let coolant = match a.get("coolant") {
                    None => None,
                    Some(V::Str(c)) => {
                        let k = match c.as_str() {
                            "flood" => forge_post::Coolant::Flood,
                            "mist" => forge_post::Coolant::Mist,
                            "through" => forge_post::Coolant::Through,
                            "off" => forge_post::Coolant::Off,
                            _ => return Err("coolant: \"flood\", \"mist\", \"through\" or \"off\"".into()),
                        };
                        // the machine must have it
                        for &si in &su {
                            let m = &self.setups[si].machine;
                            if let Ok(mr) = self.lib.machine(m)
                                && c != "off"
                                && !mr.coolants.iter().any(|x| x == c)
                            {
                                return Err(format!(
                                    "coolant: the machine `{m}` has no {c} coolant (it has: {})",
                                    mr.coolants.join(", ")
                                ));
                            }
                        }
                        Some(k)
                    }
                    Some(_) => return Err("coolant: \"flood\", \"mist\", \"through\" or \"off\"".into()),
                };
                let opts = PostOpts {
                    decimals: digits("decimals", 6.0)?,
                    feed_decimals: digits("feed_decimals", 4.0)?,
                    coolant,
                };
                // `split: "setup"`: one program per setup, `NAME/SETUP`, numbered on from `number`
                match a.get("split") {
                    None => {
                        let out = self.post(name, &su, sel.as_deref(), &post, number, &opts);
                        self.programs.push(out);
                    }
                    Some(V::Str(x)) if x == "setup" => {
                        for (k, &si) in su.iter().enumerate() {
                            let sname = self.setups[si].name.clone();
                            let mut out = self.post(
                                &format!("{name}/{sname}"),
                                &[si],
                                sel.as_deref(),
                                &post,
                                number + k as u32,
                                &opts,
                            );
                            out.setup = Some(sname);
                            self.programs.push(out);
                        }
                    }
                    Some(_) => return Err("split: \"setup\" (one program per setup)".into()),
                }
                Ok(V::Num(0.0))
            }
            _ => Err(format!(
                "`{f}` is not a machining built-in (part, machine, tool, setup, facing, contour, pocket, drill, engrave, thread_mill, edge_break, parallel, waterline, zlevel, pencil, scallop, rest, steep_shallow, spiral, radial, ramping, flat, program, box)"
            )),
        }
    }

    /// Soft jaws for the part as setup `from:` left it, turned over: the pocket is what the jaw
    /// tool cuts of the part's sections in the grip band grown by `fit`; refused unless the
    /// part, and everything `from:` left in the band, seats in it, and the seat carries at least
    /// `min_contact` (default a quarter of the part's largest section).
    fn soft_jaw(&mut self, name: &str, a: &BTreeMap<String, V>) -> Result<V, String> {
        let v = match a.get("#0") {
            Some(V::Vise(v)) => v.clone(),
            _ => return Err("soft_jaw(vise(NAME, across:), blank:, from: SETUP, grip:, fit:, tool:)".into()),
        };
        let k = match a.get("from") {
            Some(V::Setup(k)) => *k,
            _ => return Err("soft_jaw(.., from: SETUP): the setup whose result the jaws hold".into()),
        };
        let tool_r = match a.get("tool") {
            Some(V::Tool(t)) => 0.5 * self.tools[*t].tool.dc,
            _ => return Err("soft_jaw(.., tool: TOOL): the tool that cuts the jaws' pocket".into()),
        };
        let vl = self.lib.vise(&v.name).map_err(|e| format!("{e:?}"))?.clone();
        let bl = self
            .lib
            .jaw_blank(&get_str(a, "blank", None)?)
            .map_err(|e| format!("{e:?}"))?
            .clone();
        let grip = get_num(a, "grip", None)?;
        let fit = get_num(a, "fit", Some(0.02))?;
        let relief = match a.get("relief") {
            None => false,
            Some(V::Str(x)) if x == "corners" => true,
            Some(_) => {
                return Err(
                    "relief: \"corners\" (a relief of the tool's radius at each of the part's corners)"
                        .into(),
                );
            }
        };
        let orig = self.setups[k].part;
        let tp = self.turned_part(orig)?;
        let part = &self.parts[tp];
        let h = part.h();
        let (seat, top) = (part.bbox.0[2], part.bbox.0[2] + grip);
        // the band's sub-bands: between the part's flat faces' heights and the levels of the
        // stock the earlier setup left (turned)
        let mut zs = vec![seat, top];
        for f in &part.faces {
            if let Some(forge_cam_geom::slab::Prism::Flat { z, .. }) = &f.prism
                && *z > seat
                && *z < top
            {
                zs.push(*z);
            }
        }
        for z in self.setups[k].ip.levels() {
            if h - z > seat && h - z < top {
                zs.push(h - z);
            }
        }
        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        zs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        // the jaws grip what the earlier setup left in the band (it contains the part, and the
        // true stock): its proven material, turned over
        let mut band = vec![];
        for w in zs.windows(2) {
            let zm = 0.5 * (w[0] + w[1]);
            let m = self.setups[k]
                .ip
                .material(h - zm)
                .map_err(|e| {
                    format!(
                        "soft_jaw: the stock {} left at z {:.3}: {e}",
                        self.setups[k].name,
                        h - zm
                    )
                })?
                .ok_or_else(|| {
                    format!(
                        "soft_jaw: nothing to grip at z {:.3} of {}",
                        h - zm,
                        self.setups[k].name
                    )
                })?;
            band.push(forge_cam::stock::mirror_y(&m));
        }
        let part = &self.parts[tp];
        // reliefs: a disk of the jaw tool's radius at each convex corner of the part's sections
        // in the band and of what the earlier setup left there
        let mut relief_at = vec![];
        if relief {
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                if let Ok(sec) = forge_cam::section::section(&part.faces, zm, 1e-5) {
                    relief_at.extend(forge_cam::fixture::corners(&sec));
                }
            }
            for b in &band {
                relief_at.extend(forge_cam::fixture::corners(b));
            }
            relief_at.sort_by(|a, b| (a.x, a.y).partial_cmp(&(b.x, b.y)).unwrap());
            relief_at.dedup();
        }
        let height = part.bbox.1[2] - part.bbox.0[2];
        let sj = forge_cam::fixture::soft_jaws(
            name,
            forge_cam::fixture::ViseSize {
                jaw_width: vl.jaw_width,
                jaw_height: vl.jaw_height,
                jaw_thickness: vl.jaw_thickness,
                max_opening: vl.max_opening,
                bed_depth: vl.bed_depth,
            },
            forge_cam::fixture::BlankSize {
                width: bl.width,
                height: bl.height,
                thickness: bl.thickness,
            },
            v.axis,
            &band,
            seat,
            grip,
            height,
            fit,
            tool_r,
            &relief_at,
            v.clearance,
        )
        .map_err(|e| format!("soft_jaw: {e}"))?;
        // nothing the earlier setup left above the part's top may lie in the blank: turned over,
        // it would be inside the jaws below the seat, where the part could not go down
        // (softJaw_needs_grip)
        {
            let ip = &self.setups[k].ip;
            let top_k = h - seat;
            let b = sj.block;
            let blank = forge_cam_geom::region::Region {
                loops: vec![forge_cam_geom::region::Loop {
                    segs: {
                        let pt = |x: f64, y: f64| forge_cam_geom::region::p2(x, y);
                        let (a, bb, c, d) = (
                            pt(b.min[0], b.min[1]),
                            pt(b.max[0], b.min[1]),
                            pt(b.max[0], b.max[1]),
                            pt(b.min[0], b.max[1]),
                        );
                        vec![
                            Seg::Line { a, b: bb },
                            Seg::Line { a: bb, b: c },
                            Seg::Line { a: c, b: d },
                            Seg::Line { a: d, b: a },
                        ]
                    },
                }],
            };
            let mut zs: Vec<f64> = ip.levels().into_iter().filter(|z| *z >= top_k).collect();
            zs.insert(0, top_k);
            zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            zs.dedup();
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                let m = ip
                    .material(zm)
                    .map_err(|e| format!("soft_jaw: the stock {} left: {e}", self.setups[k].name))?;
                if let Some(m) = m {
                    let inside = forge_cam_geom::boolean::boolean(
                        &forge_cam::stock::mirror_y(&m),
                        &blank,
                        forge_cam_geom::boolean::Op::Inter,
                    )
                    .map_err(|e| format!("soft_jaw: {e:?}"))?;
                    if !inside.is_empty() {
                        return Err(format!(
                            "soft_jaw: `{}` leaves stock above the part's top (z {:.3} to {:.3}, its frame), which would sit in the jaws below the seat: face it to z {top_k:.3} in `{}` (facing(.., to: {top_k:.3}))",
                            self.setups[k].name, w[0], w[1], self.setups[k].name
                        ));
                    }
                }
            }
        }
        let largest = band.iter().map(forge_cam::fixture::area).fold(0.0, f64::max);
        let min_contact = get_num(a, "min_contact", Some(0.25 * largest))?;
        if sj.contact < min_contact {
            return Err(format!(
                "soft_jaw: the part's seat in the jaws is {:.1} mm², under the {min_contact:.1} mm² asked: grip where the part is wider, or a lower min_contact",
                sj.contact
            ));
        }
        let b = sj.block;
        self.parts.push(PartIn {
            src: String::new(),
            pdir: Default::default(),
            hold: Some(sj.hold.clone()),
            turn_of: None,
            file: format!("{name} (soft jaws, {})", bl.name),
            faces: sj.part.clone(),
            holes: vec![],
            bbox: (b.min, b.max),
            group: "N".into(),
            material: None,
        });
        self.jaws.push(sj);
        Ok(V::Jaws(self.jaws.len() - 1, self.parts.len() - 1))
    }

    /// The tool's tip along a sketch's curves (lines, arcs, circles; not construction), on a
    /// plane facing +z, in levels down to `depth` below it. What it clears for the stock: each
    /// line's and circle's band of the tip's radius, from the floor up (a V tool clears at least
    /// that: wider above).
    #[allow(clippy::too_many_arguments)]
    fn engrave(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        step: f64,
        slot: bool,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        let what = if slot { "slot" } else { "engrave" };
        let (pi, sk) = match a.get("#1") {
            Some(V::Sketch(p, n)) if *p == self.setups[su].part => (*p, n.clone()),
            Some(V::Sketch(..)) => {
                return Err(format!(
                    "{what}: the sketch is on the part the other way up: cut it in a setup that holds it so"
                ));
            }
            _ => return Err(format!("{what}(SETUP, PART.SKETCH, tool:, depth:)")),
        };
        // a slot: an end mill along the sketch's curves, its whole width; the band of radius
        // dc/2 about each curve is cleared from the corner radius above the floor up (a flat
        // end's disk at its tip, a bull's above its corners)
        let (tip, lift) = if slot {
            match t.tool.kind {
                Kind::Flat => (0.5 * t.tool.dc, 0.0),
                Kind::Bull => (0.5 * t.tool.dc, t.tool.re.max(0.0)),
                _ => return Err("slot: an end mill (flat or bull); a pointed tool engraves".into()),
            }
        } else {
            (0.5 * t.tool.tip_d.max(0.0), 0.0)
        };
        // a ramped entry along each curve's first line, at this angle (degrees); else plunges
        let ramp = match a.get("ramp") {
            None => None,
            Some(V::Num(x)) if slot && *x > 0.0 && *x <= 30.0 => Some(*x),
            Some(_) => {
                return Err(format!(
                    "{what}: ramp: an angle in degrees, over 0 and at most 30 (slots only)"
                ));
            }
        };
        let part = &self.parts[pi];
        let opts = || forge_lang::Options {
            dir: Some(part.pdir.clone()),
            ..Default::default()
        };
        // the sketch's plane, from the part's query (r9-rounded: toolpath placement only; the
        // path is certified as posted)
        let ans = forge_lang::query::ask(&part.src, Some(part.pdir.clone()), None, &format!("frame {sk}"));
        if !ans.ok {
            return Err(format!("{what}: {}", ans.text.trim()));
        }
        let v3 = |k: &str| -> Option<[f64; 3]> {
            let Json::Obj(fs) = &ans.json else { return None };
            let (_, Json::Arr(xs)) = fs.iter().find(|(n, _)| n == k)? else {
                return None;
            };
            let n = |j: &Json| if let Json::Num(x) = j { Some(*x) } else { None };
            Some([n(&xs[0])?, n(&xs[1])?, n(&xs[2])?])
        };
        let (o, x, y, z) = match (v3("origin"), v3("x"), v3("y"), v3("z")) {
            (Some(o), Some(x), Some(y), Some(z)) => (o, x, y, z),
            _ => return Err(format!("{what}: the sketch's frame")),
        };
        if (z[2] - 1.0).abs() > 1e-9 || z[0].abs() > 1e-9 || z[1].abs() > 1e-9 {
            return Err(format!("{what}: the sketch's plane must face +z in this setup"));
        }
        let view = forge_lang::sketch_view(&part.src, &sk, opts()).ok_or(format!("{what}: the sketch"))?;
        let s = &view.s;
        let w = |p: forge_math::Point2| [o[0] + x[0] * p.x + y[0] * p.y, o[1] + x[1] * p.x + y[1] * p.y];
        // each curve: its start, and its moves as (to, Some((centre, cw)) for an arc)
        type Piece = ([f64; 2], Vec<([f64; 2], Option<([f64; 2], bool)>)>);
        let mut curves: Vec<Piece> = vec![];
        // the sketch's handedness in the part's xy: x × y along +z keeps arcs' sense
        let flip = x[0] * y[1] - x[1] * y[0] < 0.0;
        let mut bands: Vec<Region> = vec![];
        // `offset:` moves the tool off each curve, to its left (> 0) or right (< 0) as the
        // curve runs: a line along its normal, an arc or circle to the concentric radius (a
        // trace with compensation; each curve on its own, corners between curves not joined)
        let off = get_num(a, "offset", Some(0.0))?;
        if !off.is_finite() {
            return Err(format!("{what}: offset: a length"));
        }
        for id in s.entity_ids() {
            if s.is_construction(id) {
                continue;
            }
            match s.entity(id) {
                Some(forge_sketch::Entity::Line { a: pa, b: pb }) => {
                    let (mut p0, mut p1) = (w(s.point(pa)), w(s.point(pb)));
                    if off != 0.0 {
                        let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
                        let l = (dx * dx + dy * dy).sqrt();
                        let n = [-dy / l * off, dx / l * off];
                        p0 = [p0[0] + n[0], p0[1] + n[1]];
                        p1 = [p1[0] + n[0], p1[1] + n[1]];
                    }
                    curves.push((p0, vec![(p1, None)]));
                    if tip > 0.0 {
                        bands.push(stadium(p0, p1, tip));
                    }
                }
                Some(forge_sketch::Entity::Arc {
                    center,
                    start,
                    end,
                    ccw,
                }) => {
                    let (c, mut p0, mut p1) = (w(s.point(center)), w(s.point(start)), w(s.point(end)));
                    let ccw = ccw != flip;
                    if off != 0.0 {
                        // left of a ccw arc is toward its centre
                        let r0 = ((p0[0] - c[0]).powi(2) + (p0[1] - c[1]).powi(2)).sqrt();
                        let r1 = r0 + if ccw { -off } else { off };
                        if r1 <= 0.0 {
                            return Err(format!(
                                "{what}: offset {off} takes an arc of radius {r0} to none"
                            ));
                        }
                        let k = r1 / r0;
                        p0 = [c[0] + (p0[0] - c[0]) * k, c[1] + (p0[1] - c[1]) * k];
                        p1 = [c[0] + (p1[0] - c[0]) * k, c[1] + (p1[1] - c[1]) * k];
                    }
                    // in pieces of at most a quarter turn, as posted
                    let r = ((p0[0] - c[0]).powi(2) + (p0[1] - c[1]).powi(2)).sqrt();
                    let a0 = forge_math::dmath::atan2(p0[1] - c[1], p0[0] - c[0]);
                    let mut a1 = forge_math::dmath::atan2(p1[1] - c[1], p1[0] - c[0]);
                    let tau = std::f64::consts::TAU;
                    if ccw {
                        while a1 <= a0 {
                            a1 += tau;
                        }
                    } else {
                        while a1 >= a0 {
                            a1 -= tau;
                        }
                    }
                    let n = ((a1 - a0).abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
                    let mut mv = vec![];
                    for k in 1..=n {
                        let to = if k == n {
                            p1
                        } else {
                            let (sn, cs) = forge_math::dmath::sin_cos(a0 + (a1 - a0) * k as f64 / n as f64);
                            [c[0] + r * cs, c[1] + r * sn]
                        };
                        mv.push((to, Some((c, !ccw))));
                    }
                    curves.push((p0, mv));
                }
                Some(forge_sketch::Entity::Circle { center, r }) => {
                    let c = w(s.point(center));
                    // run counter-clockwise: its left is toward the centre
                    let r = s.params()[r] - off;
                    if r <= 0.0 {
                        return Err(format!("{what}: offset {off} takes a circle to none"));
                    }
                    let p0 = [c[0] + r, c[1]];
                    let mv = (1..=4)
                        .map(|k| {
                            let to = match k {
                                1 => [c[0], c[1] + r],
                                2 => [c[0] - r, c[1]],
                                3 => [c[0], c[1] - r],
                                _ => p0,
                            };
                            (to, Some((c, false)))
                        })
                        .collect();
                    curves.push((p0, mv));
                    if tip > 0.0 && r > tip {
                        bands.push(ring(c, r, tip));
                    }
                }
                _ => {}
            }
        }
        if curves.is_empty() {
            return Err(format!(
                "{what}: the sketch `{sk}` has no lines, arcs or circles to follow"
            ));
        }
        let top = get_num(a, "from", Some(o[2]))?;
        let depth = get_num(a, "depth", None)?;
        // a NaN depth is refused too
        if depth.is_nan() || depth <= 0.0 {
            return Err(format!("{what}: depth > 0"));
        }
        // `axial_offset:` raises (> 0) or lowers the tip from the depth
        let bottom = o[2] - depth + get_num(a, "axial_offset", Some(0.0))?;
        if bottom >= top {
            return Err(format!("{what}: the bottom {bottom} is not under the top {top}"));
        }
        let mut mv = vec![];
        let mut above = top;
        for z in ops::levels(top, bottom, step) {
            for (p0, ms) in &curves {
                mv.push(PMove::Rapid {
                    to: [p0[0], p0[1], h.retract],
                });
                // the first line's direction and length, for a ramp along it
                let first_line = match ms.first() {
                    Some((q, None)) => {
                        let (dx, dy) = (q[0] - p0[0], q[1] - p0[1]);
                        let len = (dx * dx + dy * dy).sqrt();
                        if len > 0.0 { Some((*q, len)) } else { None }
                    }
                    _ => None,
                };
                match (ramp, first_line) {
                    (Some(deg), Some((q, len))) => {
                        // down to the level above (cleared there, or the top), then back and
                        // forth along the line, descending at the angle, to the level, and back
                        // to the curve's start at the level if the ramp ended at the far end
                        mv.push(PMove::Line {
                            to: [p0[0], p0[1], above],
                            f: feed.1,
                            phase: Phase::Plunge,
                        });
                        let (sa, ca) = forge_math::dmath::sin_cos(deg.to_radians());
                        let run = (above - z) * ca / sa;
                        let n = ((run / len - 1e-9).ceil().max(1.0)) as usize;
                        for k in 1..=n {
                            let zk = if k == n {
                                z
                            } else {
                                above + (z - above) * k as f64 / n as f64
                            };
                            let p = if k % 2 == 1 { q } else { *p0 };
                            mv.push(PMove::Line {
                                to: [p[0], p[1], zk],
                                f: feed.1,
                                phase: Phase::Cut,
                            });
                        }
                        if n % 2 == 1 {
                            mv.push(PMove::Line {
                                to: [p0[0], p0[1], z],
                                f: feed.0,
                                phase: Phase::Cut,
                            });
                        }
                    }
                    _ => mv.push(PMove::Line {
                        to: [p0[0], p0[1], z],
                        f: feed.1,
                        phase: Phase::Plunge,
                    }),
                }
                let mut at = *p0;
                for (to, arc) in ms {
                    mv.push(match arc {
                        None => PMove::Line {
                            to: [to[0], to[1], z],
                            f: feed.0,
                            phase: Phase::Cut,
                        },
                        Some((c, cw)) => PMove::Arc {
                            to: [to[0], to[1], z],
                            c: *c,
                            cw: *cw,
                            f: feed.0,
                        },
                    });
                    at = *to;
                }
                mv.push(PMove::Line {
                    to: [at[0], at[1], h.retract],
                    f: feed.0,
                    phase: Phase::Link,
                });
            }
            above = z;
        }
        let first = curves[0].0;
        mv.push(PMove::Rapid {
            to: [first[0], first[1], h.clear],
        });
        // the bands it clears, united (none is claimed if the union cannot be formed)
        let mut u: Option<Region> = None;
        for b in bands {
            u = match u {
                None => Some(b),
                Some(r) => forge_cam_geom::boolean::boolean(&r, &b, forge_cam_geom::boolean::Op::Union)
                    .ok()
                    .map(|l| forge_cam_geom::boolean::region_of(&l)),
            };
            if u.is_none() {
                break;
            }
        }
        if let Some(u) = u {
            voids.push((Some(u), bottom + lift));
        }
        Ok(vec![OpPath {
            tool: ti,
            start: [first[0], first[1], h.clear],
            moves: mv,
            rpm: 0.0,
            coolant: forge_post::Coolant::Flood,
        }])
    }

    /// Thread milling a hole's recorded thread, the model not carrying it: a declared removal Δ
    /// (cosmetic_thread) per place, the ring between the bore and the thread's major diameter
    /// over the thread's length, and the op certified against a target bounding a superset of
    /// P ∖ Δ: the bore's wall cut back below the band, Δ's outer wall and its floor added, the
    /// part's other faces as they are. The path: down the drilled bore, in to the radius, up a
    /// helix of the pitch (G3 climbing: a right-hand thread), back to the axis, out.
    #[allow(clippy::too_many_arguments)]
    fn thread_mill(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        out: &mut OpOut,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        use forge_cam_geom::check::Surf;
        let (pi, hn) = match a.get("#1") {
            Some(V::Holes(p, n)) if self.holds(su, *p) => (self.setups[su].part, n.clone()),
            Some(V::Holes(..)) => {
                return Err("the hole is milled from the side it opens on: in a setup that holds the part that way up".into());
            }
            _ => return Err("thread_mill(SETUP, PART.HOLE, tool:, length:)".into()),
        };
        let part = &self.parts[pi];
        let hole = part.holes.iter().find(|x| x.name == hn).unwrap().clone();
        let Some((major, pitch)) = hole.thread else {
            return Err(format!(
                "the hole `{hn}` records no thread: give it `thread:` (or a screw with fit: \"tap\")"
            ));
        };
        if hole.dir != [0.0, 0.0, -1.0] {
            return Err(not_down(&hn, hole.dir));
        }
        if t.tool.kind != Kind::ThreadMill {
            return Err(format!("thread_mill: `{}` is not a thread mill", t.tool.id));
        }
        if (t.tool.tp - pitch).abs() > 1e-9 {
            out.fix = Some(format!("a thread mill of pitch {pitch}"));
            return Err(format!(
                "thread_mill: the tool's pitch {} is not the thread's {pitch}",
                t.tool.tp
            ));
        }
        let (rb, rm, rt) = (0.5 * hole.d, 0.5 * major, 0.5 * t.tool.dc);
        if rt >= rb {
            out.fix = Some(format!("a thread mill under {:.3} mm across", hole.d));
            return Err(format!(
                "thread_mill: the tool ({} mm) does not go down the bore ({} mm)",
                t.tool.dc, hole.d
            ));
        }
        let rc = rm - rt;
        let depth = match hole.depth {
            Some(d) => d,
            None => hole.places[0][2] - part.bbox.0[2],
        };
        let len = get_num(a, "length", Some(depth - 2.0 * pitch))?;
        if !(len > 0.0 && len <= depth) {
            return Err(format!(
                "thread_mill: length {len} must be over 0 and at most the hole's depth {depth}"
            ));
        }
        // the target: P ∖ Δ's boundary (a superset's), for every place
        let mut faces: Vec<Face> = vec![];
        let mut extra: Vec<Face> = vec![];
        let mut id = 6000u32;
        let bands: Vec<([f64; 2], f64, f64)> = hole
            .places
            .iter()
            .map(|p| ([p[0], p[1]], p[2] - len, p[2]))
            .collect();
        for f in &part.faces {
            let mut kept = Some(f.clone());
            // a flat face at the band's heights (the mouth's, an inner shoulder's): Δ's disk
            // taken out of it
            if let Some(forge_cam_geom::slab::Prism::Flat { z, region, .. }) = &f.prism
                && bands.iter().any(|(_, zb, zt)| *z >= zb - 1e-9 && *z <= zt + 1e-9)
            {
                let mut rg = trim_region(region);
                for (c, zb, zt) in &bands {
                    if *z < zb - 1e-9 || *z > zt + 1e-9 {
                        continue;
                    }
                    let a0 = p2(c[0] + rm, c[1]);
                    let disk = Region {
                        loops: vec![Loop {
                            segs: vec![Seg::Arc {
                                a: a0,
                                b: a0,
                                c: p2(c[0], c[1]),
                                ccw: true,
                            }],
                        }],
                    };
                    rg = match rg.and_then(|r| {
                        forge_cam_geom::boolean::boolean(&r, &disk, forge_cam_geom::boolean::Op::Diff).ok()
                    }) {
                        Some(l) if !l.is_empty() => Some(forge_cam_geom::boolean::region_of(&l)),
                        _ => None,
                    };
                }
                kept = match rg {
                    Some(r) => Some(forge_cam::stock::flat(&r, *z, f.mu, f.id)),
                    // nothing of the face is left, or it could not be cut: the face as it is
                    // (a superset: never less strict)
                    None => Some(f.clone()),
                };
            }
            if let Surf::Cyl { o, x, y, ax, r } = f.surf
                && ax[0] == 0.0
                && ax[1] == 0.0
                && (r - rb).abs() <= 1e-6
            {
                for (c, zb, zt) in &bands {
                    if (o[0] - c[0]).abs() > 1e-6 || (o[1] - c[1]).abs() > 1e-6 {
                        continue;
                    }
                    // heights: o.z + ax.z q; the wall kept below the band only
                    let (q_at_zb, q_at_zt) = ((zb - o[2]) * ax[2], (zt - o[2]) * ax[2]);
                    let (qlo_band, qhi_band) = (q_at_zb.min(q_at_zt), q_at_zb.max(q_at_zt));
                    let below = if ax[2] > 0.0 {
                        forge_cam_geom::ival::Ival::new(f.q.lo, f.q.hi.min(qlo_band))
                    } else {
                        forge_cam_geom::ival::Ival::new(f.q.lo.max(qhi_band), f.q.hi)
                    };
                    kept = if below.lo < below.hi {
                        Some(Face::new(f.surf, f.p, below, None, f.mu, f.id))
                    } else {
                        None
                    };
                    // Δ's outer wall over the band, the same sector at the major radius
                    let band = forge_cam_geom::ival::Ival::new(qlo_band.max(f.q.lo.min(qlo_band)), qhi_band);
                    extra.push(Face::new(
                        Surf::Cyl { o, x, y, ax, r: rm },
                        f.p,
                        band,
                        None,
                        f.mu,
                        id,
                    ));
                    id += 1;
                }
            }
            if let Some(k) = kept {
                faces.push(k);
            }
        }
        for (c, zb, zt) in &bands {
            // Δ's floor: the ring at the band's bottom
            extra.push(forge_cam::stock::flat(
                &ring(*c, 0.5 * (rb + rm), 0.5 * (rm - rb)),
                *zb,
                1e-9,
                id,
            ));
            id += 1;
            out.removals.push(format!(
                "cosmetic_thread M{major}x{pitch} of `{hn}` at ({:.3}, {:.3}): the ring {rb:.3}..{rm:.3} mm from z {zb:.3} to {zt:.3}",
                c[0], c[1]
            ));
        }
        faces.extend(extra);
        out.target = Some(faces);
        // the path, each place in turn. Method (plain, textbook thread milling): per radial pass
        // a helix of the lead (starts × pitch) per turn about the hole's axis, counter-clockwise
        // (G3: climb inside with M03): up from the band's bottom for a right hand, down from its
        // top for a left; each start the same helix one pitch higher. The radial passes split
        // the depth from the bore to the major radius evenly (the last at the major); spring
        // passes repeat the last. In and out on a line (the default) or a tangent arc.
        let tp = thread_params(a)?;
        let lead_len = pitch * tp.starts as f64;
        let rc0 = rb - rt; // the tool touching the bore
        let mut mv = vec![];
        let first = bands[0].0;
        for (c, zb, zt) in &bands {
            mv.push(PMove::Rapid {
                to: [c[0], c[1], h.retract],
            });
            let turns = ((zt - zb) / lead_len - 1e-9).ceil().max(1.0) as usize;
            let rise = lead_len * turns as f64;
            let n = tp.passes;
            let radii: Vec<f64> = (1..=n)
                .map(|k| {
                    if k == n {
                        rc
                    } else {
                        rc0 + (rc - rc0) * k as f64 / n as f64
                    }
                })
                .chain(std::iter::repeat_n(rc, tp.spring))
                .collect();
            for &r in &radii {
                for st in 0..tp.starts {
                    let off = pitch * st as f64;
                    // right hand: up from zb; left: down from the top of the same helix
                    let (z0, z1) = if tp.left {
                        (zb + off + rise, zb + off)
                    } else {
                        (zb + off, zb + off + rise)
                    };
                    mv.push(PMove::Line {
                        to: [c[0], c[1], z0],
                        f: feed.1,
                        phase: Phase::Plunge,
                    });
                    if tp.arc_lead {
                        // a half circle of r/2 from the axis to the start, tangent to the helix
                        mv.push(PMove::Arc {
                            to: [c[0] + r, c[1], z0],
                            c: [c[0] + 0.5 * r, c[1]],
                            cw: false,
                            f: feed.0,
                        });
                    } else {
                        mv.push(PMove::Line {
                            to: [c[0] + r, c[1], z0],
                            f: feed.0,
                            phase: Phase::Cut,
                        });
                    }
                    for k in 1..=4 * turns {
                        let p = match k % 4 {
                            1 => [c[0], c[1] + r],
                            2 => [c[0] - r, c[1]],
                            3 => [c[0], c[1] - r],
                            _ => [c[0] + r, c[1]],
                        };
                        mv.push(PMove::Arc {
                            to: [p[0], p[1], z0 + (z1 - z0) * k as f64 / (4 * turns) as f64],
                            c: *c,
                            cw: false,
                            f: feed.0,
                        });
                    }
                    if tp.arc_lead {
                        mv.push(PMove::Arc {
                            to: [c[0], c[1], z1],
                            c: [c[0] + 0.5 * r, c[1]],
                            cw: false,
                            f: feed.0,
                        });
                    } else {
                        mv.push(PMove::Line {
                            to: [c[0], c[1], z1],
                            f: feed.0,
                            phase: Phase::Cut,
                        });
                    }
                }
            }
            mv.push(PMove::Line {
                to: [c[0], c[1], h.retract],
                f: feed.0,
                phase: Phase::Link,
            });
            // what it clears: the disk of the major radius, from the band's bottom up
            let a0 = p2(c[0] + rm, c[1]);
            let disk = Region {
                loops: vec![Loop {
                    segs: vec![Seg::Arc {
                        a: a0,
                        b: a0,
                        c: p2(c[0], c[1]),
                        ccw: true,
                    }],
                }],
            };
            voids.push((Some(disk), *zb));
        }
        mv.push(PMove::Rapid {
            to: [first[0], first[1], h.clear],
        });
        Ok(vec![OpPath {
            tool: ti,
            start: [first[0], first[1], h.clear],
            moves: mv,
            rpm: 0.0,
            coolant: forge_post::Coolant::Flood,
        }])
    }

    /// Helical bore (bore.rs): each place of a round hole along −z opened by a helix of `pitch:`
    /// (default: a 3° ramp at the helix's radius; or `ramp:` degrees), with `finish_leave:` for a
    /// finishing helix, `spring:`, `leave:`. Certified against the part, which has the hole.
    #[allow(clippy::too_many_arguments)]
    fn bore(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        ax: f64,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        let (pi, hn) = match a.get("#1") {
            Some(V::Holes(p, n)) if *p == self.setups[su].part => (*p, n.clone()),
            Some(V::Holes(..)) => {
                return Err(
                    "the hole is bored from the side it opens on: in a setup that holds the part that way up"
                        .into(),
                );
            }
            _ => return Err("bore(SETUP, PART.HOLE, tool:, pitch:, finish_leave:, spring:)".into()),
        };
        let hole = self.parts[pi]
            .holes
            .iter()
            .find(|x| x.name == hn)
            .unwrap()
            .clone();
        if hole.dir != [0.0, 0.0, -1.0] {
            return Err(format!("the hole `{hn}` is not along −z: it needs its own setup"));
        }
        let leave = get_num(a, "leave", Some(0.0))?;
        let finish_leave = get_num(a, "finish_leave", Some(0.0))?;
        if !(finish_leave >= 0.0 && finish_leave.is_finite()) {
            return Err("finish_leave: a length, 0 or more".into());
        }
        let spring = get_num(a, "spring", Some(0.0))?;
        if spring.fract() != 0.0 || !(0.0..=20.0).contains(&spring) {
            return Err("spring: a whole number from 0 to 20".into());
        }
        let r = 0.5 * hole.d;
        let rc = r - 0.5 * t.tool.dc - leave;
        // the pitch of a helix at radius rc ramping at `deg` degrees
        let at_ramp = |deg: f64| {
            let (sn, cs) = forge_math::dmath::sin_cos(deg.to_radians());
            std::f64::consts::TAU * rc.max(1e-3) * sn / cs
        };
        let pitch = match (a.get("pitch"), a.get("ramp")) {
            (Some(_), Some(_)) => return Err("give `pitch:` or `ramp:`, not both".into()),
            (Some(_), None) => get_num(a, "pitch", None)?,
            (None, Some(V::Num(x))) if *x > 0.0 && *x <= 30.0 => at_ramp(*x),
            (None, Some(_)) => return Err("ramp: an angle in degrees, over 0 and at most 30".into()),
            (None, None) => at_ramp(3.0),
        };
        if !(pitch > 0.0 && pitch.is_finite()) {
            return Err("pitch: a length over 0".into());
        }
        let mut paths = vec![];
        for p in &hole.places {
            let bottom = match hole.depth {
                Some(dd) => p[2] - dd,
                None => self.parts[pi].bbox.0[2] - 0.5,
            } + ax;
            paths.push(
                forge_cam::bore::helical_bore(
                    [p[0], p[1]],
                    r,
                    p[2],
                    bottom,
                    ops::Cut {
                        tool: t,
                        tidx: ti,
                        h,
                        feed,
                        ramp_feed: None,
                        lead_feed: None,
                    },
                    forge_cam::bore::Bore {
                        pitch,
                        finish_leave,
                        spring: spring as usize,
                        leave,
                    },
                )
                .map_err(|e| match e {
                    ops::OpError::Region(w) => w,
                    ops::OpError::ToolTooBig { why, .. } => why,
                    e => format!("{e:?}"),
                })?,
            );
            // what it clears: the disk the tool sweeps (the hole less `leave`), to the bottom
            let rr = r - leave;
            let a0 = p2(p[0] + rr, p[1]);
            voids.push((
                Some(Region {
                    loops: vec![Loop {
                        segs: vec![Seg::Arc {
                            a: a0,
                            b: a0,
                            c: p2(p[0], p[1]),
                            ccw: true,
                        }],
                    }],
                }),
                bottom,
            ));
        }
        Ok(paths)
    }

    /// Edge break (a declared removal, the master's option (a)): the part's top edges at `at:`
    /// (default its top) broken by a 45° chamfer of `size:` s with a 90° chamfer mill. Δ is the
    /// band of the top within s of its edges, below the 45° surface z' = z − s + d, d the
    /// distance inside the top's outline R. The op is certified against P ∖ Δ: the top eroded
    /// to E = R ⊖ s, the walls cut to z − s, and the chamfer surfaces from each piece of E down
    /// to the same piece moved out by s (planes for lines, cones for arcs; a line's ends at a
    /// sharp corner meet the next line's on the corner's bisector). The path: the tool's axis on
    /// R ⊕ c with c = r_tip + e at depth z − s − e (e = 0.2 mm), where the tool's flank is that
    /// surface; every point of R ⊕ c is c from R, so the flank never goes below it. In this
    /// phase the top must be the part's top, with only vertical walls through the band, and the
    /// pieces of E must meet tangentially wherever an arc meets (a sharp corner between lines is
    /// taken; one with an arc is refused).
    #[allow(clippy::too_many_arguments)]
    fn edge_break(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        out: &mut OpOut,
    ) -> Result<Vec<OpPath>, String> {
        use forge_cam_geom::check::{Surf, TSeg, Trim};
        use forge_cam_geom::ival::Ival;
        let part = &self.parts[self.setups[su].part];
        if !matches!(t.tool.kind, Kind::Chamfer | Kind::Spot)
            || !(t.tool.angle >= 30.0 && t.tool.angle <= 150.0)
        {
            out.fix = Some("a chamfer mill or spot drill of 30° to 150° (chamfer-10-90)".into());
            return Err(format!(
                "edge_break: `{}` is not a chamfer mill or spot drill of 30° to 150°",
                t.tool.id
            ));
        }
        let ztop = part.bbox.1[2];
        let z = get_num(a, "at", Some(ztop))?;
        let s = get_num(a, "size", None)?;
        let (r0, rt) = (0.5 * t.tool.tip_d, 0.5 * t.tool.dc);
        // the chamfer's angle from the wall: half the tool's; its depth s_v for a width s
        let beta = 0.5 * t.tool.angle;
        // 45° exactly for the 90° tool (tan β = 1, as the 45° construction always was)
        let (sin_b, cos_b, tan_b) = if beta == 45.0 {
            (
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                1.0,
            )
        } else {
            let (sn, cs) = forge_math::dmath::sin_cos(beta.to_radians());
            (sn, cs, sn / cs)
        };
        // the tip below the chamfer's bottom (default 0.2), and the axis that far times tan β
        // further out than where the tip's edge would touch
        let tip_off = get_num(a, "tip_offset", Some(0.2))?;
        if !(tip_off >= 0.0 && tip_off.is_finite()) {
            return Err("edge_break: tip_offset: a length, 0 or more".into());
        }
        let c = r0 + tip_off * tan_b;
        if s.is_nan() || s <= 0.0 {
            return Err("edge_break: size must be over 0".into());
        }
        let s_v = s / tan_b;
        if c + s >= rt {
            out.fix = Some(format!("a chamfer mill over {:.3} mm across", 2.0 * (c + s)));
            return Err(format!(
                "edge_break: a {s} mm break needs the flank out to {:.3} mm; the tool's is {rt}",
                c + s
            ));
        }
        if (z - ztop).abs() > 1e-9 {
            return Err(format!(
                "edge_break: at z = {z} is not the part's top ({ztop}); this phase breaks the top's edges only"
            ));
        }
        let zc = z - s_v;
        let mu = 1e-9;
        // the top's outline R: the union of the flat faces at z
        let mut tops: Option<Region> = None;
        let mut faces: Vec<Face> = vec![];
        for f in &part.faces {
            if let Some(forge_cam_geom::slab::Prism::Flat { z: fz, region, .. }) = &f.prism
                && (fz - z).abs() <= 1e-9
            {
                let rg = trim_region(region)
                    .ok_or_else(|| format!("edge_break: the top face {} has an open trim", f.id))?;
                tops = Some(match tops {
                    None => rg,
                    Some(r) => forge_cam_geom::boolean::region_of(
                        &forge_cam_geom::boolean::boolean(&r, &rg, forge_cam_geom::boolean::Op::Union)
                            .map_err(|e| format!("edge_break: the top's faces: {e:?}"))?,
                    ),
                });
                continue;
            }
            if f.bbox[2].hi <= zc + f.mu {
                faces.push(f.clone());
                continue;
            }
            // through the band: a vertical wall, kept below z − s
            match f.surf {
                Surf::Cyl { o, ax, .. } if ax[0] == 0.0 && ax[1] == 0.0 && ax[2].abs() == 1.0 => {
                    // heights o.z + ax.z q
                    let qc = (zc - o[2]) * ax[2];
                    let below = if ax[2] > 0.0 {
                        Ival::new(f.q.lo, f.q.hi.min(qc))
                    } else {
                        Ival::new(f.q.lo.max(qc), f.q.hi)
                    };
                    if below.lo < below.hi {
                        faces.push(Face::new(f.surf, f.p, below, None, f.mu, f.id));
                    }
                }
                Surf::Plane { o, u, v } if u[2] == 0.0 && v[0] == 0.0 && v[1] == 0.0 && v[2].abs() == 1.0 => {
                    // vertical over a rectangle of (p, q) (q the height): kept below z − s
                    let mut ok = true;
                    let (mut p0, mut p1, mut q0, mut q1) =
                        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
                    match &f.trim {
                        Some(tr) if tr.segs.len() == 4 => {
                            for sg in &tr.segs {
                                match *sg {
                                    TSeg::Line { a, b } if a[0] == b[0] || a[1] == b[1] => {
                                        for pt in [a, b] {
                                            p0 = p0.min(pt[0]);
                                            p1 = p1.max(pt[0]);
                                            q0 = q0.min(pt[1]);
                                            q1 = q1.max(pt[1]);
                                        }
                                    }
                                    _ => ok = false,
                                }
                            }
                        }
                        _ => ok = false,
                    }
                    if !ok {
                        return Err(format!(
                            "edge_break: the wall {} through the break's band is not a rectangle; this phase breaks a top over plain walls",
                            f.id
                        ));
                    }
                    // heights o.z + v.z q
                    let qc = (zc - o[2]) * v[2];
                    let (q0, q1) = if v[2] > 0.0 {
                        (q0, q1.min(qc))
                    } else {
                        (q0.max(qc), q1)
                    };
                    if q0 < q1 {
                        let c4 = [[p0, q0], [p1, q0], [p1, q1], [p0, q1]];
                        let segs = (0..4)
                            .map(|k| TSeg::Line {
                                a: c4[k],
                                b: c4[(k + 1) % 4],
                            })
                            .collect();
                        faces.push(Face::new(
                            f.surf,
                            Ival::new(p0 - f.mu, p1 + f.mu),
                            Ival::new(q0 - f.mu, q1 + f.mu),
                            Some(Trim { segs }),
                            f.mu,
                            f.id,
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "edge_break: face {} reaches into the break's band (z {zc:.3} to {z:.3}) and is not a vertical wall; this phase breaks a top over vertical walls",
                        f.id
                    ));
                }
            }
        }
        let rg = tops.ok_or_else(|| format!("edge_break: the part has no flat face at z = {z}"))?;
        // E = R ⊖ s, as many loops as R (no feature narrower than 2s)
        let (el, _) = forge_cam_geom::offset::offset(&rg, s).map_err(|e| format!("edge_break: {e:?}"))?;
        let el: Vec<_> = el
            .into_iter()
            .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b)))
            .collect();
        if el.len() != rg.loops.len() {
            out.fix = Some(format!(
                "a break under half the top's narrowest feature (size < {s})"
            ));
            return Err(format!(
                "edge_break: the top eroded by {s} has {} loops where the top has {}: a feature is narrower than the break",
                el.len(),
                rg.loops.len()
            ));
        }
        let er = forge_cam_geom::boolean::region_of(&el);
        let mut id = 7000u32;
        faces.push(forge_cam::stock::flat(&er, z, mu, id));
        id += 1;
        // the chamfer surfaces, piece by piece of E
        let dir = |sg: &Seg, at_end: bool| -> [f64; 2] {
            match *sg {
                Seg::Line { a, b } => {
                    let (dx, dy) = (b.x - a.x, b.y - a.y);
                    let l = (dx * dx + dy * dy).sqrt();
                    [dx / l, dy / l]
                }
                Seg::Arc { a, b, c, ccw } => {
                    let p = if at_end { b } else { a };
                    let (rx, ry) = (p.x - c.x, p.y - c.y);
                    let l = (rx * rx + ry * ry).sqrt();
                    if ccw { [-ry / l, rx / l] } else { [ry / l, -rx / l] }
                }
            }
        };
        let cross = |u: [f64; 2], v: [f64; 2]| u[0] * v[1] - u[1] * v[0];
        let dotp = |u: [f64; 2], v: [f64; 2]| u[0] * v[0] + u[1] * v[1];
        for l in &er.loops {
            let n = l.segs.len();
            for (i, sg) in l.segs.iter().enumerate() {
                let prev = &l.segs[(i + n - 1) % n];
                let next = &l.segs[(i + 1) % n];
                // a junction: tangent (no miter), or a sharp corner between two lines
                let junction = |p: &Seg, q: &Seg, at: Seg| -> Result<Option<[f64; 2]>, String> {
                    let (u0, u1) = (dir(p, true), dir(q, false));
                    if cross(u0, u1).abs() <= 1e-9 && dotp(u0, u1) > 0.0 {
                        return Ok(None);
                    }
                    let pt = at.start();
                    match (p, q) {
                        (Seg::Line { .. }, Seg::Line { .. }) if cross(u0, u1) > 0.0 => {
                            // outward normals (inside on the left): the corner's bisector
                            let (n0, n1) = ([u0[1], -u0[0]], [u1[1], -u1[0]]);
                            let k = s / (1.0 + dotp(n0, n1));
                            Ok(Some([pt.x + k * (n0[0] + n1[0]), pt.y + k * (n0[1] + n1[1])]))
                        }
                        _ => Err(format!(
                            "edge_break: the top's corner at ({:.3}, {:.3}) is sharp where an arc meets (or turns inward); this phase breaks sharp corners between lines only",
                            pt.x, pt.y
                        )),
                    }
                };
                match *sg {
                    Seg::Line { a: pa, b: pb } => {
                        let u2 = dir(sg, false);
                        let nn = [u2[1], -u2[0]];
                        let len = ((pb.x - pa.x).powi(2) + (pb.y - pa.y).powi(2)).sqrt();
                        // the slope's length from the top's edge to the chamfer's bottom
                        let qb = if beta == 45.0 {
                            s * std::f64::consts::SQRT_2
                        } else {
                            s / sin_b
                        };
                        // bottom ends: moved out by s, or the corner
                        let ca = junction(prev, sg, *sg)?;
                        let cb = junction(sg, next, *next)?;
                        let along = |p: [f64; 2]| (p[0] - pa.x) * u2[0] + (p[1] - pa.y) * u2[1];
                        let pa_b = ca.map(along).unwrap_or(0.0);
                        let pb_b = cb.map(along).unwrap_or(len);
                        let poly = [[0.0, 0.0], [len, 0.0], [pb_b, qb], [pa_b, qb]];
                        let segs: Vec<TSeg> = (0..4)
                            .map(|k| TSeg::Line {
                                a: poly[k],
                                b: poly[(k + 1) % 4],
                            })
                            .collect();
                        // o + p u + q v: u along the piece, v down the slope (β from the wall) outward; the
                        // quadrilateral runs anticlockwise in (p, q) when u × v points out of the
                        // part (up and out)
                        faces.push(Face::new(
                            Surf::Plane {
                                o: [pa.x, pa.y, z],
                                u: [u2[0], u2[1], 0.0],
                                v: [nn[0] * sin_b, nn[1] * sin_b, -cos_b],
                            },
                            Ival::new(pa_b.min(0.0) - mu, pb_b.max(len) + mu),
                            Ival::new(-mu, qb + mu),
                            Some(Trim { segs }),
                            mu,
                            id,
                        ));
                        id += 1;
                    }
                    Seg::Arc {
                        a: pa,
                        b: pb,
                        c: cc,
                        ccw,
                    } => {
                        junction(prev, sg, *sg)?;
                        junction(sg, next, *next)?;
                        let r1 = ((pa.x - cc.x).powi(2) + (pa.y - cc.y).powi(2)).sqrt();
                        let full = pa == pb;
                        // pieces of at most a quarter turn
                        let t0 = forge_math::dmath::atan2(pa.y - cc.y, pa.x - cc.x);
                        let mut t1 = if full {
                            t0 + if ccw {
                                std::f64::consts::TAU
                            } else {
                                -std::f64::consts::TAU
                            }
                        } else {
                            forge_math::dmath::atan2(pb.y - cc.y, pb.x - cc.x)
                        };
                        if !full {
                            if ccw {
                                while t1 <= t0 {
                                    t1 += std::f64::consts::TAU;
                                }
                            } else {
                                while t1 >= t0 {
                                    t1 -= std::f64::consts::TAU;
                                }
                            }
                        }
                        let m = (((t1 - t0).abs() / (0.5 * std::f64::consts::PI)).ceil() as usize).max(1);
                        for k in 0..m {
                            let (u0, u1) = (
                                t0 + (t1 - t0) * k as f64 / m as f64,
                                t0 + (t1 - t0) * (k + 1) as f64 / m as f64,
                            );
                            let um = 0.5 * (u0 + u1);
                            let hh = 0.5 * (u1 - u0).abs();
                            let (sm, cm) = forge_math::dmath::sin_cos(um);
                            // tan of a quarter of the sweep, widened a little (a superset)
                            let th = forge_math::dmath::sin(0.5 * hh) / forge_math::dmath::cos(0.5 * hh)
                                * (1.0 + 1e-12);
                            // ccw (the centre inside): the radius grows going down, tan β per
                            // unit height; cw shrinks
                            let kk = if ccw { tan_b } else { -tan_b };
                            faces.push(Face::new(
                                Surf::Cone {
                                    o: [cc.x, cc.y, z],
                                    x: [cm, sm, 0.0],
                                    y: [-sm, cm, 0.0],
                                    ax: [0.0, 0.0, -1.0],
                                    r0: r1,
                                    k: kk,
                                },
                                Ival::new(-th, th),
                                Ival::new(0.0, s_v),
                                None,
                                mu,
                                id,
                            ));
                            id += 1;
                        }
                    }
                }
            }
        }
        out.target = Some(faces);
        out.removals.push(format!(
            "edge_break {s} x {beta}° of the top at z {z:.3}: the band within {s} of its edges, from z {zc:.3} up, below the {beta}° surface"
        ));
        // the path: the axis on R ⊕ c, the tip at z − s − e
        let (pl, _) = forge_cam_geom::offset::offset(&rg, -c).map_err(|e| format!("edge_break: {e:?}"))?;
        let np = pl
            .iter()
            .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b)))
            .count();
        if np != rg.loops.len() {
            out.fix = Some(format!(
                "a chamfer mill with a smaller tip (its axis runs {c:.3} mm off the edge)"
            ));
            return Err(format!(
                "edge_break: the tool's axis path R ⊕ {c:.3} has {np} loops where the top has {}: a hole is too small for the tool",
                rg.loops.len()
            ));
        }
        let zt = zc - tip_off;
        ops::contour(
            &rg,
            ops::Cut {
                tool: t,
                tidx: ti,
                h,
                feed,
                ramp_feed: None,
                lead_feed: None,
            },
            ops::Span {
                top: zt,
                bottom: zt,
                step: 1.0,
            },
            c - rt,
            true,
        )
        .map(|p| vec![p])
        .map_err(|e| format!("{e:?}"))
    }

    /// The milling ops' feeds set from the engagement each piece meets (forge_cam::engage): the
    /// chip held at the cutting data's feed per tooth, the in-process stock as the ops so far
    /// left it the model. The paths' geometry is unchanged; they are posted and certified after.
    fn feeds_from_engagement(&self, su: usize, kind: &str, out: &OpOut, paths: Vec<OpPath>) -> Vec<OpPath> {
        use forge_cam::engage::{HeightMap, Profile, Sched, schedule};
        if !matches!(kind, "facing" | "pocket" | "clear" | "contour" | "slot") {
            return paths;
        }
        let (Some(fe), Some(ti)) = (out.feeds, out.tool) else {
            return paths;
        };
        let t = &self.tools[ti];
        let Some(prof) = Profile::of(&t.tool) else {
            return paths;
        };
        let s = &self.setups[su];
        let Some(g) = self.lib.group(&self.parts[s.part].group) else {
            return paths;
        };
        let f_max = self.lib.machine(&s.machine).map(|m| m.max_feed).unwrap_or(fe.vf);
        let cell = (t.tool.dc / 30.0).clamp(0.05, 0.25);
        let sc = Sched {
            fz: g.fz_at(t.tool.dc),
            flutes: t.tool.nof.max(1) as f64,
            rpm: fe.rpm,
            // just under the machine's limit: G5 checks the posted F against it soundly
            f_max: 0.999 * f_max,
            f_min: 0.0,
            cap: feeds::THIN_CAP,
            cell,
            // samples of one diameter, fixed by length (fto-adaptive.md D)
            sample: t.tool.dc,
        };
        let mut map = HeightMap::of_stock(&s.ip, cell);
        paths.iter().map(|p| schedule(p, &mut map, &prof, &sc)).collect()
    }

    fn feeds_for(&self, su: usize, t: &ToolUse, ae: f64, ap: f64) -> Option<feeds::Feeds> {
        let g = self.lib.group(&self.parts[self.setups[su].part].group)?;
        let m = self.lib.machine(&self.setups[su].machine).ok()?;
        Some(feeds::feeds(
            g,
            &feeds::Cut {
                d: t.tool.dc,
                flutes: t.tool.nof.max(1),
                ball: t.tool.kind == Kind::Ball,
                carbide: t.tool.material == "carbide",
                ap,
                ae,
            },
            m.rpm,
        ))
    }

    fn op(&mut self, name: &str, kind: &str, a: &BTreeMap<String, V>) -> Result<V, String> {
        let su = match a.get("#0") {
            Some(V::Setup(i)) => *i,
            _ => return Err(format!("{kind}(SETUP, ..)")),
        };
        let resolved;
        let a = if a.values().any(|v| matches!(v, V::Height { .. })) {
            resolved = self.heights_in(su, a)?;
            &resolved
        } else {
            a
        };
        let s = &self.setups[su];
        let part = &self.parts[s.part];
        let mut out = OpOut {
            name: name.into(),
            kind: match kind {
                "facing" => "facing",
                "contour" => "contour",
                "pocket" => "pocket",
                "clear" => "clear",
                "engrave" => "engrave",
                "slot" => "slot",
                "thread_mill" => "thread_mill",
                "edge_break" => "edge_break",
                "bore" => "bore",
                "parallel" => "parallel",
                "waterline" => "waterline",
                "zlevel" => "zlevel",
                "pencil" => "pencil",
                "scallop" => "scallop",
                "rest" => "rest",
                "steep_shallow" => "steep_shallow",
                "spiral" => "spiral",
                "radial" => "radial",
                "ramping" => "ramping",
                "flat" => "flat",
                _ => "drill",
            },
            setup: su,
            tool: None,
            paths: vec![],
            feeds: None,
            checks: vec![],
            pairs: 0,
            cells: 0,
            status: "built",
            refusal: None,
            fix: None,
            moves: 0,
            length_cut: 0.0,
            length_rapid: 0.0,
            time_s: 0.0,
            least: f64::INFINITY,
            plan: vec![],
            // G4 grows the part by the most stock any op leaves, either way
            leave: get_num(a, "leave", Some(0.0))?.max(get_num(a, "leave_axial", Some(0.0))?),
            target: None,
            removals: vec![],
            stock_before: Some(s.ip.clone()),
            voids: vec![],
            deepest: f64::INFINITY,
            finish: None,
            posted: vec![],
        };
        let mut voids: Vec<(Option<Region>, f64)> = vec![];
        let paths: Result<Vec<OpPath>, String> = match kind {
            "drill" => self.plan_holes(su, a, &mut out, &mut voids),
            _ => {
                let ti = match a.get("tool") {
                    Some(V::Tool(i)) => *i,
                    _ => return Err(format!("{kind}: missing `tool:`")),
                };
                out.tool = Some(ti);
                let t = &self.tools[ti];
                // the stock left on floors (Fusion's axial stock to leave): each floor raised
                // by it; `leave` stays the radial stock, on walls
                let ax = get_num(a, "leave_axial", Some(0.0))?;
                if !ax.is_finite() {
                    return Err("leave_axial: a length".into());
                }

                let d = t.tool.dc;
                // the 3-axis ops: `step:` is the step-down (as in 2D), `sample:` the spacing along a
                // pass; `down:` is step's old name, accepted for one train with a warning
                if surf3d(kind) {
                    if a.contains_key("down") && a.contains_key("step") {
                        return Err(format!(
                            "{kind}: `down:` and `step:` both given: `down:` is the old name of `step:` (the step-down); keep `step:` only"
                        ));
                    }
                    if kind == "parallel" && a.contains_key("step") {
                        return Err("parallel: `step:` no longer means the spacing along a pass: write `sample:` for that (`step:` is the step-down, which parallel has none of)".into());
                    }
                    if a.contains_key("down") {
                        self.warnings.push(format!(
                            "{name}: `down:` is now `step:` (the step-down, as in the 2D ops); `down:` is accepted until the next train"
                        ));
                    }
                }
                let step = if surf3d(kind) {
                    step_down(a, 0.5 * d)?
                } else {
                    get_num(a, "step", Some(0.5 * d))?
                };
                // a helical entry at this angle (degrees) where it fits; else plunges
                let ramp = match a.get("ramp") {
                    None => None,
                    Some(V::Num(x)) if *x > 0.0 && *x <= 30.0 => Some(*x),
                    Some(_) => return Err("ramp: an angle in degrees, over 0 and at most 30".into()),
                };
                // the radial engagement the feed is set for (chip thinning credits ae < D/2): what
                // the op cuts at most along its passes, never less (a thinner ae would overload
                // the tool where the path cuts more: slots and a clear's first pass cut full width)
                let fd = match kind {
                    "facing" => self.feeds_for(su, t, 0.7 * d, step.min(t.tool.lcf)),
                    "pocket" | "clear" => self.feeds_for(su, t, get_num(a, "stepover", Some(0.4 * d))?, step),
                    // the whole width
                    "slot" => self.feeds_for(su, t, d, step),
                    // what a contour meets is not known here: no thinning credit
                    "contour" => self.feeds_for(su, t, 0.5 * d, step),
                    // a helix down a closed hole engages up to the tool's whole width
                    "bore" => self.feeds_for(su, t, d, step),
                    _ => self.feeds_for(su, t, 0.1 * d, step),
                };
                let (vf, vp) = match fd {
                    Some(fe) => (fe.vf, 0.3 * fe.vf),
                    None => (300.0, 100.0),
                };
                out.feeds = fd;
                let h = s.heights;
                match kind {
                    "engrave" => self.engrave(su, a, t, ti, h, (vf, vp), step, false, &mut voids),
                    "slot" => self.engrave(su, a, t, ti, h, (vf, vp), step, true, &mut voids),
                    "parallel" | "waterline" | "zlevel" => {
                        self.surfacing(su, kind, a, ti, h, (vf, vp), &mut voids)
                    }
                    "steep_shallow" | "spiral" | "radial" | "ramping" | "flat" | "scallop" | "pencil"
                    | "rest" => self.finish3d(su, kind, a, ti, h, (vf, vp), &mut out),
                    "thread_mill" => self.thread_mill(su, a, t, ti, h, (vf, vp), &mut out, &mut voids),
                    "edge_break" => self.edge_break(su, a, t, ti, h, (vf, vp), &mut out),
                    "bore" => self.bore(su, a, t, ti, h, (vf, vp), ax, &mut voids),
                    "facing" => {
                        let z = get_num(a, "to", Some(part.bbox.1[2]))? + ax;
                        let p = ops::facing(
                            &s.stock,
                            ops::Cut {
                                tool: t,
                                tidx: ti,
                                h,
                                feed: (vf, vp),
                                ramp_feed: None,
                                lead_feed: None,
                            },
                            ops::Span {
                                top: s.stock.max[2],
                                bottom: z,
                                step,
                            },
                        );
                        voids.push((None, z));
                        Ok(vec![p])
                    }
                    "clear" => {
                        // everything between the stock's outline and the part's section, from a
                        // height down to another: a pocket with the part as islands
                        let top = get_num(a, "from", Some(s.stock.max[2]))?;
                        let bottom = get_num(a, "to", None)? + ax;
                        let zmid = 0.5 * (top + bottom);
                        let r = 0.5 * d;
                        let sec = forge_cam::section::section(&part.faces, zmid, 1e-5)
                            .map_err(|e| format!("the part's section at z = {zmid}: {e:?}"))?;
                        let islands = outer_or_inner(&sec, true)?;
                        // the tool may pass wholly outside the stock: a strip between the part
                        // and the stock's face narrower than the tool is still cleared
                        let g = 2.0 * r;
                        let (x0, y0, x1, y1) = (
                            s.stock.min[0] - g,
                            s.stock.min[1] - g,
                            s.stock.max[0] + g,
                            s.stock.max[1] + g,
                        );
                        let (pa, pb, pc, pd) = (p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1));
                        let mut loops = vec![Loop {
                            segs: vec![
                                Seg::Line { a: pa, b: pb },
                                Seg::Line { a: pb, b: pc },
                                Seg::Line { a: pc, b: pd },
                                Seg::Line { a: pd, b: pa },
                            ],
                        }];
                        for l in &islands.loops {
                            loops.push(l.reversed());
                        }
                        let rg = Region { loops };
                        let stepover = get_num(a, "stepover", Some(0.4 * d))?;
                        // the in-process stock's box: passes that would cut nothing in it are
                        // left out
                        let (b0, b1) = (s.ip.min, s.ip.max);
                        let q = [
                            p2(b0[0], b0[1]),
                            p2(b1[0], b0[1]),
                            p2(b1[0], b1[1]),
                            p2(b0[0], b1[1]),
                        ];
                        let sbox = Region {
                            loops: vec![Loop {
                                segs: (0..4)
                                    .map(|i| Seg::Line {
                                        a: q[i],
                                        b: q[(i + 1) % 4],
                                    })
                                    .collect(),
                            }],
                        };
                        let p = ops::pocket_in(
                            &rg,
                            ops::Cut {
                                tool: t,
                                tidx: ti,
                                h,
                                feed: (vf, vp),
                                ramp_feed: None,
                                lead_feed: None,
                            },
                            ops::Span { top, bottom, step },
                            stepover,
                            get_num(a, "leave", Some(0.0))?,
                            ramp,
                            Some(&sbox),
                        )
                        .map_err(|e| format!("{e:?}"))?;
                        if stepover <= r
                            && let Some(o) = opening(&rg, r)
                        {
                            voids.push((Some(o), bottom));
                        }
                        Ok(vec![p])
                    }
                    "contour" => {
                        let top = get_num(a, "from", Some(part.bbox.1[2]))?;
                        let bottom = get_num(a, "to", Some(part.bbox.0[2]))? + ax;
                        let side = get_str(a, "side", Some("outside"))?;
                        let leave = get_num(a, "leave", Some(0.0))?;
                        let passes = contour_passes(a, d)?;
                        let fin = ops::Finish2d {
                            leads: contour_leads(a, d)?,
                            tabs: contour_tabs(a, d, top - bottom)?,
                        };
                        let zmid = 0.5 * (top + bottom);
                        let sec = forge_cam::section::section(&part.faces, zmid, 1e-5)
                            .map_err(|e| format!("the part's section at z = {zmid}: {e:?}"))?;
                        let rg = outer_or_inner(&sec, side == "outside")?;
                        if side == "outside" {
                            // the band the loop sweeps is cleared
                            let r = 0.5 * d;
                            if let Ok((c, _)) = forge_cam_geom::offset::offset(&rg, -(r + leave)) {
                                let cr = forge_cam_geom::boolean::region_of(&c);
                                if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                    let o = forge_cam_geom::offset::offset(&cr, -r).map(|x| x.0.len());
                                    let i = forge_cam_geom::offset::offset(&cr, r).map(|x| x.0.len());
                                    eprintln!("band: c {} loops; outer {o:?} inner {i:?}", c.len());
                                }
                                if let (Ok((outer, _)), Ok((inner, _))) = (
                                    forge_cam_geom::offset::offset(&cr, -r),
                                    forge_cam_geom::offset::offset(&cr, r),
                                ) && let Ok(band) = forge_cam_geom::boolean::boolean(
                                    &forge_cam_geom::boolean::region_of(&outer),
                                    &forge_cam_geom::boolean::region_of(&inner),
                                    forge_cam_geom::boolean::Op::Diff,
                                ) {
                                    // tabs keep a bridge standing: the band counts as cleared only down to their
                                    // top (less than is cut between them: conservative)
                                    let floor = bottom + fin.tabs.as_ref().map_or(0.0, |t| t.height);
                                    voids.push((Some(forge_cam_geom::boolean::region_of(&band)), floor));
                                }
                            }
                            ops::contour_passes(
                                &rg,
                                ops::Cut {
                                    tool: t,
                                    tidx: ti,
                                    h,
                                    feed: (vf, vp),
                                    ramp_feed: None,
                                    lead_feed: None,
                                },
                                ops::Span { top, bottom, step },
                                leave,
                                true,
                                passes,
                                &fin,
                            )
                            .map(|p| vec![p])
                            .map_err(|e| format!("{e:?}"))
                        } else {
                            ops::contour_passes(
                                &rg,
                                ops::Cut {
                                    tool: t,
                                    tidx: ti,
                                    h,
                                    feed: (vf, vp),
                                    ramp_feed: None,
                                    lead_feed: None,
                                },
                                ops::Span { top, bottom, step },
                                leave,
                                false,
                                passes,
                                &fin,
                            )
                            .map(|p| vec![p])
                            .map_err(|e| format!("{e:?}"))
                        }
                    }
                    _ => {
                        // with no hole named: every hole and slot of the part's section between
                        // two heights (its inner loops, each a pocket; islands inside them kept)
                        if a.get("#1").is_none() {
                            let top = get_num(a, "from", None)?;
                            let bottom = get_num(a, "to", None)? + ax;
                            let zmid = 0.5 * (top + bottom);
                            let sec = forge_cam::section::section(&part.faces, zmid, 1e-5)
                                .map_err(|e| format!("the part's section at z = {zmid}: {e:?}"))?;
                            let pockets = pockets_of(&sec)?;
                            let stepover = get_num(a, "stepover", Some(0.4 * d))?;
                            let leave = get_num(a, "leave", Some(0.0))?;
                            let mut paths = vec![];
                            // a hole the tool does not fit (a drilled hole in the section) is left
                            // to its own op: skipped, said, and G4 lists what it leaves
                            let total = pockets.len();
                            let mut skipped: Vec<[f64; 4]> = vec![];
                            // a hole that ends above `to:` (a drilled hole's floor or point) is
                            // left to its own op: the section just above the bottom shows
                            // material where it was (when that section can be made)
                            let below = forge_cam::section::section(&part.faces, bottom + 1e-3, 1e-5).ok();
                            let mut shallow: Vec<[f64; 4]> = vec![];
                            for rg in pockets {
                                // (where no section can be made there, a drill point crossing
                                // that height say, the part's hole features decide)
                                let ends = match &below {
                                    Some(sb) => !reaches(sb, &rg),
                                    None => hole_ends_above(&part.holes, &rg, bottom),
                                };
                                if ends {
                                    shallow.push(region_box(&rg));
                                    continue;
                                }
                                // a hole or slot the tool only just fits (no room 1e-3 beyond its
                                // radius: a drilled hole or a slot of the tool's size) is left as
                                // one it does not fit: its own op cuts it (a plunge or a slot),
                                // and its walls at no gap are the costliest to certify
                                if matches!(
                                    forge_cam_geom::offset::offset(&rg, 0.5 * d + leave + 1e-3),
                                    Ok((l, _)) if l.is_empty()
                                ) {
                                    skipped.push(region_box(&rg));
                                    continue;
                                }
                                let p = ops::pocket(
                                    &rg,
                                    ops::Cut {
                                        tool: t,
                                        tidx: ti,
                                        h,
                                        feed: (vf, vp),
                                        ramp_feed: None,
                                        lead_feed: None,
                                    },
                                    ops::Span { top, bottom, step },
                                    stepover,
                                    leave,
                                    ramp,
                                );
                                match p {
                                    Ok(p) => paths.push(p),
                                    Err(ops::OpError::ToolTooBig { .. }) => {
                                        skipped.push(region_box(&rg));
                                        continue;
                                    }
                                    Err(e) => return Err(format!("{e:?}")),
                                }
                                let o = if stepover <= 0.5 * d {
                                    opening(&rg, 0.5 * d)
                                } else {
                                    None
                                };
                                if let Some(o) = o {
                                    voids.push((Some(o), bottom));
                                }
                            }
                            let boxes = |v: &[[f64; 4]]| {
                                v.iter()
                                    .map(|b| format!("x {:.2}..{:.2} y {:.2}..{:.2}", b[0], b[2], b[1], b[3]))
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            };
                            if !shallow.is_empty() {
                                self.warnings.push(format!(
                                    "{name}: {} of {total} holes in the section end above z {bottom} and are left to their own op (G4 lists them): {}",
                                    shallow.len(),
                                    boxes(&shallow)
                                ));
                            }
                            if paths.is_empty() {
                                return Err(format!(
                                    "pocket: the {d} mm tool fits none of the section's {total} holes between z {top} and {bottom}"
                                ));
                            }
                            if !skipped.is_empty() {
                                self.warnings.push(format!(
                                    "{name}: {} of {total} holes in the section are too small for the {d} mm tool (or fit it within 0.001) and are left uncut (G4 lists them): {}",
                                    skipped.len(),
                                    boxes(&skipped)
                                ));
                            }
                            Ok(paths)
                        } else {
                            let (pi, hn) = match a.get("#1") {
                            Some(V::Holes(p, n)) if self.holds(su, *p) => (self.setups[su].part, n.clone()),
                            Some(V::Holes(..)) => return Err("the hole is drilled from the side it opens on: in a setup that holds the part that way up".into()),
                            _ => return Err("pocket(SETUP, PART.HOLE, tool: ..) or pocket(SETUP, from:, to:, tool: ..)".into()),
                        };
                            let hole = self.parts[pi]
                                .holes
                                .iter()
                                .find(|x| x.name == hn)
                                .unwrap()
                                .clone();
                            if hole.dir != [0.0, 0.0, -1.0] {
                                return Err(not_down(&hn, hole.dir));
                            }
                            let stepover = get_num(a, "stepover", Some(0.4 * d))?;
                            let mut paths = vec![];
                            for p in &hole.places {
                                let r = 0.5 * hole.d;
                                let st = p2(p[0] + r, p[1]);
                                let rg = Region {
                                    loops: vec![Loop {
                                        segs: vec![Seg::Arc {
                                            a: st,
                                            b: st,
                                            c: p2(p[0], p[1]),
                                            ccw: true,
                                        }],
                                    }],
                                };
                                let bottom = match hole.depth {
                                    Some(dd) => p[2] - dd,
                                    None => self.parts[pi].bbox.0[2] - 0.5,
                                } + ax;
                                paths.push(
                                    ops::pocket(
                                        &rg,
                                        ops::Cut {
                                            tool: t,
                                            tidx: ti,
                                            h,
                                            feed: (vf, vp),
                                            ramp_feed: None,
                                            lead_feed: None,
                                        },
                                        ops::Span {
                                            top: p[2],
                                            bottom,
                                            step,
                                        },
                                        stepover,
                                        get_num(a, "leave", Some(0.0))?,
                                        ramp,
                                    )
                                    .map_err(|e| format!("{e:?}"))?,
                                );
                                if stepover <= 0.5 * d
                                    && let Some(o) = opening(&rg, 0.5 * d)
                                {
                                    voids.push((Some(o), bottom));
                                }
                            }
                            Ok(paths)
                        }
                    }
                }
            }
        };
        let paths = match paths {
            Ok(p) => p,
            Err(e) => {
                out.status = "refused";
                out.refusal = Some(e);
                self.ops.push(out);
                return Ok(V::Op(self.ops.len() - 1));
            }
        };
        let paths = self.feeds_from_engagement(su, kind, &out, paths);
        out.paths = paths;
        out.voids = voids.clone();
        for (v, floor) in voids {
            match v {
                None => self.setups[su].ip.face_to(floor),
                Some(r) => self.setups[su].ip.cut(r, floor),
            }
        }
        self.ops.push(out);
        Ok(V::Op(self.ops.len() - 1))
    }

    /// 3-axis surfacing (forge_cam::surf): the drop-cutter over the part's faces; every path is
    /// certified afterwards by the column route. Only z-level roughing records what it cleared
    /// in the in-process stock (a flat bottom over the free region, down to each level); the
    /// finishing ops leave the stock as it was (it then contains the true stock: sound, and a
    /// later op may be refused where the stock it meets is already gone).
    #[allow(clippy::too_many_arguments)]
    fn surfacing(
        &self,
        su: usize,
        kind: &str,
        a: &BTreeMap<String, V>,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        use forge_cam::surf::{self, Surfacer};
        let s = &self.setups[su];
        let part = &self.parts[s.part];
        let t = &self.tools[ti];
        let d = t.tool.dc;
        let body = t
            .shape
            .bodies
            .iter()
            .find(|b| b.part == forge_cam_geom::tool::Part::Cutter)
            .ok_or("the tool has no cutter")?;
        let leave = get_num(a, "leave", Some(0.0))?;
        let floor = get_num(a, "floor", Some(part.bbox.0[2]))?;
        let sf = Surfacer::new(&part.faces, body, leave, s.tol, floor).map_err(|e| format!("{e:?}"))?;
        let area = match a.get("area") {
            None => [part.bbox.0[0], part.bbox.0[1], part.bbox.1[0], part.bbox.1[1]],
            Some(V::Tuple(v)) if v.len() == 4 => {
                let mut r = [0.0; 4];
                for (i, x) in v.iter().enumerate() {
                    match x {
                        V::Num(n) => r[i] = *n,
                        _ => return Err("area: (x0, y0, x1, y1), tool centres".into()),
                    }
                }
                r
            }
            Some(_) => return Err("area: (x0, y0, x1, y1), tool centres".into()),
        };
        let c = ops::Cut {
            tool: t,
            tidx: ti,
            h,
            feed,
            ramp_feed: None,
            lead_feed: None,
        };
        let grid = get_num(a, "grid", Some(0.25 * d))?;
        let e = |x: forge_cam::ops::OpError| format!("{x:?}");
        let p = match kind {
            "parallel" => surf::parallel(
                &sf,
                c,
                area,
                get_num(a, "stepover", Some(0.1 * d))?,
                get_num(a, "angle", Some(0.0))?,
                get_num(a, "sample", Some(0.5))?,
            )
            .map_err(e)?,
            "waterline" => {
                let top = get_num(a, "from", Some(part.bbox.1[2]))?;
                let bottom = get_num(a, "to", Some(part.bbox.0[2]))?;
                let zs = ops::levels(top, bottom, step_down(a, 1.0)?);
                surf::waterline(&sf, c, area, &zs, grid).map_err(e)?
            }
            "zlevel" => {
                let top = get_num(a, "from", Some(s.stock.max[2]))?;
                let bottom = get_num(a, "to", Some(part.bbox.0[2]))?;
                let (p, vs) = surf::zlevel_voids(
                    &sf,
                    c,
                    area,
                    top,
                    bottom,
                    step_down(a, 0.5 * d)?,
                    get_num(a, "stepover", Some(0.4 * d))?,
                    grid,
                )
                .map_err(e)?;
                for (r, z) in vs {
                    voids.push((Some(r), z));
                }
                p
            }
            _ => return Err(format!("{kind}: not a surfacing op")),
        };
        Ok(vec![p])
    }

    /// forge-cam-3d's finishing ops on CAM-3AXIS's drop-cutter: steep_shallow, spiral, radial,
    /// ramping, flat. The passes are draped or sliced on the certified CL surface, ordered and
    /// linked (stay down / lift / retract by a fixed rule), and certified afterwards by the
    /// column route like the surfacing ops. With `cusp:` the stepover and step-down are the
    /// largest the scallop module proves leave at most that cusp on a plane at the op's slopes.
    #[allow(clippy::too_many_arguments)]
    fn finish3d(
        &self,
        su: usize,
        kind: &str,
        a: &BTreeMap<String, V>,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        out: &mut OpOut,
    ) -> Result<Vec<OpPath>, String> {
        use forge_cam::surf::Surfacer;
        use forge_cam_3d::contain::Containment;
        use forge_cam_3d::drape::Drape;
        use forge_cam_3d::scallop::{Contact, cusp, raster_step, step_for, waterline_step};
        use forge_cam_3d::{link, ramp, steep, strategy, waterline};
        use forge_cam_geom::ival::Ival;
        let t_start = forge_math::clock::Instant::now();
        let s = &self.setups[su];
        let part = &self.parts[s.part];
        let t = &self.tools[ti];
        let d = t.tool.dc;
        let body = t
            .shape
            .bodies
            .iter()
            .find(|b| b.part == forge_cam_geom::tool::Part::Cutter)
            .ok_or("the tool has no cutter")?;
        let leave = get_num(a, "leave", Some(0.0))?;
        let floor = get_num(a, "floor", Some(part.bbox.0[2]))?;
        let sf = Surfacer::new(&part.faces, body, leave, s.tol, floor).map_err(|e| format!("{e:?}"))?;
        let dc = forge_cam_3d::surfdrop::SurfDrop::new(&sf).ok_or("leave: not a usable stock to leave")?;
        let area = match a.get("area") {
            None => [part.bbox.0[0], part.bbox.0[1], part.bbox.1[0], part.bbox.1[1]],
            Some(V::Tuple(v)) if v.len() == 4 => {
                let mut r = [0.0; 4];
                for (i, x) in v.iter().enumerate() {
                    match x {
                        V::Num(n) => r[i] = *n,
                        _ => return Err("area: (x0, y0, x1, y1), tool centres".into()),
                    }
                }
                r
            }
            Some(_) => return Err("area: (x0, y0, x1, y1), tool centres".into()),
        };
        let top = get_num(a, "from", Some(part.bbox.1[2]))?;
        let bottom = get_num(a, "to", Some(floor))?;
        let grid = get_num(a, "grid", Some(0.25 * d))?;
        let tol = s.tol;
        let within = Containment {
            boundary: Some(vec![
                [area[0], area[1]],
                [area[2], area[1]],
                [area[2], area[3]],
                [area[0], area[3]],
            ]),
            zmin: bottom,
            zmax: top + 1e-9,
            ..Default::default()
        };
        // sin and cos of an angle in degrees, enclosed (dmath is within a few ulps)
        let sc = |deg: f64| -> (Ival, Ival) {
            let (sn, cs) = forge_math::dmath::sin_cos(deg * (std::f64::consts::PI / 180.0));
            let w = |x: f64| {
                let e = 8.0 * f64::EPSILON * x.abs() + f64::MIN_POSITIVE;
                Ival::new((x - e).max(0.0), (x + e).min(1.0))
            };
            (w(sn), w(cs))
        };
        let cut = t.tool.cutter();
        // the spacing along the surface from the cusp, at the slope that bounds the op's passes
        let cusp_at = |deg: f64| -> Result<Option<(f64, f64, Ival, Ival)>, String> {
            match a.get("cusp") {
                None => Ok(None),
                Some(V::Num(h)) => {
                    let (sn, cs) = sc(deg);
                    let c = Contact::of(&cut, sn, cs).map_err(|e| format!("cusp: {e:?} for this tool"))?;
                    let sp = step_for(c, *h, 2.0 * d).map_err(|e| format!("cusp: {e:?}"))?;
                    Ok(Some((*h, cusp(c, Ival::pt(sp)).hi, sn, cs)))
                }
                Some(_) => Err("cusp: a height in mm".into()),
            }
        };
        let dr = Drape {
            step: get_num(a, "sample", Some(0.5 * d))?,
            slack: 0.5 * tol,
            leave: tol,
            zmin: floor,
        };
        let mut fin = Finish {
            cusp: None,
            cusp_bound: None,
            stepover: 0.0,
            stepdown: 0.0,
            links: [0; 3],
        };
        let e = |x: String| format!("{kind}: {x}");
        let passes = match kind {
            "steep_shallow" => {
                let th = get_num(a, "threshold", Some(45.0))?;
                if !(th > 0.0 && th < 90.0) {
                    return Err(
                        "threshold: the slope between the strategies, over 0 and under 90 degrees".into(),
                    );
                }
                let (sn, cs) = sc(th);
                let (dz, so) = match cusp_at(th)? {
                    Some((h0, hb, sn, cs)) => {
                        fin.cusp = Some(h0);
                        fin.cusp_bound = Some(hb);
                        let sp = step_for(
                            Contact::of(&cut, sn, cs).map_err(|x| format!("{x:?}"))?,
                            h0,
                            2.0 * d,
                        )
                        .map_err(|x| format!("{x:?}"))?;
                        (waterline_step(sp, sn), raster_step(sp, cs))
                    }
                    None => (step_down(a, 0.1 * d)?, get_num(a, "stepover", Some(0.1 * d))?),
                };
                fin.stepover = so;
                fin.stepdown = dz;
                let p = steep::Params {
                    sin0: sn.mid(),
                    cos0: cs.mid(),
                    overlap: get_num(a, "overlap", Some(2.0 * so))?,
                    dz,
                    stepover: so,
                    angle_deg: get_num(a, "angle", Some(0.0))?,
                    top,
                    bottom,
                    grid,
                    tol,
                    area: Some([[area[0], area[1]], [area[2], area[3]]]),
                };
                let (st, sh) = steep::steep_and_shallow(&dc, p).map_err(|x| e(format!("{x:?}")))?;
                vec![within.clip(st, None), within.clip(sh, None)]
            }
            "spiral" | "radial" => {
                let so = match cusp_at(0.0)? {
                    Some((h0, hb, ..)) => {
                        fin.cusp = Some(h0);
                        fin.cusp_bound = Some(hb);
                        step_for(
                            Contact::of(&cut, Ival::pt(0.0), Ival::pt(1.0)).map_err(|x| format!("{x:?}"))?,
                            h0,
                            2.0 * d,
                        )
                        .map_err(|x| format!("{x:?}"))?
                        .min(0.9 * d)
                    }
                    None => get_num(a, "stepover", Some(0.1 * d))?,
                };
                fin.stepover = so;
                let cx = 0.5 * (area[0] + area[2]);
                let cy = 0.5 * (area[1] + area[3]);
                let centre = match a.get("center") {
                    None => [cx, cy],
                    Some(V::Tuple(v)) if v.len() == 2 => match (&v[0], &v[1]) {
                        (V::Num(x), V::Num(y)) => [*x, *y],
                        _ => return Err("center: (x, y)".into()),
                    },
                    Some(_) => return Err("center: (x, y)".into()),
                };
                let reach = ((area[2] - area[0]).powi(2) + (area[3] - area[1]).powi(2)).sqrt();
                let r0 = get_num(a, "r0", Some(0.0))?;
                let r1 = get_num(a, "r1", Some(reach))?;
                let ps = if kind == "spiral" {
                    let inward = match get_str(a, "direction", Some("out"))?.as_str() {
                        "out" => false,
                        "in" => true,
                        _ => return Err("direction: \"in\" or \"out\"".into()),
                    };
                    strategy::spiral(&dc, centre, r0, r1, so, inward, dr, &within)
                } else {
                    strategy::radial(&dc, centre, r0.max(0.5 * so), r1, so, dr, &within)
                };
                vec![ps.map_err(|x| e(format!("{x:?}")))?]
            }
            "ramping" => {
                let slope = get_num(a, "slope_min", Some(30.0))?;
                let dz = match cusp_at(slope)? {
                    Some((h0, hb, sn, cs)) => {
                        fin.cusp = Some(h0);
                        fin.cusp_bound = Some(hb);
                        let sp = step_for(
                            Contact::of(&cut, sn, cs).map_err(|x| format!("{x:?}"))?,
                            h0,
                            2.0 * d,
                        )
                        .map_err(|x| format!("{x:?}"))?;
                        waterline_step(sp, sn)
                    }
                    None => step_down(a, 0.1 * d)?,
                };
                fin.stepdown = dz;
                let mut lv = vec![
                    waterline::level(
                        &dc,
                        top,
                        waterline::Params {
                            grid,
                            tol_xy: 0.5 * tol,
                        },
                    )
                    .map_err(|x| e(format!("{x:?}")))?,
                ];
                for z in waterline::levels(top, bottom, dz) {
                    lv.push(
                        waterline::level(
                            &dc,
                            z,
                            waterline::Params {
                                grid,
                                tol_xy: 0.5 * tol,
                            },
                        )
                        .map_err(|x| e(format!("{x:?}")))?,
                    );
                }
                // the top level is only the first ramp's partner, not cut itself
                let rp = ramp::ramps(&dc, &lv, dr).map_err(|x| e(format!("{x:?}")))?;
                // ramps() drapes the top level's loops first, as they are: that many passes
                let skip = ramp::ramps(&dc, &lv[..1], dr)
                    .map_err(|x| e(format!("{x:?}")))?
                    .len();
                vec![within.clip(rp.into_iter().skip(skip).collect(), None)]
            }
            "scallop" => {
                // constant stepover on the surface (geodesic parallels), from the area's edge or
                // from walls steeper than `walls` degrees
                let so = match cusp_at(0.0)? {
                    Some((h0, hb, ..)) => {
                        fin.cusp = Some(h0);
                        fin.cusp_bound = Some(hb);
                        step_for(
                            Contact::of(&cut, Ival::pt(0.0), Ival::pt(1.0)).map_err(|x| format!("{x:?}"))?,
                            h0,
                            2.0 * d,
                        )
                        .map_err(|x| format!("{x:?}"))?
                        .min(0.9 * d)
                    }
                    None => get_num(a, "stepover", Some(0.1 * d))?,
                };
                fin.stepover = so;
                let tan_wall = match a.get("walls") {
                    None => f64::INFINITY,
                    Some(V::Num(w)) if *w > 0.0 && *w < 90.0 => {
                        let (sn, cs) = sc(*w);
                        sn.lo / cs.hi
                    }
                    Some(_) => return Err("walls: the slope (degrees, under 90) the passes run from".into()),
                };
                let gp = forge_cam_3d::geodesic::Params {
                    grid,
                    stepover: so,
                    tan_wall,
                };
                vec![forge_cam_3d::geodesic::scallop(&dc, gp, dr, &within).map_err(|x| e(format!("{x:?}")))?]
            }
            "pencil" => {
                // the double-contact creases: a corner whose CL second difference is at least
                // `kink` (1/mm; default 1/D: corners up to about D + R)
                let pp = forge_cam_3d::pencil::Params {
                    grid: get_num(a, "grid", Some(0.1 * d))?,
                    kink: get_num(a, "kink", Some(1.0 / d))?,
                    min_len: get_num(a, "min_len", Some(0.5 * d))?,
                };
                vec![forge_cam_3d::pencil::pencil(&dc, pp, dr, &within).map_err(|x| e(format!("{x:?}")))?]
            }
            "rest" => {
                // where the earlier tool (`after:`) left more than `min` (mm) that this one reaches:
                // the two cutters' lower envelopes on the grid, parallel passes over that region
                let bj = match a.get("after") {
                    Some(V::Tool(j)) => *j,
                    _ => return Err("rest(SETUP, tool:, after: EARLIER_TOOL, ..)".into()),
                };
                let bb = self.tools[bj]
                    .shape
                    .bodies
                    .iter()
                    .find(|b| b.part == forge_cam_geom::tool::Part::Cutter)
                    .ok_or("the earlier tool has no cutter")?;
                let before = Surfacer::new(
                    &part.faces,
                    bb,
                    get_num(a, "after_leave", Some(0.0))?,
                    s.tol,
                    floor,
                )
                .map_err(|x| format!("{x:?}"))?;
                let bdc = forge_cam_3d::surfdrop::SurfDrop::new(&before)
                    .ok_or("after_leave: not a usable stock to leave")?;
                let so = match cusp_at(0.0)? {
                    Some((h0, hb, ..)) => {
                        fin.cusp = Some(h0);
                        fin.cusp_bound = Some(hb);
                        step_for(
                            Contact::of(&cut, Ival::pt(0.0), Ival::pt(1.0)).map_err(|x| format!("{x:?}"))?,
                            h0,
                            2.0 * d,
                        )
                        .map_err(|x| format!("{x:?}"))?
                        .min(0.9 * d)
                    }
                    None => get_num(a, "stepover", Some(0.1 * d))?,
                };
                fin.stepover = so;
                let m = forge_cam_3d::rest::region(
                    &dc,
                    &sf.check,
                    &bdc,
                    &before.check,
                    grid,
                    get_num(a, "min", Some(0.05))?,
                );
                if !m.on.iter().any(|x| *x) {
                    return Err(
                        "rest: nothing left by the earlier tool that this one reaches (above `min`)".into(),
                    );
                }
                let rp = forge_cam_3d::rest::rest(
                    &dc,
                    &m,
                    get_num(a, "margin", Some(0.5 * d))?,
                    get_num(a, "angle", Some(0.0))?,
                    so,
                    dr,
                    &within,
                )
                .map_err(|x| e(format!("{x:?}")))?;
                vec![rp]
            }
            _ => {
                // flat: the flat heights within the band, raster passes at each
                let so = match cusp_at(0.0)? {
                    Some((h0, hb, ..)) => {
                        fin.cusp = Some(h0);
                        fin.cusp_bound = Some(hb);
                        step_for(
                            Contact::of(&cut, Ival::pt(0.0), Ival::pt(1.0)).map_err(|x| format!("{x:?}"))?,
                            h0,
                            2.0 * d,
                        )
                        .map_err(|x| format!("{x:?}"))?
                        .min(0.9 * d)
                    }
                    None => get_num(a, "stepover", Some(0.5 * d))?,
                };
                fin.stepover = so;
                let sm = steep::slope_map(&dc, grid);
                if std::env::var("FORGE_CAM3D_TIME").is_ok() {
                    eprintln!(
                        "flat: slope map {} nodes at {:.1} s, faces {}",
                        sm.tan.len(),
                        t_start.elapsed().as_secs_f64(),
                        part.faces.len()
                    );
                }
                let levels: Vec<f64> = strategy::flat_levels(&dc, &sm, 1e-3, tol, 4)
                    .into_iter()
                    .filter(|z| *z >= bottom - 1e-9 && *z <= top + 1e-9)
                    .collect();
                out.plan.push(format!("flat heights: {levels:?}"));
                if std::env::var("FORGE_CAM3D_TIME").is_ok() {
                    eprintln!("flat: levels at {:.1} s", t_start.elapsed().as_secs_f64());
                }
                vec![strategy::flat(
                    &dc,
                    &levels,
                    get_num(a, "angle", Some(0.0))?,
                    so,
                    0.25 * so.max(grid),
                    tol,
                    &within,
                )]
            }
        };
        let groups: Vec<Vec<forge_cam_3d::drape::Pass>> = passes;
        if std::env::var("FORGE_CAM3D_TIME").is_ok() {
            eprintln!(
                "{kind}: passes generated in {:.1} s",
                t_start.elapsed().as_secs_f64()
            );
        }
        let n: usize = groups.iter().map(|g| g.len()).sum();
        if n == 0 {
            return Err(format!("{kind}: no passes in the area and heights given"));
        }
        let start = [0.5 * (area[0] + area[2]), 0.5 * (area[1] + area[3]), h.clear];
        let ordered = link::order(groups, start, kind != "ramping");
        let stay = get_num(a, "stay", Some(2.0 * fin.stepover.max(fin.stepdown).max(0.1 * d)))?;
        let lift = get_num(a, "lift", Some(0.5))?;
        let em = forge_cam_3d::path::Emit {
            tool: ti,
            h,
            feed,
            rpm: 0.0,
            coolant: forge_post::Coolant::Flood,
        };
        let (p, c) = link::path(
            &dc,
            &ordered,
            link::Rules {
                max_stay: stay,
                lift,
                slack: 0.25 * tol,
            },
            em,
        );
        fin.links = [c.stay, c.lift, c.retract];
        if std::env::var("FORGE_CAM3D_TIME").is_ok() {
            eprintln!("{kind}: linked at {:.1} s", t_start.elapsed().as_secs_f64());
        }
        if let Some(h0) = fin.cusp {
            out.plan.push(format!(
                "cusp {h0} mm: stepover {:.4} mm, step-down {:.4} mm (proven cusp on a plane at the op's slopes ≤ {:.5} mm)",
                fin.stepover,
                fin.stepdown,
                fin.cusp_bound.unwrap_or(f64::NAN)
            ));
        }
        out.plan.push(format!(
            "links: {} stayed down, {} lifted, {} retracted",
            c.stay, c.lift, c.retract
        ));
        out.finish = Some(fin);
        Ok(vec![p])
    }

    /// The hole plan from the feature: spot, then drill (peck past 3 D), then tap if threaded.
    fn plan_holes(
        &mut self,
        su: usize,
        a: &BTreeMap<String, V>,
        out: &mut OpOut,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        let (pi, hn) = match a.get("#1") {
            Some(V::Holes(p, n)) if self.holds(su, *p) => (self.setups[su].part, n.clone()),
            Some(V::Holes(..)) => {
                return Err("the hole is drilled from the side it opens on: in a setup that holds the part that way up".into());
            }
            _ => return Err("drill(SETUP, PART.HOLE, plan: \"auto\")".into()),
        };
        let hole = self.parts[pi]
            .holes
            .iter()
            .find(|x| x.name == hn)
            .unwrap()
            .clone();
        if hole.dir != [0.0, 0.0, -1.0] {
            return Err(not_down(&hn, hole.dir));
        }
        let through = hole.depth.is_none();
        let depth = match hole.depth {
            Some(d) => d,
            None => hole.places[0][2] - self.parts[pi].bbox.0[2] + 1.0,
        };
        // the drill: the tap drill for a thread, else the hole's own diameter
        let drill_d = match hole.thread {
            Some((major, pitch)) => major - pitch,
            None => hole.d,
        };
        let find = |lib: &Library, k: Kind, d: f64| {
            lib.tools
                .iter()
                .find(|t| t.kind == k && (t.dc - d).abs() < 1e-9)
                .cloned()
        };
        let drill = find(&self.lib, Kind::Drill, (drill_d * 100.0).round() / 100.0)
            .or_else(|| find(&self.lib, Kind::Drill, drill_d));
        let Some(drill) = drill else {
            out.fix = Some(format!(
                "add a drill of {drill_d} mm to the project's cam/*.tools.toml"
            ));
            return Err(format!(
                "NeedTool: no {drill_d} mm drill in the library for `{hn}`"
            ));
        };
        let spot = self
            .lib
            .tools
            .iter()
            .find(|t| t.kind == Kind::Spot && t.dc >= drill_d)
            .cloned();
        // plan "auto": spot, drill, tap a thread; "drill": spot and drill, the thread left to a
        // thread mill
        let plan_kind = get_str(a, "plan", Some("auto"))?;
        if plan_kind != "auto" && plan_kind != "drill" {
            return Err("plan: \"auto\" (spot, drill, tap a thread) or \"drill\" (no tap)".into());
        }
        let tap = match hole.thread {
            Some(_) if plan_kind == "drill" => None,
            Some((major, pitch)) => {
                let t = self
                    .lib
                    .tools
                    .iter()
                    .find(|t| {
                        t.kind == Kind::Tap && (t.dc - major).abs() < 1e-9 && (t.tp - pitch).abs() < 1e-9
                    })
                    .cloned();
                if t.is_none() {
                    out.fix = Some(format!("add a tap M{major}x{pitch}"));
                    return Err(format!("NeedTool: no tap {major} x {pitch} for `{hn}`"));
                }
                t
            }
            None => None,
        };
        let s = &self.setups[su];
        let h = s.heights;
        let mut paths = vec![];
        let mut plan = vec![];
        let base_t = 10 + self.tools.len() as u32;
        let add = |this: &mut Cam, t: forge_tooling::Tool, n: u32| -> Result<usize, String> {
            let holder = this.lib.holder("bt40-er32").ok().cloned();
            let stick = t.lcf + 10.0;
            let shape = t.shape(stick, holder.as_ref()).map_err(|e| format!("{e:?}"))?;
            this.tools.push(ToolUse {
                t: n,
                tool: t,
                holder: holder.map(|h| h.id),
                stickout: stick,
                shape,
            });
            Ok(this.tools.len() - 1)
        };
        if let Some(sp) = spot {
            let ti = add(self, sp, base_t)?;
            let t = &self.tools[ti];
            let fe = self.feeds_for(su, t, t.tool.dc, 1.0);
            let f = fe.map(|x| 0.3 * x.vf).unwrap_or(100.0);
            // a 90° spot to just under the drill's radius: the chamfer ring the drill centres in
            let (sn, cs) = forge_math::dmath::sin_cos((0.5 * t.tool.angle).to_radians());
            let dspot = 0.45 * drill_d / (sn / cs).max(1e-9);
            let mut p = ops::holes(
                &hole.places,
                CycleKind::Drill,
                ops::Cut {
                    tool: t,
                    tidx: ti,
                    h,
                    feed: (f, f),
                    ramp_feed: None,
                    lead_feed: None,
                },
                0.0,
                0.0,
            );
            for m in &mut p.moves {
                if let PMove::Cycle { z, r, .. } = m {
                    *z = *r - 2.0 - dspot;
                }
            }
            // the speed the feed was set at (unset, the post's fallback S3000 ran it)
            p.rpm = fe.map(|x| x.rpm).unwrap_or(0.0);
            plan.push(format!("spot {} to {:.2} mm", t.tool.id, dspot));
            paths.push(p);
        }
        let ti = add(self, drill, base_t + 1)?;
        // a blind hole with a flat floor (no drill point modelled: a counterbored hole's, say):
        // the drill's point stops on the floor, its shoulder the point's length above it; the
        // floor's outer ring is left (said, and G4 lists it)
        let (dc, ang) = (self.tools[ti].tool.dc, self.tools[ti].tool.angle);
        let tip_len = if self.tools[ti].tool.kind == Kind::Drill && ang > 0.0 && ang < 180.0 {
            0.5 * dc / tan_rad((0.5 * ang).to_radians())
        } else {
            0.0
        };
        // a through hole: the drill's shoulder 1 mm past the part's bottom, less where the
        // support under the stock (no workholding declared) would meet its point; the point
        // keeps 0.1 above it, and at least 0.05 of break-through is needed
        let depth = if through && self.setups[su].fixture.is_none() {
            let bottom = self.parts[pi].bbox.0[2];
            let avail = bottom - self.setups[su].stock.min[2];
            let room = avail - tip_len - 0.1;
            if room < 0.05 {
                let need = tip_len + 0.15;
                out.fix = Some(format!(
                    "give the stock at least {need:.2} mm below the part (box(.., below: {need:.2})), or declare workholding that leaves the hole over a gap"
                ));
                return Err(format!(
                    "`{hn}` goes through: the {dc} mm drill's point ({tip_len:.2}) and its break-through need {need:.2} mm of stock under the part, which has {avail:.2} on the support"
                ));
            }
            depth - 1.0 + room.min(1.0)
        } else {
            depth
        };
        let flat = !through && hole.point.is_none() && tip_len > 0.0;
        let depth = if flat {
            if depth - tip_len <= 0.0 {
                return Err(format!(
                    "`{hn}` has a flat floor {depth} deep, shallower than the {dc} mm drill's point ({tip_len:.2}): an end mill cuts it (pocket(SETUP, PART.{hn}, ..))"
                ));
            }
            self.warnings.push(format!(
                "{}: `{hn}` has a flat floor: the drill stops with its point on it, {tip_len:.2} short at the wall; the floor's ring is left to an end mill (G4 lists it)",
                out.name
            ));
            depth - tip_len
        } else {
            depth
        };
        let t = &self.tools[ti];
        let peck = depth > 3.0 * t.tool.dc;
        let fe = self.feeds_for(su, t, t.tool.dc, t.tool.dc);
        let f = fe.map(|x| 0.1 * x.vf.min(2000.0)).unwrap_or(100.0);
        let kind = if peck { CycleKind::Peck } else { CycleKind::Drill };
        let mut dp = ops::holes(
            &hole.places,
            kind,
            ops::Cut {
                tool: t,
                tidx: ti,
                h,
                feed: (f, f),
                ramp_feed: None,
                lead_feed: None,
            },
            depth,
            if peck { t.tool.dc } else { 0.0 },
        );
        dp.rpm = fe.map(|x| x.rpm).unwrap_or(0.0);
        paths.push(dp);
        // each hole's circle is cleared down to where the full diameter reaches
        for p in &hole.places {
            let r = 0.5 * t.tool.dc;
            let st = p2(p[0] + r, p[1]);
            voids.push((
                Some(Region {
                    loops: vec![Loop {
                        segs: vec![Seg::Arc {
                            a: st,
                            b: st,
                            c: p2(p[0], p[1]),
                            ccw: true,
                        }],
                    }],
                }),
                p[2] - depth,
            ));
        }
        plan.push(format!(
            "{} {} {}{}",
            if peck { "peck" } else { "drill" },
            t.tool.id,
            if through { "through" } else { "to depth" },
            if peck {
                format!(", q {}", t.tool.dc)
            } else {
                String::new()
            }
        ));
        out.tool = Some(ti);
        // the counterbore, after the drill: a counterbore tool of its diameter plunged to its
        // depth, else the largest end mill under its diameter pocketing its circle (a helical
        // entry, which the drilled hole leaves in air)
        if let Some((cd, cdep)) = hole.counterbore {
            let circle = |p: &[f64; 3], r: f64| {
                let st = p2(p[0] + r, p[1]);
                Region {
                    loops: vec![Loop {
                        segs: vec![Seg::Arc {
                            a: st,
                            b: st,
                            c: p2(p[0], p[1]),
                            ccw: true,
                        }],
                    }],
                }
            };
            let cbt = self
                .lib
                .tools
                .iter()
                .find(|t| t.kind == Kind::Counterbore && (t.dc - cd).abs() < 1e-9 && t.lcf >= cdep)
                .cloned();
            let em = self
                .lib
                .tools
                .iter()
                .filter(|t| t.kind == Kind::Flat && t.dc < cd - 1e-3 && t.dc >= 0.3 * cd && t.lcf >= cdep)
                .max_by(|a, b| a.dc.total_cmp(&b.dc))
                .cloned();
            if let Some(cb) = cbt {
                let ti = add(self, cb, base_t + 3)?;
                let t = &self.tools[ti];
                let f = self
                    .feeds_for(su, t, t.tool.dc, 1.0)
                    .map(|x| 0.3 * x.vf)
                    .unwrap_or(100.0);
                paths.push(ops::holes(
                    &hole.places,
                    CycleKind::Drill,
                    ops::Cut {
                        tool: t,
                        tidx: ti,
                        h,
                        feed: (f, f),
                        ramp_feed: None,
                        lead_feed: None,
                    },
                    cdep,
                    0.0,
                ));
                for p in &hole.places {
                    voids.push((Some(circle(p, 0.5 * cd)), p[2] - cdep));
                }
                plan.push(format!("counterbore {} to {cdep}", self.tools[ti].tool.id));
            } else if let Some(em) = em {
                let ti = add(self, em, base_t + 3)?;
                let t = &self.tools[ti];
                let dc = t.tool.dc;
                let (stepover, step) = (0.4 * dc, (0.5 * dc).min(cdep));
                let (vf, vp) = match self.feeds_for(su, t, stepover, step) {
                    Some(fe) => (fe.vf, 0.3 * fe.vf),
                    None => (300.0, 100.0),
                };
                for p in &hole.places {
                    let rg = circle(p, 0.5 * cd);
                    let op = ops::pocket(
                        &rg,
                        ops::Cut {
                            tool: t,
                            tidx: ti,
                            h,
                            feed: (vf, vp),
                            ramp_feed: None,
                            lead_feed: None,
                        },
                        ops::Span {
                            top: p[2],
                            bottom: p[2] - cdep,
                            step,
                        },
                        stepover,
                        0.0,
                        Some(3.0),
                    )
                    .map_err(|e| format!("the counterbore of `{hn}`: {e:?}"))?;
                    paths.push(op);
                    if let Some(o) = opening(&rg, 0.5 * dc) {
                        voids.push((Some(o), p[2] - cdep));
                    }
                }
                plan.push(format!(
                    "counterbore by {} (pocketed, helical entry) to {cdep}",
                    self.tools[ti].tool.id
                ));
            } else {
                out.fix = Some(format!(
                    "add a counterbore of {cd} mm, or an end mill under {cd} mm with {cdep} mm of flute"
                ));
                return Err(format!(
                    "NeedTool: no counterbore of {cd} mm nor end mill to pocket it for `{hn}`"
                ));
            }
        }
        // the countersink, after the drill: a countersink, chamfer or spot tool of its angle,
        // plunged until its cone is the countersink's diameter at the mouth
        if let Some((csd, ang)) = hole.countersink {
            let ct = self
                .lib
                .tools
                .iter()
                .filter(|t| {
                    matches!(t.kind, Kind::Countersink | Kind::Chamfer | Kind::Spot)
                        && (t.angle - ang).abs() < 1e-6
                        && t.dc >= csd
                        && t.tip_d < hole.d
                })
                .min_by(|a, b| a.dc.total_cmp(&b.dc))
                .cloned();
            let Some(ct) = ct else {
                out.fix = Some(format!("add a {ang} deg countersink of at least {csd} mm"));
                return Err(format!(
                    "NeedTool: no {ang} deg countersink of at least {csd} mm for `{hn}`"
                ));
            };
            let ti = add(self, ct, base_t + 4)?;
            let t = &self.tools[ti];
            let depth = 0.5 * (csd - t.tool.tip_d) / tan_rad((0.5 * ang).to_radians());
            let f = self
                .feeds_for(su, t, t.tool.dc, 1.0)
                .map(|x| 0.3 * x.vf)
                .unwrap_or(100.0);
            paths.push(ops::holes(
                &hole.places,
                CycleKind::Drill,
                ops::Cut {
                    tool: t,
                    tidx: ti,
                    h,
                    feed: (f, f),
                    ramp_feed: None,
                    lead_feed: None,
                },
                depth,
                0.0,
            ));
            plan.push(format!("countersink {} to {:.2}", self.tools[ti].tool.id, depth));
        }
        if let Some(tp) = tap {
            let ti = add(self, tp, base_t + 2)?;
            let t = &self.tools[ti];
            let rpm = 300.0;
            let f = t.tool.tp * rpm;
            let mut p = ops::holes(
                &hole.places,
                CycleKind::Tap,
                ops::Cut {
                    tool: t,
                    tidx: ti,
                    h,
                    feed: (f, f),
                    ramp_feed: None,
                    lead_feed: None,
                },
                depth.min(hole.depth.unwrap_or(depth)),
                0.0,
            );
            p.rpm = rpm;
            paths.push(p);
            plan.push(format!("tap {} (pitch {}) at {rpm} rpm", t.tool.id, t.tool.tp));
        }
        out.plan = plan;
        // the plan's paths are posted one after the other, each with its tool
        Ok(paths)
    }
}

/// G4's tool-radius reason: the leftover loops `ul` split into what a disk of radius r inside
/// the free region F (the stock's box grown by 2r + 1, less the part's section `sec`) can meet,
/// and the rest, which none can. Computed as O = (F ⊖ B_{r−m}) ⊕ B_{r+m}, m = 1e-6 mm, which
/// contains the opening (F ⊖ B_r) ⊕ B_r: the offsets round their vertices to a 2⁻³² mm grid
/// (offset::GRID), moving the loops by far less than m, so a centre at depth ≥ r is inside the
/// first offset, and its disk inside the second. The rest, L ∖ O, then meets no disk of radius r
/// in F. None (no claim) when an offset fails, or the centres' region has a one-dimensional
/// part (a there-and-back piece: a slot of width exactly 2(r − m)), which a region cannot hold.
fn tool_radius_split(
    ip: &forge_cam::stock::InProcess,
    sec: &Region,
    r: f64,
    ul: &[forge_cam_geom::offset::OLoop],
) -> Option<(
    Vec<forge_cam_geom::offset::OLoop>,
    Vec<forge_cam_geom::offset::OLoop>,
)> {
    use forge_cam_geom::boolean::{Op, boolean, region_of};
    use forge_cam_geom::offset::offset;
    const M: f64 = 1e-6;
    if r.is_nan() || r <= 2.0 * M {
        return None;
    }
    let g = 2.0 * r + 1.0;
    let (x0, y0, x1, y1) = (ip.min[0] - g, ip.min[1] - g, ip.max[0] + g, ip.max[1] + g);
    let (pa, pb, pc, pd) = (p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1));
    let bx = Region {
        loops: vec![Loop {
            segs: vec![
                Seg::Line { a: pa, b: pb },
                Seg::Line { a: pb, b: pc },
                Seg::Line { a: pc, b: pd },
                Seg::Line { a: pd, b: pa },
            ],
        }],
    };
    let free = region_of(&boolean(&bx, sec, Op::Diff).ok()?);
    let (c, _) = offset(&free, r - M).ok()?;
    // a piece whose reverse is also a piece: a one-dimensional part
    let segs: Vec<Seg> = c.iter().flat_map(|l| l.segs.iter().copied()).collect();
    for sg in &segs {
        let rv = sg.reversed();
        if segs.contains(&rv) {
            return None;
        }
    }
    let (pts, rest): (Vec<_>, Vec<_>) = c
        .into_iter()
        .partition(|l| l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b));
    let mut open: Option<Region> = None;
    if !rest.is_empty() {
        let (o, _) = offset(&region_of(&rest), -(r + M)).ok()?;
        open = Some(region_of(&o));
    }
    for l in pts {
        let c = l.segs[0].start();
        let a = p2(c.x + r + M, c.y);
        let disk = Region {
            loops: vec![Loop {
                segs: vec![Seg::Arc {
                    a,
                    b: a,
                    c,
                    ccw: true,
                }],
            }],
        };
        open = Some(match open {
            None => disk,
            Some(o) => region_of(&boolean(&o, &disk, Op::Union).ok()?),
        });
    }
    let lr = region_of(ul);
    match open {
        None => Some((vec![], ul.to_vec())),
        Some(o) => Some((
            boolean(&lr, &o, Op::Inter).ok()?,
            boolean(&lr, &o, Op::Diff).ok()?,
        )),
    }
}

/// The opening of a region by a disc of radius r, (R ⊖ B_r) ⊕ B_r: what a pocket with
/// stepover ≤ r provably clears.
/// A flat face's trim (its pieces, loop after loop) as a region: each loop closes where a piece
/// ends at its loop's start.
fn trim_region(segs: &[forge_cam_geom::check::TSeg]) -> Option<Region> {
    use forge_cam_geom::check::TSeg;
    let mut loops = vec![];
    let mut cur: Vec<Seg> = vec![];
    for t in segs {
        let s = match *t {
            TSeg::Line { a, b } => Seg::Line {
                a: p2(a[0], a[1]),
                b: p2(b[0], b[1]),
            },
            TSeg::Arc { a, b, c, ccw, .. } => Seg::Arc {
                a: p2(a[0], a[1]),
                b: p2(b[0], b[1]),
                c: p2(c[0], c[1]),
                ccw,
            },
        };
        cur.push(s);
        if cur[0].start() == s.end() {
            loops.push(Loop {
                segs: std::mem::take(&mut cur),
            });
        }
    }
    if !cur.is_empty() || loops.is_empty() {
        return None;
    }
    Some(Region { loops })
}

/// The points within w of the segment p0 p1: a stadium (two lines, two half circles in quarter
/// arcs), anticlockwise.
fn stadium(p0: [f64; 2], p1: [f64; 2], w: f64) -> Region {
    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
    let l = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / l * w, dy / l * w);
    let (nx, ny) = (-uy, ux);
    let q = |p: [f64; 2], a: f64, b: f64| p2(p[0] + a, p[1] + b);
    let (a0, a1, a2) = (q(p1, -nx, -ny), q(p1, ux, uy), q(p1, nx, ny));
    let (b0, b1, b2) = (q(p0, nx, ny), q(p0, -ux, -uy), q(p0, -nx, -ny));
    let (c1, c0) = (p2(p1[0], p1[1]), p2(p0[0], p0[1]));
    Region {
        loops: vec![Loop {
            segs: vec![
                Seg::Line { a: b2, b: a0 },
                Seg::Arc {
                    a: a0,
                    b: a1,
                    c: c1,
                    ccw: true,
                },
                Seg::Arc {
                    a: a1,
                    b: a2,
                    c: c1,
                    ccw: true,
                },
                Seg::Line { a: a2, b: b0 },
                Seg::Arc {
                    a: b0,
                    b: b1,
                    c: c0,
                    ccw: true,
                },
                Seg::Arc {
                    a: b1,
                    b: b2,
                    c: c0,
                    ccw: true,
                },
            ],
        }],
    }
}

/// The points within w of the circle about c of radius r (r > w): a ring.
fn ring(c: [f64; 2], r: f64, w: f64) -> Region {
    let circ = |rr: f64, ccw: bool| {
        let a = p2(c[0] + rr, c[1]);
        Loop {
            segs: vec![Seg::Arc {
                a,
                b: a,
                c: p2(c[0], c[1]),
                ccw,
            }],
        }
    };
    Region {
        loops: vec![circ(r + w, true), circ(r - w, false)],
    }
}

fn opening(rg: &Region, r: f64) -> Option<Region> {
    let dbg = |e: &dyn std::fmt::Debug| {
        if std::env::var("FORGE_CAM_DEBUG").is_ok() {
            eprintln!("opening: {e:?}");
        }
    };
    let (c, _) = forge_cam_geom::offset::offset(rg, r).map_err(|e| dbg(&e)).ok()?;
    // the tool centres' region; an isolated point of it (where the tool just fits) clears the
    // disk of radius r about it, so it is grown as that disk, apart from the rest
    let (pts, rest): (Vec<_>, Vec<_>) = c
        .into_iter()
        .partition(|l| l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b));
    let mut out: Option<Region> = None;
    if !rest.is_empty() {
        let cr = forge_cam_geom::boolean::region_of(&rest);
        let (o, _) = forge_cam_geom::offset::offset(&cr, -r)
            .map_err(|e| dbg(&e))
            .ok()?;
        out = Some(forge_cam_geom::boolean::region_of(&o));
    }
    for l in pts {
        let c = l.segs[0].start();
        let a = p2(c.x + r, c.y);
        let disk = Region {
            loops: vec![Loop {
                segs: vec![Seg::Arc {
                    a,
                    b: a,
                    c,
                    ccw: true,
                }],
            }],
        };
        out = Some(match out {
            None => disk,
            Some(o) => forge_cam_geom::boolean::region_of(
                &forge_cam_geom::boolean::boolean(&o, &disk, forge_cam_geom::boolean::Op::Union)
                    .map_err(|e| dbg(&e))
                    .ok()?,
            ),
        });
    }
    out
}

/// The support under the stock when no workholding is declared: the plane of the stock's
/// bottom, over the stock's box grown by 1 m each way (a path that leaves that square is far
/// outside any table this phase models; G5 bounds it by the travel when the setup is placed).
fn support_face(st: &Stock) -> Face {
    use forge_cam_geom::check::{Surf, TSeg, Trim};
    use forge_cam_geom::ival::Ival;
    let g = 1000.0;
    let (x0, y0) = (st.min[0] - g, st.min[1] - g);
    let (w, h) = (st.max[0] - st.min[0] + 2.0 * g, st.max[1] - st.min[1] + 2.0 * g);
    let rect = Trim {
        segs: vec![
            TSeg::Line {
                a: [0.0, 0.0],
                b: [w, 0.0],
            },
            TSeg::Line {
                a: [w, 0.0],
                b: [w, h],
            },
            TSeg::Line {
                a: [w, h],
                b: [0.0, h],
            },
            TSeg::Line {
                a: [0.0, h],
                b: [0.0, 0.0],
            },
        ],
    };
    Face::new(
        Surf::Plane {
            o: [x0, y0, st.min[2]],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
        },
        Ival::new(0.0, w),
        Ival::new(0.0, h),
        Some(rect),
        1e-9,
        3000,
    )
}

/// Each hole of a section as a pocket region: the hole's loop reversed (its inside on the
/// left), with the islands standing in it, the loops it directly holds, reversed as holes. The
/// section is validated (closed loops whose pieces meet only at shared ends), so one point
/// strictly inside a loop's first piece is on no other loop, and whether that loop lies inside
/// another is decided by that point alone (exact ray casting). This is the hole less the section
/// there, without a boolean of the section against its own loops (all ties: slow).
/// tan by forge-math's sin and cos (the workspace's trigonometry).
fn tan_rad(x: f64) -> f64 {
    let (s, c) = forge_math::dmath::sin_cos(x);
    s / c
}

/// The length of (x, y).
fn len2(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// A region's box: its segments' ends and each arc's extreme points that lie on it (in f64).
fn region_box(rg: &Region) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut add = |x: f64, y: f64| b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    for l in &rg.loops {
        for sg in &l.segs {
            let q = sg.start();
            add(q.x, q.y);
            if let Seg::Arc { a, b: e, c, ccw } = *sg {
                let r = len2(a.x - c.x, a.y - c.y);
                let ang = |p: P2| forge_math::dmath::atan2(p.y - c.y, p.x - c.x);
                // the sweep from a to e in the arc's sense, a full turn when they meet
                let (t0, t1) = (ang(a), ang(e));
                let tau = std::f64::consts::TAU;
                let mut sweep = if ccw { t1 - t0 } else { t0 - t1 };
                sweep = sweep.rem_euclid(tau);
                if a == e {
                    sweep = tau;
                }
                for k in 0..4 {
                    let t = k as f64 * 0.5 * std::f64::consts::PI;
                    let d = if ccw { t - t0 } else { t0 - t };
                    if d.rem_euclid(tau) <= sweep {
                        let (sn, cs) = forge_math::dmath::sin_cos(t);
                        add(c.x + r * cs, c.y + r * sn);
                    }
                }
            }
        }
    }
    b
}

/// Whether the pocket `rg` (a hole of a higher section, its outer loop counter-clockwise) is
/// still open in the section `below`: a point just inside rg's outer loop (1e-3 to the left of
/// its first segment) lies in no material of `below` (inside an even number of its loops). A
/// planning choice, not a proof: the certifier checks whatever is cut.
fn reaches(below: &Region, rg: &Region) -> bool {
    use forge_cam_geom::region;
    let Some(sg) = rg.loops.first().and_then(|l| l.segs.first()) else {
        return true;
    };
    let eps = 1e-3;
    let q = match *sg {
        Seg::Line { a, b } => {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let n = len2(dx, dy);
            if n == 0.0 {
                return true;
            }
            (0.5 * (a.x + b.x) - eps * dy / n, 0.5 * (a.y + b.y) + eps * dx / n)
        }
        Seg::Arc { a, c, ccw, .. } => {
            let (dx, dy) = (c.x - a.x, c.y - a.y);
            let n = len2(dx, dy);
            if n == 0.0 {
                return true;
            }
            // left of travel: towards the centre on a counter-clockwise arc
            let k = if ccw { eps / n } else { -eps / n };
            (a.x + k * dx, a.y + k * dy)
        }
    };
    let mut cx = forge_cam_geom::real::Ctx::new();
    let Ok(ex) = region::validate(&mut cx, below) else {
        return true;
    };
    let p = forge_cam_geom::arr::pt(&mut cx, q.0, q.1);
    let inside = (0..below.loops.len())
        .filter(|&l| region::inside_loop(&mut cx, &ex, l, p))
        .count();
    inside % 2 == 0
}

/// Whether the pocket `rg` is a blind hole feature drilled down whose wall ends above z
/// `bottom`: its box is the hole's circle (centre and diameter within 1e-3).
fn hole_ends_above(holes: &[holes::HoleInfo], rg: &Region, bottom: f64) -> bool {
    let b = region_box(rg);
    let (cx, cy, w) = (0.5 * (b[0] + b[2]), 0.5 * (b[1] + b[3]), b[2] - b[0]);
    holes.iter().any(|h| {
        let Some(depth) = h.depth else {
            return false;
        };
        h.dir[2] < -0.999
            && (w - h.d).abs() <= 1e-3
            && h.places.iter().any(|p| {
                (p[0] - cx).abs() <= 1e-3 && (p[1] - cy).abs() <= 1e-3 && p[2] - depth > bottom + 1e-3
            })
    })
}

fn pockets_of(sec: &Region) -> Result<Vec<Region>, String> {
    use forge_cam_geom::region;
    let mut cx = forge_cam_geom::real::Ctx::new();
    let ex = region::validate(&mut cx, sec).map_err(|e| format!("{e:?}"))?;
    let n = sec.loops.len();
    let ccw: Vec<bool> = (0..n).map(|l| region::loop_ccw(&mut cx, &ex, l)).collect();
    let rep: Vec<_> = ex
        .loops
        .iter()
        .map(|&(first, _)| {
            let e = ex.elems[first];
            forge_cam_geom::arr::rep_point(&mut cx, &e.piece, e.piece.s, e.piece.e)
        })
        .collect();
    // within[k][l]: loop k lies inside loop l
    let within: Vec<Vec<bool>> = rep
        .iter()
        .enumerate()
        .map(|(k, q)| {
            (0..n)
                .map(|l| k != l && region::inside_loop(&mut cx, &ex, l, *q))
                .collect()
        })
        .collect();
    let mut out = vec![];
    for h in (0..n).filter(|&h| !ccw[h]) {
        let mut loops = vec![sec.loops[h].reversed()];
        for k in 0..n {
            // directly inside h: inside it, and inside nothing that is inside it
            if within[k][h] && !(0..n).any(|m| m != k && m != h && within[k][m] && within[m][h]) {
                loops.push(sec.loops[k].reversed());
            }
        }
        out.push(Region { loops });
    }
    if out.is_empty() {
        return Err("no inner boundary (hole) at that height".into());
    }
    Ok(out)
}

/// The section's outer loops alone (outside work), or its islands as pocket boundaries.
/// A contour's passes from its arguments: `direction: "climb" | "conventional"` (default
/// climb), `finish: n` radial passes
/// (default 1) `finish_step:` apart (default a tenth of the tool's diameter), `spring: n` the
/// last pass again (default 0).
fn contour_passes(a: &BTreeMap<String, V>, d: f64) -> Result<ops::Passes, String> {
    let count = |k: &str, default: f64, least: f64| -> Result<usize, String> {
        let x = get_num(a, k, Some(default))?;
        if x.fract() != 0.0 || x < least || x > 100.0 {
            return Err(format!("{k}: a whole number from {least} to 100"));
        }
        Ok(x as usize)
    };
    let climb = match a.get("direction") {
        None => true,
        Some(V::Str(s)) if s == "climb" => true,
        Some(V::Str(s)) if s == "conventional" => false,
        Some(_) => return Err("direction: \"climb\" or \"conventional\"".into()),
    };
    let finish_step = get_num(a, "finish_step", Some(0.1 * d))?;
    if !(finish_step > 0.0 && finish_step.is_finite()) {
        return Err("finish_step: a length over 0".into());
    }
    Ok(ops::Passes {
        climb,
        finish: count("finish", 1.0, 1.0)?,
        finish_step,
        spring: count("spring", 0.0, 0.0)?,
    })
}

/// A contour's leads from its arguments: `lead: "none" | "arc" | "line"` (default none: the
/// tool comes down on the loop), `lead_radius:` (an arc's, default half the tool's diameter),
/// `lead_sweep:` (degrees, default 90), `lead_length:` (the tangent line before the arc in and
/// after it out; default 0 for an arc, half the tool's diameter for a line), `entry: (x, y)`
/// (start each loop at its point nearest this one) and `overlap:` (cut on past the start by
/// this length before leaving, default 0). The same lead goes in and out.
fn contour_leads(a: &BTreeMap<String, V>, d: f64) -> Result<forge_cam::leads::Leads, String> {
    use forge_cam::leads::{Lead, Leads};
    let kind = get_str(a, "lead", Some("none"))?;
    let len_arg = |k: &str, default: f64| -> Result<f64, String> {
        let x = get_num(a, k, Some(default))?;
        if !(x >= 0.0 && x.is_finite()) {
            return Err(format!("{k}: a length, 0 or more"));
        }
        Ok(x)
    };
    let lead = match kind.as_str() {
        "none" => {
            for k in ["lead_radius", "lead_sweep", "lead_length"] {
                if a.contains_key(k) {
                    return Err(format!(
                        "`{k}:` needs a lead: add `lead: \"arc\"` or `lead: \"line\"`"
                    ));
                }
            }
            None
        }
        "arc" => {
            let radius = len_arg("lead_radius", 0.5 * d)?;
            let sweep_deg = get_num(a, "lead_sweep", Some(90.0))?;
            if !(sweep_deg > 0.0 && sweep_deg <= 180.0) {
                return Err("lead_sweep: degrees, over 0 and at most 180".into());
            }
            if radius <= 0.0 {
                return Err("lead_radius: a length over 0 (or `lead: \"line\"`)".into());
            }
            Some(Lead {
                radius,
                sweep_deg,
                length: len_arg("lead_length", 0.0)?,
            })
        }
        "line" => {
            if a.contains_key("lead_radius") || a.contains_key("lead_sweep") {
                return Err(
                    "a line lead takes `lead_length:` only (an arc takes `lead_radius:` and `lead_sweep:`)"
                        .into(),
                );
            }
            let length = len_arg("lead_length", 0.5 * d)?;
            if length <= 0.0 {
                return Err("lead_length: a length over 0".into());
            }
            Some(Lead {
                radius: 0.0,
                sweep_deg: 0.0,
                length,
            })
        }
        k => return Err(format!("lead: \"none\", \"arc\" or \"line\", not \"{k}\"")),
    };
    let entry = match a.get("entry") {
        None => None,
        Some(V::Tuple(t)) if t.len() == 2 => match (num(&t[0]), num(&t[1])) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some([x, y]),
            _ => return Err("entry: a point (x, y)".into()),
        },
        Some(_) => return Err("entry: a point (x, y)".into()),
    };
    Ok(Leads {
        lead_in: lead,
        lead_out: lead,
        entry,
        overlap: len_arg("overlap", 0.0)?,
    })
}

/// A contour's holding tabs from its arguments: `tabs: n` per loop evenly spaced, or
/// `tab_spacing:` (one every that length), or `tab_at: [(x, y), ..]` (one at each loop's point
/// nearest each); `tab_width:` (the bridge, default the tool's diameter), `tab_height:` (above
/// the contour's bottom, default 1, less than its depth), `tab_shape: "rect" | "triangle"`
/// (default rect). None of them: no tabs.
fn contour_tabs(
    a: &BTreeMap<String, V>,
    d: f64,
    depth: f64,
) -> Result<Option<forge_cam::tabs::Tabs>, String> {
    use forge_cam::tabs::{TabPlace, TabShape, Tabs};
    let given: Vec<&str> = ["tabs", "tab_spacing", "tab_at"]
        .into_iter()
        .filter(|k| a.contains_key(*k))
        .collect();
    let place = match given.as_slice() {
        [] => {
            for k in ["tab_width", "tab_height", "tab_shape"] {
                if a.contains_key(k) {
                    return Err(format!(
                        "`{k}:` needs tabs: add `tabs: N`, `tab_spacing:` or `tab_at:`"
                    ));
                }
            }
            return Ok(None);
        }
        ["tabs"] => {
            let n = get_num(a, "tabs", None)?;
            if n.fract() != 0.0 || !(1.0..=100.0).contains(&n) {
                return Err("tabs: a whole number from 1 to 100".into());
            }
            TabPlace::Count(n as usize)
        }
        ["tab_spacing"] => {
            let sp = get_num(a, "tab_spacing", None)?;
            if !(sp > 0.0 && sp.is_finite()) {
                return Err("tab_spacing: a length over 0".into());
            }
            TabPlace::Spacing(sp)
        }
        ["tab_at"] => {
            let vs: Vec<&V> = match a.get("tab_at") {
                // one point alone
                Some(v @ V::Tuple(t)) if t.len() == 2 && t.iter().all(|x| matches!(x, V::Num(_))) => vec![v],
                Some(V::Tuple(t)) => t.iter().collect(),
                _ => vec![],
            };
            let mut ps = vec![];
            for v in vs {
                match v {
                    V::Tuple(t) if t.len() == 2 => match (num(&t[0]), num(&t[1])) {
                        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => ps.push([x, y]),
                        _ => return Err("tab_at: points (x, y)".into()),
                    },
                    _ => return Err("tab_at: points (x, y)".into()),
                }
            }
            if ps.is_empty() {
                return Err("tab_at: points (x, y)".into());
            }
            TabPlace::At(ps)
        }
        _ => return Err("give one of `tabs:`, `tab_spacing:` and `tab_at:`".into()),
    };
    let width = get_num(a, "tab_width", Some(d))?;
    if !(width > 0.0 && width.is_finite()) {
        return Err("tab_width: a length over 0".into());
    }
    let height = get_num(a, "tab_height", Some(1.0))?;
    if !(height > 0.0 && height < depth) {
        return Err(format!(
            "tab_height: a length over 0 and under the contour's depth ({depth})"
        ));
    }
    let shape = match get_str(a, "tab_shape", Some("rect"))?.as_str() {
        "rect" => TabShape::Rect,
        "triangle" => TabShape::Triangle,
        k => return Err(format!("tab_shape: \"rect\" or \"triangle\", not \"{k}\"")),
    };
    Ok(Some(Tabs {
        place,
        width,
        height,
        shape,
    }))
}

/// A thread mill's passes from its arguments.
struct ThreadParams {
    passes: usize,
    spring: usize,
    starts: usize,
    left: bool,
    arc_lead: bool,
}

/// `passes:` radial passes (default 1), `spring:` (default 0), `starts:` (default 1),
/// `hand: "right" | "left"` (default right), `lead: "line" | "arc"` (default line).
fn thread_params(a: &BTreeMap<String, V>) -> Result<ThreadParams, String> {
    let count = |k: &str, default: f64, least: f64| -> Result<usize, String> {
        let x = get_num(a, k, Some(default))?;
        if x.fract() != 0.0 || x < least || x > 20.0 {
            return Err(format!("{k}: a whole number from {least} to 20"));
        }
        Ok(x as usize)
    };
    let left = match get_str(a, "hand", Some("right"))?.as_str() {
        "right" => false,
        "left" => true,
        k => return Err(format!("hand: \"right\" or \"left\", not \"{k}\"")),
    };
    let arc_lead = match get_str(a, "lead", Some("line"))?.as_str() {
        "line" => false,
        "arc" => true,
        k => return Err(format!("thread_mill's lead: \"line\" or \"arc\", not \"{k}\"")),
    };
    Ok(ThreadParams {
        passes: count("passes", 1.0, 1.0)?,
        spring: count("spring", 0.0, 0.0)?,
        starts: count("starts", 1.0, 1.0)?,
        left,
        arc_lead,
    })
}

/// The section's outer boundary (its loops inside no other: the outline) or its holes (the
/// loops directly inside an outer one), as a region. An island standing in a hole (a loop two
/// deep, counter-clockwise like an outline) is neither: taken as an outline it made the region
/// refused (a sense at the wrong depth), and a hole in such an island is not the part's hole.
fn outer_or_inner(sec: &Region, outer: bool) -> Result<Region, String> {
    let mut cx = forge_cam_geom::real::Ctx::new();
    let ex = forge_cam_geom::region::validate(&mut cx, sec).map_err(|e| format!("{e:?}"))?;
    let mut loops = vec![];
    for l in 0..sec.loops.len() {
        // its depth: how many other loops hold a point of it (its first vertex)
        let q = ex.elems[ex.loops[l].0].piece.s;
        let depth = (0..sec.loops.len())
            .filter(|&k| k != l && forge_cam_geom::region::inside_loop(&mut cx, &ex, k, q))
            .count();
        if depth == if outer { 0 } else { 1 } {
            loops.push(if outer {
                sec.loops[l].clone()
            } else {
                sec.loops[l].reversed()
            });
        }
    }
    if loops.is_empty() {
        return Err(if outer {
            "no outer boundary at that height".into()
        } else {
            "no inner boundary (hole) at that height".into()
        });
    }
    Ok(Region { loops })
}

impl Cam {
    /// G5 for one op's posted path: with the setup placed (`at:`), every position (an arc by its
    /// circle's box, a superset) as each joint sees it (the spindle's Z: the tip's plus the tool's
    /// length, stick-out and holder gauge) within the joint's travel, proven in outward-rounded
    /// arithmetic; unplaced, each axis's span within its travel's width. The first violation.
    fn axis_limits(&self, oi: usize, posted: &OpPath) -> Option<CheckOut> {
        let s = &self.setups[self.ops[oi].setup];
        let m = self.lib.machine(&s.machine).ok()?;
        let t = &self.tools[posted.tool];
        let gauge = t
            .holder
            .as_ref()
            .and_then(|h| self.lib.holder(h).ok())
            .map_or(0.0, |h| h.gauge_length);
        let len = t.stickout + gauge;
        // the path's box in program coordinates (part frame less the WCS origin)
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut add = |p: [f64; 3]| {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k] - s.origin[k]);
                hi[k] = hi[k].max(p[k] - s.origin[k]);
            }
        };
        add(posted.start);
        let mut at = posted.start;
        for mv in &posted.moves {
            match *mv {
                PMove::Rapid { to } | PMove::Line { to, .. } => {
                    add(to);
                    at = to;
                }
                PMove::Arc { to, c, .. } => {
                    let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt() * (1.0 + 1e-12);
                    add([c[0] - r, c[1] - r, to[2].min(at[2])]);
                    add([c[0] + r, c[1] + r, to[2].max(at[2])]);
                    add(to);
                    at = to;
                }
                PMove::Cycle { .. } => {}
            }
        }
        use forge_cam_geom::ival::Ival;
        for k in 0..3 {
            let Some(j) = m.joints.iter().find(|j| {
                let a = j.axis;
                (0..3).all(|q| if q == k { a[q].abs() == 1.0 } else { a[q] == 0.0 })
            }) else {
                continue;
            };
            let sign = j.axis[k];
            let extra = if k == 2 { len } else { 0.0 };
            let fail = |value: f64, limit: f64, what: String| CheckOut {
                rule: "axis_limit",
                verdict: "fail",
                value,
                limit,
                at: [f64::NAN; 3],
                tool_part: "machine",
                other: what,
                phase: "path",
                move_index: 0,
                path: 0,
            };
            match s.at {
                Some(o) => {
                    // the joint's coordinate at the box's ends: sign (p + at) + extra
                    for (p, end) in [(lo[k], "least"), (hi[k], "greatest")] {
                        let v = Ival::pt(p).add(Ival::pt(o[k])).add(Ival::pt(extra)).scale(sign);
                        if !(v.lo >= j.range[0] && v.hi <= j.range[1]) {
                            let lim = if v.lo < j.range[0] { j.range[0] } else { j.range[1] };
                            return Some(fail(
                                if v.lo < j.range[0] { v.lo } else { v.hi },
                                lim,
                                format!(
                                    "joint {}: the path's {end} reach is outside its travel {:?}",
                                    j.name, j.range
                                ),
                            ));
                        }
                    }
                }
                None => {
                    let span = Ival::pt(hi[k]).sub(Ival::pt(lo[k])).hi;
                    let width = j.range[1] - j.range[0];
                    if span > width {
                        return Some(fail(
                            span,
                            width,
                            format!(
                                "joint {}: the path spans {span:.3} mm, the travel {width:.3} mm",
                                j.name
                            ),
                        ));
                    }
                }
            }
        }
        None
    }

    /// G5's runtime limits for one op's posted records, as the control reads them back: every
    /// spindle speed (S, a whole number) within the machine's range, and every feed (F, at
    /// `feed_decimals`, so F = q / 10^k exactly) at most its feed limit. Decided exactly: q and
    /// the limits are compared as integers and f64 values scaled in outward-rounded arithmetic
    /// (a limit's scaled lower end), so a pass is never a rounding. The first violation.
    fn runtime_limits(&self, oi: usize, ms: &[forge_post::Motion], feed_decimals: u32) -> Option<CheckOut> {
        let s = &self.setups[self.ops[oi].setup];
        let m = self.lib.machine(&s.machine).ok()?;
        let fail = |value: f64, limit: f64, what: String, i: usize| CheckOut {
            rule: "machine_limit",
            verdict: "fail",
            value,
            limit,
            at: [f64::NAN; 3],
            tool_part: "machine",
            other: what,
            phase: "path",
            move_index: i,
            path: 0,
        };
        // 10^k exactly (k ≤ 22, so the power is an exact f64), then the limit times it
        let k = feed_decimals.min(22) as i32;
        let fs = 10f64.powi(k);
        let over = |q: i64| feed_over(q, k as u32, m.max_feed);
        for (i, mo) in ms.iter().enumerate() {
            match *mo {
                // |q| < 2^53: the integer is the f64
                forge_post::Motion::Spindle(q, _)
                    if q.unsigned_abs() >= 1 << 53
                        || (q as f64) < m.rpm[0]
                        || (q as f64) > m.rpm[1]
                        || m.rpm[0].is_nan()
                        || m.rpm[1].is_nan() =>
                {
                    let n = q as f64;
                    return Some(fail(
                        n,
                        if n < m.rpm[0] { m.rpm[0] } else { m.rpm[1] },
                        format!(
                            "spindle S{q}: outside the machine's range {} to {} rpm",
                            m.rpm[0], m.rpm[1]
                        ),
                        i,
                    ));
                }
                forge_post::Motion::Feed(_, q) | forge_post::Motion::Arc { f: q, .. }
                    if q.unsigned_abs() >= 1 << 53 || over(q) =>
                {
                    let v = q as f64 / fs;
                    return Some(fail(
                        v,
                        m.max_feed,
                        format!("feed F{v}: over the machine's feed limit {} mm/min", m.max_feed),
                        i,
                    ));
                }
                _ => {}
            }
        }
        None
    }

    /// The certified checks again, on demand, over what was posted (no op is generated again):
    /// for each op (or the one named), every posted path checked by the full checker, the
    /// staged in-process stock built for every path (none skipped), the fixture or support,
    /// and G5's travel. The checks name the path they are on (`CheckOut::path`).
    pub fn check(&self, op: Option<&str>) -> Vec<OpChecks> {
        let mut out = vec![];
        for (oi, o) in self.ops.iter().enumerate() {
            if op.is_some_and(|n| n != o.name) {
                continue;
            }
            let mut oc = OpChecks {
                op: o.name.clone(),
                clear: !o.posted.is_empty(),
                checks: vec![],
                pairs: 0,
                cells: 0,
            };
            for (pi, posted) in o.posted.iter().enumerate() {
                let mut sum = self.certify_posted(oi, posted, true);
                if let Some(c) = self.axis_limits(oi, posted) {
                    sum.clear = false;
                    sum.checks.insert(0, c);
                }
                for c in &mut sum.checks {
                    c.path = pi;
                }
                oc.clear &= sum.clear;
                oc.pairs += sum.pairs;
                oc.cells += sum.cells;
                oc.checks.extend(sum.checks);
            }
            out.push(oc);
        }
        out
    }

    /// G1–G3 and the fixture (or support) for one posted path of op `oi`, against the staged
    /// in-process stock: built where a check against it could fail, or always with `full_stock`
    /// (the on-demand check). The checks' fixture names filled in.
    fn certify_posted(&self, oi: usize, posted: &OpPath, full_stock: bool) -> ops::CheckSummary {
        let s = &self.setups[self.ops[oi].setup];
        let path = posted;
        let tool = self.tools[path.tool].clone();
        let part = &self.parts[s.part];
        // the stock at each move: the op's start, then after each completed level (a
        // retract after cutting at depth d), its voids cleared down to d (levels go down)
        let base = self.ops[oi]
            .stock_before
            .clone()
            .unwrap_or_else(|| forge_cam::stock::InProcess::new(s.stock.min, s.stock.max));
        let faces_of = |ip: &forge_cam::stock::InProcess| {
            // a clock only when asked: std's panics in a browser page
            let t0 = std::env::var("FORGE_CAM_DEBUG")
                .is_ok()
                .then(forge_math::clock::Instant::now);
            let r = ip.faces();
            if let Some(t0) = t0 {
                eprintln!(
                    "{}: stock faces {:?} in {:?} (voids {}, under {})",
                    s.name,
                    r.as_ref().map(|f| f.len()),
                    t0.elapsed(),
                    ip.voids.len(),
                    ip.under.len()
                );
            }
            r.unwrap_or_else(|e| {
                if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                    eprintln!("stock faces: {e}");
                }
                s.stock.faces()
            })
        };
        // the stock is checked against the tool's non-cutting bodies, and against every body
        // on a rapid. Where each such body stays, through each move, at least the
        // clearance (and the stock faces' μ, with room) above the stock's top, no stock point
        // comes within the limit (the stock lies under its top; the limit is at most the
        // clearance): those checks cannot fail, and the staged stock is not built
        // (Heights.lean `setDist_of_heights`). Else it is, as before.
        let top = base.max[2];
        let stock_needed = {
            use forge_cam_geom::ival::Ival;
            // the least a checked body's lowest point may be: top + clearance + room
            let need = Ival::pt(top)
                .add(Ival::pt(s.clearance.max(0.0)))
                .add(Ival::pt(1e-3))
                .hi;
            ops::checker_moves(posted).iter().any(|(m, ph, _)| {
                let tip = m.at(Ival::new(0.0, 1.0))[2].lo;
                tool.shape.bodies.iter().any(|b| {
                    let low = Ival::pt(tip).add(Ival::pt(b.z0)).lo;
                    // above it (a NaN is not)
                    let above = matches!(
                        low.partial_cmp(&need),
                        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                    );
                    (b.part == forge_cam_geom::tool::Part::NonCutting || *ph == Phase::Rapid) && !above
                })
            })
        };
        // FULL_STOCK: build it anyway (the differential test of the skip)
        let stock_needed = stock_needed || full_stock;
        let mut stages = if stock_needed {
            vec![(0usize, faces_of(&base))]
        } else {
            vec![(0usize, vec![])]
        };
        let mut deepest = f64::INFINITY;
        let mut cut_z: Option<f64> = None;
        for (i, m) in posted.moves.iter().enumerate() {
            match *m {
                // a level's depth: the lowest point it cut (the feed back up is not one)
                PMove::Line {
                    to,
                    phase: Phase::Cut,
                    ..
                }
                | PMove::Arc { to, .. } => cut_z = Some(cut_z.map_or(to[2], |c: f64| c.min(to[2]))),
                PMove::Line {
                    phase: Phase::Link, ..
                }
                | PMove::Rapid { .. } => {
                    if let Some(z) = cut_z.take()
                        && stock_needed
                        && z < deepest
                        && !self.ops[oi].voids.is_empty()
                    {
                        deepest = z;
                        let mut ip = base.clone();
                        for (v, floor) in &self.ops[oi].voids {
                            let f = floor.max(z);
                            match v {
                                None => ip.face_to(f),
                                Some(r) => ip.cut(r.clone(), f),
                            }
                        }
                        if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                            let m = ip.material(z + 0.1).map(|m| {
                                m.map(|m| {
                                    (
                                        m.loops.len(),
                                        m.loops.iter().map(|l| l.segs.len()).collect::<Vec<_>>(),
                                    )
                                })
                            });
                            eprintln!("stage at {z}: levels {:?} material above {m:?}", ip.levels());
                        }
                        stages.push((i, faces_of(&ip)));
                    }
                }
                _ => {}
            }
        }
        if std::env::var("FORGE_CAM_DEBUG").is_ok() {
            eprintln!(
                "{}: voids {:?} stages {:?}",
                self.ops[oi].name,
                self.ops[oi]
                    .voids
                    .iter()
                    .map(|v| (v.0.as_ref().map(|r| r.loops.len()), v.1))
                    .collect::<Vec<_>>(),
                stages.iter().map(|s| (s.0, s.1.len())).collect::<Vec<_>>()
            );
        }
        // with no workholding declared, whatever holds the stock is under it: the plane of the
        // stock's bottom is the support, and no body of the tool may reach it (G2)
        let (ffaces, fclear) = match &s.fixture {
            Some(f) => (f.faces(), f.clearance),
            None => (vec![support_face(&s.stock)], 0.0),
        };
        let target = self.ops[oi].target.as_ref().unwrap_or(&part.faces);
        let surfacing = matches!(
            self.ops[oi].kind,
            "parallel"
                | "waterline"
                | "zlevel"
                | "pencil"
                | "scallop"
                | "rest"
                | "steep_shallow"
                | "spiral"
                | "radial"
                | "ramping"
                | "flat"
        );
        let mut sum = ops::certify_with(
            posted,
            &tool,
            target,
            &stages,
            &ffaces,
            fclear,
            s.tol,
            s.clearance,
            surfacing,
        );
        if let Some(f) = &s.fixture {
            for c in &mut sum.checks {
                if c.rule == "fixture_collision"
                    && let Some(id) = c.other.strip_prefix("fixture face ").and_then(|x| x.parse().ok())
                    && let Some(p) = f.piece_of(id)
                {
                    c.other = format!("{} ({p})", c.other);
                }
            }
        } else {
            for c in &mut sum.checks {
                if c.rule == "fixture_collision" {
                    c.other = "the support under the stock (no workholding declared)".into();
                }
            }
        }
        sum
    }

    /// Posts the setups' ops, reads the text back, and certifies the path the text means.
    fn post(
        &mut self,
        name: &str,
        setups: &[usize],
        sel: Option<&[usize]>,
        post: &str,
        number: u32,
        opts: &PostOpts,
    ) -> ProgramOut {
        let mut po = ProgramOut {
            setups: setups.to_vec(),
            ops: vec![],
            setup: None,
            name: name.into(),
            post: post.into(),
            text: None,
            lines: 0,
            error: None,
            round_trip: false,
        };
        let mut d = match forge_post::Dialect::get(post) {
            Ok(d) => d,
            Err(e) => {
                po.error = Some(format!("{e:?}"));
                return po;
            }
        };
        // the program's own number formats: the text is quantised, read back and certified at
        // them, as at the dialect's
        if let Some(k) = opts.decimals {
            d.decimals = k;
        }
        if let Some(k) = opts.feed_decimals {
            d.feed_decimals = k;
        }
        let mut recs = vec![];
        let mut current_tool: Option<u32> = None;
        // where the last path left the tool, in the part's frame
        let mut last: Option<(usize, [f64; 3])> = None;
        // each op's records, and which records are whose
        let mut spans: Vec<(usize, usize, usize, usize)> = vec![];
        let op_ids: Vec<usize> = (0..self.ops.len())
            .filter(|&i| {
                setups.contains(&self.ops[i].setup)
                    && !self.ops[i].paths.is_empty()
                    && sel.is_none_or(|s| s.contains(&i))
            })
            .collect();
        po.ops = op_ids.clone();
        for &oi in &op_ids {
            let op = &self.ops[oi];
            let s = &self.setups[op.setup];
            let o = s.origin;
            let w = |p: [f64; 3]| [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
            recs.push(Rec::Comment(format!("{} ({})", op.name, op.kind)));
            if current_tool.is_none() {
                recs.push(Rec::Wcs(s.wcs));
            }
            // a hole plan's several tools: one path each
            for (pi, p) in op.paths.iter().enumerate() {
                let start = recs.len();
                let t = &self.tools[p.tool];
                if current_tool != Some(t.t) {
                    // GRBL changes tools by hand (M0) and has no machine-safe Z to go to: Z up to
                    // the setup's clearance height in the work coordinates first (known in the
                    // WCS without homing), and the operator told to set Z for the new tool
                    if post == "grbl"
                        && current_tool.is_some()
                        && let Some((ls, l)) = last
                        && ls == op.setup
                    {
                        recs.push(Rec::Rapid(w([l[0], l[1], s.heights.clear])));
                    }
                    if post == "grbl" && current_tool.is_some() {
                        recs.push(Rec::Comment("set Z zero for the new tool before resuming".into()));
                    }
                    if current_tool.is_some() {
                        recs.push(Rec::SpindleStop);
                        recs.push(Rec::Coolant(forge_post::Coolant::Off));
                    }
                    // Z to the machine's safe position first (the dialect's retract_home: G91 G28 Z0,
                    // G53 G0 Z0): an M06 need not move Z, and a longer tool would come down
                    // into the stock where the last one stood (GRBL has none: above)
                    if post != "grbl" {
                        recs.push(Rec::Home);
                    }
                    recs.push(Rec::ToolChange {
                        t: t.t,
                        h: t.t,
                        name: t.tool.id.clone(),
                    });
                    current_tool = Some(t.t);
                }
                let rpm = if p.rpm > 0.0 {
                    p.rpm
                } else {
                    op.feeds.map(|f| f.rpm).unwrap_or(3000.0)
                };
                recs.push(Rec::Spindle { rpm, cw: true });
                recs.push(Rec::Coolant(opts.coolant.unwrap_or(p.coolant)));
                recs.push(Rec::Rapid(w(p.start)));
                for m in &p.moves {
                    recs.push(match *m {
                        PMove::Rapid { to } => Rec::Rapid(w(to)),
                        PMove::Line { to, f, .. } => Rec::Line(w(to), f),
                        PMove::Arc { to, c, cw, f } => Rec::Arc {
                            to: w(to),
                            c: [c[0] - o[0], c[1] - o[1]],
                            cw,
                            f,
                        },
                        PMove::Cycle {
                            kind,
                            holes,
                            z,
                            r,
                            clear,
                            q,
                            dwell,
                            f,
                        } => Rec::Cycle {
                            kind,
                            holes: vec![[holes[0] - o[0], holes[1] - o[1]]],
                            z: z - o[2],
                            r: r - o[2],
                            clear: clear - o[2],
                            q,
                            dwell,
                            f,
                        },
                    });
                }
                // where this path leaves the tool (the part's frame)
                let mut at = p.start;
                for m in &p.moves {
                    match *m {
                        PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => at = to,
                        PMove::Cycle { holes, clear, .. } => at = [holes[0], holes[1], clear],
                    }
                }
                last = Some((op.setup, at));
                spans.push((oi, pi, start, recs.len()));
            }
        }
        for &oi in &op_ids {
            let op = &mut self.ops[oi];
            (
                op.moves,
                op.pairs,
                op.cells,
                op.length_cut,
                op.length_rapid,
                op.time_s,
            ) = (0, 0, 0, 0.0, 0.0, 0.0);
            op.deepest = f64::INFINITY;
            op.posted.clear();
        }
        recs.push(Rec::SpindleStop);
        recs.push(Rec::Coolant(forge_post::Coolant::Off));
        if post != "grbl" {
            recs.push(Rec::Home);
        }
        let program = forge_post::Program {
            number,
            title: name.into(),
            recs,
        };
        let q = match forge_post::quantize(&program, &d) {
            Ok(q) => q,
            Err(e) => {
                po.error = Some(format!("{e:?}"));
                return po;
            }
        };
        match forge_post::round_trip(&q, &d) {
            Ok((txt, _)) => {
                po.round_trip = true;
                po.lines = txt.lines().count();
                po.text = Some(txt);
            }
            Err(e) => {
                po.error = Some(e);
                return po;
            }
        }
        // certify what the text means: each op's records, quantised, as motion, back in the
        // part's frame
        let scale = 10f64.powi(d.decimals as i32);
        let fscale = 10f64.powi(d.feed_decimals as i32);
        for (oi, pi, a, b) in spans {
            let s = &self.setups[self.ops[oi].setup];
            let o = s.origin;
            let sub = forge_post::QProgram {
                number: 0,
                title: String::new(),
                recs: q.recs[a..b].to_vec(),
                decimals: d.decimals,
                feed_decimals: d.feed_decimals,
            };
            // the control's peck retract and chip-break lift: the posted program does not fix
            // them; the path is certified so that any value in [0, the depth already drilled]
            // gives the same verdicts (CYCLE_PARAMS: tests/peck.rs)
            let ms = forge_post::motions_with(&sub, *CYCLE_PARAMS.lock().unwrap_or_else(|e| e.into_inner()));
            // G5's runtime half: what the text commands the spindle and the feed, as read back
            let runtime = self.runtime_limits(oi, &ms, d.feed_decimals);
            let back = |p: [i64; 3]| {
                [
                    p[0] as f64 / scale + o[0],
                    p[1] as f64 / scale + o[1],
                    p[2] as f64 / scale + o[2],
                ]
            };
            let path = &self.ops[oi].paths[pi];
            let mut moves = vec![];
            let mut start: Option<[f64; 3]> = None;
            let (mut lc, mut lr, mut time) = (0.0, 0.0, 0.0);
            let mut column: Option<([f64; 2], f64, f64)> = None;
            let mut at = [0.0; 3];
            for m in &ms {
                let (to, ph, f) = match m {
                    forge_post::Motion::Rapid(p) => (back(*p), Phase::Rapid, 0.0),
                    forge_post::Motion::Feed(p, f) => (back(*p), Phase::Cut, *f as f64 / fscale),
                    forge_post::Motion::Arc { to, c, cw, f, full } => {
                        let t = back(*to);
                        if start.is_none() {
                            start = Some(t);
                        }
                        let cc = [c[0] as f64 / scale + o[0], c[1] as f64 / scale + o[1]];
                        let r = ((at[0] - cc[0]).powi(2) + (at[1] - cc[1]).powi(2)).sqrt();
                        // its length: the turn it sweeps (as the control reads it: a whole turn
                        // when `full`), with the height it climbs; its time at its feed
                        let tau = std::f64::consts::TAU;
                        let a0 = forge_math::dmath::atan2(at[1] - cc[1], at[0] - cc[0]);
                        let a1 = forge_math::dmath::atan2(t[1] - cc[1], t[0] - cc[0]);
                        let mut sweep = if *cw { a0 - a1 } else { a1 - a0 };
                        while sweep < 0.0 {
                            sweep += tau;
                        }
                        while sweep >= tau {
                            sweep -= tau;
                        }
                        if *full {
                            sweep = tau;
                        }
                        let l = ((r * sweep).powi(2) + (t[2] - at[2]).powi(2)).sqrt();
                        lc += l;
                        let fa = *f as f64 / fscale;
                        time += if fa > 0.0 { l / fa * 60.0 } else { 0.0 };
                        moves.push(PMove::Arc {
                            to: t,
                            c: [c[0] as f64 / scale + o[0], c[1] as f64 / scale + o[1]],
                            cw: *cw,
                            f: *f as f64 / fscale,
                        });
                        at = t;
                        column = None;
                        continue;
                    }
                    _ => continue,
                };
                if start.is_none() {
                    start = Some(to);
                    at = to;
                    continue;
                }
                let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2)).sqrt();
                if ph == Phase::Rapid {
                    lr += l;
                    time += l / 10_000.0 * 60.0;
                } else {
                    lc += l;
                    time += if f > 0.0 { l / f * 60.0 } else { 0.0 };
                }
                // a feed move straight down is a plunge; a rapid straight back up its column, from
                // where a plunge ended, retraces positions already checked: a link
                let ph = if ph == Phase::Cut && to[0] == at[0] && to[1] == at[1] && to[2] < at[2] {
                    Phase::Plunge
                } else {
                    ph
                };
                let ph = if ph == Phase::Rapid
                    && to[0] == at[0]
                    && to[1] == at[1]
                    && to[2] > at[2]
                    && matches!(
                        moves.last(),
                        Some(PMove::Line {
                            phase: Phase::Plunge,
                            ..
                        })
                    ) {
                    Phase::Link
                } else {
                    ph
                };
                // a rapid straight down within the span of this column the tool has already
                // been through at this xy (a peck cycle's re-entry): it passes only where it has
                // been, each such place checked already (by a rapid's stricter limits, or swept
                // by the cutter), so it is a link, checked as the feed there is (the same limits).
                // So the verdicts are the same for any peck retract or chip-break lift in [0, the
                // depth drilled]
                let ph = if ph == Phase::Rapid
                    && to[0] == at[0]
                    && to[1] == at[1]
                    && to[2] < at[2]
                    && column
                        .is_some_and(|(xy, low, high)| xy == [to[0], to[1]] && to[2] >= low && at[2] <= high)
                {
                    Phase::Link
                } else {
                    ph
                };
                // the span of the column through the current xy the tool has passed (reset when
                // the xy changes: the move arriving there gives its end only)
                column = if to[0] == at[0] && to[1] == at[1] {
                    match column {
                        Some((xy, l, h)) if xy == [to[0], to[1]] => {
                            Some((xy, l.min(to[2]).min(at[2]), h.max(to[2]).max(at[2])))
                        }
                        _ => Some(([to[0], to[1]], to[2].min(at[2]), to[2].max(at[2]))),
                    }
                } else {
                    Some(([to[0], to[1]], to[2], to[2]))
                };
                moves.push(if ph == Phase::Rapid {
                    PMove::Rapid { to }
                } else {
                    PMove::Line { to, f, phase: ph }
                });
                at = to;
            }
            let posted = OpPath {
                tool: path.tool,
                start: start.unwrap_or(path.start),
                moves,
                rpm: path.rpm,
                coolant: path.coolant,
            };
            // the lowest tip: moves are lines, arcs and helices, their heights linear between
            // their ends, so the least end is the least height (cycles are read back as lines)
            let mut low = posted.start[2];
            for m in &posted.moves {
                if let PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } = *m {
                    low = low.min(to[2]);
                }
            }
            let mut sum =
                self.certify_posted(oi, &posted, FULL_STOCK.load(std::sync::atomic::Ordering::Relaxed));
            // G5: every position the posted path reaches, as the machine's joints see it
            if let Some(c) = self.axis_limits(oi, &posted) {
                sum.clear = false;
                sum.checks.insert(0, c);
            }
            if let Some(c) = runtime {
                sum.clear = false;
                sum.checks.insert(0, c);
            }
            // every check names this path (a hole plan posts one per tool)
            for c in &mut sum.checks {
                c.path = pi;
            }
            // the lowest the tool's tip goes in a move that met the fixture
            let ends: Vec<[f64; 3]> = std::iter::once(posted.start)
                .chain(posted.moves.iter().scan(posted.start, |at, m| {
                    match *m {
                        PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => *at = to,
                        PMove::Cycle { .. } => {}
                    }
                    Some(*at)
                }))
                .collect();
            let low_fix = sum
                .checks
                .iter()
                .filter(|c| c.rule == "fixture_collision")
                .filter_map(|c| Some(ends.get(c.move_index)?[2].min(ends.get(c.move_index + 1)?[2])))
                .fold(f64::INFINITY, f64::min);
            let op = &mut self.ops[oi];
            if op.posted.len() <= pi {
                op.posted.resize(pi + 1, posted.clone());
            }
            op.posted[pi] = posted.clone();
            op.deepest = op.deepest.min(low);
            op.moves += posted.moves.len();
            op.pairs += sum.pairs;
            op.cells += sum.cells;
            op.least = op.least.min(sum.least);
            op.length_cut += lc;
            op.length_rapid += lr;
            op.time_s += time;
            op.checks.extend(sum.checks);
            if !sum.clear && op.status == "built" {
                op.status = "refused";
                let c = &op.checks[0];
                op.refusal = Some(format!(
                    "{} {} at move {} ({}): {} vs {} on {}",
                    c.rule, c.verdict, c.move_index, c.phase, c.value, c.limit, c.other
                ));
                op.fix = Some(match c.rule {
                    "machine_limit" => "a tool or feeds the machine can run (its spindle range and feed limit are in the machine's record), or another machine".into(),
                    "axis_limit" => "SetArg at: a work offset that keeps the program within travel, or the part placed elsewhere on the table (or a shorter tool for Z)".into(),
                    "fixture_collision" if s.fixture.is_none() => format!(
                        "the path reaches below the stock's bottom (z = {:.2}), where whatever holds it is: cut no lower than the stock, or SetArg stock: a box whose bottom is below the cut (the rest left to a second setup), or declare the workholding (SetArg fixture)",
                        s.stock.min[2]
                    ),
                    "fixture_collision" => {
                        let f = s.fixture.as_ref().expect("a fixture check has a fixture");
                        let seat = s.stock.min[2];
                        let clamp = f.jaw_top - seat;
                        let fit = low_fix - f.clearance - seat;
                        if low_fix.is_finite() && low_fix < f.jaw_top + f.clearance && fit > 0.0 {
                            format!(
                                "SetArg clamp: grip at most {fit:.2} mm (now {clamp:.2}): raise the stock in the vise; or leave below z = {:.2} to a second setup; or SetArg leave",
                                f.jaw_top + f.clearance
                            )
                        } else {
                            format!(
                                "the cut reaches the jaws (top z = {:.2}): split the setup (machine this in a second setup, the part turned over), or SetArg clear: a higher clearance height for the rapids",
                                f.jaw_top
                            )
                        }
                    }
                    "holder_collision" if c.other.starts_with("stock") => "SetArg tool: a tool whose flutes reach the depth (lcf above the stock top), or clear the stock above first".into(),
                    "holder_collision" => "SetArg stickout: a longer stick-out, or a smaller holder".into(),
                    "rapid_clear" => "SetArg clear: a higher clearance height".into(),
                    _ => "SetArg leave: leave stock, or choose a smaller tool".into(),
                });
            }
        }
        if self
            .ops
            .iter()
            .any(|o| setups.contains(&o.setup) && o.status != "built")
        {
            po.error = Some("an operation was refused: the program is not released".into());
            po.text = None;
        }
        po
    }

    /// Whether a program's setups are all placed (`at:`), so G5 checked its positions.
    fn placed(&self, p: &ProgramOut) -> bool {
        p.setups.iter().all(|&i| self.setups[i].at.is_some())
    }

    /// "full": every check proven, G5 on every position, G4's leftover computed; "except G5":
    /// released, but a posted setup is not placed, so G5 checked spans only; "full except G4
    /// (leftover not computed)" / "except G4 and G5 (..)": released, but the leftover could not
    /// be computed (a part outside the 2.5D section, as 3-axis surfacing's), so G4 is not
    /// claimed; "refused": not released.
    pub fn certification(&self) -> &'static str {
        let placed = self.programs.iter().all(|p| self.placed(p));
        if !self.ok {
            "refused"
        } else if self.leftover_done {
            if placed { "full" } else { "except G5" }
        } else if placed {
            "full except G4 (leftover not computed)"
        } else {
            "except G4 and G5 (leftover not computed; no `at:`)"
        }
    }

    /// The reply, `forgecad.cam/1`.
    pub fn json(&self) -> Json {
        let n = |x: f64| Json::Num(x);
        let v3 = |p: [f64; 3]| Json::Arr(vec![n(p[0]), n(p[1]), n(p[2])]);
        let setups = self
            .setups
            .iter()
            .map(|s| {
                Json::obj(vec![
                    ("name", Json::Str(s.name.clone())),
                    ("part", Json::Str(self.parts[s.part].file.clone())),
                    ("machine", Json::Str(s.machine.clone())),
                    (
                        "wcs",
                        Json::obj(vec![
                            ("origin", v3(s.origin)),
                            ("axes", Json::Str("part".into())),
                            ("offset", n(s.wcs as f64)),
                        ]),
                    ),
                    (
                        "stock",
                        Json::obj(vec![
                            ("kind", Json::Str(s.stock_kind.into())),
                            ("min", v3(s.stock.min)),
                            ("max", v3(s.stock.max)),
                        ]),
                    ),
                    ("tol", n(s.tol)),
                    ("clearance", n(s.clearance)),
                    ("at", s.at.map(v3).unwrap_or(Json::Null)),
                    (
                        "fixture",
                        match &s.fixture {
                            None => Json::Null,
                            Some(f) => Json::obj(vec![
                                ("kind", Json::Str("vise".into())),
                                ("name", Json::Str(f.name.clone())),
                                ("across", Json::Str(if f.axis == 0 { "x" } else { "y" }.into())),
                                ("jaw_top", n(f.jaw_top)),
                                ("clamp", n(f.jaw_top - s.stock.min[2])),
                                ("clearance", n(f.clearance)),
                                (
                                    "pieces",
                                    Json::Arr(
                                        f.boxes()
                                            .into_iter()
                                            .map(|(k, lo, hi)| {
                                                Json::obj(vec![
                                                    ("name", Json::Str(k)),
                                                    ("min", v3(lo)),
                                                    ("max", v3(hi)),
                                                ])
                                            })
                                            .collect(),
                                    ),
                                ),
                            ]),
                        },
                    ),
                    (
                        "heights",
                        Json::obj(vec![
                            ("clear", n(s.heights.clear)),
                            ("retract", n(s.heights.retract)),
                        ]),
                    ),
                ])
            })
            .collect();
        let ops = self
            .ops
            .iter()
            .map(|o| {
                let tool = o.tool.map(|i| {
                    let t = &self.tools[i];
                    Json::obj(vec![
                        ("id", Json::Str(t.tool.id.clone())),
                        ("T", n(t.t as f64)),
                        ("H", n(t.t as f64)),
                        ("type", Json::Str(t.tool.kind.name().into())),
                        ("d", n(t.tool.dc)),
                        ("holder", t.holder.clone().map(Json::Str).unwrap_or(Json::Null)),
                        ("stickout", n(t.stickout)),
                    ])
                });
                let feeds = o.feeds.map(|f| {
                    Json::obj(vec![
                        ("rpm", n(f.rpm)),
                        ("vc", n(f.vc)),
                        ("fz", n(f.fz)),
                        ("hex", n(f.hex)),
                        ("hm", n(f.hm)),
                        ("f", n(f.vf)),
                        ("power_kw", n(f.power)),
                        ("torque_nm", n(f.torque)),
                    ])
                });
                let checks: Vec<Json> = o
                    .checks
                    .iter()
                    .map(|c| {
                        Json::obj(vec![
                            ("rule", Json::Str(c.rule.into())),
                            ("verdict", Json::Str(c.verdict.into())),
                            ("certified", true.into()),
                            ("value", n(c.value)),
                            ("limit", n(c.limit)),
                            ("unit", Json::Str("mm".into())),
                            ("at", v3(c.at)),
                            ("tool_part", Json::Str(c.tool_part.into())),
                            ("other", Json::Str(c.other.clone())),
                            ("phase", Json::Str(c.phase.into())),
                            ("move", n(c.move_index as f64)),
                            ("path", n(c.path as f64)),
                        ])
                    })
                    .collect();
                let summary = Json::obj(vec![
                    ("rule", Json::Str("gouge, holder_collision, rapid_clear".into())),
                    (
                        "verdict",
                        Json::Str(if o.status == "built" { "pass" } else { "fail" }.into()),
                    ),
                    ("certified", true.into()),
                    ("pairs", n(o.pairs as f64)),
                    ("cells", n(o.cells as f64)),
                    (
                        "least_cutter_distance",
                        if o.least.is_finite() {
                            n(o.least)
                        } else {
                            Json::Null
                        },
                    ),
                    (
                        "against",
                        Json::Str(
                            "the path the posted text means (round trip), not the unrounded one".into(),
                        ),
                    ),
                ]);
                Json::obj(vec![
                    ("name", Json::Str(o.name.clone())),
                    ("kind", Json::Str(o.kind.into())),
                    ("setup", Json::Str(self.setups[o.setup].name.clone())),
                    ("status", Json::Str(o.status.into())),
                    ("tool", tool.unwrap_or(Json::Null)),
                    (
                        "plan",
                        Json::Arr(o.plan.iter().map(|s| Json::Str(s.clone())).collect()),
                    ),
                    (
                        "removals",
                        Json::Arr(o.removals.iter().map(|s| Json::Str(s.clone())).collect()),
                    ),
                    ("feeds", feeds.unwrap_or(Json::Null)),
                    (
                        "path",
                        Json::obj(vec![
                            ("moves", n(o.moves as f64)),
                            ("length_cut", n(o.length_cut)),
                            ("length_rapid", n(o.length_rapid)),
                            ("time_s", n(o.time_s)),
                        ]),
                    ),
                    ("certificate", summary),
                    ("checks", Json::Arr(checks)),
                    (
                        "leftover",
                        Json::obj(vec![(
                            "status",
                            Json::Str("not computed in this phase (G4 pending)".into()),
                        )]),
                    ),
                    ("refusal", o.refusal.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        let blocking: Vec<Json> = self
            .ops
            .iter()
            .filter(|o| o.status != "built")
            .map(|o| {
                Json::obj(vec![
                    ("op", Json::Str(o.name.clone())),
                    ("why", Json::Str(o.refusal.clone().unwrap_or_default())),
                ])
            })
            .collect();
        let fixes: Vec<Json> = self
            .ops
            .iter()
            .filter_map(|o| {
                o.fix.as_ref().map(|f| {
                    Json::obj(vec![
                        ("op", Json::Str(o.name.clone())),
                        ("fix", Json::Str(f.clone())),
                    ])
                })
            })
            .collect();
        let suggestions: Vec<Json> = self
            .ops
            .iter()
            .filter(|o| o.refusal.as_ref().is_some_and(|r| r.starts_with("NeedTool")))
            .map(|o| {
                Json::obj(vec![
                    ("kind", Json::Str("NeedTool".into())),
                    ("op", Json::Str(o.name.clone())),
                    ("why", Json::Str(o.refusal.clone().unwrap())),
                ])
            })
            .collect();
        let programs = self
            .programs
            .iter()
            .map(|p| {
                Json::obj(vec![
                    ("name", Json::Str(p.name.clone())),
                    ("post", Json::Str(p.post.clone())),
                    ("lines", n(p.lines as f64)),
                    ("round_trip", p.round_trip.into()),
                    (
                        "certified",
                        Json::Str(
                            if p.error.is_some() {
                                "no"
                            } else if !self.leftover_done {
                                if self.placed(p) {
                                    "full except G4 (leftover not computed)"
                                } else {
                                    "except G4 and G5 (leftover not computed; no `at:`)"
                                }
                            } else if self.placed(p) {
                                "full"
                            } else {
                                "except G5 (unchecked: no `at:`)"
                            }
                            .into(),
                        ),
                    ),
                    ("error", p.error.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        Json::obj(vec![
            ("ok", self.ok.into()),
            ("certification", Json::Str(self.certification().into())),
            ("schema", Json::Str("forgecad.cam/1".into())),
            ("errors", Json::Arr(self.errors.iter().map(|e| Json::Str(e.clone())).collect())),
            ("warnings", Json::Arr(self.warnings.iter().map(|e| Json::Str(e.clone())).collect())),
            ("setups", Json::Arr(setups)),
            ("ops", Json::Arr(ops)),
            ("blocking", Json::Arr(blocking)),
            ("fixes", Json::Arr(fixes)),
            ("suggestions", Json::Arr(suggestions)),
            ("programs", Json::Arr(programs)),
            (
                "leftover",
                Json::obj(vec![
                    ("computed", self.leftover_done.into()),
                    ("rule", Json::Str("G4: final stock ⊆ (part ⊕ B_{leave + tol}) ∪ these regions, by z band".into())),
                    (
                        "regions",
                        Json::Arr(
                            self.leftover
                                .iter()
                                .map(|l| {
                                    Json::obj(vec![
                                        ("setup", Json::Str(self.setups[l.setup].name.clone())),
                                        ("z", Json::Arr(vec![Json::Num(l.z0), Json::Num(l.z1)])),
                                        ("boxes", Json::Arr(l.boxes.iter().map(|b| Json::Arr(b.iter().map(|x| Json::Num(*x)).collect())).collect())),
                                        ("why", Json::Str(l.why.into())),
                                        ("r", l.r.map_or(Json::Null, Json::Num)),
                                        ("below", l.below.map_or(Json::Null, Json::Num)),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ]),
            ),
            (
                "scope",
                Json::Str("Proven: the cutter, shank and holder clear the exact part (within tol for the cutter on cuts), rapids clear part and stock, and every body of the tool keeps the fixture's clearance from the fixture as modelled (a placed vise's boxes), on the posted path; every posted spindle speed and feed is within the machine's record (G5, runtime). Not claimed: cutting physics, the machine's fidelity, the true in-process stock beyond the proven-cleared voids.".into()),
            ),
        ])
    }

    /// The setup sheet, as HTML.
    pub fn sheet(&self) -> String {
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let mut h = String::from(
            "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Setup sheet</title><style>body{font-family:sans-serif;margin:2em}table{border-collapse:collapse;margin:1em 0}td,th{border:1px solid #999;padding:4px 8px;text-align:left}th{background:#eee}.refused{color:#a00}</style></head><body>\n<h1>Setup sheet</h1>\n",
        );
        for s in &self.setups {
            h.push_str(&format!(
                "<h2>Setup {}</h2>\n<p>Part <b>{}</b> on <b>{}</b>. WCS G{} at ({:.3}, {:.3}, {:.3}) of the part's frame. Stock box {:.2} × {:.2} × {:.2} mm. Tolerance {} mm; clearance {} mm; clear height {:.2}, retract {:.2} (part frame).</p>\n",
                esc(&s.name),
                esc(&self.parts[s.part].file),
                esc(&s.machine),
                54 + s.wcs,
                s.origin[0],
                s.origin[1],
                s.origin[2],
                s.stock.max[0] - s.stock.min[0],
                s.stock.max[1] - s.stock.min[1],
                s.stock.max[2] - s.stock.min[2],
                s.tol,
                s.clearance,
                s.heights.clear,
                s.heights.retract
            ));
            if let Some(f) = &s.fixture {
                h.push_str(&format!(
                    "<p>Held in the vise <b>{}</b>, jaws closed along {}, gripping {:.2} mm of the stock (jaw top z = {:.2}, part frame), on parallels; every move keeps {} mm from the vise as modelled. Measure the vise before trusting the clearance.</p>\n",
                    esc(&f.name),
                    if f.axis == 0 { "x" } else { "y" },
                    f.jaw_top - s.stock.min[2],
                    f.jaw_top,
                    f.clearance
                ));
            }
        }
        h.push_str("<h2>Tools</h2>\n<table><tr><th>T</th><th>tool</th><th>type</th><th>D</th><th>flutes</th><th>holder</th><th>stick-out</th><th>source</th></tr>\n");
        for t in &self.tools {
            h.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                t.t,
                esc(&t.tool.id),
                t.tool.kind.name(),
                t.tool.dc,
                t.tool.nof,
                esc(t.holder.as_deref().unwrap_or("-")),
                t.stickout,
                esc(&format!("{}; {}", t.tool.source, t.tool.note))
            ));
        }
        h.push_str("</table>\n<h2>Operations</h2>\n<table><tr><th>op</th><th>kind</th><th>tool</th><th>rpm</th><th>feed</th><th>moves</th><th>cut mm</th><th>time</th><th>certificate</th></tr>\n");
        for o in &self.ops {
            let t = o
                .tool
                .map(|i| format!("T{} {}", self.tools[i].t, self.tools[i].tool.id))
                .unwrap_or_default();
            h.push_str(&format!(
                "<tr class=\"{}\"><td>{}</td><td>{}{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.0}</td><td>{:.0} s</td><td>{}</td></tr>\n",
                if o.status == "built" { "" } else { "refused" },
                esc(&o.name),
                o.kind,
                if o.plan.is_empty() { String::new() } else { format!(" ({})", esc(&o.plan.join("; "))) },
                esc(&t),
                o.feeds.map(|f| format!("{:.0}", f.rpm)).unwrap_or("-".into()),
                o.feeds.map(|f| format!("{:.0}", f.vf)).unwrap_or("-".into()),
                o.moves,
                o.length_cut,
                o.time_s,
                if o.status == "built" { "pass (certified)".to_string() } else { format!("refused: {}", esc(o.refusal.as_deref().unwrap_or(""))) }
            ));
        }
        h.push_str("</table>\n<p>Feeds and speeds are starting points from generic data: verify with the tool maker. The certificate covers the geometry only.</p>\n</body></html>\n");
        h
    }

    /// The program's tools, indexed as `OpOut::tool` and `OpPath::tool` index them (read only,
    /// for measurement: verify/forge-cam-quality).
    pub fn tool_uses(&self) -> &[ToolUse] {
        &self.tools
    }

    /// Setup `i` as measurement needs it (read only): its stock box, the machine named, the
    /// part's ISO 513 material group, and the work offset if placed.
    pub fn setup_view(&self, i: usize) -> Option<SetupView<'_>> {
        let s = self.setups.get(i)?;
        Some(SetupView {
            name: &s.name,
            stock: s.stock,
            machine: &s.machine,
            group: &self.parts[s.part].group,
            at: s.at,
            clear: s.heights.clear,
            retract: s.heights.retract,
        })
    }

    /// The cutting data of a material group in the library the job was made with.
    pub fn cutting_group(&self, g: &str) -> Option<&feeds::Group> {
        self.lib.group(g)
    }

    /// The machine's rpm range as the tooling library states it.
    pub fn machine_rpm(&self, name: &str) -> Option<[f64; 2]> {
        self.lib.machine(name).ok().map(|m| m.rpm)
    }

    /// The first program's text, if released.
    pub fn program_text(&self) -> Option<(&str, &str)> {
        self.programs
            .iter()
            .find_map(|p| p.text.as_deref().map(|t| (t, p.post.as_str())))
    }
}

#[cfg(test)]
mod feed_tests {
    use super::feed_over;

    /// F exactly at a whole-number limit passes (FEEDS-SPEEDS: it was refused, the limit's
    /// scaled lower end being one ulp under); one step over fails; a fractional limit keeps the
    /// outward-rounded bound.
    #[test]
    fn a_feed_at_the_limit_passes_and_one_step_over_fails() {
        assert!(!feed_over(100_000, 1, 10_000.0));
        assert!(feed_over(100_001, 1, 10_000.0));
        assert!(!feed_over(10_000, 0, 10_000.0));
        assert!(feed_over(10_001, 0, 10_000.0));
        assert!(!feed_over(5_000_000, 3, 5_000.0));
        assert!(feed_over(37_264, 1, 3726.3));
        assert!(!feed_over(37_262, 1, 3726.3));
        assert!(feed_over(1, 1, f64::NAN));
    }
}
