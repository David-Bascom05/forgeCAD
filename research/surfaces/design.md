# Surface bodies and thin extrudes: design (2026-11-23)

## What the data asks for

All of CADFS's `featurescript_fp.zip` (450 307 programs) was read, outside the repo, as data. Counts
are mine; no dataset file is in the repository.

- **No program has a THIN body.** `bodyType` is `ToolBodyType.SURFACE` in 6107 programs and
  `ExtendedToolBodyType.SURFACE` in 91; `THIN` never occurs.
- **No program thickens, splits by a surface, encloses, or replaces a face.** Their feature calls are
  extrude, fillet, revolve, chamfer, cPlane, hole, shell, mirror, transform, sweep, loft,
  circularPattern, assignVariable, booleanBodies, deleteBodies and cPoint, and nothing else.
- **A surface is almost always a leftover.** Of 6198 programs with a surface body, 5320 never refer
  to it again, and 2232 make no solid at all. Where a later feature does refer to it (by the
  argument that holds the query):

  | use | refs |
  |---|---:|
  | extrude of the surface's faces as the region | 501 |
  | fillet of its edges | 213 |
  | mirror / transform / circularPattern of the body | 369 |
  | deleteBodies | 144 |
  | an extrude up to its face (`endBoundEntityFace`, second direction) | 112 |
  | loft `sheetProfileEntities` | 96 |
  | a plane, an axis, a path from its geometry | 150 |
  | booleanBodies | 1 |

- **In the 1914-part sample, most of the 73 "surface" parts are not surfaces.** 71 parts carry a
  `"surfaceEntities"` list on a solid extrude (the edges of the same sketch, left over from the UI).
  Onshape reads `surfaceEntities` only when `bodyType` is SURFACE. The importer treats any
  `surfaceEntities` as a surface extrude and gives up. 26 sample parts make a real surface body,
  and of those, 2 refer to it again (one extrude up to a swept surface, one sweep path).

The STEP truth (`step_truth.py`) measures solids only and counts sheets. So a surface left over in
a solid part has to be built as what it is, a sheet, and must stay out of the part's volume and area.

**Order, by value:**
1. The importer stops reading `surfaceEntities` on a solid extrude or revolve. No kernel change.
2. Sheets from extrude and revolve, so that leftover surfaces translate.
3. `upto` a sheet's face.
4. Thin extrude, thicken of a sheet, and split by a sheet: the brief asks for them, but no CADFS
   program uses them.

## Kernel: sheet bodies

**Representation.** `Body` gets `pub sheet: bool`. It is set only by the operations that make a
sheet: a surface extrude or revolve, and the transforms and copies of a sheet. It is never inferred
from the topology. A solid that loses a face stays a solid and fails `check` exactly as now (an
`EdgeUses` with 1). So no broken solid can pass as a sheet. A sheet holds one or more shells; each
is an open or closed connected surface with no material. `same_sense` names the sheet's front:
for an extruded chain, the right of the chain walking it with the sketch normal toward you. That
is outside for a counter-clockwise loop, the same side a solid's outward normal takes.

Multi-body parts keep each body separate (`bodies::Solid`). A sheet is one entry with `sheet` set.

**Topology rules for a sheet** (`check`, when `body.sheet`):
- every edge has 1 or 2 uses;
- a 2-use edge's uses are distinct and run opposite ways (orientable, as now);
- a 1-use edge is a boundary edge (laminar);
- loops close (as now);
- round each vertex, the leaving coedges form one fan, which is either a cycle (inside the sheet)
  or a chain (on its boundary). A chain starts at the coedge leaving along a boundary edge and ends
  where `rot` stops, at a coedge whose predecessor is a boundary use. So at most one boundary use
  leaves the vertex and at most one arrives, and both or neither do;
- per shell, V − E + 2F − L + B is even and at most 2, where B is the shell's number of boundary
  cycles (the 1-use edges, chained at their vertices). For a surface of genus g with B boundary
  components, χ = 2 − 2g − B, and V − E + 2F − L is χ once the faces are planar domains.
- Geometry is as now: edge on face, vertex on edge, the side test, loops not crossing, names.
- `check_shells` (material overlap) does not apply: a sheet has no material. Sheets that cross
  each other or themselves are excluded by construction. Extrude and revolve refuse a chain that
  crosses itself (`SelfIntersecting`) or a revolve whose chain crosses its axis. I will not claim
  that `check` proves embeddedness. It does not prove it for solids either: no face-against-face
  test exists today.

New `Problem`s: `BoundaryInSolid { edge }` (a 1-use edge where `!sheet`, which today's `EdgeUses`
already covers and which keeps its name and rule), `BoundaryPinched { vertex }` (two boundary
chains through a vertex: a bow tie) and `SheetEuler { shell, value, boundaries }`.

**How `Valid` extends** (verify/forge-topo-check, on the fundamentals branch). `ValidSheet` is
`Valid` with these changes:
- `twoUses` becomes `uses ∈ {[x], [x, y]}`, with x ≠ y and opposite senses when there are two;
- `oriented` holds for every coedge whose edge has two uses, and `partner` is none exactly on
  1-use edges;
- `rotInto` and `rotInj` hold where `rot` is defined; `rot` is partial;
- `oneFan` becomes: the walk from the vertex's boundary-leaving coedge, if there is one, else from
  the first, reaches every leaving coedge;
- `euler` takes the boundary count.

`check_sound_sheet`: `WF b → b.sheet → b.check corners = [] → b.ValidSheet corners`. The solid
theorem stays word for word, with a hypothesis `¬ b.sheet` added. `check` takes the solid route
whenever `sheet` is false, so the existing proof and the 861-body differential test carry over.
New mutants for the test: sheets with a dropped face (still a valid sheet, with B up by one), a bow
tie, a flipped face (`EdgeUsesSameWay`), and a solid with its `sheet` bit set.

**Everything else on a sheet:**
- **Mass:** area is the sum of face areas, by the existing per-face integrals. Volume, centroid and
  inertia are a typed `NoVolume { body }`. The part's volume and area sum solids only, and its
  sheets' area is reported apart (`sheet_area`), which matches the STEP truth's convention.
- **Mesh:** faces tessellate as now. Shared edges are still shared, and boundary edges are meshed
  once. Watertightness (verify/forge-mesh-check) holds per 2-use edge; for a sheet the statement
  becomes "every mesh edge on a 2-use edge is used twice, and on a boundary edge once".
- **STEP:** `OPEN_SHELL` in a `SHELL_BASED_SURFACE_MODEL` under a
  `MANIFOLD_SURFACE_SHAPE_REPRESENTATION`. A closed sheet is a `CLOSED_SHELL` there, not a
  `MANIFOLD_SOLID_BREP`. The STEP reader reads both back into sheets.
- **STL** writes solids only, and the reply lists the sheets it left out.
- **Selection and display:** faces and edges are selected as now. Add the selector filters
  `boundary` (1-use edges) and `sheet` (the bodies), and draw sheets two-sided in the renderer.
- **Booleans and features:** a sheet is never "the last body" that a solid feature acts on. A
  feature that names a sheet as its target is refused with a typed error, `OnSheet { feature, body }`.
  The exceptions are rigid moves and patterns of bodies (they move any body), `delete`, `thicken`,
  `upto` (the face's surface is used, as for any face, so it works unchanged), and `split`.

## Thin extrude and thicken

There is one way to make each. A thin extrude **is** `thicken(surface extrude of the chain, t, side)`:
the same code builds both, so the two cannot disagree. The existing `forge_ops::thicken` already
does most of it:
- exact offset skins (plane, cylinder, cone, sphere and torus offset in closed form; a spline's
  extruded surface by `blend::Offset`);
- re-meeting neighbours at interior creases (`rebuild_some`);
- walls that are planes, cylinders or cones.

It needs two extensions:
1. **The rim of a sheet.** The faces of a sheet that a thicken takes are all its faces. Its rim is
   the sheet's boundary edges, as it is now the edges where the chosen set ends.
2. **A crease on the rim.** At a chain's corner, the extruded sheet's two faces meet at an angle on
   the cap rim, and thicken now refuses that with `Crease`. The walls of the two rim edges there
   are the same cap plane, since a wall swept by the normals of a straight extrusion's top edge is
   the cap plane. So the rule becomes: at a rim vertex where the faces crease, the two rim walls
   must lie on one surface (then they join as one face), or meet in a line through the vertex's
   offset corner (then they re-meet). Otherwise it is `Crease`, as now.

At a corner the outer offsets are extended until they meet, and the inner ones are trimmed where
they cross: a sharp mitre, as Onshape and SolidWorks do. Exact geometry:
- lines offset to lines, and arcs to concentric arcs (`Collapses` when t reaches the centre);
- a spline's wall is `blend::Offset` of its extruded surface, so its cap edges are where that
  surface meets the cap planes, and they are exact;
- where offset arms cross themselves inside a corner tighter than t, the result is
  `SelfIntersecting` at that pair, never a patch.

## `.fg` syntax

```
sketch s(on: xy) {
  path wall(start: (0, 0)) { line(to: (40, 0))  tarc(to: (50, 10))  line(to: (50, 30)) }
  l = line((0, 40), (30, 40))
}
skin = extrude(s.wall, 20, surface: true)                # a sheet: one face per chain piece
shade = revolve(s.l, axis: Y, angle: 90deg, surface: true)
web = extrude(s.wall, 20, thin: 2, side: "both")         # a solid: the wall 2 thick about the chain
lid = thicken(skin.faces, 1.5, side: "in", new: true)    # thicken already exists; now takes a sheet
stop = extrude(base.r, upto(skin.faces))                 # upto already takes FACES
```

- `extrude(PATH | PROFILE, extent, surface: true)` and `revolve(.., surface: true)` make a sheet,
  always a new body named by the feature. PATH is what sweep takes: a sketch line, arc or spline, a
  `path` block, or a list end to end. A closed PROFILE gives a tube with no caps. With `surface`,
  `taper:`, `along:`, `cut`, `keep` and `thin` are `NotWith`.
- `thin: t, side: "out" | "in" | "both"` on extrude (and on revolve, by the same thicken) uses the
  same sides as `thicken`. "out" is the front, the right of the chain, which is outside for a
  counter-clockwise loop; "in" is the back; "both" is t/2 to each side. On a closed PROFILE, thin
  gives a tube wall. On a PATH, it gives a wall with square ends.
- `split(by: SHEET)`: a solid cut in two by a sheet that passes right through it. The sheet's rim
  must lie outside the solid, else `NotThrough { edge }` at the rim edge inside it. This is the
  only new boolean form. It is last in the order, and I propose to build it only if you want it
  without a dataset use.
- **New names:** the arguments `surface` and `thin` (`side` exists already, on rib and thicken), the
  error codes `NotWith`, `OnSheet`, `NoVolume` and `NotThrough`, and the selector words `boundary`
  and `sheet`. Each goes into research/forge-lang/lean (`Impl/Core/Scope` lists and lemmas, the
  built-in argument tables) and grammar.ll1 (no grammar production changes: these are arguments and
  selector words). There is no Lean toolchain in this container, so I will write the Lean changes,
  state the theorems, and hand the build to the IEEE session.

## Importer (tools/find-parts/recreate/cadfs.py)

1. `surfaceEntities` is read only when `bodyType` is SURFACE. This removes most of the sample's 73.
2. A SURFACE extrude or revolve becomes `surface: true`; `deleteBodies` of a sheet is dropped with
   it; `upto` a sheet's face as for any face.
3. The judge reports the solids' volume and area, which is the truth's convention, and the sheets'
   area apart.
4. Before and after come from tally.py on the same 1914-part sample, with 0 regressions.

## Gates I will run

- reliability scoreboard: 0 worse;
- FUZZ_SEEDS=150: 150 of 150;
- the exact callgrind gate against a baseline saved on 3dcdfb4: app, edit, cli and regen1 ≤ base.
  Solids take today's route bit for bit; the one new branch is `if body.sheet` in check, mass,
  mesh and STEP;
- native and wasm32-wasip1 identical;
- the OCCT oracle: sheet area against BRepGProp surface props, and thin extrude and thicken
  volume and area against BRepOffsetAPI_MakeThickSolid / BRepPrimAPI on the same chains.
