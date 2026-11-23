/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeBool.Funs

/-!
# D2 on two planes: the shipped `surfaces_in` is the closed form, never the curved search

Ledger row D2 (verify/forge-bool-proof/LEDGER.md), the flat route's first step: on two planes the
translated `ssi::surfaces_in` is `ssi::closed_or_flush`, and its value comes from
forge-geom's `plane_plane` or from the flush rule. The open L4 gaps on the curved route
(`ssi::searched` and its cache, forge-geom's curved meets) are not reached.
-/

open Aeneas Aeneas.Std Result
open forge_bool

namespace forge_bool.d2

/-- A spindle is a torus: two planes are none. -/
theorem spindle_in_planes (p q : forge_geom.surface.Plane) :
    forge_geom.sheet.spindle_in (.Plane p) (.Plane q) = ok false := by
  simp [forge_geom.sheet.spindle_in, core.slice.Slice.iter, Array.to_slice, Array.make,
    core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.any,
    forge_geom.sheet.spindle_in.closure.Insts.CoreOpsFunctionFnMutTupleShared0Shared1SurfaceBool,
    forge_geom.sheet.spindle_in.closure.Insts.CoreOpsFunctionFnMutTupleShared0Shared1SurfaceBool.call_mut]

end forge_bool.d2
