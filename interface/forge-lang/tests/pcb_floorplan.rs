use forge_agent::json::Json;
use forge_lang::pcb;
use std::path::Path;

const BOARD: &str = r#"pcb
src=schematic(file:"adc-fanout.fg")
perimeter=edge(points:[(0,0),(50,0),(50,25),(0,25)])
top=copper(thickness:0.035mm,weight_oz_ft2:1)
core=dielectric(material:"FR-4",thickness:1mm)
plane=copper(kind:"plane",thickness:0.035mm,weight_oz_ft2:1)
stack=stackup(layers:[top,core,plane])
a=footprint(pads:[smd(number:"1",at:(0,0),size:(1,1))],courtyard:[(-1,-1),(1,-1),(1,1),(-1,1)])
mon=footprint(pads:[smd(number:"2",at:(0,0),size:(1,1))],courtyard:[(-1,-1),(1,-1),(1,1),(-1,1)])
b=footprint(pads:[smd(number:"7",at:(0,0),size:(1,1))],courtyard:[(-1,-1),(1,-1),(1,1),(-1,1)])
sensor=place(side:"top",footprint:a,occurrence:"main/U1",at:(5,10))
adc=place(side:"top",footprint:b,occurrence:"main/U2",at:(35,10))
monitor=place(side:"top",footprint:mon,occurrence:"main/U3",at:(45,10))
probe=place(side:"top",footprint:a,occurrence:"main/TP1",at:(15,10))
ground=net(name:"reference")
pour=zone(net:ground,layer:plane,clearance:0.2mm,points:[(1,1),(49,1),(49,24),(1,24)])
slot=keepout(layer:plane,points:[(20,1),(23,1),(23,24),(20,24)])
"#;
fn query(src: &str, args: &[&str]) -> Json {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/electronics"));
    pcb::query(
        src,
        Some(dir),
        None,
        &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )
    .unwrap()
}
fn array<'a>(obj: &'a Json, key: &str) -> &'a [Json] {
    let Json::Arr(items) = obj.get(key).unwrap() else {
        panic!("{key}")
    };
    items
}
#[test]
fn swapping_a_passive_tap_and_an_adc_changes_priority_with_identical_net_geometry() {
    let r = query(
        BOARD,
        &[
            "floorplan",
            "variant=no_monitor",
            "swap@adc=-20,0@probe=20,0",
            "collision@adc=-30,0",
            "outside@adc=900,0",
        ],
    );
    let result = r.get("result").unwrap();
    assert_eq!(result.get("winner").unwrap().str(), Some("swap"));
    let rows = array(result, "candidates");
    let term = |i: usize, id: &str| {
        array(&rows[i], "terms")
            .iter()
            .find(|x| x.get("id").unwrap().str() == Some(id))
            .unwrap()
    };
    let evidence = |i: usize| term(i, "signal/main/U1.OUT/main/U2.P0").get("evidence").unwrap();
    assert_eq!(evidence(0).get("length_mm").unwrap().num(), Some(30.));
    assert_eq!(evidence(1).get("length_mm").unwrap().num(), Some(10.));
    assert!((evidence(0).get("uncovered_guide_mm").unwrap().num().unwrap() - 3.4).abs() < 0.005);
    assert_eq!(evidence(1).get("uncovered_guide_mm").unwrap().num(), Some(0.));
    let net = array(&rows[0], "terms")
        .iter()
        .find_map(|t| t.get("id")?.str().filter(|s| s.starts_with("net/")))
        .unwrap();
    assert_eq!(term(1, net).get("delta").unwrap().num(), Some(0.));
    assert!(
        array(&rows[0], "terms")
            .iter()
            .all(|t| !t.get("id").unwrap().str().unwrap().contains("U3"))
    );
    assert!(!array(&rows[2], "blockers").is_empty());
    assert!(!array(&rows[3], "blockers").is_empty());
}
#[test]
fn duplicate_groups_and_unknown_groups_fail_without_rewriting_the_board() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/electronics"));
    for spec in [
        "move@main=1,0@adc=1,0",
        "move@absent=0,1",
        "base@adc=0,1",
        "move@adc=NaN,0",
    ] {
        assert_eq!(
            pcb::query(BOARD, Some(dir), None, &["floorplan".into(), spec.into()])
                .unwrap_err()
                .kind,
            "FloorplanCandidate"
        );
    }
}

#[test]
fn changing_the_active_pin_function_removes_only_the_incompatible_branch() {
    let dir = std::env::temp_dir().join(format!("forge-floorplan-function-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let schematic = include_str!("../../../examples/electronics/adc-fanout.fg");
    std::fs::write(
        dir.join("adc-fanout.fg"),
        schematic.replace("functions:{P0:ADC}", "functions:{P0:GPIO}"),
    )
    .unwrap();
    let report = pcb::query(BOARD, Some(&dir), None, &["floorplan".into()]).unwrap();
    std::fs::remove_file(dir.join("adc-fanout.fg")).unwrap();
    std::fs::remove_dir(dir).unwrap();
    let base = &array(report.get("result").unwrap(), "candidates")[0];
    let ids: Vec<_> = array(base, "terms")
        .iter()
        .map(|t| t.get("id").unwrap().str().unwrap())
        .collect();
    assert!(ids.contains(&"signal/main/U1.OUT/main/U3.IN"));
    assert!(!ids.contains(&"signal/main/U1.OUT/main/U2.P0"));
    assert!(
        array(base, "missing")
            .iter()
            .any(|m| m.str().unwrap().contains("U2.P0: mismatch"))
    );
}

#[test]
fn repacking_separates_coincident_courtyards_and_keeps_a_locked_connector() {
    let src = BOARD.replace("main/U2\",at:(35,10)", "main/U2\",at:(5,10)");
    let report = query(&src, &["repack", "fixed=sensor", "gap=0.5"]);
    let result = report.get("result").unwrap();
    let rows = array(result, "candidates");
    assert!(!array(&rows[0], "blockers").is_empty());
    assert!(array(&rows[1], "blockers").is_empty());
    assert!(rows[1].get("moves_mm").unwrap().get("sensor").is_none());
    assert!(rows[1].get("moves_mm").unwrap().get("adc").is_some());
    assert_eq!(result.get("winner").unwrap().str(), Some("packed"));
}

#[test]
fn independent_block_versions_are_combined_and_rejected_poses_do_not_hide_siblings() {
    let r = query(
        BOARD,
        &[
            "repack",
            "fixed=adc,probe",
            "rotate",
            "channel:straight",
            "channel:turned@adc=10,10,90",
            "tap:original",
            "tap:collision@probe=10,10,0",
        ],
    );
    let result = r.get("result").unwrap();
    let rows = array(result, "candidates");
    assert_eq!(rows.len(), 4); // original board and three successfully packed combinations
    let failed = array(result, "packing_failures");
    assert_eq!(failed.len(), 1);
    assert_eq!(
        failed[0].get("name").unwrap().str(),
        Some("channel:turned+tap:collision")
    );
    let turned = rows
        .iter()
        .find(|r| r.get("name").unwrap().str() == Some("channel:turned+tap:original"))
        .unwrap();
    assert_eq!(
        turned.get("angles_deg").unwrap().get("adc").unwrap().num(),
        Some(90.)
    );
    assert!(array(turned, "blockers").is_empty());
}

#[test]
fn refinement_orients_an_off_center_pad_and_rejects_an_overlapping_seed() {
    use pcb::packing::{Model, Rect, Seed};
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/electronics"));
    let src = BOARD.replace(
        "b=footprint(pads:[smd(number:\"7\",at:(0,0)",
        "b=footprint(pads:[smd(number:\"7\",at:(0.4,0)",
    );
    let board = pcb::compile(&src, Some(dir), None).unwrap();
    let mut model = Model::new(&board).unwrap();
    model.add_guide(&board, "adc.7", "sensor.1", 100.).unwrap();
    let mut seeds: Vec<_> = model
        .ids
        .iter()
        .map(|id| Seed {
            at: board.layout.placements[id].pose.at.map(|n| n as f64 / 1e6),
            turns: 0,
            fixed: id != "adc",
        })
        .collect();
    let adc = model.ids.iter().position(|k| k == "adc").unwrap();
    let sensor = model.ids.iter().position(|k| k == "sensor").unwrap();
    let room = Rect {
        lo: [0., 0.],
        hi: [50., 25.],
    };
    let allowed = vec![true; seeds.len()];
    let score = |s: &[Seed]| model.estimate(&s.iter().map(|p| p.at).collect::<Vec<_>>(), s);
    let before = score(&seeds);
    model.polish(&mut seeds, room, &[], 0.5, &allowed).unwrap();
    assert!(score(&seeds) < before);
    assert_eq!(seeds[sensor].at, [5., 10.]);
    assert_eq!(seeds[adc].turns, 2);
    seeds[adc].at = seeds[sensor].at;
    assert!(model.polish(&mut seeds, room, &[], 0.5, &allowed).is_err());
}

#[test]
fn two_terminal_clock_reserves_local_space_and_reports_obstructions_by_reference() {
    let root = std::env::temp_dir().join(format!("forge-local-clock-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let sch = include_str!("../../../examples/electronics/adc-fanout.fg").replacen(
        "kind:\"signal\"",
        "kind:\"clock\"",
        1,
    );
    std::fs::write(root.join("adc-fanout.fg"), &sch).unwrap();
    // Both receivers are active. A distributed clock retains its ordinary electrical guides.
    let run = |args: &[&str]| {
        pcb::query(
            BOARD,
            Some(&root),
            None,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
        .unwrap()
    };
    assert!(array(run(&["repack"]).get("result").unwrap(), "local_circuits").is_empty());
    std::fs::write(
        root.join("adc-fanout.fg"),
        sch.replace(
            "kind:\"input\",at:(-2,0),wants:[measurement]",
            "kind:\"input\",at:(-2,0),wants:[logic]",
        ),
    )
    .unwrap();
    let r = run(&["repack", "fixed=sensor"]);
    let result = r.get("result").unwrap();
    assert_eq!(array(result, "local_circuits").len(), 1);
    let rows = array(result, "candidates");
    let term = |i: usize| {
        array(&rows[i], "terms")
            .iter()
            .find(|v| v.get("id").unwrap().str() == Some("signal/main/U1.OUT/main/U2.P0"))
            .unwrap()
            .get("evidence")
            .unwrap()
    };
    assert_eq!(array(term(0), "crossed_courtyards")[0].str(), Some("probe"));
    assert!(array(term(1), "crossed_courtyards").is_empty());
    assert!(term(1).get("length_mm").unwrap().num().unwrap() < 6.);
    assert!(rows[1].get("moves_mm").unwrap().get("sensor").is_none());
    std::fs::remove_dir_all(root).unwrap();
}
