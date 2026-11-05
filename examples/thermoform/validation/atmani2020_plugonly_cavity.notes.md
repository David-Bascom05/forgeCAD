# atmani2020_plugonly_cavity: assumptions

- For a solver with a moving plug (thermo-fea's `Process::plug`). The 90 mm circular clamp window is the sharp
  rim of a deep (150 mm) cavity: the sheet on the tool's top round it is stuck, so the rim is the clamp.
  This matches the paper's round window, unlike atmani2020_plugonly.fg's square one.
- Plug (paper, Fig. 4): d42, R90 dome nose, R10 edge, tip 20 mm above the sheet at the start. Stroke NOT
  printed for Fig. 13: 55 mm ASSUMED (the middle of the 45-65 mm range whose section length matches the
  measured ~170-180 mm, as forge-thermo's bench scans).
- Dry 130 C: the sheet sticks on the plug; lubricated: it slides (`plug_slip`). 120 C lub reuses the 130 C
  law (one fit). Plug at sheet temperature (isothermal is right).
- Pressure 1e-6 MPa (none in this test; the language needs a positive one).
- Data s runs clamp -> under plug -> clamp: folded about its midpoint to arc length from the centre.
