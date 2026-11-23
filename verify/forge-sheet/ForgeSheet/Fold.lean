import Mathlib

/-!
# The fold of a sheet-metal bend zone (research/kernel-audit/audit2.md §2.0)

A bend zone is the strip `0 ≤ u ≤ BA` of the flat (u across the bend, y along it, w the depth
below the reference skin). Folded, it goes round an axis: in the cross-section spanned by `e_u`
(across, into the zone) and `n` (the reference skin's normal), the point (u, w) goes to

  fold u w = C + ρ(w) (sin φ, −σ cos φ),   φ = u / rn,   C = (0, σ ρ₀),   ρ(w) = ρ₀ + σ w,

y is kept, and `rn = BA / θ` is the neutral radius. `σ = 1` bends toward the reference skin (ρ₀ = R,
the inside radius), `σ = −1` away (ρ₀ = R + t). This is the formula forge-sheet's `Geo::Round`
lays a zone out by, and the one `forge_sheet::refold` refolds the flat with.

Proven here, over ℝ:
* `start`, `start_tangent`: at u = 0 the fold leaves the wall it is cut from, its point at depth w
  straight below the tangent line and its direction across it (continuity, §2.0 item 5);
* `fold_rotation`: the fold of (u, w) is the fold of (0, w) turned by σ u / rn round C, so every
  zone end is a rigid image of its start;
* `angle_at_end`: the far tangent line turns by exactly θ (item 3, the bend allowance);
* `iso`: on the neutral layer ρ(w) = rn the fold is an isometry, |∂fold/∂u| = 1 (item 2; ∂/∂y is
  e_y, unit and square to both);
* `neutral_inside`: the neutral layer lies in the material, its depth in [0, t], iff R ≤ rn ≤ R + t
  (item 7, physical K);
* `setback`: the reference skin's setback ρ sin θ / (1 + cos θ) is ρ tan (θ / 2);
* `deduction`: the developed length is the walls' lengths to their mold lines less the bend
  deduction 2 s − BA (item 4);
* `refold`: along a chain of walls and zones, the flat laid out by the allowances and folded zone by
  zone is the part whose walls are placed by turning each wall's continuation round its zone's
  axis: the flat refolds to the folded part, by induction on the chain (item 1).
-/

namespace ForgeSheet

open Real

/-- A bend zone's numbers. -/
structure Zone where
  ρ₀ : ℝ
  σ : ℝ
  BA : ℝ
  θ : ℝ

namespace Zone

variable (z : Zone)

/-- The neutral radius. -/
noncomputable def rn : ℝ := z.BA / z.θ

/-- The radius at depth w below the reference skin. -/
def ρ (w : ℝ) : ℝ := z.ρ₀ + z.σ * w

/-- The axis, in the cross-section. -/
def C : ℝ × ℝ := (0, z.σ * z.ρ₀)

/-- The fold of the zone point u across, w deep, in the cross-section (e_u, n). -/
noncomputable def fold (u w : ℝ) : ℝ × ℝ :=
  (z.ρ w * sin (u / z.rn), z.σ * z.ρ₀ - z.σ * z.ρ w * cos (u / z.rn))

/-- A turn of the cross-section by α round c. -/
noncomputable def turn (α : ℝ) (c p : ℝ × ℝ) : ℝ × ℝ :=
  (c.1 + cos α * (p.1 - c.1) - sin α * (p.2 - c.2), c.2 + sin α * (p.1 - c.1) + cos α * (p.2 - c.2))

/-- σ is a sense: ±1. -/
def Sense : Prop := z.σ * z.σ = 1

theorem start (h : z.Sense) (w : ℝ) : z.fold 0 w = (0, -w) := by
  unfold fold ρ
  simp only [zero_div, sin_zero, mul_zero, cos_zero, mul_one, Prod.mk.injEq, true_and]
  unfold Sense at h
  linear_combination (-w) * h

theorem fold_rotation (h : z.Sense) (u w : ℝ) :
    z.fold u w = turn (z.σ * (u / z.rn)) z.C (z.fold 0 w) := by
  rw [start z h]
  unfold fold turn C ρ
  rcases mul_self_eq_one_iff.mp h with hσ | hσ <;>
    simp only [hσ, one_mul, neg_mul, sin_neg, cos_neg, Prod.mk.injEq] <;> constructor <;> ring

theorem angle_at_end (hθ : z.θ ≠ 0) (hBA : z.BA ≠ 0) : z.BA / z.rn = z.θ := by
  unfold rn
  field_simp

/-- The far tangent line: the start turned by σ θ round the axis. -/
theorem at_end (h : z.Sense) (hθ : z.θ ≠ 0) (hBA : z.BA ≠ 0) (w : ℝ) :
    z.fold z.BA w = turn (z.σ * z.θ) z.C (z.fold 0 w) := by
  rw [fold_rotation z h, angle_at_end z hθ hBA]

/-- ∂fold/∂u, in closed form. -/
theorem hasDerivAt_fold (w u : ℝ) :
    HasDerivAt (fun u => (z.fold u w).1) (z.ρ w * (cos (u / z.rn) * (1 / z.rn))) u ∧
      HasDerivAt (fun u => (z.fold u w).2) (z.σ * z.ρ w * (sin (u / z.rn) * (1 / z.rn))) u := by
  have hu : HasDerivAt (fun u : ℝ => u / z.rn) (1 / z.rn) u := by
    simpa using (hasDerivAt_id u).div_const z.rn
  constructor
  · exact (hu.sin).const_mul (z.ρ w)
  · have := ((hu.cos).const_mul (z.σ * z.ρ w)).const_sub (z.σ * z.ρ₀)
    simp only [fold]
    convert this using 1
    ring

theorem start_tangent (w : ℝ) :
    deriv (fun u => (z.fold u w).1) 0 = z.ρ w / z.rn ∧ deriv (fun u => (z.fold u w).2) 0 = 0 := by
  obtain ⟨h1, h2⟩ := hasDerivAt_fold z w 0
  rw [h1.deriv, h2.deriv]
  simp [div_eq_mul_inv]

/-- The isometry of the neutral layer: where ρ(w) = rn the fold moves along u at unit speed. -/
theorem iso (h : z.Sense) (hrn : z.rn ≠ 0) (w : ℝ) (hw : z.ρ w = z.rn) (u : ℝ) :
    deriv (fun u => (z.fold u w).1) u ^ 2 + deriv (fun u => (z.fold u w).2) u ^ 2 = 1 := by
  obtain ⟨h1, h2⟩ := hasDerivAt_fold z w u
  rw [h1.deriv, h2.deriv, hw]
  have hσ2 : z.σ ^ 2 = 1 := by rw [sq]; exact h
  have e : ∀ a b : ℝ, (z.rn * (a * (1 / z.rn))) ^ 2 + (z.σ * z.rn * (b * (1 / z.rn))) ^ 2 =
      a ^ 2 + z.σ ^ 2 * b ^ 2 := by
    intro a b; field_simp
  rw [e, hσ2, one_mul]
  exact cos_sq_add_sin_sq _

/-- At the far tangent line the fold runs on in the start's direction turned by σ θ: (cos θ,
σ sin θ), at speed ρ / rn. -/
theorem end_tangent (h : z.Sense) (hθ : z.θ ≠ 0) (hBA : z.BA ≠ 0) (w : ℝ) :
    deriv (fun u => (z.fold u w).1) z.BA = z.ρ w / z.rn * cos (z.σ * z.θ) ∧
      deriv (fun u => (z.fold u w).2) z.BA = z.ρ w / z.rn * sin (z.σ * z.θ) := by
  obtain ⟨h1, h2⟩ := hasDerivAt_fold z w z.BA
  rw [h1.deriv, h2.deriv, angle_at_end z hθ hBA]
  rcases mul_self_eq_one_iff.mp h with hσ | hσ <;>
    simp only [hσ, one_mul, neg_mul, sin_neg, cos_neg] <;> constructor <;> ring

/-- The neutral layer's depth from the reference skin: toward it rn − R, away R + t − rn. It lies
in the material, [0, t], exactly when R ≤ rn ≤ R + t (`K = (rn − R) / t ∈ [0, 1]`). -/
theorem neutral_inside (R t rn : ℝ) :
    (0 ≤ rn - R ∧ rn - R ≤ t ↔ R ≤ rn ∧ rn ≤ R + t) ∧
      (0 ≤ R + t - rn ∧ R + t - rn ≤ t ↔ R ≤ rn ∧ rn ≤ R + t) := by
  constructor <;> constructor <;> rintro ⟨a, b⟩ <;> constructor <;> linarith

/-- The neutral depth makes the radius rn, either sense. -/
theorem neutral_radius (R t rn : ℝ) :
    (Zone.ρ ⟨R, 1, 0, 0⟩ (rn - R) = rn) ∧ (Zone.ρ ⟨R + t, -1, 0, 0⟩ (R + t - rn) = rn) := by
  simp only [ρ]
  constructor <;> ring

end Zone

/-- The setback on the reference skin, ρ sin θ / (1 + cos θ), is ρ tan (θ / 2). -/
theorem setback (ρ θ : ℝ) (h0 : 0 < θ) (hπ : θ < π) :
    ρ * sin θ / (1 + cos θ) = ρ * tan (θ / 2) := by
  have hc : 0 < cos (θ / 2) := cos_pos_of_mem_Ioo ⟨by linarith, by linarith⟩
  have hs : sin θ = 2 * sin (θ / 2) * cos (θ / 2) := by
    rw [← sin_two_mul]; ring_nf
  have hk : 1 + cos θ = 2 * cos (θ / 2) ^ 2 := by
    rw [cos_sq (θ / 2), show 2 * (θ / 2) = θ by ring]; ring
  rw [hs, hk, tan_eq_sin_div_cos]
  field_simp

/-- The length law: two walls La and Lb long to their mold lines, each trimmed by the setback s to
its tangent line, joined by a zone BA wide, lay out La + Lb − (2 s − BA) long. -/
theorem deduction (La Lb s BA : ℝ) : (La - s) + BA + (Lb - s) = La + Lb - (2 * s - BA) := by ring

/-! ## Refold along a chain

A chain alternates walls and zones: wall 0, zone 0, wall 1, zone 1, … Each wall k runs `len k`
from its zone's far tangent line (or the chain's start) to the next zone's near tangent line, in
the cross-section. Laid out flat, wall k starts at `dev k`; folded, it is placed by `place k`: a
turn and a shift of the plane. Wall k + 1 is wall k's continuation turned round zone k's axis by
σₖ θₖ (`next`). The fold of the flat (`flat`) folds zone k by `Zone.fold`, in the frame wall k
reaches its end in. `refold`: on every wall, the fold of the flat is the folded part's wall. -/

/-- A rigid motion of the cross-section: turn by α, then shift by c. -/
structure Motion where
  α : ℝ
  c : ℝ × ℝ

namespace Motion

noncomputable def apply (m : Motion) (p : ℝ × ℝ) : ℝ × ℝ :=
  (cos m.α * p.1 - sin m.α * p.2 + m.c.1, sin m.α * p.1 + cos m.α * p.2 + m.c.2)

/-- `m` after `k`: first k, then m. -/
noncomputable def comp (m k : Motion) : Motion :=
  ⟨m.α + k.α, m.apply k.c⟩

theorem apply_comp (m k : Motion) (p : ℝ × ℝ) : (m.comp k).apply p = m.apply (k.apply p) := by
  simp only [comp, apply, cos_add, sin_add, Prod.mk.injEq]
  constructor <;> ring

/-- The shift along e_u by x. -/
def shift (x : ℝ) : Motion := ⟨0, (x, 0)⟩

/-- The turn by α round c, as a motion. -/
noncomputable def round (α : ℝ) (c : ℝ × ℝ) : Motion :=
  ⟨α, (c.1 - cos α * c.1 + sin α * c.2, c.2 - sin α * c.1 - cos α * c.2)⟩

theorem round_apply (α : ℝ) (c p : ℝ × ℝ) : (round α c).apply p = Zone.turn α c p := by
  simp only [round, apply, Zone.turn, Prod.mk.injEq]
  constructor <;> ring

end Motion

/-- A chain: its walls' lengths, its zones, and the place of wall 0. -/
structure Chain where
  len : ℕ → ℝ
  zone : ℕ → Zone
  start : Motion

namespace Chain

variable (c : Chain)

/-- Where wall k starts on the flat (the developed coordinate, along e_u). -/
noncomputable def dev : ℕ → ℝ
  | 0 => 0
  | k + 1 => dev k + c.len k + (c.zone k).BA

/-- Wall k's place in the folded part: wall k + 1 is wall k's continuation past its length (where
zone k starts), turned round zone k's axis by σ θ; its own coordinates start at zone k's far
tangent line. -/
noncomputable def place : ℕ → Motion
  | 0 => c.start
  | k + 1 =>
    (place k).comp ((Motion.shift (c.len k)).comp (Motion.round ((c.zone k).σ * (c.zone k).θ) (c.zone k).C))

/-- The fold of the flat on zone k: Zone.fold in the frame wall k ends in, u from the near tangent
line. -/
noncomputable def foldZone (k : ℕ) (u w : ℝ) : ℝ × ℝ :=
  ((c.place k).comp (Motion.shift (c.len k))).apply ((c.zone k).fold u w)

/-- The fold of the flat on wall k: rigid, by the wall's place. -/
noncomputable def foldWall (k : ℕ) (x w : ℝ) : ℝ × ℝ :=
  (c.place k).apply (x - c.dev k, -w)

/-- Every zone a sense, an angle and a width not zero. -/
def Good : Prop := ∀ k, (c.zone k).Sense ∧ (c.zone k).θ ≠ 0 ∧ (c.zone k).BA ≠ 0

/-- Zone k starts where wall k ends: the fold is continuous at every near tangent line. -/
theorem zone_starts_on_wall (hc : c.Good) (k : ℕ) (w : ℝ) :
    c.foldZone k 0 w = c.foldWall k (c.dev k + c.len k) w := by
  unfold foldZone foldWall
  rw [Zone.start _ (hc k).1, Motion.apply_comp]
  congr 1
  simp [Motion.shift, Motion.apply]

/-- Zone k ends where wall k + 1 starts: the fold is continuous at every far tangent line. -/
theorem zone_ends_on_wall (hc : c.Good) (k : ℕ) (w : ℝ) :
    c.foldZone k (c.zone k).BA w = c.foldWall (k + 1) (c.dev (k + 1)) w := by
  obtain ⟨hs, hθ, hBA⟩ := hc k
  unfold foldZone foldWall
  rw [Zone.at_end _ hs hθ hBA, Zone.start _ hs, ← Motion.round_apply, place]
  simp only [Motion.apply_comp, sub_self]

/-- The refold of a chain (§2.0 item 1): laid out flat by the allowances (`dev`) and folded zone by
zone (`foldZone`, `foldWall`), it is the folded part whose walls `place` builds from the zones'
turns. At every junction, the near and the far tangent line of every zone, the zone's fold meets
the walls' in position, and (`Zone.start_tangent`, `Zone.end_tangent`, at unit speed on the
neutral layer by `Zone.iso`) in direction: wall k + 1 runs on in zone k's end direction, turned
σ θ from wall k's. -/
theorem refold (hc : c.Good) (k : ℕ) (w : ℝ) :
    c.foldZone k 0 w = c.foldWall k (c.dev k + c.len k) w ∧
      c.foldZone k (c.zone k).BA w = c.foldWall (k + 1) (c.dev (k + 1)) w :=
  ⟨c.zone_starts_on_wall hc k w, c.zone_ends_on_wall hc k w⟩

/-- Along wall k + 1, the fold of the flat is zone k's far end moved on along its end direction:
`turn` is rigid, so the wall's points lie at their developed distance from the zone's end. -/
theorem wall_after_zone (k : ℕ) (x w : ℝ) :
    c.foldWall (k + 1) (c.dev (k + 1) + x) w =
      ((c.place k).comp (Motion.shift (c.len k))).apply
        (Zone.turn ((c.zone k).σ * (c.zone k).θ) (c.zone k).C (x, -w)) := by
  unfold foldWall
  rw [place, ← Motion.round_apply, Motion.apply_comp, Motion.apply_comp, Motion.apply_comp]
  congr 3
  ring

end Chain

end ForgeSheet
