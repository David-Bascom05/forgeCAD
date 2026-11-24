//! A kind's fields, as the builder's forms show them (research/cam/library-builder.md §3): the
//! data owner says each field's type, unit, whether it is needed and whether it carries
//! provenance; the form, its units, its provenance chip and its "unknown" button follow.

/// What a field holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Num,
    Int,
    /// Fixed-length numbers (a point, a direction): n of them.
    Vec(usize),
    /// [lo, hi].
    Range,
    Text,
    Bool,
    /// One of these words.
    Choice(&'static [&'static str]),
    /// The id of a record of another kind ("holder", "machine", ...), or a joint of this one.
    Link(&'static str),
    /// Rows of these columns (each a number with its unit): a profile's rings, a power curve.
    Points(&'static [(&'static str, Unit)]),
}

/// The unit a number is kept in (mm, degrees; shown in the user's units).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    None,
    /// mm
    Length,
    /// degrees
    Angle,
    /// mm/min
    Feed,
    /// deg/min
    AngFeed,
    /// mm/s² (deg/s² on a rotary)
    Accel,
    Rpm,
    /// kW
    Power,
    /// s
    Time,
    /// kg
    Mass,
    /// m/min (cutting speed)
    Speed,
    /// N
    Force,
    /// kPa
    Pressure,
    /// N·m
    Torque,
    /// m³/h
    Flow,
}

impl Unit {
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::None => "",
            Unit::Length => "mm",
            Unit::Angle => "°",
            Unit::Feed => "mm/min",
            Unit::AngFeed => "°/min",
            Unit::Accel => "mm/s²",
            Unit::Rpm => "rpm",
            Unit::Power => "kW",
            Unit::Time => "s",
            Unit::Mass => "kg",
            Unit::Speed => "m/min",
            Unit::Force => "N",
            Unit::Pressure => "kPa",
            Unit::Torque => "N·m",
            Unit::Flow => "m³/h",
        }
    }
}

/// Whether a field must be there.
#[derive(Clone, Debug, PartialEq)]
pub enum Need {
    Required,
    /// Absent means this (said in the form).
    Optional(&'static str),
    /// Required when the condition holds (e.g. "type = bull"), else absent.
    When(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Its provenance path: "joint.X.range", "dc", "profile".
    pub path: String,
    pub label: &'static str,
    pub ty: Ty,
    pub unit: Unit,
    pub need: Need,
    /// The form's section.
    pub group: &'static str,
    pub help: &'static str,
    /// Shows a provenance chip and "unknown".
    pub cited: bool,
}

impl Field {
    pub fn new(path: impl Into<String>, label: &'static str, ty: Ty, unit: Unit) -> Field {
        Field {
            path: path.into(),
            label,
            ty,
            unit,
            need: Need::Required,
            group: "",
            help: "",
            cited: true,
        }
    }
    pub fn optional(mut self, absent: &'static str) -> Field {
        self.need = Need::Optional(absent);
        self
    }
    pub fn when(mut self, cond: &'static str) -> Field {
        self.need = Need::When(cond);
        self
    }
    pub fn group(mut self, g: &'static str) -> Field {
        self.group = g;
        self
    }
    pub fn help(mut self, h: &'static str) -> Field {
        self.help = h;
        self
    }
    pub fn uncited(mut self) -> Field {
        self.cited = false;
        self
    }
}
