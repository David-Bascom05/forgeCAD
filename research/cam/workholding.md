# Workholding as a first-class part of CAM (design note, 2026-11-23)

David: "do we have workholding and all the random things that can entail?" Today: two generic
vises and two soft-jaw blanks (kernel/forge-tooling/data/fixtures.toml); `vise(...)`, a setup's
`fixture:` and `soft_jaws` in forge-cam-lang; forge-cam/src/fixture.rs places a vise as boxes and
the certified checker (G2) keeps every tool body the fixture clearance away; cam/sim-fixes makes
the stock's underside plane an obstacle when no workholding is declared; forge-machine's setup
takes fixture boxes; forge-turn models one three-jaw chuck envelope. Fusion has no workholding
library (libraries-machines-posts-feeds.md §4.2): a user derives vise models by hand.

## 1. Where it lives

A new crate **kernel/forge-workholding**. It depends on forge-tooling (the TOML reader and the
library search order) and forge-cam-geom (the checker's `Face`), and on nothing CAM-specific, so
forge-cam / forge-cam-lang (CAM-CORE), forge-machine (CAM-MACHINE) and forge-turn (CAM-TURN) can
all depend on it without a cycle. forge-tooling keeps `[[vise]]` / `[[jaw_blank]]` as they are
(read by today's `vise(...)`); the new library is `[[workholding]]` records in
`kernel/forge-workholding/data/*.toml`, with real parts under
`kernel/forge-workholding/parts/<item>/*.fg` (§6). Nothing in kernel/forge-bool or the modelling
kernel changes.

## 2. Model

A **device** is an assembly of **bodies**, each a list of primitives (box, cylinder, cone
frustum: what the checker, the machine simulation and the renderer all take exactly) with an
optional real `.fg` part it envelopes, a **role** (base, fixed jaw, moving jaw, parallel, clamp,
step block, stud, plate, riser, dowel, chuck body, jaw, collet, centre, roller) and a
**keep-out** (mm the tool must stay away beyond the fixture clearance: a strap clamp's screw
and nut, a pull stud's top). Two frames per device:

- **mount**: how it sits on the machine: `table` (T-slots: slot width and pitch), `grid`
  (fixture-plate holes, pitch, hole size), `zero_point` (52/96 mm grids, stud size), `chuck_face`
  (lathe spindle nose). The machine definition names its table/chuck mount (CAM-MACHINE's
  `[[machine.mount]] kind = "part"`); a device's mount frame is placed on it.
- **locate**: where the part's datum goes (vise: the fixed jaw's face ∧ the parallels' top;
  dovetail: the dovetail's root; chuck: the jaws' face on the axis).

A **placement** is a device, its parameters set from the stock (vise opening = the stock across
the jaws; clamp height; parallel height; a step block's step from the stock's top; a chuck's
jaw position from the bar's radius), and a rigid pose: device frame → part frame (rotations by a
3×3 orthonormal matrix, so risers / tombstone faces / 5-axis tilts are plain data). A setup's
workholding is a list of placements (several vises, several parts, patterns: a placement with a
`pattern` of offsets). Every posed body comes out as:

- `faces()` → checker faces with stable ids per body (G2 in the certified checker, as
  `certify_fixtured` takes today);
- `prims()` → boxes/cylinders in the part frame (forge-machine's `SetupSpec.fixture`, posed with
  the table through the kinematics);
- `boxes()` / `held(box)` → leftover classification "held by fixture" (G4);
- lathe devices: a half-profile (ρ, z) envelope swept by the spindle, for forge-turn's obstacle.

## 3. Kinds (all data; each a generator of bodies from its record and the stock)

Mill: machine vise (fixed + moving jaw, bed, opening from the stock, clamp height), soft jaws
(today's `soft_jaws` op, bodies = the cut jaws), parallels (pair, height, width), toe / strap
clamps with step blocks and the T-slot stud (DIN 6314-style strap; the clamp's nose on the stock
top, the stud and nut a keep-out), edge clamps (Mitee-Bite style: low body, clamping range),
fixture plates (hole grid, dowels, plate thickness), zero-point (module/receiver, stud, riser),
self-centring and dovetail vises (5-axis: narrow body, small grip on a machined dovetail),
tombstones (2/4 faces, HMC), vacuum tables (seal area), custom (the user's own .fg parts or
assemblies, with an explicit envelope and locate frame).
Lathe: 3/4-jaw chucks (hard/soft jaws, jaw stick-out from the body, grip range), collet chucks
(ER, 5C), face drivers, tailstock centres (60° point, quill), steady rests (rollers at z).

## 4. Checks (typed errors with a fix, as forge-cam's FixtureError)

Opening (stock across the jaws ≤ max opening), clamp height (0 < clamp ≤ min(stock, jaw height)),
parallels fit and clear the cut (a parallel is a body: a cut through the stock's bottom hits it,
G2), dovetail grip present on the stock, edge-clamp range, zero-point studs on the grid,
step-block step from the stock top, vacuum **hold-down rule, stated as a check, not a proof**:
F_hold = Δp · A_seal ≥ SF · F_side (Δp the vacuum below atmosphere, A_seal the area inside the
gasket, F_side the largest tangential cutting force with friction μ: μ · Δp · A ≥ SF · F_side);
inputs are the user's (Δp, μ, SF, F_side) and the result names them; lathe: grip length ≥ the
chuck's minimum, stick-out (length out of the jaws / diameter) above the published guideline is a
warning, never a refusal unless a published limit says so. A clearance / **reach report**: for
each path, the least tool stick-out at which the holder clears every fixture body by the fixture
clearance (closed form over boxes and cylinders), and the body that sets it.

## 5. CAM integration (through the coordinator; owners in brackets)

- Setups reference workholding [CAM-CORE: `Setup.fixture: Option<Fixture>` → a list of
  placements; proposal: forge_cam::fixture::Fixture gains `from_workholding(&Posed)` (pieces from
  boxes, shaped from the other faces), so certify_fixtured is unchanged]. Re-clamping between
  setups: a placement's pose plus the stock transfer (the in-process stock of setup k, turned by
  the new setup's frame) [CAM-CORE, existing `from: SETUP`].
- Every body an obstacle in the certified checker (G2, today's path) and in the machine
  simulation [CAM-MACHINE: SetupSpec.fixture gets cylinders and a body name per prim; posing
  with the table is theirs].
- Leftover "held" (G4): `held(box)` per placement [CAM-CORE, the "held by fixture" reason
  exists for a vise; generalise to any device].
- Lathe [CAM-TURN: `Chuck` stays theirs; proposal: build it from a forge-workholding chuck /
  collet record (body, jaw height = jaw stick-out above the bar, grip) and add the tailstock /
  steady rest as further obstacles of revolution].
- UI [CAM-UI: the setup dialog's workholding tab lists the library]; drawing [CAM-SIM: prims
  and the parts].

## 6. Library

Real products first, the machine vendors' own workholding first (data/vendors.toml: Haas,
Tormach, Carbide 3D, DATRON, Sherline, Grizzly, Mazak, DN Solutions; then Kurt), then the
independents (data/products.toml, data/lathe.toml), each value cited with source and date; a
value not published is UNKNOWN (`unknown = [..]`), never filled in. Stand-ins last
(data/standins.toml, `standin = true`): only for a class no maker publishes enough of, each with
its `basis` (which published values it follows, which numbers are ours).

A body whose size is unpublished: for a vise (jaw thickness, height, the sizes no maker gives)
a conservative envelope from the published overall sizes (the union over every jaw thickness
the body allows, 0 ≤ t ≤ body length − opening; the base 1 m below the bed without a height),
said in the notes; elsewhere refused with a typed error naming the field and the fix (measure
it: `Record::with`, or `forgecad workholding new NAME --from X field=value`, provenance user). Search order as the machine
library: project `cam/workholding/*.toml`, then the user's, then built-in (a name in an upper
level hides a lower one; twice in one level is an error). Real parts under `parts/`: each built
by forge-lang in a test and checked to lie inside its envelope.

## 7. Tests

Each library item parses and validates; each generic body's part inside its envelope; sample
jobs through forge-cam's certify_fixtured: a vise on parallels where a cut too deep hits a
parallel (and one that clears), a toe-clamp job where the path that avoids the clamp is clear and
one through it is not, a dovetail job held for five-sided access (only the dovetail's grip is
held; each side's tool axis clears the vise), a lathe chuck job whose turned length needs a jaw
stick-out the chuck allows (and one it does not); vacuum rule both ways; native and wasm32.

## 8. Field schema (for MACHINE-BUILDER's library editor)

`forge_workholding::schema::schema()` (and `forgecad workholding schema [--json]`,
`forgecad.workholding-schema/1`) gives every kind: its name, what it is, lathe or mill, and its
fields: name, unit (mm, N, kPa, deg, rpm, N·m, count, mm[]), required or optional, meaning (one
meaning per field name across kinds). A record's provenance is one of: cited (a maker's values,
each `[[workholding.cite]]` with fields, source, date read), standin (with its basis), user
(`source = "user"`; no date needed). Validation is `Record::from_table` (typed errors: missing,
not published, bad value, a field the kind does not have, an uncited product value);
`Record::to_toml` writes a record that reads back equal; `schema::new_from(lib, NAME, FROM,
given)` makes a user record from any other. Posing preview data: `Workholding::prims()` (named
blocks and cylinders in the part frame) and, for the lathe, `lathe::Spin.rings` ((ρ, z)
polygons).

## 9. Generated workholding (a part you machine, with its own setup)

generate.rs: a fixture plate from the part's through holes (round pin and diamond pin in the two
holes farthest apart, toe clamps at the long sides' middles, the plate as its own `.fg`); step
jaws (soft jaws with ledges the stock seats on); lathe soft jaws bored to the part (the bore
region in (ρ, z) for CAM-TURN's boring op, the wall check). forge-cam's soft_jaws (CAM-CORE)
stays the vise soft-jaw pocket op. Contoured 3D jaws wait for CAM-3AXIS's drop-cutter.
