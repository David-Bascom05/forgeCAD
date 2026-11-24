//! One machine list for machining documents. A machine forge-machine defines (its built-in
//! records: the generics, Haas, the vendors' machines, each value cited there) is the source for
//! everything its record carries: kind, control, coolants, the linear joints' directions, travels
//! and rapid rates, the feed limit, the spindle's speed range and power curve. A forge-tooling
//! record of the same name (`machines.toml`, the user's, a project's) supplies only what the
//! record lacks; where both give a value and they differ, the name is refused (`conflict`), never
//! one of them picked silently.
//!
//! The planner's joints are the tool's motion relative to the part, in program axes (X moves the
//! tool +x over the part); forge-machine's are the physical ones, so a joint on the table's side
//! (the table moves −x for +X) has its direction turned round.

use forge_machine::def::{JointType, MachineDef, Side, SpindleKind};
use forge_tooling::{Joint, Library, Machine};

/// The planner's machine from a forge-machine record, with `base` (a forge-tooling record of the
/// same name) supplying only what the record lacks. Err: the values both give that differ.
pub fn combine(d: &MachineDef, base: Option<&Machine>) -> Result<Machine, String> {
    let mut diff: Vec<String> = vec![];
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
    let mut field = |what: &str, ours: String, theirs: Option<String>, same: bool| {
        if let Some(t) = theirs
            && !same
        {
            diff.push(format!(
                "{what}: {ours} in its machine record, {t} in the tool library"
            ));
        }
    };
    let kind = d.kind.name().to_string();
    field(
        "kind",
        kind.clone(),
        base.map(|b| b.kind.clone()),
        base.is_none_or(|b| b.kind == kind),
    );
    field(
        "control",
        d.control.clone(),
        base.map(|b| b.control.clone()),
        base.is_none_or(|b| b.control == d.control),
    );
    let coolants = if d.coolants.is_empty() {
        base.map(|b| b.coolants.clone()).unwrap_or_default()
    } else {
        if let Some(b) = base.filter(|b| !b.coolants.is_empty()) {
            let (mut a, mut c) = (d.coolants.clone(), b.coolants.clone());
            a.sort();
            c.sort();
            field("coolants", a.join(", "), Some(c.join(", ")), a == c);
        }
        d.coolants.clone()
    };
    // the milling spindle (the first spindle where none mills)
    let sp = d
        .spindles
        .iter()
        .find(|s| s.kind == SpindleKind::Milling)
        .or_else(|| d.spindles.first());
    let rpm = match (sp.and_then(|s| s.rpm), base) {
        (Some(r), Some(b)) => {
            field(
                "spindle speed",
                format!("{:?}", r),
                Some(format!("{:?}", b.rpm)),
                near(r[0], b.rpm[0]) && near(r[1], b.rpm[1]),
            );
            r
        }
        (Some(r), None) => r,
        (None, Some(b)) => b.rpm,
        (None, None) => [0.0, 0.0],
    };
    let power = match (sp.map(|s| s.power.clone()).filter(|p| !p.is_empty()), base) {
        (Some(p), Some(b)) if !b.power.is_empty() => {
            let same = p.len() == b.power.len()
                && p.iter()
                    .zip(&b.power)
                    .all(|(x, y)| near(x[0], y[0]) && near(x[1], y[1]));
            field(
                "power curve",
                format!("{p:?}"),
                Some(format!("{:?}", b.power)),
                same,
            );
            p
        }
        (Some(p), _) => p,
        (None, b) => b.map(|b| b.power.clone()).unwrap_or_default(),
    };
    // the linear joints, as the tool moves over the part
    let linear: Vec<&forge_machine::def::Joint> =
        d.joints.iter().filter(|j| j.ty == JointType::Linear).collect();
    let max_feed = match (
        linear.iter().filter_map(|j| j.max_feed).reduce(f64::min),
        base.map(|b| b.max_feed).filter(|f| *f > 0.0),
    ) {
        (Some(f), Some(b)) => {
            field(
                "feed limit",
                format!("{f} mm/min"),
                Some(format!("{b} mm/min")),
                near(f, b),
            );
            f
        }
        (Some(f), None) => f,
        (None, b) => b.unwrap_or(0.0),
    };
    let mut joints = vec![];
    for j in &linear {
        let theirs = base.and_then(|b| b.joints.iter().find(|q| q.name == j.name));
        let sign = if j.side == Side::Table { -1.0 } else { 1.0 };
        let axis = j.axis.map(|a| if a == 0.0 { 0.0 } else { sign * a });
        let range = match (j.range, theirs) {
            (Some(r), Some(t)) => {
                field(
                    &format!("joint {} travel", j.name),
                    format!("{r:?}"),
                    Some(format!("{:?}", t.range)),
                    near(r[0], t.range[0]) && near(r[1], t.range[1]),
                );
                r
            }
            (Some(r), None) => r,
            (None, Some(t)) => t.range,
            // no travel given anywhere: not a joint the planner can check
            (None, None) => continue,
        };
        if let Some(t) = theirs {
            field(
                &format!("joint {} direction", j.name),
                format!("{axis:?}"),
                Some(format!("{:?}", t.axis)),
                (0..3).all(|k| near(axis[k], t.axis[k])),
            );
        }
        let rapid = match (j.rapid, theirs) {
            (Some(r), Some(t)) => {
                field(
                    &format!("joint {} rapid", j.name),
                    format!("{r} mm/min"),
                    Some(format!("{} mm/min", t.rapid)),
                    near(r, t.rapid),
                );
                r
            }
            (Some(r), None) => r,
            (None, Some(t)) => t.rapid,
            (None, None) => 0.0,
        };
        joints.push(Joint {
            name: j.name.clone(),
            axis,
            range,
            rapid,
        });
    }
    if !diff.is_empty() {
        return Err(format!(
            "machine `{}` is defined twice and the two disagree ({}); make them agree, or remove the tool library's copy (the machine record is the source)",
            d.name,
            diff.join("; ")
        ));
    }
    Ok(Machine {
        name: d.name.clone(),
        kind,
        control: d.control.clone(),
        joints,
        rpm,
        power,
        max_feed,
        coolants,
        source: d.source.clone(),
    })
}

/// forge-machine's built-in records, read once.
fn records() -> &'static [MachineDef] {
    static R: std::sync::OnceLock<Vec<MachineDef>> = std::sync::OnceLock::new();
    R.get_or_init(forge_machine::library::builtin)
}

/// The library's machines made one list: each forge-machine record in it (added, or combined
/// with the library's record of that name). A name whose two records disagree keeps the
/// library's own; `conflict` refuses it where a document names it.
pub fn merge(lib: &mut Library) {
    for d in records() {
        match lib.machines.iter().position(|m| m.name == d.name) {
            None => {
                if let Ok(m) = combine(d, None) {
                    lib.machines.push(m);
                }
            }
            Some(i) => {
                if let Ok(m) = combine(d, Some(&lib.machines[i])) {
                    lib.machines[i] = m;
                }
            }
        }
    }
}

/// Why a machine name can't be used: its record and the library's disagree. None if it can.
pub fn conflict(lib: &Library, name: &str) -> Option<String> {
    let d = records().iter().find(|d| d.name == name)?;
    let m = lib.machine(name).ok()?;
    combine(d, Some(m)).err()
}

/// The forge-machine record of a name, if it has one.
pub fn record(name: &str) -> Option<&'static MachineDef> {
    records().iter().find(|d| d.name == name)
}
