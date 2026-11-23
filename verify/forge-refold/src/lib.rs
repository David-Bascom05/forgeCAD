//! forge-sheet's refold checker: an independent reference for "the flat pattern is correct"
//! (research/kernel-audit/audit2.md §2.0). It builds a sheet both ways and refolds the flat with
//! its own model of the fold: each wall rigidly, from the folded and flat layouts' frames; each
//! bend zone round its own axis at the neutral radius rn = BA / θ, from nothing but the layout's
//! published numbers (inside radius, angle, allowance, setback, the wall's folded frame and the
//! zone's developed place). It shares no code with the builder's `Geo::Round`, its joint frames or
//! its topology walk; it lives here, outside the kernel, used by tests. The conditions it judges, numbered as in §2.0:
//!
//! 1. refold: every vertex of the flat body, refolded, is the folded body's vertex of its name;
//! 2. isometry: the neutral layer's radius is rn, so |∂F/∂u| = 1 there;
//! 3. allowance: each zone is BA wide and its bend line lies at u = BA/2, to the bit;
//! 4. deduction: wall a's mold line lies at u = s and wall b's at u = BA - s (the two mold lines
//!    BA - 2s apart on the flat, the bend deduction);
//! 5. continuity: each zone meets its walls at u = 0 and u = BA in position and direction;
//! 7. physical K: R <= rn <= R + t (the neutral layer inside the material);
//! 8. export: one contour runs counter-clockwise, the contours close (consecutive pieces meet in
//!    one vertex class, within the spread cap 2 LINEAR), and their area times t is the flat
//!    body's volume.
//!
//! A form's own vertices are left out: formed (folded) and pierced (flat) by design.
//! (6, injectivity, is the builder's own clash and overlap checks, which refuse; it is not
//! re-judged here.) The reference is evaluated in floats: positions agree within `tol` (1e-9 of
//! the part's size, far under LINEAR), not exactly.
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::Segment;
use forge_sheet::build::{Built, Map, Place2, build_with};
use forge_sheet::pattern::outer_contour;
use forge_sheet::{Sheet, SheetError, Wall, WallId};
use std::collections::BTreeMap;

/// What the checker measured on a sheet that passed.
#[derive(Clone, Debug, PartialEq)]
pub struct Refolded {
    /// Flat vertices refolded and found.
    pub vertices: usize,
    pub bends: usize,
    /// The largest distance between a refolded vertex and its folded one.
    pub worst: f64,
}

/// Why a sheet failed the check: a build's refusal, or the conditions broken.
#[derive(Clone, Debug, PartialEq)]
pub enum RefoldError {
    Build(Map, SheetError),
    Broken(Vec<String>),
}

/// A wall's outline segment `seg`, when it is a line: a joint's mold line.
fn straight(w: &Wall, seg: u64) -> Option<(Point2, Point2)> {
    w.outline
        .iter()
        .flat_map(|l| &l.segments)
        .find(|s| s.1 == seg)
        .and_then(|s| match s.0 {
            Segment::Line { a, b } => Some((a, b)),
            _ => None,
        })
}

fn local(p: &Place2, q: Point2) -> Point2 {
    let d = q - p.origin;
    let y = Vec2::new(-p.x.y, p.x.x);
    Point2::new(d.dot(p.x), d.dot(y))
}

fn local_vec(p: &Place2, v: Vec2) -> Vec2 {
    let y = Vec2::new(-p.x.y, p.x.x);
    Vec2::new(v.dot(p.x), v.dot(y))
}

/// A wall's point at developed `q`, depth `w` below the reference skin, folded.
fn wall_at(frame: &Frame, dev: &Place2, q: Point2, w: f64) -> Point3 {
    let l = local(dev, q);
    frame.point_to_world(Point3::new(l.x, l.y, -w))
}

/// A zone, refolded: its point at (u, y, w) and its direction of increasing u there.
struct Zone {
    dev: Place2,
    ba: f64,
    len: f64,
    rn: f64,
    sigma: f64,
    rho: f64,
    /// Wall a's folded frame and developed place: where the zone starts, tangent to it.
    a: (Frame, Place2),
}

impl Zone {
    fn frame(&self, y: f64) -> (Point3, Vec3, Vec3) {
        let (fa, da) = &self.a;
        let t = wall_at(fa, da, self.dev.apply(Point2::new(0.0, y)), 0.0);
        let across = local_vec(
            da,
            self.dev.apply(Point2::new(1.0, 0.0)) - self.dev.apply(Point2::ORIGIN),
        );
        let eu = fa.vec_to_world(Vec3::new(across.x, across.y, 0.0));
        (t, eu, fa.z().get())
    }
    fn at(&self, u: f64, y: f64, w: f64) -> Point3 {
        let (t, eu, n) = self.frame(y);
        let c = t + n * (self.sigma * self.rho);
        let r = self.rho + self.sigma * w;
        let (s, k) = dmath::sin_cos(u / self.rn);
        c + (n * (-self.sigma * k) + eu * s) * r
    }
    fn along(&self, u: f64, y: f64) -> Vec3 {
        let (_, eu, n) = self.frame(y);
        let (s, k) = dmath::sin_cos(u / self.rn);
        n * (self.sigma * s) + eu * k
    }
}

/// Refold `sheet`'s flat and judge it against its folded body.
pub fn check(sheet: &Sheet) -> Result<Refolded, RefoldError> {
    let fold = build_with(sheet, Map::Folded).map_err(|e| RefoldError::Build(Map::Folded, e))?;
    let flat = build_with(sheet, Map::Flat).map_err(|e| RefoldError::Build(Map::Flat, e))?;
    judge(sheet, &fold, &flat).map_err(RefoldError::Broken)
}

/// Judge a folded and a flat build of `sheet` against each other (`check` builds them).
pub fn judge(sheet: &Sheet, fold: &Built, flat: &Built) -> Result<Refolded, Vec<String>> {
    let mut bad: Vec<String> = Vec::new();
    let t = sheet.gauge.thickness;
    let size = fold
        .body
        .vertices
        .iter()
        .map(|v| v.point.to_vec().length())
        .fold(1.0, f64::max);
    let tol = 1e-9 * size;
    // one developed plane, the two maps' layouts placing it alike
    if fold.layout.walls.len() != flat.layout.walls.len()
        || fold.layout.bends.len() != flat.layout.bends.len()
    {
        bad.push("the two layouts differ in walls or bends".into());
        return Err(bad);
    }
    let mut walls: BTreeMap<WallId, (Frame, Frame, Place2)> = BTreeMap::new();
    for (f, l) in fold.layout.walls.iter().zip(&flat.layout.walls) {
        if f.wall != l.wall || f.developed != l.developed {
            bad.push(format!(
                "wall {:?}: developed places differ between the maps",
                f.wall
            ));
        }
        walls.insert(f.wall, (f.frame, l.frame, f.developed));
    }
    let mut zones = Vec::new();
    for (b, l) in fold.layout.bends.iter().zip(&flat.layout.bends) {
        if b != l {
            bad.push(format!("bend {:?}: its numbers differ between the maps", b.joint));
        }
        let theta = b.angle.abs();
        let rn = b.allowance / theta;
        let (r, sigma) = (b.radius, b.angle.signum());
        // 7: the neutral layer inside the material
        if !(rn >= r - tol && rn <= r + t + tol) {
            bad.push(format!(
                "bend {:?}: neutral radius {rn} outside [{r}, {}] (K {})",
                b.joint,
                r + t,
                (rn - r) / t
            ));
        }
        let rho = if sigma > 0.0 { r } else { r + t };
        // 2: at the neutral depth the refolded radius is rn (the depth from the reference skin is
        // rn - R toward it, R + t - rn away)
        let wn = if sigma > 0.0 { rn - r } else { r + t - rn };
        if ((rho + sigma * wn) - rn).abs() > 1e-12 * rn.max(1.0) {
            bad.push(format!("bend {:?}: the neutral layer is not at rn", b.joint));
        }
        let Some(&(fa, _, da)) = walls.get(&b.a) else {
            bad.push(format!("bend {:?}: no wall a", b.joint));
            continue;
        };
        let z = Zone {
            dev: b.developed,
            ba: b.allowance,
            len: b.length,
            rn,
            sigma,
            rho,
            a: (fa, da),
        };
        // 4: the mold lines at u = s and u = BA - s
        let joint = sheet.joints.iter().find(|j| j.id == b.joint);
        for (wid, seg, want) in joint
            .into_iter()
            .flat_map(|j| [(j.a, j.on_a, b.setback), (j.b, j.on_b, b.allowance - b.setback)])
        {
            let (Some(w), Some(&(_, _, dw))) = (sheet.walls.iter().find(|w| w.id == wid), walls.get(&wid))
            else {
                continue;
            };
            if let Some((p, q)) = straight(w, seg) {
                for e in [p, q] {
                    let u = local(&z.dev, dw.apply(e)).x;
                    if (u - want).abs() > tol {
                        bad.push(format!(
                            "bend {:?}: wall {wid:?}'s mold line at u = {u}, not {want}",
                            b.joint
                        ));
                    }
                }
            }
        }
        // 5: position and direction at both tangent lines, both ends, both skins
        if let Some(&(fb, _, db)) = walls.get(&b.b) {
            for y in [0.0, b.length] {
                for w in [0.0, t] {
                    for (u, (f, d)) in [(0.0, (fa, da)), (b.allowance, (fb, db))] {
                        let got = z.at(u, y, w);
                        let want = wall_at(&f, &d, z.dev.apply(Point2::new(u, y)), w);
                        if got.distance(want) > tol {
                            bad.push(format!(
                                "bend {:?}: at u = {u}, y = {y}, w = {w} the zone is {} off its wall",
                                b.joint,
                                got.distance(want)
                            ));
                        }
                    }
                }
                let dir = local_vec(
                    &db,
                    z.dev.apply(Point2::new(1.0, 0.0)) - z.dev.apply(Point2::ORIGIN),
                );
                let wall_dir = fb.vec_to_world(Vec3::new(dir.x, dir.y, 0.0));
                let gap = (z.along(b.allowance, y) - wall_dir).length();
                if gap > 1e-9 {
                    bad.push(format!(
                        "bend {:?}: a kink of {gap} at its far tangent line",
                        b.joint
                    ));
                }
            }
        }
        zones.push(z);
    }
    // 3 and 8: the pattern
    if let Some(pat) = &flat.pattern {
        for (line, z) in pat.bends.iter().zip(&zones) {
            let (a, e) = (
                z.dev.apply(Point2::new(0.5 * z.ba, 0.0)),
                z.dev.apply(Point2::new(0.5 * z.ba, z.len)),
            );
            if line.a != a || line.b != e || line.allowance != z.ba {
                bad.push(format!(
                    "bend {:?}: its line is not the zone's middle",
                    line.joint
                ));
            }
        }
        let signed: Vec<f64> = pat
            .contours
            .iter()
            .map(|c| c.iter().map(|x| x.area_term()).sum())
            .collect();
        if signed.iter().filter(|a| **a > 0.0).count() != 1 || outer_contour(pat).is_none() {
            bad.push(format!(
                "{} contours run counter-clockwise, not one",
                signed.iter().filter(|a| **a > 0.0).count()
            ));
        }
        // consecutive pieces meet in one vertex class: each strip writes its own copy of the
        // class's point, so within the class spread cap, twice LINEAR (to rounding where no class
        // merges distinct points; past it where the one rule merged two, as a joint's mold-line
        // ends within LINEAR)
        let spread = 2.0 * forge_math::tol::LINEAR;
        for c in &pat.contours {
            for (k, x) in c.iter().enumerate() {
                let gap = x.ends().1.distance(c[(k + 1) % c.len()].ends().0);
                if gap > spread {
                    bad.push(format!("a contour opens by {gap}"));
                }
            }
        }
        if let Ok(m) = forge_topo::mass::mass(&flat.body) {
            let area: f64 = signed.iter().sum();
            if (area * t - m.volume).abs() > m.err[0] + 1e-6 * m.volume.abs().max(1.0) {
                bad.push(format!(
                    "the pattern's area times t is {}, the flat body {}",
                    area * t,
                    m.volume
                ));
            }
        }
    }
    // 1: every flat vertex, refolded, is its folded namesake
    let folded: BTreeMap<_, Point3> = fold.body.vertices.iter().map(|v| (v.name, v.point)).collect();
    let (f0, l0, d0) = match walls.values().next() {
        Some(&w) => w,
        None => return Err(vec!["no walls".into()]),
    };
    let _ = f0;
    let mut worst: f64 = 0.0;
    let mut count = 0;
    // a form's own vertices are its formed shape folded and its pierce flat, by design: not a
    // fold of each other
    let formed: Vec<(Point2, f64)> = sheet
        .forms
        .iter()
        .filter_map(|f| {
            let r1 = forge_sheet::form::rim_radius(&f.tool.shape, t)?;
            let &(_, _, dw) = walls.get(&f.wall)?;
            Some((dw.apply(f.at), r1 + t))
        })
        .collect();
    for v in &flat.body.vertices {
        let p = l0.point_to_local(v.point);
        let q = d0.apply(Point2::new(p.x, p.y));
        if formed.iter().any(|&(c, r)| c.distance(q) <= r) {
            continue;
        }
        let Some(&want) = folded.get(&v.name) else {
            bad.push(format!("flat vertex {:?} has no folded namesake", v.point));
            continue;
        };
        let w = -p.z;
        let mut best = f64::INFINITY;
        for &(fw, _, dw) in walls.values() {
            best = best.min(wall_at(&fw, &dw, q, w).distance(want));
        }
        for z in &zones {
            let l = local(&z.dev, q);
            if l.x >= -tol && l.x <= z.ba + tol {
                best = best.min(z.at(l.x, l.y, w).distance(want));
            }
        }
        if best > tol {
            bad.push(format!(
                "flat vertex {:?} refolds {best} from its folded namesake",
                v.point
            ));
        }
        worst = worst.max(best);
        count += 1;
    }
    if bad.is_empty() {
        Ok(Refolded {
            vertices: count,
            bends: zones.len(),
            worst,
        })
    } else {
        bad.truncate(20);
        Err(bad)
    }
}
