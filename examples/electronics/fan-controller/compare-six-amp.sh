#!/bin/sh
# Replay the placement study from the selected, openable board. The input,
# fan and CAN connectors and all four mounting holes remain fixed.
set -eu
cd "$(dirname "$0")"
mcu() {
    for ref in U2 Y1 R9 R10 C4 C5 C6 C7 C8 C9; do printf '@%s=%s' "$ref" "$1"; done
}
power() {
    for group in front/block_main TH1 R11 C11; do printf '@%s=%s' "$group" "$1"; done
}
exec "${FORGECAD:-forgecad}" pcb six-amp.fg floorplan \
    "south$(power 30,-3)$(mcu 15,-27)" \
    "power-west$(mcu 15,-27)" \
    "mcu-north$(power 30,-3)$(mcu 27,-3)" \
    "east-power$(power 54,-3)$(mcu 15,-27)" \
    "crowded$(power 30,-3)$(mcu -21,4)" --json
