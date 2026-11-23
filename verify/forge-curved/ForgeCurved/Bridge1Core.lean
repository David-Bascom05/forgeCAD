import ForgeCurved.Bridge1
import ForgeExact.DecideOnce.Simulation

/-!
# Route B, bridge (1), composed with the core

`IncLedger.coreI_zeroThickness_iff` (Simulation.lean, decide-once-lean) reads the A8 core's
ZeroThickness refusal as its touch rule; `touch_iff_xNonManifoldF` (Bridge1.lean) reads the touch
rule as `XNonManifoldF`. **`coreI_refuses_iff_xNonManifoldF`**: for a face ledger over `Nat` ids,
a face geometry whose facets' rooms are the ledger's (`rp = plus`, `rm = minus`), and the rows
read off the ledger (`rowsOf`), the core refuses ZeroThickness iff `XNonManifoldF` with the
core's keep table. With `xNonManifoldF_iff_top` (route B's obligations), iff the result is not a
manifold (`coreI_refuses_iff_top`).
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.FaceLocal

open ForgeExact.DecideOnce ForgeExact.DecideOnce.IncLedger

variable {P : Type} [TopologicalSpace P]

/-- The rows of an A8 face ledger, for route B. -/
def rowsOf (L : IncLedger Nat) (vpt : Nat → P) : Rows (P := P) where
  es := L.edges
  vs := L.verts
  erow := L.edgeFacets
  vsec v := (L.vertSectors v).map (enc Sector.facet Sector.bounds)
  vpt := vpt

/-- **The core's refusal is `XNonManifoldF`.** -/
theorem coreI_refuses_iff_xNonManifoldF (L : IncLedger Nat) (hs : L.splitOk = true)
    (hc : L.classifyOk = true) (rule : Rule) (hnd : ∀ v ∈ L.verts, (L.vertSectors v).Nodup)
    (G : FaceGeo P) (hrp : G.rp = L.plus) (hrm : G.rm = L.minus) (vpt : Nat → P)
    (hrow : ∀ e ∈ L.edges, ∀ φ ∈ L.edgeFacets e, φ ∈ L.facets)
    (hsec : ∀ v ∈ L.verts, ∀ s ∈ L.vertSectors v, s.facet ∈ L.facets) :
    (∃ x, coreI L rule = .refused (.zeroThickness x)) ↔
      XNonManifoldF G (keepInI (L.keepTab rule)) (rowsOf L vpt) := by
  rw [coreI_zeroThickness_iff L hs hc rule hnd]
  have hBd : ∀ φ ∈ L.facets, (Bd G (keepInI (L.keepTab rule)) φ ↔
      (L.result (L.keepTab rule)).contains φ = true) := by
    intro φ hφ
    unfold Bd IncLedger.result
    rw [result_contains_iff, hrp, hrm]
    simp [hφ]
  exact touch_iff_xNonManifoldF (facet := Sector.facet) (bounds := Sector.bounds)
    (rowsOf L vpt) L.vertSectors (fun _ => rfl)
    (fun e he φ hφ => hBd φ (hrow e he φ hφ))
    (fun v hv s hs' => hBd _ (hsec v hv s hs')) hnd

/-- **The core refuses iff the result is not a manifold**, given route B's obligations. -/
theorem coreI_refuses_iff_top (L : IncLedger Nat) (hs : L.splitOk = true)
    (hc : L.classifyOk = true) (rule : Rule) (hnd : ∀ v ∈ L.verts, (L.vertSectors v).Nodup)
    (G : FaceGeo P) (hrp : G.rp = L.plus) (hrm : G.rm = L.minus) (vpt : Nat → P)
    (hrow : ∀ e ∈ L.edges, ∀ φ ∈ L.edgeFacets e, φ ∈ L.facets)
    (hsec : ∀ v ∈ L.verts, ∀ s ∈ L.vertSectors v, s.facet ∈ L.facets)
    (hob : Obligations G (keepInI (L.keepTab rule)) (rowsOf L vpt)) :
    (∃ x, coreI L rule = .refused (.zeroThickness x)) ↔
      TopNonManifoldF G (Res G (keepInI (L.keepTab rule))) (rowsOf L vpt) :=
  (coreI_refuses_iff_xNonManifoldF L hs hc rule hnd G hrp hrm vpt hrow hsec).trans
    (xNonManifoldF_iff_top hob)

end ForgeCurved.FaceLocal
