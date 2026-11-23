import ForgeCurved.Manifold

/-!
# Route B: the topological refusal over the face ledger (A8, rule W2)

The face ledger (research/decide-once/face-arrangement.md §1) lists **rooms** (open, connected
components of the region off the faces), **facets** (face pieces, each with its `+` and `−` room),
and per edge the facets whose closure holds it **once per wing** (`edge_facets`). Per vertex it lists
**sectors** `(facet, edge a, edge b)` (`vert_sectors`). The core refuses at an edge with more than
two boundary wings, or at a vertex whose boundary sectors are not one class of "share a bounding
edge". This file proves that this is the topological non-manifoldness of the result, given local
obligations on the builder's geometry. No covector and no carrier arrangement appear, so neither
`Faithful` nor R1 is needed.

The obligations (curved-design.md §7, F1–F5):
* `SideLocal` (F5): the facet points are in the closure of both their rooms; near a facet point
  there is only the facet and its two rooms; rooms are open and miss the facets; rooms kept
  differently are disjoint. This gives `frontier_iff_bd`: a facet point is on the frontier of
  `R = closure (kept rooms)` iff its facet is a boundary facet (its rooms kept differently).
* `EdgeLocal` (F1–F4 at an edge point `x`): a neighbourhood `W₀` and a wing label on the facet
  points in it. Every facet point there carries a wing of the row (F2). Labels are locally constant
  (F3, separation). Each wing comes arbitrarily near `x` (F1) and is one connected piece in
  arbitrarily small neighbourhoods (F4).
* `VertLocal` (the same at a vertex, with sectors): additionally an edge label on the edge points
  near `v`. Near an edge point, only sectors bounded by that edge appear. Each sector's bounding
  edges reach `v` and lie in the sector's closure.

Theorems:
* **`edge_iff`**: more than two boundary wings at `e` iff `ThreeSheetsS` at `x`: three local
  sheets of `frontier R` on the facets.
* **`vert_iff`**: boundary sectors in two fan classes iff `TwoFansS` at `v`.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.FaceLocal

open ForgeCurved Filter Topology

variable {P : Type} [TopologicalSpace P]

/-! ## The geometry and the sets -/

/-- The builder's point sets: facets, edges and rooms by id, with each facet's two rooms. -/
structure FaceGeo (P : Type) where
  facet : Nat → Set P
  edge : Nat → Set P
  room : Nat → Set P
  /-- the room on the `+` side of a facet, and on the `−` side -/
  rp : Nat → Nat
  rm : Nat → Nat

variable (G : FaceGeo P)

def FacetPts : Set P := ⋃ f, G.facet f
def EdgePts : Set P := ⋃ a, G.edge a

/-- The kept rooms. -/
def Kept (keep : Nat → Bool) : Set P := ⋃ r, ⋃ (_ : keep r = true), G.room r

/-- The result: the closure of the kept rooms. -/
def Res (keep : Nat → Bool) : Set P := closure (Kept G keep)

/-- A boundary facet: its rooms are kept differently. -/
def Bd (keep : Nat → Bool) (f : Nat) : Prop := keep (G.rp f) ≠ keep (G.rm f)

/-- The sheets: boundary points on facets. -/
def sheet (R : Set P) : Set P := frontier R ∩ FacetPts G

/-- Round `v`: the sheets, and edge points in their closure, but not `v`. -/
def fanS (R : Set P) (v : P) : Set P :=
  {y | y ≠ v ∧ (y ∈ sheet G R ∨ (y ∈ EdgePts G ∧ y ∈ closure (sheet G R)))}

/-- Three local sheets at `x`. -/
def ThreeSheetsS (R : Set P) (x : P) : Prop :=
  ∃ W₀ ∈ 𝓝 x, ∀ U ∈ 𝓝 x, ∃ y₁ ∈ sheet G R ∩ U, ∃ y₂ ∈ sheet G R ∩ U, ∃ y₃ ∈ sheet G R ∩ U,
    ¬Joined (sheet G R ∩ W₀) y₁ y₂ ∧ ¬Joined (sheet G R ∩ W₀) y₁ y₃ ∧ ¬Joined (sheet G R ∩ W₀) y₂ y₃

/-- Two local fans at `v`. -/
def TwoFansS (R : Set P) (v : P) : Prop :=
  ∃ W₀ ∈ 𝓝 v, ∀ U ∈ 𝓝 v, ∃ y ∈ sheet G R ∩ U, ∃ z ∈ sheet G R ∩ U, ¬Joined (fanS G R v ∩ W₀) y z

/-! ## F5: sides, and the frontier -/

/-- **F5**: the local picture at facet points, and the rooms. -/
structure SideLocal (keep : Nat → Bool) : Prop where
  room_open : ∀ r, IsOpen (G.room r)
  room_facet : ∀ r f, Disjoint (G.room r) (G.facet f)
  room_disj : ∀ r r', keep r ≠ keep r' → Disjoint (G.room r) (G.room r')
  sides : ∀ f, ∀ y ∈ G.facet f, y ∈ closure (G.room (G.rp f)) ∧ y ∈ closure (G.room (G.rm f))
  local_ : ∀ f, ∀ y ∈ G.facet f, ∃ N ∈ 𝓝 y, N ⊆ G.facet f ∪ G.room (G.rp f) ∪ G.room (G.rm f)

variable {G}

theorem room_sub_kept {keep : Nat → Bool} {r : Nat} (h : keep r = true) : G.room r ⊆ Kept G keep :=
  fun _ hy => Set.mem_iUnion.mpr ⟨r, Set.mem_iUnion.mpr ⟨h, hy⟩⟩

theorem room_disj_res {keep : Nat → Bool} (hs : SideLocal G keep) {r : Nat} (h : keep r = false) :
    Disjoint (G.room r) (Res G keep) := by
  rw [Set.disjoint_iff]
  rintro y ⟨hy, hyR⟩
  obtain ⟨w, hw, hwK⟩ := mem_closure_iff.mp hyR _ (hs.room_open r) hy
  obtain ⟨r', hr'⟩ := Set.mem_iUnion.mp hwK
  obtain ⟨hk, hw'⟩ := Set.mem_iUnion.mp hr'
  exact Set.disjoint_left.mp (hs.room_disj r r' (by rw [h, hk]; decide)) hw hw'

theorem res_frontier (keep : Nat → Bool) :
    frontier (Res G keep) = Res G keep \ interior (Res G keep) := isClosed_closure.frontier_eq

/-- **A boundary facet lies on the frontier.** -/
theorem frontier_of_bd {keep : Nat → Bool} (hs : SideLocal G keep) {f : Nat} (hb : Bd G keep f) :
    G.facet f ⊆ frontier (Res G keep) := by
  intro y hy
  obtain ⟨hp, hm⟩ := hs.sides f y hy
  have key : ∀ a b : Nat, keep a = true → keep b = false → y ∈ closure (G.room a) →
      y ∈ closure (G.room b) → y ∈ frontier (Res G keep) := by
    intro a b ha hb' hya hyb
    rw [res_frontier]
    refine ⟨closure_mono (room_sub_kept ha) hya, fun hI => ?_⟩
    obtain ⟨w, hwI, hwb⟩ := mem_closure_iff.mp hyb _ isOpen_interior hI
    exact Set.disjoint_left.mp (room_disj_res hs hb') hwb (interior_subset hwI)
  unfold Bd at hb
  cases e : keep (G.rp f) <;> cases e' : keep (G.rm f) <;> rw [e, e'] at hb
  · exact absurd rfl hb
  · exact key _ _ e' e hm hp
  · exact key _ _ e e' hp hm
  · exact absurd rfl hb

/-- **A facet point on the frontier is on a boundary facet.** -/
theorem bd_of_frontier {keep : Nat → Bool} (hs : SideLocal G keep) {f : Nat} {y : P}
    (hy : y ∈ G.facet f) (hfr : y ∈ frontier (Res G keep)) : Bd G keep f := by
  intro hsame
  obtain ⟨N, hN, hNs⟩ := hs.local_ f y hy
  rw [res_frontier] at hfr
  obtain ⟨hR, hI⟩ := hfr
  cases e : keep (G.rp f)
  · -- neither kept: a neighbourhood misses the kept rooms
    have e' : keep (G.rm f) = false := by rw [← hsame, e]
    obtain ⟨w, hwN, hwK⟩ := mem_closure_iff_nhds.mp hR N hN
    obtain ⟨r, hr⟩ := Set.mem_iUnion.mp hwK
    obtain ⟨hk, hwr⟩ := Set.mem_iUnion.mp hr
    rcases hNs hwN with (hw | hw) | hw
    · exact Set.disjoint_left.mp (hs.room_facet r f) hwr hw
    · exact Set.disjoint_left.mp (hs.room_disj r (G.rp f) (by rw [hk, e]; decide)) hwr hw
    · exact Set.disjoint_left.mp (hs.room_disj r (G.rm f) (by rw [hk, e']; decide)) hwr hw
  · -- both kept: a neighbourhood lies in the closure of the kept rooms
    have e' : keep (G.rm f) = true := by rw [← hsame, e]
    apply hI
    refine mem_interior_iff_mem_nhds.mpr (Filter.mem_of_superset hN fun w hwN => ?_)
    rcases hNs hwN with (hw | hw) | hw
    · exact closure_mono (room_sub_kept e) (hs.sides f w hw).1
    · exact subset_closure (room_sub_kept e hw)
    · exact subset_closure (room_sub_kept e' hw)

/-! ## Edges -/

/-- **F1–F4 at an edge point `x`**, for the edge's row `wf` (one entry per wing). -/
structure EdgeLocal (G : FaceGeo P) (wf : List Nat) (x : P) where
  W₀ : Set P
  hW₀ : W₀ ∈ 𝓝 x
  lab : P → Nat
  /-- F2: a facet point near `x` is on a wing of the row -/
  cov : ∀ y ∈ W₀, y ∈ FacetPts G → lab y < wf.length ∧ y ∈ G.facet (wf.getD (lab y) 0)
  /-- F3: wings are separated -/
  loc : ∀ y ∈ W₀ ∩ FacetPts G, ∃ N ∈ 𝓝 y, ∀ z ∈ N ∩ (W₀ ∩ FacetPts G), lab z = lab y
  /-- F1: each wing reaches `x` -/
  near : ∀ i < wf.length, x ∈ closure {y | y ∈ W₀ ∧ y ∈ G.facet (wf.getD i 0) ∧ lab y = i}
  /-- F4: each wing is one piece in arbitrarily small neighbourhoods -/
  conn : ∀ i < wf.length, ∀ U ∈ 𝓝 x, ∃ U' ∈ 𝓝 x, U' ⊆ U ∧
    IsPreconnected ({y | y ∈ W₀ ∧ y ∈ G.facet (wf.getD i 0) ∧ lab y = i} ∩ U')

/-- The wing pieces. -/
def EdgeLocal.piece {wf : List Nat} {x : P} (h : EdgeLocal G wf x) (i : Nat) : Set P :=
  {y | y ∈ h.W₀ ∧ y ∈ G.facet (wf.getD i 0) ∧ h.lab y = i}

/-- The core's bad edge, over wings: three boundary wings. -/
def BadEdgeW (G : FaceGeo P) (keep : Nat → Bool) (wf : List Nat) : Prop :=
  ∃ i j k, i < wf.length ∧ j < wf.length ∧ k < wf.length ∧ i ≠ j ∧ i ≠ k ∧ j ≠ k ∧
    Bd G keep (wf.getD i 0) ∧ Bd G keep (wf.getD j 0) ∧ Bd G keep (wf.getD k 0)

theorem piece_sheet {keep : Nat → Bool} (hs : SideLocal G keep) {wf : List Nat} {x : P}
    (h : EdgeLocal G wf x) {i : Nat} (hb : Bd G keep (wf.getD i 0)) :
    h.piece i ⊆ sheet G (Res G keep) ∩ h.W₀ := fun _ hy =>
  ⟨⟨frontier_of_bd hs hb hy.2.1, Set.mem_iUnion.mpr ⟨_, hy.2.1⟩⟩, hy.1⟩

theorem lab_const {wf : List Nat} {x : P} (h : EdgeLocal G wf x) {S : Set P}
    (hS : S ⊆ h.W₀ ∩ FacetPts G) {y z : P} (hj : Joined S y z) : h.lab y = h.lab z := by
  rcases hj with e | ⟨C, hCS, hC, hy, hz⟩
  · rw [e]
  · exact const_of_loc h.loc hC (hCS.trans hS) hy hz

/-- **The edge: more than two boundary wings iff three local sheets.** -/
theorem edge_iff {keep : Nat → Bool} (hs : SideLocal G keep) {wf : List Nat} {x : P}
    (h : EdgeLocal G wf x) : BadEdgeW G keep wf ↔ ThreeSheetsS G (Res G keep) x := by
  constructor
  · rintro ⟨i, j, k, hi, hj, hk, hij, hik, hjk, bi, bj, bk⟩
    refine ⟨h.W₀, h.hW₀, fun U hU => ?_⟩
    have pick : ∀ l, l < wf.length → Bd G keep (wf.getD l 0) →
        ∃ y ∈ sheet G (Res G keep) ∩ U, y ∈ h.W₀ ∧ h.lab y = l := by
      intro l hl bl
      obtain ⟨y, hyU, hy⟩ := mem_closure_iff_nhds.mp (h.near l hl) U hU
      exact ⟨y, ⟨(piece_sheet hs h bl hy).1, hyU⟩, hy.1, hy.2.2⟩
    obtain ⟨y₁, s₁, w₁, l₁⟩ := pick i hi bi
    obtain ⟨y₂, s₂, w₂, l₂⟩ := pick j hj bj
    obtain ⟨y₃, s₃, w₃, l₃⟩ := pick k hk bk
    have sub : sheet G (Res G keep) ∩ h.W₀ ⊆ h.W₀ ∩ FacetPts G := fun y hy => ⟨hy.2, hy.1.2⟩
    refine ⟨y₁, s₁, y₂, s₂, y₃, s₃, fun e => hij ?_, fun e => hik ?_, fun e => hjk ?_⟩
    · rw [← l₁, ← l₂]; exact lab_const h sub e
    · rw [← l₁, ← l₃]; exact lab_const h sub e
    · rw [← l₂, ← l₃]; exact lab_const h sub e
  · rintro ⟨W₁, hW₁, H⟩
    set V := interior (h.W₀ ∩ W₁)
    have hV : V ∈ 𝓝 x := interior_mem_nhds.mpr (Filter.inter_mem h.hW₀ hW₁)
    let Q : Nat → Set P → Prop := fun i U => U ⊆ V ∧ IsPreconnected (h.piece i ∩ U)
    have hQ : ∀ i ∈ List.range wf.length, ∃ U ∈ 𝓝 x, Q i U := by
      intro i hi
      obtain ⟨U', hU', hsub, hc⟩ := h.conn i (List.mem_range.mp hi) V hV
      exact ⟨U', hU', hsub, hc⟩
    obtain ⟨g, hg, hU₀⟩ := nhds_choice _ Q hQ
    set U₀ := ⋂ i ∈ {i | i ∈ List.range wf.length}, g i
    obtain ⟨y₁, ⟨s₁, u₁⟩, y₂, ⟨s₂, u₂⟩, y₃, ⟨s₃, u₃⟩, n₁₂, n₁₃, n₂₃⟩ :=
      H (U₀ ∩ V) (Filter.inter_mem hU₀ hV)
    have info : ∀ {y}, y ∈ sheet G (Res G keep) → y ∈ U₀ ∩ V →
        h.lab y < wf.length ∧ Bd G keep (wf.getD (h.lab y) 0) ∧ y ∈ h.piece (h.lab y) ∧
          y ∈ g (h.lab y) := by
      intro y hs' hu
      have hyW : y ∈ h.W₀ := (interior_subset hu.2).1
      obtain ⟨hl, hf⟩ := h.cov y hyW hs'.2
      exact ⟨hl, bd_of_frontier hs hf hs'.1, ⟨hyW, hf, rfl⟩,
        Set.mem_iInter₂.mp hu.1 _ (List.mem_range.mpr hl)⟩
    have ne : ∀ {y z}, y ∈ sheet G (Res G keep) → z ∈ sheet G (Res G keep) → y ∈ U₀ ∩ V →
        z ∈ U₀ ∩ V → ¬Joined (sheet G (Res G keep) ∩ W₁) y z → h.lab y ≠ h.lab z := by
      intro y z hy hz uy uz hn e
      obtain ⟨hl, hb, hpy, hgy⟩ := info hy uy
      obtain ⟨-, -, hpz, hgz⟩ := info hz uz
      have hQ' := (hg _ (List.mem_range.mpr hl)).1
      apply hn
      refine Or.inr ⟨h.piece (h.lab y) ∩ g (h.lab y), fun w hw => ⟨(piece_sheet hs h hb hw.1).1, ?_⟩,
        hQ'.2, ⟨hpy, hgy⟩, ⟨e ▸ hpz, e ▸ hgz⟩⟩
      exact (interior_subset (hQ'.1 hw.2)).2
    obtain ⟨l₁, b₁, -⟩ := info s₁ u₁
    obtain ⟨l₂, b₂, -⟩ := info s₂ u₂
    obtain ⟨l₃, b₃, -⟩ := info s₃ u₃
    exact ⟨_, _, _, l₁, l₂, l₃, ne s₁ s₂ u₁ u₂ n₁₂, ne s₁ s₃ u₁ u₃ n₁₃, ne s₂ s₃ u₂ u₃ n₂₃,
      b₁, b₂, b₃⟩

/-! ## Vertices -/

/-- A sector: its facet and its two bounding edges. -/
abbrev Sec := Nat × Nat × Nat

/-- The edges bounding a sector. -/
def Sec.bounds (s : Sec) (a : Nat) : Prop := a = s.2.1 ∨ a = s.2.2

/-- **F1–F4 at a vertex `v`**, for its sectors `ss`. -/
structure VertLocal (G : FaceGeo P) (ss : List Sec) (v : P) where
  W₀ : Set P
  hW₀ : W₀ ∈ 𝓝 v
  slab : P → Nat
  elb : P → Nat
  v_facet : v ∉ FacetPts G
  v_edge : v ∉ EdgePts G
  /-- F2: a facet point near `v` is on a sector -/
  cov : ∀ y ∈ W₀, y ∈ FacetPts G → slab y < ss.length ∧ y ∈ G.facet (ss.getD (slab y) (0, 0, 0)).1
  /-- F3: sectors are separated, and facet points are off the edges -/
  loc : ∀ y ∈ W₀ ∩ FacetPts G, ∃ N ∈ 𝓝 y,
    (∀ z ∈ N ∩ (W₀ ∩ FacetPts G), slab z = slab y) ∧ ∀ z ∈ N, z ∉ EdgePts G
  /-- F2/F3 at edge points: on their labelled edge; near one, the same edge, and only sectors it
  bounds -/
  ecov : ∀ y ∈ W₀, y ∈ EdgePts G → y ∈ G.edge (elb y)
  eloc : ∀ y ∈ W₀ ∩ EdgePts G, ∃ N ∈ 𝓝 y,
    (∀ z ∈ N ∩ (W₀ ∩ EdgePts G), elb z = elb y) ∧
    ∀ z ∈ N ∩ (W₀ ∩ FacetPts G), Sec.bounds (ss.getD (slab z) (0, 0, 0)) (elb y)
  /-- F1: each sector reaches `v` -/
  near : ∀ i < ss.length,
    v ∈ closure {y | y ∈ W₀ ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧ slab y = i}
  /-- F4: each sector is one piece in arbitrarily small neighbourhoods -/
  conn : ∀ i < ss.length, ∀ U ∈ 𝓝 v, ∃ U' ∈ 𝓝 v, U' ⊆ U ∧
    IsPreconnected ({y | y ∈ W₀ ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧ slab y = i} ∩ U')
  /-- F1 for edges: a sector's bounding edges reach `v` and lie in the sector's closure near it -/
  adj : ∀ i < ss.length, ∀ a, Sec.bounds (ss.getD i (0, 0, 0)) a →
    v ∈ closure (G.edge a) ∧
      G.edge a ∩ W₀ ⊆ closure {y | y ∈ W₀ ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧ slab y = i}

def VertLocal.piece {ss : List Sec} {v : P} (h : VertLocal G ss v) (i : Nat) : Set P :=
  {y | y ∈ h.W₀ ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧ h.slab y = i}

/-- Two boundary sectors sharing a bounding edge: the core's `fanRel`, on the result. -/
def FanRel (G : FaceGeo P) (keep : Nat → Bool) (ss : List Sec) (i j : Nat) : Prop :=
  i < ss.length ∧ j < ss.length ∧ Bd G keep (ss.getD i (0, 0, 0)).1 ∧
    Bd G keep (ss.getD j (0, 0, 0)).1 ∧
    ∃ a, Sec.bounds (ss.getD i (0, 0, 0)) a ∧ Sec.bounds (ss.getD j (0, 0, 0)) a

/-- The core's bad vertex, over sectors: two boundary sectors in different fan classes. -/
def BadVertS (G : FaceGeo P) (keep : Nat → Bool) (ss : List Sec) : Prop :=
  ∃ i j, i < ss.length ∧ j < ss.length ∧ Bd G keep (ss.getD i (0, 0, 0)).1 ∧
    Bd G keep (ss.getD j (0, 0, 0)).1 ∧ ¬Relation.ReflTransGen (FanRel G keep ss) i j

theorem vpiece_sheet {keep : Nat → Bool} (hs : SideLocal G keep) {ss : List Sec} {v : P}
    (h : VertLocal G ss v) {i : Nat} (hb : Bd G keep (ss.getD i (0, 0, 0)).1) :
    h.piece i ⊆ fanS G (Res G keep) v ∩ h.W₀ := fun y hy => by
  refine ⟨⟨fun e => h.v_facet (e ▸ Set.mem_iUnion.mpr ⟨_, hy.2.1⟩), Or.inl
    ⟨frontier_of_bd hs hb hy.2.1, Set.mem_iUnion.mpr ⟨_, hy.2.1⟩⟩⟩, hy.1⟩

theorem vpiece_sheet' {keep : Nat → Bool} (hs : SideLocal G keep) {ss : List Sec} {v : P}
    (h : VertLocal G ss v) {i : Nat} (hb : Bd G keep (ss.getD i (0, 0, 0)).1) :
    h.piece i ⊆ sheet G (Res G keep) := fun _ hy =>
  ⟨frontier_of_bd hs hb hy.2.1, Set.mem_iUnion.mpr ⟨_, hy.2.1⟩⟩

theorem fan_bd {keep : Nat → Bool} {ss : List Sec} {i j : Nat}
    (hr : Relation.ReflTransGen (FanRel G keep ss) i j) (hi : Bd G keep (ss.getD i (0, 0, 0)).1) :
    Bd G keep (ss.getD j (0, 0, 0)).1 := by
  induction hr with
  | refl => exact hi
  | tail _ hr _ => exact hr.2.2.2.1

theorem fan_lt {keep : Nat → Bool} {ss : List Sec} {i j : Nat}
    (hr : Relation.ReflTransGen (FanRel G keep ss) i j) (hi : i < ss.length) : j < ss.length := by
  induction hr with
  | refl => exact hi
  | tail _ hr _ => exact hr.2.1

/-- **The vertex: two fan classes of boundary sectors iff two local fans.** -/
theorem vert_iff {keep : Nat → Bool} (hs : SideLocal G keep) {ss : List Sec} {v : P}
    (h : VertLocal G ss v) : BadVertS G keep ss ↔ TwoFansS G (Res G keep) v := by
  constructor
  · rintro ⟨i, j, hi, hj, bi, bj, hn⟩
    refine ⟨h.W₀, h.hW₀, fun U hU => ?_⟩
    have pick : ∀ l, l < ss.length → Bd G keep (ss.getD l (0, 0, 0)).1 →
        ∃ y ∈ sheet G (Res G keep) ∩ U, y ∈ h.piece l := by
      intro l hl bl
      obtain ⟨y, hyU, hy⟩ := mem_closure_iff_nhds.mp (h.near l hl) U hU
      exact ⟨y, ⟨vpiece_sheet' hs h bl hy, hyU⟩, hy⟩
    obtain ⟨y, sy, py⟩ := pick i hi bi
    obtain ⟨z, sz, pz⟩ := pick j hj bj
    refine ⟨y, sy, z, sz, ?_⟩
    rintro (e | ⟨C, hCS, hC, hyC, hzC⟩)
    · exact hn (by rw [← py.2.2, ← pz.2.2, e])
    -- membership in `i`'s fan class, read off a point's sector or edge, is locally constant
    let fz : P → Prop := fun u => (u ∈ FacetPts G ∧ Relation.ReflTransGen (FanRel G keep ss) i (h.slab u)) ∨
      (u ∉ FacetPts G ∧ ∃ k, Relation.ReflTransGen (FanRel G keep ss) i k ∧
        Sec.bounds (ss.getD k (0, 0, 0)) (h.elb u))
    have hloc : ∀ u ∈ fanS G (Res G keep) v ∩ h.W₀, ∃ N ∈ 𝓝 u,
        ∀ w ∈ N ∩ (fanS G (Res G keep) v ∩ h.W₀), fz w = fz u := by
      intro u ⟨hu, huW⟩
      by_cases huF : u ∈ FacetPts G
      · obtain ⟨N, hN, hsl, hoff⟩ := h.loc u ⟨huW, huF⟩
        refine ⟨N, hN, fun w ⟨hwN, hw, hwW⟩ => ?_⟩
        have hwF : w ∈ FacetPts G := by
          rcases hw.2 with hws | ⟨hwE, -⟩
          · exact hws.2
          · exact absurd hwE (hoff w hwN)
        have e := hsl w ⟨hwN, hwW, hwF⟩
        simp only [fz, hwF, huF, e, not_true_eq_false, false_and, or_false, true_and]
      · have huE : u ∈ EdgePts G := by
          rcases hu.2 with hus | ⟨huE, -⟩
          · exact absurd hus.2 huF
          · exact huE
        obtain ⟨N, hN, hel, hsec⟩ := h.eloc u ⟨huW, huE⟩
        -- facet points near `u` are off the edges, so `u`'s neighbourhood is split cleanly
        refine ⟨N, hN, fun w ⟨hwN, hw, hwW⟩ => ?_⟩
        by_cases hwF : w ∈ FacetPts G
        · have hbnd := hsec w ⟨hwN, hwW, hwF⟩
          have hws : w ∈ sheet G (Res G keep) := by
            rcases hw.2 with hws | ⟨hwE, -⟩
            · exact hws
            · obtain ⟨N', hN', -, hoff⟩ := h.loc w ⟨hwW, hwF⟩
              exact absurd hwE (hoff w (mem_of_mem_nhds hN'))
          obtain ⟨hl, hf⟩ := h.cov w hwW hwF
          have hbw := bd_of_frontier hs hf hws.1
          apply propext
          simp only [fz, hwF, huF, true_and, not_true_eq_false, false_and, or_false, not_false_eq_true,
            false_or]
          constructor
          · intro hiw; exact ⟨_, hiw, hbnd⟩
          · rintro ⟨k, hik, hkb⟩
            exact hik.tail ⟨fan_lt hik hi, hl, fan_bd hik bi, hbw, _, hkb, hbnd⟩
        · have hwE : w ∈ EdgePts G := by
            rcases hw.2 with hws | ⟨hwE, -⟩
            · exact absurd hws.2 hwF
            · exact hwE
          have e := hel w ⟨hwN, hwW, hwE⟩
          simp only [fz, hwF, huF, e, false_and, false_or, not_false_eq_true, true_and]
    have hfy : fz y := Or.inl ⟨(vpiece_sheet' hs h bi py).2, py.2.2 ▸ Relation.ReflTransGen.refl⟩
    have e := const_of_loc hloc hC hCS hyC hzC
    have hfz : fz z := e ▸ hfy
    rcases hfz with ⟨-, hz⟩ | ⟨hzF, -⟩
    · exact hn (pz.2.2 ▸ hz)
    · exact hzF (vpiece_sheet' hs h bj pz).2
  · rintro ⟨W₁, hW₁, H⟩
    set V := interior (h.W₀ ∩ W₁)
    have hV : V ∈ 𝓝 v := interior_mem_nhds.mpr (Filter.inter_mem h.hW₀ hW₁)
    let Q : Nat → Set P → Prop := fun i U => U ⊆ V ∧ IsPreconnected (h.piece i ∩ U)
    have hQ : ∀ i ∈ List.range ss.length, ∃ U ∈ 𝓝 v, Q i U := by
      intro i hi
      obtain ⟨U', hU', hsub, hc⟩ := h.conn i (List.mem_range.mp hi) V hV
      exact ⟨U', hU', hsub, hc⟩
    obtain ⟨g, hg, hU₀⟩ := nhds_choice _ Q hQ
    set U₀ := ⋂ i ∈ {i | i ∈ List.range ss.length}, g i
    obtain ⟨y, ⟨sy, uy⟩, z, ⟨sz, uz⟩, hn⟩ := H (U₀ ∩ V) (Filter.inter_mem hU₀ hV)
    have info : ∀ {y}, y ∈ sheet G (Res G keep) → y ∈ U₀ ∩ V →
        h.slab y < ss.length ∧ Bd G keep (ss.getD (h.slab y) (0, 0, 0)).1 ∧ y ∈ h.piece (h.slab y) := by
      intro y hs' hu
      have hyW : y ∈ h.W₀ := (interior_subset hu.2).1
      obtain ⟨hl, hf⟩ := h.cov y hyW hs'.2
      exact ⟨hl, bd_of_frontier hs hf hs'.1, ⟨hyW, hf, rfl⟩⟩
    obtain ⟨li, bi, pyi⟩ := info sy uy
    obtain ⟨lj, bj, pzj⟩ := info sz uz
    refine ⟨_, _, li, lj, bi, bj, fun hfan => hn ?_⟩
    have gU : ∀ k, k < ss.length → U₀ ⊆ g k := fun k hk w hw =>
      Set.mem_iInter₂.mp hw _ (List.mem_range.mpr hk)
    have gV : ∀ k, k < ss.length → g k ⊆ V := fun k hk => (hg k (List.mem_range.mpr hk)).1.1
    have pieceS : ∀ k, k < ss.length → Bd G keep (ss.getD k (0, 0, 0)).1 →
        h.piece k ∩ g k ⊆ fanS G (Res G keep) v ∩ W₁ := fun k hk hb w hw =>
      ⟨(vpiece_sheet hs h hb hw.1).1, (interior_subset (gV k hk hw.2)).2⟩
    have claim : ∀ k, Relation.ReflTransGen (FanRel G keep ss) (h.slab y) k →
        ∃ C ⊆ fanS G (Res G keep) v ∩ W₁, IsPreconnected C ∧ y ∈ C ∧ h.piece k ∩ g k ⊆ C := by
      intro k hk
      induction hk with
      | refl =>
        exact ⟨_, pieceS _ li bi, (hg _ (List.mem_range.mpr li)).1.2, ⟨pyi, gU _ li uy.1⟩, subset_rfl⟩
      | @tail b c _ hbc ih =>
        obtain ⟨C, hCS, hC, hyC, hbC⟩ := ih
        obtain ⟨hb, hc, bb, bc, a, hab, hac⟩ := hbc
        obtain ⟨hva, hab'⟩ := h.adj b hb a hab
        obtain ⟨-, hac'⟩ := h.adj c hc a hac
        have hint : interior U₀ ∩ interior h.W₀ ∈ 𝓝 v :=
          Filter.inter_mem (interior_mem_nhds.mpr hU₀) (interior_mem_nhds.mpr h.hW₀)
        obtain ⟨e, ⟨heU, heW⟩, hea⟩ := mem_closure_iff_nhds.mp hva _ hint
        have heb : e ∈ closure (h.piece b ∩ g b) :=
          closure_mono (Set.inter_subset_inter_right _ (interior_subset.trans (gU b hb)))
            (closure_inter_open' isOpen_interior heU (hab' ⟨hea, interior_subset heW⟩))
        have hec : e ∈ closure (h.piece c ∩ g c) :=
          closure_mono (Set.inter_subset_inter_right _ (interior_subset.trans (gU c hc)))
            (closure_inter_open' isOpen_interior heU (hac' ⟨hea, interior_subset heW⟩))
        have h1 : IsPreconnected (insert e C) := hC.subset_closure (Set.subset_insert e C)
          (Set.insert_subset_iff.mpr ⟨closure_mono hbC heb, subset_closure⟩)
        have h2 : IsPreconnected (insert e (h.piece c ∩ g c)) :=
          (hg c (List.mem_range.mpr hc)).1.2.subset_closure (Set.subset_insert e _)
            (Set.insert_subset_iff.mpr ⟨hec, subset_closure⟩)
        have heS : e ∈ fanS G (Res G keep) v ∩ W₁ := by
          refine ⟨⟨fun ev => h.v_edge (ev ▸ Set.mem_iUnion.mpr ⟨a, hea⟩), Or.inr
            ⟨Set.mem_iUnion.mpr ⟨a, hea⟩, closure_mono (fun w (hw : w ∈ h.piece b ∩ g b) => vpiece_sheet' hs h bb hw.1) heb⟩⟩, ?_⟩
          exact (interior_subset (gV b hb (gU b hb (interior_subset heU)))).2
        refine ⟨insert e C ∪ insert e (h.piece c ∩ g c), ?_, h1.union e (Set.mem_insert _ _)
          (Set.mem_insert _ _) h2, Or.inl (Set.mem_insert_of_mem _ hyC),
          fun p hp => Or.inr (Set.mem_insert_of_mem _ hp)⟩
        rintro p ((rfl | hp) | (rfl | hp))
        · exact heS
        · exact hCS hp
        · exact heS
        · exact pieceS c hc bc hp
    obtain ⟨C, hCS, hC, hyC, hzC⟩ := claim _ hfan
    exact Or.inr ⟨C, hCS, hC, hyC, hzC ⟨pzj, gU _ lj uz.1⟩⟩

/-! ## The ledger -/

/-- A face ledger's incidence rows: the listed edges and verts, `edge_facets` (with wings),
`vert_sectors`, and each vert's point. -/
structure Rows where
  es : List Nat
  vs : List Nat
  erow : Nat → List Nat
  vsec : Nat → List Sec
  vpt : Nat → P

/-- The core's refusal over the face ledger: a bad edge or a bad vert. -/
def XNonManifoldF (G : FaceGeo P) (keep : Nat → Bool) (L : Rows (P := P)) : Prop :=
  (∃ e ∈ L.es, BadEdgeW G keep (L.erow e)) ∨ (∃ v ∈ L.vs, BadVertS G keep (L.vsec v))

/-- The result is non-manifold, as sets: three local sheets at an edge point, or two local fans
at a vert. -/
def TopNonManifoldF (G : FaceGeo P) (R : Set P) (L : Rows (P := P)) : Prop :=
  (∃ e ∈ L.es, ∃ x ∈ G.edge e, ThreeSheetsS G R x) ∨ (∃ v ∈ L.vs, TwoFansS G R (L.vpt v))

/-- **The builder's obligations**: F5 for the rooms, F1–F4 at every point of every listed edge,
and at every listed vert. -/
structure Obligations (G : FaceGeo P) (keep : Nat → Bool) (L : Rows (P := P)) : Prop where
  sides : SideLocal G keep
  edge : ∀ e ∈ L.es, (G.edge e).Nonempty ∧ ∀ x ∈ G.edge e, Nonempty (EdgeLocal G (L.erow e) x)
  vert : ∀ v ∈ L.vs, Nonempty (VertLocal G (L.vsec v) (L.vpt v))

/-- **Route B's theorem**: on a face ledger whose builder meets F1–F5, the core's refusal is the
result's topological non-manifoldness. No carrier arrangement, no `Faithful`, no R1. -/
theorem xNonManifoldF_iff_top {G : FaceGeo P} {keep : Nat → Bool} {L : Rows (P := P)}
    (h : Obligations G keep L) : XNonManifoldF G keep L ↔ TopNonManifoldF G (Res G keep) L := by
  constructor
  · rintro (⟨e, he, hb⟩ | ⟨v, hv, hb⟩)
    · obtain ⟨⟨x, hx⟩, hl⟩ := h.edge e he
      exact Or.inl ⟨e, he, x, hx, (edge_iff h.sides (hl x hx).some).mp hb⟩
    · exact Or.inr ⟨v, hv, (vert_iff h.sides (h.vert v hv).some).mp hb⟩
  · rintro (⟨e, he, x, hx, h3⟩ | ⟨v, hv, h2⟩)
    · exact Or.inl ⟨e, he, (edge_iff h.sides ((h.edge e he).2 x hx).some).mpr h3⟩
    · exact Or.inr ⟨v, hv, (vert_iff h.sides (h.vert v hv).some).mpr h2⟩

end ForgeCurved.FaceLocal
