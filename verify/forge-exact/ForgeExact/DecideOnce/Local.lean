import ForgeExact.DecideOnce.Correct
import ForgeExact.DecideOnce.Frontier

/-!
# (c′): exactness over the local arrangement, in the open region (spec §3a)

The local ledger lists only cells of the open region `R°`, cut by the near carriers and the six
walls. Its exactness is `Exact` with every global clause restricted to the region:
* compatibility on the region's full cells;
* completeness of the region's facets;
* the region's edges and verts.

`In X` says the cell `X` lies in `R°`. Every cell is wholly in `R°` or wholly out (`region`), and
the cells above a region cell are in the region (`up`); both hold because a cell's wall signs are
constant and inherited upward.

Under `ExactIn`, (c) holds in the region:
* `result_iff_in`: the result's facets are exactly the region's exact boundary facets.
* `reg_in`: at every point of `R°`, the regularised union of the kept cells is the regularised
  boolean.
* `zeroThickness_iff_in`: `ZeroThickness` iff the exact result is non-manifold at a region cell.
* `correct_ok_in`: the three together for an answered body.

With F1 (`far_unchanged`, outside `R°` the result is the target or nothing), this is the spec's
"Global". No `sorry`; standard axioms only.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref Topology Ledger

/-- **An exact local ledger**: `Exact`, with compatibility, completeness, edges and verts restricted
to the region's cells `In`, and the region a union of cells (`U`, open), closed upward. -/
structure ExactIn (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (In : Cov → Prop) (U : Pred V3) : Prop where
  nonzero : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩
  open_ : IsOpen U
  region : ∀ X x, covCell ps X x → (U x ↔ In X)
  up : ∀ X Y, In X → Cov.le X Y = true → In Y
  compat : ∀ T ∈ fulls ps.length, In T → ∀ i, (covCell ps T).Sub (A i) ∨ ∀ x, covCell ps T x → ¬A i x
  pieceFacet : ∀ p ∈ L.pieces, p.1.isFacet = true ∧ p.1.zeroIdx = L.clsOf p.2 ∧ L.bodyOf p.2 < L.nb
  oneFace : ∀ p ∈ L.pieces, ∀ q ∈ L.pieces, q.1 = p.1 → L.bodyOf q.2 = L.bodyOf p.2 → q.2 = p.2
  pieceReal : ∀ p ∈ L.pieces, Realised ps p.1
  pieceIn : ∀ p ∈ L.pieces, In p.1
  witness : ∀ T ∈ L.topes, ∀ i, i < L.nb → (L.inside.contains (T, i) = true ↔ memT ps A T i)
  boundary : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (Flip ps A φ i ↔ L.isPieceOf i φ = true)
  complete : ∀ φ, In φ → Realised ps φ → φ.isFacet = true → ∀ i, i < L.nb → Flip ps A φ i → φ ∈ L.facets
  edges : ∀ e, e ∈ L.edges ↔ In e ∧ Realised ps e ∧ IsEdge ps e ∧ ∃ φ ∈ L.facets, Cov.le e φ = true
  verts : ∀ v, v ∈ L.verts ↔ In v ∧ Realised ps v ∧ IsVert ps v ∧ ∃ φ ∈ L.facets, Cov.le v φ = true

variable {L : Ledger} {ps : List Plane} {A : Nat → Pred V3} {In : Cov → Prop} {U : Pred V3}

theorem ExactIn.consistent (h : ExactIn L ps A In U) : Consistent L where
  pieceFacet := h.pieceFacet
  oneFace := h.oneFace
  realised := ⟨mOf ps A, fun T hT i hi => by
      apply Bool.eq_iff_iff.mpr
      rw [mOf_iff]; exact h.witness T hT i hi,
    fun φ hφ i hi => by
      rw [← h.boundary φ hφ i hi]
      unfold Flip
      rw [← mOf_iff (ps := ps) (A := A), ← mOf_iff (ps := ps) (A := A)]
      cases mOf ps A φ.plus i <;> cases mOf ps A φ.minus i <;> simp⟩

theorem keep_eq_in (h : ExactIn L ps A In U) (rule : Rule) {T : Cov} (hT : T ∈ L.topes) :
    keepIn (L.keepTab rule) T = xkeep L ps A rule T := by
  rw [keepIn_tab L rule hT]
  unfold keep xkeep
  congr 1
  funext i
  by_cases hi : i < L.nb
  · have hm := memb_eq (m := mOf ps A) (fun T hT i hi => by
        apply Bool.eq_iff_iff.mpr; rw [mOf_iff]; exact h.witness T hT i hi)
      (fun φ hφ i hi => by
        rw [← h.boundary φ hφ i hi]; unfold Flip
        rw [← mOf_iff (ps := ps) (A := A), ← mOf_iff (ps := ps) (A := A)]
        cases mOf ps A φ.plus i <;> cases mOf ps A φ.minus i <;> simp) hi hT
    rw [hm]
    apply Bool.eq_iff_iff.mpr
    simp [hi, mOf_iff]
  · apply Bool.eq_iff_iff.mpr
    simp [hi]

theorem facets_in (h : ExactIn L ps A In U) {φ : Cov} (hφ : φ ∈ L.facets) : In φ := by
  obtain ⟨f, hp⟩ := mem_facets.mp hφ; exact h.pieceIn _ hp

/-- **The result is the region's exact boundary.** -/
theorem result_iff_in (h : ExactIn L ps A In U) (rule : Rule) (φ : Cov) :
    φ ∈ L.result (L.keepTab rule) ↔ In φ ∧ XBoundary L ps A rule φ := by
  rw [mem_result]
  constructor
  · rintro ⟨hf, hk⟩
    obtain ⟨f, hp⟩ := mem_facets.mp hf
    refine ⟨h.pieceIn _ hp, h.pieceReal _ hp, (h.pieceFacet _ hp).1, ?_⟩
    rwa [← keep_eq_in h rule (plus_mem_topes hf), ← keep_eq_in h rule (minus_mem_topes hf)]
  · rintro ⟨hin, hr, hfa, hk⟩
    have : ∃ i, i < L.nb ∧ Flip ps A φ i := by
      apply Classical.byContradiction
      intro hno
      apply hk
      unfold xkeep
      congr 1; funext i
      apply Bool.eq_iff_iff.mpr
      simp only [decide_eq_true_iff]
      by_cases hi : i < L.nb
      · have : ¬Flip ps A φ i := fun hfl => hno ⟨i, hi, hfl⟩
        unfold Flip at this
        have := Classical.not_not.mp this
        exact ⟨fun ⟨a, b⟩ => ⟨a, this.mp b⟩, fun ⟨a, b⟩ => ⟨a, this.mpr b⟩⟩
      · exact ⟨fun ⟨a, _⟩ => absurd a hi, fun ⟨a, _⟩ => absurd a hi⟩
    obtain ⟨i, hi, hfl⟩ := this
    have hf := h.complete φ hin hr hfa i hi hfl
    refine ⟨hf, ?_⟩
    rwa [keep_eq_in h rule (plus_mem_topes hf), keep_eq_in h rule (minus_mem_topes hf)]

/-! ## The regularised boolean in the region -/

/-- The operands seen from the region: nothing outside `U`. -/
def opndsIn (L : Ledger) (A : Nat → Pred V3) (U : Pred V3) : Nat → Pred V3 := fun i x => U x ∧ opnds L A i x

theorem opndsIn_compat (h : ExactIn L ps A In U) (T : Cov) (hT : T ∈ fulls ps.length) (i : Nat) :
    (covCell ps T).Sub (opndsIn L A U i) ∨ ∀ x, covCell ps T x → ¬opndsIn L A U i x := by
  by_cases hin : In T
  · by_cases hi : i < L.nb
    · rcases h.compat T hT hin i with hs | hd
      · exact Or.inl fun x hx => ⟨(h.region T x hx).mpr hin, hi, hs x hx⟩
      · exact Or.inr fun x hx ⟨_, _, ha⟩ => hd x hx ha
    · exact Or.inr fun x _ ⟨_, h', _⟩ => hi h'
  · exact Or.inr fun x hx ⟨hU, _⟩ => hin ((h.region T x hx).mp hU)

/-- The arrangement's full cells, compatible with the operands seen from the region. -/
def decompIn (h : ExactIn L ps A In U) : Decomposition.{0, 0, 0} (opndsIn L A U) :=
  topeDecomp ps h.nonzero (opndsIn L A U) (opndsIn_compat h)

/-- The exact keep, on a region cell, seen from the region; nothing outside it. -/
noncomputable def xkeepIn (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (In : Cov → Prop) (rule : Rule)
    (T : Cov) : Prop :=
  In T ∧ xkeep L ps A rule T = true ∨ ¬In T ∧ rule (fun _ => false) = true

theorem xkeepIn_iff (h : ExactIn L ps A In U) (rule : Rule) (T : Cov) (hT : T ∈ fulls ps.length) (x : V3)
    (hx : covCell ps T x) :
    xkeepIn L ps A In rule T ↔ ruleOp rule fun i => opndsIn L A U i x := by
  by_cases hin : In T
  · have hU : U x := (h.region T x hx).mpr hin
    have e : (fun i => opndsIn L A U i x) = fun i => opnds L A i x := by
      funext i; apply propext; simp [opndsIn, hU]
    rw [e, ← xkeep_iff_in h rule T hT hin x hx]
    simp [xkeepIn, hin]
  · have hU : ¬U x := fun hu => hin ((h.region T x hx).mp hu)
    have e : (fun i => opndsIn L A U i x) = fun _ => False := by
      funext i; apply propext; simp [opndsIn, hU]
    rw [e]
    unfold ruleOp
    simp [xkeepIn, hin]
where
  xkeep_iff_in (h : ExactIn L ps A In U) (rule : Rule) (T : Cov) (hT : T ∈ fulls ps.length) (hin : In T)
      (x : V3) (hx : covCell ps T x) :
      xkeep L ps A rule T = true ↔ ruleOp rule fun i => opnds L A i x := by
    unfold xkeep ruleOp opnds
    have : (fun i => @decide (i < L.nb ∧ memT ps A T i) (Classical.propDecidable _)) =
        (fun i => @decide (i < L.nb ∧ A i x) (Classical.propDecidable _)) := by
      funext i
      apply Bool.eq_iff_iff.mpr
      simp only [decide_eq_true_iff]
      rcases h.compat T hT hin i with hs | hd
      · exact ⟨fun ⟨a, _⟩ => ⟨a, hs x hx⟩, fun ⟨a, _⟩ => ⟨a, x, hx, hs x hx⟩⟩
      · exact ⟨fun ⟨_, y, hy, hay⟩ => absurd hay (hd y hy), fun ⟨_, hax⟩ => absurd hax (hd x hx)⟩
    rw [this]

/-- **The regularised boolean, in the region.** At every point of `R°`, the regularised union of
the kept cells is the regularised boolean of the snapped solids. -/
theorem reg_in (h : ExactIn L ps A In U) (rule : Rule) :
    ∀ x, U x → (reg ((decompIn h).kept fun T => xkeepIn L ps A In rule T) x ↔
      reg (combine (ruleOp rule) (opnds L A)) x) := by
  have hk := reg_kept (decompIn h) (ruleOp rule) (fun T => xkeepIn L ps A In rule T)
    fun T x hx => xkeepIn_iff h rule T hx.1 x hx.2
  intro x hx
  rw [hk]
  apply Frontier.reg_local h.open_ _ x hx
  intro y hy
  unfold combine
  have e : (fun i => opndsIn L A U i y) = fun i => opnds L A i y := by
    funext i; apply propext; simp [opndsIn, hy]
  rw [e]

/-! ## Non-manifold, in the region -/

/-- **The exact result is non-manifold at a region cell.** -/
def XNonManifoldIn (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (In : Cov → Prop) (rule : Rule) : Prop :=
  (∃ E, In E ∧ Realised ps E ∧ IsEdge ps E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
    (XBoundary L ps A rule a ∧ Cov.le E a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le E b = true) ∧
    (XBoundary L ps A rule c ∧ Cov.le E c = true)) ∨
  (∃ V, In V ∧ Realised ps V ∧ IsVert ps V ∧ ∃ a b,
    (XBoundary L ps A rule a ∧ Cov.le V a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le V b = true) ∧
    ¬XFan L ps A rule V a b)

theorem mem_around_in (h : ExactIn L ps A In U) (rule : Rule) {e φ : Cov} (he : In e) :
    φ ∈ L.around (L.keepTab rule) e ↔ XBoundary L ps A rule φ ∧ Cov.le e φ = true := by
  rw [mem_around, result_iff_in h rule]
  constructor
  · rintro ⟨⟨_, hx⟩, hl⟩; exact ⟨hx, hl⟩
  · rintro ⟨hx, hl⟩; exact ⟨⟨h.up e φ he hl, hx⟩, hl⟩

theorem badEdge_iff_in (h : ExactIn L ps A In U) (rule : Rule) :
    (∃ e ∈ L.edges, L.badEdge (L.keepTab rule) e = true) ↔
    ∃ E, In E ∧ Realised ps E ∧ IsEdge ps E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
      (XBoundary L ps A rule a ∧ Cov.le E a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le E b = true) ∧
      (XBoundary L ps A rule c ∧ Cov.le E c = true) := by
  constructor
  · rintro ⟨e, he, hb⟩
    simp only [badEdge, decide_eq_true_eq] at hb
    obtain ⟨a, ha, b, hb', c, hc, hab, hac, hbc⟩ := (three_iff (around_nodup L (L.keepTab rule) e)).mp hb
    obtain ⟨hin, hr, hed, _⟩ := (h.edges e).mp he
    exact ⟨e, hin, hr, hed, a, b, c, hab, hac, hbc, (mem_around_in h rule hin).mp ha,
      (mem_around_in h rule hin).mp hb', (mem_around_in h rule hin).mp hc⟩
  · rintro ⟨E, hin, hr, hed, a, b, c, hab, hac, hbc, ha, hb, hc⟩
    have haf := (mem_result.mp ((result_iff_in h rule a).mpr ⟨h.up E a hin ha.2, ha.1⟩)).1
    refine ⟨E, (h.edges E).mpr ⟨hin, hr, hed, a, haf, ha.2⟩, ?_⟩
    simp only [badEdge, decide_eq_true_eq]
    exact (three_iff (around_nodup L (L.keepTab rule) E)).mpr
      ⟨a, (mem_around_in h rule hin).mpr ha, b, (mem_around_in h rule hin).mpr hb, c,
        (mem_around_in h rule hin).mpr hc, hab, hac, hbc⟩

theorem fan_iff_in (h : ExactIn L ps A In U) (rule : Rule) {v : Cov} (hv : In v) {c d : Cov}
    (_hc : c ∈ L.around (L.keepTab rule) v) :
    Conn (L.fanRel v) (L.around (L.keepTab rule) v) c d ↔ XFan L ps A rule v c d := by
  constructor
  · intro hcd
    induction hcd with
    | refl => exact XFan.refl
    | step _ hy hr ih =>
      rw [mem_around_in h rule hv] at hy
      simp only [fanRel, List.any_eq_true, Bool.and_eq_true] at hr
      obtain ⟨e, he, ⟨hve, heb⟩, hec⟩ := hr
      obtain ⟨_, her, hed, _⟩ := (h.edges e).mp he
      exact XFan.step ih hy.1 hy.2 ⟨e, her, hed, hve, heb, hec⟩
  · intro hcd
    induction hcd with
    | refl => exact Conn.refl
    | step _ hb hvc hE ih =>
      obtain ⟨E, her, hed, hvE, hEb, hEc⟩ := hE
      have hm : _ ∈ L.around (L.keepTab rule) v := (mem_around_in h rule hv).mpr ⟨hb, hvc⟩
      have hEin : In E := h.up v E hv hvE
      have hf := (mem_result.mp ((result_iff_in h rule _).mpr ⟨h.up E _ hEin hEc, hb⟩)).1
      refine Conn.step ih hm ?_
      simp only [fanRel, List.any_eq_true, Bool.and_eq_true]
      exact ⟨E, (h.edges E).mpr ⟨hEin, her, hed, _, hf, hEc⟩, ⟨hvE, hEb⟩, hEc⟩

theorem badVert_iff_in (h : ExactIn L ps A In U) (rule : Rule) :
    (∃ v ∈ L.verts, L.badVert (L.keepTab rule) v = true) ↔
    ∃ V, In V ∧ Realised ps V ∧ IsVert ps V ∧ ∃ a b,
      (XBoundary L ps A rule a ∧ Cov.le V a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le V b = true) ∧
      ¬XFan L ps A rule V a b := by
  constructor
  · rintro ⟨v, hv, hb⟩
    obtain ⟨c, hc, d, hd, hn⟩ := (badVert_iff L (L.keepTab rule) v).mp hb
    obtain ⟨hin, hr, hvx, _⟩ := (h.verts v).mp hv
    exact ⟨v, hin, hr, hvx, c, d, (mem_around_in h rule hin).mp hc, (mem_around_in h rule hin).mp hd,
      fun hx => hn ((fan_iff_in h rule hin hc).mpr hx)⟩
  · rintro ⟨V, hin, hr, hvx, a, b, ha, hb, hn⟩
    have haf := (mem_result.mp ((result_iff_in h rule a).mpr ⟨h.up V a hin ha.2, ha.1⟩)).1
    have hma := (mem_around_in h rule hin).mpr ha
    refine ⟨V, (h.verts V).mpr ⟨hin, hr, hvx, a, haf, ha.2⟩, (badVert_iff L (L.keepTab rule) V).mpr
      ⟨a, hma, b, (mem_around_in h rule hin).mpr hb, fun hc => hn ((fan_iff_in h rule hin hma).mp hc)⟩⟩

/-- **(c′), the refusal.** The core refuses `ZeroThickness` iff the exact result is non-manifold at
a cell of the region. -/
theorem zeroThickness_iff_in (h : ExactIn L ps A In U) (rule : Rule) :
    (∃ x, core L rule = .refused (.zeroThickness x)) ↔ XNonManifoldIn L ps A In rule := by
  rw [refused_iff h.consistent rule, badEdge_iff_in h rule, badVert_iff_in h rule]
  rfl

/-- **(c′), a valid result.** The body is valid; its faces are exactly the region's exact boundary
facets; and at every point of `R°` the kept cells regularise to the regularised boolean. -/
theorem correct_ok_in (h : ExactIn L ps A In U) (rule : Rule) {b : Body} (hb : core L rule = .ok b) :
    Valid L (L.keepTab rule) b ∧
    (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ In φ ∧ XBoundary L ps A rule φ) ∧
    ∀ x, U x → (reg ((decompIn h).kept fun T => xkeepIn L ps A In rule T) x ↔
      reg (combine (ruleOp rule) (opnds L A)) x) := by
  rcases coverage h.consistent rule with ⟨e, _⟩ | ⟨b', e, hv⟩ | ⟨x, e, _⟩
  · rw [e] at hb; cases hb
  · rw [e] at hb; cases hb
    exact ⟨hv, fun φ => (hv.cover φ).symm.trans (result_iff_in h rule φ), reg_in h rule⟩
  · rw [e] at hb; cases hb

end ForgeExact.DecideOnce
