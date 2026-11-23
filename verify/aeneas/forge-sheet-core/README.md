# kernel/forge-sheet-core in Lean, by Aeneas (L4 of research/PROOF-STANDARD.md)

The shipped Rust of `kernel/forge-sheet-core` (forge-sheet's combinatorial core: vertex classes,
chaining, a hole's owner; the development's rigid motions and joint frames, `place.rs`) is
translated by `tools/aeneas/translate.sh forge-sheet-core` (Charon 9e193f0, Aeneas 2e3d448 with the
float patch, release with overflow checks; 35 definitions, 1 opaque: `f64::sqrt`, which the bridge
defines as verify/ieee754's correctly rounded square root, never an axiom) into
`lean/ForgeSheetCore/{Types,Funs}.lean`, and the translation is proven against verify/sheet.

Proven (no `sorry`; `lean/ForgeSheetCore/Axioms.lean` checks the standard axioms only):
- `classes_spec`: for `n ≤ u32::MAX` points and decided pairs with indices below `n`, `classes`
  returns without panic or overflow; two points get one number iff the pairs connect them, and the
  numbers count the classes' least members in order (region.rs's numbering).
- `owner_spec`: for `k ≤ u32::MAX` outer loops with `holds` of at least `k` bits, `nest` of at
  least `k²` and `k² ≤ usize::MAX`, `owner` returns without panic or overflow the first `o` with
  `isOwnerB` (Region.lean's `isOwner` on the decided bits: `o` holds the hole's vertex and every
  other outer loop holding it holds `o`'s vertex): `List.find?` over `0 .. k-1`.
- `chain_spec`: for kept pieces from `F[e]` to `T[e]` (vertices below `nv ≤ u32::MAX`, fewer
  than `u32::MAX` pieces, `to` at least as long as `from`), `chain` returns without panic or
  overflow; `Ok` exactly when every vertex is left as often as reached and at most once, and then
  the loops are the cycles of the successor permutation (`succ_perm`: each piece leaves where the
  one before it arrives), every piece once, with `starts` their offsets and the total; `Err
  Unchained` at the first vertex of unequal degrees (its first leaving, else arriving piece, and
  both degrees); `Err Pinched` with every vertex of degree past one, in order. The walk's own
  refusal (`at != start`) never happens (`WalkModel.walk_closes`).
- `place.rs`, through the f64 bridge (each operation verify/ieee754's binary64 one, `u = 2⁻⁵³`,
  `η = 2⁻¹⁰⁷⁵`): `apply_spec`, `vec_spec`, `then_spec`, `local_spec`, `inverse_spec` (each computed
  coordinate, when finite, within a stated `a u S + b η` of the exact motion of the doubles'
  values); `develop_err` (DevelopErr.lean): along a path of `n` joints the axis error grows as
  `n ex`, the origin's as `n eo + R n² ex`, never exponentially.
- `joint_spec` (RefineJoint.lean, model JointErr.lean): on the domain where each coordinate
  difference `D = p1 − p0` of the doubles is `0` or in `[2⁻⁵⁰⁰, 2⁵⁰⁰]` in magnitude, not both `0`,
  every step's exact result is zero or normal; then the frame's origin is `p0` exactly and, when
  the outputs are finite, the length is within `(1 + u)³ − 1` of `|D|` and the axis within
  `(1 + u)³ / (1 − u)³ − 1` (about `6 u`) of `(D.y, −D.x) / |D|`, each component relatively.
- `setback_spec` (RefineSetback.lean, model SetbackErr.lean): `fold::setback(ρ, s, c)` (shop.rs's
  `ρ ⊗ s ⊘ (1 ⊕ c)`, op for op) fed dmath's `sin_cos(θ)` (verify/ieee754 `Dmath.sinCos_err`,
  within `ε = 2⁻⁵²`): for finite `ρ`, finite `|θ| ≤ 2²⁰` with `C = 1 + cos θ ≥ 2⁻⁵⁰`, the
  setback, when finite, is within `|ρ| (4ε + 4η) / C² + 10 u |ρ| / C + 3η / C + η` of
  `ρ sin θ / C = ρ tan(θ/2)`. The `ε / C²` term is the hairpin's ill-conditioning.
- `zone_spec` (RefineZone.lean, model ZoneErr.lean): build.rs `Geo::Round`'s point as composed
  there, `fold::phi` (θ at the far tangent line, else `u ⊘ rn`), dmath's `sin_cos`
  (`Dmath.sinCos_err`), `fold::round_local` (`r = ρ0 ⊕ σ⊗w`; `r⊗c`, `r⊗s`, `(−σ)⊗y`) and the axis
  frame's `Frame::point_to_world` from the aeneas session's forge-math translation (`ptw_spec`, not
  a copy): for finite inputs, `rn ≠ 0`, the exact angle within `2¹⁹`, each world coordinate, when
  finite, is within `ptwE` of the exact `o + A (R cos φ, R sin φ, −σ y)`, the local point's errors
  (`locE`: the angle's rounding, dmath's `2⁻⁵²`, each product's) carried through the axes.
  Composed from `mul_approx`/`add_approx`, one forward-error rule per operation.
- under them: the least-root union–find (`new_spec`, `find_spec`, `union_spec`), ported from the
  decide-once core's proofs (research/kernel-audit/decide-once/aeneas: the same `union_find.rs`),
  with every `+= 1` (overflow-checked in the canonical translation) shown not to overflow.

All of forge-sheet-core is proven. What remains of region.rs's L4 is the wiring (the fix-sheet
session: region.rs calls `classes`, `chain` and `owner`, bits identical) and the geometric
predicates feeding them (L5: `near` is proven, Sheet/Float.lean; `within` and the crossing signs
next); until then the region oracle (kernel/forge-sheet/tests/lean_diff_region.rs) is the interim. The bridge (`verify/aeneas/bridge`) and
`tools/aeneas/` are the aeneas session's, copied here unchanged so this builds on its own.

Build: `cd lean && lake exe cache get && lake build`.
