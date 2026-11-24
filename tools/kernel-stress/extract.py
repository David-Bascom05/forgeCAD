"""One solid out of a STEP file, as a STEP file of its own: the solid's entity and everything it
refers to, as the file wrote them, under one shape representation that holds only it (with the
file's units and uncertainty). For shrinking a failure the sweep found to the body it is on.

    python3 -I tools/kernel-stress/extract.py IN.step ENTITY OUT.step [--faces N,M,..]

ENTITY is the solid's `#` number (the sweep's `ent`). `--faces` keeps only those faces of its
shell (by their `#` numbers): an open shell, read as a sheet, for a failure on a few faces.
"""

import re
import sys


def records(text):
    """`#id` -> the record's text after `=`, strings respected."""
    out = {}
    i = text.find("DATA;")
    i = 0 if i < 0 else i + 5
    n = len(text)
    while i < n:
        j = text.find("#", i)
        if j < 0:
            break
        k = text.find("=", j)
        if k < 0:
            break
        ident = text[j + 1:k].strip()
        if not ident.isdigit():
            i = j + 1
            continue
        # to the `;` outside any string
        p = k + 1
        q = False
        while p < n:
            c = text[p]
            if c == "'":
                q = not q
            elif c == ";" and not q:
                break
            p += 1
        out[int(ident)] = text[k + 1:p].strip()
        i = p + 1
    return out


REF = re.compile(r"#(\d+)")


def strip_strings(s):
    return re.sub(r"'(?:[^']|'')*'", "''", s)


def closure(recs, roots):
    seen = set()
    todo = list(roots)
    while todo:
        r = todo.pop()
        if r in seen or r not in recs:
            continue
        seen.add(r)
        for m in REF.finditer(strip_strings(recs[r])):
            todo.append(int(m.group(1)))
    return seen


def main():
    args = sys.argv[1:]
    src, ent, dst = args[0], int(args[1]), args[2]
    faces = None
    if "--faces" in args:
        faces = [int(x) for x in args[args.index("--faces") + 1].split(",")]
    text = open(src, encoding="latin-1").read()
    recs = records(text)
    head = text[: text.find("DATA;")] + "DATA;\n"
    head = re.sub(r"FILE_NAME\s*\((.*?)\)\s*;", "FILE_NAME('extract','',(''),(''),'','','');", head, flags=re.S)
    solid = recs[ent]
    if faces is not None:
        # an open shell of the faces kept, standing for the solid
        shell = int(REF.findall(strip_strings(solid))[0])
        body = f"OPEN_SHELL('',({','.join('#' + str(f) for f in faces)}))"
        recs[shell] = body
        recs[ent] = f"SHELL_BASED_SURFACE_MODEL('',(#{shell}))"
        kind = "MANIFOLD_SURFACE_SHAPE_REPRESENTATION"
    else:
        kind = "ADVANCED_BREP_SHAPE_REPRESENTATION"
    # the representation holding the solid: its context, for units and uncertainty
    ctx = None
    for i, r in recs.items():
        if "SHAPE_REPRESENTATION" in r.split("(")[0] and re.search(rf"#{ent}\b", strip_strings(r)):
            refs = REF.findall(strip_strings(r))
            ctx = int(refs[-1])
            break
    if ctx is None:
        # any representation's context
        for i, r in recs.items():
            if r.startswith("ADVANCED_BREP_SHAPE_REPRESENTATION"):
                ctx = int(REF.findall(strip_strings(r))[-1])
                break
    keep = closure(recs, [ent, ctx])
    rep = max(recs) + 1
    out = [head]
    for i in sorted(keep):
        out.append(f"#{i}={recs[i]};\n")
    out.append(f"#{rep}={kind}('',(#{ent}),#{ctx});\n")
    out.append("ENDSEC;\nEND-ISO-10303-21;\n")
    open(dst, "w", encoding="latin-1").write("".join(out))
    print(f"{dst}: {len(keep)} entities, {sum(len(x) for x in out)} bytes")


if __name__ == "__main__":
    main()
