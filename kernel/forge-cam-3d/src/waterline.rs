//! Waterline (contour, z-level) finishing: closed passes at constant heights.
//!
//! Method (plain, published: z-level slicing of the cutter-location surface; literature.md §4,
//! surface-finish.md §2.1). At level z, the tool tip may stand at (x, y) iff the CL height there
//! is at most z. The pass is the boundary of the region where it may not ("blocked"):
//!
//! 1. sample the drop-cutter on a grid; a node is *free* when its band is proven ≤ z (or nothing
//!    is under the tool), else blocked (undecided counts as blocked: conservative);
//! 2. marching squares gives the boundary's topology, with the blocked side on the left (so the
//!    loops run climb-wise around a boss for a right-hand spindle: the material on the left);
//! 3. each crossing on a grid edge is bisected down to `tol_xy`, keeping the free end, so every
//!    vertex of a pass is a proven-free point within `tol_xy` of the blocked region;
//! 4. each chord between vertices is proven free (`seg3_below` at z, within tol_xy/2 below);
//!    a chord that is not is split at its midpoint pushed outward to a free point, and a chord
//!    whose midpoint lies more than `tol_xy` outside the boundary (it would leave material) is
//!    split at a point refined onto the boundary.
//!
//! The guarantee: every emitted move is proven free of gouges by the drop-cutter's own
//! certificate (step 4). The forge-cam checker certifies the whole path again against the exact
//! faces (refuse-only). What is left on the part (scallop between levels) is the scallop
//! module's bound for the slope; the deviation along a pass is at most `tol_xy` at its vertices.

use crate::drop::DropCutter;

/// One closed waterline pass at height `z`: the vertices in order (the last joins the first).
#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    pub z: f64,
    pub pts: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The sampling grid's spacing (features smaller than this may be missed, never gouged).
    pub grid: f64,
    /// How close (in xy) a vertex is to the true contour, and the chord's allowed sag outward.
    pub tol_xy: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WaterlineError {
    Bad(&'static str),
    /// A chord could not be proven free after splitting: where.
    Unproven {
        z: f64,
        at: [f64; 2],
    },
}

fn free(dc: &dyn DropCutter, p: [f64; 2], z: f64) -> bool {
    match dc.drop(p[0], p[1]) {
        None => true,
        Some(b) => b.hi <= z,
    }
}

/// A level chord proven free within half of `tol_xy` below (the certifier allows the op's
/// tolerance; a chord hugging a wall at exactly zero clearance is not provable against a face
/// known only to within its lowering's μ, and costs a full search to fail).
fn seg_free(dc: &dyn DropCutter, a: [f64; 2], b: [f64; 2], z: f64, slack: f64) -> bool {
    dc.seg3_below([a[0], a[1], z], [b[0], b[1], z], slack)
}

/// Bisects between a free point `f` and a blocked point `b` until they are within `tol` of each
/// other, and returns the free end.
fn refine(dc: &dyn DropCutter, mut f: [f64; 2], mut b: [f64; 2], z: f64, tol: f64) -> [f64; 2] {
    let mut k = 0;
    while k < 60 {
        let dx = b[0] - f[0];
        let dy = b[1] - f[1];
        if dx * dx + dy * dy <= tol * tol {
            break;
        }
        let m = [0.5 * (f[0] + b[0]), 0.5 * (f[1] + b[1])];
        if free(dc, m, z) {
            f = m;
        } else {
            b = m;
        }
        k += 1;
    }
    f
}

/// The waterline loops at height `z`.
pub fn level(dc: &dyn DropCutter, z: f64, p: Params) -> Result<Vec<Loop>, WaterlineError> {
    if !(p.grid > 0.0 && p.tol_xy > 0.0 && p.tol_xy < p.grid) {
        return Err(WaterlineError::Bad("grid/tol_xy"));
    }
    let [lo, hi] = dc.extent();
    if !(lo[0] <= hi[0] && lo[1] <= hi[1]) {
        return Ok(Vec::new());
    }
    // one free ring of nodes outside the extent closes every loop
    let x0 = lo[0] - p.grid;
    let y0 = lo[1] - p.grid;
    let nx = (((hi[0] + p.grid - x0) / p.grid).ceil() as usize) + 1;
    let ny = (((hi[1] + p.grid - y0) / p.grid).ceil() as usize) + 1;
    let node = |i: usize, j: usize| -> [f64; 2] { [x0 + p.grid * i as f64, y0 + p.grid * j as f64] };
    let mut blocked = vec![false; nx * ny];
    let mut j = 0;
    while j < ny {
        let mut i = 0;
        while i < nx {
            let on_ring = i == 0 || j == 0 || i + 1 == nx || j + 1 == ny;
            blocked[j * nx + i] = !on_ring && !free(dc, node(i, j), z);
            i += 1;
        }
        j += 1;
    }
    // edge ids: horizontal (i, j)–(i+1, j) is 2(j nx + i); vertical (i, j)–(i, j+1) is +1
    let hid = |i: usize, j: usize| 2 * (j * nx + i);
    let vid = |i: usize, j: usize| 2 * (j * nx + i) + 1;
    // next[e] = the edge the boundary goes to from edge e
    let mut next: Vec<usize> = vec![usize::MAX; 2 * nx * ny];
    let mut j = 0;
    while j + 1 < ny {
        let mut i = 0;
        while i + 1 < nx {
            // corners counter-clockwise: 0 (i,j) 1 (i+1,j) 2 (i+1,j+1) 3 (i,j+1)
            let c = [
                blocked[j * nx + i],
                blocked[j * nx + i + 1],
                blocked[(j + 1) * nx + i + 1],
                blocked[(j + 1) * nx + i],
            ];
            // cell edges counter-clockwise: 0 bottom (c0–c1), 1 right (c1–c2), 2 top (c3–c2),
            // 3 left (c0–c3); edge k lies between corners k and k+1 (mod 4)
            let e = [hid(i, j), vid(i + 1, j), hid(i, j + 1), vid(i, j)];
            // walking counter-clockwise round the cell, the boundary leaves a blocked corner k
            // (blocked k, free k+1) through edge k and enters it from the edge where the run of
            // blocked corners began. With the blocked side on the left the pass goes from the
            // edge entering a blocked run to the edge leaving it... resolved per run below.
            let mut k = 0;
            while k < 4 {
                // a run of blocked corners starts at k when c[k] is blocked and c[k-1] free
                let prev = (k + 3) % 4;
                if c[k] && !c[prev] {
                    // the run k, k+1, .. ends at the last blocked corner m
                    let mut m = k;
                    while c[(m + 1) % 4] && (m + 1) % 4 != k {
                        m = (m + 1) % 4;
                    }
                    // the boundary crosses edge `prev` (between prev free and k blocked) and edge
                    // m (between m blocked and m+1 free). Going from edge m to edge prev keeps the
                    // blocked run on the left? Moving from edge m (ccw-later) to edge prev
                    // (ccw-earlier) through the cell's interior runs clockwise about the cell
                    // centre, with the blocked corners on the outside of the turn: on the left.
                    next[e[m]] = e[prev];
                }
                k += 1;
            }
            i += 1;
        }
        j += 1;
    }
    // the crossing point on each edge, refined
    let mut pt: Vec<Option<[f64; 2]>> = vec![None; 2 * nx * ny];
    let mut loops = Vec::new();
    let mut seen = vec![false; 2 * nx * ny];
    let mut e0 = 0;
    while e0 < next.len() {
        if next[e0] == usize::MAX || seen[e0] {
            e0 += 1;
            continue;
        }
        let mut ring: Vec<[f64; 2]> = Vec::new();
        let mut e = e0;
        let mut guard = 0;
        while !seen[e] {
            seen[e] = true;
            if pt[e].is_none() {
                let k = e / 2;
                let (i, j) = (k % nx, k / nx);
                let a = node(i, j);
                let b = if e % 2 == 0 {
                    node(i + 1, j)
                } else {
                    node(i, j + 1)
                };
                let ba = blocked[j * nx + i];
                let q = if ba {
                    refine(dc, b, a, z, p.tol_xy)
                } else {
                    refine(dc, a, b, z, p.tol_xy)
                };
                pt[e] = Some(q);
            }
            ring.push(pt[e].unwrap());
            e = next[e];
            guard += 1;
            if e == usize::MAX || guard > next.len() {
                return Err(WaterlineError::Unproven { z, at: ring[0] });
            }
        }
        let ring = prove_chords(dc, ring, z, p)?;
        loops.push(Loop { z, pts: ring });
        e0 += 1;
    }
    Ok(loops)
}

/// Step 4: every chord proven free and within `tol_xy` of the contour, splitting as needed.
fn prove_chords(
    dc: &dyn DropCutter,
    ring: Vec<[f64; 2]>,
    z: f64,
    p: Params,
) -> Result<Vec<[f64; 2]>, WaterlineError> {
    let n = ring.len();
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(n);
    let mut k = 0;
    while k < n {
        let a = ring[k];
        let b = ring[(k + 1) % n];
        out.push(a);
        split(dc, a, b, z, p, 0, &mut out)?;
        k += 1;
    }
    Ok(out)
}

/// Appends the points strictly between a and b that make every chord free and close.
fn split(
    dc: &dyn DropCutter,
    a: [f64; 2],
    b: [f64; 2],
    z: f64,
    p: Params,
    depth: u32,
    out: &mut Vec<[f64; 2]>,
) -> Result<(), WaterlineError> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
    if len == 0.0 {
        return Ok(());
    }
    // the right-hand normal points to the free side (blocked on the left)
    let nr = [d[1] / len, -d[0] / len];
    let m = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
    let ok = seg_free(dc, a, b, z, 0.5 * p.tol_xy);
    // sag: a point tol_xy inside the chord's midpoint should be blocked, or the chord is too far
    let inner = [m[0] - nr[0] * p.tol_xy, m[1] - nr[1] * p.tol_xy];
    let close = len <= p.tol_xy || !free(dc, inner, z);
    if ok && close {
        return Ok(());
    }
    if depth > 40 {
        return Err(WaterlineError::Unproven { z, at: m });
    }
    let q = if !ok {
        // push the midpoint out to a free point, then back onto the contour
        let mut s = p.tol_xy;
        let mut f = m;
        let mut found = free(dc, m, z);
        while !found && s <= 4.0 * p.grid {
            f = [m[0] + nr[0] * s, m[1] + nr[1] * s];
            found = free(dc, f, z);
            s *= 2.0;
        }
        if !found {
            return Err(WaterlineError::Unproven { z, at: m });
        }
        if free(dc, m, z) {
            // the midpoint itself is free: bring it in towards the contour
            let g = [m[0] - nr[0] * p.grid, m[1] - nr[1] * p.grid];
            if free(dc, g, z) {
                m
            } else {
                refine(dc, m, g, z, p.tol_xy)
            }
        } else {
            refine(dc, f, m, z, p.tol_xy)
        }
    } else {
        // free but too far out: refine from the midpoint in to the contour
        let g = [m[0] - nr[0] * p.grid, m[1] - nr[1] * p.grid];
        if free(dc, g, z) {
            inner
        } else {
            refine(dc, m, g, z, p.tol_xy)
        }
    };
    if (q[0] == a[0] && q[1] == a[1]) || (q[0] == b[0] && q[1] == b[1]) {
        return Err(WaterlineError::Unproven { z, at: m });
    }
    split(dc, a, q, z, p, depth + 1, out)?;
    out.push(q);
    split(dc, q, b, z, p, depth + 1, out)
}

/// Levels from `top` down to `bottom` (inclusive of bottom) at most `dz` apart, equal steps.
pub fn levels(top: f64, bottom: f64, dz: f64) -> Vec<f64> {
    if !(dz > 0.0) || !(top > bottom) {
        return vec![bottom];
    }
    let n = ((top - bottom) / dz).ceil().max(1.0) as usize;
    let mut v = Vec::with_capacity(n);
    let mut k = 1;
    while k <= n {
        v.push(top - (top - bottom) * (k as f64) / (n as f64));
        k += 1;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    fn dome() -> Analytic {
        Analytic {
            r: 3.0,
            planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        }
    }

    #[test]
    fn dome_levels_are_circles_of_the_right_radius() {
        let d = dome();
        let p = Params {
            grid: 1.0,
            tol_xy: 0.002,
        };
        for &z in &[5.0, 10.0, 15.0] {
            let ls = level(&d, z, p).unwrap();
            assert_eq!(ls.len(), 1, "z={z}");
            // the CL circle: (ρ+r)² − d² = (z + r + 10)²
            let want = ((33.0f64 * 33.0) - (z + 13.0) * (z + 13.0)).sqrt();
            let mut sum = 0.0;
            for q in &ls[0].pts {
                let rr = (q[0] * q[0] + q[1] * q[1]).sqrt();
                // every vertex outside the CL circle, by at most tol_xy
                assert!(
                    rr >= want - 1e-9 && rr <= want + 0.0021,
                    "z={z} r={rr} want={want}"
                );
                sum += rr;
            }
            assert!(sum > 0.0);
            // blocked on the left: counter-clockwise round the boss
            let pts = &ls[0].pts;
            let mut area = 0.0;
            let mut k = 0;
            while k < pts.len() {
                let a = pts[k];
                let b = pts[(k + 1) % pts.len()];
                area += a[0] * b[1] - b[0] * a[1];
                k += 1;
            }
            assert!(area > 0.0, "ccw round a boss");
        }
    }

    #[test]
    fn every_chord_is_proven_free() {
        let d = dome();
        let p = Params {
            grid: 2.0,
            tol_xy: 0.01,
        };
        let ls = level(&d, 8.0, p).unwrap();
        for l in &ls {
            let n = l.pts.len();
            let mut k = 0;
            while k < n {
                assert!(d.seg3_below(
                    [l.pts[k][0], l.pts[k][1], 8.0],
                    [l.pts[(k + 1) % n][0], l.pts[(k + 1) % n][1], 8.0],
                    0.005
                ));
                k += 1;
            }
        }
    }

    #[test]
    fn floor_level_and_above_top() {
        let d = dome();
        let p = Params {
            grid: 1.0,
            tol_xy: 0.01,
        };
        assert!(level(&d, 25.0, p).unwrap().is_empty());
        let lv = levels(20.0, 0.0, 3.0);
        assert_eq!(lv.len(), 7);
        assert_eq!(*lv.last().unwrap(), 0.0);
    }
}
