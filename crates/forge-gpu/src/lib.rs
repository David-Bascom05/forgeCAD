//! Experimental: the n-ary boolean's region verdicts on an NVIDIA GPU.
//!
//! The kernel is forge-bool's CpuBatch laid out for a GPU: the same per-point classification
//! over the same flat points, sides and boxes. It runs each point in single precision first,
//! answering only points clear of every boundary by MARGIN32, and in double precision (the
//! CPU's margins, a hair wider) where that is undecided. A verdict it gives is the CPU's.
//!
//! The CUDA driver library is opened at run time (no toolkit at build time, no crates); the
//! kernel ships as PTX (kernels/classify.cu, rebuilt by kernels/build.sh) that the driver
//! compiles for whatever GPU it finds. This is the only crate with unsafe code: the calls
//! into the driver.

use forge_bool::{Batch, CpuBatch, Op, Seg2, Shape};
use forge_math::{Box3, Point3, tol};
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

const PTX: &str = include_str!("../kernels/classify.ptx");
/// Values per tool and per profile segment (as the kernel reads them).
const TOOL: usize = 24;
const SEG: usize = 6;
/// The CPU's margin and graze (forge-bool's nary.rs and shape.rs), a hair wider on the GPU so
/// that its math library's last-bit differences can only make it more cautious, never answer
/// where the CPU would not.
const MARGIN: f64 = tol::LINEAR * (1.0 + 1e-6);
const GRAZE: f64 = 1e-9 * (1.0 + 1e-6);
/// Single precision is good to ~2e-5 over a 300 mm part: it answers only points clear of every
/// boundary by this much, and leaves the rest to double precision.
const MARGIN32: f32 = 1e-3;
const BLOCK: u32 = 128;
/// Threads a batch is spread over: each point gets up to a warp of them (splitting the tools)
/// while the points alone would leave the GPU idle.
const FILL: usize = 1 << 16;
/// Points sent at a time (~120 MB on the device).
const CHUNK: usize = 1 << 22;

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" {
    fn dlopen(name: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

// no driver to open in a browser or a WASI runtime
#[cfg(target_arch = "wasm32")]
unsafe fn dlopen(_: *const c_char, _: c_int) -> *mut c_void {
    std::ptr::null_mut()
}
#[cfg(target_arch = "wasm32")]
unsafe fn dlsym(_: *mut c_void, _: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

type Res = c_int;
type Ptr = u64;
type Handle = *mut c_void;

/// The driver entry points used.
#[allow(clippy::type_complexity)]
struct Api {
    init: extern "C" fn(c_uint) -> Res,
    device_get: extern "C" fn(*mut c_int, c_int) -> Res,
    device_name: extern "C" fn(*mut c_char, c_int, c_int) -> Res,
    primary_ctx: extern "C" fn(*mut Handle, c_int) -> Res,
    set_current: extern "C" fn(Handle) -> Res,
    load: extern "C" fn(*mut Handle, *const c_void) -> Res,
    function: extern "C" fn(*mut Handle, Handle, *const c_char) -> Res,
    alloc: extern "C" fn(*mut Ptr, usize) -> Res,
    free: extern "C" fn(Ptr) -> Res,
    alloc_host: extern "C" fn(*mut *mut c_void, usize) -> Res,
    free_host: extern "C" fn(*mut c_void) -> Res,
    to_device: extern "C" fn(Ptr, *const c_void, usize) -> Res,
    to_device_async: extern "C" fn(Ptr, *const c_void, usize, Handle) -> Res,
    to_host_async: extern "C" fn(*mut c_void, Ptr, usize, Handle) -> Res,
    stream_create: extern "C" fn(*mut Handle, c_uint) -> Res,
    stream_sync: extern "C" fn(Handle) -> Res,
    launch: extern "C" fn(
        Handle,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        Handle,
        *mut *mut c_void,
        *mut *mut c_void,
    ) -> Res,
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
#[derive(Default)]
struct Buf {
    ptr: Ptr,
    cap: usize,
}

/// A page-locked host buffer that grows as needed.
struct Host {
    ptr: *mut u8,
    cap: usize,
}

/// What stays on the device between calls.
struct State {
    stream: Handle,
    /// The `Packed::id` of the tools on the device (0: none).
    loaded: u64,
    tools: Buf,
    segs: Buf,
    tools32: Buf,
    segs32: Buf,
    input: Buf,
    host_in: Host,
    out: Buf,
    host_out: Host,
}

/// The GPU, with the kernel loaded.
pub struct Gpu {
    api: Api,
    ctx: Handle,
    verdicts: Handle,
    wake: Handle,
    pub name: String,
    state: Mutex<State>,
}

// the context and kernel handles are process-wide driver objects; every call makes the
// context current on the calling thread first, and the buffers and stream sit behind a mutex
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
                alloc_host: cast(sym("cuMemAllocHost_v2")?),
                free_host: cast(sym("cuMemFreeHost")?),
                to_device: cast(sym("cuMemcpyHtoD_v2")?),
                to_device_async: cast(sym("cuMemcpyHtoDAsync_v2")?),
                to_host_async: cast(sym("cuMemcpyDtoHAsync_v2")?),
                stream_create: cast(sym("cuStreamCreate")?),
                stream_sync: cast(sym("cuStreamSynchronize")?),
                launch: cast(sym("cuLaunchKernel")?),
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
        let function = |name: &str| -> Result<Handle, GpuError> {
            let mut f = std::ptr::null_mut();
            let c = CString::new(name).expect("no nul");
            ok((api.function)(&mut f, module, c.as_ptr()), "cuModuleGetFunction")?;
            Ok(f)
        };
        let (verdicts, wake) = (function("verdicts")?, function("wake")?);
        // a blocking stream: it waits for the plain copies that load the tools
        let mut stream = std::ptr::null_mut();
        ok((api.stream_create)(&mut stream, 0), "cuStreamCreate")?;
        let host = || Host {
            ptr: std::ptr::null_mut(),
            cap: 0,
        };
        Ok(Gpu {
            api,
            ctx,
            verdicts,
            wake,
            name,
            state: Mutex::new(State {
                stream,
                loaded: 0,
                tools: Buf::default(),
                segs: Buf::default(),
                tools32: Buf::default(),
                segs32: Buf::default(),
                input: Buf::default(),
                host_in: host(),
                out: Buf::default(),
                host_out: host(),
            }),
        })
    }

    fn reserve(&self, buf: &mut Buf, bytes: usize) -> Result<(), GpuError> {
        let bytes = bytes.max(8);
        if buf.cap < bytes {
            if buf.ptr != 0 {
                ok((self.api.free)(buf.ptr), "cuMemFree")?;
                buf.ptr = 0;
                buf.cap = 0;
            }
            let cap = bytes.next_power_of_two();
            ok((self.api.alloc)(&mut buf.ptr, cap), "cuMemAlloc")?;
            buf.cap = cap;
        }
        Ok(())
    }

    fn reserve_host<'h>(&self, host: &'h mut Host, bytes: usize) -> Result<&'h mut [u8], GpuError> {
        let bytes = bytes.max(8);
        if host.cap < bytes {
            if !host.ptr.is_null() {
                ok((self.api.free_host)(host.ptr.cast()), "cuMemFreeHost")?;
                host.ptr = std::ptr::null_mut();
                host.cap = 0;
            }
            let cap = bytes.next_power_of_two();
            let mut p = std::ptr::null_mut();
            ok((self.api.alloc_host)(&mut p, cap), "cuMemAllocHost")?;
            host.ptr = p.cast();
            host.cap = cap;
        }
        // SAFETY: the driver gave `cap` bytes of page-locked memory at `ptr`, owned by this
        // buffer and borrowed mutably with it
        Ok(unsafe { std::slice::from_raw_parts_mut(host.ptr, host.cap) })
    }

    fn upload<T>(&self, buf: &mut Buf, data: &[T]) -> Result<(), GpuError> {
        self.reserve(buf, std::mem::size_of_val(data))?;
        if !data.is_empty() {
            ok(
                (self.api.to_device)(buf.ptr, data.as_ptr().cast(), std::mem::size_of_val(data)),
                "cuMemcpyHtoD",
            )?;
        }
        Ok(())
    }

    /// Puts `tools` on the device unless they are there already.
    fn load(&self, s: &mut State, tools: &Packed) -> Result<(), GpuError> {
        if s.loaded == tools.id {
            return Ok(());
        }
        s.loaded = 0;
        let f = |v: &[f64]| v.iter().map(|&x| x as f32).collect::<Vec<f32>>();
        self.upload(&mut s.tools, &tools.tools)?;
        self.upload(&mut s.segs, &tools.segs)?;
        self.upload(&mut s.tools32, &f(&tools.tools))?;
        self.upload(&mut s.segs32, &f(&tools.segs))?;
        s.loaded = tools.id;
        Ok(())
    }

    fn launch(&self, s: &State, f: Handle, grid: u32, params: &mut [*mut c_void]) -> Result<(), GpuError> {
        ok(
            (self.api.launch)(
                f,
                grid,
                1,
                1,
                BLOCK,
                1,
                1,
                0,
                s.stream,
                params.as_mut_ptr(),
                std::ptr::null_mut(),
            ),
            "cuLaunchKernel",
        )
    }

    /// Wakes an idle GPU (an empty kernel, not waited for): a batch some hundreds of
    /// microseconds later then runs at full clock. Skipped if the GPU is busy.
    pub fn wake(&self) {
        let Ok(s) = self.state.try_lock() else {
            return;
        };
        if (self.api.set_current)(self.ctx) == 0 {
            let _ = self.launch(&s, self.wake, 64, &mut []);
        }
    }

    /// Verdicts for `points` (each with its own tool `sides[i]`) against packed tools, every
    /// tool's box given: CpuBatch's, or 3 (undecided) where the GPU is less sure.
    pub fn run(
        &self,
        tools: &Packed,
        points: &[Point3],
        sides: &[u32],
        boxes: &[Box3],
    ) -> Result<Vec<u8>, GpuError> {
        let n = points.len();
        if n == 0 {
            return Ok(Vec::new());
        }
        assert!(n <= CHUNK, "at most CHUNK points a run");
        assert_eq!(sides.len(), n, "one side per point");
        assert_eq!(boxes.len(), tools.len(), "one box per tool");
        let nt = tools.len();
        let mut st = self.state.lock().expect("gpu state");
        let s = &mut *st;
        ok((self.api.set_current)(self.ctx), "cuCtxSetCurrent")?;
        self.load(s, tools)?;
        // one copy up: boxes (exact, then as floats), points, sides
        let at_points = 96 * nt;
        let at_sides = at_points + 24 * n;
        let bytes = at_sides + 4 * n;
        let host = self.reserve_host(&mut s.host_in, bytes)?;
        for (k, b) in boxes.iter().enumerate() {
            for (j, v) in [b.x.lo, b.y.lo, b.z.lo, b.x.hi, b.y.hi, b.z.hi]
                .iter()
                .enumerate()
            {
                let o = 48 * k + 8 * j;
                host[o..o + 8].copy_from_slice(&v.to_ne_bytes());
            }
            for (j, v) in float_box(b).iter().enumerate() {
                let o = 48 * nt + 48 * k + 4 * j;
                host[o..o + 4].copy_from_slice(&v.to_ne_bytes());
            }
        }
        for (i, p) in points.iter().enumerate() {
            for (j, v) in [p.x, p.y, p.z].iter().enumerate() {
                let o = at_points + 24 * i + 8 * j;
                host[o..o + 8].copy_from_slice(&v.to_ne_bytes());
            }
        }
        for (i, v) in sides.iter().enumerate() {
            let o = at_sides + 4 * i;
            host[o..o + 4].copy_from_slice(&v.to_ne_bytes());
        }
        self.reserve(&mut s.input, bytes)?;
        self.reserve(&mut s.out, n)?;
        self.reserve_host(&mut s.host_out, n)?;
        ok(
            (self.api.to_device_async)(s.input.ptr, s.host_in.ptr.cast(), bytes, s.stream),
            "cuMemcpyHtoDAsync",
        )?;
        let lanes: u32 = (1..=5)
            .rev()
            .map(|k| 1 << k)
            .find(|&l| n * l as usize <= FILL)
            .unwrap_or(1);
        let mut a_tools32 = s.tools32.ptr;
        let mut a_segs32 = s.segs32.ptr;
        let mut a_tools = s.tools.ptr;
        let mut a_segs = s.segs.ptr;
        let mut a_ntools = nt as c_int;
        let mut a_boxes = s.input.ptr;
        let mut a_points = s.input.ptr + at_points as u64;
        let mut a_sides = s.input.ptr + at_sides as u64;
        let mut a_n = n as c_int;
        let mut a_lanes = lanes as c_int;
        let mut a_margin32 = MARGIN32;
        let mut a_margin = MARGIN;
        let mut a_graze = GRAZE;
        let mut a_out = s.out.ptr;
        let mut params: [*mut c_void; 14] = [
            (&raw mut a_tools32).cast(),
            (&raw mut a_segs32).cast(),
            (&raw mut a_tools).cast(),
            (&raw mut a_segs).cast(),
            (&raw mut a_ntools).cast(),
            (&raw mut a_boxes).cast(),
            (&raw mut a_points).cast(),
            (&raw mut a_sides).cast(),
            (&raw mut a_n).cast(),
            (&raw mut a_lanes).cast(),
            (&raw mut a_margin32).cast(),
            (&raw mut a_margin).cast(),
            (&raw mut a_graze).cast(),
            (&raw mut a_out).cast(),
        ];
        self.launch(s, self.verdicts, (n as u32 * lanes).div_ceil(BLOCK), &mut params)?;
        ok(
            (self.api.to_host_async)(s.host_out.ptr.cast(), s.out.ptr, n, s.stream),
            "cuMemcpyDtoHAsync",
        )?;
        ok((self.api.stream_sync)(s.stream), "cuStreamSynchronize")?;
        Ok(self.reserve_host(&mut s.host_out, n)?[..n].to_vec())
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        let s = self.state.get_mut().expect("gpu state");
        for b in [&s.tools, &s.segs, &s.tools32, &s.segs32, &s.input, &s.out] {
            if b.ptr != 0 {
                (self.api.free)(b.ptr);
            }
        }
        for h in [&s.host_in, &s.host_out] {
            if !h.ptr.is_null() {
                (self.api.free_host)(h.ptr.cast());
            }
        }
    }
}

/// A box as floats rounded outward (lo xyz, hi xyz), then inward: a float point outside the
/// first is outside the box, one inside the second inside it.
fn float_box(b: &Box3) -> [f32; 12] {
    // a few float steps each way cover rounding the box and the point
    let down = |v: f64| (v as f32).next_down().next_down().next_down();
    let up = |v: f64| (v as f32).next_up().next_up().next_up();
    [
        down(b.x.lo),
        down(b.y.lo),
        down(b.z.lo),
        up(b.x.hi),
        up(b.y.hi),
        up(b.z.hi),
        up(b.x.lo),
        up(b.y.lo),
        up(b.z.lo),
        down(b.x.hi),
        down(b.y.hi),
        down(b.z.hi),
    ]
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// One plain solid as the kernel reads it, combined by op code `op` (0 union, 1 subtract, 2
/// intersect), its profile appended to `segs`.
fn plain(sh: &Shape, op: f64, segs: &mut Vec<f64>) -> [f64; TOOL] {
    let mut t = [0.0f64; TOOL];
    t[1] = op;
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
        Shape::Coil {
            frame,
            segs: ss,
            a,
            b,
            angle,
            r,
            h,
        } => {
            t[0] = 4.0;
            put_segs(&mut t, ss);
            put_frame(&mut t, frame);
            t[16..23].copy_from_slice(&[*a, *b, *angle, r.0, r.1, h.0, h.1]);
        }
        Shape::Brep(b) => {
            t[0] = 5.0;
            t[16..22].copy_from_slice(&[b.x.lo, b.y.lo, b.z.lo, b.x.hi, b.y.hi, b.z.hi]);
        }
        Shape::Csg { .. } => unreachable!("a combination's parts are plain"),
    }
    t
}

/// Tools packed as the kernel reads them. Each packing has its own id, so the GPU knows when
/// the tools on it are already these.
#[derive(Clone)]
pub struct Packed {
    tools: Vec<f64>,
    segs: Vec<f64>,
    /// Tools (their parts follow them in `tools`).
    n: usize,
    id: u64,
}

impl Packed {
    pub fn new(shapes: &[Shape], ops: &[Op]) -> Packed {
        let code = |op: Option<&Op>| match op {
            None | Some(Op::Union) => 0.0,
            Some(Op::Subtract) => 1.0,
            _ => 2.0,
        };
        let mut tools = Vec::with_capacity(shapes.len() * TOOL);
        let mut segs: Vec<f64> = Vec::new();
        // a combination's parts, packed after every tool
        let mut parts: Vec<[f64; TOOL]> = Vec::new();
        for (k, sh) in shapes.iter().enumerate() {
            let op = code(if k == 0 { None } else { ops.get(k - 1) });
            let t = match sh {
                Shape::Csg { first, rest } => {
                    let mut t = [0.0f64; TOOL];
                    t[0] = 3.0;
                    t[1] = op;
                    t[2] = (shapes.len() + parts.len()) as f64;
                    t[3] = (1 + rest.len()) as f64;
                    parts.push(plain(first, 0.0, &mut segs));
                    for (o, p) in rest {
                        parts.push(plain(p, code(Some(o)), &mut segs));
                    }
                    t
                }
                _ => plain(sh, op, &mut segs),
            };
            tools.extend_from_slice(&t);
        }
        for p in parts {
            tools.extend_from_slice(&p);
        }
        Packed {
            tools,
            segs,
            n: shapes.len(),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

/// What classifies a regen's region batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend {
    /// The GPU where it measurably beats the CPU (see `crossover`), else the CPU: the CPU too
    /// until the GPU is open, which it starts once the CPU time it could have saved reaches the
    /// cost of opening it (or at `open_in_background`).
    #[default]
    Auto,
    Cpu,
    /// The GPU for every batch; the CPU if there is no GPU.
    Gpu,
}

impl Backend {
    /// From FORGE_BATCH (auto, cpu or gpu), read once; Auto when unset.
    pub fn from_env() -> Backend {
        static ENV: OnceLock<Backend> = OnceLock::new();
        *ENV.get_or_init(|| match std::env::var("FORGE_BATCH").as_deref() {
            Err(_) | Ok("auto") => Backend::Auto,
            Ok("cpu") => Backend::Cpu,
            Ok("gpu") => Backend::Gpu,
            Ok(v) => panic!("FORGE_BATCH={v}: expected auto, cpu or gpu"),
        })
    }
}

static GPU: OnceLock<Option<Gpu>> = OnceLock::new();

/// The process's GPU, opened at first use (~0.3 s); None where there is none.
pub fn gpu() -> Option<&'static Gpu> {
    GPU.get_or_init(|| {
        #[cfg(not(target_arch = "wasm32"))]
        // SAFETY: registers a plain extern "C" fn
        unsafe {
            atexit(hand_off);
        }
        Gpu::new().ok()
    })
    .as_ref()
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" {
    fn atexit(f: extern "C" fn()) -> c_int;
    fn fork() -> c_int;
    fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn _exit(code: c_int) -> !;
}

/// At exit, hands the driver's files to a child that holds them until this process is gone and
/// then exits: the kernel's teardown of the GPU context on their release (~0.1 s) happens in
/// the child, after the process's exit, not before it. The child does nothing but wait.
#[cfg(not(target_arch = "wasm32"))]
extern "C" fn hand_off() {
    let Ok(dir) = std::fs::read_dir("/proc/self/fd") else {
        return;
    };
    let mut keep = Vec::new();
    let mut top = 0;
    for e in dir.flatten() {
        let Some(fd) = e.file_name().to_str().and_then(|n| n.parse::<c_int>().ok()) else {
            continue;
        };
        top = top.max(fd);
        if std::fs::read_link(e.path())
            .is_ok_and(|t| t.as_os_str().as_encoded_bytes().starts_with(b"/dev/nvidia"))
        {
            keep.push(fd);
        }
    }
    if keep.is_empty() {
        return;
    }
    let mut ends = [0 as c_int; 2];
    // SAFETY: pipe2, fork, close, read and _exit are async-signal-safe, the only calls a child
    // of a threaded process may make; the child touches only `keep` and `ends`, copied with it
    unsafe {
        // O_CLOEXEC: nothing this process still starts holds the write end
        if pipe2(ends.as_mut_ptr(), 0o2000000) != 0 {
            return;
        }
        match fork() {
            0 => {
                // everything but the driver's files, so no reader waits on this child
                for fd in 0..=top.max(ends[1]) {
                    if fd != ends[0] && !keep.contains(&fd) {
                        close(fd);
                    }
                }
                // the write end closes when this process is gone
                let mut b = 0u8;
                read(ends[0], (&raw mut b).cast(), 1);
                _exit(0);
            }
            _ => {
                close(ends[0]);
            }
        }
    }
}

/// Starts opening the process's GPU on another thread, if it is not open or opening already. A
/// long-running app calls this at startup; Backend::Auto otherwise opens it once it pays.
pub fn open_in_background() {
    static OPEN: Once = Once::new();
    if !cfg!(target_family = "wasm") {
        OPEN.call_once(|| {
            std::thread::spawn(gpu);
        });
    }
}

/// Nanoseconds this process spent on the CPU in batches the GPU could have taken: Auto opens the
/// GPU once they reach OPEN_COST (break-even: never more than twice the best choice's time).
static RENT: AtomicU64 = AtomicU64::new(0);
/// What opening the GPU costs a process (~0.3 s, and an exit that waits while it is in flight).
const OPEN_COST: u64 = 300_000_000;

static CROSSOVER: OnceLock<usize> = OnceLock::new();

/// The batch size from which the GPU beat the CPU in this process's probe (usize::MAX: never),
/// once a batch has been run with Backend::Auto on a GPU.
pub fn crossover() -> Option<usize> {
    CROSSOVER.get().copied()
}

/// The batch size from which the GPU beats `cpu`: both timed (median of 3) on a batch's points
/// repeated to sizes doubling from 32, the GPU woken first as in a regen. A few ms, once.
fn probe(
    gpu: &Gpu,
    packed: &Packed,
    cpu: &CpuBatch,
    points: &[Point3],
    sides: &[u32],
    boxes: &[Box3],
) -> usize {
    let median = |f: &mut dyn FnMut()| {
        let mut t: Vec<Duration> = (0..3)
            .map(|_| {
                let s = Instant::now();
                f();
                s.elapsed()
            })
            .collect();
        t.sort();
        t[1]
    };
    // puts the tools on the device
    if gpu.run(packed, &points[..1], &sides[..1], boxes).is_err() {
        return usize::MAX;
    }
    let mut n = 32;
    while n <= 1 << 14 {
        let p: Vec<Point3> = (0..n).map(|i| points[i % points.len()]).collect();
        let s: Vec<u32> = (0..n).map(|i| sides[i % sides.len()]).collect();
        let c = median(&mut || drop(cpu.verdicts(&p, &s, boxes)));
        let g = median(&mut || {
            gpu.wake();
            let _ = gpu.run(packed, &p, &s, boxes);
        });
        if g < c {
            return n;
        }
        n *= 2;
    }
    usize::MAX
}

/// The CPU batch, or the GPU's where the backend picks it. A GPU error falls back to the CPU.
pub struct AutoBatch<'a> {
    pub cpu: CpuBatch<'a>,
    gpu: Option<(&'static Gpu, Packed)>,
    backend: Backend,
}

impl<'a> AutoBatch<'a> {
    pub fn new(cpu: CpuBatch<'a>, backend: Backend) -> AutoBatch<'a> {
        // Auto runs on the CPU while the GPU opens
        let gpu = match backend {
            Backend::Cpu => None,
            Backend::Auto => GPU.get().and_then(Option::as_ref),
            Backend::Gpu => gpu(),
        };
        if backend == Backend::Gpu && gpu.is_none() {
            static SAID: Once = Once::new();
            SAID.call_once(|| eprintln!("FORGE_BATCH=gpu: no CUDA GPU here, the CPU classifies"));
        }
        let gpu = gpu.map(|g| (g, Packed::new(cpu.shapes, cpu.ops)));
        AutoBatch { cpu, gpu, backend }
    }
}

impl Batch for AutoBatch<'_> {
    fn verdicts(&self, points: &[Point3], sides: &[u32], boxes: &[Box3]) -> Vec<u8> {
        if let Some((gpu, packed)) = &self.gpu
            && !points.is_empty()
            && (self.backend == Backend::Gpu
                || points.len()
                    >= *CROSSOVER.get_or_init(|| probe(gpu, packed, &self.cpu, points, sides, boxes)))
        {
            let mut out = Vec::with_capacity(points.len());
            for (p, s) in points.chunks(CHUNK).zip(sides.chunks(CHUNK)) {
                match gpu.run(packed, p, s, boxes) {
                    Ok(v) => out.extend(v),
                    Err(_) => return self.cpu.verdicts(points, sides, boxes),
                }
            }
            return out;
        }
        if self.backend == Backend::Auto && points.len() >= crossover().unwrap_or(128) {
            let t = Instant::now();
            let v = self.cpu.verdicts(points, sides, boxes);
            if RENT.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed) >= OPEN_COST {
                open_in_background();
            }
            return v;
        }
        self.cpu.verdicts(points, sides, boxes)
    }

    fn warm(&self) {
        if let Some((gpu, _)) = &self.gpu {
            gpu.wake();
        }
    }
}
