import ForgeExact.DecideOnce.CorrectK
import ForgeExact.DecideOnce.Nef

/-!
# The flat exact ledger is an exact curved ledger (design Q0)

`exactK_of_exact`: every `Exact` ledger over planes is an `ExactK` ledger over the planes' values
(`ps.map Plane.eval`), given that no two carriers are one plane up to scale (`NoScaled`; the snap's
classes are distinct planes). So `correct_k` and `zeroThickness_iff_k` hold of every flat input the
sampler describes: the flat theorem (c) is an instance of the curved one.

The one real step is that the flat dimension tests (the rank of the zero carriers' normals,
`IsVert`, `IsEdge`) are the curved ones (`IsVertK`: one point; `IsEdgeK`: two zeros and more than
one point) on realised cells:
* `vert_unique`: three independent planes through a cell pin it to a point (Cramer's identity,
  `cramer_x`, …);
* `line_in_cell`: a direction orthogonal to every zero carrier's normal moves a point within its
  cell (`Flat.stepFor` keeps the other carriers' signs);
* `perp_exists`: with no three independent zero normals there is such a direction;
* `isVertK_iff`, `isEdgeK_iff`.
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.Ref ForgeExact.DecideOnce Topology Ledger

/-! ## Vector identities -/

theorem cramer_x (a b c v : V3) :
    det3 a b c * v.x = a.dot v * (b.cross c).x + b.dot v * (c.cross a).x + c.dot v * (a.cross b).x := by
  simp only [det3, V3.dot, V3.cross]; grind

theorem cramer_y (a b c v : V3) :
    det3 a b c * v.y = a.dot v * (b.cross c).y + b.dot v * (c.cross a).y + c.dot v * (a.cross b).y := by
  simp only [det3, V3.dot, V3.cross]; grind

theorem cramer_z (a b c v : V3) :
    det3 a b c * v.z = a.dot v * (b.cross c).z + b.dot v * (c.cross a).z + c.dot v * (a.cross b).z := by
  simp only [det3, V3.dot, V3.cross]; grind

theorem dot_sub (n x y : V3) : n.dot (x - y) = n.dot x - n.dot y := by
  show n.dot (V3.sub x y) = _
  simp only [V3.dot, V3.sub]; grind

theorem isZero_iff (a : V3) : a.isZero = true ↔ a.x = 0 ∧ a.y = 0 ∧ a.z = 0 := by
  simp [V3.isZero, and_assoc]

theorem sgnQ_zero_iff (q : Rat) : sgnQ q = .zero ↔ q = 0 := by
  unfold sgnQ
  constructor
  · intro h; split at h
    · cases h
    · split at h
      · cases h
      · grind
  · intro h; subst h; simp

/-! ## Cells of planes -/

theorem cov_getD {ps : List Plane} {X : Cov} {x : V3} (hx : covCell ps X x) {j : Nat}
    (hj : j < ps.length) : X.getD j .pos = sgnQ ((ps.getD j default).eval x) := by
  rw [Curved.covCell_iff_covOf] at hx
  subst hx
  simp [covOf, List.getD_eq_getElem?_getD, hj]

theorem mem_zerosOf {X : Cov} {j : Nat} : j ∈ zerosOf X ↔ j < X.length ∧ X.getD j .pos = .zero := by
  simp [zerosOf]

theorem eval_zero_of {ps : List Plane} {X : Cov} {x : V3} (hx : covCell ps X x) {j : Nat}
    (hj : j ∈ zerosOf X) : (ps.getD j default).eval x = 0 := by
  obtain ⟨hl, hz⟩ := mem_zerosOf.mp hj
  rw [covCell_length hx] at hl
  rw [cov_getD hx hl] at hz
  exact (sgnQ_zero_iff _).mp hz

theorem getD_mem {ps : List Plane} {j : Nat} (hj : j < ps.length) : ps.getD j default ∈ ps := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]

/-- **Three independent planes pin a cell to a point.** -/
theorem vert_unique {ps : List Plane} {X : Cov} {x y : V3} (hv : IsVert ps X) (hx : covCell ps X x)
    (hy : covCell ps X y) : x = y := by
  obtain ⟨a, ha, b, hb, c, hc, hdet⟩ := hv
  have hz : ∀ j ∈ zerosOf X, (ps.getD j default).n.dot (x - y) = 0 := by
    intro j hj
    have h1 := eval_zero_of hx hj
    have h2 := eval_zero_of hy hj
    rw [dot_sub]; simp only [Plane.eval] at h1 h2; grind
  have e1 := cramer_x (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n (x - y)
  have e2 := cramer_y (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n (x - y)
  have e3 := cramer_z (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n (x - y)
  rw [hz a ha, hz b hb, hz c hc] at e1 e2 e3
  have hx0 : (x - y).x = 0 := by
    have : det3 (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n * (x - y).x = 0 := by
      rw [e1]; grind
    exact (Rat.mul_eq_zero.mp this).resolve_left hdet
  have hy0 : (x - y).y = 0 := by
    have : det3 (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n * (x - y).y = 0 := by
      rw [e2]; grind
    exact (Rat.mul_eq_zero.mp this).resolve_left hdet
  have hz0 : (x - y).z = 0 := by
    have : det3 (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n * (x - y).z = 0 := by
      rw [e3]; grind
    exact (Rat.mul_eq_zero.mp this).resolve_left hdet
  change (V3.sub x y).x = 0 at hx0
  change (V3.sub x y).y = 0 at hy0
  change (V3.sub x y).z = 0 at hz0
  simp only [V3.sub] at hx0 hy0 hz0
  cases x; cases y; simp only [V3.mk.injEq]; grind

theorem toward_mem_sgn (p : Plane) {x y : V3} (hx : p.eval x ≠ 0) (hy : (p.toward x).eval y > 0) :
    sgnQ (p.eval y) = sgnQ (p.eval x) := by
  unfold Plane.toward at hy
  split at hy
  · rename_i h; rw [(sgnQ_pos_iff _).mpr hy, (sgnQ_pos_iff _).mpr h]
  · rename_i h
    rw [Flat.eval_neg] at hy
    have h' : p.eval x < 0 := by grind
    rw [(sgnQ_neg_iff _).mpr (by grind), (sgnQ_neg_iff _).mpr h']

/-- **A direction orthogonal to every zero carrier moves a point within its cell.** -/
theorem line_in_cell {ps : List Plane} {X : Cov} {x d : V3} (hx : covCell ps X x)
    (hd : d ≠ ⟨0, 0, 0⟩) (hperp : ∀ j ∈ zerosOf X, (ps.getD j default).n.dot d = 0) :
    ∃ y, y ≠ x ∧ covCell ps X y := by
  let hs := (ps.filter fun p => p.eval x ≠ 0).map fun p => p.toward x
  have hpos : Flat.allPos hs x := by
    intro q hq
    simp only [hs, List.mem_map, List.mem_filter] at hq
    obtain ⟨p, ⟨_, hp⟩, rfl⟩ := hq
    exact Flat.toward_pos p (by simpa using hp)
  let s := Flat.stepFor x d hs
  have hs0 : 0 < s := Flat.stepFor_pos x d hs hpos
  refine ⟨x + s * d, ?_, ?_⟩
  · intro he
    apply hd
    have hx' : (x + s * d).x = x.x := by rw [he]
    have hy' : (x + s * d).y = x.y := by rw [he]
    have hz' : (x + s * d).z = x.z := by rw [he]
    simp only [Flat.add_x, Flat.smul_x, Flat.add_y, Flat.smul_y, Flat.add_z, Flat.smul_z] at hx' hy' hz'
    have h1 : s * d.x = 0 := by grind
    have h2 : s * d.y = 0 := by grind
    have h3 : s * d.z = 0 := by grind
    have hs' : s ≠ 0 := by grind
    obtain ⟨a, b, c⟩ := d
    simp only [V3.mk.injEq]
    exact ⟨(Rat.mul_eq_zero.mp h1).resolve_left hs', (Rat.mul_eq_zero.mp h2).resolve_left hs',
      (Rat.mul_eq_zero.mp h3).resolve_left hs'⟩
  · rw [Curved.covCell_iff_covOf] at hx ⊢
    rw [← hx]
    unfold covOf
    apply List.map_congr_left
    intro p hp
    by_cases h0 : p.eval x = 0
    · obtain ⟨j, hj, rfl⟩ := List.mem_iff_getElem.mp hp
      have hjz : j ∈ zerosOf X := by
        rw [mem_zerosOf]
        have hcl : X.length = ps.length := by rw [← hx]; simp [covOf]
        refine ⟨by omega, ?_⟩
        rw [← hx]
        simp [covOf, List.getD_eq_getElem?_getD, hj, h0, sgnQ]
      have hg : ps.getD j default = ps[j] := by
        simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]
      have := hperp j hjz
      rw [hg] at this
      rw [Flat.eval_step, h0, this, Rat.mul_zero, Rat.add_zero]
    · have hq : p.toward x ∈ hs := by
        simp only [hs, List.mem_map, List.mem_filter]
        exact ⟨p, ⟨hp, by simpa using h0⟩, rfl⟩
      apply toward_mem_sgn p h0
      rw [Flat.eval_step]
      exact Flat.pos_of_small (hpos _ hq) hs0 (Flat.stepFor_le x d hs _ hq)

/-- With no three independent zero normals, some nonzero direction is orthogonal to them all. -/
theorem perp_exists {ps : List Plane} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) {X : Cov}
    (hX : X.length = ps.length) (hnv : ¬IsVert ps X) :
    ∃ d : V3, d ≠ ⟨0, 0, 0⟩ ∧ ∀ j ∈ zerosOf X, (ps.getD j default).n.dot d = 0 := by
  by_cases hA : ∃ a ∈ zerosOf X, ∃ b ∈ zerosOf X,
      ((ps.getD a default).n.cross (ps.getD b default).n).isZero = false
  · obtain ⟨a, ha, b, hb, hab⟩ := hA
    refine ⟨(ps.getD a default).n.cross (ps.getD b default).n, ?_, ?_⟩
    · intro h; rw [h] at hab; simp [V3.isZero] at hab
    · intro c hc
      have h0 : det3 (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n = 0 :=
        Classical.byContradiction fun hne => hnv ⟨a, ha, b, hb, c, hc, hne⟩
      simp only [det3, V3.dot, V3.cross] at h0 ⊢; grind
  · have hall : ∀ a ∈ zerosOf X, ∀ b ∈ zerosOf X,
        ((ps.getD a default).n.cross (ps.getD b default).n).isZero = true := by
      intro a ha b hb
      apply Classical.byContradiction; intro hn
      exact hA ⟨a, ha, b, hb, by simpa using hn⟩
    by_cases hz : ∃ a, a ∈ zerosOf X
    · obtain ⟨a, ha⟩ := hz
      have hal : a < ps.length := by rw [← hX]; exact (mem_zerosOf.mp ha).1
      have hn := hnz _ (getD_mem hal)
      generalize hna : (ps.getD a default).n = n at hn
      have perp : ∀ e : V3, ∀ c ∈ zerosOf X, (ps.getD c default).n.dot (n.cross e) = 0 := by
        intro e c hc
        have := (isZero_iff _).mp (hall a ha c hc)
        rw [hna] at this
        simp only [V3.cross, V3.dot] at this ⊢; grind
      by_cases h1 : n.cross ⟨1, 0, 0⟩ = ⟨0, 0, 0⟩
      · by_cases h2 : n.cross ⟨0, 1, 0⟩ = ⟨0, 0, 0⟩
        · refine ⟨n.cross ⟨0, 0, 1⟩, ?_, perp _⟩
          intro h3
          apply hn
          simp only [V3.cross, V3.mk.injEq] at h1 h2 h3
          obtain ⟨x, y, z⟩ := n
          simp only [V3.mk.injEq]; grind
        · exact ⟨_, h2, perp _⟩
      · exact ⟨_, h1, perp _⟩
    · exact ⟨⟨1, 0, 0⟩, by simp, fun j hj => absurd ⟨j, hj⟩ hz⟩

/-! ## The dimension tests agree -/

theorem realisedK_iff (ps : List Plane) (X : Cov) :
    RealisedK (ps.map fun p => p.eval) X ↔ Realised ps X := by
  simp only [RealisedK, Realised, covCellK_flat]

theorem memTK_iff (ps : List Plane) (A : Nat → Pred V3) (T : Cov) (i : Nat) :
    memTK (ps.map fun p => p.eval) A T i ↔ memT ps A T i := by
  simp only [memTK, memT, covCellK_flat]

theorem flipK_iff (ps : List Plane) (A : Nat → Pred V3) (φ : Cov) (i : Nat) :
    FlipK (ps.map fun p => p.eval) A φ i ↔ Flip ps A φ i := by
  simp only [FlipK, Flip, memTK_iff]

theorem isVertK_iff {ps : List Plane} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) {X : Cov} (hr : Realised ps X) :
    IsVertK (ps.map fun p => p.eval) X ↔ IsVert ps X := by
  constructor
  · rintro ⟨_, hone⟩
    apply Classical.byContradiction
    intro hnv
    obtain ⟨x, hx⟩ := hr
    obtain ⟨d, hd, hperp⟩ := perp_exists hnz (covCell_length hx) hnv
    obtain ⟨y, hyx, hy⟩ := line_in_cell hx hd hperp
    exact hyx (hone y x ((covCellK_flat ps X y).mpr hy) ((covCellK_flat ps X x).mpr hx))
  · intro hv
    exact ⟨(realisedK_iff ps X).mpr hr, fun x y hx hy =>
      vert_unique hv ((covCellK_flat ps X x).mp hx) ((covCellK_flat ps X y).mp hy)⟩

/-- No two carriers are one plane up to scale (the snap's classes are distinct planes). -/
def NoScaled (ps : List Plane) : Prop :=
  ∀ i j, i < ps.length → j < ps.length → i ≠ j → ∀ k : Rat,
    (ps.getD j default).n ≠ k * (ps.getD i default).n ∨ (ps.getD j default).c ≠ k * (ps.getD i default).c

theorem zero_of_getElem? {X : Cov} {a : Nat} (h : X[a]? = some Sgn.zero) : a ∈ zerosOf X := by
  rw [mem_zerosOf]
  have ha : a < X.length := by
    apply Classical.byContradiction; intro hn
    rw [List.getElem?_eq_none (by omega)] at h; cases h
  exact ⟨ha, by simp [List.getD_eq_getElem?_getD, h]⟩

theorem getElem?_of_zero {X : Cov} {a : Nat} (h : a ∈ zerosOf X) : X[a]? = some Sgn.zero := by
  obtain ⟨ha, hz⟩ := mem_zerosOf.mp h
  rw [List.getElem?_eq_getElem ha]
  simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem ha, Option.getD_some] at hz
  rw [hz]

theorem isEdgeK_iff {ps : List Plane} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hns : NoScaled ps) {X : Cov}
    (hr : Realised ps X) :
    IsEdgeK (ps.map fun p => p.eval) X ↔ IsEdge ps X := by
  constructor
  · rintro ⟨_, ⟨a, b, hab, ha, hb⟩, hnv⟩
    refine ⟨fun hv => hnv ((isVertK_iff hnz hr).mpr hv), a, zero_of_getElem? ha, b, zero_of_getElem? hb, ?_⟩
    apply Classical.byContradiction
    intro hc
    have hc0 : ((ps.getD a default).n.cross (ps.getD b default).n).isZero = true := by
      revert hc; cases ((ps.getD a default).n.cross (ps.getD b default).n).isZero <;> simp
    have hc' := (isZero_iff _).mp hc0
    obtain ⟨x, hx⟩ := hr
    have hal : a < ps.length := by rw [← covCell_length hx]; exact (mem_zerosOf.mp (zero_of_getElem? ha)).1
    have hbl : b < ps.length := by rw [← covCell_length hx]; exact (mem_zerosOf.mp (zero_of_getElem? hb)).1
    have hea := eval_zero_of hx (zero_of_getElem? ha)
    have heb := eval_zero_of hx (zero_of_getElem? hb)
    have hn := hnz _ (getD_mem hal)
    generalize hPa : ps.getD a default = Pa at hea hn hc'
    generalize hPb : ps.getD b default = Pb at heb hc'
    have hN : 0 < Pa.n.dot Pa.n := Flat.norm2_pos hn
    have hne : Pa.n.dot Pa.n ≠ 0 := by grind
    rcases hns a b hal hbl hab ((Pa.n.dot Pb.n) / (Pa.n.dot Pa.n)) with h | h
    · rw [hPa, hPb] at h
      apply h
      obtain ⟨⟨u1, u2, u3⟩, ca⟩ := Pa
      obtain ⟨⟨w1, w2, w3⟩, cb⟩ := Pb
      simp only [V3.cross, V3.dot] at hc' hne ⊢
      obtain ⟨h1, h2, h3⟩ := hc'
      have e1 : (u1 * u1 + u2 * u2 + u3 * u3) * w1 = (u1 * w1 + u2 * w2 + u3 * w3) * u1 := by grind
      have e2 : (u1 * u1 + u2 * u2 + u3 * u3) * w2 = (u1 * w1 + u2 * w2 + u3 * w3) * u2 := by grind
      have e3 : (u1 * u1 + u2 * u2 + u3 * u3) * w3 = (u1 * w1 + u2 * w2 + u3 * w3) * u3 := by grind
      show _ = V3.smul _ _
      simp only [V3.smul, V3.mk.injEq]
      exact ⟨by grind, by grind, by grind⟩
    · rw [hPa, hPb] at h
      apply h
      obtain ⟨⟨u1, u2, u3⟩, ca⟩ := Pa
      obtain ⟨⟨w1, w2, w3⟩, cb⟩ := Pb
      simp only [V3.cross, V3.dot, Plane.eval] at hc' hne hea heb ⊢
      obtain ⟨h1, h2, h3⟩ := hc'
      have e1 : (u1 * u1 + u2 * u2 + u3 * u3) * w1 = (u1 * w1 + u2 * w2 + u3 * w3) * u1 := by grind
      have e2 : (u1 * u1 + u2 * u2 + u3 * u3) * w2 = (u1 * w1 + u2 * w2 + u3 * w3) * u2 := by grind
      have e3 : (u1 * u1 + u2 * u2 + u3 * u3) * w3 = (u1 * w1 + u2 * w2 + u3 * w3) * u3 := by grind
      have e : (u1 * u1 + u2 * u2 + u3 * u3) * cb = (u1 * w1 + u2 * w2 + u3 * w3) * ca := by grind
      grind
  · rintro ⟨hnv, a, ha, b, hb, hab⟩
    refine ⟨(realisedK_iff ps X).mpr hr, ⟨a, b, ?_, getElem?_of_zero ha, getElem?_of_zero hb⟩,
      fun hv => hnv ((isVertK_iff hnz hr).mp hv)⟩
    rintro rfl
    simp [V3.cross, V3.isZero] at hab
    grind

/-! ## The instance -/

theorem wallK_flat (ps : List Plane) : wallK (ps.map fun p => p.eval) = Flat.wall ps := by
  funext x
  apply propext
  simp only [wallK, Flat.wall, List.mem_map]
  constructor
  · rintro ⟨_, ⟨p, hp, rfl⟩, h⟩; exact ⟨p, hp, (sgnQ_zero_iff _).mp h⟩
  · rintro ⟨p, hp, h⟩; exact ⟨_, ⟨p, hp, rfl⟩, (sgnQ_zero_iff _).mpr h⟩

/-- **Every flat exact ledger is an exact curved ledger** over the planes' values. -/
theorem exactK_of_exact {L : Ledger} {ps : List Plane} {A : Nat → Pred V3} (h : Exact L ps A)
    (hns : NoScaled ps) : ExactK L (ps.map fun p => p.eval) A where
  wallND := by rw [wallK_flat]; exact Flat.wall_nowhereDense ps h.nonzero
  compat T hT i := by
    obtain ⟨⟨x, hx⟩, hz⟩ := hT
    have hl : T.length = ps.length := covCell_length ((covCellK_flat ps T x).mp hx)
    rcases h.compat T (mem_fullsK hl hz) i with hs | hd
    · exact Or.inl fun y hy => hs y ((covCellK_flat ps T y).mp hy)
    · exact Or.inr fun y hy => hd y ((covCellK_flat ps T y).mp hy)
  pieceFacet := h.pieceFacet
  oneFace := h.oneFace
  pieceReal p hp := (realisedK_iff ps _).mpr (h.pieceReal p hp)
  witness T hT i hi := by rw [memTK_iff]; exact h.witness T hT i hi
  boundary φ hφ i hi := by rw [flipK_iff]; exact h.boundary φ hφ i hi
  complete φ hr hf i hi hfl :=
    h.complete φ ((realisedK_iff ps φ).mp hr) hf i hi ((flipK_iff ps A φ i).mp hfl)
  edges e := by
    rw [h.edges e]
    constructor
    · rintro ⟨hr, he, hφ⟩
      exact ⟨(realisedK_iff ps e).mpr hr, (isEdgeK_iff h.nonzero hns hr).mpr he, hφ⟩
    · rintro ⟨hr, he, hφ⟩
      have hr' := (realisedK_iff ps e).mp hr
      exact ⟨hr', (isEdgeK_iff h.nonzero hns hr').mp he, hφ⟩
  verts v := by
    rw [h.verts v]
    constructor
    · rintro ⟨hr, hv, hφ⟩
      exact ⟨(realisedK_iff ps v).mpr hr, (isVertK_iff h.nonzero hr).mpr hv, hφ⟩
    · rintro ⟨hr, hv, hφ⟩
      have hr' := (realisedK_iff ps v).mp hr
      exact ⟨hr', (isVertK_iff h.nonzero hr').mp hv, hφ⟩

/-- **The flat (c) as an instance of the curved one.** -/
theorem flat_correct_k {L : Ledger} {ps : List Plane} {A : Nat → Pred V3} (h : Exact L ps A)
    (hns : NoScaled ps) (rule : Rule) :
    (∀ φ, φ ∈ L.result (L.keepTab rule) ↔ XBoundaryK L (ps.map fun p => p.eval) A rule φ) ∧
    ((∃ x, core L rule = .refused (.zeroThickness x)) ↔
      XNonManifoldK L (ps.map fun p => p.eval) A rule) :=
  ⟨(correct_k (exactK_of_exact h hns) rule).1, zeroThickness_iff_k (exactK_of_exact h hns) rule⟩

end ForgeExact.DecideOnce.Curved
