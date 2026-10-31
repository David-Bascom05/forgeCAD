# Corner patch: rational quads and their fold proof

How the rational corner quads (`crates/forge-geom/src/corner/rational.rs`, `close_hole_rational`,
`plans` and `Cells` in `corner.rs`) were chosen and measured. The analysis and all results are
in `docs/fold-proof.md` (stages 0 to 4); this folder holds the code that produced them.

## What was derived, and where the code uses it

- **The construction** (quads round a centre, ribbons of degree 3 in s and 2 in d, weights
  prod d_j^2, blend degree (7, 7), cross tangents (a Q + b F) / D with F of degree <= 2, and
  |Q| proportional to D for a circle so (n D x Q) / D is quadratic): derived by hand, written
  out in `docs/fold-proof.md` stage 4 and in the module doc of `rational.rs`. Checked by
  `rq.rs`'s audits (boundary, G1 normal, derivatives, projection, boxes).
- **Cubic spokes with equal control lengths (a third of the way)**, leaving along the halving
  direction (`spokes` in `corner.rs`): `rq.rs` on the survey. Parabolic spokes left near a
  side for 8+ sides (builds refused); control lengths fitted to today's patch folded a third of
  the 8-sided holes (`FOLDS=1` prints each quad's proof, sampled fold and corner angles).
- **One face: where spokes leave, the domain, projection** (`plans`, `HolePatch::from_quads`,
  `Cells` in `corner.rs`; docs/fold-proof.md stage 4 "One face per corner"): `sharp.rs`
  surveys corners with sharp edges (odd holes and kinked vertices) and audits the one-face
  patch (boundary, G1 normal, projection, derivatives, boxes); `rq.rs` the same on the
  all-fillet survey. Per-cell seeds and Newton from several cells' seeds came from projection
  failures they printed (PROJ=1).
- **The centre from today's patch, the old heuristic beyond 12 sides**: `rq.rs` (gap to
  today's 19% with the heuristic centre on 6 sides, 2% with today's; beyond 12 sides the
  heuristic proves more).
- **The proof's parameters and regions** (`CORNER_BOX` = 1/4, `blend_regions` and
  `corner_box`: the middle whole, strips halved, Duffy triangles on each corner's eighth, halves
  along u and v in turn, phi biquadratic for plain boxes and bilinear in Duffy triangles):
  instruction counts with `ic.sh` and test counts from `rq.rs`, trying each alternative
  (docs/fold-proof.md stage 4 lists the ones dropped).
- **`HolePatch::min_radius` is zero round a kinked corner**: `kmin.rs` (dense 300 x 300
  curvature sampling of the fillet corpus' corner patches).
- **What switching builds would cost whole parts** (docs/fold-proof.md stage 4 "Switching
  builds"): `switch.patch` (against the tree before the switch, 69981df) routes
  `forge_ops::corner::patch` through `close_hole_rational` when CORNER_RATIONAL is set (falling
  back to `close_hole`) and logs each hole's path with CORNER_LOG; `corpus.rs` regenerates one
  fillet-corpus case; `switch.sh` counts both. Since the switch, builds take that path always:
  the switched column is `corpus.rs` under `perf stat` on the current tree.
- **The spread guard at the centre** (`spokes` in `corner.rs` refuses cubic spokes leaving the
  centre within a quarter of an even share of the turn of each other): `spoke-gap.patch` prints
  each plan's smallest angle between consecutive spokes there (SPOKEGAP=1) for the corpus
  (`corpus.rs`), `rq.rs` (with PER=1, beside each hole's kmax rational/today) and `sharp.rs`;
  the curvature of the corpus faces was sampled with a 400 x 400 grid of normal curvatures
  (|P_uu . n| / |P_u|^2 and the same in v) over each procedural face. docs/fold-proof.md stage 4
  "Switching builds" has the numbers.
- **The stage 1 n-gon rational patch** (superseded by the quads, kept in the record):
  `rational_compare.rs`, the stage 1 table in `docs/fold-proof.md`.

## How to rerun

The programs are examples; copy them into a crate's `examples/` to build (they are not built
with the workspace):

```
cp research/corner-patch/rq.rs crates/forge-ops/examples/
fbuild cargo build --release -p forge-ops --example rq
./target/release/examples/rq 60 3 0     # CASES EDGES PLAN: survey 3-, 4-, 5-edge corners
                                        # (EDGES 3..5) or EDGES 6..10 on their own; PLAN picks
                                        # the way spokes leave for the per-quad measurements
# env: NOSHAPE=1 (skip shape), PER=1 (per hole), FOLDS=1, KLOC=1 (curvature peaks),
#      PROJ=1 (projection failures), PROOFONLY=2 (proof timing),
#      COUNT=today|build|proof|close|cold (close: today's close_hole; cold: close_hole_rational)
sh research/corner-patch/ic.sh proof 3  # instructions per hole for one kind of work

cp research/corner-patch/sharp.rs crates/forge-ops/examples/
fbuild cargo build --release -p forge-ops --example sharp
./target/release/examples/sharp 60 1    # CASES SHARP: SHARP neighbouring edges of each corner

cp research/corner-patch/kmin.rs crates/forge-model/examples/
fbuild cargo build --release -p forge-model --example kmin
./target/release/examples/kmin two-blends-sharp-edge
```

```
git apply research/corner-patch/switch.patch
cp research/corner-patch/corpus.rs crates/forge-model/examples/
fbuild cargo build --release -p forge-model --example corpus --example regen1
sh research/corner-patch/switch.sh      # per case: regen M instructions today, switched, ratio, paths
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  CORNER_LOG=1 ./target/release/examples/regen1 $p seq 2>&1 | grep -c '^corner today'; done
```

`rational_compare.rs` ran on `refs/experiments/corner-rational-stage1` (4b81f12); check that
ref out to rerun it (`rational_compare 60 3`).

## Dead ends (nothing uses them)

- **Taylor models** (`refs/experiments/corner-fold-taylor-models`): sound, 0.25-6 s per patch;
  the remainders of 1/x and x^-1/2 compositions, not truncation, refuse. Stage 2 of the doc.
- **Mean-value regularity proof** (`refs/experiments/corner-fold-mean-value`): sound but refused
  nearly every survey patch: corner disks covered ~0.4% of the domain and the Hessian bounds
  came out 10-170x the true ones near corners.
- **Corner-sector interpolant with a first-order interval proof**
  (`refs/experiments/corner-blend-interval-proof`): worked but took 45272 cells / 345 ms for
  one octant patch, and changed the patch so 10 forced-split folds slipped past the sampled
  check.
- **The n-gon rational patch with full Jacobian Bernstein coefficients** (stage 3): sound,
  ~40 ms (degree 59); replaced by the quads and the cone test.
- Within the cone test: cones of P_x = P_sigma - t P_t / sigma (cancels coefficient-wise, wider),
  Duffy boxes of half the square (more tests), splits in four (more tests than halves).
