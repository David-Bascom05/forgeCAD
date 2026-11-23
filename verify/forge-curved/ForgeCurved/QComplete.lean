import ForgeCurved.QSides
import ForgeCurved.Sides

/-!
# `ExactK.complete` for the bounded curved ledger

**`qledgerB_complete`**: every realised facet that a body flips across is a facet of the ledger.

There are three cases, by the facet's sign on `B`:
* `pos`: both sides are outside `B`, so outside every body;
* `zero` (a facet of `B` itself): the outside side is in no body. If the inside side were in a
  body, so would be the facet's own points, since a body's formula does not read `B`
  (`onCov_set_ge`). Those points are on `B`, but the certificate puts every body strictly inside
  (`qledgerB_inside`);
* `neg`: the facet is sampled (`facet_sampled_input`). Both its sides are realised
  (`facet_sides`), so a flip is a difference of the formula, which makes it a piece.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem two_zeros : ∀ (L : List Sgn) (i k : ℕ), i ≠ k → L[i]? = some Sgn.zero →
    L[k]? = some Sgn.zero → 2 ≤ L.count Sgn.zero
  | [], _, _, _, h, _ => by simp at h
  | a :: L, 0, 0, hik, _, _ => absurd rfl hik
  | a :: L, 0, k + 1, _, hi, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
    subst hi
    simp only [List.getElem?_cons_succ] at hk
    have : 0 < L.count Sgn.zero := List.count_pos_iff.mpr (List.mem_of_getElem? hk)
    simp only [List.count_cons_self]; omega
  | a :: L, i + 1, 0, _, hi, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
    subst hk
    simp only [List.getElem?_cons_succ] at hi
    have : 0 < L.count Sgn.zero := List.count_pos_iff.mpr (List.mem_of_getElem? hi)
    simp only [List.count_cons_self]; omega
  | a :: L, i + 1, k + 1, hik, hi, hk => by
    simp only [List.getElem?_cons_succ] at hi hk
    have h1 := two_zeros L i k (by omega) hi hk
    have h2 : L.count Sgn.zero ≤ (a :: L).count Sgn.zero := List.count_le_count_cons
    omega

/-- A facet has one zero, at `zeroIdx`. -/
theorem facet_unique {φ : Cov} (hφ : φ.isFacet = true) :
    ∀ m, m ≠ φ.zeroIdx → φ[m]? ≠ some Sgn.zero := by
  intro m hm h
  have := two_zeros φ m φ.zeroIdx hm h (facet_zero hφ)
  rw [(isFacet_iff φ).mp hφ] at this; omega

theorem qledger_samples {qs : List Quad.Quadric} {forms : List SForm} {L : Ledger}
    (h : qledger qs forms = some L) :
    ∃ ss, samplesOf qs = some ss ∧ L = curvedLedger qs.length forms ss := by
  unfold qledger at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at h
  obtain ⟨⟨f, ss⟩, hss, h⟩ := h
  dsimp only at h
  split at h
  · cases h; exact ⟨ss, by simp [samplesOf, hss], rfl⟩
  · cases h

theorem sgn_at {cs : List (Carrier R3 ℝ)} {X : Cov} {x : R3} (hx : covCellK cs X x) {k : ℕ}
    (hk : k < cs.length) {s : Sgn} (hs : X[k]? = some s) : sgnK (cs[k] x) = s := by
  rw [← hx, getElem?_covOfK] at hs
  simp only [List.getElem?_eq_getElem hk, Option.map_some, Option.some.injEq] at hs
  exact hs

theorem sq_le_abs {t : ℝ} {R : ℝ} (ht : t * t ≤ R) : |t| ≤ max 1 R := by
  rcases le_or_gt |t| 1 with h | h
  · exact _root_.le_trans h (le_max_left _ _)
  · have : |t| ≤ |t| * |t| := by nlinarith
    rw [← abs_mul_abs_self t] at ht
    exact _root_.le_trans (_root_.le_trans this ht) (le_max_right _ _)

/-- The bounding sphere's inside is bounded. -/
theorem bounded_sphereR (R2 : ℚ) : Bornology.IsBounded {z : R3 | qR (sphereR R2) z ≤ 0} := by
  refine (Metric.isBounded_iff_subset_closedBall 0).mpr ⟨max 1 (R2 : ℝ) + 1, fun z hz => ?_⟩
  have e : qR (sphereR R2) z = (z.1 - 1/7) * (z.1 - 1/7) + (z.2.1 - 1/11) * (z.2.1 - 1/11) +
      (z.2.2 - 1/13) * (z.2.2 - 1/13) - R2 := by
    simp only [qR, sphereR, bCentre]; push_cast; ring
  simp only [mem_ofPred_eq] at hz
  rw [e] at hz
  have h1 := sq_le_abs (t := z.1 - 1/7) (R := R2)
    (by nlinarith [mul_self_nonneg (z.2.1 - 1/11), mul_self_nonneg (z.2.2 - 1/13)])
  have h2 := sq_le_abs (t := z.2.1 - 1/11) (R := R2)
    (by nlinarith [mul_self_nonneg (z.1 - 1/7), mul_self_nonneg (z.2.2 - 1/13)])
  have h3 := sq_le_abs (t := z.2.2 - 1/13) (R := R2)
    (by nlinarith [mul_self_nonneg (z.1 - 1/7), mul_self_nonneg (z.2.1 - 1/11)])
  simp only [Metric.mem_closedBall, dist_zero_right, Prod.norm_def, Real.norm_eq_abs, max_le_iff]
  refine ⟨?_, ?_, ?_⟩
  · have := abs_sub_abs_le_abs_sub z.1 (1/7); rw [abs_of_pos (by norm_num : (0:ℝ) < 1/7)] at this
    linarith
  · have := abs_sub_abs_le_abs_sub z.2.1 (1/11)
    rw [abs_of_pos (by norm_num : (0:ℝ) < 1/11)] at this; linarith
  · have := abs_sub_abs_le_abs_sub z.2.2 (1/13)
    rw [abs_of_pos (by norm_num : (0:ℝ) < 1/13)] at this; linarith

/-- **`ExactK.complete`, for the bounded curved ledger.** -/
theorem qledgerB_complete {qs : List Quad.Quadric} {forms : List SForm} {R2 : ℚ} {L : Ledger}
    (h : qledgerB qs forms R2 = some L) (hb : ∀ F ∈ forms, F.Below qs.length) :
    ∀ φ, RealisedK (carriersOf realModel (qs ++ [sphereR R2])) φ → φ.isFacet = true →
      ∀ i, i < L.nb →
        FlipK (carriersOf realModel (qs ++ [sphereR R2]))
          (bodiesOf realModel (qs ++ [sphereR R2]) (boundForms qs.length forms)) φ i →
        φ ∈ L.facets := by
  intro φ hφ hfac i hi hflip
  set Q := qs ++ [sphereR R2]
  set n := qs.length
  have hcs : carriersOf realModel Q = Q.map qR := carriersOf_real Q
  rw [hcs] at hφ hflip
  obtain ⟨ss, hss, hL⟩ := qledger_samples (qledgerB_parts h)
  have hnb : L.nb = forms.length := by rw [hL]; simp [curvedLedger, boundForms]
  rw [hnb] at hi
  have hlenQ : Q.length = n + 1 := by simp [Q, n]
  have hφlen : φ.length = n + 1 := by obtain ⟨x, hx⟩ := hφ; rw [← hx]; simp [covOfK, hlenQ]
  set j := φ.zeroIdx
  have hjz := facet_zero hfac
  have hjl : j < n + 1 := by rw [← hφlen]; exact zeroIdx_lt hfac
  have hbn : n < (Q.map qR).length := by simp [hlenQ]
  -- the body's formula, and membership in it
  have hF : formOf (boundForms n forms) i = .and (formOf forms i) (.lit n .neg) := by
    simp [formOf, boundForms, List.getD_eq_getElem?_getD, hi]
  have hmem : ∀ T x, covCellK (Q.map qR) T x →
      bodiesOf realModel Q (boundForms n forms) i x → T[n]? = some Sgn.neg := by
    intro T x hx hA
    unfold bodiesOf at hA
    rw [hF, hcs] at hA
    obtain ⟨-, c, hc, hsc⟩ := hA
    rw [← hx, getElem?_covOfK, hc]; simp [hsc]
  -- the sign of `φ` at `B`
  obtain ⟨σ, hσ⟩ : ∃ σ, φ[n]? = some σ := ⟨φ[n], List.getElem?_eq_getElem (by omega)⟩
  have hplus_n : j ≠ n → φ.plus[n]? = some σ := fun hjn => by
    simp only [Cov.plus]; rw [List.getElem?_set_ne hjn]; exact hσ
  have hminus_n : j ≠ n → φ.minus[n]? = some σ := fun hjn => by
    simp only [Cov.minus]; rw [List.getElem?_set_ne hjn]; exact hσ
  -- no flip across a side outside `B`
  have hnoflip : (∀ x, covCellK (Q.map qR) φ.plus x →
      ¬ bodiesOf realModel Q (boundForms n forms) i x) →
      (∀ x, covCellK (Q.map qR) φ.minus x → ¬ bodiesOf realModel Q (boundForms n forms) i x) →
      False := by
    intro h1 h2
    apply hflip
    constructor
    · rintro ⟨x, hx, hA⟩; exact absurd hA (h1 x hx)
    · rintro ⟨x, hx, hA⟩; exact absurd hA (h2 x hx)
  by_cases hjn : j = n
  · -- a facet of `B`
    exfalso
    apply hnoflip
    · intro x hx hA
      have := hmem _ x hx hA
      have e : φ.plus[n]? = some Sgn.pos := by
        show (φ.set φ.zeroIdx Sgn.pos)[n]? = _
        rw [show n = φ.zeroIdx from hjn.symm, List.getElem?_set_self (by omega)]
      rw [e] at this; cases this
    · intro x hx hA
      -- the facet's own points would be in the body
      obtain ⟨y, hy⟩ := hφ
      unfold bodiesOf at hA
      rw [hF] at hA
      have hFx := hA.1
      rw [hcs] at hFx
      have hb' : (formOf forms i).Below n := by
        simp only [formOf, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi,
          Option.getD_some]
        exact hb _ (List.getElem_mem hi)
      have hon : (formOf forms i).onCov φ = true := by
        have := ((formOf forms i).den_cell hx).mp hFx
        have e : φ.minus = φ.set n Sgn.neg := by
          show φ.set φ.zeroIdx Sgn.neg = _; rw [show φ.zeroIdx = n from hjn]
        rw [e, SForm.onCov_set_ge _ hb' φ n Sgn.neg (Nat.le_refl _)] at this
        exact this
      have hy' : (formOf forms i).den (Q.map qR) y := ((formOf forms i).den_cell hy).mpr hon
      have hyq : (formOf forms i).den (carriersOf realModel qs) y := by
        have hb'' : (formOf forms i).Below (carriersOf realModel qs).length := by
          simpa [carriersOf] using hb'
        rw [← SForm.den_append (carriersOf realModel qs) [Quadric.evK realModel (sphereR R2)] y _ hb'']
        have : carriersOf realModel qs ++ [Quadric.evK realModel (sphereR R2)] = Q.map qR := by
          rw [← hcs]; simp [carriersOf, Q]
        rw [this]; exact hy'
      have hin := qledgerB_inside realModel h hi hyq
      rw [evK_real] at hin
      have hzn : φ[n]? = some Sgn.zero := by rw [show n = φ.zeroIdx from hjn.symm]; exact hjz
      have := sgn_at hy hbn hzn
      have hQn : (Q.map qR)[n] = qR (sphereR R2) := by simp [Q, n]
      rw [hQn, sgnK_zero_iff] at this
      linarith
  · rcases σ with _ | _ | _
    · -- inside `B`: sampled, both sides realised, so a flip is a piece
      have hjq : j < (Q.map qR).length := by simp [hlenQ]; omega
      obtain ⟨s, hs, hscov⟩ := facet_sampled_input hss (bi := n) hbn
        (by
          have : {z | (Q.map qR)[n] z ≤ 0} = {z | qR (sphereR R2) z ≤ 0} := by
            simp [Q, n, List.getElem_append_right]
          rw [this]; exact bounded_sphereR R2)
        φ hσ (j := j) hjq hjz (facet_unique hfac) hφ
      -- both sides realised, through the frame
      obtain ⟨f, -, -, hsing, -⟩ := samplesOf_frame hss
      have hQj : j < Q.length := by omega
      obtain ⟨hp, hm⟩ := facet_sides hsing (j := j) (by simp [Q]; omega) hjz (facet_unique hfac)
        (realised_frame f hφ)
      have rp := realised_unframe f hp
      have rm := realised_unframe f hm
      rw [← hcs] at rp rm
      unfold FlipK at hflip
      rw [← hcs] at hflip
      rw [qmemTK_iff realModel (by simpa [Cov.plus] using rp),
        qmemTK_iff realModel (by simpa [Cov.minus] using rm)] at hflip
      -- the piece
      rw [hL]
      unfold Ledger.facets
      rw [mem_dedup]
      refine List.mem_map.mpr ⟨(φ, i * Q.length + φ.zeroIdx), ?_, rfl⟩
      simp only [curvedLedger, List.mem_flatMap, List.mem_filter, List.mem_map, List.mem_range,
        bne_iff_ne, ne_eq, decide_eq_true_eq]
      refine ⟨φ, ⟨(mem_dedup _ _).mpr (List.mem_map.mpr ⟨s, hs, hscov⟩), hfac⟩, i,
        ⟨by simpa [boundForms] using hi, fun heq => hflip (by rw [heq])⟩, rfl⟩
    · -- on `B` away from `j`: impossible, a facet has one zero
      exact absurd hσ (facet_unique hfac n (Ne.symm hjn))
    · -- outside `B`
      exfalso
      apply hnoflip
      · intro x hx hA; have := hmem _ x hx hA; rw [hplus_n hjn] at this; cases this
      · intro x hx hA; have := hmem _ x hx hA; rw [hminus_n hjn] at this; cases this

end ForgeCurved
