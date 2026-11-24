/-!
# Two `Internal` refusals of `split_face`'s cycle walk that cannot happen

kernel/forge-bool/src/boolean.rs `split_face_in`, the cycle walk:

```
for s in 0..half.len() {
    if used[s] { continue; }
    let from = walked.len();
    let mut i = s;
    while !used[i] {
        used[i] = true;
        walked.push(i);
        i = next(i)?;
        if walked.len() - from > half.len() { return Err(Lost::SplitOpen) }   // (1)
    }
    ends.push(walked.len());
}
```

and later, `plane_split`, on each cycle `w` that is not an outer one:
`w.iter().max_by(..).ok_or_else(|| Lost::EmptyCycle)` (2).

* **(1) never fires** (`walk_length_le`): each pass of the loop marks one more half used, and the
  loop runs only while the half it is at is unused; so a walk is no longer than the number of
  halves unused when it starts, at most `half.len()`.
* **(2) never fires** (`walk_ne_nil`): a walk starts at an unused `s`, so the loop runs at least
  once and the cycle has at least one half (and `walks[k]` is still whole there: only the outer
  cycles were taken out before).

The model: halves are `Fin n`, `used` a function to `Bool`, `next` any function (its `?` refusal
is another site). The loop is run with fuel, and both bounds hold for every fuel: the code's
unbounded loop is the walk with fuel above its number of passes (finite, at most the unused count,
by the same bound), and the test (1) reads a prefix of the walk, no longer than all of it. Core
Lean only.
-/

namespace ForgeBoolFallback.Walk

variable {n : Nat}

/-- How many halves are unused. -/
def unused (used : Fin n → Bool) : Nat :=
  (List.finRange n).countP fun i => !used i

/-- Mark half `i` used. -/
def mark (used : Fin n → Bool) (i : Fin n) : Fin n → Bool :=
  fun j => if j = i then true else used j

/-- The loop: from half `i`, while it is unused, mark it, push it, go to `next i`. Returns the
halves pushed and the `used` after. -/
def walk (next : Fin n → Fin n) : Nat → (Fin n → Bool) → Fin n → List (Fin n) × (Fin n → Bool)
  | 0, used, _ => ([], used)
  | fuel + 1, used, i =>
    if used i then ([], used)
    else
      let r := walk next fuel (mark used i) (next i)
      (i :: r.1, r.2)

theorem countP_mark (used : Fin n → Bool) (i : Fin n) (hi : used i = false) :
    ∀ l : List (Fin n), i ∈ l → l.Nodup →
      l.countP (fun j => !mark used i j) + 1 = l.countP (fun j => !used j) := by
  intro l
  induction l with
  | nil => intro h; cases h
  | cons a l ih =>
    intro hmem hnd
    have hnd' : l.Nodup := (List.nodup_cons.mp hnd).2
    have ha : a ∉ l := (List.nodup_cons.mp hnd).1
    rw [List.countP_cons, List.countP_cons]
    by_cases hai : a = i
    · subst hai
      have same : l.countP (fun j => !mark used a j) = l.countP (fun j => !used j) := by
        apply List.countP_congr
        intro j hj
        have hne : j ≠ a := fun h => ha (h ▸ hj)
        simp [mark, hne]
      rw [same]
      simp [mark, hi]
    · have hl : i ∈ l := by
        rcases List.mem_cons.mp hmem with h | h
        · exact absurd h.symm hai
        · exact h
      have h := ih hl hnd'
      have hm : mark used i a = used a := by simp [mark, hai]
      rw [hm]
      omega

theorem filter_mark_lt (used : Fin n → Bool) (i : Fin n) (hi : used i = false) :
    unused (mark used i) < unused used := by
  have := countP_mark used i hi _ (List.mem_finRange i) (List.nodup_finRange n)
  unfold unused
  omega

/-- **(1)**: a walk is no longer than the halves unused when it starts. -/
theorem walk_length_le (next : Fin n → Fin n) :
    ∀ fuel used i, (walk next fuel used i).1.length ≤ unused used := by
  intro fuel
  induction fuel with
  | zero => intro used i; simp [walk]
  | succ fuel ih =>
    intro used i
    unfold walk
    by_cases hu : used i = true
    · simp [hu]
    · have hf : used i = false := by simpa using hu
      simp only [hf]
      have h1 := ih (mark used i) (next i)
      have h2 := filter_mark_lt used i hf
      simp
      omega

theorem unused_le (used : Fin n → Bool) : unused used ≤ n := by
  unfold unused
  exact Nat.le_trans List.countP_le_length (by simp)

/-- **(1)**, as the code tests it: a walk's length never exceeds `half.len()`. -/
theorem splitOpen_never (next : Fin n → Fin n) (fuel : Nat) (used : Fin n → Bool) (i : Fin n) :
    ¬ (walk next fuel used i).1.length > n :=
  Nat.not_lt.mpr (Nat.le_trans (walk_length_le next fuel used i) (unused_le used))

/-- **(2)**: a walk from an unused half has at least one half. -/
theorem walk_ne_nil (next : Fin n → Fin n) (fuel : Nat) (used : Fin n → Bool) (s : Fin n)
    (hs : used s = false) : (walk next (fuel + 1) used s).1 ≠ [] := by
  simp [walk, hs]

end ForgeBoolFallback.Walk
