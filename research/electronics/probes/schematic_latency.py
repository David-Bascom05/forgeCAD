"""Time a selected-pin MCP query across unchanged and edited native schematics."""
import argparse
import json
import math
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def fixture(channels):
    rows = ['schematic', 'parts = library(file: "power-symbols.fg")', 'main = sheet()']
    for i in range(channels):
        x = 20 * (i % 32)
        y = 15 * (i // 32)
        rows.extend([
            f'u{i} = place(sheet: main, symbol: parts.MCU, at: ({x},{y}))',
            f'c{i} = place(sheet: main, symbol: parts.Cap, at: ({x + 8},{y}))',
            f's{i} = wire(sheet: main, from: u{i}.vdd, to: c{i}.plus)',
            f'r{i} = wire(sheet: main, from: u{i}.vss, to: c{i}.minus)',
            f'b{i} = fulfill(sheet: main, want: u{i}.vdd, give: c{i}.plus, role: parts.local_bypass,'
            f' return_from: c{i}.minus, return_to: u{i}.vss)',
        ])
    return '\n'.join(rows) + '\n'


def inspect(process, path, request_id):
    req = {"jsonrpc": "2.0", "id": request_id, "method": "tools/call", "params": {
        "name": "schematic", "arguments": {"file": str(path), "sel": "pin main/u0.vdd"}}}
    line = json.dumps(req) + '\n'
    start = time.perf_counter_ns()
    process.stdin.write(line)
    process.stdin.flush()
    reply = process.stdout.readline()
    elapsed = (time.perf_counter_ns() - start) / 1e6
    msg = json.loads(reply)["result"]
    assert not msg.get("isError"), msg
    data = json.loads(msg["content"][0]["text"])["result"]
    assert data["fulfillments"][0]["id"] == "main/b0", data
    return elapsed


def run(binary, channels, samples):
    library = Path(__file__).resolve().parents[3] / 'examples/electronics/power-symbols.fg'
    with tempfile.TemporaryDirectory() as td:
        directory = Path(td)
        (directory / library.name).write_bytes(library.read_bytes())
        path = directory / 'timing.fg'
        source = fixture(channels)
        path.write_text(source)
        with subprocess.Popen([binary, "mcp", "--root", td], stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, text=True) as process:
            try:
                cold = inspect(process, path, 1)
                warm = [inspect(process, path, i + 2) for i in range(samples)]
                changed = []
                for i in range(samples):
                    # Move the first capacitor without changing its connectivity.
                    path.write_text(source.replace("at: (8,0)", f"at: ({9 + i},0)", 1))
                    changed.append(inspect(process, path, samples + i + 2))
            finally:
                process.stdin.close()
    def summary(values):
        return {"p50_ms": round(statistics.median(values), 3),
                "p95_ms": round(sorted(values)[math.ceil(.95 * len(values)) - 1], 3)}
    return {"channels": channels, "pins": 4 * channels, "samples_per_case": samples,
            "cold_ms": round(cold, 3), "unchanged": summary(warm), "after_move": summary(changed)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('forgecad', help='path to the built CLI')
    parser.add_argument('--channels', type=int, default=256)
    parser.add_argument('--samples', type=int, default=30)
    args = parser.parse_args()
    if min(args.channels, args.samples) < 1:
        parser.error('channels and samples must be positive')
    print(json.dumps(run(args.forgecad, args.channels, args.samples), indent=2))
