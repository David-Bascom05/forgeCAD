/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeMath

/-!
# The opaque types of forge-geom's translation (TypesExternal_Template.lean)

- `Arc<T>` is its value: the kernel only reads through an `Arc` (`Curve::Intersection`), so a
  shared pointer to a `T` means the `T`.
- `MaybeUninit<T>` comes in with std's array and vector internals; no translated function reads
  one. It is what it can hold: a value or none.
- **Open L4 gaps (verify/forge-bool-proof/LEDGER.md, "L4 gaps"):** the procedural curve and
  surface (`ProcCurve`, `Proc`: `Arc<dyn …>`) and the traced intersection curve (`IntCurve`) are
  Lean `opaque` types. Charon's `--opaque` keeps them so (translate.conf). A proof that reaches
  one says nothing about it: the analytic cases are what the theorems cover.
-/

open Aeneas Aeneas.Std

namespace forge_geom

def alloc.sync.Arc (T : Type _) : Type _ := T
def core.mem.maybe_uninit.MaybeUninit (T : Type _) : Type _ := Option T

/-- `forge_geom::curve::ProcCurve`, `Arc<dyn ProceduralCurve>`: an open L4 gap. -/
opaque curve.ProcCurve : Type
/-- `forge_geom::surface::Proc`, `Arc<dyn Procedural>`: an open L4 gap. -/
opaque surface.Proc : Type
/-- `forge_geom::intcurve::IntCurve`, a traced intersection curve: an open L4 gap. -/
opaque intcurve.IntCurve : Type

end forge_geom
