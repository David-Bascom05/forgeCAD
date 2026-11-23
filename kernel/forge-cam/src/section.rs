//! The part's cross-section at a height, from its prism faces: each vertical wall over a segment
//! and each vertical cylinder over an arc whose heights span z contributes its section curve;
//! the curves chain into closed loops by their shared ends (the lowering's shared vertices), and
//! the loops are oriented as a region (outer counter-clockwise, islands clockwise).
use forge_cam_geom::check::Face;
use forge_cam_geom::region::{Loop, P2, Region, Seg, p2};
use forge_cam_geom::slab::{Prism, Sec};

#[derive(Clone, Debug, PartialEq)]
pub enum SectionError {
    /// A face crossing the height is not a prism along z: the section is not 2.5D there.
    NotPrism { face: u32 },
    /// The section's curves do not close into loops.
    Open { at: P2 },
    Empty,
}

fn close(a: P2, b: P2, tol: f64) -> bool {
    (a.x - b.x).abs() <= tol && (a.y - b.y).abs() <= tol
}

/// The section of the lowered part at height z (strictly inside every wall's height range it
/// meets: a height at a floor or top is refused as ambiguous by `NotPrism` of that flat face).
pub fn section(faces: &[Face], z: f64, mu: f64) -> Result<Region, SectionError> {
    let mut pieces: Vec<Seg> = Vec::new();
    for f in faces {
        if f.bbox[2].hi < z || f.bbox[2].lo > z {
            continue;
        }
        match &f.prism {
            Some(Prism::Wall { sec, z0, z1 }) => {
                if *z0 < z && z < *z1 {
                    pieces.push(match *sec {
                        Sec::Seg { a, b } => Seg::Line { a: p2(a[0], a[1]), b: p2(b[0], b[1]) },
                        Sec::Arc { c, a, b, ccw, .. } => Seg::Arc { a: p2(a[0], a[1]), b: p2(b[0], b[1]), c: p2(c[0], c[1]), ccw },
                    });
                }
            }
            Some(Prism::Flat { z: zf, .. }) => {
                if (zf - z).abs() <= mu {
                    return Err(SectionError::NotPrism { face: f.id });
                }
            }
            None => return Err(SectionError::NotPrism { face: f.id }),
        }
    }
    if pieces.is_empty() {
        return Err(SectionError::Empty);
    }
    // the walls' sections were padded by mu at their ends: snap ends that agree within 4 mu
    let tol = 4.0 * mu;
    let mut used = vec![false; pieces.len()];
    let mut loops: Vec<Loop> = Vec::new();
    for s in 0..pieces.len() {
        if used[s] {
            continue;
        }
        used[s] = true;
        let mut lp = vec![pieces[s]];
        let start = pieces[s].start();
        loop {
            let end = lp.last().unwrap().end();
            if close(end, start, tol) && lp.len() > 1 || (lp.len() == 1 && start == end) {
                break;
            }
            let mut found = false;
            for k in 0..pieces.len() {
                if used[k] {
                    continue;
                }
                let (a, b) = (pieces[k].start(), pieces[k].end());
                if close(a, end, tol) {
                    lp.push(pieces[k]);
                    used[k] = true;
                    found = true;
                    break;
                }
                if close(b, end, tol) {
                    lp.push(pieces[k].reversed());
                    used[k] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(SectionError::Open { at: end });
            }
        }
        // make ends bit-equal: each piece starts where the last ended
        let n = lp.len();
        for i in 0..n {
            let e = lp[i].end();
            let nx = &mut lp[(i + 1) % n];
            *nx = match *nx {
                Seg::Line { b, .. } => Seg::Line { a: e, b },
                Seg::Arc { b, c, ccw, .. } => Seg::Arc { a: e, b, c, ccw },
            };
        }
        loops.push(Loop { segs: lp });
    }
    // orient: validate, and flip loops whose sense is wrong for their depth
    let mut rg = Region { loops };
    for _ in 0..2 {
        let mut cx = forge_cam_geom::real::Ctx::new();
        match forge_cam_geom::region::validate(&mut cx, &rg) {
            Ok(_) => return Ok(rg),
            Err(forge_cam_geom::region::RegionError::Sense { lp, .. }) => {
                rg.loops[lp] = rg.loops[lp].reversed();
                // other loops may need it too: go round again
                let mut fixed = true;
                for _ in 0..rg.loops.len() {
                    let mut cx = forge_cam_geom::real::Ctx::new();
                    match forge_cam_geom::region::validate(&mut cx, &rg) {
                        Ok(_) => return Ok(rg),
                        Err(forge_cam_geom::region::RegionError::Sense { lp, .. }) => rg.loops[lp] = rg.loops[lp].reversed(),
                        Err(_) => {
                            fixed = false;
                            break;
                        }
                    }
                }
                if fixed {
                    continue;
                }
                return Err(SectionError::Open { at: rg.loops[0].segs[0].start() });
            }
            Err(_) => return Err(SectionError::Open { at: rg.loops[0].segs[0].start() }),
        }
    }
    Ok(rg)
}
