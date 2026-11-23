# forge-asm audit: case-patching, refusals, exact cases, interference, DOF

Read-only audit of `/home/user/forgeCAD/kernel/forge-asm` (HEAD of the working tree, 2026-11-22). No
file under kernel/ or interface/ was changed. Line numbers are for the current files.

## 0. Short answer

forge-asm is **much cleaner than the boolean**. The structure is principled: name lookup, then
union-find rigid clusters, then partitions, then Tarjan biconnected blocks. Tree edges are placed in
closed form, and only loop blocks go to Newton. There are no solver restarts from perturbed starts,
no damping ladders, and no "solver A then solver B" fallback.

The patching that is there is one pattern: **one fact ("is this constraint met / redundant / at its
limit / independent") is decided at several sites, each with its own epsilon**. None of these
epsilons is the kernel's one tolerance (`LINEAR = 1e-5`, forge-math/src/tol.rs:12). There is a
second, smaller pattern: **the conflict diagnosis re-solves by a different procedure** than the solve
it explains.

- The test suite passes: 42/42, 0 ignored.
- There is no assembly failure corpus. `tools/proof/known.txt` and `fuzz_known.txt` have no asm
  entries.
- The crate has a single squashed commit, so its history cannot be read.

The ranking below is by how much latent, wrong-answer risk each item carries, because no observed
failures exist to rank by.

## 1. Case-patching, ranked

### R1. "Satisfied / redundant vs conflicting" decided at six sites with five thresholds
| site | test | threshold |
|---|---|---|
| plan.rs:14, 549-564 `closed_form` | `gap <= SAME` -> Redundant, else Conflict | `SAME = 1e-7` (absolute mm; rad x plan.size) |
| plan.rs:374 (closing fastened cycles), 385 (extra grounds) | `joint::frame_gap` (origin distance, axis-difference x size; joint.rs:248) | SAME |
| solve.rs:1636-1682 (joints inside a rigid cluster / ground-ground) | `Ends::gap` (row residuals, angular x plan.size; joint.rs:188), **a different metric from frame_gap** | SAME |
| solve.rs:528-529 (relation follower already known) | `|had - v| * scale` | SAME |
| solve.rs:1042-1048, 1085, 1090 Newton | worst row `<= TIGHT*size` (1e-13), or stuck/capped and `<= SOLVED*size` (1e-9) -> Solved, else Stuck = conflict | 1e-13 and 1e-9, scaled by the block size |
| solve.rs:1215-1237 diagnosis | each subset re-solved and judged by the same Status | as above |

- SAME is 100x smaller than LINEAR. SOLVED x size is 1e-9 to 5e-4 depending on `size`.
- One loop that closes to 5e-8 mm is "Solved" in a loop block. The same 5e-8 mm misfit on a
  fastened cycle is "Redundant". At 2e-7 the fastened cycle is a "Conflict", while a loop block with
  size > 200 still calls 2e-7 "Solved".
- The kernel would call both of these the same point.

### R2. Rank and redundancy decided three or four times with different epsilons and different orders
- Newton step: `Qr::new(jt, n, m, 1e-12)` (solve.rs:1058).
- Reported rank and mobility: `Qr::new(jt, n, m, 1e-9)` (solve.rs:1348). Column-pivoted
  Householder. The threshold is relative to the largest *initial* row norm of J (qr.rs:213-214,
  231), not to σ_max, and has no absolute floor.
- Which rows are Redundant: a separate modified Gram-Schmidt in blame order, "twice for safety",
  with `norm > 1e-8 * norm0` (solve.rs:1462-1486). This is relative to each row's own norm, not to
  ‖J‖.
- `implied_by` coefficients: `|c| > 1e-9 * cmax.max(1.0)` (solve.rs:1503). This comes from a third
  factorization (`linalg::solve_least_squares`).
- Per-joint `free`: `projected_rank` via `qr::rank` with 1e-9 relative (solve.rs:1443-1455,
  qr.rs:322-324).
- Redundancy is reported only if the QR says `rank < m` **and** status is Solved (solve.rs:1350).
  The MGS can then find a different number of dependent rows than `m - rank`. Nothing checks that
  `Σ redundant rows == m - rank`.
- The Newton step uses a rank (1e-12) that differs from the reported one (1e-9). Near a toggle the
  step therefore inverts rows that the report calls dependent. The 16-halving line search
  (solve.rs:1066-1083) absorbs this.
- **Real defect, reproduced** with a standalone copy of qr.rs
  (scratchpad/qrx/main.rs). Because the threshold is purely relative, a matrix of pure rounding
  noise has full rank: `rank([1e-17,-3e-17],1,2) = 1` and the 3x2 noise case gives 2. Only exact
  zero gives 0.
  - In `projected_rank` the input is a joint's gradient projected on the null space. For a joint the
    mechanism cannot move, that projection is rounding noise. So `JointReport.free` can come out 1
    instead of 0. This happens, for example, for a joint in a block whose only mobility is a
    ball-ball idle spin, or at a dead centre.
  - Fix: use a tolerance relative to ‖J‖ (or σ_max of the unprojected gradients), not to the
    projected matrix itself.

### R3. Not invariant under a rigid motion of the whole document
- `plan.size` is the max |coordinate| of any written pose or anchor origin (plan.rs:259-267).
- The block `size` is the max of the anchors and the *world* pose origins, clamped to plan.size
  (solve.rs:1122-1154).
- `size` scales all of these:
  - the angular rows (solve.rs:909) and the Jacobian ω columns (1020);
  - TIGHT and SOLVED (1042-1043);
  - the ramp unit `0.1*size` (1184);
  - SAME's angular term (plan.rs:374, solve.rs:528, 1637).
- So translating an assembly 1 m away from the origin changes the convergence thresholds and the
  row scaling seen by the relative rank tests. A near-threshold redundant/conflict or rank decision
  can flip. This is the analogue of the boolean's mirror finding.
- Fix: measure size from the block's own extent (anchors relative to a block centroid).

### R4. The diagnosis re-solves by a different procedure than the solve
- The main solve ramps drive and limit targets in up to 256 steps ("so the loop stays on the written
  branch", solve.rs:1172-1205).
- The diagnosis `solves` closure (solve.rs:1215-1219) restarts every subset from `begin` with **no
  ramp**. So does the final re-solve (1262).
- So a set that is solvable only through the ramp can come out Stuck in diagnosis, and be named a
  Conflict. Or the diagnosis can close on the other branch. That is a second reading of "can these
  be met", made with a different algorithm.
- The deletion filter (QuickXplain-style, 1226-1235) is principled, but each membership test is a
  full Newton run judged by the R1 thresholds.

### R5. Limits: two tolerances for one fact, and a bounded retry
- Tree edge: slack `1e-9` rad for angles, `LINEAR` (1e-5 mm) for lengths, then clamp
  (solve.rs:751-761).
- Loop: `v < lo - 1e-9 || v > hi + 1e-9` for **both** lengths and angles (solve.rs:1295).
- So a slider 5e-6 mm past its limit is "at the limit" in a tree and "violated, re-solve" in a loop.
- The loop's hold-at-limit runs up to 4 rounds (solve.rs:1276). On failure it rolls back and reports
  every value added in that round as LimitViolated, which is not a minimal set (1319-1333).

### R6. Formulations singular at particular configurations (documented, not refused)
- `Angle{angle}` uses the row `za·zb - cos(angle)` (joint.rs:169). Its gradient vanishes at 0 and π.
- `Distance{0}`: the gradient direction is arbitrary, falling back to `Vec3::X` (joint.rs:173).
- model.rs:457-462 says "ill-conditioned near 0 and π (use Parallel)" and "near 0 (use Coincident)",
  but nothing refuses these values or lints them.
- At the singular configurations, Newton converges only linearly and stops Stuck. It is accepted
  because `e <= SOLVED*size`. For the angle row `e ≈ δ²/2`, so a residual of 1e-9 x size accepts an
  angular error δ ≈ 4.5e-5·√size rad.
- The constraint's row then has zero rank, so it is reported Redundant with an empty `implied_by`,
  and mobility is over-counted by one.

### R7. Smaller items
- `same_frame` bitwise equality decides whether loose constraints are "recognised" as a joint for
  closed-form placement (solve.rs:186-244). Otherwise they take the numeric path. These are two
  evaluators of one definition; the test property.rs:276 checks that they agree. This is an
  exact-match shortcut, which is acceptable.
- A tree freedom with no known value is silently taken as `0.0` (solve.rs:746-748). It is probably
  unreachable, but it is a quiet default, not a refusal.
- `Redundant{rows:0}` is mapped to `usize::MAX/2` in the per-constraint report (solve.rs:1691). This
  is an encoding hack.
- The `signs` guard that refuses steps onto the other branch (solve.rs:976-1010, 1070-1074) is
  legitimate: dot rows cannot see the sign.

## 2. Honest refusals (typed diagnostics, diag.rs:77-190)
- Structural: BrokenAnchor (with AnchorReason NoInstance, Suppressed, BadPath, NoDocument, NoAnchor,
  unresolved), WrongReference, SelfJoint, DuplicateInstance, DuplicateJoint, DuplicateConstraint,
  BadFreedom, BadRelation, CyclicAssembly, NoGround, PartChanged.
- Numeric: Conflict (a minimal set with residuals), NotConverged (an iteration cap of 60,
  solve.rs:12, with no claim of conflict), LimitViolated (with a SetDrive fix when a drive caused it).
- Checks: an Interference with `confirmed: false` and `boolean_failed: Some(reason)` when forge-bool
  fails (check.rs:902-919, 1485-1497). Occurrences with no mesh are listed `unchecked`
  (check.rs:1310). `ExactFail::{NoSolid, Boolean}`.
- STEP: `StepError::{NoBody, Part, NoAssembly}` (step.rs:14-25).

## 3. Legitimate exact special cases
- Tree edges are placed in closed form: `World(A) = World(B)∘M(values)` (solve.rs:724-828,
  joint.rs:45-57).
- Rigid clusters are made by union-find, and fastened cycles are checked in closed form
  (plan.rs:269-390).
- Loose-constraint bundles that exactly equal a joint are recognised and placed as that joint
  (solve.rs:196-244).
- Relations propagate in closed form when their leader is known (solve.rs:507-539).
- The angle row `xA·(cosθ yB − sinθ xB)` is smooth and zero exactly at the target (joint.rs:236-244).
- Unchanged partitions and units are reused when their inputs are bit-identical (solve.rs:653-668).
- STEP assembly export is a straight structural writer. Its only fragility is the textual
  renumbering, where a missing PRODUCT_DEFINITION silently gives id 0 (step.rs:68-110).

## 4. How interference is decided
There are three stages, each with its own threshold.
1. **Broad phase:** a BVH over occurrence boxes (check.rs:1329-1342).
2. **Narrow phase on meshes** (check.rs:550-614), with `depth = tolA + tolB + LINEAR`
   (check.rs:1389). The interface uses a mesh chord of 0.01, so depth ≈ 0.02 mm
   (interface/forge-lang/src/asm.rs:3175-3187).
   - Cross: an edge of one triangle pierces the other by more than `depth` and is farther than
     `depth` from a "sharp" edge. An edge is sharp if it is open or turns more than **20°**, a magic
     number (check.rs:68-89, 297-323).
   - Otherwise, an inside test by ray parity on a fixed "no-axis" ray direction (check.rs:525).
     Samples are one per connected piece of far triangles, plus each near triangle's centroid.
     `None` (too close to call, or the ray hits an edge exactly, check.rs:521, 540) moves on to the
     next sample. That is a sampling fallback.
3. **Exact:** forge-bool `Op::Intersect`, then mass volume (check.rs:902-919). It runs only for mesh
   hits that are not covered by a fit or rule (check.rs:1443-1448).
   **Touch iff `volume <= TOUCH_VOLUME = 1e-6 mm³`** (check.rs:1256, 1486). This is an absolute
   volume, not tied to LINEAR or to depth.

"Touch vs interfere" is therefore **not decided once**:
- A length test on the mesh (depth ≈ 0.02 mm) decides whether the exact check is even asked.
- A volume test (1e-6 mm³) then decides touch.
- Face-to-face contact never reaches stage 3. It is Clear, so it is reported neither as touching nor
  as interfering (the test checks.rs:93/115 asserts only "not interference").
- `touching` is only populated by mesh false positives that the exact check deflates.
- Overlaps shallower than about 0.02 mm are silently Clear. This is documented per pair as
  `tolerance`.

Further inconsistencies:
- Fit-covered pairs are never confirmed exactly (check.rs:1446, 1483-1484).
- Clearance reports `tolerance = tolA + tolB` **without** LINEAR (check.rs:1563), while interference
  uses `+ LINEAR` (check.rs:1492).
- The fit-rule `coaxial` test uses an absolute `cross <= 1e-9` (not ANGULAR = 1e-11, not SAME) and
  the mesh tolerance for a fact about exact anchor frames (check.rs:1083-1086, 1430-1432).
- Feature-size matching uses an absolute 1e-9 (check.rs:1071).
- The oracle test's doc comment says "never misses an overlap deeper than its tolerance"
  (checks.rs:2-3). The test actually asserts it never misses `vol > 0.01 mm³` (checks.rs:388-394).

Principled replacement: decide contact once, from the exact boolean's own tolerant classification.
- Coincident faces are touch.
- Any regular, nonzero-volume intersection is interference, with the volume scaled by LINEAR
  (e.g. `vol <= LINEAR * area_of_contact`, or a regularised intersection having no 3-cells).
- Use the mesh only as a *conservative* filter: grow the depth so it can never be Clear when the
  exact solids share volume, and send near-contact to the exact check rather than calling it Clear.

## 5. How DOF and redundancy are done, and what would be principled
**Current method:**
- The structural Grübler count is reported "as a hint only" (solve.rs:67, 1425).
- The numeric mobility is `n − rank(J)` at the solution, by column-pivoted QR with a 1e-9 relative
  threshold.
- Redundant rows come from a separate MGS (1e-8). Per-joint freedom comes from the rank of the
  gradients projected on the QR null space.
- The tests check against an FD oracle with its own threshold of 1e-6 (tests/common/mod.rs:296-305)
  and against textbook mechanisms (four-bar: mobility 1, 3 redundant rows; RSSR: mobility 2).

**Principled method:**
- (a) Get structural/generic rank exactly first. Use the bipartite matching (Dulmage-Mendelsohn) or
  pebble-game rank of the constraint graph, plus symbolic sparsity. This bounds the rank and tells
  generic redundancy (planar loops in 3D: 3 rows per loop) without any epsilon.
- (b) Certify numerically at the solution, using **one** rank decision from an SVD with a gap test:
  - Accept rank r only if σ_r / σ_{r+1} is large (e.g. > 1e6) and σ_{r+1} ≤ c·ε·‖J‖·√(mn).
  - Otherwise report "near-singular configuration" rather than guess.
  - Derive the Redundant rows, `implied_by`, mobility and per-joint free from that **same**
    factorization (the left null space gives redundancy, the right null space gives the motions).
    That removes R2.
- (c) Where the generic and numeric ranks differ (a special configuration such as a toggle or
  aligned axes), confirm with a witness. Evaluate J at a generic random point of the solution
  manifold (or a nearby random configuration satisfying the same structure). Generic rank equal to
  the structural rank means the drop is configuration-specific (a singular position), not design
  redundancy. Report it as such.
- (d) Use one closure tolerance tied to LINEAR, with angles scaled by the block's own extent. Use it
  for closed-form SAME, Newton acceptance, limits and relations alike (fixes R1, R3, R5).
- (e) Make the diagnosis use the same homotopy as the solve (fixes R4).

## 6. Git history and tests
- `git log --oneline -- kernel/forge-asm` gives exactly one commit: `979190b Install ForgeCAD
  launcher and update from latest main` (a squashed import of the whole repo). No per-fix history
  exists, so no "branch added for one failing assembly" commit can be identified.
- `tools/proof/known.txt` and `interface/forge-lang/tests/fuzz_known.txt` contain no assembly
  entries.
- The test doc comments describe gates from the prior-art doc (§11.2-11.11): construct-then-solve
  property tests from perturbed starts, textbook mechanisms against FD mobility, typed diagnosis
  with minimal sets, determinism across thread counts and wasm, the interference oracle, the
  5,000-instance scale, and the STEP round-trip.
- Single-configuration tests (not patches):
  - property.rs:216-217 `rotate_parallel` deliberately defeats bundle recognition so that the
    numeric path is exercised.
  - checks.rs:378 puts half the cases on a 1 mm grid to hit coincident faces.

## 7. cargo test -p forge-asm
All pass, none ignored:
- unit (qr, check narrow tests): 5
- checks: 9
- determinism: 2
- diagnosis: 10
- mechanisms: 8
- property: 5
- scale: 1 (timing asserted only with FORGE_ASM_TIMING=1)
- step: 2
- doc-tests: 0

Total 42 passed, 0 failed, 0 ignored.

## 8. After the fix (branch claude/repo-review-mbdrlj-fix-mesh-asm): done, and the follow-ups
Done:
- §5 (a) and (b): the generic rank (bipartite matching of the block's rows on its unknowns)
  and one SVD in the block's own frame (rank.rs, replacing qr.rs). The floor is
  64·ε·(m+n)·max(1, σmax) plus the residual, and a gap of 1e3 above it. Redundant rows,
  `implied_by`, mobility and each joint's free motions all come from that one factorization.
  Each block's report carries `generic_rank` and `rank_clear`.
- §5 (d): one met rule, `plan::met(gap)`: gap ≤ LINEAR, with angles scaled by the
  arrangement's lever (its extent about its centroid, plus the anchors' offsets). Used for
  closed-form SAME, Newton acceptance, limits and relations. tests/invariance.rs: moving a whole
  assembly by a rigid motion changes no answer.
- Interference from the boolean: a near pair (boxes within tolerance + LINEAR/2) is decided by
  the exact intersection. Touching (2V ≤ LINEAR·A) is reported as contact; TOUCH_VOLUME is gone.

Follow-ups, not done here:
- §5 (c), the witness configuration. Where `rank < generic_rank` (a toggle, aligned axes), the
  drop is reported, but not confirmed at a generic point of the solution manifold. The step:
  evaluate J at a random configuration that satisfies the same structure, and report
  "singular position" when its rank is the generic one.
- §5 (e) / R4, the ramp in re-solves. The diagnosis's subset solves (`solves`) and the final
  re-solve still start from `begin` with no ramp, so a set solvable only through the ramp can be
  named a Conflict. The step: run every membership test through the solve's own homotopy.
- A typed Issue for a block whose rank is not clear (`rank_clear == false`, no gap of 1e3 at
  the floor): today it is only a field of the block's report. It would be a forge-asm Diagnostic
  plus a forge-lang Issue (a language change, under its own review), so it is left here.
