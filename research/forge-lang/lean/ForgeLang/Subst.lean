import ForgeLang.Sites

/-!
# Substituting traces for sites, as an equation

Run the same code with a literal table `σB` in an environment whose traces had every site `s`
replaced by a trace `θ s`, and with table `σA` in the original environment, where
`σA s = (θ s).den σB`. If `θ` leaves every site the code mentions alone, the first run *is* the
second with `θ` applied to its traces and logs: same success or same typed error, same branches,
same ids. Being an equation, it gives both directions at once, which is what the sound and
complete halves of InlineExpr and MakeShared (`Shared.lean`) need.
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

def Tr.subst (θ : Site → Tr) : Tr → Tr
  | .site s => θ s
  | .const c => .const c
  | .add a b => .add (a.subst θ) (b.subst θ)
  | .mul a b => .mul (a.subst θ) (b.subst θ)

def Model.subst (θ : Site → Tr) (m : Model) : Model := m.map fun e => (e.1, e.2.subst θ)

def Bind.subst (θ : Site → Tr) : Bind → Bind
  | .num t => .num (t.subst θ)
  | .frag m => .frag (m.subst θ)

def Env.subst (θ : Site → Tr) (ρ : Env) : Env := ρ.map fun e => (e.1, e.2.subst θ)

def substLog (θ : Site → Tr) (l : Log) : Log := l.flatMap fun s => (θ s).sites

def fE (θ : Site → Tr) (r : Tr × Log) : Tr × Log := (r.1.subst θ, substLog θ r.2)
def fS (θ : Site → Tr) (r : Env × Model × Log) : Env × Model × Log :=
  (r.1.subst θ, r.2.1.subst θ, substLog θ r.2.2)
def fL (θ : Site → Tr) (r : Model × Log) : Model × Log := (r.1.subst θ, substLog θ r.2)

/-- `θ` leaves the sites in `N` alone. -/
def FixT (θ : Site → Tr) (N : List Site) : Prop := ∀ s ∈ N, θ s = .site s

theorem FixT.mono {θ : Site → Tr} {N N' : List Site} (h : FixT θ N) (hs : ∀ s ∈ N', s ∈ N) : FixT θ N' :=
  fun s hs' => h s (hs s hs')

theorem Tr.den_subst {θ : Site → Tr} {σA σB : Site → Int} (hσ : ∀ s, σA s = (θ s).den σB) :
    ∀ t : Tr, (t.subst θ).den σB = t.den σA
  | .site s => by simp [Tr.subst, Tr.den, hσ]
  | .const _ => rfl
  | .add a b => by simp [Tr.subst, Tr.den, Tr.den_subst hσ a, Tr.den_subst hσ b]
  | .mul a b => by simp [Tr.subst, Tr.den, Tr.den_subst hσ a, Tr.den_subst hσ b]

theorem Tr.sites_subst (θ : Site → Tr) : ∀ t : Tr, (t.subst θ).sites = substLog θ t.sites
  | .site _ => by simp [Tr.subst, Tr.sites, substLog]
  | .const _ => rfl
  | .add a b => by simp [Tr.subst, Tr.sites, Tr.sites_subst θ a, Tr.sites_subst θ b, substLog]
  | .mul a b => by simp [Tr.subst, Tr.sites, Tr.sites_subst θ a, Tr.sites_subst θ b, substLog]

theorem substLog_append (θ : Site → Tr) (a b : Log) : substLog θ (a ++ b) = substLog θ a ++ substLog θ b := by
  simp [substLog]

theorem substLog_nil (θ : Site → Tr) : substLog θ [] = [] := rfl

theorem obs_subst (D : Defs) (θ : Site → Tr) (t : Tr) : obs D (t.subst θ) = substLog θ (obs D t) := by
  unfold obs; split <;> simp [Tr.sites_subst, substLog]

theorem Env.subst_append (θ : Site → Tr) (A B : Env) : (A ++ B).subst θ = A.subst θ ++ B.subst θ := by
  simp [Env.subst]

theorem Model.subst_append (θ : Site → Tr) (A B : Model) : (A ++ B).subst θ = A.subst θ ++ B.subst θ := by
  simp [Model.subst]

theorem Model.subst_pre (θ : Site → Tr) (p : FId) (m : Model) : (pre p m).subst θ = pre p (m.subst θ) := by
  simp [Model.subst, pre]

theorem lookupNum_subst (θ : Site → Tr) (ρ : Env) (x : Name) :
    lookupNum (ρ.subst θ) x = (lookupNum ρ x).map (Tr.subst θ) := by
  unfold lookupNum
  rw [Env.subst, lookup_ren]
  cases ρ.lookup x with
  | none => rfl
  | some b => cases b <;> rfl

theorem lookupRef_subst (θ : Site → Tr) (ρ : Env) (x : Name) (rel : FId) :
    lookupRef (ρ.subst θ) x rel = (lookupRef ρ x rel).map (Tr.subst θ) := by
  unfold lookupRef
  rw [Env.subst, lookup_ren]
  cases ρ.lookup x with
  | none => rfl
  | some b =>
    cases b with
    | num _ => rfl
    | frag m =>
      simp only [Option.map_some, Bind.subst, Model.subst]
      rw [fid_lookup_ren]
      cases m.lookup rel <;> rfl

@[simp] theorem map_error {α β : Type} (f : α → β) (e : Err) : (Except.error e : R α).map f = .error e := rfl
@[simp] theorem map_ok {α β : Type} (f : α → β) (a : α) : (Except.ok a : R α).map f = .ok (f a) := rfl

theorem toIx_den {θ : Site → Tr} {σA σB : Site → Int} (hσ : ∀ s, σA s = (θ s).den σB) (t : Tr) :
    toIx ((t.subst θ).den σB) = toIx (t.den σA) := by rw [Tr.den_subst hσ]

structure SubstAt (θ : Site → Tr) (σA σB : Site → Int) (n : Nat) : Prop where
  e : ∀ {D ρ x}, FixT θ (codeE x ++ D.code) →
    evalE n D σB (ρ.subst θ) x = (evalE n D σA ρ x).map (fE θ)
  one : ∀ {D ρ x}, FixT θ (codeS x ++ D.code) →
    execOne n D σB (ρ.subst θ) x = (execOne n D σA ρ x).map (fS θ)
  s : ∀ {D ρ x}, FixT θ (codeB x ++ D.code) →
    execS n D σB (ρ.subst θ) x = (execS n D σA ρ x).map (fS θ)
  ln : ∀ {D ρ i b ks}, FixT θ (codeB b ++ D.code) →
    loopN n D σB (ρ.subst θ) i b ks = (loopN n D σA ρ i b ks).map (fL θ)
  lk : ∀ {D ρ i b kvs}, FixT θ (codeKV kvs ++ codeB b ++ D.code) →
    loopK n D σB (ρ.subst θ) i b kvs = (loopK n D σA ρ i b kvs).map (fL θ)

macro "fxt" fix:term : term => `(FixT.mono $fix (fun s hs => by simp [codeE, codeS, codeB, codeKV] at hs ⊢; grind))

theorem substEq {θ : Site → Tr} {σA σB : Site → Int} (hσ : ∀ s, σA s = (θ s).den σB) :
    ∀ n, SubstAt θ σA σB n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp [evalE, execOne, execS, loopN, loopK]
  | succ n ih =>
    constructor
    · intro D ρ x fix
      cases x with
      | lit s =>
        simp [evalE, fE, Tr.subst, fix s (by simp [codeE]), substLog]
      | cst c => simp [evalE, fE, Tr.subst, substLog]
      | var x =>
        simp only [evalE, lookupNum_subst]
        cases lookupNum ρ x <;> simp [fE, substLog]
      | ref x rel =>
        simp only [evalE, lookupRef_subst]
        cases lookupRef ρ x rel <;> simp [fE, substLog]
      | add a b | mul a b | eq a b =>
        simp only [evalE]
        rw [ih.e (fxt fix), ih.e (fxt fix)]
        cases evalE n D σA ρ a with
        | error e => rfl
        | ok pa =>
          cases evalE n D σA ρ b with
          | error e => rfl
          | ok pb =>
            simp [fE, Tr.subst, substLog_append, obs_subst, Tr.den_subst hσ]
      | ite c a b =>
        simp only [evalE]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ c with
        | error e => rfl
        | ok pc =>
          obtain ⟨tc, lc⟩ := pc
          simp only [map_ok, ok_bind, fE, Tr.den_subst hσ]
          by_cases hc : tc.den σA = 0
          · simp only [hc, ne_eq, not_true_eq_false, ite_false]
            rw [ih.e (fxt fix)]
            cases evalE n D σA ρ b <;> simp [fE, substLog_append, obs_subst]
          · simp only [hc, ne_eq, not_false_eq_true, ite_true]
            rw [ih.e (fxt fix)]
            cases evalE n D σA ρ a <;> simp [fE, substLog_append, obs_subst]
      | sel es k =>
        simp only [evalE]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ k with
        | error e => rfl
        | ok pk =>
          obtain ⟨tk, lk⟩ := pk
          simp only [map_ok, ok_bind, fE, toIx_den hσ]
          cases toIx (tk.den σA) with
          | error e => rfl
          | ok i =>
            simp only [ok_bind]
            cases he : es[i]? with
            | none => rfl
            | some e' =>
              have hsub := codeE_sub_codeEs (List.mem_of_getElem? he)
              simp only
              rw [ih.e (FixT.mono fix fun s hs => by
                simp only [codeE, List.mem_append] at hs ⊢
                rcases hs with hs | hs
                · exact .inl (.inl (hsub s hs))
                · exact .inr hs)]
              cases evalE n D σA ρ e' <;> simp [fE, substLog_append, obs_subst]
      | call f a =>
        simp only [evalE]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ a with
        | error e => rfl
        | ok pa =>
          obtain ⟨ta, la⟩ := pa
          simp only [map_ok, ok_bind, fE]
          cases hf : D.fns.lookup f with
          | none => rfl
          | some xb =>
            obtain ⟨x, body⟩ := xb
            simp only
            have := ih.e (D := D) (ρ := (x, .num ta) :: ρ) (x := body) (FixT.mono fix fun s hs => by
              simp only [List.mem_append] at hs ⊢
              rcases hs with hs | hs
              · exact .inr (fn_code hf s hs)
              · exact .inr hs)
            rw [show Env.subst θ ((x, Bind.num ta) :: ρ) = (x, Bind.num (ta.subst θ)) :: Env.subst θ ρ
              from rfl] at this
            rw [this]
            cases evalE n D σA ((x, .num ta) :: ρ) body <;> simp [fE, substLog_append]
      | refIx x k rel =>
        simp only [evalE]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ k with
        | error e => rfl
        | ok pk =>
          obtain ⟨tk, lk⟩ := pk
          simp only [map_ok, ok_bind, fE, toIx_den hσ]
          cases toIx (tk.den σA) with
          | error e => rfl
          | ok i =>
            simp only [ok_bind, lookupRef_subst]
            cases lookupRef ρ x (.ix i :: rel) <;> simp [fE, substLog_append, obs_subst]
    · intro D ρ x fix
      cases x with
      | let_ x e | feat x e =>
        simp only [execOne]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ e <;> simp [fE, fS, Env.subst, Bind.subst, Model.subst, Tr.subst]
      | set x e =>
        simp only [execOne, lookupNum_subst]
        cases lookupNum ρ x with
        | error e => rfl
        | ok u =>
          simp only [map_ok, ok_bind]
          rw [ih.e (fxt fix)]
          cases evalE n D σA ρ e <;> simp [fE, fS, Env.subst, Bind.subst, Model.subst]
      | forN x i c body =>
        simp only [execOne]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ c with
        | error e => rfl
        | ok pc =>
          obtain ⟨tc, lc⟩ := pc
          simp only [map_ok, ok_bind, fE, toIx_den hσ]
          cases toIx (tc.den σA) with
          | error e => rfl
          | ok k =>
            simp only [ok_bind]
            rw [ih.ln (fxt fix)]
            cases loopN n D σA ρ i body (List.range k) <;>
              simp [fL, fS, Env.subst, Bind.subst, Model.subst_pre, substLog_append, obs_subst]
      | forK x i kvs body =>
        simp only [execOne]
        rw [ih.lk (fxt fix)]
        cases loopK n D σA ρ i body kvs <;> simp [fL, fS, Env.subst, Bind.subst, Model.subst_pre]
      | ifs c a b =>
        simp only [execOne]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ c with
        | error e => rfl
        | ok pc =>
          obtain ⟨tc, lc⟩ := pc
          simp only [map_ok, ok_bind, fE, Tr.den_subst hσ]
          by_cases hc : tc.den σA = 0
          · simp only [hc, ne_eq, not_true_eq_false, ite_false]
            rw [ih.s (fxt fix)]
            cases execS n D σA ρ b <;> simp [fS, substLog_append, obs_subst, Env.subst]
          · simp only [hc, ne_eq, not_false_eq_true, ite_true]
            rw [ih.s (fxt fix)]
            cases execS n D σA ρ a <;> simp [fS, substLog_append, obs_subst, Env.subst]
      | use x p a =>
        simp only [execOne]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ a with
        | error e => rfl
        | ok pa =>
          obtain ⟨ta, la⟩ := pa
          simp only [map_ok, ok_bind, fE]
          cases hp : D.procs.lookup p with
          | none => rfl
          | some yb =>
            obtain ⟨y, body⟩ := yb
            simp only
            have := ih.s (D := D) (ρ := (y, .num ta) :: ρ) (x := body) (FixT.mono fix fun s hs => by
              simp only [List.mem_append] at hs ⊢
              rcases hs with hs | hs
              · exact .inr (proc_code hp s hs)
              · exact .inr hs)
            rw [show Env.subst θ ((y, Bind.num ta) :: ρ) = (y, Bind.num (ta.subst θ)) :: Env.subst θ ρ
              from rfl] at this
            rw [this]
            cases execS n D σA ((y, .num ta) :: ρ) body <;>
              simp [fS, substLog_append, Env.subst, Bind.subst, Model.subst_pre]
    · intro D ρ x fix
      cases x with
      | nil => simp [execS, fS, Env.subst, Model.subst, substLog]
      | cons s ss =>
        simp only [execS]
        rw [ih.one (fxt fix)]
        cases execOne n D σA ρ s with
        | error e => rfl
        | ok p1 =>
          obtain ⟨Δ1, m1, l1⟩ := p1
          simp only [map_ok, ok_bind, fS]
          have := ih.s (D := D) (ρ := Δ1 ++ ρ) (x := ss) (fxt fix)
          rw [Env.subst_append] at this
          rw [this]
          cases execS n D σA (Δ1 ++ ρ) ss <;>
            simp [fS, Env.subst_append, Model.subst_append, substLog_append]
    · intro D ρ i b ks fix
      cases ks with
      | nil => simp [loopN, fL, Model.subst, substLog]
      | cons k ks =>
        simp only [loopN]
        have := ih.s (D := D) (ρ := (i, .num (.const k)) :: ρ) (x := b) fix
        rw [show Env.subst θ ((i, Bind.num (Tr.const k)) :: ρ) = (i, Bind.num (Tr.const k)) :: Env.subst θ ρ
          from rfl] at this
        rw [this]
        cases execS n D σA ((i, .num (.const k)) :: ρ) b with
        | error e => rfl
        | ok p1 =>
          simp only [map_ok, ok_bind]
          rw [ih.ln fix]
          cases loopN n D σA ρ i b ks <;>
            simp [fL, fS, Model.subst_append, Model.subst_pre, substLog_append]
    · intro D ρ i b kvs fix
      cases kvs with
      | nil => simp [loopK, fL, Model.subst, substLog]
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK]
        rw [ih.e (fxt fix)]
        cases evalE n D σA ρ e with
        | error e => rfl
        | ok pe =>
          obtain ⟨t, l0⟩ := pe
          simp only [map_ok, ok_bind, fE]
          have := ih.s (D := D) (ρ := (i, .num t) :: ρ) (x := b) (fxt fix)
          rw [show Env.subst θ ((i, Bind.num t) :: ρ) = (i, Bind.num (t.subst θ)) :: Env.subst θ ρ
            from rfl] at this
          rw [this]
          cases execS n D σA ((i, .num t) :: ρ) b with
          | error e => rfl
          | ok p1 =>
            simp only [map_ok, ok_bind]
            rw [ih.lk (fxt fix)]
            cases loopK n D σA ρ i b kvs <;>
              simp [fL, fS, Model.subst_append, Model.subst_pre, substLog_append]

end ForgeLang
