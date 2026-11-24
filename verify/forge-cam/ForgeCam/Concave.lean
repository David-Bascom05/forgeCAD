/-
A dome cylinder's exact move bound (kernel/forge-cam-geom/src/drop.rs `cyl_chord`, `section`,
`Column::sq_convex`), L1.

In the vertical plane across a horizontal cylinder's axis (`line_bound`, Drop.lean), with the
tool's axis at lateral place d, its column meets the cylinder's disk (radius r, centre at height
0) where a disk point at lateral e and a column point at u = e − d meet:

  CL(d) = sup { T e + G u : e ∈ [−r, r], u ∈ [−R, R], e − u = d },
  T e = √(r² − e²) (the disk's top),  G u = −b(|u|) (the column's profile, upside down).

Along a straight move the tip is affine in d: z(d) = z_l + λ (d − d_l). With T and G concave
(`top_concave`, `neg_profile_concave`: b convex and non-decreasing), the move's worst point is
found exactly:

* `disk_conj`, `disk_conj_at`: the disk's part of the unconstrained maximum of T e − λ e, and
  where it is attained;
* `end_of_outside`: if a maximiser (e*, u*) of T e + G u − λ (e − u) lies at a lateral place
  d* = e* − u* on the far side of d_l from d, every pair at d is matched by one at d_l
  (convex combination with the maximiser): the max over the move is at its end;
* `move_clear_of_outside`: so a tip clear at that end is clear all along;
* `move_le_unconstrained`: else the unconstrained maximum bounds it.

Past the cylinder's end along its axis by a ≥ 0:
* `end_rho_ge`: every point of the (finite) cylinder is at least √((e − d)² + a²) from the tool's
  axis, so b(ρ) ≥ b(√(u² + a²)) (`section_mono`): the column's section a off its axis, which for
  a ball is a ball again (`ball_section`) and a flat end a flat end;
* `superadd`, `profile_superadd`: for other profiles with h = b ∘ √ convex,
  b(√(u² + a²)) ≥ b(|u|) + b(a) − b(0).

Open (RESUME.md): that conj's maximiser and the 1D candidates are found (L6-tested in
tests/drop_vs_sampling.rs), and the interval arithmetic (L5).
-/
import ForgeCam.SignedDist

open Set

namespace ForgeCam.Concave

/-- The disk's conjugate: √(r² − e²) − λ e ≤ r √(1 + λ²) on the disk (Cauchy–Schwarz). -/
theorem disk_conj {r e l : ℝ} (hr : 0 ≤ r) (he : e ^ 2 ≤ r ^ 2) :
    Real.sqrt (r ^ 2 - e ^ 2) - l * e ≤ r * Real.sqrt (1 + l ^ 2) := by
  have hs : Real.sqrt (r ^ 2 - e ^ 2) ^ 2 = r ^ 2 - e ^ 2 := Real.sq_sqrt (by linarith)
  have hw : Real.sqrt (1 + l ^ 2) ^ 2 = 1 + l ^ 2 := Real.sq_sqrt (by positivity)
  have hw0 : 0 ≤ Real.sqrt (1 + l ^ 2) := Real.sqrt_nonneg _
  have key : (Real.sqrt (r ^ 2 - e ^ 2) - l * e) ^ 2 ≤ (r * Real.sqrt (1 + l ^ 2)) ^ 2 := by
    have h1 : (r * Real.sqrt (1 + l ^ 2)) ^ 2
        = (Real.sqrt (r ^ 2 - e ^ 2) ^ 2 + e ^ 2) * (1 + l ^ 2) := by
      rw [mul_pow, hw, hs]; ring
    rw [h1]
    nlinarith [sq_nonneg (l * Real.sqrt (r ^ 2 - e ^ 2) + e)]
  exact (abs_le_of_sq_le_sq' key (mul_nonneg hr hw0)).2

/-- Where the disk's conjugate is attained: e* = −λ r / √(1 + λ²). -/
theorem disk_conj_at {r l : ℝ} (hr : 0 ≤ r) :
    (-(l * r) / Real.sqrt (1 + l ^ 2)) ^ 2 ≤ r ^ 2 ∧
      Real.sqrt (r ^ 2 - (-(l * r) / Real.sqrt (1 + l ^ 2)) ^ 2)
          - l * (-(l * r) / Real.sqrt (1 + l ^ 2)) = r * Real.sqrt (1 + l ^ 2) := by
  set w := Real.sqrt (1 + l ^ 2) with hwdef
  have hw : w ^ 2 = 1 + l ^ 2 := Real.sq_sqrt (by positivity)
  have hwp : 0 < w := Real.sqrt_pos.2 (by positivity)
  have he2 : (-(l * r) / w) ^ 2 = l ^ 2 * r ^ 2 / w ^ 2 := by ring
  have hrest : r ^ 2 - (-(l * r) / w) ^ 2 = (r / w) ^ 2 := by
    rw [he2, div_pow, hw]; field_simp; ring
  constructor
  · rw [he2, hw, div_le_iff₀ (by positivity)]
    nlinarith [sq_nonneg r]
  · rw [hrest, Real.sqrt_sq (div_nonneg hr hwp.le)]
    field_simp
    nlinarith [hw]

/-- The disk's top is concave on the disk. -/
theorem top_concave (r : ℝ) :
    ConcaveOn ℝ (Icc (-r) r) (fun e => Real.sqrt (r ^ 2 - e ^ 2)) := by
  refine ⟨convex_Icc _ _, ?_⟩
  intro x hx y hy a b ha hb hab
  simp only [smul_eq_mul]
  have hx2 : x ^ 2 ≤ r ^ 2 := sq_le_sq' hx.1 hx.2
  have hy2 : y ^ 2 ≤ r ^ 2 := sq_le_sq' hy.1 hy.2
  set X := Real.sqrt (r ^ 2 - x ^ 2)
  set Y := Real.sqrt (r ^ 2 - y ^ 2)
  have hX : X ^ 2 = r ^ 2 - x ^ 2 := Real.sq_sqrt (by linarith)
  have hY : Y ^ 2 = r ^ 2 - y ^ 2 := Real.sq_sqrt (by linarith)
  have hX0 : 0 ≤ X := Real.sqrt_nonneg _
  have hY0 : 0 ≤ Y := Real.sqrt_nonneg _
  -- x y + X Y ≤ r² (Cauchy–Schwarz: both vectors have length r)
  have hc : x * y + X * Y ≤ r ^ 2 := by
    have h2 : (x * y + X * Y) ^ 2 ≤ (r ^ 2) ^ 2 := by
      nlinarith [sq_nonneg (x * Y - y * X)]
    exact (abs_le_of_sq_le_sq' h2 (sq_nonneg r)).2
  apply Real.le_sqrt_of_sq_le
  have hb' : b = 1 - a := by linarith
  subst hb'
  nlinarith [mul_nonneg ha hb, hc]

/-- The column's profile upside down, −b(|u|), is concave on [−R, R] when b is convex and
non-decreasing on [0, R]. -/
theorem neg_profile_concave {R : ℝ} {b : ℝ → ℝ} (hc : ConvexOn ℝ (Icc 0 R) b)
    (hm : MonotoneOn b (Icc 0 R)) :
    ConcaveOn ℝ (Icc (-R) R) (fun u => -b |u|) := by
  refine ⟨convex_Icc _ _, ?_⟩
  intro x hx y hy a c ha hc' hac
  simp only [smul_eq_mul]
  have hxa : |x| ∈ Icc 0 R := ⟨abs_nonneg _, abs_le.2 ⟨hx.1, hx.2⟩⟩
  have hya : |y| ∈ Icc 0 R := ⟨abs_nonneg _, abs_le.2 ⟨hy.1, hy.2⟩⟩
  have hcomb : a * |x| + c * |y| ∈ Icc 0 R :=
    (convex_Icc 0 R) hxa hya ha hc' hac
  have hle : |a * x + c * y| ≤ a * |x| + c * |y| := by
    calc |a * x + c * y| ≤ |a * x| + |c * y| := abs_add_le _ _
      _ = a * |x| + c * |y| := by rw [abs_mul, abs_mul, abs_of_nonneg ha, abs_of_nonneg hc']
  have hin : |a * x + c * y| ∈ Icc 0 R := ⟨abs_nonneg _, hle.trans hcomb.2⟩
  have h1 : b |a * x + c * y| ≤ b (a * |x| + c * |y|) := hm hin hcomb hle
  have h2 := hc.2 hxa hya ha hc' hac
  simp only [smul_eq_mul] at h2
  linarith

/-- A pair at a lateral place between a maximiser's and its own is matched at that place: the
convex combination of the two pairs (both functions concave). -/
theorem end_of_outside {S I : Set ℝ} {T G : ℝ → ℝ} (hT : ConcaveOn ℝ S T)
    (hG : ConcaveOn ℝ I G) {l es us e u dl : ℝ} (hes : es ∈ S) (hus : us ∈ I) (he : e ∈ S)
    (hu : u ∈ I) (hmax : T e + G u - l * (e - u) ≤ T es + G us - l * (es - us))
    (hdl : dl ∈ segment ℝ (es - us) (e - u)) :
    ∃ e' ∈ S, ∃ u' ∈ I, e' - u' = dl ∧ T e + G u - l * (e - u) ≤ T e' + G u' - l * dl := by
  obtain ⟨a, c, ha, hc, hac, hd⟩ := hdl
  simp only [smul_eq_mul] at hd
  refine ⟨a * es + c * e, hT.1 hes he ha hc hac, a * us + c * u, hG.1 hus hu ha hc hac, ?_, ?_⟩
  · rw [← hd]; ring
  · have h1 := hT.2 hes he ha hc hac
    have h2 := hG.2 hus hu ha hc hac
    simp only [smul_eq_mul] at h1 h2
    rw [← hd]
    have hc' : c = 1 - a := by linarith
    subst hc'
    nlinarith [mul_le_mul_of_nonneg_left hmax ha]

/-- The move's end decides: a maximiser of T e + G u − λ (e − u) at a lateral place on the far
side of the end d_l from d, and the tip clear at d_l (every pair there at most z_l), make every
pair at d at most the tip's height there. -/
theorem move_clear_of_outside {S I : Set ℝ} {T G : ℝ → ℝ} (hT : ConcaveOn ℝ S T)
    (hG : ConcaveOn ℝ I G) {l es us zl dl : ℝ} (hes : es ∈ S) (hus : us ∈ I)
    (hmax : ∀ e ∈ S, ∀ u ∈ I, T e + G u - l * (e - u) ≤ T es + G us - l * (es - us))
    (hend : ∀ e ∈ S, ∀ u ∈ I, e - u = dl → T e + G u ≤ zl)
    {e u : ℝ} (he : e ∈ S) (hu : u ∈ I) (hdl : dl ∈ uIcc (es - us) (e - u)) :
    T e + G u ≤ zl + l * ((e - u) - dl) := by
  rw [← segment_eq_uIcc] at hdl
  obtain ⟨e', he', u', hu', hd', hv⟩ := end_of_outside hT hG hes hus he hu (hmax e he u hu) hdl
  have := hend e' he' u' hu' hd'
  linarith

/-- Else the unconstrained maximum bounds every pair along the tip's line. -/
theorem move_le_unconstrained {S I : Set ℝ} {T G : ℝ → ℝ} {l es us zl dl : ℝ}
    (hmax : ∀ e ∈ S, ∀ u ∈ I, T e + G u - l * (e - u) ≤ T es + G us - l * (es - us))
    {e u : ℝ} (he : e ∈ S) (hu : u ∈ I) :
    T e + G u - (zl + l * ((e - u) - dl)) ≤ T es + G us - l * (es - us) - zl + l * dl := by
  have := hmax e he u hu
  linarith

/-- Past a horizontal cylinder's end by a: a point of it at axial place t' (within its span,
which ends a or more before the tool's axial place t) is at least √(w² + a²) from the tool's
axis, w its lateral offset. -/
theorem end_rho_ge {w t t' a : ℝ} (ha : 0 ≤ a) (ht : a ≤ t - t') :
    Real.sqrt (w ^ 2 + a ^ 2) ≤ Real.sqrt (w ^ 2 + (t - t') ^ 2) := by
  apply Real.sqrt_le_sqrt
  nlinarith

/-- So a non-decreasing profile there is at least its section a off the axis. -/
theorem section_mono {b : ℝ → ℝ} (hm : MonotoneOn b (Ici 0)) {u a a' : ℝ} (ha : 0 ≤ a)
    (haa : a ≤ a') : b (Real.sqrt (u ^ 2 + a ^ 2)) ≤ b (Real.sqrt (u ^ 2 + a' ^ 2)) :=
  hm (Real.sqrt_nonneg _) (Real.sqrt_nonneg _)
    (Real.sqrt_le_sqrt (by nlinarith))

/-- A ball's section a off its axis is a ball of radius √(r² − a²) about the same centre. -/
theorem ball_section (c r u a : ℝ) :
    c - Real.sqrt (r ^ 2 - (Real.sqrt (u ^ 2 + a ^ 2)) ^ 2)
      = c - Real.sqrt ((r ^ 2 - a ^ 2) - u ^ 2) := by
  rw [Real.sq_sqrt (by positivity)]
  ring_nf

/-- A convex function on [0, M] with its value at 0 is superadditive there. -/
theorem superadd {M : ℝ} {h : ℝ → ℝ} (hh : ConvexOn ℝ (Icc 0 M) h) {x y : ℝ} (hx : 0 ≤ x)
    (hy : 0 ≤ y) (hxy : x + y ≤ M) : h x + h y ≤ h (x + y) + h 0 := by
  rcases eq_or_lt_of_le (add_nonneg hx hy) with h0 | hpos
  · have hx0 : x = 0 := by linarith
    have hy0 : y = 0 := by linarith
    subst hx0; subst hy0; simp
  have hM : (x + y) ∈ Icc 0 M := ⟨hpos.le, hxy⟩
  have h0M : (0 : ℝ) ∈ Icc 0 M := ⟨le_refl _, hpos.le.trans hxy⟩
  have hxs : x / (x + y) + y / (x + y) = 1 := by
    rw [← add_div, div_self hpos.ne']
  have c1 := hh.2 hM h0M (div_nonneg hx hpos.le) (div_nonneg hy hpos.le) hxs
  have c2 := hh.2 hM h0M (div_nonneg hy hpos.le) (div_nonneg hx hpos.le) (by linarith)
  simp only [smul_eq_mul, mul_zero, add_zero] at c1 c2
  have e1 : x / (x + y) * (x + y) = x := div_mul_cancel₀ x hpos.ne'
  have e2 : y / (x + y) * (x + y) = y := div_mul_cancel₀ y hpos.ne'
  rw [e1] at c1
  rw [e2] at c2
  have s1 : x / (x + y) * h (x + y) + y / (x + y) * h (x + y) = h (x + y) := by
    rw [← add_mul, hxs, one_mul]
  have s0 : y / (x + y) * h 0 + x / (x + y) * h 0 = h 0 := by
    rw [← add_mul, add_comm, hxs, one_mul]
  linarith

/-- With h = b ∘ √ convex: b(√(u² + a²)) ≥ b(|u|) + b(a) − b(0) (a ≥ 0, within the column). -/
theorem profile_superadd {R : ℝ} {b : ℝ → ℝ}
    (hh : ConvexOn ℝ (Icc 0 (R ^ 2)) (fun x => b (Real.sqrt x))) {u a : ℝ} (ha : 0 ≤ a)
    (hin : u ^ 2 + a ^ 2 ≤ R ^ 2) :
    b |u| + b a - b 0 ≤ b (Real.sqrt (u ^ 2 + a ^ 2)) := by
  have := superadd hh (sq_nonneg u) (sq_nonneg a) hin
  simp only [Real.sqrt_sq_eq_abs, Real.sqrt_sq ha, Real.sqrt_zero] at this
  linarith

end ForgeCam.Concave
