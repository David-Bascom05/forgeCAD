import ForgeBoolProof.Compose

/-!
# D7, the mathematics: the keep rule picks the boundary facets (`keepRuleBd_proof`)

On an arrangement of two valid exact solids' faces (`ArrOK`), with every region's class exact
(`ClassesOK`), `keepRule` keeps exactly the boundary facets of the kept rooms, each once. A
coplanar piece is kept once, as A's region.

The argument per nonempty facet `g`, with rooms `rp g`, `rm g`:
* **`facet_rooms`**, for a closed regular solid `S` whose frontier is on the faces:
  - `g` on `frontier S`: exactly one of the two rooms is in `S`. If both were, a neighbourhood of a
    facet point would lie in `S` (FacetData's `alone` and the sides); if neither were, the
    facet point would not be in the closure of `interior S`;
  - `g` in `interior S`: both rooms are in `S`;
  - `g` off `S`: neither is.
* A region's own solid has `g` on its frontier (`ArrOK.on_frontier`), and its class places `g`
  against the other solid. So the four memberships (room × solid) satisfy `RelB`.
* **`table`**: the keep rule against those four Booleans, checked by `decide` over every op,
  body, class and membership pattern. A region is kept only on a boundary facet, and on a
  boundary facet A's region is kept, and B's is too unless the piece is coplanar.
* `keepRoom r` is the op on the room's memberships (`keepRoom_eq`), so `Bd` is the table's
  inequality.
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeCurved.FacetSides
  ForgeCurved.F6 ForgeCurved.Rooms

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-! ## The table -/

/-- The op on Booleans. -/
def evalB : Op → Bool → Bool → Bool
  | .union, a, b => a || b
  | .inter, a, b => a && b
  | .diff, a, b => a && !b

/-- What a class says of the memberships of the facet's two rooms: `o1 o2` in the region's own
solid, `t1 t2` in the other, with exactly one room in its own solid. -/
def RelB : Class → Bool → Bool → Bool → Bool → Bool
  | .inside, _, _, t1, t2 => t1 && t2
  | .outside, _, _, t1, t2 => !t1 && !t2
  | .onSame, o1, o2, t1, t2 => (t1 == o1) && (t2 == o2)
  | .onOpposite, o1, o2, t1, t2 => (t1 == !o1) && (t2 == !o2)

/-- **The keep table.** -/
theorem table (op : Op) (s : Bool) (c : Class) (o1 o2 t1 t2 : Bool) (ho : o1 ≠ o2)
    (hr : RelB c o1 o2 t1 t2 = true) :
    (keepRule op s c = true →
      evalB op (pick s o1 t1) (pick s t1 o1) ≠ evalB op (pick s o2 t2) (pick s t2 o2)) ∧
    ((s = true ∨ c = .inside ∨ c = .outside) →
      evalB op (pick s o1 t1) (pick s t1 o1) ≠ evalB op (pick s o2 t2) (pick s t2 o2) →
      keepRule op s c = true) := by
  revert ho hr
  cases op <;> cases s <;> cases c <;> cases o1 <;> cases o2 <;> cases t1 <;> cases t2 <;> decide

/-! ## The rooms of a facet -/

variable {Aex Bex : BodyModel E} {a : Arr E}

/-- **The rooms of a facet, against a solid.** -/
theorem facet_rooms (ha : ArrOK Aex Bex a) {S : Set E} (hcl : IsClosed S)
    (hreg : closure (interior S) = S) (hfr : frontier S ⊆ a.FacesG) {g : Nat}
    (hne : (a.G.facet g).Nonempty) :
    (a.G.facet g ⊆ frontier S → (a.G.room (a.G.rp g) ⊆ S ↔ ¬ a.G.room (a.G.rm g) ⊆ S)) ∧
    (a.G.facet g ⊆ interior S → a.G.room (a.G.rp g) ⊆ S ∧ a.G.room (a.G.rm g) ⊆ S) ∧
    (Disjoint (a.G.facet g) S → ¬ a.G.room (a.G.rp g) ⊆ S ∧ ¬ a.G.room (a.G.rm g) ⊆ S) := by
  obtain ⟨y, hy⟩ := hne
  have hd := ha.data
  -- a room next to the facet that meets `S` (or its complement) lies in `S` (misses it)
  have inS : ∀ s : Bool, (∀ᶠ z in 𝓝 y, z ∈ S) →
      a.G.room (if s then a.G.rm g else a.G.rp g) ⊆ S := fun s hev => by
    obtain ⟨z, hz, hzS⟩ := side_point hd hy s (Q := (· ∈ S)) (hev.mono fun z h _ => h)
    exact room_in hd hfr (ha.conn _) hz hzS
  have outS : ∀ s : Bool, (∀ᶠ z in 𝓝 y, z ∉ S) →
      ¬ a.G.room (if s then a.G.rm g else a.G.rp g) ⊆ S := fun s hev h => by
    obtain ⟨z, hz, hzS⟩ := side_point hd hy s (Q := (· ∉ S)) (hev.mono fun z h _ => h)
    exact hzS (h hz)
  refine ⟨fun hF => ?_, fun hI => ?_, fun hD => ?_⟩
  · -- on the frontier: exactly one room in `S`
    obtain ⟨W, hW, -, hWz⟩ := hd.alone g y hy
    have ev1 := (side_room hd hy false).1
    have ev2 := (side_room hd hy true).1
    simp only [Bool.false_eq_true, ↓reduceIte] at ev1 ev2
    have hcases : ∀ᶠ z in 𝓝 y, z ∈ a.G.room (a.G.rp g) ∨ z ∈ a.G.room (a.G.rm g) ∨
        z ∈ a.G.facet g := by
      filter_upwards [hW, ev1, ev2] with z hzW h1 h2
      rcases lt_trichotomy (a.cf g z) 0 with h | h | h
      · exact Or.inr (Or.inl (h2 h))
      · exact Or.inr (Or.inr (hWz z hzW h))
      · exact Or.inl (h1 h)
    have hyF : y ∈ frontier S := hF hy
    constructor
    · intro hp hm
      have : ∀ᶠ z in 𝓝 y, z ∈ S := hcases.mono fun z hz => by
        rcases hz with h | h | h
        · exact hp h
        · exact hm h
        · exact hcl.frontier_subset (hF h)
      exact hyF.2 (mem_interior_iff_mem_nhds.mpr this)
    · intro hm
      by_contra hp
      have dis : ∀ r, ¬ a.G.room r ⊆ S → Disjoint (a.G.room r) S := fun r h =>
        (room_class (ha.conn r) ((hd.room_faces r).mono_right hfr)).resolve_left h
      have hp' := dis _ hp
      have hm' := dis _ hm
      have : ∀ᶠ z in 𝓝 y, z ∉ interior S := hcases.mono fun z hz hzI => by
        rcases hz with h | h | h
        · exact disjoint_left.mp hp' h (interior_subset hzI)
        · exact disjoint_left.mp hm' h (interior_subset hzI)
        · exact (hF h).2 hzI
      have hcl' : y ∈ closure (interior S) := by rw [hreg]; exact hcl.frontier_subset hyF
      obtain ⟨z, hz1, hz2⟩ := ((mem_closure_iff_frequently.mp hcl').and_eventually this).exists
      exact hz2 hz1
  · -- inside
    have hev : ∀ᶠ z in 𝓝 y, z ∈ S := mem_interior_iff_mem_nhds.mp (hI hy)
    exact ⟨by simpa using inS false hev, by simpa using inS true hev⟩
  · -- outside
    have hyS : y ∉ S := disjoint_left.mp hD hy
    have hev : ∀ᶠ z in 𝓝 y, z ∉ S := hcl.isOpen_compl.mem_nhds hyS
    exact ⟨by simpa using outS false hev, by simpa using outS true hev⟩

/-- A room next to a nonempty facet is nonempty. -/
theorem room_nonempty (ha : ArrOK Aex Bex a) {g : Nat} (hne : (a.G.facet g).Nonempty) (s : Bool) :
    (a.G.room (if s then a.G.rm g else a.G.rp g)).Nonempty := by
  obtain ⟨y, hy⟩ := hne
  obtain ⟨z, hz, -⟩ := side_point ha.data hy s (Q := fun _ => True)
    (Eventually.of_forall fun _ _ => trivial)
  exact ⟨z, hz⟩

open Classical in
theorem decide_eval (op : Op) (p q : Prop) :
    decide (op.eval p q) = evalB op (decide p) (decide q) := by
  by_cases hp : p <;> by_cases hq : q <;> cases op <;> simp [ForgeExact.Op.eval, evalB, hp, hq]

open Classical in
/-- **A nonempty room is kept by the op on its memberships.** -/
theorem keepRoom_eq (ha : ArrOK Aex Bex a) {SA SB : Set E} (hA : frontier SA ⊆ a.FacesG)
    (hB : frontier SB ⊆ a.FacesG) (op : Op) {r : Nat} (hne : (a.G.room r).Nonempty) :
    keepRoom a.G SA SB op r =
      evalB op (decide (a.G.room r ⊆ SA)) (decide (a.G.room r ⊆ SB)) := by
  obtain ⟨x₀, hx₀⟩ := hne
  have mem : ∀ {S : Set E}, frontier S ⊆ a.FacesG → ∀ x ∈ a.G.room r, (x ∈ S ↔ a.G.room r ⊆ S) :=
    fun hS x hx => by
      rcases room_class (ha.conn r) ((ha.data.room_faces r).mono_right hS) with h | h
      · exact ⟨fun _ => h, fun _ => h hx⟩
      · exact ⟨fun h' => absurd h' (disjoint_left.mp h hx), fun h' => absurd (h' hx)
          (disjoint_left.mp h hx)⟩
  unfold keepRoom
  rw [← decide_eval]
  congr 1
  apply propext
  constructor
  · rintro ⟨x, hx, hev⟩
    rwa [propext (mem hA x hx), propext (mem hB x hx)] at hev
  · intro h
    exact ⟨x₀, hx₀, by rwa [propext (mem hA x₀ hx₀), propext (mem hB x₀ hx₀)]⟩

/-! ## The proof -/

theorem denoteExact_pick (s : Bool) : denoteExact (pick s Aex Bex) =
    pick s (denoteExact Aex) (denoteExact Bex) := by cases s <;> rfl

open Classical in
/-- A class, read on the four memberships of its facet's rooms. -/
theorem rel_of_classAt {G : FaceGeo E} {g : Nat} {Own Oth : Set E} {k : Class}
    (hk : ClassAt G g Own Oth k)
    (RT : (G.facet g ⊆ frontier Oth → (G.room (G.rp g) ⊆ Oth ↔ ¬ G.room (G.rm g) ⊆ Oth)) ∧
      (G.facet g ⊆ interior Oth → G.room (G.rp g) ⊆ Oth ∧ G.room (G.rm g) ⊆ Oth) ∧
      (Disjoint (G.facet g) Oth → ¬ G.room (G.rp g) ⊆ Oth ∧ ¬ G.room (G.rm g) ⊆ Oth)) :
    RelB k (decide (G.room (G.rp g) ⊆ Own)) (decide (G.room (G.rm g) ⊆ Own))
      (decide (G.room (G.rp g) ⊆ Oth)) (decide (G.room (G.rm g) ⊆ Oth)) = true := by
  cases k <;> simp only [ClassAt] at hk <;> simp only [RelB]
  · obtain ⟨h1, h2⟩ := RT.2.1 hk; simp [h1, h2]
  · obtain ⟨h1, h2⟩ := RT.2.2 hk; simp [h1, h2]
  · obtain ⟨-, h1, h2⟩ := hk; simp [h1, h2]
  · obtain ⟨-, h1, h2⟩ := hk; simp [h1, h2]

open Classical in
/-- `Bd` on the four memberships, read per body. -/
theorem bd_iff {G : FaceGeo E} {g : Nat} {SA SB : Set E} {keepR : Nat → Bool} (op : Op) (s : Bool)
    (kp : keepR (G.rp g) = evalB op (decide (G.room (G.rp g) ⊆ SA)) (decide (G.room (G.rp g) ⊆ SB)))
    (km : keepR (G.rm g) = evalB op (decide (G.room (G.rm g) ⊆ SA)) (decide (G.room (G.rm g) ⊆ SB))) :
    Bd G keepR g ↔
      evalB op (pick s (decide (G.room (G.rp g) ⊆ pick s SA SB)) (decide (G.room (G.rp g) ⊆ pick s SB SA)))
          (pick s (decide (G.room (G.rp g) ⊆ pick s SB SA)) (decide (G.room (G.rp g) ⊆ pick s SA SB))) ≠
        evalB op (pick s (decide (G.room (G.rm g) ⊆ pick s SA SB)) (decide (G.room (G.rm g) ⊆ pick s SB SA)))
          (pick s (decide (G.room (G.rm g) ⊆ pick s SB SA)) (decide (G.room (G.rm g) ⊆ pick s SA SB))) := by
  unfold Bd
  rw [kp, km]
  cases s <;> simp [pick]

open Classical in
/-- **Per region**: its own frontier and its class give `RelB` on the facet's rooms, and the
table applies. -/
theorem region_table (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    (op : Op) {c : Nat → Class} (hc : ClassesOK a (denoteExact Aex) (denoteExact Bex) c)
    {f : Nat} (hf : f ∈ a.regions) (hne : (a.G.facet (a.gf f)).Nonempty) :
    (keepRule op (a.src f) (c f) = true →
      Bd a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op) (a.gf f)) ∧
    ((a.src f = true ∨ c f = .inside ∨ c f = .outside) →
      Bd a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op) (a.gf f) →
      keepRule op (a.src f) (c f) = true) := by
  have hBA := bounds_denoteExact hvA
  have hBB := bounds_denoteExact hvB
  have hfrA : frontier (denoteExact Aex) ⊆ a.FacesG := by
    rw [ha.faces]; exact hBA.frontier_sub.trans subset_union_left
  have hfrB : frontier (denoteExact Bex) ⊆ a.FacesG := by
    rw [ha.faces]; exact hBB.frontier_sub.trans subset_union_right
  have hown : ∀ t : Bool, IsClosed (pick t (denoteExact Aex) (denoteExact Bex)) ∧
      closure (interior (pick t (denoteExact Aex) (denoteExact Bex))) =
        pick t (denoteExact Aex) (denoteExact Bex) ∧
      frontier (pick t (denoteExact Aex) (denoteExact Bex)) ⊆ a.FacesG := fun t => by
    cases t
    · exact ⟨hBB.closed, hBB.regular, hfrB⟩
    · exact ⟨hBA.closed, hBA.regular, hfrA⟩
  have hoth : ∀ t : Bool, pick t (denoteExact Bex) (denoteExact Aex) =
      pick (!t) (denoteExact Aex) (denoteExact Bex) := fun t => by cases t <;> rfl
  obtain ⟨F, hF, hgF, -⟩ := ha.on_face f hf
  have hgOwn : a.G.facet (a.gf f) ⊆ frontier (pick (a.src f) (denoteExact Aex) (denoteExact Bex)) := by
    have := ha.on_frontier hvA hvB (a.gf f) (a.src f) hne (hgF.trans fun y hy => mem_iUnion₂.mpr ⟨F, hF, hy⟩)
    rwa [denoteExact_pick] at this
  obtain ⟨cO, rO, fO⟩ := hown (a.src f)
  obtain ⟨cT, rT, fT⟩ := hown (!a.src f)
  rw [← hoth] at cT rT fT
  have RO := facet_rooms ha cO rO fO hne
  have RT := facet_rooms ha cT rT fT hne
  have hr := rel_of_classAt (hc f hf) RT
  have ho : decide (a.G.room (a.G.rp (a.gf f)) ⊆ pick (a.src f) (denoteExact Aex) (denoteExact Bex)) ≠
      decide (a.G.room (a.G.rm (a.gf f)) ⊆ pick (a.src f) (denoteExact Aex) (denoteExact Bex)) := by
    have h := RO.1 hgOwn
    by_cases hp : a.G.room (a.G.rp (a.gf f)) ⊆ pick (a.src f) (denoteExact Aex) (denoteExact Bex)
    · have := h.mp hp; simp [hp, this]
    · have : a.G.room (a.G.rm (a.gf f)) ⊆ pick (a.src f) (denoteExact Aex) (denoteExact Bex) := by
        by_contra hm; exact hp (h.mpr hm)
      simp [hp, this]
  have hp := room_nonempty ha hne false
  have hm := room_nonempty ha hne true
  simp only [Bool.false_eq_true, ↓reduceIte] at hp hm
  have hBd := bd_iff op (a.src f) (keepRoom_eq ha hfrA hfrB op hp) (keepRoom_eq ha hfrA hfrB op hm)
  have T := table op (a.src f) (c f) _ _ _ _ ho hr
  exact ⟨fun h => hBd.mpr (T.1 h), fun h hb => T.2 h (hBd.mp hb)⟩

/-- **D7, the mathematics, proven.** -/
theorem keepRuleBd_proof : KeepRuleBd (E := E) := by
  intro Aex Bex a op c hvA hvB ha hc
  have hBA := bounds_denoteExact hvA
  refine ⟨fun g hg => ?_, fun f hf f' hf' hne h1 h2 h3 => ?_⟩
  · constructor
    · intro hbd
      -- a region of the side the facet lies on; A's region if the facet is on both
      by_cases hgA : a.G.facet g ⊆ Faces Aex
      · obtain ⟨f, hf, hgf, hs⟩ := ha.cover g true hg hgA
        subst hgf
        exact ⟨f, hf, rfl, (region_table hvA hvB ha op hc hf hg).2 (Or.inl hs) hbd⟩
      · have hgB : a.G.facet g ⊆ Faces Bex := (ha.facet_side g hg).resolve_left hgA
        obtain ⟨f, hf, hgf, hs⟩ := ha.cover g false hg hgB
        subst hgf
        refine ⟨f, hf, rfl, (region_table hvA hvB ha op hc hf hg).2 (Or.inr ?_) hbd⟩
        -- B's region on a facet off A's faces is inside or outside A
        have hcf : ClassAt a.G (a.gf f) (pick (a.src f) (denoteExact Aex) (denoteExact Bex))
            (pick (a.src f) (denoteExact Bex) (denoteExact Aex)) (c f) := hc f hf
        rw [hs] at hcf
        simp only [pick, Bool.false_eq_true, ↓reduceIte] at hcf
        cases hcl : c f
        · exact Or.inl rfl
        · exact Or.inr rfl
        · rw [hcl] at hcf; simp only [ClassAt] at hcf
          exact absurd (hcf.1.trans hBA.frontier_sub) hgA
        · rw [hcl] at hcf; simp only [ClassAt] at hcf
          exact absurd (hcf.1.trans hBA.frontier_sub) hgA
    · rintro ⟨f, hf, rfl, hk⟩
      exact (region_table hvA hvB ha op hc hf hg).1 hk
  · -- two kept regions on one nonempty facet: of one body by `inj`; otherwise B's would be
    -- inside or outside A while the facet is on A's frontier
    by_cases hs : a.src f = a.src f'
    · exact ha.inj f hf f' hf' h3 hs
    · exfalso
      have key : ∀ fa fb, fa ∈ a.regions → fb ∈ a.regions → a.src fa = true → a.src fb = false →
          a.gf fa = a.gf fb → (a.G.facet (a.gf fa)).Nonempty →
          keepRule op false (c fb) = true → False := by
        intro fa fb hfa hfb hsa hsb hg hne' hk
        obtain ⟨F, hF, hgF, -⟩ := ha.on_face fa hfa
        rw [hsa] at hF
        have hfrA := ha.on_frontier hvA hvB (a.gf fa) true hne'
          (hgF.trans fun y hy => mem_iUnion₂.mpr ⟨F, hF, hy⟩)
        obtain ⟨y, hy⟩ := hne'
        have hyF : y ∈ frontier (denoteExact Aex) := hfrA hy
        have hcf : ClassAt a.G (a.gf fb) (pick (a.src fb) (denoteExact Aex) (denoteExact Bex))
            (pick (a.src fb) (denoteExact Bex) (denoteExact Aex)) (c fb) := hc fb hfb
        rw [hsb] at hcf
        simp only [pick, Bool.false_eq_true, ↓reduceIte] at hcf
        rw [hg] at hy
        cases hcl : c fb <;> rw [hcl] at hcf hk <;> simp only [ClassAt] at hcf
        · exact hyF.2 (hcf hy)
        · exact disjoint_left.mp hcf hy (hBA.closed.frontier_subset hyF)
        · cases op <;> simp [keepRule] at hk
        · cases op <;> simp [keepRule] at hk
      cases hsf : a.src f <;> cases hsf' : a.src f' <;> simp only [hsf, hsf', not_true_eq_false] at hs
      · exact key f' f hf' hf hsf' hsf h3.symm (h3 ▸ hne) (by rw [← hsf]; exact h1)
      · exact key f f' hf hf' hsf hsf' h3 hne (by rw [← hsf']; exact h2)

/-- **§D ⇒ §E with the boundary lemma discharged**: the nine per-step contracts alone give
`BooleanCorrect` of the pipeline. -/
theorem boolean_correct_of_steps {Body K Q : Type} {δ : ℝ} {abs : Body → BodyModel E}
    {sc : Scope Body K Q} {Names : Body → Body → Op → Body → Prop} {Size : Body → Prop}
    {P : Pipeline Body (Refusal E K Q)} {V : View P E}
    (h1 : D1 δ abs sc Size P V) (h2 : D2 δ abs sc Size P V) (h3 : D3 δ abs sc Size P V)
    (h4 : D4 δ abs sc Size P V) (h5 : D5 δ abs sc Size P V) (h6 : D6 δ abs sc Size P V)
    (h7 : D7 δ abs sc Size P V) (h8 : D8 δ abs sc Names Size P V) (h9 : D9 δ abs sc Size P) :
    BooleanCorrect δ abs sc Names Size P.run :=
  boolean_correct_of_contracts h1 h2 h3 h4 h5 h6 h7 h8 h9 keepRuleBd_proof

end ForgeBoolProof
