//! The flat pattern and its DXF, read back by a reader of its own (group codes, entities by layer,
//! de Boor for splines): every number comes back to the bit, the contours close, their area times
//! the thickness is the flat body's volume, and bend lines lie down the zones' middles. Also the
//! roles and layout the builder reports, and the shop checks.
mod common;
use common::cases::blob;
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, Curve2, FeatureId, Gauge, Map, Scaled, Sheet, ShopProfile, Skin, Stock, Strip, TrimRef,
    Violation, WallId, build_with, check, dxf,
};
use forge_topo::Body;
use std::collections::BTreeSet;

fn rect(x: f64, y: f64, w: f64, h: f64) -> ProfileLoop {
    let p = Point2::new;
    let c = [p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)];
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

fn circle(c: Point2, r: f64) -> ProfileLoop {
    ProfileLoop::new(vec![(
        Segment::Circle {
            center: c,
            radius: r,
            ccw: true,
        },
        0,
    )])
}

fn ang(d: f64) -> f64 {
    d * dmath::PI / 180.0
}

/// A U: base 40 x 30, flanges up 90 on the right and down 60 on the left; a hole across the right
/// bend, a spline blob in the base, a window in the left flange.
fn u_part(k: f64) -> Sheet {
    u_part_with(k, true)
}

fn u_part_with(k: f64, across: bool) -> Sheet {
    let g = Gauge {
        thickness: 2.0,
        radius: 2.0,
        rule: BendRule::K(k),
    };
    let (mut s, base) = Sheet::face(FeatureId(11), g, Frame::WORLD, &[rect(0.0, 0.0, 40.0, 30.0)]).unwrap();
    let (_, _) = s.flange(base, 1, ang(90.0), 25.0).unwrap();
    let (left, _) = s.flange(base, 3, ang(-60.0), 20.0).unwrap();
    if across {
        s.cut(base, circle(Point2::new(39.0, 15.0), 3.0)).unwrap();
    }
    s.cut(base, blob(Point2::new(18.0, 14.0), 6.0, &mut Rng::new(0x51)))
        .unwrap();
    s.cut(left, rect(8.0, 8.0, 6.0, 12.0)).unwrap();
    s
}

/// Group code and value pairs.
fn pairs(text: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len().is_multiple_of(2), "pairs");
    lines
        .chunks(2)
        .map(|c| (c[0].trim().parse().expect("a group code"), c[1].to_string()))
        .collect()
}

/// The ENTITIES section's entities: kind, layer, and the rest of their pairs.
/// Kind, layer, the rest.
type Entity = (String, String, Vec<(i32, String)>);

fn entities(text: &str) -> Vec<Entity> {
    let ps = pairs(text);
    let start = ps
        .windows(2)
        .position(|w| w[0] == (0, "SECTION".into()) && w[1] == (2, "ENTITIES".into()))
        .expect("an ENTITIES section")
        + 2;
    let mut out = Vec::new();
    let mut i = start;
    while ps[i] != (0, "ENDSEC".into()) {
        let kind = ps[i].1.clone();
        let mut j = i + 1;
        let mut rest = Vec::new();
        while ps[j].0 != 0 {
            rest.push(ps[j].clone());
            j += 1;
        }
        let layer = rest.iter().find(|p| p.0 == 8).expect("a layer").1.clone();
        out.push((kind, layer, rest));
        i = j;
    }
    assert_eq!(ps.last(), Some(&(0, "EOF".into())));
    out
}

fn vals(rest: &[(i32, String)], code: i32) -> Vec<f64> {
    rest.iter()
        .filter(|p| p.0 == code)
        .map(|p| p.1.parse().expect("a number"))
        .collect()
}

fn one(rest: &[(i32, String)], code: i32) -> f64 {
    vals(rest, code)[0]
}

/// A curve as the reader rebuilds it from the entity.
fn read_curve(kind: &str, rest: &[(i32, String)]) -> Curve2 {
    let p = Point2::new;
    match kind {
        "LINE" => Curve2::Line {
            a: p(one(rest, 10), one(rest, 20)),
            b: p(one(rest, 11), one(rest, 21)),
        },
        "CIRCLE" => Curve2::Arc {
            c: p(one(rest, 10), one(rest, 20)),
            r: one(rest, 40),
            start: 0.0,
            sweep: dmath::TAU,
        },
        "ARC" => {
            let (a0, a1) = (one(rest, 50).to_radians(), one(rest, 51).to_radians());
            Curve2::Arc {
                c: p(one(rest, 10), one(rest, 20)),
                r: one(rest, 40),
                start: a0,
                sweep: a1 - a0,
            }
        }
        "SPLINE" => {
            let (xs, ys) = (vals(rest, 10), vals(rest, 20));
            let w = vals(rest, 41);
            assert_eq!(xs.len(), one(rest, 73) as usize);
            assert_eq!(vals(rest, 40).len(), one(rest, 72) as usize);
            Curve2::Spline {
                degree: one(rest, 71) as usize,
                knots: vals(rest, 40),
                points: xs.iter().zip(&ys).map(|(x, y)| p(*x, *y)).collect(),
                weights: (!w.is_empty()).then_some(w),
            }
        }
        k => panic!("an entity the pattern does not write: {k}"),
    }
}

/// de Boor on the homogeneous points.
fn nurbs_at(degree: usize, knots: &[f64], pts: &[Point2], w: Option<&Vec<f64>>, t: f64) -> Point2 {
    let n = pts.len();
    let span = (degree..n)
        .rev()
        .find(|&i| knots[i] <= t)
        .unwrap_or(degree)
        .min(n - 1);
    let wt = |i: usize| w.map_or(1.0, |w| w[i]);
    let mut d: Vec<[f64; 3]> = (0..=degree)
        .map(|j| {
            let i = span - degree + j;
            [pts[i].x * wt(i), pts[i].y * wt(i), wt(i)]
        })
        .collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let den = knots[i + degree + 1 - r] - knots[i];
            let a = if den == 0.0 { 0.0 } else { (t - knots[i]) / den };
            let prev = d[j - 1];
            for (x, p) in d[j].iter_mut().zip(prev) {
                *x = (1.0 - a) * p + a * *x;
            }
        }
    }
    Point2::new(d[degree][0] / d[degree][2], d[degree][1] / d[degree][2])
}

/// Green's area term of a curve: exact for lines and arcs, a fine polyline for splines.
fn area_term(c: &Curve2) -> f64 {
    match c {
        Curve2::Line { a, b } => 0.5 * (a.x * b.y - a.y * b.x),
        Curve2::Arc { c, r, start, sweep } => {
            let (t0, t1) = (*start, start + sweep);
            let ((s0, k0), (s1, k1)) = (dmath::sin_cos(t0), dmath::sin_cos(t1));
            0.5 * r * r * sweep + 0.5 * (c.x * r * (s1 - s0) - c.y * r * (k1 - k0))
        }
        Curve2::Spline {
            degree,
            knots,
            points,
            weights,
        } => {
            let (lo, hi) = (knots[0], knots[knots.len() - 1]);
            let n = 200_000;
            let pt = |k: usize| {
                nurbs_at(
                    *degree,
                    knots,
                    points,
                    weights.as_ref(),
                    lo + (hi - lo) * k as f64 / n as f64,
                )
            };
            let mut sum = 0.0;
            let mut a = pt(0);
            for k in 1..=n {
                let b = pt(k);
                sum += 0.5 * (a.x * b.y - a.y * b.x);
                a = b;
            }
            sum
        }
    }
}

fn mass(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).expect("mass");
    (m.volume, m.err[0])
}

#[test]
fn dxf_round_trip_against_a_reader() {
    let s = u_part(0.4);
    let flat = build_with(&s, Map::Flat).unwrap();
    let pat = flat.pattern.clone().expect("a flat pattern");
    let text = dxf(&pat);
    let ents = entities(&text);
    let cut: Vec<Curve2> = ents
        .iter()
        .filter(|e| e.1 == "CUT")
        .map(|e| read_curve(&e.0, &e.2))
        .collect();
    let bend: Vec<Curve2> = ents
        .iter()
        .filter(|e| e.1 == "BEND")
        .map(|e| read_curve(&e.0, &e.2))
        .collect();
    assert_eq!(cut.len() + bend.len(), ents.len(), "only CUT and BEND layers");
    // every number back to the bit (arcs through degrees: to rounding)
    let written: Vec<&Curve2> = pat.contours.iter().flatten().collect();
    assert_eq!(cut.len(), written.len());
    for (r, w) in cut.iter().zip(&written) {
        match (r, w) {
            (Curve2::Arc { c, r, start, sweep }, Curve2::Arc { c: c2, r: r2, .. }) => {
                assert_eq!((c, r), (c2, r2));
                if sweep.abs() >= dmath::TAU * (1.0 - 1e-12) {
                    continue;
                }
                let (a, b) = w.ends();
                let at = |t: f64| {
                    let (s, k) = dmath::sin_cos(t);
                    Point2::new(c.x + r * k, c.y + r * s)
                };
                let (p, q) = (at(*start), at(start + sweep));
                let close = |x: Point2, y: Point2| x.distance(y) < 1e-12 * r.max(1.0);
                assert!(
                    (close(p, a) && close(q, b)) || (close(p, b) && close(q, a)),
                    "{r:?} {w:?}"
                );
            }
            _ => assert_eq!(r, *w),
        }
    }
    assert_eq!(bend.len(), 2);
    for (r, b) in bend.iter().zip(&pat.bends) {
        assert_eq!(r, &Curve2::Line { a: b.a, b: b.b });
        assert!((b.a.distance(b.b) - 30.0).abs() < 1e-12);
    }
    // closed contours, each piece ending where the next starts
    for c in &pat.contours {
        for (k, x) in c.iter().enumerate() {
            let y = &c[(k + 1) % c.len()];
            assert!(x.ends().1.distance(y.ends().0) < 1e-9, "{x:?} then {y:?}");
        }
    }
    // the outer contour and five holes (the hole across the bend, the blob, the window ... )
    assert_eq!(pat.contours.len(), 4, "outline, hole, blob, window");
    // area times thickness is the flat body's volume
    let area: f64 = pat.contours.iter().flatten().map(area_term).sum::<f64>().abs();
    let holes: f64 = pat
        .contours
        .iter()
        .map(|c| c.iter().map(area_term).sum::<f64>())
        .filter(|a| *a < 0.0)
        .sum();
    assert!(holes < 0.0, "holes run clockwise");
    let (v, err) = mass(&flat.body);
    assert!((2.0 * area - v).abs() <= err + 1e-6, "{} against {v}", 2.0 * area);
    // the bend lines lie in the zones' middles: half an allowance past the tangent line
    for (b, place) in pat.bends.iter().zip(&flat.layout.bends) {
        let mid = place.developed.apply(Point2::new(0.5 * place.allowance, 0.0));
        assert_eq!(b.a, mid);
        assert_eq!(b.allowance, place.allowance);
    }
}

#[test]
fn k_moves_the_pattern_only() {
    // (a cut across a bend is placed in developed terms, so it moves with K: none here)
    let (a, b) = (u_part_with(0.4, false), u_part_with(0.3, false));
    let (fa, fb) = (
        build_with(&a, Map::Folded).unwrap(),
        build_with(&b, Map::Folded).unwrap(),
    );
    assert_eq!(
        format!("{:?}", fa.body.vertices),
        format!("{:?}", fb.body.vertices)
    );
    let (pa, pb) = (
        build_with(&a, Map::Flat).unwrap().pattern.unwrap(),
        build_with(&b, Map::Flat).unwrap().pattern.unwrap(),
    );
    assert_ne!(pa, pb);
}

#[test]
fn roles_name_faces_and_edges_the_same_folded_and_flat() {
    let s = u_part(0.4);
    let (fold, flat) = (
        build_with(&s, Map::Folded).unwrap(),
        build_with(&s, Map::Flat).unwrap(),
    );
    assert_eq!(fold.roles, flat.roles);
    let faces: BTreeSet<_> = fold.body.face_ids().map(|f| fold.body.face(f).name).collect();
    let edges: BTreeSet<_> = fold.body.edge_ids().map(|e| fold.body.edge(e).name).collect();
    for (_, _, n) in &fold.roles.skins {
        assert!(faces.contains(n));
    }
    for (_, n) in &fold.roles.sides {
        assert!(faces.contains(n));
    }
    for (_, _, n) in &fold.roles.edges {
        assert!(edges.contains(n));
    }
    // every face is a skin or a side
    let roled: BTreeSet<_> = fold
        .roles
        .skins
        .iter()
        .map(|s| s.2)
        .chain(fold.roles.sides.iter().map(|s| s.1))
        .collect();
    assert_eq!(roled, faces);
    // each wall and bend has both skins; the first wall's far edge has edges on both skins
    for st in [
        Strip::Wall(WallId(0)),
        Strip::Wall(WallId(1)),
        Strip::Bend(forge_sheet::JointId(0)),
    ] {
        assert!(!fold.roles.skin(st, Skin::Reference).is_empty());
        assert!(!fold.roles.skin(st, Skin::Back).is_empty());
    }
    let tip = TrimRef::Outline {
        wall: WallId(1),
        segment: 1,
    };
    assert_eq!(fold.roles.edge(tip, Skin::Reference).len(), 1);
    assert_eq!(fold.roles.side(tip).len(), 1);
    // the hole across the bend has side faces in the base and the zone
    let hole = TrimRef::Cut {
        cut: forge_sheet::CutId(0),
        segment: 0,
    };
    assert!(!fold.roles.side(hole).is_empty());
}

#[test]
fn layout_places_each_wall_on_its_reference_skin() {
    let s = u_part(0.4);
    for map in [Map::Folded, Map::Flat] {
        let b = build_with(&s, map).unwrap();
        for w in &b.layout.walls {
            for name in b.roles.skin(Strip::Wall(w.wall), Skin::Reference) {
                let f = b
                    .body
                    .face_ids()
                    .find(|&f| b.body.face(f).name == name)
                    .expect("the skin");
                let body = &b.body;
                let verts = body.face(f).loops.iter().flat_map(|&l| {
                    body.lp(l)
                        .coedges()
                        .map(|c| body.edge(body.coedge(c).edge).start)
                        .collect::<Vec<_>>()
                });
                for v in verts {
                    let p = w.frame.point_to_local(body.vertex(v).point);
                    assert!(p.z.abs() < 1e-12, "{map:?} {:?}: {}", w.wall, p.z);
                }
            }
        }
    }
}

#[test]
fn shop_checks() {
    let scaled = |per_t: f64| Scaled { per_t, plus: 0.0 };
    let stock = Stock {
        thickness: 2.0,
        rule: BendRule::K(0.4),
        radii: vec![2.0, 4.0],
        min_flange: scaled(11.0),
        hole_to_bend: scaled(2.0),
        hole_to_edge: scaled(1.0),
        min_slot: scaled(1.0),
        relief_width: scaled(1.0),
        relief_depth: scaled(1.0),
    };
    let shop = ShopProfile { stock: vec![stock] };
    // flanges 25 and 20 long; the die needs 22
    let s = u_part(0.4);
    let v = check(&s, &shop);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(
        matches!(v[0], Violation::FlangeTooShort { wall: WallId(2), length, limit, .. }
        if (length - 20.0).abs() < 1e-12 && limit == 22.0)
    );
    // a radius not tooled, a thickness not stocked
    let mut r = s.clone();
    r.gauge.radius = 3.0;
    for j in &mut r.joints {
        if let forge_sheet::JointKind::Bend { radius, .. } = &mut j.kind {
            *radius = 3.0;
        }
    }
    assert_eq!(
        check(&r, &shop)
            .iter()
            .filter(|v| matches!(v, Violation::RadiusNotTooled { nearest, .. } if *nearest == 2.0))
            .count(),
        2
    );
    let mut t = s.clone();
    t.gauge.thickness = 1.5;
    assert_eq!(
        check(&t, &shop),
        vec![Violation::ThicknessNotStocked {
            feature: FeatureId(11),
            thickness: 1.5,
            nearest: 2.0
        }]
    );
}
