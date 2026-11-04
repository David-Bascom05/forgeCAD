# Third-party material in the standard parts library

Our tables (`*.csv` here) are facts keyed by hand from the sources each names, released CC0-1.0.
The files under `third_party/` are copies of other projects' tables, kept only so
`tests/stdparts.rs` can compare ours with them; `tools/stdparts/import.py` writes them.

- `third_party/bd_warehouse/`: from bd_warehouse (https://github.com/gumyr/bd_warehouse), commit
  5a8a439, `src/bd_warehouse/data/*.csv`. Copyright 2024 Gumyr. Licensed under the Apache
  License, Version 2.0 (`third_party/bd_warehouse/LICENSE`). Modified: only the columns our
  tables compare are kept, renamed to our column names, sizes respelled (`M6-1` as `M6`), inch
  fractions written as decimals.

Consulted for comparison only, never copied: FreeCAD Fasteners Workbench
(https://github.com/shaise/FreeCAD_FastenersWB, GPL-2.0-or-later). Its findings appear only as
rows in `resolutions/` naming where it differed.
