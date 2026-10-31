# Multi-body parts (design)

## Bodies

A part's history may hold several solid bodies. A body is named by the line that made it:
its FEAT reference (`base`, `extrude3`, `f7`). In the model a body id is that feature's index.
Lines that make a body:

- the first solid, or a later `new` (as today, `new` throws away everything before it);
- a solid feature with op `solid` (Inventor's "new solid");
- `split ... keep both`: the piece on the `+` side of the cutter (the other keeps the old id);
- `copy`: the copy.

Regen state is `Vec<Solid>`, `Solid { id: usize, body: Body }`, in the order made. Each body is
its own `Body` and is checked on its own. Separate bodies may overlap; one `Body` may not hold
overlapping shells (bool-touch's check). The two never meet: bodies are never merged into one
`Body` for the kernel. forge-agent concatenates them into one display `Body` for selecting,
meshing and rendering only, and `check` runs per body.

Names stay unique across the whole part, so handles and selectors work unchanged. A feature
that puts the same tool into a second body, `split`, `copy` and `combine ... keep` salt the
names of the entities they would otherwise duplicate (`name.with_str("body").with(id)`).

## Targeting

`Feature` gains one variant, `Feature::Body(Box<BodyFeature>)`; existing variants are
untouched:

```
enum BodyFeature {
    In { bodies: Pick, feature: Feature },              // any feature, on chosen bodies
    Combine { op: Combine, target: usize, tools: Vec<usize>, keep: bool },
    Split { by: Cutter, keep: Keep },                   // Keep: Plus | Minus | Both
    SplitFace { faces: Vec<Name>, by: Cutter },
    Move { steps: Vec<Step>, copy: bool },              // Step: Translate | Rotate | Frame
}
enum Pick { All, Some(Vec<usize>) }
enum Cutter { Target(Target), Plane(Frame), Curve(Frame, Segment) }
```

`Combine` gains `Solid`. A feature acts on the bodies its `In` names, else:

- fillet, chamfer, draft, shell with open faces, splitface: the bodies holding their names;
- everything else: the current body, the most recently made body still alive.

A tool feature `in a,b` combines its tool into each. `split`, `move`, `copy` act on the
current or `in` bodies (`copy` on one).

## Regen

Before regenerating, `In` wrappers are stripped into a plain feature list (borrowed, not
copied, when there are none) and a per-feature target list, so all existing code (extents,
holes, patterns, up-to) sees the features it knows, at the same indices.

A history with no body features and no `solid` op is single-body: `regenerate`, `step` and
`regenerate_nary` run exactly today's code. The check is one scan of the features. A
single-body part does no extra work.

Multi-body, sequential: each feature applies to its target bodies (`step_solids`).

Multi-body, n-ary: each body is a lazy CSG expression, its base (a tool or a built body) and
the tools since, in order. A modifier, a body feature, or `combine`/`split` reading a body
flushes that body's run in one `nary_with` (analytic shapes when its base is a tool). The end
flushes every body. Tools are built in parallel up front, as now. The GPU batch is only used
for single-body parts.

`next` extents in a history that has made more than one body are Unsupported for now (the
pick would have to follow one body's tools).

Names: the n-ary regen names a face a boolean only adds a loop to differently from the
sequential one (a single-body limit too: a shell or chamfer naming such a face fails n-ary).
A body feature naming faces inherits it.

`regenerate()` and `regenerate_nary()` keep returning one `Body`; on a part that ends with
several they fail with `RegenError::Bodies`. `regenerate_solids()` and
`regenerate_nary_solids()` return them all.

## Part file

```
extrude solid 10 rect 0 0 5 5                 # a new body
extrude cut 2 circle 0 0 1 in base,f4         # any line: `in FEAT[,FEAT..]` or `in all`
combine (add|cut|keep) TARGET TOOL.. [keep]   # keep: the tool bodies stay
split BY [+|-]                                # keep one side; default both
splitface REF.. by BY                         # each face becomes pieces, the solid unchanged
move MOTION..  |  copy MOTION..               # applied in order
BY:     FEAT [@face] | plane FRAME | curve FRAME (line AX AY BX BY | arc CX CY R A0 SWEEP | circle CX CY R)
MOTION: by DX DY DZ | rot OX OY OZ AX AY AZ A | frame FRAME
```

`in` is the last two tokens of a line, so every kind takes it without touching its parser.
A curve cuts along its sketch's normal (a line's plane, an arc's cylinder).

Selectors and `query` gain `body FEAT`; `query` prints `body=NAME` on parts with more than
one body; `build` prints the body count.
