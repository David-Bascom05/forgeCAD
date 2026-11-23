#!/usr/bin/env python3
"""Stages a Thingi10K sample outside the repository: every STEPth file id (sorted), offset OFF,
from the Hugging Face dataset Thingi10K/Thingi10K (raw_meshes/<id>.<ext>). Writes a manifest
line per file: id, thing, licence, ext, bytes, sha256. Files are fetched, never committed.

  thingi10k.py META_DIR OUT_DIR MANIFEST.jsonl [--step 20] [--off 0] [--ext obj]
META_DIR holds metadata/input_summary.csv from the same dataset.
"""
import argparse, csv, hashlib, json, os, urllib.request

BASE = "https://huggingface.co/datasets/Thingi10K/Thingi10K/resolve/main/raw_meshes/"

ap = argparse.ArgumentParser()
ap.add_argument("meta"); ap.add_argument("out"); ap.add_argument("manifest")
ap.add_argument("--step", type=int, default=20); ap.add_argument("--off", type=int, default=0)
ap.add_argument("--ext", help="only files of this extension (e.g. obj: there are 42)")
a = ap.parse_args()
rows = sorted(csv.DictReader(open(os.path.join(a.meta, "input_summary.csv"))), key=lambda r: int(r["ID"]))
if a.ext:
    rows = [r for r in rows if r["Link"].rsplit(".", 1)[-1].lower() == a.ext]
pick = rows[a.off::a.step]
os.makedirs(a.out, exist_ok=True)
with open(a.manifest, "w") as m:
    for r in pick:
        ext = r["Link"].rsplit(".", 1)[-1].lower()
        name = f"{r['ID']}.{ext}"
        path = os.path.join(a.out, name)
        if not os.path.exists(path):
            try:
                urllib.request.urlretrieve(BASE + name, path)
            except Exception as e:
                m.write(json.dumps({"id": r["ID"], "err": str(e)[:100]}) + "\n")
                continue
        data = open(path, "rb").read()
        m.write(json.dumps({"id": r["ID"], "thing": r["Thing ID"], "licence": r["License"], "ext": ext,
                            "url": BASE + name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}) + "\n")
