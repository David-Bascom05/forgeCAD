//! Text in a sketch: glyphs drawn exactly from the shipped fonts, extruded and cut as regions,
//! their faces named by character so they keep their names as the text changes.

use forge_lang::{Options, Run};

fn run(src: &str) -> Run {
    let opts = Options {
        anchors: true,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn part(text: &str, extra: &str, op: &str) -> String {
    format!(
        "shared L = 80
shared W = 30
shared t = 4
param top.size = 10
param top.x = 6
param top.y = 10
param raised.h = 1.5
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (L, W))
}}
base = extrude(plan, t)
sketch top(on: base.end) {{
  label = text(\"{text}\", at: (x, y), size: size{extra})
}}
raised = {op}(top.label, h)
"
    )
}

fn volume(r: &Run) -> f64 {
    forge_topo::mass::mass(r.body.as_ref().expect("a body"))
        .expect("mass")
        .volume
}

#[test]
fn raised_and_cut_text_builds_valid() {
    for font in [
        "",
        ", font: \"Source Sans 3\"",
        ", bold: true",
        ", italic: true, align: \"center\"",
    ] {
        for op in ["extrude", "cut"] {
            let r = run(&part("FORGE 40", font, op));
            assert!(r.ok(), "{font} {op}: {:?}", r.errors);
            let b = r.body.as_ref().unwrap();
            assert!(forge_topo::check(b).is_empty(), "{font} {op}");
            let v = volume(&r);
            let plate = 80.0 * 30.0 * 4.0;
            if op == "extrude" {
                assert!(v > plate, "{font}: {v}");
            } else {
                assert!(v < plate, "{font}: {v}");
            }
        }
    }
}

/// Counters are holes: an O raised is a ring, its inside not raised.
#[test]
fn counters_are_holes() {
    let o = run(&part("O", "", "extrude"));
    let i = run(&part("l", "", "extrude"));
    assert!(o.ok() && i.ok(), "{:?} {:?}", o.errors, i.errors);
    let plate = 80.0 * 30.0 * 4.0;
    let ring = volume(&o) - plate;
    // the O's ring is far less than its outline's disc
    let b = o.body.as_ref().unwrap();
    let top = b
        .face_ids()
        .filter(|&f| forge_agent::select::plane_normal(b, f).is_some_and(|n| n.z > 0.999))
        .filter(|&f| forge_agent::select::centre(b, forge_agent::select::Ent::Face(f)).z > 4.1)
        .count();
    assert_eq!(top, 1, "the ring's top, one face with a hole");
    assert!(ring > 0.0 && ring < volume(&i) * 4.0);
}

/// Each character's walls keep their names when other characters come and go, when the text
/// moves, and when it is resized.
#[test]
fn faces_keep_their_names() {
    let names = |src: &str| -> Vec<forge_topo::Name> {
        let r = run(src);
        assert!(r.ok(), "{:?}", r.errors);
        let b = r.body.as_ref().unwrap();
        let mut n: Vec<_> = b.face_ids().map(|f| b.face(f).name).collect();
        n.sort();
        n
    };
    let ab = names(&part("AB", "", "extrude"));
    let xab = names(&part("XAB", "", "extrude"));
    let abc = names(&part("ABC", "", "extrude"));
    let moved = names(&part("AB", ", angle: 20deg", "extrude").replace("param top.x = 6", "param top.x = 9"));
    // A's and B's faces are all still there (the plate's base is its own)
    for n in &ab {
        assert!(xab.contains(n) && abc.contains(n) && moved.contains(n));
    }
}

/// `raised.side(top.label[k])` is the walls of the k-th drawn character, `raised.end` every
/// glyph's top; every face of the part has an anchor.
#[test]
fn anchors_name_every_face() {
    let r = run(&part("A B", "", "extrude"));
    assert!(r.ok(), "{:?}", r.errors);
    let a = r.anchors.as_ref().unwrap();
    let b = r.body.as_ref().unwrap();
    for f in b.face_ids() {
        assert!(a.faces.contains_key(&b.face(f).name), "a face with no anchor");
    }
    let texts: Vec<&String> = a.faces.values().collect();
    assert!(
        texts.iter().any(|t| t.as_str() == "raised.side(top.label[0])"),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.as_str() == "raised.side(top.label[1])"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t.as_str() == "raised.end"), "{texts:?}");
}

#[test]
fn refusals_are_typed() {
    let kind = |src: String| {
        let r = run(&src);
        assert!(!r.ok());
        format!("{:?}", r.errors[0])
    };
    // a family not shipped, a style it lacks, a character it lacks, nothing drawn
    assert!(kind(part("A", ", font: \"Nope Sans\"", "extrude")).contains("NoFamily"));
    assert!(kind(part("A", ", font: \"Source Sans 3\", bold: true", "extrude")).contains("NoStyle"));
    assert!(kind(part("A\u{4e2d}", "", "extrude")).contains("NoGlyph"));
    assert!(kind(part("  ", "", "extrude")).contains("Empty"));
    // Open Sans's C cedilla: its cedilla crosses the C, refused rather than joined
    assert!(kind(part("\u{c7}", "", "extrude")).contains("SketchProfile"));
}

/// Text as a part's first, new body, and as a new body beside another: its glyphs one body.
#[test]
fn text_as_a_new_body() {
    let alone = "param top.size = 10
param raised.h = 2
sketch top(on: xy) {
  label = text(\"AB\", size: size)
}
raised = extrude(top.label, h)
";
    let r = run(alone);
    assert!(r.ok(), "{:?}", r.errors);
    assert!(volume(&r) > 0.0);
    let beside = "param top.size = 10
param raised.h = 2
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (40, 20))
}
base = extrude(plan, 4)
sketch top(on: xy) {
  label = text(\"AB\", at: (0, 30), size: size)
}
raised = extrude(top.label, h, new: true)
";
    let b = run(beside);
    assert!(b.ok(), "{:?}", b.errors);
    assert!((volume(&b) - 40.0 * 20.0 * 4.0 - volume(&r)).abs() < 1e-9 * volume(&b));
}

/// A list of strings sets lines, each the font's line spacing below the one before.
#[test]
fn lines_from_a_list() {
    let one = run(&part("AB", "", "extrude"));
    let two = run(&part("AB", "", "extrude").replace("text(\"AB\"", "text([\"AB\", \"AB\"]"));
    assert!(one.ok() && two.ok(), "{:?}", two.errors);
    let plate = 80.0 * 30.0 * 4.0;
    let (v1, v2) = (volume(&one) - plate, volume(&two) - plate);
    assert!((v2 - 2.0 * v1).abs() < 1e-9 * v2, "{v1} {v2}");
}

/// The nameplate example builds valid: bold Open Sans raised, Source Sans 3 (CFF) cut.
#[test]
fn the_nameplate_example_builds() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined/nameplate.fg");
    let r = run(&std::fs::read_to_string(p).expect("the example"));
    assert!(r.ok(), "{:?}", r.errors);
    assert!(forge_topo::check(r.body.as_ref().unwrap()).is_empty());
}
