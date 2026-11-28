# Six-fan STM32 CAN FD controller

Rev B uses one synchronous buck to convert a nominal 48 V input to a common 12 V fan rail. Each of the six 4-pin outputs has its own hardware current limit, short-circuit protection, current reading and fault indication. The design target is 2 A per fan at the same time, or 144 W total. Open [the board](fan-controller.fg) in PCB Layout or [the schematic](controller.fg) in the schematic editor. The local symbol and footprint libraries travel with the project.

The [compact 6 A placement study](compact-study.md) keeps local pin circuits together while comparing fan-port arrangements, converter layouts and outlines. It includes a pad-level before-and-after of the oscillator and STM32. Open [its selected board](compact-six-amp.fg) in PCB Layout.

Two routing trials use that same 116 × 76 mm placement. The [fully connected board](compact-six-amp-pin-escape.fg) has two 0.40 mm-long, 0.20 mm-wide tracks at LM5069 pins VIN and OUT. DRC flags both against the input-power class's 0.80 mm minimum. The [width-compliant trial](compact-six-amp-routed.fg) leaves those two pins open. On both boards, the oscillator reaches the STM32 on a 2.06 mm top-layer trace with zero vias.

The [earlier 6 A placement study](six-amp-study.md) uses a cascaded 12 V, 5 V and 3.3 V power tree and compares six floorplans on the same outline.

## Power path

| Stage | Implementation |
|---|---|
| Input | J1 pin 1 is +48 V and pin 2 is the common return. F1 is an 8 A, 125 V DC fuse. D1 is an SMCJ54CA bidirectional TVS after the fuse. D2 is a series reverse-polarity Schottky. |
| Hot swap | U1, LM5069-1, drives the 100 V Q1 pass MOSFET. R1 sets a nominal 6.9 A input limit. R2–R5 set input UV/OV thresholds, R6 sets pass-FET power limiting and C2 sets a nominal 10.4 ms fault timeout. The PGD output releases both regulators after the pass FET enhances. An upstream fault can shut down the whole board. |
| Shared 12 V buck | U4, LM5146, drives two CSD19532KTT MOSFETs at 250 kHz. L1 is a Coilcraft XAL1510 8.2 µH inductor. R20 is a 3 mΩ low-side shunt. The output bank uses two 100 µF polymer capacitors and three 22 µF ceramics. At 48 V and 12 A the ideal inductor ripple is 4.39 A peak-to-peak and peak current is 14.2 A. |
| Fan protection | U11–U16 are TPS26630 eFuses. Each draws from V12 and feeds only its matching header. A 6.49 kΩ resistor programs about 2.77 A nominal current limit. The fast-trip threshold is typically twice that. The device limits overload current and latches off on thermal shutdown with MODE open. An output fault does not require an STM32 interrupt to limit current. |
| Logic | A separate LM5013 low-power buck supplies 3.3 V from the protected input for the STM32G0B1CBT6, its 8 MHz oscillator and the TCAN3414. All interfaces share ground. |

The 176 × 109 mm board has four copper layers with 2 oz outer copper and 1 oz inner copper. The 7 mm bottom-layer 12 V bus feeds all six branches. The layout uses filled, copper-capped vias in component lands. Review the via-in-pad process, stencil windowing and heavy-copper clearances with the assembler. [Main converter](https://www.ti.com/lit/ds/symlink/lm5146.pdf) · [Main inductor](https://www.coilcraft.com/en-us/products/power/shielded-inductors/molded-inductor/xal/xal1510/xal1510-822/) · [Hot-swap controller](https://www.ti.com/lit/ds/symlink/lm5069.pdf) · [Logic converter](https://www.ti.com/lit/ds/symlink/lm5013.pdf) · [Branch protection](https://www.ti.com/lit/ds/symlink/tps2663.pdf)

## Connections and measurements

| Connector | Pins in numerical order |
|---|---|
| J11–J16 | GND, protected +12 V, tach, PWM. J11 is fan 1. The PWM driver is an open-drain MOSFET. Each tach input has a 4.7 kΩ pull-up to 3.3 V. |
| J2 | CANH, CANL, common ground. Fit JP1 to connect the 120 Ω termination at a bus endpoint. D5 protects both CAN lines. |
| J3 | Target voltage reference, SWCLK, GND, SWDIO, NRST. Program the STM32 over SWD. |

Each TPS26630 IMON output sources nominally 27.9 µA/A. A 10 kΩ series resistor and a 20 kΩ resistor from the ADC tap to ground give 0.558 V/A at the MCU. A 2 A fan reads about 1.12 V. At the eFuse's 4 V monitor compliance limit the ADC tap is 2.67 V. The conversion assumes a 3.3 V ADC reference. The branch FLT lines are separate, active low and pulled up to 3.3 V. A fault bit can also indicate a supply-voltage event. [TPS26630 electrical characteristics and fault behavior](https://www.ti.com/lit/ds/symlink/tps2663.pdf)

TH1 senses the buck area, TH2 the branch distribution area and TH3 the board edge. Each uses a 10 kΩ NTC and a 10 kΩ pull-up. Firmware uses a B3435 approximation and reports a sentinel for out-of-range readings. These are local PCB temperatures. Rotor speed assumes two tach pulses per revolution. [Four-wire fan interface](https://cdn.noctua.at/media/Noctua_PWM_specifications_white_paper.pdf) · [STM32G0B1CB](https://www.st.com/en/microcontrollers-microprocessors/stm32g0b1cb.html) · [CAN transceiver](https://www.ti.com/lit/ds/symlink/tcan3414.pdf)

## CAN FD protocol

The firmware uses standard 11-bit IDs at 500 kbit/s nominal and 2 Mbit/s data. Fan PWM runs at 25 kHz. On startup it requests full speed, waits 500 ms, then enables one branch every 200 ms. After one second without a valid duty command it requests full speed on powered channels. A duty command leaves hardware fault latches alone. An operator can cycle a particular output after correcting its fault.

| ID | Length | Byte layout |
|---|---:|---|
| 0x510 duty | 8 | Version 1 at byte 0, host sequence at 1, six integer percentages 0–100 at 2–7. An invalid duty rejects the entire command. |
| 0x511 telemetry | 48 | Version 2, rolling sequence, full-speed fallback flag and missing-tach mask at 0–3. Applied duty at 4–9. Six RPM readings at 10–21. Last host sequence at 22 and active fault mask at 23. Six currents in mA at 24–35. TH1–TH3 in signed tenths of a degree Celsius at 36–41. Enabled and reset-in-progress masks at 42 and 43. Bytes 44–47 are reserved. |
| 0x512 branch reset | 3 | Version 1, key 0xA5, six-bit mask. The selected enabled outputs go low for 100 ms and are re-enabled. Bit 0 addresses J11. |

Telemetry runs every 500 ms. Multi-byte measurements are little-endian. Temperature −32768 means out of range. Two consecutive empty reporting windows on an enabled channel commanded to at least 20% assert its missing-tach bit. Current readings use the nominal monitor gain, and a saturated sample cannot measure the peak of a hard short.

With an already configured FD-capable SocketCAN interface, these frames set six 50% duties and separately request a reset of fan 3.

```sh
cansend can0 510##1012A323232323232
cansend can0 512##101A504
```

The standalone firmware is in [firmware/main.c](firmware/main.c). To build it separately, run `make -C firmware` with an Arm bare-metal GCC toolchain on PATH. It targets the STM32G0B1CB at flash address 0x08000000. ForgeCAD loads and exports the example using its native backend.

## Verification and first hardware

```sh
forgecad schematic controller.fg erc
forgecad pcb fan-controller.fg drc
forgecad pcb fan-controller.fg ratsnest
forgecad pcb fan-controller.fg pads --json > board-pads.json
forgecad export fan-controller.fg pcb-fab -o board-review.zip
python ../../../research/electronics/probes/fan_controller_readback.py board-review.zip board-pads.json
python ../../../research/electronics/probes/fan_controller_loop.py
```

The independent readback joins Gerber copper through plated Excellon bores, checks every terminal against the schematic-derived pad manifest, and verifies the main power path and six distinct protection, control and telemetry circuits by pin number. Gerbonara and Shapely are used only for this external verification. The averaged CCM loop estimate spans 190–300 µF effective output capacitance, 4–12 mΩ effective ESR, ±20% inductance and 3–12 A load. Under those assumptions it gives 10–22 kHz crossover and approximately 53–72° phase margin. [CAM probe](../../../research/electronics/probes/fan_controller_readback.py) · [Loop estimate](../../../research/electronics/probes/fan_controller_loop.py)

Hardware qualification remains open. Begin with a current-limited source and unloaded outputs. Check both rails, switch-node ringing, dead time and hot-swap startup before loading the bus. Measure loop gain and load-step response, then run six 2 A loads through thermal equilibrium at the intended enclosure temperature. With five channels loaded, short the sixth at the far end of the intended harness and scope branch current, the shared rail, the other five outputs and MCU reset. Repeat for hot plug, locked rotor and latch reset. Check Q1 against its pulsed safe operating area for the actual source and worst-case input capacitance, and verify the TVS energy, upstream fuse coordination, connector ratings and CAN FD behavior on the intended harness. These measurements establish the operating envelope for a built board.
