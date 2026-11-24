/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeBoolApply.OkSpec
import ForgeTopoBody
import ForgeBoolApply.FunsExternal

/-!
# What `Body`'s translated methods write (frame lemmas, `okspec`)

For D8's rows over `apply`: each `Body` method `apply` calls, on its `ok` result, changes the
fields it is meant to and keeps the others.
-/

namespace ForgeBoolApply

open Aeneas Aeneas.Std Result forge_topo

theorem push_val {α : Type _} (v : alloc.vec.Vec α) (x : α) :
    okspec (alloc.vec.Vec.push v x) fun w => w.val = v.val ++ [x] := by
  unfold alloc.vec.Vec.push
  dsimp only
  split
  · rw [okspec_ok]; simp [List.concat_eq_append]
  · exact okspec_fail _ _

/-- **`add_face`** appends one face, alive, with no loops yet, and the given surface, sense,
shell and name; its id is the old face count; vertices, edges, coedges and loops are kept. -/
theorem add_face_spec (self : body.Body) (shell : body.ShellId)
    (surface : forge_geom.surface.Surface) (same_sense : Bool) (n : name.Name) :
    okspec (body.Body.add_face self shell surface same_sense n) fun (id, b) =>
      id = UScalar.cast .U32 self.faces.len ∧
      (∃ f, b.faces.val = self.faces.val ++ [f] ∧ f.surface = surface ∧
        f.same_sense = same_sense ∧ f.alive = true ∧ f.name = n ∧ f.shell = shell) ∧
      b.vertices = self.vertices ∧ b.edges = self.edges ∧ b.coedges = self.coedges ∧
      b.loops = self.loops := by
  unfold body.Body.add_face
  refine okspec_let fun i1 hi1 => ?_
  simp only [lift, ok.injEq] at hi1
  refine okspec_let fun s _ => ?_
  refine okspec_let fun c _ => ?_
  refine okspec_let fun c1 _ => ?_
  refine okspec_bind (push_val _ _) fun v hv => ?_
  refine okspec_let fun i2 _ => ?_
  refine okspec_let fun ⟨s1, back⟩ _ => ?_
  refine okspec_let fun v1 _ => ?_
  refine okspec_let fun ⟨o, gback⟩ _ => ?_
  have post : ∀ hs hu, (fun x : body.FaceId × body.Body => match x with
      | (id, b) => id = UScalar.cast .U32 self.faces.len ∧
        (∃ f, b.faces.val = self.faces.val ++ [f] ∧ f.surface = surface ∧
          f.same_sense = same_sense ∧ f.alive = true ∧ f.name = n ∧ f.shell = shell) ∧
        b.vertices = self.vertices ∧ b.edges = self.edges ∧ b.coedges = self.coedges ∧
        b.loops = self.loops) (i1, { self with faces := v, shells := hs, hulls := hu }) :=
    fun _ _ => ⟨hi1.symm, ⟨_, hv, rfl, rfl, rfl, rfl, rfl⟩, rfl, rfl, rfl, rfl⟩
  cases o with
  | none => exact (okspec_ok _ _).mpr (post _ _)
  | some h =>
    dsimp only
    repeat (first | exact (okspec_ok _ _).mpr (post _ _) | refine okspec_let fun ⟨_, _⟩ _ => ?_ |
      refine okspec_let fun _ _ => ?_)


/-! ## Vectors -/

theorem vec_index {α : Type _} (v : alloc.vec.Vec α) (i : Usize) :
    okspec (alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i) fun x =>
      v.val[i.val]? = some x := by
  simp only [alloc.vec.Vec.index, core.slice.index.Usize.index, Slice.index_usize]
  split
  · exact okspec_fail _ _
  · rename_i x hx; rw [okspec_ok]; simpa [alloc.vec.Vec.val] using hx

theorem vec_index_mut {α : Type _} (v : alloc.vec.Vec α) (i : Usize) :
    okspec (alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice α) v i) fun p =>
      v.val[i.val]? = some p.1 ∧ ∀ y, (p.2 y).val = v.val.set i.val y := by
  simp only [alloc.vec.Vec.index_mut, core.slice.index.Usize.index_mut, Slice.index_mut_usize,
    Slice.index_usize]
  split
  · simp only [bind_tc_fail]; exact okspec_fail _ _
  · rename_i x hx
    simp only [bind_tc_ok, pure]
    rw [okspec_ok]
    refine ⟨by simpa [alloc.vec.Vec.val] using hx, fun y => ?_⟩
    simp [Slice.set, Slice.setAtNat, alloc.vec.Vec.val, Slice.from_val]


/-! ## `FaceKeep`: each face is still there, with its sense and surface -/

/-- Each face of `b` is in `b'` at its index, with the same sense and surface. -/
def FaceKeep (b b' : body.Body) : Prop :=
  ∀ (i : Nat) (f : body.Face), b.faces.val[i]? = some f →
    ∃ f' : body.Face, b'.faces.val[i]? = some f' ∧ f'.same_sense = f.same_sense ∧ f'.surface = f.surface

theorem FaceKeep.refl (b : body.Body) : FaceKeep b b := by
  intro _ f h; exact ⟨f, h, rfl, rfl⟩

theorem FaceKeep.trans {b b' b'' : body.Body} (h1 : FaceKeep b b') (h2 : FaceKeep b' b'') :
    FaceKeep b b'' := by
  intro i f hf
  obtain ⟨f', hf', hs, hu⟩ := h1 i f hf
  obtain ⟨f'', hf'', hs', hu'⟩ := h2 i f' hf'
  exact ⟨f'', hf'', hs'.trans hs, hu'.trans hu⟩

theorem FaceKeep.of_faces {b b' : body.Body} (h : b'.faces.val = b.faces.val) : FaceKeep b b' := by
  intro i f hf; exact ⟨f, h ▸ hf, rfl, rfl⟩

/-- Faces `l` with entry `j` replaced by one of the same sense and surface. -/
theorem FaceKeep.of_set {b b' : body.Body} {j : Nat} {g f : body.Face}
    (hj : b.faces.val[j]? = some f) (hg : g.same_sense = f.same_sense ∧ g.surface = f.surface)
    (h : b'.faces.val = b.faces.val.set j g) : FaceKeep b b' := by
  intro i f' hf'
  rw [h]
  by_cases hij : i = j
  · subst hij
    rw [hj] at hf'; cases hf'
    have hlt : i < b.faces.val.length := by
      rcases Nat.lt_or_ge i b.faces.val.length with hlt | hge
      · exact hlt
      · rw [List.getElem?_eq_none hge] at hj; cases hj
    exact ⟨g, by simp [List.getElem?_set, hlt], hg.1, hg.2⟩
  · exact ⟨f', by rw [List.getElem?_set_ne (Ne.symm hij)]; exact hf', rfl, rfl⟩

/-- Faces appended to: every old face is kept. -/
theorem FaceKeep.of_append {b b' : body.Body} {l : List body.Face}
    (h : b'.faces.val = b.faces.val ++ l) : FaceKeep b b' := by
  intro i f hf
  refine ⟨f, ?_, rfl, rfl⟩
  rw [h, List.getElem?_append_left]
  · exact hf
  · rcases Nat.lt_or_ge i b.faces.val.length with hlt | hge
    · exact hlt
    · rw [List.getElem?_eq_none hge] at hf; cases hf


/-! ## The methods that leave the faces as they were -/

/-- Close a goal `okspec m p` whose every `ok` value is a record with the faces of `self`. -/
macro "faces_steps" : tactic => `(tactic| (
  repeat (first
    | with_reducible exact (okspec_ok _ _).mpr rfl
    | with_reducible exact okspec_fail _ _
    | with_reducible refine okspec_let2 fun _ _ _ => ?_
    | with_reducible refine okspec_let fun _ _ => ?_
    | simp only [uncurry_apply_pair]
    | (split <;> try dsimp only [])
    | dsimp only [])))

theorem add_tolerant_vertex_faces (self : body.Body) (p : forge_math.vec.Point3) (t : F64)
    (n : name.Name) :
    okspec (body.Body.add_tolerant_vertex self p t n) fun r => r.2.faces = self.faces := by
  unfold body.Body.add_tolerant_vertex; faces_steps

theorem add_edge_faces (self : body.Body) (c : forge_geom.curve.Curve)
    (r : forge_math.interval.Interval) (v0 v1 : body.VertexId) (n : name.Name) :
    okspec (body.Body.add_edge self c r v0 v1 n) fun r => r.2.faces = self.faces := by
  unfold body.Body.add_edge; faces_steps

theorem add_shell_faces (self : body.Body) :
    okspec (body.Body.add_shell self) fun r => r.2.faces = self.faces := by
  unfold body.Body.add_shell; faces_steps

theorem drop_use_faces (self : body.Body) (c : body.CoedgeId) :
    okspec (body.Body.drop_use self c) fun r => r.faces = self.faces := by
  unfold body.Body.drop_use; faces_steps


theorem kill_inner_faces (it : body.Coedges) (self : body.Body) :
    okspec (body.Body.kill_face_loop0_loop0 it self) fun r => r.faces = self.faces := by
  unfold body.Body.kill_face_loop0_loop0
  refine okspec_loop _ (fun st => st.2.faces = self.faces) _ ?_ _ rfl
  rintro ⟨it, s⟩ hs
  unfold body.Body.kill_face_loop0_loop0.body
  refine okspec_let2 fun o it' _ => ?_
  cases o with
  | none => exact (okspec_ok _ _).mpr hs
  | some c =>
    exact okspec_bind (drop_use_faces s c) fun s' h => (okspec_ok _ _).mpr (h.trans hs)

theorem kill_outer_faces (it : core.ops.range.Range Usize) (self : body.Body) (f : body.FaceId) :
    okspec (body.Body.kill_face_loop0 it self f) fun r => r.faces = self.faces := by
  unfold body.Body.kill_face_loop0
  refine okspec_loop _ (fun st => st.2.faces = self.faces) _ ?_ _ rfl
  rintro ⟨it, s⟩ hs
  unfold body.Body.kill_face_loop0.body
  refine okspec_let2 fun o it' _ => ?_
  cases o with
  | none => exact (okspec_ok _ _).mpr hs
  | some li =>
    repeat (first
      | exact okspec_bind (kill_inner_faces _ _) fun s' h => (okspec_ok _ _).mpr (h.trans hs)
      | with_reducible refine okspec_let fun _ _ => ?_
      | simp only [uncurry_apply_pair])

/-- **`kill_face`** keeps every face's sense and surface. -/
theorem kill_face_keep (self : body.Body) (f : body.FaceId) :
    okspec (body.Body.kill_face self f) fun r => FaceKeep self r := by
  unfold body.Body.kill_face
  refine okspec_let fun i _ => ?_
  refine okspec_bind (vec_index _ _) fun f1 hf1 => ?_
  split
  · refine okspec_bind (vec_index_mut _ _) fun ⟨f2, back⟩ ⟨hf2, hback⟩ => ?_
    refine okspec_let fun i1 _ => ?_
    refine okspec_let2 fun o gback _ => ?_
    -- the faces after the write: `back {f2 with alive := false}`, whichever way the hulls go
    refine okspec_bind (m := (match o with
        | none => ok (back { f2 with alive := false }, gback none)
        | some h => _ : Result _)) (p := fun vc => vc.1 = back { f2 with alive := false }) ?_ ?_
    · cases o with
      | none => exact (okspec_ok _ _).mpr rfl
      | some h => dsimp only; faces_steps
    · rintro ⟨v, c⟩ hv
      dsimp only at hv ⊢
      faces_steps
      refine okspec_mono (kill_outer_faces _ _ _) fun r hr => ?_
      refine FaceKeep.of_set (g := { f2 with alive := false }) hf2 ⟨rfl, rfl⟩ ?_
      rw [hr]; dsimp only; rw [hv, hback]
  · exact (okspec_ok _ _).mpr (FaceKeep.refl _)


/-- Step to the final `ok` (each step's equation kept). -/
macro "go_steps" : tactic => `(tactic| (
  repeat (first
    | with_reducible exact okspec_fail _ _
    | with_reducible refine okspec_let2 fun _ _ _ => ?_
    | with_reducible refine okspec_let fun _ _ => ?_
    | simp only [uncurry_apply_pair]
    | (split <;> try dsimp only [])
    | dsimp only [])))

/-- A face entry rewritten through `index_mut` with the same sense and surface. -/
theorem FaceKeep.of_index_mut {self b' : body.Body} {i : Usize} {f : body.Face}
    {back : body.Face → alloc.vec.Vec body.Face}
    (h : alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice body.Face) self.faces i =
      ok (f, back)) (g : body.Face) (hb : b'.faces = back g)
    (hg : g.same_sense = f.same_sense ∧ g.surface = f.surface) : FaceKeep self b' := by
  obtain ⟨hf, hback⟩ := okspec_of_eq (vec_index_mut self.faces i) h
  exact FaceKeep.of_set hf hg (by rw [hb, hback])

/-- **`remove_loop`** keeps every face's sense and surface. -/
theorem remove_loop_keep (self : body.Body) (f : body.FaceId) (lp : body.LoopId) :
    okspec (body.Body.remove_loop self f lp) fun r => FaceKeep self r := by
  unfold body.Body.remove_loop
  refine okspec_let fun i _ => ?_
  refine okspec_let2 fun f1 back hb => ?_
  go_steps
  rw [okspec_ok]
  exact FaceKeep.of_index_mut hb _ rfl ⟨rfl, rfl⟩

/-- **`add_loop`** keeps every face's sense and surface. -/
theorem add_loop_keep (self : body.Body) (face : body.FaceId) (uses : Slice (body.EdgeId × Bool)) :
    okspec (body.Body.add_loop self face uses) fun r => FaceKeep self r.2 := by
  unfold body.Body.add_loop
  repeat (first
    | with_reducible refine okspec_let2 fun f back (hb : alloc.vec.Vec.index_mut _ self.faces _ = _) => ?_
    | with_reducible refine okspec_let2 fun _ _ _ => ?_
    | with_reducible refine okspec_let fun _ _ => ?_
    | simp only [uncurry_apply_pair])
  rw [okspec_ok]
  exact FaceKeep.of_index_mut hb _ rfl ⟨rfl, rfl⟩

/-- **`add_face`** keeps every face's sense and surface (and appends one). -/
theorem add_face_keep (self : body.Body) (shell : body.ShellId)
    (surface : forge_geom.surface.Surface) (same_sense : Bool) (n : name.Name) :
    okspec (body.Body.add_face self shell surface same_sense n) fun r => FaceKeep self r.2 :=
  okspec_mono (add_face_spec self shell surface same_sense n) fun ⟨_, _⟩ ⟨_, ⟨_, h, _⟩, _⟩ =>
    FaceKeep.of_append h

/-- The query models write only `query`. -/
theorem setQuery_keep (b : body.Body) (f : body.FaceId) (q : body.Cached body.FaceQuery) :
    okspec (forge_bool.setQuery b f q) fun r => FaceKeep b r := by
  unfold forge_bool.setQuery
  dsimp only []
  split
  · rename_i h
    rw [okspec_ok]
    exact FaceKeep.of_set (g := { b.faces.val[(f : Std.U32).val] with query := q })
      (List.getElem?_eq_getElem h) ⟨rfl, rfl⟩ (alloc.vec.Vec.from_val _ _)
  · exact okspec_fail _ _

theorem extend_query_keep (b : body.Body) (f : body.FaceId) (lp : body.LoopId) :
    okspec (forge_bool.classify.extend_query b f lp) fun r => FaceKeep b r := by
  unfold forge_bool.classify.extend_query
  exact okspec_let fun q _ => setQuery_keep b f q

theorem drop_query_loops_keep (b : body.Body) (f : body.FaceId) (gone : Slice body.LoopId) :
    okspec (forge_bool.classify.drop_query_loops b f gone) fun r => FaceKeep b r := by
  unfold forge_bool.classify.drop_query_loops
  exact okspec_let fun q _ => setQuery_keep b f q

/-- The `rebuild_shells` model changes only shells and the live faces' shell ids. -/
theorem rebuild_shells_keep (b : body.Body) :
    okspec (body.Body.rebuild_shells b) fun r => FaceKeep b r := by
  unfold body.Body.rebuild_shells
  dsimp only []
  split
  · rw [okspec_ok]
    intro i f hf
    refine ⟨if f.alive then { f with shell := rebuiltShellOf b i } else f, ?_, ?_, ?_⟩
    · simp [List.getElem?_mapIdx, hf]
    · split <;> rfl
    · split <;> rfl
  · exact okspec_fail _ _

end ForgeBoolApply
