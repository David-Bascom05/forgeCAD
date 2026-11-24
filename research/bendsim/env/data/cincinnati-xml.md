# Cincinnati press-brake program XML (`*.pgm`): writer notes

Source: Cincinnati Incorporated, EM-503 (R-09/09) "Bend Simulation Module", edited for Press Brake
Software Version 4.6.0.0 (title page). Section "XML FILE FORMAT", read 2026-11-24 from
https://wwwassets.e-ci.com/PDF/Manuals/em-503.pdf (lowercase URL).

Page numbers below are the manual's printed page numbers (PDF page = printed + 4). The task brief
says pp. 25-37, but printed p. 25 is the end of the Configuration Wizard. The XML section starts on
p. 26. The program format runs pp. 26-35, the Upper Tool format pp. 35-37, and the Lower Tool format
starts at the foot of p. 37. The PDF's text layer drops many runs of text, including some enum rows,
the sample's `ProgName` and finger names, and parts of descriptions. Every page below was therefore
read from the rendered page image, not from extracted text. Wording here is restated. Element and
attribute names, enum names and values, and the sample's numbers are reproduced as facts.

Conventions: **UNKNOWN** means the manual does not say. **INFERRED** means derived from the sample's
numbers, not stated in the text.

---

## 0. Scope and general rules (pp. 26-27, 29)

- XML replaced binary storage for Programs, Tools, CAD Parts, Batch files and Finger Library files.
  It became available with software version 3.7.1 (p. 26).
- The manual gives an annotated sample, then a description per tag and attribute (p. 26).
- **Omitted attributes:** the control uses its default for any tag that is not in the file. The
  manual suggests learning the defaults by creating a new program on the control and reading its
  fields (p. 26). Not every program uses every field. Which fields appear depends on the backgauge
  type, the reversal mode (angle, position or tonnage) and how varied the tooling is (p. 26).
- **Units:** all numeric values are stored in English units, whatever the control displays
  (English or Metric) (p. 27). Lengths are therefore in **inches**. Exceptions where the text gives
  another unit:
  - angles in **degrees** (`RevAngle`, `AngleAdj`, p. 31-32; tool `Angle`, p. 37)
  - time in **seconds** (`Dwell`, p. 32)
  - speeds in **inches per minute** (`FormSpd`/`FormSpeed`, `ReturnSpd`, p. 32-33)
  - force in **tons** (`RevTons`, p. 31). US or metric tons is UNKNOWN; US tons is likely, given
    the English-unit rule.
  - tool `MaxLoad` in **tons per foot** (p. 37)
  - integer enums and bit fields have no unit.
- **Number formatting:** floating-point precision matches the precision shown on screen for that
  field, and trailing zeros are not stored (p. 27). A writer should emit minimal decimal strings
  (`24`, not `24.000`).
- **Indentation:** files are written without indentation. The indented sample is only a browser's
  view of the file (p. 27).
- **Prolog:** every file starts with `<?xml version="1.0" ?>`. The manual notes that 1.0 is the only
  XML version and that the default encoding is UTF-8 (p. 29). It gives no `encoding=` attribute.
- **Root:** `<Document>` is the root of every Press Brake file. For a file to be recognised as a
  program, the next element must be `<PressBrakeProgram>` (p. 29).
- File extension `.pgm`: the sample's `ProgName` is `C:\PressBrake\Program\samplexml.pgm` (p. 27).

---

## 1. Element nesting tree (sample pp. 27-28; tables pp. 29-35)

`@` marks an attribute. `[n]` gives a repeat count and the attribute that holds it.

```
<?xml version="1.0" ?>
Document                                   (root, no attributes)
└── PressBrakeProgram                      @Version @ProgName @TeachName @PartName @SetupNotes
    │                                      @MatThick @MatType @KFactorAuto @KFactor @RZEnabled
    │                                      @FEnabled @LeftFingerType @RightFingerType
    │                                      @LUFingerName @LLFingerName @RUFingerName @RLFingerName
    │                                      @HasPart @NumSteps
    ├── UpperToolSets                      (no attributes)
    │   └── ToolList                       @Count   = number of ToolSetup children
    │       └── ToolSetup [Count]          @Name @ID @Length @Location @Reversed @Unmeasured
    │           │                          @StackedHolderType @HolderHeight @NumSegs
    │           └── SegStackup  (0..1)     (no attributes; present only when the tool is segmented)
    │               └── SegEntry [NumSegs] @SegValue  (empty element)
    ├── LowerToolSets                      (same sub-structure as UpperToolSets)
    │   └── ToolList @Count
    │       └── ToolSetup [Count] ...
    │           └── SegStackup
    │               └── SegEntry [NumSegs] @SegValue
    └── StepData                           (no attributes; at least one Step)
        └── Step [NumSteps]                @Repeats @RevMode @RevAngle @RevTons @RevAbsPos @RevPos
            │                              @AngleAdj @BendLen @StrokeLen @UpperID @LowerID
            │                              @SpdChgDwn @SpdChgUp @Tilt @Dwell @DownStop @UpStop
            │                              @FormSpeed @ProgFoot @ProgPalm @PartOrient @CadStep
            │                              @GageMode @GageAllow @GagePause @Retract
            │                              @XLeft @XRight @RLeft @RRight @ZLeft @ZRight
            │                              @FLeft @FRight @SSLeft @SSRight @FingerLen @ProgCrown
            │                              @ReturnSpd @GuardMute @MuteOffset @ClampMode @ClampAdj
            │                              @FingGagePt @AdaptRevPos @FingerSel
            │                              @FlgOffRight @FlgOffLeft
            ├── UIDSelectList              @Size = number of UID children
            │   └── UID [Size]             @IntValue  (empty element)
            ├── LIDSelectList              @Size
            │   └── LID [Size]             @IntValue  (empty element)
            └── ProgOutputs                (no attributes)
                └── ProgOutData [exactly 8 in the sample; "8 possible outputs per step", p. 34]
                                           @Active @FromType @ToType @FromValue @ToValue
                                           (an unused slot is written as a bare <ProgOutData />)
```

Notes on the tree:
- All data is in attributes. No element carries text content.
- There are two count-attribute spellings: `ToolList@Count` and `UIDSelectList@Size` /
  `LIDSelectList@Size`. Segments are counted by the parent `ToolSetup@NumSegs`, not on
  `SegStackup`. Steps are counted by `PressBrakeProgram@NumSteps`, not on `StepData`.
- `ProgOutputs` always holds 8 `ProgOutData` children in the sample, including empty ones. The slots
  are positional (slot k = programmable output k), INFERRED.
- Child order in the sample: UpperToolSets, LowerToolSets, StepData. Within a Step: UIDSelectList,
  LIDSelectList, ProgOutputs. Whether the reader needs this order is UNKNOWN. Keep it.
- **The sample is not well-formed XML.** The `ToolSetup` elements with `ID="2"` (upper and lower)
  end in `>` but are never closed (p. 27). A writer should close them (`<ToolSetup ... />`). The
  sample also uses typographic quotes (”). That is typesetting; use ASCII `"`.

---

## 2. Attributes: meaning, unit, default, values

Columns: **Req** = required or optional as the manual states it. "opt (default X)" means the manual
gives a default, so omitting the attribute is allowed (p. 26). UNKNOWN means no default and no
required/optional statement. In theory every attribute may be omitted (p. 26). In practice some
attributes are needed for a program to mean anything, and those are marked "needed (INFERRED)".

### 2.1 `PressBrakeProgram` (pp. 29-30)

| Attribute | Meaning (restated) | Unit / type | Default | Values | Req |
|---|---|---|---|---|---|
| Version | Internal number the application uses to record when format tags were added. Sample `9`. | int | UNKNOWN | UNKNOWN which values exist or what each adds | UNKNOWN. Write `9`, as the sample does (4.6.0.0-era). |
| ProgName | Path where the program was first created and stored. Reference only. | string | — | — | **Not required** (stated p. 29) |
| TeachName | Path of the Teach file, when the program uses Adaptive Forming mode | string | UNKNOWN (sample `""`) | — | opt (only for adaptive) |
| PartName | Path of the CAD part file, when the program came from a CAD Part | string | UNKNOWN (sample `""`) | — | opt |
| SetupNotes | Free-form operator notes, shown on the Job Setup tab. No hard length limit. Truncated on printouts. | string | UNKNOWN | — | opt |
| MatThick | Global material thickness | float, in | **0.060** | — | needed (INFERRED) |
| MatType | Material type code | int | **0** (Mild Steel) | 0 Mild Steel; 1 High Strength Steel; 2 Stainless Steel; 3 Soft Aluminum; 4 Hard Aluminum; 5 Soft Copper; 6 Hard Copper; 7 A36-Structural Steel; 8 A131-Struct.Stl Ships; 9 A242-Hi.Str Low Alloy; 10 A283-Lo.Str Pl.Struct; 11 A284-Lo.Str Pl.GenConst; 12 A285-Lo.Str Pl.Prs.Vsls; 13 A299-Carb.Mang Prs.Vsls; 14 A366-Cold Rolled Sh.Comm; 15 A441-Hi.Str Lo.Alloy Mng; 16 A442-Carbon Plt Prs.Vsls; 17 A514-High Yld Str Qu&Tmp; 18 A516-Carbon Plt Low Temp; 19 A517-Hi.Str Alloy Qu&Tpr; 20 A537-Heat Trtd Mang Plt; 21 A569-Hot Rolld Sh.Comm; 22 A570-Hot Rolld Sh.Struct; 23 A572-Hi.Str Lo.Alloy C-V; 24 A588-Hi.Str Low Alloy Stru; 25 A606-Steel Sh.&Strip CorrR; 26 A607-Steel Sh.& Strip C-V; 27 A633-Norm.High Str Struct; 28 A656-Hot Rold Hi.Str Strct; 29 A715-Hot Rold Hi.Str Sheet (complete list, pp. 29-30) | opt |
| KFactorAuto | 1 = control calculates the K-forming factor; 0 = manual | bool 0/1 | **1** | 0, 1 | opt |
| KFactor | K-forming factor to use when KFactorAuto=0. This is Cincinnati's forming (tonnage) factor, not the bend-allowance K-factor (INFERRED from "K-Forming factor"; UNKNOWN for certain). | float | **0.330** | — | opt |
| RZEnabled | Whether the backgauge R and Z axes may move | bool | **1** | 0, 1 | opt |
| FEnabled | Whether the front-gauge axis may move | bool | **1** | 0, 1 | opt |
| LeftFingerType, RightFingerType | Type and orientation of the backgauge finger in use | int | **-1** ("undefined"), as the text says. The enum's own undefined value is 0. Conflict. | 0 FINGER_UNDEFINED; 1 BLOCK_FINGER; 2 STEP_FINGER_0DEG; 3 STEP_FINGER_90DEG; 4 STEP_FINGER_270DEG; 19 NO_FINGER_SELECTED (finger present but undefined); 20 FINGER_5AXIS_STEP; 21 FINGER_5AXIS_FLAT; 22 FINGER_5AXIS_OTT (over-the-top); 29 FINGER_5AXIS_CUSTOM (complete, p. 30) | opt |
| LUFingerName, LLFingerName, RUFingerName, RLFingerName | Name of the finger type in use (left/right, upper/lower). 5-axis backgauge only. The manual's heading lists "LUFingerName, RUFIngerName, RLFingerName, LUFingername", a typo for LL. The sample uses LLFingerName. Sample value `5 Axis Step Finger`. | string | NULL (undefined) | Finger Library names | opt |
| HasPart | Whether the program was created from a CAD part | bool | UNKNOWN | 0, 1 | opt |
| NumSteps | Number of bending steps | int | **1** | ≥1 | needed (INFERRED). Must equal the number of `Step` elements (INFERRED). |

### 2.2 `ToolList` / `ToolSetup` / `SegStackup` / `SegEntry` (pp. 30-31)

`UpperToolSets` and `LowerToolSets` have the same sub-structure. There can be any number of
entries, and one tool can appear more than once at a different length and location (p. 30).

| Element.Attribute | Meaning | Unit / type | Default | Values | Req |
|---|---|---|---|---|---|
| ToolList.Count | Number of tools actually in the die space (= ToolSetup children) | int | — | — | needed |
| ToolSetup.Name | Tool name as it appears in the Tool Library (must match an Upper/Lower Tool file's `ToolName`, INFERRED) | string | — | — | needed |
| ToolSetup.ID | Internal integer that identifies this tool instance. Steps refer to it. Sample IDs 1, 2, 3 per list; upper and lower lists number independently. | int | — | — | needed |
| ToolSetup.Length | Total length of the tool (station) | float, in | UNKNOWN | — | needed (INFERRED) |
| ToolSetup.Location | Position of the tool in the die space. 0.0 = machine centre. Negative = left and positive = right, seen from the front of the press brake. | float, in | UNKNOWN. In the sample `ID="1"` has no Location, so 0.0 (centred) is probable, INFERRED. | — | opt? |
| ToolSetup.Reversed | 0 = installed normally, 1 = facing opposite to the operator | bool | UNKNOWN (0 likely) | 0, 1 | opt |
| ToolSetup.Unmeasured | 1 = disregard all tools, so every ram position must be entered as an absolute value | bool | UNKNOWN (0 likely) | 0, 1 | opt |
| ToolSetup.StackedHolderType | Extra holder used with the tool. 0 = no holder. | int | 0 (no holder) | 20 EURO_STACKING_HOLDER; 7 UCLAMP_US_HOLDER; 20 STACKING_RAIL (lower tool only). The manual gives 20 for both Euro holder and stacking rail; it is told apart by upper vs lower. | opt |
| ToolSetup.HolderHeight | Height of the stacking rail, for a lower tool on a stacking rail | float, in | UNKNOWN | — | opt |
| ToolSetup.NumSegs | Number of segments, when the tool is stored as segmented. Absent otherwise. | int | absent | — | conditional |
| SegStackup | Container for the segment lengths. Present only when the tool is segmented. | — | — | — | conditional |
| SegEntry.SegValue | Length of one segment, one entry per segment | float, in | — | — | conditional |

Sample consistency: the SegValues sum to the ToolSetup `Length` (18+3.937+1.968 = 23.905;
11.811+7.874+3.937 = 23.622; 7.874 = 7.874). Unsegmented `ID="2"` tools have `Length="24"` and no
NumSegs. INFERRED rule: `Length` = Σ SegValue.

### 2.3 `Step` (pp. 31-34)

| Attribute | Meaning | Unit / type | Default | Values | Req |
|---|---|---|---|---|---|
| Repeats | Times the step is repeated | int | **1** | ≥1 | opt |
| RevMode | Reversal mode | int | UNKNOWN in the text. Sample step 1 has no RevMode but sets RevAngle and AngleAdj, so the default is probably 0 = angle, INFERRED. | 0 REV_MODE_ANGLE; 1 REV_MODE_TONNAGE; 2 REV_MODE_POSITION; 3 REV_MODE_ABSOLUTE; 4 REV_MODE_ADAPTIVE; 5 REV_MODE_ANGLE_DTC; 6 REV_MODE_ADAP_REHIT; 7 REV_MODE_BUMPING (complete, p. 31). The list also opens with a stray row `GAGE_MODE_STANDARD =` with no value, a copy-paste artifact. | needed for an air bend (write 0) |
| RevAngle | Desired bend angle, as the **included** angle (180 = flat), when RevMode is angle | float, **deg** | UNKNOWN | — | needed in angle mode |
| RevTons | Tonnage at which the ram reverses, tonnage mode | float, tons | UNKNOWN | — | tonnage mode |
| RevAbsPos | Ram reversal position, absolute position mode | float, in | UNKNOWN | — | absolute mode. Datum of the absolute ram position UNKNOWN. |
| RevPos | Ram reversal position, position mode. Relative to **top of material**. | float, in | UNKNOWN | — | position mode |
| AngleAdj | Angle correction in angle mode: difference between desired and actual angle. Negative = more closed angle. | float, **deg** | UNKNOWN (0 likely) | — | opt |
| BendLen | Length of material under the die (bend length) | float, in | UNKNOWN | — | needed (INFERRED; the control uses it for tonnage). Sample step 2 omits it. |
| StrokeLen | Ram opening above top of material after reversal (stroke length) | float, in | UNKNOWN | — | opt? |
| UpperID (text: "UpperId") | Which upper ToolSetup the step uses: a `ToolSetup@ID` in UpperToolSets | int | UNKNOWN | an ID in the upper ToolList | needed |
| LowerID (text: "LowerId") | Same, for LowerToolSets | int | UNKNOWN | an ID in the lower ToolList | needed |
| SpdChgDwn | Height above material where the ram goes from approach to forming speed | float, in (relative to material) | UNKNOWN | — | opt |
| SpdChgUp | Height above material where the ram goes from forming to return speed | float, in | UNKNOWN | — | opt |
| Tilt | Relative position offset between the left and right cylinders | float, in | UNKNOWN (0 likely) | — | opt |
| Dwell | Time the ram dwells at reversal | float, **s** | **0** | max 9.9 | opt |
| DownStop, UpStop (text: "UpSto") | Stop the ram at speed change on the downstroke / just after reversal on the upstroke | bool | UNKNOWN | 0, 1 ("On or Off"; which number means On is not stated clearly; 1 = on is likely) | opt |
| FormSpeed (text: "FormSpd") | Ram speed during forming | float, **in/min** | UNKNOWN | — | opt |
| ProgFoot | Which footswitches are active. Bits 0-3 = footswitches 1-4. | int bitmask | UNKNOWN | 0-15 | opt |
| ProgPalm | Which palm stations are active. Bits 0-3 = palm stations 1-4. | int bitmask | UNKNOWN | 0-15 | opt |
| PartOrient | Part orientation (flips and turns). Low 4 bits = turns; high 4 bits = flips. | int bit field | UNKNOWN | Bit meanings within each nibble: UNKNOWN | opt |
| CadStep | Whether the step has a CAD graphic | bool | UNKNOWN | 0, 1 | opt |
| GageMode | Backgauge mode | int | UNKNOWN (0 likely) | 0 GAGE_MODE_STANDARD; 1 GAGE_MODE_INDEPENDENT; 2 GAGE_MODE_TAPER (complete, p. 32) | opt |
| GageAllow | Signed value added to the flange position to allow for bend allowance | float, in | **0** | — | opt |
| GagePause | **Not described** in the attribute tables. It appears only in the sample (`0.2`). Meaning, unit and default UNKNOWN (perhaps seconds of pause before gauging). | float | UNKNOWN | — | opt |
| Retract | How far the X axis retracts once the ram reaches the clamp position | float, in | **0** | — | opt |
| XLeft, XRight | Position of the left / right X-axis backgauge arm | float, in | **max travel − 1.0"** | — | needed for gauged bends (INFERRED) |
| RLeft, RRight | Position of the left / right R-axis (vertical) backgauge arm | float, in | **0.050"**. The sample writes -0.05, so the sign is significant and the default's sign may be lost in the text. UNKNOWN. | — | opt |
| ZLeft | Position of the left Z-axis (lateral) backgauge arm | float, in | **-4.000"** | — | opt |
| ZRight | Position of the right Z-axis backgauge arm | float, in | **4.000"** | — | opt |
| FLeft, FRight | Position of the left / right front-gauge arm | float, in | UNKNOWN | — | opt |
| SSLeft, SSRight | Position of the left / right sheet-support arm | float, in | UNKNOWN | — | opt |
| FingerLen | Offset applied to the X-axis arm position. Not used with a 5-axis gauge. | float, in | **0.0"** | — | opt |
| ProgCrown | Position of the programmable crown axis, if fitted | float (in, INFERRED) | **0.0** | — | opt |
| ReturnSpd | Ram speed after reversal, to top stop | float, **in/min** | **20 ipm** | — | opt |
| GuardMute | Where the guard mutes | int | UNKNOWN | 0 GUARD_MUTE_TOP_OF_MAT; 1 GUARD_MUTE_TOP_OF_DIE; 2 GUARD_MUTE_REV_POS (complete, p. 33) | opt |
| MuteOffset | Offset of the mute point when the selection is top of material + offset | float, in | **0.250"** | — | opt |
| ClampMode | Material clamp behaviour | int | UNKNOWN | 0 CLAMP_OFF; 1 CLAMP_STOP; 2 CLAMP_PAUSE (complete, p. 33) | opt |
| ClampAdj | Signed offset from the normal fixed clamp position | float, in | **0.0** | — | opt |
| FingGagePt | Which gauging surface of the backgauge finger is used | int | UNKNOWN | 0 GAGE_POINT_NONE; 1 GAGE_POINT_1 (edge for a step finger, or 1st OTT position); 2 GAGE_POINT_2 (step for a step finger, or 2nd OTT); 3 GAGE_POINT_3 (third OTT position, if used) (complete, pp. 33-34; the text layer cuts it after 0) | opt |
| AdaptRevPos | Adaptive mode: position where the ram actually reversed | float, in | UNKNOWN | — | adaptive only. The sample writes `0`. |
| FingerSel | Which finger is used, 5-axis backgauge | int | UNKNOWN | 0 FINGER_SELECT_OTT; 1 FINGER_SELECT_UPPER; 2 FINGER_SELECT_LOWER; 3 FINGER_SELECT_UPPERLEFT; 4 FINGER_SELECT_LOWERLEFT; 5 FINGER_SELECT_UPPERRIGHT; 6 FINGER_SELECT_LOWERRIGHT; 7 FINGER_SELECT_ABSOLUTEUPPER; 8 FINGER_SELECT_ABSOLUTELOWER (complete, p. 34) | opt (5-axis) |
| FlgOffRight, FlgOffLeft | Offset of the X-axis arm, 5-axis backgauge | float, in | **0.0"** | — | opt |

### 2.4 `UIDSelectList` / `LIDSelectList` (p. 34)

| Element.Attribute | Meaning | Type | Req |
|---|---|---|---|
| UIDSelectList.Size / LIDSelectList.Size | Number of UID/LID entries | int | Always present in the sample, even with one entry (INFERRED: write it) |
| UID.IntValue / LID.IntValue | ID of an upper / lower ToolSetup that the step uses. With one tool, it equals the ID on the ToolSetup. With several tools, there is one entry each. | int | — |

A step may use more than one ToolSetup only if they share the same tool name and geometry (p. 34).

### 2.5 `ProgOutputs` / `ProgOutData` (pp. 34-35)

There are 8 possible programmable outputs per step (p. 34).

| Attribute | Meaning | Type | Default | Values |
|---|---|---|---|---|
| Active | Output off (0) or on (1) | bool | UNKNOWN (bare element = inactive, INFERRED) | 0, 1 |
| FromType | Point in the stroke where the output turns on, or the output's function | int | **0** | 0 PO_TOP top of stroke; 1 PO_DN_SPD down speed change; 2 PO_CLAMP clamp position; 3 PO_REV reversal; 4 PO_UP_SPD up speed change; 5 PO_END end of stroke; 6 PO_DN_POS position on the down stroke; 7 PO_UP_POS position on the up stroke. Functions that use no "to" code: 8 POUT_UP_LIMIT_SW up limit switch function; 9 POUT_PART_COUNTER part counter pulse (counts the move from last step to first); 10 POUT_STROKE_COUNTER stroke counter (counts reversals); 11 POUT_FAULT_PRESENT on while a fault message is shown; 12 POUT_READY_TO_BEND on when gauges are in position and waiting for the footswitch; 13 POUT_SETUP_MODE on in setup mode; 14 POUT_GAGE_CMD_SENT 2-second pulse when a gauge command has been ... (**PARTIAL**: the sentence breaks off at the top of p. 35) |
| ToType | Point in the stroke where the output turns off | int | **5** | same 0-7 codes |
| FromValue, ToValue | Position, when FromType/ToType is a position code (6 or 7) | float, in | UNKNOWN | — |

Sample: step 2, slot 8 has `FromType="2" ToType="7"` but no `ToValue`, so an omitted value takes
its default (UNKNOWN).

---

## 3. Required vs optional (p. 26, plus per-attribute notes)

- The manual's only general rule is that a tag not in the file gets the control's default (p. 26).
  It never marks an attribute "required", except that `ProgName` is explicitly "not required"
  (p. 29).
- Structurally required, as stated: the prolog, `<Document>` root, and `<PressBrakeProgram>` as the
  next element (p. 29). A program always has at least one Step (p. 31).
- Defaults are stated only for: MatThick 0.060, MatType 0, KFactorAuto 1, KFactor 0.330,
  RZEnabled 1, FEnabled 1, finger types -1, finger names NULL, NumSteps 1, Repeats 1, Dwell 0,
  GageAllow 0, Retract 0, XLeft/XRight max travel − 1.0, RLeft/RRight 0.050, ZLeft -4.000,
  ZRight 4.000, FingerLen 0.0, ProgCrown 0.0, ReturnSpd 20, MuteOffset 0.250, ClampAdj 0.0,
  FlgOffLeft/Right 0.0, FromType 0, ToType 5. All other defaults are UNKNOWN.
- Minimal set a writer should always emit for N air bends (INFERRED, not stated): Version,
  MatThick, MatType, NumSteps; both ToolSets with Count, Name, ID, Length, Location (and
  NumSegs/SegStackup if segmented); per Step: RevMode=0, RevAngle, BendLen, StrokeLen, UpperID,
  LowerID, XLeft, XRight (and R/Z if the gauge has them), UIDSelectList and LIDSelectList, and
  ProgOutputs with 8 ProgOutData.

---

## 4. Stations, steps and axis datums

### Tool stations (pp. 30-31, 34)

- Each `ToolSetup` is one station: one tool instance placed along the bed. `Name` links to the
  tool's geometry in the Tool Library (Upper/Lower Tool files, pp. 35-37). `ID` is the instance
  handle. `Length` is the station length, and `Location` is its position along the bed.
- **Location datum:** 0.0 is the centre of the machine. Negative is left and positive is right,
  seen from the front of the press brake (p. 31). Whether Location is the station's **centre** or
  an end is not stated. INFERRED: centre. In sample step 1 the Z fingers sit symmetrically about
  the lower `ID=2` Location: -32.9528 − 11 = -43.9528 and -32.9528 + 11 = -21.9528, for a 24-long
  tool (p. 28).
- Upper and lower stations are separate lists with their own IDs. In the sample, the upper and lower
  Locations of the paired stations differ slightly (-33.1169 vs -32.9528; 18.3117 vs 18.1169). The
  pairing is by ID and by the step's references, not by equal Location. Why they differ is
  UNKNOWN.
- A segmented station lists its segment stack (`SegStackup/SegEntry SegValue`), with `NumSegs`
  segments that sum to `Length` (INFERRED).

### Steps to stations (pp. 32, 34)

- `Step@UpperID` / `@LowerID` name the primary upper / lower `ToolSetup@ID`.
- `UIDSelectList` / `LIDSelectList` list **every** station the step uses. This can be several
  stations, but only if they have the same tool name and geometry. With one station, the list's one
  entry equals the ToolSetup ID. In sample step 2, UpperID=3 and LowerID=3, and both lists hold
  {1, 3}. Both stations are tool 40002 / 42002-315, so the step bends across stations 1 and 3 at
  once. UpperID/LowerID equals one of the listed IDs. Which one (first, last, nearest the gauge) is
  UNKNOWN. In the sample it is the last (3).

### Axis values and datums (pp. 16-17, 20, 31-33)

- **X** (`XLeft`/`XRight`): the "position" of the X-axis arm (p. 33). Its datum is **not stated**.
  The manual says GageAllow is added to the "Flange Position" (p. 33), and the Run/Edit screen
  shows "FLANGE DIMENSION 2.000" next to "GAGE ALLOWANCE 0.057" for sample step 1 (p. 26
  screenshot; XLeft=2, GageAllow 0.057 in the file). So X appears to be stored as the **flange
  dimension**, with the control adding GageAllow and, for 3-axis gauges, FingerLen. The reference
  line (bend line / punch tip centreline) is INFERRED. The manual gives no X datum in absolute
  machine coordinates (UNKNOWN). In sample step 1 XLeft=2 and XRight=2.5, under GageMode 1
  (independent).
- **R** (`RLeft`/`RRight`): vertical position of the R-axis arm. The default is 0.050", and the
  "R-Axis Default Position" preference is a value in inches (p. 20). The datum is not stated.
  UNKNOWN. Likely the die top / bed, given the small default.
- **Z** (`ZLeft`/`ZRight`): lateral finger position. INFERRED to share Location's frame (machine
  centre = 0, + right): (a) in sample step 1 the Z values equal Location ± 11; (b) the 6XLT Z
  limit table spans negative to positive about the centre (Z-Left Min -27.75 ... Max 23.25 for 6 ft,
  p. 16); (c) "Cntr To Pivot Z" is measured from the machine centre between the housings (p. 17).
  The defaults -4 / +4 also suit a centre datum. Sample step 2's Z values (0.1214 / 10.1214) do not
  follow obviously from the stated Locations, because tool 1's Location is omitted. UNKNOWN how they
  were chosen.
- **F**, **SS** (front gauge, sheet supports): "position" only. Datum UNKNOWN.
- **GageMode** (p. 32): 0 standard (presumably left and right X/R move together, INFERRED);
  1 independent (left and right X can differ, as in sample step 1); 2 taper.
- **Ram positions:** `RevPos`, `SpdChgDwn`, `SpdChgUp`, `StrokeLen` and the clamp position are
  relative to **top of material** (pp. 31-32). `RevAbsPos` is absolute (datum UNKNOWN). The
  conversion notes (pp. 8-9) confirm that newer controls use positions relative to top of material,
  where the older CBII/FM controls used absolute ones. `ToolSetup@Unmeasured=1` forces every ram
  position to be absolute (p. 31).

---

## 5. Sample program outline (pp. 27-28)

The sample is a two-step program (illustrated by the Run/Edit screenshot, p. 26).

```
Document
  PressBrakeProgram Version=9 ProgName="C:\PressBrake\Program\samplexml.pgm" TeachName="" PartName=""
      SetupNotes="SAMPLE PROGRAM THAT SHOWS ALL PROGRAM FIELDS" MatThick=0.075 MatType=1 RZEnabled=1
      FEnabled=0 LeftFingerType=19 RightFingerType=20 LUFingerName/LLFingerName/RUFingerName/
      RLFingerName="5 Axis Step Finger" HasPart=0 NumSteps=2
    UpperToolSets / ToolList Count=3
      ToolSetup Name=40006 ID=2 Length=24     Location=-33.1169            (unsegmented; unclosed in print)
      ToolSetup Name=40002 ID=1 Length=23.905 NumSegs=3                    (no Location)
        SegStackup: SegValue 18, 3.937, 1.968
      ToolSetup Name=40002 ID=3 Length=7.874  Location=18.3117 NumSegs=1
        SegStackup: SegValue 7.874
    LowerToolSets / ToolList Count=3
      ToolSetup Name=42002-472 ID=2 Length=24     Location=-32.9528        (unsegmented; unclosed in print)
      ToolSetup Name=42002-315 ID=1 Length=23.622 NumSegs=3                (no Location)
        SegStackup: SegValue 11.811, 7.874, 3.937
      ToolSetup Name=42002-315 ID=3 Length=7.874  Location=18.1169 NumSegs=1
        SegStackup: SegValue 7.874
    StepData
      Step 1: Repeats=2 RevAngle=135 RevTons=0.9 RevAbsPos=15 AngleAdj=2 BendLen=24 StrokeLen=2
              UpperID=2 LowerID=2 SpdChgDwn=0.125 SpdChgUp=0.125 Tilt=0.0001 Dwell=1 DownStop=1
              UpStop=1 FormSpeed=40 ProgFoot=1 PartOrient=5 CadStep=0 GageMode=1 GageAllow=0.057
              Retract=1 GagePause=0.2 XLeft=2 XRight=2.5 RLeft=-0.05 RRight=-0.05
              ZLeft=-43.9528 ZRight=-21.9528 FLeft=44 FRight=44 SSLeft=3 SSRight=3
              ProgCrown=0.001 ReturnSpd=120 ClampAdj=-0.01 ClampMode=1 AdaptRevPos=0 FingerSel=2
              FlgOffRight=0.15 FlgOffLeft=0.1 ProgPalm=2
              (no RevMode -> default, presumably angle)
        UIDSelectList Size=1: UID IntValue=2
        LIDSelectList Size=1: LID IntValue=2
        ProgOutputs: [1] Active=1 FromType=1 ToType=4 (down speed change -> up speed change); [2..8] empty
      Step 2: RevMode=1 (tonnage) RevTons=10 RevAbsPos=15 StrokeLen=1 UpperID=3 LowerID=3
              FormSpeed=20 CadStep=0 XLeft=24 XRight=24 ZLeft=0.1214 ZRight=10.1214
              FLeft=44 FRight=44 SSLeft=3 SSRight=3 ClampMode=2 AdaptRevPos=0
              (no BendLen, no RevAngle)
        UIDSelectList Size=2: UID 1, 3
        LIDSelectList Size=2: LID 1, 3
        ProgOutputs: [1] Active=1 FromType=1 ToType=4; [2..7] empty;
                     [8] Active=1 FromType=2 ToType=7 (clamp -> position on upstroke; no ToValue)
```

Observations: the sample writes values that do not apply to the chosen mode, such as RevTons and
RevAbsPos in an angle-mode step and RevAbsPos in a tonnage step. The control evidently tolerates
these. PartOrient=5 = 0b0000_0101: turns nibble 5, flips 0. What the bit values mean is UNKNOWN.

---

## 6. Ambiguities, gaps and UNKNOWNs

1. **No schema.** No XSD or DTD is given or referenced. The only definition is the sample plus the
   prose tables (pp. 27-35).
2. **`Version`** is an internal format-revision counter (p. 29). Valid values, what each adds, and
   how older or newer controls treat a different value are UNKNOWN. The sample is `9` (software
   4.6.0.0, 2009). The Upper Tool file's Version is `4` and Feature3D's is `5` (p. 35).
3. **Name mismatches between tables and sample:** `UpperId`/`LowerId` (text) vs `UpperID`/`LowerID`
   (sample); `FormSpd` (text) vs `FormSpeed` (sample); `UpSto` (typo); `RUFIngerName` and a
   duplicated `LUFingername` in the heading vs `RUFingerName`/`LLFingerName` in the sample. Whether
   the parser is case-sensitive is UNKNOWN. Use the **sample's** spelling, since it was produced by
   the software.
4. **`GagePause`** appears only in the sample. Meaning, unit and default are UNKNOWN.
5. **Undocumented in the sample but in the tables:** KFactorAuto, KFactor, RevPos, FingerLen,
   GuardMute, MuteOffset, FingGagePt, Reversed, Unmeasured, StackedHolderType, HolderHeight,
   FromValue, ToValue. Their exact placement in the attribute order is UNKNOWN. Attribute order
   does not matter in XML, but whether this parser cares is UNKNOWN.
6. **Default conflicts:** finger type default "-1" vs the enum's FINGER_UNDEFINED = 0 (p. 30). The
   RLeft/RRight default is 0.050 but the sample uses -0.05 (p. 33 vs p. 28).
7. **StackedHolderType** gives 20 for both EURO_STACKING_HOLDER and STACKING_RAIL (p. 31). They are
   told apart only by upper vs lower.
8. **Location reference point** (tool centre vs end) is not stated. Centre is INFERRED.
   **ToolSetup without Location** (sample `ID="1"`): default UNKNOWN (0 = centred likely).
9. **X / R / F / SS datums** are not given in the XML section or the configuration tabs read
   (pp. 16-17, 20). X = flange dimension is INFERRED (p. 26 screenshot). The Z frame = Location
   frame is INFERRED. The **RevAbsPos** datum is UNKNOWN.
10. **RevTons** tonnage unit (US short tons vs metric) is UNKNOWN. Tons is stated without
    qualification (p. 31).
11. **PartOrient** bit meanings within each nibble are UNKNOWN (p. 32).
12. **DownStop/UpStop:** "0 or 1 (On or Off)" (p. 32) could be read as 0 = On. 1 = On is assumed
    from boolean convention and the sample (both 1 on a step that also sets Dwell=1). Treat as
    UNCERTAIN.
13. **POUT_GAGE_CMD_SENT** description is cut short (PARTIAL, p. 35).
14. **ProgOutData slot semantics:** that slot k maps to physical output k, and that all 8 must be
    written, is INFERRED from the sample.
15. **Encoding / line endings:** the prolog has no `encoding`, so UTF-8 by XML default (p. 29).
    Whether the reader accepts non-ASCII, CRLF or LF, a BOM, or `encoding="UTF-8"` is UNKNOWN.
    Safest: ASCII content, the exact `<?xml version="1.0" ?>` prolog, no BOM.
16. **The sample is malformed** (two unclosed `ToolSetup` elements, p. 27). Whether the real parser
    is lenient is UNKNOWN. A writer should emit well-formed XML.
17. **Currency:** the manual describes the 2009 software (4.6.0.0) for AUTOFORM-era controls.
    Whether current Cincinnati controls or software read this format is UNKNOWN.
18. **Lower Tool format** starts at the foot of p. 37 (`<LowerTool Version="4" ToolName="42000-15"
    Type="10" Height="1.811" Clearance="0.138" ...`) and continues past the requested range. It is
    not covered here. The Upper Tool format (pp. 35-37) is summarised in the appendix, for
    completeness.

---

## Appendix: Upper Tool file (pp. 35-37), brief

Root `Document` > `UpperTool` with attributes Version (sample 4), ToolName, Type, Length (default
24"), Height, Clearance, MaxLoad (tons/ft), Angle (deg), Radius, Offset, CADA-CADG (per-type
dimension floats, written as "CADA-CADG"; exact attribute names UNKNOWN), DieHolder (bool), and
SegCount (number of ToolSeg entries).

Type values: 0 UPPER_VEE_1; 1 UPPER_VEE_2; 2 UPPER_VEE_3; 3 UPPER_VEE_4; 4 UPPER_OFFSET;
5 UPPER_RADIUS; 6 UPPER_GOOSE_1; 7 UPPER_GOOSE_2; 8 UPPER_FLAT; 9 UPPER_OTHER; 10 UPPER_CUSTOM
(complete, p. 36).

Children:
- `SegmentList` > `ToolSeg` with `Length` (in) and `Quantity` (int; omitted in the sample's first
  entry, so presumably 1). These are the available segment lengths and counts.
- `Feature3D` with `Version` (5) and `SegCount` (22) > `SegmentList` > `Segment3D` with XVal, YVal
  and ZVal (in), plus XCent, YCent and WType on arc segments (WType 1 and 2 occur; meaning,
  presumably arc direction, is UNKNOWN). This is the punch profile. Feature3D is created
  automatically when a tool is imported from a CAD drawing (p. 37).

In the sample the profile's Y runs from +1.151 at the top of the tang down to -2.6315 at the tip.
Y = 0 is at the shoulder, so the 2.634 `Height` is shoulder to tip and excludes the tang. The axis
convention is INFERRED: Y up, X across the tool, origin on the shoulder at the tip's centreline.
