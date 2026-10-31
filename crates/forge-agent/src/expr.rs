//! Expressions over named parameters: `+ - * / ^`, parentheses, `sqrt sin cos tan min max
//! abs`, `pi`. Trig takes degrees. Evaluated straight from the text, only through IEEE
//! arithmetic and `forge_math::dmath`, so a value is the same on every target.

use forge_math::dmath;
use forge_model::Unit;

pub type Env<'a> = dyn Fn(&str) -> Option<(f64, Unit)> + 'a;

const FUNCS: [&str; 7] = ["sqrt", "sin", "cos", "tan", "min", "max", "abs"];

/// Words a parameter may not be named: functions, constants and units.
pub fn reserved(name: &str) -> bool {
    FUNCS.contains(&name) || ["pi", "mm", "deg"].contains(&name)
}

/// A name a parameter could have.
pub fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// The value of `src` and its unit.
pub fn eval(src: &str, env: &Env) -> Result<(f64, Unit), String> {
    let mut p = Ev {
        s: src.as_bytes(),
        k: 0,
        env,
    };
    let v = p.sum()?;
    p.ws();
    if p.k < p.s.len() {
        return Err(format!("unexpected '{}' in '{src}'", &src[p.k..]));
    }
    if !v.0.is_finite() {
        return Err(format!("'{src}' is not a finite number"));
    }
    Ok(v)
}

/// Each name `src` uses (functions and `pi` left out).
pub fn names(src: &str) -> Vec<&str> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut k = 0;
    while k < b.len() {
        if b[k].is_ascii_alphabetic() || b[k] == b'_' {
            let s = k;
            while k < b.len() && (b[k].is_ascii_alphanumeric() || b[k] == b'_') {
                k += 1;
            }
            let w = &src[s..k];
            if !reserved(w) {
                out.push(w);
            }
        } else if b[k].is_ascii_digit() || b[k] == b'.' {
            // a number, its exponent included
            while k < b.len() && (b[k].is_ascii_alphanumeric() || b[k] == b'.') {
                if (b[k] == b'e' || b[k] == b'E') && k + 1 < b.len() && (b[k + 1] == b'-' || b[k + 1] == b'+')
                {
                    k += 1;
                }
                k += 1;
            }
        } else {
            k += 1;
        }
    }
    out
}

/// Units combine: none takes the other's; mm and deg do not mix.
fn join(a: Unit, b: Unit) -> Result<Unit, String> {
    match (a, b) {
        (Unit::None, u) | (u, Unit::None) => Ok(u),
        (a, b) if a == b => Ok(a),
        _ => Err("mixes mm and deg".into()),
    }
}

struct Ev<'a, 'e> {
    s: &'a [u8],
    k: usize,
    env: &'a Env<'e>,
}

type V = (f64, Unit);

impl Ev<'_, '_> {
    fn ws(&mut self) {
        while self.k < self.s.len() && self.s[self.k].is_ascii_whitespace() {
            self.k += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        let hit = self.s.get(self.k) == Some(&c);
        self.k += hit as usize;
        hit
    }
    fn sum(&mut self) -> Result<V, String> {
        let mut a = self.product()?;
        loop {
            let sign = if self.eat(b'+') {
                1.0
            } else if self.eat(b'-') {
                -1.0
            } else {
                return Ok(a);
            };
            let b = self.product()?;
            a = (a.0 + sign * b.0, join(a.1, b.1)?);
        }
    }
    fn product(&mut self) -> Result<V, String> {
        let mut a = self.unary()?;
        loop {
            if self.eat(b'*') {
                let b = self.unary()?;
                a = (a.0 * b.0, join(a.1, b.1)?);
            } else if self.eat(b'/') {
                let b = self.unary()?;
                if b.0 == 0.0 {
                    return Err("division by zero".into());
                }
                a = (a.0 / b.0, join(a.1, b.1)?);
            } else {
                return Ok(a);
            }
        }
    }
    fn unary(&mut self) -> Result<V, String> {
        if self.eat(b'-') {
            let (v, u) = self.unary()?;
            return Ok((-v, u));
        }
        if self.eat(b'+') {
            return self.unary();
        }
        let base = self.atom()?;
        if !self.eat(b'^') {
            return Ok(base);
        }
        let (e, eu) = self.unary()?;
        if eu != Unit::None {
            return Err("an exponent has no unit".into());
        }
        Ok((pow(base.0, e)?, base.1))
    }
    fn atom(&mut self) -> Result<V, String> {
        self.ws();
        let s = self.k;
        let c = *self.s.get(s).ok_or("an expression ends too soon")?;
        if c == b'(' {
            self.k += 1;
            let v = self.sum()?;
            if !self.eat(b')') {
                return Err("a '(' is not closed".into());
            }
            return Ok(v);
        }
        if c.is_ascii_digit() || c == b'.' {
            while self.k < self.s.len() {
                let d = self.s[self.k];
                let exp_sign = (d == b'-' || d == b'+') && matches!(self.s[self.k - 1], b'e' | b'E');
                if !(d.is_ascii_digit() || d == b'.' || d == b'e' || d == b'E' || exp_sign) {
                    break;
                }
                self.k += 1;
            }
            let t = std::str::from_utf8(&self.s[s..self.k]).expect("ascii");
            return t
                .parse::<f64>()
                .map(|v| (v, Unit::None))
                .map_err(|_| format!("'{t}' is not a number"));
        }
        if !(c.is_ascii_alphabetic() || c == b'_') {
            return Err(format!("unexpected '{}'", c as char));
        }
        while self.k < self.s.len() && (self.s[self.k].is_ascii_alphanumeric() || self.s[self.k] == b'_') {
            self.k += 1;
        }
        let name = std::str::from_utf8(&self.s[s..self.k]).expect("ascii");
        if name == "pi" {
            return Ok((dmath::PI, Unit::None));
        }
        if FUNCS.contains(&name) {
            if !self.eat(b'(') {
                return Err(format!("{name} needs parentheses: {name}(x)"));
            }
            let mut args = vec![self.sum()?];
            while self.eat(b',') {
                args.push(self.sum()?);
            }
            if !self.eat(b')') {
                return Err(format!("{name}( is not closed"));
            }
            return call(name, &args);
        }
        (self.env)(name).ok_or_else(|| format!("unknown parameter '{name}'"))
    }
}

fn call(name: &str, args: &[V]) -> Result<V, String> {
    let want = if matches!(name, "min" | "max") { 2 } else { 1 };
    if args.len() != want {
        return Err(format!(
            "{name} takes {want} argument{}",
            if want > 1 { "s" } else { "" }
        ));
    }
    let (x, u) = args[0];
    let trig = |f: fn(f64) -> f64| {
        if u == Unit::Mm {
            return Err(format!("{name} takes an angle, not mm"));
        }
        Ok((f(x / 180.0 * dmath::PI), Unit::None))
    };
    match name {
        "sqrt" if x < 0.0 => Err("sqrt of a negative number".into()),
        "sqrt" => Ok((x.sqrt(), u)),
        "abs" => Ok((x.abs(), u)),
        "sin" => trig(dmath::sin),
        "cos" => trig(dmath::cos),
        "tan" => trig(|a| {
            let (s, c) = dmath::sin_cos(a);
            s / c
        }),
        "min" => Ok((x.min(args[1].0), join(u, args[1].1)?)),
        _ => Ok((x.max(args[1].0), join(u, args[1].1)?)),
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

    fn env(n: &str) -> Option<(f64, Unit)> {
        match n {
            "wall" => Some((2.0, Unit::Mm)),
            "a" => Some((30.0, Unit::Deg)),
            "n" => Some((4.0, Unit::None)),
            _ => None,
        }
    }

    fn ev(s: &str) -> Result<(f64, Unit), String> {
        eval(s, &env)
    }

    #[test]
    fn arithmetic() {
        assert_eq!(ev("wall*3 + 1"), Ok((7.0, Unit::Mm)));
        assert_eq!(ev("-2^2"), Ok((-4.0, Unit::None)));
        assert_eq!(ev("2^3^2"), Ok((512.0, Unit::None)));
        assert_eq!(ev("(1+2)*3 - 4/2"), Ok((7.0, Unit::None)));
        assert_eq!(ev("2^-2"), Ok((0.25, Unit::None)));
        assert_eq!(ev("9^0.5 + sqrt(16)"), Ok((7.0, Unit::None)));
        assert_eq!(ev("min(wall, 5) + max(1, n) + abs(-1)"), Ok((7.0, Unit::Mm)));
        assert_eq!(ev("1e-3*1e3 + 1.5E+1"), Ok((16.0, Unit::None)));
        assert_eq!(ev("2*pi").map(|v| v.0), Ok(dmath::TAU));
        assert_eq!(ev("sin(a)"), Ok((dmath::sin(dmath::PI / 6.0), Unit::None)));
        assert_eq!(ev("cos(90)").map(|v| v.0.abs() < 1e-16), Ok(true));
        assert!((ev("tan(45)").expect("tan").0 - 1.0).abs() < 1e-15);
        assert_eq!(ev("a*2"), Ok((60.0, Unit::Deg)));
    }

    #[test]
    fn errors_say_what() {
        let err = |s: &str| ev(s).expect_err(s);
        assert_eq!(err("wal + 1"), "unknown parameter 'wal'");
        assert_eq!(err("wall + a"), "mixes mm and deg");
        assert_eq!(err("sin(wall)"), "sin takes an angle, not mm");
        assert_eq!(err("(1 + 2"), "a '(' is not closed");
        assert_eq!(err("1/0"), "division by zero");
        assert_eq!(err("2^1.5"), "^ takes a whole exponent (or 0.5), not 1.5");
        assert_eq!(err("min(1)"), "min takes 2 arguments");
        assert_eq!(err("sqrt 2"), "sqrt needs parentheses: sqrt(x)");
        assert_eq!(err("1 2"), "unexpected '2' in '1 2'");
        assert_eq!(err("1e308*10"), "'1e308*10' is not a finite number");
        assert_eq!(err(""), "an expression ends too soon");
    }

    #[test]
    fn names_used() {
        assert_eq!(
            names("wall*3 + sqrt(d1) - pi + 1e-3 + x_2"),
            ["wall", "d1", "x_2"]
        );
    }
}
