# Part colours

`palette.py` picks the 12 pastels in `interface/forge-render/src/palette.rs` (Pastel Parts, View
tab): one OKLCH lightness (0.80) and chroma, hues spread over the circle less 200-250 degrees (the
preselect blue and the light selection blues), the chroma and starting hue that keep the least
CIEDE2000 difference largest, both between entries and against every colour the app draws marks,
sketches, the origin, joint glyphs, interference and warnings in. The list is ordered five hues
apart so neighbours differ. Rerun: `python3 palette.py` (no dependencies); it prints the table.

Result: least ΔE00 between any two entries 7.3, to any UI colour 8.8 (the light-text blue of a
chosen browser row; every 3D mark colour is further). `GREY` (0.706) is Inventor's default light
neutral grey.
