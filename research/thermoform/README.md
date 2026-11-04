# Thermoforming: where the numbers come from

## The placeholder shop (`kernel/forge-thermo/src/material.rs`, `placeholder()`)

The design guides give one set of rules for every material; no maker gives per-material draft,
radius or spacing. Used as written:

- Draft: male tool 4 deg, female 2 deg. Universal Plastics, Thermoforming Design Guidelines v1.21
  (https://www.universalplastics.com/wp-content/uploads/2014/03/UP-Design-Guide-v1.21.pdf): "min 4
  deg male (3-4 general), min 2 deg female (1.5-2 general)". Formech's Vacuum Forming Guide is
  stricter (5 deg male, 3 deg female); ACRYLITE recommends over 5 deg.
- Radii: a male tool's outside radius at least t + 1/32 in (0.8 mm); a female tool's inside corner
  as tight as 1/64 in (0.4 mm). Universal Plastics, same guide. Taken as convex t + 0.8 and concave
  0.4 on the tool's surface, either side.
- H:D: 1:1 without special techniques. Universal Plastics.
- Spacing between raised parts: 1 in of gap per 1 in of depth. Eastman, "Tips for thermoforming
  Eastar 6763 and Tritan MP100" (https://www.eastman.com/content/dam/eastman/corporate/en/literature/t/trsmed269.pdf).
  Mayku and Formech say the same in words.
- Vent holes: under half the sheet's thickness at the tool's surface (0.5-1 mm typical), back-drilled
  larger; placed where the sheet touches last (corners, cavities, internal edges). Formech, Vacuum
  Forming Guide (https://www.capla.arizona.edu/sites/default/files/fromechvacuumguide_0.pdf).
  ACRYLITE allows up to t.
- Shrink (the middle of the published ranges): ABS 0.6% (Formech 0.3-0.8, Spartech 0.45-0.7),
  HIPS 0.5% (0.3-0.5 / 0.45-0.65), PETG 0.5% (Spartech 0.5-0.7, Eastman Eastar 0.40-0.45), PC 0.7%
  (Formech 0.6-0.8), PMMA 0.6% (Formech 0.3-0.8), HDPE 3% (Formech 3.0-3.5, Spartech 2.2-3.0).
  Spartech, Extruded sheet thermoforming information, rev. 7/2024
  (https://www.spartech.com/media/documents/SPA221015-Thermoforming-Data-Sheet.pdf).
- Forming temperatures (reported, not checked): Formech's ranges.
- Stand-ins, said on every run: the areal draw ratios (RapidMade's "typical safe" upper figures,
  a vendor blog: ABS 3.0, HIPS 3.5, PETG 2.5, PC 2.0, HDPE 1.6, PMMA 2.0), the thinnest wall
  (0.25 t, a guess), and the placeholder machine's 25 mm clamp.

## Prior art

No CAD package checked (Inventor, SolidWorks, Fusion, Creo, NX, Onshape) has a thermoform feature;
they offer draft and radius analysis. The full survey, with the wall-thickness methods, lives
outside the repo with the design notes.
