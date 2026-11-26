/-!
# H.batch: the n-ary batch verdict is sound when it decides (kernel/forge-bool/src/nary.rs)

`Batch::verdicts` gives each region point a code: 0 dropped, 1 kept, 2 kept flipped, 3 undecided
(the full way decides: today's normal route, G10). The contract (ledger H.batch, Dn):

  a code other than 3 is the exact verdict of the op fold at the point.

`CpuBatch` is the shipped implementation. Its arithmetic is in `Shape::member` (an analytic solid's
membership with a margin) and the tools' boxes; this file proves the rest, the part that is
combinatorics: the fold over the tools in order, the own tool forced in and out, the early 3, and
the grid's skipping of tools whose box misses the point (only without an intersect). The geometric
hypotheses are named: `MemberSound` (an `In`/`Out` answer is the exact membership) and `BoxSound`
(a point outside a tool's box is outside the tool). Core Lean only.
-/

namespace ForgeBoolFallback.Batch

inductive Op where
  | union
  | subtract
  | intersect
  deriving DecidableEq, Repr

/-- `Shape::member`'s answer: `Near` is "too near the boundary to tell". -/
inductive Member where
  | inside
  | outside
  | near
  deriving DecidableEq, Repr

/-- One fold step: tool k's (inner, outer) membership folded into the running (vin, vout), by
`ops[k-1]` (nary.rs `CpuBatch::verdicts`, `apply`). -/
def step : Op → Bool × Bool → Bool × Bool → Bool × Bool
  | .union, (vi, vo), (bi, bo) => (vi || bi, vo || bo)
  | .subtract, (vi, vo), (bi, bo) => (vi && !bi, vo && !bo)
  | .intersect, (vi, vo), (bi, bo) => (vi && bi, vo && bo)

/-- The code of a folded pair: 0 dropped, 1 kept, 2 kept flipped. -/
def code (v : Bool × Bool) : Nat :=
  if v.1 = v.2 then 0 else if v.1 then 1 else 2

/-- Tool k's exact (inner, outer) at the point: the own tool is held just inside and just outside
its own face; another tool by its exact membership `mem k`. -/
def pairOf (own : Nat) (mem : Nat → Bool) (k : Nat) : Bool × Bool :=
  if k = own then (true, false) else (mem k, mem k)

/-- Fold tool k into the state: tool 0 sets it, tool k ≥ 1 steps by `ops[k-1]`. -/
def put (ops : List Op) (s : Bool × Bool) (k : Nat) (x : Bool × Bool) : Bool × Bool :=
  match k with
  | 0 => x
  | k + 1 => match ops[k]? with
    | some o => step o s x
    | none => s

/-- The exact verdict: every tool 0..n-1 in turn, from (false, false). -/
def exactFold (ops : List Op) (own : Nat) (mem : Nat → Bool) (ks : List Nat) : Bool × Bool :=
  ks.foldl (fun s k => put ops s k (pairOf own mem k)) (false, false)

def exactCode (ops : List Op) (own : Nat) (mem : Nat → Bool) (n : Nat) : Nat :=
  code (exactFold ops own mem (List.range n))

/-- `apply(k)` of CpuBatch: the own tool forced; a tool whose box misses the point counts out
(no member call); else `member`, `Near` stops the fold (the code is 3). -/
def applyK (own : Nat) (inBox : Nat → Bool) (member : Nat → Member) (k : Nat) :
    Option (Bool × Bool) :=
  if k = own then some (true, false)
  else if !inBox k then some (false, false)
  else match member k with
    | .near => none
    | .inside => some (true, true)
    | .outside => some (false, false)

/-- The batch's fold over the tools `ks` in that order: `none` is code 3. -/
def cpuFold (ops : List Op) (own : Nat) (inBox : Nat → Bool) (member : Nat → Member) :
    List Nat → Bool × Bool → Option (Bool × Bool)
  | [], s => some s
  | k :: ks, s => match applyK own inBox member k with
    | none => none
    | some x => cpuFold ops own inBox member ks (put ops s k x)

/-- CpuBatch's code for one point: the tools `ks` it walks (all of 0..n-1 with an intersect; the
grid's list with the own tool in its place otherwise). -/
def cpuCode (ops : List Op) (own : Nat) (inBox : Nat → Bool) (member : Nat → Member)
    (ks : List Nat) : Nat :=
  match cpuFold ops own inBox member ks (false, false) with
  | none => 3
  | some v => code v

/-- `Shape::member` is sound when it decides: `In` and `Out` are the exact membership. -/
def MemberSound (member : Nat → Member) (mem : Nat → Bool) : Prop :=
  ∀ k, (member k = .inside → mem k = true) ∧ (member k = .outside → mem k = false)

/-- A tool's box holds the tool: outside the box is outside the tool. -/
def BoxSound (inBox : Nat → Bool) (mem : Nat → Bool) : Prop :=
  ∀ k, inBox k = false → mem k = false

/-- Each decided `apply` is the exact pair. -/
theorem applyK_sound {own : Nat} {inBox : Nat → Bool} {member : Nat → Member} {mem : Nat → Bool}
    (hm : MemberSound member mem) (hb : BoxSound inBox mem) {k : Nat} {x : Bool × Bool}
    (h : applyK own inBox member k = some x) : x = pairOf own mem k := by
  unfold applyK at h
  unfold pairOf
  by_cases hk : k = own
  · simp [hk] at h ⊢
    exact h.symm
  · simp only [hk, ite_false] at h ⊢
    cases hbk : inBox k with
    | false =>
      simp [hbk] at h
      rw [hb k hbk]
      exact h.symm
    | true =>
      simp only [hbk, Bool.not_true] at h
      cases hmk : member k with
      | near => simp [hmk] at h
      | inside =>
        simp [hmk] at h
        rw [(hm k).1 hmk]
        exact h.symm
      | outside =>
        simp [hmk] at h
        rw [(hm k).2 hmk]
        exact h.symm

/-- The fold, where it decides, is the exact fold over the same tools. -/
theorem cpuFold_sound {ops : List Op} {own : Nat} {inBox : Nat → Bool} {member : Nat → Member}
    {mem : Nat → Bool} (hm : MemberSound member mem) (hb : BoxSound inBox mem) :
    ∀ (ks : List Nat) (s v : Bool × Bool), cpuFold ops own inBox member ks s = some v →
      v = ks.foldl (fun s k => put ops s k (pairOf own mem k)) s := by
  intro ks
  induction ks with
  | nil =>
    intro s v h
    simp [cpuFold] at h
    simp [h]
  | cons k ks ih =>
    intro s v h
    simp only [cpuFold] at h
    cases hx : applyK own inBox member k with
    | none => simp [hx] at h
    | some x =>
      simp only [hx] at h
      have hx' := applyK_sound (own := own) hm hb hx
      subst hx'
      simpa [List.foldl] using ih _ v h

/-- **H.batch for CpuBatch, all tools walked** (the case with an intersect): a code other than 3 is
the exact verdict. -/
theorem cpuCode_sound_all {ops : List Op} {own : Nat} {inBox : Nat → Bool}
    {member : Nat → Member} {mem : Nat → Bool} (hm : MemberSound member mem)
    (hb : BoxSound inBox mem) (n : Nat) (h3 : cpuCode ops own inBox member (List.range n) ≠ 3) :
    cpuCode ops own inBox member (List.range n) = exactCode ops own mem n := by
  unfold cpuCode at h3 ⊢
  unfold exactCode exactFold
  cases hf : cpuFold ops own inBox member (List.range n) (false, false) with
  | none => simp [hf] at h3
  | some v =>
    simp only
    rw [cpuFold_sound hm hb _ _ _ hf]

/-! ## The grid: without an intersect, a tool out at the point changes nothing

`Grid::at(p)` lists, in increasing order, a superset of the tools whose box holds `p`; the batch
walks that list with the own tool put in its place. A tool left out is out at the point (its box
misses it), and folding an out tool by a union or a subtract leaves the state as it was; tool 0
left out leaves the start, (false, false), which is what folding it would set. -/

/-- No op is an intersect. -/
def NoIntersect (ops : List Op) : Prop := ∀ o ∈ ops, o ≠ .intersect

theorem step_out (o : Op) (ho : o ≠ .intersect) (s : Bool × Bool) : step o s (false, false) = s := by
  cases o with
  | union => cases s; simp [step]
  | subtract => cases s; simp [step]
  | intersect => exact absurd rfl ho

/-- Folding an out tool k (not the own one) changes nothing, from any state that tool 0 left. -/
theorem put_out {ops : List Op} (hi : NoIntersect ops) (s : Bool × Bool) (k : Nat) (hk : k ≠ 0)
    : put ops s k (false, false) = s := by
  cases k with
  | zero => exact absurd rfl hk
  | succ k =>
    simp only [put]
    cases h : ops[k]? with
    | none => rfl
    | some o =>
      simp only
      exact step_out o (hi o (List.mem_of_getElem? h)) s

/-- The fold of an out tool is the identity, over a list of tools none of which is tool 0. -/
theorem fold_filter_tail {ops : List Op} (hi : NoIntersect ops) {own : Nat} {mem : Nat → Bool}
    {listed : Nat → Bool} (hl : ∀ k, listed k = false → k ≠ own ∧ mem k = false) :
    ∀ (l : List Nat), (∀ k ∈ l, k ≠ 0) → ∀ s,
      (l.filter listed).foldl (fun s k => put ops s k (pairOf own mem k)) s =
        l.foldl (fun s k => put ops s k (pairOf own mem k)) s := by
  intro l
  induction l with
  | nil => intro _ s; rfl
  | cons k l ih =>
    intro h0 s
    have hk0 : k ≠ 0 := h0 k (List.mem_cons_self ..)
    have ht : ∀ k ∈ l, k ≠ 0 := fun x hx => h0 x (List.mem_cons_of_mem _ hx)
    cases hk : listed k with
    | true =>
      simp only [List.filter_cons, hk, ite_true, List.foldl_cons]
      exact ih ht _
    | false =>
      simp only [List.filter_cons, hk, List.foldl_cons]
      obtain ⟨hown, hmem⟩ := hl k hk
      have hp : pairOf own mem k = (false, false) := by simp [pairOf, hown, hmem]
      rw [hp, put_out hi s k hk0]
      exact ih ht s

/-- **The grid walk refines the full walk** (no intersect): the batch walking only the listed tools
(the grid's list, with the own tool) folds to the exact verdict over all `n` tools, provided every
tool left unlisted is out at the point (its box misses it: `BoxSound`) and is not the own tool. -/
theorem grid_fold_eq {ops : List Op} (hi : NoIntersect ops) {own : Nat} {mem : Nat → Bool}
    {listed : Nat → Bool} (hl : ∀ k, listed k = false → k ≠ own ∧ mem k = false) (n : Nat) :
    exactFold ops own mem ((List.range n).filter listed) = exactFold ops own mem (List.range n) := by
  unfold exactFold
  cases n with
  | zero => rfl
  | succ n =>
    rw [List.range_succ_eq_map]
    have ht : ∀ k ∈ (List.range n).map Nat.succ, k ≠ 0 := by
      intro k hk
      simp only [List.mem_map] at hk
      obtain ⟨a, _, rfl⟩ := hk
      exact Nat.succ_ne_zero a
    cases h0 : listed 0 with
    | true =>
      simp only [List.filter_cons, h0, ite_true, List.foldl_cons]
      exact fold_filter_tail hi hl _ ht _
    | false =>
      simp only [List.filter_cons, h0, List.foldl_cons]
      obtain ⟨hown, hmem⟩ := hl 0 h0
      have hp : pairOf own mem 0 = (false, false) := by simp [pairOf, hown, hmem]
      rw [hp]
      exact fold_filter_tail hi hl _ ht _

/-- **H.batch for CpuBatch with the grid** (no intersect): a code other than 3 is the exact
verdict over all `n` tools. `listed` is the grid's list at the point with the own tool put in;
`hl` is what `Grid::at` and `BoxSound` give: a tool not listed has a box missing the point. -/
theorem cpuCode_sound_grid {ops : List Op} (hi : NoIntersect ops) {own : Nat}
    {inBox : Nat → Bool} {member : Nat → Member} {mem : Nat → Bool} (hm : MemberSound member mem)
    (hb : BoxSound inBox mem) {listed : Nat → Bool} (hl : ∀ k, listed k = false → k ≠ own ∧ mem k = false)
    (n : Nat) (h3 : cpuCode ops own inBox member ((List.range n).filter listed) ≠ 3) :
    cpuCode ops own inBox member ((List.range n).filter listed) = exactCode ops own mem n := by
  unfold cpuCode at h3 ⊢
  unfold exactCode
  cases hf : cpuFold ops own inBox member ((List.range n).filter listed) (false, false) with
  | none => simp [hf] at h3
  | some v =>
    simp only
    rw [cpuFold_sound hm hb _ _ _ hf, ← grid_fold_eq hi hl n]
    rfl

end ForgeBoolFallback.Batch
