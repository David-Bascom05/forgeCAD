/-!
# Undo and Redo (`interface/forge-lang/src/undo.rs`)

The log beside a file is a list of events: an edit (the text before and after), an undo, a redo.
Reading it gives two stacks of edits, the last of each its top: `undo` moves the top of `undos`
to `redos`, `redo` moves it back, a new edit pushes on `undos` and clears `redos`. (undo.rs keeps
the edits in a vector and the stacks as indices into it; an index is only ever read as its
edit, so a stack of edits is the same thing.) A step is refused, with nothing written, unless
the file holds the text the step starts from.

Proven, for texts of any type with decidable equality:

* `undo_write`: right after an edit that changed the text, Undo gives back the text before it,
  byte for byte. So (P1: `run` is a function of the text) the old model, ids and handles.
* `redo_undo`, `undo_redo`: Redo after Undo, and Undo after Redo, give back the text, and the
  stacks as they were. Nothing an Undo replaces is lost: it is the next Redo.
* `step_refused`: a hand edit since the last logged text refuses the step, and the file is not
  written.
* `history_keep`: `keep` (an app's undo depth saved with the file) read back by `history` is the
  stacks it was given, whatever they are.

| Assumption | Counterexample without it |
|---|---|
| A step checks that the file holds the text the log ends on | `unchecked_loses_edit`: without the check, Undo after a hand edit writes the text before the logged edit, and the hand edit is in neither the file nor the log |
-/

namespace ForgeLang.Undo

variable {α : Type}

inductive Ev (α : Type) where
  | edit (before after : α)
  | undo
  | redo

structure Log (α : Type) where
  undos : List (α × α) := []
  redos : List (α × α) := []

def Log.push : Log α → Ev α → Log α
  | L, .edit b a => { undos := L.undos ++ [(b, a)], redos := [] }
  | L, .undo => match L.undos.getLast? with
    | some e => { undos := L.undos.dropLast, redos := L.redos ++ [e] }
    | none => L
  | L, .redo => match L.redos.getLast? with
    | some e => { undos := L.undos ++ [e], redos := L.redos.dropLast }
    | none => L

def read (evs : List (Ev α)) : Log α := evs.foldl Log.push {}

/-- `write`: the log with the edit, if it changed anything (the file then holds `after`). -/
def write [DecidableEq α] (evs : List (Ev α)) (before after : α) : List (Ev α) :=
  if before = after then evs else evs ++ [.edit before after]

inductive Step where
  | undo
  | redo

inductive Refusal where
  /-- `nothing to undo|redo` -/
  | nothing
  /-- `changed since its last logged edit` -/
  | changed
  deriving DecidableEq

/-- `step`: the file's new text and the log, or a refusal (nothing written). -/
def step [DecidableEq α] (evs : List (Ev α)) (now : α) : Step → Except Refusal (α × List (Ev α))
  | .undo => match (read evs).undos.getLast? with
    | none => .error .nothing
    | some (b, a) => if now = a then .ok (b, evs ++ [.undo]) else .error .changed
  | .redo => match (read evs).redos.getLast? with
    | none => .error .nothing
    | some (b, a) => if now = b then .ok (a, evs ++ [.redo]) else .error .changed

theorem read_append (evs : List (Ev α)) (e : Ev α) : read (evs ++ [e]) = (read evs).push e := by
  simp [read, List.foldl_append]

theorem dropLast_last {β : Type} {l : List β} {x : β} (h : l.getLast? = some x) : l.dropLast ++ [x] = l := by
  rcases List.eq_nil_or_concat l with rfl | ⟨l', y, rfl⟩
  · simp at h
  · simp at h; subst h; simp

theorem undo_write [DecidableEq α] (evs : List (Ev α)) {b a : α} (h : b ≠ a) :
    step (write evs b a) a .undo = .ok (b, write evs b a ++ [.undo]) := by
  simp [write, h, step, read_append, Log.push]

theorem redo_undo [DecidableEq α] {evs evs' : List (Ev α)} {now t : α}
    (h : step evs now .undo = .ok (t, evs')) :
    ∃ evs'', step evs' t .redo = .ok (now, evs'') ∧ read evs'' = read evs := by
  unfold step at h
  cases he : (read evs).undos.getLast? with
  | none => simp [he] at h
  | some e =>
    obtain ⟨b, a⟩ := e
    simp only [he] at h
    by_cases hn : now = a
    · simp only [hn, ↓reduceIte, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      subst hn
      refine ⟨evs ++ [.undo] ++ [.redo], by simp [step, read_append, Log.push, he], ?_⟩
      have hd := dropLast_last he
      cases hr : read evs
      rw [read_append, read_append, hr]
      simp_all [Log.push]
    · simp [hn] at h

theorem undo_redo [DecidableEq α] {evs evs' : List (Ev α)} {now t : α}
    (h : step evs now .redo = .ok (t, evs')) :
    ∃ evs'', step evs' t .undo = .ok (now, evs'') ∧ read evs'' = read evs := by
  unfold step at h
  cases he : (read evs).redos.getLast? with
  | none => simp [he] at h
  | some e =>
    obtain ⟨b, a⟩ := e
    simp only [he] at h
    by_cases hn : now = b
    · simp only [hn, ↓reduceIte, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      subst hn
      refine ⟨evs ++ [.redo] ++ [.undo], by simp [step, read_append, Log.push, he], ?_⟩
      have hd := dropLast_last he
      cases hr : read evs
      rw [read_append, read_append, hr]
      simp_all [Log.push]
    · simp [hn] at h

/-- A hand edit since the last logged text: refused (and `step` writes only on success). -/
theorem step_refused [DecidableEq α] {evs : List (Ev α)} {now b a : α} :
    ((read evs).undos.getLast? = some (b, a) → now ≠ a → step evs now .undo = .error .changed) ∧
    ((read evs).redos.getLast? = some (b, a) → now ≠ b → step evs now .redo = .error .changed) := by
  constructor <;> intro he hn <;> simp [step, he, hn]

/-! ## `history` and `keep` -/

/-- The stacks as texts (undo's targets oldest first, redo's), when the log ends on `now`. -/
def history [DecidableEq α] (evs : List (Ev α)) (now : α) : Option (List α × List α) :=
  let L := read evs
  let at_ := match L.undos.getLast?, L.redos.getLast? with
    | some e, _ => some e.2
    | none, some e => some e.1
    | none, none => none
  match at_ with
  | some t => if t = now then some (L.undos.map Prod.fst, L.redos.map Prod.snd) else none
  | none => none

def windows : List α → List (α × α)
  | a :: b :: t => (a, b) :: windows (b :: t)
  | _ => []

/-- `keep`: the edits from the first text to the last, then one undo per redo step. -/
def keep (undos : List α) (now : α) (redos : List α) : List (Ev α) :=
  (windows (undos ++ now :: redos.reverse)).map (fun e => Ev.edit e.1 e.2) ++ redos.map fun _ => .undo

theorem windows_append : ∀ (u : List α) (now : α) (t : List α),
    windows (u ++ now :: t) = windows (u ++ [now]) ++ windows (now :: t)
  | [], _, _ => by simp [windows]
  | [x], now, t => by simp [windows]
  | x :: y :: u, now, t => by
    have := windows_append (y :: u) now t
    simp only [List.cons_append] at this ⊢
    simp [windows, this]

theorem windows_fst : ∀ (u : List α) (now : α), (windows (u ++ [now])).map Prod.fst = u
  | [], _ => rfl
  | [x], _ => rfl
  | x :: y :: u, now => by
    have := windows_fst (y :: u) now
    simp only [List.cons_append] at this ⊢
    simp [windows, this]

theorem windows_last (u : List α) (now : α) (h : u ≠ []) :
    (windows (u ++ [now])).getLast?.map Prod.snd = some now := by
  rcases List.eq_nil_or_concat u with rfl | ⟨u', x, rfl⟩
  · exact absurd rfl h
  · rw [List.concat_eq_append, List.append_assoc, List.singleton_append, windows_append u' x [now]]
    simp [windows]

theorem windows_snd : ∀ (now : α) (t : List α), (windows (now :: t)).map Prod.snd = t
  | _, [] => rfl
  | now, y :: t => by simp [windows, windows_snd y t]

theorem windows_len : ∀ (now : α) (t : List α), (windows (now :: t)).length = t.length
  | _, [] => rfl
  | now, y :: t => by simp [windows, windows_len y t]

theorem read_edits (L : Log α) (es : List (α × α)) (h : L.redos = []) :
    (es.map fun e => Ev.edit e.1 e.2).foldl Log.push L = { undos := L.undos ++ es, redos := [] } := by
  induction es generalizing L with
  | nil => cases L; simp_all
  | cons e es ih =>
    simp only [List.map_cons, List.foldl_cons]
    rw [ih _ rfl]
    simp [Log.push]

theorem read_undos : ∀ (k : List (α × α)) (A B R : List (α × α)), B.length = k.length →
    (k.map fun _ => (Ev.undo : Ev α)).foldl Log.push { undos := A ++ B, redos := R } =
      { undos := A, redos := R ++ B.reverse }
  | [], A, B, R, hk => by cases B <;> simp_all
  | _ :: k, A, B, R, hk => by
    rcases List.eq_nil_or_concat B with rfl | ⟨B', b, rfl⟩
    · simp at hk
    · simp only [List.map_cons, List.foldl_cons, Log.push, List.concat_eq_append, ← List.append_assoc,
        List.getLast?_concat, List.dropLast_concat]
      rw [read_undos k A B' (R ++ [b]) (by simp at hk; omega)]
      simp

/-- What `keep` writes, `history` reads back (with no steps, undo.rs writes no log at all). -/
theorem history_keep [DecidableEq α] (u : List α) (now : α) (r : List α) (h : u ≠ [] ∨ r ≠ []) :
    history (keep u now r) now = some (u, r) := by
  have hr : read (keep u now r) =
      { undos := windows (u ++ [now]), redos := (windows (now :: r.reverse)).reverse } := by
    unfold read keep
    rw [List.foldl_append, read_edits _ _ rfl, windows_append]
    have hl : r.length = (windows (now :: r.reverse)).length := by simp [windows_len]
    have : (r.map fun _ => (Ev.undo : Ev α)) =
        ((windows (now :: r.reverse)).map fun _ => (Ev.undo : Ev α)) := by
      apply List.ext_getElem <;> simp [hl]
    rw [this]
    simpa using read_undos (windows (now :: r.reverse)) (windows (u ++ [now]))
      (windows (now :: r.reverse)) [] rfl
  have hs : ((windows (now :: r.reverse)).reverse.map Prod.snd) = r := by
    rw [List.map_reverse, windows_snd, List.reverse_reverse]
  have hat : (match (windows (u ++ [now])).getLast?, (windows (now :: r.reverse)).reverse.getLast? with
      | some e, _ => some e.2
      | none, some e => some e.1
      | none, none => none) = some now := by
    by_cases hu : u = []
    · subst hu
      have hr' : r.reverse ≠ [] := by simpa using h
      obtain ⟨y, t, ht⟩ : ∃ y t, r.reverse = y :: t := by
        cases hh : r.reverse with
        | nil => exact absurd hh hr'
        | cons y t => exact ⟨y, t, rfl⟩
      simp [windows, ht, List.getLast?_reverse]
    · have hl := windows_last u now hu
      cases hw : (windows (u ++ [now])).getLast? with
      | none => simp [hw] at hl
      | some e => simp only [hw, Option.map_some, Option.some.injEq] at hl; simp [hl]
  unfold history
  rw [hr]
  simp only at hat ⊢
  rw [hat]
  simp [windows_fst, hs]

/-! ## The check is needed -/

/-- Undo without the check that the file holds the logged text. -/
def stepUnchecked (evs : List (Ev String)) : Option (String × List (Ev String)) :=
  (read evs).undos.getLast?.map fun (b, _) => (b, evs ++ [.undo])

/-- An edit `a` → `b`, then a hand edit to `hand`: the unchecked Undo writes `a`, and `hand` is in
neither the file nor the log, so no Redo brings it back. The checked one refuses. -/
theorem unchecked_loses_edit :
    stepUnchecked (write [] "a" "b") = some ("a", write [] "a" "b" ++ [.undo]) ∧
    (∀ e ∈ write [] "a" "b" ++ [.undo], ∀ x y, e = .edit x y → x ≠ "hand" ∧ y ≠ "hand") ∧
    step (write [] "a" "b") "hand" .undo = .error .changed := by
  refine ⟨by simp [stepUnchecked, write, read, Log.push], ?_, by simp [step, write, read, Log.push]⟩
  intro e he x y hxy
  simp [write] at he
  rcases he with rfl | rfl
  · cases hxy; decide
  · cases hxy

end ForgeLang.Undo
