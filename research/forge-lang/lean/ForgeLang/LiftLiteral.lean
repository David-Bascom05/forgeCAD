import ForgeLang.RenamePart

/-!
# LiftLiteral (`edit.rs`, `Op::LiftLiteral`): a literal becomes a named param

`LiftLiteral {span, name, scope: shared}` replaces the literal at `span` (site `p`) by a new name
`x0` and adds `shared x0 = v`, `v` the literal's number, so it gets a handle. In the core: every
`lit p` of the document's statements becomes `var x0` (`lfB`: a site occurs once), and
`let_ x0 (lit p)` goes first. Reading `x0` gives the trace `site p`, which is what `lit p` gave,
so nothing a document computes changes:

* `lift_nested` (`LiftN`): the lifted code, in an environment that also binds `x0` to
  `site p`, evaluates exactly as the original. Definitions are not touched (`indep`).
* `liftLiteral_body`: the document. If it evaluates, the lifted one evaluates (two more units of
  fuel: the new statement) to the same model and log, its bindings the old ones and `x0`. Every
  id, trace and observed site is as it was; the literal's site `p` is now the param's (the same
  site), so SetParam on it edits the same number (`Edits.setParam_roundtrip`).

Hypotheses (`precheck`'s `taken` and a fresh name): `x0` is not read or bound anywhere in the
document's statements, nor read by a definition. Not modelled: a `Local` scope (a param of one
feature, visible to it alone: with `x0` fresh, the same reads), the text splice.
-/

namespace ForgeLang

mutual
/-- `lit p` read as the name `x0`. -/
def lfE (x0 : Name) (p : Site) : Expr → Expr
  | .lit s => if s = p then .var x0 else .lit s
  | .cst c => .cst c
  | .var x => .var x
  | .add u v => .add (lfE x0 p u) (lfE x0 p v)
  | .mul u v => .mul (lfE x0 p u) (lfE x0 p v)
  | .bin o u v => .bin o (lfE x0 p u) (lfE x0 p v)
  | .eq u v => .eq (lfE x0 p u) (lfE x0 p v)
  | .lt u v => .lt (lfE x0 p u) (lfE x0 p v)
  | .ite c u v => .ite (lfE x0 p c) (lfE x0 p u) (lfE x0 p v)
  | .sel es k => .sel (lfEs x0 p es) (lfE x0 p k)
  | .call f u => .call f (lfE x0 p u)
  | .ref x rel => .ref x rel
  | .refIx x k rel => .refIx x (lfE x0 p k) rel
def lfEs (x0 : Name) (p : Site) : List Expr → List Expr
  | [] => []
  | e :: es => lfE x0 p e :: lfEs x0 p es
end

mutual
def lfS (x0 : Name) (p : Site) : Stmt → Stmt
  | .let_ x e => .let_ x (lfE x0 p e)
  | .set x e => .set x (lfE x0 p e)
  | .feat x e => .feat x (lfE x0 p e)
  | .forN x i c body => .forN x i (lfE x0 p c) (lfB x0 p body)
  | .forK x i kvs body => .forK x i (lfKV x0 p kvs) (lfB x0 p body)
  | .ifs c u v => .ifs (lfE x0 p c) (lfB x0 p u) (lfB x0 p v)
  | .use x q e => .use x q (lfE x0 p e)
  | .occ x c args => .occ x c (lfKV x0 p args)
def lfB (x0 : Name) (p : Site) : List Stmt → List Stmt
  | [] => []
  | s :: ss => lfS x0 p s :: lfB x0 p ss
def lfKV (x0 : Name) (p : Site) : List (Name × Expr) → List (Name × Expr)
  | [] => []
  | (k, e) :: kvs => (k, lfE x0 p e) :: lfKV x0 p kvs
end

/-- The lifted side binds `x0` to the literal's site and agrees on every other name. -/
def LRel (x0 : Name) (p : Site) (ρ ρ' : Env) : Prop :=
  ρ'.lookup x0 = some (.num (.site p)) ∧ ∀ y, y ≠ x0 → ρ'.lookup y = ρ.lookup y

theorem lrel_cons_keep {x0 : Name} {p : Site} {ρ ρ' : Env} (h : LRel x0 p ρ ρ') (k : Name) (B : Bind)
    (hk : k ≠ x0) : LRel x0 p ((k, B) :: ρ) ((k, B) :: ρ') := by
  refine ⟨?_, fun y hy => ?_⟩
  · have : (x0 == k) = false := by simpa using Ne.symm hk
    simp only [List.lookup, this]; exact h.1
  · by_cases e : y = k
    · subst e; simp [List.lookup]
    · have : (y == k) = false := by simpa using e
      simp only [List.lookup, this]; exact h.2 y hy

theorem lrel_append {x0 : Name} {p : Site} {ρ ρ' : Env} (h : LRel x0 p ρ ρ') :
    ∀ (Δ : Env), (∀ k ∈ keys Δ, k ≠ x0) → LRel x0 p (Δ ++ ρ) (Δ ++ ρ')
  | [], _ => h
  | (k, B) :: Δ, hk =>
    lrel_cons_keep (lrel_append h Δ fun y hy => hk y (by simp [keys] at hy ⊢; exact .inr hy)) k B
      (hk k (by simp [keys]))

theorem lrel_agree {x0 : Name} {p : Site} {ρ ρ' : Env} (h : LRel x0 p ρ ρ') {N : List Name}
    (hN : ∀ y ∈ N, y ≠ x0) : AgreeOn N ρ ρ' := fun y hy => (h.2 y (hN y hy)).symm

theorem lookupNum_lf {x0 : Name} {p : Site} {ρ ρ' : Env} (h : LRel x0 p ρ ρ') {x : Name} (hx : x ≠ x0)
    {t : Tr} (ht : lookupNum ρ x = .ok t) : lookupNum ρ' x = .ok t := by
  unfold lookupNum at ht ⊢; rw [h.2 x hx]; exact ht

theorem lookupRef_lf {x0 : Name} {p : Site} {ρ ρ' : Env} (h : LRel x0 p ρ ρ') {x : Name} (hx : x ≠ x0)
    {rel : FId} {t : Tr} (ht : lookupRef ρ x rel = .ok t) : lookupRef ρ' x rel = .ok t := by
  unfold lookupRef at ht ⊢; rw [h.2 x hx]; exact ht

theorem ldefs_agree {x0 : Name} {p : Site} {D : Defs} {ρ ρ' : Env} (hD : x0 ∉ D.reads)
    (h : LRel x0 p ρ ρ') : AgreeOn D.reads ρ ρ' :=
  lrel_agree h fun y hy e => hD (e ▸ hy)

theorem lfEs_getElem? {x0 : Name} {p : Site} : ∀ (es : List Expr) (i : Nat),
    (lfEs x0 p es)[i]? = (es[i]?).map (lfE x0 p)
  | [], i => by simp [lfEs]
  | e :: es, 0 => by simp [lfEs]
  | e :: es, i + 1 => by simp [lfEs, lfEs_getElem? es i]

theorem lfKV_lookup {x0 : Name} {p : Site} : ∀ (kvs : List (Name × Expr)) (k : Name),
    (lfKV x0 p kvs).lookup k = (kvs.lookup k).map (lfE x0 p)
  | [], k => by simp [lfKV, List.lookup]
  | (k', e) :: kvs, k => by
    simp only [lfKV, List.lookup]
    split <;> simp [lfKV_lookup kvs k]

theorem argsOK_lfKV {x0 : Name} {p : Site} (ps : List (Name × Expr)) :
    ∀ (args : List (Name × Expr)), argsOK ps (lfKV x0 p args) = argsOK ps args
  | [] => rfl
  | (k, e) :: args => by
    have := argsOK_lfKV (x0 := x0) (p := p) ps args
    simp only [argsOK, lfKV, List.all_cons] at this ⊢
    rw [this]

theorem dflts_lfKV {x0 : Name} {p : Site} (ps : List (Name × Expr)) (args : List (Name × Expr)) :
    dflts ps (lfKV x0 p args) = dflts ps args := by
  unfold dflts; congr 1; funext q; rw [lfKV_lookup]; cases args.lookup q.1 <;> rfl

structure LiftN (x0 : Name) (p : Site) (n : Nat) : Prop where
  e : ∀ {D σ ρ ρ' x r}, x0 ∉ D.reads → evalE n D σ ρ x = .ok r → LRel x0 p ρ ρ' → x0 ∉ fvE x →
    evalE n D σ ρ' (lfE x0 p x) = .ok r
  one : ∀ {D σ ρ ρ' x r}, x0 ∉ D.reads → execOne n D σ ρ x = .ok r → LRel x0 p ρ ρ' → NoBindS x0 x0 x →
    x0 ∉ fvS x → execOne n D σ ρ' (lfS x0 p x) = .ok r
  s : ∀ {D σ ρ ρ' x r}, x0 ∉ D.reads → execS n D σ ρ x = .ok r → LRel x0 p ρ ρ' → NoBindB x0 x0 x →
    x0 ∉ fvB x → execS n D σ ρ' (lfB x0 p x) = .ok r
  ln : ∀ {D σ ρ ρ' i body ks r}, x0 ∉ D.reads → loopN n D σ ρ i body ks = .ok r → LRel x0 p ρ ρ' →
    i ≠ x0 → i ≠ x0 → NoBindB x0 x0 body → x0 ∉ fvB body → loopN n D σ ρ' i (lfB x0 p body) ks = .ok r
  lk : ∀ {D σ ρ ρ' i body kvs r}, x0 ∉ D.reads → loopK n D σ ρ i body kvs = .ok r → LRel x0 p ρ ρ' →
    i ≠ x0 → i ≠ x0 → NoBindB x0 x0 body → x0 ∉ fvKV kvs → x0 ∉ fvB body →
    loopK n D σ ρ' i (lfB x0 p body) (lfKV x0 p kvs) = .ok r
  as : ∀ {D σ ρ ρ' as r}, x0 ∉ D.reads → evalA n D σ ρ as = .ok r → LRel x0 p ρ ρ' → x0 ∉ fvKV as →
    evalA n D σ ρ' (lfKV x0 p as) = .ok r

theorem lift_nested (x0 : Name) (p : Site) : ∀ n, LiftN x0 p n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D σ ρ ρ' x r hD h hr hb
      cases x with
      | lit s =>
        simp only [evalE, Except.ok.injEq] at h
        subst h
        by_cases hs : s = p
        · subst hs
          cases n with
          | zero => simp [lfE, evalE, lookupNum, hr.1]
          | succ n => simp [lfE, evalE, lookupNum, hr.1]
        · simp [lfE, hs, evalE]
      | cst c => simpa [evalE, lfE] using h
      | var x =>
        have hx : x ≠ x0 := fun e => hb (by simp [fvE, e])
        simp only [evalE, lfE, bind_ok] at h ⊢
        obtain ⟨t, ht, h⟩ := h
        exact ⟨t, lookupNum_lf hr hx ht, h⟩
      | ref x rel =>
        have hx : x ≠ x0 := fun e => hb (by simp [fvE, e])
        simp only [evalE, lfE, bind_ok] at h ⊢
        obtain ⟨t, ht, h⟩ := h
        exact ⟨t, lookupRef_lf hr hx ht, h⟩
      | add u v | mul u v | eq u v | lt u v | bin o u v =>
        simp only [fvE, List.mem_append, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb', h⟩ := h
        cases h
        simp [evalE, lfE, ih.e hD ha hr hb.1, ih.e hD hb' hr hb.2]
      | ite c u v =>
        simp only [fvE, List.mem_append, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hu, h⟩ := h
        cases h
        have hc' := ih.e hD hc hr hb.1.1
        have hu' : evalE n D σ ρ' (if tc.den σ ≠ 0 then lfE x0 p u else lfE x0 p v) = .ok (t, l) := by
          split
          · rename_i hne; rw [if_pos hne] at hu; exact ih.e hD hu hr hb.1.2
          · rename_i hne; rw [if_neg hne] at hu; exact ih.e hD hu hr hb.2
        simp only [evalE, lfE, hc', ok_bind, hu']
      | sel es k =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        simp only [fvE, List.mem_append, not_or] at hb
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, he', h⟩ := h
          cases h
          have hsub := fvE_sub_fvEs (List.mem_of_getElem? he)
          have he'' := ih.e hD he' hr (fun hm => hb.1 (hsub x0 hm))
          simp [evalE, lfE, ih.e hD hk hr hb.2, hi, lfEs_getElem?, he, he'']
        · cases h
      | call f u =>
        simp only [fvE] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i x body hf
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, hbd, h⟩ := h
          cases h
          simp [evalE, lfE, ih.e hD ha hr hb, hf,
            (indep n).e hbd (agree_body (fn_reads hf) (ldefs_agree hD hr))]
        · cases h
      | refIx x k rel =>
        simp only [fvE, List.mem_cons, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        have ht' := lookupRef_lf hr (Ne.symm hb.1) ht
        simp [evalE, lfE, ih.e hD hk hr hb.2, hi, ht']
    · intro D σ ρ ρ' x r hD h hr hn hb
      cases x with
      | let_ x e | feat x e =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, lfS, ih.e hD he hr hb]
      | set x e =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_cons, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        have hu' := lookupNum_lf hr (Ne.symm hb.1) hu
        simp [execOne, lfS, hu', ih.e hD he hr hb.2]
      | forN x i c body =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, lfS, ih.e hD hc hr hb.1, hk,
          ih.ln hD hl hr hn.2.2.1 hn.2.2.2.1 hn.2.2.2.2 hb.2]
      | forK x i kvs body =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, lfS, ih.lk hD hl hr hn.2.2.1 hn.2.2.2.1 hn.2.2.2.2 hb.1 hb.2]
      | ifs c u v =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hbr, h⟩ := h
        cases h
        have hc' := ih.e hD hc hr hb.1.1
        have hb' : execS n D σ ρ' (if tc.den σ ≠ 0 then lfB x0 p u else lfB x0 p v) = .ok (ρ1, mm, l) := by
          split
          · rename_i hne; rw [if_pos hne] at hbr; exact ih.s hD hbr hr hn.1 hb.1.2
          · rename_i hne; rw [if_neg hne] at hbr; exact ih.s hD hbr hr hn.2 hb.2
        simp only [execOne, lfS, hc', ok_bind, hb']
      | use x p e =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l⟩, hbd, h⟩ := h
          cases h
          simp [execOne, lfS, ih.e hD ha hr hb, hp,
            (indep n).s hbd (agree_body (proc_reads hp) (ldefs_agree hD hr))]
        · cases h
      | occ x c args =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_ok] at h
            obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l⟩, hbd, h⟩ := h
            cases h
            simp only [List.append_assoc] at hbd
            have cr := comp_reads hC
            have ag := ldefs_agree hD hr
            have ad : AgreeOn (fvKV (dflts C.params args) ++ D.reads) (only C.vis ρ) (only C.vis ρ') :=
              agree_only ag fun y hy hv => by
                rcases List.mem_append.1 hy with hy | hy
                · exact cr y (List.mem_append_right _ (fvKV_filter hy)) hv
                · exact hy
            have ab : AgreeOn (fvB C.body ++ D.reads) (Δg ++ (Δd ++ only C.vis ρ)) (Δg ++ (Δd ++ only C.vis ρ')) :=
              ((agree_only ag fun y hy hv => by
                rcases List.mem_append.1 hy with hy | hy
                · exact cr y (List.mem_append_left _ hy) hv
                · exact hy).append Δd).append Δg
            simp [execOne, lfS, hC, argsOK_lfKV, hok, dflts_lfKV, ih.as hD hg hr hb,
              (indep n).a hd ad, (indep n).s hbd ab]
          · cases h
        · cases h
    · intro D σ ρ ρ' x r hD h hr hn hb
      cases x with
      | nil => simpa [execS, lfB] using h
      | cons s ss =>
        simp only [NoBindB] at hn
        simp only [fvB, List.mem_append, not_or] at hb
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        have hk : ∀ k ∈ keys Δ1, k ≠ x0 ∧ k ≠ x0 := fun k hk' =>
          binder_noBind hn.1 (execOne_keys h1 k hk')
        have hr' : LRel x0 p (Δ1 ++ ρ) (Δ1 ++ ρ') := lrel_append hr Δ1 fun k hk' => (hk k hk').1
        simp [execS, lfB, ih.one hD h1 hr hn.1 hb.1, ih.s hD h2 hr' hn.2 hb.2]
    · intro D σ ρ ρ' i body ks r hD h hr hia hib hn hb
      cases ks with
      | nil => simpa [loopN] using h
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopN, ih.s hD h1 (lrel_cons_keep hr _ _ hia) hn hb,
          ih.ln hD h2 hr hia hib hn hb]
    · intro D σ ρ ρ' i body kvs r hD h hr hia hib hn hkv hb
      cases kvs with
      | nil => simpa [loopK, lfKV] using h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [fvKV, List.mem_append, not_or] at hkv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopK, lfKV, ih.e hD h0 hr hkv.1, ih.s hD h1 (lrel_cons_keep hr _ _ hia) hn hb,
          ih.lk hD h2 hr hia hib hn hkv.2 hb]
    · intro D σ ρ ρ' as r hD h hr hb
      cases as with
      | nil => simpa [evalA, lfKV] using h
      | cons a' as =>
        obtain ⟨y, e⟩ := a'
        simp only [fvKV, List.mem_append, not_or] at hb
        simp only [evalA, bind_ok] at h
        obtain ⟨⟨t, l⟩, h0, ⟨Δ, l'⟩, h1, h⟩ := h
        cases h
        simp [evalA, lfKV, ih.e hD h0 hr hb.1, ih.as hD h1 hr hb.2]


/-- LiftLiteral, shared scope: if the document evaluates, the lifted one (`x0 = lit p` first, every
`lit p` read as `x0`) evaluates to the same model and log; its bindings are the old ones and `x0`. -/
theorem liftLiteral_body {n : Nat} {D : Defs} {σ : Site → Int} {body : List Stmt} {Δ : Env}
    {m : Model} {l : Log} (x0 : Name) (p : Site) (hD : x0 ∉ D.reads) (hn : NoBindB x0 x0 body)
    (hb : x0 ∉ fvB body) (h : execS n D σ [] body = .ok (Δ, m, l)) :
    execS (n + 2) D σ [] (.let_ x0 (.lit p) :: lfB x0 p body) =
      .ok (Δ ++ [(x0, .num (.site p))], m, l) := by
  cases n with
  | zero => simp [execS] at h
  | succ n =>
    have h' := execS_mono h (Nat.le_succ (n + 1))
    have hr : LRel x0 p ([] : Env) [(x0, .num (.site p))] := ⟨by simp [List.lookup], fun y hy => by
      have : (y == x0) = false := by simpa using hy
      simp [List.lookup, this]⟩
    have hl := (lift_nested x0 p (n + 2)).s hD h' hr hn hb
    simp [execS, execOne, evalE, hl]

end ForgeLang
