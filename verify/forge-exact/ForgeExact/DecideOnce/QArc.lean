import ForgeExact.DecideOnce.QSample

/-!
# Q1 L2: every arc at a vertex is found (the A8 obligation, plan §2)

The face builder's sectors at a vertex `v` are read on the link: take a certified isolating box
`Q` round `v`. Every arc of `q_j ∩ q_k` from `v` leaves `Q` through its boundary (the sweep's
geometry, from LemmaD and the box certificate). The order of its exit points along `q_j ∩ ∂Q` gives
the sectors. This file proves the exact half: **every point of `q_j ∩ q_k` on `∂Q` is found.**

* `Axis.perm`, `Axis.pt`: the coordinate swap that makes a face `y = c` or `z = c` a sweep
  plane `x = c` (`perm_ev`: the swapped quadric at the swapped point is the quadric's value;
  `pt_pt`: the swap is an involution);
* `faceCrossings a c qj qk`: the crossings on the face `a = c`, in the first sheared frame of
  `shears` where `proposeEdgeAny` succeeds;
* **`faceCrossings_complete`**: every common zero on the face is a sample. Its swapped, sheared
  coordinate is the sample's root, and the sample's point is the point;
* `boxCrossings`, **`boxCrossings_complete`**: the same on all six faces of a rational box
  (each face's whole plane: the rectangle is read later, by comparisons);
* `faceCov`, `faceCov_realised`: a crossing's covector is the covector of a real point (the
  sector's edge is a lookup).
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-- `q(y, x, z)`. -/
def _root_.ForgeExact.Quad.Quadric.swapXY (q : Quadric) : Quadric :=
  ⟨q.yy, q.xx, q.zz, q.xy, q.yz, q.xz, q.ly, q.lx, q.lz, q.c⟩

/-- `q(z, y, x)`. -/
def _root_.ForgeExact.Quad.Quadric.swapXZ (q : Quadric) : Quadric :=
  ⟨q.zz, q.yy, q.xx, q.yz, q.xz, q.xy, q.lz, q.ly, q.lx, q.c⟩

/-- A box face's normal axis. -/
inductive Axis | x | y | z
  deriving DecidableEq, Repr

/-- The swap that makes the face `a = c` the plane `x = c`. -/
def Axis.perm : Axis → Quadric → Quadric
  | .x, q => q
  | .y, q => q.swapXY
  | .z, q => q.swapXZ

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- The same swap on points. -/
def Axis.pt : Axis → PK K → PK K
  | .x, p => p
  | .y, (u, v, w) => (v, u, w)
  | .z, (u, v, w) => (w, v, u)

theorem pt_pt (a : Axis) (p : PK K) : a.pt (a.pt p) = p := by
  obtain ⟨u, v, w⟩ := p; cases a <;> rfl

theorem perm_ev (a : Axis) (q : Quadric) (p : PK K) :
    Quadric.evK M (a.perm q) p = Quadric.evK M q (a.pt p) := by
  obtain ⟨u, v, w⟩ := p
  cases a <;> simp only [Axis.perm, Axis.pt, Quadric.swapXY, Quadric.swapXZ, Quadric.evK] <;> grind

end Model

/-- The crossings of `q_j ∩ q_k` with the face `a = c`: the shear used and the samples. -/
def faceCrossings (a : Axis) (c : Rat) (qj qk : Quadric) (fuel : Nat) :
    Option (Rat × List ESample) :=
  shears.findSome? fun l =>
    (proposeEdgeAny ((a.perm qj).shear l) ((a.perm qk).shear l) c fuel).map fun es => (l, es)

/-- The six faces of the box `[lo, hi]`. -/
def boxFaces (lo hi : V3) : List (Axis × Rat) :=
  [(.x, lo.x), (.x, hi.x), (.y, lo.y), (.y, hi.y), (.z, lo.z), (.z, hi.z)]

/-- The crossings of `q_j ∩ q_k` with every face of the box. -/
def boxCrossings (qj qk : Quadric) (lo hi : V3) (fuel : Nat) :
    Option (List (Axis × Rat × Rat × List ESample)) :=
  (boxFaces lo hi).mapM fun (f : Axis × Rat) =>
    (faceCrossings f.1 f.2 qj qk fuel).map fun r => (f.1, f.2, r.1, r.2)

/-- A crossing's covector over the carriers, in the face's frame. -/
def faceCov (qs : List Quadric) (a : Axis) (l : Rat) (e : ESample) : Option Cov :=
  if isoOkN e.y && e.dOk signFuel then edgeCov (qs.map fun q => (a.perm q).shear l) e signFuel
  else none

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- **Every common zero on the face is a crossing sample.** -/
theorem faceCrossings_complete {a : Axis} {c : Rat} {qj qk : Quadric} {fuel : Nat} {l : Rat}
    {es : List ESample} (h : faceCrossings a c qj qk fuel = some (l, es)) {p : PK K}
    (hf : (a.pt p).1 = M.q c) (hj : Quadric.evK M qj p = 0) (hk : Quadric.evK M qk p = 0) :
    ∃ e ∈ es, Denotes M e.y ((a.pt p).2.1 - M.q l * (a.pt p).2.2) ∧ e.dOk signFuel = true ∧
      planePt M e.c e.N e.D ((a.pt p).2.1 - M.q l * (a.pt p).2.2) =
        (M.q c, (a.pt p).2.1 - M.q l * (a.pt p).2.2, (a.pt p).2.2) := by
  unfold faceCrossings at h
  obtain ⟨l', -, hl⟩ := List.exists_of_findSome?_eq_some h
  simp only [Option.map_eq_some_iff, Prod.mk.injEq] at hl
  obtain ⟨es', hes, rfl, rfl⟩ := hl
  generalize hq : a.pt p = q at hf
  obtain ⟨X, Y, Z⟩ := q
  simp only at hf ⊢
  subst hf
  have ev : ∀ r : Quadric, Quadric.evK M ((a.perm r).shear l') (M.q c, Y - M.q l' * Z, Z) =
      Quadric.evK M r p := by
    intro r
    rw [shear_ev, perm_ev, show Y - M.q l' * Z + M.q l' * Z = Y by grind, ← hq, pt_pt]
  obtain ⟨e, he, h1, h2, -, h4⟩ := proposeEdgeAny_complete M hes (by rw [ev]; exact hj)
    (by rw [ev]; exact hk)
  exact ⟨e, he, h1, h2, h4⟩

/-- A point of the box's boundary: in the closed box, and on one of its faces. -/
def OnBoxBd (lo hi : V3) (p : PK K) : Prop :=
  ∃ f ∈ boxFaces lo hi, (f.1.pt p).1 = M.q f.2

/-- **Every common zero on the box's boundary is a crossing sample of its face.** -/
theorem boxCrossings_complete {qj qk : Quadric} {lo hi : V3} {fuel : Nat}
    {cs : List (Axis × Rat × Rat × List ESample)} (h : boxCrossings qj qk lo hi fuel = some cs)
    {p : PK K} (hb : OnBoxBd M lo hi p) (hj : Quadric.evK M qj p = 0)
    (hk : Quadric.evK M qk p = 0) :
    ∃ r ∈ cs, ∃ e ∈ r.2.2.2, Denotes M e.y ((r.1.pt p).2.1 - M.q r.2.2.1 * (r.1.pt p).2.2) ∧
      e.dOk signFuel = true ∧ (r.1.pt p).1 = M.q r.2.1 := by
  obtain ⟨f, hf, hfp⟩ := hb
  obtain ⟨r, hr, hrl⟩ := mapM_mem h f hf
  simp only [Option.map_eq_some_iff] at hr
  obtain ⟨⟨l, es⟩, hfc, rfl⟩ := hr
  obtain ⟨e, he, h1, h2, -⟩ := faceCrossings_complete M hfc hfp hj hk
  exact ⟨_, hrl, e, he, h1, h2, hfp⟩

variable [DecidableRel (α := K) (· < ·)]

/-- **A crossing's covector is the covector of a real point.** -/
theorem faceCov_realised {qs : List Quadric} {a : Axis} {l : Rat} {e : ESample} {X : Cov}
    (h : faceCov qs a l e = some X) : RealCov M qs X := by
  unfold faceCov at h
  by_cases hc : (isoOkN e.y && e.dOk signFuel) = true
  · rw [if_pos hc] at h
    simp only [Bool.and_eq_true] at hc
    obtain ⟨α, hα⟩ := denotes_of_isoOkN M hc.1
    have := edgeCov_spec M hα hc.2 _ X h
    generalize planePt M e.c e.N e.D α = q at this
    obtain ⟨X', Y', Z'⟩ := q
    refine ⟨a.pt (X', Y' + M.q l * Z', Z'), ?_⟩
    rw [this]
    simp only [covOfK, List.map_map]
    apply List.map_congr_left
    intro r _
    simp only [Function.comp, shear_ev, perm_ev]
  · rw [if_neg hc] at h; cases h

end Model

-- the unit sphere and the plane z = 0 against the box [-1/2, 3/2] × [-1/2, 1/2] × [-1/2, 1/2]:
-- the circle meets the face planes x = -1/2 at y = ±√3/2 (off the face), and y = ±1/2 at
-- x = ±√3/2 (on the faces). The crossings are plane-wide; which lie in the face rectangle is a
-- comparison of the order, not of completeness
#guard (boxCrossings sphere1 planeZ ⟨-1/2, -1/2, -1/2⟩ ⟨3/2, 1/2, 1/2⟩ 30).map
  (fun cs => cs.map fun r => (r.2.2.1, r.2.2.2.length)) =
  some [(0, 2), (0, 0), (0, 2), (0, 2), (0, 0), (0, 0)]

end ForgeExact.DecideOnce.Curved
