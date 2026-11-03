//! Bend reliefs and flange positions: reliefs added where a wall needs one, asked-for ones of each
//! kind, and refusals, each against its closed-form volume folded and flat; K moving only the
//! flat; reliefs showing on the flat pattern; flanges stood inside, outside and bend-outside.
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, End, FeatureId, Gauge, JointId, Map, Position, Relief, ReliefKind, Sheet, SheetError, WallId,
    build_with, dxf,
};
use forge_topo::Body;

fn poly(pts: &[(f64, f64)]) -> ProfileLoop {
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
                    i as u64,
                )
            })
            .collect(),
    )
}

const T: f64 = 2.0;
const R: f64 = 2.0;
const K: f64 = 0.4;
const L: f64 = 40.0;
const W: f64 = 30.0;
const M: f64 = 18.0;
const H: f64 = 25.0;

fn gauge(k: f64) -> Gauge {
    Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(k),
    }
}

/// A base whose right edge is split at y = M; a flange up 90 off its lower part, so the base
/// keeps material beside the bend's far end.
fn part(k: f64, relief: Option<Relief>) -> Sheet {
    let base = poly(&[(0.0, 0.0), (L, 0.0), (L, M), (L, W), (0.0, W)]);
    let (mut s, w) = Sheet::face(FeatureId(5), gauge(k), Frame::WORLD, &[base]).unwrap();
    let (_, j) = s.flange(w, 1, dmath::PI / 2.0, H).unwrap();
    if let Some(r) = relief {
        s.set_relief(j, End::End, r);
    }
    s
}

fn vol(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).unwrap();
    (m.volume, m.err[0])
}

fn checked(b: &Body) {
    let p = forge_topo::check::check(b);
    assert!(p.is_empty(), "{p:?}");
}

/// Folded and flat volumes of the part with `removed` taken out of the base beside the bend.
fn closed(removed: f64, k: f64) -> (f64, f64) {
    let th = dmath::PI / 2.0;
    let s = R;
    let walls = L * W - s * M + (H - s) * M - removed;
    (
        T * walls + th * (R * T + 0.5 * T * T) * M,
        T * (walls + th * (R + k * T) * M),
    )
}

fn near(what: &str, got: (f64, f64), want: f64) {
    assert!(
        (got.0 - want).abs() <= got.1 + 1e-9,
        "{what}: {} against {want}",
        got.0
    );
}

#[test]
fn a_needed_relief_is_added_from_the_default() {
    let s = part(K, None);
    let fold = build_with(&s, Map::Folded).unwrap();
    let flat = build_with(&s, Map::Flat).unwrap();
    checked(&fold.body);
    checked(&flat.body);
    assert_eq!(fold.reliefs.len(), 1);
    let r = fold.reliefs[0];
    assert!(r.automatic && r.kind == ReliefKind::Rect && r.end == End::End && r.wall == WallId(0));
    // the default: width t, depth t + R + 0.5 from the mold line
    assert_eq!((r.width, r.depth), (T, T + R + 0.5));
    let (vf, vl) = closed(r.width * r.depth, K);
    near("folded", vol(&fold.body), vf);
    near("flat", vol(&flat.body), vl);
    assert_eq!(fold.reliefs, flat.reliefs);
    // explicitly none: refused, as before reliefs
    let none = part(K, Some(Relief::None));
    assert!(matches!(
        build_with(&none, Map::Folded),
        Err(SheetError::NeedsRelief { .. })
    ));
}

#[test]
fn each_kind_asked_for() {
    let (w, d) = (3.0, 6.0);
    let kerf = forge_sheet::ReliefRule::default().kerf;
    for (kind, removed) in [
        (ReliefKind::Rect, w * d),
        (
            ReliefKind::Obround,
            w * (d - 0.5 * w) + 0.5 * dmath::PI * (0.5 * w) * (0.5 * w),
        ),
        (ReliefKind::Tear, kerf * d),
    ] {
        let s = part(
            K,
            Some(Relief::Cut {
                kind,
                width: w,
                depth: d,
            }),
        );
        let fold = build_with(&s, Map::Folded).unwrap();
        let flat = build_with(&s, Map::Flat).unwrap();
        checked(&fold.body);
        checked(&flat.body);
        assert_eq!(fold.reliefs.len(), 1, "{kind:?}");
        assert!(!fold.reliefs[0].automatic);
        let (vf, vl) = closed(removed, K);
        near(kind.name(), vol(&fold.body), vf);
        near(kind.name(), vol(&flat.body), vl);
        // the flat pattern's outline carries the notch: its area times t is the flat volume
        let p = flat.pattern.as_ref().unwrap();
        let text = dxf(p);
        assert!(text.contains("CUT"));
        let area: f64 = p
            .contours
            .iter()
            .flatten()
            .map(|c| match c {
                forge_sheet::Curve2::Line { a, b } => 0.5 * (a.x * b.y - a.y * b.x),
                forge_sheet::Curve2::Arc { c, r, start, sweep } => {
                    let ((s0, k0), (s1, k1)) = (dmath::sin_cos(*start), dmath::sin_cos(start + sweep));
                    0.5 * r * r * sweep + 0.5 * (c.x * r * (s1 - s0) - c.y * r * (k1 - k0))
                }
                _ => unreachable!("no splines here"),
            })
            .sum();
        near(kind.name(), vol(&flat.body), T * area.abs());
    }
}

#[test]
fn k_moves_only_the_flat_with_reliefs() {
    let pts = |s: &Sheet| {
        let b = build_with(s, Map::Folded).unwrap().body;
        let mut v: Vec<_> = b
            .vertex_ids()
            .map(|x| {
                (
                    b.vertex(x).name,
                    b.vertex(x).point.x.to_bits(),
                    b.vertex(x).point.z.to_bits(),
                )
            })
            .collect();
        v.sort();
        v
    };
    for r in [
        None,
        Some(Relief::Cut {
            kind: ReliefKind::Obround,
            width: 3.0,
            depth: 6.0,
        }),
    ] {
        assert_eq!(pts(&part(0.4, r)), pts(&part(0.3, r)));
        let flat = |k| vol(&build_with(&part(k, r), Map::Flat).unwrap().body).0;
        assert_ne!(flat(0.4), flat(0.3));
    }
}

#[test]
fn reliefs_refused_by_size() {
    let s = part(
        K,
        Some(Relief::Cut {
            kind: ReliefKind::Rect,
            width: 3.0,
            depth: 1.0,
        }),
    );
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::ReliefTooShallow { joint: JointId(0), end: End::End, limit, .. }) if limit > R
    ));
    let s = part(
        K,
        Some(Relief::Cut {
            kind: ReliefKind::Rect,
            width: 0.0,
            depth: 6.0,
        }),
    );
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::ReliefWidth { .. })
    ));
}

#[test]
fn positions_move_the_edge() {
    // a 40 x 30 base, a 90 flange up off its right edge, 25 high from the inner sharp
    let th = dmath::PI / 2.0;
    for (pos, shift) in [
        (Position::Outside, 0.0),
        (Position::Inside, -T),
        (Position::BendOutside, R),
    ] {
        let base = poly(&[(0.0, 0.0), (L, 0.0), (L, W), (0.0, W)]);
        let (mut s, w) = Sheet::face(FeatureId(5), gauge(K), Frame::WORLD, &[base]).unwrap();
        s.flange_at(w, 1, th, H, R, pos).unwrap();
        let fold = build_with(&s, Map::Folded).unwrap();
        checked(&fold.body);
        let walls = (L + shift - R) * W + (H - R) * W;
        near(
            &format!("{pos:?}"),
            vol(&fold.body),
            T * walls + th * (R * T + 0.5 * T * T) * W,
        );
        // the flange's outer face: at the edge (inside), t past it (outside), the bend's far end
        // past it (bend outside)
        let xs = fold.body.vertex_ids().map(|v| fold.body.vertex(v).point.x);
        let hi = xs.fold(f64::NEG_INFINITY, f64::max);
        assert!((hi - (L + shift + T)).abs() < 1e-12, "{pos:?}: {hi}");
    }
    // going down, inside is the outer sharp on the edge, which is the kernel's own
    let base = poly(&[(0.0, 0.0), (L, 0.0), (L, W), (0.0, W)]);
    let (mut s, w) = Sheet::face(FeatureId(5), gauge(K), Frame::WORLD, &[base]).unwrap();
    s.flange_at(w, 1, -th, H, R, Position::Inside).unwrap();
    let b = build_with(&s, Map::Folded).unwrap().body;
    let hi = b
        .vertex_ids()
        .map(|v| b.vertex(v).point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((hi - L).abs() < 1e-12, "{hi}");
    // an edge whose neighbour carries a flange moves, and the flange follows: shortened by t
    // at the corner
    let base = poly(&[(0.0, 0.0), (L, 0.0), (L, W), (0.0, W)]);
    let (mut s, w) = Sheet::face(FeatureId(5), gauge(K), Frame::WORLD, &[base]).unwrap();
    let (back, _) = s.flange(w, 2, th, H).unwrap();
    s.flange_at(w, 1, th, H, R, Position::Inside).unwrap();
    let len = |s: &Sheet, wall: WallId| {
        let o = &s.wall(wall).unwrap().outline[0];
        o.segments
            .iter()
            .find_map(|g| match g {
                (Segment::Line { a, b }, 3) => Some(a.distance(*b)),
                _ => None,
            })
            .unwrap()
    };
    assert!((len(&s, back) - (L - T)).abs() < 1e-12);
    let fold = build_with(&s, Map::Folded).unwrap();
    checked(&fold.body);
    assert_eq!(fold.corners.len(), 1);
    // a flange whose own mold line is beside the edge is refused
    let base = poly(&[(0.0, 0.0), (L, 0.0), (L, W), (0.0, W)]);
    let (mut s, w) = Sheet::face(FeatureId(5), gauge(K), Frame::WORLD, &[base]).unwrap();
    let (f, _) = s.flange(w, 1, th, H).unwrap();
    assert!(matches!(
        s.flange_at(f, 0, th, H, R, Position::Inside),
        Err(SheetError::EdgeMoveBlocked { .. })
    ));
}

#[test]
fn a_relief_asked_for_where_no_wall_is_cuts_nothing() {
    let r = Relief::Cut {
        kind: ReliefKind::Rect,
        width: 3.0,
        depth: 6.0,
    };
    let mut s = part(K, Some(r));
    s.set_relief(JointId(0), End::Start, r);
    let fold = build_with(&s, Map::Folded).unwrap();
    checked(&fold.body);
    let (vf, _) = closed(3.0 * 6.0, K);
    near("start beside nothing", vol(&fold.body), vf);
}

#[test]
fn reliefs_smaller_than_the_rule_are_violations() {
    let stock = forge_sheet::Stock {
        thickness: T,
        rule: BendRule::K(K),
        radii: vec![R],
        min_flange: forge_sheet::Scaled::NONE,
        hole_to_bend: forge_sheet::Scaled::NONE,
        hole_to_edge: forge_sheet::Scaled::NONE,
        min_slot: forge_sheet::Scaled::NONE,
        min_hole: forge_sheet::Scaled::NONE,
        relief: forge_sheet::ReliefRule::default(),
        hardness: None,
    };
    let shop = forge_sheet::ShopProfile {
        stock: vec![stock],
        hardware: vec![],
        kerf: 0.2,
    };
    // the rule: t wide, t + R + 0.5 deep; asked: 1.5 wide, 3 deep
    let s = part(
        K,
        Some(Relief::Cut {
            kind: ReliefKind::Rect,
            width: 1.5,
            depth: 3.0,
        }),
    );
    let v = forge_sheet::checks::reliefs(&s, &shop.relief_rule(T).unwrap());
    assert!(
        matches!(v.as_slice(), [forge_sheet::Violation::ReliefUndersized { width, depth, min_width, min_depth, .. }]
            if *width == 1.5 && *depth == 3.0 && *min_width == T && *min_depth == T + R + 0.5),
        "{v:?}"
    );
    // the automatic one is the rule's own: nothing to say
    assert!(forge_sheet::checks::reliefs(&part(K, None), &shop.relief_rule(T).unwrap()).is_empty());
}

#[test]
fn reliefs_to_the_tangent_line_and_the_remnant() {
    // up to the bend: a rect relief ending on the tangent line, beside the zone
    for (reference, kind, removed) in [
        (forge_sheet::DepthRef::UpToBend, ReliefKind::Rect, 3.0 * R),
        (forge_sheet::DepthRef::TangentToBend, ReliefKind::Tear, 0.2 * R),
        (
            forge_sheet::DepthRef::UpToBend,
            ReliefKind::Obround,
            3.0 * R + 0.5 * dmath::PI * 1.5 * 1.5,
        ),
    ] {
        let mut s = part(
            K,
            Some(Relief::Cut {
                kind,
                width: 3.0,
                depth: 0.0,
            }),
        );
        s.rule.bend.reference = reference;
        let fold = build_with(&s, Map::Folded).unwrap();
        let flat = build_with(&s, Map::Flat).unwrap();
        checked(&fold.body);
        checked(&flat.body);
        let (vf, vl) = closed(removed, K);
        near(&format!("{reference:?} {kind:?}"), vol(&fold.body), vf);
        near(&format!("{reference:?} {kind:?}"), vol(&flat.body), vl);
    }
    // a round end tangent to the line: wall is left beside the zone, so a relief is still needed
    let mut s = part(
        K,
        Some(Relief::Cut {
            kind: ReliefKind::Obround,
            width: 3.0,
            depth: 0.0,
        }),
    );
    s.rule.bend.reference = forge_sheet::DepthRef::TangentToBend;
    // (its round end leaves wall beside the zone, between it and the tangent line)
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::NeedsRelief { .. })
    ));
    // a remnant under the minimum is taken: the relief widened past the wall's edge (here the
    // edge at y = 30, 30 - (M + 3) = 9 from the relief; the rule's minimum 10)
    let mut s = part(
        K,
        Some(Relief::Cut {
            kind: ReliefKind::Rect,
            width: 3.0,
            depth: 6.0,
        }),
    );
    s.rule.bend.min_remnant = forge_sheet::Scaled {
        per_t: 0.0,
        per_r: 0.0,
        plus: 10.0,
    };
    let fold = build_with(&s, Map::Folded).unwrap();
    checked(&fold.body);
    assert!(fold.reliefs[0].remnant_taken);
    near("remnant", vol(&fold.body), closed((W - M) * 6.0, K).0);
    // the extend option is typed, not built
    s.rule.bend.extend = true;
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::ReliefExtendUnbuilt { .. })
    ));
}
