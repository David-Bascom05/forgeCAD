# Six-channel CAN FD fan controller

Open `fan-controller.fg` in PCB Layout or `controller.fg` in the schematic editor. The power sheet links to the controller/CAN interface and the six fan channels. The board resolves every placement against the schematic. Keep the two libraries beside these files when moving the project.

## Power and connectors

The design assumes a regulated nominal 48 V DC source and six **12 V**, four-wire fans rated 2 A running current each. The fans alone draw 144 W at full load. At an assumed 90% conversion efficiency that corresponds to 3.33 A from 48 V, before logic consumption. Locked-rotor and simultaneous start current depend on the fans.

| Connection | Pin assignment / use |
|---|---|
| J1 | 1 = positive 48 V, 2 = primary return. Provide an externally fused, polarity-controlled feed. TDK specifies a normal-blow input fuse of **15 A maximum**. Select the actual DC voltage rating, interrupt rating and wiring for the source. |
| J3 through J8 | Fans 1 through 6 in order. 1 = secondary GND, 2 = fused 12 V, 3 = tach, 4 = PWM. Each fan has a 3 A branch fuse. The solder-side and component-side views are mirrored. Pin 1 has a silk corner mark. |
| J2 | 1 = CANH, 2 = CANL, 3 = secondary GND. The transceiver shares ground with the fans and the controller. |
| JP1 | Fit the shunt to connect 120 Ω across CANH and CANL when this node is at a physical bus end. Leave it open for an intermediate node. |

U1 is the six-pin, negative-logic **TDK-Lambda iQL48025A120V-0B9-R**. The module accepts 36–75 V and supplies nominal 12 V at up to 25 A under its specified thermal conditions. Pin 2 returns to primary negative to enable it. Pin 6 floats for nominal output. C1, C8 and C9 provide input capacitance. The output has 680 µF bulk plus 47 µF and 1 µF ceramic. Even at −20% tolerance C7 alone exceeds the specified 488 µF minimum output capacitance. The module input rating does not establish a board-level surge rating. [Model record](https://product.tdk.com/en/search/power/switching-power/dc-dc-converter/info?part_no=IQL48025A120V-0B9-R) · [Model-specific mechanical, capacitance, protection and derating specifications](https://product.tdk.com/system/files/dam/doc/product/power/switching-power/dc-dc-converter/specification/iql48025a120v_spec.pdf)

U2 generates 5 V for a Teensy 4.1. Its onboard regulator supplies the 3.3 V tach pull-ups and TCAN3414. If the controller will receive external VIN while connected to a USB host, separate VUSB from VIN at the Teensy solder link according to PJRC. Set the controller module close enough to the enclosure opening for the intended USB plug. [RECOM regulator](https://recom-power.com/pdf/Innoline/R-78E-0.5.pdf) · [PJRC pin assignment](https://www.pjrc.com/teensy/card11a_rev4_web.pdf) · [VIN / USB power link](https://www.pjrc.com/teensy/card11b_rev4_web.pdf) · [CAN transceiver](https://www.ti.com/lit/ds/symlink/tcan3414.pdf)

## Fan control

Each Q1–Q6 sinks only the control input on pin 4. The fan receives continuous fused 12 V on pin 2. A 100 kΩ gate pull-down releases PWM during reset and a 4.7 kΩ resistor pulls the fan's open-collector tach signal up to 3.3 V. The example assumes a fan with an internal PWM pull-up, an open-collector tach output and two tach pulses per revolution. Full-speed behavior with a released PWM input, minimum running duty, cable pin order and contact current rating should be confirmed for the selected fan and mating connector. [Four-wire fan interface and tach timing](https://cdn.noctua.at/media/Noctua_PWM_specifications_white_paper.pdf) · [Molex 47053-1000, 4 A maximum per contact](https://www.molex.com/en-us/products/part-detail/0470531000) · [BSS138BK logic-level MOSFET](https://assets.nexperia.com/documents/data-sheet/BSS138BK.pdf)

| Channel | Fan / fuse | Controller PWM | Controller tach |
|---|---|---:|---:|
| 1 | J3 / F1 | 2 | 14 |
| 2 | J4 / F2 | 3 | 15 |
| 3 | J5 / F3 | 4 | 16 |
| 4 | J6 / F4 | 5 | 17 |
| 5 | J7 / F5 | 6 | 18 |
| 6 | J8 / F6 | 9 | 19 |

The four-layer, 158 × 106 mm carrier assumes 2 oz outside and 1 oz internal copper. The 10 mm top supply trunk feeds six individually fused branches. An 8 mm bottom return and the inner ground plane carry current back to the converter. Adjacent ground pour areas share boundaries so a refill can handle the two header rows in smaller sections. The primary return remains separate. The second inner layer carries signal crossovers. The local process record specifies a 0.25 mm minimum track, 0.20 mm clearance and 0.35 mm minimum drill. A fabrication quote must support this heavy-copper stack.

## CAN FD and firmware

`six_fan_node/six_fan_node.ino` is the Teensy 4.1 sketch. It runs the six PWM outputs at 25 kHz and counts falling tach edges independently. CAN3 uses pin 31 for TX and pin 30 for RX. The external target uses the Teensy board package and FlexCAN_T4. Opening, checking and exporting the ForgeCAD documents require only ForgeCAD. [FlexCAN_T4 interface](https://github.com/tonton81/FlexCAN_T4)

The example assigns standard 11-bit IDs, a nominal bitrate of 500 kbit/s and a 2 Mbit/s data phase. Reserve its IDs before joining an existing bus. All six commands apply together. Invalid frames leave the existing command and its deadline intact.

| ID | Frame | Payload, bytes numbered from zero |
|---|---|---|
| `0x510` | Command, 8-byte FD frame | 0 = version 1, 1 = host sequence, 2–7 = duties for fans 1–6 as integers 0–100. Refresh faster than once per second. |
| `0x511` | Telemetry, 24-byte FD frame with bitrate switching, every 500 ms | 0 = version 1, 1 = rolling report sequence, 2 bit 0 = full-speed fallback, 3 bits 0–5 = missing tach for fans 1–6, 4–9 = applied duty percentages, 10–21 = six little-endian unsigned 16-bit RPM values, 22 = last accepted host sequence, 23 = reserved zero. |

An example SocketCAN command requests 50% duty on all six outputs with sequence 42.

```sh
cansend can0 510##1012A323232323232
```

Startup and one second without a valid command both request 100% on every channel. At the default report period, one counted pulse corresponds to 60 RPM. A channel at 20% duty or higher raises its missing-tach bit after two empty reporting windows. The bit can indicate a stalled fan, a disconnected plug or a broken tach conductor. This revision measures RPM and reports applied duty. Current and temperature sensing would require additional hardware.

## Verification and bring-up

From this directory, the following commands use the native backend.

```sh
forgecad schematic controller.fg erc
forgecad schematic controller.fg bom-csv
forgecad pcb fan-controller.fg drc
forgecad pcb fan-controller.fg ratsnest
forgecad export fan-controller.fg pcb-fab -o fan-controller-fab.zip
```

The independent verification probe reads Gerber and Excellon, joins physical layers through plated bores, and checks each fan against its fuse, MOSFET, tach pull-up and assigned MCU header pins. It also checks primary/secondary separation, the trim pad, unused CPU pins, the termination jumper and both CAN wires. Run `python fan_controller_readback.py /path/to/fan-controller-fab.zip` in the repository's CAM verification environment. [Readback probe](../../../research/electronics/probes/fan_controller_readback.py)

Bench qualification remains open. Check header keying and purchased mechanical outlines, the permitted fuse and harness ratings, startup with all six motors, steady-state voltage drop and temperature rise on both power paths, and thermal derating with a failed fan. Exercise the CAN node across the intended cable and termination with a second FD-capable node. The included firmware was cross-linked for Teensy 4.1 and its command, tach, telemetry and timeout behaviors were exercised with simulated I/O.
