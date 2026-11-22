# A box a hair under a notched top: which decisions disagree (2026-11-22)

The part: a plate whose top T is at z=10, with a notch; a box added whose bottom B is at
z = 10 − (1e-5 − 4e-16), 4e-16 inside the 1e-5 tolerance, over the notch's corner
(verify/forge-proof tests, seeds 4, 24, 30, 482; branch worktree-agent-a2b4568cfb2a1a516).
The ulp property nudges the plate by a few ulps; 316 of 400 nudges failed.

The face pair decides once that B and T are coincident (`planes_coincident`). Every later
routine re-measures the same ~1e-5 gap against LINEAR on its own:

- `trim`, edge box test: is the wall∩T edge near the wall∩B curve at all;
- `trim`, vertex test: does that curve run along the edge (both ends within vertex tol);
- `point_in_face` on the wall: a point 1e-5 below the edge is On or Inside (Inside makes a second edge);
- `imprint`'s `on_edge`: a box edge crossing a notch edge ~1e-5 out of plane is found or missed
  (missed: T's split crosses itself, SplitCrossed);
- `place_cuts` / `end_vertex`: a crossing ~1e-5 from a corner merges or becomes its own vertex
  (a zero-length edge, EdgeDegenerate);
- `check`'s EdgeOffFace: a plate edge bounding B is 1e-5 + ulps off it.

Ulps flip some of these while the coincidence decision stays put.

The single rule: once two surfaces are decided coincident, every later decision about their
entities is made in one representative surface (project, or measure in its coordinates), never
across the gap; the gap is absorbed by the coincidence decision and carried as the affected edges'
and vertices' tolerance. Equivalently: decide "on / near" once per entity pair and look it up.

Applied at four places, gated to booleans with coincident planes measurably apart: 0 of 400 nudges
failed on that part (part 4 of the family still failed on 191, unchanged), ordinary parts within
±0.01% on every gate line. Not merged: the decide-once pilot (research/kernel-audit/README.md)
makes this the general rule rather than four more sites.
