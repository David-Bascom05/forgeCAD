import ForgeCurved.BodyModel
import Mathlib.Data.Matrix.Mul

/-!
# §1a: the exact `On` of `classify` and `point_in_face`, read on today's code

old-path-statements.md §1a (the master's ruling: `On` is exact) and the owner's approach of
2026-11-23: prove what today's code already guarantees; a failure is a BUG when the exact
reference shows a wrong result, a GAP when it is unproven but may be right.

**What today's code decides.** `Index::classify(p)` returns `On` at the first face `f` whose box
holds `p`, with `surface.distance(p) ≤ LINEAR` and `point_in_face(f, p) ≠ Outside`. Read exactly,
two facts make that answer right:
1. **on the carrier**: `p` lies exactly on `f`'s carrier (for a plane, `n·p = n·o` in exact
   dyadics);
2. **in the face's closure**: `point_in_face`'s `Inside`/`On` means `p ∈ closure ⟦f⟧`.

`classify_on_sound` proves `whereR ⟦b⟧ p = on` from these two, and `exists_face_of_on` proves the
converse direction the loop needs: an exact `On` point lies on some face's region, so the loop can
find it (given D1's boxes and the distance filter's completeness).

**Measured (FLAT-SITES, 2026-11-23**: gate parts seq + nary1, fuzz 1–40 seq, temporary probes):
* fact 1 held at **3,632 of 3,632** planar `On`s of `classify` in that corpus: every point exactly
  on the plane, as the f64 point itself, so there `foot p = p` (`foot_self`). It does **not** hold
  on the bench parts: frame (`dbh`, feature 98, a point 2.6e-16 off its plane) and volute
  (`dbolt`, feature 31, 8.6e-16 off). Requiring it in the code (proof/flat-sites-onplane: `On`
  only where `n·(p − o) = 0` exactly) made both parts refuse with `ZeroThickness`, so today's
  tolerance `On` is load-bearing there. Fact 1 is a GAP: at those points the exact answer must
  come from the point's exact key (its foot), not from the f64 point;
* fact 2 rests on `point_in_face`'s nearest-edge `On`. On a plane face with a line edge:
  2,579 points lie exactly on both planes (exactly on the edge's carrier: `on_edge_closure`);
  1,471 lie exactly on the plane across but not on the face's own plane (points constructed on the
  other face, kind (A) of §1a: their exact key is the meet of the planes, which is on both);
  **36 lie exactly on the face's plane but 1.4e-14 … 8.0e-14 off the edge's line** (patterns seq,
  pattern copies: their planes are rotated, so the meet is not representable). All 36 come from
  one caller, trim_in's stretch sampler (`inside`'s `at`): the sample is on the new edge's curve,
  the meet line L of face k's plane and the other face's, and the stretch spans the whole edge
  (27.24 mm, to 1e-13). **Checked exactly** (dyadic planes, `meet3`): at none of the 36 do the
  three planes (face k, the face across the edge, the other face) share a line. At 31, L crosses
  the plane across at one point, 1.4 to 650 mm from the sample; at 5, L is parallel to it and
  offset. So the exact point a sample stands for is off the edge, and the exact answer there is
  `Inside` or `Outside`, not today's tolerance `On`. These 36 decisions differ from the exact
  reading. The exact result would keep a sliver ~1e-14 wide along each edge where today's merges
  it. Judged against §E (the coordinator's ruling): the tolerance `On` is correct there under
  δ-Faithful, the result valid and each edge within δ of its exact meet (TolOn.lean
  `edge_within_of_ends`; measured: edges within 2.7e-13 of their exact meet lines).

**Ledger.** 1a.where: proven (this file). 1a.classify: the On branch proven from facts 1–2
(`classify_on_sound`), fact 1 a GAP (fails on the bench parts frame and volute, above); the ray
branch is open. 1a.foot: `foot_on`, `foot_self` proven.
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof.FlatSites

open Set ForgeCurved.BodyModel Matrix

/-- The exact answers. -/
inductive Where
  | inside
  | on
  | outside
  deriving DecidableEq, Repr

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The exact answer for a point and a solid `S`. -/
noncomputable def whereR (S : Set E) (p : E) : Where := by
  classical
  exact if p ∈ interior S then .inside else if p ∈ frontier S then .on else .outside

/-- The exact answer for a point and a face region `F` on a carrier with zero set `Z`: inside when
a neighbourhood's carrier points are all in `F` (the relative interior), on when in `F`'s closure
otherwise. -/
noncomputable def whereF (Z F : Set E) (p : E) : Where := by
  classical
  exact if ∃ U, IsOpen U ∧ p ∈ U ∧ U ∩ Z ⊆ F then .inside
    else if p ∈ closure F then .on else .outside

theorem whereR_on_iff {S : Set E} {p : E} : whereR S p = .on ↔ p ∈ frontier S := by
  unfold whereR
  by_cases hi : p ∈ interior S
  · simp only [hi, ite_true, reduceCtorEq, false_iff]
    exact fun hf => (Set.disjoint_left.mp disjoint_interior_frontier hi) hf
  · by_cases hf : p ∈ frontier S <;> simp [hi, hf]

theorem whereR_inside_iff {S : Set E} {p : E} : whereR S p = .inside ↔ p ∈ interior S := by
  unfold whereR
  by_cases hi : p ∈ interior S
  · simp [hi]
  · by_cases hf : p ∈ frontier S <;> simp [hi, hf]

/-- For a closed solid, `outside` is exactly "not in it". -/
theorem whereR_outside_iff {S : Set E} (hS : IsClosed S) {p : E} :
    whereR S p = .outside ↔ p ∉ S := by
  unfold whereR
  by_cases hi : p ∈ interior S
  · simp only [hi, ite_true, reduceCtorEq, false_iff, not_not]
    exact interior_subset hi
  · by_cases hf : p ∈ frontier S
    · simp only [hi, hf, ite_true, ite_false, reduceCtorEq, false_iff, not_not]
      exact hS.frontier_subset hf
    · simp only [hi, hf, ite_false, true_iff]
      intro hp
      have : p ∈ closure S \ interior S := ⟨subset_closure hp, hi⟩
      exact hf this

/-- A point of a face's region, off the vertex-cluster zones, is on the solid's frontier. (Inside a
zone the micro-faces of near-concurrent carriers need not be: ledger D5.zone.) -/
theorem on_of_region {b : BodyModel E} {S : Set E} (h : Bounds b S) {f : FaceModel E}
    (hf : f ∈ b.faces) {p : E} (hp : p ∈ f.region) (hz : p ∉ Zones b) : whereR S p = .on := by
  rw [whereR_on_iff]
  exact h.faces_sub ⟨Set.mem_iUnion₂.mpr ⟨f, hf, hp⟩, hz⟩

/-- And so is a point of its closure that is clear of the zones' closure: the frontier is closed. -/
theorem on_of_closure_region {b : BodyModel E} {S : Set E} (h : Bounds b S) {f : FaceModel E}
    (hf : f ∈ b.faces) {p : E} (hp : p ∈ closure f.region) (hz : p ∉ closure (Zones b)) :
    whereR S p = .on := by
  rw [whereR_on_iff]
  -- a neighbourhood of `p` misses the zones, so `p` is in the closure of the region off them
  have hU : IsOpen (closure (Zones b))ᶜ := isClosed_closure.isOpen_compl
  have hp' : p ∈ closure (f.region \ Zones b) := by
    rw [mem_closure_iff] at hp ⊢
    intro o ho hpo
    obtain ⟨y, hyo, hyr⟩ := hp (o ∩ (closure (Zones b))ᶜ) (ho.inter hU) ⟨hpo, hz⟩
    exact ⟨y, hyo.1, hyr, fun hy => hyo.2 (subset_closure hy)⟩
  have hsub : f.region \ Zones b ⊆ frontier S :=
    fun y hy => h.faces_sub ⟨Set.mem_iUnion₂.mpr ⟨f, hf, hy.1⟩, hy.2⟩
  exact (isClosed_frontier.closure_subset_iff.mpr hsub) hp'

/-- **The converse the loop needs**: an exact `On` lies on some face's region. -/
theorem exists_face_of_on {b : BodyModel E} {S : Set E} (h : Bounds b S) {p : E}
    (hw : whereR S p = .on) : ∃ f ∈ b.faces, p ∈ f.region := by
  rw [whereR_on_iff] at hw
  simpa [Faces] using h.frontier_sub hw

/-- `whereF ≠ outside` is membership of the closure, for a point on the carrier. -/
theorem mem_closure_of_whereF {Z F : Set E} {p : E} (hz : p ∈ Z) (hw : whereF Z F p ≠ .outside) :
    p ∈ closure F := by
  unfold whereF at hw
  by_cases hi : ∃ U, IsOpen U ∧ p ∈ U ∧ U ∩ Z ⊆ F
  · obtain ⟨U, -, hpU, hUF⟩ := hi
    exact subset_closure (hUF ⟨hpU, hz⟩)
  · by_cases hc : p ∈ closure F
    · exact hc
    · simp [hi, hc] at hw

/-- **`classify`'s `On` branch, read exactly.** On the face's carrier (fact 1) and not outside the
face (fact 2, `point_in_face`'s `Inside` or `On`), clear of the vertex-cluster zones, the exact
answer is `On`. -/
theorem classify_on_sound {b : BodyModel E} {S : Set E} (h : Bounds b S) {f : FaceModel E}
    (hf : f ∈ b.faces) {p : E} (hz : f.carrier p = 0)
    (hpf : whereF {x | f.carrier x = 0} f.region p ≠ .outside) (hzone : p ∉ closure (Zones b)) :
    whereR S p = .on :=
  on_of_closure_region h hf (mem_closure_of_whereF (Z := {x | f.carrier x = 0}) hz hpf) hzone

/-- `point_in_face`'s nearest-edge `On`, read exactly: a point on an edge the face's loops use is
in the face's closure (WellFormed's `edge_in_face`), so `whereF` is not `outside` there. -/
theorem on_edge_closure {b : BodyModel E} (hw : WellFormed b) {f : FaceModel E} (hf : f ∈ b.faces)
    {l : List (Nat × Bool)} (hl : l ∈ f.loops) {c : Nat × Bool} (hc : c ∈ l) {e : EdgeModel E}
    (he : b.edges[c.1]? = some e) {p : E} (hp : p ∈ e.curve) :
    whereF {x | f.carrier x = 0} f.region p ≠ .outside := by
  have hcl : p ∈ closure f.region := hw.edge_in_face f hf l hl c hc e he hp
  unfold whereF
  split_ifs <;> simp_all

/-! ## The foot of a probe (§1a, kind (B)) -/

/-- The foot of `p` on the plane `n·x = d`. -/
def foot (n : Fin 3 → ℚ) (d : ℚ) (p : Fin 3 → ℚ) : Fin 3 → ℚ :=
  p - ((n ⬝ᵥ p - d) / (n ⬝ᵥ n)) • n

theorem foot_on {n : Fin 3 → ℚ} (hn : n ⬝ᵥ n ≠ 0) (d : ℚ) (p : Fin 3 → ℚ) :
    n ⬝ᵥ foot n d p = d := by
  unfold foot
  rw [dotProduct_sub, dotProduct_smul, smul_eq_mul, div_mul_cancel₀ _ hn]
  ring

/-- A point already on the plane is its own foot: where today's f64 point is exactly on the
carrier (all of `classify`'s planar `On`s measured), reading the foot changes nothing. -/
theorem foot_self {n : Fin 3 → ℚ} {d : ℚ} {p : Fin 3 → ℚ} (hp : n ⬝ᵥ p = d) : foot n d p = p := by
  unfold foot
  rw [hp, sub_self, zero_div, zero_smul, sub_zero]

end ForgeBoolProof.FlatSites
