//! Experimental: the n-ary boolean's region verdicts on an NVIDIA GPU.
//!
//! The CUDA driver library is opened at run time (no toolkit at build time, no crates); the
//! kernel ships as PTX (kernels/classify.cu, rebuilt by kernels/build.sh) that the driver
//! compiles for whatever GPU it finds. This is the only crate with unsafe code: the calls
//! into the driver.

use forge_bool::{Batch, Op, Seg2, Shape};
use forge_math::{Box3, Point3};
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::sync::Mutex;

const PTX: &str = include_str!("../kernels/classify.ptx");
/// Doubles per tool and per profile segment (as the kernel reads them).
const TOOL: usize = 32;
const SEG: usize = 6;
const MARGIN: f64 = 1e-4;
/// Single precision is good to ~2e-5 over a 300 mm part: it answers only points clear of every
/// boundary by this much, and leaves the rest undecided.
const MARGIN32: f32 = 1e-3;

unsafe extern "C" {
    fn dlopen(name: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

type Res = c_int;
type Ptr = u64;

/// The driver entry points used.
#[allow(clippy::type_complexity)]
struct Api {
    init: extern "C" fn(c_uint) -> Res,
    device_get: extern "C" fn(*mut c_int, c_int) -> Res,
    device_name: extern "C" fn(*mut c_char, c_int, c_int) -> Res,
    primary_ctx: extern "C" fn(*mut *mut c_void, c_int) -> Res,
    set_current: extern "C" fn(*mut c_void) -> Res,
    load: extern "C" fn(*mut *mut c_void, *const c_void) -> Res,
    function: extern "C" fn(*mut *mut c_void, *mut c_void, *const c_char) -> Res,
    alloc: extern "C" fn(*mut Ptr, usize) -> Res,
    free: extern "C" fn(Ptr) -> Res,
    to_device: extern "C" fn(Ptr, *const c_void, usize) -> Res,
    to_host: extern "C" fn(*mut c_void, Ptr, usize) -> Res,
    launch: extern "C" fn(
        *mut c_void,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        *mut c_void,
        *mut *mut c_void,
        *mut *mut c_void,
    ) -> Res,
    sync: extern "C" fn() -> Res,
}

/// A symbol's address as the function pointer type it is.
///
/// # Safety
/// `F` must be the function pointer type of the symbol at `p`.
unsafe fn cast<F: Copy>(p: *mut c_void) -> F {
    // SAFETY: function pointers and data pointers have the same size on the targets CUDA runs on
    unsafe { std::mem::transmute_copy(&p) }
}

#[derive(Debug)]
pub struct GpuError(pub String);

fn ok(r: Res, what: &str) -> Result<(), GpuError> {
    if r == 0 {
        Ok(())
    } else {
        Err(GpuError(format!("{what} failed: CUDA error {r}")))
    }
}

/// A device buffer that grows as needed.
struct Buf {
    ptr: Ptr,
    cap: usize,
}

struct Bufs {
    tools: Buf,
    segs: Buf,
    points: Buf,
    sides: Buf,
    out: Buf,
}

/// The GPU, with the kernel loaded.
pub struct Gpu {
    api: Api,
    ctx: *mut c_void,
    kernel: *mut c_void,
    kernel32: *mut c_void,
    pub name: String,
    bufs: Mutex<Bufs>,
}

// the context and kernel handles are process-wide driver objects; every call makes the
// context current on the calling thread first, and buffers sit behind a mutex
unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}

impl Gpu {
    /// Opens the first CUDA device and loads the kernel.
    pub fn new() -> Result<Gpu, GpuError> {
        let lib = CString::new("libcuda.so.1").expect("no nul");
        // SAFETY: dlopen with a valid C string; RTLD_NOW = 2
        let h = unsafe { dlopen(lib.as_ptr(), 2) };
        if h.is_null() {
            return Err(GpuError("libcuda.so.1 not found".into()));
        }
        let sym = |name: &str| -> Result<*mut c_void, GpuError> {
            let c = CString::new(name).expect("no nul");
            // SAFETY: valid handle and C string
            let p = unsafe { dlsym(h, c.as_ptr()) };
            if p.is_null() {
                Err(GpuError(format!("{name} missing")))
            } else {
                Ok(p)
            }
        };
        // SAFETY: each symbol is the driver function of that name, with the signature declared
        // in Api (CUDA driver API, stable ABI)
        let api = unsafe {
            Api {
                init: cast(sym("cuInit")?),
                device_get: cast(sym("cuDeviceGet")?),
                device_name: cast(sym("cuDeviceGetName")?),
                primary_ctx: cast(sym("cuDevicePrimaryCtxRetain")?),
                set_current: cast(sym("cuCtxSetCurrent")?),
                load: cast(sym("cuModuleLoadData")?),
                function: cast(sym("cuModuleGetFunction")?),
                alloc: cast(sym("cuMemAlloc_v2")?),
                free: cast(sym("cuMemFree_v2")?),
                to_device: cast(sym("cuMemcpyHtoD_v2")?),
                to_host: cast(sym("cuMemcpyDtoH_v2")?),
                launch: cast(sym("cuLaunchKernel")?),
                sync: cast(sym("cuCtxSynchronize")?),
            }
        };
        ok((api.init)(0), "cuInit")?;
        let mut dev = 0;
        ok((api.device_get)(&mut dev, 0), "cuDeviceGet")?;
        let mut name = [0 as c_char; 128];
        ok((api.device_name)(name.as_mut_ptr(), 128, dev), "cuDeviceGetName")?;
        // SAFETY: the driver wrote a nul-terminated name into the buffer
        let name = unsafe { std::ffi::CStr::from_ptr(name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let mut ctx = std::ptr::null_mut();
        ok((api.primary_ctx)(&mut ctx, dev), "cuDevicePrimaryCtxRetain")?;
        ok((api.set_current)(ctx), "cuCtxSetCurrent")?;
        let ptx = CString::new(PTX).expect("ptx has no nul");
        let mut module = std::ptr::null_mut();
        ok((api.load)(&mut module, ptx.as_ptr().cast()), "cuModuleLoadData")?;
        let mut kernel = std::ptr::null_mut();
        let fname = CString::new("verdicts").expect("no nul");
        ok(
            (api.function)(&mut kernel, module, fname.as_ptr()),
            "cuModuleGetFunction",
        )?;
        let mut kernel32 = std::ptr::null_mut();
        let fname = CString::new("verdicts32").expect("no nul");
        ok(
            (api.function)(&mut kernel32, module, fname.as_ptr()),
            "cuModuleGetFunction",
        )?;
        let empty = || Buf { ptr: 0, cap: 0 };
        Ok(Gpu {
            api,
            ctx,
            kernel,
            kernel32,
            name,
            bufs: Mutex::new(Bufs {
                tools: empty(),
                segs: empty(),
                points: empty(),
                sides: empty(),
                out: empty(),
            }),
        })
    }

    fn upload<T>(&self, buf: &mut Buf, data: &[T]) -> Result<(), GpuError> {
        let bytes = std::mem::size_of_val(data).max(8);
        if buf.cap < bytes {
            if buf.ptr != 0 {
                ok((self.api.free)(buf.ptr), "cuMemFree")?;
            }
            let cap = bytes.next_power_of_two();
            ok((self.api.alloc)(&mut buf.ptr, cap), "cuMemAlloc")?;
            buf.cap = cap;
        }
        if !data.is_empty() {
            ok(
                (self.api.to_device)(buf.ptr, data.as_ptr().cast(), std::mem::size_of_val(data)),
                "cuMemcpyHtoD",
            )?;
        }
        Ok(())
    }

    /// Verdicts for `points` (each with its own tool `sides[i]`) against packed tools.
    /// in double precision, or single (`single`: faster on consumer GPUs, more left undecided).
    pub fn run(
        &self,
        tools: &Packed,
        points: &[Point3],
        sides: &[u32],
        boxes: &[Box3],
        single: bool,
    ) -> Result<Vec<u8>, GpuError> {
        let n = points.len();
        if n == 0 {
            return Ok(Vec::new());
        }
        let mut t = tools.tools.clone();
        for (k, b) in boxes.iter().enumerate() {
            let at = k * TOOL + 24;
            t[at..at + 6].copy_from_slice(&[b.x.lo, b.y.lo, b.z.lo, b.x.hi, b.y.hi, b.z.hi]);
        }
        let flat: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y, p.z]).collect();
        let mut bufs = self.bufs.lock().expect("gpu buffers");
        ok((self.api.set_current)(self.ctx), "cuCtxSetCurrent")?;
        let b = &mut *bufs;
        if single {
            let f = |v: &[f64]| v.iter().map(|&x| x as f32).collect::<Vec<f32>>();
            self.upload(&mut b.tools, &f(&t))?;
            self.upload(&mut b.segs, &f(&tools.segs))?;
            self.upload(&mut b.points, &f(&flat))?;
        } else {
            self.upload(&mut b.tools, &t)?;
            self.upload(&mut b.segs, &tools.segs)?;
            self.upload(&mut b.points, &flat)?;
        }
        self.upload(&mut b.sides, sides)?;
        self.upload::<u8>(&mut b.out, &vec![0u8; n])?;
        let mut a_tools = b.tools.ptr;
        let mut a_ntools = (t.len() / TOOL) as c_int;
        let mut a_segs = b.segs.ptr;
        let mut a_points = b.points.ptr;
        let mut a_sides = b.sides.ptr;
        let mut a_n = n as c_int;
        let mut a_any = 0 as c_int;
        let mut a_margin = MARGIN;
        let mut a_margin32 = MARGIN32;
        let mut a_out = b.out.ptr;
        let mut params: [*mut c_void; 9] = [
            (&raw mut a_tools).cast(),
            (&raw mut a_ntools).cast(),
            (&raw mut a_segs).cast(),
            (&raw mut a_points).cast(),
            (&raw mut a_sides).cast(),
            (&raw mut a_n).cast(),
            (&raw mut a_any).cast(),
            if single {
                (&raw mut a_margin32).cast()
            } else {
                (&raw mut a_margin).cast()
            },
            (&raw mut a_out).cast(),
        ];
        let block = 128u32;
        let grid = (n as u32).div_ceil(block);
        ok(
            (self.api.launch)(
                if single { self.kernel32 } else { self.kernel },
                grid,
                1,
                1,
                block,
                1,
                1,
                0,
                std::ptr::null_mut(),
                params.as_mut_ptr(),
                std::ptr::null_mut(),
            ),
            "cuLaunchKernel",
        )?;
        ok((self.api.sync)(), "cuCtxSynchronize")?;
        let mut out = vec![0u8; n];
        ok(
            (self.api.to_host)(out.as_mut_ptr().cast(), b.out.ptr, n),
            "cuMemcpyDtoH",
        )?;
        Ok(out)
    }
}

/// Tools packed as the kernel reads them (boxes filled in per run).
#[derive(Clone)]
pub struct Packed {
    tools: Vec<f64>,
    segs: Vec<f64>,
}

impl Packed {
    pub fn new(shapes: &[Shape], ops: &[Op]) -> Packed {
        let mut tools = Vec::with_capacity(shapes.len() * TOOL);
        let mut segs: Vec<f64> = Vec::new();
        for (k, sh) in shapes.iter().enumerate() {
            let mut t = [0.0f64; TOOL];
            t[1] = match ops.get(k.wrapping_sub(1)) {
                _ if k == 0 => 0.0,
                Some(Op::Union) => 0.0,
                Some(Op::Subtract) => 1.0,
                _ => 2.0,
            };
            let mut put_segs = |t: &mut [f64; TOOL], ss: &[Seg2]| {
                t[2] = (segs.len() / SEG) as f64;
                t[3] = ss.len() as f64;
                for s in ss {
                    segs.extend_from_slice(&match *s {
                        Seg2::Line { a, b } => [0.0, a.x, a.y, b.x, b.y, 0.0],
                        Seg2::Arc { c, r, start, sweep } => [1.0, c.x, c.y, r, start, sweep],
                    });
                }
            };
            let put_frame = |t: &mut [f64; TOOL], f: &forge_math::Frame| {
                let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
                t[4..16].copy_from_slice(&[
                    f.origin.x, f.origin.y, f.origin.z, x.x, x.y, x.z, y.x, y.y, y.z, z.x, z.y, z.z,
                ]);
            };
            match sh {
                Shape::Prism {
                    frame,
                    segs: ss,
                    lo,
                    hi,
                } => {
                    t[0] = 0.0;
                    put_segs(&mut t, ss);
                    put_frame(&mut t, frame);
                    t[16] = *lo;
                    t[17] = *hi;
                }
                Shape::Revolved {
                    frame,
                    origin,
                    dir,
                    perp,
                    segs: ss,
                    angle,
                } => {
                    t[0] = 1.0;
                    put_segs(&mut t, ss);
                    put_frame(&mut t, frame);
                    t[16] = angle.unwrap_or(-1.0);
                    t[18..24].copy_from_slice(&[origin.x, origin.y, dir.x, dir.y, perp.x, perp.y]);
                }
                Shape::Ball { center, radius } => {
                    t[0] = 2.0;
                    t[4..7].copy_from_slice(&[center.x, center.y, center.z]);
                    t[16] = *radius;
                }
            }
            tools.extend_from_slice(&t);
        }
        Packed { tools, segs }
    }
}

/// A batch run on the GPU; a GPU error leaves every region to the full way.
pub struct GpuBatch<'a> {
    pub gpu: &'a Gpu,
    pub packed: Packed,
    /// Single precision.
    pub single: bool,
}

impl Batch for GpuBatch<'_> {
    fn verdicts(&self, points: &[Point3], sides: &[u32], boxes: &[Box3]) -> Vec<u8> {
        self.gpu
            .run(&self.packed, points, sides, boxes, self.single)
            .unwrap_or_else(|_| vec![3; points.len()])
    }
}
