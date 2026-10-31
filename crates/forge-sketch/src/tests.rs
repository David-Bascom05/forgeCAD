use crate::*;
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

fn line_ends(s: &Sketch, l: EntityId) -> (EntityId, EntityId) {
    match s.entity(l) {
        Some(Entity::Line { a, b }) => (a, b),
        _ => panic!(),
    }
}

/// Four lines with their own end points, joined by coincident constraints.
fn rectangle(s: &mut Sketch, w: f64, h: f64, lengths: bool) -> [EntityId; 4] {
    // rough corners so the solver has work to do
    let c = [
        p(0.3, -0.2),
        p(w - 0.4, 0.5),
        p(w + 0.2, h + 0.3),
        p(-0.5, h - 0.1),
    ];
    let lines: Vec<EntityId> = (0..4).map(|i| s.add_line_points(c[i], c[(i + 1) % 4])).collect();
    for i in 0..4 {
        let (_, end) = line_ends(s, lines[i]);
        let (start, _) = line_ends(s, lines[(i + 1) % 4]);
        s.add_constraint(Constraint::Coincident(end, start)).unwrap();
    }
    s.add_constraint(Constraint::Horizontal(lines[0])).unwrap();
    s.add_constraint(Constraint::Horizontal(lines[2])).unwrap();
    s.add_constraint(Constraint::Vertical(lines[1])).unwrap();
    s.add_constraint(Constraint::Vertical(lines[3])).unwrap();
    if lengths {
        s.add_constraint(Constraint::Length(lines[0], w)).unwrap();
        s.add_constraint(Constraint::Length(lines[1], h)).unwrap();
    }
    [lines[0], lines[1], lines[2], lines[3]]
}

fn all_met(s: &Sketch) -> bool {
    s.analyze().conflicting.is_empty()
}

#[test]
fn fully_constrained_rectangle() {
    let mut s = Sketch::new(Frame::WORLD);
    let l = rectangle(&mut s, 40.0, 25.0, true);
    let corner = line_ends(&s, l[0]).0;
    s.move_point(corner, p(0.0, 0.0)).unwrap();
    s.add_constraint(Constraint::Fixed(corner)).unwrap();
    let r = s.solve();
    assert_eq!(r.status, Status::Solved, "{r:?}");
    let a = s.analyze();
    assert_eq!(a.dof, 0);
    assert!(a.redundant.is_empty(), "{a:?}");
    assert_eq!(a.fully_constrained.len(), s.entity_ids().count());
    let far = line_ends(&s, l[1]).1;
    assert!(s.point(far).distance(p(40.0, 25.0)) < 1e-9, "{:?}", s.point(far));
    assert!(s.is_fully_constrained());
}

#[test]
fn under_constrained_triangle_moves_little() {
    let mut s = Sketch::new(Frame::WORLD);
    let pts = [
        s.add_point(p(0.0, 0.0)),
        s.add_point(p(10.0, 0.0)),
        s.add_point(p(4.0, 7.0)),
    ];
    let lines: Vec<EntityId> = (0..3)
        .map(|i| s.add_line(pts[i], pts[(i + 1) % 3]).unwrap())
        .collect();
    s.add_constraint(Constraint::Fixed(pts[0])).unwrap();
    s.add_constraint(Constraint::Length(lines[0], 12.0)).unwrap();
    let r = s.solve();
    assert_eq!(r.status, Status::Solved);
    let a = s.analyze();
    assert_eq!(a.dof, 3);
    assert!(a.fully_constrained.contains(&pts[0]));
    assert!(!a.fully_constrained.contains(&pts[2]));
    // only the constrained line stretched; the apex stayed where it was
    assert_eq!(s.point(pts[2]), p(4.0, 7.0));
    assert!(
        s.point(pts[1]).distance(p(12.0, 0.0)) < 1e-9,
        "{:?}",
        s.point(pts[1])
    );
}

#[test]
fn redundant_constraint_is_named() {
    let mut s = Sketch::new(Frame::WORLD);
    let l = rectangle(&mut s, 20.0, 10.0, true);
    let par = s.add_constraint(Constraint::Parallel(l[0], l[2])).unwrap();
    assert_eq!(s.solve().status, Status::Solved);
    let a = s.analyze();
    assert!(a.redundant.contains(&par), "{a:?}");
    assert!(a.conflicting.is_empty());
}

#[test]
fn conflicting_constraints_are_named() {
    let mut s = Sketch::new(Frame::WORLD);
    let l = s.add_line_points(p(0.0, 0.0), p(10.0, 0.0));
    let a = s.add_constraint(Constraint::Length(l, 10.0)).unwrap();
    let b = s.add_constraint(Constraint::Length(l, 12.0)).unwrap();
    assert_eq!(s.solve().status, Status::NotConverged);
    let an = s.analyze();
    assert!(
        an.conflicting.contains(&a) && an.conflicting.contains(&b),
        "{an:?}"
    );
    assert!(an.redundant.contains(&a) && an.redundant.contains(&b));
}

/// A slot: two horizontal lines tangent to two equal end arcs.
fn slot(s: &mut Sketch, len: f64, r: f64) -> Vec<EntityId> {
    let c1 = s.add_point(p(0.2, 0.1));
    let c2 = s.add_point(p(len + 0.5, -0.3));
    let tl = s.add_point(p(0.0, r + 0.4));
    let tr = s.add_point(p(len - 0.3, r));
    let br = s.add_point(p(len + 0.2, -r + 0.2));
    let bl = s.add_point(p(-0.1, -r - 0.3));
    let top = s.add_line(tr, tl).unwrap();
    let bottom = s.add_line(bl, br).unwrap();
    let right = s.add_arc(c2, br, tr, true).unwrap();
    let left = s.add_arc(c1, tl, bl, true).unwrap();
    for (line, arc) in [(top, right), (top, left), (bottom, right), (bottom, left)] {
        s.add_constraint(Constraint::Tangent(line, arc)).unwrap();
    }
    s.add_constraint(Constraint::Fixed(c1)).unwrap();
    s.add_constraint(Constraint::Distance(c1, c2, len)).unwrap();
    s.add_constraint(Constraint::Horizontal(top)).unwrap();
    s.add_constraint(Constraint::Radius(left, r)).unwrap();
    s.add_constraint(Constraint::Equal(left, right)).unwrap();
    vec![top, bottom, right, left, c1, c2]
}

#[test]
fn tangent_slot_becomes_a_profile() {
    let (len, r) = (30.0, 6.0);
    let mut s = Sketch::new(Frame::WORLD);
    slot(&mut s, len, r);
    let rep = s.solve();
    assert_eq!(rep.status, Status::Solved, "{rep:?}");
    let a = s.analyze();
    assert_eq!(a.dof, 0, "{a:?}");
    let ps = profiles(&s).unwrap();
    assert_eq!(ps.len(), 1);
    assert_eq!(ps[0].loops.len(), 1);
    assert!(ps[0].normalized().is_ok());
    let want = 2.0 * r * len + dmath::PI * r * r;
    assert!((ps[0].area() - want).abs() < 1e-7, "{} vs {want}", ps[0].area());
}

#[test]
fn plate_with_hole_and_island() {
    let mut s = Sketch::new(Frame::WORLD);
    rectangle(&mut s, 50.0, 30.0, true);
    let c = s.add_point(p(25.0, 15.0));
    s.add_circle(c, 8.0).unwrap();
    let c2 = s.add_point(p(25.0, 15.0));
    let island = s.add_circle(c2, 3.0).unwrap();
    let construction = s.add_line_points(p(-10.0, -10.0), p(100.0, 100.0));
    s.set_construction(construction, true).unwrap();
    assert_eq!(s.solve().status, Status::Solved);
    let ps = profiles(&s).unwrap();
    assert_eq!(ps.len(), 2, "plate with a hole, plus the island inside the hole");
    let plate = ps.iter().find(|p| p.loops.len() == 2).unwrap();
    assert!((plate.area() - (50.0 * 30.0 - dmath::PI * 64.0)).abs() < 1e-6);
    let isl = ps.iter().find(|p| p.loops.len() == 1).unwrap();
    assert_eq!(isl.loops[0].segments[0].1, island.0);
}

#[test]
fn extraction_errors() {
    let mut s = Sketch::new(Frame::WORLD);
    let a = s.add_point(p(0.0, 0.0));
    let b = s.add_point(p(10.0, 0.0));
    let c = s.add_point(p(5.0, 8.0));
    s.add_line(a, b).unwrap();
    let l2 = s.add_line(b, c).unwrap();
    assert!(matches!(profiles(&s), Err(ExtractError::OpenChain { .. })));
    // a bow tie crosses itself
    let mut t = Sketch::new(Frame::WORLD);
    let q = [
        t.add_point(p(0.0, 0.0)),
        t.add_point(p(10.0, 10.0)),
        t.add_point(p(10.0, 0.0)),
        t.add_point(p(0.0, 10.0)),
    ];
    for i in 0..4 {
        t.add_line(q[i], q[(i + 1) % 4]).unwrap();
    }
    assert!(matches!(profiles(&t), Err(ExtractError::SelfIntersection { .. })));
    let _ = l2;
    assert_eq!(profiles(&Sketch::new(Frame::WORLD)), Err(ExtractError::Empty));
}

#[test]
fn drag_follows_the_mouse() {
    let mut s = Sketch::new(Frame::WORLD);
    let l = rectangle(&mut s, 20.0, 10.0, false);
    let corner = line_ends(&s, l[0]).0;
    s.add_constraint(Constraint::Fixed(corner)).unwrap();
    assert_eq!(s.solve().status, Status::Solved);
    let far = line_ends(&s, l[1]).1;
    let target = p(35.0, 18.0);
    let r = s.drag(far, target).unwrap();
    assert_eq!(r.status, Status::Solved);
    assert!(s.point(far).distance(target) < 1e-9, "{:?}", s.point(far));
    assert!(all_met(&s));
    let fixed = s.point(corner);
    // the fixed corner did not move and the others squared up
    let (a, b) = line_ends(&s, l[0]);
    assert!((s.point(a).y - s.point(b).y).abs() < 1e-9);
    assert!(fixed.distance(s.point(corner)) == 0.0);
    // dragging a fixed point snaps back
    let r = s.drag(corner, p(-5.0, -5.0)).unwrap();
    assert_eq!(r.status, Status::Solved);
    assert!(s.point(corner).distance(fixed) < 1e-9);
}

#[test]
fn perturbed_sketches_resolve() {
    let mut r = Rng::new(900);
    let mut base = Sketch::new(Frame::WORLD);
    slot(&mut base, 25.0, 5.0);
    let l = rectangle(&mut base, 80.0, 40.0, true);
    let c = base.add_point(p(60.0, 20.0));
    let circ = base.add_circle(c, 4.0).unwrap();
    base.add_constraint(Constraint::Tangent(l[1], circ)).unwrap();
    let mid = base.add_point(p(40.0, 0.0));
    base.add_constraint(Constraint::Midpoint(mid, l[0])).unwrap();
    let ang = base.add_line_points(p(10.0, 10.0), p(20.0, 15.0));
    base.add_constraint(Constraint::Angle(l[0], ang, 0.5)).unwrap();
    let pl = base.add_point(p(15.0, 30.0));
    base.add_constraint(Constraint::PointOnLine(pl, ang)).unwrap();
    assert_eq!(base.solve().status, Status::Solved);
    for _ in 0..40 {
        let mut s = base.clone();
        let jittered: Vec<f64> = s.params().iter().map(|&v| v + r.range(-0.5, 0.5)).collect();
        s.set_params(&jittered).unwrap();
        let rep = s.solve();
        assert_eq!(rep.status, Status::Solved, "{rep:?}");
        assert!(all_met(&s));
    }
}

#[test]
fn every_constraint_kind_solves() {
    let mut s = Sketch::new(Frame::WORLD);
    let a = s.add_line_points(p(0.0, 0.0), p(10.0, 1.0));
    let b = s.add_line_points(p(2.0, 5.0), p(9.0, 9.0));
    let (a0, a1) = line_ends(&s, a);
    let (b0, b1) = line_ends(&s, b);
    let c1 = s.add_point(p(20.0, 20.0));
    let k1 = s.add_circle(c1, 3.0).unwrap();
    let c2 = s.add_point(p(27.0, 21.0));
    let k2 = s.add_circle(c2, 2.0).unwrap();
    let c3 = s.add_point(p(20.5, 19.5));
    let k3 = s.add_circle(c3, 1.0).unwrap();
    let q = s.add_point(p(5.0, -3.0));
    let m1 = s.add_point(p(1.0, 8.0));
    let m2 = s.add_point(p(-1.0, 12.0));
    let on = s.add_point(p(23.5, 20.0));
    use Constraint::*;
    for c in [
        Fixed(a0),
        Horizontal(a),
        Length(a, 12.0),
        Perpendicular(a, b),
        Distance(a1, b0, 5.0),
        Equal(a, b),
        DistancePointLine(q, a, 4.0),
        HorizontalDistance(a0, q, 3.0),
        Diameter(k1, 7.0),
        Tangent(k1, k2),
        Concentric(k1, k3),
        Radius(k3, 1.5),
        VerticalDistance(a0, c1, 20.0),
        HorizontalDistance(a0, c1, 20.0),
        Symmetric(m1, m2, b),
        PointOnCircle(on, k2),
        Vertical(b),
        Coincident(b1, m1),
    ] {
        s.add_constraint(c).unwrap();
    }
    let rep = s.solve();
    assert_eq!(rep.status, Status::Solved, "{rep:?}");
    assert!(all_met(&s));
    assert!((s.point(q).y + 4.0).abs() < 1e-9, "kept the side q was on");
    let d = s.point(c1).distance(s.point(c2));
    assert!(
        (d - (s.radius(k1) + s.radius(k2))).abs() < 1e-9,
        "outer tangency kept"
    );
    assert!(s.point(m1).distance(s.point(b1)) < 1e-9);
}

#[test]
fn deterministic() {
    let build = || {
        let mut s = Sketch::new(Frame::WORLD);
        slot(&mut s, 20.0, 4.0);
        rectangle(&mut s, 50.0, 30.0, true);
        s.solve();
        s.params().iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    };
    assert_eq!(build(), build());
}

#[test]
fn rejects_bad_constraints() {
    let mut s = Sketch::new(Frame::WORLD);
    let pt = s.add_point(p(0.0, 0.0));
    let l = s.add_line_points(p(0.0, 0.0), p(1.0, 0.0));
    assert_eq!(
        s.add_constraint(Constraint::Horizontal(pt)),
        Err(SketchError::WrongKind(pt))
    );
    assert_eq!(
        s.add_constraint(Constraint::Length(l, -1.0)),
        Err(SketchError::BadValue)
    );
    assert_eq!(
        s.add_constraint(Constraint::Coincident(pt, EntityId(99))),
        Err(SketchError::UnknownEntity(EntityId(99)))
    );
    let c = s.add_constraint(Constraint::Length(l, 2.0)).unwrap();
    s.remove_constraint(c).unwrap();
    assert_eq!(s.remove_constraint(c), Err(SketchError::UnknownConstraint(c)));
}

#[test]
fn implied_constraints_are_named_in_order() {
    let mut s = Sketch::new(Frame::WORLD);
    let l = rectangle(&mut s, 20.0, 10.0, true);
    let corner = line_ends(&s, l[0]).0;
    s.add_constraint(Constraint::Fixed(corner)).unwrap();
    assert_eq!(s.solve().status, Status::Solved);
    assert!(s.implied().is_empty());
    let par = s.add_constraint(Constraint::Parallel(l[0], l[2])).unwrap();
    let len = s.add_constraint(Constraint::Length(l[2], 20.0)).unwrap();
    let hor = s.add_constraint(Constraint::Horizontal(l[0])).unwrap();
    let fix = s.add_constraint(Constraint::Fixed(corner)).unwrap();
    assert_eq!(s.solve().status, Status::Solved);
    assert_eq!(s.implied(), vec![par, len, hor, fix]);
}

/// Solving after an earlier solve takes the groups that did not change and ends exactly where
/// a solve from scratch does.
#[test]
fn unchanged_groups_are_taken_from_the_last_solve() {
    let make = |w: f64| {
        let mut s = Sketch::new(Frame::WORLD);
        let a = rectangle(&mut s, 20.0, 10.0, true);
        let b = rectangle(&mut s, w, 5.0, true);
        s.add_constraint(Constraint::Fixed(line_ends(&s, a[0]).0))
            .unwrap();
        s.add_constraint(Constraint::Fixed(line_ends(&s, b[0]).0))
            .unwrap();
        s
    };
    let mut prev = make(30.0);
    assert_eq!(prev.solve().status, Status::Solved);
    for w in [30.0, 31.5] {
        let (mut fresh, mut warm) = (make(w), make(w));
        let r = fresh.solve();
        let rw = warm.solve_after(Some(&prev));
        assert_eq!(rw.status, Status::Solved);
        assert_eq!(
            fresh.params().iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            warm.params().iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert!(rw.iterations <= r.iterations);
        if w == 30.0 {
            assert_eq!(rw.iterations, 0, "nothing changed");
        }
    }
}
