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

end ForgeBoolApply
