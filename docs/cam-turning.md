# Turning (`forgecad cam`, lathes)

A turning document is a machining document whose setup is a `lathe_setup(..)`. `forgecad cam F`
plans it, posts it, reads the text back, and certifies the path the text means; the reply is
`forgecad.cam/1` with `"process": "turning"`. The program is written (`-o F.nc`) only when every
op is certified. Exit 0: certified in full; 3: certified except G5 (no machine, or not placed);
1: refused. `forgecad cam F check` also keeps the exact in-process stock and lists the G4
leftover; the normal path keeps the stock as columns (a superset by construction: sound for the
rapid and holder checks, and about four times faster).

```
machining
shared bar = 32
f     = part("shaft.fg")                       # a part of revolution about its z axis
t1    = lathe_tool("od-rough-cnmg", t: 1)      # the built-in library, by id; t: the station
t2    = lathe_tool("od-finish-dnmg", t: 2)
jaws  = chuck(face: -70, grip: 30, jaw: 15)    # the jaws' face (z), grip length, jaw height
op1   = lathe_setup(f, bar: bar, front: 2, chuck: jaws, machine: "generic-lathe",
                    at: 100, offsets: ((1, 0, 0), (2, 0, 0)))
face  = turn_face(op1, tool: t1, to: 0, step: 1)
rough = turn_rough(op1, tool: t1, leave: 0.3, step: 2, to: -56)
fin   = turn_finish(op1, tool: t2, to: -55)
prog  = lathe_program(op1, post: "fanuc-lathe", number: 1)
```

The whole example is examples/machined/shaft_turn.fg (with a thread and a part-off); inside work
(drill, bore, a thread relief groove, an M30 x 1.5 internal thread) is examples/machined/nut_turn.fg;
a left-hand finish up a back chamfer is examples/machined/collar_turn.fg.

## The part

Z is the spindle axis, Z0 wherever the part puts it (the example: on the front face, the part in
z ≤ 0). The part must be a solid of revolution about z: planes across the axis, cylinders, cones,
spheres and tori about it. Any other face (a flat, a cross hole) is refused by name.

## Built-ins

| call | what |
|---|---|
| `part("F.fg")` | the part, lowered to its half-profile in (ρ, z) |
| `lathe_tool("ID", t: N)` | a tool of the built-in library (kernel/forge-turn/data/turning.toml) at station N |
| `lathe_tool(holder: "H", insert: "I", t: N)` | a holder or boring bar and an insert from the tooling library (forge-tooling's makers' records, and the user's and the project's where the caller passes them: `forge_turn_lang::run_with_library`), assembled by their ISO codes |
| `chuck(face:, grip:, jaw:, body_r: 80, body_len: 60)` | the three-jaw chuck's envelope as it turns |
| `lathe_setup(PART, bar: D, front: Z, chuck:, machine:, at:, offsets:, tol: 0.002, clearance: 1, holder_clearance: 0.25, rapid_clearance: 0.2)` | the bar (diameter D, front face at Z); `machine:` a lathe by name; `at:` (the machine Z of the part's Z0) and `offsets:` ((station, x, z), …: each tool's geometry offsets) place the setup, and G5 is then checked on every joint position |
| `turn_face(op, tool:, to: Z, step:)` | face from the bar's front to Z, the nose centre to the centre line |
| `turn_rough(op, tool:, leave:, step:, to: Z, top:)` | axial passes at every `step` and at every diameter of the part, down to the part grown by the nose radius and `leave`, or to Z |
| `turn_finish(op, tool:, leave: 0, to: Z)` | the profile, the nose centre on the exact offset of the part by the nose radius (compensation done exactly; posted as the imaginary tip with G40) |
| `turn_rough(op, tool: LEFT, leave:, step:, from: Z0, to: Z1)` | with a left-hand tool: axial passes up (+z) from Z0 (room already cut) to the part grown by the nose and leave, or Z1 |
| `turn_finish(op, tool: LEFT, leave: 0, from: Z0, to: Z1)` | with a left-hand tool: up the profile (+z) from Z0 to Z1, for the flanks that face the chuck (a collar's back chamfer, reached from a neck cut first) |
| `bore_rough(op, tool:, leave:, step:, to: Z, from: D)` | boring: the drilled hole (diameter D) opened outward in axial passes down to the bore grown by the nose radius and `leave`, or to Z; `turn_finish` with a boring tool then finishes the bore |
| `turn_groove(op, tool:, from: Z0, to: Z1, r: R)` | a groove between Z0 and Z1 to radius R, by plunges; with an ID grooving bar also `bore: D` (the smallest diameter on the way in): an internal groove, the blade entering along the axis and plunging outward |
| `turn_thread(op, tool:, d:, pitch:, from:, to:, passes: 8)` | an ISO metric thread (60°), radial infeed of equal chip area; the thread is a declared removal. External with a thread insert (the part shows the major diameter); internal with an ID thread insert (the part shows the minor diameter, d − 5H/4, H = p·√3/2) |
| `part_off(op, tool:, at: Z)` | the blade's side on the part's back face at Z, through the centre |
| `turn_drill(op, tool:, to: Z, peck: 0)` | a drill on the centre line |
| `lathe_program(op, post:, number:)` | posts and certifies the setup's ops: `fanuc-lathe`, `haas-lathe`, `linuxcnc-lathe` |

The built-in tools: `od-rough-cnmg` (CNMG120408 in PCLNR2525M12), `od-finish-dnmg` (DNMG150404 in
PDJNR2525M15), `od-profile-vbmt` (VBMT160404 in SVJBR2525M16), `groove-3`, `part-3`,
`thread-60`, `drill-8`, `drill-26`, and for bores `id-bore-ccmt` (CCMT09T304 in S20R-SCLCR09: a Ø20 bar; its body is checked as the cylinder it is), `id-groove-2` (a 2 mm blade reaching 3 above a Ø16 bar) and `id-thread-60` (a 60° insert, pitch 0.5–2, on a Ø16 bar); left hand, `od-profile-vbmt-left` (VBMT160404 in SVJBL2525M16: an L holder is its R twin mirrored across z, tip number 4). Inserts (ISO 1832: C D V T S W R) and holders and bars (ISO 5608, 6261; right or left hand) are decoded from their ISO codes; holder heads are
generic: verify with the vendor. The built-in machine: `generic-lathe` (CAM-MACHINE's generic
2-axis lathe; placeholders, not a real machine).

## What is certified

- G1: the insert keeps the op's leave from the part (less tol); a thread keeps off the part less
  its declared band.
- G2: every body of the tool keeps the fixture clearance from the chuck.
- G3: every body keeps the rapid clearance from the in-process stock on a rapid; the holder keeps
  the holder clearance from it on every cut.
- G5: with a machine, the turret station, the spindle's range (CSS under the G50 clamp), and the
  joints (placed) or the spans (not placed).
- G6: the posted text reads back as the program; the path it means is the one checked.
- G4: the leftover, listed with a reason: "nose radius r" (an inside corner no disc of radius r
  reaches, decided to within 2·tol: every piece of its boundary is r − 2·tol from where such a disc's
  centre can be; r the smallest finishing nose or grooving blade corner that holds it) or "uncut"
  (a band left by a leave among them).

The tool is checked as a solid, below centre height too (the insert's body falls away by its
clearance angle, and its points off the centre plane are further from the axis). Why a check in
the (ρ, z) plane is a check in space: verify/forge-cam/ForgeCam/Turn.lean.
