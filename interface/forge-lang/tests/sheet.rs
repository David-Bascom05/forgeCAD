//! Sheet metal in Forge: the hand-written parts in parts/sheet against forge-sheet's closed forms
//! (folded and flat volumes, the fold/flat identity), the DXF read back, K moving only the flat,
//! anchors on folded and flat bodies, and every refusal typed with its fixes.
use forge_lang::edit::apply;
use forge_lang::error::{Error, Op, Warning};
use forge_lang::{Options, Run};
use forge_math::dmath::{self, PI};
use forge_topo::Body;

fn src(name: &str) -> String {
    let path = format!("{}/parts/sheet/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect("part file")
}

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

/// A clean run: no errors, warnings or lints, every expect met.
fn clean(name: &str) -> Run {
    let s = src(name);
    let r = run(&s);
    assert!(
        r.errors.is_empty() && r.warnings.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&r, &s)
    );
    assert!(r.expects.iter().all(|e| e.ok), "{name}");
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.problems, 0, "{name}");
    r
}

fn mass(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).expect("mass");
    (m.volume, m.err[0])
}

fn folded(r: &Run) -> (f64, f64) {
    mass(r.sheets[0].folded.as_ref().expect("built"))
}

fn flat(r: &Run) -> (f64, f64) {
    mass(r.sheets[0].unfolds[0].1.as_ref().expect("unfolded"))
}

fn near(what: &str, got: (f64, f64), want: f64) {
    assert!(
        (got.0 - want).abs() <= got.1 + 1e-9 * want.abs().max(1.0),
        "{what}: {} against {want}",
        got.0
    );
}

/// The setback on the reference skin: its radius is the inside one turning towards it.
fn setback(r: f64, t: f64, angle: f64, up: bool) -> f64 {
    let rho = if up { r } else { r + t };
    let (s, c) = dmath::sin_cos(angle);
    rho * s / (1.0 + c)
}

/// t tan(angle / 2): from one skin's virtual sharp to the other's.
fn sharp(t: f64, angle: f64) -> f64 {
    let (s, c) = dmath::sin_cos(angle);
    t * s / (1.0 + c)
}

/// A bend: its turn, the joint's length; the zone's folded volume and its flat allowance.
struct Bend {
    angle: f64,
    len: f64,
}

/// Folded and flat volumes from wall areas (to the tangent lines) and bends.
fn closed(t: f64, r: f64, k: f64, walls: f64, bends: &[Bend]) -> (f64, f64) {
    let fold: f64 = bends
        .iter()
        .map(|b| b.angle * (r * t + 0.5 * t * t) * b.len)
        .sum();
    let ba: f64 = bends.iter().map(|b| b.angle * (r + k * t) * b.len).sum();
    (t * walls + fold, t * (walls + ba))
}

/// V_fold - V_flat = sum of theta len t^2 (1/2 - K), when no cut crosses a bend.
fn identity(r: &Run, t: f64, k: f64, bends: &[Bend]) {
    let want: f64 = bends.iter().map(|b| b.angle * b.len * t * t * (0.5 - k)).sum();
    let (f, l) = (folded(r), flat(r));
    assert!(
        ((f.0 - l.0) - want).abs() <= f.1 + l.1 + 1e-9,
        "{} against {want}",
        f.0 - l.0
    );
}

#[test]
fn l_matches_the_closed_form() {
    let r = clean("l");
    let (t, rr, k) = (2.0, 2.0, 0.4);
    let s = setback(rr, t, PI / 2.0, true);
    // the flange stands inside its edge (the default): the base is t tan(45) shorter
    let walls = (40.0 - sharp(t, PI / 2.0) - s) * 30.0 + (30.0 - s) * 30.0;
    let (v, _) = closed(
        t,
        rr,
        k,
        walls,
        &[Bend {
            angle: PI / 2.0,
            len: 30.0,
        }],
    );
    near("L folded", folded(&r), v);
    let m = r.measure.as_ref().expect("measured");
    assert_eq!((m.faces, m.bodies), (10, 1));
}

#[test]
fn u_matches_the_closed_form() {
    let r = clean("u");
    let (t, rr, k) = (2.0, 2.0, 0.4);
    let (a1, a2) = (PI / 2.0, PI / 3.0);
    let s1 = setback(rr, t, a1, true);
    // the left flange goes down, its height to the outer sharp: the kernel's reference skin
    let s2 = setback(rr, t, a2, false);
    // both stand inside their edges (the default): going up the right edge moves in
    // t tan(45); going down the left one stays (its outer sharp is the kernel's own)
    let base = 40.0 - sharp(t, a1);
    let walls = (base - s1 - s2) * 30.0 + (30.0 - s1) * 30.0 + (25.0 - s2) * 30.0;
    let bends = [Bend { angle: a1, len: 30.0 }, Bend { angle: a2, len: 30.0 }];
    let (vf, vl) = closed(t, rr, k, walls, &bends);
    near("U folded", folded(&r), vf);
    near("U flat", flat(&r), vl);
    identity(&r, t, k, &bends);
    // the flat pattern's length, along the base
    let l = &r.sheets[0].layout.as_ref().expect("laid out").bends;
    let flat_len = base - s1 - s2 + (30.0 - s1) + (25.0 - s2) + l.iter().map(|b| b.allowance).sum::<f64>();
    let (lo, hi) = x_extent(r.sheets[0].unfolds[0].1.as_ref().expect("unfolded"));
    assert!(
        ((hi - lo) - flat_len).abs() < 1e-9,
        "{} against {flat_len}",
        hi - lo
    );
}

fn x_extent(b: &Body) -> (f64, f64) {
    let xs: Vec<f64> = b.vertex_ids().map(|v| b.vertex(v).point.x).collect();
    (
        xs.iter().cloned().fold(f64::INFINITY, f64::min),
        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    )
}

#[test]
fn z_matches_the_closed_form() {
    let r = clean("z");
    let (t, rr, k, w) = (1.5, 1.0, 0.33, 20.0);
    let a = PI / 2.0;
    let s1 = setback(rr, t, a, true);
    let s2 = setback(rr, t, a, false);
    // web up, its height to the outer sharp: the kernel's is the inner one, t tan(45) less;
    // the lip down to its outer sharp: the kernel's own. Both stand inside their edges (the
    // default): the base's edge moves in t tan(45); the web's tip, going down, stays.
    let web = 25.0 - sharp(t, a);
    let lip = 15.0;
    let walls = (50.0 - sharp(t, a) - s1) * w + (web - s1 - s2) * w + (lip - s2) * w;
    let bends = [Bend { angle: a, len: w }, Bend { angle: a, len: w }];
    let (vf, _) = closed(t, rr, k, walls, &bends);
    near("Z folded", folded(&r), vf);
    // the lip runs away from the base: its far edge a lip past the web's inner face (50 - t)
    let b = r.sheets[0].folded.as_ref().expect("built");
    let (_, hi) = x_extent(b);
    assert!((hi - (50.0 - t + 15.0)).abs() < 1e-9, "{hi}");
}

#[test]
fn box_matches_the_closed_form() {
    let r = clean("box");
    let (t, rr, k, h, n) = (1.0, 1.0, 0.44, 20.0, 4.0);
    let (l, w) = (80.0, 50.0);
    let a = PI / 2.0;
    let s = setback(rr, t, a, true);
    let kh = h - sharp(t, a);
    let lens = [l - 2.0 * n, w - 2.0 * n, l - 2.0 * n, w - 2.0 * n];
    // each flange stands inside its edge (the default): every edge moves in t tan(45) = t
    let base = l * w - 4.0 * n * n - lens.iter().map(|x| (s + sharp(t, a)) * x).sum::<f64>();
    let walls = base + lens.iter().map(|x| (kh - s) * x).sum::<f64>();
    let bends: Vec<Bend> = lens.iter().map(|&len| Bend { angle: a, len }).collect();
    let (vf, vl) = closed(t, rr, k, walls, &bends);
    near("box folded", folded(&r), vf);
    near("box flat", flat(&r), vl);
    identity(&r, t, k, &bends);
    // four walls, four bends, open corners: 4 + 4 strips' skins and their sides
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.bodies, 2);
}

/// The area of the band x0..x1 of the circle (cx, a), by the chord's antiderivative.
fn circle_band(cx: f64, a: f64, x0: f64, x1: f64) -> f64 {
    let f = |x: f64| {
        let u = (x - cx).clamp(-a, a);
        u * (a * a - u * u).max(0.0).sqrt() + a * a * dmath::asin(u / a)
    };
    (f(x1) - f(x0)).max(0.0)
}

#[test]
fn holes_across_the_bend_match_the_closed_form() {
    let r = clean("holes");
    // the base 40 long, its edge moved in t by the flange inside it
    let (t, rr, k, w, l, h) = (2.0, 2.0, 0.4, 60.0, 38.0, 30.0);
    let a = PI / 2.0;
    let ba = a * (rr + k * t);
    let rn = ba / a;
    let s = setback(rr, t, a, true);
    let z0 = l - s;
    let holes = [(38.2, 3.5), (35.0, 3.0), (38.2, 1.0)];
    let total = (l - s + ba + h - s) * w - holes.iter().map(|(_, a)| PI * a * a).sum::<f64>();
    let zone = ba * w
        - holes
            .iter()
            .map(|&(cx, a)| circle_band(cx, a, z0, z0 + ba))
            .sum::<f64>();
    near("holes flat", flat(&r), t * total);
    near(
        "holes folded",
        folded(&r),
        t * (total - zone) + t * zone * (rr + 0.5 * t) / rn,
    );
}

/// The spline's area by a fine polyline (its own curve, sampled).
fn spline_area(r: &Run) -> f64 {
    let cuts = &r.sheets[0].sheet.cuts;
    assert_eq!(cuts.len(), 1);
    let pts: Vec<forge_math::Point2> = cuts[0]
        .lp
        .segments
        .iter()
        .flat_map(|(s, _)| s.samples(200_000))
        .collect();
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            0.5 * (p.x * q.y - p.y * q.x)
        })
        .sum::<f64>()
        .abs()
}

#[test]
fn spline_window_matches_the_closed_form() {
    let r = clean("spline");
    let (t, rr, k) = (2.0, 2.0, 0.4);
    let s = setback(rr, t, PI / 2.0, true);
    // the flange inside its edge (the default): the base t shorter
    let walls = (40.0 - sharp(t, PI / 2.0) - s) * 30.0 + (30.0 - s) * 30.0 - spline_area(&r);
    let bends = [Bend {
        angle: PI / 2.0,
        len: 30.0,
    }];
    let (vf, vl) = closed(t, rr, k, walls, &bends);
    let tol = 1e-6;
    let (f, l) = (folded(&r), flat(&r));
    assert!((f.0 - vf).abs() <= f.1 + tol, "{} against {vf}", f.0);
    assert!((l.0 - vl).abs() <= l.1 + tol, "{} against {vl}", l.0);
    identity(&r, t, k, &bends);
}

// ---- the DXF -----------------------------------------------------------------------------------

/// Kind, layer, the rest of its pairs.
type Entity = (String, String, Vec<(i32, String)>);

fn entities(text: &str) -> Vec<Entity> {
    let lines: Vec<&str> = text.lines().collect();
    let ps: Vec<(i32, String)> = lines
        .chunks(2)
        .map(|c| (c[0].trim().parse().expect("a code"), c[1].to_string()))
        .collect();
    let start = ps
        .windows(2)
        .position(|w| w[0].1 == "SECTION" && w[1].1 == "ENTITIES")
        .expect("ENTITIES")
        + 2;
    let mut out = vec![];
    let mut i = start;
    while ps[i].1 != "ENDSEC" {
        let kind = ps[i].1.clone();
        let mut j = i + 1;
        let mut rest = vec![];
        while ps[j].0 != 0 {
            rest.push(ps[j].clone());
            j += 1;
        }
        let layer = rest.iter().find(|p| p.0 == 8).expect("a layer").1.clone();
        out.push((kind, layer, rest));
        i = j;
    }
    out
}

fn num(rest: &[(i32, String)], code: i32) -> f64 {
    rest.iter()
        .find(|p| p.0 == code)
        .expect("a value")
        .1
        .parse()
        .expect("a number")
}

#[test]
fn dxf_reads_back_as_the_flat() {
    // the holes part: lines and circles; a hole across the bend shows whole on the flat
    let r = clean("holes");
    let text = r.sheets[0].dxf().expect("a pattern");
    let ents = entities(&text);
    let count = |k: &str, l: &str| ents.iter().filter(|e| e.0 == k && e.1 == l).count();
    assert_eq!(count("LINE", "CUT"), 4);
    assert_eq!(count("CIRCLE", "CUT"), 3);
    assert_eq!(count("LINE", "BEND"), 1);
    // the area inside: outline less the holes, times t, is the flat body's volume
    let mut area = 0.0;
    for (kind, layer, rest) in &ents {
        match (kind.as_str(), layer.as_str()) {
            ("LINE", "CUT") => {
                let (x0, y0, x1, y1) = (num(rest, 10), num(rest, 20), num(rest, 11), num(rest, 21));
                area += 0.5 * (x0 * y1 - y0 * x1);
            }
            ("CIRCLE", "CUT") => area -= PI * num(rest, 40).powi(2),
            _ => {}
        }
    }
    near("DXF area times t", flat(&r), 2.0 * area);
    // the bend line in the zone's middle, the joint's length long
    let b = ents.iter().find(|e| e.1 == "BEND").expect("a bend line");
    let ba = PI / 2.0 * (2.0 + 0.4 * 2.0);
    assert!((num(&b.2, 10) - (36.0 + 0.5 * ba)).abs() < 1e-12);
    assert!(((num(&b.2, 21) - num(&b.2, 20)).abs() - 60.0).abs() < 1e-12);
    // the spline part writes its window as NURBS
    let r = clean("spline");
    let ents = entities(&r.sheets[0].dxf().expect("a pattern"));
    assert!(ents.iter().any(|e| e.0 == "SPLINE" && e.1 == "CUT"));
}

// ---- K, anchors, edits -------------------------------------------------------------------------

fn set(src: &str, param: &str, value: f64, dim: forge_lang::values::Dim) -> String {
    let op = Op::SetParam {
        param: forge_lang::error::ParamKey::parse(param).expect("a param"),
        value: forge_lang::error::Literal { value, dim },
    };
    apply(src, &op).expect("applies")
}

#[test]
fn k_moves_only_the_flat() {
    let s0 = src("u");
    let s1 = set(&s0, "k", 0.3, forge_lang::values::Dim::Ratio);
    let (a, b) = (run(&s0), run(&s1));
    assert!(a.ok() && b.ok());
    let pts = |r: &Run| {
        let body = r.sheets[0].folded.as_ref().expect("built");
        let mut v: Vec<_> = body
            .vertex_ids()
            .map(|x| {
                let p = body.vertex(x).point;
                (body.vertex(x).name, [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()])
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(pts(&a), pts(&b), "folded bodies to the bit");
    assert_ne!(flat(&a).0, flat(&b).0);
}

#[test]
fn anchors_on_folded_and_flat() {
    // sketches on the flange's top, folded and flat, and on its tip's face
    let s = format!(
        "{}\nsketch on_fold(on: up.top) {{\n  p = pt(0, 0)\n}}\nsketch on_flat(on: blank.of(up).top) {{\n  p = pt(0, 0)\n}}\nsketch on_tip(on: up.tip.face) {{\n  p = pt(0, 0)\n}}\n",
        src("spline")
    );
    let r = run(&s);
    assert!(r.ok(), "{}", forge_lang::report::text(&r, &s));
    let frame = |id: &str| {
        r.feats
            .iter()
            .find(|f| f.id == id)
            .and_then(|f| f.frame)
            .expect("a plane")
    };
    // folded: the flange stands inside x = 40, its top (reference skin) at 38 facing the base
    let f = frame("on_fold");
    assert!(
        (f.origin.x - 38.0).abs() < 1e-12 && (f.z().get().x + 1.0).abs() < 1e-12,
        "{f:?}"
    );
    // flat: lying level, facing as the unfold's plane does
    let g = frame("on_flat");
    assert!(
        g.z().get().x.abs() < 1e-12 && g.z().get().y.abs() < 1e-12,
        "{g:?}"
    );
    // the tip: at the flange's height, facing up
    let h = frame("on_tip");
    assert!(
        (h.origin.z - 30.0).abs() < 1e-12 && (h.z().get().z - 1.0).abs() < 1e-12,
        "{h:?}"
    );
}

#[test]
fn edits_rename_nothing() {
    let s0 = src("holes");
    let names = |r: &Run| {
        let b = r.sheets[0].folded.as_ref().expect("built");
        let mut v: Vec<_> = b.face_ids().map(|f| b.face(f).name).collect();
        v.sort();
        v
    };
    let a = run(&s0);
    for (p, v, d) in [
        ("up.h", 45.0, forge_lang::values::Dim::Length),
        ("holes.big", 3.0, forge_lang::values::Dim::Length),
        ("k", 0.25, forge_lang::values::Dim::Ratio),
    ] {
        let b = run(&set(&s0, p, v, d));
        assert!(b.ok(), "{p}");
        assert_eq!(names(&a), names(&b), "{p}");
    }
}

// ---- refusals ----------------------------------------------------------------------------------

fn first_error(s: &str) -> Error {
    let r = run(s);
    r.errors
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("no error in\n{s}"))
}

const L: &str = "shared t = 2
shared r = 2
shared k = ratio(0.4)
param plan.L = 40
param plan.W = 30
param up.h = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
up = flange(base.edge(plan.outline[1]), height: h, measure: \"inner\")
";

/// The fix the error offers, applied, cures it.
fn cured_by_fix(s: &str, e: &Error) {
    let fixes = e.fixes();
    assert!(!fixes.is_empty(), "{} has no fix", e.kind());
    let fixed = apply(s, &fixes[0]).expect("the fix applies");
    let r = run(&fixed);
    assert!(
        !r.errors.iter().any(|x| x.kind() == e.kind()),
        "{fixed}\n{}",
        forge_lang::report::text(&r, &fixed)
    );
}

#[test]
fn ordinary_features_are_refused_both_ways() {
    let s = format!("{L}sketch boss(on: up.top) {{\n  c = circle((10, 10), t)\n}}\nb = extrude(boss.c, t)\n");
    let e = first_error(&s);
    assert_eq!(e.kind(), "OrdinaryOnSheet");
    assert_eq!(e.fixes(), vec![Op::DeleteFeature { id: "b".into() }]);
    let s = "shared t = 2
shared r = 2
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (40, 30))
}
block = extrude(plan, t)
sketch top(on: block.end) {
  o = rect(corner: (0, 0), size: (10, 10))
}
s = sheet(top, t: t, r: r, bend: kfactor(ratio(0.4)))
";
    assert_eq!(first_error(s).kind(), "SheetOnSolid");
}

/// A base whose right edge is split at y = m, a flange off its lower part: the base keeps
/// material beside the bend's far end.
const SPLIT: &str = "shared t = 2
shared r = 2
param plan.L = 40
param plan.W = 30
param plan.m = 18
param up.h = 25
sketch plan(on: xy) {
  outline = poly([(0, 0), (L, 0), (L, m), (L, W), (0, W)])
}
base = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)))
up = flange(base.edge(plan.outline[1]), height: h, measure: \"inner\")
";

/// The split part's folded and flat volumes with `removed` taken out of the base.
fn split_closed(removed: f64, width: f64) -> (f64, f64) {
    let (t, r, k, l, w, m, h) = (2.0, 2.0, 0.4, 40.0, 30.0, 18.0, 25.0);
    let th = PI / 2.0;
    // the flange stands inside its edge (the default): the base also loses t x m beside it, and
    // the relief the t between its mold line and the old edge
    let removed = removed + t * m + width * t;
    let walls = l * w - r * m + (h - r) * m - removed;
    (
        t * walls + th * (r * t + 0.5 * t * t) * m,
        t * (walls + th * (r + k * t) * m),
    )
}

fn split_run(s: &str) -> Run {
    let r = run(&format!(
        "{s}param blank.gap = 30\nblank = unfold(base, on: offset(base.bottom, gap))\n"
    ));
    assert!(r.ok(), "{}", forge_lang::report::text(&r, s));
    r
}

#[test]
fn a_needed_relief_is_added_and_reported() {
    let r = split_run(SPLIT);
    let rel = &r.sheets[0].reliefs;
    assert_eq!(rel.len(), 1);
    assert!(rel[0].automatic && rel[0].kind == forge_sheet::ReliefKind::Rect);
    // the stand-in default: width t, depth t + R + 0.5
    assert_eq!((rel[0].width, rel[0].depth), (2.0, 4.5));
    let (vf, vl) = split_closed(2.0 * 4.5, 2.0);
    near("folded", folded(&r), vf);
    near("flat", flat(&r), vl);
    // typed data in the report, not a warning
    assert!(!r.warnings.iter().any(|w| matches!(w, Warning::Sheet(_))));
    let j = forge_lang::report::json(&r, SPLIT);
    let text = format!("{j:?}");
    assert!(text.contains("reliefs") && text.contains("automatic"), "{text}");
}

#[test]
fn reliefs_asked_for_and_refused() {
    let (w, d) = (3.0, 6.0);
    let kerf = 0.2;
    for (text, removed) in [
        ("rect(3, 6)", w * d),
        (
            "obround(3, 6)",
            w * (d - 0.5 * w) + 0.5 * PI * (0.5 * w) * (0.5 * w),
        ),
        ("tear", kerf * (2.0 + 2.0 + 0.5)),
    ] {
        let s = SPLIT.replace(
            "measure: \"inner\")",
            &format!("measure: \"inner\", relief: {text})"),
        );
        let r = split_run(&s);
        let (vf, vl) = split_closed(removed, if text == "tear" { kerf } else { w });
        near(text, folded(&r), vf);
        near(text, flat(&r), vl);
        assert!(r.sheets[0].reliefs.iter().all(|x| !x.automatic));
    }
    // none where one is needed: NeedsRelief, with a fix of each kind that cures it
    let s = SPLIT.replace("measure: \"inner\")", "measure: \"inner\", relief: no_relief)");
    let e = first_error(&s);
    assert_eq!(e.kind(), "NeedsRelief");
    let fixes = e.fixes();
    assert_eq!(fixes.len(), 3, "{fixes:?}");
    for f in &fixes {
        assert!(matches!(f, Op::SetArg { arg, .. } if arg == "relief"), "{f:?}");
        let fixed = apply(&s, f).expect("applies");
        let r = run(&fixed);
        assert!(r.ok(), "{fixed}\n{}", forge_lang::report::text(&r, &fixed));
    }
    // too shallow to pass the tangent line
    let s = SPLIT.replace("measure: \"inner\")", "measure: \"inner\", relief: rect(3, 1))");
    let e = first_error(&s);
    assert_eq!(e.kind(), "ReliefTooShallow");
    cured_by_fix(&s, &e);
}

#[test]
fn flange_positions() {
    for (pos, shift) in [("outside", 0.0), ("inside", -2.0), ("bend_outside", 2.0)] {
        let s = L.replace(
            "measure: \"inner\")",
            &format!("measure: \"inner\", position: \"{pos}\")"),
        );
        let r = run(&s);
        assert!(r.ok(), "{pos}: {:?}", r.errors);
        let (t, rr, w) = (2.0, 2.0, 30.0);
        let walls = (40.0 + shift - rr) * w + (30.0 - rr) * w;
        near(pos, folded(&r), t * walls + PI / 2.0 * (rr * t + 0.5 * t * t) * w);
    }
}

/// A plain box: four flanges off a rectangle, its corners left to the builder.
const BOX: &str = "shared t = 1.5
shared r = 1
shared H = 20
param plan.L = 60
param plan.W = 40
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)))
front = flange(base.edge(plan.outline[0]), height: H, measure: \"inner\")
east = flange(base.edge(plan.outline[1]), height: H, measure: \"inner\")
back = flange(base.edge(plan.outline[2]), height: H, measure: \"inner\")
west = flange(base.edge(plan.outline[3]), height: H, measure: \"inner\")
";

/// The box with its walls leaning out 10 degrees: they meet only at the corners' reliefs.
fn splayed() -> String {
    BOX.replace("measure: \"inner\")", "measure: \"inner\", angle: 80deg)")
}

#[test]
fn box_corners() {
    // upright, each wall's end would butt on its neighbour's inner face: the builder trims
    // the later wall back t and the shop's corner gap, reported with the reliefs
    let r = run(BOX);
    assert!(r.ok(), "{:?}", r.errors);
    let trims: Vec<_> = r.sheets[0]
        .corners
        .iter()
        .filter(|c| matches!(c.kind, forge_sheet::CornerKind::Open { .. }))
        .collect();
    assert_eq!(trims.len(), 4, "{:?}", r.sheets[0].corners);
    assert!(
        trims
            .iter()
            .all(|c| c.automatic && (c.size - (1.5 + 0.4)).abs() < 1e-12)
    );
    assert_eq!(r.measure.as_ref().map(|m| m.problems), Some(0));
    // a corner given its own gap keeps it
    let r = run(&format!("{BOX}fe = corner_relief(front, east, open_corner(1))\n"));
    assert!(r.ok(), "{:?}", r.errors);
    assert!(
        r.sheets[0]
            .corners
            .iter()
            .any(|c| !c.automatic && (c.size - 2.5).abs() < 1e-12),
        "{:?}",
        r.sheets[0].corners
    );
    // splayed out, the walls never meet: the reliefs alone
    let r = run(&splayed());
    assert!(r.ok(), "{:?}", r.errors);
    let c = &r.sheets[0].corners;
    assert_eq!(c.len(), 4);
    assert!(
        c.iter()
            .all(|x| x.automatic && matches!(x.kind, forge_sheet::CornerKind::Circular { .. }))
    );
    assert_eq!(r.measure.as_ref().map(|m| m.problems), Some(0));
    // one corner made square, the sheet's default square, closed refused
    let s = format!(
        "{}param fe.h = 4\nfe = corner_relief(front, east, square_relief(h))\n",
        splayed()
    );
    let r = run(&s);
    assert!(r.ok(), "{:?}", r.errors);
    let squares = r.sheets[0]
        .corners
        .iter()
        .filter(|x| matches!(x.kind, forge_sheet::CornerKind::Square { .. }))
        .count();
    assert_eq!(squares, 1);
    let s = splayed().replace(
        "bend: kfactor(ratio(0.4)))",
        "bend: kfactor(ratio(0.4)), corner: closed_corner)",
    );
    assert_eq!(first_error(&s).kind(), "CornerUnbuilt");
    // flanges that do not meet at a corner
    let s = format!(
        "{}fb = corner_relief(front, back, circular_relief())\n",
        splayed()
    );
    assert_eq!(first_error(&s).kind(), "NotACorner");
}

#[test]
fn a_name_hiding_a_built_in_gets_a_rename_fix() {
    let s = "shared flange = 3
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (flange, flange))
}
block = extrude(plan, flange)
expect volume = 27 +- 0.1%
";
    let r = forge_lang::run(
        s,
        Options {
            build: false,
            ..Options::default()
        },
        Default::default(),
    )
    .0;
    let e = r
        .errors
        .iter()
        .find(|e| e.kind() == "ShadowsBuiltin")
        .expect("refused");
    let fixes = e.fixes();
    assert_eq!(
        fixes,
        vec![Op::Rename {
            from: "flange".into(),
            to: "flange_1".into()
        }]
    );
    let fixed = apply(s, &fixes[0]).expect("applies");
    assert!(fixed.contains("shared flange_1 = 3") && fixed.contains("size: (flange_1, flange_1)"));
    let r = run(&fixed);
    assert!(
        r.ok() && r.warnings.is_empty(),
        "{fixed}\n{}",
        forge_lang::report::text(&r, &fixed)
    );
}

#[test]
fn kernel_refusals_come_typed_with_fixes() {
    // a flange shorter than its setback: the fix makes it a thickness long past it
    let s = L.replace("param up.h = 30", "param up.h = 1");
    let e = first_error(&s);
    assert_eq!(e.kind(), "WallTooShort");
    cured_by_fix(&s, &e);
    // K out of range
    let s = L.replace("shared k = ratio(0.4)", "shared k = ratio(1.5)");
    let e = first_error(&s);
    assert_eq!(e.kind(), "BendRule");
    cured_by_fix(&s, &e);
    // an angle of 180
    let s = L
        .replace("measure: \"inner\")", "angle: 180deg, measure: \"inner\")")
        .replace("param up.h = 30", "param up.h = 30\nparam up.a = 180deg")
        .replace("angle: 180deg", "angle: a");
    let e = first_error(&s);
    assert_eq!(e.kind(), "BendAngle");
    cured_by_fix(&s, &e);
    // the same edge twice
    let s = format!("{L}again = flange(base.edge(plan.outline[1]), height: t)\n");
    assert_eq!(first_error(&s).kind(), "EdgeTaken");
    // flanges folding into each other
    let s = "shared t = 2
shared r = 2
param plan.L = 40
param plan.W = 30
param a.h = 60
param a.a = 150deg
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)))
a = flange(base.edge(plan.outline[1]), height: h, angle: a)
b = flange(base.edge(plan.outline[3]), height: a.h, angle: a.a)
";
    let r = run(s);
    let e = r.errors.first().expect("a clash");
    assert_eq!(e.kind(), "FoldedClash", "{e:?}");
    let j = e.json(s);
    assert!(format!("{j:?}").contains("depth"));
}

#[test]
fn language_refusals() {
    // an edge that is not the base outline's
    let s = format!(
        "{L}sketch c(on: base.top) {{\n  h = circle((10, 10), t)\n}}\nf = flange(base.edge(c.h), height: t)\n"
    );
    assert_eq!(first_error(&s).kind(), "NotAnEdge");
    // a cutout off its wall
    let s = format!(
        "{L}sketch c(on: offset(base.top, up.h)) {{\n  h = circle((10, 10), t)\n}}\nhole1 = cutout(c, wall: base)\n"
    );
    assert_eq!(first_error(&s).kind(), "CutOffWall");
    // a sheet feature after an unfold: the fix moves the unfold to the end
    let s = format!(
        "{L}blank = unfold(base)\nsketch c(on: base.top) {{\n  h = circle((10, 10), t)\n}}\nhole1 = cutout(c, wall: base)\n"
    );
    let e = first_error(&s);
    assert_eq!(e.kind(), "AfterUnfold");
    cured_by_fix(&s, &e);
    // a base has no bend
    let s = format!("{L}sketch c(on: base.bend) {{\n  p = pt(0, 0)\n}}\n");
    assert_eq!(first_error(&s).kind(), "NoSuchRole");
}

#[test]
fn shop_violations_are_typed_warnings_with_fixes() {
    let s = L.replace(
        "base = sheet(plan, t: t, r: r, bend: kfactor(k))",
        "shared steel = stock(t: 2, bend: kfactor(ratio(0.4)), radii: [3, 4], min_flange: per_t(ratio(18)))
shared acme = shop([steel])
base = sheet(plan, t: t, r: r, bend: kfactor(k), shop: acme)",
    );
    let r = run(&s);
    assert!(r.ok(), "{}", forge_lang::report::text(&r, &s));
    let kinds: Vec<&str> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Sheet(n) => Some(n.kind()),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, ["RadiusNotTooled", "FlangeTooShort"], "{:?}", r.warnings);
    // each fix, applied, cures its own
    for w in &r.warnings {
        let Warning::Sheet(n) = w else { continue };
        let fixed = apply(&s, &n.fixes[0]).expect("applies");
        let r2 = run(&fixed);
        assert!(r2.ok());
        assert!(
            !r2.warnings
                .iter()
                .any(|x| matches!(x, Warning::Sheet(m) if m.kind() == n.kind())),
            "{fixed}"
        );
    }
    // a thickness the shop does not stock, the radius and rule then given
    let s2 = s.replace("shared t = 2", "shared t = 1.5");
    let r = run(&s2);
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(w, Warning::Sheet(n) if n.kind() == "ThicknessNotStocked"))
    );
    // from the shop alone: radius and rule are the stock's
    let s3 = s.replace(
        "base = sheet(plan, t: t, r: r, bend: kfactor(k), shop: acme)",
        "base = sheet(plan, t: t, shop: acme)",
    );
    let r = run(&s3);
    assert!(r.ok(), "{}", forge_lang::report::text(&r, &s3));
    assert_eq!(r.sheets[0].sheet.gauge.radius, 3.0);
}

#[test]
fn a_flange_takes_its_own_radius() {
    let a = run(&L.replace("shared r = 2", "shared r = 4"));
    let b = run(&L
        .replace("param up.h = 30", "param up.h = 30\nparam up.rr = 4")
        .replace("measure: \"inner\")", "measure: \"inner\", r: rr)"));
    assert!(a.ok() && b.ok(), "{:?}", b.errors);
    let v = |r: &Run| format!("{:?}", r.sheets[0].folded.as_ref().expect("built").vertices);
    assert_eq!(v(&a), v(&b));
    let up = b.feats.iter().find(|f| f.id == "up").expect("up");
    assert!(up.dims.iter().any(|d| d.what == "radius" && d.num.v == 4.0));
}

#[test]
fn the_reference_example_builds_clean() {
    let r = forge_lang::REFERENCE;
    let at = r.find("## Sheet metal").expect("a sheet section");
    let open = at + r[at..].find("```\n").expect("an example") + 4;
    let s = &r[open..open + r[open..].find("```").expect("closed")];
    let run = run(s);
    assert!(
        run.errors.is_empty() && run.warnings.is_empty(),
        "{}",
        forge_lang::report::text(&run, s)
    );
    assert_eq!(run.measure.as_ref().map(|m| (m.bodies, m.problems)), Some((2, 0)));
}

/// A notch with a flange up off each side: folded they stand apart, flat they reach into each
/// other.
const NOTCH: &str = "shared t = 2
shared r = 2
shared k = ratio(0.4)
param plan.L = 100
param plan.W = 60
param plan.a = 60
param plan.b = 80
param plan.d = 30
param right.h = 15
param left.h = 15
sketch plan(on: xy) {
  outline = poly([(0, 0), (L, 0), (L, W), (b, W), (b, W - d), (a, W - d), (a, W), (0, W)])
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
right = flange(base.edge(plan.outline[3]), height: h, measure: \"inner\")
left = flange(base.edge(plan.outline[5]), height: h, measure: \"inner\")
";

#[test]
fn a_flat_overlap_warns_on_the_part_and_refuses_the_blank() {
    let r = run(NOTCH);
    assert!(r.ok(), "{:?}", r.errors);
    let w: Vec<_> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Sheet(n) if n.kind() == "FlatOverlap" => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(w.len(), 1, "{:?}", r.warnings);
    // the exact overlap: each flat reaches -s + BA + (h - s) into the 20 wide notch, 30 deep
    let (sb, ba) = (2.0, PI / 2.0 * (2.0 + 0.4 * 2.0));
    // the notch 20 wide, each side moved t out of it by its flange standing inside
    let want = (2.0 * (-sb + ba + 15.0 - sb) - 24.0) * 30.0;
    let total = w[0]
        .data()
        .iter()
        .find(|x| x.0 == "total")
        .and_then(|x| x.1.num())
        .expect("a total");
    assert!((total - want).abs() < 1e-9, "{total} against {want}");
    // the blank and its DXF are refused, typed
    assert!(matches!(
        r.sheets[0].dxf(),
        Err(forge_sheet::SheetError::FlatOverlap { .. })
    ));
    let s = format!("{NOTCH}param blank.gap = 40\nblank = unfold(base, on: offset(base.bottom, gap))\n");
    let e = first_error(&s);
    assert_eq!(e.kind(), "FlatOverlap");
    // each fix, a shorter flange, clears it
    assert_eq!(w[0].fixes.len(), 2, "{:?}", w[0].fixes);
    for f in &w[0].fixes {
        assert!(matches!(f, Op::SetParam { .. }));
        let fixed = apply(&s, f).expect("applies");
        let r = run(&fixed);
        // the overlap gone; the flats still sit on the notch's bottom edge, a touch of its own
        assert!(
            r.errors.iter().all(|e| e.kind() == "FlatTouch"),
            "{fixed}\n{}",
            forge_lang::report::text(&r, &fixed)
        );
        assert!(
            !r.warnings
                .iter()
                .any(|w| matches!(w, Warning::Sheet(n) if n.kind() == "FlatOverlap"))
        );
    }
    // the near miss: a little shorter than the reach that touches
    let h = (24.0 / 2.0 + 2.0 * sb - ba) - 1e-3;
    let near_miss = NOTCH
        .replace("param right.h = 15", &format!("param right.h = {h}"))
        .replace("param left.h = 15", &format!("param left.h = {h}"));
    let r = run(&near_miss);
    let sheet: Vec<_> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Sheet(n) => Some(n.kind()),
            _ => None,
        })
        .collect();
    // no overlap; both flats lie on the notch's bottom edge: a touch, warned on the part
    assert!(r.ok() && sheet == ["FlatTouch"], "{:?}", r.warnings);
    let blank =
        format!("{near_miss}param blank.gap = 40\nblank = unfold(base, on: offset(base.bottom, gap))\n");
    assert_eq!(first_error(&blank).kind(), "FlatTouch");
}

/// An L with a shop: holes 2.5 t + R from a bend, 2 t from the edge, t across.
const SHOPPED: &str = "shared t = 2
shared r = 2
shared k = ratio(0.4)
shared steel = stock(t: 2, bend: kfactor(ratio(0.4)), radii: [2], hole_to_bend: per_t(ratio(2.5), per_r: ratio(1)), hole_to_edge: per_t(ratio(2)), min_hole: per_t(ratio(1)), hardness: ratio(70))
shared acme = shop([steel])
param plan.L = 60
param plan.W = 40
param up.h = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k), shop: acme)
up = flange(base.edge(plan.outline[1]), height: h, measure: \"inner\")
";

fn notes<'a>(r: &'a Run, kind: &str) -> Vec<&'a forge_lang::sheet::SheetNote> {
    r.warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Sheet(n) if n.kind() == kind => Some(&**n),
            _ => None,
        })
        .collect()
}

#[test]
fn holes_against_the_shop_with_fixes_that_move_or_resize() {
    let s = format!(
        "{SHOPPED}param hs.x = 50
param hs.y = 20
param hs.d = 3
param hs.ex = 20
param hs.ey = 5
param hs.er = 2
param hs.tiny = 0.8
param hs.ty = 25
sketch hs(on: base.top) {{
  near_bend = circle((x, y), d)
  near_edge = circle((ex, ey), er)
  small = circle((ex, ty), tiny)
}}
holes = cutout(hs, wall: base)
"
    );
    let r = run(&s);
    assert!(r.ok(), "{}", forge_lang::report::text(&r, &s));
    for (kind, want) in [("HoleNearBend", 1), ("HoleNearEdge", 1), ("HoleTooSmall", 1)] {
        let n = notes(&r, kind);
        assert_eq!(n.len(), want, "{kind}: {:?}", r.warnings);
        assert_eq!(n[0].id, "holes");
        assert!(!n[0].fixes.is_empty(), "{kind}");
        // every fix cures its own
        for f in &n[0].fixes {
            let fixed = apply(&s, f).expect("applies");
            let r2 = run(&fixed);
            assert!(r2.ok());
            let left = notes(&r2, kind).len();
            assert!(left < want, "{kind}: {f:?} leaves {left}");
        }
    }
    // the move along the way out: x 50 -> 46 (the flange inside its edge puts the tangent line
    // at 56: 3 from it, 7 wanted)
    let n = notes(&r, "HoleNearBend");
    assert!(
        n[0].fixes
            .iter()
            .any(|f| matches!(f, Op::SetParam { param, value }
        if param.to_string() == "hs.x" && (value.value - 46.0).abs() < 1e-9)),
        "{:?}",
        n[0].fixes
    );
}

#[test]
fn pem_hardware() {
    let s = format!(
        "{SHOPPED}param ps.x = 20
param ps.y = 4
sketch ps(on: base.top) {{
  c = pt(x, y)
}}
nut = pem(\"nut\", \"M3\", at: ps.c, wall: base)
"
    );
    let r = run(&s);
    assert!(r.ok(), "{}", forge_lang::report::text(&r, &s));
    // its mounting hole, from the (placeholder) catalogue
    let plain = run(SHOPPED);
    let hole = PI * 0.25 * 4.22 * 4.22 * 2.0;
    assert!(((folded(&plain).0 - folded(&r).0) - hole).abs() < 1e-7);
    assert_eq!(notes(&r, "UnverifiedHardware").len(), 1);
    let n = notes(&r, "HardwareNearEdge");
    assert_eq!(n.len(), 1, "{:?}", r.warnings);
    let fixed = apply(&s, &n[0].fixes[0]).expect("a move");
    assert!(notes(&run(&fixed), "HardwareNearEdge").is_empty(), "{fixed}");
    // a cited row in the shop: no placeholder warning; a size it lacks is refused
    let cited = s.replace(
        "shared acme = shop([steel])",
        "shared m3 = hardware(kind: \"nut\", size: \"M3\", sheet: (1, 6), hole: 4.22, edge: 4.8, max_hardness: ratio(80), code: \"CLS-M3-1\", source: \"(cited in the test only)\")\nshared acme = shop([steel], hardware: [m3])",
    );
    let r = run(&cited);
    assert!(r.ok(), "{:?}", r.errors);
    assert!(notes(&r, "UnverifiedHardware").is_empty());
    let e = first_error(&cited.replace("\"M3\", at:", "\"M4\", at:"));
    assert_eq!(e.kind(), "NoSuchHardware");
    // too hard a sheet
    let hard = s.replace("hardness: ratio(70)", "hardness: ratio(90)");
    assert_eq!(notes(&run(&hard), "SheetTooHard").len(), 1);
}

#[test]
fn the_shops_relief_rule() {
    let shop = |rule: &str| {
        SPLIT.replace(
            "base = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)))",
            &format!(
                "shared steel = stock(t: 2, bend: kfactor(ratio(0.4)), radii: [2], relief: relief_rule({rule}))\nshared acme = shop([steel])\nbase = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)), shop: acme)"
            ),
        )
    };
    // the rule's own relief: obround, 3 wide, up to the bend
    let s = shop("shape: \"obround\", width: 3, from: \"up_to_bend\"");
    let r = split_run(&s);
    let rel = &r.sheets[0].reliefs;
    assert_eq!(rel.len(), 1);
    assert!(rel[0].automatic && rel[0].kind == forge_sheet::ReliefKind::Obround);
    // its straight sides end on the tangent line: depth s + w/2 from the mold line
    assert!((rel[0].depth - (2.0 + 1.5)).abs() < 1e-12, "{}", rel[0].depth);
    let (vf, _) = split_closed(3.0 * 2.0 + 0.5 * PI * 1.5 * 1.5, 3.0);
    near("up to bend", folded(&r), vf);
    // an override smaller than the rule: a violation, its fixes the rule's sizes
    let s = shop("width: per_t(ratio(1.5))")
        .replace("measure: \"inner\")", "measure: \"inner\", relief: rect(2, 4.5))");
    let r = run(&s);
    assert!(r.ok(), "{:?}", r.errors);
    // (both ends of the bend ask for it)
    let n = notes(&r, "ReliefUndersized");
    assert_eq!(n.len(), 2, "{:?}", r.warnings);
    let fixed = apply(&s, &n[0].fixes[0]).expect("applies");
    assert!(notes(&run(&fixed), "ReliefUndersized").is_empty(), "{fixed}");
    // extend is held as data and refused typed
    assert_eq!(first_error(&shop("extend: true")).kind(), "ReliefExtendUnbuilt");
    // corners: the rule's trim to bend, an override tear
    let b = splayed().replace(
        "base = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)))",
        "shared steel = stock(t: 1.5, bend: kfactor(ratio(0.4)), radii: [1], relief: relief_rule(corner: \"trim_to_bend\"))\nshared acme = shop([steel])\nbase = sheet(plan, t: t, r: r, bend: kfactor(ratio(0.4)), shop: acme)",
    );
    let r = run(&b);
    assert!(r.ok(), "{:?}", r.errors);
    assert!(
        r.sheets[0]
            .corners
            .iter()
            .all(|c| c.kind == forge_sheet::CornerKind::TrimToBend)
    );
    let r = run(&format!("{b}fe = corner_relief(front, east, tear_corner)\n"));
    let kinds: Vec<_> = r.sheets[0].corners.iter().map(|c| c.kind).collect();
    // a tear only slits the corner: where the bends' zones meet there it is refused, typed
    // (a clash, or the slit crossing a tangent line at a slant)
    assert!(
        r.ok() || r.errors.iter().all(|e| matches!(e, Error::Sheet(_))),
        "{:?} {kinds:?}",
        r.errors
    );
}

/// A plate with a shop that knows an M4 stud and an M6 nut (PEM's numbers), holes 5.84 from a
/// bend's tangent line.
const PEM_SHOP: &str = "shared t = 1.52
shared hw = [
  hardware(kind: \"nut\", size: \"M6\", sheet: (1.22, 12.7), hole: 8.75, edge: 8.6, max_hardness: ratio(80), code: \"S-M6-0\", source: \"PEM CL bulletin\"),
  hardware(kind: \"stud\", size: \"M4\", sheet: (1.0, 12.7), hole: 4.0, edge: 7.2, max_hardness: ratio(80), code: \"FH-M4-12\", source: \"PEM FH bulletin\")]
shared steel = stock(t: t, bend: kfactor(ratio(0.4)), radii: [1.6], hole_to_bend: per_t(ratio(0), plus: 5.84), hole_to_edge: per_t(ratio(0), plus: 0.56), hardness: ratio(57))
shared acme = shop([steel], hardware: hw)
";

#[test]
fn a_hole_drawn_in_the_base_sketch_is_checked_with_a_fix() {
    // a hole in the plan itself, 0.58 from the bend's tangent line: checked as a cutout is, and
    // its fix (moving it in the plan) cures it
    let s = format!(
        "{PEM_SHOP}shared hy = 32.5
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (50, 40))
  a = circle((25, hy), 2.3)
}}
plate = sheet(plan, t: t, r: 1.6, shop: acme)
up = flange(plate.edge(plan.outline[2]), height: 20)
"
    );
    let r = run(&s);
    assert!(r.ok(), "{:?}", r.errors);
    let n = notes(&r, "HoleNearBend");
    assert_eq!(n.len(), 1, "{:?}", r.warnings);
    assert_eq!(n[0].id, "plate");
    let fixed = apply(&s, &n[0].fixes[0]).expect("applies");
    assert!(notes(&run(&fixed), "HoleNearBend").is_empty(), "{fixed}");
}

#[test]
fn hardware_into_or_near_another_hole() {
    // a stud whose hole runs 0.5 into a slot, a nut 6.5 from a hole's edge (it needs 8.6): each
    // typed on its hardware, each fix moving the hardware clear
    let s = format!(
        "{PEM_SHOP}param plan.sy = 30
param plan.nx = 15
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (80, 60))
  p = pt(40, sy)
  q = pt(nx, 45)
}}
plate = sheet(plan, t: t, r: 1.6, shop: acme)
sketch c(on: plate.top) {{
  slot_r = rect(corner: (25, 31.5), size: (30, 7))
  hl = circle((23, 45), 1.5)
}}
cuts = cutout(c, wall: plate)
stud = pem(\"stud\", \"M4\", at: plan.p, wall: plate)
nut = pem(\"nut\", \"M6\", at: plan.q, wall: plate)
"
    );
    let r = run(&s);
    assert!(r.ok(), "{:?}", r.errors);
    let over = notes(&r, "HardwareOverlapsHole");
    assert_eq!(over.len(), 1, "{:?}", r.warnings);
    assert_eq!(over[0].id, "stud");
    let near = notes(&r, "HardwareNearHole");
    assert_eq!(near.len(), 1, "{:?}", r.warnings);
    assert_eq!(near[0].id, "nut");
    for (kind, n) in [("HardwareOverlapsHole", over[0]), ("HardwareNearHole", near[0])] {
        let fixed = apply(&s, &n.fixes[0]).expect("applies");
        let r = run(&fixed);
        assert!(
            notes(&r, kind).is_empty() && notes(&r, "HardwareNearHole").iter().all(|x| x.id != n.id),
            "{fixed}"
        );
    }
}

/// SendCutSend's process limits (rack SPEC section 3) in a shop.
const SCS_LIMITS: &str = "shared t = 1.88
shared steel = stock(t: t, bend: kfactor(ratio(0.4)), radii: [1.6], hardness: ratio(57))
shared scs = shop([steel], hardware: [hardware(kind: \"nut\", size: \"M6\", sheet: (1.22, 12.7), hole: 8.75, edge: 8.6, max_hardness: ratio(80), code: \"S-M6-0\", source: \"PEM CL bulletin\")],
  max_flat: (762, 1117.6), max_flat_coated: (762, 914.4), max_bend_length: 1117.6,
  max_box_flange: 76.2, box_flange_when: \"hardware\", bend_angles: (0deg, 130deg))
";

#[test]
fn the_shops_process_limits() {
    // a pan 600 x 950 with flanges 80 round it and a nut: too long to powder coat, flanges too
    // tall for a box with hardware; each flange's fix lowers it to the limit
    let s = format!(
        "{SCS_LIMITS}param plan.L = 950
param plan.W = 600
param pan.h = 80
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (W, L))
  p = pt(300, 400)
}}
base = sheet(plan, t: t, r: 1.6, shop: scs, finish: \"powder_coat\")
front = flange(base.edge(plan.outline[0]), height: h)
right = flange(base.edge(plan.outline[1]), height: h)
back = flange(base.edge(plan.outline[2]), height: h)
left = flange(base.edge(plan.outline[3]), height: h)
nut = pem(\"nut\", \"M6\", at: plan.p, wall: base)
"
    );
    let r = run(&s);
    assert!(r.ok(), "{:?}", r.errors);
    assert_eq!(notes(&r, "FlatTooLarge").len(), 1, "{:?}", r.warnings);
    let tall = notes(&r, "BoxFlangeTooTall");
    assert_eq!(tall.len(), 4, "{:?}", r.warnings);
    let fixed = apply(&s, &tall[0].fixes[0]).expect("applies");
    assert!(notes(&run(&fixed), "BoxFlangeTooTall").is_empty(), "{fixed}");
    // raw, the same blank fits the brake; the flanges still too tall
    let raw = s.replace(", finish: \"powder_coat\"", "");
    assert!(notes(&run(&raw), "FlatTooLarge").is_empty());
    // a bend past 130 degrees: typed, the fix brings it to 130
    let s = format!(
        "{SCS_LIMITS}sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (100, 60))
}}
base = sheet(plan, t: t, r: 1.6, shop: scs)
param up.a = 140deg
up = flange(base.edge(plan.outline[1]), height: 20, angle: a)
"
    );
    let r = run(&s);
    let n = notes(&r, "BendAngleOutOfRange");
    assert_eq!(n.len(), 1, "{:?} {:?}", r.errors, r.warnings);
    let fixed = apply(&s, &n[0].fixes[0]).expect("applies");
    assert!(notes(&run(&fixed), "BendAngleOutOfRange").is_empty(), "{fixed}");
}

#[test]
fn a_flat_touching_itself_gets_a_slot_along_the_touch() {
    // a rack strip under an ear: the side wall's flat lies on the ear's top edge. The fix cuts a
    // slot along that edge into the base (a relief along it), before the unfold, and the part
    // then builds
    let s = "shared t = 1.5
shared r = 1.5
shared k = ratio(0.42)
shared W = 440
shared D = 300
shared E = 21.3
shared a = 20
shared H = 42
sketch plan(on: xy) {
  outline = poly([(-E, 0), (W + E, 0), (W + E, a), (W, a), (W, D), (0, D), (0, a), (-E, a)])
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
front = flange(base.edge(plan.outline[0]), height: H)
right = flange(base.edge(plan.outline[3]), height: H)
param flat.gap = 80
flat = unfold(base, on: offset(base.bottom, gap))
";
    let e = first_error(s);
    assert_eq!(e.kind(), "FlatTouch", "{e:?}");
    let add = e
        .fixes()
        .into_iter()
        .find(|f| matches!(f, Op::AddFeature { .. }))
        .expect("a slot along the touch");
    let fixed = apply(s, &add).expect("applies");
    let r = run(&fixed);
    assert!(r.ok(), "{fixed}\n{}", forge_lang::report::text(&r, &fixed));
    assert!(
        !r.warnings
            .iter()
            .any(|w| matches!(w, Warning::Sheet(n) if n.kind().ends_with("Touch")))
    );
}
