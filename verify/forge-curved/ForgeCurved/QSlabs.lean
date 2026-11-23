import ForgeCurved.QEvents

/-!
# Every point off the events lies in a sampled slab

`eventsOf` lists the events separated (`sepOk`: consecutive isolating intervals are disjoint), so
their values increase strictly along the list. `planesOf` puts a rational sample plane below the
first, between each two consecutive, and above the last.

* **`slab_find`**: a level `t` that is no event lies in an open interval `(a, b)` with no event in
  it and a sample plane `c ∈ planesOf E` in it.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep

/-- The value an event denotes (where it denotes one). -/
noncomputable def valE (e : Num) : ℝ :=
  open Classical in if h : ∃ v, Denotes realModel e v then h.choose else 0

theorem valE_eq {e : Num} {v : ℝ} (h : Denotes realModel e v) : v = valE e := by
  have hx : ∃ v, Denotes realModel e v := ⟨v, h⟩
  unfold valE
  rw [dif_pos hx]
  exact Denotes.unique realModel h hx.choose_spec

theorem valE_den {e : Num} (h : ∃ v, Denotes realModel e v) : Denotes realModel e (valE e) := by
  obtain ⟨v, hv⟩ := h
  rw [← valE_eq hv]; exact hv

/-- All events denote. -/
def AllDen (E : List Num) : Prop := ∀ e ∈ E, ∃ v, Denotes realModel e v

theorem sep_lt {u w : Num} (hs : u.hi < w.lo) (hu : ∃ v, Denotes realModel u v)
    (hw : ∃ v, Denotes realModel w v) : valE u < valE w := by
  have h1 := Denotes.le_hi realModel (valE_den hu)
  have h2 := Denotes.lo_le realModel (valE_den hw)
  simp only [realModel_q] at h1 h2
  have : (u.hi : ℝ) < w.lo := by exact_mod_cast hs
  linarith

/-- The head is the least value and the last the greatest. -/
theorem bounds : ∀ (u : Num) (rest : List Num), sepOk (u :: rest) = true → AllDen (u :: rest) →
    ∀ e ∈ u :: rest, valE u ≤ valE e ∧ valE e ≤ valE ((u :: rest).getLast (by simp))
  | u, [], _, _, e, he => by
    simp only [List.mem_singleton] at he; subst he; simp
  | u, w :: rest, hs, hd, e, he => by
    simp only [sepOk, Bool.and_eq_true, decide_eq_true_eq] at hs
    have huw := sep_lt hs.1 (hd u (by simp)) (hd w (by simp))
    have ih := bounds w rest hs.2 (fun e he => hd e (List.mem_cons_of_mem _ he))
    rw [List.getLast_cons (by simp)]
    rcases List.mem_cons.mp he with rfl | he
    · exact ⟨_root_.le_refl _, _root_.le_trans huw.le (ih w (by simp)).2⟩
    · exact ⟨_root_.le_trans huw.le (ih e he).1, (ih e he).2⟩

/-- No event lies strictly between two consecutive ones. -/
theorem gap : ∀ (L : List Num), sepOk L = true → AllDen L →
    ∀ p ∈ L.zip L.tail, ∀ e ∈ L, valE e ≤ valE p.1 ∨ valE p.2 ≤ valE e
  | [], _, _, p, hp, _, _ => by simp at hp
  | [_], _, _, p, hp, _, _ => by simp at hp
  | x :: y :: rest, hs, hd, p, hp, e, he => by
    have hs' := hs
    simp only [sepOk, Bool.and_eq_true, decide_eq_true_eq] at hs'
    have hxy := sep_lt hs'.1 (hd x (by simp)) (hd y (by simp))
    have hd' : AllDen (y :: rest) := fun e he => hd e (List.mem_cons_of_mem _ he)
    have hb := bounds y rest hs'.2 hd'
    simp only [List.tail_cons, List.zip_cons_cons, List.mem_cons] at hp
    rcases hp with rfl | hp
    · rcases List.mem_cons.mp he with rfl | he
      · left; exact _root_.le_refl _
      · right; exact (hb e he).1
    · have hp' : p ∈ (y :: rest).zip (y :: rest).tail := by simpa using hp
      have hu : p.1 ∈ y :: rest := (List.of_mem_zip hp').1
      rcases List.mem_cons.mp he with rfl | he
      · left; exact _root_.le_trans hxy.le (hb p.1 hu).1
      · exact gap (y :: rest) hs'.2 hd' p hp' e he

/-- Off the events, a level is below the first, between two consecutive, or above the last. -/
theorem locate : ∀ (u : Num) (rest : List Num), sepOk (u :: rest) = true → AllDen (u :: rest) →
    ∀ t : ℝ, (∀ e ∈ u :: rest, valE e ≠ t) →
      t < valE u ∨ (∃ p ∈ (u :: rest).zip rest, valE p.1 < t ∧ t < valE p.2) ∨
        valE ((u :: rest).getLast (by simp)) < t
  | u, [], _, _, t, ht => by
    rcases lt_or_gt_of_ne (ht u (by simp)).symm with h | h
    · left; exact h
    · right; right; simpa using h
  | u, w :: rest, hs, hd, t, ht => by
    simp only [sepOk, Bool.and_eq_true, decide_eq_true_eq] at hs
    rcases lt_or_gt_of_ne (ht u (by simp)).symm with h | h
    · left; exact h
    rcases lt_or_gt_of_ne (ht w (by simp)).symm with h' | h'
    · right; left; exact ⟨(u, w), by simp, h, h'⟩
    rcases locate w rest hs.2 (fun e he => hd e (List.mem_cons_of_mem _ he)) t
        (fun e he => ht e (List.mem_cons_of_mem _ he)) with h2 | h2 | h2
    · exact absurd h2 (not_lt.mpr h'.le)
    · obtain ⟨p, hp, hp1, hp2⟩ := h2
      right; left; exact ⟨p, by simp only [List.zip_cons_cons, List.mem_cons]; right; exact hp,
        hp1, hp2⟩
    · right; right; rw [List.getLast_cons (by simp)]; exact h2

/-- **A level off the events lies in a sampled, event-free slab.** -/
theorem slab_find {E : List Num} (hs : sepOk E = true) (hd : AllDen E) {t : ℝ}
    (ht : ∀ e ∈ E, ¬ Denotes realModel e t) :
    ∃ c ∈ planesOf E, ∃ a b : ℝ, a < t ∧ t < b ∧ a < c ∧ c < b ∧
      ∀ e ∈ E, ∀ v, Denotes realModel e v → v ≤ a ∨ b ≤ v := by
  have hne : ∀ e ∈ E, valE e ≠ t := fun e he h => ht e he (h ▸ valE_den (hd e he))
  cases E with
  | nil =>
    refine ⟨0, by simp [planesOf], min t 0 - 1, max t 0 + 1, ?_, ?_, ?_, ?_, fun e he => by simp at he⟩
    all_goals push_cast
    all_goals linarith [min_le_left t 0, min_le_right t 0, le_max_left t 0, le_max_right t 0]
  | cons u rest =>
    have hb := bounds u rest hs hd
    have hval : ∀ e ∈ u :: rest, ∀ v, Denotes realModel e v → v = valE e := fun e _ v h => valE_eq h
    rcases locate u rest hs hd t hne with h | h | h
    · -- below the first: the plane u.lo − 1
      have hlo : ((u.lo - 1 : ℚ) : ℝ) < valE u := by
        have := Denotes.lo_le realModel (valE_den (hd u (by simp)))
        simp only [realModel_q] at this; push_cast; linarith
      refine ⟨u.lo - 1, by simp [planesOf], min t ((u.lo - 1 : ℚ) : ℝ) - 1, valE u, ?_, h, ?_, hlo,
        fun e he v hv => Or.inr (by rw [hval e he v hv]; exact (hb e he).1)⟩
      · linarith [min_le_left t ((u.lo - 1 : ℚ) : ℝ)]
      · linarith [min_le_right t ((u.lo - 1 : ℚ) : ℝ)]
    · -- between two consecutive: their midpoint
      obtain ⟨p, hp, hp1, hp2⟩ := h
      have hp' : p ∈ (u :: rest).zip (u :: rest).tail := by simpa using hp
      have h2 : p.2 ∈ u :: rest := List.mem_of_mem_tail (List.of_mem_zip hp').2
      have hmid := planes_between realModel hs (u := p.1) (v := p.2) (by simpa using hp')
        (valE_den (hd _ (List.of_mem_zip hp').1)) (valE_den (hd _ h2))
      refine ⟨(p.1.hi + p.2.lo) / 2, ?_, valE p.1, valE p.2, hp1, hp2, by simpa using hmid.1,
        by simpa using hmid.2, fun e he v hv => ?_⟩
      · simp only [planesOf]
        apply List.mem_cons_of_mem
        apply List.mem_append_left
        obtain ⟨p1, p2⟩ := p
        exact List.mem_map.mpr ⟨(p1, p2), hp, rfl⟩
      · rw [hval e he v hv]; exact gap (u :: rest) hs hd p hp' e he
    · -- above the last: the plane last.hi + 1
      set w := (u :: rest).getLast (by simp)
      have hw : w ∈ u :: rest := List.getLast_mem _
      have hhi : valE w < ((w.hi + 1 : ℚ) : ℝ) := by
        have := Denotes.le_hi realModel (valE_den (hd w hw))
        simp only [realModel_q] at this; push_cast; linarith
      refine ⟨w.hi + 1, ?_, valE w, max t ((w.hi + 1 : ℚ) : ℝ) + 1, h, ?_, hhi, ?_,
        fun e he v hv => Or.inl (by rw [hval e he v hv]; exact (hb e he).2)⟩
      · simp only [planesOf]
        apply List.mem_cons_of_mem
        apply List.mem_append_right
        simp [w, List.getLast?_eq_some_getLast]
      · linarith [le_max_left t ((w.hi + 1 : ℚ) : ℝ)]
      · linarith [le_max_right t ((w.hi + 1 : ℚ) : ℝ)]

end ForgeCurved
