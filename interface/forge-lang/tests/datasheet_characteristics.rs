use forge_lang::schematic as sch;
const LIB: &str = include_str!("../../../examples/electronics/datasheet-characteristics.fg");
const BUS: &str = include_str!("../../../examples/electronics/i2c-two-buses.fg");
const SUPPLY: &str = include_str!("../../../examples/electronics/supply-rating.fg");
fn findings(src: &str) -> Vec<String> {
    sch::compile(src, None, None)
        .unwrap()
        .1
        .issues
        .into_iter()
        .map(|i| i.kind.into())
        .collect()
}
#[test]
fn std_loads_once_without_imports_and_shadows_safely() {
    let a = sch::read(BUS, None, None).unwrap();
    let b = sch::read(
        &BUS.replace("std = library(file:\"forge:standard\")", ""),
        None,
        None,
    )
    .unwrap();
    assert_eq!(a.packages.json(), b.packages.json());
    assert_eq!(a.libraries, b.libraries);
    assert_eq!(
        a.attribute_types.keys().collect::<Vec<_>>(),
        b.attribute_types.keys().collect::<Vec<_>>()
    );
    let c = sch::read("schematic\nstd=sheet()", None, None).unwrap();
    assert!(!c.libraries.contains_key("std"));
    assert_eq!(c.packages.json(), a.packages.json());
}
#[test]
fn conditional_specs_preserve_paths_precision_and_ownership() {
    let d = sch::read(LIB, None, None).unwrap();
    let mos = &d.symbols["Mosfet"];
    assert_eq!(mos.specs[1].value.pins, ["G", "S"]);
    assert_eq!(mos.specs[1].value.min, Some(-20.));
    let io = &d.symbols["IoDevice"];
    assert_eq!(io.specs[1].conditions[0].pins, ["VDD", "GND"]);
    assert!(io.pins["VDD"].specs.is_empty());
    let alternate = &io.pins["IO"].functions["sense"].specs;
    let encoded = sch::characteristics::source(alternate);
    let again = sch::read(
        &format!("symbol_library\nX=symbol(pins:{{}},specs:{encoded})"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(&again.symbols["X"].specs, alternate);
    let design = LIB.replacen("symbol_library", "schematic", 1)
        + "\nmain=sheet()\nu=place(sheet:main,symbol:IoDevice,at:(0,0),functions:{IO:sense})";
    let (d, c) = sch::compile(&design, None, None).unwrap();
    let reply = sch::response(&d, &c, &["pin".into(), "main/u.IO".into()])
        .unwrap()
        .to_string();
    assert!(
        reply.contains("leakage_current") && reply.contains("pull_up") && reply.contains("on_resistance")
    );
}
#[test]
fn bad_authored_data_has_diagnostics_but_empty_symbols_are_valid() {
    for (from, to, code) in [
        (
            "pins:[D,S],max:60",
            "pins:[D,Z],max:60",
            "UnknownSpecificationPin",
        ),
        ("min:-20,max:20", "min:21,max:20", "InvalidSpecification"),
        ("unit:\"nA\"", "unit:\"Ω\"", "InvalidSpecification"),
        ("state:\"gate\"", "state:\"gatte\"", "InvalidSpecification"),
        ("class:\"absolute_max\"", "class:\"abs\"", "InvalidRatingClass"),
    ] {
        assert_eq!(
            sch::read(&LIB.replace(from, to), None, None).unwrap_err().kind,
            code
        );
    }
    assert!(sch::read("symbol_library\nX=symbol(pins:{})", None, None).is_ok());
}
#[test]
fn connected_inputs_and_power_inputs_require_appropriate_drivers() {
    let input = SUPPLY
        .replace("kind:\"power_out\"", "kind:\"input\"")
        .replace(",gives:[rail]", "")
        .replace("kind:\"power_in\"", "kind:\"input\"");
    assert_eq!(
        findings(&input).iter().filter(|k| *k == "UndrivenInput").count(),
        1
    );
    let passive = SUPPLY.replace("kind:\"power_out\"", "kind:\"passive\"");
    assert!(!findings(&passive).iter().any(|k| k == "UndrivenPowerInput"));
    assert!(
        findings(&passive.replace(",gives:[rail]", ""))
            .iter()
            .any(|k| k == "UndrivenPowerInput")
    );
}
#[test]
fn only_applicable_declared_bounds_report_voltage_findings() {
    let src=SUPPLY.replace("kind:\"power_in\"",r#"kind:"power_in",specs:[{kind:"pin_voltage",class:"operating",unit:"mV",max:3300},{kind:"pin_voltage",class:"absolute_max",max:4.0}]"#);
    assert!(findings(&src).iter().any(|k| k == "PinOperatingVoltage"));
    assert!(!findings(&src).iter().any(|k| k == "PinAbsoluteVoltage"));
    assert!(
        findings(&src.replace("[3.0,3.6]", "[5,5]"))
            .iter()
            .any(|k| k == "PinAbsoluteVoltage")
    );
    let conditional = src.replace("max:3300", "max:3300,when:[{kind:\"mode\",state:\"standby\"}]");
    for design in [
        conditional,
        src.replace("max:3300", "typ:3300"),
        src.replace(",gives:[rail]", ""),
    ] {
        assert!(!findings(&design).iter().any(|k| k == "PinOperatingVoltage"));
    }
}

#[test]
fn label_checks_use_resolved_nets_and_local_scopes() {
    let src = r#"schematic
X=symbol(pins:{a:pin(number:"1",kind:"passive",at:(0,0)),b:pin(number:"2",kind:"passive",at:(0,4))})
main=sheet()
u=place(sheet:main,symbol:X,at:(0,0))
a=label(sheet:main,on:u.a,value:"ENABLE",at:(0,0))
b=label(sheet:main,on:u.b,value:"Enable",at:(0,4))"#;
    let f = findings(src);
    assert_eq!(f.iter().filter(|r| *r == "SimilarLabelNames").count(), 1);
    assert_eq!(f.iter().filter(|r| *r == "DanglingLabel").count(), 2);
    let f = findings(&src.replace("Enable", "ENABLE"));
    assert!(!f.iter().any(|r| r == "SimilarLabelNames" || r == "DanglingLabel"));
}

const DRIVE: &str = r#"schematic
Driver=symbol(pins:{Y:pin(number:"1",kind:"output",at:(0,0),specs:[
 {kind:"source_current",class:"operating",unit:"mA",max:8},
 {kind:"output_high_voltage",min:2.4},{kind:"output_low_voltage",max:0.3}])})
Receiver=symbol(pins:{A:pin(number:"1",kind:"input",at:(0,0),specs:[
 {kind:"load_current",unit:"mA",max:5},{kind:"logic_high_threshold",min:2.5},{kind:"logic_low_threshold",max:0.4}])})
main=sheet()
y=place(sheet:main,symbol:Driver,at:(0,0))
a=place(sheet:main,symbol:Receiver,at:(10,0))
b=place(sheet:main,symbol:Receiver,at:(20,0))
w=wire(sheet:main,from:y.Y,to:a.A)
x=wire(sheet:main,from:a.A,to:b.A)
light=variant(omit:["main/b"])
"#;
#[test]
fn levels_and_current_checked_per_fitted_net_and_missing_evidence_is_not_a_finding() {
    let (doc, c) = sch::compile(DRIVE, None, None).unwrap();
    assert_eq!(c.issues.iter().filter(|i| i.kind == "LogicHighLevel").count(), 2);
    assert!(!c.issues.iter().any(|i| i.kind == "LogicLowLevel"));
    assert!(
        c.issues
            .iter()
            .any(|i| i.kind == "PinSourceCurrent" && i.objects.len() == 3)
    );
    assert!(c.not_run.iter().any(|r| r.rule == "PinSinkCurrent"
        && r.object == "main/y.Y"
        && r.fields.iter().any(|f| f.starts_with("sink_current"))));
    let fitted = sch::Circuit::build_fitted(&doc, &doc.variants["light"]).unwrap();
    assert!(!fitted.issues.iter().any(|i| i.kind == "PinSourceCurrent"));
    assert!(!fitted.not_run.iter().any(|r| r.object == "main/b.A"));
    for declared in [
        DRIVE.replace("max:8", "max:-8"),
        DRIVE.replace("class:\"operating\"", "class:\"absolute_max\""),
    ] {
        let (_, c) = sch::compile(&declared, None, None).unwrap();
        assert!(!c.issues.iter().any(|i| i.kind == "PinSourceCurrent"));
        assert!(
            c.not_run
                .iter()
                .any(|r| r.rule == "PinSourceCurrent" && r.object == "main/y.Y")
        );
    }
    let no_evidence = DRIVE.replace("min:2.4", "typ:2.4").replace("min:2.5", "typ:2.5");
    let (_, c) = sch::compile(&no_evidence, None, None).unwrap();
    assert!(!c.issues.iter().any(|i| i.kind == "LogicHighLevel"));
    assert_eq!(c.not_run.iter().filter(|i| i.rule == "LogicHighLevel").count(), 3);
    let reply = sch::response(&doc, &fitted, &["erc".into()]).unwrap();
    assert!(reply.get("not_run").unwrap().to_string().contains("main/a.A"));
}
#[test]
fn material_and_component_requirements_distinguish_mismatch_missing_data_and_valid_parts() {
    let design = LIB.replacen("symbol_library", "schematic", 1)
        + r#"
main=sheet()
q=place(sheet:main,symbol:Mosfet,at:(0,0),spec_limits:[
 {kind:"on_resistance",pins:[D,S],unit:"Ω",max:0.0034,when:[{kind:"drain_current",typ:28},{kind:"ambient_temperature",typ:25},{kind:"gate_source_voltage",pins:[G,S],unit:"mV",typ:4500}]},
 {kind:"terminal_role",pins:[G],state:"gate"}
])
"#;
    let run = |s: &str| {
        let (_, c) = sch::compile(s, None, None).unwrap();
        (
            c.issues
                .iter()
                .filter(|i| i.kind == "DatasheetRequirement")
                .count(),
            c.not_run
                .iter()
                .filter(|i| i.rule == "DatasheetRequirement")
                .count(),
        )
    };
    assert_eq!(run(&design), (0, 0));
    assert_eq!(run(&design.replace("max:0.0034", "max:0.0020")), (1, 0));
    assert_eq!(run(&design.replace("typ:4500", "typ:1800")), (0, 1));
    assert_eq!(
        run(&design.replace("pins:[G],state:\"gate\"", "pins:[G],state:\"anode\"")),
        (1, 0)
    );
}

#[test]
fn review_example_names_the_uncharacterized_part_and_keeps_standard_namespace_shared() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics");
    let (d, c) = sch::compile(
        include_str!("../../../examples/electronics/datasheet-review.fg"),
        Some(&dir),
        Some(&dir),
    )
    .unwrap();
    let skipped: Vec<_> = c
        .not_run
        .iter()
        .filter(|r| r.rule == "DatasheetRequirement")
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].object, "main/q2.D");
    assert!(skipped[0].fields[0].contains("1.8"));
    assert!(!c.issues.iter().any(|i| i.kind == "DatasheetRequirement"));
    assert!(!d.libraries.keys().any(|key| key.starts_with("parts.std")));
}

#[test]
fn enabled_datasheet_pull_reaches_its_declared_rail_and_resistance() {
    let design = r#"schematic
bias=role(kind:"pull_up",voltage_range_v:[3,3.6],value_range:["2 kΩ","10 kΩ"])
rail=role(kind:"power",voltage_mv:3300)
U=symbol(pins:{IO:pin(number:"1",kind:"input",at:(0,0),wants:[bias],specs:[
 {kind:"pull_up",state:"enabled",pins:[IO,VDD]},
 {kind:"pull_up_resistance",pins:[IO,VDD],unit:"kΩ",min:4,max:8}]),
 VDD:pin(number:"2",kind:"power_in",at:(0,2))})
J=symbol(pins:{P:pin(number:"1",kind:"power_out",at:(0,0),gives:[rail])})
main=sheet()
u=place(sheet:main,symbol:U,at:(0,0))
j=place(sheet:main,symbol:J,at:(20,0))
supply=wire(sheet:main,from:j.P,to:u.VDD)
"#;
    let f = findings(design);
    assert!(!f.iter().any(|r| r == "PullMismatch"));
    assert!(
        findings(&design.replace("min:4,max:8", "min:12,max:16"))
            .iter()
            .any(|r| r == "PullMismatch")
    );
    let (_, c) = sch::compile(
        &design.replace("state:\"enabled\"", "state:\"configurable\""),
        None,
        None,
    )
    .unwrap();
    assert!(
        c.not_run
            .iter()
            .any(|r| r.rule == "PullMismatch" && r.object == "main/u.IO")
    );
}

#[test]
fn passive_nominals_reach_placements_and_bom_and_voltage_is_across_the_part() {
    let src = r#"schematic
rail=role(kind:"power",voltage_mv:12000)
zero=role(kind:"reference",voltage_mv:0)
J=symbol(pins:{V:pin(number:"1",kind:"power_out",at:(0,0),gives:[rail]),G:pin(number:"2",kind:"power_out",at:(0,2),gives:[zero])})
R=symbol(part_kind:"resistor",pins:{A:pin(number:"1",kind:"passive",at:(-2,0)),B:pin(number:"2",kind:"passive",at:(2,0))},specs:[{kind:"resistance",typ:4.7,unit:"kΩ"},{kind:"working_voltage",pins:[A,B],class:"operating",typ:5,max:10},{kind:"working_voltage",pins:[A,B],class:"absolute_max",max:16}])
C=symbol(part_kind:"capacitor",pins:{},specs:[{kind:"capacitance",unit:"µF",typ:10},{kind:"capacitance_retained",unit:"%",typ:65,when:[{kind:"working_voltage",typ:5}]}])
main=sheet()
j=place(sheet:main,symbol:J,at:(0,0))
r=place(sheet:main,symbol:R,at:(10,0))
c=place(sheet:main,symbol:C,at:(20,0))
substitute=place(sheet:main,symbol:R,at:(30,0),value:"22 kΩ")
wire_a=wire(sheet:main,from:j.G,to:r.A)
wire_b=wire(sheet:main,from:j.V,to:r.B)"#;
    let (doc, c) = sch::compile(src, None, None).unwrap();
    let values = sch::bom(&doc, &c)
        .into_iter()
        .map(|l| l.part.value)
        .collect::<Vec<_>>();
    for value in ["4.7 kΩ", "10 µF", "22 kΩ"] {
        assert!(values.iter().any(|v| v == value), "{values:?}");
    }
    assert!(
        c.issues
            .iter()
            .any(|i| i.kind == "PinOperatingVoltage" && i.objects.contains(&"main/r.A".into()))
    );
    assert!(!c.issues.iter().any(|i| i.kind == "PinAbsoluteVoltage"));
    assert_eq!(
        sch::read(&src.replace("typ:65", "typ:101"), None, None)
            .unwrap_err()
            .kind,
        "InvalidSpecification"
    );
}

#[test]
fn every_fet_catalog_entry_is_unique_typed_and_round_trips() {
    use sch::characteristics::{self as ch, Reading, Specification, fet};
    use std::collections::BTreeSet;
    let mut kinds = BTreeSet::new();
    let mut specs = vec![];
    for p in fet::groups("fet").flat_map(|g| g.properties) {
        assert!(kinds.insert(p.kind), "duplicate {}", p.kind);
        let mut value = Reading {
            kind: p.kind.into(),
            ..Default::default()
        };
        if p.unit.is_empty() {
            let choices = ch::states(p.kind).unwrap_or_else(|| panic!("missing states for {}", p.kind));
            value.state = choices.first().copied().unwrap_or("JEDEC test board").into();
        } else {
            let units = ch::units(p.kind).unwrap_or_else(|| panic!("missing units for {}", p.kind));
            assert!(units.contains(&p.unit), "{}", p.kind);
            value.unit = p.unit.into();
            value.max = Some(0.5);
        }
        specs.push(Specification {
            value,
            class: "characteristic".into(),
            ..Default::default()
        });
    }
    let src = format!("symbol_library\nQ=symbol(pins:{{}},specs:{})", ch::source(&specs));
    assert_eq!(sch::read(&src, None, None).unwrap().symbols["Q"].specs, specs);
    let available =
        |part: &str, kind: &str| fet::groups(part).any(|g| g.properties.iter().any(|p| p.kind == kind));
    for (part, leakage, conduction) in [
        ("mosfet", true, false),
        ("jfet", false, true),
        ("fet", true, true),
        ("resistor", false, false),
    ] {
        assert_eq!(available(part, "off_state_drain_current"), leakage);
        assert_eq!(available(part, "zero_gate_drain_current"), conduction);
    }
    for (kind, unit, scale) in [
        ("commutation_current_slew_rate", "A/µs", 1e6),
        ("drain_voltage_slew_rate", "V/ns", 1e9),
        ("on_resistance_temperature_coefficient", "ppm/°C", 1e-4),
        ("threshold_temperature_coefficient", "mV/°C", 1e-3),
        ("transient_thermal_impedance_junction_case", "K/kW", 1e-3),
        ("current_noise_density", "fA/√Hz", 1e-15),
        ("s21_phase", "rad", 180. / std::f64::consts::PI),
        ("copper_area", "m²", 1e6),
    ] {
        assert_eq!(
            Reading {
                kind: kind.into(),
                unit: unit.into(),
                ..Default::default()
            }
            .unit_scale(),
            scale
        );
    }
    assert_eq!(
        sch::read(&LIB.replace("unit:\"A/µs\"", "unit:\"V/ns\""), None, None)
            .unwrap_err()
            .kind,
        "InvalidSpecification"
    );
}

#[test]
fn fet_requirements_respect_measurement_conditions_signed_bounds_and_missing_maxima() {
    let base = LIB.replacen("symbol_library", "schematic", 1);
    let run = |symbol: &str, requirement: &str| {
        let src = format!(
            "{base}\nmain=sheet()\nq=place(sheet:main,symbol:{symbol},at:(0,0),spec_limits:[{requirement}])"
        );
        let (_, c) = sch::compile(&src, None, None).unwrap();
        (
            c.issues.iter().any(|i| i.kind == "DatasheetRequirement"),
            c.not_run.iter().any(|i| i.rule == "DatasheetRequirement"),
        )
    };
    let qrr = r#"{kind:"recovery_charge",unit:"µC",typ:0.145,when:[{kind:"ambient_temperature",typ:25},{kind:"forward_current",pins:[S,D],typ:28},{kind:"drain_source_voltage",pins:[D,S],typ:30},{kind:"commutation_current_slew_rate",unit:"A/ns",typ:0.3}]}"#;
    assert_eq!(run("Mosfet", qrr), (false, false));
    assert_eq!(
        run("Mosfet", &qrr.replace("typ:0.145", "max:0.145")),
        (false, true)
    );
    assert_eq!(run("Mosfet", &qrr.replace("typ:0.3", "typ:0.4")), (false, true));
    let cutoff = r#"{kind:"gate_source_cutoff_voltage",pins:[G,S],unit:"mV",min:-5000,max:-1000,when:[{kind:"ambient_temperature",typ:25},{kind:"drain_current",pins:[D,S],typ:1,unit:"µA"},{kind:"drain_source_voltage",pins:[D,S],typ:5}]}"#;
    assert_eq!(run("J112", cutoff), (false, false));
    assert_eq!(
        run("J112", &cutoff.replace("min:-5000", "min:-4500")),
        (true, false)
    );
    assert_eq!(
        run("J112", &cutoff.replace("pins:[G,S]", "pins:[S,G]")),
        (false, true)
    );
}

#[test]
fn rf_power_preserves_logarithmic_units_through_erc_and_editing() {
    use sch::characteristics::Reading;
    let power = Reading {
        kind: "compression_output_power".into(),
        unit: "dBm".into(),
        min: Some(-10.),
        typ: Some(30.),
        ..Default::default()
    };
    let linear = power.converted("mW").unwrap();
    assert!((linear.min.unwrap() - 0.1).abs() < 1e-12);
    assert_eq!(linear.typ, Some(1000.));
    assert_eq!(linear.converted("dBm"), Some(power));
    assert!(
        Reading {
            typ: Some(0.),
            ..linear
        }
        .converted("dBm")
        .is_none()
    );
    let src = r#"schematic
Q=symbol(part_kind:"fet",pins:{},specs:[{kind:"compression_output_power",min:30,unit:"dBm"}])
main=sheet()
q=place(sheet:main,symbol:Q,at:(0,0),spec_limits:[{kind:"compression_output_power",min:1,unit:"W"}])"#;
    let (d, c) = sch::compile(src, None, None).unwrap();
    assert!(d.symbols.contains_key("Q"));
    assert!(!c.issues.iter().any(|i| i.kind == "DatasheetRequirement"));
    assert!(!c.not_run.iter().any(|i| i.rule == "DatasheetRequirement"));
    assert!(
        findings(&src.replace("min:1,", "min:2,"))
            .iter()
            .any(|i| i == "DatasheetRequirement")
    );
}

#[test]
fn gate_charge_follows_the_selected_drive_and_charge_definitions_are_distinct() {
    let base = LIB.replacen("symbol_library", "schematic", 1);
    let check = |requirement: &str| {
        let design = format!(
            "{base}\nmain=sheet()\nq=place(sheet:main,symbol:Mosfet,at:(0,0),spec_limits:[{requirement}])"
        );
        let (_, c) = sch::compile(&design, None, None).unwrap();
        (
            c.issues.iter().any(|i| i.kind == "DatasheetRequirement"),
            c.not_run.iter().any(|r| r.rule == "DatasheetRequirement"),
        )
    };
    let gate = r#"{kind:"gate_charge",pins:[G,S],max:27,unit:"nC",when:[
        {kind:"gate_source_voltage",pins:[G,S],typ:4.5},{kind:"drain_source_voltage",pins:[D,S],typ:30},
        {kind:"drain_current",pins:[D,S],typ:28},{kind:"ambient_temperature",typ:25}]}"#;
    assert_eq!(check(gate), (false, false));
    assert_eq!(check(&gate.replace("typ:4.5", "typ:10")), (true, false));
    assert_eq!(check(&gate.replace("typ:4.5", "typ:6")), (false, true));

    let charge = r#"{kind:"recovery_charge",typ:145,unit:"nC",when:[
        {kind:"drain_source_voltage",pins:[D,S],typ:30},{kind:"forward_current",pins:[S,D],typ:28},
        {kind:"commutation_current_slew_rate",typ:300,unit:"A/µs"},{kind:"ambient_temperature",typ:25}]}"#;
    for (kind, skipped) in [
        ("recovery_charge", false),
        ("capacitive_charge", true),
        ("commutation_charge", true),
    ] {
        assert_eq!(check(&charge.replace("recovery_charge", kind)), (false, skipped));
    }
}

#[test]
fn named_sweeps_preserve_axes_bounds_provenance_and_reject_ambiguous_points() {
    let doc = sch::read(LIB, None, None).unwrap();
    let points: Vec<_> = doc.symbols["Mosfet"]
        .specs
        .iter()
        .filter(|s| !s.curve.is_empty())
        .cloned()
        .collect();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].conditions[0].typ, Some(4.5));
    assert_eq!(points[1].conditions[0].typ, Some(10.));
    let encoded = sch::characteristics::source(&points);
    let src = format!(
        r#"symbol_library
T=symbol(pins:{{
    G:pin(number:"1",kind:"passive",at:(0,0)),
    D:pin(number:"2",kind:"passive",at:(1,0)),
    S:pin(number:"3",kind:"passive",at:(2,0))
}},specs:{encoded})"#
    );
    assert_eq!(sch::read(&src, None, None).unwrap().symbols["T"].specs, points);
    for bad in [
        LIB.replace(
            "coordinate:{kind:\"gate_source_voltage\",pins:[G,S],typ:10}",
            "coordinate:{kind:\"gate_source_voltage\",pins:[G,S],typ:4.5}",
        ),
        LIB.replace(
            "coordinate:{kind:\"gate_source_voltage\",pins:[G,S],typ:10}",
            "coordinate:{kind:\"gate_source_voltage\",pins:[G,S],max:10}",
        ),
        LIB.replace(
            "coordinate:{kind:\"gate_source_voltage\",pins:[G,S],typ:10}",
            "coordinate:{kind:\"gate_source_voltage\",pins:[S,G],typ:10}",
        ),
    ] {
        assert_eq!(
            sch::read(&bad, None, None).unwrap_err().kind,
            "InvalidDatasheetCurve"
        );
    }
}

#[test]
fn two_noise_traces_keep_their_bias_and_a_plotted_bound_needs_a_guarantee() {
    let src = LIB.replacen("symbol_library", "schematic", 1);
    let test = |design: &str, n: &str, bias: &str, bound: &str| {
        let src = format!(
            r#"{design}
main=sheet()
q=place(sheet:main,symbol:MatchedPair,at:(0,0),spec_limits:[{{kind:"voltage_noise_density",unit:"nV/√Hz",{bound}:{n},
when:[{{kind:"ambient_temperature",typ:25}},{{kind:"drain_source_voltage",typ:10}},{{kind:"drain_current",unit:"mA",typ:{bias}}},{{kind:"frequency",typ:1000}}]}}])"#
        );
        let (_, c) = sch::compile(&src, None, None).unwrap();
        (
            c.issues.iter().any(|i| i.kind == "DatasheetRequirement"),
            c.not_run.iter().any(|r| r.rule == "DatasheetRequirement"),
        )
    };
    assert_eq!(test(&src, "1.1", "2", "typ"), (false, false));
    assert_eq!(test(&src, "1.1", "0.1", "typ"), (true, false));
    assert_eq!(test(&src, "1.1", "1", "typ"), (false, true));
    let published_limit = src.replace(
        r#"kind:"voltage_noise_density",unit:"nV/√Hz",typ:"#,
        r#"kind:"voltage_noise_density",unit:"nV/√Hz",max:"#,
    );
    assert_eq!(test(&published_limit, "1.1", "2", "max"), (false, false));
    for basis in ["typical_curve", "bench_measurement", "simulation"] {
        let illustrative = published_limit.replace("datasheet_table", basis);
        assert_eq!(test(&illustrative, "1.1", "2", "max"), (false, true));
    }
    let invalid = src.replace(
        "coordinate:{kind:\"frequency\",unit:\"Hz\",typ:10}",
        "coordinate:{kind:\"frequency\",unit:\"Hz\",typ:0}",
    );
    assert_eq!(
        sch::read(&invalid, None, None).unwrap_err().kind,
        "InvalidDatasheetCurve"
    );
}
