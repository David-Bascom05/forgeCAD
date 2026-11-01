#!/usr/bin/env python3
"""Mutated STEP files through the reader: no panic, no hang, no runaway memory. Seeds come from a
dataset zip; mutants exist only in memory and go to a child on stdin, capped at 3 GB and a timeout.

  fuzz.py BIN --zip Z --ids IDS --name 'FMT' --n 200 --seed 1 [--timeout 20] [--flags "--no-mass"]

Prints a count per outcome and, for each failure, the seed id and the mutation's seed, which
reproduce it. Run with python3 -I. Nothing is written to the repo."""
import argparse, collections, concurrent.futures as cf, os, random, re, resource, subprocess, threading, zipfile

MEM = 3 << 30
TOKENS = [b"#", b"(", b")", b",", b";", b"'", b"$", b"*", b"=", b".T.", b".F.", b"1E308", b"-1E308", b"1E-308", b"0.", b"-1.", b"NaN", b"99999999999999999999", b"#1", b"#0", b"#4294967295", b"#99999999", b"()", b"((((", b"\x00", b"\xff", b"/*"]
NUM = re.compile(rb"-?\d+\.\d*(?:[eE][-+]?\d+)?")
REF = re.compile(rb"#\d+")


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    os.nice(15)


def mutate(b, r):
    b = bytearray(b)
    for _ in range(r.choice([1, 1, 2, 4, 16, 64])):
        k = r.randrange(8)
        n = len(b)
        if n < 8:
            break
        at = r.randrange(n)
        if k == 0:
            b[at] = r.randrange(256)
        elif k == 1:
            b[at:at] = r.choice(TOKENS)
        elif k == 2:
            del b[at : at + r.randrange(1, 64)]
        elif k == 3:
            b = b[: r.randrange(n)]
        elif k == 4:
            # a number made extreme, zero or negative
            m = NUM.search(b, at)
            if m:
                b[m.start() : m.end()] = r.choice([b"0.", b"-1.", b"1.E308", b"1.E-308", b"-1.E308", b"1000000.", b"1.E20"])
        elif k == 5:
            # a reference pointing elsewhere: cycles, wrong kinds, itself
            m = REF.search(b, at)
            o = REF.search(b, r.randrange(n))
            if m and o:
                b[m.start() : m.end()] = o.group()
        elif k == 6:
            # a line duplicated many times
            e = b.find(b";", at)
            if 0 < e - at < 400:
                b[e:e] = b[at:e] * r.randrange(2, 200)
        else:
            # a keyword replaced by another's
            ws = re.findall(rb"=\s*([A-Z_0-9]{4,})\(", bytes(b[at : at + 2000]))
            m = re.search(rb"=\s*[A-Z_0-9]{4,}\(", b[at:])
            if ws and m:
                s = at + m.start()
                b[s : at + m.end()] = b"=" + r.choice(ws) + b"("
    return bytes(b)


def main():
    a = argparse.ArgumentParser()
    a.add_argument("bin")
    a.add_argument("--zip", required=True)
    a.add_argument("--ids", required=True)
    a.add_argument("--name", default="{id}")
    a.add_argument("--stem", action="store_true")
    a.add_argument("--n", type=int, default=200)
    a.add_argument("--seed", type=int, default=1)
    a.add_argument("--jobs", type=int, default=4)
    a.add_argument("--timeout", type=int, default=20)
    a.add_argument("--flags", default="")
    a.add_argument("--max-size", type=int, default=2 << 20)
    o = a.parse_args()
    z = zipfile.ZipFile(o.zip)
    names = {}
    if o.stem:
        names = {os.path.splitext(os.path.basename(n))[0]: n for n in z.namelist()}
    ids = [l.strip() for l in open(o.ids) if l.strip()]
    lock = threading.Lock()
    flags = o.flags.split()

    def job(i):
        ms = o.seed * 1000003 + i
        r = random.Random(ms)
        seed = r.choice(ids)
        with lock:
            try:
                data = z.read(names[seed] if names else o.name.format(id=seed))
            except KeyError:
                return ("skip", seed, ms, "")
        if len(data) > o.max_size:
            return ("skip", seed, ms, "")
        m = mutate(data, r)
        try:
            p = subprocess.run([o.bin] + flags, input=m, capture_output=True, timeout=o.timeout, preexec_fn=limit)
        except subprocess.TimeoutExpired:
            return ("timeout", seed, ms, "")
        if p.returncode == 0:
            return ("ok", seed, ms, "")
        err = p.stderr.decode("utf8", "replace")
        at = [l for l in err.splitlines() if "panicked" in l or "memory allocation" in l or "overflow" in l]
        return ("crash", seed, ms, (at[0] if at else f"exit {p.returncode}")[:160])

    c = collections.Counter()
    bad = []
    with cf.ThreadPoolExecutor(o.jobs) as ex:
        for k, seed, ms, why in ex.map(job, range(o.n)):
            c[k] += 1
            if k in ("crash", "timeout"):
                bad.append((k, seed, ms, why))
    print(dict(c))
    w = collections.Counter(re.sub(r"\(\d+\)", "", b[3]) for b in bad)
    for k, n in w.most_common(16):
        print(n, k)
    for b in bad[:12]:
        print(b)


if __name__ == "__main__":
    main()
