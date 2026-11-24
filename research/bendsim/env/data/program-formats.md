# Press-brake control program formats: is any openly specified?

Read 2026-11-24. Only sources reachable without login or sign-up count. Scribd copies were not
used because Scribd sits behind a login wall and its page did not load. UNKNOWN means no open
source was found, not that no format exists.

"Program format" means the control's own saved part or program (bend steps, tooling, gauge
positions). "Import" means what the control or its offline software can read in, usually a DXF
flat pattern or a 3D model, which it then turns into its own program.

| Control / maker | Program format (native) | Openly specified? | Source (read 2026-11-24) | Notes |
|---|---|---|---|---|
| **Cincinnati Inc.** (AUTOFORM PC, AUTOFORM+, MAXFORM, PROFORM, CBII PC upgrade; offline "Press Brake" / Bend Simulation Module software v3.7.1+) | XML text files: program `*.pgm` with root `<Document><PressBrakeProgram>`, plus Upper Tool, Lower Tool, CAD Part (`*.part`), Batch and Finger Library files. Before v3.7.1 these were binary. | **Yes** (for this generation) | EM-503 (R-09/09) "Bend Simulation Module" manual, section "XML FILE FORMAT" pp. 25-37: https://wwwassets.e-ci.com/PDF/Manuals/em-503.pdf (lowercase URL works; `EM-503.pdf` returns 403). File extensions are also in EM-499 (R-12/05): https://wwwassets.e-ci.com/PDF/Preinstallation/Press-Brakes/em-499-r-12-05-autoform-cnc-forming-center-pc-control.pdf | Cincinnati says the XML is meant to "open up the capabilities for customers to manipulate programs for their own needs". It gives an annotated sample, then each element and attribute with meaning, default and enum values. Global: `Version`(=9), `ProgName`, `TeachName`, `PartName`, `SetupNotes`, `MatThick` (default 0.060), `MatType` (0 mild steel ... 20 A537; enum listed), `KFactorAuto`, `KFactor`, `RZEnabled`, `FEnabled`, finger types and names, `HasPart`, `NumSteps`. Tooling: `UpperToolSets`/`LowerToolSets` > `ToolList Count` > `ToolSetup Name ID Length Location [Reversed Unmeasured StackedHolderType HolderHeight NumSegs]` > `SegStackup/SegEntry SegValue`. Steps: `StepData/Step` with `Repeats RevMode(0 angle,1 tonnage,2 position,3 absolute,4 adaptive,6 adaptive rehit,7 bumping) RevAngle(included angle) RevTons RevAbsPos RevPos AngleAdj BendLen StrokeLen UpperID LowerID SpdChgDwn SpdChgUp Tilt Dwell DownStop UpStop FormSpeed ProgFoot ProgPalm PartOrient CadStep GageMode(0 std,1 independent,2 taper) GageAllow Retract XLeft/XRight RLeft/RRight ZLeft/ZRight FLeft/FRight SSLeft/SSRight FingerLen ProgCrown ReturnSpd GuardMute MuteOffset ClampMode ClampAdj FingGagePt AdaptRevPos FingerSel FlgOffLeft/Right`, plus `UIDSelectList/UID`, `LIDSelectList/LID` and 8 `ProgOutputs/ProgOutData Active FromType ToType`. Tool files: `<UpperTool Version ToolName Type Height Clearance ...>` with `SegmentList/ToolSeg Length Quantity` and a `Feature3D/SegmentList/Segment3D XVal YVal ZVal [XCent YCent]` profile; lower tool is similar. **All numbers are stored in inches**, whatever the display units. Omitted attributes take the control's defaults. Limits: the manual is from 2009 (software 4.6.0.0). Not every enum is written out in the text layer (some, such as `FingGagePt`, are cut short), and there is no XSD. It does not cover current Cincinnati controls or CI Bend / CI Form software (UNKNOWN whether they still use this XML). Back-gauge axis names and machine-specific fields depend on the machine's configuration. |
| **Delem** (DA-53T/58T/66T/69T; Profile-T / Profile-TL offline) | "Product" files, backed up and restored over USB or network. Extension and internals are not given in the manual. `.DLD` could not be confirmed from any source. One blog (adhmt/ruchengmachinery) says ".da", but that is unverified. | **No** | DA-66T Reference Manual v2.6 (8075-9051A, V0115), ch. 9.4 Backup/restore and 2.2 DXF import: https://machinemfg.com/wp-content/uploads/2017/09/Press-Brake-Delem-DA-66T-Operation-Manual.pdf (machine-builder mirror, not delem.com) | The manual describes only the UI for program files. **Import (partly described):** 2D DXF (optional on the DA-66T) with contour and bend-line layers assigned in the control. Bend angle comes from text near the bend line: a signed value for air bends (+ up, − down), `H<angle>` for hems, `R<radius>` for radius bends. Thickness and material can come from text labels. Per-bend allowance can also come in as text. "Projection" or "cutting" dimensions are chosen in settings. The label strings are user-configurable (Settings::Labels), so this is a convention the operator sets up, not a fixed spec. Profile-T 3D imports IGES/STEP/SAT/X_T (resellers only). "Create .dat-file" is a service dump, not a program. |
| **Cybelec** (CybTouch, ModEva, VisiTouch; PC-1200 / PC-ModEva, CyCAD) | Own part/program formats. `.DAT` appears in older CYCAD / PC1200 conversion. CybTouch uses its own format, converted with "CybTouchConverter" in PC-ModEva. ModEva RA imports and exports "ModEva RA format" jobs. | **No** | CybTouch 8 Installation & Operation manual, "USB Transfer" / "Importing parts from other Cybelec NCs": https://www.machinemfg.com/wp-content/uploads/2023/07/CybTouch-8-Installation-Operation-Manual.pdf ; CYCAD guide on manualmachine.com (third-party mirror) | Programs move only via USB transfer or Cybelec's own converter. No internals are given. **Import:** DXF/IGES flat patterns into the CYCAD/PC1200 "CYBELEC" format. ModEva RA imports DXF flat, 3D IGES and MetaCAM. "CyCAD DXF" (see the Bystronic/LVD row) is a DXF convention with bend data in extended entity data, but no field layout was found. |
| **ESA** (S630/S640/S660; ESA/GPS) | Own program files, saved with a format version (e.g. "Save version" to an earlier format such as 14.0, "Save all As..." to USB). "ESA/GPS" as a format name could not be confirmed. | **No** | Harsle documentation hub, ESA S630 "Version Saving": https://www.harsle.com/?p=66264 (machine-builder docs, not ESA) | The S630 manuals describe the UI only. Numeric and graphic programming are on the control. No file spec was found. Import: UNKNOWN beyond on-control graphic entry. |
| **TRUMPF** (TruBend; TASC 6000; TecZone Bend / TruTops Bend) | Bend program format: UNKNOWN. `.GEO` is TRUMPF's 2D/3D geometry format (flat pattern with contours, bend-line markers, material and thickness), not a bend program. | **No** (GEO: no official spec; a third-party reader exists) | https://github.com/scalenc/geo-format (BSD-3-Clause TypeScript reader by ScaleNC; does not say whether it is derived from an official spec); https://www.cadstudio.cz/en/apps/geo-translator/ (Inventor-to-GEO export: outer/inner outlines, bend-line start/end marks, material/thickness) | trumpf.com TecZone Bend pages are marketing only. A GEO writer could be built from the open-source reader, but it would be reverse-engineered, and GEO carries geometry rather than a bend sequence or tooling. |
| **Amada** (AMNC 3i/PC; VPSS 3i/4ie Bend) | UNKNOWN (`.BMF` could not be confirmed). Programs are generated offline in VPSS and sent to the AMNC. | **No** | https://cdn.amada.eu/fileadmin/Repository/Products/Brochures/software/Software-brochure_2025.pdf | Brochures only. Import: 3D CAD and DXF into VPSS. No format document is public. |
| **Bystronic** (ByVision Bending; BySoft 7 / Bending) | BySoft bending program; `*.JUPIDU` per filext.com, which says it is ZIP-based. That is unverified and not authoritative. | **No** | https://filext.com/file-extension/JUPIDU ; HiCAD "CyCAD" interface help: https://help.isdgroup.com/help/1033/2019/hicad/Subsystems/hicadsst/Content/StahlbaublechSchnittstellen/cycad.htm | Import: BySoft CAD Importer (STEP, IGES, SAT, native CAD) and DXF/DWG. HiCAD exports "CyCAD DXF" for Bystronic/LVD with layers `OUTLINE`, `BENDS`, solid/dashed lines for up/down bends, and per-bend text angle, `R=`, K allowance and `W=`. The XDATA field layout is not given. |
| **LVD** (Touch-B control; CADMAN-B offline) | Native program sent from CADMAN-B over the network: format UNKNOWN. Import to CADMAN-B (v7.5+): DXF plus an XML `*.CBBatchInput`. | **No** | HiCAD help "Bend Zone Tooling LVD": https://help.isdgroup.com/help/1033/2021/hicad/Subsystems/hicadbl/Content/biegewerkzeugbl/biegewerkzeugzuordbl.htm | `CBBatchInput` is an XML input to CADMAN-B (flat pattern and bend data), not the control's program. Its schema is not published (HiCAD says to ask LVD/ISD). |
| **SafanDarley** (E-Brake; E-Control; AutoPOL offline) | AutoPOL exports a "CNC file including graphic information" that loads into the control. Format UNKNOWN. | **No** | https://us.safandarley.com/products/autopol-3d-offline-software/ | Import (EC3D by AutoPOL on the control): 2D/3D files. AutoPOL also exports a PDF bend sheet and DXF. |
| **Accurpress** (ACCELL) | UNKNOWN | **No** (nothing found) | none found | No public manual covering files was found. |
| **Neutral standards** | ISO 14649 (STEP-NC): no press-brake/bending part found. Academic STEP-compliant bending frameworks (AP203/207/219 + ISO 14649 data model) exist only as research. DIN 66025 / ISO 6983: no press-brake dialect found. ISO 6909 covers press-brake safety, not data. | **No** usable standard | https://repository.derby.ac.uk/item/924x8/development-of-a-step-compliant-design-and-manufacturing-framework-for-discrete-sheet-metal-bend-parts ; https://www.iso.org/standard/83403.html | The field's only de facto neutral path is "DXF flat or 3D model + bend data". It carries geometry and bend angles, not a machine program, and the bend-data convention differs by vendor (Delem text labels, CyCAD DXF XDATA, LVD CBBatchInput). |

## Conflicts and uncertainties

- Delem's file type: ".DLD" (in the question) and ".da" (one reseller blog) are both unconfirmed. The
  DA-66T manual calls them "product files" and gives no extension.
- The Cincinnati Bend Simulation manual appears on Scribd (login wall) and openly on
  Cincinnati's own asset server. Only the latter was used.
- The Cincinnati XML covers 2009-era software (v3.7.1 to v4.6). The 2005 manual EM-499 (AUTOFORM PC)
  names `*.pgm` / `*.af` and mentions binary VME formats. Whether today's Cincinnati controls (CI
  Bend, CI Form, PMControl) still read this XML is UNKNOWN.
- TRUMPF GEO: the open reader shows the format can be implemented, but it is not a TRUMPF
  publication and is geometry rather than a bend program.

## Conclusion

Only one control program format turned up with an openly published spec that a writer could be
built from: **Cincinnati Incorporated's XML press-brake program** (`<Document><PressBrakeProgram>`,
`*.pgm`), with its Upper and Lower Tool XML. It is in EM-503 (R-09/09), hosted by Cincinnati at
https://wwwassets.e-ci.com/PDF/Manuals/em-503.pdf. It specifies:

- material: thickness, material-type enum, K factor;
- the die space: upper and lower tool stations with tool name, ID, length, location and segment
  stack-up, plus holder type and height;
- per step: repeats; reversal mode with target angle, tonnage or position; angle correction;
  bend length; stroke; speeds; dwell; crowning; clamp and guard-mute modes; gauge mode with
  X/R/Z/F/SS axis values left and right; finger selection; flange offsets; which tool stations
  are used; and programmable outputs.

All values are in inches, and omitted attributes fall back to the control's defaults. It is a good
target for a ForgeCAD bendsim writer (bend order, tooling and stations, back-gauge positions,
angles). Before we rely on it:

1. It is tied to Cincinnati AUTOFORM/MAXFORM/PROFORM-era controls, and we do not know if current CI
   controls accept it.
2. Some enum lists are cut short in the PDF's text layer.
3. No XSD is published.

Every other control (Delem, Cybelec, ESA, TRUMPF, Amada, Bystronic, LVD, SafanDarley, Accurpress) has
no public spec for its program format. The practical open route to those is the import side: a
**DXF flat pattern with bend lines on their own layer and angle and radius text by each line**. The
Delem DA-66T manual describes how its importer reads this: signed angle for up or down, `H` for hem,
`R` for radius, user-configurable labels, and projection or cutting dimensions. The HiCAD "CyCAD DXF"
notes describe the Bystronic/LVD variant (`OUTLINE`/`BENDS` layers, line type for direction). Neither
amounts to a full spec, and the control then builds its own program. No neutral ISO or DIN
press-brake program standard exists in usable form.
