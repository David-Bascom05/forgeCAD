import ForgeExact.DecideOnce.Core

/-!
# A8: the decide-once core over explicit incidence (`coreI`)

research/decide-once/face-arrangement.md §2.1. This is Core.lean's algorithm with the covector
arithmetic replaced by tables. Cells are of any type `α` with decidable equality:
* the plane ledger's covectors, giving `inc L` (CoreInc.lean: `core L` corresponds to
  `coreI (inc L)`);
* the face arrangement's ids, giving `toLF` of a Rust v2 ledger.

The three covector primitives become fields:

| Core.lean | here |
|---|---|
| `φ.zeroIdx` | `carrier φ` |
| `φ.plus`, `φ.minus` | `plus φ`, `minus φ` (the rooms either side) |
| `Cov.le e φ` (edge or vert below a facet) | `φ ∈ edgeFacets e`, the sectors at `v` |
| `Cov.le v e` | `e ∈ vertEdges v` |

Wings and sectors (W2, the design's §1):
* `edgeFacets e` lists a facet once per wing;
* `vertSectors v` lists the sectors at `v`, each a facet with the edges bounding it at `v`.

The touch rule counts wings at an edge and joins sectors at a vert. In the covector instance every
multiplicity is one, and there is one sector per facet round `v`.

Naming is not part of the core here: `OutFaceI` has no `origin`. A face is named by the write, by
the old path's rule (least of its pieces' wholes), stated separately.
-/

namespace ForgeExact.DecideOnce

universe u

/-- A sector at a vert: a facet, and the edges bounding it there. -/
structure Sector (α : Type u) where
  facet : α
  bounds : List α
  deriving DecidableEq

/-- The ledger as the A8 core reads it. -/
structure IncLedger (α : Type u) where
  nb : Nat
  faceBody : List Nat
  faceCls : List Nat
  /-- the facets (each a piece of at least one face) -/
  facets : List α
  /-- a facet's carrier class -/
  carrier : α → Nat
  /-- the rooms just off a facet, on its class's `+` and `−` sides -/
  plus : α → α
  minus : α → α
  /-- decided: facet `φ` lies in face `f` -/
  pieces : List (α × Nat)
  /-- decided: room `R` is inside body `i` (witnesses) -/
  inside : List (α × Nat)
  edges : List α
  verts : List α
  /-- the facets round an edge, once per wing -/
  edgeFacets : α → List α
  /-- the edges round a vert -/
  vertEdges : α → List α
  /-- the sectors round a vert -/
  vertSectors : α → List (Sector α)

/-- The keep of each room, decided once. -/
abbrev KeepTabI (α : Type u) := List (α × Bool)

def keepInI {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (kt : KeepTabI α) (T : α) : Bool :=
  ((kt.find? fun e => e.1 == T).map Prod.snd).getD false

/-- A face of the result: its facets, carrier and sense. -/
structure OutFaceI (α : Type u) where
  facets : List α
  plane : Nat
  orient : Sgn

/-- An edge of the result: a run of edge cells between one pair of faces (by their first facets). -/
structure OutEdgeI (α : Type u) where
  cells : List α
  faces : List α

structure BodyI (α : Type u) where
  faces : List (OutFaceI α)
  edges : List (OutEdgeI α)
  verts : List α

inductive RefusalI (α : Type u)
  | splitCrossed
  | inconsistentClassification
  | zeroThickness (cell : α)

inductive OutcomeI (α : Type u)
  | ok (b : BodyI α)
  | empty
  | refused (r : RefusalI α)

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α)

def bodyOf (f : Nat) : Nat := L.faceBody.getD f 0
def clsOf (f : Nat) : Nat := L.faceCls.getD f 0

/-! ### 1–2. Split and imprint -/

def isPieceOf (i : Nat) (φ : α) : Bool := L.pieces.any fun p => p.1 == φ && L.bodyOf p.2 == i

/-- The split is sound: each piece one of the listed facets, of its face's carrier, in a face of a
body that exists; no facet inside two faces of one body; a facet's two rooms distinct. -/
def splitOk : Bool :=
  L.pieces.all fun p => L.facets.contains p.1 && L.carrier p.1 == L.clsOf p.2 && decide (L.bodyOf p.2 < L.nb) &&
    L.plus p.1 != L.minus p.1 &&
    L.pieces.all fun q => !(q.1 == p.1 && L.bodyOf q.2 == L.bodyOf p.2) || q.2 == p.2

/-! ### 3. Classify -/

/-- The rooms, once each. -/
def rooms : List α := dedup (L.facets.flatMap fun φ => [L.plus φ, L.minus φ])

def crossings (i : Nat) : List (α × α) :=
  (L.facets.filter fun φ => !L.isPieceOf i φ).map fun φ => (L.plus φ, L.minus φ)

def adjOf (cr : List (α × α)) (T U : α) : Bool :=
  cr.any fun p => (p.1 == T && p.2 == U) || (p.1 == U && p.2 == T)

def adj (i : Nat) : α → α → Bool := adjOf (L.crossings i)

/-- Membership of a room in body `i`: the witness of its flood class's first member. -/
def memb (i : Nat) (T : α) : Bool :=
  match head (adjOf (L.crossings i)) L.rooms T with
  | some R => L.inside.contains (R, i)
  | none => false

def classifyOk : Bool :=
  L.facets.all fun φ => (List.range L.nb).all fun i =>
    !L.isPieceOf i φ || (L.memb i (L.plus φ) != L.memb i (L.minus φ))

/-! ### 4–5. Keep and assemble -/

def keep (rule : Rule) (T : α) : Bool := rule fun i => decide (i < L.nb) && L.memb i T

def keepTab (rule : Rule) : KeepTabI α := L.rooms.map fun T => (T, L.keep rule T)

def result (kt : KeepTabI α) : List α :=
  L.facets.filter fun φ => keepInI kt (L.plus φ) != keepInI kt (L.minus φ)

def orient (kt : KeepTabI α) (φ : α) : Sgn := if keepInI kt (L.minus φ) then .pos else .neg

/-! ### 6. The touch rule -/

/-- The result wings round an edge. -/
def aroundE (kt : KeepTabI α) (e : α) : List α := (L.edgeFacets e).filter fun φ => (L.result kt).contains φ

/-- The result sectors round a vert. -/
def aroundV (kt : KeepTabI α) (v : α) : List (Sector α) :=
  (L.vertSectors v).filter fun s => (L.result kt).contains s.facet

def badEdge (kt : KeepTabI α) (e : α) : Bool := decide (2 < (L.aroundE kt e).length)

/-- Two sectors sharing a bounding edge. -/
def fanRel (s t : Sector α) : Bool := s.bounds.any fun e => t.bounds.contains e

def badVert (kt : KeepTabI α) (v : α) : Bool :=
  match L.aroundV kt v with
  | [] => false
  | c :: _ => (L.aroundV kt v).any fun d => !(cls fanRel (L.aroundV kt v) c).contains d

/-! ### 7. Unify -/

/-- Two result facets of one carrier and sense sharing an edge. -/
def joinRel (kt : KeepTabI α) (c d : α) : Bool :=
  L.carrier c == L.carrier d && L.orient kt c == L.orient kt d &&
    L.edges.any fun e => (L.edgeFacets e).contains c && (L.edgeFacets e).contains d

def heads (kt : KeepTabI α) : List α :=
  (L.result kt).filter fun c => head (L.joinRel kt) (L.result kt) c == some c

def faceOf (kt : KeepTabI α) (h : α) : List α :=
  (L.result kt).filter fun d => (cls (L.joinRel kt) (L.result kt) h).contains d

def sameSetI (a b : List α) : Bool := a.all (b.contains ·) && b.all (a.contains ·)

/-- The faces round an edge (by first facet). -/
def facesAtE (kt : KeepTabI α) (key : α → α) (e : α) : List α := dedup ((L.aroundE kt e).map key)

/-- The faces round a vert. -/
def facesAtV (kt : KeepTabI α) (key : α → α) (v : α) : List α :=
  dedup ((L.aroundV kt v).map fun s => key s.facet)

def unify (kt : KeepTabI α) : BodyI α :=
  let hs := L.heads kt
  let fk : List (α × α) := hs.flatMap fun h => (L.faceOf kt h).map fun c => (c, h)
  let key := fun c => ((fk.find? fun e => e.1 == c).map Prod.snd).getD c
  let atE := fun e => L.facesAtE kt key e
  let atV := fun v => L.facesAtV kt key v
  let outV := L.verts.filter fun v => decide (3 ≤ (atV v).length)
  let ecs := L.edges.filter fun e => (atE e).length == 2
  let erel := fun e₁ e₂ => sameSetI (atE e₁) (atE e₂) &&
    L.verts.any fun v => (L.vertEdges v).contains e₁ && (L.vertEdges v).contains e₂ && !outV.contains v
  { faces := hs.map fun h => { facets := L.faceOf kt h, plane := L.carrier h, orient := L.orient kt h }
    edges := (ecs.filter fun e => head erel ecs e == some e).map fun e =>
      { cells := ecs.filter fun d => (cls erel ecs e).contains d, faces := atE e }
    verts := outV }

/-- The touch rule, unify and the empty test, on a given keep table (collapse composes before it). -/
def coreK (kt : KeepTabI α) : OutcomeI α :=
  match L.edges.find? (L.badEdge kt) with
  | some e => .refused (.zeroThickness e)
  | none => match L.verts.find? (L.badVert kt) with
    | some v => .refused (.zeroThickness v)
    | none => if L.result kt = [] then .empty else .ok (L.unify kt)

end IncLedger

/-- **The A8 core.** One pass, no retries. -/
def coreI {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α) (rule : Rule) : OutcomeI α :=
  if !L.splitOk then .refused .splitCrossed
  else if !L.classifyOk then .refused .inconsistentClassification
  else L.coreK (L.keepTab rule)

end ForgeExact.DecideOnce
