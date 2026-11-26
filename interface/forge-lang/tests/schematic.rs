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
        ["UnknownSupplyVoltage"]
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
        assert_eq!(check.providers[0].pin, "main/J1.OUT");
        let failed = status == "out_of_range";
        assert_eq!(
            c.issues
                .iter()
                .any(|i| i.kind == "VoltageOutOfRange" && i.objects == ["main/U1.VDD", "main/J1.OUT"]),
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
                .any(|r| r.role == *id && r.peers == [("main/J1.OUT".into(), expected)])
        );
        let report = schematic::response(&d, &c, &["pin".into(), "main/U1.VDD".into()]).unwrap();
        let Some(forge_agent::json::Json::Arr(checks)) =
            report.get("result").and_then(|r| r.get("voltage_checks"))
        else {
            panic!("missing voltage checks from pin query");
        };
        assert_eq!(checks[0].get("status").and_then(|s| s.str()), Some(status));
        let provider = schematic::inspect_pin(&c, "main/J1.OUT").unwrap();
        assert!(
            provider
                .checks
                .iter()
                .any(|row| !row.need && row.peers.contains(&("main/U1.VDD".into(), expected)))
        );
    }
    for change in [
        sheet.replace("symbol: Supply, at:", "symbol: Supply, populated: false, at:"),
        sheet.replace("feed = wire(sheet: main, from: J1.OUT, to: vdd)", ""),
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
