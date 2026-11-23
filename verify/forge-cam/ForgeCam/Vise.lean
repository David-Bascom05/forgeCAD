/-
The vise as forge-cam builds it (kernel/forge-cam/src/fixture.rs, 3e8a91d): the fixture is a
union of boxes (the fixed and moving jaws, the bed, the parallels). Every box's frontier lies in
its six faces, which are prism faces (two flat, four walls over segments). So the fixture meets
`Certified`'s face hypothesis with μ = 0. ops::certify_fixtured checks every body of the tool
(the cutter included) against those faces at the fixture clearance, so the corrected G2 holds.

Two readings of "clear by c":
- forge-cam's verdict is `Clear` when lower ≥ fclear + μ, which gives distance ≥ c. That keeps the
  tool out of the open grown fixture {x | dist(x, F) < c} (`G2open_of_closedForm`).
- The closed F ⊕ B_c of design.md §7b needs a margin: certify at some c' > c
  (`G2_of_closedForm`). The difference is the tool grazing at exactly c.

The held bands (G4): a band wholly below the jaws' top is all "held by fixture" (`held_band`). A
band that straddles the top splits there (`band_split`); forge-cam reports such a band whole as
"uncut".
-/
import ForgeCam.Prism

open Metric Set

namespace ForgeCam

/-- A box: lo i ≤ p i ≤ hi i for each axis. -/
def box (lo hi : Fin 3 → ℝ) : Set E3 := {p | ∀ i, lo i ≤ p i ∧ p i ≤ hi i}

theorem continuous_coord (i : Fin 3) : Continuous fun p : E3 => p i := by fun_prop

theorem isClosed_box (lo hi : Fin 3 → ℝ) : IsClosed (box lo hi) := by
  have : box lo hi = ⋂ i, ({p : E3 | lo i ≤ p i} ∩ {p | p i ≤ hi i}) := by
    ext p; simp [box]
  rw [this]
  exact isClosed_iInter fun i =>
    (isClosed_le continuous_const (continuous_coord i)).inter
      (isClosed_le (continuous_coord i) continuous_const)

/-- A frontier point of a box is on one of its six planes. -/
theorem frontier_box (lo hi : Fin 3 → ℝ) {p : E3} (hp : p ∈ frontier (box lo hi)) :
    p ∈ box lo hi ∧ ∃ i, p i = lo i ∨ p i = hi i := by
  rw [frontier, (isClosed_box lo hi).closure_eq] at hp
  refine ⟨hp.1, ?_⟩
  by_contra hno
  push Not at hno
  apply hp.2
  set U : Set E3 := ⋂ i, ({q : E3 | lo i < q i} ∩ {q | q i < hi i})
  have hU : IsOpen U := isOpen_iInter_of_finite fun i =>
    (isOpen_lt continuous_const (continuous_coord i)).inter
      (isOpen_lt (continuous_coord i) continuous_const)
  have hUB : U ⊆ box lo hi := fun q hq i => by
    have := mem_iInter.mp hq i; exact ⟨this.1.le, this.2.le⟩
  have hpU : p ∈ U := mem_iInter.mpr fun i =>
    ⟨lt_of_le_of_ne (hp.1 i).1 (hno i).1.symm, lt_of_le_of_ne (hp.1 i).2 (hno i).2⟩
  exact interior_maximal hUB hU hpU

/-- The six faces of a box, as prism faces: (section in xy, heights). -/
def boxFace (lo hi : Fin 3 → ℝ) : Fin 6 → Set (ℝ × ℝ) × Set ℝ
  | 0 => (Icc (lo 0) (hi 0) ×ˢ Icc (lo 1) (hi 1), {lo 2})
  | 1 => (Icc (lo 0) (hi 0) ×ˢ Icc (lo 1) (hi 1), {hi 2})
  | 2 => ({lo 0} ×ˢ Icc (lo 1) (hi 1), Icc (lo 2) (hi 2))
  | 3 => ({hi 0} ×ˢ Icc (lo 1) (hi 1), Icc (lo 2) (hi 2))
  | 4 => (Icc (lo 0) (hi 0) ×ˢ {lo 1}, Icc (lo 2) (hi 2))
  | 5 => (Icc (lo 0) (hi 0) ×ˢ {hi 1}, Icc (lo 2) (hi 2))

/-- **A box's frontier lies in its six prism faces.** -/
theorem frontier_box_subset (lo hi : Fin 3 → ℝ) :
    frontier (box lo hi) ⊆ ⋃ k, prism (boxFace lo hi k).1 (boxFace lo hi k).2 := by
  intro p hp
  obtain ⟨hb, i, hi'⟩ := frontier_box lo hi hp
  have h0 := hb 0; have h1 := hb 1; have h2 := hb 2
  simp only [mem_iUnion, prism, mem_ofPred_eq]
  fin_cases i <;> rcases hi' with e | e <;> simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one,
    Fin.reduceFinMk] at e
  · exact ⟨2, by simp [boxFace, e, h1.1, h1.2], by simp [boxFace, h2.1, h2.2]⟩
  · exact ⟨3, by simp [boxFace, e, h1.1, h1.2], by simp [boxFace, h2.1, h2.2]⟩
  · exact ⟨4, by simp [boxFace, e, h0.1, h0.2], by simp [boxFace, h2.1, h2.2]⟩
  · exact ⟨5, by simp [boxFace, e, h0.1, h0.2], by simp [boxFace, h2.1, h2.2]⟩
  · exact ⟨0, by simp [boxFace, h0.1, h0.2, h1.1, h1.2], by simp [boxFace, e]⟩
  · exact ⟨1, by simp [boxFace, h0.1, h0.2, h1.1, h1.2], by simp [boxFace, e]⟩

/-- The fixture: the union of its boxes (fixture.rs's `pieces`). -/
def fixtureOf (bs : List ((Fin 3 → ℝ) × (Fin 3 → ℝ))) : Set E3 := ⋃ b ∈ bs, box b.1 b.2

theorem isClosed_fixtureOf (bs : List ((Fin 3 → ℝ) × (Fin 3 → ℝ))) : IsClosed (fixtureOf bs) :=
  bs.finite_toSet.isClosed_biUnion fun b _ => isClosed_box b.1 b.2

theorem frontier_fixtureOf (bs : List ((Fin 3 → ℝ) × (Fin 3 → ℝ))) :
    frontier (fixtureOf bs) ⊆ ⋃ b ∈ bs, frontier (box b.1 b.2) := by
  induction bs with
  | nil => simp [fixtureOf]
  | cons b bs ih =>
    have e : fixtureOf (b :: bs) = box b.1 b.2 ∪ fixtureOf bs := by
      simp [fixtureOf, List.mem_cons]
    rw [e]
    refine (frontier_union_subset _ _).trans ?_
    rintro p (⟨hp, -⟩ | ⟨-, hp⟩)
    · exact mem_iUnion₂.mpr ⟨b, List.mem_cons_self, hp⟩
    · obtain ⟨b', hb', hp'⟩ := mem_iUnion₂.mp (ih hp)
      exact mem_iUnion₂.mpr ⟨b', List.mem_cons_of_mem _ hb', hp'⟩

/-- **The fixture's frontier lies in its boxes' prism faces**, indexed by (box, face). This is
`Certified`'s face hypothesis with μ = 0. -/
theorem frontier_fixture_faces (bs : List ((Fin 3 → ℝ) × (Fin 3 → ℝ))) :
    ∀ f ∈ frontier (fixtureOf bs), ∃ i : Fin bs.length × Fin 6,
      ∃ f' ∈ prism (boxFace (bs.get i.1).1 (bs.get i.1).2 i.2).1
        (boxFace (bs.get i.1).1 (bs.get i.1).2 i.2).2, dist f f' ≤ 0 := by
  intro f hf
  obtain ⟨b, hb, hfb⟩ := mem_iUnion₂.mp (frontier_fixtureOf bs hf)
  obtain ⟨n, rfl⟩ := List.get_of_mem hb
  obtain ⟨k, hk⟩ := mem_iUnion.mp (frontier_box_subset _ _ hfb)
  exact ⟨(n, k), f, hk, by simp⟩

/-- G2 with the open grown fixture: what forge-cam's `Clear` (lower ≥ fclear + μ) gives. -/
def G2open (T N : Set E3) (S F : ℝ → Set E3) (g : ℝ → E3 → E3) (c : ℝ) : Prop :=
  ∀ s ∈ I01, Disjoint (g s '' (T ∪ N)) (thickening c (F s)) ∧ Disjoint (g s '' N) (interior (S s))

theorem disjoint_thickening_of_clear {X A : Set E3} {c : ℝ}
    (h : ∀ x ∈ X, ∀ y ∈ A, c ≤ dist x y) : Disjoint X (thickening c A) :=
  Set.disjoint_left.mpr fun x hx hxA => by
    obtain ⟨y, hy, hd⟩ := mem_thickening_iff.mp hxA
    linarith [h x hx y hy]

/-- **G2 as forge-cam certifies it.** T and N each certified against the fixture at exactly the
clearance c > 0 (ops::certify_fixtured's `fixture_collision`), and N against the stock before
the piece at c₀ > 0: the whole tool misses the open c-neighbourhood of the fixture. -/
theorem G2open_of_closedForm {Zc Zn : Set ℝ} {wc wn : ℝ → ℝ} (hZc : Zc.OrdConnected)
    (hwc : ∀ z ∈ Zc, 0 ≤ wc z) (hZn : Zn.OrdConnected) (hwn : ∀ z ∈ Zn, 0 ≤ wn z)
    {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ} (hζ : HeightsIn ζ Zt)
    {c c₀ : ℝ} (hc : 0 < c) (hc₀ : 0 < c₀)
    {S₀ Fx : Set E3} {S F : ℝ → Set E3} (hS : ∀ s, S s ⊆ S₀) (hF : ∀ s, F s ⊆ Fx)
    (hTF : Certified Zc wc γ Zt Fx c) (hNF : Certified Zn wn γ Zt Fx c)
    (hNS : Certified Zn wn γ Zt S₀ c₀) :
    G2open (hyp Zc wc) (hyp Zn wn) S F (move γ ζ) c := by
  intro s hs
  have hT : Disjoint (move γ ζ s '' hyp Zc wc) (thickening c Fx) :=
    disjoint_thickening_of_clear (certified_clear hZc hwc hζ hc hTF s hs)
  have hN : Disjoint (move γ ζ s '' hyp Zn wn) (thickening c Fx) :=
    disjoint_thickening_of_clear (certified_clear hZn hwn hζ hc hNF s hs)
  have hNS' : Disjoint (move γ ζ s '' hyp Zn wn) S₀ :=
    disjoint_of_clear hc₀ (certified_clear hZn hwn hζ hc₀ hNS s hs)
  have hU : Disjoint (move γ ζ s '' (hyp Zc wc ∪ hyp Zn wn)) (thickening c Fx) := by
    rw [image_union]; exact hT.union_left hN
  have hU' : Disjoint (move γ ζ s '' (hyp Zc wc ∪ hyp Zn wn)) (thickening c (F s)) :=
    Disjoint.mono_right (thickening_subset_of_subset c (hF s)) hU
  have hS' : Disjoint (move γ ζ s '' hyp Zn wn) (interior (S s)) :=
    Disjoint.mono_right (interior_subset.trans (hS s)) hNS'
  exact ⟨hU', hS'⟩

/-- The vise's `Certified` reduces to the closed form's verdicts on its boxes' faces: closed,
frontier in prism faces (μ = 0 ≤ μ), and the top above it. -/
theorem certified_vise {Z : Set ℝ} {w : ℝ → ℝ} {γ : ℝ → ℝ × ℝ} {Zt : Set ℝ}
    (bs : List ((Fin 3 → ℝ) × (Fin 3 → ℝ))) {c μ : ℝ} (hμ : 0 ≤ μ)
    (D lower : Fin bs.length × Fin 6 → ℝ)
    (hv : ∀ i, ClearFace Z w γ Zt (boxFace (bs.get i.1).1 (bs.get i.1).2 i.2).1
      (boxFace (bs.get i.1).1 (bs.get i.1).2 i.2).2 (D i) (lower i))
    (hlow : ∀ i, c + μ ≤ lower i) (htop : TopAbove Z w Zt (fixtureOf bs)) :
    Certified Z w γ Zt (fixtureOf bs) c := by
  refine ⟨isClosed_fixtureOf bs, htop, Fin bs.length × Fin 6,
    ⟨fun i => (boxFace (bs.get i.1).1 (bs.get i.1).2 i.2).1,
     fun i => (boxFace (bs.get i.1).1 (bs.get i.1).2 i.2).2, D, lower⟩, μ, ?_, hv, hlow⟩
  intro f hf
  obtain ⟨i, f', hf', hd⟩ := frontier_fixture_faces bs f hf
  exact ⟨i, f', hf', hd.trans hμ⟩

/-! ### G4's held bands -/

/-- The clamped region: below the jaws' top. -/
def clamped (zj : ℝ) : Set E3 := {p | p 2 ≤ zj}

def band (z0 z1 : ℝ) : Set E3 := {p | z0 ≤ p 2 ∧ p 2 ≤ z1}

/-- **A band wholly within the grip is all "held by fixture".** -/
theorem held_band (Send P : Set E3) (r : ℝ) {zj z0 z1 : ℝ} (h : z1 ≤ zj) :
    leftover Send P r ∩ band z0 z1 = leftoverHeld Send P r (clamped zj) ∩ band z0 z1 := by
  ext p
  simp only [leftoverHeld, band, clamped, mem_inter_iff, mem_ofPred_eq]
  constructor
  · rintro ⟨hl, h0, h1⟩; exact ⟨⟨hl, h1.trans h⟩, h0, h1⟩
  · rintro ⟨⟨hl, -⟩, h0, h1⟩; exact ⟨hl, h0, h1⟩

/-- A band wholly above the grip is all "uncut". -/
theorem uncut_band (Send P : Set E3) (r : ℝ) {zj z0 z1 : ℝ} (h : zj < z0) :
    leftover Send P r ∩ band z0 z1 = leftoverUncut Send P r (clamped zj) ∩ band z0 z1 := by
  ext p
  simp only [leftoverUncut, band, clamped, mem_inter_iff, mem_sdiff, mem_ofPred_eq]
  constructor
  · rintro ⟨hl, h0, h1⟩; exact ⟨⟨hl, by linarith⟩, h0, h1⟩
  · rintro ⟨⟨hl, -⟩, h0, h1⟩; exact ⟨hl, h0, h1⟩

/-- A band that straddles the top splits there, into its held part and its uncut part. -/
theorem band_split (Send P : Set E3) (r : ℝ) {zj z0 z1 : ℝ} :
    leftover Send P r ∩ band z0 z1 =
      (leftoverHeld Send P r (clamped zj) ∩ band z0 z1) ∪
        (leftoverUncut Send P r (clamped zj) ∩ band z0 z1) := by
  rw [← union_inter_distrib_right, (G4_held Send P r (clamped zj)).2.1]

end ForgeCam
