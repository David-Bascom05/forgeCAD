# Machine spec research brief (forgeCAD machine library)

Goal: for each machine listed in your task, collect the maker's PUBLISHED specifications into one JSON
file per machine at  SCRATCH/specs/<vendor>-<model>.json  (lowercase, dashes). SCRATCH is given in the task.

Rules (binding):
- Numbers only from a page/PDF you actually fetched. Every machine records `source` = list of the exact URLs
  you read and `taken` = "2026-11-23". Prefer the maker's own site/catalog PDF; if you must use a dealer or
  third-party page, put it in `source_secondary` and say so in notes.
- Restate numbers; never copy prose. No vendor CAD files.
- Anything not published = null, and add the field name to `unknown` list. NEVER guess or fill from memory.
  If you know a value from memory but cannot find it published, leave null and say "memory says X, unverified"
  in notes.
- Do not spoof user agents or bypass bot filters. If a host refuses (403/406/429/timeout), record the exact host
  and status in `blocked_hosts` of a file SCRATCH/specs/_blocked-<yourgroup>.txt and try another public source
  (other regional maker site, catalog PDF on maker CDN, WebSearch result snippets are NOT a source by
  themselves — the page must be fetched).
- Units: convert to mm, kg, rpm, kW, N·m, m/min (rapids/feeds as published, also note original unit), deg,
  seconds. Keep `orig` strings for anything you converted from inches/hp/ipm (e.g. "40 in").
- When a model has variants (e.g. 12k/15k/20k spindle, 40/30 tool changer), record the STANDARD one in
  fields and the options in `options`.

JSON fields (use null when unpublished):
{
 "vendor": "", "model": "", "family": "", "source": [], "source_secondary": [], "taken": "2026-11-23",
 "kind": "vmc3|vmc4|hmc|table-table|head-table|head-head|lathe|lathe-c-live|lathe-y|mill-turn|swiss",
 "architecture": "free text: which unit carries each axis (e.g. 'X table on Y saddle, Z head on fixed column'; 'moving column X, Y head, Z table' etc.), rotary axes: about which machine axis they turn, which carries which (A cradle carries C table), tilt range, horizontal/vertical spindle",
 "control": {"standard": "", "options": []},
 "travel_mm": {"X":null,"Y":null,"Z":null, "W/Z2/B/...": null},
 "rotary": {"A": {"range_deg": [lo,hi], "rapid_rpm": null, "indexing_or_full": ""}, "B": null, "C": null},
 "spindle_nose_to_table_mm": [min,max], "spindle_center_to_column_mm": null,
 "spindle_center_to_pallet_top_mm": [min,max],
 "table_mm": [x,y], "table_load_kg": null, "rotary_table_dia_mm": null,
 "pallet": {"count": null, "size_mm": null, "change_s": null, "load_kg": null},
 "swing_over_bed_mm": null, "max_turning_dia_mm": null, "max_turning_length_mm": null,
 "chuck_size_in": null, "bar_capacity_mm": null, "spindle_nose_type": null,
 "spindle": {"rpm_max": null, "rpm_min": null, "taper_or_interface": "", "power_kw": {"cont":null,"30min":null,"peak/other":null},
             "torque_nm": {"cont":null,"max":null}, "curve_points": "rpm:kW pairs ONLY if a published chart/table gives numbers, else null"},
 "sub_spindle": null, "live_tool": {"rpm_max":null,"power_kw":null}, "milling_head": null,
 "turret_stations": null, "turret_index_s": null, "tailstock": null,
 "rapid_m_min": {"X":null,"Y":null,"Z":null,"A_or_B_rpm":null,"C_rpm":null},
 "max_cutting_feed_m_min": null,
 "accel_g_or_m_s2": null,
 "changer": {"pockets": null, "options": [], "tool_to_tool_s": null, "chip_to_chip_s": null,
             "max_tool_dia_mm": null, "max_tool_dia_empty_adjacent_mm": null, "max_tool_len_mm": null, "max_tool_kg": null},
 "coolant": {"through_spindle": null, "notes": ""},
 "overall_mm": {"W": null, "D": null, "H": null}, "weight_kg": null, "table_height_from_floor_mm": null,
 "unknown": [], "options": [], "notes": ""
}

Breadth first: get every listed model at least partly before deepening any one. Prefer the most common model
of each family. Final reply: a short table of what you wrote (file, % fields filled), the blocked hosts, and
anything surprising. Keep the reply under 400 words.
