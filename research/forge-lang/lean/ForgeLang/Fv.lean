import ForgeLang.Weak

/-!
# The free-name lemma

A run depends on its environment only through the names its code mentions, plus the names that
function and procedure bodies mention (`Defs.reads`: bodies see the names bound where they are
called). Two environments that agree on those names give the same result. This is what makes the
*complete* half of the structural edits true (`Complete.lean`): if nothing mentions a name, removing
or moving its binding cannot stop the document from evaluating.
-/

namespace ForgeLang

mutual
def fvE : Expr → List Name
  | .lit _ | .cst _ => []
  | .var x => [x]
  | .add a b | .mul a b | .eq a b => fvE a ++ fvE b
  | .ite c a b => fvE c ++ fvE a ++ fvE b
  | .sel es k => fvEs es ++ fvE k
  | .call _ a => fvE a
  | .ref x _ => [x]
  | .refIx x k _ => x :: fvE k
def fvEs : List Expr → List Name
  | [] => []
  | e :: es => fvE e ++ fvEs es
end

mutual
def fvS : Stmt → List Name
  | .let_ _ e | .feat _ e => fvE e
  | .set x e => x :: fvE e
  | .forN _ _ c b => fvE c ++ fvB b
  | .forK _ _ kvs b => fvKV kvs ++ fvB b
  | .ifs c a b => fvE c ++ fvB a ++ fvB b
  | .use _ _ a => fvE a
def fvB : List Stmt → List Name
  | [] => []
  | s :: ss => fvS s ++ fvB ss
def fvKV : List (Name × Expr) → List Name
  | [] => []
  | (_, e) :: kvs => fvE e ++ fvKV kvs
end

/-- Every name a function or procedure body mentions, other than its own parameter. -/
def Defs.reads (D : Defs) : List Name :=
  D.fns.flatMap (fun f => (fvE f.2.2).filter (· ≠ f.2.1)) ++
    D.procs.flatMap (fun p => (fvB p.2.2).filter (· ≠ p.2.1))

def AgreeOn (N : List Name) (ρ ρ' : Env) : Prop := ∀ y ∈ N, ρ.lookup y = ρ'.lookup y

theorem AgreeOn.mono {N N' : List Name} {ρ ρ' : Env} (h : AgreeOn N ρ ρ') (hs : ∀ y ∈ N', y ∈ N) :
    AgreeOn N' ρ ρ' := fun y hy => h y (hs y hy)

theorem AgreeOn.append (Δ : Env) {N : List Name} {ρ ρ' : Env} (h : AgreeOn N ρ ρ') :
    AgreeOn N (Δ ++ ρ) (Δ ++ ρ') := by
  intro y hy
  induction Δ with
  | nil => exact h y hy
  | cons a Δ ih =>
    obtain ⟨z, b⟩ := a
    simp only [List.cons_append, List.lookup]
    split
    · rfl
    · exact ih

theorem AgreeOn.cons (a : Name × Bind) {N : List Name} {ρ ρ' : Env} (h : AgreeOn N ρ ρ') :
    AgreeOn N (a :: ρ) (a :: ρ') := h.append [a]

theorem lookup_mem {α β : Type} [BEq α] [LawfulBEq α] {l : List (α × β)} {a : α} {b : β}
    (h : l.lookup a = some b) : (a, b) ∈ l := by
  induction l with
  | nil => cases h
  | cons p l ih =>
    obtain ⟨k, v⟩ := p
    simp only [List.lookup] at h
    split at h
    · rename_i hk
      cases h
      have : a = k := by simpa using hk
      subst this; exact List.mem_cons_self
    · exact List.mem_cons_of_mem _ (ih h)

theorem fvE_sub_fvEs {e : Expr} : ∀ {es : List Expr}, e ∈ es → ∀ y ∈ fvE e, y ∈ fvEs es
  | [], h => by cases h
  | e' :: es, h => by
    intro y hy
    rw [fvEs, List.mem_append]
    rcases List.mem_cons.1 h with rfl | h
    · exact .inl hy
    · exact .inr (fvE_sub_fvEs h y hy)

/-- A body sees its parameter, which both sides bind the same way, and names in `D.reads`. -/
theorem agree_body {D : Defs} {ρ ρ' : Env} {x : Name} {B : Bind} {body : List Name}
    (hr : ∀ y ∈ body, y ≠ x → y ∈ D.reads) (h : AgreeOn D.reads ρ ρ') :
    AgreeOn (body ++ D.reads) ((x, B) :: ρ) ((x, B) :: ρ') := by
  intro y hy
  by_cases hyx : y = x
  · subst hyx; simp [List.lookup]
  · have hy' : y ∈ D.reads := by
      rcases List.mem_append.1 hy with hy | hy
      · exact hr y hy hyx
      · exact hy
    have : (y == x) = false := by simp [hyx]
    simp only [List.lookup, this]
    exact h y hy'

theorem fn_reads {D : Defs} {f x : Name} {body : Expr} (hf : D.fns.lookup f = some (x, body)) :
    ∀ y ∈ fvE body, y ≠ x → y ∈ D.reads := by
  intro y hy hyx
  simp only [Defs.reads, List.mem_append, List.mem_flatMap, List.mem_filter]
  exact .inl ⟨(f, x, body), lookup_mem hf, hy, by simpa using hyx⟩

theorem proc_reads {D : Defs} {p y0 : Name} {body : List Stmt} (hp : D.procs.lookup p = some (y0, body)) :
    ∀ y ∈ fvB body, y ≠ y0 → y ∈ D.reads := by
  intro y hy hyx
  simp only [Defs.reads, List.mem_append, List.mem_flatMap, List.mem_filter]
  exact .inr ⟨(p, y0, body), lookup_mem hp, hy, by simpa using hyx⟩

theorem lookupNum_agree {ρ ρ' : Env} {x : Name} (h : ρ.lookup x = ρ'.lookup x) :
    lookupNum ρ x = lookupNum ρ' x := by unfold lookupNum; rw [h]

theorem lookupRef_agree {ρ ρ' : Env} {x : Name} {rel : FId} (h : ρ.lookup x = ρ'.lookup x) :
    lookupRef ρ x rel = lookupRef ρ' x rel := by unfold lookupRef; rw [h]

structure Indep (n : Nat) : Prop where
  e : ∀ {D σ ρ ρ' x r}, evalE n D σ ρ x = .ok r → AgreeOn (fvE x ++ D.reads) ρ ρ' →
    evalE n D σ ρ' x = .ok r
  one : ∀ {D σ ρ ρ' x r}, execOne n D σ ρ x = .ok r → AgreeOn (fvS x ++ D.reads) ρ ρ' →
    execOne n D σ ρ' x = .ok r
  s : ∀ {D σ ρ ρ' x r}, execS n D σ ρ x = .ok r → AgreeOn (fvB x ++ D.reads) ρ ρ' →
    execS n D σ ρ' x = .ok r
  ln : ∀ {D σ ρ ρ' i b ks r}, loopN n D σ ρ i b ks = .ok r → AgreeOn (fvB b ++ D.reads) ρ ρ' →
    loopN n D σ ρ' i b ks = .ok r
  lk : ∀ {D σ ρ ρ' i b kvs r}, loopK n D σ ρ i b kvs = .ok r →
    AgreeOn (fvKV kvs ++ fvB b ++ D.reads) ρ ρ' → loopK n D σ ρ' i b kvs = .ok r

/-- Split an agreement over `a ++ b ++ D.reads` style unions. -/
theorem ag_l {N M R : List Name} {ρ ρ' : Env} (h : AgreeOn (N ++ M ++ R) ρ ρ') : AgreeOn (N ++ R) ρ ρ' :=
  h.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind
theorem ag_r {N M R : List Name} {ρ ρ' : Env} (h : AgreeOn (N ++ M ++ R) ρ ρ') : AgreeOn (M ++ R) ρ ρ' :=
  h.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind
theorem ag_R {N R : List Name} {ρ ρ' : Env} (h : AgreeOn (N ++ R) ρ ρ') : AgreeOn R ρ ρ' :=
  h.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind
theorem ag_N {N R : List Name} {ρ ρ' : Env} (h : AgreeOn (N ++ R) ρ ρ') : AgreeOn N ρ ρ' :=
  h.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind

theorem indep : ∀ n, Indep n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK]
  | succ n ih =>
    constructor
    · intro D σ ρ ρ' x r h ag
      cases x with
      | lit s => simpa [evalE] using h
      | cst c => simpa [evalE] using h
      | var x =>
        simp only [fvE] at ag
        have := lookupNum_agree (ag x (by simp))
        simp only [evalE] at h ⊢; rw [← this]; exact h
      | ref x rel =>
        simp only [fvE] at ag
        have := lookupRef_agree (rel := rel) (ag x (by simp))
        simp only [evalE] at h ⊢; rw [← this]; exact h
      | add a b | mul a b | eq a b =>
        simp only [fvE] at ag
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp [evalE, ih.e ha (ag_l ag), ih.e hb (ag_r ag)]
      | ite c a b =>
        simp only [fvE] at ag
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hb, h⟩ := h
        cases h
        have hc' := ih.e hc (ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind)
        have hb' := ih.e hb (by
          split <;> exact ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind)
        simp only [evalE, hc', ok_bind, hb']
      | sel es k =>
        simp only [fvE] at ag
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, he', h⟩ := h
          cases h
          have hsub := fvE_sub_fvEs (List.mem_of_getElem? he)
          have he'' := ih.e he' (ag.mono fun y hy => by
            simp only [List.mem_append] at hy ⊢
            rcases hy with hy | hy
            · exact .inl (.inl (hsub y hy))
            · exact .inr hy)
          simp [evalE, ih.e hk (ag_r ag), hi, he, he'']
        · cases h
      | call f a =>
        simp only [fvE] at ag
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i x body hf
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, hb, h⟩ := h
          cases h
          simp [evalE, ih.e ha ag, hf, ih.e hb (agree_body (fn_reads hf) (ag_R ag))]
        · cases h
      | refIx x k rel =>
        simp only [fvE] at ag
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        have hx := lookupRef_agree (rel := .ix i :: rel) (ag x (by simp))
        have ag' : AgreeOn (fvE k ++ D.reads) ρ ρ' := ag.mono fun y hy => by simp_all
        rw [hx] at ht
        simp [evalE, ih.e hk ag', hi, ht]
    · intro D σ ρ ρ' x r h ag
      cases x with
      | let_ x e | feat x e =>
        simp only [fvS] at ag
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, ih.e he ag]
      | set x e =>
        simp only [fvS] at ag
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        rw [lookupNum_agree (ag x (by simp))] at hu
        have ag' : AgreeOn (fvE e ++ D.reads) ρ ρ' := ag.mono fun y hy => by simp_all
        simp [execOne, hu, ih.e he ag']
      | forN x i c body =>
        simp only [fvS] at ag
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, ih.e hc (ag_l ag), hk, ih.ln hl (ag_r ag)]
      | forK x i kvs body =>
        simp only [fvS] at ag
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, ih.lk hl ag]
      | ifs c a b =>
        simp only [fvS] at ag
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hb, h⟩ := h
        cases h
        have hc' := ih.e hc (ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind)
        have hb' := ih.s hb (by
          split <;> exact ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind)
        simp only [execOne, hc', ok_bind, hb']
      | use x p a =>
        simp only [fvS] at ag
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l⟩, hb, h⟩ := h
          cases h
          simp [execOne, ih.e ha ag, hp, ih.s hb (agree_body (proc_reads hp) (ag_R ag))]
        · cases h
    · intro D σ ρ ρ' x r h ag
      cases x with
      | nil => simpa [execS] using h
      | cons s ss =>
        simp only [fvB] at ag
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        simp [execS, ih.one h1 (ag_l ag), ih.s h2 ((ag_r ag).append Δ1)]
    · intro D σ ρ ρ' i b ks r h ag
      cases ks with
      | nil => simpa [loopN] using h
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopN, ih.s h1 (ag.cons _), ih.ln h2 ag]
    · intro D σ ρ ρ' i b kvs r h ag
      cases kvs with
      | nil => simpa [loopK] using h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [fvKV] at ag
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        have a0 : AgreeOn (fvE e ++ D.reads) ρ ρ' :=
          ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind
        have a1 : AgreeOn (fvB b ++ D.reads) ρ ρ' :=
          ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind
        have a2 : AgreeOn (fvKV kvs ++ fvB b ++ D.reads) ρ ρ' :=
          ag.mono fun y hy => by simp only [List.mem_append] at hy ⊢; grind
        simp [loopK, ih.e h0 a0, ih.s h1 (a1.cons _), ih.lk h2 a2]

end ForgeLang
