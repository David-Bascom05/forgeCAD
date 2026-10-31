# Driving ForgeCAD

A part is a `.fc` text file: its feature history, one feature per line, rebuilt top to bottom.
Units mm, angles degrees (`1.2r` is radians). `#` starts a comment. `name:` labels a line.

```
base: extrude 6 rect -30 -20 60 40          # new solid: 6 up from xy; rect X Y W H
wall: extrude 6 xz at 0 20 0 rect -30 0 60 40
hole 5 -xy at -20 -10 6 thru                # dia, frame (its z drills in), depth|thru
pattern hole1 lin 1 0 0 3 20                # copy hole1: dir, count, step
extrude cut 2 -xy at 0 5 6 circle 0 0 4     # op: new add cut keep (default add)
fillet 3 (edges between base wall concave)  # radius, then refs; a new radius applies onward
chamfer 1 @d644 @b2e4                       # @handles come from `query`
```

Features (`[op]` is new for the first solid, add after):
```
extrude [op] EXT FRAME LOOPS     EXT: N | sym N | two N M | thru [back|both] | next [back] | to FEAT [@face] [off N]
revolve [op] [TURN] FRAME [axis x|y|OX OY DX DY] LOOPS     TURN: like EXT, in degrees; default 360
ball [op] R FRAME     fillet R REF.. [R REF..]     shell T [out] REF..     draft A FRAME REF..
chamfer D REF.. | chamfer D D2 face REF REF.. | chamfer D ang A face REF REF..
hole D FRAME [cb D2 DEPTH | cs D2 A] (N | thru | next | to FEAT [@face] [off N]) [pt A] [thread SIZE PITCH]
pattern (body | FEAT..) (lin DX DY DZ N STEP [DX DY DZ N STEP] | circ OX OY OZ AX AY AZ N [A] [step] | mirror FRAME)
sweep [op] FRAME LOOPS path PATH [extend N M | to FEAT [@face] [off N]]     profile square to the path's start
coil [op] FRAME [axis ..] LOOPS (pitch P turns N | turns N height H | pitch P height H | pitch P to FEAT [@face] [off N]) [taper A] [left]
loft [op] [smooth [start W] [end W]] (profile FRAME LOOPS).. [rail I PATH.. | centre PATH]     I: vertex (where segment I starts)
PATH: (line AX AY AZ BX BY BZ | arc FRAME R START SWEEP)..     tangent-continuous; an arc's FRAME is its circle's, centre at `at`
plane FRAME                      a work plane, for `to FEAT` and `on FEAT`
FRAME: [xy|xz|yz|-xy|-xz|-yz | z A B C [x A B C] | on FEAT] [at X Y Z]     default: xy at 0 0 0
  xy: normal +z; xz: sketch x,y = world x,z, normal -y; yz: world y,z, normal +x; -: flipped
LOOPS (first is outside, rest are holes): rect X Y W H | rrect X Y W H R | circle X Y R |
  poly X Y X Y .. | loop (line AX AY BX BY | arc CX CY R START SWEEP)..
`next`: up to the next face met. `to`: a plane feature, or a feature's face (plane, cylinder..).
Fillet R: 3 | 2..4 (start to end of each edge) | 0:2,0.5:4,1:2 (fraction along:radius).
FEAT: a label, kind+count (`hole2` is the 2nd hole line), or `fN` (line N's feature)
REF: @handle | (selector), found on the part as it stands above that line
```

Selectors (`query`, or inline in parentheses): `faces|edges|verts`, then filters in order:
```
plane cyl cone sphere torus blend | line circle curve      kind
convex concave smooth                                      how an edge's faces meet
normal +z | normal A B C | along +x                        plane normal; line direction
r=3 r<3 r>3   longer 5   shorter 5                         radius; edge length
of FEAT   between FEAT FEAT   on @h                        made by; joining two; touching
top bottom left right front back   near X Y Z              keep the extreme ones; the nearest
```

Commands (a few lines of text; `--json` for JSON):
```
forgecad build F                    ok, or error line N + hint; approx volume, area; bbox, counts
forgecad query F SEL [--at FEAT]    @d644 line -x len=60 at 0,20,40   (one per hit)
forgecad measure F REF [REF]        area|length radius centre; two: distance angle
forgecad measure F mass|wall        volume area centroid inertia; thinnest wall
forgecad render F [VIEW] [--labels SEL] [--px 400] [-o F.png]   VIEW: iso front top right..
forgecad check F | diff A B         validity after each line | what B changed from A
forgecad serve|mcp [--root DIR]     stdio JSON-RPC / MCP, .fc files under DIR (default .) only
```
`serve` takes `{"method":"build","params":{"file":"b.fc"}}`; params: file sel at what to view
labels px other. Rendered labels are handles without the `@`.

Handles are prefixes of names derived from the history: they survive edits below their feature,
and fail with a hint when an upstream edit renames them. Selectors survive more edits.
