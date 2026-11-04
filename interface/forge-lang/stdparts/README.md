# Standard parts: dimension tables

One table per family (`iso4762.csv`, `iso4032.csv`, ..), in our own format, keyed by hand from the
sources each file names. The generators (`gen/*.fg`) build a part from one row; `std(..)` in an
assembly and `fg std` pick the row. Values are facts re-keyed from standards and catalogues and are
released CC0-1.0; the bundled third-party copies used only to cross-check them keep their own
licences (`third_party/`, THIRD-PARTY.md).

## Format

```
# title: Hexagon socket head cap screws
# standard: ISO 4762:2004
# kind: screw
# units: mm
# lengths: 2.5 3 4 5 6 8 10 12 16 20 25 30 35 40 45 50 55 60 65 70 80 90 100 110 120 ...
# src A: ISO 4762:2004 Table 1, iTeh preview pages 4-5 (https://cdn.standards.iteh.ai/...), keyed 2026-11-04
# src A97: ISO 4762:1997 Table 1 (continued), iTeh preview page 4, for sizes the 2004 preview leaves out
# src B: bd_warehouse 5a8a439 socket_head_cap_parameters.csv (Apache-2.0), third_party/bd_warehouse
# chk F: FreeCAD FastenersWB 2026-09-10 iso4762def.csv (GPL-2.0-or-later: consulted, never copied)
# model: nominal where the standard gives one, else the max-material limit: dk_max, k_max, s, t_min
size,P,b,dk_max,dk_min,k_max,k_min,s,t_min,l_min,l_max,l_full,src,checked
M6,1,24,10,9.78,6,5.7,5,3,10,60,30,A,A B F
```

- Lines starting `#` are the header: `key: value`, one per line. `title`, `standard` (with its
  edition), `kind`, `units` (`mm` or `in`) and at least one `src` are required. `src X:` names a
  source the values come from; `chk X:` a source that was only compared against (GPL data, vendor
  pages that may not be bundled). Each says the document, edition, table, page or URL, and when.
- The first other line is the column names; every row has every column; an empty cell means the
  standard gives no value for that size.
- `size` is the designation as the standard writes it: `M6`, `M6x0.75` (fine), `#10-24`, `1/4-20`,
  `6200`, `AS568-012`. Unique in the file.
- Columns are the standard's own symbols (ISO 225 for fasteners: `dk k s t e da ds dw w r b lf v m
  d1 d2 h`), suffixed `_min` / `_max` for a limit; a bare symbol is the nominal or reference value.
  Key every dimension the standard's table gives, limits too: the generator picks what it needs.
- Units are the header's (`in` tables are in inches, as the ASME tables print them; decimals, no
  fractions).
- Lengths (screws, pins): the header's `lengths` is the standard's series of nominal lengths;
  `l_min`, `l_max` per row bound the commercial range for the size (the thick stepped lines);
  `l_full`, where the standard has one, is the longest length threaded to the head (the dashed
  stepped line). Above it the thread is `b` long (ISO 888: lg max = l - b, ls min = lg max - 5P).
- `src`: the source letters the row's values were keyed from. `checked`: every source the row
  was compared with and agrees with (after the resolutions below). A family is cross-checked when
  each row has at least two letters in `checked`.

## Imported sources

A header line `# map B: FILE | size=COLUMN | ours=theirs | ours=theirs ..` says how a bundleable
source's table lines up with ours: `tools/stdparts/import.py` reads it, copies just those columns
of FILE into `third_party/<source>/<family>.csv` (renamed to ours, sizes in our spelling: `M6-1`
becomes `M6`, or `M8x1` when the pitch is not the row's `P`; inch fractions become decimals) and
prints every difference. `size=` names their column holding the designation (default `Size`).

## Disagreements

`resolutions/<family>.csv` records every value where a source differs from ours and which is right:

```
family,size,column,source,theirs,ours,verdict
iso7089,M12,h,B,2.7,2.5,ours: ISO 7089:2000 Table 1 gives h nom 2.5 (2.3..2.8); B keyed the max-ish DIN 125 value
```

`tests/stdparts.rs` compares every bundled third-party table against ours and fails on a
difference that has no row here. Comparisons against `chk` sources are made by
`tools/stdparts/check_*.py` (run by hand, the sources are not in the repo); their findings are
rows here too.

## Vendor parts (McMaster-Carr, PEM, Misumi, Bossard ..)

Vendor CAD is never bundled, fetched or cached by ForgeCAD: the vendors' terms forbid redistribution,
and McMaster-Carr's forbid any automated access (`forgecad-gate/stdparts/sources.md`). The path is
the user's own download:

1. Download the STEP file yourself, signed in to the vendor's site in your browser.
2. Put it in a `vendor-parts/` folder beside your assembly. That folder name is ignored by git
   (`.gitignore`), so the file stays on your machine and is never committed or pushed.
3. Make a part file beside it that imports the solid and says what it is for the BOM, with the
   anchors you will joint to:

   ```
   body = import("91290A115.step", op: "new")
   properties(part_number: "McMaster-Carr 91290A115", description: "Socket head screw M6x20",
              vendor: "McMaster-Carr", material: "ISO 898-1 class 12.9")
   head = anchor(xy)   # a plane of the vendor file's own frame; pick faces for others
   ```

4. Place it like any part: `scr = part("vendor-parts/mcmaster_91290a115.fg")`.

Prefer the library's own part (`std(..)`) where one exists: it is generated from the standard's
table, with its thread, anchors and grade.
