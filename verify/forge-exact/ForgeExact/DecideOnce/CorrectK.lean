import ForgeExact.DecideOnce.Curved

/-!
# Theorem (c) for any carriers (design Q0: research/decide-once/curved-design.md §4)

Correct.lean's theorems over `ExactK` (Curved.lean): the carriers are any functions `P → K` on
any space of points (planes, quadrics, tori, separators), the wall is any nowhere-dense union of
their zero sets. The proofs are Correct.lean's: they read the ledger's fields, never the carriers'
form, so nothing changes but the names of the cells.

* `ExactK.consistent`: an exact curved ledger is consistent, so (b) (`coverage`, `no_retry`)
  applies to it;
* `result_iff_k`: the core's result facets are exactly the exact boundary facets;
* `correct_k`, `correct_ok_k`: and the regularised union of the kept full cells is the
  regularised boolean (`reg_kept`, over `decompK`);
* `zeroThickness_iff_k`: the core refuses `ZeroThickness` iff `XNonManifoldK`, defined from the
  carriers' cells alone. Under `Faithful` that is the result's own non-manifoldness (design §1.3;
  the topological statement is Q1's `xNonManifold_iff_top`).

The flat theorems are the instance `P = V3`, `K = ℚ`, `cs = ps.map Plane.eval` (`covCellK_flat`).
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.DecideOnce Topology Ledger

set_option linter.unusedSectionVars false

variable {P K : Type} [Zero K] [LT K] [DecidableRel (α := K) (· < ·)] [Topology P]
variable {L : Ledger} {cs : List (Carrier P K)} {A : Nat → Pred P}

/-! ## The full cells as a decomposition -/

theorem mem_fullsK : ∀ {n : Nat} {T : Cov}, T.length = n → Sgn.zero ∉ T → T ∈ fulls n
  | 0, [], _, _ => by simp [fulls]
  | n + 1, t :: T, hl, hz => by
    simp only [fulls, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]
    refine ⟨T, mem_fullsK (by simpa using hl) (fun h => hz (List.mem_cons_of_mem _ h)), ?_⟩
    cases t
    · exact Or.inr rfl
    · exact absurd (List.mem_cons_self ..) hz
    · exact Or.inl rfl

theorem zero_not_mem_fulls : ∀ {n : Nat} {T : Cov}, T ∈ fulls n → Sgn.zero ∉ T
  | 0, T, h => by simp [fulls] at h; subst h; simp
  | n + 1, T, h => by
    simp only [fulls, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false] at h
    obtain ⟨l, hl, rfl | rfl⟩ := h <;>
      · intro hz
        rcases List.mem_cons.mp hz with h0 | h0
        · cases h0
        · exact zero_not_mem_fulls hl h0

/-- Off the wall, a point's covector is full. -/
theorem covOfK_full (cs : List (Carrier P K)) (x : P) (h : ¬wallK cs x) :
    covOfK cs x ∈ fulls cs.length := by
  apply mem_fullsK (by simp [covOfK])
  intro hz
  simp only [covOfK, List.mem_map] at hz
  obtain ⟨c, hc, h0⟩ := hz
  exact h ⟨c, hc, h0⟩

/-- **The full cells are a decomposition** compatible with any solids constant on each. -/
def topeDecompK (cs : List (Carrier P K)) (hw : NowhereDense (wallK cs)) (A : Nat → Pred P)
    (hc : ∀ T ∈ fulls cs.length, ∀ i, (covCellK cs T).Sub (A i) ∨ ∀ x, covCellK cs T x → ¬A i x) :
    Decomposition.{0, 0, 0} A where
  κ := Cov
  cells := fulls cs.length
  cell T x := T ∈ fulls cs.length ∧ covCellK cs T x
  wall := wallK cs
  wall_nowhereDense := hw
  cover x h := ⟨covOfK cs x, covOfK_full cs x h, covOfK_full cs x h, rfl⟩
  compatible T i := by
    by_cases hT : T ∈ fulls cs.length
    · rcases hc T hT i with hs | hd
      · exact Or.inl fun x hx => hs x hx.2
      · exact Or.inr fun x hx => hd x hx.2
    · exact Or.inr fun x hx => absurd hx.1 hT

/-- `ExactK.compat` on every full covector (an unrealised one has no point). -/
theorem ExactK.compat' (h : ExactK L cs A) (T : Cov) (hT : T ∈ fulls cs.length) (i : Nat) :
    (covCellK cs T).Sub (A i) ∨ ∀ x, covCellK cs T x → ¬A i x := by
  by_cases hr : RealisedK cs T
  · exact h.compat T ⟨hr, zero_not_mem_fulls hT⟩ i
  · exact Or.inr fun x hx _ => hr ⟨x, hx⟩

/-! ## Consistency, keep and the result -/

noncomputable def mOfK (cs : List (Carrier P K)) (A : Nat → Pred P) (T : Cov) (i : Nat) : Bool :=
  @decide (memTK cs A T i) (Classical.propDecidable _)

theorem mOfK_iff {T : Cov} {i : Nat} : mOfK cs A T i = true ↔ memTK cs A T i := by
  simp [mOfK]

theorem flip_bool (h : ExactK L cs A) {φ : Cov} (hφ : φ ∈ L.facets) {i : Nat} (hi : i < L.nb) :
    (mOfK cs A φ.plus i ≠ mOfK cs A φ.minus i ↔ L.isPieceOf i φ = true) := by
  rw [← h.boundary φ hφ i hi]
  unfold FlipK
  rw [← mOfK_iff (cs := cs) (A := A), ← mOfK_iff (cs := cs) (A := A)]
  cases mOfK cs A φ.plus i <;> cases mOfK cs A φ.minus i <;> simp

theorem witness_bool (h : ExactK L cs A) {T : Cov} (hT : T ∈ L.topes) {i : Nat} (hi : i < L.nb) :
    L.inside.contains (T, i) = mOfK cs A T i := by
  apply Bool.eq_iff_iff.mpr
  rw [mOfK_iff]; exact h.witness T hT i hi

/-- **An exact curved ledger is consistent**: the solids' membership realises it. -/
theorem ExactK.consistent (h : ExactK L cs A) : Consistent L where
  pieceFacet := h.pieceFacet
  oneFace := h.oneFace
  realised := ⟨mOfK cs A, fun _ hT _ hi => witness_bool h hT hi, fun _ hφ _ hi => flip_bool h hφ hi⟩

/-- The exact boolean's keep, on a cell. -/
noncomputable def xkeepK (L : Ledger) (cs : List (Carrier P K)) (A : Nat → Pred P) (rule : Rule)
    (T : Cov) : Bool :=
  rule fun i => @decide (i < L.nb ∧ memTK cs A T i) (Classical.propDecidable _)

/-- An exact boundary facet. -/
def XBoundaryK (L : Ledger) (cs : List (Carrier P K)) (A : Nat → Pred P) (rule : Rule) (φ : Cov) :
    Prop :=
  RealisedK cs φ ∧ φ.isFacet = true ∧ xkeepK L cs A rule φ.plus ≠ xkeepK L cs A rule φ.minus

theorem keep_eqK' (h : ExactK L cs A) (rule : Rule) {T : Cov} (hT : T ∈ L.topes) :
    L.keep rule T = xkeepK L cs A rule T := by
  unfold keep xkeepK
  congr 1
  funext i
  by_cases hi : i < L.nb
  · have hm := memb_eq (m := mOfK cs A) (fun _ hT _ hi => witness_bool h hT hi)
      (fun _ hφ _ hi => flip_bool h hφ hi) hi hT
    rw [hm]
    apply Bool.eq_iff_iff.mpr
    simp [hi, mOfK_iff]
  · apply Bool.eq_iff_iff.mpr
    simp [hi]

theorem keep_eqK (h : ExactK L cs A) (rule : Rule) {T : Cov} (hT : T ∈ L.topes) :
    keepIn (L.keepTab rule) T = xkeepK L cs A rule T := by
  rw [keepIn_tab L rule hT]; exact keep_eqK' h rule hT

/-- **The result is the exact boundary.** -/
theorem result_iff_k (h : ExactK L cs A) (rule : Rule) (φ : Cov) :
    φ ∈ L.result (L.keepTab rule) ↔ XBoundaryK L cs A rule φ := by
  rw [mem_result]
  constructor
  · rintro ⟨hf, hk⟩
    obtain ⟨f, hp⟩ := mem_facets.mp hf
    refine ⟨h.pieceReal _ hp, (h.pieceFacet _ hp).1, ?_⟩
    rwa [← keep_eqK h rule (plus_mem_topes hf), ← keep_eqK h rule (minus_mem_topes hf)]
  · rintro ⟨hr, hfa, hk⟩
    have : ∃ i, i < L.nb ∧ FlipK cs A φ i := by
      apply Classical.byContradiction
      intro hno
      apply hk
      unfold xkeepK
      congr 1; funext i
      apply Bool.eq_iff_iff.mpr
      simp only [decide_eq_true_iff]
      by_cases hi : i < L.nb
      · have : ¬FlipK cs A φ i := fun hfl => hno ⟨i, hi, hfl⟩
        unfold FlipK at this
        have := Classical.not_not.mp this
        exact ⟨fun ⟨a, b⟩ => ⟨a, this.mp b⟩, fun ⟨a, b⟩ => ⟨a, this.mpr b⟩⟩
      · exact ⟨fun ⟨a, _⟩ => absurd a hi, fun ⟨a, _⟩ => absurd a hi⟩
    obtain ⟨i, hi, hfl⟩ := this
    have hf := h.complete φ hr hfa i hi hfl
    refine ⟨hf, ?_⟩
    rwa [keep_eqK h rule (plus_mem_topes hf), keep_eqK h rule (minus_mem_topes hf)]

/-! ## The regularised boolean -/

/-- The solids as the boolean's operands: bodies past `nb` are empty. -/
def opndsK (L : Ledger) (A : Nat → Pred P) : Nat → Pred P := fun i x => i < L.nb ∧ A i x

theorem opndsK_compat (h : ExactK L cs A) (T : Cov) (hT : T ∈ fulls cs.length) (i : Nat) :
    (covCellK cs T).Sub (opndsK L A i) ∨ ∀ x, covCellK cs T x → ¬opndsK L A i x := by
  by_cases hi : i < L.nb
  · rcases h.compat' T hT i with hs | hd
    · exact Or.inl fun x hx => ⟨hi, hs x hx⟩
    · exact Or.inr fun x hx ⟨_, ha⟩ => hd x hx ha
  · exact Or.inr fun x _ ⟨h', _⟩ => hi h'

/-- The arrangement's full cells, as a decomposition compatible with the operands. -/
def decompK (h : ExactK L cs A) : Decomposition.{0, 0, 0} (opndsK L A) :=
  topeDecompK cs h.wallND (opndsK L A) (opndsK_compat h)

theorem xkeepK_iff (h : ExactK L cs A) (rule : Rule) (T : Cov) (hT : T ∈ fulls cs.length) (x : P)
    (hx : covCellK cs T x) :
    xkeepK L cs A rule T = true ↔ ruleOp rule fun i => opndsK L A i x := by
  unfold xkeepK ruleOp opndsK
  have : (fun i => @decide (i < L.nb ∧ memTK cs A T i) (Classical.propDecidable _)) =
      (fun i => @decide (i < L.nb ∧ A i x) (Classical.propDecidable _)) := by
    funext i
    apply Bool.eq_iff_iff.mpr
    simp only [decide_eq_true_iff]
    rcases h.compat' T hT i with hs | hd
    · exact ⟨fun ⟨a, _⟩ => ⟨a, hs x hx⟩, fun ⟨a, _⟩ => ⟨a, x, hx, hs x hx⟩⟩
    · exact ⟨fun ⟨_, y, hy, hay⟩ => absurd hay (hd y hy), fun ⟨_, hax⟩ => absurd hax (hd x hx)⟩
  rw [this]

/-- **Theorem (c), correctness, for any carriers.** -/
theorem correct_k (h : ExactK L cs A) (rule : Rule) :
    (∀ φ, φ ∈ L.result (L.keepTab rule) ↔ XBoundaryK L cs A rule φ) ∧
    reg ((decompK h).kept fun T => xkeepK L cs A rule T = true) =
      reg (combine (ruleOp rule) (opndsK L A)) :=
  ⟨result_iff_k h rule,
    reg_kept (decompK h) (ruleOp rule) _ fun T x hx => xkeepK_iff h rule T hx.1 x hx.2⟩

/-- **A valid result is the exact boolean, for any carriers.** -/
theorem correct_ok_k (h : ExactK L cs A) (rule : Rule) {b : Body} (hb : core L rule = .ok b) :
    Valid L (L.keepTab rule) b ∧
    (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ XBoundaryK L cs A rule φ) ∧
    reg ((decompK h).kept fun T => xkeepK L cs A rule T = true) =
      reg (combine (ruleOp rule) (opndsK L A)) := by
  rcases coverage h.consistent rule with ⟨e, _⟩ | ⟨b', e, hv⟩ | ⟨x, e, _⟩
  · rw [e] at hb; cases hb
  · rw [e] at hb; cases hb
    exact ⟨hv, fun φ => (hv.cover φ).symm.trans (result_iff_k h rule φ), (correct_k h rule).2⟩
  · rw [e] at hb; cases hb

/-! ## Non-manifold, from the carriers' cells -/

inductive XFanK (L : Ledger) (cs : List (Carrier P K)) (A : Nat → Pred P) (rule : Rule) (V : Cov)
    (a : Cov) : Cov → Prop
  | refl : XFanK L cs A rule V a a
  | step {b c : Cov} : XFanK L cs A rule V a b → XBoundaryK L cs A rule c → Cov.le V c = true →
      (∃ E, RealisedK cs E ∧ IsEdgeK cs E ∧ Cov.le V E = true ∧ Cov.le E b = true ∧
        Cov.le E c = true) →
      XFanK L cs A rule V a c

/-- **The exact result is non-manifold**, over the carriers' cells: an edge cell with three exact
boundary facets round it, or a vertex cell with two in different fans. -/
def XNonManifoldK (L : Ledger) (cs : List (Carrier P K)) (A : Nat → Pred P) (rule : Rule) : Prop :=
  (∃ E, RealisedK cs E ∧ IsEdgeK cs E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
    (XBoundaryK L cs A rule a ∧ Cov.le E a = true) ∧ (XBoundaryK L cs A rule b ∧ Cov.le E b = true) ∧
    (XBoundaryK L cs A rule c ∧ Cov.le E c = true)) ∨
  (∃ V, RealisedK cs V ∧ IsVertK cs V ∧ ∃ a b,
    (XBoundaryK L cs A rule a ∧ Cov.le V a = true) ∧ (XBoundaryK L cs A rule b ∧ Cov.le V b = true) ∧
    ¬XFanK L cs A rule V a b)

theorem mem_around_k (h : ExactK L cs A) (rule : Rule) (e φ : Cov) :
    φ ∈ L.around (L.keepTab rule) e ↔ XBoundaryK L cs A rule φ ∧ Cov.le e φ = true := by
  rw [mem_around, result_iff_k h rule]

theorem badEdge_iff_k (h : ExactK L cs A) (rule : Rule) :
    (∃ e ∈ L.edges, L.badEdge (L.keepTab rule) e = true) ↔
    ∃ E, RealisedK cs E ∧ IsEdgeK cs E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
      (XBoundaryK L cs A rule a ∧ Cov.le E a = true) ∧ (XBoundaryK L cs A rule b ∧ Cov.le E b = true) ∧
      (XBoundaryK L cs A rule c ∧ Cov.le E c = true) := by
  have mem := mem_around_k h rule
  constructor
  · rintro ⟨e, he, hb⟩
    simp only [badEdge, decide_eq_true_eq] at hb
    obtain ⟨a, ha, b, hb', c, hc, hab, hac, hbc⟩ := (three_iff (around_nodup L (L.keepTab rule) e)).mp hb
    obtain ⟨hr, hed, _⟩ := (h.edges e).mp he
    exact ⟨e, hr, hed, a, b, c, hab, hac, hbc, (mem e a).mp ha, (mem e b).mp hb', (mem e c).mp hc⟩
  · rintro ⟨E, hr, hed, a, b, c, hab, hac, hbc, ha, hb, hc⟩
    have haf := (mem_result.mp ((result_iff_k h rule a).mpr ha.1)).1
    refine ⟨E, (h.edges E).mpr ⟨hr, hed, a, haf, ha.2⟩, ?_⟩
    simp only [badEdge, decide_eq_true_eq]
    exact (three_iff (around_nodup L (L.keepTab rule) E)).mpr
      ⟨a, (mem E a).mpr ha, b, (mem E b).mpr hb, c, (mem E c).mpr hc, hab, hac, hbc⟩

theorem fan_iff_k (h : ExactK L cs A) (rule : Rule) (v : Cov) {c d : Cov} :
    Conn (L.fanRel v) (L.around (L.keepTab rule) v) c d ↔ XFanK L cs A rule v c d := by
  constructor
  · intro hcd
    induction hcd with
    | refl => exact XFanK.refl
    | step _ hy hr ih =>
      rw [mem_around, result_iff_k h rule] at hy
      simp only [fanRel, List.any_eq_true, Bool.and_eq_true] at hr
      obtain ⟨e, he, ⟨hve, heb⟩, hec⟩ := hr
      obtain ⟨her, hed, _⟩ := (h.edges e).mp he
      exact XFanK.step ih hy.1 hy.2 ⟨e, her, hed, hve, heb, hec⟩
  · intro hcd
    induction hcd with
    | refl => exact Conn.refl
    | step _ hb hvc hE ih =>
      obtain ⟨E, her, hed, hvE, hEb, hEc⟩ := hE
      have hm : _ ∈ L.around (L.keepTab rule) v :=
        (mem_around).mpr ⟨(result_iff_k h rule _).mpr hb, hvc⟩
      have hf := (mem_result.mp ((result_iff_k h rule _).mpr hb)).1
      refine Conn.step ih hm ?_
      simp only [fanRel, List.any_eq_true, Bool.and_eq_true]
      exact ⟨E, (h.edges E).mpr ⟨her, hed, _, hf, hEc⟩, ⟨hvE, hEb⟩, hEc⟩

theorem badVert_iff_k (h : ExactK L cs A) (rule : Rule) :
    (∃ v ∈ L.verts, L.badVert (L.keepTab rule) v = true) ↔
    ∃ V, RealisedK cs V ∧ IsVertK cs V ∧ ∃ a b,
      (XBoundaryK L cs A rule a ∧ Cov.le V a = true) ∧ (XBoundaryK L cs A rule b ∧ Cov.le V b = true) ∧
      ¬XFanK L cs A rule V a b := by
  have mem := mem_around_k h rule
  constructor
  · rintro ⟨v, hv, hb⟩
    obtain ⟨c, hc, d, hd, hn⟩ := (badVert_iff L (L.keepTab rule) v).mp hb
    obtain ⟨hr, hvx, _⟩ := (h.verts v).mp hv
    exact ⟨v, hr, hvx, c, d, (mem v c).mp hc, (mem v d).mp hd,
      fun hx => hn ((fan_iff_k h rule v).mpr hx)⟩
  · rintro ⟨V, hr, hvx, a, b, ha, hb, hn⟩
    have haf := (mem_result.mp ((result_iff_k h rule a).mpr ha.1)).1
    have hma := (mem V a).mpr ha
    refine ⟨V, (h.verts V).mpr ⟨hr, hvx, a, haf, ha.2⟩, (badVert_iff L (L.keepTab rule) V).mpr
      ⟨a, hma, b, (mem V b).mpr hb, fun hc => hn ((fan_iff_k h rule V).mp hc)⟩⟩

/-- **Theorem (c), the refusal, for any carriers.** -/
theorem zeroThickness_iff_k (h : ExactK L cs A) (rule : Rule) :
    (∃ x, core L rule = .refused (.zeroThickness x)) ↔ XNonManifoldK L cs A rule := by
  rw [refused_iff h.consistent rule, badEdge_iff_k h rule, badVert_iff_k h rule]
  rfl

end ForgeExact.DecideOnce.Curved
