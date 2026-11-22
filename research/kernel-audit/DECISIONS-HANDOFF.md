# Decision log (kernel-audit phase 0): handoff

State at this commit: WIP. The instrumentation, counters and runner are done, and the runner has
been run once at full depth (decisions.md). Two gates are still open: the feature-off callgrind
gate, and the release tests on the final tree.

## What is instrumented

The cargo feature is `decisions` on forge-bool, forwarded by forge-model and forge-proof. It is off
by default. All of it sits behind the `decided!` macro (lib.rs) or behind `#[cfg(feature)]`, so with
the feature off nothing is compiled in:
- `on_edge` gets `track_caller` only under the feature;
- `plan_once` is a `use` alias of `plan_once_in`;
- `fell_back` in forge-model is an `#[inline(always)]` identity.

`kernel/forge-bool/src/decisions.rs` holds the ledger:
- One ledger per plan: each sequential try (`plan_once`), each n-ary boolean (`nary_with`) and the
  imprint column path. A plan nested inside another plan writes to the outer ledger.
- A fact is keyed by its entities: (body address, V/E/F, index), unordered where the fact is
  symmetric, plus a point.
- Two decisions are the same fact when their keys are equal and their points are within LINEAR of
  each other.
- A decision is recorded only when its measure is within 100·LINEAR (`BAND`).
- When a plan closes, every fact with two answers is counted under each pair of sites that
  disagree, with one example (input, path, retry flags, entities, points, and each site's measure
  against its limit).
- With `FORGE_DECISIONS_STRICT` set, a contradiction panics (the assertion).
- With `FORGE_DECISIONS_TRACE` set, every rung prints to stderr with the name of its thread (for
  tests).

Fact kinds and the sites that record them (in boolean.rs `mod log`, unless noted):

| fact | sites |
|---|---|
| point on edge | every `on_edge` call, by caller line; `coincide`; classify.rs `point_in_face_past`, nearest edge (not where its `past` exception applies) |
| point at vertex | `on_edge` past an end; trim corner; imprint end; share_cuts at_end; place_cuts end_vertex |
| vertex on surface | derived from a coincident pair (every vertex of each face); trim's edge-box miss; trim's run-on end test |
| curve along edge | trim's run-on decision |
| two cuts one vertex | trim dedup; trim shallow (≤ 1.2 tol); place_cuts' final clusters (pairs within 4 tol) |
| two faces one surface | the sequential pair (`surfaces_in`); the n-ary pair (`closed_or_flush`); trim run-on's across-face `ssi::coincident`. A "no" is recorded only for planes nearly flush |
| touch or crossing | trim: curve_surface's touching root against crossings_near's count |

These sites are not instrumented:
- forge-geom's hair roots inside intersect.rs (another crate);
- forge-topo `check`'s EdgeOffFace, which runs on the result after the plan has closed;
- unify's SAME;
- the n-ary path's own place step and crossings_near site.

Retry rungs are counted per try, with their outcome (ok, internal, other, unknown) and the input:
- CAREFUL, ABSORB_HAIR_ROOTS, CHORDS, NO_RUN_ON, FENCES;
- the RETRIED result check, and `bends`;
- the n-ary → sequential fallbacks in forge-model `nary_regen_on`.

Runner: `verify/forge-proof/examples/decisions.rs` (required-features `decisions`). It runs:
- the reliability scoreboard's parts, on both paths;
- FUZZ_SEEDS=150 suite histories of 80 features;
- meta at nightly depth: 150 fuzz histories of 20 features, the suite parts, and 6 seeds of every
  adversarial family, under rigid, mirror, order, boolean, backends, certify and ulp (the
  properties known.txt files failures of);
- forge_lang::fuzz over its SEEDS (fuzz_known.txt);
- the near-coincident part (`box_a_hair_under_a_tolerance_below_a_notched_top`: adv near_tolerance
  2, first three features) under 400 ulp seeds;
- the bench .fg parts.

Its n-ary regens run on one thread. The `backends` property's `nary 2` runs a second thread, whose
decisions land outside the plans (counted as unscoped: 39k).

## Gates

| gate | state |
|---|---|
| fmt | passed |
| clippy `--workspace --all-targets -D warnings`, feature off | passed |
| clippy on forge-bool, forge-model and forge-proof with the feature on | passed |
| callgrind gate, feature off | NOT done. Started for the base (6979fa5c, `$SCR/base-src`, `CARGO_TARGET_DIR=$SCR/gB`) and for this tree (`$SCR/gA`, the same path length), and stopped while still building because the machine was at load 26 |
| release tests (forge-bool, forge-model, forge-lang) | NOT done. A run with the feature on and FORGE_DECISIONS_TRACE was stopped before it finished |

The clippy fixes in forge-ops that were needed at first came in from the branch instead (6979fa5c).

## Findings so far

See decisions.md for the notes and the table. In short:

1. Nearly all contradictions come from the near-coincident family:
   - point on edge: trim's crossing `on_edge` says on, imprint's says off (2316 facts);
   - vertex on surface: the coincident-pair implication against trim's edge-box test (2112) and
     against its run-on end test (1157).

   These are the six sites near-coincident.md names.
2. A contradiction predicts a lost way.
   - Sequential plans with a contradiction: 68% ended `Internal`.
   - Plans without one: 0.02% did.
   - 98.5% of `Internal` plans had a contradiction first.
3. Two cuts one vertex (656 facts, across many families, no failures): trim dedup disagrees with
   place_cuts and with trim's shallow pass.
4. Retry rungs:
   - FENCES and `bends` never fired.
   - ABSORB_HAIR_ROOTS (301), CHORDS (301) and CAREFUL (2) never succeeded.
   - NO_RUN_ON rescued one input (near_tolerance/20).
   - The soonest to delete are FENCES, CAREFUL, CHORDS and ABSORB_HAIR_ROOTS, once the regression
     tests and any Fusion/WHUCAD parts confirm this.

## How to run

```
cargo run --release -p forge-proof --features decisions --example decisions -- \
    threads 4 out research/kernel-audit/decisions.md
# subsets: only reliability,suite,meta,lang,hair,bench; rel N, fuzz N, seeds N, count N, adv N, ulp N
# rungs in the regression tests:
FORGE_DECISIONS_TRACE=1 cargo test --release -p forge-bool -p forge-model -p forge-lang \
    --features forge-bool/decisions -- --nocapture 2>&1 | grep '^decisions rung'
```

The callgrind gate puts the base and this tree in equal-length target directories, then compares
the two outputs line by line (not the committed baseline):

```
git worktree add --detach $SCR/base-src 6979fa5c
(cd $SCR/base-src && GATE_COUNTER=callgrind GATE_REPS=3 CARGO_TARGET_DIR=$SCR/gB sh tools/speed/gate.sh > $SCR/gateB.txt)
GATE_COUNTER=callgrind GATE_REPS=3 CARGO_TARGET_DIR=$SCR/gA sh tools/speed/gate.sh > $SCR/gateA.txt
```

## Next

1. Run the callgrind gate on both trees. It should show bits identical, and no line of the change
   above the base's count.
2. Run the release tests for forge-bool, forge-model and forge-lang, with the feature off.
3. Run the regression tests with FORGE_DECISIONS_TRACE, to see whether any test relies on a rung
   succeeding (the Fusion/WHUCAD regressions in forge-lang tests/kernel_*). Add the result to
   decisions.md.
4. Optionally:
   - give the `on_edge` sites names instead of line numbers;
   - instrument forge-geom's hair roots (ABSORB) and check's EdgeOffFace;
   - add the n-ary place step.
5. Make the final signed commit without "wip", and report the headline findings.
