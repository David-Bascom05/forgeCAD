# Native schematics and symbols

A schematic compiles to sheet occurrences, electrical nets, drawable geometry and functional relationships. Pin requirements and offerings carry nominal role IDs and a checked role kind. The same symbol can want and give several roles. The [controller](../examples/electronics/controller.fg) uses the [power symbol library](../examples/electronics/power-symbols.fg).

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

A symbol's map keys are logical pin names. Each pin has a unique physical `number` string, sheet-local connection point, and electrical `kind` (`passive`, `input`, `output`, `bidirectional`, `power_in`, `power_out`, `open_drain`, `no_connect`). Optional `angle` gives the pin direction. A `unit` string groups pins into gates; the default is A. Graphics accept `line(from:, to:)`, `rect(from:, to:)`, `circle(at:, radius:)`, `arc(from:, mid:, to:)`, `polyline(points:)`, and `text(at:, value:)`, each optionally limited to a unit. A library holds roles, shared numeric expressions and symbols. Import it into a schematic with `parts = library(file: "power-symbols.fg")`. Imports use the session's file fence.

## Sheets, connections and geometry

- Define a sheet with `main = sheet(title: "Power")`. Use `u = place(sheet: main, symbol: parts.Reg, at: (10mm, 5mm))`. Optional fields are `angle` (quarter turns), `mirror`, `value`, `footprint`, and `unit`. Place an additional gate on its sheet with `second = gate(sheet: main, of: u, unit: "B", at: (20, 5))`. Both gates use the same component and pin identity. Unplaced units appear in ERC.
- Connect `wire(sheet:, from:, to:, via: [...])` between explicitly named terminals, junctions and ports. Coordinates give the presentation. Endpoint attachment gives conductivity. Crossing two wires leaves their nets separate. A `junction(sheet:, at:)` shared by the wires joins them. `no_connect(sheet:, on:)` marks an intentionally unused pin.
- `label(sheet:, on:, value:, at:, global: true)` connects like names throughout the project. The default scope is one sheet occurrence. `note(sheet:, value:, at:)` adds text. `bus(sheet:, members: {scl: u.scl, sda: u.sda})` groups named endpoints while retaining their separate nets.
- A definition with `out = port(sheet: cell, at: (0, 0))` exposes that port at each `channel = subsheet(sheet: main, of: cell, at: (10, 10), ports: {out: (2, 0)})`. A wire on main refers to `channel.out`. The `ports` map is optional and supplies parent-side offsets. Sheet definitions can repeat, and cyclic instantiation is refused. Queries use qualified paths such as `main/channel/u.out`.
- `flow(sheet:, from:, to:)` records an ordered functional stage. `fulfill(sheet:, want:, give:, role:, return_from:, return_to:, prefer:)` binds a provider to a consumer for a declared role. The return pair and preference are optional. Wiring still comes from explicit electrical attachments. ERC checks supply/signal connectivity, the declared return paths, conflicting labels/drivers, connected no-connects, unconnected pins and unmet wants. Overlapping pin name, label and note layout boxes have their own readability finding. Pin inspection ranks a direct wire first, then a drawn path through explicit junctions by segment count, then a shared net. Confirmation records the designer’s choice.
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

Choose **File → New Schematic** or **File → New Symbol Library**. Each opens a document tab. Save both files in the project before linking the library. In the schematic, **Link Library** takes its relative filename and a local name. **Place Symbol** shows the definitions in the linked library. Clicking a library in the document browser opens its symbol editor. The [sensor interface](../examples/electronics/analog/sensor-interface.fg) and its [library](../examples/electronics/analog/analog-symbols.fg) are a working pair.

- Use **Wire** to pick the first pin, any bends, and the destination. Finish on an existing wire to branch it. **Junction** splits an existing segment. Moving a connected component keeps its wires attached. A plain crossing leaves the nets independent.
- **Assign Role** records a typed provider and consumer, with optional return pins and a preferred separation. Select a pin to inspect its declared roles and candidate providers. **Inspect → Electrical Checks** shows electrical and readability findings.
- In a library, select a symbol, place its pins and draw its outline. Select a pin to set its number, unit, orientation, electrical kind, wants and gives. Select a graphic to edit its coordinates. The library and all its instances share the same geometry.
- When a whole sheet is fitted, fine pin labels appear as you zoom in. Hover over an endpoint to identify it at any scale. The scroll wheel zooms around the pointer, middle drag pans, and **View → Fit All** frames the drawing. Hover over a ribbon tool to see its shortcut. Escape cancels the current tool. Source edits use the document's undo and redo history.

### Purchasing data and datasheets

Symbols accept a map of custom string fields and a datasheet reference. Placements can override individual fields and the datasheet. Field names are Forge identifiers. For example, a library can define `fields: {MPN: "TLV9001IDBVR", Manufacturer: "Texas Instruments"}`. A placement can add `fields: {Supplier: "Stockroom"}` while retaining both library fields. Value and footprint are placement properties.

**Attach PDF** on a saved symbol or placement stores the PDF bytes in the document’s `datasheets/` folder under a SHA-256 filename and records the relative path. Importing identical bytes reuses the file. A datasheet URL is stored as text. Share the project directory, including its libraries and attachments.

**Inspect → Bill of Materials** groups fitted physical occurrences by effective specification and copies CSV with the custom fields as columns. Multiple gates count once per component, and repeated sheets contribute separate occurrence paths. `populated: false` and `in_bom: false` independently exclude placements. The same data is available through the MCP schematic queries `bom` and `bom-csv`, or from the CLI.

```sh
forgecad schematic examples/electronics/analog/sensor-interface.fg bom-csv -o sensor-bom.csv
```
