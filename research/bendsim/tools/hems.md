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

## P1 status (bendsim/hems)

Done, with tests: the refold to the bit; `StepCtx::flattening` (base flat, face on the top, phi0);
`Stroke` on `Step`, `StepCtx::of_step` / `of_stroke`; tools' `Role` and `max_thickness` (tooling
files and the catalogue); the planner's pre-bend (`first_turn`, 30 degrees included less 1e-9 rad)
then flattening (`Search::flattened`, `sim_flat`, `finish_all`); `Rule::NoFlattening`,
`Rule::ToolRating`; flat sections for flattening tools; the scene from phi0; the reply's `stroke`.
Air bends time the same before and after (deep box: 2.69-2.87 s both).

Left: real flattening tools in the catalogue (section dimensions being collected); the combination
spring-action die (`Role::Hem`: how it flattens, to confirm); the flattening force table (columns
to confirm); teardrop hems; forge-render's `rule_label` needs a fallback arm before the new rules
merge (asked of BENDSIM-ENV).

## P2 design: offsets in one stroke

An offset pair forms two adjacent bends, j (off wall a) and k (off j's wall b, the web), equal
turns of opposite sign on parallel lines, in one stroke. The part's outer walls stay level and
the web tilts: zone coordinates of j keep wall a flat (as a flattening's), both zones turn by the
same phi. StepCtx gains `offset: Option<Off>`:

- pieces: wall b (the web) stays `Side::B`; zone k becomes `Kind::ZoneK` (moving: k.fold at phi,
  then its fixed iso into j's end coordinates); walls beyond k become `Side::C`, their iso into
  k's end coordinates (`(F_k . shift(ba_k))^-1 . F_w` at the construction's turns).
- `rel` for `Side::C`: k.end_apply(phi), then zone k's iso into j's end coordinates, then
  j.end_apply(phi), then the level placement (no phi / 2 tilt).
- `ztip`: the punch step's face on wall c's top skin (or the web's highest point), monotone in phi.
- rules: the part's offset (the web's rise at the end) against the tool's `z`; the thickness
  against the insert's rating (Mate's table read inversely: below the smallest Z rated for t,
  refused, citing it); segments, bed, stroke, open height; tools through the stroke.
- tools: `Role::Offset { z }` punch and die inserts, sections built from the published Z, angle
  and insert size (23 x 36, 27 or 32 x 45), cited as estimates where the shape between is drawn.
- planner: an offset pair is chosen where it is mounted and passes, and the two bends then count
  as one step (`Stroke::Offset { k }`); otherwise the two bends are bent as two air bends, as now.

## Open questions

- Teardrop hems: which tool forms the closing leg (a teardrop die, a spacer)? Refused with the
  reason until a maker's tool is cited.
