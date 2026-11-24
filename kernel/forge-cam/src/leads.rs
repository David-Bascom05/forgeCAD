//! Leads and the entry position of a closed contour loop.
//!
//! Method (plain, textbook CNC practice; no patented method): the tool joins the loop on a
//! circular arc tangent to the loop at its start (G1-continuous), optionally preceded by a
//! straight line tangent to that arc, and leaves it the same way at its end. The arc lies on
//! the air side of the loop: with the part on the tool's right (climb) the air is on the left,
//! else on the right. The entry position rotates the loop to start at the point of the loop
//! nearest a given point (the segment there is split in two). Nothing here certifies: the
//! moves go through the same certified checks as every other move.
use forge_cam_geom::region::{P2, Seg, p2};
use forge_math::dmath;

/// One lead: a tangent arc of `radius` through `sweep_deg` degrees (radius 0: no arc), then
/// (lead-in: before it; lead-out: after it) a tangent line of `length` (0: none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lead {
    pub radius: f64,
    pub sweep_deg: f64,
    pub length: f64,
}

impl Lead {
    pub fn is_none(&self) -> bool {
        self.radius <= 0.0 && self.length <= 0.0
    }
}

/// A contour loop's leads and entry. The default is none of them: the path is unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Leads {
    pub lead_in: Option<Lead>,
    pub lead_out: Option<Lead>,
    /// Start each loop at its point nearest this one (xy).
    pub entry: Option<[f64; 2]>,
    /// Cut on past the loop's start by this length before leaving (hides the witness mark).
    pub overlap: f64,
}

impl Leads {
    pub fn is_none(&self) -> bool {
        self.lead_in.is_none_or(|l| l.is_none())
            && self.lead_out.is_none_or(|l| l.is_none())
            && self.entry.is_none()
            && self.overlap <= 0.0
    }
}

fn sub(a: P2, b: P2) -> P2 {
    p2(a.x - b.x, a.y - b.y)
}
fn add(a: P2, b: P2) -> P2 {
    p2(a.x + b.x, a.y + b.y)
}
fn mul(a: P2, k: f64) -> P2 {
    p2(a.x * k, a.y * k)
}
fn len(a: P2) -> f64 {
    (a.x * a.x + a.y * a.y).sqrt()
}
fn unit(a: P2) -> P2 {
    mul(a, 1.0 / len(a))
}
/// a turned a quarter counter-clockwise.
fn left(a: P2) -> P2 {
    p2(-a.y, a.x)
}
fn rot(a: P2, t: f64) -> P2 {
    let (s, c) = dmath::sin_cos(t);
    p2(c * a.x - s * a.y, s * a.x + c * a.y)
}

/// The unit direction of travel along `s` at its point `p`.
pub fn tangent(s: &Seg, p: P2) -> P2 {
    match *s {
        Seg::Line { a, b } => unit(sub(b, a)),
        Seg::Arc { c, ccw, .. } => {
            let t = unit(left(sub(p, c)));
            if ccw { t } else { mul(t, -1.0) }
        }
    }
}

/// The angle swept from a to b about c in the arc's sense, in (0, 2π] (a == b: the full turn).
fn sweep(a: P2, b: P2, c: P2, ccw: bool) -> f64 {
    let ta = dmath::atan2(a.y - c.y, a.x - c.x);
    let tb = dmath::atan2(b.y - c.y, b.x - c.x);
    let mut d = if ccw { tb - ta } else { ta - tb };
    let tau = std::f64::consts::TAU;
    while d <= 0.0 {
        d += tau;
    }
    while d > tau {
        d -= tau;
    }
    d
}

/// A closed loop's signed area (counter-clockwise positive): the chords' shoelace plus each
/// arc's circular segment.
pub fn area(segs: &[Seg]) -> f64 {
    let mut a2 = 0.0;
    for sg in segs {
        let (a, b) = (sg.start(), sg.end());
        a2 += a.x * b.y - a.y * b.x;
        if let Seg::Arc { a, b, c, ccw } = *sg {
            let r = len(sub(a, c));
            let th = sweep(a, b, c, ccw);
            let seg = r * r * (th - dmath::sin_cos(th).0);
            a2 += if ccw { seg } else { -seg };
        }
    }
    0.5 * a2
}

/// The length of a segment.
pub fn seg_len(s: &Seg) -> f64 {
    match *s {
        Seg::Line { a, b } => len(sub(b, a)),
        Seg::Arc { a, b, c, ccw } => len(sub(a, c)) * sweep(a, b, c, ccw),
    }
}

/// The point of `s` nearest `q`, as (distance, the point).
fn nearest(s: &Seg, q: P2) -> (f64, P2) {
    let p = match *s {
        Seg::Line { a, b } => {
            let d = sub(b, a);
            let t = ((q.x - a.x) * d.x + (q.y - a.y) * d.y) / (d.x * d.x + d.y * d.y);
            add(a, mul(d, t.clamp(0.0, 1.0)))
        }
        Seg::Arc { a, b, c, ccw } => {
            let r = len(sub(a, c));
            let v = sub(q, c);
            let on = if len(v) > 0.0 { add(c, mul(unit(v), r)) } else { a };
            // on the arc when its sweep from a is within the arc's
            if a == b || sweep(a, on, c, ccw) <= sweep(a, b, c, ccw) {
                on
            } else if len(sub(q, a)) <= len(sub(q, b)) {
                a
            } else {
                b
            }
        }
    };
    (len(sub(q, p)), p)
}

/// The point of `s` nearest `q`, as (distance, the fraction of `s`'s length to it).
pub fn nearest_param(s: &Seg, q: P2) -> (f64, f64) {
    let (d, p) = nearest(s, q);
    let k = match *s {
        Seg::Line { a, b } => len(sub(p, a)) / len(sub(b, a)),
        Seg::Arc { a, b, c, ccw } => {
            if len(sub(p, a)) == 0.0 {
                0.0
            } else {
                (sweep(a, p, c, ccw) / sweep(a, b, c, ccw)).min(1.0)
            }
        }
    };
    (d, k)
}

/// The point the fraction `k` of the way along `s`.
pub fn point_at(s: &Seg, k: f64) -> P2 {
    match *s {
        Seg::Line { a, b } => add(a, mul(sub(b, a), k)),
        Seg::Arc { a, b, c, ccw } => {
            let t = sweep(a, b, c, ccw) * k;
            add(c, rot(sub(a, c), if ccw { t } else { -t }))
        }
    }
}

/// `s` split at its point `p` (not an end): the two pieces.
pub fn split_at(s: &Seg, p: P2) -> (Seg, Seg) {
    split(s, p)
}

fn split(s: &Seg, p: P2) -> (Seg, Seg) {
    match *s {
        Seg::Line { a, b } => (Seg::Line { a, b: p }, Seg::Line { a: p, b }),
        Seg::Arc { a, b, c, ccw } => (Seg::Arc { a, b: p, c, ccw }, Seg::Arc { a: p, b, c, ccw }),
    }
}

/// The loop started at its point nearest `q`. Ends within `eps` of a segment's end snap to it,
/// so no piece is shorter than `eps`.
pub fn rotate_to(segs: &[Seg], q: [f64; 2], eps: f64) -> Vec<Seg> {
    let q = p2(q[0], q[1]);
    let mut best = (f64::INFINITY, 0, q);
    for (i, s) in segs.iter().enumerate() {
        let (d, p) = nearest(s, q);
        if d < best.0 {
            best = (d, i, p);
        }
    }
    let (_, i, p) = best;
    let s = segs[i];
    let n = segs.len();
    // a full circle: the same circle started at p
    if let Seg::Arc { a, b, c, ccw } = s
        && a == b
        && n == 1
    {
        return vec![Seg::Arc { a: p, b: p, c, ccw }];
    }
    let mut out = Vec::with_capacity(n + 1);
    if len(sub(p, s.start())) <= eps {
        out.extend_from_slice(&segs[i..]);
        out.extend_from_slice(&segs[..i]);
    } else if len(sub(p, s.end())) <= eps {
        out.extend_from_slice(&segs[i + 1..]);
        out.extend_from_slice(&segs[..=i]);
    } else {
        let (h, t) = split(&s, p);
        out.push(t);
        out.extend_from_slice(&segs[i + 1..]);
        out.extend_from_slice(&segs[..i]);
        out.push(h);
    }
    out
}

/// The loop's first `d` of length (the overlap cut past the start), split where it ends.
pub fn prefix(segs: &[Seg], d: f64) -> Vec<Seg> {
    let mut out = vec![];
    let mut left_d = d;
    for s in segs {
        if left_d <= 0.0 {
            break;
        }
        let l = seg_len(s);
        if l <= left_d {
            out.push(*s);
            left_d -= l;
            continue;
        }
        let p = point_at(s, left_d / l);
        out.push(split(s, p).0);
        break;
    }
    out
}

/// The air side's unit normal at a point with travel direction `t`: left when the part is on
/// the tool's right (`part_right`), else right.
fn air(t: P2, part_right: bool) -> P2 {
    if part_right { left(t) } else { mul(left(t), -1.0) }
}

/// The lead-in ending at `s` with travel direction `t` there: its pieces, the first starting
/// where the tool comes down.
pub fn lead_in(s: P2, t: P2, part_right: bool, l: &Lead) -> Vec<Seg> {
    let n = air(t, part_right);
    let mut out = vec![];
    let (mut p, mut dir) = (s, t);
    if l.radius > 0.0 && l.sweep_deg > 0.0 {
        let c = add(s, mul(n, l.radius));
        // the arc turns toward the part when the air is on the left: ccw
        let ccw = part_right;
        let th = l.sweep_deg.to_radians();
        let a = add(c, rot(sub(s, c), if ccw { -th } else { th }));
        out.push(Seg::Arc { a, b: s, c, ccw });
        dir = tangent(&out[0], a);
        p = a;
    }
    if l.length > 0.0 {
        out.insert(
            0,
            Seg::Line {
                a: sub(p, mul(dir, l.length)),
                b: p,
            },
        );
    }
    out
}

/// The lead-out leaving `e` with travel direction `t` there.
pub fn lead_out(e: P2, t: P2, part_right: bool, l: &Lead) -> Vec<Seg> {
    let n = air(t, part_right);
    let mut out = vec![];
    let (mut p, mut dir) = (e, t);
    if l.radius > 0.0 && l.sweep_deg > 0.0 {
        let c = add(e, mul(n, l.radius));
        // turning away from the part: toward the air side
        let ccw = part_right;
        let th = l.sweep_deg.to_radians();
        let b = add(c, rot(sub(e, c), if ccw { th } else { -th }));
        let arc = Seg::Arc { a: e, b, c, ccw };
        dir = tangent(&arc, b);
        p = b;
        out.push(arc);
    }
    if l.length > 0.0 {
        out.push(Seg::Line {
            a: p,
            b: add(p, mul(dir, l.length)),
        });
    }
    out
}

/// One loop with its entry, leads and overlap: (lead-in, the loop, the overlap, the lead-out).
/// `part_right`: the part is on the tool's right as it runs the loop (climb).
#[allow(clippy::type_complexity)]
pub fn wrap(
    segs: &[Seg],
    part_right: bool,
    ld: &Leads,
    eps: f64,
) -> (Vec<Seg>, Vec<Seg>, Vec<Seg>, Vec<Seg>) {
    // a loop of no length (the tool exactly fits a round hole: the offset is the centre, a
    // circle of radius 0) has no direction to lead along, and one of no width (the tool exactly
    // fits a slot: the offset runs along its centreline and back) has no air beside it: either
    // is cut as it is, with no lead or overlap
    let total: f64 = segs.iter().map(seg_len).sum();
    if total <= eps || 2.0 * area(segs).abs() / total <= eps {
        return (vec![], segs.to_vec(), vec![], vec![]);
    }
    let mut body = match ld.entry {
        Some(q) => rotate_to(segs, q, eps),
        None => segs.to_vec(),
    };
    // a lead tangent at a sharp corner swings out past the corner's other side: start a loop
    // that begins at a corner from the middle of its first piece instead
    let has_lead = |l: Option<Lead>| l.is_some_and(|l| !l.is_none());
    if (has_lead(ld.lead_in) || has_lead(ld.lead_out)) && body.len() > 1 {
        let s0 = body[0].start();
        let (t0, t1) = (tangent(&body[body.len() - 1], s0), tangent(&body[0], s0));
        let cross = t0.x * t1.y - t0.y * t1.x;
        let dot = t0.x * t1.x + t0.y * t1.y;
        if cross.abs() > 1e-9 || dot < 0.0 {
            let m = point_at(&body[0], 0.5);
            body = rotate_to(&body, [m.x, m.y], eps);
        }
    }
    let s = body[0].start();
    let t_in = tangent(&body[0], s);
    let li = match ld.lead_in {
        Some(l) if !l.is_none() => lead_in(s, t_in, part_right, &l),
        _ => vec![],
    };
    let ov = if ld.overlap > 0.0 {
        prefix(&body, ld.overlap)
    } else {
        vec![]
    };
    let (e, t_out) = match ov.last() {
        Some(x) => (x.end(), tangent(x, x.end())),
        None => {
            let x = body.last().unwrap_or(&body[0]);
            (x.end(), tangent(x, x.end()))
        }
    };
    let lo = match ld.lead_out {
        Some(l) if !l.is_none() => lead_out(e, t_out, part_right, &l),
        _ => vec![],
    };
    (li, body, ov, lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Seg> {
        // ccw unit square of side 10
        let p = [p2(0.0, 0.0), p2(10.0, 0.0), p2(10.0, 10.0), p2(0.0, 10.0)];
        (0..4)
            .map(|i| Seg::Line {
                a: p[i],
                b: p[(i + 1) % 4],
            })
            .collect()
    }

    fn close(a: P2, b: P2) -> bool {
        len(sub(a, b)) < 1e-12
    }

    #[test]
    fn entry_splits_the_nearest_side() {
        let l = rotate_to(&square(), [5.0, -3.0], 1e-6);
        assert_eq!(l.len(), 5);
        assert!(close(l[0].start(), p2(5.0, 0.0)));
        assert!(close(l[4].end(), p2(5.0, 0.0)));
        for i in 0..5 {
            assert_eq!(l[i].end(), l[(i + 1) % 5].start());
        }
    }

    #[test]
    fn entry_snaps_to_a_corner() {
        let l = rotate_to(&square(), [10.0 + 1e-9, 10.0], 1e-6);
        assert_eq!(l.len(), 4);
        assert_eq!(l[0].start(), p2(10.0, 10.0));
    }

    #[test]
    fn lead_arcs_are_tangent_and_on_the_air_side() {
        // a ccw loop has its region on the left; as an outside contour run ccw the part is on
        // the left (conventional): the air is on the right
        let ld = Lead {
            radius: 2.0,
            sweep_deg: 90.0,
            length: 1.0,
        };
        let s = p2(5.0, 0.0);
        let t = p2(1.0, 0.0);
        let li = lead_in(s, t, false, &ld);
        assert_eq!(li.len(), 2);
        // ends at s tangent to t
        assert!(close(li[1].end(), s));
        assert!(close(tangent(&li[1], s), t));
        // the arc's start is on the air side (below the square): y < 0
        assert!(li[1].start().y < -1.9);
        // the line is tangent to the arc where they meet
        assert!(close(li[0].end(), li[1].start()));
        assert!(close(
            tangent(&li[0], li[0].end()),
            tangent(&li[1], li[1].start())
        ));
        let lo = lead_out(s, t, false, &ld);
        assert!(close(lo[0].start(), s));
        assert!(close(tangent(&lo[0], s), t));
        assert!(lo[0].end().y < -1.9);
        assert!(close(lo[1].start(), lo[0].end()));
        // the climb sense mirrors it
        let li = lead_in(s, t, true, &ld);
        assert!(li[1].start().y > 1.9);
    }

    /// A loop starting at a sharp corner is started mid-side when it has leads (a tangent arc at
    /// the corner would swing out past the other side).
    #[test]
    fn a_corner_start_moves_mid_side() {
        let l = Lead {
            radius: 1.0,
            sweep_deg: 90.0,
            length: 0.0,
        };
        let ld = Leads {
            lead_in: Some(l),
            lead_out: Some(l),
            ..Default::default()
        };
        let (li, body, _, _) = wrap(&square(), false, &ld, 1e-6);
        assert!(close(body[0].start(), p2(5.0, 0.0)), "{:?}", body[0]);
        assert!(close(li[0].end(), p2(5.0, 0.0)));
        // no leads: the loop as it was
        let (_, body, _, _) = wrap(&square(), false, &Leads::default(), 1e-6);
        assert_eq!(body, square());
    }

    #[test]
    fn areas_and_a_collapsed_loop() {
        assert!((area(&square()) - 100.0).abs() < 1e-12);
        let a = p2(2.0, 0.0);
        let circ = [Seg::Arc {
            a,
            b: a,
            c: p2(0.0, 0.0),
            ccw: true,
        }];
        assert!((area(&circ) - 4.0 * std::f64::consts::PI).abs() < 1e-9);
        // a slot's centreline there and back: no width, so no leads
        let (p, q) = (p2(60.0, 62.0), p2(102.0, 62.0));
        let back = [Seg::Line { a: q, b: p }, Seg::Line { a: p, b: q }];
        let l = Lead {
            radius: 2.0,
            sweep_deg: 90.0,
            length: 0.0,
        };
        let (li, body, _, lo) = wrap(
            &back,
            true,
            &Leads {
                lead_in: Some(l),
                lead_out: Some(l),
                ..Default::default()
            },
            1e-6,
        );
        assert!(li.is_empty() && lo.is_empty());
        assert_eq!(body, back.to_vec());
    }

    /// A point loop (a circle of radius 0, the tool exactly fitting a hole) gets no leads and no
    /// NaN: it is the plunge it was.
    #[test]
    fn a_point_loop_has_no_leads() {
        let c = p2(57.0, 38.0);
        let pt = [Seg::Arc {
            a: c,
            b: c,
            c,
            ccw: true,
        }];
        let l = Lead {
            radius: 2.0,
            sweep_deg: 90.0,
            length: 1.0,
        };
        let (li, body, ov, lo) = wrap(
            &pt,
            true,
            &Leads {
                lead_in: Some(l),
                lead_out: Some(l),
                entry: Some([0.0, 0.0]),
                overlap: 0.5,
            },
            1e-6,
        );
        assert!(li.is_empty() && ov.is_empty() && lo.is_empty());
        assert_eq!(body, pt.to_vec());
    }

    #[test]
    fn overlap_and_sweep() {
        let (li, body, ov, lo) = wrap(
            &square(),
            false,
            &Leads {
                lead_in: Some(Lead {
                    radius: 1.0,
                    sweep_deg: 45.0,
                    length: 0.0,
                }),
                lead_out: Some(Lead {
                    radius: 1.0,
                    sweep_deg: 45.0,
                    length: 0.0,
                }),
                entry: Some([5.0, -1.0]),
                overlap: 0.5,
            },
            1e-6,
        );
        assert_eq!(li.len(), 1);
        assert!((seg_len(&li[0]) - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
        assert_eq!(body.len(), 5);
        assert!(close(ov[0].end(), p2(5.5, 0.0)));
        assert!(close(lo[0].start(), p2(5.5, 0.0)));
    }
}
