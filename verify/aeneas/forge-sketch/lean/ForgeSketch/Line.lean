import ForgeSketch.Funs
import ForgeText

open Aeneas Aeneas.Std Result
open forge_sketch

namespace ForgeSketch

/-- A kernel point's exact value. -/
def q (p : forge_math.vec.Point2) : Winding2d.Pt := ⟨p.x.val, p.y.val⟩

def Finite (p : forge_math.vec.Point2) : Prop := p.x.isFinite ∧ p.y.isFinite

theorem zero_fin : (F64.Ops.ofBits 0).isFinite = true := by decide
theorem zero_val : (F64.Ops.ofBits 0).val = 0 := by decide
theorem one_fin : (F64.Ops.ofBits 4607182418800017408).isFinite = true := by decide
theorem one_ne : (F64.Ops.ofBits 4607182418800017408).val ≠ 0 := by
  rw [ne_eq, IEEE754.FP.val_eq_zero_iff one_fin]; decide
theorem one_eq_zero : F64.Ops.eq (F64.Ops.ofBits 4607182418800017408) (F64.Ops.ofBits 0) = false := by
  cases h : F64.Ops.eq (F64.Ops.ofBits 4607182418800017408) (F64.Ops.ofBits 0)
  · rfl
  · have := (IEEE754.FP.eq_iff one_fin zero_fin).1 h
    rw [zero_val] at this; exact absurd this one_ne

theorem le_min (a b z : F64) (ha : a.isFinite) (hb : b.isFinite) (hz : z.isFinite) :
    core.f64.F64.min a b ⦃ m => F64.Ops.le m z = decide (a.val ≤ z.val ∨ b.val ≤ z.val) ⦄ := by
  have nA : a.isNaN = false := by cases a <;> simp_all [IEEE754.FP.isFinite, IEEE754.FP.isNaN]
  have nB : b.isNaN = false := by cases b <;> simp_all [IEEE754.FP.isFinite, IEEE754.FP.isNaN]
  simp only [core.f64.F64.min, nA, nB, WP.spec_ok, Bool.false_eq_true, ↓reduceIte, F64.Ops.le, F64.Ops.lt]
  have hl := IEEE754.FP.le_iff ha hz
  have hl' := IEEE754.FP.le_iff hb hz
  have hlt := IEEE754.FP.lt_iff hb ha
  by_cases h : IEEE754.FP.lt b a = true <;> simp only [h, ↓reduceIte, Bool.false_eq_true]
  · have hba := hlt.1 h
    rw [Bool.eq_iff_iff, hl', decide_eq_true_iff]
    constructor
    · intro h'; exact Or.inr h'
    · rintro (h' | h') <;> linarith
  · have hba : ¬ b.val < a.val := fun h' => h (hlt.2 h')
    rw [Bool.eq_iff_iff, hl, decide_eq_true_iff]
    constructor
    · intro h'; exact Or.inl h'
    · rintro (h' | h') <;> linarith

theorem le_max (a b z : F64) (ha : a.isFinite) (hb : b.isFinite) (hz : z.isFinite) :
    core.f64.F64.max a b ⦃ m => F64.Ops.le z m = decide (z.val ≤ a.val ∨ z.val ≤ b.val) ⦄ := by
  have nA : a.isNaN = false := by cases a <;> simp_all [IEEE754.FP.isFinite, IEEE754.FP.isNaN]
  have nB : b.isNaN = false := by cases b <;> simp_all [IEEE754.FP.isFinite, IEEE754.FP.isNaN]
  simp only [core.f64.F64.max, nA, nB, WP.spec_ok, Bool.false_eq_true, ↓reduceIte, F64.Ops.le, F64.Ops.lt]
  have hl := IEEE754.FP.le_iff hz ha
  have hl' := IEEE754.FP.le_iff hz hb
  have hlt := IEEE754.FP.lt_iff ha hb
  by_cases h : IEEE754.FP.lt a b = true <;> simp only [h, ↓reduceIte, Bool.false_eq_true]
  · have hab := hlt.1 h
    rw [Bool.eq_iff_iff, hl', decide_eq_true_iff]
    constructor
    · intro h'; exact Or.inr h'
    · rintro (h' | h') <;> linarith
  · have hab : ¬ a.val < b.val := fun h' => h (hlt.2 h')
    rw [Bool.eq_iff_iff, hl, decide_eq_true_iff]
    constructor
    · intro h'; exact Or.inl h'
    · rintro (h' | h') <;> linarith

/-- What the line test needs of `orient2d` (forge-math's claim for it, proven there): a finite
result with the sign of the exact determinant of the input doubles. -/
def OrientOk (a b p : forge_math.vec.Point2) : Prop :=
  forge_math.predicates.orient2d a b p ⦃ o => o.isFinite ∧
    (0 < o.val ↔ 0 < Winding2d.orient (q a) (q b) (q p)) ∧
    (o.val < 0 ↔ Winding2d.orient (q a) (q b) (q p) < 0) ⦄

theorem lt_zero_iff {o : F64} (h : o.isFinite) : F64.Ops.lt o (F64.Ops.ofBits 0) = decide (o.val < 0) := by
  rw [Bool.eq_iff_iff, decide_eq_true_iff, F64.Ops.lt, IEEE754.FP.lt_iff h zero_fin, zero_val]

theorem gt_zero_iff {o : F64} (h : o.isFinite) : F64.Ops.gt o (F64.Ops.ofBits 0) = decide (0 < o.val) := by
  rw [Bool.eq_iff_iff, decide_eq_true_iff, F64.Ops.gt, IEEE754.FP.lt_iff zero_fin h, zero_val]

theorem eq_zero_iff {o : F64} (h : o.isFinite) : F64.Ops.eq o (F64.Ops.ofBits 0) = decide (o.val = 0) := by
  rw [Bool.eq_iff_iff, decide_eq_true_iff, F64.Ops.eq, IEEE754.FP.eq_iff h zero_fin, zero_val]

/-- **The kernel's line test is the spec's** (`winding::line` = ForgeText's `lineCrossQ` on the
doubles' exact values), for finite points, given `orient2d`'s sign. -/
theorem line_spec (a b p : forge_math.vec.Point2) (ha : Finite a) (hb : Finite b) (hp : Finite p)
    (ho : OrientOk a b p) :
    winding.line a b p ⦃ r => r.map (fun i => i.val) = ForgeText.lineCrossQ (q a) (q b) (q p) ⦄ := by
  obtain ⟨hax, hay⟩ := ha
  obtain ⟨hbx, hby⟩ := hb
  obtain ⟨hpx, hpy⟩ := hp
  unfold winding.line winding.above
  step with le_min a.x b.x p.x hax hbx hpx as ⟨fl, hfl⟩
  apply WP.spec_bind (Pₘ := fun w => (w = true ↔ (((q a).x ≤ (q p).x ∨ (q b).x ≤ (q p).x) ∧
      ((q p).x ≤ (q a).x ∨ (q p).x ≤ (q b).x) ∧ ((q a).y ≤ (q p).y ∨ (q b).y ≤ (q p).y) ∧
      ((q p).y ≤ (q a).y ∨ (q p).y ≤ (q b).y))))
  · rw [hfl]
    split
    · rename_i h1
      step with le_max a.x b.x p.x hax hbx hpx as ⟨fl1, hfl1⟩
      rw [hfl1]
      split
      · rename_i h2
        step with le_min a.y b.y p.y hay hby hpy as ⟨fl2, hfl2⟩
        rw [hfl2]
        split
        · rename_i h3
          step with le_max a.y b.y p.y hay hby hpy as ⟨fl3, hfl3⟩
          simp only [WP.spec_ok, q, hfl3, decide_eq_true_iff] at *
          tauto
        · rename_i h; simp only [WP.spec_ok, q, decide_eq_true_iff] at h ⊢; tauto
      · rename_i h; simp only [WP.spec_ok, q, decide_eq_true_iff] at h ⊢; tauto
    · rename_i h; simp only [WP.spec_ok, q, decide_eq_true_iff] at h ⊢; tauto
  intro within hw
  unfold OrientOk at ho
  generalize hO : Winding2d.orient (q a) (q b) (q p) = O at ho
  apply WP.spec_bind (Pₘ := fun o => o.isFinite ∧
    (within = true → ((0 < o.val ↔ 0 < O) ∧ (o.val < 0 ↔ O < 0))) ∧ (within = false → o.val ≠ 0))
  · cases within
    · simp only [Bool.false_eq_true, ↓reduceIte, WP.spec_ok]
      exact ⟨one_fin, fun h => absurd h (by simp), fun _ => one_ne⟩
    · simp only [↓reduceIte]
      exact WP.spec_mono ho (fun o h => ⟨h.1, fun _ => h.2, fun h' => absurd h' (by simp)⟩)
  intro o ⟨hof, hos, hon⟩
  have hyA := IEEE754.FP.lt_iff hpy hay
  have hyB := IEEE754.FP.lt_iff hpy hby
  unfold ForgeText.lineCrossQ
  simp only [hO]
  rw [eq_zero_iff hof]
  have key : o.val = 0 ↔ ((((q a).x ≤ (q p).x ∨ (q b).x ≤ (q p).x) ∧
      ((q p).x ≤ (q a).x ∨ (q p).x ≤ (q b).x) ∧ ((q a).y ≤ (q p).y ∨ (q b).y ≤ (q p).y) ∧
      ((q p).y ≤ (q a).y ∨ (q p).y ≤ (q b).y)) ∧ O = 0) := by
    cases within
    · have := hon rfl
      constructor
      · intro h; exact absurd h this
      · rintro ⟨hW, _⟩; exact absurd (hw.2 hW) (by simp)
    · obtain ⟨h1, h2⟩ := hos rfl
      have hW := hw.1 rfl
      constructor
      · intro h; refine ⟨hW, ?_⟩
        rcases lt_trichotomy O 0 with h' | h' | h'
        · have := h2.2 h'; rw [h] at this; exact absurd this (lt_irrefl _)
        · exact h'
        · have := h1.2 h'; rw [h] at this; exact absurd this (lt_irrefl _)
      · rintro ⟨_, h⟩
        rcases lt_trichotomy o.val 0 with h' | h' | h'
        · have := h2.1 h'; rw [h] at this; exact absurd this (lt_irrefl _)
        · exact h'
        · have := h1.1 h'; rw [h] at this; exact absurd this (lt_irrefl _)
  by_cases hz : o.val = 0
  · have hz' := key.1 hz
    simp [hz, hz']
  · have hz' : ¬ _ := fun h => hz (key.2 h)
    simp only [hz, hz', decide_false, Bool.false_eq_true, ↓reduceIte]
    have hga : F64.Ops.gt a.y p.y = decide ((q p).y < (q a).y) := by
      rw [Bool.eq_iff_iff, decide_eq_true_iff]; exact hyA
    have hgb : F64.Ops.gt b.y p.y = decide ((q p).y < (q b).y) := by
      rw [Bool.eq_iff_iff, decide_eq_true_iff]; exact hyB
    rw [hga, hgb]
    by_cases hab : decide ((q p).y < (q a).y) = decide ((q p).y < (q b).y)
    · simp [hab]
    · simp only [hab, ↓reduceIte]
      apply WP.spec_bind (Pₘ := fun o1 => o1.isFinite ∧ (0 < o1.val ↔ 0 < O) ∧ (o1.val < 0 ↔ O < 0))
      · cases within
        · exact ho
        · simp only [↓reduceIte, WP.spec_ok]; exact ⟨hof, hos rfl⟩
      intro o1 ⟨h1f, h1p, h1n⟩
      cases hup : decide ((q p).y < (q b).y)
      · simp only [Bool.false_eq_true, ↓reduceIte, bind_ok, lt_zero_iff h1f, h1n]
        by_cases hn : O < 0 <;> simp [hn]
      · simp only [↓reduceIte, bind_ok, gt_zero_iff h1f, h1p]
        by_cases hp' : 0 < O <;> simp [hp']
/-- On points whose values are integers (a glyph on its grid), the kernel's line test is
ForgeText's integer `lineCross`, the one `winding_is_topological` is about. -/
theorem line_int (a b p : forge_math.vec.Point2) (A B P : ForgeText.Pt)
    (hA : q a = ForgeText.toQ A) (hB : q b = ForgeText.toQ B) (hP : q p = ForgeText.toQ P)
    (ha : Finite a) (hb : Finite b) (hp : Finite p) (ho : OrientOk a b p) :
    winding.line a b p ⦃ r => r.map (fun i => i.val) = ForgeText.lineCross A B P ⦄ := by
  rw [ForgeText.lineCross_toQ, ← hA, ← hB, ← hP]
  exact line_spec a b p ha hb hp ho

end ForgeSketch
