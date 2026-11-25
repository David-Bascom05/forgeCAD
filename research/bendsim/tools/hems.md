# Hems and offsets in the bend simulator: design (P1, P2)

Status: in progress on bendsim/hems. Plan approved by the coordinator 2026-11-25 (order P0 data,
P1 hems, P2 offsets, P3 pickers and P5 showcase). Both a combination (spring-action) hem die and
separate pre-bend and flattening stations are modelled; the planner picks by what is mounted and
feasible, and a manual choice is honoured.

## What holds already (verified)

- forge-sheet models hems (closed, open, teardrop, rolled; `kernel/forge-sheet/src/hem.rs`). A hem
  is a joint of kind `JointKind::Hem` with a turn (about pi for closed and open) and an inside
  radius (open: gap / 2; closed: the shop's closed gap, never below `GAP_FLOOR`).
- With the refusal in `Model::new` lifted, the hem joint is a `BendM` like any bend, and the full
  refold equals the builder's part to the bit (open hem, up and down: `tests/hem.rs`).

## P1 design: a hem is two strokes

1. Pre-bend: an ordinary air bend of the hem's joint to the pre-bend turn (WILA: "the first step
   when creating a hem is to make a 30 degree bend", i.e. 150 degrees of turn), on an acute punch
   and die. Every existing check runs. `StepCtx::theta` is the pre-bend turn, not the hem's.
2. Flatten: the same joint from the pre-bend turn to the hem's own, between a flat punch face and
   a flat die (a flattening pair, or the flat bed of a combination die). The part's base wall lies
   flat on the die (the placement's tilt is 0, not phi / 2) and only the leg turns. The punch
   face's height is the part's highest point at turn phi (the fold's outer skin, the leg's far
   end), monotone decreasing in phi on [pre-bend, pi], so the stroke is parameterised by phi, as
   the bend phase already is, and the B&B encloses it with intervals. The end state equals the
   built hem (the refold test).
3. A hem's flatten follows its pre-bend at once (the part goes from the acute station to the
   flattening one, or stays on a combination die). A first version: no other bend between them.
4. Checks in the flatten: the rest of the part against the flattening punch, die and bed (the
   certified tool checks over the phase), the leg against the punch (the face meets it only at the
   top), the hem's two layers kept apart (gap at least the hem's), and the flattening force from
   the maker's cited table, else "not checked".

## Open questions

- Teardrop hems: which tool forms the closing leg (a teardrop die, a spacer)? Refused with the
  reason until a maker's tool is cited.
