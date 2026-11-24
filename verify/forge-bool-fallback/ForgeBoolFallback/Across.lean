/-!
# `across()`'s three `Internal` refusals, from H.curveOK

kernel/forge-bool/src/boolean.rs `across(body, f, c)` builds a surface standing across face `f`
along edge `c`, at the edge's midpoint `t`, `d = curve.deriv(t)`:

* `Lost::NoNormal` when `s.normal(s.project(d.p))` is `None`;
* `Lost::DegenerateEdge` (ellipse branch) when the leaning axis
  `z·(m/M) + x·sqrt(max(0, 1 − (m/M)²))` does not normalize;
* `Lost::DegenerateEdge` (default branch) when `d.d1 × n` does not normalize;
* (`Lost::DegenerateEdge`, parabola branch: `Ruled::new` folds. Not covered here: UNKNOWN.)

`Vec3::normalize` (forge-math vec.rs) is `Some` exactly when the length is `> 0` and finite.

**H.curveOK** (a hypothesis on the operands, discharged by the features that make them; measured,
see the ledger): at each edge's midpoint the face's surface has a normal; the edge's tangent is
finite, non-zero and not along that normal; an ellipse edge's radii are finite with major > 0.

This file models `across`'s decisions over abstract vectors, with the one IEEE fact it needs as an
explicit hypothesis (`NormalizeOK`: a finite vector of non-zero length normalizes), and proves
`across_ok`: under H.curveOK none of the three refusals is returned. The ellipse axis's length is
not zero because its square is at least 1 (`axis_sq_ge_one`): with `a = (m/M)²`,
`a + max(0, 1 − a) ≥ 1`, a statement linear in `a`, proven here over `Int` by the same case split
that proves it over ℝ (the f64 rounding of that sum is part of `NormalizeOK`'s premise, open).
Core Lean only.
-/

namespace ForgeBoolFallback.Across

/-- **The ellipse axis**: `a + max(0, 1 − a) ≥ 1` (here `a = (m/M)²`, any finite value). -/
theorem axis_sq_ge_one (a : Int) : a + max 0 (1 - a) ≥ 1 := by
  by_cases h : a ≤ 1
  · have : max 0 (1 - a) = 1 - a := by omega
    omega
  · have : max 0 (1 - a) = 0 := by omega
    omega

/-- The edge's kind, as `across` branches on it (circles never refuse). -/
inductive Kind where
  | circle
  | ellipseLeaning
  | ellipseFlat
  | parabolaSquare
  | other
  deriving DecidableEq

/-- What `across` reads at the edge's midpoint, abstractly. -/
structure At (V : Type) where
  kind : Kind
  /-- `s.normal(s.project(d.p))` -/
  normal : Option V
  /-- the vector the branch normalizes: the ellipse's axis or `d1 × n` -/
  toNormalize : V → V

/-- The refusals `across` can return. -/
inductive Refusal where
  | noNormal
  | degenerate
  deriving DecidableEq

/-- `across`'s decisions: no normal, then the branch's normalize (`normalize` returns `none` on a
zero or non-finite vector); the parabola branch's own refusal (`Ruled::new`) is `folds`. -/
def across {V U : Type} (normalize : V → Option U) (folds : Bool) (x : At V) : Except Refusal Unit :=
  match x.normal with
  | none => .error .noNormal
  | some n =>
    match x.kind with
    | .circle | .ellipseFlat => .ok ()
    | .parabolaSquare => if folds then .error .degenerate else .ok ()
    | .ellipseLeaning | .other =>
      match normalize (x.toNormalize n) with
      | none => .error .degenerate
      | some _ => .ok ()

/-- `Vec3::normalize` succeeds on a vector that is finite and not zero (vec.rs: `l > 0.0 &&
l.is_finite()`; the f64 `length` of a finite non-zero vector is positive and finite unless it
overflows, which `finite` here excludes). -/
def NormalizeOK {V U : Type} (normalize : V → Option U) (good : V → Prop) : Prop :=
  ∀ v, good v → (normalize v).isSome

/-- **H.curveOK** at this edge: a normal exists, and (on the branches that normalize) the vector
normalized is a good one: `d1 × n` with `d1` finite, non-zero and off `n`, or the ellipse's axis
with finite radii (its square at least 1, `axis_sq_ge_one`). -/
def CurveOK {V : Type} (good : V → Prop) (x : At V) : Prop :=
  ∃ n, x.normal = some n ∧
    (x.kind = .ellipseLeaning ∨ x.kind = .other → good (x.toNormalize n))

/-- **`across_ok`**: under H.curveOK and `NormalizeOK`, `across` returns neither `NoNormal` nor
the ellipse or default branch's `DegenerateEdge`; only the parabola branch's fold remains. -/
theorem across_ok {V U : Type} {normalize : V → Option U} {good : V → Prop}
    (hN : NormalizeOK normalize good) (x : At V) (h : CurveOK good x) :
    across normalize false x = .ok () := by
  obtain ⟨n, hn, hg⟩ := h
  unfold across
  rw [hn]
  cases hk : x.kind with
  | circle => rfl
  | ellipseFlat => rfl
  | parabolaSquare => rfl
  | ellipseLeaning =>
    have hs := hN _ (hg (Or.inl hk))
    cases e : normalize (x.toNormalize n) with
    | none => rw [e] at hs; cases hs
    | some _ => simp [e]
  | other =>
    have hs := hN _ (hg (Or.inr hk))
    cases e : normalize (x.toNormalize n) with
    | none => rw [e] at hs; cases hs
    | some _ => simp [e]

end ForgeBoolFallback.Across
