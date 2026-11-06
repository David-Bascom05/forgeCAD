# atmani2020_cup_d51: assumptions

- The same case as atmani2020_cup on the cup geometry measured off the authors' render, to see which geometry the
  data support. From s2_geom_atmani2020_cup_dims_from_render.csv: depth 67.5 VERIFIED (printed, p.10 sec. 5.3); upper-wall outer diameter 51 [48-56],
  bottom fillet 21 [12-25], upper-wall draft 1.8 deg [0-3] DERIVED (Fig. 15 silhouette fit). atmani2020_cup.fg keeps
  the older 70 / 56 diameters (derived from the 200 mm centreline length, assuming no flange in the profile).
- Plug (the _plug case): d42 (printed for the plug-only plug; the same plug model, p.10-11), 20 mm above the sheet,
  80 mm stroke (printed, p.10 sec. 5.3) from there, so the tip ends 60 mm below the sheet (89 % of the 67.5 depth, the
  printed fraction drawn by the plug): plug = [21, 90, 10, 20, 60], as atmani2020_cup_plug. Plug to cavity 0.82.
- Flange 10 as atmani2020_cup (the mould top disc is ~144 [133-159], so the real flange is wider; kept equal for
  the comparison). Data and fold as atmani2020_cup. The tail re-digitisation (s2_atmani2020_cup_thickness_tail.csv,
  160-202 mm) is within 0.04 mm of the scored file up to 195 mm and not scored separately.
