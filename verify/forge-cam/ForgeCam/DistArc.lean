/-
slab::dist's candidates for a segment against an arc (kernel/forge-cam-geom/src/slab.rs), proven
sound over ℝ: the least of them is a lower bound for every pair of points. In squared distances.

The arc runs counter-clockwise from a to b about c, radius r, less than a half turn
(cross(a − c, b − c) > 0). A clockwise arc from a to b is the counter-clockwise arc from b to a,
as slab.rs's `in_sector` negation says. Its sector is the closed cone between a − c and b − c.

The candidates, as slab.rs computes them (with exact signs):
- each segment end to the arc (`pt_sec`): (|q − c| − r)² if q's direction is in the sector, else
  the nearer arc end;
- each arc end to the segment (`ptSegSq`);
- the common normal through the centre: (h − r)², h = |f − c|, if the foot f of c on the segment's
  line is on the segment and its direction is in the sector;
- 0, if the segment meets the circle at a point whose direction is in the sector.

Arcs need square roots, so this spec is over ℝ (an executable form needs ℚ(√d)).
-/
import ForgeCam.Dist2
import ForgeCam.Prism

namespace ForgeCam.DistArc

open ForgeCam.Dist2

abbrev P := ℝ × ℝ

/-- The direction e (from the centre) is in the counter-clockwise sector from ua to ub. -/
def inSector (ua ub e : P) : Prop := 0 ≤ cross ua e ∧ 0 ≤ cross e ub

/-- The arc: the points on the circle whose direction is in the sector. -/
def arc (c ua ub : P) (r : ℝ) : Set P := {v | nsq (v - c) = r ^ 2 ∧ inSector ua ub (v - c)}

theorem expand_sub (q v c : P) :
    nsq (q - v) = nsq (q - c) + nsq (v - c) - 2 * dot (q - c) (v - c) := by
  simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring

/-- In the plane: |u|² ⟨e, w⟩ = ⟨u, e⟩⟨u, w⟩ + (u × e)(u × w). -/
theorem dot_decomp (u e w : P) :
    nsq u * dot e w = dot u e * dot u w + cross u e * cross u w := by
  simp only [nsq, dot, cross]; ring

theorem dot_le_of_nsq (u w : P) {r : ℝ} (hu : nsq u = r ^ 2) (hw : nsq w = r ^ 2) :
    dot u w ≤ r ^ 2 := by
  have := nsq_nonneg (u - w)
  simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub] at this hu hw ⊢
  nlinarith

/-- **The cone lemma.** For a direction e outside the sector (less than a half turn), every
point of the arc is no nearer to c + e than the nearer arc end:
⟨e, w⟩ ≤ max ⟨e, ua⟩ ⟨e, ub⟩ for every w of length r in the sector. -/
theorem dot_le_max_of_not_inSector {ua ub e w : P} {r : ℝ} (hr : 0 < r)
    (hua : nsq ua = r ^ 2) (hub : nsq ub = r ^ 2) (hab : 0 < cross ua ub)
    (hw : nsq w = r ^ 2) (hws : inSector ua ub w) (he : ¬ inSector ua ub e) :
    dot e w ≤ max (dot e ua) (dot e ub) := by
  obtain ⟨hw1, hw2⟩ := hws
  have hr2 : 0 < r ^ 2 := by positivity
  have duw := dot_le_of_nsq ua w hua hw
  have dvw := dot_le_of_nsq ub w hub hw
  have dab := dot_le_of_nsq ua ub hua hub
  have hdA := dot_decomp ua e w
  have hdB := dot_decomp ub e w
  rw [hua] at hdA; rw [hub] at hdB
  simp only [inSector, not_and_or, not_le] at he
  -- the cases where the bound is an end
  by_cases hA : cross ua e < 0 ∧ 0 ≤ dot ua e
  · -- r² ⟨e,w⟩ = ⟨ua,e⟩⟨ua,w⟩ + (ua×e)(ua×w) ≤ ⟨ua,e⟩ r²
    have : r ^ 2 * dot e w ≤ r ^ 2 * dot e ua := by
      have h1 : dot ua e * dot ua w ≤ dot ua e * r ^ 2 := mul_le_mul_of_nonneg_left duw hA.2
      have h2 : cross ua e * cross ua w ≤ 0 := mul_nonpos_of_nonpos_of_nonneg hA.1.le hw1
      have e1 : dot e ua = dot ua e := by simp only [dot]; ring
      nlinarith
    exact le_max_of_le_left (le_of_mul_le_mul_left this hr2)
  by_cases hB : 0 < cross ub e ∧ 0 ≤ dot ub e
  · have : r ^ 2 * dot e w ≤ r ^ 2 * dot e ub := by
      have h1 : dot ub e * dot ub w ≤ dot ub e * r ^ 2 := mul_le_mul_of_nonneg_left dvw hB.2
      have hc : cross ub w = -cross w ub := by simp only [cross]; ring
      have h2 : cross ub e * cross ub w ≤ 0 := by
        rw [hc]; exact mul_nonpos_of_nonneg_of_nonpos hB.1.le (by linarith)
      have e1 : dot e ub = dot ub e := by simp only [dot]; ring
      nlinarith
    exact le_max_of_le_right (le_of_mul_le_mul_left this hr2)
  -- w = λ ua + μ ub with λ, μ ≥ 0, and λ + μ ≥ 1
  set D := cross ua ub
  set l := cross w ub / D
  set m := cross ua w / D
  have hl : 0 ≤ l := div_nonneg hw2 hab.le
  have hm : 0 ≤ m := div_nonneg hw1 hab.le
  have hw_eq : w = (l * ua.1 + m * ub.1, l * ua.2 + m * ub.2) := by
    have hD : D ≠ 0 := hab.ne'
    have i1 : w.1 * D = cross w ub * ua.1 + cross ua w * ub.1 := by simp only [D, cross]; ring
    have i2 : w.2 * D = cross w ub * ua.2 + cross ua w * ub.2 := by simp only [D, cross]; ring
    ext
    · simp only [l, m]; field_simp; linarith
    · simp only [l, m]; field_simp; linarith
  have hdot : dot e w = l * dot e ua + m * dot e ub := by
    rw [hw_eq]; simp only [dot]; ring
  have hlm : 1 ≤ l + m := by
    have hn : nsq w = l ^ 2 * r ^ 2 + m ^ 2 * r ^ 2 + 2 * l * m * dot ua ub := by
      rw [hw_eq]; simp only [nsq, dot] at hua hub ⊢; nlinarith [hua, hub]
    have : r ^ 2 ≤ (l + m) ^ 2 * r ^ 2 := by
      rw [hw] at hn; nlinarith [mul_nonneg hl hm]
    have h1 : 1 ≤ (l + m) ^ 2 := le_of_mul_le_mul_right (by linarith) hr2
    nlinarith [sq_nonneg (l + m - 1)]
  -- both ends' dots ≤ 0: then λx + μy ≤ (λ+μ) max ≤ max
  by_cases hC : dot e ua ≤ 0 ∧ dot e ub ≤ 0
  · rw [hdot]
    have hmx : max (dot e ua) (dot e ub) ≤ 0 := max_le hC.1 hC.2
    have := le_max_left (dot e ua) (dot e ub)
    have := le_max_right (dot e ua) (dot e ub)
    nlinarith
  -- the rest cannot happen
  exfalso
  have eA : dot e ua = dot ua e := by simp only [dot]; ring
  have eB : dot e ub = dot ub e := by simp only [dot]; ring
  -- the plane identity: |ua|² (ub × e) = ⟨ub,ua⟩(ua × e) − (ua × ub)⟨ua, e⟩
  have id1 : r ^ 2 * cross ub e = dot ub ua * cross ua e - D * dot ua e := by
    rw [← hua]; simp only [nsq, dot, cross, D]; ring
  have id2 : r ^ 2 * dot ub e = dot ua ub * dot ua e + D * cross ua e := by
    rw [← hua]; simp only [nsq, dot, cross, D]; ring
  have id3 : r ^ 2 * cross ua e = dot ua ub * cross ub e + D * dot ub e := by
    rw [← hub]; simp only [nsq, dot, cross, D]; ring
  have id4 : r ^ 2 * dot ua e = dot ua ub * dot ub e - D * cross ub e := by
    rw [← hub]; simp only [nsq, dot, cross, D]; ring
  have dsym : dot ub ua = dot ua ub := by simp only [dot]; ring
  rw [dsym] at id1
  push Not at hA hB hC
  rcases he with h1 | h2
  · -- ua × e < 0: then ⟨ua,e⟩ < 0, so ⟨ub,e⟩ > 0, so ub × e ≤ 0: contradiction
    have p1 : dot ua e < 0 := hA h1
    have p2 : 0 < dot ub e := by
      rcases le_or_gt (dot e ua) 0 with h | h
      · have := hC h; rw [eB] at this; exact this
      · rw [eA] at h; linarith
    have p3 : cross ub e ≤ 0 := by
      by_contra h; push Not at h; linarith [hB h]
    -- from id2: r²⟨ub,e⟩ = ⟨ua,ub⟩⟨ua,e⟩ + D (ua×e) > 0 forces ⟨ua,ub⟩ < 0
    have q1 : dot ua ub < 0 := by
      by_contra h; push Not at h
      have := mul_nonpos_of_nonneg_of_nonpos h p1.le
      have := mul_neg_of_pos_of_neg hab h1
      have := mul_pos hr2 p2
      linarith [id2]
    have := mul_pos_of_neg_of_neg q1 h1
    have := mul_neg_of_pos_of_neg hab p1
    have := mul_nonpos_of_nonneg_of_nonpos hr2.le p3
    linarith [id1]
  · -- e × ub < 0, i.e. ub × e > 0: symmetric
    have h2' : 0 < cross ub e := by simp only [cross] at h2 ⊢; linarith
    have p1 : dot ub e < 0 := hB h2'
    have p2 : 0 < dot ua e := by
      rcases le_or_gt (dot e ub) 0 with h | h
      · rcases le_or_gt (dot e ua) 0 with h' | h'
        · exact absurd (hC h') (by rw [eB]; linarith)
        · rw [eA] at h'; exact h'
      · rw [eB] at h; linarith
    have p3 : 0 ≤ cross ua e := by
      by_contra h; push Not at h; linarith [hA h]
    have q1 : dot ua ub < 0 := by
      by_contra h; push Not at h
      have := mul_nonpos_of_nonneg_of_nonpos h p1.le
      have := mul_pos hab h2'
      have := mul_pos hr2 p2
      linarith [id4]
    have := mul_neg_of_neg_of_pos q1 h2'
    have := mul_neg_of_pos_of_neg hab p1
    have := mul_nonneg hr2.le p3
    linarith [id3]

theorem nsq_eq_zero {z : P} (h : nsq z = 0) : z = 0 := by
  simp only [nsq, dot] at h
  have h1 : z.1 = 0 := by nlinarith [mul_self_nonneg z.1, mul_self_nonneg z.2]
  have h2 : z.2 = 0 := by nlinarith [mul_self_nonneg z.1, mul_self_nonneg z.2]
  exact Prod.ext h1 h2

theorem lagrange2 (u x : P) : dot u x ^ 2 + cross u x ^ 2 = nsq u * nsq x := by
  simp only [nsq, dot, cross]; ring

/-- Cauchy–Schwarz: ⟨e, x⟩ ≤ |e| |x|, for |x| = r ≥ 0. -/
theorem dot_le_sqrt (e x : P) {r : ℝ} (hr : 0 ≤ r) (hx : nsq x = r ^ 2) :
    dot e x ≤ √(nsq e) * r := by
  have h := lagrange2 e x
  rw [hx] at h
  have h2 : dot e x ^ 2 ≤ (√(nsq e) * r) ^ 2 := by
    rw [mul_pow, Real.sq_sqrt (nsq_nonneg e)]; nlinarith [sq_nonneg (cross e x)]
  have h0 : 0 ≤ √(nsq e) * r := mul_nonneg (Real.sqrt_nonneg _) hr
  nlinarith [sq_nonneg (dot e x - √(nsq e) * r), sq_nonneg (dot e x + √(nsq e) * r)]

/-- Every point of the circle is at least ||q − c| − r| from q. -/
theorem circle_lower (q v c : P) {r : ℝ} (hr : 0 ≤ r) (hv : nsq (v - c) = r ^ 2) :
    (√(nsq (q - c)) - r) ^ 2 ≤ nsq (q - v) := by
  rw [expand_sub q v c, hv]
  have := dot_le_sqrt (q - c) (v - c) hr hv
  have hs : (√(nsq (q - c))) ^ 2 = nsq (q - c) := Real.sq_sqrt (nsq_nonneg _)
  nlinarith

/-- slab.rs's `pt_sec` for an arc, squared and with exact signs. -/
noncomputable def ptArcSq (q c ua ub : P) (r : ℝ) : ℝ :=
  open Classical in
  if inSector ua ub (q - c) then (√(nsq (q - c)) - r) ^ 2
  else min (nsq (q - (c + ua))) (nsq (q - (c + ub)))

/-- **The point–arc candidate is a lower bound** for every point of the arc. -/
theorem ptArcSq_le (q c ua ub : P) {r : ℝ} (hr : 0 < r) (hua : nsq ua = r ^ 2)
    (hub : nsq ub = r ^ 2) (hab : 0 < cross ua ub) {v : P} (hv : v ∈ arc c ua ub r) :
    ptArcSq q c ua ub r ≤ nsq (q - v) := by
  unfold ptArcSq
  split_ifs with hs
  · exact circle_lower q v c hr.le hv.1
  · have hc := dot_le_max_of_not_inSector hr hua hub hab hv.1 hv.2 hs
    have ea : nsq (q - (c + ua)) = nsq (q - c) + r ^ 2 - 2 * dot (q - c) ua := by
      rw [← hua]; simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub, Prod.fst_add, Prod.snd_add]; ring
    have eb : nsq (q - (c + ub)) = nsq (q - c) + r ^ 2 - 2 * dot (q - c) ub := by
      rw [← hub]; simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub, Prod.fst_add, Prod.snd_add]; ring
    rw [expand_sub q v c, hv.1, ea, eb]
    rcases le_total (dot (q - c) ua) (dot (q - c) ub) with h | h
    · rw [max_eq_right h] at hc
      exact (min_le_right _ _).trans (by linarith)
    · rw [max_eq_left h] at hc
      exact (min_le_left _ _).trans (by linarith)

/-- An arc point on the sector's first edge is the arc's first end; on its last edge, its last. -/
theorem eq_start_of_cross {c ua ub v : P} {r : ℝ} (_hr : 0 < r) (hua : nsq ua = r ^ 2)
    (hab : 0 < cross ua ub) (hv : v ∈ arc c ua ub r) (h0 : cross ua (v - c) = 0) :
    v = c + ua := by
  set x := v - c
  have hl := lagrange2 ua x
  rw [h0, hua, hv.1] at hl
  have hd : dot ua x = r ^ 2 ∨ dot ua x = -r ^ 2 := by
    have : (dot ua x - r ^ 2) * (dot ua x + r ^ 2) = 0 := by nlinarith
    rcases mul_eq_zero.mp this with h | h
    · exact Or.inl (by linarith)
    · exact Or.inr (by linarith)
  rcases hd with hd | hd
  · have : nsq (x - ua) = 0 := by
      have e : nsq (x - ua) = nsq x + nsq ua - 2 * dot ua x := by
        simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
      rw [e, hv.1, hua, hd]; ring
    have := nsq_eq_zero this
    rw [sub_eq_zero] at this
    rw [← this]; simp [x]
  · exfalso
    have : nsq (x + ua) = 0 := by
      have e : nsq (x + ua) = nsq x + nsq ua + 2 * dot ua x := by
        simp only [nsq, dot, Prod.fst_add, Prod.snd_add]; ring
      rw [e, hv.1, hua, hd]; ring
    have hx : x = -ua := eq_neg_of_add_eq_zero_left (nsq_eq_zero this)
    have := hv.2.2
    rw [show v - c = x from rfl, hx] at this
    simp only [cross, Prod.fst_neg, Prod.snd_neg] at this hab
    linarith

theorem eq_end_of_cross {c ua ub v : P} {r : ℝ} (_hr : 0 < r) (hub : nsq ub = r ^ 2)
    (hab : 0 < cross ua ub) (hv : v ∈ arc c ua ub r) (h0 : cross (v - c) ub = 0) :
    v = c + ub := by
  set x := v - c
  have hl := lagrange2 ub x
  have hc : cross ub x = 0 := by simp only [cross] at h0 ⊢; linarith
  rw [hc, hub, hv.1] at hl
  have hd : dot ub x = r ^ 2 ∨ dot ub x = -r ^ 2 := by
    have : (dot ub x - r ^ 2) * (dot ub x + r ^ 2) = 0 := by nlinarith
    rcases mul_eq_zero.mp this with h | h
    · exact Or.inl (by linarith)
    · exact Or.inr (by linarith)
  rcases hd with hd | hd
  · have : nsq (x - ub) = 0 := by
      have e : nsq (x - ub) = nsq x + nsq ub - 2 * dot ub x := by
        simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
      rw [e, hv.1, hub, hd]; ring
    have := nsq_eq_zero this
    rw [sub_eq_zero] at this
    rw [← this]; simp [x]
  · exfalso
    have : nsq (x + ub) = 0 := by
      have e : nsq (x + ub) = nsq x + nsq ub + 2 * dot ub x := by
        simp only [nsq, dot, Prod.fst_add, Prod.snd_add]; ring
      rw [e, hv.1, hub, hd]; ring
    have hx : x = -ub := eq_neg_of_add_eq_zero_left (nsq_eq_zero this)
    have := hv.2.1
    rw [show v - c = x from rfl, hx] at this
    simp only [cross, Prod.fst_neg, Prod.snd_neg] at this hab
    linarith

/-- The rotation of x by the rational parameter t (an angle 2·atan t). -/
noncomputable def rot (t : ℝ) (x : P) : P :=
  (((1 - t ^ 2) * x.1 - 2 * t * x.2) / (1 + t ^ 2), ((1 - t ^ 2) * x.2 + 2 * t * x.1) / (1 + t ^ 2))

theorem nsq_rot (t : ℝ) (x : P) : nsq (rot t x) = nsq x := by
  have h : (0:ℝ) < 1 + t ^ 2 := by positivity
  simp only [nsq, dot, rot]; field_simp; ring

theorem cross_rot_left (u x : P) (t : ℝ) :
    cross u (rot t x) = ((1 - t ^ 2) * cross u x + 2 * t * dot u x) / (1 + t ^ 2) := by
  have h : (0:ℝ) < 1 + t ^ 2 := by positivity
  simp only [cross, dot, rot]; field_simp; ring

theorem cross_rot_right (x u : P) (t : ℝ) :
    cross (rot t x) u = ((1 - t ^ 2) * cross x u - 2 * t * dot x u) / (1 + t ^ 2) := by
  have h : (0:ℝ) < 1 + t ^ 2 := by positivity
  simp only [cross, dot, rot]; field_simp; ring

theorem dot_rot (e x : P) (t : ℝ) :
    dot e (rot t x) = ((1 - t ^ 2) * dot e x + 2 * t * cross x e) / (1 + t ^ 2) := by
  have h : (0:ℝ) < 1 + t ^ 2 := by positivity
  simp only [cross, dot, rot]; field_simp; ring

/-- A positive quantity stays positive under a small rotation. -/
theorem pos_small {α β t : ℝ} (hα : 0 < α) (ht : |t| ≤ min (1 / 2) (α / (4 * (|β| + 1)))) :
    0 < (1 - t ^ 2) * α + 2 * t * β := by
  have h1 : |t| ≤ 1 / 2 := ht.trans (min_le_left _ _)
  have h2 : |t| ≤ α / (4 * (|β| + 1)) := ht.trans (min_le_right _ _)
  have ht2 : t ^ 2 ≤ 1 / 4 := by
    rw [← sq_abs]; nlinarith [abs_nonneg t]
  have hb : |2 * t * β| ≤ α / 2 := by
    rw [abs_mul, abs_mul, abs_two]
    have hβ : 0 ≤ |β| := abs_nonneg β
    have : |t| * (4 * (|β| + 1)) ≤ α := by
      rwa [le_div_iff₀ (by positivity)] at h2
    nlinarith [abs_nonneg t]
  have := neg_abs_le (2 * t * β)
  nlinarith

/-- **First order on the arc**: if an arc point strictly inside the sector is the arc's nearest
to q, then q − c and v − c are parallel. -/
theorem cross_eq_zero_of_arc_min {c ua ub q v : P} {r : ℝ} (hv : v ∈ arc c ua ub r)
    (h1 : 0 < cross ua (v - c)) (h2 : 0 < cross (v - c) ub)
    (hmin : ∀ v' ∈ arc c ua ub r, nsq (q - v) ≤ nsq (q - v')) :
    cross (v - c) (q - c) = 0 := by
  set x := v - c
  set e := q - c
  set A := dot e x
  set B := cross x e
  by_contra hB
  set δ := min (min (1 / 2) (cross ua x / (4 * (|dot ua x| + 1))))
    (min (min (1 / 2) (cross x ub / (4 * (|dot x ub| + 1)))) (|B| / (2 * (|A| + 1))))
  have hBpos : 0 < |B| := abs_pos.mpr hB
  have hδ : 0 < δ := by
    simp only [δ]
    refine lt_min (lt_min (by norm_num) (by positivity)) (lt_min (lt_min (by norm_num)
      (by positivity)) (by positivity))
  set t := if 0 < B then δ else -δ
  have hta : |t| = δ := by
    simp only [t]; split_ifs <;> simp [abs_of_pos hδ]
  -- the rotated point is on the arc
  have hv' : c + rot t x ∈ arc c ua ub r := by
    have hx : nsq x = r ^ 2 := hv.1
    refine ⟨by simp [nsq_rot, hx], ?_, ?_⟩
    · simp only [add_sub_cancel_left]
      rw [cross_rot_left]
      exact (div_pos (pos_small h1 (by rw [hta]; exact min_le_left _ _)) (by positivity)).le
    · simp only [add_sub_cancel_left]
      rw [cross_rot_right]
      have := pos_small (β := -dot x ub) h2 (t := t) (by
        rw [hta, abs_neg]; exact (min_le_right _ _).trans (min_le_left _ _))
      exact (div_pos (by linarith) (by positivity)).le
  have hm := hmin _ hv'
  -- nsq(q − v) = |e|² + r² − 2A; nsq(q − v') = |e|² + r² − 2⟨e, rot t x⟩
  have e1 : nsq (q - v) = nsq e + nsq x - 2 * A := by
    simp only [e, x, A]; rw [expand_sub q v c]
  have e2 : nsq (q - (c + rot t x)) = nsq e + nsq x - 2 * dot e (rot t x) := by
    have := expand_sub q (c + rot t x) c
    simp only [add_sub_cancel_left] at this; rw [this, nsq_rot]
  rw [e1, e2, dot_rot] at hm
  have hpos : (0:ℝ) < 1 + t ^ 2 := by positivity
  have key : t * B ≤ t ^ 2 * A := by
    have : ((1 - t ^ 2) * A + 2 * t * B) / (1 + t ^ 2) ≤ A := by linarith
    rw [div_le_iff₀ hpos] at this; nlinarith
  -- t B = δ |B| > δ² A: contradiction with δ ≤ |B| / (2(|A| + 1))
  have htB : t * B = δ * |B| := by
    simp only [t]; split_ifs with h
    · rw [abs_of_pos h]
    · push Not at h
      rw [abs_of_nonpos h]; ring
  have ht2 : t ^ 2 = δ ^ 2 := by rw [← sq_abs, hta]
  rw [htB, ht2] at key
  have hδB : δ ≤ |B| / (2 * (|A| + 1)) := (min_le_right _ _).trans (min_le_right _ _)
  rw [le_div_iff₀ (by positivity)] at hδB
  have hA : A ≤ |A| := le_abs_self A
  nlinarith [abs_nonneg A]

/-! ### Segment against arc -/

/-- The foot parameter of c on the line a + τ·d. -/
noncomputable def footT (a d c : P) : ℝ := dot (c - a) d / nsq d

/-- The candidates without the crossing and the normal. -/
noncomputable def segArcBase (a d c ua ub : P) (r : ℝ) : ℝ :=
  min (min (ptArcSq a c ua ub r) (ptArcSq (a + d) c ua ub r))
    (min (ptSegSq (c + ua) a d) (ptSegSq (c + ub) a d))

/-- **slab::dist's low end for a segment against an arc, squared, exact signs**: 0 if the
segment meets the arc; else the least of the ends' candidates and, if the centre's foot is on the
segment and in the sector, the common normal's (h − r)². -/
noncomputable def segArcLo (a d c ua ub : P) (r : ℝ) : ℝ :=
  open Classical in
  if ∃ τ, 0 ≤ τ ∧ τ ≤ 1 ∧ nsq (pt a d τ - c) = r ^ 2 ∧ inSector ua ub (pt a d τ - c) then 0
  else if 0 ≤ footT a d c ∧ footT a d c ≤ 1 ∧ inSector ua ub (pt a d (footT a d c) - c) then
    min (segArcBase a d c ua ub r) ((√(nsq (pt a d (footT a d c) - c)) - r) ^ 2)
  else segArcBase a d c ua ub r

theorem segArcLo_le_of_base {a d c ua ub : P} {r y : ℝ} (h : segArcBase a d c ua ub r ≤ y)
    (hy : 0 ≤ y) : segArcLo a d c ua ub r ≤ y := by
  unfold segArcLo; split_ifs
  · exact hy
  · exact (min_le_left _ _).trans h
  · exact h

theorem isCompact_arc (c ua ub : P) (r : ℝ) : IsCompact (arc c ua ub r) := by
  have hcl : IsClosed (arc c ua ub r) := by
    have h1 : IsClosed {v : P | nsq (v - c) = r ^ 2} :=
      isClosed_eq (by unfold nsq dot; fun_prop) continuous_const
    have h2 : IsClosed {v : P | 0 ≤ cross ua (v - c)} :=
      isClosed_le continuous_const (by unfold cross; fun_prop)
    have h3 : IsClosed {v : P | 0 ≤ cross (v - c) ub} :=
      isClosed_le continuous_const (by unfold cross; fun_prop)
    have : arc c ua ub r = {v : P | nsq (v - c) = r ^ 2} ∩
        ({v : P | 0 ≤ cross ua (v - c)} ∩ {v | 0 ≤ cross (v - c) ub}) := by
      ext v; simp [arc, inSector]
    rw [this]; exact h1.inter (h2.inter h3)
  refine (isCompact_Icc (a := c - (|r|, |r|)) (b := c + (|r|, |r|))).of_isClosed_subset hcl ?_
  intro v hv
  have h := hv.1
  simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub] at h
  have b1 : |v.1 - c.1| ≤ |r| := sq_le_sq.mp (by nlinarith [sq_nonneg (v.2 - c.2), sq_abs r])
  have b2 : |v.2 - c.2| ≤ |r| := sq_le_sq.mp (by nlinarith [sq_nonneg (v.1 - c.1), sq_abs r])
  obtain ⟨l1, u1⟩ := abs_le.mp b1
  obtain ⟨l2, u2⟩ := abs_le.mp b2
  exact ⟨⟨by simp; linarith, by simp; linarith⟩, ⟨by simp; linarith, by simp; linarith⟩⟩

theorem pt_zero (a d : P) : pt a d 0 = a := by simp [pt]
theorem pt_one (a d : P) : pt a d 1 = a + d := by
  ext <;> simp [pt]

/-- **slab::dist's low end for a segment against an arc is a lower bound** for the squared
distance of every pair of points, one on the segment, one on the arc. -/
theorem segArcLo_le (a d c ua ub : P) {r : ℝ} (hd : 0 < nsq d) (hr : 0 < r)
    (hua : nsq ua = r ^ 2) (hub : nsq ub = r ^ 2) (hab : 0 < cross ua ub)
    {τ : ℝ} (hτ0 : 0 ≤ τ) (hτ1 : τ ≤ 1) {v : P} (hv : v ∈ arc c ua ub r) :
    segArcLo a d c ua ub r ≤ nsq (pt a d τ - v) := by
  -- a least pair
  have hK : IsCompact (Set.Icc (0:ℝ) 1 ×ˢ arc c ua ub r) := isCompact_Icc.prod (isCompact_arc _ _ _ _)
  have hne : (Set.Icc (0:ℝ) 1 ×ˢ arc c ua ub r).Nonempty :=
    ⟨(0, c + ua), ⟨le_rfl, zero_le_one⟩, by simp [hua], by
      rw [add_sub_cancel_left]; exact le_of_eq (by simp only [cross]; ring), by
      rw [add_sub_cancel_left]; exact hab.le⟩
  have hcont : Continuous fun p : ℝ × P => nsq (pt a d p.1 - p.2) := by
    unfold nsq dot pt; fun_prop
  obtain ⟨⟨τ₀, v₀⟩, ⟨⟨h00, h01⟩, hv₀⟩, hmin⟩ := hK.exists_isMinOn hne hcont.continuousOn
  simp only at h00 h01 hv₀
  have hmin' : ∀ p ∈ Set.Icc (0:ℝ) 1 ×ˢ arc c ua ub r,
      nsq (pt a d τ₀ - v₀) ≤ nsq (pt a d p.1 - p.2) := isMinOn_iff.mp hmin
  refine le_trans ?_ (hmin' (τ, v) ⟨⟨hτ0, hτ1⟩, hv⟩)
  have hy : 0 ≤ nsq (pt a d τ₀ - v₀) := nsq_nonneg _
  -- an end of the segment
  rcases h00.eq_or_lt with h | hτp
  · refine segArcLo_le_of_base ?_ hy
    rw [← h, pt_zero]
    exact (min_le_left _ _).trans ((min_le_left _ _).trans (ptArcSq_le a c ua ub hr hua hub hab hv₀))
  rcases h01.eq_or_lt with h | hτq
  · refine segArcLo_le_of_base ?_ hy
    rw [h, pt_one]
    exact (min_le_left _ _).trans ((min_le_right _ _).trans
      (ptArcSq_le (a + d) c ua ub hr hua hub hab hv₀))
  -- an end of the arc
  rcases hv₀.2.1.eq_or_lt with h | hs1
  · refine segArcLo_le_of_base ?_ hy
    rw [eq_start_of_cross hr hua hab hv₀ h.symm, Dist2.nsq_comm]
    exact (min_le_right _ _).trans ((min_le_left _ _).trans (ptSegSq_le _ a d hd h00 h01))
  rcases hv₀.2.2.eq_or_lt with h | hs2
  · refine segArcLo_le_of_base ?_ hy
    rw [eq_end_of_cross hr hub hab hv₀ h.symm, Dist2.nsq_comm]
    exact (min_le_right _ _).trans ((min_le_right _ _).trans (ptSegSq_le _ a d hd h00 h01))
  -- both inside
  set u := pt a d τ₀ with hu
  set x := v₀ - c with hx
  set e := u - c with he
  by_cases hsec : inSector ua ub e
  swap
  · -- the segment point's direction is outside the sector: an arc end is nearest
    refine segArcLo_le_of_base ?_ hy
    have h1 := ptArcSq_le u c ua ub hr hua hub hab hv₀
    unfold ptArcSq at h1
    rw [ite_eq_right_of_eq_false _ _ (eq_false (show ¬ inSector ua ub (u - c) from hsec))] at h1
    refine le_trans ?_ h1
    have ha := ptSegSq_le (c + ua) a d hd h00 h01
    have hb := ptSegSq_le (c + ub) a d hd h00 h01
    rw [Dist2.nsq_comm] at ha hb
    exact (min_le_right _ _).trans (min_le_min ha hb)
  -- first order on the segment: w ⊥ d
  have hwd : dot (u - v₀) d = 0 := by
    refine Dist2.dot_eq_zero_of_min hτp hτq fun ε h1 h2 => ?_
    have := hmin' (τ₀ + ε, v₀) ⟨⟨h1, h2⟩, hv₀⟩
    convert this using 2
    simp only [u, pt]; ext <;> simp <;> ring
  -- first order on the arc: x ∥ e
  have hxe : cross x e = 0 :=
    cross_eq_zero_of_arc_min hv₀ hs1 hs2 fun v' hv' => hmin' (τ₀, v') ⟨⟨h00, h01⟩, hv'⟩
  have hxr : nsq x = r ^ 2 := hv₀.1
  have hed : nsq x * dot e d = dot x e * dot x d := by
    rw [dot_decomp x e d, hxe, zero_mul, add_zero]
  have hwd' : dot e d = dot x d := by
    have : u - v₀ = e - x := by simp [e, x]
    rw [this] at hwd; simp only [dot, Prod.fst_sub, Prod.snd_sub] at hwd ⊢; linarith
  by_cases hxd : dot x d = 0
  · -- u is the centre's foot: the normal candidate
    have hed0 : dot e d = 0 := by rw [hwd', hxd]
    have hfoot : footT a d c = τ₀ := by
      have : dot e d = dot (a - c) d + τ₀ * nsq d := by
        simp only [e, u, pt, dot, nsq, Prod.fst_sub, Prod.snd_sub]; ring
      unfold footT
      rw [div_eq_iff hd.ne']
      have hca : dot (c - a) d = -dot (a - c) d := by
        simp only [dot, Prod.fst_sub, Prod.snd_sub]; ring
      linarith
    unfold segArcLo
    split_ifs with hc hf
    · exact hy
    · rw [hfoot]
      exact (min_le_right _ _).trans (circle_lower u v₀ c hr.le hv₀.1)
    · exfalso
      apply ‹¬(0 ≤ footT a d c ∧ footT a d c ≤ 1 ∧ inSector ua ub (pt a d (footT a d c) - c))›
      rw [hfoot]; exact ⟨h00, h01, hsec⟩
  · -- u is on the arc: they meet
    have hde : dot x e = r ^ 2 := by
      have := hed; rw [hwd', hxr] at this
      exact (mul_right_cancel₀ hxd this).symm
    have hne2 : nsq e = r ^ 2 := by
      have hl := lagrange2 x e
      rw [hde, hxe, hxr] at hl
      have hr2 : 0 < r ^ 2 := by positivity
      nlinarith
    unfold segArcLo
    split_ifs with hc <;> first | exact hy | exact absurd ⟨τ₀, h00, h01, hne2, hsec⟩ hc

/-! ### Arc against arc -/

/-- k·v. -/
def sc (k : ℝ) (v : P) : P := (k * v.1, k * v.2)

/-- The end candidates: each arc's ends to the other arc. -/
noncomputable def arcArcBase (c1 ua1 ub1 : P) (r1 : ℝ) (c2 ua2 ub2 : P) (r2 : ℝ) : ℝ :=
  min (min (ptArcSq (c1 + ua1) c2 ua2 ub2 r2) (ptArcSq (c1 + ub1) c2 ua2 ub2 r2))
    (min (ptArcSq (c2 + ua2) c1 ua1 ub1 r1) (ptArcSq (c2 + ub2) c1 ua1 ub1 r1))

/-- A line-of-centres candidate: the points c1 + s1 r1 û and c2 + s2 r2 û (û along c2 − c1),
at distance |d + s2 r2 − s1 r1|, if both directions are in their sectors (else no candidate:
the base). -/
noncomputable def lineCand (c1 ua1 ub1 : P) (r1 : ℝ) (c2 ua2 ub2 : P) (r2 : ℝ) (s1 s2 : ℝ) :
    ℝ :=
  open Classical in
  if inSector ua1 ub1 (sc s1 (c2 - c1)) ∧ inSector ua2 ub2 (sc s2 (c2 - c1)) then
    (√(nsq (c2 - c1)) + s2 * r2 - s1 * r1) ^ 2
  else arcArcBase c1 ua1 ub1 r1 c2 ua2 ub2 r2

/-- **slab::dist's low end for two arcs, squared, exact signs.** Concentric: the end candidates
and (r1 − r2)². Otherwise 0 if they meet, else the end candidates and the four line-of-centres
candidates. -/
noncomputable def arcArcLo (c1 ua1 ub1 : P) (r1 : ℝ) (c2 ua2 ub2 : P) (r2 : ℝ) : ℝ :=
  open Classical in
  if c1 = c2 then min (arcArcBase c1 ua1 ub1 r1 c2 ua2 ub2 r2) ((r1 - r2) ^ 2)
  else if ∃ v, v ∈ arc c1 ua1 ub1 r1 ∧ v ∈ arc c2 ua2 ub2 r2 then 0
  else min (arcArcBase c1 ua1 ub1 r1 c2 ua2 ub2 r2)
    (min (min (lineCand c1 ua1 ub1 r1 c2 ua2 ub2 r2 1 1) (lineCand c1 ua1 ub1 r1 c2 ua2 ub2 r2 (-1) 1))
      (min (lineCand c1 ua1 ub1 r1 c2 ua2 ub2 r2 1 (-1))
        (lineCand c1 ua1 ub1 r1 c2 ua2 ub2 r2 (-1) (-1))))

theorem arcArcLo_le_of_base {c1 ua1 ub1 c2 ua2 ub2 : P} {r1 r2 y : ℝ}
    (h : arcArcBase c1 ua1 ub1 r1 c2 ua2 ub2 r2 ≤ y) (hy : 0 ≤ y) :
    arcArcLo c1 ua1 ub1 r1 c2 ua2 ub2 r2 ≤ y := by
  unfold arcArcLo; split_ifs
  · exact (min_le_left _ _).trans h
  · exact hy
  · exact (min_le_left _ _).trans h

/-- In the plane: (a × b)|w|² = (a × w)⟨b, w⟩ − (b × w)⟨a, w⟩. -/
theorem cross_mul_nsq (a b w : P) :
    cross a b * nsq w = cross a w * dot b w - cross b w * dot a w := by
  simp only [cross, dot, nsq]; ring

theorem inSector_sc {ua ub v : P} {k : ℝ} (hk : 0 < k) (h : inSector ua ub (sc k v)) :
    inSector ua ub v := by
  obtain ⟨h1, h2⟩ := h
  have e1 : cross ua (sc k v) = k * cross ua v := by simp only [cross, sc]; ring
  have e2 : cross (sc k v) ub = k * cross v ub := by simp only [cross, sc]; ring
  rw [e1] at h1; rw [e2] at h2
  exact ⟨nonneg_of_mul_nonneg_right (by linarith) hk, nonneg_of_mul_nonneg_right (by linarith) hk⟩

theorem inSector_sc' {ua ub v : P} {k : ℝ} (hk : 0 < k) (h : inSector ua ub v) :
    inSector ua ub (sc k v) := by
  obtain ⟨h1, h2⟩ := h
  have e1 : cross ua (sc k v) = k * cross ua v := by simp only [cross, sc]; ring
  have e2 : cross (sc k v) ub = k * cross v ub := by simp only [cross, sc]; ring
  show 0 ≤ cross ua (sc k v) ∧ 0 ≤ cross (sc k v) ub
  rw [e1, e2]; exact ⟨mul_nonneg hk.le h1, mul_nonneg hk.le h2⟩

/-- A vector of length r parallel to a nonzero dv is ± (r / |dv|) dv. -/
theorem parallel_eq {x dv : P} {r : ℝ} (_hr : 0 < r) (hx : nsq x = r ^ 2) (hdv : 0 < nsq dv)
    (hc : cross x dv = 0) :
    ∃ s : ℝ, (s = 1 ∨ s = -1) ∧ x = sc (s * r / √(nsq dv)) dv := by
  set L := √(nsq dv) with hL
  have hL0 : 0 < L := Real.sqrt_pos.mpr hdv
  have hL2 : L ^ 2 = nsq dv := Real.sq_sqrt hdv.le
  have hl := lagrange2 x dv
  rw [hc, hx] at hl
  have hd2 : dot x dv ^ 2 = (r * L) ^ 2 := by rw [mul_pow, hL2]; linarith
  -- nsq dv · x = ⟨x, dv⟩ dv (x × dv = 0)
  have e1 : x.1 * nsq dv = dot x dv * dv.1 := by
    simp only [cross] at hc; simp only [nsq, dot]; linear_combination dv.2 * hc
  have e2 : x.2 * nsq dv = dot x dv * dv.2 := by
    simp only [cross] at hc; simp only [nsq, dot]; linear_combination (-dv.1) * hc
  rcases sq_eq_sq_iff_eq_or_eq_neg.mp hd2 with h | h
  · rw [← hL2, h] at e1 e2
    refine ⟨1, Or.inl rfl, ?_⟩
    ext <;> simp only [sc] <;> field_simp <;> apply mul_right_cancel₀ hL0.ne'
    · linear_combination e1
    · linear_combination e2
  · rw [← hL2, h] at e1 e2
    refine ⟨-1, Or.inr rfl, ?_⟩
    ext <;> simp only [sc] <;> field_simp <;> apply mul_right_cancel₀ hL0.ne'
    · linear_combination e1
    · linear_combination e2

/-! ### Into the checker: `ClearFace` (i) for a segment move against a wall over an arc -/

theorem d2_eq_sqrt (σ q : P) : d2 σ q = √(nsq (q - σ)) := by
  simp only [d2, nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring_nf

/-- **For a segment move against a wall over an arc, D = √segArcLo is what `ClearFace` (i)
asks for.** -/
theorem segArc_D (a d c ua ub : P) {r : ℝ} (hd : 0 < nsq d) (hr : 0 < r)
    (hua : nsq ua = r ^ 2) (hub : nsq ub = r ^ 2) (hab : 0 < cross ua ub) :
    0 ≤ √(segArcLo a d c ua ub r) ∧
      ∀ s ∈ I01, ∀ σ ∈ arc c ua ub r, √(segArcLo a d c ua ub r) ≤ d2 σ (pt a d s) := by
  refine ⟨Real.sqrt_nonneg _, fun s ⟨hs0, hs1⟩ σ hσ => ?_⟩
  rw [d2_eq_sqrt]
  exact Real.sqrt_le_sqrt (segArcLo_le a d c ua ub hd hr hua hub hab hs0 hs1 hσ)

theorem clearFace_segArc {Z : Set ℝ} {w : ℝ → ℝ} (a d c ua ub : P) {r : ℝ} (hd : 0 < nsq d)
    (hr : 0 < r) (hua : nsq ua = r ^ 2) (hub : nsq ub = r ^ 2) (hab : 0 < cross ua ub)
    {Zt H : Set ℝ} {lower : ℝ}
    (hΔ : ∀ h ∈ H, ∀ z ∈ Zt, lower ≤ sdRZ Z w (√(segArcLo a d c ua ub r)) (h - z)) :
    ClearFace Z w (fun s => pt a d s) Zt (arc c ua ub r) H (√(segArcLo a d c ua ub r)) lower :=
  ⟨(segArc_D a d c ua ub hd hr hua hub hab).1, (segArc_D a d c ua ub hd hr hua hub hab).2, hΔ⟩

/-- **slab::dist's low end for two arcs is a lower bound** for the squared distance of every pair
of their points. -/
theorem arcArcLo_le (c1 ua1 ub1 : P) {r1 : ℝ} (c2 ua2 ub2 : P) {r2 : ℝ} (hr1 : 0 < r1)
    (hr2 : 0 < r2) (hua1 : nsq ua1 = r1 ^ 2) (hub1 : nsq ub1 = r1 ^ 2) (hab1 : 0 < cross ua1 ub1)
    (hua2 : nsq ua2 = r2 ^ 2) (hub2 : nsq ub2 = r2 ^ 2) (hab2 : 0 < cross ua2 ub2)
    {v1 v2 : P} (hv1 : v1 ∈ arc c1 ua1 ub1 r1) (hv2 : v2 ∈ arc c2 ua2 ub2 r2) :
    arcArcLo c1 ua1 ub1 r1 c2 ua2 ub2 r2 ≤ nsq (v1 - v2) := by
  have hK : IsCompact (arc c1 ua1 ub1 r1 ×ˢ arc c2 ua2 ub2 r2) :=
    (isCompact_arc _ _ _ _).prod (isCompact_arc _ _ _ _)
  have hne : (arc c1 ua1 ub1 r1 ×ˢ arc c2 ua2 ub2 r2).Nonempty := ⟨(v1, v2), hv1, hv2⟩
  have hcont : Continuous fun p : P × P => nsq (p.1 - p.2) := by unfold nsq dot; fun_prop
  obtain ⟨⟨p1, p2⟩, ⟨hp1, hp2⟩, hmin⟩ := hK.exists_isMinOn hne hcont.continuousOn
  simp only at hp1 hp2
  have hmin' : ∀ q ∈ arc c1 ua1 ub1 r1 ×ˢ arc c2 ua2 ub2 r2,
      nsq (p1 - p2) ≤ nsq (q.1 - q.2) := isMinOn_iff.mp hmin
  refine le_trans ?_ (hmin' (v1, v2) ⟨hv1, hv2⟩)
  have hy : 0 ≤ nsq (p1 - p2) := nsq_nonneg _
  -- an end of either arc
  rcases hp1.2.1.eq_or_lt with h | hs1
  · refine arcArcLo_le_of_base ?_ hy
    rw [eq_start_of_cross hr1 hua1 hab1 hp1 h.symm]
    exact (min_le_left _ _).trans ((min_le_left _ _).trans (ptArcSq_le _ c2 ua2 ub2 hr2 hua2 hub2 hab2 hp2))
  rcases hp1.2.2.eq_or_lt with h | hs2
  · refine arcArcLo_le_of_base ?_ hy
    rw [eq_end_of_cross hr1 hub1 hab1 hp1 h.symm]
    exact (min_le_left _ _).trans ((min_le_right _ _).trans (ptArcSq_le _ c2 ua2 ub2 hr2 hua2 hub2 hab2 hp2))
  rcases hp2.2.1.eq_or_lt with h | ht1
  · refine arcArcLo_le_of_base ?_ hy
    rw [eq_start_of_cross hr2 hua2 hab2 hp2 h.symm, Dist2.nsq_comm]
    exact (min_le_right _ _).trans ((min_le_left _ _).trans (ptArcSq_le _ c1 ua1 ub1 hr1 hua1 hub1 hab1 hp1))
  rcases hp2.2.2.eq_or_lt with h | ht2
  · refine arcArcLo_le_of_base ?_ hy
    rw [eq_end_of_cross hr2 hub2 hab2 hp2 h.symm, Dist2.nsq_comm]
    exact (min_le_right _ _).trans ((min_le_right _ _).trans (ptArcSq_le _ c1 ua1 ub1 hr1 hua1 hub1 hab1 hp1))
  -- both inside
  have hr1s : √(nsq (p1 - c1)) = r1 := by rw [hp1.1, Real.sqrt_sq hr1.le]
  by_cases hcc : c1 = c2
  · -- concentric: (r1 − r2)²
    unfold arcArcLo
    rw [if_pos hcc]
    refine (min_le_right _ _).trans ?_
    have := circle_lower p1 p2 c2 hr2.le hp2.1
    rw [← hcc, hr1s] at this
    exact this
  by_cases hmeet : ∃ v, v ∈ arc c1 ua1 ub1 r1 ∧ v ∈ arc c2 ua2 ub2 r2
  · unfold arcArcLo
    rw [if_neg hcc, if_pos hmeet]; exact hy
  have hp12 : p1 ≠ p2 := fun h => hmeet ⟨p1, hp1, h ▸ hp2⟩
  set x1 := p1 - c1
  set x2 := p2 - c2
  set w := p2 - p1
  set dv := c2 - c1
  -- first order on each arc
  have f1 : cross x1 (p2 - c1) = 0 :=
    cross_eq_zero_of_arc_min hp1 hs1 hs2 fun v' hv' => by
      rw [Dist2.nsq_comm p2 p1, Dist2.nsq_comm p2 v']; exact hmin' (v', p2) ⟨hv', hp2⟩
  have f2 : cross x2 (p1 - c2) = 0 :=
    cross_eq_zero_of_arc_min hp2 ht1 ht2 fun v' hv' => hmin' (p1, v') ⟨hp1, hv'⟩
  have hw1 : cross x1 w = 0 := by
    have : w = (p2 - c1) - x1 := by simp [w, x1]
    rw [this]; simp only [cross, Prod.fst_sub, Prod.snd_sub] at f1 ⊢; linarith
  have hw2 : cross x2 w = 0 := by
    have : w = x2 - (p1 - c2) := by simp [w, x2]
    rw [this]; simp only [cross, Prod.fst_sub, Prod.snd_sub] at f2 ⊢; linarith
  have hwn : 0 < nsq w := by
    rcases (nsq_nonneg w).eq_or_lt with h | h
    · exact absurd (sub_eq_zero.mp (nsq_eq_zero h.symm)).symm hp12
    · exact h
  have h12 : cross x1 x2 = 0 := by
    have := cross_mul_nsq x1 x2 w
    rw [hw1, hw2, zero_mul, zero_mul, sub_zero] at this
    exact (mul_eq_zero.mp this).resolve_right hwn.ne'
  have hdv_eq : dv = w - x2 + x1 := by simp [dv, w, x1, x2]
  have hc1 : cross x1 dv = 0 := by
    rw [hdv_eq]; simp only [cross, Prod.fst_add, Prod.snd_add, Prod.fst_sub, Prod.snd_sub] at hw1 h12 ⊢
    linarith
  have hc2 : cross x2 dv = 0 := by
    rw [hdv_eq]; simp only [cross, Prod.fst_add, Prod.snd_add, Prod.fst_sub, Prod.snd_sub] at hw2 h12 ⊢
    linarith
  have hdvn : 0 < nsq dv := by
    rcases (nsq_nonneg dv).eq_or_lt with h | h
    · exact absurd (sub_eq_zero.mp (nsq_eq_zero h.symm)).symm hcc
    · exact h
  obtain ⟨s1, hs1v, hx1⟩ := parallel_eq hr1 hp1.1 hdvn hc1
  obtain ⟨s2, hs2v, hx2⟩ := parallel_eq hr2 hp2.1 hdvn hc2
  replace hx1 : x1 = sc (s1 * r1 / √(nsq dv)) dv := hx1
  replace hx2 : x2 = sc (s2 * r2 / √(nsq dv)) dv := hx2
  set L := √(nsq dv) with hL
  have hL0 : 0 < L := Real.sqrt_pos.mpr hdvn
  have hL2 : L ^ 2 = nsq dv := Real.sq_sqrt hdvn.le
  -- the candidate (s1, s2) is present: both directions in their sectors
  have hsec1 : inSector ua1 ub1 (sc s1 dv) := by
    have : x1 = sc (r1 / L) (sc s1 dv) := by rw [hx1]; ext <;> simp [sc] <;> ring
    have h := hp1.2; rw [show p1 - c1 = x1 from rfl, this] at h
    exact inSector_sc (div_pos hr1 hL0) h
  have hsec2 : inSector ua2 ub2 (sc s2 dv) := by
    have : x2 = sc (r2 / L) (sc s2 dv) := by rw [hx2]; ext <;> simp [sc] <;> ring
    have h := hp2.2; rw [show p2 - c2 = x2 from rfl, this] at h
    exact inSector_sc (div_pos hr2 hL0) h
  -- and it is the pair's squared distance
  have hval : nsq (p1 - p2) = (L + s2 * r2 - s1 * r1) ^ 2 := by
    have e : p1 - p2 = -(dv + x2 - x1) := by simp [dv, x1, x2]
    rw [e, hx1, hx2]
    have hk : nsq (-(dv + sc (s2 * r2 / L) dv - sc (s1 * r1 / L) dv)) =
        (1 + s2 * r2 / L - s1 * r1 / L) ^ 2 * nsq dv := by
      simp only [nsq, dot, sc, Prod.fst_neg, Prod.snd_neg, Prod.fst_add, Prod.snd_add,
        Prod.fst_sub, Prod.snd_sub]; ring
    rw [hk, ← hL2]
    field_simp
  have hcand : lineCand c1 ua1 ub1 r1 c2 ua2 ub2 r2 s1 s2 = nsq (p1 - p2) := by
    unfold lineCand
    rw [if_pos ⟨hsec1, hsec2⟩, hval]
  unfold arcArcLo
  rw [if_neg hcc, if_neg hmeet, ← hcand]
  refine (min_le_right _ _).trans ?_
  rcases hs1v with rfl | rfl <;> rcases hs2v with rfl | rfl
  · exact (min_le_left _ _).trans (min_le_left _ _)
  · exact (min_le_right _ _).trans (min_le_left _ _)
  · exact (min_le_left _ _).trans (min_le_right _ _)
  · exact (min_le_right _ _).trans (min_le_right _ _)

/-- For a move along an arc against a wall over an arc, D = √arcArcLo is what `ClearFace` (i)
asks for (the move parametrised by any curve onto the arc). -/
theorem arcArc_D (c1 ua1 ub1 : P) {r1 : ℝ} (c2 ua2 ub2 : P) {r2 : ℝ} (hr1 : 0 < r1)
    (hr2 : 0 < r2) (hua1 : nsq ua1 = r1 ^ 2) (hub1 : nsq ub1 = r1 ^ 2) (hab1 : 0 < cross ua1 ub1)
    (hua2 : nsq ua2 = r2 ^ 2) (hub2 : nsq ub2 = r2 ^ 2) (hab2 : 0 < cross ua2 ub2)
    (γ : ℝ → P) (hγ : ∀ s ∈ I01, γ s ∈ arc c1 ua1 ub1 r1) :
    0 ≤ √(arcArcLo c1 ua1 ub1 r1 c2 ua2 ub2 r2) ∧
      ∀ s ∈ I01, ∀ σ ∈ arc c2 ua2 ub2 r2, √(arcArcLo c1 ua1 ub1 r1 c2 ua2 ub2 r2) ≤ d2 σ (γ s) := by
  refine ⟨Real.sqrt_nonneg _, fun s hs σ hσ => ?_⟩
  rw [d2_eq_sqrt]
  exact Real.sqrt_le_sqrt (arcArcLo_le c1 ua1 ub1 c2 ua2 ub2 hr1 hr2 hua1 hub1 hab1 hua2 hub2
    hab2 (hγ s hs) hσ)

end ForgeCam.DistArc
