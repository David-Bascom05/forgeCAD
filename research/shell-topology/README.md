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
- a face whose two ends shrink away together (a wall square to a face whose outline narrows to
  nothing: a prism's edge event, glyph walls) goes, and its two sides become one edge.
They are applied where the rebuild stops, and it goes on. They bring few parts on their own (9 of 135
with the spindle), because what is left is mostly global.

## Tried and dropped

Inner solid as the body less a thickened cell per face, the shell as the body less that (a Minkowski
reading of Rossignac and Requicha 1986). On the 135 refused parts: 26 built valid, 0 of 5 judged matched
the source. Cells of adjacent faces leave wedges reaching the concave edges (zero thickness, 51 parts);
`thicken` itself refuses most faces of real parts (212 no exact wall, 190 collapses).

## Islands (global, the first case)

A prism standing on a plane inside one of its inner loops (walls square to it, a top facing the same
way, a convex footprint of lines) whose footprint the thickness reaches the middle of from every
wall: in the mitred offset its walls cross and leave nothing (2D: the convex polygon's half-planes
moved in by t meet in no area, checked by clipping), so the plane's offset runs on across it and
just loses that loop (`shell::islands`, `without`). A boss thinner than twice the wall, a letter's
counter. Closed forms in `shell_events.rs`; the glyph plates and heatsink checked by the distance
definition below. Not yet: footprints with arcs, islands on a curved face, and peninsulas (a tongue
of material joined to the rest along one side, two parallel walls meeting all along: E's arms,
and the heatsink's fins, whose root fillets' offsets must then meet each other in a ridge).

The distance definition used to check (scratch tool, not shipped): points near the skin without
the opened face, each in the part, in the shell and its distance d to that skin (a fine mesh):
nearer than t must be in the shell, further than t must not, except within t of both faces of a
concave edge (the mitre leaves more wall there than the distance does) and by the opened face's rim
(the walls run on to it). It finds a lobe left out on purpose (the island dropped while thicker).

## Threads (David's option B, 2026-11-01)

A thread's flanks are screw surfaces whose offsets meet nothing where a hole cuts them (bench conrod,
frame, valve, gearbox, jack all refused there first). Offsetting them would also give a wall that
follows the helix. Instead the part is hollowed as if the thread were not there: each thread's ring
(`thread::envelopes`, about its axis from the minor radius less P/4 inside a bore, P/8 round a rod,
to the major plus P/8 inside, P/4 outside; from 1.5 P before its start to P/4 past its end, 1.25 P
with a run-out) is taken out of the body, that body's inner solid is made (`forge_ops::offset_solid`)
and the part less it is the shell. The ring holds every screw face, the chamfer cone in the material
and the run-out, with room so no face of it lies on one of the thread's: so the thread-free body lies
inside the part and every wall is at least the thickness, measured from the major radius in a bore
and the minor round a rod (from the major there the root would be thinner). Outward, the ring is
added and the outer solid less the part is the shell. `threaded_shell` (forge-model) checks the
volumes: the part less the solid must lose exactly the solid's volume, else the solid left the part.
Closed forms in `screw_parts.rs` (`shell_of_a_tapped_block`, `shell_of_a_threaded_rod`).
After it, none of the five thread parts stops at a thread; each stops at another face first.

The wavefront's event search (`rebuild_moving`) tried both partings of a vertex at every level: on
crank, once the thread no longer stopped it first, the same refusal came back 2^8 times at half a
second each (530 s). An entity whose every event failed further on is now failed at once.

## Open

Global events: walls of a thin flange meet (the flange turns solid, no cavity in it). `SelfIntersecting`
37 and most of `Collapsed` 33 are this. Yan et al. (2004, LNCS 3045 family; cites Forsyth) make the raw
offset, correct its self-intersection, then a regularized boolean: the correction is the part still
needed here. Parasolid's and ACIS's methods are not public.
