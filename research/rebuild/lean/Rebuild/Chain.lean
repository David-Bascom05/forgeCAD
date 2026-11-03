/-
A history built with some features reused equals the history built from scratch: the per-key
correctness of Build Systems à la Carte, by induction. The whole skip scheme stands on `Valid`:
each stored result is what its feature gives on the stored result before it.
-/
import Mathlib.Data.List.Basic

namespace Rebuild

variable {S : Type*}

/-- Every stage from scratch. -/
def scratch (s0 : S) : List (S → S) → S
  | [] => s0
  | f :: fs => scratch (f s0) fs

/-- A list of stored stages is valid when each is its feature applied to the one before. How the
stage was got (re-run, cached, patched) is nobody's business. -/
def Valid : S → List (S → S) → List S → Prop
  | _, [], [] => True
  | s, f :: fs, t :: ts => t = f s ∧ Valid t fs ts
  | _, _, _ => False

/-- The last stored stage, or the start if none. -/
def endOf (s0 : S) : List S → S
  | [] => s0
  | t :: ts => endOf t ts

/-- Valid stages end where from-scratch ends. The only hypothesis is that features are functions:
a feature's result may depend on nothing but its stage: no history of edits, no global state, no
thread count, no warm cache that changes the answer. -/
theorem valid_end (s0 : S) (fs : List (S → S)) (ts : List S) (h : Valid s0 fs ts) :
    endOf s0 ts = scratch s0 fs := by
  induction fs generalizing s0 ts with
  | nil =>
    cases ts with
    | nil => rfl
    | cons t ts => exact h.elim
  | cons f fs ih =>
    cases ts with
    | nil => exact h.elim
    | cons t ts =>
      obtain ⟨rfl, hv⟩ := h
      exact ih (f s0) ts hv

/-- Stages kept up to an equivalence (the same names, faces and points, whatever the order in the
arenas). What this needs is exactly that features respect it: `hf`. The kernel's features are not
functions of a body's arenas alone being equal, they must not care for their order; that is the one
assumption no proof here removes, and what the differential test in CI holds them to. -/
def ValidUpTo (r : S → S → Prop) : S → List (S → S) → List S → Prop
  | _, [], [] => True
  | s, f :: fs, t :: ts => r t (f s) ∧ ValidUpTo r t fs ts
  | _, _, _ => False

theorem valid_upto_end (r : S → S → Prop) (hrefl : ∀ a, r a a) (htrans : ∀ a b c, r a b → r b c → r a c)
    (s0 s0' : S) (fs : List (S → S)) (hf : ∀ f ∈ fs, ∀ a b, r a b → r (f a) (f b)) (ts : List S)
    (hs : r s0 s0') (h : ValidUpTo r s0 fs ts) :
    r (endOf s0 ts) (scratch s0' fs) := by
  induction fs generalizing s0 s0' ts with
  | nil =>
    cases ts with
    | nil => exact hs
    | cons t ts => exact h.elim
  | cons f fs ih =>
    cases ts with
    | nil => exact h.elim
    | cons t ts =>
      obtain ⟨ht, hv⟩ := h
      have hfa := hf f (by simp) s0 s0' hs
      exact ih t (f s0') (fun g hg => hf g (by simp [hg])) ts (htrans _ _ _ ht hfa) hv

/-- A cached result is valid when its feature depends only on a projection of the stage (the
firewall: the faces it touches) and that projection did not change. -/
theorem firewall {P : Type*} (f : S → S) (proj : S → P) (g : P → S → S)
    (hf : ∀ s, f s = g (proj s) s) {s s' : S} (hp : proj s = proj s') : f s' = g (proj s) s' := by
  rw [hf, hp]

end Rebuild
