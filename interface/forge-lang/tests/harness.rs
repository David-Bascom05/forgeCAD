//! Wire harnesses in assemblies: examples/harness routed (both forms, the same lengths), the typed
//! warnings (BendTooTight, KinkAtNode, ..) and errors, the wire library, routes following a
//! moved part, and the wire-list CSV out and back in.

use forge_lang::asm::Issue;
use forge_lang::error::{Error, Warning};
use forge_lang::harness::{self, HarnessOut, HarnessProblem};
use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};

const HEADER: &str = include_str!("../../../examples/harness/header.fg");
const PLATE: &str = include_str!("../../../examples/harness/plate.fg");
const ROBOT: &str = include_str!("../../../examples/harness/robot.fg");
const ROBOT_LOOM: &str = include_str!("../../../examples/harness/robot_loom.fg");
const LOOM: &str = include_str!("../../../examples/harness/loom.fg");

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("harness-{name}"));
    let _ = std::fs::create_dir_all(&d);
    for (f, t) in [("header.fg", HEADER), ("plate.fg", PLATE), ("loom.fg", LOOM)] {
        std::fs::write(d.join(f), t).expect("write a fixture");
    }
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn problems(r: &Run) -> Vec<HarnessProblem> {
    let of = |n: &forge_lang::asm::AsmNote| match &n.issue {
        Issue::Harness(p) => Some(p.clone()),
        _ => None,
    };
    r.errors
        .iter()
        .filter_map(|e| match e {
            Error::Asm(n) => of(n),
            _ => None,
        })
        .chain(r.warnings.iter().filter_map(|w| match w {
            Warning::Asm(n) => of(n),
            _ => None,
        }))
        .collect()
}

fn errors(r: &Run) -> Vec<String> {
    r.errors
        .iter()
        .map(|e| format!("{} {}", e.kind(), e.text()))
        .collect()
}

fn loom(r: &Run) -> &HarnessOut {
    &r.asm.as_ref().expect("an assembly").harnesses[0]
}

#[test]
fn the_example_routes_with_every_term_and_no_notes() {
    let d = dir("example");
    let r = run_in(&d, ROBOT);
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    // no wire runs into a part; where wires still pass through each other it is only where they
    // change places (a breakout, a branch), never along a bundle's run
    let h = loom(&r);
    for p in problems(&r) {
        let HarnessProblem::WireClash { with, at, .. } = p else {
            panic!("{p:?}");
        };
        assert!(with.starts_with('W'), "a wire runs into {with} at {at:?}");
        let near = h.nodes.iter().any(|n| n.at.distance(at) < 3.0 * 8.6 + 40.0);
        assert!(near, "a clash along a run at {at:?}");
    }
    assert_eq!(h.connectors.len(), 4);
    assert_eq!(h.wires.len(), 14);
    let trunk = &h.segments[0];
    assert_eq!(trunk.wires.len(), 14);
    // four 2.4, four 1.6 and six 1.1 wires: sqrt(1.5 sum d^2), and the braid's wall each side
    let d_trunk = (1.5f64 * (4.0 * 2.4 * 2.4 + 4.0 * 1.6 * 1.6 + 6.0 * 1.1 * 1.1)).sqrt() + 2.0 * 0.4;
    assert!((trunk.d - d_trunk).abs() < 1e-12, "{}", trunk.d);
    assert!((trunk.r_min - 3.0 * d_trunk).abs() < 1e-12);
    // the wires packed: wider than the rule's bundle on the motor branches, never under it on the trunk
    assert!(
        trunk.d_pack > 7.0 && trunk.d_pack < trunk.d,
        "{} {}",
        trunk.d_pack,
        trunk.d
    );
    for w in &h.wires {
        let o = w.routed.as_ref().expect("routed");
        assert_eq!(o.length % 5.0, 0.0);
        assert!(o.length >= o.exact && o.exact - o.route - o.slack - o.insertion < 1e-9);
        assert_eq!(o.insertion, 8.0);
    }
    assert!(h.wires[..4].iter().all(|w| w.path == ["trunk", "to_a"]));
    assert!(h.wires[4..8].iter().all(|w| w.path == ["trunk", "span", "to_b"]));
    assert!(h.wires[8..].iter().all(|w| w.path == ["trunk", "span", "to_s"]));
    // the BOM: wire by type and colour (what is ordered), the braid by length, the connectors
    let p18_rd: f64 = h
        .wires
        .iter()
        .filter(|w| w.ty == "p18" && w.colour == "RD")
        .map(|w| w.routed.as_ref().unwrap().length)
        .sum();
    assert_eq!(h.bom[0].item, "wire p18 RD");
    assert_eq!(h.bom[0].qty, p18_rd);
    assert!(h.bom.iter().any(|b| b.item == "covering PET braid 8"));
    assert_eq!(
        h.bom.iter().filter(|b| b.item.starts_with("connector ")).count(),
        4
    );
    // the spec: worked-out resistance, a voltage drop where a current is given
    let p18 = h.types.iter().find(|t| t.name == "p18").unwrap();
    assert_eq!(p18.get("spec"), "UL 1015");
    let area = std::f64::consts::PI / 4.0 * forge_harness::awg_d(18.0).powi(2);
    assert!((p18.ohm_km.unwrap() - 1000.0 / 58.0 / area).abs() < 1e-9 && p18.ohm_worked);
    let w1 = &h.wires[0];
    let len = w1.routed.as_ref().unwrap().length;
    assert!((w1.ohms.unwrap() - p18.ohm_km.unwrap() * len / 1e6).abs() < 1e-12);
    assert!((w1.vdrop.unwrap() - 3.0 * w1.ohms.unwrap()).abs() < 1e-12);
    assert!(h.wires[2].vdrop.is_none());
}

#[test]
fn a_harness_file_placed_routes_as_the_block_does() {
    let d = dir("file");
    let a = run_in(&d, ROBOT);
    let b = run_in(&d, ROBOT_LOOM);
    assert!(b.errors.is_empty(), "{:?}", errors(&b));
    let (ha, hb) = (loom(&a), loom(&b));
    assert_eq!(hb.file.as_deref(), Some("loom.fg"));
    assert_eq!(ha.wires, hb.wires);
    assert_eq!(ha.segments, hb.segments);
}

#[test]
fn a_harness_file_alone_says_it_is_placed() {
    let d = dir("alone");
    let r = run_in(&d, LOOM);
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    assert!(
        matches!(&problems(&r)[..], [HarnessProblem::PlacedOnly { occurrences }] if occurrences.len() == 5)
    );
}

#[test]
fn a_placement_missing_an_occurrence_is_typed() {
    let d = dir("missing");
    let src = ROBOT_LOOM.replace(", plate: plate)", ")");
    let r = run_in(&d, &src);
    assert!(
        problems(&r)
            .iter()
            .any(|p| matches!(p, HarnessProblem::Occurrences { missing, .. } if missing == &["plate"])),
        "{:?}",
        errors(&r)
    );
}

#[test]
fn a_corner_too_tight_builds_and_warns_with_the_radii() {
    let d = dir("tight");
    // the controller's exit with too short a lead for the turn off it
    let src = ROBOT.replace(
        "n1 = node(J1, out: out_ctrl)",
        "n1 = node(J1, out: out_ctrl, lead: seat_in)",
    );
    let r = run_in(&d, &src);
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    let tight: Vec<_> = problems(&r)
        .into_iter()
        .filter_map(|p| match p {
            HarnessProblem::BendTooTight {
                segment, needed, got, ..
            } => Some((segment, needed, got)),
            _ => None,
        })
        .collect();
    assert!(!tight.is_empty());
    for (_, needed, got) in &tight {
        assert!(got < needed);
    }
    assert!(tight.iter().any(|t| t.0 == "trunk"));
    // it still routes every wire
    assert!(loom(&r).wires.iter().all(|w| w.routed.is_some()));
}

#[test]
fn a_turn_through_a_free_node_says_how_far_and_which_way() {
    let d = dir("kink");
    let src = ROBOT.replace(
        "hub1 = node(plate.clip1, along: \"x\", clip: true)",
        "hub1 = node(plate.clip1, clip: true)",
    );
    let r = run_in(&d, &src);
    let kinks: Vec<_> = problems(&r)
        .into_iter()
        .filter_map(|p| match p {
            HarnessProblem::KinkAtNode {
                node,
                wire,
                angle,
                toward,
                ..
            } => Some((node, wire, angle, toward)),
            _ => None,
        })
        .collect();
    // the four wires turning off to the first motor turn hard there
    let hard: Vec<_> = kinks.iter().filter(|k| k.2 > 0.1).collect();
    assert_eq!(hard.len(), 4, "{kinks:?}");
    for (node, wire, angle, toward) in &kinks {
        assert_eq!(node, "hub1");
        assert!(*angle < 3.0);
        assert!((toward.length() - 1.0).abs() < 1e-12);
        let _ = wire;
    }
    let text = r
        .warnings
        .iter()
        .find(|w| matches!(w, Warning::Asm(n) if n.kind() == "KinkAtNode"))
        .map(|w| match w {
            Warning::Asm(n) => n.text(),
            _ => unreachable!(),
        })
        .unwrap();
    assert!(text.contains("deg at loom.hub1 toward"), "{text}");
    assert!(text.contains("add along:"), "{text}");
}

#[test]
fn a_pin_the_part_does_not_have_is_typed_with_the_ones_it_does() {
    let d = dir("pin");
    let src = ROBOT.replace("W4 = wire(J1[4], J2[4]", "W4 = wire(J1[4], J2[9]");
    let r = run_in(&d, &src);
    let p = problems(&r)
        .into_iter()
        .find(|p| matches!(p, HarnessProblem::PinUnknown { .. }))
        .unwrap_or_else(|| panic!("{:?}", errors(&r)));
    let HarnessProblem::PinUnknown {
        connector,
        pin,
        known,
        ..
    } = p
    else {
        unreachable!()
    };
    assert_eq!((connector.as_str(), pin.as_str()), ("J2", "9"));
    // the pins the harness reaches that the part has
    assert!(
        known.iter().any(|k| k == "1") && !known.iter().any(|k| k == "9"),
        "{known:?}"
    );
}

#[test]
fn a_connector_with_no_exit_node_is_typed() {
    let d = dir("exit");
    let src = ROBOT
        .replace("  n3 = node(J3, out: out)\n", "")
        .replace("  to_b = segment(hub2, n3)\n", "");
    let r = run_in(&d, &src);
    assert!(
        problems(&r)
            .iter()
            .any(|p| matches!(p, HarnessProblem::NoExitNode { connector, .. } if connector == "J3"))
    );
}

#[test]
fn a_wire_no_segment_reaches_is_typed_and_the_rest_route() {
    let d = dir("nopath");
    let src = ROBOT.replace("  to_s = segment(hub2, n4)\n", "");
    let r = run_in(&d, &src);
    let no: Vec<_> = problems(&r)
        .into_iter()
        .filter(|p| matches!(p, HarnessProblem::NoPath { .. }))
        .collect();
    assert_eq!(no.len(), 6);
    assert!(loom(&r).wires[..8].iter().all(|w| w.routed.is_some()));
}

#[test]
fn routes_follow_the_parts_they_are_on() {
    let d = dir("moves");
    let a = run_in(&d, ROBOT);
    // the plug's seat moved along x: its exit node, and the branch to it, go with it
    let moved = ROBOT.replace(
        "plate = part(\"plate.fg\", ground: true)",
        "plate = part(\"plate.fg\", ground: true, with: {ax: 10})",
    );
    let b = run_in(&d, &moved);
    assert!(b.errors.is_empty(), "{:?}", errors(&b));
    let n = |r: &Run| loom(r).nodes.iter().find(|n| n.name == "n2").unwrap().at;
    assert!((n(&a).x - 30.0).abs() < 1e-9 && (n(&b).x - 10.0).abs() < 1e-9);
    assert!(loom(&b).segments[2].length < loom(&a).segments[2].length);
}

#[test]
fn wire_types_come_from_the_library_with_their_source() {
    let d = dir("library");
    let lib = d.join("wires");
    let _ = std::fs::create_dir_all(&lib);
    std::fs::write(
        lib.join("shop.csv"),
        "name,awg,mm2,od,min_bend,colour,source\nT22-WH,22,,1.55,6,WH,test row\n",
    )
    .unwrap();
    let line = ROBOT
        .lines()
        .find(|l| l.trim_start().starts_with("s22 = wire_type("))
        .unwrap();
    let src = ROBOT.replace(line, "  s22 = wire_type(\"T22-WH\")");
    let r = run_in(&d, &src);
    assert!(r.errors.is_empty(), "{:?}", errors(&r));
    assert_eq!(loom(&r).wires[2].od, 1.55);
    let unknown = run_in(&d, &src.replace("\"T22-WH\"", "\"T20\""));
    assert!(problems(&unknown).iter().any(
        |p| matches!(p, HarnessProblem::UnknownWireType { name, known } if name == "T20" && known == &["T22-WH"])
    ));
    std::fs::write(lib.join("shop.csv"), "T22-WH,22,,1.55,6,WH,\n").unwrap();
    let unsourced = run_in(&d, &src);
    assert!(
        problems(&unsourced)
            .iter()
            .any(|p| matches!(p, HarnessProblem::WireRowUnsourced { line: 1, .. }))
    );
    let _ = std::fs::write(lib.join("shop.csv"), "");
}

#[test]
fn a_gauge_with_no_od_uses_the_conductor_and_says_so() {
    let d = dir("od");
    let src = ROBOT.replace(
        "s26 = wire_type(awg: g26, od: od26, min_bend: bend26",
        "s26 = wire_type(awg: g26, min_bend: bend26",
    );
    let r = run_in(&d, &src);
    let c = problems(&r)
        .into_iter()
        .find_map(|p| match p {
            HarnessProblem::NoInsulationOd { conductor, .. } => Some(conductor),
            _ => None,
        })
        .expect("NoInsulationOd");
    assert!((c - forge_harness::awg_d(26.0)).abs() < 1e-15);
    assert_eq!(loom(&r).wires[8].od, c);
}

#[test]
fn the_wire_list_goes_out_and_back_in_unchanged_and_a_change_is_a_diff() {
    let d = dir("csv");
    let r = run_in(&d, ROBOT);
    let csv = harness::export_csv(loom(&r));
    assert!(csv.starts_with("id,from,to,type,colour,path,route"));
    assert_eq!(csv.lines().count(), 15);
    let (ops, diff) = harness::import(ROBOT, "loom", &csv, Some(loom(&r))).unwrap();
    assert!(ops.is_empty() && diff == Default::default(), "{ops:?} {diff:?}");
    // W8 moves to another pin, W15 is new, W1 is gone
    let edited: String = csv
        .lines()
        .filter(|l| !l.starts_with("W1,"))
        .map(|l| {
            if l.starts_with("W8,") {
                l.replace("J3.4", "J3.3")
            } else {
                l.to_string()
            }
        })
        .chain(["W15,J1.14,J4.6,s26,VT,trunk;span;to_s".to_string()])
        .collect::<Vec<_>>()
        .join("\n");
    let (ops, diff) = harness::import(ROBOT, "loom", &edited, Some(loom(&r))).unwrap();
    assert_eq!(diff.added, ["W15"]);
    assert_eq!(diff.removed, ["W1"]);
    assert_eq!(
        diff.changed,
        vec![("W8".to_string(), vec![("to", "J3.4".into(), "J3.3".into())])]
    );
    assert_eq!(diff.text(), "+ W15\n- W1\n~ W8 to: J3.4 -> J3.3\n");
    let mut src = ROBOT.to_string();
    for op in &ops {
        src = forge_lang::edit::apply(&src, op).unwrap_or_else(|e| panic!("{op:?}: {}", e.text()));
    }
    assert!(
        src.contains(
            "  W15 = wire(J1[14], J4[6], type: s26, through: [trunk, span, to_s], colour: \"VT\")\n"
        ),
        "{src}"
    );
    assert!(src.contains("W8 = wire(J1[8], J3[3], type: s22"));
    assert!(!src.contains("W1 ="));
    let after = run_in(&d, &src);
    assert!(after.errors.is_empty(), "{:?}", errors(&after));
    let h = loom(&after);
    assert_eq!(h.wires.len(), 14);
    assert_eq!(h.wires.last().unwrap().path, ["trunk", "span", "to_s"]);
    // and the new list is what it now holds
    let (ops, _) = harness::import(&src, "loom", &edited, Some(loom(&after))).unwrap();
    assert!(ops.is_empty(), "{ops:?}");
}

#[test]
fn the_example_rebuilds_in_milliseconds() {
    let d = dir("speed");
    let _ = run_in(&d, ROBOT);
    let t = std::time::Instant::now();
    let r = run_in(&d, ROBOT);
    let ms = t.elapsed().as_secs_f64() * 1e3;
    assert!(r.errors.is_empty());
    // the whole document: parts loaded, solved, the harness routed
    assert!(ms < 1500.0, "{ms} ms");
}

#[test]
fn route_wires_writes_the_chosen_paths_and_clears_them() {
    let d = dir("route");
    let r = run_in(&d, ROBOT);
    let ops = harness::route_ops(ROBOT, loom(&r), true).unwrap();
    assert_eq!(ops.len(), 14);
    let mut src = ROBOT.to_string();
    for op in &ops {
        src = forge_lang::edit::apply(&src, op).unwrap_or_else(|e| panic!("{op:?}: {}", e.text()));
    }
    assert!(
        src.contains(
            "  W1 = wire(J1[1], J2[1], type: p18, through: [trunk, to_a], colour: \"RD\", current: i_motor)\n"
        ),
        "{src}"
    );
    let pinned = run_in(&d, &src);
    assert!(pinned.errors.is_empty(), "{:?}", errors(&pinned));
    assert!(loom(&pinned).wires.iter().all(|w| w.given));
    assert_eq!(
        loom(&pinned)
            .wires
            .iter()
            .map(|w| w.routed.as_ref().unwrap().length)
            .collect::<Vec<_>>(),
        loom(&r)
            .wires
            .iter()
            .map(|w| w.routed.as_ref().unwrap().length)
            .collect::<Vec<_>>()
    );
    // written again: nothing to do; cleared: back to the shortest
    assert!(harness::route_ops(&src, loom(&pinned), true).unwrap().is_empty());
    let clear = harness::route_ops(&src, loom(&pinned), false).unwrap();
    for op in &clear {
        src = forge_lang::edit::apply(&src, op).unwrap();
    }
    assert_eq!(src, ROBOT);
}

#[test]
fn colour_codes_read_base_and_stripes_in_any_writing() {
    assert_eq!(harness::colour_codes("WH/BU"), ["WH", "BU"]);
    assert_eq!(harness::colour_codes("GN-YE"), ["GN", "YE"]);
    assert_eq!(harness::colour_codes("WHGN"), ["WH", "GN"]);
    assert_eq!(harness::colour_codes("RD/WH/BK/YE"), ["RD", "WH", "BK"]);
    assert_eq!(harness::colour_codes("#3060c0"), ["#3060c0"]);
    let c = harness::rgb("#3060c0", "W1");
    assert!((c[0] - 0x30 as f32 / 255.0).abs() < 1e-6 && (c[2] - 0xc0 as f32 / 255.0).abs() < 1e-6);
    assert_eq!(harness::rgb("WH/BU", "W1"), harness::rgb("WH", "W9"));
}

#[test]
fn a_library_row_carries_every_field_by_its_header() {
    let csv = "name,awg,conductor,strands,insulation,od,temp,volts,spec,spec_tag,maker,mpn,source\n\
               T18,18,tinned Cu,16,XLPE,2.1,125,600,SAE J1128 TXL,TXL,ExampleCo,EX-18,test row\n";
    let rows = harness::parse_library("lib.csv", csv).unwrap();
    assert_eq!(rows[0].get("insulation"), "XLPE");
    assert_eq!(rows[0].get("spec"), "SAE J1128 TXL");
    assert_eq!(rows[0].spec.get("od").and_then(|v| v.num()), Some(2.1));
    // tinned copper's resistivity is the maker's: none worked out
    assert!(harness::ohm_km(&rows[0].spec).is_none());
    let bad = harness::parse_library("lib.csv", "name,gauge,source\nX,18,r\n");
    assert!(matches!(bad, Err(HarnessProblem::WireRowBad { .. })));
}

#[test]
fn two_wires_on_one_pin_are_typed_and_a_cable_routes_with_its_assembly_length() {
    let d = dir("pin-taken");
    let src = ROBOT.replace(
        "  W14 = wire(J1[14], J4[6], type: s26, colour: \"VT\")\n",
        "  W14 = wire(J1[14], J4[6], type: s26, colour: \"VT\")\n  W15 = wire(J1[14], J4[6], type: p18)\n",
    );
    let r = run_in(&d, &src);
    let ps = problems(&r);
    assert!(
        ps.iter().any(
            |p| matches!(p, HarnessProblem::PinTaken { connector, pin, wires, .. }
            if connector == "J1" && pin == "14" && wires == &["W14".to_string(), "W15".to_string()])
        ),
        "{ps:?}"
    );
    let h = &r.asm.as_ref().unwrap().harnesses[1];
    assert_eq!(h.id, "power");
    let c = h.wires.iter().find(|w| w.name == "C1").expect("the cable");
    assert_eq!((c.from.as_str(), c.to.as_str()), ("P1", "P2"));
    assert!(c.routed.is_some());
    assert!(
        h.bom
            .iter()
            .any(|b| b.item.starts_with("cable assembly pow") && b.qty == 1.0),
        "{:?}",
        h.bom
    );
}
