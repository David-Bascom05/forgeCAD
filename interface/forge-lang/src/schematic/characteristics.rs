//! Repeatable, conditional manufacturer ratings. Missing entries convey no guarantee.
pub mod fet;

use super::*;
use crate::electronics::{Params, fault, list, measure, reference, string};
use crate::syntax::{Expr, ExprKind};
use crate::values::Dim;
use forge_agent::json::Json;

/// Kind families share dimensions. Each numeric record keeps the datasheet's display unit.
pub const FAMILIES: &[(&[&str], &[&str])] = &[
    (
        &[
            "pin_voltage",
            "supply_voltage",
            "input_voltage",
            "output_voltage",
            "logic_high_threshold",
            "logic_low_threshold",
            "output_high_voltage",
            "output_low_voltage",
            "gate_source_voltage",
            "drain_source_voltage",
            "collector_emitter_voltage",
            "forward_voltage",
            "breakdown_voltage",
            "clamp_voltage",
            "dropout_voltage",
            "differential_voltage",
            "common_mode_voltage",
            "offset_voltage",
            "working_voltage",
            "withstand_voltage",
            "surge_voltage",
            "reverse_test_voltage",
        ],
        &["V", "mV", "µV", "kV"],
    ),
    (
        &[
            "load_current",
            "pullup_load_current",
            "source_current",
            "sink_current",
            "injection_current",
            "supply_current",
            "aggregate_current",
            "leakage_current",
            "drain_current",
            "collector_current",
            "forward_current",
            "reverse_current",
            "trip_current",
            "rms_current",
            "saturation_current",
            "bias_current",
            "inrush_current",
        ],
        &["A", "mA", "µA", "nA", "pA"],
    ),
    (
        &[
            "resistance",
            "on_resistance",
            "pull_up_resistance",
            "pull_down_resistance",
            "contact_resistance",
            "winding_resistance",
            "series_resistance",
            "input_resistance",
            "output_resistance",
            "impedance",
        ],
        &["Ω", "mΩ", "kΩ", "MΩ", "GΩ", "TΩ"],
    ),
    (
        &[
            "input_capacitance",
            "output_capacitance",
            "gate_capacitance",
            "junction_capacitance",
            "capacitance",
        ],
        &["F", "mF", "µF", "nF", "pF"],
    ),
    (&["inductance"], &["H", "mH", "µH", "nH"]),
    (
        &["gate_charge", "recovery_charge", "capacitive_charge"],
        &["C", "µC", "nC", "pC"],
    ),
    (
        &[
            "propagation_delay",
            "rise_time",
            "fall_time",
            "recovery_time",
            "pulse_duration",
            "trip_time",
            "setup_time",
            "hold_time",
        ],
        &["s", "ms", "µs", "ns", "ps"],
    ),
    (
        &["bandwidth", "frequency", "switching_frequency"],
        &["Hz", "kHz", "MHz", "GHz"],
    ),
    (&["transconductance", "conductance"], &["S", "mS", "µS"]),
    (&["energy", "avalanche_energy"], &["J", "mJ", "µJ"]),
    (&["slew_rate"], &["V/s", "V/ms", "V/µs", "V/ns"]),
    (
        &["gain", "common_mode_rejection", "power_supply_rejection"],
        &["dB"],
    ),
    (&["clearance", "creepage"], &["mm", "µm", "m"]),
    (&["copper_area"], &["mm²", "cm²", "m²"]),
    (
        &["s11_phase", "s12_phase", "s21_phase", "s22_phase"],
        &["deg", "rad"],
    ),
    (&["power_dissipation"], &["W", "mW", "µW"]),
    (
        &[
            "rf_input_power",
            "rf_output_power",
            "input_intercept_power",
            "output_intercept_power",
            "compression_output_power",
        ],
        &["W", "mW", "µW", "dBm"],
    ),
    (
        &["junction_temperature", "ambient_temperature", "temperature_rise"],
        &["°C"],
    ),
    (&["thermal_resistance"], &["°C/W", "K/W", "mK/W", "K/kW"]),
    (
        &["drain_current_slew_rate", "commutation_current_slew_rate"],
        &["A/s", "A/ms", "A/µs", "A/ns"],
    ),
    (
        &["threshold_temperature_coefficient"],
        &["V/°C", "mV/°C", "µV/°C"],
    ),
    (&["on_resistance_temperature_coefficient"], &["%/°C", "ppm/°C"]),
    (&["power_derating_rate"], &["W/°C", "mW/°C"]),
    (&["voltage_noise_density"], &["V/√Hz", "µV/√Hz", "nV/√Hz"]),
    (&["current_noise_density"], &["A/√Hz", "pA/√Hz", "fA/√Hz"]),
    (
        &[
            "capacitance_retained",
            "load_fraction",
            "inductance_loss",
            "duty_cycle",
            "tolerance",
        ],
        &["%"],
    ),
    (&["loaded_contacts", "resolution", "dc_current_gain"], &["1"]),
];
pub const STATES: &[(&str, &[&str])] = &[
    (
        "pull_up",
        &["present", "absent", "configurable", "enabled", "disabled"],
    ),
    (
        "pull_down",
        &["present", "absent", "configurable", "enabled", "disabled"],
    ),
    (
        "protection",
        &[
            "absent",
            "rail_clamp",
            "esd_diode",
            "tvs",
            "current_limit",
            "thermal_shutdown",
            "reverse_polarity",
        ],
    ),
    (
        "device_family",
        &[
            "n_channel_mosfet",
            "p_channel_mosfet",
            "n_channel_jfet",
            "p_channel_jfet",
            "npn",
            "pnp",
            "schottky",
            "zener",
            "tvs",
            "igbt",
        ],
    ),
    (
        "terminal_role",
        &[
            "gate",
            "source",
            "drain",
            "body",
            "kelvin_source",
            "anode",
            "cathode",
            "base",
            "collector",
            "emitter",
            "contact",
            "coil",
        ],
    ),
    (
        "driver_topology",
        &[
            "push_pull",
            "open_drain",
            "open_source",
            "tri_state",
            "high_impedance",
        ],
    ),
    (
        "fault_response",
        &["retry", "latch_off", "current_limit", "shutdown"],
    ),
    ("power_state", &["powered", "unpowered", "standby"]),
    ("signal_domain", &["digital", "analog", "power"]),
    ("channel_polarity", &["n_channel", "p_channel"]),
    ("channel_mode", &["enhancement", "depletion"]),
    (
        "semiconductor_material",
        &[
            "silicon",
            "silicon_carbide",
            "gallium_nitride",
            "gallium_arsenide",
            "other",
        ],
    ),
    (
        "fet_structure",
        &[
            "planar",
            "trench",
            "superjunction",
            "hemt",
            "cascode",
            "finfet",
            "other",
        ],
    ),
    (
        "gate_protection",
        &[
            "absent",
            "esd_diode",
            "zener_clamp",
            "bidirectional_clamp",
            "other",
        ],
    ),
    (
        "reverse_conduction_path",
        &[
            "body_diode",
            "channel",
            "co_packaged_diode",
            "external_diode",
            "other",
        ],
    ),
    ("pulse_regime", &["dc", "single_pulse", "repetitive"]),
    ("test_fixture", &[]),
    (
        "switching_regime",
        &["hard_switching", "zero_voltage", "zero_current", "soft_switching"],
    ),
    (
        "measurement_configuration",
        &[
            "common_source",
            "common_drain",
            "common_gate",
            "differential_pair",
            "other",
        ],
    ),
    ("package_variant", &[]),
    ("commutating_device", &[]),
    (
        "charge_measurement_method",
        &[
            "integrated_capacitance",
            "integrated_current",
            "double_pulse",
            "other",
        ],
    ),
    ("mode", &[]),
    ("variant", &[]),
    ("test_standard", &[]),
    ("power_domain", &[]),
];
pub const CURVE_SCALES: &[&str] = &["linear", "log_x", "log_y", "log_log"];
pub const CURVE_BASES: &[&str] = &[
    "datasheet_table",
    "typical_curve",
    "specified_limit",
    "bench_measurement",
    "simulation",
];
pub const CLASSES: &[&str] = &["characteristic", "operating", "absolute_max", "test_rating"];
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Reading {
    pub kind: String,
    pub pins: Vec<String>,
    pub unit: String,
    pub min: Option<f64>,
    pub typ: Option<f64>,
    pub max: Option<f64>,
    pub state: String,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Specification {
    pub value: Reading,
    pub class: String,
    /// A named trace uses the first condition as its swept coordinate.
    pub curve: String,
    pub curve_scale: String,
    pub curve_basis: String,
    pub conditions: Vec<Reading>,
    pub source: String,
    pub note: String,
}
pub fn units(kind: &str) -> Option<&'static [&'static str]> {
    FAMILIES
        .iter()
        .find(|(ks, _)| ks.contains(&kind))
        .map(|(_, us)| *us)
        .or_else(|| {
            let unit = fet::property(kind)?.unit;
            FAMILIES
                .iter()
                .find(|(_, us)| us.contains(&unit))
                .map(|(_, us)| *us)
        })
}
pub fn states(kind: &str) -> Option<&'static [&'static str]> {
    STATES.iter().find(|(k, _)| *k == kind).map(|(_, vs)| *vs)
}
type Fields<'a> = Map<String, &'a Expr>;
fn fields(e: &Expr) -> Result<Fields<'_>> {
    let ExprKind::Map(entries) = &e.kind else {
        return Err(fault(e, "InvalidSpecification", "expected a datasheet record"));
    };
    let mut out = Map::new();
    for (key, v) in entries {
        if out.insert(key.clone(), v).is_some() {
            return Err(fault(v, "DuplicateSpecificationField", key));
        }
    }
    Ok(out)
}
fn text(f: &mut Fields<'_>, k: &str) -> Result<String> {
    f.remove(k).map(string).transpose().map(Option::unwrap_or_default)
}
fn finish(e: &Expr, f: Fields<'_>) -> Result<()> {
    if let Some((k, _)) = f.first_key_value() {
        return Err(fault(e, "UnknownSpecificationField", k));
    }
    Ok(())
}
fn reading(e: &Expr, f: &mut Fields<'_>, p: &Params) -> Result<Reading> {
    let kind = text(f, "kind")?;
    let mut unit = text(f, "unit")?;
    let state = text(f, "state")?;
    let pins = f
        .remove("pins")
        .map(|v| list(v)?.iter().map(reference).collect::<Result<Vec<_>>>())
        .transpose()?
        .unwrap_or_default();
    let mut bound = |k: &str| f.remove(k).map(|v| measure(v, p, Dim::Any)).transpose();
    let (min, typ, max) = (bound("min")?, bound("typ")?, bound("max")?);
    let numbers: Vec<_> = [min, typ, max].into_iter().flatten().collect();
    let bad = |s: &str| fault(e, "InvalidSpecification", format!("{kind}: {s}"));
    let custom = kind
        .strip_prefix("custom.")
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
    if let Some(us) = units(&kind) {
        if unit.is_empty() {
            unit = us[0].into();
        }
        if !us.contains(&unit.as_str()) {
            return Err(bad("incompatible unit"));
        }
        if numbers.is_empty() || !state.is_empty() {
            return Err(bad("enter a minimum, typical or maximum value"));
        }
    } else if let Some(vs) = states(&kind) {
        if state.trim().is_empty()
            || (!vs.is_empty() && !vs.contains(&state.as_str()))
            || !numbers.is_empty()
            || !unit.is_empty()
        {
            return Err(bad("invalid state or unexpected numeric rating"));
        }
    } else if !custom
        || !(numbers.is_empty() && !state.trim().is_empty() && unit.is_empty()
            || !numbers.is_empty() && state.is_empty() && !unit.trim().is_empty())
    {
        return Err(bad(
            "choose a known quantity or declare custom.<name> with a unit and values, or a state",
        ));
    }
    if kind == "capacitance_retained" && numbers.iter().any(|n| !(0.0..=100.0).contains(n)) {
        return Err(bad("retained capacitance must be between 0 and 100 percent"));
    }
    if matches!(kind.as_str(), "resistance" | "capacitance") && numbers.iter().any(|n| *n < 0.) {
        return Err(bad("expected a nonnegative nominal value"));
    }
    if numbers.windows(2).any(|x| x[0] > x[1]) {
        return Err(bad("expected min ≤ typical ≤ max"));
    }
    let result = Reading {
        kind,
        pins,
        unit,
        min,
        typ,
        max,
        state,
    };
    if units(&result.kind).is_some_and(|us| us.contains(&"dBm"))
        && numbers.iter().any(|n| {
            let watts = result.canonical_value(*n);
            !watts.is_finite() || watts < 0. || result.unit == "dBm" && watts == 0.
        })
    {
        return Err(fault(
            e,
            "InvalidSpecification",
            "RF power must resolve to a finite nonnegative wattage",
        ));
    }
    Ok(result)
}
pub(super) fn parse(e: Option<&Expr>, p: &Params) -> Result<Vec<Specification>> {
    let Some(e) = e else { return Ok(vec![]) };
    let records: Vec<Specification> = list(e)?
        .iter()
        .map(|e| {
            let mut f = fields(e)?;
            let class = f
                .remove("class")
                .map(string)
                .transpose()?
                .unwrap_or_else(|| "characteristic".into());
            if !CLASSES.contains(&class.as_str()) {
                return Err(fault(e, "InvalidRatingClass", &class));
            }
            let source = text(&mut f, "source")?;
            let note = text(&mut f, "note")?;
            let curve = text(&mut f, "curve")?;
            let curve_scale = text(&mut f, "curve_scale")?;
            let curve_basis = text(&mut f, "curve_basis")?;
            for (setting, choices) in [(&curve_scale, CURVE_SCALES), (&curve_basis, CURVE_BASES)] {
                if !setting.is_empty() && (curve.is_empty() || !choices.contains(&setting.as_str())) {
                    return Err(fault(
                        e,
                        "InvalidDatasheetCurve",
                        format!("invalid curve setting {setting}"),
                    ));
                }
            }
            let at = f.remove("coordinate");
            let mut conditions = f
                .remove("when")
                .map(|v| {
                    list(v)?
                        .iter()
                        .map(|e| {
                            let mut fs = fields(e)?;
                            let r = reading(e, &mut fs, p)?;
                            finish(e, fs)?;
                            Ok(r)
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();
            if let Some(at) = at {
                let mut fs = fields(at)?;
                let axis = reading(at, &mut fs, p)?;
                finish(at, fs)?;
                if curve.trim().is_empty() || axis.typ.is_none() || axis.min.is_some() || axis.max.is_some() {
                    return Err(fault(
                        at,
                        "InvalidDatasheetCurve",
                        "a coordinate needs a curve name and one numeric typical value",
                    ));
                }
                conditions.insert(0, axis);
            } else if !curve.is_empty() {
                return Err(fault(
                    e,
                    "InvalidDatasheetCurve",
                    "a named curve requires a coordinate",
                ));
            }
            let value = reading(e, &mut f, p)?;
            if !curve.is_empty() && !value.state.is_empty() {
                return Err(fault(
                    e,
                    "InvalidDatasheetCurve",
                    "a curve needs a numeric measurement",
                ));
            }
            let log_x = matches!(curve_scale.as_str(), "log_x" | "log_log");
            let log_y = matches!(curve_scale.as_str(), "log_y" | "log_log");
            if log_x && conditions[0].typ.is_some_and(|n| n <= 0.)
                || log_y
                    && [value.min, value.typ, value.max]
                        .into_iter()
                        .flatten()
                        .any(|n| n <= 0.)
            {
                return Err(fault(
                    e,
                    "InvalidDatasheetCurve",
                    "logarithmic axes require positive displayed values",
                ));
            }
            finish(e, f)?;
            Ok(Specification {
                value,
                curve,
                curve_scale,
                curve_basis,
                class,
                source,
                note,
                conditions,
            })
        })
        .collect::<Result<_>>()?;
    let mut series = Map::new();
    for point in records.iter().filter(|p| !p.curve.is_empty()) {
        let (first, xs) = series.entry(&point.curve).or_insert_with(|| (point, Set::new()));
        if !first.compatible_curve_point(point) {
            return Err(fault(
                e,
                "InvalidDatasheetCurve",
                format!(
                    "{}: each trace needs common axes, units, terminals, rating class and fixed test conditions",
                    point.curve
                ),
            ));
        }
        let x = point.conditions[0].typ.unwrap();
        if !xs.insert(if x == 0. { 0 } else { x.to_bits() }) {
            return Err(fault(
                e,
                "InvalidDatasheetCurve",
                format!("{}: repeated axis value {x}", point.curve),
            ));
        }
    }
    Ok(records)
}
impl Reading {
    pub fn describe(&self) -> String {
        let mut parts = vec![self.kind.replace('_', " ")];
        if !self.pins.is_empty() {
            parts.push(self.pins.join(" → "));
        }
        if !self.state.is_empty() {
            parts.push(self.state.replace('_', " "));
        }
        for (key, value) in [("min", self.min), ("typ", self.typ), ("max", self.max)] {
            if let Some(n) = value {
                parts.push(format!("{key} {n} {}", self.unit));
            }
        }
        parts.join(" · ")
    }
    /// Compare like quantities in the catalog's first unit. RF power uses watts.
    pub fn canonical_value(&self, value: f64) -> f64 {
        if self.unit == "dBm" && units(&self.kind).is_some_and(|us| us.contains(&"dBm")) {
            forge_math::dmath::powf(10., value / 10. - 3.)
        } else {
            value * self.unit_scale()
        }
    }

    /// Convert all populated bounds atomically. Zero watts has no finite dBm value.
    pub fn converted(&self, unit: &str) -> Option<Self> {
        if !units(&self.kind)?.contains(&unit) {
            return None;
        }
        let mut result = self.clone();
        result.unit = unit.into();
        let scale = result.unit_scale();
        for bound in [&mut result.min, &mut result.typ, &mut result.max]
            .into_iter()
            .flatten()
        {
            let n = self.canonical_value(*bound);
            *bound = if unit == "dBm" {
                10. * ((forge_math::dmath::ln(n) / std::f64::consts::LN_10) + 3.)
            } else {
                n / scale
            };
            if !bound.is_finite() {
                return None;
            }
        }
        Some(result)
    }
    /// Linear multiplier into this kind's first catalog unit; custom quantities keep their declared unit.
    pub fn unit_scale(&self) -> f64 {
        let u = self.unit.as_str();
        if units(&self.kind).is_none_or(|us| us[0] == u) {
            return 1.;
        }
        if matches!(self.kind.as_str(), "creepage" | "clearance") {
            return match u {
                "m" => 1000.,
                "µm" => 0.001,
                _ => 1.,
            };
        }
        match u {
            "V/ms" | "A/ms" => 1e3,
            "V/µs" | "A/µs" => 1e6,
            "V/ns" | "A/ns" => 1e9,
            "ppm/°C" => 1e-4,
            "rad" => 180. / std::f64::consts::PI,
            "cm²" => 100.,
            "m²" => 1e6,
            "K/kW" => 1e-3,
            _ => match u.chars().next() {
                Some('f') => 1e-15,
                Some('p') => 1e-12,
                Some('n') => 1e-9,
                Some('µ') => 1e-6,
                Some('m') => 1e-3,
                Some('k') => 1e3,
                Some('M') => 1e6,
                Some('G') => 1e9,
                Some('T') => 1e12,
                _ => 1.,
            },
        }
    }
    fn members(&self) -> Vec<(&'static str, Json)> {
        let mut f = vec![("kind", self.kind.clone().into())];
        if !self.unit.is_empty() {
            f.push(("unit", self.unit.clone().into()));
        }
        if !self.state.is_empty() {
            f.push(("state", self.state.clone().into()));
        }
        for (k, x) in [("min", self.min), ("typ", self.typ), ("max", self.max)] {
            if let Some(x) = x {
                f.push((k, Json::Num(x)));
            }
        }
        if !self.pins.is_empty() {
            f.push((
                "pins",
                Json::Arr(self.pins.iter().cloned().map(Into::into).collect()),
            ));
        }
        f
    }
}
impl Specification {
    fn compatible_curve_point(&self, other: &Self) -> bool {
        let a = &self.conditions[0];
        let b = &other.conditions[0];
        self.class == other.class
            && self.curve_scale == other.curve_scale
            && self.curve_basis == other.curve_basis
            && self.value.kind == other.value.kind
            && self.value.unit == other.value.unit
            && self.value.pins == other.value.pins
            && a.kind == b.kind
            && a.unit == b.unit
            && a.pins == b.pins
            && self.conditions[1..] == other.conditions[1..]
    }
    pub fn describe(&self) -> String {
        let mut text = format!("{} ({})", self.value.describe(), self.class.replace('_', " "));
        if !self.conditions.is_empty() {
            text.push_str(" when ");
            text.push_str(
                &self
                    .conditions
                    .iter()
                    .map(Reading::describe)
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        text
    }
    /// Keep electrical magnitudes lossless across the human-rounded report serializer.
    pub fn json(&self) -> Json {
        fn exact(v: Json) -> Json {
            match v {
                Json::Num(n) => n.to_string().into(),
                Json::Arr(a) => Json::Arr(a.into_iter().map(exact).collect()),
                Json::Obj(fs) => Json::Obj(fs.into_iter().map(|(k, v)| (k, exact(v))).collect()),
                other => other,
            }
        }
        exact(self.record(true))
    }
    fn record(&self, report: bool) -> Json {
        let mut f = self.value.members();
        f.push(("class", self.class.clone().into()));
        if !self.source.is_empty() {
            f.push(("source", self.source.clone().into()));
        }
        if !self.note.is_empty() {
            f.push(("note", self.note.clone().into()));
        }
        let mut conditions = self.conditions.as_slice();
        if !self.curve.is_empty()
            && let Some((axis, fixed)) = conditions.split_first()
        {
            f.push(("curve", self.curve.clone().into()));
            f.push(("coordinate", Json::obj(axis.members())));
            if !self.curve_scale.is_empty() {
                f.push(("curve_scale", self.curve_scale.clone().into()));
            }
            if !self.curve_basis.is_empty() {
                f.push(("curve_basis", self.curve_basis.clone().into()));
            }
            // Rules consuming the report still see the axis among all conditions.
            if !report {
                conditions = fixed;
            }
        }
        if !conditions.is_empty() {
            f.push((
                "when",
                Json::Arr(conditions.iter().map(|v| Json::obj(v.members())).collect()),
            ));
        }
        Json::obj(f)
    }
}
pub fn source(specs: &[Specification]) -> String {
    fn fg(j: &Json, key: &str) -> String {
        match j {
            Json::Obj(f) => format!(
                "{{{}}}",
                f.iter()
                    .map(|(k, v)| format!("{k}: {}", fg(v, k)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Json::Arr(a) => format!(
                "[{}]",
                a.iter().map(|v| fg(v, key)).collect::<Vec<_>>().join(", ")
            ),
            Json::Str(s) if key == "pins" => s.clone(),
            Json::Num(n) => n.to_string(),
            _ => j.to_string(),
        }
    }
    fg(&Json::Arr(specs.iter().map(|s| s.record(false)).collect()), "")
}
