import ForgeLang.Complete

/-!
# Function scope: call-site scope equals lexical scope under one rule

The core gives function and procedure bodies the environment at the call. The lexical alternative
(what FastCAD documents: "functions see top-level lets and params") gives them the *top-level*
environment `G` instead. `execTop` runs the body's top-level statements, setting `G` to the
environment at each one; nested blocks keep it.

`scope_equiv`: the two semantics are equal, result for result and error for error, under one rule:
no *local* name (loop variable, function or procedure parameter, anything bound inside a nested
block or a procedure body) is a name some function or procedure body reads (`Defs.reads`). That
single rule also gives "a body mentions only its parameters and top-level names", because any other
name a body mentions would have to be local somewhere. Top-level names may be rebound freely as far
as this theorem is concerned. `scope_differs` shows the rule is needed.
-/

namespace ForgeLang

mutual

def evalEL : Nat → Defs → (Site → Int) → Env → Env → Expr → R (Tr × Log)
  | 0, _, _, _, _, _ => .error .fuel
  | n + 1, D, σ, G, ρ, e =>
    match e with
    | .lit s => .ok (.site s, [])
    | .cst c => .ok (.const c, [])
    | .var x => do let t ← lookupNum ρ x; .ok (t, [])
    | .add a b => do
      let (ta, la) ← evalEL n D σ G ρ a
      let (tb, lb) ← evalEL n D σ G ρ b
      .ok (.add ta tb, la ++ lb)
    | .mul a b => do
      let (ta, la) ← evalEL n D σ G ρ a
      let (tb, lb) ← evalEL n D σ G ρ b
      .ok (.mul ta tb, la ++ lb)
    | .eq a b => do
      let (ta, la) ← evalEL n D σ G ρ a
      let (tb, lb) ← evalEL n D σ G ρ b
      .ok (.const (if ta.den σ = tb.den σ then 1 else 0), la ++ lb ++ obs D ta ++ obs D tb)
    | .ite c a b => do
      let (tc, lc) ← evalEL n D σ G ρ c
      let (t, l) ← evalEL n D σ G ρ (if tc.den σ ≠ 0 then a else b)
      .ok (t, lc ++ obs D tc ++ l)
    | .sel es k => do
      let (tk, lk) ← evalEL n D σ G ρ k
      let i ← toIx (tk.den σ)
      match es[i]? with
      | some e' => do
        let (t, l) ← evalEL n D σ G ρ e'
        .ok (t, lk ++ obs D tk ++ l)
      | none => .error .range
    | .call f a => do
      let (ta, la) ← evalEL n D σ G ρ a
      match D.fns.lookup f with
      | some (x, body) => do
        let (t, l) ← evalEL n D σ G ((x, .num ta) :: G) body
        .ok (t, la ++ l)
      | none => .error (.noFn f)
    | .ref x rel => do let t ← lookupRef ρ x rel; .ok (t, [])
    | .refIx x k rel => do
      let (tk, lk) ← evalEL n D σ G ρ k
      let i ← toIx (tk.den σ)
      let t ← lookupRef ρ x (.ix i :: rel)
      .ok (t, lk ++ obs D tk)

def execOneL : Nat → Defs → (Site → Int) → Env → Env → Stmt → R (Env × Model × Log)
  | 0, _, _, _, _, _ => .error .fuel
  | n + 1, D, σ, G, ρ, s =>
    match s with
    | .let_ x e => do
      let (t, l) ← evalEL n D σ G ρ e
      .ok ([(x, .num t)], [], l)
    | .set x e => do
      let _ ← lookupNum ρ x
      let (t, l) ← evalEL n D σ G ρ e
      .ok ([(x, .num t)], [], l)
    | .feat x e => do
      let (t, l) ← evalEL n D σ G ρ e
      .ok ([(x, .frag [([], t)])], [([.nm x], t)], l)
    | .forN x i c body => do
      let (tc, lc) ← evalEL n D σ G ρ c
      let k ← toIx (tc.den σ)
      let (m, l) ← loopNL n D σ G ρ i body (List.range k)
      .ok ([(x, .frag m)], pre [.nm x] m, lc ++ obs D tc ++ l)
    | .forK x i kvs body => do
      let (m, l) ← loopKL n D σ G ρ i body kvs
      .ok ([(x, .frag m)], pre [.nm x] m, l)
    | .ifs c a b => do
      let (tc, lc) ← evalEL n D σ G ρ c
      let (_, m, l) ← execSL n D σ G ρ (if tc.den σ ≠ 0 then a else b)
      .ok ([], m, lc ++ obs D tc ++ l)
    | .use x p a => do
      let (ta, la) ← evalEL n D σ G ρ a
      match D.procs.lookup p with
      | some (y, body) => do
        let (_, m, l) ← execSL n D σ G ((y, .num ta) :: G) body
        .ok ([(x, .frag m)], pre [.nm x] m, la ++ l)
      | none => .error (.noFn p)

def execSL : Nat → Defs → (Site → Int) → Env → Env → List Stmt → R (Env × Model × Log)
  | 0, _, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, _, [] => .ok ([], [], [])
  | n + 1, D, σ, G, ρ, s :: ss => do
    let (Δ1, m1, l1) ← execOneL n D σ G ρ s
    let (Δ2, m2, l2) ← execSL n D σ G (Δ1 ++ ρ) ss
    .ok (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2)

def loopNL : Nat → Defs → (Site → Int) → Env → Env → Name → List Stmt → List Nat → R (Model × Log)
  | 0, _, _, _, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, _, _, _, [] => .ok ([], [])
  | n + 1, D, σ, G, ρ, i, body, k :: ks => do
    let (_, m1, l1) ← execSL n D σ G ((i, .num (.const k)) :: ρ) body
    let (m2, l2) ← loopNL n D σ G ρ i body ks
    .ok (pre [.ix k] m1 ++ m2, l1 ++ l2)

def loopKL : Nat → Defs → (Site → Int) → Env → Env → Name → List Stmt → List (Name × Expr) → R (Model × Log)
  | 0, _, _, _, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, _, _, _, [] => .ok ([], [])
  | n + 1, D, σ, G, ρ, i, body, (k, e) :: kvs => do
    let (t, l0) ← evalEL n D σ G ρ e
    let (_, m1, l1) ← execSL n D σ G ((i, .num t) :: ρ) body
    let (m2, l2) ← loopKL n D σ G ρ i body kvs
    .ok (pre [.key k] m1 ++ m2, l0 ++ l1 ++ l2)

end

def execTop : Nat → Defs → (Site → Int) → Env → List Stmt → R (Env × Model × Log)
  | 0, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, [] => .ok ([], [], [])
  | n + 1, D, σ, ρ, s :: ss => do
    let (Δ1, m1, l1) ← execOneL n D σ ρ ρ s
    let (Δ2, m2, l2) ← execTop n D σ (Δ1 ++ ρ) ss
    .ok (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2)

/-- The whole evaluation with lexical scope. -/
def runL (fuel : Nat) (d : Doc) : R (Model × Log) := do
  let (_, m, l) ← execTop fuel d.defs d.σ [] d.body
  if (m.map Prod.fst).Nodup then .ok (m, l) else .error .dupId

mutual
/-- Every name bound in a block: its statements' own binders and everything nested. -/
def allB : List Stmt → List Name
  | [] => []
  | s :: ss => s.binder.toList ++ locS s ++ allB ss
/-- Names bound inside a statement's nested blocks (not its own binder). -/
def locS : Stmt → List Name
  | .forN _ i _ b | .forK _ i _ b => i :: allB b
  | .ifs _ a b => allB a ++ allB b
  | _ => []
end

/-- The rule: no local name is one a body reads. -/
def DefsOK (D : Defs) : Prop :=
  (∀ f ∈ D.fns, f.2.1 ∉ D.reads) ∧ (∀ p ∈ D.procs, p.2.1 ∉ D.reads ∧ ∀ y ∈ allB p.2.2, y ∉ D.reads)

def NoReads (D : Defs) (N : List Name) : Prop := ∀ y ∈ N, y ∉ D.reads

theorem AgreeOn.ext_fresh {N : List Name} {G ρ Δ : Env} (h : AgreeOn N G ρ) (hΔ : ∀ y ∈ keys Δ, y ∉ N) :
    AgreeOn N G (Δ ++ ρ) := by
  intro y hy
  rw [lookup_append, lookup_none (fun hk => hΔ y hk hy)]
  exact h y hy

theorem nr_l {D : Defs} {A B : List Name} (h : NoReads D (A ++ B)) : NoReads D A :=
  fun y hy => h y (List.mem_append_left _ hy)
theorem nr_r {D : Defs} {A B : List Name} (h : NoReads D (A ++ B)) : NoReads D B :=
  fun y hy => h y (List.mem_append_right _ hy)

structure ScopeAt (n : Nat) : Prop where
  e : ∀ {D σ G ρL ρE x}, DefsOK D → AgreeOn (fvE x ++ D.reads) ρL ρE → AgreeOn D.reads G ρE →
    evalEL n D σ G ρL x = evalE n D σ ρE x
  one : ∀ {D σ G ρL ρE x}, DefsOK D → NoReads D (locS x) → AgreeOn (fvS x ++ D.reads) ρL ρE →
    AgreeOn D.reads G ρE → execOneL n D σ G ρL x = execOne n D σ ρE x
  s : ∀ {D σ G ρL ρE x}, DefsOK D → NoReads D (allB x) → AgreeOn (fvB x ++ D.reads) ρL ρE →
    AgreeOn D.reads G ρE → execSL n D σ G ρL x = execS n D σ ρE x
  ln : ∀ {D σ G ρL ρE i b ks}, DefsOK D → NoReads D (i :: allB b) → AgreeOn (fvB b ++ D.reads) ρL ρE →
    AgreeOn D.reads G ρE → loopNL n D σ G ρL i b ks = loopN n D σ ρE i b ks
  lk : ∀ {D σ G ρL ρE i b kvs}, DefsOK D → NoReads D (i :: allB b) →
    AgreeOn (fvKV kvs ++ fvB b ++ D.reads) ρL ρE → AgreeOn D.reads G ρE →
    loopKL n D σ G ρL i b kvs = loopK n D σ ρE i b kvs

macro "agm" h:term : term => `(AgreeOn.mono $h (fun y hy => by simp [fvE, fvS, fvB, fvKV] at hy ⊢; grind))

theorem scopeAt : ∀ n, ScopeAt n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> rfl
  | succ n ih =>
    constructor
    · intro D σ G ρL ρE x dok ag hG
      cases x with
      | lit s => rfl
      | cst c => rfl
      | var x =>
        simp only [evalEL, evalE, lookupNum_agree (ag x (by simp [fvE]))]
      | ref x rel =>
        simp only [evalEL, evalE, lookupRef_agree (rel := rel) (ag x (by simp [fvE]))]
      | add a b | mul a b | eq a b =>
        simp only [evalEL, evalE]
        rw [ih.e dok (agm ag) hG, ih.e dok (agm ag) hG]
      | ite c a b =>
        simp only [evalEL, evalE]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE c with
        | error e => rfl
        | ok pc =>
          simp only [ok_bind]
          rw [ih.e dok (by split <;> exact agm ag) hG]
      | sel es k =>
        simp only [evalEL, evalE]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE k with
        | error e => rfl
        | ok pk =>
          simp only [ok_bind]
          cases toIx (pk.1.den σ) with
          | error e => rfl
          | ok i =>
            simp only [ok_bind]
            cases he : es[i]? with
            | none => rfl
            | some e' =>
              have hsub := fvE_sub_fvEs (List.mem_of_getElem? he)
              simp only
              rw [ih.e dok (AgreeOn.mono ag fun y hy => by
                simp only [fvE, List.mem_append] at hy ⊢
                rcases hy with hy | hy
                · exact .inl (.inl (hsub y hy))
                · exact .inr hy) hG]
      | call f a =>
        simp only [evalEL, evalE]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE a with
        | error e => rfl
        | ok pa =>
          simp only [ok_bind]
          cases hf : D.fns.lookup f with
          | none => rfl
          | some xb =>
            obtain ⟨x, body⟩ := xb
            simp only
            have hx : x ∉ D.reads := dok.1 (f, x, body) (lookup_mem hf)
            rw [ih.e dok (agree_body (fn_reads hf) hG)
              (hG.ext_fresh (Δ := [(x, .num pa.1)]) (by intro y hy; simp [keys] at hy; subst hy; exact hx))]
      | refIx x k rel =>
        simp only [evalEL, evalE]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE k with
        | error e => rfl
        | ok pk =>
          simp only [ok_bind]
          cases toIx (pk.1.den σ) with
          | error e => rfl
          | ok i =>
            simp only [ok_bind, lookupRef_agree (rel := .ix i :: rel) (ag x (by simp [fvE]))]
    · intro D σ G ρL ρE x dok nr ag hG
      cases x with
      | let_ x e | feat x e =>
        simp only [execOneL, execOne]
        rw [ih.e dok (agm ag) hG]
      | set x e =>
        simp only [execOneL, execOne, lookupNum_agree (ag x (by simp [fvS]))]
        rw [ih.e dok (agm ag) hG]
      | forN x i c body =>
        simp only [locS] at nr
        simp only [execOneL, execOne]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE c with
        | error e => rfl
        | ok pc =>
          simp only [ok_bind]
          cases toIx (pc.1.den σ) with
          | error e => rfl
          | ok k => simp only [ok_bind]; rw [ih.ln dok nr (agm ag) hG]
      | forK x i kvs body =>
        simp only [locS] at nr
        simp only [execOneL, execOne]
        rw [ih.lk dok nr (agm ag) hG]
      | ifs c a b =>
        simp only [locS] at nr
        simp only [execOneL, execOne]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE c with
        | error e => rfl
        | ok pc =>
          simp only [ok_bind]
          rw [ih.s dok (by split; exact nr_l nr; exact nr_r nr) (by split <;> exact agm ag) hG]
      | use x p a =>
        simp only [execOneL, execOne]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE a with
        | error e => rfl
        | ok pa =>
          simp only [ok_bind]
          cases hp : D.procs.lookup p with
          | none => rfl
          | some yb =>
            obtain ⟨y, body⟩ := yb
            simp only
            have hy := dok.2 (p, y, body) (lookup_mem hp)
            rw [ih.s dok hy.2 (agree_body (proc_reads hp) hG)
              (hG.ext_fresh (Δ := [(y, .num pa.1)]) (by intro z hz; simp [keys] at hz; subst hz; exact hy.1))]
    · intro D σ G ρL ρE x dok nr ag hG
      cases x with
      | nil => rfl
      | cons s ss =>
        simp only [allB] at nr
        simp only [execSL, execS]
        rw [ih.one dok (nr_r (nr_l nr)) (agm ag) hG]
        cases h1 : execOne n D σ ρE s with
        | error e => rfl
        | ok p1 =>
          obtain ⟨Δ1, m1, l1⟩ := p1
          simp only [ok_bind]
          rw [ih.s dok (nr_r nr) ((agm ag).append Δ1) (hG.ext_fresh (by
            intro y hy
            exact nr_l (nr_l nr) y (by simp [execOne_keys h1 y hy])))]
    · intro D σ G ρL ρE i b ks dok nr ag hG
      cases ks with
      | nil => rfl
      | cons k ks =>
        simp only [loopNL, loopN]
        have hi : i ∉ D.reads := nr i (by simp)
        rw [ih.s dok (fun y hy => nr y (by simp [hy])) (ag.cons _)
          (hG.ext_fresh (Δ := [(i, .num (.const k))]) (by intro z hz; simp [keys] at hz; subst hz; exact hi))]
        cases execS n D σ ((i, .num (.const k)) :: ρE) b with
        | error e => rfl
        | ok p1 => simp only [ok_bind]; rw [ih.ln dok nr ag hG]
    · intro D σ G ρL ρE i b kvs dok nr ag hG
      cases kvs with
      | nil => rfl
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopKL, loopK]
        rw [ih.e dok (agm ag) hG]
        cases evalE n D σ ρE e with
        | error e => rfl
        | ok pe =>
          simp only [ok_bind]
          have hi : i ∉ D.reads := nr i (by simp)
          rw [ih.s dok (fun y hy => nr y (by simp [hy])) ((agm ag).cons _)
            (hG.ext_fresh (Δ := [(i, .num pe.1)]) (by intro z hz; simp [keys] at hz; subst hz; exact hi))]
          cases execS n D σ ((i, .num pe.1) :: ρE) b with
          | error e => rfl
          | ok p1 => simp only [ok_bind]; rw [ih.lk dok nr (agm ag) hG]

theorem execTop_eq : ∀ n {D σ ρ ss}, DefsOK D → (∀ s ∈ ss, NoReads D (locS s)) →
    execTop n D σ ρ ss = execS n D σ ρ ss
  | 0, _, _, _, _, _, _ => rfl
  | _ + 1, _, _, _, [], _, _ => rfl
  | n + 1, D, σ, ρ, s :: ss, dok, h => by
    simp only [execTop, execS]
    rw [(scopeAt n).one dok (h s (by simp)) (fun _ _ => rfl) (fun _ _ => rfl)]
    cases execOne n D σ ρ s with
    | error e => rfl
    | ok p1 => simp only [ok_bind]; rw [execTop_eq n dok (fun s hs => h s (by simp [hs]))]

/-- Call-site scope equals lexical scope, under the rule. -/
theorem scope_equiv (fuel : Nat) (d : Doc) (dok : DefsOK d.defs)
    (hb : ∀ s ∈ d.body, NoReads d.defs (locS s)) : runL fuel d = run fuel d := by
  simp only [runL, run, execTop_eq fuel dok hb]

/-- Without the rule they differ: a function reads top-level `w`, and is called inside a loop whose
variable is also `w`. Call-site scope sees the loop's `w`; lexical scope sees the top-level one. -/
def shadowDoc : Doc where
  defs := { fns := [("f", ("x", .var "w"))] }
  body := [.let_ "w" (.lit 0), .forN "h" "w" (.cst 2) [.feat "b" (.call "f" (.cst 0))]]
  σ := fun _ => 7

theorem scope_differs :
    run 30 shadowDoc = .ok ([([.nm "h", .ix 0, .nm "b"], .const 0), ([.nm "h", .ix 1, .nm "b"], .const 1)], []) ∧
    runL 30 shadowDoc = .ok ([([.nm "h", .ix 0, .nm "b"], .site 0), ([.nm "h", .ix 1, .nm "b"], .site 0)], []) :=
  ⟨by decide, by decide⟩

theorem shadowDoc_breaks_rule : ¬ NoReads shadowDoc.defs (locS (shadowDoc.body[1]'(by decide))) := by
  intro h; exact h "w" (by decide) (by decide)

end ForgeLang
