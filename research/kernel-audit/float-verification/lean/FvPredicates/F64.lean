/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import AeneasF64

/-!
# `f64` is the model's binary64

Aeneas translates Rust's `f64` to a type `F64` that it leaves undefined. The shared float bridge,
`verify/aeneas/bridge` (`AeneasF64`), defines it as the IEEE 754 model's binary64 datum
(`abbrev F64 := IEEE754.F64`). It also defines `f64::abs`, `f64::from_bits` and
`<f64 as Clone>::clone`, which this copy's translation calls. The same definitions serve the
translation of the real crate, verify/aeneas/forge-math, so theorems about the two can be stated
side by side.
-/
