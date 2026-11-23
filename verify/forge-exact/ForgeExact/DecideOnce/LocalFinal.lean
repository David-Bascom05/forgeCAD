import ForgeExact.DecideOnce.SampleLocal
import ForgeExact.DecideOnce.LocalJordan

/-!
# §3a closed: the local ledger is exact in the region, from the input alone

`LocalInput` is the domain:
* closed flat B-reps;
* each face is near (its plane a carrier) or far (a nonzero normal, and every point of the open
  region lies outside the face's vertex box: the bridge's "box disjoint from R̄").

On it, `local_compat` (the localized Jordan lemma) discharges `LocalCompat`. So `local_exact_final`
and `local_correct_final` hold with no hypothesis beyond the input's domain and valid wall indices.
-/

namespace ForgeExact.DecideOnce.Sample

open ForgeExact ForgeExact.Ref Ledger Topology

/-- **The local ledger's domain**: closed flat B-reps whose faces are near (plane a carrier) or far
(outside the region by their box). -/
structure LocalInput (ps : List Plane) (ss : List Solid) (ws : List Nat) : Prop where
  flat : ∀ s ∈ ss, Jordan.FacesFlat s
  closed : ∀ s ∈ ss, Jordan.Closed s
  faces : ∀ s ∈ ss, ∀ f ∈ Jordan.facesL s, Scaled ps f.plane ∨
    (f.plane.n ≠ ⟨0, 0, 0⟩ ∧ ∀ x, wallU ps ws x → Jordan.OutBox f x)

theorem localCompat_of {ps : List Plane} {ss : List Solid} {ws : List Nat} (hws : ∀ k ∈ ws, k < ps.length)
    (h : LocalInput ps ss ws) : LocalCompat ps (brepA ss) (wallIn ws) := by
  have hreg := wall_region ps ws hws
  have hcase : ∀ i, (Jordan.FacesFlat (solidAt ss i) ∧ Jordan.Closed (solidAt ss i)) ∧
      ∀ f ∈ Jordan.facesL (solidAt ss i), Scaled ps f.plane ∨
        (f.plane.n ≠ ⟨0, 0, 0⟩ ∧ ∀ T x, wallIn ws T → covCell ps T x → Jordan.OutBox f x) := by
    intro i
    rcases solidAt_cases ss i with hs | hs
    · refine ⟨⟨h.flat _ hs, h.closed _ hs⟩, fun f hf => ?_⟩
      rcases h.faces _ hs f hf with hn | ⟨hn, hfar⟩
      · exact Or.inl hn
      · exact Or.inr ⟨hn, fun T x hT hx => hfar x ((hreg T x hx).mpr hT)⟩
    · rw [hs]
      refine ⟨⟨fun f hf => by simp [facesL_default] at hf, fun e => ?_⟩, fun f hf => by simp [facesL_default] at hf⟩
      simp [Jordan.Closed, Jordan.allEdges, facesL_default]
  exact local_compat ps (wallIn ws) (solidAt ss) (fun i => (hcase i).1.1) (fun i => (hcase i).1.2)
    (fun i => (hcase i).2)

/-- **(c′), closed.** For carriers with nonzero normals, valid wall indices, and a `LocalInput`, the
local ledger the sampler builds is exact in the region. An answered body is valid, its faces are
exactly the region's exact boundary facets, and its kept cells regularise to the regularised boolean
at every point of `R°`. `ZeroThickness` holds iff the exact result is non-manifold at a region cell. -/
def local_correct_final {ps : List Plane} {ss : List Solid} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (ws : List Nat)
    (hws : ∀ k ∈ ws, k < ps.length) (h : LocalInput ps ss ws) (rule : Rule) :=
  local_correct_sampler (ss := ss) hnz ws hws (localCompat_of hws h) rule

end ForgeExact.DecideOnce.Sample
