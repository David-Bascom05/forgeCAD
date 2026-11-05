import ForgeLang.Fv

/-!
# Renaming sites

Run the same code twice: once in an environment `ρ` with table `σB`, once in `ρ` with every site
renamed by `θ` and table `σA`, where `σB = σA ∘ θ`. If `θ` fixes every site that appears in the code,
the two runs agree up to `θ`: same branches, same ids, traces and logs renamed. This is the engine
of SplitInstance (`Split.lean`): after the split, instance `j` carries site `q j` where before it
carried `p`, and `θ` maps `q j` back to `p`.
-/

namespace ForgeLang

def Tr.ren (θ : Site → Site) : Tr → Tr
  | .site s => .site (θ s)
  | .const c => .const c
  | .add a b => .add (a.ren θ) (b.ren θ)
  | .mul a b => .mul (a.ren θ) (b.ren θ)

def Model.ren (θ : Site → Site) (m : Model) : Model := m.map fun e => (e.1, e.2.ren θ)

def Bind.ren (θ : Site → Site) : Bind → Bind
  | .num t => .num (t.ren θ)
  | .frag m => .frag (m.ren θ)

def Env.ren (θ : Site → Site) (ρ : Env) : Env := ρ.map fun e => (e.1, e.2.ren θ)

theorem Tr.den_ren {θ : Site → Site} {σA σB : Site → Int} (hσ : ∀ s, σB s = σA (θ s)) :
    ∀ t : Tr, (t.ren θ).den σA = t.den σB
  | .site s => by simp [Tr.ren, Tr.den, hσ]
  | .const _ => rfl
  | .add a b => by simp [Tr.ren, Tr.den, Tr.den_ren hσ a, Tr.den_ren hσ b]
  | .mul a b => by simp [Tr.ren, Tr.den, Tr.den_ren hσ a, Tr.den_ren hσ b]

theorem Tr.sites_ren (θ : Site → Site) : ∀ t : Tr, (t.ren θ).sites = t.sites.map θ
  | .site _ => rfl
  | .const _ => rfl
  | .add a b => by simp [Tr.ren, Tr.sites, Tr.sites_ren θ a, Tr.sites_ren θ b]
  | .mul a b => by simp [Tr.ren, Tr.sites, Tr.sites_ren θ a, Tr.sites_ren θ b]

theorem obs_ren (D : Defs) (θ : Site → Site) (t : Tr) : obs D (t.ren θ) = (obs D t).map θ := by
  unfold obs; split <;> simp [Tr.sites_ren]

theorem Env.ren_append (θ : Site → Site) (A B : Env) : (A ++ B).ren θ = A.ren θ ++ B.ren θ := by
  simp [Env.ren]

theorem Model.ren_append (θ : Site → Site) (A B : Model) : (A ++ B).ren θ = A.ren θ ++ B.ren θ := by
  simp [Model.ren]

theorem Model.ren_pre (θ : Site → Site) (p : FId) (m : Model) : (pre p m).ren θ = pre p (m.ren θ) := by
  simp [Model.ren, pre]

theorem lookup_ren {β : Type} (f : β → β) : ∀ (l : List (Name × β)) (x : Name),
    (l.map fun e => (e.1, f e.2)).lookup x = (l.lookup x).map f
  | [], _ => rfl
  | (y, b) :: l, x => by
    simp only [List.map_cons, List.lookup]
    split <;> simp_all [lookup_ren f l x]

theorem fid_lookup_ren {β : Type} (f : β → β) : ∀ (l : List (FId × β)) (x : FId),
    (l.map fun e => (e.1, f e.2)).lookup x = (l.lookup x).map f
  | [], _ => rfl
  | (y, b) :: l, x => by
    simp only [List.map_cons, List.lookup]
    split <;> simp_all [fid_lookup_ren f l x]

theorem lookupNum_ren {θ : Site → Site} {ρ : Env} {x : Name} {t : Tr}
    (h : lookupNum (ρ.ren θ) x = .ok t) : ∃ t', lookupNum ρ x = .ok t' ∧ t'.ren θ = t := by
  unfold lookupNum at h ⊢
  rw [Env.ren, lookup_ren] at h
  cases hl : ρ.lookup x with
  | none => simp [hl] at h
  | some b =>
    cases b with
    | num t' => simp [hl, Bind.ren] at h; exact ⟨t', rfl, h⟩
    | frag m => simp [hl, Bind.ren] at h

theorem lookupRef_ren {θ : Site → Site} {ρ : Env} {x : Name} {rel : FId} {t : Tr}
    (h : lookupRef (ρ.ren θ) x rel = .ok t) : ∃ t', lookupRef ρ x rel = .ok t' ∧ t'.ren θ = t := by
  unfold lookupRef at h ⊢
  rw [Env.ren, lookup_ren] at h
  cases hl : ρ.lookup x with
  | none => simp [hl] at h
  | some b =>
    cases b with
    | num t' => simp [hl, Bind.ren] at h
    | frag m =>
      simp only [hl, Option.map_some, Bind.ren, Model.ren] at h
      rw [fid_lookup_ren] at h
      cases hm : m.lookup rel with
      | none => simp [hm] at h
      | some t' => simp [hm] at h; exact ⟨t', by simp only [hm], h⟩

/-! ## Sites in code -/

mutual
def codeE : Expr → List Site
  | .lit s => [s]
  | .cst _ | .var _ | .ref _ _ => []
  | .add a b | .mul a b | .eq a b => codeE a ++ codeE b
  | .ite c a b => codeE c ++ codeE a ++ codeE b
  | .sel es k => codeEs es ++ codeE k
  | .call _ a => codeE a
  | .refIx _ k _ => codeE k
def codeEs : List Expr → List Site
  | [] => []
  | e :: es => codeE e ++ codeEs es
end

mutual
def codeS : Stmt → List Site
  | .let_ _ e | .set _ e | .feat _ e | .use _ _ e => codeE e
  | .forN _ _ c b => codeE c ++ codeB b
  | .forK _ _ kvs b => codeKV kvs ++ codeB b
  | .ifs c a b => codeE c ++ codeB a ++ codeB b
  | .occ _ _ args => codeKV args
def codeB : List Stmt → List Site
  | [] => []
  | s :: ss => codeS s ++ codeB ss
def codeKV : List (Name × Expr) → List Site
  | [] => []
  | (_, e) :: kvs => codeE e ++ codeKV kvs
end

def Defs.code (D : Defs) : List Site :=
  D.fns.flatMap (fun f => codeE f.2.2) ++ D.procs.flatMap (fun p => codeB p.2.2) ++
    D.comps.flatMap (fun c => codeB c.2.body ++ codeKV c.2.params)

/-- `θ` fixes every site in `N`. -/
def Fix (θ : Site → Site) (N : List Site) : Prop := ∀ s ∈ N, θ s = s

theorem Fix.mono {θ : Site → Site} {N N' : List Site} (h : Fix θ N) (hs : ∀ s ∈ N', s ∈ N) : Fix θ N' :=
  fun s hs' => h s (hs s hs')

theorem codeE_sub_codeEs {e : Expr} : ∀ {es : List Expr}, e ∈ es → ∀ s ∈ codeE e, s ∈ codeEs es
  | [], h => by cases h
  | e' :: es, h => by
    intro s hs
    rw [codeEs, List.mem_append]
    rcases List.mem_cons.1 h with rfl | h
    · exact .inl hs
    · exact .inr (codeE_sub_codeEs h s hs)

theorem fn_code {D : Defs} {f x : Name} {body : Expr} (hf : D.fns.lookup f = some (x, body)) :
    ∀ s ∈ codeE body, s ∈ D.code := by
  intro s hs
  simp only [Defs.code, List.mem_append, List.mem_flatMap]
  exact .inl (.inl ⟨(f, x, body), lookup_mem hf, hs⟩)

theorem proc_code {D : Defs} {p y : Name} {body : List Stmt} (hp : D.procs.lookup p = some (y, body)) :
    ∀ s ∈ codeB body, s ∈ D.code := by
  intro s hs
  simp only [Defs.code, List.mem_append, List.mem_flatMap]
  exact .inl (.inr ⟨(p, y, body), lookup_mem hp, hs⟩)

theorem comp_code {D : Defs} {c : Name} {C : Comp} (hC : D.comps.lookup c = some C) :
    ∀ s ∈ codeB C.body ++ codeKV C.params, s ∈ D.code := by
  intro s hs
  simp only [Defs.code, List.mem_append, List.mem_flatMap]
  exact .inr ⟨(c, C), lookup_mem hC, by simpa using hs⟩

theorem codeKV_filter {p : Name × Expr → Bool} : ∀ {kvs : List (Name × Expr)} {s : Site},
    s ∈ codeKV (kvs.filter p) → s ∈ codeKV kvs
  | [], _, h => by simp [codeKV] at h
  | (k, e) :: kvs, s, h => by
    rw [List.filter_cons] at h
    split at h
    · simp only [codeKV, List.mem_append] at h ⊢
      exact h.elim .inl fun h => .inr (codeKV_filter h)
    · simp only [codeKV, List.mem_append]; exact .inr (codeKV_filter h)

theorem only_ren (N : List Name) (θ : Site → Site) (ρ : Env) : (only N ρ).ren θ = only N (ρ.ren θ) := by
  simp [only, Env.ren, List.filter_map, Function.comp_def]

/-- A component's code: its body's and defaults' sites are in `D.code`. -/
theorem comp_fix_body {θ : Site → Site} {D : Defs} {c : Name} {C : Comp} {N : List Site}
    (hC : D.comps.lookup c = some C) (fix : Fix θ (N ++ D.code)) : Fix θ (codeB C.body ++ D.code) :=
  fix.mono fun s hs => by
    rcases List.mem_append.1 hs with hs | hs
    · exact List.mem_append_right _ (comp_code hC s (List.mem_append_left _ hs))
    · exact List.mem_append_right _ hs

theorem comp_fix_dflts {θ : Site → Site} {D : Defs} {c : Name} {C : Comp} {N : List Site}
    (args : List (Name × Expr)) (hC : D.comps.lookup c = some C) (fix : Fix θ (N ++ D.code)) :
    Fix θ (codeKV (dflts C.params args) ++ D.code) :=
  fix.mono fun s hs => by
    rcases List.mem_append.1 hs with hs | hs
    · exact List.mem_append_right _ (comp_code hC s (List.mem_append_right _ (codeKV_filter hs)))
    · exact List.mem_append_right _ hs

structure RenAt (θ : Site → Site) (σA σB : Site → Int) (n : Nat) : Prop where
  e : ∀ {D ρ x r}, evalE n D σA (ρ.ren θ) x = .ok r → Fix θ (codeE x ++ D.code) →
    ∃ r', evalE n D σB ρ x = .ok r' ∧ (r'.1.ren θ, r'.2.map θ) = r
  one : ∀ {D ρ x r}, execOne n D σA (ρ.ren θ) x = .ok r → Fix θ (codeS x ++ D.code) →
    ∃ r', execOne n D σB ρ x = .ok r' ∧ (r'.1.ren θ, r'.2.1.ren θ, r'.2.2.map θ) = r
  s : ∀ {D ρ x r}, execS n D σA (ρ.ren θ) x = .ok r → Fix θ (codeB x ++ D.code) →
    ∃ r', execS n D σB ρ x = .ok r' ∧ (r'.1.ren θ, r'.2.1.ren θ, r'.2.2.map θ) = r
  ln : ∀ {D ρ i b ks r}, loopN n D σA (ρ.ren θ) i b ks = .ok r → Fix θ (codeB b ++ D.code) →
    ∃ r', loopN n D σB ρ i b ks = .ok r' ∧ (r'.1.ren θ, r'.2.map θ) = r
  lk : ∀ {D ρ i b kvs r}, loopK n D σA (ρ.ren θ) i b kvs = .ok r → Fix θ (codeKV kvs ++ codeB b ++ D.code) →
    ∃ r', loopK n D σB ρ i b kvs = .ok r' ∧ (r'.1.ren θ, r'.2.map θ) = r
  a : ∀ {D ρ as r}, evalA n D σA (ρ.ren θ) as = .ok r → Fix θ (codeKV as ++ D.code) →
    ∃ r', evalA n D σB ρ as = .ok r' ∧ (r'.1.ren θ, r'.2.map θ) = r

theorem fx {θ : Site → Site} {N M : List Site} (h : Fix θ N) (hs : ∀ s, s ∈ M → s ∈ N) : Fix θ M := h.mono hs

theorem ren : ∀ {θ σA σB}, (∀ s, σB s = σA (θ s)) → ∀ (n : Nat), RenAt θ σA σB n := by
  intro θ σA σB hσ n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D ρ x r h fix
      cases x with
      | lit s =>
        simp only [evalE, Except.ok.injEq] at h
        subst h
        exact ⟨_, rfl, by simp [Tr.ren, fix s (by simp [codeE])]⟩
      | cst c =>
        simp only [evalE, Except.ok.injEq] at h
        subst h; exact ⟨_, rfl, rfl⟩
      | var x =>
        simp only [evalE, bind_ok] at h
        obtain ⟨t, ht, h⟩ := h
        cases h
        obtain ⟨t', ht', rfl⟩ := lookupNum_ren ht
        exact ⟨_, by simp [evalE, ht'] <;> rfl, rfl⟩
      | ref x rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨t, ht, h⟩ := h
        cases h
        obtain ⟨t', ht', rfl⟩ := lookupRef_ren ht
        exact ⟨_, by simp [evalE, ht'] <;> rfl, rfl⟩
      | add a b | mul a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        obtain ⟨⟨ta', la'⟩, ha', ea⟩ := ih.e ha (fx fix fun s hs => by simp [codeE] at *; grind)
        obtain ⟨⟨tb', lb'⟩, hb', eb⟩ := ih.e hb (fx fix fun s hs => by simp [codeE] at *; grind)
        simp only [Prod.mk.injEq] at ea eb
        obtain ⟨rfl, rfl⟩ := ea
        obtain ⟨rfl, rfl⟩ := eb
        exact ⟨_, by simp [evalE, ha', hb'] <;> rfl, by simp [Tr.ren]⟩
      | eq a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        obtain ⟨⟨ta', la'⟩, ha', ea⟩ := ih.e ha (fx fix fun s hs => by simp [codeE] at *; grind)
        obtain ⟨⟨tb', lb'⟩, hb', eb⟩ := ih.e hb (fx fix fun s hs => by simp [codeE] at *; grind)
        simp only [Prod.mk.injEq] at ea eb
        obtain ⟨rfl, rfl⟩ := ea
        obtain ⟨rfl, rfl⟩ := eb
        exact ⟨_, by simp [evalE, ha', hb'] <;> rfl, by
          simp [Tr.ren, Tr.den_ren hσ, obs_ren]⟩
      | ite c a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hb, h⟩ := h
        cases h
        obtain ⟨⟨tc', lc'⟩, hc', ec⟩ := ih.e hc (fx fix fun s hs => by simp [codeE] at *; grind)
        simp only [Prod.mk.injEq] at ec
        obtain ⟨rfl, rfl⟩ := ec
        rw [Tr.den_ren hσ] at hb
        obtain ⟨⟨t', l'⟩, hb', eb⟩ := ih.e hb (by
          split <;> exact fx fix fun s hs => by simp [codeE] at *; grind)
        simp only [Prod.mk.injEq] at eb
        obtain ⟨rfl, rfl⟩ := eb
        exact ⟨_, by simp only [evalE, hc', ok_bind, hb'] <;> rfl, by simp [obs_ren]⟩
      | sel es k =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, he', h⟩ := h
          cases h
          obtain ⟨⟨tk', lk'⟩, hk', ek⟩ := ih.e hk (fx fix fun s hs => by simp [codeE] at *; grind)
          simp only [Prod.mk.injEq] at ek
          obtain ⟨rfl, rfl⟩ := ek
          rw [Tr.den_ren hσ] at hi
          have hsub := codeE_sub_codeEs (List.mem_of_getElem? he)
          obtain ⟨⟨t', l'⟩, he'', ee⟩ := ih.e he' (fx fix fun s hs => by
            simp only [codeE, List.mem_append] at hs ⊢
            rcases hs with hs | hs
            · exact .inl (.inl (hsub s hs))
            · exact .inr hs)
          simp only [Prod.mk.injEq] at ee
          obtain ⟨rfl, rfl⟩ := ee
          exact ⟨_, by simp [evalE, hk', hi, he, he''] <;> rfl, by simp [obs_ren]⟩
        · cases h
      | call f a =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i x body hf
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, hb, h⟩ := h
          cases h
          obtain ⟨⟨ta', la'⟩, ha', ea⟩ := ih.e ha (fx fix fun s hs => by simp [codeE] at *; grind)
          simp only [Prod.mk.injEq] at ea
          obtain ⟨rfl, rfl⟩ := ea
          have hb2 : evalE n D σA (Env.ren θ ((x, Bind.num ta') :: ρ)) body = .ok (t, l) := by
            simpa [Env.ren, Bind.ren] using hb
          obtain ⟨⟨t', l'⟩, hb', eb⟩ := ih.e hb2 (fx fix fun s hs => by
            simp only [List.mem_append] at hs ⊢
            rcases hs with hs | hs
            · exact .inr (fn_code hf s hs)
            · exact .inr hs)
          simp only [Prod.mk.injEq] at eb
          obtain ⟨rfl, rfl⟩ := eb
          exact ⟨_, by simp [evalE, ha', hf, hb'] <;> rfl, by simp⟩
        · cases h
      | refIx x k rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        obtain ⟨⟨tk', lk'⟩, hk', ek⟩ := ih.e hk (fx fix fun s hs => by simp [codeE] at *; grind)
        simp only [Prod.mk.injEq] at ek
        obtain ⟨rfl, rfl⟩ := ek
        rw [Tr.den_ren hσ] at hi
        obtain ⟨t', ht', rfl⟩ := lookupRef_ren ht
        exact ⟨_, by simp [evalE, hk', hi, ht'] <;> rfl, by simp [obs_ren]⟩
    · intro D ρ x r h fix
      cases x with
      | let_ x e | feat x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        obtain ⟨⟨t', l'⟩, he', ee⟩ := ih.e he (fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at ee
        obtain ⟨rfl, rfl⟩ := ee
        exact ⟨_, by simp [execOne, he'] <;> rfl, by simp [Env.ren, Bind.ren, Model.ren]⟩
      | set x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        obtain ⟨u', hu', -⟩ := lookupNum_ren hu
        obtain ⟨⟨t', l'⟩, he', ee⟩ := ih.e he (fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at ee
        obtain ⟨rfl, rfl⟩ := ee
        exact ⟨_, by simp [execOne, hu', he'] <;> rfl, by simp [Env.ren, Bind.ren, Model.ren]⟩
      | forN x i c body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        obtain ⟨⟨tc', lc'⟩, hc', ec⟩ := ih.e hc (fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at ec
        obtain ⟨rfl, rfl⟩ := ec
        rw [Tr.den_ren hσ] at hk
        obtain ⟨⟨mm', l'⟩, hl', el⟩ := ih.ln hl (fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at el
        obtain ⟨rfl, rfl⟩ := el
        exact ⟨_, by simp [execOne, hc', hk, hl'] <;> rfl, by
          simp [Env.ren, Bind.ren, Model.ren_pre, obs_ren]⟩
      | forK x i kvs body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        obtain ⟨⟨mm', l'⟩, hl', el⟩ := ih.lk hl (fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at el
        obtain ⟨rfl, rfl⟩ := el
        exact ⟨_, by simp [execOne, hl'] <;> rfl, by simp [Env.ren, Bind.ren, Model.ren_pre]⟩
      | ifs c a b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hb, h⟩ := h
        cases h
        obtain ⟨⟨tc', lc'⟩, hc', ec⟩ := ih.e hc (fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at ec
        obtain ⟨rfl, rfl⟩ := ec
        rw [Tr.den_ren hσ] at hb
        obtain ⟨⟨ρ1', mm', l'⟩, hb', eb⟩ := ih.s hb (by
          split <;> exact fx fix fun s hs => by simp [codeS] at *; grind)
        simp only [Prod.mk.injEq] at eb
        obtain ⟨-, rfl, rfl⟩ := eb
        exact ⟨_, by simp only [execOne, hc', ok_bind, hb'] <;> rfl, by simp [Env.ren, obs_ren]⟩
      | use x p a =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l⟩, hb, h⟩ := h
          cases h
          obtain ⟨⟨ta', la'⟩, ha', ea⟩ := ih.e ha (fx fix fun s hs => by simp [codeS] at *; grind)
          simp only [Prod.mk.injEq] at ea
          obtain ⟨rfl, rfl⟩ := ea
          have hb2 : execS n D σA (Env.ren θ ((y, Bind.num ta') :: ρ)) body = .ok (ρ1, mm, l) := by
            simpa [Env.ren, Bind.ren] using hb
          obtain ⟨⟨ρ1', mm', l'⟩, hb', eb⟩ := ih.s hb2 (fx fix fun s hs => by
            simp only [List.mem_append] at hs ⊢
            rcases hs with hs | hs
            · exact .inr (proc_code hp s hs)
            · exact .inr hs)
          simp only [Prod.mk.injEq] at eb
          obtain ⟨-, rfl, rfl⟩ := eb
          exact ⟨_, by simp [execOne, ha', hp, hb'] <;> rfl, by
            simp [Env.ren, Bind.ren, Model.ren_pre]⟩
        · cases h
      | occ x c args =>
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_ok] at h
            obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l⟩, hb, h⟩ := h
            cases h
            obtain ⟨⟨Δg', lg'⟩, hg', eg⟩ := ih.a hg (fx fix fun s hs => by simp [codeS] at *; grind)
            simp only [Prod.mk.injEq] at eg
            obtain ⟨rfl, rfl⟩ := eg
            rw [← only_ren] at hd
            obtain ⟨⟨Δd', ld'⟩, hd', ed⟩ := ih.a hd (comp_fix_dflts args hC fix)
            simp only [Prod.mk.injEq] at ed
            obtain ⟨rfl, rfl⟩ := ed
            have hb2 : execS n D σA (Env.ren θ (Δg' ++ Δd' ++ only C.vis ρ)) C.body = .ok (ρ1, mm, l) := by
              simpa [Env.ren_append, only_ren] using hb
            obtain ⟨⟨ρ1', mm', l'⟩, hb', eb⟩ := ih.s hb2 (comp_fix_body hC fix)
            simp only [Prod.mk.injEq] at eb
            obtain ⟨-, rfl, rfl⟩ := eb
            simp only [List.append_assoc] at hb'
            exact ⟨_, by simp [execOne, hC, hok, hg', hd', hb'] <;> rfl, by
              simp [Env.ren, Bind.ren, Model.ren_pre]⟩
          · cases h
        · cases h
    · intro D ρ x r h fix
      cases x with
      | nil =>
        cases n <;> simp [execS] at h
        all_goals (subst h; exact ⟨_, rfl, by simp [Env.ren, Model.ren]⟩)
      | cons s ss =>
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        obtain ⟨⟨Δ1', m1', l1'⟩, h1', e1⟩ := ih.one h1 (fx fix fun s hs => by simp [codeB] at *; grind)
        simp only [Prod.mk.injEq] at e1
        obtain ⟨rfl, rfl, rfl⟩ := e1
        have h2' : execS n D σA (Env.ren θ (Δ1' ++ ρ)) ss = .ok (Δ2, m2, l2) := by
          rw [Env.ren_append]; exact h2
        obtain ⟨⟨Δ2', m2', l2'⟩, h2'', e2⟩ := ih.s h2' (fx fix fun s hs => by simp [codeB] at *; grind)
        simp only [Prod.mk.injEq] at e2
        obtain ⟨rfl, rfl, rfl⟩ := e2
        exact ⟨_, by simp [execS, h1', h2''] <;> rfl, by
          simp [Env.ren_append, Model.ren_append]⟩
    · intro D ρ i b ks r h fix
      cases ks with
      | nil =>
        cases n <;> simp [loopN] at h
        all_goals (subst h; exact ⟨_, rfl, by simp [Model.ren]⟩)
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        have h1b : execS n D σA (Env.ren θ ((i, Bind.num (Tr.const k)) :: ρ)) b = .ok (Δ1, m1, l1) := by
          simpa [Env.ren, Bind.ren, Tr.ren] using h1
        obtain ⟨⟨Δ1', m1', l1'⟩, h1', e1⟩ := ih.s h1b fix
        simp only [Prod.mk.injEq] at e1
        obtain ⟨-, rfl, rfl⟩ := e1
        obtain ⟨⟨m2', l2'⟩, h2', e2⟩ := ih.ln h2 fix
        simp only [Prod.mk.injEq] at e2
        obtain ⟨rfl, rfl⟩ := e2
        exact ⟨_, by simp [loopN, h1', h2'] <;> rfl, by simp [Model.ren_append, Model.ren_pre]⟩
    · intro D ρ i b kvs r h fix
      cases kvs with
      | nil =>
        cases n <;> simp [loopK] at h
        all_goals (subst h; exact ⟨_, rfl, by simp [Model.ren]⟩)
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        obtain ⟨⟨t', l0'⟩, h0', e0⟩ := ih.e h0 (fx fix fun s hs => by simp [codeKV] at *; grind)
        simp only [Prod.mk.injEq] at e0
        obtain ⟨rfl, rfl⟩ := e0
        have h1b : execS n D σA (Env.ren θ ((i, Bind.num t') :: ρ)) b = .ok (Δ1, m1, l1) := by
          simpa [Env.ren, Bind.ren] using h1
        obtain ⟨⟨Δ1', m1', l1'⟩, h1', e1⟩ := ih.s h1b (fx fix fun s hs => by simp [codeKV] at *; grind)
        simp only [Prod.mk.injEq] at e1
        obtain ⟨-, rfl, rfl⟩ := e1
        obtain ⟨⟨m2', l2'⟩, h2', e2⟩ := ih.lk h2 (fx fix fun s hs => by simp [codeKV] at *; grind)
        simp only [Prod.mk.injEq] at e2
        obtain ⟨rfl, rfl⟩ := e2
        exact ⟨_, by simp [loopK, h0', h1', h2'] <;> rfl, by simp [Model.ren_append, Model.ren_pre]⟩
    · intro D ρ as r h fix
      cases as with
      | nil =>
        cases n <;> simp [evalA] at h
        all_goals (subst h; exact ⟨_, rfl, by simp [Env.ren]⟩)
      | cons a as =>
        obtain ⟨y, e⟩ := a
        simp only [evalA, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨Δ, l1⟩, h1, h⟩ := h
        cases h
        obtain ⟨⟨t', l0'⟩, h0', e0⟩ := ih.e h0 (fx fix fun s hs => by simp [codeKV] at *; grind)
        simp only [Prod.mk.injEq] at e0
        obtain ⟨rfl, rfl⟩ := e0
        obtain ⟨⟨Δ', l1'⟩, h1', e1⟩ := ih.a h1 (fx fix fun s hs => by simp [codeKV] at *; grind)
        simp only [Prod.mk.injEq] at e1
        obtain ⟨rfl, rfl⟩ := e1
        exact ⟨_, by simp [evalA, h0', h1'] <;> rfl, by simp [Env.ren, Bind.ren]⟩

end ForgeLang
