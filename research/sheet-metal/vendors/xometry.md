# Xometry: what its public pages state (read 2026-11-22)

Notes behind `kernel/forge-vendor/data/xometry.toml`. Restated from the vendor's public pages;
not an official profile. Verify with the vendor before ordering. Inches; MT = material thickness.

## Pages read (HTTP 200)

- https://www.xometry.com/capabilities/sheet-metal-fabrication/
- https://www.xometry.com/capabilities/sheet-metal-fabrication/metal-bending-service/
- https://www.xometry.com/capabilities/sheet-cutting/
- https://www.xometry.com/capabilities/sheet-cutting/metal-laser-cutting/ (checked against the raw
  HTML)
- https://www.xometry.com/manufacturing-standards/
- https://www.xometry.com/sheet-cutting/standard-sheet-sizes/
- https://www.xometry.com/resources/machining/sheet-metal-mini-guide/
- https://www.xometry.com/resources/sheet/understanding-sheet-metal-tolerances/

Not read: the sheet metal design guide PDF, which sits behind a download form. No per-material
bend radius, K-factor or deduction is published as page text. The only bend rule is an inside
radius of at least 1 MT. That is recorded as a note, not as tooled radii, so the profile has no
bend rule. A sheet using it must give `r:` and `bend:`.

## Rules used

From the laser cutting page; where it says "whichever is smaller/greater", so does the profile:

| Rule | Value |
|---|---|
| hole to edge | min(2 MT, 0.125) |
| hole to hole | min(6 MT, 0.125) (held as `min_web`) |
| min slot | max(0.040, 1 MT) |
| min tab | max(0.063, 1 MT) |
| min feature | 0.5 MT |
| relief | max(0.010, 1 MT) |

The fabrication page gives "bend edge to hole 6x MT". The profile uses that as hole to bend.

## Pages that disagree

- Blank size: 5' x 10' on the fabrication page, 10' x 20' on the bending page. The profile uses
  the smaller, 60 x 120.
- Brake length: 14' vs 10'. The profile leaves max bend length unset.
- Tolerances: the profile quotes the Manufacturing Standards page: bend angle +/-1 deg, edge to
  bend +/-.015, cut edge to edge +/-.010.

## Hems and PEM

- Hems are flagged for a manual quote. The profile does not list `hem`.
- PEM insertion is offered, but no sizes are published. The profile lists `hardware` with no
  catalogue rows, so `pem` finds no part in it.

Checked in the page source (2026-11-22): the laser page's "Minimum Hole to Edge Distance = 2X MT
or 0.125", whichever is smaller" and "Minimum Hole to Hole Distance = 6X MT or 0.125", whichever is
smaller"; the fabrication page's structured data "Minimum Bend Edge to Hole Distance 6X MT" and
"Blank Size 5' x 10'"; the standard sizes page's thickness lists.
