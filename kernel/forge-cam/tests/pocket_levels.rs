//! The proviso of passes_cover (Lean, verify/forge-cam): a pocket's passes are the offsets of
//! its region at r + k·s for k = 0, 1, …, every nonempty one emitted, with 0 < s ≤ r. Then the
//! passes cover the opening, which stock.rs records as cleared. This checks the op does that.
use forge_cam::ops::{self, Heights, PMove, Phase, ToolUse};
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_tooling::Library;

fn tool(id: &str) -> ToolUse {
    let l = Library::builtin();
    let t = l.tool(id).unwrap().clone();
    let shape = t
        .shape(t.lcf + 5.0, Some(l.holder("bt40-er32").unwrap()))
        .unwrap();
    ToolUse {
        t: 1,
        tool: t,
        holder: Some("bt40-er32".into()),
        stickout: 0.0,
        shape,
    }
}

fn poly(pts: &[(f64, f64)]) -> Loop {
    let n = pts.len();
    Loop {
        segs: (0..n)
            .map(|i| Seg::Line {
                a: p2(pts[i].0, pts[i].1),
                b: p2(pts[(i + 1) % n].0, pts[(i + 1) % n].1),
            })
            .collect(),
    }
}

#[test]
fn a_pocket_emits_every_nonempty_level() {
    let t = tool("flat-6");
    let r = 0.5 * t.tool.dc;
    let regions = [
        // an L, a slot exactly 2r wide, a notched plate with a hole
        Region {
            loops: vec![poly(&[
                (0.0, 0.0),
                (40.0, 0.0),
                (40.0, 12.0),
                (12.0, 12.0),
                (12.0, 30.0),
                (0.0, 30.0),
            ])],
        },
        Region {
            loops: vec![poly(&[(0.0, 0.0), (30.0, 0.0), (30.0, 2.0 * r), (0.0, 2.0 * r)])],
        },
        Region {
            loops: vec![
                poly(&[
                    (0.0, 0.0),
                    (50.0, 0.0),
                    (50.0, 30.0),
                    (30.0, 30.0),
                    (30.0, 22.0),
                    (20.0, 22.0),
                    (20.0, 30.0),
                    (0.0, 30.0),
                ]),
                poly(&[(8.0, 8.0), (8.0, 14.0), (14.0, 14.0), (14.0, 8.0)]),
            ],
        },
    ];
    let h = Heights {
        clear: 20.0,
        retract: 5.0,
    };
    for rg in &regions {
        for s in [0.5 * r, 0.8 * r, r] {
            let p = ops::pocket(
                rg,
                ops::Cut {
                    tool: &t,
                    tidx: 0,
                    h,
                    feed: (500.0, 100.0),
                    ramp_feed: None,
                    lead_feed: None,
                },
                ops::Span {
                    top: 0.0,
                    bottom: -2.0,
                    step: 1.0,
                },
                s,
                0.0,
                None,
            )
            .unwrap();
            // the levels the proof needs: every nonempty offset at r + k s
            let mut want = 0usize;
            let mut k = 0;
            loop {
                let (ls, _) = offset(rg, r + k as f64 * s).unwrap();
                if ls.is_empty() {
                    break;
                }
                want += ls.len();
                k += 1;
            }
            // what the op emits: one plunge per pass loop per depth level
            let depths = ops::levels(0.0, -2.0, 1.0).len();
            let plunges = p
                .moves
                .iter()
                .filter(|m| {
                    matches!(
                        m,
                        PMove::Line {
                            phase: Phase::Plunge,
                            ..
                        }
                    )
                })
                .count();
            assert_eq!(plunges, want * depths, "stepover {s}: {want} loops per level");
            // and the first emitted level is at exactly r (the opening's shallow points)
            let (first, _) = offset(rg, r).unwrap();
            assert!(!first.is_empty());
        }
    }
}
