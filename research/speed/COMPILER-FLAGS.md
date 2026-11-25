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

### Why it is held: crank

The coordinator's bar is faster or neutral on EVERY gate case. Crank fails it, and the failure is
real, not noise. Pinned to one CPU, 13 to 15 runs interleaved between the two binaries, on the
shipped x86-64-v3 build:

| case | cgu16 min | cgu1 min | min | median |
|---|---|---|---|---|
| crank run | 83.31 ms | 85.91 ms | +3.12% | +3.15% |
| crank edit | 91.44 ms | 93.87 ms | +2.66% | +1.00% |

What it is NOT, measured on the same source at x86-64-v2 under callgrind (crank's regen alone):

- instructions: 231,937,170 at one unit against 231,957,865 at sixteen — one unit runs 20,695
  FEWER, −0.009%. The per-function profile is the same, so there is nothing for an `#[inline]`
  or a cold split to aim at.
- I1 misses 1,235,608 against 1,231,581 (+0.33%); LLi 307 against 359; D1 349,462 against
  351,521 (−0.59%); LL 2,463 against 1,798, all negligible against 232 M instructions.

So the cost is in the front end — branch target alignment, the µop cache, code placement — which
callgrind cannot see and this VM cannot measure (no hardware counters: `perf` reports
`<not supported>`, which is why the gate uses callgrind at all).

How much of that is simply the layout lottery: an unrelated, unused function added to
`forge-geom`'s lib.rs and rebuilt at sixteen units moved crank by −0.79% (run) and −1.50% (edit)
on the minimum. So about 1% of swing comes from code moving at all, and one unit's 3% is larger
than that but the same kind of effect. (Reducing exactly that lottery is one of the reasons to
want one unit: a function's code then stops moving when an unrelated item changes elsewhere in
its crate.)

What would settle it: a machine with hardware counters, to read front-end stalls and branch
misses for crank's hot loop; or the same pinned comparison on another machine, to see whether
crank loses there too. Neither is available here.

Still owed before it lands: that, plus the wasm determinism compare and the browser gate's wall
clock (points 3 and 4). Until then the flag is not judged, and the branch is not queued.

## A note on the gate's own machinery

A change to `tools/speed/gate.sh`, `tools/speed/fixedrandom.c` or the baseline is neither of the
above: it changes the instrument. Judged by (1) the diff touching no source the gate measures, so
the band under one instrument is identically 0 with nothing to measure, (2) `bits: identical` and
`bits.txt` unchanged, and (3) the gate's self-test (`GATE_SELFTEST=1`): built from the fixed source
path, the same counts from two gate directories. Pre-verify's gate stage compares each side's OWN
gate.sh, so it reports FAIL for such a branch by construction; that is the instrument changing, not
the kernel, and the three checks above are what hold.
