# Should the geometry kernels be merged?

EXACT-CORE, 2026-11-24, from origin/main d20dddfa. Read-only survey: code, research/*, verify/*.
References are file:line.

## Answer in one paragraph

The crates are not several kernels. They are one B-rep kernel (forge-math → geom → topo → ops →
bool, mesh) with domain crates on top, and nearly every domain crate already depends on it. The
exceptions are three dependency-free crates: forge-cam-geom, forge-sheet-core and
forge-bool-core. They are dependency-free on purpose: each is a whole-crate Aeneas (L4) target
(research/cam/build.md:18-20; forge-sheet-core/src/fold.rs:4). Two things are different in kind:
- **The guarantee regime.** The shipped B-rep path is f64 with one tolerance and typed refusals.
  CAM promises exact offsets and certified Clear/Hit/Undecided verdicts.
- **The algebra CAM needs.** Offsets and arc meets need exact `÷` and `√`. The B-rep exact layer
  is ring-only.

What is accidental is the copies underneath:
- two exact-number stacks and a third fixed-width integer;
- two interval types;
- four Shewchuk copies;
- six 2D line/circle routines;
- three certified-clearance branch-and-bounds.

**Recommendation: (b), one shared numeric + 2D core, with the domain crates kept separate.** Do
not do (a).

## The crates

| crate | lines | number model | guarantee | proofs |
|---|---|---|---|---|
| **forge-math/geom/topo/ops/bool/mesh** (~106k) | f64 + `tol::LINEAR=1e-5` (forge-math/src/tol.rs:9-20); filtered exact predicates, Shewchuk then fixed `Big<N>` (predicates.rs:1-23, 170); exact dyadics `Dy`/`meet3`, ring only (exact.rs:1-5, 321, 577); Krawczyk intervals (geom certify.rs, isolate.rs, meet_cert.rs) | tolerant, refuses with typed errors (bool boolean.rs:164-173, 670-686). Not shipped: the exact decide-once boolean and certified SSI tracing are features that are **off in every shipped build** (forge-model/Cargo.toml:22-27, forge-geom/Cargo.toml:14-19) | L4: predicates' fast paths, bool-core whole (97 fns), topo, mesh cdt, the flat SSI entry points (verify/aeneas/*). Models over ℝ/ℚ: verify/forge-exact, forge-bool-proof (contracts mostly open), krawczyk, ieee754 |
| **forge-cam-geom** (8.2k), used by forge-cam, forge-turn, forge-post | exact real DAG with `+ − × ÷ √`, sign by f64 filter then big-float to the BFMSS bound (real.rs:1-12, 825-889); outward f64 `Ival` (ival.rs) | exact offsets and booleans, no epsilon (boolean.rs:7), outputs snapped to 2⁻³² (offset.rs:681-694); checker certified Clear/Hit/Undecided (check.rs:13-21, 527-544) | models over ℝ only (verify/forge-cam, ~72 theorems). Written in Aeneas style but **not translated**: no verify/aeneas/forge-cam-geom. The exact-real sign is unproven |
| **forge-sheet** (14.6k) + **sheet-core** (0.5k) | f64 + `TOL`, refusal band `2·TOL`; exact signs via sheet-core's own Shewchuk (region.rs:37-56; sheet-core exact.rs) | refuses (`NearCoincident`, `Pinched`), never snaps | sheet-core at L4 (`classes_spec`, `owner_spec`, `chain_spec`); verify/sheet is a model, near at L5 |
| **forge-brake** (5.4k) | one formula over `f64` or `Interval` (`Sc`, num.rs:1-29) | certified Clear/Hit/Undecided, 4D B&B (bb.rs:63-73) with unproven sin/cos slacks (num.rs:76, 87) | model over ℝ (verify/sheet BranchBound.lean `certify_sound`) |
| **forge-turn** (5.7k) | forge-cam-geom's exact reals and Ival | its own Clear/Hit/Undecided (check.rs:418) | model over ℝ (verify/forge-cam Turn.lean) |
| **forge-weld** (2.2k) | f64 + ad-hoc epsilons (frame.rs:241, 376, 787) | tolerant, typed refusals | model over ℚ³ (research/weld-lean); L4 for `support` only |
| **forge-dwg** (34k; draft/ is the 2D kernel) | f64 + scale-relative `Tol` (draft/geom.rs:217-231), its own `V2` | tolerant drafting, snaps tangencies | none |
| **forge-thermo** (9.6k) | f64 FEM (Ogden membrane, Newton, sparse Cholesky) | estimate under a stated model (stretch.rs:7-8) | none (SymPy check of the Hessian) |

Dependencies, from each Cargo.toml:
- brake → math, ops, sheet;
- weld → math, ops;
- dwg → math, font;
- thermo → the full kernel;
- sheet → math, geom, topo, ops, dwg, sheet-core;
- cam and turn → cam-geom, plus math, geom, topo to read the B-rep (forge-cam lower.rs:1-17).

So "CAM as a client of the kernel" is already true at the level of forge-cam. Only its geometry
core stands apart.

## What is different in kind, and what is accident

**Fundamental:**
1. **CAM needs `√` and `÷` exactly.** A line offset by r needs its unit normal (offset.rs:254-255,
   349-350). Circle–circle and line–circle meets need a square root (arr.rs:291-303, 422-426).
   Dyadic-only arithmetic could drop `÷` with homogeneous coordinates. Dropping `√` would mean
   rewriting every predicate as a squared polynomial sign with case splits, and that code does not
   exist.
2. **The B-rep's shipped regime is tolerant**, and that is the regime the speed band and bits.txt
   measure. Making it exact-with-√ would be a different kernel, not a merge.
3. **Domain logic is real and separate:** bending kinematics and tooling, lathe reduction and nose
   compensation, Ogden FEM, weld ranking, DWG I/O.

**Accident** (each copy exists because of the Aeneas "no dependencies" rule, or because two
branches grew in parallel):
- **Exact numbers:**
  - forge-math `Big` (`S(i128) | L`) / `Dy` (exact.rs:13, 321);
  - cam `Nat` / `Dy` (big.rs:9, real.rs:20);
  - predicates `Big<N>` (predicates.rs:168).
  - The two `Dy`s disagree on representation: forge-math's is unnormalised with nearest-ish
    `to_f64`, cam's is normalised with directed rounding. So a merge must keep both behaviours
    bit for bit.
- **Intervals:** forge-math `Interval` (EMPTY on NaN, has trig, proven in ieee754/Interval) and
  cam `Ival` (whole line on NaN).
- **Shewchuk** `two_sum`/`split`/`two_product`, four times: predicates.rs:278-298,
  sheet-core exact.rs:17-32, bool edge_screen.rs:39-47, and the tests of interval.rs:466. Two of
  these are under L4 translations.
- **2D line/circle and circle/circle**, six times, each with its own tangency policy:
  - geom meet.rs:1760;
  - sheet region.rs:371/389: fixed band, one point;
  - weld frame.rs:842/864: no tolerance, two equal points;
  - dwg intersect.rs:289/304: scale-relative tolerance, flagged tangent;
  - cam arr.rs:394: exact;
  - brake geo.rs.
- **Certified B&B**, three times with the same contract: brake bb.rs (386 lines), cam check.rs
  (B&B part of 1793), turn check.rs:418. build.md:22-27 says outright that it was reimplemented
  because bendsim was not merged then, and that "the two should become one crate ... mechanical".
  It is merged now.

## Speed band

The gate builds `regen1` (forge-model: math, geom, topo, bool, ops, sketch) and `benchparts`
(forge-lang, which links sheet, brake, weld, dwg and thermo too) (tools/speed/gate.sh:99-100). Its
15 bench parts (interface/forge-lang/parts/bench) are pure B-rep: no sheet, CAM or brake calls.
- forge-cam-geom is linked into neither binary.
- forge-math's `exact.rs` is compiled in but runs only under `decide_once`, which is off. Unifying
  `Dy` therefore costs nothing on the band.
- The hot shared code on the band is `predicates` (Shewchuk). Moving it must stay inlined and
  bit-identical.
- CAM's own profile: 95–99% of end-to-end time is exact-real escalation (Nat isqrt, divrem and
  schoolbook mul on offset ties). The checker is 5 µs/move (research/cam/phase1.md:91-102). A
  shared core with an i128 fast path and one filter would help CAM, not hurt the B-rep.

## The options

**(a) Merge completely.** This means folding cam-geom, sheet-core and bool-core into
forge-math/geom, and putting the domain crates on one geometry API.
- Cost: it ends the three whole-crate L4 targets. forge-geom itself translates only with opaque
  `dyn` markers (verify/aeneas/forge-geom/translate.conf). The sheet-core and bool-core proofs
  would have to be redone, and CAM's planned L4 would be lost.
- It forces one regime: either the B-rep pays for exact `√` (and the band dies), or CAM loses
  exactness (and its contract dies).
- It buys nothing (b) doesn't. **Not recommended.**

**(b) One shared numeric + 2D core** (dependency-free, Aeneas style), with the domain crates kept
separate. It contains:
- one integer (i128 fast path + limbs) and one `Dy`, with both rounding behaviours kept;
- cam's exact-real DAG and its filter (CAM-PERF's #4 goes here);
- `Ival`, with `Interval` kept in forge-math sharing its outward helpers, unless NaN semantics may
  change;
- one Shewchuk;
- 2D line/line, line/circle and circle/circle that *classify* exactly (cross, touch, disjoint) and
  return f64 points. Each client keeps its own policy: sheet refuses, dwg snaps at its tolerance,
  cam stays exact, weld keeps its behaviour, so bits are unchanged. dwg's conic and spline
  intersections stay in dwg.

As a second step, one certified-B&B crate, with brake, cam and turn as clients.

- Cost:
  - roughly 2.6k lines moved (exact 735 + big 477 + real 1017 + ival 238 + Shewchuk ~150);
  - six 2D routines (~400 lines) to re-point;
  - the B&B later (bb.rs 386 + the B&B part of check.rs);
  - the forge-math and sheet-core translations regenerated, with the Refine proofs re-pointed
    (statements unchanged; the ieee754 Interval proofs untouched);
  - every step gated, bits identical.
- Band risk: low (only predicates is hot).
- Buys:
  - one place to prove: an L4 translation of the exact core serves CAM, turn and decide-once, and
    removes forge-math's four `*_big` opaques once predicates use the core integer;
  - one filter;
  - one fix instead of six for 2D meets;
  - one B&B contract to prove at L4 instead of three at L3.

**(c) Leave them.** Costs nothing now. The ongoing cost is that every exactness or tangency fix
must be made N times. Four Shewchuk copies need separate translation upkeep. CAM's exact-real
stack, its most expensive and most safety-relevant code, stays unproven and unshared.

## Recommendation

(b), in the order the earlier plan gave:
- numbers first, with no band risk;
- then the 2D classify-and-point module, client by client;
- then Shewchuk, with the two translations regenerated;
- then the B&B crate, now that bendsim is on main.

Keep the domain crates and the two guarantee regimes separate: they are genuinely different. Step
A (the poly root finder, exact-core/poly-roots @0f8a8696, unpushed) is independent of this choice.
