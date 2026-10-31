use core::fmt;

/// A stable name for a face, edge or vertex. It is derived only from how the
/// entity was made (which feature, which role, which inputs), never from
/// storage order, so rebuilding the same history gives the same names and a
/// later feature can keep referring to "the top face of extrude 3".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Name(pub u64);

/// The bits of a name that name the whole it is a piece of (or itself, if it is no piece).
const PIECE_OF: u64 = 0xffff_ffff_0000_0000;
/// Marks a piece's half of its name.
const PIECE: u64 = 0x0005_9117;

impl Name {
    pub const NONE: Name = Name(0);

    /// A feature's own root name, from its id in the history.
    pub fn feature(id: u64) -> Name {
        Name::NONE.with(id).with_str("feature")
    }

    pub fn with(self, v: u64) -> Name {
        Name(mix((self.0 ^ 0xcbf2_9ce4_8422_2325)
            .wrapping_mul(0x0100_0000_01b3)
            ^ v))
    }

    /// `with` a word: its bytes hashed to one number first (callers pass constants).
    pub fn with_str(self, s: &str) -> Name {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for b in s.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
        self.with(h)
    }

    pub fn with_name(self, other: Name) -> Name {
        self.with(other.0)
    }

    /// A piece of the entity named `self` that a split made, told apart from its other pieces
    /// by `which`. The high half of the name is the whole's own, kept through any number of
    /// splits, so `is_piece_of` and `whole` can tell.
    pub fn piece(self, which: Name) -> Name {
        Name((self.0 & PIECE_OF) | (which.with(PIECE).0 & !PIECE_OF))
    }

    pub fn is_piece_of(self, whole: Name) -> bool {
        self != whole && self.whole() == whole.whole()
    }

    /// What a name and all its pieces share: whatever splits an entity went through.
    pub fn whole(self) -> Name {
        Name(self.0 & PIECE_OF)
    }

    /// The name moved off another body's by `salt`, keeping what it is a piece of: pieces of
    /// one whole stay pieces of one (moved) whole.
    pub fn salted(self, salt: Name) -> Name {
        Name((self.whole().with_name(salt).0 & PIECE_OF) | (self.0 & !PIECE_OF))
    }

    /// Order-independent combination, for entities defined by an unordered set
    /// of neighbours (an edge between two faces): a sum, so no sort and no allocation.
    pub fn with_set(self, names: &[Name]) -> Name {
        let sum = names.iter().fold(0u64, |a, n| a.wrapping_add(mix(n.0 ^ 0x5e7)));
        self.with(0x5e7).with(sum)
    }
}

/// A 64-bit finaliser (murmur3's): nearby inputs give unrelated outputs.
fn mix(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
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
            Name::NONE.with_set(&[f, f.with(1)]),
            Name::NONE.with_set(&[f.with(1), f])
        );
    }
}
