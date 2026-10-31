// Corner patches' min_radius against dense curvature sampling (see README.md)
use forge_geom::surface::Surface;
use forge_math::Point2;
use forge_model::fillets;
fn main() {
    let want = std::env::args().nth(1);
    for (name, part, _) in fillets::cases() {
        if want.as_deref().is_some_and(|w| w != name) {
            continue;
        }
        let Ok(b) = part.regenerate() else { continue };
        for f in &b.faces {
            let Surface::Procedural(p) = &f.surface else {
                continue;
            };
            if p.uv_scale() == (1.0, 1.0) {
                continue;
            }
            let d = p.domain();
            let m = 300;
            let (mut kmax, mut at) = (0.0f64, Point2::new(0.0, 0.0));
            for i in 0..=m {
                for j in 0..=m {
                    let x = Point2::new(
                        d.u.lo + (d.u.hi - d.u.lo) * i as f64 / m as f64,
                        d.v.lo + (d.v.hi - d.v.lo) * j as f64 / m as f64,
                    );
                    if p.project(p.point(x)).distance(x) > 1e-9 * (d.u.hi - d.u.lo) {
                        continue;
                    }
                    let dd = p.deriv(x);
                    let Some(n) = dd.du.cross(dd.dv).normalize() else {
                        continue;
                    };
                    let n = n.get();
                    let (e, ff, g) = (dd.du.dot(dd.du), dd.du.dot(dd.dv), dd.dv.dot(dd.dv));
                    let (l, mm, nn) = (dd.duu.dot(n), dd.duv.dot(n), dd.dvv.dot(n));
                    let det = e * g - ff * ff;
                    let h = (e * nn - 2.0 * ff * mm + g * l) / (2.0 * det);
                    let k = (l * nn - mm * mm) / det;
                    let km = h.abs() + (h * h - k).max(0.0).sqrt();
                    if km > kmax {
                        kmax = km;
                        at = Point2::new(
                            (x.x - d.u.lo) / (d.u.hi - d.u.lo),
                            (x.y - d.v.lo) / (d.v.hi - d.v.lo),
                        );
                    }
                }
            }
            println!(
                "{name}: min_radius {:.3}, sampled {:.3} at ({:.3}, {:.3})",
                p.min_radius(),
                1.0 / kmax,
                at.x,
                at.y
            );
        }
    }
}
