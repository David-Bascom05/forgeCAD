//! `query F bendsim`: the reply's schema (forgecad.bendsim/1, docs/bendsim.md), sequences by name
//! and number, tooling files, typed errors, and a flange too short for the die answered with a
//! SetParam fix that clears it.
use forge_agent::json::Json;
use forge_lang::bendsim::{Ask, ask_src};
use forge_lang::query;

fn ell(h: f64) -> String {
    format!(
        "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\nshared h = {h}\n\
         sketch plan(on: xy) {{\n  outline = rect(corner: (0, 0), size: (120, 60))\n}}\n\
         base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
         right = flange(base.edge(plan.outline[1]), height: h)\n"
    )
}

fn channel() -> String {
    "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\nshared h = 30\n\
     sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (100, 200))\n}\n\
     base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
     right = flange(base.edge(plan.outline[1]), height: h)\n\
     left = flange(base.edge(plan.outline[3]), height: h)\n"
        .into()
}

fn keys(j: &Json) -> Vec<String> {
    match j {
        Json::Obj(f) => f.iter().map(|(k, _)| k.clone()).collect(),
        _ => vec![],
    }
}

fn has(j: &Json, want: &[&str]) {
    let k = keys(j);
    for w in want {
        assert!(k.iter().any(|x| x == w), "missing {w} in {k:?}");
    }
}

fn arr(j: &Json, k: &str) -> Vec<Json> {
    match j.get(k) {
        Some(Json::Arr(v)) => v.clone(),
        other => panic!("{k}: {other:?}"),
    }
}

#[test]
fn the_reply_has_its_schema() {
    let a = ask_src(&channel(), None, None, &Ask::default());
    assert!(a.ok, "{}", a.text);
    let j = &a.json;
    has(
        j,
        &[
            "ok",
            "schema",
            "sheet",
            "tooling",
            "press",
            "material",
            "clearance",
            "feasible",
            "sequence",
            "bends",
            "steps",
            "blocking",
            "fixes",
            "suggestions",
            "explored",
            "exhausted",
        ],
    );
    assert_eq!(j.get("schema").and_then(Json::str), Some("forgecad.bendsim/1"));
    assert_eq!(j.get("feasible"), Some(&Json::Bool(true)));
    let seq: Vec<String> = arr(j, "sequence")
        .iter()
        .filter_map(|x| x.str().map(String::from))
        .collect();
    assert_eq!(seq.len(), 2);
    assert!(
        seq.contains(&"right".into()) && seq.contains(&"left".into()),
        "{seq:?}"
    );
    for b in arr(j, "bends") {
        has(&b, &["n", "bend", "angle", "direction", "radius", "length"]);
        assert_eq!(b.get("angle").and_then(Json::num), Some(90.0));
    }
    let steps = arr(j, "steps");
    assert_eq!(steps.len(), 2);
    for st in &steps {
        has(
            st,
            &[
                "n",
                "bend",
                "angle",
                "overbend",
                "punch",
                "die",
                "punch_length",
                "die_length",
                "orientation",
                "backgauge",
                "travel",
                "force",
                "pass",
                "checks",
            ],
        );
        has(st.get("orientation").unwrap(), &["flip", "rotate", "y"]);
        has(st.get("backgauge").unwrap(), &["x", "r", "fingers", "stop"]);
        has(
            st.get("backgauge").unwrap().get("stop").unwrap(),
            &["anchor", "strip", "name", "surface"],
        );
        let checks = arr(st, "checks");
        let rules: Vec<&str> = checks
            .iter()
            .filter_map(|c| c.get("rule").and_then(Json::str))
            .collect();
        for r in [
            "die_angle",
            "punch_angle",
            "punch_radius",
            "v_too_narrow",
            "min_flange",
            "v_depth",
            "stroke",
            "open_height",
            "segments",
            "bed_length",
            "backgauge_reach",
            "self_collision",
            "tool_collision",
            "withdraw",
        ] {
            assert!(rules.contains(&r), "{r} missing from {rules:?}");
        }
        for c in &checks {
            has(
                c,
                &[
                    "rule",
                    "pass",
                    "severity",
                    "value",
                    "limit",
                    "compare",
                    "certified",
                    "unit",
                ],
            );
            let cmp = c.get("compare").and_then(Json::str).unwrap();
            assert!(cmp == "<=" || cmp == ">=");
        }
        assert_eq!(st.get("pass"), Some(&Json::Bool(true)));
    }
    // the same words through the query entry point
    let q = query::ask(&channel(), None, None, "bendsim tooling std-eu");
    assert_eq!(q.json.get("tooling").and_then(Json::str), Some("std-eu"));
}

#[test]
fn sequences_by_name_and_number() {
    for s in ["left,right", "2,1"] {
        let a = ask_src(
            &channel(),
            None,
            None,
            &Ask {
                sequence: Some(s.into()),
                ..Ask::default()
            },
        );
        assert!(a.ok, "{}", a.text);
        let seq: Vec<String> = arr(&a.json, "sequence")
            .iter()
            .filter_map(|x| x.str().map(String::from))
            .collect();
        assert_eq!(seq, vec!["left".to_string(), "right".into()], "{s}");
    }
}

#[test]
fn typed_errors() {
    let kind = |a: &query::Answer| {
        a.json
            .get("error")
            .and_then(|e| e.get("kind"))
            .and_then(Json::str)
            .map(String::from)
    };
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            sequence: Some("front".into()),
            ..Ask::default()
        },
    );
    assert!(!a.ok);
    assert_eq!(kind(&a).as_deref(), Some("NoSuchBend"));
    assert_eq!(arr(a.json.get("error").unwrap(), "bends").len(), 2);
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            sequence: Some("1,1".into()),
            ..Ask::default()
        },
    );
    assert_eq!(kind(&a).as_deref(), Some("BadSequence"));
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            sequence: Some("3".into()),
            ..Ask::default()
        },
    );
    assert_eq!(kind(&a).as_deref(), Some("BadSequence"));
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            tooling: Some("acme".into()),
            ..Ask::default()
        },
    );
    assert_eq!(kind(&a).as_deref(), Some("NoSuchTooling"));
    let a = ask_src(
        "sketch s(on: xy) {\n  c = circle((0, 0), 5)\n}\np = extrude(s.c, 3)\n",
        None,
        None,
        &Ask::default(),
    );
    assert_eq!(kind(&a).as_deref(), Some("NoSheet"));
    let a = query::ask(&channel(), None, None, "bendsim speed 3");
    assert!(!a.ok);
}

#[test]
fn a_short_flange_gets_a_fix_that_clears_the_die() {
    // only a V32 die: a 12 mm flange's leg does not reach its shoulder
    let dir = forge_dirs::temp_dir().join(format!("forgecad-bendsim-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("wide.json"),
        r#"{"id": "wide", "punches": [{"id": "p", "kind": "straight", "height": 100, "tip_radius": 0.8, "tip_angle": 88, "width": 26, "segments": [{"length": 200, "count": 1}]}],
            "dies": [{"id": "v32", "v": 32, "angle": 88, "shoulder_radius": 4, "height": 80, "width": 80, "segments": [{"length": 200, "count": 1}]}]}"#,
    )
    .unwrap();
    let ask = Ask {
        tooling: Some("wide.json".into()),
        ..Ask::default()
    };
    let a = ask_src(&ell(12.0), Some(dir.clone()), None, &ask);
    assert!(a.ok, "{}", a.text);
    assert_eq!(a.json.get("feasible"), Some(&Json::Bool(false)), "{}", a.text);
    let blocking = arr(&a.json, "blocking");
    let rules: Vec<String> = arr(&blocking[0], "checks")
        .iter()
        .filter_map(|c| c.get("rule").and_then(Json::str).map(String::from))
        .collect();
    assert!(rules.iter().any(|r| r == "min_flange"), "{rules:?}");
    let fixes = arr(&a.json, "fixes");
    let fix = fixes
        .iter()
        .find(|f| f.get("op").and_then(Json::str) == Some("SetParam"))
        .expect("a SetParam fix");
    assert_eq!(fix.get("param").and_then(Json::str), Some("h"));
    let h = fix
        .get("value")
        .and_then(|v| v.get("value"))
        .and_then(Json::num)
        .unwrap();
    assert!(h > 12.0, "{h}");
    // the fixed height bends
    let a = ask_src(&ell(h), Some(dir), None, &ask);
    assert_eq!(a.json.get("feasible"), Some(&Json::Bool(true)), "{}", a.text);
}

#[test]
fn standard_tooling_by_default_and_why() {
    let a = ask_src(&channel(), None, None, &Ask::default());
    let src = a.json.get("tooling_source").unwrap();
    assert_eq!(src.get("kind").and_then(Json::str), Some("standard"));
    assert_eq!(src.get("id").and_then(Json::str), Some("std-eu"));
    let st = &arr(&a.json, "steps")[0];
    let c = st.get("choice").unwrap();
    has(c, &["die", "punch", "v_rule", "tried", "rejected"]);
    assert!(
        c.get("punch").and_then(Json::str).unwrap().contains("straight"),
        "{c}"
    );
    // a gooseneck is chosen only where the straight punches fail, and the reply says so
    let narrow = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
        sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (40, 200))\n}\n\
        base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
        right = flange(base.edge(plan.outline[1]), height: 60)\n\
        left = flange(base.edge(plan.outline[3]), height: 60)\n";
    let a = ask_src(narrow, None, None, &Ask::default());
    assert_eq!(a.json.get("feasible"), Some(&Json::Bool(true)), "{}", a.text);
    let st = &arr(&a.json, "steps")[1];
    let why = st
        .get("choice")
        .and_then(|c| c.get("punch"))
        .and_then(Json::str)
        .unwrap()
        .to_string();
    assert!(
        why.contains("gooseneck") && why.contains("closes_on_punch"),
        "{why}"
    );
}

#[test]
fn a_vendors_tooling_narrows_the_choice() {
    // SendCutSend publishes four V widths: a 2 mm channel bends on its 0.630 in die
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            vendor: Some("sendcutsend".into()),
            ..Ask::default()
        },
    );
    assert!(a.ok, "{}", a.text);
    let src = a.json.get("tooling_source").unwrap();
    assert_eq!(src.get("kind").and_then(Json::str), Some("vendor"));
    assert!(
        src.get("verify")
            .and_then(Json::str)
            .unwrap()
            .contains("verify with the vendor")
    );
    for st in arr(&a.json, "steps") {
        assert!(
            st.get("die")
                .and_then(Json::str)
                .unwrap()
                .starts_with("sendcutsend/V"),
            "{st}"
        );
    }
    // JLC publishes no tooling: the standard set, and the reply says why
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            vendor: Some("jlc".into()),
            ..Ask::default()
        },
    );
    assert!(a.ok, "{}", a.text);
    let src = a.json.get("tooling_source").unwrap();
    assert_eq!(src.get("kind").and_then(Json::str), Some("vendor"));
    assert!(
        src.get("note")
            .and_then(Json::str)
            .unwrap()
            .contains("publishes no press-brake tooling")
    );
    assert_eq!(a.json.get("tooling").and_then(Json::str), Some("jlc"));
    let a = ask_src(
        &channel(),
        None,
        None,
        &Ask {
            vendor: Some("acme".into()),
            ..Ask::default()
        },
    );
    assert_eq!(
        a.json
            .get("error")
            .and_then(|e| e.get("kind"))
            .and_then(Json::str),
        Some("NoSuchVendor")
    );
}

#[test]
fn findings_in_the_dfm_item_shape() {
    use forge_lang::bendsim::{items, simulate_sheet};
    let (r, _) = forge_lang::run(&ell(12.0), forge_lang::Options::default(), Default::default());
    let dir = forge_dirs::temp_dir().join(format!("forgecad-bendsim-items-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("wide.json"),
        r#"{"punches": [{"id": "p", "kind": "straight", "height": 100, "tip_radius": 0.8, "tip_angle": 88, "width": 26, "segments": [{"length": 200, "count": 1}]}],
            "dies": [{"id": "v32", "v": 32, "angle": 88, "shoulder_radius": 4, "height": 80, "width": 80, "segments": [{"length": 200, "count": 1}]}]}"#,
    )
    .unwrap();
    let sim = simulate_sheet(
        &r.sheets[0],
        Some(&dir),
        &Ask {
            tooling: Some("wide.json".into()),
            ..Ask::default()
        },
    )
    .unwrap_or_else(|a| panic!("{}", a.text));
    let it = items(&r.sheets[0], &sim);
    assert!(!it.is_empty());
    for i in &it {
        has(
            i,
            &[
                "rule", "severity", "text", "feature", "line", "measured", "limit", "sense", "unit",
                "location", "data", "fixes",
            ],
        );
        has(i.get("location").unwrap(), &["anchor", "faces", "flat", "at"]);
    }
    let mf = it
        .iter()
        .find(|i| i.get("rule").and_then(Json::str) == Some("min_flange"))
        .unwrap();
    assert_eq!(mf.get("severity").and_then(Json::str), Some("refused"));
    assert_eq!(mf.get("sense").and_then(Json::str), Some("min"));
    assert_eq!(arr(mf, "fixes").len(), 1);
}

#[test]
fn an_inch_vendors_full_length_tooling_fits_its_bed() {
    // SendCutSend's dies are one 44 in piece and its bed 44 in: in mm, equal is not over
    let src = "shared t = 0.063in\nparam plan.L = 80\nparam plan.W = 50\nparam up.h = 20\n\
        sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (L, W))\n}\n\
        base = sheet(plan, t: t, vendor: \"sendcutsend\", material: \"5052-H32 aluminum\")\n\
        up = flange(base.edge(plan.outline[1]), height: h)\n";
    let a = ask_src(src, None, None, &Ask::default());
    assert!(a.ok, "{}", a.text);
    assert_eq!(a.json.get("feasible"), Some(&Json::Bool(true)), "{}", a.text);
    let st = &arr(&a.json, "steps")[0];
    assert!(
        st.get("die")
            .and_then(Json::str)
            .unwrap()
            .starts_with("sendcutsend/V"),
        "{st}"
    );
}

fn press(p: &str, punches: Option<&[&str]>) -> Ask {
    Ask {
        press: Some(p.into()),
        punches: punches.map(|x| x.iter().map(|s| s.to_string()).collect()),
        ..Ask::default()
    }
}

#[test]
fn a_real_press_bends_with_its_makers_own_tools() {
    let a = ask_src(&channel(), None, None, &press("trumpf-trubend-5130", None));
    assert!(a.ok, "{}", a.text);
    let j = &a.json;
    let src = j.get("tooling_source").unwrap();
    assert_eq!(src.get("kind").and_then(Json::str), Some("library"));
    assert_eq!(src.get("real"), Some(&Json::Bool(true)));
    assert_eq!(j.get("feasible"), Some(&Json::Bool(true)), "{}", a.text);
    // the operator's move into the second step: checked, and here proven clear
    let steps = arr(j, "steps");
    let handling: Vec<Json> = arr(&steps[1], "checks")
        .into_iter()
        .filter(|c| c.get("rule").and_then(Json::str) == Some("handling"))
        .collect();
    assert_eq!(handling.len(), 1);
    assert_eq!(
        handling[0].get("verdict").and_then(Json::str),
        Some("pass"),
        "{:?}",
        handling[0]
    );
    assert!(
        arr(&steps[0], "checks")
            .iter()
            .all(|c| c.get("rule").and_then(Json::str) != Some("handling"))
    );
    for st in arr(j, "steps") {
        assert!(
            st.get("punch")
                .and_then(Json::str)
                .unwrap()
                .starts_with("trumpf/")
        );
        assert!(st.get("die").and_then(Json::str).unwrap().starts_with("trumpf/"));
        // the clamps' heights are not published: open height and stroke are not checked,
        // never passed
        for c in arr(&st, "checks") {
            let rule = c.get("rule").and_then(Json::str).unwrap();
            if rule == "open_height" || rule == "stroke" {
                assert_eq!(
                    c.get("verdict").and_then(Json::str),
                    Some("not_checked"),
                    "{rule}"
                );
            }
        }
    }
}

#[test]
fn a_flange_closes_on_a_real_straight_punch() {
    let u = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\nshared h = 70\n\
             sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (40, 300))\n}\n\
             base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
             right = flange(base.edge(plan.outline[1]), height: h)\n\
             left = flange(base.edge(plan.outline[3]), height: h)\n";
    let a = ask_src(
        u,
        None,
        None,
        &press(
            "bystronic-xpert-pro-100-3100",
            Some(&["bystronic/P6_RF-A_H175_R1.0"]),
        ),
    );
    assert!(a.ok, "{}", a.text);
    assert_eq!(a.json.get("feasible"), Some(&Json::Bool(false)));
    let blocking = arr(&a.json, "blocking");
    assert!(!blocking.is_empty());
    for b in blocking {
        let c = &arr(&b, "checks")[0];
        assert_eq!(c.get("rule").and_then(Json::str), Some("closes_on_punch"));
        assert_eq!(c.get("certified"), Some(&Json::Bool(true)));
        assert_eq!(
            c.get("tool").and_then(Json::str),
            Some("punch:bystronic/P6_RF-A_H175_R1.0/body")
        );
        assert!(c.get("phi").and_then(Json::num).is_some() && c.get("at").is_some());
    }
}

#[test]
fn unknown_geometry_and_unknown_clamping_refuse() {
    // Amada's clamping system and its dies' heights are not in the sources read
    let a = ask_src(&channel(), None, None, &press("amada-hg-1303", None));
    assert!(!a.ok);
    assert_eq!(
        a.json
            .get("error")
            .and_then(|e| e.get("kind"))
            .and_then(Json::str),
        Some("NoDefaultTooling"),
        "{}",
        a.text
    );
    let mut ask = press("amada-hg-1303", Some(&["trumpf/OW203-K"]));
    let die = forge_brake::catalog::Catalog::builtin()
        .dies
        .iter()
        .find(|d| d.prov.maker == "Amada" && d.die.is_err())
        .map(|d| d.id.clone())
        .unwrap();
    ask.dies = Some(vec![die]);
    let a = ask_src(&channel(), None, None, &ask);
    assert!(!a.ok);
    assert_eq!(
        a.json
            .get("error")
            .and_then(|e| e.get("kind"))
            .and_then(Json::str),
        Some("UnknownGeometry"),
        "{}",
        a.text
    );
    let a = ask_src(&channel(), None, None, &press("no-such-press", None));
    assert_eq!(
        a.json
            .get("error")
            .and_then(|e| e.get("kind"))
            .and_then(Json::str),
        Some("NoSuchPress"),
        "{}",
        a.text
    );
}

#[test]
fn ranked_programs_are_offered() {
    let ask = Ask {
        programs: Some(3),
        ..Ask::default()
    };
    let a = ask_src(&channel(), None, None, &ask);
    assert!(a.ok, "{}", a.text);
    let ps = arr(&a.json, "programs");
    assert!(!ps.is_empty() && ps.len() <= 3);
    let key = |p: &Json| {
        let n = |k: &str| p.get(k).and_then(Json::num).unwrap();
        (n("flips") + n("turns"), n("stations"), n("tool_changes"))
    };
    for w in ps.windows(2) {
        assert!(key(&w[0]) <= key(&w[1]));
    }
    for p in &ps {
        has(
            p,
            &[
                "rank",
                "sequence",
                "flips",
                "turns",
                "stations",
                "tool_changes",
                "steps",
            ],
        );
        assert_eq!(arr(p, "sequence").len(), 2);
    }
    // the plan itself is unchanged by asking
    let plain = ask_src(&channel(), None, None, &Ask::default());
    assert_eq!(plain.json.get("sequence"), a.json.get("sequence"));
}

/// Stage bending on a real press (`stations auto`): the showcase load tray on a TruBend 5130 with
/// Trumpf's catalogue tools needs more than one station (its lips and rear want goosenecks, its
/// tab a short stack); every station on the bed, none overlapping, every step at the station
/// holding its tools, the program feasible with all of them on the press, and each station says
/// why its punch.
#[test]
fn stations_auto_on_a_real_press() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    ))
    .unwrap();
    let ask =
        forge_lang::bendsim::parse(&["press", "trumpf-trubend-5130", "stations", "auto"].map(String::from))
            .unwrap();
    assert!(ask.stage);
    let a = ask_src(&src, None, None, &ask);
    assert!(a.ok, "{}", a.text);
    let j = &a.json;
    assert_eq!(j.get("feasible"), Some(&Json::Bool(true)));
    let st = j.get("stage").expect("stage");
    assert_eq!(st.get("ok"), Some(&Json::Bool(true)), "{st:?}");
    let stations = arr(st, "stations");
    assert!(stations.len() >= 2, "{stations:?}");
    let n = |x: &Json, k: &str| x.get(k).and_then(Json::num).unwrap();
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for s in &stations {
        let w = 0.5 * n(s, "punch_length").max(n(s, "die_length"));
        let y = n(s, "y");
        assert!(
            (-1615.0..=1615.0).contains(&(y - w)) && y + w <= 1615.0,
            "on the bed: {s:?}"
        );
        for (a, b) in &spans {
            assert!(y + w <= *a || y - w >= *b, "stacks overlap");
        }
        spans.push((y - w, y + w));
        assert!(!s.get("why").and_then(Json::str).unwrap_or("").is_empty());
    }
    let steps = arr(j, "steps");
    let at = arr(st, "steps");
    assert_eq!(steps.len(), at.len());
    for (s, a) in steps.iter().zip(&at) {
        let k = n(a, "station") as usize - 1;
        assert_eq!(s.get("punch"), stations[k].get("punch"));
        assert_eq!(s.get("die"), stations[k].get("die"));
        assert_eq!(s.get("pass"), Some(&Json::Bool(true)));
        // the press's speeds are published: each step's ram time is known
        assert!(a.get("seconds").and_then(Json::num).is_some_and(|x| x > 0.0));
    }
}

/// `lock B=N`: a bend made at the station the user set, the rest searched; a station that is not
/// there, or a bend that is not, is a typed error.
#[test]
fn a_lock_keeps_a_bend_at_its_station() {
    use forge_lang::bendsim::StationAsk;
    let st = |punch: &str, y: f64| StationAsk {
        punch: punch.into(),
        die: "std-eu/V12-88".into(),
        y,
        punch_length: 200.0,
        die_length: 200.0,
    };
    let mut ask = forge_lang::bendsim::parse(&["lock", "left=2"].map(String::from)).unwrap();
    assert_eq!(ask.locks, vec![("left".to_string(), 1)]);
    ask.stations = vec![st("std-eu/P85-R08-H120", -600.0), st("std-eu/G88-R08", 600.0)];
    let a = ask_src(&channel(), None, None, &ask);
    assert!(a.ok, "{}", a.text);
    assert_eq!(a.json.get("feasible"), Some(&Json::Bool(true)));
    for s in arr(&a.json, "steps") {
        let want = if s.get("bend").and_then(Json::str) == Some("left") {
            "std-eu/G88-R08"
        } else {
            "std-eu/P85-R08-H120"
        };
        assert_eq!(s.get("punch").and_then(Json::str), Some(want));
    }
    let mut bad = ask.clone();
    bad.locks = vec![("left".into(), 5)];
    let e = ask_src(&channel(), None, None, &bad);
    assert!(!e.ok && e.text.contains("BadStation"), "{}", e.text);
    bad.locks = vec![("nope".into(), 0)];
    let e = ask_src(&channel(), None, None, &bad);
    assert!(!e.ok && e.text.contains("NoSuchBend"), "{}", e.text);
    assert!(forge_lang::bendsim::parse(&["lock", "left=0"].map(String::from)).is_err());
}

/// The same question twice in one process (a reopening, Simulate Again on an unchanged part): the
/// same reply, the second from the kept plan at once; a changed part is planned afresh.
#[test]
fn an_unchanged_part_is_answered_from_the_kept_plan() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    ))
    .unwrap();
    let ask = Ask {
        programs: Some(3),
        ..Ask::default()
    };
    let t0 = std::time::Instant::now();
    let a = ask_src(&src, None, None, &ask);
    let first = t0.elapsed();
    let t1 = std::time::Instant::now();
    let b = ask_src(&src, None, None, &ask);
    let second = t1.elapsed();
    assert!(a.ok, "{}", a.text);
    assert_eq!(a.json, b.json);
    assert!(second * 5 < first, "kept plan: {second:?} vs {first:?}");
    // a changed part (the lips 1 mm taller) is planned for itself
    let c = ask_src(
        &src.replace("shared lip = 12", "shared lip = 13"),
        None,
        None,
        &ask,
    );
    assert!(c.ok);
    assert_ne!(a.json, c.json);
}

/// `begin` / `step` / `finish` (what the app drives a slice at a time, or a worker drives) give
/// the very reply `simulate` gives; the scene (model, tooling) is there before anything is
/// planned; the simulation can be sent from a worker thread.
#[test]
fn a_simulation_planned_a_little_at_a_time_is_the_simulation() {
    fn send<T: Send>() {}
    send::<forge_lang::bendsim::Sim>();
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    ))
    .unwrap();
    for (part, ask) in [
        (src.clone(), Ask::default()),
        (channel(), Ask::default()),
        (
            src.clone(),
            Ask {
                programs: Some(2),
                ..Ask::default()
            },
        ),
    ] {
        forge_lang::bendsim::forget_plans();
        let whole = ask_src(&part, None, None, &ask);
        forge_lang::bendsim::forget_plans();
        let mut p = forge_lang::bendsim::begin(&part, None, None, &ask).expect("begun");
        assert!(!p.model().bends.is_empty() && !p.tooling().punches.is_empty());
        assert!(!p.progress().done);
        let mut ticks = 0;
        while !p.step(3).done {
            ticks += 1;
            assert!(ticks < 100_000);
        }
        if ask.programs.is_none() && part == src {
            assert!(ticks > 1, "planned in slices");
        }
        let sim = p.finish();
        assert_eq!(sim.answer.json, whole.json);
    }
}

/// `contacts` and `bendsim collisions [step N]`: where the part and the tools collide, on both
/// bodies. A narrow, tall channel on straight punches, in a stated order: its second bend runs
/// its first flange into the punch (a proven hit, its depth, its faces by name, points on both
/// bodies, a box, when, words); load_tray on its own tooling has none.
#[test]
fn collisions_name_both_bodies_and_where() {
    let src = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (30, 200))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: 60)\n\
left = flange(base.edge(plan.outline[3]), height: 60)\n";
    let words = [
        "punches",
        "std-eu/P85-R08-H120,std-eu/P88-R08",
        "sequence",
        "1,2",
        "collisions",
        "step",
        "2",
    ];
    let ask = forge_lang::bendsim::parse(&words.map(String::from)).unwrap();
    assert!(ask.collisions && ask.step == Some(2));
    let a = ask_src(src, None, None, &ask);
    assert!(a.ok, "{}", a.text);
    let j = &a.json;
    assert_eq!(
        j.get("schema").and_then(Json::str),
        Some("forgecad.bendsim.collisions/1")
    );
    let cs = arr(j, "contacts");
    assert!(!cs.is_empty());
    for c in &cs {
        has(
            c,
            &[
                "step", "bend", "rule", "severity", "verdict", "phase", "tool", "estimate", "faces", "depth",
                "lower", "limit", "when", "region", "text",
            ],
        );
        has(
            c.get("region").unwrap(),
            &[
                "points",
                "tool_points",
                "bbox",
                "cells",
                "proven",
                "complete",
                "exact",
            ],
        );
        assert_eq!(c.get("step").and_then(Json::num), Some(2.0));
        for f in arr(c, "faces") {
            has(&f, &["anchor", "strip", "name", "surface", "cells", "proven"]);
        }
        assert_eq!(
            arr(c.get("region").unwrap(), "points").len(),
            arr(c.get("region").unwrap(), "tool_points").len()
        );
    }
    let hit = cs
        .iter()
        .find(|c| {
            c.get("verdict").and_then(Json::str) == Some("hit")
                && c.get("rule").and_then(Json::str) == Some("closes_on_punch")
        })
        .expect("the flange closes on the punch");
    let depth = hit.get("depth").and_then(Json::num).unwrap();
    assert!(depth < 0.0, "into the punch: {depth}");
    let text = hit.get("text").and_then(Json::str).unwrap();
    assert!(
        text.contains("right.top")
            && text.contains("runs")
            && text.contains("P85-R08-H120")
            && text.contains("step 2"),
        "{text}"
    );
    assert!(a.text.contains("runs"));
    // the same contacts sit in the whole reply
    let mut whole_ask = ask.clone();
    whole_ask.collisions = false;
    let w = ask_src(src, None, None, &whole_ask);
    assert!(arr(&w.json, "contacts").len() >= cs.len());
    // a part that bends clear has none
    let tray = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    ))
    .unwrap();
    let t = ask_src(
        &tray,
        None,
        None,
        &forge_lang::bendsim::parse(&["collisions".to_string()]).unwrap(),
    );
    assert!(arr(&t.json, "contacts").is_empty(), "{}", t.text);
}

/// Part against part: a box whose tooling asks 3 mm between the part's own pieces; its flanges
/// come within that of their neighbours. Each such contact names the moving faces and the faces
/// they meet (`others`), and says so in words.
#[test]
fn collisions_of_the_part_with_itself_name_both_faces() {
    let d = forge_dirs::temp_dir().join(format!("fg-bendsim-self-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("tight.json"),
        r#"{"base": "std-eu", "id": "tight", "self_clearance": 3}"#,
    )
    .unwrap();
    let src = "shared t = 1.5\nshared r = 1.5\nshared k = ratio(0.4)\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (120, 60))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
front = flange(base.edge(plan.outline[0]), height: 25)\n\
right = flange(base.edge(plan.outline[1]), height: 25)\n\
back = flange(base.edge(plan.outline[2]), height: 25)\n\
left = flange(base.edge(plan.outline[3]), height: 25)\n";
    let ask = forge_lang::bendsim::parse(
        &["tooling", "tight.json", "sequence", "1,2,3,4", "collisions"].map(String::from),
    )
    .unwrap();
    let a = ask_src(src, Some(d.clone()), None, &ask);
    assert!(a.ok, "{}", a.text);
    let selfs: Vec<Json> = arr(&a.json, "contacts")
        .into_iter()
        .filter(|c| c.get("rule").and_then(Json::str) == Some("self_collision"))
        .collect();
    assert!(!selfs.is_empty(), "{}", a.text);
    for c in &selfs {
        assert_eq!(c.get("tool").and_then(Json::str), Some("part"));
        let others = arr(c, "others");
        assert!(!others.is_empty());
        let text = c.get("text").and_then(Json::str).unwrap();
        let other = others[0].get("anchor").and_then(Json::str).unwrap();
        assert!(text.contains(other), "{text}");
    }
    // a flange meets its neighbour: the right flange and the front one
    assert!(selfs.iter().any(|c| {
        let t = c.get("text").and_then(Json::str).unwrap_or("");
        t.starts_with("right.") && t.contains("front.")
    }));
}

/// The operator's move is certified (rank 3b): on the staged load tray on a TruBend 5130, with
/// four stations on the bed, every step after the first has its handling check settled (a proven
/// pass, Near, or a proven fail), never an undecided envelope; the method says so.
#[test]
fn handling_is_certified_on_a_staged_press() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    ))
    .unwrap();
    let ask =
        forge_lang::bendsim::parse(&["press", "trumpf-trubend-5130", "stations", "auto"].map(String::from))
            .unwrap();
    let a = ask_src(&src, None, None, &ask);
    assert!(a.ok, "{}", a.text);
    let steps = arr(&a.json, "steps");
    let mut n = 0;
    for (k, st) in steps.iter().enumerate() {
        for c in arr(st, "checks") {
            if c.get("rule").and_then(Json::str) != Some("handling") {
                continue;
            }
            n += 1;
            assert!(k > 0, "the first step has no handling");
            let v = c.get("verdict").and_then(Json::str).unwrap();
            assert!(matches!(v, "pass" | "near" | "fail"), "step {k}: {v} {c}");
            if v != "near" {
                assert_eq!(c.get("certified"), Some(&Json::Bool(true)), "{c}");
            }
            assert!(
                c.get("method").and_then(Json::str).unwrap().contains("certified search"),
                "{c}"
            );
        }
    }
    assert_eq!(n, steps.len() - 1);
}
