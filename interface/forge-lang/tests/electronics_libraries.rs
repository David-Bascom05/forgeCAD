use forge_lang::{
    electronics_library::{Library, Loaded, RuleSpec, Severity, register},
    pcb, schematic as sch,
};
const BUSES: &str = include_str!("../../../examples/electronics/i2c-two-buses.fg");
const SUPPLY: &str = include_str!("../../../examples/electronics/supply-rating.fg");
fn circuit(s: &str) -> (sch::Document, sch::Circuit) {
    sch::compile(s, None, None).unwrap()
}
fn collision(c: &sch::Circuit) -> usize {
    c.issues.iter().filter(|i| i.kind == "I2cAddressConflict").count()
}
#[test]
fn same_address_on_two_live_buses_is_valid() {
    let (_, c) = circuit(BUSES);
    assert_eq!(collision(&c), 0);
    for (a, b) in [("right.SDA", "left.SDA"), ("right.SCL", "left.SCL")] {
        let (_, c) = circuit(&BUSES.replace(a, b));
        assert_eq!(collision(&c), 0);
    }
    let common = BUSES
        .replace("right.SDA", "left.SDA")
        .replace("right.SCL", "left.SCL");
    let (doc, c) = circuit(&common);
    assert_eq!(collision(&c), 1);
    let i = c.issues.iter().find(|i| i.kind == "I2cAddressConflict").unwrap();
    assert_eq!(i.objects, ["main/sensor_a:address", "main/sensor_b:address"]);
    let variant = sch::Circuit::build_fitted(&doc, &doc.variants["omit_b"]).unwrap();
    assert_eq!(collision(&variant), 0);
}
#[test]
fn optional_rating_checks_canonical_units_overrides_and_evidence() {
    let (doc, c) = circuit(SUPPLY);
    assert!(!c.issues.iter().any(|i| i.kind == "SupplyAboveRating"));
    let line = sch::bom(&doc, &c)
        .into_iter()
        .find(|l| l.part.symbol == "Chip")
        .unwrap();
    assert_eq!(
        line.typed_fields
            .get("MaxSupply")
            .unwrap()
            .get("value")
            .unwrap()
            .get("value")
            .unwrap()
            .num(),
        Some(3.6)
    );
    let over = SUPPLY.replace("[3.0,3.6]", "[4.9,5.1]");
    let (_, c) = circuit(&over);
    let finding = c.issues.iter().find(|i| i.kind == "SupplyAboveRating").unwrap();
    assert!(finding.objects.iter().any(|x| x == "main/reg.OUT"));
    assert!(finding.objects.iter().any(|x| x == "main/mcu.VDD"));
    let property_only = over
        .lines()
        .filter(|l| !l.starts_with("power_review"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        circuit(&property_only)
            .1
            .issues
            .iter()
            .any(|i| i.kind == "SupplyAboveRating")
    );
    let rated = over.replace(
        "symbol:Chip,at:",
        r#"symbol:Chip,fields:{MaxSupply:"5200 mV"},at:"#,
    );
    assert!(
        !circuit(&rated)
            .1
            .issues
            .iter()
            .any(|i| i.kind == "SupplyAboveRating")
    );
    assert_eq!(
        sch::compile(&rated.replace("5200 mV", "5 A"), None, None)
            .unwrap_err()
            .kind,
        "InvalidPropertyValue"
    );
}
fn sample(_: &sch::Document, _: &sch::Circuit, out: &mut Vec<sch::Issue>) {
    out.push(sch::Issue {
        kind: "house.Test",
        unknown: false,
        message: "demonstration".into(),
        objects: vec!["main".into()],
    });
}
static CHILD: Library = Library::new("child").with_erc(
    &[RuleSpec {
        id: "house.Test",
        severity: Severity::Info,
    }],
    sample,
);
static OVER: Library = Library::new("over").with_children(&[&CHILD]);
static HOUSE: Library = Library::new("example_house").with_children(&[&CHILD, &OVER]);
static CYCLE: Library = Library::new("loop").with_children(&[&CYCLE]);
#[test]
fn nested_rust_exports_run_once_and_severity_resolves_before_import() {
    register(&HOUSE).unwrap();
    let (d, c) = circuit(
        "schematic\npolicy=erc_rule(rule:\"house.Test\",severity:\"error\")\nlib=library(file:\"rust:example_house\")\nagain=library(file:\"rust:example_house/over/child\")\nmain=sheet()",
    );
    let findings = c
        .issues
        .iter()
        .filter(|i| i.kind == "house.Test")
        .collect::<Vec<_>>();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].priority(&d), "error");
    let mut loaded = Loaded::default();
    assert!(loaded.add(&CYCLE).is_err());
    assert_eq!(loaded.erc_rules().count(), 0);
    let standard = Loaded::standard();
    for r in pcb::Rule::ALL {
        assert!(standard.drc_rules().any(|(id, _)| id == r.name()));
    }
    assert_eq!(standard.drc_rules().count(), pcb::Rule::ALL.len());
}
