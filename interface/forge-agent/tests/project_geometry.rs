//! `project` rows: sketch geometry from the part above, through the file, rebuilds and edits.

use forge_agent::model::Model;
use forge_agent::tools::Part;
use forge_doc::sketch;
use forge_math::{Point2, dmath};
use forge_model::Feature;
use forge_sketch::Entity;

fn model(src: &str) -> Model {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}\n{src}", m.error);
    m
}

fn sketch_named<'a>(m: &'a Model, label: &str) -> &'a forge_model::Sketch {
    let i = m
        .lines
        .iter()
        .position(|l| l.label.as_deref() == Some(label))
        .unwrap();
    let Feature::Sketch(s) = &m.features[i] else {
        panic!("{label} is no sketch")
    };
    s
}

fn error_of(src: &str) -> String {
    Model::new(src).error.expect("an error").msg
}

fn volume(m: &mut Model) -> f64 {
    m.mass().unwrap().volume
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} against {b}");
}

fn at(s: &forge_model::Sketch, t: &str) -> Point2 {
    sketch::point(s, t).unwrap()
}

fn entity(s: &forge_model::Sketch, t: &str) -> Entity {
    let id = s.names.iter().find(|n| n.0 == t).unwrap().1;
    s.solved.entity(id).unwrap()
}

/// Handles of what a selector picks, on the part as it ends.
fn handles(m: &Model, sel: &str) -> Vec<String> {
    m.select_ref(sel)
        .unwrap()
        .iter()
        .map(|n| format!("@{:016x}", n.0))
        .collect()
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

const BLOCK: &str = "base: extrude 10 rect 0 0 40 30\n";

#[test]
fn a_straight_edge_gives_the_same_line_and_end_on_a_point() {
    let m0 = model(BLOCK);
    let top = handles(&m0, "(edges along +x top back)");
    let up = handles(&m0, "(edges along +z left back)");
    assert_eq!((top.len(), up.len()), (1, 1), "{top:?} {up:?}");
    let src = format!(
        "{BLOCK}sk: sketch xy at 0 0 50\n  project {} {}\nend\n",
        top[0], up[0]
    );
    let m = model(&src);
    let s = sketch_named(&m, "sk");
    assert!(matches!(entity(s, "j1"), Entity::Line { .. }));
    assert!(matches!(entity(s, "j2"), Entity::Point { .. }));
    let (a, b) = (at(s, "j1.a"), at(s, "j1.b"));
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    assert_eq!((x0, x1, a.y, b.y), (0.0, 40.0, 30.0, 30.0));
    assert_eq!((at(s, "j2").x, at(s, "j2").y), (0.0, 30.0));
}

#[test]
fn results_hold_no_freedom_and_a_new_line_closes_on_them_exactly() {
    let m0 = model(BLOCK);
    let e = handles(&m0, "(edges along +x top front)");
    let src = format!(
        "{BLOCK}sk: sketch xy at 0 0 7\n  project {}\n  line j1.b 20 25\n  line l1.b j1.a\nend\nextrude cut 3 sk\n",
        e[0]
    );
    let mut m = Model::new(&src);
    assert!(m.error.is_none(), "{:?}", m.error);
    let s = sketch_named(&m, "sk");
    // the third corner is the only free thing
    let (dof, free) = sketch::freedom(s);
    assert_eq!(dof, 2, "{free:?}");
    assert!(!free.contains(&"j1".to_string()), "{free:?}");
    assert_eq!(at(s, "j1.b"), at(s, "l1.a"), "the same point, not a near one");
    assert_eq!(at(s, "j1.a"), at(s, "l2.b"));
    // a triangle 40 base, 25 high, cut 3 from the top
    let v = volume(&mut m);
    close(v, 40.0 * 30.0 * 10.0 - 0.5 * 40.0 * 25.0 * 3.0);
}

#[test]
fn a_face_gives_its_boundary_and_extrudes_to_its_exact_volume() {
    let m0 = model(BLOCK);
    let top = handles(&m0, "(faces plane normal +z top)");
    let src = format!(
        "{BLOCK}sk: sketch xy at 0 0 30\n  project {}\nend\next: extrude new 5 sk\n",
        top[0]
    );
    let mut m = Model::new(&src);
    assert!(m.error.is_none(), "{:?}", m.error);
    close(volume(&mut m), 40.0 * 30.0 * 10.0 + 40.0 * 30.0 * 5.0);
    assert_eq!(sketch::freedom(sketch_named(&m, "sk")).0, 0);
}

#[test]
fn a_face_with_a_hole_and_an_arc_keeps_them_exact() {
    let src = "base: extrude 10 rrect 0 0 40 30 5\nhole 8 -xy at 20 15 10 thru\n";
    let m0 = model(src);
    let top = handles(&m0, "(faces plane normal +z top)");
    let full = format!(
        "{src}sk: sketch xy at 0 0 30\n  project {}\nend\next: extrude new 5 sk\n",
        top[0]
    );
    let mut m = Model::new(&full);
    assert!(m.error.is_none(), "{:?}", m.error);
    let area = 40.0 * 30.0 - (4.0 - core::f64::consts::PI) * 25.0 - core::f64::consts::PI * 16.0;
    close(volume(&mut m), area * 15.0);
    // the profile's arcs: tangent to its lines at shared points, none free
    assert_eq!(sketch::freedom(sketch_named(&m, "sk")).0, 0);
}

#[test]
fn circles_are_exact_square_on_lines_edge_on_and_ellipses_tilted() {
    let src = "base: extrude 10 rect 0 0 40 30\nhole 8 -xy at 20 15 10 thru\n";
    let m0 = model(src);
    let rim = handles(&m0, "(edges circle top)");
    assert_eq!(rim.len(), 1);
    // parallel: the circle
    let m = model(&format!(
        "{src}sk: sketch xy at 0 0 50\n  project {}\nend\n",
        rim[0]
    ));
    let s = sketch_named(&m, "sk");
    assert!(matches!(entity(s, "j1"), Entity::Circle { .. }));
    assert_eq!((at(s, "j1").x, at(s, "j1").y), (20.0, 15.0));
    assert_eq!(
        s.solved.radius(s.names.iter().find(|n| n.0 == "j1").unwrap().1),
        4.0
    );
    // upside down too
    let m = model(&format!(
        "{src}sk: sketch -xy at 0 0 50\n  project {}\nend\n",
        rim[0]
    ));
    let s = sketch_named(&m, "sk");
    assert_eq!((at(s, "j1").x, at(s, "j1").y), (20.0, -15.0));
    // square on: a line its diameter long
    let m = model(&format!("{src}sk: sketch xz\n  project {}\nend\n", rim[0]));
    let s = sketch_named(&m, "sk");
    assert!(matches!(entity(s, "j1"), Entity::Line { .. }));
    let (a, b) = (at(s, "j1.a"), at(s, "j1.b"));
    close((a.x - b.x).abs(), 8.0);
    close(a.x.min(b.x), 16.0);
    close(a.y, 10.0);
    close(b.y, 10.0);
    // tilted 45 degrees: the ellipse, major 4 along x, minor 4 cos 45
    let m = model(&format!("{src}sk: sketch z 0 1 1\n  project {}\nend\n", rim[0]));
    let s = sketch_named(&m, "sk");
    let (c, maj, b) = ellipse_of(s, "j1");
    let want = s.plane.point_to_local(forge_math::Point3::new(20.0, 15.0, 10.0));
    assert!(c.distance(Point2::new(want.x, want.y)) < 1e-12);
    close(c.distance(maj), 4.0);
    close(b, 4.0 * 0.5f64.sqrt());
}

#[test]
fn a_part_circle_square_on_reaches_past_its_ends_where_it_turns() {
    // the corner arcs of a rounded block seen from the side
    let src = "base: extrude 10 rrect 0 0 40 30 5\n";
    let m0 = model(src);
    let arc = handles(&m0, "(edges circle top near 40 30 10)");
    assert_eq!(arc.len(), 1);
    let m = model(&format!("{src}sk: sketch xz\n  project {}\nend\n", arc[0]));
    let s = sketch_named(&m, "sk");
    let (a, b) = (at(s, "j1.a"), at(s, "j1.b"));
    close((a.x - b.x).abs(), 5.0);
    close(a.x.max(b.x), 40.0);
}

#[test]
fn an_arc_keeps_its_way_round_from_either_side() {
    let src = "base: extrude 10 rrect 0 0 40 30 5\n";
    let m0 = model(src);
    let arc = handles(&m0, "(edges circle top near 40 30 10)");
    for (frame, ccw) in [("xy", true), ("-xy", false)] {
        let m = model(&format!(
            "{src}sk: sketch {frame} at 0 0 50\n  project {}\nend\n",
            arc[0]
        ));
        let s = sketch_named(&m, "sk");
        let Entity::Arc { ccw: way, .. } = entity(s, "j1") else {
            panic!("an arc")
        };
        assert_eq!(way, ccw, "{frame}");
    }
}

#[test]
fn it_follows_an_edit_above_and_says_so_when_the_source_goes() {
    let mut p = Part::new("w = 40\nbase: extrude 10 rect 0 0 w 30\n");
    let edge = handles(&p.model, "(edges along +y top right)");
    assert!(
        p.act(
            &format!("sk: sketch xy at 0 0 30\n project {}\nend", edge[0]),
            None
        )
        .ok
    );
    let s = sketch_named(&p.model, "sk");
    assert_eq!(at(s, "j1.a").x, 40.0);
    let done = p.act("w = 60", None);
    assert!(done.ok, "{}", done.text);
    let s = sketch_named(&p.model, "sk");
    assert_eq!(
        (at(s, "j1.a").x, at(s, "j1.b").x),
        (60.0, 60.0),
        "re-derived from the edit"
    );
    // a chamfer takes the edge: the edit is refused, naming the row
    let cut = p.act(&format!("before sk chamfer 1 {}", edge[0]), None);
    assert!(!cut.ok, "{}", cut.text);
    assert!(
        cut.text.contains("project") && cut.text.contains(&edge[0]),
        "{}",
        cut.text
    );
    // the same by hand in the file: an error at the row's line, and nothing frozen below it
    let hand = format!(
        "w = 60\nbase: extrude 10 rect 0 0 w 30\nchamfer 1 {0}\nsk: sketch xy at 0 0 30\n project {0}\nend\n",
        edge[0]
    );
    let m = Model::new(&hand);
    let f = m.error.as_ref().expect("an error");
    assert_eq!(m.lines[f.at].label.as_deref(), Some("sk"));
    assert!(f.msg.contains("project"), "{}", f.msg);
    assert!(m.features.iter().all(|f| !matches!(f, Feature::Sketch(_))));
}

#[test]
fn a_face_follows_a_new_edge_and_its_extrusions_faces_keep_their_names() {
    let src = "base: id 1 extrude 10 rect 0 0 40 30\n";
    let m0 = model(src);
    let top = handles(&m0, "(faces plane normal +z top)");
    let tail = format!(
        "sk: id 5 sketch xy at 0 0 30\n  project {}\nend\next: id 6 extrude new 5 sk\n",
        top[0]
    );
    let mut m = Model::new(&format!("{src}{tail}"));
    assert!(m.error.is_none(), "{:?}", m.error);
    let faces = |m: &Model| -> Vec<u64> {
        let b = m.solids_after(m.built()).last().unwrap().body.clone();
        b.face_ids().map(|f| b.face(f).name.0).collect()
    };
    let before = faces(&m);
    let corner = "id 2 chamfer 2 (edges along +z right back)\n";
    let mut after = Model::new(&format!("{src}{corner}{tail}"));
    assert!(after.error.is_none(), "{:?}", after.error);
    // one more side, and each side there was still has its name
    let now = faces(&after);
    assert_eq!(now.len(), before.len() + 1);
    let kept = before.iter().filter(|n| now.contains(n)).count();
    assert!(kept >= 4, "{kept} of {}", before.len());
    close(volume(&mut after) - volume(&mut m), -2.0 * (10.0 + 5.0));
}

#[test]
fn rows_cannot_name_what_can_change_in_number_but_can_name_edges_listed() {
    let src = "base: extrude 10 rect 0 0 40 30\n";
    let m0 = model(src);
    let top = handles(&m0, "(faces plane normal +z top)");
    let e = error_of(&format!(
        "{src}sk: sketch xy at 0 0 30\n  project {}\n  dist j1 40\nend\n",
        top[0]
    ));
    assert!(e.contains("list the edges"), "{e}");
    let edges = handles(&m0, "(edges of base line top)");
    let m = model(&format!(
        "{src}sk: sketch xy at 0 0 30\n  project {}\n  pt 5 5\n  dist j1.a p1 4\nend\nextrude new 5 sk\n",
        edges.join(" ")
    ));
    assert_eq!(
        sketch_named(&m, "sk")
            .names
            .iter()
            .filter(|n| n.0.starts_with('j'))
            .count(),
        4
    );
}

#[test]
fn work_features_and_sketch_points_project() {
    let src = "base: extrude 10 rect 0 0 40 30\npt1: point mid base (edges along +x top back)\nax: axis pts base (verts near 0 0 0) base (verts near 0 0 10)\n";
    let m = model(&format!("{src}sk: sketch xy at 0 0 50\n  project pt1 ax\nend\n"));
    let s = sketch_named(&m, "sk");
    assert_eq!((at(s, "j1").x, at(s, "j1").y), (20.0, 30.0));
    assert_eq!((at(s, "j2").x, at(s, "j2").y), (0.0, 0.0));
    let e = error_of(&format!("{src}sk: sketch xz\n  project ax\nend\n"));
    assert!(e.contains("no ends"), "{e}");
}

#[test]
fn construction_and_constraints_work_on_results() {
    let m0 = model(BLOCK);
    let e = handles(&m0, "(edges along +x top front)");
    let src = format!(
        "{BLOCK}sk: sketch xy at 0 0 10\n  project {} cons\n  pt 5 5\n  dist p1 j1 7\n  dist j1.a p1 9\nend\n",
        e[0]
    );
    let m = model(&src);
    let s = sketch_named(&m, "sk");
    let id = s.names.iter().find(|n| n.0 == "j1").unwrap().1;
    assert!(s.solved.is_construction(id));
    assert_eq!(sketch::freedom(s).0, 0);
    close(at(s, "p1").y, 7.0);
    // a result is held: a dimension against what it's held to is refused
    let bad = format!(
        "{BLOCK}sk: sketch xy at 0 0 10\n  project {}\n  dist j1 41\nend\n",
        e[0]
    );
    assert!(error_of(&bad).contains("does not solve"), "{}", error_of(&bad));
}

#[test]
fn the_file_reads_back_the_same_and_a_dimension_elsewhere_keeps_it() {
    let m0 = model(BLOCK);
    let e = handles(&m0, "(edges along +x top front)");
    let src = format!(
        "{BLOCK}sk: sketch xy at 0 0 10\n  project {}\n  pt 5 5\n  dist j1.a p1 9\nend\n",
        e[0]
    );
    let mut p = Part::new(&src);
    assert!(p.model.error.is_none());
    let printed = p.model.src().to_string();
    let again = Model::new(&printed);
    assert!(again.error.is_none(), "{:?}", again.error);
    assert_eq!(sketch_named(&again, "sk"), sketch_named(&p.model, "sk"));
    assert!(p.act("dim sk dist j1.a p1 8", None).ok);
    let s = sketch_named(&p.model, "sk");
    close(at(s, "j1.a").distance(at(s, "p1")), 8.0);
    assert_eq!(at(s, "j1.b").x.max(at(s, "j1.a").x), 40.0);
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
        let src = format!("base: extrude {h:?} rect {x:?} {y:?} {w:?} {d:?}\n");
        let m0 = model(&src);
        let e = handles(&m0, "(edges along +x top front)");
        let head = format!("z {dx:?} {dy:?} {dz:?} at {ox:?} {oy:?} {oz:?}");
        let m = model(&format!("{src}sk: sketch {head}\n  project {}\nend\n", e[0]));
        let s = sketch_named(&m, "sk");
        let pl = s.plane;
        let want = |q: forge_math::Point3| {
            let v = q - pl.origin;
            (v.dot(pl.x().get()), v.dot(pl.y().get()))
        };
        let (wa, wb) = (
            want(forge_math::Point3::new(x, y, h)),
            want(forge_math::Point3::new(x + w, y, h)),
        );
        let (a, b) = (at(s, "j1.a"), at(s, "j1.b"));
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

#[test]
fn random_rims_stay_exact_circles_on_random_parallel_planes() {
    let mut r = Rng(11);
    for _ in 0..100 {
        let (cx, cy, rad) = (r.range(-30.0, 30.0), r.range(-30.0, 30.0), r.range(0.5, 10.0));
        let z = r.range(20.0, 60.0);
        let src = format!("base: extrude 10 circle {cx:?} {cy:?} {rad:?}\n");
        let m0 = model(&src);
        let rim = handles(&m0, "(edges circle top)");
        let mut m = Model::new(&format!(
            "{src}sk: sketch xy at 0 0 {z:?}\n  project {}\nend\nextrude new 2 sk\n",
            rim[0]
        ));
        assert!(m.error.is_none(), "{:?}", m.error);
        let s = sketch_named(&m, "sk");
        assert_eq!((at(s, "j1").x, at(s, "j1").y), (cx, cy));
        let v = volume(&mut m);
        let want = core::f64::consts::PI * rad * rad * 12.0;
        assert!((v - want).abs() < 1e-8 * want, "{v} {want}");
    }
}

#[test]
fn random_rims_on_turned_parallel_planes_edge_on_planes_and_tilted_ones() {
    let mut r = Rng(13);
    for _ in 0..100 {
        let (cx, cy, rad) = (r.range(-30.0, 30.0), r.range(-30.0, 30.0), r.range(0.5, 10.0));
        let src = format!("base: extrude 10 circle {cx:?} {cy:?} {rad:?}\n");
        let m0 = model(&src);
        let rim = handles(&m0, "(edges circle top)");
        // turned in its own plane: a circle, the centre carried round
        let t = r.range(0.0, core::f64::consts::TAU);
        let (c, s) = (dmath::cos(t), dmath::sin(t));
        let m = model(&format!(
            "{src}sk: sketch z 0 0 1 x {c:?} {s:?} 0\n  project {}\nend\n",
            rim[0]
        ));
        let q = sketch_named(&m, "sk");
        let e = at(q, "j1");
        assert!(matches!(entity(q, "j1"), Entity::Circle { .. }));
        assert!((e.x - (cx * c + cy * s)).abs() < 1e-11 && (e.y - (cy * c - cx * s)).abs() < 1e-11);
        // a plane holding z, at any turn: a line a diameter long through the centre, at height 10
        let m = model(&format!(
            "{src}sk: sketch z {c:?} {s:?} 0 x 0 0 1\n  project {}\nend\n",
            rim[0]
        ));
        let q = sketch_named(&m, "sk");
        let (a, b) = (at(q, "j1.a"), at(q, "j1.b"));
        assert!((a.distance(b) - 2.0 * rad).abs() < 1e-11, "{a:?} {b:?}");
        assert!(
            (a.x - 10.0).abs() < 1e-11 && (b.x - 10.0).abs() < 1e-11,
            "{a:?} {b:?}"
        );
        // tilted: an exact ellipse, every point of the rim on it, and what it bounds extrudes to
        // the rim's area times the cosine
        let tilt = r.range(1.0, 89.0).to_radians();
        let (st, ct) = (dmath::sin(tilt), dmath::cos(tilt));
        let mut m = Model::new(&format!(
            "{src}sk: sketch z {st:?} 0 {ct:?} at 0 0 30\n  project {}\nend\nextrude new 2 sk\n",
            rim[0]
        ));
        assert!(m.error.is_none(), "{:?}", m.error);
        let q = sketch_named(&m, "sk");
        let (c0, maj, b) = ellipse_of(q, "j1");
        close(c0.distance(maj), rad);
        assert!((b - rad * ct).abs() < 1e-11, "{b} {}", rad * ct);
        for k in 0..32 {
            let a = k as f64 * dmath::TAU / 32.0;
            let p = forge_math::Point3::new(cx + rad * dmath::cos(a), cy + rad * dmath::sin(a), 10.0);
            let l = q.plane.point_to_local(p);
            assert!(on_ellipse(q, "j1", Point2::new(l.x, l.y)) < 1e-10);
        }
        let v = volume(&mut m);
        let want = core::f64::consts::PI * rad * rad * 10.0 + core::f64::consts::PI * rad * rad * ct * 2.0;
        assert!((v - want).abs() < 1e-7 * want, "{v} {want}");
    }
}

#[test]
fn a_sketch_on_a_plane_above_carries_its_results_when_the_plane_moves() {
    let src = "h = 30\nbase: extrude 10 rect 0 0 40 30\nwp: plane xy at 0 0 h\n";
    let m0 = model(src);
    let e = handles(&m0, "(edges along +x top front)");
    let mut p = Part::new(&format!("{src}sk: sketch on wp\n  project {}\nend\n", e[0]));
    assert!(p.model.error.is_none(), "{:?}", p.model.error);
    let before = sketch_named(&p.model, "sk").plane.origin.z;
    assert_eq!(before, 30.0);
    assert!(p.act("h = 12", None).ok);
    let q = sketch_named(&p.model, "sk");
    assert_eq!(q.plane.origin.z, 12.0);
    let a = q.plane.plane_point(at(q, "j1.a"));
    assert_eq!((a.y, a.z), (0.0, 12.0));
}

const FILLETED: &str = "base: extrude 10 rect 0 0 40 30\nfillet 3 (edges along +z right back)\n";

#[test]
fn a_fillet_face_projects_exactly_parallel_square_on_and_tilted() {
    let m0 = model(FILLETED);
    let face = handles(&m0, "(faces cyl)");
    assert_eq!(face.len(), 1);
    // parallel: the quarter arc, its tangent edges end-on at its ends
    let m = model(&format!(
        "{FILLETED}sk: sketch xy at 0 0 20\n  project {}\nend\n",
        face[0]
    ));
    let s = sketch_named(&m, "sk");
    assert_eq!(sketch::freedom(s).0, 0);
    let mut arcs: Vec<_> = s
        .names
        .iter()
        .filter(|n| matches!(s.solved.entity(n.1), Some(Entity::Arc { .. })))
        .map(|n| n.1)
        .collect();
    arcs.dedup();
    assert_eq!(arcs.len(), 1, "the top and bottom arcs are one: {:?}", s.names);
    // tilted: ellipse arcs, top and bottom apart; tangent lines as lines
    let m = model(&format!(
        "{FILLETED}sk: sketch z 0 1 1\n  project {}\nend\n",
        face[0]
    ));
    let s = sketch_named(&m, "sk");
    assert_eq!(sketch::freedom(s).0, 0);
    let ell: Vec<_> = s
        .names
        .iter()
        .filter(|n| matches!(s.solved.entity(n.1), Some(Entity::Ellipse { ends: Some(_), .. })))
        .collect();
    assert_eq!(ell.len(), 2, "{:?}", s.names);
    for (n, _) in &ell {
        let (c, maj, b) = ellipse_of(s, n);
        close(c.distance(maj), 3.0);
        close(b, 3.0 * 0.5f64.sqrt());
    }
}

#[test]
fn a_filleted_top_face_on_a_tilted_plane_extrudes_to_its_area_times_the_cosine() {
    let m0 = model(FILLETED);
    let top = handles(&m0, "(faces plane normal +z top)");
    let area = 40.0 * 30.0 - (9.0 - 9.0 * core::f64::consts::PI / 4.0);
    for (dir, cos) in [
        ("z 0 1 1", 0.5f64.sqrt()),
        ("z 1 2 3", 3.0 / 14f64.sqrt()),
        ("xy", 1.0),
    ] {
        let mut m = Model::new(&format!(
            "{FILLETED}sk: sketch {dir} at 0 0 40\n  project {}\nend\next: extrude new 5 sk\n",
            top[0]
        ));
        assert!(m.error.is_none(), "{dir}: {:?}", m.error);
        let v = volume(&mut m) - 40.0 * 30.0 * 10.0 + (9.0 - 9.0 * core::f64::consts::PI / 4.0) * 10.0;
        assert!(
            (v - area * cos * 5.0).abs() < 1e-7 * v,
            "{dir}: {v} {}",
            area * cos * 5.0
        );
    }
}

const VARIABLE: &str = "base: extrude 10 rect 0 0 40 30\nfillet 2..4 (edges along +x top front)\n";

#[test]
fn a_blend_edge_is_refused_without_approx_and_fitted_within_it_with() {
    let m0 = model(VARIABLE);
    let face = handles(&m0, "(faces blend)");
    assert_eq!(face.len(), 1);
    let e = error_of(&format!(
        "{VARIABLE}sk: sketch xy at 0 0 20\n  project {}\nend\n",
        face[0]
    ));
    assert!(e.contains("approx TOL"), "{e}");
    // the top's spring curve: where the ball meets the top, y = r = 2 + x / 20
    let spring = handles(&m0, "(edges curve top)");
    assert_eq!(spring.len(), 1);
    for tol in [0.01, 0.001, 0.0001] {
        let m = model(&format!(
            "{VARIABLE}sk: sketch xy at 0 0 20\n  project {} approx {tol}\nend\n",
            spring[0]
        ));
        let s = sketch_named(&m, "sk");
        assert_eq!(sketch::freedom(s).0, 0);
        let id = s.names.iter().find(|n| n.0 == "j1").unwrap().1;
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
fn a_blended_top_face_fitted_extrudes_within_its_tolerance_and_its_rows_say_approx() {
    let m0 = model(VARIABLE);
    let top = handles(&m0, "(faces plane normal +z top)");
    let mut base = model(VARIABLE);
    let v0 = volume(&mut base);
    let mut p = Part::new(VARIABLE);
    let done = p.act(
        &format!("sk: sketch xy at 0 0 20\n project {} approx 0.001\nend", top[0]),
        None,
    );
    assert!(done.ok, "{}", done.text);
    assert!(p.model.src().contains("approx 0.001"));
    let done = p.act("extrude new 5 sk", None);
    assert!(done.ok, "{}", done.text);
    // the top is 40 by 30 less the fillet's strip, 2 to 4 wide: 40 * 3 mm2
    let v = p.model.mass().unwrap().volume - v0;
    let want = (40.0 * 30.0 - 120.0) * 5.0;
    assert!((v - want).abs() <= 5.0 * 0.001 * 140.0, "{v} {want}");
    // a rebuild after the radius changes follows it
    let done = p.act("edit fillet1 fillet 2..6 (edges along +x top front)", None);
    assert!(done.ok, "{}", done.text);
    let v = p.model.mass().unwrap().volume
        - volume(&mut model(
            "base: extrude 10 rect 0 0 40 30\nfillet 2..6 (edges along +x top front)\n",
        ));
    let want = (40.0 * 30.0 - 160.0) * 5.0;
    assert!((v - want).abs() <= 5.0 * 0.001 * 140.0, "{v} {want}");
}

#[test]
fn random_fillets_on_random_tilted_planes_project_exactly() {
    let mut r = Rng(29);
    for _ in 0..60 {
        let (w, d, h) = (r.range(20.0, 60.0), r.range(20.0, 60.0), r.range(5.0, 20.0));
        let rad = r.range(0.5, 4.5);
        let src =
            format!("base: extrude {h:?} rect 0 0 {w:?} {d:?}\nfillet {rad:?} (edges along +z right back)\n");
        let m0 = model(&src);
        let face = handles(&m0, "(faces cyl)");
        let (nx, ny, nz) = (r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(0.3, 1.0));
        let m = model(&format!(
            "{src}sk: sketch z {nx:?} {ny:?} {nz:?} at 1 2 {:?}\n  project {}\nend\n",
            h + 10.0,
            face[0]
        ));
        let s = sketch_named(&m, "sk");
        assert_eq!(sketch::freedom(s).0, 0);
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
                        on_ellipse(s, t, Point2::new(l.x, l.y))
                    })
                    .fold(0.0, f64::max);
                best = best.min(worst);
            }
            assert!(best < 1e-9, "rim {n}: {best}");
        }
    }
}
