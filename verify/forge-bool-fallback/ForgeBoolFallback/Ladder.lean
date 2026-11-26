/-!
# G2: the retry ladder composes (kernel/forge-bool/src/boolean.rs `plan`, `plan_retried`, `plan_fenced`)

Since the retry flags became one explicit `Rung` value (and `ODD` an output of the try), a try is
a function `once : Rung → Except Err P × Bool` (the plan or the refusal, and whether a closed
curve crossed springs an odd number of times). The ladder runs tries in a fixed order, deciding
the next rung from the last try's error alone.

This file models the ladder as written and proves what the end-to-end theorem needs of it:

* `plan_ok`: an `Ok` from the ladder is the `Ok` of one try, at some rung. So the ladder is
  correct on any input where every try's `Ok` is correct (`plan_correct`): one contract per try,
  not one per history of flags (§G G2 "no one contract holds" is gone).
* `plan_err`: a refusal from the ladder is a refusal some try made, or the first try's
  `Internal` reason (the ladder hands back the first loss, not a later one's). An `Unhandled` or
  `ZeroThickness` refusal therefore has the reason of an actual try.

What remains for G2 (ledger): the per-rung contract itself (each rung's reading of the
near-tangent case is a screen plus an exact test), and G3 (the post-hoc `check` after a retry),
which this file does not touch. Core Lean only.
-/

namespace ForgeBoolFallback.Ladder

/-- The rung: the flags a try reads (boolean.rs `Rung`). -/
structure Rung where
  careful : Bool := false
  chords : Bool := false
  noRunOn : Bool := false
  fences : Bool := false
  absorb : Bool := false
  deriving DecidableEq, Repr

def plain : Rung := {}

/-- The ways a try loses its way that the ladder reads; every other `Lost` is `other`. -/
inductive Lost where
  | inconsistentClassification
  | splitCrossed
  | other (k : Nat)
  deriving DecidableEq, Repr

/-- A try's refusal: `Internal` (lost its way: what the ladder retries) or any other refusal
(unsupported, zero thickness), named by an opaque code. -/
inductive Err where
  | internal (w : Lost)
  | refused (code : Nat)
  deriving DecidableEq, Repr

variable {P : Type}

/-- `r.map_err(|e| match e { Internal(_) => Internal(w), e => e })` -/
def keepFirst (w : Lost) : Except Err P → Except Err P
  | .error (.internal _) => .error (.internal w)
  | r => r

/-- `plan_fenced`: a try; lost with ODD set, once more with the fences. -/
def fenced (once : Rung → Except Err P × Bool) (r : Rung) : Except Err P :=
  match once r with
  | (.error (.internal w), true) => keepFirst w (once { r with fences := true }).1
  | (x, _) => x

def isInternal : Except Err P → Bool
  | .error (.internal _) => true
  | _ => false

/-- `plan_retried`: a fenced try; lost by an inconsistent classification or a crossed split, the
two readings (absorb, chords) in the order the loss suggests, then for a crossed split NO_RUN_ON. -/
def retried (once : Rung → Except Err P × Bool) (r : Rung) : Except Err P :=
  match fenced once r with
  | .error (.internal w) =>
    if w = .inconsistentClassification ∨ w = .splitCrossed then
      let crossed := decide (w = .splitCrossed)
      let r1 := fenced once { r with absorb := crossed, chords := !crossed }
      if !isInternal r1 then r1 else
      let r2 := fenced once { r with absorb := !crossed, chords := crossed }
      if !isInternal r2 then r2 else
      if !crossed then .error (.internal w) else
      keepFirst w (fenced once { r with noRunOn := true })
    else .error (.internal w)
  | x => x

/-- `plan`: the retried ladder; lost on a curved input, all of it again with CAREFUL. -/
def plan (once : Rung → Except Err P × Bool) (curved : Bool) : Except Err P :=
  match retried once plain with
  | .error (.internal w) =>
    if curved then keepFirst w (retried once { plain with careful := true }) else .error (.internal w)
  | x => x

/-- Some try's `Ok` or non-internal refusal, or an internal loss (any reason). -/
def FromTry (once : Rung → Except Err P × Bool) (x : Except Err P) : Prop :=
  (∃ r, (once r).1 = x ∧ isInternal x = false) ∨ isInternal x = true

theorem keepFirst_fromTry {once : Rung → Except Err P × Bool} {w : Lost} {x : Except Err P}
    (h : FromTry once x) : FromTry once (keepFirst w x) := by
  unfold keepFirst
  split
  · right; rfl
  · exact h

theorem fenced_fromTry (once : Rung → Except Err P × Bool) (r : Rung) :
    FromTry once (fenced once r) := by
  unfold fenced
  rcases hr : once r with ⟨x, b⟩
  cases x with
  | ok p => exact Or.inl ⟨r, by simp [hr], rfl⟩
  | error e =>
    cases e with
    | refused c => exact Or.inl ⟨r, by simp [hr], rfl⟩
    | internal w =>
      cases b with
      | false => exact Or.inr rfl
      | true =>
        apply keepFirst_fromTry
        rcases hx : (once { r with fences := true }).1 with e' | p
        · cases e' with
          | internal w' => exact Or.inr rfl
          | refused c => exact Or.inl ⟨_, hx, rfl⟩
        · exact Or.inl ⟨_, hx, rfl⟩

theorem retried_fromTry (once : Rung → Except Err P × Bool) (r : Rung) :
    FromTry once (retried once r) := by
  unfold retried
  have h0 := fenced_fromTry once r
  generalize fenced once r = f at h0 ⊢
  cases f with
  | ok p => exact h0
  | error e =>
    cases e with
    | refused c => exact h0
    | internal w =>
      try dsimp only
      by_cases hw : (w = .inconsistentClassification ∨ w = .splitCrossed)
      · rw [ite_eq_left hw]
        try dsimp only
        split
        · exact fenced_fromTry once _
        · split
          · exact fenced_fromTry once _
          · split
            · exact Or.inr rfl
            · exact keepFirst_fromTry (fenced_fromTry once _)
      · rw [ite_eq_right hw]
        exact Or.inr rfl

theorem plan_fromTry (once : Rung → Except Err P × Bool) (curved : Bool) :
    FromTry once (plan once curved) := by
  unfold plan
  have h0 := retried_fromTry once plain
  generalize retried once plain = f at h0 ⊢
  cases f with
  | ok p => exact h0
  | error e =>
    cases e with
    | refused c => exact h0
    | internal w =>
      try dsimp only
      cases curved with
      | false => exact Or.inr rfl
      | true => exact keepFirst_fromTry (retried_fromTry once _)

/-- **An `Ok` from the ladder is one try's `Ok`.** -/
theorem plan_ok {once : Rung → Except Err P × Bool} {curved : Bool} {p : P}
    (h : plan once curved = .ok p) : ∃ r, (once r).1 = .ok p := by
  have := plan_fromTry once curved
  rw [h] at this
  rcases this with ⟨r, hr, _⟩ | hi
  · exact ⟨r, hr⟩
  · simp [isInternal] at hi

/-- **A refusal from the ladder that is not a loss is one try's refusal.** -/
theorem plan_err {once : Rung → Except Err P × Bool} {curved : Bool} {c : Nat}
    (h : plan once curved = .error (.refused c)) : ∃ r, (once r).1 = .error (.refused c) := by
  have := plan_fromTry once curved
  rw [h] at this
  rcases this with ⟨r, hr, _⟩ | hi
  · exact ⟨r, hr⟩
  · simp [isInternal] at hi

/-- **The ladder is correct where every try is**: for any property `Good` of an `Ok` plan
(the D-chain's conclusion), if every try's `Ok` is `Good`, the ladder's `Ok` is. -/
theorem plan_correct {once : Rung → Except Err P × Bool} {curved : Bool} {Good : P → Prop}
    (hall : ∀ r p, (once r).1 = .ok p → Good p) {p : P} (h : plan once curved = .ok p) :
    Good p := by
  obtain ⟨r, hr⟩ := plan_ok h
  exact hall r p hr

end ForgeBoolFallback.Ladder
