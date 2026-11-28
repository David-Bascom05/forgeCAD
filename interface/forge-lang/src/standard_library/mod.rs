//! Standard Library. Children are independently importable Rust libraries.
use crate::{
    electronics_library::*,
    pcb,
    schematic::{self as sch, Circuit, Document, Issue},
};
use Severity::{Error, Warning};
use std::collections::BTreeMap;
mod properties;
mod spec_limits;
pub(crate) use properties::types;
pub static SPEC_LIMITS: Library = Library::new("datasheet").with_erc(
    &[RuleSpec {
        id: "DatasheetRequirement",
        severity: Error,
    }],
    spec_limits::check,
);
pub static RATINGS: Library = Library::new("ratings").with_properties(properties::ratings);
pub static PROPERTIES: Library = Library::new("properties")
    .with_properties(types)
    .with_children(&[&RATINGS]);
pub static BUSES: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "DiffPairShort",
            severity: Error,
        },
        RuleSpec {
            id: "DiffPairConflict",
            severity: Error,
        },
        RuleSpec {
            id: "BusMemberShort",
            severity: Error,
        },
    ],
    resolve: Some(sch::standard::buses),
    ..Library::new("buses")
};
pub static WIRING: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "WireNearPin",
            severity: Warning,
        },
        RuleSpec {
            id: "WireOverlap",
            severity: Warning,
        },
        RuleSpec {
            id: "TextBoundsOverlap",
            severity: Warning,
        },
        RuleSpec {
            id: "TextGeometryOverlap",
            severity: Warning,
        },
    ],
    resolve: Some(sch::drawings),
    ..Library::new("wiring")
};
pub static VOLTAGE: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "LogicHighLevel",
            severity: Error,
        },
        RuleSpec {
            id: "LogicLowLevel",
            severity: Error,
        },
        RuleSpec {
            id: "PinSourceCurrent",
            severity: Error,
        },
        RuleSpec {
            id: "PinSinkCurrent",
            severity: Error,
        },
        RuleSpec {
            id: "PinOperatingVoltage",
            severity: Warning,
        },
        RuleSpec {
            id: "PinAbsoluteVoltage",
            severity: Error,
        },
        RuleSpec {
            id: "VoltageOutOfRange",
            severity: Error,
        },
        RuleSpec {
            id: "UnknownSupplyVoltage",
            severity: Warning,
        },
    ],
    resolve: Some(sch::standard::electrical),
    ..Library::new("voltage")
};
pub static INTENT: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "PinConnectionMismatch",
            severity: Error,
        },
        RuleSpec {
            id: "PullMismatch",
            severity: Error,
        },
    ],
    resolve: Some(sch::intent),
    ..Library::new("intent")
};
mod i2c;
pub static ATTRIBUTES: Library = Library {
    worker_checks: &[i2c::UNIQUE],
    erc_rules: &[
        RuleSpec {
            id: "ReservedI2cAddress",
            severity: Warning,
        },
        RuleSpec {
            id: "UnknownI2cBus",
            severity: Warning,
        },
        RuleSpec {
            id: "UnresolvedAttribute",
            severity: Warning,
        },
        RuleSpec {
            id: "CurrentOutsideRange",
            severity: Warning,
        },
        RuleSpec {
            id: "UnsupportedBootMode",
            severity: Warning,
        },
        RuleSpec {
            id: "UnsupportedAttributeValue",
            severity: Warning,
        },
    ],
    resolve: Some(sch::attributes),
    ..Library::new("attributes")
};
pub static SIGNALS: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "PinFunctionMismatch",
            severity: Warning,
        },
        RuleSpec {
            id: "UnknownPinFunctionCompatibility",
            severity: Warning,
        },
    ],
    resolve: Some(sch::signals),
    ..Library::new("signals")
};
pub static CONNECTIVITY: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "DriverConflict",
            severity: Error,
        },
        RuleSpec {
            id: "ConnectedNoConnect",
            severity: Error,
        },
        RuleSpec {
            id: "ReturnMismatch",
            severity: Error,
        },
        RuleSpec {
            id: "DisconnectedFulfillment",
            severity: Error,
        },
        RuleSpec {
            id: "NetLabelConflict",
            severity: Error,
        },
        RuleSpec {
            id: "UndrivenInput",
            severity: Error,
        },
        RuleSpec {
            id: "UndrivenPowerInput",
            severity: Error,
        },
        RuleSpec {
            id: "DanglingLabel",
            severity: Warning,
        },
        RuleSpec {
            id: "SimilarLabelNames",
            severity: Warning,
        },
        RuleSpec {
            id: "UnconnectedPin",
            severity: Warning,
        },
        RuleSpec {
            id: "UnfulfilledWant",
            severity: Warning,
        },
    ],
    resolve: Some(sch::standard::relations),
    ..Library::new("connectivity")
};
pub static PLACEMENT: Library = Library {
    erc_rules: &[
        RuleSpec {
            id: "UnplacedGate",
            severity: Warning,
        },
        RuleSpec {
            id: "PeripheralConflict",
            severity: Error,
        },
    ],
    erc: Some(sch::standard::placement),
    ..Library::new("placement")
};
pub static ERC: Library = Library::new("erc").with_children(&[
    &BUSES,
    &VOLTAGE,
    &INTENT,
    &ATTRIBUTES,
    &WIRING,
    &SIGNALS,
    &CONNECTIVITY,
    &PLACEMENT,
    &SUPPLY_RATING,
    &SPEC_LIMITS,
]);
pub static MANUFACTURING: Library = Library::new("manufacturing").with_drc(
    &[
        RuleSpec {
            id: "MinSlotWidth",
            severity: Error,
        },
        RuleSpec {
            id: "MinInnerDrillClearance",
            severity: Error,
        },
        RuleSpec {
            id: "MinHoleDrill",
            severity: Error,
        },
        RuleSpec {
            id: "DrillOverlap",
            severity: Error,
        },
        RuleSpec {
            id: "HoleOutsideBoard",
            severity: Error,
        },
        RuleSpec {
            id: "MinTrackWidth",
            severity: Error,
        },
        RuleSpec {
            id: "MinViaDrill",
            severity: Error,
        },
        RuleSpec {
            id: "MinAnnularRing",
            severity: Error,
        },
    ],
    pcb::standard::manufacturing,
);
pub static COPPER: Library = Library::new("copper").with_drc(
    &[
        RuleSpec {
            id: "CourtyardOverlap",
            severity: Error,
        },
        RuleSpec {
            id: "MinEdgeClearance",
            severity: Error,
        },
        RuleSpec {
            id: "ShortCircuit",
            severity: Error,
        },
        RuleSpec {
            id: "UnroutedNet",
            severity: Error,
        },
        RuleSpec {
            id: "CopperInCutout",
            severity: Error,
        },
        RuleSpec {
            id: "CopperAtHole",
            severity: Error,
        },
        RuleSpec {
            id: "CopperInKeepout",
            severity: Error,
        },
        RuleSpec {
            id: "MinMaskWeb",
            severity: Error,
        },
        RuleSpec {
            id: "SilkToMask",
            severity: Error,
        },
        RuleSpec {
            id: "ArtworkOutsideBoard",
            severity: Error,
        },
        RuleSpec {
            id: "CopperOutsideEdge",
            severity: Error,
        },
        RuleSpec {
            id: "MinCopperSpacing",
            severity: Error,
        },
    ],
    pcb::standard::copper,
);
pub static DRC: Library = Library::new("drc").with_children(&[&COPPER, &MANUFACTURING]);
pub static STANDARD: Library = Library::new("standard").with_children(&[&PROPERTIES, &ERC, &DRC]);
pub static SUPPLY_RATING: Library = Library::new("supply_rating").with_erc(
    &[RuleSpec {
        id: "SupplyAboveRating",
        severity: Error,
    }],
    properties::supply,
);
pub(crate) fn default_erc(id: &str) -> &'static str {
    if id == "LibraryCheckFailed" {
        return "error";
    }
    ERC.children
        .iter()
        .flat_map(|lib| {
            lib.erc_rules
                .iter()
                .chain(lib.worker_checks.iter().map(|c| &c.rule))
        })
        .find(|r| r.id == id)
        .map_or("warning", |r| r.severity.word())
}

pub(crate) fn known_erc(id: &str) -> bool {
    ERC.children
        .iter()
        .flat_map(|l| l.erc_rules.iter().chain(l.worker_checks.iter().map(|c| &c.rule)))
        .any(|r| r.id == id)
}
