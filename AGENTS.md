# Driving ForgeCAD

A part is a `.fg` file, a Forge program: statements that make named features in order, each built
as it is made. Units mm, angles `30deg`. `help forge` has the whole language.

```
shared L = 60                         # numbers live in params, nowhere else
shared W = 30
shared t = 6
param top.r = 8                       # a param of feature `top`
param top.hx = 10
param bolt.d = 5
param bolts.step = 40
param round.r = 2

sketch plan(on: xy) {                 # the first sketch may sit on a world plane
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
sketch top(on: base.end) {            # every later one sits on the part
  c = circle((L / 2, W / 2), r)
  h = pt(hx, W / 2)
}
boss = extrude(top.c, t)
bolt = hole(at: top.h, d: d, depth: thru)
bolts = pattern(of: bolt, layout: lin(along: plan.outline[0].dir, n: count(2), step: step))
round = fillet(edges(of: boss, crease: "convex"), r)
```

Select in the part's own text: `edges(of: boss, crease: "convex")`, `edges(between: (a, b))`,
anchors like `base.end`, `base.side(plan.outline[2])`, `hole.bore`; one copy of a pattern or one
place of a hole: `edges(of: bolts[1], crease: "convex")`. Features: extrude cut revolve
sweep loft coil thread hole fillet chamfer shell draft rib pattern, bodies, sheet metal, components
and assemblies (`help forge`).

Tools (CLI `forgecad TOOL F ..`, or MCP). Replies are typed JSON with `--json` or over MCP:
```
build F                     every feature's status and dims, then the part: volume, faces, valid
check F                     the same, stopping before the kernel
edit F 'OP'.. [--base V]    typed ops, all applied or none; replies with the build and the
                            file's `version`; with --base V, refused (StaleBase) unless still at V
diff A B [volume]           what B changed from A: features, params, faces by name, volume
query F params|features|stat|check
query F SEL [--at FEAT]     @handles: `faces plane normal +z top`, `edges of boss convex`;
                            --at: the part as it stood after that feature (measure, render too)
query F holes|aligned|interference|faces | section z=67 | gap A B | frame SKETCH
measure F REF [REF] | mass | wall [exact]
render F [VIEW] [--labels SEL] [--px N] [-o F.png]
export F [ap214|ap242] [tol MM] [-o F.step] | dxf [SHEET] | dwg [SHEET]
export F stl|3mf|obj [tol MM] [angle DEG] [ascii] [normals] [faces] [flat] [-o F.3mf]
                            meshes: an object per solid (an assembly's: per part, a 3MF item
                            per occurrence; flat: one each), tol 0.01 mm, angle 15 deg by
                            default; each checked closed and facing out (`watertight`), `manifold`
help forge
```
SEL: `faces|edges|verts`, then filters: plane cyl cone .. | line circle | convex concave |
normal +z | along +x | r=3 | longer 5 | of FEAT | between FEAT FEAT | top bottom left right front
back | near X Y Z.
REF: `@handle` or `(selector)`.

Ops, as a reply's `fixes` give them; send a fix back unchanged:
```
{"op":"SetParam","param":"t","value":{"value":3,"dim":"Length"}}
{"op":"AddFeature","before":null,"text":"edge = chamfer(edges(of: base, along: Z), 1)"}
{"op":"SetArg","id":"bolt","arg":"depth","text":"thru"}
{"op":"Rename","from":"round","to":"blend"}
```
Also AddLocalParam, MakeShared, SplitInstance, DeleteFeature, ReplaceFeature, MoveFeature,
LiftLiteral, SetProperty; `{"op":"Undo"}` and `{"op":"Redo"}` step through the file's edits. A
refusal is a typed error and nothing is written; a round too big comes back with a SetParam fix to
the largest that builds. Errors and lints
carry spans and `fixes`; a loose number is a lint with a LiftLiteral fix.

A `.fg` file is plain text: write a new part whole, then `build` it; hand edits are fine. An
assembly's names are OCC/NAME and its numbers are in its own frame (`in OCC` for a part's);
`fg query F help` lists its queries.
