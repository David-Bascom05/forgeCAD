import TopoCheck.Model

/-!
# What a valid closed, oriented, manifold B-rep is, combinatorially

`Valid` is the specification. The body is a combinatorial map: the darts are the coedges, `prev`
steps round a face's loop, and `partner` is the involution across an edge.
* Its links agree (`WF`): an edge's uses are coedges of it in loops their faces list once, and
  those loops' coedges name them and are uses of their edges. A loop no face lists is left over
  from an in-place edit (`remove_loop`), no part of the body, and `compact` drops it.
* It is closed: every edge has exactly two uses, so `partner` is a fixed-point-free involution,
  and every loop is a closed walk.
* It is oriented: the two uses of an edge run opposite ways, so `partner` reverses direction.
* It is a manifold: round every vertex, the walk `fans` takes permutes the coedges leaving the
  vertex in one cycle, so the faces there form a single disc (one fan). The walk goes by loop
  order, or by the corner table where that is one to one.
* Every vertex is an end of an edge.
* Its shells partition its faces, the faces at an edge share a shell, and each shell is joined
  through its edges: the shells are the connected pieces.
* Each shell's V − E + 2F − L is even and at most 2: its Euler–Poincaré characteristic 2 − 2g,
  once each face is a disc with holes, a planar domain, which the geometry supplies.
-/

namespace TopoCheck.Body
variable (b : Body)

/-- The links. -/
structure WF : Prop where
  ends : ∀ e < b.edges.size, (b.ed e).start < b.nv ∧ (b.ed e).stop < b.nv
  use_edge : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses, (b.co c).edge = e
  use_loop : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses,
    (b.co c).lp < b.loops.size ∧ c ∈ b.loopCoedges (b.co c).lp ∧ b.listed (b.co c).lp = 1
  loop_lp : ∀ l < b.loops.size, b.listed l = 1 → ∀ c ∈ b.loopCoedges l, (b.co c).lp = l
  loop_use : ∀ l < b.loops.size, b.listed l = 1 → ∀ c ∈ b.loopCoedges l,
    (b.co c).edge < b.edges.size ∧ c ∈ (b.ed (b.co c).edge).uses
  face_loops : ∀ f < b.faces.size, ∀ l ∈ b.faceLoops f, l < b.loops.size ∧ (b.lo l).face = f
  listed_le : ∀ l < b.loops.size, (b.lo l).face < b.faces.size ∧ b.listed l ≤ 1

/-- `n` applications of `f`. -/
def iter (f : Nat → Nat) : Nat → Nat → Nat
  | 0, x => x
  | k + 1, x => iter f k (f x)

/-- One step of a fan walk with corner rule `bef`: across the edge of the coedge closing `c`'s
corner. -/
def step (bef : Nat → Nat) (c : Nat) : Nat := (b.partner (bef c)).getD c

/-- The rotation round a vertex by loop order. -/
def rot : Nat → Nat := b.step b.prev

/-- The corner rule `fans` ends up walking with, for the coedges `frm` leaving one vertex. -/
def fanBefore (corners : Nat → Nat → List (Nat × Nat)) (frm : List Nat) : Nat → Nat :=
  let fs := b.fansBy frm none
  if fs.length < 2 then b.before none
  else
    let v := b.start (frm.headD 0)
    let faces := (fs.flatten.map b.face).eraseDups
    let closes := faces.map fun f => (f, corners f v)
    if closes.all (·.2.isEmpty) then b.before none
    else if b.closesOk (b.before (some closes)) frm then b.before (some closes) else b.before none

/-- Reachable from face `h` through edges. -/
inductive Reach (h : Nat) : Nat → Prop
  | refl : Reach h h
  | step {x y : Nat} : Reach h x → y ∈ b.nbrs x → Reach h y

/-- A valid closed, oriented, manifold B-rep (the combinatorial part). `corners` is the
geometric corner pairing check used. -/
structure Valid (corners : Nat → Nat → List (Nat × Nat)) : Prop where
  wf : b.WF
  /-- Every edge has exactly two uses, with opposite senses. -/
  twoUses : ∀ e < b.edges.size, ∃ x y, (b.ed e).uses = [x, y] ∧ x ≠ y ∧ (b.co x).fwd ≠ (b.co y).fwd
  /-- Every face has a loop. -/
  faceLoops : ∀ f < b.faces.size, b.faceLoops f ≠ []
  /-- Every loop is a non-empty closed walk. -/
  loopsClose : ∀ l < b.loops.size, 0 < (b.lo l).len ∧
    ∀ i < (b.lo l).len, b.stop ((b.lo l).first + i) = b.start ((b.lo l).first + (i + 1) % (b.lo l).len)
  /-- Across each edge, the partner use runs the other way, and partner is an involution. -/
  oriented : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses, ∃ d, b.partner c = some d ∧
    d ∈ (b.ed e).uses ∧ d ≠ c ∧ b.start d = b.stop c ∧ b.stop d = b.start c ∧ b.partner d = some c
  /-- Round each vertex, the fan walk maps the coedges leaving it to coedges leaving it… -/
  fanInto : ∀ v, ∀ c ∈ b.leaving v,
    b.step (b.fanBefore corners (b.leaving v)) c ∈ b.leaving v
  /-- …one to one, so it permutes them… -/
  fanInj : ∀ v, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v,
    b.step (b.fanBefore corners (b.leaving v)) c = b.step (b.fanBefore corners (b.leaving v)) d → c = d
  /-- …in one cycle: one fan, a disc. -/
  oneFan : ∀ v < b.nv, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v,
    ∃ k, iter (b.step (b.fanBefore corners (b.leaving v))) k c = d
  /-- Every vertex is an end of an edge. -/
  noStrays : ∀ v < b.nv, ∃ e < b.edges.size, (b.ed e).start = v ∨ (b.ed e).stop = v
  /-- Each shell lists faces, each naming it… -/
  shellsListed : ∀ s < b.shells.size, b.shellList s ≠ [] ∧
    ∀ f ∈ b.shellList s, f < b.faces.size ∧ b.faceShell f = s
  /-- …and each face is in the shell it names, once… -/
  facesInShell : ∀ f < b.faces.size, b.faceShell f < b.shells.size ∧ (b.shellList (b.faceShell f)).count f = 1
  /-- …the faces at an edge share a shell… -/
  edgeShell : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses, ∀ d ∈ (b.ed e).uses,
    b.faceShell (b.faceOf c) = b.faceShell (b.faceOf d)
  /-- …and each shell is joined through its edges. -/
  connected : ∀ s < b.shells.size, ∀ f ∈ b.shellList s, b.Reach ((b.shellList s).headD 0) f
  /-- Each shell's V − E + 2F − L is even and at most 2. -/
  euler : ∀ s < b.shells.size, b.shellValue (b.shells.getD s []) % 2 = 0 ∧
    b.shellValue (b.shells.getD s []) ≤ 2

end TopoCheck.Body
