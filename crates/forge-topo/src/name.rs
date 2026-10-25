use core::fmt;

/// A stable name for a face, edge or vertex. It is derived only from how the
/// entity was made (which feature, which role, which inputs), never from
/// storage order, so rebuilding the same history gives the same names and a
/// later feature can keep referring to "the top face of extrude 3".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Name(pub u64);

impl Name {
    pub const NONE: Name = Name(0);

    /// A feature's own root name, from its id in the history.
    pub fn feature(id: u64) -> Name {
        Name::NONE.with(id).with_str("feature")
    }

    pub fn with(self, v: u64) -> Name {
        let mut h = self.0 ^ 0xcbf2_9ce4_8422_2325;
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        // final avalanche so nearby inputs give unrelated names
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
        h ^= h >> 33;
        Name(h)
    }

    pub fn with_str(self, s: &str) -> Name {
        s.bytes().fold(self.with(s.len() as u64), |n, b| n.with(b as u64))
    }

    pub fn with_name(self, other: Name) -> Name {
        self.with(other.0)
    }

    /// Order-independent combination, for entities defined by an unordered set
    /// of neighbours (an edge between two faces).
    pub fn with_set(self, mut names: Vec<Name>) -> Name {
        names.sort();
        names.into_iter().fold(self.with(0x5e7), |n, x| n.with_name(x))
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_distinct() {
        let f = Name::feature(3);
        assert_eq!(f, Name::feature(3));
        assert_ne!(f, Name::feature(4));
        assert_ne!(f.with_str("top"), f.with_str("bottom"));
        assert_eq!(
            Name::NONE.with_set(vec![f, f.with(1)]),
            Name::NONE.with_set(vec![f.with(1), f])
        );
    }
}
