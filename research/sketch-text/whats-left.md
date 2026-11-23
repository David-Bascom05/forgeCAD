# Sketch text: what is left (when the session stopped)

## Fonts and formats

- **CFF from other fonts.** `cff_core.rs` reads CFF (OTTO) fonts, but only Source Sans 3 is
  shipped and bit-checked against our own previous f64 machine. formats-corpus found 113 of 502
  fonts are CFF. The next steps:
  - check our CFF outlines bit for bit against FreeType and fontTools, on Source Sans 3 and the
    formats-corpus font set;
  - read CFF2 (variable), which is refused today;
  - read `seac` accents, which are refused today.
- **TrueType placement (formats-corpus #8):** fixed. A glyf outline moves by hmtx lsb − xMin, as
  FreeType's phantom points do (`Font::lsb_shift`, test `placed_by_lsb_less_xmin`). The shipped
  faces have lsb = xMin, so their bits are unchanged. Re-run formats-corpus's font comparison to
  confirm the 39 fonts now match.
- **Unshipped fonts.** Only Open Sans Regular and Bold and Source Sans 3 ship. Onshape's other
  sketch fonts (the CADFS rows `TEXT_FONTS` maps) fall back or are refused. Adding a face means
  its licence (OFL or Apache), its bits pinned, and a contour-simplicity run
  (fonts.rs `shipped_contours_are_simple`).

## Language

- **Multi-line text:** `text([..lines])` lays lines out, but the line spacing is the font's
  ascender − descender + lineGap. It is not checked against Onshape's own multi-line placement,
  and CADFS has no multi-line rows to compare against.
- Italic and bold where the face is missing them are refused, not synthesized.

## Proofs

- L4 is proven for:
  - glyf_core (`read_simple`, `contour`), in verify/aeneas/forge-font;
  - cff_core (`run_spec`), in verify/aeneas/forge-font;
  - `winding::line`, in verify/aeneas/forge-sketch, under forge-math's orient2d sign claim.
- Not proven:
  - the float shell (the halving, a composite's matrix, the lsb shift); the halving and the lsb
    shift are exact on integer coordinates;
  - the hull subdivision bound (L5): stopped by ruling. The margin fix is applied, and the
    provable bound is written up in hull-margin.md, with two per-level lemmas in
    verify/aeneas/forge-sketch/lean/ForgeSketch/Hull.lean.
