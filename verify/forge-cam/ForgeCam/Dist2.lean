/-
slab::dist's candidates for two segments (kernel/forge-cam-geom/src/slab.rs), exact: the closed
form the 2.5D checker's D comes from. In squared distances, over any ordered field (so the same
definitions run over ℚ), with the minimum's existence over ℝ.

- `ptSegSq`: point to segment. The foot parameter t = ⟨q − a, d⟩ / |d|² picks the end a
  (t ≤ 0), the end b (t ≥ 1), or the perpendicular. It is a lower bound for every point of the
  segment, and it is attained (`ptSegSq_le`, `ptSegSq_attained`).
- `segSegLo`: two segments. If they are surely apart (both ends of one strictly on one side of
  the other's line), take the least of the four end-to-segment candidates; otherwise 0 (the
  crossing candidate). It is a lower bound for every pair of points (`segSegLo_le`), and when
  apart it is the exact least squared distance (`segSegLo_attained`).

slab.rs computes the same candidates in outward-rounded intervals (L5), not squared; the square
root is monotone.
-/
import ForgeCam.Prism

set_option linter.unusedSectionVars false

namespace ForgeCam.Dist2

section field
variable {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

def dot (u v : K × K) : K := u.1 * v.1 + u.2 * v.2
def cross (u v : K × K) : K := u.1 * v.2 - u.2 * v.1
def nsq (u : K × K) : K := dot u u

/-- The point a + τ·d. -/
def pt (a d : K × K) (τ : K) : K × K := (a.1 + τ * d.1, a.2 + τ * d.2)

/-- The squared distance from q to the segment from a to a + d. -/
def ptSegSq (q a d : K × K) : K :=
  let e := q - a
  let k := dot e d
  if k ≤ 0 then nsq e
  else if nsq d ≤ k then nsq (e - d)
  else cross d e ^ 2 / nsq d

theorem nsq_nonneg (u : K × K) : 0 ≤ nsq u :=
  add_nonneg (mul_self_nonneg _) (mul_self_nonneg _)

theorem lagrange (e d : K × K) : dot e d ^ 2 + cross d e ^ 2 = nsq e * nsq d := by
  unfold dot cross nsq dot; ring

theorem nsq_sub_pt (q a d : K × K) (τ : K) :
    nsq (q - pt a d τ) = nsq (q - a) - 2 * τ * dot (q - a) d + τ ^ 2 * nsq d := by
  simp only [nsq, dot, pt, Prod.fst_sub, Prod.snd_sub]; ring

/-- **The point–segment candidate is a lower bound** for every point of a nondegenerate
segment. -/
theorem ptSegSq_le (q a d : K × K) (hd : 0 < nsq d) {τ : K} (h0 : 0 ≤ τ) (h1 : τ ≤ 1) :
    ptSegSq q a d ≤ nsq (q - pt a d τ) := by
  rw [nsq_sub_pt]
  have hl := lagrange (q - a) d
  unfold ptSegSq
  simp only
  split_ifs with ha hb
  · nlinarith
  · have : nsq (q - a - d) = nsq (q - a) - 2 * dot (q - a) d + nsq d := by
      simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
    rw [this]
    nlinarith [mul_nonneg (sub_nonneg.mpr h1) (sub_nonneg.mpr hb),
      mul_nonneg (sub_nonneg.mpr h1) (sub_nonneg.mpr h1), hd.le]
  · rw [div_le_iff₀ hd]
    nlinarith [sq_nonneg (τ * nsq d - dot (q - a) d)]

/-- And it is attained: it is the exact least squared distance. -/
theorem ptSegSq_attained (q a d : K × K) (hd : 0 < nsq d) :
    ∃ τ, 0 ≤ τ ∧ τ ≤ 1 ∧ nsq (q - pt a d τ) = ptSegSq q a d := by
  unfold ptSegSq
  simp only
  split_ifs with ha hb
  · exact ⟨0, le_rfl, zero_le_one, by simp [pt]⟩
  · refine ⟨1, zero_le_one, le_rfl, ?_⟩
    simp only [nsq, dot, pt, one_mul, Prod.fst_sub, Prod.snd_sub]; ring_nf
  · push Not at ha hb
    refine ⟨dot (q - a) d / nsq d, (div_pos ha hd).le, (div_le_one hd).mpr hb.le, ?_⟩
    rw [nsq_sub_pt, eq_div_iff hd.ne']
    have hl := lagrange (q - a) d
    field_simp
    nlinarith [hl]

/-- Both ends of the segment (c, c + v) strictly on one side of the line (a, a + u). -/
def oneSide (a u c v : K × K) : Prop :=
  (0 < cross u (c - a) ∧ 0 < cross u (c + v - a)) ∨ (cross u (c - a) < 0 ∧ cross u (c + v - a) < 0)

instance (a u c v : K × K) : Decidable (oneSide a u c v) := by unfold oneSide; infer_instance

/-- slab.rs's `apart`. -/
def apart (a u c v : K × K) : Prop := oneSide a u c v ∨ oneSide c v a u

instance (a u c v : K × K) : Decidable (apart a u c v) := by unfold apart; infer_instance

/-- The least of the four end-to-segment candidates. -/
def min4 (a u c v : K × K) : K :=
  min (min (ptSegSq a c v) (ptSegSq (a + u) c v)) (min (ptSegSq c a u) (ptSegSq (c + v) a u))

/-- slab::dist's low end for two segments, squared: the candidates, with 0 unless apart. -/
def segSegLo (a u c v : K × K) : K := if apart a u c v then min4 a u c v else 0

theorem cross_pt (a u : K × K) (c v : K × K) (τ : K) :
    cross u (pt c v τ - a) = (1 - τ) * cross u (c - a) + τ * cross u (c + v - a) := by
  simp only [cross, pt, Prod.fst_sub, Prod.snd_sub, Prod.fst_add, Prod.snd_add]; ring

/-- Apart segments share no point. -/
theorem disjoint_of_oneSide {a u c v : K × K} (h : oneSide a u c v) {τ σ : K}
    (hσ0 : 0 ≤ σ) (hσ1 : σ ≤ 1) : pt a u τ ≠ pt c v σ := by
  intro he
  have h1 : cross u (pt a u τ - a) = 0 := by
    simp only [cross, pt, Prod.fst_sub, Prod.snd_sub]; ring
  rw [he, cross_pt] at h1
  rcases h with ⟨p, q⟩ | ⟨p, q⟩
  · rcases hσ0.eq_or_lt with rfl | hσ
    · nlinarith
    · nlinarith [mul_pos hσ q, mul_nonneg (sub_nonneg.mpr hσ1) p.le]
  · rcases hσ0.eq_or_lt with rfl | hσ
    · nlinarith
    · nlinarith [mul_neg_of_pos_of_neg hσ q, mul_nonpos_of_nonneg_of_nonpos (sub_nonneg.mpr hσ1) p.le]

theorem disjoint_of_apart {a u c v : K × K} (h : apart a u c v) {τ σ : K} (hτ0 : 0 ≤ τ)
    (hτ1 : τ ≤ 1) (hσ0 : 0 ≤ σ) (hσ1 : σ ≤ 1) : pt a u τ ≠ pt c v σ := by
  rcases h with h | h
  · exact disjoint_of_oneSide h hσ0 hσ1
  · exact fun he => disjoint_of_oneSide h hτ0 hτ1 (τ := σ) he.symm

end field

/-! ### The minimum, over ℝ -/

/-- The squared distance between the points at parameters τ and σ. -/
noncomputable def F (a u c v : ℝ × ℝ) (τ σ : ℝ) : ℝ := nsq (pt a u τ - pt c v σ)

theorem nsq_comm (x y : ℝ × ℝ) : nsq (x - y) = nsq (y - x) := by
  simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring

/-- First order at an interior minimum: if moving the parameter either way from τ ∈ (0, 1)
never shortens w + (t − τ)·d, then w ⊥ d. -/
theorem dot_eq_zero_of_min {w d : ℝ × ℝ} {τ : ℝ} (h0 : 0 < τ) (h1 : τ < 1)
    (hmin : ∀ ε, 0 ≤ τ + ε → τ + ε ≤ 1 → nsq w ≤ nsq (w + (ε * d.1, ε * d.2))) :
    dot w d = 0 := by
  by_contra hk
  set k := dot w d
  set n := nsq d
  have hn : 0 ≤ n := nsq_nonneg d
  have hexp : ∀ ε, nsq (w + (ε * d.1, ε * d.2)) = nsq w + 2 * ε * k + ε ^ 2 * n := by
    intro ε; simp only [nsq, dot, k, n, Prod.fst_add, Prod.snd_add]; ring
  set m := min τ (1 - τ) / ((|k| + 1) * (n + 1))
  have hmτ : 0 < min τ (1 - τ) := lt_min h0 (by linarith)
  have hm : 0 < m := by positivity
  have hkm : |k| * m ≤ min τ (1 - τ) := by
    rw [show |k| * m = min τ (1 - τ) * (|k| / ((|k| + 1) * (n + 1))) by ring]
    apply mul_le_of_le_one_right hmτ.le
    rw [div_le_one (by positivity)]
    nlinarith [abs_nonneg k]
  have hmn : m * n < 1 := by
    have : m * n = min τ (1 - τ) / (|k| + 1) * (n / (n + 1)) := by
      simp only [m]; field_simp
    rw [this]
    have h1 : min τ (1 - τ) / (|k| + 1) ≤ 1 := by
      rw [div_le_one (by positivity)]
      linarith [min_le_right τ (1 - τ), abs_nonneg k]
    have h2 : n / (n + 1) < 1 := by rw [div_lt_one (by positivity)]; linarith
    nlinarith [div_nonneg hn (by positivity : (0:ℝ) ≤ n + 1),
      div_nonneg hmτ.le (by positivity : (0:ℝ) ≤ |k| + 1)]
  have hle : |-(k * m)| ≤ min τ (1 - τ) := by
    rw [abs_neg, abs_mul, abs_of_pos hm]; exact hkm
  have hb := abs_le.mp (hle.trans (le_refl _))
  have := hmin (-(k * m)) (by linarith [min_le_left τ (1 - τ), hb.1])
    (by linarith [min_le_right τ (1 - τ), hb.2])
  rw [hexp] at this
  have hk2 : 0 < k ^ 2 := by positivity
  nlinarith [mul_pos hk2 hm]

theorem continuous_F (a u c v : ℝ × ℝ) : Continuous fun p : ℝ × ℝ => F a u c v p.1 p.2 := by
  unfold F nsq dot pt; fun_prop

/-- **For apart segments, the least of the four candidates is the least squared distance.** -/
theorem min4_le_of_disjoint (a u c v : ℝ × ℝ) (hu : 0 < nsq u) (hv : 0 < nsq v)
    (hdis : ∀ τ σ : ℝ, 0 ≤ τ → τ ≤ 1 → 0 ≤ σ → σ ≤ 1 → pt a u τ ≠ pt c v σ)
    {τ σ : ℝ} (hτ0 : 0 ≤ τ) (hτ1 : τ ≤ 1) (hσ0 : 0 ≤ σ) (hσ1 : σ ≤ 1) :
    min4 a u c v ≤ F a u c v τ σ := by
  -- a least pair
  have hK : IsCompact (Set.Icc (0:ℝ) 1 ×ˢ Set.Icc (0:ℝ) 1) := isCompact_Icc.prod isCompact_Icc
  obtain ⟨⟨τ₀, σ₀⟩, ⟨⟨h00, h01⟩, ⟨h10, h11⟩⟩, hmin⟩ := hK.exists_isMinOn
    ⟨(0, 0), ⟨le_rfl, zero_le_one⟩, ⟨le_rfl, zero_le_one⟩⟩ (continuous_F a u c v).continuousOn
  simp only at h00 h01 h10 h11
  have hmin' : ∀ p ∈ Set.Icc (0:ℝ) 1 ×ˢ Set.Icc (0:ℝ) 1,
      F a u c v τ₀ σ₀ ≤ F a u c v p.1 p.2 := isMinOn_iff.mp hmin
  have hle : F a u c v τ₀ σ₀ ≤ F a u c v τ σ := hmin' (τ, σ) ⟨⟨hτ0, hτ1⟩, ⟨hσ0, hσ1⟩⟩
  refine le_trans ?_ hle
  -- the four ways a least pair can sit with an end in it
  have endA : ∀ σ', 0 ≤ σ' → σ' ≤ 1 → F a u c v 0 σ' ≤ F a u c v τ₀ σ₀ →
      min4 a u c v ≤ F a u c v τ₀ σ₀ := by
    intro σ' h0 h1 h
    refine le_trans ?_ h
    have := ptSegSq_le a c v hv h0 h1
    have e : F a u c v 0 σ' = nsq (a - pt c v σ') := by simp [F, pt]
    rw [e]; unfold min4
    exact (min_le_left _ _).trans ((min_le_left _ _).trans this)
  have endB : ∀ σ', 0 ≤ σ' → σ' ≤ 1 → F a u c v 1 σ' ≤ F a u c v τ₀ σ₀ →
      min4 a u c v ≤ F a u c v τ₀ σ₀ := by
    intro σ' h0 h1 h
    refine le_trans ?_ h
    have := ptSegSq_le (a + u) c v hv h0 h1
    have e : F a u c v 1 σ' = nsq (a + u - pt c v σ') := by
      simp only [F, pt, one_mul]; rfl
    rw [e]; unfold min4
    exact (min_le_left _ _).trans ((min_le_right _ _).trans this)
  have endC : ∀ τ', 0 ≤ τ' → τ' ≤ 1 → F a u c v τ' 0 ≤ F a u c v τ₀ σ₀ →
      min4 a u c v ≤ F a u c v τ₀ σ₀ := by
    intro τ' h0 h1 h
    refine le_trans ?_ h
    have := ptSegSq_le c a u hu h0 h1
    have e : F a u c v τ' 0 = nsq (c - pt a u τ') := by
      rw [F, nsq_comm]; simp [pt]
    rw [e]; unfold min4
    exact (min_le_right _ _).trans ((min_le_left _ _).trans this)
  have endD : ∀ τ', 0 ≤ τ' → τ' ≤ 1 → F a u c v τ' 1 ≤ F a u c v τ₀ σ₀ →
      min4 a u c v ≤ F a u c v τ₀ σ₀ := by
    intro τ' h0 h1 h
    refine le_trans ?_ h
    have := ptSegSq_le (c + v) a u hu h0 h1
    have e : F a u c v τ' 1 = nsq (c + v - pt a u τ') := by
      rw [F, nsq_comm]; simp only [pt, one_mul]; rfl
    rw [e]; unfold min4
    exact (min_le_right _ _).trans ((min_le_right _ _).trans this)
  rcases h00.eq_or_lt with h | hτp
  · exact endA σ₀ h10 h11 (by rw [← h])
  rcases h01.eq_or_lt with h | hτq
  · exact endB σ₀ h10 h11 (by rw [h])
  rcases h10.eq_or_lt with h | hσp
  · exact endC τ₀ h00 h01 (by rw [← h])
  rcases h11.eq_or_lt with h | hσq
  · exact endD τ₀ h00 h01 (by rw [h])
  -- both inside: w = P(τ₀) − Q(σ₀) is orthogonal to u and to v
  set w := pt a u τ₀ - pt c v σ₀ with hw
  have hFmin : ∀ τ' σ', 0 ≤ τ' → τ' ≤ 1 → 0 ≤ σ' → σ' ≤ 1 →
      nsq w ≤ F a u c v τ' σ' := fun τ' σ' a1 a2 a3 a4 => hmin' (τ', σ') ⟨⟨a1, a2⟩, ⟨a3, a4⟩⟩
  have hwu : dot w u = 0 := by
    refine dot_eq_zero_of_min hτp hτq fun ε h1 h2 => ?_
    have := hFmin (τ₀ + ε) σ₀ h1 h2 h10 h11
    convert this using 1
    simp only [F, w, pt, Prod.fst_add, Prod.snd_add, Prod.fst_sub, Prod.snd_sub, nsq, dot]; ring
  have hwv : dot w (-v.1, -v.2) = 0 := by
    refine dot_eq_zero_of_min hσp hσq fun ε h1 h2 => ?_
    have := hFmin τ₀ (σ₀ + ε) h00 h01 h1 h2
    convert this using 1
    simp only [F, w, pt, Prod.fst_add, Prod.snd_add, Prod.fst_sub, Prod.snd_sub, nsq, dot]; ring
  have hwv' : dot w v = 0 := by simp only [dot] at hwv ⊢; linarith
  by_cases hx : cross u v = 0
  · -- parallel: slide along both, keeping w, until a parameter meets an end
    set l := dot v u / nsq u with hl
    have hvl : v = (l * u.1, l * u.2) := by
      have e1 : v.1 * nsq u = dot v u * u.1 := by
        simp only [cross] at hx; simp only [nsq, dot]; linear_combination (-u.2) * hx
      have e2 : v.2 * nsq u = dot v u * u.2 := by
        simp only [cross] at hx; simp only [nsq, dot]; linear_combination u.1 * hx
      ext
      · simp only [l]; field_simp; linarith
      · simp only [l]; field_simp; linarith
    have hl0 : l ≠ 0 := by
      intro h0; rw [h0] at hvl; rw [hvl] at hv; simp [nsq, dot] at hv
    -- F at (τ', σ') with τ' − τ₀ = l (σ' − σ₀) equals nsq w
    have slide : ∀ τ' σ', τ' - τ₀ = l * (σ' - σ₀) → F a u c v τ' σ' = nsq w := by
      intro τ' σ' he
      have : pt a u τ' - pt c v σ' = w + ((τ' - τ₀ - l * (σ' - σ₀)) * u.1,
          (τ' - τ₀ - l * (σ' - σ₀)) * u.2) := by
        rw [hw, hvl]; ext <;> simp only [pt, Prod.fst_add, Prod.snd_add, Prod.fst_sub,
          Prod.snd_sub] <;> ring
      rw [F, this, he, sub_self, zero_mul, zero_mul]
      simp [nsq, dot]
    rcases lt_or_gt_of_ne hl0 with hneg | hpos
    · -- l < 0
      by_cases hA : σ₀ - τ₀ / l ≤ 1
      · refine endA (σ₀ - τ₀ / l) ?_ hA (le_of_eq (slide 0 _ ?_))
        · have : 0 ≤ -(τ₀ / l) := by rw [neg_nonneg]; exact div_nonpos_of_nonneg_of_nonpos h00 hneg.le
          linarith
        · field_simp; ring
      · push Not at hA
        refine endD (τ₀ + l * (1 - σ₀)) ?_ ?_ (le_of_eq (slide _ 1 (by ring)))
        · have : -(τ₀ / l) > 1 - σ₀ := by linarith
          rw [neg_div', gt_iff_lt, lt_div_iff_of_neg hneg] at this
          linarith
        · nlinarith
    · -- l > 0
      by_cases hA : 0 ≤ σ₀ - τ₀ / l
      · refine endA (σ₀ - τ₀ / l) hA ?_ (le_of_eq (slide 0 _ ?_))
        · have : 0 ≤ τ₀ / l := div_nonneg h00 hpos.le
          linarith
        · field_simp; ring
      · push Not at hA
        refine endC (τ₀ - l * σ₀) ?_ ?_ (le_of_eq (slide _ 0 (by ring)))
        · have : σ₀ < τ₀ / l := by linarith
          rw [lt_div_iff₀ hpos] at this
          linarith
        · nlinarith
  · -- crossing directions: w = 0, a common point
    exfalso
    have h1 : w.1 = 0 := by
      apply (mul_left_cancel₀ hx)
      simp only [dot] at hwu hwv'; simp only [cross]
      linear_combination v.2 * hwu - u.2 * hwv'
    have h2 : w.2 = 0 := by
      apply (mul_left_cancel₀ hx)
      simp only [dot] at hwu hwv'; simp only [cross]
      linear_combination (-v.1) * hwu + u.1 * hwv'
    apply hdis τ₀ σ₀ h00 h01 h10 h11
    have : w = 0 := Prod.ext h1 h2
    rw [hw] at this
    exact sub_eq_zero.mp this

/-- **slab::dist's low end for two segments is a lower bound** for the squared distance of
every pair of their points. -/
theorem segSegLo_le (a u c v : ℝ × ℝ) (hu : 0 < nsq u) (hv : 0 < nsq v) {τ σ : ℝ}
    (hτ0 : 0 ≤ τ) (hτ1 : τ ≤ 1) (hσ0 : 0 ≤ σ) (hσ1 : σ ≤ 1) :
    segSegLo a u c v ≤ F a u c v τ σ := by
  unfold segSegLo
  split_ifs with h
  · exact min4_le_of_disjoint a u c v hu hv
      (fun τ σ a1 a2 a3 a4 => disjoint_of_apart h a1 a2 a3 a4) hτ0 hτ1 hσ0 hσ1
  · exact nsq_nonneg _

/-- **And when apart it is exact**: some pair of points is at exactly that squared distance. -/
theorem segSegLo_attained (a u c v : ℝ × ℝ) (hu : 0 < nsq u) (hv : 0 < nsq v)
    (h : apart a u c v) :
    ∃ τ σ : ℝ, 0 ≤ τ ∧ τ ≤ 1 ∧ 0 ≤ σ ∧ σ ≤ 1 ∧ F a u c v τ σ = segSegLo a u c v := by
  rw [segSegLo, ite_eq_left_iff.mpr (fun h' => absurd h h') ]
  obtain ⟨t1, a1, b1, e1⟩ := ptSegSq_attained a c v hv
  obtain ⟨t2, a2, b2, e2⟩ := ptSegSq_attained (a + u) c v hv
  obtain ⟨t3, a3, b3, e3⟩ := ptSegSq_attained c a u hu
  obtain ⟨t4, a4, b4, e4⟩ := ptSegSq_attained (c + v) a u hu
  have f1 : F a u c v 0 t1 = ptSegSq a c v := by rw [← e1]; simp [F, pt]
  have f2 : F a u c v 1 t2 = ptSegSq (a + u) c v := by
    rw [← e2]; simp only [F, pt, one_mul]; rfl
  have f3 : F a u c v t3 0 = ptSegSq c a u := by rw [← e3, F, nsq_comm]; simp [pt]
  have f4 : F a u c v t4 1 = ptSegSq (c + v) a u := by
    rw [← e4, F, nsq_comm]; simp only [pt, one_mul]; rfl
  unfold min4
  rcases min_cases (min (ptSegSq a c v) (ptSegSq (a + u) c v))
      (min (ptSegSq c a u) (ptSegSq (c + v) a u)) with ⟨hm, -⟩ | ⟨hm, -⟩ <;> rw [hm]
  · rcases min_cases (ptSegSq a c v) (ptSegSq (a + u) c v) with ⟨hm, -⟩ | ⟨hm, -⟩ <;> rw [hm]
    · exact ⟨0, t1, le_rfl, zero_le_one, a1, b1, f1⟩
    · exact ⟨1, t2, zero_le_one, le_rfl, a2, b2, f2⟩
  · rcases min_cases (ptSegSq c a u) (ptSegSq (c + v) a u) with ⟨hm, -⟩ | ⟨hm, -⟩ <;> rw [hm]
    · exact ⟨t3, 0, a3, b3, le_rfl, zero_le_one, f3⟩
    · exact ⟨t4, 1, a4, b4, zero_le_one, le_rfl, f4⟩

/-! ### Into the checker: hypothesis (i) of `ClearFace`, discharged -/

theorem d2_pt_eq (a u c v : ℝ × ℝ) (τ σ : ℝ) :
    d2 (pt c v σ) (pt a u τ) = √(F a u c v τ σ) := by
  rw [F, nsq_comm]; simp only [d2, nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring_nf

/-- **For a segment move against a segment section, slab::dist's D = √segSegLo is what
`ClearFace` (i) asks for**: 0 ≤ D ≤ |σ − γ(s)| at every instant and section point. -/
theorem segSeg_D (a u c v : ℝ × ℝ) (hu : 0 < nsq u) (hv : 0 < nsq v) :
    0 ≤ √(segSegLo a u c v) ∧
      ∀ s ∈ I01, ∀ σ ∈ (fun σ => pt c v σ) '' Set.Icc 0 1,
        √(segSegLo a u c v) ≤ d2 σ (pt a u s) := by
  refine ⟨Real.sqrt_nonneg _, ?_⟩
  rintro s ⟨hs0, hs1⟩ _ ⟨σ, ⟨h0, h1⟩, rfl⟩
  rw [d2_pt_eq]
  exact Real.sqrt_le_sqrt (segSegLo_le a u c v hu hv hs0 hs1 h0 h1)

/-- So a wall over a segment, checked against a segment move with the Δ search's bound, is a
`ClearFace` with no hypothesis left on D. -/
theorem clearFace_segSeg {Z : Set ℝ} {w : ℝ → ℝ} (a u c v : ℝ × ℝ) (hu : 0 < nsq u)
    (hv : 0 < nsq v) {Zt H : Set ℝ} {lower : ℝ}
    (hΔ : ∀ h ∈ H, ∀ z ∈ Zt, lower ≤ sdRZ Z w (√(segSegLo a u c v)) (h - z)) :
    ClearFace Z w (fun s => pt a u s) Zt ((fun σ => pt c v σ) '' Set.Icc 0 1) H
      (√(segSegLo a u c v)) lower :=
  ⟨(segSeg_D a u c v hu hv).1, (segSeg_D a u c v hu hv).2, hΔ⟩

/-- The executable form, over ℚ (slab.rs's candidates without the rounding). -/
def segSegLoQ (a u c v : ℚ × ℚ) : ℚ := segSegLo a u c v

-- Two parallel unit segments 3 apart, and two crossing ones.
#guard segSegLoQ (0, 0) (1, 0) (0, 3) (1, 0) = 9
#guard segSegLoQ (0, 0) (2, 2) (0, 2) (2, -2) = 0
#guard segSegLoQ (0, 0) (1, 0) (2, 1) (0, 1) = 2

end ForgeCam.Dist2
