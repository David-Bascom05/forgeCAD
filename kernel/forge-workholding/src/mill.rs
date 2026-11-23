//! Milling workholding placed on the stock (part frame, z up, the tool coming down −z).
//!
//! Each function sizes a device from its record and the stock and returns its bodies, how it
//! mounts, what it holds, and notes; a stock it cannot hold as asked is a typed `Placement`
//! error with the fix. Bodies are unions of blocks and cylinders that contain the device (a
//! dovetail's angled jaws inside their block, a tilted strap inside the block of its two rests):
//! a superset, the safe side for the clearance check.
use crate::geom::{IDENT, Pose, Prim, V3, xy_dist};
use crate::{Body, Box3, Kind, Mount, Placed, Record, Role, WhError};

fn refuse(device: &str, what: impl Into<String>, fix: impl Into<String>) -> WhError {
    WhError::Placement {
        device: device.to_string(),
        what: what.into(),
        fix: fix.into(),
    }
}

fn want(r: &Record, kinds: &[Kind]) -> Result<(), WhError> {
    if kinds.contains(&r.kind) {
        Ok(())
    } else {
        Err(refuse(
            &r.name,
            format!("is a {}", r.kind.name()),
            format!(
                "name a {}",
                kinds.iter().map(|k| k.name()).collect::<Vec<_>>().join(" or ")
            ),
        ))
    }
}

/// An axis-aligned box from its extent along the clamp axis `a`, across it (`o`), and in z.
fn bx(a: usize, along: (f64, f64), across: (f64, f64), z: (f64, f64)) -> Prim {
    let o = 1 - a;
    let mut lo = [0.0; 3];
    let mut hi = [0.0; 3];
    lo[a] = along.0;
    hi[a] = along.1;
    lo[o] = across.0;
    hi[o] = across.1;
    lo[2] = z.0;
    hi[2] = z.1;
    Prim::aabb(lo, hi)
}

/// A machine vise (or a self-centring one) closed on the stock across `axis` (0: x, 1: y),
/// gripping `clamp` of it; with parallels under the stock when the jaws are deeper than the
/// grip. `jaws`: soft-jaw blanks bolted on the hard jaws (their thickness narrows the opening).
pub fn vise(
    name: &str,
    rec: &Record,
    stock: &Box3,
    axis: usize,
    clamp: f64,
    parallels: Option<&Record>,
    jaws: Option<&Record>,
) -> Result<Placed, WhError> {
    want(rec, &[Kind::Vise, Kind::SelfCentring])?;
    if axis > 1 {
        return Err(refuse(name, "the jaws close along x or y", "across: \"x\" or \"y\""));
    }
    let (a, o) = (axis, 1 - axis);
    let jw = rec.get("jaw_width")?;
    let jh = rec.get("jaw_height")?;
    let t = rec.get("jaw_thickness")?;
    let max = rec.get("max_opening")?;
    let bl = rec.get("body_length")?;
    let bw = rec.get("body_width")?;
    let height = rec.get("height")?;
    if height < jh {
        return Err(refuse(
            &rec.name,
            format!("its height {height} is less than its jaws' {jh}"),
            "correct the record",
        ));
    }
    let size = stock.size();
    let soft = match jaws {
        Some(j) => {
            want(j, &[Kind::SoftJaws])?;
            Some((j.get("thickness")?, j.get("height")?, j.get("width")?))
        }
        None => None,
    };
    let tb = soft.map_or(0.0, |s| s.0);
    let opening = size[a] + 2.0 * tb;
    if opening > max {
        return Err(refuse(
            name,
            format!(
                "the stock is {:.2} mm across the jaws{}; the vise opens {max:.2} mm",
                size[a],
                if tb > 0.0 { format!(" (+{:.2} of soft jaws)", 2.0 * tb) } else { String::new() }
            ),
            "turn the stock, or a bigger vise",
        ));
    }
    if let Some(m) = rec.opt("min_opening").filter(|m| opening < *m) {
        return Err(refuse(
            name,
            format!("the stock is {opening:.2} mm across the jaws; the vise closes to {m:.2} mm"),
            "reverse the jaws, or a smaller vise",
        ));
    }
    let gh = soft.map_or(jh, |s| s.1);
    let cmax = size[2].min(gh);
    if !(clamp > 0.0 && clamp <= cmax) {
        return Err(refuse(
            name,
            format!("clamp depth {clamp:.2} mm: it must be > 0 and at most {cmax:.2} mm"),
            "SetArg clamp",
        ));
    }
    let seat = stock.min[2];
    let top = seat + clamp;
    // soft jaws stand on the bed beside the hard jaws: the bed is the grip's height below the
    // top, the hard jaws rise their own height from it
    let bed = top - gh;
    let htop = bed + jh;
    let table = htop - height;
    let (s0, s1) = (stock.min[a], stock.max[a]);
    let mid_o = 0.5 * (stock.min[o] + stock.max[o]);
    let mid_a = 0.5 * (s0 + s1);
    let acr = |w: f64| (mid_o - 0.5 * w, mid_o + 0.5 * w);
    let centred = rec.kind == Kind::SelfCentring;
    let base_along = if centred {
        (mid_a - 0.5 * bl, mid_a + 0.5 * bl)
    } else {
        (s0 - tb - t, s0 - tb - t + bl)
    };
    let mut bodies = vec![
        Body::new("base", Role::Base, vec![bx(a, base_along, acr(bw), (table, bed))]),
        Body::new(
            "fixed jaw",
            if centred { Role::Jaw } else { Role::FixedJaw },
            vec![bx(a, (s0 - tb - t, s0 - tb), acr(jw), (bed, htop))],
        ),
        Body::new(
            "moving jaw",
            if centred { Role::Jaw } else { Role::MovingJaw },
            vec![bx(a, (s1 + tb, s1 + tb + t), acr(jw), (bed, htop))],
        ),
    ];
    let mut notes = vec![format!(
        "{} {}: jaws closed along {} on {:.2} mm, gripping {clamp:.2} mm (jaw top z = {top:.2}); its base on the table at z = {table:.2}",
        rec.kind.name(),
        rec.name,
        ["x", "y"][a],
        size[a]
    )];
    if let Some((tb, _, sw)) = soft {
        let j = jaws.expect("soft is from jaws");
        bodies.push(Body::new(
            format!("soft jaw ({})", j.name),
            Role::Jaw,
            vec![bx(a, (s0 - tb, s0), acr(sw), (bed, top))],
        ));
        bodies.push(Body::new(
            format!("soft jaw ({})", j.name),
            Role::Jaw,
            vec![bx(a, (s1, s1 + tb), acr(sw), (bed, top))],
        ));
        notes.push(format!(
            "soft jaws {}: {tb:.2} mm each, uncut (the soft_jaws op cuts their pocket)",
            j.name
        ));
    }
    let need = seat - bed;
    if need > 1e-9 {
        let Some(p) = parallels else {
            return Err(refuse(
                name,
                format!("the stock's seat is {need:.2} mm above the vise bed"),
                format!("parallels {need:.2} mm high, or clamp {gh:.2} mm"),
            ));
        };
        want(p, &[Kind::Parallels])?;
        let hs = p.list("heights")?;
        if !hs.iter().any(|h| (h - need).abs() <= 1e-6) {
            let mut fix: Vec<String> = hs
                .iter()
                .filter(|h| **h < gh && gh - **h <= cmax)
                .map(|h| format!("clamp {:.2}", gh - h))
                .collect();
            fix.truncate(4);
            return Err(refuse(
                name,
                format!(
                    "parallels {need:.2} mm high are needed; {} has {}",
                    p.name,
                    hs.iter().map(|h| format!("{h}")).collect::<Vec<_>>().join(", ")
                ),
                if fix.is_empty() { "other parallels".to_string() } else { fix.join(" or ") },
            ));
        }
        let pw = p.get("width")?;
        let pl = p.get("length")?.min(jw);
        if 2.0 * pw >= size[a] {
            return Err(refuse(
                name,
                format!(
                    "two parallels {pw:.2} mm wide do not fit under {:.2} mm of stock",
                    size[a]
                ),
                "narrower parallels",
            ));
        }
        for (k, along) in [(s0, s0 + pw), (s1 - pw, s1)].into_iter().enumerate() {
            bodies.push(Body::new(
                format!("parallel {}", k + 1),
                Role::Parallel,
                vec![bx(a, along, acr(pl), (bed, seat))],
            ));
        }
        notes.push(format!("parallels {} at {need:.2} mm", p.name));
    }
    Ok(Placed {
        name: name.to_string(),
        kind: rec.kind,
        bodies,
        mount: Mount::Table { z: table },
        held: vec![(stock.min, [stock.max[0], stock.max[1], top])],
        notes,
    })
}

/// A dovetail fixture (5-axis): the stock's underside carries a dovetail tenon `tenon` wide
/// across `axis`, `grip` deep (the record's); the fixture is narrow so every side is open.
pub fn dovetail(name: &str, rec: &Record, stock: &Box3, axis: usize, tenon: f64) -> Result<Placed, WhError> {
    want(rec, &[Kind::Dovetail])?;
    let (a, o) = (axis.min(1), 1 - axis.min(1));
    let (bl, bw, height) = (rec.get("body_length")?, rec.get("body_width")?, rec.get("height")?);
    let (grip, angle) = (rec.get("grip")?, rec.get("angle")?);
    let (wmin, wmax) = (rec.get("min_width")?, rec.get("max_width")?);
    if !(tenon >= wmin && tenon <= wmax) {
        return Err(refuse(
            name,
            format!("a tenon {tenon:.2} mm wide; the fixture takes {wmin:.2}..{wmax:.2} mm"),
            "another fixture, or a tenon in its range",
        ));
    }
    let size = stock.size();
    if tenon > size[a] || tenon > bl {
        return Err(refuse(
            name,
            format!("a tenon {tenon:.2} mm wide on stock {:.2} mm, a fixture {bl:.2} mm long", size[a]),
            "a narrower tenon",
        ));
    }
    if grip >= size[2] {
        return Err(refuse(name, format!("the grip {grip:.2} mm is the stock's whole height"), "taller stock"));
    }
    let seat = stock.min[2];
    let top = seat + grip;
    let table = top - height;
    let m = stock.mid();
    let (ma, mo) = (m[a], m[o]);
    let acr = (mo - 0.5 * bw, mo + 0.5 * bw);
    let bodies = vec![
        Body::new("base", Role::Base, vec![bx(a, (ma - 0.5 * bl, ma + 0.5 * bl), acr, (table, seat))]),
        Body::new(
            "jaw",
            Role::Jaw,
            vec![bx(a, (ma - 0.5 * bl, ma - 0.5 * tenon), acr, (seat, top))],
        ),
        Body::new(
            "jaw",
            Role::Jaw,
            vec![bx(a, (ma + 0.5 * tenon, ma + 0.5 * bl), acr, (seat, top))],
        ),
    ];
    Ok(Placed {
        name: name.to_string(),
        kind: Kind::Dovetail,
        bodies,
        mount: Mount::Table { z: table },
        held: vec![(stock.min, [stock.max[0], stock.max[1], top])],
        notes: vec![
            format!(
                "dovetail {}: the stock's underside needs a {angle}° dovetail tenon {tenon:.2} mm wide (across {}), {grip:.2} mm deep, cut before this setup",
                rec.name,
                ["x", "y"][a]
            ),
            format!("held below z = {top:.2}; every side above it is open to the tool"),
        ],
    })
}

/// A strap clamp: its nose on the stock's top `bite` in from `at` (a point on the stock's edge)
/// along `dir` (unit, xy, into the stock), its heel on a step block, its stud `stud` from the
/// nose; the table at `table`.
#[allow(clippy::too_many_arguments)]
pub fn strap_clamp(
    name: &str,
    rec: &Record,
    step: &Record,
    stock: &Box3,
    table: f64,
    at: [f64; 2],
    dir: [f64; 2],
    bite: f64,
    stud: f64,
) -> Result<Placed, WhError> {
    want(rec, &[Kind::StrapClamp])?;
    want(step, &[Kind::StepBlock])?;
    let (l, w, t) = (rec.get("length")?, rec.get("width")?, rec.get("thickness")?);
    let (sd, above) = (rec.get("stud_d")?, rec.get("stud_above")?);
    let (rise, n, tread, sw) = (step.get("step")?, step.get("steps")?, step.get("tread")?, step.get("width")?);
    let nd = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
    if !(nd > 0.0) || !(bite > 0.0 && bite < l) || !(stud > bite && stud < l - tread) {
        return Err(refuse(
            name,
            format!("bite {bite:.2}, stud {stud:.2} on a strap {l:.2} long"),
            "0 < bite < stud < length − tread, and a direction",
        ));
    }
    let d = [dir[0] / nd, dir[1] / nd];
    let pt = |s: f64| [at[0] + d[0] * s, at[1] + d[1] * s];
    let (lo, hi) = ([stock.min[0], stock.min[1]], [stock.max[0], stock.max[1]]);
    if xy_dist(pt(bite), lo, hi) > 0.0 || xy_dist(pt(0.0), lo, hi) > 1e-6 {
        return Err(refuse(
            name,
            "the strap's nose does not rest on the stock",
            "`at` on the stock's edge, the direction into it",
        ));
    }
    let heel = pt(bite - l);
    if xy_dist(heel, lo, hi) < tread {
        return Err(refuse(name, "the strap's heel lies on the stock", "a longer strap, or another place"));
    }
    let ztop = stock.max[2];
    if ztop <= table {
        return Err(refuse(name, "the stock's top is not above the table", "the table's z"));
    }
    // the step whose top is nearest the stock's top carries the heel
    let steps = n.round() as i64;
    let k = (((ztop - table) / rise).round() as i64).clamp(1, steps);
    let zheel = table + k as f64 * rise;
    let axes = [[d[0], d[1], 0.0], [-d[1], d[0], 0.0], [0.0, 0.0, 1.0]];
    // a block along d from s0 to s1 (from `at`), w wide, z0..z1
    let blk = |s0: f64, s1: f64, wd: f64, z0: f64, z1: f64| {
        let p = pt(s0);
        Prim::Block {
            o: [p[0] + d[1] * 0.5 * wd, p[1] - d[0] * 0.5 * wd, z0],
            axes,
            size: [s1 - s0, wd, z1 - z0],
        }
    };
    let zlo = ztop.min(zheel);
    let zhi = ztop.max(zheel) + t;
    let mut bodies = vec![Body::new("strap", Role::Clamp, vec![blk(bite - l, bite, w, zlo, zhi)])];
    let sp = pt(bite - stud);
    if xy_dist(sp, lo, hi) < 0.5 * sd {
        return Err(refuse(name, "the stud stands in the stock", "a longer stud distance"));
    }
    bodies.push(Body::new("stud", Role::Stud, vec![Prim::post(sp[0], sp[1], 0.5 * sd, table, zhi + above)]));
    // the staircase: step j's tread; the heel's last `tread` rests on step k, higher steps behind
    let s_k = bite - l;
    let mut stair = Vec::new();
    for j in 1..=steps {
        let off = (j - k) as f64 * tread;
        stair.push(blk(s_k - off, s_k - off + tread, sw, table, table + j as f64 * rise));
    }
    bodies.push(Body::new("step block", Role::StepBlock, stair));
    let tilt = forge_math::dmath::atan2(zheel - ztop, l - bite - 0.5 * tread).to_degrees();
    Ok(Placed {
        name: name.to_string(),
        kind: Kind::StrapClamp,
        bodies,
        mount: Mount::Table { z: table },
        held: vec![(
            [pt(0.0)[0].min(pt(bite)[0]), pt(0.0)[1].min(pt(bite)[1]), stock.min[2]],
            [pt(0.0)[0].max(pt(bite)[0]), pt(0.0)[1].max(pt(bite)[1]), ztop],
        )],
        notes: vec![format!(
            "strap {}: nose {bite:.2} on the stock's top (z = {ztop:.2}), heel on step {k} of {} (z = {zheel:.2}, the strap tilts {tilt:.1}°), stud {stud:.2} from the nose",
            rec.name, step.name
        )],
    })
}

/// A toe or edge clamp (Mitee-Bite style) against the stock's side at `at` (on the edge),
/// pushing along `dir` (unit xy, into the stock), standing on `seat` (the plate or table under it).
pub fn side_clamp(name: &str, rec: &Record, stock: &Box3, seat: f64, at: [f64; 2], dir: [f64; 2]) -> Result<Placed, WhError> {
    want(rec, &[Kind::ToeClamp, Kind::EdgeClamp])?;
    let (bl, bw, h) = (rec.get("body_length")?, rec.get("body_width")?, rec.get("height")?);
    let nd = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
    let (lo, hi) = ([stock.min[0], stock.min[1]], [stock.max[0], stock.max[1]]);
    if !(nd > 0.0) || xy_dist(at, lo, hi) > 1e-6 {
        return Err(refuse(name, "the clamp does not touch the stock", "`at` on the stock's edge, the direction into it"));
    }
    let d = [dir[0] / nd, dir[1] / nd];
    let back = [at[0] - d[0] * 1e-3, at[1] - d[1] * 1e-3];
    if xy_dist(back, lo, hi) == 0.0 {
        return Err(refuse(name, "the clamp would stand in the stock", "the direction into the stock"));
    }
    let axes = [[d[0], d[1], 0.0], [-d[1], d[0], 0.0], [0.0, 0.0, 1.0]];
    let o = [at[0] - d[0] * bl + d[1] * 0.5 * bw, at[1] - d[1] * bl - d[0] * 0.5 * bw, seat];
    let body = Prim::Block {
        o,
        axes,
        size: [bl, bw, h],
    };
    let mut notes = vec![format!("{} {} at ({:.2}, {:.2}) pushing ({:.3}, {:.3})", rec.kind.name(), rec.name, at[0], at[1], d[0], d[1])];
    if seat + h > stock.max[2] {
        notes.push(format!(
            "it stands {:.2} mm above the stock's top: the top cannot be faced past it",
            seat + h - stock.max[2]
        ));
    }
    let side = [at[0] + d[1] * 0.5 * bw, at[1] - d[0] * 0.5 * bw];
    let side2 = [at[0] - d[1] * 0.5 * bw, at[1] + d[0] * 0.5 * bw];
    Ok(Placed {
        name: name.to_string(),
        kind: rec.kind,
        bodies: vec![Body::new("clamp", Role::Clamp, vec![body])],
        mount: Mount::Table { z: seat },
        held: vec![(
            [side[0].min(side2[0]), side[1].min(side2[1]), stock.min[2]],
            [side[0].max(side2[0]), side[1].max(side2[1]), (seat + h).min(stock.max[2])],
        )],
        notes,
    })
}

/// A fixture plate under the stock, its corner at `corner` (xy), with dowels in holes (i, j).
pub fn fixture_plate(name: &str, rec: &Record, stock: &Box3, corner: [f64; 2], dowels: &[(i32, i32)]) -> Result<Placed, WhError> {
    want(rec, &[Kind::FixturePlate])?;
    let (l, w, t) = (rec.get("length")?, rec.get("width")?, rec.get("thickness")?);
    let (pitch, off) = (rec.get("pitch")?, rec.get("hole_offset")?);
    let seat = stock.min[2];
    let (lo, hi) = ([stock.min[0], stock.min[1]], [stock.max[0], stock.max[1]]);
    if lo[0] < corner[0] || lo[1] < corner[1] || hi[0] > corner[0] + l || hi[1] > corner[1] + w {
        return Err(refuse(name, "the stock overhangs the plate", "move the plate, or a bigger one"));
    }
    let mut bodies = vec![Body::new(
        "plate",
        Role::Plate,
        vec![Prim::aabb([corner[0], corner[1], seat - t], [corner[0] + l, corner[1] + w, seat])],
    )];
    let mut notes = vec![format!("plate {}: {pitch} mm grid, top z = {seat:.2}", rec.name)];
    for &(i, j) in dowels {
        let (dd, dh) = (rec.get("dowel_d")?, rec.get("dowel_h")?);
        let c = [corner[0] + off + i as f64 * pitch, corner[1] + off + j as f64 * pitch];
        if c[0] > corner[0] + l || c[1] > corner[1] + w || i < 0 || j < 0 {
            return Err(refuse(name, format!("hole ({i}, {j}) is off the plate"), "a hole on the plate"));
        }
        let gap = xy_dist(c, lo, hi);
        if gap < 0.5 * dd - 1e-9 {
            return Err(refuse(
                name,
                format!("the dowel in hole ({i}, {j}) stands under the stock"),
                "a hole outside the stock's outline",
            ));
        }
        if (gap - 0.5 * dd).abs() <= 1e-6 {
            notes.push(format!("the dowel in hole ({i}, {j}) locates the stock"));
        }
        bodies.push(Body::new(format!("dowel ({i}, {j})"), Role::Dowel, vec![Prim::post(c[0], c[1], 0.5 * dd, seat, seat + dh)]));
    }
    Ok(Placed {
        name: name.to_string(),
        kind: Kind::FixturePlate,
        bodies,
        mount: Mount::Table { z: seat - t },
        held: vec![],
        notes,
    })
}

/// A zero-point system under a device: an optional riser, then receiver modules at whole grid
/// steps `modules` (in pitches) about `at`; `on` is then mounted on it.
pub fn zero_point(
    name: &str,
    rec: &Record,
    riser: Option<&Record>,
    on: &mut Placed,
    at: [f64; 2],
    modules: &[(i32, i32)],
) -> Result<Placed, WhError> {
    want(rec, &[Kind::ZeroPoint])?;
    if !matches!(on.mount, Mount::Table { .. }) {
        return Err(refuse(name, format!("{} is already mounted on something", on.name), "mount it once"));
    }
    let (pitch, md, mh) = (rec.get("pitch")?, rec.get("module_d")?, rec.get("module_h")?);
    let mut z = on.bottom();
    let mut bodies = vec![];
    let mut notes = vec![];
    if let Some(r) = riser {
        want(r, &[Kind::Riser])?;
        let (d, h) = (r.get("d")?, r.get("height")?);
        bodies.push(Body::new(format!("riser ({})", r.name), Role::Riser, vec![Prim::post(at[0], at[1], 0.5 * d, z - h, z)]));
        notes.push(format!("riser {}: {h} mm", r.name));
        z -= h;
    }
    if modules.is_empty() {
        return Err(refuse(name, "no receiver module", "modules: [(0, 0)]"));
    }
    for &(i, j) in modules {
        let c = [at[0] + i as f64 * pitch, at[1] + j as f64 * pitch];
        bodies.push(Body::new(format!("module ({i}, {j})"), Role::Module, vec![Prim::post(c[0], c[1], 0.5 * md, z - mh, z)]));
    }
    notes.insert(0, format!("zero-point {}: {} module(s) on a {pitch} mm grid, the device on top at z = {:.2}", rec.name, modules.len(), on.bottom()));
    on.mount = Mount::On {
        below: name.to_string(),
        z: on.bottom(),
    };
    Ok(Placed {
        name: name.to_string(),
        kind: Kind::ZeroPoint,
        bodies,
        mount: Mount::Table { z: z - mh },
        held: vec![],
        notes,
    })
}

/// A tombstone (HMC): the stock seated on one face, the face's centre at `centre` (xy of this
/// setup's frame, whose −z goes into the tombstone).
pub fn tombstone(name: &str, rec: &Record, stock: &Box3, centre: [f64; 2]) -> Result<Placed, WhError> {
    want(rec, &[Kind::Tombstone])?;
    let (w, h, dp) = (rec.get("width")?, rec.get("height")?, rec.get("depth")?);
    let seat = stock.min[2];
    let lo = [centre[0] - 0.5 * w, centre[1] - 0.5 * h];
    let hi = [centre[0] + 0.5 * w, centre[1] + 0.5 * h];
    if stock.min[0] < lo[0] || stock.min[1] < lo[1] || stock.max[0] > hi[0] || stock.max[1] > hi[1] {
        return Err(refuse(name, "the stock overhangs the face", "move it on the face, or a bigger tombstone"));
    }
    Ok(Placed {
        name: name.to_string(),
        kind: Kind::Tombstone,
        bodies: vec![Body::new("column", Role::Column, vec![Prim::aabb([lo[0], lo[1], seat - dp], [hi[0], hi[1], seat])])],
        mount: Mount::On {
            below: "the pallet (posed by the machine)".into(),
            z: seat - dp,
        },
        held: vec![],
        notes: vec![format!("tombstone {}: face {w}×{h}, the column {dp} deep behind the seat", rec.name)],
    })
}

/// A vacuum plate under the stock (centred).
pub fn vacuum(name: &str, rec: &Record, stock: &Box3) -> Result<Placed, WhError> {
    want(rec, &[Kind::Vacuum])?;
    let (l, w, t) = (rec.get("length")?, rec.get("width")?, rec.get("thickness")?);
    let s = stock.size();
    if s[0] > l || s[1] > w {
        return Err(refuse(name, "the stock overhangs the vacuum plate", "a bigger plate"));
    }
    let m = stock.mid();
    let seat = stock.min[2];
    Ok(Placed {
        name: name.to_string(),
        kind: Kind::Vacuum,
        bodies: vec![Body::new(
            "plate",
            Role::Plate,
            vec![Prim::aabb([m[0] - 0.5 * l, m[1] - 0.5 * w, seat - t], [m[0] + 0.5 * l, m[1] + 0.5 * w, seat])],
        )],
        mount: Mount::Table { z: seat - t },
        held: vec![],
        notes: vec![format!("vacuum plate {}", rec.name)],
    })
}

/// The vacuum hold-down rule, a stated check (not a proof): the sealed area A (the stock's
/// footprint less the gasket's inset each side) under the vacuum Δp (kPa below atmosphere)
/// holds F = Δp·A; with friction μ that resists a side force of μ·F, which must be at least
/// `sf` times the cut's largest side force `f_side` (N). All four inputs are the user's.
/// Ok: the margin μ·F / (sf·f_side) (≥ 1).
pub fn hold_down(rec: &Record, stock: &Box3, mu: f64, sf: f64, f_side: f64) -> Result<f64, WhError> {
    want(rec, &[Kind::Vacuum])?;
    let (dp, inset) = (rec.get("vacuum")?, rec.get("seal_inset")?);
    let s = stock.size();
    let area = (s[0] - 2.0 * inset).max(0.0) * (s[1] - 2.0 * inset).max(0.0);
    let f = dp * 1e-3 * area; // kPa = 1e-3 N/mm²
    let need = sf * f_side;
    if !(mu > 0.0 && sf >= 1.0 && f_side >= 0.0) {
        return Err(refuse(&rec.name, "μ > 0, a safety factor ≥ 1, a side force ≥ 0", "give them"));
    }
    if mu * f >= need {
        Ok(if need == 0.0 { f64::INFINITY } else { mu * f / need })
    } else {
        let amin = need / (mu * dp * 1e-3);
        Err(refuse(
            &rec.name,
            format!(
                "the sealed area {area:.0} mm² at {dp} kPa holds {f:.0} N; with μ = {mu} it resists {:.0} N, less than {sf} × {f_side:.0} N",
                mu * f
            ),
            format!("a sealed area of at least {amin:.0} mm² (tabs, a bigger blank), a lighter cut, or a mechanical clamp"),
        ))
    }
}

/// A custom fixture from the user's record: its bodies (seat at z = 0 in its frame) posed by
/// `pose` into the part frame; `held` boxes in the part frame.
pub fn custom(name: &str, rec: &Record, pose: &Pose, held: Vec<(V3, V3)>) -> Result<Placed, WhError> {
    want(rec, &[Kind::Custom])?;
    let bodies: Vec<Body> = rec
        .bodies
        .iter()
        .map(|b| Body {
            name: b.name.clone(),
            role: b.role,
            prims: b.prims.iter().map(|p| p.posed(pose)).collect(),
            keepout: 0.0,
            part: b.part.clone(),
        })
        .collect();
    let mut p = Placed {
        name: name.to_string(),
        kind: Kind::Custom,
        bodies,
        mount: Mount::Table { z: 0.0 },
        held,
        notes: vec![format!("custom fixture {}", rec.name)],
    };
    p.mount = Mount::Table { z: p.bottom() };
    Ok(p)
}

/// Copies of a placed device by poses (several vises, several parts on one plate). Each copy's
/// name gets its index.
pub fn pattern(p: &Placed, poses: &[Pose]) -> Vec<Placed> {
    poses
        .iter()
        .enumerate()
        .map(|(k, q)| {
            let mv = |x: V3| q.point(x);
            Placed {
                name: format!("{}[{}]", p.name, k + 1),
                kind: p.kind,
                bodies: p
                    .bodies
                    .iter()
                    .map(|b| Body {
                        prims: b.prims.iter().map(|x| x.posed(q)).collect(),
                        ..b.clone()
                    })
                    .collect(),
                mount: match &p.mount {
                    Mount::Table { z } => Mount::Table { z: z + q.t[2] },
                    m => m.clone(),
                },
                held: p
                    .held
                    .iter()
                    .map(|(a, b)| {
                        let (u, v) = (mv(*a), mv(*b));
                        (
                            [u[0].min(v[0]), u[1].min(v[1]), u[2].min(v[2])],
                            [u[0].max(v[0]), u[1].max(v[1]), u[2].max(v[2])],
                        )
                    })
                    .collect(),
                notes: p.notes.clone(),
            }
        })
        .collect()
}

/// Whether a pose turns about z by a whole number of quarter turns (so boxes stay boxes).
pub fn keeps_boxes(p: &Pose) -> bool {
    p.m.iter().flatten().all(|x| *x == 0.0 || x.abs() == 1.0) && p.m[2] == IDENT[2]
}
