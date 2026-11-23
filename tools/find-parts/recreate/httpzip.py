"""A zip archive read over HTTP by byte range: its central directory once (cached as an index), then single members.
The archive is never downloaded whole. Members are returned as bytes and never run. Zip64 is read.

  z = HttpZip(URL, cache=INDEX.json)
  z.names()           every member's path
  z.read(path)        one member's bytes (stored or deflated)
"""
import json
import os
import struct
import time
import urllib.error
import urllib.request
import zlib


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a):
        return None


_where = {}  # a URL -> (where it redirects, when that was asked): a hub's redirect is asked once in a while, not per read


def located(url, fresh=False):
    w = _where.get(url)
    if w and not fresh and time.time() - w[1] < 900:
        return w[0]
    op = urllib.request.build_opener(urllib.request.ProxyHandler(), NoRedirect)
    req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": "forgecad-research"})
    try:
        op.open(req, timeout=60)
        loc = url
    except urllib.error.HTTPError as x:
        loc = x.headers.get("Location") if x.code in (301, 302, 303, 307, 308) else None
        if not loc:
            raise
    _where[url] = (loc, time.time())
    return loc


def fetch(url, lo, hi, tries=6):
    """Bytes lo..hi inclusive, retried with backoff."""
    for k in range(tries):
        try:
            req = urllib.request.Request(located(url, fresh=k > 0), headers={"Range": f"bytes={lo}-{hi}", "User-Agent": "forgecad-research"})
            with urllib.request.urlopen(req, timeout=120) as r:
                data = r.read()
            if r.status == 206 and len(data) == hi - lo + 1:
                return data
            raise IOError(f"range {lo}-{hi}: status {r.status}, {len(data)} bytes")
        except Exception:
            if k == tries - 1:
                raise
            time.sleep(2 ** k)


def size_of(url):
    req = urllib.request.Request(located(url), headers={"Range": "bytes=0-0", "User-Agent": "forgecad-research"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return int(r.headers["Content-Range"].split("/")[1])


class HttpZip:
    def __init__(self, url, cache=None):
        self.url = url
        if cache and os.path.exists(cache):
            self.index = json.load(open(cache))
        else:
            self.index = self._directory()
            if cache:
                tmp = cache + ".tmp"
                json.dump(self.index, open(tmp, "w"))
                os.replace(tmp, cache)

    def _directory(self):
        n = size_of(self.url)
        tail = fetch(self.url, max(0, n - 65536 - 22), n - 1)
        e = tail.rfind(b"PK\x05\x06")
        if e < 0:
            raise IOError("no end of central directory")
        cd_size, cd_off = struct.unpack("<II", tail[e + 12 : e + 20])
        count = struct.unpack("<H", tail[e + 10 : e + 12])[0]
        if cd_off == 0xFFFFFFFF or cd_size == 0xFFFFFFFF or count == 0xFFFF:
            l = tail.rfind(b"PK\x06\x07")
            z64 = struct.unpack("<Q", tail[l + 8 : l + 16])[0]
            rec = fetch(self.url, z64, z64 + 55)
            count, cd_size, cd_off = struct.unpack("<QQQ", rec[32:56])
        cd = b""
        step = 64 << 20
        for lo in range(cd_off, cd_off + cd_size, step):
            cd += fetch(self.url, lo, min(lo + step, cd_off + cd_size) - 1)
        idx, p = {}, 0
        while p < len(cd) and cd[p : p + 4] == b"PK\x01\x02":
            meth, = struct.unpack("<H", cd[p + 10 : p + 12])
            csize, usize = struct.unpack("<II", cd[p + 20 : p + 28])
            nl, xl, cl = struct.unpack("<HHH", cd[p + 28 : p + 34])
            off, = struct.unpack("<I", cd[p + 42 : p + 46])
            name = cd[p + 46 : p + 46 + nl].decode("utf-8", "replace")
            extra = cd[p + 46 + nl : p + 46 + nl + xl]
            q = 0
            while q + 4 <= len(extra):
                hid, hl = struct.unpack("<HH", extra[q : q + 4])
                if hid == 1:
                    vals, r = [], q + 4
                    for v in (usize, csize, off):
                        if v == 0xFFFFFFFF:
                            vals.append(struct.unpack("<Q", extra[r : r + 8])[0])
                            r += 8
                        else:
                            vals.append(v)
                    usize, csize, off = vals
                q += 4 + hl
            if not name.endswith("/"):
                idx[name] = [off, csize, usize, meth]
            p += 46 + nl + xl + cl
        return idx

    def names(self):
        return list(self.index)

    def read(self, name):
        off, csize, usize, meth = self.index[name]
        head = fetch(self.url, off, off + 29)
        nl, xl = struct.unpack("<HH", head[26:30])
        lo = off + 30 + nl + xl
        data = fetch(self.url, lo, lo + csize - 1) if csize else b""
        if meth == 0:
            return data
        if meth == 8:
            return zlib.decompress(data, -15)
        raise IOError(f"compression {meth}")
