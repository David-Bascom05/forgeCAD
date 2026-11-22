# Decision log (kernel-audit phase 0): handoff

State at this commit: done. The instrumentation (including the sites the wip left out), the
counters and the runner are in, the table in decisions.md is from this tree, and the gates below
have passed.

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

Added after the wip (each behind the same feature; forge-geom and forge-topo have a `decisions`
feature of their own, forwarded by forge-bool's, holding a thread-local collector that records
only inside its `collect`):
- forge-geom's hair roots (intersect.rs `hair_log`): a conic's turn with roots beside it, one touch
  (ABSORB_HAIR_ROOTS) or the crossings, recorded as touch or crossing at trim's curve against the
  edge's surface across (macro `with_hairs!`: off, the bare call), beside `crossings_near`'s count;
- check's EdgeOffFace (check.rs `face_log`): edge_off_face (16 points), a stray edge (256),
  certify (256), as "edge on face" under the checked body's edge and face, in a "result check"
  ledger opened by `decisions::checked` (refuse_strays, the RETRIED check, verify::stage, the
  runner's validity check);
- unify's SAME, beside `ssi::coincident` on the same faces, in a "unify" ledger;
- the n-ary place step: a cut at its edge's end; two cut points one vertex by one reach and by
  the clusters.

Not instrumented: the n-ary path's crossings_near site (`cut_by`), and forge-geom's hair roots at
curve_surface calls other than trim's crossing one.

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
| clippy `--workspace --all-targets -D warnings`, feature off and on | passed |
| release tests forge-bool, forge-model, forge-lang, forge-proof, feature off | 1082 passed, 0 failed |
| the same, feature on, FORGE_DECISIONS_TRACE | 1082 passed, 0 failed |
| hidden_inputs | the log's statics and env reads listed (no-output) |
| callgrind gate, feature off, against the base at a same-length path | bits identical; no line over 0.5%; synthetic parts within 12 instructions (start-up), bench lines -0.31% to +0.39% around the base, within their run-to-run spread (heatsink nary1: 0.28% between two runs of one binary, +0.02% least to least; kernel functions net +1400 of 170M) |

The first gate run on this tree found trim 0.2% over the base (torture seq +24 571, heatsink seq
+0.65%): a `let` around trim's curve_surface_in call under `#[cfg(not(feature))]` compiled
differently from the bare call. The wip commit alone was equal to the base (torture seq within 3
instructions). `with_hairs!` expands to the bare call with the feature off; after it torture seq
is within 1 instruction of the base.

## Findings

See decisions.md. In short:
1. Every retry rung (CAREFUL, ABSORB_HAIR_ROOTS, CHORDS, FENCES, the n-ary knife-edge fallback)
   rescues a regression test, though on the corpus only NO_RUN_ON rescues anything. None can be
   deleted on the corpus's evidence; those six tests are the decide-once rewrite's acceptance
   tests.
2. The near-coincident family holds 98.5% of the `Internal` plans' contradictions (unchanged).
3. The hair-root choice disagrees with crossings_near on 4379 facts: double roots at a tangency
   kept as two crossings, then deduped; no failure goes with them.
4. unify's SAME (0.5 LINEAR) splits 20 face pairs the boolean calls coincident (near_tolerance).
5. No disagreement in the n-ary place step (one reach vs clusters) or in check's edge on face.

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

## Next (for the decide-once rewrite, the master's call)

- One answer per fact for the near-coincident sites (decisions.md headline 2), then the rungs, each
  only once its regression test passes without it.
- The hair-root and crossings_near answers as one decision about a tangency.
- unify's SAME at the boolean's threshold, or the boolean's at unify's.
