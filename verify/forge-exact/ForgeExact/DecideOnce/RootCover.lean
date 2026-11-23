import ForgeExact.QRef.Decide

/-!
# A certified cover of a polynomial's real roots (Q1 L2, for completeness)

`QRef.realRoots` proposes isolating intervals, but it is not complete: an interval still split at
the fuel limit is dropped without a word. The curved ledger's completeness (every crossing is a
root of `R`: `bezout_spec`, `bezout3_spec`) needs every root found, so the roots are taken from a
**checked cover** instead:

* breakpoints `t₀ < t₁ < … < tₘ` (rational, proposed by anyone);
* each gap `(tᵢ, tᵢ₊₁)` certified root-free (`noPosRoot` of the Möbius transform) or holding
  exactly one root (`Iso.ok`), which is then that gap's number;
* both rays certified root-free: `p(t₀ − s)` and `p(tₘ + s)` have no positive root (Descartes'
  one-sign case, `noPosRoot`), so no Cauchy bound has to be proven.

**`cover_complete`**: if the cover checks, every root of `p` in any model is denoted by one of
`coverRoots p ts` (a rational breakpoint where `p` vanishes, or a gap's isolated root). A cover
that does not check answers nothing (the bridge's `Skip::Budget`), never a missing root.
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.Alg Lean.Grind

/-- The last breakpoint. -/
def lastOf : Rat → List Rat → Rat
  | a, [] => a
  | _, b :: r => lastOf b r

/-- A gap is root-free or isolates one root. -/
def gapOk (p : Poly) (a b : Rat) : Bool :=
  decide (a < b) && (noPosRoot (moebius a b p) || Iso.ok ⟨p, a, b⟩)

def gapsOk (p : Poly) : Rat → List Rat → Bool
  | _, [] => true
  | a, b :: r => gapOk p a b && gapsOk p b r

/-- The cover `t₀ :: ts` checks: rays root-free, every gap certified. -/
def coverOk (p : Poly) (t0 : Rat) (ts : List Rat) : Bool :=
  noPosRoot (pcomp p [t0, -1]) && noPosRoot (pcomp p [lastOf t0 ts, 1]) && gapsOk p t0 ts

def gapRoots (p : Poly) : Rat → List Rat → List Num
  | _, [] => []
  | a, b :: r => (if noPosRoot (moebius a b p) then [] else [.iso ⟨p, a, b⟩]) ++ gapRoots p b r

/-- The cover's roots: the breakpoints where `p` vanishes, and the gaps' isolated roots. -/
def coverRoots (p : Poly) (t0 : Rat) (ts : List Rat) : List Num :=
  ((t0 :: ts).filter fun t => Poly.ev p t == 0).map .rat ++ gapRoots p t0 ts

theorem lastOf_mem : ∀ (a : Rat) (r : List Rat), lastOf a r ∈ a :: r
  | a, [] => by simp [lastOf]
  | _, b :: r => by simp only [lastOf]; exact List.mem_cons_of_mem _ (lastOf_mem b r)

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_lin1 (a s : Rat) (x : K) : evalWith M.ι [a, s] x = M.q a + x * M.q s := by
  simp only [evalWith_cons, evalWith_nil, Model.q]; grind

/-- Between consecutive breakpoints, a root is a breakpoint or a gap's number. -/
theorem gaps_complete (p : Poly) {x : K} (hx : M.ev p x = 0) :
    ∀ (a : Rat) (r : List Rat), gapsOk p a r = true → M.q a < x → x < M.q (lastOf a r) →
      (∃ s ∈ r, x = M.q s) ∨ ∃ n ∈ gapRoots p a r, Denotes M n x
  | a, [], _, h1, h2 => by simp only [lastOf] at h2; exact absurd (Std.lt_trans h1 h2) Std.lt_irrefl
  | a, b :: r, hg, h1, h2 => by
    simp only [gapsOk, Bool.and_eq_true] at hg
    obtain ⟨hab, hrest⟩ := hg
    rcases LinearOrder.trichotomy x (M.q b) with hb | hb | hb
    · -- in the gap (a, b)
      right
      simp only [gapOk, Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq] at hab
      obtain ⟨_, hn | hiso⟩ := hab
      · exact absurd hx (moebius_none M a b p hn h1 hb)
      · by_cases hnp : noPosRoot (moebius a b p) = true
        · exact absurd hx (moebius_none M a b p hnp h1 hb)
        · refine ⟨.iso ⟨p, a, b⟩, ?_, ?_⟩
          · simp [gapRoots, hnp]
          · simp only [Iso.ok, Bool.and_eq_true, decide_eq_true_eq] at hiso
            refine ⟨h1, hb, hx, fun y hya hyb hy => ?_, hiso.1.2⟩
            exact moebius_unique M a b p hiso.2 hya hyb hy h1 hb hx
    · exact Or.inl ⟨b, List.mem_cons_self .., hb⟩
    · have ih := gaps_complete p hx b r hrest hb (by simpa [lastOf] using h2)
      rcases ih with ⟨s, hs, e⟩ | ⟨n, hn, e⟩
      · exact Or.inl ⟨s, List.mem_cons_of_mem _ hs, e⟩
      · exact Or.inr ⟨n, by simp only [gapRoots, List.mem_append]; exact Or.inr hn, e⟩

/-- **Every root is in the cover.** -/
theorem cover_complete {p : Poly} {t0 : Rat} {ts : List Rat} (h : coverOk p t0 ts = true) {x : K}
    (hx : M.ev p x = 0) : ∃ n ∈ coverRoots p t0 ts, Denotes M n x := by
  simp only [coverOk, Bool.and_eq_true] at h
  obtain ⟨⟨hlo, hhi⟩, hg⟩ := h
  -- not below t₀
  have hge : ¬x < M.q t0 := by
    intro hlt
    have hs : 0 < M.q t0 - x := by grind
    apply noPosRoot_spec M _ hlo hs
    simp only [Model.ev, eval_pcomp, ev_lin1]
    have : M.q t0 + (M.q t0 - x) * M.q (-1) = x := by rw [M.q_neg, M.q_one]; grind
    rw [this]; exact hx
  -- not above the last
  have hle : ¬M.q (lastOf t0 ts) < x := by
    intro hlt
    have hs : 0 < x - M.q (lastOf t0 ts) := by grind
    apply noPosRoot_spec M _ hhi hs
    simp only [Model.ev, eval_pcomp, ev_lin1]
    have : M.q (lastOf t0 ts) + (x - M.q (lastOf t0 ts)) * M.q 1 = x := by rw [M.q_one]; grind
    rw [this]; exact hx
  have hbreak : ∀ s ∈ t0 :: ts, x = M.q s → ∃ n ∈ coverRoots p t0 ts, Denotes M n x := by
    intro s hs e
    refine ⟨.rat s, ?_, e⟩
    simp only [coverRoots, List.mem_append, List.mem_map, List.mem_filter, beq_iff_eq]
    left
    refine ⟨s, ⟨hs, ?_⟩, rfl⟩
    have := M.ev_q s p
    rw [← e, hx] at this
    exact (M.q_eq (a := 0) (b := Poly.ev p s)).mp (by rw [M.q_zero]; exact this) |>.symm
  rcases LinearOrder.trichotomy x (M.q t0) with h0 | h0 | h0
  · exact absurd h0 hge
  · exact hbreak t0 (List.mem_cons_self ..) h0
  · rcases LinearOrder.trichotomy x (M.q (lastOf t0 ts)) with h1 | h1 | h1
    · rcases gaps_complete M p hx t0 ts hg h0 h1 with ⟨s, hs, e⟩ | ⟨n, hn, e⟩
      · exact hbreak s (List.mem_cons_of_mem _ hs) e
      · exact ⟨n, by simp only [coverRoots, List.mem_append]; exact Or.inr hn, e⟩
    · exact hbreak _ (lastOf_mem t0 ts) h1
    · exact absurd h1 hle

end Model

/-! ## The proposer (untrusted: only `coverOk` is believed) -/

/-- Split `(a, b)` until every gap checks; the interior breakpoints, in order. -/
def splitGap (p : Poly) : Nat → Rat → Rat → Option (List Rat)
  | 0, a, b => if gapOk p a b then some [] else none
  | n + 1, a, b =>
    if gapOk p a b then some []
    else
      let m := (a + b) / 2
      match splitGap p n a m, splitGap p n m b with
      | some l, some r => some (l ++ m :: r)
      | _, _ => none

/-- The rays' bound: the Cauchy bound, doubled until both rays check. -/
def rayBound (p : Poly) : Nat → Rat → Option Rat
  | 0, _ => none
  | n + 1, B =>
    if noPosRoot (pcomp p [-B, -1]) && noPosRoot (pcomp p [B, 1]) then some B
    else rayBound p n (2 * B)

/-- A proposed cover `(t₀, rest)`, checked: `none` if it does not check within the fuel. -/
def proposeCover (p : Poly) (fuel : Nat) : Option (Rat × List Rat) := do
  let B ← rayBound p fuel (QRef.rootBound p)
  let mid ← splitGap p fuel (-B) B
  let ts := mid ++ [B]
  if coverOk p (-B) ts then some (-B, ts) else none

/-- The roots, with their completeness certificate checked (`cover_complete`). -/
def certifiedRoots (p : Poly) (fuel : Nat) : Option (List Num) :=
  (proposeCover p fuel).map fun c => coverRoots p c.1 c.2

-- y² − 1: the roots ±1, both found
#guard (certifiedRoots [-1, 0, 1] 30).map List.length = some 2
-- x³ − 2x: 0 and ±√2
#guard (certifiedRoots [0, -2, 0, 1] 30).map List.length = some 3
-- x² + 1: none
#guard (certifiedRoots [1, 0, 1] 30).map List.length = some 0

end ForgeExact.DecideOnce.Curved
