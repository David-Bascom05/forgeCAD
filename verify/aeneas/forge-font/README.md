# kernel/forge-font in Lean (Aeneas)

`translate.conf` names what is translated, written to research/aeneas/STYLE.md:
- the TrueType simple-glyph reader and the contour walk, in integers
  (kernel/forge-font/src/glyf_core.rs);
- the Type 2 charstring machine, in integers of 2⁻¹⁶ font units (kernel/forge-font/src/cff_core.rs).
  Subroutine calls push a frame instead of recursing, and a refusal is a code that stops the loop. The float
shell (`Font::outline_into`) maps the walk's pieces through a composite's matrix; every glyph of
the shipped faces comes out bit for bit as before the split.

`lean/` requires the float bridge (verify/aeneas/bridge) and the specs (verify/forge-text).
`tools/aeneas/translate.sh forge-font [--check]` writes (or checks) `lean/ForgeFont/{Types,Funs}.lean`;
then `lake build` in `lean/`.

| Level | |
|---|---|
| L4 | translated: 66 definitions. Four are opaque std items, each defined in `lean/ForgeFont/FunsExternal.lean` as std documents it: `Vec::default`, `Vec::clear`, `Vec::pop`, `mem::take`. Proven (`lean/ForgeFont/{Walk,Contour,ReadSimple,Cff}.lean`, standard axioms only: `lake env lean Axioms.lean`, no `sorry`). See below |

## What is proven

- `step_spec`: one step of the translated walk is ForgeText's `step`.
- `contour_spec`: `glyf_core::contour g first last` returns ForgeText's `contourOf` of the points
  `first..=last` on the doubled grid. It does so with no overflow and no failed index, for every
  glyph whose three arrays have one length and with `first ≤ last` in range.
- `contour_closes`: so every contour it makes is closed.
- Rust's truncating `/` is the spec's floor division here, because every sum it halves is even
  (`Grid`, carried as an invariant).
- `read_simple_spec`: `glyf_core::read_simple` never fails on any bytes (no overflow, no index out
  of range), for offsets and data under `Usize.max − 2¹⁸` and fewer than 2¹⁶ contours. When it
  returns a glyph, the glyph has `n` ends, each below 2¹⁶, and as many on flags, x and y
  coordinates as the last end says.
  - Each loop's lemma is proven on its own: `ends_spec`, `flags_spec`, `coords_loop_spec` (the
    running i64 sum stays within ±k·2¹⁵), `on_spec`.
- `contour_of_read`: the two together, as `Font::outline_into` uses them. Every contour the shell
  asks for (it refuses unless `first ≤ end < on.len`) is `contourOf` of its points.

- `run_spec` (CFF): for any bytes, any charstring range and any subroutine INDEXes,
  `cff_core::run` returns. There is no overflow, no index out of range and no runaway: at most
  2¹⁶ steps, and subroutines nest at most 10 deep. Every contour it returns is closed (`Closed`:
  its pieces chain from a start and end back on it).
  - The pen's invariant `PenOk`: every kept contour is closed, and the open one chains from its
    start to the pen.
  - It is proven for `close`, `move_to`, `line` and `curve`, then for each operator family
    (`lines`, `curves`, `fours`, `alternating`, the flexes), for `op` and for one step (`one`).
  - The loop carries `RunInv`: each step moves the pen less than 2³⁷ (every operand is within
    2³¹), so coordinates stay within 2⁵³.
  - Every coordinate is therefore exact as the shell's f64 (`/ 65536`). All the shipped faces'
    glyphs are bit-identical to the f64 machine this replaced.

Not covered: the float shell itself. It halves each coordinate (exact: they are integers below
2³³) and maps it through a composite's matrix.

The line crossing is in verify/aeneas/forge-sketch.
