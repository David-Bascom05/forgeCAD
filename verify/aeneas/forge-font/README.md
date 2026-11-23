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
| L4 | translated: 22 definitions, none opaque; elaborates. Proofs against verify/forge-text's specs (`decode`, `decode_closes`): in progress |
