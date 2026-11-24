import os, sys, stat
# usage: git ls-files -co --exclude-standard -z | python3 srcsync.py DEST
# DEST becomes the checkout's files: a file whose bytes and mode already match is left alone (its
# time too), anything else is written anew (time now), and what the checkout lacks is removed.
# So a directory's time moves only when an entry is added or removed, as cargo's checks expect.
dest = sys.argv[1]
want = [p for p in sys.stdin.buffer.read().decode().split("\0") if p]
keep = set()
for p in want:
    d = os.path.join(dest, p)
    if os.path.islink(p):
        keep.add(p)
        t = os.readlink(p)
        if not (os.path.islink(d) and os.readlink(d) == t):
            os.makedirs(os.path.dirname(d) or dest, exist_ok=True)
            if os.path.lexists(d):
                os.unlink(d)
            os.symlink(t, d)
        continue
    if not os.path.isfile(p):
        continue
    keep.add(p)
    data = open(p, "rb").read()
    mode = stat.S_IMODE(os.stat(p).st_mode)
    try:
        if stat.S_IMODE(os.stat(d).st_mode) == mode and open(d, "rb").read() == data:
            continue
    except OSError:
        pass
    os.makedirs(os.path.dirname(d) or dest, exist_ok=True)
    tmp = d + ".gate-sync"
    with open(tmp, "wb") as f:
        f.write(data)
    os.chmod(tmp, mode)
    os.replace(tmp, d)
for top, dirs, files in os.walk(dest, topdown=False):
    for f in files:
        r = os.path.relpath(os.path.join(top, f), dest)
        if r not in keep:
            os.unlink(os.path.join(top, f))
    if top != dest and not os.listdir(top):
        os.rmdir(top)
