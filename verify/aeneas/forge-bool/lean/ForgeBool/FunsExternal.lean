/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeBool.Types

/-!
# The opaque functions of forge-bool's translation (FunsExternal_Template.lean)

forge-math's and forge-geom's functions are their own translations (translate.conf `DEPS`); `f64`'s
and std's are the float bridge's. What is left here is an **open L4 gap**: a Lean `opaque`
constant, with no definition a proof can see, so a theorem holds whatever it answers. Each is a
row of verify/forge-bool-proof/LEDGER.md ("L4 gaps"):
- `ssi::searched`: the curved search behind the searched-pairs cache (a thread-local map, G9).
  `closed_or_flush` answers every pair of analytic surfaces with a closed form, so a flat pair
  never reaches it.
- forge-geom's meets of screw, ruled and spline surfaces, and a spindle torus's sheet filter
  (`screw::coaxial::meet_coaxial`, `ruled::meet::meet_ruled`, `spline::meet::meet_spline`,
  `sheet::used_by`): curved, not yet translated. `closed_form` calls the first three only when
  `plane_surface` (or the cylinder and sphere pairs) found no closed form, and `used_by` only for a
  spindle (`spindle_in`, translated).
-/

open Aeneas Aeneas.Std Result
open forge_bool

namespace forge_bool

/-! ## forge-math's constants

Aeneas reads another crate's `const` as an opaque `Result` global; forge-math's translation defines
each as its value. These bind the one to the other. -/

def forge_math.tol.LINEAR : Result F64 := ok _root_.forge_math.tol.LINEAR
def forge_math.tol.ANGULAR : Result F64 := ok _root_.forge_math.tol.ANGULAR

/-- `forge_bool::ssi::searched`: an open L4 gap (G9). -/
opaque ssi.searched : forge_geom.surface.Surface → forge_geom.surface.Surface →
    forge_math.interval.Box3 → Result forge_geom.intersect.SurfaceSurface

/-- `forge_geom::screw::coaxial::meet_coaxial`: an open L4 gap (curved). -/
opaque forge_geom.screw.coaxial.meet_coaxial : forge_geom.surface.Surface →
    forge_geom.surface.Surface → Result (Option forge_geom.intersect.SurfaceSurface)

/-- `forge_geom::ruled::meet::meet_ruled`: an open L4 gap (curved). -/
opaque forge_geom.ruled.meet.meet_ruled : forge_geom.surface.Surface →
    forge_geom.surface.Surface → Result (Option forge_geom.intersect.SurfaceSurface)

/-- `forge_geom::spline::meet::meet_spline`: an open L4 gap (curved). -/
opaque forge_geom.spline.meet.meet_spline : forge_geom.surface.Surface →
    forge_geom.surface.Surface → Result (Option forge_geom.intersect.SurfaceSurface)

/-- `forge_geom::sheet::used_by`: an open L4 gap (curved: a spindle torus's used sheet). -/
opaque forge_geom.sheet.used_by : forge_geom.surface.Surface → forge_geom.surface.Surface →
    forge_geom.intersect.SurfaceSurface → Result forge_geom.intersect.SurfaceSurface

end forge_bool
