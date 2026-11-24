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
  and arcs, no gouge-class column on the certified flange, scrub back and forth gives the same
  stock, slot and ball-groove depths, cutters from the tools' profiles, a refused op's checks as
  markers on their moves, the flange's posted program on generic-vmc3.
- `interface/forge-render/web/tests/camsim.cjs`: the same in the browser (single-threaded build,
  SwiftShader WebGPU); `FORGE_WEB_SHOTS=DIR` keeps the frames (the pictures here).

## Found by the machine simulation (flange_cam.fg on generic-vmc3)

The document gives no `at:` and no workholding, so the stock's placement is **assumed** (centred
on the table datum, its bottom on it) and the UI says so. With that:

1. The contour goes to z −0.5 but the stock's bottom is z 0: the cutter enters the table by
   0.5 mm (table vs T1 cutter, from line 276). The CAM checks don't model the table without
   workholding.
2. `T12 M06` (N3890) has no Z retract before it: the machine loads the longer T12 at T1's height
   and its first rapid (N3930, line 394) starts 21 mm inside the stock and runs through the boss.

Both assigned (coordinator): CAM-CORE (G2 with no workholding, retract before M06),
CAM-MACHINE (M06 semantics per machine).
