/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import IEEE754

/-!
# `f64` is the model's binary64

Aeneas translates Rust's `f64` to a type `F64` it leaves undefined. Here it is the IEEE 754
model's binary64 datum, so every translated `f64` value is an `IEEE754.FP binary64`.
-/

/-- Rust's `f64`. -/
abbrev F64 := IEEE754.F64
