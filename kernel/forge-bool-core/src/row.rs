//! A covector packed in two masks: bit `i` of `pos` is `Pos` at class `i`, of `neg` is `Neg`, of
//! neither is `Zero` (`pos & neg == 0`, no bits at or past the ledger's width). Every bit
//! operation is here, in small loop-free functions, so the rest of the core reads and writes signs
//! only through them (`decode` in the Lean).

use crate::Sgn;

/// A packed covector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub pos: u64,
    pub neg: u64,
}

/// The widest covector a row holds.
pub const MAX_WIDTH: u32 = 64;

/// Bit `i`, or none past 63.
pub fn bit(i: usize) -> u64 {
    if i < 64 { 1u64 << i } else { 0 }
}

/// The sign at class `i` (`Zero` past the width).
pub fn row_get(r: Row, i: usize) -> Sgn {
    let b = bit(i);
    if r.pos & b != 0 {
        Sgn::Pos
    } else if r.neg & b != 0 {
        Sgn::Neg
    } else {
        Sgn::Zero
    }
}

/// `r` with sign `s` at class `i` (unchanged past 63).
pub fn row_set(r: Row, i: usize, s: Sgn) -> Row {
    let b = bit(i);
    let p = r.pos & !b;
    let n = r.neg & !b;
    match s {
        Sgn::Pos => Row { pos: p | b, neg: n },
        Sgn::Neg => Row { pos: p, neg: n | b },
        Sgn::Zero => Row { pos: p, neg: n },
    }
}

/// The same covector.
pub fn row_eq(a: Row, b: Row) -> bool {
    a.pos == b.pos && a.neg == b.neg
}

/// A total order consistent with equality: (pos, neg) lexicographically. -1, 0 or 1.
pub fn row_cmp(a: Row, b: Row) -> i8 {
    if a.pos < b.pos {
        -1
    } else if a.pos > b.pos {
        1
    } else if a.neg < b.neg {
        -1
    } else if a.neg > b.neg {
        1
    } else {
        0
    }
}

/// `Cov.le`: `a` is a face of `b` (every nonzero sign of `a` is `b`'s there).
pub fn row_le(a: Row, b: Row) -> bool {
    (a.pos & !b.pos) == 0 && (a.neg & !b.neg) == 0
}

/// The classes `0 .. w` as a mask.
pub fn width_mask(w: usize) -> u64 {
    if w >= 64 { !0u64 } else { (1u64 << w) - 1 }
}

/// The zeros of `r` among the classes `0 .. w`, as a mask.
pub fn row_zero_mask(r: Row, w: usize) -> u64 {
    !(r.pos | r.neg) & width_mask(w)
}

/// A hash of the covector (FNV-1a steps over the two masks).
pub fn row_hash(r: Row) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    h = (h ^ r.pos).wrapping_mul(0x0000_0100_0000_01b3);
    h = (h ^ r.neg).wrapping_mul(0x0000_0100_0000_01b3);
    h ^ (h >> 29)
}

/// The row of a sign list (at most 64 signs are read).
pub fn row_of(x: &[Sgn]) -> Row {
    let mut r = Row::default();
    let mut i = 0;
    while i < x.len() && i < 64 {
        r = row_set(r, i, x[i]);
        i += 1;
    }
    r
}

/// The sign list of a row of width `w`.
pub fn row_signs(r: Row, w: usize) -> Vec<Sgn> {
    let mut v = Vec::with_capacity(w);
    let mut i = 0;
    while i < w {
        v.push(row_get(r, i));
        i += 1;
    }
    v
}
