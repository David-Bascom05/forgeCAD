import ForgeLang.Subst
import ForgeLang.Fv

/-!
# Memoisation: build a component call once, instance it everywhere

A component body runs in its *input environment* `I`: the call's resolved arguments, the defaults
it left alone, and the visible shared params (`Δg ++ Δd ++ only C.vis ρ` in `execOne`). The cache
does not store that run. It stores the *parametric* run: the same body in an environment where every
input name `y` is a fresh slot site `S.slot y`, under the table `σP` that gives each slot the
input's number. A call is then that entry with each slot replaced by the call's own input trace
(`S.theta I`).

* `memo_inst`: a call's run *is* its parametric run instantiated (both ways: one evaluates exactly
  when the other does, to the same traces, ids and log).
* `memo_key`: the parametric run depends only on the key: the body's text, the definitions, the
  numbers of the literals in them, and the *numbers* of the inputs. Not on the inputs' provenance.
* `memo_sound`: so a cache entry computed for one call, in this document or another, gives every
  other call with an equal key exactly its own run. This is what makes one build per distinct call,
  instancing, and the content-addressed build cache correct.

What the key must hold (each one shown necessary in `Counter/Memo.lean`): the body skeleton, the
literal numbers in the body and in what it calls, the definitions it calls, the input numbers *including
the visible shared params*. What it need not hold: the inputs' traces, provided the entry is
stored parametric. A cache that stores a concrete result keyed by values is wrong
(`Counter/Memo.concrete_cache_wrong`).
-/

namespace ForgeLang

/-- Slot sites for input names: no code mentions one (`NoSlot`). -/
structure Slots where
  slot : Name → Site
  unslot : Site → Option Name
  inv : ∀ y, unslot (slot y) = some y

def NoSlot (S : Slots) (N : List Site) : Prop := ∀ s ∈ N, S.unslot s = none

/-- The parametric input environment over names `K`. -/
def Slots.env (S : Slots) (K : List Name) : Env := K.map fun y => (y, .num (.site (S.slot y)))

/-- Instantiation: each slot becomes the call's input trace. -/
def Slots.theta (S : Slots) (I : Env) : Site → Tr := fun s =>
  match S.unslot s with
  | some y =>
    match lookupNum I y with
    | .ok t => if s = S.slot y then t else .site s
    | .error _ => .site s
  | none => .site s

/-- The table of the parametric run: slots read as the inputs' numbers. -/
def σP (S : Slots) (σ : Site → Int) (I : Env) : Site → Int := fun s => (S.theta I s).den σ

/-- `I` binds exactly the names `K`, each to a number. -/
def NumOn (K : List Name) (I : Env) : Prop :=
  ∀ y, (y ∈ K → ∃ t, I.lookup y = some (.num t)) ∧ (y ∉ K → I.lookup y = none)

theorem env_subst_lookup (S : Slots) (θ : Site → Tr) (y : Name) :
    ∀ K : List Name, ((S.env K).subst θ).lookup y = if y ∈ K then some (.num (θ (S.slot y))) else none
  | [] => by simp [Slots.env, Env.subst, List.lookup]
  | k :: K => by
    have ih := env_subst_lookup S θ y K
    simp only [Slots.env, Env.subst, List.map_cons, List.lookup] at ih ⊢
    by_cases h : y = k
    · subst h; simp [Bind.subst, Tr.subst]
    · have e : (y == k) = false := by simp [h]
      simp only [e, ih, List.mem_cons, h, false_or]

theorem theta_slot {S : Slots} {I : Env} {y : Name} {t : Tr} (h : I.lookup y = some (.num t)) :
    S.theta I (S.slot y) = t := by
  have : lookupNum I y = .ok t := by simp [lookupNum, h]
  simp [Slots.theta, S.inv, this]

theorem theta_noslot {S : Slots} {I : Env} {s : Site} (h : S.unslot s = none) : S.theta I s = .site s := by
  simp [Slots.theta, h]

/-- The instantiated parametric environment is the input environment, name for name. -/
theorem inst_agree {S : Slots} {K : List Name} {I : Env} (hI : NumOn K I) (N : List Name) :
    AgreeOn N ((S.env K).subst (S.theta I)) I := by
  intro y _
  rw [env_subst_lookup]
  by_cases hy : y ∈ K
  · obtain ⟨t, ht⟩ := (hI y).1 hy
    rw [if_pos hy, theta_slot ht, ht]
  · rw [if_neg hy, (hI y).2 hy]

/-- A call's run is its parametric run, instantiated with the call's inputs. -/
theorem memo_inst {S : Slots} {n : Nat} {D : Defs} {σ : Site → Int} {I : Env} {K : List Name}
    {body : List Stmt} {r : Env × Model × Log} (hI : NumOn K I) (hc : NoSlot S (codeB body ++ D.code)) :
    execS n D σ I body = .ok r ↔ (execS n D (σP S σ I) (S.env K) body).map (fS (S.theta I)) = .ok r := by
  have eq := (substEq (θ := S.theta I) (σA := σP S σ I) (σB := σ) (fun _ => rfl) n).s
    (D := D) (ρ := S.env K) (x := body) (fun s hs => theta_noslot (hc s hs))
  rw [← eq]
  exact ⟨fun h => (indep n).s h (inst_agree hI _).symm, fun h => (indep n).s h (inst_agree hI _)⟩

/-- The parametric run depends only on the key: same definitions and body, same literal numbers in
their code, same input names and numbers. -/
theorem memo_key {S : Slots} {n : Nat} {D : Defs} {σ1 σ2 : Site → Int} {I1 I2 : Env} {K : List Name}
    {body : List Stmt} {R : Env × Model × Log} (htr : D.track = true)
    (hI1 : NumOn K I1) (hI2 : NumOn K I2) (hc : NoSlot S (codeB body ++ D.code))
    (hlit : ∀ s ∈ codeB body ++ D.code, σ1 s = σ2 s)
    (hval : ∀ y ∈ K, (lookupNum I1 y).map (Tr.den σ1) = (lookupNum I2 y).map (Tr.den σ2))
    (h : execS n D (σP S σ1 I1) (S.env K) body = .ok R) :
    execS n D (σP S σ2 I2) (S.env K) body = .ok R := by
  obtain ⟨Δ, m, l⟩ := R
  have si := (sitesIn n).s h
  refine (sim n).s (.inl htr) h ?_ (Nat.le_refl n)
  intro s hs
  rcases si s (List.mem_append_right _ hs) with hs | hs | hs
  · have := hc s (List.mem_append_left _ hs)
    simp only [σP, theta_noslot this, Tr.den]
    exact hlit s (List.mem_append_left _ hs)
  · have := hc s (List.mem_append_right _ hs)
    simp only [σP, theta_noslot this, Tr.den]
    exact hlit s (List.mem_append_right _ hs)
  · simp only [envSites, Slots.env, List.flatMap_map, List.mem_flatMap, Bind.sites, Tr.sites,
      List.mem_singleton] at hs
    obtain ⟨y, hy, rfl⟩ := hs
    obtain ⟨t1, h1⟩ := (hI1 y).1 hy
    obtain ⟨t2, h2⟩ := (hI2 y).1 hy
    have hv := hval y hy
    simp only [lookupNum, h1, h2, map_ok, Except.ok.injEq] at hv
    simp only [σP, theta_slot h1, theta_slot h2]
    exact hv

/-- Memoisation is sound. A cache entry `R` built from one call gives any other call with an equal
key (same body and definitions, same literal numbers in their code, same input numbers) exactly the
result evaluating that call would give, with the other call's own provenance. The two calls may be
in one document (`σ1 = σ2`: instancing) or in two (the build cache). -/
theorem memo_sound {S : Slots} {n : Nat} {D : Defs} {σ1 σ2 : Site → Int} {I1 I2 : Env} {K : List Name}
    {body : List Stmt} {r1 : Env × Model × Log} (htr : D.track = true)
    (hI1 : NumOn K I1) (hI2 : NumOn K I2) (hc : NoSlot S (codeB body ++ D.code))
    (hlit : ∀ s ∈ codeB body ++ D.code, σ1 s = σ2 s)
    (hval : ∀ y ∈ K, (lookupNum I1 y).map (Tr.den σ1) = (lookupNum I2 y).map (Tr.den σ2))
    (h1 : execS n D σ1 I1 body = .ok r1) :
    ∃ R, execS n D (σP S σ1 I1) (S.env K) body = .ok R ∧ fS (S.theta I1) R = r1 ∧
      execS n D σ2 I2 body = .ok (fS (S.theta I2) R) := by
  have e1 := (memo_inst (S := S) hI1 hc).1 h1
  cases hR : execS n D (σP S σ1 I1) (S.env K) body with
  | error e => rw [hR] at e1; cases e1
  | ok R =>
    rw [hR] at e1
    simp only [map_ok, Except.ok.injEq] at e1
    refine ⟨R, rfl, e1, ?_⟩
    have h2 := memo_key htr hI1 hI2 hc hlit hval hR
    exact (memo_inst (S := S) hI2 hc).2 (by rw [h2]; rfl)

/-- The numbers agree too: an instantiated trace reads, under the call's table, as the cached trace
reads under the cached table. So the geometry built once is the geometry of every instance. -/
theorem memo_nums (S : Slots) (σ : Site → Int) (I : Env) (t : Tr) :
    (t.subst (S.theta I)).den σ = t.den (σP S σ I) :=
  Tr.den_subst (fun _ => rfl) t

end ForgeLang
