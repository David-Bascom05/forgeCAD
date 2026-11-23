use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::*;
fn poly(c: &[Point2]) -> ProfileLoop {
    let n = c.len();
    ProfileLoop::new((0..n).map(|i| (Segment::Line { a: c[i], b: c[(i + 1) % n] }, i as u64)).collect())
}
fn main() {
    let g = Gauge { thickness: 2.0, radius: 2.0, rule: BendRule::K(0.4) };
    let p = Point2::new;
    for deg in [90.0f64, 95.0, 100.0, 120.0, 85.0, 75.0] {
        // interior angle deg at v1; both far ends square to their edges
        let a = (180.0 - deg).to_radians();
        let v1 = p(60.0, 0.0);
        let d1 = Vec2::new(a.cos(), a.sin());
        let v2 = v1 + d1 * 40.0;
        let n1 = Vec2::new(-d1.y, d1.x);
        let v3 = v2 + n1 * 15.0;
        let pts = [p(0.0, 0.0), v1, v2, v3, p(0.0, 45.0)];
        for shape in ["one0", "one1", "auto", "circ", "sq", "tear", "trim", "open"] {
            let (mut s, base) = match Sheet::face(FeatureId(1), g.clone(), Frame::WORLD, &[poly(&pts)]) { Ok(x) => x, Err(e) => { println!("{deg}: face {e:?}"); break } };
            let kind = match shape {
                "circ" => Some(CornerKind::Circular { size: None, placement: Placement::Tangent }),
                "sq" => Some(CornerKind::Square { size: None, placement: Placement::Tangent }),
                "tear" => Some(CornerKind::Tear),
                "trim" => Some(CornerKind::TrimToBend),
                "open" => Some(CornerKind::Open { gap: 0.5 }),
                _ => None,
            };
            s.corner_default = kind;
            if shape != "one1" { s.flange(base, 0, dmath::PI / 2.0, 20.0).unwrap(); }
            if shape != "one0" { s.flange(base, 1, dmath::PI / 2.0, 20.0).unwrap(); }
            let mut line = format!("{deg} {shape}:");
            for m in [Map::Folded, Map::Flat] {
                match build_with(&s, m) {
                    Ok(b) => { let pr = forge_topo::check::check(&b.body); line += &format!(" {m:?} ok({})", pr.len()); }
                    Err(e) => { let d = format!("{e:?}"); line += &format!(" {m:?} {}", &d[..d.len().min(110)]); }
                }
            }
            println!("{line}");
        }
    }
}
