//! The newer sheet features in Forge (contour, ...): parts in parts/sheet against forge-sheet's
//! closed forms, and their refusals typed.
use forge_lang::error::Error;
use forge_lang::{Options, Run};
use forge_math::dmath::PI;
use forge_topo::Body;

fn src(name: &str) -> String {
    let path = format!("{}/parts/sheet/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect("part file")
}

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn clean(name: &str) -> Run {
    let s = src(name);
    let r = run(&s);
    assert!(
        r.errors.is_empty() && r.warnings.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&r, &s)
    );
    assert!(r.expects.iter().all(|e| e.ok), "{name}");
    assert_eq!(r.measure.as_ref().expect("measured").problems, 0, "{name}");
    r
}

fn mass(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).expect("mass");
    (m.volume, m.err[0])
}

fn near(what: &str, got: (f64, f64), want: f64) {
    assert!(
        (got.0 - want).abs() <= got.1 + 1e-9 * want.abs().max(1.0),
        "{what}: {} against {want}",
        got.0
    );
}

fn first_error(s: &str) -> Error {
    run(s)
        .errors
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("no error in\n{s}"))
}

/// Folded and flat volumes: straights (tangent line to tangent line) and bends (angle, inside
/// radius), all `w` long.
fn closed(t: f64, k: f64, w: f64, straights: f64, bends: &[(f64, f64)]) -> (f64, f64) {
    let zones: f64 = bends.iter().map(|&(a, r)| a * (r * t + 0.5 * t * t)).sum();
    let ba: f64 = bends.iter().map(|&(a, r)| a * (r + k * t)).sum();
    (t * w * straights + w * zones, t * w * (straights + ba))
}

#[test]
fn a_hat_contour_matches_the_closed_form() {
    let r = clean("hat");
    let (t, k, w) = (1.5, 0.4, 25.0);
    let q = PI / 2.0;
    // the r 4 arc turns towards the material: inside radius 4 - t; the sharp corner the same way
    // at the sheet's radius 2, so both lines there lose (2 + t) to the reference setback
    let straights = 15.0 + 14.0 + (18.0 - 3.5) + (21.0 - 3.5);
    let (vf, vl) = closed(t, k, w, straights, &[(q, 3.0), (q, 4.0 - t), (q, 2.0)]);
    let s = &r.sheets[0];
    assert_eq!(s.walls.len(), 4);
    near("hat folded", mass(s.folded.as_ref().expect("built")), vf);
    near("hat flat", mass(s.unfolds[0].1.as_ref().expect("unfolded")), vl);
}

#[test]
fn a_contour_off_an_edge_matches_the_closed_form() {
    let r = clean("lip");
    let (t, k, w) = (2.0, 0.42, 30.0);
    let q = PI / 2.0;
    // the base to its tangent line (setback 2 at r 2), the wall up (17 less that setback), the
    // lip after the r 3 arc (12), the arc turning towards the material (inside radius 3 - t)
    let straights = (40.0 - 2.0) + (17.0 - 2.0) + 12.0;
    let (vf, vl) = closed(t, k, w, straights, &[(q, 2.0), (q, 3.0 - t)]);
    let s = &r.sheets[0];
    assert_eq!(s.walls.len(), 3);
    near("lip folded", mass(s.folded.as_ref().expect("built")), vf);
    near("lip flat", mass(s.unfolds[0].1.as_ref().expect("unfolded")), vl);
}

#[test]
fn contour_refusals_are_typed() {
    // an arc turning towards the material tighter than t
    let s = src("hat").replace("param sec.r2 = 4", "param sec.r2 = 1");
    assert_eq!(first_error(&s).kind(), "ContourRadius");
    // a last line shorter than the sharp corner's setback (2 + t)
    let s = src("hat").replace("line(to: (b, 0))", "line(to: (b, h + r2 - 3))");
    assert_eq!(first_error(&s).kind(), "ContourWallTooShort");
    // off an edge, the profile on a plane not square to it
    let s = src("lip").replace("about: plan.outline[0]", "about: plan.outline[1]");
    let e = first_error(&s);
    assert!(
        matches!(
            e.kind(),
            "ProfileOffSection" | "ContourStartOffEdge" | "ContourCoplanar"
        ),
        "{}",
        e.kind()
    );
    // a contour takes no width off an edge
    let s = src("lip").replace(
        "edge: base.edge(plan.outline[1]))",
        "edge: base.edge(plan.outline[1]), width: 3)",
    );
    assert_eq!(first_error(&s).kind(), "NotWith");
}

#[test]
fn hems_match_the_closed_form() {
    let r = clean("hems");
    let (t, k, w, rr) = (1.5, 0.4, 30.0, 2.0);
    let q = PI / 2.0;
    // base 50 less the flange's setback (2) and the closed hem's fold (gap floor / 2 + t); the
    // flange 25 to the outer sharp (t less on the reference skin) less its setback and the open
    // hem's fold (0.5 + t); the legs
    let floor = forge_sheet::GAP_FLOOR;
    let straights = (50.0 - rr - (0.5 * floor + t)) + (25.0 - t - rr - (0.5 + t)) + 8.0 + 10.0;
    let zones =
        q * (rr * t + 0.5 * t * t) + PI * (0.5 * t + 0.5 * t * t) + PI * (0.5 * floor * t + 0.5 * t * t);
    let ba = q * (rr + k * t) + PI * (0.5 + k * t) + 2.1;
    let s = &r.sheets[0];
    near(
        "hems folded",
        mass(s.folded.as_ref().expect("built")),
        t * w * straights + w * zones,
    );
    near(
        "hems flat",
        mass(s.unfolds[0].1.as_ref().expect("unfolded")),
        t * w * (straights + ba),
    );
}

#[test]
fn hem_refusals_are_typed() {
    // a closed hem with no row in the shop's table
    let s = src("hems")
        .replace(", closed_hems: [(t, hem_ba)]", "")
        .replace("shared hem_ba = 2.1\n", "");
    assert_eq!(first_error(&s).kind(), "HemNotTabled");
    let s = src("hems").replace("param lip.g = 1", "param lip.g = 0.01");
    assert_eq!(first_error(&s).kind(), "HemGap");
    let s = src("hems").replace(
        "kind: \"open\", length: leg, gap: g",
        "kind: \"open\", length: leg",
    );
    assert_eq!(first_error(&s).kind(), "MissingArg");
}

#[test]
fn seams_build_clean() {
    // the planes the seams put the walls' ends on are checked exactly in forge-sheet's tests;
    // here the four seams build clean both ways, and the right wall reaches the back's far skin
    let r = clean("seams");
    let b = r.sheets[0].folded.as_ref().expect("built");
    let (l, w, t) = (60.0, 40.0, 1.5);
    let ymax = b
        .vertex_ids()
        .map(|v| b.vertex(v).point)
        .filter(|p| p.x > l - 1e-9)
        .map(|p| p.y)
        .fold(f64::MIN, f64::max);
    assert!((ymax - (w + t)).abs() <= 1e-12 * w, "{ymax}");
    assert!(r.sheets[0].unfolds[0].1.is_some());
}

#[test]
fn seam_refusals_are_typed() {
    let s = src("seams").replace("s1 = seam(right, back,", "s1 = seam(right, left,");
    assert_eq!(first_error(&s).kind(), "SeamNotACorner");
    let s = src("seams").replace("shared g = 1", "shared g = 0.001");
    assert_eq!(first_error(&s).kind(), "SeamGap");
}

#[test]
fn punches_take_out_their_tools() {
    let r = clean("punched");
    let plain = {
        let s: String = src("punched")
            .lines()
            .filter(|l| !l.starts_with('p') || l.starts_with("param"))
            .collect::<Vec<_>>()
            .join("\n");
        run(&s)
    };
    let t = 1.5;
    let area = PI * 9.0 + (7.0 * 5.0 + PI * 6.25) + 0.5 * 3.0f64.sqrt() * 100.0;
    for (what, a, b) in [
        (
            "folded",
            r.sheets[0].folded.as_ref(),
            plain.sheets[0].folded.as_ref(),
        ),
        (
            "flat",
            r.sheets[0].unfolds[0].1.as_ref(),
            plain.sheets[0].unfolds[0].1.as_ref(),
        ),
    ] {
        let (a, b) = (mass(a.expect("built")), mass(b.expect("built")));
        near(what, (b.0 - a.0, a.1 + b.1), t * area);
    }
    let dxf = r.sheets[0].dxf().expect("a dxf");
    assert!(dxf.contains("\nPUNCH\n") && dxf.contains("\nHX10\n"));
}

#[test]
fn punch_refusals_and_violations() {
    let s = src("punched").replace("punch(\"R6\"", "punch(\"R9\"");
    assert_eq!(first_error(&s).kind(), "NoSuchTool");
    // a tool written out that the shop does not stock: built, a typed violation
    let s = src("punched").replace("punch(\"R6\"", "punch((\"R7\", \"round\", 7)");
    let r = run(&s);
    assert!(r.errors.is_empty());
    // a cluster tool's counts: whole, finite, 1 to the cap
    for counts in ["1e9, 1", "1, 1e400", "2.5, 2", "0, 3", "1001, 1"] {
        let s = src("punched").replace(
            "punch(\"R6\"",
            &format!("punch((\"C\", \"cluster\", 0.5, 1, 1, {counts})"),
        );
        assert_eq!(first_error(&s).kind(), "PunchSize", "{counts}");
    }
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(w, forge_lang::error::Warning::Sheet(n) if n.kind() == "ToolNotInShop")),
        "{}",
        forge_lang::report::text(&r, &s)
    );
}

#[test]
fn forms_shape_their_walls() {
    let r = clean("formed");
    let plain = {
        let s: String = src("formed")
            .lines()
            .filter(|l| !l.starts_with("f") || l.starts_with("flat"))
            .collect::<Vec<_>>()
            .join("\n");
        run(&s)
    };
    let t = 1.5;
    let dimple = forge_sheet::FormShape::Dimple {
        d: 20.0,
        depth: 4.0,
        angle: PI / 3.0,
        die_r: 1.0,
        punch_r: 2.0,
    };
    let hole = forge_sheet::FormShape::ExtrudedHole {
        d: 8.0,
        height: 5.0,
        die_r: 1.0,
        pierce: 5.0,
    };
    let folded = 2.0 * dimple.volume_change(t).unwrap() + hole.volume_change(t).unwrap();
    let (a, b) = (
        mass(r.sheets[0].folded.as_ref().expect("built")),
        mass(plain.sheets[0].folded.as_ref().expect("built")),
    );
    near("folded", (a.0 - b.0, a.1 + b.1), folded);
    let (a, b) = (
        mass(r.sheets[0].unfolds[0].1.as_ref().expect("built")),
        mass(plain.sheets[0].unfolds[0].1.as_ref().expect("built")),
    );
    near("flat", (b.0 - a.0, a.1 + b.1), PI * 2.5 * 2.5 * t);
    let dxf = r.sheets[0].dxf().expect("a dxf");
    assert!(
        dxf.contains("\nFORM\n")
            && dxf.contains("DM20 DOWN")
            && dxf.contains("DM20 UP")
            && dxf.contains("EH8 DOWN")
    );
}

#[test]
fn form_refusals_and_violations() {
    let s = src("formed").replace("form(\"DM20\", at: holes.a", "form(\"DM9\", at: holes.a");
    assert_eq!(first_error(&s).kind(), "NoSuchTool");
    let s = src("formed").replace(
        "form(\"DM20\", at: holes.a",
        "form((\"L\", \"louvre\", 20, 5, 3), at: holes.a",
    );
    assert_eq!(first_error(&s).kind(), "FormUnbuilt");
    let s = src("formed").replace(
        "form(\"DM20\", at: holes.a",
        "form((\"D\", \"dimple\", 20, 1, 60deg, 1, 2), at: holes.a",
    );
    assert_eq!(first_error(&s).kind(), "FormTooShallow");
    // over the edge of the base: refused when built
    let s = src("formed").replace("param holes.y = 25", "param holes.y = 5");
    assert_eq!(first_error(&s).kind(), "FormNotClear");
    // a tool the shop does not have: built, a typed violation
    let s = src("formed").replace(
        "form(\"DM20\", at: holes.a",
        "form((\"D22\", \"dimple\", 22, 4, 60deg, 1, 2), at: holes.a",
    );
    let r = run(&s);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, &s));
    let kinds: Vec<String> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            forge_lang::error::Warning::Sheet(n) => Some(n.kind().to_string()),
            _ => None,
        })
        .collect();
    assert!(kinds.contains(&"FormToolNotInShop".to_string()), "{kinds:?}");
    // the shop's tool the wrong way, and near its bend
    let s = src("formed")
        .replace(
            "f2 = form(\"EH8\", at: holes.b, wall: base)",
            "f2 = form(\"EH8\", at: holes.b, wall: base, up: true)",
        )
        .replace("param holes.x2 = 58", "param holes.x2 = 70");
    let s = s.replace(
        "(\"EH8\", \"extruded_hole\", 8, 5, 1, 5)",
        "(\"EH8\", \"extruded_hole\", 8, 5, 1, 5, \"down\")",
    );
    let r = run(&s);
    let kinds: Vec<String> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            forge_lang::error::Warning::Sheet(n) => Some(n.kind().to_string()),
            _ => None,
        })
        .collect();
    assert!(
        kinds.contains(&"FormDirection".to_string()) && kinds.contains(&"FormNearBend".to_string()),
        "{kinds:?}"
    );
}
