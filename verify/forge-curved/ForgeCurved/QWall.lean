import ForgeCurved.QLift
import ForgeCurved.Bridge

/-!
# The wall of nonzero quadrics is nowhere dense

`ExactK.wallND`. A quadric vanishing on an open set is zero: restrict it to six directions
through a point of the set, at three step sizes each (`quad_three`). So each nonzero carrier's
zero set is closed with empty interior, and so is their finite union.

* `nzB`: some coefficient is nonzero (a checked certificate);
* `quad_zero_of_open`, `interior_zero`;
* **`wallND_q`**.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set

/-- Some coefficient is nonzero. -/
def nzB (q : Quad.Quadric) : Bool :=
  !(q.xx == 0 && q.yy == 0 && q.zz == 0 && q.xy == 0 && q.xz == 0 && q.yz == 0 && q.lx == 0 &&
    q.ly == 0 && q.lz == 0 && q.c == 0)

/-- On an open set round `p`, along a direction `v` with `‖v‖ ≤ 1`: no first or second order
term. -/
theorem along_zero {q : Quad.Quadric} {p : R3} {r : ℝ} (hr : 0 < r)
    (hU : ∀ z ∈ Metric.ball p r, qR q z = 0) (v : R3) (hv : ‖v‖ ≤ 1) :
    qq q v = 0 ∧ gx q p * v.1 + gy q p * v.2.1 + gz q p * v.2.2 = 0 := by
  have hin : ∀ s, 0 < s → s < r →
      qR q (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) = 0 := by
    intro s hs hsr
    apply hU
    rw [Metric.mem_ball]
    have : dist (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) p = s * ‖v‖ := by
      rw [dist_eq_norm]
      have : (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) - p = s • v := by
        obtain ⟨a, b, c⟩ := p; obtain ⟨x, y, z⟩ := v
        simp only [Prod.mk_sub_mk, Prod.smul_mk, smul_eq_mul, Prod.mk.injEq]
        refine ⟨by ring, by ring, by ring⟩
      rw [this, norm_smul, Real.norm_eq_abs, abs_of_pos hs]
    rw [this]
    nlinarith
  have e := fun s hs hsr => (qR_line q p v s).symm.trans (hin s hs hsr)
  have h0 : qR q p = 0 := hU p (Metric.mem_ball_self hr)
  have e1 := e (r / 4) (by linarith) (by linarith)
  have e2 := e (r / 2) (by linarith) (by linarith)
  have e3 := e (3 * r / 4) (by linarith) (by linarith)
  rw [h0] at e1 e2 e3
  set A := gx q p * v.1 + gy q p * v.2.1 + gz q p * v.2.2
  set Z := qq q v
  have f1 : 0 + A * (r / 4) + Z * (r / 4) * (r / 4) = 0 := by linear_combination e1
  have f2 : 0 + A * (r / 2) + Z * (r / 2) * (r / 2) = 0 := by linear_combination e2
  have f3 : 0 + A * (3 * r / 4) + Z * (3 * r / 4) * (3 * r / 4) = 0 := by linear_combination e3
  obtain ⟨hZ, hA⟩ := quad_three (by intro h; linarith) (by intro h; linarith)
    (by intro h; linarith) f1 f2 f3
  exact ⟨hZ, hA⟩

/-- **A quadric vanishing on an open set is zero.** -/
theorem quad_zero_of_open {q : Quad.Quadric} {U : Set R3} (hU : IsOpen U) {p : R3} (hp : p ∈ U)
    (h : ∀ z ∈ U, qR q z = 0) : nzB q = false := by
  obtain ⟨r, hr, hball⟩ := Metric.isOpen_iff.mp hU p hp
  have hB : ∀ z ∈ Metric.ball p r, qR q z = 0 := fun z hz => h z (hball hz)
  have n1 : ‖((1 : ℝ), (0 : ℝ), (0 : ℝ))‖ ≤ 1 := by simp [Prod.norm_def]
  have n2 : ‖((0 : ℝ), (1 : ℝ), (0 : ℝ))‖ ≤ 1 := by simp [Prod.norm_def]
  have n3 : ‖((0 : ℝ), (0 : ℝ), (1 : ℝ))‖ ≤ 1 := by simp [Prod.norm_def]
  have n12 : ‖((1 : ℝ), (1 : ℝ), (0 : ℝ))‖ ≤ 1 := by simp [Prod.norm_def]
  have n13 : ‖((1 : ℝ), (0 : ℝ), (1 : ℝ))‖ ≤ 1 := by simp [Prod.norm_def]
  have n23 : ‖((0 : ℝ), (1 : ℝ), (1 : ℝ))‖ ≤ 1 := by simp [Prod.norm_def]
  obtain ⟨a1, b1⟩ := along_zero hr hB _ n1
  obtain ⟨a2, b2⟩ := along_zero hr hB _ n2
  obtain ⟨a3, b3⟩ := along_zero hr hB _ n3
  obtain ⟨a12, -⟩ := along_zero hr hB _ n12
  obtain ⟨a13, -⟩ := along_zero hr hB _ n13
  obtain ⟨a23, -⟩ := along_zero hr hB _ n23
  simp only [qq, mul_one, mul_zero, add_zero, zero_add] at a1 a2 a3 a12 a13 a23 b1 b2 b3
  have hxx : (q.xx : ℝ) = 0 := a1
  have hyy : (q.yy : ℝ) = 0 := a2
  have hzz : (q.zz : ℝ) = 0 := a3
  have hxy : (q.xy : ℝ) = 0 := by linarith
  have hxz : (q.xz : ℝ) = 0 := by linarith
  have hyz : (q.yz : ℝ) = 0 := by linarith
  simp only [gx, gy, gz, hxx, hyy, hzz, hxy, hxz, hyz, mul_zero, zero_mul, zero_add] at b1 b2 b3
  have h0 := hB p (Metric.mem_ball_self hr)
  simp only [qR, hxx, hyy, hzz, hxy, hxz, hyz, b1, b2, b3, zero_mul, zero_add] at h0
  have c1 : q.xx = 0 := by exact_mod_cast hxx
  have c2 : q.yy = 0 := by exact_mod_cast hyy
  have c3 : q.zz = 0 := by exact_mod_cast hzz
  have c4 : q.xy = 0 := by exact_mod_cast hxy
  have c5 : q.xz = 0 := by exact_mod_cast hxz
  have c6 : q.yz = 0 := by exact_mod_cast hyz
  have c7 : q.lx = 0 := by exact_mod_cast b1
  have c8 : q.ly = 0 := by exact_mod_cast b2
  have c9 : q.lz = 0 := by exact_mod_cast b3
  have c10 : q.c = 0 := by exact_mod_cast h0
  simp [nzB, c1, c2, c3, c4, c5, c6, c7, c8, c9, c10]

theorem interior_zero {q : Quad.Quadric} (hq : nzB q = true) : interior {z | qR q z = 0} = ∅ := by
  rw [eq_empty_iff_forall_notMem]
  intro p hp
  have := quad_zero_of_open isOpen_interior hp (fun z hz => interior_subset hz)
  rw [hq] at this; cases this

theorem union_zero : ∀ {qs : List Quad.Quadric}, qs.all nzB = true →
    IsClosed (⋃ q ∈ qs, {z : R3 | qR q z = 0}) ∧ interior (⋃ q ∈ qs, {z : R3 | qR q z = 0}) = ∅
  | [], _ => by simp
  | q :: qs, h => by
    simp only [List.all_cons, Bool.and_eq_true] at h
    obtain ⟨hc, hi⟩ := union_zero h.2
    simp only [List.mem_cons, iUnion_iUnion_eq_or_left]
    have hcl : IsClosed {z | qR q z = 0} := isClosed_eq (qR_cont q) continuous_const
    refine ⟨hcl.union hc, ?_⟩
    rw [union_comm, interior_union_isClosed_of_interior_empty hc (interior_zero h.1)]
    exact hi

/-- **The wall of nonzero quadrics is nowhere dense.** -/
theorem wallND_q {qs : List Quad.Quadric} (h : qs.all nzB = true) :
    ForgeExact.Topology.NowhereDense (wallK (qs.map qR)) := by
  rw [nowhereDense_iff]
  have hW : {y | wallK (qs.map qR) y} = ⋃ q ∈ qs, {z | qR q z = 0} := by
    ext y
    simp only [wallK, mem_ofPred_eq, List.mem_map, mem_iUnion, exists_prop]
    constructor
    · rintro ⟨c, ⟨q, hq, rfl⟩, h0⟩; exact ⟨q, hq, sgnK_zero_iff.mp h0⟩
    · rintro ⟨q, hq, h0⟩; exact ⟨qR q, ⟨q, hq, rfl⟩, sgnK_zero_iff.mpr h0⟩
  rw [hW, closure_eq_iff_isClosed.mpr (union_zero h).1]
  exact (union_zero h).2

end ForgeCurved
