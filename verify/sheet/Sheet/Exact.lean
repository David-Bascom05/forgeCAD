import Sheet.Region

/-!
# The exact fallbacks: `near`, `within`, `inDisc` as signs of sums of double products

The float filters (Float.lean) decide most queries; the rest go to an exact fallback. This file
is the fallback's statement, in the form region.rs builds it from forge-math's predicates.rs
expansion helpers (the ones orient2d's exact stage uses, proven by the aeneas session, with
verify/ieee754's `two_sum_exact` and `two_product_exact` underneath):

* a difference of doubles is an exact pair `dh + dl = a − b` (TwoSum);
* a product of doubles is an exact pair (TwoProduct), so every product of two pair components
  is a sum of two doubles;
* the decision is the sign of the exact sum of a list of such terms (an expansion), which the
  helpers compute exactly (expansion sum, then the sign of its largest nonzero component).

Here a term list is a `List ℚ` and its value `esum`. `near_terms`, `disc_terms`, `within_terms`
are the lists region.rs forms, and `*_exact` say the sign of their sum is the spec's decision
(Region.lean's `near`, `inDisc`, `within`). `filtered_eq`: a filter whose every verdict is the
exact one (Float.lean's `filter_sound`, `within_filter_sound`), backed by this fallback, is the
exact rule.
-/

namespace Sheet.Exact

open Sheet.Region

/-- An expansion's value. -/
def esum (e : List ℚ) : ℚ := e.sum

/-- All products of two expansions' components: the product's expansion. -/
def prodList (a b : List ℚ) : List ℚ := a.flatMap fun x => b.map (x * ·)

theorem esum_prod (a b : List ℚ) : esum (prodList a b) = esum a * esum b := by
  induction a with
  | nil => simp [esum, prodList]
  | cons x xs ih =>
      simp only [esum, prodList, List.flatMap_cons, List.sum_append, List.sum_cons] at ih ⊢
      rw [ih, List.sum_map_mul_left, List.map_id']; ring

theorem esum_append (a b : List ℚ) : esum (a ++ b) = esum a + esum b := by
  simp [esum, List.sum_append]

theorem esum_neg (a : List ℚ) : esum (a.map (- ·)) = - esum a := by
  simp only [esum]; induction a with
  | nil => simp
  | cons x xs ih => simp only [List.map_cons, List.sum_cons, ih]; ring

/-- A filter backed by a fallback: the filter's verdict when it gives one. -/
def filtered (f : Option Bool) (e : Bool) : Bool := f.getD e

/-- **Filter plus exact fallback is the exact rule**, when every verdict the filter gives is. -/
theorem filtered_eq {f : Option Bool} {e : Bool} (h : ∀ v, f = some v → v = e) :
    filtered f e = e := by
  cases f with
  | none => rfl
  | some v => exact h v rfl

/-- A squared difference from its exact pair: `(dh + dl)² = dh² + 2 dh dl + dl²`. -/
def sq_terms (dh dl : ℚ) : List ℚ := [dh * dh, 2 * (dh * dl), dl * dl]

theorem esum_sq (dh dl : ℚ) : esum (sq_terms dh dl) = (dh + dl) ^ 2 := by
  simp [esum, sq_terms]; ring

/-- `|p − q|² − r²` from the differences' exact pairs and `r²`'s pair `(rh, rl)`. -/
def norm_terms (xh xl yh yl rh rl : ℚ) : List ℚ :=
  sq_terms xh xl ++ sq_terms yh yl ++ [-rh, -rl]

theorem esum_norm {p q : P} {r xh xl yh yl rh rl : ℚ} (hx : xh + xl = p.1 - q.1)
    (hy : yh + yl = p.2 - q.2) (hr : rh + rl = r * r) :
    esum (norm_terms xh xl yh yl rh rl) = norm2 (sub p q) - r * r := by
  simp only [norm_terms, esum_append, esum_sq, hx, hy, norm2, sub]
  simp only [esum, List.sum_cons, List.sum_nil]; rw [← hr]; ring

/-- **`near`, exactly**: `near L p q` is the sign of the terms' sum (`≤ 0`). -/
theorem near_exact {L : ℚ} {p q : P} {xh xl yh yl lh ll : ℚ} (hx : xh + xl = p.1 - q.1)
    (hy : yh + yl = p.2 - q.2) (hl : lh + ll = L * L) :
    near L p q = decide (esum (norm_terms xh xl yh yl lh ll) ≤ 0) := by
  rw [esum_norm hx hy hl]; simp only [near]
  exact decide_eq_decide.mpr ⟨fun h => by linarith, fun h => by linarith⟩

/-- **`inDisc`, exactly**: the sign of the terms' sum, `< 0` in; `= 0` by the tie rule on the
coordinates (comparisons of doubles, exact). -/
theorem disc_exact {p c : P} {r xh xl yh yl rh rl : ℚ} (hx : xh + xl = p.1 - c.1)
    (hy : yh + yl = p.2 - c.2) (hr : rh + rl = r * r) :
    inDisc p c r = (decide (esum (norm_terms xh xl yh yl rh rl) < 0) ||
      (decide (esum (norm_terms xh xl yh yl rh rl) = 0) &&
        (decide (p.2 < c.2) || (decide (p.2 = c.2) && decide (p.1 < c.1))))) := by
  rw [esum_norm hx hy hr]; simp only [inDisc]
  congr 1
  · exact decide_eq_decide.mpr ⟨fun h => by linarith, fun h => by linarith⟩
  · congr 1; exact decide_eq_decide.mpr ⟨fun h => by linarith, fun h => by linarith⟩

/-- `within`'s terms: `O² − L² N`, from `O`'s expansion `o` (orient2d's exact stage), `|b − a|²`'s
terms `n` (`sq_terms` of the differences' pairs) and `L²`'s pair `l`. -/
def within_terms (o n l : List ℚ) : List ℚ := prodList o o ++ (prodList l n).map (- ·)

/-- **`within`, exactly**: the sign of the terms' sum (`≤ 0`). -/
theorem within_exact {L : ℚ} {a b c : P} {o n l : List ℚ} (ho : esum o = orient a b c)
    (hn : esum n = norm2 (sub b a)) (hl : esum l = L * L) :
    within L a b c = decide (esum (within_terms o n l) ≤ 0) := by
  simp only [within_terms, esum_append, esum_neg, esum_prod, ho, hn, hl, within]
  exact decide_eq_decide.mpr ⟨fun h => by linarith, fun h => by linarith⟩

/-- `|b − a|²` from the differences' pairs. -/
theorem esum_n {a b : P} {xh xl yh yl : ℚ} (hx : xh + xl = b.1 - a.1) (hy : yh + yl = b.2 - a.2) :
    esum (sq_terms xh xl ++ sq_terms yh yl) = norm2 (sub b a) := by
  rw [esum_append, esum_sq, esum_sq, hx, hy]; simp only [norm2, sub]; ring

end Sheet.Exact
