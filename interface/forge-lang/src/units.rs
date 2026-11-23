//! What a person types in a value field, as Forge text: the part holds mm and degrees, so `1 in`
//! is written `25.4mm`, `3/8 in` is `9.525mm`, `45 deg` is `45deg`, `1 ft 3 in` is
//! `304.8mm+76.2mm`, and a bare number takes the unit the part is shown in (`# units in`,
//! forge-units' table). Every number written is one Forge's lexer reads: digits first, then `mm`,
//! `deg` or nothing; a value given a unit keeps saying what it measures.

pub use forge_units::*;

use crate::syntax::{self, TokKind, Unit};
use crate::values::fmt;
use forge_math::dmath;

/// What a name in a field stands for: its value in mm or degrees, and what it measures (none: a
/// plain number).
pub type Env<'a> = dyn Fn(&str) -> Option<(f64, Option<Kind>)> + 'a;

/// The functions a field may call, all Forge's own, and `pi`.
const FUNCS: [&str; 11] = [
    "sqrt", "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "min", "max", "abs",
];

/// Is this one of the functions, or `pi`?
pub fn is_call(name: &str) -> bool {
    FUNCS.contains(&name) || name == "pi"
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

/// A number as Forge's lexer reads it: a digit first (`.5` is `0.5`).
fn number(n: &str) -> String {
    if n.starts_with('.') {
        format!("0{n}")
    } else {
        n.to_string()
    }
}

/// A value of this unit's dimension, in mm or degrees, as Forge writes it; `None` for a unit
/// Forge has no number of (a mass).
fn literal(v: f64, unit: &str) -> Option<String> {
    let (d, top, bottom) = lookup(unit)?;
    let v = v * top / bottom;
    match d {
        LENGTH => Some(format!("{}mm", fmt(v))),
        ANGLE => Some(format!("{}deg", fmt(v))),
        _ => None,
    }
}

/// What's typed in a field, as Forge text: `1 in` is `25.4mm`, `3/8 in` is `9.525mm`, `1 ft 3 in`
/// is `304.8mm+76.2mm`, `30 deg` (or `30°`) is `30deg`, and text of numbers alone is in the part's
/// unit (`2` in a part shown in inches is `50.8mm`, `1+2` in cm is `(1+2)*10mm`; in degrees,
/// `45deg`).
/// `is_param` says a word is a parameter, which then isn't a unit. Text that's nothing like this
/// comes back much as it was, for the reader to say why.
pub fn to_file(typed: &str, doc_unit: &str, is_param: &dyn Fn(&str) -> bool) -> String {
    let t = toks(typed);
    let is_unit = |w: &str| lookup(w).is_some() && !is_param(w);
    let mut out = String::new();
    let mut k = 0;
    let mut any_unit = false;
    let mut last_literal = false;
    // a value with its unit, a literal of its own; joined to one just before by `+`
    let push = |out: &mut String, v: f64, u: &str, raw: String, last: bool| {
        if last {
            out.push('+');
        }
        out.push_str(&literal(v, u).unwrap_or(raw));
    };
    while k < t.len() {
        // a fraction with its unit: 3/8 in
        if let (Tok::Num(a), Some(Tok::Other('/')), Some(Tok::Num(c)), Some(Tok::Word(u))) =
            (&t[k], t.get(k + 1), t.get(k + 2), t.get(k + 3))
            && is_unit(u)
            && let (Ok(x), Ok(y)) = (a.parse::<f64>(), c.parse::<f64>())
        {
            push(&mut out, x / y, u, format!("({a}/{c}){u}"), last_literal);
            k += 4;
            any_unit = true;
            last_literal = true;
            continue;
        }
        match &t[k] {
            Tok::Num(n) => {
                let unit = match t.get(k + 1) {
                    Some(Tok::Word(u)) if is_unit(u) => Some(u.as_str()),
                    Some(Tok::Other('°')) => Some("deg"),
                    _ => None,
                };
                if let (Some(u), Ok(v)) = (unit, n.parse::<f64>()) {
                    push(&mut out, v, u, format!("{n}{u}"), last_literal);
                    k += 2;
                    any_unit = true;
                    last_literal = true;
                    continue;
                }
                out.push_str(&number(n));
            }
            Tok::Word(w) => {
                // a unit right after a bracket: a factor
                if out.ends_with(')') && is_unit(w) {
                    any_unit = true;
                    match literal(1.0, w) {
                        Some(f) => {
                            out.push('*');
                            out.push_str(&f);
                        }
                        None => out.push_str(w),
                    }
                } else {
                    out.push_str(w);
                }
            }
            Tok::Other(c) => out.push(*c),
        }
        last_literal = false;
        k += 1;
    }
    let words = t.iter().any(|x| matches!(x, Tok::Word(_)));
    // nothing but numbers: all in the part's unit; mm is what a bare number means
    if !any_unit && !words && !out.is_empty() && doc_unit != "mm" {
        if let [Tok::Num(n)] = &t[..]
            && let Ok(v) = n.parse::<f64>()
            && let Some(l) = literal(v, doc_unit)
        {
            return l;
        }
        if let Some(f) = literal(1.0, doc_unit) {
            return format!("({out})*{f}");
        }
    }
    out
}

/// Forge text as a person reads it: `30deg` is `30 deg`, `10mm+25.4mm` is `10 mm + 25.4 mm`.
/// Expressions of anything else read as they are.
pub fn to_person(file: &str) -> String {
    let t = toks(file);
    let mut out = String::new();
    let mut k = 0;
    while k < t.len() {
        match &t[k] {
            Tok::Num(n) => {
                out.push_str(n);
                if let Some(Tok::Word(u)) = t.get(k + 1)
                    && lookup(u).is_some()
                {
                    out.push(' ');
                    out.push_str(u);
                    k += 1;
                }
            }
            Tok::Word(w) => out.push_str(w),
            Tok::Other(c @ ('+' | '-')) if !out.is_empty() && !out.ends_with(['(', '*', '/', '^', ',']) => {
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
            Tok::Word(w) => lookup(w).is_some() || is_param(w) || is_call(w),
            _ => true,
        })
}

/// A field's text, for the part: a value in `doc`'s unit, or anything else just as it is.
pub fn field_file(typed: &str, kind: Kind, doc: &str, is_param: &dyn Fn(&str) -> bool) -> String {
    if !is_value(&toks(typed), is_param) {
        return typed.trim().to_string();
    }
    to_file(typed, unit_of(kind, doc), is_param)
}

/// The part's text for a field: a bare length is in mm, so it says so unless that's what the part
/// is shown in.
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

/// What's typed, checked: the Forge text and its value in mm or degrees; or why not, for the
/// field.
pub fn read_field(typed: &str, kind: Kind, doc_unit: &str, env: &Env) -> Result<(String, f64), String> {
    let file = to_file(typed, unit_of(kind, doc_unit), &|w| env(w).is_some());
    let (v, got) = eval(&file, env).map_err(|why| format!("'{typed}': {why}"))?;
    match (kind, got) {
        (_, None) => Ok((file, v)),
        (Kind::Length, Some(LEN)) | (Kind::Angle, Some(ANG)) => Ok((file, v)),
        (Kind::Length, Some(ANG)) => Err(format!("'{typed}' is an angle; a length goes here")),
        (Kind::Angle, Some(LEN)) => Err(format!("'{typed}' is a length; an angle goes here")),
        (_, Some(d)) => Err(format!(
            "'{typed}' measures {}, not {}",
            words(d),
            if kind == Kind::Length {
                "a length"
            } else {
                "an angle"
            }
        )),
    }
}

/// Powers of length and angle; none for a plain number, which takes any.
type D = Option<[i8; 2]>;
const LEN: [i8; 2] = [1, 0];
const ANG: [i8; 2] = [0, 1];

fn words(d: [i8; 2]) -> String {
    match d {
        LEN => "a length".into(),
        ANG => "an angle".into(),
        [2, 0] => "an area".into(),
        [3, 0] => "a volume".into(),
        [l, a] => format!("length^{l} angle^{a}"),
    }
}

/// The value of Forge text in mm or degrees and what it measures: `+ - * / ^`, brackets, the
/// functions (trig takes degrees, inverse trig gives them), `pi`, and names from `env`.
fn eval(src: &str, env: &Env) -> Result<(f64, D), String> {
    let toks = syntax::lex(src).map_err(|_| format!("'{src}' is not Forge text"))?;
    let toks: Vec<TokKind> = toks
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| !matches!(k, TokKind::Nl | TokKind::Eof))
        .collect();
    let mut p = Ev { t: &toks, k: 0, env };
    let v = p.sum()?;
    if p.k < toks.len() {
        return Err("more after the value than it reads".into());
    }
    if !v.0.is_finite() {
        return Err("not a finite number".into());
    }
    Ok(v)
}

struct Ev<'a, 'e> {
    t: &'a [TokKind],
    k: usize,
    env: &'a Env<'e>,
}

type V = (f64, D);

impl Ev<'_, '_> {
    fn eat(&mut self, op: &str) -> bool {
        let hit = matches!(self.t.get(self.k), Some(TokKind::Op(o)) if *o == op);
        self.k += hit as usize;
        hit
    }
    fn sum(&mut self) -> Result<V, String> {
        let mut a = self.product()?;
        loop {
            let sign = if self.eat("+") {
                1.0
            } else if self.eat("-") {
                -1.0
            } else {
                return Ok(a);
            };
            let b = self.product()?;
            let d = match (a.1, b.1) {
                (Some(x), Some(y)) if x != y => return Err(format!("adds {} to {}", words(y), words(x))),
                (x, y) => x.or(y),
            };
            a = (a.0 + sign * b.0, d);
        }
    }
    fn product(&mut self) -> Result<V, String> {
        let mut a = self.unary()?;
        loop {
            let sign = if self.eat("*") {
                1
            } else if self.eat("/") {
                -1
            } else {
                return Ok(a);
            };
            let b = self.unary()?;
            let d = match (a.1, b.1) {
                (Some(x), Some(y)) => Some([x[0] + sign * y[0], x[1] + sign * y[1]]),
                (None, Some(_)) if sign < 0 => return Err("a number over a length or an angle".into()),
                (x, y) => x.or(y),
            };
            a = if sign > 0 {
                (a.0 * b.0, d)
            } else if b.0 == 0.0 {
                return Err("division by zero".into());
            } else {
                (a.0 / b.0, d)
            };
        }
    }
    fn unary(&mut self) -> Result<V, String> {
        if self.eat("-") {
            let (v, d) = self.unary()?;
            return Ok((-v, d));
        }
        if self.eat("+") {
            return self.unary();
        }
        let base = self.atom()?;
        if !self.eat("^") {
            return Ok(base);
        }
        let (e, ed) = self.unary()?;
        if ed.is_some() {
            return Err("an exponent is a plain number".into());
        }
        let d = base.1.map(|d| d.map(|p| p * e as i8));
        Ok((pow(base.0, e)?, d))
    }
    fn atom(&mut self) -> Result<V, String> {
        let t = self.t.get(self.k).cloned().ok_or("the value ends too soon")?;
        self.k += 1;
        match t {
            TokKind::Op("(") => {
                let v = self.sum()?;
                if !self.eat(")") {
                    return Err("a '(' is not closed".into());
                }
                Ok(v)
            }
            TokKind::Num(v, Unit::None) => Ok((v, None)),
            TokKind::Num(v, Unit::Mm) => Ok((v, Some(LEN))),
            TokKind::Num(v, Unit::Deg) => Ok((v, Some(ANG))),
            TokKind::Id(name) if name == "pi" => Ok((dmath::PI, None)),
            TokKind::Id(name) if FUNCS.contains(&name.as_str()) => {
                if !self.eat("(") {
                    return Err(format!("{name} needs parentheses: {name}(x)"));
                }
                let mut args = vec![self.sum()?];
                while self.eat(",") {
                    args.push(self.sum()?);
                }
                if !self.eat(")") {
                    return Err(format!("{name}( is not closed"));
                }
                call(&name, &args)
            }
            TokKind::Id(name) => {
                let (v, k) = (self.env)(&name).ok_or_else(|| format!("unknown parameter '{name}'"))?;
                Ok((
                    v,
                    k.map(|k| match k {
                        Kind::Length => LEN,
                        Kind::Angle => ANG,
                    }),
                ))
            }
            t => Err(format!("unexpected {t:?}")),
        }
    }
}

fn call(name: &str, args: &[V]) -> Result<V, String> {
    let want = if matches!(name, "min" | "max" | "atan2") {
        2
    } else {
        1
    };
    if args.len() != want {
        return Err(format!(
            "{name} takes {want} argument{}",
            if want > 1 { "s" } else { "" }
        ));
    }
    let (x, d) = args[0];
    let same = |d: D, e: D| match (d, e) {
        (Some(a), Some(b)) if a != b => Err(format!("{name} of {} and {}", words(a), words(b))),
        (a, b) => Ok(a.or(b)),
    };
    let trig = |f: fn(f64) -> f64| {
        if d.is_some_and(|d| d != ANG) {
            return Err(format!("{name} takes an angle"));
        }
        Ok((f(x / 180.0 * dmath::PI), None))
    };
    // an angle out, in degrees, from a ratio (length over length) or a point
    let inv = |a: f64, d: D| {
        if d == Some(ANG) {
            return Err(format!("{name} takes a ratio, not an angle"));
        }
        if a.is_nan() {
            return Err(format!("{name} of a number outside -1..1"));
        }
        Ok((a * (180.0 / dmath::PI), Some(ANG)))
    };
    match name {
        "sqrt" if x < 0.0 => Err("sqrt of a negative number".into()),
        "sqrt" => match d {
            Some([l, a]) if l % 2 != 0 || a % 2 != 0 => Err(format!("sqrt of {}", words([l, a]))),
            d => Ok((x.sqrt(), d.map(|d| d.map(|p| p / 2)))),
        },
        "abs" => Ok((x.abs(), d)),
        "sin" => trig(dmath::sin),
        "cos" => trig(dmath::cos),
        "tan" => trig(|a| {
            let (s, c) = dmath::sin_cos(a);
            s / c
        }),
        "asin" => inv(dmath::asin(x), d),
        "acos" => inv(dmath::acos(x), d),
        "atan" => inv(dmath::atan(x), d),
        "atan2" => inv(dmath::atan2(x, args[1].0), same(d, args[1].1)?),
        "min" => Ok((x.min(args[1].0), same(d, args[1].1)?)),
        _ => Ok((x.max(args[1].0), same(d, args[1].1)?)),
    }
}

/// `x^e` for a whole `e` (by squaring) or `e` = 0.5.
fn pow(x: f64, e: f64) -> Result<f64, String> {
    if e == 0.5 {
        return if x < 0.0 {
            Err("^0.5 of a negative number".into())
        } else {
            Ok(x.sqrt())
        };
    }
    if e.fract() != 0.0 || e.abs() > 1024.0 {
        return Err(format!("^ takes a whole exponent (or 0.5), not {e}"));
    }
    let (mut b, mut n, mut r) = (x, e.abs() as u32, 1.0);
    while n > 0 {
        if n & 1 == 1 {
            r *= b;
        }
        b *= b;
        n >>= 1;
    }
    Ok(if e < 0.0 { 1.0 / r } else { r })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<(f64, Option<Kind>)> {
        None
    }

    /// Forge's lexer reads it, a number at a time.
    fn lexes(s: &str) -> bool {
        syntax::lex(s).is_ok()
    }

    #[test]
    fn what_is_typed_is_written_as_forge_numbers_in_mm_and_degrees() {
        let w = |t: &str, u: &str| to_file(t, u, &|n| n == "w");
        for (typed, file) in [
            ("1 in", "25.4mm"),
            ("25.4 mm", "25.4mm"),
            ("10 mm + 1 in", "10mm+25.4mm"),
            ("3/8 in", "9.525mm"),
            ("1 ft 3 in", "304.8mm+76.2mm"),
            ("1.5e-3 m", "1.5mm"),
            ("45 deg", "45deg"),
            ("45°", "45deg"),
            ("0.5 turn", "180deg"),
        ] {
            assert_eq!(w(typed, "mm"), file);
            assert_eq!(w(typed, "in"), file, "a unit written stays");
            assert!(lexes(file), "{file}");
        }
        // bare numbers take the part's unit; in mm they are as typed
        assert_eq!(w("2", "in"), "50.8mm");
        assert_eq!(w("2", "mm"), "2");
        assert_eq!(w(".5", "mm"), "0.5");
        assert_eq!(w("1+2", "cm"), "(1+2)*10mm");
        assert_eq!(w("45", "deg"), "45deg");
        assert_eq!(w("(1+2)in", "mm"), "(1+2)*25.4mm");
        // a parameter is not a unit, and its own expression stays as it is
        assert_eq!(w("w/2", "in"), "w/2");
        assert_eq!(w("w/2 + 1 in", "mm"), "w/2+25.4mm");
        assert_eq!(to_file("2*m", "mm", &|n| n == "m"), "2*m");
        assert_eq!(to_file("2 m", "mm", &|n| n == "m"), "2m");
        for f in ["50.8mm", "(1+2)*10mm", "45deg", "w/2+25.4mm"] {
            assert!(lexes(f), "{f}");
        }
    }

    #[test]
    fn values_read_in_mm_and_degrees_and_a_mix_is_refused_in_the_field() {
        let v = |t: &str, k| read_field(t, k, "mm", &none);
        assert_eq!(v("1 in", Kind::Length).unwrap(), ("25.4mm".into(), 25.4));
        assert_eq!(v("1 ft 3 in", Kind::Length).unwrap().1, 381.0);
        assert_eq!(v("3/8 in", Kind::Length).unwrap().1, 9.525);
        assert_eq!(v("1.5e-3 m", Kind::Length).unwrap().1, 1.5);
        assert_eq!(v("0.5 turn", Kind::Angle).unwrap(), ("180deg".into(), 180.0));
        assert_eq!(v("45", Kind::Angle).unwrap(), ("45deg".into(), 45.0));
        assert_eq!(
            read_field("2", Kind::Length, "in", &none).unwrap(),
            ("50.8mm".into(), 50.8)
        );
        assert_eq!(
            read_field("2", Kind::Length, "mm", &none).unwrap(),
            ("2".into(), 2.0)
        );
        assert!(v("10 mm + 45 deg", Kind::Length).is_err());
        assert!(v("45 deg", Kind::Length).unwrap_err().contains("angle"));
        assert!(v("1 in", Kind::Angle).unwrap_err().contains("length"));
        assert!(v("2 in * 3 in", Kind::Length).is_err(), "an area isn't a length");
        let env = |n: &str| (n == "w").then_some((40.0, Some(Kind::Length)));
        assert_eq!(
            read_field("w/2 + 1 in", Kind::Length, "mm", &env).unwrap().1,
            45.4
        );
        assert_eq!(read_field("sqrt(4)", Kind::Length, "mm", &none).unwrap().1, 2.0);
        assert!((read_field("atan(1)", Kind::Angle, "mm", &none).unwrap().1 - 45.0).abs() < 1e-12);
    }

    #[test]
    fn a_field_shows_what_the_part_says_and_gives_it_back_unchanged() {
        let none = |_: &str| false;
        for (doc, file, shown) in [
            ("mm", "10", "10"),
            ("in", "10", "10 mm"),
            ("mm", "30deg", "30 deg"),
            ("mm", "10mm+25.4mm", "10 mm + 25.4 mm"),
            ("mm", "thru", "thru"),
            ("in", "to base off 3", "to base off 3"),
        ] {
            let kind = if file.ends_with("deg") {
                Kind::Angle
            } else {
                Kind::Length
            };
            assert_eq!(field_show(file, kind, doc), shown);
        }
        assert_eq!(field_file("thru", Kind::Length, "in", &none), "thru");
        assert_eq!(
            field_file("to base off 3", Kind::Length, "in", &none),
            "to base off 3"
        );
        assert_eq!(field_file("2", Kind::Length, "in", &none), "50.8mm");
        assert_eq!(field_file("45", Kind::Angle, "in", &none), "45deg");
        assert_eq!(field_file("sqrt(4)", Kind::Length, "mm", &none), "sqrt(4)");
        assert!(is_bare("12.5") && is_bare("1+2") && !is_bare("30deg") && !is_bare("w"));
    }
}
