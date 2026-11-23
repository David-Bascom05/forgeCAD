# kernel/forge-font in Lean (Aeneas)

`translate.conf` names what is translated: the TrueType simple-glyph reader and the contour walk,
in integers (kernel/forge-font/src/glyf_core.rs), written to research/aeneas/STYLE.md. The float
shell (`Font::outline_into`) maps the walk's pieces through a composite's matrix; every glyph of
the shipped faces comes out bit for bit as before the split.

`lean/` requires the float bridge (verify/aeneas/bridge) and the specs (verify/forge-text).
`tools/aeneas/translate.sh forge-font [--check]` writes (or checks) `lean/ForgeFont/{Types,Funs}.lean`;
then `lake build` in `lean/`.

| Level | |
|---|---|
| L4 | translated: 22 definitions, none opaque. Proven (`lean/ForgeFont/{Walk,Contour}.lean`, standard axioms only: `lake env lean Axioms.lean`): `step_spec`, one step of the walk is ForgeText's `step`; `contour_spec`, `glyf_core::contour` returns ForgeText's `contourOf` of the points `first..=last` on the doubled grid, with no overflow and no failed index, for every glyph whose three arrays have one length and `first ≤ last` in range; `contour_closes`, so every contour it makes is closed. Rust's truncating `/` is the spec's floor division here because every sum it halves is even (`Grid`). Next: `read_simple` (never fails, and its arrays have the lengths `contour` needs) |
