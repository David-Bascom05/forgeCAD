//! Parameters and sketches, built in a session.

use crate::model::Model;
use crate::select;
use forge_doc::text;
use forge_model::{At, Feature, Head, Layout, Part, Thread};

fn built(src: &str) -> Model {
    let m = Model::new(src);
    if let Some(f) = &m.error {
        panic!("line {}: {} ({})\n{src}", m.lines[f.at].no, f.msg, f.hint);
    }
    m
}

/// The finished part, every value and name (caches left out).
fn body(src: &str) -> String {
    format!("{:?}", built(src).body().expect("a part").compact())
}

/// The first error's message and hint.
fn error(src: &str) -> (usize, String, String) {
    let m = Model::new(src);
    let f = m.error.expect("fails");
    (m.lines[f.at].no, f.msg, f.hint)
}

/// The mean centre of the faces a selector finds (a hole's two half walls, say), rounded.
fn at(m: &Model, sel: &str) -> [f64; 3] {
    let (b, ents) = m.select(sel, None).expect("selects");
    let n = ents.len() as f64;
    assert!(n > 0.0, "{sel}");
    let c = ents
        .iter()
        .map(|&e| select::centre(b, e))
        .fold([0.0; 3], |s, c| [s[0] + c.x, s[1] + c.y, s[2] + c.z]);
    c.map(|v| (v / n * 1e6).round() / 1e6 + 0.0)
}

#[test]
fn parameters_are_values_typed_in() {
    let with = "w = 40\nh = w/2 + 5 mm\nn = 2 + 1\na = 90/n deg\nr = min(w, h)/10\n\
                extrude w/4 rect -w/2 -h/2 w h\n\
                hole r*2 -xy at 0 0 (w/4) thru\n\
                pattern hole1 circ 0 0 0 0 0 1 n a*n*2\n\
                fillet r/2..r (edges along +z of extrude1)\n";
    // the same lines, so the same feature ids and names
    let typed = "w = 40\nh = w/2 + 5 mm\nn = 2 + 1\na = 90/n deg\nr = min(w, h)/10\n\
                 extrude 10 rect -20 -12.5 40 25\nhole 5 -xy at 0 0 10 thru\n\
                 pattern hole1 circ 0 0 0 0 0 1 3 180\nfillet 1.25..2.5 (edges along +z of extrude1)\n";
    assert_eq!(body(with), body(typed));
}

#[test]
fn parameter_errors_say_what() {
    let e = |src: &str| error(src).1;
    assert_eq!(
        e("w = 3\nextrude wal rect 0 0 1 1\n"),
        "expected a number or a parameter, found 'wal'"
    );
    assert_eq!(e("w = w + 1\n"), "'w' refers to itself");
    assert_eq!(e("w = 1\nw = 2\n"), "parameter 'w' is already defined above");
    assert_eq!(
        e("rect = 1\n"),
        "'rect' is a word the file uses; pick another name"
    );
    assert_eq!(e("l2 = 1\n"), "'l2' is a word the file uses; pick another name");
    assert_eq!(
        e("a = 30 deg\nextrude a rect 0 0 1 1\n"),
        "'a' is in deg; a length is wanted"
    );
    assert_eq!(
        e("w = 3 mm\nrevolve w rect 1 0 1 1\n"),
        "'w' is in mm; an angle is wanted"
    );
    assert_eq!(e("a = 30 deg\nb = a + 2 mm\n"), "'a + 2' is in deg");
    assert_eq!(e("w = 3 mm\nb = w + 2 deg\n"), "'w + 2' is in mm");
    assert_eq!(e("a = 30 deg\nw = 2 mm\nb = a*w\n"), "mixes mm and deg");
    let (line, msg, hint) = error("a = b + 1\nb = 2\n");
    assert_eq!((line, msg.as_str()), (1, "unknown parameter 'b'"));
    assert_eq!(
        hint,
        "b is defined on line 2, below; define parameters above their use"
    );
    let (_, msg, _) = error(
        "n = 2.5\nextrude 1 rect 0 0 1 1\nhole 0.2 -xy at 0.5 0.5 1 thru\npattern hole1 lin 1 0 0 n 0.1\n",
    );
    assert_eq!(msg, "expected a count, found 'n' (2.5)");
}

#[test]
fn changing_a_parameter_rebuilds_from_its_first_use() {
    let src = |w: f64| {
        format!(
            "w = {w}\nd = 2\nextrude 10 rect 0 0 30 30\nhole 3 -xy at 5 5 10 thru\n\
             hole d*2 -xy at 15 15 10 thru\nhole w -xy at 25 25 10 thru\n"
        )
    };
    let mut m = built(&src(4.0));
    m.update(&src(5.0));
    assert_eq!(m.rebuilt, 1, "only the last hole uses w");
    assert_eq!(
        format!("{:?}", m.body().expect("built").compact()),
        body(&src(5.0))
    );
    m.update(&src(5.0).replace("d = 2", "d = 2.5"));
    assert_eq!(m.rebuilt, 2);
    m.update(&src(5.0).replace("d = 2", "d = 4/2"));
    assert_eq!(m.rebuilt, 2, "same values: nothing rebuilt past the hole using d");
    // the same, line for line
    let m2 = built(&src(5.0).replace("d = 2", "d = 4/2"));
    assert_eq!(
        m.body().expect("built").compact(),
        m2.body().expect("built").compact()
    );
}

const PLATE: &str = "w = 40\nh = 25\n\
    sk: sketch\n  rect 0 0 38 27\n  circle 20 12 4\n  fix l1.a\n  dist l1 w\n  dist l2 h\n\
      dia c1 8\n  hdist l1.a c1 (w/2)\n  vdist l1.a c1 h/2\nend\n\
    base: extrude 6 sk l1\n";

#[test]
fn a_sketch_drives_an_extrude() {
    let m = built(PLATE);
    let b = m.body().expect("built");
    let bb = b
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |x, f| x.hull(&b.face_box(f)));
    assert_eq!(
        [bb.x.lo, bb.y.lo, bb.x.hi, bb.y.hi].map(|v| v.round()),
        [0.0, 0.0, 40.0, 25.0]
    );
    assert_eq!(at(&m, "faces cyl"), [20.0, 12.5, 3.0]);
    // as much as the same plate typed in
    let typed = "extrude 6 rect 0 0 40 25 circle 20 12.5 4 cw\n";
    let (mut a, mut b) = (built(PLATE), built(typed));
    assert!((a.stats().0 - b.stats().0).abs() < 1e-6);
}

/// Fillets named by handle survive a dimension change and a new circle in the sketch.
#[test]
fn edges_from_a_sketch_keep_their_names() {
    let m = built(PLATE);
    let (b, ents) = m.select("edges convex along +z", None).expect("selects");
    let names: Vec<String> = ents
        .iter()
        .map(|&e| format!("@{:016x}", select::name(b, e).0))
        .collect();
    assert_eq!(names.len(), 4);
    let src = format!("{PLATE}fillet 2 {}\n", names.join(" "));
    let before = built(&src);
    let after = built(&src.replace("w = 40", "w = 55").replace("h = 25", "h = 30"));
    let more = built(&src.replace(
        "  circle 20 12 4\n",
        "  circle 20 12 4\n  circle 5 5 1\n  rad c2 1\n",
    ));
    for m in [&before, &after, &more] {
        assert_eq!(m.select("faces cyl r=2", None).expect("selects").1.len(), 4);
    }
    assert_eq!(at(&after, "faces cyl r=4"), [27.5, 15.0, 3.0]);
}

#[test]
fn sketches_on_faces_and_holes_at_sketch_points() {
    let src = format!(
        "{PLATE}top: sketch on (faces top of base)\n  pt 30 5\n  circle 30 20 3\n  rad c1 2\nend\n\
         hole 3 -on top p1 thru\nhole 2 -on top c1 4\nextrude to top rect 41 0 5 5\n"
    );
    let m = built(&src);
    assert_eq!(at(&m, "faces cyl r=1.5"), [30.0, 5.0, 3.0]);
    assert_eq!(at(&m, "faces cyl r=1"), [30.0, 20.0, 4.0]);
    // the extrude up to the sketch's plane is as tall as the plate
    let b = m.body().expect("built");
    let bb = b
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |x, f| x.hull(&b.face_box(f)));
    assert_eq!((bb.x.hi.round(), bb.z.hi.round()), (46.0, 6.0));
    let (_, msg, _) = error(&src.replace("(faces top of base)", "(faces cyl)"));
    assert_eq!(msg, "(faces cyl) is not a planar face");
}

/// A sketch on a work plane follows it, and the file keeps saying where it is.
#[test]
fn a_sketch_follows_its_plane() {
    let src = |z: f64| {
        format!(
            "h = {z}\nextrude 20 rect 0 0 30 30\nplane at 0 0 h\nsk: sketch -on plane1 at 0 0 h\n  pt 10 -10\n\
             end\nsketch on sk p1\n  circle 0 0 3\nend\nhole 6 on sketch2 c1 4\n"
        )
    };
    let mut m = built(&src(12.0));
    m.update(&src(20.0));
    assert_eq!(at(&m, "faces cyl r=3"), [10.0, 10.0, 18.0]);
    let printed = crate::text::print(&crate::text::parse(&src(20.0)).expect("parses"));
    assert!(
        printed.contains("\nsketch -on plane1 at 0 0 20\n  pt 10 -10\nend\nsketch on sketch1 p1\n"),
        "{printed}"
    );
}

#[test]
fn a_revolve_about_a_sketch_line() {
    let src = "sketch xz\n  line 0 -5 0 25 cons\n  rect 10 0 5 20\n  fix l2.a\n  dist l2 5\n  dist l3 20\n\
               end\nrevolve sketch1 axis l1\n";
    let typed = "revolve xz rect 10 0 5 20\n";
    let (mut a, mut b) = (built(src), built(typed));
    assert!((a.stats().0 - b.stats().0).abs() < 1e-6);
    assert_eq!(a.body().map(|b| b.faces.len()), b.body().map(|b| b.faces.len()));
}

/// As much solid as the same shapes typed in.
fn same_volume(src: &str, typed: &str) {
    let (mut a, mut b) = (built(src), built(typed));
    let (va, vb) = (a.stats().0, b.stats().0);
    assert!(va > 0.0 && (va - vb).abs() < 1e-6 * vb, "{va} {vb}");
}

#[test]
fn sweeps_lofts_and_coils_from_sketches() {
    same_volume(
        "sketch xz\n  line 0 0 0 30\n  arc 10 30 l1.b 10 40 cw\n  line a1.b 25 40\nend\n\
         sweep circle 0 0 2 path sketch1\n",
        "sweep circle 0 0 2 path line 0 0 0 0 0 30 arc xz at 10 0 30 10 180 -90 line 10 0 40 25 0 40\n",
    );
    // the same path named by a piece, drawn back to front
    same_volume(
        "sketch xz\n  line 25 40 10 40\n  arc 10 30 10 40 0 30\n  line 0 30 0 0\n  circle 50 50 3\nend\n\
         sweep z -1 0 0 at 25 0 40 circle 0 0 2 path sketch1 a1\n",
        "sweep z -1 0 0 at 25 0 40 circle 0 0 2 path line 25 0 40 10 0 40 arc xz at 10 0 30 10 90 90 line 0 0 30 0 0 0\n",
    );
    same_volume(
        "sketch\n  rect 0 0 10 10\nend\nsketch at 0 0 20\n  rect 2 2 6 6\nend\nloft profile sketch1 profile sketch2\n",
        "loft profile rect 0 0 10 10 profile at 0 0 20 rect 2 2 6 6\n",
    );
    same_volume(
        "sketch xz\n  circle 10 0 1\nend\ncoil sketch1 pitch 5 turns 2\n",
        "coil xz circle 10 0 1 pitch 5 turns 2\n",
    );
    let e = |rows: &str| {
        error(&format!(
            "sketch xz\n{rows}end\nsweep circle 0 0 1 path sketch1\n"
        ))
        .1
    };
    assert_eq!(
        e("  line 0 0 0 10\n  line 0 10 5 10\n  line 0 10 -5 10\n"),
        "the path branches at an end of l2"
    );
    assert_eq!(
        e("  line 0 0 0 10\n  line 5 0 5 10\n"),
        "the sketch has more than one chain; name one by an entity on it: l1 or l2"
    );
}

#[test]
fn every_constraint_and_dimension() {
    let src = "r = 3\nsketch\n\
        poly 0 0 50 0 50 30 0 30\n\
        pt 25 15\n\
        pt 43 22\n\
        line 10 10 20 12 cons\n\
        line 30 10 40 12 cons\n\
        line 25 -5 25 45 cons\n\
        circle 12 22 3\n\
        circle 12 22 1.5 cons\n\
        circle 40 22 3\n\
        circle 45 15 5 cons\n\
        arc 25 40 15 40 35 40 cw cons\n\
        pt 60 60\n\
        fix p3\n\
        co l1.a o\n\
        hor l1 l3\n\
        ver l2 l4\n\
        hdist o l2.a 50\n\
        vdist l1.a l3.a 30\n\
        mid p1 l3\n\
        par l5 l6\n\
        dist l5 10\n\
        eq l5 l6\n\
        dist l5.a l1 10\n\
        hdist o l5.a 10\n\
        perp l5 l4\n\
        ver l7\n\
        co l7.a l1\n\
        hdist o l7.a 25\n\
        sym l5.a l6.b l7\n\
        conc c1 c2\n\
        rad c1 r\n\
        dia c2 r\n\
        eq c1 c3\n\
        hor c1 c3\n\
        dist c1 c3 28\n\
        hdist o c1 12\n\
        vdist o c1 22\n\
        co p2 c3\n\
        tan l2 c4\n\
        rad c4 2\n\
        vdist o c4 15\n\
        sym a1.a a1.b l7\n\
        dist a1.a a1.b 20\n\
        ref ang l1 l7\n\
        ref dist l5.a l1\n\
        end\nextrude 4 sketch1 l1\n";
    let m = built(src);
    let forge_model::Feature::Sketch(s) = &m.features[1] else {
        panic!("a sketch")
    };
    let pt = |t: &str| crate::sketch::point(s, t).map(|p| [p.x, p.y].map(|v| (v * 1e9).round() / 1e9 + 0.0));
    assert_eq!(pt("p1"), Ok([25.0, 30.0]));
    assert_eq!(pt("l6.a"), Ok([30.0, 10.0]));
    assert_eq!(pt("l6.b"), Ok([40.0, 10.0]));
    assert_eq!(pt("c3"), Ok([40.0, 22.0]));
    assert_eq!(pt("c4"), Ok([48.0, 15.0]));
    assert_eq!(pt("a1.a").map(|p| p[0]), Ok(15.0));
    let p2 = crate::sketch::point(s, "p2").expect("a point");
    assert!((p2.distance(forge_math::Point2::new(40.0, 22.0)) - 3.0).abs() < 1e-9);
    // p2 round c3, the arc's centre and ends up and down the axis, the axis' top
    assert_eq!(
        crate::sketch::freedom(s),
        (4, ["p2", "l7", "a1"].map(String::from).to_vec())
    );
    // driven: the angle the others make, and a distance shown, not held
    let shown: Vec<f64> = s.driven.iter().map(|d| (d.1 * 1e9).round() / 1e9).collect();
    assert_eq!(shown, [(forge_math::dmath::FRAC_PI_2 * 1e9).round() / 1e9, 10.0]);
    assert_eq!(m.select("faces cyl r=3", None).expect("selects").1.len(), 2);
}

#[test]
fn over_constrained_names_the_row_and_the_fix() {
    let e = |rows: &str| {
        error(&format!(
            "sketch\n  rect 0 0 10 10\n  fix l1.a\n  dist l1 10\n{rows}end\n"
        ))
        .1
    };
    assert_eq!(
        e("  dist l3 10\n"),
        "`dist l3 10` is implied by the rows above it (over-constrained): remove it, or make it driven (it then shows its value): `ref dist l3`"
    );
    assert_eq!(
        e("  hor l3\n"),
        "`hor l3` is implied by the rows above it (over-constrained): remove it"
    );
    assert_eq!(
        e("  ref rad l1\n"),
        "`ref rad l1`: rad and dia need a circle or arc"
    );
    assert_eq!(e("  ref dist l1 4\n").split(':').next(), Some("`ref dist l1 4`"));
    assert_eq!(
        e("  ref par l1 l3\n"),
        "`ref par l1 l3`: ref goes before a dimension (dist hdist vdist rad dia ang)"
    );
    let dir = crate::cmd::tests::Dir::new("ref");
    let f = dir.file(
        "r.fc",
        "sketch\n  rect 0 0 10 10\n  fix l1.a\n  dist l1 10\n  ref dist l3\n  ref hdist l1.a l2.b\nend\n",
    );
    let r = crate::cmd::Session::default().run("check", &crate::cmd::tests::args(&f, &[]));
    assert!(
        r.text
            .contains("\n  `ref dist l3` = 10\n  `ref hdist l1.a l2.b` = 10"),
        "{}",
        r.text
    );
}

#[test]
fn sketch_diagnostics() {
    let e = |rows: &str| error(&format!("w = 3\nsketch\n{rows}end\nextrude 1 sketch1\n"));
    let (line, msg, hint) = e("  rect 0 0 10 10\n  dist l1 w\n  dist l1 5\n");
    assert_eq!(line, 2);
    assert_eq!(
        msg,
        "the sketch does not solve: `dist l1 w`, `dist l1 5` conflict"
    );
    assert!(hint.starts_with("change or remove one"), "{hint}");
    assert_eq!(
        e("  rect 0 0 10 10\n  dia c9 3\n").1,
        "`dia c9 3`: no entity 'c9' above this row"
    );
    assert_eq!(
        e("  rect 0 0 10 10\n  dia l1 3\n").1,
        "`dia l1 3`: an entity of the wrong kind for this row"
    );
    assert_eq!(
        e("  rect 0 0 10 10\n  dist l1 -3\n").1,
        "`dist l1 -3`: a value out of range (lengths and radii > 0)"
    );
    assert_eq!(
        e("  rect 0 0 10 10\n  glue l1\n").1.split(':').nth(1),
        Some(
            " unknown row (pt line poly rect circle arc | fix co hor ver par perp tan eq conc mid sym | dist hdist vdist rad dia ang)"
        )
    );
    assert_eq!(
        e("  rect 0 0 10 10\n  circle 30 30 2\n").1,
        "the sketch has 2 regions; name one by an entity on its outside: c1 l1"
    );
    assert_eq!(
        e("  poly 0 0 10 0 10 10 open\n").1,
        "the chain through l1 is open"
    );
    assert_eq!(
        e("  rect 0 0 10 10\n  line 5 -5 5 15\n").1,
        "the chain through l5 is open"
    );
    assert_eq!(e("  rect 0 0 10 10\n  rect 5 5 10 10\n").1, "l2 and l5 cross");
    assert_eq!(e("  rect 0 0 10 10 cons\n").1, "the sketch encloses nothing");
    assert_eq!(
        e("  rect 0 0 10 10\n  fix l1.d\n").1,
        "`fix l1.d`: 'l1.d' is not a point"
    );
    assert_eq!(
        e("  rect 0 0 10 10\n  co l1 l2\n").1,
        "`co l1 l2`: co needs a point"
    );
    // under-constrained is fine, and says how free it is
    let m = built("sketch\n  rect 0 0 10 10\nend\nextrude 1 sketch1\n");
    let forge_model::Feature::Sketch(s) = &m.features[0] else {
        panic!("a sketch")
    };
    assert_eq!(
        crate::sketch::freedom(s),
        (
            4,
            vec!["l1", "l2", "l3", "l4"]
                .into_iter()
                .map(String::from)
                .collect()
        )
    );
}

/// A sketch written at rest reads back the same, every time and through print.
#[test]
fn sketches_and_parameters_print_back() {
    let src = format!("{PLATE}fillet 1 (edges convex along +z)\n");
    let part =
        crate::text::parse(&src.replace("(edges convex along +z)", "@0000000000000001")).expect("parses");
    let printed = crate::text::print(&part);
    assert!(
        printed.starts_with("w = 40\nh = 25\nsketch\n  rect 0 0 38 27\n"),
        "{printed}"
    );
    assert!(
        printed.contains("\n  vdist l1.a c1 h/2\nend\nextrude 6 "),
        "{printed}"
    );
    assert_eq!(crate::text::parse(&printed).expect("parses"), part);
    let m = built(&src);
    let mut m2 = built(&src.replace("h/2", "(h / 2)"));
    assert_eq!(m.body(), m2.body());
    m2.update(&src);
    assert_eq!(
        m2.rebuilt, 0,
        "the sketch reads the same, so nothing below rebuilds"
    );
}

/// Edits through the document rebuild from the first line they change, and end where a fresh
/// build of the file they print does; references follow the items they name.
#[test]
fn edits_rebuild_like_the_file_they_print() {
    use forge_doc::Edit;
    let src = "w = 40\nsketch\n  rect 0 0 w 25\n  fix l1.a\n  dist l1 w\n  dist l2 25\nend\n\
               extrude 6 sketch1\nsketch on sketch1\n  circle 10 10 2\nend\nhole 3 on sketch2 c1 thru\n";
    let mut m = built(src);
    let fresh = |m: &Model| {
        assert!(m.error.is_none(), "{:?}\n{}", m.error, m.src());
        assert_eq!(format!("{:?}", m.body().expect("built").compact()), body(m.src()));
    };
    let item = |m: &Model, k: usize| m.doc.features().nth(k).expect("a line").id;
    m.edit(Edit::SetParam {
        name: "w".into(),
        expr: "55".into(),
    })
    .expect("set");
    assert_eq!(m.rebuilt, 3, "from the extrude on");
    fresh(&m);
    m.edit(Edit::SetDimension {
        item: item(&m, 1),
        row: 3,
        value: "30".into(),
    })
    .expect("dimension");
    fresh(&m);
    // a line in above: the hole still drills at the second sketch, now the third
    m.edit(Edit::Insert {
        at: 1,
        text: "sketch xz\n  circle 0 0 1\nend".into(),
    })
    .expect("insert");
    assert!(
        m.src()
            .ends_with("sketch on sketch2\n  circle 10 10 2\nend\nhole 3 on sketch3 c1 thru\n"),
        "{}",
        m.src()
    );
    fresh(&m);
    let hole = item(&m, 5);
    m.edit(Edit::SetArg {
        item: hole,
        arg: 0,
        text: "w/10".into(),
    })
    .expect("arg");
    assert_eq!(m.rebuilt, 1);
    fresh(&m);
    m.edit(Edit::Delete { item: item(&m, 1) }).expect("delete");
    assert!(m.src().contains("hole w/10 on sketch2 c1 thru"), "{}", m.src());
    fresh(&m);
    assert!(
        m.edit(Edit::SetParam {
            name: "nope".into(),
            expr: "1".into()
        })
        .is_err()
    );
    // a re-read file keeps the links of the lines it kept
    let src = m.src().to_string();
    m.update(&src.replace("hole w/10", "hole w/11"));
    m.edit(Edit::Move {
        item: item(&m, 0),
        to: 0,
    })
    .expect("move");
    assert!(m.src().contains("hole w/11 on sketch2 c1 thru"), "{}", m.src());
}

/// A dimension changed alone is set in place, and the sketch is what reading it afresh gives,
/// bit for bit; a coordinate or parameter changed reads the rows again.
#[test]
fn a_dimension_changed_alone_reads_as_afresh() {
    let src = |d: &str, x: &str| {
        format!(
            "w = 40\nsketch\n  rect 0.2 0 {x} 25\n  circle 20 12 4\n  fix l1.a\n  dist l1 w\n  dist l2 {d}\n\
             dia c1 8\n  hdist l1.a c1 (w/2)\n  vdist l1.a c1 12\n  ref dist l3\nend\nextrude 6 sketch1 l1\n"
        )
    };
    let mut m = built(&src("25", "40"));
    for (d, x) in [("30", "40"), ("w/2", "40"), ("w/2", "41"), ("26", "41")] {
        m.update(&src(d, x));
        let fresh = built(&src(d, x));
        assert_eq!(m.features, fresh.features, "{d} {x}");
    }
}

/// Printed and read back, a part is the same part.
fn round_trip(part: &forge_model::Part) {
    let text = text::print(part);
    let back = text::parse(&text).unwrap_or_else(|(l, e)| panic!("line {l}: {e}\n{text}"));
    assert_eq!(back.features, part.features, "{text}");
}

/// Work features, patterns along a path and at points, and standard holes, written as an
/// agent would, build and print back to the same features.
#[test]
fn work_features_and_standards() {
    let src = "base: extrude 10 rect 0 0 60 40\n\
        boss: extrude 8 at 50 30 10 circle 0 0 4\n\
        top: plane off base (faces of base normal +z) 5\n\
        ax: axis boss (faces cyl)\n\
        c: point meet ax top\n\
        mid: point mid base (edges of base along +x front top)\n\
        ucs pts base (verts near 0 0 10) base (verts near 60 0 10) base (verts near 0 40 10)\n\
        plane ang base (faces of base normal +z) base (edges of base along +x front top) 30\n\
        extrude cut 2 on top at c circle 0 0 2\n\
        h: hole M6 tap -xy at 10 10 10 thru\n\
        hole M5 ansi close cbore -xy at 30 30 10 12\n\
        pattern h pts 10 10 10 10 20 10 10 30 10\n\
        pattern h path base (edges of base along +x front top) 3 15\n\
        pattern hole2 path base (edges of base line top) 6 fit fixed\n";
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error);
    let Feature::Hole(h) = &m.features[9] else {
        panic!("a hole")
    };
    assert_eq!(
        (h.diameter, h.thread),
        (
            5.0,
            Some(Thread {
                size: 6.0,
                pitch: 1.0
            })
        )
    );
    let Feature::Hole(h) = &m.features[10] else {
        panic!("a hole")
    };
    assert_eq!(
        (h.diameter, h.head),
        (
            5.3,
            Head::Counterbore {
                diameter: 9.75,
                depth: 5.0
            }
        )
    );
    let part = Part {
        features: m.features.clone(),
    };
    let text = text::print(&part);
    assert!(
        text.contains("hole M6 iso tap -xy") && text.contains("hole M5 ansi close cbore -xy"),
        "{text}"
    );
    round_trip(&part);
    let vol = |b: &forge_topo::Body| {
        assert!(forge_topo::check(b).is_empty());
        forge_mesh::tessellate(b, &forge_mesh::Quality::default())
            .expect("meshes")
            .volume()
    };
    let (a, b) = (
        vol(&part.regenerate_nary(1).expect("nary")),
        vol(&part.regenerate().expect("regen")),
    );
    assert!((a - b).abs() < 1e-6 * a, "{a} vs {b}");
    assert_eq!(*m.body().expect("built"), part.regenerate().expect("regen"));
}

/// A hole copied to a sketch's points follows them when the sketch changes.
#[test]
fn holes_at_sketch_points() {
    let src = |x: &str| {
        format!(
            "sk: sketch xy at 0 0 10\n  pt 10 10\n  pt 30 10\n  pt {x} 25\nend\n\
             extrude 10 rect 0 0 60 40\n\
             h: hole M4 iso tap -on sk p1 thru\n\
             pattern h pts sk\n\
             pattern h pts sk p2 p1\n"
        )
    };
    for x in ["50", "45"] {
        let m = Model::new(&src(x));
        assert!(m.error.is_none(), "{:?}", m.error);
        let Feature::Pattern {
            layout: Layout::Points(at),
            ..
        } = &m.features[3]
        else {
            panic!("a points pattern")
        };
        let want = [(10.0, 10.0), (30.0, 10.0), (x.parse::<f64>().expect("x"), 25.0)];
        let At::Sketch { points, at, .. } = &at[0] else {
            panic!("sketch points")
        };
        assert!(points.is_empty());
        for (p, (x, y)) in at.iter().zip(want) {
            assert!(
                (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9 && p.z == 10.0,
                "{p:?}"
            );
        }
        let part = Part {
            features: m.features.clone(),
        };
        let text = text::print(&part);
        assert!(
            text.contains("pattern hole1 pts sketch1\n") && text.contains("pts sketch1 p2 p1"),
            "{text}"
        );
        round_trip(&part);
        // the second pattern copies onto p1 again from p2: the third hole lands at p1 + (p1 - p2)
        let b = part.regenerate_nary(1).expect("regen");
        assert!(forge_topo::check(&b).is_empty());
    }
}

/// `centers` adds a sketch's circle and arc centres, in sketch order, to its points.
#[test]
fn holes_at_sketch_centers() {
    let src = "sk: sketch xy at 0 0 10\n  pt 10 10\n  circle 30 10 3\n  arc 50 25 53 25 47 25\nend\n\
               extrude 10 rect 0 0 60 40\n\
               h: hole 4 -on sk p1 thru\n\
               pattern h pts sk centers\n";
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error);
    let Feature::Pattern {
        layout: Layout::Points(at),
        ..
    } = &m.features[3]
    else {
        panic!("a points pattern")
    };
    let At::Sketch { centers, at, .. } = &at[0] else {
        panic!("sketch points")
    };
    let got: Vec<(f64, f64)> = at.iter().map(|p| (p.x, p.y)).collect();
    assert!(
        *centers && got == [(10.0, 10.0), (30.0, 10.0), (50.0, 25.0)],
        "{got:?}"
    );
    let part = Part {
        features: m.features.clone(),
    };
    assert!(text::print(&part).contains("pattern hole1 pts sketch1 centers\n"));
    round_trip(&part);
    let b = part.regenerate_nary(1).expect("regen");
    assert!(forge_topo::check(&b).is_empty());
}
