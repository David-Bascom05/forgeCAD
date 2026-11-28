# Experimental fan-controller router

This stand-alone experiment targets the [116 × 76 mm six-fan controller](../../../examples/electronics/fan-controller/compact-six-amp.fg). The routing grid, pad manifest, net priorities, rail geometry and fine-pitch escapes assume that placement. The Rust programs build separately with rustc and the standard library. Python drives the experiment and invokes the ForgeCAD CLI for connectivity.

## Run

From this directory, with rustc (edition 2024), Python 3 and forgecad on PATH, choose a new output directory. Set RUSTC or FORGECAD to an executable path to select a particular build.

```sh
python3 run.py escape /tmp/forge-pin-escape
python3 run.py route /tmp/forge-routing-trial
```

The escape replay starts from the checked-in width-compliant board and checks the generated pin-escape board byte for byte. The full routing trial starts with the fixed placement and pads.tsv. It lays out the 12 V trunk, searches three signal-routing layers, retries congested connections, stitches ground, and tries top-layer clock breakouts. The ground plane is restored from the placement file for the final boards. A new search can yield a different route. Inspect the resulting boards with ForgeCAD DRC.

## Recorded result

The [width-compliant trial](../../../examples/electronics/fan-controller/compact-six-amp-routed.fg) leaves two LM5069 pins open. The [pin-escape trial](../../../examples/electronics/fan-controller/compact-six-amp-pin-escape.fg) connects all 624 pads on 149 nets and has two explicit minimum-width violations. Each 0.40 mm long escape is 0.20 mm wide in a net class with a 0.80 mm minimum.

[Verification results for the recorded boards](verification.json) include the native DRC findings and independent readback of Gerber copper and plated drills. To repeat that readback, install Gerbonara and Shapely 2.x in a verification environment and run the following commands from this directory.

```sh
board=../../../examples/electronics/fan-controller/compact-six-amp-pin-escape.fg
forgecad pcb "$board" pads --json > /tmp/fan-pads.json
forgecad export "$board" pcb-fab -o /tmp/fan-review.zip
python3 verify_routing.py /tmp/fan-review.zip /tmp/fan-pads.json /tmp/fan-readback.json
```

[Four copper layers](connected-layers.png) · [Clock and LM5069 pin escapes](escape-details.png) · [Main buck switching path](buck.png)
