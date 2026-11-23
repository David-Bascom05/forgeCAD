# The proof standard

Every kernel feature is proven to the level the boolean is. The owner set this rule on 2026-11-23:
"it should all be proven to the same level as boolean is". Old code may stay where it is proven
correct. Retries, fallbacks and order- or budget-dependent results each need one decided rule.

## The six levels

A feature meets the standard when it has all six.

| # | Level | The boolean's instance |
|---|---|---|
| L1 | **A spec in Lean over the reals.** It is the one definition of the feature's result, with standard axioms only and no `sorry`. | `reg_kept`: the regularised boolean (verify/forge-exact) |
| L2 | **An exact executable reference.** It runs over ℚ, or over exact algebraic numbers, and is proven equal to L1 on its domain. | the flat exact reference; Alg/Quad for curved |
| L3 | **Correctness for every input in a stated domain.** No enumeration of logged cases: the algorithm's decisions give exactly L1, and a refusal happens exactly when L1 has no valid result. | decide-once (c): `flat_correct`; refusal iff `XNonManifold` |
| L4 | **The shipped Rust refines the proven algorithm.** The kernel crate goes through Aeneas to Lean, with no panic and no overflow within stated size limits. Where Aeneas can't take the code, a differential test against the L2 oracle with a committed digest is the interim, marked interim. | forge-bool-core: union–find, flood, index, rules, … |
| L5 | **Its floating-point predicates are proven** against verify/ieee754, under stated preconditions that debug builds check. | orient2d, cross_sign, dot3, incircle; TwoSum and TwoProduct |
| L6 | **A differential check against L2 on fuzz and real corpora,** judged stage by stage, in CI. | forge-exact-fuzz, repros, and the box family |

## Proving the actual code (the owner, 2026-11-23)

"Use the Rust-to-Lean (Aeneas) wherever possible to prove that code is right."

- L4 is done by Aeneas, translating the real Rust and proving it refines the proven algorithm, wherever that can be made to work.
- A hand-written Lean model plus a differential test counts only while Aeneas can't take the code yet, and is marked interim with the blocker named.
- Writing new kernel code in the style Aeneas accepts (research/aeneas/STYLE.md) is the default, as long as bits and speed stay identical.
- The aeneas session (branch suffix `aeneas`) owns:
  - the toolchain;
  - the float bridge to verify/ieee754;
  - a CI check that the translations are current;
  - the per-module survey (research/aeneas/SURVEY.md) that names each module's L4 target.

## No proofs at runtime for what is proven (the owner, 2026-11-23)

"If we have proven all cases of a thing, and you are REALLY SURE, then we should not need to do a proof in runtime."

- Code proven for every input in its stated domain (L3 plus L4) runs with no runtime re-check. Certificate checkers and re-verification of proven results run only in tests, fuzzing, CI and explicit on-demand commands, never on a normal build, edit or render.
- A precondition the proof relies on is checked in debug builds only (as L5 already says).
- Runtime work remains where the result depends on the particular input and can't be decided ahead of time: the exact fallback behind a proven float filter, and certifying a traced intersection curve. That is computing the answer, not re-proving it, and it is held to the speed rule like any other code.
- A runtime check is removed only once the proof covers the shipped code (L4 by Aeneas, or the interim differential test named), not just the algorithm.

## Features

Each row says who owns it, which levels it has, and what is next. The levels are counted from what
has been reported to the master.

| Feature | Owner session (branch suffix) | Has | Next |
|---|---|---|---|
| Boolean, flat | decide-once-lean, decide-once (Rust), ieee | L1 L2 L3 L5 (partial L4, L6) | L4 split A–D, local ledger |
| Boolean, curved | exact-curved, trace | L1 (quadrics) | L2 executable, L3 |
| Fillets / rounds | fillet-spec, fillets, conic-fillet | — | L1 (ball closing), L2 |
| Splines | spline-spec | — | L1, L2 oracle |
| Sheet metal (fold, regions, refold) | sheet-spec, fix-sheet | — | L1 fold, L2 refold checker |
| Bend simulation | sheet-spec, bendsim | certified intervals | L3 soundness |
| Validity check | fundamentals | L1 L3 (check_sound), L6 | L4 |
| Mass properties | fundamentals | L1 (rules exact) | derived rounding, L4 |
| Rigid transforms and patterns | fundamentals | L1 L3 | L4 |
| Tessellation | fundamentals, fix-mesh-asm | L1 (closed, oriented; chord sag) | after fix-mesh-asm |
| Sketch solver | fundamentals (item 5) | — | L1 |
| Intersection tracing | trace | step lemma (Krawczyk) | phase B, binary64 |
| .fg language | ieee (language proofs) | lexer, LL(1), edits, reparse | — |
| Extrude / revolve / sweep / loft / coil / thread | sweep-spec | extrude: L1 (`Prism`, `through_finite`), L2 over ℚ (polygons), L6 (`sweep-diff`, interim L4); taper: L1 algebra (mitre, events linear), the decided rule, L2 over ℚ (rational edge lengths), L6; revolve: lt_mul_sqrt_iff, reach_arc, CrossesAxis exact; coil: coil_injective, exact TurnsTouch; loft: ruled_translate, L2 over ℚ (homothetic), L6 | revolve L2 (QForm or the sign lemma over Fillet.Rev); taper over Alg (research/sweep-spec) |
| Shell / draft / hole / rib / chamfer / pattern-of-features | ops-spec, fix-shell, fix-draft | L1 partial: pattern P1–P3, hole H1 H3 H4, chamfer C1 C2 (plane pair), draft D1–D3 (planes) | L1 recarrier, shell, rib; L2 reference; L6 harness |
| Thicken | surfaces (thin/thicken) | — | L1 with ValidSheet |
| Surfaces and thin | surfaces | — | ValidSheet with fundamentals |
| Sketch text | sketch-text | exact glyph geometry | L1 |
| Weldments | weldments | — | L1 (members are extrudes and half-spaces) |
| Assemblies and interference | fix-mesh-asm | — | L1 (one classification from the boolean) |
| STEP import / export | import-robust | — | L1 (round-trip) |
| CAM (toolpaths, stock, posts) | cam | — | L1: G1–G6 of research/cam (no gouge, holder/rapids clear, stock within tolerance, limits, post round-trip) |
| Mesh export | mesh-export | closed / oriented / manifold check | L1 via tessellation |

A branch that adds or changes a feature says which levels it keeps or adds. Merges are judged
against this table.
