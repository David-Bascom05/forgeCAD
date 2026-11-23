import ForgeExact.Regular

/-!
# Patterns: the union of the rigid images, decided once

research/ops-spec/design.md §1. A pattern applies one feature's tool `T` at the images `gᵢ T` under
motions `gᵢ` (forge-transform: rigid motions and mirrors, each a homeomorphism of space). The
kernel makes one n-ary boolean of the part with every copy (forge-model lib.rs:1997–2028); its
`may_empty` path applies them one at a time (lib.rs:953). This file proves those are one result.

* `Homeo`, `img`: a homeomorphism and the image of a set under it (as the preimage under the
  inverse, so that it stays a predicate).
* P1 `reg_img`: images commute with regularisation, so a copy of a solid is a solid (`solid_img`).
* P2 `reg_diff_reg`: `reg (reg X \ C) = reg (X \ C)` for every `X` and `C`;
  `reg_union_reg`: `reg (reg X ∪ Y) = reg (X ∪ Y)` when `X ⊆ reg X`.
* `steps_eq`: a feature history of regularised adds and cuts of solid tools, applied one at a time,
  is the regularisation of its pointwise formula: the one n-ary boolean.
* `cuts_perm`, `adds_perm`: the copies of one pattern (all one op) may be applied in any order.
* P3 `steps_dedup`: a copy equal to an earlier one of the same op may be dropped.

Generic: any `Topology X`. No `sorry`; `Ops/Axioms.lean` checks the axioms.
-/

namespace ForgeExact.Ops

open ForgeExact Topology

universe u

set_option linter.unusedSectionVars false

variable {X : Type u} [Topology X]

/-! ## Homeomorphisms and images -/

/-- A homeomorphism: inverse maps, each pulling open sets back to open sets. -/
structure Homeo (X : Type u) [Topology X] where
  fwd : X → X
  inv : X → X
  inv_fwd : ∀ x, inv (fwd x) = x
  fwd_inv : ∀ y, fwd (inv y) = y
  open_fwd : ∀ U : Pred X, IsOpen U → IsOpen fun x => U (fwd x)
  open_inv : ∀ U : Pred X, IsOpen U → IsOpen fun y => U (inv y)

/-- The image of `S`: `y` is in it iff its preimage is in `S`. -/
def img (h : Homeo X) (S : Pred X) : Pred X := fun y => S (h.inv y)

/-- The inverse homeomorphism. -/
def Homeo.symm (h : Homeo X) : Homeo X where
  fwd := h.inv
  inv := h.fwd
  inv_fwd := h.fwd_inv
  fwd_inv := h.inv_fwd
  open_fwd := h.open_inv
  open_inv := h.open_fwd

theorem interior_img (h : Homeo X) (S : Pred X) : interior (img h S) = img h (interior S) := by
  apply Pred.ext; intro y; constructor
  · rintro ⟨U, hU, hUS, hy⟩
    refine ⟨fun x => U (h.fwd x), h.open_fwd U hU, fun x hx => ?_, by simp only [h.fwd_inv]; exact hy⟩
    have := hUS _ hx; simp only [img, h.inv_fwd] at this; exact this
  · rintro ⟨U, hU, hUS, hy⟩
    exact ⟨fun y => U (h.inv y), h.open_inv U hU, fun y hy => hUS _ hy, hy⟩

theorem closure_img (h : Homeo X) (S : Pred X) : closure (img h S) = img h (closure S) := by
  apply Pred.ext; intro y; constructor
  · intro hy U hU hxU
    obtain ⟨z, hz, hzS⟩ := hy (fun y => U (h.inv y)) (h.open_inv U hU) hxU
    exact ⟨h.inv z, hz, hzS⟩
  · intro hy U hU hyU
    obtain ⟨z, hz, hzS⟩ := hy (fun x => U (h.fwd x)) (h.open_fwd U hU)
      (by simp only [h.fwd_inv]; exact hyU)
    exact ⟨h.fwd z, hz, by simp only [img, h.inv_fwd]; exact hzS⟩

/-- **P1.** Images commute with regularisation. -/
theorem reg_img (h : Homeo X) (S : Pred X) : reg (img h S) = img h (reg S) := by
  simp only [reg, interior_img, closure_img]

/-- A solid: a regular closed set. -/
def Solid (S : Pred X) : Prop := reg S = S

/-- A copy of a solid is a solid. -/
theorem solid_img (h : Homeo X) {S : Pred X} (hS : Solid S) : Solid (img h S) := by
  unfold Solid at *; rw [reg_img, hS]

/-! ## Regularised steps -/

/-- An open set inside the closure of `S` meets `S` wherever it is nonempty: the set of its
points within the open `U` lies in the closure of `U ∩ S`. -/
theorem open_inter_closure {U S : Pred X} (hU : IsOpen U) :
    (Pred.inter U (closure S)).Sub (closure (Pred.inter U S)) := by
  intro x ⟨hxU, hxS⟩ V hV hxV
  obtain ⟨y, ⟨hyV, hyU⟩, hyS⟩ := hxS (Pred.inter V U) (isOpen_inter _ _ hV hU) ⟨hxV, hxU⟩
  exact ⟨y, hyV, hyU, hyS⟩

theorem closure_sub_of_sub {S T : Pred X} (h : S.Sub (closure T)) : (closure S).Sub (closure T) :=
  fun x hx => closure_closure T x (closure_mono h x hx)

/-- **P2, cuts.** Regularising before a cut changes nothing: `reg (reg X \ C) = reg (X \ C)`. -/
theorem reg_diff_reg (Y C : Pred X) : reg ((reg Y).diff C) = reg (Y.diff C) := by
  apply Pred.ext; intro x; constructor
  · -- interior (cl int Y \ C) lies in the closure of int (Y \ C)
    apply closure_sub_of_sub
    intro z ⟨U, hU, hUsub, hzU⟩ V hV hzV
    have hzc : closure (interior Y) z := (hUsub z hzU).1
    obtain ⟨w, ⟨hwV, hwU⟩, hwY⟩ := hzc (Pred.inter V U) (isOpen_inter _ _ hV hU) ⟨hzV, hzU⟩
    obtain ⟨W, hW, hWY, hwW⟩ := hwY
    refine ⟨w, hwV, Pred.inter W U, isOpen_inter _ _ hW hU, ?_, hwW, hwU⟩
    intro q ⟨hqW, hqU⟩
    exact ⟨hWY q hqW, (hUsub q hqU).2⟩
  · -- interior (Y \ C) is open, inside int Y ⊆ cl int Y, and off C
    apply closure_mono
    intro z ⟨U, hU, hUsub, hzU⟩
    refine ⟨U, hU, fun q hq => ⟨?_, (hUsub q hq).2⟩, hzU⟩
    exact sub_closure _ q ⟨U, hU, fun r hr => (hUsub r hr).1, hq⟩

/-- **P2, adds.** `reg (reg Y ∪ Z) ⊆ reg (Y ∪ Z)` always. -/
theorem reg_union_reg_sub (Y Z : Pred X) : (reg ((reg Y).union Z)).Sub (reg (Y.union Z)) := by
  apply closure_sub_of_sub
  intro z ⟨U, hU, hUsub, hzU⟩ V hV hzV
  -- either V ∩ U meets int Y, or it misses it, and then it misses cl int Y and lies in Z
  by_cases hm : ∃ w, (V w ∧ U w) ∧ interior Y w
  · obtain ⟨w, ⟨hwV, hwU⟩, ⟨W, hW, hWY, hwW⟩⟩ := hm
    exact ⟨w, hwV, W, hW, fun q hq => Or.inl (hWY q hq), hwW⟩
  · refine ⟨z, hzV, Pred.inter V U, isOpen_inter _ _ hV hU, ?_, hzV, hzU⟩
    intro q ⟨hqV, hqU⟩
    rcases hUsub q hqU with hq | hq
    · exfalso
      obtain ⟨w, hw, hwY⟩ := hq (Pred.inter V U) (isOpen_inter _ _ hV hU) ⟨hqV, hqU⟩
      exact hm ⟨w, hw, hwY⟩
    · exact Or.inr hq

/-- Every point of `Y` is a limit of interior points: `Y ⊆ reg Y`. Solids are such, and so is
every pointwise formula of solids a history builds (`steps_full`). -/
def Full (Y : Pred X) : Prop := Y.Sub (reg Y)

theorem reg_union_reg {Y Z : Pred X} (hY : Full Y) : reg ((reg Y).union Z) = reg (Y.union Z) := by
  apply Pred.ext; intro x
  refine ⟨reg_union_reg_sub Y Z x, ?_⟩
  apply closure_mono
  apply interior_mono
  intro q hq
  rcases hq with hq | hq
  · exact Or.inl (hY q hq)
  · exact Or.inr hq

theorem reg_mono {S T : Pred X} (h : S.Sub T) : (reg S).Sub (reg T) :=
  closure_mono (interior_mono h)

theorem full_of_solid {S : Pred X} (hS : Solid S) : Full S := by
  intro x hx; unfold Solid at hS; rw [hS]; exact hx

theorem full_union {Y Z : Pred X} (hY : Full Y) (hZ : Full Z) : Full (Y.union Z) := by
  intro x hx
  rcases hx with hx | hx
  · exact reg_mono (fun q hq => Or.inl hq) x (hY x hx)
  · exact reg_mono (fun q hq => Or.inr hq) x (hZ x hx)

/-- A solid is closed. -/
theorem closed_of_solid {C : Pred X} (hC : Solid C) : (closure C).Sub C := by
  intro x hx; unfold Solid reg at hC; rw [← hC]
  rw [← hC] at hx; exact closure_closure _ x hx

/-- Cutting a closed set keeps `Full`. -/
theorem full_diff {Y C : Pred X} (hY : Full Y) (hC : (closure C).Sub C) : Full (Y.diff C) := by
  intro x ⟨hxY, hxC⟩ U hU hxU
  -- a neighbourhood V of x misses C
  have : ∃ V, IsOpen V ∧ V x ∧ ∀ w, V w → ¬C w := by
    apply Classical.byContradiction; intro hno
    apply hxC; apply hC; intro V hV hxV
    apply Classical.byContradiction; intro hmiss
    exact hno ⟨V, hV, hxV, fun w hw hcw => hmiss ⟨w, hw, hcw⟩⟩
  obtain ⟨V, hV, hxV, hVC⟩ := this
  obtain ⟨w, ⟨hwU, hwV⟩, ⟨W, hW, hWY, hwW⟩⟩ :=
    hY x hxY (Pred.inter U V) (isOpen_inter _ _ hU hV) ⟨hxU, hxV⟩
  exact ⟨w, hwU, Pred.inter W V, isOpen_inter _ _ hW hV,
    fun q ⟨hqW, hqV⟩ => ⟨hWY q hqW, hVC q hqV⟩, hwW, hwV⟩

/-! ## Histories: one at a time is one n-ary boolean -/

/-- One step of a history: add or cut a tool. -/
inductive Step (X : Type u)
  | add (T : Pred X)
  | cut (T : Pred X)

/-- The pointwise effect of a step. -/
def Step.apply : Step X → Pred X → Pred X
  | .add T, Y => Y.union T
  | .cut T, Y => Y.diff T

def Step.tool : Step X → Pred X
  | .add T => T
  | .cut T => T

/-- The pointwise formula of a history: the one n-ary boolean (`combine`, decided cell by cell). -/
def formula (S : Pred X) (ss : List (Step X)) : Pred X := ss.foldl (fun Y s => s.apply Y) S

/-- The history applied one step at a time, regularising after each. -/
def steps (S : Pred X) (ss : List (Step X)) : Pred X := ss.foldl (fun Y s => reg (s.apply Y)) S

theorem reg_step (s : Step X) {Y : Pred X} (hY : Full Y) :
    reg (s.apply (reg Y)) = reg (s.apply Y) := by
  cases s with
  | add T => exact reg_union_reg hY
  | cut T => exact reg_diff_reg Y T

theorem full_step (s : Step X) (hs : Solid s.tool) {Y : Pred X} (hY : Full Y) :
    Full (s.apply Y) := by
  cases s with
  | add T => exact full_union hY (full_of_solid hs)
  | cut T => exact full_diff hY (closed_of_solid hs)

/-- Every intermediate formula of a history of solid tools is `Full`. -/
theorem formula_full {S : Pred X} (hS : Full S) (ss : List (Step X))
    (hss : ∀ s ∈ ss, Solid s.tool) : Full (formula S ss) := by
  induction ss generalizing S with
  | nil => exact hS
  | cons s ss ih =>
    exact ih (full_step s (hss s (List.mem_cons_self ..)) hS)
      fun t ht => hss t (List.mem_cons_of_mem _ ht)

/-- **One at a time is one n-ary boolean.** For a solid part and solid tools, regularising after
every step gives the regularised pointwise formula of the whole history. -/
theorem steps_eq {S : Pred X} (hS : Solid S) (ss : List (Step X))
    (hss : ∀ s ∈ ss, Solid s.tool) : steps S ss = reg (formula S ss) := by
  -- the invariant: after a prefix, the accumulator is `reg` of a `Full` formula
  suffices h : ∀ (Y : Pred X), Full Y → ∀ ss : List (Step X), (∀ s ∈ ss, Solid s.tool) →
      ss.foldl (fun Y s => reg (s.apply Y)) (reg Y) = reg (formula Y ss) by
    have := h S (full_of_solid hS) ss hss
    unfold Solid at hS; rw [hS] at this; exact this
  intro Y hY ss hss
  induction ss generalizing Y with
  | nil => rfl
  | cons s ss ih =>
    simp only [List.foldl_cons, formula]
    rw [reg_step s hY]
    exact ih _ (full_step s (hss s (List.mem_cons_self ..)) hY)
      (fun t ht => hss t (List.mem_cons_of_mem _ ht))

/-! ## The copies of one pattern -/

/-- The union of a list of sets. -/
def bigUnion (Ts : List (Pred X)) : Pred X := fun x => ∃ T ∈ Ts, T x

theorem formula_cuts (S : Pred X) (Ts : List (Pred X)) :
    formula S (Ts.map Step.cut) = S.diff (bigUnion Ts) := by
  induction Ts generalizing S with
  | nil => apply Pred.ext; intro x; simp [formula, Pred.diff, bigUnion]
  | cons T Ts ih =>
    simp only [List.map_cons, formula, List.foldl_cons] at *
    rw [ih]; apply Pred.ext; intro x
    simp only [Step.apply, Pred.diff, bigUnion, List.mem_cons]
    constructor
    · rintro ⟨⟨h1, h2⟩, h3⟩; refine ⟨h1, ?_⟩; rintro ⟨U, hU | hU, hx⟩
      · subst hU; exact h2 hx
      · exact h3 ⟨U, hU, hx⟩
    · rintro ⟨h1, h2⟩
      exact ⟨⟨h1, fun h => h2 ⟨T, Or.inl rfl, h⟩⟩, fun ⟨U, hU, hx⟩ => h2 ⟨U, Or.inr hU, hx⟩⟩

theorem formula_adds (S : Pred X) (Ts : List (Pred X)) :
    formula S (Ts.map Step.add) = S.union (bigUnion Ts) := by
  induction Ts generalizing S with
  | nil => apply Pred.ext; intro x; simp [formula, Pred.union, bigUnion]
  | cons T Ts ih =>
    simp only [List.map_cons, formula, List.foldl_cons] at *
    rw [ih]; apply Pred.ext; intro x
    simp only [Step.apply, Pred.union, bigUnion, List.mem_cons]
    constructor
    · rintro ((h | h) | ⟨U, hU, hx⟩)
      · exact Or.inl h
      · exact Or.inr ⟨T, Or.inl rfl, h⟩
      · exact Or.inr ⟨U, Or.inr hU, hx⟩
    · rintro (h | ⟨U, hU | hU, hx⟩)
      · exact Or.inl (Or.inl h)
      · subst hU; exact Or.inl (Or.inr hx)
      · exact Or.inr ⟨U, hU, hx⟩

theorem bigUnion_perm {Ts Ts' : List (Pred X)} (h : Ts.Perm Ts') : bigUnion Ts = bigUnion Ts' :=
  Pred.ext fun _ => ⟨fun ⟨T, hT, hx⟩ => ⟨T, h.mem_iff.mp hT, hx⟩,
    fun ⟨T, hT, hx⟩ => ⟨T, h.mem_iff.mpr hT, hx⟩⟩

/-- A pattern's cuts, one at a time in any order, are one result: `reg (S \ ⋃ copies)`. -/
theorem cuts_eq {S : Pred X} (hS : Solid S) (Ts : List (Pred X)) (hTs : ∀ T ∈ Ts, Solid T) :
    steps S (Ts.map Step.cut) = reg (S.diff (bigUnion Ts)) := by
  rw [steps_eq hS _ (by
    intro s hs; obtain ⟨T, hT, rfl⟩ := List.mem_map.mp hs; exact hTs T hT), formula_cuts]

theorem adds_eq {S : Pred X} (hS : Solid S) (Ts : List (Pred X)) (hTs : ∀ T ∈ Ts, Solid T) :
    steps S (Ts.map Step.add) = reg (S.union (bigUnion Ts)) := by
  rw [steps_eq hS _ (by
    intro s hs; obtain ⟨T, hT, rfl⟩ := List.mem_map.mp hs; exact hTs T hT), formula_adds]

theorem cuts_perm {S : Pred X} (hS : Solid S) {Ts Ts' : List (Pred X)} (h : Ts.Perm Ts')
    (hTs : ∀ T ∈ Ts, Solid T) :
    steps S (Ts.map Step.cut) = steps S (Ts'.map Step.cut) := by
  rw [cuts_eq hS Ts hTs, cuts_eq hS Ts' (fun T hT => hTs T (h.mem_iff.mpr hT)), bigUnion_perm h]

theorem adds_perm {S : Pred X} (hS : Solid S) {Ts Ts' : List (Pred X)} (h : Ts.Perm Ts')
    (hTs : ∀ T ∈ Ts, Solid T) :
    steps S (Ts.map Step.add) = steps S (Ts'.map Step.add) := by
  rw [adds_eq hS Ts hTs, adds_eq hS Ts' (fun T hT => hTs T (h.mem_iff.mpr hT)), bigUnion_perm h]

/-- The pattern of a tool: its images under the copies' motions, the identity first. -/
def copies (T : Pred X) (gs : List (Homeo X)) : List (Pred X) := T :: gs.map fun g => img g T

theorem copies_solid {T : Pred X} (hT : Solid T) (gs : List (Homeo X)) :
    ∀ C ∈ copies T gs, Solid C := by
  intro C hC
  rcases List.mem_cons.mp hC with rfl | hC
  · exact hT
  · obtain ⟨g, _, rfl⟩ := List.mem_map.mp hC; exact solid_img g hT

/-- **The pattern spec.** A cut pattern is `reg (S \ ⋃ gᵢ T)`, however the kernel orders it. -/
theorem pattern_cut {S T : Pred X} (hS : Solid S) (hT : Solid T) (gs : List (Homeo X)) :
    steps S ((copies T gs).map Step.cut) = reg (S.diff (bigUnion (copies T gs))) :=
  cuts_eq hS _ (copies_solid hT gs)

theorem pattern_add {S T : Pred X} (hS : Solid S) (hT : Solid T) (gs : List (Homeo X)) :
    steps S ((copies T gs).map Step.add) = reg (S.union (bigUnion (copies T gs))) :=
  adds_eq hS _ (copies_solid hT gs)

/-- **P3.** Dropping a repeated copy changes nothing (lib.rs:1062 `distinct`). -/
theorem bigUnion_dedup {Ts : List (Pred X)} {T : Pred X} (h : T ∈ Ts) :
    bigUnion (Ts ++ [T]) = bigUnion Ts :=
  Pred.ext fun _ => ⟨fun ⟨U, hU, hx⟩ => by
    rcases List.mem_append.mp hU with hU | hU
    · exact ⟨U, hU, hx⟩
    · rw [List.mem_singleton.mp hU] at hx; exact ⟨T, h, hx⟩,
    fun ⟨U, hU, hx⟩ => ⟨U, List.mem_append_left _ hU, hx⟩⟩

end ForgeExact.Ops
