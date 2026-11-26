# Native schematics and symbols

A schematic compiles to sheet occurrences, electrical nets, drawable geometry and functional relationships. Pin requirements and offerings carry nominal role IDs and a checked role kind. The same symbol can want and give several roles. The [controller](../examples/electronics/controller.fg) uses an [example library of power-circuit components](../examples/electronics/power-symbols.fg).

```sh
forgecad build examples/electronics/controller.fg --json
forgecad schematic examples/electronics/controller.fg pin main/reg.out --json
forgecad schematic examples/electronics/controller.fg tree main/reg.out --json
forgecad schematic examples/electronics/controller.fg sheet main --json
forgecad check examples/electronics/controller.fg --json
```

## Roles and symbols

```text
symbol_library
bypass = role(kind: "bypass", key: "example.bypass100n", capacitance_nf: 100)
rail = role(kind: "power", key: "example.3v3", voltage_mv: 3300)
Reg = symbol(pins: {
  out: pin(number: "3", kind: "power_out", at: (2, 0), gives: [rail], wants: [bypass]),
  gnd: pin(number: "1", kind: "passive", at: (0, -2))
}, graphics: [rect(from: (-2, -2), to: (2, 2))])
```

A role declaration binds an identifier. Pin lists and `fulfill(role: ...)` take references to these declarations. The schema encodes a role as an object, for example `{"id":"@example.3v3","kind":"power","rating":{"kind":"voltage","unit":"mV","value":3300},"prefer_mm":null}`. A quoted role name in a pin list, an unknown role, a mismatch between a want and a give, and incompatible definitions of a shared key produce located compile errors. Distinct libraries can intentionally share a type by using the same explicit key and definition. Without a key the import namespace qualifies the role. The declared kind is one of `power`, `return`, `bypass`, `signal`, `clock`, `bias`, `termination`, `feedback`, `protection`, `reference`, or `interface`. Optional capacitance in nF applies to bypass roles, voltage in mV to power and reference roles. An optional positive length `prefer` records a project placement preference. The PCB consumer owns its eventual measurement.

### Pin-to-pin connection wants

A target pin can be a sibling such as `OUT` on the requesting component, or a pin such as `J1.P1` on the same sheet occurrence. A direct want checks net equality, including connections made through junctions, labels and sheet ports. A through-part want checks for one fitted component bridging two distinct nets through a permitted pair of terminals. A wire shorting those nets fails the check. The target pin can be passive and needs no role declaration.

```fg
feedback = role(kind: "feedback", connect_to: OUT)
input_filter = role(kind: "signal", connect_to: J1.P1, through: "resistor", value_range: ["2.2 kΩ", "4.7 kΩ"])
```

A quoted type accepts any symbol declaring that `part_kind`. Part types are open names such as `"capacitor"`, `"inductor"`, `"mosfet"`, and `"relay"`. A symbol reference such as `through: analog.Resistor` selects one library symbol. Set **Part type** and **Terminal paths** in symbol properties.

Two-pin symbols with no named paths can be traversed in either order. Other symbols declare ordered pin pairs. For example, a MOSFET can declare `paths: {channel: [D, S]}`. A want such as `role(kind: "return", connect_to: P2, through: "mosfet", part_path: "channel")` checks that the requester reaches the drain and the target reaches the source on the same fitted MOSFET. A second MOSFET symbol can map the same path to differently named pins. With one declared path, the want may omit its name. With several paths, select one explicitly. These rules check wiring topology. Gate voltage and transistor operating state are separate electrical questions. [MOSFET, capacitor and inductor example](../examples/electronics/analog/switched-load.fg)

Put the roles in the pin's Wants list or on a placement, for example `wants: {INP: [input_filter], INM: [feedback]}`. Omit the value constraint to accept any nominal value, or use `value: "4.7 kΩ"` for an exact match. Values use Ω, ohm, F or H with SI prefixes. Resistor markings such as `4k7` are accepted. An unknown or dimensionally incompatible placed value fails a value-constrained check. These checks concern nominal values and the two terminals of a single component. Ordered paths can specify polarity. Multi-part chains and manufacturing tolerances remain outside this check.

In **Pin Role**, enter the target pin and choose **Direct connection**, a declared part type, or a specific symbol. For a symbol, enter one value or both inclusive limits. The pin checklist and `forgecad schematic FILE pin main/U1.INP --json` report the result and the bridging pins. The pin query returns value bounds as decimal strings with a common base unit to preserve small electrical values. [Amplifier input and feedback example](../examples/electronics/analog/pin-connections.fg)

### Connector pin roles

A generic connector starts with numbered passive contacts and empty wants and provides. Put the external circuit's signal or supply on the placed connector. In the schematic, select the contact, expand **Placement roles**, and check **Wants** or **Provides** for that pin. **New role…** declares a typed role in the schematic.

```fg
incoming = role(kind: "power", voltage_mv: 3300)
J1 = place(sheet: main, symbol: analog.Connector2, at: (0,0), gives: {P1: [incoming]})
J2 = place(sheet: main, symbol: analog.Connector2, at: (20,0))
```

Here J1 pin 1 declares that the attached equipment supplies 3.3 V. J2 inherits the connector's passive contacts. For a dedicated connector with a fixed interface, the library author can specify pin roles once. Each placement inherits those roles and can add its own. A contact's hardware voltage rating is a limit on the connector, independent of the voltage supplied by the external circuit. Like the wiring, a role on a placement in a repeated sheet definition applies in each instance of that sheet.

### Voltage wants and provides

Attach a reusable operating range to a normal pin through its Wants list. A power source can declare a single voltage or a guaranteed range. All endpoints are inclusive.

```fg
operating_supply = role(kind: "power", voltage_range_v: [1.8, 5.5])
regulated_output = role(kind: "power", voltage_mv: 3300)
IC = symbol(pins: {VDD: pin(number: "5", kind: "power_in", at: (0,8), wants: [operating_supply])})
Reg = symbol(pins: {OUT: pin(number: "1", kind: "power_out", at: (4,0), gives: [regulated_output])})
```

In the symbol editor, create a power role, enter **Minimum (V)** and **Maximum (V)**, then check **Wants** on the supply pin. The schematic checks the connected net automatically. The checklist shows the required interval and the pin and voltage of each fitted provider. A 3.3 V source satisfies this want. A 12 V source produces `VoltageOutOfRange`. A source range must fit entirely inside the wanted range. Without a declared source, ERC reports `UnknownSupplyVoltage`. Rail labels alone establish connectivity. This check assumes a shared voltage reference. The pin query exposes `voltage_checks` for CLI and MCP. [Powered TLV9001 example](../examples/electronics/analog/powered-amplifier.fg)

A symbol can also declare `peripherals: {I2C1: {SDA: [PA0], SCL: [PA1]}, UART2: {TX: [PA0]}}`. Each signal lists the physical pins that can carry it. The reader resolves the names against this symbol and exposes the groups in the symbol query.

A symbol's map keys are logical pin names. Each pin has a unique physical `number` string, sheet-local connection point, and electrical `kind` (`passive`, `input`, `output`, `bidirectional`, `power_in`, `power_out`, `open_drain`, `no_connect`). Optional `angle` gives the pin direction in degrees, counterclockwise from the positive X axis. Fractional and negative angles are accepted. A `unit` string groups pins into sub parts; the default is A. Graphics accept `line(from:, to:)`, `rect(from:, to:)`, `circle(at:, radius:)`, `arc(from:, mid:, to:)`, `polyline(points:)`, and `text(at:, value:)`, each optionally limited to one sub part. A library holds roles, shared numeric expressions and symbols. Import it into a schematic with `parts = library(file: "power-symbols.fg")`. Imports use the session's file fence.

## Sheets, connections and geometry

- Define a sheet with `main = sheet(title: "Power")`. Use `u = place(sheet: main, symbol: parts.Reg, at: (10mm, 5mm))`. Optional fields are `angle` (quarter turns), `mirror`, `value`, `footprint`, and `unit`. Use **Place Sub Part** to draw another sub part on its sheet with `second = gate(sheet: main, of: u, unit: "B", at: (20, 5))`. Both sub parts use the same component and pin identity. Unplaced sub parts appear in ERC.
- Connect `wire(sheet:, from:, to:, via: [...])` between explicitly named terminals, junctions and ports. Coordinates give the presentation. Endpoint attachment gives conductivity. Crossing two wires leaves their nets separate. A `junction(sheet:, at:)` shared by the wires joins them. `no_connect(sheet:, on:)` marks an intentionally unused pin.
- `label(sheet:, on:, value:, at:, global: true)` connects like names throughout the project. The default scope is one sheet occurrence. `note(sheet:, value:, at:)` adds text. `bus(sheet:, members: {scl: u.scl, sda: u.sda})` groups named endpoints while retaining their separate nets.
- A definition with `out = port(sheet: cell, at: (0, 0))` exposes that port at each `channel = subsheet(sheet: main, of: cell, at: (10, 10), ports: {out: (2, 0)})`. A wire on main refers to `channel.out`. The `ports` map is optional and supplies parent-side offsets. Sheet definitions can repeat, and cyclic instantiation is refused. Queries use qualified paths such as `main/channel/u.out`.
- `flow(sheet:, from:, to:)` records an ordered functional stage. `fulfill(sheet:, want:, give:, role:, return_from:, return_to:, prefer:)` binds a provider to a consumer for a declared role. The return pair and preference are optional. Wiring still comes from explicit electrical attachments. ERC checks supply/signal connectivity, the declared return paths, conflicting labels/drivers, connected no-connects, unconnected pins and unmet wants. Readability checks report overlapping printed fields, pin names, labels and notes (`TextBoundsOverlap`), and text crossing wires or symbol graphics or obscuring a symbol body (`TextGeometryOverlap`). Each warning identifies both objects. Pin inspection ranks a direct wire first, then a drawn path through explicit junctions by segment count, then a shared net. Confirmation records the designer’s choice.
- Coordinates support mm, inches and shared dimensional expressions using the existing .fg dimensional arithmetic. Sheet presentation, logical nets and functional links can be queried separately.

## Agent and CLI tools

MCP `new_schematic(file, kind)` creates an empty `schematic` or `symbol_library` in a writable session and refuses to overwrite an existing file. Use `edit` to add definitions, placements and wires. The CLI exports a drawing with `forgecad schematic examples/electronics/controller.fg svg main -o controller.svg`.

MCP `schematic` accepts `file` and a `sel` string. The queries are `summary`, `symbols`, `symbol ID`, `sheet PATH`, `components`, `nets`, `netlist`, `bus PATH`, `erc`, `links`, `pin PATH`, `tree PATH`, `svg SHEET_PATH`, and `symbol-svg ID [UNIT]`. SVG queries return SVG text in the JSON result. The reply includes the source version, and `erc_ok` distinguishes electrical errors from warnings. `check` returns failure for electrical errors. A successful `build` can describe a partly wired drawing.

MCP `plan_fulfillment` confirms a candidate from an existing drawing. Supply the qualified want and give pins, the role ID, a new relationship ID and an optional pair of return pins. It proposes one checked source edit. The electrical wiring stays in place.

MCP `plan_bypass` takes the file, qualified consumer and return pin, symbol and its two pin keys, nominal role, new occurrence ID and sheet coordinates. It returns four validated `AddFeature` operations and the base version. Send that batch to the normal writable `edit` tool. The symbol, supply wire, return wire and typed relationship commit together and undo together. Planning is read-only. A plan targeting a repeated sheet definition requests an explicit definition edit because every instance would change. On the CLI, the same operation is available as `schematic FILE plan-bypass WANT RETURN SYMBOL GIVE_KEY RETURN_KEY ROLE NEW_ID X_MM Y_MM`. Ordinary source edits and the existing version, undo and redo tools apply to both document kinds.

## Verification

The [Lean model](../research/forge-lang/lean/ForgeLang/Schematic.lean) proves nominal role admissibility, preservation of wiring by role confirmation, and invariant reachability when a wire is reversed. Grammar witnesses use the shared .fg parser model. The Rust tests compare net partitions against an independent transitive-closure oracle and exercise hierarchy, gate placement and role refusals. The MCP integration test plans, commits, rejects a bad edit, undoes, and invalidates a changed library. The [coverage ledger](../research/forge-lang/lean/COVERAGE.md) identifies the remaining Rust-to-model proof gaps.

The session cache recognizes a change confined to the literal coordinates of a placement. It updates pin positions and print-bounds findings on each affected sheet occurrence, preserves confirmed links and electrical partitions, and adjusts source locations. Changes to wiring, parameters or imported symbols use a fresh compilation. The regression suite compares cached results, source locations and SVG against fresh compilation across repeated sheets and invalid edits. The [latency probe](../research/electronics/probes/README.md#native-schematic-response-time) exercises this path through MCP.

## Editing in ForgeCAD

Choose **File → New Schematic** or **File → New Symbol Library**. Each opens a document tab. Save both files in the project before linking the library. In the schematic, **Link Library** takes its relative filename and a local name. **Place Symbol** shows the definitions in the linked library. The left tree navigates sheet occurrences. **Parts** in the right dock searches document and linked-library symbols by name, pin description, and custom fields such as MPN. Select a result to preview and place it, or open the library from the same dock. The [sensor interface](../examples/electronics/analog/sensor-interface.fg) and its [library](../examples/electronics/analog/analog-symbols.fg) are a working pair.

- Use **Wire** to pick the first pin, any bends, and the destination. Finish on an existing wire to branch it. **Junction** splits an existing segment. Moving a connected component keeps its wires attached. A plain crossing leaves the nets independent.
- **Assign Role** records a typed provider and consumer, with optional return pins and a preferred separation. Select a pin to inspect its declared roles and candidate providers. **Inspect → Electrical Checks** shows electrical and readability findings.
- In a library, select a symbol, place its pins and draw its outline. Select a pin to set its number, sub part, orientation, electrical kind, wants and gives. Select a graphic to edit its coordinates. The library and all its instances share the same geometry.
- When a whole sheet is fitted, fine pin labels appear as you zoom in. Hover over an endpoint to identify it at any scale. The scroll wheel zooms around the pointer, middle drag pans, and **View → Fit All** frames the drawing. Hover over a ribbon tool to see its shortcut. Escape cancels the current tool. Source edits use the document's undo and redo history.

### Pin inspection and signal directions

Select or right-click a pin to open **Pin checklist** at the top of **Properties**. It shows the short library description and the verified count for **Wants** and **Provides**. Check an open role to select a compatible pin on the same sheet and net. The assignment form accepts optional return pins and preferred separation. Once verified, the row is checked. An invalid assignment shows a mixed checkbox and **Check wiring**. Activate a checked or mixed row to remove an individual assignment. These edits support Undo. In the symbol editor, the **Pin roles** table sets Wants and Provides independently for each declared role. A verified relationship names the other occurrence and pin. An electrically connected match without a confirmed assignment is a candidate. An assignment with a disconnected supply or invalid return is marked **Check wiring**. Selecting a reference opens that pin, including on another sheet. The canvas highlights the selected pin, its peers and their drawn paths, including the connected power symbol. Other pins on the same component and the bypass return stay dim. The return still participates in electrical verification. A connected power symbol is listed under a power need even if a source has yet to be assigned. The checklist stays in the right dock. Escape closes inspection.

In the symbol editor, edit **Pin name** and **Direction (°)** and select **Apply pin changes**. Expand **Sub parts** to change which sub part contains the pin. For example, the two amplifiers in one dual op-amp package can be drawn as U1A and U1B. The selected pin shows an orientation arrow. Renaming keeps the pin number and records the previous name in `aliases` so existing schematic references still connect to the same terminal.

Select the symbol itself to edit **Reference prefix**. For example, `Resistor = symbol(ref_prefix: "R", pins: {...})` suggests R1, R2 and so on when placed. Prefixes can contain multiple letters such as TP. Existing placements keep their references.

The symbol editor saves the description with the pin, for example `pin(number: "3", kind: "input", at: (-7, 3), description: "Noninverting input. Connect the filtered sensor signal here.")`. The CLI and MCP pin and symbol queries include this field.

**View → Direction Arrows** (F6) and **Moving Dots** (F7) toggle independently. Green follows a known driver or an explicit `flow` along the connected net. Red markers travel both ways on a multiple-driver net or a wire with conflicting direction evidence. An undetermined wire stays unmarked. The markers move at a fixed screen speed to show logical direction.

### Drawing and grid

**Draw** provides line, rectangle, circle, three-point arc and text tools. Pick two corners for a box, a center and circumference point for a circle, or start, middle and end for an arc. Set stroke width or text height and bold in Properties before placing. Select the graphic to edit its points, radius or appearance, move it, or delete it. The same geometry and text print to SVG. Graphic strokes can cross pins and wires without joining nets. A graphical frame can enclose annotations, and a stroke crossing text produces a readability warning.

The grid control below the canvas accepts metric presets, 10/25/50/100 mil presets, and a custom pitch in millimetres. **Snap** and grid visibility are independent. The grid and snap share the sheet origin and pitch. Zooming out thins the displayed dots by powers of two while retaining the selected snap pitch. The placement cursor shows the point a click will use. Drawing also snaps to nearby pin anchors, shape endpoints, box corners and circle quadrants. Wires attach exactly to the chosen terminal even for an off-grid pin.

```text
frame = rect(sheet: main, from: (0, 0), to: (40, 20), width: 0.3mm)
title = note(sheet: main, at: (2, 15), value: "Analog input", size: 2.8mm, bold: true)
```

The [two-channel filter example](../examples/electronics/analog/filter-sheets.fg) repeats one annotated filter sheet twice and defines the signal flow through its input and output wires.

### Reading and placing fields and net names

On a physical symbol the larger field is its reference designator, and the next field is its value. Select the component and use **Field positions** in Properties to move either field independently. The source arguments `ref_offset` and `value_offset` measure from the component origin in sheet axes. Set either to `auto` to restore its default position. A hollow circle in the schematic marks a pin with no drawn attachment. Wiring it, attaching a label or marking it no-connect clears that circle. The symbol editor shows all pin anchors.

| Tool | Appearance | Electrical scope |
| --- | --- | --- |
| Label → Local label | Plain text above its anchor | Equal names join within one sheet occurrence. |
| Label → Global label | Outlined tag with a point at the attachment | Equal names join across the design. |
| Port | Outlined tag with a flat attachment edge | Exposes a terminal on a hierarchical sheet. A parent wire attaches to the instance's port, such as `channel.out`. |
| Power | Rail arrow or ground symbol with its net name | Joins the same global net as a global label of that name. |

**Power** includes +3V3, +5V, +12V, VCC, VDD, GND, AGND and DGND. The Power tab also searches custom power symbols declared by the document or its linked libraries. Click an existing terminal to attach the symbol in one edit, or place it on the grid and wire it later. Power symbols have one passive pin and are excluded from the BOM. Power sources such as regulators declare their own driver pins. The command inserts the bundled library into the document on first use. The same design can be written directly in the language.

```text
power = library(file: "forge:power")
vcc = place(sheet: main, symbol: power.V3V3, at: (10, 12))
feed = wire(sheet: main, from: vcc.net, to: U1.VDD)
```

### Purchasing data and datasheets

Symbols accept a map of custom string fields and a datasheet reference. Placements can override individual fields and the datasheet. Field names are Forge identifiers. For example, a library can define `fields: {MPN: "TLV9001IDBVR", Manufacturer: "Texas Instruments"}`. A placement can add `fields: {Supplier: "Stockroom"}` while retaining both library fields. Value and footprint are placement properties.

**Attach PDF** on a saved symbol or placement stores the PDF bytes in the document’s `datasheets/` folder under a SHA-256 filename and records the relative path. Importing identical bytes reuses the file. A datasheet URL is stored as text. Share the project directory, including its libraries and attachments.

**Inspect → Bill of Materials** groups fitted physical occurrences by effective specification and copies CSV with the custom fields as columns. Multiple sub parts count once per component, and repeated sheets contribute separate occurrence paths. `populated: false` and `in_bom: false` independently exclude placements. The same data is available through the MCP schematic queries `bom` and `bom-csv`, or from the CLI.

```sh
forgecad schematic examples/electronics/analog/sensor-interface.fg bom-csv -o sensor-bom.csv
```
