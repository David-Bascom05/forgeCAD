import ForgeLang.Suppress

/-!
# An edit of one statement: what it can change is the closure of its name

SetArg (`edit_ops.rs`, `apply_set_arg`: a call's argument set to any text) and ReplaceFeature on a
statement that is read change that statement and, through what it binds, the statements that read
it, directly or through one another. `ScopedMore.replace_complete` covers the statement nothing
reads; here, any statement:

* `closure_runs`: two runs of the same statements, in environments that agree on every name
  outside `names`, give the same fragment and log at every statement the closure of `names`
  (`suppFrom`, Suppress's closure) does not take.
* `edit_closure`: a document with statement `s` replaced by `s'` binding the same name. If both
  evaluate, the statements before keep their fragments (`m1`, `l1`), and so does every statement
  after that does not read what `s` binds, directly or through another statement
  (`keepBy (suppFrom (binders [s]) ys)` of the parts is the same list). Only `s`'s own fragment
  and its closure's may differ. The closure is a name lookup in the source, as Suppress's is.

The hypothesis is Suppress's: no function or component body reads a name the closure binds
(`D.reads`), the condition under which a closure is syntactic. That the edited document evaluates
is not assumed of anything else: an edit that breaks a reader is a typed error of that reader.
-/

namespace ForgeLang

/-- Two runs of the same statements in environments that agree outside `names`: every statement
the closure of `names` leaves gives the same fragment and log in both. -/
theorem closure_runs : ∀ (ys : List Stmt) (names : List Name) {n D σ ρ ρ' Δ m l Δ' m' l'},
    (∀ x ∈ D.reads, x ∉ names ∧ x ∉ binders ys) →
    (∀ x, x ∉ names → ρ.lookup x = ρ'.lookup x) →
    execS n D σ ρ ys = .ok (Δ, m, l) →
    execS n D σ ρ' ys = .ok (Δ', m', l') →
    ∃ parts parts' : List (Model × Log), parts.length = ys.length ∧ parts'.length = ys.length ∧
      m = (parts.map Prod.fst).flatten ∧ l = (parts.map Prod.snd).flatten ∧
      m' = (parts'.map Prod.fst).flatten ∧ l' = (parts'.map Prod.snd).flatten ∧
      keepBy (suppFrom names ys) parts = keepBy (suppFrom names ys) parts'
  | [], names, n, D, σ, ρ, ρ', Δ, m, l, Δ', m', l', _, _, h, h' => by
    cases n with
    | zero => simp [execS] at h
    | succ n =>
      simp only [execS, Except.ok.injEq, Prod.mk.injEq] at h h'
      obtain ⟨-, rfl, rfl⟩ := h
      obtain ⟨-, rfl, rfl⟩ := h'
      exact ⟨[], [], rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
  | s :: ss, names, n, D, σ, ρ, ρ', Δ, m, l, Δ', m', l', hD, hag, h, h' => by
    obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_cons h
    obtain ⟨Δ1', m1', l1', Δ2', m2', l2', h1', h2', e'⟩ := execS_cons h'
    simp only [Prod.mk.injEq] at e e'
    obtain ⟨-, rfl, rfl⟩ := e
    obtain ⟨-, rfl, rfl⟩ := e'
    have hD' : ∀ x ∈ D.reads, x ∉ binders ss := fun x hx hb =>
      (hD x hx).2 (by rw [binders_cons]; exact List.mem_append_right _ hb)
    by_cases hf : (fvS s).any (· ∈ names) = true
    · -- taken: what it binds joins the names, on both sides
      have hag' : ∀ x, x ∉ names ++ binders [s] → (Δ1 ++ ρ).lookup x = (Δ1' ++ ρ').lookup x := by
        intro x hx
        simp only [List.mem_append, not_or] at hx
        rw [lookup_append, lookup_none (fun hk => hx.2 (keys_binders h1 x hk)), lookup_append,
          lookup_none (fun hk => hx.2 (keys_binders h1' x hk))]
        exact hag x hx.1
      have hD2 : ∀ x ∈ D.reads, x ∉ names ++ binders [s] ∧ x ∉ binders ss := by
        intro x hx
        refine ⟨?_, hD' x hx⟩
        simp only [List.mem_append, not_or]
        exact ⟨(hD x hx).1, fun hb => (hD x hx).2 (by rw [binders_cons]; exact List.mem_append_left _ hb)⟩
      obtain ⟨parts, parts', hl, hl', hm, hlg, hm', hlg', hk⟩ :=
        closure_runs ss (names ++ binders [s]) hD2 hag' h2 h2'
      refine ⟨(m1, l1) :: parts, (m1', l1') :: parts', by simp [hl], by simp [hl'], by simp [hm],
        by simp [hlg], by simp [hm'], by simp [hlg'], ?_⟩
      simpa [suppFrom, hf, keepBy] using hk
    · -- left: it reads no name of the closure, so both sides run it alike
      have hf' : ∀ y ∈ fvS s, y ∉ names := by
        intro y hy hn
        exact hf (List.any_eq_true.2 ⟨y, hy, by simpa using hn⟩)
      have ags : AgreeOn (fvS s ++ D.reads) ρ ρ' := by
        intro y hy
        rcases List.mem_append.1 hy with hy | hy
        · exact hag y (hf' y hy)
        · exact hag y (hD y hy).1
      have h1'' := (indep n).one h1 ags
      rw [h1''] at h1'
      simp only [Except.ok.injEq, Prod.mk.injEq] at h1'
      obtain ⟨rfl, rfl, rfl⟩ := h1'
      have hag' : ∀ x, x ∉ names → (Δ1 ++ ρ).lookup x = (Δ1 ++ ρ').lookup x := by
        intro x hx
        rw [lookup_append, lookup_append, hag x hx]
      obtain ⟨parts, parts', hl, hl', hm, hlg, hm', hlg', hk⟩ :=
        closure_runs ss names (fun x hx => ⟨(hD x hx).1, hD' x hx⟩) hag' h2 h2'
      refine ⟨(m1, l1) :: parts, (m1, l1) :: parts', by simp [hl], by simp [hl'], by simp [hm],
        by simp [hlg], by simp [hm'], by simp [hlg'], ?_⟩
      simp [suppFrom, hf, keepBy, hk]

theorem binders_eq_of_binder {s s' : Stmt} (hb : s'.binder = s.binder) : binders [s'] = binders [s] := by
  simp only [binders, List.filterMap_cons, List.filterMap_nil, hb]

/-- An edit of one statement keeping its name (SetArg with any text, ReplaceFeature): if the
document evaluates before and after, the fragments and logs before the statement are the same,
and so are those of every later statement outside the closure of what it binds. -/
theorem edit_closure {N N' D σ xs s s' ys Δ m l Δ' m' l'} (hb : s'.binder = s.binder)
    (hD : ∀ x ∈ D.reads, x ∉ binders [s] ∧ x ∉ binders ys)
    (h : execS N D σ [] (xs ++ s :: ys) = .ok (Δ, m, l))
    (h' : execS N' D σ [] (xs ++ s' :: ys) = .ok (Δ', m', l')) :
    ∃ (m1 : Model) (l1 : Log) (ms : Model) (ls : Log) (ms' : Model) (ls' : Log)
      (parts parts' : List (Model × Log)),
      parts.length = ys.length ∧ parts'.length = ys.length ∧
      m = m1 ++ ms ++ (parts.map Prod.fst).flatten ∧ l = l1 ++ ls ++ (parts.map Prod.snd).flatten ∧
      m' = m1 ++ ms' ++ (parts'.map Prod.fst).flatten ∧
      l' = l1 ++ ls' ++ (parts'.map Prod.snd).flatten ∧
      keepBy (suppFrom (binders [s]) ys) parts = keepBy (suppFrom (binders [s]) ys) parts' := by
  have hA := execS_mono h (Nat.le_max_left N N')
  have hB := execS_mono h' (Nat.le_max_right N N')
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: ys) hA
  obtain ⟨Δ1', m1', l1', Δ2', m2', l2', h1', h2', e'⟩ := execS_append xs (s' :: ys) hB
  rw [h1] at h1'
  simp only [Except.ok.injEq, Prod.mk.injEq] at h1'
  obtain ⟨rfl, rfl, rfl⟩ := h1'
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  obtain ⟨Δs', ms', ls', Δ3', m3', l3', hs', h3', e2'⟩ := execS_cons h2'
  have hbs := binders_eq_of_binder hb
  have hag : ∀ x, x ∉ binders [s] → (Δs ++ (Δ1 ++ [])).lookup x = (Δs' ++ (Δ1 ++ [])).lookup x := by
    intro x hx
    have ha : (Δs ++ (Δ1 ++ [])).lookup x = (Δ1 ++ []).lookup x := by
      rw [lookup_append, lookup_none (fun hk => hx (keys_binders hs x hk))]; rfl
    have hb' : (Δs' ++ (Δ1 ++ [])).lookup x = (Δ1 ++ []).lookup x := by
      rw [lookup_append, lookup_none (fun hk => hx (hbs ▸ keys_binders hs' x hk))]; rfl
    rw [ha, hb']
  obtain ⟨parts, parts', hl, hl', hm, hlg, hm', hlg', hk⟩ := closure_runs ys (binders [s]) hD hag h3 h3'
  simp only [Prod.mk.injEq] at e e' e2 e2'
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e'
  obtain ⟨-, rfl, rfl⟩ := e2
  obtain ⟨-, rfl, rfl⟩ := e2'
  exact ⟨m1, l1, ms, ls, ms', ls', parts, parts', hl, hl', by rw [hm]; simp,
    by rw [hlg]; simp, by rw [hm']; simp, by rw [hlg']; simp, hk⟩

end ForgeLang
