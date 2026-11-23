# Post-dialect research brief (forgeCAD forge-post)

Goal: for each control dialect in your task, find the control maker's OPENLY PUBLISHED programming
manual (no registration, no login, no sign-up; skip gated documents) and restate, per item below,
what the control accepts, with the exact source (URL + manual title/edition + page or section) for
EACH item. Write one JSON file per dialect: SCRATCH/posts/<dialect>.json.

Rules (binding):
- Only from documents/pages you actually fetched. Restate; never copy prose. No user-agent spoofing.
- Item not found in an open source = null with "UNKNOWN" (it will be refused by the post, never guessed).
  Memory may be noted as "memory says X, unverified", never used as a value.
- A third-party page (forum, dealer, CAM vendor post) is NOT a source for a code; note it only.
- If a host refuses or a document is gated, record host + status in SCRATCH/posts/_blocked-<group>.txt.

Items (mill, 3-axis, metric; the forge-post dialect keys):
program_open / program_close (program start/name line, end code M30/M2/M02, % marks),
comment syntax (parentheses? semicolon to end of line? both?), line numbers (N words; allowed/required; max),
block end (newline; or `*` like Heidenhain ISO; or `;`),
safe_start codes (absolute G90, feed per min G94, plane G17, metric G21 or G71, cutter comp cancel G40,
length comp cancel G49, cycle cancel G80; and any mode switch needed e.g. Siemens G291 ISO mode),
tool change (T word, M6, D word?), tool length comp (G43 H? G56 H? automatic with T?),
spindle (S M3/M4/M5), work offset (G54..G59 count and extended; Okuma G15 H?), arc format (G2/G3 with
I J K incremental? absolute? R?), rapid/linear (G0/G1), feed word F (mm/min), decimal point rules
(is "X10" 10 mm or 10 um without a point?), max digits/decimals,
retract to machine home (G28? G53? G91 G28 Z0? SUPA?),
coolant (flood M8, mist M7, through-spindle code?, off M9),
dwell (G4 with P ms? P s? X s? F s? units),
canned cycles: drill G81, dwell drill G82, peck G83, chip-break G73, rigid tap G84 (and what F means),
bore/ream G85, return level G98/G99, cycle cancel G80; the word for peck depth (Q), dwell in cycles (P units).
Also: the control's native (non-ISO) language if any and whether ISO/G-code mode exists, how it is
selected, and what an ISO program must start with.

JSON shape: {"dialect": "...", "control": "...", "sources": [{"id": "S1", "url": "...", "title": "...",
"edition": "...", "taken": "2026-11-23"}], "items": {"<item>": {"value": "...", "source": "S1", "where":
"page/section", "note": ""}}, "unknown": [...], "notes": ""}

Final reply: a short table (dialect, source documents, items filled / total, blocked), and surprises.
Under 400 words.
