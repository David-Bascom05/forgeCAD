/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeBoolApply.Funs
import ForgeBoolApply.BodySpec

/-!
# D8.sense against the translation: the senses `apply` writes

The translated boolean.rs `apply`, on its `ok` result: every face of A keeps its sense and
surface, and every face it adds is written from a kept region (`Written`): its sense is the
source face's `!= flip`, its surface the source face's.
-/

namespace ForgeBoolApply

open Aeneas Aeneas.Std Result forge_topo forge_bool

theorem faces_of_eq {α : Type _} {b b' : body.Body} {x : α}
    {m : Result (α × body.Body)} (hm : okspec m fun r => r.2.faces = b.faces) (h : m = ok (x, b')) :
    FaceSame b b' :=
  FaceSame.of_faces (by rw [okspec_of_eq hm h])

/-- `Made::vertex` adds at most a vertex: the faces are the same. -/
theorem vertex_same {S : Type _} (inst : imprint.Src S) (made : boolean.Made) (a : body.Body)
    (b : S) (plan : boolean.Plan) (v : boolean.VKey) :
    okspec (boolean.Made.vertex inst made a b plan v) fun r => FaceSame a r.2.2 := by
  unfold boolean.Made.vertex
  go_steps
  all_goals first
    | exact (okspec_ok _ _).mpr (FaceSame.refl _)
    | exact (okspec_ok _ _).mpr (faces_of_eq (add_tolerant_vertex_faces _ _ _ _) ‹_›)


/-- A translated loop over `(iterator, body)` whose every step keeps the faces. -/
theorem loop_same {ι : Type _}
    (step : ι × body.Body → Result (ControlFlow (ι × body.Body) body.Body))
    (hstep : ∀ it b, okspec (step (it, b)) fun r => match r with
      | .cont (_, b') => FaceSame b b'
      | .done b' => FaceSame b b')
    (it : ι) (a : body.Body) : okspec (loop step (it, a)) fun r => FaceSame a r := by
  refine okspec_loop step (fun st => FaceSame a st.2) _ ?_ (it, a) (FaceSame.refl a)
  rintro ⟨it, b⟩ hb
  refine okspec_mono (hstep it b) fun r hr => ?_
  rcases r with ⟨it', b'⟩ | b'
  · exact hb.trans hr
  · exact hb.trans hr

/-- Close a step's final `ok` with the faces kept. -/
macro "same_close" : tactic => `(tactic| (
  first
    | exact (okspec_ok _ _).mpr (FaceSame.refl _)
    | exact (okspec_ok _ _).mpr (FaceSame.of_faces rfl)
    | (rw [okspec_ok]; dsimp only; exact okspec_of_eq (kill_face_keep _ _) ‹_›)))

/-- The flush loop writes vertex names only. -/
theorem loop0_same (it : core.slice.iter.Iter (boolean.VKey × name.Name)) (a : body.Body) :
    okspec (boolean.apply_loop0 it a) fun r => FaceSame a r := by
  unfold boolean.apply_loop0
  refine loop_same _ (fun it b => ?_) it a
  unfold boolean.apply_loop0.body
  go_steps
  all_goals same_close

/-- The kill loop kills faces. -/
theorem loop2_same (it : core.slice.iter.Iter body.FaceId) (a : body.Body)
    (staying : alloc.vec.Vec body.FaceId) :
    okspec (boolean.apply_loop2 it a staying) fun r => FaceSame a r := by
  unfold boolean.apply_loop2
  refine loop_same _ (fun it b => ?_) it a
  unfold boolean.apply_loop2.body
  go_steps
  all_goals same_close

/-- The grown-tolerance loop writes vertex tolerances only. -/
theorem loop4_same (it : core.slice.iter.Iter (boolean.VKey × F64)) (a : body.Body) :
    okspec (boolean.apply_loop4 it a) fun r => FaceSame a r := by
  unfold boolean.apply_loop4
  refine loop_same _ (fun it b => ?_) it a
  unfold boolean.apply_loop4.body
  go_steps
  all_goals same_close


theorem removeLoops_same (a : body.Body) (f : body.FaceId) (ls : List body.LoopId) :
    okspec (removeLoops a f ls) fun r => FaceSame a r := by
  induction ls generalizing a with
  | nil => exact (okspec_ok _ _).mpr (FaceSame.refl a)
  | cons l ls ih =>
    unfold removeLoops
    exact okspec_bind (remove_loop_keep a f l) fun a' h =>
      okspec_mono (ih a') fun r hr => h.trans hr

theorem dropReplacedOne_same (specs : Slice boolean.Spec) (a : body.Body) (out : List Usize)
    (si : Usize) (flip : Bool) :
    okspec (dropReplacedOne specs a out si flip) fun r => FaceSame a r.1 := by
  unfold dropReplacedOne
  refine okspec_let fun s _ => ?_
  split
  · refine okspec_let fun _ _ => ?_
    refine okspec_let fun _ _ => ?_
    refine okspec_let fun _ _ => ?_
    refine okspec_bind (drop_query_loops_keep a _ _) fun a1 h1 => ?_
    refine okspec_bind (removeLoops_same a1 _ _) fun a2 h2 => ?_
    exact (okspec_ok _ _).mpr (h1.trans h2)
  · exact (okspec_ok _ _).mpr (FaceSame.refl a)

theorem dropReplacedWalk_same (specs : Slice boolean.Spec) (a : body.Body) (out : List Usize)
    (kept : List (Usize × Bool)) :
    okspec (dropReplacedWalk specs a out kept) fun r => FaceSame a r.1 := by
  induction kept generalizing a out with
  | nil => exact (okspec_ok _ _).mpr (FaceSame.refl a)
  | cons p rest ih =>
    obtain ⟨si, flip⟩ := p
    unfold dropReplacedWalk
    refine okspec_bind (dropReplacedOne_same specs a out si flip) fun ⟨a', out'⟩ h => ?_
    exact okspec_mono (ih a' out') fun r hr => h.trans hr

/-- The `drop_replaced_loops` model keeps the faces. -/
theorem drop_replaced_loops_same (a : body.Body) (specs : Slice boolean.Spec)
    (kept : Slice (Usize × Bool)) :
    okspec (boolean.drop_replaced_loops a specs kept) fun r => FaceSame a r.2 := by
  unfold boolean.drop_replaced_loops
  refine okspec_bind (dropReplacedWalk_same specs a [] kept.val) fun ⟨a', out⟩ h => ?_
  simp only [uncurry_apply_pair]
  split
  · exact (okspec_ok _ _).mpr h
  · exact okspec_fail _ _


/-! ## Relative forms: from `FaceSame a0 a`, the result is `FaceSame a0 _` -/

section Rel
variable {a0 : body.Body}

theorem vertex_rel {S : Type _} {inst : imprint.Src S} {made : boolean.Made} {a : body.Body}
    (h : FaceSame a0 a) (b : S) (plan : boolean.Plan) (v : boolean.VKey) :
    okspec (boolean.Made.vertex inst made a b plan v) fun r => FaceSame a0 r.2.2 :=
  okspec_mono (vertex_same inst made a b plan v) fun _ hr => h.trans hr

theorem add_edge_rel {a : body.Body} (h : FaceSame a0 a) (c : forge_geom.curve.Curve)
    (r : forge_math.interval.Interval) (v0 v1 : body.VertexId) (n : name.Name) :
    okspec (body.Body.add_edge a c r v0 v1 n) fun r => FaceSame a0 r.2 :=
  okspec_mono (add_edge_faces a c r v0 v1 n) fun _ hr => h.trans (FaceSame.of_faces (by rw [hr]))

theorem add_loop_rel {a : body.Body} (h : FaceSame a0 a) (face : body.FaceId)
    (uses : Slice (body.EdgeId × Bool)) :
    okspec (body.Body.add_loop a face uses) fun r => FaceSame a0 r.2 :=
  okspec_mono (add_loop_keep a face uses) fun _ hr => h.trans hr

theorem extend_query_rel {a : body.Body} (h : FaceSame a0 a) (f : body.FaceId) (lp : body.LoopId) :
    okspec (classify.extend_query a f lp) fun r => FaceSame a0 r :=
  okspec_mono (extend_query_keep a f lp) fun _ hr => h.trans hr

end Rel

/-- One `Body`-method bind with its relative lemma (extended below, one rule per lemma). -/
syntax "same_rel" : tactic
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (vertex_rel ‹_› _ _ _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (add_edge_rel ‹_› _ _ _ _ _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (add_loop_rel ‹_› _ _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (extend_query_rel ‹_› _ _) fun _ _ => ?_)

/-- Step through a loop body, binding each `Body` method with its relative lemma (the input's
`FaceSame` found by assumption). -/
macro "same_steps" : tactic => `(tactic| (
  repeat' (first
    | with_reducible exact okspec_fail _ _
    | same_rel
    | simp only [bind_tc_eq, Std.bind_assoc, Std.pure_bind, uncurry_apply_pair, uncurry_eta]
    | (split <;> try dsimp only [])
    | with_reducible_and_instances refine okspec_let2 fun _ _ _ => ?_
    | with_reducible_and_instances refine okspec_let fun _ _ => ?_
    | ((fail_if_success with_reducible refine (okspec_ok _ _).mpr ?_);
       (first | refine okspec_let2 fun _ _ _ => ?_ | refine okspec_let fun _ _ => ?_))
    | dsimp only [])))

/-- Close a final `ok` whose body is (a record update of) one with a `FaceSame` in hand. -/
macro "same_finish" : tactic => `(tactic| (
  rw [okspec_ok]; dsimp only; assumption))

/-- The piece loop adds vertices and edges: the faces are the same. -/
theorem loop3_loop0_loop0_rel {S : Type _} (inst : imprint.Src S)
    (it : core.slice.iter.Iter (Usize × Bool)) {a0 a : body.Body} (h : FaceSame a0 a) (b : S)
    (v : alloc.vec.Vec boolean.Piece) (v1 v2 : alloc.vec.Vec name.Name)
    (v3 : alloc.vec.Vec boolean.Spec) (v4 : alloc.vec.Vec (Usize × Bool))
    (v5 : alloc.vec.Vec forge_math.vec.Point3) (v6 : alloc.vec.Vec name.Name)
    (v7 : alloc.vec.Vec (boolean.VKey × name.Name)) (v8 : alloc.vec.Vec F64)
    (v9 : alloc.vec.Vec (boolean.VKey × F64)) (v10 : alloc.vec.Vec (boolean.VKey × boolean.VKey))
    (v11 : alloc.vec.Vec (name.Name × name.Name)) (v12 v13 : alloc.vec.Vec body.FaceId)
    (pieces : alloc.vec.Vec boolean.Piece) (made : boolean.Made)
    (emap : alloc.vec.Vec (Option body.EdgeId)) (uses : alloc.vec.Vec (body.EdgeId × Bool)) :
    okspec (boolean.apply_loop3_loop0_loop0 inst it a b v v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12
      v13 pieces made emap uses) fun r => FaceSame a0 r.1 := by
  unfold boolean.apply_loop3_loop0_loop0
  refine okspec_loop _ (fun st => FaceSame a0 st.2.1) _ ?_ _ h
  rintro ⟨it, a, made, emap, uses⟩ (hs : FaceSame a0 a)
  unfold boolean.apply_loop3_loop0_loop0.body
  same_steps
  all_goals same_finish


macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (loop3_loop0_loop0_rel _ _ ‹_› _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)
    fun _ _ => ?_)

/-- The loop over a spec's loops adds edges and loops: the faces are the same. -/
theorem loop3_loop0_rel {S : Type _} (inst : imprint.Src S)
    (it : core.slice.iter.Iter (alloc.vec.Vec (Usize × Bool))) {a0 a : body.Body}
    (h : FaceSame a0 a) (b : S)
    (v : alloc.vec.Vec boolean.Piece) (v1 v2 : alloc.vec.Vec name.Name)
    (v3 : alloc.vec.Vec boolean.Spec) (v4 : alloc.vec.Vec (Usize × Bool))
    (v5 : alloc.vec.Vec forge_math.vec.Point3) (v6 : alloc.vec.Vec name.Name)
    (v7 : alloc.vec.Vec (boolean.VKey × name.Name)) (v8 : alloc.vec.Vec F64)
    (v9 : alloc.vec.Vec (boolean.VKey × F64)) (v10 : alloc.vec.Vec (boolean.VKey × boolean.VKey))
    (v11 : alloc.vec.Vec (name.Name × name.Name)) (v12 v13 : alloc.vec.Vec body.FaceId)
    (pieces : alloc.vec.Vec boolean.Piece) (made : boolean.Made)
    (emap : alloc.vec.Vec (Option body.EdgeId)) (flip b1 : Bool) (face : body.FaceId) :
    okspec (boolean.apply_loop3_loop0 inst it a b v v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13
      pieces made emap flip b1 face) fun r => FaceSame a0 r.1 := by
  unfold boolean.apply_loop3_loop0
  refine okspec_loop _ (fun st => FaceSame a0 st.2.1) _ ?_ _ h
  rintro ⟨it, a, made, emap⟩ (hs : FaceSame a0 a)
  unfold boolean.apply_loop3_loop0.body
  same_steps
  all_goals same_finish

end ForgeBoolApply
