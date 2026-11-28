"""Run the board-specific routing experiment in a separate output directory."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

SOURCE = Path(__file__).resolve().parent
BOARD = SOURCE.parents[2] / "examples/electronics/fan-controller"
PLACEMENT = "compact-six-amp.fg"
STRICT = "compact-six-amp-routed.fg"
CONNECTED = "compact-six-amp-pin-escape.fg"


def run(*command):
    subprocess.run(command, check=True)


def python(script, *args):
    run(sys.executable, SOURCE / script, *args)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["route", "escape"])
    parser.add_argument("output", type=Path, help="new working directory")
    parser.add_argument("--passes", type=int, default=32)
    args = parser.parse_args()
    if args.passes < 1:
        parser.error("--passes must be positive")
    args.output.mkdir(parents=True, exist_ok=False)
    os.chdir(args.output)
    for name in [PLACEMENT, "six-amp-schematic.fg", "footprints.fg", "symbols.fg"]:
        shutil.copy(BOARD / name, name)
    for name in ["check", "pads.tsv", "fast.py", "merge.py"]:
        shutil.copy(SOURCE / name, name)

    placement = Path(PLACEMENT).read_text()
    no_plane, marker, _ = placement.partition("ground = zone")
    if not marker:
        raise ValueError("Ground plane missing from the placement fixture")
    Path("no-plane.fg").write_text(no_plane)
    rustc = os.environ.get("RUSTC", "rustc")
    run(rustc, "--edition=2024", "-O", SOURCE / "repair.rs", "-o", "repair2")

    if args.mode == "route":
        run(rustc, "--edition=2024", "-O", SOURCE / "route.rs", "-o", "route")
        run("./route", "pads.tsv", "initial.copper", "V12", SOURCE / "fanout-seed.txt", str(args.passes))
        python("hunt.py", "initial.copper")
        best = "best.copper" if Path("best.copper").exists() else "initial.copper"

        # Smaller adjoining zones speed up ground-connectivity queries during repair.
        panels = [no_plane]
        for i in range(8):
            for j in range(5):
                x, y = 2 + 14*i, 2 + 14.4*j
                corners = f"({x},{y:.3f}),({x+14},{y:.3f}),({x+14},{y+14.4:.3f}),({x},{y+14.4:.3f})"
                panels.append(f"ground_{i}_{j} = zone(net:GND, layer:gnd, clearance:0.36mm, points:[{corners}])\n")
        Path("panel.fg").write_text("".join(panels))
        python("prep_repair.py", best)
        run("./repair2")
        python("merge.py", best, "ground.copper")
        python("fast.py", "ground.copper")
        run("./repair2", "--thermal")
        python("merge.py", "ground.copper", "final.copper")
        python("tryclock.py")
        clock = Path("c0.55.copper")
        copper = clock.read_text() if clock.exists() else Path("final.copper").read_text()
        Path(STRICT).write_text(placement + copper)
    else:
        strict = (BOARD / STRICT).read_text()
        if not strict.startswith(placement):
            raise ValueError("Strict trial and placement fixture differ")
        copper = strict[len(placement):]

    Path("strict.copper").write_text(copper)
    python("fast.py", "strict.copper")
    run("./repair2", "--escape")
    python("merge.py", "strict.copper", "breakouts.copper")
    python("fast.py", "breakouts.copper")
    run("./repair2", "RECT")
    python("merge.py", "breakouts.copper", "connected.copper")
    Path(CONNECTED).write_text(placement + Path("connected.copper").read_text())
    if args.mode == "escape" and Path(CONNECTED).read_bytes() != (BOARD / CONNECTED).read_bytes():
        raise RuntimeError("Replayed pin escapes differ from the checked-in trial")


if __name__ == "__main__":
    main()
