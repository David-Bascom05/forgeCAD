#!/usr/bin/env python3
"""Makes the minimal inputs for the bugs in real-files.md, each from nothing (no dataset file).

  python3 -I make.py OUT_DIR
"""
import io, os, struct, sys, zipfile

CT = b'<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>'
RELS = b'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>'


def tetra_model(extra_model_attrs=b"", extra=b""):
    return (b'<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" '
            + extra_model_attrs + b'><resources><object id="1" type="model"><mesh><vertices>'
            b'<vertex x="0" y="0" z="0"/><vertex x="10" y="0" z="0"/><vertex x="0" y="10" z="0"/><vertex x="0" y="0" z="10"/>'
            b'</vertices><triangles><triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="1" v3="3"/><triangle v1="1" v2="2" v3="3"/><triangle v1="0" v2="3" v3="2"/>'
            b'</triangles></mesh></object>' + extra + b'</resources><build><item objectid="1"/></build></model>')


def package(model, force_zip64=False):
    b = io.BytesIO()
    with zipfile.ZipFile(b, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in [("[Content_Types].xml", CT), ("_rels/.rels", RELS), ("3D/3dmodel.model", model)]:
            with z.open(name, "w", force_zip64=force_zip64) as f:
                f.write(data)
    d = bytearray(b.getvalue())
    if force_zip64:
        # what large-file writers do: the classic end record's counts and offset all 0xFF..,
        # the real ones in the ZIP64 end record before it
        e = d.rfind(b"PK\x05\x06")
        struct.pack_into("<HHHHII", d, e + 4, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFFFFFF, 0xFFFFFFFF)
        if d.rfind(b"PK\x06\x06") < 0:
            raise SystemExit("python wrote no ZIP64 end record")
    return bytes(d)


def zip64_eocd(d):
    """Adds a ZIP64 end record and locator to a plain zip and marks the classic record."""
    d = bytearray(d)
    e = d.rfind(b"PK\x05\x06")
    n, size, off = struct.unpack_from("<HII", d, e + 10)
    z64 = struct.pack("<IQHHIIQQQQ", 0x06064B50, 44, 45, 45, 0, 0, n, n, size, off)
    loc = struct.pack("<IIQI", 0x07064B50, 0, e, 1)
    tail = bytearray(d[e:])
    struct.pack_into("<HHHHII", tail, 4, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFFFFFF, 0xFFFFFFFF)
    return bytes(d[:e] + z64 + loc + tail)


out = sys.argv[1]
os.makedirs(out, exist_ok=True)
plain = package(tetra_model())
open(os.path.join(out, "tetra.3mf"), "wb").write(plain)
open(os.path.join(out, "tetra-zip64.3mf"), "wb").write(zip64_eocd(plain))
# a required extension no reader here knows: a consumer must refuse it (3MF core 1.4, 2.3.1;
# test case N_XXX_0428)
open(os.path.join(out, "tetra-required-unknown.3mf"), "wb").write(package(tetra_model(
    b'xmlns:q="http://example.com/forge/unknown/2026" requiredextensions="q"')))
# a build item in another model part (production extension's p:path): refused, but as "no object 1"
PROD = b'http://schemas.microsoft.com/3dmanufacturing/production/2015/06'
root = (b'<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" '
        b'xmlns:p="' + PROD + b'" requiredextensions="p"><resources/><build><item objectid="1" p:path="/3D/other.model"/></build></model>')
b = io.BytesIO()
with zipfile.ZipFile(b, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("[Content_Types].xml", CT)
    z.writestr("_rels/.rels", RELS)
    z.writestr("3D/3dmodel.model", root)
    z.writestr("3D/_rels/3dmodel.model.rels", b'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/other.model" Id="rel1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>')
    z.writestr("3D/other.model", tetra_model())
open(os.path.join(out, "build-item-path.3mf"), "wb").write(b.getvalue())
# OBJ: a line continued with a backslash (the format allows it; trimesh and assimp read it)
open(os.path.join(out, "continuation.obj"), "w").write("v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 3 2\nf 1 2 4\nf 2 3 \\\n4\nf 1 4 3\n")
