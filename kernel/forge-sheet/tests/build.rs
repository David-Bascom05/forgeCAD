//! Folded and flat solids against closed forms: flat length, bend allowance, the volume identity
//! V_fold - V_flat = theta W t^2 (1/2 - K), the fold map's round trip; and check, closed meshes,
//! names under edits, typed refusals, and a fuzz on random parts.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, End, FeatureId, Gauge, Map, Sheet, SheetError, TrimCurveKind, TrimRef, WallId, build,
};
use forge_topo::{Body, Name};
use std::collections::BTreeSet;

fn lines(pts: &[(f64, f64)], id0: u64) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop::new(
        (0..n)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    id0 + i as u64,
                )
            })
            .collect(),
    )
}

fn rect(x: f64, y: f64, w: f64, h: f64, id0: u64) -> ProfileLoop {
    lines(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], id0)
}

fn gauge(t: f64, r: f64, k: f64) -> Gauge {
    Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    }
}

/// A base `len` x `w` with flanges on its right (segment 1) and, for a U, left (segment 3) edges.
fn part(g: Gauge, len: f64, w: f64, flanges: &[(u64, f64, f64)]) -> Sheet {
    let (mut s, base) = Sheet::face(FeatureId(7), g, Frame::WORLD, &[rect(0.0, 0.0, len, w, 0)]).unwrap();
    for &(seg, angle, h) in flanges {
        s.flange(base, seg, angle, h).unwrap();
    }
    s
}

fn checked(b: &Body) {
    let problems = forge_topo::check::check(b);
    assert!(problems.is_empty(), "{problems:?}");
    let mesh = forge_mesh::tessellate(b, &forge_mesh::Quality::default()).expect("a mesh");
    assert!(mesh.open_edges().is_empty(), "open mesh");
}

fn volume(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).expect("mass");
    (m.volume, m.err[0])
}

fn ang(d: f64) -> f64 {
    d * dmath::PI / 180.0
}

/// Closed forms for a base and flanges without cuts: reference setback rho tan(theta/2).
struct Expect {
    flat_len: f64,
    v_fold: f64,
    v_flat: f64,
}

fn expect(g: &Gauge, k: f64, len: f64, w: f64, flanges: &[(f64, f64)]) -> Expect {
    let t = g.thickness;
    let r = g.radius;
    let mut walls = len;
    let mut bends = 0.0;
    let mut zone_fold = 0.0;
    for &(angle, h) in flanges {
        let th = angle.abs();
        let rho = if angle > 0.0 { r } else { r + t };
        let s = rho * dmath::sin(th) / (1.0 + dmath::cos(th));
        walls += h - 2.0 * s;
        let ba = th * (r + k * t);
        bends += ba;
        zone_fold += th * (r * t + 0.5 * t * t) * w;
    }
    Expect {
        flat_len: walls + bends,
        v_fold: t * w * walls + zone_fold,
        v_flat: t * w * (walls + bends),
    }
}

fn x_extent(b: &Body) -> f64 {
    let xs: Vec<f64> = b.vertex_ids().map(|v| b.vertex(v).point.x).collect();
    xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min)
}

#[test]
fn l_and_u_against_closed_forms() {
    let (t, r, k, w) = (2.0, 2.0, 0.4, 30.0);
    for (flanges, faces) in [
        (vec![(1u64, ang(90.0), 30.0)], 10),
        (vec![(1, ang(45.0), 30.0)], 10),
        (vec![(1, ang(135.0), 30.0)], 10),
        (vec![(1, ang(-90.0), 30.0)], 10),
        (vec![(1, ang(90.0), 30.0), (3, ang(90.0), 50.0)], 14),
        (vec![(1, ang(60.0), 30.0), (3, ang(-120.0), 25.0)], 14),
    ] {
        let g = gauge(t, r, k);
        let s = part(g.clone(), 40.0, w, &flanges);
        let e = expect(
            &g,
            k,
            40.0,
            w,
            &flanges.iter().map(|f| (f.1, f.2)).collect::<Vec<_>>(),
        );
        let fold = build(&s, Map::Folded).unwrap();
        let flat = build(&s, Map::Flat).unwrap();
        checked(&fold);
        checked(&flat);
        assert_eq!(fold.face_ids().count(), faces, "{flanges:?}");
        assert_eq!(flat.face_ids().count(), faces);
        let ((vf, ef), (vl, el)) = (volume(&fold), volume(&flat));
        assert!(
            (vf - e.v_fold).abs() <= ef.max(1e-9) + 1e-9,
            "{flanges:?}: {vf} against {}",
            e.v_fold
        );
        assert!(
            (vl - e.v_flat).abs() <= el.max(1e-9) + 1e-9,
            "{vl} against {}",
            e.v_flat
        );
        // flat length, and the identity per bend
        assert!(
            (x_extent(&flat) - e.flat_len).abs() < 1e-11,
            "{} against {}",
            x_extent(&flat),
            e.flat_len
        );
        let identity: f64 = flanges.iter().map(|f| f.1.abs() * w * t * t * (0.5 - k)).sum();
        assert!(((e.v_fold - e.v_flat) - identity).abs() < 1e-10);
        assert!(
            ((vf - vl) - identity).abs() <= ef + el + 1e-9,
            "{flanges:?}: {} against {identity}",
            vf - vl
        );
    }
}

#[test]
fn flat_body_is_level() {
    let s = part(
        gauge(1.5, 1.0, 0.33),
        50.0,
        20.0,
        &[(1, ang(90.0), 20.0), (3, ang(-45.0), 15.0)],
    );
    let flat = build(&s, Map::Flat).unwrap();
    for v in flat.vertex_ids() {
        let z = flat.vertex(v).point.z;
        assert!(z == 0.0 || (z + 1.5).abs() < 1e-15, "{z}");
    }
}

/// The L's fold map undone by hand: base along x to its tangent line, bend round the axis at
/// (40 - s, y, R), flange up the plane x = 40.
fn unfold_l(p: Point3, t: f64, r: f64, ba: f64) -> Point3 {
    let s = r;
    let (x0, rn) = (40.0 - s, ba / (dmath::PI / 2.0));
    if p.x <= x0 + 1e-12 {
        return Point3::new(p.x, p.y, p.z);
    }
    let (dx, dz) = (p.x - x0, r - p.z);
    if p.z < r - 1e-12 && dx >= 0.0 && dz > 0.0 && dmath::atan2(dx, dz) <= dmath::PI / 2.0 - 1e-12 {
        let rho = (dx * dx + dz * dz).sqrt();
        return Point3::new(x0 + rn * dmath::atan2(dx, dz), p.y, -(rho - r));
    }
    // the flange: its reference skin is the plane x = 40, material towards +x
    let _ = t;
    Point3::new(x0 + ba + (p.z - r), p.y, -(p.x - 40.0))
}

#[test]
fn fold_unfold_round_trip() {
    let (t, r, k) = (2.0, 2.0, 0.4);
    let s = part(gauge(t, r, k), 40.0, 30.0, &[(1, ang(90.0), 30.0)]);
    let (fold, flat) = (build(&s, Map::Folded).unwrap(), build(&s, Map::Flat).unwrap());
    let ba = dmath::PI / 2.0 * (r + k * t);
    let mut worst = 0.0f64;
    for v in fold.vertex_ids() {
        let name = fold.vertex(v).name;
        let w = flat
            .vertex_ids()
            .find(|&q| flat.vertex(q).name == name)
            .expect("one name, both maps");
        let back = unfold_l(fold.vertex(v).point, t, r, ba);
        worst = worst.max(back.distance(flat.vertex(w).point));
    }
    assert!(worst < 1e-13, "{worst}");
}

fn names(b: &Body) -> (BTreeSet<Name>, BTreeSet<Name>, BTreeSet<Name>) {
    (
        b.face_ids().map(|f| b.face(f).name).collect(),
        b.edge_ids().map(|e| b.edge(e).name).collect(),
        b.vertex_ids().map(|v| b.vertex(v).name).collect(),
    )
}

#[test]
fn names_are_unique_and_stable_under_edits() {
    let g = gauge(2.0, 2.0, 0.4);
    let base = part(g.clone(), 40.0, 30.0, &[(1, ang(90.0), 30.0)]);
    let n0 = names(&build(&base, Map::Folded).unwrap());
    let b = build(&base, Map::Folded).unwrap();
    assert_eq!(n0.0.len(), b.face_ids().count());
    assert_eq!(n0.1.len(), b.edge_ids().count());
    assert_eq!(n0.2.len(), b.vertex_ids().count());
    // the same names in the flat
    assert_eq!(names(&build(&base, Map::Flat).unwrap()), n0);
    // K, the angle and the flange's height rename nothing
    let mut k = base.clone();
    k.gauge.rule = BendRule::K(0.33);
    assert_eq!(names(&build(&k, Map::Folded).unwrap()), n0);
    assert_eq!(
        names(&part(g.clone(), 40.0, 30.0, &[(1, ang(60.0), 45.0)]).build_folded()),
        n0
    );
    // a hole in the flange renames nothing that was there
    let mut holed = base.clone();
    holed
        .cut(
            WallId(1),
            ProfileLoop::new(vec![(
                Segment::Circle {
                    center: Point2::new(18.0, 15.0),
                    radius: 4.0,
                    ccw: true,
                },
                0,
            )]),
        )
        .unwrap();
    let n1 = names(&build(&holed, Map::Folded).unwrap());
    assert!(n0.0.is_subset(&n1.0) && n0.1.is_subset(&n1.1) && n0.2.is_subset(&n1.2));
    assert_eq!(n1.0.len(), n0.0.len() + 2, "the hole's two side faces");
}

trait BuildFolded {
    fn build_folded(&self) -> Body;
}

impl BuildFolded for Sheet {
    fn build_folded(&self) -> Body {
        build(self, Map::Folded).unwrap()
    }
}

#[test]
fn k_moves_nothing_folded_to_the_bit() {
    let g = gauge(2.0, 2.0, 0.4);
    let a = part(
        g.clone(),
        40.0,
        30.0,
        &[(1, ang(70.0), 30.0), (3, ang(-110.0), 20.0)],
    );
    let mut b = a.clone();
    b.gauge.rule = BendRule::K(0.25);
    let (fa, fb) = (a.build_folded(), b.build_folded());
    let pts = |x: &Body| -> Vec<(Name, [u64; 3])> {
        let mut v: Vec<_> = x
            .vertex_ids()
            .map(|v| {
                let p = x.vertex(v).point;
                (x.vertex(v).name, [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()])
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(pts(&fa), pts(&fb));
}

#[test]
fn holes_and_cuts_across_a_bend() {
    let (t, r, k, w) = (2.0, 2.0, 0.4, 30.0);
    let g = gauge(t, r, k);
    let mut s = part(g.clone(), 40.0, w, &[(1, ang(90.0), 30.0)]);
    // a hole in the base, a slanted window across the bend, a notch from the flange's far edge
    s.cut(
        WallId(0),
        ProfileLoop::new(vec![(
            Segment::Circle {
                center: Point2::new(15.0, 15.0),
                radius: 5.0,
                ccw: true,
            },
            0,
        )]),
    )
    .unwrap();
    let window = lines(&[(30.0, 4.0), (48.0, 8.0), (47.0, 12.0), (29.0, 9.0)], 0);
    s.cut(WallId(0), window.clone()).unwrap();
    s.cut(WallId(1), rect(20.0, 20.0, 12.0, 5.0, 0)).unwrap();
    let fold = build(&s, Map::Folded).unwrap();
    let flat = build(&s, Map::Flat).unwrap();
    checked(&fold);
    checked(&flat);
    // volumes: flat area by pieces; the zone's share of the window by clipping it to the zone
    let ba = dmath::PI / 2.0 * (r + k * t);
    let x0 = 40.0 - r;
    let poly = |p: &[(f64, f64)]| -> f64 {
        let n = p.len();
        0.5 * (0..n)
            .map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1)
            .sum::<f64>()
    };
    let wpts = [(30.0, 4.0), (48.0, 8.0), (47.0, 12.0), (29.0, 9.0)];
    let in_zone = poly(&clip(&clip(&wpts, x0, 1.0), x0 + ba, -1.0));
    // the notch runs off the flange's far edge at 30: 10 by 5 of it is material
    let flat_area = (38.0 + ba + 28.0) * w - dmath::PI * 25.0 - poly(&wpts) - 50.0;
    let zone_area = ba * w - in_zone;
    let rn = ba / (dmath::PI / 2.0);
    let v_flat = t * flat_area;
    let v_fold = t * (flat_area - zone_area) + t * zone_area * (r + 0.5 * t) / rn;
    let ((vf, ef), (vl, el)) = (volume(&fold), volume(&flat));
    assert!((vl - v_flat).abs() <= el + 1e-9, "{vl} against {v_flat}");
    assert!((vf - v_fold).abs() <= ef + 1e-9, "{vf} against {v_fold}");
}

/// The polygon's part where x >= at (dir 1) or x <= at (dir -1).
fn clip(p: &[(f64, f64)], at: f64, dir: f64) -> Vec<(f64, f64)> {
    let inside = |q: (f64, f64)| (q.0 - at) * dir >= 0.0;
    let mut out = Vec::new();
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        if inside(a) {
            out.push(a);
        }
        if inside(a) != inside(b) {
            let f = (at - a.0) / (b.0 - a.0);
            out.push((at, a.1 + f * (b.1 - a.1)));
        }
    }
    out
}

#[test]
fn refusals_are_typed() {
    let g = gauge(2.0, 2.0, 0.4);
    // a flange narrower than its edge leaves base material beside the bend: no relief
    let (mut s, base) = Sheet::face(
        FeatureId(1),
        g.clone(),
        Frame::WORLD,
        &[lines(
            &[(0.0, 0.0), (40.0, 0.0), (40.0, 10.0), (40.0, 30.0), (0.0, 30.0)],
            0,
        )],
    )
    .unwrap();
    s.flange(base, 2, ang(90.0), 20.0).unwrap();
    assert!(matches!(
        build(&s, Map::Folded),
        Err(SheetError::NeedsRelief { end: End::Start, .. })
    ));
    // a hole across the bend waits for wrap::Flat::Arc
    let mut s = part(g.clone(), 40.0, 30.0, &[(1, ang(90.0), 30.0)]);
    s.cut(
        WallId(0),
        ProfileLoop::new(vec![(
            Segment::Circle {
                center: Point2::new(39.0, 15.0),
                radius: 4.0,
                ccw: true,
            },
            9,
        )]),
    )
    .unwrap();
    assert!(matches!(
        build(&s, Map::Folded),
        Err(SheetError::TrimCurveUnsupported {
            curve: TrimCurveKind::Arc,
            trim: TrimRef::Cut { segment: 9, .. },
            ..
        })
    ));
    // ... but builds flat
    checked(&build(&s, Map::Flat).unwrap());
    // a relief edge on the tangent line: designed (reliefs.md), refused until decided
    let mut s = part(g.clone(), 40.0, 30.0, &[(1, ang(90.0), 30.0)]);
    s.cut(WallId(0), rect(30.0, 10.0, 8.0, 5.0, 0)).unwrap();
    assert!(matches!(
        build(&s, Map::Folded),
        Err(SheetError::TrimAlongBendLine { .. } | SheetError::TangentMismatch { .. })
    ));
    // a flange too short for its setback, and an arc edge
    let mut s = part(g, 40.0, 30.0, &[]);
    assert!(matches!(
        s.flange(WallId(0), 1, ang(90.0), 1.5),
        Err(SheetError::WallTooShort { .. })
    ));
    assert!(matches!(
        s.flange(WallId(0), 1, 0.0, 10.0),
        Err(SheetError::BendAngle { .. })
    ));
}

#[test]
fn fuzz_random_brackets() {
    let mut rng = Rng::new(0x5ee7);
    let (mut built, mut refused, mut chains) = (0, 0, 0);
    for seed in 0..300u64 {
        let t = rng.range(0.5, 3.0);
        let r = rng.range(0.3, 3.0) * t;
        let k = rng.range(0.25, 0.5);
        let (len, w) = (rng.range(30.0, 80.0), rng.range(15.0, 60.0));
        let mut flanges = vec![(
            1u64,
            rng.range(0.2, 2.9) * if rng.below(2) == 0 { 1.0 } else { -1.0 },
            rng.range(15.0, 40.0),
        )];
        if rng.below(2) == 0 {
            flanges.push((
                3,
                rng.range(0.2, 2.9) * if rng.below(2) == 0 { 1.0 } else { -1.0 },
                rng.range(15.0, 40.0),
            ));
        }
        let g = gauge(t, r, k);
        let setbacks: f64 = flanges
            .iter()
            .map(|f| forge_sheet::build::setback_of(&g, r, f.1))
            .sum();
        if setbacks > 0.6 * len
            || flanges
                .iter()
                .any(|f| forge_sheet::build::setback_of(&g, r, f.1) > 0.6 * f.2)
        {
            continue;
        }
        // flanges turned back past square must not reach each other (folded clashes are not
        // detected yet: SheetWarning::FoldedClash is planned, and check() finds them as LoopsCross)
        let back: f64 = flanges
            .iter()
            .map(|f| f.2 * (-dmath::cos(f.1.abs())).max(0.0))
            .sum();
        if back > 0.8 * len {
            continue;
        }
        let mut s = part(g.clone(), len, w, &flanges);
        // a chain: a flange on the first flange's far edge, kept short of curling back into the base
        if flanges[0].1.abs() < 1.75 && rng.below(2) == 0 {
            let a = rng.range(0.2, 1.75) * if rng.below(2) == 0 { 1.0 } else { -1.0 };
            let h = rng.range(10.0, 30.0);
            if forge_sheet::build::setback_of(&g, r, a) + forge_sheet::build::setback_of(&g, r, flanges[0].1)
                < 0.6 * flanges[0].2
                && forge_sheet::build::setback_of(&g, r, a) < 0.6 * h
            {
                s.flange(WallId(1), 1, a, h).unwrap();
                chains += 1;
            }
        }
        // cuts in separate bands along y: holes in the base, slanted windows across the right bend
        let bands = 1 + rng.below(4);
        for i in 0..bands {
            let (y0, y1) = (w * i as f64 / bands as f64, w * (i + 1) as f64 / bands as f64);
            let (lo, hi) = (y0 + 0.15 * (y1 - y0), y1 - 0.15 * (y1 - y0));
            if rng.below(2) == 0 {
                let c = Point2::new(rng.range(0.3, 0.6) * len, 0.5 * (lo + hi));
                let rad = 0.4 * (hi - lo).min(0.2 * len);
                s.cut(
                    WallId(0),
                    ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: c,
                            radius: rad,
                            ccw: true,
                        },
                        0,
                    )]),
                )
                .unwrap();
            } else {
                let x = len - rng.range(0.1, 0.3) * len;
                let reach = rng.range(5.0, 12.0);
                let slant = rng.range(-0.3, 0.3) * (hi - lo);
                let p = [
                    (x, lo),
                    (len + reach, lo + slant.max(0.0)),
                    (len + reach, hi + slant.min(0.0)),
                    (x, hi),
                ];
                s.cut(WallId(0), lines(&p, 0)).unwrap();
            }
        }
        for map in [Map::Folded, Map::Flat] {
            match build(&s, map) {
                Ok(b) => {
                    let problems = forge_topo::check::check(&b);
                    assert!(problems.is_empty(), "seed {seed} {map:?}: {problems:?}");
                    let mesh = forge_mesh::tessellate(&b, &forge_mesh::Quality::default()).expect("a mesh");
                    assert!(mesh.open_edges().is_empty(), "seed {seed} {map:?}: open mesh");
                    built += 1;
                }
                // a window corner within the tolerance of a tangent line; a hole reaching a bend
                // (circles across bends wait for wrap::Flat::Arc)
                Err(
                    SheetError::TrimTouchesBendLine { .. }
                    | SheetError::TrimAlongBendLine { .. }
                    | SheetError::TrimCurveUnsupported {
                        curve: TrimCurveKind::Arc,
                        ..
                    },
                ) if map == Map::Folded => refused += 1,
                Err(e) => panic!("seed {seed} {map:?}: {e:?}"),
            }
        }
    }
    println!("{built} built, {refused} refused, {chains} chains of three walls");
    assert!(chains > 30);
    assert!(
        built >= 400 && refused <= built / 4,
        "{built} built, {refused} refused"
    );
}

#[test]
fn bits_are_the_same_everywhere() {
    // a digest of every vertex and the volumes of a part with a bend each way and a slanted window
    let g = gauge(2.0, 2.0, 0.4);
    let mut s = part(g, 40.0, 30.0, &[(1, ang(70.0), 30.0), (3, ang(-110.0), 20.0)]);
    s.cut(
        WallId(0),
        lines(&[(30.0, 4.0), (48.0, 8.0), (47.0, 12.0), (29.0, 9.0)], 0),
    )
    .unwrap();
    let mut h = Name::NONE;
    for map in [Map::Folded, Map::Flat] {
        let b = build(&s, map).unwrap();
        for v in b.vertex_ids() {
            let p = b.vertex(v).point;
            h = h.with(p.x.to_bits()).with(p.y.to_bits()).with(p.z.to_bits());
        }
        h = h.with(volume(&b).0.to_bits());
    }
    println!("digest {:#x}", h.0);
    assert_eq!(h.0, 0xecad_f06b_920e_37a3);
}
