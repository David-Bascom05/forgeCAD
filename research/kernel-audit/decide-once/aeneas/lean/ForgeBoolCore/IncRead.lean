import ForgeBoolCore.Funs
import ForgeExact.DecideOnce.CoverageI

/-!
# The Rust `inc` ledger, read as an incidence ledger (`toLF`), and LedgerWF v2

face-arrangement.md §1. A Rust `forge_bool_core::inc::Ledger` names facets, rooms, edges and verts
by `u32` ids in separate spaces. Lean tags them (`FCell`), so one `IncLedger FCell` holds all four
kinds. `toLF` is a direct read:
* a facet's carrier is `facet_class[φ]` and its rooms are `facet_rooms[φ]`;
* a Csr rowL is `ids[start[k] .. start[k+1]]`;
* a sector `(facet, a, b)` is the `Sector` with bounds `[a, b]`.

`LedgerWF2` is `inc::ledger_wf`'s checks as a proposition (the Rust's debug check). The `inc`
refinement proofs assume it, and it gives the nodup facts that CoverageI and Simulation need.
-/

open Aeneas Aeneas.Std

namespace forge_bool_core

open ForgeExact.DecideOnce

/-- A cell of the face arrangement, tagged by kind. -/
inductive FCell
  | room (n : Nat)
  | facet (n : Nat)
  | edge (n : Nat)
  | vert (n : Nat)
  deriving DecidableEq, Inhabited

namespace inc

/-- Row `k` of a Csr. -/
def rowL (c : Csr) (k : Nat) : List Nat :=
  let s := (c.start.val.getD k 0#u32).val
  let e := (c.start.val.getD (k + 1) 0#u32).val
  ((c.ids.val.drop s).take (e - s)).map (·.val)

/-- The number of rows of a Csr. -/
def nrowsL (c : Csr) : Nat := c.start.val.length - 1

def secRowL (c : SecCsr) (k : Nat) : List Sector :=
  let s := (c.start.val.getD k 0#u32).val
  let e := (c.start.val.getD (k + 1) 0#u32).val
  (c.sec.val.drop s).take (e - s)

def nfacets (l : Ledger) : Nat := l.facet_class.val.length

def facetNum : FCell → Nat
  | .facet n => n
  | _ => 0

def edgeNum : FCell → Nat
  | .edge n => n
  | _ => 0

def vertNum : FCell → Nat
  | .vert n => n
  | _ => 0

/-- **The Rust ledger as an incidence ledger.** -/
def toLF (l : Ledger) : IncLedger FCell where
  nb := l.nb.val
  faceBody := l.face_body.val.map fun (b : BodyId) => (b : U32).val
  faceCls := l.face_class.val.map (·.val)
  facets := (List.range (nfacets l)).map .facet
  carrier := fun x => (l.facet_class.val.getD (facetNum x) 0#u32).val
  plus := fun x => .room (l.facet_rooms.val.getD (facetNum x) (0#u32, 0#u32)).1.val
  minus := fun x => .room (l.facet_rooms.val.getD (facetNum x) (0#u32, 0#u32)).2.val
  pieces := l.pieces.val.map fun p => (.facet (p.1 : U32).val, (p.2 : U32).val)
  inside := l.inside.val.map fun p => (.room (p.1 : U32).val, (p.2 : U32).val)
  edges := (List.range (nrowsL l.edge_facets)).map .edge
  verts := (List.range (nrowsL l.vert_edges)).map .vert
  edgeFacets := fun x => (rowL l.edge_facets (edgeNum x)).map .facet
  vertEdges := fun x => (rowL l.vert_edges (vertNum x)).map .edge
  vertSectors := fun x => (secRowL l.vert_sectors (vertNum x)).map fun s =>
    ⟨.facet s.facet.val, [.edge s.a.val, .edge s.b.val]⟩

/-- A well-formed Csr: starts at 0, monotone, ends at the ids' length, ids below `n`. -/
structure CsrOK (c : Csr) (n : Nat) : Prop where
  nonempty : c.start.val ≠ []
  zero : (c.start.val.getD 0 0#u32).val = 0
  mono : ∀ k, k + 1 < c.start.val.length →
    (c.start.val.getD k 0#u32).val ≤ (c.start.val.getD (k + 1) 0#u32).val
  last : (c.start.val.getD (c.start.val.length - 1) 0#u32).val = c.ids.val.length
  ids : ∀ x ∈ c.ids.val, x.val < n

/-- **LedgerWF v2**: `inc::ledger_wf` as a proposition. -/
structure LedgerWF2 (l : Ledger) : Prop where
  edge_facets : CsrOK l.edge_facets (nfacets l)
  vert_edges : CsrOK l.vert_edges (nrowsL l.edge_facets)
  sectors_rows : l.vert_sectors.start.val.length = l.vert_edges.start.val.length
  wings : ∀ e, e < nrowsL l.edge_facets → ∀ φ, (rowL l.edge_facets e).count φ ≤ 2
  vert_edges_nodup : ∀ v, v < nrowsL l.vert_edges → (rowL l.vert_edges v).Nodup
  sectors : ∀ v, v < nrowsL l.vert_edges → ∀ s ∈ secRowL l.vert_sectors v,
    s.facet.val < nfacets l ∧ s.a.val ∈ rowL l.vert_edges v ∧ s.b.val ∈ rowL l.vert_edges v ∧
    s.facet.val ∈ rowL l.edge_facets s.a.val ∧ s.facet.val ∈ rowL l.edge_facets s.b.val
  sectors_nodup : ∀ v, v < nrowsL l.vert_edges → (secRowL l.vert_sectors v).Nodup
  rooms : l.facet_rooms.val.length = nfacets l ∧
    ∀ p ∈ l.facet_rooms.val, p.1.val < l.n_rooms.val ∧ p.2.val < l.n_rooms.val ∧ p.1 ≠ p.2
  pieces : ∀ p ∈ l.pieces.val, (p.1 : U32).val < nfacets l ∧ (p.2 : U32).val < l.face_body.val.length ∧
    (l.face_class.val.getD (p.2 : U32).val 0#u32) = l.facet_class.val.getD (p.1 : U32).val 0#u32
  inside : ∀ p ∈ l.inside.val, (p.1 : U32).val < l.n_rooms.val ∧ (p.2 : U32).val < l.nb.val
  every_piece : ∀ φ, φ < nfacets l → ∃ p ∈ l.pieces.val, (p.1 : U32).val = φ
  face_lists : l.face_class.val.length = l.face_body.val.length
  small : nfacets l < U32.max ∧ l.n_rooms.val < U32.max ∧ l.pieces.val.length < U32.max ∧
    l.face_body.val.length < U32.max ∧ nfacets l * l.nb.val ≤ Usize.max

/-- The facets of `toLF` are listed once each. -/
theorem toLF_facets_nodup (l : Ledger) : (toLF l).facets.Nodup := by
  simp only [toLF]
  exact (List.nodup_range).map fun a b h => by cases h; rfl

end inc

end forge_bool_core
