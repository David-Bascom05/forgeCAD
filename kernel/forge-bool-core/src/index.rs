//! Cells by key: the ledger's ids sorted by covector, so a covector's id is found by binary search.

use crate::sort::{cmp_cov, rank, sort_by_cov, sort_hashed};
use crate::{NONE, Sgn};

/// The ledger's cell ids in covector order, and (hash of covector, id) pairs sorted, for a lookup
/// that compares a covector only where its hash matches.
#[derive(Clone, Debug)]
pub struct Index {
    order: Vec<u32>,
    hs: Vec<(u64, u32)>,
}

/// FNV-1a over the signs' ranks.
pub(crate) fn hash_cov(c: &[Sgn]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < c.len() {
        h = (h ^ rank(c[i]) as u64).wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    h
}

/// Two covectors equal.
fn same(a: &[Sgn], b: &[Sgn]) -> bool {
    let mut same = a.len() == b.len();
    let mut i = 0;
    while same && i < a.len() {
        same = a[i] == b[i];
        i += 1;
    }
    same
}

/// (hash of covector, id) for every cell, ascending.
fn hash_table(cells: &[Vec<Sgn>]) -> Vec<(u64, u32)> {
    let mut hs = Vec::with_capacity(cells.len());
    let mut i = 0;
    while i < cells.len() {
        hs.push((hash_cov(&cells[i]), i as u32));
        i += 1;
    }
    sort_hashed(&mut hs);
    hs
}

impl Index {
    pub fn new(cells: &[Vec<Sgn>]) -> Index {
        let mut order = Vec::with_capacity(cells.len());
        let mut i = 0;
        while i < cells.len() {
            order.push(i as u32);
            i += 1;
        }
        sort_by_cov(&mut order, cells);
        let hs = hash_table(cells);
        Index { order, hs }
    }

    /// The id whose covector is `key`, or `u32::MAX`, by hash: the first of the pairs with
    /// `key`'s hash (binary search) whose covector is `key`.
    pub fn find_h(&self, cells: &[Vec<Sgn>], key: &[Sgn]) -> u32 {
        let h = hash_cov(key);
        // the first position whose hash is at least h
        let mut lo = 0;
        let mut hi = self.hs.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.hs[mid].0 < h {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let mut at = NONE;
        let mut p = lo;
        while at == NONE && p < self.hs.len() && self.hs[p].0 == h {
            let id = self.hs[p].1;
            if same(&cells[id as usize], key) {
                at = id;
            }
            p += 1;
        }
        at
    }

    /// The id whose covector is `key`, or `u32::MAX`.
    pub fn find(&self, cells: &[Vec<Sgn>], key: &[Sgn]) -> u32 {
        let mut lo = 0;
        let mut hi = self.order.len();
        let mut at = NONE;
        while at == NONE && lo < hi {
            let mid = lo + (hi - lo) / 2;
            let id = self.order[mid];
            let o = cmp_cov(&cells[id as usize], key);
            if o < 0 {
                lo = mid + 1;
            } else if o > 0 {
                hi = mid;
            } else {
                at = id;
            }
        }
        at
    }
}
