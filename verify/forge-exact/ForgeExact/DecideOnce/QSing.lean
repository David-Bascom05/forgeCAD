import ForgeExact.DecideOnce.QSweep

/-!
# Q1 L2: singular points lie on other carriers, certified

A facet cell is on one carrier only. The sweep's local arguments need that carrier to be smooth
there, with a nonzero gradient. **The certificate** (`SingCert`, checked by `singOk`) says every
point where a carrier vanishes with zero gradient lies on another carrier. A cone's apex lies on
its axial seams, so it is a vertex, never a facet point.

* `plane`: a plane's gradient is a nonzero constant;
* `point p₀`: `∇q(p₀) = 0` and `q(p₀) ≠ 0`. `q` is constant on its singular set (`sing_const`), so no
  singular point is on the surface;
* `apex p₀ m`: `∇q(p₀) = 0`, `det H ≠ 0` (the only singular point), and carrier `m ≠ j` vanishes
  at `p₀`;
* `none w`: `H w = 0` and `w · l ≠ 0`, so `H p = −l` has no solution.

**`singOk_spec`**: under a checked certificate, a singular zero of carrier `j` is a zero of
another carrier, in every model of the reals.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef ForgeExact.DecideOnce
  Lean.Grind

/-- The gradient at a rational point. -/
def gradQ (q : Quadric) (p : V3) : V3 :=
  ⟨2 * q.xx * p.x + q.xy * p.y + q.xz * p.z + q.lx, q.xy * p.x + 2 * q.yy * p.y + q.yz * p.z + q.ly,
   q.xz * p.x + q.yz * p.y + 2 * q.zz * p.z + q.lz⟩

/-- `H w` (the gradient's linear part). -/
def hessQ (q : Quadric) (w : V3) : V3 :=
  ⟨2 * q.xx * w.x + q.xy * w.y + q.xz * w.z, q.xy * w.x + 2 * q.yy * w.y + q.yz * w.z,
   q.xz * w.x + q.yz * w.y + 2 * q.zz * w.z⟩

def detH (q : Quadric) : Rat :=
  2 * q.xx * (2 * q.yy * (2 * q.zz) - q.yz * q.yz) - q.xy * (q.xy * (2 * q.zz) - q.yz * q.xz) +
    q.xz * (q.xy * q.yz - 2 * q.yy * q.xz)

def evQ (q : Quadric) (p : V3) : Rat :=
  q.xx * p.x * p.x + q.yy * p.y * p.y + q.zz * p.z * p.z + q.xy * p.x * p.y + q.xz * p.x * p.z +
    q.yz * p.y * p.z + q.lx * p.x + q.ly * p.y + q.lz * p.z + q.c

inductive SingCert
  | plane
  | point (p0 : V3)
  | apex (p0 : V3) (m : Nat)
  | none_ (w : V3)
  deriving Repr

def SingCert.ok (qs : List Quadric) (j : Nat) (q : Quadric) : SingCert → Bool
  | .plane => isPlaneQ q && !(q.lx == 0 && q.ly == 0 && q.lz == 0)
  | .point p0 => (gradQ q p0).isZero && evQ q p0 != 0
  | .apex p0 m => (gradQ q p0).isZero && detH q != 0 && m != j &&
      (match qs[m]? with | some r => evQ r p0 == 0 | none => false)
  | .none_ w => (hessQ q w).isZero && (w.x * q.lx + w.y * q.ly + w.z * q.lz != 0)

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- The gradient in a model. -/
def dxK (M : Model K) (q : Quadric) (p : PK K) : K :=
  (1 + 1) * M.q q.xx * p.1 + M.q q.xy * p.2.1 + M.q q.xz * p.2.2 + M.q q.lx

/-- `p` is a singular point of `q`. -/
def SingAt (M : Model K) (q : Quadric) (p : PK K) : Prop :=
  dxK M q p = 0 ∧ dyK M q p = 0 ∧ dzK M q p = 0


theorem q_two' : M.q 2 = 1 + 1 := by
  rw [show (2 : Rat) = 1 + 1 by grind, Model.q_add, Model.q_one]

/-- **`q` is constant on its singular set**: `q(p) − q(p₀) = ∇q(p₀)·v + ½ v·(∇q(p) − ∇q(p₀))`. -/
theorem sing_const (q : Quadric) (p0 : V3) (hp0 : (gradQ q p0).isZero = true) {p : PK K}
    (hp : SingAt M q p) : Quadric.evK M q p = M.q (evQ q p0) := by
  obtain ⟨x, y, z⟩ := p
  obtain ⟨hx, hy, hz⟩ := hp
  simp only [V3.isZero, gradQ, Bool.and_eq_true, beq_iff_eq] at hp0
  obtain ⟨⟨g1, g2⟩, g3⟩ := hp0
  have e1 := congrArg M.q g1; have e2 := congrArg M.q g2; have e3 := congrArg M.q g3
  simp only [Model.q_add, Model.q_mul, Model.q_zero, q_two'] at e1 e2 e3
  simp only [dxK, dyK, dzK] at hx hy hz
  simp only [Quadric.evK, evQ, Model.q_add, Model.q_mul]
  generalize M.q q.xx = a at *; generalize M.q q.yy = b at *; generalize M.q q.zz = c at *
  generalize M.q q.xy = d at *; generalize M.q q.xz = e at *; generalize M.q q.yz = f at *
  generalize M.q q.lx = l1 at *; generalize M.q q.ly = l2 at *; generalize M.q q.lz = l3 at *
  generalize M.q p0.x = u at *; generalize M.q p0.y = v at *; generalize M.q p0.z = w at *
  grind

theorem sing_unique (q : Quadric) (hd : detH q ≠ 0) (p0 : V3) (hp0 : (gradQ q p0).isZero = true)
    {p : PK K} (hp : SingAt M q p) : p = (M.q p0.x, M.q p0.y, M.q p0.z) := by
  obtain ⟨x, y, z⟩ := p
  obtain ⟨hx, hy, hz⟩ := hp
  simp only [V3.isZero, gradQ, Bool.and_eq_true, beq_iff_eq] at hp0
  obtain ⟨⟨g1, g2⟩, g3⟩ := hp0
  have e1 := congrArg M.q g1; have e2 := congrArg M.q g2; have e3 := congrArg M.q g3
  simp only [Model.q_add, Model.q_mul, Model.q_zero, q_two'] at e1 e2 e3
  simp only [dxK, dyK, dzK] at hx hy hz
  have hd' : M.q (detH q) ≠ 0 := fun h0 => hd ((Model.q_eq M).mp (by rw [h0, Model.q_zero]))
  simp only [detH, Model.q_add, Model.q_mul, Model.q_sub, q_two'] at hd'
  generalize M.q q.xx = a at *; generalize M.q q.yy = b at *; generalize M.q q.zz = c at *
  generalize M.q q.xy = d at *; generalize M.q q.xz = e at *; generalize M.q q.yz = f at *
  generalize M.q q.lx = l1 at *; generalize M.q q.ly = l2 at *; generalize M.q q.lz = l3 at *
  generalize M.q p0.x = u at *; generalize M.q p0.y = v at *; generalize M.q p0.z = w at *
  -- H (p − p₀) = 0 and det H ≠ 0
  have k1 : (1 + 1) * a * (x - u) + d * (y - v) + e * (z - w) = 0 := by grind
  have k2 : d * (x - u) + (1 + 1) * b * (y - v) + f * (z - w) = 0 := by grind
  have k3 : e * (x - u) + f * (y - v) + (1 + 1) * c * (z - w) = 0 := by grind
  obtain ⟨D, hD⟩ : ∃ D, D = (1 + 1) * a * ((1 + 1) * b * ((1 + 1) * c) - f * f) -
      d * (d * ((1 + 1) * c) - f * e) + e * (d * f - (1 + 1) * b * e) := ⟨_, rfl⟩
  have c1 : D * (x - u) = 0 := by
    have : D * (x - u) = ((1 + 1) * b * ((1 + 1) * c) - f * f) * ((1 + 1) * a * (x - u) + d * (y - v) + e * (z - w)) -
        (d * ((1 + 1) * c) - e * f) * (d * (x - u) + (1 + 1) * b * (y - v) + f * (z - w)) +
        (d * f - e * ((1 + 1) * b)) * (e * (x - u) + f * (y - v) + (1 + 1) * c * (z - w)) := by
      rw [hD]; grind
    rw [this, k1, k2, k3]; grind
  have c2 : D * (y - v) = 0 := by
    have : D * (y - v) = -(d * ((1 + 1) * c) - f * e) * ((1 + 1) * a * (x - u) + d * (y - v) + e * (z - w)) +
        ((1 + 1) * a * ((1 + 1) * c) - e * e) * (d * (x - u) + (1 + 1) * b * (y - v) + f * (z - w)) -
        ((1 + 1) * a * f - d * e) * (e * (x - u) + f * (y - v) + (1 + 1) * c * (z - w)) := by
      rw [hD]; grind
    rw [this, k1, k2, k3]; grind
  have c3 : D * (z - w) = 0 := by
    have : D * (z - w) = (d * f - (1 + 1) * b * e) * ((1 + 1) * a * (x - u) + d * (y - v) + e * (z - w)) -
        ((1 + 1) * a * f - e * d) * (d * (x - u) + (1 + 1) * b * (y - v) + f * (z - w)) +
        ((1 + 1) * a * ((1 + 1) * b) - d * d) * (e * (x - u) + f * (y - v) + (1 + 1) * c * (z - w)) := by
      rw [hD]; grind
    rw [this, k1, k2, k3]; grind
  have hD0 : D ≠ 0 := by rw [hD]; grind
  have r1 : x - u = 0 := by
    have hinv : D * D⁻¹ = 1 := Field.mul_inv_cancel hD0
    calc x - u = D⁻¹ * (D * (x - u)) := by grind
      _ = 0 := by rw [c1]; grind
  have r2 : y - v = 0 := by
    have hinv : D * D⁻¹ = 1 := Field.mul_inv_cancel hD0
    calc y - v = D⁻¹ * (D * (y - v)) := by grind
      _ = 0 := by rw [c2]; grind
  have r3 : z - w = 0 := by
    have hinv : D * D⁻¹ = 1 := Field.mul_inv_cancel hD0
    calc z - w = D⁻¹ * (D * (z - w)) := by grind
      _ = 0 := by rw [c3]; grind
  simp only [Prod.mk.injEq]
  exact ⟨by grind, by grind, by grind⟩

theorem evK_rat (q : Quadric) (p0 : V3) :
    Quadric.evK M q (M.q p0.x, M.q p0.y, M.q p0.z) = M.q (evQ q p0) := by
  simp only [Quadric.evK, evQ, Model.q_add, Model.q_mul]

/-- **Under a checked certificate, a singular zero of carrier `j` is a zero of another carrier.** -/
theorem singOk_spec {qs : List Quadric} {j : Nat} {q : Quadric} {c : SingCert}
    (hok : c.ok qs j q = true) {p : PK K} (hs : SingAt M q p) (h0 : Quadric.evK M q p = 0) :
    ∃ m, m ≠ j ∧ ∃ r, qs[m]? = some r ∧ Quadric.evK M r p = 0 := by
  cases c with
  | plane =>
    exfalso
    simp only [SingCert.ok, Bool.and_eq_true, Bool.not_eq_true', Bool.and_eq_false_iff,
      beq_iff_eq] at hok
    obtain ⟨hp, hl⟩ := hok
    obtain ⟨h1, h2, h3, h4, h5, h6⟩ := isPlaneQ_iff hp
    obtain ⟨hx, hy, hz⟩ := hs
    simp only [dxK, dyK, dzK, h1, h2, h3, h4, h5, h6, Model.q_zero] at hx hy hz
    have ex : M.q q.lx = M.q 0 := by rw [Model.q_zero]; grind
    have ey : M.q q.ly = M.q 0 := by rw [Model.q_zero]; grind
    have ez : M.q q.lz = M.q 0 := by rw [Model.q_zero]; grind
    rw [Model.q_eq] at ex ey ez
    rcases hl with (hl | hl) | hl
    · simp [ex] at hl
    · simp [ey] at hl
    · simp [ez] at hl
  | point p0 =>
    exfalso
    simp only [SingCert.ok, Bool.and_eq_true, bne_iff_ne, ne_eq] at hok
    rw [sing_const M q p0 hok.1 hs] at h0
    exact hok.2 ((Model.q_eq M).mp (by rw [h0, Model.q_zero]))
  | apex p0 m =>
    simp only [SingCert.ok, Bool.and_eq_true, bne_iff_ne, ne_eq] at hok
    obtain ⟨⟨⟨hg, hd⟩, hm⟩, hr⟩ := hok
    have hpe := sing_unique M q hd p0 hg hs
    cases hq : qs[m]? with
    | none => rw [hq] at hr; cases hr
    | some r =>
      rw [hq] at hr
      simp only [beq_iff_eq] at hr
      refine ⟨m, hm, r, hq, ?_⟩
      rw [hpe, evK_rat, hr, Model.q_zero]
  | none_ w =>
    exfalso
    simp only [SingCert.ok, Bool.and_eq_true, bne_iff_ne, ne_eq, V3.isZero, hessQ,
      beq_iff_eq] at hok
    obtain ⟨⟨⟨h1, h2⟩, h3⟩, hl⟩ := hok
    obtain ⟨x, y, z⟩ := p
    obtain ⟨hx, hy, hz⟩ := hs
    have e1 := congrArg M.q h1; have e2 := congrArg M.q h2; have e3 := congrArg M.q h3
    simp only [Model.q_add, Model.q_mul, Model.q_zero, q_two'] at e1 e2 e3
    simp only [dxK, dyK, dzK] at hx hy hz
    apply hl
    apply (Model.q_eq M).mp
    simp only [Model.q_add, Model.q_mul, Model.q_zero]
    -- w · (H p + l) = (H w) · p + w · l
    have : M.q w.x * ((1 + 1) * M.q q.xx * x + M.q q.xy * y + M.q q.xz * z + M.q q.lx) +
        M.q w.y * (M.q q.xy * x + (1 + 1) * M.q q.yy * y + M.q q.yz * z + M.q q.ly) +
        M.q w.z * (M.q q.xz * x + M.q q.yz * y + (1 + 1) * M.q q.zz * z + M.q q.lz) = 0 := by
      rw [hx, hy, hz]; grind
    grind

end Model

/-! ## The proposer (untrusted) -/

/-- Row-reduce `[A | b | I]` (3×3, rational): a solution of `A p = b` (free variables zero), or a
`y` with `y A = 0` and `y · b ≠ 0`. -/
def solve3 (A : Array (Array Rat)) (b : Array Rat) : Sum V3 V3 := Id.run do
  let mut M : Array (Array Rat) := (List.range 3).toArray.map fun i =>
    A[i]! ++ #[b[i]!] ++ (List.range 3).toArray.map fun k => if k == i then 1 else 0
  let mut row := 0
  let mut piv : Array (Nat × Nat) := #[]
  for col in [0:3] do
    if row < 3 then
      match (List.range (3 - row)).find? (fun k => M[row + k]![col]! != 0) with
      | none => pure ()
      | some k =>
        let r := row + k
        let tmp := M[row]!
        M := (M.set! row M[r]!).set! r tmp
        let pv := M[row]![col]!
        M := M.set! row (M[row]!.map (· / pv))
        for i in [0:3] do
          if i != row then
            let f := M[i]![col]!
            M := M.set! i ((M[i]!.zip M[row]!).map fun (a, c) => a - f * c)
        piv := piv.push (row, col)
        row := row + 1
  -- inconsistent: a zero row of A with a nonzero right side
  for i in [row:3] do
    if M[i]![3]! != 0 then return .inr ⟨M[i]![4]!, M[i]![5]!, M[i]![6]!⟩
  let mut sol : Array Rat := #[0, 0, 0]
  for (r, c) in piv do sol := sol.set! c M[r]![3]!
  return .inl ⟨sol[0]!, sol[1]!, sol[2]!⟩

def proposeSing (qs : List Quadric) (j : Nat) (q : Quadric) : SingCert :=
  if isPlaneQ q then .plane
  else
    let H : Array (Array Rat) := #[#[2 * q.xx, q.xy, q.xz], #[q.xy, 2 * q.yy, q.yz],
      #[q.xz, q.yz, 2 * q.zz]]
    match solve3 H #[-q.lx, -q.ly, -q.lz] with
    | .inr w => .none_ w
    | .inl p0 =>
      if evQ q p0 != 0 then .point p0
      else match (List.range qs.length).find? (fun m => m != j &&
          match qs[m]? with | some r => evQ r p0 == 0 | none => false) with
        | some m => .apex p0 m
        | none => .point p0

/-- **Every carrier's singular zeros lie on other carriers**, certified. -/
def singAll (qs : List Quadric) : Bool :=
  (List.range qs.length).all fun j => match qs[j]? with
    | some q => (proposeSing qs j q).ok qs j q
    | none => true

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem singAll_spec {qs : List Quadric} (h : singAll qs = true) {j : Nat} {q : Quadric}
    (hq : qs[j]? = some q) {p : PK K} (hs : SingAt M q p) (h0 : Quadric.evK M q p = 0) :
    ∃ m, m ≠ j ∧ ∃ r, qs[m]? = some r ∧ Quadric.evK M r p = 0 := by
  have hj : j < qs.length := (List.getElem?_eq_some_iff.mp hq).1
  have := List.all_eq_true.mp h j (List.mem_range.mpr hj)
  rw [hq] at this
  exact singOk_spec M this hs h0

end Model

-- the cone x² + y² = z² is singular at its apex; with the plane y = 0 through it, certified
#guard singAll [⟨1, 1, -1, 0, 0, 0, 0, 0, 0, 0⟩, ⟨0, 0, 0, 0, 0, 0, 0, 1, 0, 0⟩]
-- alone, not: the apex lies on no other carrier
#guard !singAll [⟨1, 1, -1, 0, 0, 0, 0, 0, 0, 0⟩]
-- spheres, cylinders and planes have no singular zeros
#guard singAll [sphere1, ⟨1, 1, 0, 0, 0, 0, 0, 0, 0, -1/4⟩, ⟨0, 0, 0, 0, 0, 0, 1, 2, 3, 4⟩,
  ⟨1, 1, 1, 0, 0, 0, 0, 0, 0, -9⟩]

end ForgeExact.DecideOnce.Curved
