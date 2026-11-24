# FAST-VERIFY phase 1: where a branch's time to main goes (measured 2026-11-24)

Nothing in tools/ or .github/ has been changed. This file, the prototype `pargate.sh`, and
`train-crates.txt` (which crates each train 6/7 branch touches) are measurements only.

## 1. Timeline (git, GitHub, TRAIN-CONDUCTOR's logs)

| | freeze | gates start | PR opened | merged | freeze → main |
|---|---|---|---|---|---|
| train 5 | 06:20 | (split into 5a/5b mid-way) | 13:30 | 15:34 | 9 h 15 |
| train 6 | 17:40 | 17:55 | 19:05 | 19:40 | 2 h 00 |
| train 7 | 21:00 | 21:04, restarted 22:08 | — | — | > 1 h 30 so far |

Gaps between trains: T5 landed 15:34, T6 froze 17:40 (2 h). T6 landed 19:40, T7 froze 21:00 (1 h 20).
A branch that becomes ready just after a freeze waits for one gap plus two trains: about **4–6 h**.
It waits about 4 h more if its train drops it.

Train 6's gates ran **serially in one script on one 4-core VM**, with `target/` deleted between
stages for disk:

| stage | wall |
|---|---|
| assembly by hand (13 merges, 5 squashes, 4 conflicts) | 14 min |
| native workspace tests (native-tests.py, JOBS=3) | 27 min |
| forge-render clippy + tests (lavapipe) | 19 min |
| callgrind gate.sh on the train | 8 min |
| gate --save on a fresh main worktree, for band.py | 8 min |
| wasm32 (wasm-tests.py, 395 binaries) | 15 min |
| **serial local total** | **~77 min** (+ ~35 min lost: disk restart, hidden_inputs round-trip) |
| GitHub CI on the PR (45 checks) | 23–35 min; critical path is web `model` 23 min, web `gpu` 22 min |

Train 7 lost more time: native tests (29 min) found 3 forge-cam-quality failures and forge-render found 1. A bisect by
hand took about 40 min, then two branches were dropped and everything rebuilt. A registry re-review round-trip took about 1 h. **Both failing
branches would have failed alone on their own crate's tests.**

GitHub CI is not the bottleneck (Lean is already cached and path-filtered). Neither is the gate.
The bottleneck is **serial local stages, train failures found only after the freeze, and the
idle gap between trains.**

## 2. The speed gate, measured on this VM (main @ded2d224)

* Build (x86-64-v2, own target, cold): **214 s**. Count phase, 99 lines, one CPU: **215 s**.
* Prototype `pargate.sh` with the same binaries, the same count1 recipe, and one CPU per worker:
  **57 s with 4 workers, every count identical to the serial gate (99/99, two runs).**
* **Race found (pre-existing, affects gates run side by side today):** every benchparts run rewrites
  `/tmp/forge-benchparts/block.ttf`, a fixed path. Two counted runs at once can read each other's
  half-written font. My first parallel run, without isolation, got `gearbox edit` 448177715
  instead of 448178079 (364 off). Fix that keeps counts identical: give each worker a private bind mount at
  the same path (`unshare --mount` + `mount --bind`, which works in these containers). `pargate.sh`
  does this. Running the train and main gates in parallel without it is **not** safe.
* Main's counts against the committed baseline: **main fails its own baseline** (+0.01% on
  synthetic lines) because train 6 landed without re-saving `baseline-valgrind.txt` (last saved at 468acf0c).
* Is the gate comparable across VMs? I rebuilt 468acf0c here: **10/99 lines differ** from the baseline committed
  from another VM (frame, gearbox, valve, volute; ≤0.005%), and bits are identical. On this one VM,
  4 runs of a binary (serial, parallel, under load) are identical. So **exact counts are per VM** (likely
  glibc's per-CPU memcpy/strlen choice; this VM: Xeon with AVX-512 and ERMS, glibc 2.39, valgrind 3.22).
  The conductor's second "main" run is needed today, but it can be **cached per (main sha, VM fingerprint)**
  and run off the critical path. Main changes only when a train lands.

## 3. How much of a train the gate can actually see

Crates in forge-model and forge-lang's normal-dependency closure (what the gate's binaries are built from): 32 of 74.
Of train 6's 23 branches, 14 touch none of them. Of train 7's 21, 13 touch none. `train-crates.txt` has the list.
For those branches the gate's binaries are **byte-identical to main's**, and the same binary on the same VM gives the same
count. So a memo keyed by sha256(regen1, bits, benchparts, parts/*.fg, fixedrandom.so, gate.sh,
valgrind and glibc version, CPU model) returns main's counts exactly. That is a cache, not a skip.

## 4. Proposals (for the coordinator; nothing changed yet)

None of these drops a check from what lands. Every train still runs every stage on the exact tree that merges.

A. **Pre-verify each branch on its owner's VM before the freeze** (25 VMs in parallel). The owner runs:
   tests of the crates the diff reaches by the Cargo reverse-dependency graph, clippy on those crates, the cheap
   guards (registry, hidden_inputs, sizes, removed-checks, patent grep: seconds), and the gate if
   the binaries changed (memo otherwise). A branch joins a train only with a green pre-verify record
   (sha + results). This would have caught both train-7 failures and both re-review round-trips.
   Expected effect: trains rarely bisect.
B. **Shard the train's stages across VMs**, all on the same train sha: native | forge-render | wasm |
   gate + clippy + guards. The critical path becomes max(29, 19, 15, 6) ≈ **30 min instead of ~77**.
   The gate shard runs on the VM that holds main's cached counts.
C. **Gate**: parallel counts with the font isolated (−2.5 min per side). Main's side is computed per main sha
   while the train assembles (−8 min on the critical path). The binary-hash memo for pre-verify. And train PRs commit
   the re-saved baseline, so main always passes its own baseline.
D. **Pipeline trains (merge-queue style)**: train N+1 freezes on top of train N's head when N's PR
   opens, not after it lands. That removes the 1.5–2 h gap. If N fails, N+1 is rebased (its pre-verified
   branches rarely fail). Smaller trains, about 8 branches, every ~45 min.
E. **Fast lane** (docs, `interface/` web text, research/, non-closure crates): pre-verify plus the next
   train's full run. It joins the next train like everything else, so it adds no separate path to main.
   With D, a fast lane is barely needed.
F. Keep each stage's `target/` between trains, not deleting between stages: their builds are disjoint
   (debug test, wasm32, x86-64-v2 release), and only debug tests are large. Or sccache with a size cap.

Estimated: median branch-ready → main goes from about 4–6 h to about **1–1.5 h** (wait ≤ 45 min for the next freeze,
plus ~30 min sharded verify, plus ~25 min CI, which overlaps the next train), with far fewer drops.

## 5. Validation plan for phase 2

Re-run train 6 and train 7 (final sha) both ways on one VM. Compare the gate lines (exact), test
pass/fail sets, clippy output, and guard outputs. Every verdict must be identical. Report the per-stage
wall-clock before and after.
