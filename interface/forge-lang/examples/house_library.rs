//! A distributable Rust electronics library. Build this executable once per native target.
use forge_lang::electronics_library::{
    Library, RuleSpec, Severity,
    worker::{self, Check, Diagnostic, Fact, Input},
};
use forge_lang::schematic::{AttributeType, AttributeUnit, ConversionGraph, StrapKind};
use std::collections::BTreeMap;

fn types() -> BTreeMap<String, AttributeType> {
    let units = vec![
        AttributeUnit {
            name: "A".into(),
            factor: 1.,
            radix: None,
            min: Some(0.),
            max: Some(20.),
        },
        AttributeUnit {
            name: "mA".into(),
            factor: 0.001,
            radix: None,
            min: Some(0.),
            max: Some(20000.),
        },
    ];
    let kind = StrapKind::Quantity;
    let conversions = ConversionGraph::build(kind, &units, ConversionGraph::defaults(&units)).unwrap();
    BTreeMap::from([(
        "output_current".into(),
        AttributeType {
            label: "Rated output current".into(),
            origin: None,
            kind,
            units,
            conversions,
            modes: vec![],
            range: None,
        },
    )])
}
fn purchasing(parts: &[Fact]) -> Vec<Diagnostic> {
    parts
        .iter()
        .filter_map(|p| {
            let Fact::Component { id, fields, .. } = p else {
                return None;
            };
            fields
                .get("MPN")
                .is_none_or(|s| s.trim().is_empty())
                .then(|| Diagnostic {
                    objects: vec![id.clone()],
                    message: "The fitted component has no manufacturer part number".into(),
                })
        })
        .collect()
}
fn power_feed(tracks: &[Fact]) -> Vec<Diagnostic> {
    tracks
        .iter()
        .filter_map(|p| {
            let Fact::Track {
                id, net, width_nm, ..
            } = p
            else {
                return None;
            };
            (net == "48V" && *width_nm < 500_000).then(|| Diagnostic {
                objects: vec![id.clone()],
                message: format!("48V feed width is {width_nm} nm; the house minimum is 500000 nm"),
            })
        })
        .collect()
}
static RATINGS: Library = Library::new("ratings").with_properties(types);
static ASSEMBLY: Library = Library::new("assembly").with_checks(&[Check {
    rule: RuleSpec {
        id: "house.MissingMPN",
        severity: Severity::Warning,
    },
    input: Input::Components,
    evaluate: purchasing,
}]);
static LAYOUT: Library = Library::new("layout").with_checks(&[Check {
    rule: RuleSpec {
        id: "house.PowerFeedWidth",
        severity: Severity::Error,
    },
    input: Input::Tracks,
    evaluate: power_feed,
}]);
static REVIEW: Library = Library::new("review").with_children(&[&ASSEMBLY, &LAYOUT]);
static HOUSE: Library = Library::new("house").with_children(&[&RATINGS, &REVIEW]);
fn main() -> Result<(), String> {
    worker::serve(&HOUSE, "1.0.0")
}
