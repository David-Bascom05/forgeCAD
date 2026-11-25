//! Rest machining's single pass: a piece of tool centres C whose material all lies within reach
//! of the part of C's edge that is the wall's offset (lang's `rest_region`, `edge_only`) is cut
//! by running that edge, the wall runs as cuts and the rest of C's edge (centres that only touch
//! the material, or less) as links at the cutting feed, level by level, straight down between
//! levels. Method: plain contour following along an offset (textbook).
use crate::leads::{nearest_param, point_at, seg_len};
use crate::ops::{Cut, OpPath, PMove, Phase, Span, levels};
use forge_cam_geom::region::{Region, Seg};

/// Whether `s` lies on the offset of `wall` by `r` (its middle at r from the wall, to `tol`).
fn on_offset(s: &Seg, wall: &Region, r: f64, tol: f64) -> bool {
    let m = point_at(s, 0.5);
    let d = wall
        .loops
        .iter()
        .flat_map(|l| l.segs.iter())
        .map(|w| nearest_param(w, m).0)
        .fold(f64::INFINITY, f64::min);
    (d - r).abs() <= tol
}

fn push(mv: &mut Vec<PMove>, s: &Seg, z: f64, f: f64, phase: Phase) {
    match *s {
        Seg::Line { b, .. } => mv.push(PMove::Line {
            to: [b.x, b.y, z],
            f,
            phase,
        }),
        Seg::Arc { b, c, ccw, .. } => {
            // (an arc is a cut; one off the wall is in air by construction)
            mv.push(PMove::Arc {
                to: [b.x, b.y, z],
                c: [c.x, c.y],
                cw: !ccw,
                f,
            })
        }
    }
}

/// The single pass round piece `c` (one counter-clockwise loop of tool centres: the wall on the
/// right, climb with M03) inside the hole `wall`, the tool's radius plus leave `r` from it. None
/// when the piece is not one loop or has no run on the wall's offset.
pub fn rest_pass(c: &Region, wall: &Region, r: f64, cut: Cut, sp: Span) -> Option<OpPath> {
    if c.loops.len() != 1 {
        return None;
    }
    let segs = &c.loops[0].segs;
    let n = segs.len();
    let tol = 1e-6 * (1.0 + r);
    let on: Vec<bool> = segs
        .iter()
        .map(|s| seg_len(s) > 0.0 && on_offset(s, wall, r, tol))
        .collect();
    // start where a run on the wall's offset begins (else nowhere to cut)
    let first = (0..n).find(|&i| on[i] && !on[(i + n - 1) % n])?;
    let order: Vec<usize> = (0..n).map(|k| (first + k) % n).collect();
    let Cut { tidx, h, feed, .. } = cut;
    let s0 = segs[first].start();
    let mut mv = vec![PMove::Rapid {
        to: [s0.x, s0.y, h.retract],
    }];
    // one run on the wall: back from its end to its start on a straight link where that chord
    // stays in the piece (allowed centres: the certified checks hold it either way); else on
    // round the rest of the edge
    let run: Vec<usize> = order.iter().copied().take_while(|&i| on[i]).collect();
    let single = run.len() < n && order[run.len()..].iter().all(|&i| !on[i]);
    let e0 = segs[run[run.len() - 1]].end();
    let chord = single && chord_inside(segs, e0, s0);
    for z in levels(sp.top, sp.bottom, sp.step) {
        // down at the start: from the retract height, then from each level to the next
        mv.push(PMove::Line {
            to: [s0.x, s0.y, z],
            f: feed.1,
            phase: Phase::Plunge,
        });
        if chord {
            for &i in &run {
                push(&mut mv, &segs[i], z, feed.0, Phase::Cut);
            }
            mv.push(PMove::Line {
                to: [s0.x, s0.y, z],
                f: feed.0,
                phase: Phase::Link,
            });
            continue;
        }
        for &i in &order {
            let phase = if on[i] { Phase::Cut } else { Phase::Link };
            push(&mut mv, &segs[i], z, feed.0, phase);
        }
    }
    mv.push(PMove::Line {
        to: [s0.x, s0.y, h.retract],
        f: feed.0,
        phase: Phase::Link,
    });
    mv.push(PMove::Rapid {
        to: [s0.x, s0.y, h.clear],
    });
    Some(OpPath {
        tool: tidx,
        start: [s0.x, s0.y, h.clear],
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    })
}

/// Whether the segment a→b stays inside the loop (its polyline, arcs by their ends and
/// middles): sampled, the certified checks being the guard.
fn chord_inside(segs: &[Seg], a: forge_cam_geom::region::P2, b: forge_cam_geom::region::P2) -> bool {
    let mut poly = vec![];
    for s in segs {
        poly.push(s.start());
        if let Seg::Arc { .. } = s {
            poly.push(point_at(s, 0.25));
            poly.push(point_at(s, 0.5));
            poly.push(point_at(s, 0.75));
        }
    }
    let inside = |x: f64, y: f64| {
        let mut w = false;
        let m = poly.len();
        for k in 0..m {
            let (p, q) = (poly[k], poly[(k + 1) % m]);
            if (p.y > y) != (q.y > y) && x < p.x + (y - p.y) / (q.y - p.y) * (q.x - p.x) {
                w = !w;
            }
        }
        w
    };
    (1..20).all(|k| {
        let t = k as f64 / 20.0;
        inside(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    })
}
