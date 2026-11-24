# Bend simulator: design

A press-brake simulator for ForgeCAD sheet parts. It folds the part bend by bend on the vendor's
tooling, the exact partial refolds of audit2 §2.0, checks tool clearance and the rules that go with
it through the motion, plans a bend order and tooling, and answers in typed JSON.

New crate `kernel/forge-brake` (depends on forge-math and forge-sheet only; nothing in forge-sheet
changes, so the sheet-geometry session is not disturbed). The host side lives in forge-lang
(`query F bendsim`), the CLI and MCP get it via `query`, and the app gets a view.

## 1. Tooling as data

Units mm and degrees in files, radians inside. Every tool profile is a closed loop of lines and arcs
in the press's section plane (x across the bed toward the operator, z up), extruded along the bed
(y). The punch tip and the die's V centre are at x = 0.

```
Punch {
  id: "EU-P88-R08",            // unique in a tooling set
  style: "eu" | "us",          // Promecam-style tang / American tang
  kind: "straight" | "gooseneck" | "acute",
  height: 120,                 // tip to clamping face
  tip_radius: 0.8,
  tip_angle: 88,               // included, at the tip
  width: 26,                   // body width at the clamp
  relief: { depth: 50, z0: 30, z1: 95 } | null,   // gooseneck throat: how far behind the tip
                                                  // line, between heights z0..z1 above the tip
  profile: [Seg..] | null,     // explicit loop overrides the generated one
  segments: [{ length: 10, count: 2 }, ..],       // segmented lengths available
  max_load: 100,               // kN/m the tool is rated for (optional)
}
Die {
  id: "EU-V12-88", style, v: 12,        // V opening at the top, between the sharp edges
  angle: 88, shoulder_radius: 2, height: 60, width: 60,
  profile: null | [..], segments: [..], max_load: ..,
}
Press {
  id, open_height: 400,        // daylight: ram clamp face to bed die seat, at top dead centre
  stroke: 200, bed_length: 3000, housing_gap: 2550,   // clear width between the side frames
  ram:  [Seg..] | { front: 60, back: 60 },             // upper beam envelope above the punch
  bed:  [Seg..] | { front: 60, back: 60 },             // lower beam envelope below the die
  max_force: 1000,             // kN (optional)
  backgauge: {
    fingers: [{ y: -400 }, { y: 400 }],               // positions along the bed
    x: [5, 600],               // stop face range behind the V centre line
    r: [0, 150],               // finger height range above the die top
    finger: { depth: 20, height: 20, width: 15 },     // the finger's box behind its stop face
    retract: 5,                // how far the fingers back off once the punch clamps the part
  },
}
Tooling { id, punches: [..], dies: [..], press: Press,
          clearance: 0.5,          // tool to part, the least allowed
          self_clearance: 0,       // part to itself, the least gap allowed
          springback: [{ material: "steel", angle: 90, overbend: 2 }, ..] }
```

Standard library, numbers restated by us (no catalogue copied), ids under `std`:
- `std-eu` (Promecam-style): straight punches P88 and P85 (R0.8 / R3), gooseneck G88-R08
  (relief 50 deep), acute A30-R04; dies V6, V8, V10, V12, V16, V20, V25, V32 at 88°,
  acute dies V8-30 and V12-30; a 100 t / 3 m press.
- `std-us` (American): straight 85° and 90°, gooseneck 85°, acute 30°; dies 1/4" .. 1-1/4" V at
  85° / 90°, 30° acute; a 90 t / 10 ft press.

**Vendor format.** The DFM session's vendor profile gets a `tooling` section: either
`"tooling": "std-eu"` (a library set by id) or a `Tooling` object above, or
`{ "base": "std-eu", "punches": [..], "dies": [..], "press": {..}, .. }` (the set with lines
added or replaced by id). Field names as above, to be agreed with that session.

## 2. The simulation

A **fold state** assigns every bend j an angle φ_j in [0, θ_j + overbend_j]. It is the exact partial
refold of the developed part D (audit2 §2.0): walls are the same rigid pieces, and zone j folds with
its neutral length kept, BA_j fixed, so its neutral radius is rn' = BA_j / φ_j and every layer
moves by rn' − rn. At φ_j = θ_j this is the folded part to the bit (tested against the builder's
layout); at φ_j = 0 it is the flat strip. Isometry of the neutral layer (item 2) holds at every φ.
All formulas are written with sinc and versine forms, so φ → 0 is stable.

A **step** bends one bend from 0 (or from its done state, never) to θ + overbend, then springs back
to θ. The press places the part:
- flipped so the bend's inside faces the punch, rotated end for end (two orientations), shifted
  along the bed (centred on the tooling, or a stated y);
- both legs tangent to the die shoulders (air bending, both legs at φ/2), the punch tip on the
  zone's inside midline; z of the tip follows from tangency in closed form.

The tool and part pieces are exact: tool profiles are lines and arcs; part pieces are planar slabs
(the wall's region: its outline minus the zones' slots, lines and arcs) and cylindrical shells (the
zone's slot), mapped by the exact fold state.

## 3. The checks

Per step, through the motion:

| rule | what | value vs limit |
|---|---|---|
| `tool_collision` | part vs punch body, die, ram, bed, backgauge fingers | least distance vs `clearance` |
| `closes_on_punch` | an already-bent flange closes on the punch (box / channel) | least distance vs `clearance` |
| `self_collision` | moving side vs the rest of the part | least gap vs `self_clearance` (0: touching allowed) |
| `min_flange` | each leg reaches past the die shoulder's contact | leg length vs contact reach |
| `v_too_narrow` | the shoulder contact falls on the zone, not the leg | contact x vs zone half-chord |
| `die_angle` | the overbent included angle vs the die's angle | included vs die angle |
| `punch_angle` | the same vs the punch tip angle | included vs punch angle |
| `punch_radius` | punch tip radius vs inside radius | tip radius vs R |
| `backgauge_reach` | a gauging edge parallel to the bend on the back leg, within the fingers' x and r | gauge x vs range |
| `open_height` | the part, flat on the die, clears the open punch tip | part height vs daylight |
| `stroke` | ram travel from open to the bottom of the bend | travel vs stroke |
| `bed_length` | the part's extent along the bed vs bed and housing gap | length vs limit |
| `segments` | a sum of available segments covers the bend and fits between flanges along y | best sum vs [need, room] |
| `tonnage` | air-bending force (if the material's UTS is known) vs press and tool | kN vs kN |

**Exact collision, certified through the motion.** Collision never uses a mesh. Each tool is a
prism along y, so its signed distance is closed form (2D distance to lines and arcs, combined with
the y-interval). A part piece is a box of parameters (u, v in the strip, w through the thickness)
times an interval of φ. Its image is enclosed by interval arithmetic (forge-math `Interval`,
outward-rounded; sin and cos with their ulp bounds), so the distance over the whole cell, every φ
in it included, is at least d(centre) − half-diagonal of the enclosure. Branch and bound:
- a cell whose lower bound is ≥ the limit is proven clear, over its whole φ interval: no
  sampling gap;
- a witness point (a real part point at a real φ) under the limit is a proven collision, reported
  with its distance, place, φ, and the part face and tool feature named;
- cells are split until one or the other holds or they are smaller than `resolution`
  (limit / 8); the verdict states the certified bound: pass means distance ≥ limit − resolution
  everywhere, through the motion.

Part-to-part uses the exact slab distance (plane offset with 2D distance to the region) the same
way; pieces fixed relative to each other in a step are not paired, adjacent walls of the moving
bend are excluded by an exact half-space argument, and the working contacts (punch tip on the
zone, legs on the shoulders) are decided by the closed-form rules above, not by distance.

## 4. Sequence planning

Depth-first over the set of bends done, deterministic: bends tried outermost first (deepest in the
wall tree), then by name; per bend, orientations (2), then punches (straight, gooseneck, acute,
by id), then dies (V nearest the vendor's rule, 8t by default), then the gauge edge. A step's
result depends only on (bends done, bend, setup), so results are memoised and failed states pruned.
A user sequence (`--sequence 3,1,2` or names) is simulated as given, choosing tooling per step. If
nothing passes: `feasible: false`, and for every bend the blocking checks of its best attempt.

## 5. Output: `forgecad query F bendsim [--vendor V] [--tooling T] [--sequence S]`

```
{ "ok": true, "schema": "forgecad.bendsim/1", "sheet": "tray",
  "tooling": "std-eu", "press": "std-eu-100t",
  "feasible": true, "sequence": ["right_lip", "left_lip", "right", "left", "rear", "tab"],
  "steps": [ { "n": 1, "bend": "right_lip", "angle": 90, "overbend": 2,
      "punch": "std-eu/G88-R08", "die": "std-eu/V12-88",
      "orientation": { "flip": false, "rotate": true, "y": 0 },
      "backgauge": { "x": 42.5, "r": 0, "edge": "@e12" },
      "checks": [ { "rule": "min_flange", "pass": true, "value": 12.0, "limit": 8.4 }, ..,
                  { "rule": "tool_collision", "pass": false, "value": 0.21, "limit": 0.5,
                    "part": "right.top", "tool": "punch:std-eu/P88-R08/body",
                    "at": [x, y, z], "phi": 61.0 } ] } ],
  "blocking": [ { "bend": "rear", "checks": [..] } ],      // only when infeasible
  "fixes": [ {"op":"SetParam","param":"lip","value":{"value":14,"dim":"Length"}} ],
  "suggestions": [ {"kind":"UseTool","bend":"rear","punch":"std-eu/G88-R08"} ] }
```
Errors are the query's typed errors (`NoSheet`, `NoSuchBend` with the bend names, `NoSuchTooling`,
`BadSequence` with the item, `HemNotSimulated`), with spans where they come from the file.

## 6. App

A bend-sim panel: step list, play / step / scrub through φ, the press section (punch, die, ram,
bed, fingers) drawn with the part, collision pairs highlighted, the failing checks listed. Self
contained until the UI session's tokens land.

## Gaps, stated

- Hems are not simulated (they need a pre-bend and a flattening die): `HemNotSimulated`.
- Holes and cuts inside a bend zone are counted as material (conservative).
- One punch and one die per step (no multi-station setups yet).

## As built (notes)

- **Working contacts.** The die lies wholly under each leg's outside-skin half-plane past that
  leg's tangent line, and the punch tip wholly over its inside one, while the angle and radius
  rules hold. Any material there (cell by cell, whatever piece it is) is excluded from those two
  elements; the moving zone is excluded from both. Everything else is checked by distance.
- **Regions.** Every strip's region is its outline (or its zone's slot) within the flat pattern D
  (holes, reliefs, corner cuts), less the zones' slots, with its boundary kept exactly; a zone
  with cuts is bounded by the developed distance stretched by (2/pi) min(1, r_in/rn) below and
  max(1, r_out/rn) above.
- **Prism bound.** Against a tool (a y-prism) a cell's bound is the 2D bound in (x, z) combined
  exactly with the least y distance, so cells are not split along the bed.
- **Measuring.** A decision stops at the limit; a reported value is measured to the resolution or
  a fifth of the margin, and pairs are measured nearest first with the best so far as a cut.
- **Planner caches.** The gauge (per bends done, bend, orientation), the legs and part length,
  ram-side and fixed-side decisions, and every verdict keyed by the done bends on the piece's own
  path to the moving bend. Warnings (withdraw) never decide a setup.
- **Tool lengths.** The longest segment sum within [bend − 2 short_end, bend − 2 clearance], else
  the shortest past the first: a box's side tooling fits between its standing ends.
- **Speed.** The example load tray (6 bends) plans in about 4 s; a 4-bend box with no feasible
  order is refuted in about 5 s.
