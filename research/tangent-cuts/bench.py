import argparse
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description='Compare instruction counts on resolved jack prefixes')
p.add_argument('baseline', type=Path)
p.add_argument('candidate', type=Path)
p.add_argument('resolved_part', type=Path)
p.add_argument('scratch', type=Path)
a = p.parse_args()
a.scratch.mkdir(parents=True, exist_ok=True)
lines = a.resolved_part.read_text().splitlines()
env = dict(os.environ, FORGE_BATCH='cpu')
for n in [10, 12, 13, 14, 15, 16, 17, 18]:
    part = a.scratch / f'jack-prefix-{n}.fc'
    part.write_text('\n'.join(lines[:n]) + '\n')
    counts = []
    for exe in [a.baseline, a.candidate]:
        r = subprocess.run(
            ['fbuild', 'perf', 'stat', '-x,', '-e', 'instructions:u',
             str(exe.resolve()), 'run', str(part.resolve()), 'seq'],
            env=env, capture_output=True, text=True, check=True,
        )
        counts.append(next(int(s.split(',')[0]) for s in r.stderr.splitlines()
                           if 'instructions:u' in s))
    print(n, *counts, 'delta', counts[1] - counts[0])
