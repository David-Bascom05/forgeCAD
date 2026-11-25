# LANG-PROOFS handoff

The worker that kept forge-lang's proofs in step with the language: `research/forge-lang/lean`
(COVERAGE.md is the ledger; `tests/lang_coverage.rs` the drift check, 9 tests), plus the
determinism and wording items of 2026-11-25. Wound down at the coordinator's request.

## Branches

| branch | sha | what | state |
|---|---|---|---|
| `claude/lang-det` | 8be3bcac | f64::log10 replaced by an exact decimal exponent (forge-lang, forge-render's two saved uses); one source run per checked SetParam; edits evaluate inside the document's own root (`edit::At`); `research/forge-lang/determinism.md` | second review SAFE; pre-verify on main c135b274 run; final pre-verify after TRAIN-CONDUCTOR's gate-memfns re-baseline |
| `claude/powi` | 7fce97b1 | on lang-det: `forge_math::ipow`, every f64/f32 powi migrated, clippy disallows powi and f64 exp2/sinh/cosh/tanh/ln_1p/exp_m1; `mass::floors` | second review SAFE (bit-identical to compiler-rt across 108.8M cases); gate bits identical, band +0.0003%; pre-verify after lang-det's |
| `claude/expect-words` | (this branch) | a failed expect in words (`expected no interferences, found 1`), the line as written its detail (hover in the app) | screenshot sent to UX-LEAD; pre-verify after the re-baseline |

Order: lang-det, then powi (it contains lang-det), then expect-words (independent of both).

Everything else of this worker's is on main (the proofs, the ledger, lang_coverage, the train
rows of 4b/5).

## Open items, not started (wind-down)

- **f32 libm in clippy.** The configs disallow every f64 libm method but not the f32 ones
  (sin, cos, tan, atan2, powf, log10, exp, ...). Disallowing them hits 180 places in forge-render
  (camera.rs 41, viewcube.rs 14, studio/env.rs 11, scene.rs 10, ...). Most are view math, never
  saved, but `studio/` (the path tracer's CPU mirror) and anything feeding a saved value should
  move to an f32 dmath or f64. A sweep: classify each as display (allow with reason) or saved
  (fix).
- **Analyze Interference's expect list** still reads `FAIL line 3: interferences 1 (limit 0)`
  (`fgasm.rs`, from `ExpectOut`, which has no comparison). Asked UX-LEAD whether it should use
  the words too: ExpectOut would need `cmp` and `tol` (then `expect_words::words`).
- **SetParam check to one evaluation.** `apply_checked_at` now runs the source once and the
  edited text once. The app (`forge-render/src/fg.rs`) and the CLI already hold the source's run:
  an `apply_checked_from(before: &Run, ..)` would make it one. Incremental re-evaluation from the
  changed param needs evaluator checkpoints; `FamiliesRename.edit_prefix` and
  `EditClosure.edit_closure` already prove the prefix is unchanged.
- **Hosted callers of `edit`.** The hosted server's validation, open replay and client (not in
  this tree) should call the `_at` forms with the session's root; the coordinator has asked
  HOSTED-BUILD. `refactor::apply_checked` still checks with no directory.
- **Folded-vs-runtime check in CI.** Asked HOSTED-BUILD (ci/determinism): release builds on
  x86-64, aarch64 and wasm32 of a bits dump of constant expressions, diffed, so a host-dependent
  fold of any intrinsic is caught, not only powi's.
- **Separate workspaces still call powi** (`research/*` prototypes, `verify/ieee754/check`): not
  linted by CI, not shipped.

## Risks and notes

- `ipow`'s speed depends on LLVM inlining it with a constant `n` (match arms to 8, then a loop).
  Where it was slower than the intrinsic (forge_topo::mass recomputing the same powers) the
  intrinsic had been CSE'd as a pure call; ipow's multiplies are not shared across calls. Watch
  the band for hot `ipow` with repeated bases; the fix is to compute the powers once (as
  `mass::floors`).
- `decimal_exponent` reads the shortest round-trip decimal: exact and portable, but it allocates
  (a `format!`). Fine where it is (suggestions, lints, drags); not for a hot loop.
- The Lean ledger (COVERAGE.md) is checked against the Rust by lang_coverage on every build: a
  new built-in, document kind, argument list or edit op fails it until its rows exist. With this
  worker gone, whoever adds language must write the rows (the test's messages say which).
- `tools/preverify` before 13d78f06 could mark a step that failed to compile as ok; this worker's
  old GREEN for cb88d331 was such a one (superseded).
