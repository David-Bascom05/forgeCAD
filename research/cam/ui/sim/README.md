# CAM simulation views (CAM-SIM)

Branch `cam/sim`. The toolpath, stock and machine simulation of a machining document, in the
Manufacture workspace, native and in the browser. Everything is driven by the real run
(`forge_cam_lang::run`) and, for the machine, by the posted text on `forge-machine`.

## What it shows

| piece | where | what |
|---|---|---|
| timeline | `camsim/timeline.rs` | every **posted** move (`OpOut::posted`: what the certified checks ran on), each with its time: feeds as programmed, rapids at forge-cam-lang's rate, canned cycles (pecks included) as their motion; the tool's place at any time; a check's move found on it |
| stock | `camsim/zmap.rs` | a display-only dexel grid: one material interval per column, swept exactly for moves at one height (lines, arcs), stamped for the rest; a setup turned over continues from the last setup's stock; checkpoints make any scrub a few ms (4.1 ms release on the flange, 160k columns) |
| comparison | `zmap::compare` | left on / within tolerance / below the part, labelled a preview: within a cell of a wall the grid can't tell, so no column there is called below the part |
| verdict | `Sim::markers` | the certified checks that did not pass and G4's exact leftover regions, as markers on the timeline and in the view (click to go there); the panel's verdict line is the run's `certification()` |
| machine | `camsim/machine.rs`, `forge-cam-lang/src/sim.rs` | the setup's posted program interpreted on its machine (forge-machine), timed (`t_stop`, with `t_min`), every body checked through the motion; played as the simulation's source, so the stock and the toolpath follow what the posted text makes the machine do; the machine's bodies posed about the part; overtravel, feed clamps and collisions as markers |
| chrome | `gui_camsim_panel.rs` | timeline bar (ops as bands, setups, markers, playhead, play/pause/step, speed, follow tool, stop at issues), panel tabs Program (setups and ops, visibility, stock after an op, Play: toolpath or posted code on the machine), Display, Checks, Moves |
| glue | `gui_camsim.rs` | the workspace's run simulated off the window's thread; its `Focus` followed (setup, op, only selected, a located check, stock after an op); `CamSimUi::cursor()` published for its browser; `Gui::camsim_command` words for hosts and tests |

## Driving it

`Gui::camsim_command` (the page: `window.forgecad.fc.app_camsim(words)`, state in
`state().camsim`): `t=S`, `at=F`, `play`, `pause`, `speed=X`, `op=N`, `marker=N`,
`locate=OP,PATH,MOVE,S`, `source=toolpath|machine`, `colour=material|compare`,
`part=hidden|ghost|solid`, `tab=program|display|checks|moves`, `follow=0|1`, `rapids=0|1`,
`played=0|1`, `stock=0|1`, `tool=0|1`, `machine=0|1`, `cells=N`, `fit=stock|machine`.

## Tests

- `interface/forge-render/tests/camsim.rs` (headless, on `examples/machined`): timeline continuity
  and arcs (a posted arc's radius blended from its start's to its end's, as forge-machine does),
  no gouge-class column on the certified flange, scrub back and forth gives the same stock, slot
  and ball-groove depths, cutters from the tools' profiles, a refused op's checks as markers on
  their moves, the flange's posted program on generic-vmc3 placed by its `at:`, per-move chip
  load from forge-cam-quality laid on the posted moves.
- `interface/forge-render/tests/camsim_ui.rs` (native app, headless): the flange simulated in the
  app at 1400×860, 1600×1000 and 1024×700; a turning document gets no milling simulation.
  `FORGE_UI_SHOTS=DIR` keeps the `native-*` set (here). Without a GPU, Mesa's lavapipe:
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`.
- `interface/forge-render/web/tests/camsim.cjs`: the same in the browser (single-threaded build,
  SwiftShader WebGPU), plus play and a refused document; `FORGE_WEB_SHOTS=DIR` keeps the `web-*`
  set (here).

The review set (STANDARD §10/§11): `native-sim-*` and `web-sim-*`, each at 1400x860, 1600x1000
and 1024x700: 01 start, 02 the contour (selected: blue), 03 the end compared with the part, 04 the
move list, 05 the posted code on generic-vmc3, 06 a turning document (no milling simulation),
07 a refused op's checks (web).

## Review against docs/ux/STANDARD.md (UX-LEAD, cam/ui-3 f15eb2c8)

| item | what changed |
|---|---|
| P0 lathe doc on the web showed milling errors | that branch carried an old copy of the glue without the turning skip; the skip now keys on the run's own process (`Run::process`) as well as the text, and a simulation finished for a document that is now turning is dropped. Tested native and web (`*-06-turning-no-sim`) |
| P0 errors as parser text | the verdict says "the document doesn't run (N problems)"; one line per problem, `Line 7` then what, the accepted-names list left to the hover, the first three shown and the rest behind "N more". forge-cam-lang's errors carry no typed fixes yet, so there is no fix button (CAM-CORE's to add) |
| P1 header | a fold chevron and the tabs as one segmented control |
| P1 Play: radio run-on | segmented `Toolpath \| Posted code`, the program, dialect and machine as the line beneath; a run that can't be made is drawn off with its reason on hover |
| P1 op rows | a right-aligned time column in words (`1 min 17 s`); "after" is a stock icon button, "Show the stock after this op" |
| P1 transport | painted icons (start, back, play/pause, on, end) with their keys in the tooltips; Space, ←/→, Home/End (not while a text field has the keyboard). To move to the shared toolkit's transport when UX-LEAD's lands |
| P1 appearances | stock in RENDER-STUDIO's "Aluminium, machined", holder and workholding "Steel, machined" (their F0 at the overlay's diffuse share), cutter and shank a carbide grey (stand-in: the library has no carbide); blue only for the selected op's path (cuts teal, material left violet, rubbing violet) |
| P2 framing | the stock and the tool's tip at 70 % of the shorter side of the view not under the panel and the timeline, centred there, from the 3/4 view |
| also | nothing under 11 px; the playing readout and machine lines in words with units, cut short with the whole on hover |

Not done here: the panel floats over the view's right rather than docking full height (the
shell's layout, STANDARD §1); the transport and appearances move to the shared toolkit and the
appearance library when they land.

## Found by the machine simulation (flange_cam.fg on generic-vmc3)

The document gives no `at:` and no workholding, so the stock's placement is **assumed** (centred
on the table datum, its bottom on it) and the UI says so. With that:

1. The contour goes to z −0.5 but the stock's bottom is z 0: the cutter enters the table by
   0.5 mm (table vs T1 cutter, from line 276). The CAM checks don't model the table without
   workholding.
2. `T12 M06` (N3890) has no Z retract before it: the machine loads the longer T12 at T1's height
   and its first rapid (N3930, line 394) starts 21 mm inside the stock and runs through the boss.

Both assigned (coordinator): CAM-CORE (G2 with no workholding, retract before M06),
CAM-MACHINE (M06 semantics per machine). Both since fixed: the flange's stock now stands 3 mm
under the part (`below: 3`), the setup is placed (`at: (0, 0, -250)`) and Z retracts before
every tool change; its posted code now runs on generic-vmc3 with no contact and no limit.
