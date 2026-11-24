# Bend Simulation environment: data, shots

- `data/`: the research notes every library value is restated from (one cite per value: URL,
  date read, the source's statement). `vendor-tooling.md` (Trumpf, Bystronic, Amada),
  `maker-tooling.md` (WILA, Wilson Tool, Mate, Rolleri), `press-brakes.md` (32 press brakes).
  The library TOML (kernel/forge-brake/data/) is generated from them by tools/bendsim/
  md2toml.py and press2toml.py.
- `shots/native-*.png`: interface/forge-render/tests/bendsim_env.rs on the view's renderer
  (Vulkan, lavapipe), `BENDSIM_SHOTS=DIR`, at the environment's own framings (View: Part and
  Tools, the default; Press Overview):
  - 00: a 100 x 200 channel on a Trumpf TruBend 5130 with Trumpf's own punches and dies, the
    default view (the part, the punch and the die nearly end on, the bend line marked amber)
    and the Press Overview (C-frame housings from the published throat, ram, bed);
  - 01-04: approach, first bend, the operator's handling between steps, second bend (the eye
    follows the part to the second station);
  - 05: a 40 mm U with 70 mm flanges on a Bystronic Xpert Pro 100/3100, the straight P6 punch
    only: the second flange closes on the punch body at phi 45 deg (certified witness), the
    punch and the flange drawn red, the check named in words with "locate";
  - 06: the program written as the part's bending document: each step pictured end on (drag a
    picture onto another row to move the bend), its station, the step's results with the checks
    folded to failures and those not checked;
  - 07: Machine & Tools: the press by its maker's name, the tool library as cards (drawn
    cross-section, name, angle, radius, height, rated load; the id on hover), only what fits;
  - 08: the stations on the bed, drawn to scale between the housings, each stack in its
    published segments; dragged to place.
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
- Punch bodies are checked as envelopes (twice the largest published horizontal dimension,
  symmetric about the tip line) unless an outline is given: they err large. They are drawn at
  the published dimension (`drawn_width`); the envelope is not drawn (the view's renderer has
  no transparency for a ghost). Goosenecks whose throat is not dimensioned are refused.
- The flat-pattern link opens the Flat Pattern with bend lines on and names the bend; it does
  not yet pick that one line out.
- Livery colours are not in the data: the press is RENDER-STUDIO's neutral machine paint; the
  bend zone's amber is the panel's annotation colour, not an appearance.
- The tool library's "fits" filter uses the rules' own sides (tip angle and radius against
  the sharpest bend, die angle) and the clamping system; open height only where it is checked.
