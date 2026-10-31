#!/usr/bin/env python3
"""Where whole-part regens go, by function, inclusive: perf call graphs of the bench parts in a
frame-pointer build (see README). usage: centres.py BIN FCDIR [PARTS...]"""
import collections, glob, os, re, subprocess, sys

bin_, fcdir = sys.argv[1:3]
names = sys.argv[3:] or [os.path.basename(f)[:-3] for f in sorted(glob.glob(fcdir + "/*.fc"))]
os.environ["FORGE_BATCH"] = "cpu"
PERIOD = 500000
data = f".scratch/centres-{os.getpid()}.data"
SKIP = re.compile(r"^(\[unknown|\{closure|fold|map|collect|from_iter|branch|par_map|lang_start|__libc|_start|catch_unwind|main|call_init|do_call|__rust_begin|std::|call_once|try_fold|next|unwrap|map_err|and_then|ok_or|for_each|sum|<F|<&|<core|<alloc|<std|new<|into_iter|extend|spec_|core::|alloc::|hint|inner|deref|as_|try_|find|position|any|all|filter|min_by|max_by|flatten|cloned|index|get|clone|abs |dot|mul |add |sub |div |neg |sqrt |min |max |len<|ptr<|non_null|write<|read<|with<|is_nan|initialize<|call |block_box|ceil|floor|eq |lt |le |hull |to_bits|shr |overflowing|forward|push|pop|is_|sequential|regenerate|apply|combine|add_tools)")
total, incl = 0, collections.Counter()
per = {}
for n in names:
    subprocess.run(["perf", "record", "-q", "-e", "instructions:u", "-c", str(PERIOD), "--call-graph", "fp", "-o", data, bin_, "run", f"{fcdir}/{n}.fc", "100000", "seq"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    out = subprocess.run(["perf", "script", "-i", data, "-F", "ip,sym"], capture_output=True, text=True, errors="replace").stdout
    os.remove(data)
    c = collections.Counter()
    for s in out.split("\n\n"):
        syms = {re.sub(r"::h[0-9a-f]{16}$", "", re.sub(r"\+0x[0-9a-f]+$", "", l.strip().split(None, 1)[1]).strip()) for l in s.splitlines() if len(l.strip().split(None, 1)) == 2}
        if syms:
            total += 1
            incl.update(syms)
            c.update(syms)
    per[n] = c
print(f"# total {total * PERIOD / 1e6:.0f}M in the frame-pointer build; percentages are what carry over")
for x, c in incl.most_common(400):
    if SKIP.match(x):
        continue
    top = max(per, key=lambda n: per[n][x])
    print(f"{100 * c / total:6.1f}%  {x[:70]:70}  most in {top} {100 * per[top][x] / c:.0f}%")
