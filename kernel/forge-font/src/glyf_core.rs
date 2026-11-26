//! A simple TrueType glyph's contours, in integers: the points as the glyf table holds them, and
//! the walk that makes them lines and quadratics. Coordinates are on the doubled grid (twice the
//! font unit), so an implied on-curve point (midway between two off ones) is a grid point too.
//!
//! Written for Aeneas (research/aeneas/STYLE.md: index loops, no closure that captures, no
//! iterator adaptor, no `break` or `return` inside a loop), so the translation is this code and
//! verify/forge-text's specs are proven of it. The float shell (`Font::outline_into`) maps what
//! this returns through a composite's matrix.

use crate::FontError;

/// A point on the doubled grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pi {
    pub x: i64,
    pub y: i64,
}

/// A piece of a contour on the doubled grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegI {
    Line(Pi, Pi),
    /// From the first point to the last, pulled towards the control point between.
    Quad(Pi, Pi, Pi),
}

/// A simple glyph as its record holds it: where each contour ends, each point's on-curve flag,
/// its coordinates (font units).
pub struct Simple {
    pub ends: Vec<usize>,
    pub on: Vec<bool>,
    pub xs: Vec<i32>,
    pub ys: Vec<i32>,
}

/// The byte at `o`, 0 past the end (the caller checks `o < d.len()`).
fn byte_or0(d: &[u8], o: usize) -> u8 {
    if o < d.len() { d[o] } else { 0 }
}

/// The big-endian word at `o`, 0 past the end (the caller checks `o + 1 < d.len()`).
fn word_or0(d: &[u8], o: usize) -> u16 {
    ((byte_or0(d, o) as u16) << 8) | byte_or0(d, o + 1) as u16
}

fn word(d: &[u8], o: usize) -> Result<u16, FontError> {
    if o + 1 < d.len() {
        Ok(word_or0(d, o))
    } else {
        Err(FontError::Truncated)
    }
}

/// One axis's coordinates from offset `o`: each a delta from the one before, as the flags say
/// (`short`: a byte, its sign by `same`; else, unless `same`, a word). Returns them and where
/// the data after them starts.
fn coords(d: &[u8], o: usize, flags: &[u8], short: u8, same: u8) -> Result<(Vec<i32>, usize), FontError> {
    let mut out = Vec::with_capacity(flags.len());
    // summed wide, kept as i32 wraps it (as the deltas of a corrupt glyph may): never an overflow
    let mut v: i64 = 0;
    let mut o = o;
    let mut k = 0;
    // (no early return inside a loop: a read past the end stops it and is the error after)
    let mut bad = false;
    while !bad && k < flags.len() {
        let f = flags[k];
        if f & short != 0 {
            if o < d.len() {
                let dv = byte_or0(d, o) as i64;
                o += 1;
                if f & same != 0 { v += dv } else { v -= dv }
            } else {
                bad = true;
            }
        } else if f & same == 0 {
            if o + 1 < d.len() {
                v += word_or0(d, o) as i16 as i64;
                o += 2;
            } else {
                bad = true;
            }
        }
        if !bad {
            out.push(v as i32);
        }
        k += 1;
    }
    if bad {
        return Err(FontError::Truncated);
    }
    Ok((out, o))
}

/// A simple glyph's record from `a` (its number of contours `n`, read already), within `d`.
/// `budget` is what the points may spend.
pub fn read_simple(d: &[u8], a: usize, n: usize, budget: usize) -> Result<Simple, FontError> {
    // the contours' ends must be there (the instructions' length is read after the budget)
    if a + 10 + 2 * n > d.len() {
        return Err(FontError::Truncated);
    }
    let mut ends = Vec::with_capacity(n);
    let mut k = 0;
    while k < n {
        ends.push(word_or0(d, a + 10 + 2 * k) as usize);
        k += 1;
    }
    let points = if n == 0 { 0 } else { ends[n - 1] + 1 };
    if points + 1 > budget {
        return Err(FontError::Unsupported("a glyph too large"));
    }
    let ins = word(d, a + 10 + 2 * n)? as usize;
    let mut o = a + 12 + 2 * n + ins;
    // one loop (Aeneas: no early return from a nested loop): a flag, then as many repeats of
    // it as its repeat byte says, the last flag still to repeat carried along
    let mut flags: Vec<u8> = Vec::with_capacity(points);
    let mut last: u8 = 0;
    let mut repeats: u8 = 0;
    let mut bad = false;
    while !bad && flags.len() < points {
        if repeats > 0 {
            flags.push(last);
            repeats -= 1;
        } else if o >= d.len() {
            bad = true;
        } else {
            let f = byte_or0(d, o);
            o += 1;
            flags.push(f);
            if f & 8 != 0 {
                if o < d.len() {
                    repeats = byte_or0(d, o);
                    o += 1;
                } else {
                    bad = true;
                }
            }
            last = f;
        }
    }
    if bad {
        return Err(FontError::Truncated);
    }
    let (xs, o) = coords(d, o, &flags, 2, 16)?;
    let (ys, _) = coords(d, o, &flags, 4, 32)?;
    let mut on = Vec::with_capacity(points);
    let mut i = 0;
    while i < points {
        on.push(flags[i] & 1 != 0);
        i += 1;
    }
    Ok(Simple { ends, on, xs, ys })
}

/// Point `k` of the glyph on the doubled grid.
fn at(g: &Simple, k: usize) -> Pi {
    Pi {
        x: 2 * g.xs[k] as i64,
        y: 2 * g.ys[k] as i64,
    }
}

fn mid(a: Pi, b: Pi) -> Pi {
    Pi {
        x: (a.x + b.x) / 2,
        y: (a.y + b.y) / 2,
    }
}

/// The walk's state: where the pen is, the pending control point, the pieces so far.
struct Walk {
    at_: Pi,
    ctrl: Option<Pi>,
    out: Vec<SegI>,
}

/// One point of the walk (verify/forge-text's `step`).
fn step(w: Walk, p: Pi, on: bool) -> Walk {
    let Walk { at_, ctrl, mut out } = w;
    match (on, ctrl) {
        (true, None) => {
            if p != at_ {
                out.push(SegI::Line(at_, p));
            }
            Walk {
                at_: p,
                ctrl: None,
                out,
            }
        }
        (true, Some(c)) => {
            if p != at_ {
                out.push(SegI::Quad(at_, c, p));
            }
            Walk {
                at_: p,
                ctrl: None,
                out,
            }
        }
        (false, None) => Walk {
            at_,
            ctrl: Some(p),
            out,
        },
        (false, Some(c)) => {
            let m = mid(c, p);
            if m != at_ {
                out.push(SegI::Quad(at_, c, m));
            }
            Walk {
                at_: m,
                ctrl: Some(p),
                out,
            }
        }
    }
}

/// Points `first..=last` as a closed contour: an off point between two others is a quadratic's
/// control, two off points in a row have an on point implied midway; the walk starts at the
/// first on point (or midway between the first two, if none is on) and ends back there.
pub fn contour(g: &Simple, first: usize, last: usize) -> Vec<SegI> {
    let n = last + 1 - first;
    if n < 2 {
        return Vec::new();
    }
    // the first on point, if any
    let mut k = 0;
    let mut found = false;
    while !found && k < n {
        if g.on[first + k] {
            found = true;
        } else {
            k += 1;
        }
    }
    let (start, from) = if found {
        (at(g, first + k), k + 1)
    } else {
        (mid(at(g, first), at(g, first + 1)), 1)
    };
    let mut w = Walk {
        at_: start,
        ctrl: None,
        out: Vec::new(),
    };
    // the points after the start, round to it, then the start again (on the curve)
    let mut i = 0;
    while i < n {
        let j = first + (from + i) % n;
        let after_last = found && i + 1 == n;
        if !after_last {
            w = step(w, at(g, j), g.on[j]);
        }
        i += 1;
    }
    w = step(w, start, true);
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(pts: &[(i32, i32, bool)]) -> Simple {
        Simple {
            ends: vec![pts.len() - 1],
            on: pts.iter().map(|p| p.2).collect(),
            xs: pts.iter().map(|p| p.0).collect(),
            ys: pts.iter().map(|p| p.1).collect(),
        }
    }

    #[test]
    fn walks() {
        let p = |x, y| Pi { x, y };
        // four off points: four quadratics through the implied midpoints (doubled grid)
        let c = contour(
            &g(&[(0, -10, false), (10, 0, false), (0, 10, false), (-10, 0, false)]),
            0,
            3,
        );
        assert_eq!(c.len(), 4);
        assert_eq!(c[0], SegI::Quad(p(10, -10), p(20, 0), p(10, 10)));
        let c = contour(&g(&[(0, 0, false), (10, 0, true), (10, 10, true)]), 0, 2);
        assert_eq!(
            c,
            vec![
                SegI::Line(p(20, 0), p(20, 20)),
                SegI::Quad(p(20, 20), p(0, 0), p(20, 0))
            ]
        );
    }
}
