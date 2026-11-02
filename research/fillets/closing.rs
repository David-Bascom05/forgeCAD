// closing rr cy rb r h: added volume of a rolling-ball closing of the block+boss scene, on a grid, nothing shared with the kernel
fn edt1(f:&mut [f32],v:&mut [usize],z:&mut [f32],o:&mut [f32]){let n=f.len();let mut k=0;v[0]=0;z[0]=f32::NEG_INFINITY;z[1]=f32::INFINITY;for q in 1..n{if f[q]>=1e18{continue}
 if f[v[0]]>=1e18{v[0]=q;continue}
 loop{let p=v[k];let s=((f[q]+(q*q) as f32)-(f[p]+(p*p) as f32))/(2.0*(q as f32-p as f32));if s<=z[k]&&k>0{k-=1;continue}
  if s<=z[k]{v[0]=q;z[1]=f32::INFINITY;break}
  k+=1;v[k]=q;z[k]=s;z[k+1]=f32::INFINITY;break}}
 if f[v[0]]>=1e18{return}
 k=0;for q in 0..n{while z[k+1]<q as f32{k+=1}let p=v[k];let d=q as f32-p as f32;o[q]=d*d+f[p]}f.copy_from_slice(&o[..n]);}
fn edt(m:&[bool],d:[usize;3])->Vec<f32>{let [a,b,c]=d;let mut g:Vec<f32>=m.iter().map(|&x|if x{0.0}else{1e20}).collect();let mx=a.max(b).max(c);let(mut v,mut z,mut o,mut l)=(vec![0;mx+1],vec![0f32;mx+2],vec![0f32;mx+1],vec![0f32;mx+1]);
 for i in 0..a{for j in 0..b{let s=(i*b+j)*c;edt1(&mut g[s..s+c],&mut v,&mut z,&mut o);}}
 for i in 0..a{for k in 0..c{for j in 0..b{l[j]=g[(i*b+j)*c+k]}edt1(&mut l[..b],&mut v,&mut z,&mut o);for j in 0..b{g[(i*b+j)*c+k]=l[j]}}}
 for j in 0..b{for k in 0..c{for i in 0..a{l[i]=g[(i*b+j)*c+k]}edt1(&mut l[..a],&mut v,&mut z,&mut o);for i in 0..a{g[(i*b+j)*c+k]=l[i]}}}
 g}
fn main(){let a:Vec<f64>=std::env::args().skip(1).map(|s|s.parse().unwrap()).collect();let(rr,cy,rb,r,h)=(a[0],a[1],a[2],a[3],a[4]);let pad=2.0*r+2.0;
 let(x0,y0,z0)=(-pad,30.0-rr-pad,-12.0-pad);let n=|l:f64|(l/h).ceil() as usize;let d=[n(rb+2.0*pad),n(cy+rb+pad-y0),n(46.0-z0)];
 let mut s=vec![false;d[0]*d[1]*d[2]];
 for i in 0..d[0]{let x=x0+(i as f64+0.5)*h;for j in 0..d[1]{let y=y0+(j as f64+0.5)*h;for k in 0..d[2]{let z=z0+(k as f64+0.5)*h;
  let mut b=y<=30.0&&z<=30.0;if y>30.0-rr&&z>30.0-rr&&(y-30.0+rr).powi(2)+(z-30.0+rr).powi(2)>rr*rr{b=false}
  let c=x*x+(y-cy).powi(2)<=rb*rb;s[(i*d[1]+j)*d[2]+k]=b||c;}}}
 let ds=edt(&s,d);let r2=((r/h)*(r/h)) as f32;let far:Vec<bool>=ds.iter().map(|&q|q>=r2).collect();drop(ds);let d2=edt(&far,d);
 let mut n_add=0u64;for i in 0..s.len(){let k=i%d[2];let z=z0+(k as f64+0.5)*h;let ix=i/d[2]/d[1];let x=x0+(ix as f64+0.5)*h;if z>=0.0&&x>=0.0&&!s[i]&&d2[i]>r2{n_add+=1}}println!("h={h} added {}",2.0*n_add as f64*h*h*h);}
