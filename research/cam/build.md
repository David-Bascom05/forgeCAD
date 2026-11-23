# CAM: the build design (session cam, 2026-11-23)

The research (README.md, design.md and the library notes) says *what*. This note says what this
branch builds, in which crates, with which syntax and formats, and how Phase 0's go/no-go is
measured. Where it departs from design.md it says why.

## 1. Crates

| crate | where | what | deps | proof target |
|---|---|---|---|---|
| `forge-cam-geom` | kernel/ | exact numbers (`big`: naturals and integers; `real`: exact reals over +,−,×,÷,√ with a sign decided exactly); line/arc profiles and regions; exact region offsets; tool shapes as revolved line/arc profiles; the certified clearance checker (its own branch and bound, written to forge-brake's `Problem` contract) | none (std only) | L4 by Aeneas, L5, L6 |
| `forge-cam` | kernel/ | setups, WCS, stock (box, cylinder, from solid), operations as pure functions (face, contour, pocket, slot, chamfer, trace/engrave, drill/peck/tap, bore, thread mill), linking (leads, ramps, retracts, heights), the in-process stock for 2.5D (exact, as line/arc slabs), feature-aware hole plans, cycle time, the `forgecad.cam/1` reply | forge-cam-geom, forge-math, forge-geom, forge-topo, forge-tooling | safe by construction: every path passes the checker or is refused |
| `forge-tooling` | kernel/ | tools, holders, assemblies, machines, materials' cutting data and feeds (chip thinning), as our TOML; a built-in library in our own numbers; search order project → user → built in | forge-math | data, validated |
| `forge-post` | kernel/ | the dialect engine (Fanuc, Haas, LinuxCNC, GRBL as TOML data), integer-quantised printing, and a reader of our own output (round trip, G6) | forge-cam-geom (motion types) | L4 by Aeneas for the quantiser and reader |
| CAM in `forge-lang` | interface/ | the `machining` document; `query F cam`, `export F prog`, `export F setup-sheet`; MCP | forge-cam, forge-post, forge-tooling | — |
| `forge-cam-import` | interface/ | the Fusion tool-library JSON importer (and later GTC, FreeCAD, LinuxCNC export) | forge-tooling | — |

Departure: design.md has forge-cam-geom depend on forge-math/geom/topo. Here it depends on
nothing, so Aeneas can take the whole crate: the B-rep is lowered to the checker's own face
pieces in forge-cam, and refused there (typed) where a face is outside the 2.5D domain.

Departure: the B&B is not imported from forge-brake (bendsim branch, not on our base, and its
code uses closures, iterator chains and a BinaryHeap, which Aeneas at the pin cannot take). It is
reimplemented in forge-cam-geom to the same contract, with the soundness rule the bendsim fix
states: `Clear` only when the proven lower bound is ≥ the limit (no "within res" pass),
`Hit` only with a real witness, else `Undecided`, which is refused. When bendsim merges, the two
should become one crate; the contract is identical so that is mechanical.

## 2. Exactness: how decisions are made

- **Numbers.** Inputs are f64, so exact dyadic rationals. Constructed values (offset lines carry
  √(dx²+dy²); line/circle meets carry a nested √) are kept as an expression DAG over the inputs.
- **Sign, decided exactly.** A filter first: the DAG in directed-rounding f64 intervals. If 0 is
  inside, the DAG is re-evaluated in big-float intervals at doubling precision until 0 is
  excluded or the interval is inside the BFMSS separation bound (Burnikel, Fleischer, Mehlhorn,
  Schirra 2000), which proves the value is 0. One rule, no epsilon anywhere. Ties (a slot exactly
  the tool's width, four offset lines meeting at a square's centre) are decided as ties.
- **Representative points are rational.** The keep test of an offset piece is made at a point
  chosen with a rational parameter (lines) or a rational tangent-half-angle (circles: an exact
  rational point on a rational circle), so it needs no radicals of its own.

## 3. The 2D offset (Phase 0)

The region R: outer loops CCW, islands CW, each a closed chain of lines and arcs with shared f64
endpoints, validated exactly (closed, simple, disjoint). The inward offset at distance r is
C_r = {p ∈ R : d(p, ∂R) ≥ r}, the tool-centre region; outward is the same on the complement.

- **One rule for corners:** every vertex contributes an arc of radius r about it, between the
  normals of its two edges; every line its parallel at r on the inside; every arc its concentric
  arc at ρ ∓ r (a point at 0, gone below). The raw offset is these pieces in boundary order.
- **Events decided exactly:** all meets of raw pieces, ordered exactly along each piece; the
  pieces are split there; a sub-piece is kept iff its rational representative point has
  d(p, e) ≥ r for every boundary element e (exact). Kept pieces are linked into loops. A
  1-dimensional part of C_r (a slot of width exactly 2r) comes out as a there-and-back loop,
  which is the right tool path.
- **Proof statement (L1):** the loops bound exactly {p ∈ R : d(p, ∂R) ≥ r}, i.e. R ⊖ B_r, and
  every pass of a pocket is offset from the original boundary, never from the previous pass.

## 4. Tools and the clearance checker (Phase 0)

- **Tools** (flat, ball, bull, chamfer, drill, and spot/tap/thread-mill as their envelopes) are an
  exact profile in (ρ, z): lines and arcs from the tip up the flutes, the neck, the shank; the
  holder is the same kind of profile above the gauge line. The cutter T and non-cutting N are
  separate profiles. Signed distance to a solid of revolution is a 2D distance in (ρ, z): closed
  form, provable (L5).
- **The checker** is forge-brake's shape with the roles swapped: the moving thing is the *part's
  faces seen from the tool* (cells over each face's (u, v) and the motion s), and the target is
  the tool's profile, whose signed distance is the closed form above. A face is a piece the
  forge-cam lowering produces: plane with a line/arc region; cylinder, cone, sphere, torus with
  (u, v) bounded by iso-lines. Others are refused for now (typed `UnsupportedFace`).
  Moves: line, arc, helix in s ∈ [0, 1]. A cell's enclosure is interval arithmetic; the lower
  bound is sd(centre) − half-diagonal minus the error budget; a witness is a face point proven
  inside its trim at a real s.
- **G1** then reads: min over faces, cells and s of sd_T ≥ δ − ε_in (a tangent finishing pass
  clears by ε_in, stated in the formula), plus one point of the tool proven outside P (so no
  tool sits wholly inside). **G2/G3** are the same checker with N (or T on rapids) against the
  in-process stock, which in 2.5D is exact line/arc slabs.
- **Culling:** a face whose box is farther than the limit from the move's swept box clears in one
  certified step (box distance is a lower bound), so far moves cost one box test per nearby face
  (a uniform grid over face boxes).

## 5. The Phase 0 speed experiment (the go/no-go)

Bench: examples/machined/flange.fg (and the other machined examples that lower to 2.5D faces).
For each: an offset pocket and a contour finishing pass with a 6 mm flat and a 6 mm bull, at
1 000, 10 000 and 100 000 moves (the long ones by subdividing arcs/lines, which is what an
adaptive path's move count looks like). Measured natively (release) and under callgrind:
- cells per move, near and far;
- µs per move, and seconds per 10⁵-move op.

**Go** if a 10⁵-move op certifies in ≤ 1 s natively (design.md's line). **Conditional go** up
to 10 s: certify in the background, the op shows `undecided: pending`, never `pass`, until it
lands. **No-go** above that: the design changes (exact 2.5D distance by slabs instead of B&B).

## 6. The .fg surface

No grammar change. A machining document is a `.fg` file whose first statement is the bare name
`machining` (an expression statement, as `drawing` is), then calls. New built-ins are abstract
IDs, so research/forge-lang/lean needs no new production; FG_LEAN=1 parse_diff must still pass.

```
machining
shared stock_z = 22
param rough.step = 2
f   = part("flange.fg", material: "6061-T6")
m   = machine("generic-vmc3")
lib = tools("shop.tools.toml")
op1 = setup(f, machine: m, wcs: (origin: f.plate.end, z: +z),
            stock: box(margin: (2, 2, 2)), fixture: vise(open: 100))
face  = facing(op1, tool: lib.T1, down: 1)
rough = pocket(op1, f.pocket, tool: lib.T2, step: step, down: 3, leave: 0.3)
prof  = contour(op1, edges(of: f.plate, crease: "convex"), tool: lib.T3, comp: "computer")
holes = drill(op1, f.bolts, plan: auto)       # spot, drill or peck, tap or ream from the hole
prog  = program([op1], post: "haas", number: 1001)
```

## 7. Library formats

As libraries-tools.md §5.2–5.4 and libraries-machines-posts-feeds.md §5.2–5.4, with these
choices: one TOML subset reader (forge-material's, lifted into forge-tooling); required fields
per type refused with a typed error and a fix, never defaulted; every record has `source`,
`taken`, `note`; vendor data marked "verify with the vendor". Built-in sets are ours: generic
flat/ball/bull 1–20 mm, a drill series, M taps, spot, chamfer, thread mill; generic BT40/CAT40
ER holders labelled conservative; generic 3-axis VMC; dialects fanuc, haas, linuxcnc, grbl.
Cutting data: per ISO group (P M K N S H) `vc`, `fz` by diameter, `kc1_1`, `mc`, sourced.

## 8. The reply

`forgecad.cam/1`, bendsim's shape: setups, ops (tool, feeds with hex after thinning, path stats,
`checks[]` with rule/verdict/certified/value/limit/at/s/tool part/other/phase, leftover),
`blocking[]`, `fixes[]` (edit ops), `suggestions[]` (`NeedTool`), `programs[]`.

## 9. Not in this phase

Adaptive/constant-engagement clearing (patents.md: counsel first), 3-axis surfacing, 3+2,
turning, 5-axis: design only.
