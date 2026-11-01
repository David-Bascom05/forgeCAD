# prec2.py IDS OUT: gprop volume and area of each file read at its stated precision and forced to 1e-6
import sys,json
from OCP.STEPControl import STEPControl_Reader
from OCP.Interface import Interface_Static
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
def rd(i,mode):
    Interface_Static.SetIVal_s('read.precision.mode',mode); Interface_Static.SetRVal_s('read.precision.val',1e-6)
    Interface_Static.SetIVal_s('read.maxprecision.mode',mode); Interface_Static.SetRVal_s('read.maxprecision.val',1e-6 if mode else 1.0)
    r=STEPControl_Reader(); r.ReadFile('/home/david-bascom/forgecad-data/whucad/run/stp/'+i+'.stp'); r.TransferRoots(); s=r.OneShape()
    g=GProp_GProps(); BRepGProp.VolumeProperties_s(s,g); a=GProp_GProps(); BRepGProp.SurfaceProperties_s(s,a); return g.Mass(),a.Mass()
o=open(sys.argv[2],'w')
for i in open(sys.argv[1]).read().split():
    try:
        v0,a0=rd(i,0); v1,a1=rd(i,1); o.write(json.dumps(dict(id=i,v0=v0,a0=a0,v1=v1,a1=a1))+'\n')
    except Exception as x: o.write(json.dumps(dict(id=i,err=repr(x)[:80]))+'\n')
    o.flush()
