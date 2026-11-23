# Sketch text in the CADFS importer (2026-11-23)

The coverage census (research/import/coverage.md, on the import branch) found sketch text the
cause for 58 of the 1914 sampled CADFS (Onshape) parts. This branch adds text to `.fg` sketches
(`text(..)`, interface/forge-lang/src/reference.md) and reads Onshape's `skText` into it
(tools/find-parts/recreate/cadfs.py). Data was staged as the census staged it, outside the repo:
the 1914 parts `sorted(unique_models.json)[::200]` of VladPyatov/CADFS at dd76a9b, by HTTP range.
"Before" is this branch's base (3dcdfb4), "after" this branch; both kernels built from their trees.
Truths are OpenCascade's measures of the dataset's STEP files (step_truth.py). Tables from
research/import/tally.py.

## How Onshape places text

Read off the sketch records (`BTSketchTextInfo`) and the STEP solids:

- the font is Open Sans **1.10** (2011): every glyph extent the STEP solids show agrees with 1.10
  to the scale's precision, and not with the 2020 Open Sans (its 'D', 'E' differ by whole units);
- the box's `lowerLeftCornerPosition2d` is the pen's start on the baseline;
- `height` is 0.72 of the em (the scale every glyph extent gives, 0.010343 mm a unit in 00015890,
  is height / 0.72 / 2048);
- `baselineDirection2d` gives the angle (its length is the box's width: the pen's start to the last
  glyph's right ink edge, not used).

So the shipped Open Sans is 1.10 (Apache 2.0) and the importer writes
`text("..", at: corner, size: height / 0.72, font: "Open Sans", bold:, italic:, angle:)`.

## Fonts by name

Onshape names a sketch text's font by file. Of the 58 parts, 51 name only `OpenSans-Regular.ttf`
or `OpenSans-Bold.ttf`, which are shipped (matched by name); 7 name a font not shipped
(Allerta Stencil 2, Droid Sans Mono 2, Roboto Slab Bold 2, Arimo Bold 1) and stay untranslated,
the cause naming the font.

## Before and after

| class | before | after |
|---|---:|---:|
| match (1e-4) | 1099 | 1114 |
| mismatch | 81 | 88 |
| refused | 85 | 87 |
| fail | 8 | 9 |
| timeout | 1 | 1 |
| untranslated | 640 | 615 |

Sketch text as the cause: 58 before, 0 after. Of the 58: 25 translate (15 match, 7 off, 3
refused or fail); 7 name a font not shipped; 7 draw text beside other curves in one sketch
(`sketch text beside other curves`: the regions of text mixed with other curves are not taken);
4 find no record of the text (3) or two alike (1); the rest stop at the next cause they hit
(a reference to an entity the sketch no longer has, 9; others 1 or 2 each).

No part changed class but those 25: nothing that matched before is lost, nothing that built
before fails.

The 7 off: 00239261 (2.1e-1) and 00245556 (5.5e-2) have text in a sketch with other regions
the part takes; the five others are off by 1.3e-4 to 1.0e-3 in volume. One of them, 00893478, is a
single 'C' extruded 25.4 mm: its box differs from ours by a few micrometres, a different amount
on each side, where the glyph is 52 mm tall. That is a difference of shape, not of placement or
scale (00015890's twelve characters agree to 1.4e-8), consistent with Onshape fitting its own
curves to the font's quadratics; ours are the font's curves exactly.

The 3 refused or failing: two fillets refused (`Overflow`) and one cut's boolean (`SplitCrossed`)
on the part around the text.
