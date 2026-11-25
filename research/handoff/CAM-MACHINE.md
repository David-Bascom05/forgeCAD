# Handoff: CAM-MACHINE (forge-machine: simulation, collision, controls, timing)

Wound down 2026-11-25. Branch: `machines/real-collision` (on main c135b274, with
`cam/workholding-haas` merged in for the shared compiled-face pipeline).

## What landed on this branch

### A. Drawn = checked at the part mount (done)
- CAM-SIM found the VF-2's table drawn 1 mm under the part mount's datum. The cause was
  systemic: machine parts were generated as their envelope shrunk by its margin on every face,
  the mounting face too. 177 table/bed/chuck parts plus the VF-2's now meet their datum exactly
  (each moved onto it along the mount's direction; its bottom stays inside the envelope).
- `forge-machine-lang/tests/datum.rs` checks every built-in part mount against its real part's
  top, exactly. `check_parts` returns each part's bounds.
- Named exceptions: 17 horizontal machining centres (`HMC_PALLET_DATUM_INSIDE`). Their records
  draw the pallet as a vertical plate (face at y 0) with the part mount at its centre along +z
  and a −z spindle: the datum is inside the pallet. A record-modelling fault for the records'
  owner; the test fails when one is fixed, so the list shrinks.

### B. The collision check on the real geometry (done, behind a switch)
- `forge_machine::mesh::Tree`: one box per triangle of a body's compiled mesh, grown by the
  chord and the body's rounding (the largest mu of its compiled faces), under a BVH; points on
  each connected piece; the mesh itself for an inside test.
- Built generically from any B-rep by `forge_machine_lang::real_tree(body, chord)` through
  WORKHOLDING's pipeline (`forge_workholding_builder::compile::compile_solid`, factored out of
  `compile` unchanged). `real_trees(def, dir)` compiles every `.fg` body of a machine once per
  content hash (built-ins from the embedded parts with `dir = None`). Nothing is keyed to a maker.
- `collide::check_with(s, tl, work, &trees)` checks those bodies by their tree; plain
  `collide::check` is unchanged (the old path: the one-train switch).
- Soundness, as built:
  - Clear is proven: every point of a body's surface lies in one of its boxes (the box holds
    its triangle; the surface lies within the chord of the mesh), and the posed-interval
    box-to-box gap is collide's own.
  - Nesting (one body wholly inside another touches no surface): once per segment, at its start,
    each piece of a tree body, or some primitive centre of a primitive body, must be outside the
    other by `Tree::inside` (ray parity on the mesh, answered only away from every box); else
    that segment uses the envelopes. With the surfaces apart over the segment that holds for all
    of it; a segment whose surfaces meet is not clear anyway.
  - With real geometry the check never claims a Hit from an envelope's witness: a contact is
    Near (possible) until an exact check confirms it (item 2 below).
  - Moves after a skip move use the envelopes (their own span logic).
- Assumptions to keep in view: forge-mesh's tessellation of a valid solid is closed and within
  the chord of the surface both ways (WORKHOLDING's compile relies on the same).
- Tests (`forge-machine-lang/tests/real_collision.rs`): the bound (points on the exact surface
  of every VF-2 and UMC-750 body and of a model outside the catalogue, all in the boxes); a graze
  at half the clearance still reported; the gain (a 6 mm tool 5 mm down the VF-2 table's T-slot:
  the envelope says contact, the real table proves it clear); the time. The differential: every
  vendor job (`tests/vendor_jobs.rs`) is clean by the envelopes and by the real geometry.
  `the_real_check_s_time_on_the_umc`: a grid and a 3+2 pass on the UMC-750, no contact the
  envelopes did not have. (A CAM document cannot name a Haas machine on main yet: forge-cam-lang's
  `machine()` knows the generic ones; CAM-SIM's branch adds them.)
- Speed (same VM, release): VF-2 rapid grid, 161 segments: 0.058 s real against 0.059 s by the
  envelopes; UMC-750 grid and 3+2 pass, 126 segments: 0.067 s against 0.094 s. Compiling the
  trees is once per part content per process (not in these times).
- Display-only bodies (`check = false`) stay out even with a tree: the Haas enclosure's part is a
  solid block (checking it calls everything inside a collision), the way covers deform. They
  need real models (a hollow shell; covers per pose) before the record checks them.

### C. Item 5, minimal (done)
- `forge_cam_lang::Cam::machine_run` now checks the stock as given (a superset of the
  in-process stock) against the machine and the tool's holder and shank, at no clearance,
  never against the cutter (`CBody::superset`: never claimed a Hit), and the machine's bodies
  by their real geometry (`check_with`).

## Not done (designs)
- **2. Exact confirm** (with EXACT-CORE): a Near between two trees comes down to leaf box pairs;
  each leaf is a triangle (or a compiled exact face) under a posed interval. Confirm by the
  certified checker's face-to-face distance (forge_cam_geom check) at the span's poses: proven
  Clear (raise to Clear), proven Hit (with a witness on the real faces), else Near.
- **3. The tool as its revolved profile**: `tool_bodies` stacks rmax cylinders. Take the
  certified checker's revolved profile (forge_cam's tool shape) as a tree of its revolution's
  faces (`real_tree` on its B-rep), or as cones/cylinders per profile segment.
- **5, full**: the final part as a body (its B-rep through `real_tree`, in the setup's frame;
  a contact with it is real: the part is always inside the stock), and the in-process stock
  from the certified checker's conservative stock (forge_cam's InProcess: a box minus 2.5D
  voids) as boxes, in place of the stock as given. Z-map markers then display-only.
- **Workholding bodies**: WORKHOLDING's `Exact{model, group, pose}` faces (cam/workholding-any2:
  `forge_cam::fixture::body_source`) through the same tree (each face's bbox grown by its mu),
  as `Role::Fixture` bodies in `machine_run`.
- **CAM-SIM's ratchet** (`what_is_drawn_against_what_is_checked`, GAPS_AT_MOST 1170, on
  cam/sim-vise): agreed with CAM-SIM to make it the one test: `checked` = Part where `real_trees`
  returns the body; lower the constant in the commit that wires it.
- **HMC records** (above) and the Haas enclosure / way cover models.

## Other open items (from earlier work)
- cam/three-plus-two @8e829b2c: the 3+2 orientation solver, parked (3+2 cancelled).
- The U5-620's TRAORI arcs at ~40 % of F (research/cam/timing/rptu-u5-620): unexplained.
- Block cycle and look-ahead depth (`machine.block_cycle`, `machine.lookahead`) are UNKNOWN on
  every record; LinuxCNC's per-segment limit has no verified citation yet.

## Risks
- The real check is only as good as the parts: a `.fg` stand-in that is smaller than the real
  machine (a cored casting drawn solid is fine; a part drawn short is not) makes a contact
  missed that the envelope caught. tests/parts.rs keeps every part inside its envelope, not the
  envelope's fidelity to the machine.
- The inside test's ray parity assumes a closed mesh; a part with an open or self-intersecting
  mesh falls back to the envelopes only where a probe lands near a box, not everywhere.
