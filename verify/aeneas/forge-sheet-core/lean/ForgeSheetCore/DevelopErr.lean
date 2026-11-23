import Mathlib.Analysis.Complex.Basic
import Mathlib.Tactic

/-!
# The development along a path: how rounding accumulates (the model)

A rigid motion of the plane is `p ↦ o + x p` over ℂ (`x` unit for an exact motion); composing is
`(A ∘ B) = (A.o + A.x B.o, A.x B.x)` (`thenE`, place.rs `then`). build.rs develops each wall by
composing the joints' motions along its path from the root. If each computed composition is within
`eo` (origin) and `ex` (axis) of the exact composition of its computed inputs
(`then_spec`, `RefinePlace.lean`: `5 u S + 5 η` and `3 u (…) + 4 η` per coordinate), and every
joint motion has `|J.o| ≤ R` and `|J.x| ≤ 1`, then after `d` joints (`develop_err`):

* the axis is within `d · ex` of the exact one, and
* the origin within `d · eo + d² · ex · R`

— linear in depth for the axis, quadratic for the origin (an axis error is a rotation, moved
by the lever of each joint's origin), never exponential.
-/

namespace ForgeSheetCore.Develop

/-- A motion: origin and axis. -/
abbrev M := ℂ × ℂ

def thenE (A B : M) : M := (A.1 + A.2 * B.1, A.2 * B.2)

/-- Composing exactly along a list of joints, from `A`. -/
def chainE (A : M) : List M → M
  | [] => A
  | J :: Js => chainE (thenE A J) Js

/-- A computed chain: from `C`, each step within `eo` (origin) and `ex` (axis) of composing the
computed motion so far exactly with the joint, ending at `F`. -/
inductive Steps (eo ex : ℝ) : M → List M → M → Prop
  | nil (C : M) : Steps eo ex C [] C
  | cons (C J C' F : M) (Js : List M) :
      ‖C'.1 - (thenE C J).1‖ ≤ eo → ‖C'.2 - (thenE C J).2‖ ≤ ex → Steps eo ex C' Js F →
      Steps eo ex C (J :: Js) F

/-- **Accumulated error of a computed chain**, for joints with `|J.o| ≤ R`, `|J.x| ≤ 1`, from a
start within `ao`, `ax` of the exact one: after `n` joints the axis is within `ax + n ex` and the
origin within `ao + n eo + R (n ax + n² ex)` of the exact chain. -/
theorem develop_err {R eo ex : ℝ} (hR : 0 ≤ R) (heo : 0 ≤ eo) (hex : 0 ≤ ex) :
    ∀ (Js : List M), (∀ J ∈ Js, ‖J.1‖ ≤ R ∧ ‖J.2‖ ≤ 1) →
    ∀ (A C F : M) (ao ax : ℝ), Steps eo ex C Js F → ‖C.1 - A.1‖ ≤ ao → ‖C.2 - A.2‖ ≤ ax →
      ‖F.2 - (chainE A Js).2‖ ≤ ax + Js.length * ex ∧
      ‖F.1 - (chainE A Js).1‖ ≤ ao + Js.length * eo + R * (Js.length * ax + Js.length ^ 2 * ex)
  | [], _, A, C, F, ao, ax, hs, h1, h2 => by
      cases hs; simp [chainE]; exact ⟨h2, h1⟩
  | J :: Js, hJ, A, C, F, ao, ax, hs, h1, h2 => by
      cases hs with
      | cons _ _ C' _ _ ho hx hrest =>
        obtain ⟨hJo, hJx⟩ := hJ J (by simp)
        have hax0 : 0 ≤ ax := le_trans (norm_nonneg _) h2
        -- one step: the new start's distance from the exact composition
        have sx : ‖C'.2 - (thenE A J).2‖ ≤ ax + ex := by
          have : C'.2 - (thenE A J).2 = (C'.2 - (thenE C J).2) + (C.2 - A.2) * J.2 := by
            simp only [thenE]; ring
          rw [this]
          calc _ ≤ ‖C'.2 - (thenE C J).2‖ + ‖(C.2 - A.2) * J.2‖ := norm_add_le _ _
            _ ≤ ex + ax * 1 := by
                rw [norm_mul]; exact add_le_add hx (mul_le_mul h2 hJx (norm_nonneg _) hax0)
            _ = ax + ex := by ring
        have so : ‖C'.1 - (thenE A J).1‖ ≤ ao + eo + R * ax := by
          have : C'.1 - (thenE A J).1 = (C'.1 - (thenE C J).1) + (C.1 - A.1) + (C.2 - A.2) * J.1 := by
            simp only [thenE]; ring
          rw [this]
          calc _ ≤ ‖(C'.1 - (thenE C J).1) + (C.1 - A.1)‖ + ‖(C.2 - A.2) * J.1‖ := norm_add_le _ _
            _ ≤ eo + ao + ax * R := by
                rw [norm_mul]
                exact add_le_add (le_trans (norm_add_le _ _) (add_le_add ho h1))
                  (mul_le_mul h2 hJo (norm_nonneg _) hax0)
            _ = ao + eo + R * ax := by ring
        obtain ⟨rx, ro⟩ := develop_err hR heo hex Js (fun J' h => hJ J' (by simp [h]))
          (thenE A J) C' F _ _ hrest so sx
        simp only [chainE, List.length_cons, Nat.cast_add, Nat.cast_one]
        constructor
        · linarith
        · have hn : (0 : ℝ) ≤ Js.length := Nat.cast_nonneg _
          have : R * ((Js.length : ℝ) * (ax + ex) + (Js.length : ℝ) ^ 2 * ex) ≤
              R * (((Js.length : ℝ) + 1) * ax + ((Js.length : ℝ) + 1) ^ 2 * ex) - R * ax := by
            have : (Js.length : ℝ) * (ax + ex) + (Js.length : ℝ) ^ 2 * ex + ax ≤
                ((Js.length : ℝ) + 1) * ax + ((Js.length : ℝ) + 1) ^ 2 * ex := by nlinarith
            nlinarith
          linarith

end ForgeSheetCore.Develop
