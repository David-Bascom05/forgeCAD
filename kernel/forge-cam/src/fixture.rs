//! Workholding: a vise placed on the stock, as boxes the certified checker holds every body of
//! the tool away from by the fixture clearance (G2: for every s, g(s)·(T ∪ N) ∩ (F ⊕ B_c) = ∅).
//!
//! The vise closes along the clamp axis on the stock's two faces across it. Its jaws stand
//! `clamp` above the stock's seat (so they grip that much of the stock); under the stock, two
//! parallels of `parallel` width run inside the jaws, and the vise bed lies `jaw_height` below
//! the jaw top. Every piece is a box: the fixture is their union, and the distance to a union is
//! the least distance to its pieces, so checking each piece's faces checks the fixture.
use crate::ops::Stock;
use forge_cam_geom::check::Face;

/// A placed fixture: its named boxes, in the part's frame.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub name: String,
    pub pieces: Vec<(String, Stock)>,
    /// The jaws' top: stock below it is held, and no cut may reach it.
    pub jaw_top: f64,
    /// 0 for x, 1 for y.
    pub axis: usize,
    /// The clearance every body of the tool keeps from it.
    pub clearance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FixtureError {
    /// The stock is wider than the vise opens.
    Opening { need: f64, max: f64 },
    /// The clamp depth is not within (0, min(stock height, jaw height)].
    Clamp { clamp: f64, max: f64 },
    /// The parallels would not fit between the jaws.
    Parallels { width: f64, opening: f64 },
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureError::Opening { need, max } => write!(
                f,
                "the stock is {need:.2} mm across the jaws; the vise opens {max:.2} mm: turn the stock, or a bigger vise"
            ),
            FixtureError::Clamp { clamp, max } => {
                write!(f, "clamp depth {clamp:.2} mm: it must be > 0 and at most {max:.2} mm")
            }
            FixtureError::Parallels { width, opening } => write!(
                f,
                "two parallels {width:.2} mm wide do not fit an opening of {opening:.2} mm: narrower parallels"
            ),
        }
    }
}

/// The vise's size, as the library holds it.
#[derive(Clone, Copy, Debug)]
pub struct ViseSize {
    pub jaw_width: f64,
    pub jaw_height: f64,
    pub jaw_thickness: f64,
    pub max_opening: f64,
    pub bed_depth: f64,
}

/// Places a vise on the stock: jaws closed on the stock's faces across `axis`, gripping `clamp`
/// of it, parallels `parallel` wide under it.
pub fn vise(
    name: &str,
    v: ViseSize,
    stock: &Stock,
    axis: usize,
    clamp: f64,
    parallel: f64,
    clearance: f64,
) -> Result<Fixture, FixtureError> {
    let (a, o) = (axis, 1 - axis);
    let opening = stock.max[a] - stock.min[a];
    if opening > v.max_opening {
        return Err(FixtureError::Opening {
            need: opening,
            max: v.max_opening,
        });
    }
    let h = stock.max[2] - stock.min[2];
    let cmax = h.min(v.jaw_height);
    if !(clamp > 0.0 && clamp <= cmax) {
        return Err(FixtureError::Clamp { clamp, max: cmax });
    }
    if 2.0 * parallel >= opening {
        return Err(FixtureError::Parallels {
            width: parallel,
            opening,
        });
    }
    let seat = stock.min[2];
    let top = seat + clamp;
    let bed = top - v.jaw_height;
    let mid = 0.5 * (stock.min[o] + stock.max[o]);
    let (w0, w1) = (mid - 0.5 * v.jaw_width, mid + 0.5 * v.jaw_width);
    let bx = |lo_a: f64, hi_a: f64, z0: f64, z1: f64| {
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        min[a] = lo_a;
        max[a] = hi_a;
        min[o] = w0;
        max[o] = w1;
        min[2] = z0;
        max[2] = z1;
        Stock { min, max }
    };
    let (s0, s1) = (stock.min[a], stock.max[a]);
    let t = v.jaw_thickness;
    let mut pieces = vec![
        ("fixed jaw".to_string(), bx(s0 - t, s0, bed - v.bed_depth, top)),
        ("moving jaw".to_string(), bx(s1, s1 + t, bed - v.bed_depth, top)),
        ("bed".to_string(), bx(s0, s1, bed - v.bed_depth, bed)),
    ];
    if seat > bed {
        pieces.push(("parallel".to_string(), bx(s0, s0 + parallel, bed, seat)));
        pieces.push(("parallel".to_string(), bx(s1 - parallel, s1, bed, seat)));
    }
    Ok(Fixture {
        name: name.into(),
        pieces,
        jaw_top: top,
        axis,
        clearance,
    })
}

impl Fixture {
    /// Every piece's faces, ids from 3000 by piece.
    pub fn faces(&self) -> Vec<Face> {
        let mut out = vec![];
        for (k, (_, b)) in self.pieces.iter().enumerate() {
            for mut f in b.faces() {
                f.id = 3000 + 10 * k as u32 + (f.id - 1000);
                out.push(f);
            }
        }
        out
    }

    /// The piece a fixture face id names.
    pub fn piece_of(&self, id: u32) -> Option<&str> {
        let k = id.checked_sub(3000)? / 10;
        self.pieces.get(k as usize).map(|p| p.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V: ViseSize = ViseSize {
        jaw_width: 150.0,
        jaw_height: 50.0,
        jaw_thickness: 20.0,
        max_opening: 160.0,
        bed_depth: 20.0,
    };

    #[test]
    fn placed_on_the_stock() {
        let s = Stock {
            min: [-40.0, -30.0, 0.0],
            max: [40.0, 30.0, 20.0],
        };
        let f = vise("v", V, &s, 0, 5.0, 6.0, 1.0).unwrap();
        assert_eq!(f.jaw_top, 5.0);
        assert_eq!(f.pieces[0].1.max, [-40.0, 75.0, 5.0]);
        assert_eq!(f.pieces[1].1.min, [40.0, -75.0, -65.0]);
        assert_eq!(f.pieces.len(), 5);
        assert_eq!(f.faces().len(), 30);
        assert_eq!(f.piece_of(3013), Some("moving jaw"));
        assert!(matches!(
            vise("v", V, &s, 0, 25.0, 6.0, 1.0),
            Err(FixtureError::Clamp { .. })
        ));
        let wide = Stock {
            min: [-90.0, -30.0, 0.0],
            max: [90.0, 30.0, 20.0],
        };
        assert!(matches!(
            vise("v", V, &wide, 0, 5.0, 6.0, 1.0),
            Err(FixtureError::Opening { .. })
        ));
        // across y it fits
        assert!(vise("v", V, &wide, 1, 5.0, 6.0, 1.0).is_ok());
    }
}
