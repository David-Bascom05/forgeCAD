//! An `expect` that failed, in words: `expected no interferences, found 1`, `expected clearance at
//! least 2 mm, found 1.4 mm`. In general `expected <what> <comparison in words> <n>, found <m>`, the
//! comparison as a person says it, counts with their noun in the singular or plural and none as
//! "no". The line as written (`interferences = 0`) is the issue's detail (`raw`).
//!
//! The comparisons are the language's (`syntax::Cmp`: `=`, `<`, `<=`, `>`, `>=`); an `=` may carry a
//! tolerance (`+- T`, `+- T%`), said as `± T`.

use crate::syntax::Cmp;
use crate::values::fmt;

/// How a measure reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A whole number of things: the noun, singular and plural.
    Count(&'static str, &'static str),
    /// A quantity: what it is, and its unit (empty: a bare number).
    Amount(&'static str, &'static str),
    /// `expect valid`: whether the solid is valid.
    Valid,
}

/// A measure's name (an assembly's or a part's, as the expect line names it) as it reads.
pub fn kind(metric: &str) -> Kind {
    match metric {
        "interferences" => Kind::Count("interference", "interferences"),
        "mobility" => Kind::Count("degree of freedom", "degrees of freedom"),
        "faces" => Kind::Count("face", "faces"),
        "edges" => Kind::Count("edge", "edges"),
        "bodies" => Kind::Count("body", "bodies"),
        "holes" => Kind::Count("hole", "holes"),
        "valid" => Kind::Valid,
        "interference" => Kind::Amount("interference volume", "mm³"),
        "clearance" => Kind::Amount("clearance", "mm"),
        "distance" => Kind::Amount("distance", "mm"),
        "gap" => Kind::Amount("gap", "mm"),
        "axis_offset" => Kind::Amount("axis offset", "mm"),
        "axis_angle" => Kind::Amount("axis angle", "°"),
        "x" => Kind::Amount("x", "mm"),
        "y" => Kind::Amount("y", "mm"),
        "z" => Kind::Amount("z", "mm"),
        "volume" => Kind::Amount("volume", "mm³"),
        "area" => Kind::Amount("area", "mm²"),
        "section_area" => Kind::Amount("section area", "mm²"),
        "mass" => Kind::Amount("mass", "kg"),
        "bbox" => Kind::Amount("bounding box size", "mm"),
        // `value(j.turn)`, and any measure without a unit of its own
        _ => Kind::Amount("value", ""),
    }
}

/// A number with its unit: `2 mm`, `30°` (a degree sign follows without a space), `0.5`.
fn with_unit(x: f64, unit: &str) -> String {
    match unit {
        "" => fmt(x),
        "°" => format!("{}°", fmt(x)),
        u => format!("{} {u}", fmt(x)),
    }
}

/// `n` things: `1 interference`, `2 interferences`, `0 interferences`.
fn things(n: f64, one: &str, many: &str) -> String {
    format!("{} {}", fmt(n), if n == 1.0 { one } else { many })
}

/// The failed expect in words: `expected <what> <comparison> <limit>, found <measured>`.
pub fn words(metric: &str, cmp: Cmp, limit: f64, tol: f64, measured: f64) -> String {
    words_in(metric, cmp, limit, tol, measured, false)
}

/// `words`, the numbers radians said in degrees when `deg` (an angle's limit: `value(j.turn)`).
pub fn words_in(metric: &str, cmp: Cmp, limit: f64, tol: f64, measured: f64, deg: bool) -> String {
    let k = if deg {
        Kind::Amount(if metric == "value" { "value" } else { "angle" }, "°")
    } else {
        kind(metric)
    };
    let d = |x: f64| if deg { x.to_degrees() } else { x };
    let (limit, tol, measured) = (d(limit), d(tol), d(measured));
    match k {
        Kind::Valid => {
            let found = if measured == 1.0 { "a valid one" } else { "an invalid one" };
            format!("expected a valid solid, found {found}")
        }
        Kind::Count(one, many) => {
            // a count is a whole number at least 0: "fewer than 1" and "at most 0" are none
            let want = match cmp {
                Cmp::Eq | Cmp::Le if limit == 0.0 => format!("no {many}"),
                Cmp::Lt if limit == 1.0 => format!("no {many}"),
                Cmp::Gt if limit == 0.0 => format!("at least 1 {one}"),
                Cmp::Eq => things(limit, one, many),
                Cmp::Lt => format!("fewer than {}", things(limit, one, many)),
                Cmp::Le => format!("at most {}", things(limit, one, many)),
                Cmp::Gt => format!("more than {}", things(limit, one, many)),
                Cmp::Ge => format!("at least {}", things(limit, one, many)),
            };
            let tol = if cmp == Cmp::Eq && tol > 0.0 {
                format!(" ± {}", fmt(tol))
            } else {
                String::new()
            };
            format!("expected {want}{tol}, found {}", fmt(measured))
        }
        Kind::Amount(what, unit) => {
            let lim = with_unit(limit, unit);
            let want = match cmp {
                Cmp::Eq if tol > 0.0 => format!("{what} of {lim} ± {}", with_unit(tol, unit)),
                Cmp::Eq => format!("{what} of {lim}"),
                Cmp::Lt => format!("{what} under {lim}"),
                Cmp::Le => format!("{what} at most {lim}"),
                Cmp::Gt => format!("{what} over {lim}"),
                Cmp::Ge => format!("{what} at least {lim}"),
            };
            format!("expected {want}, found {}", with_unit(measured, unit))
        }
    }
}

/// The comparison as the language writes it: `interferences = 0`, `volume = 11966 +- 12`.
pub fn raw(metric: &str, cmp: Cmp, limit: f64, tol: f64) -> String {
    raw_in(metric, cmp, limit, tol, false)
}

/// `raw`, an angle's numbers in degrees when `deg` (`value = -20deg`).
pub fn raw_in(metric: &str, cmp: Cmp, limit: f64, tol: f64, deg: bool) -> String {
    let n = |x: f64| if deg { format!("{}deg", fmt(x.to_degrees())) } else { fmt(x) };
    if kind(metric) == Kind::Valid {
        return "valid".into();
    }
    let op = match cmp {
        Cmp::Eq => "=",
        Cmp::Lt => "<",
        Cmp::Le => "<=",
        Cmp::Gt => ">",
        Cmp::Ge => ">=",
    };
    let tol = if cmp == Cmp::Eq && tol > 0.0 {
        format!(" +- {}", n(tol))
    } else {
        String::new()
    };
    format!("{metric} {op} {}{tol}", n(limit))
}
