//! Units, all in one table. Parts and the kernel are in mm and degrees; everything here is how a
//! person writes and reads those. Changing the units a part is shown in changes no geometry.
//! Forge text (`forge_lang::units`) is written from this table.

/// Powers of length, angle and mass.
pub type Dim = [i8; 3];

pub const LENGTH: Dim = [1, 0, 0];
pub const ANGLE: Dim = [0, 1, 0];
pub const AREA: Dim = [2, 0, 0];
pub const VOLUME: Dim = [3, 0, 0];
pub const MASS: Dim = [0, 0, 1];
pub const DENSITY: Dim = [-3, 0, 1];
pub const INERTIA: Dim = [2, 0, 1];

/// A unit's name, what it measures, and how many mm, degrees or kg one of it is, as a fraction:
/// multiplying by the top and dividing by the bottom rounds once, so 3/8 in is the 9.525 written.
/// Exact where a definition makes it so: 1 in is 25.4 mm, 1 lb is 0.45359237 kg.
pub const TABLE: [(&str, Dim, f64, f64); 24] = [
    ("nm", LENGTH, 1.0, 1e6),
    ("um", LENGTH, 1.0, 1e3),
    ("mm", LENGTH, 1.0, 1.0),
    ("cm", LENGTH, 10.0, 1.0),
    ("m", LENGTH, 1000.0, 1.0),
    ("km", LENGTH, 1e6, 1.0),
    ("thou", LENGTH, 254.0, 1e4),
    ("mil", LENGTH, 254.0, 1e4),
    ("in", LENGTH, 254.0, 10.0),
    ("ft", LENGTH, 3048.0, 10.0),
    ("yd", LENGTH, 9144.0, 10.0),
    ("mi", LENGTH, 16_093_440.0, 10.0),
    ("deg", ANGLE, 1.0, 1.0),
    ("rad", ANGLE, 180.0, core::f64::consts::PI),
    ("grad", ANGLE, 9.0, 10.0),
    ("turn", ANGLE, 360.0, 1.0),
    ("arcmin", ANGLE, 1.0, 60.0),
    ("arcsec", ANGLE, 1.0, 3600.0),
    ("kg", MASS, 1.0, 1.0),
    ("g", MASS, 1.0, 1e3),
    ("mg", MASS, 1.0, 1e6),
    ("t", MASS, 1000.0, 1.0),
    ("lb", MASS, 45_359_237.0, 1e8),
    ("oz", MASS, 45_359_237.0, 16e8),
];

/// The length units a part can be shown in.
pub const DOC_LENGTHS: [&str; 5] = ["mm", "cm", "m", "in", "ft"];

pub fn lookup(word: &str) -> Option<(Dim, f64, f64)> {
    TABLE.iter().find(|u| u.0 == word).map(|u| (u.1, u.2, u.3))
}

/// A unit: one of the table's, or built of them, as `mm^2`, `kg/m^3`, `lb*in^2`. What it measures
/// and how many base units (mm, deg, kg) one of it is, as a fraction.
pub fn compound(text: &str) -> Result<(Dim, f64, f64), String> {
    let (mut dim, mut top, mut bottom) = ([0i8; 3], 1.0f64, 1.0f64);
    let b = text.as_bytes();
    let (mut k, mut sign) = (0, 1i8);
    if text.trim().is_empty() {
        return Err("no unit".into());
    }
    loop {
        let s = k;
        while k < b.len() && b[k].is_ascii_alphabetic() {
            k += 1;
        }
        let (d, n, m) = lookup(&text[s..k]).ok_or_else(|| format!("'{}' is not a unit", &text[s..k]))?;
        let mut power = 1i8;
        if b.get(k) == Some(&b'^') {
            let s = k + 1;
            k = s;
            while k < b.len() && (b[k].is_ascii_digit() || b[k] == b'-') {
                k += 1;
            }
            power = text[s..k]
                .parse()
                .map_err(|_| format!("'{text}': a power is a whole number"))?;
        }
        let power = power * sign;
        for i in 0..3 {
            dim[i] += d[i] * power;
        }
        for _ in 0..power.abs() {
            if power > 0 {
                (top, bottom) = (top * n, bottom * m);
            } else {
                (top, bottom) = (top * m, bottom * n);
            }
        }
        match b.get(k) {
            None => return Ok((dim, top, bottom)),
            Some(b'*') => sign = 1,
            Some(b'/') => sign = -1,
            _ => return Err(format!("'{text}' is not a unit")),
        }
        k += 1;
    }
}

/// A value in base units, as a person reads it in this unit.
pub fn show(base: f64, unit: &str) -> String {
    match compound(unit) {
        Ok((_, n, m)) => format!("{} {unit}", trim(base * m / n)),
        Err(_) => trim(base),
    }
}

/// Without a tail of digits rounding left: six significant figures or the integer part.
pub fn trim(v: f64) -> String {
    let a = v.abs();
    let places = if a == 0.0 || a >= 100.0 {
        3
    } else if a >= 1.0 {
        4
    } else {
        6
    };
    let s = format!("{v:.places$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.into()
    }
}

/// What a person wants a value to be read as: length or angle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Length,
    Angle,
}

/// A value in mm or degrees, as this kind reads in the part's unit, without the word.
pub fn number_in(base: f64, kind: Kind, doc: &str) -> String {
    let unit = unit_of(kind, doc);
    match lookup(unit) {
        Some((_, n, m)) => trim(base * m / n),
        None => trim(base),
    }
}

/// The unit a value of this kind reads in, in a part shown in `doc`.
pub fn unit_of(kind: Kind, doc: &str) -> &str {
    match kind {
        Kind::Length => doc,
        Kind::Angle => "deg",
    }
}

/// The comment line a part's units are kept on (`# units in`): nothing reads it to build.
pub const LINE: &str = "units ";

/// The units a part's text says it's shown in: its length unit, or mm.
pub fn of_text(src: &str) -> &'static str {
    for line in src.lines() {
        if let Some(n) = line.trim().strip_prefix('#')
            && let Some(u) = n.trim().strip_prefix(LINE)
            && let Some(w) = u.split_whitespace().next()
            && let Some(k) = DOC_LENGTHS.iter().position(|d| *d == w)
        {
            return DOC_LENGTHS[k];
        }
    }
    "mm"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_what_the_definitions_say_and_goes_there_and_back() {
        let one = |w: &str| {
            let (_, n, m) = lookup(w).unwrap();
            n / m
        };
        assert_eq!(one("in"), 25.4);
        assert_eq!(one("ft"), 304.8);
        assert_eq!(one("yd"), 914.4);
        assert_eq!(one("mi"), 1_609_344.0);
        assert_eq!(one("thou"), 0.0254);
        assert_eq!(one("mil"), 0.0254);
        assert_eq!(one("lb"), 0.45359237);
        assert_eq!(one("turn"), 360.0);
        assert_eq!(one("grad"), 0.9);
        for (name, _, n, m) in TABLE {
            for v in [1.0, 0.1, 3.7, 1234.5, 1e-3] {
                let back = v * n / m * m / n;
                assert!((back - v).abs() <= v * 4.0 * f64::EPSILON, "{name} {v} {back}");
            }
            assert!(TABLE.iter().filter(|u| u.0 == name).count() == 1, "{name} twice");
        }
    }

    #[test]
    fn compound_units_measure_what_they_say() {
        assert_eq!(compound("mm^2").unwrap(), (AREA, 1.0, 1.0));
        assert_eq!(compound("in^3").unwrap().0, VOLUME);
        assert_eq!(compound("kg/m^3").unwrap().0, DENSITY);
        assert_eq!(compound("kg*m^2").unwrap().0, INERTIA);
        let (d, n, m) = compound("in^2").unwrap();
        assert_eq!((d, n / m), (AREA, 645.16));
        assert!(compound("furlong").is_err());
        assert_eq!(show(645.16, "in^2"), "1 in^2");
    }

    #[test]
    fn a_value_reads_in_the_parts_unit() {
        assert_eq!(number_in(25.4, Kind::Length, "in"), "1");
        assert_eq!(number_in(45.0, Kind::Angle, "in"), "45");
        assert_eq!(unit_of(Kind::Angle, "cm"), "deg");
    }

    #[test]
    fn a_parts_units_are_on_a_comment_and_missing_means_mm() {
        assert_eq!(of_text("sketch s(on: xy) {}\n"), "mm");
        assert_eq!(of_text("# units in\nshared t = 6\n"), "in");
        assert_eq!(of_text("# units furlong\n"), "mm");
    }
}
