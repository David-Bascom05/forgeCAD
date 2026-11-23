# The ops spec: what is done, and what is left

2026-11-23, at the session's end (branch claude/repo-review-mbdrlj-ops-spec). The owner has
de-prioritised proofs of code that active features are not changing, so the remaining Lean
references were stopped by the master. This lists where each item stands.

## Done

- **Design** (design.md). It covers:
  - the L1 set each feature produces;
  - the exact refusals;
  - a decided rule for each retry or fallback.
- **L1 in Lean** (verify/forge-exact/ForgeExact/Ops/, checked in Ops/Axioms.lean, standard axioms
  only, no sorry, in forge-exact.yml):
  - pattern P1–P3, and one-at-a-time equals one n-ary boolean;
  - hole H1, H3, H4;
  - chamfer C1, C2, and the corner rule with both rows;
  - planar draft D1–D3, and drafted cylinders as frusta (ThroughApex exact);
  - shell S1, S2;
  - rib B1;
  - re-carrying R1.
- **L2 for holes:**
  - `forge-hole-ref` runs the spec's profile over ℚ;
  - `kernel_hole_ref`: 60 repo holes, all the spec's; in CI.
- **L6 for through tools** (`kernel_thru_clear`): every repo part's every through hole place and
  through-extrude cut path, on single- and multi-solid stages, is clear of the bodies it cuts.
  75 hole places and 2468 cut paths today, held to a digest.
- **Kernel fixes** (wrong results before), each with an exact-volume test:
  - through tools sized past ribs;
  - past tapered and slanted extrudes;
  - past moved, offset, replaced, deleted or drafted faces and outward shells (the part's box as
    built: `Given`);
  - pattern copies of through tools sized at the pattern's step.
  The lazy tool face names (upto::names) are a broad speed saving. Gates are passed under the
  layout rule (gate-layout-diff-*.txt).

## Left

1. **Proven outer box** (outer-box.md):
   - IntCurve edge boxes: the sag is measured at chord midpoints. This waits on the trace
     session's certified enclosure.
   - Blends whose bounds rest on sampled derivatives: varblend, Swept, corner (fillet sessions);
     heal Iso (import).
   - A thread on a through hole past a face edit takes the hole's step's box (thread::hole_depth).
2. **L1 still to write:**
   - draft of non-planar faces past the cylinder;
   - the shell's event rule (straight-skeleton order), and its kernel change, which replaces the
     depth-12 search;
   - rib's component spec beyond B1 (the probe germ, B2);
   - Recarrier validity (L3, deciding the homeomorphism from the lattice);
   - chamfer on non-planar pairs.
3. **L2/L6 not built** (stopped): pattern, draft, chamfer, shell and rib references.
4. **L4 (Aeneas):** not started. Candidates are the hole profile builder, `pattern::fixed`, and the
   chamfer distance selection.
5. **L5:** the one rounding per derived number (an angle to its pair, a countersink depth, a drill
   point) is not yet bounded.
6. **Kernel decided rules from design.md, not yet applied:**
   - Hole NoTarget as a lint with a fix, after listing the parts it affects.
   - The chamfer corner rule as an upfront predicate (proven; chamfer.rs:669 still tries and falls
     back).
   - The `along` carrier-line definition.
   - The rib probe germ.
   - The draft error order (AlongPull and NoNeutralLine are one condition).
   - The shell event order.
   - Through tools up to a face or the next face (exact next-face test, H2): not covered by the L2
     or L6 checks.
