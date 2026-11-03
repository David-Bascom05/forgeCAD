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
fn for in if else return expect and or not true false sketch path at` (`at:` is still a keyword
argument). A line break ends a statement,
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
expect METRIC [= < <= > >=] VALUE [+- TOL[%]] [at PARAM = VALUE, ..]
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

`project(EDGES | FACES | POINT | AXIS, approx: TOL)` lays 3D geometry of the part as it stands on
the sketch's plane, fixed there (read afresh every run): lines, circles, arcs, ellipses exactly; a
general curve only with `approx` (a fitted cubic within TOL, else `NeedsApprox`); an axis only
seen end-on (else `NoImage`). One curve gives an entity, several a loop; a face gives its
boundary. Bind the edges outside the sketch (`pe = edges(of: peg.end)`, then `c = project(pe)`).

Lines for constructing points: `shift(LINE, d)` (moved d to the line's left, into a
counter-clockwise loop), `hline(y)`, `vline(x)` (in this sketch), `meet(LINE, LINE)` (where they
cross, as `(x, y)` in this sketch). LINE: a sketch line of any sketch, or one of these.

Helpers: `polar(n: count, r, start: A)` gives n points round the origin. Math: `sqrt sin cos tan
asin acos atan atan2(y, x) abs min max len pi`. Index arithmetic on counts: `div(k, n)` (the
whole quotient) and `mod(k, n)` (the remainder), so `xs[mod(k, 3)], ys[div(k, 3)]` walks a grid.
`k / n` of counts is a count only where it comes out whole (else a Ratio): use `div`.

## Features

```
extrude(PROFILE, d | sym(d) | two(f, b) | thru | thru_all | next | upto(FACES | WORKPLANE, offset),
        taper: A, new: false)       adds (a new body for the first solid, or with new: true)
cut(PROFILE, d | ...)               into the material: against the sketch normal
intersect(PROFILE, extent)          keeps what lies inside
revolve(PROFILE, axis: LINE, angle: A, cut: false, new: false)   LINE: a line of the same sketch
hole(..), thread(..)                see Holes and threads
fillet(EDGES, R, from: POINT, setbacks: [(POINT, d) | (POINT, EDGES, d)], only: false)
fillet(groups: [(EDGES, R) | (EDGES, R, POINT), ..], setbacks: .., only: false)
                                    R: r | vary(a, b) (linear along each edge) |
                                    vary([(t, r), ..]) (radii at fractions t of the way, 0 to 1,
                                    increasing; a smooth law through them). from: the vertex at
                                    POINT each edge's law starts from; setbacks: how far along
                                    the edges from the corner at POINT (one edge, or all) the
                                    blends stop; only: the edges named, not their tangent chains
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
replace_face(FACE, with: FACES | WORKPLANE)   a face put on another face's surface, or a work plane
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

## Holes and threads

```
hole(at: POINT | [POINTS], d: D, depth: DEPTH, cbore: (D, depth), csink: (D, A), point: A,
     thread: "M36x2" | (size, pitch), flip: false, on: PLANE, bodies: BODIES)
hole(at: .., screw: SIZE, convention: "iso"|"ansi", fit: "close"|"normal"|"loose"|"tap",
     seat: "cbore"|"csink", depth: DEPTH, point: A, ..)
DEPTH: d | thru | next | upto(FACES | WORKPLANE, offset)
```

Drilled against the plane's normal (`flip: true`: with it), the mouth at each point, x along the
plane's x. The points are a sketch's (one sketch; the plane is its own unless `on:` gives one)
or points worked out here (work points, `mid(..)`), then `on:` is required; each is dropped onto
the plane. Several points make one hole feature, the rest copies of the first. `point:` is a
blind hole's drill point (the cone's included angle; none on `thru`). `thread:` records a thread
(modeled by `thread(hole: ..)`). SIZE: `M6`, `M8x1`, `1/4-20`, `#10-24` from the tables: its fit's
diameter, its seat's head; `fit: "tap"` the tap drill, its thread recorded. Options that cannot
apply are errors, never dropped: `cbore` with `csink` or `seat`, `d` or `thread` with `screw`
(`Conflict`); `fit`, `seat`, `convention` without `screw`, `point` on `thru` (`NotApplicable`);
an unknown size (`UnknownSize`) or word (`UnknownWord`).

```
thread(hole: HOLE, length: L)                   the hole's recorded thread, at every place and copy
thread(face: FACES, spec: SPEC, length: L, flip: false)   round a cylinder face (outside or in)
thread(on: PLANE, at: POINT, spec: SPEC, internal: false, length: L, flip: false)   about an axis
       square to the plane through POINT (default its origin), into the material
length: d | "full" | upto(FACES | WORKPLANE, offset);  left: false, chamfer: false, runout: false
SPEC: "M6" | "M6x0.75" | "1/4-20" | (major, pitch)
```

Exactly one of `hole`, `face`, `on` (`Forms`); `spec` with `hole`, `at`/`internal` without `on`
are `NotApplicable`. Modeled ISO/UTS 60 degree form; `chamfer` a 45 degree start, `runout` the
groove drawn out over a turn.

```
param tap.d = 5
param tap.depth = 12
tap = hole(at: top.s, screw: "M6", fit: "tap", depth: depth, point: 118deg)
tapped = thread(hole: tap, length: "full", chamfer: true)
```

## Patterns

A pattern is one feature: copies of earlier features (`of:`) or of the whole part
(`whole: true`), laid out by a layout value. The copies are named from the pattern's id and each
copy's index, so changing a count keeps every surviving copy's names. The copies are the pattern's,
not the copied feature's: `edges(of: peg)` is the original's alone, `of: [peg, pegs]` both.

```
pattern(of: F | [F..], layout: LAYOUT, bodies: BODIES) | pattern(whole: true, layout: LAYOUT)
lin(along: DIR, n: N, step: d, along2: DIR, n2: N, step2: d)     one or two rows (N counts the original)
circ(about: AXIS, n: N, over: A | each: A)      N over A (default a whole turn), or A apart
mirror(on: PLANE)
follow(curve: EDGES | WORKAXIS, n: N, step: d, fixed: false, reverse: false)
                                    N spaced d (no step: spread over the whole curve); turned
                                    with the curve unless fixed
places(at: [POINTS] | MAP)         the original at the first point, a copy at each other
```

DIR from anchors (`S.l.dir`, `E.side(..).normal`, a work axis); a world direction is the
`WorldAfterFirst` lint. `over` with `each` is a `Conflict`. A loop (`for i in 0..n`) makes
features one by one; a pattern records and edits them as one.

```
param bolts.n = count(4)
bolts = pattern(of: bolt, layout: circ(about: hub.axis, n: n))
```

## Work features

Named features that make no solid, followed by the kernel through every edit above them. Each
takes exactly one form (`Forms`), and only that form's arguments (`NotApplicable`).

```
workplane(on: PLANE) | workplane(offset: FACE, d: d) | workplane(turn: PLANE, about: AXIS, by: A)
  | workplane(tangent: CYL, parallel: PLANE) | workplane(through: (P, P, P))
  | workplane(mid: (PLANE, PLANE)) | workplane(normal_to: CURVE, at: P)
workaxis(along: EDGE | CYL | CONE) | workaxis(through: (P, P)) | workaxis(meet: (PLANE, PLANE))
  | workaxis(normal_to: PLANE, at: P)
workpoint(at: VERTEX | P) | workpoint(mid: EDGE) | workpoint(meet: (EDGE | AXIS, FACE | PLANE))
ucs(on: PLANE) | ucs(through: (ORIGIN, ON_X, IN_XY))
```

References (P, FACE, EDGE, AXIS, PLANE above) are what the kernel can follow: work features, a
sketch (its plane), a named sketch point (or a named line's end `l.a`, a circle's centre), one
face (or the pieces of one surface) or one edge. A value worked out here (`mid(..)`, `plane(..)`)
is `NotARef`; `workplane(on:)` takes any plane. A work plane is a PLANE (a sketch's, an `upto`
end, a split); a work axis an AXIS; a work point a POINT. Members: `.plane .normal .origin`,
`.axis .dir`, `.point`.

```
bax = workaxis(along: faces(of: boss, shape: "cyl"))
bp = workpoint(meet: (bax, l.side(lsk.outline[5])))
```

## Bodies

`new: true` on an extrude or revolve makes a body; a body is named by the feature that made it.
`bodies: F | [F..] | "all"` on a hole, thread, pattern, text, import or split makes it
act on those bodies, not the last made.

```
union(TARGET, tools: BODIES, keep: false)   subtract(..)   intersect(TARGET, tools: BODIES, ..)
combine(TARGET, tools: BODIES, op: "add"|"cut"|"keep", keep: false)
split(by: WORKPLANE | FACE | PLANE | LINE | ARC | CIRCLE, keep: "both"|"plus"|"minus")
splitface(faces: FACES, by: ..)       faces split where the cutter crosses them
copybody(of: BODY, moves: [MOTION..])    movebody(of: BODIES, moves: [MOTION..])
MOTION: slide(along: DIR, by: d) | spin(about: AXIS, by: A) | place(on: PLANE)
```

`keep: true` leaves the tools. A split keeping both makes the cutter's normal side a new body; a
sketch line, arc or circle cuts swept along its plane's normal. `intersect` with `tools:` joins
bodies; without, it is the extrude.

```
bolt2 = copybody(of: bolt, moves: [slide(along: jaw.outline[0].dir, by: pitch)])
seats = subtract(lower, tools: [bolt, bolt2], keep: true)
```

## Text and imports

```
text(STRING, font: FILE, size: d, depth: d, on: PLANE, at: POINT, cut: false, wrap: R)
import(FILE, op: "new"|"add"|"cut")
entity(of: IMPORT, solid: N, id: N)   the face (or edges) a STEP file's entity #id of solid #solid became
```

Files (.ttf/.otf fonts; .step/.stp/.brep/.igs/.iges solids) are found beside the part file;
a missing, wrong-kind or unreadable one is `File`. Text: the baseline from the point along the
plane's x, the letters up its y, `depth` along its normal (cut: into the part); `wrap: R` sets
it round a cylinder about the plane's normal. Members: `T.top` (embossed letter tops),
`T.floor` (engraved floors).

```
mark = text("FORGE-40W", font: "block.ttf", size: size, depth: depth, on: base.end, at: plate.mark_at, cut: true)
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
H.wall H.floor H.bore H.seat H.sink     a hole's faces by role (forge_model::hole::roles), at
                        every place; a role it has no face for is `NoFace`
T.flanks T.root T.chamfer   a thread's faces (forge_model::thread::roles)
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
`of` means made by that feature itself (lineage through later booleans); `between: (A, B)` the
edges where A's faces meet B's, A and B features or face sets. A pattern's copies are the
pattern's own, under its id, not the copied feature's: to take a feature and its copies, name
both (`edges(of: [pocket, pockets], ..)`, `between: ([blade, fins], base)`). Directions may be world axes `X Y Z` here (orientation,
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
sheet(PROFILE, t: T, r: R, bend: RULE, shop: SHOP, corner: CORNER, material: "NAME",
      finish: "raw"|"powder_coat")
    the base wall: PROFILE's outline and holes. r and bend may come from the shop's stock of
    thickness t instead, and so does the default relief. corner: what corners get (default
    circular_relief(), just clear of both bends). material: the stock's name for the DXF's
    part note. Its own dims: thickness, radius, k.
flange(EDGE, height: H, angle: A = 90deg, side: "up"|"down" = "up",
       measure: "inner"|"outer" = "outer", r: R, relief: RELIEF,
       position: "inside"|"outside"|"bend_outside" = "inside")
    a wall turned A (0 < A < 180deg) off a straight wall edge, as long as the edge. up turns
    towards the wall's top. H runs along the flange from the virtual sharp of the inner or the
    outer skins. r: this bend's own inside radius (default the sheet's). position: the flange's
    material outside the edge (its inner virtual sharp on it), inside it (the outer sharp on it:
    the edge moves in), or the bend starting at the edge (the edge moves out); the edges beside
    move with it (a flange on one follows; one in line gets a jog). The default is inside:
    Autodesk Inventor's documented default ("Inside of base face extents"); Fusion's dialog
    starts there by its tutorials; SolidWorks' and Onshape's help name no default.
    relief: at both ends of the bend; left out, the builder adds the default where the wall
    keeps material beside the bend, and nothing elsewhere.
corner_relief(F, G, CORNER)
    the treatment where flanges F and G, off one wall at a corner of its outline, meet
    (NotACorner if they do not).
cutout(PROFILE, wall: W, crosses_bend: false)
    PROFILE taken out of the sheet. Its sketch lies on W's top or bottom; the profile is placed
    in W's developed coordinates, so it cuts across bends into the walls beyond as they unfold.
    A whole sketch cuts all its regions; lines, arcs, circles and splines. With a shop, a cut
    running across a bend is the violation CutCrossesBend (fix: crosses_bend: true, which makes
    it information).
pattern(of: CUTOUT, layout: LAYOUT)
    the cutout repeated on its wall; a copy that leaves the wall's plane is PatternOffWall,
    other sheet features PatternOfSheet.
contour(PATH, width: W, t: T, r: R, bend: RULE, side: "left"|"right" = "right", shop: SHOP)
    a sheet whose walls are PATH's lines and whose bends are its arcs, run W along its sketch's
    normal. PATH is the reference skin, the material on its `side`; open, lines and tangent arcs,
    starting and ending with a line. An arc of radius a bends at inside radius a turning away
    from the material and a - t turning towards it; a sharp corner bends at r with the corner
    as its virtual sharp. Splines, circles, two arcs in a row, inside radii below the shop's
    least (Contour* errors, by profile piece).
contour(PATH, edge: EDGE)
    the same run off a straight edge, as long as it: PATH lies in a plane square to the edge and
    starts on it, its material continuing the wall's. C.tip is its last wall's far edge.
hem(EDGE, kind: "closed"|"open"|"teardrop"|"rolled" = "closed", length: L, gap: G, r: R,
    angle: A, side: "up"|"down" = "up", position: "inside"|"outside" = "inside")
    a fold of half a turn or more off a straight edge, as long as it. inside: the edge is the
    fold's outside (the wall keeps its size); outside: the fold starts at the edge. L is the leg
    from the fold's far tangent line (rolled: optional, none ends the roll free). open takes G
    between the layers; teardrop (A past 180deg) and rolled take R (default the sheet's) and A.
    A closed hem is modelled with the shop's hem_gap (never under 0.02, a solid has no touching
    skins) and needs its allowance in shop(closed_hems: [(t, BA), ..]); others use the bend rule.
    HemGap, HemAngle, HemRadius, HemLeg, HemNoRoom, HemNotTabled. C.tip is the leg's far edge.
seam(A, B, kind: "overlap"|"underlap"|"closed" = "overlap", gap: G)
    flanges A and B, off adjacent edges of one wall, cut to meet at their corner (normal cuts,
    edited in folded terms). overlap: A runs on to B's far skin, B stops G short of A's near
    skin; underlap the other way; closed: both stop G/2 back from the plane bisecting their
    mid-surfaces (on a square corner that leaves the t x t corner open: use overlap to close
    it). Each cut starts G above its flange's tangent line; the base corner takes the shop
    rule's corner relief. G defaults to the shop's kerf, never under 0.02. corner_relief(A, B,
    closed_corner | overlap_corner(gap: G)) is the same seam. SeamNotACorner, SeamNotAFlange,
    SeamTipTaken (seam before putting hems on the tips), SeamGap, SeamParallel, SeamNoRoom.
    constant_width_corner(width: W) is a corner relief: a W-wide slot (default t) along the
    corner's diagonal; relief_rule(corner: "constant_width") makes it the default.
punch(TOOL, at: POINT | (x, y), wall: W, angle: A = 0)
    a turret hit on wall W: TOOL is a tool id from the shop's library (NoSuchTool if absent) or
    a tool written out. The tool's shape is cut (exact lines and arcs), so the hole checks read
    it; the flat's DXF carries each hit (a POINT and the tool id as TEXT, turned to the hit) on
    layer PUNCH. Shop checks (warnings): ToolNotInShop, PunchAcrossBend (the hit past a tangent
    line), PunchSpacing (the web between hits under shop(punch_web: S), default t).
    A tool: ("ID", "round", D) | ("ID", "obround", L, W) | ("ID", "square", S) |
    ("ID", "rect", L, W) | ("ID", "d", D, FLAT) | ("ID", "keyhole", D, W, L) | ("ID", "hex", AF) |
    ("ID", "cluster", D, PX, PY, NX, NY); shop(tools: [TOOL, ..]) is the library.
unfold(S, on: PLANE)
    the flat body of S's sheet as a new body, its first wall on PLANE (default where it is
    folded). Sheet features after an unfold of their sheet are AfterUnfold.
```

RELIEF: `rect(W, D)`, `obround(W, D)` (W wide, D deep from the mold line, past the tangent line:
else ReliefTooShallow), `tear` (a slit the shop's kerf wide), `no_relief` (where one is needed,
NeedsRelief, with a SetArg fix for each kind). Sizes left out (`rect()`) are the default's. The
default: the shop stock's relief rule and the shop's `kerf`; with no shop, the rules of thumb
below. An automatic relief is deepened to the tangent line where the formula falls short. A
relief is a notch beside the bend's end, open past the edge; one ending on the tangent line is
built. Its depth is from the rule's reference (`from`).

CORNER: `circular_relief(R, placement: "intersection"|"tangent")`, `square_relief(H, placement:)`
(a circle of radius R or a square of half side H centred on the bend lines' crossing or the
tangent lines' crossing, cut through base, bends and flanges; size left out: just clear of both
bends' zones), `open_corner(GAP)` (a square at the corner that much larger, and the later wall
trimmed back t + GAP), `trim_to_bend` (the corner cut back a kerf past both tangent lines across
both zones), `tear_corner` (a kerf-wide slit along
the diagonal), `closed_corner`, `overlap_corner(GAP)` (refused: CornerUnbuilt). Cuts that part the
sheet are SheetSplit.

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
bend_table([(R, A, BA), ..])      measured allowances for each radius, linear in angle (rows by
                                  radius then angle, increasing: else TableOrder at the row)
per_t(F, plus: L = 0)             a length F t + L (F a Ratio)
stock(t: T, bend: RULE, radii: [R, ..], min_flange: S, hole_to_bend: S, hole_to_edge: S,
      min_slot: S, min_hole: S, relief: RELIEF_RULE)
      S: per_t(F, per_r: G, plus: L) = F t + G R + L; left out: no limit
relief_rule(shape: "rect"|"obround"|"tear", width: Z, depth: Z,
            from: "bend_line"|"up_to_bend"|"tangent_to_bend", min_remnant: Z, extend: false,
            corner: "round"|"square"|"tear"|"trim_to_bend"|"closed",
            corner_placement: "intersection"|"tangent", corner_size: Z,
            corner3: "none"|"intersection"|"full_round"|"round", corner3_radius: Z)
      Z: a Length (sized) or per_t(..) (scaled). Left out, the rules of thumb (not standards; no
      ISO or DIN standard fixes relief geometry): rect, width t, depth t + R + 0.5 from the bend
      line (SendCutSend's guide; Xometry and Protocase at least t + R and t), round corners at the
      bend lines' crossing just clear of both bends, no minimum remnant, no three-bend corner;
      kerf 0.2 (a stand-in). The bend line is taken as the mold line, so K moves only the flat.
      extend: true is refused (ReliefExtendUnbuilt). min_remnant: wall left beyond a relief under
      it is taken (the relief widened past the edge). A relief asked for smaller than the rule's
      is the violation ReliefUndersized, with SetArg fixes to the rule's size.
shop([STOCK, ..], kerf: K)
```

```
stock(.., min_hole: S, hardness: ratio(HRB))   the smallest round hole; the sheet's Rockwell B
hardware(kind: "nut"|"stud"|"standoff", size: "M3", sheet: (MIN, MAX), hole: D, edge: E,
         max_hardness: ratio(HRB), code: "..", source: "..")   one catalogue row; no source: a
                                  placeholder (warned as UnverifiedHardware)
shop([STOCK, ..], kerf: K, hardware: [HW, ..])  hardware left out: the placeholder catalogue
shop(.., max_flat: (W, L), max_flat_coated: (W, L), max_bend_length: L, max_box_flange: H,
     box_flange_when: "always"|"hardware", bend_angles: (A0, A1), min_part: (W, L))
                                  the fabricator's process limits, each left out unchecked: the
                                  flat blank's least bounding rectangle at any turn (shorter,
                                  longer) within max_flat, or max_flat_coated for a sheet with
                                  finish: "powder_coat" (FlatTooLarge) and at least min_part
                                  (PartTooSmall); each bend line (BendTooLong) and angle
                                  (BendAngleOutOfRange, fix: the angle into range); flanges round a
                                  closed box (four or more off one wall, one way), measured as
                                  min_flange is (BoxFlangeTooTall, fix: the height to the limit),
                                  only on a sheet with hardware when box_flange_when: "hardware"
pem(KIND, SIZE, at: POINT|CIRCLE, wall: W)      a catalogue part pressed into W at a point
                                  sketched on its top or bottom: its mounting hole is cut
```

A sheet given `shop:` is checked against it at the end of the run, on the flat in developed
terms: thickness stocked, each bend's radius tooled, each wall reaching the die's minimum flange;
each hole (a cut not meeting the edge, or a hole drawn in the sheet's own sketch) its distance to each bend's tangent line (HoleNearBend,
hole_to_bend with R the bend's radius) and to the edge (HoleNearEdge), round holes their size
(HoleTooSmall), other cuts the widest circle they hold (SlotTooNarrow). Hardware against its own
row, shop or not: the sheet's thickness in range (HardwareThickness), the hole's centre to an edge
or bend (HardwareNearEdge), its hole running into another hole or the centre inside one
(HardwareOverlapsHole), another hole's edge nearer the centre than the part's edge distance
(HardwareNearHole; other hardware's holes too), the sheet no harder than the part takes (SheetTooHard;
HardnessUnknown when the stock does not say). Each is a typed warning (severity `violation`)
with fixes: SetParam on the param that placed a hole or part, moving it the way out by what is
missing when that way is along a sketch axis, or shrinking its radius; a round hole's radius to
the minimum; the thickness into a part's range. The placeholder catalogue is not a maker's
table (forge_sheet::hardware); its parts carry UnverifiedHardware.

Rules: features that are not sheet metal are refused in a part with a sheet
(`OrdinaryOnSheet`, fix DeleteFeature), and a sheet in a part with a solid is `SheetOnSolid`.
Every kernel refusal is a typed error named as the kernel names it (`WallTooShort`,
`BendAngle`, `BendRule`, `EdgeTaken`, `NeedsRelief`, `FoldedClash`, `Pinched`, `NearCoincident`,
`TrimTouchesBendLine`, `TrimAlongBendLine`, `TangentMismatch`, ...), its walls, joints, cuts and
trims named by the Forge features that made them, with a SetParam fix where a param cures it
(a flange's height, its angle, K, the radius). Others: `NotAnEdge`, `CutOffWall`, `OtherSheet`,
`NoSuchRole`, `TableRow`, `NotACorner`. `fg run --json` lists, per sheet, the reliefs and corner
reliefs the builder cut (flange, end, kind, size, automatic): data, not warnings. K moves only the
flat: the folded body is the same
to the bit for any K.

Every sheet build checks both the folded part (strips passing through each other) and the flat
(strips overlapping once unfolded), whatever is asked for. A folded clash refuses the sheet
feature (`FoldedClash`) and is a warning on an unfold; a flat overlap refuses `unfold` and the DXF
(`FlatOverlap`: the pair overlapping most, a point, its area and the total, exact) and is a
warning on the part, with a SetParam fix for each flange involved: the tallest height that clears
it. Two strips whose flats come within the boolean's tolerance of touching are
`FlatOverlapUndecided`, never taken as clear. Strips that touch without overlapping make no valid
solid either: `FoldedTouch` (walls meeting edge to face, folded) refuses the sheet feature,
`FlatTouch` (a flat edge on another) refuses `unfold`; each is a warning on the other, with fixes
each tried on the kernel first (the corner opened, a slot along the touching edge of the base
before the refused step, heights). Where two walls off one wall would meet at a corner, the
builder trims the later back t and the shop's corner gap (on the shorter edge; on edges of one
length, the one starting at the corner), listed with the corner reliefs as an automatic open
trim. Hardware off its wall is `HardwareOffWall`; on a lip too narrow for its edge distance both
ways, `LipTooShort` (fix: the lip raised till it fits).

`fg run F.fg --dxf OUT.dxf` writes each sheet's flat pattern (R2000 DXF, mm): cut contours on
layer CUT (lines, arcs, circles, NURBS), bend lines down each zone's middle on layer BEND, and on
layer BEND_NOTES a note along each bend line (`UP 90.0 DEG IR 2.00`: UP turns towards the
drawing's viewer, DOWN away; the angle; the inside radius) and the part's note below the pattern
(`BASE  MATERIAL 1008 CR STEEL  T 1.50  K 0.420`). `fg run F.fg --step OUT.step` writes a part's
folded body as STEP AP214 in OUT (any flat pattern left out) and each unfold's flat beside it as
OUT-<unfold>.step; for an assembly, its structure (one product per part, one use per instance,
placed by the solve). `fg run --json` lists each sheet: thickness, radius, rule, and each bend's flange, angle, radius,
allowance and setback.

## Functions and recursion

Functions are pure: they see their arguments, shared params, other functions and built-ins. A
call assigned to a name puts that name in the ids of the features its body makes. Recursion is
allowed; each top-level statement may make at most 100000 calls, nested at most 128 deep (`Fuel`).

## expect

`expect volume|area = V +- T[%]`, `expect faces|edges|bodies = N`, `expect valid`. A failed line is
`ExpectFailed` with the measured value and the limit. An expect whose limit is the measured value
to the last digit is the lint `ExpectCopied`.

## Assemblies

A file whose first line is `assembly` is an assembly document. Its statements make instances of
part files and subassemblies, joints and loose constraints between their references, relations,
fits and fit rules; it is solved by forge-asm (constraints between frames: trees placed in closed
form, loops by Newton). Ids follow the rules above: `bolt = part(..)` inside `for k in 0..4` is
`bolt[0]`..`bolt[3]`. Part features (`extrude`, `sketch`, ..) are refused here (`WrongDocument`).

```
assembly

shared m = 6
param crank.turn = 30deg                       # drives are params: motion is a SetParam

base = part("base.fg", ground: true)           # a part file, relative to this file
arm = part("link.fg", at: pose(at: (0, 50, 0), rz: 30deg))    # the written pose: the solve's start
post = subassembly("post.fg", at: (100, 0, 0))
crank = revolute(arm.a, base.o2, turn: turn, limit: (lo, hi))
slide = slider(block.glide, base.track, along: x)
nut = cylindrical(n.bore, post.rod.axis)       # through a subassembly: post's instance rod
seat = mate(cover.face, base.top, offset: 0.5) # loose constraints mix with joints
pins = gear(crank, idler, ratio: k)            # relations: gear, rack_pinion, screw, linear
f = fit(bolt, base)                            # this pair's contact is meant
m6 = fit_rule(external_thread, internal_thread, size: m, min_engagement: 6)
expect clearance(arm, post) >= 2 at crank.turn = 90deg
```

- **Instances:** `part(FILE, body:, at:, ground:, reference:, suppressed:)` and
  `subassembly(FILE, at:, ..)`. `at:` is a pose: `pose(at: (x, y, z), z: DIR, x: DIR, rz: ANGLE)`,
  a point, or a plane. Written poses are where the solve starts, not design numbers: no
  literals-in-params lint. A reference instance is checked but not counted in the BOM.
- **References:** `inst.NAME` is the part's reference by name, evaluated in the part file: an
  anchor (`a = anchor(plane(..))`), a role name (`shank.axis`, `bar.end`), a sketch point; through
  subassemblies, `sub.inst.NAME`. `inst` alone is the instance's own frame. A reference that
  resolves to nothing is `BrokenAnchor`, before any solve.
- **Joints** (A moving, B the reference; values in B's frame): `fastened`, `revolute` (`turn`),
  `slider` (`along`), `cylindrical` (`along`, `turn`), `pin_slot` (`along` B's x, `turn`), `planar`
  (`x`, `y`, `turn`), `ball`. A value given is a drive; `limit: (lo, hi)` (or `turn_limit:` ..)
  bounds it; `flip: true` turns A's z over. `j.turn` is the value, for relations and expects.
- **Constraints**, each a known number of freedoms removed: `coincident` (3), `on_plane` (1,
  `offset:`), `on_line` (2), `mate` / `flush` (3, `offset:`; mate opposes the normals),
  `concentric` (4), `insert` (5), `parallel` (2), `perpendicular` (1), `angle(a, b, V)` (1),
  `distance(a, b, V)` (1), `lock` (6). Loose constraints that are exactly a joint's bundle are
  solved as that joint.
- **Fits:** `fit(a, b)` declares one pair's contact intended. `fit_rule(A, B, size:,
  min_engagement:)` covers every pair it matches: by anchor kind (`external_thread`,
  `internal_thread`, `shaft`, `bore`, or a part's own `kind("dowel")`) and size, the two anchors on
  one axis, or by part file (`fit_rule("bolt.fg", "plate.fg")`). Threads engaged less than the
  rule's minimum, or the shop's (one diameter), are `ShallowEngagement`; `min_engagement: full`
  asks the screw to fill the internal thread's whole depth. Every threaded fit's engagement is
  reported (`assembly.engagements`: measured, minimum, short or not).
- **In a part file:** `tap = anchor(REF, kind: internal_thread, size: m, depth: t)` declares an
  anchor with a fit feature (a thread runs `depth` along the anchor's z); `dowel = anchor_kind()`
  declares a kind of the part's own.
- **Expect:** `interferences`, `interference(a, b)` (volume), `clearance(a, b)`, `distance(a, b)`
  (origins), `x|y|z(a)`, `value(j.turn)`, `mobility`; `at P = V, ..` holds params for that line.
  Wherever an instance goes, a reference does too (`x(rail.eia[0])`, `z(crank.bar.end)`): its
  frame in the world. A subassembly is all of its parts: `clearance(s, a)` is the least over
  them, `interference(s, a)` their summed volume. `interference(a, b)` is the solids' exact
  shared volume whatever a fit or fit rule says (fits change only which overlaps warn;
  `interferences` counts the unintended ones). An expect that cannot be measured is the error
  `ExpectUnevaluable` (why: a missing argument, no such reference, not a place, no solid, no
  joint value), never skipped; `fg run --json` lists every expect with its measured value, limit
  and verdict.
- **Results**, typed: errors `BrokenAnchor`, `WrongReference`, `Conflict` (the minimal set,
  per constraint with its joint; fix: delete the statement left out), `LimitViolated` (fix:
  SetParam the drive to the nearer limit), `NotConverged`, `NoGround`, `UnknownKind`,
  `PartFile`; warnings `Redundant`, `Interference` (fix: add a `fit`), `ShallowEngagement`;
  information `IntendedContact`. `fg run --json` gives `assembly`: poses, joint values and free
  motions, mobility, fit rules and the pairs they covered, BOM, part hashes, timings.
  `fg run --render` draws the posed assembly; `fg serve` keeps the solve, so a drive edit
  re-solves only its partition and re-tests only what moved.

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
SetArg {id, arg, text}              keyword argument arg of the call making id becomes text
Rename {from, to}                   a top-level name and every use of it renamed
```

A name a built-in also has is `ShadowsBuiltin`, with the fix Rename to `NAME_1`.

An op fails only with a typed error: `UnknownParam`, `UnknownFeature`, `NotAStatement` (an
instance made in a loop or call: edit its params, or SplitInstance), `NameTaken`, `EditDim` (the
value's dimension is not the param's), or the edited document's own errors.

## CLI

```
fg run FILE [--json] [--render OUT.png] [--view iso|front|top|right|back|left|bottom] [--px N]
           [--dxf OUT.dxf] [--step OUT.step]
fg check FILE [--json]
fg edit FILE 'OP' [--json]
fg handles FILE
fg serve                    JSON lines {"cmd":"run","file":F}: stages kept, so a rerun builds
                            only from the first feature that changed
fg reference
```
