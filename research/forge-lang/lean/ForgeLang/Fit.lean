/-!
# `Options::fit_sizes` (`interface/forge-lang/src/lib.rs`, `fit_sizes`)

A fillet, chamfer, shell or face fillet the kernel refused, its size driven by a param `p` at
value `top`: eight halvings, then twelve bisection steps, then the value rounded down to three
figures if that still builds. The result is put on the error as its fix, `SetParam p := v`; the
document and the run are not changed (only the error's `fit` field is set).

Values are counted in units of `top / 2^20`: `top` is `2^20`, the halvings are `2^(20-h)`, and
every bisection midpoint is exact (the gap starts at `2^(20-h)` with `h ≤ 8` and halves twelve
times). `builds v` is "the document with `p := v` builds that feature": a function, as P1 needs
of the kernel.

* `fit_builds`: whatever value is returned builds. For any `builds` and any rounding.
* `fit_bracket`: when `top` itself fails, the bisection ends between a value that builds and one
  that fails, one part in 4096 of the value apart.
* `fit_near_max`: if `builds` is downward closed (a smaller size builds whenever a larger one
  does), no value at or above the failing end builds: the fix is within a part in 4096 of the
  largest size that builds, and at most it.
* The fix is a SetParam: `setParam_roundtrip`, `setParam_ids` (`Edits.lean`) say what applying it
  does (a re-read when `p` is a data param).

| Assumption | Counterexample without it |
|---|---|
| Sizes that build are downward closed (for "largest") | `fit_misses_larger`: sizes up to a quarter build, and also 0.625 to 0.75 of `top`; the search returns a quarter, and 0.6875 builds. Still sound (`fit_builds`) |
| `builds` a function of the value (a deterministic kernel) | Without it `fit_builds` has no meaning: the fix was checked on another run |
| Arithmetic exact (f64: `top` times a power of two and the midpoints are exact while `top`'s mantissa has 20 bits to spare) | Not modelled |
-/

namespace ForgeLang.Fit

/-- The halvings: the first of `v, v/2, ..` (at most `k` of them) that builds, with the one above. -/
def halv (b : Nat → Bool) : Nat → Nat → Nat → Option (Nat × Nat)
  | 0, _, _ => none
  | k + 1, v, bad => if b v then some (v, bad) else halv b k (v / 2) v

def bisect (b : Nat → Bool) : Nat → Nat → Nat → Nat × Nat
  | 0, lo, bad => (lo, bad)
  | k + 1, lo, bad => let mid := (lo + bad) / 2; if b mid then bisect b k mid bad else bisect b k lo mid

def top : Nat := 2 ^ 20

/-- The fix, if any (`round` is the three-figure rounding down; any function will do). -/
def fit (b : Nat → Bool) (round : Nat → Nat) : Option Nat :=
  match halv b 8 (top / 2) top with
  | none => none
  | some (lo, bad) =>
    let lo := (bisect b 12 lo bad).1
    let s := round lo
    if 0 < s ∧ s < lo then (if b s then some s else some lo) else some lo

theorem gap (j : Nat) (h : 12 ≤ j) : 2 ^ (j + 1) - 2 ^ j = 2 ^ (j - 12) * 2 ^ 12 := by
  rw [← Nat.pow_add, Nat.sub_add_cancel h, Nat.pow_succ]; omega

theorem halv_spec (b : Nat → Bool) : ∀ (k m lo bad : Nat), k ≤ m + 1 →
    halv b k (2 ^ m) (2 ^ (m + 1)) = some (lo, bad) →
    ∃ j, j ≤ m ∧ m < j + k ∧ lo = 2 ^ j ∧ bad = 2 ^ (j + 1) ∧ b lo = true ∧
      (b (2 ^ (m + 1)) = false → b bad = false)
  | 0, _, _, _, _, h => by simp [halv] at h
  | k + 1, m, lo, bad, hk, h => by
    unfold halv at h
    split at h
    · cases h; exact ⟨m, by omega, by omega, rfl, rfl, by assumption, id⟩
    · rename_i hv
      cases m with
      | zero => cases k <;> simp_all [halv]
      | succ m =>
        have e : 2 ^ (m + 1) / 2 = 2 ^ m := by rw [Nat.pow_succ, Nat.mul_div_cancel _ (by decide)]
        rw [e] at h
        obtain ⟨j, h1, h2, h3, h4, h5, h6⟩ := halv_spec b k m lo bad (by omega) h
        exact ⟨j, by omega, by omega, h3, h4, h5, fun _ => h6 (by simpa using hv)⟩

theorem bisect_spec (b : Nat → Bool) : ∀ (k lo bad g : Nat), lo ≤ bad → bad - lo = g * 2 ^ k →
    b lo = true → (bisect b k lo bad).2 - (bisect b k lo bad).1 = g ∧ lo ≤ (bisect b k lo bad).1 ∧
      b (bisect b k lo bad).1 = true ∧ (b bad = false → b (bisect b k lo bad).2 = false)
  | 0, lo, bad, g, _, hg, hb => by simp_all [bisect]
  | k + 1, lo, bad, g, hl, hg, hb => by
    have hm : (lo + bad) / 2 - lo = g * 2 ^ k ∧ bad - (lo + bad) / 2 = g * 2 ^ k := by
      have h2 : bad - lo = 2 * (g * 2 ^ k) := by
        rw [hg, Nat.pow_succ, ← Nat.mul_assoc, Nat.mul_comm (g * 2 ^ k) 2]
      constructor <;> omega
    unfold bisect
    simp only
    split
    · rename_i hmid
      obtain ⟨a1, a2, a3, a4⟩ := bisect_spec b k ((lo + bad) / 2) bad g (by omega) hm.2 hmid
      exact ⟨a1, by omega, a3, a4⟩
    · rename_i hmid
      obtain ⟨a1, a2, a3, a4⟩ := bisect_spec b k lo ((lo + bad) / 2) g (by omega) hm.1 hb
      exact ⟨a1, a2, a3, fun _ => a4 (by simpa using hmid)⟩

/-- Whatever `fit` returns builds. -/
theorem fit_builds (b : Nat → Bool) (round : Nat → Nat) {v : Nat} (h : fit b round = some v) :
    b v = true := by
  unfold fit at h
  split at h
  · cases h
  · rename_i lo bad hh
    obtain ⟨j, hj, hj', rfl, rfl, hlo, -⟩ := halv_spec b 8 19 _ _ (by omega) (by simpa [top] using hh)
    have hg := gap j (by omega)
    obtain ⟨-, -, hb, -⟩ := bisect_spec b 12 _ _ _ (Nat.le_of_lt (Nat.pow_lt_pow_right (by decide) (by omega))) hg hlo
    simp only at h
    split at h
    · split at h <;> (cases h; first | assumption | exact hb)
    · cases h; exact hb

/-- Downward-closed sizes: the fix is within a part in 4096 of the largest size that builds. -/
theorem fit_near_max (b : Nat → Bool) (round : Nat → Nat)
    (mono : ∀ x y, x ≤ y → b y = true → b x = true) (htop : b top = false) {v : Nat}
    (h : fit b round = some v) :
    b v = true ∧ ∃ lo bad, v ≤ lo ∧ b lo = true ∧ (bad - lo) * 4096 ≤ lo ∧ ∀ w, bad ≤ w → b w = false := by
  refine ⟨fit_builds b round h, ?_⟩
  unfold fit at h
  split at h
  · cases h
  · rename_i lo bad hh
    obtain ⟨j, hj, hj', rfl, rfl, hlo, hbad⟩ := halv_spec b 8 19 _ _ (by omega) (by simpa [top] using hh)
    have hg := gap j (by omega)
    obtain ⟨a1, a2, a3, a4⟩ := bisect_spec b 12 _ _ _ (Nat.le_of_lt (Nat.pow_lt_pow_right (by decide) (by omega))) hg hlo
    have e : 2 ^ (j - 12) * 4096 = 2 ^ j := by
      rw [show (4096 : Nat) = 2 ^ 12 by rfl, ← Nat.pow_add, Nat.sub_add_cancel (by omega)]
    refine ⟨_, (bisect b 12 (2 ^ j) (2 ^ (j + 1))).2, ?_, a3, by rw [a1, e]; exact a2, ?_⟩
    · simp only at h
      split at h
      · rename_i hs; split at h <;> (cases h; omega)
      · cases h; exact Nat.le_refl _
    · intro w hw
      cases hb : b w
      · rfl
      · have := a4 (hbad (by simpa [top] using htop))
        rw [mono _ _ hw hb] at this; cases this

/-- Not downward closed: sizes up to a quarter build, and 0.625 to 0.75 of `top`. The fix is
a quarter of `top`; 0.6875 builds. -/
def gappy (w : Nat) : Bool := w ≤ 2 ^ 18 || (655360 ≤ w && w ≤ 786432)

theorem fit_misses_larger : fit gappy id = some 262144 ∧ gappy 720896 = true := by decide

end ForgeLang.Fit
