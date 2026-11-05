import ForgeLang.NestedOps

/-!
# Scoped ops at any depth, in loops and conditionals, placed any number of times

`NestedOps.lift_occ` lifts an edit of component `c0` through blocks that place it directly, once.
Here the whole evaluator is lifted at once: under `D' = D.withComp c0 C'`, every statement list,
statement and loop evaluates as before, its model differing only in pieces that the op relates
(`MR`: equal, or put together from related pieces, under any prefix), and its log likewise (`LR`).
A component placed inside a `for`, a keyed `for` or an `if` of a block, or several times, at any
depth, is covered.

The condition is the design's rule R2, per block (`R2B`): a statement that can reach `c0` (places
it, or a component that does, at any depth inside it) binds a name nothing after it in the block
(nor a function, nor a component's visible reads, before or after the edit) mentions. Without it a
reader of the edited fragment dangles (`Counter.nested_delete_dangles`). Every component block must
keep the rule (`hbodies`), the new block must not reach `c0` (no cycle), and procedures place
nothing that reaches it.
-/

namespace ForgeLang

/-- Two models equal but for pieces related by `Rm`. -/
inductive MR (Rm : Model → Model → Prop) : Model → Model → Prop
  | refl (m : Model) : MR Rm m m
  | app {a a' b b'} : MR Rm a a' → MR Rm b b' → MR Rm (a ++ b) (a' ++ b')
  | pre (p : FId) {a a'} : MR Rm a a' → MR Rm (pre p a) (pre p a')
  | op {a a'} : Rm a a' → MR Rm a a'

/-- Two logs equal but for pieces related by `Rl`. -/
inductive LR (Rl : Log → Log → Prop) : Log → Log → Prop
  | refl (l : Log) : LR Rl l l
  | app {a a' b b'} : LR Rl a a' → LR Rl b b' → LR Rl (a ++ b) (a' ++ b')
  | op {a a'} : Rl a a' → LR Rl a a'

section
variable (S : Name → Prop) (Rd : List Name)

/-- A statement that can reach `c0`: it places a component outside `S`. -/
def Taints (s : Stmt) : Prop := ∃ e ∈ placesS s, ¬ S e

mutual
/-- R2 per block, at every level of loops and conditionals. -/
def R2S : Stmt → Prop
  | .forN _ _ _ b | .forK _ _ _ b => R2B b
  | .ifs _ a b => R2B a ∧ R2B b
  | _ => True
def R2B : List Stmt → Prop
  | [] => True
  | s :: ss => (Taints S s → NotRead s (fvB ss ++ Rd)) ∧ R2S s ∧ R2B ss
end
end

theorem mr_app3 {Rm} {a a' b b' c c'} (h1 : MR Rm a a') (h2 : MR Rm b b') (h3 : MR Rm c c') :
    MR Rm (a ++ b ++ c) (a' ++ b' ++ c') := .app (.app h1 h2) h3

/-- The lift, at fuel `n`. -/
structure LiftAll (D : Defs) (c0 : Name) (C' : Comp) (S : Name → Prop) (Rm : Model → Model → Prop)
    (Rl : Log → Log → Prop) (n : Nat) : Prop where
  one : ∀ {σ ρ x Δ m l}, R2S S (D.reads ++ (D.withComp c0 C').reads) x →
    execOne n D σ ρ x = .ok (Δ, m, l) →
    ∃ k Δ' m' l', execOne k (D.withComp c0 C') σ ρ x = .ok (Δ', m', l') ∧ MR Rm m m' ∧ LR Rl l l' ∧
      (¬ Taints S x → Δ' = Δ)
  s : ∀ {σ ρ x Δ m l}, R2B S (D.reads ++ (D.withComp c0 C').reads) x →
    execS n D σ ρ x = .ok (Δ, m, l) →
    ∃ k Δ' m' l', execS k (D.withComp c0 C') σ ρ x = .ok (Δ', m', l') ∧ MR Rm m m' ∧ LR Rl l l'
  ln : ∀ {σ ρ i b ks m l}, R2B S (D.reads ++ (D.withComp c0 C').reads) b →
    loopN n D σ ρ i b ks = .ok (m, l) →
    ∃ k m' l', loopN k (D.withComp c0 C') σ ρ i b ks = .ok (m', l') ∧ MR Rm m m' ∧ LR Rl l l'
  lk : ∀ {σ ρ i b kvs m l}, R2B S (D.reads ++ (D.withComp c0 C').reads) b →
    loopK n D σ ρ i b kvs = .ok (m, l) →
    ∃ k m' l', loopK k (D.withComp c0 C') σ ρ i b kvs = .ok (m', l') ∧ MR Rm m m' ∧ LR Rl l l'

theorem not_taints_places {S : Name → Prop} {x : Stmt} (h : ¬ Taints S x) : ∀ e ∈ placesS x, S e :=
  fun e he => Classical.byContradiction fun hn => h ⟨e, he, hn⟩

theorem taints_of_placesB {S : Name → Prop} {b : List Stmt} (h : ¬ ∀ e ∈ placesB b, S e) :
    ∃ e ∈ placesB b, ¬ S e :=
  Classical.byContradiction fun hn => h fun e he => Classical.byContradiction fun hs => hn ⟨e, he, hs⟩

/-- The whole evaluator under the edit: every run that finished finishes after it, equal but for
the op's pieces, wherever (in loops, conditionals, at any depth) and however often `c0` is placed. -/
theorem lift_all {D : Defs} {c0 : Name} {C C' : Comp} {S : Name → Prop} {Rm : Model → Model → Prop}
    {Rl : Log → Log → Prop} (hA : Avoids D c0 S) (hc : D.comps.lookup c0 = some C)
    (hp : C'.params = C.params) (hv : C'.vis = C.vis) (hb : ∀ e ∈ placesB C'.body, S e)
    (hop : ∀ {n σ ρ Δ m l}, execS n D σ ρ C.body = .ok (Δ, m, l) →
      ∃ N Δ' m' l', execS N D σ ρ C'.body = .ok (Δ', m', l') ∧ Rm m m' ∧ Rl l l')
    (hbodies : ∀ c Cc, D.comps.lookup c = some Cc → c ≠ c0 → R2B S (D.reads ++ (D.withComp c0 C').reads) Cc.body) :
    ∀ n, LiftAll D c0 C' S Rm Rl n := by
  intro n
  induction n with
  | zero =>
    constructor
    · intro σ ρ x Δ m l _ h; simp [execOne] at h
    · intro σ ρ x Δ m l _ h; simp [execS] at h
    · intro σ ρ i b ks m l _ h; simp [loopN] at h
    · intro σ ρ i b kvs m l _ h; simp [loopK] at h
  | succ n ih =>
    have cong := defs_congr_closed (C' := C') hA
    -- a statement that cannot reach `c0` runs the same
    have same : ∀ {σ ρ x Δ m l}, ¬ Taints S x → execOne (n + 1) D σ ρ x = .ok (Δ, m, l) →
        ∃ k Δ' m' l', execOne k (D.withComp c0 C') σ ρ x = .ok (Δ', m', l') ∧ MR Rm m m' ∧ LR Rl l l' ∧
          (¬ Taints S x → Δ' = Δ) := by
      intro σ ρ x Δ m l ht h
      refine ⟨n + 1, Δ, m, l, ?_, .refl _, .refl _, fun _ => rfl⟩
      rw [(cong (n + 1)).one (not_taints_places ht)]; exact h
    constructor
    · intro σ ρ x Δ m l hr h
      by_cases ht : Taints S x
      · cases x with
        | let_ | set | feat | use =>
          exfalso; obtain ⟨e, he, _⟩ := ht; simp [placesS] at he
        | forN x i c b =>
          simp only [execOne, bind_ok] at h
          obtain ⟨⟨tc, lc⟩, hce, k0, hk, ⟨mb, lb⟩, hl, h⟩ := h
          cases h
          obtain ⟨K, mb', lb', hK, hm, hlr⟩ := ih.ln hr hl
          refine ⟨max n K + 1, [(x, .frag mb')], pre [.nm x] mb', lc ++ obs (D.withComp c0 C') tc ++ lb', ?_,
            .pre _ hm, .app (.refl _) hlr, fun h' => absurd ht h'⟩
          simp only [execOne, (cong _).e, evalE_mono hce (Nat.le_max_left n K), ok_bind, hk,
            loopN_mono hK (Nat.le_max_right n K), obs_withComp]
        | forK x i kvs b =>
          simp only [execOne, bind_ok] at h
          obtain ⟨⟨mb, lb⟩, hl, h⟩ := h
          cases h
          obtain ⟨K, mb', lb', hK, hm, hlr⟩ := ih.lk hr hl
          exact ⟨K + 1, [(x, .frag mb')], pre [.nm x] mb', lb', by simp [execOne, hK], .pre _ hm, hlr,
            fun h' => absurd ht h'⟩
        | ifs c a b =>
          simp only [execOne, bind_ok] at h
          obtain ⟨⟨tc, lc⟩, hce, ⟨Δb, mb, lb⟩, hbr, h⟩ := h
          cases h
          obtain ⟨hra, hrb⟩ := hr
          have hrr : R2B S (D.reads ++ (D.withComp c0 C').reads) (if tc.den σ ≠ 0 then a else b) := by
            split <;> assumption
          obtain ⟨K, Δb', mb', lb', hK, hm, hlr⟩ := ih.s hrr hbr
          refine ⟨max n K + 1, [], mb', lc ++ obs (D.withComp c0 C') tc ++ lb', ?_, hm, .app (.refl _) hlr,
            fun h' => absurd ht h'⟩
          simp only [execOne, (cong _).e, evalE_mono hce (Nat.le_max_left n K), ok_bind, obs_withComp,
            execS_mono hK (Nat.le_max_right n K)]
        | occ x c args =>
          by_cases e0 : c = c0
          · subst e0
            rw [execOne_occ hc] at h
            unfold occRun at h
            split at h
            · rename_i hok
              simp only [bind_ok] at h
              obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, lb⟩, hbr, h⟩ := h
              cases h
              obtain ⟨K, Δ', mm', lb', hK, hm, hl⟩ := hop hbr
              refine ⟨max n K + 1, [(x, .frag mm')], pre [.nm x] mm', lg ++ ld ++ lb', ?_,
                .pre _ (.op hm), .app (.refl _) (.op hl), fun h' => absurd ht h'⟩
              rw [scoped_occ_closed hA hb]
              simp only [occRun, hp, hv, hok, ite_true, evalA_mono hg (Nat.le_max_left n K),
                evalA_mono hd (Nat.le_max_left n K), ok_bind]
              rw [execS_mono hK (Nat.le_max_right n K)]
              rfl
            · cases h
          · cases hcl : D.comps.lookup c with
            | none => simp [execOne, hcl] at h
            | some Cc =>
              rw [execOne_occ hcl] at h
              unfold occRun at h
              split at h
              · rename_i hok
                simp only [bind_ok] at h
                obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, lb⟩, hbr, h⟩ := h
                cases h
                obtain ⟨K, Δ', mm', lb', hK, hm, hl⟩ := ih.s (hbodies c Cc hcl e0) hbr
                have hcl' : (D.withComp c0 C').comps.lookup c = some Cc := by
                  rw [withComp_lookup_other D C' e0]; exact hcl
                refine ⟨max n K + 1, [(x, .frag mm')], pre [.nm x] mm', lg ++ ld ++ lb', ?_,
                  .pre _ hm, .app (.refl _) hl, fun h' => absurd ht h'⟩
                rw [execOne_occ hcl']
                simp only [occRun, hok, ite_true, (cong _).a, evalA_mono hg (Nat.le_max_left n K),
                  evalA_mono hd (Nat.le_max_left n K), ok_bind]
                rw [execS_mono hK (Nat.le_max_right n K)]
                rfl
              · cases h
      · exact same ht h
    · intro σ ρ x Δ m l hr h
      cases x with
      | nil => exact ⟨n + 1, Δ, m, l, by simp [execS] at h ⊢; exact h, .refl _, .refl _⟩
      | cons s ss =>
        obtain ⟨hns, hrs, hrss⟩ := hr
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        obtain ⟨k1, Δ1', m1', l1', h1', hm1, hl1, heq⟩ := ih.one hrs h1
        obtain ⟨k2, Δ2', m2', l2', h2', hm2, hl2⟩ := ih.s hrss h2
        -- the rest after the statement: its environment changed only at the statement's binder,
        -- which nothing after it reads (R2), so the rest runs the same
        have h2'' : execS k2 (D.withComp c0 C') σ (Δ1' ++ ρ) ss = .ok (Δ2', m2', l2') := by
          by_cases ht : Taints S s
          · have hnr := hns ht
            have hnr' : NotRead s (fvB ss ++ (D.withComp c0 C').reads) :=
              fun y hy hm => hnr y hy (by
                simp only [List.mem_append] at hm ⊢
                rcases hm with h | h
                · exact Or.inl h
                · exact Or.inr (Or.inr h))
            have a1 := agree_drop h1 hnr' ρ
            have a2 := agree_drop h1' hnr' ρ
            exact (indep k2).s h2' (fun y hy => (a1 y hy).trans (a2 y hy).symm)
          · rw [heq ht]; exact h2'
        obtain ⟨N, hN⟩ := execS_cons_compose h1' h2''
        exact ⟨N, _, _, _, hN, by simpa using MR.app hm1 hm2, by simpa using LR.app hl1 hl2⟩
    · intro σ ρ i b ks m l hr h
      cases ks with
      | nil => exact ⟨n + 1, m, l, by simp [loopN] at h ⊢; exact h, .refl _, .refl _⟩
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        obtain ⟨K1, Δ1', m1', l1', h1', hm1, hl1⟩ := ih.s hr h1
        obtain ⟨K2, m2', l2', h2', hm2, hl2⟩ := ih.ln hr h2
        refine ⟨max K1 K2 + 1, pre [.ix k] m1' ++ m2', l1' ++ l2', ?_, .app (.pre _ hm1) hm2, .app hl1 hl2⟩
        simp [loopN, execS_mono h1' (Nat.le_max_left K1 K2), loopN_mono h2' (Nat.le_max_right K1 K2)]
    · intro σ ρ i b kvs m l hr h
      cases kvs with
      | nil => exact ⟨n + 1, m, l, by simp [loopK] at h ⊢; exact h, .refl _, .refl _⟩
      | cons kv kvs =>
        obtain ⟨key, e⟩ := kv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨te, le⟩, he, ⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        obtain ⟨K1, Δ1', m1', l1', h1', hm1, hl1⟩ := ih.s hr h1
        obtain ⟨K2, m2', l2', h2', hm2, hl2⟩ := ih.lk hr h2
        refine ⟨max n (max K1 K2) + 1, pre [.key key] m1' ++ m2', le ++ l1' ++ l2', ?_,
          .app (.pre _ hm1) hm2, .app (.app (.refl _) hl1) hl2⟩
        simp [loopK, (cong _).e, evalE_mono he (Nat.le_max_left _ _),
          execS_mono h1' (Nat.le_trans (Nat.le_max_left K1 K2) (Nat.le_max_right _ _)),
          loopK_mono h2' (Nat.le_trans (Nat.le_max_right K1 K2) (Nat.le_max_right _ _))]

end ForgeLang
