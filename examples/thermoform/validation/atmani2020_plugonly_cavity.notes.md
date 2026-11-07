# atmani2020_plugonly_cavity: assumptions

- For a solver with a moving plug (thermo-fea's `Process::plug`). The 90 mm circular clamp window is the sharp
  rim of a deep (150 mm) cavity: the sheet on the tool's top round it is stuck, so the rim is the clamp.
  This matches the paper's round window, unlike atmani2020_plugonly.fg's square one.
- Plug (paper, Fig. 4): d42, R90 dome nose, R10 edge, tip 20 mm above the sheet at the start. Stroke 60 mm
  VERIFIED (printed for the same test series in the authors' 2019 papers: s2_geom_case_dims_from_renders.csv atmani2020_plugonly
  plug_stroke_course 60, atmani2019cfm p.3, atmani2019csma p.3; was 55 ASSUMED). Unresolved: the 2019 force curves
  leave zero at ~9 mm of course (s2_geom_case_dims_from_renders.csv force_onset_course 9 [8-10]), so the tip may start ~10 mm, not 20, above
  the sheet; 20 is kept (2020 paper).
- Cross-check data: s2_atmani2019_plugonly_thickness.csv (130 C and 120 C lubricated, the 2019 papers) agrees
  with the 2020 Fig. 13 file within its +-0.03 mm bars; not scored separately.
- Dry 130 C: the sheet sticks on the plug; lubricated: it slides (`plug_slip`). 120 C lub reuses the 130 C
  law (one fit). Plug at sheet temperature (isothermal is right).
- Pressure 1e-6 MPa (none in this test; the language needs a positive one).
- Data s runs clamp -> under plug -> clamp: folded about its midpoint to arc length from the centre.
