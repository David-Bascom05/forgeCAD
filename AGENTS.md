# Driving ForgeCAD

A part is a `.fc` text file: its features, one a line, rebuilt top to bottom. Units mm, degrees.
Change it with `act`; the reply says what happened. `help TOPIC..` has the full grammar of anything.

```
base: extrude 6 rect -30 -20 60 40          # first solid: 6 up from xy; rect X Y W H
wall: extrude 6 xz at 0 20 0 rect -30 0 60 40
hole 5 -xy at -20 -10 6 thru                # dia, frame (its z drills in), depth|thru
pattern hole1 lin 1 0 0 3 20                # copy hole1: dir, count, step
extrude cut 2 -xy at 0 5 6 circle 0 0 4     # op: add (default) cut keep new
fillet 3 (edges between base wall concave)  # radius, then refs
chamfer 1 @e78d2213f @e172f9490             # @handles come from `query`
w = 40                                      # a parameter: any number below may be w/2, (w + 1)..
```

`at X Y Z` is a frame's origin, its z the extrude or drill direction: xy +z, xz -y, yz +x, -xy -z
(`help frame`). Loops: rect circle rrect ellipse poly spline thru loop (`help loops`). `help` lists every topic:
sketch, revolve, sweep, loft, coil, thread, text, import (a STEP, .brep or IGES file), shell, draft, rib, hole, pattern, bodies, ...

Tools (CLI `forgecad TOOL F ..`, or MCP; lines on stdin if none given):
```
act F 'LINE'.. [--base N]   all lines or none. Any .fc line goes on the end; also
                            NAME = EXPR | before FEAT LINE | edit FEAT LINE | set FEAT K TEXT |
                            dim SKETCH ROW VALUE | rm FEAT | mv FEAT FEAT2|end | suppress FEAT |
                            unsuppress ID | rename FEAT NAME | undo | redo
query F SEL [--at FEAT]     find entities, for handles: `edges of hole1`, `faces plane normal +z top`
query F stat|file|check|dims|since N
measure F REF [REF] | mass | wall
render F [VIEW] [--labels SEL] [--px N] [-o F.png]
export F [ap214|ap242] [tol MM] [-o F.step]
help [TOPIC]
```
FEAT: a label, kind+count (`hole2`), `fN` (Nth line). REF: `@handle` or `(selector)`, in a line too.
Selectors (`help selectors`): `faces|edges|verts`, then filters: plane cyl cone .. | line circle |
convex concave | normal +z | along +x | r=3 | longer 5 | of FEAT | between FEAT FEAT | top bottom
left right front back | near X Y Z.

A reply is a diff: `ok v7; +4 faces, volume -120.4 mm3, valid; picked fillet1:4`. Check the counts.
An error gives the line, the reason and, where it can, `try:` a replacement that builds; nothing
was applied. A `.fc` file is plain text: hand edits are fine, `act` picks them up. New file: `act`
on an empty `.fc` file.
