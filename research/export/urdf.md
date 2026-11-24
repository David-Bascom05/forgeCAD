# URDF export (`forgecad export F urdf`)

A part or an assembly as a URDF robot description: one ZIP (the reply's bytes) holding
`NAME/NAME.urdf` and `NAME/meshes/*.stl`; the CLI unpacks it into `NAME_urdf/` (or writes the
ZIP as is with `-o X.zip`). Code: `interface/forge-urdf` (no kernel code changes; forge-lang keeps only the assembly's `kin` hook and the `robot` document kind's words, so its own code generation, which the speed gate counts, is unchanged).

## Links
- A part is one link, named for the file; its frame is the part's origin.
- An assembly: one link per **rigid group** of top-level instances, grouped as forge-asm's
  plan groups them (union over `fastened` joints and `lock` constraints), and every grounded
  instance in one group, the root (ground is one rigid body). Subassemblies are rigid in
  forge-asm v1, so a subassembly instance is one body and its internal joints are fixed at their
  solved values (the reply's notes list them).
- No ground: the root is the group of the first instance in document order (said in the notes).
- The root link's frame is the assembly's frame. Every other link's frame is the anchor frame
  of its parent joint on its own side (below), so joint values are forge's values.
- Reference instances (`reference: true`) stay in the kinematic graph but give no visual,
  collision or mass.

## Visual, collision, inertial
- One binary STL per part file (`forge_mesh` tessellation, `meshfile` defaults: tol 0.01 mm,
  15 deg), in the part's own coordinates in **mm**, with `scale="0.001 0.001 0.001"` in the URDF.
  Each occurrence in a link is a `<visual>` with origin `link⁻¹ ∘ pose`.
- Collision: the visual mesh itself (not a convex hull, not decomposed). Follow-up: convex hull
  or V-HACD-like decomposition as an option.
- Inertial: each solid's exact `MassProps` (unit density, `forge_topo::mass`) × its material's
  density (`forge_material`, kg/m³), composed in the link frame by `props::combine` (parallel
  axis, rotation `R I Rᵀ`). Written: mass kg, `<origin>` at the centre of mass with rpy 0, the
  tensor about the centre of mass in link axes, kg m² (mm² → m²: × 1e-6). URDF's ixy is the
  tensor entry (−∫xy dm), the same sign mass.rs gives.
- A solid without a material: refused (`NoMaterial`, the solids listed). No density is assumed.
- Error bounds: each solid's `MassProps::err` (volume, centroid, inertia entry), scaled to kg / m
  / kg m², reported per link in the reply. The composition's own rounding is not bounded
  (UNKNOWN; the f64 sums of a few terms).

## Joints
URDF value `q` = forge's value: angles rad, lengths m (forge mm / 1000). A forge joint holds when
`B⁻¹ ∘ A' = M(values)` (A' = A's anchor, flipped if `flip`). Tree edge with B on the parent
side: joint origin = B's anchor frame in the parent link, child link frame = A'. With A on the
parent side, `A'⁻¹ ∘ B = M⁻¹`: origin = A' in the parent link, child frame = B, and the
elementary motions are taken in reverse order with each axis negated (M is a product of single
axis motions, so M⁻¹ is too).

| forge        | M                 | URDF                                                     |
|--------------|-------------------|----------------------------------------------------------|
| fastened     | I                 | fixed (inside a group: no joint at all)                  |
| revolute     | Rz(θ)             | revolute if limited, else continuous                     |
| slider       | T(0,0,d)          | prismatic                                                |
| cylindrical  | T(0,0,d) Rz(θ)    | prismatic z → dummy link → revolute/continuous z          |
| pin_slot     | T(d,0,0) Rz(θ)    | prismatic x → dummy → revolute/continuous z               |
| planar       | T(u,v,0) Rz(θ)    | no limits: URDF `planar` (axis z); limited: prismatic x → prismatic y → revolute z |
| ball         | any R, origins on | continuous z → y → x through two dummies (R = Rz Ry Rx; gimbal lock at y = ±90°) |

- Dummy links: `J__d1`, `J__d2`, no `<inertial>` (massless). Kinematically exact; physics
  engines that drop massless links (Gazebo classic) need SDF/MJCF (follow-ups, which have ball
  and universal joints natively).
- URDF prismatic needs limits. A translation with no forge limit is refused
  (`UnlimitedTranslation`, joint and freedom named) unless `travel MM` is given (±MM, the
  user's choice, listed in the notes).
- `<limit effort velocity>` are required by URDF and forgeCAD has no actuator data: they are
  given by `effort N velocity V` (all joints) or `effort J N velocity J V` (one joint, by its
  forge or URDF name). A moving joint with neither is refused (`MissingLimits`, the joints named):
  0 is not a known value, and planners read velocity 0 as "cannot move". `placeholder-limits`
  (opt-in) writes 0 with a `PLACEHOLDER` comment on each such joint and a warning.
- Ball limits have no URDF form: refused (`BallLimits`).
- A driven joint (`turn: open`) is a joint like any other; its drive is the export pose.
- Relations → `<mimic joint multiplier offset>` on the follower's URDF joint: multiplier =
  slope × unit(follower) / unit(leader), offset = (solved follower − slope × solved leader) in
  the follower's unit. A relation on a ball or a native planar's value: refused.
- Loose constraints between two groups have no URDF form: refused (`LooseConstraint`) unless
  cut. Inside one group they are implied or already errors of the solve.
- Groups with no path to the root: a `floating` joint from the root at the solved pose (they are
  free bodies), listed in the notes.
- The reply's `positions` give every URDF joint's value at the solved pose (a ball's as its ZYX
  angles), so the export pose can be reproduced.

## Trees only
The group graph (edges: the non-fastened joints and loose constraints, ground merged) must be a
tree. The first edge in document order closing a cycle is refused as `ClosedLoop` with the loop's
joints named, and each as a fix `cut J`. With `cut J` (repeatable, opt-in) that joint is left out;
the URDF carries a comment and the reply a `cut` list. A relation whose joint is cut is refused.
Nothing is dropped silently.

## Names and determinism
Names: `[A-Za-z0-9_]`, anything else `_`, a leading digit prefixed `_`, collisions numbered
`_2`, `_3`. Numbers are Rust's shortest round-trip f64 text (identical native and wasm32), `-0`
written `0`; rpy from `forge_math::dmath` (atan2/asin, no libm). Every order is document order.

## Follow-ups
SDF (native ball, universal, gearbox; nested models for loops), MJCF (equality constraints close
loops: the honest answer for closed chains), collision convex hulls, `<transmission>` from drives.
