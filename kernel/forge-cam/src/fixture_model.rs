//! A workholding model's faces for the checker, compiled once from its STEP (the workholding
//! builder's `compile`) and kept as text: the checker's faces of each of its bodies (`group`s:
//! the base, the fixed jaw, the moving jaw ...) in the device's own frame, every number as f64's
//! shortest round-trip text, so reading gives back the very faces written.
//!
//! ```text
//! forge-workholding check 1
//! group NAME
//! box x0 y0 z0 x1 y1 z1       (its faces' boxes' hull: what the checker sees)
//! tight x0 y0 z0 x1 y1 z1     (its own box, mesh grown by the chord: the body, for drawing)
//! P ox oy oz ux uy uz vx vy vz p0 p1 q0 q1 mu [L a0 a1 b0 b1 | A a0 a1 b0 b1 c0 c1 r ccw]...
//! C|K|S|T ox oy oz xx xy xz yx yy yz ax ay az PARAMS.. p0 p1 q0 q1 mu
//! X ox oy oz ux uy uz vx vy vz mu
//! ```
//!
//! (C: r; K: r0 k; S: r; T: big r cm sm.) An `X` line is a triangle of a meshed face: a plane
//! over p, q in [0, 1] trimmed to (0,0), (1,0), (0,1).
//!
//! `moved` poses a face: a turn that permutes the axes and changes their signs (exact in f64),
//! then a shift (its rounding is added to the face's mu).
use forge_cam_geom::check::{Face, Surf, TSeg, Trim, V3};
use forge_cam_geom::ival::Ival;

pub const HEADER: &str = "forge-workholding check 1";

fn num(out: &mut String, x: f64) {
    use std::fmt::Write;
    let _ = write!(out, " {x:?}");
}
fn nums(out: &mut String, xs: &[f64]) {
    for x in xs {
        num(out, *x);
    }
}

/// One face as a line (no newline).
pub fn face_line(f: &Face) -> String {
    let mut s = String::new();
    if let Surf::Plane { o, u, v } = f.surf
        && is_unit_triangle(f)
    {
        s.push('X');
        nums(&mut s, &o);
        nums(&mut s, &u);
        nums(&mut s, &v);
        num(&mut s, f.mu);
        return s;
    }
    let tail = |s: &mut String| nums(s, &[f.p.lo, f.p.hi, f.q.lo, f.q.hi, f.mu]);
    match f.surf {
        Surf::Plane { o, u, v } => {
            s.push('P');
            nums(&mut s, &o);
            nums(&mut s, &u);
            nums(&mut s, &v);
            tail(&mut s);
            if let Some(t) = &f.trim {
                for g in &t.segs {
                    match *g {
                        TSeg::Line { a, b } => {
                            s.push_str(" L");
                            nums(&mut s, &[a[0], a[1], b[0], b[1]]);
                        }
                        TSeg::Arc { a, b, c, r, ccw } => {
                            s.push_str(" A");
                            nums(&mut s, &[a[0], a[1], b[0], b[1], c[0], c[1], r]);
                            s.push_str(if ccw { " 1" } else { " 0" });
                        }
                    }
                }
            }
        }
        Surf::Cyl { o, x, y, ax, r } => {
            s.push('C');
            nums(&mut s, &o);
            nums(&mut s, &x);
            nums(&mut s, &y);
            nums(&mut s, &ax);
            num(&mut s, r);
            tail(&mut s);
        }
        Surf::Cone { o, x, y, ax, r0, k } => {
            s.push('K');
            nums(&mut s, &o);
            nums(&mut s, &x);
            nums(&mut s, &y);
            nums(&mut s, &ax);
            nums(&mut s, &[r0, k]);
            tail(&mut s);
        }
        Surf::Sphere { o, x, y, ax, r } => {
            s.push('S');
            nums(&mut s, &o);
            nums(&mut s, &x);
            nums(&mut s, &y);
            nums(&mut s, &ax);
            num(&mut s, r);
            tail(&mut s);
        }
        Surf::Torus {
            o,
            x,
            y,
            ax,
            big,
            r,
            cm,
            sm,
        } => {
            s.push('T');
            nums(&mut s, &o);
            nums(&mut s, &x);
            nums(&mut s, &y);
            nums(&mut s, &ax);
            nums(&mut s, &[big, r, cm, sm]);
            tail(&mut s);
        }
    }
    s
}

/// A face's every number as its bits, with its kind and id: what the checker reads of it, so two
/// faces with the same bytes are the same face to the checker (a memo's key, without formatting
/// a float).
pub fn face_bits(f: &Face, out: &mut Vec<u8>) {
    let mut put = |xs: &[f64]| {
        for x in xs {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
    };
    match f.surf {
        Surf::Plane { o, u, v } => {
            put(&[0.0]);
            put(&o);
            put(&u);
            put(&v);
        }
        Surf::Cyl { o, x, y, ax, r } => {
            put(&[1.0, r]);
            put(&o);
            put(&x);
            put(&y);
            put(&ax);
        }
        Surf::Cone { o, x, y, ax, r0, k } => {
            put(&[2.0, r0, k]);
            put(&o);
            put(&x);
            put(&y);
            put(&ax);
        }
        Surf::Sphere { o, x, y, ax, r } => {
            put(&[3.0, r]);
            put(&o);
            put(&x);
            put(&y);
            put(&ax);
        }
        Surf::Torus {
            o,
            x,
            y,
            ax,
            big,
            r,
            cm,
            sm,
        } => {
            put(&[4.0, big, r, cm, sm]);
            put(&o);
            put(&x);
            put(&y);
            put(&ax);
        }
    }
    put(&[f.p.lo, f.p.hi, f.q.lo, f.q.hi, f.mu, f.id as f64]);
    match &f.trim {
        None => put(&[-1.0]),
        Some(t) => {
            put(&[t.segs.len() as f64]);
            for g in &t.segs {
                match *g {
                    TSeg::Line { a, b } => put(&[0.0, a[0], a[1], b[0], b[1]]),
                    TSeg::Arc { a, b, c, r, ccw } => put(&[
                        1.0,
                        a[0],
                        a[1],
                        b[0],
                        b[1],
                        c[0],
                        c[1],
                        r,
                        if ccw { 1.0 } else { 0.0 },
                    ]),
                }
            }
        }
    }
}

/// The compiled text of a model's groups, each with its solids' own box (`tight`).
pub fn write(groups: &[(String, Vec<Face>, [V3; 2])]) -> String {
    let mut s = String::from(HEADER);
    s.push('\n');
    for (n, fs, tight) in groups {
        s.push_str("group ");
        s.push_str(n);
        s.push('\n');
        // the faces' box (forge-workholding reads it without the faces)
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for f in fs {
            for k in 0..3 {
                lo[k] = lo[k].min(f.bbox[k].lo);
                hi[k] = hi[k].max(f.bbox[k].hi);
            }
        }
        s.push_str("box");
        nums(&mut s, &lo);
        nums(&mut s, &hi);
        s.push('\n');
        s.push_str("tight");
        nums(&mut s, &tight[0]);
        nums(&mut s, &tight[1]);
        s.push('\n');
        for f in fs {
            s.push_str(&face_line(f));
            s.push('\n');
        }
    }
    s
}

fn unit_trim() -> Trim {
    Trim {
        segs: vec![
            TSeg::Line {
                a: [0.0, 0.0],
                b: [1.0, 0.0],
            },
            TSeg::Line {
                a: [1.0, 0.0],
                b: [0.0, 1.0],
            },
            TSeg::Line {
                a: [0.0, 1.0],
                b: [0.0, 0.0],
            },
        ],
    }
}

/// Whether a planar face is `triangle`'s form: (p, q) over [0, 1]² trimmed to the unit triangle.
fn is_unit_triangle(f: &Face) -> bool {
    f.p == Ival::new(0.0, 1.0) && f.q == Ival::new(0.0, 1.0) && f.trim.as_ref() == Some(&unit_trim())
}

/// A triangle (corners counter-clockwise seen from outside) as a planar face of mu `mu`.
pub fn triangle(a: V3, b: V3, c: V3, mu: f64, id: u32) -> Face {
    let d = |p: V3, q: V3| [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
    Face::new(
        Surf::Plane {
            o: a,
            u: d(b, a),
            v: d(c, a),
        },
        Ival::new(0.0, 1.0),
        Ival::new(0.0, 1.0),
        Some(unit_trim()),
        mu,
        id,
    )
}

struct Toks<'a> {
    toks: Vec<&'a str>,
    k: usize,
    line: &'a str,
}

impl Toks<'_> {
    fn more(&self) -> bool {
        self.k < self.toks.len()
    }
    fn word(&mut self) -> Result<&str, String> {
        let t = self
            .toks
            .get(self.k)
            .ok_or_else(|| format!("a short face line: {}", self.line))?;
        self.k += 1;
        Ok(t)
    }
    fn f(&mut self) -> Result<f64, String> {
        let t = self.word()?;
        t.parse::<f64>().map_err(|e| format!("{t}: {e}"))
    }
    fn v3(&mut self) -> Result<V3, String> {
        Ok([self.f()?, self.f()?, self.f()?])
    }
    fn v2(&mut self) -> Result<[f64; 2], String> {
        Ok([self.f()?, self.f()?])
    }
}

fn parse_line(line: &str, id: u32) -> Result<Face, String> {
    let mut it = line.split_ascii_whitespace();
    let kind = it.next().ok_or("an empty face line")?;
    let mut t = Toks {
        toks: it.collect(),
        k: 0,
        line,
    };
    if kind == "X" {
        let (o, u, v, mu) = (t.v3()?, t.v3()?, t.v3()?, t.f()?);
        return Ok(Face::new(
            Surf::Plane { o, u, v },
            Ival::new(0.0, 1.0),
            Ival::new(0.0, 1.0),
            Some(unit_trim()),
            mu,
            id,
        ));
    }
    let surf = match kind {
        "P" => Surf::Plane {
            o: t.v3()?,
            u: t.v3()?,
            v: t.v3()?,
        },
        "C" | "K" | "S" | "T" => {
            let (o, x, y, ax) = (t.v3()?, t.v3()?, t.v3()?, t.v3()?);
            match kind {
                "C" => Surf::Cyl {
                    o,
                    x,
                    y,
                    ax,
                    r: t.f()?,
                },
                "K" => Surf::Cone {
                    o,
                    x,
                    y,
                    ax,
                    r0: t.f()?,
                    k: t.f()?,
                },
                "S" => Surf::Sphere {
                    o,
                    x,
                    y,
                    ax,
                    r: t.f()?,
                },
                _ => Surf::Torus {
                    o,
                    x,
                    y,
                    ax,
                    big: t.f()?,
                    r: t.f()?,
                    cm: t.f()?,
                    sm: t.f()?,
                },
            }
        }
        _ => return Err(format!("not a face line: {line}")),
    };
    let (p0, p1, q0, q1, mu) = (t.f()?, t.f()?, t.f()?, t.f()?, t.f()?);
    let mut segs = Vec::new();
    while t.more() {
        match t.word()? {
            "L" => segs.push(TSeg::Line {
                a: t.v2()?,
                b: t.v2()?,
            }),
            "A" => segs.push(TSeg::Arc {
                a: t.v2()?,
                b: t.v2()?,
                c: t.v2()?,
                r: t.f()?,
                ccw: t.f()? != 0.0,
            }),
            w => return Err(format!("a trim piece `{w}`: {line}")),
        }
    }
    let trim = if segs.is_empty() {
        None
    } else {
        Some(Trim { segs })
    };
    Ok(Face::new(
        surf,
        Ival::new(p0, p1),
        Ival::new(q0, q1),
        trim,
        mu,
        id,
    ))
}

/// A compiled model's groups (faces numbered from 0 in each).
pub fn read(text: &str) -> Result<Vec<(String, Vec<Face>)>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err(format!("not a compiled workholding model (`{HEADER}`)"));
    }
    let mut out: Vec<(String, Vec<Face>)> = Vec::new();
    for line in lines {
        if let Some(n) = line.strip_prefix("group ") {
            out.push((n.to_string(), Vec::new()));
        } else if line.starts_with("box ") || line.starts_with("tight ") {
            continue;
        } else if !line.trim().is_empty() {
            let g = out.last_mut().ok_or("a face before any group")?;
            let id = g.1.len() as u32;
            g.1.push(parse_line(line, id)?);
        }
    }
    Ok(out)
}

/// A proper rotation that only permutes axes and changes their signs: `R[i]` = (the axis of the
/// input row i reads, its sign). Applying one is exact in f64.
pub type Rot = [(usize, f64); 3];

/// No turn.
pub const IDENT: Rot = [(0, 1.0), (1, 1.0), (2, 1.0)];
/// A quarter turn about z: (x, y, z) ↦ (−y, x, z).
pub const QUARTER_Z: Rot = [(1, -1.0), (0, 1.0), (2, 1.0)];

/// Whether `r` is a proper rotation of that kind (each axis once, signs ±1, determinant +1).
pub fn is_rot(r: &Rot) -> bool {
    let mut seen = [false; 3];
    for &(a, s) in r {
        if a > 2 || seen[a] || s.abs() != 1.0 {
            return false;
        }
        seen[a] = true;
    }
    // the permutation's sign times the signs' product
    let perm = [r[0].0, r[1].0, r[2].0];
    let inv = (perm[0] > perm[1]) as u8 + (perm[0] > perm[2]) as u8 + (perm[1] > perm[2]) as u8;
    let psign = if inv.is_multiple_of(2) { 1.0 } else { -1.0 };
    psign * r[0].1 * r[1].1 * r[2].1 == 1.0
}

/// The face turned by `rot` (exact), then shifted by `t`; its mu grows by the shift's rounding.
pub fn moved(f: &Face, rot: &Rot, t: V3, id: u32) -> Face {
    debug_assert!(is_rot(rot));
    let r = |v: V3| {
        [
            rot[0].1 * v[rot[0].0],
            rot[1].1 * v[rot[1].0],
            rot[2].1 * v[rot[2].0],
        ]
    };
    let mut big = t.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    let mut pt = |v: V3| {
        let w = r(v);
        big = big.max(w.iter().fold(0.0f64, |m, x| m.max(x.abs())));
        [w[0] + t[0], w[1] + t[1], w[2] + t[2]]
    };
    let surf = match f.surf {
        Surf::Plane { o, u, v } => Surf::Plane {
            o: pt(o),
            u: r(u),
            v: r(v),
        },
        Surf::Cyl { o, x, y, ax, r: rr } => Surf::Cyl {
            o: pt(o),
            x: r(x),
            y: r(y),
            ax: r(ax),
            r: rr,
        },
        Surf::Cone { o, x, y, ax, r0, k } => Surf::Cone {
            o: pt(o),
            x: r(x),
            y: r(y),
            ax: r(ax),
            r0,
            k,
        },
        Surf::Sphere { o, x, y, ax, r: rr } => Surf::Sphere {
            o: pt(o),
            x: r(x),
            y: r(y),
            ax: r(ax),
            r: rr,
        },
        Surf::Torus {
            o,
            x,
            y,
            ax,
            big: bg,
            r: rr,
            cm,
            sm,
        } => Surf::Torus {
            o: pt(o),
            x: r(x),
            y: r(y),
            ax: r(ax),
            big: bg,
            r: rr,
            cm,
            sm,
        },
    };
    // each coordinate of o + t is rounded once: at most half an ulp of the larger
    let mu = f.mu + 2.0 * f64::EPSILON * big;
    Face::new(surf, f.p, f.q, f.trim.clone(), mu, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_read_back_as_written_and_move_exactly() {
        let mut fs = vec![triangle(
            [0.0, 0.0, 0.0],
            [1.0, 0.1, 0.0],
            [0.0, 1.0, 0.3],
            0.02,
            0,
        )];
        fs.push(Face::new(
            Surf::Cyl {
                o: [1.0 / 3.0, 2.0, -0.5],
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                ax: [0.0, 0.0, 1.0],
                r: 4.25,
            },
            Ival::new(-0.4, 0.4),
            Ival::new(0.0, 7.0),
            None,
            1e-6,
            1,
        ));
        fs.push(Face::new(
            Surf::Plane {
                o: [0.0; 3],
                u: [1.0, 0.0, 0.0],
                v: [0.0, 1.0, 0.0],
            },
            Ival::new(0.0, 2.0),
            Ival::new(0.0, 2.0),
            Some(Trim {
                segs: vec![
                    TSeg::Arc {
                        a: [2.0, 0.0],
                        b: [0.0, 2.0],
                        c: [0.0, 0.0],
                        r: 2.0,
                        ccw: true,
                    },
                    TSeg::Line {
                        a: [0.0, 2.0],
                        b: [0.0, 0.0],
                    },
                    TSeg::Line {
                        a: [0.0, 0.0],
                        b: [2.0, 0.0],
                    },
                ],
            }),
            1e-6,
            2,
        ));
        let text = write(&[("base".into(), fs.clone(), [[0.0; 3], [1.0; 3]])]);
        let back = read(&text).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].0, "base");
        assert_eq!(back[0].1, fs);
        // a quarter turn: x to y, y to −x; the box follows
        assert!(is_rot(&IDENT) && is_rot(&QUARTER_Z) && !is_rot(&[(1, 1.0), (0, 1.0), (2, 1.0)]));
        let m = moved(&fs[1], &QUARTER_Z, [10.0, 0.0, 0.0], 1);
        let Surf::Cyl { o, x, ax, .. } = m.surf else {
            panic!()
        };
        assert_eq!(
            (o, x, ax),
            ([8.0, 1.0 / 3.0, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0])
        );
        assert!(m.mu > fs[1].mu && m.mu < fs[1].mu + 1e-13);
        assert!(read("nope").is_err());
    }
}
