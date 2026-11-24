import ForgeSheetCore.ExpansionInst
import ForgeSheetCore.RefineLineGap

/-!
# `exact::line_gap` is exact

`line_gap` decides `sign(orient(a, b, c)² − l²|b − a|²)`: a float filter (proven in
RefineLineGap.lean), else expansions:
`o = ux·vy − uy·vx`, `n = ux² + uy²`, `s = o·o − (l·l)·n`, each difference `diffL`, each product
`mulL`, each sum `addL`, the sign `signL`. Here:
* `budgetG`, `add_gen`, `mul_gen`: the sum and product facts of ExactModel.lean for any grid level
  and lengths up to `2¹¹` (the product `o·o` has up to 512 components);
* `line_model`: under the expansion theory, the expansion's sign is `sgnQ (lineQ …)`;
* `line_gap_spec`: the translated `line_gap` returns `sgnQ (lineQ …)` on the domain (inputs on
  the `2⁻²⁰⁰` grid, within `2¹⁰⁰`), both branches;
* `line_gap_exact`: the same with the expansion theory discharged (ExpansionInst.lean).
-/

namespace forge_sheet_core

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP

theorem uq_tiny : uq ≤ 1 / 2 ^ 40 := by rw [uq_def]; norm_num

/-- The grow budget for lengths up to `2¹¹`. -/
theorem budgetG {M : ℚ} {N k : ℕ} (hM : 0 ≤ M) (hN : N ≤ 2048) (hk : k ≤ 2048) :
    M * (1 + 8 * N * k * uq) + 6 * N * k * ηq ≤ 2 * M + 1 ∧ (N : ℚ) * k * (8 * uq) ≤ 1 := by
  have hN' : (N : ℚ) ≤ 2048 := by exact_mod_cast hN
  have hk' : (k : ℚ) ≤ 2048 := by exact_mod_cast hk
  have h0 : (0 : ℚ) ≤ N := Nat.cast_nonneg _
  have h1 : (0 : ℚ) ≤ k := Nat.cast_nonneg _
  have hu := uq_tiny
  have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
  have nk : (N : ℚ) * k ≤ 2048 * 2048 := mul_le_mul hN' hk' h1 (by norm_num)
  have nk0 : (0 : ℚ) ≤ N * k := by positivity
  have a : 8 * (N : ℚ) * k * uq ≤ 1 := by
    have h2 : (N : ℚ) * k * uq ≤ (2048 * 2048) * (1 / 2 ^ 40) := mul_le_mul nk hu hu0 (by norm_num)
    have e : 8 * (N : ℚ) * k * uq = 8 * (N * k * uq) := by ring
    have : (8 : ℚ) * ((2048 * 2048) * (1 / 2 ^ 40)) ≤ 1 := by norm_num
    rw [e]; exact le_trans (mul_le_mul_of_nonneg_left h2 (by norm_num)) this
  have b : 6 * (N : ℚ) * k * ηq ≤ 1 := by
    have h3 : (N : ℚ) * k * ηq ≤ N * k * uq := mul_le_mul_of_nonneg_left ηq_le_uq nk0
    have e : 6 * (N : ℚ) * k * ηq = 6 * (N * k * ηq) := by ring
    have e2 : 8 * (N : ℚ) * k * uq = 8 * (N * k * uq) := by ring
    rw [e2] at a; rw [e]
    have : 6 * (N * k * ηq) ≤ 8 * (N * k * uq) := by
      have := mul_le_mul_of_nonneg_left h3 (by norm_num : (0 : ℚ) ≤ 6)
      have : (0 : ℚ) ≤ N * k * uq := by positivity
      linarith
    exact le_trans this a
  refine ⟨?_, by have e : (N : ℚ) * k * (8 * uq) = 8 * N * k * uq := by ring
                 rw [e]; exact a⟩
  have := mul_le_mul_of_nonneg_left a hM
  have e : M * (1 + 8 * N * k * uq) = M + M * (8 * N * k * uq) := by ring
  rw [e]; linarith

/-- A sum of two expansions on the `2^L` grid (the first carrying the invariant). -/
theorem add_gen (Tth : ExpansionTheory) (L : ℤ) (hL : -1074 ≤ L) {A B : List _root_.F64}
    (hPA : Tth.P A) (hA : ∀ x ∈ A, Good L x) (hB : ∀ x ∈ B, Good L x) {X : ℚ}
    (hm : massL A + massL B ≤ X) (hX : 2 * X + 1 ≤ Kmax) (hl : A.length + B.length ≤ 2048) :
    Tth.P (addL A B) ∧ (∀ x ∈ addL A B, Good L x) ∧ esumL (addL A B) = esumL A + esumL B ∧
      massL (addL A B) ≤ 2 * X + 1 ∧ (addL A B).length ≤ A.length + B.length := by
  have h0 : 0 ≤ massL A + massL B := by have := massL_nonneg A; have := massL_nonneg B; linarith
  obtain ⟨bA, bB⟩ := budgetG (M := massL A + massL B) (N := A.length + B.length) (k := B.length) h0
    hl (by omega)
  obtain ⟨m0, m1, m2, m3⟩ := growMany_good L hL (A.length + B.length) Tth.P
    (Tth.grow L hL) B A hPA hA hB le_rfl (by exact_mod_cast bB) (by
      have : (↑(A.length + B.length) : ℚ) = (A.length : ℚ) + B.length := by push_cast; ring
      linarith)
  have e : addL A B = growMany A B := rfl
  rw [e]
  refine ⟨m0, m1, m2, by linarith, growMany_len A B⟩

/-- The rounding slack of a product's terms, crudely: within twice the masses' product. -/
theorem slack2 {a b : ℚ} {m n : ℕ} (hab : 1 ≤ a * b) (hm : m ≤ 32) (hn : n ≤ 32) :
    (1 + 2 * uq) * (a * b) + 2 * m * n * ηq ≤ 2 * (a * b) := by
  have hu := uq_tiny
  have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
  have hη : ηq ≤ uq := ηq_le_uq
  have hη0 : (0 : ℚ) ≤ ηq := ηq_pos.le
  have hm' : (m : ℚ) ≤ 32 := by exact_mod_cast hm
  have hn' : (n : ℚ) ≤ 32 := by exact_mod_cast hn
  have h0 : (0 : ℚ) ≤ m := Nat.cast_nonneg _
  have h1 : (0 : ℚ) ≤ n := Nat.cast_nonneg _
  have mn : (m : ℚ) * n ≤ 1024 := by nlinarith
  have a1 : 2 * uq * (a * b) ≤ (1 / 2) * (a * b) :=
    mul_le_mul_of_nonneg_right (by linarith) (by linarith)
  have a2 : 2 * (m : ℚ) * n * ηq ≤ 1 / 2 := by
    have : (m : ℚ) * n * ηq ≤ 1024 * (1 / 2 ^ 40) :=
      mul_le_mul mn (le_trans hη hu) hη0 (by norm_num)
    nlinarith
  nlinarith

/-- A product of two expansions on the `2^La` and `2^Lb` grids, within the operand bound. -/
theorem mul_gen (Tth : ExpansionTheory) {La Lb : ℤ} (hΛ : -1074 ≤ La + Lb) {D E : List _root_.F64}
    (hD : ∀ x ∈ D, Good La x) (hE : ∀ x ∈ E, Good Lb x) {MD ME : ℚ}
    (hmD : massL D ≤ MD) (hmE : massL E ≤ ME) (hMD : MD ≤ Amax) (hME : ME ≤ Amax)
    (h1 : 1 ≤ MD * ME) (hlD : D.length ≤ 32) (hlE : E.length ≤ 32)
    (hK : 4 * (MD * ME) + 1 ≤ Kmax) :
    let M := mulL D E
    Tth.P M ∧ (∀ x ∈ M, Good (La + Lb) x) ∧ esumL M = esumL D * esumL E ∧
      massL M ≤ 4 * (MD * ME) + 1 ∧ M.length ≤ 2 * D.length * E.length := by
  intro M
  have cD : ∀ x ∈ D, Good La x ∧ |x.val| ≤ Amax := fun x h =>
    ⟨hD x h, le_trans (comp_le_mass h) (le_trans hmD hMD)⟩
  have cE : ∀ x ∈ E, Good Lb x ∧ |x.val| ≤ Amax := fun x h =>
    ⟨hE x h, le_trans (comp_le_mass h) (le_trans hmE hME)⟩
  obtain ⟨t1, t2, t3⟩ := prodTerms_good hΛ D E cD cE
  have hlen : (prodTerms D E).length ≤ 2048 := by
    rw [prodTerms_len]; have := Nat.mul_le_mul (Nat.mul_le_mul_left 2 hlD) hlE; omega
  have hmD0 := massL_nonneg D
  have hmE0 := massL_nonneg E
  have hmT : massL (prodTerms D E) ≤ 2 * (MD * ME) := by
    have a : massL D * massL E ≤ MD * ME := mul_le_mul hmD hmE hmE0 (le_trans hmD0 hmD)
    have := slack2 (a := MD) (b := ME) (m := D.length) (n := E.length) h1 hlD hlE
    have : (1 + 2 * uq) * (massL D * massL E) ≤ (1 + 2 * uq) * (MD * ME) :=
      mul_le_mul_of_nonneg_left a (by have := uq_pos; linarith)
    linarith
  obtain ⟨bA, bB⟩ := budgetG (M := 0 + massL (prodTerms D E)) (N := 2048)
    (k := (prodTerms D E).length) (by have := massL_nonneg (prodTerms D E); linarith) le_rfl hlen
  obtain ⟨m0, m1, m2, m3⟩ := growMany_good (La + Lb) hΛ 2048 Tth.P (Tth.grow (La + Lb) hΛ)
    (prodTerms D E) [] Tth.nil (by simp) t1 (by simp; omega) (by simpa using bB)
    (by simp only [massL_nil] at bA ⊢; linarith)
  simp only [M]; rw [mulL_eq]
  refine ⟨m0, m1, by rw [m2, t2]; simp, ?_, ?_⟩
  · simp only [massL_nil, zero_add] at m3 bA; linarith
  · have := growMany_len [] (prodTerms D E); rw [prodTerms_len] at this; simp at this; omega

theorem C0_A2 : 128 * (C0 * C0) ≤ Amax := by
  rw [C0_def, Amax_def, ← two_zpow_add]
  have : (128 : ℚ) = 2 ^ (7 : ℤ) := by norm_num
  rw [this, ← two_zpow_add]; exact two_zpow_le (by norm_num)

/-- The orientation's expansion, `ux·vy − uy·vx`. -/
def lgO (ax ay bx «by» cx cy : _root_.F64) : List _root_.F64 :=
  addL (mulL (diffL bx ax) (diffL cy ay)) (negL (mulL (diffL «by» ay) (diffL cx ax)))

/-- The squared length's expansion, `ux² + uy²`. -/
def lgN (ax ay bx «by» : _root_.F64) : List _root_.F64 :=
  addL (mulL (diffL bx ax) (diffL bx ax)) (mulL (diffL «by» ay) (diffL «by» ay))

/-- **`line_gap`'s exact branch on the models**: with the expansion theory, the expansion
`line_gap` forms has the sign of the exact `orient(a, b, c)² − l²|b − a|²`. -/
theorem line_model (Tth : ExpansionTheory) (ax ay bx «by» cx cy l : _root_.F64)
    (hax : Good (-200) ax ∧ |ax.val| ≤ C0) (hay : Good (-200) ay ∧ |ay.val| ≤ C0)
    (hbx : Good (-200) bx ∧ |bx.val| ≤ C0) (hby : Good (-200) «by» ∧ |by.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hl : Good (-200) l ∧ |l.val| ≤ C0) :
    signL (addL (mulL (lgO ax ay bx «by» cx cy) (lgO ax ay bx «by» cx cy))
      (negL (mulL (mulL (ofL l) (ofL l)) (lgN ax ay bx «by»)))) =
      sgnQ (lineQ ax ay bx «by» cx cy l) := by
  unfold lgO lgN
  obtain ⟨ux1, ux2, ux3, ux4⟩ := diff_facts hbx hax
  obtain ⟨uy1, uy2, uy3, uy4⟩ := diff_facts hby hay
  obtain ⟨vx1, vx2, vx3, vx4⟩ := diff_facts hcx hax
  obtain ⟨vy1, vy2, vy3, vy4⟩ := diff_facts hcy hay
  obtain ⟨l1, l2, l3, l4⟩ := ofL_good hl.1
  have hC := C0_ge
  have hK := C0_K
  have hK4 := C0_K4
  have hA2 := C0_A2
  have hs2 : C0 ≤ C0 * C0 := C0_sq
  have hc2 : (256 : ℚ) ≤ C0 * C0 := by have := C0_16; nlinarith
  have hc4 : C0 * C0 ≤ C0 * C0 * (C0 * C0) := by nlinarith
  -- the orientation and the squared length
  obtain ⟨a0, a1, a2, a3, a4⟩ := mul_facts Tth ux1 vy1 ux3 vy3 ux4 vy4
  obtain ⟨b0, b1, b2, b3, b4⟩ := mul_facts Tth uy1 vx1 uy3 vx3 uy4 vx4
  obtain ⟨nb1, nb2, nb3, nb4⟩ := negL_good b1
  obtain ⟨o0, o1, o2, o3, o4⟩ := add_facts Tth a0 a1 nb1 (X := 42 * (C0 * C0) + 2)
    (by rw [nb3]; linarith) (by linarith) (by omega)
  obtain ⟨c0, c1, c2, c3, c4⟩ := mul_facts Tth ux1 ux1 ux3 ux3 ux4 ux4
  obtain ⟨d0, d1, d2, d3, d4⟩ := mul_facts Tth uy1 uy1 uy3 uy3 uy4 uy4
  obtain ⟨n0, n1, n2, n3, n4⟩ := add_facts Tth c0 c1 d1 (X := 42 * (C0 * C0) + 2)
    (by linarith) (by linarith) (by omega)
  have hmO := le_trans o3 (show _ ≤ 85 * (C0 * C0) by linarith)
  have hmN := le_trans n3 (show _ ≤ 85 * (C0 * C0) by linarith)
  have eOO : 85 * (C0 * C0) * (85 * (C0 * C0)) = 7225 * (C0 * C0 * (C0 * C0)) := by ring
  have eEN : 22 * (C0 * C0) * (85 * (C0 * C0)) = 1870 * (C0 * C0 * (C0 * C0)) := by ring
  have hc4' : (256 : ℚ) ≤ C0 * C0 * (C0 * C0) := le_trans hc2 hc4
  -- o·o
  obtain ⟨p0, p1, p2, p3, p4⟩ := mul_gen Tth (La := -400) (Lb := -400) (by norm_num) o1 o1
    hmO hmO (by linarith) (by linarith) (by rw [eOO]; linarith only [hc4'])
    (by omega) (by omega) (by rw [eOO]; linarith only [hc4', hK4])
  -- (l·l)·n
  obtain ⟨e0, e1, e2, e3, e4⟩ := mul_facts Tth l1 l1 (by rw [l3]; linarith [hl.2])
    (by rw [l3]; linarith [hl.2]) (by omega) (by omega)
  have hmE : massL (mulL (ofL l) (ofL l)) ≤ 22 * (C0 * C0) := by linarith
  obtain ⟨q0, q1, q2, q3, q4⟩ := mul_gen Tth (La := -400) (Lb := -400) (by norm_num) e1 n1
    hmE hmN (by linarith) (by linarith) (by rw [eEN]; linarith only [hc4'])
    (by omega) (by omega) (by rw [eEN]; linarith only [hc4', hK4])
  obtain ⟨nq1, nq2, nq3, nq4⟩ := negL_good q1
  -- the difference
  norm_num at p1 q1 nq1
  have hO16 := le_trans o4 (show _ ≤ 16 by rw [nb4]; omega)
  have hN16 := le_trans n4 (show _ ≤ 16 by omega)
  have hl1 := le_trans p4 (Nat.mul_le_mul (Nat.mul_le_mul_left 2 hO16) hO16)
  have hl2 := le_trans q4 (Nat.mul_le_mul (Nat.mul_le_mul_left 2 e4) hN16)
  obtain ⟨s0, s1, s2, s3, s4⟩ := add_gen Tth (-800) (by norm_num) p0 p1 nq1
    (X := 4 * (85 * (C0 * C0) * (85 * (C0 * C0))) + 1 + (4 * (22 * (C0 * C0) * (85 * (C0 * C0))) + 1))
    (by rw [nq3]; linarith only [p3, q3]) (by rw [eOO, eEN]; linarith only [hc4', hK4])
    (by rw [nq4]; omega)
  rw [Tth.sign _ s0 (fun x h => (s1 x h).1), s2, nq2, p2, q2, e2, o2, n2, nb2, a2, b2, c2, d2,
    l2, ux2, uy2, vx2, vy2]
  congr 1; unfold lineQ; ring

/-! ## The shipped `line_gap` -/

theorem mulL_len (e o : List _root_.F64) : (mulL e o).length ≤ 2 * e.length * o.length := by
  have := mulL_len_aux e o []; simpa [mulL] using this

theorem diffL_len (a b : _root_.F64) : (diffL a b).length ≤ 2 := by
  have := growL_len (ofL a) (FP.neg b); have := ofL_len a; unfold diffL; omega

theorem negL_len (e : List _root_.F64) : (negL e).length = e.length := by simp [negL]

theorem mul_len_le {e o : List _root_.F64} {m n : ℕ} (he : e.length ≤ m) (ho : o.length ≤ n) :
    (mulL e o).length ≤ 2 * m * n :=
  le_trans (mulL_len e o) (Nat.mul_le_mul (Nat.mul_le_mul_left 2 he) ho)

/-- **`line_gap` is exact** (the shipped `exact::line_gap`), under the expansion theory: on the
domain it returns `+1`, `−1` or `+0` as `orient(a, b, c)² − l²|b − a|²` of the input doubles is
positive, negative or zero. -/
theorem line_gap_spec (Tth : ExpansionTheory) (ax ay bx «by» cx cy l : _root_.F64)
    (hax : Good (-200) ax ∧ |ax.val| ≤ C0) (hay : Good (-200) ay ∧ |ay.val| ≤ C0)
    (hbx : Good (-200) bx ∧ |bx.val| ≤ C0) (hby : Good (-200) «by» ∧ |by.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hl : Good (-200) l ∧ |l.val| ≤ C0) :
    exact.line_gap ax ay bx «by» cx cy l ⦃ s => s = sgnQ (lineQ ax ay bx «by» cx cy l) ⦄ := by
  obtain ⟨hv, hT, hflt⟩ := line_gap_filter hax hay hbx hby hcx hcy hl
  unfold exact.line_gap
  simp only [lift, bind_ok, core.f64.F64.abs_eq, FILTER_eq, F64.Ops.sub_def, F64.Ops.add_def,
    F64.Ops.mul_def]
  split
  · rename_i hg
    rw [sgn_eq]; simp only [WP.spec_ok]
    simp only [F64.Ops.gt_def] at hg
    rw [lt_iff hT (by simpa using hv), abs_val] at hg
    obtain ⟨k1, k2⟩ := hflt hg
    exact sgnM_of_sign hv k1 k2
  · have hmax := usize_big
    have d1 := diffL_len bx ax
    have d2 := diffL_len «by» ay
    have d3 := diffL_len cx ax
    have d4 := diffL_len cy ay
    have dl := ofL_len l
    have m1 := mul_len_le d1 d4
    have m2 := mul_len_le d2 d3
    have m3 := mul_len_le d1 d1
    have m4 := mul_len_le d2 d2
    have me := mul_len_le dl dl
    have hO := le_trans (addL_len (mulL (diffL bx ax) (diffL cy ay))
      (negL (mulL (diffL «by» ay) (diffL cx ax)))) (show _ ≤ 16 by simp [negL]; omega)
    have hN := le_trans (addL_len (mulL (diffL bx ax) (diffL bx ax))
      (mulL (diffL «by» ay) (diffL «by» ay))) (show _ ≤ 16 by omega)
    have mOO := mul_len_le hO hO
    have mEN := mul_len_le me hN
    step with exp_diff_spec bx ax as ⟨eux, heux⟩
    step with exp_diff_spec «by» ay as ⟨euy, heuy⟩
    step with exp_diff_spec cx ax as ⟨evx, hevx⟩
    step with exp_diff_spec cy ay as ⟨evy, hevy⟩
    step with exp_mul_spec (alloc.vec.Vec.deref eux) (alloc.vec.Vec.deref evy)
      (by simp [heux]; omega) (by simp [hevy]; omega) as ⟨o1, ho1⟩
    step with exp_mul_spec (alloc.vec.Vec.deref euy) (alloc.vec.Vec.deref evx)
      (by simp [heuy]; omega) (by simp [hevx]; omega) as ⟨v1, hv1⟩
    step with exp_neg_spec (alloc.vec.Vec.deref v1) (by simp [hv1, heuy, hevx]; omega) as ⟨v2, hv2⟩
    step with exp_add_spec (alloc.vec.Vec.deref o1) (alloc.vec.Vec.deref v2)
      (by simp [ho1, hv2, hv1, heux, hevy, heuy, hevx, negL]; omega) as ⟨o2, ho2⟩
    step with exp_mul_spec (alloc.vec.Vec.deref eux) (alloc.vec.Vec.deref eux)
      (by simp [heux]; omega) (by simp [heux]; omega) as ⟨n1, hn1⟩
    step with exp_mul_spec (alloc.vec.Vec.deref euy) (alloc.vec.Vec.deref euy)
      (by simp [heuy]; omega) (by simp [heuy]; omega) as ⟨v3, hv3⟩
    step with exp_add_spec (alloc.vec.Vec.deref n1) (alloc.vec.Vec.deref v3)
      (by simp [hn1, hv3, heux, heuy]; omega) as ⟨n2, hn2⟩
    step with exp_of_spec l as ⟨el, hel⟩
    step with exp_mul_spec (alloc.vec.Vec.deref o2) (alloc.vec.Vec.deref o2)
      (by simp [ho2, hv2, hv1, ho1, heux, hevy, heuy, hevx]; omega)
      (by simp [ho2, hv2, hv1, ho1, heux, hevy, heuy, hevx]; omega) as ⟨s15, hs15⟩
    step with exp_mul_spec (alloc.vec.Vec.deref el) (alloc.vec.Vec.deref el)
      (by simp [hel]; omega) (by simp [hel]; omega) as ⟨v4, hv4⟩
    step with exp_mul_spec (alloc.vec.Vec.deref v4) (alloc.vec.Vec.deref n2)
      (by simp [hv4, hel]; omega) (by simp [hn2, hn1, hv3, heux, heuy]; omega) as ⟨v5, hv5⟩
    step with exp_neg_spec (alloc.vec.Vec.deref v5)
      (by simp [hv5, hv4, hel, hn2, hn1, hv3, heux, heuy]; omega) as ⟨v6, hv6⟩
    step with exp_add_spec (alloc.vec.Vec.deref s15) (alloc.vec.Vec.deref v6)
      (by simp [hs15, ho2, hv2, hv1, ho1, heux, hevy, heuy, hevx, hv6, hv5, hv4, hel, hn2, hn1,
        hv3, negL_len]; omega) as ⟨s23, hs23⟩
    step with exp_sign_spec (alloc.vec.Vec.deref s23) as ⟨g, hg⟩
    simp only [hg, deref_val, hs23, hs15, ho2, hv2, hv1, ho1, hv6, hv5, hv4, hn2, hn1, hv3,
      heux, heuy, hevx, hevy, hel]
    have hm := line_model Tth ax ay bx «by» cx cy l hax hay hbx hby hcx hcy hl
    simp only [lgO, lgN] at hm
    exact hm

/-- **`exact::line_gap` is exact**, with no assumption beyond the domain. -/
theorem line_gap_exact (ax ay bx «by» cx cy l : _root_.F64)
    (hax : Good (-200) ax ∧ |ax.val| ≤ C0) (hay : Good (-200) ay ∧ |ay.val| ≤ C0)
    (hbx : Good (-200) bx ∧ |bx.val| ≤ C0) (hby : Good (-200) «by» ∧ |by.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hl : Good (-200) l ∧ |l.val| ≤ C0) :
    exact.line_gap ax ay bx «by» cx cy l ⦃ s => s = sgnQ (lineQ ax ay bx «by» cx cy l) ⦄ :=
  line_gap_spec expansionTheory ax ay bx «by» cx cy l hax hay hbx hby hcx hcy hl

end forge_sheet_core
