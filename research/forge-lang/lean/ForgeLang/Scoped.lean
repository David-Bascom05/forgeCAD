import ForgeLang.Refactor
import ForgeLang.Print

/-!
# Scoped edit ops: the same ops, applied to a component's block

The design gives every existing op scope by id prefix: `cup.body` is statement `body` of block
`cup`, and the op is the same function applied to the block's statements. Here:

* `congr`: changing one component's definition changes nothing that does not place it. The
  hypothesis `NoPlace` (no procedure and no component body places it) is the design's
  `NestedComponent` rule.
* `scoped_occ`: so an occurrence of the edited component evaluates exactly as the occurrence of a
  component whose block *is* the edited list. Every block op is the unscoped op on the block.
* `scopedDelete_ok`, `scopedAdd_ok`: DeleteFeature `{id: "cup.x"}` and AddFeature
  `{before: "cup.y" | "cup."}` as functions that either refuse with a typed reason or return the
  edited definitions; when they return, every occurrence of `cup` that evaluated before
  evaluates after, to its old model with the fragment removed (or added) under that occurrence.
* `addParam_occ`: AddComponentParam with a literal default leaves every occurrence's result
  unchanged, as long as nothing reads the new name.
* `scoped_text_insert`: the printed document changes by exactly the inserted statement's tokens
  inside the block. With P6 (`LL1`), the text parses back to the edited document.

SetArg on an occurrence's literal argument and SetArg on a definition's literal default are
SetParam on that literal's site: `setParam_roundtrip` covers both (a default's site is in
`D.code`, an argument's in the body), with its condition that the site is not observed (a count,
a branch); otherwise it is a re-run.
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

def Defs.withComp (D : Defs) (c : Name) (C : Comp) : Defs := { D with comps := (c, C) :: D.comps }

@[simp] theorem withComp_fns (D : Defs) (c : Name) (C : Comp) : (D.withComp c C).fns = D.fns := rfl
@[simp] theorem withComp_procs (D : Defs) (c : Name) (C : Comp) : (D.withComp c C).procs = D.procs := rfl
@[simp] theorem obs_withComp (D : Defs) (c : Name) (C : Comp) (t : Tr) : obs (D.withComp c C) t = obs D t := rfl

theorem withComp_lookup_self (D : Defs) (c : Name) (C : Comp) : (D.withComp c C).comps.lookup c = some C := by
  simp [Defs.withComp, List.lookup]

theorem withComp_lookup_other (D : Defs) {c c' : Name} (C : Comp) (h : c' ≠ c) :
    (D.withComp c C).comps.lookup c' = D.comps.lookup c' := by
  have : (c' == c) = false := by simp [h]
  simp [Defs.withComp, List.lookup, this]

mutual
/-- The components a statement places. -/
def placesS : Stmt → List Name
  | .occ _ c _ => [c]
  | .forN _ _ _ b | .forK _ _ _ b => placesB b
  | .ifs _ a b => placesB a ++ placesB b
  | _ => []
def placesB : List Stmt → List Name
  | [] => []
  | s :: ss => placesS s ++ placesB ss
end

/-- No procedure or component body places `c0`. -/
def NoPlace (c0 : Name) (D : Defs) : Prop :=
  (∀ p ∈ D.procs, c0 ∉ placesB p.2.2) ∧ (∀ c ∈ D.comps, c0 ∉ placesB c.2.body)

structure CongrAt (D : Defs) (c0 : Name) (C' : Comp) (n : Nat) : Prop where
  e : ∀ {σ ρ x}, evalE n (D.withComp c0 C') σ ρ x = evalE n D σ ρ x
  one : ∀ {σ ρ x}, c0 ∉ placesS x → execOne n (D.withComp c0 C') σ ρ x = execOne n D σ ρ x
  s : ∀ {σ ρ x}, c0 ∉ placesB x → execS n (D.withComp c0 C') σ ρ x = execS n D σ ρ x
  ln : ∀ {σ ρ i b ks}, c0 ∉ placesB b → loopN n (D.withComp c0 C') σ ρ i b ks = loopN n D σ ρ i b ks
  lk : ∀ {σ ρ i b kvs}, c0 ∉ placesB b → loopK n (D.withComp c0 C') σ ρ i b kvs = loopK n D σ ρ i b kvs
  a : ∀ {σ ρ as}, evalA n (D.withComp c0 C') σ ρ as = evalA n D σ ρ as

theorem congr {D : Defs} {c0 : Name} {C' : Comp} (hD : NoPlace c0 D) : ∀ n, CongrAt D c0 C' n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro σ ρ x
      cases x <;> simp only [evalE, ih.e, obs_withComp, withComp_fns]
    · intro σ ρ x hp
      cases x with
      | let_ | set | feat => simp only [execOne, ih.e]
      | forN x i c b =>
        simp only [placesS] at hp
        simp only [execOne, ih.e, ih.ln hp, obs_withComp]
      | forK x i kvs b =>
        simp only [placesS] at hp
        simp only [execOne, ih.lk hp]
      | ifs c a b =>
        simp only [placesS, List.mem_append, not_or] at hp
        simp only [execOne, ih.e, obs_withComp]
        cases evalE n D σ ρ c with
        | error _ => rfl
        | ok pc =>
          simp only [ok_bind]
          split
          · rw [ih.s hp.1]
          · rw [ih.s hp.2]
      | use x p a =>
        simp only [execOne, ih.e, withComp_procs]
        cases evalE n D σ ρ a with
        | error _ => rfl
        | ok pa =>
          simp only [ok_bind]
          cases hq : D.procs.lookup p with
          | none => rfl
          | some yb =>
            obtain ⟨y, body⟩ := yb
            simp only
            rw [ih.s (hD.1 (p, y, body) (lookup_mem hq))]
      | occ x c args =>
        simp only [placesS, List.mem_singleton] at hp
        simp only [execOne, withComp_lookup_other D C' (Ne.symm hp)]
        cases hc : D.comps.lookup c with
        | none => rfl
        | some C =>
          simp only
          split
          · have hb := hD.2 (c, C) (lookup_mem hc)
            simp only at hb
            simp only [ih.a, ih.s hb]
          · rfl
    · intro σ ρ x hp
      cases x with
      | nil => rfl
      | cons s ss =>
        simp only [placesB, List.mem_append, not_or] at hp
        simp only [execS, ih.one hp.1, ih.s hp.2]
    · intro σ ρ i b ks hp
      cases ks with
      | nil => rfl
      | cons k ks => simp only [loopN, ih.s hp, ih.ln hp]
    · intro σ ρ i b kvs hp
      cases kvs with
      | nil => rfl
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK, ih.e, ih.s hp, ih.lk hp]
    · intro σ ρ as
      cases as with
      | nil => rfl
      | cons a as =>
        obtain ⟨y, e⟩ := a
        simp only [evalA, ih.e, ih.a]

/-- What an occurrence of a component `C` does, given its definition. -/
def occRun (n : Nat) (D : Defs) (σ : Site → Int) (ρ : Env) (x c : Name) (C : Comp)
    (args : List (Name × Expr)) : R (Env × Model × Log) :=
  if argsOK C.params args then do
    let (Δg, lg) ← evalA n D σ ρ args
    let (Δd, ld) ← evalA n D σ (only C.vis ρ) (dflts C.params args)
    let (_, m, l) ← execS n D σ (Δg ++ Δd ++ only C.vis ρ) C.body
    .ok ([(x, .frag m)], pre [.nm x] m, lg ++ ld ++ l)
  else .error (.noParam c)

theorem execOne_occ {n D σ ρ x c args C} (h : D.comps.lookup c = some C) :
    execOne (n + 1) D σ ρ (.occ x c args) = occRun n D σ ρ x c C args := by
  simp [execOne, h, occRun]

/-- An occurrence of an edited component is the occurrence of the edited block, evaluated with
the old definitions: every scoped op is the unscoped op on the block's statement list. -/
theorem scoped_occ {n D σ ρ x c0 args} {C' : Comp} (hD : NoPlace c0 D) (hb : c0 ∉ placesB C'.body) :
    execOne (n + 1) (D.withComp c0 C') σ ρ (.occ x c0 args) = occRun n D σ ρ x c0 C' args := by
  rw [execOne_occ (withComp_lookup_self D c0 C')]
  simp only [occRun, (congr hD n).a, (congr hD n).s hb]

/-! ## Ops as functions: refusal or the edited definitions -/

inductive Refusal where
  | noComp (c : Name)
  | noFeature (c x : Name)
  | stillRead (c x : Name)
  | taken (c x : Name)
  deriving DecidableEq, Repr

/-- The statement named `nm` in a block, with what is before and after it. -/
def findSplit (nm : Name) : List Stmt → Option (List Stmt × Stmt × List Stmt)
  | [] => none
  | s :: ss =>
    if s.binder = some nm then some ([], s, ss)
    else (findSplit nm ss).map fun (xs, t, ys) => (s :: xs, t, ys)

theorem findSplit_spec {nm : Name} : ∀ {b : List Stmt} {xs s ys}, findSplit nm b = some (xs, s, ys) →
    b = xs ++ s :: ys ∧ s.binder = some nm
  | [], _, _, _, h => by simp [findSplit] at h
  | t :: ts, xs, s, ys, h => by
    simp only [findSplit] at h
    split at h
    · cases h; exact ⟨rfl, ‹_›⟩
    · cases hf : findSplit nm ts with
      | none => rw [hf] at h; cases h
      | some r =>
        obtain ⟨xs', s', ys'⟩ := r
        rw [hf] at h
        simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        obtain ⟨e, hb⟩ := findSplit_spec hf
        exact ⟨by simp [e], hb⟩

/-- DeleteFeature `{id: "c.nm"}`. Refused when the block has no such statement, or when a later
statement of the block or a function reads it (`NotRead`, as unscoped). -/
def scopedDelete (D : Defs) (c nm : Name) : Except Refusal Defs :=
  match D.comps.lookup c with
  | none => .error (.noComp c)
  | some C =>
    match findSplit nm C.body with
    | none => .error (.noFeature c nm)
    | some (xs, _, ys) =>
      if nm ∈ fvB ys ++ D.reads then .error (.stillRead c nm)
      else .ok (D.withComp c { C with body := xs ++ ys })

theorem placesB_append (xs ys : List Stmt) : placesB (xs ++ ys) = placesB xs ++ placesB ys := by
  induction xs with
  | nil => rfl
  | cons s xs ih => simp [placesB, ih, List.append_assoc]

/-- DeleteFeature in a block, all-or-nothing: if the op returns, every occurrence of the block
that evaluated still evaluates, to its old fragment without the deleted statement's part. -/
theorem scopedDelete_ok {D D' : Defs} {c nm : Name} (hD : NoPlace c D) (hop : scopedDelete D c nm = .ok D')
    {n σ ρ x args Δ m l} (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l)) :
    ∃ C xs s ys, D.comps.lookup c = some C ∧ C.body = xs ++ s :: ys ∧ s.binder = some nm ∧
      ∃ mi m1 ms m3 li l1 ls l3, m = pre [.nm x] mi ∧ mi = m1 ++ ms ++ m3 ∧ l = li ++ (l1 ++ ls ++ l3) ∧
        ∃ k, execOne k D' σ ρ (.occ x c args) = .ok ([(x, .frag (m1 ++ m3))], pre [.nm x] (m1 ++ m3), li ++ (l1 ++ l3)) := by
  unfold scopedDelete at hop
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
      split at hop
      · cases hop
      · rename_i hnr
        cases hop
        obtain ⟨hbody, hbind⟩ := findSplit_spec hs
        refine ⟨C, xs, s, ys, rfl, hbody, hbind, ?_⟩
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
            rw [hbody] at hb
            obtain ⟨m1, ms, m3, l1, ls, l3, rfl, rfl, K, Δ', hK⟩ :=
              delete_complete (s := s) (fun y hy => by rw [hbind] at hy; cases hy; exact hnr) hb
            have pb : c ∉ placesB (xs ++ ys) := by
              have := hD.2 (c, C) (lookup_mem hc)
              simp only [hbody, placesB_append, placesB, List.mem_append, not_or] at this ⊢
              exact ⟨this.1, this.2.2⟩
            refine ⟨m1 ++ ms ++ m3, m1, ms, m3, lg ++ ld, l1, ls, l3, rfl, rfl, by simp, max n K + 1, ?_⟩
            rw [scoped_occ (C' := { C with body := xs ++ ys }) hD pb]
            simp only [occRun, hok, ite_true, evalA_mono hg (Nat.le_max_left n K),
              evalA_mono hd (Nat.le_max_left n K), ok_bind]
            rw [execS_mono hK (Nat.le_max_right n K)]
            simp
          · cases h

/-- AddFeature `{before: "c.nm"}` or `{before: "c."}` (`nm = none`: the block's end). Refused
when the block or the target does not exist, or when the new statement's name is already
mentioned after it or by a function (it would capture a read). -/
def addSplit (b : List Stmt) : Option Name → Option (List Stmt × List Stmt)
  | none => some (b, [])
  | some nm => (findSplit nm b).map fun r => (r.1, r.2.1 :: r.2.2)

theorem addSplit_spec {b : List Stmt} {before : Option Name} {xs ys : List Stmt}
    (h : addSplit b before = some (xs, ys)) : b = xs ++ ys := by
  cases before with
  | none => simp [addSplit] at h; obtain ⟨rfl, rfl⟩ := h; simp
  | some nm =>
    simp only [addSplit] at h
    cases hf : findSplit nm b with
    | none => rw [hf] at h; cases h
    | some r =>
      obtain ⟨xs', t, ys'⟩ := r
      rw [hf] at h
      simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact (findSplit_spec hf).1

def scopedAdd (D : Defs) (c : Name) (before : Option Name) (s : Stmt) : Except Refusal Defs :=
  match D.comps.lookup c with
  | none => .error (.noComp c)
  | some C =>
    match addSplit C.body before with
    | none => .error (.noFeature c (before.getD ""))
    | some (xs, ys) =>
      match s.binder with
      | some b => if b ∈ fvB ys ++ D.reads then .error (.taken c b)
                  else .ok (D.withComp c { C with body := xs ++ s :: ys })
      | none => .ok (D.withComp c { C with body := xs ++ s :: ys })

/-- AddFeature in a block, all-or-nothing: if the op returns and the new statement evaluates where
it is inserted, every occurrence that evaluated still evaluates, to its old fragment with the new
statement's part in place. -/
theorem scopedAdd_ok {D D' : Defs} {c : Name} {before : Option Name} {s : Stmt}
    (hD : NoPlace c D) (hs : c ∉ placesS s) (hop : scopedAdd D c before s = .ok D')
    {n σ ρ x args Δ m l} (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l)) :
    ∃ C xs ys, D.comps.lookup c = some C ∧ C.body = xs ++ ys ∧
      D' = D.withComp c { C with body := xs ++ s :: ys } ∧
      ∃ n' I mi li lb, m = pre [.nm x] mi ∧ l = li ++ lb ∧
        ((∀ Δ1 m1 l1, execS n' D σ I xs = .ok (Δ1, m1, l1) → ∃ k r, execOne k D σ (Δ1 ++ I) s = .ok r) →
          ∃ m1 ms m3 l1 ls l3, mi = m1 ++ m3 ∧ lb = l1 ++ l3 ∧
            ∃ k, execOne k D' σ ρ (.occ x c args) = .ok ([(x, .frag (m1 ++ ms ++ m3))],
              pre [.nm x] (m1 ++ ms ++ m3), li ++ (l1 ++ ls ++ l3))) := by
  unfold scopedAdd at hop
  cases hc : D.comps.lookup c with
  | none => rw [hc] at hop; cases hop
  | some C =>
    rw [hc] at hop
    simp only at hop
    cases hsp : addSplit C.body before with
    | none => rw [hsp] at hop; cases hop
    | some r =>
    obtain ⟨xs, ys⟩ := r
    have hbody := addSplit_spec hsp
    rw [hsp] at hop
    simp only at hop
    have hnr : NotRead s (fvB ys ++ D.reads) := by
      intro b hb
      rw [hb] at hop
      simp only at hop
      split at hop
      · cases hop
      · assumption
    have hD' : D' = D.withComp c { C with body := xs ++ s :: ys } := by
      cases hb : s.binder with
      | none => rw [hb] at hop; cases hop; rfl
      | some b =>
        rw [hb] at hop
        simp only at hop
        split at hop
        · cases hop
        · cases hop; rfl
    refine ⟨C, xs, ys, rfl, hbody, hD', ?_⟩
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
        simp only [List.append_assoc] at hb
        refine ⟨n, Δg ++ (Δd ++ only C.vis ρ), mm, lg ++ ld, lb, rfl, by simp, fun hev => ?_⟩
        rw [hbody] at hb
        obtain ⟨m1, ms, m3, l1, ls, l3, rfl, rfl, K, Δ', hK⟩ := add_complete hnr hb hev
        have pb : c ∉ placesB (xs ++ s :: ys) := by
          have := hD.2 (c, C) (lookup_mem hc)
          simp only [hbody, placesB_append, placesB, List.mem_append, not_or] at this ⊢
          exact ⟨this.1, hs, this.2⟩
        refine ⟨m1, ms, m3, l1, ls, l3, rfl, rfl, max n K + 1, ?_⟩
        rw [hD', scoped_occ (C' := { C with body := xs ++ s :: ys }) hD pb]
        simp only [occRun, hok, ite_true, evalA_mono hg (Nat.le_max_left n K),
          evalA_mono hd (Nat.le_max_left n K), ok_bind, List.append_assoc]
        rw [execS_mono hK (Nat.le_max_right n K)]
        simp [List.append_assoc]
      · cases h

/-! ## AddComponentParam -/

/-- AddComponentParam `y` with a literal default: an occurrence that evaluated gives the same
result, provided no call names `y` yet and nothing reads it. -/
theorem addParam_occ {D : Defs} {c y : Name} {q : Site} {C : Comp} (hD : NoPlace c D)
    (hc : D.comps.lookup c = some C) (hy : y ∉ C.params.map Prod.fst) (hr : y ∉ fvB C.body ++ D.reads)
    {n σ ρ x args r} (h : execOne n D σ ρ (.occ x c args) = .ok r) :
    ∃ k, execOne k (D.withComp c { C with params := C.params ++ [(y, .lit q)] }) σ ρ (.occ x c args) = .ok r := by
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
      have names := argsOK_names hok
      have hya : args.lookup y = none := by
        cases hl : args.lookup y with
        | none => rfl
        | some e => exact absurd (names (y, e) (lookup_mem hl)) hy
      have hok' : argsOK (C.params ++ [(y, .lit q)]) args = true := by
        unfold argsOK at hok ⊢
        rw [List.all_eq_true] at hok ⊢
        intro a ha
        have := hok a ha
        rw [List.lookup_append]
        cases hp : C.params.lookup a.1 <;> simp_all [Option.or]
      have hdf : dflts (C.params ++ [(y, .lit q)]) args = dflts C.params args ++ [(y, .lit q)] := by
        simp [dflts, List.filter_append, hya]
      have hd' : evalA (n + 2) D σ (only C.vis ρ) (dflts C.params args ++ [(y, .lit q)]) =
          .ok (Δd ++ [(y, .num (.site q))], ld ++ []) :=
        evalA_app _ _ hd (m := 2) (by simp [evalA, evalE])
      have pb : c ∉ placesB C.body := hD.2 (c, C) (lookup_mem hc)
      have ag : AgreeOn (fvB C.body ++ D.reads) (Δg ++ Δd ++ only C.vis ρ)
          (Δg ++ (Δd ++ [(y, .num (.site q))]) ++ only C.vis ρ) := by
        intro z hz
        have hzy : z ≠ y := fun e => hr (e ▸ hz)
        simp only [List.append_assoc, lookup_append]
        congr 1; congr 1
        have : ([(y, Bind.num (.site q))] : Env).lookup z = none := by
          have e : (z == y) = false := by simp [hzy]
          simp [List.lookup, e]
        rw [this]
        cases Δd.lookup z <;> rfl
      have hb' := (indep n).s hb ag
      refine ⟨n + 2 + 1, ?_⟩
      rw [scoped_occ (C' := { C with params := C.params ++ [(y, .lit q)] }) hD pb]
      simp only [occRun, hok', ite_true, evalA_mono hg (by omega : n ≤ n + 2), hdf, hd', ok_bind]
      rw [execS_mono hb' (by omega : n ≤ n + 2)]
      simp
    · cases h

/-! ## Text -/

/-- A component block as printed: `component c(ps) { body }`. -/
def skelComp (c : Name) (C : Comp) : List Tok :=
  [.sym "component", .id c, .sym "("] ++ skelKV C.params ++ [.sym ")", .sym "{"] ++ skelB C.body ++ [.sym "}"]

/-- A document with its component blocks, then its body. -/
def docText (σ : Site → Int) (comps : List (Name × Comp)) (body : List Stmt) : List Tok :=
  ((comps.flatMap fun c => skelComp c.1 c.2) ++ skelB body).map (render σ)

/-- AddFeature in block `c`: the text changes by the new statement's tokens at that place, and
nowhere else. -/
theorem scoped_text_insert (σ : Site → Int) (pre post : List (Name × Comp)) (c : Name) (C : Comp)
    (xs ys body : List Stmt) (s : Stmt) (hb : C.body = xs ++ ys) :
    ∃ A B, docText σ (pre ++ (c, C) :: post) body = A ++ B ∧
      docText σ (pre ++ (c, { C with body := xs ++ s :: ys }) :: post) body = A ++ (skelS s).map (render σ) ++ B := by
  refine ⟨((pre.flatMap fun (c : Name × Comp) => skelComp c.1 c.2) ++ ([Tok.sym "component", Tok.id c, Tok.sym "("] : List Tok) ++
      skelKV C.params ++ ([Tok.sym ")", Tok.sym "{"] : List Tok) ++ skelB xs).map (render σ),
    ((skelB ys ++ ([Tok.sym "}"] : List Tok)) ++ (post.flatMap fun (c : Name × Comp) => skelComp c.1 c.2) ++
      skelB body).map (render σ), ?_, ?_⟩
  · simp [docText, skelComp, hb, skelB_append, List.append_assoc]
  · simp [docText, skelComp, skelB_append, skelB, List.append_assoc]

end ForgeLang
