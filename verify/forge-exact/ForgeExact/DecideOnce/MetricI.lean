import ForgeExact.DecideOnce.Simulation

/-!
# A8 L5(b): the metric collapse over an incidence ledger (rooms)

Metric.lean's collapse, ported line for line to `IncLedger`. Thin and enclosed are tables per
room. The combinatorial theorems carry over: decided once, local (`collapse_local`), one feature
one answer (`collapse_comp`, `collapse_uniform`), idempotent (`collapse_idem`), refusal at the spot
(`collapse_refuse`) and only deletes (`collapse_subset`). The core with the metric ledger,
`coreMI`, is `coreK` on the collapsed room table. By `coreK_refines`, it corresponds to the fine
core on any fine table that agrees with it facet-side by facet-side.
-/

set_option linter.unusedSectionVars false

namespace ForgeExact.DecideOnce

universe u

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] {L : IncLedger α}

/-! ## The collapse -/


/-- Two side cells on either side of a facet. -/
def tAdj (T U : α) : Bool :=
  L.facets.any fun φ => ((L.plus φ) == T && (L.minus φ) == U) || ((L.plus φ) == U && (L.minus φ) == T)

/-- The thin side cells, in arena order. -/
def thinTopes (thin : List α) : List α := L.rooms.filter fun T => thin.contains T

/-- The feature a thin cell belongs to: its class among the thin cells. -/
def comp (thin : List α) (T : α) : List α := cls L.tAdj (L.thinTopes thin) T

/-- The feature's rim: the cells that are not thin, adjacent to it. -/
def rim (thin : List α) (T : α) : List α :=
  L.rooms.filter fun U => !thin.contains U && (L.comp thin T).any fun V => L.tAdj V U

/-- The rim is kept alike. -/
def uniform (kt : KeepTabI α) (thin : List α) (T : α) : Bool :=
  (L.rim thin T).all fun U => keepInI kt U == keepInI kt ((L.rim thin T).headD T)

/-- The keep a thin cell takes: its rim's (or its own, if it has no rim). -/
def fill (kt : KeepTabI α) (thin : List α) (T : α) : Bool :=
  match L.rim thin T with
  | [] => keepInI kt T
  | U :: _ => keepInI kt U

inductive CollapseI (α : Type u)
  | ok (kt : KeepTabI α)
  | refuse (cell : α)

/-- A feature can be collapsed: its rim is kept alike and every cell of it is enclosed by pieces
(so no facet the core does not hold can become boundary). -/
def collapsible (kt : KeepTabI α) (thin enclosed : List α) (T : α) : Bool :=
  L.uniform kt thin T && (L.comp thin T).all fun V => enclosed.contains V

/-- **The collapse.** -/
def collapse (kt : KeepTabI α) (thin enclosed : List α) : CollapseI α :=
  match (L.thinTopes thin).find? fun T => !L.collapsible kt thin enclosed T with
  | some T => .refuse T
  | none => .ok (L.rooms.map fun T => (T, if thin.contains T then L.fill kt thin T else keepInI kt T))

/-- **The core with the metric ledger.** -/
def coreMI (rule : Rule) (thin enclosed : List α) : OutcomeI α :=
  if !L.splitOk then .refused .splitCrossed
  else if !L.classifyOk then .refused .inconsistentClassification
  else match L.collapse (L.keepTab rule) thin enclosed with
    | .refuse T => .refused (.zeroThickness T)
    | .ok kt' => L.coreK kt'


/-! ## The rule's properties -/

theorem keepIn_map (f : α → Bool) {xs : List α} {T : α} (hT : T ∈ xs) :
    keepInI (xs.map fun x => (x, f x)) T = f T := by
  unfold keepInI
  rw [IncLedger.find?_pair' f T xs hT]
  rfl

theorem collapsible_of_ok {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt')
    {T : α} (hT : T ∈ L.thinTopes thin) : L.collapsible kt thin enc T = true := by
  unfold collapse at h
  split at h
  · cases h
  · rename_i hf
    have := List.find?_eq_none.mp hf T hT
    simpa using this

theorem collapsed_at {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt')
    {T : α} (hT : T ∈ L.rooms) :
    keepInI kt' T = if thin.contains T then L.fill kt thin T else keepInI kt T := by
  unfold collapse at h
  split at h
  · cases h
  · cases h
    exact keepIn_map (fun T => if thin.contains T then L.fill kt thin T else keepInI kt T) hT

/-- **Local**: a cell that is not thin keeps its keep. -/
theorem collapse_local {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt')
    {T : α} (hT : T ∈ L.rooms) (hn : thin.contains T = false) : keepInI kt' T = keepInI kt T := by
  rw [collapsed_at h hT, hn]; rfl

theorem rim_not_thin {thin : List α} {T U : α} (h : U ∈ L.rim thin T) :
    U ∈ L.rooms ∧ thin.contains U = false := by
  unfold rim at h
  simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true'] at h
  exact ⟨h.1, h.2.1⟩

theorem uniform_iff {kt : KeepTabI α} {thin : List α} {T : α} :
    L.uniform kt thin T = true ↔ ∀ U ∈ L.rim thin T, keepInI kt U = keepInI kt ((L.rim thin T).headD T) := by
  unfold uniform
  simp [List.all_eq_true]

/-- **The rim decides**: past the collapse, a thin cell with a rim has its rim's keep, the same
for every rim cell. -/
theorem collapse_uniform {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt')
    {T : α} (hT : T ∈ L.thinTopes thin) : ∀ U ∈ L.rim thin T, keepInI kt' T = keepInI kt U := by
  have hu : L.uniform kt thin T = true := by
    have := collapsible_of_ok h hT
    unfold collapsible at this
    exact (Bool.and_eq_true _ _).mp this |>.1
  have hT' : T ∈ L.rooms ∧ thin.contains T = true := by
    unfold thinTopes at hT; simpa using hT
  intro U hU
  rw [collapsed_at h hT'.1, hT'.2]
  simp only [ite_true]
  unfold fill
  have hall := uniform_iff.mp hu
  split
  · rename_i hnil; rw [hnil] at hU; exact nomatch hU
  · rename_i U0 rest hcons
    have h1 := hall U hU
    rw [hcons] at h1
    simp only [List.headD_cons] at h1
    exact h1.symm

/-- **Refused at the spot**: a refusal names a thin cell whose feature's rim is kept both ways —
material and empty space less than LINEAR apart — or whose feature is not enclosed by pieces. -/
theorem collapse_refuse {kt : KeepTabI α} {thin enc : List α} {T : α} (h : L.collapse kt thin enc = .refuse T) :
    T ∈ L.thinTopes thin ∧
      ((∃ U ∈ L.rim thin T, ∃ V ∈ L.rim thin T, keepInI kt U ≠ keepInI kt V) ∨
       (∃ V ∈ L.comp thin T, enc.contains V = false)) := by
  unfold collapse at h
  split at h
  · rename_i T' hf
    cases h
    refine ⟨List.mem_of_find?_eq_some hf, ?_⟩
    have hn := List.find?_some hf
    simp only [Bool.not_eq_true', collapsible, Bool.and_eq_false_iff] at hn
    rcases hn with hn | hn
    · left
      apply Classical.byContradiction
      intro hno
      have : L.uniform kt thin T = true := by
        apply uniform_iff.mpr
        intro U hU
        apply Classical.byContradiction
        intro hne
        have hm : (L.rim thin T).headD T ∈ L.rim thin T := by
          cases hr : L.rim thin T with
          | nil => rw [hr] at hU; exact nomatch hU
          | cons U0 rest => simp
        exact hno ⟨U, hU, _, hm, hne⟩
      rw [this] at hn; cases hn
    · right
      simp only [List.all_eq_false] at hn
      obtain ⟨V, hV, hne⟩ := hn
      exact ⟨V, hV, by simpa using hne⟩
  · cases h

theorem tAdj_symm (L : IncLedger α) : ∀ T U, L.tAdj T U = L.tAdj U T := by
  intro T U
  unfold tAdj
  congr 1; funext φ
  cases (L.plus φ) == T <;> cases (L.minus φ) == U <;> cases (L.plus φ) == U <;> cases (L.minus φ) == T <;> rfl

theorem thinTopes_nodup (L : IncLedger α) (thin : List α) : (L.thinTopes thin).Nodup :=
  (nodup_dedup _).filter _

/-- **One feature, one answer**: cells of one component have one rim, so one keep. -/
theorem rim_comp {thin : List α} {T T' : α} (hT : T ∈ L.thinTopes thin)
    (hc : Conn L.tAdj (L.thinTopes thin) T T') : L.rim thin T = L.rim thin T' := by
  unfold rim comp
  congr 1; funext U
  congr 1
  have he := cls_eq L.tAdj (tAdj_symm L) (thinTopes_nodup L thin) hT hc
  cases h1 : (cls L.tAdj (L.thinTopes thin) T).any fun V => L.tAdj V U <;>
    cases h2 : (cls L.tAdj (L.thinTopes thin) T').any fun V => L.tAdj V U
  · rfl
  · exfalso
    simp only [List.any_eq_true, List.any_eq_false] at h1 h2
    obtain ⟨V, hV, hVU⟩ := h2
    exact h1 V ((he V).mpr hV) hVU
  · exfalso
    simp only [List.any_eq_true, List.any_eq_false] at h1 h2
    obtain ⟨V, hV, hVU⟩ := h1
    exact h2 V ((he V).mp hV) hVU
  · rfl

theorem collapse_comp {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt')
    {T T' : α} (hT : T ∈ L.thinTopes thin) (hc : Conn L.tAdj (L.thinTopes thin) T T') :
    keepInI kt' T = keepInI kt' T' ∨ L.rim thin T = [] := by
  have hT' : T' ∈ L.thinTopes thin := Conn.mem hT hc
  cases hr : L.rim thin T with
  | nil => exact Or.inr rfl
  | cons U rest =>
    left
    have hU : U ∈ L.rim thin T := by rw [hr]; exact List.mem_cons_self ..
    rw [collapse_uniform h hT U hU, collapse_uniform h hT' U (by rw [← rim_comp hT hc]; exact hU)]

/-- **Idempotent**: the collapsed table collapses to itself. -/
theorem collapse_idem {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt') :
    L.collapse kt' thin enc = .ok kt' := by
  -- rim cells are not thin, so they keep their keep; so every rim stays uniform and every
  -- thin cell's fill is unchanged
  have hrim : ∀ T U, U ∈ L.rim thin T → keepInI kt' U = keepInI kt U := fun T U hU =>
    collapse_local h (rim_not_thin hU).1 (rim_not_thin hU).2
  have hfill : ∀ T, T ∈ L.thinTopes thin → L.fill kt' thin T = L.fill kt thin T ∨ L.rim thin T = [] := by
    intro T _
    unfold fill
    cases hr : L.rim thin T with
    | nil => exact Or.inr rfl
    | cons U rest => left; exact hrim T U (by rw [hr]; exact List.mem_cons_self ..)
  have hu : ∀ T ∈ L.thinTopes thin, L.uniform kt' thin T = true := by
    intro T hT
    have hu0 : L.uniform kt thin T = true := by
      have := collapsible_of_ok h hT
      unfold collapsible at this
      exact (Bool.and_eq_true _ _).mp this |>.1
    apply uniform_iff.mpr
    intro U hU
    have hall := uniform_iff.mp hu0 U hU
    rw [hrim T U hU, hall]
    cases hr : L.rim thin T with
    | nil => rw [hr] at hU; exact nomatch hU
    | cons U0 rest =>
      simp only [List.headD_cons]
      rw [hrim T U0 (by rw [hr]; exact List.mem_cons_self ..)]
  have hnone : (L.thinTopes thin).find? (fun T => !L.collapsible kt' thin enc T) = none := by
    apply List.find?_eq_none.mpr
    intro T hT
    have hc := collapsible_of_ok h hT
    unfold collapsible at hc ⊢
    rw [hu T hT, ((Bool.and_eq_true _ _).mp hc).2]
    simp
  unfold collapse
  rw [hnone]
  simp only
  congr 1
  -- the tables agree entry by entry
  have hkt' : kt' = L.rooms.map fun T => (T, if thin.contains T then L.fill kt thin T else keepInI kt T) := by
    unfold collapse at h
    split at h
    · cases h
    · cases h; rfl
  conv => rhs; rw [hkt']
  apply List.map_congr_left
  intro T hT
  simp only [Prod.mk.injEq, true_and]
  rw [collapsed_at h hT]
  cases hc : thin.contains T
  · rfl
  · simp only [ite_true]
    have hTt : T ∈ L.thinTopes thin := by
      unfold thinTopes; exact List.mem_filter.mpr ⟨hT, hc⟩
    rcases hfill T hTt with e | e
    · exact e
    · unfold fill; rw [e]; simp only
      rw [collapsed_at h hT, hc]; simp only [ite_true]; unfold fill; rw [e]

theorem mem_thinTopes {thin : List α} {T : α} (hT : T ∈ L.rooms) (hc : thin.contains T = true) :
    T ∈ L.thinTopes thin := List.mem_filter.mpr ⟨hT, hc⟩

theorem tAdj_facet {φ : α} (hφ : φ ∈ L.facets) : L.tAdj (L.plus φ) (L.minus φ) = true := by
  unfold tAdj
  exact List.any_eq_true.mpr ⟨φ, hφ, by simp⟩

/-- The other side of a facet of a feature, if not thin, is on the feature's rim. -/
theorem in_rim {thin : List α} {T U : α} (hT : T ∈ L.thinTopes thin) (hU : U ∈ L.rooms)
    (hn : thin.contains U = false) (ha : L.tAdj T U = true) : U ∈ L.rim thin T := by
  unfold rim
  refine List.mem_filter.mpr ⟨hU, ?_⟩
  simp only [hn, Bool.not_false, Bool.true_and, List.any_eq_true]
  exact ⟨T, self_mem_cls _ (thinTopes_nodup L thin) hT, ha⟩

/-- **The collapse only deletes**: every facet of the collapsed result was a facet of the result,
and no facet with a side in a feature that has a rim survives. -/
theorem collapse_subset {kt kt' : KeepTabI α} {thin enc : List α} (h : L.collapse kt thin enc = .ok kt')
    {φ : α} (hφ : φ ∈ L.result kt') : φ ∈ L.result kt := by
  rw [IncLedger.mem_result] at hφ ⊢
  obtain ⟨hf, hk⟩ := hφ
  refine ⟨hf, ?_⟩
  have hp := L.plus_mem_rooms hf
  have hm := L.minus_mem_rooms hf
  have ha := tAdj_facet (L := L) hf
  have ha' : L.tAdj (L.minus φ) (L.plus φ) = true := by rw [tAdj_symm]; exact ha
  cases cp : thin.contains (L.plus φ) <;> cases cm : thin.contains (L.minus φ)
  · rwa [collapse_local h hp cp, collapse_local h hm cm] at hk
  · -- minus thin, plus on its rim
    have hT := mem_thinTopes hm cm
    have := collapse_uniform h hT (L.plus φ) (in_rim hT hp cp ha')
    rw [this, collapse_local h hp cp] at hk
    exact absurd rfl hk
  · have hT := mem_thinTopes hp cp
    have := collapse_uniform h hT (L.minus φ) (in_rim hT hm cm ha)
    rw [this, collapse_local h hm cm] at hk
    exact absurd rfl hk
  · -- both thin: one feature
    have hT := mem_thinTopes hp cp
    have hT' := mem_thinTopes hm cm
    have hconn : Conn L.tAdj (L.thinTopes thin) (L.plus φ) (L.minus φ) := Conn.step Conn.refl hT' ha
    rcases collapse_comp h hT hconn with e | e
    · exact absurd e hk
    · -- no rim: both keep their own keep
      have e' : L.rim thin (L.minus φ) = [] := by rw [← rim_comp hT hconn]; exact e
      rw [collapsed_at h hp, cp, collapsed_at h hm, cm] at hk
      simp only [ite_true, fill, e, e'] at hk
      exact hk


end IncLedger

end ForgeExact.DecideOnce

namespace ForgeExact.DecideOnce

namespace Refines

universe v w

variable {α : Type v} {β : Type w} [DecidableEq α] [BEq α] [LawfulBEq α] [DecidableEq β] [BEq β] [LawfulBEq β]
  {A : IncLedger α} {B : IncLedger β} {σ : α → β} {τ ν : α → Option β} {κ : α → Sector α → Sector β}

/-- **Exact away from the thin rooms.** After the coarse collapse, a coarse facet neither of whose
rooms is thin is in the result iff its fine preimages are in the fine (uncollapsed) result. So the
collapsed face ledger differs from the exact boundary only at thin rooms, each within LINEAR by the
metric ledger's meaning. -/
theorem collapse_exact_off_thin (h : Refines A B σ τ ν κ) (rule : Rule) {thin enc : List β} {kt' : KeepTabI β}
    (hc : B.collapse (B.keepTab rule) thin enc = .ok kt') {φ : α} (hφ : φ ∈ A.facets)
    (hp : thin.contains (B.plus (σ φ)) = false) (hm : thin.contains (B.minus (σ φ)) = false) :
    σ φ ∈ B.result kt' ↔ φ ∈ A.result (A.keepTab rule) := by
  have hψ := h.facet_mem φ hφ
  have e : (σ φ ∈ B.result kt') ↔ (σ φ ∈ B.result (B.keepTab rule)) := by
    rw [IncLedger.mem_result, IncLedger.mem_result,
      IncLedger.collapse_local hc (B.plus_mem_rooms hψ) hp, IncLedger.collapse_local hc (B.minus_mem_rooms hψ) hm]
  rw [e]
  exact result_iff h (keepCorr_tab h rule) hφ

end Refines

end ForgeExact.DecideOnce
