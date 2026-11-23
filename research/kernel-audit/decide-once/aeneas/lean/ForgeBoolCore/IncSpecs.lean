import ForgeBoolCore.IncRead
import ForgeBoolCore.RefineKeep
import ForgeExact.DecideOnce.Simulation
import ForgeExact.DecideOnce.MetricI

/-!
# A8: what the `inc` core owes, and the end-to-end theorem it gives (`EndToEndF`)

The obligation, which the inc refinement groups prove:
* `IncCoreSpec`: on a well-formed v2 ledger and metric ledger, `inc::core l r m` returns `ok`, and
  its outcome read into Lean (`outLF`) is `coreMI (toLF l)` on `m`'s thin and enclosed rooms.

What it gives:
* `coreMI_nil`: with no thin rooms, `coreMI` is `coreI`.
* `endToEndF_of`: `IncCoreSpec` plus `flat_refines`. For a v2 ledger refining the plane ledger read
  as incidence, and no thin rooms, `inc::core`'s outcome corresponds (`OutCorr`) to the old core's
  on the plane ledger. So every end-to-end fact about the old core (`end_to_end_proof`'s
  conclusions) transfers. The metric case (thin rooms) is the collapse's own end-to-end theorem,
  from MetricI's `collapse_exact_off_thin`; it is stated separately.
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

/-- A metric ledger's rooms, read into Lean. -/
def thinL (m : MetricLedger) : List FCell := m.thin.val.map fun (r : RoomId) => .room (r : U32).val
def enclL (m : MetricLedger) : List FCell := m.enclosed.val.map fun (r : RoomId) => .room (r : U32).val

/-- A metric ledger fits its face ledger: its rooms are rooms. -/
structure MetricWF2 (l : Ledger) (m : MetricLedger) : Prop where
  thin : ∀ r ∈ m.thin.val, (r : U32).val < l.n_rooms.val
  enclosed : ∀ r ∈ m.enclosed.val, (r : U32).val < l.n_rooms.val

/-- **The inc core refines `coreMI`**, on a well-formed v2 ledger and metric ledger (the inc groups'
target). -/
def IncCoreSpec : Prop :=
  ∀ (l : Ledger) (r : Rule) (m : MetricLedger), LedgerWF2 l → MetricWF2 l m →
    core l r m ⦃ o => outLF o = (toLF l).coreMI (ruleL l.nb.val r) (thinL m) (enclL m) ⦄

/-- **With no thin rooms, `coreMI` is `coreI`.** -/
theorem coreMI_nil {α : Type} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α)
    (rule : ForgeExact.DecideOnce.Rule) (enc : List α) : L.coreMI rule [] enc = coreI L rule := by
  unfold IncLedger.coreMI coreI
  split
  · rfl
  · split
    · rfl
    · have ht : L.thinTopes [] = [] := by simp [IncLedger.thinTopes]
      have hc : L.collapse (L.keepTab rule) [] enc =
          .ok (L.rooms.map fun T => (T, keepInI (L.keepTab rule) T)) := by
        unfold IncLedger.collapse; rw [ht]; simp
      rw [hc]
      simp only
      congr 1
      unfold IncLedger.keepTab
      apply List.map_congr_left
      intro T hT
      rw [IncLedger.keepIn_map _ hT]

/-- **EndToEndF** (flat). For a v2 ledger that refines the plane ledger `L` read as incidence, with
`L`'s split sound, `inc::core`'s outcome corresponds to the old core's outcome on `L`. -/
def EndToEndF : Prop :=
  ∀ (L : ForgeExact.DecideOnce.Ledger) (l : Ledger) (r : Rule) (σ : Cov → FCell) (τ ν : Cov → Option FCell)
    (κ : Cov → ForgeExact.DecideOnce.Sector Cov → ForgeExact.DecideOnce.Sector FCell),
    ∀ (m : MetricLedger), m.thin.val = [] → MetricWF2 l m →
    L.splitOk = true → LedgerWF2 l → Refines L.inc (toLF l) σ τ ν κ →
    core l r m ⦃ o => OutCorr σ τ ν (outLF o) (ForgeExact.DecideOnce.core L (ruleL l.nb.val r)).toI ⦄

theorem endToEndF_of (h : IncCoreSpec) : EndToEndF := by
  intro L l r σ τ ν κ m hm hmw hs hw hR
  have := h l r m hw hmw
  apply WP.spec_mono this
  intro o ho
  rw [ho, show thinL m = [] by simp [thinL, hm], coreMI_nil]
  exact flat_refines L hs (toLF l) σ τ ν κ hR (ruleL l.nb.val r)

end inc

end forge_bool_core
