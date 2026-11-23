import ForgeExact.Quad.Basic

/-!
# Certificates over quadric arrangements

* `pairCheck`: an **exclusion certificate** for two quadrics: `λ a + μ b = Σ d_k ℓ_k²` with
  `λ, μ, d_k ≥ 0` and `λ + μ > 0` (the S-lemma's form), checked coefficient by coefficient. Then
  no point has `a < 0` and `b < 0` (`pairCheck_spec`). This is how the reference proves a
  tangency is a touch and not a crossing: two balls touching from outside can have no common
  interior point, and the certificate says so exactly.
* `empty_of_patterns`: if the formula fails on the pattern of every point off the wall, the
  regularised boolean is empty.
-/

namespace ForgeExact.Quad

open ForgeExact Topology Ref Flat

/-- An affine form `a x + b y + c z + d`. -/
structure Affine where
  a : Rat
  b : Rat
  c : Rat
  d : Rat
  deriving Inhabited, Repr, BEq

def Affine.eval (l : Affine) (p : V3) : Rat := l.a * p.x + l.b * p.y + l.c * p.z + l.d

namespace Quadric

def add (q r : Quadric) : Quadric :=
  ⟨q.xx + r.xx, q.yy + r.yy, q.zz + r.zz, q.xy + r.xy, q.xz + r.xz, q.yz + r.yz, q.lx + r.lx,
   q.ly + r.ly, q.lz + r.lz, q.c + r.c⟩

def smul (k : Rat) (q : Quadric) : Quadric :=
  ⟨k * q.xx, k * q.yy, k * q.zz, k * q.xy, k * q.xz, k * q.yz, k * q.lx, k * q.ly, k * q.lz, k * q.c⟩

def zero : Quadric := ⟨0, 0, 0, 0, 0, 0, 0, 0, 0, 0⟩

/-- `ℓ²`. -/
def sq (l : Affine) : Quadric :=
  ⟨l.a * l.a, l.b * l.b, l.c * l.c, 2 * l.a * l.b, 2 * l.a * l.c, 2 * l.b * l.c, 2 * l.a * l.d,
   2 * l.b * l.d, 2 * l.c * l.d, l.d * l.d⟩

theorem eval_add (q r : Quadric) (p : V3) : (q.add r).eval p = q.eval p + r.eval p := by
  simp only [add, eval]; grind
theorem eval_smul (k : Rat) (q : Quadric) (p : V3) : (q.smul k).eval p = k * q.eval p := by
  simp only [smul, eval]; grind
theorem eval_zero (p : V3) : zero.eval p = 0 := by simp only [zero, eval]; grind
theorem eval_sq (l : Affine) (p : V3) : (sq l).eval p = l.eval p * l.eval p := by
  simp only [sq, eval, Affine.eval]; grind

/-- `Σ d_k ℓ_k²`. -/
def sos : List (Rat × Affine) → Quadric
  | [] => zero
  | (d, l) :: t => ((sq l).smul d).add (sos t)

theorem sos_nonneg : ∀ (ts : List (Rat × Affine)), (∀ t ∈ ts, 0 ≤ t.1) → ∀ p, 0 ≤ (sos ts).eval p
  | [], _, p => by rw [sos, eval_zero]; decide
  | (d, l) :: t, h, p => by
    rw [sos, eval_add, eval_smul, eval_sq]
    have h1 := h (d, l) (List.mem_cons_self ..)
    have h2 := sos_nonneg t (fun u hu => h u (List.mem_cons_of_mem _ hu)) p
    have h3 := Flat.mul_self_nonneg' (l.eval p)
    have := Rat.mul_nonneg h1 h3
    grind

end Quadric

/-- The exclusion certificate: `λ a + μ b = Σ d_k ℓ_k²`. -/
def pairCheck (a b : Quadric) (lam mu : Rat) (ts : List (Rat × Affine)) : Bool :=
  decide (0 ≤ lam) && decide (0 ≤ mu) && decide (0 < lam + mu) && ts.all (fun t => decide (0 ≤ t.1)) &&
    decide ((a.smul lam).add (b.smul mu) = Quadric.sos ts)

/-- **No point has `a < 0` and `b < 0`.** -/
theorem pairCheck_spec {a b : Quadric} {lam mu : Rat} {ts : List (Rat × Affine)}
    (h : pairCheck a b lam mu ts = true) (p : V3) : ¬(a.eval p < 0 ∧ b.eval p < 0) := by
  simp only [pairCheck, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at h
  obtain ⟨⟨⟨⟨hl, hm⟩, hlm⟩, hts⟩, he⟩ := h
  intro ⟨ha, hb⟩
  have hs := Quadric.sos_nonneg ts hts p
  rw [← he, Quadric.eval_add, Quadric.eval_smul, Quadric.eval_smul] at hs
  have k1 := Rat.mul_le_mul_of_nonneg_left (Rat.le_of_lt ha) hl
  have k2 := Rat.mul_le_mul_of_nonneg_left (Rat.le_of_lt hb) hm
  by_cases hl0 : 0 < lam
  · have := Rat.mul_lt_mul_of_pos_left ha hl0; grind
  · have hm0 : 0 < mu := by grind
    have := Rat.mul_lt_mul_of_pos_left hb hm0; grind

universe v

/-- **Emptiness from patterns**: if the formula fails on the pattern of every point off the wall,
the regularised boolean is empty. -/
theorem empty_of_patterns {ι : Type v} (F : ι → QForm) (qs : List Quadric) (hqs : ∀ q ∈ qs, q.NZ)
    (hF : ∀ i, ∀ q ∈ (F i).quadrics, q ∈ qs) (op : (ι → Prop) → Prop)
    (hno : ∀ x, ¬wall qs x → ¬op fun i => (F i).onSigns qs (patternOf qs x) = true) (x : V3) :
    ¬reg (combine op fun i => (F i).den) x := by
  intro hx
  obtain ⟨y, _, U, hU, hUs, hyU⟩ := hx (fun _ => True) isOpen_univ trivial
  obtain ⟨hs, hys, hsU⟩ := hU y hyU
  obtain ⟨z, hz, hoff⟩ := Quad.escape qs hqs hs y hys
  have hzw : ¬wall qs z := fun ⟨q, hq, h0⟩ => hoff q hq h0
  have hzc : combine op (fun i => (F i).den) z := hUs z (hsU z hz)
  apply hno z hzw
  have hc := cellOf_patternOf qs z hzw
  have : (fun i => (F i).onSigns qs (patternOf qs z) = true) = (fun i => (F i).den z) :=
    funext fun i => propext (QForm.den_iff_onSigns qs _ z hc (F i) (hF i)).symm
  rw [this]; exact hzc

end ForgeExact.Quad
