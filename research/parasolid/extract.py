"""Cuts one body out of a Parasolid text transmit file: the body as the root, every node it
reaches, the file's own schema words for each type. For small test files and reproducers from
the corpus.

    python3 research/parasolid/extract.py IN.x_t BODY_INDEX OUT.x_t
    python3 research/parasolid/extract.py IN.x_t list      (each body: index, faces, surface kinds)
"""
import collections
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import xtproto  # noqa: E402

LEAVE = {10, 11, 100}  # assembly, instance, transform: the cut stands where its part does


def num(x):
    if x is None:
        return "?"
    if isinstance(x, float):
        r = repr(x)
        if r.endswith(".0"):
            r = r[:-2]
        return r + " "
    return f"{x} "


def emit(typ, k, v, cnt):
    """One field's text: numbers spaced, characters and logicals not, nulls as '?'."""
    if k == 1 and typ == "c":
        return v.decode("latin1").replace("\\", "\\\\")
    vals = v if k else [v]
    out = []
    for x in vals:
        if typ in "cl":
            out.append(x.decode("latin1") if isinstance(x, bytes) else x)
        elif typ in "vh":
            out.append("?" if x is None else "".join(num(float(c)) for c in x))
        elif typ in "ib":
            out.append("?" if x is None else "".join(num(float(c)) for c in x))
        elif typ == "f":
            out.append(num(None if x is None else float(x)))
        else:
            out.append(num(x))
    return "".join(out)


def reach(nodes, schema, body):
    """Every node the body reaches by its pointers, but other parts and the assembly."""
    keep = set()
    stack = [body]
    while stack:
        i = stack.pop()
        if i in keep or i not in nodes:
            continue
        ty, r = nodes[i]
        if ty in LEAVE or (ty == 12 and i != body):
            continue
        keep.add(i)
        for name, typ, k in schema[ty]:
            if typ != "p":
                continue
            vals = r[name] if k else [r[name]]
            for x in vals:
                if isinstance(x, int) and x > 0:
                    stack.append(x)
    return keep


def main():
    src, which = sys.argv[1], sys.argv[2]
    data = open(src, "rb").read()
    hdr, ver, sch, schema, nodes = xtproto.parse(data)
    if which == "list":
        for i, (ty, r) in nodes.items():
            if ty == 12:
                keep = reach(nodes, schema, i)
                kinds = collections.Counter(nodes[j][0] for j in keep)
                print(i, len(keep), dict(sorted(kinds.items())))
        return
    body = int(which)
    assert nodes[body][0] == 12, "not a body"
    keep = reach(nodes, schema, body)
    order = [body] + sorted(keep - {body})
    renum = {i: i for i in keep}
    renum[body] = 1
    if 1 in keep and body != 1:
        renum[1] = body
    ver_s, sch_s, maxtypes, usfld = xtproto.HEAD
    out = ["T", f"{len(ver_s)} {ver_s.decode('latin1')}", f"{len(sch_s)} {sch_s}"]
    if maxtypes is not None:
        out.append(f"{maxtypes} ")
    out.append(f"{usfld} ")
    seen = set()
    for i in order:
        ty, r = nodes[i]
        fields = schema[ty]
        s = f"{ty} "
        if ty not in seen:
            s += xtproto.DESC[ty].decode("latin1")
            seen.add(ty)
        var = [f for f in fields if f[2] == 1]
        if var:
            s += f"{len(r[var[0][0]])} "
        s += f"{renum[i]} "
        for name, typ, k in fields:
            v = r[name]
            if typ == "p":
                fix = lambda x: (renum.get(x, 0) if isinstance(x, int) and x > 0 else x)
                v = [fix(x) for x in v] if k else fix(v)
            s += emit(typ, k, v, None)
        out.append(s)
    out.append("1 0 ")
    text = "".join(out)
    recs = []
    while text:
        n = 80
        while n > 1 and text[n - 1 : n] == " ":
            n -= 1
        recs.append(text[:n])
        text = text[n:]
    head = hdr if hdr.endswith("\n") else hdr + "\n"
    Path(sys.argv[3]).write_text(head + "\n".join(recs) + "\n", encoding="latin1")
    print(sys.argv[3], len(order), "nodes")


if __name__ == "__main__":
    main()
