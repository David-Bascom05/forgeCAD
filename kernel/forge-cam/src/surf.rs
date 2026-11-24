//! 3-axis surfacing: toolpaths over the part's faces by the certified drop-cutter
//! (forge_cam_geom::drop).
//!
//! The generators here need not be proven correct, only safe (design.md §5.3, L4): every path
//! they make is certified afterwards by the checker (`ops::certify_with`, column route on), and an
//! op is shipped only when every check is Clear. To make that likely, each generator checks its
//! own cutting moves with the same vertical-gap test at half the op's tolerance, and splits a
//! move (with a new drop-cutter point at its middle) where it cannot prove it.
//!
//! - `parallel`: raster (zig-zag) finishing, rows at a stepover along a direction.
//! - `waterline`: constant-z contours of the column's reach: the boundary, at height z, of
//!   {(x, y) : CL(x, y) > z}, found by certified bisection of the drop-cutter (push-cutter).
//! - `zlevel`: z-level roughing (offset clearing at each level of the tool-centre region the
//!   column leaves free, from a grid of CL heights: not adaptive / constant engagement).
//! - `pencil`: the creases of the CL surface (where the tool touches two faces at once).
//! - `rest`: parallel passes only where a larger earlier tool's CL stood more than a threshold
//!   above this tool's.
//! - `ZMap`: the in-process stock as a height map whose every cell's top is at or above the
//!   material there (so a clearance against it is sound); its faces for the checker.
use crate::ops::{Cut, OpError, OpPath, PMove, Phase};
use forge_cam_geom::check::{Face, Move, Surf, TSeg, Trim};
use forge_cam_geom::drop::{self, Cl, Column, Gap};
use forge_cam_geom::ival::Ival;

/// The faces whose xy box meets a cell of a uniform grid.
#[derive(Clone, Debug)]
pub struct XyGrid {
    x0: f64,
    y0: f64,
    h: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

impl XyGrid {
    pub fn new(faces: &[Face], pad: f64) -> XyGrid {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for f in faces {
            for k in 0..2 {
                lo[k] = lo[k].min(f.bbox[k].lo - pad - f.mu);
                hi[k] = hi[k].max(f.bbox[k].hi + pad + f.mu);
            }
        }
        if !(lo[0] < hi[0] && lo[1] < hi[1]) {
            return XyGrid {
                x0: 0.0,
                y0: 0.0,
                h: 1.0,
                nx: 1,
                ny: 1,
                cells: vec![Vec::new()],
            };
        }
        let ext = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let h = (ext / 64.0).max(pad).max(1e-6);
        let nx = (((hi[0] - lo[0]) / h).ceil() as usize).max(1);
        let ny = (((hi[1] - lo[1]) / h).ceil() as usize).max(1);
        let mut g = XyGrid {
            x0: lo[0],
            y0: lo[1],
            h,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
        };
        for (i, f) in faces.iter().enumerate() {
            let (a, b) = g.range(
                f.bbox[0].lo - pad - f.mu,
                f.bbox[1].lo - pad - f.mu,
                f.bbox[0].hi + pad + f.mu,
                f.bbox[1].hi + pad + f.mu,
            );
            for iy in a[1]..=b[1] {
                for ix in a[0]..=b[0] {
                    g.cells[iy * nx + ix].push(i as u32);
                }
            }
        }
        g
    }

    fn range(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> ([usize; 2], [usize; 2]) {
        let c = |v: f64, o: f64, n: usize| -> usize {
            let k = ((v - o) / self.h).floor();
            if k < 0.0 {
                0
            } else if k >= n as f64 {
                n - 1
            } else {
                k as usize
            }
        };
        (
            [c(x0, self.x0, self.nx), c(y0, self.y0, self.ny)],
            [c(x1, self.x0, self.nx), c(y1, self.y0, self.ny)],
        )
    }

    /// The faces registered in cells meeting the box (a superset of those whose padded box
    /// meets it), sorted, without repeats.
    pub fn near(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<u32> {
        let (a, b) = self.range(x0, y0, x1, y1);
        let mut out = Vec::new();
        for iy in a[1]..=b[1] {
            for ix in a[0]..=b[0] {
                out.extend_from_slice(&self.cells[iy * self.nx + ix]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Why a surfacing op could not be made.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfError {
    /// The tool's cutter has no column (not a flat, ball, bull, chamfer or drill end).
    NoColumn,
    /// A move the generator could not prove clear even after splitting it to `len`: where.
    Stuck { at: [f64; 3], len: f64 },
    /// A parameter out of range: which.
    Bad(&'static str),
}

impl From<SurfError> for OpError {
    fn from(e: SurfError) -> OpError {
        OpError::Region(format!("surfacing: {e:?}"))
    }
}

/// The drop-cutter over a part, for one tool.
pub struct Surfacer<'a> {
    pub faces: &'a [Face],
    /// The cutter's body (for the Euclidean check where the column's is too strict: side contact).
    pub cutter: forge_cam_geom::tool::Body,
    /// The cutter's own column (for the checks).
    pub check: Column,
    /// The column the paths are made with: the cutter grown by the leave.
    pub make: Column,
    pub leave: f64,
    grid: XyGrid,
    /// The CL resolution (hi − lo ≤ res).
    pub res: f64,
    /// The lowest the tool goes where no face is in reach.
    pub floor: f64,
    /// The op's tolerance (the cutter may come within it of the part).
    pub tol: f64,
    pub budget: usize,
}

impl<'a> Surfacer<'a> {
    pub fn new(
        faces: &'a [Face],
        cutter: &forge_cam_geom::tool::Body,
        leave: f64,
        tol: f64,
        floor: f64,
    ) -> Result<Surfacer<'a>, SurfError> {
        let check = Column::of(cutter).ok_or(SurfError::NoColumn)?;
        if tol.is_nan() || tol <= 0.0 {
            return Err(SurfError::Bad("tol"));
        }
        // the path is made with the cutter grown by the leave and widened a quarter of the
        // tolerance past its rim: a tool exactly tangent to a wall would make the drop jump (the
        // footprint is closed at R, and intervals cannot tell ρ = R) and its moves along the wall
        // unprovable; this keeps it a little off tall walls instead
        let grown = check.grow(leave).ok_or(SurfError::Bad("leave"))?;
        // (an off-round width, so grids laid out from round numbers do not land on its rim)
        let make = grown.widened(0.2371 * tol).ok_or(SurfError::Bad("tol"))?;
        let grid = XyGrid::new(faces, make.r);
        Ok(Surfacer {
            faces,
            cutter: cutter.clone(),
            check,
            make,
            leave,
            grid,
            res: tol / 8.0,
            floor,
            tol,
            budget: 200_000,
        })
    }

    /// The certified CL height of the generating column at (x, y), at least the floor.
    pub fn cl(&self, x: f64, y: f64) -> Cl {
        let r = self.make.r;
        let idx = self.grid.near(x - r, y - r, x + r, y + r);
        // a drop left unresolved keeps its upper bound: safe, at worst high (a rim exactly
        // tangent to a wall, which intervals cannot settle)
        let mut c = drop::drop_above(
            &self.make,
            self.faces,
            &idx,
            x,
            y,
            self.floor,
            self.res,
            self.budget / 10,
        );
        if c.hi < self.floor {
            c.hi = self.floor;
        }
        if c.lo < self.floor {
            c.lo = self.floor;
        }
        c
    }

    /// The vertical-gap check of one cutting move against every face near it, at `limit`.
    pub fn gap(&self, a: [f64; 3], b: [f64; 3], limit: f64) -> Gap {
        self.gap_at(a, b, limit).0
    }

    /// `gap`, and the face that decided it (the first not proven clear).
    pub fn gap_at(&self, a: [f64; 3], b: [f64; 3], limit: f64) -> (Gap, Option<usize>) {
        let r = self.check.r;
        let idx = self.grid.near(
            a[0].min(b[0]) - r,
            a[1].min(b[1]) - r,
            a[0].max(b[0]) + r,
            a[1].max(b[1]) + r,
        );
        let mv = Move::Line { a, b };
        let mut least = 0.0f64;
        for &i in &idx {
            let f = &self.faces[i as usize];
            let (g, _) = drop::gap_move(&self.check, f, &mv, limit, self.res, self.budget);
            match g {
                Gap::Clear { lower } => least = least.min(lower),
                // a proven penetration of the column: the piece is split (the cutter may still
                // be clear, but the generator does not spend a search to find out)
                Gap::Over { .. } => return (g, Some(i as usize)),
                Gap::Unknown => {
                    // the cutter's own signed distance, as the certifier then checks it, on a
                    // tenth of the budget
                    match forge_cam_geom::check::certify(
                        &self.cutter,
                        f,
                        &mv,
                        limit,
                        self.res,
                        self.budget / 10,
                    )
                    .0
                    {
                        forge_cam_geom::check::Verdict::Clear { lower } => least = least.min(lower),
                        _ => return (g, Some(i as usize)),
                    }
                }
            }
        }
        (Gap::Clear { lower: least }, None)
    }

    /// A 2D polyline dropped onto the part: samples at most `step` apart, then each piece split
    /// at its middle (a new drop) until its chord is within tol/4 of the surface at the middle
    /// and its own vertical gap is proven at tol/2.
    pub fn drop_polyline(&self, pts: &[[f64; 2]], step: f64) -> Result<Vec<[f64; 3]>, SurfError> {
        if step.is_nan() || step <= 0.0 {
            return Err(SurfError::Bad("step"));
        }
        let mut out: Vec<[f64; 3]> = Vec::new();
        let mut i = 0;
        while i < pts.len() {
            let p = pts[i];
            if i == 0 {
                out.push([p[0], p[1], self.cl(p[0], p[1]).hi]);
                i += 1;
                continue;
            }
            let q = pts[i - 1];
            let len = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
            let n = ((len / step).ceil() as usize).max(1);
            for k in 1..=n {
                let t = k as f64 / n as f64;
                let (x, y) = if k == n {
                    (p[0], p[1])
                } else {
                    (q[0] + t * (p[0] - q[0]), q[1] + t * (p[1] - q[1]))
                };
                let z = self.cl(x, y).hi;
                let a = out[out.len() - 1];
                self.refine(a, [x, y, z], 0, &mut out)?;
            }
            i += 1;
        }
        Ok(out)
    }

    fn refine(
        &self,
        a: [f64; 3],
        b: [f64; 3],
        depth: usize,
        out: &mut Vec<[f64; 3]>,
    ) -> Result<(), SurfError> {
        let m2 = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let small = len <= self.tol || depth >= 24;
        if !small {
            let cm = self.cl(m2[0], m2[1]);
            let chord = 0.5 * (a[2] + b[2]);
            if (cm.hi - chord).abs() > 0.25 * self.tol {
                let m = [m2[0], m2[1], cm.hi];
                self.refine(a, m, depth + 1, out)?;
                return self.refine(m, b, depth + 1, out);
            }
        }
        // a steep piece (the CL falling off a rounded wall's side): the step first (straight up
        // at the lower end, then level, or level then straight down), which the column proves
        // where it touches only from the side
        if (b[2] - a[2]).abs() > 4.0 * len && len <= 4.0 * self.tol {
            let mid = if b[2] > a[2] {
                [a[0], a[1], b[2]]
            } else {
                [b[0], b[1], a[2]]
            };
            if matches!(self.gap(a, mid, -0.5 * self.tol), Gap::Clear { .. })
                && matches!(self.gap(mid, b, -0.5 * self.tol), Gap::Clear { .. })
            {
                out.push(mid);
                out.push(b);
                return Ok(());
            }
        }
        match self.gap(a, b, -0.5 * self.tol) {
            Gap::Clear { .. } => {
                out.push(b);
                Ok(())
            }
            _ if !small => {
                let cm = self.cl(m2[0], m2[1]);
                let m = [m2[0], m2[1], cm.hi];
                self.refine(a, m, depth + 1, out)?;
                self.refine(m, b, depth + 1, out)
            }
            _ => {
                // a cliff (a vertical wall between the ends): straight up at the lower end, then
                // across at the higher height (or across, then straight down)
                let mid = if b[2] > a[2] {
                    [a[0], a[1], b[2]]
                } else {
                    [b[0], b[1], a[2]]
                };
                if matches!(self.gap(a, mid, -0.5 * self.tol), Gap::Clear { .. })
                    && matches!(self.gap(mid, b, -0.5 * self.tol), Gap::Clear { .. })
                {
                    out.push(mid);
                    out.push(b);
                    return Ok(());
                }
                if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                    let (g, fi) = self.gap_at(a, b, -0.5 * self.tol);
                    eprintln!(
                        "stuck {a:?} -> {b:?}: {g:?} face {:?}",
                        fi.map(|i| &self.faces[i])
                    );
                }
                Err(SurfError::Stuck { at: b, len })
            }
        }
    }
}

/// A path of cutting points as an op: rapid to above the first, plunge, cut through them, retract.
fn as_path(c: Cut, pts: &[[f64; 3]]) -> OpPath {
    let Cut { tidx, h, feed, .. } = c;
    let p0 = pts[0];
    let mut mv = vec![
        PMove::Rapid {
            to: [p0[0], p0[1], h.retract],
        },
        PMove::Line {
            to: p0,
            f: feed.1,
            phase: Phase::Plunge,
        },
    ];
    for p in &pts[1..] {
        mv.push(PMove::Line {
            to: *p,
            f: feed.0,
            phase: Phase::Cut,
        });
    }
    let pl = pts[pts.len() - 1];
    mv.push(PMove::Line {
        to: [pl[0], pl[1], h.retract],
        f: feed.0,
        phase: Phase::Link,
    });
    mv.push(PMove::Rapid {
        to: [pl[0], pl[1], h.clear],
    });
    OpPath {
        tool: tidx,
        start: [p0[0], p0[1], h.clear],
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    }
}

/// Several cutting polylines as one op, each entered from the retract height.
fn as_paths(c: Cut, runs: &[Vec<[f64; 3]>]) -> Option<OpPath> {
    let runs: Vec<&Vec<[f64; 3]>> = runs.iter().filter(|r| !r.is_empty()).collect();
    if runs.is_empty() {
        return None;
    }
    let mut p = as_path(c, runs[0]);
    for r in &runs[1..] {
        let q = as_path(c, r);
        // drop the previous run's final rapid to clear: link at the retract height instead
        p.moves.pop();
        p.moves.extend(q.moves);
    }
    Some(p)
}

/// The rows of a raster over an xy box: along the direction at `angle_deg`, `stepover` apart,
/// zig-zag, each joined to the next along the box's edge.
pub fn raster(area: [f64; 4], stepover: f64, angle_deg: f64) -> Result<Vec<[f64; 2]>, SurfError> {
    if stepover.is_nan() || stepover <= 0.0 {
        return Err(SurfError::Bad("stepover"));
    }
    let (s, c) = forge_math::dmath::sin_cos(angle_deg.to_radians());
    let d = [c, s];
    let n = [-s, c];
    let corners = [
        [area[0], area[1]],
        [area[2], area[1]],
        [area[2], area[3]],
        [area[0], area[3]],
    ];
    let (mut u0, mut u1, mut v0, mut v1) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in corners {
        let u = p[0] * d[0] + p[1] * d[1];
        let v = p[0] * n[0] + p[1] * n[1];
        u0 = u0.min(u);
        u1 = u1.max(u);
        v0 = v0.min(v);
        v1 = v1.max(v);
    }
    let rows = (((v1 - v0) / stepover - 1e-9).ceil().max(0.0) as usize) + 1;
    let mut out = Vec::new();
    for k in 0..rows {
        let v = if rows == 1 {
            0.5 * (v0 + v1)
        } else {
            v0 + (v1 - v0) * k as f64 / (rows - 1) as f64
        };
        let (ua, ub) = if k % 2 == 0 { (u0, u1) } else { (u1, u0) };
        out.push([ua * d[0] + v * n[0], ua * d[1] + v * n[1]]);
        out.push([ub * d[0] + v * n[0], ub * d[1] + v * n[1]]);
    }
    // keep within the box where the rotation's rounding strays
    for p in out.iter_mut() {
        p[0] = p[0]
            .max(area[0].min(area[2]) - 1e-9 * (1.0 + area[0].abs()))
            .min(area[2].max(area[0]) + 1e-9 * (1.0 + area[2].abs()));
        p[1] = p[1]
            .max(area[1].min(area[3]) - 1e-9 * (1.0 + area[1].abs()))
            .min(area[3].max(area[1]) + 1e-9 * (1.0 + area[3].abs()));
    }
    Ok(out)
}

/// Parallel (raster) finishing over `area` (x0, y0, x1, y1: tool centres), rows `stepover`
/// apart along `angle_deg`, samples at most `step` apart along them.
pub fn parallel(
    sf: &Surfacer,
    c: Cut,
    area: [f64; 4],
    stepover: f64,
    angle_deg: f64,
    step: f64,
) -> Result<OpPath, OpError> {
    let rows = raster(area, stepover, angle_deg)?;
    // each row and the link after it dropped on its own (on all cores), joined in order
    let n = rows.len() / 2;
    let parts: Vec<Result<Vec<[f64; 3]>, SurfError>> = forge_math::par::map(n, |k| {
        let mut pl = vec![rows[2 * k], rows[2 * k + 1]];
        if 2 * k + 2 < rows.len() {
            pl.push(rows[2 * k + 2]);
        }
        sf.drop_polyline(&pl, step)
    });
    let mut pts: Vec<[f64; 3]> = Vec::new();
    for part in parts {
        let mut part = part?;
        if !pts.is_empty() {
            // the next row starts where this link ended
            part.remove(0);
        }
        pts.extend(part);
    }
    Ok(as_path(c, &pts))
}

/// A certified push-cutter along a fibre at tool-tip height z: on the segment a → b (xy), the
/// first s where the column at height z is proven to meet the part (CL.lo > z), bracketed to
/// `res` in xy length by bisection after samples at most `step` apart: Some((s_free, s_hit)), the
/// last sample proven free (CL.hi ≤ z) before it and the first proven blocked. None when no sample
/// is proven blocked.
pub fn push(sf: &Surfacer, a: [f64; 2], b: [f64; 2], z: f64, step: f64) -> Option<(f64, f64)> {
    let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let n = ((len / step).ceil() as usize).max(1);
    let at = |t: f64| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
    let mut free = None;
    for k in 0..=n {
        let t = k as f64 / n as f64;
        let p = at(t);
        let c = sf.cl(p[0], p[1]);
        if c.lo > z {
            let mut lo = free?;
            let mut hi = t;
            while (hi - lo) * len > sf.res {
                let m = 0.5 * (lo + hi);
                let pm = at(m);
                let cm = sf.cl(pm[0], pm[1]);
                if cm.lo > z {
                    hi = m;
                } else if cm.hi <= z {
                    lo = m;
                } else {
                    // within the drop's resolution of z: call it blocked (the free side stays proven)
                    hi = m;
                }
            }
            return Some((lo, hi));
        }
        if c.hi <= z {
            free = Some(t);
        }
    }
    None
}

/// The tool-centre grid of CL heights over an area at spacing h: `cl[iy * nx + ix]` the
/// certified upper bound at (x0 + ix h, y0 + iy h).
pub struct ClGrid {
    pub x0: f64,
    pub y0: f64,
    pub h: f64,
    pub nx: usize,
    pub ny: usize,
    pub hi: Vec<f64>,
    pub lo: Vec<f64>,
}

impl ClGrid {
    pub fn new(sf: &Surfacer, area: [f64; 4], h: f64) -> Result<ClGrid, SurfError> {
        if h.is_nan() || h <= 0.0 {
            return Err(SurfError::Bad("grid"));
        }
        let nx = (((area[2] - area[0]) / h).ceil() as usize).max(1) + 1;
        let ny = (((area[3] - area[1]) / h).ceil() as usize).max(1) + 1;
        let rows: Vec<Vec<Cl>> = forge_math::par::map(ny, |iy| {
            (0..nx)
                .map(|ix| sf.cl(area[0] + ix as f64 * h, area[1] + iy as f64 * h))
                .collect()
        });
        let mut hi = Vec::with_capacity(nx * ny);
        let mut lo = Vec::with_capacity(nx * ny);
        for r in rows {
            for c in r {
                hi.push(c.hi);
                lo.push(c.lo);
            }
        }
        Ok(ClGrid {
            x0: area[0],
            y0: area[1],
            h,
            nx,
            ny,
            hi,
            lo,
        })
    }

    pub fn at(&self, ix: usize, iy: usize) -> [f64; 2] {
        [self.x0 + ix as f64 * self.h, self.y0 + iy as f64 * self.h]
    }
}

/// Waterline (constant-z) finishing: at each height, the closed contours where the column's CL
/// crosses z, traced by marching squares over a CL grid at spacing `h`, each crossing found by
/// the push-cutter's bisection, then dropped as a constant-z path (each move's gap proven by the
/// generator, split where not).
pub fn waterline(sf: &Surfacer, c: Cut, area: [f64; 4], zs: &[f64], h: f64) -> Result<OpPath, OpError> {
    let g = ClGrid::new(sf, area, h)?;
    // every loop of every level traced on its own, kept in order. A loop's vertices, and then
    // its pieces between consecutive vertices, are independent of each other (each a function
    // of its own ends): both are made on all cores (forge_math::par) as flat lists over every
    // loop, so a few long loops do not leave cores idle, and assembled in order as one pass would
    let mut jobs: Vec<(f64, Contour)> = Vec::new();
    for &z in zs {
        for lp in contours(&g, z) {
            jobs.push((z, lp));
        }
    }
    // each vertex: the crossing on its grid edge, bisected to the free side
    let edges: Vec<(usize, usize)> = jobs
        .iter()
        .enumerate()
        .flat_map(|(j, (_, lp))| (0..lp.len()).map(move |e| (j, e)))
        .collect();
    let verts: Vec<[f64; 3]> = forge_math::par::map(edges.len(), |i| {
        let (j, e) = edges[i];
        let (z, lp) = &jobs[j];
        let (a, b) = lp[e];
        let p = match push(sf, a, b, *z, h) {
            Some((s, _)) => [a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1])],
            None => a,
        };
        [p[0], p[1], *z]
    });
    // each loop's vertices, closed (a loop of fewer than three is dropped)
    let mut loops: Vec<Vec<[f64; 3]>> = Vec::with_capacity(jobs.len());
    let mut k = 0;
    for (_, lp) in &jobs {
        let mut pts = verts[k..k + lp.len()].to_vec();
        k += lp.len();
        if pts.len() >= 3 {
            let first = pts[0];
            pts.push(first);
        } else {
            pts.clear();
        }
        loops.push(pts);
    }
    // each piece refined from its start (the vertex before it: a refinement ends at its end)
    let pieces: Vec<(usize, usize)> = loops
        .iter()
        .enumerate()
        .flat_map(|(j, pts)| (1..pts.len()).map(move |i| (j, i)))
        .collect();
    let refined: Vec<Result<Vec<[f64; 3]>, SurfError>> = forge_math::par::map(pieces.len(), |i| {
        let (j, k) = pieces[i];
        let mut out = Vec::new();
        level_refine(sf, loops[j][k - 1], loops[j][k], 0, &mut out)?;
        Ok(out)
    });
    let mut runs: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut refined = refined.into_iter();
    for pts in &loops {
        if pts.is_empty() {
            continue;
        }
        let mut run = vec![pts[0]];
        for _ in 1..pts.len() {
            run.extend(refined.next().expect("a piece per pair")?);
        }
        runs.push(run);
    }
    as_paths(c, &runs).ok_or(OpError::ToolTooBig {
        tool_d: 2.0 * sf.check.r,
        why: "no waterline at these heights".into(),
    })
}

/// A traced contour: its grid edges as (free node, blocked node).
type Contour = Vec<([f64; 2], [f64; 2])>;

/// A constant-z piece proven at tol/2, else split at its middle pushed out to the free side
/// (along the piece's normal, toward lower CL).
fn level_refine(
    sf: &Surfacer,
    a: [f64; 3],
    b: [f64; 3],
    depth: usize,
    out: &mut Vec<[f64; 3]>,
) -> Result<(), SurfError> {
    let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    // the middle pushed out to the free side's boundary (along the piece's normal, toward lower
    // CL): split there when the boundary strays from the chord, before any proof is tried
    let mid = |sf: &Surfacer| -> [f64; 3] {
        let m = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        let n = [-(b[1] - a[1]) / len, (b[0] - a[0]) / len];
        let e = 0.25 * len;
        let c1 = sf.cl(m[0] + e * n[0], m[1] + e * n[1]).hi;
        let c2 = sf.cl(m[0] - e * n[0], m[1] - e * n[1]).hi;
        let dir = if c1 <= c2 { n } else { [-n[0], -n[1]] };
        let far = [m[0] + len * dir[0], m[1] + len * dir[1]];
        let p = match push(sf, far, m, a[2], 0.25 * len) {
            Some((s, _)) => [far[0] + s * (m[0] - far[0]), far[1] + s * (m[1] - far[1])],
            None => m,
        };
        [p[0], p[1], a[2]]
    };
    if len > sf.tol && depth < 24 {
        let mm = mid(sf);
        let off = ((mm[0] - 0.5 * (a[0] + b[0])).powi(2) + (mm[1] - 0.5 * (a[1] + b[1])).powi(2)).sqrt();
        if off > 0.25 * sf.tol {
            level_refine(sf, a, mm, depth + 1, out)?;
            return level_refine(sf, mm, b, depth + 1, out);
        }
    }
    if let Gap::Clear { .. } = sf.gap(a, b, -0.5 * sf.tol) {
        out.push(b);
        return Ok(());
    }
    if len <= sf.res || depth >= 24 {
        return Err(SurfError::Stuck { at: b, len });
    }
    let mm = mid(sf);
    level_refine(sf, a, mm, depth + 1, out)?;
    level_refine(sf, mm, b, depth + 1, out)
}

/// Marching squares on the grid at level z: closed loops of grid edges (each edge from a node
/// with CL ≤ z, free, to one with CL > z, blocked), in order around each blocked component,
/// free side on the right. Ambiguous saddles are split so blocked cells connect.
fn contours(g: &ClGrid, z: f64) -> Vec<Vec<([f64; 2], [f64; 2])>> {
    let (nx, ny) = (g.nx, g.ny);
    let blocked = |ix: usize, iy: usize| g.hi[iy * nx + ix] > z;
    // directed segments between edge midpoints, keyed by edge id; an edge id is
    // (node a, node b) with a < b in index order
    let node = |ix: usize, iy: usize| iy * nx + ix;
    let mut next: std::collections::BTreeMap<(usize, usize), (usize, usize)> =
        std::collections::BTreeMap::new();
    let ekey = |a: usize, b: usize| if a < b { (a, b) } else { (b, a) };
    for iy in 0..ny.saturating_sub(1) {
        for ix in 0..nx.saturating_sub(1) {
            // corners counter-clockwise: 0 (ix,iy) 1 (ix+1,iy) 2 (ix+1,iy+1) 3 (ix,iy+1)
            let cs = [
                node(ix, iy),
                node(ix + 1, iy),
                node(ix + 1, iy + 1),
                node(ix, iy + 1),
            ];
            let bs = [
                blocked(ix, iy),
                blocked(ix + 1, iy),
                blocked(ix + 1, iy + 1),
                blocked(ix, iy + 1),
            ];
            // the cell's crossing edges in counter-clockwise order; each entering (free→blocked
            // going ccw) pairs with the next leaving one
            let mut cross: Vec<(usize, bool)> = Vec::new();
            for k in 0..4 {
                let (a, b) = (bs[k], bs[(k + 1) % 4]);
                if a != b {
                    cross.push((k, !a && b));
                }
            }
            // pair them so the blocked side is on the left of each segment: from an edge where
            // going ccw enters blocked, to the next edge (ccw) where it leaves
            let m = cross.len();
            for i in 0..m {
                let (k, enters) = cross[i];
                if !enters {
                    continue;
                }
                // the next leaving edge ccw (saddles: blocked corners connect through the cell)
                let mut j = (i + 1) % m;
                while cross[j].1 {
                    j = (j + 1) % m;
                }
                let (k2, _) = cross[j];
                let e1 = ekey(cs[k], cs[(k + 1) % 4]);
                let e2 = ekey(cs[k2], cs[(k2 + 1) % 4]);
                next.insert(e1, e2);
            }
        }
    }
    // the edge's two nodes as (free, blocked) points
    let pt = |n: usize| [g.x0 + (n % nx) as f64 * g.h, g.y0 + (n / nx) as f64 * g.h];
    let fb = |e: (usize, usize)| {
        let (a, b) = e;
        let ab = g.hi[a] > z;
        if ab { (pt(b), pt(a)) } else { (pt(a), pt(b)) }
    };
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.iter().next() {
        let mut lp = Vec::new();
        let mut e = start;
        loop {
            lp.push(fb(e));
            let Some(n) = next.remove(&e) else {
                break;
            };
            e = n;
            if e == start {
                break;
            }
        }
        next.remove(&start);
        loops.push(lp);
    }
    loops
}

/// Pencil finishing: along rows over the area, the points where the CL surface creases (the
/// tool touches the part on two sides at once: the CL's slope jumps), chained into runs.
/// A crease shows as a second difference of the CL rows above `kink` (in mm over the grid
/// spacing).
pub fn pencil(sf: &Surfacer, c: Cut, area: [f64; 4], h: f64, kink: f64) -> Result<OpPath, OpError> {
    let g = ClGrid::new(sf, area, h)?;
    let mut mark = vec![false; g.nx * g.ny];
    for iy in 0..g.ny {
        for ix in 0..g.nx {
            let k = iy * g.nx + ix;
            let mut v = false;
            if ix > 0 && ix + 1 < g.nx {
                let d2 = g.hi[k - 1] + g.hi[k + 1] - 2.0 * g.hi[k];
                v |= d2 > kink;
            }
            if iy > 0 && iy + 1 < g.ny {
                let d2 = g.hi[k - g.nx] + g.hi[k + g.nx] - 2.0 * g.hi[k];
                v |= d2 > kink;
            }
            // a crease is a valley of the CL surface: concave up and at the floor of neither
            mark[k] = v && g.hi[k] > sf.floor;
        }
    }
    // chain marked nodes into runs by 8-neighbour walks
    let mut used = vec![false; mark.len()];
    let mut runs2: Vec<Vec<[f64; 2]>> = Vec::new();
    for s in 0..mark.len() {
        if !mark[s] || used[s] {
            continue;
        }
        let mut run = vec![s];
        used[s] = true;
        let mut cur = s;
        loop {
            let (cx, cy) = ((cur % g.nx) as i64, (cur / g.nx) as i64);
            let mut found = None;
            for (dx, dy) in [
                (1, 0),
                (0, 1),
                (-1, 0),
                (0, -1),
                (1, 1),
                (-1, 1),
                (1, -1),
                (-1, -1),
            ] {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= g.nx as i64 || y >= g.ny as i64 {
                    continue;
                }
                let k = y as usize * g.nx + x as usize;
                if mark[k] && !used[k] {
                    found = Some(k);
                    break;
                }
            }
            match found {
                Some(k) => {
                    used[k] = true;
                    run.push(k);
                    cur = k;
                }
                None => break,
            }
        }
        if run.len() >= 2 {
            runs2.push(run.iter().map(|&k| g.at(k % g.nx, k / g.nx)).collect());
        }
    }
    let mut runs = Vec::new();
    for r in &runs2 {
        runs.push(sf.drop_polyline(r, h)?);
    }
    as_paths(c, &runs).ok_or(OpError::ToolTooBig {
        tool_d: 2.0 * sf.check.r,
        why: "no crease the tool rides in".into(),
    })
}

/// The lowest a tool's cutter reaches over each node of a grid, ideally: E(q) = min over the
/// grid's tool positions X within R of q of CL(X) + b(|q − X|), from a CL grid of that tool.
/// An estimate of the material the tool leaves (the true envelope of a tool moving over every
/// position is lower by at most the grid's scallop), for planning only.
fn envelope(g: &ClGrid, col: &Column) -> Vec<f64> {
    let k = (col.r / g.h).floor() as i64;
    let mut out = vec![f64::INFINITY; g.nx * g.ny];
    // the offsets within reach and the profile's height there, once
    let mut offs: Vec<(i64, i64, f64)> = Vec::new();
    for dy in -k..=k {
        for dx in -k..=k {
            let rho = ((dx * dx + dy * dy) as f64).sqrt() * g.h;
            if rho <= col.r {
                offs.push((dx, dy, col.b_at(rho).mid()));
            }
        }
    }
    let rows: Vec<Vec<f64>> = forge_math::par::map(g.ny, |iy| {
        let mut row = vec![f64::INFINITY; g.nx];
        for (ix, o) in row.iter_mut().enumerate() {
            let mut m = f64::INFINITY;
            for &(dx, dy, bz) in &offs {
                {
                    let (x, y) = (ix as i64 + dx, iy as i64 + dy);
                    if x < 0 || y < 0 || x >= g.nx as i64 || y >= g.ny as i64 {
                        continue;
                    }
                    let v = g.hi[y as usize * g.nx + x as usize] + bz;
                    if v < m {
                        m = v;
                    }
                }
            }
            *o = m;
        }
        row
    });
    for (iy, r) in rows.into_iter().enumerate() {
        out[iy * g.nx..(iy + 1) * g.nx].copy_from_slice(&r);
    }
    out
}

/// Rest finishing: parallel rows of this tool, cut only over the tool positions whose reach
/// holds material an earlier tool (`before`) left more than `min_rest` thick: where the earlier
/// tool's envelope stands that much above this tool's (both estimated on a grid at spacing
/// `h`). Rows are broken into runs over the marked stretches.
#[allow(clippy::too_many_arguments)]
pub fn rest(
    sf: &Surfacer,
    before: &Surfacer,
    c: Cut,
    area: [f64; 4],
    stepover: f64,
    step: f64,
    h: f64,
    min_rest: f64,
) -> Result<OpPath, OpError> {
    // the grid covers the area grown by the larger reach, so every envelope over it is whole
    // (whole millimetres: a grid laid from the reach itself lands its nodes on the widened
    // rims of tools beside walls at round coordinates)
    let pad = before.make.r.max(sf.make.r).ceil();
    let big = [area[0] - pad, area[1] - pad, area[2] + pad, area[3] + pad];
    let ga = ClGrid::new(before, big, h)?;
    let gb = ClGrid::new(sf, big, h)?;
    let ea = envelope(&ga, &before.make);
    let eb = envelope(&gb, &sf.make);
    let rest: Vec<bool> = (0..ea.len())
        .map(|i| ea[i] - eb[i] > min_rest && eb[i] > sf.floor)
        .collect();
    // a tool position is wanted when a rest node lies within its reach
    let k = (sf.make.r / h).ceil() as i64;
    let wanted = |x: f64, y: f64| -> bool {
        let ix = ((x - ga.x0) / h).round() as i64;
        let iy = ((y - ga.y0) / h).round() as i64;
        for dy in -k..=k {
            for dx in -k..=k {
                let (u, v) = (ix + dx, iy + dy);
                if u < 0 || v < 0 || u >= ga.nx as i64 || v >= ga.ny as i64 {
                    continue;
                }
                if ((dx * dx + dy * dy) as f64).sqrt() * h > sf.make.r {
                    continue;
                }
                if rest[v as usize * ga.nx + u as usize] {
                    return true;
                }
            }
        }
        false
    };
    let rows = raster(area, stepover, 0.0)?;
    let mut runs = Vec::new();
    let mut r = 0;
    while r + 1 < rows.len() {
        let (a, b) = (rows[r], rows[r + 1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = ((len / step).ceil() as usize).max(1);
        let on: Vec<bool> = (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                wanted(a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))
            })
            .collect();
        let mut i = 0;
        while i <= n {
            if !on[i] {
                i += 1;
                continue;
            }
            let s0 = i;
            while i <= n && on[i] {
                i += 1;
            }
            let s1 = (i - 1).max(s0);
            let at = |j: usize| {
                [
                    a[0] + j as f64 / n as f64 * (b[0] - a[0]),
                    a[1] + j as f64 / n as f64 * (b[1] - a[1]),
                ]
            };
            if s1 > s0 {
                runs.push(sf.drop_polyline(&[at(s0), at(s1)], step)?);
            }
        }
        r += 2;
    }
    as_paths(c, &runs).ok_or(OpError::ToolTooBig {
        tool_d: 2.0 * sf.check.r,
        why: "nothing left for this tool".into(),
    })
}

/// Scallop (constant stepover) finishing: passes `stepover` apart measured along the part,
/// as the level sets of the distance over the CL surface (on a grid at spacing h, by Dijkstra
/// with each grid step's 3D length) from the area's boundary, each traced by marching squares
/// and dropped.
pub fn scallop(sf: &Surfacer, c: Cut, area: [f64; 4], stepover: f64, h: f64) -> Result<OpPath, OpError> {
    if stepover.is_nan() || stepover <= 0.0 {
        return Err(SurfError::Bad("stepover").into());
    }
    let g = ClGrid::new(sf, area, h)?;
    let (nx, ny) = (g.nx, g.ny);
    let mut dist = vec![f64::INFINITY; nx * ny];
    let mut heap = std::collections::BinaryHeap::new();
    for iy in 0..ny {
        for ix in 0..nx {
            if ix == 0 || iy == 0 || ix + 1 == nx || iy + 1 == ny {
                let k = iy * nx + ix;
                dist[k] = 0.0;
                heap.push((std::cmp::Reverse(Ord64(0.0)), k));
            }
        }
    }
    while let Some((std::cmp::Reverse(Ord64(d)), k)) = heap.pop() {
        if d > dist[k] {
            continue;
        }
        let (ix, iy) = ((k % nx) as i64, (k / nx) as i64);
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let (x, y) = (ix + dx, iy + dy);
            if x < 0 || y < 0 || x >= nx as i64 || y >= ny as i64 {
                continue;
            }
            let j = y as usize * nx + x as usize;
            let dz = g.hi[j] - g.hi[k];
            let w = (((dx * dx + dy * dy) as f64) * h * h + dz * dz).sqrt();
            if d + w < dist[j] {
                dist[j] = d + w;
                heap.push((std::cmp::Reverse(Ord64(d + w)), j));
            }
        }
    }
    let far = dist.iter().cloned().fold(0.0f64, f64::max);
    let mut runs: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut level = 0.5 * stepover;
    while level < far {
        let field = ClGrid {
            x0: g.x0,
            y0: g.y0,
            h,
            nx,
            ny,
            hi: dist.clone(),
            lo: dist.clone(),
        };
        for lp in contours(&field, level) {
            // each crossing interpolated on its edge
            let mut pts: Vec<[f64; 2]> = Vec::new();
            for (a, b) in lp {
                let ia = node_of(&g, a);
                let ib = node_of(&g, b);
                let (da, db) = (dist[ia], dist[ib]);
                let t = if db != da {
                    ((level - da) / (db - da)).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                pts.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
            if pts.len() < 3 {
                continue;
            }
            let first = pts[0];
            pts.push(first);
            runs.push(sf.drop_polyline(&pts, h)?);
        }
        level += stepover;
    }
    as_paths(c, &runs).ok_or(OpError::ToolTooBig {
        tool_d: 2.0 * sf.check.r,
        why: "no scallop pass in the area".into(),
    })
}

fn node_of(g: &ClGrid, p: [f64; 2]) -> usize {
    let ix = ((p[0] - g.x0) / g.h).round() as usize;
    let iy = ((p[1] - g.y0) / g.h).round() as usize;
    iy.min(g.ny - 1) * g.nx + ix.min(g.nx - 1)
}

/// f64 ordered by total_cmp, for the Dijkstra heap.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ord64(f64);
impl Eq for Ord64 {}
impl PartialOrd for Ord64 {
    fn partial_cmp(&self, o: &Ord64) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Ord64 {
    fn cmp(&self, o: &Ord64) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// The in-process stock as a height map: cell (ix, iy) is the square [x0 + ix h, + h] ×
/// [y0 + iy h, + h]; `top` is at or above the material's highest point in it, `bottom` the
/// stock's floor. Cutting lowers a cell only to a height every point of the cell is proven cut
/// down to, so the map always contains the true stock.
#[derive(Clone, Debug, PartialEq)]
pub struct ZMap {
    pub x0: f64,
    pub y0: f64,
    pub h: f64,
    pub nx: usize,
    pub ny: usize,
    pub bottom: f64,
    pub top: Vec<f64>,
}

impl ZMap {
    /// A box of stock, cells `h` wide.
    pub fn new(min: [f64; 3], max: [f64; 3], h: f64) -> ZMap {
        let nx = (((max[0] - min[0]) / h).ceil() as usize).max(1);
        let ny = (((max[1] - min[1]) / h).ceil() as usize).max(1);
        ZMap {
            x0: min[0],
            y0: min[1],
            h,
            nx,
            ny,
            bottom: min[2],
            top: vec![max[2]; nx * ny],
        }
    }

    fn cell(&self, ix: usize, iy: usize) -> [Ival; 2] {
        let x = Ival::pt(self.x0).add(Ival::pt(ix as f64).mul(Ival::pt(self.h)));
        let y = Ival::pt(self.y0).add(Ival::pt(iy as f64).mul(Ival::pt(self.h)));
        [
            Ival {
                lo: x.lo,
                hi: x.add(Ival::pt(self.h)).hi,
            },
            Ival {
                lo: y.lo,
                hi: y.add(Ival::pt(self.h)).hi,
            },
        ]
    }

    /// Cuts the map with a tool (its cutter's column) along a straight move a → b: a cell is
    /// lowered to an upper bound of the tool's bottom over it when every point of the cell is
    /// proven within the column's reach at the instant nearest to it (its foot on the move), the
    /// bound being that instant's tip height plus the profile at the farthest such reach.
    pub fn cut(&mut self, col: &Column, a: [f64; 3], b: [f64; 3]) {
        let r = col.r;
        let (xa, xb) = (a[0].min(b[0]) - r, a[0].max(b[0]) + r);
        let (ya, yb) = (a[1].min(b[1]) - r, a[1].max(b[1]) + r);
        let ix0 = (((xa - self.x0) / self.h).floor().max(0.0)) as usize;
        let iy0 = (((ya - self.y0) / self.h).floor().max(0.0)) as usize;
        let ix1 = ((((xb - self.x0) / self.h).ceil()).max(0.0) as usize).min(self.nx);
        let iy1 = ((((yb - self.y0) / self.h).ceil()).max(0.0) as usize).min(self.ny);
        let (dx, dy) = (
            Ival::pt(b[0]).sub(Ival::pt(a[0])),
            Ival::pt(b[1]).sub(Ival::pt(a[1])),
        );
        let dd = dx.sqr().add(dy.sqr());
        for iy in iy0..iy1 {
            for ix in ix0..ix1 {
                let [cx, cy] = self.cell(ix, iy);
                // the farthest the cell's points are from the move's xy path: each corner's
                // distance to the segment (the cell is convex and the distance to a segment is
                // convex, so its maximum over the cell is at a corner)
                let mut far = Ival::pt(0.0);
                let mut tz = Ival::pt(f64::NEG_INFINITY);
                for (px, py) in [(cx.lo, cy.lo), (cx.hi, cy.lo), (cx.hi, cy.hi), (cx.lo, cy.hi)] {
                    let rx = Ival::pt(px).sub(Ival::pt(a[0]));
                    let ry = Ival::pt(py).sub(Ival::pt(a[1]));
                    let (d, s) = if dd.hi == 0.0 {
                        (rx.sqr().add(ry.sqr()).sqrt(), Ival::pt(0.0))
                    } else {
                        let t = rx.mul(dx).add(ry.mul(dy)).div(dd);
                        let t = Ival {
                            lo: t.lo.clamp(0.0, 1.0),
                            hi: t.hi.clamp(0.0, 1.0),
                        };
                        let ex = rx.sub(t.mul(dx));
                        let ey = ry.sub(t.mul(dy));
                        (ex.sqr().add(ey.sqr()).sqrt(), t)
                    };
                    far = far.max(d);
                    // the tip's height at the feet of the cell's points: a hull over the corners'
                    // feet holds every point's foot (the foot is monotone along the move)
                    let z = Ival::pt(a[2]).add(s.mul(Ival::pt(b[2]).sub(Ival::pt(a[2]))));
                    tz = if tz.lo == f64::NEG_INFINITY { z } else { tz.hull(z) };
                }
                if far.hi.is_nan() || far.hi > r {
                    continue;
                }
                let Some(bz) = col.b(Ival { lo: 0.0, hi: far.hi }) else {
                    continue;
                };
                let floor = tz.add(Ival::pt(bz.hi)).hi;
                let k = iy * self.nx + ix;
                if floor < self.top[k] {
                    self.top[k] = floor.max(self.bottom);
                }
            }
        }
    }

    /// Cuts the map with every cutting move of a path.
    pub fn cut_path(&mut self, col: &Column, p: &OpPath) {
        let mut at = p.start;
        for m in &p.moves {
            match *m {
                PMove::Rapid { to } => at = to,
                PMove::Line { to, .. } => {
                    self.cut(col, at, to);
                    at = to;
                }
                PMove::Arc { to, .. } => at = to,
                PMove::Cycle { holes, clear, .. } => at = [holes[0], holes[1], clear],
            }
        }
    }

    /// The map as the checker's faces: each cell's top, and the walls where a cell stands above
    /// its neighbour (or the map's edge).
    pub fn faces(&self, id0: u32) -> Vec<Face> {
        let mut out = Vec::new();
        let rect = |w: f64, h: f64| Trim {
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
        let mut id = id0;
        let mut mk = |o: [f64; 3], u: [f64; 3], v: [f64; 3], w: f64, hh: f64, out: &mut Vec<Face>| {
            if w > 0.0 && hh > 0.0 {
                out.push(Face::new(
                    Surf::Plane { o, u, v },
                    Ival::new(0.0, w),
                    Ival::new(0.0, hh),
                    Some(rect(w, hh)),
                    1e-9,
                    id,
                ));
                id += 1;
            }
        };
        for iy in 0..self.ny {
            for ix in 0..self.nx {
                let k = iy * self.nx + ix;
                let t = self.top[k];
                if t <= self.bottom {
                    continue;
                }
                let x = self.x0 + ix as f64 * self.h;
                let y = self.y0 + iy as f64 * self.h;
                let h = self.h;
                mk([x, y, t], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], h, h, &mut out);
                // walls down to the neighbour's top (or the floor at the edge)
                let nb = |jx: i64, jy: i64| -> f64 {
                    if jx < 0 || jy < 0 || jx >= self.nx as i64 || jy >= self.ny as i64 {
                        self.bottom
                    } else {
                        self.top[jy as usize * self.nx + jx as usize].max(self.bottom)
                    }
                };
                let (ixs, iys) = (ix as i64, iy as i64);
                let w = nb(ixs - 1, iys);
                if w < t {
                    mk([x, y, w], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], h, t - w, &mut out);
                }
                let w = nb(ixs + 1, iys);
                if w < t {
                    mk(
                        [x + h, y, w],
                        [0.0, 1.0, 0.0],
                        [0.0, 0.0, 1.0],
                        h,
                        t - w,
                        &mut out,
                    );
                }
                let w = nb(ixs, iys - 1);
                if w < t {
                    mk([x, y, w], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], h, t - w, &mut out);
                }
                let w = nb(ixs, iys + 1);
                if w < t {
                    mk(
                        [x, y + h, w],
                        [1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0],
                        h,
                        t - w,
                        &mut out,
                    );
                }
            }
        }
        out
    }
}

/// Z-level roughing: at each level from `top` down to `bottom` (at most `down` apart), the tool
/// centres the generating column leaves free at that height (CL ≤ z on a grid at spacing h, the
/// blocked nodes' cells grown by one cell), cleared by contour-parallel passes `stepover` apart
/// along the free region's boundary loops. Offset clearing, not adaptive.
#[allow(clippy::too_many_arguments)]
pub fn zlevel(
    sf: &Surfacer,
    c: Cut,
    area: [f64; 4],
    top: f64,
    bottom: f64,
    down: f64,
    stepover: f64,
    h: f64,
) -> Result<OpPath, OpError> {
    zlevel_voids(sf, c, area, top, bottom, down, stepover, h).map(|x| x.0)
}

/// `zlevel`, and what each level provably cleared, for the in-process stock: where the cutter
/// has a flat bottom of radius rf ≥ stepover, the level's free tool-centre region C grown by rf,
/// down to the level (the passes, k·stepover apart, cover it: passes_cover).
#[allow(clippy::too_many_arguments)]
pub fn zlevel_voids(
    sf: &Surfacer,
    c: Cut,
    area: [f64; 4],
    top: f64,
    bottom: f64,
    down: f64,
    stepover: f64,
    h: f64,
) -> Result<(OpPath, Vec<(forge_cam_geom::region::Region, f64)>), OpError> {
    use forge_cam_geom::boolean::{Op, boolean, region_of};
    use forge_cam_geom::offset::offset;
    use forge_cam_geom::region::{Loop, Region, Seg, p2};
    // passes k·stepover apart cover the free region's opening only for stepover ≤ the tool's
    // radius (passes_cover, verify/forge-cam Pocket.lean)
    if !(stepover > 0.0 && stepover <= sf.check.r && down > 0.0) {
        return Err(SurfError::Bad("stepover").into());
    }
    let g = ClGrid::new(sf, area, h)?;
    let mut runs: Vec<([f64; 3], Vec<PMove>)> = Vec::new();
    let mut voids = Vec::new();
    // the cutter's flat bottom: its first profile piece, if level
    let rf = match sf.check.segs[0] {
        drop::CSeg::Line { a, b } if a[1] == b[1] => b[0],
        _ => 0.0,
    };
    // the levels, and the part's own top within them (its highest CL), so the stock above the
    // part's top is taken down to it
    let mut zs = crate::ops::levels(top, bottom, down);
    let peak = g.hi.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if peak > bottom && peak < top && !zs.iter().any(|&z| (z - peak).abs() <= sf.tol) {
        zs.push(peak);
        zs.sort_by(|a, b| b.total_cmp(a));
    }
    for z in zs {
        // blocked cells: a cell any of whose corners is not proven free
        let (cx, cy) = (g.nx - 1, g.ny - 1);
        let mut blk = vec![false; cx * cy];
        for iy in 0..cy {
            for ix in 0..cx {
                let k = |x: usize, y: usize| g.hi[y * g.nx + x] > z;
                blk[iy * cx + ix] = k(ix, iy) || k(ix + 1, iy) || k(ix, iy + 1) || k(ix + 1, iy + 1);
            }
        }
        // grown by a cell, and saddles closed, so the boundary is a set of simple loops
        let mut grown = blk.clone();
        for iy in 0..cy {
            for ix in 0..cx {
                if !blk[iy * cx + ix] {
                    continue;
                }
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let (x, y) = (ix as i64 + dx, iy as i64 + dy);
                        if x >= 0 && y >= 0 && x < cx as i64 && y < cy as i64 {
                            grown[y as usize * cx + x as usize] = true;
                        }
                    }
                }
            }
        }
        loop {
            let mut changed = false;
            for iy in 0..cy.saturating_sub(1) {
                for ix in 0..cx.saturating_sub(1) {
                    let a = grown[iy * cx + ix];
                    let b = grown[iy * cx + ix + 1];
                    let c2 = grown[(iy + 1) * cx + ix];
                    let d = grown[(iy + 1) * cx + ix + 1];
                    if (a && d && !b && !c2) || (b && c2 && !a && !d) {
                        grown[iy * cx + ix] = true;
                        grown[iy * cx + ix + 1] = true;
                        grown[(iy + 1) * cx + ix] = true;
                        grown[(iy + 1) * cx + ix + 1] = true;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // the blocked cells' union as loops: every cell side between blocked and free (or the
        // area's edge is free: the area is the tool centres' bound) as a directed edge with
        // the blocked cell on its left, chained
        let corner = |ix: usize, iy: usize| (ix, iy);
        let mut edges: std::collections::BTreeMap<(usize, usize), (usize, usize)> =
            std::collections::BTreeMap::new();
        let b = |ix: i64, iy: i64| {
            ix >= 0 && iy >= 0 && ix < cx as i64 && iy < cy as i64 && grown[iy as usize * cx + ix as usize]
        };
        for iy in 0..cy as i64 {
            for ix in 0..cx as i64 {
                if !b(ix, iy) {
                    continue;
                }
                let (x, y) = (ix as usize, iy as usize);
                if !b(ix, iy - 1) {
                    edges.insert(corner(x, y), corner(x + 1, y));
                }
                if !b(ix + 1, iy) {
                    edges.insert(corner(x + 1, y), corner(x + 1, y + 1));
                }
                if !b(ix, iy + 1) {
                    edges.insert(corner(x + 1, y + 1), corner(x, y + 1));
                }
                if !b(ix - 1, iy) {
                    edges.insert(corner(x, y + 1), corner(x, y));
                }
            }
        }
        let pt = |c: (usize, usize)| p2(g.x0 + c.0 as f64 * g.h, g.y0 + c.1 as f64 * g.h);
        let mut blocked = Region { loops: Vec::new() };
        while let Some((&s, _)) = edges.iter().next() {
            let mut cs = vec![s];
            let mut e = s;
            while let Some(n) = edges.remove(&e) {
                if n == s {
                    break;
                }
                cs.push(n);
                e = n;
            }
            // merge collinear runs
            let mut keep = Vec::new();
            let m = cs.len();
            for i in 0..m {
                let (p, q, r) = (cs[(i + m - 1) % m], cs[i], cs[(i + 1) % m]);
                let d1 = (q.0 as i64 - p.0 as i64, q.1 as i64 - p.1 as i64);
                let d2 = (r.0 as i64 - q.0 as i64, r.1 as i64 - q.1 as i64);
                if d1.0 * d2.1 - d1.1 * d2.0 != 0 {
                    keep.push(q);
                }
            }
            let n = keep.len();
            if n < 4 {
                continue;
            }
            blocked.loops.push(Loop {
                segs: (0..n)
                    .map(|i| Seg::Line {
                        a: pt(keep[i]),
                        b: pt(keep[(i + 1) % n]),
                    })
                    .collect(),
            });
        }
        let outer = Region {
            loops: vec![Loop {
                segs: vec![
                    Seg::Line {
                        a: p2(area[0], area[1]),
                        b: p2(area[2], area[1]),
                    },
                    Seg::Line {
                        a: p2(area[2], area[1]),
                        b: p2(area[2], area[3]),
                    },
                    Seg::Line {
                        a: p2(area[2], area[3]),
                        b: p2(area[0], area[3]),
                    },
                    Seg::Line {
                        a: p2(area[0], area[3]),
                        b: p2(area[0], area[1]),
                    },
                ],
            }],
        };
        let free = if blocked.loops.is_empty() {
            outer
        } else {
            let d = boolean(&outer, &blocked, Op::Diff).map_err(|e| OpError::Region(format!("{e:?}")))?;
            region_of(&d)
        };
        if free.loops.is_empty() {
            continue;
        }
        if rf > 0.0
            && stepover <= rf
            && let Ok((v, _)) = offset(&free, -rf)
        {
            voids.push((region_of(&v), z));
        }
        // passes: the free region's boundary, then its offsets inward by k·stepover
        let mut k = 0usize;
        loop {
            let loops: Vec<Vec<Seg>> = if k == 0 {
                free.loops.iter().map(|l| l.segs.clone()).collect()
            } else {
                let (o, _) =
                    offset(&free, k as f64 * stepover).map_err(|e| OpError::Region(format!("{e:?}")))?;
                o.into_iter()
                    .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b)))
                    .map(|l| l.segs)
                    .collect()
            };
            if loops.is_empty() {
                break;
            }
            for l in loops {
                // the loop as it is, arcs as arcs (the stock's void rests on the exact loops)
                let st = l[0].start();
                let mut mv = Vec::new();
                for sg in &l {
                    match *sg {
                        Seg::Line { b, .. } => mv.push(PMove::Line {
                            to: [b.x, b.y, z],
                            f: c.feed.0,
                            phase: Phase::Cut,
                        }),
                        Seg::Arc { b, c: ctr, ccw, .. } => mv.push(PMove::Arc {
                            to: [b.x, b.y, z],
                            c: [ctr.x, ctr.y],
                            cw: !ccw,
                            f: c.feed.0,
                        }),
                    }
                }
                runs.push(([st.x, st.y, z], mv));
            }
            k += 1;
            if k > 10_000 {
                return Err(OpError::Region("too many passes".into()));
            }
        }
    }
    if runs.is_empty() {
        return Err(OpError::ToolTooBig {
            tool_d: 2.0 * sf.check.r,
            why: "nothing to rough between these heights".into(),
        });
    }
    // each loop entered from the retract height, left back up to it
    let h = c.h;
    let s0 = runs[0].0;
    let mut mv = Vec::new();
    for (st, ms) in runs {
        mv.push(PMove::Rapid {
            to: [st[0], st[1], h.retract],
        });
        mv.push(PMove::Line {
            to: st,
            f: c.feed.1,
            phase: Phase::Plunge,
        });
        let end = match ms.last() {
            Some(PMove::Line { to, .. }) | Some(PMove::Arc { to, .. }) => *to,
            _ => st,
        };
        mv.extend(ms);
        mv.push(PMove::Line {
            to: [end[0], end[1], h.retract],
            f: c.feed.0,
            phase: Phase::Link,
        });
    }
    mv.push(PMove::Rapid {
        to: [s0[0], s0[1], h.clear],
    });
    let p = OpPath {
        tool: c.tidx,
        start: [s0[0], s0[1], h.clear],
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    };
    Ok((p, voids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_covers_the_box() {
        let r = raster([0.0, 0.0, 10.0, 4.0], 1.0, 0.0).unwrap();
        assert_eq!(r.len(), 10);
        assert_eq!(r[0], [0.0, 0.0]);
        assert_eq!(r[8], [0.0, 4.0]);
        assert_eq!(r[9], [10.0, 4.0]);
    }
}
