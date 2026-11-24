/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeTopoBody

/-!
# The opaque types of forge-bool-apply's translation (TypesExternal_Template.lean)

- `core::iter::adapters::filter::Filter<I, P>`: std's fields, the iterator and the predicate
  (Aeneas's library has `Map` but not `Filter`).

The other types the template lists are defined by the DEPS' translations (forge-math, forge-geom,
forge-topo-body) and the float bridge.
-/

open Aeneas Aeneas.Std

universe u v in
/-- `Filter<I, P>`: `iter` and `predicate`, as std's struct. -/
structure core.iter.adapters.filter.Filter (I : Type u) (P : Type v) where
  iter : I
  predicate : P
