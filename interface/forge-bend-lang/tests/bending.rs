//! The bending document: written from a planned program, simulated as stated, edited by
//! forge-lang's ops (a reorder is a MoveFeature), and its typed refusals.
use forge_agent::json::Json;
use forge_bend_lang::{document, is_bending, read, run};
use forge_lang::bendsim::{Ask, simulate};
use forge_lang::edit::apply;
use forge_lang::error::Op;
use std::path::PathBuf;

const CHANNEL: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\nshared h = 30\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (100, 200))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: h)\n\
left = flange(base.edge(plan.outline[3]), height: h)\n";

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-bending-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("channel.fg"), CHANNEL).unwrap();
    d
}

fn planned(press: &str) -> String {
    let ask = Ask {
        press: Some(press.into()),
        ..Ask::default()
    };
    let sim = simulate(CHANNEL, None, None, &ask).expect("simulated");
    assert!(sim.plan.feasible);
    document("channel.fg", None, Some(press), "std-eu", &sim)
}

fn sequence(j: &Json) -> Vec<String> {
    match j.get("simulation").and_then(|s| s.get("sequence")) {
        Some(Json::Arr(v)) => v.iter().filter_map(|x| x.str().map(String::from)).collect(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_planned_program_written_and_run_as_stated() {
    let d = dir("run");
    let doc = planned("trumpf-trubend-5130");
    assert!(is_bending(&doc), "{doc}");
    let r = read(&doc).unwrap();
    assert_eq!(r.bends.len(), 2);
    assert!(r.bends.iter().all(|b| b.station.is_some()));
    let b = run(&doc, &d);
    assert!(b.ok, "{:?}\n{doc}", b.errors);
    let j = b.json();
    assert_eq!(j.get("schema").and_then(Json::str), Some("forgecad.bending/1"));
    let sim = j.get("simulation").unwrap();
    assert_eq!(sim.get("feasible"), Some(&Json::Bool(true)), "{doc}");
    // the stations are the press's: the tools at the stated places
    let s = b.sim.as_ref().unwrap();
    assert!(!s.tooling.stations.is_empty());
    assert!(b.sheet().contains("Tool layout") && b.sheet().contains("trumpf/"));
}

#[test]
fn a_reorder_is_a_move_of_a_bend_line() {
    let d = dir("move");
    let doc = planned("trumpf-trubend-5130");
    let before = sequence(&run(&doc, &d).json());
    let moved = apply(
        &doc,
        &Op::MoveFeature {
            id: "b2".into(),
            before: Some("b1".into()),
        },
    )
    .expect("moved");
    let after = sequence(&run(&moved, &d).json());
    assert_eq!(after, vec![before[1].clone(), before[0].clone()]);
    // turned end for end: an argument set
    let turned = apply(
        &moved,
        &Op::SetArg {
            id: "b1".into(),
            arg: "rotate".into(),
            text: "true".into(),
        },
    )
    .expect("set");
    assert_eq!(
        read(&turned)
            .unwrap()
            .bends
            .iter()
            .find(|b| b.id == "b1")
            .unwrap()
            .rotate,
        Some(true)
    );
    assert!(run(&turned, &d).sim.is_some());
}

#[test]
fn refusals_are_typed() {
    let d = dir("refuse");
    let doc = planned("trumpf-trubend-5130") + "prog = program(format: \"delem\")\n";
    let b = run(&doc, &d);
    assert!(!b.ok);
    let j = b.json();
    let kinds: Vec<String> = match j.get("errors") {
        Some(Json::Arr(v)) => v
            .iter()
            .filter_map(|e| e.get("kind").and_then(Json::str).map(String::from))
            .collect(),
        _ => vec![],
    };
    assert!(
        kinds.contains(&"ProgramFormatUnavailable".to_string()),
        "{kinds:?}"
    );
    // the simulation still answers
    assert!(b.sim.is_some());
    assert!(read("bending\nx = drill(3)\n").is_err());
    assert!(read("bending\npart = part(\"a.fg\")\nb = bend(\"x\", \"y\")\n").is_err());
    assert!(!is_bending("plate = extrude(plan, 3)\n"));
}

#[test]
fn arguments_are_checked_against_the_list() {
    let e = read("bending\npart = part(\"a.fg\")\nb = bend(\"x\", rotat: true)\n").unwrap_err();
    assert!(
        e[0].1.starts_with("UnknownArg") && e[0].1.contains("nearest `rotate`"),
        "{e:?}"
    );
    let e = read("bending\npart = part(\"a.fg\")\nb = bend(\"x\", \"y\")\n").unwrap_err();
    assert!(e[0].1.contains("positional argument #1"), "{e:?}");
    // every built-in has its list
    for n in forge_bend_lang::NAMES {
        assert!(forge_bend_lang::ARGS.iter().any(|a| a.0 == *n), "{n}");
    }
}
