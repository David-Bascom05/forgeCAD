import ForgeLang.Ren
import ForgeLang.Complete
import ForgeLang.Errors

/-!
# SplitInstance, in general

The op: in a range loop whose body starts by binding a shared value `w` to a literal `p`,

    x = for i in 0..n { let w = p; rest }

replace that literal by a list param with one fresh literal per instance, each set to `p`'s number:

    x = for i in 0..n { let w = [q0, q1, ...][i]; rest }

`split_doc`: for any document around the loop, any `rest`, any count expression and any
definitions, if the original evaluates then the split one evaluates, to the same ids and the same
numbers; its traces are the old ones with `q j` in place of `p` in instance `j`. Hypotheses: the
`q j` are fresh (no code mentions them, and `p` is not one of them), and the count fits the list
(otherwise the split document is the typed error `range`).
-/

namespace ForgeLang

/-- The new literals all name the old one. -/
def SplitTheta (θ : Site → Site) (qs : List Site) (p : Site) : Prop := ∀ q ∈ qs, θ q = p

theorem let_lit {f D σ ρ w p r} (h : execOne f D σ ρ (.let_ w (.lit p)) = .ok r) :
    r = ([(w, .num (.site p))], [], []) := by
  match f, h with
  | f + 2, h => simpa [execOne, evalE] using h.symm

theorem split_let {f D σ ρ i w} {qs : List Site} {j : Nat} (hj : j < qs.length) :
    execOne (f + 3) D σ ((i, .num (.const j)) :: ρ) (.let_ w (.sel (qs.map .lit) (.var i))) =
      .ok ([(w, .num (.site qs[j]))], [], []) := by
  have h0 : ¬ ((j : Int) < 0) := by omega
  simp [execOne, evalE, lookupNum, List.lookup, toIx, Tr.den, obs, Tr.sites, h0, hj]

def splitBody (w : Name) (qs : List Site) (i : Name) (rest : List Stmt) : List Stmt :=
  .let_ w (.sel (qs.map .lit) (.var i)) :: rest

theorem split_loop {θ σA σB} (hσ : ∀ s, σB s = σA (θ s)) {qs : List Site} {p : Site}
    (hθ : SplitTheta θ qs p) {D : Defs} {ρ : Env} {i w : Name} {rest : List Stmt}
    (fix : Fix θ (codeB rest ++ D.code)) :
    ∀ (ks : List Nat) {n mm l}, (∀ j ∈ ks, j < qs.length) →
      loopN n D σA (ρ.ren θ) i (.let_ w (.lit p) :: rest) ks = .ok (mm, l) →
      ∃ N r', loopN N D σB ρ i (splitBody w qs i rest) ks = .ok r' ∧ (r'.1.ren θ, r'.2.map θ) = (mm, l)
  | [], n, mm, l, _, h => by
    cases n with
    | zero => simp [loopN] at h
    | succ n =>
      simp only [loopN, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact ⟨1, ([], []), by simp [loopN], by simp [Model.ren]⟩
  | j :: ks, n, mm, l, hk, h => by
    cases n with
    | zero => simp [loopN] at h
    | succ n =>
      simp only [loopN, bind_ok] at h
      obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
      cases h
      obtain ⟨Δl, ml, ll, Δr, mr, lr, hl, hr, e⟩ := execS_cons h1
      cases let_lit hl
      have hj : j < qs.length := hk j (by simp)
      have envEq : ([(w, Bind.num (Tr.site p))] ++ (i, Bind.num (Tr.const j)) :: ρ.ren θ) =
          Env.ren θ ((w, .num (.site qs[j])) :: (i, .num (.const j)) :: ρ) := by
        simp [Env.ren, Bind.ren, Tr.ren, hθ _ (List.getElem_mem hj)]
      rw [envEq] at hr
      obtain ⟨⟨Δr', mr', lr'⟩, hr', er⟩ := (ren hσ n).s hr fix
      simp only [Prod.mk.injEq] at er e
      obtain ⟨-, rfl, rfl⟩ := er
      obtain ⟨-, rfl, rfl⟩ := e
      obtain ⟨N1, hN1⟩ := execS_cons_compose (split_let (f := 0) (D := D) (σ := σB) (ρ := ρ) (w := w) hj) hr'
      obtain ⟨N2, ⟨m2', l2'⟩, hN2, e2⟩ := split_loop hσ hθ fix ks (fun j hj => hk j (by simp [hj])) h2
      simp only [Prod.mk.injEq] at e2
      obtain ⟨rfl, rfl⟩ := e2
      refine ⟨max N1 N2 + 1, (pre [.ix j] ([] ++ mr') ++ m2', ([] ++ lr') ++ l2'), ?_, ?_⟩
      · simp only [loopN, splitBody] at hN2 ⊢
        rw [execS_mono hN1 (Nat.le_max_left N1 N2), ok_bind,
          loopN_mono hN2 (Nat.le_max_right N1 N2)]
        rfl
      · simp [Model.ren_append, Model.ren_pre]

/-- The count fits the list: every count the loop can compute is at most the number of literals. -/
def CountFits (D : Defs) (σ : Site → Int) (c : Expr) (qs : List Site) : Prop :=
  ∀ f ρ t l, evalE f D σ ρ c = .ok (t, l) → (t.den σ).toNat ≤ qs.length

theorem split_stmt {θ σA σB} (hσ : ∀ s, σB s = σA (θ s)) {qs : List Site} {p : Site}
    (hθ : SplitTheta θ qs p) {D : Defs} {ρ : Env} {x i w : Name} {c : Expr} {rest : List Stmt}
    (fix : Fix θ (codeE c ++ codeB rest ++ D.code)) (hfit : CountFits D σA c qs) {n r}
    (h : execOne n D σA (ρ.ren θ) (.forN x i c (.let_ w (.lit p) :: rest)) = .ok r) :
    ∃ N r', execOne N D σB ρ (.forN x i c (splitBody w qs i rest)) = .ok r' ∧
      (r'.1.ren θ, r'.2.1.ren θ, r'.2.2.map θ) = r := by
  cases n with
  | zero => simp [execOne] at h
  | succ n =>
    simp only [execOne, bind_ok] at h
    obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
    cases h
    have kfit : k ≤ qs.length := by
      have := hfit _ _ _ _ hc
      unfold toIx at hk
      split at hk
      · cases hk
      · cases hk; exact this
    obtain ⟨⟨tc', lc'⟩, hc', ec⟩ := (ren hσ n).e hc (fx fix fun s hs => by simp at *; grind)
    simp only [Prod.mk.injEq] at ec
    obtain ⟨rfl, rfl⟩ := ec
    rw [Tr.den_ren hσ] at hk
    obtain ⟨N, ⟨mm', l'⟩, hN, el⟩ := split_loop hσ hθ (fx fix fun s hs => by simp at *; grind)
      (List.range k) (fun j hj => by simp at hj; omega) hl
    simp only [Prod.mk.injEq] at el
    obtain ⟨rfl, rfl⟩ := el
    refine ⟨max n N + 1, ([(x, .frag mm')], pre [.nm x] mm', lc' ++ obs D tc' ++ l'), ?_, ?_⟩
    · simp only [execOne]
      rw [evalE_mono hc' (Nat.le_max_left n N), ok_bind, hk, ok_bind, loopN_mono hN (Nat.le_max_right n N)]
      rfl
    · simp [Env.ren, Bind.ren, Model.ren_pre, obs_ren]

/-- SplitInstance on a whole document: the split document evaluates, with the same ids and the
same numbers; its traces are the old ones up to `θ`. -/
theorem split_doc {θ σA σB} (hσ : ∀ s, σB s = σA (θ s)) {qs : List Site} {p : Site}
    (hθ : SplitTheta θ qs p) {D : Defs} {xs ys rest : List Stmt} {x i w : Name} {c : Expr}
    (fix : Fix θ (codeB xs ++ codeE c ++ codeB rest ++ codeB ys ++ D.code)) (hfit : CountFits D σA c qs)
    {n Δ m l} (h : execS n D σA [] (xs ++ .forN x i c (.let_ w (.lit p) :: rest) :: ys) = .ok (Δ, m, l)) :
    ∃ N Δ' m' l', execS N D σB [] (xs ++ .forN x i c (splitBody w qs i rest) :: ys) = .ok (Δ', m', l') ∧
      m'.ren θ = m ∧ l'.map θ = l ∧ m'.nums σB = m.nums σA := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append _ _ h
  obtain ⟨ΔS, mS, lS, Δ3, m3, l3, hS, h3, e2⟩ := execS_cons h2
  obtain ⟨⟨Δ1', m1', l1'⟩, h1', e1⟩ := (ren hσ n).s (ρ := []) h1 (fx fix fun s hs => by simp at *; grind)
  simp only [Prod.mk.injEq] at e1
  obtain ⟨rfl, rfl, rfl⟩ := e1
  have hS2 : execOne n D σA (Env.ren θ (Δ1' ++ [])) (.forN x i c (.let_ w (.lit p) :: rest)) =
      .ok (ΔS, mS, lS) := by
    simpa [Env.ren_append] using hS
  obtain ⟨N2, ⟨ΔS', mS', lS'⟩, hS', eS⟩ := split_stmt hσ hθ (fx fix fun s hs => by simp at *; grind) hfit hS2
  simp only [Prod.mk.injEq] at eS
  obtain ⟨rfl, rfl, rfl⟩ := eS
  have h3b : execS n D σA (Env.ren θ (ΔS' ++ (Δ1' ++ []))) ys = .ok (Δ3, m3, l3) := by
    simpa [Env.ren_append] using h3
  obtain ⟨⟨Δ3', m3', l3'⟩, h3', e3⟩ := (ren hσ n).s h3b (fx fix fun s hs => by simp at *; grind)
  simp only [Prod.mk.injEq] at e3
  obtain ⟨-, rfl, rfl⟩ := e3
  obtain ⟨Na, ha⟩ := execS_cons_compose hS' h3'
  obtain ⟨N, hN⟩ := execS_compose' h1' ha
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  refine ⟨N, _, _, _, hN, by simp [Model.ren_append], by simp, ?_⟩
  simp only [Model.nums, List.map_append, Model.ren, List.map_map]
  simp [Function.comp_def, Tr.den_ren hσ]

end ForgeLang
