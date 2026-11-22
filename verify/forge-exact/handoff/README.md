# Uncommitted forge-exact work, recovered (2026-11-22)

The first reference agent's worktree was removed while it worked; these are its scratch files.
Rebuild from them:

1. **Fix to the step-3 tolerance rule (lost, redo).** Two perpendicular faces that share an edge
   have grown boxes overlapping only in a thin column along that edge, and there they test as
   tolerance-equal, so whole bodies merge into one class. The fix: add `Box.wide` (more than
   4·LINEAR across along at least two axes) to `tolEq`, add a `wide` law to `Sym`, and prove it for
   each symmetry instance.
2. **Harness changes (lost, redo):** `NothingToCombine` judged out of scope (it comes from dropping
   a program's curved base statement); the `FORGE_EXACT_TRACE` switch; `reference_input_rule`.
3. **Repro tests:** `fe_addrepros.py` adds them to `verify/forge-exact/fuzz/tests/repros.rs`.
   - Bug 3's shrunk part literal: `fe_cw1.rs`.
   - Bug 4's shrunk program: `fe_lang28.fg`.
   - Bug 5's test: `fe_notched_test.rs`. The notched near-coincident part under the snap rule:
     V26 E40 F16, 23439.9988 mm³. Of its 400 ulp-moved stages, 67 agree, 301 refuse
     `Internal(SplitCrossed)` and 32 build `EdgeDegenerate`.
4. **Run outputs:** `fe_strict.txt` (the strict-rule comparison), `fe_notched.txt`, and `fe_snap.txt`
   (the snap-rule run, possibly partial).
5. **One unproven assumption** in RefFacts.lean: a closed planar B-rep's ray-parity membership is
   constant on every cell of its planes.

Delete this folder once all of it is rebuilt.
