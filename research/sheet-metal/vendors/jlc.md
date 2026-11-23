# JLCCNC sheet metal (JLC): what its public pages state (read 2026-11-23)

Notes behind `kernel/forge-vendor/data/jlc.toml`. This is restated from JLCCNC's public help and
capability pages and is not an official profile. Verify with the vendor before ordering.
Measurements are in mm.

## Pages read

All of these returned HTTP 200 and are server-rendered.

Official help and capability pages, which are the ones used:
- https://jlccnc.com/help/article/sheet-metal-fabrication-guidelines (updated Oct 26, 2026). Its
  bend-constraint table is an image, Bending-Design-Constraints.png, which was read visually and
  checked against the transcription.
- https://jlccnc.com/capabilities/sheet-metal-fabrication
- https://jlccnc.com/sheet-metal-fabrication
- https://jlccnc.com/help/article/materials-supported-in-sheet-metal-fabrication
- https://jlccnc.com/help/article/sheet-metal-fasteners-chart. Its tables are images. The chart
  says its values are "manually measured, for reference only" and that availability depends on
  the factory.
- https://jlccnc.com/help/article/sheet-metal-fabrication-file-requirements

JLCCNC's blog posts (sheet-metal-design, sheet-metal-bending, sheet-metal-enclosure-design) give
generic rules of thumb. They disagree with each other and with the guidelines, so none of their
numbers are used.

## What the profile takes

- **Minimum flange and hole to bend.** For each thickness the profile takes the table's minimum
  bending edge for a single bend as `min_flange`, and its circular-hole edge to bending edge as
  `hole_to_bend`.
  - Each stock's note carries the second-bend minimum and the square-hole distance.
  - The table does not say where it measures from. The profile uses the tangent line, the
    stricter reading, and measures the flange to the outside, as for the other vendors.
- **Minimum hole and hole spacing.**
  - Minimum hole: at least half the thickness and at least 1 mm, so `max(0.5t, 1)`.
  - Minimum distance between holes: 1 mm, held as `min_web`.
- **Tolerances**, as quoted. The bend angle tolerance is ±1.0° on the capability page; the
  guidelines print "+1.0°".
- **Materials and thicknesses.** Every material comes in 0.5, 0.8, 1.0, 1.2, 1.5, 2.0, 2.5, 3.0,
  4.0, 5.0 and 6.0 mm, and other thicknesses are not supported. The materials are 5052 aluminum
  (no temper stated), 201 and 304 stainless, SPCC cold rolled and galvanized.
  - Only 304 names a library material (304 annealed), for mass. The others are not given one
    rather than assumed: 5052's temper is not stated, and SPCC and galvanized steel are not in
    the library.
- **Hardware.** The profile holds clinch nuts (CLS, code 1), studs (FH) and through-hole
  standoffs (SO), each with its mounting hole, minimum sheet thickness and minimum distance.
  - The minimum distance is taken as hole centre to edge; the chart does not say.
  - No maximum sheet thickness is published, so 6 mm, the thickest stock, is used.

## Not published

These are not in the profile and are not checked:
- **Bend radius and K.**
  - JLCCNC gives no K-factor, bend deduction or inside radius for its own process.
  - The guidelines list an "outer R corner radius" per thickness: 0.5→1.2, 1→1.8, 1.2→2,
    1.5→2.75, 2→3.7, 2.5→5, 3→6, 4→8, 5→10, 6→12. The term is not defined, so it is kept as
    each stock's note and not converted to an inside radius.
  - A bent sheet on this profile must give `r:` and `bend:` itself.
- **Part and bend limits.**
  - Maximum part length is 800 mm, with no width given. Because a size check needs both sides,
    this is in the note and is not checked.
  - No minimum part size, maximum bend length or bend angle range is published.
- **Hole to edge and relief size.**
- **V-die openings.** The press-brake list names a TRUMPF brake and standard, step and radius
  dies (R3 to R20), with no V openings.
