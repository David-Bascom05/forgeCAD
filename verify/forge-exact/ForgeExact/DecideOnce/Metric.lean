import ForgeExact.DecideOnce.Correct

/-!
# The metric ledger: §6.1 step 3, sub-LINEAR features of the result

After the exact operation, a result may be thinner than LINEAR somewhere: a sliver of material a
hair thick, or a crack a hair wide. §6.1 says the kernel must either collapse such a feature
validly, within LINEAR, or refuse with ZeroThickness there; silently making an invalid body is the
one forbidden answer.

**The metric ledger** is two more tables of decisions made once, per side cell: whether it is
thinner than LINEAR (`thin`), and whether every facet of it is a piece (`enclosed`), both decided
by the bridge from the cell's exact geometry (their metric meaning is the bridge's obligation,
decide-once-spec.md §6). The core reads them only as lists of cell keys.

**The rule** (`collapse`), a stage between keep and assemble:
* the thin cells fall into *components*: the classes of the thin cells under "adjacent across a
  facet" (`Closure.cls`, so a slab of several thin cells is one feature);
* a component's *rim* is the non-thin cells adjacent to it;
* if every component's rim is kept alike and every cell of it is enclosed, each thin cell takes its
  rim's keep (a void or crack inside material is filled, a speck inside empty space removed); a
  component with no rim keeps its own keep;
* otherwise the first such thin cell (in arena order) is refused `ZeroThickness`: its feature's
  rim is kept both ways — material and empty space less than LINEAR apart, a sub-LINEAR wall,
  fin or gap — or the feature reaches a facet the core does not hold.

**Proven** (`collapse_*`, `coverageM`, `correct_off_thin`):
* decided once: the rule is a function of the keep table and the thin table;
* *local*: no cell that is not thin changes;
* *one feature, one answer*: every cell of a component takes the same keep;
* *idempotent*: collapsing again changes nothing;
* *refusal at the spot*: a refusal names a thin cell whose component's rim is kept both ways or
  that is not enclosed;
* *only deletes*: every facet of the collapsed result was a facet of the result (`collapse_subset`),
  so a collapse never adds boundary;
* *valid*: past the collapse the core ends in empty, a valid body or a stated ZeroThickness,
  exactly as (b), for every consistent ledger and every thin table;
* *exact away from the features*: for an exact ledger, a facet neither of whose sides is thin is in
  the collapsed result iff it is an exact boundary facet (so the result differs from the exact
  one only on thin cells, each within LINEAR by the metric ledger's meaning).
-/

namespace ForgeExact.DecideOnce

open Ledger

namespace Ledger

variable (L : Ledger)

/-! ## The post-classification core, for any keep table -/

/-- The touch rule, unify and the empty test, on a given keep table. -/
def coreK (kt : KeepTab) : Outcome :=
  match L.edges.find? (L.badEdge kt) with
  | some e => .refused (.zeroThickness e)
  | none => match L.verts.find? (L.badVert kt) with
    | some v => .refused (.zeroThickness v)
    | none => if L.result kt = [] then .empty else .ok (L.unify kt)

variable {L}

/-- (b), for any keep table: the stages after classification never misbehave. -/
theorem coverageK (kt : KeepTab) :
    (L.coreK kt = .empty ∧ L.result kt = []) ∨
    (∃ b, L.coreK kt = .ok b ∧ Valid L kt b) ∨
    (∃ x, L.coreK kt = .refused (.zeroThickness x) ∧ ZeroThicknessAt L kt x) := by
  unfold coreK
  cases hE : L.edges.find? (L.badEdge kt) with
  | some e =>
    right; right
    refine ⟨e, rfl, Or.inl ⟨List.mem_of_find?_eq_some hE, ?_⟩⟩
    have := List.find?_some hE
    simpa [badEdge] using this
  | none =>
    cases hV : L.verts.find? (L.badVert kt) with
    | some v =>
      right; right
      refine ⟨v, rfl, Or.inr ⟨List.mem_of_find?_eq_some hV, ?_⟩⟩
      exact (badVert_iff L kt v).mp (List.find?_some hV)
    | none =>
      simp only
      by_cases hr : L.result kt = []
      · left; simp [hr]
      · right; left
        simp only [hr, ite_false]
        exact ⟨_, rfl, unify_valid hr hE hV⟩

/-! ## The collapse -/

variable (L)

/-- Two side cells on either side of a facet. -/
def tAdj (T U : Cov) : Bool :=
  L.facets.any fun φ => (φ.plus == T && φ.minus == U) || (φ.plus == U && φ.minus == T)

/-- The thin side cells, in arena order. -/
def thinTopes (thin : List Cov) : List Cov := L.topes.filter fun T => thin.contains T

/-- The feature a thin cell belongs to: its class among the thin cells. -/
def comp (thin : List Cov) (T : Cov) : List Cov := cls L.tAdj (L.thinTopes thin) T

/-- The feature's rim: the cells that are not thin, adjacent to it. -/
def rim (thin : List Cov) (T : Cov) : List Cov :=
  L.topes.filter fun U => !thin.contains U && (L.comp thin T).any fun V => L.tAdj V U

/-- The rim is kept alike. -/
def uniform (kt : KeepTab) (thin : List Cov) (T : Cov) : Bool :=
  (L.rim thin T).all fun U => keepIn kt U == keepIn kt ((L.rim thin T).headD T)

/-- The keep a thin cell takes: its rim's (or its own, if it has no rim). -/
def fill (kt : KeepTab) (thin : List Cov) (T : Cov) : Bool :=
  match L.rim thin T with
  | [] => keepIn kt T
  | U :: _ => keepIn kt U

inductive Collapse
  | ok (kt : KeepTab)
  | refuse (cell : Cov)

/-- A feature can be collapsed: its rim is kept alike and every cell of it is enclosed by pieces
(so no facet the core does not hold can become boundary). -/
def collapsible (kt : KeepTab) (thin enclosed : List Cov) (T : Cov) : Bool :=
  L.uniform kt thin T && (L.comp thin T).all fun V => enclosed.contains V

/-- **The collapse.** -/
def collapse (kt : KeepTab) (thin enclosed : List Cov) : Collapse :=
  match (L.thinTopes thin).find? fun T => !L.collapsible kt thin enclosed T with
  | some T => .refuse T
  | none => .ok (L.topes.map fun T => (T, if thin.contains T then L.fill kt thin T else keepIn kt T))

/-- **The core with the metric ledger.** -/
def coreM (rule : Rule) (thin enclosed : List Cov) : Outcome :=
  if !L.splitOk then .refused .splitCrossed
  else if !L.classifyOk then .refused .inconsistentClassification
  else match L.collapse (L.keepTab rule) thin enclosed with
    | .refuse T => .refused (.zeroThickness T)
    | .ok kt' => L.coreK kt'

variable {L}

/-! ## The rule's properties -/

theorem keepIn_map (f : Cov → Bool) {xs : List Cov} {T : Cov} (hT : T ∈ xs) :
    keepIn (xs.map fun x => (x, f x)) T = f T := by
  unfold keepIn
  rw [find?_pair f T xs hT]
  rfl

theorem collapsible_of_ok {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt')
    {T : Cov} (hT : T ∈ L.thinTopes thin) : L.collapsible kt thin enc T = true := by
  unfold collapse at h
  split at h
  · cases h
  · rename_i hf
    have := List.find?_eq_none.mp hf T hT
    simpa using this

theorem collapsed_at {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt')
    {T : Cov} (hT : T ∈ L.topes) :
    keepIn kt' T = if thin.contains T then L.fill kt thin T else keepIn kt T := by
  unfold collapse at h
  split at h
  · cases h
  · cases h
    exact keepIn_map (fun T => if thin.contains T then L.fill kt thin T else keepIn kt T) hT

/-- **Local**: a cell that is not thin keeps its keep. -/
theorem collapse_local {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt')
    {T : Cov} (hT : T ∈ L.topes) (hn : thin.contains T = false) : keepIn kt' T = keepIn kt T := by
  rw [collapsed_at h hT, hn]; rfl

theorem rim_not_thin {thin : List Cov} {T U : Cov} (h : U ∈ L.rim thin T) :
    U ∈ L.topes ∧ thin.contains U = false := by
  unfold rim at h
  simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true'] at h
  exact ⟨h.1, h.2.1⟩

theorem uniform_iff {kt : KeepTab} {thin : List Cov} {T : Cov} :
    L.uniform kt thin T = true ↔ ∀ U ∈ L.rim thin T, keepIn kt U = keepIn kt ((L.rim thin T).headD T) := by
  unfold uniform
  simp [List.all_eq_true]

/-- **The rim decides**: past the collapse, a thin cell with a rim has its rim's keep, the same
for every rim cell. -/
theorem collapse_uniform {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt')
    {T : Cov} (hT : T ∈ L.thinTopes thin) : ∀ U ∈ L.rim thin T, keepIn kt' T = keepIn kt U := by
  have hu : L.uniform kt thin T = true := by
    have := collapsible_of_ok h hT
    unfold collapsible at this
    exact (Bool.and_eq_true _ _).mp this |>.1
  have hT' : T ∈ L.topes ∧ thin.contains T = true := by
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
theorem collapse_refuse {kt : KeepTab} {thin enc : List Cov} {T : Cov} (h : L.collapse kt thin enc = .refuse T) :
    T ∈ L.thinTopes thin ∧
      ((∃ U ∈ L.rim thin T, ∃ V ∈ L.rim thin T, keepIn kt U ≠ keepIn kt V) ∨
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

theorem tAdj_symm (L : Ledger) : ∀ T U, L.tAdj T U = L.tAdj U T := by
  intro T U
  unfold tAdj
  congr 1; funext φ
  cases φ.plus == T <;> cases φ.minus == U <;> cases φ.plus == U <;> cases φ.minus == T <;> rfl

theorem thinTopes_nodup (L : Ledger) (thin : List Cov) : (L.thinTopes thin).Nodup :=
  (topes_nodup L).filter _

/-- **One feature, one answer**: cells of one component have one rim, so one keep. -/
theorem rim_comp {thin : List Cov} {T T' : Cov} (hT : T ∈ L.thinTopes thin)
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

theorem collapse_comp {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt')
    {T T' : Cov} (hT : T ∈ L.thinTopes thin) (hc : Conn L.tAdj (L.thinTopes thin) T T') :
    keepIn kt' T = keepIn kt' T' ∨ L.rim thin T = [] := by
  have hT' : T' ∈ L.thinTopes thin := Conn.mem hT hc
  cases hr : L.rim thin T with
  | nil => exact Or.inr rfl
  | cons U rest =>
    left
    have hU : U ∈ L.rim thin T := by rw [hr]; exact List.mem_cons_self ..
    rw [collapse_uniform h hT U hU, collapse_uniform h hT' U (by rw [← rim_comp hT hc]; exact hU)]

/-- **Idempotent**: the collapsed table collapses to itself. -/
theorem collapse_idem {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt') :
    L.collapse kt' thin enc = .ok kt' := by
  -- rim cells are not thin, so they keep their keep; so every rim stays uniform and every
  -- thin cell's fill is unchanged
  have hrim : ∀ T U, U ∈ L.rim thin T → keepIn kt' U = keepIn kt U := fun T U hU =>
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
  have hkt' : kt' = L.topes.map fun T => (T, if thin.contains T then L.fill kt thin T else keepIn kt T) := by
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

theorem mem_thinTopes {thin : List Cov} {T : Cov} (hT : T ∈ L.topes) (hc : thin.contains T = true) :
    T ∈ L.thinTopes thin := List.mem_filter.mpr ⟨hT, hc⟩

theorem tAdj_facet {φ : Cov} (hφ : φ ∈ L.facets) : L.tAdj φ.plus φ.minus = true := by
  unfold tAdj
  exact List.any_eq_true.mpr ⟨φ, hφ, by simp⟩

/-- The other side of a facet of a feature, if not thin, is on the feature's rim. -/
theorem in_rim {thin : List Cov} {T U : Cov} (hT : T ∈ L.thinTopes thin) (hU : U ∈ L.topes)
    (hn : thin.contains U = false) (ha : L.tAdj T U = true) : U ∈ L.rim thin T := by
  unfold rim
  refine List.mem_filter.mpr ⟨hU, ?_⟩
  simp only [hn, Bool.not_false, Bool.true_and, List.any_eq_true]
  exact ⟨T, self_mem_cls _ (thinTopes_nodup L thin) hT, ha⟩

/-- **The collapse only deletes**: every facet of the collapsed result was a facet of the result,
and no facet with a side in a feature that has a rim survives. -/
theorem collapse_subset {kt kt' : KeepTab} {thin enc : List Cov} (h : L.collapse kt thin enc = .ok kt')
    {φ : Cov} (hφ : φ ∈ L.result kt') : φ ∈ L.result kt := by
  rw [mem_result] at hφ ⊢
  obtain ⟨hf, hk⟩ := hφ
  refine ⟨hf, ?_⟩
  have hp := plus_mem_topes hf
  have hm := minus_mem_topes hf
  have ha := tAdj_facet (L := L) hf
  have ha' : L.tAdj φ.minus φ.plus = true := by rw [tAdj_symm]; exact ha
  cases cp : thin.contains φ.plus <;> cases cm : thin.contains φ.minus
  · rwa [collapse_local h hp cp, collapse_local h hm cm] at hk
  · -- minus thin, plus on its rim
    have hT := mem_thinTopes hm cm
    have := collapse_uniform h hT φ.plus (in_rim hT hp cp ha')
    rw [this, collapse_local h hp cp] at hk
    exact absurd rfl hk
  · have hT := mem_thinTopes hp cp
    have := collapse_uniform h hT φ.minus (in_rim hT hm cm ha)
    rw [this, collapse_local h hm cm] at hk
    exact absurd rfl hk
  · -- both thin: one feature
    have hT := mem_thinTopes hp cp
    have hT' := mem_thinTopes hm cm
    have hconn : Conn L.tAdj (L.thinTopes thin) φ.plus φ.minus := Conn.step Conn.refl hT' ha
    rcases collapse_comp h hT hconn with e | e
    · exact absurd e hk
    · -- no rim: both keep their own keep
      have e' : L.rim thin φ.minus = [] := by rw [← rim_comp hT hconn]; exact e
      rw [collapsed_at h hp, cp, collapsed_at h hm, cm] at hk
      simp only [ite_true, fill, e, e'] at hk
      exact hk

/-! ## Valid, and exact away from the features -/

/-- **The metric pass keeps (b).** For every consistent ledger, every op and every thin table, the
core with the collapse ends in empty, a valid body, or a ZeroThickness refusal at a stated spot: a
non-manifold cell of the collapsed result, or a thin cell whose feature's rim is kept both ways. -/
theorem coverageM (h : Consistent L) (rule : Rule) (thin enc : List Cov) :
    (∃ T, L.coreM rule thin enc = .refused (.zeroThickness T) ∧
      T ∈ L.thinTopes thin ∧
      ((∃ U ∈ L.rim thin T, ∃ V ∈ L.rim thin T, keepIn (L.keepTab rule) U ≠ keepIn (L.keepTab rule) V) ∨
       (∃ V ∈ L.comp thin T, enc.contains V = false))) ∨
    ∃ kt', L.collapse (L.keepTab rule) thin enc = .ok kt' ∧
      ((L.coreM rule thin enc = .empty ∧ L.result kt' = []) ∨
       (∃ b, L.coreM rule thin enc = .ok b ∧ Valid L kt' b) ∨
       (∃ x, L.coreM rule thin enc = .refused (.zeroThickness x) ∧ ZeroThicknessAt L kt' x)) := by
  have e : L.coreM rule thin enc = match L.collapse (L.keepTab rule) thin enc with
      | .refuse T => .refused (.zeroThickness T)
      | .ok kt' => L.coreK kt' := by
    unfold coreM; rw [splitOk_of h, classifyOk_of h]; rfl
  rw [e]
  cases hc : L.collapse (L.keepTab rule) thin enc with
  | refuse T => exact Or.inl ⟨T, rfl, collapse_refuse hc⟩
  | ok kt' => exact Or.inr ⟨kt', rfl, coverageK kt'⟩

end Ledger

variable {L : Ledger} {ps : List Ref.Plane} {A : Nat → Pred Ref.V3}

/-- **Exact away from the features.** For an exact ledger, a facet neither of whose sides is thin
is in the collapsed result iff it is an exact boundary facet. -/
theorem correct_off_thin (h : Exact L ps A) (rule : Rule) {thin enc : List Cov} {kt' : KeepTab}
    (hc : L.collapse (L.keepTab rule) thin enc = .ok kt') (φ : Cov)
    (hp : thin.contains φ.plus = false) (hm : thin.contains φ.minus = false) :
    φ ∈ L.result kt' ↔ XBoundary L ps A rule φ := by
  rw [← result_iff h rule, mem_result, mem_result]
  constructor
  · rintro ⟨hf, hk⟩
    rw [collapse_local hc (plus_mem_topes hf) hp, collapse_local hc (minus_mem_topes hf) hm] at hk
    exact ⟨hf, hk⟩
  · rintro ⟨hf, hk⟩
    rw [← collapse_local hc (plus_mem_topes hf) hp, ← collapse_local hc (minus_mem_topes hf) hm] at hk
    exact ⟨hf, hk⟩

end ForgeExact.DecideOnce
