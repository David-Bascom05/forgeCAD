/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeNum.Types

/-!
# The opaque functions of forge-num's translation (FunsExternal_Template.lean)

translate.sh lists, in FunsExternal_Template.lean, every function Aeneas left opaque. Each one must be
defined here, in the float bridge (verify/aeneas/bridge), or by a DEP's translation; translate.sh
fails and names any that is not.

What the pinned Aeneas Std (2e3d448) lacks, checked in its source, and the wrapper file in
kernel/forge-num keeps behind one name each:
- `unsigned_abs` (i128 → u128): define it as `if x < 0 then -x else x` on the integer value.
- `trailing_zeros` (u32/u64/u128): define it by its value (the largest k with 2^k ∣ x, and the bit
  width for 0), as a `def` a proof can unfold.
What Std has: I128/U128 arithmetic (overflow-checked: `ok` proves no overflow), checked_add/sub/mul/
div/rem, leading_zeros, wrapping_add/sub/mul/shl/shr, overflowing_*, saturating_*, rotate_*.

Anything kept opaque for speed (iterator chains Aeneas cannot take, and so on) is a Lean `opaque`
here, never an `axiom`: no definition a proof can see, so a theorem holds whatever it answers. Each
one is an open L4 gap; list it in the ledger.
-/

open Aeneas Aeneas.Std Result
open forge_num

namespace forge_num

end forge_num
