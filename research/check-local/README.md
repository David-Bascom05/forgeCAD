# The local validity check

`forge_topo::check_against(new, old)` answers as `check(new)` does, given that `old` passed, testing only what differs.
Used by chamfer, draft, shell, thicken and the direct edits in place of `check`, and by `forge_model::verify` on a worker: after a
commit or a settled rebuild, each stage against the stage before it. Regeneration itself keeps its cheap checks, so a frame never
waits. A save runs the whole `check`; a fault warns and the file is still written (David, 2026-10-31).

## What is proved (`lean/`, no sorry; `lake build`, `lake env lean Axioms.lean`)

```
full_of_local      old passed, and what differs passes: new passes
local_of_full      new passes: the local check passes (it only skips tests)
local_iff_full     given old passed, the two answers are one
old_is_needed      without old passing, a shared footprint that fails is missed: counterexample
apart_of_flagged   face pairs: only a pair with a changed face can newly cross
boxes_apart        faces in disjoint boxes share no point
kept_loop_same_answer    within a plane face: loops new and gone away from a probe leave its even-odd answer
material_same_outside    boundaries differing only in R: a ray clear of R gives the same material
ray_clear_of_box, outside_box_no_material    a point past a box has a ray clear of it, and is outside what the box holds
cover              every rule to how it is covered; exhaustive by the match
```

## Every rule of `check`

```
rule                                   runs                    theorem
edge-uses, edge-same-way, edge-degenerate  whole body, always  none needed: the same function as check
face-no-loops, loop-empty, loop-broken     whole body, always  none needed
duplicate-name, euler, shell-euler, vertex-fans  whole body    none needed
vertex-off-edge, vertex-tolerance, edge-off-face  edges not same   full_of_local, edge footprint
outer-not-first, loops-cross                      faces not same   full_of_local, face footprint (a plane face's loops-cross runs on all its loops)
face-wrong-side                        faces not same, and on a plane face only coedges near a change  full_of_local, kept_loop_same_answer
shells-overlap                         faces changed or within where material changed   material_same_outside, outside_box_no_material
```
`selfx::crossing` (not in `check`, the operations' own test) takes the flags the operation gives or-ed with `changed_faces`, so a wrong
report can only cost time: `apart_of_flagged`.

No rule could not be made local. The whole-body rules are cheap (under 0.5% of a regen together), so they simply run.

## Assumptions that remain

1. Each test is a function of its footprint: `same_edge` reads what `check_geometry` reads, `same_face` what `check_sides` reads. The Rust is
   outside the model, so this is what the tests below hold it to. `uv_loop`, `draw`, `loop_samples` read only the face's surface and sense and
   its loops' edges and end points.
2. `==` on floats treats 0.0 and -0.0 as equal, so a footprint can match with a zero's sign different. A surface or curve is compared with
   `==` (procedural: the same `Arc`). NaN compares unequal, so it is only retested.
3. Jordan-Brouwer for a closed shell: parity along any ray tells material. `check_shells` itself assumes this.
4. A plane face's new loop lies in the hull of its edges, and a chord of an edge in the edge's box: both plain facts, not in Lean.
5. `old` passed. Regeneration does not establish this: the worker does, stage by stage, and `old_is_needed` shows it cannot be dropped. Hence
   the chamfer's local check lands together with the worker.

Stripped: nothing is assumed of the operation that made the body (the diff finds what changed; a name only picks which old entity to compare
with); nothing of arena order or ids.

## What it has found

- `loops-cross` (added 2026-10-31: a fillet's rim overshoot got through `check`, which did not test a face's loops against each other): a crossing of the default chords is
  believed only if the chords drawn 8 times finer cross too, at more than about 6 degrees. Without the first, p06075 and a ruled fuzz part were false alarms; without the second, a cylinder
  tangent inside another (scoreboard `parallel_tangent_inside`), whose chords cross beside the touch at any resolution. So a crossing shallower than 6 degrees is not found. A chord whose middle on the surface lies further from its ends than they are apart is left out: history seed 30's screw face is drawn a whole turn off at one
  join, one chord long in parameters between ends that are one point (the face is sound, and meshes).
  5 of 100 traced-fillet fuzz parts fault at their fillet.
- `duplicate-name`: threads through cross holes, t06025, t06096: spans of different pieces of one pair of whole faces between the same edges were ranked only within a face
  pair. Fixed in `nary::name_spans`. A name only at a mid stage, gone once finished (t06032), is not a fault: `verify::stage` asks the finished stage.
- `vertex-tolerance`: Fusion 40887_454ffc48_0010, an edge about 2e-5 long.

## Why only some coedges of a plane face

A plate's top face changes with every hole, and retesting it is quadratic. A coedge of a loop kept as it was tests the same as before when the loops
new and gone have edges at least 2 long (only an edge under 2 sets the step below 0.1) and lie at least 0.4 from it (4 steps, so they set no clearance,
and every probe is outside their box).

## How it is held to the Rust

`interface/forge-agent/tests/check_local.rs`, in CI:
- Against itself and against the stage before a hole, one fault in any one face, edge, vertex or coedge: `check_against` gives exactly `check`'s answer, and
  every rule named has fired. A shell moved, loops moved over a rim likewise.
- `check_against_both_ways(true)` runs both on every checked operation: the bench (`bench_parts.rs`), the random histories (`history.rs`) and `fuzz/corpus.txt`, panicking
  if the old body fails or the answers differ.

`mutate.py`, by hand from a scratch copy: loosens each comparison and each locality test in turn; the tests must fail. Regenerate the corpus with
`python3 research/check-local/fuzz/gen.py DIR 0 60` and `cat DIR/*.fc`.

## Cost (instructions:u)

Every feature checked inside regen, tried first: seq clamp +127%, conrod +130%, valve +123%, gearbox +59%, frame +47%, jack +28%, heatsink +9%; the suite's stress parts up to x43
(torture) after the reductions. Taken off the regen path for that reason (David, 2026-10-31).

As shipped, against pipe-6: gate sum 11988M to 11602M, bracket -30%, heatsink -21%, hub -17%, insert -9%, volute -8%, no line up. See the commit's numbers for later moves.

Delay to the flag, one thread at load 40, every stage from scratch: heatsink 0.17 s, gearbox 0.06 s, crank 0.03 s, frame 0.03-0.17 s, volute 0.02 s; the save's whole check 0.02 s or less.

Dropped: reusing the uv loops a boolean drew (a different discretisation changes verdicts at the margin, so local would no longer equal full).
