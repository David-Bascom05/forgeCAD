//! Every suite part, fuzz history and fillet case meshes closed, with no degenerate triangle,
//! and every triangle within the chord of its face's surface at its centroid and at its edge
//! midpoints, on every kind of face.
use forge_geom::{Surface, SurfaceGeom};
use forge_math::{Point2, Point3, dmath};
use forge_mesh::{Quality, TriMesh, tessellate};
use forge_model::{Part, fillets, suite};
use forge_topo::{Body, FaceId};

/// The distance from `x` to the surface nearest it about `uv`: Newton on the squared distance
/// from there, each step halved until it comes closer. Any surface point bounds the distance from
/// above; this one is the nearest about the triangle, which a projection over the whole surface
/// may miss.
fn gap_from(s: &Surface, x: Point3, uv: Point2) -> f64 {
    let (mut uv, mut d) = (uv, s.deriv(uv));
    let mut best = d.p.distance(x);
    for _ in 0..30 {
        let r = d.p - x;
        let (gu, gv) = (d.du.dot(r), d.dv.dot(r));
        let (a, b, c) = (
            d.du.dot(d.du) + d.duu.dot(r),
            d.du.dot(d.dv) + d.duv.dot(r),
            d.dv.dot(d.dv) + d.dvv.dot(r),
        );
        // Newton where the Hessian is positive, Gauss-Newton otherwise
        let (a, b, c) = if a > 0.0 && a * c - b * b > 0.0 {
            (a, b, c)
        } else {
            (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv))
        };
        let det = a * c - b * b;
        if det <= 0.0 {
            break;
        }
        let step = ((c * gu - b * gv) / det, (a * gv - b * gu) / det);
        let mut t = 1.0;
        let mut moved = false;
        for _ in 0..20 {
            let q = Point2::new(uv.x - t * step.0, uv.y - t * step.1);
            let dq = s.deriv(q);
            if dq.p.distance(x) < best {
                (best, uv, d, moved) = (dq.p.distance(x), q, dq, true);
                break;
            }
            t *= 0.5;
        }
        if !moved {
            break;
        }
    }
    best
}

/// A procedural triangle's corners' parameters, each within half a period of the first's; None
/// if a corner does not project back onto itself (the triangle is then measured by projection).
fn corner_uvs(s: &Surface, p: [Point3; 3]) -> Option<[Point2; 3]> {
    let mut uv = [Point2::ORIGIN; 3];
    for k in 0..3 {
        uv[k] = s.project(p[k]);
        if s.eval(uv[k]).distance(p[k]) > 1e-7 {
            return None;
        }
    }
    let d = s.domain();
    for k in 1..3 {
        if let Some(t) = d.u.period {
            uv[k].x -= t * ((uv[k].x - uv[0].x) / t).round();
        }
        if let Some(t) = d.v.period {
            uv[k].y -= t * ((uv[k].y - uv[0].y) / t).round();
        }
    }
    Some(uv)
}

/// Smallest angle of a triangle (radians); zero when it has no area.
fn min_angle(p: [Point3; 3]) -> f64 {
    (0..3)
        .map(|i| {
            let (u, v) = (p[(i + 1) % 3] - p[i], p[(i + 2) % 3] - p[i]);
            dmath::atan2(u.cross(v).length(), u.dot(v))
        })
        .fold(f64::INFINITY, f64::min)
}

/// What is wrong with a mesh, if anything: open edges, degenerate triangles, and triangles off
/// their surface by more than the chord (the worst, in chords).
fn audit(body: &Body, m: &TriMesh, chord: f64) -> Option<String> {
    let open = m.open_edges().len();
    let (mut degenerate, mut over, mut worst) = (0, 0, 0.0f64);
    for (t, &f) in m.triangles.iter().zip(&m.face_of) {
        let s = &body.face(FaceId(f)).surface;
        let p = t.map(|i| m.positions[i as usize]);
        // the thinnest left are forced by the faces (one 1e-4 wide, three boundary samples all
        // but in a line); a collapsed triangle lies far below
        if min_angle(p) < 1e-6 {
            degenerate += 1;
        }
        for w in [[1.0 / 3.0; 3], [0.5, 0.5, 0.0], [0.0, 0.5, 0.5], [0.5, 0.0, 0.5]] {
            let x = Point3::ORIGIN + (p[0].to_vec() * w[0] + p[1].to_vec() * w[1] + p[2].to_vec() * w[2]);
            let mut d = s.distance(x);
            // the projection's nearest can be another part of the surface than the triangle's
            if d > chord
                && matches!(s, Surface::Procedural(_))
                && let Some(u) = corner_uvs(s, p)
            {
                let at = Point2::new(
                    w[0] * u[0].x + w[1] * u[1].x + w[2] * u[2].x,
                    w[0] * u[0].y + w[1] * u[1].y + w[2] * u[2].y,
                );
                d = d.min(gap_from(s, x, at));
            }
            worst = worst.max(d / chord);
            if d > chord * (1.0 + 1e-6) {
                over += 1;
                break;
            }
        }
    }
    (open + degenerate + over > 0).then(|| {
        format!("{open} open edges, {degenerate} degenerate, {over} over the chord (worst {worst:.3})")
    })
}

fn check(parts: Vec<(String, Part)>, chords: &[f64]) {
    let mut bad = Vec::new();
    for (name, part) in parts {
        let b = part.regenerate().expect("regenerates");
        for &chord in chords {
            let q = Quality {
                chord,
                ..Quality::default()
            };
            match tessellate(&b, &q) {
                Ok(m) => {
                    if let Some(e) = audit(&b, &m, chord) {
                        bad.push(format!("{name} at {chord}: {e}"));
                    }
                }
                Err(e) => bad.push(format!("{name} at {chord}: {e:?}")),
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn suite_parts() {
    let parts = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    check(parts, &[0.01, 0.25]);
}

#[test]
fn fuzz_histories() {
    let parts = [1u64, 2, 12, 15, 16]
        .into_iter()
        .map(|s| (format!("fuzz {s}"), suite::fuzz(s, 80)))
        .collect();
    check(parts, &[0.01, 0.25]);
}

#[test]
fn fillet_cases() {
    let mut parts: Vec<(String, Part)> = Vec::new();
    for (n, p, _) in fillets::cases()
        .into_iter()
        .chain(fillets::variant_cases())
        .chain(fillets::variable_cases())
    {
        if !parts.iter().any(|(m, _)| m == n) {
            parts.push((n.to_string(), p));
        }
    }
    check(parts, &[0.002, 0.01, 0.25]);
}
