# Parameters and sketches: design

A part is a Forge (`.fg`) program (`interface/forge-lang`, the language in its
`src/reference.md`). Evaluating it replaces every expression by its value and every sketch
reference by the solved geometry, feature by feature. So a parameter change regenerates exactly as
if the new value had been typed in, and `forge-model` keeps working on plain numbers (no regen
cost, nothing for the kernel to learn).

## Parameters

```
shared wall = 2              # read by any feature
param boss.h = wall * 3 + 1  # a param of feature `boss`
param boss.a = 30deg
param bolts.n = count(4)
```

- Numbers live in params; a loose number in a statement is the lint `LooseLiteral`, whose fix
  (`LiftLiteral`) makes it one. Inside feature F's statement a bare name finds F's own param
  first, then names in scope, then shared params.
- Expressions: `+ - * / ^`, parentheses, `sqrt sin cos tan asin acos atan atan2 min max abs`,
  `pi`, `div mod` on counts. Trig takes angles (`30deg`) and gives ratios; inverse trig gives
  angles. Arithmetic is `forge_math::dmath`: bit-identical everywhere.
- Dimensions are checked, not converted: Length, Angle, Count, Ratio. A size is never a count;
  mixing them otherwise is `DimMismatch`/`WrongDim`.
- A name is bound once, above its use: rebinding is `Rebind`, so there are no cycles.
- Every number records where it came from: driving (one param read by name, which `SetParam`
  edits), driven (an expression of params), or const. A param whose value evaluation observed (a
  loop bound, an index, a selection's filter) is structural: editing it re-runs evaluation.

## Sketches

```
param base.w = 40
param base.h = 25
sketch base(on: xy) {
  outline = rect(corner: (0, 0), size: (w, h))   # sides outline[0..3]
  c = circle((w / 2, h / 2), 4)
  l = line(near(0, -5), near(w, -5), construction: true)
  horizontal(l)
  distance(l.a, outline[0], 5)
}
```

- Entities are named by their binding (`base.c`, `base.outline[2]`); their members are points
  (`l.a`, `l.b`, `c.center`), directions (`l.dir`) and axes. Coordinates written are fixed;
  `near(x, y)` is a free point. Constraints are statements (`tangent`, `coincident`,
  `horizontal`, `distance`, `radius`, `angle`, `symmetric`, ..); the solver is forge-sketch.
- A sketch is one feature, solved from the coordinates as written every time it is evaluated,
  so a regen never depends on an earlier solve. It sits on a world plane (the first feature
  only), an anchor of the part (`boss.end`, `base.side(plan.outline[1])`), `offset(..)`,
  `rotate(..)`, a work plane or `plane(at:, normal:, x:)`, and follows it through every edit.
- Downstream, a profile is a sketch's one region or one of its loops or closed entities
  (`extrude(base.c, t)`); a revolve's axis, a coil's, a path or a hole's points are its
  entities.

## Projected geometry

`project(EDGES | FACES | POINT | AXIS, approx: TOL)` lays 3D geometry of the part as it stands on
the sketch's plane, read afresh every run, so it follows edits there; each point it makes is
fixed, so it adds no freedom.

- Lines, circles and ellipses project exactly: a circle or ellipse is O + cos t A + sin t B in the
  plane (A, B its semi-axes laid flat), an ellipse whose axes are where |A cos t + B sin t| is
  greatest and least, tan 2t = 2 A.B / (|A|^2 - |B|^2).
- Any other curve has no exact image: it is `NeedsApprox` unless `approx: TOL` is given, then a
  fitted cubic within TOL, so a fitted result is never silent. An axis projects only seen end-on
  (`NoImage` otherwise).
- One curve gives an entity, several a loop; a face gives its boundary.

## References

Faces and edges a sketch's entities make are named from the feature's id and the entity's
binding, not from its position in the solver, so `boss.side(base.outline[1])` and selections
of them (`edges(between: (a, b))`) keep naming the same faces when dimensions change and when
entities are added: fillets and chamfers downstream survive.

## Diagnostics

- Under-constrained is allowed, the warning `UnderConstrained` with the freedom left: it solves
  from the coordinates as written, moving them as little as possible.
- A sketch that does not solve is `SketchConflict` naming the conflicting rows.
- Every error and lint is typed (kind, span, data) with its fixes as edit ops.

## Incremental rebuilds

A session keeps each feature's built stage; a rebuild starts at the first feature whose inputs
changed. Editing a non-structural param re-reads the stored formulas without evaluating again.

## The document

The source of truth is the `.fg` text. Edits are typed ops on it (`SetParam`, `AddFeature`,
`ReplaceFeature`, `MoveFeature`, `Rename`, ..; `forge_lang::edit`), each replacing one span or
inserting at one point, every other byte kept: a hand edit and an agent's edit are the same
kind of change.

## Over-constrained

As in Inventor, a constraint or dimension the rows above it already imply is an error, even
when consistent: the solver finds it (ties met twice, or a row adding no rank to its group's
Jacobian at the solution) and names the row.
