//! Adaptive clearing clears the same stock whatever the stock box's numbers: a round box (its
//! corner and steps on the grids the planner samples) and boxes a thousandth or half a millimetre
//! off it leave nothing above the floor. Once a pass's start beside a stock corner decided, by
//! where the grids fell, whether it ran the whole face or stalled at once, and a round box left
//! half the clearing undone (verify/forge-cam-quality's equal-work guard caught it).
//!
//! Each case runs a deep_boss document (adaptive one-way or two-way) with the
//! stock box swapped, then sweeps the posted facing and roughing paths through a fine model of
//! the stock and measures what stands above the roughing floor away from the boss and the
//! stock's edge, where every pass and ring reaches.
mod common;
use forge_cam::engage::{HeightMap, Profile};
use forge_cam::ops::PMove;
use std::path::Path;

/// Sweeps a → b in pieces of at most 2 mm: the model visits a segment's whole box, so a long
/// diagonal link or rapid would visit most of it for nothing; the pieces sweep the same.
fn sweep_short(map: &mut HeightMap, a: [f64; 3], b: [f64; 3], prof: &Profile) {
    let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let n = (l / 2.0).ceil().max(1.0) as usize;
    let mut p = a;
    for k in 1..=n {
        let s = k as f64 / n as f64;
        let q = if k == n {
            b
        } else {
            [
                a[0] + s * (b[0] - a[0]),
                a[1] + s * (b[1] - a[1]),
                a[2] + s * (b[2] - a[2]),
            ]
        };
        map.sweep(p, q, prof);
        p = q;
    }
}

const BOXES: [([f64; 3], [f64; 3]); 5] = [
    ([-3.0, -3.0, 0.0], [73.0, 73.0, 26.0]),
    ([-2.999, -2.999, 0.0], [72.999, 72.999, 26.0]),
    ([-3.001, -3.001, 0.0], [73.001, 73.001, 26.0]),
    ([-2.5, -2.5, 0.0], [72.5, 72.5, 26.0]),
    ([-3.5, -3.5, 0.0], [73.5, 73.5, 26.0]),
];

/// The stock left above `floor` after the doc's facing and roughing, in mm³, over the cells more
/// than `keep` from the boss (its square, 20..50) and the stock's edge.
fn left_above(
    dir: &Path,
    doc: &str,
    min: [f64; 3],
    max: [f64; 3],
    floor: f64,
    keep: f64,
) -> (f64, Vec<String>) {
    let src = std::fs::read_to_string(dir.join(doc)).unwrap().replace(
        "box(margin: (3, 3, 1))",
        &format!(
            "box(min: ({}, {}, {}), max: ({}, {}, {}))",
            min[0], min[1], min[2], max[0], max[1], max[2]
        ),
    );
    let cam = forge_cam_lang::run(&src, dir);
    assert!(cam.ok || cam.errors.is_empty(), "{doc} {min:?}: {:?}", cam.errors);
    let h = 0.1;
    let mut map = HeightMap::new(min, max, h);
    for name in ["skim", "rough"] {
        let o = cam
            .ops
            .iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{doc}: no {name}"));
        assert_eq!(o.status, "built", "{doc} {min:?} {name}: {:?}", o.refusal);
        let tu = &cam.tool_uses()[o.tool.unwrap()];
        let prof = Profile::of(&tu.tool).unwrap();
        for p in &o.paths {
            let mut at = p.start;
            for m in &p.moves {
                match *m {
                    PMove::Rapid { to } | PMove::Line { to, .. } => {
                        sweep_short(&mut map, at, to, &prof);
                        at = to;
                    }
                    PMove::Arc { to, c, cw, .. } => {
                        // chords of at most 0.05 mm sag, the height linear in the angle
                        let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                        let a0 = (at[1] - c[1]).atan2(at[0] - c[0]);
                        let mut a1 = (to[1] - c[1]).atan2(to[0] - c[0]);
                        if cw {
                            while a1 >= a0 {
                                a1 -= std::f64::consts::TAU;
                            }
                        } else {
                            while a1 <= a0 {
                                a1 += std::f64::consts::TAU;
                            }
                        }
                        let n = (((a1 - a0).abs() * r) / 0.5).ceil().max(1.0) as usize;
                        let mut prev = at;
                        for k in 1..=n {
                            let s = k as f64 / n as f64;
                            let a = a0 + s * (a1 - a0);
                            let q = if k == n {
                                to
                            } else {
                                [
                                    c[0] + r * a.cos(),
                                    c[1] + r * a.sin(),
                                    at[2] + s * (to[2] - at[2]),
                                ]
                            };
                            sweep_short(&mut map, prev, q, &prof);
                            prev = q;
                        }
                        at = to;
                    }
                    _ => {}
                }
            }
        }
    }
    let mut left = 0.0;
    for j in 0..map.ny {
        let y = map.y0 + (j as f64 + 0.5) * h;
        for i in 0..map.nx {
            let x = map.x0 + (i as f64 + 0.5) * h;
            let away_boss = x < 20.0 - keep || x > 50.0 + keep || y < 20.0 - keep || y > 50.0 + keep;
            let away_edge = x > min[0] + keep && x < max[0] - keep && y > min[1] + keep && y < max[1] - keep;
            let z = map.z[j * map.nx + i];
            if away_boss && away_edge && z > floor + 0.01 {
                left += (z - floor) * h * h;
            }
        }
    }
    let notes = cam
        .warnings
        .iter()
        .filter(|w| w.contains("left uncut"))
        .cloned()
        .collect();
    (left, notes)
}

/// One document and box: at most a cell-scale patch left (a 1 mm² patch through the whole
/// 14.8 mm depth), and no "left uncut" report.
fn cleared(doc: &str, k: usize) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../verify/forge-cam-quality/parts");
    let (min, max) = BOXES[k];
    // the roughing floor 10.2; within 0.5 mm of the boss and the stock's edge is the wall
    // passes' and the fine model's rounding
    let (left, notes) = left_above(&dir, doc, min, max, 10.2, 0.5);
    assert!(
        left <= 15.0 && notes.is_empty(),
        "{doc} {min:?}: {left:.0} mm³ left; {notes:?}"
    );
}

// One test per case, so they run in parallel. The p-group documents plan the same paths as the
// n-group ones (only their feeds differ): the round box, the case that failed, is run for both.
macro_rules! cases {
    ($($name:ident: $doc:literal, $k:literal;)*) => {
        $(
            #[test]
            fn $name() {
                cleared($doc, $k);
            }
        )*
    };
}

cases! {
    n_adaptive_round: "deep_boss_n_adaptive_cam.fg", 0;
    n_adaptive_under_a_thousandth: "deep_boss_n_adaptive_cam.fg", 1;
    n_adaptive_over_a_thousandth: "deep_boss_n_adaptive_cam.fg", 2;
    n_adaptive_under_a_half: "deep_boss_n_adaptive_cam.fg", 3;
    n_adaptive_over_a_half: "deep_boss_n_adaptive_cam.fg", 4;
    n_zigzag_round: "deep_boss_n_zigzag_cam.fg", 0;
    n_zigzag_under_a_thousandth: "deep_boss_n_zigzag_cam.fg", 1;
    n_zigzag_over_a_thousandth: "deep_boss_n_zigzag_cam.fg", 2;
    n_zigzag_under_a_half: "deep_boss_n_zigzag_cam.fg", 3;
    n_zigzag_over_a_half: "deep_boss_n_zigzag_cam.fg", 4;
    p_adaptive_round: "deep_boss_p_adaptive_cam.fg", 0;
    p_zigzag_round: "deep_boss_p_zigzag_cam.fg", 0;
}
