/-
slab::dist_region and Trim::classify (kernel/forge-cam-geom/src/check.rs, slab.rs, at CAM
787239e): the point-in-trim test and the distance from a path to a flat face's region.

The trim's pieces (787239e): its lines; and for each arc, the circle's arc of radius r between the
rays through its stored ends a, b, plus the radial pieces joining the circle to a and b (a stored
end may lie off the circle by the gap g ≥ ||a − c| − r|; neighbours share the ends bit for bit,
so the trim closes exactly).

`ray_side`, in its ray's frame (the query point at the origin, the ray along +x, as its `rot`
makes it), with exact signs:
- a line: undecided if an end is on the ray's line ahead of the point; else it crosses iff its
  ends are on opposite sides and the crossing is ahead (undecided if exactly at the point);
- an arc: undecided if a stored end lies within g of the ray's line and not behind by more than
  g (the radial pieces); undecided if the ray's line is tangent to the circle; else each of the
  line's two meetings with the circle is undecided if its sector test is (on a sector edge) or
  if it is at the point, and counts if it is in the sector and ahead.
Proven (`raySide_sound`): when every piece decides, the parity it computes is the parity of
the true number of meetings of the ray with the pieces (radial pieces included, which it never
meets), and the ray is generic: it meets no piece end and touches no circle.

The region: `EvenOdd R pieces` states the even-odd rule for generic rays, the meaning of a face
region bounded by its trim (a Jordan-curve fact, assumed here). With it, a decided `ray_side`
classifies exactly (`classify_sound`).

`dist_region` (`distRegion_lower`): the path is connected and its start is classified outside,
the region's frontier lies in the pieces, and each piece has a lower bound on its distance to
the path. Then the least bound is a lower bound on the distance from every path point to every
point of the region. An arc's circle part and both radial pieces are bounded by the arc's
distance less the gap (`radial_lower`), as `tseg_dist` does.
-/
import ForgeCam.DistArc

open Set ForgeCam.Dist2 ForgeCam.DistArc

namespace ForgeCam.Trim

noncomputable section

/-- The ray from the origin along +x (the frame `ray_side`'s `rot` makes). -/
def ray : Set P := {v | 0 ≤ v.1 ∧ v.2 = 0}

/-- The segment from a to b. -/
def seg (a b : P) : Set P := (fun τ => pt a (b - a) τ) '' Icc 0 1

/-- The circle point on the ray from c through e. -/
noncomputable def onCircle (c e : P) (r : ℝ) : P := c + sc (r / √(nsq (e - c))) (e - c)

/-- The pieces of the trim. -/
inductive Piece
  | line (a b : P)
  /-- an arc, counter-clockwise from the ray through a to the ray through b, about c, radius r,
  its stored ends within g of the circle; it stands for three pieces: the radial piece from a,
  the circle's arc, the radial piece to b -/
  | arc (c a b : P) (r g : ℝ)

/-- The points of a piece. -/
def Piece.pts : Piece → Set P
  | .line a b => seg a b
  | .arc c a b r _ => seg a (onCircle c a r) ∪ DistArc.arc c (a - c) (b - c) r ∪ seg (onCircle c b r) b

/-- How many times the ray meets a piece (radial pieces included). -/
noncomputable def Piece.meets : Piece → ℕ
  | .line a b => (ray ∩ seg a b).ncard
  | .arc c a b r _ => (ray ∩ seg a (onCircle c a r)).ncard + (ray ∩ DistArc.arc c (a - c) (b - c) r).ncard +
      (ray ∩ seg (onCircle c b r) b).ncard

theorem mem_seg {a b v : P} : v ∈ seg a b ↔ ∃ τ, 0 ≤ τ ∧ τ ≤ 1 ∧ v = (a.1 + τ * (b.1 - a.1), a.2 + τ * (b.2 - a.2)) := by
  simp only [seg, mem_image, mem_Icc, pt, Prod.fst_sub, Prod.snd_sub]
  constructor
  · rintro ⟨τ, ⟨h0, h1⟩, rfl⟩; exact ⟨τ, h0, h1, rfl⟩
  · rintro ⟨τ, h0, h1, rfl⟩; exact ⟨τ, ⟨h0, h1⟩, rfl⟩

/-- A convex combination of two negatives is negative. -/
theorem comb_neg {x y τ : ℝ} (hx : x < 0) (hy : y < 0) (h0 : 0 ≤ τ) (h1 : τ ≤ 1) :
    x + τ * (y - x) < 0 := by
  nlinarith [mul_nonneg h0 (sub_nonneg.mpr (le_max_right x y)),
    mul_nonneg (sub_nonneg.mpr h1) (sub_nonneg.mpr (le_max_left x y)), max_lt hx hy]

theorem comb_pos {x y τ : ℝ} (hx : 0 < x) (hy : 0 < y) (h0 : 0 ≤ τ) (h1 : τ ≤ 1) :
    0 < x + τ * (y - x) := by
  have := comb_neg (neg_neg_of_pos hx) (neg_neg_of_pos hy) h0 h1
  linarith

theorem ncard_zero_of {S : Set P} (h : ∀ v, v ∉ S) : S.ncard = 0 := by
  rw [show S = ∅ from eq_empty_of_forall_notMem h, ncard_empty]

/-- `ray_side` on a line piece, with exact signs: `none` is undecided. -/
def lineDec (a b : P) : Option ℕ :=
  open Classical in
  if a.2 = 0 ∨ b.2 = 0 then
    if (a.2 = 0 ∧ 0 ≤ a.1) ∨ (b.2 = 0 ∧ 0 ≤ b.1) then none else some 0
  else if (0 < a.2 ↔ 0 < b.2) then some 0
  else if a.1 - a.2 * (b.1 - a.1) / (b.2 - a.2) = 0 then none
  else if 0 < a.1 - a.2 * (b.1 - a.1) / (b.2 - a.2) then some 1 else some 0

/-- **A decided line piece**: the count is the true number of meetings, and neither end is on
the ray. -/
theorem lineDec_sound {a b : P} {n : ℕ} (h : lineDec a b = some n) :
    (ray ∩ seg a b).ncard = n ∧ a ∉ ray ∧ b ∉ ray := by
  unfold lineDec at h
  have hend : ∀ e : P, e.2 = 0 → ¬ (0 ≤ e.1) → e ∉ ray := fun e h0 h1 hr => h1 hr.1
  split_ifs at h with h1 h2 h3 h4 h5
  · -- an end on the ray's line, behind
    obtain rfl := (Option.some.inj h).symm
    push Not at h2
    obtain ⟨ha, hb⟩ := h2
    have hna : a ∉ ray := fun hr => absurd hr.1 (not_le.mpr (ha hr.2))
    have hnb : b ∉ ray := fun hr => absurd hr.1 (not_le.mpr (hb hr.2))
    refine ⟨?_, hna, hnb⟩
    refine ncard_zero_of fun v ⟨hv, hs⟩ => ?_
    obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hs
    obtain ⟨hx, hy⟩ := hv
    simp only at hx hy
    rcases h1 with ha0 | hb0
    · have hax := ha ha0
      rw [ha0] at hy
      rcases (eq_or_ne τ 0) with rfl | hτ
      · simp at hx; linarith
      · have hb0 : b.2 = 0 := by
          have : τ * b.2 = 0 := by linarith
          exact (mul_eq_zero.mp this).resolve_left hτ
        have hbx := hb hb0
        linarith [comb_neg hax hbx h0 h1']
    · have hbx := hb hb0
      rw [hb0] at hy
      rcases (eq_or_ne τ 1) with rfl | hτ
      · simp at hx; linarith
      · have ha0 : a.2 = 0 := by
          have : (1 - τ) * a.2 = 0 := by linarith
          exact (mul_eq_zero.mp this).resolve_left (sub_ne_zero.mpr (Ne.symm hτ))
        have hax := ha ha0
        linarith [comb_neg hax hbx h0 h1']
  · -- both ends strictly on one side
    obtain rfl := (Option.some.inj h).symm
    push Not at h1
    refine ⟨?_, fun hr => h1.1 hr.2, fun hr => h1.2 hr.2⟩
    refine ncard_zero_of fun v ⟨hv, hs⟩ => ?_
    obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hs
    have hy := hv.2
    simp only at hy
    rcases lt_or_gt_of_ne h1.1 with ha | ha
    · have hb : b.2 < 0 := by
        rcases lt_or_gt_of_ne h1.2 with hb | hb
        · exact hb
        · exact absurd (h3.mpr hb) (not_lt.mpr ha.le)
      linarith [comb_neg ha hb h0 h1']
    · have hb : 0 < b.2 := h3.mp ha
      linarith [comb_pos ha hb h0 h1']
  · -- opposite sides: one meeting, at xc > 0
    obtain rfl := (Option.some.inj h).symm
    push Not at h1
    have hd : b.2 - a.2 ≠ 0 := by
      intro h0
      have : b.2 = a.2 := by linarith
      rw [this] at h3; exact h3 Iff.rfl
    set xc := a.1 - a.2 * (b.1 - a.1) / (b.2 - a.2)
    refine ⟨?_, fun hr => h1.1 hr.2, fun hr => h1.2 hr.2⟩
    have : ray ∩ seg a b = {(xc, 0)} := by
      ext v
      simp only [mem_inter_iff, mem_singleton_iff]
      constructor
      · rintro ⟨hv, hs⟩
        obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hs
        have hy := hv.2; simp only at hy
        have hτ : τ = -a.2 / (b.2 - a.2) := by field_simp; linarith
        ext
        · simp only [xc]; rw [hτ]; field_simp; ring
        · simp only; exact hy
      · rintro rfl
        refine ⟨⟨h5.le, rfl⟩, mem_seg.mpr ⟨-a.2 / (b.2 - a.2), ?_, ?_, ?_⟩⟩
        · rcases lt_or_gt_of_ne h1.1 with ha | ha
          · have hb : 0 < b.2 := by
              by_contra hb; push Not at hb
              exact h3 ⟨fun h => absurd h (not_lt.mpr ha.le),
                fun h => absurd h (not_lt.mpr hb)⟩
            exact div_nonneg (by linarith) (by linarith)
          · have hb : b.2 < 0 := by
              by_contra hb; push Not at hb
              exact h3 ⟨fun _ => lt_of_le_of_ne hb (Ne.symm h1.2), fun _ => ha⟩
            exact div_nonneg_of_nonpos (by linarith) (by linarith)
        · rcases lt_or_gt_of_ne h1.1 with ha | ha
          · have hb : 0 < b.2 := by
              by_contra hb; push Not at hb
              exact h3 ⟨fun h => absurd h (not_lt.mpr ha.le),
                fun h => absurd h (not_lt.mpr hb)⟩
            rw [div_le_one (by linarith)]; linarith
          · have hb : b.2 < 0 := by
              by_contra hb; push Not at hb
              exact h3 ⟨fun _ => lt_of_le_of_ne hb (Ne.symm h1.2), fun _ => ha⟩
            rw [div_le_one_of_neg (by linarith)]; linarith
        · ext
          · simp only [xc]; field_simp; ring
          · simp only; field_simp; ring
    rw [this, ncard_singleton]
  · -- opposite sides, the meeting behind
    obtain rfl := (Option.some.inj h).symm
    push Not at h1 h5
    have hd : b.2 - a.2 ≠ 0 := by
      intro h0
      have : b.2 = a.2 := by linarith
      rw [this] at h3; exact h3 Iff.rfl
    refine ⟨?_, fun hr => h1.1 hr.2, fun hr => h1.2 hr.2⟩
    refine ncard_zero_of fun v ⟨hv, hs⟩ => ?_
    obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hs
    obtain ⟨hx, hy⟩ := hv
    simp only at hx hy
    have hτ : τ = -a.2 / (b.2 - a.2) := by field_simp; linarith
    rw [hτ] at hx
    have : a.1 + -a.2 / (b.2 - a.2) * (b.1 - a.1) = a.1 - a.2 * (b.1 - a.1) / (b.2 - a.2) := by
      field_simp; ring
    rw [this] at hx
    exact h4 (le_antisymm h5 hx)

/-! ### Arc pieces: the radial pieces and the circle's arc -/

/-- `ray_side`'s gap test for a stored end e: within g of the ray's line, not behind by more. -/
def near (e : P) (g : ℝ) : Prop := -g ≤ e.2 ∧ e.2 ≤ g ∧ -g ≤ e.1

/-- The stored end and its circle point differ by the gap at most, in each coordinate. -/
theorem onCircle_close {c e : P} {r g : ℝ} (he : 0 < nsq (e - c))
    (hg : |√(nsq (e - c)) - r| ≤ g) :
    |(onCircle c e r).1 - e.1| ≤ g ∧ |(onCircle c e r).2 - e.2| ≤ g := by
  set L := √(nsq (e - c)) with hL
  have hL0 : 0 < L := Real.sqrt_pos.mpr he
  have hL2 : L ^ 2 = nsq (e - c) := Real.sq_sqrt he.le
  have k1 : (e.1 - c.1) ^ 2 ≤ L ^ 2 := by
    rw [hL2]; simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; nlinarith [sq_nonneg (e.2 - c.2)]
  have k2 : (e.2 - c.2) ^ 2 ≤ L ^ 2 := by
    rw [hL2]; simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; nlinarith [sq_nonneg (e.1 - c.1)]
  have a1 : |e.1 - c.1| ≤ L := by rw [← abs_of_pos hL0]; exact sq_le_sq.mp k1
  have a2 : |e.2 - c.2| ≤ L := by rw [← abs_of_pos hL0]; exact sq_le_sq.mp k2
  have e1 : (onCircle c e r).1 - e.1 = (r - L) / L * (e.1 - c.1) := by
    simp only [onCircle, sc, Prod.fst_add, Prod.fst_sub, ← hL]; field_simp; ring
  have e2 : (onCircle c e r).2 - e.2 = (r - L) / L * (e.2 - c.2) := by
    simp only [onCircle, sc, Prod.snd_add, Prod.snd_sub, ← hL]; field_simp; ring
  have hrl : |r - L| ≤ g := by rw [abs_sub_comm]; exact hg
  constructor
  · rw [e1, abs_mul, abs_div, abs_of_pos hL0]
    calc |r - L| / L * |e.1 - c.1| ≤ |r - L| / L * L :=
          mul_le_mul_of_nonneg_left a1 (div_nonneg (abs_nonneg _) hL0.le)
      _ = |r - L| := by field_simp
      _ ≤ g := hrl
  · rw [e2, abs_mul, abs_div, abs_of_pos hL0]
    calc |r - L| / L * |e.2 - c.2| ≤ |r - L| / L * L :=
          mul_le_mul_of_nonneg_left a2 (div_nonneg (abs_nonneg _) hL0.le)
      _ = |r - L| := by field_simp
      _ ≤ g := hrl

/-- Every point of a segment from e to a point within g of e is within g of e. -/
theorem seg_close {e f y : P} {g : ℝ} (h1 : |f.1 - e.1| ≤ g) (h2 : |f.2 - e.2| ≤ g)
    (hy : y ∈ seg e f ∪ seg f e) : |y.1 - e.1| ≤ g ∧ |y.2 - e.2| ≤ g := by
  rcases hy with hy | hy <;> obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hy
  · simp only
    rw [show e.1 + τ * (f.1 - e.1) - e.1 = τ * (f.1 - e.1) by ring,
      show e.2 + τ * (f.2 - e.2) - e.2 = τ * (f.2 - e.2) by ring, abs_mul, abs_mul,
      abs_of_nonneg h0]
    exact ⟨(mul_le_of_le_one_left (abs_nonneg _) h1').trans h1,
      (mul_le_of_le_one_left (abs_nonneg _) h1').trans h2⟩
  · simp only
    rw [show f.1 + τ * (e.1 - f.1) - e.1 = (1 - τ) * (f.1 - e.1) by ring,
      show f.2 + τ * (e.2 - f.2) - e.2 = (1 - τ) * (f.2 - e.2) by ring, abs_mul, abs_mul,
      abs_of_nonneg (by linarith : (0:ℝ) ≤ 1 - τ)]
    exact ⟨(mul_le_of_le_one_left (abs_nonneg _) (by linarith)).trans h1,
      (mul_le_of_le_one_left (abs_nonneg _) (by linarith)).trans h2⟩

/-- **A radial piece misses the ray when its end is not near**: `ray_side`'s gap test. -/
theorem radial_misses {c e : P} {r g : ℝ} (he : 0 < nsq (e - c))
    (hg : |√(nsq (e - c)) - r| ≤ g) (hn : ¬ near e g) {y : P}
    (hy : y ∈ seg e (onCircle c e r) ∪ seg (onCircle c e r) e) : y ∉ ray := by
  obtain ⟨h1, h2⟩ := onCircle_close he hg
  obtain ⟨k1, k2⟩ := seg_close h1 h2 hy
  rintro ⟨hx, hy0⟩
  apply hn
  rw [hy0] at k2
  obtain ⟨l1, _⟩ := abs_le.mp k1
  obtain ⟨l2, u2⟩ := abs_le.mp k2
  exact ⟨by linarith, by linarith, by linarith⟩

/-- One of the ray's line's meetings with the circle, at x: decided or not, and its count. -/
def meetDec (c a b : P) (x : ℝ) : Option ℕ :=
  open Classical in
  if 0 < cross (a - c) (x - c.1, -c.2) ∧ 0 < cross (x - c.1, -c.2) (b - c) then
    (if x = 0 then none else if 0 < x then some 1 else some 0)
  else if cross (a - c) (x - c.1, -c.2) < 0 ∨ cross (x - c.1, -c.2) (b - c) < 0 then some 0
  else none

def addO : Option ℕ → Option ℕ → Option ℕ
  | some m, some n => some (m + n)
  | _, _ => none

/-- `ray_side` on an arc piece (its circle's arc and its two radial pieces). -/
def arcDec (c a b : P) (r g : ℝ) : Option ℕ :=
  open Classical in
  if near a g ∨ near b g then none
  else if r ^ 2 - c.2 ^ 2 < 0 then some 0
  else if r ^ 2 - c.2 ^ 2 = 0 then none
  else addO (meetDec c a b (c.1 + √(r ^ 2 - c.2 ^ 2))) (meetDec c a b (c.1 - √(r ^ 2 - c.2 ^ 2)))

theorem addO_some {m n : Option ℕ} {k : ℕ} (h : addO m n = some k) :
    ∃ i j, m = some i ∧ n = some j ∧ k = i + j := by
  cases m <;> cases n <;> simp [addO] at h
  exact ⟨_, _, rfl, rfl, h.symm⟩

/-- A point of the ray's line on the circle is one of the two meetings. -/
theorem ray_circle {c v : P} {r : ℝ} (hy : v.2 = 0) (hc : nsq (v - c) = r ^ 2) :
    0 ≤ r ^ 2 - c.2 ^ 2 ∧
      (v = (c.1 + √(r ^ 2 - c.2 ^ 2), 0) ∨ v = (c.1 - √(r ^ 2 - c.2 ^ 2), 0)) := by
  simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub, hy] at hc
  have h2 : (v.1 - c.1) ^ 2 = r ^ 2 - c.2 ^ 2 := by nlinarith
  refine ⟨by rw [← h2]; positivity, ?_⟩
  have hs : √(r ^ 2 - c.2 ^ 2) = |v.1 - c.1| := by rw [← h2, Real.sqrt_sq_eq_abs]
  rcases le_total 0 (v.1 - c.1) with h | h
  · left; ext
    · rw [hs, abs_of_nonneg h]; ring
    · exact hy
  · right; ext
    · rw [hs, abs_of_nonpos h]; ring
    · exact hy

/-- The meetings on the ray, as a finite set's count. -/
theorem ncard_two {p q : P} (hpq : p ≠ q) (Q : P → Prop) :
    {v | (v = p ∨ v = q) ∧ Q v}.ncard =
      (open Classical in (if Q p then 1 else 0) + (if Q q then 1 else 0)) := by
  classical
  by_cases hp : Q p <;> by_cases hq : Q q
  · rw [show {v | (v = p ∨ v = q) ∧ Q v} = {p, q} by
      ext v; simp only [mem_setOf_eq, mem_insert_iff, mem_singleton_iff]
      constructor
      · rintro ⟨h, -⟩; exact h
      · rintro (rfl | rfl) <;> simp_all]
    rw [ncard_pair hpq]; simp [hp, hq]
  · rw [show {v | (v = p ∨ v = q) ∧ Q v} = {p} by
      ext v; simp only [mem_setOf_eq, mem_singleton_iff]
      constructor
      · rintro ⟨rfl | rfl, h⟩
        · rfl
        · exact absurd h hq
      · rintro rfl; exact ⟨Or.inl rfl, hp⟩]
    rw [ncard_singleton]; simp [hp, hq]
  · rw [show {v | (v = p ∨ v = q) ∧ Q v} = {q} by
      ext v; simp only [mem_setOf_eq, mem_singleton_iff]
      constructor
      · rintro ⟨rfl | rfl, h⟩
        · exact absurd h hp
        · rfl
      · rintro rfl; exact ⟨Or.inr rfl, hq⟩]
    rw [ncard_singleton]; simp [hp, hq]
  · rw [show {v | (v = p ∨ v = q) ∧ Q v} = ∅ by
      ext v; simp only [mem_setOf_eq, mem_empty_iff_false, iff_false, not_and]
      rintro (rfl | rfl) <;> assumption]
    simp [hp, hq]

/-- A decided meeting counts exactly when the point is on the ray and in the sector. -/
theorem meetDec_sound {c a b : P} {x : ℝ} {n : ℕ} (h : meetDec c a b x = some n) :
    n = (open Classical in if (0 ≤ x ∧ inSector (a - c) (b - c) (x - c.1, -c.2)) then 1 else 0) := by
  classical
  unfold meetDec at h
  split_ifs at h with h1 h2 h3 h4 <;> simp only [Option.some.injEq] at h <;> subst h
  · rw [if_pos ⟨h3.le, h1.1.le, h1.2.le⟩]
  · push Not at h3
    rw [if_neg]; rintro ⟨hx, -⟩; exact h2 (le_antisymm h3 hx)
  · rw [if_neg]; rintro ⟨-, s1, s2⟩
    rcases h4 with h4 | h4 <;> linarith


theorem onCircle_sub {c e : P} (r : ℝ) : onCircle c e r - c = sc (r / √(nsq (e - c))) (e - c) := by
  simp [onCircle]

theorem onCircle_nsq {c e : P} {r : ℝ} (hr : 0 ≤ r) (he : 0 < nsq (e - c)) :
    nsq (onCircle c e r - c) = r ^ 2 := by
  rw [onCircle_sub]
  have hL : (√(nsq (e - c))) ^ 2 = nsq (e - c) := Real.sq_sqrt he.le
  have hL0 : 0 < √(nsq (e - c)) := Real.sqrt_pos.mpr he
  have : nsq (sc (r / √(nsq (e - c))) (e - c)) = (r / √(nsq (e - c))) ^ 2 * nsq (e - c) := by
    simp only [nsq, dot, sc]; ring
  rw [this, div_pow, hL]; field_simp

/-- The meeting point at x, from the centre. -/
theorem meet_sub (c : P) (x : ℝ) : ((x, 0) : P) - c = (x - c.1, -c.2) := by
  ext <;> simp

/-- **A decided arc piece**: no meeting with either radial piece, the circle's arc met exactly
the counted number of times, no end on the ray, and the ray's line not tangent to the circle. -/
theorem arcDec_sound {c a b : P} {r g : ℝ} {n : ℕ} (hr : 0 < r) (ha : 0 < nsq (a - c))
    (hb : 0 < nsq (b - c)) (hga : |√(nsq (a - c)) - r| ≤ g) (hgb : |√(nsq (b - c)) - r| ≤ g)
    (hab : 0 < cross (a - c) (b - c)) (h : arcDec c a b r g = some n) :
    (ray ∩ seg a (onCircle c a r)).ncard = 0 ∧
    (ray ∩ DistArc.arc c (a - c) (b - c) r).ncard = n ∧
    (ray ∩ seg (onCircle c b r) b).ncard = 0 ∧
    a ∉ ray ∧ b ∉ ray ∧ onCircle c a r ∉ ray ∧ onCircle c b r ∉ ray ∧ r ^ 2 ≠ c.2 ^ 2 := by
  classical
  have hg0 : 0 ≤ g := (abs_nonneg _).trans hga
  unfold arcDec at h
  by_cases hn : near a g ∨ near b g
  · rw [if_pos hn] at h; exact absurd h (by simp)
  rw [if_neg hn] at h
  push Not at hn
  have hna : ¬ near a g := fun hh => hn.1 hh
  have hnb : ¬ near b g := fun hh => hn.2 hh
  have endNot : ∀ e : P, ¬ near e g → e ∉ ray := fun e hne he =>
    hne ⟨by rw [he.2]; linarith, by rw [he.2]; exact hg0, by linarith [he.1]⟩
  have rad_a : (ray ∩ seg a (onCircle c a r)).ncard = 0 :=
    ncard_zero_of fun v ⟨hv, hs⟩ => radial_misses ha hga hna (Or.inl hs) hv
  have rad_b : (ray ∩ seg (onCircle c b r) b).ncard = 0 :=
    ncard_zero_of fun v ⟨hv, hs⟩ => radial_misses hb hgb hnb (Or.inr hs) hv
  have ona := onCircle_nsq (e := a) hr.le ha
  have onb := onCircle_nsq (e := b) hr.le hb
  split_ifs at h with h2 h3
  · -- the line misses the circle
    obtain rfl := (Option.some.inj h).symm
    have miss : ∀ v, v ∈ ray → nsq (v - c) = r ^ 2 → False := fun v hv hc =>
      absurd (ray_circle hv.2 hc).1 (not_le.mpr h2)
    refine ⟨rad_a, ncard_zero_of fun v ⟨hv, hs⟩ => miss v hv hs.1, rad_b, endNot a hna,
      endNot b hnb, fun hv => miss _ hv ona, fun hv => miss _ hv onb, ?_⟩
    intro he; rw [he] at h2; simp at h2
  · -- two meetings
    push Not at h2
    have hpos : 0 < r ^ 2 - c.2 ^ 2 := lt_of_le_of_ne h2 (Ne.symm h3)
    obtain ⟨i, j, hi, hj, rfl⟩ := addO_some h
    set s := √(r ^ 2 - c.2 ^ 2) with hs
    have hs0 : 0 < s := Real.sqrt_pos.mpr hpos
    have hs2 : s ^ 2 = r ^ 2 - c.2 ^ 2 := Real.sq_sqrt hpos.le
    set Pp : P := (c.1 + s, 0)
    set Pm : P := (c.1 - s, 0)
    have hne : Pp ≠ Pm := by
      intro he; have := congrArg Prod.fst he; simp [Pp, Pm] at this; linarith
    have hcirc : ∀ x : ℝ, (x = c.1 + s ∨ x = c.1 - s) → nsq (((x, 0) : P) - c) = r ^ 2 := by
      rintro x (rfl | rfl) <;> simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub] <;> nlinarith
    have hset : ray ∩ DistArc.arc c (a - c) (b - c) r =
        {v | (v = Pp ∨ v = Pm) ∧ (0 ≤ v.1 ∧ inSector (a - c) (b - c) (v - c))} := by
      ext v
      simp only [mem_inter_iff, mem_setOf_eq, DistArc.arc]
      constructor
      · rintro ⟨hv, hc, hsec⟩
        exact ⟨(ray_circle hv.2 hc).2, hv.1, hsec⟩
      · rintro ⟨hpm, hx, hsec⟩
        have hy : v.2 = 0 := by rcases hpm with rfl | rfl <;> rfl
        refine ⟨⟨hx, hy⟩, ?_, hsec⟩
        have hv : v = (v.1, 0) := by ext <;> simp [hy]
        rw [hv]; apply hcirc
        rcases hpm with rfl | rfl
        · left; rfl
        · right; rfl
    have ci := meetDec_sound hi
    have cj := meetDec_sound hj
    refine ⟨rad_a, ?_, rad_b, endNot a hna, endNot b hnb, ?_, ?_, ?_⟩
    · rw [hset, ncard_two hne, ci, cj]
      simp only [Pp, Pm, meet_sub]
    · -- the circle end a* on the ray would be a meeting whose sector test is undecided
      intro hv
      have hk : 0 < r / √(nsq (a - c)) := div_pos hr (Real.sqrt_pos.mpr ha)
      rcases (ray_circle hv.2 ona).2 with he | he
      · have : meetDec c a b (c.1 + s) = none := by
          unfold meetDec
          have hvv : ((c.1 + s - c.1, -c.2) : P) = sc (r / √(nsq (a - c))) (a - c) := by
            rw [← meet_sub, ← he, onCircle_sub]
          rw [hvv]
          have e1 : cross (a - c) (sc (r / √(nsq (a - c))) (a - c)) = 0 := by
            simp only [cross, sc]; ring
          have e2 : cross (sc (r / √(nsq (a - c))) (a - c)) (b - c) =
              r / √(nsq (a - c)) * cross (a - c) (b - c) := by simp only [cross, sc]; ring
          rw [e1, e2]
          simp [(mul_pos hk hab).not_gt]
        rw [this] at hi; exact absurd hi (by simp)
      · have : meetDec c a b (c.1 - s) = none := by
          unfold meetDec
          have hvv : ((c.1 - s - c.1, -c.2) : P) = sc (r / √(nsq (a - c))) (a - c) := by
            rw [← meet_sub, ← he, onCircle_sub]
          rw [hvv]
          have e1 : cross (a - c) (sc (r / √(nsq (a - c))) (a - c)) = 0 := by
            simp only [cross, sc]; ring
          have e2 : cross (sc (r / √(nsq (a - c))) (a - c)) (b - c) =
              r / √(nsq (a - c)) * cross (a - c) (b - c) := by simp only [cross, sc]; ring
          rw [e1, e2]
          simp [(mul_pos hk hab).not_gt]
        rw [this] at hj; exact absurd hj (by simp)
    · intro hv
      have hk : 0 < r / √(nsq (b - c)) := div_pos hr (Real.sqrt_pos.mpr hb)
      have e1 : cross (a - c) (sc (r / √(nsq (b - c))) (b - c)) =
          r / √(nsq (b - c)) * cross (a - c) (b - c) := by simp only [cross, sc]; ring
      have e2 : cross (sc (r / √(nsq (b - c))) (b - c)) (b - c) = 0 := by
        simp only [cross, sc]; ring
      rcases (ray_circle hv.2 onb).2 with he | he
      · have : meetDec c a b (c.1 + s) = none := by
          unfold meetDec
          have hvv : ((c.1 + s - c.1, -c.2) : P) = sc (r / √(nsq (b - c))) (b - c) := by
            rw [← meet_sub, ← he, onCircle_sub]
          rw [hvv, e1, e2]
          simp [(mul_pos hk hab).not_gt]
        rw [this] at hi; exact absurd hi (by simp)
      · have : meetDec c a b (c.1 - s) = none := by
          unfold meetDec
          have hvv : ((c.1 - s - c.1, -c.2) : P) = sc (r / √(nsq (b - c))) (b - c) := by
            rw [← meet_sub, ← he, onCircle_sub]
          rw [hvv, e1, e2]
          simp [(mul_pos hk hab).not_gt]
        rw [this] at hj; exact absurd hj (by simp)
    · intro he; rw [he] at hpos; simp at hpos


/-! ### The whole trim: `ray_side`, the even-odd rule, `classify` -/

def Piece.dec : Piece → Option ℕ
  | .line a b => lineDec a b
  | .arc c a b r g => arcDec c a b r g

/-- What the trim's construction guarantees of an arc piece (787239e). -/
def Piece.ok : Piece → Prop
  | .line _ _ => True
  | .arc c a b r g => 0 < r ∧ 0 < nsq (a - c) ∧ 0 < nsq (b - c) ∧
      |√(nsq (a - c)) - r| ≤ g ∧ |√(nsq (b - c)) - r| ≤ g ∧ 0 < cross (a - c) (b - c)

def Piece.ends : Piece → Set P
  | .line a b => {a, b}
  | .arc c a b r _ => {a, onCircle c a r, onCircle c b r, b}

def Piece.tangent : Piece → Prop
  | .line _ _ => False
  | .arc c _ _ r _ => r ^ 2 = c.2 ^ 2

/-- A generic ray: it meets no piece's end and touches no arc's circle. -/
def Generic (ps : List Piece) : Prop := ∀ p ∈ ps, (∀ e ∈ p.ends, e ∉ ray) ∧ ¬ p.tangent

/-- `ray_side`: the parity of the counts, if every piece decides. -/
def raySide (ps : List Piece) : Option Bool :=
  (ps.mapM Piece.dec).map fun ns => decide (ns.sum % 2 = 1)

theorem Piece.dec_sound {p : Piece} (hok : p.ok) {n : ℕ} (h : p.dec = some n) :
    p.meets = n ∧ (∀ e ∈ p.ends, e ∉ ray) ∧ ¬ p.tangent := by
  cases p with
  | line a b =>
    obtain ⟨h1, h2, h3⟩ := lineDec_sound h
    refine ⟨h1, ?_, fun h => h⟩
    intro e he; rcases he with rfl | he
    · exact h2
    · rw [mem_singleton_iff.mp he]; exact h3
  | arc c a b r g =>
    obtain ⟨hr, ha, hb, hga, hgb, hab⟩ := hok
    obtain ⟨r1, r2, r3, e1, e2, e3, e4, ht⟩ := arcDec_sound hr ha hb hga hgb hab h
    refine ⟨by simp only [Piece.meets, r1, r2, r3]; ring, ?_, ht⟩
    intro e he
    simp only [Piece.ends, mem_insert_iff, mem_singleton_iff] at he
    rcases he with rfl | rfl | rfl | rfl
    · exact e1
    · exact e3
    · exact e4
    · exact e2

theorem mapM_sound {ps : List Piece} (hok : ∀ p ∈ ps, p.ok) {ns : List ℕ}
    (h : ps.mapM Piece.dec = some ns) :
    Generic ps ∧ ns.sum = (ps.map Piece.meets).sum := by
  induction ps generalizing ns with
  | nil =>
    simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at h
    subst h; exact ⟨fun p hp => absurd hp (List.not_mem_nil), rfl⟩
  | cons p ps ih =>
    simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at h
    obtain ⟨n, hn, ns', hns', rfl⟩ := h
    obtain ⟨hm, he, ht⟩ := Piece.dec_sound (hok p List.mem_cons_self) hn
    obtain ⟨hg, hs⟩ := ih (fun q hq => hok q (List.mem_cons_of_mem _ hq)) hns'
    refine ⟨?_, by simp [hs, hm]⟩
    intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · exact ⟨he, ht⟩
    · exact hg q hq

/-- **`ray_side` is sound**: when every piece decides, the ray is generic and the parity it
reports is the parity of its true meetings with the trim (radial pieces included). -/
theorem raySide_sound {ps : List Piece} (hok : ∀ p ∈ ps, p.ok) {b : Bool}
    (h : raySide ps = some b) :
    Generic ps ∧ (b = true ↔ (ps.map Piece.meets).sum % 2 = 1) := by
  unfold raySide at h
  obtain ⟨ns, hns, rfl⟩ := Option.map_eq_some_iff.mp h
  obtain ⟨hg, hs⟩ := mapM_sound hok hns
  exact ⟨hg, by rw [← hs]; simp⟩

/-- The even-odd rule: the meaning of a region bounded by its trim (a Jordan-curve fact,
assumed): the query point (the origin of the ray's frame) is in the region iff a generic ray
meets the trim an odd number of times. -/
def EvenOdd (R : Set P) (ps : List Piece) : Prop :=
  Generic ps → ((0 : P) ∈ R ↔ (ps.map Piece.meets).sum % 2 = 1)

/-- **One direction decides correctly.** -/
theorem raySide_classifies {R : Set P} {ps : List Piece} (hok : ∀ p ∈ ps, p.ok)
    (heo : EvenOdd R ps) {b : Bool} (h : raySide ps = some b) : ((0 : P) ∈ R ↔ b = true) := by
  obtain ⟨hg, hb⟩ := raySide_sound hok h
  rw [heo hg, hb]

/-- `point_side`: the directions tried in turn (each its ray's frame: the region and the pieces
turned about the query point), the first that decides answers. -/
def pointSide : List (List Piece) → Option Bool
  | [] => none
  | ps :: rest => match raySide ps with
    | some b => some b
    | none => pointSide rest

/-- **`classify` on a point is sound**: if a direction decides, its answer is whether the point
is in the region. Each frame's region is the face's region turned about the point, so the point
is in it iff it is in the face's (`hframe`). -/
theorem pointSide_sound {q : Prop} :
    ∀ (frames : List (Set P × List Piece)),
      (∀ f ∈ frames, (∀ p ∈ f.2, p.ok) ∧ EvenOdd f.1 f.2 ∧ ((0 : P) ∈ f.1 ↔ q)) →
      ∀ {b : Bool}, pointSide (frames.map Prod.snd) = some b → (q ↔ b = true)
  | [], _, _, h => by simp [pointSide] at h
  | f :: rest, hf, b, h => by
    obtain ⟨hok, heo, hq⟩ := hf f List.mem_cons_self
    simp only [List.map_cons, pointSide] at h
    cases hr : raySide f.2 with
    | some b' =>
      rw [hr] at h; simp only [Option.some.injEq] at h; subst h
      rw [← hq]; exact raySide_classifies hok heo hr
    | none =>
      rw [hr] at h
      exact pointSide_sound rest (fun g hg => hf g (List.mem_cons_of_mem _ hg)) h

/-! ### `dist_region`: the distance from a path to a flat face's region -/

theorem d2_comm (x y : P) : d2 x y = d2 y x := by
  simp only [d2]; ring_nf

theorem d2_self (x : P) : d2 x x = 0 := by simp [d2]

theorem d2_nonneg (x y : P) : 0 ≤ d2 x y := Real.sqrt_nonneg _

/-- Along a segment from σ to x, the distance to x shrinks. -/
theorem d2_le_of_mem_segment {x σ f : P} (hf : f ∈ segment ℝ σ x) : d2 x f ≤ d2 x σ := by
  obtain ⟨s, t, hs, ht, hst, rfl⟩ := hf
  simp only [d2, Prod.fst_add, Prod.snd_add, Prod.smul_fst, Prod.smul_snd, smul_eq_mul]
  apply Real.sqrt_le_sqrt
  have : t = 1 - s := by linarith
  subst this
  nlinarith [sq_nonneg (x.1 - σ.1), sq_nonneg (x.2 - σ.2), mul_nonneg hs hs,
    mul_nonneg hs ht, mul_nonneg ht ht]

/-- **`dist_region` is sound.** The path starts outside the region (`classify` said so); the
region's frontier lies in the trim's pieces C; L bounds the distance from every path point to
every point of C. Then L bounds the distance from every path point to every point of the
region (ClearFace (i) with S the region). -/
theorem distRegion_lower {R C : Set P} (hfr : frontier R ⊆ C) {γ : ℝ → P}
    (hγ : ContinuousOn γ I01) (hstart : γ 0 ∉ R) {L : ℝ}
    (hL : ∀ s ∈ I01, ∀ y ∈ C, L ≤ d2 y (γ s)) :
    ∀ s ∈ I01, ∀ σ ∈ R, L ≤ d2 σ (γ s) := by
  intro s hs σ hσ
  by_cases hin : γ s ∈ R
  · -- the path enters the region: it crosses the frontier, where L ≤ 0
    have hpre : IsPreconnected (γ '' Icc 0 s) :=
      (isPreconnected_Icc.image _ (hγ.mono (Icc_subset_Icc le_rfl hs.2)))
    have h0 : γ 0 ∈ γ '' Icc 0 s := ⟨0, ⟨le_rfl, hs.1⟩, rfl⟩
    have h1 : γ s ∈ γ '' Icc 0 s := ⟨s, ⟨hs.1, le_rfl⟩, rfl⟩
    obtain ⟨f, hf, hfr'⟩ : (γ '' Icc 0 s ∩ frontier R).Nonempty := by
      by_contra hno
      rw [not_nonempty_iff_eq_empty] at hno
      have hsplit : γ '' Icc 0 s ⊆ interior R ∪ interior Rᶜ := by
        intro z hz
        by_cases hzi : z ∈ interior R
        · exact Or.inl hzi
        · right; rw [interior_compl]
          intro hzc
          exact (eq_empty_iff_forall_notMem.mp hno) z ⟨hz, hzc, hzi⟩
      have hd : Disjoint (interior R) (interior Rᶜ) :=
        Set.disjoint_left.mpr fun z h1 h2 => (interior_subset h2 : z ∈ Rᶜ) (interior_subset h1)
      have hout : γ 0 ∈ interior Rᶜ := by
        rcases hsplit h0 with h | h
        · exact absurd (interior_subset h) hstart
        · exact h
      have := hpre.subset_right_of_subset_union isOpen_interior isOpen_interior hd hsplit
        ⟨γ 0, h0, hout⟩
      exact (interior_subset (this h1) : γ s ∈ Rᶜ) hin
    obtain ⟨s', hs', rfl⟩ := hf
    have := hL s' ⟨hs'.1, hs'.2.trans hs.2⟩ _ (hfr hfr')
    rw [d2_self] at this
    exact this.trans (d2_nonneg _ _)
  · -- the nearest region point along the segment is a frontier point
    obtain ⟨f, hf, hseg⟩ := exists_frontier_mem_segment (B := R) hσ hin
    have h1 := hL s hs f (hfr hf)
    have h2 := d2_le_of_mem_segment hseg
    calc L ≤ d2 f (γ s) := h1
      _ = d2 (γ s) f := d2_comm _ _
      _ ≤ d2 (γ s) σ := h2
      _ = d2 σ (γ s) := d2_comm _ _

/-! ### `tseg_dist`: an arc's bound less the gap bounds its radial pieces -/

theorem d2_triangle (x y z : P) : d2 x z ≤ d2 x y + d2 y z := by
  have h := dot_le_sqrt (x - y) (y - z) (Real.sqrt_nonneg (nsq (y - z)))
    (Real.sq_sqrt (nsq_nonneg _)).symm
  have hx : d2 x z = √(nsq (x - z)) := by simp only [d2, nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring_nf
  have hy : d2 x y = √(nsq (x - y)) := by simp only [d2, nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring_nf
  have hz : d2 y z = √(nsq (y - z)) := by simp only [d2, nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring_nf
  rw [hx, hy, hz]
  rw [Real.sqrt_le_left (by positivity)]
  have e : nsq (x - z) = nsq (x - y) + nsq (y - z) + 2 * dot (x - y) (y - z) := by
    simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
  rw [e, add_sq, Real.sq_sqrt (nsq_nonneg _), Real.sq_sqrt (nsq_nonneg _)]
  nlinarith

/-- **The radial pieces are within the gap of the arc**: a bound L on the distance from x to
the circle's arc gives L − g for both radial pieces (`tseg_dist` subtracts the gap). -/
theorem radial_lower {c a b x : P} {r g L : ℝ} (hr : 0 < r) (ha : 0 < nsq (a - c))
    (hga : |√(nsq (a - c)) - r| ≤ g) (hab : 0 < cross (a - c) (b - c))
    (hL : ∀ y ∈ DistArc.arc c (a - c) (b - c) r, L ≤ d2 y x) :
    ∀ y ∈ seg a (onCircle c a r), L - g ≤ d2 y x := by
  intro y hy
  set e := onCircle c a r
  have he : e ∈ DistArc.arc c (a - c) (b - c) r := by
    refine ⟨onCircle_nsq hr.le ha, ?_, ?_⟩
    · rw [onCircle_sub]; simp only [cross, sc]; ring_nf; exact le_refl _
    · rw [onCircle_sub]
      have hk : 0 < r / √(nsq (a - c)) := div_pos hr (Real.sqrt_pos.mpr ha)
      have : cross (sc (r / √(nsq (a - c))) (a - c)) (b - c) =
          r / √(nsq (a - c)) * cross (a - c) (b - c) := by simp only [cross, sc]; ring
      rw [this]; exact (mul_pos hk hab).le
  have h1 := hL e he
  -- |y − e| ≤ |a − e| = ||a − c| − r| ≤ g
  obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hy
  have hae : d2 a e = |√(nsq (a - c)) - r| := by
    set Lc := √(nsq (a - c)) with hLd
    have hL0 : 0 < Lc := Real.sqrt_pos.mpr ha
    have hL2 : Lc ^ 2 = nsq (a - c) := Real.sq_sqrt ha.le
    simp only [d2, e, onCircle, sc, Prod.fst_add, Prod.snd_add, ← hLd]
    rw [← Real.sqrt_sq_eq_abs]; congr 1
    have : (a.1 - (c.1 + r / Lc * (a - c).1)) ^ 2 + (a.2 - (c.2 + r / Lc * (a - c).2)) ^ 2 =
        (1 - r / Lc) ^ 2 * nsq (a - c) := by
      simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
    rw [this, ← hL2]; field_simp
  have hye : d2 (a.1 + τ * (e.1 - a.1), a.2 + τ * (e.2 - a.2)) e = (1 - τ) * d2 a e := by
    simp only [d2]
    rw [show (a.1 + τ * (e.1 - a.1) - e.1) ^ 2 + (a.2 + τ * (e.2 - a.2) - e.2) ^ 2 =
      (1 - τ) ^ 2 * ((a.1 - e.1) ^ 2 + (a.2 - e.2) ^ 2) by ring, Real.sqrt_mul (by positivity),
      Real.sqrt_sq (by linarith)]
  have ht := d2_triangle e (a.1 + τ * (e.1 - a.1), a.2 + τ * (e.2 - a.2)) x
  have hye' : d2 e (a.1 + τ * (e.1 - a.1), a.2 + τ * (e.2 - a.2)) = (1 - τ) * d2 a e := by
    rw [d2_comm]; exact hye
  rw [hye', hae] at ht
  have : (1 - τ) * |√(nsq (a - c)) - r| ≤ g :=
    (mul_le_of_le_one_left (abs_nonneg _) (by linarith)).trans hga
  linarith


theorem seg_symm (p q : P) : seg p q = seg q p := by
  ext v; simp only [mem_seg]
  constructor <;> rintro ⟨τ, h0, h1, rfl⟩ <;> refine ⟨1 - τ, by linarith, by linarith, ?_⟩ <;>
    ext <;> simp <;> ring

/-- The same at the arc's other end (the radial piece from the circle to b). -/
theorem radial_lower_end {c a b x : P} {r g L : ℝ} (hr : 0 < r) (hb : 0 < nsq (b - c))
    (hgb : |√(nsq (b - c)) - r| ≤ g) (hab : 0 < cross (a - c) (b - c))
    (hL : ∀ y ∈ DistArc.arc c (a - c) (b - c) r, L ≤ d2 y x) :
    ∀ y ∈ seg (onCircle c b r) b, L - g ≤ d2 y x := by
  -- the circle end b* is on the arc; the rest as for a, mirrored
  have hk : 0 < r / √(nsq (b - c)) := div_pos hr (Real.sqrt_pos.mpr hb)
  have hbe : onCircle c b r ∈ DistArc.arc c (a - c) (b - c) r := by
    refine ⟨onCircle_nsq hr.le hb, ?_, ?_⟩
    · rw [onCircle_sub]
      have : cross (a - c) (sc (r / √(nsq (b - c))) (b - c)) =
          r / √(nsq (b - c)) * cross (a - c) (b - c) := by simp only [cross, sc]; ring
      rw [this]; exact (mul_pos hk hab).le
    · rw [onCircle_sub]; simp only [cross, sc]; ring_nf; exact le_refl _
  intro y hy
  rw [seg_symm] at hy
  set e := onCircle c b r
  have h1 := hL e hbe
  obtain ⟨τ, h0, h1', rfl⟩ := mem_seg.mp hy
  have hae : d2 b e = |√(nsq (b - c)) - r| := by
    set Lc := √(nsq (b - c)) with hLd
    have hL0 : 0 < Lc := Real.sqrt_pos.mpr hb
    have hL2 : Lc ^ 2 = nsq (b - c) := Real.sq_sqrt hb.le
    simp only [d2, e, onCircle, sc, Prod.fst_add, Prod.snd_add, ← hLd]
    rw [← Real.sqrt_sq_eq_abs]; congr 1
    have : (b.1 - (c.1 + r / Lc * (b - c).1)) ^ 2 + (b.2 - (c.2 + r / Lc * (b - c).2)) ^ 2 =
        (1 - r / Lc) ^ 2 * nsq (b - c) := by
      simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
    rw [this, ← hL2]; field_simp
  have hye : d2 e (b.1 + τ * (e.1 - b.1), b.2 + τ * (e.2 - b.2)) = (1 - τ) * d2 b e := by
    rw [d2_comm]; simp only [d2]
    rw [show (b.1 + τ * (e.1 - b.1) - e.1) ^ 2 + (b.2 + τ * (e.2 - b.2) - e.2) ^ 2 =
      (1 - τ) ^ 2 * ((b.1 - e.1) ^ 2 + (b.2 - e.2) ^ 2) by ring, Real.sqrt_mul (by positivity),
      Real.sqrt_sq (by linarith)]
  have ht := d2_triangle e (b.1 + τ * (e.1 - b.1), b.2 + τ * (e.2 - b.2)) x
  rw [hye, hae] at ht
  have : (1 - τ) * |√(nsq (b - c)) - r| ≤ g :=
    (mul_le_of_le_one_left (abs_nonneg _) (by linarith)).trans hgb
  linarith

/-- **A trim arc piece's bound**: a lower bound L on the distance from x to its circle's arc
gives L − g for the whole piece (both radial pieces), as `tseg_dist` computes it. -/
theorem arcPiece_lower {c a b x : P} {r g L : ℝ} (hok : (Piece.arc c a b r g).ok)
    (hL : ∀ y ∈ DistArc.arc c (a - c) (b - c) r, L ≤ d2 y x) :
    ∀ y ∈ (Piece.arc c a b r g).pts, L - g ≤ d2 y x := by
  obtain ⟨hr, ha, hb, hga, hgb, hab⟩ := hok
  have hg0 : 0 ≤ g := (abs_nonneg _).trans hga
  intro y hy
  rcases hy with (hy | hy) | hy
  · exact radial_lower hr ha hga hab hL y hy
  · linarith [hL y hy]
  · exact radial_lower_end hr hb hgb hab hL y hy


/-- **ClearFace (i) for a flat face** (`dist_region`): the move's path starts outside the face's
region (classified so), the region's frontier lies in its trim's pieces, and D ≥ 0 bounds the
distance from the path to every piece. Then D is what ClearFace (i) asks for, with S the region.
(D must not be negative: the closed form's monotonicity is in ρ ≥ 0.) -/
theorem clearFace_region {Z : Set ℝ} {w : ℝ → ℝ} {γ : ℝ → P} (hγ : ContinuousOn γ I01)
    {R : Set P} {ps : List Piece} (hfr : frontier R ⊆ ⋃ p ∈ ps, p.pts) (hstart : γ 0 ∉ R)
    {D : ℝ} (hD0 : 0 ≤ D) (hD : ∀ p ∈ ps, ∀ s ∈ I01, ∀ y ∈ p.pts, D ≤ d2 y (γ s))
    {Zt H : Set ℝ} {lower : ℝ} (hΔ : ∀ h ∈ H, ∀ z ∈ Zt, lower ≤ sdRZ Z w D (h - z)) :
    ClearFace Z w γ Zt R H D lower := by
  refine ⟨hD0, fun s hs σ hσ => ?_, hΔ⟩
  refine distRegion_lower hfr hγ hstart (fun s hs y hy => ?_) s hs σ hσ
  obtain ⟨p, hp, hy⟩ := mem_iUnion₂.mp hy
  exact hD p hp s hs y hy

end

end ForgeCam.Trim
