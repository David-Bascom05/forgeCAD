//! Two closed spline blobs crossing twice, about 300 mm out and much further: the crossings must
//! not depend on where the pair sits. A fixed Newton stop lost one of them at 300 mm.
use forge_geom::spline::Spline;
use forge_geom::spline::meet::cross_sections;
use forge_math::{Point3, Vec3};

fn closed(c: (f64, f64), v: &[(f64, f64)]) -> Spline {
    let mut p: Vec<Point3> = v
        .iter()
        .map(|&(x, y)| Point3::new(c.0 + x, c.1 + y, 0.0))
        .collect();
    p.push(p[0]);
    Spline::through(&p, None, None).unwrap()
}

fn pair() -> (Spline, Spline) {
    let a = closed(
        (300.0, 150.0),
        &[
            (20.0, 0.0),
            (12.0, 15.0),
            (-5.0, 18.0),
            (-19.0, 6.0),
            (-16.0, -11.0),
            (0.0, -17.0),
            (14.0, -13.0),
        ],
    );
    let b = closed(
        (295.0, 154.0),
        &[
            (15.0, 0.0),
            (8.0, 11.0),
            (-6.0, 13.0),
            (-14.0, 2.0),
            (-9.0, -12.0),
            (6.0, -10.0),
        ],
    );
    (a, b)
}

fn crossings(a: &Spline, b: &Spline) -> Vec<f64> {
    let hits = cross_sections(a, a.domain(), b, b.domain());
    assert!(hits.iter().all(|h| !h.1), "{hits:?}");
    hits.iter().map(|h| h.0).collect()
}

#[test]
fn both_crossings_found_300_mm_out() {
    let (a, b) = pair();
    let at = crossings(&a, &b);
    assert_eq!(at.len(), 2, "{at:?}");
    for &s in &at {
        let p = a.point(s);
        let q = b.point(b.closest(p, b.domain()));
        assert!(p.distance(q) <= 1e-9, "{s}: {p:?} {q:?}");
    }
    let back = crossings(&b, &a);
    assert_eq!(back.len(), 2, "{back:?}");
}

#[test]
fn same_crossings_far_out() {
    let (a, b) = pair();
    let near = crossings(&a, &b);
    for d in [-300.0, 1e3, 1e4, 1e5] {
        let m = Vec3::new(d, 0.5 * d, 0.0);
        let (fa, fb) = (a.mapped(|p| p + m), b.mapped(|p| p + m));
        let far = crossings(&fa, &fb);
        assert_eq!(far.len(), near.len(), "moved {d}: {far:?}");
        for (x, y) in near.iter().zip(&far) {
            assert!((x - y).abs() <= 1e-6, "moved {d}: {near:?} {far:?}");
        }
    }
}
