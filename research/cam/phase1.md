# CAM Phase 1: status (2026-11-23)

Branch claude/repo-review-mbdrlj-cam. User doc: docs/cam.md. Phase 0: phase0.md.

## Built

| piece | where | state |
|---|---|---|
| exact reals, regions, offsets | kernel/forge-cam-geom (big, real, arr, region, offset) | done; L6 fuzz |
| exact region booleans | forge-cam-geom/boolean.rs | done; L6 fuzz (60 cases, >10k points) |
| tools as revolved profiles | forge-cam-geom/tool.rs | flat, ball, bull, chamfer/spot/countersink/engrave, drill; taps, thread mills, reamers as envelopes |
| certified checker | forge-cam-geom/check.rs (B&B), slab.rs (2.5D closed form) | done; L6 differential |
| B-rep lowering, section | kernel/forge-cam (lower, section) | planes, and cylinders, cones, tori about z; others refused by kind; turned over for a second setup (lower::turned) |
| in-process stock (2.5D) | forge-cam/stock.rs | box minus proven-cleared voids (openings, bands, holes), and voids from below for a turned-over setup |
| workholding | forge-cam/fixture.rs | vises (jaws, parallels, bed) and soft jaws cut to what the earlier setup left; every body keeps the fixture clearance (G2 corrected); generic presets in forge-tooling/data/fixtures.toml |
| ops | forge-cam/ops.rs | facing, clear, contour, pocket (hole), drill/peck/tap cycles; linking by retract |
| libraries | kernel/forge-tooling | tools, holders, assemblies, machines, cutting groups; feeds with chip thinning, ball D_eff, power |
| post | kernel/forge-post | fanuc, haas, linuxcnc, grbl as data; exact quantisation; reader; round trip |
| Fusion importer | interface/forge-cam-import | JSON → our TOML, read back |
| machining document | interface/forge-cam-lang | no grammar change; `forgecad cam`, MCP `cam`; forge-lang gains only `hole_view` |
| feature-aware holes | forge-lang holes.rs `HoleInfo`, `hole_view` | from `hole()` features: d, depth, thread, fit, heads; its own evaluation, never in a part's build |
| reply, setup sheet | cam.rs | `forgecad.cam/1`; HTML sheet |

The flange (examples/machined/flange_cam.fg): five ops, every move certified on the posted path,
program released (Fanuc, 363 lines, round trip exact). The checks caught two real planning errors
while the example was written: a short-fluted tool whose shank would enter the stock in the bore,
and bolt-hole rapids into stock nobody had cleared (both now test cases with typed refusals).

## Gates

| gate | result |
|---|---|
| reliability scoreboard | 0 worse (3 better) against tools/reliability/baseline.txt |
| FUZZ_SEEDS=150 suite | 150/150 |
| n-ary | 167/167 |
| parse_diff (Rust side) | pass |
| forge-render tests | 450 passed, 4 failed: all four panic with "no GPU" (no adapter in this container) |
| FG_LEAN=1 parse_diff | **not run**: no Lean toolchain in this container, and installing one was refused by the permission policy. No grammar change was made (`machining` is a bare name). |
| callgrind speed gate | pass: 67 of 67 strict lines exactly equal to a baseline saved on 29a524e (built at a path of the same length), bits identical. Two lessons on the way: a base built at a longer path differs by a few instructions (string layout), and any byte added to forge-lang shifts the bench binaries' counts by tens of instructions, so CAM's document layer lives in interface/forge-cam-lang and forge-lang is unchanged |
| native = wasm | kernel/forge-cam/tests/bits.rs: one digest over offsets, booleans, verdicts and the four dialects' text, equal on x86-64 and wasm32-wasip1 |
| no third-party crates in kernel/ | none: the new kernel crates depend on std and our own crates only |

## Levels (PROOF-STANDARD.md)

- L1: **verify/forge-cam** (CI forge-cam.yml): Lean 4.36 and Mathlib, standard axioms, no sorry.
  It has 72 theorems; verify/forge-cam/README.md maps them and RESUME.md gives the state.
  - `sd_hyp_mono`, the lemma the 2.5D checker rests on.
  - The closed form's verdicts imply G1–G3 for prism faces and planar, vertical or helical
    moves, the corrected G2 (open form) included, with faces lowered within μ.
  - G4 (with P' = P ∖ Δ and the held bands), G5 (3-axis), and G6 (modal round trip, arcs'
    full-circle semantics).
  - Soft-jaw seating and pocket coverage.
  - slab::dist's candidates, sound for segment–segment, segment–arc, arc–arc and flat regions
    (the point-in-trim test, the radial end pieces, the 2·gap lowering).
  - Assumed: the trim's even-odd rule (a Jordan-curve fact), and L5 (the interval arithmetic).
- L2: the offset and boolean decisions are exact (exact reals to the BFMSS bound).
- L3: not claimed.
- L4: forge-cam-geom is in the Aeneas style (no dependencies, loops) but not translated.
- L5: every float operation of the checker rounds outward (ival.rs); not yet tied to
  verify/ieee754.
- L6: offset fuzz, boolean fuzz, closed form vs B&B, posted round trip on all dialects.
- G4 is computed exactly for 2.5D (the one-sided bound S_end ⊆ (P ⊕ B_tol) ∪ L); P ⊆ S_end follows
  from G1 on every cut.

Two setups with soft jaws (examples/machined/bracket_cam.fg): op1 in a vise, the jaws cut in
their own setup, op2 turned over in them; three programs, every op certified, 0.9 s. G4 lists the
notch's sharp inside corners (no end mill reaches them).

## Gaps, in order

1. The Lean L1 found and closed four soundness issues. Each is fixed here, and the Lean is
   matched to the fix:
   - the null-move printer (b6ec16c);
   - the vanishing helical arc (4bfe2a2);
   - the soft jaws' grip (4bfe2a2);
   - the trim arcs' stored-end gap (3a388e5, bfa4a72).
   Open: L5 for the checker's intervals.
2. G4 (leftover) is checked: per z band, the final stock less the part's section grown by tol,
   exactly (booleans); the flange has none, a facing-only program lists its regions. The
   reasons (tool radius, reach) are not yet classified: every region says "uncut".
3. Ops: done since this note: engraving, thread milling (declared Δ), edge_break (chamfer /
   deburr), helical ramps in pockets. cam/pocket-islands: `slot` (end mill along centrelines,
   ramped along the first line) and pockets of the section keep their islands (the region is
   the hole less the section; before, the island was ignored and the pocket refused as a
   gouge). Slot certification is slow (3.5 s for two slots, ~148k B&B cells: gap 5).
4. Declared removals Δ (deburr, chamfer and thread milling where the model lacks the geometry).
5. Speed: a grid over face boxes and shared distances (−40% instructions); the 10⁵-move line at
   2.7 s is above the 1 s target; next, an f64 filter with a proven error bound.
6. G5 (axis limits) is not checked yet; G4 uses tol where it should use δ + ε (leave).
7. The Manufacture workspace in the app (UI session): the reply above is its data contract.
