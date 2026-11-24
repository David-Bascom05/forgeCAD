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

#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub cuts: Vec<Cut>,
    /// Minutes of feed moves down (plunges, ramps) that were not sampled.
    pub unsampled_minutes: f64,
    /// Canned cycles in the path (not in the estimate).
    pub cycles: usize,
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
    for s in &samples {
        let Some(e) = s.engage else { continue };
        let mv = &path.moves[s.mv];
        let f = match *mv {
            PMove::Line { f, .. } | PMove::Arc { f, .. } => f,
            _ => continue,
        };
        if !(pos(f) && pos(rpm)) {
            continue;
        }
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
        // the plunge outside the stock: 7 mm at 300 → not sampled, counted
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
