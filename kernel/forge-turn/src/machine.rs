//! The lathe a program runs on, from CAM-MACHINE's machine definitions (crate forge-machine,
//! docs/cam-machine.md): a machine of kind lathe or mill-turn with its Z and X joints (X a
//! diameter: `diameter = true`, the joint is half the word), its turning spindle's rpm range
//! (rpm[1] the G50 clamp) and its turret's stations. The boundary conversion: only the fields
//! turning needs, read from forge_machine's MachineDef.
//!
//! G5 for turning: a joint's value is the imaginary tip's position plus the placement — the work
//! offset (the machine Z of the part's Z0) and the tool's geometry offsets (from touch-off). A
//! line move is within travel iff its ends are; an arc is when its circle's box is (ForgeCam
//! Machine.lean, `G5_line_iff`, `G5_arc`). An unplaced setup gets the necessary check only (the
//! tip's span along each axis within the axis's travel) and says so.
use forge_machine::def::{Kind, MachineDef, SpindleKind};
use forge_machine::library::Library;

#[derive(Clone, Debug, PartialEq)]
pub struct Lathe {
    pub name: String,
    /// The dialect its control speaks (post.rs).
    pub control: String,
    /// The X joint's travel, as a radius (the joint value).
    pub x: [f64; 2],
    pub z: [f64; 2],
    /// The turning spindle's speed range; rpm[1] is the G50 clamp ceiling.
    pub rpm: [f64; 2],
    pub stations: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MachineError {
    Def(String),
    NotFound(String),
    Missing { machine: String, what: String },
}

/// The lathe of a machine definition.
pub fn from_def(m: &MachineDef) -> Result<Lathe, MachineError> {
    let missing = |what: &str| MachineError::Missing {
        machine: m.name.clone(),
        what: what.into(),
    };
    if !matches!(m.kind, Kind::Lathe | Kind::MillTurn) {
        return Err(missing("kind = \"lathe\" or \"mill-turn\""));
    }
    let joint = |n: &str| m.joints.iter().find(|j| j.letter == n || j.name == n);
    let zj = joint("Z").ok_or_else(|| missing("joint Z"))?;
    let xj = joint("X").ok_or_else(|| missing("joint X"))?;
    if !xj.diameter {
        return Err(missing("joint X with diameter = true"));
    }
    let spindle = m
        .spindles
        .iter()
        .find(|s| s.kind == SpindleKind::Turning)
        .ok_or_else(|| missing("a turning spindle"))?;
    let stations = m
        .turrets
        .first()
        .map(|t| t.stations)
        .ok_or_else(|| missing("a turret"))?;
    Ok(Lathe {
        name: m.name.clone(),
        control: m.control.clone(),
        x: xj.range,
        z: zj.range,
        rpm: spindle.rpm,
        stations,
    })
}

/// The lathe named `name` in a machine-definition TOML text.
pub fn lathe(src: &str, name: &str) -> Result<Lathe, MachineError> {
    let defs = MachineDef::parse_all(src).map_err(|e| MachineError::Def(format!("{e:?}")))?;
    let m = defs
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| MachineError::NotFound(name.into()))?;
    from_def(m)
}

/// A lathe of forge-machine's built-in library by name.
pub fn builtin(name: &str) -> Result<Lathe, MachineError> {
    let lib = Library::with_builtin();
    let m = lib.get(name).map_err(|e| MachineError::NotFound(e.to_string()))?;
    from_def(m)
}

/// Where the work and the tools sit on the machine: the machine Z of the part's Z0, and each
/// station's geometry offsets [x (radius), z] (joint = tip + offset).
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub z0: f64,
    pub tools: Vec<(u32, [f64; 2])>,
}
