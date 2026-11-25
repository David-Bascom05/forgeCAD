//! Hems: the refold of a hem joint against the builder.
use forge_brake::model::Model;
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, FeatureId, Gauge, HemKind, HemSpec, Map, Sheet};

fn rect(w: f64, h: f64) -> ProfileLoop {
    let pts = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % 4]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

fn hemmed(kind: HemKind, up: bool) -> Sheet {
    let (mut s, base) = Sheet::face(
        FeatureId(1),
        Gauge {
            thickness: 1.0,
            radius: 1.0,
            rule: BendRule::K(0.4),
        },
        Frame::WORLD,
        &[rect(120.0, 60.0)],
    )
    .unwrap();
    s.hem(base, 1, HemSpec { kind, leg: 10.0, up }, None).unwrap();
    s
}

#[test]
fn zz_probe_hem_refold() {
    for (kind, up) in [
        (HemKind::Open { gap: 1.0 }, true),
        (HemKind::Open { gap: 1.0 }, false),
        (HemKind::Teardrop { radius: 2.0, angle: dmath::PI * 1.2 }, true),
    ] {
        let s = hemmed(kind, up);
        let m = Model::new(&s).unwrap();
        let built = forge_sheet::build_with(&s, Map::Folded).unwrap();
        let b = &m.bends[0];
        eprintln!("ZZ {kind:?} up {up}: theta {} sigma {} r {} ba {}", b.theta, b.sigma, b.r, b.ba);
        let phi: Vec<f64> = m.bends.iter().map(|b| b.theta).collect();
        let f = m.frames(&phi);
        for (wi, w) in m.walls.iter().enumerate() {
            let place = built.layout.walls.iter().find(|p| p.wall == w.id).unwrap();
            for p in [(0.0, 0.0, 0.0), (5.0, 5.0, -1.0), (3.0, 40.0, 0.0)] {
                let mine = f[wi].apply(forge_brake::geo::V3::new(p.0, p.1, p.2));
                let theirs = place.frame.point_to_world(Point3::new(p.0, p.1, p.2));
                let d = ((mine.x - theirs.x).powi(2) + (mine.y - theirs.y).powi(2) + (mine.z - theirs.z).powi(2)).sqrt();
                eprintln!("ZZ   wall {wi} d {d:e}");
            }
        }
    }
}
