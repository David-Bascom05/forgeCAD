# buntinx2014_tray25: assumptions

- Mould: top 190 x 132, wall conicity 10 deg (taper), depth 25, floor R1 (paper). Vertical corners 'R ~1' (R5 for 50R5): taken
  as R1 at the floor, so R 5.408175 at the top (a tapered extrude shrinks the corner by D tan 10 deg).
- Material: the rows are PP, PP/EVOH/PP, PS/EVOH/PE, APET/PE, PA/PE and multilayers; one model (library PP, a
  neo-Hookean stand-in) serves all: the MEAN wall of a fully formed sticking part is t0 x opening area / part area,
  independent of the law, so only the geometry and draw-in enter. t0 0.4 mm here; rows compare t_mean / t0.
- Plug (syntactic foam) + compressed air in the paper (pressure and plug speed NOT given): modelled air only at
  0.4 MPa ASSUMED. The plug moves material but not the mean, unless the sheet draws in from the flange.
- Measured mean: Magna-Mike over ~1 cm2 squares; the square counts match HALF the tray surface (DERIVED), so the
  mean is over the part (flange excluded). The model's mean is over the sheet below the rim plane.
