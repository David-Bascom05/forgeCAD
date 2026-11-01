# run from research/gprop-truth (reads tc3.py beside it)
# occtbuilt.py: a thin tube cut across by a small cylinder, built by OCCT itself: gprop, its closure defect, the mesh
import sys
exec(open('tc3.py').read().split('out=open(')[0])
from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.gp import gp_Ax2,gp_Dir
for wall,rc in ((0.0078125,0.0434),(0.05,0.0434),(0.0078125,0.2)):
    o=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(0.1,0.2,0),gp_Dir(0,0,1)),0.33,0.2).Shape()
    i=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(0.1,0.2,-0.1),gp_Dir(0,0,1)),0.33-wall,0.4).Shape()
    t=BRepAlgoAPI_Cut(o,i).Shape()
    c=BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(-0.5,0.2,0.1),gp_Dir(1,0,0)),rc,1.2).Shape()
    s=BRepAlgoAPI_Cut(t,c).Shape()
    F=faces(s); L0=baryc(s); bb=Bnd_Box(); BRepBndLib.Add_s(s,bb); dg=bb.CornerMin().Distance(bb.CornerMax())
    g=GProp_GProps(); BRepGProp.VolumeProperties_s(s,g); gv=g.Mass()
    v0=gvol(F,L0); vx=gvol(F,(L0[0]+dg,L0[1],L0[2])); vy=gvol(F,(L0[0],L0[1]+dg,L0[2])); vz=gvol(F,(L0[0],L0[1],L0[2]+dg))
    cn=(((v0-vx)**2+(v0-vy)**2+(v0-vz)**2)**.5)*3/dg
    L1=(L0[0]+dg,L0[1]+dg,L0[2]+dg)
    res=[]
    for d,a in ((5e-4,.1),(1e-4,.03),(3e-5,.01)):
        (m,mb),ma,miss=mesh(s,F,d,a,L0,L1); res.append(m)
    ed=edgedev(F)
    print('wall %.4f cutter r %.4f: gprop %.8g  mesh %s  (gprop-mesh)/mesh %+.2e  closure defect |sum N dA| %.2e -> reach %.1e rel  pcurve off edge %.1e  edge tol %.1e'%(wall,rc,gv,' '.join('%.8g'%m for m in res),(gv-res[-1])/res[-1],cn,cn*dg/3/abs(res[-1]),ed[0],maxtol(s,TopAbs_EDGE)))
