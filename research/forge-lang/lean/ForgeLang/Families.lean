import ForgeLang.Builtins

/-!
# Built-in families of the `machining` documents: adding built-ins changes no accepted document

`Builtins.lean` is the part document's rule: one namespace, a binding first and then a built-in,
and a binding may not take a built-in's name (`ShadowsBuiltin`). The `machining` documents
(forge-cam-lang's `run`, forge-turn-lang's `run`) are read by the same parser but evaluated by
their own evaluators, whose names work differently, and this file models that:

* **closed namespaces**: a call's head is looked up among the built-ins only, never among the
  bindings (`call(name, fname, ..)` dispatches on `fname` by a `match`; `Cam::eval`'s `box`,
  `vise`, `Turn::eval`'s `chuck`); a read name is looked up among the bindings only (`env.get(n)`,
  else "unknown name"). So a binding may take a built-in's name (`drill = drill(op1, ..)`): there
  is no `ShadowsBuiltin` here and nothing to refuse.
* **two tables**: a statement `x = f(..)` runs `f` from the statement table (`Cam::call`,
  `Turn::call`); a call nested in an expression runs from the expression table (`box`, `vise`,
  `chuck`), and nothing else is callable there.
* **state**: a statement's built-in may change the document's state (setups, ops, programs) and
  may leave a change behind when it fails; an expression's built-in reads it.
* **every statement runs**: an error is recorded with the statement's line, the name is not
  bound, and the run goes on (`cam.errors.push(..)`); the document is accepted when no statement
  failed (`cam.ok`).

A later round adds match arms: tables `M` after the old ones (a `match` takes its first arm, as
`List.lookup` takes its first entry). Proven:

* `eval_ext_ok`, `stmt_ext_ok`: what evaluated before evaluates the same after.
* `eval_ext_avoid`, `run_ext_avoid`: a document that calls none of the new names (or only names
  the old tables already had) runs exactly as before, errors and state included.
* `stmt_new_refused`: a statement calling a name the old statement table lacks failed before.
* `run_ext_accepted`: so every document accepted before runs exactly as before after.
  `cam_conservative`, `turn_conservative` and `bending_conservative` are this for the CAM, the
  turning and the bending built-ins.
* `families_not_keywords`: none of the built-ins, nor the document headers `machining`,
  `assembly` and `bending`, is one of the lexer's keywords (`Impl.keywords`): each lexes as an identifier, so
  the documents are sentences of the one grammar the LL(1) proofs cover (`LL1/Documents.lean`).

Not modelled: what a built-in computes (the toolpaths, the checks: forge-cam's own proofs); named
arguments are a list of values in order (their keywords are not names that resolve).
-/

namespace ForgeLang
namespace Families

open Builtins

/-- An expression of a machining document, as its evaluator sees it. -/
inductive Ex where
  /-- a number or a string -/
  | lit : Nat → Ex
  /-- a name read -/
  | read : Name → Ex
  /-- `+ - * /` (and `.field`): one step on two values -/
  | op : Ex → Ex → Ex
  /-- `f(args..)` nested in an expression -/
  | call : Name → List Ex → Ex

/-- A statement: `x = e`, `x = f(args..)` with the statement's `param`s (evaluated first, seen by
its arguments), `f(args..)` binding nothing (the robot document's statements, from a table of
their own), or a statement the evaluator passes over (the header; in a CAM or turning document,
any bare expression). -/
inductive St where
  | bind : Name → Ex → St
  | bindCall : Name → List (Name × Ex) → Name → List Ex → St
  | exec : Name → List Ex → St
  | skip : St

inductive Err where
  | unknownName : Name → Err
  /-- a nested call of a name the expression table lacks ("is a statement's built-in here") -/
  | notHere : Name → Err
  /-- a statement calling a name the statement table lacks ("is not a machining built-in") -/
  | unknownCall : Name → Err
  /-- an argument the built-in does not take (`named` against its list: forge-cam-lang's
  argument lists) -/
  | unknownArg : Name → Name → Err
  /-- any error of a built-in or of arithmetic -/
  | other : String → Err

/-- What the evaluators do with literals and operators. -/
structure Prims (α : Type) where
  lit : Nat → α
  op : α → α → Except Err α

variable {S α : Type} (P : Prims α)

/-- An expression's built-ins: they read the state. -/
abbrev ETab (S α : Type) := List (Name × (S → List α → Except Err α))
/-- A statement's built-ins: they may change the state, even when they fail. -/
abbrev TTab (S α : Type) := List (Name × (S → List α → S × Except Err α))

abbrev Env (α : Type) := List (Name × α)

mutual
def eval (E : ETab S α) (s : S) (ρ : Env α) : Ex → Except Err α
  | .lit n => .ok (P.lit n)
  | .read x => match ρ.lookup x with
    | some v => .ok v
    | none => .error (.unknownName x)
  | .op a b => match eval E s ρ a with
    | .error e => .error e
    | .ok u => match eval E s ρ b with
      | .error e => .error e
      | .ok v => P.op u v
  | .call f as => match evalL E s ρ as with
    | .error e => .error e
    | .ok vs => match E.lookup f with
      | some g => g s vs
      | none => .error (.notHere f)

def evalL (E : ETab S α) (s : S) (ρ : Env α) : List Ex → Except Err (List α)
  | [] => .ok []
  | e :: es => match eval E s ρ e with
    | .error err => .error err
    | .ok v => match evalL E s ρ es with
      | .error err => .error err
      | .ok vs => .ok (v :: vs)
end

/-- A statement's `param`s, each evaluated in the document's environment, then put before it. -/
def locals (E : ETab S α) (s : S) (ρ : Env α) : List (Name × Ex) → Except Err (Env α)
  | [] => .ok ρ
  | (k, e) :: ps => match eval P E s ρ e with
    | .error err => .error err
    | .ok v => match locals E s ρ ps with
      | .error err => .error err
      | .ok ρ' => .ok ((k, v) :: ρ')

/-- One statement: the new state and environment, and whether it failed. -/
def stmt (E : ETab S α) (T X : TTab S α) (s : S) (ρ : Env α) : St → S × Env α × Except Err Unit
  | .bind x e => match eval P E s ρ e with
    | .ok v => (s, (x, v) :: ρ, .ok ())
    | .error err => (s, ρ, .error err)
  | .bindCall x ps f as => match locals P E s ρ ps with
    | .error err => (s, ρ, .error err)
    | .ok ρl => match evalL P E s ρl as with
      | .error err => (s, ρ, .error err)
      | .ok vs => match T.lookup f with
        | none => (s, ρ, .error (.unknownCall f))
        | some g => match g s vs with
          | (s', .ok v) => (s', (x, v) :: ρ, .ok ())
          | (s', .error err) => (s', ρ, .error err)
  | .exec f as => match evalL P E s ρ as with
    | .error err => (s, ρ, .error err)
    | .ok vs => match X.lookup f with
      | none => (s, ρ, .error (.unknownCall f))
      | some g => match g s vs with
        | (s', .ok _) => (s', ρ, .ok ())
        | (s', .error err) => (s', ρ, .error err)
  | .skip => (s, ρ, .ok ())

/-- Every statement, in order, each failure recorded and the run going on. -/
def run (E : ETab S α) (T X : TTab S α) : S → Env α → List St → S × Env α × List (Except Err Unit)
  | s, ρ, [] => (s, ρ, [])
  | s, ρ, st :: sts =>
    let (s', ρ', r) := stmt P E T X s ρ st
    let (s'', ρ'', rs) := run E T X s' ρ' sts
    (s'', ρ'', r :: rs)

/-- Accepted: no statement failed. -/
def Accepted (rs : List (Except Err Unit)) : Prop := ∀ r ∈ rs, r = .ok ()

/-! ## Tables extended: the first arm stands -/

theorem lookup_append_some {β : Type} (L M : List (Name × β)) (f : Name) (g : β)
    (h : L.lookup f = some g) : (L ++ M).lookup f = some g := by
  induction L with
  | nil => simp at h
  | cons p L ih =>
    obtain ⟨k, b⟩ := p
    by_cases hk : f = k
    · subst hk; simp [List.lookup] at h ⊢; exact h
    · have hk' : (f == k) = false := by simpa using hk
      simp only [List.lookup, hk', List.cons_append] at h ⊢
      exact ih h

theorem lookup_append_none {β : Type} (L M : List (Name × β)) (f : Name)
    (h : L.lookup f = none) : (L ++ M).lookup f = M.lookup f := by
  induction L with
  | nil => rfl
  | cons p L ih =>
    obtain ⟨k, b⟩ := p
    by_cases hk : f = k
    · subst hk; simp [List.lookup] at h
    · have hk' : (f == k) = false := by simpa using hk
      simp only [List.lookup, hk', List.cons_append] at h ⊢
      exact ih h

/-- A name the extension cannot change: the old table has it, or the new arms do not. -/
def Kept {β γ : Type} (L : List (Name × β)) (M : List (Name × γ)) (f : Name) : Prop :=
  L.lookup f ≠ none ∨ M.lookup f = none

theorem lookup_kept {β : Type} (L M : List (Name × β)) (f : Name) (h : Kept L M f) :
    (L ++ M).lookup f = L.lookup f := by
  cases hL : L.lookup f with
  | some g => exact lookup_append_some L M f g hL
  | none =>
    rw [lookup_append_none L M f hL]
    rcases h with h | h
    · exact absurd hL h
    · exact h

/-! ## What evaluated before evaluates the same -/

mutual
theorem eval_ext_ok (E M : ETab S α) (s : S) (ρ : Env α) :
    ∀ (e : Ex) (v : α), eval P E s ρ e = .ok v → eval P (E ++ M) s ρ e = .ok v
  | .lit _, v, h => by simpa [eval] using h
  | .read _, v, h => by simpa [eval] using h
  | .op a b, v, h => by
    unfold eval at h ⊢
    cases ha : eval P E s ρ a with
    | error e => rw [ha] at h; cases h
    | ok u =>
      rw [ha] at h
      rw [eval_ext_ok E M s ρ a u ha]
      cases hb : eval P E s ρ b with
      | error e => rw [hb] at h; cases h
      | ok w =>
        rw [hb] at h
        rw [eval_ext_ok E M s ρ b w hb]
        exact h
  | .call f as, v, h => by
    unfold eval at h ⊢
    cases ha : evalL P E s ρ as with
    | error e => rw [ha] at h; cases h
    | ok vs =>
      rw [ha] at h
      rw [evalL_ext_ok E M s ρ as vs ha]
      cases hf : E.lookup f with
      | none => rw [hf] at h; cases h
      | some g =>
        rw [hf] at h
        rw [lookup_append_some E M f g hf]
        exact h

theorem evalL_ext_ok (E M : ETab S α) (s : S) (ρ : Env α) :
    ∀ (es : List Ex) (vs : List α), evalL P E s ρ es = .ok vs → evalL P (E ++ M) s ρ es = .ok vs
  | [], vs, h => by simpa [evalL] using h
  | e :: es, vs, h => by
    unfold evalL at h ⊢
    cases he : eval P E s ρ e with
    | error x => rw [he] at h; cases h
    | ok v =>
      rw [he] at h
      rw [eval_ext_ok E M s ρ e v he]
      cases hs : evalL P E s ρ es with
      | error x => rw [hs] at h; cases h
      | ok ws =>
        rw [hs] at h
        rw [evalL_ext_ok E M s ρ es ws hs]
        exact h
end

theorem locals_ext_ok (E M : ETab S α) (s : S) (ρ : Env α) :
    ∀ (ps : List (Name × Ex)) (ρ' : Env α), locals P E s ρ ps = .ok ρ' →
      locals P (E ++ M) s ρ ps = .ok ρ'
  | [], ρ', h => by simpa [locals] using h
  | (k, e) :: ps, ρ', h => by
    unfold locals at h ⊢
    cases he : eval P E s ρ e with
    | error x => rw [he] at h; cases h
    | ok v =>
      rw [he] at h
      rw [eval_ext_ok P E M s ρ e v he]
      cases hs : locals P E s ρ ps with
      | error x => rw [hs] at h; cases h
      | ok ρ2 =>
        rw [hs] at h
        rw [locals_ext_ok E M s ρ ps ρ2 hs]
        exact h

/-- A statement that succeeded succeeds the same after, to the same state and environment. -/
theorem stmt_ext_ok (E ME : ETab S α) (T MT X MX : TTab S α) (s : S) (ρ : Env α) (st : St)
    (s' : S) (ρ' : Env α) (h : stmt P E T X s ρ st = (s', ρ', .ok ())) :
    stmt P (E ++ ME) (T ++ MT) (X ++ MX) s ρ st = (s', ρ', .ok ()) := by
  cases st with
  | skip => simpa [stmt] using h
  | bind x e =>
    simp only [stmt] at h ⊢
    cases he : eval P E s ρ e with
    | error x => simp [he] at h
    | ok v => rw [eval_ext_ok P E ME s ρ e v he]; simpa [he] using h
  | exec f as =>
    simp only [stmt] at h ⊢
    cases ha : evalL P E s ρ as with
    | error x => simp [ha] at h
    | ok vs =>
      rw [evalL_ext_ok P E ME s ρ as vs ha]
      simp only [ha] at h ⊢
      cases hf : X.lookup f with
      | none => simp [hf] at h
      | some g =>
        rw [lookup_append_some X MX f g hf]
        simpa [hf] using h
  | bindCall x ps f as =>
    simp only [stmt] at h ⊢
    cases hl : locals P E s ρ ps with
    | error x => simp [hl] at h
    | ok ρl =>
      rw [locals_ext_ok P E ME s ρ ps ρl hl]
      simp only [hl] at h ⊢
      cases ha : evalL P E s ρl as with
      | error x => simp [ha] at h
      | ok vs =>
        rw [evalL_ext_ok P E ME s ρl as vs ha]
        simp only [ha] at h ⊢
        cases hf : T.lookup f with
        | none => simp [hf] at h
        | some g =>
          rw [lookup_append_some T MT f g hf]
          simpa [hf] using h

/-- A statement calling a name the old statement table lacks was refused before. -/
theorem stmt_new_refused (E : ETab S α) (T X : TTab S α) (s : S) (ρ : Env α) (x : Name)
    (ps : List (Name × Ex)) (f : Name) (as : List Ex) (hf : T.lookup f = none) :
    ∃ s' ρ' err, stmt P E T X s ρ (.bindCall x ps f as) = (s', ρ', .error err) := by
  simp only [stmt]
  cases locals P E s ρ ps with
  | error e => exact ⟨_, _, _, rfl⟩
  | ok ρl =>
    simp only
    cases evalL P E s ρl as with
    | error e => exact ⟨_, _, _, rfl⟩
    | ok vs => simp only [hf]; exact ⟨_, _, _, rfl⟩

/-- A bare call of a name the old statement table lacks was refused before. -/
theorem exec_new_refused (E : ETab S α) (T X : TTab S α) (s : S) (ρ : Env α) (f : Name)
    (as : List Ex) (hf : X.lookup f = none) :
    ∃ s' ρ' err, stmt P E T X s ρ (.exec f as) = (s', ρ', .error err) := by
  simp only [stmt]
  cases evalL P E s ρ as with
  | error e => exact ⟨_, _, _, rfl⟩
  | ok vs => simp only [hf]; exact ⟨_, _, _, rfl⟩

/-- Every document accepted before runs exactly as before, to the same state and environment. -/
theorem run_ext_accepted (E ME : ETab S α) (T MT X MX : TTab S α) :
    ∀ (sts : List St) (s : S) (ρ : Env α) (s' : S) (ρ' : Env α) (rs : List (Except Err Unit)),
      run P E T X s ρ sts = (s', ρ', rs) → Accepted rs →
      run P (E ++ ME) (T ++ MT) (X ++ MX) s ρ sts = (s', ρ', rs)
  | [], s, ρ, s', ρ', rs, h, _ => by simpa [run] using h
  | st :: sts, s, ρ, s', ρ', rs, h, ha => by
    simp only [run] at h ⊢
    rcases hst : stmt P E T X s ρ st with ⟨s1, ρ1, r⟩
    rcases hrun : run P E T X s1 ρ1 sts with ⟨s2, ρ2, rs2⟩
    rw [hst] at h
    simp only [hrun] at h
    obtain ⟨rfl, rfl, rfl⟩ : s2 = s' ∧ ρ2 = ρ' ∧ r :: rs2 = rs := by simpa using h
    have hr : r = .ok () := ha r List.mem_cons_self
    subst hr
    rw [stmt_ext_ok P E ME T MT X MX s ρ st s1 ρ1 hst]
    simp only
    rw [run_ext_accepted E ME T MT X MX sts s1 ρ1 s2 ρ2 rs2 hrun
      (fun r hr => ha r (List.mem_cons_of_mem _ hr))]

/-! ## A document that calls no new name runs exactly as before -/

mutual
/-- Every call head in an expression is kept by the extension of the expression table. -/
def KeptE {β γ : Type} (E : List (Name × β)) (M : List (Name × γ)) : Ex → Prop
  | .lit _ => True
  | .read _ => True
  | .op a b => KeptE E M a ∧ KeptE E M b
  | .call f as => Kept E M f ∧ KeptL E M as

def KeptL {β γ : Type} (E : List (Name × β)) (M : List (Name × γ)) : List Ex → Prop
  | [] => True
  | e :: es => KeptE E M e ∧ KeptL E M es
end

mutual
theorem eval_ext_avoid (E M : ETab S α) (s : S) (ρ : Env α) :
    ∀ (e : Ex), KeptE E M e → eval P (E ++ M) s ρ e = eval P E s ρ e
  | .lit _, _ => rfl
  | .read _, _ => rfl
  | .op a b, ⟨ha, hb⟩ => by
    unfold eval
    rw [eval_ext_avoid E M s ρ a ha, eval_ext_avoid E M s ρ b hb]
  | .call f as, ⟨hf, has⟩ => by
    unfold eval
    rw [evalL_ext_avoid E M s ρ as has, lookup_kept E M f hf]

theorem evalL_ext_avoid (E M : ETab S α) (s : S) (ρ : Env α) :
    ∀ (es : List Ex), KeptL E M es → evalL P (E ++ M) s ρ es = evalL P E s ρ es
  | [], _ => rfl
  | e :: es, ⟨he, hes⟩ => by
    unfold evalL
    rw [eval_ext_avoid E M s ρ e he, evalL_ext_avoid E M s ρ es hes]
end

theorem locals_ext_avoid (E M : ETab S α) (s : S) (ρ : Env α) :
    ∀ (ps : List (Name × Ex)), (∀ p ∈ ps, KeptE E M p.2) →
      locals P (E ++ M) s ρ ps = locals P E s ρ ps
  | [], _ => rfl
  | (k, e) :: ps, h => by
    unfold locals
    rw [eval_ext_avoid P E M s ρ e (h (k, e) List.mem_cons_self),
      locals_ext_avoid E M s ρ ps (fun p hp => h p (List.mem_cons_of_mem _ hp))]

/-- A statement whose every call (nested or its own) is kept by the extension. -/
def KeptSt {β γ δ ε ζ η : Type} (E : List (Name × β)) (ME : List (Name × γ)) (T : List (Name × δ))
    (MT : List (Name × ε)) (X : List (Name × ζ)) (MX : List (Name × η)) : St → Prop
  | .bind _ e => KeptE E ME e
  | .bindCall _ ps f as => (∀ p ∈ ps, KeptE E ME p.2) ∧ KeptL E ME as ∧ Kept T MT f
  | .exec f as => KeptL E ME as ∧ Kept X MX f
  | .skip => True

theorem stmt_ext_avoid (E ME : ETab S α) (T MT X MX : TTab S α) (s : S) (ρ : Env α) (st : St)
    (h : KeptSt E ME T MT X MX st) : stmt P (E ++ ME) (T ++ MT) (X ++ MX) s ρ st = stmt P E T X s ρ st := by
  cases st with
  | skip => rfl
  | bind x e => simp only [stmt]; rw [eval_ext_avoid P E ME s ρ e h]
  | exec f as =>
    obtain ⟨ha, hf⟩ := h
    simp only [stmt]
    rw [evalL_ext_avoid P E ME s ρ as ha, lookup_kept X MX f hf]
  | bindCall x ps f as =>
    obtain ⟨hp, ha, hf⟩ := h
    simp only [stmt]
    rw [locals_ext_avoid P E ME s ρ ps hp]
    cases locals P E s ρ ps with
    | error _ => rfl
    | ok ρl =>
      simp only
      rw [evalL_ext_avoid P E ME s ρl as ha, lookup_kept T MT f hf]

/-- A document all of whose calls are kept runs exactly as before: the same failures, state and
environment, whatever it was before. -/
theorem run_ext_avoid (E ME : ETab S α) (T MT X MX : TTab S α) :
    ∀ (sts : List St) (s : S) (ρ : Env α), (∀ st ∈ sts, KeptSt E ME T MT X MX st) →
      run P (E ++ ME) (T ++ MT) (X ++ MX) s ρ sts = run P E T X s ρ sts
  | [], _, _, _ => rfl
  | st :: sts, s, ρ, h => by
    unfold run
    rw [stmt_ext_avoid P E ME T MT X MX s ρ st (h st List.mem_cons_self)]
    obtain ⟨s1, ρ1, r⟩ := stmt P E T X s ρ st
    simp only
    rw [run_ext_avoid E ME T MT X MX sts s1 ρ1 (fun t ht => h t (List.mem_cons_of_mem _ ht))]

/-! ## The families -/

/-- forge-cam-lang: `Cam::call`'s arms (statement built-ins). -/
def camStmt : List Name :=
  ["part", "machine", "tool", "setup", "facing", "contour", "pocket", "clear", "adaptive", "drill",
    "engrave", "slot", "thread_mill", "edge_break", "bore", "soft_jaw", "parallel", "waterline",
    "zlevel", "pencil", "scallop", "rest", "steep_shallow", "spiral", "radial", "ramping", "flat",
    "program"]

/-- forge-cam-lang: `Cam::eval`'s built-ins (nested in an expression). -/
def camExpr : List Name := ["box", "vise", "cylinder", "tube", "solid"]

/-- forge-turn-lang: `Turn::call`'s arms. -/
def turnStmt : List Name :=
  ["part", "lathe_tool", "lathe_setup", "turn_face", "turn_rough", "bore_rough", "turn_finish",
    "turn_groove", "turn_thread", "part_off", "turn_drill", "lathe_program"]

/-- forge-turn-lang: `Turn::eval`'s built-ins. -/
def turnExpr : List Name := ["chuck"]

/-- forge-bend-lang: `Reader::call`'s arms (a bending document's statement built-ins; no
built-in nests in an expression: `eval` has no call, so its table is empty). -/
def bendStmt : List Name :=
  ["part", "press", "tooling", "station", "bend", "setup_sheet", "program"]

/-- The document headers that are names, not keywords (`drawing` and `harness` are keywords). -/
def headers : List Name := ["machining", "assembly", "bending"]

/-- A table of a family: each of its names with what it does. -/
def tab {β : Type} (names : List Name) (impl : Name → β) : List (Name × β) :=
  names.map fun n => (n, impl n)

/-- The CAM built-ins, whatever each computes, extended by a later round's arms `ME`, `MT` (new
names, or old ones, which the old arms keep): every document accepted before means what it
meant. -/
theorem cam_conservative (ie : Name → S → List α → Except Err α)
    (is : Name → S → List α → S × Except Err α) (ME : ETab S α) (MT : TTab S α)
    (sts : List St) (s s' : S) (ρ ρ' : Env α) (rs : List (Except Err Unit))
    (h : run P (tab camExpr ie) (tab camStmt is) [] s ρ sts = (s', ρ', rs)) (ha : Accepted rs) :
    run P (tab camExpr ie ++ ME) (tab camStmt is ++ MT) [] s ρ sts = (s', ρ', rs) := by
  have := run_ext_accepted P _ ME _ MT [] [] sts s ρ s' ρ' rs h ha
  simpa using this

/-- The turning built-ins, likewise. -/
theorem turn_conservative (ie : Name → S → List α → Except Err α)
    (is : Name → S → List α → S × Except Err α) (ME : ETab S α) (MT : TTab S α)
    (sts : List St) (s s' : S) (ρ ρ' : Env α) (rs : List (Except Err Unit))
    (h : run P (tab turnExpr ie) (tab turnStmt is) [] s ρ sts = (s', ρ', rs)) (ha : Accepted rs) :
    run P (tab turnExpr ie ++ ME) (tab turnStmt is ++ MT) [] s ρ sts = (s', ρ', rs) := by
  have := run_ext_accepted P _ ME _ MT [] [] sts s ρ s' ρ' rs h ha
  simpa using this

/-- The bending built-ins (forge-bend-lang), extended by a later round's: every bending document
accepted before means what it meant. The expression table is empty: a nested call is refused
(`eval`: "a value this document does not take"). -/
theorem bending_conservative (is : Name → S → List α → S × Except Err α) (ME : ETab S α)
    (MT : TTab S α) (sts : List St) (s s' : S) (ρ ρ' : Env α) (rs : List (Except Err Unit))
    (h : run P [] (tab bendStmt is) [] s ρ sts = (s', ρ', rs)) (ha : Accepted rs) :
    run P ME (tab bendStmt is ++ MT) [] s ρ sts = (s', ρ', rs) := by
  have := run_ext_accepted P [] ME _ MT [] [] sts s ρ s' ρ' rs h ha
  simpa using this

/-- forge-lang's drawing document (`drawing/doc.rs`): `x = f(..)` and a nested `f(..)` both call
from `FUNCTIONS`, nothing else is callable, and a read is a binding's (closed). -/
def drawingFns : List Name :=
  ["part", "assembly", "sheet", "base_view", "projected", "section", "detail", "flat_view", "dim", "angle", "radius", "diameter", "hole_note", "ordinate", "baseline", "chain", "center_mark", "center_marks", "centerlines", "centerline", "leader", "note", "balloon", "auto_balloons", "fcf", "datum", "datum_target", "surface", "weld", "edge", "parts_list", "e", "f"]

/-- The drawing built-ins, extended by a later round's: every drawing accepted before means what
it meant (one table for both a statement's call and a nested one; no bare call statements). -/
theorem drawing_conservative (ie : Name → S → List α → Except Err α)
    (is : Name → S → List α → S × Except Err α) (ME : ETab S α) (MT : TTab S α)
    (sts : List St) (s s' : S) (ρ ρ' : Env α) (rs : List (Except Err Unit))
    (h : run P (tab drawingFns ie) (tab drawingFns is) [] s ρ sts = (s', ρ', rs)) (ha : Accepted rs) :
    run P (tab drawingFns ie ++ ME) (tab drawingFns is ++ MT) [] s ρ sts = (s', ρ', rs) := by
  have := run_ext_accepted P _ ME _ MT [] [] sts s ρ s' ρ' rs h ha
  simpa using this

/-- No built-in of the machining families, and no header word, is a keyword: each lexes as an
identifier, so the grammar and the proven parser are unchanged by them. -/
theorem families_not_keywords :
    ∀ n ∈ camStmt ++ camExpr ++ turnStmt ++ turnExpr ++ bendStmt ++ headers, codes n ∉ LL1.Impl.keywords := by
  decide

end Families
end ForgeLang
