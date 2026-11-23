# SendCutSend: what its public pages state (read 2026-11-22)

Notes behind `kernel/forge-vendor/data/sendcutsend.toml`. Restated from the vendor's public
design pages; not an official profile. Verify with the vendor before ordering. Inches unless
marked.

## Pages read (HTTP 200)

- https://sendcutsend.com/bending-calculator/ (per material and thickness: K, bend deduction at
  90deg, effective inside radius, die width, min formed flange, min corner relief distance from
  the bend line, bend relief depth). The table values were taken from the page's static HTML,
  row by row, by a script, not retyped.
- https://sendcutsend.com/materials/processing-min-max/ (min flat flange, max bend length per
  thickness; processing size limits)
- https://sendcutsend.com/services/cnc-bending/ (same content as /guidelines/bending/)
- https://sendcutsend.com/services/sheet-cutting/
- https://sendcutsend.com/materials/min-max/
- https://sendcutsend.com/materials/gauge-sizes-for-sheet-metal/
- https://sendcutsend.com/guidelines/bend-deformation/
- https://sendcutsend.com/services/hardware/ and https://sendcutsend.com/guidelines/hardware/catalog/
- https://sendcutsend.com/services/countersinking/ and https://sendcutsend.com/services/tapping/
- https://sendcutsend.com/materials/5052-aluminum/, /6061-aluminum/, /mild-steel/, /stainless-steel/
- https://sendcutsend.com/blog/guide-to-designing-bend-reliefs/
- https://sendcutsend.com/blog/basic-tolerances-and-cut-feature-relationships/

Not readable: the per-thickness "View specs" pop-ups on the material pages, which are not in the
static HTML. So there are no per-thickness minimum hole, tap hole-to-edge or bridge values.

## Choices where the pages disagree

- **Minimum hole and web.** The cutting and bending pages and the tolerances blog say holes and
  bridges can be as small as 50% of the thickness. The 5052 and 6061 material pages recommend
  1x the thickness. The profile uses 0.5 t, the stated capability. The note says so.
- **Hole to edge.** The tolerances blog gives 0.5 t (bridge to the edge). The 5052 page
  recommends 2 t. The profile uses 0.5 t.
- **Maximum cut size.** The material pages' headers say 44 x 68 for an instant quote; their FAQs
  say 30 x 44. The min-max chart gives 44 x 68 for most thicknesses. The profile uses 44 x 68 for
  flat parts. For bent parts it uses the bending page's 30 x 44.
- **5052 .080 radius.** The calculator gives .038; the processing chart gives .045. The profile
  uses the calculator's value.
- **K and bend deduction.** Both are published, and they do not agree exactly with each other.
  The profile bends by K (the kernel's rule), so its flat pattern can differ from the
  calculator's deduction by up to a few thousandths of an inch. Both values are kept in the file.
- **Hole and feature to bend.** The rule is "at least half the die width from the bend
  centerline". It is held as half the die width measured from the bend's centre line
  (`hole_to_bend_from = "centre"`), and the same rule is used for bend to edge.

## Not offered for bending

- 6061-T6 (it is cut only).
- Hems, coining, bumps, rolls and custom radii.

## Hardware

The catalog's H is the hole, its K is the minimum distance from the hole centre to a cut edge,
and its N is the same to the bend allowance area. Each catalog row also gives the thinnest sheet
it takes.

The profile holds the nut rows for Al/steel (S, SS) and stainless (SP) in 4-40 through 1/4-20 and
M3 through M6. The catalog gives no maximum thickness per nut, so the service's 0.500 range is
used. No hardness is published.
