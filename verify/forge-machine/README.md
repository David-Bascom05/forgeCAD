# verify/forge-machine: the G-code interpreter's spec (L1)

Lean 4.36.0 and Mathlib v4.36.0 (shared with verify/forge-cam: its `.lake/packages`), standard
axioms only, no `sorry`. The model is `ForgeMachine/Interp.lean`: kernel/forge-machine/src/gcode.rs
at the level of parsed words in integer units (motion codes, G90/G91, G98/G99, G28, axis words over
any number of axes, I J, R, Q, F, T; canned cycles 81 82 83 84 85 73).

```
(cd ../forge-cam && lake exe cache get) && lake build && lake env lean Axioms.lean
```

| theorem | says |
|---|---|
| `holds` | in G90 an axis without a word keeps its value |
| `incremental_target`, `incremental_eq_absolute` | G91 adds to the known position; it equals the absolute form of the prefix sums |
| `pecks_bounds`, `pecks_sorted`, `pecks_last`, `pecks_length` | a peck's depths lie in [Z, R), strictly decrease, end at Z, and number ⌈(R − Z)/Q⌉ |
| `hole_last`, `hole_feeds_above` | every hole ends with the rapid to the retract level; it never feeds outside [Z, R] |
| `home_forgets` | G28 goes through the intermediate point; the homed axes are then unknown to the program |
| `step_refines`, `refines_post` | on forge-cam's post language (ForgeCam.Post), whatever that control model reads, this interpreter does: same motions, in order |
| `print_wf` | forge-cam's printer writes motion codes 0–3 only |
| `posted_program` | **the posted program does what was planned**: the interpreter run on the printer's output gives the program's motions less null moves (forge-cam's G6 `round_trip` carried through `refines_post`) |

## What is not covered (open, not assumed)

- The character level (digits and words to integers): the Rust parser is tested (L6) against
  forge-post's reader on 240 random programs, four dialects (tests/gcode_vs_post.rs); no proof.
- The Rust code is not translated (L4): gcode.rs is related to this model by reading and by the
  differential test, not by Aeneas.
- Lathe words (T0101, G50, G96/G97), G53, G43/G49 and work offsets: modelled in Rust only (they
  change offsets and the spindle, not the program-coordinate path this model states).
- The kinematics, timing, collision and removal (sim.rs, collide.rs, removal.rs): no Lean model.
  collide.rs's `Clear` rests on forge-math's interval contracts (L5, outward rounding; dmath sin/cos
  within 1 ulp), not proven here.
