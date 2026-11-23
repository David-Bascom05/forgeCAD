import ForgeBoolCore.RefineUnifyHas

/-!
# Group U, part 2: a union–find over positions names the Lean's classes

`unify` runs two union–finds over positions: of the result facets (faces) and of the edge cells
between two faces (edges). The Lean names the same classes by `cls`/`head` over a list of
covectors. For a list `(range n).map f` with `f` injective, pairs `E` that are sound (each relates)
and complete (each related pair is joined) give `Eqv E i j ↔ Conn r xs (f i) (f j)`
(`u_eqv_iff`), so the least position of a class (`Least`) is its `head` (`u_head`), and a class is
the positions with its root (`u_cls_filter`).
-/

open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

section
variable {α : Type}

theorem u_mem_range_map {f : Nat → α} {n : Nat} {x : α} :
    x ∈ (List.range n).map f ↔ ∃ i, i < n ∧ f i = x := by
  simp

theorem u_nodup_range_map {f : Nat → α} {n : Nat} (hinj : ∀ i j, i < n → j < n → f i = f j → i = j) :
    ((List.range n).map f).Nodup := by
  apply List.Nodup.map_on _ List.nodup_range
  intro a ha b hb h; simp at ha hb; exact hinj a b ha hb h

/-- **Sound and complete pairs: their classes are the closure.** -/
theorem u_eqv_iff [DecidableEq α] (f : Nat → α) (n : Nat) (r : α → α → Bool) (hr : ∀ x y, r x y = r y x)
    (E : List (Nat × Nat)) (hs : ∀ p ∈ E, p.1 < n ∧ p.2 < n ∧ r (f p.1) (f p.2) = true)
    (hc : ∀ i j, i < n → j < n → r (f i) (f j) = true → Eqv E i j)
    (hinj : ∀ i j, i < n → j < n → f i = f j → i = j) {i j : Nat} (hi : i < n) (hj : j < n) :
    Eqv E i j ↔ Conn r ((List.range n).map f) (f i) (f j) := by
  have hE : ∀ e ∈ E, e.1 < n ∧ e.2 < n := fun e he => ⟨(hs e he).1, (hs e he).2.1⟩
  constructor
  · intro h
    suffices H : ∀ y z, Eqv E y z → y < n → Conn r ((List.range n).map f) (f y) (f z) from H i j h hi
    intro y z hyz
    induction hyz with
    | refl _ => exact fun _ => Conn.refl
    | @edge p hp =>
      intro _
      obtain ⟨_, h2, h3⟩ := hs p hp
      exact Conn.step Conn.refl (u_mem_range_map.mpr ⟨_, h2, rfl⟩) h3
    | @symm y z h ih =>
      intro hz
      rcases Eqv.range hE h with e | ⟨hy, _⟩
      · subst e; exact Conn.refl
      · exact Conn.symm hr (u_mem_range_map.mpr ⟨_, hy, rfl⟩) (ih hy)
    | @trans x y z h1 _ ih1 ih2 =>
      intro hx
      have hy : y < n := by
        rcases Eqv.range hE h1 with e | ⟨_, hy⟩
        · exact e ▸ hx
        · exact hy
      exact (ih1 hx).trans (ih2 hy)
  · intro h
    suffices H : ∀ b, Conn r ((List.range n).map f) (f i) b → ∃ k, k < n ∧ f k = b ∧ Eqv E i k by
      obtain ⟨k, hk, hfk, hik⟩ := H _ h
      rw [hinj k j hk hj hfk] at hik; exact hik
    intro b hb
    induction hb with
    | refl => exact ⟨i, hi, rfl, Eqv.refl _⟩
    | @step b c _ hc' hbc ih =>
      obtain ⟨k, hk, rfl, hik⟩ := ih
      obtain ⟨m, hm, rfl⟩ := u_mem_range_map.mp hc'
      exact ⟨m, hm, rfl, hik.trans (hc k m hk hm hbc)⟩

theorem u_find_range {P : Nat → Bool} {n k : Nat} (hk : k < n) (hp : P k = true)
    (hmin : ∀ j, j < k → P j = false) : (List.range n).find? P = some k := by
  rw [List.find?_eq_some_iff_getElem]
  refine ⟨hp, k, by simpa using hk, by simp, fun j hj => ?_⟩
  simp only [List.getElem_range]; rw [hmin j hj]; rfl

/-- **The least position is the `head`.** -/
theorem u_head [DecidableEq α] (f : Nat → α) (n : Nat) (r : α → α → Bool) (E : List (Nat × Nat)) {Rt : Nat → Nat}
    (hl : Least E n Rt)
    (hiff : ∀ i j, i < n → j < n → (Eqv E i j ↔ Conn r ((List.range n).map f) (f i) (f j)))
    (hinj : ∀ i j, i < n → j < n → f i = f j → i = j) {t : Nat} (ht : t < n) :
    head r ((List.range n).map f) (f t) = some (f (Rt t)) ∧
    ∀ d, d < n → ((cls r ((List.range n).map f) (f t)).contains (f d) = true ↔ Rt d = Rt t) := by
  have hnd := u_nodup_range_map hinj
  have hmem : f t ∈ (List.range n).map f := u_mem_range_map.mpr ⟨t, ht, rfl⟩
  have hcls : ∀ d, d < n → ((cls r ((List.range n).map f) (f t)).contains (f d) = true ↔ Rt d = Rt t) := by
    intro d hd
    rw [List.contains_iff_mem, cls_spec r hnd hmem, ← hiff t d ht hd, least_iff hl ht hd]
    exact eq_comm
  refine ⟨?_, hcls⟩
  obtain ⟨hR, hRe, hRmin⟩ := hl t ht
  unfold head; simp only
  rw [List.find?_map]
  have hRR : Rt (Rt t) = Rt t := ((least_iff hl ht hR).mp hRe).symm
  rw [u_find_range (k := Rt t) hR (by simp only [Function.comp]; exact (hcls _ hR).mpr hRR)]
  · rfl
  · intro j hj
    simp only [Function.comp]
    cases h : (cls r ((List.range n).map f) (f t)).contains (f j)
    · rfl
    · exfalso
      have hjn : j < n := by omega
      have := (hcls j hjn).mp h
      have := hRmin j ((least_iff hl ht hjn).mpr this.symm)
      omega

/-- A list `(range n).map f` filtered by a class: the positions with its root. -/
theorem u_filter_range_map (f : Nat → α) (n : Nat) (P : α → Bool) :
    ((List.range n).map f).filter P = ((List.range n).filter (fun i => P (f i))).map f := by
  rw [List.filter_map]; rfl

end

end forge_bool_core
