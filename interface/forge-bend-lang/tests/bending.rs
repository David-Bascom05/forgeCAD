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
    let d = forge_dirs::temp_dir().join(format!("fg-bending-{}-{tag}", forge_dirs::process_id()));
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
    // a step with no name would be silently nothing: refused, typed
    let e = read("bending\npart = part(\"a.fg\")\nbend(\"left\")\n").unwrap_err();
    assert!(e.iter().any(|(_, m)| m.starts_with("Unnamed")), "{e:?}");
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

#[test]
fn a_cincinnati_program_is_written() {
    let d = dir("cinci");
    let doc = planned("trumpf-trubend-5130") + "prog = program(format: \"cincinnati\")\n";
    let b = run(&doc, &d);
    assert!(b.ok, "{:?}", b.errors);
    let (xml, ext, notes) = b.program_text().unwrap().unwrap();
    assert_eq!(ext, "pgm");
    assert!(xml.starts_with("<?xml version=\"1.0\" ?>\n<Document><PressBrakeProgram Version=\"9\""));
    assert!(xml.contains("NumSteps=\"2\"") && xml.matches("<Step ").count() == 2);
    // a 90 degree bend: included 90, in angle mode
    assert!(xml.contains("RevMode=\"0\" RevAngle=\"90\""), "{xml}");
    // 2 mm stock in inches
    assert!(xml.contains("MatThick=\"0.0787\""), "{xml}");
    assert_eq!(xml.matches("<ProgOutData />").count(), 16);
    // well formed: every element opened is closed, in order
    let mut stack: Vec<String> = Vec::new();
    for tag in xml.split('<').skip(2) {
        let body = tag.split('>').next().unwrap();
        if let Some(name) = body.strip_prefix('/') {
            assert_eq!(stack.pop().as_deref(), Some(name), "{xml}");
        } else if !body.ends_with('/') {
            stack.push(body.split_whitespace().next().unwrap().to_string());
        }
    }
    assert!(stack.is_empty(), "{stack:?}");
    assert!(notes.iter().any(|n| n.contains("Tool Library")));
    // an unspecified format: refused, naming what is written
    let other = planned("trumpf-trubend-5130") + "prog = program(format: \"delem\")\n";
    let e = run(&other, &d).errors;
    assert!(
        e.iter()
            .any(|(_, t)| t.contains("ProgramFormatUnavailable") && t.contains("cincinnati"))
    );
}

/// Re-planning keeps what the user set (David: "Manual is also good. Don't delete manual to get
/// automatic"): the stations byte for byte, a locked bend at its station; only the bend
/// statements are rewritten. A lock the plan cannot pass is refused by name, the document left as
/// it was.
#[test]
fn a_replan_keeps_the_users_stations_and_locks() {
    let d = dir("replan");
    let doc = "bending\n\
# my layout: the gooseneck on the right\n\
part = part(\"channel.fg\")\n\
press = tooling(\"std-eu\")\n\
s1 = station(punch: \"std-eu/P85-R08-H120\", die: \"std-eu/V12-88\", y: -600, length: 200)\n\
s2 = station(punch: \"std-eu/G88-R08\", die: \"std-eu/V12-88\", y: 600, length: 200)\n\
b1 = bend(\"right\", at: s1)\n\
b2 = bend(\"left\", at: s2, lock: true)\n\
sheet = setup_sheet()\n";
    let r = read(doc).unwrap();
    assert!(r.bends[1].lock && !r.bends[0].lock);
    let (new, sim) = forge_bend_lang::replan(doc, CHANNEL, Some(&d)).expect("a plan");
    assert!(sim.plan.feasible);
    // everything but the bend statements as it was
    let keep = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| !l.contains("= bend("))
            .map(String::from)
            .collect()
    };
    assert_eq!(keep(doc), keep(&new), "{new}");
    let r2 = read(&new).unwrap();
    let left = r2.bends.iter().find(|b| b.bend == "left").unwrap();
    assert_eq!((left.station.as_deref(), left.lock), (Some("s2"), true), "{new}");
    // the planner's own choice for the unlocked bend: the straight punch, station 1
    let right = r2.bends.iter().find(|b| b.bend == "right").unwrap();
    assert_eq!(
        (right.station.as_deref(), right.lock),
        (Some("s1"), false),
        "{new}"
    );
    // the new document runs as stated, the same plan
    let b = forge_bend_lang::run(&new, &d);
    assert!(b.ok, "{:?}", b.errors);
    // a lock the plan cannot pass: station 2's stacks (50 mm) cannot cover the 200 mm bend
    let bad = doc.replace("y: 600, length: 200", "y: 600, length: 50");
    match forge_bend_lang::replan(&bad, CHANNEL, Some(&d)) {
        Ok((t, s)) => assert!(!s.plan.feasible, "expected a refusal: {t}"),
        Err(why) => {
            assert!(
                why.iter()
                    .any(|(at, w)| at == "b2" && w.contains("locked to s2") && w.contains("unlock")),
                "{why:?}"
            );
        }
    }
}
