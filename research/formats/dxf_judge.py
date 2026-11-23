#!/usr/bin/env python3
"""The DXF and DWG flat-pattern writers (forge-draw, forge-dwg) on the repository's own sheet
parts, by a rule fixed before any result was looked at (research/formats/real-files.md, "DXF").
Forge reads no DXF, so DXF corpora have nothing to judge.

  $VENV/bin/python -I dxf_judge.py FORGECAD DWGREAD OUT.jsonl FILE.fg.. [@LIST]

Per sheet: `export F dxf ID` read by ezdxf (readfile, then its auditor: no error), and `export F
dwg ID` read by dwgread. Match: both read, ezdxf's audit finds no error, the model space's entity
types and counts in the DXF (ezdxf) equal the DWG's (dwgread's entity names, model space,
less its BLOCK/ENDBLK/SEQEND structure: counted at first, which a DXF keeps in BLOCKS).
The DXF's extents (ezdxf.bbox) are kept for the record.
"""
import collections, json, os, subprocess, sys, tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def run(cmd, timeout=300):
    return subprocess.run(cmd, capture_output=True, timeout=timeout)


def sheets(cli, f):
    # -o always: without it, export writes F.dxf beside the part
    with tempfile.TemporaryDirectory() as t:
        p = run([cli, "export", f, "dxf", "--json", "-o", os.path.join(t, "s.dxf")])
    try:
        d = json.loads(p.stdout.decode().strip().splitlines()[-1])
    except Exception:
        return []
    if d.get("ok"):
        return [d.get("sheet")]
    e = str(d.get("error", ""))
    if "several sheets" in e:
        return e.split("one of ", 1)[1].split()
    return []


def main(argv):
    cli, dwgread, out = argv[:3]
    fs = []
    for a in argv[3:]:
        fs += [l.strip() for l in open(a[1:]) if l.strip()] if a.startswith("@") else [a]
    import ezdxf
    from ezdxf import bbox

    with open(out, "w") as o:
        for f in fs:
            for s in sheets(cli, f):
                r = {"file": f, "sheet": s}
                with tempfile.TemporaryDirectory() as t:
                    dxf, dwg, js = (os.path.join(t, x) for x in ("a.dxf", "a.dwg", "a.json"))
                    run([cli, "export", f, "dxf", s, "-o", dxf])
                    run([cli, "export", f, "dwg", s, "-o", dwg])
                    try:
                        doc = ezdxf.readfile(dxf)
                        aud = doc.audit()
                        msp = doc.modelspace()
                        r["dxf"] = dict(collections.Counter(e.dxftype() for e in msp))
                        ext = bbox.extents(msp)
                        r["dxf_box"] = [ext.extmin.x, ext.extmin.y, ext.extmax.x, ext.extmax.y] if ext.has_data else None
                        r["audit"] = [str(e.message)[:100] for e in aud.errors]
                    except Exception as e:
                        r["verdict"], r["why"] = "ezdxf-refused", [f"{type(e).__name__}: {str(e)[:150]}"]
                    p = run([dwgread, "-O", "json", "-o", js, dwg])
                    try:
                        d = json.load(open(js, errors="replace"))
                        # model space's own BLOCK/ENDBLK (and SEQENDs) are its structure, kept in a DXF's BLOCKS
                        ms = [x for x in d["OBJECTS"] if "entity" in x and x.get("entmode") == 2 and x["entity"] not in ("BLOCK", "ENDBLK", "SEQEND")]
                        r["dwg"] = dict(collections.Counter(x["entity"] for x in ms))
                    except Exception as e:
                        r.setdefault("verdict", "dwgread-refused")
                        r.setdefault("why", [f"dwgread exit {p.returncode}"])
                if "verdict" not in r:
                    why = []
                    if r["audit"]:
                        why.append(f"audit: {r['audit'][:3]}")
                    if r["dxf"] != r["dwg"]:
                        why.append(f"entities {r['dxf']} vs {r['dwg']}")
                    r["verdict"], r["why"] = ("match" if not why else "mismatch"), why
                o.write(json.dumps(r) + "\n")
                o.flush()


if __name__ == "__main__":
    main(sys.argv[1:])
