# Driving ForgeCAD

A part is a `.fc` text file: its feature history, one feature per line, rebuilt top to bottom.
Units mm, angles degrees (`1.2r` is radians). `#` starts a comment. `name:` labels a line.
A FRAME places a sketch or hole: `at` is its origin in world coordinates, its z the extrude or
drill direction. A hole's mouth is at the origin.

```
base: extrude 6 rect -30 -20 60 40          # new solid: 6 up from xy; rect X Y W H
wall: extrude 6 xz at 0 20 0 rect -30 0 60 40
hole 5 -xy at -20 -10 6 thru                # dia, frame (its z drills in), depth|thru
pattern hole1 lin 1 0 0 3 20                # copy hole1: dir, count, step
extrude cut 2 -xy at 0 5 6 circle 0 0 4     # op: new add cut keep (default add)
fillet 3 (edges between base wall concave)  # radius, then refs; a new radius applies onward
chamfer 1 @d644 @b2e4                       # @handles come from `query`
```

Parameters and sketches (each makes nothing; lines below use them):
```
w = 40                      # NAME = EXPR [mm|deg], defined above its use; any number below
d = w*3 + 1 mm              #   may be an expression: w/2, (h + 2), sqrt(a), min(a,b)
plate: sketch xy            # sketch FRAME | sketch on REF (a planar face); rows to `end`
  rect 0 0 38 27            # l1..l4 (corners shared, kept square)    EXPR: + - * / ^ ( )
  circle 20 12 4            # c1                     sqrt sin cos tan (degrees) min max abs pi
  pt 6 6                    # p1
  fix l1.a                  # constraints and dimensions, valued by EXPR:
  dist l1 w                 #   fix E.. | co A B | hor|ver E.. | par|perp|tan|conc A B
  dia c1 8                  #   eq A B.. | mid P L | sym P Q L | dist A B V | dist L V
  hdist l1.a c1 w/2         #   hdist|vdist P Q V | rad|dia C V | ang L M V
end
extrude 6 plate             # its one region; `plate E`: the region E bounds outside
hole 3 on plate p1 thru     # FRAME on a sketch's plane (-on: flipped), at its point
```
Rows: `pt P`, `line P P`, `poly P P.. [open]`, `rect X Y W H`, `circle P R`, `arc C S E [cw]`
(append `cons` for construction). P: `X Y`, or a point: `p1`, `l2.a`|`.b`, `c1` (centre), `o`
(origin). Entities count by kind: p l c a. Under-constrained solves from the coordinates
written; `check` reports each sketch's freedom; conflicts name their rows.

Features (`[op]` is new for the first solid, add after):
```
extrude [op] EXT (FRAME LOOPS | SKETCH [E]) EXT: N | sym N | two N M | thru [back|both] | next [back] | to FEAT [@face] [off N]
revolve [op] [TURN] (FRAME [axis x|y|OX OY DX DY] LOOPS | SKETCH [axis L] [E]) TURN: like EXT, in degrees; default 360
ball [op] R FRAME     fillet R REF.. [R REF..]     draft A FRAME REF..
shell T [out] REF..              hollow to walls T thick (out: outside the skin), REF faces open
chamfer D REF.. | chamfer D D2 face REF REF.. | chamfer D ang A face REF REF..
hole D FRAME [cb D2 DEPTH | cs D2 A] (N | thru | next | to FEAT [@face] [off N]) [pt A] [thread SIZE PITCH]
  N: depth, flat bottom unless `pt 118` (drill point angle)
pattern (body | FEAT..) (lin DX DY DZ N STEP [DX DY DZ N STEP] | circ OX OY OZ AX AY AZ N [A] [step] | mirror FRAME)
  N counts the original; lin: STEP mm apart along DIR; circ: N spread over A (default 360),
  or A apart with `step`; mirror: in the frame's xy plane (`mirror yz` is across x=0)
sweep [op] FRAME LOOPS path PATH [extend N M | to FEAT [@face] [off N]]     profile square to the path's start
coil [op] FRAME [axis ..] LOOPS (pitch P turns N | turns N height H | pitch P height H | pitch P to FEAT [@face] [off N]) [taper A] [left]
loft [op] [smooth [start W] [end W]] (profile FRAME LOOPS).. [rail I PATH.. | centre PATH]     I: vertex (where segment I starts)
PATH: (line AX AY AZ BX BY BZ | arc FRAME R START SWEEP)..     tangent-continuous; an arc's FRAME is its circle's, centre at `at`
plane FRAME                      a work plane, for `to FEAT` and `on FEAT`
rib T [pos|neg] [web] [flip] [draft A] FRAME (line AX AY BX BY | arc CX CY R START SWEEP)..
  a wall along an open path, ends run on until they meet the part. Default: fills across the path
  to its left (flip: right), T thick along the frame's z (sym; pos/neg: one side). web: fills along
  -z (flip: +z), T thick across the path (pos: to its left). draft A >= 0: thicker towards the part
offset D REF..                   faces moved D along their outward normals, neighbours re-met
thicken [op] T [neg|sym] REF..   a slab T thick on the faces, outward (neg: inward); op new = slab alone
delete REF..                     faces removed, the gap closed by the faces around it: a hole, boss or
                                 pocket (its rim a whole loop of a face), or a chamfer/fillet strip
replace REF with FEAT [@face]    a face put on a work plane or an earlier feature's face
move REF.. (by DX DY DZ | turn A OX OY OZ AX AY AZ)   faces moved or turned, neighbours re-met
FRAME: [xy|xz|yz|-xy|-xz|-yz | z A B C [x A B C] | [-]on FEAT [P] | on REF] [at X Y Z]     default: xy at 0 0 0
  xy: normal +z; xz: sketch x,y = world x,z, normal -y; yz: world y,z, normal +x;
  -xy: sketch y = world -y, normal -z (drill down); -xz: normal +y; -yz: normal -x
LOOPS (first is outside, rest holes; X Y a corner): rect X Y W H | rrect X Y W H CORNER_R | circle CX CY R |
  poly X Y X Y .. | loop (line AX AY BX BY | arc CX CY R START SWEEP)..
`next`: up to the next face met. `to`: a plane or sketch feature, or a feature's face (plane, cylinder..).
Fillet R: 3 | 2..4 (from the edge's first vertex, by name, to the other) | 0:2,0.5:4,1:2 (fraction along from that vertex:radius). A constant radius holds along its own edge; a tangent chain changes radius over its variable and unpicked edges.
FEAT: a label, kind+count (`hole2` the 2nd hole, `pattern1`..), or `fN` (the Nth feature line)
REF: @handle | (selector), found on the part as it stands above that line
```

Selectors (`query`, or inline in parentheses): `faces|edges|verts`, then filters in order.
`query` asks the finished part, or the part after a line with `--at FEAT`; `build` says how
many entities each inline selector picked, so check those counts.
```
plane cyl cone sphere torus blend | line circle curve      kind (fillets: cyl, or torus round a circle)
convex concave smooth                                      how an edge's faces meet
normal +z | normal A B C | along +x                        plane normal; line direction
r=3 r<3 r>3   longer 5   shorter 5                         radius; edge length
of FEAT..   between FEAT FEAT   on @h                      made by (copies too); joining; touching
top bottom left right front back   near X Y Z              keep the extreme ones (ties too); nearest
```

Commands (a few lines of text; `--json` for JSON):
```
forgecad build F                    ok, or error at file line N + hint; approx volume, area; bbox
forgecad query F SEL [--at FEAT]    @d644 line -x len=60 at 0,20,40   (round: axis and centre;
                                    a round face may be two halves, two handles on one line)
forgecad measure F REF [REF]        area|length radius centre; two: closest distance, angle, axes
forgecad measure F mass|wall        volume area centroid inertia; thinnest wall anywhere
forgecad render F [VIEW] [--labels SEL] [--px 400] [-o F.png]   VIEW: iso front top right.. px: long side
forgecad check F | diff A B         validity after each line | what B changed from A
forgecad serve|mcp [--root DIR]     stdio JSON-RPC / MCP, .fc files under DIR (default .) only
```
`serve` takes one JSON object per line, `{"id":1,"method":"build","params":{"file":"b.fc"}}`;
params: file sel at what to view labels px other. Rendered labels are handles without `@`.

Handles are stable name prefixes: they survive edits below their feature; selectors survive more.
