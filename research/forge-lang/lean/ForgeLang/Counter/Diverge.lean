import ForgeLang.Edits

/-!
# P2: termination needs a bound

`fn f(n) = n == 0 ? 0 : f(n - 1)` terminates on every count and on no negative number. As a
relation (the semantics a language without a budget has) `f(-1)` has no value at all: evaluation is
not a function. Two ways out, either enough:

* a budget: every run ends, and `run_mono` says a bigger budget never changes a finished result;
* a type that makes the recursion structural: if `n` is a `Count` (a natural number), `n - 1` on
  `n ≠ 0` is structural descent and termination is a theorem about the program, not a budget.

So "fuel" is not essential; *some* bound is.
-/

namespace ForgeLang.Counter

inductive Down : Int → Int → Prop
  | zero : Down 0 0
  | step {n r : Int} : n ≠ 0 → Down (n - 1) r → Down n r

theorem down_nonneg {n r : Int} (h : Down n r) : 0 ≤ n := by
  induction h with
  | zero => exact Int.le_refl 0
  | step _ _ ih => omega

theorem no_value (r : Int) : ¬ Down (-1) r := fun h => by have := down_nonneg h; omega

/-- With a budget the same program is a total function: a typed error for `-1`, a model for `3`. -/
def downDefs : Defs :=
  { fns := [("f", ("x", .ite (.eq (.var "x") (.cst 0)) (.cst 0) (.call "f" (.add (.var "x") (.cst (-1))))))] }

def downDoc (n : Int) : Doc := { defs := downDefs, body := [.feat "a" (.call "f" (.lit 0))], σ := fun _ => n }

theorem down_budget :
    run 40 (downDoc (-1)) = .error .fuel ∧ (run 40 (downDoc 3)).map (fun r => r.1.nums (fun _ => 3)) =
      .ok [([.nm "a"], 0)] := ⟨by decide, by decide⟩

/-- The count is observed at every step, so it is a structural param: editing it re-runs. -/
theorem down_count_observed : (run 40 (downDoc 3)).map (fun r => r.2.contains 0) = .ok true := by decide

/-- Structural recursion on a count needs no budget: Lean accepts it as total as written. -/
def downNat : Nat → Nat
  | 0 => 0
  | n + 1 => downNat n

end ForgeLang.Counter
