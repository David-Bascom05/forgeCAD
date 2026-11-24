/-
The closed forms by symmetry (kernel/forge-cam-geom/src/shadow.rs, CAM 007ce37), sound: a plane
with a horizontal direction (a 45° chamfer, a draft) and a cone with a vertical axis (a chamfer
round an arc, a countersink). `shadow::clear` only ever proves a clearance; anything else goes to
the branch and bound.

The checked quantity is sd(tool at A, P) = sdRZ(|P − A| horizontally, P.z − A.z)
(`sd_hyp_eq_sdRZ`). The statements, as CAM gave them:
- S1 (`shadow_point`): a lower bound h ≥ 0 on the horizontal distance and the heights in Z give
  min over z ∈ Z of sdRZ(h, z) as a lower bound;
- S2 (plane): in the horizontal frame (n across e, ê along it, `frame_orthonormal`), the
  horizontal distance is at least √(gap across² + gap along²) (`frame_gap`). The face's
  coordinates are linear in its parameters (`plane_coord`); a line move's lie between its
  ends' (`line_coord`); an arc move's lie within r of its centre's (`arc_coord`);
- S3 (cone): the face's distance from the axis is |r0 + t k|·|c x + s y| with
  |c x + s y|² ∈ [min(|x|², |y|²) − |x·y|, max(|x|², |y|²) + |x·y|] (`cone_factor`); the
  horizontal distance is at least the gap between the face's and the move's distances from the
  axis (`axis_gap`). A move's distance from the axis: a line's at most its ends' larger
  (`line_far`), at least slab's point–segment candidate (`ptSegSq_le`); an arc's at most
  |c − o| + r (`arc_far`), at least its point–arc candidate less its ends' gap (`arc_near`);
- S4 (`shadow_cover`): leaves covering the face's parameter cell and the move's [0, 1], each
  with its bound, give their least as a lower bound everywhere. `certified_clear_gen` carries a
  per-face lower bound (any face kind) with the faces lowered within μ into the clearance.

The frame vectors and dot products are exact reals here. shadow.rs encloses each of them in an
outward-rounded interval (n = (−e.y, e.x)/|e| and ê = (n.y, −n.x) from an interval |e|), which is
the L5 part.
-/
import ForgeCam.Trim

open Metric Set ForgeCam.Dist2 ForgeCam.DistArc ForgeCam.Trim

namespace ForgeCam.Shadow

noncomputable section

/-! ### S1 -/

/-- **S1, the pointwise bound.** -/
theorem shadow_point {Z : Set ℝ} {w : ℝ → ℝ} {P A : E3} {h L : ℝ} {Zs : Set ℝ} (hh : 0 ≤ h)
    (hρ : h ≤ ρ (P - A)) (hz : P 2 - A 2 ∈ Zs) (hL : ∀ z ∈ Zs, L ≤ sdRZ Z w h z) :
    L ≤ sd (translate A (hyp Z w)) P := by
  rw [sd_translate]
  have h1 := hL _ hz
  have h2 := sdRZ_le_sd (Z := Z) (w := w) (p := P - A) hh hρ
  have e : (P - A) 2 = P 2 - A 2 := by simp
  rw [e] at h2
  linarith

/-- The horizontal distance in space is the plane distance of the points' xy. -/
def xy (p : E3) : ForgeCam.DistArc.P := (p 0, p 1)

theorem ρ_sub_eq (P A : E3) : ρ (P - A) = √(nsq (xy P - xy A)) := by
  unfold ρ; congr 1; simp only [xy, nsq, dot, Prod.fst_sub, Prod.snd_sub, PiLp.sub_apply]; ring

/-! ### S2: the plane -/

/-- The least distance between two intervals [a0, a1], [b0, b1] (0 where they meet): shadow.rs's
`gap`. -/
def gapI (a0 a1 b0 b1 : ℝ) : ℝ := max 0 (max (a0 - b1) (b0 - a1))

theorem gapI_le {a0 a1 b0 b1 x y : ℝ} (hx : a0 ≤ x ∧ x ≤ a1) (hy : b0 ≤ y ∧ y ≤ b1) :
    gapI a0 a1 b0 b1 ≤ |x - y| := by
  unfold gapI
  refine max_le (abs_nonneg _) (max_le ?_ ?_)
  · exact le_trans (by linarith) (le_abs_self _)
  · exact le_trans (by linarith : b0 - a1 ≤ y - x) (by rw [abs_sub_comm]; exact le_abs_self _)

theorem gapI_nonneg (a0 a1 b0 b1 : ℝ) : 0 ≤ gapI a0 a1 b0 b1 := le_max_left _ _

/-- The horizontal frame of e ≠ 0: n across it, ê along it. -/
def nrm (e : ForgeCam.DistArc.P) : ForgeCam.DistArc.P := sc (1 / √(nsq e)) (-e.2, e.1)
def along (e : ForgeCam.DistArc.P) : ForgeCam.DistArc.P := sc (1 / √(nsq e)) e

theorem frame_orthonormal {e : ForgeCam.DistArc.P} (he : 0 < nsq e) :
    nsq (nrm e) = 1 ∧ nsq (along e) = 1 ∧ dot (nrm e) (along e) = 0 ∧
      along e = ((nrm e).2, -(nrm e).1) := by
  have hL : (√(nsq e)) ^ 2 = nsq e := Real.sq_sqrt he.le
  have hL0 : 0 < √(nsq e) := Real.sqrt_pos.mpr he
  refine ⟨?_, ?_, ?_, ?_⟩
  · have : nsq (nrm e) = (1 / √(nsq e)) ^ 2 * nsq e := by simp only [nrm, nsq, dot, sc]; ring
    rw [this, div_pow, hL]; field_simp
  · have : nsq (along e) = (1 / √(nsq e)) ^ 2 * nsq e := by simp only [along, nsq, dot, sc]; ring
    rw [this, div_pow, hL]; field_simp
  · simp only [nrm, along, dot, sc]; ring
  · ext <;> simp [nrm, along, sc]

/-- In an orthonormal frame, the squared distance is the sum of the squared coordinates. -/
theorem frame_nsq {n e x : ForgeCam.DistArc.P} (hn : nsq n = 1) (he : nsq e = 1) (hne : dot n e = 0) :
    nsq x = dot x n ^ 2 + dot x e ^ 2 := by
  simp only [nsq, dot] at hn he hne ⊢
  obtain ⟨l, hld⟩ : ∃ l : ℝ, l = n.1 * e.2 - n.2 * e.1 := ⟨_, rfl⟩
  have he1 : e.1 = -l * n.2 := by linear_combination (-e.1) * hn + n.1 * hne + n.2 * hld
  have he2 : e.2 = l * n.1 := by linear_combination (-e.2) * hn + n.2 * hne - n.1 * hld
  have hl : l ^ 2 = 1 := by
    rw [he1, he2] at he; linear_combination he - l ^ 2 * hn
  rw [he1, he2]
  linear_combination (-(x.1 * x.1 + x.2 * x.2)) * hn - (x.2 * n.1 - x.1 * n.2) ^ 2 * hl

/-- **S2: the gap bound.** If x's and y's coordinates across and along lie in the intervals, the
distance is at least √(gap across² + gap along²). -/
theorem frame_gap {n e x y : ForgeCam.DistArc.P} (hn : nsq n = 1) (he : nsq e = 1)
    (hne : dot n e = 0) {a0 a1 b0 b1 c0 c1 d0 d1 : ℝ}
    (hxn : a0 ≤ dot x n ∧ dot x n ≤ a1) (hyn : b0 ≤ dot y n ∧ dot y n ≤ b1)
    (hxe : c0 ≤ dot x e ∧ dot x e ≤ c1) (hye : d0 ≤ dot y e ∧ dot y e ≤ d1) :
    √(gapI a0 a1 b0 b1 ^ 2 + gapI c0 c1 d0 d1 ^ 2) ≤ √(nsq (x - y)) := by
  apply Real.sqrt_le_sqrt
  rw [frame_nsq hn he hne]
  have e1 : dot (x - y) n = dot x n - dot y n := by simp only [dot, Prod.fst_sub, Prod.snd_sub]; ring
  have e2 : dot (x - y) e = dot x e - dot y e := by simp only [dot, Prod.fst_sub, Prod.snd_sub]; ring
  rw [e1, e2]
  have g1 := gapI_le hxn hyn
  have g2 := gapI_le hxe hye
  have p1 := gapI_nonneg a0 a1 b0 b1
  have p2 := gapI_nonneg c0 c1 d0 d1
  rw [← sq_abs (dot x n - dot y n), ← sq_abs (dot x e - dot y e)]
  nlinarith [abs_nonneg (dot x n - dot y n), abs_nonneg (dot x e - dot y e)]

/-- The face's coordinate: o + p e + t g, linear in p and t. -/
theorem plane_coord (o e g v : ForgeCam.DistArc.P) (p t : ℝ) :
    dot (o + sc p e + sc t g) v = dot o v + p * dot e v + t * dot g v := by
  simp only [dot, sc, Prod.fst_add, Prod.snd_add]; ring

/-- A line move's coordinate lies between its ends'. -/
theorem line_coord (a b v : ForgeCam.DistArc.P) {s : ℝ} (h0 : 0 ≤ s) (h1 : s ≤ 1) :
    min (dot a v) (dot b v) ≤ dot (pt a (b - a) s) v ∧ dot (pt a (b - a) s) v ≤ max (dot a v) (dot b v) := by
  have e : dot (pt a (b - a) s) v = dot a v + s * (dot b v - dot a v) := by
    simp only [dot, pt, Prod.fst_sub, Prod.snd_sub]; ring
  rw [e]
  constructor
  · rcases le_total (dot a v) (dot b v) with h | h
    · rw [min_eq_left h]; nlinarith
    · rw [min_eq_right h]; nlinarith
  · rcases le_total (dot a v) (dot b v) with h | h
    · rw [max_eq_right h]; nlinarith
    · rw [max_eq_left h]; nlinarith

/-- An arc move's coordinate lies within r of its centre's (v a unit vector). -/
theorem arc_coord {c A v : ForgeCam.DistArc.P} {r : ℝ} (hr : 0 ≤ r) (hA : nsq (A - c) = r ^ 2)
    (hv : nsq v = 1) : dot c v - r ≤ dot A v ∧ dot A v ≤ dot c v + r := by
  have h1 := dot_le_sqrt (A - c) v (zero_le_one) (by rw [hv]; norm_num)
  have h2 := dot_le_sqrt (c - A) v (zero_le_one) (by rw [hv]; norm_num)
  have s1 : √(nsq (A - c)) = r := by rw [hA, Real.sqrt_sq hr]
  have s2 : √(nsq (c - A)) = r := by rw [Dist2.nsq_comm, hA, Real.sqrt_sq hr]
  rw [s1] at h1; rw [s2] at h2
  have e1 : dot (A - c) v = dot A v - dot c v := by simp only [dot, Prod.fst_sub, Prod.snd_sub]; ring
  have e2 : dot (c - A) v = dot c v - dot A v := by simp only [dot, Prod.fst_sub, Prod.snd_sub]; ring
  rw [e1] at h1; rw [e2] at h2
  constructor <;> linarith

/-! ### S3: the cone -/

/-- **The cone's radius factor**: for c² + s² = 1, |c x + s y|² lies within the bounds shadow.rs
encloses. -/
theorem cone_factor (x y : ForgeCam.DistArc.P) {c s : ℝ} (hcs : c ^ 2 + s ^ 2 = 1) :
    min (nsq x) (nsq y) - |dot x y| ≤ nsq (sc c x + sc s y) ∧
      nsq (sc c x + sc s y) ≤ max (nsq x) (nsq y) + |dot x y| := by
  have e : nsq (sc c x + sc s y) = c ^ 2 * nsq x + s ^ 2 * nsq y + 2 * c * s * dot x y := by
    simp only [nsq, dot, sc, Prod.fst_add, Prod.snd_add]; ring
  rw [e]
  have h2cs : |2 * c * s| ≤ 1 := by
    rw [abs_le]; constructor <;> nlinarith [sq_nonneg (c + s), sq_nonneg (c - s)]
  have hb : |2 * c * s * dot x y| ≤ |dot x y| := by
    rw [abs_mul]; exact mul_le_of_le_one_left (abs_nonneg _) h2cs
  obtain ⟨hb1, hb2⟩ := abs_le.mp hb
  have hc2 := sq_nonneg c; have hs2 := sq_nonneg s
  constructor
  · have : min (nsq x) (nsq y) ≤ c ^ 2 * nsq x + s ^ 2 * nsq y := by
      have h1 := mul_le_mul_of_nonneg_left (min_le_left (nsq x) (nsq y)) hc2
      have h2 := mul_le_mul_of_nonneg_left (min_le_right (nsq x) (nsq y)) hs2
      have h3 : (c ^ 2 + s ^ 2) * min (nsq x) (nsq y) = min (nsq x) (nsq y) := by rw [hcs, one_mul]
      linarith [show (c ^ 2 + s ^ 2) * min (nsq x) (nsq y) =
        c ^ 2 * min (nsq x) (nsq y) + s ^ 2 * min (nsq x) (nsq y) by ring]
    linarith
  · have : c ^ 2 * nsq x + s ^ 2 * nsq y ≤ max (nsq x) (nsq y) := by
      have h1 := mul_le_mul_of_nonneg_left (le_max_left (nsq x) (nsq y)) hc2
      have h2 := mul_le_mul_of_nonneg_left (le_max_right (nsq x) (nsq y)) hs2
      have h3 : (c ^ 2 + s ^ 2) * max (nsq x) (nsq y) = max (nsq x) (nsq y) := by rw [hcs, one_mul]
      linarith [show (c ^ 2 + s ^ 2) * max (nsq x) (nsq y) =
        c ^ 2 * max (nsq x) (nsq y) + s ^ 2 * max (nsq x) (nsq y) by ring]
    linarith

/-- The cone's face point, horizontally: o + rr (c x + s y), at distance |rr|·|c x + s y| from
the axis. -/
theorem cone_radius (o x y : ForgeCam.DistArc.P) (rr c s : ℝ) :
    √(nsq (o + sc rr (sc c x + sc s y) - o)) = |rr| * √(nsq (sc c x + sc s y)) := by
  have : nsq (o + sc rr (sc c x + sc s y) - o) = rr ^ 2 * nsq (sc c x + sc s y) := by
    simp only [nsq, dot, sc, Prod.fst_add, Prod.snd_add, Prod.fst_sub, Prod.snd_sub]; ring
  rw [this, Real.sqrt_mul (sq_nonneg _), Real.sqrt_sq_eq_abs]

/-- **The axis gap**: the horizontal distance is at least the gap between the two distances from
the axis. -/
theorem axis_gap {P A o : ForgeCam.DistArc.P} {a0 a1 b0 b1 : ℝ}
    (hP : a0 ≤ d2 o P ∧ d2 o P ≤ a1) (hA : b0 ≤ d2 o A ∧ d2 o A ≤ b1) :
    gapI a0 a1 b0 b1 ≤ d2 A P := by
  refine (gapI_le hP hA).trans ?_
  rw [abs_le]
  have t1 := d2_triangle o A P
  have t2 := d2_triangle o P A
  rw [d2_comm P A] at t2
  constructor <;> linarith

theorem sqrt_nsq_add_le (u v : ForgeCam.DistArc.P) : √(nsq (u + v)) ≤ √(nsq u) + √(nsq v) := by
  have h := d2_triangle 0 u (u + v)
  rw [d2_eq_sqrt, d2_eq_sqrt, d2_eq_sqrt, sub_zero, sub_zero, add_sub_cancel_left] at h
  exact h

theorem sqrt_nsq_sc (k : ℝ) (w : ForgeCam.DistArc.P) : √(nsq (sc k w)) = |k| * √(nsq w) := by
  have : nsq (sc k w) = k ^ 2 * nsq w := by simp only [nsq, dot, sc]; ring
  rw [this, Real.sqrt_mul (sq_nonneg _), Real.sqrt_sq_eq_abs]

/-- A line move's distance from the axis is at most its ends' larger. -/
theorem line_far (o a b : ForgeCam.DistArc.P) {s : ℝ} (h0 : 0 ≤ s) (h1 : s ≤ 1) :
    d2 o (pt a (b - a) s) ≤ max (d2 o a) (d2 o b) := by
  have e : pt a (b - a) s - o = sc (1 - s) (a - o) + sc s (b - o) := by
    ext <;> simp [pt, sc] <;> ring
  rw [d2_eq_sqrt, e]
  refine (sqrt_nsq_add_le _ _).trans ?_
  rw [sqrt_nsq_sc, sqrt_nsq_sc, abs_of_nonneg (by linarith : (0:ℝ) ≤ 1 - s), abs_of_nonneg h0,
    ← d2_eq_sqrt, ← d2_eq_sqrt]
  have := le_max_left (d2 o a) (d2 o b); have := le_max_right (d2 o a) (d2 o b)
  nlinarith

/-- An arc move's distance from the axis is at most |c − o| + r. -/
theorem arc_far {o c A : ForgeCam.DistArc.P} {r : ℝ} (hr : 0 ≤ r) (hA : nsq (A - c) = r ^ 2) :
    d2 o A ≤ d2 o c + r := by
  have t := d2_triangle o c A
  have : d2 c A = r := by rw [d2_eq_sqrt, hA, Real.sqrt_sq hr]
  linarith

/-- **An arc move's distance from the axis is at least its point–arc candidate less its ends'
gap** (the candidate with the stored ends, off the circle by at most g, as `pt_sec` computes
it): the fix for shadow.rs's `dmin`. -/
theorem arc_near (o c ea eb : ForgeCam.DistArc.P) {r g : ℝ} (hok : (Piece.arc c ea eb r g).ok)
    {A : ForgeCam.DistArc.P} (hA : A ∈ DistArc.arc c (ea - c) (eb - c) r) :
    √(ptArcSq o c (ea - c) (eb - c) r) - g ≤ d2 o A := by
  have hr : 0 < r := hok.1
  have hg : 0 ≤ g := (abs_nonneg _).trans hok.2.2.2.1
  obtain ⟨hka, hkb, hu, hv, hab, hA', hB', harc⟩ := onCircle_data hok
  have h1 := sqrt_le_of_le_sqg (ptArcSq_nonneg _ _ _ _ _) hg
    (ptArcSq_shift o c (ea - c) (eb - c) (r := r) hka hkb hg hA' hB')
  have hA2 := hA; rw [← harc] at hA2
  have h2 := Real.sqrt_le_sqrt (ptArcSq_le o c _ _ hr hu hv hab hA2)
  rw [show √(nsq (o - A)) = d2 A o from dist_sqrt o A, d2_comm A o] at h2
  linarith

/-! ### S3b: the torus about a vertical axis

The torus's point is o + (R + r cos φ)(c x + s y) + r sin φ ax with φ = φm + ψ. With ax vertical
and x, y horizontal, its horizontal part is the cone's with the radius rr = R + r cos φ
(`cone_radius`, `cone_factor`, `axis_gap` hold for any rr), and its height o.z + ax.z r sin φ:
both functions of ψ alone, so shadow.rs bounds them over a cell of ψ's parameter for the whole
turn of the azimuth. What is new is the enclosure of (cos φ, sin φ): the rational map gives a
unit pair (`circ_unit`), turning by φm keeps it unit (`turn_unit`), and each coordinate of a unit
pair lies in [−1, 1] (`unit_coord`), so shadow.rs's `tube_angle` may meet its enclosures with
[−1, 1]. -/

/-- The torus's horizontal radius is the cone's with rr = R + r cos φ. -/
theorem torus_split (o x y : ForgeCam.DistArc.P) (R r cf c s : ℝ) :
    √(nsq (o + sc (R + r * cf) (sc c x + sc s y) - o)) = |R + r * cf| * √(nsq (sc c x + sc s y)) :=
  cone_radius o x y (R + r * cf) c s

/-- The rational map of a half-angle tangent is a unit pair (check.rs's `circ`). -/
theorem circ_unit (t : ℝ) :
    ((1 - t ^ 2) / (1 + t ^ 2)) ^ 2 + (2 * t / (1 + t ^ 2)) ^ 2 = 1 := by
  have h : (1 + t ^ 2) ≠ 0 := by positivity
  field_simp
  ring

/-- Turning a unit pair by a unit pair keeps it unit: (cos(φm + ψ), sin(φm + ψ)). -/
theorem turn_unit {cp sp cm sm : ℝ} (h1 : cp ^ 2 + sp ^ 2 = 1) (h2 : cm ^ 2 + sm ^ 2 = 1) :
    (cp * cm - sp * sm) ^ 2 + (cp * sm + sp * cm) ^ 2 = 1 := by
  linear_combination (cm ^ 2 + sm ^ 2) * h1 + h2

/-- Each coordinate of a unit pair lies in [−1, 1]. -/
theorem unit_coord {u v : ℝ} (h : u ^ 2 + v ^ 2 = 1) : -1 ≤ u ∧ u ≤ 1 := by
  constructor <;> nlinarith [sq_nonneg v, sq_nonneg (u - 1), sq_nonneg (u + 1)]

/-! ### S4: the search, and the clearance -/

/-- **S4, the cover.** Leaves covering a domain, each a lower bound on q over itself and each at
least c, make c a lower bound on q everywhere. -/
theorem shadow_cover {α ι : Type*} {D : Set α} {leaf : ι → Set α} {lb : ι → ℝ} {q : α → ℝ}
    {c : ℝ} (hcov : ∀ x ∈ D, ∃ i, x ∈ leaf i) (hlb : ∀ i, ∀ x ∈ leaf i, lb i ≤ q x)
    (hc : ∀ i, c ≤ lb i) : ∀ x ∈ D, c ≤ q x := by
  intro x hx
  obtain ⟨i, hi⟩ := hcov x hx
  exact (hc i).trans (hlb i x hi)

/-- **The clearance from per-face lower bounds, any face kind** (a prism's by `clearFace_sound`, a
plane's or cone's by S1–S4). Each face i bounds the moved body's signed distance below by
lower i ≥ c + μ at every point and instant, and the closed set's frontier is within μ of the faces.
With the body's top above the set, every point of the moved body is at least c from it. -/
theorem certified_clear_gen {ι : Type} {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ} (hζ : HeightsIn ζ Zt)
    {A : Set E3} (hA : IsClosed A) (htop : TopAbove Z w Zt A) {c μ : ℝ} (hc : 0 < c)
    (faces : ι → Set E3) (lower : ι → ℝ)
    (hfr : ∀ f ∈ frontier A, ∃ i, ∃ f' ∈ faces i, dist f f' ≤ μ)
    (hfc : ∀ i, ∀ s ∈ I01, ∀ f ∈ faces i, lower i ≤ sd (translate (tip γ ζ s) (hyp Z w)) f)
    (hlow : ∀ i, c + μ ≤ lower i) :
    ∀ s ∈ I01, ∀ x ∈ move γ ζ s '' hyp Z w, ∀ y ∈ A, c ≤ dist x y := by
  intro s hs x hx y hy
  have hx' : x ∈ translate (tip γ ζ s) (hyp Z w) := hx
  refine clear_of_frontier (F := frontier A) (translate_preconnected (hyp_preconnected hZ hw) _)
    hA subset_rfl hc (fun f hf => ?_) (out_of_topAbove htop γ hζ hs) x hx' y hy
  obtain ⟨i, f', hf', hd⟩ := hfr f hf
  have h1 := hfc i s hs f' hf'
  have h2 := sd_le_sd_add_dist (translate (tip γ ζ s) (hyp Z w)) f' f
  rw [dist_comm] at h2
  linarith [hlow i]

/-- **shadow::clear's Clear, assembled (S1 + S4).** A face given by parameters u ∈ U (its points
φ u), a move's tip at instant s. Leaves cover U × [0, 1]. On each leaf, h i ≥ 0 bounds the
horizontal distance from below (S2 for a plane, S3 for a cone), the heights lie in Zs i, and
lb i ≤ sdRZ(h i, z) over Zs i (the interval `sd_rz` enclosure). Every leaf is ≥ L. Then L bounds
the moved body's signed distance at every face point and instant: the per-face bound
`certified_clear_gen` takes (with L = lower + μ as shadow.rs reports it). -/
theorem shadow_face {Z : Set ℝ} {w : ℝ → ℝ} {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Uα ι : Type*}
    {U : Set Uα} (φ : Uα → E3) {leaf : ι → Set (Uα × ℝ)} {h lb : ι → ℝ} {Zs : ι → Set ℝ}
    {L : ℝ} (hcov : ∀ u ∈ U, ∀ s ∈ I01, ∃ i, (u, s) ∈ leaf i) (hh : ∀ i, 0 ≤ h i)
    (hρ : ∀ i, ∀ x ∈ leaf i, h i ≤ ρ (φ x.1 - tip γ ζ x.2))
    (hz : ∀ i, ∀ x ∈ leaf i, φ x.1 2 - tip γ ζ x.2 2 ∈ Zs i)
    (hlb : ∀ i, ∀ z ∈ Zs i, lb i ≤ sdRZ Z w (h i) z) (hL : ∀ i, L ≤ lb i) :
    ∀ s ∈ I01, ∀ u ∈ U, L ≤ sd (translate (tip γ ζ s) (hyp Z w)) (φ u) := by
  intro s hs u hu
  obtain ⟨i, hi⟩ := hcov u hu s hs
  have h1 : h i ≤ ρ (φ u - tip γ ζ s) := hρ i (u, s) hi
  have h2 : φ u 2 - tip γ ζ s 2 ∈ Zs i := hz i (u, s) hi
  exact (hL i).trans (shadow_point (Z := Z) (w := w) (hh i) h1 h2 (hlb i))

end

end ForgeCam.Shadow
