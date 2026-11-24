import ForgeBoolProof.Formula
import ForgeBoolProof.Orient
import Mathlib.Analysis.Calculus.FDeriv.Add
import Mathlib.Analysis.Calculus.FDeriv.Linear

/-!
# The geometry rows for flat-faced bodies (`_planar`)

For bodies whose carriers are affine (planes), the geometric hypotheses of D5.thin, D8.reg and
D8.cells hold:

* **`compl_meets_preconnected`**, the key lemma: a convex open set minus finitely many meets of
  two independent affine functions (`{ℓ₁ = c₁, ℓ₂ = c₂}`: lines in 3D, where two planes meet) is
  preconnected. Two of its points `p`, `q` are joined through a middle point `m`: the segment
  `p m` meets such a set `M` only if `m` is on a hyperplane through `p` containing `M` (a
  combination of `ℓ₁` and `ℓ₂`, `hyper_through`), so `m` off finitely many hyperplanes (empty
  interior) joins both. No dimension assumption.
* **`reg_planar`** (D8.reg): a nonzero affine function is regular everywhere.
* **`thin_planar`** (D5.thin): when the arrangement's face points off its facets lie on finitely
  many lines, they have empty interior and do not cut small balls.
* **`cells_planar`** (D8.cells, its connectivity half): a zone (a ball) minus finitely many lines
  is preconnected.

The curved-case rows stay open.
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology Module ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeExact ForgeExact.DecideOnce
  ForgeExact.DecideOnce.Curved

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- Where two affine functions vanish: `ℓ₁ = c₁` and `ℓ₂ = c₂`. -/
structure Meet (E : Type) [NormedAddCommGroup E] [NormedSpace ℝ E] where
  l1 : E →L[ℝ] ℝ
  l2 : E →L[ℝ] ℝ
  c1 : ℝ
  c2 : ℝ

/-- Its points. -/
def Meet.pts (M : Meet E) : Set E := {x | M.l1 x = M.c1 ∧ M.l2 x = M.c2}

/-- Its functionals are independent (the two planes are not parallel). -/
def Meet.Indep (M : Meet E) : Prop := LinearIndependent ℝ ![M.l1, M.l2]

/-- The hyperplane through `p` where `ℓ` vanishes. -/
def hyperAt (ℓ : E →L[ℝ] ℝ) (p : E) : Set E := {y | ℓ (y - p) = 0}

theorem hyperAt_closed (ℓ : E →L[ℝ] ℝ) (p : E) : IsClosed (hyperAt ℓ p) :=
  isClosed_eq (ℓ.continuous.comp (continuous_id.sub continuous_const)) continuous_const

/-- A hyperplane has empty interior. -/
theorem hyperAt_interior {ℓ : E →L[ℝ] ℝ} (hℓ : ℓ ≠ 0) (p : E) : interior (hyperAt ℓ p) = ∅ := by
  rw [eq_empty_iff_forall_notMem]
  intro y hy
  obtain ⟨v, hv⟩ : ∃ v, ℓ v ≠ 0 := by
    by_contra h; push Not at h; exact hℓ (ContinuousLinearMap.ext h)
  have hev : ∀ᶠ s in 𝓝 (0 : ℝ), y + s • v ∈ hyperAt ℓ p := by
    have hc : Continuous fun s : ℝ => y + s • v :=
      continuous_const.add (continuous_id.smul continuous_const)
    have h0 : (fun s : ℝ => y + s • v) 0 = y := by simp
    have := hc.continuousAt.preimage_mem_nhds
      (by rw [h0]; exact isOpen_interior.mem_nhds hy : interior (hyperAt ℓ p) ∈ 𝓝 ((fun s : ℝ => y + s • v) 0))
    exact Filter.mem_of_superset this fun s (hs : y + s • v ∈ interior (hyperAt ℓ p)) =>
      (interior_subset hs : y + s • v ∈ hyperAt ℓ p)
  have hy0 : y ∈ hyperAt ℓ p := interior_subset hy
  obtain ⟨s, hs, hs0⟩ := ((hev.filter_mono nhdsWithin_le_nhds).and
    (self_mem_nhdsWithin : ({0}ᶜ : Set ℝ) ∈ 𝓝[≠] (0:ℝ))).exists
  replace hs0 : s ≠ 0 := hs0
  simp only [hyperAt, mem_ofPred_eq] at hs hy0
  have : ℓ (y + s • v - p) = ℓ (y - p) + s * ℓ v := by
    rw [show y + s • v - p = (y - p) + s • v by abel, map_add, map_smul, smul_eq_mul]
  rw [this, hy0, zero_add] at hs
  exact (mul_ne_zero hs0 hv) hs

/-- **A hyperplane through a point off a meet, containing the meet.** -/
theorem hyper_through {M : Meet E} (hM : M.Indep) {p : E} (hp : p ∉ M.pts) :
    ∃ ℓ : E →L[ℝ] ℝ, ℓ ≠ 0 ∧ ∀ x ∈ M.pts, ℓ (x - p) = 0 := by
  set α := M.l2 p - M.c2
  set β := -(M.l1 p - M.c1)
  refine ⟨α • M.l1 + β • M.l2, fun h0 => hp ?_, fun x ⟨h1, h2⟩ => ?_⟩
  · obtain ⟨ha, hb⟩ := (LinearIndependent.pair_iff.mp hM) α β h0
    refine ⟨?_, ?_⟩
    · have : -(M.l1 p - M.c1) = 0 := hb
      linarith
    · have : M.l2 p - M.c2 = 0 := ha
      linarith
  · simp only [ContinuousLinearMap.add_apply, ContinuousLinearMap.smul_apply, smul_eq_mul, map_sub,
      h1, h2, α, β]
    ring

/-- A segment from `p` (off `M`) to a point off a hyperplane through `p` containing `M` misses
`M`. -/
theorem segment_avoid {M : Meet E} {p m : E} {ℓ : E →L[ℝ] ℝ}
    (hℓ : ∀ x ∈ M.pts, ℓ (x - p) = 0) (hp : p ∉ M.pts) (hm : m ∉ hyperAt ℓ p) :
    Disjoint (segment ℝ p m) M.pts := by
  rw [Set.disjoint_left]
  rintro x ⟨α, β, hα, hβ, hαβ, rfl⟩ hx
  by_cases hb : β = 0
  · subst hb
    simp only [add_zero, zero_smul] at hx hαβ
    rw [hαβ, one_smul] at hx
    exact hp hx
  · apply hm
    have hxp : α • p + β • m - p = β • (m - p) := by
      rw [show α = 1 - β by linarith]; simp only [sub_smul, one_smul, smul_sub]; abel
    have h1 := hℓ _ hx
    rw [hxp, map_smul, smul_eq_mul] at h1
    exact (mul_eq_zero.mp h1).resolve_left hb

/-- An open set minus finitely many closed sets with empty interior: if it was nonempty, still
nonempty. -/
theorem open_diff_list {X : Type} (L : List X) (f : X → Set E)
    (hf : ∀ x ∈ L, IsClosed (f x) ∧ interior (f x) = ∅) :
    ∀ {U : Set E}, IsOpen U → U.Nonempty → (U \ ⋃ x ∈ L, f x).Nonempty := by
  induction L with
  | nil => intro U _ hne; simpa using hne
  | cons x xs ih =>
    intro U hU hne
    obtain ⟨hc, hi⟩ := hf x (List.mem_cons_self ..)
    have h1 := diff_nonempty_of_open hU hne hi
    obtain ⟨y, hy⟩ := ih (fun z hz => hf z (List.mem_cons_of_mem _ hz)) (hU.sdiff hc) h1
    refine ⟨y, hy.1.1, ?_⟩
    simp only [List.mem_cons, iUnion_iUnion_eq_or_left, mem_union, not_or]
    exact ⟨hy.1.2, hy.2⟩

/-- **A convex open set minus finitely many meets of independent pairs is preconnected.** -/
theorem compl_meets_preconnected {U : Set E} (hU : IsOpen U) (hc : Convex ℝ U) (L : List (Meet E))
    (hind : ∀ M ∈ L, M.Indep) : IsPreconnected (U \ ⋃ M ∈ L, M.pts) := by
  apply isPreconnected_of_forall_pair
  rintro p ⟨hpU, hpA⟩ q ⟨hqU, hqA⟩
  simp only [mem_iUnion, not_exists] at hpA hqA
  have hp' : ∀ M : Meet E, ∃ ℓ : E →L[ℝ] ℝ, M ∈ L → ℓ ≠ 0 ∧ ∀ x ∈ M.pts, ℓ (x - p) = 0 :=
    fun M => by
      by_cases h : M ∈ L
      · obtain ⟨ℓ, h1, h2⟩ := hyper_through (hind M h) (hpA M h); exact ⟨ℓ, fun _ => ⟨h1, h2⟩⟩
      · exact ⟨0, fun h' => absurd h' h⟩
  have hq' : ∀ M : Meet E, ∃ ℓ : E →L[ℝ] ℝ, M ∈ L → ℓ ≠ 0 ∧ ∀ x ∈ M.pts, ℓ (x - q) = 0 :=
    fun M => by
      by_cases h : M ∈ L
      · obtain ⟨ℓ, h1, h2⟩ := hyper_through (hind M h) (hqA M h); exact ⟨ℓ, fun _ => ⟨h1, h2⟩⟩
      · exact ⟨0, fun h' => absurd h' h⟩
  choose ℓp hℓp using hp'
  choose ℓq hℓq using hq'
  obtain ⟨m, hmU, hmB⟩ := open_diff_list L (fun M => hyperAt (ℓp M) p ∪ hyperAt (ℓq M) q)
    (fun M hM => ⟨(hyperAt_closed _ _).union (hyperAt_closed _ _), by
      rw [interior_union_isClosed_of_interior_empty (hyperAt_closed _ _)
        (hyperAt_interior (hℓq M hM).1 q)]
      exact hyperAt_interior (hℓp M hM).1 p⟩) hU ⟨p, hpU⟩
  simp only [mem_iUnion, not_exists, mem_union, not_or] at hmB
  refine ⟨segment ℝ p m ∪ segment ℝ m q, ?_, Or.inl (left_mem_segment ℝ p m),
    Or.inr (right_mem_segment ℝ m q), ?_⟩
  · rintro x (hx | hx)
    · refine ⟨hc.segment_subset hpU hmU hx, ?_⟩
      simp only [mem_iUnion, not_exists]
      intro M hM hxM
      exact Set.disjoint_left.mp (segment_avoid (hℓp M hM).2 (hpA M hM) (hmB M hM).1) hx hxM
    · refine ⟨hc.segment_subset hmU hqU hx, ?_⟩
      simp only [mem_iUnion, not_exists]
      intro M hM hxM
      rw [segment_symm] at hx
      exact Set.disjoint_left.mp (segment_avoid (hℓq M hM).2 (hqA M hM) (hmB M hM).2) hx hxM
  · exact (convex_segment p m).isPreconnected.union m (right_mem_segment ℝ p m)
      (left_mem_segment ℝ m q) (convex_segment m q).isPreconnected

theorem Meet.closed (M : Meet E) : IsClosed M.pts :=
  (isClosed_eq M.l1.continuous continuous_const).inter (isClosed_eq M.l2.continuous continuous_const)

theorem Meet.interior (M : Meet E) (hM : M.Indep) : interior M.pts = ∅ := by
  rcases M.pts.eq_empty_or_nonempty with h | ⟨a, ha⟩
  · rw [h, interior_empty]
  · have hl1 : M.l1 ≠ 0 := fun h0 => by
      have := (LinearIndependent.pair_iff.mp hM) 1 0 (by rw [h0]; simp)
      exact one_ne_zero this.1
    refine eq_empty_of_subset_empty ((interior_mono fun x hx => ?_).trans
      (hyperAt_interior hl1 a).subset)
    simp only [hyperAt, mem_ofPred_eq, map_sub, hx.1, ha.1, sub_self]

/-- Removing part of finitely many meets from a convex open set leaves it preconnected. -/
theorem preconnected_off_meets {U : Set E} (hU : IsOpen U) (hc : Convex ℝ U) (L : List (Meet E))
    (hind : ∀ M ∈ L, M.Indep) {N : Set E} (hN : N ⊆ ⋃ M ∈ L, M.pts) :
    IsPreconnected (U \ N) := by
  refine (compl_meets_preconnected hU hc L hind).subset_closure
    (diff_subset_diff_right hN) fun z hz => ?_
  rw [mem_closure_iff]
  intro W hW hzW
  obtain ⟨y, hy, hyA⟩ := open_diff_list L (fun M => M.pts) (fun M hM => ⟨M.closed, M.interior (hind M hM)⟩)
    (hW.inter hU) ⟨z, hzW, hz.1⟩
  exact ⟨y, hy.1, hy.2, hyA⟩

/-- Part of finitely many meets has empty interior. -/
theorem interior_off_meets (L : List (Meet E)) (hind : ∀ M ∈ L, M.Indep) {N : Set E}
    (hN : N ⊆ ⋃ M ∈ L, M.pts) : interior N = ∅ := by
  rw [eq_empty_iff_forall_notMem]
  intro y hy
  obtain ⟨z, hz, hzA⟩ := open_diff_list L (fun M => M.pts)
    (fun M hM => ⟨M.closed, M.interior (hind M hM)⟩) isOpen_interior ⟨y, hy⟩
  exact hzA (hN (interior_subset hz))

/-- **D5.thin, planar**: when the arrangement's face points off its facets lie on finitely many
meets of independent pairs of planes (its edges and vertices), they are thin. -/
theorem thin_planar {a : Arr E} (L : List (Meet E)) (hind : ∀ M ∈ L, M.Indep)
    (hoff : OffFacets a ⊆ ⋃ M ∈ L, M.pts) : Thin a := by
  refine ⟨interior_off_meets L hind hoff, fun x _ U hU => ?_⟩
  obtain ⟨r, hr, hball⟩ := Metric.mem_nhds_iff.mp hU
  exact ⟨Metric.ball x r, hball, Metric.isOpen_ball, Metric.mem_ball_self hr,
    preconnected_off_meets Metric.isOpen_ball (convex_ball x r) L hind hoff⟩

/-- **D8.cells, planar (its connectivity)**: a zone, a ball, minus points on finitely many meets
of independent pairs of planes is preconnected. -/
theorem cells_planar (v : E) (r : ℝ) (L : List (Meet E)) (hind : ∀ M ∈ L, M.Indep) {J : Set E}
    (hJ : J ⊆ ⋃ M ∈ L, M.pts) : IsPreconnected (Metric.ball v r \ J) :=
  preconnected_off_meets Metric.isOpen_ball (convex_ball v r) L hind hJ

/-- An affine function with a nonzero linear part. -/
def Affine (c : E → ℝ) : Prop := ∃ ℓ : E →L[ℝ] ℝ, ℓ ≠ 0 ∧ ∃ k : ℝ, ∀ x, c x = ℓ x + k

/-- **D8.reg, planar**: an affine carrier is regular everywhere. -/
theorem reg_planar {c : E → ℝ} (hc : Affine c) (y : E) :
    ∃ L : E →L[ℝ] ℝ, HasStrictFDerivAt c L y ∧ L ≠ 0 := by
  obtain ⟨ℓ, hℓ, k, hk⟩ := hc
  have : c = fun x => ℓ x + k := funext hk
  subst this
  exact ⟨ℓ, (ℓ.hasStrictFDerivAt (x := y)).add_const k, hℓ⟩

/-- Two affine carriers with independent linear parts vanish together on a meet. -/
theorem zeros_meet {c1 c2 : E → ℝ} {ℓ1 ℓ2 : E →L[ℝ] ℝ} {k1 k2 : ℝ} (h1 : ∀ x, c1 x = ℓ1 x + k1)
    (h2 : ∀ x, c2 x = ℓ2 x + k2) :
    {x | c1 x = 0 ∧ c2 x = 0} = (⟨ℓ1, ℓ2, -k1, -k2⟩ : Meet E).pts := by
  ext x
  simp only [mem_ofPred_eq, Meet.pts, h1, h2]
  constructor
  · rintro ⟨a, b⟩; exact ⟨by linarith, by linarith⟩
  · rintro ⟨a, b⟩; exact ⟨by linarith, by linarith⟩

/-! ## The D8 theorems for flat-faced bodies -/

/-- **D8.orient, planar**: with affine carriers, the regularity hypothesis (D8.reg) is gone and
the thinness (D5.thin) is the arrangement's off-facet points lying on finitely many meets. -/
theorem oriented_result_planar {Aex Bex : BodyModel E} {a : Arr E} (hvA : ValidExact Aex)
    (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a) (L : List (Meet E)) (hind : ∀ M ∈ L, M.Indep)
    (hoff : OffFacets a ⊆ ⋃ M ∈ L, M.pts) (op : Op) {c : Nat → Class}
    (hc : ClassesOK a (denoteExact Aex) (denoteExact Bex) c)
    {k : Nat → Bool} (hk : KeepOK a op c k)
    (hK : KeptIsBd a k (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op))
    {Rs : BodyModel E} (hwk : WritesKept a k Rs) (hw : WritesSense a op k Rs)
    (hrs : ∀ f ∈ a.regions, k f = true → RegionSense Aex Bex a f)
    (hcl : ∀ G ∈ Rs.faces, IsClosed G.region)
    (hdense : ∀ F ∈ Rs.faces, F.region ⊆ closure (F.region ∩ FacetPts a.G))
    (haff : ∀ F ∈ Rs.faces, Affine F.carrier) :
    ∀ F ∈ Rs.faces, ∀ y, Alone Rs F y →
      ForgeCurved.F6.Oriented F.carrier (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op))
        F.sense y :=
  oriented_result_of_writes hvA hvB ha (thin_planar L hind hoff) op hc hk hK hwk hw hrs hcl hdense
    fun F hF y _ _ => reg_planar (haff F hF) y

/-- **D8.seeded, planar**: thinness from the meets. -/
theorem seeded_result_planar [Nontrivial E] {Aex Bex Rs : BodyModel E} {a : Arr E}
    (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a) (L : List (Meet E))
    (hind : ∀ M ∈ L, M.Indep) (hoff : OffFacets a ⊆ ⋃ M ∈ L, M.pts)
    {k : Nat → Bool} (hw : WritesKept a k Rs) (hlone : LoneOnFacets a Rs) : Seeded Rs :=
  seeded_result hvA hvB ha (thin_planar L hind hoff) hw hlone

/-- **D8.formula, planar**: in a zone (a ball) the connectivity of D8.cells holds once the points
where two incident carriers vanish lie on finitely many meets of independent pairs (`zeros_meet`:
two incident planes that are not parallel meet on one). -/
theorem local_formula_planar {Rs : BodyModel E} {S : Set E} (hS : IsClosed S) (hwf : WellFormed Rs)
    {u : Nat} {Φ : SForm} (hΦ : Rs.formulas[u]? = some Φ) (v : E) (r : ℝ) (J : Set E)
    (hJ : J = {y | ∃ (i j : Nat) (F G : FaceModel E), i ≠ j ∧ Rs.faces[i]? = some F ∧
      Rs.faces[j]? = some G ∧ IncV Rs u F ∧ IncV Rs u G ∧
      {x | F.carrier x = 0} ≠ {x | G.carrier x = 0} ∧ F.carrier y = 0 ∧ G.carrier y = 0} ∪
      (Metric.ball v r ∩ ⋃ e ∈ Rs.edges, e.curve))
    -- every edge is used by a face (the combinatorial check: `EdgeUses`)
    (hused : ∀ (i : Nat) (e : EdgeModel E), Rs.edges[i]? = some e → ∃ f ∈ Rs.faces, IncE f i)
    (L : List (Meet E)) (hind : ∀ M ∈ L, M.Indep) (hJL : J ⊆ ⋃ M ∈ L, M.pts)
    (hiso : ∀ F ∈ Rs.faces, ∀ y ∈ F.region, y ∈ Metric.ball v r → IncV Rs u F)
    (hfrR : frontier S ⊆ Faces Rs)
    (hor : ∀ F ∈ Rs.faces, ∀ y, Alone Rs F y → ForgeCurved.F6.Oriented F.carrier S F.sense y)
    (hcone : ∀ F ∈ Rs.faces, IncV Rs u F → ∀ y ∈ Metric.ball v r \ J, y ∈ F.region →
      ∀ᶠ z in 𝓝 y, (ForgeCurved.F6.inSide F.sense (F.carrier z) → Φ.den (carriers Rs) z) ∧
        (ForgeCurved.F6.outSide F.sense (F.carrier z) → ¬ Φ.den (carriers Rs) z))
    (hext : ∀ y ∈ Metric.ball v r \ J, (∀ F ∈ Rs.faces, IncV Rs u F → y ∉ F.region) →
      ∃ N ∈ 𝓝 y, ∀ z ∈ N, ∀ z' ∈ N, (∀ F ∈ Rs.faces, IncV Rs u F → F.carrier z ≠ 0) →
        (∀ F ∈ Rs.faces, IncV Rs u F → F.carrier z' ≠ 0) →
        (Φ.den (carriers Rs) z ↔ Φ.den (carriers Rs) z'))
    (hbase : ∃ F ∈ Rs.faces, IncV Rs u F ∧ ∃ y ∈ F.region, y ∈ Metric.ball v r \ J) :
    ∀ x ∈ Metric.ball v r, (∀ F ∈ Rs.faces, IncV Rs u F → F.carrier x ≠ 0) →
      (x ∈ S ↔ Φ.den (carriers Rs) x) :=
  local_formula_of_cone hS hwf hΦ Metric.isOpen_ball J hJ hused hiso hfrR hor hcone hext
    (cells_planar v r L hind hJL) hbase

end ForgeBoolProof
