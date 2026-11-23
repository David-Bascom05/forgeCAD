-- The opaque std functions forge-sheet-core's translation calls, defined (never axioms): each
-- is verify/ieee754's binary64 operation, as the bridge (verify/aeneas/bridge) defines
-- `core.f64.F64.sqrt`.
import Aeneas
import AeneasF64
import ForgeSheetCore.Types
open Aeneas Aeneas.Std Result

namespace forge_sheet_core

/-- `f64::sqrt`: IEEE 754 square root, correctly rounded (`IEEE754.sqrt_eq_round`). -/
def std.f64.F64.sqrt (x : F64) : Result F64 := ok (IEEE754.FP.sqrt x)

end forge_sheet_core
