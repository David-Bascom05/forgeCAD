# kernel/forge-sheet-core in Lean, by Aeneas (L4 of research/PROOF-STANDARD.md)

The shipped Rust of `kernel/forge-sheet-core` (forge-sheet's combinatorial core: vertex classes,
chaining, a hole's owner) is translated by `tools/aeneas/translate.sh forge-sheet-core` (Charon
9e193f0, Aeneas 2e3d448 with the float patch; 26 definitions, nothing opaque) into
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
- under them: the least-root union–find (`new_spec`, `find_spec`, `union_spec`), ported from the
  decide-once core's proofs (research/kernel-audit/decide-once/aeneas: the same `union_find.rs`),
  with the release build's wrapping `+= 1` shown not to wrap.

All of forge-sheet-core is proven. What remains of region.rs's L4 is the wiring (the fix-sheet
session: region.rs calls `classes`, `chain` and `owner`, bits identical) and the geometric
predicates feeding them (L5: `near` is proven, Sheet/Float.lean; `within` and the crossing signs
next); until then the region oracle (kernel/forge-sheet/tests/lean_diff_region.rs) is the interim. The bridge (`verify/aeneas/bridge`) and
`tools/aeneas/` are the aeneas session's, copied here unchanged so this builds on its own.

Build: `cd lean && lake exe cache get && lake build`.
