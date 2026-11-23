import ForgeCertify.Tri3

/-!
# Constancy: a triangle's hits do not change along a segment it misses

Along the segment `z s = e + s (w - e)`, `s ∈ [0, 1]`, of points of an atom's plane, a triangle `u`
off that plane changes its `+x` hit only where the ray from `z s` passes through one of its
edges (a *shadow* of the edge, `shadow`), provided the segment misses `u` itself (`pt_*`). Summed
over a closed triangle soup the shadows cancel (`chainZ`), so the soup's hit parity is the same at
both ends: the classification at the atom's witness holds on the whole atom.
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-- The point `s` of the way from `e` to `w`. -/
def seg (e w : V3) (s : Rat) : V3 := e + s * (w - e)

def ex : V3 := ⟨1, 0, 0⟩

/-- The ray from `z s` meets the open edge `b c`, for some `s ∈ (0, 1)`. -/
def shadow (e w : V3) (E : Seg) : Prop :=
  ∃ s μ l : Rat, 0 < s ∧ s < 1 ∧ 0 < μ ∧ μ < 1 ∧ 0 < l ∧ seg e w s + l * ex = E.1 + μ * (E.2 - E.1)

/-- A triangle on plane `Q` is hit by the `+x` ray from `z`. -/
def hit (Q : Plane) (u : Tri) (z : V3) : Bool := ahead Q z && u.ins z

theorem plane_seg (Q : Plane) (e w : V3) (s : Rat) :
    Q.eval (seg e w s) = Q.eval e + s * (Q.eval w - Q.eval e) := by
  simp only [Plane.eval, seg, V3.dot]; simp; grind

theorem yz_seg (e w : V3) (s : Rat) :
    yz (seg e w s) = ⟨(yz e).u + s * ((yz w).u - (yz e).u), (yz e).v + s * ((yz w).v - (yz e).v)⟩ := by
  simp [yz, seg]

theorem o2_seg1 (e w : V3) (s : Rat) (B C : P2) :
    o2 (yz (seg e w s)) B C = o2 (yz e) B C + s * (o2 (yz w) B C - o2 (yz e) B C) := by
  rw [yz_seg]; unfold o2; grind
theorem o2_seg2 (e w : V3) (s : Rat) (A C : P2) :
    o2 A (yz (seg e w s)) C = o2 A (yz e) C + s * (o2 A (yz w) C - o2 A (yz e) C) := by
  rw [yz_seg]; unfold o2; grind
theorem o2_seg3 (e w : V3) (s : Rat) (A B : P2) :
    o2 A B (yz (seg e w s)) = o2 A B (yz e) + s * (o2 A B (yz w) - o2 A B (yz e)) := by
  rw [yz_seg]; unfold o2; grind

theorem seg_zero (e w : V3) : seg e w 0 = e := by
  apply v3_ext <;> simp [seg] <;> grind
theorem seg_one (e w : V3) : seg e w 1 = w := by
  apply v3_ext <;> simp [seg] <;> grind

/-- The affine function with value `v₀` at 0 and `v₁` at 1. -/
def aff01 (v₀ v₁ : Rat) : Aff := ⟨v₀, v₁ - v₀⟩

theorem aff01_ev (v₀ v₁ s : Rat) : (aff01 v₀ v₁).ev s = v₀ + s * (v₁ - v₀) := by
  simp [aff01, Aff.ev]; grind

/-- A point of `Q` (with `n.x ≠ 0`) is the barycentric combination, by its yz sides, of a
triangle of `Q`. -/
theorem lift (Q : Plane) (u : Tri) (p : V3) (hnx : Q.n.x ≠ 0) (ha : Q.eval u.a = 0)
    (hb : Q.eval u.b = 0) (hc : Q.eval u.c = 0) (hp : Q.eval p = 0) :
    u.D * p.x = o2 (yz p) (yz u.b) (yz u.c) * u.a.x + o2 (yz u.a) (yz p) (yz u.c) * u.b.x +
      o2 (yz u.a) (yz u.b) (yz p) * u.c.x ∧
    u.D * p.y = o2 (yz p) (yz u.b) (yz u.c) * u.a.y + o2 (yz u.a) (yz p) (yz u.c) * u.b.y +
      o2 (yz u.a) (yz u.b) (yz p) * u.c.y ∧
    u.D * p.z = o2 (yz p) (yz u.b) (yz u.c) * u.a.z + o2 (yz u.a) (yz p) (yz u.c) * u.b.z +
      o2 (yz u.a) (yz u.b) (yz p) * u.c.z := by
  have hu := bary_u (yz u.a) (yz u.b) (yz u.c) (yz p)
  have hv := bary_v (yz u.a) (yz u.b) (yz u.c) (yz p)
  have hs := bary_sum (yz u.a) (yz u.b) (yz u.c) (yz p)
  simp only [yz] at hu hv hs
  unfold Tri.D
  simp only [yz]
  refine ⟨?_, hu, hv⟩
  simp only [Plane.eval, V3.dot] at ha hb hc hp
  -- n.x (D p.x) = n.x Σ w v.x, both from the plane
  have : Q.n.x * (o2 ⟨u.a.y, u.a.z⟩ ⟨u.b.y, u.b.z⟩ ⟨u.c.y, u.c.z⟩ * p.x) =
      Q.n.x * (o2 ⟨p.y, p.z⟩ ⟨u.b.y, u.b.z⟩ ⟨u.c.y, u.c.z⟩ * u.a.x +
        o2 ⟨u.a.y, u.a.z⟩ ⟨p.y, p.z⟩ ⟨u.c.y, u.c.z⟩ * u.b.x +
        o2 ⟨u.a.y, u.a.z⟩ ⟨u.b.y, u.b.z⟩ ⟨p.y, p.z⟩ * u.c.x) := by
    have e1 : Q.n.x * p.x = Q.c - Q.n.y * p.y - Q.n.z * p.z := by grind
    have e2 : Q.n.x * u.a.x = Q.c - Q.n.y * u.a.y - Q.n.z * u.a.z := by grind
    have e3 : Q.n.x * u.b.x = Q.c - Q.n.y * u.b.y - Q.n.z * u.b.z := by grind
    have e4 : Q.n.x * u.c.x = Q.c - Q.n.y * u.c.y - Q.n.z * u.c.z := by grind
    grind
  grind

/-- So a plane `H` nonpositive at the vertices is nonpositive at a point of `Q` inside. -/
theorem lift_H (Q H : Plane) (u : Tri) (p : V3) (hnx : Q.n.x ≠ 0) (ha : Q.eval u.a = 0)
    (hb : Q.eval u.b = 0) (hc : Q.eval u.c = 0) (hp : Q.eval p = 0) (hin : u.ins p = true)
    (Ha : H.eval u.a ≤ 0) (Hb : H.eval u.b ≤ 0) (Hc : H.eval u.c ≤ 0) : H.eval p ≤ 0 := by
  obtain ⟨lx, ly, lz⟩ := lift Q u p hnx ha hb hc hp
  have hs := bary_sum (yz u.a) (yz u.b) (yz u.c) (yz p)
  simp only [Tri.ins, inside2, Bool.and_eq_true, decide_eq_true_eq] at hin
  obtain ⟨⟨h1, h2⟩, h3⟩ := hin
  have hD : u.D ≠ 0 := by intro h; unfold Tri.D at h; rw [h] at h1; simp at h1
  -- D² H(p) = Σ (D w) H(v)
  have e : u.D * (u.D * H.eval p) =
      u.D * o2 (yz p) (yz u.b) (yz u.c) * H.eval u.a + u.D * o2 (yz u.a) (yz p) (yz u.c) * H.eval u.b +
        u.D * o2 (yz u.a) (yz u.b) (yz p) * H.eval u.c := by
    simp only [Plane.eval, V3.dot]
    unfold Tri.D at *
    have : o2 (yz u.a) (yz u.b) (yz u.c) * (H.n.x * p.x + H.n.y * p.y + H.n.z * p.z - H.c) =
      H.n.x * (o2 (yz u.a) (yz u.b) (yz u.c) * p.x) + H.n.y * (o2 (yz u.a) (yz u.b) (yz u.c) * p.y) +
        H.n.z * (o2 (yz u.a) (yz u.b) (yz u.c) * p.z) - H.c * o2 (yz u.a) (yz u.b) (yz u.c) := by grind
    rw [this, lx, ly, lz, ← hs]; grind
  unfold Tri.D at e hD
  have i1 := Rat.mul_le_mul_of_nonneg_left Ha (Rat.le_of_lt h1)
  have i2 := Rat.mul_le_mul_of_nonneg_left Hb (Rat.le_of_lt h2)
  have i3 := Rat.mul_le_mul_of_nonneg_left Hc (Rat.le_of_lt h3)
  apply Classical.byContradiction; intro hn
  have := Rat.mul_pos (sq_pos' hD) (show 0 < H.eval p by grind)
  grind

theorem H_seg_pos (H : Plane) (e w : V3) (he : 0 < H.eval e) (hw : 0 < H.eval w) (s : Rat)
    (h0 : 0 ≤ s) (h1 : s ≤ 1) : 0 < H.eval (seg e w s) := by
  rw [plane_seg]
  have := (aff01 (H.eval e) (H.eval w)).pos_between (s₀ := 0) (s₁ := 1) (s := s)
    (by rw [aff01_ev]; grind) (by rw [aff01_ev]; grind) h0 h1
  rw [aff01_ev] at this; exact this

/-! ## The general case: a triangle off `P`, not along `x`, no edge on `P` -/

/-- `X` from `D X = α X₁ + β X₂` with `α + β = D`. -/
theorem comb {D α β X X₁ X₂ : Rat} (hD : D ≠ 0) (hs : α + β = D) (h : D * X = α * X₁ + β * X₂) :
    X = X₁ + β / D * (X₂ - X₁) := by
  have k : β / D * D = β := Rat.div_mul_cancel hD
  have : D * (X₁ + β / D * (X₂ - X₁)) = D * X := by
    rw [h, Rat.mul_add, ← Rat.mul_assoc, Rat.mul_comm D (β / D), k]; grind
  grind

/-- A point of `Q` on the yz line of an edge is on the edge's line. -/
theorem on_edge_line (Q : Plane) (u : Tri) (p : V3) (hnx : Q.n.x ≠ 0) (ha : Q.eval u.a = 0)
    (hb : Q.eval u.b = 0) (hc : Q.eval u.c = 0) (hp : Q.eval p = 0) (hD : u.D ≠ 0) :
    (o2 (yz p) (yz u.b) (yz u.c) = 0 → ∃ μ : Rat, p = u.b + μ * (u.c - u.b)) ∧
    (o2 (yz u.a) (yz p) (yz u.c) = 0 → ∃ μ : Rat, p = u.c + μ * (u.a - u.c)) ∧
    (o2 (yz u.a) (yz u.b) (yz p) = 0 → ∃ μ : Rat, p = u.a + μ * (u.b - u.a)) := by
  obtain ⟨lx, ly, lz⟩ := lift Q u p hnx ha hb hc hp
  have hs := bary_sum (yz u.a) (yz u.b) (yz u.c) (yz p)
  unfold Tri.D at hD lx ly lz
  refine ⟨fun h => ?_, fun h => ?_, fun h => ?_⟩
  · rw [h] at lx ly lz hs
    refine ⟨o2 (yz u.a) (yz u.b) (yz p) / o2 (yz u.a) (yz u.b) (yz u.c), v3_ext ?_ ?_ ?_⟩ <;> simp
    · exact comb (α := o2 (yz u.a) (yz p) (yz u.c)) hD (by grind) (by grind)
    · exact comb (α := o2 (yz u.a) (yz p) (yz u.c)) hD (by grind) (by grind)
    · exact comb (α := o2 (yz u.a) (yz p) (yz u.c)) hD (by grind) (by grind)
  · rw [h] at lx ly lz hs
    refine ⟨o2 (yz p) (yz u.b) (yz u.c) / o2 (yz u.a) (yz u.b) (yz u.c), v3_ext ?_ ?_ ?_⟩ <;> simp
    · exact comb (α := o2 (yz u.a) (yz u.b) (yz p)) hD (by grind) (by grind)
    · exact comb (α := o2 (yz u.a) (yz u.b) (yz p)) hD (by grind) (by grind)
    · exact comb (α := o2 (yz u.a) (yz u.b) (yz p)) hD (by grind) (by grind)
  · rw [h] at lx ly lz hs
    refine ⟨o2 (yz u.a) (yz p) (yz u.c) / o2 (yz u.a) (yz u.b) (yz u.c), v3_ext ?_ ?_ ?_⟩ <;> simp
    · exact comb (α := o2 (yz p) (yz u.b) (yz u.c)) hD (by grind) (by grind)
    · exact comb (α := o2 (yz p) (yz u.b) (yz u.c)) hD (by grind) (by grind)
    · exact comb (α := o2 (yz p) (yz u.b) (yz u.c)) hD (by grind) (by grind)

/-- Two sides vanish only at a vertex (both coordinates). -/
theorem two_sides_vertex' (a b c q : P2) (hD : o2 a b c ≠ 0) :
    (o2 q b c = 0 → o2 a q c = 0 → q = c) ∧ (o2 a q c = 0 → o2 a b q = 0 → q = a) ∧
      (o2 q b c = 0 → o2 a b q = 0 → q = b) := by
  have hs := bary_sum a b c q
  have hv := bary_v a b c q
  have hu := bary_u a b c q
  refine ⟨fun h1 h2 => ?_, fun h1 h2 => ?_, fun h1 h2 => ?_⟩
  · rw [h1, h2] at hs hv hu
    cases q; cases c; simp only [P2.mk.injEq]; simp at hv hu hs; constructor <;> grind
  · rw [h1, h2] at hs hv hu
    cases q; cases a; simp only [P2.mk.injEq]; simp at hv hu hs; constructor <;> grind
  · rw [h1, h2] at hs hv hu
    cases q; cases b; simp only [P2.mk.injEq]; simp at hv hu hs; constructor <;> grind

/-- The four functions along the segment: the plane ahead, and the three sides. -/
def ptFns (Q : Plane) (u : Tri) (e w : V3) : List Aff :=
  [aff01 (-Q.eval e * Q.n.x) (-Q.eval w * Q.n.x),
   aff01 (u.D * o2 (yz e) (yz u.b) (yz u.c)) (u.D * o2 (yz w) (yz u.b) (yz u.c)),
   aff01 (u.D * o2 (yz u.a) (yz e) (yz u.c)) (u.D * o2 (yz u.a) (yz w) (yz u.c)),
   aff01 (u.D * o2 (yz u.a) (yz u.b) (yz e)) (u.D * o2 (yz u.a) (yz u.b) (yz w))]

theorem ptFns_ev (Q : Plane) (u : Tri) (e w : V3) (s : Rat) :
    (ptFns Q u e w).map (·.ev s) =
      [-Q.eval (seg e w s) * Q.n.x, u.D * o2 (yz (seg e w s)) (yz u.b) (yz u.c),
       u.D * o2 (yz u.a) (yz (seg e w s)) (yz u.c), u.D * o2 (yz u.a) (yz u.b) (yz (seg e w s))] := by
  simp only [ptFns, List.map_cons, List.map_nil, aff01_ev, plane_seg, o2_seg1, o2_seg2, o2_seg3]
  simp only [List.cons.injEq, and_true]; refine ⟨?_, ?_, ?_, ?_⟩ <;> grind

theorem ahead_eval (Q : Plane) (z : V3) : ahead Q z = decide (0 < -Q.eval z * Q.n.x) := by
  unfold ahead; simp only [Plane.eval]; congr 2; grind

theorem pos_ptFns (Q : Plane) (u : Tri) (e w : V3) (s : Rat) :
    pos (ptFns Q u e w) s = hit Q u (seg e w s) := by
  have := ptFns_ev Q u e w s
  simp only [pos, hit, ahead_eval, Tri.ins, inside2, ptFns, List.all_cons, List.all_nil, Bool.and_true]
    at this ⊢
  simp only [List.map_cons, List.map_nil, List.cons.injEq] at this
  rw [this.1, this.2.1, this.2.2.1, this.2.2.2.1]
  unfold Tri.D; simp [Bool.and_assoc]

theorem ptFns_cyc (Q : Plane) (u : Tri) (e w : V3) :
    ptFns Q ⟨u.b, u.c, u.a⟩ e w = [(ptFns Q u e w)[0]!, (ptFns Q u e w)[2]!, (ptFns Q u e w)[3]!,
      (ptFns Q u e w)[1]!] := by
  simp only [ptFns, Tri.D, List.getElem!_cons_zero, List.getElem!_cons_succ]
  have h1 : o2 (yz u.b) (yz u.c) (yz u.a) = o2 (yz u.a) (yz u.b) (yz u.c) := (o2_cyc _ _ _).symm
  rw [h1]
  have e1 : ∀ x, o2 x (yz u.c) (yz u.a) = o2 (yz u.a) x (yz u.c) := fun x => by unfold o2; grind
  have e2 : ∀ x, o2 (yz u.b) x (yz u.a) = o2 (yz u.a) (yz u.b) x := fun x => by unfold o2; grind
  have e3 : ∀ x, o2 (yz u.b) (yz u.c) x = o2 x (yz u.b) (yz u.c) := fun x => by unfold o2; grind
  rw [e1, e1, e2, e2, e3, e3]

/-- The hypotheses on the segment that nothing is tied. -/
structure PTGen (Q : Plane) (u : Tri) (e w : V3) : Prop where
  qe : Q.eval e ≠ 0
  qw : Q.eval w ≠ 0
  le : o2 (yz e) (yz u.b) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz e) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz u.b) (yz e) ≠ 0
  lw : o2 (yz w) (yz u.b) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz w) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz u.b) (yz w) ≠ 0
  verts : ∀ s, 0 < s → s < 1 → yz (seg e w s) ≠ yz u.a ∧ yz (seg e w s) ≠ yz u.b ∧ yz (seg e w s) ≠ yz u.c

theorem nonpos4_second (g₀ g₁ g₂ g₃ : Aff) (r : Rat) (h : g₁.ev r = 0) :
    nonpos [g₀, g₁, g₂, g₃] r = 1 ↔ (0 < g₀.ev r ∧ 0 < g₂.ev r ∧ 0 < g₃.ev r) := by
  simp only [nonpos, List.countP_cons, List.countP_nil, h, Rat.le_refl, decide_true, ite_true]
  by_cases a : 0 < g₀.ev r <;> by_cases b : 0 < g₂.ev r <;> by_cases c : 0 < g₃.ev r <;>
    simp [a, b, c, show (g₀.ev r ≤ 0 ↔ ¬ 0 < g₀.ev r) from by grind,
      show (g₂.ev r ≤ 0 ↔ ¬ 0 < g₂.ev r) from by grind, show (g₃.ev r ≤ 0 ↔ ¬ 0 < g₃.ev r) from by grind]

/-- The first side's root is an event iff the edge `b c` casts a shadow on the segment. -/
theorem pt_ev_A (Q : Plane) (u : Tri) (e w : V3) (hnx : Q.n.x ≠ 0) (ha : Q.eval u.a = 0)
    (hb : Q.eval u.b = 0) (hc : Q.eval u.c = 0) (hD : u.D ≠ 0)
    (hle : o2 (yz e) (yz u.b) (yz u.c) ≠ 0) :
    isEv 0 1 (ptFns Q u e w) (ptFns Q u e w)[1]! = decide (shadow e w (u.b, u.c)) := by
  obtain ⟨g, hg⟩ : ∃ g, g = aff01 (u.D * o2 (yz e) (yz u.b) (yz u.c)) (u.D * o2 (yz w) (yz u.b) (yz u.c)) :=
    ⟨_, rfl⟩
  have hfns : (ptFns Q u e w)[1]! = g := by rw [hg]; rfl
  rw [hfns]
  have hg0 : g.ev 0 ≠ 0 := by rw [hg, aff01_ev]; have := mul_ne_zero' hD hle; grind
  have hgs : ∀ s, g.ev s = u.D * o2 (yz (seg e w s)) (yz u.b) (yz u.c) := fun s => by
    rw [hg, aff01_ev, o2_seg1]; grind
  apply Bool.eq_iff_iff.mpr
  simp only [decide_eq_true_eq]
  constructor
  · intro hev
    unfold isEv at hev
    simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hev
    obtain ⟨⟨⟨hm, hr0⟩, hr1⟩, hn⟩ := hev
    have hz := Aff.ev_root g hm
    rw [hgs] at hz
    have hq0 : o2 (yz (seg e w g.root)) (yz u.b) (yz u.c) = 0 := by
      rcases Rat.mul_eq_zero.mp hz with h | h
      · exact absurd h hD
      · exact h
    -- the others positive at the root
    have hev := ptFns_ev Q u e w g.root
    have hl : ptFns Q u e w = [(ptFns Q u e w)[0]!, g, (ptFns Q u e w)[2]!, (ptFns Q u e w)[3]!] := by
      rw [← hfns]; simp [ptFns]
    rw [hl] at hn hev
    simp only [List.map_cons, List.map_nil, List.cons.injEq] at hev
    obtain ⟨e0, -, e2, e3, -⟩ := hev
    obtain ⟨p0, p2, p3⟩ := (nonpos4_second _ _ _ _ _ (Aff.ev_root g hm)).mp hn
    rw [e0] at p0; rw [e2] at p2; rw [e3] at p3
    -- the crossing point on the edge
    obtain ⟨hu, hv⟩ := on_bc (yz u.a) (yz u.b) (yz u.c) _ hD hq0
    obtain ⟨μ, hμ⟩ : ∃ μ, μ = o2 (yz u.a) (yz u.b) (yz (seg e w g.root)) / u.D := ⟨_, rfl⟩
    have hμ0 : 0 < μ := by rw [hμ]; exact div_pos_of_mul hD p3
    have hμ1 : μ < 1 := by
      have hs := bary_sum (yz u.a) (yz u.b) (yz u.c) (yz (seg e w g.root))
      rw [hq0] at hs
      have : 0 < o2 (yz u.a) (yz (seg e w g.root)) (yz u.c) / u.D := div_pos_of_mul hD p2
      have k1 := Rat.div_mul_cancel (a := o2 (yz u.a) (yz u.b) (yz (seg e w g.root))) hD
      have k2 := Rat.div_mul_cancel (a := o2 (yz u.a) (yz (seg e w g.root)) (yz u.c)) hD
      unfold Tri.D at *
      have : (μ + o2 (yz u.a) (yz (seg e w g.root)) (yz u.c) / o2 (yz u.a) (yz u.b) (yz u.c)) *
          o2 (yz u.a) (yz u.b) (yz u.c) = 1 * o2 (yz u.a) (yz u.b) (yz u.c) := by
        rw [hμ, Rat.add_mul, k1, k2]; grind
      have : μ + o2 (yz u.a) (yz (seg e w g.root)) (yz u.c) / o2 (yz u.a) (yz u.b) (yz u.c) = 1 := by
        grind
      grind
    have hμ' : o2 (yz u.a) (yz u.b) (yz (seg e w g.root)) / o2 (yz u.a) (yz u.b) (yz u.c) = μ := by
      rw [hμ]; rfl
    rw [hμ'] at hu hv
    -- r on the edge, on Q, straight ahead of the segment's point
    obtain ⟨r, hr⟩ : ∃ r, r = u.b + μ * (u.c - u.b) := ⟨_, rfl⟩
    have hrQ : Q.eval r = 0 := by
      rw [hr]; simp only [Plane.eval, V3.dot] at hb hc ⊢; simp; grind
    refine ⟨g.root, μ, r.x - (seg e w g.root).x, hr0, hr1, hμ0, hμ1, ?_, ?_⟩
    · -- ahead: the plane's value along x
      have : Q.eval r = Q.eval (seg e w g.root) + (r.x - (seg e w g.root).x) * Q.n.x := by
        simp only [Plane.eval, V3.dot]
        have ey : r.y = (seg e w g.root).y := by rw [hr]; simp [yz] at hu ⊢; grind
        have ez : r.z = (seg e w g.root).z := by rw [hr]; simp [yz] at hv ⊢; grind
        rw [ey, ez]; grind
      rw [hrQ] at this
      have hx := sq_pos' hnx
      apply Classical.byContradiction; intro hn
      have : -Q.eval (seg e w g.root) * Q.n.x = (r.x - (seg e w g.root).x) * (Q.n.x * Q.n.x) := by grind
      have := Rat.mul_le_mul_of_nonneg_left (a := r.x - (seg e w g.root).x) (b := 0) (by grind)
        (c := Q.n.x * Q.n.x) (Rat.le_of_lt hx)
      grind
    · rw [← hr]
      apply v3_ext
      · simp [ex]; grind
      · simp [ex, hr]; simp [yz] at hu; grind
      · simp [ex, hr]; simp [yz] at hv; grind
  · rintro ⟨s, μ, l, hs0, hs1, hμ0, hμ1, hl0, heq⟩
    have hyz : yz (seg e w s) = ⟨u.b.y + μ * (u.c.y - u.b.y), u.b.z + μ * (u.c.z - u.b.z)⟩ := by
      have ey := congrArg V3.y heq; have ez := congrArg V3.z heq
      simp [ex] at ey ez; simp [yz]; constructor <;> grind
    have hz : o2 (yz (seg e w s)) (yz u.b) (yz u.c) = 0 := by rw [hyz]; simp [yz]; unfold o2; grind
    have hm : g.m ≠ 0 := by
      intro hm
      have : g.ev s = g.ev 0 := by unfold Aff.ev; rw [hm]; grind
      rw [hgs, hz, Rat.mul_zero] at this
      exact hg0 this.symm
    have hroot : g.root = s := by
      have := hgs s; rw [hz, Rat.mul_zero, Aff.ev_eq g hm] at this
      rcases Rat.mul_eq_zero.mp this with h | h
      · exact absurd h hm
      · grind
    unfold isEv
    rw [hroot]
    simp only [ne_eq, hm, not_false_eq_true, decide_true, Bool.true_and, Bool.and_eq_true,
      decide_eq_true_eq, beq_iff_eq]
    refine ⟨⟨hs0, hs1⟩, ?_⟩
    have hev := ptFns_ev Q u e w s
    have hl : ptFns Q u e w = [(ptFns Q u e w)[0]!, g, (ptFns Q u e w)[2]!, (ptFns Q u e w)[3]!] := by
      rw [← hfns]; simp [ptFns]
    rw [hl] at hev ⊢
    simp only [List.map_cons, List.map_nil, List.cons.injEq] at hev
    obtain ⟨e0, -, e2, e3, -⟩ := hev
    rw [nonpos4_second _ _ _ _ _ (by rw [hgs, hz, Rat.mul_zero]), e0, e2, e3]
    -- the three others positive
    have p2 : 0 < u.D * o2 (yz u.a) (yz (seg e w s)) (yz u.c) := by
      rw [hyz]
      have : o2 (yz u.a) ⟨u.b.y + μ * (u.c.y - u.b.y), u.b.z + μ * (u.c.z - u.b.z)⟩ (yz u.c) =
          (1 - μ) * u.D := by unfold Tri.D o2; simp [yz]; grind
      rw [this]; have := Rat.mul_pos (sq_pos' hD) (show 0 < 1 - μ by grind); grind
    have p3 : 0 < u.D * o2 (yz u.a) (yz u.b) (yz (seg e w s)) := by
      rw [hyz]
      have : o2 (yz u.a) (yz u.b) ⟨u.b.y + μ * (u.c.y - u.b.y), u.b.z + μ * (u.c.z - u.b.z)⟩ =
          μ * u.D := by unfold Tri.D o2; simp [yz]; grind
      rw [this]; have := Rat.mul_pos (sq_pos' hD) hμ0; grind
    have p0 : 0 < -Q.eval (seg e w s) * Q.n.x := by
      have hrQ : Q.eval (u.b + μ * (u.c - u.b)) = 0 := by
        simp only [Plane.eval, V3.dot] at hb hc ⊢; simp; grind
      rw [← heq] at hrQ
      have : Q.eval (seg e w s + l * ex) = Q.eval (seg e w s) + l * Q.n.x := by
        simp only [Plane.eval, V3.dot, ex]; simp; grind
      rw [this] at hrQ
      have : -Q.eval (seg e w s) * Q.n.x = l * (Q.n.x * Q.n.x) := by grind
      rw [this]; exact Rat.mul_pos hl0 (sq_pos' hnx)
    exact ⟨p0, p2, p3⟩

theorem Tri.D_cyc (u : Tri) : (Tri.mk u.b u.c u.a).D = u.D := by
  unfold Tri.D; exact (o2_cyc _ _ _).symm

/-- The segment meets no edge line of `u` inside. -/
def LinesMissed (u : Tri) (e w : V3) : Prop :=
  ∀ s, 0 < s → s < 1 → ∀ μ : Rat, seg e w s ≠ u.b + μ * (u.c - u.b) ∧ seg e w s ≠ u.c + μ * (u.a - u.c) ∧
    seg e w s ≠ u.a + μ * (u.b - u.a)

/-- `H` separates the segment from the triangle. -/
structure Sep (H : Plane) (u : Tri) (e w : V3) : Prop where
  he : 0 < H.eval e
  hw : 0 < H.eval w
  ha : H.eval u.a ≤ 0
  hb : H.eval u.b ≤ 0
  hc : H.eval u.c ≤ 0

theorem PTGen.cyc {Q : Plane} {u : Tri} {e w : V3} (G : PTGen Q u e w) : PTGen Q ⟨u.b, u.c, u.a⟩ e w := by
  obtain ⟨qe, qw, ⟨l1, l2, l3⟩, ⟨m1, m2, m3⟩, hv⟩ := G
  refine ⟨qe, qw, ⟨?_, ?_, ?_⟩, ⟨?_, ?_, ?_⟩, fun s h0 h1 => ?_⟩ <;> simp only
  · intro h; apply l2; unfold o2 at *; grind
  · intro h; apply l3; unfold o2 at *; grind
  · intro h; apply l1; unfold o2 at *; grind
  · intro h; apply m2; unfold o2 at *; grind
  · intro h; apply m3; unfold o2 at *; grind
  · intro h; apply m1; unfold o2 at *; grind
  · obtain ⟨a, b, c⟩ := hv s h0 h1; exact ⟨b, c, a⟩

theorem LinesMissed.cyc {u : Tri} {e w : V3} (h : LinesMissed u e w) : LinesMissed ⟨u.b, u.c, u.a⟩ e w :=
  fun s h0 h1 μ => let ⟨a, b, c⟩ := h s h0 h1 μ; ⟨b, c, a⟩

/-- **Constancy, the general case**: a triangle of a plane `Q` not along `x`, the segment off
its edge lines and missing it: its hit changes from `e` to `w` by the parity of its edges'
shadows. -/
theorem pt_general (Q H : Plane) (u : Tri) (e w : V3) (hnx : Q.n.x ≠ 0) (ha : Q.eval u.a = 0)
    (hb : Q.eval u.b = 0) (hc : Q.eval u.c = 0) (hD : u.D ≠ 0) (G : PTGen Q u e w)
    (G4 : LinesMissed u e w) (S : Sep H u e w) :
    (hit Q u e ^^ hit Q u w) = par (u.segs.map fun E => decide (shadow e w E)) := by
  obtain ⟨g0, g1, g2, g3, hl⟩ : ∃ g0 g1 g2 g3, ptFns Q u e w = [g0, g1, g2, g3] := ⟨_, _, _, _, rfl⟩
  have hends : ∀ g ∈ ptFns Q u e w, g.ev 0 ≠ 0 ∧ g.ev 1 ≠ 0 := by
    have h0 := ptFns_ev Q u e w 0
    have h1 := ptFns_ev Q u e w 1
    rw [seg_zero] at h0; rw [seg_one] at h1
    obtain ⟨qe, qw, ⟨l1, l2, l3⟩, ⟨m1, m2, m3⟩, -⟩ := G
    rw [hl] at h0 h1 ⊢
    simp only [List.map_cons, List.map_nil, List.cons.injEq] at h0 h1
    intro g hg
    simp only [List.mem_cons, List.mem_nil_iff, or_false] at hg
    rcases hg with rfl | rfl | rfl | rfl
    · rw [h0.1, h1.1]; exact ⟨mul_ne_zero' (by grind) hnx, mul_ne_zero' (by grind) hnx⟩
    · rw [h0.2.1, h1.2.1]; exact ⟨mul_ne_zero' hD l1, mul_ne_zero' hD m1⟩
    · rw [h0.2.2.1, h1.2.2.1]; exact ⟨mul_ne_zero' hD l2, mul_ne_zero' hD m2⟩
    · rw [h0.2.2.2.1, h1.2.2.2.1]; exact ⟨mul_ne_zero' hD l3, mul_ne_zero' hD m3⟩
  have zero_iff : ∀ a b : Rat, a ≠ 0 → (a * b = 0 ↔ b = 0) := fun a b ha => by
    constructor
    · intro h; rcases Rat.mul_eq_zero.mp h with h | h; exact absurd h ha; exact h
    · intro h; rw [h]; grind
  have hdist : Distinct (ptFns Q u e w) 0 1 := by
    intro s h0 h1
    have hev := ptFns_ev Q u e w s
    rw [hl] at hev ⊢
    simp only [List.map_cons, List.map_nil, List.cons.injEq] at hev
    obtain ⟨e0, e1, e2, e3, -⟩ := hev
    simp only [List.countP_cons, List.countP_nil, e0, e1, e2, e3]
    have k0 : -Q.eval (seg e w s) * Q.n.x = 0 ↔ Q.eval (seg e w s) = 0 := by
      rw [Rat.mul_comm, zero_iff _ _ hnx]; grind
    simp only [k0, zero_iff _ _ hD]
    obtain ⟨t1, t2, t3⟩ := two_sides_vertex' (yz u.a) (yz u.b) (yz u.c) (yz (seg e w s)) hD
    obtain ⟨v1, v2, v3⟩ := G.verts s h0 h1
    have n1 := G4 s h0 h1
    by_cases kq : Q.eval (seg e w s) = 0
    · obtain ⟨o1, o2', o3⟩ := on_edge_line Q u (seg e w s) hnx ha hb hc kq hD
      by_cases k1 : o2 (yz (seg e w s)) (yz u.b) (yz u.c) = 0
      · obtain ⟨μ, hμ⟩ := o1 k1; exact absurd hμ (n1 μ).1
      by_cases k2 : o2 (yz u.a) (yz (seg e w s)) (yz u.c) = 0
      · obtain ⟨μ, hμ⟩ := o2' k2; exact absurd hμ (n1 μ).2.1
      by_cases k3 : o2 (yz u.a) (yz u.b) (yz (seg e w s)) = 0
      · obtain ⟨μ, hμ⟩ := o3 k3; exact absurd hμ (n1 μ).2.2
      simp [kq, k1, k2, k3]
    · by_cases k1 : o2 (yz (seg e w s)) (yz u.b) (yz u.c) = 0 <;>
        by_cases k2 : o2 (yz u.a) (yz (seg e w s)) (yz u.c) = 0 <;>
        by_cases k3 : o2 (yz u.a) (yz u.b) (yz (seg e w s)) = 0 <;> simp [kq, k1, k2, k3]
      · exact v3 (t1 k1 k2)
      · exact v3 (t1 k1 k2)
      · exact v2 (t3 k1 k3)
      · exact v1 (t2 k2 k3)
  have hip := ip (ptFns Q u e w) 0 1 (by decide) hends hdist
  rw [pos_ptFns, pos_ptFns, seg_zero, seg_one] at hip
  rw [hip]
  -- the plane's root is no event: there the point would be inside the triangle
  have evL : isEv 0 1 (ptFns Q u e w) g0 = false := by
    apply Classical.byContradiction; intro hn
    have hev : isEv 0 1 (ptFns Q u e w) g0 = true := by simpa using hn
    unfold isEv at hev
    simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hev
    obtain ⟨⟨⟨hm, hr0⟩, hr1⟩, hn⟩ := hev
    have hevs := ptFns_ev Q u e w g0.root
    rw [hl] at hn hevs
    simp only [List.map_cons, List.map_nil, List.cons.injEq] at hevs
    obtain ⟨e0, e1, e2, e3, -⟩ := hevs
    have hz0 : g0.ev g0.root = 0 := Aff.ev_root g0 hm
    have hn' : nonpos [g0, g1, g2, g3] g0.root = 1 ↔ (0 < g1.ev g0.root ∧ 0 < g2.ev g0.root ∧ 0 < g3.ev g0.root) := by
      simp only [nonpos, List.countP_cons, List.countP_nil, hz0, Rat.le_refl, decide_true, ite_true]
      by_cases a : 0 < g1.ev g0.root <;> by_cases b : 0 < g2.ev g0.root <;> by_cases c : 0 < g3.ev g0.root <;>
        simp [a, b, c, show (g1.ev g0.root ≤ 0 ↔ ¬ 0 < g1.ev g0.root) from by grind,
          show (g2.ev g0.root ≤ 0 ↔ ¬ 0 < g2.ev g0.root) from by grind,
          show (g3.ev g0.root ≤ 0 ↔ ¬ 0 < g3.ev g0.root) from by grind]
    obtain ⟨p1, p2, p3⟩ := hn'.mp hn
    rw [e1] at p1; rw [e2] at p2; rw [e3] at p3
    rw [e0] at hz0
    have hq : Q.eval (seg e w g0.root) = 0 := by
      have : Q.eval (seg e w g0.root) * Q.n.x = 0 := by grind
      rcases Rat.mul_eq_zero.mp this with h | h; exact h; exact absurd h hnx
    have hin : u.ins (seg e w g0.root) = true := by
      simp only [Tri.ins, inside2, Bool.and_eq_true, decide_eq_true_eq]; unfold Tri.D at p1 p2 p3
      exact ⟨⟨p1, p2⟩, p3⟩
    have := lift_H Q H u _ hnx ha hb hc hq hin S.ha S.hb S.hc
    have := H_seg_pos H e w S.he S.hw g0.root (by grind) (by grind)
    grind
  have evA := pt_ev_A Q u e w hnx ha hb hc hD G.le.1
  have evB := pt_ev_A Q ⟨u.b, u.c, u.a⟩ e w hnx hb hc ha (by rw [Tri.D_cyc]; exact hD) G.cyc.le.1
  have evC := pt_ev_A Q ⟨u.c, u.a, u.b⟩ e w hnx hc ha hb
    (by have := Tri.D_cyc ⟨u.b, u.c, u.a⟩; rw [this, Tri.D_cyc]; exact hD) G.cyc.cyc.le.1
  rw [ptFns_cyc] at evB
  rw [show (Tri.mk u.c u.a u.b) = Tri.mk (Tri.mk u.b u.c u.a).b (Tri.mk u.b u.c u.a).c (Tri.mk u.b u.c u.a).a
    from rfl, ptFns_cyc, ptFns_cyc] at evC
  rw [hl] at evA evB evC
  simp only [List.getElem!_cons_zero, List.getElem!_cons_succ] at evA evB evC
  have p1 : [g0, g2, g3, g1].Perm [g0, g1, g2, g3] :=
    List.Perm.cons g0 ((List.Perm.swap g2 g1 [g3]).trans (List.Perm.cons g2 (List.Perm.swap g3 g1 []))).symm
  have p2 : [g0, g3, g1, g2].Perm [g0, g1, g2, g3] :=
    List.Perm.cons g0 ((List.Perm.cons g1 (List.Perm.swap g3 g2 [])).trans (List.Perm.swap g3 g1 [g2])).symm
  rw [isEv_perm p1] at evB
  rw [isEv_perm p2] at evC
  rw [hl]
  simp only [List.map_cons, List.map_nil, Tri.segs]
  rw [hl] at evL
  rw [evL, evA, evB, evC]
  simp only [par_cons, par_nil, Bool.false_xor, Bool.xor_false]
  cases decide (shadow e w (u.a, u.b)) <;> cases decide (shadow e w (u.b, u.c)) <;>
    cases decide (shadow e w (u.c, u.a)) <;> rfl

end ForgeCertify
