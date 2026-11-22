import ForgeExact.DecideOnce.Closure

/-!
# The decide-once flat-face boolean: the combinatorial core

A function of the inputs' names and the ledger only (`core : Ledger → Rule → Outcome`). It takes no
point, plane or number: every geometric fact it uses is an entry of the ledger, decided once before
it runs (DecideOnce/Ledger.lean), and every use is a lookup.

## Cells

The carriers, after §6.1's snap, are the classes `0 .. n-1` (one plane each, the representative's).
A cell of their arrangement is named by its covector (`Cov`): its side of every class, `neg`, `zero`
(on it) or `pos`. Two cells are one cell iff their covectors are equal, so a cell is decided once and
is the same object wherever it is reached: a cut point found from an edge of A and from an edge of B
is one vertex because it is one covector, not because two distances were compared.

* a *facet* has exactly one zero (it lies in that one carrier); its two *sides* (`plus`, `minus`)
  are the full cells just off it;
* `X ≤ Y` (`Cov.le`): X is a face of Y (X agrees with Y wherever X is not zero);
* edges and vertices are the cells the ledger lists as such (their dimension is decided once,
  exactly, from the carriers through them).

## The stages

1. **split** (`facets`): each input face is the set of facets the ledger decided lie inside it
   (`pieces`). `splitOk` refuses `SplitCrossed` if a piece is not a facet of the face's own
   carrier, or one facet is inside two faces of one body.
2. **imprint**: a facet inside faces of both bodies (coplanar overlap) is one cell, by its key.
3. **classify** (`memb`): for body `i`, the side cells (`topes`) are flooded across every facet that
   is not a piece of body `i` (being in `i` cannot change there), and each flood class is
   classified once, at its first member, by the ledger's witness (`inside`). `classifyOk` refuses
   `InconsistentClassification` if some piece of body `i` has both sides alike.
4. **keep**: a cell is kept by the op's pointwise rule on its memberships, decided once per side
   cell into a table (`keepTab`) that every later stage looks up.
5. **assemble** (`result`): the facets with exactly one side kept, facing the dropped side
   (`orient`).
6. **touch rule**: more than two result facets round an edge, or more than one fan of result
   facets round a vertex, is `ZeroThickness` at that cell. Non-manifold results stay refused.
7. **unify** (`unify`): result facets of one carrier and sense, joined across an edge where they
   are the only two, are one face. It reads the assembled facets by reference and writes a fresh
   `Body`: faces in the order of their first facet, which is the input faces' arena order; edges
   as straight runs between one pair of faces; vertices where three or more faces meet.

The specification in prose, with the Rust interface: research/kernel-audit/decide-once-spec.md.

There is no retry: the outcome type has none, and DecideOnce/Coverage.lean proves the two refusals
that used to start the retry ladder (`SplitCrossed`, `InconsistentClassification`) unreachable for
every consistent ledger.
-/

namespace ForgeExact.DecideOnce

/-- A side of a carrier. -/
inductive Sgn
  | neg
  | zero
  | pos
  deriving DecidableEq, Repr, Inhabited

/-- A cell of the arrangement of the carrier classes, by its side of each. -/
abbrev Cov := List Sgn

namespace Cov

/-- `X` is a face of `Y`: of one length, and agreeing wherever `X` is off the carrier. -/
def le (X Y : Cov) : Bool :=
  X.length == Y.length && (X.zip Y).all fun p => p.1 == .zero || p.1 == p.2

/-- The carrier a facet lies in: its (first) zero. -/
def zeroIdx (X : Cov) : Nat := X.idxOf .zero

/-- A facet: exactly one zero. -/
def isFacet (X : Cov) : Bool := X.count .zero == 1

/-- The cell just off a facet on the positive side of its carrier. -/
def plus (X : Cov) : Cov := X.set X.zeroIdx .pos
/-- And on the negative side. -/
def minus (X : Cov) : Cov := X.set X.zeroIdx .neg

end Cov

/-- The ledger, as the core reads it. Faces are named by their arena index. Every field is a
table of decisions made once (DecideOnce/Ledger.lean builds it); the core only looks them up. -/
structure Ledger where
  /-- the number of bodies (operands); bodies are `0 .. nb-1` -/
  nb : Nat
  /-- per input face, its body -/
  faceBody : List Nat
  /-- per input face, its carrier class (two faces one surface) -/
  faceCls : List Nat
  /-- decided: facet `φ` lies inside face `f` (point inside face, in the representative) -/
  pieces : List (Cov × Nat)
  /-- decided: side cell `T` is inside body `i` (one ray per flood class) -/
  inside : List (Cov × Nat)
  /-- decided: the cells of dimension 1 round the pieces -/
  edges : List Cov
  /-- decided: the cells of dimension 0 round the pieces -/
  verts : List Cov

/-- An op, as a pointwise rule on the membership vector (union, intersection, difference, or a
whole history). -/
abbrev Rule := (Nat → Bool) → Bool

/-- The keep of each side cell, decided once. -/
abbrev KeepTab := List (Cov × Bool)

def keepIn (kt : KeepTab) (T : Cov) : Bool := ((kt.find? fun e => e.1 == T).map Prod.snd).getD false

namespace Ledger

variable (L : Ledger)

def bodyOf (f : Nat) : Nat := L.faceBody.getD f 0
def clsOf (f : Nat) : Nat := L.faceCls.getD f 0

/-! ### 1–2. Split and imprint -/

/-- Facet `φ` is a piece of a face of body `i`. -/
def isPieceOf (i : Nat) (φ : Cov) : Bool := L.pieces.any fun p => p.1 == φ && L.bodyOf p.2 == i

/-- Every piece, once (a coplanar overlap of two bodies is one facet). -/
def facets : List Cov := dedup (L.pieces.map Prod.fst)

/-- The split is sound: each piece a facet of its face's carrier, in a face of a body that
exists, and no facet inside two faces of one body. -/
def splitOk : Bool :=
  L.pieces.all fun p => p.1.isFacet && p.1.zeroIdx == L.clsOf p.2 && decide (L.bodyOf p.2 < L.nb) &&
    L.pieces.all fun q => !(q.1 == p.1 && L.bodyOf q.2 == L.bodyOf p.2) || q.2 == p.2

/-! ### 3. Classify: flood plus witness -/

/-- The side cells of the pieces. -/
def topes : List Cov := dedup (L.facets.flatMap fun φ => [φ.plus, φ.minus])

/-- The side-cell pairs across the facets that are not pieces of body `i`: where being in `i`
cannot change. -/
def crossings (i : Nat) : List (Cov × Cov) :=
  (L.facets.filter fun φ => !L.isPieceOf i φ).map fun φ => (φ.plus, φ.minus)

def adjOf (cr : List (Cov × Cov)) (T U : Cov) : Bool :=
  cr.any fun p => (p.1 == T && p.2 == U) || (p.1 == U && p.2 == T)

/-- Two side cells on either side of a facet that is not a piece of body `i`. -/
def adj (i : Nat) : Cov → Cov → Bool := adjOf (L.crossings i)

/-- Membership of a side cell in body `i`: the witness of its flood class's first member. -/
def memb (i : Nat) (T : Cov) : Bool :=
  let cr := L.crossings i
  match head (adjOf cr) L.topes T with
  | some R => L.inside.contains (R, i)
  | none => false

/-- Every piece of a body has one side in it and one out. -/
def classifyOk : Bool :=
  L.facets.all fun φ => (List.range L.nb).all fun i =>
    !L.isPieceOf i φ || (L.memb i φ.plus != L.memb i φ.minus)

/-! ### 4–5. Keep and assemble -/

def keep (rule : Rule) (T : Cov) : Bool := rule fun i => decide (i < L.nb) && L.memb i T

/-- **Classified once**: each side cell's keep, in a table every later stage looks up. -/
def keepTab (rule : Rule) : KeepTab := L.topes.map fun T => (T, L.keep rule T)

/-- The result's facets: exactly one side kept. -/
def result (kt : KeepTab) : List Cov :=
  L.facets.filter fun φ => keepIn kt φ.plus != keepIn kt φ.minus

/-- Outward: toward the dropped side. -/
def orient (kt : KeepTab) (φ : Cov) : Sgn := if keepIn kt φ.minus then .pos else .neg

/-! ### 6. The touch rule -/

/-- The result facets round a cell. -/
def around (kt : KeepTab) (x : Cov) : List Cov := (L.result kt).filter fun φ => Cov.le x φ

/-- More than two result facets round an edge. -/
def badEdge (kt : KeepTab) (e : Cov) : Bool := decide (2 < (L.around kt e).length)

/-- Two result facets round `v` sharing an edge round `v`. -/
def fanRel (v : Cov) (c d : Cov) : Bool :=
  L.edges.any fun e => Cov.le v e && Cov.le e c && Cov.le e d

/-- More than one fan round a vertex: some result facet round it out of the first one's fan. -/
def badVert (kt : KeepTab) (v : Cov) : Bool :=
  match L.around kt v with
  | [] => false
  | c :: _ => (L.around kt v).any fun d => !(cls (L.fanRel v) (L.around kt v) c).contains d

/-! ### 7. Unify: read by reference, write fresh -/

/-- Two result facets of one carrier and sense sharing an edge (past the touch kt, the only two
round it). -/
def joinRel (kt : KeepTab) (c d : Cov) : Bool :=
  c.zeroIdx == d.zeroIdx && Ledger.orient kt c == Ledger.orient kt d &&
    L.edges.any fun e => Cov.le e c && Cov.le e d

/-- The input face a facet came from: the first in arena order. -/
def origin (φ : Cov) : Nat := ((L.pieces.find? fun p => p.1 == φ).map Prod.snd).getD 0

end Ledger

/-- A face of the result, written fresh. -/
structure OutFace where
  facets : List Cov
  plane : Nat
  orient : Sgn
  /-- the arena index of the input face its first facet came from: its name -/
  origin : Nat

/-- An edge of the result: a straight run of edge cells between one pair of faces. -/
structure OutEdge where
  /-- its edge cells, in arena order -/
  cells : List Cov
  /-- the two faces it joins, by their first facets -/
  faces : List Cov

/-- The result body, written fresh. -/
structure Body where
  faces : List OutFace
  edges : List OutEdge
  /-- the vertex cells where three or more faces meet -/
  verts : List Cov

namespace Ledger

variable (L : Ledger)

/-- The first facet of each face, in the order of the result's facets. -/
def heads (kt : KeepTab) : List Cov :=
  (L.result kt).filter fun c => head (L.joinRel kt) (L.result kt) c == some c

def faceOf (kt : KeepTab) (h : Cov) : List Cov :=
  (L.result kt).filter fun d => (cls (L.joinRel kt) (L.result kt) h).contains d

/-- Two lists with the same members. -/
def sameSet (a b : List Cov) : Bool := a.all (b.contains ·) && b.all (a.contains ·)

/-- The faces round a cell, each named by its first facet (`key` names a result facet's face). -/
def facesAt (kt : KeepTab) (key : Cov → Cov) (x : Cov) : List Cov := dedup ((L.around kt x).map key)

/-- **Unify.** Reads the assembled facets (by reference) and writes a fresh body: faces are the
classes of `joinRel`, listed by first facet; a vertex is a vertex cell where three or more faces
meet; an edge is a class of edge cells between one pair of faces, joined through vertex cells that
are not vertices (so a straight run between two faces is one edge, with no redundant vertex). -/
def unify (kt : KeepTab) : Body :=
  let hs := L.heads kt
  let fk : List (Cov × Cov) := hs.flatMap fun h => (L.faceOf kt h).map fun c => (c, h)
  let key := fun c => ((fk.find? fun e => e.1 == c).map Prod.snd).getD c
  let fa : List (Cov × List Cov) := (L.edges ++ L.verts).map fun x => (x, L.facesAt kt key x)
  let at_ := fun x => ((fa.find? fun e => e.1 == x).map Prod.snd).getD []
  let outV := L.verts.filter fun v => decide (3 ≤ (at_ v).length)
  let ecs := L.edges.filter fun e => (at_ e).length == 2
  let erel := fun e₁ e₂ => sameSet (at_ e₁) (at_ e₂) &&
    L.verts.any fun v => Cov.le v e₁ && Cov.le v e₂ && !outV.contains v
  { faces := hs.map fun h =>
      { facets := L.faceOf kt h, plane := h.zeroIdx, orient := Ledger.orient kt h, origin := L.origin h }
    edges := (ecs.filter fun e => head erel ecs e == some e).map fun e =>
      { cells := ecs.filter fun d => (cls erel ecs e).contains d, faces := at_ e }
    verts := outV }

end Ledger

/-- Why the core refuses. -/
inductive Refusal
  | splitCrossed
  | inconsistentClassification
  /-- a non-manifold edge or vertex of the result, at that cell -/
  | zeroThickness (cell : Cov)
  deriving DecidableEq

inductive Outcome
  | ok (b : Body)
  | empty
  | refused (r : Refusal)

/-- **The core.** One pass, no retries. -/
def core (L : Ledger) (rule : Rule) : Outcome :=
  if !L.splitOk then .refused .splitCrossed
  else if !L.classifyOk then .refused .inconsistentClassification
  else
    let kt := L.keepTab rule
    match L.edges.find? (L.badEdge kt) with
    | some e => .refused (.zeroThickness e)
    | none => match L.verts.find? (L.badVert kt) with
      | some v => .refused (.zeroThickness v)
      | none => if L.result kt = [] then .empty else .ok (L.unify kt)

end ForgeExact.DecideOnce
