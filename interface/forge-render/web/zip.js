// A project in and out of the browser as a .zip: written stored (no compression), read stored or
// deflated (the browser's DecompressionStream inflates).

const table = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(b) {
  let c = 0xffffffff;
  for (let i = 0; i < b.length; i++) c = table[(c ^ b[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/// [{path, bytes}] as the bytes of a zip.
export function zip(files) {
  const enc = new TextEncoder();
  const parts = [];
  const central = [];
  let at = 0;
  for (const f of files) {
    const name = enc.encode(f.path);
    const crc = crc32(f.bytes);
    const local = new DataView(new ArrayBuffer(30));
    local.setUint32(0, 0x04034b50, true);
    local.setUint16(4, 20, true);
    local.setUint16(6, 0x0800, true); // UTF-8 names
    local.setUint32(14, crc, true);
    local.setUint32(18, f.bytes.length, true);
    local.setUint32(22, f.bytes.length, true);
    local.setUint16(26, name.length, true);
    parts.push(new Uint8Array(local.buffer), name, f.bytes);
    const c = new DataView(new ArrayBuffer(46));
    c.setUint32(0, 0x02014b50, true);
    c.setUint16(4, 20, true);
    c.setUint16(6, 20, true);
    c.setUint16(8, 0x0800, true);
    c.setUint32(16, crc, true);
    c.setUint32(20, f.bytes.length, true);
    c.setUint32(24, f.bytes.length, true);
    c.setUint16(28, name.length, true);
    c.setUint32(42, at, true);
    central.push(new Uint8Array(c.buffer), name);
    at += 30 + name.length + f.bytes.length;
  }
  const size = central.reduce((n, p) => n + p.length, 0);
  const end = new DataView(new ArrayBuffer(22));
  end.setUint32(0, 0x06054b50, true);
  end.setUint16(8, files.length, true);
  end.setUint16(10, files.length, true);
  end.setUint32(12, size, true);
  end.setUint32(16, at, true);
  return new Blob([...parts, ...central, new Uint8Array(end.buffer)], { type: "application/zip" });
}

async function inflate(b) {
  const s = new Blob([b]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return new Uint8Array(await new Response(s).arrayBuffer());
}

/// A zip's files: [{path, bytes}] (directories left out).
export async function unzip(bytes) {
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let e = bytes.length - 22;
  while (e >= 0 && v.getUint32(e, true) !== 0x06054b50) e--;
  if (e < 0) throw new Error("not a zip file");
  const count = v.getUint16(e + 10, true);
  let c = v.getUint32(e + 16, true);
  const dec = new TextDecoder();
  const out = [];
  for (let i = 0; i < count; i++) {
    if (v.getUint32(c, true) !== 0x02014b50) throw new Error("a broken zip file");
    const method = v.getUint16(c + 10, true);
    const csize = v.getUint32(c + 20, true);
    const nlen = v.getUint16(c + 28, true);
    const xlen = v.getUint16(c + 30, true);
    const klen = v.getUint16(c + 32, true);
    const off = v.getUint32(c + 42, true);
    const path = dec.decode(bytes.subarray(c + 46, c + 46 + nlen));
    c += 46 + nlen + xlen + klen;
    if (path.endsWith("/")) continue;
    const start = off + 30 + v.getUint16(off + 26, true) + v.getUint16(off + 28, true);
    const data = bytes.subarray(start, start + csize);
    if (method === 0) out.push({ path, bytes: data.slice() });
    else if (method === 8) out.push({ path, bytes: await inflate(data) });
    else throw new Error(`${path}: compressed in a way this can't read (${method})`);
  }
  return out;
}
