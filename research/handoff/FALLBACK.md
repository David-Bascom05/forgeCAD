# FALLBACK: handoff

Written 2026-11-25 at wind-down, on main 5767cdb4. FALLBACK's standing jobs were the speed gate,
the Haas boolean refusals, and whatever the coordinator handed over. Everything below is either on
main, in a train, or named as unfinished with its state.

## Landed on main

- **5767cdb4, the speed gate's own machinery** (`tools/gate-memfns`, PR #63). Counted runs preload
  length-only `mem*` stubs, so a count no longer depends on the machine's glibc; each gate gets its
  own `TMPDIR` (two gates counting at once read each other's half-written bench font: the gearbox
  edit line was off by 364); `FORGECAD_CACHE=off` in every counted run; `bits` reads the checkout's
  own parts (`FORGE_BENCH_PARTS`) instead of the shared build folder another gate empties; and the
  baseline is re-saved. **Every later branch's band is measured against this baseline.**
  Pre-verify: guards, hidden, clippy, native, render, wasm green (tool 0965d9af); the gate stage
  FAILS by construction, because pre-verify runs each side's own `gate.sh` and this branch IS the
  new instrument (+3.80% total is the stubs, measured against main's glibc `mem*`). What stands in
  its place is in `research/speed/COMPILER-FLAGS.md` under "the gate's own machinery", plus an
  isolation table: the branch against itself with main's `benchparts.rs` restored, one instrument,
  **all 99 count lines byte-identical, +0.0000%**.
- Earlier trains: the CI native clippy job, the per-gate source copy, the font-race fix, and the
  Haas boolean fixes listed in `research/kernel-stress/haas-report.md`.

## In a train (T11), with their numbers

Each is bit-identical to main (all 84 `bits.txt` lines checked with `bits` and `benchparts bits`),
so a band line that RISES in T11 is not one of these: they only remove work.

| branch | sha | what | measured (old gate, callgrind, one unit, regen alone) |
|---|---|---|---|
| `ci/corpus-guard-budget` | 55791412 | `native-tests.py` gives a `corpus_guard_<doc>` test 15 s (`CORPUS_BUDGET`) and a 37.5 s re-timing limit; `rust-toolchain.toml` adds `wasm32-unknown-unknown`; `model-licences.py` measures a Git LFS file as its index pointer | tools only: every line 0 |
| `perf/intcurve-eval-memo` | a7d773c5 | `IntCurve::eval_kept`: a traced edge's points kept by t for the edge-face subdivision search | volute −32.5%, jack −25.3%, valve −18.4% |
| `perf/varblend-rail-uv` | 55d9d68d | `VarBlend::curve_uv`: a blend's own spring edge gets its uv in closed form instead of a ball solve per sample | crank −21.2%, bracket −0.004% |

Both perf branches carry a test that pins the new path to the old answer bit for bit
(`a_kept_point_is_evals_bit_for_bit_and_a_clone_starts_afresh`,
`a_springs_parameters_are_in_closed_form_where_projection_finds_them`).

## Held, with everything it needs to resume

**`perf/release-one-codegen-unit` @e8a42659** — `[profile.release] codegen-units = 1` plus
`research/speed/COMPILER-FLAGS.md`, which records the coordinator's standing rule for judging a
compiler flag (wall clock in the shipped build plus the instruction total; per-line instruction
bands still apply to code). Held because the bar is "faster or neutral on every gate case" and
crank is not: +3.12% (run) and +2.66% (edit) on the minimum, pinned and interleaved, while the
whole bench is −1.25% and 24 of 30 cases are better.

Crank's 3% is **not** instructions (one unit runs 0.009% FEWER on crank's regen), not I1 misses
(+0.33%), not D1 (−0.59%), not LL. The per-function profile is unchanged, so no `#[inline]` or cold
split has anything to aim at. It is front-end cost callgrind cannot see and this VM cannot measure
(`perf` says `<not supported>`). An unrelated unused function added to `forge-geom` and rebuilt at
sixteen units moves crank −0.79% and −1.50%, so ~1% is the layout lottery and one unit's 3% is
larger but of that kind. **To resume**: read front-end stalls and branch misses on a machine with
hardware counters, or repeat the pinned comparison on a second machine. Also still owed: the wasm
determinism compare (a release flag reaches `profile.wasm` through `inherits`) and the browser
gate's wall clock.

## Unfinished: the Haas boolean refusals

KERNEL-STRESS's corpus (`research/kernel-stress/`, `tools/kernel-stress/`) is the real-world source.
My queue was parked when David's direction turned to speed; refusals are honest, and a wrong-Ok
jumps the queue over any of them.

- **Two tracer fixes never merged** (they predate the wind-down and were overtaken; both are real
  and measured): `fix/nurbs-stall` @da29417d and `fix/far-line-torus` @8231923b, in
  `/home/user/wtnc` and `/home/user/wtbis`. Sweeps: nurbs-stall gave 102 refused→ok with 0
  refused→wrong over 5 files; far-line gave 0 refused→wrong and turned ST-28L #190333 and #192655
  `union_beside` from wrong to ok. nurbs-stall also makes TR160 #167334 `union_over` 29.1 s → 8.6 s
  and `int_over` 25.5 s → 2.9 s (still refused). **Worth picking up**; they need a fresh band on the
  new gate and a pre-verify.
- **Still refusing, with the diagnosis I reached**: hrt210s #16865 `union_over2`
  (InconsistentClassification across piece 15, the circle at z=−117.602 between torus face 9 and
  cylinder face 1, split at New(12) from the cylinder side; no torus-side seg reaches New(12)),
  hvs-1 #2060 (over only), TR160 #173731. Fixtures in `/home/user/haas/y/`.
- **The O-ring work is held** (`/home/user/wtk4`, fine torus tubes + stride-1 touch params on the
  careful retry): it fixed K004/K006 but gave wrong volumes on #56170 at offsets 0.4137 and 0.417,
  which is not tracer resolution. Do not land it as it stands. KERNEL-STRESS has a live oracle
  guard for that case (`a_part_with_tori_with_copies_near_over_answers_right_or_refuses`).
- `fix/torus-seam-cut` @614514d7 (`/home/user/wtts`) is the seam fix, folded into the nurbs-stall
  item for the train; band total −0.0014%, worst +0.0045%.

## Where the speed is, if someone picks up kernel speed

Measured at one codegen unit, callgrind, 26 gate parts, 4.87 G instructions; "mean" is the average
share per part, which is fairer across parts than the sum.

- booleans dominate: `boolean_into` 78% of all instructions / 57% mean; `trim_in` 40 / 28;
  `curve_surface_subdivide` under it 23 / 10 (the memo above takes a third of that on volute);
  `ssi::searched` 12 / 6; `split_face` 11.6 / 10.9.
- classification: `point_in_face_past` 9.2 / 9.4; `Index::new` 5.8 / 5.4 (crank 56%, which the
  VarBlend uv branch cuts); `Index::classify` 2.8 / 3.8.
- uv-loop drawing `face_uv_loops_with` 9.2 / 8.4 (manifold 50%); `nary_body` 10.3 / 19 (hole-grid
  89%); self cost: `dmath::sin_cos` 7.3, `IntCurve::eval` 5.4, allocation ~6 mean.
- **The two biggest remaining wins I found and did not take**:
  1. `Ellipse::closest` runs ~100 `sin_cos` samples per call — 623 k calls for 6.1 k closest()
     calls on gearbox, 26% of that part. A closed form or a bracketed solve would remove nearly all
     of it. Callers are the boolean's `settle`, `plan_once_in`, `trim_in`, `clear_step`. Handed to
     CURVED-SITES.
  2. The NURBS projection in the tracer: ST30L #1012290 `union_beside` (71 s, Ok) is 95%
     `trace_marched` → `intcurve::refine` → `Nurbs::implicit_gradient` → `Nurbs::nearest`: 17.6 M
     from-scratch Newton projections at ~48 k instructions each, plus 11% `memset` inside
     `eval_all`. Seeding the projection from the walk's last uv (as `value_near` does for walking
     surfaces) and not zeroing buffers per eval are the two obvious wins. Handed to IMPORT-SPEED
     and CURVED-SITES.
  3. `VarBlend`'s face box is the rest of crank's `Index::new`: `VarBlend::bbox` over 60 uv columns
     (28 M) and `Track::bbox` edge boxes (38 M), both solving a ball per sample. A bound-based box
     would be cheaper but CHANGES THE BOX VALUE, so bits may move: that is the rigor path (a
     differential run, the proofs redone, oracles, adversarial tests, a second review, the old path
     behind a switch for one train).

## Two product bugs found while measuring, both handed on

- `query interference` on `examples/rack18u/frame/test.fg` takes 4.8 s and then fails entirely,
  because ONE of 133 exact clash booleans refuses (`pan_nut[tl,2]` × `scr_tl[0]`, 4.39 s in the
  screw tracer, `RetryInvalid`) and the refusal aborts the whole query. The other 132 pairs total
  about 0.1 s. One refusal must not kill the query: it should be reported unconfirmed with its
  reason. Given to HOSTED-DESIGN (the forge-asm check path); the screw refusal itself to
  CURVED-SITES.
- `forge-cam-geom` `contact.rs`: a straddling disc capsule's bound floored at −rho instead of
  −(a + rho), so `clear()` cleared a 4.83 mm gouge at a negative limit and reported `lower` wrong
  by 3.8 mm. Fix and two adversarial tests on `review/bull-disc-floor` @eb5f279e; CAM-SPEED
  cherry-picked it onto `cam/speed-bull` (ccc7d37e).

## Risks, in the order I would worry about them

1. **The new baseline is the reference for every future band.** It was saved once, on this VM, with
   `GATE_SELFTEST=1` passing (built from the fixed source path; identical counts from two gate
   directories). If a later train sees every line move by a few per cent at once, suspect the
   baseline or the instrument, not the code — and check `bits.txt` first, which must never move
   except for a judged geometry change.
2. **Wall clock on this VM is unreadable under about 3%** unless pinned to a CPU and interleaved
   over 10+ runs. I had a part read 6.4% slower on one run and +1.0% min / −1.3% median when
   measured properly, and a part the change could not affect swing 4 points. That nearly made me
   gate the eval memo on a false regression. Instruction counts are exact; prefer them, and say
   "pinned, interleaved, least of N" whenever quoting time.
3. **The eval memo's map is per `IntCurve` and behind a `Mutex`.** Contention measured as nil on a
   4-thread n-ary build (nary4 deltas track seq), because threads work on different face pairs and
   the critical section is one hash probe. If the boolean's threading changes to share curves,
   re-measure before trusting it.
4. **The O-ring fix's wrong volumes (#56170 at 0.4137 and 0.417) are unexplained.** Whoever takes
   torus–torus work should start there: it is the one place I saw a wrong answer rather than a
   refusal, and it is held precisely because of that.
5. KERNEL-STRESS's "21 regressions" were booleans against main's healing, not regressions: main
   refused #69561 back to db5c0759. Do not chase them as regressions.
