//! The lathe a program runs on, read from CAM-MACHINE's machine definition format
//! (docs/cam-machine.md on branch cam/machine, crate forge-machine): a `[[machine]]` of kind
//! `lathe` with its Z and X joints (X a diameter: `diameter = true`, the joint is half the word),
//! its turning spindle's rpm range (rpm[1] the G50 clamp) and its turret's stations. This is the
//! boundary conversion until forge-machine is on main: the same TOML, only the fields turning
//! needs.
//!
//! G5 for turning: a joint's value is the imaginary tip's position plus the placement — the work
//! offset (the machine Z of the part's Z0) and the tool's geometry offsets (from touch-off). A
//! line move is within travel iff its ends are; an arc is when its circle's box is (ForgeCam
//! Machine.lean, `G5_line_iff`, `G5_arc`). An unplaced setup gets the necessary check only (the
//! tip's span along each axis within the axis's travel) and says so.
use forge_tooling::toml::{self, Table, Val};

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
    Toml(String),
    NotFound(String),
    Missing { machine: String, what: String },
}

fn nums2(t: &Table, k: &str) -> Option<[f64; 2]> {
    toml::nums(t, k).filter(|v| v.len() == 2).map(|v| [v[0], v[1]])
}

/// The lathe named `name` in a machine-definition TOML text.
pub fn lathe(src: &str, name: &str) -> Result<Lathe, MachineError> {
    let root = toml::parse(src).map_err(|e| MachineError::Toml(format!("line {}: {}", e.line, e.what)))?;
    let m = toml::tables(&root, "machine")
        .iter()
        .find(|m| toml::text(m, "name") == Some(name))
        .ok_or_else(|| MachineError::NotFound(name.into()))?;
    let missing = |what: &str| MachineError::Missing {
        machine: name.into(),
        what: what.into(),
    };
    if toml::text(m, "kind") != Some("lathe") && toml::text(m, "kind") != Some("mill-turn") {
        return Err(missing("kind = \"lathe\" or \"mill-turn\""));
    }
    let control = toml::text(m, "control")
        .ok_or_else(|| missing("control"))?
        .to_string();
    let joint = |n: &str| {
        toml::tables(m, "joint")
            .iter()
            .find(|j| toml::text(j, "name") == Some(n))
    };
    let zj = joint("Z").ok_or_else(|| missing("joint Z"))?;
    let xj = joint("X").ok_or_else(|| missing("joint X"))?;
    if !matches!(xj.get("diameter"), Some(Val::Bool(true))) {
        return Err(missing("joint X with diameter = true"));
    }
    let spindle = toml::tables(m, "spindle")
        .iter()
        .find(|s| toml::text(s, "kind") == Some("turning"))
        .ok_or_else(|| missing("a turning spindle"))?;
    let stations = toml::tables(m, "turret")
        .first()
        .and_then(|t| toml::num(t, "stations"))
        .ok_or_else(|| missing("a turret with stations"))? as u32;
    Ok(Lathe {
        name: name.into(),
        control,
        x: nums2(xj, "range").ok_or_else(|| missing("joint X range"))?,
        z: nums2(zj, "range").ok_or_else(|| missing("joint Z range"))?,
        rpm: nums2(spindle, "rpm").ok_or_else(|| missing("spindle rpm"))?,
        stations,
    })
}

/// Where the work and the tools sit on the machine: the machine Z of the part's Z0, and each
/// station's geometry offsets [x (radius), z] (joint = tip + offset).
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub z0: f64,
    pub tools: Vec<(u32, [f64; 2])>,
}
