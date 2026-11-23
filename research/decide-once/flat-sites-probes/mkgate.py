import sys
V = '/nix/store/bi3mnvps3yr4ir0jbd5vikqc4vra76xv-valgrind-3.27.1/bin/valgrind'
src, dst = sys.argv[1], sys.argv[2]
s = open(src).read()
a1 = '          valgrind --fair-sched'
a2 = '        valgrind --tool'
assert a1 in s and a2 in s
s = s.replace(a1, '          ' + V + ' --fair-sched').replace(a2, '        ' + V + ' --tool')
# dir= the checkout's tools/speed (baseline, bits, fixedrandom.c)
s = s.replace('dir=$(dirname "$0")', 'dir=tools/speed')
open(dst, 'w').write(s)
