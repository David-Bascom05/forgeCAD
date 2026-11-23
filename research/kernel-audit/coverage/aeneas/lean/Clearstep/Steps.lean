import Clearstep.Proofs

/-!
# What `interior_point` decides: the first clear step, and the strip theorem

On the reference `ipRef` (equal to the extracted code by `interior_point_spec`):

* `first_clear_step`: a `Step (p, f, k)` result means step `k` off half-edge `(p, f)` is probed
  and clear of every other piece of the region, and every earlier step was probed and not clear.
* `chosen_gets_step`: when the chosen half-edge's probes are all defined and some step is clear,
  the result is a step off the chosen half-edge, at the first clear step.
* `strip_step_iff`: G3/G4's edge on the code. In a strip, every half-edge's step-`k` probe is
  clear unless the strip's opposite side is nearer than the step. With steps
  `step n k = 2^(n-1-k)` units, that means unless `w < 2 * step n k`. Then the chosen half-edge
  gets a step iff `2 ≤ w`, at the least `k` with `2 * step n k ≤ w`; otherwise no half-edge gets
  one.
-/

open Aeneas Aeneas.Std

namespace clearstep

/-! ## `clear_step` is the first step that is not refused -/

def firstStep (P C : Nat → Bool) (n k : Nat) : StepR :=
  if k < n then (if P k then (if C k then .found k else firstStep P C n (k + 1)) else .fail) else .exh
termination_by n - k

theorem csIn_stop (L : List (List (Nat × Bool))) (P : Nat → Bool) (row : Nat → Nat → Bool) (p n : Nat)
    (res : StepR) (k : Nat) : csIn L P row p n res true k = res := by
  unfold csIn; split <;> simp

theorem csIn_first (L : List (List (Nat × Bool))) (P : Nat → Bool) (row : Nat → Nat → Bool) (p n k : Nat) :
    csIn L P row p n .exh false k = firstStep P (fun j => clearAllRef (row j) p L) n k := by
  fun_induction firstStep P (fun j => clearAllRef (row j) p L) n k with
  | case1 k hk hp hc => unfold csIn; simp [hk, hp, hc, csIn_stop]
  | case2 k hk hp hc ih => unfold csIn; simp only [hk, hp, ite_true, Bool.false_eq_true, ite_false] at hc ⊢; simp [hc, ih]
  | case3 k hk hp => unfold csIn; simp only [hk, ite_true, Bool.false_eq_true, ite_false] at hp ⊢; simp [hp, csIn_stop]
  | case4 k hk => unfold csIn; simp [hk]

theorem firstStep_found (P C : Nat → Bool) (n k m : Nat) (h : firstStep P C n k = .found m) :
    k ≤ m ∧ m < n ∧ P m = true ∧ C m = true ∧ ∀ j, k ≤ j → j < m → P j = true ∧ C j = false := by
  revert h
  fun_induction firstStep P C n k with
  | case1 k hk hp hc => intro h; cases h; exact ⟨le_refl _, hk, hp, hc, fun j h1 h2 => by omega⟩
  | case2 k hk hp hc ih =>
    intro h
    obtain ⟨a, b, c, d, e⟩ := ih h
    refine ⟨by omega, b, c, d, fun j hj hjm => ?_⟩
    by_cases hjk : j = k
    · subst hjk; exact ⟨hp, by simpa using hc⟩
    · exact e j (by omega) hjm
  | case3 k hk hp => intro h; cases h
  | case4 k hk => intro h; cases h

theorem firstStep_of (P C : Nat → Bool) (n k : Nat) (hP : ∀ j, k ≤ j → j < n → P j = true)
    (m : Nat) (hkm : k ≤ m) (hm : m < n) (hC : C m = true) :
    ∃ k0, firstStep P C n k = .found k0 ∧ k ≤ k0 ∧ k0 ≤ m ∧ C k0 = true ∧ ∀ j, k ≤ j → j < k0 → C j = false := by
  revert hP hkm
  fun_induction firstStep P C n k with
  | case1 k hk hp hc => intro _ hkm; exact ⟨k, rfl, le_refl _, hkm, hc, fun j h1 h2 => by omega⟩
  | case2 k hk hp hc ih =>
    intro hP hkm
    have hck : C k = false := by simpa using hc
    have hkm' : k + 1 ≤ m := by
      by_cases e : k = m
      · subst e; rw [hC] at hck; cases hck
      · omega
    obtain ⟨k0, h1, h2, h3, h4, h5⟩ := ih (fun j a b => hP j (by omega) b) hkm'
    refine ⟨k0, h1, by omega, h3, h4, fun j hj hjk => ?_⟩
    by_cases hjk' : j = k
    · subst hjk'; exact hck
    · exact h5 j (by omega) hjk
  | case3 k hk hp => intro hP _; have := hP k (le_refl _) hk; simp_all
  | case4 k hk => intro _ hkm; omega

theorem firstStep_exh (P C : Nat → Bool) (n k : Nat) (h : ∀ j, k ≤ j → j < n → P j = true ∧ C j = false) :
    firstStep P C n k = .exh := by
  revert h
  fun_induction firstStep P C n k with
  | case1 k hk hp hc => intro h; have := (h k (le_refl _) hk).2; simp_all
  | case2 k hk hp hc ih => intro h; exact ih (fun j a b => h j (by omega) b)
  | case3 k hk hp => intro h; have := (h k (le_refl _) hk).1; simp_all
  | case4 k hk => intro _; rfl

/-! ## `clear_all` is "every other piece of the region is clear" -/

theorem clrIn_all (row : Nat → Bool) (h0 : Nat) (l : List (Nat × Bool)) (ok : Bool) (j : Nat) :
    clrIn row h0 l ok j = (ok && (l.drop j).all fun o => o.1 == h0 || row o.1) := by
  induction hd : l.length - j generalizing ok j with
  | zero =>
    unfold clrIn; rw [if_neg (by omega)]
    simp [List.drop_eq_nil_of_le (show l.length ≤ j by omega)]
  | succ d ih =>
    unfold clrIn
    rw [if_pos (by omega), ih _ (j + 1) (by omega)]
    rw [List.drop_eq_getElem_cons (show j < l.length by omega), getElem!_pos l j (by omega)]
    simp only [List.all_cons]
    by_cases he : l[j].1 = h0
    · simp [he]
    · cases hr : row l[j].1 <;> simp [he, hr]

theorem clrOut_all (row : Nat → Bool) (h0 : Nat) (L : List (List (Nat × Bool))) (ok : Bool) (i : Nat) :
    clrOut row h0 L ok i = (ok && (L.drop i).all fun l => l.all fun o => o.1 == h0 || row o.1) := by
  induction hd : L.length - i generalizing ok i with
  | zero =>
    unfold clrOut; rw [if_neg (by omega)]
    simp [List.drop_eq_nil_of_le (show L.length ≤ i by omega)]
  | succ d ih =>
    unfold clrOut
    rw [if_pos (by omega), ih _ (i + 1) (by omega), clrIn_all]
    rw [List.drop_eq_getElem_cons (show i < L.length by omega), getElem!_pos L i (by omega)]
    simp only [List.drop_zero, List.all_cons, Bool.and_assoc]

/-- `clear_all`: every piece of the region other than the half-edge's own is clear. -/
theorem clearAllRef_iff (row : Nat → Bool) (h0 : Nat) (L : List (List (Nat × Bool))) :
    clearAllRef row h0 L = true ↔ ∀ l ∈ L, ∀ o ∈ l, o.1 ≠ h0 → row o.1 = true := by
  unfold clearAllRef
  rw [clrOut_all]
  simp only [Bool.true_and, List.drop_zero, List.all_eq_true, Bool.or_eq_true, beq_iff_eq]
  constructor
  · intro h l hl o ho hne; exact (h l hl o ho).resolve_left hne
  · intro h l hl o ho
    by_cases hne : o.1 = h0
    · exact Or.inl hne
    · exact Or.inr (h l hl o ho hne)

/-! ## The second pass -/

theorem soIn_found (cs : Nat × Bool → StepR) (l : List (Nat × Bool)) (h : Nat × Bool) (out : R2) (j : Nat)
    (p : Nat) (f : Bool) (k : Nat) (hr : soIn cs l h out j = .found p f k) :
    out = .found p f k ∨ cs (p, f) = .found k := by
  revert hr
  fun_induction soIn cs l h out j with
  | case1 j hj ih =>
    intro hr
    rcases ih hr with h1 | h1
    · split at h1
      · right
        generalize hc : cs l[j]! = c at h1
        cases c <;> simp only [r2of, reduceCtorEq, R2.found.injEq] at h1
        obtain ⟨e1, e2, e3⟩ := h1
        have : l[j]! = (p, f) := by rw [← e1, ← e2]
        rw [← this, hc, e3]
      · cases h1
    · right; exact h1
  | case2 => intro hr; left; exact hr
  | case3 => intro hr; left; exact hr

theorem scanIn_found (cs : Nat × Bool → StepR) (L : List (List (Nat × Bool))) (h : Nat × Bool) (out : R2) (i : Nat)
    (p : Nat) (f : Bool) (k : Nat) (hr : scanIn cs L h out i = .found p f k) :
    out = .found p f k ∨ cs (p, f) = .found k := by
  revert hr
  fun_induction scanIn cs L h out i with
  | case1 i hi ih =>
    intro hr
    rcases ih hr with h1 | h1
    · right; rcases soIn_found cs _ h _ 0 p f k h1 with h2 | h2
      · cases h2
      · exact h2
    · right; exact h1
  | case2 => intro hr; left; exact hr
  | case3 => intro hr; left; exact hr

theorem soIn_go (cs : Nat × Bool → StepR) (l : List (Nat × Bool)) (h : Nat × Bool) (j : Nat)
    (hx : ∀ o ∈ l, cs o = .exh) : soIn cs l h .go j = .go := by
  induction hd : l.length - j generalizing j with
  | zero => unfold soIn; rw [if_neg (by omega)]
  | succ d ih =>
    unfold soIn
    rw [if_pos (by omega)]
    simp only [ite_true]
    have he := hx _ (gmem l j (by omega))
    split
    · rw [he]; exact ih (j + 1) (by omega)
    · exact ih (j + 1) (by omega)

theorem scanIn_go (cs : Nat × Bool → StepR) (L : List (List (Nat × Bool))) (h : Nat × Bool) (i : Nat)
    (hx : ∀ l ∈ L, ∀ o ∈ l, cs o = .exh) : scanIn cs L h .go i = .go := by
  induction hd : L.length - i generalizing i with
  | zero => unfold scanIn; rw [if_neg (by omega)]
  | succ d ih =>
    unfold scanIn
    rw [if_pos (by omega)]
    simp only [ite_true]
    rw [soIn_go cs _ h 0 (hx _ (gmem L i (by omega)))]
    exact ih (i + 1) (by omega)

theorem ipRef_step (L : List (List (Nat × Bool))) (sm : Nat → Bool) (cs : Nat × Bool → StepR)
    (fbf : Nat → Nat → Bool) (p : Nat) (f : Bool) (k : Nat) (h : ipRef L sm cs fbf = .step p f k) :
    cs (p, f) = .found k := by
  unfold ipRef at h
  split at h
  · cases h
  split at h
  · cases h
  simp only at h
  split at h
  · cases h
  · rename_i k' hk; cases h; exact hk
  · split at h
    · split at h <;> cases h
    · cases h
    · rename_i p' f' k' hs
      cases h
      rcases scanIn_found cs L _ _ 0 p f k hs with h1 | h1
      · cases h1
      · exact h1

/-! ## (b): the first clear step -/

/-- Step `k` off half-edge `(p, f)` is clear: every other piece of the region is at least the
step from its probe. -/
def ClearAt (loops : Loops) (clear : T4) (p : Nat) (f : Bool) (k : Nat) : Prop :=
  ∀ l ∈ LV loops, ∀ o ∈ l, o.1 ≠ p → cl clear p (fiN f) k o.1 = true

theorem csRef_first (loops : Loops) (nsteps : alloc.vec.Vec Usize) (probed : T3) (clear : T4) (p : Nat) (f : Bool) :
    csRef (LV loops) nsteps probed clear p f =
      firstStep (pr probed p (fiN f)) (fun j => clearAllRef (row1 (clr clear p (fiN f) j)) p (LV loops)) (ns nsteps p) 0 := by
  unfold csRef; rw [csIn_first]

theorem clearAt_iff (loops : Loops) (clear : T4) (p : Nat) (f : Bool) (k : Nat) :
    clearAllRef (row1 (clr clear p (fiN f) k)) p (LV loops) = true ↔ ClearAt loops clear p f k := by
  rw [clearAllRef_iff]; rfl

/-- **(b) The first clear step.** A step result off `(p, f)` at `k`: step `k` is tried, probed and
clear, and every earlier step was probed and not clear. -/
theorem first_clear_step (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool)) (p : Nat) (f : Bool) (k : Nat)
    (h : ipOf loops small nsteps probed clear fallback = .step p f k) :
    k < ns nsteps p ∧ pr probed p (fiN f) k = true ∧ ClearAt loops clear p f k ∧
      ∀ j < k, pr probed p (fiN f) j = true ∧ ¬ ClearAt loops clear p f j := by
  have hc := ipRef_step _ _ _ _ p f k h
  simp only at hc
  rw [csRef_first] at hc
  obtain ⟨-, h2, h3, h4, h5⟩ := firstStep_found _ _ _ 0 k hc
  refine ⟨h2, h3, (clearAt_iff _ _ _ _ _).1 h4, fun j hj => ⟨(h5 j (Nat.zero_le _) hj).1, ?_⟩⟩
  rw [← clearAt_iff]; simp [(h5 j (Nat.zero_le _) hj).2]

/-- The half-edge `interior_point` chooses (6152-6162). -/
def chosen (loops : Loops) (small : alloc.vec.Vec Bool) : Nat × Bool := pickRef (LV loops)[0]! (sm small)

/-- **(b) The chosen half-edge gets its first clear step**, when its probes are all defined and
some step is clear. -/
theorem chosen_gets_step (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool))
    (hL : (LV loops).length ≠ 0) (hL0 : (LV loops)[0]!.length ≠ 0)
    (hpr : ∀ j < ns nsteps (chosen loops small).1, pr probed (chosen loops small).1 (fiN (chosen loops small).2) j = true)
    (m : Nat) (hm : m < ns nsteps (chosen loops small).1)
    (hc : ClearAt loops clear (chosen loops small).1 (chosen loops small).2 m) :
    ∃ k, ipOf loops small nsteps probed clear fallback = .step (chosen loops small).1 (chosen loops small).2 k ∧
      k ≤ m ∧ ClearAt loops clear (chosen loops small).1 (chosen loops small).2 k ∧
      ∀ j < k, ¬ ClearAt loops clear (chosen loops small).1 (chosen loops small).2 j := by
  obtain ⟨k0, h1, -, h3, h4, h5⟩ := firstStep_of (pr probed (chosen loops small).1 (fiN (chosen loops small).2))
    (fun j => clearAllRef (row1 (clr clear (chosen loops small).1 (fiN (chosen loops small).2) j))
      (chosen loops small).1 (LV loops)) _ 0 (fun j _ hj => hpr j hj) m (Nat.zero_le _) hm
    ((clearAt_iff _ _ _ _ _).2 hc)
  refine ⟨k0, ?_, h3, (clearAt_iff _ _ _ _ _).1 h4, fun j hj => ?_⟩
  · unfold ipOf ipRef
    rw [if_neg hL, if_neg hL0]
    simp only
    rw [csRef_first]
    unfold chosen at h1
    rw [h1]
    rfl
  · rw [← clearAt_iff]; simp [h5 j (Nat.zero_le _) hj]

/-! ## (c): the strip theorem -/

/-- The `k`-th step of `n`, in units of the last: `2^(n-1-k)` (Coverage.Probe.step, u = 1). -/
def step (n k : Nat) : Nat := 2 ^ (n - 1 - k)

/-- A strip of width `w` (in units of the last step) on every half-edge of the region: all
`n` steps are tried and probed, and step `k`'s probe is clear of every piece but the half-edge's
opposite side `opp p`, which it clears iff `2 * step n k ≤ w` (the probe is `step n k` in, so
`w - step n k` from the far side). -/
structure Strip (loops : Loops) (nsteps : alloc.vec.Vec Usize) (probed : T3) (clear : T4)
    (n w : Nat) (opp : Nat → Nat) : Prop where
  hn : ∀ l ∈ LV loops, ∀ o ∈ l, ns nsteps o.1 = n
  hp : ∀ l ∈ LV loops, ∀ o ∈ l, ∀ k < n, pr probed o.1 (fiN o.2) k = true
  hc : ∀ l ∈ LV loops, ∀ o ∈ l, ∀ k < n, ∀ l' ∈ LV loops, ∀ q ∈ l', q.1 ≠ o.1 →
    (cl clear o.1 (fiN o.2) k q.1 = true ↔ (q.1 ≠ opp o.1 ∨ 2 * step n k ≤ w))
  ho : ∀ l ∈ LV loops, ∀ o ∈ l, opp o.1 ≠ o.1 ∧ ∃ l' ∈ LV loops, ∃ q ∈ l', q.1 = opp o.1

theorem strip_clear (loops : Loops) (nsteps : alloc.vec.Vec Usize) (probed : T3) (clear : T4)
    (n w : Nat) (opp : Nat → Nat) (hS : Strip loops nsteps probed clear n w opp)
    (l : List (Nat × Bool)) (hl : l ∈ LV loops) (o : Nat × Bool) (hoo : o ∈ l) (k : Nat) (hk : k < n) :
    ClearAt loops clear o.1 o.2 k ↔ 2 * step n k ≤ w := by
  constructor
  · intro h
    obtain ⟨hne, l', hl', q, hq, hqo⟩ := hS.ho l hl o hoo
    have := (hS.hc l hl o hoo k hk l' hl' q hq (by rw [hqo]; exact hne)).1 (h l' hl' q hq (by rw [hqo]; exact hne))
    rcases this with h1 | h1
    · exact absurd hqo h1
    · exact h1
  · intro h l' hl' q hq hne
    exact (hS.hc l hl o hoo k hk l' hl' q hq hne).2 (Or.inr h)

theorem chosen_mem (loops : Loops) (small : alloc.vec.Vec Bool) (hL : (LV loops).length ≠ 0)
    (hL0 : (LV loops)[0]!.length ≠ 0) : (LV loops)[0]! ∈ LV loops ∧ chosen loops small ∈ (LV loops)[0]! :=
  ⟨gmem _ 0 (by omega), pickRef_mem _ _ (by omega)⟩

theorem two_le_step (n k : Nat) : 2 ≤ 2 * step n k := by
  unfold step; have := Nat.one_le_two_pow (n := n - 1 - k); omega

/-- **(c) The strip theorem, first fit.** In a strip of width `w ≥ 2` (twice the last step),
the chosen half-edge gets a step, at the first (largest) step `k` with `2 * step n k ≤ w`. -/
theorem strip_first_fit (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool)) (n w : Nat) (opp : Nat → Nat)
    (hS : Strip loops nsteps probed clear n w opp) (hL : (LV loops).length ≠ 0)
    (hL0 : (LV loops)[0]!.length ≠ 0) (hn : 0 < n) (hw : 2 ≤ w) :
    ∃ k, ipOf loops small nsteps probed clear fallback = .step (chosen loops small).1 (chosen loops small).2 k ∧
      k < n ∧ 2 * step n k ≤ w ∧ ∀ j < k, w < 2 * step n j := by
  obtain ⟨h0m, hcm⟩ := chosen_mem loops small hL hL0
  have hnn := hS.hn _ h0m _ hcm
  have hlast : 2 * step n (n - 1) ≤ w := by unfold step; simp; omega
  obtain ⟨k, h1, h2, h3, h4⟩ := chosen_gets_step loops small nsteps probed clear fallback hL hL0
    (fun j hj => hS.hp _ h0m _ hcm j (by omega)) (n - 1) (by omega)
    ((strip_clear loops nsteps probed clear n w opp hS _ h0m _ hcm (n - 1) (by omega)).2 hlast)
  refine ⟨k, h1, by omega, (strip_clear loops nsteps probed clear n w opp hS _ h0m _ hcm k (by omega)).1 h3,
    fun j hj => ?_⟩
  have := h4 j hj
  rw [strip_clear loops nsteps probed clear n w opp hS _ h0m _ hcm j (by omega)] at this
  omega

/-- **(c) The strip theorem, no fit.** In a strip narrower than twice the last step no
half-edge gets a step: `interior_point` falls back to the default probe (or returns none). -/
theorem strip_no_fit (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool)) (n w : Nat) (opp : Nat → Nat)
    (hS : Strip loops nsteps probed clear n w opp) (hw : w < 2) (p : Nat) (f : Bool) (k : Nat) :
    ipOf loops small nsteps probed clear fallback ≠ .step p f k := by
  have hex : ∀ l ∈ LV loops, ∀ o ∈ l, csRef (LV loops) nsteps probed clear o.1 o.2 = .exh := by
    intro l hl o ho
    rw [csRef_first, hS.hn l hl o ho]
    apply firstStep_exh
    intro j _ hj
    refine ⟨hS.hp l hl o ho j hj, ?_⟩
    cases hc : clearAllRef (row1 (clr clear o.1 (fiN o.2) j)) o.1 (LV loops)
    · rfl
    · have := (strip_clear loops nsteps probed clear n w opp hS l hl o ho j hj).1 ((clearAt_iff _ _ _ _ _).1 hc)
      have := two_le_step n j; omega
  intro h
  have := ipRef_step _ _ _ _ p f k h
  simp only at this
  unfold ipOf ipRef at h
  split at h
  · cases h
  split at h
  · cases h
  rename_i hL hL0
  obtain ⟨h0m, hcm⟩ := chosen_mem loops small hL hL0
  simp only at h
  have hx := hex _ h0m _ hcm
  unfold chosen at hx
  rw [hx] at h
  simp only at h
  have hg := scanIn_go (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (LV loops)
    (pickRef (LV loops)[0]! (sm small)) 0 hex
  rw [hg] at h
  simp only at h
  split at h <;> cases h

/-- **(c) The strip theorem.** The chosen half-edge gets a step iff the strip is at least twice
the last step wide. At a half chord ≥ 5 the kernel tries n = 12 steps, 0.05·2^-k mm for k < 12,
the last u = 0.05·2^-11 mm. Lengths here are in units of u: a real width W reads as ⌊W/u⌋, exact
because `2 * step n k` is a whole number of units. So `interior_point` probes the strip iff
W ≥ 2u = 0.05·2^-10 mm = 4.8828125 LINEAR, G3/G4's edge (Coverage.Probe.g3_edge). -/
theorem strip_step_iff (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool)) (n w : Nat) (opp : Nat → Nat)
    (hS : Strip loops nsteps probed clear n w opp) (hL : (LV loops).length ≠ 0)
    (hL0 : (LV loops)[0]!.length ≠ 0) (hn : 0 < n) :
    (∃ k, ipOf loops small nsteps probed clear fallback = .step (chosen loops small).1 (chosen loops small).2 k) ↔
      2 ≤ w := by
  constructor
  · rintro ⟨k, hk⟩
    by_contra hw
    exact strip_no_fit loops small nsteps probed clear fallback n w opp hS (by omega) _ _ k hk
  · intro hw
    obtain ⟨k, hk, -⟩ := strip_first_fit loops small nsteps probed clear fallback n w opp hS hL hL0 hn hw
    exact ⟨k, hk⟩

/-- The kernel's case: n = 12 steps; the edge is w = 2 units. -/
theorem g3_edge_code (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool)) (w : Nat) (opp : Nat → Nat)
    (hS : Strip loops nsteps probed clear 12 w opp) (hL : (LV loops).length ≠ 0)
    (hL0 : (LV loops)[0]!.length ≠ 0) :
    (∃ k, ipOf loops small nsteps probed clear fallback = .step (chosen loops small).1 (chosen loops small).2 k) ↔
      2 ≤ w :=
  strip_step_iff loops small nsteps probed clear fallback 12 w opp hS hL hL0 (by omega)

end clearstep
