# Exact mass properties: what the speed choices rest on

`crates/forge-topo/src/mass.rs` integrates each face by Green's theorem on its uv loops, or a
corner patch's face over its domain polygon directly. The choices below were measured with
`masstime.rs` here (copy it to `crates/forge-agent/examples/` to build it; it is not part of the
workspace) and the two scripts, run from the repository root through `fbuild`.

## How to rerun

```
cp research/mass-props/masstime.rs crates/forge-agent/examples/
fbuild cargo build --release -p forge-agent --example masstime
# the same harness built from the tree to compare against, e.g. main in a scratch worktree
research/mass-props/table.sh BASE target/release/examples/masstime torture tour heatsink ...
research/mass-props/count.sh BASE target/release/examples/masstime torture tour ...
target/release/examples/masstime fillets        # the fillet corpus
REGEN=1 target/release/examples/masstime ball-rod-pipe   # regeneration time (booleans)
```

`table.sh` prints, per body, the exact mass's microseconds before and after (the least of two
interleaved runs, each the least of several single calls: other loads only ever add), the
mesh's, and both volumes with their relative error bounds. Wall times on a shared machine move
by tens of percent from run to run; `count.sh` counts instructions instead (deterministic).

## Choices and the measurements behind them

- Corner patches (`whole_patch`, `HolePatch::domain_rule`): a face that is its whole domain
  polygon is integrated over it, each side's triangle to the centre halved and each half
  collapsed onto its corner (Duffy), m Gauss points each way. On the heatsink's 48 five-sided
  patches the volume integrand's relative change from m = 8 to 16 was about 3e-8 and from 16 to
  32 about 1e-14: geometric convergence, about 0.155 per point. So 12 points reach about 2e-11,
  below the 1e-10 a face needs, and 12 against 16 settles almost every face (32 is the
  fallback; 12 against 20 cost a quarter more for nothing). The rational quads converge more slowly within a cell (12 to 32 differ by 2.5e-8
  on the box corner's): the 32 fallback and its reported error cover them.
- Procedural curve edges (`coedge_pieces`) start in pieces bending about a quarter of a circle:
  a chord over a quarter circle sags 0.19 of its length, and a curve's `segments(range, chord)`
  goes as 1 / sqrt(chord), so n = s^2 * 1e-4 / 0.19 with s = segments(range, 1e-4 l). On a
  12-turn helix that gives 50 pieces of 1.5 radians (bisecting from one piece wasted as many
  nodes as it kept).
- Swept surfaces (pipes, variable blends) integrate along v per node by Clenshaw-Curtis on 17
  then 33 points, one section per column: on the variable-blend cases a section is about 15k
  instructions and a point on it a few hundred, so per-node columns beat a Chebyshev fit's
  sections (each fit point along u is a section).
- Pipe spines walk by chord distance in stretches turning under 30 degrees (as the intersection
  edges in mass.rs already did): within a stretch the parametrisation is as smooth as the
  surfaces, so Chebyshev tiles and Gauss-Kronrod pieces no longer break at every chain point
  (46 to 82 per pipe face before).

## Results

See `results.txt` (written by the scripts on 2026-10-31 against main 2059f0c): instruction
counts (deterministic) and wall times, per part and per body, gate, bench and fillet parts.

## Tried and dropped

- Gauss-Kronrod with exact derivatives on today's corner patch (`HolePatch::exact_d1`): the
  ribbon blend's derivatives turn with direction at the corners, the 7-15 difference
  underestimated the error there (one face off by 0.6 with a bound of 0.06). The domain rule
  replaced the loop integral for whole patches; trimmed ones keep the interpolated path.
- Clenshaw-Curtis before Gauss-Kronrod on every surface: on corner patches it spent 33
  evaluations per node and failed, and its panels were not charged to the face's budget, so
  refinement ran on (5x slower). It is used on swept surfaces only.
