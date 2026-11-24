import Mathlib.Order.Interval.Set.Basic
import Mathlib.Algebra.Order.Archimedean.Real.Basic

/-!
# The certified event search for a torus on a cylinder (curved-sites gap #7)

forge-geom's `intersect` follows the curve F(u, s) = 0 of a torus on a cylinder (`Form::Poly4`)
between *events*: values of u where F(u, ·) has a multiple root, the only places its real root
count can change. Today the events are found by sampling a scaled discriminant at 64 places, which
missed a thin loop on fuzz seed 295. `meet_cert.rs` certifies the events by interval subdivision:
each box U × S of a cover of the (u, s) region either encloses F away from 0, or encloses ∂F/∂s
away from 0, or is an *event box* of small u-width.

`no_event_of_cover`: off the event boxes there is no event. Every (u, s) with F = 0 lies in
some box; that box cannot be one where F excludes 0, so it is one where ∂F/∂s excludes 0, or an
event box. The enclosures are hypotheses (`Encloses`), discharged by forge_math::Interval's
outward rounding (an IEEE lemma, open: `IntervalSound`).

What follows from it, stated and open: between two consecutive event clusters the number of real
roots of F(u, ·) is constant (`CountConstant`: simple roots move continuously, by the implicit
function theorem, and no root enters from |s| > S since the region bounds them), so every loop of
the curve is found by tracking the roots from a sample in each interval.
-/

namespace ForgeCurved.EventCover

/-- A box of the cover, with what it certifies. -/
structure Box where
  u : Set ℝ
  s : Set ℝ
  /-- 0: F excludes 0 on it; 1: ∂F/∂s excludes 0 on it; 2: an event box. -/
  kind : Fin 3

/-- The enclosures the code computed are sound for the box's claim. -/
def Encloses (F Fs : ℝ → ℝ → ℝ) (b : Box) : Prop :=
  (b.kind = 0 → ∀ u ∈ b.u, ∀ s ∈ b.s, F u s ≠ 0) ∧
  (b.kind = 1 → ∀ u ∈ b.u, ∀ s ∈ b.s, Fs u s ≠ 0)

/-- **No event off the event boxes.** If the boxes cover the region and each is sound, then at
any zero (u, s) of F in the region that lies in no event box, ∂F/∂s ≠ 0: a simple root. -/
theorem no_event_of_cover (F Fs : ℝ → ℝ → ℝ) (boxes : List Box) (region : Set (ℝ × ℝ))
    (hcover : ∀ p ∈ region, ∃ b ∈ boxes, p.1 ∈ b.u ∧ p.2 ∈ b.s)
    (hsound : ∀ b ∈ boxes, Encloses F Fs b)
    (u s : ℝ) (hp : (u, s) ∈ region) (hF : F u s = 0)
    (hoff : ∀ b ∈ boxes, b.kind = 2 → u ∉ b.u) : Fs u s ≠ 0 := by
  obtain ⟨b, hb, hu, hs⟩ := hcover (u, s) hp
  obtain ⟨h0, h1⟩ := hsound b hb
  match hk : b.kind with
  | 0 => exact absurd hF (h0 hk u hu s hs)
  | 1 => exact h1 hk u hu s hs
  | 2 => exact absurd hu (hoff b hb hk)

/-- Stated, open: the interval enclosures of `meet_cert::enclose` are sound (IEEE, outward
rounding of forge_math::Interval, and `sin`/`cos` within their stated ulps). -/
def IntervalSound (F : ℝ → ℝ → ℝ) (enclose : Set ℝ → Set ℝ → Set ℝ) : Prop :=
  ∀ U S : Set ℝ, ∀ u ∈ U, ∀ s ∈ S, F u s ∈ enclose U S

/-- Stated, open: with no multiple root over an interval of u and all roots inside |s| < S, the
real root count of F(u, ·) is constant there. -/
def CountConstant (F Fs : ℝ → ℝ → ℝ) (I : Set ℝ) (S : ℝ) (count : ℝ → ℕ) : Prop :=
  (∀ u ∈ I, ∀ s, F u s = 0 → Fs u s ≠ 0) → (∀ u ∈ I, ∀ s, F u s = 0 → |s| < S) →
    ∀ u ∈ I, ∀ v ∈ I, count u = count v

end ForgeCurved.EventCover
