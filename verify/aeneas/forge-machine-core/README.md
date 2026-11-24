# verify/aeneas/forge-machine-core: the G-code interpreter, from the shipped Rust (L4)

`tools/aeneas/translate.sh forge-machine-core` translates the real `kernel/forge-machine-core`
(release build, overflow checks on), as `translate.conf` selects, into
`lean/ForgeMachineCore/{Types,Funs}.lean`: 35 definitions, none opaque. The proofs in
`lean/ForgeMachineCore/Refine.lean` are about that translation, against the interpreter's spec
`verify/forge-machine` (ForgeMachine/Interp.lean). Each rests on `propext`, `Classical.choice` and
`Quot.sound` only (`lean/Axioms.lean`), plus the trusted base of verify/aeneas/README.md (Charon and
Aeneas translate faithfully).

| Rust (kernel/forge-machine-core/src/lib.rs) | Lean | what is proven |
|---|---|---|
| `step` | `step_spec` | **one block**: `Ok (events, state)` exactly when the spec's `step` gives `some`, with the same events and state (through `E`, `SA`, `BA`); `Err` exactly when it gives `none`. The post-state keeps the invariants the next block needs (length n, a well-formed cycle) |
| `step` iterated | `run_spec` | **a program**: `runT` (the shipped step over a block list) is the spec's `runFrom`, for every program whose spec run stays inside the bounds (`ReachBd`) |
| `arc` | `arc_spec` | G2/G3: plane axes, centre from I/J/K or R, full circle, every refusal (R0, R with centre, full-circle R, no centre) |
| `hole` | `hole_spec` | the canned cycle's events: G81/G82 drill and dwell, G83/G73 pecks (`hole_loop_spec`, `pecks_flat`), G84/G85 feed out |
| `motion_code` | `motion_code_spec` | G0–G3, G80 and the cycle codes, the cycle's words, `CycOK` kept |
| `target`, `put`, `step_loop0/1` | `target_spec`/`target_rel`, `put_P2`, `step_loop0_spec`, `step_loop1_spec` | G90/G91 targets (an incremental word from an unknown position refuses), the cycle's retract position, which axes a block words, G28's homed axes and position |

With verify/forge-machine's `refines_post` and `posted_program` (the spec refines forge-cam's post
model), this says the shipped interpreter does the planned motion for every program forge-cam
posts, within the bounds.

## Hypotheses (stated, not assumed)

- Every coordinate and word is within 2^29 in the control's integer units (`Bd`, `BdO`, `BlkOK`):
  sums of two such values cannot overflow the `i64` the Rust computes in, so the theorem also says
  the release build, which wraps, computes the same.
- At most 2^16 axes, every position and block of length n, the axis indices below n.
- The cycle is well formed (Z at or below R, bounded: `CycOK`), and so are the peck clearance and
  chip lift.
- For a run, every position the **spec's** run reaches stays within 2^29 (`ReachBd`): a property of
  the program and the spec, not of the implementation. G91 can walk a position out of range; the
  theorem is about programs that do not.

## What is not covered

- The shell, kernel/forge-machine's gcode.rs: reading text into words, and the words that do not
  move the axes (tools, spindle, coolant, offsets, G53, dwell). It is checked against forge-post's
  reader (L6, tests/gcode_vs_post.rs).
- `runT` is the shipped `step` iterated in Lean; the shell's own loop around it is not translated.

## Changes to the Rust for the translation

The same results bit for bit (tests/bits.rs's digest unchanged): the four modal-word matches at the
top of `step` became the helpers `or_word`, `or_bool` and `or_plane`.
