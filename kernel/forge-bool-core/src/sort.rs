//! Merge sorts written out as loops (no closures), for Aeneas.

use crate::Sgn;

/// Lexicographic order on covectors.
pub(crate) fn cmp_cov(a: &[Sgn], b: &[Sgn]) -> std::cmp::Ordering {
    let mut i = 0;
    while i < a.len() && i < b.len() {
        if a[i] < b[i] {
            return std::cmp::Ordering::Less;
        }
        if b[i] < a[i] {
            return std::cmp::Ordering::Greater;
        }
        i += 1;
    }
    a.len().cmp(&b.len())
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
                let take_left = i < mid
                    && (j >= hi
                        || cmp_cov(&cells[src[i] as usize], &cells[src[j] as usize])
                            != std::cmp::Ordering::Greater);
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
