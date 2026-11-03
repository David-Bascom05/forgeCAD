import ForgeLang.Split

/-!
# Where a trace's sites can come from, and per-instance independence after a split

`sitesIn`: every site in a run's traces and log is a site of the code, of a function or procedure
body, or of a trace already in the environment. Nothing else can appear.

`split_independent`: after SplitInstance, instance `k`'s features carry no other instance's new
literal. With `sole_driver`, editing `q j` changes instance `j` and leaves every other instance's
numbers alone (when `q j` is not observed; if the body branches on `w`, `q j` is structural and its
edit is a re-run, as for any observed literal).
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

def Bind.sites : Bind → List Site
  | .num t => t.sites
  | .frag m => m.flatMap fun e => e.2.sites

def envSites (ρ : Env) : List Site := ρ.flatMap fun e => e.2.sites

def modelSites (m : Model) : List Site := m.flatMap fun e => e.2.sites

theorem envSites_append (A B : Env) : envSites (A ++ B) = envSites A ++ envSites B := by
  simp [envSites]

theorem modelSites_pre (p : FId) (m : Model) : modelSites (pre p m) = modelSites m := by
  induction m with
  | nil => rfl
  | cons e m ih => simp_all [modelSites, pre]

/-- A site is allowed: from the code `C`, the definitions, or the environment. -/
def Allowed (C : List Site) (D : Defs) (ρ : Env) (s : Site) : Prop := s ∈ C ∨ s ∈ D.code ∨ s ∈ envSites ρ

def AllIn (L C : List Site) (D : Defs) (ρ : Env) : Prop := ∀ s ∈ L, Allowed C D ρ s

theorem allIn_append {A B C : List Site} {D : Defs} {ρ : Env} :
    AllIn (A ++ B) C D ρ ↔ AllIn A C D ρ ∧ AllIn B C D ρ := by
  simp only [AllIn, List.mem_append]
  exact ⟨fun h => ⟨fun s hs => h s (.inl hs), fun s hs => h s (.inr hs)⟩,
    fun h s hs => hs.elim (h.1 s) (h.2 s)⟩

theorem allIn_nil {C : List Site} {D : Defs} {ρ : Env} : AllIn [] C D ρ := fun _ h => by cases h

theorem AllIn.mono {L C C' : List Site} {D : Defs} {ρ : Env} (h : AllIn L C D ρ)
    (hc : ∀ s ∈ C, s ∈ C') : AllIn L C' D ρ := by
  intro s hs
  rcases h s hs with h | h | h
  · exact .inl (hc s h)
  · exact .inr (.inl h)
  · exact .inr (.inr h)

theorem AllIn.toDefs {L C C' : List Site} {D : Defs} {ρ : Env} (h : AllIn L C D ρ)
    (hc : ∀ s ∈ C, s ∈ D.code) : AllIn L C' D ρ := by
  intro s hs
  rcases h s hs with h | h | h
  · exact .inr (.inl (hc s h))
  · exact .inr (.inl h)
  · exact .inr (.inr h)

theorem AllIn.sub {L L' C : List Site} {D : Defs} {ρ : Env} (h : AllIn L C D ρ) (hs : ∀ s ∈ L', s ∈ L) :
    AllIn L' C D ρ := fun s h' => h s (hs s h')

theorem AllIn.obs {t : Tr} {C : List Site} {D : Defs} {ρ : Env} (h : AllIn t.sites C D ρ) :
    AllIn (ForgeLang.obs D t) C D ρ :=
  h.sub fun s hs => by unfold ForgeLang.obs at hs; split at hs <;> simp_all

/-- Moving from an extended environment back to the outer one, when the extension is allowed. -/
theorem AllIn.unenv {L C : List Site} {D : Defs} {ρ Δ : Env} (h : AllIn L C D (Δ ++ ρ))
    (hΔ : AllIn (envSites Δ) C D ρ) : AllIn L C D ρ := by
  intro s hs
  rcases h s hs with h | h | h
  · exact .inl h
  · exact .inr (.inl h)
  · rw [envSites_append, List.mem_append] at h
    rcases h with h | h
    · exact hΔ s h
    · exact .inr (.inr h)

theorem lookupNum_sites {ρ : Env} {x : Name} {t : Tr} {C : List Site} {D : Defs}
    (h : lookupNum ρ x = .ok t) : AllIn t.sites C D ρ := by
  unfold lookupNum at h
  split at h
  · rename_i t' ht
    cases h
    intro s hs
    refine .inr (.inr ?_)
    simp only [envSites, List.mem_flatMap]
    exact ⟨(x, .num _), lookup_mem ht, hs⟩
  · cases h
  · cases h

theorem lookupRef_sites {ρ : Env} {x : Name} {rel : FId} {t : Tr} {C : List Site} {D : Defs}
    (h : lookupRef ρ x rel = .ok t) : AllIn t.sites C D ρ := by
  unfold lookupRef at h
  split at h
  · rename_i m hm
    split at h
    · rename_i t' ht
      cases h
      intro s hs
      refine .inr (.inr ?_)
      simp only [envSites, List.mem_flatMap]
      exact ⟨(x, .frag m), lookup_mem hm, by
        simp only [Bind.sites, List.mem_flatMap]
        exact ⟨(rel, _), lookup_mem ht, hs⟩⟩
    · cases h
  · cases h
  · cases h

theorem envSites_cons (x : Name) (b : Bind) (ρ : Env) : envSites ((x, b) :: ρ) = b.sites ++ envSites ρ := by
  simp [envSites]

@[simp] theorem bind_sites_num (t : Tr) : Bind.sites (.num t) = t.sites := rfl
@[simp] theorem bind_sites_frag (m : Model) : Bind.sites (.frag m) = modelSites m := rfl
@[simp] theorem envSites_nil : envSites [] = [] := rfl
theorem modelSites_append (A B : Model) : modelSites (A ++ B) = modelSites A ++ modelSites B := by
  simp [modelSites]
@[simp] theorem modelSites_single (i : FId) (t : Tr) : modelSites [(i, t)] = t.sites := by
  simp [modelSites]
@[simp] theorem modelSites_nil : modelSites [] = [] := rfl

structure SitesIn (n : Nat) : Prop where
  e : ∀ {D σ ρ x t l}, evalE n D σ ρ x = .ok (t, l) → AllIn (t.sites ++ l) (codeE x) D ρ
  one : ∀ {D σ ρ x Δ m l}, execOne n D σ ρ x = .ok (Δ, m, l) →
    AllIn (envSites Δ ++ modelSites m ++ l) (codeS x) D ρ
  s : ∀ {D σ ρ x Δ m l}, execS n D σ ρ x = .ok (Δ, m, l) →
    AllIn (envSites Δ ++ modelSites m ++ l) (codeB x) D ρ
  ln : ∀ {D σ ρ i b ks m l}, loopN n D σ ρ i b ks = .ok (m, l) → AllIn (modelSites m ++ l) (codeB b) D ρ
  lk : ∀ {D σ ρ i b kvs m l}, loopK n D σ ρ i b kvs = .ok (m, l) →
    AllIn (modelSites m ++ l) (codeKV kvs ++ codeB b) D ρ

macro "fin" : tactic => `(tactic| ((repeat' apply And.intro) <;> first | assumption | exact allIn_nil))

theorem sitesIn : ∀ n, SitesIn n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK]
  | succ n ih =>
    constructor
    · intro D σ ρ x t l h
      cases x with
      | lit s =>
        simp [evalE] at h; obtain ⟨rfl, rfl⟩ := h
        intro s' hs; simp [Tr.sites] at hs; subst hs; exact .inl (by simp [codeE])
      | cst c => simp [evalE] at h; obtain ⟨rfl, rfl⟩ := h; simpa [Tr.sites] using allIn_nil
      | var x =>
        simp only [evalE, bind_ok] at h
        obtain ⟨t', ht, h⟩ := h; cases h
        simpa using lookupNum_sites ht
      | ref x rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨t', ht, h⟩ := h; cases h
        simpa using lookupRef_sites ht
      | add a b | mul a b | eq a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        have A := ((ih.e ha).mono (C' := codeE a ++ codeE b) (by simp_all))
        have B := ((ih.e hb).mono (C' := codeE a ++ codeE b) (by simp_all))
        rw [allIn_append] at A B
        obtain ⟨A1, A2⟩ := A; obtain ⟨B1, B2⟩ := B
        have A' := A1.obs (D := D); have B' := B1.obs (D := D)
        simp only [codeE, Tr.sites, allIn_append, List.append_assoc]
        fin
      | ite c a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t', l'⟩, hb, h⟩ := h
        cases h
        have C := (ih.e hc).mono (C' := codeE (.ite c a b)) (by simp [codeE]; grind)
        have Bb := (ih.e hb).mono (C' := codeE (.ite c a b)) (by split <;> (simp [codeE]; grind))
        rw [allIn_append] at C Bb
        obtain ⟨C1, C2⟩ := C; obtain ⟨B1, B2⟩ := Bb
        have C' := C1.obs (D := D)
        simp only [allIn_append, List.append_assoc]
        fin
      | sel es k =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t', l'⟩, he', h⟩ := h
          cases h
          have hsub := codeE_sub_codeEs (List.mem_of_getElem? he)
          have K := (ih.e hk).mono (C' := codeE (.sel es k)) (by intro s hs; simp [codeE, hs])
          have E := (ih.e he').mono (C' := codeE (.sel es k)) (by intro s hs; simp [codeE, hsub s hs])
          rw [allIn_append] at K E
          obtain ⟨K1, K2⟩ := K; obtain ⟨E1, E2⟩ := E
          have K' := K1.obs (D := D)
          simp only [allIn_append, List.append_assoc]
          fin
        · cases h
      | call f a =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i x body hf
          rw [bind_ok] at h
          obtain ⟨⟨t', l'⟩, hb, h⟩ := h
          cases h
          have A := (ih.e ha).mono (C' := codeE (.call f a)) (by simp [codeE])
          rw [allIn_append] at A
          obtain ⟨A1, A2⟩ := A
          have B := ((ih.e hb).toDefs (C' := codeE (.call f a)) (fn_code hf)).unenv (Δ := [(x, .num ta)])
            (by simpa [envSites, Bind.sites] using A1)
          rw [allIn_append] at B
          obtain ⟨B1, B2⟩ := B
          simp only [allIn_append]
          fin
        · cases h
      | refIx x k rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t', ht, h⟩ := h
        cases h
        have K := (ih.e hk).mono (C' := codeE (.refIx x k rel)) (by simp [codeE])
        rw [allIn_append] at K
        obtain ⟨K1, K2⟩ := K
        have K' := K1.obs (D := D)
        have T : AllIn _ (codeE (.refIx x k rel)) D ρ := lookupRef_sites ht
        simp only [allIn_append]
        fin
    · intro D σ ρ x Δ m l h
      cases x with
      | let_ x e | feat x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l'⟩, he, h⟩ := h
        cases h
        have E := ih.e he
        rw [allIn_append] at E
        obtain ⟨E1, E2⟩ := E
        simp only [codeS, envSites_cons, bind_sites_num, bind_sites_frag, modelSites_single, modelSites_nil,
          envSites_nil, allIn_append, List.append_nil]
        fin
      | set x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l'⟩, he, h⟩ := h
        cases h
        have E := (ih.e he).mono (C' := codeS (.set x e)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        rw [allIn_append] at E
        obtain ⟨E1, E2⟩ := E
        simp only [envSites_cons, bind_sites_num, modelSites_nil, envSites_nil, allIn_append,
          List.append_nil]
        fin
      | forN x i c body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l'⟩, hl, h⟩ := h
        cases h
        have C := (ih.e hc).mono (C' := codeS (.forN x i c body)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        have L := (ih.ln hl).mono (C' := codeS (.forN x i c body)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        rw [allIn_append] at C L
        obtain ⟨C1, C2⟩ := C; obtain ⟨L1, L2⟩ := L
        have C' := C1.obs (D := D)
        simp only [envSites_cons, bind_sites_frag, modelSites_pre, envSites_nil, allIn_append,
          List.append_nil]
        fin
      | forK x i kvs body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l'⟩, hl, h⟩ := h
        cases h
        have L := (ih.lk hl).mono (C' := codeS (.forK x i kvs body)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        rw [allIn_append] at L
        obtain ⟨L1, L2⟩ := L
        simp only [envSites_cons, bind_sites_frag, modelSites_pre, envSites_nil, allIn_append,
          List.append_nil]
        fin
      | ifs c a b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l'⟩, hb, h⟩ := h
        cases h
        have C := (ih.e hc).mono (C' := codeS (.ifs c a b)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        have B := (ih.s hb).mono (C' := codeS (.ifs c a b)) (by split <;> (intro s hs; simp [codeS, codeB, codeKV, hs]))
        rw [allIn_append, allIn_append] at B
        rw [allIn_append] at C
        obtain ⟨C1, C2⟩ := C; obtain ⟨⟨-, B1⟩, B2⟩ := B
        have C' := C1.obs (D := D)
        simp only [envSites_nil, allIn_append, List.nil_append]
        fin
      | use x p a =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l'⟩, hb, h⟩ := h
          cases h
          have A := (ih.e ha).mono (C' := codeS (.use x p a)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
          rw [allIn_append] at A
          obtain ⟨A1, A2⟩ := A
          have B := ((ih.s hb).toDefs (C' := codeS (.use x p a)) (proc_code hp)).unenv
            (Δ := [(y, .num ta)]) (by simpa [envSites] using A1)
          rw [allIn_append, allIn_append] at B
          obtain ⟨⟨-, B1⟩, B2⟩ := B
          simp only [envSites_cons, bind_sites_frag, modelSites_pre, envSites_nil, allIn_append,
            List.append_nil]
          fin
        · cases h
    · intro D σ ρ x Δ m l h
      cases x with
      | nil => simp [execS] at h; obtain ⟨rfl, rfl, rfl⟩ := h; simpa using allIn_nil
      | cons s ss =>
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        have S1 := (ih.one h1).mono (C' := codeB (s :: ss)) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        rw [allIn_append, allIn_append] at S1
        obtain ⟨⟨S1a, S1b⟩, S1c⟩ := S1
        have S2 := ((ih.s h2).mono (C' := codeB (s :: ss)) (by intro s hs; simp [codeS, codeB, codeKV, hs])).unenv S1a
        rw [allIn_append, allIn_append] at S2
        obtain ⟨⟨S2a, S2b⟩, S2c⟩ := S2
        simp only [envSites_append, modelSites_append, allIn_append]
        fin
    · intro D σ ρ i b ks m l h
      cases ks with
      | nil => simp [loopN] at h; obtain ⟨rfl, rfl⟩ := h; simpa using allIn_nil
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        have S1 := (ih.s h1).unenv (Δ := [(i, .num (.const k))]) (by simpa [envSites, Tr.sites] using allIn_nil)
        rw [allIn_append, allIn_append] at S1
        obtain ⟨⟨-, S1b⟩, S1c⟩ := S1
        have S2 := ih.ln h2
        rw [allIn_append] at S2
        obtain ⟨S2a, S2b⟩ := S2
        simp only [modelSites_append, modelSites_pre, allIn_append]
        fin
    · intro D σ ρ i b kvs m l h
      cases kvs with
      | nil => simp [loopK] at h; obtain ⟨rfl, rfl⟩ := h; simpa using allIn_nil
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        have E := (ih.e h0).mono (C' := codeKV ((k, e) :: kvs) ++ codeB b) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        rw [allIn_append] at E
        obtain ⟨E1, E2⟩ := E
        have S1 := ((ih.s h1).mono (C' := codeKV ((k, e) :: kvs) ++ codeB b) (by intro s hs; simp [hs])).unenv
          (Δ := [(i, .num t)]) (by simpa [envSites] using E1)
        rw [allIn_append, allIn_append] at S1
        obtain ⟨⟨-, S1b⟩, S1c⟩ := S1
        have S2 := (ih.lk h2).mono (C' := codeKV ((k, e) :: kvs) ++ codeB b) (by intro s hs; simp [codeS, codeB, codeKV, hs])
        rw [allIn_append] at S2
        obtain ⟨S2a, S2b⟩ := S2
        simp only [modelSites_append, modelSites_pre, allIn_append]
        fin

/-- After SplitInstance, a feature of instance `k` carries no other instance's literal. -/
theorem split_independent {D : Defs} {σ : Site → Int} {ρ : Env} {i w : Name} {qs : List Site}
    {rest : List Stmt} (hnd : qs.Nodup)
    (hfresh : ∀ q ∈ qs, q ∉ codeB rest ∧ q ∉ D.code ∧ q ∉ envSites ρ) :
    ∀ (ks : List Nat) {n mm l}, (∀ k ∈ ks, k < qs.length) →
      loopN n D σ ρ i (splitBody w qs i rest) ks = .ok (mm, l) →
      ∀ e ∈ mm, ∀ (k j : Nat) (hj : j < qs.length), e.1.head? = some (.ix k) → j ≠ k → qs[j] ∉ e.2.sites
  | [], n, mm, l, _, h => by
    cases n <;> simp [loopN] at h
    obtain ⟨rfl, rfl⟩ := h
    simp
  | k0 :: ks, n, mm, l, hk, h => by
    cases n with
    | zero => simp [loopN] at h
    | succ n =>
      simp only [loopN, bind_ok] at h
      obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
      cases h
      intro e he k j hj hhead hjk
      rcases List.mem_append.1 he with he | he
      · simp only [pre, List.mem_map] at he
        obtain ⟨⟨id, t⟩, hm, rfl⟩ := he
        simp only [List.cons_append, List.nil_append, List.head?_cons, Option.some.injEq,
          Seg.ix.injEq] at hhead
        subst hhead
        have hk0 : k0 < qs.length := hk k0 (by simp)
        obtain ⟨Δl, ml, ll, Δr, mr, lr, hl, hr, e1⟩ := execS_cons h1
        cases execOne_unique hl (split_let (f := 0) (D := D) (σ := σ) (ρ := ρ) (w := w) hk0)
        simp only [Prod.mk.injEq] at e1
        obtain ⟨-, rfl, -⟩ := e1
        have S := (sitesIn n).s hr
        intro hq
        have hin : qs[j] ∈ modelSites m1 := by
          simp only [modelSites, List.mem_flatMap]
          exact ⟨(id, t), by simpa using hm, hq⟩
        rcases S qs[j] (by simp [hin]) with hc | hc | hc
        · exact (hfresh _ (List.getElem_mem hj)).1 hc
        · exact (hfresh _ (List.getElem_mem hj)).2.1 hc
        · simp only [envSites_cons, bind_sites_num, Tr.sites, List.cons_append, List.nil_append,
            List.mem_cons] at hc
          rcases hc with hc | hc
          · exact hjk ((hnd.getElem_inj).1 hc)
          · exact (hfresh _ (List.getElem_mem hj)).2.2 hc
      · exact split_independent hnd hfresh ks (fun k hk' => hk k (by simp [hk'])) h2 e he k j hj hhead hjk

/-- So editing instance `j`'s literal leaves every other instance's numbers alone. -/
theorem split_edit_one {D : Defs} {σ : Site → Int} {ρ : Env} {i w : Name} {qs : List Site}
    {rest : List Stmt} (hnd : qs.Nodup)
    (hfresh : ∀ q ∈ qs, q ∉ codeB rest ∧ q ∉ D.code ∧ q ∉ envSites ρ) {ks : List Nat} {n mm l}
    (hk : ∀ k ∈ ks, k < qs.length) (h : loopN n D σ ρ i (splitBody w qs i rest) ks = .ok (mm, l))
    (j : Nat) (hj : j < qs.length) (v : Int) :
    ∀ e ∈ mm, ∀ k, e.1.head? = some (.ix k) → j ≠ k → e.2.den (upd σ qs[j] v) = e.2.den σ := by
  intro e he k hhead hjk
  have := split_independent hnd hfresh ks hk h e he k j hj hhead hjk
  exact (Tr.den_congr e.2 (agree_upd this)).symm

end ForgeLang
