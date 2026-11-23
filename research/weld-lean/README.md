# Weldments in Lean (research/PROOF-STANDARD.md, L1)

Core Lean over ℚ³ on the boolean's own ground (verify/forge-exact: `Ref.V3`, `Pred`, planes,
`wall_nowhereDense`), no Mathlib. `lake build`, then `lake env lean Axioms.lean` (only propext,
Classical.choice, Quot.sound).

- `ForgeWeld/Member.lean`: a straight member is its section's prism less its cuts' half-spaces;
  `straight_in_support`, `miter_apart`, `butt_apart`, `runon_flush`, `apart_nowhereDense`.
  The "apart without a boolean" shortcut is proven for straight members (or a shared opposite
  plane); for arc members it is false in general and forge-lang keeps the boolean check there.
- `ForgeWeld/Node.lean`: the rank rule (`cutsOf`) and `node_apart`: every pair at a node lies on
  the two sides of one plane.
- `ForgeWeld/Envelope.lean`: the intended envelope (prisms restricted by the rule's half-spaces,
  gap slabs out, run-ons in), `union_eq_envelope`, `envelope_disjoint`.
- `ForgeWeld/CutList.lean`: `reach_le`, `reach_attained`, `long_point_unique`, `path_crossing`:
  the cut list's overall and length are the half-spaces' extents.
- `ForgeWeld/Section.lean`: `support_of_corners` (polygonal sections), and Green's moments:
  `line_eq_fan_x`, `line_eq_fan_y` round a closed loop, `fan_ne_line_on_an_edge`.

Not yet: arc members and arc sides of sections (ℚ(√·), the L2 step), L2–L6.
