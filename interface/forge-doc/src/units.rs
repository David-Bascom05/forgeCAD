//! Units, all in one table. Files and the kernel are in mm and degrees; everything here is how a
//! person writes and reads those. Changing the units a part is shown in changes no geometry.

use crate::expr;

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
    ("rad", ANGLE, 180.0, forge_math::dmath::PI),
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

/// The length units a document can be shown in.
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

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    /// A number, as written.
    Num(String),
    /// A word: a unit if the table has it and nothing above is called that.
    Word(String),
    /// Anything else, as written.
    Other(char),
}

fn toks(s: &str) -> Vec<Tok> {
    let b = s.as_bytes();
    let (mut k, mut out) = (0, Vec::new());
    while k < b.len() {
        let c = b[k];
        if c.is_ascii_whitespace() {
            k += 1;
        } else if c.is_ascii_digit() || c == b'.' {
            let st = k;
            while k < b.len() {
                let d = b[k];
                let exp = (d == b'-' || d == b'+') && matches!(b[k - 1], b'e' | b'E');
                let e = (d == b'e' || d == b'E')
                    && b.get(k + 1)
                        .is_some_and(|n| n.is_ascii_digit() || *n == b'-' || *n == b'+');
                if !(d.is_ascii_digit() || d == b'.' || e || exp) {
                    break;
                }
                k += 1;
            }
            out.push(Tok::Num(s[st..k].to_string()));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let st = k;
            while k < b.len() && (b[k].is_ascii_alphanumeric() || b[k] == b'_') {
                k += 1;
            }
            out.push(Tok::Word(s[st..k].to_string()));
        } else {
            let ch = s[k..].chars().next().expect("in bounds");
            k += ch.len_utf8();
            out.push(Tok::Other(ch));
        }
    }
    out
}

/// What's typed in a field, as the file writes it: `1 in` is `1in`, `3/8 in` is `(3/8)in`, `1 ft 3 in`
/// is `1ft+3in`, and a number with no unit takes the document's. `is_param` says a word is a
/// parameter, which then isn't a unit. Text that's nothing like this comes back as it was, for the
/// reader to say why.
pub fn to_file(typed: &str, doc_unit: &str, is_param: &dyn Fn(&str) -> bool) -> String {
    let t = toks(typed);
    let is_unit = |w: &str| lookup(w).is_some() && !is_param(w);
    let mut out = String::new();
    let mut k = 0;
    let mut any_unit = false;
    let mut last_literal = false;
    while k < t.len() {
        // a fraction with its unit: 3/8 in
        if let (Tok::Num(a), Some(Tok::Other('/')), Some(Tok::Num(c)), Some(Tok::Word(u))) =
            (&t[k], t.get(k + 1), t.get(k + 2), t.get(k + 3))
            && is_unit(u)
        {
            if last_literal {
                out.push('+');
            }
            out.push_str(&format!("({a}/{c}){u}"));
            k += 4;
            any_unit = true;
            last_literal = true;
            continue;
        }
        match &t[k] {
            Tok::Num(n) => {
                if let Some(Tok::Word(u)) = t.get(k + 1)
                    && is_unit(u)
                {
                    if last_literal {
                        out.push('+');
                    }
                    out.push_str(n);
                    out.push_str(u);
                    k += 2;
                    any_unit = true;
                    last_literal = true;
                    continue;
                }
                out.push_str(n);
            }
            Tok::Word(w) => {
                // a unit right after a bracket
                let closes = out.ends_with(')');
                if closes && is_unit(w) {
                    any_unit = true;
                }
                out.push_str(w);
            }
            Tok::Other(c) => out.push(*c),
        }
        last_literal = false;
        k += 1;
    }
    let words = t.iter().any(|x| matches!(x, Tok::Word(_)));
    // nothing but numbers: all in the document's unit; mm and degrees are what bare numbers mean
    if !any_unit && !words && !out.is_empty() && !matches!(doc_unit, "mm" | "deg") {
        let single = matches!(t[..], [Tok::Num(_)]);
        return if single {
            format!("{out}{doc_unit}")
        } else {
            format!("({out}){doc_unit}")
        };
    }
    out
}

/// The file's text as a person reads it: `1in` is `1 in`, `(3/8)in` is `3/8 in`, `1ft+3in` is
/// `1 ft 3 in`, `10mm+1in` is `10 mm + 1 in`. Expressions of anything else read as they are.
pub fn to_person(file: &str) -> String {
    let t = toks(file);
    let mut out = String::new();
    let mut k = 0;
    while k < t.len() {
        if let (
            Tok::Other('('),
            Some(Tok::Num(a)),
            Some(Tok::Other('/')),
            Some(Tok::Num(c)),
            Some(Tok::Other(')')),
            Some(Tok::Word(u)),
        ) = (
            &t[k],
            t.get(k + 1),
            t.get(k + 2),
            t.get(k + 3),
            t.get(k + 4),
            t.get(k + 5),
        ) && lookup(u).is_some()
        {
            out.push_str(&format!("{a}/{c} {u}"));
            k += 6;
            continue;
        }
        match &t[k] {
            Tok::Num(n) => {
                out.push_str(n);
                if let Some(Tok::Word(u)) = t.get(k + 1)
                    && lookup(u).is_some()
                {
                    out.push(' ');
                    out.push_str(u);
                    k += 1;
                    // feet then inches read as one length
                    let joined = matches!(u.as_str(), "ft")
                        && matches!(t.get(k + 1), Some(Tok::Other('+')))
                        && matches!(t.get(k + 3), Some(Tok::Word(w)) if w == "in");
                    if joined {
                        out.push(' ');
                        k += 1;
                    }
                }
            }
            Tok::Word(w) => out.push_str(w),
            Tok::Other(c @ ('+' | '-')) if !out.is_empty() => {
                out.push(' ');
                out.push(*c);
                out.push(' ');
            }
            Tok::Other(c) => out.push(*c),
        }
        k += 1;
    }
    out
}

/// Is this a value at all? `to base off 3` and `thru` aren't, and stay as they are.
fn is_value(t: &[Tok], is_param: &dyn Fn(&str) -> bool) -> bool {
    !t.is_empty()
        && t.windows(2).all(|w| match (&w[0], &w[1]) {
            (Tok::Word(_), Tok::Word(_)) => false,
            (Tok::Num(_), Tok::Word(u)) => lookup(u).is_some() && !is_param(u),
            _ => true,
        })
        && t.iter().all(|x| match x {
            Tok::Word(w) => lookup(w).is_some() || is_param(w) || expr::is_call(w),
            _ => true,
        })
}

/// A field's text, for the file: a value in `doc`'s unit, or anything else just as it is.
pub fn field_file(typed: &str, kind: Kind, doc: &str, is_param: &dyn Fn(&str) -> bool) -> String {
    if !is_value(&toks(typed), is_param) {
        return typed.trim().to_string();
    }
    let unit = match kind {
        Kind::Length => doc,
        Kind::Angle => "deg",
    };
    to_file(typed, unit, is_param)
}

/// The file's text for a field: a bare number's in mm or degrees, so it says so unless that's what
/// the document's in, when the field's own unit says it.
pub fn field_show(file: &str, kind: Kind, doc: &str) -> String {
    let words = toks(file)
        .windows(2)
        .any(|w| matches!((&w[0], &w[1]), (Tok::Word(_), Tok::Word(_))));
    if words {
        return file.to_string();
    }
    let bare = file.parse::<f64>().is_ok();
    match (kind, bare) {
        (Kind::Length, true) if doc != "mm" => format!("{file} mm"),
        _ => to_person(file),
    }
}

/// Does a field showing this need its unit beside it?
pub fn is_bare(text: &str) -> bool {
    let t = toks(text);
    !t.is_empty() && !t.iter().any(|x| matches!(x, Tok::Word(_)))
}

/// A value in mm or degrees, as this kind reads in the document's unit, without the word.
pub fn number_in(base: f64, kind: Kind, doc: &str) -> String {
    let unit = unit_of(kind, doc);
    match lookup(unit) {
        Some((_, n, m)) => trim(base * m / n),
        None => trim(base),
    }
}

pub fn unit_of(kind: Kind, doc: &str) -> &str {
    match kind {
        Kind::Length => doc,
        Kind::Angle => "deg",
    }
}

/// What's typed, checked: the file's text and its value in mm or degrees; or why not, for the field.
pub fn read_field(typed: &str, kind: Kind, doc_unit: &str, env: &expr::Env) -> Result<(String, f64), String> {
    let unit = match kind {
        Kind::Length => doc_unit,
        Kind::Angle => "deg",
    };
    let file = to_file(typed, unit, &|w| env(w).is_some());
    let (v, got) = expr::eval(&file, env)?;
    use forge_model::Unit;
    match (kind, got) {
        (Kind::Length, Unit::Deg) => Err(format!("'{typed}' is an angle; a length goes here")),
        (Kind::Angle, Unit::Mm) => Err(format!("'{typed}' is a length; an angle goes here")),
        _ => Ok((file, v)),
    }
}

/// The comment line a part's units are kept on: nothing reads it to build.
pub const LINE: &str = "units ";

/// The units a part's text says it's shown in: its length unit, or mm.
pub fn of_text(src: &str) -> &'static str {
    for line in src.lines() {
        if let Some(n) = line.trim().strip_prefix('#')
            && let Some(u) = n.trim().strip_prefix(LINE)
            && let Some(w) = u.split_whitespace().next()
            && DOC_LENGTHS.contains(&w)
        {
            return DOC_LENGTHS[DOC_LENGTHS.iter().position(|d| *d == w).expect("found")];
        }
    }
    "mm"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<(f64, forge_model::Unit)> {
        None
    }

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
    fn what_is_typed_is_written_with_no_space_and_reads_back_as_typed() {
        let w = |t: &str, u: &str| to_file(t, u, &|n| n == "w");
        for (typed, file) in [
            ("1 in", "1in"),
            ("25.4 mm", "25.4mm"),
            ("10 mm + 1 in", "10mm+1in"),
            ("3/8 in", "(3/8)in"),
            ("1 ft 3 in", "1ft+3in"),
            ("1.5e-3 m", "1.5e-3m"),
            ("45 deg", "45deg"),
        ] {
            assert_eq!(w(typed, "mm"), file);
            assert_eq!(w(typed, "in"), file, "a unit written stays");
            assert_eq!(to_person(file), typed);
        }
        // bare numbers take the document's unit; mm stays as bare numbers have always read
        assert_eq!(w("2", "in"), "2in");
        assert_eq!(w("2", "mm"), "2");
        assert_eq!(w("1+2", "cm"), "(1+2)cm");
        // a parameter is not a unit, and its own expression stays as it is
        assert_eq!(w("w/2", "in"), "w/2");
        assert_eq!(to_file("2*m", "mm", &|n| n == "m"), "2*m");
        assert_eq!(to_file("2 m", "mm", &|n| n == "m"), "2m");
    }

    #[test]
    fn values_read_in_mm_and_degrees_and_a_mix_is_refused_in_the_field() {
        let v = |t: &str, k| read_field(t, k, "mm", &none);
        assert_eq!(v("1 in", Kind::Length).unwrap().1, 25.4);
        assert_eq!(v("1 ft 3 in", Kind::Length).unwrap().1, 381.0);
        assert_eq!(v("3/8 in", Kind::Length).unwrap().1, 9.525);
        assert_eq!(v("1.5e-3 m", Kind::Length).unwrap().1, 1.5);
        assert_eq!(v("0.5 turn", Kind::Angle).unwrap().1, 180.0);
        assert!(v("10 mm + 45 deg", Kind::Length).is_err());
        assert!(v("45 deg", Kind::Length).unwrap_err().contains("angle"));
        assert!(v("1 in", Kind::Angle).unwrap_err().contains("length"));
        assert!(v("2 in * 3 in", Kind::Length).is_err(), "an area isn't a length");
    }

    #[test]
    fn a_field_shows_what_the_file_says_and_gives_it_back_unchanged() {
        let none = |_: &str| false;
        for (doc, file, shown) in [
            ("mm", "10", "10"),
            ("in", "10", "10 mm"),
            ("in", "1in", "1 in"),
            ("mm", "(3/8)in", "3/8 in"),
            ("mm", "thru", "thru"),
            ("in", "to base off 3", "to base off 3"),
        ] {
            assert_eq!(field_show(file, Kind::Length, doc), shown);
        }
        assert_eq!(field_file("thru", Kind::Length, "in", &none), "thru");
        assert_eq!(
            field_file("to base off 3", Kind::Length, "in", &none),
            "to base off 3"
        );
        assert_eq!(field_file("2", Kind::Length, "in", &none), "2in");
        assert_eq!(field_file("45", Kind::Angle, "in", &none), "45");
        assert_eq!(field_file("sqrt(4)", Kind::Length, "mm", &none), "sqrt(4)");
        assert_eq!(number_in(25.4, Kind::Length, "in"), "1");
        assert!(is_bare("12.5") && is_bare("1+2") && !is_bare("1 in") && !is_bare("w"));
    }

    #[test]
    fn a_parts_units_are_on_a_comment_and_missing_means_mm() {
        assert_eq!(of_text("base: extrude 5 rect 0 0 1 1\n"), "mm");
        assert_eq!(of_text("# units in\nbase: extrude 5 rect 0 0 1 1\n"), "in");
        assert_eq!(of_text("# units furlong\n"), "mm");
    }
}
