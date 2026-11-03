# RACK-18U order package: fabricated parts

From asm/rack.fg's run (out/rack.json). SendCutSend, cold rolled 1008 mild steel, black textured
powder coat (SPEC 12). Each part: STEP of the folded part (SendCutSend's route for bent parts,
read back by OpenCascade), DXF of the flat, and the bend table for entering the bends by hand
if the STEP is refused. Bend angle sign is Forge's (negative: the flange turns to the DXF's back
face). Open items in the assembly at the time of this package are listed at the end.

## RACK-101 post, qty 4

- file: `frame/post.fg`, STEP `order/RACK-101_post.step`, DXF `order/RACK-101_post.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 258028.2 (Forge 258028.2)
- thickness 3.02, flat bbox 95.08 x 906.00
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
leg_b                -90.0      1.60   906.00    (2.46, 906.00) - (2.46, 0.00)
```

PEM hardware, per part (pressed by SendCutSend):
```
FH-M6-20   x2   in leg_a
FH-M6-20   x2   in leg_b
S-M6-0     x6   in leg_a
S-M6-0     x6   in leg_b
SS-M5-0    x6   in leg_a
SS-M5-0    x6   in leg_b
```

## RACK-111 rail carrier, qty 4

- file: `frame/carrier.fg`, STEP `order/RACK-111_carrier.step`, DXF `order/RACK-111_carrier.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 89155.6 (Forge 89155.6)
- thickness 1.88, flat bbox 714.00 x 68.73
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
vert                  90.0      1.60   714.00    (0.00, 1.63) - (714.00, 1.63)
```

PEM hardware, per part (pressed by SendCutSend):
```
S-M6-0     x3   in horiz
S-M6-0     x28  in vert
```

## RACK-121 base pan, qty 1

- file: `frame/base_pan.fg`, STEP `order/RACK-121_base_pan.step`, DXF `order/RACK-121_base_pan.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 1441496.9 (Forge 1441496.9)
- thickness 3.02, flat bbox 641.07 x 774.11
- shop and DFM warnings: 0
- bends: 4

```
flange              angle   inside r  length    bend line in the DXF
front                 90.0      1.60   584.88    (-292.44, -359.52) - (292.44, -359.52)
right                 90.0      1.60   717.92    (293.00, -358.96) - (293.00, 358.96)
rear                  90.0      1.60   584.88    (292.44, 359.52) - (-292.44, 359.52)
left                  90.0      1.60   717.92    (-293.00, 358.96) - (-293.00, -358.96)
```

## RACK-131 top pan, qty 1

- file: `frame/top_pan.fg`, STEP `order/RACK-131_top_pan.step`, DXF `order/RACK-131_top_pan.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 1374629.0 (Forge 1374629.1)
- thickness 3.02, flat bbox 641.07 x 774.11
- shop and DFM warnings: 0
- bends: 4

```
flange              angle   inside r  length    bend line in the DXF
front                 90.0      1.60   584.88    (-292.44, -359.52) - (292.44, -359.52)
right                 90.0      1.60   717.92    (293.00, -358.96) - (293.00, 358.96)
rear                  90.0      1.60   584.88    (292.44, 359.52) - (-292.44, 359.52)
left                  90.0      1.60   717.92    (-293.00, 358.96) - (-293.00, -358.96)
```

PEM hardware, per part (pressed by SendCutSend):
```
FH-M8-25   x4   in floor
```

## RACK-201 EIA rail, qty 4

- file: `frame/rail.fg`, STEP `order/RACK-201_rail.step`, DXF `order/RACK-201_rail.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 105880.5 (Forge 105880.5)
- thickness 1.88, flat bbox 68.61 x 890.00
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
web                  -90.0      1.60   890.00    (1.63, 890.00) - (1.63, 0.00)
```

## RACK-141 side half-panel, qty 4

- file: `panels/side_panel.fg`, STEP `order/RACK-141_side_panel.step`, DXF `order/RACK-141_side_panel.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 565273.5 (Forge 565273.5)
- thickness 1.52, flat bbox 800.00 x 470.23
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
rtn                   90.0      1.60   800.00    (800.00, 451.61) - (0.00, 451.61)
```

## RACK-301 door, qty 2

- file: `panels/door.fg`, STEP `order/RACK-301_door.step`, DXF `order/RACK-301_door.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 693654.9 (Forge 693655.0)
- thickness 1.52, flat bbox 630.92 x 872.46
- shop and DFM warnings: 0
- bends: 4

```
flange              angle   inside r  length    bend line in the DXF
top_r                 90.0      1.60   583.42    (1.52, -826.61) - (584.94, -826.61)
latch_r               90.0      1.60   824.96    (585.07, -826.48) - (585.07, -1.52)
bot_r                 90.0      1.60   583.42    (584.94, -1.39) - (1.52, -1.39)
hinge_r               90.0      1.60   824.96    (1.39, -1.52) - (1.39, -826.48)
```

PEM hardware, per part (pressed by SendCutSend):
```
FH-M4-12   x3   in field
S-M6-0     x1   in bot_r
S-M6-0     x1   in top_r
```

## RACK-311 hinge bracket, qty 4

- file: `panels/hinge_bracket.fg`, STEP `order/RACK-311_hinge_bracket.step`, DXF `order/RACK-311_hinge_bracket.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 4618.3 (Forge 4618.3)
- thickness 1.88, flat bbox 34.00 x 77.53
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
vert                  90.0      1.60    34.00    (17.00, 19.37) - (-17.00, 19.37)
```

## RACK-312 strike, qty 2

- file: `panels/strike.fg`, STEP `order/RACK-312_strike.step`, DXF `order/RACK-312_strike.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 4295.6 (Forge 4295.6)
- thickness 1.88, flat bbox 60.61 x 40.00
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
web_f                 90.0      1.60    40.00    (14.77, -20.00) - (14.77, 20.00)
```

## RACK-313 latch cam, qty 2

- file: `panels/cam.fg`, STEP `order/RACK-313_cam.step`, DXF `order/RACK-313_cam.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 2596.3 (Forge 2596.3)
- thickness 1.88, flat bbox 48.90 x 43.80
- shop and DFM warnings: 0
- bends: 0

PEM hardware, per part (pressed by SendCutSend):
```
S-M4-0     x2   in cam
```

## RACK-321 latch handle, qty 2

- file: `panels/latch_handle.fg`, STEP `order/RACK-321_latch_handle.step`, DXF `order/RACK-321_latch_handle.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 4172.5 (Forge 4172.5)
- thickness 1.88, flat bbox 47.00 x 66.49
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
fin_f                 90.0      1.60    28.98    (14.49, 18.75) - (-14.49, 18.75)
```

## RACK-322 latch base, qty 2

- file: `panels/latch_base.fg`, STEP `order/RACK-322_latch_base.step`, DXF `order/RACK-322_latch_base.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 13899.0 (Forge 13899.0)
- thickness 3.02, flat bbox 53.40 x 94.62
- shop and DFM warnings: 0
- bends: 1

```
flange              angle   inside r  length    bend line in the DXF
tab_f                 90.0      1.60    53.40    (21.40, 29.54) - (-32.00, 29.54)
```

## RACK-401 fan tray, qty 1

- file: `fans/rack401_tray.fg`, STEP `order/RACK-401_rack401_tray.step`, DXF `order/RACK-401_rack401_tray.dxf`
- STEP read back by OpenCascade: 1 solid, valid True, volume 107807.6 (Forge 107807.6)
- thickness 1.88, flat bbox 333.47 x 300.00
- shop and DFM warnings: 0
- bends: 2

```
flange              angle   inside r  length    bend line in the DXF
right                -90.0      1.60   300.00    (148.37, -150.00) - (148.37, 150.00)
left                 -90.0      1.60   300.00    (-148.37, 150.00) - (-148.37, -150.00)
```

PEM hardware, per part (pressed by SendCutSend):
```
S-M4-0     x16  in tray
```

## PEM totals (whole rack)

```
FH-M4-12   6
FH-M6-20   16
FH-M8-25   4
S-M4-0     20
S-M6-0     176
SS-M5-0    48
total      270   (SPEC 4: 270)
```

## Open items in the assembly

- none
