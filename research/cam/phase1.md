# CAM Phase 1: status (2026-11-23)

Branch claude/repo-review-mbdrlj-cam. User doc: docs/cam.md. Phase 0: phase0.md.

## Built

| piece | where | state |
|---|---|---|
| exact reals, regions, offsets | kernel/forge-cam-geom (big, real, arr, region, offset) | done; L6 fuzz |
| exact region booleans | forge-cam-geom/boolean.rs | done; L6 fuzz (60 cases, >10k points) |
| tools as revolved profiles | forge-cam-geom/tool.rs | flat, ball, bull, chamfer/spot/countersink/engrave, drill; taps, thread mills, reamers as envelopes |
| certified checker | forge-cam-geom/check.rs (B&B), slab.rs (2.5D closed form) | done; L6 differential |
| B-rep lowering, section | kernel/forge-cam (lower, section) | planes and z-cylinders; other faces refused by kind |
| in-process stock (2.5D) | forge-cam/stock.rs | box minus proven-cleared voids (openings, bands, holes) |
| ops | forge-cam/ops.rs | facing, clear, contour, pocket (hole), drill/peck/tap cycles; linking by retract |
| libraries | kernel/forge-tooling | tools, holders, assemblies, machines, cutting groups; feeds with chip thinning, ball D_eff, power |
| post | kernel/forge-post | fanuc, haas, linuxcnc, grbl as data; exact quantisation; reader; round trip |
| Fusion importer | interface/forge-cam-import | JSON → our TOML, read back |
| machining document | interface/forge-lang/src/cam.rs | no grammar change; `fg cam`, `forgecad cam`, MCP `cam` |
| feature-aware holes | forge-lang holes.rs `HoleInfo` | from `hole()` features: d, depth, thread, fit, heads |
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
| FG_LEAN=1 parse_diff | **not run**: no Lean toolchain in this container, and installing one was refused by the permission policy. No grammar change was made (`machining` is a bare name). |
| callgrind speed gate | see the milestone report (run against a baseline saved on 29a524e) |
| native = wasm | kernel/forge-cam/tests/bits.rs: one digest over offsets, booleans, verdicts and the four dialects' text, equal on x86-64 and wasm32-wasip1 |
| no third-party crates in kernel/ | none: the new kernel crates depend on std and our own crates only |

## Levels (PROOF-STANDARD.md)

- L1: **not yet**. G1–G6 are stated in prose (design.md §5.2, the module docs); the Lean files
  wait for a Lean toolchain here. The one new lemma the 2.5D checker rests on: the signed distance
  of a body {ρ ≤ w(z)} is non-decreasing in ρ at fixed z (proof sketch in tool.rs / slab.rs).
- L2: the offset and boolean decisions are exact (exact reals to the BFMSS bound).
- L3: not claimed.
- L4: forge-cam-geom is in the Aeneas style (no dependencies, loops) but not translated.
- L5: every float operation of the checker rounds outward (ival.rs); not yet tied to
  verify/ieee754.
- L6: offset fuzz, boolean fuzz, closed form vs B&B, posted round trip on all dialects.

## Gaps, in order

1. Lean L1 for G1–G6 and the monotonicity lemma (needs the toolchain).
2. G4 (leftover): the exact 2.5D removal is now computable (booleans, openings); the check that
   the final stock is within ε of the target is not written.
3. Ops: slot, chamfer (deburr), thread milling, engraving, inside contours of pockets with
   islands from the part; helical ramps (the B&B certifies them; generation is missing).
4. Lowering for cones and tori (chamfers, fillets): the B&B takes them; the lowering refuses.
5. Speed: a grid over face boxes; the 10⁵-move line at 3.2 s is above the 1 s target.
6. The Manufacture workspace in the app (UI session): the reply above is its data contract.
