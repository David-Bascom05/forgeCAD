import ForgeBoolProof.Seeded

/-!
# D8.formula: the result's local formula `Φ_u` inside a vertex zone

`BoundsFaces.local_formula`: inside result vertex `u`'s zone `Z`, off `u`'s incident carriers,
`x ∈ S ↔ Φ_u.den x`. From:

* (a) **D8.iso**: the result faces that meet `Z` are `u`'s incident faces, and the frontier of
  `S` lies on the faces (`hfrR`, as for D8.orient);
* `oriented`: the result's faces are oriented (D8.orient, `oriented_result`);
* (c) **D8.cone, faces**: at a point of an incident face (off `J`, where two incident carriers
  vanish), `Φ_u` holds on the face's inside side and fails on its outside;
* (c') **D8.cone, extensions**: at a zero of an incident carrier that is on no incident face (the
  carrier's extension past its face, off `J`), `Φ_u` does not change: it is constant off the
  incident carriers near there;
* (d) **D8.cells**: `Z \ J` is preconnected (crossings are generic: through pieces of one carrier,
  not where carriers meet), and some incident face has a point there.

`(c)` and `(c')` together are the code's half of H.abs (D8.cone): what the translated `Φ_u` must
mean. The L-block's inside corner (three planar faces bounding the one missing octant, the other
seven inside): the octant diagonal to the missing one touches no face; `Φ_u` is true there, and
reaching it crosses only the three carriers' extensions, where by (c') neither `Φ_u` nor the solid
changes.

The argument: off the incident carriers both sides of `↔` are locally constant (`Φ_u` reads only
incident carriers, `formula_reads`; the frontier is on the incident faces). Near a face point
orientation and (c) make them agree; near an extension point neither changes (c'). So "agree
nearby" and "disagree nearby" are open, cover `Z \ J` and are disjoint, and a face point agrees;
by (d) all of `Z \ J` agrees, so every point off the incident carriers does.
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.F6 ForgeExact ForgeExact.DecideOnce
  ForgeExact.DecideOnce.Curved

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- A formula's value depends only on the signs of the literals it reads. -/
theorem den_congr {P : Nat → Prop} {cs : List (Carrier E ℝ)} {x z : E} :
    ∀ {Φ : SForm}, Reads P Φ →
      (∀ j, P j → ∀ c, cs[j]? = some c → sgnK (c z) = sgnK (c x)) →
      (Φ.den cs z ↔ Φ.den cs x)
  | .lit j s, hr, h => by
    simp only [SForm.den]
    constructor
    · rintro ⟨c, hc, hs⟩; exact ⟨c, hc, by rw [← h j hr c hc]; exact hs⟩
    · rintro ⟨c, hc, hs⟩; exact ⟨c, hc, by rw [h j hr c hc]; exact hs⟩
  | .tt, _, _ => Iff.rfl
  | .ff, _, _ => Iff.rfl
  | .and a b, hr, h => by
    simp only [SForm.den]; rw [den_congr hr.1 h, den_congr hr.2 h]
  | .or a b, hr, h => by
    simp only [SForm.den]; rw [den_congr hr.1 h, den_congr hr.2 h]
  | .not a, hr, h => by
    simp only [SForm.den]; rw [den_congr (Φ := a) hr h]

/-- Near a point where a continuous carrier is nonzero, its sign is the same. -/
theorem sgn_eventually {c : E → ℝ} (hc : Continuous c) {x : E} (hx : c x ≠ 0) :
    ∀ᶠ z in 𝓝 x, sgnK (c z) = sgnK (c x) := by
  rcases lt_or_gt_of_ne hx with h | h
  · filter_upwards [hc.continuousAt.eventually (gt_mem_nhds h)] with z hz
    simp [sgnK, hz, h, not_lt.mpr hz.le, not_lt.mpr h.le]
  · filter_upwards [hc.continuousAt.eventually (lt_mem_nhds h)] with z hz
    simp [sgnK, hz, h]

/-- Off a closed set's frontier, membership is locally constant. -/
theorem mem_eventually {S : Set E} (hS : IsClosed S) {x : E} (hx : x ∉ frontier S) :
    ∀ᶠ z in 𝓝 x, (z ∈ S ↔ x ∈ S) := by
  by_cases hi : x ∈ interior S
  · filter_upwards [mem_interior_iff_mem_nhds.mp hi] with z hz
    exact ⟨fun _ => interior_subset hi, fun _ => hz⟩
  · have hxS : x ∉ S := fun h => hx ⟨subset_closure h, hi⟩
    filter_upwards [hS.isOpen_compl.mem_nhds hxS] with z hz
    exact ⟨fun h => absurd h hz, fun h => absurd h hxS⟩

theorem inSide_or_outSide (o : Bool) {a : ℝ} (h : a ≠ 0) : inSide o a ∨ outSide o a := by
  rcases lt_or_gt_of_ne h with h | h <;> cases o <;> simp [inSide, outSide, h]

/-- **D8.formula.** -/
theorem local_formula_of_cone {Rs : BodyModel E} {S : Set E} (hS : IsClosed S) (hwf : WellFormed Rs)
    {u : Nat} {Φ : SForm} (hΦ : Rs.formulas[u]? = some Φ) {Z : Set E} (hZ : IsOpen Z)
    -- the points where two incident carriers vanish
    (J : Set E)
    (hJ : J = {y | ∃ (i j : Nat) (F G : FaceModel E), i ≠ j ∧ Rs.faces[i]? = some F ∧ Rs.faces[j]? = some G ∧
      IncV Rs u F ∧ IncV Rs u G ∧ F.carrier y = 0 ∧ G.carrier y = 0})
    -- (a) D8.iso: the faces meeting the zone are incident; the frontier is on the faces
    (hiso : ∀ F ∈ Rs.faces, ∀ y ∈ F.region, y ∈ Z → IncV Rs u F)
    (hfrR : frontier S ⊆ Faces Rs)
    -- D8.orient
    (hor : ∀ F ∈ Rs.faces, ∀ y, Alone Rs F y → Oriented F.carrier S F.sense y)
    -- (c) D8.cone at the faces
    (hcone : ∀ F ∈ Rs.faces, IncV Rs u F → ∀ y ∈ Z \ J, y ∈ F.region → ∀ᶠ z in 𝓝 y,
      (inSide F.sense (F.carrier z) → Φ.den (carriers Rs) z) ∧
      (outSide F.sense (F.carrier z) → ¬ Φ.den (carriers Rs) z))
    -- (c') D8.cone at the extensions
    (hext : ∀ y ∈ Z \ J, (∀ F ∈ Rs.faces, IncV Rs u F → y ∉ F.region) → ∃ N ∈ 𝓝 y,
      ∀ z ∈ N, ∀ z' ∈ N, (∀ F ∈ Rs.faces, IncV Rs u F → F.carrier z ≠ 0) →
        (∀ F ∈ Rs.faces, IncV Rs u F → F.carrier z' ≠ 0) →
        (Φ.den (carriers Rs) z ↔ Φ.den (carriers Rs) z'))
    -- (d) D8.cells
    (hconn : IsPreconnected (Z \ J))
    (hbase : ∃ F ∈ Rs.faces, IncV Rs u F ∧ ∃ y ∈ F.region, y ∈ Z \ J) :
    ∀ x ∈ Z, (∀ F ∈ Rs.faces, IncV Rs u F → F.carrier x ≠ 0) → (x ∈ S ↔ Φ.den (carriers Rs) x) := by
  set cs := carriers Rs
  set Off : E → Prop := fun z => z ∈ Z ∧ ∀ F ∈ Rs.faces, IncV Rs u F → F.carrier z ≠ 0
  set Pr : E → Prop := fun z => (z ∈ S ↔ Φ.den cs z)
  set Gs := {y | ∀ᶠ z in 𝓝 y, Off z → Pr z}
  set Bs := {y | ∀ᶠ z in 𝓝 y, Off z → ¬ Pr z}
  have hreads := hwf.formula_reads u Φ hΦ
  -- the frontier, inside the zone, is on the incident faces
  have hfrZ : ∀ y ∈ Z, y ∈ frontier S → ∃ F ∈ Rs.faces, IncV Rs u F ∧ y ∈ F.region := by
    intro y hyZ hy
    obtain ⟨F, hF, hyF⟩ := mem_iUnion₂.mp (hfrR hy)
    exact ⟨F, hF, hiso F hF y hyF hyZ, hyF⟩
  -- `Φ_u` is locally constant off the incident carriers
  have hΦloc : ∀ x, Off x → ∀ᶠ z in 𝓝 x, (Φ.den cs z ↔ Φ.den cs x) := by
    intro x hx
    have hj : ∀ j : Nat, ∀ᶠ z in 𝓝 x, (∃ f, Rs.faces[j]? = some f ∧ IncV Rs u f) →
        ∀ c : Carrier E ℝ, cs[j]? = some c → sgnK (c z) = sgnK (c x) := by
      intro j
      by_cases hP : ∃ f, Rs.faces[j]? = some f ∧ IncV Rs u f
      · obtain ⟨f, hf, hinc⟩ := hP
        have hfm : f ∈ Rs.faces := List.mem_of_getElem? hf
        filter_upwards [sgn_eventually f.cont (hx.2 f hfm hinc)] with z hz _ c hc
        have : c = f.carrier := by
          simp only [cs, carriers, List.getElem?_map, hf, Option.map_some, Option.some.injEq] at hc
          exact hc.symm
        rw [this]; exact hz
      · exact Filter.Eventually.of_forall fun z h => absurd h hP
    have hall : ∀ᶠ z in 𝓝 x, ∀ j : Nat, j ∈ Finset.range Rs.faces.length →
        (∃ f, Rs.faces[j]? = some f ∧ IncV Rs u f) →
          ∀ c : Carrier E ℝ, cs[j]? = some c → sgnK (c z) = sgnK (c x) :=
      (Finset.range Rs.faces.length).eventually_all.mpr fun j _ => hj j
    filter_upwards [hall] with z hz
    refine den_congr hreads fun j hPj c hc => ?_
    have ⟨f, hf, _⟩ := hPj
    exact hz j (Finset.mem_range.mpr (List.getElem?_eq_some_iff.mp hf).1) hPj c hc
  -- the face points of `Z \ J` agree nearby
  have face_case : ∀ y ∈ Z \ J, ∀ G ∈ Rs.faces, IncV Rs u G → y ∈ G.region → y ∈ Gs := by
    intro y hy G hG hinc hyG
    have hal : Alone Rs G y := by
      refine ⟨hyG, fun H hH hne hyH => hy.2 ?_⟩
      obtain ⟨i, hi⟩ := List.mem_iff_getElem?.mp hG
      obtain ⟨j, hj⟩ := List.mem_iff_getElem?.mp hH
      rw [hJ]
      refine ⟨i, j, G, H, fun hij => hne ?_, hi, hj, hinc, hiso H hH y hyH hy.1,
        G.on_carrier y hyG, H.on_carrier y hyH⟩
      subst hij; rw [hi] at hj; exact (Option.some.inj hj).symm
    obtain ⟨hin, hout⟩ := hor G hG y hal
    filter_upwards [hin, hout, hcone G hG hinc y hy hyG] with z h1 h2 ⟨h3, h4⟩ hz
    rcases inSide_or_outSide G.sense (hz.2 G hG hinc) with hs | hs
    · exact ⟨fun _ => h3 hs, fun _ => h1 hs⟩
    · exact ⟨fun h => absurd h (h2 hs), fun h => absurd h (h4 hs)⟩
  -- every point of `Z \ J` agrees or disagrees nearby
  have hcov : Z \ J ⊆ Gs ∪ Bs := by
    intro y hy
    by_cases hoff : Off y
    · have hfr : y ∉ frontier S := fun h => by
        obtain ⟨F, hF, hinc, hyF⟩ := hfrZ y hy.1 h
        exact hoff.2 F hF hinc (F.on_carrier y hyF)
      have hloc : ∀ᶠ z in 𝓝 y, (Pr z ↔ Pr y) := by
        filter_upwards [mem_eventually hS hfr, hΦloc y hoff] with z h1 h2
        simp only [Pr]; rw [h1, h2]
      by_cases hPy : Pr y
      · exact Or.inl (hloc.mono fun z h _ => h.mpr hPy)
      · exact Or.inr (hloc.mono fun z h _ hz => hPy (h.mp hz))
    · by_cases hface : ∃ G ∈ Rs.faces, IncV Rs u G ∧ y ∈ G.region
      · obtain ⟨G, hG, hinc, hyG⟩ := hface
        exact Or.inl (face_case y hy G hG hinc hyG)
      · push Not at hface
        have hfr : y ∉ frontier S := fun h => by
          obtain ⟨F, hF, hinc, hyF⟩ := hfrZ y hy.1 h
          exact hface F hF hinc hyF
        obtain ⟨N, hN, hNc⟩ := hext y hy hface
        have hM : N ∩ {z | (z ∈ S ↔ y ∈ S)} ∈ 𝓝 y := inter_mem hN (mem_eventually hS hfr)
        by_cases hgood : ∃ z0 ∈ N ∩ {z | (z ∈ S ↔ y ∈ S)}, Off z0 ∧ Pr z0
        · obtain ⟨z0, ⟨hz0N, hz0S⟩, hz0o, hz0P⟩ := hgood
          refine Or.inl (Filter.mem_of_superset hM fun z ⟨hzN, hzS⟩ hzo => ?_)
          have hΦ' := hNc z hzN z0 hz0N hzo.2 hz0o.2
          simp only [Pr, mem_ofPred_eq] at hzS hz0S hz0P ⊢
          rw [hzS, hΦ', ← hz0S]; exact hz0P
        · push Not at hgood
          exact Or.inr (Filter.mem_of_superset hM fun z hz hzo => hgood z hz hzo)
  -- not both
  have hdis : Z \ J ∩ (Gs ∩ Bs) = ∅ := by
    rw [eq_empty_iff_forall_notMem]
    rintro y ⟨hy, hyG, hyB⟩
    obtain ⟨W, hW, hWo, hyW⟩ := mem_nhds_iff.mp (hyG.and hyB)
    obtain ⟨z, ⟨hzW, hzZ⟩, hz0⟩ := diff_nonempty_of_open (hWo.inter hZ) ⟨y, hyW, hy.1⟩
      (zeros_interior Rs)
    have hzo : Off z := ⟨hzZ, fun F hF _ h0 => hz0 (mem_iUnion₂.mpr ⟨F, hF, h0⟩)⟩
    obtain ⟨h1, h2⟩ := hW hzW
    exact h2 hzo (h1 hzo)
  have hGo : IsOpen Gs := isOpen_setOfPred_eventually_nhds
  have hBo : IsOpen Bs := isOpen_setOfPred_eventually_nhds
  have hallG : Z \ J ⊆ Gs := by
    rcases (isPreconnected_iff_subset_of_disjoint.mp hconn) Gs Bs hGo hBo hcov hdis with h | h
    · exact h
    · exfalso
      obtain ⟨F, hF, hinc, y, hyF, hy⟩ := hbase
      have : y ∈ Z \ J ∩ (Gs ∩ Bs) := ⟨hy, face_case y hy F hF hinc hyF, h hy⟩
      rw [hdis] at this; exact this
  intro x hxZ hoff
  have hxJ : x ∉ J := by
    rw [hJ]; rintro ⟨i, j, F, G, -, hi, -, hinc, -, h0, -⟩
    exact hoff F (List.mem_of_getElem? hi) hinc h0
  exact (hallG ⟨hxZ, hxJ⟩).self_of_nhds ⟨hxZ, hoff⟩

end ForgeBoolProof
