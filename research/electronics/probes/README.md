# Reproducing the electronics measurements

These portable probes accompany the [live-check design](../live-checks.md) and the [planar geometry study](../backend.md#4-planar-copper-geometry-and-the-forgecad-reuse-decision). The original run used Rust 1.99.1 and the ForgeCAD sources at `d9d93a0a`. The two TSV files contain front-copper via centres, outer diameters (integer nanometres), and assigned net codes. The region probe uses Koyomi via centres in millimetres. All four programs run outside the production build.

Run from the repository root. The native DRC probe compares full sets of offending pairs under a fixed 0.20 mm rule after every move.

```sh
p=research/electronics/probes
build=$(mktemp -d)
rustc --edition=2024 -O "$p/via_frame.rs" -o "$build/via-frame"
"$build/via-frame" "$p/koyomi-lvds-hat.tsv" "$p/inkplate-2.tsv"
```

The language probe times in-process calls on the hat-section part. The region probe times a single difference operation and reports the output loop count. Use the same Cargo profile for comparisons.

```sh
cargo build --locked --target-dir "$build" -p forge-lang -p forge-cam-geom
rustc --edition=2024 "$p/fg_latency.rs" -L "dependency=$build/debug/deps" --extern "forge_lang=$build/debug/libforge_lang.rlib" -o "$build/fg-latency"
"$build/fg-latency" interface/forge-lang/parts/sheet/hat.fg 200
rustc --edition=2024 "$p/region.rs" -L "dependency=$build/debug/deps" --extern "forge_cam_geom=$build/debug/libforge_cam_geom.rlib" -o "$build/region"
for n in 8 32 112; do "$build/region" "$n"; done
```

To repeat the independent extraction, obtain the two pinned boards and save them as `koyomi-lvds-hat.kicad_pcb` and `inkplate-2.kicad_pcb`. With KiCad 9.0.2 Python bindings, the command below also checks grid-query results against an exhaustive bounding-box scan for each front via. Compare the resulting TSVs with the committed inputs. [Koyomi source](https://github.com/TensorFleet/koyomi-lvds-hat/blob/bd6af086f8528749ccd98f745891a731011d5fac/koyomi-lvds-hat.kicad_pcb) · [Inkplate source](https://github.com/SolderedElectronics/Soldered-Inkplate-2-hardware-design/blob/57c5179a669f5237cbca6a0c0cbc7aa936c7b1df/CAD/V1.2.3/Inkplate%202.kicad_pcb)

```sh
VIA_FIXTURES="$build" /usr/bin/python3 "$p/board_locality.py" koyomi-lvds-hat.kicad_pcb inkplate-2.kicad_pcb
```

## Native schematic response time

The probe builds a sheet of MCU and capacitor pairs, starts the native MCP server, and queries a supply pin. Each pair has a confirmed bypass relationship and separate supply and return wires. It checks the pin's response after every sample. The cold sample includes process startup. Subsequent timings measure the request and reply over stdio. In the move case the probe changes a capacitor's coordinates on disk before starting the timer. File reading, dependency validation, solving, and response serialization are included. GUI painting and physical-board DRC are outside this measurement.

```sh
cargo build --release --locked -p forge-cli
python3 research/electronics/probes/schematic_latency.py target/release/forgecad --channels 256 --samples 40
```
