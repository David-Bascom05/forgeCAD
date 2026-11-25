"""An embedded-schema text file rewritten as one in its own schema, carried by nothing: each
type's schema record taken out and the schema name cut to SCH_<version>_<schema>, as Onshape's
translation service writes files. For a fixture of that path through the reader.

    python3 research/parasolid/native.py IN.x_t OUT.x_t
"""
import sys
sys.path.insert(0, __import__("os").path.dirname(__file__))
import xtproto


def main(src, dst):
    data = open(src, "rb").read()
    xtproto.parse(data)
    ver, sch, maxtypes, usfld = xtproto.HEAD
    assert sch.count("_") == 3, "not an embedded-schema file"
    i = data.index(b"**END_OF_HEADER")
    i = data.index(b"\n", i) + 1
    head = data[:i].decode("latin1")
    body = xtproto.BODY
    # the record parts: cut out each first node's schema record
    cuts = sorted(xtproto.DESCPOS.values())
    out, at = [], 0
    for a, b in cuts:
        out.append(body[at:a])
        at = b
    out.append(body[at:])
    nodes = b"".join(out)
    new_sch = "_".join(sch.split("_")[:3])
    old_lead = f"{len(sch)} {sch}{maxtypes} {usfld} ".encode()
    assert old_lead in nodes[:300], nodes[:200]
    nodes = nodes.replace(old_lead, f"{len(new_sch)} {new_sch}{usfld} ".encode(), 1)
    head = head.replace(f"SCH={sch[: sch.rindex('_')]}", f"SCH={new_sch}")
    lines = [nodes[k : k + 80] for k in range(0, len(nodes), 80)]
    # a record may not end in a space: move it on
    res, carry = [], b""
    rest = nodes
    while rest:
        n = min(80, len(rest))
        while n > 1 and rest[n - 1 : n] == b" " and n < len(rest):
            n -= 1
        res.append(rest[:n])
        rest = rest[n:]
    open(dst, "wb").write(head.encode("latin1") + b"\n".join(res) + b"\n")


if __name__ == "__main__":
    main(*sys.argv[1:3])
