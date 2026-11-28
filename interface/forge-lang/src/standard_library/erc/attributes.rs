//! Standard rules for fitted attributes and ordered I²C bus identities.
use super::*;
impl Circuit {
    fn address_bus(&self, owner: &str, symbol: &Symbol, table: &StrapTable) -> Option<(String, String)> {
        let net_for = |lane: &str, explicit: Option<&String>| {
            let mut pins = symbol.pins.keys().filter_map(|pin| {
                if explicit.is_some_and(|p| p != pin) {
                    return None;
                }
                let id = format!("{owner}.{pin}");
                let t = self.terminals.get(&id)?;
                let active = match t.function.as_deref() {
                    Some(f) => {
                        f.eq_ignore_ascii_case(lane)
                            || f.rsplit_once('_')
                                .is_some_and(|(_, s)| s.eq_ignore_ascii_case(lane))
                    }
                    None => explicit.is_some() || pin.eq_ignore_ascii_case(lane),
                };
                active.then(|| self.net_of[&id].clone())
            });
            let net = pins.next()?;
            pins.next().is_none().then_some(net)
        };
        let sda = net_for("SDA", table.bus.as_ref().map(|p| &p[0]))?;
        let scl = net_for("SCL", table.bus.as_ref().map(|p| &p[1]))?;
        (sda != scl).then_some((sda, scl))
    }
    pub(super) fn check_attribute(
        &self,
        owner: &str,
        symbol: &Symbol,
        name: &str,
        table: &StrapTable,
        result: &StrapResult,
        checks: &mut AttributeChecks,
    ) {
        let status = result.status;
        let value = result.value.as_deref();
        let object = format!("{owner}:{name}");
        let bus = if table.kind == StrapKind::Address7 {
            self.address_bus(owner, symbol, table)
        } else {
            None
        };
        use crate::electronics_library::evidence::missing;
        match table.kind {
            StrapKind::Address7 => {
                if bus.is_none() {
                    missing("I2cAddressConflict", &object, "resolved SDA and SCL nets");
                }
                if value.is_none() {
                    for rule in ["ReservedI2cAddress", "I2cAddressConflict"] {
                        missing(rule, &object, "resolved 7-bit address");
                    }
                }
            }
            StrapKind::Current => {
                if table.range.is_none() {
                    missing("CurrentOutsideRange", &object, "allowed current range");
                }
                if value.is_none() {
                    missing("CurrentOutsideRange", &object, "resolved current");
                }
            }
            StrapKind::Choice | StrapKind::BootMode => {
                let rule = if table.kind == StrapKind::Choice {
                    "UnsupportedAttributeValue"
                } else {
                    "UnsupportedBootMode"
                };
                if table.modes.is_empty() {
                    missing(rule, &object, "allowed modes");
                }
                if value.is_none() {
                    missing(rule, &object, "resolved mode");
                }
            }
            _ => {}
        }
        let mut finding = |kind, unknown, message: String| {
            checks.issues.push(Issue {
                kind,
                unknown,
                objects: vec![object.clone()],
                message,
            })
        };
        if let Some(v) = value {
            match table.kind {
                StrapKind::Address7 => {
                    let address = table
                        .kind
                        .typed_with_units(v, &table.units, table.unit.as_deref())
                        .unwrap()
                        .num()
                        .unwrap() as u8;
                    if !(0x08..=0x77).contains(&address) {
                        finding(
                            "ReservedI2cAddress",
                            false,
                            format!("{object} uses reserved seven-bit address 0x{address:02X}"),
                        );
                    }
                    let nets = bus;
                    if let Some((sda, scl)) = nets {
                        checks
                            .addresses
                            .push(crate::electronics_library::worker::Fact::Address {
                                owner: owner.into(),
                                attribute: object.clone(),
                                sda,
                                scl,
                                address,
                            });
                    } else if !symbol.pins.is_empty() {
                        finding(
                            "UnknownI2cBus",
                            true,
                            format!("Select the SDA and SCL pins for {object} to check address collisions"),
                        );
                    }
                }
                StrapKind::Current => {
                    if let Some([lo, hi]) = &table.range
                        && !within_current(table, v, lo, hi)
                    {
                        finding(
                            "CurrentOutsideRange",
                            false,
                            format!("{object} selects {v}, outside {lo} to {hi}"),
                        );
                    }
                }
                StrapKind::BootMode | StrapKind::Choice => {
                    if !table.modes.is_empty() && !table.modes.iter().any(|m| m == v) {
                        finding(
                            if table.kind == StrapKind::BootMode {
                                "UnsupportedBootMode"
                            } else {
                                "UnsupportedAttributeValue"
                            },
                            false,
                            format!(
                                "{object} selects {v}. Allowed modes are {}",
                                table.modes.join(", ")
                            ),
                        );
                    }
                }
                StrapKind::Text | StrapKind::Quantity | StrapKind::Integer | StrapKind::Lookup => {}
            }
        } else {
            finding(
                "UnresolvedAttribute",
                status == "unresolved",
                format!("{object} has {status} input states"),
            );
        }
    }
}
