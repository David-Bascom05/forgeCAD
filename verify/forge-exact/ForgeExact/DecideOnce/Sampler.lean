import ForgeExact.DecideOnce.Sample

/-!
# A complete sampler: (c) for every flat input, with no hypothesis on the samples

`sampler ps` lists, in exact rationals, points that hit every cell of the arrangement lying in a
carrier (`sampler_covers`: `Sample.Covers`). With `Sample.sample_exact` this makes `Exact` and (c) theorems for
every flat input (`flat_exact`, `flat_correct`). The hypotheses are nonzero carrier normals and
`FlatInput` only.

The route is a slab sweep, one carrier at a time:
* **1D** (`cand1`, `cand1_ok`): for any root list, the roots, each root ± 1, each root's midpoint
  with the next root above, and 0 give, for every `t`, a candidate on the same side of every root;
* **2D** (`S2`, `cover2`): on a carrier, the others are lines `α + β u + γ v`. Take the critical `u`
  (vertical lines' roots, crossings of non-vertical lines, `crit`) and their 1D candidates. Between
  two critical values the lines keep their order (`aff_same`), so a point's place among the lines
  carries to the sampled vertical line (`emb`). On that line, the 1D candidates of the lines'
  heights give a sample with every sign;
* **3D** (`lift`, `proj`, `coef`): each carrier is a graph over two coordinates. Every other
  carrier restricted to it is affine (`coef_ev`), and every point of it is the lift of its
  projection (`lift_proj`).
-/

namespace ForgeExact.DecideOnce.Sampler

open ForgeExact ForgeExact.Ref

/-! ## One dimension: a point on every side of every root -/

theorem sgn_sub_eq {t t' r : Rat} (h1 : r < t → r < t') (h2 : t < r → t' < r) (h3 : t ≠ r) :
    sgnQ (t' - r) = sgnQ (t - r) := by
  by_cases a : r < t
  · have := h1 a
    rw [(sgnQ_pos_iff _).mpr (by grind), (sgnQ_pos_iff _).mpr (by grind)]
  · have b : t < r := by grind
    have := h2 b
    rw [(sgnQ_neg_iff _).mpr (by grind), (sgnQ_neg_iff _).mpr (by grind)]

/-- The least root above `r`. -/
def minAbove (r : Rat) : List Rat → Option Rat
  | [] => none
  | s :: l => match minAbove r l with
    | none => if r < s then some s else none
    | some a => if r < s ∧ s < a then some s else some a

theorem minAbove_some (r : Rat) : ∀ (l : List Rat) (m : Rat), minAbove r l = some m →
    m ∈ l ∧ r < m ∧ ∀ s ∈ l, r < s → m ≤ s
  | [], m, h => by simp [minAbove] at h
  | s :: l, m, h => by
    unfold minAbove at h
    cases hl : minAbove r l with
    | none =>
      rw [hl] at h
      simp only at h
      split at h
      · cases h
        have hn := minAbove_none r l hl
        refine ⟨List.mem_cons_self .., by assumption, fun x hx hrx => ?_⟩
        rcases List.mem_cons.mp hx with rfl | hx
        · exact Rat.le_refl
        · exact absurd hrx (hn x hx)
      · cases h
    | some a =>
      rw [hl] at h
      simp only at h
      obtain ⟨ha, hra, hmin⟩ := minAbove_some r l a hl
      split at h
      · cases h
        rename_i hc
        refine ⟨List.mem_cons_self .., hc.1, fun x hx hrx => ?_⟩
        rcases List.mem_cons.mp hx with rfl | hx
        · exact Rat.le_refl
        · have := hmin x hx hrx; grind
      · cases h
        rename_i hc
        refine ⟨List.mem_cons_of_mem _ ha, hra, fun x hx hrx => ?_⟩
        rcases List.mem_cons.mp hx with rfl | hx
        · grind
        · exact hmin x hx hrx
where
  minAbove_none (r : Rat) : ∀ (l : List Rat), minAbove r l = none → ∀ s ∈ l, ¬r < s
    | [], _, s, hs => by simp at hs
    | s :: l, h, x, hx => by
      unfold minAbove at h
      cases hl : minAbove r l with
      | none =>
        rw [hl] at h
        simp only at h
        split at h
        · cases h
        · rcases List.mem_cons.mp hx with rfl | hx
          · assumption
          · exact minAbove_none r l hl x hx
      | some a => rw [hl] at h; simp only at h; split at h <;> cases h

theorem minAbove_none (r : Rat) (l : List Rat) (h : minAbove r l = none) : ∀ s ∈ l, ¬r < s :=
  minAbove_some.minAbove_none r l h

/-- The 1D candidates of a list of roots. -/
def cand1 (rs : List Rat) : List Rat :=
  rs ++ rs.map (· + 1) ++ rs.map (· - 1) ++
    rs.filterMap (fun r => (minAbove r rs).map fun s => (r + s) / 2) ++ [0]

theorem exists_max {α : Type} (f : α → Rat) : ∀ l : List α, l ≠ [] → ∃ a ∈ l, ∀ b ∈ l, f b ≤ f a
  | [], h => absurd rfl h
  | [a], _ => ⟨a, List.mem_cons_self .., fun b hb => by simp at hb; subst hb; exact Rat.le_refl⟩
  | a :: b :: l, _ => by
    obtain ⟨m, hm, hmax⟩ := exists_max f (b :: l) (by simp)
    by_cases h : f a ≤ f m
    · exact ⟨m, List.mem_cons_of_mem _ hm, fun x hx => by
        rcases List.mem_cons.mp hx with rfl | hx
        · exact h
        · exact hmax x hx⟩
    · exact ⟨a, List.mem_cons_self .., fun x hx => by
        rcases List.mem_cons.mp hx with rfl | hx
        · exact Rat.le_refl
        · have := hmax x hx; grind⟩

theorem exists_min {α : Type} (f : α → Rat) (l : List α) (h : l ≠ []) : ∃ a ∈ l, ∀ b ∈ l, f a ≤ f b := by
  obtain ⟨a, ha, hm⟩ := exists_max (fun x => -f x) l h
  exact ⟨a, ha, fun b hb => by have := hm b hb; grind⟩

/-- **Every point has a candidate on the same side of every root.** -/
theorem cand1_ok (rs : List Rat) (t : Rat) :
    ∃ t' ∈ cand1 rs, ∀ r ∈ rs, sgnQ (t' - r) = sgnQ (t - r) := by
  unfold cand1
  simp only [List.mem_append, List.mem_map, List.mem_filterMap, Option.map_eq_some_iff,
    List.mem_cons, List.mem_nil_iff, or_false]
  by_cases ht : t ∈ rs
  · exact ⟨t, Or.inl (Or.inl (Or.inl (Or.inl ht))), fun _ _ => rfl⟩
  have hne : ∀ r ∈ rs, t ≠ r := fun r hr h => ht (h ▸ hr)
  by_cases hb : rs.filter (· < t) = []
  · have hall : ∀ r ∈ rs, t < r := by
      intro r hr
      have : ¬r < t := by
        intro h
        have : r ∈ rs.filter (· < t) := List.mem_filter.mpr ⟨hr, by simpa using h⟩
        rw [hb] at this; simp at this
      have := hne r hr; grind
    by_cases hr0 : rs = []
    · subst hr0; exact ⟨0, Or.inr rfl, fun r hr => by simp at hr⟩
    · obtain ⟨U, hU, hmin⟩ := exists_min id rs hr0
      refine ⟨U - 1, Or.inl (Or.inl (Or.inr ⟨U, hU, rfl⟩)), fun r hr => ?_⟩
      have h1 := hmin r hr; have h2 := hall r hr
      simp only [id] at h1
      exact sgn_sub_eq (fun h => by grind) (fun _ => by grind) (hne r hr)
  · obtain ⟨L, hL, hmax⟩ := exists_max id _ hb
    have hL' := List.mem_filter.mp hL
    simp only [decide_eq_true_eq] at hL'
    have below : ∀ r ∈ rs, r < t → r ≤ L := fun r hr h =>
      hmax r (List.mem_filter.mpr ⟨hr, by simpa using h⟩)
    cases hm : minAbove L rs with
    | none =>
      have hn := minAbove_none L rs hm
      refine ⟨L + 1, Or.inl (Or.inl (Or.inl (Or.inr ⟨L, hL'.1, rfl⟩))), fun r hr => ?_⟩
      have h0 := hn r hr
      have : r < t := by
        have := hne r hr
        by_cases h : r < t
        · exact h
        · exfalso; apply h0; grind
      have := below r hr this
      exact sgn_sub_eq (fun _ => by grind) (fun h => by grind) (hne r hr)
    | some U =>
      obtain ⟨hU, hLU, hmin⟩ := minAbove_some L rs U hm
      have htU : t < U := by
        have := hne U hU
        by_cases h : U < t
        · have := below U hU h; grind
        · grind
      refine ⟨(L + U) / 2, Or.inl (Or.inr ⟨L, hL'.1, U, hm, rfl⟩), fun r hr => ?_⟩
      apply sgn_sub_eq _ _ (hne r hr)
      · intro h
        have := below r hr h
        grind
      · intro h
        have := hmin r hr (by grind)
        grind

/-! ## Signs -/

theorem sgn_eq_of {q q' : Rat} (h1 : 0 < q ↔ 0 < q') (h2 : q < 0 ↔ q' < 0) : sgnQ q = sgnQ q' := by
  unfold sgnQ
  by_cases a : q > 0
  · rw [if_pos a, if_pos (h1.mp a)]
  · rw [if_neg a, if_neg (fun h => a (h1.mpr h))]
    by_cases b : q < 0
    · rw [if_pos b, if_pos (h2.mp b)]
    · rw [if_neg b, if_neg (fun h => b (h2.mpr h))]

theorem sgn_pos_of {q q' : Rat} (h : sgnQ q = sgnQ q') : 0 < q ↔ 0 < q' := by
  have a := sgnQ_pos_iff q; have b := sgnQ_pos_iff q'
  rw [h] at a; exact a.symm.trans b

theorem sgn_neg_of {q q' : Rat} (h : sgnQ q = sgnQ q') : q < 0 ↔ q' < 0 := by
  have a := sgnQ_neg_iff q; have b := sgnQ_neg_iff q'
  rw [h] at a; exact a.symm.trans b

theorem sgnQ_mul_eq {B x y : Rat} (hB : B ≠ 0) (h : sgnQ x = sgnQ y) : sgnQ (B * x) = sgnQ (B * y) := by
  have p := sgn_pos_of h
  have n := sgn_neg_of h
  rcases Jordan.ne_cases hB with hb | hb
  · apply sgn_eq_of
    · rw [Jordan.mul_pos_iff_neg hb, Jordan.mul_pos_iff_neg hb]; exact n
    · have e1 : B * x < 0 ↔ 0 < B * (-x) := by grind
      have e2 : B * y < 0 ↔ 0 < B * (-y) := by grind
      rw [e1, e2, Jordan.mul_pos_iff_neg hb, Jordan.mul_pos_iff_neg hb]
      constructor
      · intro h'; have := p.mp (by grind); grind
      · intro h'; have := p.mpr (by grind); grind
  · apply sgn_eq_of
    · rw [Jordan.mul_pos_iff_left hb, Jordan.mul_pos_iff_left hb]; exact p
    · have e1 : B * x < 0 ↔ 0 < B * (-x) := by grind
      have e2 : B * y < 0 ↔ 0 < B * (-y) := by grind
      rw [e1, e2, Jordan.mul_pos_iff_left hb, Jordan.mul_pos_iff_left hb]
      constructor
      · intro h'; have := n.mp (by grind); grind
      · intro h'; have := n.mpr (by grind); grind

/-- The root of `A + B t`, if it has one. -/
def root1 (A B : Rat) : Option Rat := if B = 0 then none else some (-A / B)

/-- An affine function of one variable keeps its sign between two points on one side of its root. -/
theorem aff_same (A B t t' : Rat) (h : ∀ c, root1 A B = some c → sgnQ (t' - c) = sgnQ (t - c)) :
    sgnQ (A + B * t') = sgnQ (A + B * t) := by
  by_cases hB : B = 0
  · subst hB; simp
  · have hc := h (-A / B) (by simp [root1, hB])
    have e : ∀ s, A + B * s = B * (s - -A / B) := by
      intro s
      have := Rat.mul_inv_cancel B hB
      rw [Rat.div_def]; grind
    rw [e t', e t]
    exact sgnQ_mul_eq hB hc

/-! ## The order of points, carried to another line -/

theorem sgn_sub_pos {a b : Rat} (h : a < b) : sgnQ (b - a) = .pos := (sgnQ_pos_iff _).mpr (by grind)
theorem sgn_sub_neg {a b : Rat} (h : b < a) : sgnQ (b - a) = .neg := (sgnQ_neg_iff _).mpr (by grind)

/-- Points `r_j` and `r'_j` in the same order: a point placed among the `r` has a place among the
`r'` on the same side of each. -/
theorem emb (prs : List (Rat × Rat)) (hp : ∀ p ∈ prs, ∀ q ∈ prs, sgnQ (p.2 - q.2) = sgnQ (p.1 - q.1))
    (v : Rat) : ∃ v', ∀ p ∈ prs, sgnQ (v' - p.2) = sgnQ (v - p.1) := by
  by_cases hon : ∃ p ∈ prs, p.1 = v
  · obtain ⟨p, hp0, rfl⟩ := hon
    exact ⟨p.2, fun q hq => hp p hp0 q hq⟩
  have hne : ∀ p ∈ prs, p.1 ≠ v := fun p h e => hon ⟨p, h, e⟩
  -- signs from the order
  have ord : ∀ p ∈ prs, ∀ q ∈ prs, p.1 < q.1 → p.2 < q.2 := by
    intro p hp' q hq h
    have := hp p hp' q hq
    rw [sgn_sub_neg (show p.1 < q.1 from h)] at this
    have := (sgnQ_neg_iff _).mp this; grind
  by_cases hA : prs.filter (fun p => p.1 < v) = []
  · have hall : ∀ p ∈ prs, v < p.1 := by
      intro p hp'
      have : ¬p.1 < v := by
        intro h
        have : p ∈ prs.filter (fun p => p.1 < v) := List.mem_filter.mpr ⟨hp', by simpa using h⟩
        rw [hA] at this; simp at this
      have := hne p hp'; grind
    by_cases h0 : prs = []
    · subst h0; exact ⟨0, fun p hp' => by simp at hp'⟩
    · obtain ⟨b, hb, hmin⟩ := exists_min Prod.snd prs h0
      refine ⟨b.2 - 1, fun p hp' => ?_⟩
      have := hmin p hp'; have := hall p hp'
      rw [sgn_sub_neg (show b.2 - 1 < p.2 by grind), sgn_sub_neg (show v < p.1 by grind)]
  · obtain ⟨a, ha, hmax⟩ := exists_max Prod.snd _ hA
    have ha' := List.mem_filter.mp ha
    simp only [decide_eq_true_eq] at ha'
    have below : ∀ p ∈ prs, p.1 < v → p.2 ≤ a.2 := fun p hp' h =>
      hmax p (List.mem_filter.mpr ⟨hp', by simpa using h⟩)
    by_cases hB : prs.filter (fun p => v < p.1) = []
    · have hall : ∀ p ∈ prs, p.1 < v := by
        intro p hp'
        have : ¬v < p.1 := by
          intro h
          have : p ∈ prs.filter (fun p => v < p.1) := List.mem_filter.mpr ⟨hp', by simpa using h⟩
          rw [hB] at this; simp at this
        have := hne p hp'; grind
      refine ⟨a.2 + 1, fun p hp' => ?_⟩
      have := below p hp' (hall p hp'); have := hall p hp'
      rw [sgn_sub_pos (show p.2 < a.2 + 1 by grind), sgn_sub_pos (show p.1 < v by grind)]
    · obtain ⟨b, hb, hmin⟩ := exists_min Prod.snd _ hB
      have hb' := List.mem_filter.mp hb
      simp only [decide_eq_true_eq] at hb'
      have above : ∀ p ∈ prs, v < p.1 → b.2 ≤ p.2 := fun p hp' h =>
        hmin p (List.mem_filter.mpr ⟨hp', by simpa using h⟩)
      have hab := ord a ha'.1 b hb'.1 (by grind)
      refine ⟨(a.2 + b.2) / 2, fun p hp' => ?_⟩
      have := hne p hp'
      by_cases h : p.1 < v
      · have := below p hp' h
        rw [sgn_sub_pos (show p.2 < (a.2 + b.2) / 2 by grind), sgn_sub_pos h]
      · have h' : v < p.1 := by grind
        have := above p hp' h'
        rw [sgn_sub_neg (show (a.2 + b.2) / 2 < p.2 by grind), sgn_sub_neg h']

/-! ## Two dimensions: a slab sweep -/

/-- An affine function on the plane, `α + β u + γ v`. -/
structure A2 where
  α : Rat
  β : Rat
  γ : Rat

def A2.ev (h : A2) (u v : Rat) : Rat := h.α + h.β * u + h.γ * v

/-- The height of a non-vertical line at `u`. -/
def ht (h : A2) (u : Rat) : Rat := -(h.α + h.β * u) / h.γ

/-- The critical `u`: where a vertical line stands, and where two non-vertical lines cross. -/
def crit (H : List A2) : List Rat :=
  H.filterMap (fun h => if h.γ = 0 then root1 h.α h.β else none) ++
  H.flatMap fun h => H.filterMap fun k =>
    if h.γ ≠ 0 ∧ k.γ ≠ 0 then root1 (-h.α / h.γ + k.α / k.γ) (-h.β / h.γ + k.β / k.γ) else none

/-- The heights of the non-vertical lines at `u`. -/
def hts (H : List A2) (u : Rat) : List Rat := H.filterMap fun h => if h.γ = 0 then none else some (ht h u)

/-- **The 2D sampler**: candidates in `u` between the critical values, and on each sampled vertical
line candidates between the lines' heights. -/
def S2 (H : List A2) : List (Rat × Rat) :=
  (cand1 (crit H)).flatMap fun u => (cand1 (hts H u)).map fun v => (u, v)

theorem ev_split (h : A2) (hγ : h.γ ≠ 0) (u v : Rat) : h.ev u v = h.γ * (v - ht h u) := by
  have := Rat.mul_inv_cancel h.γ hγ
  simp only [A2.ev, ht, Rat.div_def]; grind

theorem ev_vert (h : A2) (hγ : h.γ = 0) (u v : Rat) : h.ev u v = h.α + h.β * u := by
  simp only [A2.ev, hγ]; grind

theorem ht_diff (h k : A2) (t : Rat) :
    ht h t - ht k t = (-h.α / h.γ + k.α / k.γ) + (-h.β / h.γ + k.β / k.γ) * t := by
  simp only [ht, Rat.div_def]; grind

/-- **Every point of the plane has a sample with the same sign of every line.** -/
theorem cover2 (H : List A2) (u v : Rat) : ∃ p ∈ S2 H, ∀ h ∈ H, sgnQ (h.ev p.1 p.2) = sgnQ (h.ev u v) := by
  obtain ⟨u', hu', hsu⟩ := cand1_ok (crit H) u
  -- the vertical lines
  have hvert : ∀ h ∈ H, h.γ = 0 → sgnQ (h.α + h.β * u') = sgnQ (h.α + h.β * u) := by
    intro h hh hγ
    apply aff_same
    intro c hc
    apply hsu
    exact List.mem_append_left _ (List.mem_filterMap.mpr ⟨h, hh, by simp [hγ, hc]⟩)
  -- the lines' order is kept
  let prs := H.filterMap fun h => if h.γ = 0 then none else some (ht h u, ht h u')
  have mem_prs : ∀ p, p ∈ prs ↔ ∃ h ∈ H, h.γ ≠ 0 ∧ p = (ht h u, ht h u') := by
    intro p
    simp only [prs, List.mem_filterMap]
    constructor
    · rintro ⟨h, hh, he⟩
      split at he
      · cases he
      · cases he; rename_i hγ; exact ⟨h, hh, hγ, rfl⟩
    · rintro ⟨h, hh, hγ, rfl⟩; exact ⟨h, hh, by simp [hγ]⟩
  have hpair : ∀ p ∈ prs, ∀ q ∈ prs, sgnQ (p.2 - q.2) = sgnQ (p.1 - q.1) := by
    intro p hp q hq
    obtain ⟨h, hh, hγ, rfl⟩ := (mem_prs p).mp hp
    obtain ⟨k, hk, kγ, rfl⟩ := (mem_prs q).mp hq
    simp only
    rw [ht_diff, ht_diff]
    apply aff_same
    intro c hc
    apply hsu
    apply List.mem_append_right
    exact List.mem_flatMap.mpr ⟨h, hh, List.mem_filterMap.mpr ⟨k, hk, by simp [hγ, kγ, hc]⟩⟩
  obtain ⟨v', hv'⟩ := emb prs hpair v
  obtain ⟨v'', hv'', hsv⟩ := cand1_ok (hts H u') v'
  refine ⟨(u', v''), List.mem_flatMap.mpr ⟨u', hu', List.mem_map.mpr ⟨v'', hv'', rfl⟩⟩, fun h hh => ?_⟩
  simp only
  by_cases hγ : h.γ = 0
  · rw [ev_vert h hγ, ev_vert h hγ]; exact hvert h hh hγ
  · rw [ev_split h hγ, ev_split h hγ]
    apply sgnQ_mul_eq hγ
    rw [hsv (ht h u') (List.mem_filterMap.mpr ⟨h, hh, by simp [hγ]⟩)]
    exact hv' _ ((mem_prs _).mpr ⟨h, hh, hγ, rfl⟩)

/-! ## Three dimensions: each carrier as a graph -/

/-- The point of carrier `P` over `(u, v)`: `P` as a graph over two coordinates, solved for the
first of z, y, x its normal has. -/
def lift (P : Plane) (u v : Rat) : V3 :=
  if P.n.z ≠ 0 then ⟨u, v, (P.c - P.n.x * u - P.n.y * v) / P.n.z⟩
  else if P.n.y ≠ 0 then ⟨u, (P.c - P.n.x * u - P.n.z * v) / P.n.y, v⟩
  else ⟨(P.c - P.n.y * u - P.n.z * v) / P.n.x, u, v⟩

def proj (P : Plane) (x : V3) : Rat × Rat :=
  if P.n.z ≠ 0 then (x.x, x.y) else if P.n.y ≠ 0 then (x.x, x.z) else (x.y, x.z)

theorem lift_proj {P : Plane} (hn : P.n ≠ ⟨0, 0, 0⟩) {x : V3} (hx : P.eval x = 0) :
    lift P (proj P x).1 (proj P x).2 = x := by
  obtain ⟨⟨nx, ny, nz⟩, c⟩ := P
  obtain ⟨x1, x2, x3⟩ := x
  simp only [Plane.eval, V3.dot] at hx
  unfold lift proj
  simp only
  by_cases hz : nz ≠ 0
  · simp only [hz, ne_eq, not_false_eq_true, ite_true, V3.mk.injEq, true_and]
    have := Rat.mul_inv_cancel nz hz
    rw [Rat.div_def]; grind
  · by_cases hy : ny ≠ 0
    · simp only [hz, hy, ne_eq, not_false_eq_true, ite_true, ite_false, V3.mk.injEq, true_and, and_true]
      have := Rat.mul_inv_cancel ny hy
      rw [Rat.div_def]; grind
    · have hz' : nz = 0 := by grind
      have hy' : ny = 0 := by grind
      have hx' : nx ≠ 0 := by intro h; apply hn; rw [h, hy', hz']
      simp only [hz, hy, ite_false, V3.mk.injEq, and_true]
      have := Rat.mul_inv_cancel nx hx'
      rw [Rat.div_def]; grind

/-- Carrier `q` on carrier `P`, as an affine function of `(u, v)`. -/
def coef (P q : Plane) : A2 :=
  let a := q.eval (lift P 0 0)
  ⟨a, q.eval (lift P 1 0) - a, q.eval (lift P 0 1) - a⟩

theorem coef_ev (P q : Plane) (u v : Rat) : (coef P q).ev u v = q.eval (lift P u v) := by
  obtain ⟨⟨nx, ny, nz⟩, c⟩ := P
  obtain ⟨⟨mx, my, mz⟩, d⟩ := q
  unfold coef A2.ev lift
  simp only [Plane.eval, V3.dot]
  by_cases hz : nz ≠ 0
  · simp only [hz, ne_eq, not_false_eq_true, ite_true, Rat.div_def]; grind
  · by_cases hy : ny ≠ 0
    · simp only [hz, hy, ne_eq, not_false_eq_true, ite_true, ite_false, Rat.div_def]; grind
    · simp only [hz, hy, ite_false, Rat.div_def]; grind

/-- The samples on carrier `P`. -/
def samplesOn (ps : List Plane) (P : Plane) : List V3 :=
  (S2 (ps.map (coef P))).map fun p => lift P p.1 p.2

/-- **The sampler**: the samples on every carrier. Executable, in exact rationals. -/
def sampler (ps : List Plane) : List V3 := ps.flatMap (samplesOn ps)

/-- **The sampler hits every cell lying in a carrier.** -/
theorem sampler_covers {ps : List Plane} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) :
    Sample.Covers ps (sampler ps) := by
  intro x hx
  unfold covOf at hx
  obtain ⟨P, hP, hz⟩ := List.mem_map.mp hx
  have h0 := (sgnQ_zero_iff _).mp hz
  obtain ⟨p, hp, hs⟩ := cover2 (ps.map (coef P)) (proj P x).1 (proj P x).2
  refine ⟨lift P p.1 p.2, List.mem_flatMap.mpr ⟨P, hP, List.mem_map.mpr ⟨p, hp, rfl⟩⟩, ?_⟩
  unfold covOf
  apply List.map_congr_left
  intro q hq
  have := hs (coef P q) (List.mem_map.mpr ⟨q, hq, rfl⟩)
  rw [coef_ev, coef_ev, lift_proj (hnz P hP) h0] at this
  exact this

/-! ## (c) for every flat input, with no hypothesis on the samples -/

open Sample in
/-- **`Exact` for every flat input.** The ledger built from the sampler is exact for the reference's
solids, for any carriers with nonzero normals and any closed flat-faced B-reps on them. -/
theorem flat_exact {ps : List Plane} {ss : List Solid} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩)
    (hi : FlatInput ps ss) : Exact (sampleLedger ps ss (sampler ps)) ps (brepA ss) :=
  sample_exact hnz hi (samples_of_covers (sampler_covers hnz))

open Sample in
/-- **(c) for every flat input.** On the sampler's ledger, each valid answer of the core is the exact
regularised boolean of the reference's solids, its faces exactly the boundary facets, and the
zero-thickness refusal is exactly non-manifoldness. Hypotheses: nonzero carrier normals and
`FlatInput` only. -/
theorem flat_correct {ps : List Plane} {ss : List Solid} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩)
    (hi : FlatInput ps ss) (rule : Rule) :
    (∀ b, core (sampleLedger ps ss (sampler ps)) rule = .ok b →
      DecideOnce.Valid (sampleLedger ps ss (sampler ps)) ((sampleLedger ps ss (sampler ps)).keepTab rule) b ∧
      (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔
        XBoundary (sampleLedger ps ss (sampler ps)) ps (brepA ss) rule φ) ∧
      reg ((decomp (flat_exact hnz hi)).kept fun T =>
          xkeep (sampleLedger ps ss (sampler ps)) ps (brepA ss) rule T = true) =
        reg (combine (ruleOp rule) (opnds (sampleLedger ps ss (sampler ps)) (brepA ss)))) ∧
    ((∃ x, core (sampleLedger ps ss (sampler ps)) rule = .refused (.zeroThickness x)) ↔
      XNonManifold (sampleLedger ps ss (sampler ps)) ps (brepA ss) rule) :=
  ⟨fun _ hb => correct_ok (flat_exact hnz hi) rule hb, zeroThickness_iff (flat_exact hnz hi) rule⟩

/-! ## Checks (compiled evaluation: tests, not theorems)

The sampler as the sample source, on the boxes of `Sample.Check` given as reference B-reps. -/

section Check
open Sample Sample.Check

#guard (core (sampleLedger (grid [0, 1]) [boxS 0 1 0 1 0 1] (sampler (grid [0, 1]))) uni).summary ==
  .ok 6 6 12 8
#guard (core (sampleLedger (grid [0, 1, 2]) [boxS 0 1 0 1 0 1, boxS 1 2 0 1 0 1]
  (sampler (grid [0, 1, 2]))) uni).summary == .ok 6 10 12 8
#guard (match (core (sampleLedger (grid [0, 1, 2]) [boxS 0 1 0 1 0 1, boxS 1 2 1 2 0 1]
  (sampler (grid [0, 1, 2]))) uni).summary with | .zeroThickness _ => true | _ => false)

end Check

end ForgeExact.DecideOnce.Sampler
