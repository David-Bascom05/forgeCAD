import ForgeExact.DecideOnce.Boxes
import ForgeExact.DecideOnce.Correct
import ForgeExact.DecideOnce.Metric

/-!
# (d) The six retry-rung acceptance tests

decisions.md: every rung of the retry ladder rescues at least one regression test, so the
decide-once rewrite must make each pass without its rung. Each rung re-asks one fact kind with the
other answer:

| rung | test | fact kind it flips | input |
|---|---|---|---|
| CAREFUL | forge-bool `a_hole_on_a_rounds_centre_circle_at_any_angle` | touch or crossing (tracer accuracy) | torus, cylinder |
| ABSORB_HAIR_ROOTS | forge-lang `disc_rim_tangent_to_a_flat_it_sits_on` | touch or crossing (two roots or one touch) | cylinder on a flat |
| CHORDS | forge-lang `a_tilted_cutter_hovering_over_a_face` | point on edge / vertex on surface (hovering edge) | tilted cylinder |
| FENCES | forge-lang `a_rounded_blade_stands_on_a_spline_hub` | two cuts one vertex (a closed curve crossed an odd number of times) | loft, spline revolve |
| NO_RUN_ON | `near_tolerance/20` (the corpus) | curve along edge (run-on or crossing) | blocks |
| n-ary → sequential | forge-lang `disc_in_a_scoop_with_knife_edges` | a second evaluator (knife edge) | cylinders |

**What is proven, for every one of them** (`rung_covered`, `rung_exact`). An instance of a rung is
the ledger its input gives, under *any* reading of the fact the rung flipped. Whatever the
reading, if the ledger is consistent the core ends, with no retry, in empty, a valid body or a
stated `ZeroThickness` (theorem (b)); `SplitCrossed` and `InconsistentClassification`, the ladder's
triggers, cannot occur, so no rung has anything to catch. The reading that is exact for the snapped
geometry gives the regularised boolean (theorem (c)). The n-ary rung has no second evaluator to
fall back to: the core takes any pointwise rule over any number of bodies, so the n-ary history is
one call (`Rule` over the family), not a different algorithm.

**What is checked on flat instances.** Five of the six inputs are curved: the flat proof covers the
fact the rung flipped (each is one ledger entry, decided once) but not their curved geometry,
which waits for stage 2 (quadrics). Each rung gets a flat instance with the same decision at its
core, after the snap: a box ledger (`boxLedger`) whose consistency is a kernel-checked `decide`, so
(b) applies to it as a theorem; and the core's answer on it is evaluated at build time (`#guard`,
which runs the compiled code; it is a test, not a theorem). Where a rung's fact had a second
reading, that reading is an instance too, and is covered alike (`noRunOn_gap`).
-/

namespace ForgeExact.DecideOnce

open Ledger

/-- The six rungs. -/
inductive Rung
  | careful
  | absorbHairRoots
  | chords
  | fences
  | noRunOn
  | naryToSequential
  deriving DecidableEq, Repr

def Rung.test : Rung → String
  | .careful => "forge-bool tests/booleans.rs a_hole_on_a_rounds_centre_circle_at_any_angle"
  | .absorbHairRoots => "forge-lang tests/kernel_fusion_parts.rs disc_rim_tangent_to_a_flat_it_sits_on"
  | .chords => "forge-lang tests/kernel_build_failures.rs a_tilted_cutter_hovering_over_a_face"
  | .fences => "forge-lang tests/kernel_misc.rs a_rounded_blade_stands_on_a_spline_hub"
  | .noRunOn => "corpus near_tolerance/20"
  | .naryToSequential => "forge-lang tests/kernel_fusion_parts.rs disc_in_a_scoop_with_knife_edges"

/-- An instance of a rung: the ledger of its input under each reading of the fact it flipped. -/
structure Instance (r : Rung) where
  readings : List Ledger
  rule : Rule

/-- **Every rung's instance is covered with no retry**, under every consistent reading. -/
theorem rung_covered {r : Rung} (I : Instance r) (h : ∀ L ∈ I.readings, Consistent L) :
    ∀ L ∈ I.readings,
      core L I.rule ≠ .refused .splitCrossed ∧ core L I.rule ≠ .refused .inconsistentClassification ∧
      ((core L I.rule = .empty ∧ L.result (L.keepTab I.rule) = []) ∨
       (∃ b, core L I.rule = .ok b ∧ Valid L (L.keepTab I.rule) b) ∨
       (∃ x, core L I.rule = .refused (.zeroThickness x) ∧ ZeroThicknessAt L (L.keepTab I.rule) x)) :=
  fun L hL => ⟨(no_retry (h L hL) I.rule).1, (no_retry (h L hL) I.rule).2, coverage (h L hL) I.rule⟩

/-- **And the exact reading is correct**: its result is the exact boundary of the regularised
boolean of the snapped solids, and it refuses exactly when that is non-manifold. -/
theorem rung_exact {r : Rung} (I : Instance r) {L : Ledger} {ps : List Ref.Plane}
    {A : Nat → Pred Ref.V3} (h : Exact L ps A) :
    (∀ φ, φ ∈ L.result (L.keepTab I.rule) ↔ XBoundary L ps A I.rule φ) ∧
    reg ((decomp h).kept fun T => xkeep L ps A I.rule T = true) =
      reg (combine (ruleOp I.rule) (opnds L A)) ∧
    ((∃ x, core L I.rule = .refused (.zeroThickness x)) ↔ XNonManifold L ps A I.rule) :=
  ⟨(correct h I.rule).1, (correct h I.rule).2, zeroThickness_iff h I.rule⟩

/-! ## Flat instances

Units are grid steps: a sub-LINEAR gap that §6.1 snaps is no step at all; one that it keeps is
drawn a step wide. -/

def uni : Rule := fun m => m 0 || m 1
def sub : Rule := fun m => m 0 && !m 1
def uni3 : Rule := fun m => m 0 || m 1 || m 2

/-- NO_RUN_ON, near_tolerance/20: a block on a plate, its end wall within LINEAR of the plate's
end, snapped flush. The block's bottom edge then runs along the plate's top edge by lookup (the
curve-along-edge fact is derived, `Decisions.on_of_onEdge`), never re-measured. -/
def noRunOn : Ledger := boxLedger 2 [⟨(0, 0, 0), (2, 1, 1)⟩, ⟨(1, 0, 1), (2, 1, 2)⟩]

/-- The other reading: the gap kept (not snapped), the block a step short of the end. -/
def noRunOn_gap : Ledger := boxLedger 3 [⟨(0, 0, 0), (3, 1, 1)⟩, ⟨(1, 0, 1), (2, 1, 2)⟩]

/-- CHORDS: a cutter whose rim hovers within LINEAR of the part's bottom, snapped into it: the
hovering edge is the bottom's own (vertex on surface, decided in the representative). -/
def chords : Ledger := boxLedger 2 [⟨(0, 0, 0), (2, 1, 2)⟩, ⟨(1, 0, 0), (2, 1, 1)⟩]

/-- ABSORB_HAIR_ROOTS: a body sitting on a flat, touching it along its whole underside: one touch,
not two crossings (touch or crossing, one answer). -/
def absorb : Ledger := boxLedger 2 [⟨(0, 0, 0), (2, 1, 1)⟩, ⟨(0, 0, 1), (1, 1, 2)⟩]

/-- FENCES: a blade standing on a hub: a closed curve of contact inside one face, crossed by no
edge (two cuts one vertex: each corner of the loop is one cell). -/
def fences : Ledger := boxLedger 3 [⟨(0, 0, 0), (3, 3, 1)⟩, ⟨(1, 1, 1), (2, 2, 2)⟩]

/-- CAREFUL: a hole whose wall is flush with the part's faces it runs between (the lobes on both
sides found, by cells, not by tracing). -/
def careful : Ledger := boxLedger 3 [⟨(0, 0, 0), (3, 3, 1)⟩, ⟨(1, 1, 0), (2, 2, 1)⟩]

/-- n-ary → sequential: three bodies in one call, two meeting the third along coincident faces. -/
def nary : Ledger :=
  boxLedger 2 [⟨(0, 0, 0), (1, 1, 1)⟩, ⟨(1, 0, 0), (2, 1, 1)⟩, ⟨(1, 0, 1), (2, 1, 2)⟩]

/-- And the touch rule in one n-ary call: two of three bodies meeting along an edge only. -/
def nary_touch : Ledger :=
  boxLedger 2 [⟨(0, 0, 0), (1, 1, 1)⟩, ⟨(1, 1, 0), (2, 2, 1)⟩, ⟨(0, 1, 1), (1, 2, 2)⟩]

theorem noRunOn_consistent : Consistent noRunOn :=
  consistent_of_check (m := noRunOn.witness) (by decide)
theorem noRunOn_gap_consistent : Consistent noRunOn_gap :=
  consistent_of_check (m := noRunOn_gap.witness) (by decide)
theorem chords_consistent : Consistent chords :=
  consistent_of_check (m := chords.witness) (by decide)
theorem absorb_consistent : Consistent absorb :=
  consistent_of_check (m := absorb.witness) (by decide)
theorem fences_consistent : Consistent fences :=
  consistent_of_check (m := fences.witness) (by decide)
theorem careful_consistent : Consistent careful :=
  consistent_of_check (m := careful.witness) (by decide)
theorem nary_consistent : Consistent nary :=
  consistent_of_check (m := nary.witness) (by decide)
theorem nary_touch_consistent : Consistent nary_touch :=
  consistent_of_check (m := nary_touch.witness) (by decide)

/-- The instances, each with its readings. -/
def instNoRunOn : Instance .noRunOn := ⟨[noRunOn, noRunOn_gap], uni⟩
def instChords : Instance .chords := ⟨[chords], sub⟩
def instAbsorb : Instance .absorbHairRoots := ⟨[absorb], uni⟩
def instFences : Instance .fences := ⟨[fences], uni⟩
def instCareful : Instance .careful := ⟨[careful], sub⟩
def instNary : Instance .naryToSequential := ⟨[nary, nary_touch], uni3⟩

/-- **(d): each flat instance falls under (b), with no retry, under every reading.** -/
theorem instances_covered :
    (∀ L ∈ instNoRunOn.readings, core L uni ≠ .refused .splitCrossed ∧
        core L uni ≠ .refused .inconsistentClassification) ∧
    (∀ L ∈ instChords.readings, core L sub ≠ .refused .splitCrossed ∧
        core L sub ≠ .refused .inconsistentClassification) ∧
    (∀ L ∈ instAbsorb.readings, core L uni ≠ .refused .splitCrossed ∧
        core L uni ≠ .refused .inconsistentClassification) ∧
    (∀ L ∈ instFences.readings, core L uni ≠ .refused .splitCrossed ∧
        core L uni ≠ .refused .inconsistentClassification) ∧
    (∀ L ∈ instCareful.readings, core L sub ≠ .refused .splitCrossed ∧
        core L sub ≠ .refused .inconsistentClassification) ∧
    (∀ L ∈ instNary.readings, core L uni3 ≠ .refused .splitCrossed ∧
        core L uni3 ≠ .refused .inconsistentClassification) := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> intro L hL
  · exact (rung_covered instNoRunOn (fun L hL => by
      simp only [instNoRunOn, List.mem_cons, List.mem_nil_iff, or_false] at hL
      rcases hL with rfl | rfl
      · exact noRunOn_consistent
      · exact noRunOn_gap_consistent) L hL |>.imp id fun h => h.1)
  · exact (rung_covered instChords (fun L hL => by
      simp only [instChords, List.mem_cons, List.mem_nil_iff, or_false] at hL
      subst hL; exact chords_consistent) L hL |>.imp id fun h => h.1)
  · exact (rung_covered instAbsorb (fun L hL => by
      simp only [instAbsorb, List.mem_cons, List.mem_nil_iff, or_false] at hL
      subst hL; exact absorb_consistent) L hL |>.imp id fun h => h.1)
  · exact (rung_covered instFences (fun L hL => by
      simp only [instFences, List.mem_cons, List.mem_nil_iff, or_false] at hL
      subst hL; exact fences_consistent) L hL |>.imp id fun h => h.1)
  · exact (rung_covered instCareful (fun L hL => by
      simp only [instCareful, List.mem_cons, List.mem_nil_iff, or_false] at hL
      subst hL; exact careful_consistent) L hL |>.imp id fun h => h.1)
  · exact (rung_covered instNary (fun L hL => by
      simp only [instNary, List.mem_cons, List.mem_nil_iff, or_false] at hL
      rcases hL with rfl | rfl
      · exact nary_consistent
      · exact nary_touch_consistent) L hL |>.imp id fun h => h.1)

/-! ## What the core answers (evaluated at build time: tests, not theorems)

`Summary.ok faces facets edges vertices`. -/

-- The L-shaped union, one call, no retry.
#guard (core noRunOn uni).summary == .ok 8 14 18 12
-- The other reading: a plate and a block on it a step from the end.
#guard (core noRunOn_gap uni).summary == .ok 10 18 24 16
-- The notch the cutter leaves.
#guard (core chords sub).summary == .ok 8 14 18 12
-- Joined across the whole underside: an L, the contact dissolved.
#guard (core absorb uni).summary == .ok 8 14 18 12
-- The blade on the hub: the top keeps the loop as an inner boundary.
#guard (core fences uni).summary == .ok 11 34 24 16
-- The hole through: a ring, ten faces.
#guard (core careful sub).summary == .ok 10 32 24 16
-- Three bodies, one call: the L again.
#guard (core nary uni3).summary == .ok 8 14 18 12
-- Two of three meeting along an edge only: refused at that edge, in the one n-ary call.
#guard match (core nary_touch uni3).summary with
  | .zeroThickness _ => true
  | _ => false

-- Two cubes at a corner, united: non-manifold at the shared vertex (coverage.md's G1).
#guard (core (boxLedger 2 [⟨(0, 0, 0), (1, 1, 1)⟩, ⟨(1, 1, 1), (2, 2, 2)⟩]) uni).summary ==
  .zeroThickness (cov 2 (2, 2, 2))
-- At an edge: refused at that edge cell.
#guard (core (boxLedger 2 [⟨(0, 0, 0), (1, 1, 1)⟩, ⟨(1, 1, 0), (2, 2, 1)⟩]) uni).summary ==
  .zeroThickness (cov 2 (2, 2, 1))
-- Face to face: one box, the contact dissolved (exact-comparison.md's "contact left imprinted").
#guard (core (boxLedger 2 [⟨(0, 0, 0), (1, 1, 1)⟩, ⟨(1, 0, 0), (2, 1, 1)⟩]) uni).summary ==
  .ok 6 10 12 8

/-! ## The metric ledger on two features (build-time checks) -/

def outcomeM (L : Ledger) (rule : Rule) (thin enc : List Cov) : Summary := (L.coreM rule thin enc).summary

-- A void inside a block, decided thinner than LINEAR and enclosed by the cut's faces: filled; the
-- block is one box again.
#guard outcomeM (boxLedger 3 [⟨(0, 0, 0), (3, 3, 3)⟩, ⟨(1, 1, 1), (2, 2, 2)⟩]) sub
  [cov 3 (3, 3, 3)] [cov 3 (3, 3, 3)] == .ok 6 54 12 8
-- Without the metric decision the void stays: twelve faces.
#guard outcomeM (boxLedger 3 [⟨(0, 0, 0), (3, 3, 3)⟩, ⟨(1, 1, 1), (2, 2, 2)⟩]) sub [] [] == .ok 12 60 24 16
-- A fin on a plate, decided thinner than LINEAR: its rim is material below and empty space round
-- it, so it is refused at the fin.
#guard outcomeM (boxLedger 3 [⟨(0, 0, 0), (3, 1, 1)⟩, ⟨(1, 0, 1), (2, 1, 2)⟩]) uni
  [cov 3 (3, 1, 3)] [cov 3 (3, 1, 3)] == .zeroThickness (cov 3 (3, 1, 3))

end ForgeExact.DecideOnce
