-- The opaque std functions forge-sheet-core's translation calls: only `f64::sqrt`, which the
-- bridge defines (verify/aeneas/bridge AeneasF64/Std.lean `std.f64.F64.sqrt`, verify/ieee754's
-- correctly rounded square root, never an axiom).
import Aeneas
import AeneasF64
import ForgeSheetCore.Types
