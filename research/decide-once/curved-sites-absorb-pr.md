## What

Absorbs fix-curved 9470edc (with fix-touch 20afa5b and exact-curved 4f38bad) into the curved-sites work, plus the decision
log (FORGE_DECISIONS_DUMP), `edge_screen` (decision log only), OnEdge.lean and CarrierKey.lean / carrier_key.rs.
Current merge-train/1 merged in (conflicts: forge-bool lib.rs, trim_in's borrowed `across` with held_across, tests and
Lean lists kept both).

## bits.txt: 29 lines, attributed and judged

Attribution (bits + benchparts bits, FORGECAD_CACHE=off, one VM): every changed line comes from the fix-curved merge; replaying
each fix-curved commit onto main + exact-curved (229b518, which alone changes 0 lines), only three commits move bits, all the
forge-geom meet march's turn rule by the curve's tangent:
- bd28fc1 "Curved band" moves 18 lines (all moved again later);
- f051836 sets 12 final lines (sweep, torture-2, valve, fuzz5, fuzz7 nary, fuzz8 nary, fuzz9): **note: it is labelled a speed
  change ("at the cost of none") but changes 12 lines**;
- e15beef (fixes f051836's own regression on kernel_misc's hole near-tangent to a rim) sets 17 (shaft, mixed, fuzz1-4, fuzz6,
  fuzz7 seq, fuzz10).
Every other commit (exact-curved, fix-touch, 9470edc, the decision log, edge_screen, carrier keys) changes 0.

Judgement: the exact reference is out of scope for every changed part (a feature that is not the intersection of its faces'
inner sides: mixed f151, sweep f2, torture-2 f0, fuzz1-10 f1..21). Old (main) against new, seq and nary 4, this VM:

| part | V/E/F | check | volume | max vertex move | traced chains (changed edges: max sag old -> new; per edge larger/smaller) | verdict |
|---|---|---|---|---|---|---|
| shaft seq | same | 0 / 0 | 271034.555130 (Δ 5.8e-11) | 7.45e-14 | 2.55e-02 -> 2.55e-02; 10/15 | same |
| shaft nary | same | 0 / 0 | 271034.555130 (Δ 5.8e-11) | 1.54e-12 | 4.79e-02 -> 4.79e-02; 7/11 | same |
| mixed seq | same | 0 / 0 | equal | 1.12e-13 | 2.83e-02 -> 2.83e-02; 2/41 | same |
| mixed nary | same | 0 / 0 | equal | 2.84e-14 | 1.67e-02 -> 1.67e-02; 0/27 | same |
| sweep seq | same | 0 / 0 | equal | 0.00e+00 | 1.29e-02 -> 1.29e-02; 0/0 | same |
| sweep nary | same | 0 / 0 | equal | 0.00e+00 | 1.29e-02 -> 1.29e-02; 0/0 | same |
| torture-2 seq | same | 0 / 0 | equal | 0.00e+00 | 2.65e-04 -> 2.65e-04; 0/0 | same |
| torture-2 nary | same | 0 / 0 | equal | 0.00e+00 | 2.61e-04 -> 2.61e-04; 0/0 | same |
| fuzz1 seq | same | 0 / 0 | equal | 7.32e-15 | 7.43e-03 -> 7.43e-03; 2/3 | same |
| fuzz1 nary | same | 0 / 0 | equal | 0.00e+00 | 1.25e-02 -> 1.25e-02; 1/1 | same |
| fuzz2 seq | same | 0 / 0 | equal | 2.01e-14 | 5.40e-03 -> 5.40e-03; 1/6 | same |
| fuzz2 nary | same | 0 / 0 | equal | 1.42e-14 | 1.90e-02 -> 1.90e-02; 2/3 | same |
| fuzz3 seq | same | 0 / 0 | 243840.712036 (Δ 5.8e-11) | 3.19e-14 | 7.46e-03 -> 5.80e-03; 3/4 | same |
| fuzz3 nary | same | 0 / 0 | equal | 1.42e-14 | 8.83e-03 -> 8.83e-03; 3/4 | same |
| fuzz4 seq | same | 0 / 0 | 74160.774509 (Δ 1.5e-11) | 7.94e-15 | 2.94e-03 -> 2.94e-03; 1/2 | same |
| fuzz4 nary | same | 0 / 0 | equal | 1.46e-14 | 1.58e-03 -> 1.37e-03; 0/5 | same |
| fuzz5 seq | same | 0 / 0 | equal | 1.48e-14 | 3.61e-03 -> 3.61e-03; 0/0 | same |
| fuzz5 nary | same | 0 / 0 | equal | 1.08e-14 | 3.03e-03 -> 2.70e-03; 0/1 | same |
| fuzz6 seq | same | 0 / 0 | equal | 3.55e-15 | 1.99e-03 -> 1.99e-03; 0/2 | same |
| fuzz6 nary | same | 0 / 0 | equal | 6.80e-57 | 2.09e-03 -> 2.09e-03; 0/2 | same |
| fuzz7 seq | same | 0 / 0 | equal | 1.38e-17 | 1.15e-04 -> 1.15e-04; 0/0 | same |
| fuzz7 nary | same | 0 / 0 | equal | 3.57e-15 | 1.30e-02 -> 1.30e-02; 0/0 | same |
| fuzz8 seq | same | 0 / 0 | equal | 0.00e+00 | none changed | same |
| fuzz8 nary | same | 0 / 0 | equal | 0.00e+00 | 4.40e-03 -> 4.40e-03; 0/0 | same |
| fuzz9 seq | same | 0 / 0 | equal | 1.63e-15 | 7.92e-03 -> 7.92e-03; 0/3 | same |
| fuzz9 nary | same | 0 / 0 | equal | 2.85e-14 | 1.56e-02 -> 1.56e-02; 0/1 | same |
| fuzz10 seq | same | 0 / 0 | equal | 0.00e+00 | 1.29e-03 -> 9.86e-04; 0/2 | same |
| fuzz10 nary | same | 0 / 0 | equal | 2.06e-14 | 1.62e-03 -> 1.62e-03; 0/3 | same |
| valve seq | same | 0 / 0 | equal | 0 | 2 traced edges changed, no vertex moved | same |
| valve nary4 | same | 0 / 0 | equal | 0 | 2 traced edges changed, no vertex moved | same |

No line is worse: topology identical, volumes equal to 2 ulp, vertices within 1.5e-12, every part's largest traced-chain sag equal or smaller (smaller on fuzz3 seq, fuzz4 nary, fuzz5 nary, fuzz10 seq); per edge 133 smaller / 34 larger, chains 0.5-1% denser. fuzz8 seq (not among the 29) is unchanged.

## Gates (this VM)

- Speed band (exact callgrind gate, tools/speed/band.py; base merge-train/1 @2697baa and this branch counted on the same VM and
  valgrind): **pass**, every strict line faster; strict total 20129851604 vs 20198057291 (**-0.3377%**), worst strict line
  -0.0109% (gearbox app); synthetic lines -0.22% (plate nary1) to -2.02% (upto-flat nary1); shaft -1.4%, mixed -0.65%.
- Bits: the gate's bits (x86-64-v2 build) equal the judged bits.txt; wasm32-wasip1 bits identical to native.
- `cargo test --release --workspace --no-fail-fast`: 307 test binaries ok, 0 failed.
- `cargo clippy --workspace --all-targets`: clean but for main's kernel/forge-geom/examples/zz_meet295.rs (two warnings, from #33).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
