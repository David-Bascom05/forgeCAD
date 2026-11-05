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
    let wrong = s.replace("material: \"HIPS\")", "material: \"HIPS\", tool: \"female\")");
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

#[test]
fn a_shelled_part_says_its_own_thickness() {
    let s = src("shell_cover");
    let r = built("shell_cover", &s);
    assert!((r.thermos[0].t - 1.5).abs() < 0.01, "{}", r.thermos[0].t);
    // a thickness given that is not the wall's is refused, with the wall's as the fix
    let wrong = s.replace("from: \"wall\",", "from: \"wall\", t: 3,");
    let r = run(&wrong);
    let e = r.errors.first().expect("refused");
    assert_eq!(e.kind(), "WallNotT");
    let fixed = apply(&wrong, &e.fixes()[0]).expect("applies");
    assert!(run(&fixed).errors.is_empty());
}

#[test]
fn the_three_ways_in_give_the_moulds_they_should() {
    let s = src("cover");
    let top = |r: &Run| {
        let m = &r.thermos[0].formed.mould;
        m.vertex_ids()
            .map(|v| m.vertex(v).point.z)
            .fold(f64::MIN, f64::max)
    };
    // the solid is the outside: the mould is a wall's thickness under it, grown by the shrink
    let outer = built("cover", &s);
    let k = 1.0 + forge_thermo::placeholder().material("HIPS").unwrap().shrink;
    assert!((top(&outer) - (35.0 - 2.0) * k).abs() < 0.05, "{}", top(&outer));
    // the solid is the inside: the mould is the solid
    let inner = built(
        "cover",
        &s.replace("material: \"HIPS\")", "material: \"HIPS\", from: \"inner\")"),
    );
    assert!((top(&inner) - 35.0 * k).abs() < 1e-6, "{}", top(&inner));
}

#[test]
fn a_cover_reports_its_verdict_its_two_masses_and_its_field() {
    let s = src("cover");
    let r = built("cover", &s);
    let o = &r.thermos[0];
    let j = forge_agent::json::Json::parse(&forge_lang::report::json(&r, &s).to_string()).unwrap();
    let t = match j.get("thermoforms") {
        Some(forge_agent::json::Json::Arr(a)) => &a[0],
        _ => panic!("no thermoforms"),
    };
    let num = |path: &[&str]| path.iter().try_fold(t, |j, k| j.get(k)).and_then(|x| x.num());
    let w = o.walls.as_ref().unwrap();
    assert_eq!(t.get("verdict").and_then(|v| v.str()), Some("pass"));
    assert!((num(&["limit", "min_wall"]).unwrap() - o.limit()).abs() < 1e-3);
    assert!(w.min.0 * o.t >= o.limit());
    // HIPS: 1040 kg/m3; the modelled part at t, the simulated one thinner everywhere but the top
    let d = num(&["mass", "density_kg_m3"]).unwrap();
    assert_eq!(d, 1040.0);
    let mm = num(&["mass", "modelled", "mass_kg"]).unwrap();
    let ms = num(&["mass", "simulated", "mass_kg"]).unwrap();
    let sim = o.sim.as_ref().unwrap();
    // the report keeps four figures
    assert!((ms - sim.mass.volume * d * 1e-9).abs() < 2e-3 * ms);
    // the simulated mass is the wall integrated over the part, to 0.5 %
    assert!((sim.mass.volume - sim.sheet_volume).abs() < 0.005 * sim.sheet_volume);
    assert!(ms < mm && ms > 0.4 * mm, "{ms} {mm}");
    // the field: every point on the part, inside the extremes
    let field = o.field();
    assert!(field.len() > 500);
    // the report says how many, the points only when asked (`fg run --field`)
    assert_eq!(num(&["field", "count"]), Some(field.len() as f64));
    assert!(t.get("field").and_then(|f| f.get("points")).is_none());
    assert!(
        field
            .iter()
            .all(|(_, h)| *h >= w.min.0 * o.t - 1e-9 && *h <= w.max.0 * o.t + 1e-9)
    );
    // the run says where each solid's faces are: the part's and the mould's together are the body
    let b = r.body.as_ref().unwrap();
    let total: u32 = r.solids.iter().map(|x| x.faces.len() as u32).sum();
    assert_eq!(total as usize, b.faces.len());
    assert_eq!(b.dead, 0);
    assert!(r.solids.iter().any(|x| x.solid == o.part_solid));
    assert!(r.solids.iter().any(|x| x.solid == o.mould_solid));
    // the blank: the trim's box and the clamp, t thick, closed
    assert!(o.blank().open_edges().is_empty());
    let (bw, bl) = o.blank_size();
    assert!((o.blank().volume() - bw * bl * o.t).abs() < 1e-6 * bw * bl);
}

#[test]
fn an_assembly_draws_a_formed_part_without_its_mould_and_keeps_the_mould_as_tooling() {
    let dir = format!("{}/../../examples/thermoform", env!("CARGO_MANIFEST_DIR"));
    let text = "assembly\n\nlid = part(\"cover.fg\", ground: true)\n";
    let opts = Options {
        dir: Some(dir.into()),
        render: true,
        ..Options::default()
    };
    let r = forge_lang::run(text, opts, Default::default()).0;
    assert!(
        r.errors.is_empty(),
        "{:?}",
        r.errors.iter().map(|e| e.kind()).collect::<Vec<_>>()
    );
    let a = r.asm.as_ref().expect("an assembly");
    let part = run(&src("cover"));
    let o = &part.thermos[0];
    let faces = |id: usize| {
        part.solids
            .iter()
            .find(|s| s.solid == id)
            .map(|s| s.faces.len())
            .unwrap()
    };
    // the occurrence draws the formed part alone; its mould is the tooling beside it
    assert_eq!(a.draw.len(), 1);
    assert_eq!(a.draw[0].0.faces.len(), faces(o.part_solid));
    let tool = a.occ_tooling[0].as_ref().expect("the mould");
    assert_eq!(tool.faces.len(), faces(o.mould_solid));
    assert_eq!(a.interferences, 0);
}
