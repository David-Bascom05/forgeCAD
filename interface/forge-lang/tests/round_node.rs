//! A round too big for one fillet (fg_mcp's plate: its top face's four edges and a dent's rim),
//! built as two rounds the part takes together (forge-model's `overlapping_rounds`): the two
//! blends' springs cross on the top face, where the blends touch (forge-geom's node.rs). Before the
//! node was mended, about one radius in twenty between 10.9 and 24.3 was refused (SplitCrossed,
//! InconsistentClassification, ZeroThickness), scattered, so "the largest radius that builds" had no
//! meaning: research/curved-sites/blend-node.md.

const PLATE: &str = "shared L = 60
shared t = 6

sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)

param mark.d = 4
param mark.cy = 40
sketch mark(on: base.end) {
  c = circle((30, cy), d)
}
param dent.depth = 2
dent = cut(mark, depth)
";

/// The plate with its top face's edges rounded at `r`: the part's volume, or the refusal.
fn rounded(r: f64) -> Result<f64, String> {
    let src = format!("{PLATE}param round.r = {r}\nround = fillet(edges(of: base.end), r)\n");
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    if let Some(e) = run.errors.first() {
        return Err(format!("{e:?}"));
    }
    let body = run.body.ok_or("no body")?;
    let problems = forge_topo::check(&body);
    if !problems.is_empty() {
        return Err(format!("{problems:?}"));
    }
    forge_topo::mass::mass(&body)
        .map(|m| m.volume)
        .map_err(|e| format!("{e:?}"))
}

/// Every radius the sweeps found refused before (step 0.01 from 10.9 to 13.5, 0.1 to 30), each
/// built, checked and of a volume.
#[test]
fn every_radius_once_refused_builds() {
    let once = [
        10.98, 11.09, 11.1, 11.15, 11.22, 11.44, 11.53, 11.61, 11.7, 11.9, 12.06, 12.07, 12.08, 12.12, 12.13,
        12.14, 12.18, 12.31, 12.4, 12.48, 12.5, 12.64, 12.75, 12.91, 12.92, 12.98, 13.0, 13.08, 13.35, 14.9,
        15.1, 16.7, 23.3, 24.3,
    ];
    let refused: Vec<String> = once
        .iter()
        .filter_map(|&r| rounded(r).err().map(|e| format!("{r}: {e}")))
        .collect();
    assert!(refused.is_empty(), "{refused:#?}");
}

/// From the first radius the one fillet refuses to the corners' limit, every radius on a 0.1 grid
/// builds, and the volume falls smoothly with it: strictly, each step's fall within 5% of the
/// last's (measured: within 1%). A blend region lost or doubled at a node would jump.
#[test]
fn the_volume_falls_smoothly_through_the_two_rounds_range() {
    let rs: Vec<f64> = (0..=143).map(|i| 10.0 + 0.1 * i as f64).collect();
    let vs: Vec<f64> = rs
        .iter()
        .map(|&r| rounded(r).unwrap_or_else(|e| panic!("r = {r}: {e}")))
        .collect();
    let d: Vec<f64> = vs.windows(2).map(|w| w[1] - w[0]).collect();
    for (i, x) in d.iter().enumerate() {
        assert!(
            *x < 0.0,
            "volume rose from r = {} to {}: {:?}",
            rs[i],
            rs[i + 1],
            &vs[i..i + 2]
        );
    }
    for i in 1..d.len() {
        assert!(
            (d[i] - d[i - 1]).abs() <= 0.05 * d[i - 1].abs(),
            "a jump at r = {}: falls {} then {}",
            rs[i],
            d[i - 1],
            d[i]
        );
    }
}

/// Past the corners' limit the round is refused at every radius (Mitre: the two blends at each
/// corner of the plate cut each other where no patch can follow): acceptance is monotone.
#[test]
fn past_the_corners_limit_every_radius_is_refused() {
    for r in [24.4, 25.0, 27.3, 30.0] {
        let e = rounded(r).expect_err("refused");
        assert!(e.contains("Mitre"), "r = {r}: {e}");
    }
}
