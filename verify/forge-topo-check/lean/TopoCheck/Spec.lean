import TopoCheck.Model

/-!
# What a valid closed, oriented, manifold B-rep is, combinatorially

`WF` is what `check` takes as given: the back-links `Body::add_loop` (body.rs:898–914) sets
up. `check` itself never tests them (see the test `back_links_unchecked` in
`verify/forge-topo-check/tests/diff.rs`).

`Valid` is the specification. The body is a combinatorial map: the darts are the coedges, `prev`
steps round a face's loop, and `partner` is the involution across an edge. It is closed: every
edge has exactly two uses, so `partner` is a fixed-point-free involution, and every loop is a
closed walk. It is oriented: the two uses of an edge run opposite ways, so `partner` reverses
direction. It is a manifold: round every vertex, the rotation `rot c = partner (prev c)` permutes
the coedges leaving the vertex in one cycle, so the faces there form a single disc (one fan).
And each shell's V − E + 2F − L is even and at most 2: its Euler–Poincaré characteristic
2 − 2g, once each face is a disc with holes, a planar domain, which the geometry supplies.
-/

namespace TopoCheck.Body
variable (b : Body)

/-- The back-links: an edge's uses are coedges of that edge, each in the loop it names; a loop's
coedges name that loop, and each is among its edge's uses. -/
structure WF : Prop where
  use_edge : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses, (b.co c).edge = e
  use_loop : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses,
    (b.co c).lp < b.loops.size ∧ c ∈ b.loopCoedges (b.co c).lp
  loop_lp : ∀ l < b.loops.size, ∀ c ∈ b.loopCoedges l, (b.co c).lp = l
  loop_use : ∀ l < b.loops.size, ∀ c ∈ b.loopCoedges l,
    (b.co c).edge < b.edges.size ∧ c ∈ (b.ed (b.co c).edge).uses

/-- `n` applications of `f`. -/
def iter (f : Nat → Nat) : Nat → Nat → Nat
  | 0, x => x
  | k + 1, x => iter f k (f x)

/-- One step of a fan walk with corner rule `bef`: across the edge of the coedge closing `c`'s
corner. -/
def step (bef : Nat → Nat) (c : Nat) : Nat := (b.partner (bef c)).getD c

/-- The rotation round a vertex by loop order. -/
def rot : Nat → Nat := b.step b.prev

/-- The corner rule `fans` ends up walking with, for the coedges `frm` leaving one vertex
(body.rs:950–970). -/
def fanBefore (corners : Nat → Nat → List (Nat × Nat)) (frm : List Nat) : Nat → Nat :=
  let fs := b.fansBy frm none
  if fs.length < 2 then b.before none
  else
    let v := b.start (frm.headD 0)
    let faces := (fs.flatten.map b.face).eraseDups
    let closes := faces.map fun f => (f, corners f v)
    if closes.all (·.2.isEmpty) then b.before none else b.before (some closes)

/-- A valid closed, oriented, manifold B-rep (the combinatorial part). `corners` is the
geometric corner pairing check used; with none, `rot` is the fan rule. -/
structure Valid (corners : Nat → Nat → List (Nat × Nat)) : Prop where
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
  /-- Round each vertex, `rot` maps the coedges leaving it to coedges leaving it… -/
  rotInto : ∀ v, ∀ c ∈ b.leaving v, b.rot c ∈ b.leaving v
  /-- …one to one, so it permutes them. -/
  rotInj : ∀ v, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, b.rot c = b.rot d → c = d
  /-- One fan: every coedge leaving a vertex is reached from the first by the walk `fans` takes. -/
  oneFan : ∀ v < b.nv, ∀ c ∈ b.leaving v,
    ∃ k, iter (b.step (b.fanBefore corners (b.leaving v))) k ((b.leaving v).headD 0) = c
  /-- Each shell's V − E + 2F − L is even and at most 2. -/
  euler : ∀ s < b.shells.size, b.shellValue (b.shells.getD s []) % 2 = 0 ∧
    b.shellValue (b.shells.getD s []) ≤ 2

end TopoCheck.Body
