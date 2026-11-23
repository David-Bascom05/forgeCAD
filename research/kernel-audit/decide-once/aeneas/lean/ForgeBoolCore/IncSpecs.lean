import ForgeBoolCore.IncRead
import ForgeBoolCore.RefineKeep
import ForgeExact.DecideOnce.Simulation

/-!
# A8: what the `inc` core owes, and the end-to-end theorem it gives (`EndToEndF`)

The obligation, which the inc refinement groups prove:
* `IncCoreSpec`: on a well-formed v2 ledger, `inc::core` returns `ok`, and its outcome read into
  Lean (`outLF`) is `coreI (toLF l)`.

What it gives:
* `endToEndF_of`: `IncCoreSpec` plus `flat_refines`. For a v2 ledger refining the plane ledger read
  as incidence, `inc::core`'s outcome corresponds (`OutCorr`) to the old core's on the plane ledger.
  So every end-to-end fact about the old core (`end_to_end_proof`'s conclusions) transfers.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

open ForgeExact.DecideOnce

namespace inc

def faceL (F : OutFace) : OutFaceI FCell :=
  { facets := F.facets.val.map fun (c : FacetId) => .facet (c : U32).val, plane := F.plane.val, orient := toS F.orient }

def edgeL (E : OutEdge) : OutEdgeI FCell :=
  { cells := E.cells.val.map fun (c : EdgeId) => .edge (c : U32).val,
    faces := E.faces.val.map fun (c : FacetId) => .facet (c : U32).val }

def bodyL (b : Body) : BodyI FCell :=
  { faces := b.faces.val.map faceL, edges := b.edges.val.map edgeL,
    verts := b.verts.val.map fun (v : VertId) => .vert (v : U32).val }

/-- An inc outcome, read into Lean. -/
def outLF : Outcome → OutcomeI FCell
  | .Ok b => .ok (bodyL b)
  | .Empty => .empty
  | .Refused .SplitCrossed => .refused .splitCrossed
  | .Refused .InconsistentClassification => .refused .inconsistentClassification
  | .Refused (.ZeroThicknessEdge e) => .refused (.zeroThickness (.edge (e : U32).val))
  | .Refused (.ZeroThicknessVert v) => .refused (.zeroThickness (.vert (v : U32).val))
  | .Refused (.ZeroThicknessRoom r) => .refused (.zeroThickness (.room (r : U32).val))

/-- **The inc core refines `coreI`**, on a well-formed v2 ledger (the inc groups' target). -/
def IncCoreSpec : Prop :=
  ∀ (l : Ledger) (r : Rule), LedgerWF2 l → core l r ⦃ o => outLF o = coreI (toLF l) (ruleL l.nb.val r) ⦄

/-- **EndToEndF** (flat). For a v2 ledger that refines the plane ledger `L` read as incidence, with
`L`'s split sound, `inc::core`'s outcome corresponds to the old core's outcome on `L`. -/
def EndToEndF : Prop :=
  ∀ (L : ForgeExact.DecideOnce.Ledger) (l : Ledger) (r : Rule) (σ : Cov → FCell) (τ ν : Cov → Option FCell)
    (κ : Cov → ForgeExact.DecideOnce.Sector Cov → ForgeExact.DecideOnce.Sector FCell),
    L.splitOk = true → LedgerWF2 l → Refines L.inc (toLF l) σ τ ν κ →
    core l r ⦃ o => OutCorr σ τ ν (outLF o) (ForgeExact.DecideOnce.core L (ruleL l.nb.val r)).toI ⦄

theorem endToEndF_of (h : IncCoreSpec) : EndToEndF := by
  intro L l r σ τ ν κ hs hw hR
  have := h l r hw
  apply WP.spec_mono this
  intro o ho
  rw [ho]
  exact flat_refines L hs (toLF l) σ τ ν κ hR (ruleL l.nb.val r)

end inc

end forge_bool_core
