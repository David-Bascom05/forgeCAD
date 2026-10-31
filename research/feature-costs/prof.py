#!/usr/bin/env python3
"""prof.py SRC N MODE [K] : function profile of feature N-1 alone: inclusive and self, prefix N minus prefix PREV"""
import collections, os, re, subprocess, sys, tempfile
BIN='target-prof/release/examples/featcost'
os.environ['FORGE_BATCH']='cpu'
period=500000
def prof(src,n,mode):
    d='.scratch/perf-%d.data'%os.getpid()
    subprocess.run(['perf','record','-q','-e','instructions:u','-c',str(period),'--call-graph','fp','-o',d,BIN,'run',src,str(n),mode],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    out=subprocess.run(['perf','script','-i',d,'-F','ip,sym','--no-demangle' if False else '-G'] if False else ['perf','script','-i',d,'-F','ip,sym'],capture_output=True,text=True,errors='replace').stdout
    os.remove(d)
    incl=collections.Counter(); self_=collections.Counter(); tot=0
    stacks=out.split('\n\n')
    for s in stacks:
        syms=[]
        for l in s.splitlines():
            parts=l.strip().split(None,1)
            if len(parts)==2: syms.append(clean(parts[1]))
        if not syms: continue
        tot+=1
        self_[syms[0]]+=1
        for x in set(syms): incl[x]+=1
    return tot,incl,self_
def clean(s):
    s=re.sub(r'\+0x[0-9a-f]+$','',s).strip()
    s=re.sub(r'::h[0-9a-f]{16}$','',s)
    s=re.sub(r'<impl [^>]*>','<impl>',s)
    return s
src,n,mode=sys.argv[1],int(sys.argv[2]),sys.argv[3]
k=int(sys.argv[4]) if len(sys.argv)>4 else 30
prev=int(sys.argv[5]) if len(sys.argv)>5 else n-1
a=prof(src,n,mode);b=prof(src,prev,mode)
d=(a[0]-b[0])*period
print(f'# {src} prefix {prev}..{n} {mode}: {d/1e6:.1f}M')
for name,ctr,base in (('inclusive',a[1],b[1]),):
    print('--',name)
    diff=collections.Counter({x:ctr[x]-base[x] for x in ctr if not re.match(r'(\{closure|fold|abs |dot |mul |add |sub |div |neg |sqrt |min |max |len<|as_|ptr<|non_null|write<|read<|with<|try_with|is_nan|initialize<|call |block_box|ceil|floor|eq |lt |le |hull |to_bits|shr |overflowing|map|collect|from_iter|branch|par_map|lang_start|__libc|_start|catch_unwind|main|call_init|do_call|__rust_begin|std::rt|call_once|try_fold|next|unwrap_or|map_err|and_then|ok_or|for_each|sum|<F|<&|<core|<alloc|<std|new<|into_iter|extend|spec_|core::|alloc::|hint|inner|deref|as_ref|try_for|find|position|any|all|filter|min_by|max_by|flatten|cloned|index|get|clone)',x)})
    for x,c in diff.most_common(k):
        print(f'{c*period/1e6:9.1f}M {100*c*period/max(d,1):5.1f}%  {x[:120]}')
