import ForgeLang.Errors

/-!
# The language's comparisons, logic and arithmetic, in the core

The core has `==` and `<` (`Expr.eq`, `Expr.lt`: observed, the result a constant 0 or 1) and the
unobserved operators `+`, `*`, `-`, `/`, `%`, `^` (`Expr.add`, `Expr.mul`, `Expr.bin`). The rest of
Forge's operators are abbreviations of these, so every theorem about `Expr` (P1 to P5, and the
inductions under them) covers them with no new case. Each equation here says that the
abbreviation means what `eval.rs` does:

* `<=`, `>`, `>=`, `!=`: both operands evaluated, left first, both observed (`binop`'s
  `observe_value` on `x` and `y`), the result 1 or 0;
* `not a`: `a == 0`;
* `a and b`, `a or b`: short-circuit. `b` is evaluated only when `a` does not decide (`binop`'s
  early return), and nothing more is observed: `a` is already a comparison's result, a constant
  with no sites (Rust's `Value::Bool` carries no provenance);
* `-a`: `0 - a`.

`expect` is `ifs c [] []`: it observes its comparison and makes no feature (`expect_obs`).
-/

namespace ForgeLang

namespace Expr
def le (a b : Expr) : Expr := .lt a (.add b (.cst 1))
def gt (a b : Expr) : Expr := .eq (.lt a (.add b (.cst 1))) (.cst 0)
def ge (a b : Expr) : Expr := .eq (.lt a b) (.cst 0)
def ne (a b : Expr) : Expr := .eq (.eq a b) (.cst 0)
def not (a : Expr) : Expr := .eq a (.cst 0)
def and (a b : Expr) : Expr := .ite a b (.cst 0)
def or (a b : Expr) : Expr := .ite a (.cst 1) b
def neg (a : Expr) : Expr := .bin .sub (.cst 0) a
end Expr

/-- `expect a < b` and the like: an observation, no feature. -/
def Stmt.expect (c : Expr) : Stmt := .ifs c [] []

variable {n : Nat} {D : Defs} {σ : Site → Int} {ρ : Env} {a b : Expr} {ta tb : Tr} {la lb : Log}

theorem evalE_lt (k) : evalE (k + 1) D σ ρ (.lt a b) = (do
    let (ta, la) ← evalE k D σ ρ a
    let (tb, lb) ← evalE k D σ ρ b
    .ok (.const (if ta.den σ < tb.den σ then 1 else 0), la ++ lb ++ obs D ta ++ obs D tb)) := rfl
theorem evalE_eq (k) : evalE (k + 1) D σ ρ (.eq a b) = (do
    let (ta, la) ← evalE k D σ ρ a
    let (tb, lb) ← evalE k D σ ρ b
    .ok (.const (if ta.den σ = tb.den σ then 1 else 0), la ++ lb ++ obs D ta ++ obs D tb)) := rfl
theorem evalE_add (k) : evalE (k + 1) D σ ρ (.add a b) = (do
    let (ta, la) ← evalE k D σ ρ a
    let (tb, lb) ← evalE k D σ ρ b
    .ok (.add ta tb, la ++ lb)) := rfl
theorem evalE_bin (k o) : evalE (k + 1) D σ ρ (.bin o a b) = (do
    let (ta, la) ← evalE k D σ ρ a
    let (tb, lb) ← evalE k D σ ρ b
    .ok (.bin o ta tb, la ++ lb)) := rfl
theorem evalE_cst (k c) : evalE (k + 1) D σ ρ (.cst c) = .ok (.const c, []) := rfl
theorem evalE_ite (k) {c : Expr} : evalE (k + 1) D σ ρ (.ite c a b) = (do
    let (tc, lc) ← evalE k D σ ρ c
    let (t, l) ← evalE k D σ ρ (if tc.den σ ≠ 0 then a else b)
    .ok (t, lc ++ obs D tc ++ l)) := rfl

private theorem itp {α : Type} {c : Prop} [Decidable c] {x y : α} (h : c) : (if c then x else y) = x := by
  simp [h]
private theorem itn {α : Type} {c : Prop} [Decidable c] {x y : α} (h : ¬ c) : (if c then x else y) = y := by
  simp [h]

private theorem pos_of_ok (h : evalE n D σ ρ a = .ok (ta, la)) : ∃ k, n = k + 1 := by
  cases n with
  | zero => simp [evalE] at h
  | succ k => exact ⟨k, rfl⟩

/-- The observed comparison, as `eval.rs`'s `binop` does it. -/
def cmpRes (D : Defs) (c : Bool) (ta tb : Tr) (la lb : Log) : Tr × Log :=
  (.const (if c then 1 else 0), la ++ lb ++ obs D ta ++ obs D tb)

theorem le_eval (ha : evalE n D σ ρ a = .ok (ta, la)) (hb : evalE n D σ ρ b = .ok (tb, lb)) :
    evalE (n + 2) D σ ρ (a.le b) = .ok (cmpRes D (decide (ta.den σ ≤ tb.den σ)) ta tb la lb) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  have ha' := evalE_mono ha (m := k + 1 + 1) (by omega)
  simp only [Expr.le, evalE_lt, evalE_add, evalE_cst, ha', hb, ok_bind, cmpRes, obs, Tr.sites, Tr.den]
  show Except.ok (Tr.const (if ta.den σ < tb.den σ + 1 then (1:Int) else 0), _) = _
  by_cases h : ta.den σ ≤ tb.den σ
  · rw [itp (by omega)]; by_cases tr : D.track <;> simp [tr, h]
  · rw [itn (by omega)]; by_cases tr : D.track <;> simp [tr, h]

theorem ge_eval (ha : evalE n D σ ρ a = .ok (ta, la)) (hb : evalE n D σ ρ b = .ok (tb, lb)) :
    evalE (n + 2) D σ ρ (a.ge b) = .ok (cmpRes D (decide (tb.den σ ≤ ta.den σ)) ta tb la lb) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  simp only [Expr.ge, evalE_eq, evalE_lt, evalE_cst, ha, hb, ok_bind, cmpRes, obs, Tr.sites, Tr.den]
  show Except.ok (Tr.const (if (if ta.den σ < tb.den σ then (1:Int) else 0) = 0 then (1:Int) else 0), _) = _
  by_cases h : tb.den σ ≤ ta.den σ
  · rw [itn (by omega : ¬ ta.den σ < tb.den σ), itp rfl]; by_cases tr : D.track <;> simp [tr, h]
  · rw [itp (by omega : ta.den σ < tb.den σ), itn (by omega)]; by_cases tr : D.track <;> simp [tr, h]

theorem gt_eval (ha : evalE n D σ ρ a = .ok (ta, la)) (hb : evalE n D σ ρ b = .ok (tb, lb)) :
    evalE (n + 3) D σ ρ (a.gt b) = .ok (cmpRes D (decide (tb.den σ < ta.den σ)) ta tb la lb) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  have ha' := evalE_mono ha (m := k + 1 + 1) (by omega)
  simp only [Expr.gt, evalE_eq, evalE_lt, evalE_add, evalE_cst, ha', hb, ok_bind, cmpRes, obs, Tr.sites, Tr.den]
  show Except.ok (Tr.const (if (if ta.den σ < tb.den σ + 1 then (1:Int) else 0) = 0 then (1:Int) else 0), _) = _
  by_cases h : tb.den σ < ta.den σ
  · rw [itn (by omega : ¬ ta.den σ < tb.den σ + 1), itp rfl]; by_cases tr : D.track <;> simp [tr, h]
  · rw [itp (by omega : ta.den σ < tb.den σ + 1), itn (by omega)]; by_cases tr : D.track <;> simp [tr, h]

theorem ne_eval (ha : evalE n D σ ρ a = .ok (ta, la)) (hb : evalE n D σ ρ b = .ok (tb, lb)) :
    evalE (n + 2) D σ ρ (a.ne b) = .ok (cmpRes D (decide (ta.den σ ≠ tb.den σ)) ta tb la lb) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  simp only [Expr.ne, evalE_eq, evalE_cst, ha, hb, ok_bind, cmpRes, obs, Tr.sites, Tr.den]
  show Except.ok (Tr.const (if (if ta.den σ = tb.den σ then (1:Int) else 0) = 0 then (1:Int) else 0), _) = _
  by_cases h : ta.den σ = tb.den σ
  · rw [itp h, itn (by omega)]; by_cases tr : D.track <;> simp [tr, h]
  · rw [itn h, itp rfl]; by_cases tr : D.track <;> simp [tr, h]

theorem not_eval (ha : evalE n D σ ρ a = .ok (ta, la)) :
    evalE (n + 1) D σ ρ a.not = .ok (.const (if ta.den σ = 0 then 1 else 0), la ++ obs D ta) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  simp only [Expr.not, evalE_eq, evalE_cst, ha, ok_bind, obs, Tr.sites, Tr.den]
  show Except.ok (Tr.const (if ta.den σ = 0 then (1:Int) else 0), _) = _
  by_cases tr : D.track <;> simp [tr]

/-- `a and b` with `a` a comparison's result (a constant): false decides, `b` unevaluated. -/
theorem and_false {c : Int} (ha : evalE n D σ ρ a = .ok (.const c, la)) (hc : c = 0) :
    evalE (n + 1) D σ ρ (a.and b) = .ok (.const 0, la) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  have ha' := evalE_mono ha (m := k + 1) (by omega)
  simp [Expr.and, evalE_ite, evalE_cst, ha', hc, obs, Tr.sites, Tr.den]

theorem and_true {c : Int} (ha : evalE n D σ ρ a = .ok (.const c, la)) (hc : c ≠ 0)
    (hb : evalE n D σ ρ b = .ok (tb, lb)) :
    evalE (n + 1) D σ ρ (a.and b) = .ok (tb, la ++ lb) := by
  simp [Expr.and, evalE_ite, ha, hb, hc, obs, Tr.sites, Tr.den]

theorem or_true {c : Int} (ha : evalE n D σ ρ a = .ok (.const c, la)) (hc : c ≠ 0) :
    evalE (n + 1) D σ ρ (a.or b) = .ok (.const 1, la) := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  have ha' := evalE_mono ha (m := k + 1) (by omega)
  simp [Expr.or, evalE_ite, evalE_cst, ha, hc, obs, Tr.sites, Tr.den]

theorem or_false {c : Int} (ha : evalE n D σ ρ a = .ok (.const c, la)) (hc : c = 0)
    (hb : evalE n D σ ρ b = .ok (tb, lb)) :
    evalE (n + 1) D σ ρ (a.or b) = .ok (tb, la ++ lb) := by
  simp [Expr.or, evalE_ite, ha, hb, hc, obs, Tr.sites, Tr.den]

/-- Negation is arithmetic: `-p` carries `p`'s sites and has no handle, unobserved. -/
theorem neg_eval (ha : evalE n D σ ρ a = .ok (ta, la)) :
    evalE (n + 1) D σ ρ a.neg = .ok (.bin .sub (.const 0) ta, la) ∧
      (Tr.bin .sub (.const 0) ta).den σ = - ta.den σ ∧ (Tr.bin .sub (.const 0) ta).handle = none := by
  obtain ⟨k, rfl⟩ := pos_of_ok ha
  refine ⟨?_, by simp [Tr.den, AOp.app], rfl⟩
  have h0 : evalE (k + 1) D σ ρ (.cst 0) = .ok (.const 0, []) := rfl
  simp [Expr.neg, evalE_bin, h0, ha]

/-- `expect`: what it compares is observed, no feature is made, nothing is bound. -/
theorem expect_obs {c : Expr} {tc : Tr} {lc : Log} (hc : evalE n D σ ρ c = .ok (tc, lc)) :
    execOne (n + 2) D σ ρ (.expect c) = .ok ([], [], lc ++ obs D tc) := by
  have hc' := evalE_mono hc (m := n + 1) (by omega)
  have e : ∀ k, execS (k + 1) D σ ρ [] = .ok ([], [], []) := fun _ => by simp [execS]
  simp [Stmt.expect, execOne, hc', e]

end ForgeLang
