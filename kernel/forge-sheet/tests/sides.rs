//! Flanges off a flange's end edges (a rack ear, a wall's mounting tab): the edge bends only from
//! the first bend's tangent line on, the stretch inside its band left to that bend. Checked
//! against closed forms folded and flat, both ways and both ends, and valid.
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, FeatureId, Gauge, Map, Sheet, build_with, setback_of};
use forge_topo::Body;

fn rect(l: f64, w: f64) -> ProfileLoop {
    let p = Point2::new;
    let c = [p(0.0, 0.0), p(l, 0.0), p(l, w), p(0.0, w)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                (
                    Segment::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

fn vol(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).unwrap();
    (m.volume, m.err[0])
}

#[test]
fn ears_off_end_edges() {
    let (t, r, k, l, w, h, e) = (1.5, 1.5, 0.42, 120.0, 80.0, 30.0, 25.0);
    let g = Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    };
    let th = dmath::PI / 2.0;
    for (side, angle) in [(2u64, th), (0, th), (2, -th), (0, -th)] {
        let (mut s, base) = Sheet::face(FeatureId(3), g.clone(), Frame::WORLD, &[rect(l, w)]).unwrap();
        let (wall, _) = s.flange(base, 1, th, h).unwrap();
        s.flange(wall, side, angle, e).unwrap();
        let fold = build_with(&s, Map::Folded).unwrap_or_else(|x| panic!("{side} {angle}: {x:?}"));
        let flat = build_with(&s, Map::Flat).unwrap();
        for b in [&fold.body, &flat.body] {
            let p = forge_topo::check::check(b);
            assert!(p.is_empty(), "{side} {angle}: {p:?}");
        }
        // on the flat, in the first bend's frame (u across from its mold line into the flange,
        // v along it): base, first zone, flange, the ear's zone and wall, less the corner relief
        // u in [-(A (r + t) + t), s1 + s2 + t], v within s2 + t of the ear's edge
        let s1 = setback_of(&g, r, th);
        let s2 = setback_of(&g, r, angle);
        let ba = th * (r + k * t);
        let span = h - s1;
        let (c0, c1) = (-(th * (r + t) + t), s1 + s2 + t);
        let cv = s2 + t;
        let over = |a0: f64, a1: f64, b0: f64, b1: f64| (a1.min(b1) - a0.max(b0)).max(0.0);
        // band widths across v near the ear: base, zone 1, flange cut where v in [w - s2 - t, w]
        let base = (l - s1) * w - over(s1 - ba - (l - s1), s1 - ba, c0, c1) * cv;
        let zone1 = ba * w - ba * cv;
        let flange = span * (w - s2) - over(s1, h, c0, c1) * t;
        let zone2 = ba * span - over(s1, h, c0, c1) * cv.min(ba);
        let ear = (e - s2) * span - over(s1, h, c0, c1) * (cv - ba).max(0.0);
        let fold_zone = |a: f64| a * (r + 0.5 * t) / (ba / th);
        let vf = t * (base + flange + ear) + t * (fold_zone(zone1) + fold_zone(zone2));
        let vl = t * (base + zone1 + flange + zone2 + ear);
        let ((f, ef), (fl, el)) = (vol(&fold.body), vol(&flat.body));
        assert!((f - vf).abs() <= ef + 1e-9, "{side} {angle}: {f} against {vf}");
        assert!((fl - vl).abs() <= el + 1e-9, "{side} {angle}: {fl} against {vl}");
    }
}
