/-
Dropping pocket pass loops (H.dropLoop, verify/forge-cam/RESUME.md). `ops::pocket_in` leaves out
an outer loop L of pass k (0 < s ≤ r) when the band (L ⊕ B_r) ∩ S lies in R_{k+1} ⊕ B_r, decided
by exact booleans at run time: S the in-process stock's box, R_{k+1} = {depth ≥ r + (k+1)·s}.
The kept passes still cover the opening R ∘ B_r inside S, so the void the op claims stands.

* `passes_cover_from m`: a point within r of a point at least r + m·s deep is within r of a
  point of some pass j ≥ m (`passes_cover` is m = 0).
* `drop_cover`: whatever is dropped from each pass (`drops k`, any set: one loop, several, in any
  passes), if every dropped point's band within S lies in R_{k+1} ⊕ B_r, every point of the
  opening inside S is within r of a kept point (a pass's point not dropped). By induction on how
  many passes lie between the covering pass and the deepest one within reach (depth is
  1-Lipschitz, so a pass within r of q has r + j·s ≤ depth q + r).
-/
import ForgeCam.Pocket

open Metric Set

namespace ForgeCam.Pocket

variable (R : Set E2)

theorem depth_le_add (x y : E2) : depth R x ≤ depth R y + dist x y :=
  infDist_le_infDist_add_dist

/-- **Passes from m on.** If r + m·s ≤ depth p and dist q p ≤ r, some pass j ≥ m has a point
within r of q. -/
theorem passes_cover_from (hR : (Rᶜ).Nonempty) {r s : ℝ} (hs : 0 < s) (hsr : s ≤ r) (m : ℕ)
    {q p : E2} (hp : r + m * s ≤ depth R p) (hqp : dist q p ≤ r) :
    ∃ j : ℕ, m ≤ j ∧ ∃ z ∈ pass R r s j, dist q z ≤ r := by
  have hr : 0 < r := lt_of_lt_of_le hs hsr
  by_cases hq : r + m * s ≤ depth R q
  · -- q is itself that deep: the level just above it on the way to its nearest outside point
    obtain ⟨b, hb, hdb⟩ := exists_mem_closure_infDist_eq_dist hR q
    set d := depth R q with hd
    have hdq : d = dist q b := hdb
    set k := ⌊(d - r) / s⌋₊ with hk
    have hk1 : (k : ℝ) * s ≤ d - r := by
      have := Nat.floor_le (a := (d - r) / s) (div_nonneg (by nlinarith [(Nat.cast_nonneg m : (0:ℝ) ≤ m)]) hs.le)
      rwa [le_div_iff₀ hs] at this
    have hk2 : d - r < (k + 1) * s := by
      have := Nat.lt_floor_add_one ((d - r) / s)
      rwa [div_lt_iff₀ hs] at this
    have hmk : m ≤ k := by
      apply Nat.le_floor
      rw [le_div_iff₀ hs]; linarith
    have hb0 : depth R b = 0 := by
      unfold depth; exact infDist_zero_of_mem_closure hb
    have hlev : r + k * s ∈ uIcc (depth R q) (depth R (q + (1:ℝ) • (b - q))) := by
      rw [one_smul, add_sub_cancel, hb0]
      exact Set.mem_uIcc.mpr (Or.inr ⟨by positivity, by linarith⟩)
    obtain ⟨t, ht, hz⟩ := exists_level R q b _ hlev
    refine ⟨k, hmk, q + t • (b - q), hz, ?_⟩
    rw [dist_lineMap_left q b t ht]
    have hzb : depth R (q + t • (b - q)) ≤ dist (q + t • (b - q)) b := by
      unfold depth
      rw [← infDist_closure]; exact infDist_le_dist_of_mem hb
    rw [hz, dist_lineMap_right q b t ht, ← hdq] at hzb
    rw [← hdq]
    nlinarith
  · -- q is shallower: the level r + m·s on the way to p
    push Not at hq
    have hlev : r + m * s ∈ uIcc (depth R q) (depth R (q + (1:ℝ) • (p - q))) := by
      rw [one_smul, add_sub_cancel]
      exact Set.mem_uIcc.mpr (Or.inl ⟨hq.le, hp⟩)
    obtain ⟨t, ht, hz⟩ := exists_level R q p _ hlev
    refine ⟨m, le_rfl, q + t • (p - q), hz, ?_⟩
    rw [dist_lineMap_left q p t ht]
    nlinarith [ht.1, ht.2, dist_nonneg (x := q) (y := p)]

/-- **Dropping loops keeps the cover (H.dropLoop).** `drops k` is what the op leaves out of pass
k. If every dropped point's band within S lies in R_{k+1} ⊕ B_r, every point of the opening inside
S is within r of a kept point. -/
theorem drop_cover (hR : (Rᶜ).Nonempty) {r s : ℝ} (hs : 0 < s) (hsr : s ≤ r) (S : Set E2)
    (drops : ℕ → Set E2)
    (hdrop : ∀ k, ∀ q ∈ S, ∀ z ∈ drops k ∩ pass R r s k, dist q z ≤ r →
      ∃ p, r + (k + 1 : ℕ) * s ≤ depth R p ∧ dist q p ≤ r) :
    ∀ q ∈ opening R r ∩ S, ∃ j : ℕ, ∃ z ∈ pass R r s j \ drops j, dist q z ≤ r := by
  intro q ⟨hq, hqS⟩
  -- every pass within r of q is at most B: r + j·s = depth z ≤ depth q + r
  set B := ⌊depth R q / s⌋₊ with hB
  have hbound : ∀ j : ℕ, ∀ z ∈ pass R r s j, dist q z ≤ r → j ≤ B := by
    intro j z hz hd
    have h1 := depth_le_add R z q
    rw [dist_comm] at h1
    have : (j : ℝ) * s ≤ depth R q := by
      have hz' : depth R z = r + j * s := hz
      linarith
    apply Nat.le_floor
    rw [le_div_iff₀ hs]; exact this
  -- by induction on B - j
  have key : ∀ n : ℕ, ∀ j : ℕ, B - j ≤ n → (∃ z ∈ pass R r s j, dist q z ≤ r) →
      ∃ j' : ℕ, ∃ z ∈ pass R r s j' \ drops j', dist q z ≤ r := by
    intro n
    induction n with
    | zero =>
      intro j hj ⟨z, hz, hd⟩
      by_cases hzd : z ∈ drops j
      · obtain ⟨p, hp, hqp⟩ := hdrop j q hqS z ⟨hzd, hz⟩ hd
        obtain ⟨j', hj', z', hz', hd'⟩ := passes_cover_from R hR hs hsr (j + 1) hp hqp
        have := hbound j' z' hz' hd'
        have := hbound j z hz hd
        omega
      · exact ⟨j, z, ⟨hz, hzd⟩, hd⟩
    | succ n ih =>
      intro j hj ⟨z, hz, hd⟩
      by_cases hzd : z ∈ drops j
      · obtain ⟨p, hp, hqp⟩ := hdrop j q hqS z ⟨hzd, hz⟩ hd
        obtain ⟨j', hj', z', hz', hd'⟩ := passes_cover_from R hR hs hsr (j + 1) hp hqp
        exact ih j' (by omega) ⟨z', hz', hd'⟩
      · exact ⟨j, z, ⟨hz, hzd⟩, hd⟩
  obtain ⟨k, z, hz, hd⟩ := passes_cover R hR hs hsr q hq
  exact key (B - k) k le_rfl ⟨z, hz, hd⟩

end ForgeCam.Pocket
