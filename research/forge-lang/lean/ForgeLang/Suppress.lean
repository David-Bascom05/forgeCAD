import ForgeLang.Complete

/-!
# Suppress and Unsuppress (`interface/forge-lang/src/edit_ops.rs`)

`Suppress {id}` takes out the statement making `id` and every later statement that reads what the
ones taken out bind (`suppFrom`: the closure, in one pass, as `apply_suppress` computes it), each
kept as a comment `#~ID LINE`. A statement that shares its line with one kept goes on a line of
its own, the break put in marked in its tag (`#~ID<`: the break before, `#~ID>`: the one after).
`Unsuppress {id}` strips the tags and takes those breaks out.

* `suppress_complete`: if the document evaluated and no function or component body reads a name
  the closure binds, the suppressed document evaluates, and its model and log are the old ones
  with exactly the taken-out statements' fragments removed, every other fragment (ids, traces)
  unchanged. Nothing outside the closure needs to be checked: the closure is closed under reads.
* `suppress_live`: on the text, the parser reads exactly the pieces of the statements kept, in
  order, however statements share lines; `suppress_text_closure`: those are `suppressAt`'s
  statements. No assumption on lines.
* `unsuppress_suppress`: on the text, Unsuppress after Suppress is the identity, byte for byte,
  every line (`readTag_render`: each tagged line reads back as its marks and its text, given
  that no kept line already starts with the tag, what `apply_suppress` refuses).

| Assumption | Counterexample without it |
|---|---|
| No function or component body reads a taken-out name | Not needed for soundness: the run is then a typed error (`unbound`), refused |

Before this, Suppress commented whole lines, so with `a = ..; b = ..` on one line Suppress `b`
took out `a` too (found in fg; the Lean counterexample was `same_line_drops`, under the
assumption of one statement per line, now gone).
-/

namespace ForgeLang

/-- Which statements the closure takes out: those reading a name in `names`, each adding what it
binds. -/
def suppFrom (names : List Name) : List Stmt → List Bool
  | [] => []
  | s :: ss => if (fvS s).any (· ∈ names) then true :: suppFrom (names ++ binders [s]) ss
    else false :: suppFrom names ss

/-- The entries whose flag is `false`. -/
def keepBy {α : Type} : List Bool → List α → List α
  | b :: bs, x :: xs => if b then keepBy bs xs else x :: keepBy bs xs
  | _, xs => xs

/-- `Suppress {id}` on `xs ++ s :: ys` with `s` making `id`. -/
def suppressAt (s : Stmt) (id : Name) (xs ys : List Stmt) : List Stmt :=
  xs ++ keepBy (suppFrom (binders [s] ++ [id]) ys) ys

theorem binders_cons (s : Stmt) (ss : List Stmt) : binders (s :: ss) = binders [s] ++ binders ss := by
  simp [binders, List.filterMap_cons]; cases s.binder <;> simp

theorem keys_binders {n D σ ρ s Δ m l} (h : execOne n D σ ρ s = .ok (Δ, m, l)) :
    ∀ x ∈ keys Δ, x ∈ binders [s] := by
  intro x hx
  have := execOne_keys h x hx
  simp [binders, this]

/-- The closure run: kept statements give their own fragments, in an environment that agrees on
every name the closure did not take. -/
theorem supp_run : ∀ (ys : List Stmt) (names : List Name) {n D σ ρ ρ' Δ m l},
    (∀ x ∈ D.reads, x ∉ names ∧ x ∉ binders ys) →
    (∀ x, x ∉ names → ρ.lookup x = ρ'.lookup x) →
    execS n D σ ρ ys = .ok (Δ, m, l) →
    ∃ parts : List (Model × Log), parts.length = ys.length ∧
      m = (parts.map Prod.fst).flatten ∧ l = (parts.map Prod.snd).flatten ∧
      ∃ N Δ', execS N D σ ρ' (keepBy (suppFrom names ys) ys) =
        .ok (Δ', ((keepBy (suppFrom names ys) parts).map Prod.fst).flatten,
          ((keepBy (suppFrom names ys) parts).map Prod.snd).flatten)
  | [], names, n, D, σ, ρ, ρ', Δ, m, l, _, _, h => by
    cases n with
    | zero => simp [execS] at h
    | succ n =>
      simp only [execS, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨-, rfl, rfl⟩ := h
      exact ⟨[], rfl, rfl, rfl, 1, [], by simp [suppFrom, keepBy, execS]⟩
  | s :: ss, names, n, D, σ, ρ, ρ', Δ, m, l, hD, hag, h => by
    obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_cons h
    simp only [Prod.mk.injEq] at e
    obtain ⟨-, rfl, rfl⟩ := e
    have hD' : ∀ x ∈ D.reads, x ∉ binders ss := fun x hx hb =>
      (hD x hx).2 (by rw [binders_cons]; exact List.mem_append_right _ hb)
    by_cases hf : (fvS s).any (· ∈ names) = true
    · -- taken out: what it binds joins the names
      have hag' : ∀ x, x ∉ names ++ binders [s] → (Δ1 ++ ρ).lookup x = ρ'.lookup x := by
        intro x hx
        simp only [List.mem_append, not_or] at hx
        rw [lookup_append, lookup_none (fun hk => hx.2 (keys_binders h1 x hk))]
        exact hag x hx.1
      have hD2 : ∀ x ∈ D.reads, x ∉ names ++ binders [s] ∧ x ∉ binders ss := by
        intro x hx
        refine ⟨?_, hD' x hx⟩
        simp only [List.mem_append, not_or]
        exact ⟨(hD x hx).1, fun hb => (hD x hx).2 (by rw [binders_cons]; exact List.mem_append_left _ hb)⟩
      obtain ⟨parts, hl, hm, hl2, N, Δ', hN⟩ := supp_run ss (names ++ binders [s]) hD2 hag' h2
      refine ⟨(m1, l1) :: parts, by simp [hl], by simp [hm], by simp [hl2], N, Δ', ?_⟩
      simpa [suppFrom, hf, keepBy] using hN
    · -- kept: it reads no taken-out name, so it runs as before
      have hf' : ∀ y ∈ fvS s, y ∉ names := by
        intro y hy hn
        exact hf (List.any_eq_true.2 ⟨y, hy, by simpa using hn⟩)
      have ags : AgreeOn (fvS s ++ D.reads) ρ ρ' := by
        intro y hy
        rcases List.mem_append.1 hy with hy | hy
        · exact hag y (hf' y hy)
        · exact hag y (hD y hy).1
      have h1' := (indep n).one h1 ags
      have hag' : ∀ x, x ∉ names → (Δ1 ++ ρ).lookup x = (Δ1 ++ ρ').lookup x := by
        intro x hx
        rw [lookup_append, lookup_append, hag x hx]
      obtain ⟨parts, hl, hm, hl2, N, Δ', hN⟩ :=
        supp_run ss names (fun x hx => ⟨(hD x hx).1, hD' x hx⟩) hag' h2
      obtain ⟨N', hN'⟩ := execS_cons_compose h1' hN
      exact ⟨(m1, l1) :: parts, by simp [hl], by simp [hm], by simp [hl2], N', _,
        by simpa [suppFrom, hf, keepBy] using hN'⟩

/-- Suppress, complete: the suppressed document evaluates, to the old model and log with the
closure's fragments removed (`ms`, the fragment of `s`, and the taken-out `parts`), every other
fragment exactly as it was. -/
theorem suppress_complete {n D σ xs s ys Δ m l} {id : Name}
    (hD : ∀ x ∈ D.reads, x ∉ binders [s] ++ [id] ∧ x ∉ binders ys)
    (h : execS n D σ [] (xs ++ s :: ys) = .ok (Δ, m, l)) :
    ∃ m1 l1 ms ls parts, parts.length = ys.length ∧
      m = m1 ++ ms ++ (parts.map Prod.fst).flatten ∧ l = l1 ++ ls ++ (parts.map Prod.snd).flatten ∧
      ∃ N Δ', execS N D σ [] (suppressAt s id xs ys) =
        .ok (Δ', m1 ++ ((keepBy (suppFrom (binders [s] ++ [id]) ys) parts).map Prod.fst).flatten,
          l1 ++ ((keepBy (suppFrom (binders [s] ++ [id]) ys) parts).map Prod.snd).flatten) := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  have hag : ∀ x, x ∉ binders [s] ++ [id] → (Δs ++ (Δ1 ++ [])).lookup x = (Δ1 ++ []).lookup x := by
    intro x hx
    simp only [List.mem_append, not_or] at hx
    rw [lookup_append, lookup_none (fun hk => hx.1 (keys_binders hs x hk))]
    rfl
  obtain ⟨parts, hl, hm, hl2, N, Δ', hN⟩ := supp_run ys _ hD hag h3
  obtain ⟨N', hN'⟩ := execS_compose' h1 hN
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  exact ⟨m1, l1, ms, ls, parts, hl, by simp [hm], by simp [hl2], N', _, by simpa [suppressAt] using hN'⟩

/-! ## The text: statements, not lines

A line of the source is a list of pieces, each a fragment of one statement (a statement over
several lines has a fragment on each) or a gap (blanks, separators, a comment). Suppress tags
the pieces of the statements taken out, and may tag a gap (`apply_suppress` takes the blanks
before a statement and a comment after it). Each run of tagged pieces goes on a line of its own,
`#~ID LINE`, with `<` when Suppress put in the line break before it, `>` when it put in the one
after it; the untagged runs stay as they are. Unsuppress strips the tags and takes those breaks
out again. -/

abbrev Line := List Char

/-- A piece of a line: a fragment of statement `stmt`, or a gap (`none`). -/
structure Piece where
  stmt : Option Nat
  text : Line

def textOf (ps : List Piece) : Line := (ps.map Piece.text).flatten

/-- A line of the suppressed text: kept as it is, or tagged, with whether the break before it and
the break after it were put in. -/
inductive OLine where
  | live (ps : List Piece)
  | tagd (before after : Bool) (ps : List Piece)

def OLine.pieces : OLine → List Piece
  | .live ps => ps
  | .tagd _ _ ps => ps

def OLine.text (o : OLine) : Line := textOf o.pieces

/-- The pieces the parser reads: those of the untagged lines. -/
def OLine.liveP : OLine → List Piece
  | .live ps => ps
  | .tagd _ _ _ => []

def OLine.before : OLine → Bool
  | .tagd b _ _ => b
  | .live _ => false

def OLine.after : OLine → Bool
  | .tagd _ a _ => a
  | .live _ => false

/-- The pieces of a line in runs, each tagged or not as its pieces are. -/
def runs (out : Piece → Bool) : List Piece → List (Bool × List Piece)
  | [] => []
  | p :: ps => match runs out ps with
    | (b, qs) :: rs => if out p = b then (b, p :: qs) :: rs else (out p, [p]) :: (b, qs) :: rs
    | [] => [(out p, [p])]

/-- Whether an untagged run comes next on the line. -/
def nextLive : List (Bool × List Piece) → Bool
  | [] => false
  | (b, _) :: _ => !b

/-- A run as a line: tagged, marking the break before it unless it starts the line (`f`) and the
break after it when an untagged run follows (`n`), or kept. -/
def mkLine (f b n : Bool) (ps : List Piece) : OLine := if b then .tagd (!f) n ps else .live ps

theorem mkLine_text (f b n : Bool) (ps : List Piece) : (mkLine f b n ps).text = textOf ps := by
  cases b <;> rfl

def emit : Bool → List (Bool × List Piece) → List OLine
  | _, [] => []
  | f, (b, ps) :: rest => mkLine f b (nextLive rest) ps :: emit false rest

def supLine (out : Piece → Bool) (ps : List Piece) : List OLine :=
  match runs out ps with
  | [] => [.live []]
  | r :: rs => emit true (r :: rs)

/-- `Suppress` on the text: each line's runs on lines of their own. -/
def suppressText (out : Piece → Bool) (L : List (List Piece)) : List OLine := L.flatMap (supLine out)

/-- Two lines of the suppressed text were one line: a break between them was put in. -/
def joins (o p : OLine) : Bool := o.after || p.before

/-- Line `o` before the lines `ls` Unsuppress gives for what follows, joined to the first when
`j`. -/
def glue (o : OLine) (j : Bool) : List Line → List Line
  | [] => [o.text]
  | l :: ls => if j then (o.text ++ l) :: ls else o.text :: l :: ls

/-- `Unsuppress`: the tags stripped, each break put in taken out. -/
def unsup : List OLine → List Line
  | [] => []
  | [o] => [o.text]
  | o :: p :: ps => glue o (joins o p) (unsup (p :: ps))

theorem glue_append (o : OLine) (j : Bool) (l : Line) (ls U : List Line) :
    glue o j (l :: ls ++ U) = glue o j (l :: ls) ++ U := by
  cases j <;> rfl

theorem unsup_ne_nil : ∀ (o : OLine) (ps : List OLine), ∃ l ls, unsup (o :: ps) = l :: ls
  | o, [] => ⟨o.text, [], rfl⟩
  | o, p :: ps => by
    obtain ⟨l, ls, h⟩ := unsup_ne_nil p ps
    show ∃ l ls, glue o (joins o p) (unsup (p :: ps)) = l :: ls
    rw [h]
    cases joins o p
    · exact ⟨_, _, rfl⟩
    · exact ⟨_, _, rfl⟩

/-- Unsuppress is line by line where no break was put in. -/
theorem unsup_append : ∀ (A : List OLine) (o : OLine) (B : List OLine) (p : OLine),
    joins o p = false → unsup (A ++ o :: p :: B) = unsup (A ++ [o]) ++ unsup (p :: B)
  | [], o, B, p, hj => by
    obtain ⟨l, ls, h⟩ := unsup_ne_nil p B
    show glue o (joins o p) (unsup (p :: B)) = [o.text] ++ unsup (p :: B)
    rw [h, hj]
    rfl
  | a :: A, o, B, p, hj => by
    have ih := unsup_append A o B p hj
    cases A with
    | nil =>
      show glue a (joins a o) (unsup (o :: p :: B)) = glue a (joins a o) [o.text] ++ unsup (p :: B)
      rw [show unsup (o :: p :: B) = unsup ([] ++ o :: p :: B) from rfl, ih]
      exact glue_append a _ o.text [] _
    | cons a' A' =>
      show glue a (joins a a') (unsup (a' :: (A' ++ o :: p :: B))) =
        glue a (joins a a') (unsup (a' :: (A' ++ [o]))) ++ unsup (p :: B)
      rw [show a' :: (A' ++ o :: p :: B) = (a' :: A') ++ o :: p :: B from rfl, ih]
      obtain ⟨l, ls, h⟩ := unsup_ne_nil a' (A' ++ [o])
      rw [show (a' :: A') ++ [o] = a' :: (A' ++ [o]) from rfl, h]
      exact glue_append a _ l ls _

/-- The runs, in order, are the line's pieces. -/
theorem runs_flatten (out : Piece → Bool) : ∀ ps : List Piece, ((runs out ps).map Prod.snd).flatten = ps
  | [] => rfl
  | p :: ps => by
    have ih := runs_flatten out ps
    unfold runs
    split
    · rename_i b qs rs h
      rw [h] at ih
      split <;> simp_all
    · rename_i h
      rw [h] at ih
      simp_all

/-- Next runs differ: a run is tagged exactly when the one before is not. -/
def Alt : List (Bool × List Piece) → Prop
  | r :: s :: rs => r.1 ≠ s.1 ∧ Alt (s :: rs)
  | _ => True

theorem runs_alt (out : Piece → Bool) : ∀ ps : List Piece, Alt (runs out ps)
  | [] => trivial
  | p :: ps => by
    have ih := runs_alt out ps
    unfold runs
    split
    · rename_i b qs rs h
      rw [h] at ih
      split
      · cases rs with
        | nil => trivial
        | cons r rs => exact ih
      · rename_i e
        exact ⟨e, ih⟩
    · trivial

theorem runs_ne_nil (out : Piece → Bool) (p : Piece) (ps : List Piece) : runs out (p :: ps) ≠ [] := by
  unfold runs
  split
  · split <;> simp
  · simp

theorem textOf_append (a b : List Piece) : textOf (a ++ b) = textOf a ++ textOf b := by
  simp [textOf]

/-- A line's runs, unsuppressed: one line again, its pieces' text. -/
theorem unsup_emit : ∀ (f : Bool) (r : Bool × List Piece) (rs : List (Bool × List Piece)),
    Alt (r :: rs) → unsup (emit f (r :: rs)) = [textOf ((r :: rs).map Prod.snd).flatten]
  | f, (b, ps), [], _ => by
    show [(mkLine f b false ps).text] = _
    simp [mkLine_text]
  | f, (b, ps), (b', qs) :: rest, hc => by
    have ih := unsup_emit false (b', qs) rest hc.2
    have hb : b ≠ b' := hc.1
    have j : joins (mkLine f b (nextLive ((b', qs) :: rest)) ps) (mkLine false b' (nextLive rest) qs) = true := by
      cases b <;> cases b' <;> simp_all [joins, mkLine, nextLive, OLine.after, OLine.before]
    show glue (mkLine f b (nextLive ((b', qs) :: rest)) ps) _ (unsup (emit false ((b', qs) :: rest))) = _
    rw [ih, j]
    simp [glue, mkLine_text, textOf_append]

theorem supLine_text (out : Piece → Bool) (ps : List Piece) : unsup (supLine out ps) = [textOf ps] := by
  unfold supLine
  cases hr : runs out ps with
  | nil =>
    cases ps with
    | nil => rfl
    | cons p ps => exact absurd hr (runs_ne_nil out p ps)
  | cons r rs =>
    have ha := runs_alt out ps
    rw [hr] at ha
    have hf := runs_flatten out ps
    rw [hr] at hf
    simp only
    rw [unsup_emit true r rs ha, hf]

theorem emit_last : ∀ (f : Bool) (r : Bool × List Piece) (rs : List (Bool × List Piece)),
    ∃ A o, emit f (r :: rs) = A ++ [o] ∧ o.after = false
  | f, (b, ps), [] => ⟨[], mkLine f b false ps, rfl, by cases b <;> rfl⟩
  | f, (b, ps), r :: rest => by
    obtain ⟨A, o, h, ho⟩ := emit_last false r rest
    exact ⟨mkLine f b (nextLive (r :: rest)) ps :: A, o, by
      show mkLine f b (nextLive (r :: rest)) ps :: emit false (r :: rest) = _
      rw [h]; rfl, ho⟩

theorem supLine_cons (out : Piece → Bool) (ps : List Piece) :
    ∃ o os, supLine out ps = o :: os ∧ o.before = false := by
  unfold supLine
  cases runs out ps with
  | nil => exact ⟨_, _, rfl, rfl⟩
  | cons r rs =>
    obtain ⟨b, qs⟩ := r
    exact ⟨_, _, rfl, by cases b <;> rfl⟩

theorem supLine_last (out : Piece → Bool) (ps : List Piece) :
    ∃ A o, supLine out ps = A ++ [o] ∧ o.after = false := by
  unfold supLine
  cases runs out ps with
  | nil => exact ⟨[], _, rfl, rfl⟩
  | cons r rs => exact emit_last true r rs

/-- Unsuppress after Suppress is the identity on the text: every line back as it was, byte for
byte, whatever shares a line with what was taken out. -/
theorem unsuppress_suppress (out : Piece → Bool) :
    ∀ L : List (List Piece), unsup (suppressText out L) = L.map textOf
  | [] => rfl
  | [ps] => by simp [suppressText, supLine_text]
  | ps :: qs :: L => by
    have ih := unsuppress_suppress out (qs :: L)
    obtain ⟨A, o, hA, ho⟩ := supLine_last out ps
    obtain ⟨p, B, hB, hp⟩ := supLine_cons out qs
    have e2 : suppressText out (qs :: L) = p :: (B ++ suppressText out L) := by
      simp only [suppressText, List.flatMap_cons, hB, List.cons_append]
    have e : suppressText out (ps :: qs :: L) = A ++ o :: p :: (B ++ suppressText out L) := by
      rw [show suppressText out (ps :: qs :: L) = supLine out ps ++ suppressText out (qs :: L) by
        simp [suppressText], e2, hA]
      simp
    rw [e, unsup_append A o _ p (by simp [joins, ho, hp]), ← hA, supLine_text, ← e2, ih]
    rfl

/-- The pieces the parser reads after Suppress are the untagged ones: of every line, in order. -/
theorem suppress_live_pieces (out : Piece → Bool) (L : List (List Piece)) :
    (suppressText out L).flatMap OLine.liveP = L.flatten.filter (fun p => !out p) := by
  have runs_live : ∀ ps : List Piece,
      (runs out ps).flatMap (fun r => if r.1 then [] else r.2) = ps.filter (fun p => !out p) := by
    intro ps
    induction ps with
    | nil => rfl
    | cons p ps ih =>
      unfold runs
      split
      · rename_i b qs rs h
        rw [h] at ih
        split
        · rename_i e
          cases hb : b <;> simp_all
        · cases hp : out p <;> simp_all
      · rename_i h
        rw [h] at ih
        cases hp : out p <;> simp_all
  have emit_live : ∀ (f : Bool) (rs : List (Bool × List Piece)),
      (emit f rs).flatMap OLine.liveP = rs.flatMap (fun r => if r.1 then [] else r.2) := by
    intro f rs
    induction rs generalizing f with
    | nil => rfl
    | cons r rs ih =>
      obtain ⟨b, ps⟩ := r
      simp only [emit, List.flatMap_cons, ih]
      cases b <;> rfl
  have line : ∀ ps : List Piece, (supLine out ps).flatMap OLine.liveP = ps.filter (fun p => !out p) := by
    intro ps
    unfold supLine
    have hl := runs_live ps
    cases hr : runs out ps with
    | nil =>
      cases ps with
      | nil => rfl
      | cons p ps => exact absurd hr (runs_ne_nil out p ps)
    | cons r rs =>
      rw [hr] at hl
      simp only
      rw [emit_live, hl]
  induction L with
  | nil => rfl
  | cons ps L ih =>
    simp only [suppressText, List.flatMap_cons, List.flatMap_append, List.flatten_cons,
      List.filter_append] at ih ⊢
    rw [line, ← ih]

/-- Suppress, on the text: the parser reads exactly the fragments of the statements kept, every
one of them, in order, however the statements share lines. `out` must tag the fragments of the
statements flagged and no fragment of one kept; a gap it may tag or not. -/
theorem suppress_live (out : Piece → Bool) (flag : Nat → Bool) (L : List (List Piece))
    (h : ∀ p ∈ L.flatten, ∀ i, p.stmt = some i → out p = flag i) :
    ((suppressText out L).flatMap OLine.liveP).filterMap Piece.stmt =
      (L.flatten.filterMap Piece.stmt).filter (fun i => !flag i) := by
  rw [suppress_live_pieces]
  generalize L.flatten = ps at h
  induction ps with
  | nil => rfl
  | cons p ps ih =>
    have ih' := ih (fun q hq => h q (List.mem_cons_of_mem _ hq))
    cases hs : p.stmt with
    | none =>
      cases hp : out p <;> simp [hp, hs, ih']
    | some i =>
      have := h p (List.mem_cons_self ..) i hs
      cases hp : out p <;> simp_all

/-- Statements one fragment each, in order: the statements the parser reads after Suppress are
`keepBy`, the statements `suppressAt` keeps. Nothing about lines: any number share one. -/
theorem keepBy_range {α : Type} : ∀ (bs : List Bool) (xs : List α), bs.length = xs.length →
    ((List.range xs.length).filter (fun i => !bs.getD i false)).filterMap (xs[·]?) = keepBy bs xs
  | [], [], _ => rfl
  | b :: bs, x :: xs, hl => by
    have ih := keepBy_range bs xs (by simpa using hl)
    cases b <;> simp [keepBy, List.filterMap_map, List.filter_map, List.range_succ_eq_map,
      Function.comp_def, ← ih]

theorem suppress_stmts (out : Piece → Bool) (L : List (List Piece)) (flags : List Bool) (ss : List Stmt)
    (hl : flags.length = ss.length)
    (hone : L.flatten.filterMap Piece.stmt = List.range ss.length)
    (h : ∀ p ∈ L.flatten, ∀ i, p.stmt = some i → out p = flags.getD i false) :
    (((suppressText out L).flatMap OLine.liveP).filterMap Piece.stmt).filterMap (ss[·]?) = keepBy flags ss := by
  rw [suppress_live out _ L h, hone, keepBy_range flags ss hl]

theorem suppFrom_length : ∀ (names : List Name) (ss : List Stmt), (suppFrom names ss).length = ss.length
  | _, [] => rfl
  | names, s :: ss => by
    unfold suppFrom; split <;> simp [suppFrom_length]

/-- Suppress on the text, one fragment a statement: what the parser reads is `suppressAt`'s
closure kept, `keepBy (suppFrom ..)`, with no assumption on lines. -/
theorem suppress_text_closure (out : Piece → Bool) (L : List (List Piece)) (s : Stmt) (id : Name) (ys : List Stmt)
    (hone : L.flatten.filterMap Piece.stmt = List.range ys.length)
    (h : ∀ p ∈ L.flatten, ∀ i, p.stmt = some i → out p = (suppFrom (binders [s] ++ [id]) ys).getD i false) :
    (((suppressText out L).flatMap OLine.liveP).filterMap Piece.stmt).filterMap (ys[·]?) =
      keepBy (suppFrom (binders [s] ++ [id]) ys) ys :=
  suppress_stmts out L _ ys (suppFrom_length _ _) hone h

/-! ## The tag on a line -/

/-- The marks after the tag: `<`, `>`, both, or none, then a space. -/
def marks (b a : Bool) : Line := (if b then ['<'] else []) ++ (if a then ['>'] else []) ++ [' ']

def render (tag : Line) : OLine → Line
  | .live ps => textOf ps
  | .tagd b a ps => tag ++ marks b a ++ textOf ps

/-- Unsuppress reading a line: a tagged line's marks and the line as it was, or none. -/
def readTag (tag : Line) (l : Line) : Option (Bool × Bool × Line) :=
  if tag.isPrefixOf l then
    match l.drop tag.length with
    | '<' :: '>' :: ' ' :: t => some (true, true, t)
    | '<' :: ' ' :: t => some (true, false, t)
    | '>' :: ' ' :: t => some (false, true, t)
    | ' ' :: t => some (false, false, t)
    | _ => none
  else none

theorem isPrefixOf_append (tag l : Line) : tag.isPrefixOf (tag ++ l) = true := by
  induction tag with
  | nil => rfl
  | cons c t ih => simp [ih]

/-- A tagged line reads back as its marks and its text. -/
theorem readTag_render (tag : Line) (b a : Bool) (ps : List Piece) :
    readTag tag (render tag (.tagd b a ps)) = some (b, a, textOf ps) := by
  simp only [render, readTag, List.append_assoc, isPrefixOf_append, ↓reduceIte, List.drop_left]
  cases b <;> cases a <;> rfl

/-- An untagged line reads as untagged when it does not start with a tag already (what
`apply_suppress` refuses, and a statement's text after a break put in starts with code or a
blank, never `#`). -/
theorem readTag_live (tag : Line) (ps : List Piece) (h : readTag tag (textOf ps) = none) :
    readTag tag (render tag (.live ps)) = none := h

end ForgeLang
