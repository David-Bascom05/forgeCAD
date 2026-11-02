// closing4 h0 r h: added volume of a rolling-ball closing of plate + wall (line and half round, top a sloping plane), on a grid
fn edt1(f:&mut [f32],v:&mut [usize],z:&mut [f32],o:&mut [f32]){let n=f.len();let mut k=0;v[0]=0;z[0]=f32::NEG_INFINITY;z[1]=f32::INFINITY;
 for q in 1..n{ if f[q]>=1e18{continue}
  if f[v[0]]>=1e18{v[0]=q;continue}
  loop{let p=v[k];let s=((f[q]+(q*q) as f32)-(f[p]+(p*p) as f32))/(2.0*(q as f32-p as f32));if s<=z[k]&&k>0{k-=1;continue}
   if s<=z[k]{v[0]=q;z[1]=f32::INFINITY;break}
   k+=1;v[k]=q;z[k]=s;z[k+1]=f32::INFINITY;break}}
 if f[v[0]]>=1e18{return}
 k=0;for q in 0..n{while z[k+1]<q as f32{k+=1}let p=v[k];let d=q as f32-p as f32;o[q]=d*d+f[p]}f.copy_from_slice(&o[..n]);}
fn edt(m:&[bool],d:[usize;3])->Vec<f32>{let [a,b,c]=d;let mut g:Vec<f32>=m.iter().map(|&x|if x{0.0}else{1e20}).collect();
 let w=a.max(b).max(c);let (mut v,mut z,mut o,mut l)=(vec![0usize;w],vec![0f32;w+1],vec![0f32;w],vec![0f32;w]);
 for i in 0..a{for j in 0..b{let s=(i*b+j)*c;edt1(&mut g[s..s+c],&mut v,&mut z,&mut o);}}
 for i in 0..a{for k in 0..c{for j in 0..b{l[j]=g[(i*b+j)*c+k]}edt1(&mut l[..b],&mut v,&mut z,&mut o);for j in 0..b{g[(i*b+j)*c+k]=l[j]}}}
 for j in 0..b{for k in 0..c{for i in 0..a{l[i]=g[(i*b+j)*c+k]}edt1(&mut l[..a],&mut v,&mut z,&mut o);for i in 0..a{g[(i*b+j)*c+k]=l[i]}}}
 g}
fn main(){let a:Vec<f64>=std::env::args().skip(1).map(|s|s.parse().unwrap()).collect();let(h0,r,h)=(a[0],a[1],a[2]);
 let (x0,y0,z0)=(-12.0f64,-16.0f64,-2.0f64);let n=|l:f64|(l/h).ceil() as usize;let d=[n(50.0),n(32.0),n(14.0)];
 let s_=(0.5-h0)/5.0;let top=|x:f64|h0+s_*(x-20.0);
 let mut s=vec![false;d[0]*d[1]*d[2]];
 for i in 0..d[0]{let x=x0+(i as f64+0.5)*h;for j in 0..d[1]{let y=y0+(j as f64+0.5)*h;
  // the plate is taken as unbounded below and out to the box: the far ends are subtracted by a control below
  let inw=x>=0.0&&((x<=20.0&&y.abs()<=5.0)||((x-20.0).powi(2)+y*y<=25.0));
  for k in 0..d[2]{let z=z0+(k as f64+0.5)*h;s[(i*d[1]+j)*d[2]+k]=z<=0.0||(inw&&z<=top(x)&&z<=30.0);}}}
 let ds=edt(&s,d);let r2=((r/h)*(r/h)) as f32;let far:Vec<bool>=ds.iter().map(|&q|q>=r2).collect();drop(ds);let d2=edt(&far,d);
 let mut n_add=0u64;let mut bad=0u64;for i in 0..s.len(){if !s[i]&&d2[i]>r2{n_add+=1}}let _=&mut bad;println!("h={h} added {}",n_add as f64*h*h*h);}
