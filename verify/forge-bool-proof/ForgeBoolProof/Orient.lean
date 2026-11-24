import ForgeBoolProof.KeepRule
import ForgeBoolProof.Assembly

/-!
# D8.orient, the keep rule's half: a kept region's written sense points into the kept rooms

A region `f` lies on a face of its own body with sense `a.sense f`: its inside side is the side of
`a.cf (gf f)` that `inSide (a.sense f)` reads. `apply` writes each kept region with the sense
`writtenSense op (src f) (a.sense f)`: its own, reversed for B's regions in a difference (A \ B
keeps B's inside regions, with A \ B on their outside).

**`kept_sense`**: if the region's inside room is in its own solid (`RegionSense`, what `on_face`
and the body's orientation give), then for a kept region the room on the inside of its written
sense is kept and the room on its outside is not. With this, a result face carried by kept
regions with their written senses is oriented at its facet points (`oriented_on_facet`).
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeCurved.FacetSides
  ForgeCurved.F6 ForgeCurved.Rooms ForgeCurved.Chart

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The op is a difference. -/
def Op.isDiff : Op → Bool
  | .diff => true
  | _ => false

/-- The sense `apply` writes a kept region with: its own, reversed for B's regions in a
difference. -/
def writtenSense (op : Op) (s sn : Bool) : Bool := xor (op.isDiff && !s) sn

/-- **The sense table**: for a kept region whose inside room is in its own solid, the op keeps
the room on the inside of the written sense and drops the one on its outside. -/
theorem sense_table (op : Op) (s : Bool) (c : Class) (sn o1 o2 t1 t2 : Bool) (ho : o1 ≠ o2)
    (hr : RelB c o1 o2 t1 t2 = true) (hk : keepRule op s c = true)
    (hin : (if sn then o2 else o1) = true) :
    (if writtenSense op s sn then evalB op (pick s o2 t2) (pick s t2 o2)
      else evalB op (pick s o1 t1) (pick s t1 o1)) = true ∧
    (if writtenSense op s sn then evalB op (pick s o1 t1) (pick s t1 o1)
      else evalB op (pick s o2 t2) (pick s t2 o2)) = false := by
  revert ho hr hk hin
  cases op <;> cases s <;> cases c <;> cases sn <;> cases o1 <;> cases o2 <;> cases t1 <;>
    cases t2 <;> decide

variable {Aex Bex : BodyModel E} {a : Arr E}

/-- A region's sense points into its own solid: the room on the inside of its sense is in it. -/
def RegionSense (Aex Bex : BodyModel E) (a : Arr E) (f : Nat) : Prop :=
  a.G.room (inRoom a.G (a.sense f) (a.gf f)) ⊆ pick (a.src f) (denoteExact Aex) (denoteExact Bex)

open Classical in
theorem eval_pick_mem (op : Op) (s : Bool) (R SA SB : Set E) :
    evalB op (decide (R ⊆ SA)) (decide (R ⊆ SB)) =
      evalB op (pick s (decide (R ⊆ pick s SA SB)) (decide (R ⊆ pick s SB SA)))
        (pick s (decide (R ⊆ pick s SB SA)) (decide (R ⊆ pick s SA SB))) := by
  cases s <;> rfl

open Classical in
/-- **The kept room is on the inside of the written sense.** -/
theorem kept_sense (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    (op : Op) {c : Nat → Class} (hc : ClassesOK a (denoteExact Aex) (denoteExact Bex) c)
    {f : Nat} (hf : f ∈ a.regions) (hne : (a.G.facet (a.gf f)).Nonempty)
    (hk : keepRule op (a.src f) (c f) = true) (hs : RegionSense Aex Bex a f) :
    keepRoom a.G (denoteExact Aex) (denoteExact Bex) op
        (inRoom a.G (writtenSense op (a.src f) (a.sense f)) (a.gf f)) = true ∧
      keepRoom a.G (denoteExact Aex) (denoteExact Bex) op
        (outRoom a.G (writtenSense op (a.src f) (a.sense f)) (a.gf f)) = false := by
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
    have := ha.on_frontier hvA hvB (a.gf f) (a.src f) hne
      (hgF.trans fun y hy => mem_iUnion₂.mpr ⟨F, hF, hy⟩)
    rwa [denoteExact_pick] at this
  obtain ⟨cO, rO, fO⟩ := hown (a.src f)
  obtain ⟨cT, rT, fT⟩ := hown (!a.src f)
  rw [← hoth] at cT rT fT
  have RO := facet_rooms ha cO rO fO hne
  have RT := facet_rooms ha cT rT fT hne
  have hr := rel_of_classAt (hc f hf) RT
  set Own := pick (a.src f) (denoteExact Aex) (denoteExact Bex)
  set Oth := pick (a.src f) (denoteExact Bex) (denoteExact Aex)
  have ho : decide (a.G.room (a.G.rp (a.gf f)) ⊆ Own) ≠ decide (a.G.room (a.G.rm (a.gf f)) ⊆ Own) := by
    have h := RO.1 hgOwn
    by_cases hp : a.G.room (a.G.rp (a.gf f)) ⊆ Own
    · have := h.mp hp; simp [hp, this]
    · have : a.G.room (a.G.rm (a.gf f)) ⊆ Own := by
        by_contra hm; exact hp (h.mpr hm)
      simp [hp, this]
  have hin : (if a.sense f then decide (a.G.room (a.G.rm (a.gf f)) ⊆ Own)
      else decide (a.G.room (a.G.rp (a.gf f)) ⊆ Own)) = true := by
    unfold RegionSense inRoom at hs
    cases hsn : a.sense f <;> simp only [hsn, Bool.false_eq_true, ↓reduceIte] at hs ⊢ <;>
      exact decide_eq_true hs
  have hp := room_nonempty ha hne false
  have hm := room_nonempty ha hne true
  simp only [Bool.false_eq_true, ↓reduceIte] at hp hm
  have kp := keepRoom_eq ha hfrA hfrB op hp
  have km := keepRoom_eq ha hfrA hfrB op hm
  -- the memberships, A and B, through the region's own and other solid
  have hAB : ∀ r, evalB op (decide (a.G.room r ⊆ denoteExact Aex)) (decide (a.G.room r ⊆ denoteExact Bex)) =
      evalB op (pick (a.src f) (decide (a.G.room r ⊆ Own)) (decide (a.G.room r ⊆ Oth)))
        (pick (a.src f) (decide (a.G.room r ⊆ Oth)) (decide (a.G.room r ⊆ Own))) := fun r =>
    eval_pick_mem op (a.src f) _ _ _
  have T := sense_table op (a.src f) (c f) (a.sense f) _ _ _ _ ho hr hk hin
  unfold inRoom outRoom
  cases hw : writtenSense op (a.src f) (a.sense f) <;>
    simp only [hw, Bool.false_eq_true, ↓reduceIte] at T ⊢ <;>
    rw [kp, km, hAB, hAB] <;> exact T

/-- **Oriented on a kept facet.** A face (carrier `c`, sense `o`) whose sides near a point `y` of
a facet `g` are the sides of `g`'s carrier with sense `w`, where the room on the inside of `w` is
kept and the one on its outside is not, is oriented at `y` against the kept rooms. -/
theorem oriented_on_facet (ha : ArrOK Aex Bex a) {keepR : Nat → Bool} {g : Nat} {w : Bool}
    (hin : keepR (inRoom a.G w g) = true) (hout : keepR (outRoom a.G w g) = false)
    {c : E → ℝ} {o : Bool} {y : E} (hy : y ∈ a.G.facet g)
    (hsides : ∀ᶠ z in 𝓝 y, (inSide o (c z) ↔ inSide w (a.cf g z)) ∧
      (outSide o (c z) ↔ outSide w (a.cf g z))) :
    Oriented c (Res a.G keepR) o y := by
  have hd := ha.data
  have hs := sideLocal_of_data hd keepR
  obtain ⟨evI, -⟩ := side_room hd hy w
  obtain ⟨evO, -⟩ := side_room hd hy (!w)
  refine ⟨?_, ?_⟩
  · filter_upwards [hsides, evI] with z hz h1 hzi
    have hroom := h1 (by have := hz.1.mp hzi; cases w <;> simpa [inSide] using this)
    have : a.G.room (inRoom a.G w g) ⊆ Res a.G keepR :=
      (room_sub_kept hin).trans subset_closure
    exact this (by cases w <;> simpa [inRoom] using hroom)
  · filter_upwards [hsides, evO] with z hz h1 hzo
    have hroom := h1 (by have := hz.2.mp hzo; cases w <;> simpa [outSide] using this)
    exact Set.disjoint_left.mp (room_disj_res hs hout) (by cases w <;> simpa [outRoom] using hroom)

/-- **A region's sense from its face's orientation**: where the region's face `F` is alone in its
own body at a point `y` of the facet, and the region's sides are `F`'s there (`on_face`), the
room on the inside of the region's sense is in its own solid. -/
theorem regionSense_of_alone (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    {f : Nat} {F : FaceModel E} (hF : F ∈ (pick (a.src f) Aex Bex).faces) {y : E}
    (hy : y ∈ a.G.facet (a.gf f)) (hal : Alone (pick (a.src f) Aex Bex) F y)
    (hsides : ∀ᶠ z in 𝓝 y,
      (inSide (a.sense f) (a.cf (a.gf f) z) ↔ inSide F.sense (F.carrier z)) ∧
      (outSide (a.sense f) (a.cf (a.gf f) z) ↔ outSide F.sense (F.carrier z))) :
    RegionSense Aex Bex a f := by
  have hv : ValidExact (pick (a.src f) Aex Bex) := by cases a.src f <;> assumption
  have hor := oriented_denote hv hF hal
  have hfr : frontier (denoteExact (pick (a.src f) Aex Bex)) ⊆ a.FacesG := by
    rw [ha.faces]
    exact (bounds_denoteExact hv).frontier_sub.trans (by
      cases a.src f
      · exact subset_union_right
      · exact subset_union_left)
  have hin : ∀ᶠ z in 𝓝 y, (if a.sense f then a.cf (a.gf f) z < 0 else 0 < a.cf (a.gf f) z) →
      z ∈ denoteExact (pick (a.src f) Aex Bex) := by
    filter_upwards [hsides, hor.1] with z hz h1 hs
    exact h1 (hz.1.mp (by cases hsn : a.sense f <;> simp only [hsn] at hs <;> simpa [inSide, hsn] using hs))
  obtain ⟨z, hz, hzS⟩ := side_point ha.data hy (a.sense f) hin
  have := room_in ha.data hfr (ha.conn _) hz hzS
  unfold RegionSense inRoom
  rw [← denoteExact_pick]
  exact this

/-- **What `apply` writes, with senses** (ledger D8.writes, the sense half): each point of a result
face that is on a facet is on the facet of a kept region, near which the face's sides are that
facet's carrier's sides with the region's written sense. -/
def WritesSense (a : Arr E) (op : Op) (k : Nat → Bool) (Rs : BodyModel E) : Prop :=
  ∀ F ∈ Rs.faces, ∀ y ∈ F.region, y ∈ FacetPts a.G → ∃ f ∈ a.regions, k f = true ∧
    y ∈ a.G.facet (a.gf f) ∧ ∀ᶠ z in 𝓝 y,
      (inSide F.sense (F.carrier z) ↔ inSide (writtenSense op (a.src f) (a.sense f)) (a.cf (a.gf f) z)) ∧
      (outSide F.sense (F.carrier z) ↔ outSide (writtenSense op (a.src f) (a.sense f)) (a.cf (a.gf f) z))

/-- **D8.orient on the facets**: from exact classes, the keep rule's flags, what `apply` writes
(`WritesSense`) and the regions' senses (`RegionSense`, e.g. by `regionSense_of_alone`), each
result face is oriented against the kept rooms at every one of its points on a facet. -/
theorem oriented_on_facets (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    (op : Op) {c : Nat → Class} (hc : ClassesOK a (denoteExact Aex) (denoteExact Bex) c)
    {k : Nat → Bool} (hk : KeepOK a op c k) {Rs : BodyModel E} (hw : WritesSense a op k Rs)
    (hrs : ∀ f ∈ a.regions, k f = true → RegionSense Aex Bex a f)
    {F : FaceModel E} (hF : F ∈ Rs.faces) {y : E} (hy : y ∈ F.region) (hyP : y ∈ FacetPts a.G) :
    Oriented F.carrier (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)) F.sense y := by
  obtain ⟨f, hf, hkf, hyf, hsides⟩ := hw F hF y hy hyP
  have hkr : keepRule op (a.src f) (c f) = true := by rw [← hk f hf]; exact hkf
  obtain ⟨hin, hout⟩ := kept_sense hvA hvB ha op hc hf ⟨y, hyf⟩ hkr (hrs f hf hkf)
  exact oriented_on_facet ha hin hout hyf hsides

/-! ## Off the facets: by the limit of facet points -/

/-- **Oriented by nearby oriented points.** At a regular zero `y` of `c`, if near `y` the
frontier of the closed set `S` is on `c`'s zeros, and arbitrarily near `y` there are points where
the face is oriented with both its sides met, then the face is oriented at `y`: each half of a
small box round `y` is connected and off the frontier, so wholly in `S` or wholly out, and the
nearby oriented point decides which. -/
theorem oriented_of_near {S : Set E} (hS : IsClosed S) {c : E → ℝ} {o : Bool} {y : E}
    {L : E →L[ℝ] ℝ} (hc : HasStrictFDerivAt c L y) (hL : L ≠ 0) (hc0 : c y = 0)
    {W : Set E} (hW : W ∈ 𝓝 y) (hfr : frontier S ∩ W ⊆ {z | c z = 0})
    (hnear : ∀ U ∈ 𝓝 y, ∃ y' ∈ U, Oriented c S o y' ∧ (∃ᶠ z in 𝓝 y', inSide o (c z)) ∧
      ∃ᶠ z in 𝓝 y', outSide o (c z)) :
    Oriented c S o y := by
  obtain ⟨N, hNo, hyN, hNW, hp1, -, hn1, -⟩ := box_halves hc hL hc0 hW
  -- each side of the box: wholly in `S` or wholly out of it
  have half : ∀ (P : ℝ → Prop), (∀ a, P a → a ≠ 0) → IsPreconnected (N ∩ {z | P (c z)}) →
      N ∩ {z | P (c z)} ⊆ interior S ∨ N ∩ {z | P (c z)} ⊆ Sᶜ := by
    intro P hP hconn
    have hcov : N ∩ {z | P (c z)} ⊆ interior S ∪ Sᶜ := by
      intro z hz
      by_cases hzI : z ∈ interior S
      · exact Or.inl hzI
      · right; intro hzS
        exact hP _ hz.2 (hfr ⟨⟨subset_closure hzS, hzI⟩, hNW hz.1⟩)
    have hdis : Disjoint (interior S) Sᶜ := disjoint_compl_right.mono_left interior_subset
    by_cases hne : (N ∩ {z | P (c z)}).Nonempty
    · rcases (isPreconnected_iff_subset_of_disjoint.mp hconn) _ _ isOpen_interior
          hS.isOpen_compl hcov (by rw [Set.disjoint_iff_inter_eq_empty.mp hdis, Set.inter_empty]) with h | h
      · exact Or.inl h
      · exact Or.inr h
    · rw [not_nonempty_iff_eq_empty] at hne; rw [hne]; exact Or.inl (empty_subset _)
  have hin_conn : IsPreconnected (N ∩ {z | inSide o (c z)}) := by
    cases o
    · simpa [inSide] using hp1
    · simpa [inSide] using hn1
  have hout_conn : IsPreconnected (N ∩ {z | outSide o (c z)}) := by
    cases o
    · simpa [outSide] using hn1
    · simpa [outSide] using hp1
  have hinP : ∀ a, inSide o a → a ≠ 0 := fun a h => by
    cases o <;> simp only [inSide, Bool.false_eq_true, ↓reduceIte] at h
    · exact ne_of_gt h
    · exact ne_of_lt h
  have houtP : ∀ a, outSide o a → a ≠ 0 := fun a h => by
    cases o <;> simp only [outSide, Bool.false_eq_true, ↓reduceIte] at h
    · exact ne_of_lt h
    · exact ne_of_gt h
  obtain ⟨y', hy'N, ⟨hI, hO⟩, hfI, hfO⟩ := hnear N (hNo.mem_nhds hyN)
  have hN' : N ∈ 𝓝 y' := hNo.mem_nhds hy'N
  refine ⟨?_, ?_⟩
  · rcases half _ hinP hin_conn with h | h
    · exact Filter.mem_of_superset (hNo.mem_nhds hyN) fun z hz hs => interior_subset (h ⟨hz, hs⟩)
    · exfalso
      obtain ⟨z, ⟨hs, hzS⟩, hzN⟩ := ((hfI.and_eventually hI).and_eventually hN').exists
      exact h ⟨hzN, hs⟩ (hzS hs)
  · rcases half _ houtP hout_conn with h | h
    · exfalso
      obtain ⟨z, ⟨hs, hzS⟩, hzN⟩ := ((hfO.and_eventually hO).and_eventually hN').exists
      exact hzS hs (interior_subset (h ⟨hzN, hs⟩))
    · exact Filter.mem_of_superset (hNo.mem_nhds hyN) fun z hz hs => h ⟨hz, hs⟩

/-- At a point of a facet, a face whose sides are the facet carrier's (with some sense) meets both
its sides arbitrarily close. -/
theorem freq_sides (ha : ArrOK Aex Bex a) {g : Nat} {w : Bool} {c : E → ℝ} {o : Bool} {y : E}
    (hy : y ∈ a.G.facet g)
    (hsides : ∀ᶠ z in 𝓝 y, (inSide o (c z) ↔ inSide w (a.cf g z)) ∧
      (outSide o (c z) ↔ outSide w (a.cf g z))) :
    (∃ᶠ z in 𝓝 y, inSide o (c z)) ∧ ∃ᶠ z in 𝓝 y, outSide o (c z) := by
  obtain ⟨-, fI⟩ := side_room ha.data hy w
  obtain ⟨-, fO⟩ := side_room ha.data hy (!w)
  refine ⟨(fI.and_eventually hsides).mono fun z ⟨h, hz⟩ => hz.1.mpr ?_,
    (fO.and_eventually hsides).mono fun z ⟨h, hz⟩ => hz.2.mpr ?_⟩
  · cases w <;> simpa [inSide] using h
  · cases w <;> simpa [outSide] using h

/-- **D8.orient.** From exact classes, the keep rule's flags, what `apply` writes (`WritesSense`)
and the regions' senses (`RegionSense`), every result face is oriented against the kept rooms at
each of its lone points, given:
* the result's face regions are closed, and its faces carry the frontier of the kept rooms
  (`hfrR`: from `BoundsFaces.frontier_on` and `frontier_res_sub`);
* each result face is the closure of its points on facets (`hdense`: `apply` writes kept facets);
* off the facets (on the arrangement's edges and vertices), a result face's carrier is regular
  at its points (`hreg`).
-/
theorem oriented_result (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    (op : Op) {c : Nat → Class} (hc : ClassesOK a (denoteExact Aex) (denoteExact Bex) c)
    {k : Nat → Bool} (hk : KeepOK a op c k) {Rs : BodyModel E} (hw : WritesSense a op k Rs)
    (hrs : ∀ f ∈ a.regions, k f = true → RegionSense Aex Bex a f)
    (hcl : ∀ G ∈ Rs.faces, IsClosed G.region)
    (hfrR : frontier (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)) ⊆ Faces Rs)
    (hdense : ∀ F ∈ Rs.faces, F.region ⊆ closure (F.region ∩ FacetPts a.G))
    (hreg : ∀ F ∈ Rs.faces, ∀ y ∈ F.region, y ∉ FacetPts a.G →
      ∃ L : E →L[ℝ] ℝ, HasStrictFDerivAt F.carrier L y ∧ L ≠ 0) :
    ∀ F ∈ Rs.faces, ∀ y, Alone Rs F y →
      Oriented F.carrier (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)) F.sense y := by
  intro F hF y hal
  set S := Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)
  have hyF := hal.1
  by_cases hyP : y ∈ FacetPts a.G
  · exact oriented_on_facets hvA hvB ha op hc hk hw hrs hF hyF hyP
  obtain ⟨L, hL, hL0⟩ := hreg F hF y hyF hyP
  -- a neighbourhood off the other faces
  set T := ⋃ G ∈ {G | G ∈ Rs.faces ∧ G ≠ F}, G.region
  have hT : IsClosed T :=
    Set.Finite.isClosed_biUnion (s := {G | G ∈ Rs.faces ∧ G ≠ F})
      (Rs.faces.finite_toSet.subset fun G hG => (show G ∈ Rs.faces ∧ G ≠ F from hG).1)
      fun G hG => hcl G (show G ∈ Rs.faces ∧ G ≠ F from hG).1
  have hyT : y ∉ T := by
    intro h
    obtain ⟨G, ⟨hG, hGF⟩, hyG⟩ := mem_iUnion₂.mp h
    exact hal.2 G hG hGF hyG
  have hW : Tᶜ ∈ 𝓝 y := hT.isOpen_compl.mem_nhds hyT
  have hfr : frontier S ∩ Tᶜ ⊆ {z | F.carrier z = 0} := by
    rintro z ⟨hz, hzT⟩
    obtain ⟨G, hG, hzG⟩ := mem_iUnion₂.mp (hfrR hz)
    by_cases hGF : G = F
    · subst hGF; exact G.on_carrier z hzG
    · exact absurd (mem_iUnion₂.mpr ⟨G, ⟨hG, hGF⟩, hzG⟩) hzT
  refine oriented_of_near isClosed_closure hL hL0 (F.on_carrier y hyF) hW hfr fun U hU => ?_
  obtain ⟨y', hy'U, hy'F, hy'P⟩ := mem_closure_iff_nhds.mp (hdense F hF hyF) U hU
  obtain ⟨f, -, -, hyf, hsides⟩ := hw F hF y' hy'F hy'P
  exact ⟨y', hy'U, oriented_on_facets hvA hvB ha op hc hk hw hrs hF hy'F hy'P,
    freq_sides ha hyf hsides⟩

/-- **D8.orient with the boundary half**: `hfrR` from the arrangement's thin edges (`Thin`), the
keep rule picking the boundary facets (`KeptIsBd`) and `apply` writing them (`WritesKept`). -/
theorem oriented_result_of_writes (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    (ht : Thin a) (op : Op) {c : Nat → Class} (hc : ClassesOK a (denoteExact Aex) (denoteExact Bex) c)
    {k : Nat → Bool} (hk : KeepOK a op c k)
    (hK : KeptIsBd a k (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op))
    {Rs : BodyModel E} (hwk : WritesKept a k Rs) (hw : WritesSense a op k Rs)
    (hrs : ∀ f ∈ a.regions, k f = true → RegionSense Aex Bex a f)
    (hcl : ∀ G ∈ Rs.faces, IsClosed G.region)
    (hdense : ∀ F ∈ Rs.faces, F.region ⊆ closure (F.region ∩ FacetPts a.G))
    (hreg : ∀ F ∈ Rs.faces, ∀ y ∈ F.region, y ∉ FacetPts a.G →
      ∃ L : E →L[ℝ] ℝ, HasStrictFDerivAt F.carrier L y ∧ L ≠ 0) :
    ∀ F ∈ Rs.faces, ∀ y, Alone Rs F y →
      Oriented F.carrier (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)) F.sense y := by
  have hb := boundsFaces_boundary ha ht hK hwk
  refine oriented_result hvA hvB ha op hc hk hw hrs hcl (fun x hx => ?_) hdense hreg
  exact hb.1 ⟨frontier_res_sub ha.data _ hx, hx⟩

end ForgeBoolProof
