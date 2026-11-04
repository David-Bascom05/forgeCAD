# Thermoforming: where the numbers come from

## The material data (`kernel/forge-thermo/data/materials.json`)

Every number carries its source (a key of the file's `sources`, or the text itself) or
`stand_in: true`; `material::from_data` refuses one with neither, and a test checks it.

- Design rules, for every material (no maker gives them per material): draft 4 deg male, 2 deg
  female, radii t + 0.8 convex and 0.4 concave, H:D 1 (Universal Plastics, Thermoforming Design
  Guidelines v1.21); a gap as wide as the depth between raised parts (Eastman, Tips for
  thermoforming Eastar 6763); vents under t / 2 (Formech, Vacuum Forming Guide). Formech asks more
  draft (5 male, 3 female).
- Forming windows and normal temperatures: Plastiglas Technical Manual Table 10 (ABS), Spartech's
  thermoforming data sheet rev. 7/2024 (HIPS, PETG, HDPE), Makrolon fabrication guide (PC), ACRYLITE
  tech brief (PMMA). Shrink: the middle of Spartech's in/in ranges, Interstate's for PC, Formech's
  for PMMA. Largest areal draw ratio: Morris, The Science and Technology of Flexible Packaging
  (2017) Table 11.1 (ABS 10, HIPS 8, PC 8, HDPE 8); PETG and PMMA keep a vendor blog's figure
  (RapidMade), a stand-in. The thinnest wall (0.25 t) is a stand-in: no guide states one.
- Laws the thickness model reads: ABS 3-term Ogden at 150 C and Mooney-Rivlin C1 = 0.230 MPa at
  145 C; HIPS 3-term Ogden at 145 C and Mooney-Rivlin C1 = 0.085 MPa at 150 C; all from free bubble
  inflation (Erchiqui & Kandil, ANTEC 2004, SPE Thermoforming Quarterly 24(1) 2005, Tables 1-3).
  HDPE: neo-Hookean with G = 0.337 MPa, the instantaneous modulus of the 6-mode Lodge spectrum in
  Erchiqui 2010 (InTech, Finite Element Analysis, Table 1). PETG, PC and PMMA: no open fit at forming
  temperature (Dong 2005 for PMMA and Cheng & Chang 2012 for PC are paywalled): a stand-in.
- Kept for a viscoelastic model, not read yet: ABS K-BKZ (Wagner) 8-mode spectrum at 170 C with WLF
  and damping (Cha et al. 2019, Int J Adv Manuf Technol 103, Table 1), ABS Lodge at 143 C (Erchiqui
  & Gakwaya 2003), HDPE Lodge spectrum (Erchiqui 2010), HIPS G'Sell-Jonas 100-140 C (Atmani et al.
  2020, Mechanics & Industry 21).

## The thickness model against measured parts (`src/bench.rs`, `data/benchmarks/*.csv`)

`cargo test -p forge-thermo bench -- --nocapture` forms each part as published (fillets assumed
where not given, as each CSV says), reads the thickness along the measured cut and prints both.
Measured profiles were digitised by eye from the papers' figures (about 0.03 mm).

```
part                                   law (source)                  mean |error|   minimum: measured / model
Duarte 2022 PS/HIPS cone, 1 mm, 140 C  HIPS Ogden 145 C              1.2% of t0     0.050 / 0.067 mm, both at the bottom edge
Erdogan 2014 HIPS cylinder, 2.5 mm     HIPS Mooney-Rivlin 150 C      16.7%          0.83 / 0.40 mm
Erdogan 2014 HIPS box, axis cut, 2 mm  "                             12.7%          0.60 / 0.49 mm
Erdogan 2014 HIPS box, diagonal cut    "                             12.3%          0.19 / 0.27 mm
```

(80 sheet nodes across; 56, the default, gives 3.2% on Duarte's cone.) Erdogan's parts were formed
at 180 C, past the fits' 145-150 C, and hold more material than their opening: the sheet slid in
from the flange, which sticking contact cannot do, so the model thins their floors too much.

## Prior art

No CAD package checked (Inventor, SolidWorks, Fusion, Creo, NX, Onshape) has a thermoform feature;
they offer draft and radius analysis. The full survey, with the wall-thickness methods, lives
outside the repo with the design notes.
