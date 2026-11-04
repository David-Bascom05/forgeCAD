//! Thermoforming in Forge: the example parts in examples/thermoform built, their checks typed, the
//! fixes applied and the faults gone, and the refusals typed with their fixes.
use forge_lang::edit::apply;
use forge_lang::error::{Error, Op, Warning};
use forge_lang::thermo::ThermoNote;
use forge_lang::{Options, Run};

fn src(name: &str) -> String {
    let path = format!(
        "{}/../../examples/thermoform/{name}.fg",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).expect("part file")
}

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn notes(r: &Run) -> Vec<&ThermoNote> {
    r.warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Thermo(n) => Some(&**n),
            _ => None,
        })
        .collect()
}

fn kinds(r: &Run) -> Vec<&'static str> {
    notes(r).iter().map(|n| n.kind()).collect()
}

fn built(name: &str, s: &str) -> Run {
    let r = run(s);
    assert!(r.errors.is_empty(), "{name}: {}", forge_lang::report::text(&r, s));
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.problems, 0, "{name}");
    assert_eq!(m.bodies, 2, "{name}: the part and the tool");
    assert_eq!(r.thermos.len(), 1, "{name}");
    r
}

#[test]
fn a_clean_cover_says_only_what_is_a_stand_in() {
    let s = src("cover");
    let r = built("cover", &s);
    assert_eq!(kinds(&r), ["StandIn"]);
    let rep = &r.thermos[0].formed.report;
    let j = forge_lang::report::json(&r, &s).to_string();
    let hips = forge_thermo::placeholder().material("HIPS").cloned().unwrap();
    assert!(
        (rep.scale - 1.0 - hips.shrink).abs() < 1e-12,
        "HIPS's shrink grows the tool"
    );
    // the thickness model ran on the material's own fit, and says where the wall is thinnest
    let w = r.thermos[0].walls.as_ref().expect("walls");
    assert!(!r.thermos[0].law.stand_in, "{:?}", r.thermos[0].law);
    assert!(
        w.min.0 > 0.0 && w.min.0 < w.max.0 && w.unformed.is_empty(),
        "{:?}",
        (w.min, w.max)
    );
    assert!(j.contains("\"walls\"") && j.contains("law_source"), "{j}");
    assert!(rep.adr > 1.5 && rep.adr < 3.0, "{}", rep.adr);
    assert!(!rep.vents.is_empty());
    let j = forge_lang::report::json(&r, &s).to_string();
    assert!(
        j.contains("\"thermoforms\"") && j.contains("areal_draw_ratio"),
        "{j}"
    );
}

#[test]
fn a_snap_bead_locks_and_close_posts_web() {
    let s = src("snap_lid");
    let r = built("snap_lid", &s);
    let k = kinds(&r);
    for want in ["Undercut", "Webbing", "DraftTooLow", "SharpEdge"] {
        assert!(k.contains(&want), "{want}: {k:?}");
    }
    // the bead's zero-draft sides: its taper is the fix, and applying it cures them
    let n = notes(&r)
        .into_iter()
        .find(|n| n.kind() == "DraftTooLow" && n.made_by == ["bead"])
        .expect("the bead's sides");
    assert_eq!(
        n.fixes,
        [Op::SetArg {
            id: "bead".into(),
            arg: "taper".into(),
            text: "4deg".into()
        }]
    );
    let fixed = apply(&s, &n.fixes[0]).expect("applies");
    let r2 = run(&fixed);
    assert!(
        !notes(&r2)
            .iter()
            .any(|n| n.kind() == "DraftTooLow" && n.made_by == ["bead"]),
        "{}",
        forge_lang::report::text(&r2, &fixed)
    );
}

#[test]
fn a_deep_cup_draws_too_deep() {
    let s = src("cup");
    let r = built("cup", &s);
    let k = kinds(&r);
    assert!(
        k.contains(&"DrawTooDeep") && k.contains(&"AreaDrawTooHigh"),
        "{k:?}"
    );
    // the vents are on the cup's floor
    assert!(r.thermos[0].formed.report.vents.iter().all(|v| v.z < -70.0));
}

#[test]
fn a_tray_pockets_sharp_rims_are_rounded_by_the_fix() {
    let s = src("tray");
    let r = built("tray", &s);
    let sharp: Vec<&ThermoNote> = notes(&r)
        .into_iter()
        .filter(|n| n.kind() == "SharpEdge")
        .collect();
    assert!(!sharp.is_empty());
    let mut fixed = s.clone();
    for n in &sharp {
        let Some(Op::AddFeature { text, .. }) = n.fixes.first() else {
            panic!("no fix: {n:?}")
        };
        assert!(text.contains("edges(between:"), "{text}");
        fixed = apply(&fixed, &n.fixes[0]).expect("applies");
    }
    let r2 = run(&fixed);
    assert!(r2.errors.is_empty(), "{}", forge_lang::report::text(&r2, &fixed));
    assert!(
        !kinds(&r2).contains(&"SharpEdge"),
        "{}",
        forge_lang::report::text(&r2, &fixed)
    );
}

#[test]
fn refusals_are_typed_with_fixes() {
    let s = src("cover");
    let r = run(&s.replace("material: \"HIPS\"", "material: \"PVC\""));
    let Some(e @ Error::Thermo(_)) = r.errors.first() else {
        panic!("{:?}", r.errors)
    };
    assert_eq!(e.kind(), "UnknownMaterial");
    assert!(e.fixes().len() >= 6);
    // a male shape said to be female: its walls lean the wrong way for a cavity
    let wrong = s.replace("material: \"HIPS\")", "material: \"HIPS\", side: \"outer\")");
    let r = run(&wrong);
    assert!(
        kinds(&r).contains(&"Undercut"),
        "{}",
        forge_lang::report::text(&r, &wrong)
    );
    // a ball stands on no flat face
    let ball = "sketch s(on: xy) {\n  c = circle((0, 0), 10)\n}\nknob = ball(s.c, 15)\n\
                f = thermoform(knob, t: 2, material: \"ABS\")\n";
    let r = run(ball);
    let e = r.errors.first().expect("refused");
    assert_eq!(e.kind(), "NoOpening", "{}", forge_lang::report::text(&r, ball));
    assert_eq!(e.fixes().len(), 1);
}

#[test]
fn the_shop_reads_gauges_and_the_sheet() {
    let s = src("cover").replace(
        "cover = thermoform(body, t: t, material: \"HIPS\")",
        "shared former = thermo_shop(gauges: [1.5, 3], max_sheet: (150, 180))\n\
         cover = thermoform(body, t: t, material: \"HIPS\", shop: former, sheet: (100, 100))",
    );
    let r = run(&s);
    let k = kinds(&r);
    for want in ["GaugeNotStocked", "SheetTooSmall", "SheetTooLarge"] {
        assert!(k.contains(&want), "{want}: {k:?}");
    }
    let g = notes(&r)
        .into_iter()
        .find(|n| n.kind() == "GaugeNotStocked")
        .expect("gauge");
    let fixed = apply(&s, &g.fixes[0]).expect("applies");
    assert!(!kinds(&run(&fixed)).contains(&"GaugeNotStocked"));
}
