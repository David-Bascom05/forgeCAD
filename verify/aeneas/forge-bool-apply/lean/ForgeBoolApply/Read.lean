/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeBoolApply.ApplySense
import ForgeCurved.BodyModel
import ForgeBoolProof.Orient

/-!
# D8.read: a translated `Body` read as faces

The statements (verify/forge-bool-proof) are about `BodyModel`s: faces as carriers, regions and
senses. A `Reading` says what a translated `body.Body` means there:

- `carrier`: each surface's carrier function (continuous, its zeros with empty interior):
  forge-geom's exact surfaces, a parameter here;
- `region`: each face's region, from its loops (the point-in-face reading; D8.writes's geometric
  content), cut to its carrier's zeros.

`readBody R r` has one face per live face of `r`, with that face's surface as carrier and its
`same_sense` as sense. `read_written` carries `apply_writes` across: each read face of the result
past A's is written from a kept spec, its carrier the source surface's and its sense the source's
`!= flip`.
-/

namespace ForgeBoolApply

open Aeneas Aeneas.Std Result forge_topo forge_bool ForgeCurved.BodyModel

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- What a translated body means as faces (the geometry is a parameter). -/
structure Reading (E : Type) [NormedAddCommGroup E] where
  carrier : forge_geom.surface.Surface → E → ℝ
  cont : ∀ s, Continuous (carrier s)
  nd : ∀ s, interior {x | carrier s x = 0} = ∅
  region : body.Body → Nat → Set E

/-- Face `i` of `r`, read. -/
def readFace (R : Reading E) (r : body.Body) (i : Nat) (f : body.Face) : FaceModel E where
  carrier := R.carrier f.surface
  region := R.region r i ∩ {x | R.carrier f.surface x = 0}
  sense := f.same_sense
  loops := []
  cont := R.cont f.surface
  nd := R.nd f.surface
  on_carrier := fun _ hy => hy.2

/-- The live faces of `r`, read (edges and vertices are not read here). -/
def readBody (R : Reading E) (r : body.Body) : BodyModel E where
  faces := ((r.faces.val.zipIdx).filter (fun p => p.1.alive)).map fun p => readFace R r p.2 p.1
  edges := []
  verts := []

/-- A face of the reading is a live face of `r`, read. -/
theorem mem_readBody {R : Reading E} {r : body.Body} {F : FaceModel E}
    (h : F ∈ (readBody R r).faces) :
    ∃ i f, r.faces.val[i]? = some f ∧ f.alive = true ∧ F = readFace R r i f := by
  simp only [readBody, List.mem_map, List.mem_filter] at h
  obtain ⟨⟨f, i⟩, ⟨hm, ha⟩, rfl⟩ := h
  refine ⟨i, f, ?_, ha, rfl⟩
  rw [List.mem_zipIdx_iff_getElem?] at hm
  simpa using hm

/-- Each entry of what the main loop writes comes from a kept spec. -/
theorem Writes.mem {S : Type _} {inst : imprint.Src S} {b : S} {specs : alloc.vec.Vec boolean.Spec}
    {as_is : alloc.vec.Vec Bool} {a0 : body.Body} {ks : List (Usize × Bool)}
    {L : List (Bool × forge_geom.surface.Surface)} (hw : Writes inst b specs as_is a0 ks L) :
    ∀ p ∈ L, ∃ si flip s σ srf, (si, flip) ∈ ks ∧ specs.val[si.val]? = some s ∧
      SrcSense inst b a0 s σ ∧ SrcSurf inst b a0 s srf ∧ p = ((σ != flip), srf) := by
  induction hw with
  | nil => intro p hp; cases hp
  | asIs _ _ ih =>
    intro p hp
    obtain ⟨si, flip, s, σ, srf, hk, h⟩ := ih p hp
    exact ⟨si, flip, s, σ, srf, List.mem_append_left _ hk, h⟩
  | reuse _ _ _ _ _ ih =>
    intro p hp
    obtain ⟨si, flip, s, σ, srf, hk, h⟩ := ih p hp
    exact ⟨si, flip, s, σ, srf, List.mem_append_left _ hk, h⟩
  | @write ks L si flip s σ srf _ _ hs _ hσ hsrf ih =>
    intro p hp
    rcases List.mem_append.mp hp with hp | hp
    · obtain ⟨si', flip', s', σ', srf', hk, h⟩ := ih p hp
      exact ⟨si', flip', s', σ', srf', List.mem_append_left _ hk, h⟩
    · simp only [List.mem_singleton] at hp
      exact ⟨si, flip, s, σ, srf, List.mem_append_right _ (List.mem_singleton_self _), hs, hσ,
        hsrf, hp⟩

/-- An entry of `r`'s faces past `n` is in `tailProj n r`. -/
theorem proj_mem_tail {r : body.Body} {n i : Nat} {f : body.Face} (hi : n ≤ i)
    (hf : r.faces.val[i]? = some f) : proj f ∈ tailProj n r := by
  unfold tailProj
  refine List.mem_map.mpr ⟨f, ?_, rfl⟩
  have : (r.faces.val.drop n)[i - n]? = some f := by
    rw [List.getElem?_drop, Nat.add_sub_cancel' hi]; exact hf
  exact List.mem_of_getElem? this

/-- **D8.read, combinatorial half.** On `apply`'s `ok` result, every face of the reading is a face
of A (same index, sense and surface as A had) or is written from a kept spec: its carrier is the
source surface's and its sense the source's `!= flip`. -/
theorem read_written {S : Type _} (R : Reading E) (inst : imprint.Src S) (a : body.Body) (b : S)
    (plan : boolean.Plan)
    (hfrom : ∀ s ∈ plan.specs.val, s.side = 0#usize → (s.from : Std.U32).val < a.faces.val.length) :
    okspec (boolean.apply inst a b plan) fun r => ∀ F ∈ (readBody R r).faces,
      (∃ (i : Nat) (g : body.Face), a.faces.val[i]? = some g ∧ F.carrier = R.carrier g.surface ∧
        F.sense = g.same_sense) ∨
      (∃ si flip s σ srf, (si, flip) ∈ plan.kept.val ∧ plan.specs.val[si.val]? = some s ∧
        SrcSense inst b a s σ ∧ SrcSurf inst b a s srf ∧
        F.carrier = R.carrier srf ∧ F.sense = (σ != flip)) := by
  refine okspec_mono (apply_writes inst a b plan hfrom) fun r ⟨hk, as_is, hw⟩ F hF => ?_
  obtain ⟨i, f, hf, -, rfl⟩ := mem_readBody hF
  rcases Nat.lt_or_ge i a.faces.val.length with hlt | hge
  · obtain ⟨f', hf', hs, hu⟩ := hk i _ (List.getElem?_eq_getElem hlt)
    rw [hf] at hf'; cases hf'
    exact Or.inl ⟨i, _, List.getElem?_eq_getElem hlt, by simp [readFace, hu], by simp [readFace, hs]⟩
  · obtain ⟨si, flip, s, σ, srf, hkm, hs, hσ, hsrf, hp⟩ := hw.mem _ (proj_mem_tail hge hf)
    simp only [proj, Prod.mk.injEq] at hp
    exact Or.inr ⟨si, flip, s, σ, srf, hkm, hs, hσ, hsrf,
      by simp [readFace, hp.2], by simp [readFace, hp.1]⟩

/-- **`WritesSense` from equal carriers and senses.** If each read face's points on a facet lie on
the facet of a kept region whose carrier is the face's and whose written sense is the face's, the
faces are written with senses (`WritesSense`): near such a point both sides agree outright. What
is left of D8.sense is that premise: its carrier and sense half from `read_written` and the plan's
correspondence to the arrangement (each kept spec's source a kept region's carrier and written
sense), its point half the regions' geometry (D8.writes). -/
theorem writesSense_of_eq (R : Reading E) (r : body.Body) (a : ForgeBoolProof.Arr E)
    (op : ForgeBoolProof.Op) (k : Nat → Bool)
    (h : ∀ F ∈ (readBody R r).faces, ∀ y ∈ F.region, y ∈ ForgeCurved.FaceLocal.FacetPts a.G →
      ∃ f ∈ a.regions, k f = true ∧ y ∈ a.G.facet (a.gf f) ∧ F.carrier = a.cf (a.gf f) ∧
        F.sense = ForgeBoolProof.writtenSense op (a.src f) (a.sense f)) :
    ForgeBoolProof.WritesSense a op k (readBody R r) := by
  intro F hF y hy hyf
  obtain ⟨f, hf, hk, hfac, hc, hs⟩ := h F hF y hy hyf
  refine ⟨f, hf, hk, hfac, Filter.Eventually.of_forall fun z => ?_⟩
  rw [hc, hs]
  exact ⟨Iff.rfl, Iff.rfl⟩

end ForgeBoolApply
