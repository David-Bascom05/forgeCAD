import ForgeLang.Complete

/-!
# Suppress and Unsuppress (`interface/forge-lang/src/edit_ops.rs`)

`Suppress {id}` takes out the statement making `id` and every later statement that reads what the
ones taken out bind (`suppFrom`: the closure, in one pass, as `apply_suppress` computes it), each
line kept as a comment `#~id LINE`. `Unsuppress {id}` strips the tag.

* `suppress_complete`: if the document evaluated and no function or component body reads a name
  the closure binds, the suppressed document evaluates, and its model and log are the old ones
  with exactly the taken-out statements' fragments removed, every other fragment (ids, traces)
  unchanged. Nothing outside the closure needs to be checked: the closure is closed under reads.
* `unsuppress_suppress`: on the text, Unsuppress after Suppress is the identity, given that no
  line already starts with the tag (what `apply_suppress` refuses).
* `lines_eq_stmts`: commenting the closure's lines is taking out the closure's statements when
  each statement has a line of its own.

| Assumption | Counterexample without it |
|---|---|
| One statement per line (or: Suppress comments statements, not lines) | `same_line_drops`: `a = ..; b = ..` on one line, Suppress `b` takes out `a` too, though nothing ties `a` to `b`, and nothing is refused. Found in fg: `apply_suppress` comments the whole line from its start |
| No function or component body reads a taken-out name | Not needed for soundness: the run is then a typed error (`unbound`), refused |
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

/-! ## The text: a tag on each line -/

abbrev Line := List Char

def tagLines (tag : Line) (S : Nat → Bool) (ls : List Line) : List Line :=
  (ls.zipIdx).map fun (l, i) => if S i then tag ++ l else l

def untagLines (tag : Line) (ls : List Line) : List Line :=
  ls.map fun l => if tag.isPrefixOf l then l.drop tag.length else l

theorem isPrefixOf_append (tag l : Line) : tag.isPrefixOf (tag ++ l) = true := by
  induction tag with
  | nil => rfl
  | cons c t ih => simp [ih]

/-- Unsuppress after Suppress is the identity on the text, when no line carried the tag before. -/
theorem unsuppress_suppress (tag : Line) (S : Nat → Bool) (ls : List Line)
    (h : ∀ l ∈ ls, tag.isPrefixOf l = false) : untagLines tag (tagLines tag S ls) = ls := by
  unfold untagLines tagLines
  rw [List.map_map]
  conv => rhs; rw [← List.zipIdx_map_fst 0 ls]
  apply List.map_congr_left
  intro ⟨l, i⟩ hm
  have hl : l ∈ ls := List.fst_mem_of_mem_zipIdx hm
  have h' : ¬ tag <+: l := by
    intro hp; have := h l hl; rw [List.isPrefixOf_iff_prefix.2 hp] at this; cases this
  by_cases hs : S i <;> simp [hs, h']

/-! ## Lines and statements -/

/-- Commenting the lines of the flagged statements: statement `i` goes when a flagged statement
shares its line. -/
def dropLines (lineOf : Nat → Nat) (flags : List Bool) (ss : List Stmt) : List Stmt :=
  ((ss.zipIdx).filter fun p =>
    !((List.range flags.length).any fun j => flags.getD j false && lineOf j == lineOf p.2)).map Prod.fst

theorem keepBy_idx {α : Type} : ∀ (bs : List Bool) (xs : List α) (k : Nat),
    keepBy bs xs = ((xs.zipIdx k).filter fun p => !(bs.getD (p.2 - k) false)).map Prod.fst
  | [], xs, k => by
    have : ((xs.zipIdx k).filter fun p => !(([] : List Bool).getD (p.2 - k) false)) = xs.zipIdx k := by
      simp
    simp only [keepBy, List.getD_nil, Bool.not_false]
    rw [List.filter_eq_self.2 (fun _ _ => rfl), List.zipIdx_map_fst]
  | _ :: _, [], _ => rfl
  | b :: bs, x :: xs, k => by
    have ih := keepBy_idx bs xs (k + 1)
    have e : ((xs.zipIdx (k + 1)).filter fun p => !((b :: bs).getD (p.2 - k) false)) =
        ((xs.zipIdx (k + 1)).filter fun p => !(bs.getD (p.2 - (k + 1)) false)) := by
      apply List.filter_congr
      intro p hp
      have := List.le_snd_of_mem_zipIdx hp
      have h2 : p.2 - k = p.2 - (k + 1) + 1 := by omega
      simp [h2]
    rw [List.zipIdx_cons, List.filter_cons, e]
    cases b <;> simp [keepBy, ih]

theorem suppFrom_length : ∀ (names : List Name) (ss : List Stmt), (suppFrom names ss).length = ss.length
  | _, [] => rfl
  | names, s :: ss => by
    unfold suppFrom; split <;> simp [suppFrom_length]

/-- One statement per line: commenting lines takes out exactly the flagged statements. -/
theorem lines_eq_stmts {lineOf : Nat → Nat} (inj : ∀ i j, lineOf i = lineOf j → i = j)
    (flags : List Bool) (ss : List Stmt) (hl : flags.length = ss.length) :
    dropLines lineOf flags ss = keepBy flags ss := by
  rw [keepBy_idx flags ss 0, dropLines]
  congr 1
  apply List.filter_congr
  intro p hp
  have hi := (List.mem_zipIdx' (x := p.1) (i := p.2) hp).1
  congr 1
  apply Bool.eq_iff_iff.2
  simp only [List.any_eq_true, List.mem_range, Bool.and_eq_true, beq_iff_eq, Nat.sub_zero]
  constructor
  · rintro ⟨j, -, hj, e⟩; rw [← inj _ _ e]; exact hj
  · intro h'; exact ⟨p.2, by omega, h', rfl⟩

/-- Suppress on a document with a line per statement: the commented text is `suppressAt`. -/
theorem suppress_lines {lineOf : Nat → Nat} (inj : ∀ i j, lineOf i = lineOf j → i = j)
    (s : Stmt) (id : Name) (ys : List Stmt) :
    dropLines lineOf (suppFrom (binders [s] ++ [id]) ys) ys = keepBy (suppFrom (binders [s] ++ [id]) ys) ys :=
  lines_eq_stmts inj _ ys (suppFrom_length _ _)

/-- `a = ..; b = ..` on one line, Suppress `b`: the closure is `b` alone, the commented line
takes `a` with it. -/
theorem same_line_drops :
    keepBy [false, true] [Stmt.feat "a" (.cst 1), Stmt.feat "b" (.cst 2)] = [Stmt.feat "a" (.cst 1)] ∧
    dropLines (fun _ => 0) [false, true] [Stmt.feat "a" (.cst 1), Stmt.feat "b" (.cst 2)] = [] :=
  ⟨rfl, rfl⟩

end ForgeLang
