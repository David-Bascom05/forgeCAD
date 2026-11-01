# loops.py FILE..: per STEP file, loops whose consecutive oriented edges do not share a VERTEX_POINT (a missing edge), and the
# largest distance from a vertex point to its edge curve's nearest sampled point is left to OCCT; here only the topology
import re,sys
for fn in sys.argv[1:]:
    t=open(fn,errors='replace').read().replace('\n','')
    ent={int(m.group(1)):(m.group(2),m.group(3)) for m in re.finditer(r'#(\d+)\s*=\s*([A-Z_0-9]+)\s*\((.*?)\)\s*;',t)}
    refs=lambda s:[int(x) for x in re.findall(r'#(\d+)',s)]
    bad=0; nl=0
    for k,(kw,a) in ent.items():
        if kw!='EDGE_LOOP': continue
        nl+=1; ends=[]
        for oe in refs(a):
            _,oa=ent[oe]; ec=refs(oa)[-1]; fw=oa.strip().endswith('.T.')
            _,ea=ent[ec]; v0,v1=refs(ea)[:2]
            ends.append((v0,v1) if fw else (v1,v0))
        for j in range(len(ends)):
            if ends[j][1]!=ends[(j+1)%len(ends)][0]: bad+=1
    print(fn.split('/')[-1],'loops',nl,'junctions not sharing a vertex',bad)
