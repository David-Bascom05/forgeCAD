import Winding2d.Witness

/-!
# Polygons by index, and simple polygons

`vtx vs i` is vertex `i mod n`; side `i` is `(vtx vs i, vtx vs (i+1))`, and `sides vs` lists the
sides `0 … n-1` (`sides_eq`). A polygon is `Simple` when it has at least three vertices, no side
is a point, two consecutive sides meet only at their common vertex, and two sides that are not
consecutive do not meet.

Consequences used by the Jordan argument: a point inside a side lies on no other side
(`interior_off`, `sub_off`), and a vertex lies on no side but its two (`vertex_off`).
-/

namespace Winding2d

def vtx (vs : List Pt) (i : Nat) : Pt := vs.getD (i % vs.length) default

/-- The next index round the polygon. -/
def nxt (n i : Nat) : Nat := if i + 1 = n then 0 else i + 1

theorem vtx_mod (vs : List Pt) (i : Nat) : vtx vs (i % vs.length) = vtx vs i := by
  simp [vtx, Nat.mod_mod]

theorem vtx_nxt (vs : List Pt) (i : Nat) (hi : i < vs.length) : vtx vs (nxt vs.length i) = vtx vs (i + 1) := by
  unfold nxt; split
  · rename_i h; rw [h]; simp [vtx, Nat.mod_self, Nat.zero_mod]
  · rfl

theorem nxt_lt {n i : Nat} (hi : i < n) : nxt n i < n := by unfold nxt; split <;> omega

theorem getD_append_lt : ∀ (l l' : List Pt) (i : Nat), i < l.length → (l ++ l').getD i default = l.getD i default
  | [], _, _, h => by simp at h
  | _ :: _, _, 0, _ => rfl
  | _ :: l, l', i + 1, h => by
    simp only [List.cons_append, List.getD_cons_succ]
    exact getD_append_lt l l' i (by simp at h; omega)

theorem getD_append_len : ∀ (l : List Pt) (x : Pt), (l ++ [x]).getD l.length default = x
  | [], _ => rfl
  | _ :: l, x => by simp only [List.cons_append, List.length_cons, List.getD_cons_succ]; exact getD_append_len l x

theorem chain_getD : ∀ l : List Pt,
    chain l = (List.range (l.length - 1)).map fun i => (l.getD i default, l.getD (i + 1) default)
  | [] => rfl
  | [_] => rfl
  | a :: b :: t => by
    rw [chain, chain_getD (b :: t)]
    simp only [List.length_cons, Nat.add_sub_cancel]
    rw [List.range_succ_eq_map]
    simp [List.getD_cons_succ, List.getD_cons_zero]

/-- **The sides by index.** -/
theorem sides_eq (vs : List Pt) :
    sides vs = (List.range vs.length).map fun i => (vtx vs i, vtx vs (i + 1)) := by
  cases vs with
  | nil => rfl
  | cons v rest =>
    simp only [sides]
    rw [chain_getD]
    simp only [List.length_append, List.length_cons, List.length_nil, Nat.add_sub_cancel]
    apply List.map_congr_left
    intro i hi
    simp only [List.mem_range] at hi
    have h1 : (v :: rest ++ [v]).getD i default = vtx (v :: rest) i := by
      rw [getD_append_lt _ _ _ (by simp; omega)]
      simp only [vtx, List.length_cons]; rw [Nat.mod_eq_of_lt (by omega)]
    have h2 : (v :: rest ++ [v]).getD (i + 1) default = vtx (v :: rest) (i + 1) := by
      by_cases hl : i + 1 < rest.length + 1
      · rw [getD_append_lt _ _ _ (by simp; omega)]
        simp only [vtx, List.length_cons]; rw [Nat.mod_eq_of_lt (by omega)]
      · have e : i + 1 = (v :: rest).length := by simp; omega
        rw [e, getD_append_len]
        simp [vtx]
    rw [h1, h2]

theorem sum_range_single (g : Nat → Int) : ∀ (n i0 : Nat), i0 < n → (∀ i < n, i ≠ i0 → g i = 0) →
    ((List.range n).map g).sum = g i0
  | 0, _, h, _ => by omega
  | n + 1, i0, h, hz => by
    rw [List.range_succ, List.map_append, List.sum_append]
    by_cases e : i0 = n
    · subst e
      have : ((List.range i0).map g).sum = 0 := by
        rw [List.map_congr_left (g := fun _ => (0 : Int)) (fun i hi => hz i (by simp at hi; omega)
          (by simp at hi; omega))]
        clear hz h; induction i0 with
        | zero => rfl
        | succ k ih => rw [List.range_succ, List.map_append, List.sum_append, ih]; rfl
      rw [this]; simp
    · rw [sum_range_single g n i0 (by omega) (fun i hi hne => hz i (by omega) hne)]
      simp [hz n (by omega) (Ne.symm e)]

theorem wind_eq_sum (vs : List Pt) (q : Pt) :
    wind vs q = ((List.range vs.length).map fun i => cross q (vtx vs i) (vtx vs (i + 1))).sum := by
  unfold wind; rw [sides_eq, List.map_map]; rfl

theorem offPoly_iff (vs : List Pt) (q : Pt) :
    OffPoly vs q ↔ ∀ i < vs.length, ¬OnSeg q (vtx vs i) (vtx vs (i + 1)) := by
  unfold OffPoly; rw [sides_eq]; simp

theorem allMiss_iff (vs : List Pt) (p p' : Pt) :
    (∀ e ∈ sides vs, ¬Meets p p' e.1 e.2) ↔ ∀ i < vs.length, ¬Meets p p' (vtx vs i) (vtx vs (i + 1)) := by
  rw [sides_eq]; simp

/-- **A simple polygon.** -/
structure Simple (vs : List Pt) : Prop where
  three : 3 ≤ vs.length
  nondeg : ∀ i, vtx vs i ≠ vtx vs (i + 1)
  adj : ∀ i z, OnSeg z (vtx vs i) (vtx vs (i + 1)) → OnSeg z (vtx vs (i + 1)) (vtx vs (i + 2)) →
    z = vtx vs (i + 1)
  apart : ∀ i j, i < vs.length → j < vs.length → i ≠ j → nxt vs.length i ≠ j → nxt vs.length j ≠ i →
    ¬Meets (vtx vs i) (vtx vs (i + 1)) (vtx vs j) (vtx vs (j + 1))

theorem lerp_eq_right {a b : Pt} {t : Rat} (hab : a ≠ b) (h : lerp a b t = b) : t = 1 := by
  have hx : (t - 1) * (b.x - a.x) = 0 := by have := congrArg Pt.x h; simp only [lerp] at this; grind
  have hy : (t - 1) * (b.y - a.y) = 0 := by have := congrArg Pt.y h; simp only [lerp] at this; grind
  apply Classical.byContradiction; intro ht
  have h1 : t - 1 ≠ 0 := by grind
  apply hab
  have := mul_eq_zero_left (k := t - 1) h1 (by grind : (b.x - a.x) * (t - 1) = 0)
  have := mul_eq_zero_left (k := t - 1) h1 (by grind : (b.y - a.y) * (t - 1) = 0)
  ext <;> grind

theorem lerp_eq_left {a b : Pt} {t : Rat} (hab : a ≠ b) (h : lerp a b t = a) : t = 0 := by
  have e : lerp b a (1 - t) = lerp a b t := by ext <;> simp only [lerp] <;> grind
  have := lerp_eq_right (Ne.symm hab) (by rw [e, h])
  grind

theorem nxt_cases {n i : Nat} (hi : i < n) : (i + 1 = n ∧ nxt n i = 0) ∨ (i + 1 < n ∧ nxt n i = i + 1) := by
  unfold nxt; split <;> omega

section simple
variable {vs : List Pt} (hs : Simple vs)
include hs

theorem len_pos' : 0 < vs.length := by have := hs.three; omega

/-- A point strictly inside side `i` lies on no other side. -/
theorem interior_off {i j : Nat} (hi : i < vs.length) (hj : j < vs.length) (hij : i ≠ j) {t : Rat}
    (t0 : 0 < t) (t1 : t < 1) : ¬OnSeg (lerp (vtx vs i) (vtx vs (i + 1)) t) (vtx vs j) (vtx vs (j + 1)) := by
  intro hz
  have hz0 : OnSeg (lerp (vtx vs i) (vtx vs (i + 1)) t) (vtx vs i) (vtx vs (i + 1)) :=
    ⟨t, by grind, by grind, rfl⟩
  have hne : lerp (vtx vs i) (vtx vs (i + 1)) t ≠ vtx vs (i + 1) := fun h =>
    absurd (lerp_eq_right (hs.nondeg i) h) (by grind)
  have hne' : lerp (vtx vs i) (vtx vs (i + 1)) t ≠ vtx vs i := fun h =>
    absurd (lerp_eq_left (hs.nondeg i) h) (by grind)
  generalize lerp (vtx vs i) (vtx vs (i + 1)) t = z at hz hz0 hne hne'
  by_cases e1 : nxt vs.length i = j
  · have hj' : OnSeg z (vtx vs (i + 1)) (vtx vs (i + 2)) := by
      have a1 : vtx vs j = vtx vs (i + 1) := by rw [← e1, vtx_nxt vs i hi]
      have a2 : vtx vs (j + 1) = vtx vs (i + 2) := by
        rw [← e1]; unfold nxt; split
        · rename_i h; rw [show i + 2 = (i + 1) + 1 from rfl, h]
          simp [vtx]
        · rfl
      rw [a1, a2] at hz; exact hz
    exact hne (hs.adj i _ hz0 hj')
  · by_cases e2 : nxt vs.length j = i
    · have hj' : OnSeg z (vtx vs (j + 1)) (vtx vs (j + 2)) := by
        have a1 : vtx vs (j + 1) = vtx vs i := by rw [← e2, vtx_nxt vs j hj]
        have a2 : vtx vs (j + 2) = vtx vs (i + 1) := by
          rw [← e2]; unfold nxt; split
          · rename_i h; rw [show j + 2 = (j + 1) + 1 from rfl, h]
            simp [vtx]
          · rfl
        rw [a1, a2]; exact hz0
      have := hs.adj j _ hz hj'
      rw [show vtx vs (j + 1) = vtx vs i by rw [← e2, vtx_nxt vs j hj]] at this
      exact hne' this
    · exact hs.apart i j hi hj hij e1 e2 ⟨_, hz0, hz⟩

/-- A closed piece strictly inside side `i` misses every other side. -/
theorem sub_off {i j : Nat} (hi : i < vs.length) (hj : j < vs.length) (hij : i ≠ j) {t0 t1 : Rat}
    (h0 : 0 < t0) (h1 : t0 < 1) (h2 : 0 < t1) (h3 : t1 < 1) :
    ¬Meets (lerp (vtx vs i) (vtx vs (i + 1)) t0) (lerp (vtx vs i) (vtx vs (i + 1)) t1)
      (vtx vs j) (vtx vs (j + 1)) := by
  rintro ⟨z, ⟨l, l0, l1, rfl⟩, hz⟩
  have e : lerp (lerp (vtx vs i) (vtx vs (i + 1)) t0) (lerp (vtx vs i) (vtx vs (i + 1)) t1) l =
      lerp (vtx vs i) (vtx vs (i + 1)) ((1 - l) * t0 + l * t1) := by
    ext <;> simp only [lerp] <;> grind
  rw [e] at hz
  exact interior_off hs hi hj hij (conv_pos h0 h2 l0 l1)
    (by have := conv_pos (o1 := 1 - t0) (o2 := 1 - t1) (by grind) (by grind) l0 l1; grind) hz

/-- Vertex `i+1` lies on no side but sides `i` and `i+1`. -/
theorem vertex_off {i j : Nat} (hi : i < vs.length) (hj : j < vs.length) (h1 : j ≠ i)
    (h2 : j ≠ nxt vs.length i) : ¬OnSeg (vtx vs (i + 1)) (vtx vs j) (vtx vs (j + 1)) := by
  intro hv
  have n3 := hs.three
  have ve : OnSeg (vtx vs (i + 1)) (vtx vs i) (vtx vs (i + 1)) := onSeg_right
  by_cases e2 : nxt vs.length j = i
  · -- side `j` is the one before side `i`
    have hi' := nxt_lt (n := vs.length) hi
    have v1 : OnSeg (vtx vs (i + 1)) (vtx vs (nxt vs.length i)) (vtx vs (nxt vs.length i + 1)) := by
      have a1 : vtx vs (nxt vs.length i) = vtx vs (i + 1) := vtx_nxt vs i hi
      rw [a1]; exact onSeg_left
    by_cases n3' : vs.length = 3
    · -- a triangle: side `j` is also the one after side `i + 1`, so the vertex is vertex `i`
      have e3 : vtx vs (j + 1) = vtx vs i := by rw [← e2, vtx_nxt vs j hj]
      have e4 : vtx vs (j + 2) = vtx vs (i + 1) := by
        rcases nxt_cases (n := vs.length) hj with ⟨a, b⟩ | ⟨a, b⟩
        · rw [b] at e2; subst e2
          rw [show j + 2 = (j + 1) + 1 from rfl, a]; simp [vtx]
        · rw [b] at e2; subst e2; rfl
      have := hs.adj j _ hv (by rw [e3, e4]; exact ve)
      rw [e3] at this
      exact hs.nondeg i this.symm
    · have hne : nxt vs.length (nxt vs.length i) ≠ j := by
        rcases nxt_cases (n := vs.length) hi with ⟨a, b⟩ | ⟨a, b⟩ <;>
        rcases nxt_cases (n := vs.length) hj with ⟨c, d⟩ | ⟨c, d⟩ <;>
        simp only [b, d] at e2 h2 ⊢ <;>
        first
          | (rcases nxt_cases (n := vs.length) (i := 0) (by omega) with ⟨e, f⟩ | ⟨e, f⟩ <;> rw [f] <;> omega)
          | (rcases nxt_cases (n := vs.length) (i := i + 1) (by omega) with ⟨e, f⟩ | ⟨e, f⟩ <;> rw [f] <;> omega)
          | omega
      exact hs.apart _ j hi' hj (Ne.symm h2) hne (by rw [e2]; intro h; rcases nxt_cases (n := vs.length) hi with
          ⟨a, b⟩ | ⟨a, b⟩ <;> omega) ⟨_, v1, hv⟩
  · exact hs.apart i j hi hj (Ne.symm h1) (Ne.symm h2) e2 ⟨_, ve, hv⟩

end simple

end Winding2d
