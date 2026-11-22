//! Units as `.fc` text writes them (`1in`, `(3/8)in`): the table itself is forge-units', here as
//! it always was for this crate's readers.

use crate::expr;

pub use forge_units::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<(f64, forge_model::Unit)> {
        None
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
}
