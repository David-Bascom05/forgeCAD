//! A union–find whose root is always the least member: the Lean's `head` (the first member of a
//! class in list order) when the members are list positions.

pub(crate) struct Uf {
    parent: Vec<u32>,
}

impl Uf {
    pub(crate) fn new(n: usize) -> Uf {
        let mut parent = Vec::with_capacity(n);
        let mut i = 0;
        while i < n {
            parent.push(i as u32);
            i += 1;
        }
        Uf { parent }
    }

    /// The least member of `x`'s class (path halving).
    pub(crate) fn find(&mut self, x: usize) -> usize {
        let mut x = x;
        while self.parent[x] as usize != x {
            let p = self.parent[x] as usize;
            let g = self.parent[p];
            self.parent[x] = g;
            x = g as usize;
        }
        x
    }

    /// Join two classes, the greater root under the lesser.
    pub(crate) fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra < rb {
            self.parent[rb] = ra as u32;
        } else if rb < ra {
            self.parent[ra] = rb as u32;
        }
    }
}
