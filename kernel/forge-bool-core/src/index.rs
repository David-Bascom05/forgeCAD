//! Cells by key: the ledger's ids sorted by covector, so a covector's id is found by binary search.

use crate::sort::{cmp_cov, sort_by_cov};
use crate::{NONE, Sgn};

/// The ledger's cell ids in covector order.
#[derive(Clone, Debug)]
pub struct Index {
    order: Vec<u32>,
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
        Index { order }
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
