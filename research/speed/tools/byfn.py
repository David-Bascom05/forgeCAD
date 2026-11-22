import re, subprocess, sys, collections
# exclusive cost by function (all files folded), functions matching a regex: byfn.py CG REGEX [N]
out = subprocess.run(['callgrind_annotate', '--threshold=100', '--auto=no', sys.argv[1]], capture_output=True, text=True).stdout
c = collections.Counter()
for line in out.splitlines():
    m = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+(\S*?):(.*?)( \[/.*\])?$', line)
    if m:
        k = re.sub(r"'\d+", "", m.group(3))
        c[k] += int(m.group(1).replace(',', ''))
pat = sys.argv[2]
d = sorted(((v, k) for k, v in c.items() if re.search(pat, k)), reverse=True)
print(sum(v for v, k in d))
for v, k in d[:int(sys.argv[3]) if len(sys.argv) > 3 else 40]:
    print(f'{v:10d} {k[:150]}')
