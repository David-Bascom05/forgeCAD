#!/usr/bin/env python3
"""OCCT's area of each face of a STEP file (precision 1e-9), to hold against `stepread --faces`.
  $VENV/bin/python -I occt_faces.py FILE"""
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepAdaptor import BRepAdaptor_Surface
r=STEPControl_Reader(); r.ReadFile(sys.argv[1]); r.TransferRoots(); s=r.OneShape()
ex=TopExp_Explorer(s,TopAbs_FACE); i=0
while ex.More():
    f=TopoDS.Face(ex.Current()); g=GProp_GProps(); BRepGProp.SurfaceProperties_s(f,g,1e-9)
    print('occt face',i,BRepAdaptor_Surface(f).GetType(),g.Mass()); i+=1; ex.Next()
