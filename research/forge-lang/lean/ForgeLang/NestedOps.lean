import ForgeLang.Nested
import ForgeLang.ScopedMore

/-!
# Scoped ops at depth: an edit to a component, seen through every block that places it

`D' = D.withComp c0 C'` is an edit of component `c0`'s block. `lift_block` carries what the edit
does to one occurrence of a component `c` up one level: a block `xs ++ [occ i c args] ++ ys`
that places `c` directly, whose other statements cannot reach `c0` and after which nothing (nor a
function) reads the occurrence `i`, evaluates after the edit to its old model with `i`'s fragment
replaced as `c`'s occurrences are. `lift_occ` is the same for an occurrence of the placing
component `d`. Applied once per level from `c0` up, it is the edit at any depth (`scoped_occ` by
induction on depth): with the in-block theorems (`scopedDelete_ok`, `scopedAdd_ok`, `scopedMove_ok`,
`scopedReplace_ok`) for `c0` itself, a scoped op is exact at every occurrence of every component that
places it, however deep.

The condition "nothing after reads `i`" is the design rule R2 (Counter/Nested.nested_delete_dangles):
without it a reader `i.x` of the deleted feature dangles.
-/

namespace ForgeLang

/-- What the edit does to an occurrence of a component: before and after, related by `R`. -/
def OccRel (D D' : Defs) (σ : Site → Int) (c : Name) (R : Model × Log → Model × Log → Prop) : Prop :=
  ∀ {n ρ i args Δ m l}, execOne n D σ ρ (.occ i c args) = .ok (Δ, m, l) →
    ∃ k Δ' m' l', execOne k D' σ ρ (.occ i c args) = .ok (Δ', m', l') ∧ R (m, l) (m', l')

/-- One level up: the block placing `c` directly evaluates after the edit, `i`'s fragment changed as
`R` says and every other fragment the same. -/
theorem lift_block {D : Defs} {c0 : Name} {C' : Comp} {S : Name → Prop} (hA : Avoids D c0 S)
    {σ : Site → Int} {c i : Name} {args : List (Name × Expr)} {xs ys : List Stmt}
    {R : Model × Log → Model × Log → Prop} (hc : OccRel D (D.withComp c0 C') σ c R)
    (hxs : ∀ e ∈ placesB xs, S e) (hys : ∀ e ∈ placesB ys, S e)
    (hr : NotRead (.occ i c args) (fvB ys ++ D.reads))
    {n ρ Δ m l} (h : execS n D σ ρ (xs ++ .occ i c args :: ys) = .ok (Δ, m, l)) :
    ∃ m1 ms ms' m3 l1 ls ls' l3, m = m1 ++ ms ++ m3 ∧ l = l1 ++ ls ++ l3 ∧ R (ms, ls) (ms', ls') ∧
      ∃ N Δ', execS N (D.withComp c0 C') σ ρ (xs ++ .occ i c args :: ys) = .ok (Δ', m1 ++ ms' ++ m3, l1 ++ ls' ++ l3) := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (.occ i c args :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hS, h3, e2⟩ := execS_cons h2
  obtain ⟨k, Δs', ms', ls', hS', hR⟩ := hc hS
  -- the statements before: the same under the edit
  have h1' : execS n (D.withComp c0 C') σ ρ xs = .ok (Δ1, m1, l1) := by
    rw [(defs_congr_closed hA n).s hxs]; exact h1
  -- the statements after: they do not read `i`, so `i`'s new fragment changes nothing for them
  have hr' : NotRead (.occ i c args) (fvB ys ++ D.reads) := hr
  have h3a := (indep n).s h3 (agree_drop hS hr _)
  have agree' : AgreeOn (fvB ys ++ D.reads) (Δ1 ++ ρ) (Δs' ++ (Δ1 ++ ρ)) := (agree_drop hS' hr' _).symm
  have h3b := (indep n).s h3a agree'
  have h3c : execS n (D.withComp c0 C') σ (Δs' ++ (Δ1 ++ ρ)) ys = .ok (Δ3, m3, l3) := by
    rw [(defs_congr_closed hA n).s hys]; exact h3b
  obtain ⟨N1, hN1⟩ := execS_cons_compose hS' h3c
  obtain ⟨N, hN⟩ := execS_compose' h1' hN1
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  exact ⟨m1, ms, ms', m3, l1, ls, ls', l3, by simp, by simp, hR, N, _, by simpa [List.append_assoc] using hN⟩

/-- An occurrence of `d`, whose block places `c` as above: after the edit it evaluates, its
fragment the old one with the inner occurrence's changed as `R` says. So `OccRel` for `c` gives
`OccRel` for `d`, and the edit holds at every depth. -/
theorem lift_occ {D : Defs} {c0 : Name} {C' : Comp} {S : Name → Prop} (hA : Avoids D c0 S)
    {σ : Site → Int} {c d j : Name} {Cd : Comp} {args : List (Name × Expr)} {xs ys : List Stmt}
    {R : Model × Log → Model × Log → Prop} (hc : OccRel D (D.withComp c0 C') σ c R)
    (hd : D.comps.lookup d = some Cd) (hdne : d ≠ c0) (hbody : Cd.body = xs ++ .occ j c args :: ys)
    (hxs : ∀ e ∈ placesB xs, S e) (hys : ∀ e ∈ placesB ys, S e)
    (hr : NotRead (.occ j c args) (fvB ys ++ D.reads)) :
    OccRel D (D.withComp c0 C') σ d (fun a b => ∃ p : FId, ∃ m1 ms ms' m3 l1 ls ls' l3 lo,
      a.1 = pre p (m1 ++ ms ++ m3) ∧ b.1 = pre p (m1 ++ ms' ++ m3) ∧
      a.2 = lo ++ (l1 ++ ls ++ l3) ∧ b.2 = lo ++ (l1 ++ ls' ++ l3) ∧ R (ms, ls) (ms', ls')) := by
  intro n ρ i args' Δ m l h
  cases n with
  | zero => simp [execOne] at h
  | succ n =>
    rw [execOne_occ hd] at h
    unfold occRun at h
    split at h
    · rename_i hok
      simp only [bind_ok] at h
      obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hdf, ⟨ρ1, mm, lb⟩, hb, h⟩ := h
      cases h
      rw [hbody] at hb
      obtain ⟨m1, ms, ms', m3, l1, ls, ls', l3, rfl, rfl, hR, K, Δ', hK⟩ := lift_block hA hc hxs hys hr hb
      have hd' : (D.withComp c0 C').comps.lookup d = some Cd := by rw [withComp_lookup_other D C' hdne]; exact hd
      refine ⟨max n K + 1, [(i, .frag (m1 ++ ms' ++ m3))], pre [.nm i] (m1 ++ ms' ++ m3), lg ++ ld ++ (l1 ++ ls' ++ l3), ?_, ?_⟩
      · rw [execOne_occ hd']
        simp only [occRun, hok, ite_true, (defs_congr_closed hA _).a, evalA_mono hg (Nat.le_max_left n K),
          evalA_mono hdf (Nat.le_max_left n K), ok_bind, hbody]
        rw [execS_mono hK (Nat.le_max_right n K)]
        rfl
      · exact ⟨[.nm i], m1, ms, ms', m3, l1, ls, ls', l3, lg ++ ld, rfl, rfl,
          by simp [List.append_assoc], by simp [List.append_assoc], hR⟩
    · cases h

/-- The bottom level: an occurrence of the edited component `c0` itself. A block op that keeps
"evaluates, related by `R`" (under the old definitions) is `OccRel` for `c0`, with nesting allowed
elsewhere (`Avoids` in place of NoPlace) as long as the new block cannot reach `c0`. -/
theorem occ_body_closed {D : Defs} {σ : Site → Int} {c0 : Name} {C C' : Comp} {S : Name → Prop}
    (R : Model × Log → Model × Log → Prop) (hA : Avoids D c0 S) (hc : D.comps.lookup c0 = some C)
    (hp : C'.params = C.params) (hv : C'.vis = C.vis) (hb : ∀ e ∈ placesB C'.body, S e)
    (hbody : ∀ {n ρ0 Δ m l}, execS n D σ ρ0 C.body = .ok (Δ, m, l) →
      ∃ N Δ' m' l', execS N D σ ρ0 C'.body = .ok (Δ', m', l') ∧ R (m, l) (m', l')) :
    OccRel D (D.withComp c0 C') σ c0 (fun a b => ∃ p : FId, ∃ mi mi' lo lb lb',
      a.1 = pre p mi ∧ b.1 = pre p mi' ∧ a.2 = lo ++ lb ∧ b.2 = lo ++ lb' ∧ R (mi, lb) (mi', lb')) := by
  intro n ρ i args Δ m l h
  cases n with
  | zero => simp [execOne] at h
  | succ n =>
    rw [execOne_occ hc] at h
    unfold occRun at h
    split at h
    · rename_i hok
      simp only [bind_ok] at h
      obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, lb⟩, hbr, h⟩ := h
      cases h
      obtain ⟨K, Δ', mm', lb', hK, hR⟩ := hbody hbr
      refine ⟨max n K + 1, [(i, .frag mm')], pre [.nm i] mm', lg ++ ld ++ lb', ?_,
        ⟨[.nm i], mm, mm', lg ++ ld, lb, lb', rfl, rfl, rfl, rfl, hR⟩⟩
      rw [scoped_occ_closed hA hb]
      simp only [occRun, hp, hv, hok, ite_true, evalA_mono hg (Nat.le_max_left n K),
        evalA_mono hd (Nat.le_max_left n K), ok_bind]
      rw [execS_mono hK (Nat.le_max_right n K)]
      rfl
    · cases h

end ForgeLang
