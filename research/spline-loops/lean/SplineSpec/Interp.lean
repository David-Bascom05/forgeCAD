/-
The through-spline exists and is unique (`interp_exists_unique`): for parameters strictly increasing
(so for consecutive points distinct), there is exactly one C2 cubic spline through the points with
the end conditions asked, natural, a slope, or periodic. It is the Hermite form of the slope system's
solution, which is what `Spline::through` builds.
-/
import SplineSpec.Slopes

namespace SplineSpec

set_option linter.unusedSectionVars false
set_option linter.unusedTactic false
set_option linter.unnecessarySeqFocus false

variable {K : Type} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

/-! ### The piece algebra -/

theorem hermite_d1start (a b ma mb h : K) (hh : h ≠ 0) : d1start (hermite a b ma mb h) h = ma := by
  unfold d1start hermite; field_simp; ring

theorem hermite_d1end (a b ma mb h : K) (hh : h ≠ 0) : d1end (hermite a b ma mb h) h = mb := by
  unfold d1end hermite; field_simp; ring

/-- C2 between two Hermite pieces is the middle row. -/
theorem hermite_c2_iff (q0 q1 q2 m0 m1 m2 h0 h1 : K) (h0p : h0 ≠ 0) (h1p : h1 ≠ 0) :
    d2end (hermite q0 q1 m0 m1 h0) h0 = d2start (hermite q1 q2 m1 m2 h1) h1 ↔
      h1 * m0 + (1 + 1) * (h0 + h1) * m1 + h0 * m2
        - (1 + 1 + 1) * (h1 * ((q1 - q0) / h0) + h0 * ((q2 - q1) / h1)) = 0 := by
  unfold d2end d2start hermite
  simp only
  rw [div_eq_div_iff (mul_ne_zero h0p h0p) (mul_ne_zero h1p h1p)]
  constructor
  · intro H
    have : (h1 * m0 + (1 + 1) * (h0 + h1) * m1 + h0 * m2
        - (1 + 1 + 1) * (h1 * ((q1 - q0) / h0) + h0 * ((q2 - q1) / h1))) * (h0 * h1) = 0 := by
      field_simp
      try simp only [hm1] at *
      try simp only [hii] at *
      try simp only [hjj] at *
      linear_combination (3 : K) * H
    rcases mul_eq_zero.1 this with h | h
    · exact h
    · exact absurd h (mul_ne_zero h0p h1p)
  · intro H
    have e : (h1 * m0 + (1 + 1) * (h0 + h1) * m1 + h0 * m2
        - (1 + 1 + 1) * (h1 * ((q1 - q0) / h0) + h0 * ((q2 - q1) / h1))) * (h0 * h1) = 0 := by
      rw [H, zero_mul]
    field_simp at e
    try simp only [hm1] at *
    try simp only [hii] at *
    try simp only [hjj] at *
    linear_combination e / 3

/-- A natural start is the natural first row. -/
theorem hermite_natural_start (q0 q1 m0 m1 h : K) (hh : h ≠ 0) :
    d2start (hermite q0 q1 m0 m1 h) h = 0 ↔ (1 + 1) * m0 + 1 * m1 - (1 + 1 + 1) * ((q1 - q0) / h) = 0 := by
  unfold d2start hermite
  simp only
  rw [div_eq_zero_iff, or_iff_left (mul_ne_zero hh hh)]
  constructor
  · intro H; field_simp; linear_combination (-3 : K) * H
  · intro H; field_simp at H; linear_combination (-1 / 3 : K) * H

/-- A natural end is the natural last row. -/
theorem hermite_natural_end (q0 q1 m0 m1 h : K) (hh : h ≠ 0) :
    d2end (hermite q0 q1 m0 m1 h) h = 0 ↔ 1 * m0 + (1 + 1) * m1 - (1 + 1 + 1) * ((q1 - q0) / h) = 0 := by
  unfold d2end hermite
  simp only
  rw [div_eq_zero_iff, or_iff_left (mul_ne_zero hh hh)]
  constructor
  · intro H; field_simp; linear_combination (3 : K) * H
  · intro H; field_simp at H; linear_combination (1 / 3 : K) * H

/-- A piece with these end values and end slopes is the Hermite piece. -/
theorem piece_eq_hermite (B : Piece K) (h : K) (hh : h ≠ 0) :
    ∀ k ≤ 3, B k = hermite (B 0) (B 3) (d1start B h) (d1end B h) h k := by
  intro k hk
  interval_cases k <;> simp [hermite, d1start, d1end] <;> field_simp <;> ring

/-! ### Existence: the Hermite pieces of a solution -/

/-- Parameters of the spline and the closing point of a periodic one. -/
structure DataOk (τ q : ℕ → K) (m : ℕ) (e : Ends K) : Prop extends SysOk τ m e where
  closes : e = .periodic → q m = q 0

theorem h_ne {τ : ℕ → K} {m : ℕ} {e : Ends K} (h : SysOk τ m e) {i : ℕ} (hi : i < m) :
    τ (i + 1) - τ i ≠ 0 := ne_of_gt (sub_pos.2 (h.pos i hi))

theorem exists_of_slopes {τ q : ℕ → K} {m : ℕ} {e : Ends K} (hd : DataOk τ q m e) (x : ℕ → K)
    (hx : ∀ i < unknowns m e, residual τ q m e x i = 0) :
    IsInterp τ q m e (piecesOf τ q m e x) := by
  have hs := hd.toSysOk
  have hsz := hs.size
  refine ⟨fun i _ => ⟨rfl, rfl⟩, fun i hi => ⟨?_, ?_⟩, ?_⟩
  · simp only [piecesOf]
    rw [hermite_d1end _ _ _ _ _ (h_ne hs (by omega)), hermite_d1start _ _ _ _ _ (h_ne hs hi)]
  · simp only [piecesOf]
    rw [hermite_c2_iff _ _ _ _ _ _ _ _ (h_ne hs (by omega)) (h_ne hs hi)]
    have hr := hx (i + 1) (by cases e <;> simp [unknowns] <;> omega)
    rw [residual_eq] at hr
    cases e with
    | open_ s f =>
      simp only [rows, unknowns, slopeAt, leftIx, rightIx] at hr ⊢
      have h1 : i + 1 ≠ 0 := by omega
      have h2 : i + 1 ≠ m := by omega
      have h3 : i + 1 + 1 ≠ m + 1 := by omega
      simp only [h1, h2, h3, ite_false, Nat.add_sub_cancel] at hr
      try simp only [hm1] at *
      try simp only [hii] at *
      try simp only [hjj] at *
      linear_combination hr
    | periodic =>
      simp only [rows, unknowns, slopeAt, leftIx, rightIx] at hr ⊢
      have h1 : i + 1 ≠ 0 := by omega
      have h2 : i ≠ m := by omega
      have h2' : i + 1 ≠ m := by omega
      simp only [h1, h2, h2', ite_false, Nat.add_sub_cancel] at hr ⊢
      by_cases h4 : i + 1 + 1 = m
      · simp only [h4, ite_true] at hr ⊢
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination hr
      · simp only [h4, ite_false] at hr ⊢
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination hr
  · cases e with
    | open_ s f =>
      simp only [unknowns] at hsz
      refine ⟨?_, ?_⟩
      · have hr := hx 0 (by simp [unknowns])
        rw [residual_eq] at hr
        cases s with
        | natural =>
          simp only [piecesOf, slopeAt]
          rw [hermite_natural_start _ _ _ _ _ (h_ne hs (by omega))]
          simp only [rows, leftIx, rightIx, unknowns, ite_true] at hr
          have : (0 : ℕ) + 1 ≠ m + 1 := by omega
          simp only [this, ite_false, zero_add] at hr
          try simp only [hm1] at *
          try simp only [hii] at *
          try simp only [hjj] at *
          linear_combination hr
        | slope v =>
          simp only [piecesOf, slopeAt]
          rw [hermite_d1start _ _ _ _ _ (h_ne hs (by omega))]
          simp only [rows, ite_true] at hr
          try simp only [hm1] at *
          try simp only [hii] at *
          try simp only [hjj] at *
          linear_combination hr
      · have hr := hx m (by simp [unknowns])
        rw [residual_eq] at hr
        have hm0 : m ≠ 0 := by omega
        have hm1 : m - 1 + 1 = m := by omega
        have hlast : τ m - τ (m - 1) ≠ 0 := by
          have := h_ne hs (i := m - 1) (by omega); rwa [hm1] at this
        cases f with
        | natural =>
          simp only [piecesOf, slopeAt]
          rw [hm1, hermite_natural_end _ _ _ _ _ hlast]
          simp only [rows, leftIx, rightIx, unknowns, hm0, ite_false, ite_true] at hr
          try simp only [hm1] at *
          try simp only [hii] at *
          try simp only [hjj] at *
          linear_combination hr
        | slope v =>
          simp only [piecesOf, slopeAt]
          rw [hm1, hermite_d1end _ _ _ _ _ hlast]
          simp only [rows, hm0, ite_false, ite_true] at hr
          try simp only [hm1] at *
          try simp only [hii] at *
          try simp only [hjj] at *
          linear_combination hr
    | periodic =>
      simp only [unknowns] at hsz
      have hq := hd.closes rfl
      have hm1 : m - 1 + 1 = m := by omega
      have hlast : τ m - τ (m - 1) ≠ 0 := by
        have := h_ne hs (i := m - 1) (by omega); rwa [hm1] at this
      refine ⟨?_, ?_⟩
      · simp only [piecesOf, slopeAt]
        rw [hm1, hermite_d1end _ _ _ _ _ hlast,
          hermite_d1start _ _ _ _ _ (h_ne hs (by omega))]
        have : m - 1 ≠ m := by omega
        have h0 : (0 : ℕ) ≠ m := by omega
        simp [this, h0]
      · have hr := hx 0 (by simp [unknowns]; omega)
        rw [residual_eq] at hr
        simp only [piecesOf, slopeAt]
        rw [hm1]
        have e := (hermite_c2_iff (q (m - 1)) (q m) (q 1) (if m - 1 = m then x 0 else x (m - 1))
          (if m = m then x 0 else x m) (if (1 : ℕ) = m then x 0 else x 1) (τ m - τ (m - 1))
          (τ (0 + 1) - τ 0) hlast
          (h_ne hs (by omega))).2
        have h0 : (0 : ℕ) ≠ m := by omega
        have h01 : (0 : ℕ) + 1 ≠ m := by omega
        have h1m : (1 : ℕ) ≠ m := by omega
        have hmm : m - 1 ≠ m := by omega
        simp only [rows, leftIx, rightIx, unknowns, ite_true, h01, ite_false] at hr
        rw [hq] at e
        simp only [h0, h1m, hmm, ite_false, ite_true, zero_add] at e ⊢
        rw [hm1] at hr
        rw [hq]
        apply e
        rw [hq] at hr
        simp only [zero_add] at hr
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination hr

/-! ### Uniqueness: every interpolant is the Hermite form of a solution -/

/-- The slopes a spline's pieces have at its points: each piece's start, and the last piece's end
for an open one. -/
def slopesOf (τ : ℕ → K) (m : ℕ) (B : ℕ → Piece K) (i : ℕ) : K :=
  if i < m then d1start (B i) (τ (i + 1) - τ i) else d1end (B (m - 1)) (τ m - τ (m - 1))

theorem pieces_of_interp {τ q : ℕ → K} {m : ℕ} {e : Ends K} (hd : DataOk τ q m e) (B : ℕ → Piece K)
    (hB : IsInterp τ q m e B) :
    ∀ i < m, ∀ k ≤ 3, B i k = piecesOf τ q m e (slopesOf τ m B) i k := by
  have hs := hd.toSysOk
  have hsz := hs.size
  obtain ⟨hpts, hmid, hends⟩ := hB
  intro i hi k hk
  rw [piece_eq_hermite (B i) _ (h_ne hs hi) k hk]
  simp only [piecesOf]
  have hm1 : m - 1 + 1 = m := by omega
  have a0 := (hpts i hi).1
  have a3 := (hpts i hi).2
  -- the start slope is the piece's own; the end slope is the next piece's start (or the end's)
  have hst : slopeAt m e (slopesOf τ m B) i = d1start (B i) (τ (i + 1) - τ i) := by
    cases e <;> simp [slopeAt, slopesOf, hi, Nat.ne_of_lt hi]
  have hen : slopeAt m e (slopesOf τ m B) (i + 1) = d1end (B i) (τ (i + 1) - τ i) := by
    by_cases hlt : i + 1 < m
    · have := (hmid i hlt).1
      cases e <;> simp [slopeAt, slopesOf, hlt, Nat.ne_of_lt hlt, this]
    · have him : i + 1 = m := by omega
      have hi' : i = m - 1 := by omega
      cases e with
      | open_ s f => simp [slopeAt, slopesOf, hi', hm1]
      | periodic =>
        have h0 : 0 < m := by omega
        simp only [slopeAt, slopesOf, him, ite_true, h0]
        simpa [hi', hm1] using hends.1.symm
  rw [hst, hen, a0, a3]

theorem slopes_of_interp {τ q : ℕ → K} {m : ℕ} {e : Ends K} (hd : DataOk τ q m e) (B : ℕ → Piece K)
    (hB : IsInterp τ q m e B) :
    ∀ i < unknowns m e, residual τ q m e (slopesOf τ m B) i = 0 := by
  have hs := hd.toSysOk
  have hsz := hs.size
  have hP := pieces_of_interp hd B hB
  have hB' := hB
  obtain ⟨hpts, hmid, hends⟩ := hB'
  -- rewrite every condition on B as one on the Hermite pieces of its slopes
  have hpiece : ∀ i < m, B i = fun k => if k ≤ 3 then piecesOf τ q m e (slopesOf τ m B) i k else B i k := by
    intro i hi; funext k; split_ifs with hk
    · exact hP i hi k hk
    · rfl
  have d1s : ∀ i < m, d1start (B i) (τ (i + 1) - τ i) = d1start (piecesOf τ q m e (slopesOf τ m B) i) (τ (i + 1) - τ i) := by
    intro i hi; unfold d1start; rw [hP i hi 0 (by omega), hP i hi 1 (by omega)]
  have d1e : ∀ i < m, d1end (B i) (τ (i + 1) - τ i) = d1end (piecesOf τ q m e (slopesOf τ m B) i) (τ (i + 1) - τ i) := by
    intro i hi; unfold d1end; rw [hP i hi 2 (by omega), hP i hi 3 (by omega)]
  have d2s : ∀ i < m, d2start (B i) (τ (i + 1) - τ i) = d2start (piecesOf τ q m e (slopesOf τ m B) i) (τ (i + 1) - τ i) := by
    intro i hi; unfold d2start; rw [hP i hi 0 (by omega), hP i hi 1 (by omega), hP i hi 2 (by omega)]
  have d2e : ∀ i < m, d2end (B i) (τ (i + 1) - τ i) = d2end (piecesOf τ q m e (slopesOf τ m B) i) (τ (i + 1) - τ i) := by
    intro i hi; unfold d2end; rw [hP i hi 1 (by omega), hP i hi 2 (by omega), hP i hi 3 (by omega)]
  clear hpiece
  set x := slopesOf τ m B with hxdef
  have hm1 : m - 1 + 1 = m := by
    cases e <;> simp at hsz <;> omega
  have hlast : τ m - τ (m - 1) ≠ 0 := by
    have := h_ne hs (i := m - 1) (by cases e <;> simp at hsz <;> omega); rwa [hm1] at this
  intro i hi
  rw [residual_eq]
  -- a middle row is C2 at its point
  have mid : ∀ j, 0 < j → j < m →
      (τ (j + 1) - τ j) * slopeAt m e x (j - 1) + (1 + 1) * ((τ (j - 1 + 1) - τ (j - 1)) + (τ (j + 1) - τ j)) * slopeAt m e x j
        + (τ (j - 1 + 1) - τ (j - 1)) * slopeAt m e x (j + 1)
        - (1 + 1 + 1) * ((τ (j + 1) - τ j) * ((q (j - 1 + 1) - q (j - 1)) / (τ (j - 1 + 1) - τ (j - 1)))
            + (τ (j - 1 + 1) - τ (j - 1)) * ((q (j + 1) - q j) / (τ (j + 1) - τ j))) = 0 := by
    intro j hj0 hjm
    have hjj : j - 1 + 1 = j := by omega
    have c2 := (hmid (j - 1) (by omega)).2
    rw [d2e _ (by omega), hjj, d2s _ hjm] at c2
    simp only [piecesOf, hjj] at c2
    have := (hermite_c2_iff _ _ _ _ _ _ _ _ (by simpa [hjj] using h_ne hs (i := j - 1) (by omega))
      (h_ne hs hjm)).1 c2
    simpa [hjj] using this
  cases e with
  | open_ s f =>
    simp only [unknowns] at hi hsz
    by_cases h0 : i = 0
    · subst h0
      cases s with
      | natural =>
        have c := hends.1
        rw [d2s 0 (by omega)] at c
        simp only [piecesOf] at c
        have := (hermite_natural_start _ _ _ _ _ (h_ne hs (by omega))).1 c
        simp only [rows, leftIx, rightIx, ite_true, slopeAt] at this ⊢
        have : (0 : ℕ) + 1 ≠ m + 1 := by omega
        simp only [this, ite_false, zero_add, unknowns]
        simp only [slopeAt, zero_add] at *
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination (by assumption : (1 + 1) * x 0 + 1 * x 1 - (1 + 1 + 1) * ((q 1 - q 0) / (τ 1 - τ 0)) = 0)
      | slope v =>
        have c := hends.1
        rw [d1s 0 (by omega)] at c
        simp only [piecesOf] at c
        rw [hermite_d1start _ _ _ _ _ (h_ne hs (by omega))] at c
        simp only [rows, ite_true, slopeAt] at c ⊢
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination c
    · by_cases hm : i = m
      · subst hm
        have hm0 : i ≠ 0 := h0
        cases f with
        | natural =>
          have c := hends.2
          rw [d2e _ (by omega)] at c
          simp only [piecesOf] at c
          rw [hm1] at c
          have := (hermite_natural_end _ _ _ _ _ hlast).1 c
          simp only [rows, leftIx, rightIx, hm0, ite_false, ite_true, unknowns, slopeAt] at this ⊢
          rw [hm1]
          try simp only [hm1] at *
          try simp only [hii] at *
          try simp only [hjj] at *
          linear_combination this
        | slope v =>
          have c := hends.2
          rw [d1e _ (by omega)] at c
          simp only [piecesOf] at c
          rw [hm1, hermite_d1end _ _ _ _ _ hlast] at c
          simp only [rows, hm0, ite_false, ite_true, slopeAt] at c ⊢
          try simp only [hm1] at *
          try simp only [hii] at *
          try simp only [hjj] at *
          linear_combination c
      · have := mid i (by omega) (by omega)
        simp only [rows, leftIx, rightIx, unknowns, h0, hm, ite_false, slopeAt] at this ⊢
        have h3 : i + 1 ≠ m + 1 := by omega
        simp only [h3, ite_false]
        have hii : i - 1 + 1 = i := by omega
        rw [hii] at this
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination this
  | periodic =>
    simp only [unknowns] at hi hsz
    have hq := hd.closes rfl
    by_cases h0 : i = 0
    · subst h0
      -- C2 across the seam
      have c := hends.2
      rw [d2e _ (by omega), d2s 0 (by omega)] at c
      simp only [piecesOf] at c
      have hmm : m - 1 ≠ m := by omega
      have h1m : (1 : ℕ) ≠ m := by omega
      have h0m : (0 : ℕ) ≠ m := by omega
      have h01 : (0 : ℕ) + 1 ≠ m := by omega
      simp only [slopeAt, ite_true, hmm, h1m, h0m, ite_false, hm1, hq, zero_add] at c
      have e := (hermite_c2_iff (q (m - 1)) (q 0) (q 1) (x (m - 1)) (x 0) (x 1) (τ m - τ (m - 1))
        (τ 1 - τ 0) hlast (by simpa using h_ne hs (i := 0) (by omega))).1 c
      simp only [rows, leftIx, rightIx, unknowns, ite_true, h01, ite_false, slopeAt, hmm, h1m, h0m,
        zero_add, hm1, hq] at ⊢
      linear_combination e
    · have := mid i (by omega) hi
      have hii : i - 1 + 1 = i := by omega
      have him : i ≠ m := by omega
      have hi1 : i - 1 ≠ m := by omega
      simp only [rows, leftIx, rightIx, h0, ite_false, slopeAt, him, hi1] at this ⊢
      rw [hii] at this
      by_cases h4 : i + 1 = m
      · simp only [h4, ite_true] at this ⊢
        simp only [unknowns, ite_true]
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination this
      · simp only [h4, ite_false] at this ⊢
        simp only [unknowns, h4, ite_false]
        try simp only [hm1] at *
        try simp only [hii] at *
        try simp only [hjj] at *
        linear_combination this

/-- **Existence and uniqueness of the through-spline.** For parameters strictly increasing (and
`q m = q 0` when periodic), there is a C2 cubic spline through the points with the end conditions,
and any two agree on every piece. -/
theorem interp_exists_unique {τ q : ℕ → K} {m : ℕ} {e : Ends K} (hd : DataOk τ q m e) :
    (∃ B, IsInterp τ q m e B) ∧
    ∀ B B', IsInterp τ q m e B → IsInterp τ q m e B' → ∀ i < m, ∀ k ≤ 3, B i k = B' i k := by
  refine ⟨?_, ?_⟩
  · obtain ⟨x, hx⟩ := slopes_exist (q := q) hd.toSysOk
    exact ⟨_, exists_of_slopes hd x hx⟩
  · intro B B' hB hB' i hi k hk
    rw [pieces_of_interp hd B hB i hi k hk, pieces_of_interp hd B' hB' i hi k hk]
    have hu := slopes_unique hd.toSysOk _ _ (slopes_of_interp hd B hB) (slopes_of_interp hd B' hB')
    have hsz := hd.size
    have hn : ∀ j ≤ m, slopeAt m e (slopesOf τ m B) j = slopeAt m e (slopesOf τ m B') j := by
      intro j hj
      cases e with
      | open_ s f => exact hu j (by simp [unknowns]; omega)
      | periodic =>
        simp only [slopeAt]
        simp only at hsz
        split_ifs
        · exact hu 0 (by simp [unknowns]; omega)
        · exact hu j (by simp [unknowns]; omega)
    simp only [piecesOf]
    rw [hn i (by omega), hn (i + 1) (by omega)]

/-- And the spline passes through every point: each piece starts at `q i` and ends at `q (i+1)`
(the Bezier form's end points are the curve's, `bez3 b 0 = b 0`, `bez3 b 1 = b 3`). -/
theorem interp_through {τ q : ℕ → K} {m : ℕ} {e : Ends K} (B : ℕ → Piece K) (hB : IsInterp τ q m e B) :
    ∀ i < m, bez3 (B i) 0 = q i ∧ bez3 (B i) 1 = q (i + 1) := by
  intro i hi
  obtain ⟨a, b⟩ := hB.1 i hi
  constructor
  · simp [bez3, a]
  · simp [bez3, b]

end SplineSpec
