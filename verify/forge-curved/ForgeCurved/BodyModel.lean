import ForgeCurved.F6
import ForgeExact.DecideOnce.QForm

/-!
# A Lean model of a `forge_topo::Body`: `⟦b⟧`, `⟦f⟧`, `ValidSolid`, and the bridge to rooms

old-path-statements.md §0 (the coordinator's, 9cd330bb), with the master's ruling of 2026-11-23.
This file is the target of the aeneas session's abstraction function. It maps the translated
`Body` to a `BodyModel`, whose parts are:
* **faces** (`FaceModel`):
  - a continuous **carrier** `c_f`, the surface's implicit function, whose zero set has empty
    interior;
  - the **region** `⟦f⟧`, the surface's points inside the trimming loops;
  - the **sense** `o_f` (from `same_sense` and FaceSense.lean's per-kind rule; with `true`, the
    solid is on `c_f < 0`);
  - the **loops**, as coedges `(edge, reversed)`;
* **edges** (`EdgeModel`): the curve's point set, and its end vertices (`none` for a closed edge);
* **vertices**: points.

**Exact bodies.**
* `Bounds b S`: `S` is closed, regular and bounded; its frontier is on the faces, and every face
  point outside the vertex-cluster zones is on it; each face is oriented at every point where it
  is alone. One fact about the faces is included (`seeded`):
  every connected open set off the faces is unbounded or reaches a lone face point.
* **`bounds_unique`**: `Bounds b S → Bounds b S' → S = S'`.
* `WellFormed b`:
  - distinct faces meet only on edges, so coincident faces are excluded;
  - each edge lies in the faces whose loops use it;
  - each edge's ends are on it.
* `ValidExact b := WellFormed b ∧ ∃ S, Bounds b S`, and `denoteExact b` is that unique `S`.
* **`Φ_u`, the local formula per vertex** (`formulas`): read from the topology, so the same on
  stored and exact carriers. It reads only the vertex's incident carriers (`WellFormed`). Inside
  the vertex's zone, off those carriers, the solid is `Φ_u` on the carriers' signs (`Bounds`).
  `mem_denote_zone` is the exact decision there: evaluate `Φ_u` at `p`'s signs.

**Stored bodies** (the master's ruling, and the coordinator's canonical counterpart).
* `Refines δ b bs`: `b` has `bs`'s cell structure one-to-one (the same faces, loops, edges, ends and
  vertices, by index, and the same senses), and each entity lies within `δ` of its counterpart:
  regions and curves in two-sided distance, and each f64 vertex is mapped to its cluster.
* `exactOf δ b`: the canonical exact counterpart, under the master's ruling (option iv, vertex
  clusters). It keeps the carriers and senses. Each f64 vertex becomes its **cluster**, the meet
  of its incident carriers in its δ-ball (`vertCore`), which must be nonempty, connected and of
  diameter at most δ (`ClusterOK`). Otherwise the result is `none`, which happens only for invalid
  input. Each edge and region is the exact meet near the stored one, cut at the clusters.
  `exactOf_refines` gives `Refines δ b (exactOf δ b)`, one-to-one up to clusters.
* `ValidSolid δ b := ∃ bs, exactOf δ b = some bs ∧` (edges isolated, `EdgeIsolated`) `∧
  ValidExact bs`, and
  `⟦b⟧ := denoteExact (exactOf δ b)` (`denote`, `denote_spec`, `denote_refines`).

**The bridge**, for an exact valid body and any arrangement `G` (F5's `FacetData`) whose face set
contains its faces, with connected rooms:
* `room_in_or_out`: each room is inside the solid or misses it;
* `denote_eq_closure_rooms`: the solid is the closure of the rooms inside it;
* `f6_hyps`, `oriented_denote`: F6's hypotheses hold, so F5/F6 apply unchanged.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.BodyModel

open Set Filter Topology ForgeCurved.FaceLocal ForgeCurved.FacetSides ForgeCurved.Rooms
  ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved
  ForgeCurved.F6

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- A live face, abstracted. -/
structure FaceModel (E : Type) [NormedAddCommGroup E] where
  carrier : E → ℝ
  region : Set E
  sense : Bool
  /-- its loops: each a list of coedges `(edge index, reversed)` -/
  loops : List (List (Nat × Bool))
  cont : Continuous carrier
  nd : interior {x | carrier x = 0} = ∅
  on_carrier : ∀ y ∈ region, carrier y = 0

/-- An edge, abstracted: its point set and end vertices (`none`: a closed edge). -/
structure EdgeModel (E : Type) where
  curve : Set E
  ends : Option (Nat × Nat)

/-- A body, abstracted: faces, edges, vertices, with incidences by index. -/
structure BodyModel (E : Type) [NormedAddCommGroup E] where
  faces : List (FaceModel E)
  edges : List (EdgeModel E)
  verts : List E
  /-- an exact body's vertex clusters: per vertex, the exact meet of its carriers near it (a
  stored body leaves this empty) -/
  clusters : List (Set E) := []
  /-- an exact body's cluster zones: per vertex, the closed δ-ball where micro-faces may lie -/
  zones : List (Set E) := []
  /-- per vertex, the local formula `Φ_u` read from the topology: a Boolean formula over the signs
  of the vertex's incident carriers (a literal `j` reads face `j`'s carrier) -/
  formulas : List SForm := []

/-- Face `f` uses an edge ending at vertex `u`. -/
def IncV (b : BodyModel E) (u : Nat) (f : FaceModel E) : Prop :=
  ∃ l ∈ f.loops, ∃ c ∈ l, ∃ e, b.edges[c.1]? = some e ∧ ∃ w, e.ends = some (u, w) ∨ e.ends = some (w, u)

/-- Face `f` uses edge `i`. -/
def IncE (f : FaceModel E) (i : Nat) : Prop := ∃ l ∈ f.loops, ∃ c ∈ l, c.1 = i

/-- The carriers, by face index: what a local formula's literals read. -/
def carriers (b : BodyModel E) : List (Carrier E ℝ) := b.faces.map (·.carrier)

/-- A formula reads only literals `j` with `P j`. -/
def Reads (P : Nat → Prop) : SForm → Prop
  | .lit j _ => P j
  | .tt => True
  | .ff => True
  | .and a c => Reads P a ∧ Reads P c
  | .or a c => Reads P a ∧ Reads P c
  | .not a => Reads P a

/-- The faces' points. -/
def Faces (b : BodyModel E) : Set E := ⋃ f ∈ b.faces, f.region

/-- The cluster zones' points. -/
def Zones (b : BodyModel E) : Set E := ⋃ z ∈ b.zones, z

/-- The carriers' zeros: closed, with empty interior. -/
def Zeros (b : BodyModel E) : Set E := ⋃ f ∈ b.faces, {x | f.carrier x = 0}

/-- `y` is on face `f` and on no other face of `b`. -/
def Alone (b : BodyModel E) (f : FaceModel E) (y : E) : Prop :=
  y ∈ f.region ∧ ∀ g ∈ b.faces, g ≠ f → y ∉ g.region

/-- **Seeded**: every connected open set off the faces is unbounded, or has a lone face point in
its closure. A fact about the faces only. -/
def Seeded (b : BodyModel E) : Prop :=
  ∀ x, x ∉ Faces b → ∃ K : Set E, IsOpen K ∧ IsPreconnected K ∧ x ∈ K ∧ Disjoint K (Faces b) ∧
    (¬ Bornology.IsBounded K ∨ ∃ f ∈ b.faces, ∃ y, Alone b f y ∧ y ∈ closure K)

/-- **`S` is the solid `b` bounds.** -/
structure Bounds (b : BodyModel E) (S : Set E) : Prop where
  closed : IsClosed S
  regular : closure (interior S) = S
  bounded : Bornology.IsBounded S
  /-- the solid's boundary is on the faces -/
  frontier_sub : frontier S ⊆ Faces b
  /-- every face point outside the cluster zones is on the boundary (inside a zone, micro-faces
  of the near-concurrent carriers need not be) -/
  faces_sub : Faces b \ Zones b ⊆ frontier S
  /-- **`Φ_u`**: inside vertex `u`'s zone, off its incident carriers, the solid is the local formula
  on the carriers' signs -/
  local_formula : ∀ u Φ z, b.formulas[u]? = some Φ → b.zones[u]? = some z → ∀ x ∈ z,
    (∀ f ∈ b.faces, IncV b u f → f.carrier x ≠ 0) → (x ∈ S ↔ Φ.den (carriers b) x)
  oriented : ∀ f ∈ b.faces, ∀ y, Alone b f y → Oriented f.carrier S f.sense y
  seeded : Seeded b

/-- **Well formed**: faces meet only along edges (no coincident faces); each edge lies in the faces
whose loops use it; each edge's ends are on it. -/
structure WellFormed (b : BodyModel E) : Prop where
  meet_on_edges : ∀ i j (hi : i < b.faces.length) (hj : j < b.faces.length), i ≠ j →
    b.faces[i].region ∩ b.faces[j].region ⊆ ⋃ e ∈ b.edges, e.curve
  edge_in_face : ∀ f ∈ b.faces, ∀ l ∈ f.loops, ∀ c ∈ l, ∀ e, b.edges[c.1]? = some e →
    e.curve ⊆ closure f.region
  ends_on : ∀ e ∈ b.edges, ∀ u v, e.ends = some (u, v) → ∀ C D, b.clusters[u]? = some C →
    b.clusters[v]? = some D → (closure e.curve ∩ C).Nonempty ∧ (closure e.curve ∩ D).Nonempty
  /-- a formula per vertex, each reading only that vertex's incident carriers -/
  formulas_len : b.formulas.length = b.verts.length
  formula_reads : ∀ u Φ, b.formulas[u]? = some Φ →
    Reads (fun j => ∃ f, b.faces[j]? = some f ∧ IncV b u f) Φ

/-! ## Uniqueness -/

theorem zeros_closed (b : BodyModel E) : IsClosed (Zeros b) := by
  unfold Zeros
  induction b.faces with
  | nil => simp
  | cons f fs ih =>
    simp only [List.mem_cons, iUnion_iUnion_eq_or_left]
    exact (isClosed_eq f.cont continuous_const).union ih

theorem zeros_interior (b : BodyModel E) : interior (Zeros b) = ∅ := by
  unfold Zeros
  induction b.faces with
  | nil => simp
  | cons f fs ih =>
    simp only [List.mem_cons, iUnion_iUnion_eq_or_left]
    rw [union_comm, interior_union_isClosed_of_interior_empty (by
      clear ih; induction fs with
      | nil => simp
      | cons g gs ih' =>
        simp only [List.mem_cons, iUnion_iUnion_eq_or_left]
        exact (isClosed_eq g.cont continuous_const).union ih') f.nd]
    exact ih

theorem faces_sub_zeros (b : BodyModel E) : Faces b ⊆ Zeros b := by
  intro y hy
  obtain ⟨f, hf, hyf⟩ := mem_iUnion₂.mp hy
  exact mem_iUnion₂.mpr ⟨f, hf, f.on_carrier y hyf⟩

/-- An open set is in the closure of its points off a closed set with empty interior. -/
theorem open_sub_closure_diff {V Z : Set E} (hV : IsOpen V) (hZ : IsClosed Z)
    (hZi : interior Z = ∅) : V ⊆ closure (V \ Z) := by
  intro x hx
  rw [mem_closure_iff]
  intro o ho hxo
  by_contra hne
  rw [not_nonempty_iff_eq_empty] at hne
  have : o ∩ V ⊆ interior Z := interior_maximal (fun z hz => by
    by_contra hzZ; exact (Set.eq_empty_iff_forall_notMem.mp hne) z ⟨hz.1, hz.2, hzZ⟩) (ho.inter hV)
  rw [hZi] at this
  exact this ⟨hxo, hx⟩

/-- **Off the faces, two solids with the same faces agree.** -/
theorem agree_off_faces {b : BodyModel E} {S S' : Set E} (h : Bounds b S) (h' : Bounds b S')
    {x : E} (hx : x ∉ Faces b) : (x ∈ S ↔ x ∈ S') := by
  obtain ⟨K, hKo, hKc, hxK, hKF, hcase⟩ := h.seeded x hx
  have hKS := room_class hKc (hKF.mono_right h.frontier_sub)
  have hKS' := room_class hKc (hKF.mono_right h'.frontier_sub)
  rcases hcase with hunb | ⟨f, hf, y, hy, hyK⟩
  · -- unbounded: outside both
    have out : ∀ {T : Set E}, Bornology.IsBounded T → (K ⊆ T ∨ Disjoint K T) → x ∉ T := by
      intro T hT hc hxT
      rcases hc with hs | hd
      · exact hunb (hT.subset hs)
      · exact Set.disjoint_left.mp hd hxK hxT
    exact ⟨fun hxS => absurd hxS (out h.bounded hKS), fun hxS => absurd hxS (out h'.bounded hKS')⟩
  · -- seeded at a lone face point: one point of `K` off the carrier decides both
    obtain ⟨hin, hout⟩ := h.oriented f hf y hy
    obtain ⟨hin', hout'⟩ := h'.oriented f hf y hy
    obtain ⟨N, hN, hNo, hyN⟩ := mem_nhds_iff.mp ((hin.and hout).and (hin'.and hout'))
    have hKN : (K ∩ N).Nonempty := mem_closure_iff.mp hyK N hNo hyN |> fun ⟨z, hzN, hzK⟩ => ⟨z, hzK, hzN⟩
    obtain ⟨z, ⟨hzK, hzN⟩, hz0⟩ : ∃ z ∈ K ∩ N, f.carrier z ≠ 0 := by
      by_contra hall
      push Not at hall
      have hsub : K ∩ N ⊆ interior {x | f.carrier x = 0} := interior_maximal hall (hKo.inter hNo)
      rw [f.nd] at hsub
      obtain ⟨w, hw⟩ := hKN
      exact hsub hw
    obtain ⟨⟨i1, o1⟩, ⟨i2, o2⟩⟩ := hN hzN
    have hzz : (z ∈ S ↔ z ∈ S') := by
      rcases lt_or_gt_of_ne hz0 with hn | hp
      · cases hs : f.sense
        · have := o1 (by simp [outSide, hs, hn]); have := o2 (by simp [outSide, hs, hn]); tauto
        · have := i1 (by simp [inSide, hs, hn]); have := i2 (by simp [inSide, hs, hn]); tauto
      · cases hs : f.sense
        · have := i1 (by simp [inSide, hs, hp]); have := i2 (by simp [inSide, hs, hp]); tauto
        · have := o1 (by simp [outSide, hs, hp]); have := o2 (by simp [outSide, hs, hp]); tauto
    have key : ∀ {T : Set E}, (K ⊆ T ∨ Disjoint K T) → (x ∈ T ↔ z ∈ T) := by
      intro T hc
      rcases hc with hs | hd
      · exact ⟨fun _ => hs hzK, fun _ => hs hxK⟩
      · exact ⟨fun h1 => absurd h1 (Set.disjoint_left.mp hd hxK),
          fun h1 => absurd h1 (Set.disjoint_left.mp hd hzK)⟩
    rw [key hKS, key hKS', hzz]

/-- **The solid a body bounds is unique.** -/
theorem bounds_unique {b : BodyModel E} {S S' : Set E} (h : Bounds b S) (h' : Bounds b S') :
    S = S' := by
  have half : ∀ {S S' : Set E}, Bounds b S → Bounds b S' → S ⊆ S' := by
    intro S S' h h'
    rw [← h.regular]
    refine closure_minimal (fun x hx => ?_) h'.closed
    have := open_sub_closure_diff isOpen_interior (zeros_closed b) (zeros_interior b) hx
    refine closure_minimal (fun z hz => ?_) h'.closed this
    exact (agree_off_faces h h' fun hzF => hz.2 (faces_sub_zeros b hzF)).mp (interior_subset hz.1)
  exact Set.Subset.antisymm (half h h') (half h' h)

/-! ## Exact and stored bodies -/

/-- **An exact valid solid.** -/
def ValidExact (b : BodyModel E) : Prop := WellFormed b ∧ ∃ S, Bounds b S

open Classical in
/-- **The solid of an exact body** (empty if invalid). -/
noncomputable def denoteExact (b : BodyModel E) : Set E :=
  if h : ∃ S, Bounds b S then Classical.choose h else ∅

theorem bounds_denoteExact {b : BodyModel E} (h : ValidExact b) : Bounds b (denoteExact b) := by
  unfold denoteExact; rw [dif_pos h.2]; exact Classical.choose_spec h.2

/-- The solid is the one `S` with `Bounds b S`. -/
theorem denoteExact_eq {b : BodyModel E} {S : Set E} (hw : WellFormed b) (h : Bounds b S) :
    denoteExact b = S := bounds_unique (bounds_denoteExact ⟨hw, S, h⟩) h

/-- `A` and `B` are within `δ` of each other, both ways. -/
def WithinH (δ : ℝ) (A B : Set E) : Prop :=
  (∀ a ∈ A, ∃ c ∈ B, dist a c ≤ δ) ∧ ∀ c ∈ B, ∃ a ∈ A, dist a c ≤ δ

/-- **`b` refines `bs` within `δ`**: one-to-one cell structure, each entity within `δ`. -/
structure Refines (δ : ℝ) (b bs : BodyModel E) : Prop where
  faces_len : b.faces.length = bs.faces.length
  edges_len : b.edges.length = bs.edges.length
  verts_len : b.verts.length = bs.verts.length
  clusters_len : b.verts.length = bs.clusters.length
  /-- the same local formulas: read from the shared topology -/
  formulas_eq : b.formulas = bs.formulas
  face : ∀ i (h : i < b.faces.length),
    b.faces[i].loops = (bs.faces[i]'(faces_len ▸ h)).loops ∧
    b.faces[i].sense = (bs.faces[i]'(faces_len ▸ h)).sense ∧
    WithinH δ b.faces[i].region (bs.faces[i]'(faces_len ▸ h)).region
  edge : ∀ i (h : i < b.edges.length),
    b.edges[i].ends = (bs.edges[i]'(edges_len ▸ h)).ends ∧
    WithinH δ b.edges[i].curve (bs.edges[i]'(edges_len ▸ h)).curve
  /-- each f64 vertex ↦ its cluster: nonempty, within δ of it -/
  vert : ∀ i (h : i < b.verts.length), (bs.clusters[i]'(clusters_len ▸ h)).Nonempty ∧
    ∀ p ∈ bs.clusters[i]'(clusters_len ▸ h), dist p b.verts[i] ≤ δ

/-! ## The canonical exact counterpart (`exactOf`), with vertex clusters

The master's ruling (option iv): the exact body is the exact carriers plus the topology. Around a
vertex of four or more faces the f64 planes are only nearly concurrent, so the exact vertex is a
**cluster**: the meet of its incident carriers inside the δ-ball round the f64 vertex, with its
micro-edges (and the micro-faces they bound). A cluster must be nonempty, connected and of diameter
at most `δ` (`ClusterOK`). Otherwise `exactOf` is `none`, which happens only for invalid input.
Edge isolation (`EdgeIsolated`) is a validity condition (`ValidSolid`), not a refusal. -/

/-- **The vertex cluster**: the points of the δ-ball round `p₀` on two of `u`'s carriers. -/
def vertCore (δ : ℝ) (b : BodyModel E) (u : Nat) (p₀ : E) : Set E :=
  {p | dist p p₀ ≤ δ ∧ ∃ f ∈ b.faces, ∃ g ∈ b.faces, f ≠ g ∧ IncV b u f ∧ IncV b u g ∧
    f.carrier p = 0 ∧ g.carrier p = 0}

/-- A good cluster: nonempty, connected, diameter at most `δ`. -/
def ClusterOK (δ : ℝ) (C : Set E) : Prop :=
  C.Nonempty ∧ IsPreconnected C ∧ Metric.diam C ≤ δ

/-- The exact edge: its faces' carriers' meet, within δ of the stored curve. -/
def edgeMeet (δ : ℝ) (b : BodyModel E) (i : Nat) (e : EdgeModel E) : Set E :=
  {p | (∀ f ∈ b.faces, IncE f i → f.carrier p = 0) ∧ ∃ q ∈ e.curve, dist p q ≤ δ}

/-- **The edge is isolated in its tube**: the meet near the stored curve is connected (a validity
condition). -/
def EdgeIsolated (δ : ℝ) (b : BodyModel E) (i : Nat) (e : EdgeModel E) : Prop :=
  IsPreconnected (edgeMeet δ b i e)

/-- The exact edges a face's loops use. -/
def faceEdges (δ : ℝ) (b : BodyModel E) (f : FaceModel E) : Set E :=
  ⋃ i, ⋃ (_ : IncE f i), ⋃ e ∈ b.edges[i]?, edgeMeet δ b i e

/-- All the vertex clusters. -/
def allCores (δ : ℝ) (b : BodyModel E) : Set E :=
  ⋃ u, ⋃ p ∈ b.verts[u]?, vertCore δ b u p

/-- The exact region: the closure of the components of the carrier's zeros, off the exact edges
and the clusters, that lie within δ of the stored region. -/
def regionMeet (δ : ℝ) (b : BodyModel E) (f : FaceModel E) : Set E :=
  let Z := {x | f.carrier x = 0} \ (faceEdges δ b f ∪ allCores δ b)
  closure {x | x ∈ Z ∧ connectedComponentIn Z x ⊆ Metric.thickening δ f.region}

theorem regionMeet_on (δ : ℝ) (b : BodyModel E) (f : FaceModel E) :
    ∀ y ∈ regionMeet δ b f, f.carrier y = 0 := by
  intro y hy
  have hc : IsClosed {x | f.carrier x = 0} := isClosed_eq f.cont continuous_const
  unfold regionMeet at hy
  refine closure_minimal ?_ hc hy
  intro x hx
  simp only [Set.mem_setOf_eq, Set.mem_diff] at hx
  exact hx.1.1

/-- The exact face: the same carrier, sense and loops, the exact region. -/
def exactFace (δ : ℝ) (b : BodyModel E) (f : FaceModel E) : FaceModel E where
  carrier := f.carrier
  region := regionMeet δ b f
  sense := f.sense
  loops := f.loops
  cont := f.cont
  nd := f.nd
  on_carrier := regionMeet_on δ b f

open Classical in
/-- **The canonical exact counterpart**: exact faces, edges and vertex clusters (`none` only
when a cluster is not good: invalid input). -/
noncomputable def exactOf (δ : ℝ) (b : BodyModel E) : Option (BodyModel E) :=
  if ∀ u (h : u < b.verts.length), ClusterOK δ (vertCore δ b u b.verts[u]) then
    some { faces := b.faces.map (exactFace δ b)
           edges := b.edges.mapIdx fun i e => { curve := edgeMeet δ b i e, ends := e.ends }
           verts := b.verts
           clusters := b.verts.mapIdx fun u p => vertCore δ b u p
           zones := b.verts.map fun p => Metric.closedBall p δ
           formulas := b.formulas }
  else none

/-- **(i) `exactOf` refines**: when each stored face region and edge curve is within δ of its exact
one, `b` refines `exactOf δ b` within δ, up to vertex clusters. -/
theorem exactOf_refines {δ : ℝ} {b bs : BodyModel E} (h : exactOf δ b = some bs)
    (hface : ∀ f ∈ b.faces, WithinH δ f.region (regionMeet δ b f))
    (hedge : ∀ i (hi : i < b.edges.length), WithinH δ b.edges[i].curve (edgeMeet δ b i b.edges[i])) :
    Refines δ b bs := by
  unfold exactOf at h
  split at h
  · rename_i hall
    cases h
    refine ⟨by simp, by simp, by simp, by simp, rfl, fun i hi => ?_, fun i hi => ?_, fun u hu => ?_⟩
    · simp only [List.getElem_map, exactFace]
      exact ⟨by simp, by simp, hface _ (List.getElem_mem hi)⟩
    · simp only [List.getElem_mapIdx]
      exact ⟨by simp, hedge i hi⟩
    · simp only [List.getElem_mapIdx]
      exact ⟨(hall u hu).1, fun p hp => hp.1⟩
  · cases h

/-- **(ii) A valid stored body**: its canonical exact counterpart exists, its edges are isolated,
and it is a valid solid. -/
def ValidSolid (δ : ℝ) (b : BodyModel E) : Prop :=
  ∃ bs, exactOf δ b = some bs ∧ (∀ i (h : i < b.edges.length), EdgeIsolated δ b i b.edges[i]) ∧
    ValidExact bs

/-- **(iii) `⟦b⟧`**: the canonical exact counterpart's solid (no choice). -/
noncomputable def denote (δ : ℝ) (b : BodyModel E) : Set E :=
  match exactOf δ b with
  | some bs => denoteExact bs
  | none => ∅

theorem denote_spec {δ : ℝ} {b bs : BodyModel E} (h : exactOf δ b = some bs) :
    denote δ b = denoteExact bs := by
  unfold denote; rw [h]

/-- With a valid counterpart and the δ-closeness of regions and edges, `⟦b⟧` is the solid of an
exact body that `b` refines up to clusters. -/
theorem denote_refines {δ : ℝ} {b : BodyModel E} (hv : ValidSolid δ b)
    (hface : ∀ f ∈ b.faces, WithinH δ f.region (regionMeet δ b f))
    (hedge : ∀ i (hi : i < b.edges.length), WithinH δ b.edges[i].curve (edgeMeet δ b i b.edges[i])) :
    ∃ bs, exactOf δ b = some bs ∧ Refines δ b bs ∧ ValidExact bs ∧ denote δ b = denoteExact bs := by
  obtain ⟨bs, hb, -, hvx⟩ := hv
  exact ⟨bs, hb, exactOf_refines hb hface hedge, hvx, denote_spec hb⟩

/-- **A decision inside a vertex zone**: off the incident carriers, membership in `⟦b⟧` is the
local formula `Φ_u` evaluated at the exact carriers' signs (the stored and the exact carriers are
the same functions). -/
theorem mem_denote_zone {δ : ℝ} {b bs : BodyModel E} (h : exactOf δ b = some bs)
    (hv : ValidExact bs) {u : Nat} {Φ : SForm} {z : Set E} (hΦ : bs.formulas[u]? = some Φ)
    (hz : bs.zones[u]? = some z) {x : E} (hx : x ∈ z)
    (hoff : ∀ f ∈ bs.faces, IncV bs u f → f.carrier x ≠ 0) :
    (x ∈ denote δ b ↔ Φ.den (carriers bs) x) := by
  rw [denote_spec h]
  exact (bounds_denoteExact hv).local_formula u Φ z hΦ hz x hx hoff

/-- `⟦f⟧`. -/
def denoteFace (f : FaceModel E) : Set E := f.region

/-! ## The bridge to the rooms (for an exact valid body) -/

variable {G : FaceGeo E} {FacesG : Set E} {cf : Nat → E → ℝ}

/-- **Every room is inside the solid or misses it.** -/
theorem room_in_or_out (hd : FacetData G FacesG cf) (hconn : ∀ r, IsPreconnected (G.room r))
    {b : BodyModel E} (hv : ValidExact b) (hsub : Faces b ⊆ FacesG) (r : Nat) :
    G.room r ⊆ denoteExact b ∨ Disjoint (G.room r) (denoteExact b) := by
  have hfr : frontier (denoteExact b) ⊆ FacesG := by
    exact (bounds_denoteExact hv).frontier_sub.trans hsub
  exact room_class (hconn r) ((hd.room_faces r).mono_right hfr)

/-- **The solid is the closure of the rooms inside it**, when the arrangement's faces have empty
interior. -/
theorem denote_eq_closure_rooms (hd : FacetData G FacesG cf) (hconn : ∀ r, IsPreconnected (G.room r))
    {b : BodyModel E} (hv : ValidExact b) (hsub : Faces b ⊆ FacesG)
    (hnd : interior FacesG = ∅) :
    denoteExact b = closure (⋃ r, ⋃ (_ : G.room r ⊆ denoteExact b), G.room r) := by
  have hB := bounds_denoteExact hv
  apply Set.Subset.antisymm
  · conv_lhs => rw [← hB.regular]
    apply closure_minimal _ isClosed_closure
    intro x hx
    rw [mem_closure_iff]
    intro o ho hxo
    have hU : IsOpen (o ∩ interior (denoteExact b)) := ho.inter isOpen_interior
    have hne : (o ∩ interior (denoteExact b)).Nonempty := ⟨x, hxo, hx⟩
    obtain ⟨z, hz, hzF⟩ : ∃ z ∈ o ∩ interior (denoteExact b), z ∉ FacesG := by
      by_contra hall
      push Not at hall
      have : o ∩ interior (denoteExact b) ⊆ interior FacesG := interior_maximal hall hU
      rw [hnd] at this
      obtain ⟨w, hw⟩ := hne
      exact this hw
    obtain ⟨r, hr⟩ := hd.room_cover z hzF
    have hrin : G.room r ⊆ denoteExact b := by
      rcases room_in_or_out hd hconn hv hsub r with h | h
      · exact h
      · exact absurd (interior_subset hz.2) (Set.disjoint_left.mp h hr)
    exact ⟨z, hz.1, mem_iUnion.mpr ⟨r, mem_iUnion.mpr ⟨hrin, hr⟩⟩⟩
  · exact closure_minimal (iUnion_subset fun r => iUnion_subset fun h => h) hB.closed

/-- **F6's hypotheses hold for the solid.** -/
theorem f6_hyps {b : BodyModel E} (hv : ValidExact b) (hsub : Faces b ⊆ FacesG) :
    frontier (denoteExact b) ⊆ FacesG ∧
      ∀ φ : Set E, Disjoint φ (Faces b) → Disjoint φ (frontier (denoteExact b)) := by
  have hfe := (bounds_denoteExact hv).frontier_sub
  exact ⟨hfe.trans hsub, fun φ hφ => hφ.mono_right hfe⟩

/-- **F6's orientation for the solid.** -/
theorem oriented_denote {b : BodyModel E} (hv : ValidExact b) {f : FaceModel E} (hf : f ∈ b.faces)
    {y : E} (hy : Alone b f y) : Oriented f.carrier (denoteExact b) f.sense y :=
  (bounds_denoteExact hv).oriented f hf y hy

end ForgeCurved.BodyModel
