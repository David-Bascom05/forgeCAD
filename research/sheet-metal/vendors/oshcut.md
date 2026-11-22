# OSH Cut: what its public pages state (read 2026-11-22)

Notes behind `kernel/forge-vendor/data/oshcut.toml`. Restated from the vendor's public pages;
not an official profile. Verify with the vendor before ordering. Inches.

Pages read (HTTP 200):
- https://www.oshcut.com/materialdetails/aluminum-5052-h32
- https://www.oshcut.com/materialdetails/aluminum-6061-t6
- https://www.oshcut.com/materialdetails/steel-a1008-cr
- https://www.oshcut.com/materialdetails/stainless-steel-304-2b-finish
- https://www.oshcut.com/services/bending
- https://www.oshcut.com/design-guide/metal-bending-design-rules
- https://www.oshcut.com/design-guide/brake-sheet-metal-tooling
- https://www.oshcut.com/minimum-maximum-material-sizes
- https://www.oshcut.com/capabilities
- https://www.oshcut.com/faq
- the services/hardware-insertion, /metal-tapping and /countersinking pages

What each per-thickness page gives:
- the max part size;
- a minimum hole, and a minimum *recommended* hole;
- a minimum recommended channel and wall;
- the stock thickness tolerance;
- the services offered at that thickness.

The profile uses the minimum hole as `min_hole`. It uses the recommended wall as `min_web` and
the recommended channel as `min_slot`.

Not read, and so left out of the profile:
- **Bend radii, K-factor and minimum flange per material.** OSH Cut publishes these in its web
  app's catalog (https://app.oshcut.com/catalog). That page needs JavaScript; its data endpoints
  were not fetched. The profile has no bend data. The only generic statement is "K around 0.4
  for soft metals like aluminum, around 0.45 for steel and stainless", which is not a per-stock
  figure, so it is not used.
- **The brake tooling page's flange-length charts** are images.
- **Hems** are not mentioned. No min/max bend angle is published. Bending is on, and its
  tolerance is quoted as +/-1 deg.
- **6061-T6:** no thickness lists bending.
