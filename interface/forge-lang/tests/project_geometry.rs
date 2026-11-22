//! `project(..)` in a sketch: geometry from the part above, exact, through rebuilds and edits.
mod common;

use forge_lang::build::Cache;
use forge_lang::{Options, Run};
use forge_math::{Point2, dmath};
use forge_model::{Feature, Part};
use forge_sketch::Entity;

/// The run, which must be free of errors.
fn ok(src: &str) -> Run {
    let (r, _) = common::run(src);
    assert!(
        r.errors.is_empty(),
        "{}\n{src}",
        forge_lang::report::text(&r, src)
    );
    r
}

/// The run's error text.
fn error_of(src: &str) -> String {
    common::refused(src)
}

/// The volume of all the part's bodies.
fn volume(r: &Run) -> f64 {
    r.measure.as_ref().expect("measured").volume
}

/// The part's last sketch, as the kernel takes it.
fn last_sketch(src: &str) -> forge_model::Sketch {
    let (r, cache) = common::run(src);
    assert!(
        r.errors.is_empty(),
        "{}\n{src}",
        forge_lang::report::text(&r, src)
    );
    let (f, _) = cache.history();
    f.iter()
        .rev()
        .find_map(|f| match f {
            Feature::Sketch(s) => Some((**s).clone()),
            _ => None,
        })
        .expect("a sketch")
}

/// The run's sketches all fully held: no UnderConstrained warning.
fn no_freedom(r: &Run) {
    let free: Vec<_> = r
        .warnings
        .iter()
        .filter(|w| matches!(w, forge_lang::error::Warning::UnderConstrained { .. }))
        .collect();
    assert!(free.is_empty(), "{free:?}");
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} against {b}");
}

fn id(s: &forge_model::Sketch, t: &str) -> forge_sketch::EntityId {
    s.names
        .iter()
        .find(|n| n.0 == t)
        .unwrap_or_else(|| panic!("no {t} in {:?}", s.names))
        .1
}

fn entity(s: &forge_model::Sketch, t: &str) -> Entity {
    s.solved.entity(id(s, t)).unwrap()
}

/// A named point, a circle's or arc's centre, or a line's end (`l.a`, `l.b`).
fn at(s: &forge_model::Sketch, t: &str) -> Point2 {
    let (name, end) = match t.split_once('.') {
        Some((n, e)) if e == "a" || e == "b" => (n, Some(e)),
        _ => (t, None),
    };
    let p = match (entity(s, name), end) {
        (Entity::Point { .. }, None) => id(s, name),
        (Entity::Line { a, .. }, Some("a")) => a,
        (Entity::Line { b, .. }, Some("b")) => b,
        (Entity::Circle { center, .. } | Entity::Arc { center, .. }, None) => center,
        (e, _) => panic!("{t}: {e:?}"),
    };
    s.solved.point(p)
}

/// An ellipse's centre, major end and semi-minor axis.
fn ellipse_of(s: &forge_model::Sketch, t: &str) -> (Point2, Point2, f64) {
    let Entity::Ellipse { center, major, b, .. } = entity(s, t) else {
        panic!("{t} is no ellipse: {:?}", entity(s, t))
    };
    (
        s.solved.point(center),
        s.solved.point(major),
        s.solved.params()[b],
    )
}

/// How far a point is off an ellipse, in millimetres near it.
fn on_ellipse(s: &forge_model::Sketch, t: &str, p: Point2) -> f64 {
    let (c, m, b) = ellipse_of(s, t);
    let a = c.distance(m);
    let (ux, uy) = ((m.x - c.x) / a, (m.y - c.y) / a);
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let (u, v) = (dx * ux + dy * uy, dy * ux - dx * uy);
    ((u * u / (a * a) + v * v / (b * b)).sqrt() - 1.0).abs() * a
}

/// A sketch `NAME` holding the point (x, y, z) as `p`, on a plane square to z.
fn point(name: &str, x: f64, y: f64, z: f64) -> String {
    format!("sketch {name}(on: offset(XY, {z:?})) {{\n  p = pt({x:?}, {y:?})\n}}\n")
}

/// Planes for anywhere in a part, before its first feature: the world's, not linted after it.
const PLANES: &str = "XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)\nXZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)\n";

/// A block 40 x 30 x 10 on the origin: sketch `s` (rect `r`), extrude `base`.
const BLOCK: &str = "XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)\nXZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)\nsketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 30))\n}\nbase = extrude(s, 10)\n";

/// The block's top edge along x at y = 0.
const FRONT_TOP: &str = "edges(between: (base.end, base.side(s.r[0])))";
/// The block's top edge along x at y = 30.
const BACK_TOP: &str = "edges(between: (base.end, base.side(s.r[2])))";

#[test]
fn a_straight_edge_gives_the_same_line_and_end_on_a_point() {
    let src = format!(
        "{BLOCK}top = {BACK_TOP}\nup = edges(between: (base.side(s.r[2]), base.side(s.r[3])))\nsketch k(on: offset(XY, 50)) {{\n  j = project(top)\n  u = project(up)\n}}\n"
    );
    let s = last_sketch(&src);
    assert!(matches!(entity(&s, "j"), Entity::Line { .. }));
    assert!(matches!(entity(&s, "u"), Entity::Point { .. }));
    let (a, b) = (at(&s, "j.a"), at(&s, "j.b"));
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    assert_eq!((x0, x1, a.y, b.y), (0.0, 40.0, 30.0, 30.0));
    assert_eq!((at(&s, "u").x, at(&s, "u").y), (0.0, 30.0));
}

#[test]
fn results_hold_no_freedom_and_a_new_line_closes_on_them_exactly() {
    let src = format!(
        "{BLOCK}e = {FRONT_TOP}\nsketch k(on: offset(XY, 7)) {{\n  j = project(e)\n  l1 = line(j.b, (20, 25))\n  l2 = line(l1.b, j.a)\n}}\npit = cut(k, -3)\n"
    );
    let r = ok(&src);
    // the lines' ends are the projection's own points, so nothing is free
    no_freedom(&r);
    // a triangle 40 base, 25 high, cut 3 from the top
    close(volume(&r), 40.0 * 30.0 * 10.0 - 0.5 * 40.0 * 25.0 * 3.0);
}

#[test]
fn a_face_gives_its_boundary_and_extrudes_to_its_exact_volume() {
    let r = ok(&format!(
        "{BLOCK}top = base.end\nsketch k(on: offset(XY, 30)) {{\n  f = project(top)\n}}\next = extrude(k, 5, new: true)\n"
    ));
    close(volume(&r), 40.0 * 30.0 * 10.0 + 40.0 * 30.0 * 5.0);
    no_freedom(&r);
}

/// A block 40 x 30 x 10 with corners of 5 (sketch `s`, outline `o`: the corner at (40, 30) is
/// `o[4]`), extrude `base`.
const ROUNDED: &str = "XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)\nXZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)\nsketch s(on: xy) {\n  o = hull(disc((5, 5), 5), disc((35, 5), 5), disc((35, 25), 5), disc((5, 25), 5))\n}\nbase = extrude(s, 10)\n";

#[test]
fn a_face_with_a_hole_and_an_arc_keeps_them_exact() {
    let r = ok(&format!(
        "{ROUNDED}sketch hs(on: base.end) {{\n  p = pt(20, 15)\n}}\nh = hole(at: hs.p, d: 8, depth: thru)\ntop = base.end\nsketch k(on: offset(XY, 30)) {{\n  f = project(top)\n}}\next = extrude(k, 5, new: true)\n"
    ));
    let area = 40.0 * 30.0 - (4.0 - core::f64::consts::PI) * 25.0 - core::f64::consts::PI * 16.0;
    close(volume(&r), area * 15.0);
    // the profile's arcs: tangent to its lines at shared points, none free
    no_freedom(&r);
}

#[test]
fn circles_are_exact_square_on_lines_edge_on_and_ellipses_tilted() {
    let src = format!(
        "{BLOCK}sketch hs(on: base.end) {{\n  p = pt(20, 15)\n}}\nh = hole(at: hs.p, d: 8, depth: thru)\nrim = edges(between: (h.wall, base.end))\n"
    );
    let on = |plane: &str| {
        last_sketch(&format!(
            "{src}sketch k(on: {plane}) {{\n  j = project(rim)\n}}\n"
        ))
    };
    // parallel: the circle
    let s = on("offset(XY, 50)");
    assert!(matches!(entity(&s, "j"), Entity::Circle { .. }));
    assert_eq!((at(&s, "j").x, at(&s, "j").y), (20.0, 15.0));
    assert_eq!(s.solved.radius(id(&s, "j")), 4.0);
    // upside down too
    let s = on("offset(plane(at: xy.origin, normal: -xy.normal, x: yz.normal), -50)");
    assert_eq!((at(&s, "j").x, at(&s, "j").y), (20.0, -15.0));
    // square on: a line its diameter long
    let s = on("XZ");
    assert!(matches!(entity(&s, "j"), Entity::Line { .. }));
    let (a, b) = (at(&s, "j.a"), at(&s, "j.b"));
    close((a.x - b.x).abs(), 8.0);
    close(a.x.min(b.x), 16.0);
    close(a.y, 10.0);
    close(b.y, 10.0);
    // tilted 45 degrees: the ellipse, major 4 along x, minor 4 cos 45
    let s = last_sketch(&format!(
        "{src}{}sketch k(on: plane(at: xy.origin, normal: axis(xy.origin, q.p))) {{\n  j = project(rim)\n}}\n",
        point("q", 0.0, 1.0, 1.0)
    ));
    let (c, maj, b) = ellipse_of(&s, "j");
    let want = s.plane.point_to_local(forge_math::Point3::new(20.0, 15.0, 10.0));
    assert!(c.distance(Point2::new(want.x, want.y)) < 1e-12);
    close(c.distance(maj), 4.0);
    close(b, 4.0 * 0.5f64.sqrt());
}

#[test]
fn a_part_circle_square_on_reaches_past_its_ends_where_it_turns() {
    // the corner arcs of a rounded block seen from the side
    let s = last_sketch(&format!(
        "{ROUNDED}corner = edges(between: (base.end, base.side(s.o[4])))\nsketch k(on: XZ) {{\n  j = project(corner)\n}}\n"
    ));
    let (a, b) = (at(&s, "j.a"), at(&s, "j.b"));
    close((a.x - b.x).abs(), 5.0);
    close(a.x.max(b.x), 40.0);
}

#[test]
fn an_arc_keeps_its_way_round_from_either_side() {
    for (plane, ccw) in [
        ("offset(XY, 50)", true),
        (
            "offset(plane(at: xy.origin, normal: -xy.normal, x: yz.normal), -50)",
            false,
        ),
    ] {
        let s = last_sketch(&format!(
            "{ROUNDED}corner = edges(between: (base.end, base.side(s.o[4])))\nsketch k(on: {plane}) {{\n  j = project(corner)\n}}\n"
        ));
        let Entity::Arc { ccw: way, .. } = entity(&s, "j") else {
            panic!("an arc")
        };
        assert_eq!(way, ccw, "{plane}");
    }
}

#[test]
fn it_follows_an_edit_above() {
    // one session: the projection is read afresh when the block's width changes
    let src = |w: f64| {
        format!(
            "{PLANES}shared w = {w:?}\nsketch s(on: xy) {{\n  r = rect(corner: (0, 0), size: (w, 30))\n}}\nbase = extrude(s, 10)\ne = edges(between: (base.end, base.side(s.r[1])))\nsketch k(on: offset(XY, 30)) {{\n  j = project(e)\n}}\n"
        )
    };
    let dir = std::env::temp_dir();
    let opts = || Options {
        dir: Some(dir.clone()),
        ..Options::default()
    };
    let (r, cache) = forge_lang::run(&src(40.0), opts(), Cache::default());
    assert!(r.errors.is_empty());
    let sk = |cache: &Cache| {
        cache
            .history()
            .0
            .iter()
            .rev()
            .find_map(|f| match f {
                Feature::Sketch(s) => Some((**s).clone()),
                _ => None,
            })
            .expect("a sketch")
    };
    assert_eq!(at(&sk(&cache), "j.a").x, 40.0);
    let (r, cache) = forge_lang::run(&src(60.0), opts(), cache);
    assert!(r.errors.is_empty());
    let s = sk(&cache);
    assert_eq!(
        (at(&s, "j.a").x, at(&s, "j.b").x),
        (60.0, 60.0),
        "re-derived from the edit"
    );
}

#[test]
fn a_face_follows_a_new_edge_and_its_extrusions_faces_keep_their_names() {
    let tail = "top = base.end\nsketch k(on: offset(XY, 30)) {\n  f = project(top)\n}\next = extrude(k, 5, new: true)\n";
    let faces = |src: &str| -> Vec<u64> {
        let (r, cache) = common::run(src);
        assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
        let (f, ids) = cache.history();
        let solids = Part { features: f.to_vec() }
            .regenerate_solids_ids(Some(ids))
            .expect("builds");
        let b = &solids.last().unwrap().body;
        b.face_ids().map(|f| b.face(f).name.0).collect()
    };
    let before = faces(&format!("{BLOCK}{tail}"));
    let corner = "corner = chamfer(edges(between: (base.side(s.r[1]), base.side(s.r[2]))), 2)\n";
    let now = faces(&format!("{BLOCK}{corner}{tail}"));
    // one more side, and each side there was still has its name
    assert_eq!(now.len(), before.len() + 1);
    let kept = before.iter().filter(|n| now.contains(n)).count();
    assert!(kept >= 4, "{kept} of {}", before.len());
    let v = volume(&ok(&format!("{BLOCK}{corner}{tail}"))) - volume(&ok(&format!("{BLOCK}{tail}")));
    close(v, -2.0 * (10.0 + 5.0));
}

#[test]
fn work_features_and_sketch_points_project() {
    let src = format!(
        "{BLOCK}pt1 = workpoint(mid: {BACK_TOP})\nax = workaxis(along: edges(between: (base.side(s.r[0]), base.side(s.r[3]))))\n"
    );
    let s = last_sketch(&format!(
        "{src}sketch k(on: offset(XY, 50)) {{\n  j1 = project(pt1)\n  j2 = project(ax)\n}}\n"
    ));
    assert_eq!((at(&s, "j1").x, at(&s, "j1").y), (20.0, 30.0));
    assert_eq!((at(&s, "j2").x, at(&s, "j2").y), (0.0, 0.0));
    // an axis lying in the plane has no image there
    let e = error_of(&format!("{src}sketch k(on: XZ) {{\n  j = project(ax)\n}}\n"));
    assert!(e.contains("NoImage"), "{e}");
}

#[test]
fn constraints_work_on_results() {
    let src = format!(
        "{BLOCK}e = {FRONT_TOP}\nsketch k(on: base.end) {{\n  j = project(e)\n  p = near(5, 5)\n  distance(p, j, 7)\n  distance(j.a, p, 9)\n}}\n"
    );
    let r = ok(&src);
    no_freedom(&r);
    close(at(&last_sketch(&src), "p").y, 7.0);
    // a result is held: a dimension against what it's held to is refused
    let bad = format!(
        "{BLOCK}e = {FRONT_TOP}\nsketch k(on: base.end) {{\n  j = project(e)\n  distance(j, 41)\n}}\n"
    );
    let e = error_of(&bad);
    assert!(e.contains("SketchConflict"), "{e}");
}

/// A small generator, so this needs no dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / (1u64 << 31) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// A plane through (ox, oy, oz) square to (dx, dy, dz), x as `plane(..)` takes it: the sketches
/// that hold its points, and the plane.
fn tilted(o: (f64, f64, f64), d: (f64, f64, f64)) -> (String, &'static str) {
    (
        point("po", o.0, o.1, o.2) + &point("pd", o.0 + d.0, o.1 + d.1, o.2 + d.2),
        "plane(at: po.p, normal: axis(po.p, pd.p))",
    )
}

#[test]
fn random_blocks_on_random_planes_project_to_where_the_arithmetic_says() {
    let mut r = Rng(7);
    for _ in 0..200 {
        let (x, y, w, d, h) = (
            r.range(-50.0, 50.0),
            r.range(-50.0, 50.0),
            r.range(1.0, 60.0),
            r.range(1.0, 60.0),
            r.range(1.0, 30.0),
        );
        let (dx, dy, dz) = (r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(0.2, 1.0));
        let (ox, oy, oz) = (r.range(-40.0, 40.0), r.range(-40.0, 40.0), r.range(40.0, 90.0));
        let (pts, plane) = tilted((ox, oy, oz), (dx, dy, dz));
        let s = last_sketch(&format!(
            "{PLANES}sketch s(on: xy) {{\n  r = rect(corner: ({x:?}, {y:?}), size: ({w:?}, {d:?}))\n}}\nbase = extrude(s, {h:?})\ne = {FRONT_TOP}\n{pts}sketch k(on: {plane}) {{\n  j = project(e)\n}}\n"
        ));
        let pl = s.plane;
        let want = |q: forge_math::Point3| {
            let v = q - pl.origin;
            (v.dot(pl.x().get()), v.dot(pl.y().get()))
        };
        let (wa, wb) = (
            want(forge_math::Point3::new(x, y, h)),
            want(forge_math::Point3::new(x + w, y, h)),
        );
        let (a, b) = (at(&s, "j.a"), at(&s, "j.b"));
        let d0 = (a.x - wa.0).abs() + (a.y - wa.1).abs() + (b.x - wb.0).abs() + (b.y - wb.1).abs();
        let d1 = (a.x - wb.0).abs() + (a.y - wb.1).abs() + (b.x - wa.0).abs() + (b.y - wa.1).abs();
        assert!(d0.min(d1) < 1e-11, "{d0} {d1}");
        // the projected ends lie on the plane under the 3D points: back up the normal
        for p in [a, b] {
            let w3 = pl.plane_point(p);
            let n = (w3 - forge_math::Point3::new(0.0, 0.0, 0.0)).dot(pl.z().get())
                - (pl.origin - forge_math::Point3::new(0.0, 0.0, 0.0)).dot(pl.z().get());
            assert!(n.abs() < 1e-10);
        }
    }
}

/// A disc of radius `rad` round (cx, cy), 10 thick, its top rim bound as `rim`.
fn disc(cx: f64, cy: f64, rad: f64) -> String {
    format!(
        "{PLANES}sketch s(on: xy) {{\n  c = circle(({cx:?}, {cy:?}), {rad:?})\n}}\nbase = extrude(s, 10)\nrim = edges(between: (base.end, base.side(s.c)))\n"
    )
}

#[test]
fn random_rims_stay_exact_circles_on_random_parallel_planes() {
    let mut r = Rng(11);
    for _ in 0..100 {
        let (cx, cy, rad) = (r.range(-30.0, 30.0), r.range(-30.0, 30.0), r.range(0.5, 10.0));
        let z = r.range(20.0, 60.0);
        let src = format!(
            "{}sketch k(on: offset(XY, {z:?})) {{\n  j = project(rim)\n}}\next = extrude(k, 2, new: true)\n",
            disc(cx, cy, rad)
        );
        let s = last_sketch(&src);
        assert_eq!((at(&s, "j").x, at(&s, "j").y), (cx, cy));
        let v = volume(&ok(&src));
        let want = core::f64::consts::PI * rad * rad * 12.0;
        assert!((v - want).abs() < 1e-8 * want, "{v} {want}");
    }
}

#[test]
fn random_rims_on_turned_parallel_planes_edge_on_planes_and_tilted_ones() {
    let mut r = Rng(13);
    for _ in 0..100 {
        let (cx, cy, rad) = (r.range(-30.0, 30.0), r.range(-30.0, 30.0), r.range(0.5, 10.0));
        let src = disc(cx, cy, rad);
        // turned in its own plane: a circle, the centre carried round
        let t = r.range(0.0, core::f64::consts::TAU);
        let (c, s) = (dmath::cos(t), dmath::sin(t));
        let q = last_sketch(&format!(
            "{src}{}sketch k(on: plane(at: xy.origin, normal: xy.normal, x: axis(xy.origin, t.p))) {{\n  j = project(rim)\n}}\n",
            point("t", c, s, 0.0)
        ));
        let e = at(&q, "j");
        assert!(matches!(entity(&q, "j"), Entity::Circle { .. }));
        assert!((e.x - (cx * c + cy * s)).abs() < 1e-11 && (e.y - (cy * c - cx * s)).abs() < 1e-11);
        // a plane holding z, at any turn: a line a diameter long through the centre, at height 10
        let q = last_sketch(&format!(
            "{src}{}sketch k(on: plane(at: xy.origin, normal: axis(xy.origin, t.p), x: xy.normal)) {{\n  j = project(rim)\n}}\n",
            point("t", c, s, 0.0)
        ));
        let (a, b) = (at(&q, "j.a"), at(&q, "j.b"));
        assert!((a.distance(b) - 2.0 * rad).abs() < 1e-11, "{a:?} {b:?}");
        assert!(
            (a.x - 10.0).abs() < 1e-11 && (b.x - 10.0).abs() < 1e-11,
            "{a:?} {b:?}"
        );
        // tilted: an exact ellipse, every point of the rim on it, and what it bounds extrudes to
        // the rim's area times the cosine
        let tilt = r.range(1.0, 89.0).to_radians();
        let (st, ct) = (dmath::sin(tilt), dmath::cos(tilt));
        let (pts, plane) = tilted((0.0, 0.0, 30.0), (st, 0.0, ct));
        let full = format!(
            "{src}{pts}sketch k(on: {plane}) {{\n  j = project(rim)\n}}\next = extrude(k, 2, new: true)\n"
        );
        let q = last_sketch(&full);
        let (c0, maj, b) = ellipse_of(&q, "j");
        close(c0.distance(maj), rad);
        assert!((b - rad * ct).abs() < 1e-11, "{b} {}", rad * ct);
        for k in 0..32 {
            let a = k as f64 * dmath::TAU / 32.0;
            let p = forge_math::Point3::new(cx + rad * dmath::cos(a), cy + rad * dmath::sin(a), 10.0);
            let l = q.plane.point_to_local(p);
            assert!(on_ellipse(&q, "j", Point2::new(l.x, l.y)) < 1e-10);
        }
        let v = volume(&ok(&full));
        let want = core::f64::consts::PI * rad * rad * 10.0 + core::f64::consts::PI * rad * rad * ct * 2.0;
        assert!((v - want).abs() < 1e-7 * want, "{v} {want}");
    }
}

#[test]
fn a_sketch_on_a_plane_above_carries_its_results_when_the_plane_moves() {
    let src = |h: f64| {
        format!(
            "{BLOCK}shared h = {h:?}\nwp = workplane(on: offset(s.plane, h))\ne = {FRONT_TOP}\nsketch k(on: wp) {{\n  j = project(e)\n}}\n"
        )
    };
    assert_eq!(last_sketch(&src(30.0)).plane.origin.z, 30.0);
    let q = last_sketch(&src(12.0));
    assert_eq!(q.plane.origin.z, 12.0);
    let a = q.plane.plane_point(at(&q, "j.a"));
    assert_eq!((a.y, a.z), (0.0, 12.0));
}

/// The block, its upright edge at (40, 30) rounded 3 by `round`.
const FILLETED: &str = "XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)\nXZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)\nsketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 30))\n}\nbase = extrude(s, 10)\nround = fillet(edges(between: (base.side(s.r[1]), base.side(s.r[2]))), 3)\n";

#[test]
fn a_fillet_face_projects_exactly_parallel_square_on_and_tilted() {
    let face = "face = faces(of: round)\n";
    // parallel: the quarter arc, its tangent edges end-on at its ends
    let src = format!("{FILLETED}{face}sketch k(on: offset(XY, 20)) {{\n  f = project(face)\n}}\n");
    no_freedom(&ok(&src));
    let s = last_sketch(&src);
    // the top and bottom arcs land on each other: one circle, the quarter round the corner
    let arcs: Vec<_> = s
        .names
        .iter()
        .filter_map(|n| match s.solved.entity(n.1) {
            Some(Entity::Arc { center, start, .. }) => Some((s.solved.point(center), s.solved.point(start))),
            _ => None,
        })
        .collect();
    assert!(!arcs.is_empty(), "{:?}", s.names);
    for (c, a) in &arcs {
        assert!(c.distance(Point2::new(37.0, 27.0)) < 1e-12, "{c:?}");
        close(c.distance(*a), 3.0);
    }
    // tilted: ellipse arcs, top and bottom apart; tangent lines as lines
    let src = format!(
        "{FILLETED}{face}{}sketch k(on: plane(at: xy.origin, normal: axis(xy.origin, q.p))) {{\n  f = project(face)\n}}\n",
        point("q", 0.0, 1.0, 1.0)
    );
    no_freedom(&ok(&src));
    let s = last_sketch(&src);
    let ell: Vec<_> = s
        .names
        .iter()
        .filter(|n| matches!(s.solved.entity(n.1), Some(Entity::Ellipse { ends: Some(_), .. })))
        .collect();
    assert_eq!(ell.len(), 2, "{:?}", s.names);
    for (n, _) in &ell {
        let (c, maj, b) = ellipse_of(&s, n);
        close(c.distance(maj), 3.0);
        close(b, 3.0 * 0.5f64.sqrt());
    }
}

#[test]
fn a_filleted_top_face_on_a_tilted_plane_extrudes_to_its_area_times_the_cosine() {
    let area = 40.0 * 30.0 - (9.0 - 9.0 * core::f64::consts::PI / 4.0);
    for (dir, cos) in [
        ((0.0, 1.0, 1.0), 0.5f64.sqrt()),
        ((1.0, 2.0, 3.0), 3.0 / 14f64.sqrt()),
        ((0.0, 0.0, 1.0), 1.0),
    ] {
        let (pts, plane) = tilted((0.0, 0.0, 40.0), dir);
        let r = ok(&format!(
            "{FILLETED}top = base.end\n{pts}sketch k(on: {plane}) {{\n  f = project(top)\n}}\next = extrude(k, 5, new: true)\n"
        ));
        let v = volume(&r) - 40.0 * 30.0 * 10.0 + (9.0 - 9.0 * core::f64::consts::PI / 4.0) * 10.0;
        assert!(
            (v - area * cos * 5.0).abs() < 1e-7 * v,
            "{dir:?}: {v} {}",
            area * cos * 5.0
        );
    }
}

/// The block, its top front edge rounded 2 at x = 0 to `to` at x = 40 by `round`.
fn variable(to: f64) -> String {
    format!(
        "{BLOCK}sketch c(on: base.end) {{\n  o = pt(0, 0)\n}}\nround = fillet({FRONT_TOP}, vary(2, {to:?}), from: c.o)\n"
    )
}

#[test]
fn a_blend_edge_is_refused_without_approx_and_fitted_within_it_with() {
    let e = error_of(&format!(
        "{}face = faces(of: round)\nsketch k(on: offset(XY, 20)) {{\n  f = project(face)\n}}\n",
        variable(4.0)
    ));
    assert!(e.contains("NeedsApprox"), "{e}");
    // the top's spring curve: where the ball meets the top, y = r = 2 + x / 20
    for tol in [0.01, 0.001, 0.0001] {
        let src = format!(
            "{}spring = edges(between: (round, base.end))\nsketch k(on: offset(XY, 20)) {{\n  j = project(spring, approx: {tol:?})\n}}\n",
            variable(4.0)
        );
        no_freedom(&ok(&src));
        let s = last_sketch(&src);
        let id = id(&s, "j");
        assert!(matches!(s.solved.entity(id), Some(Entity::Spline { .. })));
        let pts = s.solved.spline_samples(id, 4001);
        let worst = pts
            .iter()
            .map(|p| (p.y - (2.0 + p.x / 20.0)).abs())
            .fold(0.0, f64::max);
        assert!(worst <= tol, "{tol}: {worst}");
        assert!(pts.iter().all(|p| (-1e-9..=40.0 + 1e-9).contains(&p.x)));
    }
}

#[test]
fn a_blended_top_face_fitted_extrudes_within_its_tolerance() {
    for (to, strip) in [(4.0, 120.0), (6.0, 160.0)] {
        let base = variable(to);
        let v0 = volume(&ok(&base));
        let r = ok(&format!(
            "{base}top = base.end\nsketch k(on: offset(XY, 20)) {{\n  f = project(top, approx: 0.001)\n}}\next = extrude(k, 5, new: true)\n"
        ));
        // the top is 40 by 30 less the fillet's strip, 2 to `to` wide
        let v = volume(&r) - v0;
        let want = (40.0 * 30.0 - strip) * 5.0;
        assert!((v - want).abs() <= 5.0 * 0.001 * 140.0, "{to}: {v} {want}");
    }
}

#[test]
fn random_fillets_on_random_tilted_planes_project_exactly() {
    let mut r = Rng(29);
    for _ in 0..60 {
        let (w, d, h) = (r.range(20.0, 60.0), r.range(20.0, 60.0), r.range(5.0, 20.0));
        let rad = r.range(0.5, 4.5);
        let (nx, ny, nz) = (r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(0.3, 1.0));
        let (pts, plane) = tilted((1.0, 2.0, h + 10.0), (nx, ny, nz));
        let src = format!(
            "{PLANES}sketch s(on: xy) {{\n  r = rect(corner: (0, 0), size: ({w:?}, {d:?}))\n}}\nbase = extrude(s, {h:?})\nround = fillet(edges(between: (base.side(s.r[1]), base.side(s.r[2]))), {rad:?})\nface = faces(of: round)\n{pts}sketch k(on: {plane}) {{\n  f = project(face)\n}}\n"
        );
        no_freedom(&ok(&src));
        let s = last_sketch(&src);
        // every point of each rim arc lies on its ellipse, between its ends
        let (cx, cy) = (w - rad, d - rad);
        for (z, n) in [(0.0, 0usize), (h, 1)] {
            let names: Vec<&String> = s
                .names
                .iter()
                .filter(|q| matches!(s.solved.entity(q.1), Some(Entity::Ellipse { .. })))
                .map(|q| &q.0)
                .collect();
            let mut best = f64::MAX;
            for t in &names {
                let worst = (0..=16)
                    .map(|k| {
                        let a = k as f64 * dmath::FRAC_PI_2 / 16.0;
                        let p =
                            forge_math::Point3::new(cx + rad * dmath::cos(a), cy + rad * dmath::sin(a), z);
                        let l = s.plane.point_to_local(p);
                        on_ellipse(&s, t, Point2::new(l.x, l.y))
                    })
                    .fold(0.0, f64::max);
                best = best.min(worst);
            }
            assert!(best < 1e-9, "rim {n}: {best}");
        }
    }
}
