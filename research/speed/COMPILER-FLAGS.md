# How a compiler-flag change is judged

The coordinator's ruling (2026-11-25), from David's goal: speed users feel. It applies to a change
that alters no source the kernel runs — a profile setting or a codegen flag (`codegen-units`, LTO,
`target-cpu`, `panic`, `opt-level`) — and to nothing else. **A change to code is still judged by the
exact band, every strict line and the total within +0.03%.**

## Why the band does not decide it

The gate counts instructions. Cross-unit inlining trades instruction count for locality: with
`codegen-units = 1` the kernel ran 0.48% fewer instructions in total, yet `clamp edit` ran 3.27%
MORE — and that same line was 2.8% to 10.3% FASTER on the clock. A per-line instruction band would
have refused a change that is quicker everywhere it was timed. The band stays for code because
there a line's rise is the code's own cost; for a flag it is the compiler's layout.

## What a flag change must show

All of these, on one machine, with nothing else running:

1. **Wall clock, the shipped build.** The real target (`x86-64-v3`, the release profile as shipped),
   least of 7 runs per case, every gate case (each bench part's `edit` and `run`), the two sides
   interleaved so drift cannot favour one. Faster or neutral on every case; name any case that is
   worse and by how much.
2. **The instruction total.** Not worse, from the gate's own counts with the same gate on both
   sides. Per-line rises are reported, not refused.
3. **Bits, native and wasm.** `tools/speed/bits.txt` identical, and the wasm bits too: a flag in
   `[profile.release]` reaches the wasm build (`[profile.wasm] inherits = "release"`), so the wasm
   determinism compare runs as well as the native one.
4. **Wasm wall clock.** Neutral or better on the browser gate's cases.
5. **Build time.** Measured and written down, cold, for the shipped binaries. Not a blocker: the
   trade is the owner's to make, but it is never silent.

## What was measured for `codegen-units = 1`

On main c135b274, this VM, by FALLBACK:

- Instructions, same gate both sides (the mem* preload in both), 1 against 16: strict total
  **−0.4846%**. Better on most lines (volute −1.6% app and edit, −3.5% regen; upto −2.0%). Worse on
  a few: clamp edit +3.27% (worst), valve app +1.47% and edit +1.50%, quadarm cli +0.37% and edit
  +0.44%, torture nary1 +0.74%.
- Wall clock, the shipped x86-64-v3 build, all 15 bench parts, `edit` and `run seq`, least of 10,
  interleaved: total **1923.5 ms → 1899.5 ms, −1.25%**; 24 of 30 lines better (clamp run −4.90%,
  bracket edit −4.27%, bracket run −4.07%, hub edit −3.94%, gearbox edit −3.38%, volute run −2.77%);
  4 worse (crank edit +3.97% and run +2.66%, heatsink edit +0.83%, gearbox run +0.81%).
- Earlier, on main 0de84ac1: 3–5% quicker on the CAM documents (mold_core_cam 12.48 → 11.93 s,
  manifold_cam 1.22 → 1.17 s); release builds about 20% longer (203 → 242 s cold, two binaries).
- Fat LTO, measured the same way: 2.9% fewer instructions, no quicker on the clock, 66% longer
  builds. Not taken.

Still owed before it lands (points 3 and 4): the wasm determinism compare and the browser gate's
wall clock. Until those are in, the flag is not judged.

## A note on the gate's own machinery

A change to `tools/speed/gate.sh`, `tools/speed/fixedrandom.c` or the baseline is neither of the
above: it changes the instrument. Judged by (1) the diff touching no source the gate measures, so
the band under one instrument is identically 0 with nothing to measure, (2) `bits: identical` and
`bits.txt` unchanged, and (3) the gate's self-test (`GATE_SELFTEST=1`): built from the fixed source
path, the same counts from two gate directories. Pre-verify's gate stage compares each side's OWN
gate.sh, so it reports FAIL for such a branch by construction; that is the instrument changing, not
the kernel, and the three checks above are what hold.
