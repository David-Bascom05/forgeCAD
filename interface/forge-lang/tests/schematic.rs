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
gateB = gate(sheet: cell, of: U2, unit: "B", at: (20,0))
spare = place(sheet: cell, symbol: S, at: (30,0), populated: false)
fixture = place(sheet: cell, symbol: S, at: (40,0), in_bom: false)
"#;
    let (d, c) = run(source).unwrap();
    let lines = schematic::bom(&d, &c);
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| l.references.len() == 2));
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
    let dir = std::env::temp_dir().join(format!("forge-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let owner = dir.join("library.fg");
    let bytes = b"%PDF-1.4\n%%EOF";
    let path = store_pdf(&owner, bytes).unwrap();
    assert_eq!(store_pdf(&owner, bytes).unwrap(), path);
    assert!(store_pdf(&owner, b"other").is_err());
    assert_eq!(std::fs::read(dir.join(path)).unwrap(), bytes);
    std::fs::remove_dir_all(dir).unwrap();
}
