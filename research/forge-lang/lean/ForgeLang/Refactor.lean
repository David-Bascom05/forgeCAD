import ForgeLang.Complete
import ForgeLang.Errors

/-!
# Refactors between inline components, files and bodies

**ExtractComponent / InlineComponent** (`extract_occ`, an equation read both ways). An inline
`component c(ps) { body }` sees the document's shared params (`vis`). In its own file it sees
nothing of this document: its params are the file's shared params. The op turns every visible name
the block reads (`ws`) into a param and every call `c(args)` into `c'(args, W: W, ..)` (the design's
`part(FILE, with: {.., W: W})`). Then each occurrence gives *the same* bindings, model and log, so
ids (`OCC.NAME`) and traces are unchanged and no reference needs a rewrite. Side conditions: every
visible name the block (or a function) reads is passed; the new params are fresh; defaults are
literals. `Counter/Refactor.lean` shows the first is needed in both of its failure modes (an
`unbound` refusal, or a silent cut of the link when the value is copied instead of passed).

**MakeComponent** (`makeComponent`). Moving statements `ms` into `component N() { ms }` and writing
`o = part(N)` in their place: the model is the old one with the moved fragment under `o`. So the
moved features' ids *change* (`lid` becomes `o.lid`); every other id and trace is kept. Side
conditions: no later statement or function reads what `ms` binds (the design's refusal), the new
name is unread, and (missing from the design) `ms` reads no name of the document that the
component cannot see.
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

/-! ## Lemmas on argument lists -/

theorem evalA_nil {n D σ ρ} : evalA (n + 1) D σ ρ [] = .ok ([], []) := rfl

theorem evalA_append : ∀ (a b : List (Name × Expr)) {n D σ ρ Δ l}, evalA n D σ ρ (a ++ b) = .ok (Δ, l) →
    ∃ Δa la Δb lb, evalA n D σ ρ a = .ok (Δa, la) ∧ evalA n D σ ρ b = .ok (Δb, lb) ∧ Δ = Δa ++ Δb ∧ l = la ++ lb
  | [], b, n, D, σ, ρ, Δ, l, h => by
    cases n with
    | zero => simp [evalA] at h
    | succ n => exact ⟨[], [], Δ, l, rfl, h, rfl, rfl⟩
  | (y, e) :: a, b, n, D, σ, ρ, Δ, l, h => by
    cases n with
    | zero => simp [evalA] at h
    | succ n =>
      simp only [List.cons_append, evalA, bind_ok] at h
      obtain ⟨⟨t, l0⟩, h0, ⟨Δ', l'⟩, h1, h⟩ := h
      cases h
      obtain ⟨Δa, la, Δb, lb, ha, hb, rfl, rfl⟩ := evalA_append a b h1
      exact ⟨(y, .num t) :: Δa, l0 ++ la, Δb, lb, by simp [evalA, h0, ha], evalA_mono hb (Nat.le_succ n),
        rfl, by simp⟩

theorem evalA_app : ∀ (a b : List (Name × Expr)) {n m D σ ρ Δa la Δb lb}, evalA n D σ ρ a = .ok (Δa, la) →
    evalA m D σ ρ b = .ok (Δb, lb) → evalA (n + m) D σ ρ (a ++ b) = .ok (Δa ++ Δb, la ++ lb)
  | [], b, n, m, D, σ, ρ, Δa, la, Δb, lb, ha, hb => by
    cases n with
    | zero => simp [evalA] at ha
    | succ n =>
      simp [evalA] at ha; obtain ⟨rfl, rfl⟩ := ha
      simpa using evalA_mono hb (by omega)
  | (y, e) :: a, b, n, m, D, σ, ρ, Δa, la, Δb, lb, ha, hb => by
    cases n with
    | zero => simp [evalA] at ha
    | succ n =>
      simp only [evalA, bind_ok] at ha
      obtain ⟨⟨t, l0⟩, h0, ⟨Δ', l'⟩, h1, h⟩ := ha
      cases h
      have e1 : n + 1 + m = (n + m) + 1 := by omega
      rw [e1]
      simp [evalA, evalE_mono h0 (by omega : n ≤ n + m), evalA_app a b h1 hb]

/-- Literal arguments read nothing: their evaluation is the same in every environment. -/
def AllLit (as : List (Name × Expr)) : Prop := ∀ p ∈ as, ∃ s, p.2 = .lit s

theorem evalA_lit : ∀ (as : List (Name × Expr)) {n D σ ρ ρ'}, AllLit as → evalA n D σ ρ as = evalA n D σ ρ' as
  | [], n, _, _, _, _, _ => by cases n <;> rfl
  | (y, e) :: as, n, D, σ, ρ, ρ', h => by
    cases n with
    | zero => rfl
    | succ n =>
      obtain ⟨s, hs⟩ := h (y, e) (by simp)
      simp only at hs
      subst hs
      cases n with
      | zero => rfl
      | succ k =>
        simp only [evalA, evalE, ok_bind]
        rw [evalA_lit as (fun p hp => h p (by simp [hp]))]

theorem AllLit.filter {as : List (Name × Expr)} (h : AllLit as) (p : Name × Expr → Bool) : AllLit (as.filter p) :=
  fun q hq => h q (List.mem_filter.1 hq).1

/-- `W: W` for each passed name. -/
def wargs (ws : List Name) : List (Name × Expr) := ws.map fun w => (w, .var w)

theorem wargs_ok : ∀ (ws : List Name) {n D σ ρ Δ l}, evalA n D σ ρ (wargs ws) = .ok (Δ, l) →
    l = [] ∧ ∀ y, Δ.lookup y = if y ∈ ws then ρ.lookup y else none
  | [], n, _, _, _, _, _, h => by
    cases n <;> simp [evalA, wargs] at h
    obtain ⟨rfl, rfl⟩ := h; simp [List.lookup]
  | w :: ws, n, D, σ, ρ, Δ, l, h => by
    cases n with
    | zero => simp [evalA, wargs] at h
    | succ n =>
      simp only [wargs, List.map_cons, evalA, bind_ok] at h
      obtain ⟨⟨t, l0⟩, h0, ⟨Δ', l'⟩, h1, h⟩ := h
      cases h
      cases n with
      | zero => simp [evalE] at h0
      | succ k =>
        simp only [evalE, bind_ok] at h0
        obtain ⟨t', ht, h0⟩ := h0
        simp only [Except.ok.injEq, Prod.mk.injEq] at h0
        obtain ⟨rfl, rfl⟩ := h0
        obtain ⟨rfl, hl⟩ := wargs_ok ws h1
        refine ⟨by simp, fun y => ?_⟩
        have hb : ρ.lookup w = some (.num t') := by
          unfold lookupNum at ht
          split at ht
          · rename_i t'' h''; cases ht; exact h''
          · cases ht
          · cases ht
        by_cases hy : y = w
        · subst hy; simp [List.lookup, hb]
        · have e : (y == w) = false := by simp [hy]
          simp only [List.lookup, e, hl, List.mem_cons, hy, false_or]

theorem wargs_exists : ∀ (ws : List Name) {D σ ρ}, (∀ w ∈ ws, ∃ t, ρ.lookup w = some (.num t)) →
    ∃ k Δ, evalA k D σ ρ (wargs ws) = .ok (Δ, [])
  | [], _, _, _, _ => ⟨1, [], rfl⟩
  | w :: ws, D, σ, ρ, h => by
    obtain ⟨t, ht⟩ := h w (by simp)
    obtain ⟨k, Δ, hk⟩ := wargs_exists ws (D := D) (σ := σ) (fun w hw => h w (by simp [hw]))
    refine ⟨k + 2, (w, .num t) :: Δ, ?_⟩
    have hk' := evalA_mono hk (by omega : k ≤ k + 1)
    have hv : evalE (k + 1) D σ ρ (.var w) = .ok (t, []) := by simp [evalE, lookupNum, ht]
    show evalA (k + 1 + 1) D σ ρ ((w, .var w) :: wargs ws) = _
    simp only [evalA, hv, ok_bind, hk']
    rfl

/-! ## ExtractComponent and InlineComponent -/

/-- The inline block `c` and the file form `c'`: the same body, the passed names as params, nothing
of the document visible. -/
structure ExtractPair (D : Defs) (c c' : Name) (ws : List Name) (dw : Name → Expr) where
  C : Comp
  hc : D.comps.lookup c = some C
  hc' : D.comps.lookup c' = some ⟨C.params ++ ws.map (fun w => (w, dw w)), C.body, []⟩
  /-- passed names are visible to the block -/
  wvis : ∀ w ∈ ws, w ∈ C.vis
  /-- and fresh as params -/
  wfresh : ∀ w ∈ ws, w ∉ C.params.map Prod.fst
  /-- every visible name the block or a function reads is passed -/
  reads : ∀ y ∈ fvB C.body ++ D.reads, y ∈ C.vis → y ∈ ws
  /-- defaults are literals -/
  lits : AllLit C.params

theorem lookup_wps {ws : List Name} {dw : Name → Expr} {y : Name} (hy : y ∉ ws) :
    (ws.map fun w => (w, dw w)).lookup y = none := by
  induction ws with
  | nil => rfl
  | cons w ws ih =>
    simp only [List.mem_cons, not_or] at hy
    have e : (y == w) = false := by simp [hy.1]
    simp [List.lookup, e, ih hy.2]

theorem lookup_wargs_some {ws : List Name} {y : Name} (hy : y ∈ ws) : ((wargs ws).lookup y).isSome := by
  induction ws with
  | nil => cases hy
  | cons w ws ih =>
    simp only [wargs, List.map_cons, List.lookup]
    by_cases h : y = w
    · simp [h]
    · have e : (y == w) = false := by simp [h]
      simp only [e]
      exact ih (by simpa [h] using hy)

theorem lookup_wargs_none {ws : List Name} {y : Name} (hy : y ∉ ws) : (wargs ws).lookup y = none := by
  induction ws with
  | nil => rfl
  | cons w ws ih =>
    simp only [List.mem_cons, not_or] at hy
    have e : (y == w) = false := by simp [hy.1]
    simp only [wargs, List.map_cons, List.lookup, e]
    exact ih hy.2

/-- The calls agree on which params they default. -/
theorem dflts_extract {ps args : List (Name × Expr)} {ws : List Name} {dw : Name → Expr}
    (hf : ∀ w ∈ ws, w ∉ ps.map Prod.fst) (ha : ∀ a ∈ args, a.1 ∉ ws) :
    dflts (ps ++ ws.map (fun w => (w, dw w))) (args ++ wargs ws) = dflts ps args := by
  unfold dflts
  rw [List.filter_append]
  have h2 : (ws.map fun w => (w, dw w)).filter (fun p => ((args ++ wargs ws).lookup p.1).isNone) = [] := by
    rw [List.filter_eq_nil_iff]
    intro p hp
    obtain ⟨w, hw, rfl⟩ := List.mem_map.1 hp
    rw [List.lookup_append]
    cases hl : args.lookup w with
    | some e =>
      have := ha (w, e) (lookup_mem hl)
      exact absurd hw this
    | none =>
      have := lookup_wargs_some hw
      cases h' : (wargs ws).lookup w with
      | none => rw [h'] at this; cases this
      | some _ => simp [Option.or]
  rw [h2, List.append_nil]
  apply List.filter_congr
  intro p hp
  rw [List.lookup_append]
  cases hl : args.lookup p.1 with
  | some e => rfl
  | none =>
    have : p.1 ∉ ws := fun h => hf p.1 h (List.mem_map.2 ⟨p, hp, rfl⟩)
    simp [Option.or, lookup_wargs_none this]

theorem lookup_wps_some {ws : List Name} {dw : Name → Expr} {w : Name} (hw : w ∈ ws) :
    ((ws.map fun w => (w, dw w)).lookup w).isSome := by
  induction ws with
  | nil => cases hw
  | cons v ws ih =>
    simp only [List.map_cons, List.lookup]
    by_cases h : w = v
    · simp [h]
    · have e : (w == v) = false := by simp [h]
      simp only [e]
      exact ih (by simpa [h] using hw)

theorem argsOK_extract {ps args : List (Name × Expr)} {ws : List Name} {dw : Name → Expr}
    (ha : ∀ a ∈ args, a.1 ∉ ws) :
    argsOK (ps ++ ws.map (fun w => (w, dw w))) (args ++ wargs ws) = argsOK ps args := by
  unfold argsOK
  rw [List.all_append]
  have hw : (wargs ws).all (fun a => ((ps ++ ws.map (fun w => (w, dw w))).lookup a.1).isSome) = true := by
    rw [List.all_eq_true]
    intro a ha'
    obtain ⟨w, hw, rfl⟩ := List.mem_map.1 ha'
    rw [List.lookup_append]
    cases ps.lookup w with
    | some _ => rfl
    | none => simpa [Option.or] using lookup_wps_some (dw := dw) hw
  rw [hw, Bool.and_true]
  apply Bool.eq_iff_iff.2
  simp only [List.all_eq_true]
  constructor
  · intro H a h
    have := H a h
    rw [List.lookup_append, lookup_wps (ha a h)] at this
    cases hp : ps.lookup a.1 <;> simp_all [Option.or]
  · intro H a h
    have := H a h
    rw [List.lookup_append, lookup_wps (ha a h)]
    cases hp : ps.lookup a.1 <;> simp_all [Option.or]

theorem argsOK_names {ps args : List (Name × Expr)} (h : argsOK ps args = true) :
    ∀ a ∈ args, a.1 ∈ ps.map Prod.fst := by
  intro a ha
  have := List.all_eq_true.1 h a ha
  obtain ⟨d, hd⟩ := Option.isSome_iff_exists.1 this
  exact List.mem_map.2 ⟨(a.1, d), lookup_mem hd, rfl⟩

/-- The body sees the same names: the passed ones as params instead of as visible names. -/
theorem extract_agree {D : Defs} {C : Comp} {ws : List Name} {ρ Δg Δw Δd : Env}
    (wvis : ∀ w ∈ ws, w ∈ C.vis) (reads : ∀ y ∈ fvB C.body ++ D.reads, y ∈ C.vis → y ∈ ws)
    (hdk : ∀ y ∈ keys Δd, y ∉ ws) (hw : ∀ y, Δw.lookup y = if y ∈ ws then ρ.lookup y else none) :
    AgreeOn (fvB C.body ++ D.reads) (Δg ++ (Δd ++ only C.vis ρ)) (Δg ++ Δw ++ (Δd ++ only [] ρ)) := by
  intro y hy
  rw [List.append_assoc]
  simp only [lookup_append, lookup_only]
  cases Δg.lookup y with
  | some _ => rfl
  | none =>
    simp only [Option.or, hw]
    by_cases hyw : y ∈ ws
    · have : Δd.lookup y = none := lookup_none (fun h => hdk y h hyw)
      simp only [hyw, ite_true, this, wvis y hyw]
      cases ρ.lookup y <;> rfl
    · simp only [hyw, ite_false]
      cases Δd.lookup y with
      | some _ => rfl
      | none =>
        simp only [List.not_mem_nil, ite_false]
        split
        · exact absurd (reads y hy ‹_›) hyw
        · rfl

/-- ExtractComponent and InlineComponent: an occurrence of the inline block and the same
occurrence of the file form, with the passed names as arguments, give the same result (bindings,
model, ids, traces, log). Read left to right it is ExtractComponent, right to left
InlineComponent. -/
theorem extract_occ {D : Defs} {σ : Site → Int} {ρ : Env} {x c c' : Name} {ws : List Name}
    {dw : Name → Expr} {args : List (Name × Expr)} {r : Env × Model × Log}
    (P : ExtractPair D c c' ws dw) (hargs : ∀ a ∈ args, a.1 ∉ ws)
    (hnum : ∀ w ∈ ws, ∃ t, ρ.lookup w = some (.num t)) :
    (∃ k, execOne k D σ ρ (.occ x c args) = .ok r) ↔
      (∃ k, execOne k D σ ρ (.occ x c' (args ++ wargs ws)) = .ok r) := by
  obtain ⟨C, hc, hc', wvis, wfresh, reads, lits⟩ := P
  have hdk : ∀ {k Δd ld ρ'}, evalA k D σ ρ' (dflts C.params args) = .ok (Δd, ld) → ∀ y ∈ keys Δd, y ∉ ws := by
    intro k Δd ld ρ' hd y hy hyw
    rw [evalA_keys _ hd] at hy
    obtain ⟨p, hp, rfl⟩ := List.mem_map.1 hy
    exact wfresh p.1 hyw (List.mem_map.2 ⟨p, (List.mem_filter.1 hp).1, rfl⟩)
  constructor
  · rintro ⟨k, h⟩
    cases k with
    | zero => simp [execOne] at h
    | succ n =>
      simp only [execOne, hc] at h
      split at h
      · rename_i hok
        simp only [bind_ok] at h
        obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l⟩, hb, h⟩ := h
        cases h
        simp only [List.append_assoc] at hb
        obtain ⟨kw, Δw, hw⟩ := wargs_exists ws (D := D) (σ := σ) hnum
        have hwl := (wargs_ok ws hw).2
        have hg' := evalA_app args (wargs ws) hg hw
        have hd' : evalA (n + kw) D σ (only [] ρ) (dflts C.params args) = .ok (Δd, ld) := by
          unfold dflts at hd ⊢
          rw [evalA_lit _ (lits.filter _)]; exact evalA_mono hd (by omega)
        have hb' := (indep n).s hb (extract_agree wvis reads (hdk hd) hwl)
        refine ⟨n + kw + 1, ?_⟩
        simp only [execOne, hc', argsOK_extract hargs, hok, ite_true, dflts_extract wfresh hargs]
        simp only [hg', hd', ok_bind, List.append_assoc]
        rw [execS_mono (by simpa [List.append_assoc] using hb') (by omega : n ≤ n + kw)]
        simp
      · cases h
  · rintro ⟨k, h⟩
    cases k with
    | zero => simp [execOne] at h
    | succ n =>
      simp only [execOne, hc'] at h
      split at h
      · rename_i hok
        rw [argsOK_extract hargs] at hok
        simp only [dflts_extract wfresh hargs, bind_ok] at h
        obtain ⟨⟨Δg', lg'⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l⟩, hb, h⟩ := h
        cases h
        obtain ⟨Δg, lg, Δw, lw, hga, hw, rfl, rfl⟩ := evalA_append args (wargs ws) hg
        obtain ⟨rfl, hwl⟩ := wargs_ok ws hw
        simp only [List.append_assoc] at hb
        have hb2 : execS n D σ (Δg ++ Δw ++ (Δd ++ only [] ρ)) C.body = .ok (ρ1, mm, l) := by
          simpa [List.append_assoc] using hb
        have hb' := (indep n).s hb2 (extract_agree (Δg := Δg) wvis reads (hdk hd) hwl).symm
        have hd' : evalA n D σ (only C.vis ρ) (dflts C.params args) = .ok (Δd, ld) := by
          unfold dflts at hd ⊢
          rw [evalA_lit _ (lits.filter _)]; exact hd
        refine ⟨n + 1, ?_⟩
        simp only [execOne, hc, hok, ite_true, hga, hd', ok_bind, List.append_assoc]
        rw [hb']
        simp
      · cases h

/-! ## MakeComponent -/

/-- MakeComponent, sound and complete in one: moving `ms` into `component N() { ms }` and placing
it as `o` gives the old model with `ms`'s fragment under `o`, and every other fragment and the log
unchanged. -/
theorem makeComponent {n D σ ρ xs ms ys o N vis Δ m l}
    (hN : D.comps.lookup N = some ⟨[], ms, vis⟩)
    (h : execS n D σ ρ (xs ++ ms ++ ys) = .ok (Δ, m, l))
    (hout : ∀ y ∈ binders ms, y ∉ fvB ys ++ D.reads)
    (ho : o ∉ fvB ys ++ D.reads)
    (hin : ∀ y ∈ fvB ms ++ D.reads, y ∉ vis → y ∉ binders xs ∧ ρ.lookup y = none) :
    ∃ m1 mm m3 l1 lm l3, m = m1 ++ mm ++ m3 ∧ l = l1 ++ lm ++ l3 ∧
      ∃ K Δ', execS K D σ ρ (xs ++ .occ o N [] :: ys) = .ok (Δ', m1 ++ pre [.nm o] mm ++ m3, l1 ++ lm ++ l3) := by
  rw [List.append_assoc] at h
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (ms ++ ys) h
  obtain ⟨Δm, mm, lm, Δ3, m3, l3, hm, h3, e2⟩ := execS_append ms ys h2
  -- the moved block runs the same where it sees only the visible names
  have agm : AgreeOn (fvB ms ++ D.reads) (Δ1 ++ ρ) ([] ++ [] ++ only vis (Δ1 ++ ρ)) := by
    intro y hy
    simp only [List.nil_append, lookup_only]
    split
    · rfl
    · obtain ⟨hx, hρ⟩ := hin y hy ‹_›
      rw [lookup_append, lookup_none (fun hk => hx (execS_keys xs h1 y hk)), hρ]; rfl
  have hm' := (indep n).s hm agm
  have hocc : execOne (n + 2) D σ (Δ1 ++ ρ) (.occ o N []) = .ok ([(o, .frag mm)], pre [.nm o] mm, lm) := by
    simp only [execOne, hN]
    simp only [argsOK, List.all_nil, ite_true, dflts, List.filter_nil, evalA_nil, ok_bind]
    rw [execS_mono hm' (Nat.le_succ n)]
    simp
  -- the rest sees neither what moved nor the new name
  have agy : AgreeOn (fvB ys ++ D.reads) (Δm ++ (Δ1 ++ ρ)) ([(o, .frag mm)] ++ (Δ1 ++ ρ)) := by
    intro y hy
    rw [lookup_append (Δm), lookup_append [(o, Bind.frag mm)]]
    have h1' : Δm.lookup y = none := lookup_none fun hk => hout y (execS_keys ms hm y hk) hy
    have h2' : ([(o, Bind.frag mm)] : Env).lookup y = none := by
      simp only [List.lookup]
      have : (y == o) = false := by simp; intro e; subst e; exact ho hy
      simp [this]
    rw [h1', h2']
  have h3' := (indep n).s h3 agy
  obtain ⟨K1, hK1⟩ := execS_cons_compose hocc h3'
  obtain ⟨K, hK⟩ := execS_compose' h1 hK1
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  exact ⟨m1, mm, m3, l1, lm, l3, by simp, by simp, K, _, by simpa [List.append_assoc] using hK⟩

end ForgeLang
