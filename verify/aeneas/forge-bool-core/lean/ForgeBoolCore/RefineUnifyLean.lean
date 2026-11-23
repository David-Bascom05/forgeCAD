import ForgeBoolCore.RefineUnifyEdges

/-!
# Group U, part 8: the Lean `unify`, by positions

For a result listed as `(range n).map cr` whose join classes are a union–find's (`Least`), the
Lean's heads, faces and face names (`key`) are the positions' roots (`u_heads_eq`, `u_faceOf_eq`,
`u_key_eq`). The same holds for the edge classes (by `u_head`).
-/

open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

section
variable (L : ForgeExact.DecideOnce.Ledger) (kt : KeepTab) (n : Nat) (cr : Nat → Cov)
  (E : List (Nat × Nat)) (Rt : Nat → Nat)

theorem u_least_facts {E : List (Nat × Nat)} {n : Nat} {Rt : Nat → Nat} (hl : Least E n Rt) :
    ∀ i, i < n → Rt i < n ∧ Rt i ≤ i ∧ Rt (Rt i) = Rt i := by
  intro i hi
  obtain ⟨h1, h2, h3⟩ := hl i hi
  exact ⟨h1, h3 i (Eqv.refl _), ((least_iff hl hi h1).mp h2).symm⟩

/-- A class filter over a list `(range n).map cr`, by roots. -/
theorem u_cls_filter {α : Type} [DecidableEq α] {f : Nat → α} {n : Nat} {r : α → α → Bool} {E : List (Nat × Nat)}
    {Rt : Nat → Nat} (hl : Least E n Rt)
    (hiff : ∀ i j, i < n → j < n → (Eqv E i j ↔ Conn r ((List.range n).map f) (f i) (f j)))
    (hinj : ∀ i j, i < n → j < n → f i = f j → i = j) :
    ((List.range n).map f).filter (fun c => head r ((List.range n).map f) c == some c) = (hdL Rt n).map f ∧
    ∀ t, t < n → ((List.range n).map f).filter (fun d => (cls r ((List.range n).map f) (f t)).contains d) =
      ((List.range n).filter fun d => Rt d = Rt t).map f := by
  have hf := u_least_facts hl
  constructor
  · rw [u_filter_range_map, hdL]
    congr 1
    apply List.filter_congr
    intro t ht
    have ht' : t < n := List.mem_range.mp ht
    rw [(u_head f n r E hl hiff hinj ht').1]
    apply Bool.eq_iff_iff.mpr
    simp only [beq_iff_eq, Option.some.injEq, decide_eq_true_eq]
    exact ⟨fun h => hinj _ _ (hf t ht').1 ht' h, fun h => by rw [h]⟩
  · intro t ht
    rw [u_filter_range_map]
    congr 1
    apply List.filter_congr
    intro d hd
    have hd' : d < n := List.mem_range.mp hd
    apply Bool.eq_iff_iff.mpr
    rw [(u_head f n r E hl hiff hinj ht).2 d hd']
    simp

end

/-- **The face names.** For a result position `p`, the Lean's `key` (the head of its face) is its
root's facet. -/
theorem u_key {α : Type} [DecidableEq α] {f : Nat → α} {n : Nat} {Rt : Nat → Nat}
    (hf : ∀ i, i < n → Rt i < n ∧ Rt i ≤ i ∧ Rt (Rt i) = Rt i)
    (hinj : ∀ i j, i < n → j < n → f i = f j → i = j) (FO : α → List α)
    (hFO : ∀ t, t < n → Rt t = t → FO (f t) = ((List.range n).filter fun d => Rt d = t).map f)
    {p : Nat} (hp : p < n) :
    (((((hdL Rt n).map f).flatMap fun h => (FO h).map fun c => (c, h)).find? fun e => e.1 == f p).map
      Prod.snd).getD (f p) = f (Rt p) := by
  obtain ⟨hR, _, hRR⟩ := hf p hp
  have huniq : ∀ e ∈ ((hdL Rt n).map f).flatMap (fun h => (FO h).map fun c => (c, h)), e.1 = f p → e.2 = f (Rt p) := by
    intro e he he1
    simp only [List.mem_flatMap, List.mem_map] at he
    obtain ⟨h, ⟨t, ht, rfl⟩, c, hc, rfl⟩ := he
    obtain ⟨htn, htt⟩ := u_mem_hdL.mp ht
    rw [hFO t htn htt] at hc
    simp only [List.mem_map, List.mem_filter, List.mem_range, decide_eq_true_eq] at hc
    obtain ⟨d, ⟨hdn, hdt⟩, rfl⟩ := hc
    simp only at he1 ⊢
    rw [hinj d p hdn hp he1] at hdt; rw [hdt]
  have hmem : (f p, f (Rt p)) ∈ ((hdL Rt n).map f).flatMap (fun h => (FO h).map fun c => (c, h)) := by
    simp only [List.mem_flatMap, List.mem_map]
    refine ⟨f (Rt p), ⟨Rt p, u_mem_hdL.mpr ⟨hR, hRR⟩, rfl⟩, f p, ?_, rfl⟩
    rw [hFO _ hR hRR]
    exact List.mem_map.mpr ⟨p, by simp [hp], rfl⟩
  cases hfd : (((hdL Rt n).map f).flatMap fun h => (FO h).map fun c => (c, h)).find? fun e => e.1 == f p with
  | none =>
    have := List.find?_eq_none.mp hfd _ hmem; simp at this
  | some e =>
    have h1 := List.find?_some hfd
    have h2 := List.mem_of_find?_eq_some hfd
    simp only [beq_iff_eq] at h1
    simp only [Option.map_some, Option.getD_some]
    exact huniq e h2 h1

/-- Face numbers and face names: `dedup` commutes with naming a face by its head. -/
theorem u_dedup_names {α : Type} [DecidableEq α] {f : Nat → α} {n : Nat} {Rt : Nat → Nat}
    (hf : ∀ i, i < n → Rt i < n ∧ Rt i ≤ i ∧ Rt (Rt i) = Rt i)
    (hinj : ∀ i j, i < n → j < n → f i = f j → i = j) {AL : List Nat} (hAL : ∀ p ∈ AL, p < n) :
    dedup (AL.map fun p => f (Rt p)) =
      (dedup (AL.map fun p => (hdL Rt n).idxOf (Rt p))).map fun k => f ((hdL Rt n).getD k 0) := by
  have hmem : ∀ p ∈ AL, Rt p ∈ hdL Rt n := fun p hp => u_mem_hdL.mpr ⟨(hf p (hAL p hp)).1, (hf p (hAL p hp)).2.2⟩
  have e : (AL.map fun p => f (Rt p)) =
      (AL.map fun p => (hdL Rt n).idxOf (Rt p)).map fun k => f ((hdL Rt n).getD k 0) := by
    rw [List.map_map]; apply List.map_congr_left; intro p hp
    simp only [Function.comp]; rw [u_getD_idxOf (hmem p hp)]
  rw [e, ForgeBoolCore.Dedup.dedup_map]
  intro a ha b hb hab
  simp only [List.mem_map] at ha hb
  obtain ⟨pa, hpa, rfl⟩ := ha
  obtain ⟨pb, hpb, rfl⟩ := hb
  rw [u_getD_idxOf (hmem pa hpa), u_getD_idxOf (hmem pb hpb)] at hab
  have := hinj _ _ (hf pa (hAL pa hpa)).1 (hf pb (hAL pb hpb)).1 hab
  rw [this]

theorem u_contains_map {α : Type} [BEq α] [LawfulBEq α] {g : Nat → α} {A : List Nat} {x : Nat}
    (hg : ∀ a ∈ A, g a = g x → a = x) : (A.map g).contains (g x) = decide (x ∈ A) := by
  apply Bool.eq_iff_iff.mpr
  rw [List.contains_iff_mem, decide_eq_true_iff, List.mem_map]
  constructor
  · rintro ⟨a, ha, e⟩; rw [← hg a ha e]; exact ha
  · intro h; exact ⟨x, h, rfl⟩

theorem u_all_congr {α : Type} {l : List α} {p q : α → Bool} (h : ∀ x ∈ l, p x = q x) : l.all p = l.all q := by
  induction l with
  | nil => rfl
  | cons a t ih => simp only [List.all_cons]; rw [h a (by simp), ih fun x hx => h x (by simp [hx])]

/-- `sameSet` of named lists is `SS` of the numbers. -/
theorem u_sameSet_map {g : Nat → Cov} {A B : List Nat} (hg : ∀ a ∈ A ++ B, ∀ b ∈ A ++ B, g a = g b → a = b) :
    Ledger.sameSet (A.map g) (B.map g) = SS A B := by
  unfold Ledger.sameSet SS
  rw [List.all_map, List.all_map]
  have h1 : A.all ((fun x => (B.map g).contains x) ∘ g) = A.all (fun x => decide (x ∈ B)) := by
    apply u_all_congr; intro x hx; simp only [Function.comp]
    exact u_contains_map (fun a ha e => hg a (List.mem_append_right _ ha) x (List.mem_append_left _ hx) e)
  have h2 : B.all ((fun x => (A.map g).contains x) ∘ g) = B.all (fun x => decide (x ∈ A)) := by
    apply u_all_congr; intro x hx; simp only [Function.comp]
    exact u_contains_map (fun a ha e => hg a (List.mem_append_left _ ha) x (List.mem_append_right _ hx) e)
  rw [h1, h2]

end forge_bool_core
