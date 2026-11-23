import ForgeCurved.Cells
import ForgeExact.DecideOnce.CorrectK

/-!
# The result's boundary, from the cells

`R := reg (combine (ruleOp rule) (opndsK L A))` is the regularised boolean, L1's answer, a set and
nothing else. Over an exact curved ledger of continuous carriers (`ExactK`) and with both sides of
each listed facet beside it (`Faithful.sides`, R3):

* `R_eq`: `R` is the closure of the kept full cells (the core's `keep` table, read as a set);
* `not_frontier_of_full`: a point off every carrier is not on `R`'s frontier;
* **`frontier_iff`**: a point of a facet cell is on `R`'s frontier iff the facet is an exact
  boundary facet (`XBoundaryK`): its two sides are kept differently. The core's `result` is read
  off the set `R` itself.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Ledger Filter Topology

variable {P : Type} [TopologicalSpace P] {L : Ledger} {cs : List (Carrier P ℝ)} {A : Nat → Pred P}

/-- The kept full cells, as a set. -/
def Kset (L : Ledger) (cs : List (Carrier P ℝ)) (A : Nat → Pred P) (rule : Rule) : Set P :=
  {x | Sgn.zero ∉ covOfK cs x ∧ xkeepK L cs A rule (covOfK cs x) = true}

/-- **The regularised boolean** (L1), as a set. -/
def Rset (L : Ledger) (A : Nat → Pred P) (rule : Rule) : Set P :=
  {x | reg (combine (ruleOp rule) (opndsK L A)) x}

theorem Kset_open (hc : ∀ c ∈ cs, Continuous c) (rule : Rule) : IsOpen (Kset L cs A rule) := by
  rw [isOpen_iff_forall_mem_open]
  intro x hx
  refine ⟨cell cs (covOfK cs x), fun y hy => ?_, full_open hc hx.1, rfl⟩
  rw [mem_cell] at hy
  exact ⟨hy ▸ hx.1, hy ▸ hx.2⟩

theorem kept_iff (h : ExactK L cs A) (rule : Rule) (x : P) :
    (decompK h).kept (fun T => xkeepK L cs A rule T = true) x ↔ x ∈ Kset L cs A rule := by
  show (∃ T ∈ fulls cs.length, xkeepK L cs A rule T = true ∧ T ∈ fulls cs.length ∧ covCellK cs T x) ↔ _
  constructor
  · rintro ⟨T, hT, hk, -, hx⟩
    unfold covCellK at hx
    subst hx
    exact ⟨zero_not_mem_fulls hT, hk⟩
  · rintro ⟨hz, hk⟩
    have hT := mem_fullsK (length_covOfK cs x) hz
    exact ⟨_, hT, hk, hT, rfl⟩

/-- **`R` is the closure of the kept full cells.** -/
theorem R_eq (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (rule : Rule) :
    Rset L A rule = closure (Kset L cs A rule) := by
  unfold Rset
  rw [← (correct_k h rule).2]
  show {x | ForgeExact.Topology.closure (ForgeExact.Topology.interior _) x} = _
  rw [closure_eq, interior_eq]
  have : {y | (decompK h).kept (fun T => xkeepK L cs A rule T = true) y} = Kset L cs A rule := by
    ext y; exact kept_iff h rule y
  rw [this, (Kset_open hc rule).interior_eq]

theorem R_closed (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (rule : Rule) :
    IsClosed (Rset L A rule) := by
  rw [R_eq hc h rule]; exact isClosed_closure

/-- A full cell kept lies in `K`. -/
theorem cell_sub_K {T : Cov} (hT : Sgn.zero ∉ T) {rule : Rule} (hk : xkeepK L cs A rule T = true) :
    cell cs T ⊆ Kset L cs A rule := fun y hy => by
  rw [mem_cell] at hy; subst hy; exact ⟨hT, hk⟩

/-- A full cell not kept misses `R`. -/
theorem cell_disj_R (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) {T : Cov} (hT : Sgn.zero ∉ T)
    {rule : Rule} (hk : xkeepK L cs A rule T = false) : ∀ y ∈ cell cs T, y ∉ Rset L A rule := by
  intro y hy hyR
  rw [R_eq hc h rule] at hyR
  obtain ⟨w, hw, hwK⟩ := mem_closure_iff.mp hyR _ (full_open hc hT) hy
  rw [mem_cell] at hw
  have k2 := hwK.2
  rw [hw, hk] at k2; cases k2

theorem frontier_eq (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (rule : Rule) :
    frontier (Rset L A rule) = Rset L A rule \ interior (Rset L A rule) :=
  (R_closed hc h rule).frontier_eq

/-- **Off every carrier, no boundary.** -/
theorem not_frontier_of_full (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (rule : Rule)
    {x : P} (hx : Sgn.zero ∉ covOfK cs x) : x ∉ frontier (Rset L A rule) := by
  rw [frontier_eq hc h rule]
  rintro ⟨hR, hI⟩
  cases hk : xkeepK L cs A rule (covOfK cs x)
  · exact cell_disj_R hc h hx hk x rfl hR
  · apply hI
    rw [R_eq hc h rule]
    exact interior_maximal (fun y hy => subset_closure (cell_sub_K hx hk hy)) (full_open hc hx) rfl

/-- A boundary point lies on a carrier. -/
theorem zero_of_frontier (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (rule : Rule) {x : P}
    (hx : x ∈ frontier (Rset L A rule)) : Sgn.zero ∈ covOfK cs x :=
  Classical.byContradiction fun hz => not_frontier_of_full hc h rule hz hx

/-- A facet whose sides are kept alike has no boundary point. -/
theorem not_frontier_of_same (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (rule : Rule)
    {x : P} (hφ : (covOfK cs x).isFacet = true)
    (hs : xkeepK L cs A rule (covOfK cs x).plus = xkeepK L cs A rule (covOfK cs x).minus) :
    x ∉ frontier (Rset L A rule) := by
  obtain ⟨N, hNo, hxN, hN⟩ := exists_open_le hc x
  have hsd := facet_side_full hφ
  have side : ∀ y ∈ N, Sgn.zero ∉ covOfK cs y →
      covOfK cs y = (covOfK cs x).plus ∨ covOfK cs y = (covOfK cs x).minus :=
    fun y hy hz => full_above_facet (hN y hy) hφ hz
  rw [frontier_eq hc h rule]
  rintro ⟨hR, hI⟩
  cases hk : xkeepK L cs A rule (covOfK cs x).plus
  · -- neither side kept: `N` misses `K`
    rw [hk] at hs
    rw [R_eq hc h rule] at hR
    obtain ⟨y, hyN, hyK⟩ := mem_closure_iff.mp hR N hNo hxN
    have k2 := hyK.2
    rcases side y hyN hyK.1 with e | e <;> rw [e] at k2
    · rw [hk] at k2; cases k2
    · rw [← hs] at k2; cases k2
  · -- both kept: `N` lies in the closure of `K`
    rw [hk] at hs
    apply hI
    rw [R_eq hc h rule]
    refine interior_maximal (fun y hyN => ?_) hNo hxN
    rw [mem_closure_iff]
    intro O hO hyO
    obtain ⟨w, ⟨hwO, hwN⟩, hwz⟩ := exists_off_wall h.wallND (hO.inter hNo) ⟨hyO, hyN⟩
    refine ⟨w, hwO, hwz, ?_⟩
    rcases side w hwN hwz with e | e <;> rw [e]
    · exact hk
    · exact hs.symm

/-- **A point of an exact boundary facet is on `R`'s frontier** (R3: both sides beside it). -/
theorem frontier_of_boundary (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (rule : Rule) {φ : Cov} (hb : XBoundaryK L cs A rule φ) {x : P}
    (hx : covOfK cs x = φ) : x ∈ frontier (Rset L A rule) := by
  have hφF : φ ∈ L.facets := (mem_result.mp ((result_iff_k h rule φ).mpr hb)).1
  obtain ⟨_, hfa, hk⟩ := hb
  obtain ⟨hp, hm⟩ := hf.sides φ (Or.inl hφF) hfa x hx
  rw [closure_iff, cell_eq] at hp hm
  obtain ⟨fp, fm⟩ := facet_side_full hfa
  -- one side kept, one not
  have key : ∀ T U : Cov, Sgn.zero ∉ T → Sgn.zero ∉ U → xkeepK L cs A rule T = true →
      xkeepK L cs A rule U = false → x ∈ closure (cell cs T) → x ∈ closure (cell cs U) →
      x ∈ frontier (Rset L A rule) := by
    intro T U hT hU hkT hkU hxT hxU
    rw [frontier_eq hc h rule]
    refine ⟨?_, fun hI => ?_⟩
    · rw [R_eq hc h rule]; exact closure_mono (cell_sub_K hT hkT) hxT
    · obtain ⟨y, hyI, hyU⟩ := mem_closure_iff.mp hxU _ isOpen_interior hI
      exact cell_disj_R hc h hU hkU y hyU (interior_subset hyI)
  cases e : xkeepK L cs A rule φ.plus <;> cases e' : xkeepK L cs A rule φ.minus
  · rw [e, e'] at hk; exact absurd rfl hk
  · exact key _ _ fm fp e' e hm hp
  · exact key _ _ fp fm e e' hp hm
  · rw [e, e'] at hk; exact absurd rfl hk

/-- **A facet point is on the boundary iff its facet is an exact boundary facet.** -/
theorem frontier_iff (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (rule : Rule) {x : P} (hφ : (covOfK cs x).isFacet = true) :
    x ∈ frontier (Rset L A rule) ↔ XBoundaryK L cs A rule (covOfK cs x) := by
  constructor
  · intro hx
    exact ⟨⟨x, rfl⟩, hφ, fun hs => not_frontier_of_same hc h rule hφ hs hx⟩
  · intro hb; exact frontier_of_boundary hc h hf rule hb rfl

end ForgeCurved
