# Driving ForgeCAD

A part is a `.fc` text file: its feature history, one feature per line, rebuilt top to bottom.
Units mm, angles degrees (`1.2r` is radians). `#` starts a comment. `name:` labels a line.

```
base: extrude 6 rect -30 -20 60 40          # new solid: 6 up from xy, rect x y w h
wall: extrude 6 xz at 0 20 0 rect -30 0 60 40
hole 5 -xy at -20 -10 6 thru                # dia, frame (z drills in), depth|thru
pattern hole1 lin 1 0 0 3 20                # dx dy dz count step
extrude cut 2 -xy at 0 5 6 circle 0 0 4     # op: new add cut keep (default add)
fillet 3 (edges between base wall)          # radius then refs; a later radius applies after it
chamfer 1 @9f3c @a1e0                       # @handles come from `query`
```

Features (`[op]` defaults to new for the first solid, add after):
```
extrude [op] EXT FRAME LOOPS     EXT: N | sym N | two N M | thru [back|both] | to FEAT [@face] [off N]
revolve [op] [TURN] FRAME [axis x|y|ox oy dx dy] LOOPS     TURN: like EXT in degrees, default 360
ball [op] R FRAME                fillet R REF.. [R REF..]   shell T [out] REF..   draft A FRAME REF..
chamfer D REF.. | chamfer D D2 face REF REF.. | chamfer D ang A face REF REF..
hole D FRAME [cb D2 DEPTH | cs D2 A] (N | thru | to FEAT [@face] [off N]) [pt A] [thread SIZE PITCH]
pattern (body | FEAT..) (lin DX DY DZ N STEP [DX DY DZ N STEP] | circ OX OY OZ AX AY AZ N [A] | mirror FRAME)
plane FRAME                      a work plane for `to`, or sketch on it with `on FEAT`
FRAME: [xy|xz|yz|-xy|-xz|-yz | z A B C [x A B C] | on FEAT] [at X Y Z]     default xy at 0 0 0
LOOPS: rect X Y W H | rrect X Y W H R | circle X Y R [cw] | poly X Y X Y.. | loop (line AX AY BX BY | arc CX CY R A0 SWEEP)..
FEAT: a label, or kind+count (`hole2` = second hole), or `fN` (line N's feature)
REF: @handle (a stable entity name prefix) | (selector), resolved against the part at that line
```

Selectors (`query`, or inline in parentheses): `faces|edges|verts` then filters, applied in order:
```
plane cyl cone sphere torus blend | line circle curve        kind (planar, circular work too)
normal +z | normal A B C | along +x                          face normal, edge direction
r=3 r<3 r>3   longer 5   shorter 5                           radius, edge length
of FEAT   between FEAT FEAT   on REF                         made by; shared by two; touching
top bottom left right front back   near X Y Z                keep the extreme ones; the nearest
```

Commands (terse text; add `--json` for JSON):
```
forgecad build F                 ok|error line N: what, hint; volume area bbox counts ms
forgecad query F SEL [--at FEAT] handles, one per line: @9f3c plane +z z=6 area=2400
forgecad measure F REF [REF]     distance / angle / radius;  measure F mass | wall
forgecad render F [iso|front|top|right] [--labels SEL] [--px 400] [-o out.png]
forgecad check F                 validity of the result and after every feature
forgecad diff A B                first changed line, volume/bbox change, faces added/removed
forgecad serve | forgecad mcp    JSON-RPC / MCP on stdio; same commands, cached incremental regen
```
Handles are prefixes of names derived from the history, so they survive edits below their feature
and break (with a hint) when an upstream feature changes; selectors survive more edits.
