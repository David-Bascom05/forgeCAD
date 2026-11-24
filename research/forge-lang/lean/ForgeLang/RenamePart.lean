import ForgeLang.Fv
import ForgeLang.Struct
import ForgeLang.Weak

/-!
# Rename in a part document (`edit_ops::apply_rename`)

Rename `{from: a, to: b}` rewrites the binding `a` and every read of it (a name, the base of a
reference `a.rel` or `a[k].rel`) to `b`, and nothing else: not a member after `.`, not a keyword
argument's label, not a loop's label, not a function's name (`rnE`, `rnS`). In the core: a
top-level statement binds `a`; feature ids come from names (P5), so `a`'s features change id from
`a...` to `b...`, and nothing else may change.

* `ren_nested` (`RenN`): code that binds neither name (a loop's body, a branch inside one) runs
  exactly as before with its reads renamed, in an environment that reads `rn x` as the old one
  reads `x`. Function, procedure and component bodies are not touched by the op and read neither
  name (`D.reads`), so the free-name lemma (`indep`) carries them.
* `ren_top` (`RenTop`): the document's statements, where `a` is bound, run to the same traces and
  log, every binding renamed (`rnEnv`) and every feature's id renamed in its first segment
  (`rnModel`: `nm a` to `nm b`, all else as it was).
* `rename_body`: the document. Exact, at the same fuel: no structure changes.

Hypotheses, what `precheck_rename` and an accepted document give: `b` is fresh (not read, not
bound at top level: `NameTaken`), no function, procedure or component body reads `a` or `b`
(`D.reads`; Rust renames inside function bodies too, which this does not model), and neither name
is bound inside a loop's body or is a loop variable (`TopB`; Rust's `bound` lists a loop body's
names at top level, where the core nests them under the loop's id).
-/

namespace ForgeLang

/-- `a` read as `b`. -/
def rn (a b z : Name) : Name := if z = a then b else z

mutual
def rnE (a b : Name) : Expr → Expr
  | .lit s => .lit s
  | .cst c => .cst c
  | .var x => .var (rn a b x)
  | .add u v => .add (rnE a b u) (rnE a b v)
  | .mul u v => .mul (rnE a b u) (rnE a b v)
  | .bin o u v => .bin o (rnE a b u) (rnE a b v)
  | .eq u v => .eq (rnE a b u) (rnE a b v)
  | .lt u v => .lt (rnE a b u) (rnE a b v)
  | .ite c u v => .ite (rnE a b c) (rnE a b u) (rnE a b v)
  | .sel es k => .sel (rnEs a b es) (rnE a b k)
  | .call f u => .call f (rnE a b u)
  | .ref x rel => .ref (rn a b x) rel
  | .refIx x k rel => .refIx (rn a b x) (rnE a b k) rel
def rnEs (a b : Name) : List Expr → List Expr
  | [] => []
  | e :: es => rnE a b e :: rnEs a b es
end

mutual
def rnS (a b : Name) : Stmt → Stmt
  | .let_ x e => .let_ (rn a b x) (rnE a b e)
  | .set x e => .set (rn a b x) (rnE a b e)
  | .feat x e => .feat (rn a b x) (rnE a b e)
  | .forN x i c body => .forN (rn a b x) i (rnE a b c) (rnB a b body)
  | .forK x i kvs body => .forK (rn a b x) i (rnKV a b kvs) (rnB a b body)
  | .ifs c u v => .ifs (rnE a b c) (rnB a b u) (rnB a b v)
  | .use x p e => .use (rn a b x) p (rnE a b e)
  | .occ x c args => .occ (rn a b x) c (rnKV a b args)
def rnB (a b : Name) : List Stmt → List Stmt
  | [] => []
  | s :: ss => rnS a b s :: rnB a b ss
def rnKV (a b : Name) : List (Name × Expr) → List (Name × Expr)
  | [] => []
  | (k, e) :: kvs => (k, rnE a b e) :: rnKV a b kvs
end

mutual
/-- Code binding neither name, at any depth. -/
def NoBindS (a b : Name) : Stmt → Prop
  | .let_ x _ | .set x _ | .feat x _ | .use x _ _ | .occ x _ _ => x ≠ a ∧ x ≠ b
  | .forN x i _ body | .forK x i _ body => x ≠ a ∧ x ≠ b ∧ i ≠ a ∧ i ≠ b ∧ NoBindB a b body
  | .ifs _ u v => NoBindB a b u ∧ NoBindB a b v
def NoBindB (a b : Name) : List Stmt → Prop
  | [] => True
  | s :: ss => NoBindS a b s ∧ NoBindB a b ss
end

/-- The renamed environment reads `rn x` as the old one reads `x`, for every `x` but `b`. -/
def RRel (a b : Name) (ρ ρ' : Env) : Prop := ∀ x, x ≠ b → ρ'.lookup (rn a b x) = ρ.lookup x

theorem rn_ne {a b x : Name} (h : x ≠ a) : rn a b x = x := by simp [rn, h]

theorem rn_inj {a b x y : Name} (hx : x ≠ b) (hy : y ≠ b) (h : rn a b x = rn a b y) : x = y := by
  unfold rn at h
  by_cases ha : x = a <;> by_cases hb : y = a <;> simp only [ha, hb, ↓reduceIte] at h
  · rw [ha, hb]
  · exact absurd h.symm hy
  · exact absurd h hx
  · exact h

theorem rrel_cons_ren {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') (k : Name) (B : Bind) (hk : k ≠ b) :
    RRel a b ((k, B) :: ρ) ((rn a b k, B) :: ρ') := by
  intro x hx
  by_cases e : x = k
  · subst e; simp [List.lookup]
  · have h1 : (rn a b x == rn a b k) = false := by
      simp only [beq_eq_false_iff_ne]
      exact fun h' => e (rn_inj hx hk h')
    have h2 : (x == k) = false := by simpa using e
    simp only [List.lookup, h1, h2]
    exact h x hx

theorem rrel_cons_keep {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') (k : Name) (B : Bind)
    (ha : k ≠ a) (hb : k ≠ b) : RRel a b ((k, B) :: ρ) ((k, B) :: ρ') := by
  have := rrel_cons_ren h k B hb
  rwa [rn_ne ha] at this

/-- The bindings a statement adds, renamed. -/
def rnEnv (a b : Name) (Δ : Env) : Env := Δ.map fun p => (rn a b p.1, p.2)

theorem rrel_append_ren {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') :
    ∀ (Δ : Env), (∀ k ∈ keys Δ, k ≠ b) → RRel a b (Δ ++ ρ) (rnEnv a b Δ ++ ρ')
  | [], _ => h
  | (k, B) :: Δ, hk => by
    have := rrel_cons_ren (rrel_append_ren h Δ fun x hx => hk x (by simp [keys] at hx ⊢; exact .inr hx))
      k B (hk k (by simp [keys]))
    simpa [rnEnv] using this

theorem rnEnv_keep {a b : Name} : ∀ (Δ : Env), (∀ k ∈ keys Δ, k ≠ a) → rnEnv a b Δ = Δ
  | [], _ => rfl
  | (k, B) :: Δ, hk => by
    simp only [rnEnv, List.map_cons, rn_ne (hk k (by simp [keys]))]
    have := rnEnv_keep (b := b) Δ fun x hx => hk x (by simp [keys] at hx ⊢; exact .inr hx)
    simp only [rnEnv] at this
    rw [this]

/-- Names both sides read alike: those that are neither `a` nor `b`. -/
theorem rrel_agree {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') {N : List Name}
    (hN : ∀ y ∈ N, y ≠ a ∧ y ≠ b) : AgreeOn N ρ ρ' := by
  intro y hy
  have := h y (hN y hy).2
  rw [rn_ne (hN y hy).1] at this
  exact this.symm

theorem lookupNum_rn {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') {x : Name} (hx : x ≠ b) {t : Tr}
    (ht : lookupNum ρ x = .ok t) : lookupNum ρ' (rn a b x) = .ok t := by
  unfold lookupNum at ht ⊢; rw [h x hx]
  cases hl : ρ.lookup x with
  | none => rw [hl] at ht; cases ht
  | some B => rw [hl] at ht; cases B <;> simp_all

theorem lookupRef_rn {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') {x : Name} (hx : x ≠ b) {rel : FId}
    {t : Tr} (ht : lookupRef ρ x rel = .ok t) : lookupRef ρ' (rn a b x) rel = .ok t := by
  unfold lookupRef at ht ⊢; rw [h x hx]
  cases hl : ρ.lookup x with
  | none => rw [hl] at ht; cases ht
  | some B =>
    rw [hl] at ht
    cases B with
    | num _ => cases ht
    | frag m => simp only at ht ⊢; cases hm : m.lookup rel <;> simp_all

/-- Neither name is read by a definition. -/
def RnDefsOK (a b : Name) (D : Defs) : Prop := a ∉ D.reads ∧ b ∉ D.reads

theorem defs_agree {a b : Name} {D : Defs} {ρ ρ' : Env} (hD : RnDefsOK a b D) (h : RRel a b ρ ρ') :
    AgreeOn D.reads ρ ρ' :=
  rrel_agree h fun y hy => ⟨fun e => hD.1 (e ▸ hy), fun e => hD.2 (e ▸ hy)⟩

theorem binder_noBind {a b : Name} {s : Stmt} (hs : NoBindS a b s) {x : Name} (hx : s.binder = some x) :
    x ≠ a ∧ x ≠ b := by
  cases s <;> simp only [Stmt.binder, Option.some.injEq, reduceCtorEq] at hx <;> subst hx <;>
    simp only [NoBindS] at hs <;> exact ⟨hs.1, by first | exact hs.2 | exact hs.2.1⟩

structure RenN (a b : Name) (n : Nat) : Prop where
  e : ∀ {D σ ρ ρ' x r}, RnDefsOK a b D → evalE n D σ ρ x = .ok r → RRel a b ρ ρ' → b ∉ fvE x →
    evalE n D σ ρ' (rnE a b x) = .ok r
  one : ∀ {D σ ρ ρ' x r}, RnDefsOK a b D → execOne n D σ ρ x = .ok r → RRel a b ρ ρ' → NoBindS a b x →
    b ∉ fvS x → execOne n D σ ρ' (rnS a b x) = .ok r
  s : ∀ {D σ ρ ρ' x r}, RnDefsOK a b D → execS n D σ ρ x = .ok r → RRel a b ρ ρ' → NoBindB a b x →
    b ∉ fvB x → execS n D σ ρ' (rnB a b x) = .ok r
  ln : ∀ {D σ ρ ρ' i body ks r}, RnDefsOK a b D → loopN n D σ ρ i body ks = .ok r → RRel a b ρ ρ' →
    i ≠ a → i ≠ b → NoBindB a b body → b ∉ fvB body → loopN n D σ ρ' i (rnB a b body) ks = .ok r
  lk : ∀ {D σ ρ ρ' i body kvs r}, RnDefsOK a b D → loopK n D σ ρ i body kvs = .ok r → RRel a b ρ ρ' →
    i ≠ a → i ≠ b → NoBindB a b body → b ∉ fvKV kvs → b ∉ fvB body →
    loopK n D σ ρ' i (rnB a b body) (rnKV a b kvs) = .ok r
  as : ∀ {D σ ρ ρ' as r}, RnDefsOK a b D → evalA n D σ ρ as = .ok r → RRel a b ρ ρ' → b ∉ fvKV as →
    evalA n D σ ρ' (rnKV a b as) = .ok r

theorem rnEs_getElem? {a b : Name} : ∀ (es : List Expr) (i : Nat),
    (rnEs a b es)[i]? = (es[i]?).map (rnE a b)
  | [], i => by simp [rnEs]
  | e :: es, 0 => by simp [rnEs]
  | e :: es, i + 1 => by simp [rnEs, rnEs_getElem? es i]

theorem rnKV_keys {a b : Name} : ∀ (kvs : List (Name × Expr)) (k : Name),
    (rnKV a b kvs).lookup k = (kvs.lookup k).map (rnE a b)
  | [], k => by simp [rnKV, List.lookup]
  | (k', e) :: kvs, k => by
    simp only [rnKV, List.lookup]
    split <;> simp [rnKV_keys kvs k]

theorem argsOK_rnKV {a b : Name} (ps : List (Name × Expr)) :
    ∀ (args : List (Name × Expr)), argsOK ps (rnKV a b args) = argsOK ps args
  | [] => rfl
  | (k, e) :: args => by
    have := argsOK_rnKV (a := a) (b := b) ps args
    simp only [argsOK, rnKV, List.all_cons] at this ⊢
    rw [this]

theorem dflts_rnKV {a b : Name} (ps : List (Name × Expr)) (args : List (Name × Expr)) :
    dflts ps (rnKV a b args) = dflts ps args := by
  unfold dflts; congr 1; funext p; rw [rnKV_keys]; cases args.lookup p.1 <;> rfl

theorem ren_nested (a b : Name) : ∀ n, RenN a b n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D σ ρ ρ' x r hD h hr hb
      cases x with
      | lit s => simpa [evalE, rnE] using h
      | cst c => simpa [evalE, rnE] using h
      | var x =>
        have hx : x ≠ b := fun e => hb (by simp [fvE, e])
        simp only [evalE, rnE, bind_ok] at h ⊢
        obtain ⟨t, ht, h⟩ := h
        exact ⟨t, lookupNum_rn hr hx ht, h⟩
      | ref x rel =>
        have hx : x ≠ b := fun e => hb (by simp [fvE, e])
        simp only [evalE, rnE, bind_ok] at h ⊢
        obtain ⟨t, ht, h⟩ := h
        exact ⟨t, lookupRef_rn hr hx ht, h⟩
      | add u v | mul u v | eq u v | lt u v | bin o u v =>
        simp only [fvE, List.mem_append, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb', h⟩ := h
        cases h
        simp [evalE, rnE, ih.e hD ha hr hb.1, ih.e hD hb' hr hb.2]
      | ite c u v =>
        simp only [fvE, List.mem_append, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hu, h⟩ := h
        cases h
        have hc' := ih.e hD hc hr hb.1.1
        have hu' : evalE n D σ ρ' (if tc.den σ ≠ 0 then rnE a b u else rnE a b v) = .ok (t, l) := by
          split
          · rename_i hne; rw [if_pos hne] at hu; exact ih.e hD hu hr hb.1.2
          · rename_i hne; rw [if_neg hne] at hu; exact ih.e hD hu hr hb.2
        simp only [evalE, rnE, hc', ok_bind, hu']
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
          have he'' := ih.e hD he' hr (fun hm => hb.1 (hsub b hm))
          simp [evalE, rnE, ih.e hD hk hr hb.2, hi, rnEs_getElem?, he, he'']
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
          simp [evalE, rnE, ih.e hD ha hr hb, hf,
            (indep n).e hbd (agree_body (fn_reads hf) (defs_agree hD hr))]
        · cases h
      | refIx x k rel =>
        simp only [fvE, List.mem_cons, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        have ht' := lookupRef_rn hr (Ne.symm hb.1) ht
        simp [evalE, rnE, ih.e hD hk hr hb.2, hi, ht']
    · intro D σ ρ ρ' x r hD h hr hn hb
      cases x with
      | let_ x e | feat x e =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, ih.e hD he hr hb, rn_ne hn.1]
      | set x e =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_cons, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        have hu' := lookupNum_rn hr (Ne.symm hb.1) hu
        rw [rn_ne hn.1] at hu'
        simp [execOne, rnS, rn_ne hn.1, hu', ih.e hD he hr hb.2]
      | forN x i c body =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, rn_ne hn.1, ih.e hD hc hr hb.1, hk,
          ih.ln hD hl hr hn.2.2.1 hn.2.2.2.1 hn.2.2.2.2 hb.2]
      | forK x i kvs body =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, rn_ne hn.1, ih.lk hD hl hr hn.2.2.1 hn.2.2.2.1 hn.2.2.2.2 hb.1 hb.2]
      | ifs c u v =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hbr, h⟩ := h
        cases h
        have hc' := ih.e hD hc hr hb.1.1
        have hb' : execS n D σ ρ' (if tc.den σ ≠ 0 then rnB a b u else rnB a b v) = .ok (ρ1, mm, l) := by
          split
          · rename_i hne; rw [if_pos hne] at hbr; exact ih.s hD hbr hr hn.1 hb.1.2
          · rename_i hne; rw [if_neg hne] at hbr; exact ih.s hD hbr hr hn.2 hb.2
        simp only [execOne, rnS, hc', ok_bind, hb']
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
          simp [execOne, rnS, rn_ne hn.1, ih.e hD ha hr hb, hp,
            (indep n).s hbd (agree_body (proc_reads hp) (defs_agree hD hr))]
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
            have ag := defs_agree hD hr
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
            simp [execOne, rnS, rn_ne hn.1, hC, argsOK_rnKV, hok, dflts_rnKV, ih.as hD hg hr hb,
              (indep n).a hd ad, (indep n).s hbd ab]
          · cases h
        · cases h
    · intro D σ ρ ρ' x r hD h hr hn hb
      cases x with
      | nil => simpa [execS, rnB] using h
      | cons s ss =>
        simp only [NoBindB] at hn
        simp only [fvB, List.mem_append, not_or] at hb
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        have hk : ∀ k ∈ keys Δ1, k ≠ a ∧ k ≠ b := fun k hk' =>
          binder_noBind hn.1 (execOne_keys h1 k hk')
        have hr' : RRel a b (Δ1 ++ ρ) (Δ1 ++ ρ') := by
          have := rrel_append_ren hr Δ1 fun k hk' => (hk k hk').2
          rwa [rnEnv_keep Δ1 fun k hk' => (hk k hk').1] at this
        simp [execS, rnB, ih.one hD h1 hr hn.1 hb.1, ih.s hD h2 hr' hn.2 hb.2]
    · intro D σ ρ ρ' i body ks r hD h hr hia hib hn hb
      cases ks with
      | nil => simpa [loopN] using h
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopN, ih.s hD h1 (rrel_cons_keep hr _ _ hia hib) hn hb,
          ih.ln hD h2 hr hia hib hn hb]
    · intro D σ ρ ρ' i body kvs r hD h hr hia hib hn hkv hb
      cases kvs with
      | nil => simpa [loopK, rnKV] using h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [fvKV, List.mem_append, not_or] at hkv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopK, rnKV, ih.e hD h0 hr hkv.1, ih.s hD h1 (rrel_cons_keep hr _ _ hia hib) hn hb,
          ih.lk hD h2 hr hia hib hn hkv.2 hb]
    · intro D σ ρ ρ' as r hD h hr hb
      cases as with
      | nil => simpa [evalA, rnKV] using h
      | cons a' as =>
        obtain ⟨y, e⟩ := a'
        simp only [fvKV, List.mem_append, not_or] at hb
        simp only [evalA, bind_ok] at h
        obtain ⟨⟨t, l⟩, h0, ⟨Δ, l'⟩, h1, h⟩ := h
        cases h
        simp [evalA, rnKV, ih.e hD h0 hr hb.1, ih.as hD h1 hr hb.2]

/-! ## The document's statements, where `a` is bound -/

/-- An id with its first segment renamed (`nm a` to `nm b`). -/
def renFirst (a b : Name) : FId → FId
  | .nm x :: r => .nm (rn a b x) :: r
  | p => p

/-- A model with every feature's id renamed in its first segment; traces as they were. -/
def rnModel (a b : Name) (m : Model) : Model := m.map fun e => (renFirst a b e.1, e.2)

theorem rnModel_pre (a b x : Name) (m : Model) : rnModel a b (pre [.nm x] m) = pre [.nm (rn a b x)] m := by
  simp [rnModel, pre, renFirst]

mutual
/-- The document's statements: a top-level binder may be `a` (none is `b`); a loop's variable is
neither name, and nothing in a loop's body binds either. -/
def TopS (a b : Name) : Stmt → Prop
  | .let_ x _ | .set x _ | .feat x _ | .use x _ _ | .occ x _ _ => x ≠ b
  | .forN x i _ body | .forK x i _ body => x ≠ b ∧ i ≠ a ∧ i ≠ b ∧ NoBindB a b body
  | .ifs _ u v => TopB a b u ∧ TopB a b v
def TopB (a b : Name) : List Stmt → Prop
  | [] => True
  | s :: ss => TopS a b s ∧ TopB a b ss
end

theorem binder_top {a b : Name} {s : Stmt} (hs : TopS a b s) {x : Name} (hx : s.binder = some x) : x ≠ b := by
  cases s <;> simp only [Stmt.binder, Option.some.injEq, reduceCtorEq] at hx <;> subst hx <;>
    simp only [TopS] at hs <;> first | exact hs | exact hs.1

structure RenTop (a b : Name) (n : Nat) : Prop where
  one : ∀ {D σ ρ ρ' x Δ m l}, RnDefsOK a b D → execOne n D σ ρ x = .ok (Δ, m, l) → RRel a b ρ ρ' →
    TopS a b x → b ∉ fvS x → execOne n D σ ρ' (rnS a b x) = .ok (rnEnv a b Δ, rnModel a b m, l)
  s : ∀ {D σ ρ ρ' x Δ m l}, RnDefsOK a b D → execS n D σ ρ x = .ok (Δ, m, l) → RRel a b ρ ρ' →
    TopB a b x → b ∉ fvB x → execS n D σ ρ' (rnB a b x) = .ok (rnEnv a b Δ, rnModel a b m, l)

theorem ren_top (a b : Name) : ∀ n, RenTop a b n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [execOne, execS]
  | succ n ih =>
    have N := ren_nested a b n
    constructor
    · intro D σ ρ ρ' x Δ m l hD h hr ht hb
      cases x with
      | let_ x e =>
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l'⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, N.e hD he hr hb, rnEnv, rnModel]
      | feat x e =>
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l'⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, N.e hD he hr hb, rnEnv, rnModel, renFirst]
      | set x e =>
        simp only [fvS, List.mem_cons, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l'⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, lookupNum_rn hr (Ne.symm hb.1) hu, N.e hD he hr hb.2, rnEnv, rnModel]
      | forN x i c body =>
        simp only [TopS] at ht
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l'⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, N.e hD hc hr hb.1, hk, N.ln hD hl hr ht.2.1 ht.2.2.1 ht.2.2.2 hb.2,
          rnEnv, rnModel_pre]
      | forK x i kvs body =>
        simp only [TopS] at ht
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l'⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, N.lk hD hl hr ht.2.1 ht.2.2.1 ht.2.2.2 hb.1 hb.2, rnEnv, rnModel_pre]
      | ifs c u v =>
        simp only [TopS] at ht
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l'⟩, hbr, h⟩ := h
        cases h
        have hc' := N.e hD hc hr hb.1.1
        have hb' : execS n D σ ρ' (if tc.den σ ≠ 0 then rnB a b u else rnB a b v) =
            .ok (rnEnv a b ρ1, rnModel a b mm, l') := by
          split
          · rename_i hne; rw [if_pos hne] at hbr; exact ih.s hD hbr hr ht.1 hb.1.2
          · rename_i hne; rw [if_neg hne] at hbr; exact ih.s hD hbr hr ht.2 hb.2
        simp only [execOne, rnS, hc', ok_bind, hb']
        simp [rnEnv]
      | use x p e =>
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l'⟩, hbd, h⟩ := h
          cases h
          simp [execOne, rnS, N.e hD ha hr hb, hp,
            (indep n).s hbd (agree_body (proc_reads hp) (defs_agree hD hr)), rnEnv, rnModel_pre]
        · cases h
      | occ x c args =>
        simp only [fvS] at hb
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_ok] at h
            obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l'⟩, hbd, h⟩ := h
            cases h
            simp only [List.append_assoc] at hbd
            have cr := comp_reads hC
            have ag := defs_agree hD hr
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
            simp [execOne, rnS, hC, argsOK_rnKV, hok, dflts_rnKV, N.as hD hg hr hb,
              (indep n).a hd ad, (indep n).s hbd ab, rnEnv, rnModel_pre]
          · cases h
        · cases h
    · intro D σ ρ ρ' x Δ m l hD h hr ht hb
      cases x with
      | nil =>
        simp only [execS, Except.ok.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        simp [execS, rnB, rnEnv, rnModel]
      | cons s ss =>
        simp only [TopB] at ht
        simp only [fvB, List.mem_append, not_or] at hb
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        have hr' := rrel_append_ren hr Δ1 fun k hk => binder_top ht.1 (execOne_keys h1 k hk)
        simp only [execS, rnB, ih.one hD h1 hr ht.1 hb.1, ok_bind, ih.s hD h2 hr' ht.2 hb.2]
        simp [rnEnv, rnModel]

/-- Rename in a part document: if the document evaluates, the renamed one evaluates at the same
fuel, to the same traces and log, every binding renamed and every feature's id renamed in its
first segment (`a`'s features are now `b`'s; no other id changes). -/
theorem rename_body {n : Nat} {D : Defs} {σ : Site → Int} {body : List Stmt} {Δ : Env} {m : Model}
    {l : Log} (a b : Name) (hD : RnDefsOK a b D) (ht : TopB a b body) (hb : b ∉ fvB body)
    (h : execS n D σ [] body = .ok (Δ, m, l)) :
    execS n D σ [] (rnB a b body) = .ok (rnEnv a b Δ, rnModel a b m, l) :=
  (ren_top a b n).s hD h (fun _ _ => rfl) ht hb

/-- Only `a`'s features change id: an id whose first segment is not `nm a` is kept. -/
theorem renFirst_keep {a b : Name} : ∀ (p : FId), p.head? ≠ some (.nm a) → renFirst a b p = p
  | [], _ => rfl
  | .nm x :: r, h => by
    have : x ≠ a := fun e => h (by simp [e])
    simp [renFirst, rn_ne this]
  | .ix _ :: _, _ => rfl
  | .key _ :: _, _ => rfl

end ForgeLang
