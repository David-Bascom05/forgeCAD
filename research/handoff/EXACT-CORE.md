# EXACT-CORE handoff

Goal: one dependency-free exact-number core (`kernel/forge-num`) shared by the kernel and CAM,
plus speed work on CAM's exact arithmetic. Ground rules: the kernel's geometry stays bit-identical
(bits.txt). CAM-only arithmetic (`Ival`, `dual::D`, the exact reals behind forge-num's `real`
feature) must be sound but need not be bit-identical. A rewrite lands with a differential run,
redone proofs, adversarial tests, an independent review, and the old path behind a build feature
for one train.

## Merged (on main)

- num-core and num-cam: forge-num nat/int/dy; CAM's Real/Ival/Dy moved into forge-num behind
  `real`. The `real` gate keeps the kernel's shared generics untouched.
- real-speed @497dd78f: exact-real refinement speed (train 7).

## Being merged now

- **exact-core/ival-fast** (this branch). Stacked on exact-core/ties, so it carries both.
  - **ties**: exact multi-radical sign in the tower (`real.rs` `Tower`, `tower_sign`), capped by
    CAP 4096 and WORK 50 000, plus the `refined_bits` work counter (`offset::Stats`) and
    `tests/ties.rs`. On deep_boss_n_adaptive this cuts refinement from 1 397 760 to 24 576
    bits.
  - **Interval speed step 1**: `Ival::mul` picks its two products by sign class (finite, ordered
    operands), bit-identical to the old rule. Mould went from 14.6 to 10.2 s.
  - **Interval speed step 2′, the exact-zero rule**: x + [0,0] → x and x · [0,0] → [0,0] (other
    operand finite and ordered). Mould went from 10.2 to 7.75 s, one thread.
    - Lean proofs are in `verify/forge-cam/ForgeCam/IvalZero.lean`.
    - Differential over 33 machined docs and the forge-cam-quality parts: no certificate lost.
      The differences are higher feeds, drops up to 0.002 mm tighter, and ulp ties at 3
      decimals.
    - Adversarial tests: `forge-cam-geom/tests/ival_zero.rs`, plus
      `forge-num ival::tests::only_an_exact_zero_gives_an_exact_zero`.
    - The independent review came back SAFE.
  - **Switch**: the cargo feature `forge-cam-geom/ival-strict` (→ `forge-num/ival-strict`)
    restores the old rule. `forge_num::ival::STRICT` selects the matching CAM digest pins
    (forge-cam bits/surf, forge-turn bits) and quality pin (`pinned_strict.txt`). Remove the
    feature, the STRICT pins and `pinned_strict.txt` one train after merge.
  - **Review notes (non-blocking)**:
    - `Ival::sqrt` of an all-negative interval returns [0,0]. This predates the change, and every
      caller clamps first.
    - The `dd.hi == 0.0` branches at forge-cam-geom `check.rs:966` and forge-cam `surf.rs:1086`
      are unreachable, because sqr never gives hi = 0. They can be removed later.

## Unmerged, parked (not wound into this train)

- **exact-core/shewchuk @e5173b42** (step 4): Shewchuk-style adaptive predicates in forge-num,
  and DESIGN.md "six 2D line/circle policies stay separate" (step 5, closed by design). It was
  held for FALLBACK's re-baseline of the gate. Next steps:
  1. Merge main.
  2. Re-gate with band.py; zero loss on the strict lines is required. bits.txt must stay
     identical.
  3. Pre-verify, then hand to the train.
- **exact-core/meet2-weld @12abcd95**: a weld of the meet2 path onto forge-num. Same hold, same
  next steps as shewchuk.
- **exact-core/bb-brake @f8b9aa37** (step 6): `kernel/forge-bb`, a generic certified branch and
  bound (`Search` trait; `Policy{limit, res, budget, measure, near}`; verdicts
  Clear/Near/Hit/Undecided; extend the budget once, refine on Near). forge-brake is the first
  client (`forge-brake/src/bb.rs` adapter), and the Near verdict follows David's ruling. It was
  with BEND-SIM for review. `exact-core/forge-bb` and `exact-core/brake-near` are its
  predecessors, superseded by it. Next steps:
  1. Finish BEND-SIM's review.
  2. Merge main and pre-verify.
  3. Moving CAM's own B&B onto forge-bb is parked: it needs the full rewrite rigor and a
     cam_bench before/after.
- **exact-core/explore @bcbbd2ce**: research note only (should the geometry kernels be
  merged?).
- **Interval step (3)**: not started. It was planned under the same rigor rule.

## Risks

- **Rule changes to `Ival` move CAM outputs.** They move G-code feeds, drop heights and
  native=wasm digests, though not certificates. Every future rule change should rerun the
  33-doc and quality differential with strict and new builds side by side, as done here (the
  scratch scripts compared the JSON head and the G-code line by line).
- **The tower is bounded by CAP and WORK.** Hitting either bound falls back to refinement
  (sound, only slower). A pathological tie could still be slow, but no longer hang.
- **Workspace clippy needs SPINE's `#[allow]` at forge-topo `body.rs:907/909`.** It is on main
  since train 7. Do not "fix" that closure: it is deliberate for the Aeneas translation.
- **Pre-verify records from before tools/preverify ccd7ef7d may be false GREENs.** A step
  that failed to compile was marked ok. Re-run pre-verify, do not trust old records.
