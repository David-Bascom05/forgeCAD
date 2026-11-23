import ForgeCurved.QFields
import ForgeCurved.QComplete
import ForgeCurved.QFacetSampled

/-!
# `ExactK.edges` and `ExactK.verts`, in the sampling frame

In a frame that passes `frameOk` and `singAll`, where `samplesAt` succeeds, with a carrier `bi`
whose sublevel set is bounded, every listed facet on its negative side, and (U) (`vertOneAt`) on
every listed vertex, the curved ledger's `edges` and `verts` are exactly the realised edge and
vertex cells under a listed facet (`edges_frame`, `verts_frame`).

* a non-vertex sample with two zeros is an edge (`nonvert_edge`);
* an edge with a point off the events is such a sample (`twozero_sampled`); one without lies on
  its (U) triple (`onTriple`), so it is a vertex sample, unhit, hence listed, and (U) makes it a
  point;
* a vertex cell under a facet has no point off the events (`vert_no_off`), so it lies on its
  triple and is a vertex sample; no non-vertex sample hits it (`nonvert_point`).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem count_two : ∀ (X : Cov) {a b : ℕ}, a ≠ b → X[a]? = some Sgn.zero →
    X[b]? = some Sgn.zero → 2 ≤ X.count Sgn.zero
  | [], a, _, _, ha, _ => by simp at ha
  | s :: X, a, b, hab, ha, hb => by
    have hmem : ∀ {i}, X[i]? = some Sgn.zero → 1 ≤ X.count Sgn.zero := fun {i} hi => by
      obtain ⟨hl, he⟩ := List.getElem?_eq_some_iff.mp hi
      exact List.count_pos_iff.mpr (he ▸ List.getElem_mem hl)
    cases a with
    | zero =>
      cases b with
      | zero => exact absurd rfl hab
      | succ b =>
        simp only [List.getElem?_cons_zero, Option.some.injEq, List.getElem?_cons_succ] at ha hb
        subst ha; rw [List.count_cons_self]; have := hmem hb; omega
    | succ a =>
      cases b with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq, List.getElem?_cons_succ] at ha hb
        subst hb; rw [List.count_cons_self]; have := hmem ha; omega
      | succ b =>
        simp only [List.getElem?_cons_succ] at ha hb
        have := count_two X (by omega) ha hb
        rw [List.count_cons]; split <;> omega

theorem uTriple_some {qs : List Quad.Quadric} {X : Cov} (h : 2 ≤ X.count Sgn.zero) :
    ∃ t, uTriple qs X = some t := by
  obtain ⟨a, b, hab, ha, hb⟩ := two_idx X h
  have ha' := mem_zeroIdxs.mpr ha
  have hb' := mem_zeroIdxs.mpr hb
  unfold uTriple
  revert ha' hb'
  rcases zeroIdxs X with _ | ⟨i, _ | ⟨j, _ | ⟨k, r⟩⟩⟩
  · simp
  · simp; omega
  · intros; exact ⟨_, rfl⟩
  · intros; exact ⟨_, rfl⟩

theorem uTriple_two {qs : List Quad.Quadric} {X : Cov}
    {t : Quad.Quadric × Quad.Quadric × Quad.Quadric} (ht : uTriple qs X = some t) :
    ∃ a b : ℕ, a ≠ b ∧ X[a]? = some Sgn.zero ∧ X[b]? = some Sgn.zero := by
  have hs := zeroIdxs_sorted X
  unfold uTriple at ht
  split at ht
  · rename_i i j k rest hzs
    rw [hzs] at hs
    refine ⟨i, j, ?_, mem_zeroIdxs.mp (by rw [hzs]; simp), mem_zeroIdxs.mp (by rw [hzs]; simp)⟩
    simp at hs; omega
  · rename_i i j hzs
    rw [hzs] at hs
    refine ⟨i, j, ?_, mem_zeroIdxs.mp (by rw [hzs]; simp), mem_zeroIdxs.mp (by rw [hzs]; simp)⟩
    simp at hs; omega
  · cases ht

theorem cell_length {qs : List Quad.Quadric} {X : Cov} {p : R3} (hp : covCellK (qs.map qR) X p) :
    X.length = qs.length := by
  rw [← hp]; simp [covOfK]

/-- **A realised facet has a point off the events.** -/
theorem facet_off {qs : List Quad.Quadric} (hf : frameOk qs = true) (hsing : singAll qs = true)
    (E : List Num) {X : Cov} (hfac : X.isFacet = true) (hX : RealisedK (qs.map qR) X) :
    ∃ p, covCellK (qs.map qR) X p ∧ OffEvents E p := by
  obtain ⟨p, hp⟩ := hX
  have hXj := facet_zero hfac
  have hXo := facet_unique hfac
  set j := X.zeroIdx
  have hj : j < qs.length := by
    rw [← cell_length hp]; exact (List.getElem?_eq_some_iff.mp hXj).1
  obtain ⟨h0, hne⟩ := facet_signs hp hj hXj hXo
  have hg : gx qs[j] p ≠ 0 ∨ gy qs[j] p ≠ 0 ∨ gz qs[j] p ≠ 0 := by
    by_contra hn
    push Not at hn
    obtain ⟨m, hmj, r, hr, hr0⟩ := singAll_spec realModel hsing (j := j) (q := qs[j])
      (List.getElem?_eq_getElem hj) (p := p)
      ⟨by rw [dxK_real]; exact hn.1, by rw [dyK_real]; exact hn.2.1, by rw [dzK_real]; exact hn.2.2⟩
      (by rw [evK_real]; exact h0)
    have hm : m < qs.length := (List.getElem?_eq_some_iff.mp hr).1
    have : qs[m] = r := (List.getElem?_eq_some_iff.mp hr).2
    exact hne m hm hmj (by rw [this, ← evK_real]; exact hr0)
  obtain ⟨p1, h01, hs1, hoff1⟩ := facet_offX hf hj E h0 hne hg
  exact ⟨p1, cell_of_signs hp hj h01 hXj hs1, hoff1⟩

/-- **A vertex cell under a facet on `B`'s negative side has no point off the events.** -/
theorem vert_no_off {qs : List Quad.Quadric} {E : List Num} (hE : eventsOf qs rootFuel = some E)
    {bi : ℕ} (hbi : bi < (qs.map qR).length) (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0})
    {v φ : Cov} (hle : Cov.le v φ = true) (hφ : φ.isFacet = true) (hφb : φ[bi]? = some Sgn.neg)
    (hv : IsVertK (qs.map qR) v) : ∀ p, covCellK (qs.map qR) v p → ¬ OffEvents E p := by
  intro p hp hoff
  by_cases h1 : v.count Sgn.zero ≤ 1
  · have := le_facet_eq hle hφ h1
    rw [← this] at hφ hφb
    exact facet_not_point hE hbi hB v hφb hφ hp hoff hv
  · obtain ⟨a, b, hab, ha, hb⟩ := two_idx v (by omega)
    exact two_not_point hE hbi hB v (le_bi hle hφb) hab ha hb hp hoff hv

/-- **(U) makes a cell with no point off the events a point.** -/
theorem vert_of_one {qs : List Quad.Quadric} {E : List Num} {X : Cov}
    (hnoff : ∀ p, covCellK (qs.map qR) X p → ¬ OffEvents E p) (hU : vertOneAt qs X = true)
    (hr : RealisedK (qs.map qR) X) : IsVertK (qs.map qR) X := by
  obtain ⟨t, ht, huniq⟩ := vertOneAt_spec realModel hU
  have hlen : X.length = qs.length := by obtain ⟨p, hp⟩ := hr; exact cell_length hp
  refine ⟨hr, fun x y hx hy => ?_⟩
  obtain ⟨x1, x2, x3⟩ := onTriple hlen hnoff ht hx
  obtain ⟨y1, y2, y3⟩ := onTriple hlen hnoff ht hy
  exact huniq x y (by rw [evK_real]; exact x1) (by rw [evK_real]; exact x2)
    (by rw [evK_real]; exact x3) (by rw [evK_real]; exact y1) (by rw [evK_real]; exact y2)
    (by rw [evK_real]; exact y3) (by rw [cov_real]; exact hx) (by rw [cov_real]; exact hy)

/-- No non-vertex sample hits a cell with no point off the events. -/
theorem not_hit {qs : List Quad.Quadric} (hf : frameOk qs = true) {ss : List QS}
    (h : samplesAt qs = some ss) {E : List Num} (hE : eventsOf qs rootFuel = some E) {X : Cov}
    (hnoff : ∀ p, covCellK (qs.map qR) X p → ¬ OffEvents E p) :
    ¬ ∃ s ∈ ss, s.tag ≠ .vert ∧ s.cov = X := by
  rintro ⟨s, hs, htag, hcov⟩
  obtain ⟨-, -, q, -, hoff, hq⟩ := nonvert_point hf h hE hs htag
  exact hnoff q (by rw [← hcov]; exact hq) hoff

/-- A cell with two zeros and no point off the events is a vertex sample. -/
theorem vert_sample {qs : List Quad.Quadric} {ss : List QS} (h : samplesAt qs = some ss)
    {E : List Num} {X : Cov} (hnoff : ∀ p, covCellK (qs.map qR) X p → ¬ OffEvents E p)
    (h2 : 2 ≤ X.count Sgn.zero) (hr : RealisedK (qs.map qR) X) :
    ∃ s ∈ ss, s.tag = .vert ∧ s.cov = X := by
  obtain ⟨t, ht⟩ := uTriple_some (qs := qs) h2
  obtain ⟨p, hp⟩ := hr
  have hlen := cell_length hp
  obtain ⟨t1, t2, t3⟩ := onTriple hlen hnoff ht hp
  obtain ⟨s, hs, htag, hcov⟩ := vert_sampled h (uTriple_mem hlen ht) t1 t2 t3
  exact ⟨s, hs, htag, hcov.trans hp⟩

section Frame

variable {qs : List Quad.Quadric} (hf : frameOk qs = true) (hsing : singAll qs = true)
  {ss : List QS} (h : samplesAt qs = some ss) {bi : ℕ} (hbi : bi < (qs.map qR).length)
  (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0}) {forms : List SForm}
  (hφB : ∀ φ ∈ (curvedLedger qs.length forms ss).facets,
    φ[bi]? = some Sgn.neg ∧ φ.isFacet = true)
  (hU : ∀ v ∈ (curvedLedger qs.length forms ss).verts, vertOneAt qs v = true)
include hf hsing h hbi hB hφB hU

/-- **`ExactK.verts`, in the sampling frame.** -/
theorem verts_frame (v : Cov) :
    v ∈ (curvedLedger qs.length forms ss).verts ↔
      RealisedK (qs.map qR) v ∧ IsVertK (qs.map qR) v ∧
        ∃ φ ∈ (curvedLedger qs.length forms ss).facets, Cov.le v φ = true := by
  obtain ⟨E, hE, -⟩ := samplesAt_parts h
  constructor
  · intro hv
    obtain ⟨⟨s, hs, -, hcov⟩, hnot, φ, hφ, hle⟩ := mem_verts.mp hv
    obtain ⟨hφb, -⟩ := hφB φ hφ
    have hreal : RealisedK (qs.map qR) v := by
      obtain ⟨p, hp⟩ := samplesAt_realised realModel h s hs
      rw [cov_real] at hp
      exact ⟨p, hp.symm.trans hcov⟩
    have hlen : v.length = qs.length := by obtain ⟨p, hp⟩ := hreal; exact cell_length hp
    obtain ⟨t, ht, -⟩ := vertOneAt_spec realModel (hU v hv)
    obtain ⟨a, b, hab, ha, hb⟩ := uTriple_two ht
    have hnoff : ∀ p, covCellK (qs.map qR) v p → ¬ OffEvents E p := by
      intro p hp hoff
      have ha' : a < qs.length := hlen ▸ (List.getElem?_eq_some_iff.mp ha).1
      have hb' : b < qs.length := hlen ▸ (List.getElem?_eq_some_iff.mp hb).1
      exact hnot (twozero_sampled hf h hE hbi hB v (le_bi hle hφb) ha' hb' hab ha hb hp hoff)
    exact ⟨hreal, vert_of_one hnoff (hU v hv) hreal, φ, hφ, hle⟩
  · rintro ⟨hreal, hvert, φ, hφ, hle⟩
    obtain ⟨hφb, hφf⟩ := hφB φ hφ
    have hnoff := vert_no_off hE hbi hB hle hφf hφb hvert
    have h2 : 2 ≤ v.count Sgn.zero := by
      by_contra h1
      have := le_facet_eq hle hφf (by omega)
      rw [← this] at hφf
      obtain ⟨p, hp, hoff⟩ := facet_off hf hsing E hφf hreal
      exact hnoff p hp hoff
    exact mem_verts.mpr ⟨vert_sample h hnoff h2 hreal, not_hit hf h hE hnoff, φ, hφ, hle⟩

/-- **`ExactK.edges`, in the sampling frame.** -/
theorem edges_frame (e : Cov) :
    e ∈ (curvedLedger qs.length forms ss).edges ↔
      RealisedK (qs.map qR) e ∧ IsEdgeK (qs.map qR) e ∧
        ∃ φ ∈ (curvedLedger qs.length forms ss).facets, Cov.le e φ = true := by
  obtain ⟨E, hE, -⟩ := samplesAt_parts h
  constructor
  · intro he
    obtain ⟨⟨s, hs, htag, h2, rfl⟩, φ, hφ, hle⟩ := mem_edges.mp he
    obtain ⟨hφb, -⟩ := hφB φ hφ
    obtain ⟨hr, hnv⟩ := nonvert_edge hf h hE hbi hB hs htag h2 (le_bi hle hφb)
    exact ⟨hr, ⟨hr, two_idx _ h2, hnv⟩, φ, hφ, hle⟩
  · rintro ⟨hreal, ⟨-, ⟨a, b, hab, ha, hb⟩, hnv⟩, φ, hφ, hle⟩
    obtain ⟨hφb, -⟩ := hφB φ hφ
    have hlen : e.length = qs.length := by obtain ⟨p, hp⟩ := hreal; exact cell_length hp
    have h2 := count_two e hab ha hb
    refine mem_edges.mpr ⟨?_, φ, hφ, hle⟩
    by_cases hex : ∃ p, covCellK (qs.map qR) e p ∧ OffEvents E p
    · obtain ⟨p, hp, hoff⟩ := hex
      have ha' : a < qs.length := hlen ▸ (List.getElem?_eq_some_iff.mp ha).1
      have hb' : b < qs.length := hlen ▸ (List.getElem?_eq_some_iff.mp hb).1
      obtain ⟨s, hs, htag, hcov⟩ :=
        twozero_sampled hf h hE hbi hB e (le_bi hle hφb) ha' hb' hab ha hb hp hoff
      exact ⟨s, hs, htag, by rw [hcov]; exact h2, hcov⟩
    · exfalso
      have hnoff : ∀ p, covCellK (qs.map qR) e p → ¬ OffEvents E p :=
        fun p hp hoff => hex ⟨p, hp, hoff⟩
      have hv : e ∈ (curvedLedger qs.length forms ss).verts :=
        mem_verts.mpr ⟨vert_sample h hnoff h2 hreal, not_hit hf h hE hnoff, φ, hφ, hle⟩
      exact hnv (vert_of_one hnoff (hU e hv) hreal)

end Frame

end ForgeCurved
