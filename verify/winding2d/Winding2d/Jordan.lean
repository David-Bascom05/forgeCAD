import Winding2d.Vertex

/-!
# Theorem 3: a simple polygon's winding number is `0` or `±1`

`wind_simple`: for a simple polygon there is `σ = ±1` with `wind vs q ∈ {0, σ}` at every point
`q` off it. The argument (the complement has at most two values, the polygon's two sides):

* every side has a left and a right value, one apart (`jump`), and they carry round every vertex
  (`vertex`), so all left values are one `l` and all right values one `r = l - 1`;
* every point off the polygon, nudged to a height with no vertex (`generic`), sees along its ray
  `+x` either nothing (winding `0`) or a first side; just before it the winding number is that
  side's left or right value (`reach`);
* a point left of every vertex, at the height of a non-horizontal side, has winding `0` and sees
  a side (`zero_value`), so `0 ∈ {l, r}`.
-/

namespace Winding2d

section simple
variable {vs : List Pt} (hs : Simple vs)
include hs

/-- The vertex heights can be avoided by a small upward nudge. -/
theorem generic {q : Pt} (hq : OffPoly vs q) :
    ∃ q', OffPoly vs q' ∧ (∀ j < vs.length, (vtx vs j).y ≠ q'.y) ∧ wind vs q' = wind vs q := by
  rw [offPoly_iff] at hq
  have h1 := ev_forall_lt vs.length (fun j η => ¬Meets q (addv q η ⟨0, 1⟩) (vtx vs j) (vtx vs (j + 1)))
    (fun j hj => (ev_not_meets_pt (hs.nondeg j) (hq j hj) ⟨0, 0⟩ ⟨0, 1⟩).mono fun _ _ h => by
      rw [addv_zero] at h; exact h)
  have h2 := ev_forall_lt vs.length (fun j η => (vtx vs j).y ≠ (addv q η ⟨0, 1⟩).y) (fun j _ => by
    by_cases hv : q.y < (vtx vs j).y
    · exact (ev_lt (d := (vtx vs j).y - q.y) (by grind)).mono fun η _ h => by simp only [addv]; grind
    · exact Ev.true.mono fun η hη _ => by simp only [addv]; grind)
  obtain ⟨η, hη, hall⟩ := h1.and h2
  obtain ⟨m, g⟩ := hall η hη (by grind)
  refine ⟨addv q η ⟨0, 1⟩, ?_, g, (wind_eq_of_seg vs _ _ ((allMiss_iff vs _ _).mpr m)).symm⟩
  rw [offPoly_iff]
  intro j hj hon
  exact m j hj ⟨_, onSeg_right, hon⟩

end simple

/-! ## Along a ray at a height with no vertex -/

def tau (q a b : Pt) : Rat := (q.y - a.y) / (b.y - a.y)
def cpt (q a b : Pt) : Pt := lerp a b (tau q a b)
def Strad (q a b : Pt) : Prop := (a.y < q.y ∧ q.y < b.y) ∨ (b.y < q.y ∧ q.y < a.y)

instance (q a b : Pt) : Decidable (Strad q a b) := by unfold Strad; infer_instance

theorem strad_facts {q a b : Pt} (h : Strad q a b) :
    0 < tau q a b ∧ tau q a b < 1 ∧ (cpt q a b).y = q.y ∧
      orient a b q = ((cpt q a b).x - q.x) * (b.y - a.y) ∧
      ∀ z, OnSeg z a b → z.y = q.y → z = cpt q a b := by
  have hd : b.y - a.y ≠ 0 := by unfold Strad at h; grind
  have ht := Rat.div_mul_cancel (a := q.y - a.y) hd
  unfold cpt tau
  generalize (q.y - a.y) / (b.y - a.y) = τ at *
  have hu : 0 < τ ∧ τ < 1 := by
    unfold Strad at h
    rcases ne_cases hd with hd' | hd'
    · have k1 := pos_mul_iff_neg hd' (x := τ)
      have k2 := pos_mul_iff_neg hd' (x := 1 - τ)
      have : (1 - τ) * (b.y - a.y) = b.y - q.y := by grind
      grind
    · have k1 := pos_mul_iff hd' (x := τ)
      have k2 := pos_mul_iff hd' (x := 1 - τ)
      have : (1 - τ) * (b.y - a.y) = b.y - q.y := by grind
      grind
  refine ⟨hu.1, hu.2, by simp only [lerp]; grind, ?_, ?_⟩
  · simp only [orient, lerp]
    have := congrArg (· * (b.x - a.x)) ht
    grind
  · rintro z ⟨t, -, -, rfl⟩ hz
    have : (t - τ) * (b.y - a.y) = 0 := by simp only [lerp] at hz; grind
    have := mul_eq_zero_left hd this
    ext <;> simp only [lerp] <;> grind

/-- At a height with no vertex, a side counts `±1` iff it straddles the height right of `q`. -/
theorem cross_generic {q a b : Pt} (ha : a.y ≠ q.y) (hb : b.y ≠ q.y) (hq : ¬OnSeg q a b) :
    cross q a b = if Strad q a b ∧ q.x < (cpt q a b).x then (if a.y < b.y then 1 else -1) else 0 := by
  by_cases hs : Strad q a b
  · obtain ⟨t0, t1, hy, ho, -⟩ := strad_facts hs
    have hne : (cpt q a b).x ≠ q.x := by
      intro h; apply hq; have : q = cpt q a b := by ext <;> grind
      rw [this]; exact ⟨_, Rat.le_of_lt t0, Rat.le_of_lt t1, rfl⟩
    unfold Strad at hs
    rcases hs with hs | hs
    · have k := pos_mul_iff (k := b.y - a.y) (x := (cpt q a b).x - q.x) (by grind)
      have k' := neg_mul_iff (k := b.y - a.y) (x := (cpt q a b).x - q.x) (by grind)
      unfold cross Strad; grind
    · have k := neg_mul_iff (k := a.y - b.y) (x := (cpt q a b).x - q.x) (by grind)
      have k' := pos_mul_iff (k := a.y - b.y) (x := (cpt q a b).x - q.x) (by grind)
      have e : ((cpt q a b).x - q.x) * (b.y - a.y) = -(((cpt q a b).x - q.x) * (a.y - b.y)) := by grind
      unfold cross Strad; grind
  · unfold Strad at hs; unfold cross Strad; grind

theorem exists_min {α : Type} (f : α → Rat) : ∀ l : List α, l ≠ [] → ∃ j ∈ l, ∀ k ∈ l, f j ≤ f k
  | [], h => absurd rfl h
  | [x], _ => ⟨x, by simp, by simp⟩
  | x :: y :: t, _ => by
    obtain ⟨j, hj, hm⟩ := exists_min f (y :: t) (by simp)
    by_cases h : f x ≤ f j
    · exact ⟨x, by simp, fun k hk => by
        rcases List.mem_cons.mp hk with rfl | hk
        · grind
        · have := hm k hk; grind⟩
    · exact ⟨j, List.mem_cons_of_mem _ hj, fun k hk => by
        rcases List.mem_cons.mp hk with rfl | hk
        · grind
        · exact hm k hk⟩

theorem exists_lower (f : Nat → Rat) : ∀ n : Nat, ∃ m, ∀ j < n, m ≤ f j
  | 0 => ⟨0, fun _ h => absurd h (by omega)⟩
  | n + 1 => by
    obtain ⟨m, hm⟩ := exists_lower f n
    by_cases h : m ≤ f n
    · refine ⟨m, fun j hj => ?_⟩
      by_cases e : j = n
      · rw [e]; exact h
      · exact hm j (by omega)
    · refine ⟨f n, fun j hj => ?_⟩
      by_cases e : j = n
      · rw [e]; grind
      · have := hm j (by omega); grind

section simple
variable {vs : List Pt} (hs : Simple vs)
include hs

/-- No side straddles the height right of `q`: the winding number is `0`. -/
theorem reach_none {q : Pt} (hq : OffPoly vs q) (hg : ∀ j < vs.length, (vtx vs j).y ≠ q.y)
    (hn : ∀ j < vs.length, ¬(Strad q (vtx vs j) (vtx vs (j + 1)) ∧ q.x < (cpt q (vtx vs j) (vtx vs (j + 1))).x)) :
    wind vs q = 0 := by
  rw [offPoly_iff] at hq
  rw [wind_eq_sum]
  have : ((List.range vs.length).map fun i => cross q (vtx vs i) (vtx vs (i + 1))) =
      (List.range vs.length).map fun _ => (0 : Int) := by
    apply List.map_congr_left; intro j hj
    simp only [List.mem_range] at hj
    rw [cross_generic (hg j hj) (by rw [← vtx_nxt vs j hj]; exact hg _ (nxt_lt hj)) (hq j hj)]
    simp [hn j hj]
  rw [this]; clear this; induction vs.length with
  | zero => rfl
  | succ k ih => rw [List.range_succ, List.map_append, List.sum_append, ih]; rfl

/-- **Reach.** If some side straddles the height right of `q`, the first one along the ray
gives `q` its value. -/
theorem reach_some {q : Pt} (hq : OffPoly vs q) (hg : ∀ j < vs.length, (vtx vs j).y ≠ q.y)
    (hh : ∃ j < vs.length, Strad q (vtx vs j) (vtx vs (j + 1)) ∧ q.x < (cpt q (vtx vs j) (vtx vs (j + 1))).x) :
    ∃ j < vs.length, ∃ ς, Sgn ς ∧ wind vs q = val vs j ς := by
  rw [offPoly_iff] at hq
  let H := (List.range vs.length).filter fun j =>
    decide (Strad q (vtx vs j) (vtx vs (j + 1)) ∧ q.x < (cpt q (vtx vs j) (vtx vs (j + 1))).x)
  have hH : H ≠ [] := by
    obtain ⟨j, hj, h⟩ := hh
    intro he
    have : j ∈ H := by simp [H, hj, h]
    rw [he] at this; simp at this
  obtain ⟨js, hjs, hmin⟩ := exists_min (fun j => (cpt q (vtx vs j) (vtx vs (j + 1))).x) H hH
  simp only [H, List.mem_filter, List.mem_range, decide_eq_true_eq] at hjs hmin
  obtain ⟨hjn, hst, hright⟩ := hjs
  obtain ⟨t0, t1, hcy, -, -⟩ := strad_facts hst
  have hab := hs.nondeg js
  generalize ha : vtx vs js = a at *
  generalize hb : vtx vs (js + 1) = b at *
  -- the side to the left of `-x`: `ς` with `ς·hdir = (-1, 0)`
  have hab' : a.y ≠ b.y := by unfold Strad at hst; grind
  let ς : Rat := if a.y < b.y then 1 else -1
  have hς : Sgn ς := by unfold Sgn; by_cases h : a.y < b.y <;> simp [ς, h]
  have hdirx : smul ς (hdir a b) = ⟨-1, 0⟩ := by
    unfold hdir smul; by_cases h : a.y < b.y
    · simp [ς, h]
    · have : b.y < a.y := by grind
      simp [ς, h, this]
  have hv := val_spec hs hjn hς t0 t1
  rw [ha, hb] at hv
  obtain ⟨κ, hκ, hall⟩ := hv.and (ev_lt (d := (cpt q a b).x - q.x) (by grind))
  obtain ⟨hw, hκ1⟩ := hall κ hκ (by grind)
  refine ⟨js, hjn, ς, hς, ?_⟩
  rw [← hw]
  apply wind_eq_of_seg
  rw [allMiss_iff]
  intro j hj
  have hr : probe a b (tau q a b) ς κ = ⟨(cpt q a b).x - κ, q.y⟩ := by
    rw [probe_eq, hdirx]; ext <;> simp only [addv, cpt] <;> simp only [cpt] at hcy <;> grind
  rw [hr]
  have hj1 : (vtx vs j).y ≠ q.y := hg j hj
  have hj2 : (vtx vs (j + 1)).y ≠ q.y := by rw [← vtx_nxt vs j hj]; exact hg _ (nxt_lt hj)
  by_cases hsj : Strad q (vtx vs j) (vtx vs (j + 1))
  · obtain ⟨-, -, -, -, honly⟩ := strad_facts hsj
    rintro ⟨z, ⟨t, tt0, tt1, rfl⟩, hz⟩
    have hzy : (lerp q ⟨(cpt q a b).x - κ, q.y⟩ t).y = q.y := by simp only [lerp]; grind
    have e := honly _ hz hzy
    have hzx : (lerp q ⟨(cpt q a b).x - κ, q.y⟩ t).x = q.x + t * ((cpt q a b).x - κ - q.x) := by
      simp only [lerp]
    have := Rat.mul_nonneg tt0 (by grind : (0:Rat) ≤ (cpt q a b).x - κ - q.x)
    have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (by grind : (0:Rat) ≤ (cpt q a b).x - κ - q.x)
    rw [e] at hzx
    by_cases hin : q.x < (cpt q (vtx vs j) (vtx vs (j + 1))).x
    · have := hmin j ⟨hj, hsj, hin⟩
      grind
    · have hxe : (cpt q (vtx vs j) (vtx vs (j + 1))).x = q.x := by grind
      apply hq j hj
      have : q = cpt q (vtx vs j) (vtx vs (j + 1)) := by
        ext
        · exact hxe.symm
        · obtain ⟨-, -, hy', -, -⟩ := strad_facts hsj; exact hy'.symm
      rw [this, ← e]; exact hz
  · -- the side is wholly above or wholly below the height
    have hside : (q.y < (vtx vs j).y ∧ q.y < (vtx vs (j + 1)).y) ∨
        ((vtx vs j).y < q.y ∧ (vtx vs (j + 1)).y < q.y) := by unfold Strad at hsj; grind
    have oz : ∀ z : Pt, orient ⟨0, q.y⟩ ⟨1, q.y⟩ z = z.y - q.y := by intro z; simp only [orient]; grind
    intro hm
    rcases hside with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · exact sep (P := ⟨0, q.y⟩) (Q := ⟨1, q.y⟩) (ς := 1) (by rw [oz]; grind) (by rw [oz]; grind)
        (by rw [oz]; grind) (by rw [oz]; grind) (meets_symm hm)
    · exact sep (P := ⟨0, q.y⟩) (Q := ⟨1, q.y⟩) (ς := -1) (by rw [oz]; grind) (by rw [oz]; grind)
        (by rw [oz]; grind) (by rw [oz]; grind) (meets_symm hm)

omit hs in
theorem sum_index_diff (vs : List Pt) (g : Pt → Int) :
    ((List.range vs.length).map fun j => g (vtx vs (j + 1)) - g (vtx vs j)).sum = 0 := by
  have := sum_sides_diff g vs
  rw [sides_eq, List.map_map] at this
  exact this

/-- **A side of value `0`**: left of every vertex, at the height of a side that is not
horizontal, the winding number is `0` and the ray sees a side. -/
theorem zero_value (hh : ∃ j0 < vs.length, (vtx vs j0).y ≠ (vtx vs (j0 + 1)).y) :
    ∃ j < vs.length, ∃ ς, Sgn ς ∧ val vs j ς = 0 := by
  obtain ⟨j0, hj0, hne⟩ := hh
  obtain ⟨mx, hmx⟩ := exists_lower (fun j => (vtx vs j).x) vs.length
  have hmx' : ∀ j < vs.length, mx ≤ (vtx vs (j + 1)).x := fun j hj => by
    rw [← vtx_nxt vs j hj]; exact hmx _ (nxt_lt hj)
  have hpt : ∀ j < vs.length, ∀ z, OnSeg z (vtx vs j) (vtx vs (j + 1)) → mx ≤ z.x := by
    rintro j hj z ⟨t, t0, t1, rfl⟩
    exact convex_ge (hmx j hj) (hmx' j hj) t0 t1
  generalize ha : vtx vs j0 = a at hne
  generalize hb : vtx vs (j0 + 1) = b at hne
  obtain ⟨my, hmy⟩ : ∃ my : Rat, my = a.y + (1 / 2) * (b.y - a.y) := ⟨_, rfl⟩
  obtain ⟨δ, hδ1, hδ2⟩ : ∃ δ : Rat, (a.y < b.y → δ = b.y - a.y) ∧ (¬a.y < b.y → δ = a.y - b.y) := by
    by_cases h : a.y < b.y
    · exact ⟨b.y - a.y, fun _ => rfl, fun h' => absurd h h'⟩
    · exact ⟨a.y - b.y, fun h' => absurd h' h, fun _ => rfl⟩
  have hδ : 0 < δ := by by_cases h : a.y < b.y <;> grind
  have hd2 := Rat.div_mul_cancel (a := δ) (b := 2) (by decide)
  have hE := (ev_lt (d := δ / 2) (by grind)).and
    (ev_forall_lt vs.length (fun j η => (vtx vs j).y ≠ my + η) (fun j _ => by
      by_cases hv : my < (vtx vs j).y
      · exact (ev_lt (d := (vtx vs j).y - my) (by grind)).mono fun η _ h => by grind
      · exact Ev.true.mono fun η hη _ => by grind))
  obtain ⟨η, hη, hall⟩ := hE
  obtain ⟨hη1, hgen⟩ := hall η hη (by grind)
  clear hall
  obtain ⟨q0, hq0x, hq0y⟩ : ∃ q0 : Pt, q0.x = mx - 1 ∧ q0.y = my + η := ⟨⟨mx - 1, my + η⟩, rfl, rfl⟩
  have hoff : OffPoly vs q0 := by
    rw [offPoly_iff]; intro j hj hon
    have := hpt j hj q0 hon; grind
  have hg : ∀ j < vs.length, (vtx vs j).y ≠ q0.y := by rw [hq0y]; exact hgen
  have hst : Strad q0 a b := by
    unfold Strad; by_cases h : a.y < b.y <;> grind
  have hcx : q0.x < (cpt q0 a b).x := by
    have := hpt j0 hj0 (cpt q0 a b) (by
      obtain ⟨t0, t1, -, -, -⟩ := strad_facts hst
      rw [ha, hb]; exact ⟨_, Rat.le_of_lt t0, Rat.le_of_lt t1, rfl⟩)
    grind
  obtain ⟨j, hj, ς, hς, hw⟩ := reach_some hs hoff hg ⟨j0, hj0, by rw [ha, hb]; exact ⟨hst, hcx⟩⟩
  refine ⟨j, hj, ς, hς, ?_⟩
  rw [← hw, wind_eq_sum]
  rw [offPoly_iff] at hoff
  have : ((List.range vs.length).map fun i => cross q0 (vtx vs i) (vtx vs (i + 1))) =
      (List.range vs.length).map fun i =>
        (fun v => if q0.y < v.y then (1 : Int) else 0) (vtx vs (i + 1)) -
        (fun v => if q0.y < v.y then (1 : Int) else 0) (vtx vs i) := by
    apply List.map_congr_left; intro i hi
    simp only [List.mem_range] at hi
    have h1 := hg i hi
    have h2 : (vtx vs (i + 1)).y ≠ q0.y := by rw [← vtx_nxt vs i hi]; exact hg _ (nxt_lt hi)
    rw [cross_generic h1 h2 (hoff i hi)]
    clear hw hcx hst hgen hη1 hd2 hδ hδ1 hδ2 hmy hq0y
    by_cases hs' : Strad q0 (vtx vs i) (vtx vs (i + 1))
    · have hx : q0.x < (cpt q0 (vtx vs i) (vtx vs (i + 1))).x := by
        have := hpt i hi (cpt q0 (vtx vs i) (vtx vs (i + 1))) (by
          obtain ⟨t0, t1, -, -, -⟩ := strad_facts hs'
          exact ⟨_, Rat.le_of_lt t0, Rat.le_of_lt t1, rfl⟩)
        grind
      rw [if_pos ⟨hs', hx⟩]
      unfold Strad at hs'
      generalize vtx vs i = c at *
      generalize vtx vs (i + 1) = d at *
      rcases hs' with ⟨l1, l2⟩ | ⟨l1, l2⟩
      · have e1 : c.y < d.y := by grind
        have e2 : ¬q0.y < c.y := by grind
        simp only [e1, l2, e2, ite_true, ite_false]; rfl
      · have e1 : ¬c.y < d.y := by grind
        have e2 : ¬q0.y < d.y := by grind
        simp only [e1, l2, e2, ite_true, ite_false]; rfl
    · rw [if_neg (fun h => hs' h.1)]
      unfold Strad at hs'
      generalize vtx vs i = c at *
      generalize vtx vs (i + 1) = d at *
      by_cases k1 : q0.y < c.y <;> by_cases k2 : q0.y < d.y <;> simp only [k1, k2, ite_true, ite_false] <;>
        first | rfl | grind
  rw [this]; exact sum_index_diff vs (fun v => if q0.y < v.y then (1 : Int) else 0)

theorem val_all {ς : Rat} (hς : Sgn ς) : ∀ j < vs.length, val vs j ς = val vs 0 ς := by
  intro j
  induction j with
  | zero => intro _; rfl
  | succ k ih =>
    intro hk
    have hv := vertex hs (i := k) (by omega) hς
    have : nxt vs.length k = k + 1 := by unfold nxt; split <;> omega
    rw [this] at hv
    rw [← hv]; exact ih (by omega)

/-- **Theorem 3.** A simple polygon's winding number is `0` or `σ` at every point off it, for one
`σ = ±1`. -/
theorem wind_simple : ∃ σ : Int, (σ = 1 ∨ σ = -1) ∧
    ∀ q, OffPoly vs q → wind vs q = 0 ∨ wind vs q = σ := by
  have hn := len_pos' hs
  have hj := jump hs hn
  by_cases hh : ∃ j0 < vs.length, (vtx vs j0).y ≠ (vtx vs (j0 + 1)).y
  · obtain ⟨j, hjn, ς, hς, h0⟩ := zero_value hs hh
    rw [val_all hs hς j hjn] at h0
    have hz : val vs 0 1 = 0 ∨ val vs 0 (-1) = 0 := by
      rcases hς with rfl | rfl
      · exact Or.inl h0
      · exact Or.inr h0
    refine ⟨if val vs 0 (-1) = 0 then 1 else -1, by split <;> simp, fun q hq => ?_⟩
    obtain ⟨q', hq', hg, hw⟩ := generic hs hq
    rw [← hw]
    by_cases hit : ∃ j < vs.length, Strad q' (vtx vs j) (vtx vs (j + 1)) ∧
        q'.x < (cpt q' (vtx vs j) (vtx vs (j + 1))).x
    · obtain ⟨j, hjn, ς, hς, hv⟩ := reach_some hs hq' hg hit
      rw [hv, val_all hs hς j hjn]
      rcases hς with rfl | rfl <;> split <;> grind
    · exact Or.inl (reach_none hs hq' hg (fun j hj h => hit ⟨j, hj, h⟩))
  · refine ⟨1, Or.inl rfl, fun q _ => Or.inl ?_⟩
    rw [wind_eq_sum]
    have : ((List.range vs.length).map fun i => cross q (vtx vs i) (vtx vs (i + 1))) =
        (List.range vs.length).map fun _ => (0 : Int) := by
      apply List.map_congr_left; intro i hi
      simp only [List.mem_range] at hi
      have : (vtx vs i).y = (vtx vs (i + 1)).y := by
        apply Classical.byContradiction; intro h; exact hh ⟨i, hi, h⟩
      unfold cross; grind
    rw [this]; clear this; induction vs.length with
    | zero => rfl
    | succ k ih => rw [List.range_succ, List.map_append, List.sum_append, ih]; rfl

/-- The same, as sketch text states it: the count is `1`, `-1` or `0`. -/
theorem wind_simple_mem {q : Pt} (hq : OffPoly vs q) : wind vs q = 0 ∨ wind vs q = 1 ∨ wind vs q = -1 := by
  obtain ⟨σ, hσ, h⟩ := wind_simple hs
  rcases h q hq with h | h
  · exact Or.inl h
  · rcases hσ with rfl | rfl
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr h)

/-- Every value off the polygon is `0` or a value of side `0`. -/
theorem wind_vals {q : Pt} (hq : OffPoly vs q) : wind vs q = 0 ∨ ∃ ς, Sgn ς ∧ wind vs q = val vs 0 ς := by
  obtain ⟨q', hq', hg, hw⟩ := generic hs hq
  rw [← hw]
  by_cases hit : ∃ j < vs.length, Strad q' (vtx vs j) (vtx vs (j + 1)) ∧
      q'.x < (cpt q' (vtx vs j) (vtx vs (j + 1))).x
  · obtain ⟨j, hjn, ς, hς, hv⟩ := reach_some hs hq' hg hit
    exact Or.inr ⟨ς, hς, by rw [hv, val_all hs hς j hjn]⟩
  · exact Or.inl (reach_none hs hq' hg (fun j hj h => hit ⟨j, hj, h⟩))

omit hs in
theorem wind_below {q : Pt} (h : ∀ j < vs.length, q.y < (vtx vs j).y) : wind vs q = 0 := by
  rw [wind_eq_sum]
  have : ((List.range vs.length).map fun i => cross q (vtx vs i) (vtx vs (i + 1))) =
      (List.range vs.length).map fun _ => (0 : Int) := by
    apply List.map_congr_left; intro i hi
    simp only [List.mem_range] at hi
    have h1 := h i hi
    have h2 : q.y < (vtx vs (i + 1)).y := by rw [← vtx_nxt vs i hi]; exact h _ (nxt_lt hi)
    unfold cross; grind
  rw [this]; clear this; induction vs.length with
  | zero => rfl
  | succ k ih => rw [List.range_succ, List.map_append, List.sum_append, ih]; rfl

/-- Vertex `i+1` is the lowest, and the leftmost of the lowest. -/
def Lowest (vs : List Pt) (i : Nat) : Prop :=
  ∀ j < vs.length, (vtx vs (i + 1)).y < (vtx vs j).y ∨
    ((vtx vs (i + 1)).y = (vtx vs j).y ∧ (vtx vs (i + 1)).x ≤ (vtx vs j).x)

omit hs in
theorem exists_lowest (h : 0 < vs.length) : ∃ i < vs.length, Lowest vs i := by
  have key : ∀ m, m ≤ vs.length → 0 < m → ∃ k < m, ∀ j < m, (vtx vs k).y < (vtx vs j).y ∨
      ((vtx vs k).y = (vtx vs j).y ∧ (vtx vs k).x ≤ (vtx vs j).x) := by
    intro m
    induction m with
    | zero => intro _ h; omega
    | succ m ih =>
      intro hm _
      by_cases h0 : m = 0
      · subst h0; exact ⟨0, by omega, fun j hj => by
          have : j = 0 := by omega
          subst this; right; constructor <;> grind⟩
      · obtain ⟨k, hk, hkm⟩ := ih (by omega) (by omega)
        by_cases hb : (vtx vs k).y < (vtx vs m).y ∨
            ((vtx vs k).y = (vtx vs m).y ∧ (vtx vs k).x ≤ (vtx vs m).x)
        · exact ⟨k, by omega, fun j hj => by
            by_cases e : j = m
            · rw [e]; exact hb
            · exact hkm j (by omega)⟩
        · exact ⟨m, by omega, fun j hj => by
            by_cases e : j = m
            · rw [e]; right; constructor <;> grind
            · have := hkm j (by omega); grind⟩
  obtain ⟨k, hk, hkm⟩ := key vs.length (by omega) h
  -- vertex `k` is vertex `i + 1` for `i = k - 1` (mod n)
  refine ⟨if k = 0 then vs.length - 1 else k - 1, by split <;> omega, fun j hj => ?_⟩
  have e : vtx vs ((if k = 0 then vs.length - 1 else k - 1) + 1) = vtx vs k := by
    split
    · rename_i h0; rw [h0, show vs.length - 1 + 1 = vs.length by omega]
      simp [vtx]
    · rw [show k - 1 + 1 = k by omega]
  rw [e]; exact hkm j hj

/-- **By orientation.** At the lowest (then leftmost) vertex the turn `orient u v w` is not zero,
and its sign is the polygon's: every point off it winds `0` or `sign (orient u v w)`. So `σ = 1`
for a counter-clockwise polygon, `-1` for a clockwise one. -/
theorem wind_simple_orient {i : Nat} (hi : i < vs.length) (hl : Lowest vs i) :
    orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) ≠ 0 ∧
    ∀ q, OffPoly vs q → wind vs q = 0 ∨
      wind vs q = if 0 < orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) then 1 else -1 := by
  have hn := len_pos' hs
  have hj := jump hs hn
  have hu := hs.nondeg i
  have hv : vtx vs (i + 1) ≠ vtx vs (i + 2) := hs.nondeg (i + 1)
  have ns := no_spike hs i
  have l1 := hl i hi
  have l2 : (vtx vs (i + 1)).y < (vtx vs (i + 2)).y ∨
      ((vtx vs (i + 1)).y = (vtx vs (i + 2)).y ∧ (vtx vs (i + 1)).x ≤ (vtx vs (i + 2)).x) := by
    have := hl _ (nxt_lt (nxt_lt (n := vs.length) hi))
    rwa [vtx_nxt vs _ (nxt_lt hi), vtx_nxt_succ vs i hi] at this
  -- the turn is not zero
  have hturn : orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) ≠ 0 := by
    intro h0
    have hd := ns h0
    generalize vtx vs i = u at *
    generalize vtx vs (i + 1) = v at *
    generalize vtx vs (i + 2) = w at *
    simp only [orient, dot, sub] at h0 hd
    have nu : ¬(v.x - u.x = 0 ∧ v.y - u.y = 0) := fun ⟨a, b⟩ => hu (by ext <;> grind)
    have nw : ¬(w.x - v.x = 0 ∧ w.y - v.y = 0) := fun ⟨a, b⟩ => hv (by ext <;> grind)
    by_cases hy1 : v.y - u.y = 0
    · have : (v.x - u.x) * (w.y - v.y) = 0 := by grind
      rcases Rat.mul_eq_zero.mp this with h | h
      · exact nu ⟨h, hy1⟩
      · have e1 : v.y = u.y := by grind
        have e2 : w.y = v.y := by grind
        have a1 : v.x ≤ u.x := by grind
        have a2 : v.x ≤ w.x := by grind
        have := Rat.mul_nonneg (by grind : (0:Rat) ≤ u.x - v.x) (by grind : (0:Rat) ≤ w.x - v.x)
        rw [hy1, h] at hd
        grind
    · by_cases hy2 : w.y - v.y = 0
      · have : (v.y - u.y) * (w.x - v.x) = 0 := by grind
        rcases Rat.mul_eq_zero.mp this with h | h
        · exact hy1 h
        · exact nw ⟨h, hy2⟩
      · have hp : 0 < w.y - v.y := by grind
        have hn' : v.y - u.y < 0 := by grind
        have e : ((v.x - u.x) * (w.x - v.x) + (v.y - u.y) * (w.y - v.y)) * (w.y - v.y) =
            (v.y - u.y) * ((w.x - v.x) * (w.x - v.x) + (w.y - v.y) * (w.y - v.y)) := by grind
        have k1 := Rat.mul_pos hd hp
        have k2 := mul_self_nonneg' (w.x - v.x)
        have k3 := Rat.mul_pos hp hp
        have k4 := (neg_mul_iff (k := (w.x - v.x) * (w.x - v.x) + (w.y - v.y) * (w.y - v.y))
          (x := v.y - u.y) (by grind)).mpr hn'
        grind
  refine ⟨hturn, fun q hq => ?_⟩
  have hc := corner hs hi (ς := if 0 < orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) then -1 else 1)
    (by split <;> simp [Sgn])
  -- the corner point on the outer side is below every vertex
  obtain ⟨s, hs0, hcs⟩ := hc
  obtain ⟨hc1, -⟩ := hcs s hs0 (by grind)
  have hbelow : wind vs (addv (vtx vs (i + 1)) s
      (smul (if 0 < orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) then -1 else 1)
        (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2))))) = 0 := by
    apply wind_below
    intro j hj
    have hlj := hl j hj
    have hy : (smul (if 0 < orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) then -1 else 1)
        (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)))).y < 0 := by
      unfold cdir smul
      generalize vtx vs i = u at *
      generalize vtx vs (i + 1) = v at *
      generalize vtx vs (i + 2) = w at *
      have hyy : ¬(v.y - u.y = 0 ∧ w.y - v.y = 0) := by
        rintro ⟨a, b⟩; apply hturn; simp only [orient]; grind
      by_cases h1 : 0 < orient u v w
      · simp only [h1, ite_true]; grind
      · have h2 : orient u v w < 0 := by grind
        simp only [h1, h2, ite_true, ite_false]; grind
    simp only [addv]
    have := Rat.mul_pos hs0 (by grind : 0 < -(smul (if 0 < orient (vtx vs i) (vtx vs (i + 1))
      (vtx vs (i + 2)) then -1 else 1) (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)))).y)
    grind
  rw [hbelow, val_all hs (by split <;> simp [Sgn]) i hi] at hc1
  rcases wind_vals hs hq with h | ⟨ς, hς, h⟩
  · exact Or.inl h
  · rw [h]
    by_cases ht : 0 < orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2))
    · simp only [ht, ite_true] at hc1 ⊢
      rcases hς with rfl | rfl <;> grind
    · simp only [ht, ite_false] at hc1 ⊢
      rcases hς with rfl | rfl <;> grind

end simple

end Winding2d
