# Bend Simulation environment: data, shots

- `data/`: the research notes every library value is restated from (one cite per value: URL,
  date read, the source's statement). `vendor-tooling.md` (Trumpf, Bystronic, Amada),
  `maker-tooling.md` (WILA, Wilson Tool, Mate, Rolleri), `press-brakes.md` (32 press brakes).
  The library TOML (kernel/forge-brake/data/) is generated from them by tools/bendsim/
  md2toml.py and press2toml.py.
- `shots/native-*.png`: interface/forge-render/tests/bendsim_env.rs on the view's renderer
  (Vulkan, lavapipe), `BENDSIM_SHOTS=DIR`:
  - 00: a 100 x 200 channel on a Trumpf TruBend 5130 with Trumpf's own punches and dies, the
    default view and the whole press (C-frame housings from the published throat, ram, bed);
  - 01-04: approach, first bend, the operator's handling between steps, second bend;
  - 05: a 40 mm U with 70 mm flanges on a Bystronic Xpert Pro 100/3100, the straight P6 punch
    only: the second flange closes on the punch body at phi 45 deg (certified witness), the
    punch and the flange drawn red.
- `shots/web-*.png`: interface/forge-render/web/tests/bendsim.cjs in Chromium (software WebGPU),
  the wasm32 build: the same channel on the library's first press with its maker's tools
  (TruBend 5085), at the start, the first bend and the second.
- `band-27900c27.txt`: tools/speed/band.py, main f04c533e vs bendsim/env 27900c27, exact
  callgrind on one machine: every strict line <= +0.0000%, strict total -0.0085%; bits.txt
  unchanged.

## Gaps, stated

- No maker publishes clamp heights, upper/lower beam offsets or backgauge finger geometry:
  open height, stroke and backgauge reach report `not_checked`; a collision with a beam
  envelope (an estimate that errs large) is flagged `against: estimated envelope`.
- Amada's tooling and Rolleri's PDF catalogue were not openly readable (Cloudflare block; a
  contact form): Amada tools are listed UNKNOWN and refused; Rolleri comes from its open online
  listing only. LVD and Accurpress publish no tool dimensions.
- Punch bodies are envelopes (twice the largest published horizontal dimension, symmetric
  about the tip line) unless an outline is given: they err large. Goosenecks whose throat is not
  dimensioned are refused.
- Operator handling between steps is shown, not checked. The flat-pattern link opens the Flat
  Pattern with bend lines on and names the bend; it does not yet pick that one line out.
- Livery colours are not in the data: the press is a neutral grey; appearances are the
  viewport's tint/metal until RENDER-STUDIO's appearance API lands.
