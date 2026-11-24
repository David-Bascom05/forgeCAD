//! Helical boring: a round hole opened to size by a helix down its wall.
//!
//! Method (plain, textbook helical interpolation): the tool centre on the circle of the hole's
//! radius less the tool's (less any stock left for a finishing pass), counter-clockwise (G3:
//! climb inside with M03), down at `pitch` per turn from the hole's top to its bottom, then a full
//! circle at the bottom to clean the floor's ramp, back to the axis and up. A finishing pass
//! repeats the helix at the final radius; spring passes repeat that. The tool must reach the
//! axis from its circle (circle radius ≤ tool radius), so no core is left standing.
use crate::ops::{Cut, OpError, OpPath, PMove, Phase};

/// A helical bore's passes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bore {
    /// Down per turn of the helix.
    pub pitch: f64,
    /// Radial stock the first helix leaves for a finishing helix (0: one helix only).
    pub finish_leave: f64,
    /// The last helix again, this many times.
    pub spring: usize,
    /// Radial stock left on the wall by the last pass.
    pub leave: f64,
}

/// The helices at `c` from `top` down to `bottom` opening a hole of radius `r_hole`.
pub fn helical_bore(
    c: [f64; 2],
    r_hole: f64,
    top: f64,
    bottom: f64,
    cut: Cut,
    b: Bore,
) -> Result<OpPath, OpError> {
    let Cut {
        tool,
        tidx,
        h,
        feed,
        ramp_feed,
        ..
    } = cut;
    // the helix down is a ramp: FEEDS-SPEEDS' ramp feed where given
    let ramp_f = ramp_feed.unwrap_or(feed.0);
    let rt = 0.5 * tool.tool.dc;
    let rc = r_hole - rt - b.leave;
    if rc <= 0.0 {
        return Err(OpError::ToolTooBig {
            tool_d: tool.tool.dc,
            why: format!("the tool does not fit the hole ({:.3} mm across)", 2.0 * r_hole),
        });
    }
    let r0 = rc - b.finish_leave.max(0.0);
    if rc > rt || r0 <= 0.0 {
        return Err(OpError::Region(format!(
            "a core of {:.3} mm would stand at the hole's axis: the tool ({} mm) must be at least half the hole across, or pocket(..) the hole",
            2.0 * (rc - rt).max(0.0),
            tool.tool.dc
        )));
    }
    let mut radii = vec![];
    if b.finish_leave > 0.0 {
        radii.push(r0);
    }
    radii.push(rc);
    for _ in 0..b.spring {
        radii.push(rc);
    }
    let turns = ((top - bottom) / b.pitch - 1e-9).ceil().max(1.0) as usize;
    let mut mv = vec![];
    for &r in &radii {
        mv.push(PMove::Rapid {
            to: [c[0] + r, c[1], h.retract],
        });
        mv.push(PMove::Line {
            to: [c[0] + r, c[1], top],
            f: feed.1,
            phase: Phase::Plunge,
        });
        let quarter = |k: usize| match k % 4 {
            1 => [c[0], c[1] + r],
            2 => [c[0] - r, c[1]],
            3 => [c[0], c[1] - r],
            _ => [c[0] + r, c[1]],
        };
        // down the helix, then once round at the bottom
        for k in 1..=4 * (turns + 1) {
            let p = quarter(k);
            let z = if k >= 4 * turns {
                bottom
            } else {
                top - (top - bottom) * k as f64 / (4 * turns) as f64
            };
            mv.push(PMove::Arc {
                to: [p[0], p[1], z],
                c,
                cw: false,
                f: if k > 4 * turns { feed.0 } else { ramp_f },
            });
        }
        mv.push(PMove::Line {
            to: [c[0], c[1], bottom],
            f: feed.0,
            phase: Phase::Cut,
        });
        mv.push(PMove::Line {
            to: [c[0], c[1], h.retract],
            f: feed.0,
            phase: Phase::Link,
        });
    }
    mv.push(PMove::Rapid {
        to: [c[0], c[1], h.clear],
    });
    Ok(OpPath {
        tool: tidx,
        start: [c[0] + radii[0], c[1], h.clear],
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    })
}
