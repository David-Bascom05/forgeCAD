# Shell: what stops it on real parts, and what topology changes do

Shell keeps a body's topology and moves its faces to their offset surfaces (`forge_ops::rebuild`).
This records what real parts ask of that, how it was measured, and what was tried. No dataset content
is here; run output stays in `~/forgecad-data`.

## How measured

Every CADFS (1 in 20) and WHUCAD part file with a shell, cut after its first shell line, through
`forgecad query F check` and `forgecad build F`: 681 parts reach the shell. Volumes against the source
for parts whose last feature is the shell. OCCT through `tools/occt/oracle.py` on the same files
(`oracle dir:DIR`). The driver scripts are ten lines over `research/complexity/common.py`'s loaders.

```
                            before   after
CADFS   built                69%      80%   of 343
WHUCAD  built                71%      80%   of 338
judged against the source   208 of 211 match    224 of 234 match
```
Of the 10 that differ, 7 have a source face count the translated part cannot reach (the importer left
something out), so the shell is not what is being judged; the rest are not all read.

What the classes were, on that census: two opened faces side by side, 65 (rims now end along the edge
they share); an opened plane its neighbours leave tangentially (walls end in strips square to it: the
volumes match the source, OCCT's whole-solid answer on such a part was not hollow); tangent edges and
procedural seams (`moved_conic`); arcs ending on the axis (a spindle torus offsets to a spindle).

## Local events (`forge_ops::wavefront`)

A face moved can stop meeting its neighbours as it did. In a moving wavefront (straight skeletons; the
offset events of Forsyth, "Shelling and offsetting bodies", 1995, as later papers describe it: offset,
trim or extend, rebuild the closed model) there are local events and global ones:
- an edge shrinks away: its end faces meet instead (flip);
- a vertex of four faces no longer lies on one point: it parts in two along a new edge.
Both are applied where the rebuild stops, and it goes on. They bring few parts on their own (9 of 135
with the spindle), because what is left is mostly global.

## Tried and dropped

Inner solid as the body less a thickened cell per face, the shell as the body less that (a Minkowski
reading of Rossignac and Requicha 1986). On the 135 refused parts: 26 built valid, 0 of 5 judged matched
the source. Cells of adjacent faces leave wedges reaching the concave edges (zero thickness, 51 parts);
`thicken` itself refuses most faces of real parts (212 no exact wall, 190 collapses).

## Open

Global events: walls of a thin flange meet (the flange turns solid, no cavity in it). `SelfIntersecting`
37 and most of `Collapsed` 33 are this. Yan et al. (2004, LNCS 3045 family; cites Forsyth) make the raw
offset, correct its self-intersection, then a regularized boolean: the correction is the part still
needed here. Parasolid's and ACIS's methods are not public.
