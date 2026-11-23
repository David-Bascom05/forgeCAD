import ForgeExact.DecideOnce.CoreInc

/-!
# A8 L2: a coarse incidence ledger refines a fine one (`Refines`), and what the cores then share

face-arrangement.md §2.2 and §2.2a. `A` is the fine ledger and `B` the coarse one. For flat inputs
`A` is `inc (sampleLedger …)`, the plane ledger read as incidence (CoreInc.lean). For curved
inputs it is the carrier ledger. `B` is the face arrangement.

The maps are:
* `σ`, fine facets to coarse facets;
* `τ`, fine edges to coarse edges, or `none` for an edge inside a coarse facet or room;
* `ν`, fine verts to coarse verts, or `none`;
* `κ v`, fine sectors at `v` to coarse sectors at `ν v`.

Rooms are not mapped. They are reached through the facet-sides, so several rooms may lie in one
component.

Everything here is combinatorial, with no geometry. So the simulation theorem (`core_refines`,
Simulation.lean) holds for any fine reference with the old core's facts. Geometry enters only when
a builder proves `Refines`: there, `room_const` (the wedge lemma) gives `rooms`.
-/

namespace ForgeExact.DecideOnce

universe u v

variable {α : Type u} {β : Type v} [DecidableEq α] [BEq α] [LawfulBEq α] [DecidableEq β] [BEq β] [LawfulBEq β]

/-- The coarse ledger `B` refines the fine ledger `A` through `σ τ ν κ`. -/
structure Refines (A : IncLedger α) (B : IncLedger β) (σ : α → β) (τ ν : α → Option β)
    (κ : α → Sector α → Sector β) : Prop where
  /-- one set of bodies and input faces -/
  nb : B.nb = A.nb
  faceBody : B.faceBody = A.faceBody
  faceCls : B.faceCls = A.faceCls
  nodupA : A.facets.Nodup
  nodupB : B.facets.Nodup
  /-- (s) σ maps the fine facets onto the coarse ones, keeping the carrier -/
  facet_mem : ∀ φ ∈ A.facets, σ φ ∈ B.facets
  facet_onto : ∀ ψ ∈ B.facets, ∃ φ ∈ A.facets, σ φ = ψ
  carrier : ∀ φ ∈ A.facets, B.carrier (σ φ) = A.carrier φ
  /-- (s) pieces correspond facet by facet, and every coarse piece comes from a fine one -/
  pieces : ∀ φ ∈ A.facets, ∀ f, (φ, f) ∈ A.pieces ↔ (σ φ, f) ∈ B.pieces
  pieces_from : ∀ p ∈ B.pieces, ∃ φ ∈ A.facets, σ φ = p.1
  pieces_facets : ∀ p ∈ A.pieces, p.1 ∈ A.facets
  /-- a coarse facet's two rooms differ (LedgerWF) -/
  sidesB : ∀ ψ ∈ B.facets, B.plus ψ ≠ B.minus ψ
  /-- (r) rooms, per facet-side: the coarse membership is the fine one -/
  memb_plus : ∀ φ ∈ A.facets, ∀ i, i < A.nb → B.memb i (B.plus (σ φ)) = A.memb i (A.plus φ)
  memb_minus : ∀ φ ∈ A.facets, ∀ i, i < A.nb → B.memb i (B.minus (σ φ)) = A.memb i (A.minus φ)
  /-- (e) edges: a bijection onto the coarse wings, or constant inside a facet or room -/
  edgesA_nodup : A.edges.Nodup
  edgesB_nodup : B.edges.Nodup
  edge_mem : ∀ e ∈ A.edges, ∀ E, τ e = some E → E ∈ B.edges
  edge_onto : ∀ E ∈ B.edges, ∃ e ∈ A.edges, τ e = some E
  edge_facets_A : ∀ e ∈ A.edges, ∀ φ ∈ A.edgeFacets e, φ ∈ A.facets
  edge_wings : ∀ e ∈ A.edges, ∀ E, τ e = some E → ((A.edgeFacets e).map σ).Perm (B.edgeFacets E)
  edge_inner : ∀ e ∈ A.edges, τ e = none →
    (A.edgeFacets e).length ≤ 2 ∧ ∀ φ ∈ A.edgeFacets e, ∀ φ' ∈ A.edgeFacets e, σ φ = σ φ'
  /-- (s') a coarse facet's fine facets are joined through edges inside it -/
  facet_fibre : ∀ φ ∈ A.facets, ∀ φ' ∈ A.facets, σ φ = σ φ' →
    Conn (fun c d => A.edges.any fun e => τ e == none && (A.edgeFacets e).contains c &&
      (A.edgeFacets e).contains d) A.facets φ φ'
  /-- (v) verts: sectors map onto the coarse sectors, fibres joined; edges a bijection -/
  vert_mem : ∀ w ∈ A.verts, ∀ V, ν w = some V → V ∈ B.verts
  /-- a coarse vert is one point: one fine vert -/
  vert_inj : ∀ w ∈ A.verts, ∀ w' ∈ A.verts, ∀ V, ν w = some V → ν w' = some V → w = w'
  vert_onto : ∀ V ∈ B.verts, ∃ w ∈ A.verts, ν w = some V
  sector_mem : ∀ w ∈ A.verts, ∀ V, ν w = some V → ∀ s ∈ A.vertSectors w, κ w s ∈ B.vertSectors V
  sector_onto : ∀ w ∈ A.verts, ∀ V, ν w = some V → ∀ S ∈ B.vertSectors V, ∃ s ∈ A.vertSectors w, κ w s = S
  sector_facet : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, (κ w s).facet = σ s.facet
  sector_fibre : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, ∀ s' ∈ A.vertSectors w, κ w s = κ w s' →
    Conn (fun a b => a.bounds.any fun e => b.bounds.contains e && τ e == none) (A.vertSectors w) s s'
  /-- fine sectors across an edge inside a coarse facet lie in one coarse sector -/
  sector_inner : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, ∀ s' ∈ A.vertSectors w, ∀ e ∈ s.bounds, e ∈ s'.bounds →
    τ e = none → κ w s = κ w s'
  sector_bounds_sub : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, ∀ e ∈ s.bounds, e ∈ A.vertEdges w
  sector_bounds : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, ∀ e ∈ s.bounds, ∀ E, τ e = some E →
    E ∈ (κ w s).bounds
  sector_bounds_from : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, ∀ E ∈ (κ w s).bounds,
    ∃ s' ∈ A.vertSectors w, κ w s' = κ w s ∧ ∃ e ∈ s'.bounds, τ e = some E
  vert_edges : ∀ w ∈ A.verts, ∀ V, ν w = some V →
    (((A.vertEdges w).filterMap τ)).Perm (B.vertEdges V)
  sector_facets_A : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, s.facet ∈ A.facets
  sectorsA_nodup : ∀ w ∈ A.verts, (A.vertSectors w).Nodup
  sectorsB_nodup : ∀ V ∈ B.verts, (B.vertSectors V).Nodup
  vertEdgesB_nodup : ∀ V ∈ B.verts, (B.vertEdges V).Nodup
  /-- at a fine vert inside a coarse facet or edge: sectors of one coarse facet are joined across
  edges inside it, and any two coarse facets' sectors there meet at a common edge -/
  inner_same : ∀ w ∈ A.verts, ν w = none → ∀ s ∈ A.vertSectors w, ∀ t ∈ A.vertSectors w,
    σ s.facet = σ t.facet →
    Conn (fun a b => a.bounds.any fun e => b.bounds.contains e && τ e == none) (A.vertSectors w) s t
  inner_meet : ∀ w ∈ A.verts, ν w = none → ∀ s ∈ A.vertSectors w, ∀ t ∈ A.vertSectors w,
    ∃ s' ∈ A.vertSectors w, ∃ t' ∈ A.vertSectors w, σ s'.facet = σ s.facet ∧ σ t'.facet = σ t.facet ∧
      ∃ e ∈ s'.bounds, e ∈ t'.bounds
  /-- an edge inside a coarse facet has all its fine facets in that facet -/
  inner_edge_facet : ∀ w ∈ A.verts, ∀ s ∈ A.vertSectors w, ∀ t ∈ A.vertSectors w, ∀ e ∈ s.bounds,
    e ∈ t.bounds → τ e = none → σ s.facet = σ t.facet
  vert_edges_A : ∀ w ∈ A.verts, ∀ e ∈ A.vertEdges w, e ∈ A.edges
  /-- at a fine vert inside a coarse edge, the fine edges along it all map to that edge -/
  inner_edges : ∀ w ∈ A.verts, ν w = none → ∀ e ∈ A.vertEdges w, ∀ e' ∈ A.vertEdges w, ∀ E E',
    τ e = some E → τ e' = some E' → E = E'
  vert_inner : ∀ w ∈ A.verts, ν w = none →
    (∃ ψ, ∀ s ∈ A.vertSectors w, σ s.facet = ψ) ∨
    (∃ E ∈ B.edges, ∃ e ∈ A.edges, τ e = some E ∧ ∀ s ∈ A.vertSectors w, σ s.facet ∈ B.edgeFacets E)
  /-- (e') a coarse edge's fine edges are joined through verts inside it -/
  edge_fibre : ∀ e ∈ A.edges, ∀ e' ∈ A.edges, ∀ E, τ e = some E → τ e' = some E →
    Conn (fun a b => A.verts.any fun w => ν w == none && (A.vertEdges w).contains a &&
      (A.vertEdges w).contains b) (A.edges.filter fun x => τ x == some E) e e'

/-- A coarse face corresponds to a fine one: one carrier and sense, and its facets are the images. -/
def FaceCorr (σ : α → β) (F : OutFaceI β) (F₀ : OutFaceI α) : Prop :=
  F.plane = F₀.plane ∧ F.orient = F₀.orient ∧ ∀ ψ, ψ ∈ F.facets ↔ ∃ φ ∈ F₀.facets, σ φ = ψ

/-- Facets `ψ` (coarse) and `φ` (fine) lie in corresponding faces. An output edge names its faces
by their first facets, and the first facet of a coarse face need not be the image of the fine
face's first facet, so faces are matched through their facets. -/
def SameFace (σ : α → β) (b : BodyI β) (b₀ : BodyI α) (ψ : β) (φ : α) : Prop :=
  ∃ F ∈ b.faces, ∃ F₀ ∈ b₀.faces, ψ ∈ F.facets ∧ φ ∈ F₀.facets ∧ FaceCorr σ F F₀

def EdgeCorr (σ : α → β) (τ : α → Option β) (b : BodyI β) (b₀ : BodyI α) (E : OutEdgeI β) (E₀ : OutEdgeI α) : Prop :=
  (∀ X, X ∈ E.cells ↔ ∃ x ∈ E₀.cells, τ x = some X) ∧
  (∀ ψ ∈ E.faces, ∃ φ ∈ E₀.faces, SameFace σ b b₀ ψ φ) ∧ (∀ φ ∈ E₀.faces, ∃ ψ ∈ E.faces, SameFace σ b b₀ ψ φ)

/-- Two bodies correspond: faces and edges matched both ways, verts the ν-images. -/
def BodyCorr (σ : α → β) (τ ν : α → Option β) (b : BodyI β) (b₀ : BodyI α) : Prop :=
  (∀ F ∈ b.faces, ∃ F₀ ∈ b₀.faces, FaceCorr σ F F₀) ∧ (∀ F₀ ∈ b₀.faces, ∃ F ∈ b.faces, FaceCorr σ F F₀) ∧
  (∀ E ∈ b.edges, ∃ E₀ ∈ b₀.edges, EdgeCorr σ τ b b₀ E E₀) ∧
  (∀ E₀ ∈ b₀.edges, ∃ E ∈ b.edges, EdgeCorr σ τ b b₀ E E₀) ∧
  ∀ V, V ∈ b.verts ↔ ∃ w ∈ b₀.verts, ν w = some V

/-- Two outcomes correspond. A ZeroThickness names a cell on each side; the coarse one is the
image of some fine bad cell. -/
def OutCorr (σ : α → β) (τ ν : α → Option β) : OutcomeI β → OutcomeI α → Prop
  | .ok b, .ok b₀ => BodyCorr σ τ ν b b₀
  | .empty, .empty => True
  | .refused .splitCrossed, .refused .splitCrossed => True
  | .refused .inconsistentClassification, .refused .inconsistentClassification => True
  | .refused (.zeroThickness _), .refused (.zeroThickness _) => True
  | _, _ => False

/-- **The simulation theorem, stated** (face-arrangement.md §2.2). Its proof is Simulation.lean. For
a fine ledger with a sound split, and a coarse ledger refining it, the two cores' outcomes
correspond. -/
def CoreRefines : Prop :=
  ∀ {α : Type u} {β : Type v} [DecidableEq α] [BEq α] [LawfulBEq α] [DecidableEq β] [BEq β] [LawfulBEq β]
    (A : IncLedger α) (B : IncLedger β) (σ : α → β) (τ ν : α → Option β) (κ : α → Sector α → Sector β),
    A.splitOk = true → Refines A B σ τ ν κ → ∀ rule, OutCorr σ τ ν (coreI B rule) (coreI A rule)

end ForgeExact.DecideOnce
