"""rangezip.py URL OUT.zip PREFIXES.txt [--suffix .txt]: the members of a remote zip whose base name
starts with one of PREFIXES (an 8-character id per line), fetched by HTTP range requests and written
into a local zip, so a sample of a multi-GB archive (CADFS's step.zip, featurescript_fp.zip,
sketch_raw.zip) can be staged without the whole file. Reads the central directory (zip64 too),
then each member's local header and data. The members are data and are never executed."""
import struct, sys, urllib.request, zipfile, zlib, os

url, out, pref = sys.argv[1:4]
suffix = sys.argv[sys.argv.index("--suffix") + 1] if "--suffix" in sys.argv else ""
want = {l.strip()[:8] for l in open(pref) if l.strip()}


def get(a, b):
    req = urllib.request.Request(url, headers={"Range": f"bytes={a}-{b}", "User-Agent": "rangezip"})
    for k in range(5):
        try:
            with urllib.request.urlopen(req, timeout=120) as r:
                return r.read()
        except Exception as e:
            if k == 4:
                raise
            print("retry", e, file=sys.stderr)


req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": "rangezip"})
size = int(urllib.request.urlopen(req, timeout=60).headers["Content-Length"])
tail = get(size - 66000, size - 1)
e = tail.rfind(b"PK\x05\x06")
cd_n, cd_size, cd_off = struct.unpack("<HII", tail[e + 10 : e + 20])
z64 = tail.rfind(b"PK\x06\x07")
if z64 >= 0:
    (eocd64,) = struct.unpack("<Q", tail[z64 + 8 : z64 + 16])
    rec = get(eocd64, eocd64 + 55)
    cd_n, cd_size, cd_off = struct.unpack("<QQQ", rec[32:56])
cd = get(cd_off, cd_off + cd_size - 1)
members, i = [], 0
while i < len(cd) and cd[i : i + 4] == b"PK\x01\x02":
    meth, = struct.unpack("<H", cd[i + 10 : i + 12])
    csize, usize = struct.unpack("<II", cd[i + 20 : i + 28])
    nl, xl, cl = struct.unpack("<HHH", cd[i + 28 : i + 34])
    (lho,) = struct.unpack("<I", cd[i + 42 : i + 46])
    name = cd[i + 46 : i + 46 + nl].decode("utf-8", "replace")
    extra = cd[i + 46 + nl : i + 46 + nl + xl]
    j = 0
    while j + 4 <= len(extra):
        hid, hl = struct.unpack("<HH", extra[j : j + 4])
        if hid == 1:
            vals = extra[j + 4 : j + 4 + hl]
            k = 0
            if usize == 0xFFFFFFFF:
                (usize,) = struct.unpack("<Q", vals[k : k + 8]); k += 8
            if csize == 0xFFFFFFFF:
                (csize,) = struct.unpack("<Q", vals[k : k + 8]); k += 8
            if lho == 0xFFFFFFFF:
                (lho,) = struct.unpack("<Q", vals[k : k + 8]); k += 8
        j += 4 + hl
    base = os.path.basename(name)
    if base[:8] in want and name.endswith(suffix):
        members.append((name, meth, csize, lho))
    i += 46 + nl + xl + cl
print(len(members), "members of", cd_n, file=sys.stderr)
with zipfile.ZipFile(out, "a", zipfile.ZIP_DEFLATED) as zo:
    have = set(zo.namelist())
    for name, meth, csize, lho in members:
        if name in have:
            continue
        h = get(lho, lho + 29)
        nl, xl = struct.unpack("<HH", h[26:30])
        data = get(lho + 30 + nl + xl, lho + 30 + nl + xl + csize - 1) if csize else b""
        raw = zlib.decompress(data, -15) if meth == 8 else data
        zo.writestr(name, raw)
