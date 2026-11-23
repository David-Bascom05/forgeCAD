//! Cells by key: the ledger's ids sorted by covector, so a covector's id is found by binary search.

use crate::NONE;
use crate::row::{Row, row_cmp, row_eq, row_hash};
use crate::sort::{sort_by_cov, sort_hashed};

/// The ledger's cell ids in covector order, and (hash of covector, id) pairs sorted, for a lookup
/// that compares a covector only where its hash matches.
#[derive(Clone, Debug)]
pub struct Index {
    order: Vec<u32>,
    hs: Vec<(u64, u32)>,
}

/// (hash of covector, id) for every cell, ascending.
fn hash_table(cells: &[Row]) -> Vec<(u64, u32)> {
    let mut hs = Vec::with_capacity(cells.len());
    let mut i = 0;
    while i < cells.len() {
        hs.push((row_hash(cells[i]), i as u32));
        i += 1;
    }
    sort_hashed(&mut hs);
    hs
}

impl Index {
    pub fn new(cells: &[Row]) -> Index {
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
    pub fn find_h(&self, cells: &[Row], key: Row) -> u32 {
        let h = row_hash(key);
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
            if row_eq(cells[id as usize], key) {
                at = id;
            }
            p += 1;
        }
        at
    }

    /// The id whose covector is `key`, or `u32::MAX`.
    pub fn find(&self, cells: &[Row], key: Row) -> u32 {
        let mut lo = 0;
        let mut hi = self.order.len();
        let mut at = NONE;
        while at == NONE && lo < hi {
            let mid = lo + (hi - lo) / 2;
            let id = self.order[mid];
            let o = row_cmp(cells[id as usize], key);
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
