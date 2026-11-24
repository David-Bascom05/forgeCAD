//! The cutting conditions a milling op's tool actually meets, per fixed-length sample, for the
//! tool-life model (forge_tooling::life).
//!
//! The op's final path (feeds as posted) is replayed against the in-process stock with
//! `engage::schedule_samples`, the same measurement engagement feed uses. For each engaged
//! sample: its minutes (length / its move's feed), the programmed feed per tooth
//! fz = F / (z n), the peak chip fz·max cos θ (`Engage::cos_max`), the mean chip over the
//! engaged span fz·(sin θ_hi − sin θ_lo)/(θ_hi − θ_lo) (the chip at angle θ from the travel is
//! fz cos θ; the span may have gaps, so this is the mean over its hull), the axial depth, and
//! the cutting speed π D_eff n / 1000 at the engaged depth (D_eff for a ball,
//! forge_tooling::feeds::effective_diameter).
//!
//! Samples in air add nothing. Feed moves the sampler does not measure (straight plunges and
//! ramps down, canned cycles) are counted in `Measured::unsampled_minutes` so a report can say
//! what the estimate leaves out.
use crate::engage::{HeightMap, Profile, Sched, move_len, schedule_samples};
use crate::ops::{OpPath, PMove, Phase};
use forge_tooling::feeds::effective_diameter;
use forge_tooling::life::Cut;

/// An entry move into stock (for forge_tooling::entry): a straight plunge, a linear ramp or a
/// helix, or the first engaged sample after air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    /// A sample meeting stock after air: its feed, mm/min.
    Contact { mv: usize, feed: f64 },
    /// Straight down into stock: its feed.
    Plunge { mv: usize, feed: f64 },
    /// A linear descent into stock: its angle to the xy plane, degrees.
    Ramp { mv: usize, deg: f64 },
    /// A descending arc into stock: its angle, degrees, and the diameter it cuts over D.
    Helix { mv: usize, deg: f64, dia_ratio: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub cuts: Vec<Cut>,
    /// Minutes of feed moves down (plunges, ramps) that were not sampled.
    pub unsampled_minutes: f64,
    /// Canned cycles in the path (not in the estimate).
    pub cycles: usize,
    /// Entries into stock, in path order, and the largest feed of an engaged sample (mm/min).
    pub entries: Vec<Entry>,
    pub max_cut_feed: f64,
}

/// Whether stock stands above `z` within the tool's footprint at (x, y) (centre and 8 points at
/// 0.9 r).
fn in_stock(map: &HeightMap, p: [f64; 3], r: f64) -> bool {
    const EPS: f64 = 1e-3;
    if map.top(p[0], p[1]) > p[2] + EPS {
        return true;
    }
    (0..8).any(|k| {
        let (sn, cs) = forge_math::dmath::sin_cos(k as f64 * std::f64::consts::FRAC_PI_4);
        map.top(p[0] + 0.9 * r * cs, p[1] + 0.9 * r * sn) > p[2] + EPS
    })
}

/// The entries of `path` into the stock of `map` (copied: the caller's map is not changed).
fn scan_entries(path: &OpPath, map: &HeightMap, prof: &Profile, d: f64) -> Vec<Entry> {
    let mut m = HeightMap {
        x0: map.x0,
        y0: map.y0,
        h: map.h,
        nx: map.nx,
        ny: map.ny,
        bottom: map.bottom,
        z: map.z.clone(),
    };
    let r = prof.r();
    let mut out = Vec::new();
    let mut at = path.start;
    for (i, mv) in path.moves.iter().enumerate() {
        match *mv {
            PMove::Rapid { to } => {
                m.sweep(at, to, prof);
                at = to;
            }
            PMove::Line { to, f, .. } => {
                if to[2] < at[2] && in_stock(&m, to, r) {
                    let dxy = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt();
                    if dxy < 1e-9 {
                        out.push(Entry::Plunge { mv: i, feed: f });
                    } else {
                        let deg = forge_math::dmath::atan2(at[2] - to[2], dxy).to_degrees();
                        out.push(Entry::Ramp { mv: i, deg });
                    }
                }
                m.sweep(at, to, prof);
                at = to;
            }
            PMove::Arc { to, c, .. } => {
                let rr = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                let len = move_len(at, mv);
                if to[2] < at[2] && in_stock(&m, to, r) {
                    let xy = (len * len - (to[2] - at[2]).powi(2)).max(0.0).sqrt();
                    let deg = forge_math::dmath::atan2(at[2] - to[2], xy).to_degrees();
                    out.push(Entry::Helix {
                        mv: i,
                        deg,
                        dia_ratio: (2.0 * rr + d) / d,
                    });
                }
                // sweep along chords no longer than 0.5 mm
                let n = (len / 0.5).ceil().max(1.0) as usize;
                let a0 = forge_math::dmath::atan2(at[1] - c[1], at[0] - c[0]);
                let a1 = forge_math::dmath::atan2(to[1] - c[1], to[0] - c[0]);
                let cw = matches!(*mv, PMove::Arc { cw: true, .. });
                let mut sw = a1 - a0;
                if cw && sw >= 0.0 {
                    sw -= std::f64::consts::TAU;
                }
                if !cw && sw <= 0.0 {
                    sw += std::f64::consts::TAU;
                }
                let mut p = at;
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let (sn, cs) = forge_math::dmath::sin_cos(a0 + t * sw);
                    let q = if k == n {
                        to
                    } else {
                        [c[0] + rr * cs, c[1] + rr * sn, at[2] + t * (to[2] - at[2])]
                    };
                    m.sweep(p, q, prof);
                    p = q;
                }
                at = to;
            }
            PMove::Cycle { .. } => {}
        }
    }
    out
}

/// The tool, as the measurement needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolGeom {
    pub d: f64,
    pub flutes: u32,
    pub ball: bool,
}

/// Samples of `path` (op number `op`) against `map` (left as the path cut it), each no longer
/// than `sample` mm, measured on a grid of `cell` mm. `rpm` is the spindle speed the op runs at
/// (the path's own when it carries one, > 0).
#[allow(clippy::too_many_arguments)]
pub fn measure(
    op: usize,
    path: &OpPath,
    map: &mut HeightMap,
    prof: &Profile,
    tool: ToolGeom,
    sample: f64,
    cell: f64,
    rpm: f64,
) -> Measured {
    let rpm = if path.rpm > 0.0 { path.rpm } else { rpm };
    let sc = Sched {
        fz: 0.0,
        flutes: tool.flutes.max(1) as f64,
        rpm,
        f_max: 0.0,
        f_min: 0.0,
        cap: 1.0,
        cell,
        sample,
    };
    let mut entries = scan_entries(path, map, prof, tool.d);
    let samples = schedule_samples(path, map, prof, &sc);
    // each move's start, length and sample count
    let mut starts = Vec::with_capacity(path.moves.len());
    let mut at = path.start;
    for mv in &path.moves {
        starts.push(at);
        if let PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } = *mv {
            at = to;
        }
    }
    let mut count = vec![0usize; path.moves.len()];
    for s in &samples {
        count[s.mv] += 1;
    }
    let z = tool.flutes.max(1) as f64;
    let mut cuts = Vec::new();
    let mut max_cut_feed: f64 = 0.0;
    let mut prev_engaged = false;
    for s in &samples {
        let engaged = s.engage.is_some();
        if let (true, false, PMove::Line { f, .. } | PMove::Arc { f, .. }) = (engaged, prev_engaged, path.moves[s.mv]) {
            entries.push(Entry::Contact { mv: s.mv, feed: f });
        }
        prev_engaged = engaged;
        let Some(e) = s.engage else { continue };
        let mv = &path.moves[s.mv];
        let f = match *mv {
            PMove::Line { f, .. } | PMove::Arc { f, .. } => f,
            _ => continue,
        };
        if !(pos(f) && pos(rpm)) {
            continue;
        }
        max_cut_feed = max_cut_feed.max(f);
        let len = move_len(starts[s.mv], mv) / count[s.mv].max(1) as f64;
        let fz = f / (z * rpm);
        let span = e.th_hi - e.th_lo;
        let mean_cos = if span > 1e-12 {
            ((forge_math::dmath::sin(e.th_hi) - forge_math::dmath::sin(e.th_lo)) / span).clamp(0.0, e.cos_max)
        } else {
            e.cos_max
        };
        let d_eff = effective_diameter(tool.d, tool.ball, e.ap);
        cuts.push(Cut {
            op,
            path: 0,
            mv: Some(s.mv),
            line: None,
            minutes: len / f,
            vc: std::f64::consts::PI * d_eff * rpm / 1000.0,
            fz,
            hex: fz * e.cos_max,
            hm: fz * mean_cos,
            f_rev: fz * z,
            ap: e.ap,
        });
    }
    let mut unsampled = 0.0;
    let mut cycles = 0;
    for (i, mv) in path.moves.iter().enumerate() {
        match *mv {
            PMove::Line { to, f, phase }
                if count[i] == 0 && phase != Phase::Rapid && to[2] < starts[i][2] && f > 0.0 =>
            {
                unsampled += move_len(starts[i], mv) / f;
            }
            PMove::Arc { to, f, .. } if count[i] == 0 && to[2] < starts[i][2] && f > 0.0 => {
                unsampled += move_len(starts[i], mv) / f;
            }
            PMove::Cycle { .. } => cycles += 1,
            _ => {}
        }
    }
    Measured {
        cuts,
        unsampled_minutes: unsampled,
        cycles,
        entries,
        max_cut_feed,
    }
}

/// x > 0 (false for NaN).
fn pos(x: f64) -> bool {
    x > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_through_a_block() {
        // a 10 mm flat end slotting 2 mm deep through 50 mm of stock at F 600, 6000 rpm, 3 flutes
        let mut map = HeightMap::new([-20.0, -20.0, -10.0], [70.0, 20.0, 0.0], 0.25);
        let prof = Profile::Bull { r: 5.0, re: 0.0 };
        let path = OpPath {
            tool: 0,
            start: [-15.0, 0.0, 5.0],
            moves: vec![
                PMove::Line {
                    to: [-15.0, 0.0, -2.0],
                    f: 300.0,
                    phase: Phase::Cut,
                },
                PMove::Line {
                    to: [65.0, 0.0, -2.0],
                    f: 600.0,
                    phase: Phase::Cut,
                },
            ],
            rpm: 6000.0,
            coolant: forge_post::Coolant::Off,
        };
        let m = measure(
            0,
            &path,
            &mut map,
            &prof,
            ToolGeom {
                d: 10.0,
                flutes: 3,
                ball: false,
            },
            10.0,
            0.25,
            0.0,
        );
        // the plunge (the map's box is all stock): an entry, at half the cutting feed
        assert!(m.entries.contains(&Entry::Plunge { mv: 0, feed: 300.0 }), "{:?}", m.entries);
        assert_eq!(m.max_cut_feed, 600.0);
        // 7 mm at 300 → not sampled, counted
        assert!((m.unsampled_minutes - 7.0 / 300.0).abs() < 1e-12);
        let mins: f64 = m.cuts.iter().map(|c| c.minutes).sum();
        // engaged from x ≈ -5 to 50 plus the tool's exit: between 50 and 70 mm of the 80 mm move
        assert!(mins > 50.0 / 600.0 && mins <= 80.0 / 600.0, "{mins}");
        let mid = m.cuts[m.cuts.len() / 2];
        assert!((mid.fz - 600.0 / (3.0 * 6000.0)).abs() < 1e-12);
        assert!((mid.ap - 2.0).abs() < 0.3, "ap {}", mid.ap);
        assert!((mid.vc - std::f64::consts::PI * 10.0 * 6.0).abs() < 1e-9);
        // a full slot: peak chip = fz, mean chip ≈ 2 fz/π
        assert!((mid.hex - mid.fz).abs() < 0.02 * mid.fz, "hex {}", mid.hex);
        assert!(
            (mid.hm / mid.fz - 2.0 / std::f64::consts::PI).abs() < 0.05,
            "hm/fz {}",
            mid.hm / mid.fz
        );
    }
}
