"""A small reader of Parasolid text transmit files, for surveys and for cutting fixtures out of
the corpus (extract.py): the tokenizer, the base schema SCH_13006 and the embedded edits of it.
The Rust reader (kernel/forge-step/src/xt) is the real one; this one keeps each type's schema
words as the file wrote them, so a cut can carry them unchanged.

    python3 research/parasolid/xtproto.py FILE.x_t...   (a census of node types)
"""
import sys, collections
G="node_id:d af:p owner:p next:p previous:p geometric_owner:p sense:c"
BASE_SRC = {
10:"highest_node_id:d af:p attribute_chains:p list:p surface:p curve:p point:p key:p res_size:f res_linear:f ref_instance:p next:p previous:p state:u owner:p type:u sub_instance:p",
11:"node_id:d af:p type:u part:p transform:p assembly:p next_in_part:p prev_in_part:p next_of_part:p prev_of_part:p",
12:"highest_node_id:d af:p attribute_chains:p surface:p curve:p point:p key:p res_size:f res_linear:f ref_instance:p next:p previous:p state:u owner:p body_type:u nom_geom_state:u shell:p boundary_surface:p boundary_curve:p boundary_point:p region:p edge:p vertex:p",
13:"node_id:d af:p body:p next:p face:p edge:p vertex:p region:p front_face:p",
14:"node_id:d af:p tolerance:f next:p previous:p loop:p shell:p surface:p sense:c next_on_surface:p previous_on_surface:p next_front:p previous_front:p front_shell:p",
15:"node_id:d af:p fin:p face:p next:p",
16:"node_id:d af:p tolerance:f fin:p previous:p next:p curve:p next_on_curve:p previous_on_curve:p owner:p",
17:"af:p loop:p forward:p backward:p vertex:p other:p edge:p curve:p next_at_vx:p sense:c",
18:"node_id:d af:p fin:p previous:p next:p point:p tolerance:f owner:p",
19:"node_id:d af:p body:p next:p previous:p shell:p type:c",
29:"node_id:d af:p owner:p next:p previous:p pvec:v",
30:G+" pvec:v direction:v",
31:G+" centre:v normal:v x_axis:v radius:f",
32:G+" centre:v normal:v x_axis:v major_radius:f minor_radius:f",
38:G+" surface:p[2] chart:p start:p end:p",
40:"base_parameter:f base_scale:f chart_count:d chordal_error:f angular_error:f parameter_error:f[2] hvec:h[]",
41:"type:c hvec:h[]",
45:"vertices:f[]",
50:G+" pvec:v normal:v x_axis:v",
51:G+" pvec:v axis:v radius:f x_axis:v",
52:G+" pvec:v axis:v radius:f sin_half_angle:f cos_half_angle:f x_axis:v",
53:G+" centre:v radius:f axis:v x_axis:v",
54:G+" centre:v axis:v major_radius:f minor_radius:f x_axis:v",
56:G+" blend_type:c surface:p[2] spine:p range:f[2] thumb_weight:f[2] boundary:p[2] start:p end:p",
59:G+" boundary:n blend:p",
60:G+" check:c true_offset:l surface:p offset:f scale:f",
67:G+" section:p sweep:v scale:f",
68:G+" profile:p base:v axis:v start:v end:v start_param:f end_param:f x_axis:v scale:f",
70:"node_id:d owner:p next:p previous:p list_type:d list_length:d block_length:d size_of_entry:d list_block:p",
74:"n_entries:d next_block:p entries:p[]",
79:"string:c[]",
80:"next:p identifier:p type_id:d actions:u[8] field_names:p legal_owners:l[14] fields:u[]",
81:"node_id:d definition:p owner:p next:p previous:p next_of_type:p previous_of_type:p fields:p[]",
82:"values:d[]", 83:"values:f[]", 84:"values:c[]", 85:"values:v[]", 86:"values:v[]", 87:"values:v[]", 88:"values:t[]", 89:"values:v[]",
90:"node_id:d af:p owner:p next:p previous:p type:u first_member:p",
91:"dummy_node_id:d owning_group:p owner:p next:p previous:p next_member:p previous_member:p",
98:"values:w[]", 99:"names:p[]",
100:"node_id:d owner:p next:p previous:p rotation_matrix:f[9] translation_vector:v scale:f flag:d perspective_vector:v",
101:"assembly:p attribute:p body:p transform:p surface:p curve:p point:p alive:l attrib_def:p highest_id:d current_id:d",
102:"string:c[]",
120:G+" type:c data:p tf:p internal_geom:p[]",
121:"geom_type:d real_array:p int_array:p", 122:"key:p real_array:p int_array:p",
124:G+" nurbs:p data:p",
125:"original_uint:i original_vint:i extended_uint:i extended_vint:i self_int:u original_u_start:c original_u_end:c original_v_start:c original_v_end:c extended_u_start:c extended_u_end:c extended_v_start:c extended_v_end:c analytic_form_type:c swept_form_type:c spun_form_type:c blend_form_type:c analytic_form:p swept_form:p spun_form:p blend_form:p",
126:"u_periodic:l v_periodic:l u_degree:n v_degree:n n_u_vertices:d n_v_vertices:d u_knot_type:u v_knot_type:u n_u_knots:d n_v_knots:d rational:l u_closed:l v_closed:l surface_form:u vertex_dim:n bspline_vertices:p u_knot_mult:p v_knot_mult:p u_knots:p v_knots:p",
127:"mult:n[]", 128:"knots:f[]",
130:G+" type:c data:p tf:p internal_geom:p[]",
133:G+" basis_curve:p point_1:v point_2:v parm_1:f parm_2:f",
134:G+" nurbs:p data:p",
135:"self_int:u analytic_form:p",
136:"degree:n n_vertices:d vertex_dim:n n_knots:d knot_type:u periodic:l closed:l rational:l curve_form:u bspline_vertices:p knot_mult:p knots:p",
137:G+" surface:p b_curve:p original:p tolerance_to_original:f",
163:"axis_pt:v axis_dir:v hand:c turns:i pitch:f gap:f tol:f",
184:"axis_pt:v axis_dir:v point:v hand:c turns:i pitch:f tol:f",
141:"owner:p next:p previous:p shared_geometry:p",
}
def fields(src):
    out=[]
    for f in src.split():
        n,t=f.split(':'); k=0
        if '[' in t:
            t,a=t[:-1].split('['); k=1 if a=='' else int(a)
        out.append((n,t,k))
    return out
BASE={k:fields(v) for k,v in BASE_SRC.items()}

class Tok:
    def __init__(s,b): s.b=b; s.i=0
    def num(s):
        b=s.b; i=s.i
        if b[i:i+1]==b'?':
            s.i=i+1; return b'?'
        j=b.index(b' ',i)
        s.i=j+1
        return b[i:j]
    def ch(s):
        c=s.b[s.i:s.i+1]; s.i+=1; return c
    def int_(s):
        x=s.num()
        return None if x==b'?' else int(x)
    def string(s):
        n=s.int_(); r=s.b[s.i:s.i+n]; s.i+=n; return r

def read_field_desc(t):
    name=t.string().decode(); pc=t.int_(); ne=t.int_()
    typ=None if pc else t.string().decode()
    if pc: typ='p'
    xmt=None
    if ne==1: xmt=t.ch()
    return (name,typ,ne,xmt)

def val(t,typ):
    if typ in 'cl': return t.ch()
    if typ in 'vh':
        x=t.num()
        if x==b'?': return None
        return (float(x),float(t.num()),float(t.num()))
    if typ in 'ib':
        x=t.num()
        if x==b'?': return None
        return (float(x),)+tuple(float(t.num()) for _ in range(1 if typ=='i' else 5))
    x=t.num()
    return None if x==b'?' else (float(x) if typ=='f' else int(x))

def parse(data):
    i=data.index(b'**END_OF_HEADER'); i=data.index(b'\n',i)+1
    hdr=data[:i].decode('latin1')
    b=data[i:].replace(b'\r',b'').replace(b'\n',b'')
    t=Tok(b)
    assert t.ch()==b'T'
    ver=t.string(); sch=t.string().decode()
    embedded = sch.count('_')==3
    maxtypes = t.int_() if embedded else None
    usfld=t.int_()
    schema={}   # type -> (fields, variable)
    nodes={}
    global DESC, HEAD, DESCPOS, BODY
    DESC={}     # type -> the schema words the file wrote at its first node, raw
    DESCPOS={}  # type -> where they lie in BODY (the records joined)
    BODY=b
    HEAD=(ver,sch,maxtypes,usfld)
    while True:
        ty=t.int_()
        if ty==1:
            idx=t.int_(); assert idx==0; break
        if ty not in schema:
            d0=t.i
            base=BASE.get(ty)
            if embedded:
                if base is None:
                    n=t.int_(); nm=t.string(); ds=t.string()
                    fl=[read_field_desc(t) for _ in range(n)]
                    cur=[(a,b_,c) for a,b_,c,_ in fl]
                else:
                    n=t.int_()
                    if n==255: cur=list(base)
                    else:
                        cur=[]; bi=0
                        while True:
                            c=t.ch()
                            if c==b'C': cur.append(base[bi]); bi+=1
                            elif c==b'D': bi+=1
                            elif c in (b'I',b'A'):
                                d=read_field_desc(t); cur.append(d[:3])
                            elif c==b'Z': break
                            else: raise Exception(f'bad edit {c} type {ty} at {t.i}')
                        assert bi==len(base), (ty,bi,len(base))
                        assert len(cur)==n,(ty,len(cur),n)
            else:
                cur=base
            schema[ty]=cur
            DESC[ty]=t.b[d0:t.i]
            DESCPOS[ty]=(d0,t.i)
        cur=schema[ty]
        if ty==1:
            idx=t.int_(); assert idx==0; break
        var = any(k==1 for _,_,k in cur)
        vl = t.int_() if var else None
        idx=t.int_()
        rec={}
        global LAST
        LAST=(ty,idx,cur,t.i)
        for name,typ,k in cur:
            if k==0: rec[name]=val(t,typ)
            else:
                cnt = vl if k==1 else k
                if typ=='c' and k==1: rec[name]=t.b[t.i:t.i+cnt]; t.i+=cnt
                else: rec[name]=[val(t,typ) for _ in range(cnt)]
        nodes[idx]=(ty,rec)
    return hdr,ver,sch,schema,nodes

if __name__=='__main__':
    tot=collections.Counter()
    for f in sys.argv[1:]:
        try: hdr,ver,sch,schema,nodes=parse(open(f,'rb').read())
        except Exception as e:
            ty,idx,cur,at=LAST
            d=open(f,'rb').read(); i=d.index(b'**END_OF_HEADER'); i=d.index(b'\n',i)+1
            b=d[i:].replace(b'\r',b'').replace(b'\n',b'')
            print('FAIL',f,e,'type',ty,'idx',idx,cur); print(b[at-300:at]); print(b[at:at+300]); continue
        c=collections.Counter(ty for ty,_ in nodes.values())
        tot+=c
        print(f, sch, len(nodes), dict(sorted(c.items())))
