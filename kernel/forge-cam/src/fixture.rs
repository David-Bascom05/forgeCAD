//! Workholding: a vise placed on the stock, as boxes the certified checker holds every body of
//! the tool away from by the fixture clearance (G2: for every s, g(s)·(T ∪ N) ∩ (F ⊕ B_c) = ∅).
//!
//! The vise closes along the clamp axis on the stock's two faces across it. Its jaws stand
//! `clamp` above the stock's seat (so they grip that much of the stock); under the stock, two
//! parallels of `parallel` width run inside the jaws, and the vise bed lies `jaw_height` below
//! the jaw top. Every piece is a box: the fixture is their union, and the distance to a union is
//! the least distance to its pieces, so checking each piece's faces checks the fixture.
use crate::ops::Stock;
use crate::stock::InProcess;
use forge_cam_geom::boolean::{Op, boolean, region_of};
use forge_cam_geom::check::Face;
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, P2, Region, Seg};

/// A placed fixture: its named boxes, in the part's frame.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub name: String,
    pub pieces: Vec<(String, Stock)>,
    /// Pieces that are not boxes (soft jaws): their faces, already lowered.
    pub shaped: Vec<(String, Vec<Face>)>,
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
    /// The jaw-cutting tool cannot enter the grip: its radius, and the grip's.
    ToolTooBig { r: f64 },
    /// The grip comes within `wall` of the blank's edge.
    Blank { wall: f64, need: f64 },
    /// The part does not seat in the pocket the tool can cut: where (a box of what is missed).
    DoesNotSeat { at: [f64; 4] },
    /// The grip depth is not within (0, the part's height less the fixture clearance).
    Grip { grip: f64, max: f64 },
    Geometry(String),
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
            FixtureError::ToolTooBig { r } => write!(
                f,
                "a jaw-cutting tool of radius {r:.3} mm cannot enter the grip: a smaller tool"
            ),
            FixtureError::Blank { wall, need } => write!(
                f,
                "the grip leaves {wall:.2} mm of the blank (at least {need:.2}): a bigger blank, or grip across the other axis"
            ),
            FixtureError::DoesNotSeat { at } => write!(
                f,
                "the part does not seat in the jaws: the tool's radius leaves material at x {:.3}..{:.3}, y {:.3}..{:.3}: a smaller jaw tool, or `relief: \"corners\"` (a relief at each corner)",
                at[0], at[2], at[1], at[3]
            ),
            FixtureError::Grip { grip, max } => {
                write!(
                    f,
                    "grip depth {grip:.2} mm: it must be > 0 and under {max:.2} mm (the part's height less the fixture clearance)"
                )
            }
            FixtureError::Geometry(e) => write!(f, "soft jaws: {e}"),
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
        shaped: vec![],
        jaw_top: top,
        axis,
        clearance,
    })
}

/// A soft-jaw blank's size.
#[derive(Clone, Copy, Debug)]
pub struct BlankSize {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
}

/// A pair of soft jaws machined to one part: the fixture that holds the part, the vise that
/// holds the jaws while their pocket is cut, and the jaws as a part to machine.
#[derive(Clone, Debug)]
pub struct SoftJaws {
    /// For the setup that grips the part: the vise's bodies and the two jaws, pocketed.
    pub fixture: Fixture,
    /// For the setup that cuts the jaws: the vise's bodies.
    pub hold: Fixture,
    /// The two jaws closed, with the pocket: the target of the jaw-cutting setup.
    pub part: Vec<Face>,
    pub block: Stock,
    /// The pocket's outline (cut from the jaw top down to the seat).
    pub pocket: Region,
    pub seat: f64,
    pub top: f64,
    /// The seat's area under the part (its section just above the seat, mm²).
    pub contact: f64,
}

/// The area a region encloses (f64; lines by the shoelace, arcs with their segments).
pub fn area(r: &Region) -> f64 {
    let mut a = 0.0;
    for l in &r.loops {
        for s in &l.segs {
            match *s {
                Seg::Line { a: p, b: q } => a += 0.5 * (p.x * q.y - q.x * p.y),
                Seg::Arc { a: p, b: q, c, ccw } => {
                    let rr = ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt();
                    let t0 = forge_math::dmath::atan2(p.y - c.y, p.x - c.x);
                    let t1 = forge_math::dmath::atan2(q.y - c.y, q.x - c.x);
                    let tau = std::f64::consts::TAU;
                    let mut th = if ccw { t1 - t0 } else { t0 - t1 };
                    while th <= 0.0 {
                        th += tau;
                    }
                    if p == q {
                        th = tau;
                    }
                    let seg = 0.5 * rr * rr * (th - forge_math::dmath::sin(th));
                    a += 0.5 * (p.x * q.y - q.x * p.y) + if ccw { seg } else { -seg };
                }
            }
        }
    }
    a
}

fn bbox_of(r: &Region) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for l in &r.loops {
        for s in &l.segs {
            for p in [s.start(), s.end()] {
                b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
            }
            if let Seg::Arc { a, c, .. } = *s {
                let rr = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                b = [b[0].min(c.x - rr), b[1].min(c.y - rr), b[2].max(c.x + rr), b[3].max(c.y + rr)];
            }
        }
    }
    b
}

fn disk(c: P2, r: f64) -> Region {
    let a = P2 { x: c.x + r, y: c.y };
    Region {
        loops: vec![Loop {
            segs: vec![Seg::Arc { a, b: a, c, ccw: true }],
        }],
    }
}

/// The convex corners of a region's loops where two lines meet.
pub fn corners(r: &Region) -> Vec<P2> {
    let mut v = vec![];
    for l in &r.loops {
        let n = l.segs.len();
        for i in 0..n {
            if let (Seg::Line { a, b }, Seg::Line { b: c, .. }) = (l.segs[i], l.segs[(i + 1) % n]) {
                let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
                if cross > 0.0 {
                    v.push(b);
                }
            }
        }
    }
    v
}

fn geo<E: std::fmt::Debug>(e: E) -> FixtureError {
    FixtureError::Geometry(format!("{e:?}"))
}

/// Soft jaws for a part held by its sections `band` (each a section of the part, in the frame of
/// the setup that grips it, at the heights the jaws cover), seated at `seat`, gripped `grip`
/// deep. The pocket is the band's opening by a tool of radius `tool_r`, grown by `fit`:
/// M = (G ⊖ r) ⊕ (r + fit), plus a disk of radius r + fit at each point of `relief_at` (the part's
/// convex corners). It is refused unless every section lies inside M (the part seats), M keeps
/// `wall` of the blank, and the vise opens to the two blanks.
#[allow(clippy::too_many_arguments)]
pub fn soft_jaws(
    name: &str,
    v: ViseSize,
    b: BlankSize,
    axis: usize,
    band: &[Region],
    seat: f64,
    grip: f64,
    height: f64,
    fit: f64,
    tool_r: f64,
    relief_at: &[P2],
    clearance: f64,
) -> Result<SoftJaws, FixtureError> {
    if !(fit > 0.0) {
        return Err(FixtureError::Geometry(format!("fit {fit}: the jaws' pocket must be larger than the part (fit > 0)")));
    }
    // the part stands above the jaws by more than the fixture clearance, or nothing of it could
    // be cut in the setup the jaws hold
    if !(grip > 0.0 && grip < height - clearance) {
        return Err(FixtureError::Grip {
            grip,
            max: height - clearance,
        });
    }
    if 2.0 * b.thickness > v.max_opening {
        return Err(FixtureError::Opening {
            need: 2.0 * b.thickness,
            max: v.max_opening,
        });
    }
    let (a, o) = (axis, 1 - axis);
    // the jaws grip the outline: each section's outer loops, its holes filled
    let band: Vec<Region> = band
        .iter()
        .map(|r| Region {
            loops: r
                .loops
                .iter()
                .filter(|l| area(&Region { loops: vec![(*l).clone()] }) > 0.0)
                .cloned()
                .collect(),
        })
        .collect();
    let band = &band[..];
    // G: every section of the band, united
    let mut g = band.first().cloned().ok_or(FixtureError::Geometry("no section to grip".into()))?;
    for s in &band[1..] {
        g = region_of(&boolean(&g, s, Op::Union).map_err(geo)?);
    }
    // the pocket: what the tool's centre reaches inside G, grown by the tool and the fit; a
    // tool of radius r cuts it (it is ((G ⊖ r) ⊕ fit) ⊕ r), and it holds G's opening by `fit`
    let inner = offset(&g, tool_r).map_err(geo)?.0;
    if inner.is_empty() {
        return Err(FixtureError::ToolTooBig { r: tool_r });
    }
    let mut m = region_of(&offset(&region_of(&inner), -(tool_r + fit)).map_err(geo)?.0);
    // a relief is a disk of the tool's radius plus the fit: the tool cuts it by a circle of
    // radius `fit`, so it is cut whatever the rounding (a disk of exactly the tool's radius would
    // be cut only by a plunge at its exact centre)
    for &c in relief_at {
        m = region_of(&boolean(&m, &disk(c, tool_r + fit), Op::Union).map_err(geo)?);
    }
    // the part seats: every section inside the pocket
    for s in band {
        let miss = boolean(s, &m, Op::Diff).map_err(geo)?;
        if !miss.is_empty() {
            return Err(FixtureError::DoesNotSeat {
                at: bbox_of(&region_of(&miss)),
            });
        }
    }
    let mb = bbox_of(&m);
    let gb = bbox_of(&g);
    let mid = [0.5 * (gb[0] + gb[2]), 0.5 * (gb[1] + gb[3])];
    let top = seat + grip;
    let bot = top - b.height;
    let mut lo = [0.0; 3];
    let mut hi = [0.0; 3];
    lo[a] = mid[a] - b.thickness;
    hi[a] = mid[a] + b.thickness;
    lo[o] = mid[o] - 0.5 * b.width;
    hi[o] = mid[o] + 0.5 * b.width;
    lo[2] = bot;
    hi[2] = top;
    let block = Stock { min: lo, max: hi };
    let need = 3.0;
    let wall = (mb[0] - lo[0]).min(mb[1] - lo[1]).min(hi[0] - mb[2]).min(hi[1] - mb[3]);
    if wall < need {
        return Err(FixtureError::Blank { wall, need });
    }
    let jaw = |l: [f64; 3], h: [f64; 3]| -> Result<Vec<Face>, FixtureError> {
        let mut ip = InProcess::new(l, h);
        ip.cut(m.clone(), seat);
        ip.faces().map_err(FixtureError::Geometry)
    };
    let (mut ha, mut lb) = (hi, lo);
    ha[a] = mid[a];
    lb[a] = mid[a];
    let part = jaw(lo, hi)?;
    let jaws = vec![("soft jaw".to_string(), jaw(lo, ha)?), ("soft jaw".to_string(), jaw(lb, hi)?)];
    // the vise: its jaws behind the blanks, its bed under them
    let bx = |la: f64, ha: f64, z0: f64, z1: f64| {
        let mut min = lo;
        let mut max = hi;
        min[a] = la;
        max[a] = ha;
        min[2] = z0;
        max[2] = z1;
        Stock { min, max }
    };
    let t = v.jaw_thickness;
    let vtop = (bot + v.jaw_height).min(top);
    let pieces = vec![
        ("fixed jaw".to_string(), bx(lo[a] - t, lo[a], bot - v.bed_depth, vtop)),
        ("moving jaw".to_string(), bx(hi[a], hi[a] + t, bot - v.bed_depth, vtop)),
        ("bed".to_string(), bx(lo[a], hi[a], bot - v.bed_depth, bot)),
    ];
    let hold = Fixture {
        name: name.into(),
        pieces: pieces.clone(),
        shaped: vec![],
        jaw_top: vtop,
        axis,
        clearance,
    };
    let contact = area(band.first().expect("a band"));
    Ok(SoftJaws {
        fixture: Fixture {
            name: name.into(),
            pieces,
            shaped: jaws,
            jaw_top: top,
            axis,
            clearance,
        },
        hold,
        part,
        block,
        pocket: m,
        seat,
        top,
        contact,
    })
}

impl Fixture {
    /// Every piece's faces, ids from 3000 by piece.
    pub fn faces(&self) -> Vec<Face> {
        let mut out = vec![];
        for (k, (_, b)) in self.pieces.iter().enumerate() {
            for mut f in b.faces() {
                f.id = 3000 + 1000 * k as u32 + (f.id - 1000);
                out.push(f);
            }
        }
        for (j, (_, fs)) in self.shaped.iter().enumerate() {
            let k = self.pieces.len() + j;
            for (i, f) in fs.iter().enumerate() {
                let mut f = f.clone();
                f.id = 3000 + 1000 * k as u32 + i as u32;
                out.push(f);
            }
        }
        out
    }

    /// The piece a fixture face id names.
    pub fn piece_of(&self, id: u32) -> Option<&str> {
        let k = (id.checked_sub(3000)? / 1000) as usize;
        match self.pieces.get(k) {
            Some(p) => Some(p.0.as_str()),
            None => self.shaped.get(k - self.pieces.len()).map(|p| p.0.as_str()),
        }
    }

    /// Every piece's name and box (a shaped piece's box encloses it).
    pub fn boxes(&self) -> Vec<(String, [f64; 3], [f64; 3])> {
        let mut v: Vec<_> = self.pieces.iter().map(|(n, b)| (n.clone(), b.min, b.max)).collect();
        for (n, fs) in &self.shaped {
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            for f in fs {
                for k in 0..3 {
                    lo[k] = lo[k].min(f.bbox[k].lo + f.mu);
                    hi[k] = hi[k].max(f.bbox[k].hi - f.mu);
                }
            }
            v.push((n.clone(), lo, hi));
        }
        v
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
        assert_eq!(f.piece_of(4003), Some("moving jaw"));
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
