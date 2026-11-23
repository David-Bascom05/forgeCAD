import ForgeExact.Flat

/-!
# Extrude: the prism of a planar region (research/sweep-spec/DESIGN.md §1)

An extrude's result, as a set: the region of the profile, placed in its sketch frame, moved along
the travel over an interval.

* `Frame3`: a sketch frame as the kernel holds it, the origin `o`, the in-plane axes `ex`, `ey`
  and the travel `v` (the plane's normal, or any direction off the plane for an extrude along a
  direction). Its numbers are the kernel's f64s, so they need not be orthonormal: only
  `det ≠ 0` (the three directions independent) is assumed.
* `crosses2`, `inRegion`: the region of a list of loops of points, by the even-odd rule along a
  ray toward +x from the point perturbed by `(0, ε)` (the same simulation of simplicity as the
  boolean's `crossesYZ`). For a profile the kernel accepts (simple loops, holes inside the
  outer loop and apart) this is its region: winding number 1.
* `Prism loops F lo hi`: the points `o + x·ex + y·ey + s·v` with `(x, y)` in the region and
  `lo < s < hi` (open, as the boolean's half-spaces are; the closed prism has the same `reg`).

Theorems:
* `coords_at`, `at_coords`: a frame with `det ≠ 0` is a coordinate system (Cramer's rule), so
* `prism_iff`: membership of the prism is the executable `prismMem` (the reference's test);
* `through_finite`: cutting or keeping a part with a prism whose range holds the part's whole
  extent along the travel is cutting or keeping it with the infinite prism. The kernel's finite
  "through everything" range (the part's reach plus a margin) is therefore exact, whatever
  margin is used.
* `range_*`: each extent's range (distance either way, symmetric, two-sided) as the kernel takes it.
-/

namespace ForgeExact.Swept

open ForgeExact Ref Flat

/-! ## Frames -/

/-- A sketch frame and a travel direction, in the kernel's own numbers. -/
structure Frame3 where
  o : V3
  ex : V3
  ey : V3
  v : V3
  deriving Inhabited, Repr

namespace Frame3

/-- The point at frame coordinates `(x, y, s)`. -/
def pt (F : Frame3) (x y s : Rat) : V3 := F.o + x * F.ex + y * F.ey + s * F.v

/-- The frame's volume: nonzero when its three directions are independent. -/
def det (F : Frame3) : Rat := F.ex.dot (F.ey.cross F.v)

/-- The frame coordinates of `p` (Cramer's rule). -/
def cx (F : Frame3) (p : V3) : Rat := (p - F.o).dot (F.ey.cross F.v) / F.det
def cy (F : Frame3) (p : V3) : Rat := (p - F.o).dot (F.v.cross F.ex) / F.det
def cs (F : Frame3) (p : V3) : Rat := (p - F.o).dot (F.ex.cross F.ey) / F.det

end Frame3

@[simp] theorem sub_x (a b : V3) : (a - b).x = a.x - b.x := rfl
@[simp] theorem sub_y (a b : V3) : (a - b).y = a.y - b.y := rfl
@[simp] theorem sub_z (a b : V3) : (a - b).z = a.z - b.z := rfl

theorem v3_ext {a b : V3} (hx : a.x = b.x) (hy : a.y = b.y) (hz : a.z = b.z) : a = b := by
  cases a; cases b; simp_all

/-- Coordinates read back: the point at `(x, y, s)` has coordinates `(x, y, s)`. -/
theorem coords_at (F : Frame3) (h : F.det ≠ 0) (x y s : Rat) :
    F.cx (F.pt x y s) = x ∧ F.cy (F.pt x y s) = y ∧ F.cs (F.pt x y s) = s := by
  obtain ⟨⟨ox, oy, oz⟩, ⟨ax, ay, az⟩, ⟨bx, by_, bz⟩, ⟨vx, vy, vz⟩⟩ := F
  simp only [Frame3.cx, Frame3.cy, Frame3.cs, Frame3.pt, Frame3.det, V3.dot, V3.cross] at h ⊢
  simp only [sub_x, sub_y, sub_z, add_x, add_y, add_z, smul_x, smul_y, smul_z]
  have hi := Rat.mul_inv_cancel _ h
  refine ⟨?_, ?_, ?_⟩ <;>
  · simp only [Rat.div_def]; grind

/-- Every point is at its coordinates. -/
theorem at_coords (F : Frame3) (h : F.det ≠ 0) (p : V3) : F.pt (F.cx p) (F.cy p) (F.cs p) = p := by
  obtain ⟨⟨ox, oy, oz⟩, ⟨ax, ay, az⟩, ⟨bx, by_, bz⟩, ⟨vx, vy, vz⟩⟩ := F
  obtain ⟨px, py, pz⟩ := p
  simp only [Frame3.cx, Frame3.cy, Frame3.cs, Frame3.pt, Frame3.det, V3.dot, V3.cross] at h ⊢
  apply v3_ext <;> simp only [sub_x, sub_y, sub_z, add_x, add_y, add_z, smul_x, smul_y, smul_z]
  all_goals
    simp only [Rat.div_def]
    have hi := Rat.mul_inv_cancel _ h
    grind

/-! ## Regions -/

/-- Does the ray from `q` toward +x (its origin perturbed by `(0, ε)`) cross segment `ab`? -/
def crosses2 (q a b : Rat × Rat) : Bool :=
  let aa := decide (a.2 > q.2)
  let ab := decide (b.2 > q.2)
  if aa == ab then false
  else
    let dy := b.2 - a.2
    let d0 := (b.1 - a.1) * (q.2 - a.2) - (q.1 - a.1) * dy
    let s := if d0 != 0 then sgn d0 else sgn (-dy)
    s * sgn dy > 0

/-- The edges of a closed loop: each point to the next, the last back to the first. -/
def edges (l : List (Rat × Rat)) : List ((Rat × Rat) × (Rat × Rat)) :=
  match l with
  | [] => []
  | a :: _ => (l.zip (l.drop 1)) ++ [(l.getLast?.getD a, a)]

/-- How many edges of the loops the ray from `q` crosses. -/
def crossings (loops : List (List (Rat × Rat))) (q : Rat × Rat) : Nat :=
  (loops.flatMap edges).countP fun e => crosses2 q e.1 e.2

/-- The region: an odd number of crossings. -/
def inRegion (loops : List (List (Rat × Rat))) (q : Rat × Rat) : Bool := crossings loops q % 2 == 1

/-! ## The prism -/

/-- **L1, extrude.** The region of `loops` in frame `F`, moved along `F.v` over `(lo, hi)`. -/
def Prism (loops : List (List (Rat × Rat))) (F : Frame3) (lo hi : Rat) : Pred V3 :=
  fun p => ∃ x y s, p = F.pt x y s ∧ inRegion loops (x, y) = true ∧ lo < s ∧ s < hi

/-- The infinite prism (through everything, both ways). -/
def PrismAll (loops : List (List (Rat × Rat))) (F : Frame3) : Pred V3 :=
  fun p => ∃ x y s, p = F.pt x y s ∧ inRegion loops (x, y) = true

/-- The reference's membership test: the point's coordinates, then the region and the range. -/
def prismMem (loops : List (List (Rat × Rat))) (F : Frame3) (lo hi : Rat) (p : V3) : Bool :=
  inRegion loops (F.cx p, F.cy p) && decide (lo < F.cs p) && decide (F.cs p < hi)

theorem prism_iff (loops : List (List (Rat × Rat))) (F : Frame3) (h : F.det ≠ 0) (lo hi : Rat)
    (p : V3) : Prism loops F lo hi p ↔ prismMem loops F lo hi p = true := by
  constructor
  · rintro ⟨x, y, s, rfl, hr, hlo, hhi⟩
    obtain ⟨e1, e2, e3⟩ := coords_at F h x y s
    simp [prismMem, e1, e2, e3, hr, hlo, hhi]
  · intro hm
    simp only [prismMem, Bool.and_eq_true, decide_eq_true_eq] at hm
    exact ⟨_, _, _, (at_coords F h p).symm, hm.1.1, hm.1.2, hm.2⟩

theorem prismAll_iff (loops : List (List (Rat × Rat))) (F : Frame3) (h : F.det ≠ 0) (p : V3) :
    PrismAll loops F p ↔ inRegion loops (F.cx p, F.cy p) = true := by
  constructor
  · rintro ⟨x, y, s, rfl, hr⟩
    obtain ⟨e1, e2, _⟩ := coords_at F h x y s
    rw [e1, e2]; exact hr
  · intro hr; exact ⟨_, _, _, (at_coords F h p).symm, hr⟩

/-- **Through everything is a finite prism.** If the part lies strictly between `lo` and `hi`
along the travel, it meets the prism over `(lo, hi)` exactly where it meets the infinite one:
cutting with either, or keeping with either, gives the same set. -/
theorem through_finite (loops : List (List (Rat × Rat))) (F : Frame3) (h : F.det ≠ 0)
    (part : Pred V3) (lo hi : Rat) (hp : ∀ p, part p → lo < F.cs p ∧ F.cs p < hi) :
    part.inter (Prism loops F lo hi) = part.inter (PrismAll loops F) ∧
      part.diff (Prism loops F lo hi) = part.diff (PrismAll loops F) := by
  have key : ∀ p, part p → (Prism loops F lo hi p ↔ PrismAll loops F p) := fun p hpart => by
    rw [prism_iff loops F h, prismAll_iff loops F h]
    simp only [prismMem, Bool.and_eq_true, decide_eq_true_eq]
    have := hp p hpart
    constructor
    · exact fun hm => hm.1.1
    · exact fun hr => ⟨⟨hr, this.1⟩, this.2⟩
  constructor
  · funext p; apply propext
    simp only [Pred.inter]
    constructor
    · exact fun ⟨a, b⟩ => ⟨a, (key p a).mp b⟩
    · exact fun ⟨a, b⟩ => ⟨a, (key p a).mpr b⟩
  · funext p; apply propext
    simp only [Pred.diff]
    constructor
    · exact fun ⟨a, b⟩ => ⟨a, fun c => b ((key p a).mpr c)⟩
    · exact fun ⟨a, b⟩ => ⟨a, fun c => b ((key p a).mp c)⟩

/-! ## Extents -/

/-- How far an extrude goes, as the kernel's `Extent` (the variants that are one prism with a
range fixed by the feature alone). -/
inductive Extent
  | distance (d : Rat)
  | symmetric (d : Rat)
  | twoSided (forward back : Rat)

/-- Each extent's range along the normal. -/
def Extent.range : Extent → Rat × Rat
  | .distance d => if 0 < d then (0, d) else (d, 0)
  | .symmetric d => (-d / 2, d / 2)
  | .twoSided f b => (-b, f)

/-- The kernel builds an extent as one prism over its range when the range is not empty: a
two-sided extrude is one prism, not the union of two (and so has no face at the sketch). -/
theorem range_twoSided_union (loops : List (List (Rat × Rat))) (F : Frame3) (h : F.det ≠ 0)
    (f b : Rat) (hf : 0 < f) (hb : 0 < b) (p : V3)
    (hp : F.cs p ≠ 0) :
    Prism loops F (-b) f p ↔ (Prism loops F 0 f p ∨ Prism loops F (-b) 0 p) := by
  simp only [prism_iff loops F h, prismMem, Bool.and_eq_true, decide_eq_true_eq]
  constructor
  · rintro ⟨⟨hr, h1⟩, h2⟩
    by_cases hs : 0 < F.cs p
    · exact Or.inl ⟨⟨hr, hs⟩, h2⟩
    · exact Or.inr ⟨⟨hr, h1⟩, by grind⟩
  · rintro (⟨⟨hr, h1⟩, h2⟩ | ⟨⟨hr, h1⟩, h2⟩)
    · exact ⟨⟨hr, by grind⟩, h2⟩
    · exact ⟨⟨hr, h1⟩, by grind⟩

end ForgeExact.Swept
