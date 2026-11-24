# forge-num's translation package, ready to drop in (TRANSLATE for EXACT-CORE)

This is verify/aeneas/forge-num as it should look once kernel/forge-num exists. It waits here, not
under verify/aeneas, because `tools/aeneas/translate.sh --all` and CI's Lean matrix (aeneas.yml,
every verify/aeneas/*/lean) would fail on a package with no crate behind it. The general how-to is
research/aeneas/NEW-CRATE.md.

## Drop in (in the same commit as the first translatable kernel/forge-num)

1. `git mv research/aeneas/forge-num-scaffold verify/aeneas/forge-num`, and drop this README or turn
   it into the package's notes.
2. Fill in translate.conf's `--start-from` roots: the modules that exist (`forge_num::nat`,
   `forge_num::big` are placeholders; use the real paths).
3. `tools/aeneas/translate.sh forge-num` writes lean/ForgeNum/{Types,Funs}.lean and the two
   `*_Template.lean` files. If it names an opaque with no definition, define it in
   lean/ForgeNum/FunsExternal.lean (`unsigned_abs` and `trailing_zeros` are the two Std lacks; see
   that file).
4. `cd verify/aeneas/forge-num/lean && lake exe cache get && lake build`. This also writes
   lake-manifest.json; commit it.
5. List the translated functions in lean/ForgeNum/Axioms.lean's `#standard_axioms`.
6. `tools/aeneas/translate.sh --all --check` must pass before pushing.

kernel/forge-num needs no Cargo.toml `members` entry: `kernel/*` covers it. Nothing under verify/
here is a Rust crate.

## forge-math on forge-num (the step that drops the four `--opaque *_big`)

In verify/aeneas/forge-math:
- translate.conf: `DEPS=(forge-num)`. Then forge-num's items keep one definition, forge-num's:
  translate.sh drops what Charon brings along from `kernel/forge-num/` and imports ForgeNum. Add
  `--include forge_num::<Type>` for each forge-num type forge-math reads the fields of. Remove the
  `--opaque forge_math::predicates::{cross,orient2d,dot3,incircle}_big` lines once those functions
  translate through forge-num.
- lean/lakefile.toml: add `[[require]] name = "forgenum" path = "../../forge-num/lean"`.
- lean/ForgeMath/FunsExternal.lean: drop the `opaque predicates.*_big` definitions that are now
  translated. forge-num's constants used from forge-math come in as opaque `Result` globals: bind
  each to forge-num's value, as forge-geom's FunsExternal.lean does for forge-math's tolerances.
- Regenerate forge-math, then everything downstream (forge-geom, forge-bool: their DEPS chain
  through forge-math), and run `--all --check`.

Shewchuk's move (step 4): two_sum, split and two_product, with their lemmas (Refine.lean), move to
ForgeNum. ForgeMath restates them through the re-export, so there is one proof.

## Order

Land after #22 (MERGE: forge-math opaques, forge-geom) and #18 (TRANSLATE: forge-math's dmath, and
sum_into/scale_into/estimate translated again). Rebase onto main once both are in.
