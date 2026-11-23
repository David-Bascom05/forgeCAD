import ForgeExact.Fillet.Basic

/-!
# The chamfer corner rule: point or patch, decided once

research/ops-spec/design.md §5, and the master's rule: at a vertex where three chamfered edges
meet, the corner is a **point** iff the three chamfer planes meet in one point inside the
material, else a **patch**. Decided up front by an exact predicate, never tried and fallen back
from (the kernel's chamfer.rs:669–696 tries the point and falls back).

In the corner's own coordinates `u = (u₁, u₂, u₃)`, `uₖ` the depth below face `k` (an affine
change of coordinates: three independent planes; the material is `u ≥ 0`), the chamfer of the
edge between faces `i` and `j` has its contact on face `i` at `uⱼ = dᵢⱼ` and on face `j` at
`uᵢ = dⱼᵢ` (all `d > 0`), so its plane is `dᵢⱼ uᵢ + dⱼᵢ uⱼ = dᵢⱼ dⱼᵢ`, kept where `≥`.

* `meet_on_planes`, `meet_unique`: the three planes meet in exactly one point `m`
  (`D m₁ = d₂₁ d₃₁ (d₁₂ d₂₃ + d₁₃ d₃₂ − d₂₃ d₃₂)` and cyclically; `D = d₁₂ d₂₃ d₃₁ + d₂₁ d₃₂ d₁₃`).
* The corner contacts: `q₃ = (d₃₁, d₃₂, 0)` is where the two chamfers at face 3 meet on it
  (`q₁`, `q₂` cyclically): `contact_on_planes`.
* **The rule**, `point_iff_kept`: `m` is inside the material (`mₖ > 0` for every `k`) iff each
  corner contact is strictly kept by the opposite chamfer. One exact predicate, decided on the
  six distances.
* Rows: `point_row`, the three chamfer faces meet at one point inside the material, a corner of
  degree three; `patch_row`, no point inside the material lies on all three chamfer planes, so
  the point corner does not exist there and the patch through `q₁ q₂ q₃` is the corner.
-/

namespace ForgeExact.Ops

open Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace ChamferCorner

/-- The six contact distances, all positive. -/
structure Corner (K : Type) [LT K] [OfNat K 0] where
  d₁₂ : K
  d₂₁ : K
  d₂₃ : K
  d₃₂ : K
  d₃₁ : K
  d₁₃ : K
  pos₁₂ : 0 < d₁₂
  pos₂₁ : 0 < d₂₁
  pos₂₃ : 0 < d₂₃
  pos₃₂ : 0 < d₃₂
  pos₃₁ : 0 < d₃₁
  pos₁₃ : 0 < d₁₃

namespace Corner

variable (C : Corner K)

/-- The chamfer planes, as the kept side's margin. -/
def c₁₂ (u₁ u₂ : K) : K := C.d₁₂ * u₁ + C.d₂₁ * u₂ - C.d₁₂ * C.d₂₁
def c₂₃ (u₂ u₃ : K) : K := C.d₂₃ * u₂ + C.d₃₂ * u₃ - C.d₂₃ * C.d₃₂
def c₃₁ (u₃ u₁ : K) : K := C.d₃₁ * u₃ + C.d₁₃ * u₁ - C.d₃₁ * C.d₁₃

/-- The determinant. -/
def D : K := C.d₁₂ * C.d₂₃ * C.d₃₁ + C.d₂₁ * C.d₃₂ * C.d₁₃

theorem D_pos : 0 < C.D := by
  have := Fillet.mul_pos' (Fillet.mul_pos' C.pos₁₂ C.pos₂₃) C.pos₃₁
  have := Fillet.mul_pos' (Fillet.mul_pos' C.pos₂₁ C.pos₃₂) C.pos₁₃
  simp only [D]; grind

/-- The brackets that fix the signs of the meet point's coordinates. -/
def β₁ : K := C.d₁₂ * C.d₂₃ + C.d₁₃ * C.d₃₂ - C.d₂₃ * C.d₃₂
def β₂ : K := C.d₂₃ * C.d₃₁ + C.d₂₁ * C.d₁₃ - C.d₃₁ * C.d₁₃
def β₃ : K := C.d₃₁ * C.d₁₂ + C.d₃₂ * C.d₂₁ - C.d₁₂ * C.d₂₁

/-- The meet point, scaled by `D`. -/
def M₁ : K := C.d₂₁ * C.d₃₁ * C.β₁
def M₂ : K := C.d₃₂ * C.d₁₂ * C.β₂
def M₃ : K := C.d₁₃ * C.d₂₃ * C.β₃

/-- `m = M / D` lies on the three chamfer planes (stated scaled by `D`). -/
theorem meet_on_planes :
    C.d₁₂ * C.M₁ + C.d₂₁ * C.M₂ = C.D * (C.d₁₂ * C.d₂₁) ∧
    C.d₂₃ * C.M₂ + C.d₃₂ * C.M₃ = C.D * (C.d₂₃ * C.d₃₂) ∧
    C.d₃₁ * C.M₃ + C.d₁₃ * C.M₁ = C.D * (C.d₃₁ * C.d₁₃) := by
  simp only [M₁, M₂, M₃, β₁, β₂, β₃, D]
  refine ⟨?_, ?_, ?_⟩ <;> grind

/-- Only one point lies on all three planes. -/
theorem meet_unique {u₁ u₂ u₃ : K} (h₁₂ : C.c₁₂ u₁ u₂ = 0) (h₂₃ : C.c₂₃ u₂ u₃ = 0)
    (h₃₁ : C.c₃₁ u₃ u₁ = 0) : C.D * u₁ = C.M₁ ∧ C.D * u₂ = C.M₂ ∧ C.D * u₃ = C.M₃ := by
  simp only [c₁₂, c₂₃, c₃₁] at *
  -- the differences from the meet point solve the homogeneous system, whose determinant is D
  obtain ⟨e₁, e₂, e₃⟩ := meet_on_planes C
  have hD := D_pos C
  have w₁ : C.D * (C.D * u₁ - C.M₁) = 0 := by
    simp only [M₁, M₂, M₃, β₁, β₂, β₃, D] at *; grind
  have w₂ : C.D * (C.D * u₂ - C.M₂) = 0 := by
    simp only [M₁, M₂, M₃, β₁, β₂, β₃, D] at *; grind
  have w₃ : C.D * (C.D * u₃ - C.M₃) = 0 := by
    simp only [M₁, M₂, M₃, β₁, β₂, β₃, D] at *; grind
  have z : ∀ a : K, C.D * a = 0 → a = 0 := by
    intro a h
    rcases Fillet.le_or_lt' a 0 with ha | ha
    · rcases Fillet.le_or_lt' 0 a with hb | hb
      · grind
      · have := Fillet.mul_pos' hD (show 0 < -a by grind); grind
    · have := Fillet.mul_pos' hD ha; grind
  exact ⟨by have := z _ w₁; grind, by have := z _ w₂; grind, by have := z _ w₃; grind⟩

/-- The corner contacts lie on face `k` and on its two chamfers. -/
theorem contact_on_planes :
    C.c₂₃ C.d₃₂ 0 = 0 ∧ C.c₃₁ 0 C.d₃₁ = 0 ∧      -- q₃ = (d₃₁, d₃₂, 0)
    C.c₁₂ 0 C.d₁₂ = 0 ∧ C.c₃₁ C.d₁₃ 0 = 0 ∧      -- q₁ = (0, d₁₂, d₁₃)
    C.c₁₂ C.d₂₁ 0 = 0 ∧ C.c₂₃ 0 C.d₂₃ = 0 := by  -- q₂ = (d₂₁, 0, d₂₃)
  simp only [c₁₂, c₂₃, c₃₁]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> grind

/-- The opposite chamfer at each contact is the bracket of the meet point's coordinate. -/
theorem kept_eq_bracket :
    C.c₁₂ C.d₃₁ C.d₃₂ = C.β₃ ∧ C.c₂₃ C.d₁₂ C.d₁₃ = C.β₁ ∧ C.c₃₁ C.d₂₃ C.d₂₁ = C.β₂ := by
  simp only [c₁₂, c₂₃, c₃₁, β₁, β₂, β₃]; refine ⟨?_, ?_, ?_⟩ <;> grind

theorem pos_iff_mul {a b : K} (ha : 0 < a) : 0 < a * b ↔ 0 < b := by
  constructor
  · intro h
    rcases Fillet.le_or_lt' b 0 with hb | hb
    · have := Fillet.mul_nonneg' (le_of_lt ha) (show 0 ≤ -b by grind); grind
    · exact hb
  · intro h; exact Fillet.mul_pos' ha h
where le_of_lt {a b : K} (h : a < b) : a ≤ b := by grind

/-- **The rule.** The meet point is inside the material iff each corner contact is strictly kept
by the opposite chamfer. -/
theorem point_iff_kept :
    (0 < C.M₁ ∧ 0 < C.M₂ ∧ 0 < C.M₃) ↔
      (0 < C.c₂₃ C.d₁₂ C.d₁₃ ∧ 0 < C.c₃₁ C.d₂₃ C.d₂₁ ∧ 0 < C.c₁₂ C.d₃₁ C.d₃₂) := by
  obtain ⟨k₃, k₁, k₂⟩ := kept_eq_bracket C
  rw [k₁, k₂, k₃]
  simp only [M₁, M₂, M₃]
  rw [pos_iff_mul (Fillet.mul_pos' C.pos₂₁ C.pos₃₁), pos_iff_mul (Fillet.mul_pos' C.pos₃₂ C.pos₁₂),
    pos_iff_mul (Fillet.mul_pos' C.pos₁₃ C.pos₂₃)]

/-- The point is in the material iff its scaled coordinates are positive. -/
theorem scaled_pos {u M : K} (h : C.D * u = M) : 0 < u ↔ 0 < M := by
  rw [← h]; exact (pos_iff_mul (D_pos C)).symm

/-- **Point row.** Under the rule, the three chamfer planes meet in one point strictly inside the
material, and in no other point: the corner is a vertex of degree three. -/
theorem point_row (hp : 0 < C.c₂₃ C.d₁₂ C.d₁₃ ∧ 0 < C.c₃₁ C.d₂₃ C.d₂₁ ∧ 0 < C.c₁₂ C.d₃₁ C.d₃₂) :
    ∀ u₁ u₂ u₃ : K, C.c₁₂ u₁ u₂ = 0 → C.c₂₃ u₂ u₃ = 0 → C.c₃₁ u₃ u₁ = 0 →
      0 < u₁ ∧ 0 < u₂ ∧ 0 < u₃ := by
  intro u₁ u₂ u₃ h₁ h₂ h₃
  obtain ⟨m₁, m₂, m₃⟩ := (point_iff_kept C).mpr hp
  obtain ⟨e₁, e₂, e₃⟩ := meet_unique C h₁ h₂ h₃
  exact ⟨(scaled_pos C e₁).mpr m₁, (scaled_pos C e₂).mpr m₂, (scaled_pos C e₃).mpr m₃⟩

/-- **Patch row.** Otherwise no point strictly inside the material lies on all three chamfer
planes: the point corner does not exist, and the corner is the patch through the contacts. -/
theorem patch_row
    (hp : ¬(0 < C.c₂₃ C.d₁₂ C.d₁₃ ∧ 0 < C.c₃₁ C.d₂₃ C.d₂₁ ∧ 0 < C.c₁₂ C.d₃₁ C.d₃₂)) :
    ∀ u₁ u₂ u₃ : K, C.c₁₂ u₁ u₂ = 0 → C.c₂₃ u₂ u₃ = 0 → C.c₃₁ u₃ u₁ = 0 →
      ¬(0 < u₁ ∧ 0 < u₂ ∧ 0 < u₃) := by
  intro u₁ u₂ u₃ h₁ h₂ h₃ ⟨p₁, p₂, p₃⟩
  obtain ⟨e₁, e₂, e₃⟩ := meet_unique C h₁ h₂ h₃
  exact hp ((point_iff_kept C).mp
    ⟨(scaled_pos C e₁).mp p₁, (scaled_pos C e₂).mp p₂, (scaled_pos C e₃).mp p₃⟩)

/-- Equal distances always give the point (`mₖ = d/2`). -/
theorem equal_is_point {d : K} (hd : 0 < d) (h : C.d₁₂ = d ∧ C.d₂₁ = d ∧ C.d₂₃ = d ∧
    C.d₃₂ = d ∧ C.d₃₁ = d ∧ C.d₁₃ = d) :
    0 < C.c₂₃ C.d₁₂ C.d₁₃ ∧ 0 < C.c₃₁ C.d₂₃ C.d₂₁ ∧ 0 < C.c₁₂ C.d₃₁ C.d₃₂ := by
  obtain ⟨a, b, c, e, f, g⟩ := h
  simp only [c₁₂, c₂₃, c₃₁, a, b, c, e, f, g]
  have := Fillet.mul_pos' hd hd
  refine ⟨?_, ?_, ?_⟩ <;> grind

end Corner

end ChamferCorner

end ForgeExact.Ops
