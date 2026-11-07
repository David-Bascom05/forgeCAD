//! Sheet refusals that say what and where: a seam on a hem, a corner left closed, an edge at a
//! slant to a tangent line, a corner round too big; and `unfold`, asked before an unfold is.

use forge_agent::json::Json;
use forge_lang::error::Op;
use forge_lang::{Options, Run};

const HEAD: &str = "shared t = 1.5
shared w = 100
shared d = 80
shared h = 30
shared leg = 8
shared g = 0.5
shared k = ratio(0.42)
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (w, d))
}
";

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn error(r: &Run, src: &str) -> Json {
    r.errors.first().unwrap_or_else(|| panic!("refused")).json(src)
}

fn fixes(j: &Json) -> Vec<Op> {
    match j.get("fixes") {
        Some(Json::Arr(x)) => x.iter().filter_map(Op::from_json).collect(),
        _ => vec![],
    }
}

#[test]
fn a_seam_on_a_hem_names_the_hem_and_its_edge() {
    let src = format!(
        "{HEAD}base = sheet(plan, t: t, r: t, bend: kfactor(k))
a = flange(base.edge(plan.outline[1]), height: h)
b = hem(base.edge(plan.outline[2]), kind: \"open\", length: leg, gap: g, side: \"down\")
s = seam(a, b, gap: g)
"
    );
    let j = error(&run(&src), &src);
    assert_eq!(j.get("kind").and_then(Json::str), Some("SeamNotAFlange"));
    let text = j.get("text").and_then(Json::str).unwrap();
    assert!(text.contains("b is not a plain flange: it is a hem"), "{text}");
    assert_eq!(j.get("flange").and_then(Json::str), Some("b"));
    // the back edge of the base, its top skin at z = 0, cut back 1.5 at x = 100 where a stands
    // inside the base's edge
    let Some(Json::Arr(e)) = j.get("edge") else {
        panic!("{j}")
    };
    let p = |x: &Json| match x {
        Json::Arr(v) => v.iter().map(|n| n.num().unwrap()).collect::<Vec<f64>>(),
        _ => panic!(),
    };
    assert_eq!(p(&e[0]), vec![98.5, 80.0, 0.0]);
    assert_eq!(p(&e[1]), vec![0.0, 80.0, 0.0]);
}

#[test]
fn a_closed_corner_gets_a_relief_that_builds() {
    let src = format!(
        "{HEAD}base = sheet(plan, t: t, r: t, bend: kfactor(k), corner: closed_corner)
a = flange(base.edge(plan.outline[1]), height: h)
b = flange(base.edge(plan.outline[2]), height: h)
"
    );
    let j = error(&run(&src), &src);
    assert_eq!(j.get("kind").and_then(Json::str), Some("CornerUnbuilt"));
    let text = j.get("text").and_then(Json::str).unwrap();
    // both flanges stand inside the base's edges: the corner moved in t each way
    assert!(
        text.contains("the corner where a and b meet at (98.5, 78.5, 0)"),
        "{text}"
    );
    let f = fixes(&j);
    assert_eq!(f.len(), 1, "{j}");
    let fixed = forge_lang::edit::apply(&src, &f[0]).unwrap();
    let r = run(&fixed);
    assert!(r.errors.is_empty(), "{:?}", r.errors.first().map(|e| e.text()));
}

#[test]
fn a_corner_round_rounds_free_corners_in_the_blank_and_says_when_it_is_too_big() {
    // a base with two flanges: their tips' four corners and the base's two free ones
    let src = format!(
        "{HEAD}shared rr = 4
base = sheet(plan, t: t, r: t, bend: kfactor(k))
a = flange(base.edge(plan.outline[1]), height: h)
b = flange(base.edge(plan.outline[3]), height: h)
round = corner_round(base, r: rr)
flat = unfold(base, on: offset(base.bottom, 60))
"
    );
    let r = run(&src);
    assert!(r.errors.is_empty(), "{:?}", r.errors.first().map(|e| e.text()));
    let body = r.body.as_ref().unwrap();
    // rounds are cylinders of radius 4 square to their walls, on both bodies (folded and flat)
    let rounds = body
        .face_ids()
        .filter(|f| match &body.face(*f).surface {
            forge_geom::Surface::Cylinder(c) => (c.radius - 4.0).abs() < 1e-9,
            _ => false,
        })
        .count();
    // the base's outline corners all carry a bend on one side: only the flanges' tips round
    assert_eq!(rounds, 2 * 4, "four tip corners, folded and flat");
    let dxf = r.sheets[0].dxf().unwrap();
    assert_eq!(dxf.matches("\nARC\n").count(), 4, "the blank's four rounds");
    // too big for a 30 high flange's half: the fix is the largest that fits, and builds
    let big = src.replace("shared rr = 4", "shared rr = 20");
    let j = error(&run(&big), &big);
    assert_eq!(j.get("kind").and_then(Json::str), Some("CornerRoundTooBig"));
    let f = fixes(&j);
    let fixed = forge_lang::edit::apply(&big, &f[0]).unwrap();
    assert!(run(&fixed).errors.is_empty());
}

#[test]
fn unfold_says_before_an_unfold_whether_one_blank_folds_it() {
    let src = format!(
        "{HEAD}base = sheet(plan, t: t, r: t, bend: kfactor(k))
a = flange(base.edge(plan.outline[1]), height: h)
"
    );
    let a = forge_lang::query::ask(&src, None, None, "unfold");
    assert!(a.ok, "{}", a.text);
    assert_eq!(a.json.get("unfolds"), Some(&Json::Bool(true)));
    // the blank: the base and the flange, less one 90deg bend's deduction (two setbacks less the
    // allowance), the outer dimensions 100 and 30 along x
    let ba = (1.5 + 0.42 * 1.5) * forge_math::dmath::FRAC_PI_2;
    let Some(Json::Arr(sh)) = a.json.get("sheets") else {
        panic!()
    };
    let Some(Json::Arr(size)) = sh[0].get("size") else {
        panic!()
    };
    let x = size[0].num().unwrap();
    assert!((x - (100.0 + 30.0 - 2.0 * 3.0 + ba)).abs() < 1e-6, "{x}");
    assert_eq!(size[1].num(), Some(80.0));
    // a tab turned down off a notch's side lies on the floor in the blank: said before any unfold
    let src = "shared t = 1.5
shared w = 100
shared d = 80
shared h = 15
shared k = ratio(0.42)
sketch plan(on: xy) {
  outline = poly([(0, 0), (30, 0), (30, -18), (70, -18), (70, 0), (w, 0), (w, d), (0, d)])
}
base = sheet(plan, t: t, r: t, bend: kfactor(k))
tab = flange(base.edge(plan.outline[3]), height: h, side: \"down\")
";
    let a = forge_lang::query::ask(src, None, None, "unfold");
    assert_eq!(a.json.get("unfolds"), Some(&Json::Bool(false)), "{}", a.text);
    let Some(Json::Arr(stops)) = a.json.get("stops") else {
        panic!()
    };
    assert_eq!(stops[0].get("kind").and_then(Json::str), Some("FlatTouch"));
    assert!(!fixes(&stops[0]).is_empty());
}

#[test]
fn a_slanted_edge_at_a_tangent_line_says_where() {
    // a trapezoid's sides lean in 10 over 80: a, inside the base's edge, moves it to y = 1.5,
    // where the right side is at 100 - 1.5 / 8; 3 further up, at the tangent line, 0.375 in
    let src = "shared t = 1.5
shared k = ratio(0.42)
shared h = 30
sketch plan(on: xy) {
  outline = poly([(0, 0), (100, 0), (90, 80), (10, 80)])
}
base = sheet(plan, t: t, r: t, bend: kfactor(k))
a = flange(base.edge(plan.outline[0]), height: h)
";
    let j = error(&run(src), src);
    assert_eq!(j.get("kind").and_then(Json::str), Some("TangentMismatch"));
    let text = j.get("text").and_then(Json::str).unwrap();
    assert!(
        text.contains("base's edge meets a's tangent line on base's side 99.625 along the bend"),
        "{text}"
    );
    let Some(Json::Arr(p)) = j.get("at_part") else {
        panic!("{j}")
    };
    assert!(
        (p[0].num().unwrap() - 99.8125).abs() < 1e-9 && p[1].num() == Some(1.5),
        "{j}"
    );
}
