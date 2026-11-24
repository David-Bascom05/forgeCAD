import ForgeCertify.Sound

/-!
# The tied points lie on finitely many planes

`walls` lists, from the checker's data, a plane for every way a point can be tied (a crossing
through a vertex's level or along a segment's line, two planes met at one position, the line's point
on a plane on another surface, a sweep through a vertex's shadow). `gen_of_walls` : off all of them,
and when the checker accepts, `Gen` holds.
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-- `z = v.z`. -/
def zPlane (v : V3) : Plane := ⟨⟨0, 0, 1⟩, v.z⟩

/-- The points whose yz projection is on the line `B C`. -/
def yzPlane (B C : P2) : Plane := ⟨⟨0, B.v - C.v, C.u - B.u⟩, -(B.u * C.v - B.v * C.u)⟩

theorem yzPlane_eval (B C : P2) (x : V3) : (yzPlane B C).eval x = o2 (yz x) B C := by
  simp only [yzPlane, Plane.eval, V3.dot, o2, yz]; grind

/-- `tpos P - tpos P'`. -/
def diffPlane (P P' : Plane) : Plane := ⟨⟨0, P'.n.y - P.n.y, P'.n.z - P.n.z⟩, P'.c - P.c⟩

theorem diffPlane_eval (P P' : Plane) (x : V3) : (diffPlane P P').eval x = tpos x P - tpos x P' := by
  simp only [diffPlane, Plane.eval, V3.dot, tpos, eP]; grind

/-- `Q`'s value where the `+x` line meets `P`. -/
def liftPlane (P Q : Plane) : Plane :=
  ⟨⟨0, Q.n.y - Q.n.x * P.n.y, Q.n.z - Q.n.x * P.n.z⟩, Q.c - Q.n.x * P.c⟩

theorem liftPlane_eval (P Q : Plane) (x : V3) : (liftPlane P Q).eval x = Q.eval (eP P x) := by
  simp only [liftPlane, Plane.eval, V3.dot, eP]; grind

/-- For a plane along `x`: the yz point of the plane with `v`'s in-plane level. -/
def mPoint (Q : Plane) (v : V3) : P2 :=
  if Q.n.z ≠ 0 then ⟨v.y, (Q.c - Q.n.y * v.y) / Q.n.z⟩ else ⟨Q.c / Q.n.y, v.z⟩

def vertsOf (t : Tri) : List V3 := [t.a, t.b, t.c]

/-- Every plane a tie lies on. -/
def rawWalls (ins : List Opd) (res : List Face) : List Plane :=
  let L := planes ins res
  let S := allSoup ins
  ((allSegs ins res).flatMap fun s => [zPlane s.1, zPlane s.2, yzPlane (yz s.1) (yz s.2)]) ++
  (L.flatMap fun P => L.map fun P' => diffPlane P P') ++
  (L.flatMap fun P => S.map fun t => liftPlane P t.1) ++
  (L.flatMap fun P => (atoms ins P).flatMap fun τ =>
    let w := witness ins P τ
    S.flatMap fun t => (vertsOf t.2).flatMap fun v => [yzPlane (yz w) (yz v), yzPlane (yz w) (mPoint t.1 v)])

def walls (ins : List Opd) (res : List Face) : List Plane :=
  (rawWalls ins res).filter fun Q => nonzero Q.n

theorem walls_nonzero (ins : List Opd) (res : List Face) : ∀ Q ∈ walls ins res, Q.n ≠ ⟨0, 0, 0⟩ := by
  intro Q hQ
  have := (List.mem_filter.mp hQ).2
  intro e; rw [e] at this; simp [nonzero] at this

/-- Off the walls, a raw wall is either nonzero there or has no normal. -/
theorem off_raw {ins : List Opd} {res : List Face} {x : V3} (hx : ∀ Q ∈ walls ins res, Q.eval x ≠ 0)
    {Q : Plane} (hQ : Q ∈ rawWalls ins res) (hn : Q.n ≠ ⟨0, 0, 0⟩) : Q.eval x ≠ 0 :=
  hx Q (List.mem_filter.mpr ⟨hQ, by
    cases h : Q.n with
    | mk a b c =>
      rw [h] at hn; simp only [nonzero, Bool.not_eq_true']
      by_cases ha : a = 0 <;> by_cases hb : b = 0 <;> by_cases hc : c = 0 <;> simp_all⟩)

theorem yzPlane_n (B C : P2) (h : B ≠ C) : (yzPlane B C).n ≠ ⟨0, 0, 0⟩ := by
  intro e
  simp only [yzPlane, V3.mk.injEq] at e
  apply h; cases B; cases C; simp only [P2.mk.injEq]; constructor <;> grind

theorem zPlane_n (v : V3) : (zPlane v).n ≠ ⟨0, 0, 0⟩ := by simp [zPlane]

/-- Along the sweep from `e` to `w`, a point with `v`'s projection puts `v` on their line. -/
theorem sweep_collinear (e w : V3) (V : P2) (s : Rat) (h : yz (seg e w s) = V) : o2 (yz e) (yz w) V = 0 := by
  rw [← h]; simp only [yz, seg]; unfold o2; simp; grind

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

theorem mem_raw_seg {ins : List Opd} {res : List Face} {s : Seg} (h : s ∈ allSegs ins res) :
    zPlane s.1 ∈ rawWalls ins res ∧ zPlane s.2 ∈ rawWalls ins res ∧
      yzPlane (yz s.1) (yz s.2) ∈ rawWalls ins res := by
  unfold rawWalls; simp only [List.mem_append, List.mem_flatMap]
  refine ⟨?_, ?_, ?_⟩ <;> exact Or.inl (Or.inl (Or.inl ⟨s, h, by simp⟩))

theorem mem_raw_diff {ins : List Opd} {res : List Face} {P P' : Plane} (h : P ∈ planes ins res)
    (h' : P' ∈ planes ins res) : diffPlane P P' ∈ rawWalls ins res := by
  unfold rawWalls; simp only [List.mem_append, List.mem_flatMap, List.mem_map]
  exact Or.inl (Or.inl (Or.inr ⟨P, h, P', h', rfl⟩))

theorem mem_raw_lift {ins : List Opd} {res : List Face} {P : Plane} {t : Plane × Tri} (h : P ∈ planes ins res)
    (ht : t ∈ allSoup ins) : liftPlane P t.1 ∈ rawWalls ins res := by
  unfold rawWalls; simp only [List.mem_append, List.mem_flatMap, List.mem_map]
  exact Or.inl (Or.inr ⟨P, h, t, ht, rfl⟩)

theorem mem_raw_w {ins : List Opd} {res : List Face} {P : Plane} {τ : Tri} {t : Plane × Tri} {v : V3}
    (h : P ∈ planes ins res) (hτ : τ ∈ atoms ins P) (ht : t ∈ allSoup ins) (hv : v ∈ vertsOf t.2) :
    yzPlane (yz (witness ins P τ)) (yz v) ∈ rawWalls ins res ∧
      yzPlane (yz (witness ins P τ)) (mPoint t.1 v) ∈ rawWalls ins res := by
  unfold rawWalls; simp only [List.mem_append, List.mem_flatMap]
  constructor <;> exact Or.inr ⟨P, h, τ, hτ, t, ht, v, hv, by simp⟩

theorem n_zero {n : V3} (a : n.x = 0) (b : n.y = 0) (c : n.z = 0) : n = ⟨0, 0, 0⟩ := by
  cases n; simp_all

theorem canon_of_scaled (P Q : Plane) (hP : P.n.x = 1) (k : Rat) (hk : k ≠ 0) (hx : Q.n.x = k)
    (hy : Q.n.y = k * P.n.y) (hz : Q.n.z = k * P.n.z) (hc : Q.c = k * P.c) : Q.canon = P := by
  have hn : Q.canon.n = (1 / k) * Q.n := by unfold Plane.canon; simp [hx, hk]
  have hcc : Q.canon.c = Q.c / k := by unfold Plane.canon; simp [hx, hk]
  have i : 1 / k * k = 1 := Rat.div_mul_cancel (a := 1) hk
  obtain ⟨⟨a, b, c⟩, d⟩ := P
  obtain ⟨⟨a', b', c'⟩, d'⟩ := Q
  simp only at hP hx hy hz hc hn hcc
  subst hP hx hy hz hc
  have e : ∀ X : Plane, X = ⟨⟨X.n.x, X.n.y, X.n.z⟩, X.c⟩ := fun X => rfl
  rw [e (Plane.canon _), hn, hcc]
  simp only [v3_smul_x, v3_smul_y, v3_smul_z, Plane.mk.injEq, V3.mk.injEq]
  refine ⟨⟨i, ?_, ?_⟩, ?_⟩
  · rw [← Rat.mul_assoc, i, Rat.one_mul]
  · rw [← Rat.mul_assoc, i, Rat.one_mul]
  · rw [Rat.mul_comm, Rat.mul_div_cancel hk]

/-- **Off the walls, and when the checker accepts, nothing is tied.** -/
theorem gen_of_walls (op : List Bool → Bool) (ins : List Opd) (res : List Face) (hc : check op ins res = true)
    (x : V3) (hx : ∀ Q ∈ walls ins res, Q.eval x ≠ 0) : Gen ins res x := by
  unfold check at hc
  simp only [Bool.and_eq_true, beq_iff_eq] at hc
  obtain ⟨⟨⟨-, ht⟩, -⟩, hpl⟩ := hc
  rw [List.all_eq_true] at hpl
  have hatom : ∀ P ∈ planes ins res, ∀ τ ∈ atoms ins P, atomOK ins P τ = true := by
    intro P hP τ hτ
    have := hpl P hP; simp only [Bool.and_eq_true, List.all_eq_true] at this
    exact this.1.2 τ hτ
  have hw : ∀ P ∈ planes ins res, ∀ τ ∈ atoms ins P, ∀ t ∈ allSoup ins, offP P t = true →
      witnessOK (witness ins P τ) t.1 t.2 = true := by
    intro P hP τ hτ t htm hoff
    have := hatom P hP τ hτ
    unfold atomOK at this; simp only [List.all_eq_true] at this
    have := this t htm; rw [hoff] at this
    simp only [Bool.not_true, Bool.false_or, Bool.and_eq_true] at this; exact this.1
  refine ⟨fun s hs => ?_, fun P hP P' hP' he => ?_, fun P hP t htm hoff => ?_, ?_, ?_⟩
  · -- crossings
    obtain ⟨m1, m2, m3⟩ := mem_raw_seg hs
    have z1 := off_raw hx m1 (zPlane_n _)
    have z2 := off_raw hx m2 (zPlane_n _)
    refine ⟨fun e => z1 (by simp [zPlane, Plane.eval, V3.dot, e]; grind),
      fun e => z2 (by simp [zPlane, Plane.eval, V3.dot, e]; grind),
      fun hne => ?_⟩
    have := off_raw hx m3 (yzPlane_n _ _ hne)
    rwa [yzPlane_eval] at this
  · -- distinct planes, distinct positions
    have h1 := planes_nx hP; have h2 := planes_nx hP'
    by_cases hn : (diffPlane P P').n = ⟨0, 0, 0⟩
    · simp only [diffPlane, V3.mk.injEq] at hn
      have := diffPlane_eval P P' x
      have hc' : P.c = P'.c := by
        simp only [diffPlane, Plane.eval, V3.dot, tpos, eP] at this he; grind
      obtain ⟨⟨a, b, c⟩, d⟩ := P; obtain ⟨⟨a', b', c'⟩, d'⟩ := P'
      simp only at h1 h2 hn hc' ⊢
      simp only [Plane.mk.injEq, V3.mk.injEq]; refine ⟨⟨by grind, by grind, by grind⟩, hc'⟩
    · have := off_raw hx (mem_raw_diff hP hP') hn
      rw [diffPlane_eval] at this; exact absurd (by grind) this
  · -- the line's point is off every other surface
    have hPx := planes_nx hP
    rw [← liftPlane_eval]
    by_cases hn : (liftPlane P t.1).n = ⟨0, 0, 0⟩
    · simp only [liftPlane, V3.mk.injEq] at hn
      intro h0
      rw [liftPlane_eval] at h0
      obtain ⟨hn0, -, -⟩ := allSoup_ok ht htm
      by_cases hk : t.1.n.x = 0
      · apply hn0
        rw [hk] at hn; exact n_zero hk (by grind) (by grind)
      · have hcc : t.1.c = t.1.n.x * P.c := by
          simp only [Plane.eval, V3.dot, eP] at h0; grind
        have := canon_of_scaled P t.1 hPx t.1.n.x hk rfl (by grind) (by grind) hcc
        have hon : onP P t = true := by simp [onP, nonvert, hk, this]
        rw [offP_onP, hon] at hoff; simp at hoff
    · rw [liftPlane_eval] at *; have := off_raw hx (mem_raw_lift hP htm) hn; rwa [liftPlane_eval] at this
  · -- the sweep misses every vertex's projection
    intro P hP τ hτ t htm hoff s h0 h1
    have hwok := hw P hP τ hτ t htm hoff
    unfold witnessOK at hwok
    simp only [Bool.and_eq_true, bne_iff_ne, ne_eq] at hwok
    obtain ⟨⟨⟨⟨-, na⟩, nb⟩, nc⟩, -⟩ := hwok
    have one : ∀ v ∈ vertsOf t.2, yz (witness ins P τ) ≠ yz v → yz (seg (eP P x) (witness ins P τ) s) ≠ yz v := by
      intro v hv hne e
      have col := sweep_collinear _ _ _ s e
      have := off_raw hx (mem_raw_w hP hτ htm hv).1 (yzPlane_n _ _ hne)
      rw [yzPlane_eval, ← yz_eP P x] at this; exact this col
    exact ⟨one _ (by simp [vertsOf]) na, one _ (by simp [vertsOf]) nb, one _ (by simp [vertsOf]) nc⟩
  · -- for a plane along x: the sweep's crossing of it misses every vertex's level
    intro P hP τ hτ t htm hoff hv s h0 h1 hq
    have hwok := hw P hP τ hτ t htm hoff
    obtain ⟨qw, -⟩ := witnessOK_sound hwok
    obtain ⟨hn0, -, -⟩ := allSoup_ok ht htm
    have hnx : t.1.n.x = 0 := by simp [nonvert] at hv; exact hv
    have one : ∀ v ∈ vertsOf t.2, (piv t.1 (seg (eP P x) (witness ins P τ) s)).v ≠ (piv t.1 v).v := by
      intro v hvm hlev
      -- the sweep's point is the plane's point at that level
      have hm : yz (seg (eP P x) (witness ins P τ) s) = mPoint t.1 v := by
        simp only [Plane.eval, V3.dot, hnx] at hq
        unfold piv at hlev; unfold mPoint
        by_cases hz : t.1.n.z ≠ 0
        · simp only [hz, ne_eq, not_false_eq_true, ite_true] at hlev ⊢
          simp only [yz, P2.mk.injEq]
          refine ⟨hlev, ?_⟩
          have : (seg (eP P x) (witness ins P τ) s).z * t.1.n.z = t.1.c - t.1.n.y * v.y := by
            rw [← hlev]; grind
          rw [← this, Rat.mul_div_cancel (by simpa using hz)]
        · have hz' : t.1.n.z = 0 := by simpa using hz
          simp only [hz, ite_false] at hlev ⊢
          have hy : t.1.n.y ≠ 0 := by
            intro hy; exact hn0 (n_zero hnx hy hz')
          simp only [yz, P2.mk.injEq]
          refine ⟨?_, hlev⟩
          have : (seg (eP P x) (witness ins P τ) s).y * t.1.n.y = t.1.c := by rw [hz'] at hq; grind
          rw [← this, Rat.mul_div_cancel hy]
      have hne : yz (witness ins P τ) ≠ mPoint t.1 v := by
        intro e; apply qw
        simp only [Plane.eval, V3.dot, hnx]
        unfold mPoint at e
        by_cases hz : t.1.n.z ≠ 0
        · simp only [hz, ne_eq, not_false_eq_true, ite_true, yz, P2.mk.injEq] at e
          rw [e.2, e.1]
          have := Rat.div_mul_cancel (a := t.1.c - t.1.n.y * v.y) (by simpa using hz)
          grind
        · have hz' : t.1.n.z = 0 := by simpa using hz
          simp only [hz, ite_false, yz, P2.mk.injEq] at e
          have hy : t.1.n.y ≠ 0 := by
            intro hy; exact hn0 (n_zero hnx hy hz')
          rw [e.1, hz']
          have := Rat.div_mul_cancel (a := t.1.c) hy
          grind
      have col := sweep_collinear _ _ _ s hm
      have := off_raw hx (mem_raw_w hP hτ htm hvm).2 (yzPlane_n _ _ hne)
      rw [yzPlane_eval, ← yz_eP P x] at this; exact this col
    exact ⟨one _ (by simp [vertsOf]), one _ (by simp [vertsOf]), one _ (by simp [vertsOf])⟩

end ForgeCertify
