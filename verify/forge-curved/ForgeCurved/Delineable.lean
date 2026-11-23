import Mathlib.Analysis.Calculus.Deriv.MeanValue
import Mathlib.Analysis.Calculus.Deriv.Mul
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.Connected.TotallyDisconnected
import Mathlib.Topology.Compactness.Compact
import Mathlib.Topology.MetricSpace.ProperSpace
import Mathlib.Topology.Instances.Real.Lemmas
import Mathlib.Data.Finset.Sort

/-!
# Delineability: simple zeros of a continuous family are ordered continuous graphs

This is the geometric input of dol-curved's `qsampler_covers` (curved-l2-plan.md, module 5):
between the critical values of the sweep, the zeros of the restricted carriers are graphs.

The family is `f t y`, jointly continuous, with `∂f/∂y = f' t y` jointly continuous. On an open
preconnected parameter set `I` (an open interval between consecutive critical values):
* every zero is simple, `f t y = 0 → f' t y ≠ 0` (no critical point of the sweep);
* the zeros are locally bounded in `t` (no zero escapes to infinity; for a polynomial in `y`
  this is a leading coefficient that does not vanish on `I`).

**`delineable`**: then there are `n` and continuous `r 0 < r 1 < … < r (n-1)` on `I` whose
graphs are exactly the zero set over `I`. In particular the number of zeros is constant on `I`.

The proof uses no implicit function theorem: near a simple zero, `f t` is strictly monotone
uniformly in `t` (`local_pos`, from the mean value theorem). The intermediate value theorem gives
one zero in a small window, and the tube lemma excludes zeros off the windows (`local_graphs`).
The sorted zeros are then continuous (`Finset.orderEmbOfFin_unique`), and their number is
locally constant, so constant on `I` (`IsPreconnected.constant`).
-/

namespace ForgeCurved.Delineable

open Filter Topology Set

/-- A `C¹`-in-`y` family: `f` and `∂f/∂y = f'` jointly continuous. -/
structure Smooth (f f' : ℝ → ℝ → ℝ) : Prop where
  cont : Continuous (Function.uncurry f)
  cont' : Continuous (Function.uncurry f')
  deriv : ∀ t y, HasDerivAt (f t) (f' t y) y

variable {f f' : ℝ → ℝ → ℝ}

theorem Smooth.neg (hs : Smooth f f') : Smooth (fun t y => -f t y) (fun t y => -f' t y) :=
  ⟨hs.cont.neg, hs.cont'.neg, fun t y => (hs.deriv t y).neg⟩

theorem Smooth.cont_t (hs : Smooth f f') (y : ℝ) : Continuous fun t => f t y :=
  hs.cont.comp (continuous_id.prodMk continuous_const)

/-- Near a zero with positive slope: strictly increasing on one window for all nearby `t`, and a
zero in every smaller window for `t` near enough. -/
theorem local_pos (hs : Smooth f f') {t₀ y₀ : ℝ} (h0 : f t₀ y₀ = 0) (hp : 0 < f' t₀ y₀) :
    ∃ ε₀ > 0, (∀ᶠ t in 𝓝 t₀, StrictMonoOn (f t) (Ioo (y₀ - ε₀) (y₀ + ε₀))) ∧
      ∀ ε, 0 < ε → ε ≤ ε₀ → ∀ᶠ t in 𝓝 t₀, ∃ y ∈ Ioo (y₀ - ε) (y₀ + ε), f t y = 0 := by
  have hev : ∀ᶠ z in 𝓝 (t₀, y₀), 0 < Function.uncurry f' z :=
    (hs.cont'.tendsto (t₀, y₀)).eventually (lt_mem_nhds hp)
  obtain ⟨η, hη, hball⟩ := Metric.eventually_nhds_iff.mp hev
  have hmono : ∀ t, dist t t₀ < η → StrictMonoOn (f t) (Ioo (y₀ - η) (y₀ + η)) := by
    intro t ht
    apply strictMonoOn_of_deriv_pos (convex_Ioo _ _)
    · exact fun y _ => (hs.deriv t y).continuousAt.continuousWithinAt
    · intro y hy
      rw [interior_Ioo] at hy
      rw [(hs.deriv t y).deriv]
      refine @hball (t, y) ?_
      rw [Prod.dist_eq, max_lt_iff, Real.dist_eq y y₀, abs_sub_lt_iff]
      exact ⟨ht, by constructor <;> linarith [hy.1, hy.2]⟩
  refine ⟨η, hη, Metric.eventually_nhds_iff.mpr ⟨η, hη, fun t ht => hmono t ht⟩, ?_⟩
  intro ε hε hεη
  have hm0 := hmono t₀ (by simpa using hη)
  have ha : f t₀ (y₀ - ε / 2) < 0 := by
    rw [← h0]; exact hm0 ⟨by linarith, by linarith⟩ ⟨by linarith, by linarith⟩ (by linarith)
  have hb : 0 < f t₀ (y₀ + ε / 2) := by
    rw [← h0]; exact hm0 ⟨by linarith, by linarith⟩ ⟨by linarith, by linarith⟩ (by linarith)
  have ha' := ((hs.cont_t (y₀ - ε / 2)).tendsto t₀).eventually (gt_mem_nhds ha)
  have hb' := ((hs.cont_t (y₀ + ε / 2)).tendsto t₀).eventually (lt_mem_nhds hb)
  filter_upwards [ha', hb'] with t hta htb
  have hivt := intermediate_value_Ioo (show y₀ - ε / 2 ≤ y₀ + ε / 2 by linarith)
    (fun y _ => (hs.deriv t y).continuousAt.continuousWithinAt)
  obtain ⟨y, hy, hy0⟩ := hivt ⟨hta, htb⟩
  exact ⟨y, ⟨by linarith [hy.1], by linarith [hy.2]⟩, hy0⟩

/-- Near a simple zero: injective on one window for all nearby `t`, and a zero in every smaller
window for `t` near enough. -/
theorem local_ne (hs : Smooth f f') {t₀ y₀ : ℝ} (h0 : f t₀ y₀ = 0) (hne : f' t₀ y₀ ≠ 0) :
    ∃ ε₀ > 0, (∀ᶠ t in 𝓝 t₀, InjOn (f t) (Ioo (y₀ - ε₀) (y₀ + ε₀))) ∧
      ∀ ε, 0 < ε → ε ≤ ε₀ → ∀ᶠ t in 𝓝 t₀, ∃ y ∈ Ioo (y₀ - ε) (y₀ + ε), f t y = 0 := by
  rcases hne.lt_or_gt with hn | hp
  · obtain ⟨ε₀, hε₀, hm, hex⟩ := local_pos hs.neg (t₀ := t₀) (y₀ := y₀) (by simp [h0]) (by simpa using hn)
    refine ⟨ε₀, hε₀, hm.mono fun t ht => ?_, fun ε hε hεe => (hex ε hε hεe).mono fun t ⟨y, hy, h⟩ =>
      ⟨y, hy, by simpa using h⟩⟩
    intro a ha b hb hab
    exact ht.injOn ha hb (by simp [hab])
  · obtain ⟨ε₀, hε₀, hm, hex⟩ := local_pos hs h0 hp
    exact ⟨ε₀, hε₀, hm.mono fun t ht => ht.injOn, hex⟩

theorem isClosed_zeros (hs : Smooth f f') (t : ℝ) : IsClosed {y | f t y = 0} :=
  isClosed_eq (hs.cont.comp (continuous_const.prodMk continuous_id)) continuous_const

/-- A bounded fibre of simple zeros is finite. -/
theorem finite_zeros (hs : Smooth f f') {t R : ℝ} (hsimple : ∀ y, f t y = 0 → f' t y ≠ 0)
    (hR : ∀ y, f t y = 0 → |y| ≤ R) : {y | f t y = 0}.Finite := by
  by_contra hinf
  obtain ⟨x, -, hacc⟩ := Set.Infinite.exists_accPt_of_subset_isCompact hinf
    (isCompact_Icc : IsCompact (Icc (-R) R)) (fun y hy => abs_le.mp (hR y hy))
  have hx : f t x = 0 := by
    have : x ∈ closure {y | f t y = 0} := mem_closure_iff_clusterPt.mpr hacc.clusterPt
    rwa [(isClosed_zeros hs t).closure_eq] at this
  obtain ⟨ε₀, hε₀, hinj, -⟩ := local_ne hs hx (hsimple x hx)
  have hfr := accPt_iff_frequently.mp hacc
  have hwin : ∀ᶠ y in 𝓝 x, y ∈ Ioo (x - ε₀) (x + ε₀) :=
    isOpen_Ioo.mem_nhds ⟨by linarith, by linarith⟩
  obtain ⟨y, ⟨hyx, hy0⟩, hyw⟩ := (hfr.and_eventually hwin).exists
  exact hyx (hinj.self_of_nhds hyw ⟨by linarith, by linarith⟩ (hy0.trans hx.symm))

/-- **The local picture**: if the zeros at `t₀` are `y 0 < … < y (n-1)`, all simple, then for `t`
near `t₀` the zeros are `g 0 < … < g (n-1)` with each `g j` within `ε` of `y j`. -/
theorem local_graphs (hs : Smooth f f') {t₀ : ℝ} {n : ℕ} {y : Fin n → ℝ} (hy : StrictMono y)
    (hZ : ∀ z, f t₀ z = 0 ↔ ∃ j, y j = z) (hsimple : ∀ j, f' t₀ (y j) ≠ 0)
    (hb : ∃ R, ∀ᶠ t in 𝓝 t₀, ∀ z, f t z = 0 → |z| ≤ R) (ε : ℝ) (hε : 0 < ε) :
    ∀ᶠ t in 𝓝 t₀, ∃ g : Fin n → ℝ, StrictMono g ∧ (∀ z, f t z = 0 ↔ ∃ j, g j = z) ∧
      ∀ j, |g j - y j| < ε := by
  choose ε₀ hε₀ hinj hex using fun j => local_ne hs ((hZ _).2 ⟨j, rfl⟩) (hsimple j)
  -- a window width below `ε`, every `ε₀ j`, and half of every gap
  have hev : ∀ᶠ e in 𝓝[>] (0 : ℝ), 0 < e ∧ e ≤ ε ∧ (∀ j, e ≤ ε₀ j) ∧
      ∀ i j, i < j → y i + e < y j - e := by
    have h1 : ∀ᶠ e in 𝓝[>] (0 : ℝ), 0 < e := self_mem_nhdsWithin
    have h2 : ∀ᶠ e in 𝓝[>] (0 : ℝ), e ≤ ε :=
      nhdsWithin_le_nhds ((gt_mem_nhds hε).mono fun e h => h.le)
    have h3 : ∀ᶠ e in 𝓝[>] (0 : ℝ), ∀ j, e ≤ ε₀ j :=
      nhdsWithin_le_nhds (eventually_all.2 fun j => (gt_mem_nhds (hε₀ j)).mono fun e h => h.le)
    have h4 : ∀ᶠ e in 𝓝[>] (0 : ℝ), ∀ i j, i < j → y i + e < y j - e := by
      refine nhdsWithin_le_nhds (eventually_all.2 fun i => eventually_all.2 fun j => ?_)
      by_cases hij : i < j
      · have hg : 0 < (y j - y i) / 2 := by linarith [hy hij]
        exact (gt_mem_nhds hg).mono fun e h _ => by linarith
      · exact Eventually.of_forall fun e h => absurd h hij
    exact h1.and (h2.and (h3.and h4))
  obtain ⟨e, he0, heε, heε₀, hsep⟩ := hev.exists
  obtain ⟨R, hR⟩ := hb
  set K := Icc (-R) R ∩ (⋃ j, Ioo (y j - e) (y j + e))ᶜ with hKdef
  have hKc : IsCompact K :=
    isCompact_Icc.inter_right (isOpen_iUnion fun _ => isOpen_Ioo).isClosed_compl
  have hK : ∀ᶠ t in 𝓝 t₀, ∀ z ∈ K, f t z ≠ 0 := by
    refine hKc.eventually_forall_of_forall_eventually fun z hz => ?_
    have hne : Function.uncurry f (t₀, z) ≠ 0 := by
      intro h0
      obtain ⟨j, rfl⟩ := (hZ z).1 h0
      exact hz.2 (mem_iUnion.2 ⟨j, by constructor <;> linarith⟩)
    exact hs.cont.continuousAt.eventually_ne hne
  have hE : ∀ᶠ t in 𝓝 t₀, ∀ j, ∃ z ∈ Ioo (y j - e) (y j + e), f t z = 0 :=
    eventually_all.2 fun j => hex j e he0 (heε₀ j)
  have hI : ∀ᶠ t in 𝓝 t₀, ∀ j, InjOn (f t) (Ioo (y j - ε₀ j) (y j + ε₀ j)) :=
    eventually_all.2 hinj
  filter_upwards [hR, hK, hE, hI] with t hRt hKt hEt hIt
  choose g hgw hg0 using hEt
  refine ⟨g, fun i j hij => ?_, fun z => ⟨fun hz => ?_, ?_⟩, fun j => ?_⟩
  · have := hsep i j hij
    linarith [(hgw i).2, (hgw j).1]
  · have hzR := abs_le.mp (hRt z hz)
    have hzK : z ∉ K := fun h => hKt z h hz
    have hzU : z ∈ ⋃ j, Ioo (y j - e) (y j + e) := by
      by_contra h
      exact hzK ⟨⟨hzR.1, hzR.2⟩, h⟩
    obtain ⟨j, hj⟩ := mem_iUnion.1 hzU
    have hej := heε₀ j
    refine ⟨j, hIt j ⟨by linarith [(hgw j).1], by linarith [(hgw j).2]⟩
      ⟨by linarith [hj.1], by linarith [hj.2]⟩ ((hg0 j).trans hz.symm)⟩
  · rintro ⟨j, rfl⟩
    exact hg0 j
  · rw [abs_sub_lt_iff]
    constructor <;> linarith [(hgw j).1, (hgw j).2]

/-- **Delineability.** On an open preconnected `I` where every zero is simple and the zeros are
locally bounded, the zero set over `I` is the union of the graphs of `n` continuous functions,
strictly ordered at every `t ∈ I`. -/
theorem delineable (hs : Smooth f f') {I : Set ℝ} (hIo : IsOpen I) (hIc : IsPreconnected I)
    (hsimple : ∀ t ∈ I, ∀ y, f t y = 0 → f' t y ≠ 0)
    (hb : ∀ t ∈ I, ∃ R, ∀ᶠ s in 𝓝 t, ∀ y, f s y = 0 → |y| ≤ R) :
    ∃ n, ∃ r : Fin n → ℝ → ℝ, (∀ i, ContinuousOn (r i) I) ∧ (∀ t ∈ I, StrictMono fun i => r i t) ∧
      ∀ t ∈ I, ∀ y, f t y = 0 ↔ ∃ i, r i t = y := by
  classical
  rcases I.eq_empty_or_nonempty with hI0 | ⟨t₁, ht₁⟩
  · exact ⟨0, fun i => i.elim0, fun i => i.elim0, by simp [hI0], by simp [hI0]⟩
  have hfin : ∀ t ∈ I, {y | f t y = 0}.Finite := fun t ht => by
    obtain ⟨R, hR⟩ := hb t ht
    exact finite_zeros hs (hsimple t ht) hR.self_of_nhds
  let Z : ℝ → Finset ℝ := fun t => if h : {y | f t y = 0}.Finite then h.toFinset else ∅
  have hZ : ∀ t ∈ I, ∀ y, f t y = 0 ↔ y ∈ Z t := fun t ht y => by
    simp [Z, hfin t ht]
  -- the enumeration at `t` of its own zeros
  have henum : ∀ t ∈ I, ∀ {k} (h : (Z t).card = k), ∀ z,
      f t z = 0 ↔ ∃ j, (Z t).orderEmbOfFin h j = z := fun t ht k h z => by
    rw [hZ t ht, ← Finset.mem_coe, ← Finset.range_orderEmbOfFin _ h, mem_range]
  -- near `t₀`, the zeros are an ordered family of the same size, close to `t₀`'s
  have hnear : ∀ t₀ ∈ I, ∀ {k} (h : (Z t₀).card = k), ∀ ε > 0, ∀ᶠ t in 𝓝 t₀,
      ∃ hk : (Z t).card = k, ∀ j, |(Z t).orderEmbOfFin hk j - (Z t₀).orderEmbOfFin h j| < ε := by
    intro t₀ ht₀ k h ε hε
    obtain ⟨R, hR⟩ := hb t₀ ht₀
    have hloc := local_graphs hs ((Z t₀).orderEmbOfFin h).strictMono (henum t₀ ht₀ h)
      (fun j => hsimple t₀ ht₀ _ ((henum t₀ ht₀ h _).2 ⟨j, rfl⟩)) ⟨R, hR⟩ ε hε
    filter_upwards [hloc, hIo.mem_nhds ht₀] with t ⟨g, hg, hgZ, hgc⟩ ht
    have hcard : (Z t).card = k := by
      have : Z t = Finset.univ.image g := by
        ext z; rw [← hZ t ht, hgZ, Finset.mem_image]; simp
      rw [this, Finset.card_image_of_injective _ hg.injective, Finset.card_univ, Fintype.card_fin]
    refine ⟨hcard, fun j => ?_⟩
    have hu := Finset.orderEmbOfFin_unique hcard (f := g)
      (fun x => (hZ t ht _).1 ((hgZ _).2 ⟨x, rfl⟩)) hg
    rw [← hu]
    exact hgc j
  -- the number of zeros is locally constant, so constant on `I`
  set n := (Z t₁).card
  have hconst : ∀ t ∈ I, (Z t).card = n := by
    have hcont : ContinuousOn (fun t => (Z t).card) I := by
      intro t₀ ht₀
      refine ((continuousAt_const (y := (Z t₀).card)).congr ?_).continuousWithinAt
      filter_upwards [hnear t₀ ht₀ rfl 1 one_pos] with t ⟨hk, _⟩
      exact hk.symm
    exact fun t ht => hIc.constant hcont ht ht₁
  let r : Fin n → ℝ → ℝ := fun i t =>
    if h : (Z t).card = n then (Z t).orderEmbOfFin h i else 0
  have hr : ∀ t (ht : t ∈ I) (i : Fin n), r i t = (Z t).orderEmbOfFin (hconst t ht) i := fun t ht i => by
    simp [r, hconst t ht]
  refine ⟨n, r, fun i t₀ ht₀ => ?_, fun t ht a b hab => ?_, fun t ht y => ?_⟩
  · refine ContinuousAt.continuousWithinAt (Metric.tendsto_nhds.mpr fun ε hε => ?_)
    filter_upwards [hnear t₀ ht₀ (hconst t₀ ht₀) ε hε, hIo.mem_nhds ht₀] with t ⟨hk, hc⟩ ht
    rw [Real.dist_eq, hr t ht, hr t₀ ht₀]
    exact hc i
  · simp only [hr t ht]
    exact (Finset.orderEmbOfFin _ _).strictMono hab
  · simp only [hr t ht]
    exact henum t ht _ y

end ForgeCurved.Delineable
