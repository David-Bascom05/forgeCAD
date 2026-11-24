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


/-! ## The main loop: `Sensed` -/

/-- Every face of `a0` keeps its sense and surface in `r`, and every face past `a0`'s satisfies
`W` (of its sense and surface). -/
def Sensed (W : Bool → forge_geom.surface.Surface → Prop) (a0 r : body.Body) : Prop :=
  FaceKeep a0 r ∧ ∀ i f, a0.faces.val.length ≤ i → r.faces.val[i]? = some f → W f.same_sense f.surface

theorem FaceKeep.length_le {b b' : body.Body} (h : FaceKeep b b') :
    b.faces.val.length ≤ b'.faces.val.length := by
  rcases Nat.eq_zero_or_pos b.faces.val.length with h0 | hpos
  · omega
  · obtain ⟨f', hf', -⟩ := h (b.faces.val.length - 1) _ (List.getElem?_eq_getElem (by omega))
    have := (List.getElem?_eq_some_iff.mp hf').1
    omega

theorem Sensed.refl (W : Bool → forge_geom.surface.Surface → Prop) (a0 : body.Body) :
    Sensed W a0 a0 :=
  ⟨FaceKeep.refl a0, fun i f hi hf => by
    rw [List.getElem?_eq_none hi] at hf; cases hf⟩

/-- `Sensed` carries over the same faces. -/
theorem Sensed.same {W : Bool → forge_geom.surface.Surface → Prop} {a0 x y : body.Body}
    (hx : Sensed W a0 x) (h : FaceSame x y) : Sensed W a0 y := by
  refine ⟨hx.1.trans h.1, fun i f hi hf => ?_⟩
  obtain ⟨f', hf', hs, hu⟩ := h.2 i f hf
  rw [← hs, ← hu]
  exact hx.2 i f' hi hf'

/-- `Sensed` carries over a face appended that satisfies `W`. -/
theorem Sensed.append {W : Bool → forge_geom.surface.Surface → Prop} {a0 x y : body.Body}
    {g : body.Face} (hx : Sensed W a0 x) (hy : y.faces.val = x.faces.val ++ [g])
    (hg : W g.same_sense g.surface) : Sensed W a0 y := by
  refine ⟨hx.1.trans (FaceKeep.of_append hy), fun i f hi hf => ?_⟩
  rw [hy] at hf
  rcases Nat.lt_or_ge i x.faces.val.length with hlt | hge
  · rw [List.getElem?_append_left hlt] at hf
    exact hx.2 i f hi hf
  · rw [List.getElem?_append_right hge] at hf
    rcases Nat.eq_or_lt_of_le hge with heq | hgt
    · subst heq; simp at hf; subst hf; exact hg
    · rw [List.getElem?_eq_none (by simp; omega)] at hf; cases hf

theorem ok_inj {α : Type _} {x y : α} (h : (ok x : Result α) = ok y) : x = y :=
  (okspec_of_eq ((okspec_ok x (· = x)).mpr rfl) h).symm

theorem iter_next {T : Type _} (it : core.slice.iter.Iter T) :
    okspec (core.slice.iter.IteratorSliceIter.next it) fun r =>
      r.2.slice = it.slice ∧ ∀ x, r.1 = some x → x ∈ it.slice.val := by
  unfold core.slice.iter.IteratorSliceIter.next
  split
  · rw [okspec_ok]
    refine ⟨rfl, fun x hx => ?_⟩
    cases hx
    exact List.getElem_mem _
  · rw [okspec_ok]; exact ⟨rfl, fun x hx => by cases hx⟩


/-- The sense `apply` reads for spec `s`: A's face `from` as `apply` began (side 0), or B's. -/
def SrcSense {S : Type _} (inst : imprint.Src S) (b : S) (a0 : body.Body) (s : boolean.Spec)
    (σ : Bool) : Prop :=
  (s.side = 0#usize ∧ ∃ g, a0.faces.val[(s.from : Std.U32).val]? = some g ∧ σ = g.same_sense) ∨
  (s.side ≠ 0#usize ∧ ∃ srf n, inst.face b s.from = ok (srf, σ, n))

/-- The surface `apply` clones for spec `s`. -/
def SrcSurf {S : Type _} (inst : imprint.Src S) (b : S) (a0 : body.Body) (s : boolean.Spec)
    (srf : forge_geom.surface.Surface) : Prop :=
  (s.side = 0#usize ∧ ∃ g, a0.faces.val[(s.from : Std.U32).val]? = some g ∧ srf = g.surface) ∨
  (s.side ≠ 0#usize ∧ ∃ σ n, inst.face b s.from = ok (srf, σ, n))

theorem faceId_index (f : body.FaceId) :
    body.FaceId.index f = ok (UScalar.cast .Usize (f : Std.U32)) := rfl

theorem cast_usize_val (f : Std.U32) : (UScalar.cast .Usize f).val = f.val :=
  UScalar.cast_val_mod_pow_of_inBounds_eq _ _ (by scalar_tac)

theorem face_eq {a : body.Body} {f : body.FaceId} {g : body.Face} (h : body.Body.face a f = ok g) :
    a.faces.val[(f : Std.U32).val]? = some g := by
  unfold body.Body.face at h
  rw [faceId_index, Std.pure_bind] at h
  have := okspec_of_eq (vec_index _ _) h
  rwa [cast_usize_val] at this

theorem surface_clone_eq {x y : forge_geom.surface.Surface}
    (h : forge_geom.surface.Surface.Insts.CoreCloneClone.clone x = ok y) : y = x :=
  (ok_inj h).symm

/-- What the main loop writes: the sense and surface of a face it adds, from a kept spec. A's
side (0): the face `from` of A as `apply` began, its sense `!= flip`; B's: `B.face(from)`'s. -/
def WrittenW {S : Type _} (inst : imprint.Src S) (b : S) (specs : alloc.vec.Vec boolean.Spec)
    (kept : List (Usize × Bool)) (a0 : body.Body) (ss : Bool) (srf : forge_geom.surface.Surface) :
    Prop :=
  ∃ si flip s, (si, flip) ∈ kept ∧ specs.val[si.val]? = some s ∧
    ((s.side = 0#usize ∧ ∃ g, a0.faces.val[(s.from : Std.U32).val]? = some g ∧
        ss = (g.same_sense != flip) ∧ srf = g.surface) ∨
     (s.side ≠ 0#usize ∧ ∃ b2 n, inst.face b s.from = ok (srf, b2, n) ∧ ss = (b2 != flip)))

/-- The sense and the surface `apply` reads for one spec make the face it adds written. -/
theorem written_of {S : Type _} {inst : imprint.Src S} {b : S} {a0 : body.Body}
    {specs : alloc.vec.Vec boolean.Spec} {kept : List (Usize × Bool)} {si : Usize} {flip : Bool}
    {s : boolean.Spec} {σ : Bool} {srf : forge_geom.surface.Surface}
    (hk : (si, flip) ∈ kept) (hspec : specs.val[si.val]? = some s)
    (hσ : SrcSense inst b a0 s σ) (hsrf : SrcSurf inst b a0 s srf) :
    WrittenW inst b specs kept a0 (σ != flip) srf := by
  refine ⟨si, flip, s, hk, hspec, ?_⟩
  rcases hσ with ⟨h0, g, hg, hσ⟩ | ⟨h1, srf', n, hf⟩
  · rcases hsrf with ⟨-, g', hg', hs⟩ | ⟨h1, -⟩
    · rw [hg] at hg'; cases hg'
      exact Or.inl ⟨h0, g, hg, by rw [hσ], hs⟩
    · exact absurd h0 h1
  · rcases hsrf with ⟨h0, -⟩ | ⟨-, σ', n', hf'⟩
    · exact absurd h0 h1
    · rw [hf] at hf'
      have := ok_inj hf'
      simp only [Prod.mk.injEq] at this
      obtain ⟨rfl, rfl, rfl⟩ := this
      exact Or.inr ⟨h1, σ, n, hf, rfl⟩

/-- Reading A's face `from` in the body as the loop has it, with A's faces kept. -/
theorem read_face {a0 a : body.Body} (hk : FaceKeep a0 a) {f : body.FaceId}
    (hlt : (f : Std.U32).val < a0.faces.val.length) {g : body.Face}
    (h : body.Body.face a f = ok g) :
    ∃ g0, a0.faces.val[(f : Std.U32).val]? = some g0 ∧ g.same_sense = g0.same_sense ∧
      g.surface = g0.surface := by
  obtain ⟨g', hg', hs, hu⟩ := hk _ _ (List.getElem?_eq_getElem hlt)
  rw [face_eq h] at hg'; cases hg'
  exact ⟨_, List.getElem?_eq_getElem hlt, hs, hu⟩

/-- The surface `apply` clones for spec `s` is its source face's. -/
theorem surf_block {S : Type _} (inst : imprint.Src S) (b : S) {a0 a : body.Body}
    (hk : FaceKeep a0 a) (s : boolean.Spec)
    (hlt : s.side = 0#usize → (s.from : Std.U32).val < a0.faces.val.length) :
    okspec (if s.side = 0#usize then do
        let f ← body.Body.face a s.from
        forge_geom.surface.Surface.Insts.CoreCloneClone.clone f.surface
      else do
        let (s1, _, _) ← inst.face b s.from
        forge_geom.surface.Surface.Insts.CoreCloneClone.clone s1)
      (SrcSurf inst b a0 s) := by
  split
  · rename_i h0
    refine okspec_let fun f hf => okspec_mono (okspec_self _) fun y hy => ?_
    obtain ⟨g0, hg0, -, hu⟩ := read_face hk (hlt h0) hf
    exact Or.inl ⟨h0, g0, hg0, by rw [surface_clone_eq hy, hu]⟩
  · rename_i h1
    refine okspec_let fun r hr => ?_
    simp only [uncurry_eta]
    refine okspec_mono (okspec_self _) fun y hy => ?_
    exact Or.inr ⟨h1, r.2.1, r.2.2, by rw [hr, surface_clone_eq hy]⟩

/-- **The main loop** (`for &(si, flip) in kept`): every face of A keeps its sense and surface;
every face it adds is `WrittenW`. (`hfrom`: a spec of A's side names one of A's faces.) -/
theorem loop3_sensed {S : Type _} (inst : imprint.Src S)
    (it : core.slice.iter.Iter (Usize × Bool)) (a : body.Body) (b : S)
    (v : alloc.vec.Vec boolean.Piece) (v1 v2 : alloc.vec.Vec name.Name)
    (v3 : alloc.vec.Vec boolean.Spec) (v4 : alloc.vec.Vec (Usize × Bool))
    (v5 : alloc.vec.Vec forge_math.vec.Point3) (v6 : alloc.vec.Vec name.Name)
    (v7 : alloc.vec.Vec (boolean.VKey × name.Name)) (v8 : alloc.vec.Vec F64)
    (v9 : alloc.vec.Vec (boolean.VKey × F64)) (v10 : alloc.vec.Vec (boolean.VKey × boolean.VKey))
    (v11 : alloc.vec.Vec (name.Name × name.Name)) (v12 v13 : alloc.vec.Vec body.FaceId)
    (pieces : alloc.vec.Vec boolean.Piece) (specs : alloc.vec.Vec boolean.Spec)
    (as_is : alloc.vec.Vec Bool) (new_outer : alloc.vec.Vec Usize) (made : boolean.Made)
    (emap : alloc.vec.Vec (Option body.EdgeId)) (shell : body.ShellId)
    (hfrom : ∀ s ∈ specs.val, s.side = 0#usize → (s.from : Std.U32).val < a.faces.val.length)
    (kept : List (Usize × Bool)) (hkept : it.slice.val = kept) :
    okspec (boolean.apply_loop3 inst it a b v v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 pieces
      specs as_is new_outer made emap shell)
      fun r => Sensed (WrittenW inst b specs kept a) a r := by
  subst hkept
  unfold boolean.apply_loop3
  refine okspec_loop _ (fun st => st.1.slice = it.slice ∧
      Sensed (WrittenW inst b specs it.slice.val a) a st.2.1) _ ?_ _ ⟨rfl, Sensed.refl _ a⟩
  rintro ⟨it', a', made', emap'⟩ ⟨hsl, hs⟩
  dsimp only at hsl hs
  unfold boolean.apply_loop3.body
  refine okspec_bind (iter_next it') fun ⟨o, it1⟩ ⟨hsl1, hmem⟩ => ?_
  dsimp only at hsl1 hmem
  cases o with
  | none => exact (okspec_ok _ _).mpr hs
  | some p =>
    obtain ⟨si, flip⟩ := p
    have hk : (si, flip) ∈ it.slice.val := hsl ▸ hmem _ rfl
    have hit : it1.slice = it.slice := hsl1.trans hsl
    simp only [uncurry_apply_pair]
    refine okspec_let fun b1 _ => ?_
    split
    · exact (okspec_ok _ _).mpr ⟨hit, hs⟩
    refine okspec_bind (vec_index specs si) fun s hspec => ?_
    have hmemS : s ∈ specs.val := List.mem_of_getElem? hspec
    -- the sense
    refine okspec_bind (p := fun r => SrcSense inst b a s r.1) ?_ fun ⟨σ, bx⟩ hσ => ?_
    · split
      · rename_i h0
        refine okspec_let fun f hf => ?_
        go_steps
        all_goals
          rw [okspec_ok]
          obtain ⟨g0, hg0, hs0, -⟩ := read_face hs.1 (hfrom s hmemS h0) hf
          exact Or.inl ⟨h0, g0, hg0, hs0⟩
      · rename_i h1
        refine okspec_let fun r hr => ?_
        go_steps
        all_goals
          rw [okspec_ok]
          exact Or.inr ⟨h1, r.1, r.2.2, by rw [hr]⟩
    simp only [uncurry_apply_pair]
    have hlt : s.side = 0#usize → (s.from : Std.U32).val < a.faces.val.length := hfrom s hmemS
    -- the face: s.from, or one added with the source's surface and sense != flip
    refine okspec_bind (p := fun r => Sensed (WrittenW inst b specs it.slice.val a) a r.1) ?_
      fun ⟨a1, face⟩ ha1 => ?_
    · have add : ∀ srf, SrcSurf inst b a s srf →
          okspec (do
            let (face1, a2) ← body.Body.add_face a' shell srf (σ != flip) s.name
            ok (a2, face1)) fun r => Sensed (WrittenW inst b specs it.slice.val a) a r.1 := by
        intro srf hsrf
        refine okspec_bind (add_face_spec a' shell srf (σ != flip) s.name)
          fun ⟨fid, a2⟩ ⟨_, ⟨g, hg, hsurf, hss, _⟩, _⟩ => ?_
        rw [uncurry_apply_pair, okspec_ok]
        exact Sensed.append hs hg (by rw [hss, hsurf]; exact written_of hk hspec hσ hsrf)
      split
      · split
        · exact okspec_bind (surf_block inst b hs.1 s hlt) add
        · exact (okspec_ok _ _).mpr hs
      · exact okspec_bind (surf_block inst b hs.1 s hlt) add
    simp only [uncurry_apply_pair]
    -- the box: the body as it was
    refine okspec_bind (p := fun r => r = a1) ?_ fun a2 (ha2 : a2 = a1) => ?_
    · go_steps
      all_goals exact (okspec_ok _ _).mpr rfl
    subst ha2
    refine okspec_let fun iter2 _ => ?_
    -- the spec's loops: edges, vertices and loops added, the faces the same
    refine okspec_bind (loop3_loop0_rel inst iter2 (FaceSame.refl a2) b v v1 v2 v3 v4 v5 v6 v7 v8
      v9 v10 v11 v12 v13 pieces made' emap' flip s.extend face) fun ⟨a3, made1, emap1⟩ h3 => ?_
    have ha3 := ha1.same h3
    simp only [uncurry_apply_pair]
    -- the outer loop rotated to the front: the same faces
    go_steps
    all_goals first
      | exact (okspec_ok _ _).mpr ⟨hit, ha3⟩
      | (rw [okspec_ok]; dsimp only
         have hi := ‹alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice body.Face)
           _ _ = ok _›
         exact ⟨hit, ha3.same (FaceSame.of_index_mut hi _ rfl ⟨rfl, rfl⟩)⟩)

theorem FaceSame.length_eq {b b' : body.Body} (h : FaceSame b b') :
    b'.faces.val.length = b.faces.val.length :=
  Nat.le_antisymm h.2.length_le h.1.length_le

theorem into_iter_slice {T : Type _} {A : Type _} {v : alloc.vec.Vec T}
    {it : core.slice.iter.Iter T}
    (h : SharedAVec.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter A v = ok it) :
    it.slice.val = v.val := by
  have := ok_inj h
  subst this
  simp [alloc.vec.Vec.deref, alloc.vec.Vec.val, Slice.from_val]

/-- `WrittenW` relative to a body with the same faces. -/
theorem WrittenW.of_same {S : Type _} {inst : imprint.Src S} {b : S}
    {specs : alloc.vec.Vec boolean.Spec} {kept : List (Usize × Bool)} {a a4 : body.Body}
    (h : FaceSame a a4) {ss : Bool} {srf : forge_geom.surface.Surface}
    (hw : WrittenW inst b specs kept a4 ss srf) : WrittenW inst b specs kept a ss srf := by
  obtain ⟨si, flip, s, hk, hs, h0 | h1⟩ := hw
  · obtain ⟨hside, g, hg, hss, hsrf⟩ := h0
    obtain ⟨g', hg', hs', hu'⟩ := h.2 _ _ hg
    exact ⟨si, flip, s, hk, hs, Or.inl ⟨hside, g', hg', by rw [hss, hs'], by rw [hsrf, hu']⟩⟩
  · exact ⟨si, flip, s, hk, hs, Or.inr h1⟩

/-- `Sensed` from a body with the same faces. -/
theorem Sensed.of_same {S : Type _} {inst : imprint.Src S} {b : S}
    {specs : alloc.vec.Vec boolean.Spec} {kept : List (Usize × Bool)} {a a4 r : body.Body}
    (h : FaceSame a a4) (hs : Sensed (WrittenW inst b specs kept a4) a4 r) :
    Sensed (WrittenW inst b specs kept a) a r :=
  ⟨h.1.trans hs.1, fun i f hi hf =>
    (hs.2 i f (by rw [h.length_eq]; exact hi) hf).of_same h⟩

theorem Sensed.weaken {W : Bool → forge_geom.surface.Surface → Prop} {a r r' : body.Body}
    (hs : Sensed W a r) (h : FaceSame r r') : Sensed W a r' := hs.same h

/-! ## `apply` -/

section Rel2
variable {a0 : body.Body}

theorem loop0_rel {a : body.Body} (h : FaceSame a0 a) (it : core.slice.iter.Iter (boolean.VKey × name.Name)) :
    okspec (boolean.apply_loop0 it a) fun r => FaceSame a0 r :=
  okspec_mono (loop0_same it a) fun _ hr => h.trans hr

theorem loop2_rel {a : body.Body} (h : FaceSame a0 a) (it : core.slice.iter.Iter body.FaceId)
    (staying : alloc.vec.Vec body.FaceId) :
    okspec (boolean.apply_loop2 it a staying) fun r => FaceSame a0 r :=
  okspec_mono (loop2_same it a staying) fun _ hr => h.trans hr

theorem loop4_rel {a : body.Body} (h : FaceSame a0 a) (it : core.slice.iter.Iter (boolean.VKey × F64)) :
    okspec (boolean.apply_loop4 it a) fun r => FaceSame a0 r :=
  okspec_mono (loop4_same it a) fun _ hr => h.trans hr

theorem drop_rel {a : body.Body} (h : FaceSame a0 a) (specs : Slice boolean.Spec)
    (kept : Slice (Usize × Bool)) :
    okspec (boolean.drop_replaced_loops a specs kept) fun r => FaceSame a0 r.2 :=
  okspec_mono (drop_replaced_loops_same a specs kept) fun _ hr => h.trans hr

theorem add_shell_rel {a : body.Body} (h : FaceSame a0 a) :
    okspec (body.Body.add_shell a) fun r => FaceSame a0 r.2 :=
  okspec_mono (add_shell_faces a) fun _ hr => h.trans (FaceSame.of_faces (by rw [hr]))

theorem rebuild_rel {a : body.Body} (h : FaceSame a0 a) :
    okspec (body.Body.rebuild_shells a) fun r => FaceSame a0 r :=
  okspec_mono (rebuild_shells_keep a) fun _ hr => h.trans hr

end Rel2

macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (loop0_rel ‹_› _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (loop2_rel ‹_› _ _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (loop4_rel ‹_› _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (drop_rel ‹_› _ _) fun _ _ => ?_)
macro_rules | `(tactic| same_rel) => `(tactic| with_reducible_and_instances
  refine okspec_bind (add_shell_rel ‹_›) fun _ _ => ?_)

set_option maxHeartbeats 4000000 in
/-- **D8.sense's combinatorial core, of the shipped `apply`.** On its `ok` result, every face of
A keeps its sense and surface, and every face `apply` adds is written from a kept spec: its
sense the source face's `!= flip` (A's face `from` as `apply` began, or `B.face(from)`'s) and its
surface the source's. (`hfrom`: each of A's specs names one of A's faces.) -/
theorem apply_sense {S : Type _} (inst : imprint.Src S) (a : body.Body) (b : S)
    (plan : boolean.Plan)
    (hfrom : ∀ s ∈ plan.specs.val, s.side = 0#usize → (s.from : Std.U32).val < a.faces.val.length) :
    okspec (boolean.apply inst a b plan)
      fun r => Sensed (WrittenW inst b plan.specs plan.kept.val a) a r := by
  unfold boolean.apply
  have h0 := FaceSame.refl a
  same_steps
  all_goals
    have h3e := ‹boolean.apply_loop3 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ = ok _›
    have h4 := okspec_of_eq (loop4_same _ _) ‹boolean.apply_loop4 _ _ = ok _›
    have hA4 : FaceSame a _ := ‹_›
    have h3 := okspec_of_eq (loop3_sensed _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _
      (fun s hs h0 => by rw [hA4.length_eq]; exact hfrom s hs h0) plan.kept.val
      (into_iter_slice ‹_›)) h3e
    have hS := (Sensed.of_same hA4 h3).weaken h4
    first
      | exact (okspec_ok _ _).mpr hS
      | (refine okspec_mono (rebuild_shells_keep _) ?_
         intro r hr
         exact hS.weaken hr)

end ForgeBoolApply
