import ForgeWeld.Member

/-!
# A section's exact measures: its support, and its moments by Green's theorem

Over ℚ, for sections bounded by straight sides (the polygonal part of forge_weld::section; arcs,
whose points need square roots, are the L2 step over ℚ(√·)).

**Support.** A section lies in the hull of its corners and holds them (an outline's corners are on
it, its region is inside their hull). Then its support in any direction is the largest value of
that direction over the corners, and a corner attains it (`support_of_corners`): what
`Section::support` computes from the loop's ends.

**Moments.** forge_weld computes a loop's area and first moments as sums over its edges of one
1-form each: `½(x dy - y dx)`, `½ x² dy`, `-½ y² dx` (Green's theorem). On a straight edge these
are `cross/2`, `dy (px² + px qx + qx²)/6` and `-dx (py² + py qy + qy²)/6` (`lineMx`, `lineMy`).
The usual polygon formulas sum the triangles fanned from the origin, `(px + qx) cross / 6`
(`fanMx`, `fanMy`): a different 1-form. Over a closed loop the two sums agree
(`line_eq_fan_x`, `line_eq_fan_y`: edge by edge they differ by an exact difference `F q - F p`,
which telescopes round the loop), but edge by edge they do not (`fan_ne_line_on_an_edge`): a
loop mixing the two (arcs by Green's form, lines by the fan) is wrong. That was a bug forge-weld's
first centroid had; `Section::moments` now uses Green's form on every piece.
-/

namespace ForgeWeld

/-! ## Support -/

/-- The hull of a list of points of the plane, as the points reached from the corners by
segments. -/
inductive InHull (vs : List (Rat × Rat)) : Rat × Rat → Prop
  | corner {p} : p ∈ vs → InHull vs p
  | seg {p q} (t : Rat) : InHull vs p → InHull vs q → 0 ≤ t → t ≤ 1 →
      InHull vs (p.1 + t * (q.1 - p.1), p.2 + t * (q.2 - p.2))

/-- The largest value of `f` over a nonempty list. -/
def maxOf (f : Rat × Rat → Rat) : List (Rat × Rat) → Rat
  | [] => 0
  | [p] => f p
  | p :: q :: r => if f p ≤ maxOf f (q :: r) then maxOf f (q :: r) else f p

theorem le_maxOf (f : Rat × Rat → Rat) : ∀ {vs : List (Rat × Rat)} {p}, p ∈ vs → f p ≤ maxOf f vs
  | [_], _, h => by simp at h; subst h; simp [maxOf]
  | _ :: q :: r, _, h => by
    have ih := fun {p} (h : p ∈ q :: r) => le_maxOf f (vs := q :: r) (p := p) h
    simp only [maxOf]
    rcases List.mem_cons.mp h with h | h
    · subst h; split <;> grind
    · have := ih h; split <;> grind

theorem maxOf_mem (f : Rat × Rat → Rat) : ∀ {vs : List (Rat × Rat)}, vs ≠ [] →
    ∃ p ∈ vs, f p = maxOf f vs
  | [p], _ => ⟨p, by simp, rfl⟩
  | p :: q :: r, _ => by
    simp only [maxOf]
    obtain ⟨m, hm, he⟩ := maxOf_mem f (vs := q :: r) (by simp)
    split
    · exact ⟨m, List.mem_cons_of_mem _ hm, he⟩
    · exact ⟨p, by simp, rfl⟩

/-- A linear function over the hull is at most its largest value at a corner. -/
theorem hull_le (wu wv : Rat) {vs : List (Rat × Rat)} {p : Rat × Rat} (hp : InHull vs p) :
    wu * p.1 + wv * p.2 ≤ maxOf (fun q => wu * q.1 + wv * q.2) vs := by
  induction hp with
  | corner h => exact le_maxOf (fun q => wu * q.1 + wv * q.2) h
  | seg t _ _ h0 h1 ihp ihq =>
    rename_i p q _ _
    -- the value on the segment is (1 - t) f(p) + t f(q)
    have e : wu * (p.1 + t * (q.1 - p.1)) + wv * (p.2 + t * (q.2 - p.2)) =
        (1 - t) * (wu * p.1 + wv * p.2) + t * (wu * q.1 + wv * q.2) := by grind
    simp only
    rw [e]
    have a := Rat.mul_le_mul_of_nonneg_left ihp (by grind : (0 : Rat) ≤ 1 - t)
    have b := Rat.mul_le_mul_of_nonneg_left ihq h0
    grind

/-- **The support of a section is the largest value over its corners**, attained at one: where the
section lies in its corners' hull and holds them. -/
theorem support_of_corners {S : Rat → Rat → Prop} {vs : List (Rat × Rat)} (hne : vs ≠ [])
    (hin : ∀ u v, S u v → InHull vs (u, v)) (hcorners : ∀ p ∈ vs, S p.1 p.2) (wu wv : Rat) :
    Upper S wu wv (maxOf (fun q => wu * q.1 + wv * q.2) vs) ∧
      ∃ p ∈ vs, S p.1 p.2 ∧ wu * p.1 + wv * p.2 = maxOf (fun q => wu * q.1 + wv * q.2) vs := by
  refine ⟨fun u v h => hull_le wu wv (hin u v h), ?_⟩
  obtain ⟨p, hp, he⟩ := maxOf_mem (fun q => wu * q.1 + wv * q.2) hne
  exact ⟨p, hp, hcorners p hp, he⟩

/-! ## Moments by Green's theorem -/

/-- A sum over consecutive pairs of a list. -/
def pathSum (f : Rat × Rat → Rat × Rat → Rat) : List (Rat × Rat) → Rat
  | p :: q :: r => f p q + pathSum f (q :: r)
  | _ => 0

/-- Over a closed loop: the last point back to the first. -/
def loopSum (f : Rat × Rat → Rat × Rat → Rat) : List (Rat × Rat) → Rat
  | [] => 0
  | p :: r => pathSum f (p :: r ++ [p])

/-- An exact difference telescopes along a path. -/
theorem pathSum_exact (F : Rat × Rat → Rat) :
    ∀ (p : Rat × Rat) (r : List (Rat × Rat)),
      pathSum (fun a b => F b - F a) (p :: r) = F ((p :: r).getLast (by simp)) - F p
  | p, [] => by simp [pathSum]; grind
  | p, q :: r => by
    simp only [pathSum]
    rw [pathSum_exact F q r]
    simp [List.getLast_cons]
    grind

/-- And vanishes round a closed loop. -/
theorem loopSum_exact (F : Rat × Rat → Rat) (l : List (Rat × Rat)) :
    loopSum (fun a b => F b - F a) l = 0 := by
  cases l with
  | nil => rfl
  | cons p r =>
    show pathSum (fun a b => F b - F a) (p :: (r ++ [p])) = 0
    rw [pathSum_exact F p (r ++ [p])]
    have : (p :: (r ++ [p])).getLast (by simp) = p := by
      simp [List.getLast_cons]
    rw [this]; grind

theorem pathSum_sub (f g : Rat × Rat → Rat × Rat → Rat) :
    ∀ l : List (Rat × Rat), pathSum (fun a b => f a b - g a b) l = pathSum f l - pathSum g l
  | p :: q :: r => by simp only [pathSum]; rw [pathSum_sub f g (q :: r)]; grind
  | [] => by simp [pathSum]; grind
  | [_] => by simp [pathSum]; grind

theorem loopSum_sub (f g : Rat × Rat → Rat × Rat → Rat) (l : List (Rat × Rat)) :
    loopSum (fun a b => f a b - g a b) l = loopSum f l - loopSum g l := by
  cases l with
  | nil => simp [loopSum]; grind
  | cons p r => simp only [loopSum]; exact pathSum_sub f g _

/-- `½ ∮ x² dy` on a straight edge from `p` to `q`. -/
def lineMx (p q : Rat × Rat) : Rat := (q.2 - p.2) * (p.1 * p.1 + p.1 * q.1 + q.1 * q.1) / 6
/-- `-½ ∮ y² dx` on a straight edge. -/
def lineMy (p q : Rat × Rat) : Rat := -(q.1 - p.1) * (p.2 * p.2 + p.2 * q.2 + q.2 * q.2) / 6
/-- The triangle fanned from the origin to the edge: its first moments. -/
def fanMx (p q : Rat × Rat) : Rat := (p.1 + q.1) * (p.1 * q.2 - q.1 * p.2) / 6
def fanMy (p q : Rat × Rat) : Rat := (p.2 + q.2) * (p.1 * q.2 - q.1 * p.2) / 6

/-- Edge by edge, Green's form and the fan differ by an exact difference. -/
theorem lineMx_sub_fanMx (p q : Rat × Rat) :
    lineMx p q - fanMx p q = q.1 * q.1 * q.2 / 6 - p.1 * p.1 * p.2 / 6 := by
  simp only [lineMx, fanMx]; grind

theorem lineMy_sub_fanMy (p q : Rat × Rat) :
    lineMy p q - fanMy p q = -(q.1 * q.2 * q.2) / 6 - -(p.1 * p.2 * p.2) / 6 := by
  simp only [lineMy, fanMy]; grind

/-- **Round a closed loop the two agree**: either gives the loop's first moment in x. -/
theorem line_eq_fan_x (l : List (Rat × Rat)) : loopSum lineMx l = loopSum fanMx l := by
  have h := loopSum_exact (fun p => p.1 * p.1 * p.2 / 6) l
  have e : (fun a b => (fun p : Rat × Rat => p.1 * p.1 * p.2 / 6) b -
      (fun p : Rat × Rat => p.1 * p.1 * p.2 / 6) a) = fun a b => lineMx a b - fanMx a b := by
    funext a b; rw [lineMx_sub_fanMx]
  rw [e, loopSum_sub] at h
  grind

theorem line_eq_fan_y (l : List (Rat × Rat)) : loopSum lineMy l = loopSum fanMy l := by
  have h := loopSum_exact (fun p => -(p.1 * p.2 * p.2) / 6) l
  have e : (fun a b => (fun p : Rat × Rat => -(p.1 * p.2 * p.2) / 6) b -
      (fun p : Rat × Rat => -(p.1 * p.2 * p.2) / 6) a) = fun a b => lineMy a b - fanMy a b := by
    funext a b; rw [lineMy_sub_fanMy]
  rw [e, loopSum_sub] at h
  grind

/-- **But not edge by edge**: one edge's terms differ, so a sum taking some pieces by one form and
some by the other is not the moment. -/
theorem fan_ne_line_on_an_edge : lineMx (0, 0) (1, 1) ≠ fanMx (0, 0) (1, 1) := by
  simp only [lineMx, fanMx]; grind

end ForgeWeld
