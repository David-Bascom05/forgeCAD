use forge_lang::schematic::{self, Circuit, Document};
use std::path::Path;

const LIB: &str = include_str!("../../../examples/electronics/power-symbols.fg");
const DESIGN: &str = include_str!("../../../examples/electronics/controller.fg");
fn run(src: &str) -> schematic::Result<(Document, Circuit)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics");
    schematic::compile(src, Some(&dir), Some(&dir))
}

#[test]
fn roles_are_nominal_typed_and_separate_from_nets() {
    let (d, c) = run(DESIGN).unwrap();
    assert_eq!(c.nets.len(), 2);
    assert_eq!(c.links.len(), 2);
    let t = &c.terminals["main/reg.out"];
    assert_eq!(t.wants.len(), 1);
    assert_eq!(t.gives.len(), 1);
    let info = schematic::response(&d, &c, &["pin".into(), "main/reg.out".into()])
        .unwrap()
        .to_string();
    assert!(
        info.contains("@examples.bypass100n") && info.contains("capacitance") && info.contains("voltage")
    );
    let no_links = DESIGN.split("bypass = fulfill").next().unwrap();
    assert_eq!(run(no_links).unwrap().1.net_of, c.net_of);
    let unknown = DESIGN.replace("role: parts.local_bypass", "role: parts.typo");
    assert_eq!(run(&unknown).unwrap_err().kind, "UnknownRole");
    let other_type = DESIGN
        .replace(
            "main = sheet",
            "other = role(kind: \"bypass\", capacitance_nf: 100)\nmain = sheet",
        )
        .replace("role: parts.local_bypass", "role: other");
    assert_eq!(run(&other_type).unwrap_err().kind, "RoleMismatch");
    let bad = DESIGN.replace("role: parts.local_bypass", "role: parts.supply_3v3");
    assert_eq!(run(&bad).unwrap_err().kind, "RoleMismatch");
    let bad = DESIGN.replace("role: parts.local_bypass", "role: \"bypass\"");
    assert_eq!(run(&bad).unwrap_err().kind, "ExpectedReference");
    let swapped = DESIGN.replace("from: u.vdd, to: c.plus", "from: c.plus, to: u.vdd");
    assert_eq!(run(&swapped).unwrap().1.net_of, c.net_of);
    let no_return = DESIGN.replace("return_to: u.vss", "return_to: u.vdd");
    assert!(
        run(&no_return)
            .unwrap()
            .1
            .issues
            .iter()
            .any(|i| i.kind == "ReturnMismatch")
    );
    assert_eq!(run(&DESIGN.replace("3mm", "3deg")).unwrap_err().kind, "WrongDim");
}
#[test]
fn roles_from_distinct_libraries_share_only_explicit_compatible_keys() {
    let src = DESIGN
        .replace(
            "shared spacing",
            "alt = library(file: \"power-symbols.fg\")\nshared spacing",
        )
        .replace("symbol: parts.Cap", "symbol: alt.Cap");
    assert!(run(&src).is_ok());
    let conflict = "symbol_library\nx = role(kind: \"power\", key: \"x\", voltage_mv: 3300)\ny = role(kind: \"power\", key: \"x\", voltage_mv: 5000)";
    assert_eq!(run(conflict).unwrap_err().kind, "ConflictingRoleDefinition");
    assert_eq!(
        run(&LIB.replace("capacitance_nf: 100", "voltage_mv: 100"))
            .unwrap_err()
            .kind,
        "InvalidRoleRating"
    );
}
#[test]
fn hierarchy_scopes_labels_and_qualifies_terminals() {
    let s = "schematic
parts = library(file: \"power-symbols.fg\")
top = sheet()
cell = sheet()
u = place(sheet: cell, symbol: parts.MCU, at: (0,0))
out = port(sheet: cell, at: (0,1))
w = wire(sheet: cell, from: u.vdd, to: out)
local = label(sheet: cell, on: out, value: \"V\", at: (0,2))
g = label(sheet: cell, on: u.vss, value: \"GND\", global: true, at: (0,3))
a = subsheet(sheet: top, of: cell, at: (0,0), ports: {out: (2,0)})
b = subsheet(sheet: top, of: cell, at: (10,0), ports: {out: (2,0)})
signals = bus(sheet: top, members: {first: a.out, second: b.out})
";
    let (_, c) = run(s).unwrap();
    assert_ne!(c.net_of["top/a/u.vdd"], c.net_of["top/b/u.vdd"]);
    assert_eq!(c.net_of["top/a/u.vss"], c.net_of["top/b/u.vss"]);
    assert_eq!(c.locations["top/b.out"], [12., 0.]);
    assert_eq!(c.buses["top/signals"].len(), 2);
    assert_eq!(
        run(&s.replace("of: cell", "of: top")).unwrap_err().kind,
        "SheetCycle"
    );
}
#[test]
fn units_mirror_flow_and_cycle() {
    let src="schematic
S = symbol(pins: {a: pin(unit: \"A\",number: \"1\", kind: \"input\", at: (2,0)), b: pin(unit: \"B\",number: \"2\",kind: \"output\",at: (1,0))})
top = sheet()
chip = place(sheet: top, symbol: S, at: (0,0), angle: 90deg)
second = gate(sheet: top, of: chip, unit: \"B\", at: (10, 0), mirror: true)
forward = flow(sheet: top, from: chip.a, to: chip.b)
back = flow(sheet: top, from: chip.b, to: chip.a)
";
    let (_, c) = run(src).unwrap();
    assert_eq!(c.locations["top/chip.a"], [0., 2.]);
    assert_eq!(c.locations["top/chip.b"], [9., 0.]);
    assert_eq!(c.nets.len(), 2);
    let steps = c.rooted("top/chip.a").unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!((steps[0].revisit, steps[1].revisit), (false, true));
    assert_eq!(
        run(&src.replace(
            "second = gate",
            "extra = gate(sheet: top, of: chip, unit: \"B\", at:(0,0))\nsecond = gate"
        ))
        .unwrap_err()
        .kind,
        "DuplicateGate"
    );
}
#[test]
fn partitions_match_transitive_closure_for_every_four_terminal_graph() {
    for mask in 0..64 {
        let mut s = "schematic\ntop=sheet()\n".to_string();
        for i in 0..4 {
            s += &format!("p{i}=junction(sheet:top,at:(0,0))\n");
        }
        let mut reach: [[bool; 4]; 4] = std::array::from_fn(|i| std::array::from_fn(|j| i == j));
        let edges = [(1, 0), (2, 0), (2, 1), (3, 0), (3, 1), (3, 2)];
        for (bit, (i, j)) in edges.into_iter().enumerate() {
            if mask & (1 << bit) != 0 {
                reach[i][j] = true;
                reach[j][i] = true;
                s += &format!("w{bit}=wire(sheet:top,from:p{i},to:p{j})\n");
            }
        }
        for k in 0..4 {
            for i in 0..4 {
                for j in 0..4 {
                    reach[i][j] |= reach[i][k] && reach[k][j];
                }
            }
        }
        let (_, c) = run(&s).unwrap();
        for (i, row) in reach.iter().enumerate() {
            for (j, connected) in row.iter().enumerate() {
                assert_eq!(
                    *connected,
                    c.net_of[&format!("top/p{i}")] == c.net_of[&format!("top/p{j}")]
                );
            }
        }
    }
}

#[test]
fn freehand_wiring_ranks_providers_and_reports_obscured_pin_text() {
    let src = DESIGN.replace("from: u.vdd, to: c.plus", "from: u.vdd, to: j")
        + "\nj = junction(sheet: main, at: (5,6))\nbranch = wire(sheet: main, from: j, to: c.plus)\nremote = place(sheet: main, symbol: parts.Cap, at: (40, 0))\nremote_rail = label(sheet: main, on: remote.plus, value: \"3V3\", at: (40,3))\ncaption = note(sheet: main, value: \"pin overlap\", at: (0.8,4))\n";
    let (d, c) = run(&src).unwrap();
    let result = schematic::response(&d, &c, &["pin".into(), "main/u.vdd".into()]).unwrap();
    let Some(forge_agent::json::Json::Arr(candidates)) = result.get("result").unwrap().get("candidates")
    else {
        panic!("{result}")
    };
    let evidence = |pin: &str| {
        candidates
            .iter()
            .find(|x| x.get("give").and_then(|v| v.str()) == Some(pin))
            .unwrap()
            .get("evidence")
            .unwrap()
            .str()
            .unwrap()
    };
    assert_eq!(evidence("main/c.plus"), "drawn_branch");
    assert_eq!(evidence("main/remote.plus"), "same_net");
    assert_eq!(evidence("main/reg.out"), "direct_wire");
    assert!(c.issues.iter().any(|i| i.kind == "TextBoundsOverlap"
        && i.objects.contains(&"main/caption".into())
        && i.objects.contains(&"main/u.vdd".into())));
}

#[test]
fn cached_moves_equal_fresh_results_in_every_sheet_occurrence() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics");
    let file = dir.join("repeated.fg");
    let mut cache = schematic::Cache::default();
    let base = format!(
        "{DESIGN}\ntop = sheet()\na = subsheet(sheet:top, of:main, at:(0,0))\nb = subsheet(sheet:top, of:main, at:(30,0))"
    );
    for step in 0..40 {
        let pos = match step % 6 {
            0 => "(0, 0)".into(),
            1 => "(0.5in, -2mm)".into(),
            2 => "(0.5mm, 0)".into(),
            3 => format!("({}mm, -1)", step * 7),
            4 => "(spacing + 2mm, 3mm)".into(),
            _ => "(0, 0deg)".into(),
        };
        let text = base.replace("at: (spacing, 0)", &format!("at: {pos}"));
        let fresh = run(&text);
        for query in [
            "erc",
            "sheet top/a",
            "svg top/b",
            "netlist",
            "components",
            "pin top/a/c.plus",
            "links",
        ] {
            let words = query.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
            let result = cache.query(&file, Some(&dir), &text, &words);
            match &fresh {
                Ok((doc, c)) => assert_eq!(
                    result.unwrap(),
                    schematic::response(doc, c, &words).unwrap(),
                    "step {step}: {query}"
                ),
                Err(fault) => assert_eq!(result.unwrap_err(), *fault),
            }
        }
    }
}

#[test]
fn local_labels_on_a_sheet_named_global_stay_local() {
    let src = r#"schematic
global = sheet()
a = junction(sheet: global, at: (0, 0))
b = junction(sheet: global, at: (5, 0))
la = label(sheet: global, on: a, value: "VCC", at: (0, 0))
lb = label(sheet: global, on: b, value: "VCC", global: true, at: (5, 0))
"#;
    let (_, c) = run(src).unwrap();
    assert_ne!(c.net_of["global/a"], c.net_of["global/b"]);
}

#[test]
fn a_return_pin_cannot_confirm_its_own_connection() {
    let src = DESIGN.replace("return_from: c.minus", "return_from: u.vss");
    let (_, c) = run(&src).unwrap();
    assert!(c.issues.iter().any(|i| i.kind == "ReturnMismatch"));
    assert!(c.issues.iter().any(|i| i.kind == "UnfulfilledWant"));
}

#[test]
fn fitted_bom_resolves_metadata_across_sheet_occurrences_and_gates() {
    let source = r#"schematic
parts = library(file: "analog/analog-symbols.fg")
S = symbol(pins: {a: pin(number: "1", kind: "input", at: (0,0)), b: pin(number: "2", kind: "output", unit: "B", at: (0,0))})
root = sheet()
cell = sheet()
left = subsheet(sheet: root, of: cell, at: (0,0))
right = subsheet(sheet: root, of: cell, at: (10,0))
U1 = place(sheet: cell, symbol: parts.Amplifier, at: (0,0), fields: {Manufacturer: "Local, Inc.", Package: "DBV"})
U2 = place(sheet: cell, symbol: S, at: (10,0))
U3 = place(sheet: cell, symbol: S, at: (50,0), footprint: "")
gateB = gate(sheet: cell, of: U2, unit: "B", at: (20,0))
spare = place(sheet: cell, symbol: S, at: (30,0), populated: false)
fixture = place(sheet: cell, symbol: S, at: (40,0), in_bom: false)
"#;
    let (d, c) = run(source).unwrap();
    let lines = schematic::bom(&d, &c);
    assert_eq!(lines.len(), 2);
    let local = lines.iter().find(|l| l.part.symbol == "S").unwrap();
    assert_eq!(local.references.len(), 4);
    let amp = lines.iter().find(|l| l.part.symbol == "parts.Amplifier").unwrap();
    assert_eq!(amp.references, ["root/left/U1", "root/right/U1"]);
    assert_eq!(amp.part.fields["Manufacturer"], "Local, Inc.");
    assert_eq!(amp.part.fields["MPN"], "TLV9001IDBVR");
    assert!(amp.part.datasheet.as_ref().unwrap().ends_with("tlv9001.pdf"));
    assert!(schematic::bom_csv(&lines).contains("\"Local, Inc.\""));
}

#[test]
fn pdf_attachments_are_addressed_by_content() {
    use schematic::datasheet::store_pdf;
    let dir = forge_dirs::temp_dir().join(format!("forge-pdf-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let owner = dir.join("library.fg");
    let bytes = b"%PDF-1.4\n%%EOF";
    let path = store_pdf(&owner, bytes).unwrap();
    assert_eq!(store_pdf(&owner, bytes).unwrap(), path);
    assert!(store_pdf(&owner, b"other").is_err());
    assert_eq!(std::fs::read(dir.join(path)).unwrap(), bytes);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn printed_fields_and_geometry_warnings_use_the_same_layout() {
    let src = r#"schematic
top = sheet()
S = symbol(pins: {}, graphics: [rect(from:(-4,-3),to:(4,3))])
U1 = place(sheet:top,symbol:S,at:(0,0),value:"Device",ref_offset:(-2,-1),value_offset:(10,6))
a = junction(sheet:top,at:(8,0))
b = junction(sheet:top,at:(20,12))
w = wire(sheet:top,from:a,to:b)
hits = note(sheet:top,at:(12,4),value:"X")
clear = note(sheet:top,at:(9,10),value:"X")
j = junction(sheet:top,at:(30,30))
mark = note(sheet:top,at:(30,29.9),value:"X")
"#;
    let (d, c) = run(src).unwrap();
    let warn = |a: &str, b: &str| {
        c.issues
            .iter()
            .any(|i| i.kind == "TextGeometryOverlap" && i.severity() == "warning" && i.objects == [a, b])
    };
    assert!(warn("top/U1:reference", "top/U1"));
    assert!(warn("top/hits", "top/w"));
    assert!(!warn("top/clear", "top/w"));
    assert!(warn("top/mark", "top/j"));
    let page = schematic::sheet_page(&d, &c, "top").unwrap();
    let field = |s: &str| page.texts.iter().find(|t| t.text == s).unwrap();
    assert!(field("U1").h > field("Device").h);
    assert_eq!(field("Device").at.x, 10.);
}

#[test]
fn power_symbols_name_global_nets_and_attachment_markers_follow_the_drawing() {
    let src = r#"schematic
p=library(file:"forge:power")
top=sheet()
child=sheet()
sub=subsheet(sheet:top,of:child,at:(40,0))
one=place(sheet:top,symbol:p.GND,at:(0,0))
two=place(sheet:child,symbol:p.GND,at:(0,0))
av=place(sheet:top,symbol:p.AGND,at:(10,0))
j=junction(sheet:top,at:(0,3))
w=wire(sheet:top,from:j,to:one.net)
"#;
    let (d, c) = run(src).unwrap();
    assert_eq!(c.net_of["top/one.net"], c.net_of["top/sub/two.net"]);
    assert_ne!(c.net_of["top/one.net"], c.net_of["top/av.net"]);
    assert!(c.is_attached("top/one.net"));
    assert!(!c.is_attached("top/sub/two.net"));
    assert!(schematic::bom(&d, &c).is_empty());
    assert!(
        c.issues
            .iter()
            .any(|i| i.kind == "UnconnectedPin" && i.objects == ["top/av.net"])
    );
    let page = schematic::sheet_page(&d, &c, "top").unwrap();
    assert!(page.texts.iter().any(|t| t.text == "GND"));
    assert!(!page.texts.iter().any(|t| t.text == "one"));
}

#[test]
fn sensor_interface_has_clear_print_and_four_physical_components() {
    let source = include_str!("../../../examples/electronics/analog/sensor-interface.fg");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let (doc, c) = schematic::compile(source, Some(&dir), None).unwrap();
    // The example consumes an external 3V3 rail. Naming the rail must not fabricate a driver.
    assert_eq!(
        c.issues.iter().map(|i| i.kind).collect::<Vec<_>>(),
        ["UndrivenPowerInput", "UnknownSupplyVoltage"]
    );
    assert_eq!(
        schematic::bom(&doc, &c)
            .iter()
            .map(|l| l.references.len())
            .sum::<usize>(),
        4
    );
}

#[test]
fn local_global_and_hierarchical_labels_have_distinct_drawings_and_scopes() {
    let src = r#"schematic
top = sheet()
child = sheet()
instance = subsheet(sheet:top, of:child, at:(100,0))
a = junction(sheet:top, at:(0,0))
b = junction(sheet:child, at:(0,0))
g = junction(sheet:child, at:(0,8))
h = junction(sheet:top, at:(0,8))
l1 = label(sheet:top, on:a, at:(0,0), value:"sense")
l2 = label(sheet:child, on:b, at:(0,0), value:"sense")
g1 = label(sheet:top, on:h, at:(0,8), value:"enable", global:true)
g2 = label(sheet:child, on:g, at:(0,8), value:"enable", global:true)
signal = port(sheet:child, at:(0,16))
"#;
    let (doc, circuit) = run(src).unwrap();
    assert_ne!(circuit.net_of["top/a"], circuit.net_of["top/instance/b"]);
    assert_eq!(circuit.net_of["top/h"], circuit.net_of["top/instance/g"]);
    assert_eq!(
        circuit.net_of["top/instance.signal"],
        circuit.net_of["top/instance/signal"]
    );
    let page = schematic::sheet_page(&doc, &circuit, "top/instance").unwrap();
    // Junction dots are fills. Only the global and hierarchical tags introduce outline strokes.
    let outlines: Vec<_> = page.strokes.iter().map(|s| s.geom.points(0.03)).collect();
    assert_eq!(outlines.len(), 2);
    assert!(
        outlines
            .iter()
            .any(|p| p[0].x == 0. && p[0].y == 8. && p[1].x > 0.)
    );
    assert!(
        outlines
            .iter()
            .any(|p| p[0].x == p[1].x && p[0].y < 16. && p[1].y > 16.)
    );
    assert!(page.texts.iter().any(|t| t.text == "sense" && t.at.y > 0.));
}

#[test]
fn pin_inspection_preserves_evidence_in_both_directions() {
    use schematic::Evidence;
    let file = include_str!("../../../examples/electronics/analog/sensor-interface.fg");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    for (source, evidence) in [
        (file.to_owned(), Evidence::Verified),
        (
            file.replace("return_to: U1.VSS", "return_to: U1.OUT"),
            Evidence::Invalid,
        ),
        (
            file.lines()
                .filter(|s| !s.starts_with("local_cap ="))
                .collect::<Vec<_>>()
                .join("\n"),
            Evidence::Candidate,
        ),
    ] {
        let (doc, c) = schematic::compile(&source, Some(&dir), None).unwrap();
        assert!(
            doc.symbols["analog.Amplifier"].pins["VDD"]
                .description
                .contains("Bypass")
        );
        for (id, need, peer) in [
            ("main/U1.VDD", true, "main/C2.P"),
            ("main/C2.P", false, "main/U1.VDD"),
        ] {
            let inspection = schematic::inspect_pin(&c, id).unwrap();
            assert!(
                inspection
                    .checks
                    .iter()
                    .any(|r| r.need == need && r.peers.contains(&(peer.into(), evidence)))
            );
            assert!(inspection.wires.contains("main/w11"));
            assert!(inspection.wires.contains("main/w10"));
            assert_eq!(inspection.rails["+3V3"], "main/supply.net");
            assert!(!inspection.wires.contains("main/w12"));
            assert!(!inspection.wires.contains("main/w14"));
            assert!(!inspection.pins.contains("main/U1.VSS"));
            assert!(!inspection.pins.contains("main/R1.A"));
            let page = schematic::pin_inspection_page(&doc, &c, "main", &inspection).unwrap();
            assert!(page.texts.iter().any(|t| t.text == "+3V3"));
            assert!(page.texts.iter().any(|t| t.text == "VDD (5)"));
            assert!(!page.texts.iter().any(|t| t.text == "VSS (2)" || t.text == "GND"));
            let vss = c.locations["main/U1.VSS"];
            assert!(
                !page
                    .strokes
                    .iter()
                    .any(|s| s.geom.points(0.1).iter().any(|p| [p.x, p.y] == vss))
            );
        }
        let ret = schematic::inspect_pin(&c, "main/U1.VSS").unwrap();
        assert_eq!(ret.rails["GND"], "main/ground.net");
        assert!(!ret.pins.contains("main/C2.N")); // Another GND symbol across the global join.
        assert!(!ret.pins.contains("main/U1.VDD"));
        assert!(ret.wires.contains("main/w12"));
        assert!(!ret.wires.contains("main/w10"));
    }
}

#[test]
fn signal_directions_respect_bends_branches_opposed_flows_and_unknowns() {
    use schematic::Direction::*;
    let file = include_str!("../../../examples/electronics/analog/sensor-interface.fg");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    for (extra, expected) in [
        ("", Forward),
        ("\nopposed=flow(sheet:main,from:output,to:feedback)", Conflict),
    ] {
        let (_, c) = schematic::compile(&format!("{file}{extra}"), Some(&dir), None).unwrap();
        let d = schematic::wire_directions(&c);
        assert_eq!(d["main/w7"], expected);
        assert_eq!(d["main/w8"], Forward);
        assert!(!d.contains_key("main/w10")); // Passive power symbols do not establish drive direction.
    }
    let src = "schematic\nmain=sheet()\nD=symbol(pins:{x:pin(number:\"1\",kind:\"output\",at:(0,0))})\na=place(sheet:main,symbol:D,at:(0,0))\nb=place(sheet:main,symbol:D,at:(5,0))\nj=junction(sheet:main,at:(2,0))\nw=wire(sheet:main,from:j,to:a.x)\n";
    assert_eq!(
        schematic::wire_directions(&run(src).unwrap().1)["main/w"],
        Reverse
    );
    let joined = format!("{src}v=wire(sheet:main,from:j,to:b.x)");
    assert_eq!(
        schematic::wire_directions(&run(&joined).unwrap().1)["main/w"],
        Conflict
    );
}

#[test]
fn sheet_annotations_round_trip_styles_without_creating_connections() {
    use forge_draw::Pen;
    let file = include_str!("../../../examples/electronics/analog/filter-sheets.fg");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let (doc, c) = schematic::compile(file, Some(&dir), None).unwrap();
    let bare = file.split("outline =").next().unwrap();
    assert_eq!(
        c.net_of,
        schematic::compile(bare, Some(&dir), None).unwrap().1.net_of
    );
    assert!(
        !c.issues.iter().any(|i| i.kind == "TextGeometryOverlap"),
        "{:?}",
        c.issues
    );
    let page = schematic::sheet_page(&doc, &c, "main/left_filter").unwrap();
    assert!(page.strokes.iter().any(|s| s.pen == Pen::Annotation(450)));
    assert!(
        page.texts
            .iter()
            .any(|t| t.text == "Analog input filter" && t.h == 2.8 && t.pen == Pen::BoldText)
    );
    let svg = schematic::response(&doc, &c, &["svg".into(), "main/right_filter".into()]).unwrap();
    let svg = svg.get("result").unwrap().str().unwrap_or("");
    assert!(
        svg.contains("font-weight=\"bold\"") && svg.contains("stroke-width=\"0.45\""),
        "{svg}"
    );
    for bad in [
        file.replace("size: 2.8mm", "size: -2mm"),
        file.replace("width: 0.3mm", "width: 3deg"),
    ] {
        assert!(schematic::compile(&bad, Some(&dir), None).is_err());
    }
}

#[test]
fn pin_aliases_preserve_existing_sheet_references() {
    let lib = include_str!("../../../examples/electronics/analog/analog-symbols.fg");
    let sheet = include_str!("../../../examples/electronics/analog/sensor-interface.fg");
    let lib = lib.replacen(
        "VDD: pin(",
        "SUPPLY: pin(aliases: [VDD, VPLUS], angle: -22.5deg, ",
        1,
    );
    let sheet = sheet
        .replace(
            "analog = library(file: \"analog-symbols.fg\")",
            lib.trim_start_matches("symbol_library"),
        )
        .replace("analog.", "")
        + "\nold_name = no_connect(sheet: main, on: U1.VPLUS)\n";
    let (doc, c) = schematic::compile(&sheet, None, None).unwrap();
    assert_eq!(doc.symbols["Amplifier"].pins["SUPPLY"].angle, 337.5);
    assert_eq!(c.wires["main/w9"].0, "main/U1.SUPPLY");
    assert_eq!(c.links[0].want, "main/U1.SUPPLY");
    assert!(!c.terminals.contains_key("main/U1.VDD"));
    schematic::sheet_page(&doc, &c, "main").unwrap();
    let bad = sheet.replace("[VDD, VPLUS]", "[VDD, VSS]");
    assert_eq!(
        schematic::compile(&bad, None, None).unwrap_err().kind,
        "DuplicatePin"
    );
}

#[test]
fn reference_prefixes_are_checked_symbol_metadata() {
    let lib = "symbol_library\nOld = symbol(pins: {})\nGround = symbol(power_net: \"GND\", pins: {G: pin(number: \"1\", kind: \"passive\", at: (0,0))})\nTestPoint = symbol(ref_prefix: \"TP\", pins: {})\n";
    let (doc, _) = schematic::compile(lib, None, None).unwrap();
    assert_eq!(doc.symbols["Old"].ref_prefix, "U");
    assert_eq!(doc.symbols["Ground"].ref_prefix, "PWR");
    assert_eq!(doc.symbols["TestPoint"].ref_prefix, "TP");
    for invalid in ["", "TP1", "R?", " A"] {
        let src = lib.replace("\"TP\"", &format!("{invalid:?}"));
        assert_eq!(
            schematic::compile(&src, None, None).unwrap_err().kind,
            "InvalidReferencePrefix"
        );
    }
}

#[test]
fn supply_want_checks_the_actual_connected_voltage() {
    let sheet = include_str!("../../../examples/electronics/analog/powered-amplifier.fg");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    for (supply, status) in [
        ("voltage_mv: 1800", "satisfied"),
        ("voltage_mv: 3300", "satisfied"),
        ("voltage_mv: 5500", "satisfied"),
        ("voltage_mv: 12000", "out_of_range"),
        ("voltage_mv: 1799", "out_of_range"),
        ("voltage_range_v: [3.2, 3.4]", "satisfied"),
        ("voltage_range_v: [5.4, 5.6]", "out_of_range"),
    ] {
        let src = sheet.replace("voltage_mv: 3300", supply);
        let (d, c) = schematic::compile(&src, Some(&dir), None).unwrap();
        let id = &d.role_names["analog.supply_voltage"];
        let check = &c.voltages["main/U1.VDD"][id];
        assert_eq!(check.status(), status);
        assert_eq!(check.providers[0].pin, "main/J1.P1");
        let failed = status == "out_of_range";
        assert_eq!(
            c.issues
                .iter()
                .any(|i| i.kind == "VoltageOutOfRange" && i.objects == ["main/U1.VDD", "main/J1.P1"]),
            failed
        );
        let inspection = schematic::inspect_pin(&c, "main/U1.VDD").unwrap();
        let expected = if failed {
            schematic::Evidence::Invalid
        } else {
            schematic::Evidence::Verified
        };
        assert!(
            inspection
                .checks
                .iter()
                .any(|r| r.role == *id && r.peers == [("main/J1.P1".into(), expected)])
        );
        let report = schematic::response(&d, &c, &["pin".into(), "main/U1.VDD".into()]).unwrap();
        let Some(forge_agent::json::Json::Arr(checks)) =
            report.get("result").and_then(|r| r.get("voltage_checks"))
        else {
            panic!("missing voltage checks from pin query");
        };
        assert_eq!(checks[0].get("status").and_then(|s| s.str()), Some(status));
        let provider = schematic::inspect_pin(&c, "main/J1.P1").unwrap();
        assert!(
            provider
                .checks
                .iter()
                .any(|row| !row.need && row.peers.contains(&("main/U1.VDD".into(), expected)))
        );
    }
    for change in [
        sheet.replace(
            "symbol: analog.Connector2, gives:",
            "symbol: analog.Connector2, populated: false, gives:",
        ),
        sheet.replace("feed = wire(sheet: main, from: J1.P1, to: vdd)", ""),
    ] {
        let (d, c) = schematic::compile(&change, Some(&dir), None).unwrap();
        assert_eq!(
            c.voltages["main/U1.VDD"][&d.role_names["analog.supply_voltage"]].status(),
            "unknown"
        );
    }
    let mixed = format!(
        r#"{sheet}
high = role(kind: "power", voltage_mv: 12000)
HighSource = symbol(pins: {{out: pin(number: "1", kind: "power_out", at: (0,0), gives: [high])}})
J2 = place(sheet: main, symbol: HighSource, at: (40,30))
overvoltage = wire(sheet: main, from: J2.out, to: vdd)
"#
    );
    let (d, c) = schematic::compile(&mixed, Some(&dir), None).unwrap();
    let result = &c.voltages["main/U1.VDD"][&d.role_names["analog.supply_voltage"]];
    assert_eq!(result.status(), "out_of_range");
    assert_eq!(result.providers.len(), 2);
    let src = sheet.replace("voltage_mv: 3300", "voltage_range_v: [5.5,1.8]");
    assert_eq!(
        schematic::compile(&src, Some(&dir), None).unwrap_err().kind,
        "InvalidVoltageRange"
    );
}

#[test]
fn peripheral_membership_is_scoped_to_the_symbol_and_survives_pin_renames() {
    let src = r#"symbol_library
MCU = symbol(peripherals: {I2C1: {SDA: [PA0], SCL: [PA1]}, UART2: {TX: [PA0]}}, pins: {
    P0: pin(aliases:[PA0], number:"17", kind:"bidirectional", at:(0,0)),
    PA1: pin(number:"18", kind:"bidirectional", at:(0,1))
})
"#;
    let (d, _) = schematic::compile(src, None, None).unwrap();
    let groups = &d.symbols["MCU"].peripherals;
    assert!(groups["I2C1"]["SDA"].contains("P0"));
    assert!(groups["UART2"]["TX"].contains("P0"));
    assert!(groups["I2C1"]["SCL"].contains("PA1"));
    let bad = src.replace("TX: [PA0]", "TX: [PA9]");
    assert_eq!(
        schematic::compile(&bad, None, None).unwrap_err().kind,
        "UnknownPeripheralPin"
    );
}

#[test]
fn connector_roles_belong_to_the_placement_and_inherit_opt_in_symbol_roles() {
    let sheet = r#"schematic
main = sheet()
source = role(kind:"power", voltage_mv:3300)
sink = role(kind:"power", voltage_range_v:[1.8,5.5])
keyed = role(kind:"interface")
Connector = symbol(ref_prefix:"J", pins:{contact:pin(number:"1",aliases:[p],kind:"passive",at:(0,0)), shield:pin(number:"2",kind:"passive",at:(0,1),wants:[keyed])})
J1 = place(sheet:main,symbol:Connector,at:(0,0),gives:{p:[source]})
J2 = place(sheet:main,symbol:Connector,at:(12,0),wants:{contact:[sink]})
J3 = place(sheet:main,symbol:Connector,at:(30,0))
link = wire(sheet:main,from:J1.contact,to:J2.contact)
"#;
    let (d, c) = schematic::compile(sheet, None, None).unwrap();
    assert!(d.symbols["Connector"].pins["contact"].gives.is_empty());
    assert!(c.terminals["main/J1.contact"].wants.is_empty());
    assert!(c.terminals["main/J2.contact"].gives.is_empty());
    let spare = &c.terminals["main/J3.contact"];
    assert!(spare.gives.is_empty() && spare.wants.is_empty());
    assert_eq!(
        c.voltages["main/J2.contact"][&d.role_names["sink"]].status(),
        "satisfied"
    );
    assert!(
        c.terminals["main/J3.shield"]
            .wants
            .contains(&d.role_names["keyed"])
    );
    for (replacement, kind) in [
        ("bogus:[source]", "UnknownPin"),
        ("p:[missing]", "UnknownRole"),
        ("p:[source],contact:[source]", "DuplicatePin"),
    ] {
        assert_eq!(
            schematic::compile(&sheet.replace("p:[source]", replacement), None, None)
                .unwrap_err()
                .kind,
            kind
        );
    }
}

#[test]
fn pin_connection_wants_follow_wiring_type_and_fitted_value() {
    const FILE: &str = include_str!("../../../examples/electronics/analog/pin-connections.fg");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let compile = |s: &str| schematic::compile(s, Some(&dir), Some(&dir));
    let (doc, c) = compile(FILE).unwrap();
    let pin = "main/U1.INP";
    let wanted = &doc.role_names["input_filter"];
    assert!(c.connections["main/U1.INM"][&doc.role_names["feedback"]].satisfied);
    assert_eq!(c.connections[pin][wanted].bridge_pins, ["main/R1.B", "main/R1.A"]);
    let inspection = schematic::inspect_pin(&c, pin).unwrap();
    assert!(inspection.wires.contains("main/input_wire") && inspection.wires.contains("main/filtered_wire"));
    assert!(!inspection.pins.contains("main/U1.OUT"));
    assert!(
        schematic::response(&doc, &c, &["pin".into(), pin.into()])
            .unwrap()
            .to_string()
            .contains("connection_checks")
    );
    for (from, to, pass) in [
        ("value: \"4.7 kΩ\"", "value: \"2200 ohm\"", true),
        ("value: \"4.7 kΩ\"", "value: \"4k7\"", true),
        ("value: \"4.7 kΩ\"", "value: \"4701 Ω\"", false),
        ("value: \"4.7 kΩ\"", "value: \"4.7 nF\"", false),
        ("value: \"4.7 kΩ\"", "value: \"TBD\"", false),
        ("value: \"4.7 kΩ\"", "populated: false", false),
        ("from: J1.P1, to: R1.A", "from: J1.P2, to: R1.A", false),
        ("connect_to: J1.P1", "connect_to: J1.absent", false),
        ("through: analog.Resistor", "through: \"resistor\"", true),
        ("value_range: [\"2.2 kΩ\", \"4.7 kΩ\"]", "value: \"4700 Ω\"", true),
        (
            "value_range: [\"2.2 kΩ\", \"4.7 kΩ\"]",
            "value: \"3300 Ω\"",
            false,
        ),
        (", value_range: [\"2.2 kΩ\", \"4.7 kΩ\"]", "", true),
        ("from: J1.P1, to: R1.A", "from: J1.P1, to: U1.INP", false),
    ] {
        let (_, c) = compile(&FILE.replace(from, to)).unwrap();
        assert_eq!(c.connections[pin][wanted].satisfied, pass, "{to}");
        assert_eq!(
            c.issues
                .iter()
                .any(|i| i.kind == "PinConnectionMismatch" && i.objects[0] == pin),
            !pass
        );
    }
    for (from, to, kind) in [
        (
            "through: analog.Resistor",
            "through: analog.Amplifier",
            "ExpectedPartPath",
        ),
        ("through: analog.Resistor", "through: unknown", "UnknownSymbol"),
        (
            "through: analog.Resistor",
            "through: analog.Capacitor",
            "WrongPartValueDimension",
        ),
        ("2.2 kΩ", "22 F", "InvalidPartRange"),
        ("2.2 kΩ", "22 kΩ", "InvalidPartRange"),
    ] {
        assert_eq!(compile(&FILE.replace(from, to)).unwrap_err().kind, kind);
    }
    // A part class accepts a different library symbol; an exact-symbol rule still distinguishes it.
    let variant = "Other = symbol(part_kind: \"resistor\", pins: {A: pin(number: \"1\", kind: \"passive\", at: (0,0)), B: pin(number: \"2\", kind: \"passive\", at: (3,0))})";
    let substituted = FILE
        .replace("main = sheet", &format!("{variant}\nmain = sheet"))
        .replace("symbol: analog.Resistor,", "symbol: Other,");
    assert!(!compile(&substituted).unwrap().1.connections[pin][wanted].satisfied);
    let generic = substituted.replace("through: analog.Resistor", "through: \"resistor\"");
    assert!(compile(&generic).unwrap().1.connections[pin][wanted].satisfied);
    let disconnected = FILE.replace("from: U1.OUT, to: U1.INM", "from: U1.OUT, to: J1.P2");
    assert!(
        !compile(&disconnected).unwrap().1.connections["main/U1.INM"][&doc.role_names["feedback"]].satisfied
    );
    let self_link = FILE.replace("connect_to: OUT", "connect_to: INM");
    assert!(
        !compile(&self_link).unwrap().1.connections["main/U1.INM"][&doc.role_names["feedback"]].satisfied
    );
    let swapped = FILE
        .replace("R1.A", "R1.swap")
        .replace("R1.B", "R1.A")
        .replace("R1.swap", "R1.B");
    assert!(compile(&swapped).unwrap().1.connections[pin][wanted].satisfied);
    let repeated = format!(
        "{FILE}\ntop = sheet()\nfirst = subsheet(sheet: top, of: main, at: (0,0))\nsecond = subsheet(sheet: top, of: main, at: (10,0))\n"
    );
    let (_, c) = compile(&repeated).unwrap();
    for scope in ["top/first", "top/second"] {
        let check = &c.connections[&format!("{scope}/U1.INP")][wanted];
        assert!(check.satisfied);
        assert_eq!(check.target, format!("{scope}/J1.P1"));
    }
}

#[test]
fn paths_use_the_declared_component_type_and_terminal_pair() {
    const FILE: &str = include_str!("../../../examples/electronics/analog/switched-load.fg");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let compile = |s: &str| schematic::compile(s, Some(&dir), Some(&dir));
    let (doc, c) = compile(FILE).unwrap();
    let report = schematic::response(&doc, &c, &["pin".into(), "main/J1.P1".into()])
        .unwrap()
        .to_string();
    assert!(
        report.contains(r#""min":"0.00000001","max":"0.00000001","unit":"F""#),
        "{report}"
    );
    let role = &doc.role_names["load_return"];
    let status = |c: &Circuit| c.connections["main/J1.P1"][role].satisfied;
    assert!(status(&c));
    assert_eq!(
        c.connections["main/J1.P1"][role].bridge_pins,
        ["main/Q1.D", "main/Q1.S"]
    );
    assert!(c.connections["main/J1.P1"][&doc.role_names["output_cap"]].satisfied);
    assert!(c.connections["main/J2.P1"][&doc.role_names["load_coil"]].satisfied);
    for (old, new, pass) in [
        ("to: Q1.D", "to: Q1.G", false),
        ("channel: [D, S]", "channel: [S, D]", false),
        ("channel: [D, S]", "channel: [D, S], control: [G,S]", true),
        (
            "through: \"mosfet\", part_path: \"channel\"",
            "through: Switch, part_path: \"channel\"",
            true,
        ),
        ("part_kind: \"mosfet\"", "part_kind: \"relay\"", false),
        ("mosfet", "custom_optocoupler", true),
        ("D: pin(number:", "Drain: pin(aliases: [D], number:", true),
        (
            "symbol: Switch, at:",
            "symbol: Switch, populated: false, at:",
            false,
        ),
        (", part_path: \"channel\"", "", true),
    ] {
        assert_eq!(
            status(&compile(&FILE.replace(old, new)).unwrap().1),
            pass,
            "{new}"
        );
    }
    let ambiguous = FILE
        .replace(", part_path: \"channel\"", "")
        .replace("channel: [D, S]", "channel: [D, S], control: [G,S]");
    assert!(!status(&compile(&ambiguous).unwrap().1));
    assert_eq!(
        compile(&FILE.replace("channel: [D, S]", "channel: [G, missing]"))
            .unwrap_err()
            .kind,
        "UnknownPathPin"
    );
    let cap_wrong = compile(&FILE.replace("value: \"10 nF\")\ndrain", "value: \"100 nF\")\ndrain"))
        .unwrap()
        .1;
    assert!(!cap_wrong.connections["main/J1.P1"][&doc.role_names["output_cap"]].satisfied);
}

#[test]
fn source_edits_resolve_pin_targets_after_placements_and_quantities_in_order() {
    use forge_lang::{edit, error::Op};
    let src = "schematic\nmain = sheet()\nlate = junction(sheet: main, at: (1,1))\n";
    let add = |text: &str| Op::AddFeature {
        before: Some("late".into()),
        text: text.into(),
    };
    let s = edit::apply(src, &add("intent = role(kind: \"bias\", connect_to: late.P1)")).unwrap();
    let updated = edit::apply(
        &s,
        &Op::SetArg {
            id: "intent".into(),
            arg: "connect_to".into(),
            text: "late.P2".into(),
        },
    )
    .unwrap();
    assert!(updated.contains("connect_to: late.P2"));
    assert!(edit::apply(src, &add("rail = role(kind: \"power\", voltage_mv: late)")).is_err());
}

const SWITCH_CARD: &str = include_str!("../../../examples/electronics/load-switch/switch-card.fg");
fn switch_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/load-switch")
}

#[test]
fn child_library_edits_reach_board_handoff_templates_and_variants() {
    let (d, c) = schematic::compile(SWITCH_CARD, Some(&switch_dir()), None).unwrap();
    assert!(d.libraries.contains_key("components.discrete.switches"));
    assert_eq!(d.library_paths().count(), 3);
    assert!(c.connections["overview/left/J1.IN"].values().all(|v| v.satisfied));
    let board = schematic::board(&d, &c, Some("prototype")).unwrap();
    let forge_agent::json::Json::Arr(parts) = board.get("components").unwrap() else {
        panic!()
    };
    let right = parts
        .iter()
        .find(|p| p.get("id").unwrap().str() == Some("overview/right/Q1"))
        .unwrap();
    assert_eq!(right.get("populated"), Some(&false.into()));
    assert!(right.get("pads").unwrap().to_string().contains("\"EP\""));
    assert!(
        schematic::search(&d, &c, "load switch")
            .iter()
            .any(|h| h.id == "overview/right/Q1" && h.zone.is_some())
    );
    let page = schematic::sheet_page(&d, &c, "overview/right").unwrap();
    assert!(page.texts.iter().any(|t| t.text.contains("Sheet 3 of 3")));
    let data = schematic::sheet_pdf(&d, &c).unwrap();
    let pdf = String::from_utf8_lossy(&data);
    assert!(pdf.starts_with("%PDF-") && pdf.contains("/Count 3") && pdf.contains("Load switch channel"));
}

#[test]
fn nested_cycle_and_leaf_reload_have_a_located_result() {
    let dir = forge_dirs::temp_dir().join(format!("forge-nested-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(dir.join("children")).unwrap();
    let parent = dir.join("parent.fg");
    let child = dir.join("children/parts.fg");
    let src = "schematic\nlib=library(file:\"parent.fg\")\ns=sheet()\nu=place(sheet:s,symbol:lib.child.Part,at:(0,0))";
    std::fs::write(
        &parent,
        "symbol_library\nchild=library(file:\"children/parts.fg\")",
    )
    .unwrap();
    let leaf = "symbol_library\nsig=role(kind:\"signal\")\nPart=symbol(pins:{p:pin(number:\"1\",kind:\"input\",at:(0,0),wants:[sig])})";
    std::fs::write(&child, leaf).unwrap();
    let mut cache = schematic::Cache::default();
    let mut query = || {
        cache.query(
            &dir.join("design.fg"),
            Some(&dir),
            src,
            &["pin".into(), "s/u.p".into()],
        )
    };
    assert!(query().unwrap().to_string().contains("lib.child.sig"));
    std::fs::write(&child, leaf.replace("input", "output")).unwrap();
    assert!(query().unwrap().to_string().contains("output"));
    std::fs::write(&child, "symbol_library\nlooplib=library(file:\"../parent.fg\")").unwrap();
    let e = query().unwrap_err();
    assert_eq!(e.kind, "LibraryCycle");
    assert!(e.file.unwrap().ends_with("children/parts.fg"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn alternate_functions_drive_erc_and_waivers_match_only_one_occurrence() {
    let src = r#"schematic
clock = role(kind: "clock")
IC = symbol(pins:{A:pin(kind:"input",number:"1",at:(-1,0),functions:{CLK:function(kind:"output",gives:[clock])}),B:pin(kind:"input",number:"2",at:(1,0),functions:{CLK:function(kind:"output")})},peripherals:{tim:{CLK:[A,B]}})
top=sheet()
cell=sheet()
one=subsheet(sheet:top,of:cell,at:(0,0))
two=subsheet(sheet:top,of:cell,at:(5,5))
chip=place(sheet:cell,symbol:IC,at:(0,0),functions:{A:CLK,B:CLK})
link=wire(sheet:cell,from:chip.A,to:chip.B)
bus1=bus(sheet:cell,members:{left:chip.A,right:chip.B})
relax=erc_rule(rule:"BusMemberShort",severity:"info")
allow=waiver(rule:"DriverConflict",objects:["top/one/chip.A","top/one/chip.B"],reason:"prototype fixture")
"#;
    let (d, c) = run(src).unwrap();
    assert_eq!(
        c.terminals["top/two/chip.A"].electrical,
        schematic::Electrical::Output
    );
    assert!(
        c.terminals["top/two/chip.A"]
            .gives
            .contains(&schematic::RoleId("clock".into()))
    );
    let clashes: Vec<_> = c.issues.iter().filter(|i| i.kind == "DriverConflict").collect();
    assert_eq!(clashes.len(), 2);
    assert_eq!(clashes.iter().filter(|i| i.waiver(&d).is_some()).count(), 1);
    assert_eq!(
        c.issues.iter().filter(|i| i.kind == "PeripheralConflict").count(),
        2
    );
    assert!(
        c.issues
            .iter()
            .filter(|i| i.kind == "BusMemberShort")
            .all(|i| i.priority(&d) == "info")
    );
    assert_eq!(
        run(&src.replace("A:CLK,B:CLK", "A:oops")).unwrap_err().kind,
        "UnknownPinFunction"
    );
}

#[test]
fn block_copy_retains_internal_typed_paths_and_source_identity() {
    let dir = switch_dir();
    let (d, c) = schematic::compile(SWITCH_CARD, Some(&dir), None).unwrap();
    let selection = ["J1", "Q1"].map(str::to_string).into_iter().collect();
    let plan =
        schematic::block_plan(SWITCH_CARD, &d, &c, "overview/left", &selection, [0., 24.], true).unwrap();
    let mut next = SWITCH_CARD.to_owned();
    for op in plan.ops {
        next = forge_lang::edit::apply(&next, &op).unwrap();
    }
    let (d2, c2) = schematic::compile(&next, Some(&dir), None).unwrap();
    let want = c2.connections["overview/left/J2.IN"].values().next().unwrap();
    assert!(want.satisfied);
    assert!(want.bridge_pins[0].contains("/Q2."));
    assert_ne!(c2.net_of["overview/left/J1.IN"], c2.net_of["overview/left/J2.IN"]);
    next = forge_lang::edit::apply(
        &next,
        &forge_lang::error::Op::SetArg {
            id: "J2".into(),
            arg: "reference".into(),
            text: "\"J17\"".into(),
        },
    )
    .unwrap();
    let (d3, c3) = schematic::compile(&next, Some(&dir), None).unwrap();
    assert_eq!(c2.net_of, c3.net_of);
    let diff = schematic::electrical_diff((&d2, &c2), (&d3, &c3))
        .unwrap()
        .to_string();
    assert!(diff.contains("J17") && diff.contains("overview/right/J2"));
}

#[test]
fn project_reannotation_reserves_locked_occurrences_and_preserves_net_identity() {
    let original = format!(
        "{SWITCH_CARD}\nlocked=annotation(occurrence:\"overview/right/Q1\",reference:\"Q1\",locked:true)"
    );
    let (d, c) = schematic::compile(&original, Some(&switch_dir()), None).unwrap();
    let mut next = original.clone();
    for op in schematic::project_annotation(&original, &d, &c).unwrap() {
        next = forge_lang::edit::apply(&next, &op).unwrap();
    }
    let (d2, c2) = schematic::compile(&next, Some(&switch_dir()), None).unwrap();
    assert_eq!(c.net_of, c2.net_of);
    assert_eq!(d2.reference("overview/left/Q1", "Q1"), "Q2");
    let reply = schematic::response(&d2, &c2, &["sheet".into(), "overview/left".into()]).unwrap();
    let items = reply.get("result").unwrap().get("items").unwrap();
    let forge_agent::json::Json::Arr(items) = items else {
        panic!()
    };
    assert_eq!(
        items
            .iter()
            .find(|i| i.get("id").and_then(|v| v.str()) == Some("Q1"))
            .unwrap()
            .get("reference")
            .unwrap()
            .str(),
        Some("Q2")
    );
    assert_eq!(d2.reference("overview/right/Q1", "Q1"), "Q1");
    assert!(schematic::project_annotation(&next, &d2, &c2).unwrap().is_empty());
    let result = schematic::response(&d2, &c2, &["erc".into(), "prototype".into()])
        .unwrap()
        .to_string();
    assert!(result.contains("overview/right/J1.IN") && result.contains("PinConnectionMismatch"));
}

#[test]
fn copying_an_internal_want_rebinds_the_specific_target_component() {
    const FILE: &str = include_str!("../../../examples/electronics/analog/pin-connections.fg");
    let dir = switch_dir().parent().unwrap().join("analog");
    let (d, c) = schematic::compile(FILE, Some(&dir), None).unwrap();
    let selected = ["U1", "J1", "R1"].map(str::to_owned).into_iter().collect();
    let plan = schematic::block_plan(FILE, &d, &c, "main", &selected, [20., 0.], true).unwrap();
    let mut src = FILE.to_owned();
    for op in plan.ops {
        src = forge_lang::edit::apply(&src, &op).unwrap();
    }
    let (_, clone) = schematic::compile(&src, Some(&dir), None).unwrap();
    let path = clone.connections["main/U2.INP"].values().next().unwrap();
    assert_eq!(path.target, "main/J2.P1");
    assert!(path.satisfied, "{}", path.detail);
}

#[cfg(unix)]
#[test]
fn linked_catalogs_resolve_pdf_assets_from_the_owning_file() {
    use std::os::unix::fs::symlink;
    let dir = std::env::temp_dir().join(format!("fg-library-assets-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("physical/devices")).unwrap();
    std::fs::create_dir_all(dir.join("aliases")).unwrap();
    std::fs::write(
        dir.join("physical/catalog.fg"),
        "symbol_library\nchild=library(file:\"devices/custom.fg\")",
    )
    .unwrap();
    std::fs::write(
        dir.join("physical/devices/custom.fg"),
        "symbol_library\nFET=symbol(pins:{},datasheet:\"datasheets/fet.pdf\")",
    )
    .unwrap();
    symlink("../physical/catalog.fg", dir.join("aliases/chips.fg")).unwrap();
    let (doc, _) = schematic::compile(
        "schematic\nlib=library(file:\"aliases/chips.fg\")",
        Some(&dir),
        None,
    )
    .unwrap();
    assert_eq!(
        doc.symbols["lib.child.FET"].datasheet.as_deref(),
        Some("physical/devices/datasheets/fet.pdf")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

const BIAS: &str = include_str!("../../../examples/electronics/i2c-bias.fg");

#[test]
fn pull_check_uses_active_internal_or_fitted_resistor_and_actual_rail() {
    fn evidence(src: &str, omit: bool) -> (bool, bool, String) {
        let (d, mut c) = run(src).unwrap();
        if omit {
            c = Circuit::build_fitted(&d, &d.variants["no_sda_pull"]).unwrap();
        }
        let check = &c.connections["main/U1.SDA"][&d.role_names["logic_high"]];
        (check.satisfied, check.unknown, check.detail.clone())
    }
    let (ok, _, why) = evidence(BIAS, false);
    assert!(
        ok && why.contains("R4") && why.contains("4.7 kΩ") && why.contains("3.3 V"),
        "{why}"
    );
    let omitted = evidence(BIAS, true);
    assert!(!omitted.0 && omitted.2.contains("omitted"));
    let alternate = BIAS.replace(
        "symbol:Controller,at:",
        "symbol:Controller,functions:{SDA:BIAS},at:",
    );
    assert!(evidence(&alternate, true).0);
    assert!(
        !evidence(
            &BIAS.replace(
                "v33 = role(kind: \"power\", voltage_mv: 3300)",
                "v33 = role(kind: \"power\", voltage_mv: 5000)"
            ),
            false
        )
        .0
    );
    assert!(!evidence(&BIAS.replace("4.7 kΩ", "47 kΩ"), false).0);
    assert!(evidence(&BIAS.replace("4.7 kΩ", "TBD"), false).1);
    assert!(
        evidence(
            &BIAS
                .replace("pull_up", "pull_down")
                .replace("[3.0,3.6]", "[0,0]")
                .replace("3300", "0"),
            false
        )
        .0
    );
    let (d, c) = run(BIAS).unwrap();
    let focus = schematic::inspect_pin(&c, "main/U1.SDA").unwrap();
    for pin in ["main/R4.A", "main/R4.B", "main/J1.OUT"] {
        assert!(focus.pins.contains(pin));
    }
    assert!(!focus.pins.contains("main/R5.A"));
    assert!(
        schematic::response(
            &d,
            &c,
            &["pin".into(), "main/U1.SDA".into(), "no_sda_pull".into()]
        )
        .unwrap()
        .to_string()
        .contains("omitted")
    );
}

#[test]
fn differential_pairs_keep_polarity_through_copy_and_label_changes() {
    let src = "schematic\nmain = sheet()\nother = sheet()\nplus = port(sheet:main,at:(0,0))\nminus = port(sheet:main,at:(0,5))\nla = label(sheet:main,on:plus,value:\"USB_D+\",at:(0,0))\nlb = label(sheet:main,on:minus,value:\"USB_D-\",at:(0,5))\nusb = diff_pair(sheet:main,positive:plus,negative:minus)\n";
    let (d, c) = run(src).unwrap();
    assert_eq!(
        schematic::pair_suggestions(&d, "main"),
        vec![("USB_D+".into(), "USB_D-".into(), "plus".into(), "minus".into())]
    );
    for (a, b) in [("USB_DP", "USB_DM"), ("RX_P", "RX_N"), ("TXP", "TXN")] {
        let (d, _) = run(&src.replace("USB_D+", a).replace("USB_D-", b)).unwrap();
        assert_eq!(schematic::pair_suggestions(&d, "main").len(), 1);
    }
    let pair = &c.diff_pairs["main/usb"];
    assert_eq!(pair, &["main/plus", "main/minus"]);
    assert!(
        schematic::board(&d, &c, None)
            .unwrap()
            .to_string()
            .contains("positive_net")
    );
    let (_, renamed) = run(&src.replace("USB_D+", "A")).unwrap();
    assert_eq!(&renamed.diff_pairs["main/usb"], pair);
    let plan = schematic::block_plan_to(
        src,
        &d,
        &c,
        "main",
        "other",
        &["plus".into(), "minus".into()].into(),
        [4., 5.],
        true,
    )
    .unwrap();
    let mut next = src.to_owned();
    for op in plan.ops {
        next = forge_lang::edit::apply(&next, &op).unwrap();
    }
    let (_, copied) = run(&next).unwrap();
    assert_eq!(copied.diff_pairs["other/usb1"], ["other/plus1", "other/minus1"]);
    let (_, short) = run(&src.replace("USB_D-", "USB_D+")).unwrap();
    assert!(short.issues.iter().any(|i| i.kind == "DiffPairShort"));
    let (_, conflict) = run(&format!(
        "{src}reversed = diff_pair(sheet:main,positive:minus,negative:plus)\n"
    ))
    .unwrap();
    assert!(conflict.issues.iter().any(|i| i.kind == "DiffPairConflict"));
}

#[test]
fn visually_ambiguous_wires_remain_electrically_separate() {
    let src = "schematic\nS = symbol(pins:{P:pin(kind:\"input\",number:\"1\",at:(0,0))})\nmain = sheet()\nU1 = place(sheet:main,symbol:S,at:(10.25,0))\na = junction(sheet:main,at:(0,0))\nb = junction(sheet:main,at:(10,0))\nc = junction(sheet:main,at:(4,0))\nf = junction(sheet:main,at:(6,0))\nw1 = wire(sheet:main,from:a,to:b)\nw2 = wire(sheet:main,from:c,to:f)\n";
    let (_, c) = run(src).unwrap();
    assert!(c.issues.iter().any(|i| i.kind == "WireNearPin"));
    assert!(c.issues.iter().any(|i| i.kind == "WireOverlap"));
    assert_ne!(c.net_of["main/a"], c.net_of["main/c"]);
    // The fixed overlapping wires must survive removal of a near-pin warning.
    // Moving a wire endpoint must create and clear collinear overlap independently.
    let mut cache = schematic::Cache::default();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = dir.join("wire-moves.fg");
    for source in [src.to_owned(), src.replace("from:a,to:b", "from:a,to:U1.P")] {
        for y in [0., 1., 0., 0.25, 20., 0.] {
            let text = source.replace("10.25,0", &format!("10.25,{y}"));
            let (doc, actual) = run(&text).unwrap();
            let query = ["erc".into()];
            assert_eq!(
                cache.query(&file, Some(dir), &text, &query).unwrap(),
                schematic::response(&doc, &actual, &query).unwrap()
            );
        }
    }
    let (_, clear) = run(&src
        .replace("10.25,0", "12,0")
        .replace("4,0", "5,-4")
        .replace("6,0", "5,4"))
    .unwrap();
    assert!(
        !clear
            .issues
            .iter()
            .any(|i| matches!(i.kind, "WireNearPin" | "WireOverlap"))
    );
}

#[test]
fn active_signal_contracts_drive_erc_inspection_and_board_handoff_per_branch() {
    use forge_agent::json::Json;
    let fixture = include_str!("../../../examples/electronics/adc-fanout.fg");
    for (mode, status, rule) in [
        ("ADC", "compatible", None),
        ("GPIO", "mismatch", Some("PinFunctionMismatch")),
        ("RAW", "unknown", Some("UnknownPinFunctionCompatibility")),
    ] {
        // Change the library pin's display identity and reverse a wire gesture as well.
        let src = fixture
            .replace("P0:ADC", &format!("P0:{mode}"))
            .replace("P0", "PORT")
            .replace("from:tap,to:U2.PORT", "from:U2.PORT,to:tap");
        let (d, c) = run(&src).unwrap();
        let board = schematic::board(&d, &c, None).unwrap();
        let Some(Json::Arr(paths)) = board.get("signal_paths") else {
            panic!("missing signal_paths")
        };
        assert_eq!(paths.len(), 2);
        for (i, (receiver, expected)) in [("main/U2.PORT", status), ("main/U3.IN", "compatible")]
            .iter()
            .enumerate()
        {
            let p = &paths[i];
            let sink = p.get("receiver").unwrap();
            assert_eq!(sink.get("terminal").unwrap().str(), Some(*receiver));
            assert_eq!(
                p.get("source").unwrap().get("terminal").unwrap().str(),
                Some("main/U1.OUT")
            );
            assert_eq!(p.get("status").unwrap().str(), Some(*expected));
            if i == 0 {
                assert_eq!(sink.get("active_function").unwrap().str(), Some(mode));
                let alternatives = if rule.is_some() {
                    vec!["ADC".into()]
                } else {
                    vec![]
                };
                assert_eq!(
                    p.get("compatible_receiver_functions"),
                    Some(&Json::Arr(alternatives))
                );
            } else {
                assert_eq!(sink.get("active_function"), Some(&Json::Null));
            }
        }
        let findings: Vec<_> = c
            .issues
            .iter()
            .filter(|f| matches!(f.kind, "PinFunctionMismatch" | "UnknownPinFunctionCompatibility"))
            .collect();
        assert_eq!(findings.len(), usize::from(rule.is_some()));
        if let Some(rule) = rule {
            let finding = findings[0];
            assert_eq!(finding.kind, rule);
            assert_eq!(finding.unknown, mode == "RAW");
            assert_eq!(finding.objects, ["main/U1.OUT", "main/U2.PORT"]);
            assert!(finding.message.contains("ADC"));
        }
        assert!(!c.issues.iter().any(|i| i.kind == "UnfulfilledWant"));
        let check = schematic::inspect_pin(&c, "main/U3.IN").unwrap();
        assert_eq!(
            check.checks[0].peers,
            [("main/U1.OUT".into(), schematic::Evidence::Verified)]
        );
        let fitted = schematic::board(&d, &c, Some("no_monitor")).unwrap();
        assert_eq!(
            fitted.get("signal_paths"),
            Some(&Json::Arr(vec![paths[0].clone()]))
        );
    }
    let strict = format!(
        "{}\nstrict=erc_rule(rule:\"PinFunctionMismatch\",severity:\"error\")\nfixed_only=variant(omit:[\"main/U2\"])",
        fixture.replace("P0:ADC", "P0:GPIO")
    );
    let (d, c) = run(&strict).unwrap();
    let base = schematic::response(&d, &c, &["board".into()]).unwrap();
    assert_eq!(base.get("erc_ok"), Some(&Json::Bool(false)));
    let fitted = schematic::response(&d, &c, &["board".into(), "fixed_only".into()]).unwrap();
    assert_eq!(fitted.get("erc_ok"), Some(&Json::Bool(true)));
}

#[test]
fn named_straps_follow_fitted_inputs_and_pinless_defaults_without_erc_findings() {
    let fixture = include_str!("../../../examples/electronics/strap-attributes.fg");
    let (d, c) = run(fixture).unwrap();
    let assert_value = |c: &Circuit, owner: &str, attr: &str, status: &str, value: Option<&str>| {
        let r = &c.straps[owner][attr];
        assert_eq!((r.status, r.value.as_deref()), (status, value));
    };
    assert_value(&c, "main/U1", "i2c_address", "matched", Some("0x49"));
    assert_value(&c, "main/U1", "current_limit", "matched", Some("500 mA"));
    assert_value(&c, "main/U2", "i2c_address", "default", Some("0x48"));
    assert_value(&c, "main/U2", "current_limit", "matched", Some("1 A"));
    assert_value(&c, "main/U3", "address", "default", Some("0x2C"));
    assert_value(&c, "main/U3", "powerup_mode", "default", Some("standby"));
    assert!(
        c.straps["main/U1"]["i2c_address"].inputs["main/U1.A0"]
            .evidence
            .contains("main/R1.B")
    );
    let (d2, rewired) = run(&fixture.replace("from:R1.B,to:U1.VDD", "from:R1.B,to:U1.VSS")).unwrap();
    assert_value(&rewired, "main/U1", "i2c_address", "default", Some("0x48"));
    let amb = format!("{fixture}\nshort=wire(sheet:main,from:U1.VDD,to:U1.VSS)");
    let (_, clash) = run(&amb).unwrap();
    assert_value(&clash, "main/U1", "i2c_address", "unresolved", None);
    assert!(!clash.issues.iter().any(|i| i.kind.contains("Strap")));
    let fitted = Circuit::build_fitted(&d, &d.variants["prototype"]).unwrap();
    assert!(!fitted.straps.contains_key("main/U3"));
    assert_value(&fitted, "main/U1", "i2c_address", "default", Some("0x48"));
    assert!(fitted.straps["main/U1"]["i2c_address"].inputs["main/U1.A0"].open);
    let handoff = schematic::board(&d2, &rewired, None).unwrap();
    let Some(forge_agent::json::Json::Arr(parts)) = handoff.get("components") else {
        panic!()
    };
    let values = parts
        .iter()
        .find(|p| p.get("id").unwrap().str() == Some("main/U1"))
        .unwrap()
        .get("derived_attributes")
        .unwrap();
    assert_eq!(
        values
            .get("current_limit")
            .unwrap()
            .get("value")
            .unwrap()
            .get("value")
            .unwrap()
            .num(),
        Some(0.5)
    );
    assert_eq!(
        values.get("i2c_address").unwrap().get("value").unwrap().num(),
        Some(72.)
    );
    assert_eq!(
        run(&fixture.replace("A0:high", "A0:missing")).unwrap_err().kind,
        "InvalidStrapRow"
    );
    assert_eq!(
        run(&fixture.replace("0x49", "0x80")).unwrap_err().kind,
        "InvalidStrapValue"
    );
}

#[test]
fn typed_tables_check_bus_identity_values_variants_and_sheet_fields() {
    let lib = include_str!("../../../examples/electronics/strap-address-matrix.fg");
    let source = format!(
        "{}\nmain=sheet()\na=place(sheet:main,symbol:Device,at:(0,0),value:\"sensor\")\nb=place(sheet:main,symbol:Device,at:(80,0),visibility:{{reference:false,derived:{{address:false}},custom:{{Manufacturer:true}}}})\nsda=wire(sheet:main,from:a.SDA,to:b.SDA)\nscl=wire(sheet:main,from:a.SCL,to:b.SCL)\nprototype=variant(omit:[\"main/b\"])",
        lib.replacen("symbol_library", "schematic", 1)
    );
    let (d, c) = run(&source).unwrap();
    let conflict: Vec<_> = c
        .issues
        .iter()
        .filter(|i| i.kind == "I2cAddressConflict")
        .collect();
    assert_eq!(conflict.len(), 1);
    assert_eq!(conflict[0].objects, ["main/a:address", "main/b:address"]);
    assert_eq!(conflict[0].severity(), "error");
    let fitted = Circuit::build_fitted(&d, &d.variants["prototype"]).unwrap();
    assert!(!fitted.issues.iter().any(|i| i.kind == "I2cAddressConflict"));
    let page = schematic::sheet_page(&d, &c, "main").unwrap();
    let lines: Vec<_> = page.texts.iter().map(|t| t.text.as_str()).collect();
    assert!(lines.contains(&"a") && !lines.contains(&"b") && lines.contains(&"sensor"));
    assert_eq!(lines.iter().filter(|s| **s == "address: 0x48").count(), 1);
    assert_eq!(lines.iter().filter(|s| **s == "Manufacturer: Example").count(), 1);
    assert_eq!(lines.iter().filter(|s| **s == "boot: Flash").count(), 2);
    for disconnected in [
        "sda=wire(sheet:main,from:a.SDA,to:b.SDA)",
        "scl=wire(sheet:main,from:a.SCL,to:b.SCL)",
    ] {
        let (_, c) = run(&source.replace(disconnected, "")).unwrap();
        assert!(!c.issues.iter().any(|i| i.kind == "I2cAddressConflict"));
    }
    let (_, c) = run(&source
        .replace("500 mA", "3000 mA")
        .replace("default:\"Flash\"", "default:\"External\"")
        .replace("default:\"0x48\"", "default:\"0b1111100\""))
    .unwrap();
    for rule in ["CurrentOutsideRange", "UnsupportedBootMode", "ReservedI2cAddress"] {
        assert_eq!(c.issues.iter().filter(|i| i.kind == rule).count(), 2, "{rule}");
    }
    let a = schematic::StrapKind::Address7;
    assert_eq!(a.converted("0b1001000", "Hex").as_deref(), Some("0x48"));
    assert_eq!(a.converted("0x48", "Decimal").as_deref(), Some("72"));
    assert_eq!(
        schematic::StrapKind::Current
            .converted("500000 µA", "mA")
            .as_deref(),
        Some("500 mA")
    );
}

#[test]
fn resolved_field_visibility_controls_overlap_evidence_and_inherits_on_reset() {
    let src = r#"schematic
Chip=symbol(pins:{},fields:{MPN:"ABC"},visibility:{custom:{MPN:true}},strap_tables:{address:strap_table(kind:"i2c_address",default:"72")})
main=sheet()
part=place(sheet:main,symbol:Chip,at:(0,0))
"#;
    let (d, c) = run(src).unwrap();
    let page = schematic::sheet_page(&d, &c, "main").unwrap();
    let field = page.texts.iter().find(|t| t.text == "address: 72").unwrap();
    let source = format!(
        "{src}\nmarker=note(sheet:main,value:\"aligned\",at:({},{}))",
        field.at.x, field.at.y
    );
    let (_, c) = run(&source).unwrap();
    let has_overlap = |c: &Circuit| {
        c.issues.iter().any(|i| {
            i.kind == "TextBoundsOverlap" && i.objects.iter().any(|s| s == "main/part:derived.address")
        })
    };
    assert!(has_overlap(&c));
    let (d, c) = run(&source.replace(
        "symbol:Chip,at:",
        "symbol:Chip,visibility:{derived:{address:false}},at:",
    ))
    .unwrap();
    assert!(!has_overlap(&c));
    assert!(
        schematic::sheet_page(&d, &c, "main")
            .unwrap()
            .texts
            .iter()
            .any(|t| t.text == "MPN: ABC")
    );
    assert_eq!(c.straps["main/part"]["address"].value.as_deref(), Some("72"));
}

#[test]
fn nested_attribute_types_keep_identity_and_erc_defaults_compose() {
    let dir = std::env::temp_dir().join(format!("fg-attribute-packs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, src: &str| std::fs::write(dir.join(name), src).unwrap();
    write(
        "firmware.fg",
        include_str!("../../../examples/electronics/attribute-types.fg"),
    );
    write(
        "parts.fg",
        r#"symbol_library
fw=library(file:"firmware.fg")
IC=symbol(pins:{},strap_tables:{startup:strap_table(kind:fw.startup_source,default:"Typo")})
policy=erc_rule(rule:"UnsupportedAttributeValue",severity:"warning")"#,
    );
    write(
        "review.fg",
        r#"symbol_library
policy=erc_rule(rule:"UnsupportedAttributeValue",severity:"info")"#,
    );
    let base = "schematic\ncomponents=library(file:\"parts.fg\")\nmain=sheet()\nu=place(sheet:main,symbol:components.IC,at:(0,0))";
    let compile = |src: &str| schematic::compile(src, Some(&dir), Some(&dir));
    let (doc, c) = compile(base).unwrap();
    let t = &doc.symbols["components.IC"].strap_tables["startup"];
    assert_eq!(t.source_type.as_deref(), Some("components.fw.startup_source"));
    assert_eq!(t.modes, doc.attribute_types["components.fw.startup_source"].modes);
    assert_eq!(
        c.issues
            .iter()
            .find(|i| i.kind == "UnsupportedAttributeValue")
            .unwrap()
            .priority(&doc),
        "warning"
    );
    let both = format!("{base}\nreview=library(file:\"review.fg\")");
    assert_eq!(compile(&both).unwrap_err().kind, "ConflictingRuleDefaults");
    let override_rule = "local=erc_rule(rule:\"UnsupportedAttributeValue\",severity:\"error\")";
    for src in [
        format!("{both}\n{override_rule}"),
        both.replacen("schematic", &format!("schematic\n{override_rule}"), 1),
    ] {
        let (doc, c) = compile(&src).unwrap();
        assert_eq!(
            c.issues
                .iter()
                .find(|i| i.kind == "UnsupportedAttributeValue")
                .unwrap()
                .priority(&doc),
            "error"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn voltage_strap_cells_use_provided_limits_and_evidence() {
    let src = r#"schematic
rail=role(kind:"power",voltage_range_v:[1.7,1.9])
Source=symbol(pins:{OUT:pin(number:"1",kind:"power_out",at:(0,0),gives:[rail])})
Device=symbol(pins:{MODE:pin(number:"1",kind:"input",at:(0,0))},
 strap_tables:{boot:strap_table(kind:"text",inputs:{MODE:{window:[1.7,1.9],off:"floating"}},
 rows:[[{MODE:window},"Service"],[{MODE:off},"Normal"]])})
main=sheet()
P=place(sheet:main,symbol:Source,at:(0,0))
U=place(sheet:main,symbol:Device,at:(20,0))
w=wire(sheet:main,from:P.OUT,to:U.MODE)"#;
    let check = |src: &str| run(src).unwrap().1;
    let c = check(src);
    let attr = &c.straps["main/U"]["boot"];
    assert_eq!(attr.value.as_deref(), Some("Service"));
    assert!(attr.inputs["main/U.MODE"].evidence.contains("main/P.OUT"));
    assert_eq!(check(&src.replace("voltage_range_v:[1.7,1.9]","voltage_range_v:[1.6,1.9]")).straps["main/U"]["boot"].status,"unresolved");
    assert_eq!(
        check(&src.replace("gives:[rail]", "wants:[rail]")).straps["main/U"]["boot"].status,
        "unresolved"
    );
    assert_eq!(
        check(src.split("\nw=wire").next().unwrap()).straps["main/U"]["boot"]
            .value
            .as_deref(),
        Some("Normal")
    );
    for (from, to, kind) in [
        ("window:[1.7,1.9]", "window:[1.9,1.7]", "InvalidStrapVoltage"),
        (
            "window:[1.7,1.9]",
            "window:[1.7,1.9],other:[1.9,2.1]",
            "InvalidStrapInput",
        ),
        ("window:[1.7,1.9]", "window:[1.7mm,1.9]", "WrongDim"),
    ] {
        assert_eq!(run(&src.replace(from, to)).unwrap_err().kind, kind);
    }
}

#[test]
fn library_units_bound_convert_and_name_attributes_without_changing_keys() {
    let base = r#"schematic
Rate=attribute_type(label:"Frequency",kind:"quantity",units:[{name:"Hz",factor:1,min:0,max:2000000},{name:"kHz",factor:1000,min:0,max:2000}])
IC=symbol(pins:{},strap_tables:{f:strap_table(label:"Switching frequency",kind:Rate,unit:"kHz",default:"125000 Hz")})
main=sheet()
u=place(sheet:main,symbol:IC,at:(0,0))"#;
    let (doc, c) = run(base).unwrap();
    assert_eq!(c.straps["main/u"]["f"].value.as_deref(), Some("125 kHz"));
    let page = schematic::sheet_page(&doc, &c, "main").unwrap();
    assert!(
        page.texts
            .iter()
            .any(|t| t.text == "Switching frequency: 125 kHz")
    );
    for (old, new, err) in [
        ("125000 Hz", "2100 kHz", "InvalidStrapValue"),
        ("125000 Hz", "125 V", "InvalidStrapValue"),
        ("unit:\"kHz\"", "unit:\"MHz\"", "UnknownAttributeUnit"),
        ("factor:1000", "factor:0", "InvalidAttributeUnit"),
        ("factor:1,min:0", "factor:2,min:0", "InvalidAttributeUnit"),
        ("name:\"kHz\"", "name:\"Hz\"", "InvalidAttributeUnit"),
        (
            "label:\"Switching frequency\"",
            "label:\"\"",
            "EmptyAttributeLabel",
        ),
    ] {
        assert_eq!(run(&base.replace(old, new)).unwrap_err().kind, err);
    }
    let missing = base.replace("kind:Rate", "kind:\"quantity\"");
    assert_eq!(run(&missing).unwrap_err().kind, "MissingAttributeUnits");
    let untyped = "symbol_library\nR=attribute_type(label:\"Rate\",kind:\"quantity\")";
    assert_eq!(run(untyped).unwrap_err().kind, "MissingAttributeUnits");
    let a = schematic::StrapKind::Address7;
    let units = a.standard_units();
    assert_eq!(
        a.format_unit("48", units, Some("Hex"), "Decimal").as_deref(),
        Some("72")
    );
    assert!(a.typed_with_units("80", units, Some("Hex")).is_none());
}

#[test]
fn every_ternary_address_is_selected_by_the_three_fitted_pin_states() {
    let def = include_str!("../../../examples/electronics/ternary-address.fg").replacen(
        "symbol_library",
        "schematic",
        1,
    );
    let base = format!("{def}\nmain=sheet()\nu=place(sheet:main,symbol:Decoder,at:(0,0))");
    for i in 0..27 {
        let mut src = base.clone();
        for (pin, trit) in [("A2", i / 9), ("A1", (i / 3) % 3), ("A0", i % 3)] {
            if trit < 2 {
                let rail = ["VSS", "VDD"][trit];
                src.push_str(&format!("\nw{pin}=wire(sheet:main,from:u.{pin},to:u.{rail})"));
            }
        }
        let (d, c) = run(&src).unwrap();
        let table = &d.symbols["Decoder"].strap_tables["address"];
        assert_eq!(table.column_order, ["A2", "A1", "A0"]);
        assert_eq!(table.rows.len(), 27);
        assert_eq!(
            c.straps["main/u"]["address"].value,
            Some(format!("0x{:02X}", 32 + i))
        );
    }
}

#[test]
fn integer_formats_round_trip_by_declared_radix_and_report_invalid_digits() {
    use schematic::StrapKind;
    let address = StrapKind::Address7;
    let units = address.standard_units();
    for n in 0..128 {
        for from in units {
            let value = address
                .format_unit(&n.to_string(), units, Some("Decimal"), &from.name)
                .unwrap();
            for to in units {
                let encoded = address
                    .format_unit(&value, units, Some(&from.name), &to.name)
                    .unwrap();
                assert_eq!(
                    address
                        .typed_with_units(&encoded, units, Some(&to.name))
                        .unwrap()
                        .num(),
                    Some(n as f64)
                );
            }
        }
    }
    let source = r#"symbol_library
Register=attribute_type(label:"Register code",kind:"integer",units:[{name:"Count",radix:10,min:0,max:255},{name:"Bank octal",radix:8,prefix:"o:",digits:3},{name:"Word",radix:16,prefix:"$",digits:2}])
Chip=symbol(pins:{},strap_tables:{code:strap_table(kind:Register,unit:"Word",default:"o:110")})"#;
    let (d, _) = run(source).unwrap();
    let t = &d.symbols["Chip"].strap_tables["code"];
    assert_eq!(t.default.as_deref(), Some("$48"));
    assert_eq!(
        t.kind
            .format_unit("$48", &t.units, Some("Word"), "Bank octal")
            .as_deref(),
        Some("o:110")
    );
    assert_eq!(
        t.kind
            .format_unit("110", &t.units, Some("Bank octal"), "Count")
            .as_deref(),
        Some("72")
    );
    for (a, b, kind) in [
        ("o:110", "o:189", "InvalidStrapValue"),
        ("o:110", "$100", "InvalidStrapValue"),
        ("radix:8", "radix:1", "InvalidAttributeUnit"),
        ("radix:8", "factor:2,radix:8", "InvalidAttributeUnit"),
        ("digits:3", "digits:33", "InvalidAttributeUnit"),
        ("prefix:\"o:\"", "prefix:\"$\"", "InvalidAttributeUnit"),
    ] {
        assert_eq!(run(&source.replace(a, b)).unwrap_err().kind, kind);
    }
}

#[test]
fn multiple_conversion_tables_compose_in_both_directions_and_check_every_value() {
    let src = include_str!("../../../examples/electronics/conversion-chain.fg");
    let (doc, _) = run(src).unwrap();
    let t = &doc.attribute_types["Register_code"];
    let cv = |s, f, to| t.conversions.convert(t.kind, s, &t.units, Some(f), to);
    assert_eq!(t.conversions.tables().len(), 2);
    assert_eq!(cv("0x48", "Hex", "Binary").as_deref(), Some("01001000"));
    assert_eq!(cv("01001001", "Binary", "Hex").as_deref(), Some("0x49"));
    assert_eq!(cv("74", "Numeric", "Binary").as_deref(), Some("01001010"));
    assert!(cv("0xFF", "Hex", "Binary").is_none());
    let (d, c) = run(&format!(
        "{}\nmain=sheet()\nu=place(sheet:main,symbol:Device,at:(0,0))",
        src.replacen("symbol_library", "schematic", 1)
    ))
    .unwrap();
    let json = schematic::response(&d, &c, &["symbol".into(), "Device".into()])
        .unwrap()
        .to_string();
    assert!(json.contains("0x4A"));
    for (a, b) in [
        ("{name:\"Binary\"}", "{name:\"Binary\"},{name:\"Orphan\"}"),
        ("[\"73\",\"01001001\"]", "[\"73\",\"01001000\"]"),
        (",[\"74\",\"01001010\"]", ""),
        (
            "from:\"Numeric\",to:\"Binary\"",
            "from:\"Numeric\",to:\"Unknown\"",
        ),
    ] {
        assert_eq!(
            run(&src.replace(a, b)).unwrap_err().kind,
            "InvalidConversionGraph"
        );
    }
    let cycle = src.replace(
        "\n ])",
        ", {name:\"Contradiction\",from:\"Binary\",to:\"Hex\",rows:[[\"01001000\",\"0x49\"]]}\n ])",
    );
    assert_eq!(run(&cycle).unwrap_err().kind, "InvalidConversionGraph");
}

#[test]
fn computed_conversion_paths_obey_intermediate_ranges_and_find_an_alternative() {
    let src = r#"symbol_library
R=attribute_type(label:"Radix", kind:"integer",units:[{name:"Count",radix:10},{name:"Hex",radix:16,prefix:"$"},{name:"Bridge",radix:8,prefix:"o",max:15},{name:"Bits",radix:2,prefix:"b"}],conversions:[
 {from:"Count",to:"Hex"},{from:"Hex",to:"Bridge"},{from:"Bridge",to:"Bits"}])"#;
    let run_path = |s: &str| {
        let (doc, _) = run(s).unwrap();
        let t = &doc.attribute_types["R"];
        t.conversions
            .convert(t.kind, "$48", &t.units, Some("Hex"), "Bits")
    };
    assert!(run_path(src).is_none());
    assert_eq!(
        run_path(&src.replace(
            "{from:\"Bridge\",to:\"Bits\"}",
            "{from:\"Bridge\",to:\"Bits\"},{from:\"Count\",to:\"Bits\"}"
        ))
        .as_deref(),
        Some("b1001000")
    );
}
