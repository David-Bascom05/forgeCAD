import argparse
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description="Compare instruction counts on prefixes of the jack's kernel features")
p.add_argument('baseline', type=Path, help='featcost built on the baseline (research/feature-costs)')
p.add_argument('candidate', type=Path, help='featcost built on the candidate')
p.add_argument('part', type=Path, help='the .fg part (interface/forge-lang/parts/bench/jack.fg)')
p.add_argument('--prefixes', default='', help='kernel feature counts to regenerate, comma separated (default: all)')
a = p.parse_args()
env = dict(os.environ, FORGE_BATCH='cpu')
listed = subprocess.run([str(a.baseline.resolve()), 'list', str(a.part.resolve())], env=env, capture_output=True, text=True, check=True)
n_all = len(listed.stdout.splitlines())
prefixes = [int(x) for x in a.prefixes.split(',') if x] or list(range(1, n_all + 1))
for n in prefixes:
    counts = []
    for exe in [a.baseline, a.candidate]:
        # featcost builds the whole program, then regenerates its first n kernel features: the build
        # is the same on both sides, so the difference is the regen's
        r = subprocess.run(
            ['fbuild', 'perf', 'stat', '-x,', '-e', 'instructions:u',
             str(exe.resolve()), 'run', str(a.part.resolve()), str(n), 'seq'],
            env=env, capture_output=True, text=True, check=True,
        )
        counts.append(next(int(s.split(',')[0]) for s in r.stderr.splitlines()
                           if 'instructions:u' in s))
    print(n, *counts, 'delta', counts[1] - counts[0])
