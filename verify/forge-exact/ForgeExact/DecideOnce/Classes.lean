import ForgeExact.DecideOnce.FlatK
import ForgeExact.DecideOnce.Sampler

/-!
# The carrier classes are distinct planes: `NoScaled` from the class builder

The bridge's carrier classes are "the distinct snapped planes" (kernel/forge-bool/src/decide_once.rs,
`classes`): a plane joins an earlier class iff the two have one point set (`same_set`), else it
starts a new class. `classesOf` is that rule over canonical planes (`Plane.canon`: scaled so the
first nonzero normal component is 1; two planes have one point set iff their canonical planes are
equal, `canon_eq_of_scaled`, `scaled_of_canon_eq`).

* `noScaled_classesOf`: the classes satisfy `NoScaled` (FlatK.lean);
* `scaled_classesOf`: every input plane is a nonzero multiple of a class, so a flat input on the
  planes is a flat input on the classes (`flatInput_classesOf`);
* **`flat_correct_classes`**: so `exactK_of_exact` needs no hypothesis on the sampler path: for
  every flat input on nonzero planes `qs`, the sampler's ledger over `classesOf qs` is an exact
  curved ledger, and `correct_k` and `zeroThickness_iff_k` hold of it.
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.Ref ForgeExact.DecideOnce Topology Ledger

/-- The first nonzero normal component (the one `Plane.canon` divides by). -/
def lead (n : V3) : Rat := if n.x != 0 then n.x else if n.y != 0 then n.y else n.z

theorem lead_ne {n : V3} (h : n ≠ ⟨0, 0, 0⟩) : lead n ≠ 0 := by
  obtain ⟨a, b, c⟩ := n
  unfold lead
  simp only [bne_iff_ne, ne_eq]
  split
  · assumption
  · split
    · assumption
    · intro hc; apply h; simp_all

theorem canon_eq (p : Plane) : p.canon = ⟨(1 / lead p.n) * p.n, p.c / lead p.n⟩ := rfl

theorem lead_smul {k : Rat} (hk : k ≠ 0) (n : V3) : lead (k * n) = k * lead n := by
  obtain ⟨a, b, c⟩ := n
  show lead ⟨k * a, k * b, k * c⟩ = _
  unfold lead
  simp only [bne_iff_ne, ne_eq]
  by_cases ha : a = 0
  · by_cases hb : b = 0
    · simp [ha, hb]
    · have : k * b ≠ 0 := fun h => (Rat.mul_eq_zero.mp h).elim hk hb
      simp [ha, hb, this]
  · have : k * a ≠ 0 := fun h => (Rat.mul_eq_zero.mp h).elim hk ha
    simp [ha, this]

/-- A nonzero multiple has the same canonical plane. -/
theorem canon_smul {k : Rat} (hk : k ≠ 0) (n : V3) (c : Rat) (hn : n ≠ ⟨0, 0, 0⟩) :
    Plane.canon ⟨k * n, k * c⟩ = Plane.canon ⟨n, c⟩ := by
  rw [canon_eq, canon_eq]
  simp only
  rw [lead_smul hk]
  have hl := lead_ne hn
  obtain ⟨a, b, c'⟩ := n
  generalize lead ⟨a, b, c'⟩ = l at hl
  show Plane.mk ⟨_, _, _⟩ _ = Plane.mk ⟨_, _, _⟩ _
  simp only [Plane.mk.injEq, V3.mk.injEq, Flat.smul_x, Flat.smul_y, Flat.smul_z]
  refine ⟨⟨?_, ?_, ?_⟩, ?_⟩ <;> grind

theorem eval_canon (p : Plane) (x : V3) : (p.canon).eval x = (1 / lead p.n) * p.eval x := by
  rw [canon_eq]
  obtain ⟨⟨a, b, c⟩, d⟩ := p
  simp only [Plane.eval, V3.dot]
  show (1 / lead ⟨a, b, c⟩ * a) * x.x + (1 / lead ⟨a, b, c⟩ * b) * x.y + (1 / lead ⟨a, b, c⟩ * c) * x.z - _ = _
  grind

/-- Equal canonical planes: one is a nonzero multiple of the other. -/
theorem scaled_of_canon_eq {p q : Plane} (hp : p.n ≠ ⟨0, 0, 0⟩) (hq : q.n ≠ ⟨0, 0, 0⟩)
    (h : p.canon = q.canon) : ∃ k : Rat, k ≠ 0 ∧ ∀ x : V3, p.eval x = k * q.eval x := by
  have hlp := lead_ne hp
  have hlq := lead_ne hq
  refine ⟨lead p.n / lead q.n, by grind, fun x => ?_⟩
  have e := congrArg (fun P : Plane => P.eval x) h
  simp only [eval_canon] at e
  grind

/-- **The class builder**: keep a plane iff no kept plane has its canonical form. -/
def classesGo : List Plane → List Plane → List Plane
  | acc, [] => acc
  | acc, p :: ps =>
    classesGo (if acc.any (fun q => decide (q.canon = p.canon)) then acc else acc ++ [p]) ps

def classesOf (qs : List Plane) : List Plane := classesGo [] qs

theorem classesGo_spec : ∀ (ps acc : List Plane),
    acc.Pairwise (fun a b => a.canon ≠ b.canon) →
    (classesGo acc ps).Pairwise (fun a b => a.canon ≠ b.canon) ∧
    (∀ r ∈ classesGo acc ps, r ∈ acc ∨ r ∈ ps) ∧
    (∀ p ∈ acc ++ ps, ∃ r ∈ classesGo acc ps, r.canon = p.canon)
  | [], acc, h => ⟨h, fun r hr => Or.inl hr, fun p hp => ⟨p, by simp only [classesGo]; simpa using hp, rfl⟩⟩
  | p :: ps, acc, h => by
    by_cases hany : acc.any (fun q => decide (q.canon = p.canon)) = true
    · have e : classesGo acc (p :: ps) = classesGo acc ps := by simp [classesGo, hany]
      obtain ⟨h1, h2, h3⟩ := classesGo_spec ps acc h
      refine ⟨by rw [e]; exact h1, fun r hr => ?_, fun p' hp' => ?_⟩
      · rw [e] at hr
        rcases h2 r hr with h | h
        · exact Or.inl h
        · exact Or.inr (List.mem_cons_of_mem _ h)
      · rw [e]
        simp only [List.mem_append, List.mem_cons] at hp'
        rcases hp' with hp' | rfl | hp'
        · exact h3 p' (List.mem_append_left _ hp')
        · obtain ⟨q, hq, hc⟩ := List.any_eq_true.mp hany
          obtain ⟨r, hr, hrc⟩ := h3 q (List.mem_append_left _ hq)
          exact ⟨r, hr, hrc.trans (of_decide_eq_true hc)⟩
        · exact h3 p' (List.mem_append_right _ hp')
    · have e : classesGo acc (p :: ps) = classesGo (acc ++ [p]) ps := by simp [classesGo, hany]
      have hacc : (acc ++ [p]).Pairwise (fun a b => a.canon ≠ b.canon) := by
        rw [List.pairwise_append]
        refine ⟨h, by simp, fun a ha b hb => ?_⟩
        simp only [List.mem_singleton] at hb; subst hb
        intro hc
        apply hany
        exact List.any_eq_true.mpr ⟨a, ha, decide_eq_true hc⟩
      obtain ⟨h1, h2, h3⟩ := classesGo_spec ps (acc ++ [p]) hacc
      refine ⟨by rw [e]; exact h1, fun r hr => ?_, fun p' hp' => ?_⟩
      · rw [e] at hr
        rcases h2 r hr with h | h
        · simp only [List.mem_append, List.mem_singleton] at h
          rcases h with h | rfl
          · exact Or.inl h
          · exact Or.inr (List.mem_cons_self ..)
        · exact Or.inr (List.mem_cons_of_mem _ h)
      · rw [e]
        apply h3
        simp only [List.mem_append, List.mem_cons] at hp' ⊢
        rcases hp' with h | h | h
        · exact Or.inl (Or.inl h)
        · exact Or.inl (Or.inr (Or.inl h))
        · exact Or.inr h

theorem classesOf_sub {qs : List Plane} {r : Plane} (h : r ∈ classesOf qs) : r ∈ qs := by
  rcases (classesGo_spec qs [] List.Pairwise.nil).2.1 r h with h | h
  · cases h
  · exact h

theorem classesOf_pairwise (qs : List Plane) :
    (classesOf qs).Pairwise (fun a b => a.canon ≠ b.canon) :=
  (classesGo_spec qs [] List.Pairwise.nil).1

/-- **The classes are distinct planes.** -/
theorem noScaled_of_pairwise {ps : List Plane} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩)
    (hp : ps.Pairwise (fun a b => a.canon ≠ b.canon)) : NoScaled ps := by
  intro i j hi hj hij k
  apply Classical.byContradiction
  intro hall
  simp only [not_or, Classical.not_not] at hall
  obtain ⟨hn, hc⟩ := hall
  have gi : ps.getD i default = ps[i] := by simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi]
  have gj : ps.getD j default = ps[j] := by simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]
  rw [gi, gj] at hn hc
  have hni := hnz _ (List.getElem_mem hi)
  have hnj := hnz _ (List.getElem_mem hj)
  have hk : k ≠ 0 := by
    rintro rfl; apply hnj; rw [hn]
    show V3.smul 0 _ = _
    simp [V3.smul]
  have hpj : ps[j] = ⟨k * ps[i].n, k * ps[i].c⟩ := by
    rw [← hn, ← hc]
  have hcan : ps[j].canon = ps[i].canon := by
    rw [hpj, canon_smul hk _ _ hni]
  rcases Nat.lt_or_gt_of_ne hij with h | h
  · exact (List.pairwise_iff_getElem.mp hp) i j hi hj h hcan.symm
  · exact (List.pairwise_iff_getElem.mp hp) j i hj hi h hcan

theorem noScaled_classesOf {qs : List Plane} (hnz : ∀ p ∈ qs, p.n ≠ ⟨0, 0, 0⟩) :
    NoScaled (classesOf qs) :=
  noScaled_of_pairwise (fun p hp => hnz p (classesOf_sub hp)) (classesOf_pairwise qs)

/-- Every input plane is a nonzero multiple of a class. -/
theorem scaled_classesOf {qs : List Plane} (hnz : ∀ p ∈ qs, p.n ≠ ⟨0, 0, 0⟩) {q : Plane}
    (hq : q ∈ qs) : Scaled (classesOf qs) q := by
  obtain ⟨r, hr, hrc⟩ := (classesGo_spec qs [] List.Pairwise.nil).2.2 q (by simpa using hq)
  obtain ⟨k, hk, he⟩ := scaled_of_canon_eq (hnz q hq) (hnz r (classesOf_sub hr)) hrc.symm
  exact ⟨r, hr, k, hk, he⟩

theorem scaled_trans {ps : List Plane} {P : Plane} {qs : List Plane}
    (h : ∀ q ∈ qs, Scaled ps q) (hP : Scaled qs P) : Scaled ps P := by
  obtain ⟨q, hq, k, hk, he⟩ := hP
  obtain ⟨r, hr, k', hk', he'⟩ := h q hq
  refine ⟨r, hr, k * k', fun h0 => (Rat.mul_eq_zero.mp h0).elim hk hk', fun x => ?_⟩
  rw [he, he']; grind

/-- A flat input on the planes is a flat input on their classes. -/
theorem flatInput_classesOf {qs : List Plane} (hnz : ∀ p ∈ qs, p.n ≠ ⟨0, 0, 0⟩) {ss : List Solid}
    (hi : Sample.FlatInput qs ss) : Sample.FlatInput (classesOf qs) ss where
  flat := hi.flat
  closed := hi.closed
  planes s hs f hf := scaled_trans (fun _ hq => scaled_classesOf hnz hq) (hi.planes s hs f hf)

open Sample Sampler in
/-- **The flat (c) through the curved theorems, with no extra hypothesis.** For every flat input
on nonzero planes `qs`, the sampler's ledger over their classes is an exact curved ledger, and the
core's result and refusal are `correct_k`'s and `zeroThickness_iff_k`'s. -/
theorem flat_correct_classes {qs : List Plane} (hnz : ∀ p ∈ qs, p.n ≠ ⟨0, 0, 0⟩) {ss : List Solid}
    (hi : FlatInput qs ss) (rule : Rule) :
    ExactK (sampleLedger (classesOf qs) ss (sampler (classesOf qs)))
      ((classesOf qs).map fun p => p.eval) (brepA ss) ∧
    (∀ φ, φ ∈ (sampleLedger (classesOf qs) ss (sampler (classesOf qs))).result
        ((sampleLedger (classesOf qs) ss (sampler (classesOf qs))).keepTab rule) ↔
      XBoundaryK (sampleLedger (classesOf qs) ss (sampler (classesOf qs)))
        ((classesOf qs).map fun p => p.eval) (brepA ss) rule φ) ∧
    ((∃ x, core (sampleLedger (classesOf qs) ss (sampler (classesOf qs))) rule =
        .refused (.zeroThickness x)) ↔
      XNonManifoldK (sampleLedger (classesOf qs) ss (sampler (classesOf qs)))
        ((classesOf qs).map fun p => p.eval) (brepA ss) rule) := by
  have hnz' : ∀ p ∈ classesOf qs, p.n ≠ ⟨0, 0, 0⟩ := fun p hp => hnz p (classesOf_sub hp)
  have he := flat_exact hnz' (flatInput_classesOf hnz hi)
  exact ⟨exactK_of_exact he (noScaled_classesOf hnz), flat_correct_k he (noScaled_classesOf hnz) rule⟩

end ForgeExact.DecideOnce.Curved
