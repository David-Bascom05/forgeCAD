import ForgeLang.Scoped

/-!
# MoveFeature and ReplaceFeature in a block, as op functions; defaults see only shared names

`Scoped.lean` states DeleteFeature and AddFeature in a component's block as functions (refuse, or
return the edited definitions) and proves every occurrence follows. The implementation has two more
block ops, and this file gives them the same form:

* `move_down`, `move_up`: a statement moved past any number of neighbours (not only one, as
  `move_complete`) evaluates, to a permutation of the old model and log (ids are unique, so the
  same id → feature map: `Counter/Order.perm_lookup`). The condition on each neighbour is
  `Indep2`: they bind different names and neither mentions what the other binds.
* `scopedMove` / `scopedMove_ok`: MoveFeature `{id: "c.nm", before: "c.b" | none}` (down or up).
* `replace_complete`, `scopedReplace` / `scopedReplace_ok`: ReplaceFeature `{id: "c.nm"}` by a
  statement binding the same name, when nothing after it reads the name (otherwise the op is a
  re-run, as in the implementation, and no fragment is promised). The old fragment of the
  statement is swapped for the new one's; every other fragment is unchanged.
* `occ_body`: the one lemma these share: an op on a block that keeps "evaluates, related by `Rel`"
  carries over to every occurrence of the block.
* `default_no_capture`: a default that mentions a caller's name that is not shared is `unbound`,
  however the caller binds it (the implementation reads the caller's scope: impl-proof-findings
  E10).
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

/-- Two statements may pass each other: different binders, neither mentions the other's. -/
def Indep2 (D : Defs) (s t : Stmt) : Prop :=
  (∀ x, s.binder = some x → t.binder ≠ some x) ∧ NotRead s (fvS t ++ D.reads) ∧ NotRead t (fvS s ++ D.reads)

def indep2B (D : Defs) (s t : Stmt) : Bool :=
  (match s.binder, t.binder with
    | some x, some y => x != y
    | _, _ => true) &&
  (match s.binder with
    | some x => !decide (x ∈ fvS t ++ D.reads)
    | none => true) &&
  (match t.binder with
    | some y => !decide (y ∈ fvS s ++ D.reads)
    | none => true)

theorem indep2B_spec {D : Defs} {s t : Stmt} (h : indep2B D s t = true) : Indep2 D s t := by
  unfold indep2B at h
  simp only [Bool.and_eq_true] at h
  obtain ⟨⟨h1, h2⟩, h3⟩ := h
  refine ⟨?_, ?_, ?_⟩
  · intro x hx hy
    rw [hx, hy] at h1
    simp at h1
  · intro x hx
    rw [hx] at h2
    simpa using h2
  · intro x hx
    rw [hx] at h3
    simpa using h3

theorem perm_swap4 {α : Type} (a b c d : List α) : (a ++ c ++ b ++ d).Perm (a ++ b ++ c ++ d) := by
  simp only [List.append_assoc]
  exact List.Perm.append_left a (by
    simpa [List.append_assoc] using List.Perm.append_right d (List.perm_append_comm (l₁ := c) (l₂ := b)))

/-- A statement moved down past `ts`: the block evaluates, to a permutation of the model and log. -/
theorem move_down {D : Defs} {σ : Site → Int} {ρ : Env} {s : Stmt} {ys : List Stmt} :
    ∀ (ts xs : List Stmt) {n Δ m l}, (∀ t ∈ ts, Indep2 D s t) →
    execS n D σ ρ (xs ++ (s :: (ts ++ ys))) = .ok (Δ, m, l) →
    ∃ N Δ' m' l', execS N D σ ρ (xs ++ (ts ++ (s :: ys))) = .ok (Δ', m', l') ∧ m'.Perm m ∧ l'.Perm l
  | [], xs, n, Δ, m, l, _, h => ⟨n, Δ, m, l, by simpa using h, List.Perm.refl _, List.Perm.refl _⟩
  | t :: ts, xs, n, Δ, m, l, hi, h => by
    obtain ⟨hst, hs, ht⟩ := hi t List.mem_cons_self
    have h' : execS n D σ ρ (xs ++ s :: t :: (ts ++ ys)) = .ok (Δ, m, l) := by simpa using h
    obtain ⟨m1, ms, mt, m3, l1, ls, lt, l3, rfl, rfl, N, Δ', hN⟩ := move_complete hst hs ht h'
    have hN' : execS N D σ ρ ((xs ++ [t]) ++ (s :: (ts ++ ys))) = .ok (Δ', m1 ++ mt ++ ms ++ m3, l1 ++ lt ++ ls ++ l3) := by
      simpa using hN
    obtain ⟨N2, Δ2, m2, l2, h2, pm, pl⟩ := move_down ts (xs ++ [t]) (fun u hu => hi u (List.mem_cons_of_mem _ hu)) hN'
    exact ⟨N2, Δ2, m2, l2, by simpa using h2, pm.trans (perm_swap4 _ _ _ _), pl.trans (perm_swap4 _ _ _ _)⟩

/-- A statement moved up past `us`: the same. -/
theorem move_up {D : Defs} {σ : Site → Int} {ρ : Env} {s : Stmt} {ys : List Stmt} :
    ∀ (us xs : List Stmt) {n Δ m l}, (∀ u ∈ us, Indep2 D u s) →
    execS n D σ ρ (xs ++ (us ++ (s :: ys))) = .ok (Δ, m, l) →
    ∃ N Δ' m' l', execS N D σ ρ (xs ++ (s :: (us ++ ys))) = .ok (Δ', m', l') ∧ m'.Perm m ∧ l'.Perm l
  | [], xs, n, Δ, m, l, _, h => ⟨n, Δ, m, l, by simpa using h, List.Perm.refl _, List.Perm.refl _⟩
  | u :: us, xs, n, Δ, m, l, hi, h => by
    have h' : execS n D σ ρ ((xs ++ [u]) ++ (us ++ (s :: ys))) = .ok (Δ, m, l) := by simpa using h
    obtain ⟨N1, Δ1, m1', l1', h1, pm1, pl1⟩ := move_up us (xs ++ [u]) (fun v hv => hi v (List.mem_cons_of_mem _ hv)) h'
    obtain ⟨hus, hu, hs⟩ := hi u List.mem_cons_self
    have h1' : execS N1 D σ ρ (xs ++ u :: s :: (us ++ ys)) = .ok (Δ1, m1', l1') := by simpa using h1
    obtain ⟨a1, au, as_, a3, b1, bu, bs, b3, rfl, rfl, N, Δ', hN⟩ := move_complete hus hu hs h1'
    exact ⟨N, Δ', _, _, by simpa using hN, (perm_swap4 _ _ _ _).trans pm1, (perm_swap4 _ _ _ _).trans pl1⟩

/-- ReplaceFeature, complete: `s` replaced by `s'` binding the same name, nothing after it (nor a
function) reading it, and `s'` evaluating where it stands: the block evaluates, with `s`'s
fragment swapped for `s'`'s and every other fragment unchanged. -/
theorem replace_complete {n D σ ρ xs s s' ys Δ m l} (hb : s'.binder = s.binder) (hr : NotRead s (fvB ys ++ D.reads))
    (h : execS n D σ ρ (xs ++ s :: ys) = .ok (Δ, m, l))
    (hs : ∀ Δ1 m1 l1, execS n D σ ρ xs = .ok (Δ1, m1, l1) → ∃ k r, execOne k D σ (Δ1 ++ ρ) s' = .ok r) :
    ∃ m1 ms ms' m3 l1 ls ls' l3, m = m1 ++ ms ++ m3 ∧ l = l1 ++ ls ++ l3 ∧
      ∃ N Δ', execS N D σ ρ (xs ++ s' :: ys) = .ok (Δ', m1 ++ ms' ++ m3, l1 ++ ls' ++ l3) := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hS, h3, e2⟩ := execS_cons h2
  obtain ⟨k, ⟨Δs', ms', ls'⟩, hS'⟩ := hs Δ1 m1 l1 h1
  have hr' : NotRead s' (fvB ys ++ D.reads) := fun x hx => hr x (hb ▸ hx)
  have h3a := (indep n).s h3 (agree_drop hS hr _)
  have h3b := (indep n).s h3a (agree_drop hS' hr' _).symm
  obtain ⟨N1, hN1⟩ := execS_cons_compose hS' h3b
  obtain ⟨N, hN⟩ := execS_compose' h1 hN1
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  exact ⟨m1, ms, ms', m3, l1, ls, ls', l3, by simp, by simp, N, _, by simpa [List.append_assoc] using hN⟩

/-! ## Any block op that keeps "evaluates, related by `Rel`" holds at every occurrence -/

theorem occ_body {D : Defs} {σ : Site → Int} {c : Name} {C : Comp} {B : List Stmt} (Rel : Model × Log → Model × Log → Prop)
    (hD : NoPlace c D) (hc : D.comps.lookup c = some C) (hp : c ∉ placesB B)
    (hbody : ∀ {n ρ0 Δ m l}, execS n D σ ρ0 C.body = .ok (Δ, m, l) →
      ∃ N Δ' m' l', execS N D σ ρ0 B = .ok (Δ', m', l') ∧ Rel (m, l) (m', l'))
    {n ρ x args Δ m l} (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l)) :
    ∃ mi li lb mi' lb', m = pre [.nm x] mi ∧ l = li ++ lb ∧ Rel (mi, lb) (mi', lb') ∧
      ∃ k, execOne k (D.withComp c { C with body := B }) σ ρ (.occ x c args) =
        .ok ([(x, .frag mi')], pre [.nm x] mi', li ++ lb') := by
  cases n with
  | zero => simp [execOne] at h
  | succ n =>
    rw [execOne_occ hc] at h
    unfold occRun at h
    split at h
    · rename_i hok
      simp only [bind_ok] at h
      obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, lb⟩, hb, h⟩ := h
      cases h
      obtain ⟨K, Δ', mm', lb', hK, hR⟩ := hbody hb
      refine ⟨mm, lg ++ ld, lb, mm', lb', rfl, rfl, hR, max n K + 1, ?_⟩
      rw [scoped_occ (C' := { C with body := B }) hD hp]
      simp only [occRun, hok, ite_true, evalA_mono hg (Nat.le_max_left n K),
        evalA_mono hd (Nat.le_max_left n K), ok_bind]
      rw [execS_mono hK (Nat.le_max_right n K)]
      simp
    · cases h

theorem not_places_perm {c : Name} {a b : List Stmt} (h : a.Perm b) (hb : c ∉ placesB b) : c ∉ placesB a := by
  have e : ∀ l : List Stmt, placesB l = l.flatMap placesS := by
    intro l; induction l with
    | nil => rfl
    | cons s l ih => simp [placesB, ih]
  rw [e] at hb ⊢
  intro hm
  exact hb ((List.Perm.flatMap_right placesS h).subset hm)

theorem occ_perm {D : Defs} {σ : Site → Int} {c : Name} {C : Comp} {B : List Stmt}
    (hD : NoPlace c D) (hc : D.comps.lookup c = some C) (hp : c ∉ placesB B)
    (hbody : ∀ {n ρ0 Δ m l}, execS n D σ ρ0 C.body = .ok (Δ, m, l) →
      ∃ N Δ' m' l', execS N D σ ρ0 B = .ok (Δ', m', l') ∧ m'.Perm m ∧ l'.Perm l)
    {n ρ x args Δ m l} (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l)) :
    ∃ mi li lb mi' lb', m = pre [.nm x] mi ∧ l = li ++ lb ∧ mi'.Perm mi ∧ lb'.Perm lb ∧
      ∃ k, execOne k (D.withComp c { C with body := B }) σ ρ (.occ x c args) =
        .ok ([(x, .frag mi')], pre [.nm x] mi', li ++ lb') := by
  obtain ⟨mi, li, lb, mi', lb', e1, e2, ⟨p1, p2⟩, k, hk⟩ :=
    occ_body (fun a b => b.1.Perm a.1 ∧ b.2.Perm a.2) hD hc hp (fun hb => by
      obtain ⟨N, Δ', m', l', h', p1, p2⟩ := hbody hb
      exact ⟨N, Δ', m', l', h', p1, p2⟩) h
  exact ⟨mi, li, lb, mi', lb', e1, e2, p1, p2, k, hk⟩

/-! ## MoveFeature in a block -/

/-- MoveFeature `{id: "c.nm", before: "c.b"}` (`before = none`: to the block's end). Refused when
the block, the statement or the target is missing, or when the moved statement and one it passes
bind one name or mention what the other binds. -/
def scopedMove (D : Defs) (c nm : Name) (before : Option Name) : Except Refusal Defs :=
  match D.comps.lookup c with
  | none => .error (.noComp c)
  | some C =>
    match findSplit nm C.body with
    | none => .error (.noFeature c nm)
    | some (xs, s, rest) =>
      match before with
      | none =>
        if rest.all (indep2B D s) then .ok (D.withComp c { C with body := xs ++ (rest ++ [s]) })
        else .error (.stillRead c nm)
      | some b =>
        match findSplit b rest with
        | some (ts, t, ys) =>
          if ts.all (indep2B D s) then .ok (D.withComp c { C with body := xs ++ (ts ++ (s :: t :: ys)) })
          else .error (.stillRead c nm)
        | none =>
          match findSplit b xs with
          | some (pre, t, mid) =>
            if (t :: mid).all (fun u => indep2B D u s) then
              .ok (D.withComp c { C with body := pre ++ (s :: ((t :: mid) ++ rest)) })
            else .error (.stillRead c nm)
          | none => .error (.noFeature c b)

/-- MoveFeature in a block, all-or-nothing: if the op returns, every occurrence that evaluated
still evaluates, to a permutation of its old fragment and log (the same features, ids, traces). -/
theorem scopedMove_ok {D D' : Defs} {c nm : Name} {before : Option Name} (hD : NoPlace c D)
    (hop : scopedMove D c nm before = .ok D') {n σ ρ x args Δ m l}
    (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l)) :
    ∃ mi li lb mi' lb', m = pre [.nm x] mi ∧ l = li ++ lb ∧ mi'.Perm mi ∧ lb'.Perm lb ∧
      ∃ k, execOne k D' σ ρ (.occ x c args) = .ok ([(x, .frag mi')], pre [.nm x] mi', li ++ lb') := by
  unfold scopedMove at hop
  cases hc : D.comps.lookup c with
  | none => rw [hc] at hop; cases hop
  | some C =>
    rw [hc] at hop
    simp only at hop
    cases hs : findSplit nm C.body with
    | none => rw [hs] at hop; cases hop
    | some r =>
      obtain ⟨xs, s, rest⟩ := r
      rw [hs] at hop
      simp only at hop
      obtain ⟨hbody, -⟩ := findSplit_spec hs
      have pc : c ∉ placesB C.body := hD.2 (c, C) (lookup_mem hc)
      cases before with
      | none =>
        simp only at hop
        split at hop
        · rename_i hall
          cases hop
          refine occ_perm hD hc (not_places_perm (by rw [hbody]; exact List.Perm.append_left xs (by simp)) pc) ?_ h
          intro n ρ0 Δ m l hb
          rw [hbody] at hb
          have := move_down (ys := []) rest xs (fun t ht => indep2B_spec (List.all_eq_true.1 hall t ht)) (by simpa using hb)
          obtain ⟨N, Δ', m', l', h', p1, p2⟩ := this
          exact ⟨N, Δ', m', l', by simpa using h', p1, p2⟩
        · cases hop
      | some b =>
        simp only at hop
        cases hf : findSplit b rest with
        | some r2 =>
          obtain ⟨ts, t, ys⟩ := r2
          rw [hf] at hop
          simp only at hop
          obtain ⟨hrest, -⟩ := findSplit_spec hf
          split at hop
          · rename_i hall
            cases hop
            refine occ_perm hD hc (not_places_perm ?_ pc) ?_ h
            · rw [hbody, hrest]
              exact List.Perm.append_left xs (by simp)
            · intro n ρ0 Δ m l hb
              rw [hbody, hrest] at hb
              exact move_down (ys := t :: ys) ts xs (fun u hu => indep2B_spec (List.all_eq_true.1 hall u hu)) hb
          · cases hop
        | none =>
          rw [hf] at hop
          simp only at hop
          cases hg : findSplit b xs with
          | none => rw [hg] at hop; cases hop
          | some r3 =>
            obtain ⟨pre', t, mid⟩ := r3
            rw [hg] at hop
            simp only at hop
            obtain ⟨hxs, -⟩ := findSplit_spec hg
            split at hop
            · rename_i hall
              cases hop
              refine occ_perm hD hc (not_places_perm ?_ pc) ?_ h
              · rw [hbody, hxs]
                simp only [List.append_assoc]
                exact List.Perm.append_left pre' (by
                  simpa using (List.perm_middle (a := s) (l₁ := t :: mid) (l₂ := rest)).symm)
              · intro n ρ0 Δ m l hb
                rw [hbody, hxs] at hb
                have hb' : execS n D σ ρ0 (pre' ++ ((t :: mid) ++ (s :: rest))) = .ok (Δ, m, l) := by
                  simpa using hb
                exact move_up (ys := rest) (t :: mid) pre' (fun u hu => indep2B_spec (List.all_eq_true.1 hall u hu)) hb'
            · cases hop

/-! ## ReplaceFeature in a block -/

/-- ReplaceFeature `{id: "c.nm"}` by `s'`. Refused when the block or statement is missing, when
`s'` binds another name, or when something after it (or a function) reads the name: then the op
is a re-run, not a fragment swap. -/
def scopedReplace (D : Defs) (c nm : Name) (s' : Stmt) : Except Refusal Defs :=
  match D.comps.lookup c with
  | none => .error (.noComp c)
  | some C =>
    match findSplit nm C.body with
    | none => .error (.noFeature c nm)
    | some (xs, _, ys) =>
      if s'.binder = some nm then
        if nm ∈ fvB ys ++ D.reads then .error (.stillRead c nm)
        else .ok (D.withComp c { C with body := xs ++ s' :: ys })
      else .error (.taken c nm)

/-- ReplaceFeature in a block: if the op returns and the new statement evaluates where it stands,
every occurrence that evaluated still evaluates, with the old statement's part of its fragment
swapped for the new one's and every other part unchanged. -/
theorem scopedReplace_ok {D D' : Defs} {c nm : Name} {s' : Stmt} (hD : NoPlace c D) (hs' : c ∉ placesS s')
    (hop : scopedReplace D c nm s' = .ok D') {n σ ρ x args Δ m l}
    (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l))
    (hev : ∀ {n ρ0 xs ys Δ1 m1 l1}, (∃ s, (D.comps.lookup c).map Comp.body = some (xs ++ s :: ys)) →
      execS n D σ ρ0 xs = .ok (Δ1, m1, l1) → ∃ k r, execOne k D σ (Δ1 ++ ρ0) s' = .ok r) :
    ∃ mi li lb mi' lb', m = pre [.nm x] mi ∧ l = li ++ lb ∧
      (∃ m1 ms ms' m3 l1 ls ls' l3, mi = m1 ++ ms ++ m3 ∧ lb = l1 ++ ls ++ l3 ∧
        mi' = m1 ++ ms' ++ m3 ∧ lb' = l1 ++ ls' ++ l3) ∧
      ∃ k, execOne k D' σ ρ (.occ x c args) = .ok ([(x, .frag mi')], pre [.nm x] mi', li ++ lb') := by
  unfold scopedReplace at hop
  cases hc : D.comps.lookup c with
  | none => rw [hc] at hop; cases hop
  | some C =>
    rw [hc] at hop
    simp only at hop
    cases hs : findSplit nm C.body with
    | none => rw [hs] at hop; cases hop
    | some r =>
      obtain ⟨xs, s, ys⟩ := r
      rw [hs] at hop
      simp only at hop
      obtain ⟨hbody, hbind⟩ := findSplit_spec hs
      split at hop
      · rename_i hb'
        split at hop
        · cases hop
        · rename_i hnr
          cases hop
          have pc : c ∉ placesB C.body := hD.2 (c, C) (lookup_mem hc)
          have pB : c ∉ placesB (xs ++ s' :: ys) := by
            rw [hbody] at pc
            simp only [placesB_append, placesB, List.mem_append, not_or] at pc ⊢
            exact ⟨pc.1, hs', pc.2.2⟩
          let Rel : Model × Log → Model × Log → Prop := fun a b =>
            ∃ m1 ms ms' m3 l1 ls ls' l3, a.1 = m1 ++ ms ++ m3 ∧ a.2 = l1 ++ ls ++ l3 ∧
              b.1 = m1 ++ ms' ++ m3 ∧ b.2 = l1 ++ ls' ++ l3
          obtain ⟨mi, li, lb, mi', lb', e1, e2, hR, k, hk⟩ := occ_body Rel hD hc pB (fun {n ρ0 Δ m l} hb => by
            rw [hbody] at hb
            have hr : NotRead s (fvB ys ++ D.reads) := fun y hy => by rw [hbind] at hy; cases hy; exact hnr
            obtain ⟨m1, ms, ms', m3, l1, ls, ls', l3, rfl, rfl, N, Δ', hN⟩ :=
              replace_complete (hb'.trans hbind.symm) hr hb (fun Δ1 m1 l1 h1 =>
                @hev n ρ0 xs ys Δ1 m1 l1 ⟨s, by simp [hc, hbody]⟩ h1)
            exact ⟨N, Δ', _, _, hN, m1, ms, ms', m3, l1, ls, ls', l3, rfl, rfl, rfl, rfl⟩) h
          exact ⟨mi, li, lb, mi', lb', e1, e2, hR, k, hk⟩
      · cases hop

/-! ## Defaults see only the shared params -/

/-- `component cup(d: 80, h: d * 2) { b = h }` placed as `c = part(cup(d: 50))` after a caller
binding `d = 104`: the default `h` mentions `d`, which is not a visible (shared) name of the
block, so the occurrence is `unbound d`; the caller's `d` is never read. -/
def cupH : Comp := { params := [("d", .cst 80), ("h", .mul (.var "d") (.cst 2))], body := [.feat "b" (.var "h")], vis := ["w"] }

theorem default_no_capture :
    run 40 { defs := { comps := [("cup", cupH)] },
             body := [.let_ "w" (.lit 1), .let_ "d" (.add (.var "w") (.cst 100)), .occ "c" "cup" [("d", .cst 50)]],
             σ := fun _ => 4 } = .error (.unbound "d") := rfl

/-- And in general: an occurrence's result does not depend on caller names that are neither
mentioned by its arguments nor read by a definition (`Defs.reads` holds only a component's
visible names). -/
theorem occ_caller_indep {n D σ ρ ρ' x c args r} (h : execOne n D σ ρ (.occ x c args) = .ok r)
    (ag : AgreeOn (fvKV args ++ D.reads) ρ ρ') : execOne n D σ ρ' (.occ x c args) = .ok r :=
  (indep n).one h (by simpa [fvS] using ag)

end ForgeLang
