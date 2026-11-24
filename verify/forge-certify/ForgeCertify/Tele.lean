import ForgeCertify.Sign

/-!
# Along one line

The `+x` line from a point meets distinct planes at distinct positions. Each plane `P` flips the
operands its triangles at that point belong to (`α P`). The boolean `op` of the operands'
memberships at the point (the parity of everything ahead) is the parity, over the planes ahead, of
`op`'s change across each: `op (A ⊕ α) ⊕ op A`, `A` the flips beyond that plane (`tele`). Nothing far
away is kept (`op 0 = false`).
-/

namespace ForgeCertify

/-- The vector of `n` booleans. -/
def vec (n : Nat) (f : Nat → Bool) : List Bool := (List.range n).map f

theorem exists_min {β : Type} (t : β → Rat) : ∀ (L : List β), L ≠ [] → ∃ m ∈ L, ∀ P ∈ L, t m ≤ t P
  | [], h => absurd rfl h
  | [a], _ => ⟨a, by simp, by simp⟩
  | a :: b :: L, _ => by
    obtain ⟨m, hm, hmin⟩ := exists_min t (b :: L) (by simp)
    by_cases h : t a ≤ t m
    · refine ⟨a, by simp, fun P hP => ?_⟩
      rcases List.mem_cons.mp hP with rfl | hP
      · exact Rat.le_refl
      · exact Rat.le_trans h (hmin P hP)
    · refine ⟨m, List.mem_cons_of_mem _ hm, fun P hP => ?_⟩
      rcases List.mem_cons.mp hP with rfl | hP
      · grind
      · exact hmin P hP

theorem par_filter_split {β : Type} (L : List β) (p q : β → Bool) (f : β → Bool) :
    par (L.map fun P => p P && f P) = (par (L.map fun P => (p P && q P) && f P) ^^
      par (L.map fun P => (p P && !q P) && f P)) := by
  rw [← par_map_xor]; congr 1; apply List.map_congr_left; intro P _
  cases p P <;> cases q P <;> cases f P <;> rfl

/-- Over a list without repeats, a sum of terms all false but at `m`'s is `m`'s term. -/
theorem par_single {β : Type} (L : List β) (hnd : L.Nodup) (m : β) (hm : m ∈ L) (f : β → Bool)
    (h : ∀ P ∈ L, P ≠ m → f P = false) : par (L.map f) = f m := by
  induction L with
  | nil => simp at hm
  | cons a L ih =>
    obtain ⟨ha, hnd⟩ := List.nodup_cons.mp hnd
    simp only [List.map_cons, par_cons]
    by_cases e : a = m
    · subst e
      rw [par_all_false]
      · simp
      · intro b hb
        obtain ⟨P, hP, rfl⟩ := List.mem_map.mp hb
        exact h P (List.mem_cons_of_mem _ hP) (fun e => ha (e ▸ hP))
    · rw [h a (by simp) e, ih hnd ((List.mem_cons.mp hm).resolve_left (Ne.symm e))
        (fun P hP => h P (List.mem_cons_of_mem _ hP))]
      simp

theorem countP_or_one {β : Type} [DecidableEq β] (L : List β) (hnd : L.Nodup) (m : β) (hm : m ∈ L)
    (p : β → Bool) (hp : p m = false) :
    L.countP (fun P => p P || decide (P = m)) = L.countP p + 1 := by
  induction L with
  | nil => simp at hm
  | cons a L ih =>
    obtain ⟨ha, hnd⟩ := List.nodup_cons.mp hnd
    rw [List.countP_cons, List.countP_cons]
    by_cases e : a = m
    · subst e
      have : L.countP (fun P => p P || decide (P = a)) = L.countP p := by
        apply List.countP_congr; intro P hP; have : P ≠ a := fun h => ha (h ▸ hP); simp [this]
      rw [this]; simp [hp]
    · rw [ih hnd ((List.mem_cons.mp hm).resolve_left (Ne.symm e))]; simp [e]; omega

/-- **Telescoping along a line.** -/
theorem tele (n : Nat) (op : List Bool → Bool) (h0 : op (vec n fun _ => false) = false) {β : Type}
    [DecidableEq β] (L : List β) (hnd : L.Nodup) (t : β → Rat)
    (hinj : ∀ P ∈ L, ∀ P' ∈ L, t P = t P' → P = P') (α : β → Nat → Bool) :
    ∀ (k : Nat) (x₀ : Rat), (L.filter fun P => decide (x₀ < t P)).length = k →
    op (vec n fun i => par (L.map fun P => decide (x₀ < t P) && α P i)) =
      par (L.map fun P => decide (x₀ < t P) &&
        (op (vec n fun i => par (L.map fun P' => decide (t P < t P') && α P' i) ^^ α P i) ^^
          op (vec n fun i => par (L.map fun P' => decide (t P < t P') && α P' i)))) := by
  intro k
  induction k with
  | zero =>
    intro x₀ hk
    have hno : ∀ P ∈ L, decide (x₀ < t P) = false := by
      intro P hP; apply Classical.byContradiction; intro h
      have : P ∈ L.filter fun P => decide (x₀ < t P) := List.mem_filter.mpr ⟨hP, by simpa using h⟩
      rw [List.length_eq_zero_iff.mp hk] at this; simp at this
    have hr : par (L.map fun P => decide (x₀ < t P) &&
        (op (vec n fun i => par (L.map fun P' => decide (t P < t P') && α P' i) ^^ α P i) ^^
          op (vec n fun i => par (L.map fun P' => decide (t P < t P') && α P' i)))) = false := by
      apply par_all_false; intro b hb; obtain ⟨P, hP, rfl⟩ := List.mem_map.mp hb; rw [hno P hP]; rfl
    rw [hr]
    have : (fun i => par (L.map fun P => decide (x₀ < t P) && α P i)) = fun _ => false := by
      funext i; apply par_all_false; intro b hb
      obtain ⟨P, hP, rfl⟩ := List.mem_map.mp hb; rw [hno P hP]; rfl
    rw [this, h0]
  | succ k ih =>
    intro x₀ hk
    have hne : (L.filter fun P => decide (x₀ < t P)) ≠ [] := by intro h; rw [h] at hk; simp at hk
    obtain ⟨m, hm, hmin⟩ := exists_min t _ hne
    obtain ⟨hmL, hmx⟩ := List.mem_filter.mp hm
    simp only [decide_eq_true_eq] at hmx
    -- past `m` there is one plane fewer
    have split : ∀ P ∈ L, decide (x₀ < t P) = (decide (t m < t P) || decide (P = m)) := by
      intro P hP
      by_cases e : P = m
      · subst e; simp [hmx]
      · have : x₀ < t P ↔ t m < t P := by
          constructor
          · intro h
            have hle := hmin P (List.mem_filter.mpr ⟨hP, by simpa using h⟩)
            rcases Rat.le_iff_lt_or_eq.mp hle with h' | h'
            · exact h'
            · exact absurd (hinj m hmL P hP h').symm e
          · intro h; grind
        simp [this, e]
    have hk' : (L.filter fun P => decide (t m < t P)).length = k := by
      have e1 : (L.filter fun P => decide (x₀ < t P)) = L.filter fun P => decide (t m < t P) || decide (P = m) :=
        List.filter_congr (fun P hP => split P hP)
      rw [e1, ← List.countP_eq_length_filter, countP_or_one L hnd m hmL (fun P => decide (t m < t P))
        (by simp)] at hk
      rw [← List.countP_eq_length_filter]; omega
    have ihm := ih (t m) hk'
    -- the sums over the planes past x₀ are those past m, and m's own
    have sumx : ∀ (f : β → Bool), par (L.map fun P => decide (x₀ < t P) && f P) =
        (f m ^^ par (L.map fun P => decide (t m < t P) && f P)) := by
      intro f
      rw [List.map_congr_left (fun P hP => by rw [split P hP])]
      have e2 : par (L.map fun P => (decide (t m < t P) || decide (P = m)) && f P) =
          (par (L.map fun P => decide (P = m) && f P) ^^ par (L.map fun P => decide (t m < t P) && f P)) := by
        rw [← par_map_xor]; congr 1; apply List.map_congr_left; intro P _
        by_cases e : P = m
        · subst e; simp
        · simp [e]
      rw [e2, par_single L hnd m hmL (fun P => decide (P = m) && f P) (fun P _ e => by simp [e])]
      simp
    rw [sumx (fun P => op (vec n fun i => par (L.map fun P' => decide (t P < t P') && α P' i) ^^ α P i) ^^
          op (vec n fun i => par (L.map fun P' => decide (t P < t P') && α P' i)))]
    rw [← ihm]
    have hv : (fun i => par (L.map fun P => decide (x₀ < t P) && α P i)) =
        fun i => par (L.map fun P' => decide (t m < t P') && α P' i) ^^ α m i := by
      funext i; rw [sumx (fun P => α P i)]; cases α m i <;> simp
    rw [hv]
    cases op (vec n fun i => par (L.map fun P' => decide (t m < t P') && α P' i) ^^ α m i) <;>
      cases op (vec n fun i => par (L.map fun P' => decide (t m < t P') && α P' i)) <;> rfl

end ForgeCertify
