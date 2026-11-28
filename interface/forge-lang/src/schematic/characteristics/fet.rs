//! FET datasheet quantities and their conventional notation. The unit is a display
//! preference; parsing accepts every compatible unit. Terminal order uses roles.
#[derive(Clone, Copy, Debug)]
pub struct Property {
    pub kind: &'static str,
    pub notation: &'static str,
    pub unit: &'static str,
    pub terminals: Option<[&'static str; 2]>,
}
impl Property {
    pub fn label(&self) -> String {
        let name = self.kind.replace('_', " ");
        if self.notation.is_empty() {
            name
        } else {
            format!("{name} ({})", self.notation)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Family {
    All,
    Insulated,
    Junction,
}

pub struct Group {
    pub title: &'static str,
    pub family: Family,
    pub properties: &'static [Property],
}

pub fn is_fet(part: &str) -> bool {
    matches!(part, "fet" | "jfet" | "mosfet")
}
pub fn groups(part: &str) -> impl Iterator<Item = &'static Group> + '_ {
    GROUPS.iter().filter(move |g| match part {
        "fet" => true,
        "jfet" => g.family != Family::Insulated,
        "mosfet" => g.family != Family::Junction,
        _ => false,
    })
}
pub fn property(kind: &str) -> Option<&'static Property> {
    GROUPS.iter().flat_map(|g| g.properties).find(|p| p.kind == kind)
}

const DS: Option<[&str; 2]> = Some(["drain", "source"]);
const GS: Option<[&str; 2]> = Some(["gate", "source"]);
const GD: Option<[&str; 2]> = Some(["gate", "drain"]);
const SD: Option<[&str; 2]> = Some(["source", "drain"]);
const fn p(
    kind: &'static str,
    notation: &'static str,
    unit: &'static str,
    terminals: Option<[&'static str; 2]>,
) -> Property {
    Property {
        kind,
        notation,
        unit,
        terminals,
    }
}
pub const GROUPS: &[Group] = &[
    Group {
        title: "Device and construction",
        family: Family::All,
        properties: &[
            p("device_family", "", "", None),
            p("channel_polarity", "", "", None),
            p("channel_mode", "", "", None),
            p("semiconductor_material", "", "", None),
            p("fet_structure", "", "", None),
            p("gate_protection", "", "", None),
            p("reverse_conduction_path", "", "", None),
        ],
    },
    Group {
        title: "Terminal ratings and DC characteristics",
        family: Family::All,
        properties: &[
            p("drain_source_voltage", "VDS", "V", DS),
            p("gate_source_voltage", "VGS", "V", GS),
            p("gate_drain_voltage", "VGD", "V", GD),
            p("drain_source_breakdown_voltage", "BVDSS", "V", DS),
            p("gate_source_breakdown_voltage", "BV(GS)", "V", GS),
            p("gate_drain_breakdown_voltage", "BV(GD)", "V", GD),
            p("drain_current", "ID", "A", DS),
            p("pulsed_drain_current", "IDM", "A", DS),
            p("package_limited_drain_current", "ID(pkg)", "A", DS),
            p("silicon_limited_drain_current", "ID(chip)", "A", DS),
            p("bulk_source_voltage", "VBS", "V", Some(["body", "source"])),
            p("bulk_drain_voltage", "VBD", "V", Some(["body", "drain"])),
            p(
                "gate_kelvin_source_voltage",
                "VGKS",
                "V",
                Some(["gate", "kelvin_source"]),
            ),
            p("gate_current", "IG", "mA", None),
            p("pulsed_gate_current", "IGM", "mA", None),
            p("on_resistance", "RDS(on)", "mΩ", DS),
            p("esd_human_body_model_voltage", "HBM", "kV", None),
            p("esd_charged_device_model_voltage", "CDM", "V", None),
            p("normalized_on_resistance", "RDS(on)/RDS(on),ref", "1", DS),
            p("on_resistance_temperature_coefficient", "αRDS", "%/°C", DS),
            p("drain_source_on_voltage", "VDS(on)", "V", DS),
            p("gate_forward_leakage_current", "IGSS(+)", "nA", GS),
            p("gate_reverse_leakage_current", "IGSS(−)", "nA", GS),
            p("transconductance", "gfs / yfs", "S", None),
            p("output_conductance", "gos / yos", "µS", None),
            p("output_resistance", "ro", "kΩ", None),
        ],
    },
    Group {
        title: "Insulated-gate characteristics",
        family: Family::Insulated,
        properties: &[
            p("gate_threshold_voltage", "VGS(th)", "V", GS),
            p("threshold_temperature_coefficient", "dVth/dT", "mV/°C", GS),
            p("off_state_drain_current", "IDSS", "µA", DS),
            p("dynamic_on_resistance", "dynamic RDS(on)", "mΩ", DS),
            p("gate_plateau_voltage", "Vplateau", "V", GS),
            p("gate_drive_on_voltage", "VGS(on)", "V", GS),
            p("gate_drive_off_voltage", "VGS(off drive)", "V", GS),
            p("gate_transient_voltage", "VGS(transient)", "V", GS),
        ],
    },
    Group {
        title: "Junction-gate characteristics",
        family: Family::Junction,
        properties: &[
            p("gate_source_cutoff_voltage", "VGS(off)", "V", GS),
            p("zero_gate_drain_current", "IDSS", "mA", DS),
            p("gate_forward_voltage", "VGS(F)", "V", GS),
            p("gate_cutoff_current", "IG(off)", "nA", None),
            p("drain_cutoff_current", "ID(off)", "nA", DS),
            p("gate_operating_current", "IG(op)", "pA", None),
            p("gate_drain_leakage_current", "IGDO", "pA", GD),
        ],
    },
    Group {
        title: "Capacitance and gate charge",
        family: Family::All,
        properties: &[
            p("input_capacitance", "Ciss", "pF", None),
            p("output_capacitance", "Coss", "pF", None),
            p("reverse_transfer_capacitance", "Crss", "pF", None),
            p("gate_source_capacitance", "Cgs", "pF", GS),
            p("gate_drain_capacitance", "Cgd", "pF", GD),
            p("drain_source_capacitance", "Cds", "pF", DS),
            p("gate_charge", "Qg (total)", "nC", GS),
            p("gate_source_charge", "Qgs", "nC", GS),
            p("gate_drain_charge", "Qgd (Miller)", "nC", GD),
            p("gate_source_pre_plateau_charge", "Qgs1", "nC", GS),
            p("gate_source_switching_charge", "Qgs2", "nC", GS),
            p("gate_threshold_charge", "Qg(th)", "nC", None),
            p("switching_gate_charge", "Qsw", "nC", None),
            p("capacitive_charge", "Qc", "nC", None),
            p("commutation_charge", "Qcomm", "nC", None),
            p("charge_measurement_method", "", "", None),
            p("output_charge", "Qoss", "nC", DS),
            p("output_capacitance_energy_equivalent", "Coss(er)", "pF", DS),
            p("output_capacitance_time_equivalent", "Coss(tr)", "pF", DS),
            p("output_capacitance_energy", "Eoss", "µJ", DS),
            p("internal_gate_resistance", "Rg(int)", "Ω", None),
        ],
    },
    Group {
        title: "Switching and test circuit",
        family: Family::All,
        properties: &[
            p("turn_on_delay", "td(on)", "ns", None),
            p("turn_off_delay", "td(off)", "ns", None),
            p("rise_time", "tr", "ns", None),
            p("fall_time", "tf", "ns", None),
            p("turn_on_time", "ton", "ns", None),
            p("turn_off_time", "toff", "ns", None),
            p("turn_on_energy", "Eon", "µJ", None),
            p("turn_off_energy", "Eoff", "µJ", None),
            p("switching_frequency", "fsw", "kHz", None),
            p("drain_voltage_slew_rate", "dVDS/dt", "V/ns", DS),
            p("drain_current_slew_rate", "dID/dt", "A/µs", DS),
            p("external_gate_resistance_on", "Rg(on)", "Ω", None),
            p("external_gate_resistance_off", "Rg(off)", "Ω", None),
            p("drive_voltage", "Vdrive", "V", None),
            p("bus_voltage", "VDD", "V", None),
            p("driver_source_current", "IG(source)", "A", None),
            p("driver_sink_current", "IG(sink)", "A", None),
            p("driver_source_resistance", "Rdrv(source)", "Ω", None),
            p("driver_sink_resistance", "Rdrv(sink)", "Ω", None),
            p("commutating_device", "", "", None),
            p("reverse_test_voltage", "VR", "V", None),
            p("load_inductance", "Lload", "µH", None),
            p("load_resistance", "RL", "Ω", None),
            p("loop_inductance", "Lloop", "nH", None),
            p("dead_time", "tdead", "ns", None),
            p("pulse_duration", "tp", "µs", None),
            p("pulse_period", "T", "µs", None),
            p("duty_cycle", "D", "%", None),
        ],
    },
    Group {
        title: "Reverse conduction and recovery",
        family: Family::Insulated,
        properties: &[
            p("reverse_conduction_voltage", "VSD", "V", SD),
            p("reverse_current", "IS", "A", SD),
            p("pulsed_reverse_current", "ISM", "A", SD),
            p("reverse_recovery_rise_time", "ta", "ns", None),
            p("reverse_recovery_fall_time", "tb", "ns", None),
            p("recovery_time", "trr", "ns", None),
            p("recovery_charge", "Qrr", "nC", None),
            p("peak_reverse_recovery_current", "Irrm", "A", None),
            p("reverse_recovery_energy", "Erec", "µJ", None),
            p("reverse_recovery_softness", "S factor", "1", None),
            p("commutation_current_slew_rate", "diF/dt", "A/µs", None),
            p(
                "reverse_recovery_voltage_slew_rate",
                "dv/dt recovery",
                "V/ns",
                None,
            ),
        ],
    },
    Group {
        title: "Avalanche and safe operating area",
        family: Family::All,
        properties: &[
            p("avalanche_energy", "EAS (single pulse)", "mJ", None),
            p("repetitive_avalanche_energy", "EAR", "mJ", None),
            p("avalanche_current", "IAS", "A", DS),
            p("repetitive_avalanche_current", "IAR", "A", DS),
            p("avalanche_duration", "tAV", "µs", None),
            p("safe_operating_drain_current", "SOA ID at VDS", "A", DS),
            p("short_circuit_withstand_time", "tSC", "µs", None),
            p("short_circuit_drain_current", "ISC", "A", DS),
            p("pulse_regime", "", "", None),
        ],
    },
    Group {
        title: "Thermal and package",
        family: Family::All,
        properties: &[
            p("power_dissipation", "PD", "W", None),
            p("power_derating_rate", "dPD/dTC", "W/°C", None),
            p("junction_temperature", "TJ", "°C", None),
            p("ambient_temperature", "TA", "°C", None),
            p("case_temperature", "TC", "°C", None),
            p("mounting_base_temperature", "Tmb", "°C", None),
            p("storage_temperature", "Tstg", "°C", None),
            p("soldering_temperature", "Tsolder", "°C", None),
            p("thermal_resistance_junction_case", "RθJC", "°C/W", None),
            p("thermal_resistance_junction_ambient", "RθJA", "°C/W", None),
            p("thermal_resistance_junction_board", "RθJB", "°C/W", None),
            p(
                "transient_thermal_impedance_junction_case",
                "ZθJC(t)",
                "°C/W",
                None,
            ),
            p(
                "transient_thermal_impedance_junction_ambient",
                "ZθJA(t)",
                "°C/W",
                None,
            ),
            p("common_source_inductance", "LCS", "nH", None),
            p("source_inductance", "LS", "nH", None),
            p("drain_inductance", "LD", "nH", None),
            p("thermal_characterization_junction_top", "ΨJT", "°C/W", None),
            p("thermal_characterization_junction_board", "ΨJB", "°C/W", None),
            p("lead_temperature", "TL", "°C", None),
            p("copper_area", "Acu", "mm²", None),
            p("gate_inductance", "LG", "nH", None),
            p("test_standard", "", "", None),
            p("test_fixture", "", "", None),
        ],
    },
    Group {
        title: "Matched channels and gate protection",
        family: Family::All,
        properties: &[
            p("gate_source_voltage_mismatch", "ΔVGS", "mV", None),
            p("gate_source_mismatch_drift", "dΔVGS/dT", "µV/°C", None),
            p("drain_current_matching_ratio", "ID1/ID2", "1", None),
            p("transconductance_matching_ratio", "gm1/gm2", "1", None),
            p("thermal_coupling_between_dies", "Zθ12", "°C/W", None),
            p("input_resistance", "Rin", "TΩ", None),
            p("gate_clamp_voltage", "Vclamp", "V", None),
            p("gate_clamp_leakage", "Iclamp", "pA", None),
            p("gate_clamp_pulse_current", "Iclamp(pk)", "mA", None),
        ],
    },
    Group {
        title: "Bias history and measurement setup",
        family: Family::All,
        properties: &[
            p("quiescent_gate_bias", "VGSQ", "V", GS),
            p("quiescent_drain_bias", "VDSQ", "V", DS),
            p("off_state_stress_duration", "toff(stress)", "µs", None),
            p("post_turn_on_sampling_delay", "tmeas", "ns", None),
            p("measurement_window", "twindow", "ns", None),
            p("stress_cycle_count", "Ncycles", "1", None),
            p("switching_regime", "", "", None),
            p("measurement_configuration", "", "", None),
            p("package_variant", "", "", None),
            p("thermal_resistance_junction_case_top", "RθJC(top)", "°C/W", None),
            p(
                "thermal_resistance_junction_case_bottom",
                "RθJC(bot)",
                "°C/W",
                None,
            ),
        ],
    },
    Group {
        title: "Small signal, RF and noise",
        family: Family::All,
        properties: &[
            p("cutoff_frequency", "fT", "MHz", None),
            p("maximum_oscillation_frequency", "fmax", "GHz", None),
            p("frequency", "f", "MHz", None),
            p("bandwidth", "BW", "MHz", None),
            p("gain", "Gp", "dB", None),
            p("noise_figure", "NF", "dB", None),
            p("input_noise_voltage_peak_to_peak", "en(pp)", "µV", None),
            p("input_noise_voltage_rms", "en(rms)", "µV", None),
            p("noise_band_lower_frequency", "fL", "Hz", None),
            p("noise_band_upper_frequency", "fH", "Hz", None),
            p("noise_source_resistance", "RS(noise)", "kΩ", None),
            p("voltage_noise_density", "en", "nV/√Hz", None),
            p("current_noise_density", "in", "pA/√Hz", None),
            p("noise_corner_frequency", "fcorner", "Hz", None),
            p("maximum_available_gain", "MAG", "dB", None),
            p("maximum_stable_gain", "MSG", "dB", None),
            p("power_added_efficiency", "PAE", "%", None),
            p("rf_reference_impedance", "Z0", "Ω", None),
            p("rf_input_power", "Pin", "dBm", None),
            p("rf_output_power", "Pout", "dBm", None),
            p("s11_magnitude", "|S11|", "1", None),
            p("s11_phase", "∠S11", "deg", None),
            p("s21_magnitude", "|S21|", "1", None),
            p("s21_phase", "∠S21", "deg", None),
            p("s12_magnitude", "|S12|", "1", None),
            p("s12_phase", "∠S12", "deg", None),
            p("s22_magnitude", "|S22|", "1", None),
            p("s22_phase", "∠S22", "deg", None),
            p("input_intercept_power", "IIP3", "dBm", None),
            p("output_intercept_power", "OIP3", "dBm", None),
            p("compression_output_power", "P1dB", "dBm", None),
        ],
    },
];
