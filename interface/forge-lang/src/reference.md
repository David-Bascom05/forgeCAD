# Forge (.fg) reference

A part is a program. Its statements make named features in order; each feature is built as it is
made, so later anchors and selections read the part as it stands. Units: mm, angles in degrees
(`30deg`). `fg run F.fg` checks, builds and reports every feature; `fg check F.fg` stops before the
kernel; `fg edit F.fg 'OP'` applies a typed edit.

```
shared L = 60                       # params: the only place numbers belong
shared t = 6                        # shared: read by several features
param top.r = 8                     # a param of feature `top`
param round.r = 2

sketch plan(on: xy) {               # the first feature may sit on a world plane
  outline = rect(corner: (0, 0), size: (L, L / 2))
}
base = extrude(plan, t)
sketch top(on: base.end) {          # every later sketch sits on an anchor
  c = circle((L / 2, L / 2 / 2), r) # r is top.r
}
boss = extrude(top.c, t)
round = fillet(edges(of: boss, crease: "convex"), r)
expect volume = 11966 +- 0.1%
```

## Lexical

`#` starts a comment. Numbers: `12`, `0.5`, `1e3`, with a unit `mm` or `deg`. A minus sign
directly before a digit, where no value precedes it, is part of the number (`-1`, `(-1, 2)`);
`a -1` and `a - 1` subtract. Strings: `"M6"`. Names: letters, digits, `_`. Keywords: `shared param
fn for in if else return expect and or not true false sketch path`. A line break ends a statement,
except inside `( )` and `[ ]`; `;` also ends one.

## Statements

```
shared NAME = EXPR                  a param every feature may read
param FEATURE.NAME = EXPR           a param of one feature (and what it makes)
param FEATURE[KEY].NAME = EXPR      that param for one loop instance only (SplitInstance writes it)
NAME = EXPR                         a binding; when EXPR makes a feature, NAME is its id
sketch NAME(on: PLANE, at: POINT) { ROWS }
path NAME(start: P) { SEGMENTS }    inside a sketch: a chain of segments
for VAR in RANGE|MAP|LIST { .. }    for K, VAR in ..: K is the key (index or label)
if COND { .. } else { .. }          `} else` on the same line
fn NAME(A, B: Type, C = DEFAULT) { .. return EXPR }
return EXPR
expect METRIC [= < <= > >=] VALUE [+- TOL[%]]
```

Expressions: `+ - * / ^`, comparisons `== != < <= > >=`, `and or not`, `a if c else b`, ranges
`0..n` (end excluded) and `0..=n`, lists `[a, b]`, comprehensions `[e for x in xs if c]`, tuples
`(x, y)` (a 2D point or vector), keyed maps `{left: e, right: e}`, calls `f(a, key: b)`, members
`a.b`, indexing `a[i]`. Methods and members bind tightest: `-a.b` is `-(a.b)`.

## Names, ids and single assignment

- A name is bound once. Rebinding it, or binding a name a visible param or an enclosing scope
  already has, is the error `Rebind`. A name may not hide a built-in (`ShadowsBuiltin`).
- A feature's id comes only from syntax: the name it is assigned to, the call path, and loop keys.
  `bolt = hole(..)` at top level is `bolt`; inside `for i in 0..4` it is `bolt[0]`..`bolt[3]`;
  inside `for c in {lf: .., rf: ..}` it is `bolt[lf]`, `bolt[rf]`; inside `lug = boss(..)` a
  statement `hub = ..` of the function body is `lug.hub`. Ids never come from evaluation order,
  list positions, values or strings.
- A loop over a plain list may compute values but may not make features or sketch entities
  (`PositionalId`): its positions shift when the list is edited. Loop over a range or a keyed map.
- After a loop, every name its body bound is a map from key to value: `bolt[2]`, `bolt.lf`.
- A feature made anywhere but the right side of an assignment has no name (`UnnamedFeature`).
- References are absolute: `h[5]` with four instances, or `h[-1]`, is `IndexOut`. There is no
  "last".

## Params, dimensions and provenance

Dimensions: `Length` (a bare number in a param, or `mm`), `Angle` (`deg`), `Count` (`count(4)`),
`Ratio` (`ratio(0.5)`). Lengths add to lengths; `Length * Count` and `Length * Ratio` are lengths;
`Length / Length` is a Ratio; `Length * Length` is an Area (`sqrt` of an Area is a Length); trig
takes Angles and gives Ratios. Mixing them otherwise is the error `DimMismatch` or `WrongDim`; a
size is never a count, and a count over a size is nothing. A bare number takes the other side's
dimension, in a param too: `W / 2` and `W * 0.25` are Lengths, `1 / n` a Ratio; a param whose
value is still bare is a Length (`shared t = 6`). A bare number times a Ratio is a Ratio:
`shared k = 38 * sin(60deg)` is a Ratio; write `shared pcd = 38` and use `pcd * sin(60deg)`.

A param's right side may be any expression of earlier params: shared ones, the same feature's own,
and another feature's by its full name (`param bore.r = hub.r - 1`): an explicit link, shown as a
relation. List and tuple params hold data (`shared sae6 = [(1, 7.6), (2.6, 6.3)]`).

Inside feature F's statement (its sketch rows, its call's body), a bare name finds F's own param
first (an instance's own, `F[2].x`, before `F.x`), then names bound in scope, then shared params.

Every number records where it came from:
- **driving**: exactly one param read by name. `SetParam` on that param is its editable handle.
- **driven**: an expression of params. It has no handle; edit its inputs.
- **const**: a structural constant.
A param whose value was **observed** (a loop bound, an index, a comparison, a selection's filter
value, a solver's input) is structural: editing it re-runs evaluation; editing any other param
only re-reads the stored formulas. `fg run --json` lists, for every param, its value, links,
whether it is structural, and every feature its values drive.

## Placement

Planes, points and axes come from anchors. The world planes `xy`, `xz`, `yz` (and world directions
`X Y Z` in `plane(..)`) are for the first feature; after it they are the lint `WorldAfterFirst`.

```
sketch S(on: PLANE, at: POINT)      at: the plane's origin moves to POINT dropped onto it
offset(PLANE, d)                    the plane moved d along its normal
rotate(PLANE, about: AXIS, by: A)   the plane's normal turned about an axis lying in it
                                    (origin on the axis, x along it)
plane(at: POINT, normal: DIR, x: DIR)
axis(P, Q)                          the axis through two points
mid(LINE)                           a sketch line's midpoint, as a 3D point
```

PLANE: a sketch (its plane), `S.plane`, a planar face set (`E.end`, `E.side(l)`), or a plane value.
POINT: a sketch point or circle centre (`S.c`, `S.l.a`), a point value, an axis. AXIS: a sketch
line, `axis(P, Q)`, `E.axis`, `F.axis` of a cylindrical face. DIR: `F.normal`, `S.l.dir`, an axis.

## Sketches

Coordinates are the sketch plane's, from its origin. Points written as coordinates are fixed;
`near(x, y)` is a free point (a starting guess). The solver (forge-sketch) holds every constraint;
tangency is never written as computed numbers.

```
p = pt(x, y)                        fixed point (listed in S.points)
q = near(x, y)                      free point
l = line(a, b, construction: false) a, b: points or (x, y)
a = arc(center: c, from: p, to: q, cw: false)
c = circle(center, r)
r = rect(corner: (x, y) | center: (x, y), size: (w, h))   sides 0..3: bottom, right, top, left
s = slot(a, b, r)
h = hull(ITEM, ITEM, ..)            convex outline, counter-clockwise: an item is a point or
                                    disc(center, r); lines tangent to the discs, arcs round them.
                                    Segments in order: for each item, its arc (discs only), then
                                    the line to the next item
o = poly([P, ..], closed: true)     side k runs from point k to point k+1
s = spline([P, ..], closed: true)   the cubic through the points
path o(start: P) {                  a chain: line(to: P, tangent: false), tarc(to: P)
  line(to: P)                       (an arc tangent to the segment before), spline(through:
  tarc(to: P)                       [P, ..], to: P, tangent: true), close()
  close()
}
```

A loop value (rect, slot, hull, poly, path) indexes its segments: `o[2]`, `o.segs`. Entity
members: points `x y`; lines `a b dir`; arcs `center a b`; circles `center`.

Constraints (bare statements): `tangent(a, b)` (lines, arcs, circles, a spline's end),
`coincident(p, q|line|circle)`, `horizontal(l)`, `vertical(l)`, `parallel(l, m)`,
`perpendicular(l, m)`, `equal(a, b)`, `midpoint(p, l)`, `distance(p, q|l, d)`, `distance(l, d)`,
`radius(c, r)`, `angle(l, m, A)`, `fix(p)`. A conflict is `SketchConflict` naming the rows;
freedom left is the warning `UnderConstrained` with the count.

Lines for constructing points: `shift(LINE, d)` (moved d to the line's left, into a
counter-clockwise loop), `hline(y)`, `vline(x)` (in this sketch), `meet(LINE, LINE)` (where they
cross, as `(x, y)` in this sketch). LINE: a sketch line of any sketch, or one of these.

Helpers: `polar(n: count, r, start: A)` gives n points round the origin. Math: `sqrt sin cos tan
asin acos atan atan2(y, x) abs min max len pi`.

## Features

```
extrude(PROFILE, d | sym(d) | two(f, b) | thru | thru_all | next | upto(FACES, offset),
        taper: A, new: false)       adds (a new body for the first solid, or with new: true)
cut(PROFILE, d | ...)               into the material: against the sketch normal
intersect(PROFILE, extent)          keeps what lies inside
revolve(PROFILE, axis: LINE, angle: A, cut: false, new: false)   LINE: a line of the same sketch
hole(at: POINT | [POINTS], d: D, depth: d | thru | next, cbore: (D, depth), csink: (D, A),
     flip: false)                   drilled against the sketch normal (flip: with it)
hole(at: .., screw: "M6", fit: "close"|"normal"|"loose"|"tap", seat: "cbore"|"csink", depth: ..)
fillet(EDGES, r) | fillet(groups: [(EDGES, r), ..])
chamfer(EDGES, d)
shell(t, open: FACES, out: false) hollow to walls t thick (out: outside the skin), FACES open
```

PROFILE: a sketch with one region, or one of its loops or closed entities (`S.outline`, `S.c`).
A negative depth runs the other way. `taper:` takes a depth only: with `sym`, `two`, `thru`,
`thru_all`, `next` or `upto` it is `NotWith`; on an outline with a spline side it is
`TaperOnCurve` (the kernel's draft has no exact leaning form for a free curve).

### Swept, wound and lofted shapes

```
sweep(PROFILE, along: PATH, extend: (s, e) | upto: FACES, offset: d, cut|keep|new: true)
coil(PROFILE, axis: LINE, pitch: p, turns: n, height: h, upto: FACES, offset: d, taper: A,
     left: false, cut|keep|new: true)
loft([PROFILE | POINT | dome(POINT, DIR, w), ..], smooth: false, start: w, end: w,
     align: false, seams: [POINT, ..], rails: [(CORNER, PATH), ..] | centre: PATH,
     cut|keep|new: true)
```

- PATH: a sketch line, arc or spline, a loop or `path` block, or a list of them in order (from
  any sketches: a path in 3D). Pieces are turned to run end to end; one that does not start
  where the one before ends is `PathGap`. A sweep's path starts on the profile's plane, square
  to it, and is tangent-continuous (the kernel's `Kink`, `NotSquare` otherwise); the profile is
  carried square to it without twist (the kernel has no twist or frame option). `extend` runs it
  straight on before the start and after the end; `upto` cuts it off at a face's surface,
  `offset` on past it (one or the other: `NotWith`).
- coil: winds PROFILE about LINE (a line of the same sketch); any two of `pitch`, `turns`,
  `height` (all three, or `upto` with more than a pitch, is `NotWith`; fewer, `MissingArg`), or
  `pitch` with `upto` (wound on to a face). `taper` widens it as it rises; `left` winds it
  left-handed.
- loft: through the profiles in order, ruled, or `smooth` (a curvature-continuous blend, `start`
  and `end` the weights its walls leave the end profiles with, square to them). Profiles with
  different segment counts are matched by the tool: a whole circle is cut at the angles of the
  other profile's corners (so a square to a circle runs corner to arc end), and otherwise the
  longest lines and arcs are halved until the counts agree; a profile of splines alone that
  would need splitting is `LoftCurves`; a different number of loops (outline and holes) is
  `LoftLoops`. A point first or last closes the loft on it; `dome(P, DIR, w)` rounds it, the
  walls arriving square to DIR. `align` starts each section nearest the one before's start;
  `seams` puts a profile's start at a point of its sketch. `rails`: (a corner of the first
  profile, a path of lines and arcs through the same corner of every profile); `centre`: a path
  crossing each profile's plane (one or the other: `NotWith`).
- Ops: added (a new body for the first solid), or `cut`, `keep` (intersect), `new` (a body of its
  own); two of them is `NotWith`.

```
sketch sec(on: xy) {
  c = circle((0, 0), r)
  base = line((0, 0), (w, 0), construction: true)
}
sketch rail(on: rotate(sec.plane, about: sec.base, by: 90deg)) {
  path route(start: (0, 0)) {
    line(to: (0, h))
    tarc(to: (R, h + R))
  }
}
tube = sweep(sec.c, along: rail.route, extend: (0, w))
sketch spr(on: rotate(sec.plane, about: sec.base, by: 90deg)) {
  c = circle((2 * R, 0), r / 2)
  ax = line((0, 0), (0, h), construction: true)
}
spring = coil(spr.c, axis: spr.ax, pitch: p, turns: n)
sketch top(on: offset(sec.plane, h)) {
  sq = rect(center: (0, 0), size: (w, w))
}
neck = loft([sec.c, top.sq])               # a circle to a square: matched corner to arc end
```

### Faces moved, slabs, ribs, rounds, balls

```
draft(FACES, angle: A, neutral: PLANE)    faces tilted A about where they cross the neutral
                                          plane, the pull along its normal
offset_faces(FACES, d)                    faces moved d along their outward normals
move_faces(FACES, along: DIR, d: D | about: AXIS, by: A)   faces moved or turned, neighbours re-met
delete_faces(FACES)                       a pocket, boss or blend taken away, the gap closed
replace_face(FACE, with: FACES)           a face put on another face's surface
thicken(FACES, t, side: "out"|"in"|"both", cut|keep|new: true)   a slab on the faces
rib(PATH, t, side: "both"|"pos"|"neg", web: false, flip: false, draft: A)
facefillet(FACES, FACES, r)               round between two face sets
fullround(side: FACES, middle: FACES, other: FACES)   each one face (`OneFace`)
ball(POINT, r, cut|keep|new: true)        a ball (sphere: the same)
```

- `offset(PLANE, d)` is a plane; `offset_faces` moves faces of the part.
- rib: a wall t thick along PATH (lines and arcs of one sketch, end to end), its ends run on to
  the part, filling from the path to the part on its left (`flip`: right) in the sketch plane, t
  along the sketch normal (`side` one way or both); a `web` reaches along the normal instead,
  t across the path; `draft` thickens it toward the part.
- ball: the centre a sketch point (the ball's frame is the sketch's), a point, or an axis.

```
cup = shell(t, open: box.end)
lip = fillet(edges(between: (cup.inner, cup.rim)), t / 4)
tilt = draft(box.side(base.r[2]), angle: a, neutral: base.plane)
grow = offset_faces(box.side(base.r[1]), g)
turn = move_faces(box.side(base.r[1]), about: base.r[1], by: a)
fill = delete_faces(pit.faces)
wall = rib(web.l, t, flip: true)
round = facefillet(box.side(base.r[0]), box.end, r)
knob = ball(top.c, r)
```

## Anchors

Every feature exports anchors by role; they resolve to the kernel's role names
(`forge_ops::roles`), never to hashes read off a part.

```
E.start  E.end          extrude caps (cut: E.end is the floor), with planes whose x is the
                        sketch's x and origin the sketch origin dropped onto them
E.side(ENTITY)          the wall a profile entity made (a line: plane with origin at its start,
                        x along it); revolves too: R.side(ENTITY)
E.axis                  a single-circle extrude's axis; a revolve's, coil's or hole's axis
E.faces  E.edges        everything the feature made, as the part stands
H.rim                   a hole's (or bore's) mouth: the loop nearest it where its faces meet
                        the part's (any curve: on a blend too); rounded by a later fillet, it is
                        `MouthRounded`
W.start  W.end          a sweep's, coil's or loft's end caps (planes carried from the profile's
                        sketch); W.side(ENTITY) the walls a profile entity made over the whole
                        path (a loft: an entity of its first profile)
S.inner  S.rim          a shell's inner (outward: outer) skin, and the rims round its open faces
S.plane  S.origin  S.points  S.NAME   a sketch's plane, origin, points, named entities
FACES.plane  FACES.normal  FACES.edges  FACES.axis (cylindrical)
```

## Selections

```
edges(of: F | [F..] | FACES, between: (A, B), crease: "concave"|"convex"|"smooth",
      shape: "line"|"circle"|"curve", along: DIR, r: R, longer: L, shorter: L)
faces(of: F | [F..], shape: "plane"|"cyl"|"cone"|"sphere"|"torus"|"blend", normal: DIR, r: R)
EDGES + EDGES
FACES + FACES                       one face set (a slab, an offset over several features' faces)
```

An empty face set on a part built without an earlier failure is an empty selection
(`EmptySelection` at a fillet or chamfer), never silently nothing. A cap (`E.start`, `E.end`) a
later edit moved is still found by its name, its plane where it is now.
`of` means made by (lineage through later booleans); `between: (A, B)` the edges where A's faces
meet B's, A and B features or face sets. Directions may be world axes `X Y Z` here (orientation,
not placement). Selecting by where things lie (`extreme:`) is the error `PositionalSelection`:
an unrelated edit can retarget it. A selection that picks nothing at a fillet or chamfer is
`EmptySelection`.

## Sheet metal

A sheet part is a graph of walls joined by bends (forge-sheet), built whole after each sheet
feature, folded and, on request, flat. The sketch plane of the base is the sheet's reference
skin (`top`); the material lies a thickness behind it, against the sketch normal.

```
shared t = 2
shared r = 2
shared k = ratio(0.4)
param plan.L = 40
param plan.W = 30
param up.h = 30
param hs.x = 15
param hs.y = 15
param hs.d = 6
param blank.gap = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
up = flange(base.edge(plan.outline[1]), height: h, measure: "inner")
sketch hs(on: up.top) {
  c = circle((x, y), d)                    # in the flange's own coordinates
}
vent = cutout(hs.c, wall: up)
blank = unfold(base, on: offset(base.bottom, gap))
```

```
sheet(PROFILE, t: T, r: R, bend: RULE, shop: SHOP)
    the base wall: PROFILE's outline and holes. r and bend may come from the shop's stock of
    thickness t instead. Its own dims: thickness, radius, k.
flange(EDGE, height: H, angle: A = 90deg, side: "up"|"down" = "up",
       measure: "inner"|"outer" = "outer", r: R, relief: ..)
    a wall turned A (0 < A < 180deg) off a straight wall edge, as long as the edge. up turns
    towards the wall's top. H runs along the flange from the virtual sharp of the inner or the
    outer skins. r: this bend's own inside radius (default the sheet's). relief: refused,
    ReliefUndecided (reliefs wait on a decision; cut one with cutout past the tangent line).
cutout(PROFILE, wall: W)
    PROFILE taken out of the sheet. Its sketch lies on W's top or bottom; the profile is placed
    in W's developed coordinates, so it cuts across bends into the walls beyond as they unfold.
    A whole sketch cuts all its regions; lines, arcs, circles and splines.
unfold(S, on: PLANE)
    the flat body of S's sheet as a new body, its first wall on PLANE (default where it is
    folded). Sheet features after an unfold of their sheet are AfterUnfold.
```

EDGE: `B.edge(ENTITY)` for the base B (a straight line of its profile), `F.tip`, `F.start_side`,
`F.end_side` for a flange F (the far edge; the sides at the start and the end of the edge it
turned off).

Anchors, the same on the folded body and on an unfold's (`U.of(F)`):

```
W.top  W.bottom          a wall's skins (base or flange): planes in the wall's own coordinates
                         (x, y as its sketch or, for a flange, x from the mold line to the tip,
                         y along it), z out of the material
F.bend  F.bend_top  F.bend_bottom   a flange's bend zone skins
EDGE.face  EDGE.edges  EDGE.top  EDGE.bottom   an edge's side face (a plane: origin at the edge's
                         start on the top skin, x along it, z out of the wall) and its edges
X.faces  X.edges         what a sheet feature made; U.faces, U.edges: the whole flat
U.of(X)                  feature X of the unfolded sheet, on the flat body
```

The shop is data, held by params:

```
kfactor(K)                        K a Ratio in [0, 1]: BA = angle (r + K t)
ktable([(R_OVER_T, K), ..])       K by r/t, linear between rows (Ratios, increasing)
bend_table([(R, A, BA), ..])      measured allowances for each radius, linear in angle
per_t(F, plus: L = 0)             a length F t + L (F a Ratio)
stock(t: T, bend: RULE, radii: [R, ..], min_flange: S, hole_to_bend: S, hole_to_edge: S,
      min_slot: S, relief_width: S, relief_depth: S)   S: per_t(..); left out: no limit
shop([STOCK, ..])
```

A sheet given `shop:` is checked against it at the end of the run: thickness stocked, each
bend's radius tooled, each wall reaching the die's minimum flange from its bend. Each is a typed
warning (`ThicknessNotStocked`, `RadiusNotTooled`, `FlangeTooShort`; severity `violation`) with a
SetParam fix. Holes and slots against bends and edges are not checked yet.

Rules: features that are not sheet metal are refused in a part with a sheet
(`OrdinaryOnSheet`, fix DeleteFeature), and a sheet in a part with a solid is `SheetOnSolid`.
Every kernel refusal is a typed error named as the kernel names it (`WallTooShort`,
`BendAngle`, `BendRule`, `EdgeTaken`, `NeedsRelief`, `FoldedClash`, `Pinched`, `NearCoincident`,
`TrimTouchesBendLine`, `TrimAlongBendLine`, `TangentMismatch`, ...), its walls, joints, cuts and
trims named by the Forge features that made them, with a SetParam fix where a param cures it
(a flange's height, its angle, K, the radius). Others: `NotAnEdge`, `CutOffWall`, `OtherSheet`,
`NoSuchRole`, `TableRow`, `ReliefUndecided`. K moves only the flat: the folded body is the same
to the bit for any K.

`fg run F.fg --dxf OUT.dxf` writes each sheet's flat pattern (R2000 DXF, mm): cut contours on
layer CUT (lines, arcs, circles, NURBS), bend lines down each zone's middle on layer BEND. `fg run
--json` lists each sheet: thickness, radius, rule, and each bend's flange, angle, radius,
allowance and setback.

## Functions and recursion

Functions are pure: they see their arguments, shared params, other functions and built-ins. A
call assigned to a name puts that name in the ids of the features its body makes. Recursion is
allowed; each top-level statement may make at most 100000 calls, nested at most 128 deep (`Fuel`).

## expect

`expect volume|area = V +- T[%]`, `expect faces|edges|bodies = N`, `expect valid`. A failed line is
`ExpectFailed` with the measured value and the limit. An expect whose limit is the measured value
to the last digit is the lint `ExpectCopied`.

## Rules

| rule | level | what |
|---|---|---|
| named-features | error | a feature is made only by an assignment |
| keyed-ids | error | loop instances keyed by range index or label |
| single-assignment | error | no rebinding, no shadowing |
| no-positional-selection | error | no `extreme:` selections |
| literals-in-params | lint | numbers other than 0, 1, 2, 90/180/360deg (and whole numbers in counts: range bounds, indices) belong in params; fixes: LiftLiteral (local or shared) |
| scoped-params | lint | a param of another feature read without a link; fixes: AddLocalParam, MakeShared |
| world-only-first | lint | world planes or directions after the first feature |
| expect-copied | lint | an expect copied from a run |

Dimension mixing, unknown names, wrong types, arity and index errors are type errors, always.

## Errors, lints and fixes

Every error and lint is a typed record: `kind`, `span` (byte offsets, line, column), its data (the
param, the measured value, the limit, ..), a `text` rendered from the data, and `fixes`: typed
edit ops that would remove it. Warnings that are not lints: `UnderConstrained`, `UnusedParam`,
`NoEffect` (a feature that changed nothing).

## Edit ops

Each is a pure function of (document, op): it replaces one span or inserts at one point; every
other byte stays. `fg edit F.fg '{"op":"SetParam","param":"base.t","value":{"value":8,"dim":"Length"}}'`

```
SetParam {param, value}             the param's right side becomes the literal (all it drives)
AddLocalParam {feature, name, value}   param FEATURE.name = value, above the feature
MakeShared {param}                  param F.x = v  becomes  shared x = v
SplitInstance {id, param}           param ID.x = <param's right side>: that instance's own
AddFeature {before, text}           statements put in above `before` (or at the end)
DeleteFeature {id}                  the statement making id taken out
MoveFeature {id, before}            that statement moved above `before` (or to the end)
LiftLiteral {span, name, scope}     a loose number replaced by a new param (shared or local)
```

An op fails only with a typed error: `UnknownParam`, `UnknownFeature`, `NotAStatement` (an
instance made in a loop or call: edit its params, or SplitInstance), `NameTaken`, `EditDim` (the
value's dimension is not the param's), or the edited document's own errors.

## CLI

```
fg run FILE [--json] [--render OUT.png] [--view iso|front|top|right|back|left|bottom] [--px N]
           [--dxf OUT.dxf]
fg check FILE [--json]
fg edit FILE 'OP' [--json]
fg handles FILE
fg serve                    JSON lines {"cmd":"run","file":F}: stages kept, so a rerun builds
                            only from the first feature that changed
fg reference
```
