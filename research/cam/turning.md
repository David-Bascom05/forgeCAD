# CAM Phase 5, turning: status (2026-11-23)

Crate: kernel/forge-turn (std and our own crates only). Lean: verify/forge-cam/ForgeCam/Turn.lean.
Design: design.md §7 row 5 ("a lathe setup; face, profile rough and finish, groove, thread,
part-off, drill on centre; insert and holder libraries (ISO insert codes). All 2D and exact").

## Built

| piece | where | state |
|---|---|---|
| half-profile | profile.rs | a body of revolution about z lowered to its (ρ, z) region, exact in kind: planes ⟂ z, cylinders, cones, spheres and tori about z (seams skipped; a torus or sphere arc's side from the face's uv range). Anything else (flats, cross holes, keyways, faces about another axis) refused by kind, `NotTurned`, with the face. |
| inserts and holders | insert.rs, data/turning.toml | ISO 1832 insert codes (C D V T S W R shapes: the trigon W a hexagon of six equal eroded sides, clearance letter, IC series, thickness, nose radius) and ISO 5608 external holders (P/S clamping, styles A B D F G J K L N S T V, right hand, shank, length). Grooving and part-off blades, 60° thread inserts, drills on the centre line. Generic library (verify with the vendor). |
| tool model | insert.rs | every body is K ⊕ B_r (K a convex core); below the centre height the outline shrinks by the clearance angle; holder head (a seat inset 0.5 under the insert, relieved like it) and shank. |
| certified checker | check.rs | 2D, exact in kind: the swept core Z = K ⊕ [0, e]·x̂ ⊕ [a, b] = conv(P) against each obstacle piece by separating axes (the gap between P's and the piece's projections, proven: `sep_axis_hull`); a concave arc not separated whole is split into sub-arcs; containment by `clear_of_frontier`; the full section H ∪ mirror(H); arc moves as chords with the sagitta; witnesses (Hit) only at real points of the tool. |
| ops | ops.rs | face (to the centre line), rough (axial passes, G71-like, expanded, at every ap and every cylinder of the grown part), finish (the exact offset H ⊕ B_{r_ε + leave}: nose-radius compensation, posted as the imaginary tip with G40), groove (plunges, walls exact), part-off (past the centre), thread (ISO metric 60°, radial infeed of equal chip area, a declared removal Δ), drill on the centre line (pecking). |
| in-process stock | ops.rs | the bar less what each cut provably cleared (the hull of outline points along each cut, inside the swept outline; a finishing cut's corners sampled to within 5e-4 of their disc), by exact booleans; a removal that would not round to a valid region is left out (the stock stays a superset) and counted. |
| certify | ops.rs | G1 (cutter from P ⊕ B_leave, P ∖ Δ for a thread), G2 (every body from the chuck's envelope by the fixture clearance), G3 (every body on a rapid, the non-cutting ones on every cut, from the stock; trailing bodies piece by piece where needed), G5 (the tip within the travel when given), G4 (leftover = final stock less P ⊕ B_tol, less Δ, listed). |
| posts | post.rs, data/lathe-*.toml | fanuc-lathe, haas-lathe, linuxcnc-lathe: G18, diameter X, G96/G50 (or D), G99/G95, G02/G03 with I (radius) K, G32/G33; exact quantisation (forge_post::quant), a reader of our own output, the round trip (G6 as for milling), and the posted path certified (the moves the text means). |

## The example

A stepped shaft from a 32 mm bar (tests/job.rs): face, rough (CNMG 120408 in PCLNR), finish
(DNMG 150404 in PDJNR), thread M20 x 1.5, part off: 624 checks, certified, 0.28 s release;
posted in all three dialects, read back exactly, the posted path certified again. The leftover
lists one region, classified exactly: it lies within the finishing nose's 0.4 mm of the part
(the sharp inside corner at the shoulder no nose of that radius reaches).

What the checks caught while the examples were written (each now a test, tests/refuse.rs):
- a 55° DNMG finishing a 45° V-groove gouges its falling flank with its trailing edge (G1, Hit);
  a 35° VBMT in SVJBR certifies;
- a drill into a solid shaft (G1, Hit at the tip); a groove plunged 0.5 deeper than the part's
  (G1, Hit at the corner); a part-off blade shorter than the bar's radius (refused at the op);
- in planning: the in-process stock must update within a pass (the holder head trails the
  insert through what it has just cut), the half-profile's axis segment is no boundary (the
  check is made in the full section), and a parting blade or a drill leaving its own cut has no
  room for a rapid's clearance (those retracts are feeds).

## Gates

| gate | result |
|---|---|
| tests | forge-turn 32, forge-turn-lang 6; all pass native and wasm32-wasip1 |
| native = wasm32 | tests/bits.rs: one digest over the profile, the paths, the verdicts, the leftover and the three dialects' text, equal on x86-64 and wasm32-wasip1 |
| clippy | clean; transcendental functions through forge_math::dmath |
| no third-party crates in kernel/ | none |
| speed band (callgrind, tools/speed/band.py) | not yet measured: forge-turn is a new crate that no measured path calls (regen1 parts, bench app/edit/cli do not link it); to be run against main before the PR, as required |

## Levels (PROOF-STANDARD.md)

- L1 (Turn.lean, 17 theorems, standard axioms): π is 1-Lipschitz (`pi_lipschitz`,
  `turn_clear3`): a clearance in the half-plane is one in space; the smear bound (`smear_le`,
  `smear_anti`, `abs_le_rho`); the full section (`fold_clear`, `fold_sigma`, `sigma_mem`); the
  checker's one distance bound, a separating axis between hulls (`sep_axis_hull`, `sep_axis`),
  with the swept core a hull of finitely many points (`zono_eq_hull`); the smear vertex by
  vertex (`smear_hull`, Jensen) and a slab shrunk past the nose radius (`eroded_gap`); the
  in-process stock as columns (`rect_in_convex`); the assembly through
  `clear_of_frontier` (`turn_clear`, `turn_clear_ball`).
- Hypotheses, open (not proven, not assumed true elsewhere):
  - **H.arc** (the control): an arc block whose ends lie at unequal radii (quantisation) moves
    between those radii. Checked by adding the difference to the clearance; a property of the
    control, not of our code.
  - **T.model**: the tool solid is inside its model (data, as a milling cutter's profile):
    generic holders are supersets restated from catalogue dimensions.
  - **T.cut**: what a cutter's outline sweeps at depth 0 is removed (the definition of cutting
    used by G4 and the stock, as in milling).
  - **T.pieces**: the obstacle's pieces cover its frontier (the half-profile's construction from
    the B-rep's meridians; the region is validated exactly).
- Closed while building: the checker first enumerated the swept core's boundary as segments
  (needing T.zono, that boundary lies in them, and T.loops, a box test for loops inside Z);
  it now uses separating axes only, which `sep_axis_hull` proves, so both are gone.
- L2: the profile offsets and stock booleans are exact (forge-cam-geom); the checker is
  certified-interval.
- L5: the checker's intervals round outward (forge-cam-geom ival.rs); not tied to verify/ieee754.
- L6: the posted round trip on three dialects; native = wasm digest.

## Gaps, in order

1. Lean: L5 (the checker's outward rounding against verify/ieee754), and an executable ℚ
   oracle for `axis_gap` (L6).
2. ID work: boring is built (ISO 6261 bars decoded; the bar a cylinder whose σ-image is an
   exact rectangle per move; `bore_rough`, `finish` inside; a bush bored from a Ø26 drilled hole
   with an S20 bar certifies, a hole too small for the bar is refused). Internal grooving and
   threading are built (ID blades and 60° inserts on bars, each bar checked as its cylinder;
   `id_groove` enters along the axis and plunges outward, `id_thread` feeds out from the minor
   diameter with Δ = [r1, d/2] declared; examples/machined/nut_turn.fg certifies in full, 58 ms;
   caught: finishing the bore on through the relief groove, the boring insert's trailing edge
   runs up through the groove's top wall; an M31 in an M30's bore, G1; an Ø16 bar in an M12's
   bore, refused). Left-hand tools are built: an L holder decodes to its R twin mirrored across
   z (`mirrored`: convex cores to convex cores, the nose kept first, tip number 3 ↔ 4), refused by
   the ops that feed toward the chuck; `finish_up` walks the grown profile up (+z) from z_lo to
   z_hi (examples/machined/collar_turn.fg: a back chamfer, certified in full; caught: the
   right-hand VBMT on down the chamfer, and the left-hand one started 1 mm over the shoulder,
   its insert hanging 9 mm toward the chuck). Left-hand bars decode (the R twin mirrored) and are refused by the ops that feed toward the chuck. Left: left-hand roughing, a left-hand bore op. tests/vendor_codes.rs (CAM-HAAS): 75 real catalogue codes decode to their makers' stated IC, s, re, l, κr, hand, shank and length, and an undecodable code fails.
   Other workholding: `Setup.fixtures`, named envelopes of revolution (a tailstock's centre and
   quill, a steady rest), checked as the chuck is (G2); a live centre refuses facing to the
   centre line and finishing from over it (tests/refuse.rs). The .fg records come from
   WORKHOLDING (forge-workholding), lowered in forge-turn-lang.
3. Done: the .fg surface (interface/forge-turn-lang, docs/cam-turning.md; forge-cli sends a
   turning document there by one branch on `is_turning`, no change to forge-cam-lang), and the
   machine from CAM-MACHINE's forge-machine (machine.rs converts its MachineDef; the lathes of
   its built-in library by name, `machine::builtin`). Left: the setup sheet for turning, more
   than one setup per document.
5. G4 reasons: "nose radius r" (an inside corner: the region misses the free region F_r = box ∖
   (P ⊕ B_r) where a disc's centre can be, and each piece of its boundary is at least r − 2·tol
   from it by the checker's certified bound; r the smallest finishing corner, a nose or a
   grooving blade's, that holds it) or "uncut" (a leave's band among them; before, "within r of
   the part" called a band thinner than r a nose corner). Reach and undercut are not told apart
   yet.
6. Speed (David: prove a check unneeded rather than pay for it). Measured on the shaft job:
   generate 11 ms, post and round trip 0.13 ms, certify 267 ms, of which ~73% the in-process
   stock's exact booleans. Done: the normal path keeps the stock as columns (stair.rs: it loses
   only rectangles proven inside a cut's swept outline, `rect_in_convex`, so it stays a superset
   and G3 against it is sound) with no booleans; the staircase's inside test on its steps (no
   exact region); a box pre-test per check (the image lies in the core's box grown by r and,
   along x, by the depth: the smear never exceeds it, `smear_le` at xm = 0; only a positive gap
   concludes). Certify 267 → 31 ms on the shaft job. The exact stock and the G4 leftover run in `cam F check` and as the test oracle.
   Next: G1 for the nose disc by construction from the exact offset (the remaining checks are
   the input-dependent ones: the insert's flanks and the holder against the part and the
   chuck).
7. Canned cycles (G71/G70/G76) as posted text (today the cycles are expanded into moves, which
   is what G6 reads back and certifies).
