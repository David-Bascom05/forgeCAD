import re,sys,collections as C
from inclusive import stacks
Q=['frame','heatsink','gearbox','manifold','crank','volute','conrod']
PROC=re.compile(r'forge_geom::(ruled|varblend|blend|skinned|screw|corner|intcurve|loftmeet|meet|smooth|wrap)|Procedural|procedural')
def cat(s):
    j=' '.join(s); p='P' if PROC.search(j) else 'C'
    has=lambda *w: any(x in j for x in w)
    if 'PairOut' in j: return 'pairs'
    if 'boolean::plan' in j and has('ssi::searched','boolean::trim','ssi::closed_form','boolean::touches','rolls_off','branches_meet'): return 'pairs'
    if has('check::check','selfx::','check::open_at'): return 'validity_'+p
    if has('part_shape','classify::Index>::new','Index>::boxes'): return 'boxes_'+p
    if has('boolean::plan','nary::nary_body','boolean::general','imprint::'):
        if has('classify_specs','Index>::classify','point_in_face','local_class','region_points','face_point','classify::'): return 'classify_'+p
        if has('split_face','lone_cycle','walks','plane_split','local_split','encloses','boolean::apply'.replace('apply','clockwise')): return 'splitfaces_'+p
        if has('boolean::settle','grown_tols','place_cuts','boolean::imprint','boolean::edge_split','share_cuts'): return 'vertices_'+p
        if has('boolean::apply','split_new_pinches','split_old_pinches','add_edge','add_loop','rebuild_shells','Body>::add_','sole_survivors'): return 'sew_'+p
        if has('Curve as forge_geom::curve::CurveGeom>::closest','closest'): return 'vertices_'+p
        return 'boolother_'+p
    if has('forge_ops::fillet'):
        return 'fillet_'+p
    if has('forge_ops::chamfer'): return 'chamfer_'+p
    if has('forge_model::swept::tool_of','forge_model::thread::tools','forge_ops::loft','forge_ops::sweep','forge_ops::skin'): return 'tool_P'
    if has('forge_model::tools','forge_model::tool','forge_ops::'): return 'tool_'+p
    if has('forge_doc','Model>::rebuild::{closure','text::'): return 'parse'
    if has('forge_model::step','step_solids'): return 'stepother_'+p
    return 'modelother'
def steps(part):
    Mi=[];pairs=[];n=[]
    for l in open(f'steps-{part}.txt'):
        if l.startswith('step'): Mi.append(float(l.split()[2]))
        if l.startswith('pairs'):
            a=l.split(' n '); pairs.append([float(x) for x in a[0].split()[1:]]); n.append(eval(a[1]))
    return sum(Mi)/len(Mi),[sum(c)/len(c) for c in zip(*pairs)],[sum(c)/len(c) for c in zip(*n)]
if __name__=='__main__':
  rows={}
  for part in Q:
    c=C.Counter(); N=0
    for s in stacks(f'prof/{part}.txt.gz'):
        j=' '.join(s)
        if 'Model>::update' not in j or 'Model>::edit' in j or 'bench::model' in j: continue
        N+=1; c[cat(s)]+=1
    mi,pr,n=steps(part)
    r={k:v/N*mi for k,v in c.items()}
    r['_total']=mi; r['_pr']=pr; r['_n']=n; rows[part]=r
    print(f'== {part} {mi:.0f} Mi/frame; sampled pairs {r.get("pairs",0):.0f} vs counted {sum(pr):.0f}; pairs n closed kept {n[0]:.0f} rej {n[1]:.0f}, proc kept {n[2]:.0f} rej {n[3]:.0f}')
    print('   closed kept  ssi %.1f edge+ %.1f edge- %.1f rest %.1f | closed rej ssi %.1f edge- %.1f rest %.1f'%(pr[0],pr[1],pr[2],pr[3],pr[4],pr[6],pr[7]))
    print('   proc   kept  ssi %.1f edge+ %.1f edge- %.1f rest %.1f | proc   rej ssi %.1f edge- %.1f rest %.1f'%(pr[8],pr[9],pr[10],pr[11],pr[12],pr[14],pr[15]))
    print('   '+'  '.join(f'{k}={v:.0f}' for k,v in sorted(r.items(),key=lambda x:-x[1] if not x[0].startswith('_') else 0) if not k.startswith('_') and k!='pairs'))
