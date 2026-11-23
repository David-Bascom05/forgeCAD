"""An independent check of ForgeCAD's mesh files: each read by trimesh (and an STL by numpy-stl too),
each body closed, manifold, consistently wound and facing out; a 3MF's package parts checked and
its model validated against the 3MF Core schema. One JSON line per file; exit 1 if any fails.

    $VENV/bin/python tools/meshcheck/check.py FILE.stl|FILE.3mf|FILE.obj ..

Test only (tools/meshcheck/setup.sh makes the venv); nothing in the workspace depends on it.
"""

import io
import json
import os
import sys
import zipfile

import numpy as np
import trimesh
from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
CORE = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02"


def bodies(mesh):
    """Each closed piece of a mesh, or each geometry of a scene, as placed."""
    if isinstance(mesh, trimesh.Scene):
        return list(mesh.dump())
    return [mesh]


def judge(meshes):
    out = []
    for m in meshes:
        # vertices at the same point made one, whatever their normals (trimesh splits an OBJ's
        # by v//vn pair): an STL's welded as a slicer would
        m.merge_vertices(merge_tex=True, merge_norm=True)
        out.append(
            {
                "faces": int(len(m.faces)),
                "watertight": bool(m.is_watertight),
                "winding": bool(m.is_winding_consistent),
                "volume": float(m.volume),
                "euler": int(m.euler_number),
            }
        )
    return out


def check_3mf(path, rec):
    z = zipfile.ZipFile(path)
    names = z.namelist()
    rec["parts"] = names
    for i in z.infolist():
        assert i.compress_type in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED), i
    assert z.testzip() is None, "a member's CRC is wrong"
    ct = etree.fromstring(z.read("[Content_Types].xml"))
    exts = {d.get("Extension"): d.get("ContentType") for d in ct}
    assert exts.get("model") == "application/vnd.ms-package.3dmanufacturing-3dmodel+xml", exts
    assert exts.get("rels") == "application/vnd.openxmlformats-package.relationships+xml", exts
    rels = etree.fromstring(z.read("_rels/.rels"))
    start = [
        r.get("Target")
        for r in rels
        if r.get("Type") == "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"
    ]
    assert len(start) == 1, start
    model = z.read(start[0].lstrip("/"))
    # libxml2 takes no maxOccurs past 2^31 - 2: the schema's "fewer than 2^31" read as unbounded
    with open(os.path.join(HERE, "3mf-core.xsd"), "rb") as x:
        xsd = x.read().replace(b'maxOccurs="2147483647"', b'maxOccurs="unbounded"')
    # and xml:lang from the xml namespace's schema beside it
    imp = b'<xs:import namespace="http://www.w3.org/XML/1998/namespace" schemaLocation="xml.xsd"/>'
    xsd = xsd.replace(b'blockDefault="#all">', b'blockDefault="#all">' + imp, 1)
    schema = etree.XMLSchema(etree.fromstring(xsd, base_url=os.path.join(HERE, "3mf-core.xsd")))
    doc = etree.parse(io.BytesIO(model))
    rec["schema"] = bool(schema.validate(doc))
    if not rec["schema"]:
        rec["schema_errors"] = [str(e) for e in list(schema.error_log)[:5]]
    root = doc.getroot()
    rec["unit"] = root.get("unit")
    ns = {"m": CORE}
    rec["objects"] = [o.get("name") for o in root.findall("m:resources/m:object", ns)]
    rec["items"] = len(root.findall("m:build/m:item", ns))
    rec["materials"] = [
        (b.get("name"), b.get("displaycolor"))
        for b in root.findall("m:resources/m:basematerials/m:base", ns)
    ]
    # each object's mesh as written: every edge in exactly two triangles, in opposite directions
    for o in root.findall("m:resources/m:object", ns):
        tris = np.array(
            [[int(t.get(k)) for k in ("v1", "v2", "v3")] for t in o.iterfind(".//m:triangle", ns)]
        )
        e = np.concatenate([tris[:, [0, 1]], tris[:, [1, 2]], tris[:, [2, 0]]])
        directed = {tuple(x) for x in e.tolist()}
        assert len(directed) == len(e), f"{o.get('name')}: a directed edge used twice"
        missing = sum(1 for (a, b) in directed if (b, a) not in directed)
        assert missing == 0, f"{o.get('name')}: {missing} edges without their opposite"


def main(files):
    bad = False
    for f in files:
        rec = {"file": os.path.basename(f)}
        try:
            ext = f.rsplit(".", 1)[-1].lower()
            if ext == "3mf":
                check_3mf(f, rec)
            # an OBJ's objects apart, as its `o` lines make them
            extra = {"split_objects": True, "group_material": False} if ext == "obj" else {}
            loaded = trimesh.load(f, process=False, **extra)
            rec["bodies"] = judge(bodies(loaded))
            if ext == "stl":
                from stl import mesh as npstl

                s = npstl.Mesh.from_file(f)
                # numpy-stl's own normals from the corners, against those written
                written = s.normals / np.maximum(np.linalg.norm(s.normals, axis=1)[:, None], 1e-30)
                s.update_normals()
                computed = s.normals / np.maximum(np.linalg.norm(s.normals, axis=1)[:, None], 1e-30)
                good = np.linalg.norm(computed, axis=1) > 0.5
                rec["normals_off"] = int(
                    np.sum(np.linalg.norm(written[good] - computed[good], axis=1) > 1e-3)
                )
                rec["npstl_volume"] = float(s.get_mass_properties()[0])
            ok = all(b["watertight"] and b["winding"] and b["volume"] > 0 for b in rec["bodies"])
            ok = ok and rec.get("schema", True) and rec.get("normals_off", 0) == 0
            rec["ok"] = bool(ok)
        except Exception as e:  # noqa: BLE001 - every failure is the file's
            rec["ok"] = False
            rec["error"] = repr(e)
        bad |= not rec["ok"]
        print(json.dumps(rec))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main(sys.argv[1:])
