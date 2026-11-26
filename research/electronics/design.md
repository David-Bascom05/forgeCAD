# Native electronics in ForgeCAD

Design proposal · 25 November 2026

A designer draws a circuit, places it on a board, installs the board in an assembly and connects a harness. Throughout that work ForgeCAD can answer three questions about a selected object: what it is connected to, why it is placed there, and what the last edit changed. The native document is `.fg`. The graphical editor, text editor, CLI and MCP share the same typed model and edit operations. This record describes the proposed electronics tools and the evidence needed to ship them.

## First milestone

Determine how ForgeCAD can infer **which components serve which pins or functional groups**, and what that implies for placement, from an ordinary hand-drawn schematic. Start with bypass capacitors. Evaluate the inferred assignments against documented reference designs before looking at their layouts. The cross-view contract supports carrying those findings into the PCB. [Worked RP2040 example, inference design and benchmark](design-intent.md#inferring-which-pins-a-capacitor-serves)

## Design decisions

- **Drawing is sufficient.** Placing a symbol records its library identity, occurrence, pin roles and sheet geometry. Drawing a wire records its electrical attachments and visible path. A designer who never opens the text editor gets the same diagnostics, soft placement links, undo and save/reopen behavior as a designer editing source. A gesture produces a targeted source patch and preserves unrelated comments and formatting. [Authoring and placement](schematic-and-placement.md)
- **Readability is part of the document.** Symbols, glyphs, wires, junctions, ports and labels have geometry in sheet units. A warning can point to an obscured pin name or the measurable gap where a wire looks attached. A readability edit must preserve electrical connectivity unless its preview explicitly shows a connection change. Suggestions about signal flow respect the conventions of the sheet and the kind of circuit.
- **Placement intent can be soft and pin-specific.** A bypass capacitor can serve one supply pin of one MCU occurrence. Library guidance and the drawn circuit can supply that relationship. When several assignments fit, show the evidence and let the designer confirm one on either canvas. Preserve the pairing when either component moves. A shared power net alone is insufficient evidence. Side preference, measured land-to-land distance, and a routed supply/return path are distinct quantities.
- **Give an exact, current answer every frame where feasible.** Treat a drag as an edit to a transient revision. Update each affected rule, remove obsolete findings and retain findings whose inputs are provably unchanged. A warning identifies the offending pair, measurement, limit and source. An all-clear requires current coverage of every enabled, applicable rule. Record how frequently the entire verdict reaches the screen within a frame. [Measurements and latency targets](live-checks.md)
- **Keep electrical and mechanical identity across edits.** Give parts, pads, pins and occurrences stable identities with explicit provenance. Separate intended nets, physical copper contact and signal paths. Map a board connector's logical pin to its land, installed mating frame and harness cavity. Source-linked simplified geometry can stand in for a detailed mechanical body while the connector interface remains available. [Backend architecture](backend.md)
- **Extend the existing language proof and keep the production build dependency-free.** Establish graphical/text edit equivalence, stability of qualified references, sound invalidation and equality between incremental and fresh rule results. Keep external ECAD tools and reference solvers in verification. [Proof obligations](language-proof.md)

## Starting from a pin

Selecting a pin exposes what its library definition wants and gives. Both may apply to one pin. A regulator output gives power and can want a local output capacitor. A passive terminal acquires a job when it fulfills a particular want. **Add bypass capacitor** should create the occurrence, connect supply and return, and record which pin or pin group it serves in one undoable source edit. The designer can make the same connection freehand and inspect suggested associations.

A want or give names a **declared role type**. For example, a regulator pin can give a 3.3 V rail and want a 100 nF bypass at the same time. Roles carry a stable identity, a checked kind and optional typed ratings. A fulfillment must match the same role identity at both endpoints. Libraries share an identity explicitly. CLI, canvas and MCP consumers read the same structured role objects, including units. The [native schematic reference](../../docs/schematic.md) gives the declarations and wire format.

Show a rooted view from any selected pin. Keep electrical attachments undirected, distinguish ordered functional stages from parallel peers, and represent shared nodes or feedback edges by reference. Reversing a wire gesture preserves connectivity and functional intent. PCB placement reads the confirmed endpoints and their preferred metric, including after reannotation.

## First acceptance test

Start from an empty sheet. With the graphical editor, draw two repeated MCU channels that share a power rail, a bypass capacitor in each channel, a bulk capacitor and a connector. Cover one pin name with a label. The editor should identify the visible clash, preserve the electrical connections when the label moves, and explain which capacitor-to-pin assignments it can infer. Confirm one pairing graphically.

On the board, drag that capacitor past its preferred distance, through a neighbour's courtyard, and back. The distance warning and the collision must appear and disappear independently, with the measured geometry highlighted. Undo, save, reopen, and perform the equivalent change as a text edit. Compare the pin partitions, placements, relationships and complete finding sets with a fresh solve. Install two instances of the board and move the connector on one installed instance through an assembly placement change. Verify that the harness follows the correct occurrence. Record time from input to visible result and the coverage of the displayed revision.

## Evidence and release criteria

A verification probe replayed 4,000 via moves on the published Koyomi and Inkplate boards. For one fixed circular via-land clearance rule, the incremental and fresh violating-pair sets agreed on every move. On Inkplate's 393 vias the incremental p95 was 1.64 µs. A separate planar Boolean probe took 82.14 ms median for 112 circular cutouts. These measurements justify testing both localized work and long-range effects from zone refills. Proposed frame budgets and exact probe scope are in the [live-check design](live-checks.md) and [reproduction instructions](probes/README.md).

The first professional pilot should use a previously shipped board with its approved parts, accepted manufacturing outputs, enclosure and mating harness. Compare the regenerated release with the accepted package, including variant-specific BOM, drill and placement outputs. Record lost semantics and manual repairs during import. Firms that promise editable Altium sources have a different adoption condition from firms that control their choice of editor. [Professional workflows and firsthand sources](professional-workflows.md)

## Decisions still open

- Which library or project data establishes the preferred distance for a given bypass connection? How should the inspector rank competing inferred pairings when a power rail has several consumers?
- Which KiCad rule constructs, padstacks, and manufacturing processes must round-trip for the first release? Which unsupported constructs prevent sign-off?
- Which native and browser reference machines define the 60 Hz and 120 Hz gates? How often do dense layouts and remote zone-island changes meet a whole-board frame deadline?
- Which electronics query should be the first to carry a Lean proof and a Rust differential oracle? The via clearance probe provides a small worked candidate.

The [implementation sequence, fixtures and public operation contracts](backend.md#11-verification-scale-and-sequencing) describe how to grow beyond this first board.

## Editor boundaries and backend readiness

Schematics and symbol libraries are separate editable Forge documents. Opening one selects its own canvas, ribbon, browser and properties in the existing window. This follows the drawing document and tab lifecycle described in the [UI standard](../../docs/ux/STANDARD.md). A schematic places references to library symbols. A library save can refresh the schematic across open tabs. The file on disk is the same source used by the CLI and MCP.

The board should be a third, linked layout document. A package on the board should resolve a physical component occurrence and its logical pin to a footprint and pad. The board should own the stackup, footprints, routing and physical constraints. Its generated mechanical representation can then be placed in an assembly and expose connector interfaces to a harness. Keep preferred component proximity and return paths attached to typed schematic relationships so they survive movement on a sheet. Implement persistent physical identity and pin-to-pad resolution before offering a board workspace.

| Area | Available in this implementation | Work still needed |
| --- | --- | --- |
| Symbols and parts | Bundled power and ground symbols, project-local reusable library files, multiple units, numbered pins, graphic primitives, typed wants and gives, fields and datasheets | Searchable managed catalogs, lifecycle and approved alternates, footprint library and pin-to-pad validation |
| Schematics | Hierarchical sheets, gates, wires, junctions, labels, buses, ports, notes, named nets, functional links, ERC, independently positioned reference and value fields, text collision warnings and source-backed undo | Multi-sheet physical reannotation, bulk and multi-object editing, interactive wire dragging, project-scale cross-probing |
| Purchasing | Inherited string fields with occurrence overrides, content-addressed local PDFs, URL references, fitted and BOM flags, grouped occurrence-correct BOM with CSV | Named assembly variants, supplier data pipelines and release snapshots that include referenced assets |
| Layout and performance | Logical occurrences, terminals, connections, confirmed placement preferences, dimensional coordinates | Copper and clearance geometry, routing, placement checking, stable ECO interchange and measured interactive budgets on large designs |

The [editing guide](../../docs/schematic.md#editing-in-forgecad) includes the sensor interface example. Backend and GUI tests exercise saving and reopening the source, building the connectivity graph, and resolving the effective BOM. The editor compiles committed changes. The release criteria for frame-rate checking on a populated board still require measurement with actual board geometry.


The review set uses the same analog interface in both renderers. It opens the component and pin through the document browser at three window sizes.

| Window | Desktop | Browser |
| --- | --- | --- |
| Schematic, U1 selected, 1600 × 1000 | [Desktop schematic](shots/native-schematic.png) | [Browser schematic](shots/web-schematic.png) |
| Symbol, VDD selected, 1400 × 860 | [Desktop symbol](shots/native-symbol.png) | [Browser symbol](shots/web-symbol.png) |
| Schematic, C2 selected, 1024 × 700 | [Compact desktop](shots/native-compact.png) | [Compact browser](shots/web-compact.png) |

The [readability example](shots/native-readability.png) places the note `scope_probe` across the output wire `w7`. Electrical Checks links to each object. Moving the note clear of that wire removes the overlap warning and preserves the electrical net.
