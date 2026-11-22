#!/usr/bin/env python3
"""The pipe set with each part's solids as label_pipe reads them: fc_twin.py DIR SEED0 COUNT.

Writes DIR/pNNNNN.fg (as research/pipe-speed/fuzz/gen.py writes it) and DIR/pNNNNN.fc beside it.
The .fc twin is one line per solid ("NAME: extrude [add] D z .. x .. at .. circle|rect ..") with the
numbers the .fg was given, then "fillet R". The pipe set is gen.py seeds 0-199.
"""
import os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "../../pipe-speed/fuzz"))
import gen

lines = []
_extrude = gen.extrude


def extrude(P, name, op, d, z, x, at, loop):
    if op == "add":
        g = lambda v: " ".join(repr(gen.g(c)) for c in v)
        lines.append(f"{name}: extrude add {d!r} z {g(z)} x {g(x)} at {g(at)} {loop[0]} " + " ".join(repr(v) for v in loop[1:]))
    _extrude(P, name, op, d, z, x, at, loop)


gen.extrude = extrude

if __name__ == "__main__":
    d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    os.makedirs(d, exist_ok=True)
    for s in range(s0, s0 + n):
        lines.clear()
        text = gen.part(s)
        r = re.search(r"^round = fillet\(.*, ([-\d.e+]+)\)$", text, re.M).group(1)
        open(f"{d}/p{s:05d}.fg", "w").write(text)
        open(f"{d}/p{s:05d}.fc", "w").write(f"# seed {s}\n" + "\n".join(lines[:2]) + f"\nfillet {r}\n")
