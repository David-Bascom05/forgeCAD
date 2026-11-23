import Mathlib
import Tess.Sag

/-!
# L1: what a valid body's tessellation is

The tessellation of a valid body at chord c > 0 (research/tessellation-proof, kernel/forge-mesh
tessellate.rs) is a list of triangles over vertex ids, each the triangle of one face, such that:

* (a) it is closed and consistently oriented, `openEdges = []`, with one fan round each vertex:
  the mesh image of check's closed manifold (TopoCheck's `twoUses` and `oneFan`); the two-
  tetrahedra pinch passes `openEdges` but has two fans at its shared vertex, so both are stated;
* (c) each vertex of a face's triangle lies on the face's surface;
* (d) each point of a face's triangle lies within c of the face's surface, measured by the
  kernel's signed distance (`implicit`, forge-geom surface.rs).

(b), the faces sharing their edges' cut points, is what makes (a) possible across faces; it is
a fact about the vertex ids, carried by (a).

`directed`, `openEdges`, `corners` and `fans` are verify/forge-mesh-check's (MeshCheck/Mesh.lean),
stated again here word for word so this project stands alone.
-/

noncomputable section

namespace Tess

abbrev V := EuclideanSpace ℝ (Fin 3)
abbrev Tri := Nat × Nat × Nat

/-- mesh.rs: each triangle's three directed edges (MeshCheck.directed). -/
def directed (ts : List Tri) : List (Nat × Nat) :=
  ts.flatMap fun ⟨a, b, c⟩ => [(a, b), (b, c), (c, a)]

def rev (e : Nat × Nat) : Nat × Nat := (e.2, e.1)

/-- The directed edges that occur twice, and those with no reverse (MeshCheck.openEdges). -/
def openEdges (ts : List Tri) : List (Nat × Nat) :=
  let d := directed ts
  (d.filter fun e => d.count e ≥ 2) ++ d.filter fun e => rev e ∉ d

/-- Round vertex v, each triangle's step from the corner before it to the corner after
(MeshCheck.corners). -/
def corners (ts : List Tri) (v : Nat) : List (Nat × Nat) :=
  ts.filterMap fun ⟨a, b, c⟩ =>
    if a = v then some (b, c) else if b = v then some (c, a) else if c = v then some (a, b) else none

/-- The fans at v (MeshCheck.fans). -/
def fans (ts : List Tri) (v : Nat) : Nat :=
  go (corners ts v) (corners ts v).length
where
  go : List (Nat × Nat) → Nat → Nat
    | [], _ => 0
    | _, 0 => 0
    | s :: rest, fuel + 1 =>
      let fan := grow rest [s] (rest.length + 1)
      1 + go (rest.filter fun x => x ∉ fan) fuel
  grow : List (Nat × Nat) → List (Nat × Nat) → Nat → List (Nat × Nat)
    | _, acc, 0 => acc
    | pool, acc, fuel + 1 =>
      let next := pool.filter fun x => x ∉ acc ∧ acc.any fun y => y.2 = x.1 ∨ x.2 = y.1
      if next = [] then acc else grow pool (acc ++ next) fuel

/-- A face's surface, as the kernel draws it: an origin `o` and unit axis `a` where it has
one. -/
inductive Kind
  | plane (o n : V)
  | cylinder (o a : V) (r : ℝ)
  | cone (o a : V) (r₀ ca sa : ℝ)
  | sphere (o : V) (r : ℝ)
  | torus (o a : V) (major minor : ℝ)
  /-- a surface given by its own signed distance (a procedural face) -/
  | other (d : V → ℝ)

/-- A point's height along the axis and its distance from it. -/
def height (o a p : V) : ℝ := inner ℝ (p - o) a
def fromAxis (o a p : V) : ℝ := ‖(p - o) - height o a p • a‖

/-- The kernel's signed distance to a face's surface (surface.rs `implicit`). -/
def sag : Kind → V → ℝ
  | .plane o n, p => inner ℝ (p - o) n
  | .cylinder o a r, p => fromAxis o a p - r
  | .cone o a r₀ ca sa, p => (fromAxis o a p - r₀) * ca - height o a p * sa
  | .sphere o r, p => ‖p - o‖ - r
  | .torus o a R r, p =>
      Real.sqrt ((fromAxis o a p - R) ^ 2 + height o a p ^ 2) - r
  | .other d, p => d p

/-- A point of a triangle, by weights. -/
def at3 (x₁ x₂ x₃ : V) (m₁ m₂ m₃ : ℝ) : V := m₁ • x₁ + m₂ • x₂ + m₃ • x₃

/-- **The spec.** A tessellation (positions `pos`, triangles `tris`, the face of each `faceOf`)
of faces `face` at chord `c`. -/
structure Spec (c : ℝ) (face : Nat → Kind) (pos : Nat → V) (tris : List Tri) (faceOf : Nat → Nat) :
    Prop where
  closed : openEdges tris = []
  oneFan : ∀ v, fans tris v ≤ 1
  onSurface : ∀ i (h : i < tris.length), let t := tris[i]
    sag (face (faceOf i)) (pos t.1) = 0 ∧ sag (face (faceOf i)) (pos t.2.1) = 0 ∧
      sag (face (faceOf i)) (pos t.2.2) = 0
  within : ∀ i (h : i < tris.length), let t := tris[i]
    ∀ m₁ m₂ m₃ : ℝ, 0 ≤ m₁ → 0 ≤ m₂ → 0 ≤ m₃ → m₁ + m₂ + m₃ = 1 →
      |sag (face (faceOf i)) (at3 (pos t.1) (pos t.2.1) (pos t.2.2) m₁ m₂ m₃)| ≤ c

/-- A point of a triangle with its corners on a sphere lies in the ball: its signed distance is
at most 0. -/
theorem sphere_inside (o x₁ x₂ x₃ : V) (r m₁ m₂ m₃ : ℝ)
    (h₁ : ‖x₁ - o‖ = r) (h₂ : ‖x₂ - o‖ = r) (h₃ : ‖x₃ - o‖ = r)
    (p₁ : 0 ≤ m₁) (p₂ : 0 ≤ m₂) (p₃ : 0 ≤ m₃) (ms : m₁ + m₂ + m₃ = 1) :
    ‖at3 x₁ x₂ x₃ m₁ m₂ m₃ - o‖ ≤ r := by
  have : at3 x₁ x₂ x₃ m₁ m₂ m₃ - o = m₁ • (x₁ - o) + m₂ • (x₂ - o) + m₃ • (x₃ - o) := by
    have : m₁ • o + m₂ • o + m₃ • o = o := by rw [← add_smul, ← add_smul, ms, one_smul]
    simp only [at3, smul_sub]
    calc m₁ • x₁ + m₂ • x₂ + m₃ • x₃ - o
        = m₁ • x₁ + m₂ • x₂ + m₃ • x₃ - (m₁ • o + m₂ • o + m₃ • o) := by rw [this]
      _ = m₁ • x₁ - m₁ • o + (m₂ • x₂ - m₂ • o) + (m₃ • x₃ - m₃ • o) := by abel
  rw [this]
  calc ‖m₁ • (x₁ - o) + m₂ • (x₂ - o) + m₃ • (x₃ - o)‖
      ≤ ‖m₁ • (x₁ - o)‖ + ‖m₂ • (x₂ - o)‖ + ‖m₃ • (x₃ - o)‖ := norm_add₃_le
    _ = m₁ * r + m₂ * r + m₃ * r := by
        rw [norm_smul, norm_smul, norm_smul, Real.norm_of_nonneg p₁, Real.norm_of_nonneg p₂,
          Real.norm_of_nonneg p₃, h₁, h₂, h₃]
    _ = r := by rw [← add_mul, ← add_mul, ms, one_mul]

/-- **The sphere's rule meets (d).** A triangle of a sphere face accepted by the rule (its
corners on the sphere, its edges' middles within c, and its foot within c where it lies in it)
is within c of the sphere at every point, as `Spec.within` asks. -/
theorem sphere_within (o x₁ x₂ x₃ f : V) (l₁ l₂ l₃ r c : ℝ) (hc : 0 ≤ c)
    (h₁ : ‖x₁ - o‖ = r) (h₂ : ‖x₂ - o‖ = r) (h₃ : ‖x₃ - o‖ = r)
    (hs : l₁ + l₂ + l₃ = 1) (hf : f = l₁ • x₁ + l₂ • x₂ + l₃ • x₃)
    (q₁ : inner ℝ (f - o) (x₁ - f) = 0) (q₂ : inner ℝ (f - o) (x₂ - f) = 0)
    (q₃ : inner ℝ (f - o) (x₃ - f) = 0)
    (m₁₂ : r - ‖(x₁ + (1 / 2 : ℝ) • (x₂ - x₁)) - o‖ ≤ c)
    (m₁₃ : r - ‖(x₁ + (1 / 2 : ℝ) • (x₃ - x₁)) - o‖ ≤ c)
    (m₂₃ : r - ‖(x₂ + (1 / 2 : ℝ) • (x₃ - x₂)) - o‖ ≤ c)
    (foot : 0 ≤ l₁ → 0 ≤ l₂ → 0 ≤ l₃ → r - ‖f - o‖ ≤ c)
    (m₁ m₂ m₃ : ℝ) (p₁ : 0 ≤ m₁) (p₂ : 0 ≤ m₂) (p₃ : 0 ≤ m₃) (ms : m₁ + m₂ + m₃ = 1) :
    |sag (.sphere o r) (at3 x₁ x₂ x₃ m₁ m₂ m₃)| ≤ c := by
  simp only [sag]
  have deep := Sag.sphere_exact o x₁ x₂ x₃ f l₁ l₂ l₃ m₁ m₂ m₃ r c h₁ h₂ h₃ hs hf q₁ q₂ q₃ ms p₁ p₂
    p₃ m₁₂ m₁₃ m₂₃ foot
  have ins := sphere_inside o x₁ x₂ x₃ r m₁ m₂ m₃ h₁ h₂ h₃ p₁ p₂ p₃ ms
  rw [abs_le]
  constructor
  · simp only [at3] at deep ⊢; linarith
  · linarith

end Tess

namespace Tess

/-- The two tetrahedra sharing a vertex (MeshCheck.pinch): closed and oriented, but two fans at
vertex 0, so not a valid body's mesh by `Spec.oneFan`. -/
def pinch : List Tri :=
  [(0, 2, 1), (0, 1, 3), (0, 3, 2), (1, 2, 3),
   (0, 5, 4), (0, 4, 6), (0, 6, 5), (4, 5, 6)]

theorem pinch_is_not_one_fan : openEdges pinch = [] ∧ fans pinch 0 = 2 := by decide

end Tess
