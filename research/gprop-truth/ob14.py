# ob14.py PARTS.json: OCCT's own build of a part (tools/occt), then gprop, its closure defect, the mesh, and the faces that differ most
import sys,json
sys.path.insert(0,'../../tools/occt')  # run from research/gprop-truth
from build import Opts, build
exec(open('tc3.py').read().split('out=open(')[0])
d=json.load(open(sys.argv[1]))
for p in d['parts']:
    s=build(p['features'],Opts(False,0))
    F=faces(s); L0=baryc(s); bb=Bnd_Box(); BRepBndLib.Add_s(s,bb); dg=bb.CornerMin().Distance(bb.CornerMax())
    g=GProp_GProps(); BRepGProp.VolumeProperties_s(s,g); gv=g.Mass()
    v0=gvol(F,L0); vs=[gvol(F,tuple(L0[k]+(dg if k==j else 0) for k in range(3))) for j in range(3)]
    cn=sum(((v0-v)*3/dg)**2 for v in vs)**.5
    L1=(L0[0]+dg,L0[1]+dg,L0[2]+dg); (m,mb),ma,miss=mesh(s,F,3e-5,0.01,L0,L1); ed=edgedev(F)
    print(p['name'],'built: gprop %.8g mesh %.8g (%+.2e) mesh shift %.1e  closure defect %.2e reach %.1e rel  pcurve off edge %.1e (rel %.2f) edge tol max %.1e'%(gv,m,(gv-m)/m,abs(m-mb)/m,cn,cn*dg/3/m,ed[0],ed[1],maxtol(s,TopAbs_EDGE)))
