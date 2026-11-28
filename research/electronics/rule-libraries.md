# Rust electronics libraries

ForgeCAD uses Rust to author electronics libraries. One package can export child libraries, properties, ERC rules, and DRC rules. Each export is optional. Projects select a package or a child through the existing `.fg` library declaration. Source libraries containing symbols and footprints can import Rust packages and re-export them through their existing aliases.

## The Standard Library

`forge:standard` is the bundled root. It contains `properties`, `erc` and `drc`. The properties library has a `ratings` child. ERC includes connectivity, buses, wiring and readability, voltage, pin intent, resolved attributes, signals, placement, and supply ratings. DRC includes copper and manufacturing. Each child owns its rule IDs and default severities. The existing schematic graph and board geometry engines supply the resolved nets, fitted occurrences, and spatial witnesses. Shared checks can inspect related facts in a single pass. [Standard Library](../../interface/forge-lang/src/standard_library/mod.rs)

Schematic, symbol, board and footprint documents start with the complete standard rule library. Schematic and symbol documents expose standard types under `std.properties` by default. Existing explicit imports and local declarations named `std` keep their meaning. Projects can override individual severities and add other libraries. Explicit imports below also provide shorter names.

```forge
ratings = library(file:"forge:standard/properties/ratings")
review = library(file:"forge:standard/erc/supply_rating")
```

Bind an ordinary symbol field with `field_types:{MaxSupply:ratings.maximum_supply_voltage}`. A symbol or placement can set `fields:{MaxSupply:"3600 mV"}`. The symbol editor shows loaded types alongside custom fields. The reader validates the value against the selected type. BOM and schematic-to-board queries return the original field and a canonical typed value. The typed record includes the source type reference and a stable `type_id` for the exported Rust property, so two import aliases can refer to the same property. The supply check compares the upper end of each fitted provider's declared voltage against the rating on each power input. A missing supply rating leaves this particular comparison inapplicable. [Supply-rating example](../../examples/electronics/supply-rating.fg)

The standard address check groups fitted device occurrences by **SDA net, SCL net, and decoded 7-bit address**. Two devices at 0x48 on different buses are valid. Sharing just one of the two nets also leaves the groups distinct. A collision identifies every participating attribute. Population variants use their own fitted graph. Bus identity currently follows electrically resolved nets; switchable mux channels require an explicit model of channel connectivity. The uniqueness check uses the same typed Rust check interface available to external libraries. [Bus/address rule](../../interface/forge-lang/src/standard_library/i2c.rs) · [Two live buses](../../examples/electronics/i2c-two-buses.fg) · [Attribute rules](../../interface/forge-lang/src/standard_library/erc/attributes.rs)

## Writing and distributing a library

Use the first-party Rust API in `forge_lang::electronics_library`. A `Library` can be assembled with `with_children`, `with_properties`, and `with_checks`. A worker check declares a stable rule ID, default severity, input kind, and an ordinary Rust function. The function receives a batch of typed facts and returns diagnostics containing object IDs and a message. The published worker interface currently exposes fitted components with custom and typed fields, power-input/provider intervals, decoded addresses on resolved I²C buses, and routed tracks. Exact copper broad-phase queries and the full schematic graph are available to linked native passes. [Library API](../../interface/forge-lang/src/electronics_library.rs) · [Worker API](../../interface/forge-lang/src/electronics_library/worker.rs)

Build and install the included example on a desktop:

```sh
cargo build -p forge-lang --example house_library
cargo run -p forge-cli --bin forgecad -- electronics-library install target/debug/examples/house_library
```

Installation runs the supplied executable in manifest mode and stores the executable and manifest under the user's ForgeCAD configuration directory. Install only a trusted binary. A native worker has its user's OS permissions. Third-party installation is explicit and each name/version is immutable. Library authors distribute a binary per target. Designers using a prebuilt package need only ForgeCAD. Nest packages under `with_children` to publish them together. Multiple nested imports of the same leaf execute that leaf once. Conflicting rule IDs are errors.

```forge
house = library(file:"rust:house@1.0.0")
purchasing = library(file:"rust:house@1.0.0/review/assembly")
level = erc_rule(rule:"house.MissingMPN",severity:"error")
```

A property-only import reads the stored manifest. The host starts a persistent worker on its first executable check and exchanges versioned, size-limited messages over pipes. The application shows a check failure when a worker times out or exits. Loading the built-in standard library uses linked Rust on desktop and web. Native worker executables require a desktop host. [Runnable package with two levels of nested libraries](../../interface/forge-lang/examples/house_library.rs)

## Inspection and verification

Use `forgecad schematic FILE libraries --json` or `forgecad pcb FILE libraries --json` to see loaded libraries and their exports. The editor's rule severity menus use the same catalog. Existing project overrides, ERC waivers and findings keep their stable rule and object IDs. MCP reaches these same document queries.

The current worker protocol batches the requested fact family on each check. It does not promise a frame-time bound. Additional fact families should identify their invalidation keys and carry geometry witnesses from the host. Benchmark a single pin edit and a single trace move, including serialization and time to display the finding. The Lean model covers bus/address identity and selected unit-conversion laws. Arbitrary third-party Rust callbacks are outside that proof.

## ERC coverage in every library

The host owns one coverage report per fitted circuit evaluation. A skipped comparison has a rule ID, an occurrence or pin path, and the missing inputs. Coverage is separate from violation severity and waivers. The host deduplicates repeated reads, preserves independent checks on other occurrences, and exposes the same report to the inspector and JSON clients.

A Rust package can use `Library::with_requirements` to declare the component fields or pin specification kinds a worker check reads. The host checks those prerequisites **before** dispatch to either a linked or an installed worker. An optional symbol selector restricts a requirement. When a required value is missing, that occurrence is withheld and its missing fields are recorded.

For a branch-dependent check, use `electronics_library::evidence::require(rule, object, field, value)` after establishing applicability. It returns the original option and records a missing value. `evidence::missing` records a prerequisite computed across multiple facts. Installed workers send these records back to the same host ledger. Both paths are exercised by the [out-of-process worker test](../../interface/forge-lang/tests/electronics_worker.rs). Existing workers remain compatible. The host can report declared or observed prerequisites; an old Rust check that silently discards a raw `None` exposes no information about that decision.
