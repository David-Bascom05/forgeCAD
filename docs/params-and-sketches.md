# Parameters and sketches: design

The `.fc` text is the parametric document. A `Part` is that text evaluated: every expression
is replaced by its value and every sketch reference by the solved geometry when the line is
read. So a parameter change regenerates exactly as if the new value had been typed in, and
`forge-model` keeps working on plain numbers (no regen cost, nothing for the kernel to learn).

## Parameters

```
wall = 2                 # a line of its own; a history step that makes nothing
d = wall*3 + 1 mm        # optional unit: mm | deg (none: a plain number)
a = 30 deg
extrude d rect 0 0 w/2 (h + wall)        # any number in any line may be an expression
```

- Expressions: `+ - * / ^`, parentheses, `sqrt sin cos tan asin acos atan atan2 min max abs`,
  `pi`. Trig takes degrees (the file's angle unit); inverse trig gives degrees, a `deg` value,
  from a ratio (`atan2(y, x)`: the angle of the point x, y), so an arc through computed points
  stays parametric. `^` takes a whole exponent, or 0.5. Hand-written recursive
  descent over `f64`, trig through `forge_math::dmath`: bit-identical everywhere.
- In a feature line an expression is one token (`w/2`) or a parenthesized one (`(h + wall)`);
  a `(` token is a selector only if it starts with `faces|edges|verts`.
- A name must be defined above its use, once. Unknown names, self reference and names defined
  only below give errors naming the line. Define-before-use rules out cycles.
- Units are checked, not converted: a `deg` value may only fill an angle, an `mm` value only a
  length; mixing them in one expression is an error. Counts must come out whole.
- Stored as `Feature::Param { name, expr (text), unit, value }`, printed back verbatim.

## Sketches

```
sk: sketch xy at 0 0 6          # FRAME, or `on REF` (a planar face: its plane, outward z)
  rect 0 0 40 25                # l1..l4, corners shared, horizontal/vertical held
  circle 20 12 4                # c1
  arc l2.b 30 30 l3.a           # a1: centre, start, end (ccw; `cw`); points may be refs
  line 0 -5 40 -5 cons          # l5, construction (not in profiles)
  fix l1.a                      # constraints: fix co hor ver par perp tan eq conc mid sym
  dist l1 w                     # dimensions: dist hdist vdist rad dia ang, valued by
  dist c1 l1 (h/2)              #   any expression
  dia c1 8
end
```

- Entities are named by kind and count in the block: `p` points, `l` lines, `c` circles,
  `a` arcs; their points `.a` start, `.b` end, `.c` centre (a bare circle/arc where a point is
  wanted means its centre). `o` is the sketch origin, fixed.
- The block is one history step, `Feature::Sketch` (its plane, rows as written, and the solved
  `forge_sketch::Sketch`). It is solved from the coordinates as written every time it is read,
  so a regen never depends on an earlier solve.
- Downstream:
  - `extrude 6 sk [E]`, `revolve sk [axis L] [E]`: anywhere LOOPS go, a sketch's one region,
    or the region whose outside loop runs through entity E.
  - FRAME `on sk [P]` / `-on sk [P]`: the sketch's plane (flipped), origin at point P; so
    `hole 5 -on sk c1 thru` drills at a sketch circle's centre.
  - PATH `sk [E]` (sweep, loft rails): the sketch's one chain of lines and arcs, or E's.
- A sketch placed `on` a plane, sketch or face keeps that reference (`Sketch.on`), printed
  back as written and re-resolved every time the line is read, so it follows its plane.

## References

Profile segments from a sketch carry ids from the entity's kind and count (`l3`, `c1`), not
its position in the solver, so extruded faces and edges keep their names when dimensions
change, and when entities of other kinds are added: fillets and chamfers downstream survive,
the same as for features today.

## Diagnostics

- Under-constrained is allowed: it solves from the coordinates as written, moving them as
  little as possible. `forgecad check` reports each sketch's degrees of freedom.
- A sketch that does not solve is an error on its line naming the conflicting rows
  (`dist l1 40`, `dist l1 w`) and the redundant ones, with the hint to remove or change one.
- Profiles that do not close, branch or cross name the entity: `the chain through l3 is open`.
- Rank analysis runs only on failure or when asked, never on a regen that solves.

## Incremental rebuilds

A session reparses every line after a changed one; a line whose result equals the old one
keeps its built stage as long as everything changed above it makes no solid (parameters,
sketches). Changing `wall` rebuilds from the first line whose numbers change.

## The document (forge-doc)

The source of truth is `forge_doc::Document`: one item per line (parameters, sketches with
their rows, feature lines with their arguments as written, blank and comment lines), each
with an identity kept through edits. The `.fc` file is its print, and reads back to it. A
feature reference (`sketch2`, `f4`, a label) records the item it named when the line was
first evaluated and prints by that item's place, so it follows the item when lines are
inserted, deleted or moved. Edits: set a parameter, set a dimension, set an argument (a value
or reference), insert, delete, move. Evaluation is the part-file parser, line by line: the
session rebuilds from the first line an edit changes, keeping later stages while lines read
the same. The kernel still sees only `Feature`s of plain numbers.

## Over-constrained

As in Inventor, a constraint or dimension the rows above it already imply is an error, even
when consistent: the solver finds it (ties met twice, or a row adding no rank to its group's
Jacobian at the solution) and the message names the row and offers `ref`, a driven dimension
that constrains nothing and shows its measured value in `check`.
