//! Unit declarations on named attribute types. Factors convert to the first (base) unit.
use super::*;
use crate::electronics::{Params, fault, list, measure, string};
use crate::syntax::{Expr, ExprKind};
use crate::values::Dim;
use forge_agent::json::Json;

#[derive(Clone, Debug, PartialEq)]
pub struct Radix {
    pub base: u32,
    pub prefix: String,
    pub digits: usize,
}
impl Radix {
    fn encode(&self, mut n: u32) -> Option<String> {
        if !(2..=36).contains(&self.base) || self.digits > 32 {
            return None;
        }
        let mut digits = Vec::new();
        loop {
            digits.push(
                char::from_digit(n % self.base, self.base)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            n /= self.base;
            if n == 0 {
                break;
            }
        }
        digits.resize(digits.len().max(self.digits), '0');
        Some(self.prefix.chars().chain(digits.into_iter().rev()).collect())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeUnit {
    pub name: String,
    pub factor: f64,
    pub radix: Option<Radix>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
impl AttributeUnit {
    fn accepts(&self, n: f64) -> bool {
        n.is_finite() && self.min.is_none_or(|v| n >= v) && self.max.is_none_or(|v| n <= v)
    }
    pub fn json(&self) -> Json {
        let mut fields = vec![
            ("name", self.name.clone().into()),
            ("min", self.min.map_or(Json::Null, Into::into)),
            ("max", self.max.map_or(Json::Null, Into::into)),
        ];
        if let Some(r) = &self.radix {
            fields.extend([
                ("radix", (r.base as usize).into()),
                ("prefix", r.prefix.clone().into()),
                ("digits", r.digits.into()),
            ]);
        } else {
            fields.push(("factor", self.factor.into()));
        }
        Json::obj(fields)
    }
}
pub fn validate_units(units: &[AttributeUnit]) -> std::result::Result<(), String> {
    if !units.first().is_some_and(|v| v.factor == 1.) {
        return Err("Declare a base unit first, with a conversion factor of 1.".into());
    }
    let encoded = units[0].radix.is_some();
    let mut prefixes = Set::new();
    for u in units {
        if encoded != u.radix.is_some() {
            return Err("Use either scaled units or integer encodings within a type.".into());
        }
        if let Some(r) = &u.radix {
            if !(2..=36).contains(&r.base)
                || r.digits > 32
                || !r.prefix.is_ascii()
                || r.prefix.chars().any(char::is_whitespace)
                || !prefixes.insert(r.prefix.to_ascii_lowercase())
            {
                return Err("Use bases 2–36, distinct ASCII prefixes and at most 32 padding digits.".into());
            }
            if u.min
                .into_iter()
                .chain(u.max)
                .any(|n| n.fract() != 0. || !(0.0..=u32::MAX as f64).contains(&n))
            {
                return Err("Integer limits must be unsigned 32-bit values.".into());
            }
        }
    }
    for short in units.iter().filter_map(|u| u.radix.as_ref()) {
        let head = short.prefix.to_ascii_lowercase();
        if prefixes.iter().any(|other| {
            other
                .strip_prefix(&head)
                .is_some_and(|tail| !tail.is_empty() && tail.chars().all(|c| c.is_digit(short.base)))
        }) {
            return Err("An encoding prefix is ambiguous with the digits of another format.".into());
        }
    }
    let mut names = Set::new();
    for u in units {
        if u.name.trim().is_empty() || u.name.trim() != u.name || !names.insert(&u.name) {
            return Err("Use a distinct, nonempty name for each unit.".into());
        }
        if !u.factor.is_finite() || u.factor <= 0. {
            return Err(format!("{} needs a positive, finite conversion factor.", u.name));
        }
        if u.min.into_iter().chain(u.max).any(|v| !v.is_finite())
            || u.min.zip(u.max).is_some_and(|(lo, hi)| lo > hi)
        {
            return Err(format!("Check the limits for {}.", u.name));
        }
    }
    Ok(())
}
pub(super) fn read_units(e: &Expr, lookup: bool) -> Result<Vec<AttributeUnit>> {
    let mut units = Vec::new();
    for entry in list(e)? {
        let ExprKind::Map(args) = &entry.kind else {
            return Err(fault(
                entry,
                "InvalidAttributeUnit",
                "use a record with name, factor and optional min and max",
            ));
        };
        let mut fields = Map::new();
        for (name, value) in args {
            if !matches!(
                name.as_str(),
                "name" | "factor" | "min" | "max" | "radix" | "prefix" | "digits"
            ) || fields.insert(name.as_str(), value).is_some()
            {
                return Err(fault(
                    value,
                    "InvalidAttributeUnit",
                    format!("unknown or repeated unit field {name}"),
                ));
            }
        }
        let req = |s| {
            fields
                .get(s)
                .copied()
                .ok_or_else(|| fault(entry, "InvalidAttributeUnit", format!("missing unit {s}")))
        };
        let scalar = |v: &Expr| measure(v, &Params::new(), Dim::Ratio);
        let radix = if let Some(base) = fields.get("radix") {
            if fields.contains_key("factor") {
                return Err(fault(
                    entry,
                    "InvalidAttributeUnit",
                    "choose a radix or a scale factor",
                ));
            }
            let natural = |v: &Expr| {
                let n = scalar(v)?;
                if !(0.0..=36.).contains(&n) || n.fract() != 0. {
                    Err(fault(
                        v,
                        "InvalidAttributeUnit",
                        "expected a small nonnegative integer",
                    ))
                } else {
                    Ok(n as u32)
                }
            };
            Some(Radix {
                base: natural(base)?,
                prefix: fields
                    .get("prefix")
                    .map(|v| string(v))
                    .transpose()?
                    .unwrap_or_default(),
                digits: fields.get("digits").map(|v| natural(v)).transpose()?.unwrap_or(0) as usize,
            })
        } else {
            if fields.contains_key("prefix") || fields.contains_key("digits") {
                return Err(fault(
                    entry,
                    "InvalidAttributeUnit",
                    "declare the radix for this encoding",
                ));
            }
            None
        };
        units.push(AttributeUnit {
            name: string(req("name")?)?,
            factor: if radix.is_some() || (lookup && !fields.contains_key("factor")) {
                1.
            } else {
                scalar(req("factor")?)?
            },
            radix,
            min: fields.get("min").map(|v| scalar(v)).transpose()?,
            max: fields.get("max").map(|v| scalar(v)).transpose()?,
        });
    }
    validate_units(&units).map_err(|msg| fault(e, "InvalidAttributeUnit", msg))?;
    Ok(units)
}
impl StrapKind {
    pub fn standard_units(self) -> &'static [AttributeUnit] {
        standard_attribute_types()
            .values()
            .find(|t| t.kind == self)
            .map_or(&[], |t| &t.units)
    }
    fn quantity(self, value: &str, units: &[AttributeUnit], selected: Option<&str>) -> Option<f64> {
        let value = value.trim();
        let alias;
        let value = if self == Self::Current && (value.ends_with("uA") || value.ends_with("μA")) {
            alias = format!(
                "{}µA",
                value.strip_suffix("uA").or_else(|| value.strip_suffix("μA"))?
            );
            &alias
        } else {
            value
        };
        let (n, u) = if let Ok(n) = value.parse::<f64>() {
            (n, units.iter().find(|u| Some(u.name.as_str()) == selected)?)
        } else {
            let u = units
                .iter()
                .filter(|u| value.ends_with(&u.name))
                .max_by_key(|u| u.name.len())?;
            (value.strip_suffix(&u.name)?.trim().parse().ok()?, u)
        };
        let base = n * u.factor;
        (u.accepts(n) && units.first()?.accepts(base)).then_some(base)
    }
    pub fn typed_with_units(
        self,
        value: &str,
        units: &[AttributeUnit],
        selected: Option<&str>,
    ) -> Option<Json> {
        if matches!(self, Self::Quantity | Self::Current) {
            let base = self.quantity(value, units, selected)?;
            return Some(Json::obj(vec![
                ("value", base.into()),
                ("unit", units.first()?.name.clone().into()),
            ]));
        }
        if matches!(self, Self::Address7 | Self::Integer) {
            let raw = value.trim();
            let prefixed = units
                .iter()
                .filter_map(|unit| {
                    let r = unit.radix.as_ref()?;
                    (!r.prefix.is_empty()
                        && raw
                            .get(..r.prefix.len())
                            .is_some_and(|s| s.eq_ignore_ascii_case(&r.prefix)))
                    .then_some((unit, r))
                })
                .max_by_key(|(_, r)| r.prefix.len());
            let (unit, digits) = if let Some((u, r)) = prefixed {
                (u, &raw[r.prefix.len()..])
            } else {
                let u = selected
                    .and_then(|name| units.iter().find(|u| u.name == name))
                    .or_else(|| {
                        units
                            .iter()
                            .find(|u| u.radix.as_ref().is_some_and(|r| r.prefix.is_empty()))
                    })
                    .or_else(|| units.first())?;
                (u, raw)
            };
            let base = unit.radix.as_ref()?.base;
            if !(2..=36).contains(&base) || digits.is_empty() || !digits.chars().all(|c| c.is_digit(base)) {
                return None;
            }
            let n = u32::from_str_radix(digits, base).ok()?;
            return ((self != Self::Address7 || n < 128)
                && unit.accepts(n as f64)
                && units.first()?.accepts(n as f64))
            .then(|| (n as usize).into());
        }
        self.typed(value)
    }
    pub fn format_unit(
        self,
        value: &str,
        units: &[AttributeUnit],
        from: Option<&str>,
        to: &str,
    ) -> Option<String> {
        let target = units.iter().find(|u| u.name == to)?;
        if matches!(self, Self::Quantity | Self::Current) {
            let n = self.quantity(value, units, from)? / target.factor;
            return target.accepts(n).then(|| format!("{n} {}", target.name));
        }
        if let Some(r) = &target.radix {
            let n = self.typed_with_units(value, units, from)?.num()?;
            return target.accepts(n).then(|| r.encode(n as u32)).flatten();
        }
        self.typed(value).map(|_| value.to_string())
    }
}
