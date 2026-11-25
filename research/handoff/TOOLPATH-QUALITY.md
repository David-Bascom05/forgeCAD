# TOOLPATH-QUALITY handoff (2026-11-25, wind-down)

## Branches

| branch | sha | state |
|---|---|---|
| `cam/hop-map-under` | 6a7b384d | **train 8**. PRE-VERIFY GREEN, sent. Conservative hop map, adaptive's descent check, guard: nothing fast meets material, ledger row, re-posted examples |
| `cam/adaptive-only` | a7d36860 | **train 9**. Only the shallow-level hint (1f0fb8e7) now. The second-round crumb take-back (0cfa31b9) and its docs were reverted: main's engagement-cap guard showed it hands more of deep_islands to the over-cap rings (538 → 641 mm over). It was a cycle-time change (−9 % two-way islands); it comes back only with rings that hold the cap |
| `cam/crazy-guards` | see WOUND DOWN | **train 8**. Contains `cam/hop-map-under`. The crazy-path guards: `forge_cam_quality::crazy`, `guards.rs` for the parts, a `corpus_guard_<doc>` per example. CAM-2D-FINISH (`cam/crazy-2d`) and CAM-3D-FINISH (`cam/rest-air`) build on it |
| `cam/park-descent` | daa6c225 | CAM-CORE's to merge. The same two map commits plus `Field::park_clear`, on their pass-start WIP |
| `cam/rings-cap` | this branch | **WIP, not for a train.** Rings within the cap, option B's ring half, behind switches. This file |
| CAM-CORE's `cam/adaptive-pass-start` | f8fb1b4d | theirs: the pass-start / round-box fix. Needs `cam/hop-map-under` in main first (alone, boss_adaptive's links graze at the link feed) |
| `cam/adaptive-loops` | 40df387a | parked: tangent loops between two-way passes (R 0.5 mm). Re-judge under a block-rate model (research/cam/toolpath-quality/adaptive-transitions.md) |

The analysis branch `cam/ship-scurve` (S-curve harness switches) is not needed any more.
Its local accounting switches were never pushed: `FORGE_ADAPTIVE_ACCT`, `FORGE_P2_DROP`,
`FORGE_CRUMB_NOSTALL`, `FORGE_RETRACE_PULL`. Their results are in
research/cam/toolpath-quality/adaptive-islands.md.

## Unfinished: adaptive's rings within the engagement cap (option B)

**Problem.** The end-of-level cleanup rings are plain offsets half a diameter apart. They cut at
90°, and at 180° where the first ring enters a band no pass touched, whatever `engagement:` says.
CAM-CORE's `engagement_cap.rs` (`cam/adaptive-cap-warning`) lists every adaptive op over its cap.
Rings and the wall pass carry about 45 s of boss_adaptive's over-cap time, and all 35 s of
deep_boss_n's.

**Measured** (main plus the hop-map fix; mm of level cutting over the cap, from CAM-CORE's
measure):

| variant | deep_boss_n | deep_islands | rptu3–6 | time | volume |
|---|---|---|---|---|---|
| base | 893 | 412 | 440–636 | — | — |
| `FORGE_RING_AE=1` (rings r(1 − cos cap) apart) | 731 | 629 | 148–445 | +11 to +52 % | RPTU 4/5 +5–8 %: the corners the base leaves; not a gouge, every cert passes, under clear's volume |
| `FORGE_RING_CAP=1` (a run stops at its first step over the cap) | 139 | 67 | 0–219 | −4 to −31 % | **8–39 % left uncut** |

**Why spacing alone fails.** On deep_boss the passes stall early and leave the lower-left band,
about 57 % of the volume, to the rings. The first ring into an untouched band is a full slot,
whatever the spacing. Plot: phi by position on deep_boss_n, made during the work, not kept.

**What B still needs.**
1. CAM-CORE's rounds loop (not done: CAM-CORE wound down; the place is the `next_sweep!` macro
   in adaptive.rs, which already decides "sweep again vs rings"; research/handoff/CAM-CORE.md): `for round in 0..2 { sweeps; rings k > 0; if left_area(f, f.r) <=
   threshold { break } }`, with every pass kept in round 2 and the wall pass (k = 0) only after
   the last round. Agreed with CAM-CORE; not landed.
2. Then `FORGE_RING_CAP` as the default, with the material it leaves handed to round 2's passes.
3. Cap the wall pass too, or show its over-cap is only the last band. deep_open_slot's 408 mm
   over is outside the cleanup rings.
4. Judge on CAM-CORE's start-order branch, not main: today the rings do most of the clearing
   because the passes stall.
5. Gates:
   - `crazy` and `engagement_cap` take entries off and add none;
   - `guards.rs` equal volume;
   - every certificate;
   - by David's rule for a rewrite: a differential old/new over every CAM doc and quality part,
     adversarial tests, a second-agent review, and the old path behind a switch for one train.

**Measuring.** `CAPSCAN_OUT=FILE cargo test --release -p forge-cam-quality --test zz_capscan --
--ignored` prints each adaptive op's over-cap mm, t_la and volume. Run it with the switches set
in the environment.

## Other open items (not started; for whoever picks adaptive up)

- **Helix entries.** On boss_adaptive (10 mm tool, one 14.8 mm level), 528 of 667 s of cutting is
  the 16 full-depth helix entries. Measure a ramp entry, or two levels, against it. Add an advisor
  hint only if one is faster. (CAM-CORE's observation.)
- **Corpus guard time.** `corpus_guard_mold_core_finish` took 18.9 s alone in debug against its
  15 s budget, measured at load 13 on 4 cores. CAM-3D-FINISH measured the mold_core pair at 23.7 s
  in release, also under load. Re-time it on a quiet machine. If still over, it is a CAM speed bug
  (the coordinator's ruling), not a guard to loosen.
- **Speed.**
  - A dilated-max map in HeightMap, to win back `engage_conservative`'s cost
    (kernel/forge-cam/tests/engage_conservative.rs is the spec).
  - Adaptive's Field blocks onto `engage::Tiles`.
  - Both are speed only, with byte-identical output.
- **Islands.** Design (iii), passes planned for their transitions, is deprioritised: it is
  cycle time only (research/cam/toolpath-quality/adaptive-islands.md).

## Risks

- **`Field::swept`.** It records every adaptive move for `descent_meets` and `park_clear`. The
  crumb take-back truncates it with the map (`undo_swept` beside `undo_map`). Anything that undoes
  map state without truncating `swept` makes the descent check see moves that never happened.
  That errs toward plunge feed, so it stays safe, but it is wrong.
- **The two maps.** The hop map is conservative (`sweep_under`). The engagement map, adaptive's
  steering map and `disk_has_material` are still centre-sampled. Any new fast move judged against
  a centre-sampled map can graze a sub-cell skin. The fast-cut guard catches it on the harness
  only, so keep new parks and descents on `descent_meets` / `park_clear`.
- **`crazy::KNOWN` after CAM-CORE's `cam/clear-degenerate` (d9c8ddaa).** It fixes rptu1 nut's
  re-cut. When it merges main after train 8, it must drop ("rptu1_clear_cam", "nut", "recut", …)
  and set the nut "rapids" entry to 397.0 (the slot's far-end trips for the islands' corner arcs;
  a stock-aware air test is in CAM-CORE's handoff). Otherwise guards.rs fails: the stale entry
  errors.
- **`crazy::KNOWN`.** Values carry 10 % headroom, and a listed op that no longer breaks its bound
  fails the test. A legitimate change that fixes one must take its entry off in the same commit.
