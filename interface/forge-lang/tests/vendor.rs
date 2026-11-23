//! Vendor profiles in Forge: a sheet takes its vendor's radius and K for its stock (so the flat
//! develops by them), a vendor shop at the part level, the refusals with their options and fixes,
//! a project's own profile, inches (`0.063in`), and the DFM report: each rule just inside and just outside its
//! limit, with where it is.
use forge_lang::dfm;
use forge_lang::edit::apply;
use forge_lang::error::{Error, Op};
use forge_lang::{Options, Run};

const IN: f64 = 25.4;

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

/// A 80 x 50 plate in `vendor`'s `material`, `t` thick, a flange up off its right edge and a
/// round hole in the base.
fn part(vendor: &str, material: &str, t: &str, hole_x: f64) -> String {
    format!(
        r#"shared t = {t}
param plan.L = 80
param plan.W = 50
param up.h = 20
param hs.x = {hole_x}
param hs.y = 25
param hs.r = 2
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (L, W))
}}
base = sheet(plan, t: t, vendor: "{vendor}", material: "{material}")
up = flange(base.edge(plan.outline[1]), height: h)
sketch hs(on: base.top) {{
  c = circle((x, y), r)
}}
vent = cutout(hs.c, wall: base)
"#
    )
}

fn errors(r: &Run) -> Vec<&'static str> {
    r.errors.iter().map(Error::kind).collect()
}

#[test]
fn a_sheet_bends_by_its_vendors_stock() {
    let r = run(&part("sendcutsend", "5052-H32 aluminum", "0.063in", 40.0));
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    let sh = &r.sheets[0];
    // the calculator's row for 0.063 in 5052: radius 0.035 in, K 0.42
    assert_eq!(sh.sheet.gauge.thickness, 0.063 * IN);
    assert_eq!(sh.sheet.gauge.radius, 0.035 * IN);
    assert_eq!(sh.sheet.gauge.rule, forge_sheet::BendRule::K(0.42));
    // so the flat develops by them: BA = angle (R + K t)
    let b = &sh.layout.as_ref().unwrap().bends[0];
    let want = std::f64::consts::FRAC_PI_2 * (0.035 * IN + 0.42 * 0.063 * IN);
    assert!((b.allowance - want).abs() < 1e-12, "{} {want}", b.allowance);
    // mass from the vendor material's library name; the drawing's note names the vendor's own
    assert_eq!(sh.material.as_deref(), Some("5052-H32"));
    assert!(sh.note().contains("5052-H32 ALUMINUM"), "{}", sh.note());
    let v = sh.vendor.as_ref().unwrap();
    assert_eq!(
        (v.id.as_str(), v.taken.as_deref()),
        ("sendcutsend", Some("2026-11-22"))
    );
}

#[test]
fn a_vendor_shop_at_the_part_level() {
    let src = r#"shared fab = vendor("fabworks", material: "304")
shared t = 0.06in
param plan.L = 60
param plan.W = 40
param up.h = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, shop: fab)
up = flange(base.edge(plan.outline[1]), height: h)
"#;
    let r = run(src);
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    let g = &r.sheets[0].sheet.gauge;
    // Fabworks' 304-2B 0.060 in: inside radius 0.074 in, K 0.420
    assert_eq!(g.radius, 0.074 * IN);
    assert_eq!(g.rule, forge_sheet::BendRule::K(0.42));
}

#[test]
fn what_the_vendor_does_not_offer_is_refused_with_the_options() {
    // a thickness not stocked: the stocked ones listed, a fix to the nearest that builds
    let src = part("sendcutsend", "5052", "1.5mm", 40.0);
    let r = run(&src);
    assert_eq!(errors(&r), ["VendorThickness"]);
    let e = &r.errors[0];
    assert!(e.text().contains("1.6002 mm (0.063 in, 14 ga)"), "{}", e.text());
    let fix = e.fixes();
    assert!(
        matches!(&fix[..], [Op::SetParam { param, .. }] if param.name == "t"),
        "{fix:?}"
    );
    let fixed = apply(&src, &fix[0]).unwrap();
    assert!(run(&fixed).errors.is_empty());
    // a material it does not offer
    let r = run(&part("sendcutsend", "5025 aluminium", "0.063in", 40.0));
    assert_eq!(errors(&r), ["VendorMaterial"]);
    let fix = r.errors[0].fixes();
    assert!(
        matches!(&fix[0], Op::SetArg { arg, text, .. } if arg == "material" && text == "\"5052-H32 aluminum\"")
    );
    // a vendor not found
    let r = run(&part("sendcutsnd", "5052", "0.063in", 40.0));
    assert_eq!(errors(&r), ["UnknownVendor"]);
    assert!(matches!(&r.errors[0].fixes()[0], Op::SetArg { text, .. } if text == "\"sendcutsend\""));
}

#[test]
fn a_projects_own_profile_is_found_first() {
    let dir = std::env::temp_dir().join(format!("fg-vendor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("vendors")).unwrap();
    std::fs::write(
        dir.join("vendors/shop.toml"),
        r#"format = 1
id = "sendcutsend"
name = "Our own"
units = "mm"
[[material]]
name = "5052"
[[material.stock]]
t = 2
radius = [3]
k = 0.5
"#,
    )
    .unwrap();
    let opts = Options {
        dir: Some(dir.clone()),
        ..Options::default()
    };
    let r = forge_lang::run(
        &part("sendcutsend", "5052", "2mm", 40.0),
        opts,
        Default::default(),
    )
    .0;
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    assert_eq!(r.sheets[0].sheet.gauge.radius, 3.0);
    assert_eq!(r.sheets[0].vendor.as_ref().unwrap().name, "Our own");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The DFM report of a source.
fn dfm(src: &str) -> dfm::Report {
    dfm::report(src, None, None)
}

fn rule<'a>(r: &'a dfm::Report, rule: &str) -> Vec<&'a dfm::Item> {
    r.sheets
        .iter()
        .flat_map(|s| &s.items)
        .filter(|i| i.rule == rule)
        .collect()
}

#[test]
fn dfm_hole_to_bend_just_inside_and_just_outside() {
    // SendCutSend: half the die width (0.472 in) from the bend's centre, for 0.063 in 5052
    let limit = 0.236 * IN;
    let probe = dfm(&part("sendcutsend", "5052", "0.063in", 75.0));
    let near = rule(&probe, "HoleNearBend");
    assert_eq!(near.len(), 1, "{}", probe.text());
    let i = near[0];
    assert!((i.limit.unwrap() - limit).abs() < 1e-9);
    assert_eq!(i.sense, Some("min"));
    // where it is: the cutout's line, its faces, its place on the flat and folded
    assert_eq!((i.feature.as_str(), i.line), ("vent", 16));
    assert!(!i.faces.is_empty() && i.anchor.as_deref() == Some("vent.faces"));
    assert_eq!(i.flat.len(), 1);
    let at = i.at.unwrap();
    assert!((at.x - 75.0).abs() < 1e-9 && (at.y - 25.0).abs() < 1e-9, "{at:?}");
    // the distance moves with the hole one for one: just outside and just inside the limit
    // moving the hole away from the bend (-x) adds to the distance one for one
    let gap = i.measured.unwrap() - limit;
    for (dx, flagged) in [(gap + 0.01, true), (gap - 0.01, false)] {
        let r = dfm(&part("sendcutsend", "5052", "0.063in", 75.0 + dx));
        assert_eq!(
            !rule(&r, "HoleNearBend").is_empty(),
            flagged,
            "dx {dx}: {}",
            r.text()
        );
    }
}

#[test]
fn dfm_hole_size_and_edge_just_inside_and_just_outside() {
    // 0.5 t for both
    let t = 0.063 * IN;
    let with_r = |r: f64, y: f64| {
        part("sendcutsend", "5052", "0.063in", 40.0)
            .replace("param hs.r = 2", &format!("param hs.r = {r}"))
            .replace("param hs.y = 25", &format!("param hs.y = {y}"))
    };
    let small = dfm(&with_r(0.25 * t - 0.001, 25.0));
    assert_eq!(rule(&small, "HoleTooSmall").len(), 1, "{}", small.text());
    assert!(rule(&dfm(&with_r(0.25 * t + 0.001, 25.0)), "HoleTooSmall").is_empty());
    // its edge 0.5 t from the plate's edge at y = 0
    let near = dfm(&with_r(2.0, 2.0 + 0.5 * t - 0.001));
    assert_eq!(rule(&near, "HoleNearEdge").len(), 1, "{}", near.text());
    assert!(rule(&dfm(&with_r(2.0, 2.0 + 0.5 * t + 0.001)), "HoleNearEdge").is_empty());
}

#[test]
fn dfm_flange_bend_length_and_operations() {
    // 0.303 in the least formed flange for 0.063 in 5052 (measured from the mold line)
    let flange = |h: f64| {
        part("sendcutsend", "5052", "0.063in", 40.0).replace("param up.h = 20", &format!("param up.h = {h}"))
    };
    let short = dfm(&flange(0.303 * IN - 0.01));
    assert_eq!(rule(&short, "FlangeTooShort").len(), 1, "{}", short.text());
    assert_eq!(
        rule(&short, "FlangeTooShort")[0].anchor.as_deref(),
        Some("up.faces")
    );
    assert!(rule(&dfm(&flange(0.303 * IN + 0.01)), "FlangeTooShort").is_empty());
    // a hem: SendCutSend does not hem
    let hemmed = format!(
        "{}tip = hem(base.edge(plan.outline[3]), kind: \"open\", gap: t, length: 10)\n",
        part("sendcutsend", "5052", "0.063in", 40.0)
    );
    let r = dfm(&hemmed);
    let ops = rule(&r, "OperationNotOffered");
    assert_eq!(ops.len(), 1, "{}", r.text());
    assert_eq!(ops[0].feature, "tip");
    // 6061 is cut, not bent
    let r = dfm(&part("sendcutsend", "6061", "0.125in", 40.0));
    assert_eq!(rule(&r, "OperationNotOffered").len(), 1, "{}", r.text());
}

#[test]
fn dfm_reports_refusals_and_parts_without_a_shop() {
    let r = dfm(&part("sendcutsend", "5052", "1.5mm", 40.0));
    assert_eq!(r.refused.len(), 1);
    assert_eq!(r.refused[0].rule, "VendorThickness");
    assert_eq!(r.refused[0].limit, Some(0.063 * IN));
    let j = r.json();
    let mut s = String::new();
    j.write(&mut s);
    assert!(
        s.contains("\"query\":\"dfm\"") && s.contains("\"refused\":[{"),
        "{s}"
    );
    // the JSON of a clean part: the vendor's provenance and the rules it was held to
    let r = dfm(&part("sendcutsend", "5052", "0.063in", 40.0));
    // clean by the vendor's rules and on its press brake (SendCutSend's [tooling])
    assert_eq!(r.count(), 0, "{}", r.text());
    let b = r.sheets[0].brake.as_ref().expect("bent on the vendor's brake");
    assert!(b.feasible, "{}", r.text());
    assert!(b.tooling != forge_agent::json::Json::Null);
    let mut s = String::new();
    r.json().write(&mut s);
    assert!(s.contains("\"verify\":\"Verify with the vendor before ordering.\""));
    assert!(s.contains("\"hole_to_bend\":5.994"), "{s}");
}

#[test]
fn inches() {
    let r = run("shared a = 1in\nshared b = a + 0.5in\n");
    assert!(r.errors.is_empty());
    let v = |n: &str| match &r.shared.iter().find(|p| p.key.name == n).unwrap().value {
        forge_lang::eval::Value::Num(x) => x.v,
        _ => f64::NAN,
    };
    assert_eq!(v("a"), 25.4);
    assert_eq!(v("b"), 1.5 * 25.4);
}

#[test]
fn a_partial_vendor_makes_up_no_bend_data() {
    // OSH Cut states no radius or K: a bent sheet must say its own
    let src = part("oshcut", "5052", "0.063in", 40.0);
    assert_eq!(errors(&run(&src)), ["MissingArg"]);
    let own = src.replace(
        "vendor: \"oshcut\"",
        "r: 1mm, bend: kfactor(ratio(0.4)), vendor: \"oshcut\"",
    );
    let r = run(&own);
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    // and its own rules hold: OSH Cut's minimum hole for 0.063 in 5052 is 0.01575 in
    let d = dfm(&own.replace("param hs.r = 2", "param hs.r = 0.19"));
    assert_eq!(rule(&d, "HoleTooSmall").len(), 1, "{}", d.text());
}

/// A 6 mm plate with a flange `h` tall, on the standard tooling (no vendor).
fn thick(h: f64) -> String {
    format!(
        r#"shared t = 6
shared r = 6
shared k = ratio(0.44)
param plan.L = 80
param plan.W = 50
param up.h = {h}
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (L, W))
}}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
up = flange(base.edge(plan.outline[1]), height: h)
"#
    )
}

#[test]
fn dfm_carries_the_press_brakes_findings() {
    // 20 tall: the standard set bends it
    let r = dfm(&thick(20.0));
    let b = r.sheets[0].brake.as_ref().expect("bends are simulated");
    assert!(b.feasible, "{}", r.text());
    assert_eq!(b.sequence, ["up"]);
    assert!(
        b.items
            .iter()
            .all(|i| i.severity != "violation" && i.severity != "refused"),
        "{}",
        r.text()
    );
    // 14 tall: no die holds it; the simulator's own rule word, on the flange, counted to fix
    let r = dfm(&thick(14.0));
    let b = r.sheets[0].brake.as_ref().unwrap();
    assert!(!b.feasible);
    let short: Vec<_> = b.items.iter().filter(|i| i.rule == "min_flange").collect();
    assert!(!short.is_empty(), "{}", r.text());
    assert_eq!((short[0].feature.as_str(), short[0].line), ("up", 11));
    assert!(short[0].limit.unwrap() > short[0].measured.unwrap());
    // the press-frame point kept as data, never drawn on the folded part
    assert!(short.iter().all(|i| i.at.is_none()));
    assert!(r.count() >= 1);
    let mut s = String::new();
    r.json().write(&mut s);
    assert!(
        s.contains("\"brake\":{") && s.contains("\"rule\":\"min_flange\""),
        "{s}"
    );
    // a flat plate is not put on the brake
    let flat = dfm(&thick(20.0)
        .replace("up = flange(base.edge(plan.outline[1]), height: h)\n", "")
        .replace("param up.h = 20\n", ""));
    assert!(flat.sheets[0].brake.is_none());
}
