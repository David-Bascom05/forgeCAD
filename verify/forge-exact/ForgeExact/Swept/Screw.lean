import ForgeExact.Topology

/-!
# Coil and thread: when a helical sweep meets itself (research/sweep-spec/DESIGN.md §3.2)

A coil carries its section point `(r, h)` at turn parameter `u ∈ [0, U]` to the point with
cylindrical coordinates `(r + a·u, h + b·u, u mod T)` (`T` the whole turn, `b = ±pitch/T`,
`a = pitch/T · tan taper`). Off the axis (`r + a·u > 0`) cylindrical coordinates name a point once,
with the angle taken mod `T`; so the coil is that map in the **screw chart**, and it meets itself
exactly where two `(q, u)` give the same chart point.

* `same_point_iff`: `(q, u)` and `(q', u')` land together exactly when `u' = u + k·T` for an
  integer `k` and `q = q' + k·(a T, b T)`.
* `coil_injective`: so the coil is one-to-one on `R × [0, U]` exactly when, for every whole number
  of turns `1 ≤ k` with `k·T ≤ U`, the section `R` and its translate `R + k·(a T, b T)` share no
  point. That is an exact test on the profile against its translates (orient2d for lines, a
  quadratic for arcs), in place of the kernel's `TurnsTouch` (pitch ≤ the section's height +
  LINEAR), which is sufficient but refuses sections that slant or bend and never meet.
-/

namespace ForgeExact.Swept.Screw

open Lean.Grind

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- A coil's motion in the screw chart. -/
structure Motion (K : Type) where
  a : K
  b : K
  /-- one whole turn -/
  T : K

/-- `k` times `x`, as a sum (no cast from ℕ is assumed of the field). -/
def nsm : Nat → K → K
  | 0, _ => 0
  | k + 1, x => nsm k x + x

theorem nsm_mul (k : Nat) (c x : K) : c * nsm k x = nsm k (c * x) := by
  induction k with
  | zero => simp only [nsm]; grind
  | succ n ih => simp only [nsm]; rw [← ih]; grind

theorem nsm_pos {k : Nat} (hk : 1 ≤ k) {x : K} (hx : 0 < x) : 0 < nsm k x := by
  induction k with
  | zero => omega
  | succ n ih =>
    simp only [nsm]
    rcases Nat.eq_zero_or_pos n with h | h
    · subst h; simp only [nsm]; grind
    · have := ih h; grind

/-- The chart point of section point `q` at `u`: radius and height (the angle is `u` mod `T`). -/
def chart (m : Motion K) (q : K × K) (u : K) : K × K := (q.1 + m.a * u, q.2 + m.b * u)

/-- Two chart points are one point of space: the same radius and height, the angles a whole number
of turns apart. -/
def Same (m : Motion K) (q : K × K) (u : K) (q' : K × K) (u' : K) : Prop :=
  chart m q u = chart m q' u' ∧ ∃ k : Nat, u' = u + nsm k m.T ∨ u = u' + nsm k m.T

/-- `q` moved by `k` whole turns of the section's drift `(a T, b T)`. -/
def shift (m : Motion K) (k : Nat) (q : K × K) : K × K :=
  (q.1 + nsm k (m.a * m.T), q.2 + nsm k (m.b * m.T))

theorem same_point_iff (m : Motion K) (q q' : K × K) (u u' : K) :
    Same m q u q' u' ↔ ∃ k : Nat, (u' = u + nsm k m.T ∧ q = shift m k q') ∨
      (u = u' + nsm k m.T ∧ q' = shift m k q) := by
  obtain ⟨x, y⟩ := q; obtain ⟨x', y'⟩ := q'
  simp only [Same, chart, shift, Prod.mk.injEq]
  have ea := fun k => nsm_mul k m.a m.T
  have eb := fun k => nsm_mul k m.b m.T
  constructor
  · rintro ⟨⟨h1, h2⟩, k, hk | hk⟩
    · refine ⟨k, Or.inl ⟨hk, ?_, ?_⟩⟩ <;> (subst hk; simp only [← ea, ← eb]; grind)
    · refine ⟨k, Or.inr ⟨hk, ?_, ?_⟩⟩ <;> (subst hk; simp only [← ea, ← eb]; grind)
  · rintro ⟨k, ⟨hk, h1, h2⟩ | ⟨hk, h1, h2⟩⟩
    · rw [← ea k] at h1; rw [← eb k] at h2
      exact ⟨⟨by subst hk; grind, by subst hk; grind⟩, k, Or.inl hk⟩
    · rw [← ea k] at h1; rw [← eb k] at h2
      exact ⟨⟨by subst hk; grind, by subst hk; grind⟩, k, Or.inr hk⟩

/-- **When a coil meets itself.** On `R × [0, U]` with `T > 0`, the coil is one-to-one (no two
distinct `(q, u)` land on one point) exactly when no section point is another's translate by a
whole number `k ≥ 1` of turns that fits (`k·T ≤ U`): `R ∩ (R + k·(a T, b T)) = ∅`. -/
theorem coil_injective (m : Motion K) (hT : 0 < m.T) (R : K × K → Prop) (U : K) :
    (∀ q u q' u', R q → R q' → 0 ≤ u → u ≤ U → 0 ≤ u' → u' ≤ U →
        Same m q u q' u' → q = q' ∧ u = u') ↔
      (∀ k : Nat, 1 ≤ k → nsm k m.T ≤ U → ∀ q, R q → ¬R (shift m k q)) := by
  constructor
  · intro hinj k hk hkU q hq hs
    -- (q + k drift, 0) and (q, k T) are one point
    have hsame : Same m (shift m k q) 0 q (nsm k m.T) :=
      (same_point_iff m _ _ _ _).mpr ⟨k, Or.inl ⟨by grind, rfl⟩⟩
    have hkT := nsm_pos hk hT
    have := (hinj _ 0 q _ hs hq (Std.le_refl _) (by grind) (Std.le_of_lt hkT) hkU hsame).2
    grind
  · intro hno q u q' u' hq hq' hu0 huU hu'0 hu'U hs
    obtain ⟨k, ⟨hk, hqe⟩ | ⟨hk, hqe⟩⟩ := (same_point_iff m q q' u u').mp hs
    · rcases Nat.eq_zero_or_pos k with h0 | hpos
      · subst h0
        simp only [shift, nsm] at hqe hk
        obtain ⟨x', y'⟩ := q'
        refine ⟨?_, by grind⟩
        rw [hqe]; simp only [Prod.mk.injEq]; constructor <;> grind
      · exact absurd (hqe ▸ hq) (hno k hpos (by grind) q' hq')
    · rcases Nat.eq_zero_or_pos k with h0 | hpos
      · subst h0
        simp only [shift, nsm] at hqe hk
        obtain ⟨x, y⟩ := q
        refine ⟨?_, by grind⟩
        rw [hqe]; simp only [Prod.mk.injEq]; constructor <;> grind
      · exact absurd (hqe ▸ hq') (hno k hpos (by grind) q hq)

end ForgeExact.Swept.Screw
