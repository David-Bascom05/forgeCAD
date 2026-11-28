use forge_lang::pcb::{self, Rule, Witness};
use std::path::Path;

const SRC: &str = include_str!("../../../examples/electronics/controller-board.fg");
fn dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/electronics"))
}
fn read(src: &str) -> Result<pcb::Board, forge_lang::schematic::Fault> {
    pcb::compile(src, Some(dir()), None)
}
fn changed(a: &str, b: &str) -> String {
    assert!(SRC.contains(a));
    SRC.replacen(a, b, 1)
}

#[test]
fn physical_stack_and_routed_net_are_independent_of_visible_names() {
    let b = read(SRC).unwrap();
    assert_eq!(b.layers.iter().map(|l| l.thickness_nm).sum::<i64>(), 1_600_000);
    assert_eq!(b.tracks["feed"].points.last(), Some(&b.vias["transition"].at));
    assert_eq!(
        b.tracks["on_bottom"].points.first(),
        Some(&b.vias["transition"].at)
    );
    assert_ne!(b.nets["supply"].name, b.nets["ground"].name);
    assert_eq!(b.nets["supply"].terminal.as_deref(), Some("main/u.vdd"));
    assert_eq!(b.tracks["feed"].net, "supply");
    assert_eq!(b.check().rules.len(), 6);
    assert!(b.check().findings.is_empty());
    let result = pcb::response(&b, &["handoff".into()]).unwrap();
    let handoff = result.get("result").unwrap();
    let Some(forge_agent::json::Json::Arr(rows)) = handoff.get("relationships") else {
        panic!("missing relationships")
    };
    let bypass = rows
        .iter()
        .find(|x| x.get("id").and_then(|v| v.str()) == Some("main/bypass"))
        .unwrap();
    assert_eq!(bypass.get("return_from").unwrap().str(), Some("main/c.minus"));
    assert_eq!(bypass.get("return_to").unwrap().str(), Some("main/u.vss"));
}

#[test]
fn invalid_declarations_report_the_offending_source() {
    for (from, to, kind) in [
        (
            "(board_width, 0), (board_width, 20)",
            "(board_width, 20), (board_width, 0)",
            "InvalidBoardEdge",
        ),
        ("upper, inner1", "upper, top", "InvalidStackLayer"),
        ("top, upper, inner1", "top, inner1, upper", "AdjacentCopperLayers"),
        ("upper, inner1, core", "upper, core", "InvalidStack"),
        ("layer: top", "layer: upper", "ExpectedCopperLayer"),
        ("layer: top", "layer: ghost", "ExpectedCopperLayer"),
        ("width: trace_width", "width: 2deg", "WrongDim"),
        ("width: trace_width", "width: 0.0000001mm", "BoardGrid"),
        ("net: supply", "net: missing", "UnknownBoardNet"),
        ("main/u.vdd", "main/u.missing", "UnknownTerminal"),
        ("drill: 0.3mm", "drill: 0.6mm", "InvalidViaLand"),
        ("(9, 8)", "(4, 8)", "InvalidTrack"),
        ("weight_oz_ft2: 1", "weight_oz_ft2: 1mm", "WrongDim"),
        ("controller.fg", "../../README.md", "SchematicUnavailable"),
    ] {
        let src = changed(from, to);
        let e = read(&src).unwrap_err();
        assert_eq!(e.kind, kind, "{from}: {e:?}");
        assert!(e.span.hi > e.span.lo);
    }
}

#[test]
fn process_thresholds_preserve_half_nm_and_accept_equality() {
    let src = SRC
        .replace("trace_width = 0.25mm", "trace_width = 0.2mm")
        .replace("diameter: 0.6mm, drill: 0.3mm", "diameter: 0.5mm, drill: 0.2mm");
    assert!(read(&src).unwrap().check().findings.is_empty());
    let src = src
        .replace("0.5mm, drill: 0.2mm", "0.499998mm, drill: 0.199999mm")
        .replace("trace_width = 0.2mm", "trace_width = 0.199999mm");
    let b = read(&src).unwrap();
    let findings = b.check().findings;
    assert_eq!(findings.len(), 4);
    assert_eq!(
        findings.iter().filter(|f| f.rule == Rule::MinTrackWidth).count(),
        2
    );
    let annular = findings.iter().find(|f| f.rule == Rule::MinAnnularRing).unwrap();
    assert!(matches!(
        annular.witness,
        Witness::BelowMinimum {
            twice_measured_nm: 299999,
            minimum_nm: 150000
        }
    ));
}

#[test]
fn copper_envelope_checks_concavities_tangencies_and_negative_coordinates() {
    let mut b = read(SRC).unwrap();
    let mm = 1_000_000;
    b.outline = vec![
        [-5, -5],
        [5, -5],
        [5, 5],
        [1, 5],
        [1, 1],
        [-1, 1],
        [-1, 5],
        [-5, 5],
    ]
    .into_iter()
    .map(|[x, y]| [x * mm, y * mm])
    .collect();
    b.tracks.retain(|k, _| k == "feed");
    b.vias.clear();
    b.process = None;
    let set = |b: &mut pcb::Board, y| {
        b.tracks.get_mut("feed").unwrap().points = vec![[-3 * mm, y], [3 * mm, y]];
    };
    set(&mut b, 3 * mm);
    assert_eq!(b.check().findings[0].rule, Rule::CopperOutsideEdge);
    let tangent = mm - b.tracks["feed"].width_nm / 2;
    set(&mut b, tangent);
    assert!(b.check().findings.is_empty());
    b.outline.reverse();
    assert!(b.check().findings.is_empty());
    set(&mut b, tangent + 1);
    assert_eq!(b.check().findings.len(), 1);
    assert_eq!(b.check().rules, vec![Rule::CopperOutsideEdge, Rule::ShortCircuit]);
}

#[test]
fn an_unlinked_board_accepts_local_nets_and_inch_geometry() {
    let src = SRC
        .lines()
        .filter(|l| !l.starts_with("source = "))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("terminal: \"main/u.vdd\"", "name: \"VDD\"")
        .replace("terminal: \"main/u.vss\"", "name: \"VSS\"")
        .replace("board_width = 40mm", "board_width = 2in");
    let b = pcb::compile(&src, None, None).unwrap();
    assert_eq!(b.outline[1][0], 50_800_000);
    assert!(b.schematic.is_none());
}

#[test]
fn spacing_is_layer_and_net_specific_and_accounts_for_via_lands() {
    let mut b = read(SRC).unwrap();
    let width = 200_000;
    let threshold = 350_000;
    b.tracks.retain(|k, _| k != "on_bottom");
    b.vias.clear();
    let feed = b.tracks.get_mut("feed").unwrap();
    feed.points = vec![[2_000_000, 2_000_000], [5_000_000, 2_000_000]];
    feed.width_nm = width;
    let other = b.tracks.get_mut("return_trace").unwrap();
    other.points = vec![
        [2_000_000, 2_000_000 + threshold],
        [5_000_000, 2_000_000 + threshold],
    ];
    other.width_nm = width;
    other.layer = "top".into();
    let count = |b: &pcb::Board| {
        b.check()
            .findings
            .iter()
            .filter(|f| f.rule == Rule::MinCopperSpacing)
            .count()
    };
    assert_eq!(count(&b), 0);
    for p in &mut b.tracks.get_mut("return_trace").unwrap().points {
        p[1] -= 1;
    }
    assert_eq!(count(&b), 1);
    b.tracks.get_mut("return_trace").unwrap().net = "supply".into();
    assert_eq!(count(&b), 0);
    let other = b.tracks.get_mut("return_trace").unwrap();
    other.net = "ground".into();
    other.layer = "bottom".into();
    assert_eq!(count(&b), 0);
    let mut via = read(SRC).unwrap().vias.remove("transition").unwrap();
    via.at = [3_000_000, 2_000_000 + threshold];
    b.vias.insert("v".into(), via);
    assert_eq!(count(&b), 1);
    assert!(
        b.check()
            .findings
            .iter()
            .any(|f| matches!(f.witness, Witness::Spacing { layer: "bottom", .. }))
    );
}

#[test]
fn nested_rule_libraries_apply_severity_to_actual_drc_findings() {
    let dir = forge_dirs::temp_dir().join(format!("fg-board-rule-packs-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, src: &str| std::fs::write(dir.join(name), src).unwrap();
    write(
        "review.fg",
        "footprint_library\nw=drc_rule(rule:\"MinTrackWidth\",severity:\"info\")",
    );
    write("house.fg", "footprint_library\nr=library(file:\"review.fg\")");
    write(
        "production.fg",
        "footprint_library\nw=drc_rule(rule:\"MinTrackWidth\",severity:\"error\")",
    );
    for file in ["controller.fg", "power-symbols.fg"] {
        std::fs::copy(self::dir().join(file), dir.join(file)).unwrap();
    }
    let src = format!(
        "{}\nhouse=library(file:\"house.fg\")",
        changed("trace_width = 0.25mm", "trace_width = 0.19mm")
    );
    let compile = |s: &str| pcb::compile(s, Some(&dir), Some(&dir));
    let b = compile(&src).unwrap();
    let report = pcb::response(&b, &["drc".into()]).unwrap();
    assert_eq!(report.get("ok").unwrap().to_string(), "true");
    assert!(report.to_string().contains("\"severity\":\"info\""));
    let conflict = format!("{src}\nproduction=library(file:\"production.fg\")");
    assert_eq!(compile(&conflict).unwrap_err().kind, "ConflictingRuleDefaults");
    let b = compile(&format!(
        "{conflict}\nw=drc_rule(rule:\"MinTrackWidth\",severity:\"error\")"
    ))
    .unwrap();
    let report = pcb::response(&b, &["drc".into()]).unwrap();
    assert_eq!(report.get("ok").unwrap().to_string(), "false");
    assert_eq!(
        compile(&src.replace(
            "house=library(file:\"house.fg\")",
            "w=drc_rule(rule:\"Bogus\",severity:\"info\")"
        ))
        .unwrap_err()
        .kind,
        "UnknownDrcRule"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
