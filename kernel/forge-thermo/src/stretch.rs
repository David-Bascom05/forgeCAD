//! The formed wall's thickness: the sheet as a membrane of incompressible hyperelastic material
//! (Ogden; neo-Hookean is one term of exponent 2), clamped round the window, draped over a male
//! tool as the clamp comes down and drawn onto the tool by the pressure, each point stopping where
//! it first touches (sticking contact, the rule every forming model keeps). Each load step is an
//! equilibrium found by L-BFGS on the membrane's energy plus the pressure's work; a point whose
//! step carries it into the tool stops where it crossed. The thickness is t0 times the area the
//! sheet had over the area it has. Isothermal, no friction slip, no plug: an estimate under a
//! stated model, best at saying where the sheet is thinnest. Design: research/thermoform/.
use crate::build::{Formed, Spec, Tool, inside, polygon};
use crate::field::Field;
use forge_math::{Point2, Point3, Vec3, dmath};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex, RwLock};

/// An incompressible Ogden law: W = sum mu_i / alpha_i (l1^a + l2^a + l3^a - 3), mu in MPa.
#[derive(Clone, Debug, PartialEq)]
pub struct Law {
    pub terms: Vec<(f64, f64)>,
}

impl Law {
    pub fn neo_hookean(mu: f64) -> Law {
        Law {
            terms: vec![(mu, 2.0)],
        }
    }
    /// The small-strain shear modulus: half the sum of mu_i alpha_i.
    pub fn shear(&self) -> f64 {
        0.5 * self.terms.iter().map(|(m, a)| m * a).sum::<f64>()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    pub law: Law,
    /// The forming pressure across the sheet, MPa (a vacuum is at most 0.1).
    pub pressure: f64,
    /// Sheet nodes along the window's longer side.
    pub resolution: usize,
    /// What the sheet meets: the tool's height over each cell, or its 3-D distance field.
    pub surface: Surface,
    /// Draw-in: the sheet lying on the tool's top (a female tool's, round the cavity) slides on it
    /// against Coulomb friction, this coefficient times the forming pressure pressing it there
    /// (None: it sticks where it lies, the reference).
    pub slide: Option<f64>,
}

/// The tool as the contact reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface {
    /// The height over each cell of the trim plane, the gap along the pull (the reference).
    #[default]
    Height,
    /// A 3-D signed distance field: the gap along the surface's normal (`sdf.rs`).
    Distance,
}

/// The gap from a point (on the plane's axes) to the tool.
enum Gap<'a> {
    Height(&'a Field),
    Distance(&'a crate::sdf::Sdf),
}

impl Gap<'_> {
    fn at(&self, p: Vec3) -> f64 {
        match self {
            Gap::Height(f) => p.z - height(f, Point2::new(p.x, p.y)),
            Gap::Distance(s) => s.distance(p),
        }
    }
}

impl Default for Process {
    fn default() -> Self {
        Process {
            law: Law::neo_hookean(0.5),
            pressure: 0.09,
            resolution: 56,
            surface: Surface::Height,
            slide: None,
        }
    }
}

/// The formed sheet: its nodes where they ended (world), its triangles, each triangle's
/// thickness over the sheet's, and the extremes over the part (inside the trim).
#[derive(Clone, Debug, PartialEq)]
pub struct Stretch {
    pub nodes: Vec<Point3>,
    /// The sheet after each load step, the last the result.
    pub steps: Vec<Step>,
    /// The nodes are a grid, (nx + 1) by (ny + 1), row by row from the window's corner.
    pub grid: (usize, usize),
    pub tris: Vec<[u32; 3]>,
    pub ratio: Vec<f64>,
    /// Each node's thickness over the sheet's (its triangles' by area), and whether it touched.
    pub node_ratio: Vec<f64>,
    pub touched: Vec<bool>,
    /// Over the nodes inside the trim that touched the tool: the thinnest and thickest, and where.
    pub min: (f64, Point3),
    pub max: (f64, Point3),
    pub mean: f64,
    /// Points inside the trim the sheet never reached (it bridges there).
    pub unformed: Vec<Point3>,
    /// L-BFGS iterations spent, all steps.
    pub iterations: usize,
}

#[derive(Clone)]
struct Tri {
    v: [usize; 3],
    /// The inverse of the reference edge matrix, and the reference volume.
    dinv: [[f64; 2]; 2],
    vol: f64,
}

/// How far past the forming pressure the creep is followed.
const CREEP: f64 = 50.0;

fn pw(x: f64, a: f64) -> f64 {
    if a == 2.0 {
        x * x
    } else if a == -2.0 {
        1.0 / (x * x)
    } else if a == 1.0 {
        x
    } else {
        dmath::powf(x, a)
    }
}

/// Energy of one triangle and its gradient at its second and third corners (the first's is
/// minus both), or None where it is turned inside out.
fn element(law: &Law, t: &Tri, x: &[Vec3]) -> Option<(f64, Vec3, Vec3)> {
    let (x0, x1, x2) = (x[t.v[0]], x[t.v[1]], x[t.v[2]]);
    let (e1, e2) = (x1 - x0, x2 - x0);
    let f0 = e1 * t.dinv[0][0] + e2 * t.dinv[1][0];
    let f1 = e1 * t.dinv[0][1] + e2 * t.dinv[1][1];
    let (a, b, c) = (f0.dot(f0), f0.dot(f1), f1.dot(f1));
    let det = a * c - b * b;
    if det.is_nan() || det <= 1e-12 {
        return None;
    }
    let half = 0.5 * (a - c);
    let disc = (half * half + b * b).sqrt();
    let s1 = 0.5 * (a + c) + disc;
    let s2 = det / s1;
    let (l1, l2) = (s1.sqrt(), s2.sqrt());
    let l3 = 1.0 / (l1 * l2);
    let mut w = 0.0;
    let (mut g1, mut g2) = (0.0, 0.0);
    // a stretch's logarithm once, each term's power from it
    let logs = law
        .terms
        .iter()
        .any(|t| !matches!(t.1, 1.0 | 2.0 | -2.0))
        .then(|| (dmath::ln(l1), dmath::ln(l2)));
    for &(mu, al) in &law.terms {
        let (p1, p2, p3) = match logs {
            Some((a1, a2)) if !matches!(al, 1.0 | 2.0 | -2.0) => (
                dmath::exp(al * a1),
                dmath::exp(al * a2),
                dmath::exp(-al * (a1 + a2)),
            ),
            _ => (pw(l1, al), pw(l2, al), pw(l3, al)),
        };
        w += mu / al * (p1 + p2 + p3 - 3.0);
        g1 += mu * (p1 - p3) / l1;
        g2 += mu * (p2 - p3) / l2;
    }
    // S = sum (dW/dl_a / l_a) N_a N_a^T
    let (k1, k2) = (g1 / l1, g2 / l2);
    let (sxx, sxy, syy) = if disc <= 1e-12 * (a + c) {
        let k = 0.5 * (k1 + k2);
        (k, 0.0, k)
    } else {
        // the first eigenvector, from whichever row is better conditioned
        let (vx, vy) = if (s1 - a).abs() > (s1 - c).abs() {
            (b, s1 - a)
        } else {
            (s1 - c, b)
        };
        let n = (vx * vx + vy * vy).sqrt();
        let (nx, ny) = (vx / n, vy / n);
        let (pxx, pxy, pyy) = (nx * nx, nx * ny, ny * ny);
        (
            k1 * pxx + k2 * (1.0 - pxx),
            (k1 - k2) * pxy,
            k1 * pyy + k2 * (1.0 - pyy),
        )
    };
    let p0 = (f0 * sxx + f1 * sxy) * t.vol;
    let p1 = (f0 * sxy + f1 * syy) * t.vol;
    let ga = p0 * t.dinv[0][0] + p1 * t.dinv[0][1];
    let gb = p0 * t.dinv[1][0] + p1 * t.dinv[1][1];
    Some((t.vol * w, ga, gb))
}

/// The triangles' energies on all cores: workers kept for the whole solve, each its share of the
/// triangles per call; the answers come back in triangle order, so they are summed as one thread
/// would sum them.
struct Pool {
    shared: Arc<Shared>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

/// A triangle's energy and its gradient at its second and third corners.
type Part = (f64, Vec3, Vec3);

struct Shared {
    law: Law,
    tris: Vec<Tri>,
    x: RwLock<Vec<Vec3>>,
    stop: AtomicBool,
    start: Barrier,
    done: Barrier,
    out: Vec<Mutex<Vec<Option<Part>>>>,
}

impl Shared {
    fn share(&self, w: usize) -> std::ops::Range<usize> {
        let n = self.out.len();
        w * self.tris.len() / n..(w + 1) * self.tris.len() / n
    }

    fn run(&self, w: usize, x: &[Vec3]) {
        let mut o = self.out[w].lock().unwrap_or_else(|e| e.into_inner());
        o.clear();
        o.extend(self.tris[self.share(w)].iter().map(|t| element(&self.law, t, x)));
    }
}

impl Pool {
    /// Below this many triangles a call is not worth sharing out.
    const MIN: usize = 1024;

    fn new(law: &Law, tris: &[Tri]) -> Pool {
        let n = forge_math::par::threads().min(tris.len() / 256).max(1);
        let shared = Arc::new(Shared {
            law: law.clone(),
            tris: tris.to_vec(),
            x: RwLock::new(vec![]),
            stop: AtomicBool::new(false),
            start: Barrier::new(n),
            done: Barrier::new(n),
            out: (0..n).map(|_| Mutex::new(vec![])).collect(),
        });
        let workers = if n > 1 && tris.len() >= Self::MIN {
            (1..n)
                .map(|w| {
                    let s = shared.clone();
                    std::thread::spawn(move || {
                        loop {
                            s.start.wait();
                            if s.stop.load(Ordering::Acquire) {
                                return;
                            }
                            let x = s.x.read().unwrap_or_else(|e| e.into_inner());
                            s.run(w, &x);
                            drop(x);
                            s.done.wait();
                        }
                    })
                })
                .collect()
        } else {
            vec![]
        };
        Pool { shared, workers }
    }

    /// Each triangle's energy and gradient at `x`, in order (None: one is turned inside out).
    fn each(&self, x: &[Vec3], f: &mut dyn FnMut(usize, (f64, Vec3, Vec3))) -> Option<()> {
        let s = &self.shared;
        if self.workers.is_empty() {
            for (k, t) in s.tris.iter().enumerate() {
                f(k, element(&s.law, t, x)?);
            }
            return Some(());
        }
        {
            let mut shared_x = s.x.write().unwrap_or_else(|e| e.into_inner());
            shared_x.clear();
            shared_x.extend_from_slice(x);
        }
        s.start.wait();
        s.run(0, x);
        s.done.wait();
        let mut k = 0;
        for o in &s.out {
            for e in o.lock().unwrap_or_else(|e| e.into_inner()).iter() {
                f(k, (*e)?);
                k += 1;
            }
        }
        Some(())
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        if self.workers.is_empty() {
            return;
        }
        self.shared.stop.store(true, Ordering::Release);
        self.shared.start.wait();
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

struct Sheet {
    pool: Pool,
    tris: Vec<Tri>,
    idx: Vec<[usize; 3]>,
    free: Vec<bool>,
    /// Nodes sliding on the tool's top: held to its plane, each against its friction force
    /// (N) from where it stood at the load step's start.
    slide: Vec<bool>,
    hold: Vec<f64>,
    anchor: Vec<Vec3>,
}

/// Sliding nodes carried off the tool's top (over the cavity) fall free.
fn release(sheet: &mut Sheet, tool: &Gap, x: &[Vec3], off: f64) {
    for (s, p) in sheet.slide.iter_mut().zip(x) {
        if *s && tool.at(*p) > off {
            *s = false;
        }
    }
}

/// The friction's smoothing length, mm: below it the hold grows like a spring, past it it is the
/// Coulomb force (the lagged, smoothed friction of an incremental potential).
const SLIP: f64 = 0.02;

impl Sheet {
    /// The membrane's energy plus the pressure's work (p times the signed volume under the
    /// sheet), and its gradient on the free nodes.
    fn energy(&self, x: &[Vec3], p: f64, mut g: Option<&mut Vec<Vec3>>) -> Option<f64> {
        if let Some(g) = g.as_deref_mut() {
            g.iter_mut().for_each(|v| *v = Vec3::ZERO);
        }
        let mut e = 0.0;
        self.pool.each(x, &mut |k, (w, ga, gb)| {
            let v = &self.idx[k];
            e += w;
            if let Some(g) = g.as_deref_mut() {
                let t = &self.tris[k];
                g[t.v[1]] += ga;
                g[t.v[2]] += gb;
                g[t.v[0]] = g[t.v[0]] - ga - gb;
            }
            if p != 0.0 {
                let (a, b, c) = (x[v[0]], x[v[1]], x[v[2]]);
                e += p / 6.0 * a.dot(b.cross(c));
                if let Some(g) = g.as_deref_mut() {
                    g[v[0]] += b.cross(c) * (p / 6.0);
                    g[v[1]] += c.cross(a) * (p / 6.0);
                    g[v[2]] += a.cross(b) * (p / 6.0);
                }
            }
        })?;
        for k in 0..x.len() {
            if !self.slide[k] {
                continue;
            }
            let d = Vec3::new(x[k].x - self.anchor[k].x, x[k].y - self.anchor[k].y, 0.0);
            let l = (d.length_sq() + SLIP * SLIP).sqrt();
            e += self.hold[k] * (l - SLIP);
            if let Some(g) = g.as_deref_mut() {
                g[k] += d * (self.hold[k] / l);
            }
        }
        if let Some(g) = g {
            for (k, (gi, &f)) in g.iter_mut().zip(&self.free).enumerate() {
                if !f {
                    *gi = Vec3::ZERO;
                } else if self.slide[k] {
                    // held to the tool's top
                    gi.z = 0.0;
                }
            }
        }
        Some(e)
    }
}

/// The tool's height under a point (bilinear between cell centres).
fn height(field: &Field, p: Point2) -> f64 {
    let fx = ((p.x - field.x0) / field.h - 0.5).clamp(0.0, (field.nx - 1) as f64);
    let fy = ((p.y - field.y0) / field.h - 0.5).clamp(0.0, (field.ny - 1) as f64);
    let (i, j) = (
        (fx as usize).min(field.nx.saturating_sub(2)),
        (fy as usize).min(field.ny.saturating_sub(2)),
    );
    let (u, v) = (fx - i as f64, fy - j as f64);
    let z = |i: usize, j: usize| {
        let z = field.z[(j.min(field.ny - 1)) * field.nx + i.min(field.nx - 1)];
        if z.is_nan() { 0.0 } else { z }
    };
    let (a, b, c, d) = (z(i, j), z(i + 1, j), z(i, j + 1), z(i + 1, j + 1));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

/// Each triangle's thickness over the sheet's (its first area over its area now), and each
/// node's, its triangles' by first area.
fn ratios(idx: &[[usize; 3]], reference: &[Vec3], x: &[Vec3]) -> (Vec<f64>, Vec<f64>) {
    let area = |v: &[usize; 3], x: &[Vec3]| 0.5 * (x[v[1]] - x[v[0]]).cross(x[v[2]] - x[v[0]]).length();
    let ratio: Vec<f64> = idx
        .iter()
        .map(|v| area(v, reference) / area(v, x).max(1e-12))
        .collect();
    let mut node = vec![0.0; x.len()];
    let mut weight = vec![0.0; x.len()];
    for (v, r) in idx.iter().zip(&ratio) {
        let a = area(v, reference);
        for &k in v {
            node[k] += a * r;
            weight[k] += a;
        }
    }
    for (r, w) in node.iter_mut().zip(&weight) {
        *r /= w.max(1e-12);
    }
    (ratio, node)
}

fn to_world(f: &forge_math::Frame, p: [f32; 3]) -> [f32; 3] {
    let w = f.point_to_world(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64));
    [w.x as f32, w.y as f32, w.z as f32]
}

/// The sheet as it stood after a load step, for watching it form: its nodes (world), each
/// node's thickness over the sheet's, and the stage (drape, vacuum, creep).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub nodes: Vec<[f32; 3]>,
    pub ratio: Vec<f32>,
    pub stage: &'static str,
}

/// Where the move from `a` to `b` first comes within `reach` of the tool, if it does.
fn crossing(field: &Gap, a: Vec3, b: Vec3, reach: f64) -> Option<Vec3> {
    let n = 8;
    let mut lo = a;
    for k in 1..=n {
        let hi = a + (b - a) * (k as f64 / n as f64);
        if field.at(hi) <= reach {
            let mut hi = hi;
            for _ in 0..30 {
                let m = (lo + hi) * 0.5;
                if field.at(m) <= reach {
                    hi = m;
                } else {
                    lo = m;
                }
            }
            return Some(lo);
        }
        lo = hi;
    }
    None
}

/// Minimises the sheet's energy at pressure `p` over its free nodes, stopping each node that
/// crosses into the tool where it crosses. Returns the iterations spent.
#[allow(clippy::too_many_arguments)]
fn relax(
    sheet: &mut Sheet,
    field: &Gap,
    x: &mut [Vec3],
    p: f64,
    step_cap: f64,
    tol: f64,
    reach: &[f64],
    max_it: usize,
) -> usize {
    const M: usize = 6;
    let n = x.len();
    let mut g = vec![Vec3::ZERO; n];
    let Some(mut e) = sheet.energy(x, p, Some(&mut g)) else {
        return 0;
    };
    let (mut ss, mut ys): (Vec<Vec<Vec3>>, Vec<Vec<Vec3>>) = (vec![], vec![]);
    let dot = |a: &[Vec3], b: &[Vec3]| a.iter().zip(b).map(|(u, v)| u.dot(*v)).sum::<f64>();
    let mut g2 = vec![Vec3::ZERO; n];
    let mut trial = x.to_vec();
    for it in 0..max_it {
        let gmax = g.iter().map(|v| v.length()).fold(0.0, f64::max);
        if gmax <= tol {
            return it;
        }
        // two-loop recursion
        let mut q = g.clone();
        let mut al = vec![0.0; ss.len()];
        for k in (0..ss.len()).rev() {
            let rho = 1.0 / dot(&ys[k], &ss[k]);
            al[k] = rho * dot(&ss[k], &q);
            for (qi, yi) in q.iter_mut().zip(&ys[k]) {
                *qi -= *yi * al[k];
            }
        }
        let gamma = match (ss.last(), ys.last()) {
            (Some(s), Some(y)) => dot(s, y) / dot(y, y),
            _ => step_cap / gmax,
        };
        q.iter_mut().for_each(|v| *v = *v * gamma);
        for k in 0..ss.len() {
            let rho = 1.0 / dot(&ys[k], &ss[k]);
            let beta = rho * dot(&ys[k], &q);
            for (qi, si) in q.iter_mut().zip(&ss[k]) {
                *qi += *si * (al[k] - beta);
            }
        }
        let mut d: Vec<Vec3> = q.iter().map(|v| -*v).collect();
        let mut slope = dot(&d, &g);
        if slope.is_nan() || slope >= 0.0 {
            d = g.iter().map(|v| -*v * (step_cap / gmax)).collect();
            slope = dot(&d, &g);
            ss.clear();
            ys.clear();
        }
        // no node moves more than the cap in one step
        let dmax = d.iter().map(|v| v.length()).fold(0.0, f64::max);
        let mut a = if dmax > step_cap { step_cap / dmax } else { 1.0 };
        let mut taken = None;
        for _ in 0..30 {
            for i in 0..n {
                trial[i] = x[i] + d[i] * a;
            }
            if let Some(e2) = sheet.energy(&trial, p, Some(&mut g2))
                && e2 <= e + 1e-4 * a * slope
            {
                taken = Some(e2);
                break;
            }
            a *= 0.5;
        }
        let Some(mut e2) = taken else { return it };
        // contact: a node carried into the tool stops where it crossed, and stays
        let mut stopped = false;
        for i in 0..n {
            if sheet.free[i]
                && !sheet.slide[i]
                && let Some(c) = crossing(field, x[i], trial[i], reach[i])
            {
                trial[i] = c;
                sheet.free[i] = false;
                stopped = true;
            }
        }
        if stopped {
            match sheet.energy(&trial, p, Some(&mut g2)) {
                Some(v) => e2 = v,
                None => return it,
            }
            ss.clear();
            ys.clear();
        } else {
            ss.push((0..n).map(|i| trial[i] - x[i]).collect());
            ys.push((0..n).map(|i| g2[i] - g[i]).collect());
            if ss.len() > M {
                ss.remove(0);
                ys.remove(0);
            }
            if dot(ys.last().expect("pushed"), ss.last().expect("pushed")) <= 0.0 {
                ss.clear();
                ys.clear();
            }
        }
        x.copy_from_slice(&trial);
        std::mem::swap(&mut g, &mut g2);
        e = e2;
    }
    max_it
}

/// The thickness the sheet comes to over the tool `f` formed with `spec` under `pr`.
pub fn stretch(f: &Formed, spec: &Spec, pr: &Process) -> Stretch {
    let male = spec.tool == Tool::Male;
    let (tlo, thi) = f.report.trim_box;
    // the clamp window: the tool's outline
    let m = spec.margin.max(spec.t);
    let (wlo, whi) = (
        Point2::new(tlo.x - m, tlo.y - m),
        Point2::new(thi.x + m, thi.y + m),
    );
    let field = Field::new(f.frame, wlo, whi, &f.mesh, &f.contact, male, |_| true);
    let sdf = (pr.surface == Surface::Distance).then(|| {
        // nodes half the sheet apart (a quarter of a millimetre to a millimetre), exact a sheet
        // and a half either side of the surface
        let h = (0.5 * spec.t).clamp(0.25, 1.0);
        crate::sdf::Sdf::new(
            f.frame,
            &f.mesh,
            &f.contact,
            male,
            field.clone(),
            wlo,
            whi,
            h,
            1.5 * spec.t,
        )
    });
    let tool = match &sdf {
        Some(s) => Gap::Distance(s),
        None => Gap::Height(&field),
    };
    let (w, h) = (whi.x - wlo.x, whi.y - wlo.y);
    let res = pr.resolution.clamp(8, 160);
    let s = w.max(h) / res as f64;
    let (nx, ny) = ((w / s).ceil() as usize, (h / s).ceil() as usize);
    let (sx, sy) = (w / nx as f64, h / ny as f64);
    let id = |i: usize, j: usize| j * (nx + 1) + i;
    let mut x = vec![Vec3::ZERO; (nx + 1) * (ny + 1)];
    let mut free = vec![true; x.len()];
    let top = field
        .z
        .iter()
        .copied()
        .filter(|z| !z.is_nan())
        .fold(0.0, f64::max);
    let z_start = if male { top + s } else { 0.0 };
    for j in 0..=ny {
        for i in 0..=nx {
            let k = id(i, j);
            x[k] = Vec3::new(wlo.x + i as f64 * sx, wlo.y + j as f64 * sy, z_start);
            if i == 0 || j == 0 || i == nx || j == ny {
                free[k] = false;
            }
        }
    }
    let reference = x.clone();
    let edge: Vec<usize> = (0..x.len()).filter(|&k| !free[k]).collect();
    let mut idx = vec![];
    for j in 0..ny {
        for i in 0..nx {
            let (a, b, c, d) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
            // diagonals alternating, so the mesh has no grain
            if (i + j) % 2 == 0 {
                idx.push([a, b, c]);
                idx.push([a, c, d]);
            } else {
                idx.push([a, b, d]);
                idx.push([b, c, d]);
            }
        }
    }
    let tris: Vec<Tri> = idx
        .iter()
        .map(|v| {
            let (p0, p1, p2) = (reference[v[0]], reference[v[1]], reference[v[2]]);
            let (a, b, c, d) = (p1.x - p0.x, p2.x - p0.x, p1.y - p0.y, p2.y - p0.y);
            let det = a * d - b * c;
            Tri {
                v: *v,
                dinv: [[d / det, -b / det], [-c / det, a / det]],
                vol: 0.5 * det * spec.t,
            }
        })
        .collect();
    // a female sheet lies on the tool's top round the cavity from the start: stuck there, or
    // sliding on it
    let mut slide = vec![false; x.len()];
    for k in 0..x.len() {
        if free[k] && tool.at(x[k]) <= 0.0 {
            if pr.slide.is_some() {
                slide[k] = true;
            } else {
                free[k] = false;
            }
        }
    }
    // each node's share of the sheet, for its friction
    let mut share = vec![0.0; x.len()];
    for v in &idx {
        let a = 0.5
            * (reference[v[1]] - reference[v[0]])
                .cross(reference[v[2]] - reference[v[0]])
                .length();
        v.iter().for_each(|&k| share[k] += a / 3.0);
    }
    let hold_at = |p: f64| -> Vec<f64> { share.iter().map(|a| pr.slide.unwrap_or(0.0) * p * a).collect() };
    let mut sheet = Sheet {
        pool: Pool::new(&pr.law, &tris),
        tris,
        idx: idx.clone(),
        free,
        hold: hold_at(0.0),
        anchor: x.clone(),
        slide,
    };
    let mu = pr.law.shear().max(1e-9);
    let tol = 1e-5 * mu * spec.t * s;
    let cap = 0.3 * s;
    // the height field: a point within a quarter of the sheet's thickness above the tool is on it
    // (a sheet cannot follow the tool into a corner much tighter than it is thick); the distance
    // field: the sheet's middle is on the tool when half its thickness now, from each node's
    // triangles, is off it along the tool's normal
    let reach_at = |x: &[Vec3]| -> Vec<f64> {
        match pr.surface {
            Surface::Height => vec![0.25 * spec.t; x.len()],
            Surface::Distance => ratios(&idx, &reference, x)
                .1
                .iter()
                .map(|r| 0.5 * spec.t * r)
                .collect(),
        }
    };
    let mut its = 0;
    let mut steps: Vec<Step> = vec![];
    let mut snap = |x: &[Vec3], stage: &'static str| {
        let (_, r) = ratios(&idx, &reference, x);
        steps.push(Step {
            nodes: x.iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]).collect(),
            ratio: r.iter().map(|&v| v as f32).collect(),
            stage,
        });
    };
    snap(&x, "sheet");
    if male {
        // the clamp comes down over the tool to the table
        let drop = 6;
        for k in 1..=drop {
            let z = z_start * (1.0 - k as f64 / drop as f64);
            for &k in &edge {
                x[k].z = z;
            }
            sheet.anchor.copy_from_slice(&x);
            sheet.hold = hold_at(0.0);
            let reach = reach_at(&x);
            its += relax(&mut sheet, &tool, &mut x, 0.0, cap, tol, &reach, 150);
            release(&mut sheet, &tool, &x, 0.25 * spec.t);
            snap(&x, "drape");
        }
    }
    let loads = 10;
    for k in 1..=loads {
        let p = pr.pressure * k as f64 / loads as f64;
        sheet.anchor.copy_from_slice(&x);
        sheet.hold = hold_at(p);
        let reach = reach_at(&x);
        its += relax(&mut sheet, &tool, &mut x, p, cap, tol, &reach, 400);
        release(&mut sheet, &tool, &x, 0.25 * spec.t);
        snap(&x, "vacuum");
    }
    // a hot sheet held under the pressure creeps on until it meets the tool: an elastic sheet
    // gets there only at a higher one, so the pressure is raised while some of the part is free
    let poly = polygon(&f.trim);
    let open = |sheet: &Sheet, x: &[Vec3]| {
        (0..x.len())
            .filter(|&k| sheet.free[k] && inside(&poly, Point2::new(x[k].x, x[k].y)).0)
            .count()
    };
    // until it is formed, or two raises in a row reach no more of it: it bridges there
    let mut p = pr.pressure;
    let (mut left, mut stalled) = (open(&sheet, &x), 0);
    while left > 0 && stalled < 2 && p < CREEP * pr.pressure {
        p *= 1.25;
        sheet.anchor.copy_from_slice(&x);
        sheet.hold = hold_at(p);
        let reach = reach_at(&x);
        its += relax(&mut sheet, &tool, &mut x, p, cap, tol, &reach, 200);
        release(&mut sheet, &tool, &x, 0.25 * spec.t);
        snap(&x, "creep");
        let now = open(&sheet, &x);
        stalled = if now < left { 0 } else { stalled + 1 };
        left = now;
    }
    let (ratio, node_ratio) = ratios(&idx, &reference, &x);
    let world = |p: Vec3| f.frame.point_to_world(Point3::new(p.x, p.y, p.z));
    steps
        .iter_mut()
        .for_each(|s| s.nodes.iter_mut().for_each(|p| *p = to_world(&f.frame, *p)));
    let (mut min, mut max) = (
        (f64::INFINITY, Point3::ORIGIN),
        (f64::NEG_INFINITY, Point3::ORIGIN),
    );
    // over the sheet on the tool: where it bridges it is said apart (`unformed`)
    for (k, &r) in node_ratio.iter().enumerate() {
        if sheet.free[k] || !inside(&poly, Point2::new(x[k].x, x[k].y)).0 {
            continue;
        }
        if r < min.0 {
            min = (r, world(x[k]));
        }
        if r > max.0 {
            max = (r, world(x[k]));
        }
    }
    let (mut sum, mut total) = (0.0, 0.0);
    for (v, &r) in idx.iter().zip(&ratio) {
        let c = (x[v[0]] + x[v[1]] + x[v[2]]) * (1.0 / 3.0);
        if inside(&poly, Point2::new(c.x, c.y)).0 {
            let a = 0.5
                * (reference[v[1]] - reference[v[0]])
                    .cross(reference[v[2]] - reference[v[0]])
                    .length();
            sum += a;
            total += a / r;
        }
    }
    let touched: Vec<bool> = sheet.free.iter().map(|f| !f).collect();
    let unformed = (0..x.len())
        .filter(|&k| !touched[k] && inside(&poly, Point2::new(x[k].x, x[k].y)).0)
        .map(|k| world(x[k]))
        .collect();
    Stretch {
        nodes: x.iter().map(|&p| world(p)).collect(),
        steps,
        grid: (nx, ny),
        tris: idx.iter().map(|v| v.map(|k| k as u32)).collect(),
        ratio,
        node_ratio,
        touched,
        min,
        max,
        mean: if total > 0.0 { sum / total } else { 1.0 },
        unformed,
        iterations: its,
    }
}
