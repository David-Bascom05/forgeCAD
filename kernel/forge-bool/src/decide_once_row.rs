//! The bridge's packed covector (the core's ledger holds sign lists: the rows are unpacked there).
//! A covector packed in two masks: bit `i` of `pos` is `Pos` at class `i`, of `neg` is `Neg`, of
//! neither is `Zero` (`pos & neg == 0`, no bits at or past the ledger's width). Every bit
//! operation is here, in small loop-free functions, so the rest of the core reads and writes signs
//! only through them (`decode` in the Lean).

use forge_bool_core::Sgn;

/// A packed covector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Row {
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
