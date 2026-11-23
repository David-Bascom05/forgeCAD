import ForgeCurved.QTransverse

/-!
# Off the events, the sweep is regular

If `eventsOf qs` succeeds, then at every point whose `x` is none of the events:

* every carrier through it is transverse to the sweep (`(q, ∂_y q, ∂_z q)` is an event triple);
* every two carriers through it are transverse together (`(q_j, q_k, critQ)` is one);
* no three carriers pass through it (`(q_i, q_j, q_k)` is one).

That is LemmaD's `Transverse` and `NoTriple` on every slab free of events (`slab_regular`),
certified exactly by forge-exact's `eventsOf_complete`.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep

theorem dyK_real (q : Quad.Quadric) (p : R3) : dyK realModel q p = gy q p := by
  simp only [dyK, gy, realModel_q]; ring
theorem dzK_real (q : Quad.Quadric) (p : R3) : dzK realModel q p = gz q p := by
  simp only [dzK, gz, realModel_q]; ring

theorem mem_pairs : ∀ {qs : List Quad.Quadric} {j k : ℕ} (hj : j < qs.length) (hk : k < qs.length),
    j < k → (qs[j], qs[k]) ∈ pairs qs
  | [], _, _, hj, _, _ => absurd hj (by simp)
  | a :: rest, j, k, hj, hk, hjk => by
    simp only [pairs, List.mem_append, List.mem_map]
    cases j with
    | zero =>
      cases k with
      | zero => omega
      | succ k =>
        left
        have hk' : k < rest.length := by simp at hk; omega
        exact ⟨rest[k], List.getElem_mem hk', by simp⟩
    | succ j =>
      cases k with
      | zero => omega
      | succ k =>
        right
        have := mem_pairs (qs := rest) (j := j) (k := k) (by simp at hj; omega) (by simp at hk; omega)
          (by omega)
        simpa using this

theorem mem_tri : ∀ {qs : List Quad.Quadric} {i j k : ℕ} (hi : i < qs.length) (hj : j < qs.length)
    (hk : k < qs.length), i < j → j < k → (qs[i], qs[j], qs[k]) ∈ tri qs
  | [], _, _, _, hi, _, _, _, _ => absurd hi (by simp)
  | a :: rest, i, j, k, hi, hj, hk, hij, hjk => by
    simp only [tri, List.mem_append, List.mem_flatMap, List.mem_map]
    cases i with
    | zero =>
      left
      cases j with
      | zero => omega
      | succ j =>
        cases k with
        | zero => omega
        | succ k =>
          have hj' : j < rest.length := by simp at hj; omega
          have hk' : k < rest.length := by simp at hk; omega
          refine ⟨(rest[j], j), List.mem_zipIdx_iff_getElem?.mpr (by simp [hj']), rest[k], ?_, by simp⟩
          rw [List.mem_iff_getElem]
          refine ⟨k - (j + 1), by simp; omega, ?_⟩
          simp only [List.getElem_drop]
          congr 1; omega
    | succ i =>
      cases j with
      | zero => omega
      | succ j =>
        cases k with
        | zero => omega
        | succ k =>
          right
          have := mem_tri (qs := rest) (i := i) (j := j) (k := k) (by simp at hi; omega)
            (by simp at hj; omega) (by simp at hk; omega) (by omega) (by omega)
          simpa using this

variable {qs : List Quad.Quadric} {fuel : ℕ} {E : List Num}

/-- A point off the events: no event denotes its `x`. -/
def OffEvents (E : List Num) (p : R3) : Prop := ∀ e ∈ E, ¬ Denotes realModel e p.1

theorem evK_eq (q : Quad.Quadric) (p : R3) : Quadric.evK realModel q (p.1, p.2.1, p.2.2) = qR q p :=
  evK_real q p

theorem off_triple (h : eventsOf qs fuel = some E) {p : R3} (hp : OffEvents E p) {a b c : Quad.Quadric}
    (ht : (a, b, c) ∈ eventTriples qs) (ha : qR a p = 0) (hb : qR b p = 0) (hc : qR c p = 0) : False := by
  obtain ⟨e, he, hd⟩ := eventsOf_complete realModel h ht (x := p.1) (y := p.2.1) (z := p.2.2)
    (by rw [evK_eq]; exact ha) (by rw [evK_eq]; exact hb) (by rw [evK_eq]; exact hc)
  exact hp e he hd

/-- **One carrier, off the events, is transverse.** -/
theorem off_transverse1 (h : eventsOf qs fuel = some E) {p : R3} (hp : OffEvents E p)
    {q : Quad.Quadric} (hq : q ∈ qs) (h0 : qR q p = 0) : Transverse ℓx [qR q] p := by
  apply transverse1
  by_contra hn
  push Not at hn
  apply off_triple h hp (a := q) (b := gradPlaneY q) (c := gradPlaneZ q)
  · simp only [eventTriples, List.mem_append, List.mem_map]
    right; exact ⟨q, hq, rfl⟩
  · exact h0
  · rw [← evK_eq, gradPlaneY_ev, dyK_real]; exact hn.1
  · rw [← evK_eq, gradPlaneZ_ev, dzK_real]; exact hn.2

/-- **Two carriers, off the events, are transverse together.** -/
theorem off_transverse2 (h : eventsOf qs fuel = some E) {p : R3} (hp : OffEvents E p)
    {j k : ℕ} (hj : j < qs.length) (hk : k < qs.length) (hjk : j ≠ k)
    (h0j : qR qs[j] p = 0) (h0k : qR qs[k] p = 0) : Transverse ℓx [qR qs[j], qR qs[k]] p := by
  apply transverse2
  intro hc
  rcases Nat.lt_or_gt_of_ne hjk with hlt | hlt
  · apply off_triple h hp (a := qs[j]) (b := qs[k]) (c := critQ qs[j] qs[k])
    · simp only [eventTriples, List.mem_append, List.mem_map]
      left; right; exact ⟨(qs[j], qs[k]), mem_pairs hj hk hlt, rfl⟩
    · exact h0j
    · exact h0k
    · rw [← evK_eq, critQ_ev, dyK_real, dzK_real, dyK_real, dzK_real]; exact hc
  · apply off_triple h hp (a := qs[k]) (b := qs[j]) (c := critQ qs[k] qs[j])
    · simp only [eventTriples, List.mem_append, List.mem_map]
      left; right; exact ⟨(qs[k], qs[j]), mem_pairs hk hj hlt, rfl⟩
    · exact h0k
    · exact h0j
    · rw [← evK_eq, critQ_ev, dyK_real, dzK_real, dyK_real, dzK_real]; linarith

/-- **No three carriers pass through a point off the events.** -/
theorem off_noTriple (h : eventsOf qs fuel = some E) {p : R3} (hp : OffEvents E p)
    {i j k : ℕ} (hi : i < qs.length) (hj : j < qs.length) (hk : k < qs.length)
    (hij : i ≠ j) (hjk : j ≠ k) (hik : i ≠ k) :
    ¬(qR qs[i] p = 0 ∧ qR qs[j] p = 0 ∧ qR qs[k] p = 0) := by
  rintro ⟨h1, h2, h3⟩
  have key : ∀ a b c (ha : a < qs.length) (hb : b < qs.length) (hc : c < qs.length), a < b → b < c →
      qR qs[a] p = 0 → qR qs[b] p = 0 → qR qs[c] p = 0 → False :=
    fun a b c ha hb hc hab hbc za zb zc => off_triple h hp
      (by simp only [eventTriples, List.mem_append]; left; left; exact mem_tri ha hb hc hab hbc)
      za zb zc
  rcases Nat.lt_or_gt_of_ne hij with h_ij | h_ij <;> rcases Nat.lt_or_gt_of_ne hjk with h_jk | h_jk <;>
    rcases Nat.lt_or_gt_of_ne hik with h_ik | h_ik
  all_goals first
    | exact key i j k hi hj hk h_ij h_jk h1 h2 h3
    | exact key i k j hi hk hj h_ik (by omega) h1 h3 h2
    | exact key j i k hj hi hk (by omega) h_ik h2 h1 h3
    | exact key j k i hj hk hi h_jk (by omega) h2 h3 h1
    | exact key k i j hk hi hj (by omega) h_ij h3 h1 h2
    | exact key k j i hk hj hi (by omega) (by omega) h3 h2 h1
    | omega

/-- **A slab free of events is regular**: LemmaD's transversality and `NoTriple` hold on it. -/
theorem slab_regular (h : eventsOf qs fuel = some E) {a b : ℝ}
    (hfree : ∀ p ∈ Slab ℓx a b, OffEvents E p) :
    (∀ j (hj : j < (qs.map qR).length), ∀ x ∈ Zs ℓx [(qs.map qR)[j]] a b,
        Transverse ℓx [(qs.map qR)[j]] x) ∧
    (∀ j k (hj : j < (qs.map qR).length) (hk : k < (qs.map qR).length), j ≠ k →
      ∀ x ∈ Zs ℓx [(qs.map qR)[j], (qs.map qR)[k]] a b,
        Transverse ℓx [(qs.map qR)[j], (qs.map qR)[k]] x) ∧
    NoTriple ℓx (qs.map qR) a b := by
  refine ⟨?_, ?_, ?_⟩
  · intro j hj x hx
    simp only [List.getElem_map]
    have hj' : j < qs.length := by simpa using hj
    exact off_transverse1 h (hfree x hx.1) (List.getElem_mem hj')
      (by simpa using hx.2 _ (List.mem_singleton_self _))
  · intro j k hj hk hjk x hx
    simp only [List.getElem_map]
    have hj' : j < qs.length := by simpa using hj
    have hk' : k < qs.length := by simpa using hk
    have h2 := hx.2
    simp only [List.getElem_map, List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp,
      forall_eq] at h2
    exact off_transverse2 h (hfree x hx.1) hj' hk' hjk h2.1 h2.2
  · intro x hx i j k hi hj hk hij hjk hik
    simp only [List.getElem_map]
    exact off_noTriple h (hfree x hx) (by simpa using hi) (by simpa using hj) (by simpa using hk)
      hij hjk hik

end ForgeCurved
