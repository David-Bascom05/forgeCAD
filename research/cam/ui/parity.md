# The Manufacture workspace against Fusion: the UI gap list (2026-11-24)

Source of the bar: research/cam/toolpath-quality/fusion-benchmark.md (branch
research/toolpath-quality, 69 items) and research/cam/fusion-workflow-2d-3d.md. Only the items a
user meets in the workspace are listed (dialogs, libraries, programs, sheet, time, simulation);
the path behaviour itself (R, S items) is the generators'. Each row says where we are and who
closes it:

- **done**: in the workspace now (cam/ui-3).
- **UI**: the workspace can close it alone; **UI next**: planned here, in order.
- **LANG**: needs an argument or built-in in the machining language (forge-cam-lang, CAM-CORE)
  first; the form then gets its field (schema.rs, one row each).
- **SIM**: the simulation view's (CAM-SIM). **QUAL**: the quality harness's (TOOLPATH-QUALITY).

## Op dialogs (tabs and parameters)

| Fusion | here | state |
|---|---|---|
| Tool / Geometry / Heights / Passes / Linking tabs | the same five, data-driven from schema.rs, every op (Linking always) | done |
| Tool tab: tool picker with its geometry | document tools, type, D, flutes, flute length; Tool Library… | done |
| Tool tab: feeds and speeds shown (rpm, vc, feed, fz) | from the run's cutting data, with thinned chip | done |
| Tool tab: override rpm ⇄ vc, feed ⇄ fz; ramp / plunge / lead / transition feeds; feed per rev (drilling) (F1) | Feeds & Speeds tab on every milling op (branch cam/ui-feeds, on FEEDS-SPEEDS' feeds/model): vc, rpm, chip, fz, feed, plunge/ramp/lead-in feeds, limits; each left out shows its resolved value and source; derived chip, engagement, power, torque and the checks | done on cam/ui-feeds (lands with feeds/model); drilling feed per rev: not yet |
| Named cutting-data presets per tool, expressions of tool geometry (F2) | cutting groups P M K N S H, no presets | LANG + library (forge-tooling `[[preset]]`) |
| Presets filtered by stock material (F3) | the part's `group:` picks the cutting data | partly (group); presets LANG |
| Geometry: pick model edges / faces / holes / sketches | holes and sketches from history or picked in the view | done (holes, sketches); edge chains LANG |
| Heights: clearance, retract, feed, top, bottom — each a reference + offset | setup clearance and retract (setup dialog Heights); op top/bottom (`from:`/`to:`) as numbers | partly; references (`to: f.plate.end`) LANG |
| Passes: multiple depths / max stepdown | `step:` | done |
| Passes: stock to leave radial and axial, negative | `leave:` (one value) | partly; axial/radial LANG |
| Passes: finishing passes, spring pass, even stepdowns | none | LANG |
| Passes: direction climb / conventional / both ways (R4, R5) | none (climb by construction) | LANG |
| Passes: tolerance, smoothing (arc fit) (S17) | setup `tol:`; arcs posted where the path is arcs | partly |
| Linking: retraction policy, high-feed mode, stay-down, lift height, safe distance (L1–L5, L11) | retract to the setup's retract height; Linking tab says so and opens Setup Heights | partly; policies LANG |
| Linking: leads (horizontal / vertical radii) (L6) | none | LANG |
| Linking: ramp type and angle (L9, R16, R17) | helical `ramp:` angle on clear/pocket | partly; other types LANG |
| Feed optimisation at corners (F4) | none | LANG (generator) |
| Templates (save op as template, insert) | none | UI next: an op's text copied into a template file and inserted by AddFeature |

## Setups

| Fusion | here | state |
|---|---|---|
| Machine, WCS orientation and origin, stock modes, fixture | machine, origin (stock top centre/corner, part origin, point), work offset, `at:`, fixture (soft jaws, vise), `from:` (turned over), stock box+offsets or fixed box | done |
| Stock: cylinder, tube, from solid, relative/fixed | box only | LANG (CAM-CORE) |
| WCS origin picked in the view | numbers / named choices | UI next (needs a pick → part-frame point; anchors give names, not points) |
| Probe WCS | none | LANG |
| Lathe setup: chuck, bar, Z0, machine | done (lathe dialog) | done |

## Tool library

| Fusion | here | state |
|---|---|---|
| Browse built-in + document + project libraries, filter | built-in + project `cam/*.toml`, filter; lathe tools for turning | done |
| Add a library tool to the document with a T number | Add → `tool(..)` / `lathe_tool(..)` at the next free T | done |
| Import vendor / Fusion JSON libraries | Tool Library ▸ Import: a Fusion JSON beside the document → `cam/<name>.toml` (forge-cam-import), library reloaded | done |
| Edit a tool's geometry, holders, assemblies | read-only | UI (writes the project TOML) — later |
| Feeds presets per tool | none | LANG + library |

## NC programs, posting, sheet

| Fusion | here | state |
|---|---|---|
| NC program manager: list, which setups, post, number, output folder | NC Programs window: every program, its setups (checks), dialect, number, lines, certification; file name, folder, extension, comment, tool list (statement args); Post / Export… / Re-post Stale; posted/stale per program (nc-export.md) | done |
| NC program: ops selection, one file per setup, post options (units, decimals, line numbers) | whole setups; one program per setup | LANG (`ops:`) + forge-post args (nc-export.md) |
| Post to a folder / machine | Export… (native dialog; browser save picker or download, zip for several); send-to-machine designed (nc-export.md) | done (export); send: design |
| Post preview | NC text in the post dialog | done |
| Setup sheet (HTML, Excel) | HTML written beside the document; turning: a simple table | done (HTML); Excel not planned |
| Machining time per op and per setup, feed scale (O4) | per op; per setup in the browser row and its details; the programs' machine-timeline time in the Quality window | done (feed scale: not yet) |
| Jerk-limited time, consistent everywhere (O5, O6) | — | QUAL (forge-cam-quality t_stop) shown once wired |

## Simulation and verification

| Fusion | here | state |
|---|---|---|
| Play / step / scrub, tool and holder, stock | CAM-SIM's hosted view | SIM |
| Collisions and gouges marked | certified checks per op, witness located, Check Now (`Cam::check`) | done (BETTER: certified) |
| Stock comparison colouring | — | SIM |
| Toolpath coloured by feed / engagement | — | **UI next with QUAL + SIM**: per-move chip load from forge-cam-quality, a colour toggle the sim view reads from `Focus` |
| Per-op statistics (air cutting, plunges, retracts, engagement, chip load, power) | measured on every milling Generate (forge-cam-quality `measure_cam`): per op in the browser's details, the Quality window (every op, each setup's feed time and air share, the programs' machine-timeline time) | done (BETTER) |
| Turning section view | done (turnview.rs) | done |

## Order of work (this branch)

1. Per-setup cycle-time summary (from the run; QUAL's machine time once wired).
2. Programs window (NC program manager).
3. Tool library: Import Fusion JSON.
4. Quality panel per op and per setup (forge-cam-quality's `measure_cam`), then chip-load colouring
   once its per-move values and CAM-SIM's colour hook exist.
5. LANG rows go to CAM-CORE as one list (the arguments, their defaults, the check each needs).
