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
size is never a count. The constants `0 1 2` (and `-1 -2`) take the other side's dimension.

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
shell(t, open: FACES)
loft([PROFILE, PROFILE, ..])
```

PROFILE: a sketch with one region, or one of its loops or closed entities (`S.outline`, `S.c`).
A negative depth runs the other way.

## Anchors

Every feature exports anchors by role; they resolve to the kernel's role names
(`forge_ops::roles`), never to hashes read off a part.

```
E.start  E.end          extrude caps (cut: E.end is the floor), with planes whose x is the
                        sketch's x and origin the sketch origin dropped onto them
E.side(ENTITY)          the wall a profile entity made (a line: plane with origin at its start,
                        x along it); revolves too: R.side(ENTITY)
E.axis                  a single-circle extrude's axis; a revolve's or hole's axis
E.faces  E.edges        everything the feature made, as the part stands
H.rim                   a hole's edges at its mouth
S.plane  S.origin  S.points  S.NAME   a sketch's plane, origin, points, named entities
FACES.plane  FACES.normal  FACES.edges  FACES.axis (cylindrical)
```

## Selections

```
edges(of: F | [F..] | FACES, between: (A, B), crease: "concave"|"convex"|"smooth",
      shape: "line"|"circle"|"curve", along: DIR, r: R, longer: L, shorter: L)
faces(of: F | [F..], shape: "plane"|"cyl"|"cone"|"sphere"|"torus"|"blend", normal: DIR, r: R)
EDGES + EDGES
```

`of` means made by (lineage through later booleans); `between: (A, B)` the edges where A's faces
meet B's, A and B features or face sets. Directions may be world axes `X Y Z` here (orientation,
not placement). Selecting by where things lie (`extreme:`) is the error `PositionalSelection`:
an unrelated edit can retarget it. A selection that picks nothing at a fillet or chamfer is
`EmptySelection`.

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
fg check FILE [--json]
fg edit FILE 'OP' [--json]
fg handles FILE
fg serve                    JSON lines {"cmd":"run","file":F}: stages kept, so a rerun builds
                            only from the first feature that changed
fg reference
```
