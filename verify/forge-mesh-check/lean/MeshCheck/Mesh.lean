import Mathlib

/-!
# What `TriMesh::open_edges` empty means (kernel/forge-mesh/src/mesh.rs, at 8250ebb)

A mesh is a list of triangles over vertex ids. `open_edges` (mesh.rs:57–76) lists the directed
edges that occur twice, and those whose reverse does not occur. Its doc says empty means "closed,
manifold and consistently oriented".

Proven:
* `closed_of_open_edges`: empty means every directed edge occurs once and its reverse occurs, so
  every edge has exactly two triangles, walking it opposite ways: closed and consistently oriented;
* `area_vectors_cancel`: the triangles' area vectors then sum to zero;
* `volume_translate`: so `volume()` (mesh.rs:24–33) does not depend on where the origin is;
* `pinch_passes_open_edges`: but it is not "manifold". Two tetrahedra sharing one vertex pass
  `open_edges`, with two fans of triangles at that vertex. A vertex's link being one cycle is a
  separate test (`fans`, run on the kernel's meshes in tests/mesh.rs).
-/

noncomputable section
open Matrix

namespace MeshCheck

abbrev Tri := Nat × Nat × Nat

/-- mesh.rs:58–62: each triangle's three directed edges. -/
def directed (ts : List Tri) : List (Nat × Nat) :=
  ts.flatMap fun ⟨a, b, c⟩ => [(a, b), (b, c), (c, a)]

def rev (e : Nat × Nat) : Nat × Nat := (e.2, e.1)

/-- mesh.rs:63–75, as a set: the directed edges that occur twice, and those with no reverse. -/
def openEdges (ts : List Tri) : List (Nat × Nat) :=
  let d := directed ts
  (d.filter fun e => d.count e ≥ 2) ++ d.filter fun e => rev e ∉ d

theorem closed_of_open_edges (ts : List Tri) (h : openEdges ts = []) :
    (directed ts).Nodup ∧ ∀ e ∈ directed ts, rev e ∈ directed ts := by
  unfold openEdges at h
  rw [List.append_eq_nil_iff, List.filter_eq_nil_iff, List.filter_eq_nil_iff] at h
  refine ⟨List.nodup_iff_count_le_one.2 fun e => ?_, fun e he => ?_⟩
  · by_cases he : e ∈ directed ts
    · have := h.1 e he; simp at this; omega
    · rw [List.count_eq_zero_of_not_mem he]; omega
  · have := h.2 e he; simpa using this

/-! ## Area vectors and volume -/

abbrev V3 := Fin 3 → ℝ

/-- A triangle's doubled area vector, (b − a) × (c − a) = a×b + b×c + c×a. -/
def areaVec (p : Nat → V3) (t : Tri) : V3 := p t.1 ⨯₃ p t.2.1 + p t.2.1 ⨯₃ p t.2.2 + p t.2.2 ⨯₃ p t.1

theorem areaVec_eq (p : Nat → V3) (t : Tri) :
    areaVec p t = (p t.2.1 - p t.1) ⨯₃ (p t.2.2 - p t.1) := by
  ext i; fin_cases i <;> simp [areaVec, cross_apply] <;> ring

theorem sum_areaVec (p : Nat → V3) (ts : List Tri) :
    (ts.map (areaVec p)).sum = ((directed ts).map fun e => p e.1 ⨯₃ p e.2).sum := by
  induction ts with
  | nil => simp [directed]
  | cons t ts ih =>
    simp only [List.map_cons, List.sum_cons, ih, directed, List.flatMap_cons, List.map_append,
      List.sum_append]
    simp [areaVec, add_assoc]

/-- On a closed, consistently oriented mesh the area vectors cancel. -/
theorem area_vectors_cancel (p : Nat → V3) (ts : List Tri) (h : openEdges ts = []) :
    (ts.map (areaVec p)).sum = 0 := by
  obtain ⟨nd, cl⟩ := closed_of_open_edges ts h
  rw [sum_areaVec]
  set d := directed ts
  -- reversing is a bijection of d's edges, so the sum equals its reverse's, which is its negative
  have perm : (d.map rev).Perm d := by
    apply (List.perm_ext_iff_of_nodup (nd.map (fun x y hxy => by
      simp only [rev, Prod.mk.injEq] at hxy; exact Prod.ext hxy.2 hxy.1)) nd).2
    intro e
    constructor
    · intro he
      obtain ⟨x, hx, rfl⟩ := List.mem_map.1 he
      exact cl x hx
    · intro he
      exact List.mem_map.2 ⟨rev e, cl e he, by simp [rev]⟩
  have hs : (d.map fun e => p e.1 ⨯₃ p e.2).sum = ((d.map rev).map fun e => p e.1 ⨯₃ p e.2).sum :=
    (perm.map _).sum_eq.symm
  rw [List.map_map] at hs
  have hneg : ((d.map ((fun e => p e.1 ⨯₃ p e.2) ∘ rev))).sum = -(d.map fun e => p e.1 ⨯₃ p e.2).sum := by
    have neg_sum : ∀ l : List (Nat × Nat), (l.map fun e => -(p e.1 ⨯₃ p e.2)).sum =
        -(l.map fun e => p e.1 ⨯₃ p e.2).sum := by
      intro l; induction l with
      | nil => simp
      | cons x l ih => simp only [List.map_cons, List.sum_cons, ih, neg_add]
    rw [← neg_sum]
    congr 1
    apply List.map_congr_left
    intro e _
    simp only [Function.comp, rev]
    rw [cross_anticomm]
  rw [hneg] at hs
  have : (2 : ℝ) • (d.map fun e => p e.1 ⨯₃ p e.2).sum = 0 := by rw [two_smul]; nth_rw 1 [hs]; simp
  exact (smul_eq_zero.1 this).resolve_left (by norm_num)

/-- mesh.rs:24–33: Σ a·(b×c) / 6. -/
def volume (p : Nat → V3) (ts : List Tri) : ℝ :=
  (ts.map fun t => p t.1 ⬝ᵥ (p t.2.1 ⨯₃ p t.2.2)).sum / 6

theorem triple_translate (a b c v : V3) :
    (a + v) ⬝ᵥ ((b + v) ⨯₃ (c + v)) = a ⬝ᵥ (b ⨯₃ c) + v ⬝ᵥ (a ⨯₃ b + b ⨯₃ c + c ⨯₃ a) := by
  simp [dotProduct, cross_apply, Fin.sum_univ_three]; ring

theorem dot_sum (p : Nat → V3) (v : V3) (l : List Tri) :
    (l.map fun t => v ⬝ᵥ areaVec p t).sum = v ⬝ᵥ (l.map (areaVec p)).sum := by
  induction l with
  | nil => simp
  | cons t l ih => simp [ih, dotProduct_add]

/-- On a closed, consistently oriented mesh, `volume()` does not depend on the origin. -/
theorem volume_translate (p : Nat → V3) (ts : List Tri) (h : openEdges ts = []) (v : V3) :
    volume (fun i => p i + v) ts = volume p ts := by
  unfold volume
  congr 1
  have : (ts.map fun t => (p t.1 + v) ⬝ᵥ ((p t.2.1 + v) ⨯₃ (p t.2.2 + v))) =
      (ts.map fun t => p t.1 ⬝ᵥ (p t.2.1 ⨯₃ p t.2.2) + v ⬝ᵥ areaVec p t) := by
    apply List.map_congr_left; intro t _; rw [triple_translate]; rfl
  rw [this, List.sum_map_add, dot_sum, area_vectors_cancel p ts h, dotProduct_zero, add_zero]

/-! ## A pinch passes -/

/-- Round vertex v: each triangle with a corner at v gives the step from the corner before it
to the corner after. A fan follows the steps; this counts the cycles (tests' `vertex_fans`). -/
def corners (ts : List Tri) (v : Nat) : List (Nat × Nat) :=
  ts.filterMap fun ⟨a, b, c⟩ =>
    if a = v then some (b, c) else if b = v then some (c, a) else if c = v then some (a, b) else none

/-- The fans at v by flood: the corners reached from one by following steps, until none is left. -/
def fans (ts : List Tri) (v : Nat) : Nat :=
  go (corners ts v) (corners ts v).length
where
  go : List (Nat × Nat) → Nat → Nat
    | [], _ => 0
    | _, 0 => 0
    | s :: rest, fuel + 1 =>
      -- the fan of s: every corner whose start is reached from s's end, repeatedly
      let fan := grow rest [s] (rest.length + 1)
      1 + go (rest.filter fun x => x ∉ fan) fuel
  grow : List (Nat × Nat) → List (Nat × Nat) → Nat → List (Nat × Nat)
    | _, acc, 0 => acc
    | pool, acc, fuel + 1 =>
      let next := pool.filter fun x => x ∉ acc ∧ acc.any fun y => y.2 = x.1 ∨ x.2 = y.1
      if next = [] then acc else grow pool (acc ++ next) fuel

/-- Two tetrahedra, both outward, sharing vertex 0 and nothing else. -/
def pinch : List Tri :=
  [(0, 2, 1), (0, 1, 3), (0, 3, 2), (1, 2, 3),
   (0, 5, 4), (0, 4, 6), (0, 6, 5), (4, 5, 6)]

theorem pinch_passes_open_edges : openEdges pinch = [] ∧ fans pinch 0 = 2 := by decide

end MeshCheck
end
