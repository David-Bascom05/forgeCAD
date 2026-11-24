# Hurco machines in forge-machine

GENERATED: `kernel/forge-machine/data/machines/hurco.toml` and `parts/hurco/` come from
`tools/machines/hurco.py`. Source: Hurco's public line cards (first party, cms.hurco.com, linked
from hurco.com's product pages; read 2026-11-23): VM, VMX (24-50 in X), SRT/SW, TM. The figures
used are restated in the generator's tables, model by model.

| records | kind | |
|---|---|---|
| hurco-vm-one, -vm10i, -vm10hsi, -vm15di, -vm20i, -vm30i | vmc3 | |
| hurco-vmx24i, -vmx30i, -vmx42i, -vmx50i, -vmx50i-50t | vmc3 | |
| hurco-vmx42srti, -vmx42hsrti, -vmx60srti | head-table | B swivel head (+/-92 deg) + C rotary table |
| hurco-tm6i, -tm8i, -tm10i, -tm12i | lathe | 12-station slotted-disc turret |

Not modelled: SW models (B head only, no C: no format kind fits a single head rotary,
FORMAT-GAPS #10), VMX60-84 (second VMX card), VMXD, VC/U 5-axis, TMM/TMX/TMMY mill-turns, BX,
DCX, HBMX.

Published and cited: travels, table, nose-to-table, rapids and programmable feed, max rpm, the
30-min/continuous power and torque at rpm, tool type, ATC capacity, tool diameter/length/weight,
tool-to-tool time, B/C ranges and rpm, rotary table diameter, swing, turning diameter and
length, chuck, turret stations.

The cards contradict themselves in places, recorded as `conflict`: VM20i X "40 in / 1,106 mm"
(held: 1,016), VMX50i-50T peak torque "294.4 ft. lbs (217.2 Nm)", TM8i torque "105.65 ft. lbs
(43.2 Nm)". Power curves therefore use only the continuous power at its rpm (never above it).
Tool-to-tool time stands in for chip-to-chip (errs short). Estimated or unknown: layout, home
placement, the SRT B pivot length and rotary-table position, head and turret sizes, lathe feed
limits, minimum rpm. Control: WinMax's "Industry Standard NC" → the `fanuc` dialect (estimate).
