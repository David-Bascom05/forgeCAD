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

- `ForgeWeld/Extreme.lean`, `ForgeWeld/Cell.lean` (L3 for the long point with several cuts):
  `line_max` (on an interval the minimum of affine functions is matched at an end or a crossing),
  `move`, and `cell_max`: over a bounded convex cell, the reach is matched at a corner, a ridge on
  an edge or an apex (forge_weld::frame `extreme`'s candidates, with the same determinants).
- `ForgeWeld/PolyMax.lean`: a polygonal section in the boolean's terms (membership by the signs
  against its edge lines; a box round it): `section_max` (every section point is matched by a
  candidate of its closed face: two lines meeting, a ridge on a line, an apex) and `face_sound`
  (every such point is a limit of section points). So the long point is the best candidate among
  the meetings of the edge lines and ridge lines in the section's closure: what `extreme` computes.

Not yet: arc members and arc sides of sections (ℚ(√·), the L2 step), L2–L6.
