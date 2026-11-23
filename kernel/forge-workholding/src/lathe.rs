//! Lathe workholding as envelopes of revolution: each body is a polygon in (ρ, z) (ρ ≥ 0 the
//! radius, z along the spindle, the part's free end toward +z) that the body sweeps as the
//! spindle turns. The jaws' face is at `z_face`; the bar is gripped behind it.
//!
//! What forge-turn takes: `to_region` gives each polygon as a forge-cam-geom `Region` (x = ρ,
//! y = z), the form its chuck obstacle already has.
use crate::{Kind, Record, Role, WhError};
use forge_cam_geom::region::{Loop, Region, Seg, p2};

fn refuse(device: &str, what: impl Into<String>, fix: impl Into<String>) -> WhError {
    WhError::Placement {
        device: device.to_string(),
        what: what.into(),
        fix: fix.into(),
    }
}

fn want(r: &Record, k: Kind) -> Result<(), WhError> {
    if r.kind == k {
        Ok(())
    } else {
        Err(refuse(
            &r.name,
            format!("is a {}", r.kind.name()),
            format!("name a {}", k.name()),
        ))
    }
}

/// A body of revolution: a counter-clockwise polygon in (ρ, z).
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    pub name: String,
    pub role: Role,
    pub poly: Vec<[f64; 2]>,
}

fn rect(name: impl Into<String>, role: Role, r0: f64, r1: f64, z0: f64, z1: f64) -> Ring {
    Ring {
        name: name.into(),
        role,
        poly: vec![[r0, z0], [r1, z0], [r1, z1], [r0, z1]],
    }
}

/// A lathe device placed on the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Spin {
    pub name: String,
    pub kind: Kind,
    pub rings: Vec<Ring>,
    /// The bar is held for z in this range (ρ up to the bar's radius): no op may reach it.
    pub held: Option<[f64; 2]>,
    pub notes: Vec<String>,
}

/// A chuck gripping a bar of diameter `bar_d` with its jaws' face at `z_face`, `grip` of the
/// bar inside the jaws (at most the jaws' length).
pub fn chuck(name: &str, rec: &Record, bar_d: f64, z_face: f64, grip: f64) -> Result<Spin, WhError> {
    want(rec, Kind::Chuck)?;
    let (bd, bl) = (rec.get("body_d")?, rec.get("body_len")?);
    let (jh, jl) = (rec.get("jaw_height")?, rec.get("jaw_len")?);
    let (gmin, gmax) = (rec.get("grip_min")?, rec.get("grip_max")?);
    if !(bar_d >= gmin && bar_d <= gmax) {
        return Err(refuse(
            name,
            format!("a bar {bar_d:.2} mm across; the chuck grips {gmin:.2}..{gmax:.2} mm"),
            "other jaws (reversed, or soft jaws bored to the bar), or another chuck",
        ));
    }
    if !(grip > 0.0 && grip <= jl) {
        return Err(refuse(
            name,
            format!("grip length {grip:.2} mm: it must be > 0 and at most the jaws' {jl:.2} mm"),
            "SetArg grip",
        ));
    }
    let r = 0.5 * bar_d;
    let zb = z_face - jl;
    let rings = vec![
        rect("body", Role::ChuckBody, 0.0, 0.5 * bd, zb - bl, zb),
        rect("jaws", Role::Jaw, r, r + jh, zb, z_face),
    ];
    let mut notes = vec![format!(
        "chuck {} ({} jaws): bar {bar_d:.2} gripped {grip:.2} mm behind z = {z_face:.2}; the jaws sweep to ρ = {:.2}",
        rec.name,
        rec.get("jaws")?,
        r + jh
    )];
    if r + jh > 0.5 * bd {
        notes.push(format!(
            "the jaws stand out {:.2} mm past the chuck body",
            r + jh - 0.5 * bd
        ));
    }
    Ok(Spin {
        name: name.to_string(),
        kind: Kind::Chuck,
        rings,
        held: Some([z_face - grip, z_face]),
        notes,
    })
}

/// A collet chuck: its nose ends at `z_face`; the bar within the collet's range.
pub fn collet(name: &str, rec: &Record, bar_d: f64, z_face: f64) -> Result<Spin, WhError> {
    want(rec, Kind::Collet)?;
    let (lo, hi) = (rec.get("range_min")?, rec.get("range_max")?);
    let (nd, nl) = (rec.get("nose_d")?, rec.get("nose_len")?);
    if !(bar_d >= lo && bar_d <= hi) {
        return Err(refuse(
            name,
            format!("a bar {bar_d:.2} mm across; the collet closes on {lo:.2}..{hi:.2} mm"),
            "the collet of that size",
        ));
    }
    Ok(Spin {
        name: name.to_string(),
        kind: Kind::Collet,
        rings: vec![rect("nose", Role::Collet, 0.0, 0.5 * nd, z_face - nl, z_face)],
        held: Some([z_face - rec.opt("collet_len").unwrap_or(0.0), z_face]),
        notes: vec![format!(
            "collet {}: bar {bar_d:.2}, nose ⌀{nd} ending at z = {z_face:.2}",
            rec.name
        )],
    })
}

/// A face driver: its pins bite the part's end face at `z_face`; it needs a tailstock centre.
pub fn face_driver(name: &str, rec: &Record, work_d: f64, z_face: f64) -> Result<Spin, WhError> {
    want(rec, Kind::FaceDriver)?;
    let (bd, bl) = (rec.get("body_d")?, rec.get("body_len")?);
    let (lo, hi) = (rec.get("range_min")?, rec.get("range_max")?);
    if !(work_d >= lo && work_d <= hi) {
        return Err(refuse(
            name,
            format!("a part {work_d:.2} mm across; the driver takes {lo:.2}..{hi:.2} mm"),
            "another driving head",
        ));
    }
    Ok(Spin {
        name: name.to_string(),
        kind: Kind::FaceDriver,
        rings: vec![rect(
            "driver",
            Role::ChuckBody,
            0.0,
            0.5 * bd,
            z_face - bl,
            z_face,
        )],
        held: None,
        notes: vec![format!(
            "face driver {}: the part is driven on its face at z = {z_face:.2}; a tailstock centre holds the other end",
            rec.name
        )],
    })
}

/// A tailstock centre in the part's centre hole at its free end `z_end` (the hole's radius at
/// the face `hole_r`): the cone, then the body to `back` behind the point.
pub fn centre(name: &str, rec: &Record, z_end: f64, hole_r: f64, back: f64) -> Result<Spin, WhError> {
    want(rec, Kind::Centre)?;
    let (ang, bd, pl) = (rec.get("angle")?, rec.get("body_d")?, rec.get("point_len")?);
    let half = 0.5 * ang.to_radians();
    let (s, c) = forge_math::dmath::sin_cos(half);
    let slope = s / c;
    let rb = 0.5 * bd;
    // the cone from the hole's radius at the face, widening by tan(half) per mm, to the body
    let rp = (hole_r + pl * slope).min(rb);
    let zc = z_end + pl;
    let ring = Ring {
        name: "centre".into(),
        role: Role::Centre,
        poly: vec![
            [0.0, z_end],
            [hole_r, z_end],
            [rp, zc],
            [rb, zc],
            [rb, zc + back],
            [0.0, zc + back],
        ],
    };
    Ok(Spin {
        name: name.to_string(),
        kind: Kind::Centre,
        rings: vec![ring],
        held: None,
        notes: vec![format!(
            "centre {} ({ang}°) in the end at z = {z_end:.2}",
            rec.name
        )],
    })
}

/// A steady rest at `z` on a bar of diameter `bar_d`.
pub fn steady_rest(name: &str, rec: &Record, bar_d: f64, z: f64) -> Result<Spin, WhError> {
    want(rec, Kind::SteadyRest)?;
    let (lo, hi) = (rec.get("range_min")?, rec.get("range_max")?);
    let (fr, w) = (rec.get("frame_r")?, rec.get("width")?);
    if !(bar_d >= lo && bar_d <= hi) {
        return Err(refuse(
            name,
            format!("a bar {bar_d:.2}; the rest takes {lo:.2}..{hi:.2} mm"),
            "another rest",
        ));
    }
    Ok(Spin {
        name: name.to_string(),
        kind: Kind::SteadyRest,
        rings: vec![rect(
            "rest",
            Role::Rest,
            0.5 * bar_d,
            fr,
            z - 0.5 * w,
            z + 0.5 * w,
        )],
        held: Some([z - 0.5 * w, z + 0.5 * w]),
        notes: vec![format!(
            "steady rest {} at z = {z:.2}: no tool passes it at ρ > {:.2}",
            rec.name,
            0.5 * bar_d
        )],
    })
}

/// Distance from a point to a polygon's boundary, negative inside.
fn signed_dist(poly: &[[f64; 2]], p: [f64; 2]) -> f64 {
    let n = poly.len();
    let mut d = f64::INFINITY;
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        let l2 = ex * ex + ey * ey;
        let t = if l2 > 0.0 {
            (((p[0] - a[0]) * ex + (p[1] - a[1]) * ey) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (qx, qy) = (a[0] + t * ex - p[0], a[1] + t * ey - p[1]);
        d = d.min((qx * qx + qy * qy).sqrt());
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * ex {
            inside = !inside;
        }
    }
    if inside { -d } else { d }
}

/// What a lathe path met: the least gap, and the first ring it came within `clearance` of.
#[derive(Clone, Debug, PartialEq)]
pub struct LatheCheck {
    pub least: f64,
    pub hit: Option<(String, [f64; 2])>,
}

/// Checks a path of tool-nose centres (ρ, z) of nose radius `nose_r` against every ring of
/// every device: each point's gap to a ring, less the nose radius, must be at least
/// `clearance`. A straight move between points is checked by points at most `step` apart (a
/// sampled check, for the report: the certified one is forge-turn's).
pub fn check_path(devices: &[Spin], path: &[[f64; 2]], nose_r: f64, clearance: f64, step: f64) -> LatheCheck {
    let mut out = LatheCheck {
        least: f64::INFINITY,
        hit: None,
    };
    let mut pts = Vec::new();
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = (len / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            pts.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    if let Some(l) = path.last() {
        pts.push(*l);
    }
    for p in pts {
        for d in devices {
            for r in &d.rings {
                let g = signed_dist(&r.poly, p) - nose_r;
                if g < out.least {
                    out.least = g;
                }
                if g < clearance && out.hit.is_none() {
                    out.hit = Some((format!("{}/{}", d.name, r.name), p));
                }
            }
        }
    }
    out
}

/// The stick-out a path needs: the bar must come out of the jaws far enough that its deepest
/// point toward the chuck (`z_min` of the nose centres, at radius above the jaws' bore) keeps
/// the nose radius and the clearance from the jaws' face. Returns (needed, have): the length
/// out of the jaws the path needs, and what the bar has (z_front − z_face).
pub fn stick_out(z_face: f64, z_front: f64, z_min: f64, nose_r: f64, clearance: f64) -> (f64, f64) {
    (z_front - z_min + nose_r + clearance, z_front - z_face)
}

/// The overhang guideline, stated (a machine maker's, not a standard; no chuck maker was found
/// publishing a number): "Workpieces longer than 3 times the chucking diameter must be supported
/// by the tailstock or a steady rest." (JET GH-1340 manual M-JT1-2708, safety item 33,
/// https://www.trick-tools.com/common/documentation/gh1340_manual.pdf, read 2026-11-23). A note
/// when the length out of the jaws exceeds 3 × the gripped diameter with nothing at the far end;
/// never a refusal.
pub fn overhang(len_out: f64, grip_d: f64, supported: bool) -> Option<String> {
    (!supported && len_out > 3.0 * grip_d).then(|| {
        format!(
            "{len_out:.1} mm out of the jaws is {:.2} × the gripped ⌀{grip_d:.1}: over the 3 × guideline (JET GH-1340 manual): a tailstock centre or a steady rest",
            len_out / grip_d
        )
    })
}

/// A ring as a forge-cam-geom region (x = ρ, y = z).
pub fn to_region(r: &Ring) -> Region {
    let n = r.poly.len();
    Region {
        loops: vec![Loop {
            segs: (0..n)
                .map(|i| Seg::Line {
                    a: p2(r.poly[i][0], r.poly[i][1]),
                    b: p2(r.poly[(i + 1) % n][0], r.poly[(i + 1) % n][1]),
                })
                .collect(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_distance() {
        let sq = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        assert_eq!(signed_dist(&sq, [1.0, 1.0]), -1.0);
        assert_eq!(signed_dist(&sq, [5.0, 1.0]), 3.0);
    }
}
