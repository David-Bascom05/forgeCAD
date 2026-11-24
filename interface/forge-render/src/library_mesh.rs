//! Library records drawn as what they are (docs/ux/STANDARD.md §4): every solid comes from the
//! record's own dimensions; a dimension the record does not hold is drawn as a ghost (a dashed
//! outline, no faces) with a label saying which value is missing. Nothing is drawn solid that the
//! record does not give.
//!
//! - a cutting tool: its fluted length (helical grooves, `nof` of them at the record's `helix`,
//!   straight when it gives none) on the tip's own shape (ball, bull, chamfer, drill point), then
//!   its neck (`neck_d`, `reach`) and shank, from forge-tooling-builder's outline;
//! - a holder, collet, nut or pull stud: its turned profile;
//! - a vise: base, fixed jaw, moving jaw at the maximum opening (`body_length`, `body_width`,
//!   `height`, `jaw_width`, `jaw_height`, `jaw_thickness`, `max_opening`); the screw is not in
//!   the record: its axis only, labelled;
//! - a lathe chuck: body (`body_d`, `body_len`) and `jaws` jaws open to `grip_max` over `jaw_len`;
//!   the jaws' width is not in the record (ghosts);
//! - a press brake: bed, ram and both side housings at `bed_length`, `housing_gap`, `open_height`,
//!   `table_height`, `throat`, `frame.*`, `ram.*`, `bed.*`;
//! - a punch or die: forge-brake's exact section swept along each of its segments.
use crate::machine_mesh::{Tris, prims_tris};
use forge_machine::def::Prim;

/// What a part is made of, for its appearance (data/appearances.toml).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Ground tool steel: punches, dies, jaws.
    ToolSteel,
    /// Machined steel: holders, collets, chuck bodies.
    Steel,
    /// A cutter's fluted carbide: no carbide appearance in the library yet, so black-oxide steel
    /// reads as the darker cutting part.
    Cutting,
    /// A cast, painted body: vise bases, press frames.
    MachinePaint,
    /// A dark beam (a press's bed and ram), so the frame's parts separate.
    DarkSteel,
    /// What stands in for a part of the scene the record does not hold (a press's stand-in punch
    /// and die): a contrasting warm tone, so it is never taken for the record's own.
    StandIn,
    /// The floor the machine stands on.
    Floor,
}

impl Look {
    /// The appearance's name in the library.
    pub fn appearance(self) -> &'static str {
        match self {
            Look::ToolSteel => "Tool steel, ground",
            Look::Steel => "Steel, machined",
            Look::Cutting => "Steel, black oxide",
            Look::MachinePaint => "Paint, machine gloss",
            Look::DarkSteel => "Steel, black oxide",
            Look::StandIn => "Brass",
            Look::Floor => "Concrete floor",
        }
    }

    /// The raster view's tint (linear colour, metalness) from the appearance library.
    pub fn tint(self) -> [f32; 4] {
        let apps = crate::studio::appearance::Appearances::builtin();
        apps.find(self.appearance())
            .map(|a| a.viewport_tint(None))
            .unwrap_or(crate::palette::GREY)
    }
}

/// A value the record does not give, where it would be.
#[derive(Clone, Debug, PartialEq)]
pub struct Ghost {
    /// A box ghost's bounds (drawn see-through by `crate::ghost::ghost_box`); None: `edges`.
    pub bounds: Option<([f64; 3], [f64; 3])>,
    /// Dashed lines, for a ghost that is not a box (a screw's axis, a turned jaw).
    pub edges: Vec<Vec<[f64; 3]>>,
    /// Where its label points.
    pub at: [f64; 3],
    /// "height · unknown".
    pub label: String,
}

/// A section to draw to scale beside the view: its outline (x, z) and its dimensions (name, value).
pub type Inset = (Vec<[f64; 2]>, Vec<(String, String)>);

/// A record's preview.
#[derive(Clone, Debug, Default)]
pub struct Preview {
    pub parts: Vec<(Tris, Look)>,
    pub ghosts: Vec<Ghost>,
    /// A long tool's section, to show to scale beside the view: (outline x z, dimensions).
    pub section: Option<Inset>,
    /// Look along the length (a die, a punch, a press): the axis the subject is long on.
    pub long_axis: Option<usize>,
    /// Framed as its operator sees it (front, lower), not from the 3/4 iso.
    pub operator: bool,
}

impl Preview {
    /// Bounds of everything drawn, solid or ghost.
    pub fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let pts = self
            .parts
            .iter()
            .flat_map(|(t, _)| t.pos.iter())
            .chain(self.ghosts.iter().flat_map(|g| g.edges.iter().flatten()))
            .chain(
                self.ghosts
                    .iter()
                    .filter_map(|g| g.bounds.as_ref())
                    .flat_map(|(a, b)| [a, b]),
            );
        for p in pts {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        lo[0].is_finite().then_some((lo, hi))
    }

    fn solid_box(&mut self, min: [f64; 3], max: [f64; 3], look: Look) {
        self.parts.push((prims_tris(&[Prim::Box { min, max }]), look));
    }

    fn solid_cyl(&mut self, base: [f64; 3], axis: [f64; 3], r: f64, h: f64, look: Look) {
        self.parts
            .push((prims_tris(&[Prim::Cyl { base, axis, r, h }]), look));
    }

    /// A body with no size from any source: a see-through box and its label.
    pub fn ghost_box(&mut self, min: [f64; 3], max: [f64; 3], label: impl Into<String>) {
        self.ghosts.push(Ghost {
            bounds: Some((min, max)),
            edges: Vec::new(),
            at: [max[0], (min[1] + max[1]) / 2.0, max[2]],
            label: label.into(),
        });
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A line as dashes (each a 2-point polyline), dash and gap `step` long.
fn dashes(a: [f64; 3], b: [f64; 3], step: f64) -> Vec<Vec<[f64; 3]>> {
    let l = dist(a, b);
    if l <= 0.0 || step <= 0.0 {
        return Vec::new();
    }
    let n = ((l / step).ceil() as usize).max(1);
    let at = |s: f64| {
        [
            a[0] + (b[0] - a[0]) * s,
            a[1] + (b[1] - a[1]) * s,
            a[2] + (b[2] - a[2]) * s,
        ]
    };
    (0..n)
        .step_by(2)
        .map(|i| vec![at(i as f64 / n as f64), at(((i + 1) as f64 / n as f64).min(1.0))])
        .collect()
}

/// A box's twelve edges, dashed.
pub fn dashed_box(min: [f64; 3], max: [f64; 3]) -> Vec<Vec<[f64; 3]>> {
    let c = |i: usize| {
        [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ]
    };
    let step = dist(min, max) / 40.0;
    let mut v = Vec::new();
    for i in 0..8 {
        for bit in [1, 2, 4] {
            if i & bit == 0 {
                v.extend(dashes(c(i), c(i | bit), step));
            }
        }
    }
    v
}

/// A closed section in a plane, swept along an axis: `prism` along y, turned to `along`
/// (0 = x, 1 = y), from `y0` to `y1`.
fn sweep(outline: &[[f64; 2]], y0: f64, y1: f64) -> Tris {
    let mut t = crate::machine_mesh::prism(outline, y1 - y0);
    let mid = (y0 + y1) / 2.0;
    for p in t.pos.iter_mut() {
        p[1] += mid;
    }
    for e in t.edges.iter_mut() {
        for p in e.iter_mut() {
            p[1] += mid;
        }
    }
    t
}

// ---------------------------------------------------------------------------------------------
// cutting tools

/// The radius of a turned outline (z, r) at `z` (its first crossing).
fn radius_at(outline: &[[f64; 2]], z: f64) -> f64 {
    for w in outline.windows(2) {
        let ([z0, r0], [z1, r1]) = (w[0], w[1]);
        if (z0..=z1).contains(&z) && z1 > z0 {
            return r0 + (r1 - r0) * (z - z0) / (z1 - z0);
        }
    }
    0.0
}

/// A cutter's fluted length: the tip's shape (`outline` up to `lcf`) with `nof` helical grooves
/// at `helix` degrees (0: straight).
pub fn fluted(outline: &[[f64; 2]], lcf: f64, nof: u32, helix: f64) -> Tris {
    let mut t = Tris::default();
    let rc = outline
        .iter()
        .filter(|p| p[0] <= lcf + 1e-9)
        .map(|p| p[1])
        .fold(0.0, f64::max)
        .max(1e-6);
    let (hs, hc) = forge_math::dmath::sin_cos(helix.to_radians());
    let tan = hs / hc;
    let nof = nof.max(1) as f64;
    // the groove: a quarter of each pitch, 22 % of the radius deep
    let g = |phi: f64| {
        let u = (nof * phi / std::f64::consts::TAU).rem_euclid(1.0);
        if u < 0.3 {
            let (s, _) = forge_math::dmath::sin_cos(std::f64::consts::PI * u / 0.3);
            1.0 - 0.22 * s * s
        } else {
            1.0
        }
    };
    let (nz, nt) = (64usize, 120usize);
    let pt = |i: usize, k: usize| {
        let z = lcf * i as f64 / nz as f64;
        let th = std::f64::consts::TAU * k as f64 / nt as f64;
        let phi = th - z * tan / rc;
        let r = radius_at(outline, z) * g(phi);
        let (sn, cs) = forge_math::dmath::sin_cos(th);
        [r * cs, r * sn, z]
    };
    for i in 0..nz {
        for k in 0..nt {
            let q = [pt(i, k), pt(i, k + 1), pt(i + 1, k + 1), pt(i + 1, k)];
            quad(&mut t, q);
        }
    }
    // the end face of a flat or bull tip: a fan to the axis
    if radius_at(outline, 0.0) > 1e-6 {
        let b = t.pos.len() as u32;
        t.pos.push([0.0, 0.0, 0.0]);
        t.nrm.push([0.0, 0.0, -1.0]);
        for k in 0..=nt {
            t.pos.push(pt(0, k));
            t.nrm.push([0.0, 0.0, -1.0]);
        }
        for k in 0..nt as u32 {
            t.tri.push([b, b + 2 + k, b + 1 + k]);
        }
    }
    // the cutting edges (each flute's lip) and the end of the flutes
    for f in 0..nof as usize {
        let th0 = std::f64::consts::TAU * f as f64 / nof;
        t.edges.push(
            (0..=nz)
                .map(|i| {
                    let z = lcf * i as f64 / nz as f64;
                    let th = th0 + z * tan / rc;
                    let r = radius_at(outline, z) * 1.002;
                    let (sn, cs) = forge_math::dmath::sin_cos(th);
                    [r * cs, r * sn, z]
                })
                .collect(),
        );
    }
    t.edges.push((0..=nt).map(|k| pt(nz, k)).collect());
    let (n_t, n_v) = (t.tri.len(), t.pos.len());
    t.patches.push((0, n_t as u32, 0, n_v as u32));
    t
}

fn quad(t: &mut Tris, q: [[f64; 3]; 4]) {
    let u = [q[1][0] - q[0][0], q[1][1] - q[0][1], q[1][2] - q[0][2]];
    let v = [q[3][0] - q[0][0], q[3][1] - q[0][1], q[3][2] - q[0][2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if l == 0.0 {
        return;
    }
    let n = [n[0] / l, n[1] / l, n[2] / l];
    let b = t.pos.len() as u32;
    t.pos.extend_from_slice(&q);
    t.nrm.extend_from_slice(&[n; 4]);
    t.tri.push([b, b + 1, b + 2]);
    t.tri.push([b, b + 2, b + 3]);
}

/// A cutting tool: its fluted length, then the rest of its outline (neck, shank) turned.
pub fn cutter(outline: &[[f64; 2]], lcf: f64, nof: u32, helix: f64) -> Preview {
    let mut p = Preview::default();
    if nof > 0 && lcf > 0.0 {
        p.parts.push((fluted(outline, lcf, nof, helix), Look::Cutting));
        // the rest, from the end of the flutes: neck and shank
        let mut rest: Vec<[f64; 2]> = vec![[lcf, 0.0]];
        rest.extend(outline.iter().copied().filter(|q| q[0] >= lcf - 1e-9));
        p.parts
            .push((crate::machine_mesh::revolve(&rest), Look::ToolSteel));
    } else {
        p.parts
            .push((crate::machine_mesh::revolve(outline), Look::ToolSteel));
    }
    p
}

/// A turned record (holder, collet, nut, pull stud): its profile.
pub fn turned(outline: &[[f64; 2]]) -> Preview {
    let mut p = Preview::default();
    p.parts.push((crate::machine_mesh::revolve(outline), Look::Steel));
    p
}

// ---------------------------------------------------------------------------------------------
// workholding

/// A record's value for `k`: the published one, else the first value its note gives for a
/// conflict ("height in conflict: the page gives OAH 110.74 but ..."), with what it is.
fn best(r: &forge_workholding::Record, k: &str) -> Option<(f64, Basis)> {
    if let Some(v) = r.vals.get(k).copied().filter(|x| *x > 0.0) {
        return Some((v, Basis::Cited));
    }
    if r.conflict.iter().any(|x| x == k) {
        let at = r.note.find(&format!("{k} in conflict"))?;
        let v = first_number(&r.note[at..])?;
        return Some((v, Basis::Conflict));
    }
    None
}

/// How a drawn size was come by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    Cited,
    /// The first of disagreeing cited values.
    Conflict,
    /// Not in the record: the rule given with it.
    Estimate,
}

fn first_number(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let j = (i..b.len())
                .find(|&j| !(b[j].is_ascii_digit() || b[j] == b'.'))
                .unwrap_or(b.len());
            return s[i..j].parse().ok();
        }
        i += 1;
    }
    None
}

impl Preview {
    /// A dimension the record is not sure of: a dashed line along it, and its label.
    fn mark(&mut self, a: [f64; 3], b: [f64; 3], label: impl Into<String>) {
        let l = dist(a, b);
        let mut e = dashes(a, b, l / 14.0);
        // end ticks across, a tenth of the length
        let t = l * 0.06;
        for p in [a, b] {
            e.push(vec![[p[0] - t, p[1], p[2]], [p[0] + t, p[1], p[2]]]);
        }
        self.ghosts.push(Ghost {
            bounds: None,
            edges: e,
            at: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0],
            label: label.into(),
        });
    }
}

/// A machine vise from its record (docs/ux/STANDARD.md §4.2): base, fixed jaw, moving jaw at the
/// maximum opening and the screw, solid at their best sizes; a size the record does not settle
/// (a conflict, an estimate) marked by a dashed dimension line and its label. Its seat (the
/// table) at z = 0, the clamp axis along x, the fixed jaw at -x.
pub fn vise(r: &forge_workholding::Record) -> Preview {
    let mut p = Preview::default();
    let jw = best(r, "jaw_width");
    let jh = best(r, "jaw_height");
    let mo = best(r, "max_opening");
    let bl = best(r, "body_length");
    let bw = best(r, "body_width");
    let h = best(r, "height");
    let jt = best(r, "jaw_thickness");
    // a size from no source: the rule given in its label
    let scale = [jw, bw, mo, bl]
        .iter()
        .flatten()
        .map(|x| x.0)
        .fold(0.0, f64::max)
        .max(50.0);
    let est = |v: Option<(f64, Basis)>, rule: f64| v.unwrap_or((rule, Basis::Estimate));
    let (jw, jh) = (est(jw, scale), est(jh, scale * 0.3));
    let jt = est(jt, jh.0 * 0.5);
    let mo = est(mo, scale);
    let bw = est(bw, jw.0);
    let bl = est(bl, mo.0 + 4.0 * jt.0);
    // the base up to the jaws' seat: the height less the jaw height
    let base = match h {
        Some((h, b)) if h > jh.0 => (h - jh.0, b),
        _ => (jh.0, Basis::Estimate),
    };
    let (x0, x1, y) = (-bl.0 / 2.0, bl.0 / 2.0, bw.0 / 2.0);
    let bt = base.0;
    p.solid_box([x0, -y, 0.0], [x1, y, bt], Look::MachinePaint);
    // the jaws: the fixed one near the -x end, the moving one at the maximum opening
    let fixed_in = x0 + jt.0 + bl.0 * 0.04;
    p.solid_box(
        [fixed_in - jt.0, -jw.0 / 2.0, bt],
        [fixed_in, jw.0 / 2.0, bt + jh.0],
        Look::ToolSteel,
    );
    let mv = fixed_in + mo.0;
    p.solid_box(
        [mv, -jw.0 / 2.0, bt],
        [mv + jt.0, jw.0 / 2.0, bt + jh.0],
        Look::ToolSteel,
    );
    // the screw out of the +x end: not in the record, drawn at an estimate
    let sd = (jw.0 * 0.14).max(8.0);
    let zc = bt * 0.55;
    let sl = bl.0 * 0.12;
    p.solid_cyl([x1, 0.0, zc], [1.0, 0.0, 0.0], sd / 2.0, sl, Look::Steel);
    p.mark(
        [x1, 0.0, zc + sd],
        [x1 + sl, 0.0, zc + sd],
        "Screw · not in the record (an estimate)",
    );
    // the sizes it is not sure of
    let say = |what: &str, v: f64, b: Basis| match b {
        Basis::Cited => String::new(),
        Basis::Conflict => format!(
            "{what} {} mm · sources disagree (the first cited)",
            forge_units::trim(v)
        ),
        Basis::Estimate => format!("{what} · not published (an estimate)"),
    };
    if let Some((hv, hb)) = h.map(|x| (x.0, x.1)).or(Some((bt + jh.0, Basis::Estimate)))
        && hb != Basis::Cited
    {
        p.mark([x0, -y, 0.0], [x0, -y, hv], say("Height", hv, hb));
    }
    if jt.1 != Basis::Cited {
        p.mark(
            [fixed_in - jt.0, -jw.0 / 2.0, bt + jh.0],
            [fixed_in, -jw.0 / 2.0, bt + jh.0],
            match jt.1 {
                Basis::Estimate => "Jaw thickness · not published (half the jaw height)".to_string(),
                b => say("Jaw thickness", jt.0, b),
            },
        );
    }
    for (what, v, a, b) in [
        (
            "Jaw width",
            jw,
            [fixed_in, -jw.0 / 2.0, bt + jh.0],
            [fixed_in, jw.0 / 2.0, bt + jh.0],
        ),
        ("Max opening", mo, [fixed_in, y, bt + jh.0], [mv, y, bt + jh.0]),
        ("Body length", bl, [x0, y, 0.0], [x1, y, 0.0]),
        ("Body width", bw, [x1, -y, 0.0], [x1, y, 0.0]),
    ] {
        if v.1 != Basis::Cited {
            p.mark(a, b, say(what, v.0, v.1));
        }
    }
    p
}

/// A lathe chuck from its record (STANDARD §4.2): body and jaws solid, the jaws open to the
/// largest grip; a size the record does not give drawn at an estimate and marked. Its face at
/// z = 0, its body behind (-z), its jaws in front.
pub fn chuck(r: &forge_workholding::Record) -> Preview {
    let mut p = Preview::default();
    let gmax = best(r, "grip_max");
    let bd = best(r, "body_d").unwrap_or((gmax.map_or(200.0, |g| g.0 * 1.3), Basis::Estimate));
    let bl = best(r, "body_len").unwrap_or((bd.0 * 0.4, Basis::Estimate));
    let jl = best(r, "jaw_len").unwrap_or((bd.0 * 0.15, Basis::Estimate));
    let n = best(r, "jaws").map_or(3, |n| n.0.round() as usize).clamp(2, 8);
    p.solid_cyl([0.0, 0.0, -bl.0], [0.0, 0.0, 1.0], bd.0 / 2.0, bl.0, Look::Steel);
    let r_in = gmax.map_or(bd.0 * 0.15, |x| x.0 / 2.0).min(bd.0 * 0.45);
    let r_out = bd.0 / 2.0;
    let w = ((r_out - r_in) * 0.45).max(4.0);
    for k in 0..n {
        let th = std::f64::consts::TAU * k as f64 / n as f64;
        let (sn, cs) = forge_math::dmath::sin_cos(th);
        let (u, v) = ([cs, sn, 0.0], [-sn, cs, 0.0]);
        let o = [r_in * cs - w / 2.0 * v[0], r_in * sn - w / 2.0 * v[1], 0.0];
        let mut t = Tris::default();
        crate::machine_mesh::obox_tris(o, [u, v, [0.0, 0.0, 1.0]], [r_out - r_in, w, jl.0], &mut t);
        p.parts.push((t, Look::ToolSteel));
    }
    let say = |what: &str, b: Basis| match b {
        Basis::Estimate => format!("{what} · not published (an estimate)"),
        Basis::Conflict => format!("{what} · sources disagree (the first cited)"),
        Basis::Cited => String::new(),
    };
    if bl.1 != Basis::Cited {
        p.mark([r_out, 0.0, -bl.0], [r_out, 0.0, 0.0], say("Body length", bl.1));
    }
    if jl.1 != Basis::Cited {
        p.mark([r_out, 0.0, 0.0], [r_out, 0.0, jl.0], say("Jaw length", jl.1));
    }
    p.mark(
        [r_in, -w / 2.0, jl.0],
        [r_in, w / 2.0, jl.0],
        "Jaw width · not in the record (an estimate)",
    );
    if bd.1 != Basis::Cited {
        p.mark([-r_out, 0.0, 0.0], [r_out, 0.0, 0.0], say("Body diameter", bd.1));
    }
    p
}

// ---------------------------------------------------------------------------------------------
// press brakes

/// A prism whose section is in the (y, z) plane, swept along x from `x0` to `x1`.
fn sweep_x(outline_yz: &[[f64; 2]], x0: f64, x1: f64) -> Tris {
    let mut t = crate::machine_mesh::prism(outline_yz, x1 - x0);
    let mid = (x0 + x1) / 2.0;
    // (y as x, x as y): swap, which mirrors, so the triangles turn the other way
    let swap = |p: &mut [f64; 3]| {
        let (a, b) = (p[0], p[1]);
        p[0] = b + mid;
        p[1] = a;
    };
    for p in t.pos.iter_mut() {
        swap(p);
    }
    for n in t.nrm.iter_mut() {
        let (a, b) = (n[0], n[1]);
        n[0] = b;
        n[1] = a;
    }
    for tr in t.tri.iter_mut() {
        tr.swap(1, 2);
    }
    for e in t.edges.iter_mut() {
        for p in e.iter_mut() {
            swap(p);
        }
    }
    t
}

/// A press brake from its record (STANDARD §4.2): two C-frame side housings with the throat
/// open in front, the bed below and the ram above at their cited sizes, and a punch and die in
/// place (forgecad's stand-in set, said so); what the record does not give drawn at an estimate
/// and marked. The floor at z = 0, the bed along x, the operator at -y.
pub fn press(t: &forge_vendor::toml::Table) -> Preview {
    use crate::shell::machine_brake::get;
    use forge_vendor::toml::Item;
    let g = |k: &str| match get(t, k) {
        Some(Item::Num(x)) if *x > 0.0 => Some(*x),
        _ => None,
    };
    let mut p = Preview {
        long_axis: Some(0),
        ..Default::default()
    };
    let mut est: Vec<&str> = Vec::new();
    let mut or = |v: Option<f64>, e: f64, what: &'static str| {
        v.unwrap_or_else(|| {
            est.push(what);
            e
        })
    };
    let bl = or(g("bed_length"), 3000.0, "bending length");
    let gap = or(g("housing_gap"), bl * 0.8, "width between housings");
    let oh = or(g("open_height"), 400.0, "open height");
    let th = or(g("table_height"), 900.0, "table height");
    let throat = or(g("throat"), 400.0, "throat depth");
    let stroke = g("stroke");
    let ft = or(g("frame.thickness"), 90.0, "housing thickness");
    let fd = or(g("frame.depth"), throat * 2.4, "housing depth");
    let (bf, bb) = (or(g("bed.front"), 90.0, "bed"), or(g("bed.back"), 90.0, "bed"));
    let (rf, rb) = (or(g("ram.front"), 90.0, "ram"), or(g("ram.back"), 90.0, "ram"));
    let est: Vec<&str> = {
        let mut e = est;
        e.dedup();
        e
    };
    // the bed beam under the table line, the ram above the open height
    let bed_h = th * 0.45;
    let ram_h = oh * 1.6;
    let ram_z = th + oh;
    p.solid_box([-bl / 2.0, -bf, th - bed_h], [bl / 2.0, bb, th], Look::DarkSteel);
    p.solid_box(
        [-bl / 2.0, -rf, ram_z],
        [bl / 2.0, rb, ram_z + ram_h],
        Look::DarkSteel,
    );
    // the housings: a C in (y, z), the throat cut in front between the table and the ram
    let top = ram_z + ram_h + oh * 0.4;
    let y0 = -(bf.max(rf)) - 60.0;
    let y1 = y0 + fd.max(throat + 200.0);
    let yt = (-bf).min(-rf) + throat;
    let c = [
        [y0, 0.0],
        [y1, 0.0],
        [y1, top],
        [y0, top],
        // the throat: the working gap's height (table to ram, plus a margin), cut back to the
        // throat depth, so it shows as the C's mouth between the beams
        [y0, ram_z + oh * 0.15],
        [yt, ram_z + oh * 0.15],
        [yt, th - oh * 0.15],
        [y0, th - oh * 0.15],
    ];
    for s in [-1.0, 1.0] {
        let (a, b) = (s * gap / 2.0, s * (gap / 2.0 + ft));
        p.parts
            .push((sweep_x(&c, a.min(b), a.max(b)), Look::MachinePaint));
    }
    // a punch and die in place: forgecad's stand-in set, its sections along the bed
    let tpl = forge_brake_builder::templates();
    let mut tool_len = bl * 0.6;
    if let Some(stroke) = stroke {
        // the ram's travel, as a mark at the front
        p.mark(
            [bl / 2.0 + 40.0, -rf, ram_z],
            [bl / 2.0 + 40.0, -rf, ram_z - stroke],
            String::new(),
        );
    }
    for r in tpl.iter().filter(|r| r.kind == "die" || r.kind == "punch") {
        let Ok(secs) = forge_brake_builder::preview(r) else {
            continue;
        };
        for (_, o, _) in &secs {
            // the die on the table, the punch from the ram
            let (dz, flip) = if r.kind == "die" {
                (th, false)
            } else {
                (ram_z, true)
            };
            let yz: Vec<[f64; 2]> = o
                .iter()
                .map(|q| [q[0], if flip { dz - q[1].abs() } else { dz + q[1] }])
                .collect();
            p.parts
                .push((sweep_x(&yz, -tool_len / 2.0, tool_len / 2.0), Look::StandIn));
        }
        tool_len *= 1.0;
    }
    // labels: what is cited is the drawing; what is not, said once each
    p.mark(
        [-gap / 2.0 - ft, yt, th],
        [-gap / 2.0 - ft, (-bf).min(-rf), th],
        format!("Throat {} mm", forge_units::trim(throat)),
    );
    // the throat is cited: no dashes, only its label
    if let Some(g) = p.ghosts.last_mut() {
        g.edges.clear();
    }
    if !est.is_empty() {
        p.mark(
            [gap / 2.0 + ft, y1, 0.0],
            [gap / 2.0 + ft, y1, top],
            format!("Not published, drawn at an estimate: {}", est.join(", ")),
        );
    }
    p.mark(
        [0.0, -bf, th + 2.0],
        [tool_len / 2.0, -bf, th + 2.0],
        "Punch and die (brass tone) · forgecad's stand-in set",
    );
    if let Some(g) = p.ghosts.last_mut() {
        g.edges.clear();
    }
    // the floor it stands on: context, not the record
    let m = bl.max(fd) * 0.25;
    p.parts.push((
        prims_tris(&[Prim::Box {
            min: [-bl / 2.0 - ft - m, y0 - m * 2.0, -20.0],
            max: [bl / 2.0 + ft + m, y1 + m, 0.0],
        }]),
        Look::Floor,
    ));
    p.operator = true;
    p
}

/// A punch or die: forge-brake's exact sections, each swept along every segment of the set in
/// turn (segments laid end to end, 2 mm apart), along y.
pub fn brake_tool(secs: &[forge_brake_builder::Section], segments: &[(f64, u32)]) -> Preview {
    let mut p = Preview {
        long_axis: Some(1),
        ..Default::default()
    };
    let mut lens: Vec<f64> = segments
        .iter()
        .flat_map(|(l, n)| std::iter::repeat_n(*l, *n as usize))
        .collect();
    if lens.is_empty() {
        lens = vec![secs.iter().map(|s| s.2).fold(100.0, f64::max)];
    }
    let mut y = 0.0;
    for l in &lens {
        for (_, outline, _) in secs {
            p.parts.push((sweep(outline, y, y + l), Look::ToolSteel));
        }
        y += l + 2.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_end_mill_is_fluted_then_turned() {
        let o = [
            [0.0, 0.0],
            [0.0, 5.0],
            [25.0, 5.0],
            [25.0, 5.0],
            [72.0, 5.0],
            [72.0, 0.0],
        ];
        let p = cutter(&o, 25.0, 3, 30.0);
        assert_eq!(p.parts.len(), 2);
        let (f, look) = &p.parts[0];
        assert_eq!(*look, Look::Cutting);
        // the flutes: some points well inside the cutting radius, none outside it
        let rs: Vec<f64> = f.pos.iter().map(|q| (q[0] * q[0] + q[1] * q[1]).sqrt()).collect();
        assert!(rs.iter().all(|r| *r <= 5.0 + 1e-9));
        assert!(rs.iter().any(|r| *r < 4.0));
        // three cutting-edge lines plus the end ring
        assert_eq!(f.edges.len(), 4);
        let (lo, hi) = p.bounds().unwrap();
        assert!((hi[2] - 72.0).abs() < 1e-9 && lo[2].abs() < 1e-9);
    }

    #[test]
    fn a_ghost_has_no_faces() {
        let e = dashed_box([0.0; 3], [10.0, 10.0, 10.0]);
        assert!(e.len() > 12);
        assert!(e.iter().all(|d| d.len() == 2));
    }
}

#[cfg(test)]
mod c_frame_tests {
    /// A press housing's C (the throat cut in front): its side face covers the plate less the
    /// notch, so the throat shows.
    #[test]
    fn a_housing_c_has_its_throat_cut() {
        let c = [
            [-150.0, 0.0],
            [900.0, 0.0],
            [900.0, 2600.0],
            [-150.0, 2600.0],
            [-150.0, 1600.0],
            [330.0, 1600.0],
            [330.0, 900.0],
            [-150.0, 900.0],
        ];
        let t = super::sweep_x(&c, 875.0, 965.0);
        // the +x side face's triangles, in (y, z)
        let area: f64 = t
            .tri
            .iter()
            .filter(|tr| tr.iter().all(|&i| (t.pos[i as usize][0] - 965.0).abs() < 1e-9))
            .map(|tr| {
                let [a, b, c] = tr.map(|i| t.pos[i as usize]);
                0.5 * ((b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1])).abs()
            })
            .sum();
        let full = 1050.0 * 2600.0;
        let notch = 480.0 * 700.0;
        assert!((area - (full - notch)).abs() < 1e-6, "{area} vs {}", full - notch);
    }
}
