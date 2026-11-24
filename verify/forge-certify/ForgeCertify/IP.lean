import ForgeCertify.Sign

/-!
# Interval parity: the engine of every crossing argument

Affine functions `g s = c + m s` on an interval `[s₀, s₁]`. The set where all of them are positive
is an interval; it holds `s₀` or `s₁` or both or neither, and each of its ends inside `(s₀, s₁)`
is a root of one of them at which every other is positive. So:

`ip` : `[all positive at s₀] ⊕ [all positive at s₁]` is the parity of the functions with a root
inside `(s₀, s₁)` at which no other function is `≤ 0` (an *event*), provided none vanishes at an
end and no two vanish at one inner point.

Each crossing lemma of the certificate is this, with the functions read off a triangle (its edge
sides and its plane) along a segment or a ray, and its events read as crossings of edges.
-/

namespace ForgeCertify

/-- An affine function of one rational. -/
structure Aff where
  c : Rat
  m : Rat
  deriving DecidableEq, Inhabited

namespace Aff

def ev (g : Aff) (s : Rat) : Rat := g.c + g.m * s

/-- Its root (meaningful when `m ≠ 0`). -/
def root (g : Aff) : Rat := -g.c / g.m

theorem ev_eq (g : Aff) (h : g.m ≠ 0) (s : Rat) : g.ev s = g.m * (s - g.root) := by
  unfold ev root; grind

theorem ev_root (g : Aff) (h : g.m ≠ 0) : g.ev g.root = 0 := by
  rw [ev_eq g h]; grind

/-- An affine function is positive between two points where it is positive. -/
theorem pos_between (g : Aff) {s₀ s₁ s : Rat} (h0 : 0 < g.ev s₀) (h1 : 0 < g.ev s₁)
    (hs0 : s₀ ≤ s) (hs1 : s ≤ s₁) : 0 < g.ev s := by
  unfold ev at *
  rcases lt_trichotomy' g.m 0 with hm | hm | hm
  · have := Rat.mul_le_mul_of_nonneg_left (a := s) (b := s₁) hs1 (c := -g.m) (by grind)
    grind
  · have e : g.m * s₀ = 0 := by rw [hm]; grind
    have e' : g.m * s = 0 := by rw [hm]; grind
    grind
  · have := Rat.mul_le_mul_of_nonneg_left (a := s₀) (b := s) hs0 (c := g.m) (by grind)
    grind

theorem neg_between (g : Aff) {s₀ s₁ s : Rat} (h0 : g.ev s₀ < 0) (h1 : g.ev s₁ < 0)
    (hs0 : s₀ ≤ s) (hs1 : s ≤ s₁) : g.ev s < 0 := by
  have := pos_between ⟨-g.c, -g.m⟩ (s := s) (s₀ := s₀) (s₁ := s₁) (by unfold ev at *; grind)
    (by unfold ev at *; grind) hs0 hs1
  unfold ev at *; grind

/-- A sign change from `+` at `s₀` to `-` at `s₁ > s₀`: a root between, and the sign on each
side of it. -/
theorem down (g : Aff) {s₀ s₁ : Rat} (h01 : s₀ < s₁) (h0 : 0 < g.ev s₀) (h1 : g.ev s₁ < 0) :
    g.m < 0 ∧ s₀ < g.root ∧ g.root < s₁ ∧ ∀ s, (0 < g.ev s ↔ s < g.root) := by
  have hm : g.m < 0 := by
    apply Classical.byContradiction; intro hm
    have := Rat.mul_le_mul_of_nonneg_left (a := s₀) (b := s₁) (Rat.le_of_lt h01) (c := g.m) (by grind)
    unfold ev at h0 h1; grind
  have hm0 : g.m ≠ 0 := by grind
  have e := ev_eq g hm0
  refine ⟨hm, ?_, ?_, fun s => ?_⟩
  · rw [e] at h0; have := (mul_pos_iff' g.m (s₀ - g.root)).mp h0; grind
  · rw [e] at h1; have := (mul_neg_iff' g.m (s₁ - g.root)).mp h1; grind
  · rw [e, mul_pos_iff']; grind

theorem up (g : Aff) {s₀ s₁ : Rat} (h01 : s₀ < s₁) (h0 : g.ev s₀ < 0) (h1 : 0 < g.ev s₁) :
    0 < g.m ∧ s₀ < g.root ∧ g.root < s₁ ∧ ∀ s, (0 < g.ev s ↔ g.root < s) := by
  have hm : 0 < g.m := by
    apply Classical.byContradiction; intro hm
    have := Rat.mul_le_mul_of_nonneg_left (a := s₀) (b := s₁) (Rat.le_of_lt h01) (c := -g.m) (by grind)
    unfold ev at h0 h1; grind
  have hm0 : g.m ≠ 0 := by grind
  have e := ev_eq g hm0
  refine ⟨hm, ?_, ?_, fun s => ?_⟩
  · rw [e] at h0; have := (mul_neg_iff' g.m (s₀ - g.root)).mp h0; grind
  · rw [e] at h1; have := (mul_pos_iff' g.m (s₁ - g.root)).mp h1; grind
  · rw [e, mul_pos_iff']; grind

end Aff

/-- All positive at `s`. -/
def pos (gs : List Aff) (s : Rat) : Bool := gs.all fun g => decide (0 < g.ev s)

/-- How many are `≤ 0` at `s`. -/
def nonpos (gs : List Aff) (s : Rat) : Nat := gs.countP fun g => decide (g.ev s ≤ 0)

/-- `g`'s root is an event of `gs` in `(s₀, s₁)`: inside, and every other function positive
there (`g` itself is the one function of `gs` not positive). -/
def isEv (s₀ s₁ : Rat) (gs : List Aff) (g : Aff) : Bool :=
  decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && nonpos gs g.root == 1

/-- No two of them vanish at one point of `(s₀, s₁)`. -/
def Distinct (gs : List Aff) (s₀ s₁ : Rat) : Prop :=
  ∀ s, s₀ < s → s < s₁ → gs.countP (fun g => decide (g.ev s = 0)) ≤ 1

theorem nonpos_zero_iff (gs : List Aff) (s : Rat) : nonpos gs s = 0 ↔ pos gs s = true := by
  unfold nonpos pos
  rw [List.countP_eq_zero, List.all_eq_true]
  constructor
  · intro h g hg; have := h g hg; simp at this ⊢; grind
  · intro h g hg; have := h g hg; simp at this ⊢; grind

theorem isEv_cons_self (s₀ s₁ : Rat) (g : Aff) (gs : List Aff) :
    isEv s₀ s₁ (g :: gs) g =
      (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) := by
  unfold isEv
  by_cases hm : g.m = 0
  · simp [hm]
  · have h0 := Aff.ev_root g hm
    have : nonpos (g :: gs) g.root = nonpos gs g.root + 1 := by
      unfold nonpos; rw [List.countP_cons]; simp [h0]
    rw [this]
    have hz := nonpos_zero_iff gs g.root
    cases hp : pos gs g.root
    · have : nonpos gs g.root ≠ 0 := fun h => by rw [hz.mp h] at hp; cases hp
      simp; grind
    · have := hz.mpr hp; simp [this]

theorem isEv_cons_other (s₀ s₁ : Rat) (g h : Aff) (gs : List Aff) (hh : h ∈ gs) :
    isEv s₀ s₁ (g :: gs) h = (isEv s₀ s₁ gs h && decide (0 < g.ev h.root)) := by
  unfold isEv
  by_cases hm : h.m = 0
  · simp [hm]
  · have h0 := Aff.ev_root h hm
    have hpos : 0 < nonpos gs h.root := by
      unfold nonpos; apply List.countP_pos_iff.mpr; exact ⟨h, hh, by simp [h0]⟩
    by_cases hg : 0 < g.ev h.root
    · have : nonpos (g :: gs) h.root = nonpos gs h.root := by
        unfold nonpos; rw [List.countP_cons]; simp; grind
      rw [this]; simp [hg]
    · have : nonpos (g :: gs) h.root = nonpos gs h.root + 1 := by
        unfold nonpos; rw [List.countP_cons]; simp; grind
      rw [this]; simp [hg]; grind

theorem distinct_tail {g : Aff} {gs : List Aff} {s₀ s₁ : Rat} (h : Distinct (g :: gs) s₀ s₁) :
    Distinct gs s₀ s₁ := fun s h0 h1 => by
  have := h s h0 h1; rw [List.countP_cons] at this; grind

theorem distinct_sub {gs : List Aff} {s₀ s₁ t₀ t₁ : Rat} (h : Distinct gs s₀ s₁) (h0 : s₀ ≤ t₀)
    (h1 : t₁ ≤ s₁) : Distinct gs t₀ t₁ := fun s a b => h s (by grind) (by grind)

/-- At a root of `g` inside the interval, no other function vanishes. -/
theorem distinct_root {g : Aff} {gs : List Aff} {s₀ s₁ : Rat} (h : Distinct (g :: gs) s₀ s₁)
    (hm : g.m ≠ 0) (h0 : s₀ < g.root) (h1 : g.root < s₁) : ∀ k ∈ gs, k.ev g.root ≠ 0 := by
  intro k hk hz
  have := h g.root h0 h1
  rw [List.countP_cons] at this
  have hpos : 0 < gs.countP (fun g' => decide (g'.ev g.root = 0)) :=
    List.countP_pos_iff.mpr ⟨k, hk, by simp [hz]⟩
  simp [Aff.ev_root g hm] at this; grind

/-- **Interval parity.** -/
theorem ip : ∀ (gs : List Aff) (s₀ s₁ : Rat), s₀ < s₁ →
    (∀ g ∈ gs, g.ev s₀ ≠ 0 ∧ g.ev s₁ ≠ 0) → Distinct gs s₀ s₁ →
    (pos gs s₀ ^^ pos gs s₁) = par (gs.map (isEv s₀ s₁ gs))
  | [], _, _, _, _, _ => by simp [pos]
  | g :: gs, s₀, s₁, h01, hends, hd => by
    have hg := hends g (List.mem_cons_self ..)
    have hends' : ∀ k ∈ gs, k.ev s₀ ≠ 0 ∧ k.ev s₁ ≠ 0 := fun k hk => hends k (List.mem_cons_of_mem _ hk)
    have hd' := distinct_tail hd
    simp only [List.map_cons, par_cons, isEv_cons_self]
    rw [List.map_congr_left (fun h hh => isEv_cons_other s₀ s₁ g h gs hh)]
    have hposc : ∀ s, pos (g :: gs) s = (decide (0 < g.ev s) && pos gs s) := fun s => by
      simp [pos]
    rw [hposc, hposc]
    rcases lt_trichotomy' 0 (g.ev s₀) with a0 | a0 | a0 <;>
      rcases lt_trichotomy' 0 (g.ev s₁) with a1 | a1 | a1
    · -- positive throughout: no root inside, and every event keeps its sign
      have hpos : ∀ s, s₀ ≤ s → s ≤ s₁ → 0 < g.ev s := fun s h0 h1 => g.pos_between a0 a1 h0 h1
      have hno : (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) = false := by
        by_cases hm : g.m = 0
        · simp [hm]
        · have := hpos g.root
          have := Aff.ev_root g hm
          simp; intro _ h0 h1; have := hpos g.root (by grind) (by grind); grind
      rw [hno]
      rw [List.map_congr_left (g := isEv s₀ s₁ gs) (fun h hh => by
        unfold isEv
        cases hev : (decide (h.m ≠ 0) && decide (s₀ < h.root) && decide (h.root < s₁) && nonpos gs h.root == 1)
        · simp
        · simp at hev; simp; exact hpos h.root (by grind) (by grind))]
      simp [a0, a1]
      exact ip gs s₀ s₁ h01 hends' hd'
    · exact absurd a1.symm hg.2
    · -- + to -: the root r, and everything to its left
      obtain ⟨hm, hr0, hr1, hsg⟩ := g.down h01 a0 a1
      have hm0 : g.m ≠ 0 := by grind
      have hnz := distinct_root hd hm0 hr0 hr1
      have ih := ip gs s₀ g.root hr0 (fun k hk => ⟨(hends' k hk).1, hnz k hk⟩)
        (distinct_sub hd' (Rat.le_refl) (Rat.le_of_lt hr1))
      rw [List.map_congr_left (g := isEv s₀ g.root gs) (fun h hh => by
        unfold isEv; rw [show decide (0 < g.ev h.root) = decide (h.root < g.root) from by simp [hsg]]
        cases decide (h.m ≠ 0) <;> cases hd1 : decide (s₀ < h.root) <;> simp at hd1 ⊢ <;> grind)]
      rw [← ih]
      simp [a0, show ¬ 0 < g.ev s₁ by grind, hm0, hr0, hr1]
      cases pos gs s₀ <;> cases pos gs g.root <;> rfl
    · exact absurd a0.symm hg.1
    · exact absurd a0.symm hg.1
    · exact absurd a0.symm hg.1
    · -- - to +: the root r, and everything to its right
      obtain ⟨hm, hr0, hr1, hsg⟩ := g.up h01 a0 a1
      have hm0 : g.m ≠ 0 := by grind
      have hnz := distinct_root hd hm0 hr0 hr1
      have ih := ip gs g.root s₁ hr1 (fun k hk => ⟨hnz k hk, (hends' k hk).2⟩)
        (distinct_sub hd' (Rat.le_of_lt hr0) (Rat.le_refl))
      rw [List.map_congr_left (g := isEv g.root s₁ gs) (fun h hh => by
        unfold isEv; rw [show decide (0 < g.ev h.root) = decide (g.root < h.root) from by simp [hsg]]
        cases decide (h.m ≠ 0) <;> cases hd1 : decide (h.root < s₁) <;> simp at hd1 ⊢ <;> grind)]
      rw [← ih]
      simp [a1, show ¬ 0 < g.ev s₀ by grind, hm0, hr0, hr1]
    · exact absurd a1.symm hg.2
    · -- negative throughout: nothing is positive, nothing is an event
      have hneg : ∀ s, s₀ ≤ s → s ≤ s₁ → g.ev s < 0 := fun s h0 h1 => g.neg_between a0 a1 h0 h1
      have hno : (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) = false := by
        by_cases hm : g.m = 0
        · simp [hm]
        · simp; intro _ h0 h1; have := hneg g.root (by grind) (by grind)
          have := Aff.ev_root g hm; grind
      rw [hno]
      rw [par_all_false]
      · simp [show ¬ 0 < g.ev s₀ by grind, show ¬ 0 < g.ev s₁ by grind]
      · intro b hb
        obtain ⟨h, hh, rfl⟩ := List.mem_map.mp hb
        unfold isEv
        cases hev : (decide (h.m ≠ 0) && decide (s₀ < h.root) && decide (h.root < s₁) && nonpos gs h.root == 1)
        · simp
        · simp at hev; simp; intro _; have := hneg h.root (by grind) (by grind); grind

/-! ## Double roots where some function is negative -/

/-- Wherever two of them vanish together inside, some function is negative. -/
def DistinctW (gs : List Aff) (s₀ s₁ : Rat) : Prop :=
  ∀ s, s₀ < s → s < s₁ → 2 ≤ gs.countP (fun g => decide (g.ev s = 0)) → ∃ k ∈ gs, k.ev s < 0

theorem distinctW_sub {gs : List Aff} {s₀ s₁ t₀ t₁ : Rat} (h : DistinctW gs s₀ s₁) (h0 : s₀ ≤ t₀)
    (h1 : t₁ ≤ s₁) : DistinctW gs t₀ t₁ := fun s a b => h s (by grind) (by grind)

theorem distinctW_tail {g : Aff} {gs : List Aff} {s₀ s₁ : Rat} (h : DistinctW (g :: gs) s₀ s₁)
    (hg : ∀ s, s₀ < s → s < s₁ → 0 ≤ g.ev s) : DistinctW gs s₀ s₁ := by
  intro s h0 h1 h2
  have := h s h0 h1 (by rw [List.countP_cons]; grind)
  obtain ⟨k, hk, hneg⟩ := this
  rcases List.mem_cons.mp hk with rfl | hk
  · have := hg s h0 h1; grind
  · exact ⟨k, hk, hneg⟩

/-- Just below `r` there is a point no root lies between. -/
theorem step_below (gs : List Aff) {s₀ r : Rat} (h : s₀ < r) :
    ∃ r', s₀ < r' ∧ r' < r ∧ ∀ g ∈ gs, g.m ≠ 0 → ¬ (r' ≤ g.root ∧ g.root < r) := by
  induction gs with
  | nil => exact ⟨(s₀ + r) / 2, by grind, by grind, fun _ h => nomatch h⟩
  | cons g gs ih =>
    obtain ⟨r', h0, h1, hr⟩ := ih
    by_cases hg : g.m ≠ 0 ∧ r' ≤ g.root ∧ g.root < r
    · refine ⟨(g.root + r) / 2, by grind, by grind, fun k hk hm hk' => ?_⟩
      rcases List.mem_cons.mp hk with rfl | hk
      · grind
      · exact hr k hk hm ⟨by grind, hk'.2⟩
    · refine ⟨r', h0, h1, fun k hk hm => ?_⟩
      rcases List.mem_cons.mp hk with rfl | hk
      · intro h'; exact hg ⟨hm, h'⟩
      · exact hr k hk hm

theorem step_above (gs : List Aff) {r s₁ : Rat} (h : r < s₁) :
    ∃ r', r < r' ∧ r' < s₁ ∧ ∀ g ∈ gs, g.m ≠ 0 → ¬ (r < g.root ∧ g.root ≤ r') := by
  induction gs with
  | nil => exact ⟨(r + s₁) / 2, by grind, by grind, fun _ h => nomatch h⟩
  | cons g gs ih =>
    obtain ⟨r', h0, h1, hr⟩ := ih
    by_cases hg : g.m ≠ 0 ∧ r < g.root ∧ g.root ≤ r'
    · refine ⟨(r + g.root) / 2, by grind, by grind, fun k hk hm hk' => ?_⟩
      rcases List.mem_cons.mp hk with rfl | hk
      · grind
      · exact hr k hk hm ⟨hk'.1, by grind⟩
    · refine ⟨r', h0, h1, fun k hk hm => ?_⟩
      rcases List.mem_cons.mp hk with rfl | hk
      · intro h'; exact hg ⟨hm, h'⟩
      · exact hr k hk hm

/-- An affine function nonzero at `t` with no root in `[t, r)`, negative at `r`, is negative at
`t`. -/
theorem neg_of_noroot (k : Aff) {t r : Rat} (htr : t < r) (hr : k.ev r < 0)
    (hno : k.m ≠ 0 → ¬ (t ≤ k.root ∧ k.root < r)) : k.ev t < 0 := by
  by_cases hm : k.m = 0
  · simp only [Aff.ev, hm, Rat.zero_mul] at hr ⊢; exact hr
  · have hno := hno hm
    rcases lt_trichotomy' 0 (k.ev t) with h | h | h
    · obtain ⟨-, h1, h2, -⟩ := k.down htr h hr; exact absurd ⟨Rat.le_of_lt h1, h2⟩ hno
    · have : k.root = t := by
        rw [Aff.ev_eq k hm] at h
        rcases Rat.mul_eq_zero.mp h.symm with h | h; exact absurd h hm; grind
      exact absurd ⟨by grind, by grind⟩ hno
    · exact h

theorem neg_of_noroot' (k : Aff) {r t : Rat} (htr : r < t) (hr : k.ev r < 0)
    (hno : k.m ≠ 0 → ¬ (r < k.root ∧ k.root ≤ t)) : k.ev t < 0 := by
  by_cases hm : k.m = 0
  · simp only [Aff.ev, hm, Rat.zero_mul] at hr ⊢; exact hr
  · have hno := hno hm
    rcases lt_trichotomy' 0 (k.ev t) with h | h | h
    · obtain ⟨-, h1, h2, -⟩ := k.up htr hr h; exact absurd ⟨h1, Rat.le_of_lt h2⟩ hno
    · have : k.root = t := by
        rw [Aff.ev_eq k hm] at h
        rcases Rat.mul_eq_zero.mp h.symm with h | h; exact absurd h hm; grind
      exact absurd ⟨by grind, by grind⟩ hno
    · exact h

theorem pos_false_of_neg {gs : List Aff} {s : Rat} {k : Aff} (hk : k ∈ gs) (h : k.ev s < 0) :
    pos gs s = false := by
  unfold pos; rw [List.all_eq_false]; exact ⟨k, hk, by simp; grind⟩

theorem ev_ne_of_noroot (k : Aff) (t s₀ : Rat) (h0 : k.ev s₀ ≠ 0) (hne : k.m ≠ 0 → k.root ≠ t) :
    k.ev t ≠ 0 := by
  by_cases hm : k.m = 0
  · simp only [Aff.ev, hm, Rat.zero_mul] at h0 ⊢; exact h0
  · rw [Aff.ev_eq k hm]; exact mul_ne_zero' hm (by have := hne hm; grind)

theorem isEv_shrink (gs : List Aff) (h : Aff) {s₀ r' r : Rat} (hlt : r' < r)
    (hno : h.m ≠ 0 → ¬ (r' ≤ h.root ∧ h.root < r)) : isEv s₀ r gs h = isEv s₀ r' gs h := by
  unfold isEv
  by_cases hm : h.m = 0
  · simp [hm]
  · have := hno hm
    by_cases hb : h.root < r'
    · simp [hm, hb, show h.root < r by grind]
    · simp [hm, hb, show ¬ h.root < r by grind]

theorem isEv_shrink' (gs : List Aff) (h : Aff) {r r' s₁ : Rat} (hlt : r < r')
    (hno : h.m ≠ 0 → ¬ (r < h.root ∧ h.root ≤ r')) : isEv r s₁ gs h = isEv r' s₁ gs h := by
  unfold isEv
  by_cases hm : h.m = 0
  · simp [hm]
  · have := hno hm
    by_cases hb : r' < h.root
    · simp [hm, hb, show r < h.root by grind]
    · simp [hm, hb, show ¬ r < h.root by grind]

/-- **Interval parity**, double roots allowed where some function is negative. -/
theorem ipW : ∀ (gs : List Aff) (s₀ s₁ : Rat), s₀ < s₁ →
    (∀ g ∈ gs, g.ev s₀ ≠ 0 ∧ g.ev s₁ ≠ 0) → DistinctW gs s₀ s₁ →
    (pos gs s₀ ^^ pos gs s₁) = par (gs.map (isEv s₀ s₁ gs))
  | [], _, _, _, _, _ => by simp [pos]
  | g :: gs, s₀, s₁, h01, hends, hd => by
    have hg := hends g (List.mem_cons_self ..)
    have hends' : ∀ k ∈ gs, k.ev s₀ ≠ 0 ∧ k.ev s₁ ≠ 0 := fun k hk => hends k (List.mem_cons_of_mem _ hk)
    simp only [List.map_cons, par_cons, isEv_cons_self]
    rw [List.map_congr_left (fun h hh => isEv_cons_other s₀ s₁ g h gs hh)]
    have hposc : ∀ s, pos (g :: gs) s = (decide (0 < g.ev s) && pos gs s) := fun s => by
      simp [pos]
    rw [hposc, hposc]
    rcases lt_trichotomy' 0 (g.ev s₀) with a0 | a0 | a0 <;>
      rcases lt_trichotomy' 0 (g.ev s₁) with a1 | a1 | a1
    · have hpos : ∀ s, s₀ ≤ s → s ≤ s₁ → 0 < g.ev s := fun s h0 h1 => g.pos_between a0 a1 h0 h1
      have hd' := distinctW_tail hd (fun s h0 h1 => Rat.le_of_lt (hpos s (by grind) (by grind)))
      have hno : (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) = false := by
        by_cases hm : g.m = 0
        · simp [hm]
        · have := Aff.ev_root g hm
          simp; intro _ h0 h1; have := hpos g.root (by grind) (by grind); grind
      rw [hno]
      rw [List.map_congr_left (g := isEv s₀ s₁ gs) (fun h hh => by
        unfold isEv
        cases hev : (decide (h.m ≠ 0) && decide (s₀ < h.root) && decide (h.root < s₁) && nonpos gs h.root == 1)
        · simp
        · simp at hev; simp; exact hpos h.root (by grind) (by grind))]
      simp [a0, a1]
      exact ipW gs s₀ s₁ h01 hends' hd'
    · exact absurd a1.symm hg.2
    · obtain ⟨hm, hr0, hr1, hsg⟩ := g.down h01 a0 a1
      have hm0 : g.m ≠ 0 := by grind
      have hd' : DistinctW gs s₀ g.root := distinctW_tail (distinctW_sub hd (Rat.le_refl) (Rat.le_of_lt hr1))
        (fun s _ h1 => Rat.le_of_lt ((hsg s).mpr h1))
      have hev_r : ∀ h ∈ gs, (isEv s₀ s₁ gs h && decide (0 < g.ev h.root)) = isEv s₀ g.root gs h := by
        intro h hh
        unfold isEv; rw [show decide (0 < g.ev h.root) = decide (h.root < g.root) from by simp [hsg]]
        cases decide (h.m ≠ 0) <;> cases hd1 : decide (s₀ < h.root) <;> simp at hd1 ⊢ <;> grind
      by_cases hz : ∃ k ∈ gs, k.ev g.root = 0
      · -- a double root at r: some function is negative there; step just below it
        obtain ⟨k, hk, hk0⟩ := hz
        obtain ⟨j, hj, hjneg⟩ := hd g.root hr0 hr1 (by
          rw [List.countP_cons]
          have : 1 ≤ gs.countP (fun g' => decide (g'.ev g.root = 0)) :=
            List.countP_pos_iff.mpr ⟨k, hk, by simp [hk0]⟩
          simp [Aff.ev_root g hm0]; grind)
        have hj' : j ∈ gs := by
          rcases List.mem_cons.mp hj with rfl | hj
          · rw [Aff.ev_root j hm0] at hjneg; grind
          · exact hj
        obtain ⟨r', h0', h1', hno⟩ := step_below gs hr0
        have hends'' : ∀ k ∈ gs, k.ev s₀ ≠ 0 ∧ k.ev r' ≠ 0 := fun k hk =>
          ⟨(hends' k hk).1, ev_ne_of_noroot k r' s₀ (hends' k hk).1
            (fun hm h => hno k hk hm ⟨by grind, by grind⟩)⟩
        have ih := ipW gs s₀ r' h0' hends'' (distinctW_sub hd' (Rat.le_refl) (Rat.le_of_lt h1'))
        have hpr' : pos gs r' = false := pos_false_of_neg hj'
          (neg_of_noroot j h1' hjneg (hno j hj'))
        rw [hpr', Bool.xor_false] at ih
        rw [List.map_congr_left (g := isEv s₀ r' gs) (fun h hh => by
          rw [hev_r h hh]; exact isEv_shrink gs h h1' (hno h hh))]
        rw [← ih]
        have hpg : pos gs g.root = false := pos_false_of_neg hj' hjneg
        simp [a0, show ¬ 0 < g.ev s₁ by grind, hpg]
      · have hnz : ∀ k ∈ gs, k.ev g.root ≠ 0 := fun k hk h => hz ⟨k, hk, h⟩
        have ih := ipW gs s₀ g.root hr0 (fun k hk => ⟨(hends' k hk).1, hnz k hk⟩) hd'
        rw [List.map_congr_left (g := isEv s₀ g.root gs) hev_r]
        rw [← ih]
        simp [a0, show ¬ 0 < g.ev s₁ by grind, hm0, hr0, hr1]
        cases pos gs s₀ <;> cases pos gs g.root <;> rfl
    · exact absurd a0.symm hg.1
    · exact absurd a0.symm hg.1
    · exact absurd a0.symm hg.1
    · obtain ⟨hm, hr0, hr1, hsg⟩ := g.up h01 a0 a1
      have hm0 : g.m ≠ 0 := by grind
      have hd' : DistinctW gs g.root s₁ := distinctW_tail (distinctW_sub hd (Rat.le_of_lt hr0) (Rat.le_refl))
        (fun s h0 _ => Rat.le_of_lt ((hsg s).mpr h0))
      have hev_r : ∀ h ∈ gs, (isEv s₀ s₁ gs h && decide (0 < g.ev h.root)) = isEv g.root s₁ gs h := by
        intro h hh
        unfold isEv; rw [show decide (0 < g.ev h.root) = decide (g.root < h.root) from by simp [hsg]]
        cases decide (h.m ≠ 0) <;> cases hd1 : decide (h.root < s₁) <;> simp at hd1 ⊢ <;> grind
      by_cases hz : ∃ k ∈ gs, k.ev g.root = 0
      · obtain ⟨k, hk, hk0⟩ := hz
        obtain ⟨j, hj, hjneg⟩ := hd g.root hr0 hr1 (by
          rw [List.countP_cons]
          have : 1 ≤ gs.countP (fun g' => decide (g'.ev g.root = 0)) :=
            List.countP_pos_iff.mpr ⟨k, hk, by simp [hk0]⟩
          simp [Aff.ev_root g hm0]; grind)
        have hj' : j ∈ gs := by
          rcases List.mem_cons.mp hj with rfl | hj
          · rw [Aff.ev_root j hm0] at hjneg; grind
          · exact hj
        obtain ⟨r', h0', h1', hno⟩ := step_above gs hr1
        have hends'' : ∀ k ∈ gs, k.ev r' ≠ 0 ∧ k.ev s₁ ≠ 0 := fun k hk =>
          ⟨ev_ne_of_noroot k r' s₀ (hends' k hk).1 (fun hm h => hno k hk hm ⟨by grind, by grind⟩),
            (hends' k hk).2⟩
        have ih := ipW gs r' s₁ h1' hends'' (distinctW_sub hd' (Rat.le_of_lt h0') (Rat.le_refl))
        have hpr' : pos gs r' = false := pos_false_of_neg hj'
          (neg_of_noroot' j h0' hjneg (hno j hj'))
        rw [hpr', Bool.false_xor] at ih
        rw [List.map_congr_left (g := isEv r' s₁ gs) (fun h hh => by
          rw [hev_r h hh]; exact isEv_shrink' gs h h0' (hno h hh))]
        rw [← ih]
        have hpg : pos gs g.root = false := pos_false_of_neg hj' hjneg
        simp [a1, show ¬ 0 < g.ev s₀ by grind, hpg]
      · have hnz : ∀ k ∈ gs, k.ev g.root ≠ 0 := fun k hk h => hz ⟨k, hk, h⟩
        have ih := ipW gs g.root s₁ hr1 (fun k hk => ⟨hnz k hk, (hends' k hk).2⟩) hd'
        rw [List.map_congr_left (g := isEv g.root s₁ gs) hev_r]
        rw [← ih]
        simp [a1, show ¬ 0 < g.ev s₀ by grind, hm0, hr0, hr1]
    · exact absurd a1.symm hg.2
    · have hneg : ∀ s, s₀ ≤ s → s ≤ s₁ → g.ev s < 0 := fun s h0 h1 => g.neg_between a0 a1 h0 h1
      have hno : (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) = false := by
        by_cases hm : g.m = 0
        · simp [hm]
        · simp; intro _ h0 h1; have := hneg g.root (by grind) (by grind)
          have := Aff.ev_root g hm; grind
      rw [hno]
      rw [par_all_false]
      · simp [show ¬ 0 < g.ev s₀ by grind, show ¬ 0 < g.ev s₁ by grind]
      · intro b hb
        obtain ⟨h, hh, rfl⟩ := List.mem_map.mp hb
        unfold isEv
        cases hev : (decide (h.m ≠ 0) && decide (s₀ < h.root) && decide (h.root < s₁) && nonpos gs h.root == 1)
        · simp
        · simp at hev; simp; intro _; have := hneg h.root (by grind) (by grind); grind

end ForgeCertify
