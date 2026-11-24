/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeBool.Funs

/-!
# D2 on two planes: the shipped `surfaces_in` is the closed form, never the curved search

Ledger row D2 (verify/forge-bool-proof/LEDGER.md), the flat route's first step: on two planes the
translated `ssi::surfaces_in` is `ssi::closed_or_flush` (`surfaces_in_planes`), and its value comes
from forge-geom's `plane_plane` or from the flush rule's `Coincident`. The open L4 gaps on the
curved route (`ssi::searched` and its cache, forge-geom's curved meets) are not reached.

`Only P m` says every value `m` can return satisfies `P`. `Result` is an interaction tree, so it is
stated by continuations: two continuations that agree on those values agree after `m`. It composes
through `bind` and `if`, so a function's `Only` follows its text.

Not here: F1 for two planes (each point of both within δ of the answer). `plane_plane`'s parallel
branch decides Empty/Coincident at `q.frame.origin` (ANGULAR = 1e-11, LINEAR = 1e-5), which
holds only where the crossing is within LINEAR/ANGULAR = 1e6 of it; inside InputSize (|coord| ≤
1.6e7) the exact reference refuted it (B1, research/kernel-audit/tilt-wrong-ok.md). Unless the
normals are parallel to the bit, `closed_or_flush` now has `nearly_flush` decide such planes again
over the region (`parallel_in`: the affine gap at the corners). F1 of that rule is open: it needs
the IEEE error of the corner gaps and of the line's foot.
-/

open Aeneas Aeneas.Std Result
open forge_geom.intersect

namespace forge_bool.d2

/-- Every value `m` can return satisfies `P`: two continuations that agree on those values
agree after `m`. (Stated by continuations: `Result` is an interaction tree.) -/
def Only {α : Type} (P : α → Prop) (m : Result α) : Prop :=
  ∀ {β : Type} (k k' : α → Result β), (∀ a, P a → k a = k' a) → Std.bind m k = Std.bind m k'

theorem Only.ok {α : Type} {P : α → Prop} {a : α} (h : P a) : Only P (Result.ok a) := by
  intro β k k' hk; simp [bind_ok, hk a h]

theorem Only.bind {α γ : Type} {P : α → Prop} {Q : γ → Prop} {m : Result α}
    {f : α → Result γ} (hm : Only P m) (hf : ∀ a, P a → Only Q (f a)) :
    Only Q (Std.bind m f) := by
  intro β k k' hk
  rw [Std.bind_assoc, Std.bind_assoc]
  exact hm _ _ fun a ha => hf a ha k k' hk

theorem Only.bind_any {α γ : Type} {Q : γ → Prop} (m : Result α) {f : α → Result γ}
    (hf : ∀ a, Only Q (f a)) : Only Q (Std.bind m f) := by
  intro β k k' hk
  rw [Std.bind_assoc, Std.bind_assoc]
  congr 1; funext a; exact hf a k k' hk

theorem Only.ite {α : Type} {P : α → Prop} (c : Prop) [Decidable c] {a b : Result α}
    (ha : Only P a) (hb : Only P b) : Only P (if c then a else b) := by
  split <;> assumption

theorem Only.dite {α : Type} {P : α → Prop} (c : Prop) [Decidable c] {a : c → Result α}
    {b : ¬c → Result α} (ha : ∀ h, Only P (a h)) (hb : ∀ h, Only P (b h)) :
    Only P (if h : c then a h else b h) := by
  split <;> simp_all

theorem Only.weaken {α : Type} {P Q : α → Prop} {m : Result α} (hm : Only P m)
    (h : ∀ a, P a → Q a) : Only Q m := by
  intro β k k' hk; exact hm k k' fun a ha => hk a (h a ha)

/-- `m` agrees with `m >>= ok` re-routed through any continuation equal to `ok` on its values. -/
theorem Only.bind_eq {α : Type} {P : α → Prop} {m : Result α} (hm : Only P m)
    (k : α → Result α) (hk : ∀ a, P a → k a = .ok a) : Std.bind m k = m := by
  rw [hm k Result.ok hk, Std.bind_pure]

theorem plane_plane_only (p q : forge_geom.surface.Plane) :
    Only (· ≠ SurfaceSurface.NotAnalytic) (forge_geom.intersect.plane_plane p q) := by
  unfold forge_geom.intersect.plane_plane

  repeat' first
    | exact Only.ok (by simp)
    | apply Only.ite
    | apply Only.bind_any; intro _

/-- B1's re-decision over the region, for planes within ANGULAR of parallel (not to the bit):
Coincident, Empty or a line, or none (`plane_plane`'s answer stands). -/
theorem parallel_in_only (p q : forge_geom.surface.Plane) (region : forge_math.interval.Box3) :
    Only (fun o => o = none ∨ ∃ f, o = some f ∧ f ≠ SurfaceSurface.NotAnalytic)
      (forge_bool.ssi.parallel_in p q region) := by
  unfold forge_bool.ssi.parallel_in
  repeat' first
    | (refine Only.ok ?_; simp)
    | apply Only.ite
    | (apply Only.bind_any; intro _)
    | split

theorem nearly_flush_planes_only (p q : forge_geom.surface.Plane) (region : forge_math.interval.Box3) :
    Only (fun o => o = none ∨ ∃ f, o = some f ∧ f ≠ SurfaceSurface.NotAnalytic)
      (forge_bool.ssi.nearly_flush (.Plane p) (.Plane q) region) := by
  unfold forge_bool.ssi.nearly_flush
  repeat' first
    | exact parallel_in_only p q region
    | (refine Only.ok ?_; simp)
    | apply Only.ite
    | (apply Only.bind_any; intro _)

theorem spindle_in_planes (p q : forge_geom.surface.Plane) :
    forge_geom.sheet.spindle_in (.Plane p) (.Plane q) = ok false := by
  unfold forge_geom.sheet.spindle_in
  simp only [lift, core.slice.Slice.iter]
  simp only [bind_ok]
  rw [core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.any.eq_def]
  simp [Array.to_slice, Array.make, forge_geom.sheet.spindle_in.closure.Insts.CoreOpsFunctionFnMutTupleShared0Shared1SurfaceBool,
    forge_geom.sheet.spindle_in.closure.Insts.CoreOpsFunctionFnMutTupleShared0Shared1SurfaceBool.call_mut]
  rw [core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.any.eq_def]
  simp [forge_geom.sheet.spindle_in.closure.Insts.CoreOpsFunctionFnMutTupleShared0Shared1SurfaceBool.call_mut]
  rw [core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.any.eq_def]
  simp

theorem closed_or_flush_planes_only (p q : forge_geom.surface.Plane) (region : forge_math.interval.Box3) :
    Only (· ≠ SurfaceSurface.NotAnalytic)
      (forge_bool.ssi.closed_or_flush (.Plane p) (.Plane q) region) := by
  unfold forge_bool.ssi.closed_or_flush forge_bool.ssi.closed_form
  simp only [forge_geom.intersect.plane_surface, spindle_in_planes, bind_ok, Std.bind_assoc]
  apply Only.bind (plane_plane_only p q)
  intro r hr
  cases r with
  | NotAnalytic => exact absurd rfl hr
  | _ =>
    simp only [bind_ok, Bool.false_eq_true, ite_false]
    repeat' first
      | (refine Only.bind (nearly_flush_planes_only p q region) ?_; intro o ho
         rcases ho with rfl | ⟨f, rfl, hf⟩ <;>
           first | exact Only.ok hf | (refine Only.ok ?_; simp_all))
      | (refine Only.ok ?_; simp)
      | apply Only.ite
      | (apply Only.bind_any; intro _)
      | dsimp only

/-- D2 on two planes, the flat route: `surfaces_in` is `closed_or_flush`; the curved search
(`ssi::searched`, its cache, forge-geom's curved meets: the L4 gaps L4.searched, L4.cached) is
never reached. -/
theorem surfaces_in_planes (p q : forge_geom.surface.Plane) (region : forge_math.interval.Box3) :
    forge_bool.ssi.surfaces_in (.Plane p) (.Plane q) region =
      forge_bool.ssi.closed_or_flush (.Plane p) (.Plane q) region := by
  unfold forge_bool.ssi.surfaces_in
  apply Only.bind_eq (closed_or_flush_planes_only p q region)
  intro r hr
  cases r <;> simp_all

end forge_bool.d2
