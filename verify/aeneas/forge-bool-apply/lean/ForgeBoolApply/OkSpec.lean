/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas

/-!
# `okspec`: what a translated function's `ok` value satisfies

`okspec m p`: if `m` returns `ok v`, then `p v` (a panic or divergence satisfies it). D8's rows
describe the body `apply` writes when it returns; whether it can panic is a separate question
(no D8 row depends on it). Aeneas's `dspec` also excludes failure, so this is the same partial
WP (`DWLP`, its coinductive weakest liberal precondition) over the Rust effects, with a failure
counted as satisfying every postcondition.

- `okspec_ok`, `okspec_fail`, `okspec_div`, `okspec_bind`, `okspec_mono`;
- `okspec_of_eq`: from `m = ok v` and `okspec m p`, `p v`;
- `okspec_loop`: an invariant of a translated loop's state holds at its `ok` result (fixpoint
  induction on Aeneas's `loop`, which is a `partial_fixpoint`: the predicate is admissible).
-/

namespace ForgeBoolApply

open Aeneas Aeneas.Std Result Aeneas.Data.Coinductive Lean.Order

unseal Result in
/-- The Rust effects' WP, a failure satisfying anything. -/
@[reducible]
def okWP : EffectWP RustEffect where
  State := Unit
  wp effect _ _ :=
    match effect with
    | .fail _ => True

instance : EffectWP.Monotonic okWP where
  wp_monotonic _ _ h := h

instance : EffectWP.Conjunctive okWP where
  wp_conj := by
    intro e _ _ _ _
    cases e
    trivial

unseal Result in
/-- If `m` returns `ok v`, then `p v`. -/
def okspec {α : Type _} (m : Result α) (p : α → Prop) : Prop :=
  DWLP okWP m (fun v _ => p v) ()

unseal Result in
@[simp] theorem okspec_ok {α : Type _} (x : α) (p : α → Prop) : okspec (ok x) p ↔ p x :=
  DWLP.ret_iff

unseal Result in
@[simp] theorem okspec_fail {α : Type _} (e : Error) (p : α → Prop) : okspec (fail e) p := by
  rw [Result.fail_eq_vis]
  exact DWLP.vis trivial

unseal Result in
@[simp] theorem okspec_div {α : Type _} (p : α → Prop) : okspec (div : Result α) p :=
  DWLP.div

unseal Result in
theorem okspec_mono {α : Type _} {m : Result α} {p q : α → Prop} (h : okspec m p)
    (hpq : ∀ x, p x → q x) : okspec m q :=
  DWLP.mono h fun v _ => hpq v

unseal Result in
theorem okspec_bind {α β : Type _} {m : Result α} {k : α → Result β} {p : α → Prop}
    {q : β → Prop} (hm : okspec m p) (hk : ∀ x, p x → okspec (k x) q) :
    okspec (Std.bind m k) q :=
  DWLP.bind (k := k) hm fun v _ hv => hk v hv

/-- The `ok` value of `m` satisfies `p`. -/
theorem okspec_of_eq {α : Type _} {m : Result α} {p : α → Prop} {v : α} (h : okspec m p)
    (hm : m = ok v) : p v := by
  rw [hm, okspec_ok] at h; exact h

unseal Result in
theorem okspec_admissible {α : Type _} (p : α → Prop) :
    admissible (fun m : Result α => okspec m p) :=
  DWLP.admissible okWP _ ()

/-- **Loop invariant.** If the body keeps `inv` on `cont` and gives `post` on `done`, the loop's
`ok` result satisfies `post`. -/
theorem okspec_loop {α β : Type _} (body : α → Result (ControlFlow α β)) (inv : α → Prop)
    (post : β → Prop)
    (hbody : ∀ x, inv x → okspec (body x) fun r =>
      match r with
      | .cont x' => inv x'
      | .done y => post y)
    (x : α) (hx : inv x) : okspec (loop body x) post := by
  have key : ∀ x, inv x → okspec (loop body x) post := by
    refine loop.fixpoint_induct body (fun f => ∀ x, inv x → okspec (f x) post) ?_ ?_
    · apply admissible_pi
      intro x
      apply admissible_pi
      intro _
      exact admissible_apply (fun _ m => okspec m post) x (okspec_admissible post)
    · intro f hf x hx
      refine okspec_bind (hbody x hx) ?_
      intro r hr
      cases r with
      | cont x' => exact hf x' hr
      | done y => exact (okspec_ok y post).mpr hr
  exact key x hx

end ForgeBoolApply

namespace ForgeBoolApply

open Aeneas Aeneas.Std Result Aeneas.Data.Coinductive

unseal Result in
theorem okspec_and {α : Type _} {m : Result α} {p q : α → Prop} (hp : okspec m p)
    (hq : okspec m q) : okspec m fun x => p x ∧ q x :=
  DWLP.and_iff.mpr ⟨hp, hq⟩

unseal Result in
/-- **A step's own value**: if `m` returns `ok x`, then `m = ok x`. -/
theorem okspec_self {α : Type _} (m : Result α) : okspec m fun x => m = ok x := by
  cases m using ITree.cases with
  | ret v => exact (okspec_ok v _).mpr rfl
  | div => exact DWLP.div
  | vis e k => cases e; exact DWLP.vis trivial

theorem okspec_true {α : Type _} (m : Result α) : okspec m fun _ => True :=
  okspec_mono (okspec_self m) fun _ _ => trivial

/-- Bind, carrying the step's own equation. -/
theorem okspec_let {α β : Type _} {m : Result α} {k : α → Result β} {q : β → Prop}
    (hk : ∀ x, m = ok x → okspec (k x) q) : okspec (Std.bind m k) q :=
  okspec_bind (okspec_self m) hk

/-- Bind of a pair, carrying the step's own equation with the pair's components. -/
theorem okspec_let2 {α β γ : Type _} {m : Result (α × β)} {k : α × β → Result γ} {q : γ → Prop}
    (hk : ∀ a b, m = ok (a, b) → okspec (k (a, b)) q) : okspec (Std.bind m k) q :=
  okspec_let fun ⟨a, b⟩ h => hk a b h

end ForgeBoolApply
