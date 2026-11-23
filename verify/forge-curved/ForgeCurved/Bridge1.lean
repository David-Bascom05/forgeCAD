import ForgeCurved.FaceLocal
import ForgeExact.DecideOnce.Closure

/-!
# Route B, bridge (1): the core's touch rule is `XNonManifoldF`

The A8 core (`IncLedger.coreI`, IncCore.lean on `decide-once-lean`) refuses ZeroThickness, past its
two checks, iff (`coreI_zeroThickness_iff`, Simulation.lean @ 983ebcc):
* some edge has more than two **result wings**: `2 < (edgeFacets e).filter (∈ result)).length`; or
* some vert has two **result sectors** `c, d` with `¬Conn fanRel (aroundV v) c d`.

FaceLocal's `XNonManifoldF` says the same over positions: three distinct positions of the edge row
on boundary facets (`BadEdgeW`), or two boundary sectors not joined by `ReflTransGen FanRel`
(`BadVertS`). The encoding (positions in the wing-filtered list; `Conn ↔ ReflTransGen`):

* **`badEdgeW_iff`**: `BadEdgeW G keep l ↔ 2 < (l.filter p).length` when `Bd` is `p` on the row;
* **`badVertS_iff`**: `BadVertS G keep (ss.map enc) ↔ ∃ c d` result sectors not `Conn`, for a
  nodup sector list.

Both are stated over plain lists (the sector type is any `S` with `facet` and `bounds`), so this
branch does not need IncCore. With `S := Sector Nat`, `p := (result kt).contains` and
`keep := keepInI kt`, they turn `coreI_zeroThickness_iff`'s right side into `XNonManifoldF`; with
`xNonManifoldF_iff_top`, refusal is the result's topological non-manifoldness.
`result_contains_iff` is the filter fact behind `Bd ↔ p`.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.FaceLocal

open ForgeExact.DecideOnce

/-! ## Positions in a filtered list -/

theorem length_filter_eq_card (p : Nat → Bool) :
    ∀ l : List Nat, (l.filter p).length =
      ((Finset.range l.length).filter fun i => p (l.getD i 0) = true).card
  | [] => by simp
  | a :: t => by
    have ih := length_filter_eq_card p t
    rw [List.length_cons, Finset.range_add_one', Finset.filter_insert, Finset.filter_map]
    simp only [Function.comp_def, List.getD_cons_zero, List.filter_cons]
    by_cases ha : p a = true
    · rw [if_pos ha, if_pos ha, List.length_cons, Finset.card_insert_of_notMem (by simp),
        Finset.card_map, ih]
      rfl
    · rw [if_neg ha, if_neg ha, Finset.card_map, ih]
      rfl

theorem two_lt_filter_iff (l : List Nat) (p : Nat → Bool) :
    2 < (l.filter p).length ↔ ∃ i j k, i < l.length ∧ j < l.length ∧ k < l.length ∧
      i ≠ j ∧ i ≠ k ∧ j ≠ k ∧ p (l.getD i 0) = true ∧ p (l.getD j 0) = true ∧
        p (l.getD k 0) = true := by
  rw [length_filter_eq_card, Finset.two_lt_card_iff]
  simp only [Finset.mem_filter, Finset.mem_range]
  constructor
  · rintro ⟨i, j, k, ⟨hi, pi⟩, ⟨hj, pj⟩, ⟨hk, pk⟩, hij, hik, hjk⟩
    exact ⟨i, j, k, hi, hj, hk, hij, hik, hjk, pi, pj, pk⟩
  · rintro ⟨i, j, k, hi, hj, hk, hij, hik, hjk, pi, pj, pk⟩
    exact ⟨i, j, k, ⟨hi, pi⟩, ⟨hj, pj⟩, ⟨hk, pk⟩, hij, hik, hjk⟩

variable {P : Type} [TopologicalSpace P] {G : FaceGeo P}

/-- **Edges**: three boundary wings at positions of the row iff more than two result wings. -/
theorem badEdgeW_iff {keep : Nat → Bool} {l : List Nat} {p : Nat → Bool}
    (hp : ∀ φ ∈ l, (Bd G keep φ ↔ p φ = true)) :
    BadEdgeW G keep l ↔ 2 < (l.filter p).length := by
  rw [two_lt_filter_iff]
  have hg : ∀ i, i < l.length → (Bd G keep (l.getD i 0) ↔ p (l.getD i 0) = true) := fun i hi =>
    hp _ (by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, Option.getD_some]; exact List.getElem_mem hi)
  constructor
  · rintro ⟨i, j, k, hi, hj, hk, hij, hik, hjk, bi, bj, bk⟩
    exact ⟨i, j, k, hi, hj, hk, hij, hik, hjk, (hg i hi).mp bi, (hg j hj).mp bj, (hg k hk).mp bk⟩
  · rintro ⟨i, j, k, hi, hj, hk, hij, hik, hjk, bi, bj, bk⟩
    exact ⟨i, j, k, hi, hj, hk, hij, hik, hjk, (hg i hi).mpr bi, (hg j hj).mpr bj, (hg k hk).mpr bk⟩

/-- The result facets: a filter of the facets. -/
theorem result_contains_iff (F : List Nat) (q : Nat → Bool) (φ : Nat) :
    (F.filter q).contains φ = true ↔ φ ∈ F ∧ q φ = true := by
  simp [List.mem_filter]

/-! ## Verts: positions of sectors, `Conn ↔ ReflTransGen` -/

section Vert

variable {S : Type} [DecidableEq S] (facet : S → Nat) (bounds : S → List Nat)

/-- A sector, as FaceLocal's `Sec`: its facet and its bounding edges. -/
def enc (s : S) : Sec := (facet s, bounds s)

/-- The core's `fanRel`: two sectors sharing a bounding edge. -/
def fanRelS (s t : S) : Bool := (bounds s).any fun e => (bounds t).contains e

variable {facet bounds}

theorem bounds_enc (s : S) (a : Nat) : Sec.bounds (enc facet bounds s) a ↔ a ∈ bounds s :=
  Iff.rfl

theorem getD_map_enc {ss : List S} {i : Nat} (hi : i < ss.length) :
    (ss.map (enc facet bounds)).getD i (0, []) = enc facet bounds ss[i] := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_eq_getElem hi]; rfl

theorem fanRelS_iff (s t : S) :
    fanRelS bounds s t = true ↔
      ∃ a, Sec.bounds (enc facet bounds s) a ∧ Sec.bounds (enc facet bounds t) a := by
  simp only [fanRelS, List.any_eq_true, List.contains_iff_mem, bounds_enc]

/-- **Verts**: two boundary sectors in different fan classes iff two result sectors not `Conn`. -/
theorem badVertS_iff {keep : Nat → Bool} {p : Nat → Bool} {ss : List S} (hnd : ss.Nodup)
    (hp : ∀ s ∈ ss, (Bd G keep (facet s) ↔ p (facet s) = true)) :
    BadVertS G keep (ss.map (enc facet bounds)) ↔
      ∃ c ∈ ss.filter (fun s => p (facet s)), ∃ d ∈ ss.filter (fun s => p (facet s)),
        ¬Conn (fanRelS bounds) (ss.filter fun s => p (facet s)) c d := by
  set A := ss.filter fun s => p (facet s)
  have hlen : (ss.map (enc facet bounds)).length = ss.length := List.length_map _
  have hBd : ∀ i (hi : i < ss.length), (Bd G keep ((ss.map (enc facet bounds)).getD i (0, [])).1 ↔
      ss[i] ∈ A) := by
    intro i hi
    rw [getD_map_enc hi]
    simp only [enc, A, List.mem_filter, List.getElem_mem, true_and]
    exact hp _ (List.getElem_mem hi)
  -- the positions' relation, read off
  have hfan : ∀ i j, FanRel G keep (ss.map (enc facet bounds)) i j ↔ ∃ (hi : i < ss.length)
      (hj : j < ss.length), ss[i] ∈ A ∧ ss[j] ∈ A ∧ fanRelS bounds ss[i] ss[j] = true := by
    intro i j
    unfold FanRel
    constructor
    · rintro ⟨hi, hj, bi, bj, hab⟩
      rw [hlen] at hi hj
      refine ⟨hi, hj, (hBd i hi).mp bi, (hBd j hj).mp bj, ?_⟩
      rw [fanRelS_iff]
      rw [getD_map_enc hi, getD_map_enc hj] at hab
      exact hab
    · rintro ⟨hi, hj, ai, aj, hr⟩
      refine ⟨hlen ▸ hi, hlen ▸ hj, (hBd i hi).mpr ai, (hBd j hj).mpr aj, ?_⟩
      rw [getD_map_enc hi, getD_map_enc hj]
      exact (fanRelS_iff _ _).mp hr
  -- ReflTransGen on positions gives Conn on sectors
  have h1 : ∀ i j (hi : i < ss.length), ss[i] ∈ A → Relation.ReflTransGen (FanRel G keep
      (ss.map (enc facet bounds))) i j → ∃ hj : j < ss.length, Conn (fanRelS bounds) A ss[i] ss[j] := by
    intro i j hi _ h
    induction h with
    | refl => exact ⟨hi, Conn.refl⟩
    | tail _ hkj ih =>
      obtain ⟨hk, hj, _, aj, hr⟩ := (hfan _ _).mp hkj
      obtain ⟨_, hc⟩ := ih
      exact ⟨hj, Conn.step hc aj hr⟩
  -- Conn on sectors gives ReflTransGen on positions
  have h2 : ∀ i (hi : i < ss.length), ss[i] ∈ A → ∀ b, Conn (fanRelS bounds) A ss[i] b →
      ∃ j, ∃ hj : j < ss.length, ss[j] = b ∧ ss[j] ∈ A ∧
        Relation.ReflTransGen (FanRel G keep (ss.map (enc facet bounds))) i j := by
    intro i hi ai b h
    induction h with
    | refl => exact ⟨i, hi, rfl, ai, Relation.ReflTransGen.refl⟩
    | @step b c _ hc hr ih =>
      obtain ⟨j, hj, rfl, aj, hrt⟩ := ih
      have hcs : c ∈ ss := (List.mem_filter.mp hc).1
      obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hcs
      exact ⟨k, hk, rfl, hc, hrt.tail ((hfan _ _).mpr ⟨hj, hk, aj, hc, hr⟩)⟩
  constructor
  · rintro ⟨i, j, hi, hj, bi, bj, hn⟩
    rw [hlen] at hi hj
    refine ⟨ss[i], (hBd i hi).mp bi, ss[j], (hBd j hj).mp bj, fun hc => hn ?_⟩
    obtain ⟨j', hj', e, -, hrt⟩ := h2 i hi ((hBd i hi).mp bi) _ hc
    rw [(hnd.getElem_inj_iff).mp e] at hrt
    exact hrt
  · rintro ⟨c, hc, d, hd, hn⟩
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem (List.mem_filter.mp hc).1
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem (List.mem_filter.mp hd).1
    refine ⟨i, j, hlen ▸ hi, hlen ▸ hj, (hBd i hi).mpr hc, (hBd j hj).mpr hd, fun hrt => hn ?_⟩
    obtain ⟨_, h⟩ := h1 i j hi hc hrt
    exact h

/-- **Bridge (1)**: the touch rule as `coreI_zeroThickness_iff` states it (more than two result
wings at an edge, or two result sectors at a vert not `Conn`) is `XNonManifoldF`, for rows whose
vert rows are the core's sectors encoded (`enc`), nodup, and with
`Bd` the core's result test `p` on every row facet. -/
theorem touch_iff_xNonManifoldF {keep p : Nat → Bool} (L : Rows (P := P)) (sec : Nat → List S)
    (hsec : ∀ v, L.vsec v = (sec v).map (enc facet bounds))
    (hpe : ∀ e ∈ L.es, ∀ φ ∈ L.erow e, (Bd G keep φ ↔ p φ = true))
    (hpv : ∀ v ∈ L.vs, ∀ s ∈ sec v, (Bd G keep (facet s) ↔ p (facet s) = true))
    (hnd : ∀ v ∈ L.vs, (sec v).Nodup) :
    ((∃ e ∈ L.es, 2 < ((L.erow e).filter p).length) ∨
      (∃ v ∈ L.vs, ∃ c ∈ (sec v).filter (fun s => p (facet s)),
        ∃ d ∈ (sec v).filter (fun s => p (facet s)),
          ¬Conn (fanRelS bounds) ((sec v).filter fun s => p (facet s)) c d)) ↔
    XNonManifoldF G keep L := by
  unfold XNonManifoldF
  refine or_congr ?_ ?_
  · exact exists_congr fun e => and_congr_right fun he => (badEdgeW_iff (hpe e he)).symm
  · refine exists_congr fun v => and_congr_right fun hv => ?_
    rw [hsec v]
    exact (badVertS_iff (hnd v hv) (hpv v hv)).symm

end Vert

end ForgeCurved.FaceLocal
