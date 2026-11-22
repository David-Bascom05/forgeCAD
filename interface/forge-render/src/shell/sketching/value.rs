//! A row's value as Forge reads it: a number, a param's name, or a Forge expression over them
//! (`w / 2`, `30deg`, `sqrt(a ^ 2 + b ^ 2)`), parsed by forge-lang's own parser. A param's value
//! is its text in the block, read the same way. Angles are kept in degrees here (`30deg` is 30, an
//! angle), so a dimension reads them as the rows write them; Forge's trig takes radians.

use forge_lang::syntax::{self, BinOp, Expr, ExprKind, StmtKind, Unit};
use forge_math::dmath;

/// A value: its number, and whether it is an angle (then in degrees).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Val {
    pub v: f64,
    pub angle: bool,
}

/// How deep params may read params: past this, one reads itself.
const DEPTH: usize = 16;

/// `text`'s value, its names read through `env` (a param's text, by its bare name).
pub fn eval(text: &str, env: &dyn Fn(&str) -> Option<String>) -> Result<Val, String> {
    eval_at(text, env, 0)
}

fn eval_at(text: &str, env: &dyn Fn(&str) -> Option<String>, depth: usize) -> Result<Val, String> {
    if depth > DEPTH {
        return Err(format!("'{text}' reads itself"));
    }
    // what most values are, a number or a param's bare name, read without the parser
    let bare = text.trim();
    if let Ok(v) = bare.parse::<f64>() {
        return Ok(plain(v));
    }
    let name = bare
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && bare.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if name && bare != "pi" && !syntax::KEYWORDS.contains(&bare) {
        let t = env(bare).ok_or_else(|| format!("'{bare}' is no param here"))?;
        let v = match t.parse::<f64>() {
            Ok(v) => plain(v),
            Err(_) => eval_at(&t, env, depth + 1)?,
        };
        if !v.v.is_finite() {
            return Err(format!("'{text}' is not a finite number"));
        }
        return Ok(v);
    }
    let prog =
        syntax::parse(&format!("value_ = ({text})\n")).map_err(|e| format!("'{text}': {}", e.text()))?;
    let e = match &prog[..] {
        [s] => match &s.kind {
            StmtKind::Assign { value, .. } => value,
            _ => return Err(format!("'{text}' is not a value")),
        },
        _ => return Err(format!("'{text}' is not a value")),
    };
    let v = Ev { text, env, depth }.expr(e)?;
    if !v.v.is_finite() {
        return Err(format!("'{text}' is not a finite number"));
    }
    Ok(v)
}

struct Ev<'a> {
    text: &'a str,
    env: &'a dyn Fn(&str) -> Option<String>,
    depth: usize,
}

fn plain(v: f64) -> Val {
    Val { v, angle: false }
}

fn angle(v: f64) -> Val {
    Val { v, angle: true }
}

impl Ev<'_> {
    fn not_value(&self) -> String {
        format!("'{}' is not a value the sketcher reads", self.text)
    }

    fn expr(&self, e: &Expr) -> Result<Val, String> {
        Ok(match &e.kind {
            ExprKind::Num(v, u) => Val {
                v: *v,
                angle: *u == Unit::Deg,
            },
            ExprKind::Name(n) if n == "pi" => plain(core::f64::consts::PI),
            ExprKind::Name(n) => {
                let t = (self.env)(n).ok_or_else(|| format!("'{n}' is no param here"))?;
                match t.parse::<f64>() {
                    Ok(v) => plain(v),
                    Err(_) => eval_at(&t, self.env, self.depth + 1)?,
                }
            }
            ExprKind::Neg(x) => {
                let a = self.expr(x)?;
                Val { v: -a.v, ..a }
            }
            ExprKind::Bin(op, x, y) => {
                let (a, b) = (self.expr(x)?, self.expr(y)?);
                match op {
                    BinOp::Add => Val {
                        v: a.v + b.v,
                        angle: a.angle || b.angle,
                    },
                    BinOp::Sub => Val {
                        v: a.v - b.v,
                        angle: a.angle || b.angle,
                    },
                    BinOp::Mul => Val {
                        v: a.v * b.v,
                        angle: a.angle != b.angle,
                    },
                    BinOp::Div => Val {
                        v: a.v / b.v,
                        angle: a.angle && !b.angle,
                    },
                    // Forge's: a whole power
                    BinOp::Pow if b.v.fract() == 0.0 && !b.angle => Val {
                        v: a.v.powi(b.v as i32),
                        angle: false,
                    },
                    BinOp::Pow => return Err(format!("'{}': ^ takes a whole power", self.text)),
                    _ => return Err(self.not_value()),
                }
            }
            ExprKind::Call(f, args) => {
                let ExprKind::Name(f) = &f.kind else {
                    return Err(self.not_value());
                };
                let xs = args
                    .iter()
                    .map(|a| self.expr(&a.value))
                    .collect::<Result<Vec<Val>, String>>()?;
                // Forge's trig takes an angle (a bare number is radians) and its inverses give one
                let rad = |a: Val| {
                    if a.angle {
                        a.v * core::f64::consts::PI / 180.0
                    } else {
                        a.v
                    }
                };
                let deg = |r: f64| angle(r * 180.0 / core::f64::consts::PI);
                match (f.as_str(), &xs[..]) {
                    ("sqrt", [a]) if a.v >= 0.0 => plain(a.v.sqrt()),
                    ("abs", [a]) => Val { v: a.v.abs(), ..*a },
                    ("sin", [a]) => plain(dmath::sin(rad(*a))),
                    ("cos", [a]) => plain(dmath::cos(rad(*a))),
                    ("tan", [a]) => {
                        let (s, c) = dmath::sin_cos(rad(*a));
                        plain(s / c)
                    }
                    ("asin", [a]) => deg(dmath::asin(a.v)),
                    ("acos", [a]) => deg(dmath::acos(a.v)),
                    ("atan", [a]) => deg(dmath::atan(a.v)),
                    ("atan2", [y, x]) => deg(dmath::atan2(y.v, x.v)),
                    ("min", [a, b]) => Val {
                        v: a.v.min(b.v),
                        angle: a.angle || b.angle,
                    },
                    ("max", [a, b]) => Val {
                        v: a.v.max(b.v),
                        angle: a.angle || b.angle,
                    },
                    _ => return Err(self.not_value()),
                }
            }
            _ => return Err(self.not_value()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        match name {
            "w" => Some("30".into()),
            "half" => Some("w / 2".into()),
            "a" => Some("30deg".into()),
            "me" => Some("me + 1".into()),
            _ => None,
        }
    }

    #[test]
    fn forge_text_reads_as_forge_reads_it() {
        assert_eq!(eval("20", &env), Ok(plain(20.0)));
        assert_eq!(eval("w / 2 + 1", &env), Ok(plain(16.0)));
        assert_eq!(eval("half * 2", &env), Ok(plain(30.0)));
        assert_eq!(eval("10mm", &env), Ok(plain(10.0)));
        assert_eq!(eval("a / 2", &env), Ok(angle(15.0)));
        assert_eq!(eval("-(2 ^ 3)", &env), Ok(plain(-8.0)));
        assert!((eval("sin(30deg)", &env).unwrap().v - 0.5).abs() < 1e-15);
        // a bare number in Forge's trig is radians
        assert!((eval("cos(pi)", &env).unwrap().v + 1.0).abs() < 1e-15);
        assert!((eval("atan2(1, 1)", &env).unwrap().v - 45.0).abs() < 1e-12);
        assert_eq!(eval("max(w, 40)", &env), Ok(plain(40.0)));
        assert!(eval("nope + 1", &env).unwrap_err().contains("no param"));
        assert!(eval("me", &env).unwrap_err().contains("reads itself"));
        assert!(eval("1in", &env).is_err());
        assert!(eval("w / 0", &env).unwrap_err().contains("finite"));
    }
}
