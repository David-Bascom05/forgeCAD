//! Merge sorts written out as loops (no closures), for Aeneas.

use crate::Sgn;

pub(crate) fn rank(s: Sgn) -> u8 {
    match s {
        Sgn::Neg => 0,
        Sgn::Zero => 1,
        Sgn::Pos => 2,
    }
}

/// Lexicographic order on covectors: -1, 0 or 1.
pub(crate) fn cmp_cov(a: &[Sgn], b: &[Sgn]) -> i8 {
    let mut o: i8 = 0;
    let mut i = 0;
    while o == 0 && i < a.len() && i < b.len() {
        let (x, y) = (rank(a[i]), rank(b[i]));
        if x < y {
            o = -1;
        } else if y < x {
            o = 1;
        }
        i += 1;
    }
    if o != 0 {
        o
    } else if a.len() < b.len() {
        -1
    } else if b.len() < a.len() {
        1
    } else {
        0
    }
}

/// Ascending, stable.
pub(crate) fn sort_u32(v: &mut Vec<u32>) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut src = v.clone();
    let mut dst = v.clone();
    let mut width = 1;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let mid = if lo + width < n { lo + width } else { n };
            let hi = if lo + 2 * width < n { lo + 2 * width } else { n };
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while k < hi {
                if i < mid && (j >= hi || src[i] <= src[j]) {
                    dst[k] = src[i];
                    i += 1;
                } else {
                    dst[k] = src[j];
                    j += 1;
                }
                k += 1;
            }
            lo = hi;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *v = src;
}

/// `a ≤ b` lexicographically, field by field.
fn le3(a: (u32, u32, u32), b: (u32, u32, u32)) -> bool {
    a.0 < b.0 || (a.0 == b.0 && (a.1 < b.1 || (a.1 == b.1 && a.2 <= b.2)))
}

/// Ascending lexicographically, stable.
pub(crate) fn sort_triples(v: &mut Vec<(u32, u32, u32)>) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut src = v.clone();
    let mut dst = v.clone();
    let mut width = 1;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let mid = if lo + width < n { lo + width } else { n };
            let hi = if lo + 2 * width < n { lo + 2 * width } else { n };
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while k < hi {
                if i < mid && (j >= hi || le3(src[i], src[j])) {
                    dst[k] = src[i];
                    i += 1;
                } else {
                    dst[k] = src[j];
                    j += 1;
                }
                k += 1;
            }
            lo = hi;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *v = src;
}

/// Ids ascending by their covectors (ties by id), stable.
pub(crate) fn sort_by_cov(v: &mut Vec<u32>, cells: &[Vec<Sgn>]) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut src = v.clone();
    let mut dst = v.clone();
    let mut width = 1;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let mid = if lo + width < n { lo + width } else { n };
            let hi = if lo + 2 * width < n { lo + 2 * width } else { n };
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while k < hi {
                let take_left =
                    i < mid && (j >= hi || cmp_cov(&cells[src[i] as usize], &cells[src[j] as usize]) != 1);
                if take_left {
                    dst[k] = src[i];
                    i += 1;
                } else {
                    dst[k] = src[j];
                    j += 1;
                }
                k += 1;
            }
            lo = hi;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *v = src;
}

/// `a ≤ b` on (hash, id), field by field.
fn le2(a: (u64, u32), b: (u64, u32)) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1)
}

/// Ascending on (hash, id), stable.
pub(crate) fn sort_hashed(v: &mut Vec<(u64, u32)>) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut src = v.clone();
    let mut dst = v.clone();
    let mut width = 1;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let mid = if lo + width < n { lo + width } else { n };
            let hi = if lo + 2 * width < n { lo + 2 * width } else { n };
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while k < hi {
                if i < mid && (j >= hi || le2(src[i], src[j])) {
                    dst[k] = src[i];
                    i += 1;
                } else {
                    dst[k] = src[j];
                    j += 1;
                }
                k += 1;
            }
            lo = hi;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *v = src;
}
