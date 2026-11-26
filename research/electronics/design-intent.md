# Design intent across .fg, schematic and PCB

**Milestone 1 · research and design proposal · 25 November 2026**

When an engineer draws a bypass capacitor between VDD and GND, four different things may matter later. Its terminals belong to two nets. The drawing places it beside a particular MCU pin. The engineer may intend it to serve that pin. A manufactured board needs a supply and return path with suitable geometry. A shared netlist preserves the first fact. A picture preserves the second. ForgeCAD needs to represent all four, with their provenance, so an edit in one view has a predictable meaning in the others.

The first research question is whether ForgeCAD can recognize **what a component is doing for another component** from what an engineer drew. The cross-view editing contract below describes how to retain and act on the answer.

## Inferring which pins a capacitor serves

**A reference case with an answer from the designer.** In Raspberry Pi’s minimal RP2040 design, Figure 5 draws C9 and C14 as 100 nF capacitors on the same 3V3 and ground rails. The local branches show how the caps sit relative to the power pins. The authors explicitly say **C9 serves both package pins 48 and 49**. Figure 6 shows the implemented layout and the text explains the escape-routing trade-off. The same design also uses C1 and C4 at the external regulator input and output, and has special decoupling requirements for the MCU’s internal regulator. A rule that assigns every capacitor to its nearest IC, or insists on exactly one cap per supply pin, loses information the designer actually used. [Hardware design with RP2040, §2.1, figures 4–6](https://datasheets.raspberrypi.com/rp2040/hardware-design-with-rp2040.pdf#page=8)

**Proposed inference.** Start on the schematic, before any parts have board positions.

1. Identify a **role to fill** on an actual part and package. Read component-library pin roles and available manufacturer guidance. A local I/O supply bypass, a regulator output capacitor and a bulk rail reservoir can all be wired across the same nominal supply and return. Each implies different placement advice. Where library metadata is sparse, report how specifically the remaining evidence identifies the role.
2. Preserve the **drawn branch before flattening the net**. Starting at a capacitor terminal, trace wires and junctions to candidate load pins. Record which pins are reached on the local branch, which require following a common rail or crossing a named port, the return domain, explicit functional grouping, repeated-channel membership and any nearby annotation. A labelled local branch directly serving a pin is stronger evidence than raw distance between two symbol centres. Drawing conventions vary; a capacitor bank on a dedicated power sheet may identify only a chip or rail.
3. Compare the **competing explanations together**. Use pin compatibility, package-specific guidance, value range where known and local branch evidence to propose a relation with a target pin, pin group, component, or rail. Allow a cap to cover two pins, several caps to serve one pin, and interchangeable caps to remain a set. A series filter, different return domain or incompatible pin role can eliminate a candidate. Shared rail membership alone leaves ownership unresolved. Electronics knowledge should rule out bad explanations before geometric proximity breaks a tie.
4. Carry a **soft relationship with a reason** onto the board. For example, “C9 likely bypasses these two supply pins; highlight their drawn branches.” An engineer can leave it inferred, confirm it, correct it or mark a deliberate exception on the canvas. The board pose measures whether the proposed relationship is being satisfied. Dragging a part away cannot silently transfer ownership to the next-nearest IC.
5. Translate “close” into a **named measurement**. Before routing, show the distances from the capacitor’s supply land to each target land, and show the relevant return connection where known. After routing, inspect the supply path, return path, and vias. If the datasheet gives only qualitative guidance, preserve that and give the movement delta. A numeric pass/fail requires a sourced limit or a project preference with a specified metric. A short air-line distance cannot certify a good high-frequency loop.

**First experiment.** Give the inference pass the schematic and permitted library data. Withhold the reference layout and the captions or design notes that explicitly give the answer. Use those as independent checks afterward. Include the RP2040 shared-cap case, a private supply after a ferrite bead, a regulator with input/output capacitors, a separated power sheet, two repeated channels, and genuinely interchangeable capacitors. Have a reviewer label role, target granularity and acceptable alternative assignments. Compare net-only, page-nearest, and role-plus-branch methods on held-out designs. Track exact pin/group hits, useful component-level hits, ambiguous results and confidently wrong assignments separately. Redraw an electrically identical example with different wire bends, and deliberately drag a correctly paired capacitor toward another IC on the PCB. Neither irrelevant edit should manufacture new evidence of ownership. Report actual outcomes before shipping automatic pin-specific warnings.

The same approach can later describe a crystal and its load network, a feedback divider and sense pin, or ESD protection at a connector. Each role supplies its own electrical evidence and physical measurement.

## Datasheet requirements and alternate pin functions

**Proposed extension.** Today a pin lists declared role IDs. A role can carry one voltage in millivolts or one capacitance in nanofarads. A confirmed fulfillment checks the role IDs, the common net and any explicit return. Component values such as `"10k"` are display strings. There is no present basis for a checklist to certify the resistance of R7, the operating voltage at its far end or the state of an MCU pad after reset. Keep the current nominal role for intentional ownership. Add typed requirements and guarantees for compatibility across independently authored libraries. [Reader and types](../../interface/forge-lang/src/schematic/mod.rs) · [Fulfillment checks](../../interface/forge-lang/src/schematic/graph.rs)

The analog example already contains a concrete test. Its TLV9001IDBVR library pin requests a role keyed to the example’s 3.3 V rail. TI specifies a 1.8–5.5 V supply span and recommends a local 0.1 µF bypass. A designer can legitimately use this op-amp at 5 V. Put the operating span between the two supply pins in the reusable symbol, keep the choice of 3.3 V in the project, and give the capacitor its own role. Show a separate board placement preference when the datasheet says “close” without giving a numeric distance. [Analog library](../../examples/electronics/analog/analog-symbols.fg) · [TI TLV900x datasheet, Power Supply Recommendations and Layout Guidelines](https://www.ti.com/lit/ds/symlink/tlv9001.pdf)

### What a library author declares

| Subject | Example of an authored fact |
| --- | --- |
| Physical pin | Package pin 17 belongs to the VDDIO bank. Its absolute input bounds depend on VDDIO. These limits apply to every function. |
| Function of that pin | SDA uses an open-drain driver, accepts lows and highs, and needs a pull-up to the selected I/O domain. UART TX on the same pin drives push-pull and has a different set of guarantees. |
| Operating state | During reset the pin is high impedance with an internal pull-down. Firmware can enable a pull-up later. Record the assumed configuration and the phase in which it takes effect. |
| Two-terminal device | This resistor offers an unordered pin pair, resistance and tolerance. A diode offers an ordered anode/cathode pair. A resistor array declares individual pairs and any shared terminal. |
| Evidence | Each limit carries the part/package, datasheet revision, page or table, condition and whether the value is guaranteed, typical, recommended, or an absolute maximum. Project exceptions carry their own rationale and scope. |

Select a function on each **placed occurrence**. Save its stable library function key. The library defines the default and the legal options. Invariant pad limits and the selected function's requirements apply together. An inactive function contributes no function-specific drive, pull or requirement. The physical pin number, net, wires, board-pad link and existing designator survive a switch. If a library update removes a selected function, surface the unresolved selection. KiCad 10 already allows changing the displayed name, electrical type and graphic style per placed pin. ForgeCAD can make the selected function change the checks and the direction overlay as well. [KiCad 10 schematic editor, Alternate Pin Function Definitions](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#alternate_pin_function_definitions)

### Three predicates for the first implementation

1. **Voltage across two terminals.** A consumer accepts an inclusive operating interval relative to its return pin. A source offers a guaranteed interval between its output and return under declared load and temperature conditions. A specific voltage is a single-point interval, and a nominal value with tolerance expands to an interval. Verify containment of the entire offered interval plus the return connection and required domain. A partial overlap gives a worst-case violation of the requirement. A nominal-only source leaves tolerance-dependent results open. Keep absolute maximum and recommended operating range separately visible. A net label such as +3V3 is a name and can appear on an undriven net.
2. **A connection through a device.** From a particular terminal, follow its net to a fitted device pin, cross a declared pin pair of the requested type, then follow that pin's net to the target. Accept an exact target pin, or an explicitly scoped target such as this chip's VDDIO domain. Check each device parameter and tolerance against its typed bounds. Return the qualified endpoints, device and pin pair. Keep the two net IDs distinct. Check any declared exclusions, such as a parallel short around a required series resistor. Start with one device and compose bounded, ordered steps for later networks.
3. **Bias in a named state.** Model `wants pull-up`, `wants pull-down`, `provides internal pull-up` and `provides internal pull-down` separately. A need specifies its destination domain, acceptable effective resistance, allowed internal/external sources, and reset/run/sleep condition. Identify all participating pulls and calculate the parallel bounds for resistors to the same potential. Opposing pulls form a divider and need a threshold check with input leakage. A configurable pull contributes only when its enable state is known. A requirement for an external fitted part still needs its own path witness.

Extend the existing quantity parser and driver provenance for voltage and resistance first. Add current, capacitance, time and frequency as check families need them. In source, units such as `3.3V`, `6800ohm` and `6.8kohm` should produce dimensioned values and source spans. Keep the printed Value independently editable, and let the UI generate it from a typed value when requested. Use explicitly closed bounds and conservative arithmetic at comparisons. Record the reference terminal for voltage and direction for current. [Forge quantities](../../interface/forge-lang/src/values.rs)

Here is one candidate spelling in `.fg`. These declarations describe the teaching fixture below. They illustrate the proposed syntax.

```fg
io_supply = supply(relative_to: VSS, operating: range(1.8V, 3.6V))
sda_bias = pull_up(to: VDDIO, resistance: range(4.7kohm, 10kohm), source: external)
series_out = path(to: peer.IN, through: resistor(resistance: range(22ohm, 47ohm)),
                  count: 1, allow_bypass: false)

# Inside a library symbol's pin map
IO3: pin(number: "17", kind: "input", at: (0, 0), functions: {
  SDA: function(kind: "open_drain", wants: [sda_bias], gives: [i2c_data]),
  TX:  function(kind: "output", wants: [series_out], gives: [uart_data])
})
# Select a function when placing a symbol. Bind peer to the intended receiver
# in the occurrence's interface mapping. The wire still attaches to U1.IO3.
U1 = place(sheet: main, symbol: parts.MCU, functions: {IO3: TX}, at: (10, 5))
```

`relative_to` and `to` resolve scoped terminal references. `i2c_data` and `uart_data` are previously declared interface capabilities. The symbol author would put `io_supply` on VDDIO. A source library can independently publish its output range and return. A diagnostics row can then quote the two declarations and the measured containment margin. If the MCU datasheet requires an open drain only when a peripheral configuration bit is set, the function carries that condition.

### What the designer sees

The symbol editor provides **Add requirement** and **Add guarantee** under each function. Choosing *Connection through part* exposes fields for endpoint, device type, pin pair and parameter range. Limits can reference another pin or a library parameter. The schematic pin inspector offers a function picker, a state selector and a checklist assembled from the resulting predicates. Clicking a row highlights its specific terminals and parts across sheets. The same typed result powers text diagnostics and MCP queries. A unique electrically sufficient witness can satisfy a check directly. Reserve manual assignment for ambiguities or intentional ownership such as which capacitor serves which power pin.

Consider a teaching fixture with U1.IO3 selected as SDA, a source guaranteeing 3.201–3.399 V relative to VSS, an allowed bank supply of 1.8–3.6 V, and a project pull-up requirement of 4.7–10 kΩ. These are fixture inputs, independent of any manufacturer's SDA specification.

| Edit | Inspector result |
| --- | --- |
| Fit R7, 6.8 kΩ ±1%, from SDA to VDDIO | Supply range passes. Pull-up passes with 6.732–6.868 kΩ. Selecting it highlights U1.IO3 → R7.1 → R7.2 → VDDIO and identifies the source. |
| Move R7's far pin to 5 V | The value check still passes. The destination check fails, even though both nets have supply symbols. |
| Add another identical pull-up in parallel | Each resistor meets the individual range. Effective resistance becomes 3.366–3.434 kΩ, below the minimum. Show both reference designators. |
| Leave the source tolerance unspecified | The connection is known. List the missing guaranteed output bounds before awarding a supply pass. |
| Mark R7 unpopulated in a variant with no other pull | That variant loses the external path. An enabled internal pull can satisfy only an obligation that permits it and accepts its guaranteed range. |
| Switch this occurrence to UART TX | Keep its wire and pad. Remove the SDA pull-up obligation, check the new drive mode against every other enabled driver and flag conflicting peripheral assignments. Undo restores the SDA checks. |

The same checker should answer **“What can I connect here?”** While starting a wire, inspect compatible destination pins and available peripheral choices in the current variant. If a direct connection fails, show the first conflicting guarantee, or a specific missing bridge such as a level translator between two referenced I/O domains. Preview a fix on a temporary revision, recalculate affected checks on both ends, and let the designer accept one undoable edit. For an I²C pair, preview both pins together so a legal SDA choice cannot strand SCL.

### Next checks worth adding

- **Logic margins and power-off behavior.** Compare the sender's worst-case VOH/VOL at the actual load against the receiver's VIH/VIL for its bank voltage. Check whether an attached device can back-power an unpowered bank and surface any clamp-current limit.
- **Resource budgets and shared buses.** Sum worst-case active loads on a regulator and drive group. For I²C, use effective pull resistance, bus capacitance and sink capability to check rise time and low-level current at the selected bus speed. A missing capacitance estimate leaves the rise-time result open. [TI, I²C Bus Pullup Resistor Calculation](https://www.ti.com/lit/an/slva689/slva689.pdf)
- **Pin groups and mutually exclusive choices.** Bind SDA and SCL to the same peripheral instance, enforce differential P/N pairs, reserve debug pins, and detect two devices answering at the same fixed address. Offer compatible free pins with the reason each alternative qualifies.
- **Startup and sequencing.** A strap must be at the required level when sampled. An enable may require power-good from a particular source. A design that works in the run state can fail during reset or power-down.
- **Local networks with board obligations.** A regulator output capacitor can require effective capacitance under DC bias and an ESR interval. A series termination can require a resistor of the right value close to the driver. Report electrical suitability, component ownership and board distance as individually inspectable results.

### Implementation and proof boundary

Start with a small native checker for interval containment and one-device paths in a fitted occurrence graph. Index device pin pairs by net and type. Cache each verdict by the identities and revisions it read, including target domain, active function, selected operating state, variant and device value. Track candidate nets as well as the matched path so adding a parallel pull invalidates an existing pass. Moving text preserves these results. Cutting a wire, changing R7 or switching one pin function invalidates the affected checks. Keep unknown, contradicted and satisfied results distinct and show the offending value or missing input.

Extend the existing Lean schematic model with dimensioned intervals, effective functions and witnesses over net and device edges. Prove that a satisfied interval accepts every guaranteed value, switching a function preserves the terminal partition, and a series-device witness never merges its two nets. Differential-test Rust against exhaustive small fixtures, including equality at a bound, reversed diode, disconnected return, a shunt around a series part, a DNP bridge and a removed pin function. Compare incremental and fresh results after edits and undo. The first graphical acceptance test is the SDA fixture above, drawn without writing source, saved, reopened and inspected via MCP. [Current schematic proof](../forge-lang/lean/ForgeLang/Schematic.lean) · [Live-check design](live-checks.md)

## What the prior art actually encodes

### KiCad separates symbol identity, page geometry, board geometry and synchronization

On the published Koyomi board, schematic C2 is a `Device:C` at (85.09, 142.875) with symbol UUID `ca7063f9-4ead-4a93-8b73-19c443a64cb5`. Its PCB footprint has its own UUID `0e7617b2-2e1f-4f14-9378-fb5ea9cdfd54`, board position (113.2575, 49.4) and orientation 180°. Its `path` names the schematic UUID. Board pads 1 and 2 carry +3V3 and GND. Neither the two positions nor the printed text “C2” is the join. KiCad 10 documents normal linking by unique identifier, explicit reference-based relinking, and a bounded set of changes that can be back-annotated. Deleting and recreating a symbol with the same designator can require a deliberate relink. [Koyomi schematic](https://github.com/TensorFleet/koyomi-lvds-hat/blob/bd6af086f8528749ccd98f745891a731011d5fac/koyomi-lvds-hat.kicad_sch#L15084) · [Corresponding footprint](https://github.com/TensorFleet/koyomi-lvds-hat/blob/bd6af086f8528749ccd98f745891a731011d5fac/koyomi-lvds-hat.kicad_pcb#L218) · [KiCad 10 forward and back annotation](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html)

**Test on copies of the source file.** Using the locally available KiCad 9.0.2 CLI, export each version with `kicad-cli sch export netlist --format kicadxml`. Compare the complete partition of (reference, pin) pairs from the XML, mapping C202 back to C2 in the rename case. The partition ignores net numbering and XML order. This tests schematic compilation; the footprint link above was inspected in the published board file.

| Single change to C2 | Exported nets / pin entries | Observed electrical effect |
|---|---:|---|
| Original | 96 / 253 | C2.1 on +3V3 and C2.2 on GND |
| Move the printed reference 10 mm in X, leaving the symbol origin fixed | 96 / 253 | Identical partition |
| Rename C2 to C202 in its reference property and instance record | 96 / 253 | Identical partition after the explicit rename mapping |
| Move the symbol origin alone to (80.017, 144.333), leaving the wires behind | 98 / 253 | Both C2 pins acquire separate unconnected nets |

The last two moves might both start as a drag. One edits ink. Another leaves two actual endpoints behind. A gesture, an API caller, and an undo record must preserve which operation the designer performed.

### LibrePCB gives the circuit a first-class record

In the published D0 reader example, `circuit/circuit.lp` gives C1 the component instance UUID `833ecbcf-4fa8-4d41-8a7b-76d8aa2aca38`, its library component and variant, and two signal-to-net bindings. In `schematics/main/schematic.lp` the drawn symbol has its own UUID and refers to that component and a library gate. In `boards/default/board.lp` a device record uses that component instance ID, selects a library device and footprint, and supplies placement. An actual copper trace endpoint refers to the device instance **and** pad `466e0fd0-5b85-4d68-84d3-efa655938898`. The package/device mapping lets the application follow a connection through different library representations. This is a concrete precedent for preserving independently editable drawing and board records bound to shared logical objects. [Pinned example project archive](https://github.com/LibrePCB/librepcb-example-projects/blob/3aad10dae67ecfea9535a2cac328497d67e4d142/d0-reader.lppz) · [LibrePCB library model](https://librepcb.org/docs/quickstart/create-library-elements/)

### tscircuit and atopile expose what survives compilation from code

In tscircuit's design048, the source connects a VCC label to `sel.TP1.pin1` and separately places it on the schematic. In the published circuit JSON, `source_trace_0` names `source_port_0` and `source_net_0`. Both `schematic_port_0` and `pcb_port_0` point to `source_port_0`. The plated hole points to `pcb_port_0`. This gives a concrete path for cross-probing from rendered copper back to a logical terminal. The serialized IDs demonstrate correspondence **within this artifact**. Stability across edits to the generating program is a separate contract. The board API also accepts serialized manual layout/routing edits. ForgeCAD has to decide what happens to such an edit when a generator changes. [TSX fixture](https://github.com/tscircuit/schematic-corpus/blob/f7c86af567e46611d80e1bd811043499dc11b34c/designs/design048.circuit.tsx) · [Published circuit JSON](https://unpkg.com/@tscircuit/schematic-corpus@0.0.114/dist/designs/design048/circuit.json) · [Source trace schema](https://github.com/tscircuit/circuit-json/blob/e41c36c6c4632afd638a8388ea7433850e02ae2a/src/source/source_trace.ts) · [Manual edit API](https://docs.tscircuit.com/elements/board)

atopile's documented power-filter example declares an `ElectricPower` interface, connects a capacitor and resistor across it, and assigns voltage and component constraints. The interface preserves a relationship between supply and return that two flattened nets cannot recover on their own. This suggests a place for typed buses, rails and differential pairs in ForgeCAD. “Belongs to this power domain” and “decouples this particular package pin” still express different knowledge. [atopile language reference and worked filter](https://github.com/atopile/atopile/blob/main/.claude/skills/ato-language/SKILL.md)

### Altium treats differences between views as engineering decisions

Altium's documented comparator, component links and engineering change orders provide an explicit synchronization step. Its pin/pair/part swapping workflow illustrates why reverse edits matter. A layout engineer can improve routing by selecting a legal swap. The library defines equivalent members, and a net reassignment must reach the schematic. An arbitrary contact made by dragging copper has an entirely different meaning. Schematic classes carried to PCB placement rooms show how a functional association can also cross domains without changing net membership. These observations are about the published workflow. [Synchronization and ECOs](https://www.altium.com/documentation/altium-designer/sch-pcb/keeping-synchronized) · [Pin, pair and part swaps](https://www.altium.com/documentation/altium-designer/sch-pcb/swapping-pins-pairs-parts) · [Classes and rooms from a schematic](https://www.altium.com/documentation/knowledge-base/altium-designer/creating-user-defined-component-classes-and-rooms-directly-from-the-schematic)

## Proposed contract for ForgeCAD

Use a native `.fg` project with **one authoritative declaration for each authored fact**. Several files, reusable definitions, and occurrence overrides can participate. A single monolithic source file is unnecessary. Text is an authoring surface for *all* persisted facts. Schematic and PCB gestures are typed, scoped edits to the facts they own or explicitly propose to change. Compile revisioned views with links back to declarations, exact inputs and occurrence paths. This proposal leaves file partitioning and identity encoding open for comparison.

| Fact | Encode and retain | Permitted reverse edit |
|---|---|---|
| Definition, occurrence and terminal | Definition key, instance path, logical interface member or pin, library revision; independent display name. Map logical terminal to symbol gate/pin and device/package land or lands | Reannotate updates labels and references. Replace a library device compares pin maps. A missing or ambiguous map produces an explicit unresolved link |
| Electrical intent | Scoped terminal connections, junction/port/label semantics, explicit no-connects, bus members and legal swap groups. Derive the pin partition and aliases | Drawing an attached wire changes connectivity. A PCB pin swap proposes a checked mapping edit with its logical and routing consequences. A copper short reports a disagreement with intent |
| Schematic presentation | Per-sheet symbol poses, wire paths, anchored labels, text poses, visible junctions and functional groups. Distinguish decoration from electrical attachment | Dragging label text can preserve attachment; reattaching the label can change net membership. Keep manual geometry and locks when the circuit is regenerated |
| Physical implementation | Per-board device realization, pad stack, pose, layer, copper segments, vias, pours and keepouts. Measure realized connectivity independently | Moving a footprint changes its physical pose and dependent routes/checks. Preserve hand routing where anchors survive and report detached or obsolete ends |
| Functional intent and constraints | A typed relation with occurrence-qualified endpoints, role, applicability, strength, quantity/metric, and provenance. Record declared or confirmed intent. Derive suggestions and carry their evidence separately | Confirming a bypass association on either canvas writes the same relation. Moving a capacitor updates its distance finding. It cannot change the confirmed owner merely by approaching another MCU |
| Generated or externally owned data | Definition and call site, keyed occurrence, parameter provenance, variant, imported source keys and last synchronized revision as applicable | A single driving site can yield a direct patch. Shared or computed values require a scoped operation. An upstream replacement that removes a target yields a conflict with a repair location |

### A cross-view example

Consider C17 and an MCU with two VDD pins. These are illustrative records, independent of the proposed surface spelling. A graphical designer can produce every authored record with ordinary placement, wiring and an inspector.

| Meaning | Example authored record | What each editor can show |
|---|---|---|
| Logical attachment | Channel B, C17 terminal 1 and U3 terminals VDD1 and VDD2 belong to rail 3V3 | Text shows the wired interface. The schematic draws its connected branches. The PCB highlights the assigned pads. Physical contact still depends on copper |
| Graphical explanation | On the power sheet, the C17 wire terminates at the VDD1 branch's junction; its label is offset above that segment | Moving the text retains the endpoint. Moving the attachment to another net changes the connection. Drawing the capacitor next to VDD1 is useful evidence of purpose |
| Intended role | C17.1 decouples U3.VDD1 in channel B; the return is U3.VSS1; this project prefers a 3 mm pad-centre distance | A source declaration, a relation picked on the schematic, or a PCB inspector confirmation all yield one persisted fact. Another capacitor on 3V3 does not inherit the assignment |
| Physical realization | The selected device maps VDD1 to land 14; the board stores C17's pose and its routed copper | The PCB can measure land-to-land distance before routing, then evaluate the routed supply and return separately. A long route can fail its own rule even when placement is close |

From a simple netlist, only the first row's electrical membership is available. From a routed PCB, a short pad-to-pad distance can suggest the third row but cannot establish it. A confirmed relation survives a replacement whose validated pin map preserves the endpoint; loss of that pin invalidates the relation with a source-linked diagnostic.

### What the three storage strategies imply

| Strategy | What it handles naturally | Where to test it hardest |
|---|---|---|
| Regenerate both views entirely from code | Hierarchy, parametric arrays, repeated channels and deterministic regeneration | A drawn wire bend, tuned trace or single-instance placement must survive a change to the generator. Every persisted manual choice needs an addressable owner |
| Give each document an independent model and reconcile through ECOs | Separate authors, external file formats, inspectable forward/back annotation and explicit conflict review | A simple native edit can produce two competing copies of the same assignment. Split, merge and stale-link semantics need explicit reconciliation |
| Share authored logical declarations and persist the manual choices of each view | Text and graphical edits use one logical identity, while page geometry, copper and instance overrides can evolve independently | The hard case is the inverse of a generated expression. Identity, scope, provenance and orphan handling determine whether this survives real projects |

The third strategy is the working candidate for native projects. Use change proposals at external or independently revised boundaries. The study should overturn this choice if a realistic edit cannot be represented and reversed without losing manual work.

**One difficult inverse edit.** Suppose two channels instantiate the same regulator definition. Their capacitor locations derive from a shared offset. The designer drags only channel B's capacitor. An unconditional edit to the offset moves A as well. Writing a literal into every generated instance destroys the reusable design. The edit resolver should explain the driver, preview its affected occurrences, and allow a keyed B-only placement override or a deliberate edit to the shared source. The override should survive inserting a third channel. Deleting B should expose an orphaned override. A source span locates the statement for a human; the occurrence key identifies the intended target across revisions.

**One conflict between authors.** From the same base, a board engineer requests a pin swap while a schematic engineer changes the selected device. Compare the base pin map with both edits. Apply automatically only when the preconditions and resulting mapping still agree. Otherwise retain both changes as a typed conflict and show the affected terminals, physical pads, and routes. Concurrent changes to a sheet label's typography and a board footprint's position can commute if their read/write sets are independent. These cases constrain the design of undo, synchronization and incremental solving before choosing a database.

**Reuse the language that already exists.** ForgeCAD already provides `Dim`, numeric driver provenance (`Src`), `ParamKey`, typed `Op`s, checked source edits and a model of structural observations. Board and schematic locations both measure length; frame and occurrence are properties of a reference or transform. The design question is which *electrical* quantities or relations need extensions, and how new operations satisfy the existing proof assumptions. For a generated placement, a reverse edit has to account for drivers and control dependencies before offering a patch. [Dimensions and provenance](../../interface/forge-lang/src/values.rs) · [Typed operations](../../interface/forge-lang/src/error.rs) · [Checked source edits](../../interface/forge-lang/src/edit.rs) · [Language assumptions and counterexamples](../forge-lang/ASSUMPTIONS.md)

## Cross-view contract review

1. **Produce an ownership and identity map.** Cover component definitions and occurrences, symbols and gates, pins and pads, rail/bus members, physical copper, visual wires, soft relations, variants, and connector cavities. Mark every field as authored, imported, confirmed, inferred or derived where applicable. Specify creation, rename, deletion and orphan behavior. Compare syntax-path identity with explicit persisted keys for generated instances; document counterexamples and the chosen hybrid or alternative.
2. **Write side-by-side native encodings for the same real circuit.** Work through an archived board with its matching schematic, and a repeated-channel design expressed from reusable code. Start once with text, once with only graphical commands, and once with existing routed artwork. Preserve pin partitions, drawing geometry, board edits and lock state. Document which intent can be recovered from each input. An imported netlist lacking a page layout or a capacitor-to-pin assignment must leave those facts unknown.
3. **Write the edit matrix in both directions.** Include rename versus delete/recreate, label move versus reattachment, instance edit versus shared definition edit, legal pad swap versus accidental short, a differential pair across layers, DNP variant, library pin-map change, dangling override and two edits based on the same revision. For each, show the authoritative patch, preserved manual work, derived changes, refusal conditions, diagnostics and undo result.
4. **Specify what equivalence and freshness mean.** Independently compare logical partitions up to the stated identity mapping, anchored schematic geometry, pad mappings, board realization, soft-link provenance and coverage of findings. A netlist-only equality test misses drawing loss and changed placement intent. Identify query dependencies for each edit, the fresh reference result, how reverse edits fit the language proof, and how a partially current verdict is exposed.
5. **Review the alternatives with evidence.** Compare fully regenerated views, independent documents reconciled by ECO, and shared logical declarations with persisted view-specific edits. Each must explain the same generated-instance, imported-board and concurrent-swap examples. Record the chosen serialization, backward-edit policy and remaining gaps in this design record. That decision gates the first production backend slice.
