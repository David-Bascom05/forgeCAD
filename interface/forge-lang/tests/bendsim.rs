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
