import ForgeExact.DecideOnce.Coverage

/-!
# Box ledgers: concrete inputs for the core

Axis-aligned boxes with integer ends in `0 .. K` (after §6.1's snap every coincidence is exact, so
a box family's ledger has its carriers at shared values). The carriers are the planes `x = k`,
`y = k`, `z = k` for `k ≤ K`, class `axis * (K + 1) + k`. A cell is given by its doubled
coordinates (`2k` on plane `k`, `2k + 1` between `k` and `k + 1`, `-1` below 0), and its covector
is read off them (`cov`).

`boxLedger K boxes` is the ledger the decide-once pass gives: each box's six faces, the facets
inside each (the face split at every carrier crossing it), the side cells' witnesses, and the
edge and vertex cells round the pieces. `checkConsistent` decides `Consistent` for a given
membership; `consistent_of_check` is its soundness, so an instance's consistency is a
kernel-checked `decide`.
-/

namespace ForgeExact.DecideOnce

/-- Doubled coordinates of a cell. -/
abbrev DCell := Int × Int × Int

/-- A box: low and high ends per axis. -/
structure BoxN where
  lo : Nat × Nat × Nat
  hi : Nat × Nat × Nat

def axSigns (K : Nat) (a : Int) : List Sgn :=
  (List.range (K + 1)).map fun (k : Nat) =>
    if a < 2 * (k : Int) then .neg else if a = 2 * (k : Int) then .zero else .pos

def cov (K : Nat) (c : DCell) : Cov := axSigns K c.1 ++ axSigns K c.2.1 ++ axSigns K c.2.2

/-- The open intervals between `lo` and `hi`, doubled. -/
def odds (lo hi : Nat) : List Int := (List.range (hi - lo)).map fun t => 2 * ((lo + t : Nat) : Int) + 1

def BoxN.ax (b : BoxN) : Nat → Nat × Nat
  | 0 => (b.lo.1, b.hi.1)
  | 1 => (b.lo.2.1, b.hi.2.1)
  | _ => (b.lo.2.2, b.hi.2.2)

/-- A cell with doubled coordinate `a` on axis `ax` and the others `u`, `v` (in order). -/
def place (ax : Nat) (a u v : Int) : DCell :=
  match ax with
  | 0 => (a, u, v)
  | 1 => (u, a, v)
  | _ => (u, v, a)

/-- The facets inside face `side` of a box: `side = 2 * axis + (0 low | 1 high)`. -/
def faceCells (b : BoxN) (side : Nat) : List DCell :=
  let ax := side / 2
  let k := if side % 2 = 0 then (b.ax ax).1 else (b.ax ax).2
  let o1 := (ax + 1) % 3
  let o2 := (ax + 2) % 3
  let (us, vs) := if o1 < o2 then (odds (b.ax o1).1 (b.ax o1).2, odds (b.ax o2).1 (b.ax o2).2)
    else (odds (b.ax o2).1 (b.ax o2).2, odds (b.ax o1).1 (b.ax o1).2)
  us.flatMap fun u => vs.map fun v => place ax (2 * (k : Int)) u v

/-- A full cell inside a box. -/
def BoxN.holds (b : BoxN) (c : DCell) : Bool :=
  decide (2 * (b.lo.1 : Int) < c.1 ∧ c.1 < 2 * (b.hi.1 : Int)) &&
  decide (2 * (b.lo.2.1 : Int) < c.2.1 ∧ c.2.1 < 2 * (b.hi.2.1 : Int)) &&
  decide (2 * (b.lo.2.2 : Int) < c.2.2 ∧ c.2.2 < 2 * (b.hi.2.2 : Int))

/-- The axis a facet's even coordinate is on. -/
def evenAx (c : DCell) : Nat := if c.1 % 2 = 0 then 0 else if c.2.1 % 2 = 0 then 1 else 2

def bump (ax : Nat) (d : Int) (c : DCell) : DCell :=
  match ax with
  | 0 => (c.1 + d, c.2.1, c.2.2)
  | 1 => (c.1, c.2.1 + d, c.2.2)
  | _ => (c.1, c.2.1, c.2.2 + d)

/-- The pieces of every face: (cell, face index); face `6 i + side` is box `i`'s. -/
def boxPieces (boxes : List BoxN) : List (DCell × Nat) :=
  (List.range boxes.length).flatMap fun i =>
    (List.range 6).flatMap fun side =>
      (faceCells (boxes.getD i ⟨(0, 0, 0), (0, 0, 0)⟩) side).map fun c => (c, 6 * i + side)

def boxLedger (K : Nat) (boxes : List BoxN) : Ledger :=
  let ps := boxPieces boxes
  let cells := dedup (ps.map Prod.fst)
  let sides := dedup (cells.flatMap fun c => [bump (evenAx c) 1 c, bump (evenAx c) (-1) c])
  let es := dedup (cells.flatMap fun c =>
    let a := evenAx c
    let o1 := (a + 1) % 3
    let o2 := (a + 2) % 3
    [bump o1 1 c, bump o1 (-1) c, bump o2 1 c, bump o2 (-1) c])
  let vs := dedup (cells.flatMap fun c =>
    let a := evenAx c
    let o1 := (a + 1) % 3
    let o2 := (a + 2) % 3
    [bump o2 1 (bump o1 1 c), bump o2 (-1) (bump o1 1 c), bump o2 1 (bump o1 (-1) c),
      bump o2 (-1) (bump o1 (-1) c)])
  { nb := boxes.length
    faceBody := (List.range (6 * boxes.length)).map (· / 6)
    faceCls := (List.range (6 * boxes.length)).map fun f =>
      let b := boxes.getD (f / 6) ⟨(0, 0, 0), (0, 0, 0)⟩
      let side := f % 6
      let ax := side / 2
      ax * (K + 1) + (if side % 2 = 0 then (b.ax ax).1 else (b.ax ax).2)
    pieces := ps.map fun p => (cov K p.1, p.2)
    inside := sides.flatMap fun t =>
      ((List.range boxes.length).filter fun i => (boxes.getD i ⟨(0, 0, 0), (0, 0, 0)⟩).holds t).map
        fun i => (cov K t, i)
    edges := es.map (cov K)
    verts := vs.map (cov K) }

/-! ## Consistency, decided -/

namespace Ledger

/-- `Consistent`, for a given membership, as a Bool. -/
def checkConsistent (L : Ledger) (m : Cov → Nat → Bool) : Bool :=
  (L.pieces.all fun p => p.1.isFacet && p.1.zeroIdx == L.clsOf p.2 && decide (L.bodyOf p.2 < L.nb)) &&
  (L.pieces.all fun p => L.pieces.all fun q => !(q.1 == p.1 && L.bodyOf q.2 == L.bodyOf p.2) || q.2 == p.2) &&
  (L.topes.all fun T => (List.range L.nb).all fun i => L.inside.contains (T, i) == m T i) &&
  (L.facets.all fun φ => (List.range L.nb).all fun i => (m φ.plus i != m φ.minus i) == L.isPieceOf i φ)

theorem consistent_of_check {L : Ledger} {m : Cov → Nat → Bool} (h : L.checkConsistent m = true) :
    Consistent L := by
  simp only [checkConsistent, Bool.and_eq_true, List.all_eq_true, beq_iff_eq, decide_eq_true_eq,
    Bool.or_eq_true, Bool.not_eq_true', List.mem_range] at h
  obtain ⟨⟨⟨h1, h2⟩, h3⟩, h4⟩ := h
  refine ⟨fun p hp => ⟨(h1 p hp).1.1, (h1 p hp).1.2, (h1 p hp).2⟩, ?_, m, ?_, ?_⟩
  · intro p hp q hq e1 e2
    rcases h2 p hp q hq with h | h
    · simp [e1, e2] at h
    · exact h
  · intro T hT i hi; exact h3 T hT i hi
  · intro φ hφ i hi
    have := h4 φ hφ i hi
    rw [← this]
    cases m φ.plus i <;> cases m φ.minus i <;> simp

end Ledger

/-- The witness table as a membership. -/
def Ledger.witness (L : Ledger) (T : Cov) (i : Nat) : Bool := L.inside.contains (T, i)

/-! ## A summary of an outcome, for the checks -/

inductive Summary
  | ok (faces facets edges verts : Nat)
  | empty
  | splitCrossed
  | inconsistentClassification
  | zeroThickness (cell : Cov)
  deriving DecidableEq, Repr

def Outcome.summary : Outcome → Summary
  | .ok b => .ok b.faces.length (b.faces.foldl (fun n F => n + F.facets.length) 0) b.edges.length
      b.verts.length
  | .empty => .empty
  | .refused .splitCrossed => .splitCrossed
  | .refused .inconsistentClassification => .inconsistentClassification
  | .refused (.zeroThickness x) => .zeroThickness x

end ForgeExact.DecideOnce
