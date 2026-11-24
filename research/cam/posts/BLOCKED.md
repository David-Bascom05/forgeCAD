# Post dialects we cannot restate (2026-11-23)

A dialect is restated only from the control maker's openly published programming manual (no login,
no sign-up; no third-party mirrors of the manuals). These three are refused with
`PostError::Unavailable` (forge-post `UNAVAILABLE`) and the reason; the research files beside this
one hold what is known, with sources.

| dialect | status | known (open sources) | missing |
|---|---|---|---|
| brother (CNC-C00/D00) | BLOCKED: the manual ships with the machine | 19 of 31 codes from Brother's public FAQ PDFs (download.brother.com/pub/jp/machine/pdf/faq/) and the CNC-D00 spec page: O-number programs, `( )` comments, N words, M30, G54-G59 and G54.1 P1-48, G28/G30, 0.001 mm increment, G43 H, M8; examples show `G100 T_ ... G43 H_ Z_` (M-series tool change) and G77 / G133 / G134 tapping | M3/M4/M5, M9, G2/G3 arc format, dwell, G81-G85 / G73 cycles, G21/G40/G49/G80, the meaning of a number without a decimal point |
| mazak-eia (SmoothG/X/Ai EIA/ISO) | BLOCKED: manuals behind the MPower login | that the control runs EIA/ISO G-code beside MAZATROL (a Mazak blog page) | every code |
| okuma-osp (OSP-P300/P500) | BLOCKED: manuals through distributors / on the control | nothing (the open OSP white paper has no syntax) | every code (G15 H work offsets, G56 length comp: memory only, unverified) |

Hosts: research/cam/posts/_blocked-jp.txt. Library machines with these controls (data/machines:
control = "brother" / "mazak-eia" / "okuma-osp") get the refusal when posting is asked for.

| syntec | BLOCKED: manuals need a syntecclub.com login | nothing | every code |

## Dialects restated (forge-post data/*.toml, every code cited)
siemens (SINUMERIK ISO Dialect, G291; Siemens ISO Milling 04.2007 + Fundamentals 03/2013),
heidenhain (TNC 640 ISO, .I; ISO Programming 10/2023 + Machining Cycles 10/2022), pathpilot
(tormach.com pages + post guidelines), mach3 (Using Mach3Mill 1.84), mach4 (Mach4 Mill G-code
Programming Guide v1.1), centroid-cnc12 (CNC12 v4.12 mill operator's manual), masso
(docs.masso.com.au). The research files beside this one hold each item with its page.

## Finding: peck and chip-break retracts are control parameters
forge-post's cycle model fixes the G83 re-approach clearance (0.5 mm) and the G73 lift (0.2 mm)
(PECK_CLEAR, CHIP_LIFT). The controls set them otherwise: Siemens GUD _ZSFR[10] / _ZSFR[1] (0 means
1 mm), Mach4 parameters 5115 / 5114 (0.25 / 0.6 mm default), Centroid G10 P83 / P73, Mach3 a
Settings DRO, PathPilot G73 a fixed 0.010 in, MASSO G73 a fixed 1.0 mm. The new dialects therefore
post G83/G73 as explicit moves (the simulated path is the machine's). fanuc, haas and linuxcnc
(existing, unchanged) still write G83/G73 with the model's values: their simulation can differ
from the machine by those retracts.
