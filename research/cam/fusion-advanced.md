# Fusion advanced CAM: multi-axis, turning, probing, additive, simulation, automation

Research only, read 2026-11-23. Reference product: Autodesk Fusion (Manufacture workspace, with
the paid Fusion Manufacturing Extension, "MfgExt" below). Comparisons with hyperMILL, NX CAM,
PowerMill, Mastercam and ESPRIT EDGE are included where they show what a new entrant would be
measured against. Sibling file `commercial.md` covers the base toolpath list and the vendors in
general; this file goes into the advanced areas in depth.

Source tags:
- **[R]** I fetched and read the page myself on 2026-11-23.
- **[S]** I saw only a search-engine snippet or summary on 2026-11-23, not the page.
- **[K]** Background knowledge I did not re-read this session.

Each claim cites a source ID (`F12`, `P3`...). The IDs resolve in the source list at the end.
Everything here is my own summary of the sources. Nothing is quoted at length.

Priority tags for a new entrant (forgeCAD) are on the last line of each subsection:
- **MUST**: needed for credible CAM on day one of the relevant market.
- **DIFF**: a differentiator, where exact geometry and proofs could beat the incumbents.
- **LATER**: worth doing only after the core works.

---

## 0. Summary of what matters for forgeCAD

1. **Fusion's accuracy model is triangulated.** Its milling reference page defines machining
   tolerance as toolpath-generation tolerance plus geometry-triangulation tolerance, and any
   smoothing or filter tolerance comes on top of that [F40 R]. The DED reference says the same
   [F33 R]. An exact B-rep kernel can offer a tolerance that is actually certified against the
   true surface. No incumbent documents such a guarantee.
2. **Fusion gates most advanced capability behind the MfgExt.** That covers simultaneous
   multi-axis (Swarf, Advanced Swarf, Multi-Axis Contour/Finishing/Roughing, Rotary, Geodesic,
   Deburr), collision-avoidance tilting, tool-axis smoothing and limits, Probe Geometry, Inspect
   Surface, part alignment, Live Connection, Hole Recognition, DED, MPBF and Leads & Links
   editing [F1-F20, F23-F27, F31-F35, F45 R]. Probe WCS, manual inspection, wrap and indexed 3+2
   are base features [F25, F7 R].
3. **Fusion's tool-axis control is a small, clean set.** Primary mode is one of Vertical,
   Lead/Lean, From/To Point or From/To Curve. On top of that come Collision Avoidance (Automatic,
   or toward/away from a point or curve), Smoothing (distance and angle), Limits (min/max tilt,
   then trim or hold at the limit) and an Override Tilt Angle cone [F2-F6 R]. PowerMill
   (Autodesk's high-end product) adds Lines, Fixed Direction, Blisk, Automatic swarf rulings, a
   Fixed-angle lock and "Lead then Lean" fallbacks [P1, P2 R]. hyperMILL adds automatic
   indexing (it splits a path into 3+2 segments), prioritising a rotary axis by machine
   kinematics, and predicting tool length [H1, H2 R].
4. **Fusion does not simulate posted G-code natively.** Its simulation is driven by toolpath
   data, with machine motion from the machine definition plus the post's kinematics [F36 R].
   G-code-driven verification at Autodesk is CAMplete (TruePath/TurnMill), which Autodesk says
   it is migrating into Fusion and is phasing out as a separate product [F48 S]. Vericut and
   NCSIMUL remain the independent standard [V1 R, V2 S].
5. **Mill-turn in Fusion is thin compared with ESPRIT and NX.** Fusion has 12 turning strategies
   plus three part-handling ops (Subspindle Grab, Subspindle Return, Bar Pull), live-tooling and
   turret items in the kinematics tree, and ISO insert codes [F21-F30 R]. I found no
   multi-channel synchronisation. NX has a Synchronization Manager [N2 R], and ESPRIT EDGE
   markets multi-channel sync and a full machine digital twin [E1 S].
6. **AI CAM in late 2026 is mostly assistive.** Fusion's "Autodesk Assistant" is
   conversational, and Autodesk describes "Instant Prismatic Machining" as exploratory
   [F50, F51 S]. CloudNC CAM Assist (3-axis and 3+2, inside Fusion, NX, Mastercam and others)
   [C1 R], Toolpath.com (Fusion) [C2 R], NX AI Make Machining Suggestion (2512, generative in
   2606) [N1 R], Mastercam Copilot [M1 R] and ESPRIT ProPlanAI [E2 S] are the comparators. All
   of them produce editable native operations. None claims verified output.
7. **Fusion's CAM API is complete enough for headless pipelines.** From Python or C++ a script
   can create setups and operations by strategy name, set any parameter by name, apply
   templates, generate asynchronously, post-process, and emit setup sheets [F55-F57 R]. That
   is the bar for a scriptable entrant, and forgeCAD's typed-op CLI/MCP model already exceeds it
   in spirit.

---

## 1. Multi-axis milling

### 1.1 3+2 positional: tool orientation and WCS per operation

- **Setup WCS.** The setup chooses its orientation from the model, from Z plus X, Z plus Y,
  X plus Y, or from a user coordinate system. The origin is the model origin, a selected point,
  a stock-box point or a model-box point. Any axis can be flipped. An "accessibility analysis"
  shows what the current orientation can reach, and fixtures (vise, clamps, chuck) are tagged
  so stock simulation can flag collisions with them [F7 R].
- **Per-operation tool orientation.** Each operation can override the setup's tool orientation
  [K]. In posted code, however, the coordinates normally stay relative to the setup WCS unless
  the post uses a dynamic work offset (or the post property `mapWorkOrigin` is set). Autodesk's
  support article calls this expected behaviour [F8 S], and forum users confirm that a "new
  origin" inside the tool orientation is ignored [F9 S]. This is a known source of confusion.
- **Additive ops** also carry a per-operation Tool Orientation and Model override [F33 R],
  which shows the pattern is general across operation types.
- **hyperMILL** documents indexed machining with workplane transformations, a "fixed 3+2" mode
  in which areas are checked so they neither overlap nor leave gaps, and *automatic indexing*.
  Automatic indexing finds fixed inclinations by itself and splits a path wherever the holder
  would collide [H1 R]. hyperMILL 2025 added an automatic mode that pre-analyses a whole path
  and mixes simultaneous and indexed segments [H3 S]. I found no equivalent automatic
  indexing in Fusion's help.

MUST: 3+2 with a per-op frame, emitted as a proper tilted work plane (CYCLE800/G68.2/PLANE SPATIAL) and not left to post tricks.
DIFF: automatic indexing, i.e. choosing the fewest orientations that cover the reachable faces, which is an exact visibility problem.

### 1.2 4-axis: wrap and rotary

| Fusion feature | What it does | Tier |
|---|---|---|
| Wrap (in 2D Adaptive, 2D Pocket/Offset Rough, 2D Contour) | Unwraps the geometry onto a true cylinder, plans the 2D path, then re-wraps it at the cylinder radius. Cones and freeform faces are refused. An Offset nudges the path. Walls tapered across X cannot be made correctly | base [F10 R] |
| Rotary Parallel | Finishing with the 4th axis. Styles are Spiral (continuous, with closing circles), Line (indexed between passes) and Circular. Range is 360 degrees or limited | MfgExt [F11 S, F12 R] |
| Rotary Roughing (formerly Rotary Pocket) | Bulk roughing on rotary parts. Automatic stepdown lines up with cylindrical faces. A Tool Offset keeps a ball nose off its centre | MfgExt [F13 R] |
| Rotary Contour | Finishing steep walls on rotary parts | MfgExt [F11 S] |
| Rotary Parallel collision avoidance | Tilts the tool about the rotary axis to clear the shaft and holder (clearances set on the Tool tab). Where 4-axis motion cannot clear, that section is trimmed | MfgExt [F12 R] |

MUST (4-axis wrap plus indexed rotary). LATER (simultaneous rotary roughing and finishing).

### 1.3 Simultaneous 5-axis strategies in Fusion (all MfgExt)

| Strategy | Inputs and behaviour | Source |
|---|---|---|
| Swarf | Cuts with the flank on chamfers, bevels and tapered walls that the side of the tool can lie on. Drive Mode is Surfaces or Contours, and with Contours the first contour sets the tip side. A Tool Offset on the Passes tab shifts contact up the flute to sharper, less-worn sections. An up/down smoothing setting exists because the path sometimes has to bob to avoid gouging | [F14 R, F15 S] |
| Advanced Swarf (flank milling) | Upper and lower guide curves, picked directly or derived from selected surfaces (recommended). Optional extra guide curves steer the tool axis. Floor surfaces keep the tool from cutting through, and "Tool Tracks Floor" follows curved floors. Upper and lower curves are synchronised manually or automatically. Single or multiple cuts using the full flute. Output can be 3-, 4- or 5-axis | [F16 R] |
| Multi-Axis Contour | Follows 3D curves (model edges or curves projected onto a face). The tool defaults to normal to the underlying surface, and the Multi-Axis tab tunes it | [F17 R] |
| Multi-Axis Finishing | Finishes walls and pocket floors with circle-segment (barrel) tools for large stepovers. The contact point on the barrel is picked automatically within a user min/max range, and a wider range gives smoother rotary motion and reaches tighter corners. Walls-only machining also needs floor surfaces, whose normal sets the entry side | [F18 R] |
| Multi-Axis Roughing (formerly Multi-Axis Clearing) | Clears pockets with curved floors by offsetting from the floor surface, not by Z slices. A ceiling surface is optional, and if none is given, large openings are capped automatically | [F19 R] |
| Geodesic | Finishes freeform faces and undercuts over "drive surfaces" (which must be joined). Pass types are Blend (variable stepover between two curves, for fillets and chamfers) and Scallop (constant stepover). Offsets start from the boundary, the drive-surface boundary, a centre circle or line, or chosen curves. 3-, 4- or 5-axis. Ball tool recommended | [F20 S] |
| Deburr | Finds sharp or rounded external edges (rounded ones only if opted in). Edges can be excluded, including those near fixtures or clamps. Cuts a chamfer or a fillet, in multiple passes for fillets and big chamfers. Removes redundant points. Lollipop tool shown. 3-, 4- or 5-axis. The 4-axis tilt mode is "to rotary axis" or "zero lead angle" | [F23 R, F24 S] |
| Steep and Shallow, 5-axis mode | The 3D Steep and Shallow finishing strategy (contour on steep areas, parallel or scallop on shallow) becomes 5-axis when the Multi-Axis tab's Primary Mode is non-vertical. It is the only strategy that offers Lead/Lean, and it can machine undercuts | [F2, F3 R, F22 S] |
| Multi-axis on 3D strategies | Autodesk added the Multi-Axis tab (primary mode, collision avoidance, limits) to many 3D finishing strategies, making them tiltable | [F21 S] |

MUST (Swarf, Multi-Axis Contour, a tiltable 3D finisher). DIFF (barrel-cutter tangent-plane finishing and flank milling with *proved* deviation bounds, since flank error is a ruled-surface approximation problem). LATER (Geodesic, multi-axis roughing).

### 1.4 Tool-axis control (Fusion's Multi-Axis tab)

| Control | Options and semantics | Source |
|---|---|---|
| Machining Type | 3-axis, 4-axis or 5-axis | [F4 R] |
| Primary Tool Axis Mode | **Vertical**: along the tool orientation. **Lead and Lean** (Steep and Shallow only): lead/lag in the travel direction (for example +15 lead, -15 lag), then lean sideways. At 0/0 the tool stays normal to the surface. **From Point / To Point**: the tip tilts away from or toward a point (sketched or model). **From Curve / To Curve**: the same with a guiding curve or line | [F2 R] |
| Override Tilt Angle | A Preferred Angle measured from the Tool Axis Reference (tool-orientation Z or setup Z). It constrains the axis to a cone of that half-angle and snaps the primary-mode axis to the nearest cone ruling, which effectively locks one rotary axis. Non-vertical primary modes only | [F3 R] |
| Collision Avoidance | **Automatic** (Vertical primary mode only, slower, good for feature-rich parts), **From/To Point**, **From/To Curve**. Shaft Clearance and Holder Clearance are set on the Tool tab. The tool tilts until clear, within the axis limits. Collision-avoidance values override the preferred angle. Requires a machine with more than 3 axes | [F1, F4 R, F21 S] |
| Tool Axis Smoothing | Smoothing Distance (before and after the event where the rotaries must move) and Smoothing Angle (maximum deviation from the primary-mode axis during the transition). It exists to avoid dwell marks caused by sudden axis flips. Available only when tilting occurs | [F5 R] |
| Tool Axis Limits | Minimum and Maximum Tilt. "Areas beyond limit" are either Trim Toolpath or Machine At Tilt Limit | [F6 R] |
| 3D-strategy holder handling (3-axis) | Shaft & Holder modes: Disabled, Pull away, Trimmed, Detect tool length, plus Fail on collision | [F40 R] |

There is no "To Line" option. The question's "Tool axis: from point / to line / lead-lag"
list matches **PowerMill**, not Fusion. PowerMill's Tool axis definitions are Vertical,
Lead/Lean, Towards/From point, Towards/From line, Towards/From curve, Fixed Direction (an IJK
vector, searched in both directions to handle undercuts), Blisk (hub, shroud or radial
elevation) and Automatic (following swarf rulings), plus a Fixed-angle lock on azimuth or
elevation [P2 R]. PowerMill's collision-avoidance tilt methods are Lean, Lead, Lead-then-Lean,
Lean-then-Lead, Specified direction (towards/from line or curve, surface normal, fixed
direction, blisk around or between blades) and Automatic (Vertical only). It offers holder and
shank clearance, a smoothing distance, and "keep colliding moves" [P1, P2 R]. NX's
variable-axis enum includes Normal to Part, Normal to Drive, Relative to Drive, Interpolate
Vector/Angle, Swarf Drive, Optimized to Drive and others, with lead and tilt angles and an
ApplySmoothing flag [N3 S].

MUST: Vertical, Lead/Lean, To/From point and curve, tilt limits, and smoothing.
DIFF: exact, certified collision clearance, i.e. the minimum distance from holder and shank to the B-rep as a proved bound rather than a mesh test.

### 1.5 Automatic collision avoidance: tilt, retract, trim

- **Fusion** tilts away (automatically, or guided by a point or curve) until the shaft and
  holder clear by their clearances. Anything still colliding at the tilt limit is trimmed or
  held at the limit [F1, F4, F6 R]. On 4-axis, tilting happens about the rotary axis and the
  rest is trimmed [F12 R]. Simulation reports the residual collisions [F36 R].
- **hyperMILL** computes a collision-free angle automatically. If tilting fails, it drops the
  colliding segments and continues with a longer tool or a changed angle. The rotary axis to
  prioritise is user-chosen per machine kinematics. It predicts tool extension (minimum safe
  stick-out), and in roughing it shifts paths sideways [H2 R].
- **PowerMill**: see 1.4. Two-stage fallbacks and "keep colliding moves" [P1 R].
- **Mastercam 2026.R2** extended automatic tilting collision control to bull-nose tools [M1 R].

MUST: tilt-until-clear plus trim. DIFF: tool-length prediction and the exact minimum stick-out.

### 1.6 Multi-axis linking, rewinds, retract-and-reconfigure

- Fusion's Linking tab has Retraction Policy (Full, Minimum, Shortest path), Safe Distance,
  Maximum Stay-Down Distance and Radial Safe Distance. These appear on the DED page and the
  Leads-and-Links page [F33, F45 R, F46 S].
- **Leads and Links** (MfgExt) edits the leads and links of an existing toolpath after the
  fact: stay-down, radial safe distance and clearance geometry, applied to all moves or a
  selection [F45 R].
- **Rewinds.** When a rotary axis hits its travel limit mid-path, the machine definition's
  Multi-axis tab offers "Retract and reconfigure" (Autodesk rates it high severity). Without it,
  posting fails with the "rewind ... has been disabled" error. Each rotary axis also has a Reset
  rule for its start position and for rewinding [F38, F39 R]. The 2025 API exposes safe-retract
  distance and feedrate along the tool axis for rewinds [F47 S; the page itself returned 404].
- Machine-definition tips also flag home positions for safe retract, rotary ranges, the
  feed-output tolerance and the feedrate method (inverse-time and similar) [F39 R].

MUST: rewind handling and a rotary-range solution choice. DIFF: a provably collision-free retract corridor.

### 1.7 Beyond Fusion (hyperMILL, NX, PowerMill)

| Capability | Who | Notes |
|---|---|---|
| Automatic indexing (path split into 3+2 segments) | hyperMILL | [H1 R, H3 S] |
| Rotary axis priority for avoidance, by kinematics | hyperMILL | [H2 R] |
| Tool-length prediction | hyperMILL | [H2 R] |
| 5-axis tangent-plane machining with conical barrel cutters | hyperMILL MAXX | Barrel radii of 250-1500 mm allow huge stepovers. Contact point is controlled continuously. The vendor claims up to 90% less finishing time [H4 S] |
| 5-axis rest machining, 5-axis Z-level, shape-offset roughing | hyperMILL | [H1 R] |
| Application packages (impeller/blisk, turbine blade, tube, tyre, blow mould) | hyperMILL, PowerMill (Blisk axis), NX | [H1, P2 R] |
| True Shape: paths computed directly from CAD surfaces | hyperMILL | [H1 R]. Interesting because it is a claim of non-tessellated computation |
| Fixed-angle lock, Lead-then-Lean fallback, Blisk elevation modes | PowerMill | [P1, P2 R] |
| Many variable-axis drive types | NX | [N3 S] |

DIFF: barrel tangent-plane machining with exact contact-point math. LATER: application packages.

---

## 2. Turning and mill-turn

### 2.1 Fusion turning strategies

From the turning operations overview [F25t R] and the individual pages:

| Strategy | Notes | Source |
|---|---|---|
| Turning Face | Faces the end flat and perpendicular to the spindle axis, often used as the Z reference | [F26 S] |
| Turning Profile Roughing / Profile Finishing | The old combined "Turning Profile" was split in two. The legacy op can still be edited | [F27 S] |
| Turning Adaptive Roughing | Small overlapping passes in both directions, typically with indexable grooving inserts. OD or ID modes. Finishing allowance in X and Z. Groove Suppression skips or shallows grooves to cut force | [F28 R] |
| Turning Groove Roughing / Groove Finishing / Turning Groove | OD, ID and face grooving tools | [F25t R, F27 S] |
| Turning Single Groove | A groove at picked positions, for example a thread relief | [F27 S] |
| Turning Thread | OD and ID, cylindrical and conical. Needs spindle-synchronised feed. The insert profile angle is no longer fixed at 60 degrees | [F27 S] |
| Turning Chamfer | Chamfers undesigned sharp edges, with extension, width and angle | [F26 S] |
| Turning Part | Cut-off from bar | [F26 S] |
| Turning Trace | Follows a traced path | [F25t R] |
| Common features | Retract and approach in X; turning on the negative diameter | [F29 R] |

Turning setup: Operation Type "Turning or mill/turn". The WCS is picked from a cylindrical face
on the centreline, with Z pointing away from the front face [F30 R].

### 2.2 Part handling (sub-spindle)

- **Subspindle Grab** moves the sub-spindle to the part and clamps it, with an approach
  feedrate, a feed plane and a chuck plane [F31 R, F31b S].
- **Subspindle Return** sends the sub-spindle home after the main spindle unclamps. It usually
  follows Turning Part, and can optionally stop the spindle [F31 R].
- **Bar Pull** has a pull distance and two modes. *Compensate toolpaths* shifts every later Z
  by the pull distance. *New WCS offset* needs a second work offset exactly one pull distance
  away. The turret-clamp variant may need the main spindle stopped, and a dwell is available
  [F32 R, F31b S].
- Part handling creates no toolpath. The post must emit the codes, and machine-specific post
  edits are often needed [F31 R].

### 2.3 Tools and inserts

- Insert fields: ISO code (auto-fills the rest; mm for ISO, inch for ANSI), shape letter,
  relief angle, tolerance letter, cross-section (hole and chipbreaker), IC size, thickness and
  corner-radius codes, material, groove width, thread pitch range, internal flag, thread
  profile angle (<90), and tip type (flat, point or round) with its radius or width [F34 R].
- Holder fields: style, hand, clamping, cutting width, head length, overall length, shank
  width and height, round shank [F34 R]. Forum history shows some ISO holder styles missing
  and no custom-profile turning tools (workaround: a round insert) [F35 S, older].

### 2.4 Live tooling, Y-axis and kinematics

- The kinematics tree has axes (linear X/Y/Z or rotary A/B/C, with home, rapid and max feed,
  orientation, preference, range, TCP, resolution, axis function such as a "supportive"
  sub-spindle, offsets and reset), heads (spindle or stationary, speed range, max tool),
  turrets (stations, attach points, Live Tooling flag and speeds) and spindles (speed, power,
  peak torque, Reverse Z for the sub-spindle) [F37 R].
- Y-axis and live-tool milling on lathes depend heavily on the post. Forum threads show users
  fighting orientation and XZC polar modes [F36f S].

### 2.5 ESPRIT and NX comparison

| | Fusion | NX CAM | ESPRIT EDGE |
|---|---|---|---|
| Multi-channel synchronisation | Not found in help | Synchronization Manager: one column per channel, drag sync events, posts per channel in the background. Effectively unlimited channels [N2 R, N4 S] | Multi-channel programming with sync, superimposed motion, Swiss support (Citizen L, Star SR) [E1 S] |
| Machine digital twin | Kinematics tree plus optional model [F37 R] | ISV (Integrated Simulation and Verification) [K] | Full twin including collet, guide bushing and tool blocks. Analytics flag axis limits, acceleration and channel conflicts [E1 S] |
| Adaptive turning | Turning Adaptive Roughing [F28 R] | Rough Turn Adaptive, new in 2606 [N1 R] | [K] |
| AI planning for turning | No | AI suggestions (milling-focused) [N1 R] | ProPlanAI extended to turning, mill-turn and Swiss in April 2026 [E2 S] |

MUST: face, profile rough/finish, groove, thread, part-off, ISO inserts.
LATER: sub-spindle part handling, live tooling and Y-axis.
DIFF: none obvious, since turning is 2D and exactness buys little except exact profile offsets and thread geometry.
LATER (and costly): multi-channel synchronisation.

---

## 3. Probing and inspection

### 3.1 Probe WCS (base Fusion)

- **Cycle families:** single axis, corner, block centre, rectangular cavity centre, circular
  boss centre, circular hole centre, rectangular and circular cavity with islands, and between
  two walls [F41 R, F42 R].
- **Inference from selection.** The number and type of picked faces (Model or Stock mode)
  decide the type: Z surface, angle along X or Y, XY outer or inner corner, X/Y channel (web or
  pocket), X/Y wall, XY circular or rectangular boss, XY circular hole, partial hole or boss,
  and block centre [F41 R].
- **Parameters:** link, lead-in and measure feedrates (measure must match the probe
  calibration feed), approach distance (default twice the probe diameter), spacing for angle
  measurement, overtravel, tolerances (position, upper and lower size, angle), clearance,
  retract and bottom heights with references, WCS override and the WCS to update, stop on
  Out-of-position, Wrong-size or Askew, Print Results (on the control plus an optional results
  file to import), and Increment Component for multi-part runs [F41 R].
- **Order of operations:** angle (rotation) probing should come last when chaining, because
  some controls cannot change offsets while rotation compensation is active [F43 S].

### 3.2 Probe Geometry, Inspect Surface, part alignment (MfgExt)

- **Probe Geometry** measures in-process holes, bosses, surfaces, walls, channels and corners.
  It runs either as controller *probing cycles* (macros, which can update tool wear) or as
  user-defined *probe paths* fitted to circles (position, diameter, circularity), cylinders
  (diameter, cylindricity, axis) or planes (normal tilt, flatness). It can stop the machine when
  out of tolerance, and needs a supporting post [F44 R].
- **Inspect Surface** uses a user-defined probe path on freeform parts with an upper and lower
  tolerance band. Deviation is the offset from nominal to the contact point, and "error" is the
  excess beyond tolerance. Its purpose is to cut CMM trips [F49 R, F49b S].
- **Part alignment** computes a best-fit from surface inspection to compensate for a misplaced
  casting, forging or near-net part, and updates the WCS for the rest of the setup. **Live part
  alignment** does the same over a live connection [F25 R, F49b S].
- **Manual inspection** builds an inspection plan and records caliper, micrometer or height
  gauge readings in Fusion. PMI size tolerances auto-fill, digital gauges can stream readings,
  and indirect measurements such as hole centre-to-centre are supported [F25 R, F50m S].
- **Reports** are PDFs with the results, a title, a canvas snapshot, the document, units and
  tolerance legends. Results files arrive in controller-specific formats (`.mpf .msr .dat .log
  .a .out .bat .txt`) [F51r R].
- **Live Connection** (MfgExt) runs Fusion against a per-control Machine Connector app on a
  Windows PC. Mac is not supported, so the connector must run on another machine. Ethernet or
  serial to the control. Haas needs MDC enabled. IP, port and data-buffer settings live in the
  Machine Definition [F52 R]. Forum posts say connectors now exist for Haas, Fanuc, Heidenhain
  and Siemens [F52b S].

### 3.3 Renishaw cycles in posts

- Fusion does not generate macro bodies. The post *calls* the control's probing macros, so the
  machine must have the macro package installed [F53 S]. The post API exposes
  `isProbingCycle`, `isProbeOperation`, `isInspectionOperation`, `TOOL_PROBE`,
  `COMMAND_PROBE_ON/OFF` and `COMMAND_VERIFY/CALIBRATE` [F54 R].
- Renishaw's basic Inspection software covers single surfaces in X, Y and Z, web/pocket,
  bore/boss, external corner, and single- and two-point measures, with lathe variants, offset
  update and printed results. Inspection Plus is the fuller set [R1 R]. Protected positioning
  (for example a G65 P9810 call) prevents stylus crashes [R2 S].
- NX 2606 added built-in Blum and Renishaw tool-setting cycles, TCL logic in probing notes and
  closed-loop cut-measure-cut with automatic tool correction [N1 R]. Mastercam 2026.R2 added
  Blum Digilog tool-setter support across Fanuc, Siemens, Okuma, Heidenhain, Mazak and Makino
  [M1 R].

MUST: Probe WCS (wall, web, pocket, boss, bore, corner, angle) emitted as macro calls.
DIFF: inspection against the exact B-rep, with deviation computed to the true surface and not to a mesh; that is a natural forgeCAD strength.
LATER: Inspect Surface, part alignment, live connection, manual inspection and PDF reports.

---

## 4. Additive and hybrid

### 4.1 Fusion additive technologies

| Technology | Fusion support | Tier |
|---|---|---|
| FFF | Setup, print settings, supports, toolpath, simulation, statistics, G-code or 3MF | base [F55a R] |
| SLA/DLP | Supports (some need MfgExt), machine file | mixed [F55a R] |
| SLS, MJF | Positioning, 3MF or machine file. MJF sends to HP SmartStream through an add-in | base [F55a R] |
| MPBF (laser) and eBeam | Build strategies, supports, toolpath. MPBF only: *process simulation* predicts distortion and compensates for it | MfgExt [F55a R] |
| Binder jetting | Base-plate and setter supports, 3MF. Part is sintered afterwards | [F55a R] |
| DED / multi-axis deposition | Separate workflow, below | MfgExt [F55b R] |

### 4.2 DED and hybrid

- **Workflow:** simplify the design and add stock, then create an additive setup (machine plus
  starting-stock body), define a depositing tool, generate Feature Construction, animate the
  deposition (volumetric in-process stock), step layer by layer, and post [F55b R].
- **Feature Construction parameters** [F33 R]:
  - Tools: electric-arc wire, laser powder, laser wire.
  - Base: Plane, Cylinder, Surface of Revolution or Arbitrary freeform.
  - Bead and layers: bead width, offset from base, layer thickness.
  - Infill: stepover, allowance, one-way or two-way, start corner, angle, rotate or alternate
    per layer.
  - Perimeter: before or after infill, gap, pass count or distance, seam rotation within and
    between layers.
  - Tool axis: forward and sideways tilt, relative to the base normal or toward the previous
    layer.
  - Linking: full, minimum or shortest-path retraction, safe distance, lead radii, and the
    DED-specific transitions none, straight, shortest and smooth.
  - Tolerance: the toolpath plus triangulation sum again.
  - Not documented: laser power or wire-feed parameters.
- Also usable for FFF and concrete printing with post edits [F56 R].
- **Hybrid.** At preview launch, Autodesk said a deposition stock model can feed a later
  milling setup for finishing, and that only flat stock was supported. The current base types
  (cylinder, freeform) supersede that flat-stock limit [F57 S, F33 R].
- **Comparators.** NX 2606 added rotary non-cutting moves and finish-pass direction control in
  multi-axis additive "Rotary Buildup" [N1 R].

LATER: forgeCAD's exact kernel adds little to FFF slicing. DIFF (later still): hybrid DED plus finish milling where the as-deposited stock is a true solid.

---

## 5. Simulation and verification

### 5.1 Fusion stock simulation

- Simulation covers milling, drilling, turning, probing, cutting and DED [F36 R].
- **Animation** moves the tool and updates the stock. **Verification** runs background checks.
  By default, collision checking is skipped during animation for speed. A preference turns it
  back on [F36 R].
- **Stock display** [F58 R]:
  - Material appearance.
  - Colourisation, including **Comparison**: leftover-stock offset plus tolerance. Blue means
    under-cut, green means within tolerance, red means gouge. Red and blue can be swapped.
  - An **Accuracy slider** that trades a coarser stock for speed.
  - Transparency, Regenerate on Rewind, and Stop on Collisions (equivalent to setting Errors to
    "Always stop").
- Distance from stock to model can be measured, and leftover stock can be exported (STL,
  according to the forums) [F36 R, F59 S].
- The internal stock representation (voxel, dexel or mesh) is **not documented**. The
  Accuracy slider implies a discretised model.

### 5.2 Machine simulation

- **With a machine**: milling setups only. It uses kinematics from the machine definition and
  the post, and it is slower but more faithful. **Without**: milling, turning and cutting,
  showing tool, stock and collisions only [F36 R].
- **Issue types** [F36 R]:
  - collisions: machine vs stock, machine vs fixture, machine vs itself;
  - overtravel beyond axis limits;
  - "retract and reconfigure" auto-inserted to avoid overtravel;
  - rapid moves into stock.
- **Collision pairs** are chosen per kinematics item, with defaults. Excluding pairs speeds up
  verification [F60 R]. The pages I read did not document clearance or near-miss distances.

### 5.3 NC-code verification

- Fusion simulates toolpath data, not the posted NC. Autodesk's G-code-driven route is
  **CAMplete TruePath** (3-5-axis mills) and **TurnMill** (Nakamura-Tome). Autodesk says it is
  moving CAMplete's post and machine-simulation capabilities into Fusion and phasing CAMplete
  out [F48 S].
- **Vericut** interprets G-code and control logic against a twin of the machine, fixture and
  stock, including travel limits, multi-channel code, tool, holder, spindle, table and fixture
  collisions, and rapid-into-material. Its add-ons are AUTO-DIFF (stock vs design) and Force
  optimisation (the vendor claims up to 40% cycle time) [V1 R].
- **NCSIMUL** (Hexagon) is the main rival. Opinion on which is stronger is split [V2 S].
- ESPRIT EDGE markets a digital twin with analytics for collisions, axis limits, acceleration
  and channel conflicts [E1 S]. Mastercam 2026.R2 claims GPU verification up to 10 times faster
  [M1 R].

### 5.4 Gouge checking and the accuracy model

- **Generation tolerance.** Fusion's tolerance is the toolpath tolerance plus the
  triangulation tolerance, and filtering is additional. Splines and surfaces are linearised
  into segments [F40 R]. Smoothing (arc fitting within tolerance) is off by default and adds to
  the total [F61 R].
- **Model awareness.** 3D strategies are model-aware: they hold stock-to-leave against the
  setup model and use shaft and holder checks [F40 R]. Forum users say 2D toolpaths are not
  model-aware, so a green check only means the calculation finished, and simulation is the
  gouge check [F59 S].
- **Comparison** colouring is the only quantified gouge display [F58 R].
- **Implication for forgeCAD.** No incumbent I read promises that the *real* swept tool volume
  stays within ε of the exact surface. Fusion's bound is stated against a triangulation of
  unstated quality. A certified gouge and excess check (swept tool vs exact B-rep, with interval
  bounds) is a genuine differentiator, and it matches the project's proof culture.

MUST: stock simulation with comparison colouring and holder/fixture collisions.
DIFF: certified gouge-free and certified tolerance; exact stock booleans where affordable.
LATER: full machine simulation, then G-code-driven simulation (that is Vericut's turf, so integrate rather than compete).

---

## 6. Feature recognition and automation

### 6.1 Fusion Hole Recognition (MfgExt)

- Finds cylinders (including partial holes cut by edges) and groups them by diameter, then
  assigns an action: spot, drill, counterbore, bore, tap, ream or ignore. Thread type applies
  when tapping [F62 R, F63 S].
- **Modifiers:** explode groups, delete the top or bottom segment, split the signature (machine
  a through hole from both sides), and flip the side [F62 R].
- **Options:** Multi-Axis Machining finds holes in all planes, not just the active Z. Find by
  angle (min/max from the spindle) and by maximum diameter. Organize by fewest tool changes or
  by hole. Restrict to a template-library folder. Use the fewest spot drills [F62 R].
- **Tools:** must match the hole diameter or be a smaller substitute. Counterbores look for
  flat end mills. If no tool matches, the operation is still created but will not generate
  [F62 R].
- **Templates:** "Store as Hole Template" saves custom actions, which appear once a feature
  matches [F62 R].
- Multi-Axis Drilling exists as a strategy, but its help page returned 404 [F64 404].

### 6.2 Fusion AI and automation

| Item | Status, late 2026 | Source |
|---|---|---|
| Autodesk Assistant in CAM | Conversational. Builds a setup (name, stock, WCS, bodies), adds common milling ops, searches the tool library, explains choices. The user tunes the output | [F50 S] |
| "AI-generated CAM programming" | Announced as coming. It is meant to learn from user edits | [F51 S] |
| Instant Prismatic Machining | Exploratory. Toolpaths for standard features from pocket, hole and fillet recognition, kept as editable standard ops | [F51b S] |
| Project Bernini | **Not CAM.** An Autodesk Research generative 3D-shape model (text, image or point cloud to shape), May 2024, not commercial | [A1 S] |
| Machining Essentials add-in | Prismatic feature analysis (MfgExt) | [F51c S] |

### 6.3 Third-party and competitor AI and automation

| Product | What it does | Source |
|---|---|---|
| CloudNC CAM Assist | Runs inside Fusion, Mastercam, NX, GibbsCAM and SolidCAM (Creo coming). 3-axis and 3+2 milling only, turning in development. Inputs are the model, machine, tools, material, workholding and preferred directions, plus an optional 2D drawing for hole specs. Output is editable native ops with physics-based feeds and speeds. Claims about 80% of a program | [C1 R] |
| Toolpath.com | DFM, quoting and programming. Generates toolpaths inside Fusion. Also offers a free viewer and the open-source "Chatter" machine monitor | [C2 R] |
| NX AI Make Machining Suggestion | Introduced in 2512. Pick a face and get three suggestions (ops, tools, parameters) from history. 2606 adds a history-free generative third suggestion and auto-creates parametric tools | [N1 R, N5 S] |
| NX Feature-Based Machining and Machining Line Planner | FBM recognises features and applies rules. MLP plans across a line digital twin | [N5 S] |
| NX Copilot | A chat assistant for domain questions and docs | [N5 S] |
| NX Background Generate, Fixture Automation | Generation in a background process; automatic fixture alignment (2606) | [N1 R] |
| Mastercam Copilot | Beta in 2026. In 2026.R2 it edits feeds, speeds and machine parameters by voice or text, covers about 200 toolpath types and builds machine groups | [M1 R, M2 S] |
| ESPRIT ProPlanAI | Learns process choices from a shop's past ESPRIT files, so no rules need writing. 2.5-axis milling in 2025, then turning, mill-turn and Swiss in April 2026. Hexagon claims up to 75% less programming time | [E2 S] |

### 6.4 Fusion CAM API

- Lives in `adsk.cam`. Python and C++, with TypeScript in the samples. The CAM product exists
  only once CAM data exists [F65 R].
- **Create:** `setups.createInput(OperationTypes.MillingOperation)` then `setups.add`, with
  stock and WCS set through named parameters (`job_stockOffsetMode`, `wcs_origin_boxPoint`...).
  Operations come from `operations.createInput('<strategy>')` (`'face'`, `'adaptive'`,
  `'scallop'`...). Tools are assigned from library JSON. `compatibleStrategies` lists what the
  licence allows [F65, F66 R].
- **Templates:** `Setup.createFromCAMTemplate2(CreateFromCAMTemplateInput)`. The older
  `createFromCAMTemplate` was retired in July 2024. `CAMTemplate.createFromFile`,
  `createFromOperations` and `createFromXML` also exist [F67 S].
- **Generate:** `generateToolpath` or `generateAllToolpaths` return a
  `GenerateToolpathFuture`, which the script polls [F65, F66 R].
- **Post:** query the post library, import the post configuration, then
  `ncPrograms.createInput` and `postProcess(NCProgramPostProcessOptions)`. Post properties are
  set by name [F66 R].
- **Setup sheets:** `generateSetupSheet` (HTML; Excel is not supported on Mac) [F65 R].
- The machine-definition API exposes multi-axis retract and reconfigure settings (2025
  preview) [F47 S].

MUST: hole recognition with templates; a full scripting API (already natural for forgeCAD).
DIFF: recognition on exact topology with provable feature classification, plus an agent-driven op generator whose every op passes a certified gouge check.
LATER: learned process planning in the style of ProPlanAI.

---

## 7. Setup sheets, NC program management, DNC (brief)

- **NC Program:** post selection with an optional *cascading post* (a second post, for example
  for verification files), "Use machine configuration", setup and operation selection,
  multiple setups in one program with several fixtures and offsets, output folder, Post to
  Fusion Hub (cloud copy), and per-post properties covering home moves, tool-change positions,
  numbering, headers and limits [F68 R].
- **Setup sheet** (from the NC program): Summary (totals, times, date, design, tool count,
  travels), Tools (from the library plus per-op cycle time, cut distance and Z range), Setup
  (stock), and Operations. Configurable styles, printable, published to a hub folder [F69 R].
  Custom configurations were an Insider preview [F70 S].
- **DNC and monitoring:** Fusion Machine Connect sends NC to machines and collects data over
  MTConnect, OPC-UA (Siemens), FOCAS (Fanuc) and DCOM (Heidenhain), including Haas
  [F71 S]. A Mazak add-in pushes data to Mazak controls [F72 S].

MUST: NC program plus setup sheet. LATER: DNC and monitoring (buy or partner).

---

## 8. Priority table for a new entrant

| # | Item | Tag |
|---|---|---|
| 1 | 3+2 per-op frames, tilted-plane output | MUST |
| 1 | Automatic indexing (fewest orientations) | DIFF |
| 1 | 4-axis wrap plus indexed rotary | MUST |
| 1 | Simultaneous rotary rough and finish | LATER |
| 1 | Swarf, Multi-Axis Contour, tiltable 3D finish | MUST |
| 1 | Flank milling and barrel tangent-plane with proved deviation | DIFF |
| 1 | Geodesic, multi-axis roughing, deburr | LATER |
| 1 | Tool-axis modes, limits, smoothing | MUST |
| 1 | Tilt-until-clear collision avoidance plus trim | MUST |
| 1 | Certified holder and shank clearance; minimum stick-out | DIFF |
| 1 | Rewind / retract-and-reconfigure handling | MUST |
| 2 | Core turning (face, profile, groove, thread, part-off, ISO inserts) | MUST |
| 2 | Sub-spindle part handling, live tooling, Y-axis | LATER |
| 2 | Multi-channel synchronisation | LATER |
| 3 | Probe WCS cycles via macro calls | MUST |
| 3 | Inspection deviation against the exact B-rep | DIFF |
| 3 | Inspect Surface, part alignment, live connection, reports | LATER |
| 4 | Additive (FFF, MPBF, binder) | LATER |
| 4 | DED plus finish-milling hybrid with true-solid stock | LATER/DIFF |
| 5 | Stock simulation with comparison colouring and fixture collisions | MUST |
| 5 | Certified gouge and excess check against the exact surface | DIFF |
| 5 | Machine simulation with kinematics | LATER |
| 5 | G-code-driven verification | LATER (integrate Vericut or NCSIMUL) |
| 6 | Hole recognition with templates | MUST |
| 6 | Scripting API / agent ops | MUST (already the forgeCAD model) |
| 6 | Exact feature recognition plus verified auto-programming | DIFF |
| 6 | ML process planning | LATER |
| 7 | NC program plus setup sheet | MUST |
| 7 | DNC and monitoring | LATER |

---

## Sources (all accessed 2026-11-23)

Fusion help base URL: `https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/` (shown as `FC/`).
Fusion API base URL: `https://help.autodesk.com/cloudhelp/ENU/Fusion-360-API/files/` (shown as `FA/`).

| ID | Page | Tag |
|---|---|---|
| F1 | FC/MFG-REF-3D-STEEP-SHALLOW-COLLISION-AVOIDANCE.htm | R |
| F2 | FC/MFG-REF-3D-STEEP-SHALLOW-TOOL-AXIS-MODE.htm | R |
| F3 | FC/MFG-REF-3D-STEEP-SHALLOW-OVERRIDE-TILT-ANGLE.htm | R |
| F4 | FC/MFG-AVOID-COLLISIONS.htm; FC/MFG-REF-3D-STEEP-SHALLOW-5X.htm | R |
| F5 | FC/MFG-REF-3D-STEEP-SHALLOW-SMOOTHING-DISTANCE.htm | R |
| F6 | FC/MFG-APPLY-TOOL-AXIS-LIMITS.htm | R |
| F7 | FC/MFG-REF-SETUP-MILL.htm | R |
| F8 | autodesk.com/support/technical/article/Posted-g-code-refers-to-the-Setup-WCS-rather-than-tool-orientation-in-Fusion-360 (403 on fetch) | S |
| F9 | forums.autodesk.com "Tool orientation, does setting a new origin actually work?" | S |
| F10 | FC/GUID-7FF25192-AED7-4230-92DC-E22ADF15D498.htm (4th Axis Wrap reference) | R |
| F11 | FC/MFG-MULTI-AXIS-ROTARY.htm; FC/MFG-MULTI-AXIS-ROTARY-CONTOUR-READ.htm | S |
| F12 | FC/MFG-MULTI-AXIS-ROTARY-COLLISION-AVOIDANCE.htm | R |
| F13 | FC/MFG-MULTI-AXIS-ROTARY-POCKET-READ.htm | R |
| F14 | FC/MFG-MULTI-AXIS-SWARF.htm | R |
| F15 | help.autodesk.com/cloudhelp/2022/ENU/FCAM/files/GUID-36548514-... (Swarf Milling tab, up/down smoothing) | S |
| F16 | FC/MFG-MULTI-AXIS-ADV-SWARF.htm | R |
| F17 | FC/MFG-MULTI-AXIS-CONTOUR-READ.htm | R |
| F18 | FC/MFG-MULTI-AXIS-FINISHING.htm | R |
| F19 | FC/MFG-MULTI-AXIS-CLEARING-OVERVIEW.htm | R |
| F20 | FC/MFG-MULTI-AXIS-GEODESIC-READ.htm | S |
| F21 | autodesk.com Fusion blog, "Upgraded the multi-axis capabilities" and "5-axis Part 2: collision avoidance" (403 on fetch) | S |
| F22 | FC/3D-STEEP-AND-SHALLOW-READ.htm; FC/3D-STEEP-AND-SHALLOW-MACHINE-UNDERCUTS.htm | S |
| F23 | FC/MFG-MULTI-AXIS-DEBURR-READ.htm | R |
| F24 | autodesk.com Fusion blog, deburr posts (403 on fetch) | S |
| F25 | FC/MFG-PROBING-OVERVIEW.htm | R |
| F25t | FC/MFG-TURN-OPERATIONS-OVERVIEW.htm | R |
| F26 | FC/TURN-FACE-READ.htm; FC/TURN-CHAMFER-READ.htm; FC/TURN-PART-READ.htm | S |
| F27 | Fusion blog turning updates; Inventor HSM 2016 turning help (thread, groove) | S |
| F28 | FC/TURN-ADAPT-ROUGH-READ.htm | R |
| F29 | FC/TURN-COMMON-FEATURES.htm | R |
| F30 | FC/MFG-OVERVIEW-TURNING-WORKFLOW.htm | R |
| F31 | FC/MFG-PART-HANDLING-OVERVIEW.htm | R |
| F31b | FC/TURN-SECONDARY-SPINDLE-CHUCK-STEPS.htm; FC/TURN-BAR-PULL-STEPS.htm; FC/TURN-SECONDARY-SPINDLE-RETURN-READ.htm | S |
| F32 | FC/TURN-BAR-PULL.htm | R |
| F33 | FC/MFG-AM-REFERENCE-FEATURE-CONSTRUCTION.htm | R |
| F34 | FC/GUID-6C532684-30CC-4C00-B983-ACE28DBFBDDE.htm (turning insert/holder) | R |
| F35 | forums.autodesk.com turning-holder and lathe-tool threads (older) | S |
| F36 | FC/GUID76F8D8EF-2725-4203-944B-B9345936DDDB.htm (Simulation for manufacturing) | R |
| F36f | forums.autodesk.com "Live tooling lathe with Y axis", "mill-turn radial live tooling XZC" | S |
| F37 | FC/MFG-MACHINE-EDIT-KINEMATICS.htm | R |
| F38 | autodesk.com support "rewind of machine is required..." (via search) | S |
| F39 | FC/MFG-REF-UNSUIT-MACHINE-CONFIG.htm | R |
| F40 | FC/GUID09E44604-DAD8-47D6-ADC6-C100869DE724.htm (3D Adaptive reference: tolerance, stock to leave, rest sources, holder modes) | R |
| F41 | FC/GUID-61123A95-8828-4214-A8FE-5A4DD1C2EE4F.htm (Probe WCS reference) | R |
| F42 | help.autodesk.com/cloudhelp/2020/ENU/INVCAM/files/GUID-8FB4E732-... (Probing cycle reference) | R |
| F43 | forums.autodesk.com "Probing and updating WCS for angle" | S |
| F44 | FC/MFG-PROBE-GEOMETRY-OVERVIEW.htm | R |
| F45 | FC/MODIFY-LEADS-AND-LINKS.htm | R |
| F46 | Autodesk Learn multi-axis cert prep unit; FC/GUID2E5ABA80-... (Stay-Down) | S |
| F47 | FA/MultiAxisRetractAndReconfigureSettings.htm (404 on fetch; snippet only) | S |
| F48 | autodesk.com/ca-en/products/camplete/overview and localized pages (403 on fetch) | S |
| F49 | FC/MFG-INSPECT-SURFACE-OVERVIEW.htm | R |
| F49b | FC/MFG-REF-INSPECTION-RESULTS-WINDOW.htm; FC/MFG-HOW-TO-PART-ALIGNMENT-INSPECT.htm | S |
| F50 | Fusion blog "Simplify CNC Milling Programming in Fusion with Autodesk Assistant"; "8 practical ways" | S |
| F50m | FC/MFG-MANUAL-INSPECTION-OVERVIEW.htm; FC/MFG-PMI-FOR-INSPECTION-TOLERANCES-MANUAL-INSPECTION.htm | S |
| F51 | Fusion blog "AI-powered workflows coming to Autodesk Fusion" (Sep 2026) | S |
| F51b | Fusion blog "What's New and Next for Fusion Manufacturing: IMTS and AU" | S |
| F51c | Autodesk Marketplace "Machining Essentials" | S |
| F51r | FC/MFG-INSPECTION-REPORT-OVERVIEW.htm | R |
| F52 | FC/MFG-LIVE-CONNECTION-INSPECTION-OVERVIEW.htm | R |
| F52b | forums.autodesk.com "Fanuc Inspection Connector" (2024) | S |
| F53 | forums.autodesk.com "Fanuc inspection post: configure for Renishaw macros" | S |
| F54 | cam.autodesk.com/posts/reference/classPostProcessor.html (first 100k chars) | R |
| F55a | FC/MFG-AM-WORKFLOW-PREPARE-PARTS-FOR-ADDITIVE-MANUFACTURE.htm | R |
| F55b | FC/MFG-AM-WORKFLOW-PREPARE-FOR-ADDITIVE-MULTI-AXIS.htm | R |
| F56 | FC/MFG-AM-FEATURE-CONSTRUCTION.htm | R |
| F57 | Fusion blog "access multi-axis deposition DED toolpaths" (403 on fetch) | S |
| F58 | FC/GUIDB27B568A-3BBF-48A6-B430-BBA619B906D0.htm (Display stock during simulation) | R |
| F59 | forums.autodesk.com "Model gouge warning", "Simulation and rendering accuracy" | S |
| F60 | FC/MFG-MACHINE-MANAGE-COLLISION-CHECKS.htm | R |
| F61 | FC/GUID5A951723-4797-4F5F-A7A3-45DCBCA2D26F.htm (Smoothing reference) | R |
| F62 | FC/GUID-A4AAF46E-F757-4797-A678-261C201D85F7.htm (Hole Recognition reference) | R |
| F63 | FC/LP-READ-IRONHOLERECOGNITION.htm; Fusion blog hole recognition | S |
| F64 | FC/MFG-MULTI-AXIS-DRILLING-OVERVIEW.htm | 404 |
| F65 | FA/CAMIntroduction_UM.htm | R |
| F66 | FA/BasicMillingWorkflowSample_Sample.htm | R |
| F67 | FA/Setup_createFromCAMTemplate2.htm, FA/cam_CAMTemplate.htm, FA/New_Operation_Sample_Sample.htm | S |
| F68 | FC/MFG-REF-NC-PROGRAM.htm | R |
| F69 | FC/MFG-REF-NC-PROGRAM-SETUP-SHEET.htm | R |
| F70 | autodesk.com support "How to customize CAM Setup Sheet Configurations" | S |
| F71 | autodesk.com/uk/products/fusion-machine-connect/overview | S |
| F72 | Fusion blog "Mazak add-in for Fusion 360" | S |
| A1 | research.autodesk.com Project Bernini (May 2024) | S |
| P1 | help.autodesk.com/cloudhelp/2022/ENU/PWRM-ReferenceHelp/files/GUID-35CFC2A8-553F-43CB-9909-67C08927B3DA.htm (PowerMill collision avoidance) | R |
| P2 | help.autodesk.com/cloudhelp/2022/ENU/PWRM-ReferenceHelp/files/GUID-89C9450F-3E72-4036-B94F-68E9404F2FFE.htm (PowerMill tool axis) | R |
| H1 | openmind-tech.com/en/cam/5-axis-milling/ and /tilt-strategies | R |
| H2 | openmind-tech.com/en/cam/5-axis-milling/collision-avoidance | R |
| H3 | newequipment.com "hyperMILL 2025 ... 5-axis toolpath efficiency" | S |
| H4 | openmind-tech.com/en/cam/hypermill-maxx-machining/finishing/conical-barrel-cutter; mmsonline barrel articles | S |
| N1 | blogs.sw.siemens.com/nx-manufacturing/whats-new-in-nx-for-manufacturing-2606-june-2026/ (19 Jun 2026) | R |
| N2 | blog.janus-engineering.com "Synchronization of multi-channel machines in NX CAM" (28 Mar 2023) | R |
| N3 | docs.plm.automation.siemens.com NXOpen.CAM.ToolAxisVariableTypes / ToolAxisVariable | S |
| N4 | support.sw.siemens.com PL8003454 (channels in Sync Manager) | S |
| N5 | blogs.sw.siemens.com NX 2512 and "AI has arrived for CAM programming" | S |
| M1 | mastercam.com/solutions/products/mastercam-2026-r2/ | R |
| M2 | mastercam.com Mastercam 2026 press release; DE247 | S |
| E1 | hexagon.com/products/esprit-edge-swiss-machining (403 on fetch); dmgmori.com ESPRIT EDGE pages | S |
| E2 | hexagon.com press releases 2025 (ProPlanAI) and 2026 (ESPRIT EDGE capabilities) | S |
| C1 | cloudnc.com/cam-assist | R |
| C2 | toolpath.com | R |
| V1 | vericut.com/en-gb/guides/g-code-simulator | R |
| V2 | PTC community thread and comparison listicles on Vericut vs NCSIMUL | S |
| R1 | renishaw.com/en/inspection-software-for-machining-centres--6093 | R |
| R2 | Renishaw Inspection Plus programming manual (Haas-hosted PDF) via search | S |

### Pages I could not reach

- **All `autodesk.com/products/fusion-360/blog/*` posts**, including the wp-json endpoints:
  HTTP 403. That covers the 4-axis, 5-axis, deburr, turning-updates, DED-launch, AI and
  Assistant posts, so those claims rest on snippets only.
- **`autodesk.com/solutions/mill-turn-software`, `autodesk.com/products/fusion-360/ai-automation`,
  `autodesk.com/ca-en/products/camplete/overview`, and the Autodesk support article on the
  setup WCS vs tool orientation:** 403.
- **`hexagon.com/products/esprit-edge-swiss-machining`:** 403.
- **`openmind-tech.com/en/cam/maxx-machining/`:** 404. I used the conical-barrel page snippet
  instead.
- **`FC/MFG-MULTI-AXIS-DRILLING-OVERVIEW.htm` and `FA/MultiAxisRetractAndReconfigureSettings.htm`:**
  404.
- **Siemens NX mill-turn product page** (after its redirect): reachable, but it had none of the
  claimed details.
- **Not read at all:** NCSIMUL's own pages, the full Renishaw Inspection Plus manual, Fusion's
  Linking-tab reference (exact Retraction Policy list), and the Geodesic and Rotary Parallel
  pages beyond their snippets.

### Open questions worth a follow-up

- What stock representation does Fusion use for simulation (voxel, dexel or mesh), and what
  does its Accuracy slider map to? Not documented in anything I read.
- Does Fusion have automatic 3+2 indexing? I found none in the help.
- What is the current list of controls with a Machine Connector for live inspection?
