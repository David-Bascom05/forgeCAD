"""Writes kernel/forge-step/src/xt/known.rs: the schemas a transmit file may name without
carrying them (Onshape's translation service writes such files), each node type's record taken
from files that do carry that schema.

    cargo build --release -p forge-step --example xtschemas
    target/release/examples/xtschemas FILES... > records.tsv
    python3 research/parasolid/known_schemas.py records.tsv...

Merged with what known.rs already holds, so runs over different corpora add up. Where two
files disagree on a type's record in one schema, the script stops and says so.
"""
import re
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "kernel/forge-step/src/xt/known.rs"


def existing():
    have = {}
    if not OUT.exists():
        return have
    src = OUT.read_text()
    for m in re.finditer(r"\((\d+), &\[(.*?)\]\),\n", src, re.S):
        sid = int(m.group(1))
        for t in re.finditer(r'\((\d+), b"((?:[^"\\]|\\.)*)"\)', m.group(2)):
            raw = t.group(2).encode().decode("unicode_escape").encode("latin-1")
            have.setdefault(sid, {})[int(t.group(1))] = raw
    return have


def rust_bytes(b):
    out = []
    for c in b:
        ch = chr(c)
        if ch in '"\\':
            out.append("\\" + ch)
        elif 32 <= c < 127:
            out.append(ch)
        else:
            out.append(f"\\x{c:02x}")
    return "".join(out)


def main(files):
    have = existing()
    for f in files:
        for line in open(f):
            schema, ty, hx = line.rstrip("\n").split("\t")
            parts = schema.split("_")
            if len(parts) < 4 or parts[2] == "13006":
                continue
            sid, ty, rec = int(parts[2]), int(ty), bytes.fromhex(hx)
            old = have.setdefault(sid, {}).get(ty)
            if old is not None and old != rec:
                sys.exit(f"schema {sid} type {ty}: files disagree:\n  {old!r}\n  {rec!r}")
            have[sid][ty] = rec
    body = []
    for sid in sorted(have):
        recs = ", ".join(f'({ty}, b"{rust_bytes(r)}")' for ty, r in sorted(have[sid].items()))
        body.append(f"    ({sid}, &[{recs}]),\n")
    src = OUT.read_text()
    head, rest = src.split("pub(super) static SCHEMAS", 1)
    tail = rest[rest.index("];") + 2 :]
    OUT.write_text(
        head
        + "pub(super) static SCHEMAS: &[(u32, Records)] = &[\n"
        + "".join(body)
        + "];"
        + tail
    )
    print(f"{len(have)} schemas, {sum(len(v) for v in have.values())} records")


if __name__ == "__main__":
    main(sys.argv[1:])
