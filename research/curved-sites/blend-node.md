# Tangent nodes: two rounds touching where their springs cross

## The finding

fg_mcp's test plate (60 × 60 × 6 with a 4 mm dent 2 deep) rounded on all its top face's edges,
`fillet(edges(of: base.end), r)`, was refused at scattered radii: 21 between 10.98 and 12.5 at 0.01
steps, and on a 0.1 grid from 2 to 30:

| r | then |
|---|---|
| ≤ 10.8 | the one fillet builds (89 of 89) |
| 10.9 – 24.3 | the one fillet is refused (Overflow at the dent's rim: its round and the outer ones meet on the top face); forge-model's `overlapping_rounds` rounds each group alone and intersects the two parts; that boolean was refused at about 1 radius in 20 (SplitCrossed, InconsistentClassification, ZeroThickness, once a result `check` refused), and builds came back at 12.6–12.9, 13.1–14.8, 16.8–23.2 |
| ≥ 24.4 | refused at every radius (Mitre at the plate's corners) |

So "the largest radius that builds" (fit_sizes) was whichever hole a bisection met; 12.5 was no limit.

## The mechanism

Both operands keep the top plane S (z = 6). The rim's round (a pipe about a circle, a spindle: its
ball rolls on S and pivots on the dent's floor rim) touches S along its spring, a circle about the
dent; each outer round (a cylinder) along a line. Where the circle crosses a line, both rounds have
S's normal: they touch. Near there each is z ≈ 6 − d²/2r, d its distance past its own spring, so
they meet in d_A = ±d_B, an X. The boolean traced that intersection with the marching tracer
(procedural × cylinder has no closed form), which near the node:

- ends a hair off it: a height error e moves the end about √(2 r e) (1.3e-10 in z gave 2.4e-5 and
  1.3e-4 along S), so the traced end and the springs' crossing (from the coplanar faces) became two
  vertices with a sliver between them (SplitCrossed on S), or
- traced the ridge twice, from both nodes (two edges on one curve: D9, ZeroThickness).

## The fix (kernel/forge-bool/src/node.rs)

For a face pair whose traced curves pass within two steps of a node, and only where every condition
below holds (else the trace stands as before):

1. both surfaces are tubes, recognised exactly: cylinder, torus, a pipe about a line or a circle;
2. both rolls off a plane face along an edge, the two planes coincide (S), each spine lies
   parallel to S at its radius from it, both on one side;
3. the node is the springs' crossing, in closed form on S (line × line, line × circle, circle ×
   circle), crossing at a sine of at least 0.02, on both springs' edges;
4. the mend stays within a quarter of min(r, a circular spine's radius) of the node, where each cap
   is one sheet, and four reaches clear of any other crossing of the two springs.

Each traced chain is cut where it comes within its reach of the node; each stretch that ran into
the node is walked back into it along both surfaces (`towards_touch`), ending on it exactly, its
tangent there the last chord's (the normals' cross product is noise at the node); a stretch that
runs along one kept already is the same branch, dropped.

Why one branch, proven (verify/forge-curved/ForgeCurved/BlendNode.lean): over S each cap within
its radius of the spring is the depth h(ρ) = r − √(r² − ρ²) (`cap_section`), strictly increasing
(`h_strictMonoOn`); where both faces lie past their springs, h_A(ρ_A) = h_B(ρ_B) has at most one
ρ_B per ρ_A (`one_branch`, `tubes_one_branch`), increasing (`branch_increasing`), and meets either
spring only at the node (`branch_meets_springs_only_at_node`). The other arm of the X lies where one
face or the other is not.

## Measured

- fg_mcp's plate, 10.9 – 24.3 at 0.01: 1341 of 1341 build and pass `check` (was ~5% refused);
  2 – 30 at 0.1: every radius to 24.3 builds, every one from 24.4 is Mitre: monotone.
- Volume, 10.0 – 24.3 at 0.1: strictly falling, each step's fall within 1% of the last's, across
  the switch from the one fillet to the two rounds at ~10.85 too.
- The exact curved reference (planes, cylinders, cones, spheres) cannot judge the part: its rim
  round is a pipe. Hence the smoothness test in place of an enclosure.
