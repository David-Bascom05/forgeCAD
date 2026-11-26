/-!
# H.batch's `MemberSound`, the combination part (kernel/forge-bool/src/shape.rs `Shape::member`)

`Shape::member(p, m)` answers `In` (p is inside the solid, clear of its boundary by m), `Out`
(outside, clear by m) or `Near`. For a `Csg` it folds its parts' answers in Kleene's three-valued
logic, skipping a part where the running answer is `Out` under a subtract or an intersect.

Here, over any space and any solids (sets as predicates), with `ball` the points within m of p:
`In S` means the ball lies in S, and `Out S` means the ball misses S. If every part's answer is
sound in that sense, the Csg's answer is sound for the combined solid (`csg_sound`). With
`ball p` (the ball holds its centre), a sound `In` gives p ∈ S and a sound `Out` gives p ∉ S:
the exact membership that `Batch.MemberSound` asks for (`sound_mem`).

The leaves (`Prism`, `Revolved`, …) are float screens; their soundness (an `In`/`Out` decided
with a margin larger than the rounding) is the IEEE part, left to the screen proofs. Core Lean
only.
-/

namespace ForgeBoolFallback.Member

inductive Op where
  | union
  | subtract
  | intersect
  deriving DecidableEq, Repr

inductive Member where
  | inside
  | outside
  | near
  deriving DecidableEq, Repr

variable {α : Type}

/-- The combined solid. -/
def combine : Op → (α → Prop) → (α → Prop) → (α → Prop)
  | .union, a, b => fun x => a x ∨ b x
  | .subtract, a, b => fun x => a x ∧ ¬ b x
  | .intersect, a, b => fun x => a x ∧ b x

/-- An answer is sound for solid `S` on the ball: `In` ⇒ the ball is in S, `Out` ⇒ it misses S. -/
def Sound (ball : α → Prop) (S : α → Prop) : Member → Prop
  | .inside => ∀ x, ball x → S x
  | .outside => ∀ x, ball x → ¬ S x
  | .near => True

/-- One step of the Csg fold (shape.rs: the `match op` inside `Shape::Csg`'s loop, with the
`continue` where `v` is `Out` and the op is not a union). -/
def step (op : Op) (v : Member) (w : Member) : Member :=
  if v = .outside ∧ op ≠ .union then v else
  match op with
  | .union =>
    if v = .inside ∨ w = .inside then .inside
    else if v = .outside ∧ w = .outside then .outside else .near
  | .subtract =>
    if v = .outside ∨ w = .inside then .outside
    else if v = .inside ∧ w = .outside then .inside else .near
  | .intersect =>
    if v = .outside ∨ w = .outside then .outside
    else if v = .inside ∧ w = .inside then .inside else .near

theorem step_sound {ball : α → Prop} {A B : α → Prop} {op : Op} {v w : Member}
    (hv : Sound ball A v) (hw : Sound ball B w) : Sound ball (combine op A B) (step op v w) := by
  unfold step
  by_cases hs : v = .outside ∧ op ≠ .union
  · rw [ite_eq_left hs]
    obtain ⟨rfl, hop⟩ := hs
    cases op with
    | union => exact absurd rfl hop
    | subtract => intro x hx h; exact hv x hx h.1
    | intersect => intro x hx h; exact hv x hx h.1
  · rw [ite_eq_right hs]
    cases op with
    | union =>
      simp only
      by_cases h1 : v = .inside ∨ w = .inside
      · rw [ite_eq_left h1]
        rcases h1 with rfl | rfl
        · intro x hx; exact Or.inl (hv x hx)
        · intro x hx; exact Or.inr (hw x hx)
      · rw [ite_eq_right h1]
        by_cases h2 : v = .outside ∧ w = .outside
        · rw [ite_eq_left h2]
          obtain ⟨rfl, rfl⟩ := h2
          intro x hx h
          rcases h with h | h
          · exact hv x hx h
          · exact hw x hx h
        · rw [ite_eq_right h2]; trivial
    | subtract =>
      simp only
      by_cases h1 : v = .outside ∨ w = .inside
      · rw [ite_eq_left h1]
        rcases h1 with rfl | rfl
        · intro x hx h; exact hv x hx h.1
        · intro x hx h; exact h.2 (hw x hx)
      · rw [ite_eq_right h1]
        by_cases h2 : v = .inside ∧ w = .outside
        · rw [ite_eq_left h2]
          obtain ⟨rfl, rfl⟩ := h2
          intro x hx; exact ⟨hv x hx, hw x hx⟩
        · rw [ite_eq_right h2]; trivial
    | intersect =>
      simp only
      by_cases h1 : v = .outside ∨ w = .outside
      · rw [ite_eq_left h1]
        rcases h1 with rfl | rfl
        · intro x hx h; exact hv x hx h.1
        · intro x hx h; exact hw x hx h.2
      · rw [ite_eq_right h1]
        by_cases h2 : v = .inside ∧ w = .inside
        · rw [ite_eq_left h2]
          obtain ⟨rfl, rfl⟩ := h2
          intro x hx; exact ⟨hv x hx, hw x hx⟩
        · rw [ite_eq_right h2]; trivial

/-- A Csg's parts after the first: each op with the part's solid and its answer. -/
structure Part (α : Type) where
  op : Op
  solid : α → Prop
  answer : Member

/-- The Csg's solid and answer: the parts folded left. -/
def foldSolid (first : α → Prop) (rest : List (Part α)) : α → Prop :=
  rest.foldl (fun s p => combine p.op s p.solid) first

def foldAnswer (v : Member) (rest : List (Part α)) : Member :=
  rest.foldl (fun v p => step p.op v p.answer) v

/-- **The Csg's answer is sound** when the first part's and every other part's answers are. -/
theorem csg_sound {ball : α → Prop} :
    ∀ (rest : List (Part α)) (first : α → Prop) (v : Member),
      Sound ball first v → (∀ p ∈ rest, Sound ball p.solid p.answer) →
      Sound ball (foldSolid first rest) (foldAnswer v rest) := by
  intro rest
  induction rest with
  | nil => intro first v hv _; exact hv
  | cons p ps ih =>
    intro first v hv hall
    simp only [foldSolid, foldAnswer, List.foldl_cons]
    exact ih _ _ (step_sound hv (hall p (List.mem_cons_self ..)))
      (fun q hq => hall q (List.mem_cons_of_mem _ hq))

/-- A sound decided answer gives the exact membership of the ball's centre. -/
theorem sound_mem {ball : α → Prop} {S : α → Prop} {p : α} (hp : ball p) :
    (Sound ball S .inside → S p) ∧ (Sound ball S .outside → ¬ S p) :=
  ⟨fun h => h p hp, fun h => h p hp⟩

end ForgeBoolFallback.Member
