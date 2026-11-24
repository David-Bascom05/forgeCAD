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
