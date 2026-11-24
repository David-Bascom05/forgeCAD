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
}

impl Look {
    /// The appearance's name in the library.
    pub fn appearance(self) -> &'static str {
        match self {
            Look::ToolSteel => "Tool steel, ground",
            Look::Steel => "Steel, machined",
            Look::Cutting => "Steel, black oxide",
            Look::MachinePaint => "Paint, machine gloss",
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
    pub edges: Vec<Vec<[f64; 3]>>,
    /// Where its label points.
    pub at: [f64; 3],
    /// "height · unknown".
    pub label: String,
}

/// A record's preview.
#[derive(Clone, Debug, Default)]
pub struct Preview {
    pub parts: Vec<(Tris, Look)>,
    pub ghosts: Vec<Ghost>,
    /// A long tool's section, to show to scale beside the view: (outline x z, dimensions).
    pub section: Option<(Vec<[f64; 2]>, Vec<(String, String)>)>,
    /// Look along the length (a die, a punch, a press): the axis the subject is long on.
    pub long_axis: Option<usize>,
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
            .chain(self.ghosts.iter().flat_map(|g| g.edges.iter().flatten()));
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

    fn ghost_box(&mut self, min: [f64; 3], max: [f64; 3], label: impl Into<String>) {
        self.ghosts.push(Ghost {
            edges: dashed_box(min, max),
            at: [max[0], (min[1] + max[1]) / 2.0, max[2]],
            label: label.into(),
        });
    }

    fn ghost_line(&mut self, a: [f64; 3], b: [f64; 3], label: impl Into<String>) {
        self.ghosts.push(Ghost {
            edges: dashes(a, b, dist(a, b) / 24.0),
            at: b,
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

type Vals<'a> = &'a std::collections::BTreeMap<String, f64>;

fn why(r: &forge_workholding::Record, k: &str) -> &'static str {
    if r.conflict.iter().any(|x| x == k) {
        "sources disagree"
    } else {
        "unknown"
    }
}

/// A machine vise from its record. Its seat (the table) at z = 0, the clamp axis along x, the
/// fixed jaw at -x.
pub fn vise(r: &forge_workholding::Record) -> Preview {
    let v: Vals = &r.vals;
    let mut p = Preview::default();
    let g = |k: &str| v.get(k).copied().filter(|x| *x > 0.0);
    let jw = g("jaw_width");
    let jh = g("jaw_height");
    let jt = g("jaw_thickness");
    let mo = g("max_opening");
    let bl = g("body_length");
    let bw = g("body_width");
    let h = g("height");
    // the sizes a ghost is drawn at when its value is missing, from what is known
    let scale = [jw, bw, mo, bl]
        .into_iter()
        .flatten()
        .fold(0.0, f64::max)
        .max(50.0);
    let bw_d = bw.or(jw).unwrap_or(scale);
    let bl_d = bl.or(mo.map(|m| m * 1.8)).unwrap_or(scale * 2.0);
    let jh_d = jh.unwrap_or(scale * 0.3);
    let jt_d = jt.unwrap_or(scale * 0.12);
    let mo_d = mo.unwrap_or(scale);
    let jw_d = jw.unwrap_or(bw_d);
    // the base: up to the jaws' seat (height less the jaw height)
    let base_top = match (h, jh) {
        (Some(h), Some(jh)) if h > jh => Some(h - jh),
        _ => None,
    };
    let bt = base_top.unwrap_or(jh_d);
    let (x0, x1, y) = (-bl_d / 2.0, bl_d / 2.0, bw_d / 2.0);
    match (bl, bw, base_top) {
        (Some(_), Some(_), Some(_)) => p.solid_box([x0, -y, 0.0], [x1, y, bt], Look::MachinePaint),
        _ => {
            let mut missing = Vec::new();
            if bl.is_none() {
                missing.push(format!("body length · {}", why(r, "body_length")));
            }
            if bw.is_none() {
                missing.push(format!("body width · {}", why(r, "body_width")));
            }
            if base_top.is_none() {
                missing.push(format!("height · {}", why(r, "height")));
            }
            p.ghost_box(
                [x0, -y, 0.0],
                [x1, y, bt],
                format!("body: {}", missing.join(", ")),
            );
        }
    }
    // the jaws: the fixed one at the -x end, the moving one at the maximum opening
    let fixed_in = x0 + jt_d.max(bl_d * 0.08);
    let jaws = [
        (
            [fixed_in - jt_d, -jw_d / 2.0, bt],
            [fixed_in, jw_d / 2.0, bt + jh_d],
            "fixed jaw",
        ),
        (
            [fixed_in + mo_d, -jw_d / 2.0, bt],
            [fixed_in + mo_d + jt_d, jw_d / 2.0, bt + jh_d],
            "moving jaw",
        ),
    ];
    for (a, b, name) in jaws {
        if jw.is_some() && jh.is_some() && jt.is_some() && mo.is_some() {
            p.solid_box(a, b, Look::ToolSteel);
        } else {
            let mut missing = Vec::new();
            for (k, have) in [
                ("jaw_width", jw),
                ("jaw_height", jh),
                ("jaw_thickness", jt),
                ("max_opening", mo),
            ] {
                if have.is_none() {
                    missing.push(format!("{} · {}", k.replace('_', " "), why(r, k)));
                }
            }
            p.ghost_box(a, b, format!("{name}: {}", missing.join(", ")));
        }
    }
    // the screw is not in the record: its axis only
    let zc = bt * 0.5;
    p.ghost_line(
        [fixed_in + mo_d + jt_d, 0.0, zc],
        [x1 + bl_d * 0.12, 0.0, zc],
        "screw · not in the record",
    );
    p
}

/// A lathe chuck from its record: its face at z = 0, its body behind (-z), its jaws in front.
pub fn chuck(r: &forge_workholding::Record) -> Preview {
    let v: Vals = &r.vals;
    let mut p = Preview::default();
    let g = |k: &str| v.get(k).copied().filter(|x| *x > 0.0);
    let (bd, bl) = (g("body_d"), g("body_len"));
    let n = g("jaws").map_or(3, |n| n.round() as usize).clamp(2, 8);
    let (jl, gmax) = (g("jaw_len"), g("grip_max"));
    let bd_d = bd.or(gmax.map(|x| x * 1.3)).unwrap_or(200.0);
    match (bd, bl) {
        (Some(d), Some(l)) => p.solid_cyl([0.0, 0.0, -l], [0.0, 0.0, 1.0], d / 2.0, l, Look::Steel),
        _ => {
            let l = bl.unwrap_or(bd_d * 0.4);
            p.ghost_box(
                [-bd_d / 2.0, -bd_d / 2.0, -l],
                [bd_d / 2.0, bd_d / 2.0, 0.0],
                format!(
                    "body: {}",
                    [("body_d", bd), ("body_len", bl)]
                        .iter()
                        .filter(|(_, x)| x.is_none())
                        .map(|(k, _)| format!("{} · {}", k.replace('_', " "), why(r, k)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    }
    // the jaws open to the largest grip, out to the body's rim; their width is not published
    let r_in = gmax.map_or(bd_d * 0.15, |x| x / 2.0).min(bd_d * 0.45);
    let r_out = bd_d / 2.0;
    let len = jl.unwrap_or(bd_d * 0.15);
    let w = (r_out - r_in).max(1.0) * 0.5;
    for k in 0..n {
        let th = std::f64::consts::TAU * k as f64 / n as f64;
        let (sn, cs) = forge_math::dmath::sin_cos(th);
        // a jaw's box in its own radial frame, as its outline at the jaw's mid-width
        let pts: Vec<[f64; 3]> = [
            (r_in, -w / 2.0),
            (r_out, -w / 2.0),
            (r_out, w / 2.0),
            (r_in, w / 2.0),
        ]
        .iter()
        .flat_map(|(rr, t)| [0.0, len].map(|z| [rr * cs - t * sn, rr * sn + t * cs, z]))
        .collect();
        let mut edges = Vec::new();
        for (a, b) in [
            (0, 2),
            (2, 4),
            (4, 6),
            (6, 0),
            (1, 3),
            (3, 5),
            (5, 7),
            (7, 1),
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
        ] {
            edges.extend(dashes(pts[a], pts[b], len.max(w) / 10.0));
        }
        p.ghosts.push(Ghost {
            edges,
            at: pts[3],
            label: if k == 0 {
                format!(
                    "jaws: width · not in the record{}",
                    if jl.is_none() {
                        ", jaw length · unknown"
                    } else {
                        ""
                    }
                )
            } else {
                String::new()
            },
        });
    }
    p
}

// ---------------------------------------------------------------------------------------------
// press brakes

/// A press brake from its record (forge-brake's table): the floor at z = 0, the bed along x,
/// the operator at -y.
pub fn press(t: &forge_vendor::toml::Table) -> Preview {
    use crate::shell::machine_brake::get;
    use forge_vendor::toml::Item;
    let g = |k: &str| match get(t, k) {
        Some(Item::Num(x)) if *x > 0.0 => Some(*x),
        _ => None,
    };
    let mut p = Preview::default();
    p.long_axis = Some(0);
    let bl = g("bed_length");
    let gap = g("housing_gap");
    let oh = g("open_height");
    let th = g("table_height");
    let throat = g("throat");
    let (ft, fd) = (g("frame.thickness"), g("frame.depth"));
    let (rf, rb) = (g("ram.front"), g("ram.back"));
    let (bf, bb) = (g("bed.front"), g("bed.back"));
    let bl_d = bl.unwrap_or(3000.0);
    let th_d = th.unwrap_or(900.0);
    let oh_d = oh.unwrap_or(400.0);
    let gap_d = gap.unwrap_or(bl_d * 0.85);
    let beam = |front: Option<f64>, back: Option<f64>| (front.unwrap_or(60.0), back.unwrap_or(60.0));
    // the bed: its top at the table height
    let (f0, b0) = beam(bf, bb);
    let bed = ([-bl_d / 2.0, -f0, 0.0], [bl_d / 2.0, b0, th_d]);
    let missing = |ks: &[(&str, Option<f64>)]| {
        ks.iter()
            .filter(|(_, v)| v.is_none())
            .map(|(k, _)| format!("{k} · unknown"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let m = missing(&[
        ("bed_length", bl),
        ("table_height", th),
        ("bed.front", bf),
        ("bed.back", bb),
    ]);
    if m.is_empty() {
        p.solid_box(bed.0, bed.1, Look::MachinePaint);
    } else {
        p.ghost_box(bed.0, bed.1, format!("bed: {m}"));
    }
    // the ram: its underside at the open height above the table, its own height not published
    let (f1, b1) = beam(rf, rb);
    let ram_h = oh_d * 0.8;
    let ram = (
        [-bl_d / 2.0, -f1, th_d + oh_d],
        [bl_d / 2.0, b1, th_d + oh_d + ram_h],
    );
    let m = missing(&[
        ("bed_length", bl),
        ("open_height", oh),
        ("ram.front", rf),
        ("ram.back", rb),
    ]);
    p.ghost_box(
        ram.0,
        ram.1,
        if m.is_empty() {
            "ram: its height · not in the record".to_string()
        } else {
            format!("ram: {m}, its height · not in the record")
        },
    );
    // the side housings, the throat open in front between table and ram
    let (t_d, d_d) = (
        ft.unwrap_or(60.0),
        fd.or(throat.map(|x| x * 2.0)).unwrap_or(900.0),
    );
    let top = th_d + oh_d + ram_h;
    for s in [-1.0, 1.0] {
        let x_in = s * gap_d / 2.0;
        let x_out = s * (gap_d / 2.0 + t_d);
        let (xa, xb) = (x_in.min(x_out), x_in.max(x_out));
        let m = missing(&[("housing_gap", gap), ("frame.thickness", ft), ("frame.depth", fd)]);
        let box_ = ([xa, -f0.max(f1), 0.0], [xb, d_d - f0.max(f1), top]);
        if m.is_empty() && oh.is_some() && th.is_some() {
            // a C: below the table and above the ram solid, the throat's depth open in front
            let thr = throat.unwrap_or(0.0).min(d_d);
            let y0 = box_.0[1];
            p.solid_box([xa, y0, 0.0], [xb, box_.1[1], th_d], Look::MachinePaint);
            p.solid_box(
                [xa, y0 + thr, th_d],
                [xb, box_.1[1], th_d + oh_d],
                Look::MachinePaint,
            );
            p.ghost_box(
                [xa, y0, th_d + oh_d],
                [xb, box_.1[1], top],
                "housing above the ram · not in the record",
            );
        } else {
            p.ghost_box(
                box_.0,
                box_.1,
                if s < 0.0 {
                    format!("housing: {m}")
                } else {
                    String::new()
                },
            );
        }
    }
    p
}

/// A punch or die: forge-brake's exact sections, each swept along every segment of the set in
/// turn (segments laid end to end, 2 mm apart), along y.
pub fn brake_tool(secs: &[forge_brake_builder::Section], segments: &[(f64, u32)]) -> Preview {
    let mut p = Preview::default();
    p.long_axis = Some(1);
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
