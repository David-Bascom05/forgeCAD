# Machine library, worker B (industrial vendors), 2026-11-23

56 machines in kernel/forge-machine/data/machines/<vendor>.toml (14 vendor files), each body a
conservative stand-in part under kernel/forge-machine/parts/<vendor>/<model>/. Every number is
from the maker's page or PDF named in the record's `source` (taken 2026-11-23), restated; where only a
dealer/importer page had it, the note says so. The research restatements (one JSON per machine,
with every UNKNOWN listed) are in b/specs/; the generator that turns them into records is in b/gen/
(conv.py: published -> field, unpublished geometry -> named estimate, unpublished check value ->
vacuous value named UNKNOWN; a machine whose travels, rpm or tilt range are unpublished is left out).

Tests: interface/forge-machine-lang/tests/vendors.rs (every file parses and validates, every part
builds and lies inside its envelope, every machine is clear at home and with each joint at each end
of travel); tests/vendor_jobs.rs (a sample job per added architecture: A/C trunnion 3+2 on the Brother
M200X3, HMC B-index face on the Makino a51nx; tip position in the part frame and no contact but the
cutting edge in the stock).

## Machines

| vendor | machines | kinds |
|---|---|---|
| Brother | SPEEDIO S300X2, S500X2, S700X2, M140X2, M200X3 | vmc3, table-table (A/C) |
| FANUC | ROBODRILL alpha-D21MiB5, alpha-D21LiB5 | vmc3 |
| Mazak | VCN-530C L, VCN-700, VARIAXIS i-600, i-700, HCN-5000, INTEGREX i-200, QUICK TURN 200 | vmc3, table-table, HMC (vmc4), mill-turn, lathe |
| DN Solutions | DNM 4500/5700/6700, DVF 5000, NHP 4000/5000, Lynx 2100A/LA/LMA, Puma 2600, 2600Y II | vmc3, table-table (B/C), HMC, lathe |
| Hyundai WIA | KF5600, F500PLUS, KF5, HS4000 II, HS5000 II, SE2200, SE2600 | vmc3, HMC, lathe |
| Makino | a51nx, a61nx, D500 | HMC, table-table |
| Matsuura | MX-330, MX-520 | table-table |
| Kitamura | Mycenter-3XV, HX400iG | vmc3, HMC |
| Okuma | GENOS M560-V, MB-46VII | vmc3 |
| DMG MORI | DMU 50 3rd gen, NHX 4000, NHX 5000, NLX 2500/700, CLX 350 | table-table, HMC, lathe |
| Hermle | C 22 U, C 32 U, C 42 U | table-table (A/C) |
| Romi | D 800, DCM 620-5X, GL 250 | vmc3, table-table, lathe |
| Bridgeport (Hardinge) | V480 APC, V710 | vmc3 |
| Spinner | U5-630, TC600 | table-table, lathe |

## Left out (defining numbers unpublished on reachable pages; specs in b/specs/)
DMG MORI CMX 600 V / 1100 V (no spindle rpm), DMU 65 monoBLOCK and DN DVF 5000 2nd gen, Okuma
MU-5000V, Grob G350, Kern Micro HD, Hyundai WIA XF6300, Chiron FZ 16 S / MILL 800 (no tilt range or
travels), Okuma GENOS L3000-e, LB3000 EX III, MB-4000H, MULTUS B250II (no travels), Makino PS95,
Hardinge GS 200, T42 SP (no data), Bridgeport Series I (knee mill), Nakamura-Tome NTRX-300,
WT-150II, Willemin-Macodel 408MT / 508MT2, Puma 2600SY II, Mazak QT-250MY (mill-turn and twin-spindle
lathes: format gaps B12 and the multi-channel turret machines; next round).

## Findings
- Dialects named by these records but not in forge-post: `fanuc-lathe` (CAM-TURN's), `heidenhain`,
  `siemens`, `okuma-osp`, `mazak-eia` (+ `-lathe`), `brother`. forge-post has fanuc, haas,
  linuxcnc, grbl. Records whose control is unpublished say `unknown`.
- Format gaps B1-B13 in FORMAT-GAPS.md (APC, HMC table-side Z, live-tool lathes, unpublished
  required fields, power ratings, …).
- Makers disagree with themselves (Mazak CAT40 vs BT40 and 12k vs 18k on one model; Okuma EU vs JP
  ratings; DMG MORI en vs us counter-spindle power): the spec page is used and the conflict noted.
- Gated data: Okuma, DMG MORI, DN Solutions and Chiron put full technical data behind a sign-up;
  not used. Asking David whether to register is his call.
- Blocked or dead hosts: b/specs/_blocked-*.txt (exact host and status).
