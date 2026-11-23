import TopoCheck.Model
import TopoCheck.Spec

/-!
# `check` with sheets: the model's changes, `ValidSheet`, and the theorems to prove

For verify/forge-topo-check on claude/repo-review-mbdrlj-fundamentals, which owns `Model`, `Spec`
and `Proof`. This file is written against that branch's `TopoCheck.Model` and `TopoCheck.Spec` and
is in no lakefile here. There is no Lean toolchain in the container that wrote it. The theorems are
stated as `Prop`s, with no `sorry`, for the session that owns the language proofs to prove.

Line references are to `kernel/forge-topo/src/{body,check}.rs` on claude/repo-review-mbdrlj-surfaces.

## What changed in the Rust

* `Face::sheet` (body.rs): set only by what makes a sheet; `Body::has_sheet` is "some live face
  has it". A body with no sheet face takes every line of `check` as before. `sheets` below is
  `has_sheet`, worked out once in `check_known`.
* `check_edges`: an edge with one use whose face is a sheet's is a boundary, with no problem. An
  edge with two uses, one on a sheet face and one not, is `SheetMeetsSolid`.
* `check_fans`: with sheets, the coedges leaving a vertex are walked boundary use first (a stable
  sort by "the edge has not one use"). A chain from the boundary use is one fan, and a second
  boundary use starts a second (a bow tie).
* `check_shell_euler`: a sheet shell's value is V − E + 2F − L + B, B its boundary cycles
  (`boundaries`: the connected pieces of the graph its one-use edges make on their vertices).
* `open_at` and the gate in `check_known` are as before, with `EdgeUses` now not raised for a
  sheet's boundary.
* The route is chosen once: `check_known` calls `check_known_as::<true>` when `has_sheet`, else
  `check_known_as::<false>`, and each sheet rule is guarded by the const `S`. So the `false`
  instance is the solid check with no sheet branch in it (`check_fans` walks `leaving` unsorted),
  and `SolidRouteUnchanged` below is a statement about that instance. For an Aeneas translation
  (L4), the two instances are two functions; the model's `checkS` with `hasSheet = false` is
  `check_known_as::<false>`.

Written against `TopoCheck.Model` as of 3d17b13. Fundamentals has since moved it (25cb295: `fshell`,
`checkLinks`, `checkShellFaces`, `checkStrays`, and `check_sound` without `WF`); `checkS` must take
those passes as `check` does when this file is built against it. Not built here: no Lean toolchain
in this container, and installing one was refused.
-/

namespace TopoCheck.Body
variable (b : Body)

/-- The sheet model: a body with each face's sheet mark, and `boundaries` (check.rs, `fn
boundaries`) as a parameter, `bnd es`: the number of connected pieces of the one-use edges among
`es`. -/
structure Sheets where
  sheet : Nat → Bool
  bnd : List Nat → Nat

variable (s : Sheets)

/-- `Body::has_sheet`. -/
def hasSheet : Bool := (List.range b.faces.size).any s.sheet

/-- Problem `SheetMeetsSolid` is modelled as `edgeUses e 2`'s neighbour: a new constructor in
`Model.Problem`, `sheetMeetsSolid (edge : Nat)`. Here it is written as `edgeUsesSameWay` would be
in a separate list, so that this file needs no change to `Model`. -/
def sheetMeetsSolid (e : Nat) : List Nat := [e]

/-- check.rs `check_edges`, with sheets: the boundary passes, the rest as `checkEdges`. The second
list is the `SheetMeetsSolid` edges. -/
def checkEdgesS : List Problem × List Nat :=
  let sh := b.hasSheet s
  ((List.range b.edges.size).flatMap fun e =>
      match (b.ed e).uses with
      | [x] => if sh && s.sheet (b.face x) then [] else [.edgeUses e 1]
      | [x, y] => if (b.co x).fwd == (b.co y).fwd then [.edgeUsesSameWay e] else []
      | us => [.edgeUses e us.length],
   (List.range b.edges.size).flatMap fun e =>
      match (b.ed e).uses with
      | [x, y] => if sh && s.sheet (b.face x) != s.sheet (b.face y) then b.sheetMeetsSolid e else []
      | _ => [])

/-- The leaving coedges in the order the sheet route walks them: a boundary use first (a stable
sort, as Rust's `sort_by_key` is). -/
def leavingS (v : Nat) : List Nat :=
  if b.hasSheet s then
    let l := b.leaving v
    l.filter (fun c => (b.ed (b.co c).edge).uses.length == 1) ++
      l.filter (fun c => (b.ed (b.co c).edge).uses.length != 1)
  else b.leaving v

/-- check.rs `check_shell_euler` with sheets. -/
def shellValueS (fs : List Nat) : Int :=
  let ls := fs.flatMap b.faceLoops
  let es := (ls.flatMap b.loopCoedges).map fun c => (b.co c).edge
  b.shellValue fs + (if b.hasSheet s && s.sheet (fs.headD 0) then (s.bnd es : Int) else 0)

def checkShellEulerS : List Problem :=
  (List.range b.shells.size).filterMap fun i =>
    let value := b.shellValueS s (b.shells.getD i [])
    if value % 2 != 0 || value > 2 then some (.shellEuler i value) else none

def checkFansS (corners : Nat → Nat → List (Nat × Nat)) : List Problem :=
  (List.range b.nv).filterMap fun v =>
    let n := (b.fans corners (b.leavingS s v)).length
    if n > 1 then some (.vertexFans v n) else none

/-- check.rs `check_known`'s combinatorial part, with sheets (the `SheetMeetsSolid` edges apart). -/
def checkS (corners : Nat → Nat → List (Nat × Nat)) : List Problem × List Nat :=
  let (es, meets) := b.checkEdgesS s
  let out := es ++ b.checkLoops
  if out.any blocks then (out, meets)
  else (out ++ b.checkShellEulerS s ++ b.checkFansS s corners, meets)

/-- The partial rotation round a vertex: across the edge of the coedge before `c`, where it has a
partner. -/
def rotS (c : Nat) : Option Nat := b.partner (b.prev c)

/-- A valid oriented 2-manifold with boundary, combinatorially: a sheet's spec. Every shell of
sheet faces, every edge used once (a boundary, on a sheet face) or twice (opposite senses, both
faces sheet or both not). Round each vertex the leaving coedges are one fan: one cycle, or one
chain from its boundary use. Each sheet shell's V − E + 2F − L + B is even and at most 2. Solid
shells are as `Valid`. -/
structure ValidSheet (corners : Nat → Nat → List (Nat × Nat)) : Prop where
  uses : ∀ e < b.edges.size,
    (∃ x, (b.ed e).uses = [x] ∧ s.sheet (b.face x) = true) ∨
    (∃ x y, (b.ed e).uses = [x, y] ∧ x ≠ y ∧ (b.co x).fwd ≠ (b.co y).fwd ∧
      s.sheet (b.face x) = s.sheet (b.face y))
  faceLoops : ∀ f < b.faces.size, b.faceLoops f ≠ []
  loopsClose : ∀ l < b.loops.size, 0 < (b.lo l).len ∧
    ∀ i < (b.lo l).len, b.stop ((b.lo l).first + i) = b.start ((b.lo l).first + (i + 1) % (b.lo l).len)
  /-- `partner` is none exactly on a one-use edge, and an involution reversing direction on the rest. -/
  oriented : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses,
    ((b.ed e).uses.length = 1 ∧ b.partner c = none) ∨
    ((b.ed e).uses.length = 2 ∧ ∃ d, b.partner c = some d ∧ d ∈ (b.ed e).uses ∧ d ≠ c ∧
      b.start d = b.stop c ∧ b.stop d = b.start c ∧ b.partner d = some c)
  /-- `rotS` maps the leaving coedges into themselves where it is defined, one to one. -/
  rotInto : ∀ v, ∀ c ∈ b.leaving v, ∀ d, b.rotS c = some d → d ∈ b.leaving v
  rotInj : ∀ v, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, ∀ x, b.rotS c = some x → b.rotS d = some x → c = d
  /-- One fan: walked from the boundary use (if any), as `check_fans` walks it, every leaving
  coedge is reached. -/
  oneFan : ∀ v < b.nv, ∀ c ∈ b.leavingS s v,
    ∃ k, iter (b.step (b.fanBefore corners (b.leavingS s v))) k ((b.leavingS s v).headD 0) = c
  euler : ∀ i < b.shells.size, b.shellValueS s (b.shells.getD i []) % 2 = 0 ∧
    b.shellValueS s (b.shells.getD i []) ≤ 2

/-- **Soundness with sheets** (to prove): a body with the back-links whose sheet check is empty, and
no edge where a sheet meets a solid, is a valid sheet-or-solid map. -/
def CheckSoundSheet : Prop :=
  ∀ (b : Body) (s : Sheets) corners, b.WF → b.checkS s corners = ([], []) → b.ValidSheet s corners

/-- **Solids unchanged** (to prove, and the reason the existing `check_sound` carries over): with
no sheet face, the sheet route is the solid one, problem for problem. -/
def SolidRouteUnchanged : Prop :=
  ∀ (b : Body) (s : Sheets) corners, b.hasSheet s = false → b.checkS s corners = (b.check corners, [])

/-- **A sheet's Valid restricted to solids is Valid** (to prove): with no sheet face, `ValidSheet`
is `Valid`, as the one-use and boundary-first branches never apply. -/
def ValidSheetNoSheets : Prop :=
  ∀ (b : Body) (s : Sheets) corners, b.hasSheet s = false →
    (b.ValidSheet s corners ↔ b.Valid corners)

end TopoCheck.Body

/-! ## Mutants for `tests/diff.rs` (the model against the kernel)

For each sheet in `kernel/forge-ops/tests/sheet.rs` and `kernel/forge-model/tests/sheets.rs`:
* a face dropped from an open sheet: still a valid sheet, with B up by one where it opens a new hole;
* two sheets joined at one vertex (a bow tie): `vertexFans 2`;
* a face's sense flipped: `edgeUsesSameWay` on each of its two-use edges;
* the sheet's marks cleared (a solid with a missing face): `edgeUses e 1` on every boundary edge;
* one face's mark cleared in a sheet: `SheetMeetsSolid` on its two-use edges, `edgeUses` on its
  boundary ones;
* one face meeting a boundary vertex twice (fundamentals' case for the corner table's guard: a
  sheet face whose loop passes the same boundary vertex at two corners, as a slit square does at
  the slit's root). If the guard rejects the table, `Body::fans` falls back to loop-order fans and
  the boundary-first sort decides: a valid sheet must still give one fan there, and a bow tie
  `vertexFans 2`.
-/
