# CAM Phase 0: result and go/no-go (2026-11-23)

Branch claude/repo-review-mbdrlj-cam. Crate `kernel/forge-cam-geom` (no dependencies), plus the
lowering in `kernel/forge-cam`. Bench: `cargo run --release -p forge-lang --example cam_gonogo`.

## What was built

| piece | file | how it is exact | tests |
|---|---|---|---|
| exact reals | big.rs, real.rs | hash-consed DAG over f64 inputs; sign by f64-interval filter, then big-float intervals to the BFMSS separation bound; `x − x` is 0 by construction | ties (√2·√2 − 2, three radicals), near-ties |
| regions | region.rs | lines + arcs (exact rational centres on the ends' bisector), validated exactly: closed, no crossing, no spike, sense vs nesting by exact ray casting | valid/invalid shapes |
| offset R ⊖ B_r (and R ⊕ B_r) | offset.rs | one corner rule; every event an exact meet; keep test at rational points; slot = 2r gives the there-and-back path, a tool-sized hole gives its centre point | 6 cases + L6 fuzz: 60 random regions (islands, in and out), 24 000 points against the definition |
| tools | tool.rs | flat, ball, bull, chamfer, drill (and envelopes) + shank + holder as revolved line/arc profiles; signed distance as an interval extension, no assumed error budget | closed-form values |
| checker, general | check.rs, dual.rs | interval B&B over face params × motion (forge-brake's contract, Clear only if proven ≥ limit); mean-value forms; planar moves closed form in the motion | tangent passes clear, 0.01 mm gouges found with witnesses |
| checker, 2.5D | slab.rs | for prism faces (horizontal planes, rectangular vertical walls, vertical cylinders) and planar moves: sd is non-decreasing in ρ, so min = sd(D, Δ), D a closed-form 2D curve distance, Δ a 1D search | L6: 400 random cases against the B&B, never contradicting, both verdicts exercised |
| lowering | forge-cam lower.rs | planes keep their frame, trims snap to the B-rep's vertices; cylinders by the rational half-angle map; mu = tol::LINEAR + rounding on every face; other surfaces refused by kind | flange lowers whole |

## The measurement

flange.fg (13 B-rep faces, 19 checker faces), finishing contours from the exact offset (plate
outside, boss outside, bore inside), every move subdivided; cutter limit −0.01 mm, shank and
holder clearance +0.1 mm; release build, one core.

| method | moves | near checks / move | µs / move | 10⁵ moves | refused |
|---|---|---|---|---|---|
| general B&B only | 280 | 12 | 100 000–250 000 | hours | 0 |
| closed form (2.5D) | 980 | 3.1 | 32 | — | 0 |
| closed form (2.5D) | 9 996 | 3.1 | 37 | — | 0 |
| closed form (2.5D) | 99 988 | 3.1 | 32 | **3.2 s** | 0 |

Bull 6 r1 is the same to within 3%.

## Verdict

- **No-go** for the general B&B as the 2.5D checker: 3–4 orders of magnitude too slow at
  tangency (every finishing pass is tangent along its length).
- **Go (conditional)** for the pre-registered alternative, closed-form 2.5D distance: 3.2 s for
  10⁵ moves, linear in moves. That is above the 1 s line and inside the 10 s line: ops up to about
  3·10⁴ moves certify inline (≤ 1 s); larger ones certify in the background and show
  `undecided: pending`, never `pass`, until done. The B&B stays for faces outside the prism
  classes (tilted planes, cones, tori) and non-planar moves (ramps, helices), where it is the only
  method and where moves are few.
- Next speedups (not needed for the go): a uniform grid over face boxes (the per-move face loop is
  now a large share), and reusing the classification of a path start between consecutive moves.

## Levels reached (PROOF-STANDARD.md)

- L1: not yet (the Lean statements G1–G6 are next; the monotonicity lemma "sd of a ρ-hypograph
  body is non-decreasing in ρ" is the one new fact the 2.5D form rests on, proof sketch in slab.rs).
- L2: the offset's decisions are exact (the exact reals); the checker is certified-interval, not
  exact, by design.
- L4: forge-cam-geom is written in the Aeneas style (loops, no iterator chains in the algorithms,
  no u128) but not yet translated; blockers expected: HashMap in the hash-consing, generics in
  dual.rs.
- L5: every float operation in the checker rounds outward (ival.rs); its correctness against
  verify/ieee754 is the next proof.
- L6: the offset fuzz and the closed-form vs B&B differential, both in CI-able tests.
