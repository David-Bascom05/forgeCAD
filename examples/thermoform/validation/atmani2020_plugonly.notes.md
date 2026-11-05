# atmani2020_plugonly: assumptions

- The solver has NO plug: the plug-only test is modelled as a male tool (the plug) with the clamp coming down over it
  (the same relative motion). Sticking contact on the plug = the DRY case; the lubricated runs slide (not modelled).
- Plug: d42, edge fillet R10 (paper). The R90 dome nose is left flat (its sag over r21 is 2.5 mm). Height 60 =
  the stroke, DERIVED from the measured profile length (~180 mm across a 90 mm window); the stroke is not printed.
- Window: the model's clamp window is SQUARE (trim box + margin = 90 x 90); the paper's is a 90 mm CIRCLE. Compared
  along the x axis through the centre, where both edges sit at 45 mm.
- Pressure 1e-6 MPa (no vacuum in this test; the language needs a positive pressure).
- Plug and sheet at 130 C (plug heated to sheet T): isothermal is right here. 120 C lub reuses the 130 C model (one fit).
- Data s runs clamp -> under plug -> clamp; folded about its middle to arc length from the clamp.
