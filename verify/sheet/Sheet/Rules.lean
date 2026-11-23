import Sheet.Fold

/-!
# The bend rules the kernel branches on, and the vendor formulas

* `BA_from_rule`: `BendRule::K` and `BendRule::KTable` (shop.rs:48-89), `BA = θ (R + K t)`.
* `BA_from_table`: `BendRule::Allowance`, rows `(θᵢ, BAᵢ)` at one radius, linear in `θ`.
* The table interpolation of shop.rs:119-136: the first window `x₀ ≤ x ≤ x₁` of strictly
  increasing rows.
* The vendor side (forge-vendor lib.rs:1025-1050, forge-sheet checks.rs:56-100): outside and
  inside setbacks, the bend deduction `BD = 2 OSSB − BA`, `BD90`, and the minimum flange measured
  to the outside.
-/

namespace Sheet

open Real

/-! ## K and the allowance -/

/-- The allowance of a K rule. -/
noncomputable def baOfK (R θ t K : ℝ) : ℝ := θ * (R + K * t)

/-- The effective K of an allowance. -/
noncomputable def kOfBA (R θ t BA : ℝ) : ℝ := (BA / θ - R) / t

theorem kOfBA_baOfK {R θ t K : ℝ} (hθ : θ ≠ 0) (ht : t ≠ 0) : kOfBA R θ t (baOfK R θ t K) = K := by
  simp only [kOfBA, baOfK]; field_simp; ring

theorem baOfK_kOfBA {R θ t BA : ℝ} (hθ : θ ≠ 0) (ht : t ≠ 0) : baOfK R θ t (kOfBA R θ t BA) = BA := by
  simp only [kOfBA, baOfK]; field_simp; ring

/-- **`BA_from_rule`**: under a K rule the bend's effective `κ` is `K`, so `K ∈ [0, 1]` (shop.rs's
`checked_k`) makes the neutral layer physical. -/
theorem BA_from_rule (b : Bend) (K : ℝ) (h : b.BA = baOfK b.R b.θ b.t K) : b.κ = K := by
  have := kOfBA_baOfK (R := b.R) (K := K) b.hθ0.ne' b.ht.ne'
  rw [← h] at this
  simpa [Bend.κ, Bend.rn, kOfBA] using this

theorem BA_from_rule_physical (b : Bend) (K : ℝ) (h : b.BA = baOfK b.R b.θ b.t K)
    (hK : 0 ≤ K ∧ K ≤ 1) : b.R ≤ b.rn ∧ b.rn ≤ b.R + b.t :=
  (b.rn_mem_iff).mpr (by rw [BA_from_rule b K h]; exact hK)

/-! ## Interpolation -/

/-- One window of the interpolation. -/
noncomputable def lerp (x₀ y₀ x₁ y₁ x : ℝ) : ℝ := y₀ + (x - x₀) / (x₁ - x₀) * (y₁ - y₀)

/-- The table interpolation, as shop.rs `interpolate` does it on strictly increasing rows: the
first window holding `x`; `none` off the table. (A single row matches only its own `x`.) -/
noncomputable def interp : List (ℝ × ℝ) → ℝ → Option ℝ
  | [], _ => none
  | [(x₀, y₀)], x => if x = x₀ then some y₀ else none
  | (x₀, y₀) :: (x₁, y₁) :: rest, x =>
      if x₀ ≤ x ∧ x ≤ x₁ then some (lerp x₀ y₀ x₁ y₁ x) else interp ((x₁, y₁) :: rest) x

/-- Strictly increasing in `x`. -/
def Increasing : List (ℝ × ℝ) → Prop
  | [] | [_] => True
  | (x₀, _) :: (x₁, y₁) :: rest => x₀ < x₁ ∧ Increasing ((x₁, y₁) :: rest)

/-- A window's value is a convex combination of its rows. -/
theorem lerp_convex {x₀ y₀ x₁ y₁ x : ℝ} (h01 : x₀ < x₁) (h0 : x₀ ≤ x) (h1 : x ≤ x₁) :
    ∃ l, 0 ≤ l ∧ l ≤ 1 ∧ x = (1 - l) * x₀ + l * x₁ ∧ lerp x₀ y₀ x₁ y₁ x = (1 - l) * y₀ + l * y₁ := by
  have hd : x₁ - x₀ ≠ 0 := by linarith
  refine ⟨(x - x₀) / (x₁ - x₀), div_nonneg (by linarith) (by linarith),
    (div_le_one (by linarith)).mpr (by linarith), ?_, ?_⟩
  · field_simp; ring
  · simp only [lerp]; field_simp; ring

/-- **The interpolation keeps every affine bound of the rows**: if each row `(x, y)` satisfies
`lo x ≤ y ≤ hi x` with `lo` and `hi` affine, so does every interpolated value. -/
theorem interp_affine (lo hi : ℝ → ℝ) (hlo : ∀ a b l : ℝ, lo ((1 - l) * a + l * b) = (1 - l) * lo a + l * lo b)
    (hhi : ∀ a b l : ℝ, hi ((1 - l) * a + l * b) = (1 - l) * hi a + l * hi b) :
    ∀ (rows : List (ℝ × ℝ)), Increasing rows → (∀ r ∈ rows, lo r.1 ≤ r.2 ∧ r.2 ≤ hi r.1) →
      ∀ x y, interp rows x = some y → lo x ≤ y ∧ y ≤ hi x
  | [], _, _, x, y, h => by simp [interp] at h
  | [(x₀, y₀)], _, hr, x, y, h => by
      simp only [interp] at h
      split_ifs at h with hx
      · cases h; subst hx; exact hr (x, y₀) (by simp)
  | (x₀, y₀) :: (x₁, y₁) :: rest, hinc, hr, x, y, h => by
      simp only [interp] at h
      split_ifs at h with hx
      · cases h
        obtain ⟨l, l0, l1, hxl, hy⟩ := lerp_convex (y₀ := y₀) (y₁ := y₁) hinc.1 hx.1 hx.2
        have r0 := hr (x₀, y₀) (by simp)
        have r1 := hr (x₁, y₁) (by simp)
        rw [hy, hxl, hlo, hhi]
        simp only at r0 r1
        constructor <;> nlinarith [r0.1, r0.2, r1.1, r1.2]
      · exact interp_affine lo hi hlo hhi ((x₁, y₁) :: rest) hinc.2
          (fun r hm => hr r (List.mem_cons_of_mem _ hm)) x y h

/-- **`KTable`**: K interpolated against `R/t` between rows in `[0, 1]` stays in `[0, 1]`. -/
theorem ktable_physical (rows : List (ℝ × ℝ)) (hinc : Increasing rows)
    (hr : ∀ r ∈ rows, 0 ≤ r.2 ∧ r.2 ≤ 1) (q K : ℝ) (h : interp rows q = some K) :
    0 ≤ K ∧ K ≤ 1 :=
  interp_affine (fun _ => 0) (fun _ => 1) (fun _ _ _ => by ring) (fun _ _ _ => by ring) rows hinc hr q K h

/-- **`BA_from_table`**: allowance rows `(θᵢ, BAᵢ)` at one radius, each physical
(`R θᵢ ≤ BAᵢ ≤ (R + t) θᵢ`, i.e. its κ in `[0, 1]`), interpolate to a physical allowance at every
angle on the table, because `BA − R θ` and `(R + t) θ − BA` are affine in `θ`. So item 7 is decided
once per row, when the table is read. -/
theorem allowance_physical (R t : ℝ) (rows : List (ℝ × ℝ)) (hinc : Increasing rows)
    (hr : ∀ r ∈ rows, R * r.1 ≤ r.2 ∧ r.2 ≤ (R + t) * r.1) (θ BA : ℝ)
    (h : interp rows θ = some BA) : R * θ ≤ BA ∧ BA ≤ (R + t) * θ :=
  interp_affine (fun x => R * x) (fun x => (R + t) * x) (fun _ _ _ => by ring)
    (fun _ _ _ => by ring) rows hinc hr θ BA h

/-- A physical allowance makes a bend whose neutral layer is in the material. -/
theorem allowance_physical_bend (b : Bend) (h : b.R * b.θ ≤ b.BA ∧ b.BA ≤ (b.R + b.t) * b.θ) :
    0 ≤ b.κ ∧ b.κ ≤ 1 := by
  rw [← b.rn_mem_iff]
  have hθ := b.hθ0
  simp only [Bend.rn]
  constructor
  · rw [le_div_iff₀ hθ]; linarith [h.1]
  · rw [div_le_iff₀ hθ]; linarith [h.2]

/-- The converse: a row with `κ ∉ [0, 1]` is not sheet metal, and refusing it is right. -/
theorem allowance_unphysical (b : Bend) (h : ¬ (0 ≤ b.κ ∧ b.κ ≤ 1)) :
    ¬ (b.R ≤ b.rn ∧ b.rn ≤ b.R + b.t) := by rwa [b.rn_mem_iff]

/-! ## The vendor side: setbacks, the deduction, the minimum flange -/

/-- The outside setback, `OSSB = (R + t) tan (θ / 2)`. -/
noncomputable def ossb (R t θ : ℝ) : ℝ := (R + t) * tan (θ / 2)

/-- The inside setback, `ISSB = R tan (θ / 2)`. -/
noncomputable def issb (R θ : ℝ) : ℝ := R * tan (θ / 2)

/-- The bend deduction from the outside mold lines. -/
noncomputable def bd (R t θ BA : ℝ) : ℝ := 2 * ossb R t θ - BA

theorem ossb_sub_issb (R t θ : ℝ) : ossb R t θ - issb R θ = t * tan (θ / 2) := by
  simp only [ossb, issb]; ring

/-- The kernel's setback is the inside one toward the reference side and the outside one away
from it. -/
theorem s_eq_setback (b : Bend) :
    b.s = match b.sense with
      | .pos => issb b.R b.θ
      | .neg => ossb b.R b.t b.θ := by
  rw [b.s_eq_tan]; cases h : b.sense <;> simp [Bend.ρ, h, issb, ossb]

/-- The outside dimension of a flange is its inside one plus `t tan (θ / 2)`: the two mold lines
of a bend (inside skins meeting, outside skins meeting) are that far apart along the wall. -/
noncomputable def outsideDim (R t θ Lin : ℝ) : ℝ := Lin + (ossb R t θ - issb R θ)

/-- **One flat length, either way it is measured**: flanges given to the outside mold line with
`BD = 2 OSSB − BA`, or to the inside mold line with `2 ISSB − BA`, develop to the same length. -/
theorem flat_length_consistent (R t θ BA La Lb : ℝ) :
    outsideDim R t θ La + outsideDim R t θ Lb - bd R t θ BA =
      La + Lb - (2 * issb R θ - BA) := by
  simp only [outsideDim, bd]; ring

/-- The kernel's deduction `2 s − BA` is the vendor's `BD` when the reference skin is the outside
(`neg`), and the inside deduction when it is the inside (`pos`). -/
theorem deduction_is_bd (b : Bend) :
    2 * b.s - b.BA = match b.sense with
      | .pos => 2 * issb b.R b.θ - b.BA
      | .neg => bd b.R b.t b.θ b.BA := by
  rw [s_eq_setback]; cases b.sense <;> simp [bd]

/-- `tan (π / 4) = 1`, so at 90° the outside setback is `R + t`. -/
theorem ossb_90 (R t : ℝ) : ossb R t (π / 2) = R + t := by
  simp [ossb, show π / 2 / 2 = π / 4 by ring, Real.tan_pi_div_four]

/-- **`BD90`** (forge-vendor `Vendor::shop`): a 90° deduction converts to `BA = 2 (R + t) − BD`
and `K = (BA / (π / 2) − R) / t`, and that K gives back the deduction. -/
theorem bd90 (R t BD : ℝ) (ht : t ≠ 0) :
    let BA := 2 * (R + t) - BD
    bd R t (π / 2) (baOfK R (π / 2) t (kOfBA R (π / 2) t BA)) = BD := by
  intro BA
  rw [baOfK_kOfBA (by positivity) ht, bd, ossb_90]; ring

/-- **The minimum flange to the outside** (checks.rs:56-100): a flange whose reach is measured on
the reference skin to its mold line is `reach + t tan (θ / 2)` long to the outside mold line when
the reference skin is the inside (`pos`, the kernel's `angle > 0`), and `reach` when it already is
the outside. -/
noncomputable def outsideReach (b : Bend) (reach : ℝ) : ℝ := match b.sense with
  | .pos => reach + b.t * tan (b.θ / 2)
  | .neg => reach

theorem outsideReach_eq (b : Bend) (reach : ℝ) :
    outsideReach b reach = reach + (ossb b.R b.t b.θ - (match b.sense with
      | .pos => issb b.R b.θ
      | .neg => ossb b.R b.t b.θ)) := by
  cases h : b.sense <;> simp [outsideReach, h, ossb_sub_issb]

end Sheet
