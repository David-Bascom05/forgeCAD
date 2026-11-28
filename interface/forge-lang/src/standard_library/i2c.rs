//! One respondent per decoded address on a resolved SDA/SCL pair.
use crate::electronics_library::{
    RuleSpec, Severity,
    worker::{Check, Diagnostic, Fact, Input},
};
use std::collections::BTreeMap;
pub(super) const UNIQUE: Check = Check {
    input: Input::Addresses,
    rule: RuleSpec {
        id: "I2cAddressConflict",
        severity: Severity::Error,
    },
    evaluate: collisions,
};
fn collisions(facts: &[Fact]) -> Vec<Diagnostic> {
    type Devices<'a> = BTreeMap<&'a str, Vec<&'a str>>;
    let mut buses: BTreeMap<(&str, &str, u8), Devices<'_>> = BTreeMap::new();
    for fact in facts {
        if let Fact::Address {
            owner,
            attribute,
            sda,
            scl,
            address,
        } = fact
        {
            buses
                .entry((sda, scl, *address))
                .or_default()
                .entry(owner)
                .or_default()
                .push(attribute);
        }
    }
    buses
        .into_iter()
        .filter(|(_, devices)| devices.len() > 1)
        .map(|((sda, scl, address), devices)| Diagnostic {
            message: format!("Multiple fitted devices respond to 0x{address:02X} on SDA {sda} and SCL {scl}"),
            objects: devices.into_values().flatten().map(String::from).collect(),
        })
        .collect()
}
