import TopoCheck.Proof

/-!
# The model's flood reaches every face it can

`flood_sound` (Proof.lean): what the flood reaches is reachable. Here the converse, where every
neighbour of a face is a face (as `checkLinks = []` gives): with `floodFuel`, the flood's result
holds its start and is closed under `nbrs`, so it holds every face reachable from the start
(`reached_iff`). Each face is pushed at most once (`flood` pushes only unseen faces, each once),
so the stack's length plus the unseen faces falls by one at each pop: `faces.size + 1` pops are
enough.
-/

namespace TopoCheck.Body
variable (b : Body)

/-- The faces not yet seen. -/
def unseen (seen : List Nat) : Nat := (List.range b.faces.size).countP (· ∉ seen)

theorem countP_drop_one {p : Nat → Bool} (l : List Nat) (hl : l.Nodup) (y : Nat) (hy : y ∈ l)
    (hp : p y = true) : l.countP p = l.countP (fun x => p x && x != y) + 1 := by
  induction l with
  | nil => simp at hy
  | cons a t ih =>
    rw [List.nodup_cons] at hl
    by_cases hay : a = y
    · subst hay
      have : t.countP (fun x => p x && x != a) = t.countP p := by
        apply List.countP_congr
        intro x hx
        have : x ≠ a := fun h => hl.1 (h ▸ hx)
        simp [this]
      simp [List.countP_cons, hp, this]
    · have hyt : y ∈ t := by
        rcases List.mem_cons.1 hy with h | h
        · exact absurd h.symm hay
        · exact h
      rw [List.countP_cons, List.countP_cons, ih hl.2 hyt]
      have : (a != y) = true := by simp [hay]
      cases p a <;> simp [this] <;> omega

theorem unseen_add (seen : List Nat) (y : Nat) (hy : y < b.faces.size) (hys : y ∉ seen) :
    b.unseen seen = b.unseen (seen ++ [y]) + 1 := by
  unfold unseen
  rw [countP_drop_one _ (List.nodup_range) y (List.mem_range.2 hy) (by simp [hys])]
  congr 1
  apply List.countP_congr
  intro x _
  by_cases h : x = y <;> simp [h]

/-- Pushing `new` (distinct faces, none seen) takes `new.length` off the unseen. -/
theorem unseen_append : ∀ (new seen : List Nat), new.Nodup → (∀ y ∈ new, y < b.faces.size ∧ y ∉ seen) →
    b.unseen seen = b.unseen (seen ++ new) + new.length
  | [], seen, _, _ => by simp
  | y :: rest, seen, hnd, hin => by
    rw [List.nodup_cons] at hnd
    have hy := hin y (by simp)
    rw [b.unseen_add seen y hy.1 hy.2]
    have := unseen_append rest (seen ++ [y]) hnd.2 (by
      intro z hz
      refine ⟨(hin z (by simp [hz])).1, ?_⟩
      simp only [List.mem_append, List.mem_singleton, not_or]
      exact ⟨(hin z (by simp [hz])).2, fun h => hnd.1 (h ▸ hz)⟩)
    rw [this]; simp [List.append_assoc]; omega

theorem nodup_eraseDups_len : ∀ (n : Nat) (l : List Nat), l.length ≤ n → l.eraseDups.Nodup
  | _, [], _ => by simp
  | 0, _ :: _, h => by simp at h
  | n + 1, a :: as, h => by
    rw [List.eraseDups_cons, List.nodup_cons]
    refine ⟨?_, nodup_eraseDups_len n _ ?_⟩
    · intro hm
      have := List.mem_filter.1 (List.mem_eraseDups.1 hm)
      simp at this
    · simp only [List.length_cons] at h
      exact Nat.le_trans (List.length_filter_le _ _) (by omega)

theorem nodup_eraseDups (l : List Nat) : l.eraseDups.Nodup := nodup_eraseDups_len l.length l (Nat.le_refl _)

/-- The flood's result holds what was seen and is closed under `nbrs`, given fuel for the stack
and every unseen face, a stack of seen faces, and every seen face off the stack done. -/
theorem flood_closed (hn : ∀ f, ∀ y ∈ b.nbrs f, y < b.faces.size) :
    ∀ fuel seen stack, stack.length + b.unseen seen ≤ fuel → (∀ x ∈ stack, x ∈ seen) →
      (∀ x ∈ seen, x ∉ stack → ∀ y ∈ b.nbrs x, y ∈ seen) →
      (∀ x ∈ seen, x ∈ b.flood fuel seen stack) ∧
      ∀ x ∈ b.flood fuel seen stack, ∀ y ∈ b.nbrs x, y ∈ b.flood fuel seen stack := by
  intro fuel
  induction fuel with
  | zero =>
    intro seen stack hfuel _ hdone
    have : stack = [] := List.eq_nil_of_length_eq_zero (by omega)
    subst this
    simp only [flood]
    exact ⟨fun x hx => hx, fun x hx y hy => hdone x hx (by simp) y hy⟩
  | succ n ih =>
    intro seen stack hfuel hst hdone
    cases stack with
    | nil =>
      simp only [flood]
      exact ⟨fun x hx => hx, fun x hx y hy => hdone x hx (by simp) y hy⟩
    | cons f st =>
      simp only [flood]
      have hnd := nodup_eraseDups ((b.nbrs f).filter (· ∉ seen))
      have hin : ∀ y ∈ ((b.nbrs f).filter (· ∉ seen)).eraseDups, y < b.faces.size ∧ y ∉ seen := by
        intro y hy
        have := List.mem_filter.1 (List.mem_eraseDups.1 hy)
        exact ⟨hn f y this.1, by simpa using this.2⟩
      have hu := b.unseen_append _ seen hnd hin
      have ⟨h1, h2⟩ := ih (seen ++ ((b.nbrs f).filter (· ∉ seen)).eraseDups)
        (((b.nbrs f).filter (· ∉ seen)).eraseDups ++ st)
        (by simp only [List.length_append, List.length_cons] at hfuel ⊢; omega)
        (by
          intro x hx
          rcases List.mem_append.1 hx with h | h
          · exact List.mem_append.2 (Or.inr h)
          · exact List.mem_append.2 (Or.inl (hst x (by simp [h]))))
        (by
          intro x hx hxs y hy
          rcases List.mem_append.1 hx with h | h
          · by_cases hxf : x = f
            · subst hxf
              by_cases hys : y ∈ seen
              · exact List.mem_append.2 (Or.inl hys)
              · exact List.mem_append.2 (Or.inr (List.mem_eraseDups.2 (List.mem_filter.2 ⟨hy, by simpa using hys⟩)))
            · have : x ∉ f :: st := by
                intro hm
                rcases List.mem_cons.1 hm with h' | h'
                · exact hxf h'
                · exact hxs (List.mem_append.2 (Or.inr h'))
              exact List.mem_append.2 (Or.inl (hdone x h this y hy))
          · exact absurd (List.mem_append.2 (Or.inl h)) hxs)
      exact ⟨fun x hx => h1 x (List.mem_append.2 (Or.inl hx)), h2⟩

/-- Closed under `nbrs` and holding `h`: holds all `h` reaches. -/
theorem reach_mem (h : Nat) (R : List Nat) (hh : h ∈ R) (hc : ∀ x ∈ R, ∀ y ∈ b.nbrs x, y ∈ R) :
    ∀ x, b.Reach h x → x ∈ R := by
  intro x hx
  induction hx with
  | refl => exact hh
  | step _ hy ih => exact hc _ ih _ hy

/-- With every neighbour a face, a shell's flood from a face reaches exactly what it reaches. -/
theorem reached_iff (hn : ∀ f, ∀ y ∈ b.nbrs f, y < b.faces.size) (s : Nat)
    (hh : (b.shellList s).headD 0 < b.faces.size) (x : Nat) :
    x ∈ b.reached s ↔ b.Reach ((b.shellList s).headD 0) x := by
  constructor
  · intro hx
    unfold reached at hx
    exact b.flood_sound _ _ _ _ (by simp [Reach.refl]) (by simp [Reach.refl]) x hx
  · intro hx
    have hu : b.unseen [(b.shellList s).headD 0] + 1 = b.faces.size := by
      have := b.unseen_add [] _ hh (by simp)
      simp only [List.nil_append] at this
      have h0 : b.unseen [] = b.faces.size := by simp [unseen]
      omega
    have ⟨h1, h2⟩ := b.flood_closed hn b.floodFuel [(b.shellList s).headD 0] [(b.shellList s).headD 0]
      (by unfold floodFuel; rw [List.length_singleton]; omega) (by simp) (by simp)
    exact b.reach_mem _ _ (h1 _ (by simp)) h2 x hx

end TopoCheck.Body
