# verify/forge-bool-proof: today's boolean, proven as a whole

This package is the spine of the proof that the shipped boolean (kernel/forge-bool: boolean.rs
`general`, classify, imprint, unify, ssi, nary) is correct. The plan and its rulings are in
research/decide-once/old-path-statements.md. **[LEDGER.md](LEDGER.md)** says what is still
unproven, row by row.

| file | what |
|---|---|
| `ForgeBoolProof/Statement.lean` | §E: `opSet`, `reg`, `Faithful` (F1), `ManifoldPt`, `Refusal`, `RefusalTrue`, `Correct`, **`BooleanCorrect`** |
| `ForgeBoolProof/Contracts.lean` | §D: `general` as a nine-step `Pipeline` with a `View` per stage; the contracts `D1`–`D9`, each stage's postcondition (`PairsOK` … `ArrOK`, `ClassesOK`, `KeepOK`, `ApplyPost`), and the boundary lemma `KeepRuleBd` |
| `ForgeBoolProof/Compose.lean` | **`boolean_correct_of_contracts`**: D1–D9 and `KeepRuleBd` give `BooleanCorrect` of `Pipeline.run`; `res_keepRoom` (the closure of the kept rooms is `reg (A op B)`, through F5's `res_eq_reg`); `fallback_correct` (today's path, then A8); `seq_correct` (sequential regen) |
| `ForgeBoolProof/Axioms.lean` | the axiom report |

All of it is stated over the body layer `ForgeCurved.BodyModel` (verify/forge-curved). A stored
body is read through `abs : Body → BodyModel E`. Until the forge-bool translation exists, `abs` is
a parameter. The translated `general` being `Pipeline.run` of its translated steps is the shape
lemma (ledger E.shape). Unproven contracts are `def … : Prop` hypotheses of the composition, never
`sorry`.

Build: `lake exe cache get && lake build`, then `lake env lean ForgeBoolProof/Axioms.lean`. Mathlib
and the other packages are shared with verify/forge-curved (`packagesDir`). CI:
.github/workflows/forge-bool-proof.yml.
