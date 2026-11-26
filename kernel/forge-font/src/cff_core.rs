//! A Type 2 charstring run to contours, in integers: every operand is a whole number or a 16.16
//! fixed-point one, so every coordinate is a whole number of 2^-16 units, held here as an i64.
//! The shell (`cff::Cff::outline`) maps each to f64 (`/ 65536`, exact).
//!
//! Written for Aeneas (research/aeneas/STYLE.md): no recursion (a subroutine call pushes a frame),
//! no closure that captures, no slice pattern, no `break` or `return` inside a loop (an error is
//! a code set once, that stops the loop). verify/aeneas/forge-font proves of the translation that
//! it never fails and that every contour it returns is closed.

// `%` over `is_multiple_of` and `div_ceil` spelled out: Aeneas models the operators, not those
#![allow(clippy::manual_is_multiple_of)]

use crate::FontError;

/// What one outline may run: operators and operands (subroutine calls included) in all.
pub const STEPS: usize = 1 << 16;
/// Subroutine calls nested this deep are refused (the format's own limit).
pub const NESTING: usize = 10;
/// The operand stack's depth (Type 2's limit is 48; CFF2 allows 513 and is not read).
pub const STACK: usize = 48;

/// A point in 2^-16 font units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pc {
    pub x: i64,
    pub y: i64,
}

/// A piece of a contour, in 2^-16 font units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegC {
    Line(Pc, Pc),
    Cubic(Pc, Pc, Pc, Pc),
}

/// Why a charstring is refused (`fail` gives the error).
pub type Code = u8;
const OK: Code = 0;
const TRUNCATED: Code = 1;
const TOO_LONG: Code = 2;
const OVERFLOW: Code = 3;
const NO_MOVETO: Code = 4;
const MOVETO_OPERANDS: Code = 5;
const RLINETO: Code = 6;
const SHORT: Code = 7;
const RRCURVETO: Code = 8;
const RCURVELINE: Code = 9;
const RLINECURVE: Code = 10;
const VVCURVETO: Code = 11;
const HHCURVETO: Code = 12;
const VHCURVETO: Code = 13;
const NO_NUMBER: Code = 14;
const NOT_WHOLE: Code = 15;
const NOT_THERE: Code = 16;
const NESTED: Code = 17;
const SEAC: Code = 18;
const FLEX: Code = 19;
const DEPRECATED: Code = 20;
const UNKNOWN: Code = 21;
const NO_END: Code = 22;

/// The error a refusal code stands for.
pub fn fail(c: Code) -> FontError {
    let u = FontError::Unsupported;
    match c {
        TRUNCATED => FontError::Truncated,
        TOO_LONG => u("a charstring that runs too long"),
        OVERFLOW => u("a charstring's stack overflows"),
        NO_MOVETO => u("a charstring drawing before its first moveto"),
        MOVETO_OPERANDS => u("a charstring moveto of the wrong operands"),
        RLINETO => u("rlineto of odd operands"),
        SHORT => u("a charstring operator short of operands"),
        RRCURVETO => u("rrcurveto of operands not in sixes"),
        RCURVELINE => u("rcurveline of the wrong operands"),
        RLINECURVE => u("rlinecurve of the wrong operands"),
        VVCURVETO => u("vvcurveto of the wrong operands"),
        HHCURVETO => u("hhcurveto of the wrong operands"),
        VHCURVETO => u("vhcurveto or hvcurveto of the wrong operands"),
        NO_NUMBER => u("a subroutine call with no number"),
        NOT_WHOLE => u("a subroutine number that is not whole"),
        NOT_THERE => u("a call to a subroutine not there"),
        NESTED => u("charstring subroutines nested too deep"),
        SEAC => u("an accented glyph built by endchar (seac)"),
        FLEX => u("a flex of the wrong operands"),
        DEPRECATED => u("a deprecated or unknown charstring operator"),
        UNKNOWN => u("an unknown charstring operator"),
        _ => u("a charstring that does not end"),
    }
}

/// The machine's drawing state.
pub struct Pen {
    pub at: Pc,
    /// The open contour's first point.
    pub start: Option<Pc>,
    pub contour: Vec<SegC>,
    pub out: Vec<Vec<SegC>>,
}

/// The operand stack, and how many of its first entries are a width read past.
pub struct Stack {
    pub v: Vec<i64>,
    pub lo: usize,
}

const ONE: i64 = 65536;

fn byte_or0(d: &[u8], o: usize) -> u8 {
    if o < d.len() { d[o] } else { 0 }
}

/// Closes the open contour (a line back to its start if the pen is not there) and keeps it.
pub fn close(p: &mut Pen) {
    if let Some(s) = p.start {
        p.start = None;
        if p.at != s {
            p.contour.push(SegC::Line(p.at, s));
        }
        if !p.contour.is_empty() {
            let c = std::mem::take(&mut p.contour);
            p.out.push(c);
        }
    }
}

pub fn move_to(p: &mut Pen, dx: i64, dy: i64) {
    close(p);
    p.at = Pc {
        x: p.at.x + dx,
        y: p.at.y + dy,
    };
    p.start = Some(p.at);
}

pub fn line(p: &mut Pen, dx: i64, dy: i64) -> Code {
    let a = p.at;
    p.at = Pc {
        x: a.x + dx,
        y: a.y + dy,
    };
    if p.start.is_none() {
        return NO_MOVETO;
    }
    if a != p.at {
        p.contour.push(SegC::Line(a, p.at));
    }
    OK
}

#[allow(clippy::too_many_arguments)]
pub fn curve(p: &mut Pen, d0: i64, d1: i64, d2: i64, d3: i64, d4: i64, d5: i64) -> Code {
    let a = p.at;
    let c1 = Pc {
        x: a.x + d0,
        y: a.y + d1,
    };
    let c2 = Pc {
        x: c1.x + d2,
        y: c1.y + d3,
    };
    let b = Pc {
        x: c2.x + d4,
        y: c2.y + d5,
    };
    p.at = b;
    if p.start.is_none() {
        return NO_MOVETO;
    }
    if !(a == c1 && c1 == c2 && c2 == b) {
        p.contour.push(SegC::Cubic(a, c1, c2, b));
    }
    OK
}

/// The width, if the stack holds one operand more than `want` (or an odd one for stems), is
/// read past.
fn width(s: &mut Stack, read: &mut bool, odd: bool, want: usize) {
    if !*read {
        *read = true;
        // (two ifs, not a bool from one: Aeneas types an `if` of two comparisons as a Prop)
        if odd {
            if s.v.len() % 2 == 1 {
                s.lo = 1;
            }
        } else if s.v.len() > want {
            s.lo = 1;
        }
    }
}

/// Operand `k` after the width.
fn arg(s: &Stack, k: usize) -> i64 {
    s.v[s.lo + k]
}

/// rlineto (5), hlineto (6), vlineto (7).
fn lines(p: &mut Pen, s: &Stack, op: u8) -> Code {
    let n = s.v.len() - s.lo;
    let mut e = OK;
    if op == 5 {
        if n == 0 || n % 2 != 0 {
            e = RLINETO;
        }
        let mut k = 0;
        while e == OK && k + 1 < n {
            e = line(p, arg(s, k), arg(s, k + 1));
            k += 2;
        }
    } else {
        if n < 1 {
            e = SHORT;
        }
        let mut h = op == 6;
        let mut k = 0;
        while e == OK && k < n {
            let v = arg(s, k);
            e = if h { line(p, v, 0) } else { line(p, 0, v) };
            h = !h;
            k += 1;
        }
    }
    e
}

/// The curves of `n6` sixes of operands from `k`.
fn sixes(p: &mut Pen, s: &Stack, k0: usize, n6: usize) -> Code {
    let mut e = OK;
    let mut j = 0;
    while e == OK && j < n6 {
        let k = k0 + 6 * j;
        e = curve(
            p,
            arg(s, k),
            arg(s, k + 1),
            arg(s, k + 2),
            arg(s, k + 3),
            arg(s, k + 4),
            arg(s, k + 5),
        );
        j += 1;
    }
    e
}

/// rrcurveto (8), rcurveline (24), rlinecurve (25).
fn curves(p: &mut Pen, s: &Stack, op: u8) -> Code {
    let n = s.v.len() - s.lo;
    if op == 8 {
        if n == 0 || n % 6 != 0 {
            return RRCURVETO;
        }
        sixes(p, s, 0, n / 6)
    } else if op == 24 {
        if n < 8 || (n - 2) % 6 != 0 {
            return RCURVELINE;
        }
        let e = sixes(p, s, 0, (n - 2) / 6);
        if e != OK {
            return e;
        }
        line(p, arg(s, n - 2), arg(s, n - 1))
    } else {
        if n < 8 || (n - 6) % 2 != 0 {
            return RLINECURVE;
        }
        let mut e = OK;
        let mut k = 0;
        while e == OK && k + 6 < n {
            e = line(p, arg(s, k), arg(s, k + 1));
            k += 2;
        }
        if e != OK {
            return e;
        }
        sixes(p, s, n - 6, 1)
    }
}

/// vvcurveto (26), hhcurveto (27): an optional first delta, then fours.
fn fours(p: &mut Pen, s: &Stack, op: u8) -> Code {
    let n = s.v.len() - s.lo;
    let (mut d1, k0) = if n % 4 == 1 { (arg(s, 0), 1) } else { (0, 0) };
    if n - k0 == 0 || (n - k0) % 4 != 0 {
        return if op == 26 { VVCURVETO } else { HHCURVETO };
    }
    let mut e = OK;
    let mut k = k0;
    while e == OK && k + 3 < n {
        let (a, b, c, d) = (arg(s, k), arg(s, k + 1), arg(s, k + 2), arg(s, k + 3));
        e = if op == 26 {
            curve(p, d1, a, b, c, 0, d)
        } else {
            curve(p, a, d1, b, c, d, 0)
        };
        d1 = 0;
        k += 4;
    }
    e
}

/// vhcurveto (30), hvcurveto (31): alternating, the last with an optional final delta.
fn alternating(p: &mut Pen, s: &Stack, op: u8) -> Code {
    let n = s.v.len() - s.lo;
    if n < 4 || !(n % 4 == 0 || n % 4 == 1) {
        return VHCURVETO;
    }
    let mut h = op == 31;
    let m = n / 4;
    let mut e = OK;
    let mut k = 0;
    while e == OK && k < m {
        let (a, b, c, d) = (
            arg(s, 4 * k),
            arg(s, 4 * k + 1),
            arg(s, 4 * k + 2),
            arg(s, 4 * k + 3),
        );
        let last = if k + 1 == m && n % 4 == 1 {
            arg(s, n - 1)
        } else {
            0
        };
        e = if h {
            curve(p, a, 0, b, c, last, d)
        } else {
            curve(p, 0, a, b, c, d, last)
        };
        h = !h;
        k += 1;
    }
    e
}

fn abs(v: i64) -> i64 {
    if v < 0 { -v } else { v }
}

/// The flexes (12 34 hflex, 35 flex, 36 hflex1, 37 flex1), each of its own number of operands.
fn flex(p: &mut Pen, s: &Stack, b1: u8) -> Code {
    let n = s.v.len() - s.lo;
    if b1 == 35 {
        if n == 13 { sixes(p, s, 0, 2) } else { FLEX }
    } else if b1 == 34 {
        if n == 7 { hflex(p, s) } else { FLEX }
    } else if b1 == 36 {
        if n == 9 { hflex1(p, s) } else { FLEX }
    } else if b1 == 37 {
        if n == 11 { flex1(p, s) } else { FLEX }
    } else {
        DEPRECATED
    }
}

fn hflex(p: &mut Pen, s: &Stack) -> Code {
    let (dx1, dx2, dy2, dx3, dx4, dx5, dx6) = (
        arg(s, 0),
        arg(s, 1),
        arg(s, 2),
        arg(s, 3),
        arg(s, 4),
        arg(s, 5),
        arg(s, 6),
    );
    let e = curve(p, dx1, 0, dx2, dy2, dx3, 0);
    if e != OK {
        return e;
    }
    curve(p, dx4, 0, dx5, -dy2, dx6, 0)
}

fn hflex1(p: &mut Pen, s: &Stack) -> Code {
    let (dx1, dy1, dx2, dy2, dx3) = (arg(s, 0), arg(s, 1), arg(s, 2), arg(s, 3), arg(s, 4));
    let (dx4, dx5, dy5, dx6) = (arg(s, 5), arg(s, 6), arg(s, 7), arg(s, 8));
    let e = curve(p, dx1, dy1, dx2, dy2, dx3, 0);
    if e != OK {
        return e;
    }
    curve(p, dx4, 0, dx5, dy5, dx6, -(dy1 + dy2 + dy5))
}

/// The last point's other delta returns to the start's line.
fn flex1(p: &mut Pen, s: &Stack) -> Code {
    let dx = arg(s, 0) + arg(s, 2) + arg(s, 4) + arg(s, 6) + arg(s, 8);
    let dy = arg(s, 1) + arg(s, 3) + arg(s, 5) + arg(s, 7) + arg(s, 9);
    let (ex, ey) = if abs(dx) > abs(dy) {
        (arg(s, 10), -dy)
    } else {
        (-dx, arg(s, 10))
    };
    let e = curve(
        p,
        arg(s, 0),
        arg(s, 1),
        arg(s, 2),
        arg(s, 3),
        arg(s, 4),
        arg(s, 5),
    );
    if e != OK {
        return e;
    }
    curve(p, arg(s, 6), arg(s, 7), arg(s, 8), arg(s, 9), ex, ey)
}

/// The subroutine number bias for an INDEX of `n` objects.
fn bias(n: usize) -> i64 {
    if n < 1240 {
        107
    } else if n < 33900 {
        1131
    } else {
        32768
    }
}

/// Subroutine `k` (as the charstring numbers it) of the INDEX with offsets `offs` (count + 1,
/// absolute in `d`): its range in `d`, if it is there.
fn subr(d: &[u8], offs: &[usize], k: i64) -> Option<(usize, usize)> {
    let n = offs.len();
    let count = if n == 0 { 0 } else { n - 1 };
    let i = k + bias(count);
    if i < 0 || i as u64 >= count as u64 {
        return None;
    }
    let i = i as usize;
    let (a, b) = (offs[i], offs[i + 1]);
    if a <= b && b <= d.len() {
        Some((a, b))
    } else {
        None
    }
}

/// Where the interpreter is: the charstring it runs (a range of `d`) and its next byte.
#[derive(Clone, Copy)]
pub struct Frame {
    pub s: usize,
    pub e: usize,
    pub o: usize,
}

/// An operand at `o` (first byte `b0`) of the charstring ending at `e`: its value (2^-16 units)
/// and where the next byte is, or None if it runs past the end.
fn operand(d: &[u8], o: usize, e: usize, b0: u8) -> Option<(i64, usize)> {
    if b0 == 28 {
        if o + 1 < e {
            let w = ((byte_or0(d, o) as u16) << 8 | byte_or0(d, o + 1) as u16) as i16 as i64;
            Some((w * ONE, o + 2))
        } else {
            None
        }
    } else if b0 <= 246 {
        Some(((b0 as i64 - 139) * ONE, o))
    } else if b0 <= 250 {
        if o < e {
            Some((
                ((b0 as i64 - 247) * 256 + byte_or0(d, o) as i64 + 108) * ONE,
                o + 1,
            ))
        } else {
            None
        }
    } else if b0 <= 254 {
        if o < e {
            Some((
                (-(b0 as i64 - 251) * 256 - byte_or0(d, o) as i64 - 108) * ONE,
                o + 1,
            ))
        } else {
            None
        }
    } else if o + 3 < e {
        let w = (byte_or0(d, o) as u32) << 24
            | (byte_or0(d, o + 1) as u32) << 16
            | (byte_or0(d, o + 2) as u32) << 8
            | byte_or0(d, o + 3) as u32;
        Some((w as i32 as i64, o + 4))
    } else {
        None
    }
}

/// Runs the charstring `d[cs.0..cs.1]` (its subroutines in the INDEXes with offsets `gsubrs` and
/// `local`) to its closed contours.
pub fn run(
    d: &[u8],
    cs: (usize, usize),
    gsubrs: &[usize],
    local: &[usize],
) -> Result<Vec<Vec<SegC>>, FontError> {
    let mut p = Pen {
        at: Pc { x: 0, y: 0 },
        start: None,
        contour: Vec::new(),
        out: Vec::new(),
    };
    let mut s = Stack {
        v: Vec::with_capacity(STACK),
        lo: 0,
    };
    let mut f = Frame {
        s: cs.0,
        e: if cs.1 <= d.len() { cs.1 } else { d.len() },
        o: cs.0,
    };
    let mut frames: Vec<Frame> = Vec::with_capacity(NESTING);
    let mut m = Machine {
        stems: 0,
        width_read: false,
        steps: STEPS,
        done: false,
        left: false,
    };
    let mut err = OK;
    while err == OK && !m.done && !m.left && (f.o < f.e || !frames.is_empty()) {
        if f.o >= f.e {
            // a subroutine run off its end returns
            f = frames.pop().unwrap_or(f);
        } else {
            err = one(d, gsubrs, local, &mut f, &mut frames, &mut p, &mut s, &mut m);
        }
    }
    if err != OK {
        return Err(fail(err));
    }
    if !m.done {
        return Err(fail(NO_END));
    }
    close(&mut p);
    Ok(p.out)
}

/// The machine's counters and flags.
pub struct Machine {
    pub stems: usize,
    pub width_read: bool,
    pub steps: usize,
    pub done: bool,
    /// The top charstring's `return`: it stops there, unended.
    pub left: bool,
}

/// One step: an operand, a call, a return or an operator, at `f.o < f.e`.
#[allow(clippy::too_many_arguments)]
fn one(
    d: &[u8],
    gsubrs: &[usize],
    local: &[usize],
    f: &mut Frame,
    frames: &mut Vec<Frame>,
    p: &mut Pen,
    s: &mut Stack,
    m: &mut Machine,
) -> Code {
    if m.steps == 0 {
        return TOO_LONG;
    }
    m.steps -= 1;
    let b0 = byte_or0(d, f.o);
    f.o += 1;
    if b0 >= 32 || b0 == 28 {
        match operand(d, f.o, f.e, b0) {
            Some((v, o)) => {
                f.o = o;
                if s.v.len() >= STACK {
                    OVERFLOW
                } else {
                    s.v.push(v);
                    OK
                }
            }
            None => TRUNCATED,
        }
    } else if b0 == 10 || b0 == 29 {
        // callsubr callgsubr
        match s.v.pop() {
            None => NO_NUMBER,
            Some(k) => {
                if k % ONE != 0 {
                    NOT_WHOLE
                } else {
                    let idx = if b0 == 10 { local } else { gsubrs };
                    match subr(d, idx, k / ONE) {
                        None => NOT_THERE,
                        Some((a, b)) => {
                            if frames.len() + 1 > NESTING {
                                NESTED
                            } else {
                                frames.push(*f);
                                *f = Frame { s: a, e: b, o: a };
                                OK
                            }
                        }
                    }
                }
            }
        }
    } else if b0 == 11 {
        // return
        match frames.pop() {
            Some(g) => *f = g,
            None => m.left = true,
        }
        OK
    } else {
        let e = op(d, f, p, s, &mut m.stems, &mut m.width_read, &mut m.done, b0);
        s.v.clear();
        s.lo = 0;
        e
    }
}

/// One operator other than an operand, a call or a return.
#[allow(clippy::too_many_arguments)]
fn op(
    d: &[u8],
    f: &mut Frame,
    p: &mut Pen,
    s: &mut Stack,
    stems: &mut usize,
    width_read: &mut bool,
    done: &mut bool,
    b0: u8,
) -> Code {
    if b0 == 1 || b0 == 3 || b0 == 18 || b0 == 23 {
        // hstem vstem hstemhm vstemhm
        width(s, width_read, true, 0);
        *stems += (s.v.len() - s.lo) / 2;
        OK
    } else if b0 == 19 || b0 == 20 {
        // hintmask cntrmask: an implied vstem first, then the mask's bytes
        width(s, width_read, true, 0);
        *stems += (s.v.len() - s.lo) / 2;
        let skip = stems.div_ceil(8);
        if skip > f.e - f.o {
            f.o = f.e;
            TRUNCATED
        } else {
            f.o += skip;
            OK
        }
    } else if b0 == 21 || b0 == 22 || b0 == 4 {
        // rmoveto hmoveto vmoveto
        let want = if b0 == 21 { 2 } else { 1 };
        width(s, width_read, false, want);
        if s.v.len() - s.lo != want {
            MOVETO_OPERANDS
        } else {
            if b0 == 21 {
                move_to(p, arg(s, 0), arg(s, 1));
            } else if b0 == 22 {
                move_to(p, arg(s, 0), 0);
            } else {
                move_to(p, 0, arg(s, 0));
            }
            OK
        }
    } else if b0 == 5 || b0 == 6 || b0 == 7 {
        lines(p, s, b0)
    } else if b0 == 8 || b0 == 24 || b0 == 25 {
        curves(p, s, b0)
    } else if b0 == 26 || b0 == 27 {
        fours(p, s, b0)
    } else if b0 == 30 || b0 == 31 {
        alternating(p, s, b0)
    } else if b0 == 14 {
        // endchar
        width(s, width_read, true, 0);
        if s.v.len() - s.lo >= 4 {
            SEAC
        } else {
            close(p);
            *done = true;
            OK
        }
    } else if b0 == 12 {
        if f.o < f.e {
            let b1 = byte_or0(d, f.o);
            f.o += 1;
            flex(p, s, b1)
        } else {
            TRUNCATED
        }
    } else {
        UNKNOWN
    }
}
