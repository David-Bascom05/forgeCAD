import ForgeLang.Scoped

/-!
# Round 2: nesting to any depth (NestedComponent retired)

`Scoped.defs_congr` needed `NoPlace c0 D`: no block places the edited component. With nesting a
block may place components that place others. `defs_congr_closed` replaces the hypothesis by a
set `S` of components that cannot reach `c0`: closed under placement (what an `S` block places
is in `S`), procedures place only `S`, and `c0 ∉ S`. Changing `c0`'s definition then changes no
statement that places only `S`, at any depth. `NoPlace` is the case `S = (· ≠ c0)`.
-/

namespace ForgeLang

/-- `S`: components that, through any chain of placements, never reach `c0`. -/
structure Avoids (D : Defs) (c0 : Name) (S : Name → Prop) : Prop where
  ne : ∀ c, S c → c ≠ c0
  comp : ∀ c C, S c → D.comps.lookup c = some C → ∀ d ∈ placesB C.body, S d
  proc : ∀ p ∈ D.procs, ∀ d ∈ placesB p.2.2, S d

structure CongrS (D : Defs) (c0 : Name) (C' : Comp) (S : Name → Prop) (n : Nat) : Prop where
  e : ∀ {σ ρ x}, evalE n (D.withComp c0 C') σ ρ x = evalE n D σ ρ x
  one : ∀ {σ ρ x}, (∀ d ∈ placesS x, S d) → execOne n (D.withComp c0 C') σ ρ x = execOne n D σ ρ x
  s : ∀ {σ ρ x}, (∀ d ∈ placesB x, S d) → execS n (D.withComp c0 C') σ ρ x = execS n D σ ρ x
  ln : ∀ {σ ρ i b ks}, (∀ d ∈ placesB b, S d) → loopN n (D.withComp c0 C') σ ρ i b ks = loopN n D σ ρ i b ks
  lk : ∀ {σ ρ i b kvs}, (∀ d ∈ placesB b, S d) → loopK n (D.withComp c0 C') σ ρ i b kvs = loopK n D σ ρ i b kvs
  a : ∀ {σ ρ as}, evalA n (D.withComp c0 C') σ ρ as = evalA n D σ ρ as

/-- Editing `c0` changes nothing that cannot reach it, however deeply it nests. -/
theorem defs_congr_closed {D : Defs} {c0 : Name} {C' : Comp} {S : Name → Prop} (hA : Avoids D c0 S) :
    ∀ n, CongrS D c0 C' S n := by
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
        simp only [placesS, List.mem_append] at hp
        simp only [execOne, ih.e, obs_withComp]
        cases evalE n D σ ρ c with
        | error _ => rfl
        | ok pc =>
          simp only [ok_bind]
          split
          · rw [ih.s (fun d hd => hp d (Or.inl hd))]
          · rw [ih.s (fun d hd => hp d (Or.inr hd))]
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
            rw [ih.s (hA.proc (p, y, body) (lookup_mem hq))]
      | occ x c args =>
        simp only [placesS, List.mem_singleton, forall_eq] at hp
        simp only [execOne, withComp_lookup_other D C' (hA.ne c hp)]
        cases hc : D.comps.lookup c with
        | none => rfl
        | some C =>
          simp only
          split
          · simp only [ih.a, ih.s (hA.comp c C hp hc)]
          · rfl
    · intro σ ρ x hp
      cases x with
      | nil => rfl
      | cons s ss =>
        simp only [placesB, List.mem_append] at hp
        simp only [execS, ih.one (fun d hd => hp d (Or.inl hd)), ih.s (fun d hd => hp d (Or.inr hd))]
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

/-- `NoPlace` (the retired NestedComponent rule) is the special case `S = (· ≠ c0)`. -/
theorem noPlace_avoids {D : Defs} {c0 : Name} (h : NoPlace c0 D) : Avoids D c0 (· ≠ c0) where
  ne := fun _ h => h
  comp := fun c C _ hc _d hd e => h.2 (c, C) (lookup_mem hc) (e ▸ hd)
  proc := fun p hp _d hd e => h.1 p hp (e ▸ hd)

/-- An occurrence path, at any depth, is a prefix of every id it makes: a nested occurrence's
features are `pre [x]` of its block's, which are `pre [y]` of the inner block's, and so on. -/
theorem occ_pre {n D σ ρ x c args Δ m l} (h : execOne n D σ ρ (.occ x c args) = .ok (Δ, m, l)) :
    ∃ m', m = pre [.nm x] m' ∧ Δ = [(x, .frag m')] := by
  match n with
  | 0 => simp [execOne] at h
  | k + 1 =>
    simp only [execOne] at h
    split at h
    · split at h
      · simp only [bind, Except.bind] at h
        split at h
        · cases h
        · split at h
          · cases h
          · split at h
            · cases h
            · simp only [Except.ok.injEq, Prod.mk.injEq] at h
              exact ⟨_, h.2.1.symm, h.1.symm⟩
      · cases h
    · cases h

/-- A component reaches another through its block's placements, directly or not. -/
inductive Reach (D : Defs) : Name → Name → Prop
  | step {c d C} : D.comps.lookup c = some C → d ∈ placesB C.body → Reach D c d
  | trans {c d e} : Reach D c d → Reach D d e → Reach D c e

/-- A rank certificate: every component a block places ranks below the block's component. -/
def RankOK (D : Defs) (r : Name → Nat) : Prop :=
  ∀ c C, D.comps.lookup c = some C → ∀ d ∈ placesB C.body, r d < r c

theorem reach_rank {D r} (h : RankOK D r) {c d} (hr : Reach D c d) : r d < r c := by
  induction hr with
  | step hc hd => exact h _ _ hc _ hd
  | trans _ _ ih1 ih2 => omega

/-- With a rank certificate no component reaches itself: the static `ComponentCycle` check. -/
theorem rank_acyclic {D r} (h : RankOK D r) (c : Name) : ¬ Reach D c c := fun hr => by
  have := reach_rank h hr; omega

/-- The components that cannot reach `c0` avoid it, when no procedure places one that can. -/
theorem avoids_reach {D : Defs} {c0 : Name}
    (hp : ∀ p ∈ D.procs, ∀ d ∈ placesB p.2.2, d ≠ c0 ∧ ¬ Reach D d c0) :
    Avoids D c0 (fun c => c ≠ c0 ∧ ¬ Reach D c c0) where
  ne := fun _ h => h.1
  comp := fun c C hc hl d hd => by
    refine ⟨fun e => hc.2 (e ▸ .step hl hd), fun r => hc.2 (.trans (.step hl hd) r)⟩
  proc := hp

/-- `scoped_occ` without NestedComponent: an occurrence of the edited component, at any depth,
evaluates as the occurrence of the edited block under the old definitions, as long as the new
block does not reach `c0` (no cycle). -/
theorem scoped_occ_closed {n D σ ρ x c0 args} {C' : Comp} {S : Name → Prop} (hA : Avoids D c0 S)
    (hb : ∀ d ∈ placesB C'.body, S d) :
    execOne (n + 1) (D.withComp c0 C') σ ρ (.occ x c0 args) = occRun n D σ ρ x c0 C' args := by
  rw [execOne_occ (withComp_lookup_self D c0 C')]
  simp only [occRun, (defs_congr_closed hA n).a, (defs_congr_closed hA n).s hb]

theorem pre_pre (p q : FId) (m : Model) : pre p (pre q m) = pre (p ++ q) m := by
  simp [pre, List.map_map, Function.comp_def, List.append_assoc]

end ForgeLang
