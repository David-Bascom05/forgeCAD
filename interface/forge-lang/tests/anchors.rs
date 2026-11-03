//! Anchors: each face and edge of a finished part with the anchor text that names it, the text
//! being what the evaluator resolves back to that same face or edge.

use forge_lang::{Options, Run};

fn run(src: &str) -> Run {
    let opts = Options {
        anchors: true,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

const BOX: &str = "shared L = 40
shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
param top.r = 5
sketch top(on: base.end) {
  c = circle((L / 2, L / 2), r)
}
param boss.h = 4
boss = extrude(top.c, h)
";

#[test]
fn every_face_of_an_extruded_part_has_its_role_anchor() {
    let r = run(BOX);
    assert!(r.ok(), "{:?}", r.errors);
    let a = r.anchors.as_ref().unwrap();
    let body = r.body.as_ref().unwrap();
    let mut texts: Vec<&str> = body
        .face_ids()
        .map(|f| a.faces.get(&body.face(f).name).map_or("-", String::as_str))
        .collect();
    texts.sort();
    assert!(!texts.contains(&"-"), "{texts:?}");
    for want in ["base.start", "base.side(plan.outline[1])", "boss.end", "boss.side(top.c)"] {
        assert!(texts.contains(&want), "{want} not in {texts:?}");
    }
    // the boss's top rim: between its wall and its cap
    let rim: Vec<&String> = a.edges.values().filter(|t| t.contains("boss.end")).collect();
    assert!(
        rim.iter().any(|t| *t == "edges(between: (boss.end, boss.side(top.c)))"),
        "{rim:?}"
    );
    // its foot, where it meets the base's top
    assert!(a.edges.values().any(|t| t == "edges(between: (base.end, boss.side(top.c)))"));
    let plan = a.sketches.iter().find(|s| s.id == "plan").unwrap();
    assert!(plan.lines.contains(&"plan.outline[0]".to_string()), "{plan:?}");
    assert!(plan.closed.contains(&"plan.outline".to_string()));
}

#[test]
fn an_anchor_written_into_a_feature_builds_what_was_picked() {
    let r = run(BOX);
    let a = r.anchors.as_ref().unwrap();
    let body = r.body.as_ref().unwrap();
    let e = body
        .edge_ids()
        .find(|&e| a.edges.get(&body.edge(e).name).is_some_and(|t| t.contains("boss.end")))
        .unwrap();
    let sel = &a.edges[&body.edge(e).name];
    let src = format!("{BOX}param round.r = 1\nround = fillet({sel}, r)\n");
    let r2 = run(&src);
    assert!(r2.ok(), "{:?}", r2.errors);
    assert_eq!(
        r2.measure.as_ref().unwrap().faces,
        r.measure.as_ref().unwrap().faces + 1
    );
}

#[test]
fn bracket_anchors_most_faces_and_loops_and_calls_resolve() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/parts/bracket.fg")).unwrap();
    let t = std::time::Instant::now();
    let r = run(&src);
    let ms = t.elapsed().as_secs_f64() * 1e3;
    let plain = forge_lang::run(&src, Options::default(), Default::default()).0;
    assert!(r.ok(), "{:?}", r.errors);
    eprintln!("bracket: run with anchors {ms:.1} ms, without {:.1} ms", plain.timings.total_ms);
    let a = r.anchors.as_ref().unwrap();
    let body = r.body.as_ref().unwrap();
    let n = body.face_ids().count();
    let named = body.face_ids().filter(|&f| a.faces.contains_key(&body.face(f).name)).count();
    eprintln!("bracket: {named} of {n} faces anchored, {} edges", a.edges.len());
    assert!(named * 2 > n, "{named} of {n}");
}

#[test]
fn a_face_anchors_normal_directs_a_pattern() {
    for (dir, ok) in [
        ("base.end.normal", true),
        ("plan.outline[0].dir", true),
        ("base.side(plan.outline[1]).normal", true),
    ] {
        let src = format!(
            "{BOX}param pat.n = count(2)\nparam pat.s = 20\npat = pattern(of: boss, layout: lin(along: {dir}, n: n, step: s))\n"
        );
        let r = run(&src);
        eprintln!("{dir}: {:?}", r.errors.iter().map(|e| e.text()).collect::<Vec<_>>());
        assert_eq!(r.ok(), ok, "{dir}");
    }
}

#[test]
fn a_side_faces_normal_checks_without_the_kernel() {
    let src = format!(
        "{BOX}param pat.n = count(2)\nparam pat.s = 20\npat = pattern(of: boss, layout: lin(along: base.side(plan.outline[1]).normal, n: n, step: s))\n"
    );
    let opts = Options {
        build: false,
        ..Options::default()
    };
    let r = forge_lang::run(&src, opts, Default::default()).0;
    assert!(r.errors.is_empty(), "{:?}", r.errors);
}
