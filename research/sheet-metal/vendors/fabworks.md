# Fabworks: what its public pages state (read 2026-11-22)

Notes behind `kernel/forge-vendor/data/fabworks.toml`. Restated from the vendor's public pages;
not an official profile. Verify with the vendor before ordering. Inches.

Pages read (HTTP 200; each also serves Markdown with `.md` appended):
- https://www.fabworks.com/resources/guidelines/bending (the Material Table)
- https://www.fabworks.com/resources/guidelines/laser-cutting/sheet
- https://www.fabworks.com/resources/guidelines/hardware
- https://www.fabworks.com/resources/materials/sheet/aluminum, /steel and /stainless-steel
- https://www.fabworks.com/blog/minimum-web-width-heat-distortion
- https://www.fabworks.com/faq
- https://www.fabworks.com/resources/guidelines/max-part-size

## Bending

The bending page's Material Table gives, for each stock bent:
- max bend length and max angle;
- min flange and distortion zone at 90 deg;
- inside radius at 90 deg and tooling radius;
- inside radius at 135 deg;
- min flange at 45 and 135 deg;
- K.

Fabworks air bends, and its K and radius come from measured 90 deg test bends. The profile
takes from that table:
- the 90 deg inside radius as the one tooled radius;
- the K;
- the 90 deg minimum flange;
- the distortion zone as hole to bend, measured from the bend's centre (`bend_from = "centre"`:
  the zone runs out from the bend line on each side);
- the max angle and the max bend length.

Fabworks says any radius from the punch radius up to the finished radius can be modelled. The
profile holds only the 90 deg radius. A different radius is reported as RadiusNotTooled, which
may be stricter than Fabworks is.

## Cutting and features

- **Min hole and slot.** The material pages give a minimum feature size per stock; the profile
  uses it for both.
- **Web and hole to edge.** Both are 1 t, the web-width blog's rule of thumb. That blog calls a
  web "the narrowest strip of metal left between cuts that stays flat"; it is wider than the
  minimum feature.
- **Relief.** Width at least 1 t and at least 0.030; depth at least r + t. The guide measures the
  depth "from the flange edge"; the profile takes it from the bend line.

## Not bent

6061-T6 and 7075-T6 ("cracks on sharp bends"), and A36 at 0.375 and 0.500.

## Hardware

Fabworks lists its PEM families and hole sizes. It publishes no hole-centre-to-edge distance, so
no catalogue rows are in the profile: `hardware` is offered, but `pem` finds no part in it.
