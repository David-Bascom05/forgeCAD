// Region verdicts for the n-ary boolean: forge-bool's CpuBatch, laid out for a GPU. Each point
// finds which tools hold it (from their analytic solids: shape.rs) and folds the CSG
// expression just inside and just outside its own tool. See the layout in lib.rs.
//
// tools: TOOL values each (combinations' parts after all tools): kind (3: a combination of
//        nseg parts from record seg0), op, seg0, nseg, frame origin xyz, x xyz, y xyz, z xyz,
//        a, b (prism lo hi / revolve angle (<0 whole) / ball radius), origin2 xy, dir2 xy,
//        perp2 xy
// segs:  SEG values each: kind (0 line, 1 arc), then a.x a.y b.x b.y | c.x c.y r start sweep
// boxes: 6 doubles per tool (lo xyz, hi xyz), then 12 floats per tool: the box rounded
//        outward and inward. Single precision culls by those and leaves a point between them
//        undecided; double precision tests the exact box: a point is in a box exactly when the
//        CPU finds it so.
// out:   0 dropped, 1 kept, 2 kept flipped, 3 undecided (near some tool's boundary)

#define TOOL 24
#define SEG 6
#define TAU 6.283185307179586
#define HALF_PI 1.5707963267948966

template <typename T> struct Prof { int ok; int in; T dist; };

template <typename T> __device__ bool on_sweep(T a, T start, T sweep) {
    if (fabs(sweep) >= ((T)TAU) - (T)1e-12) return true;
    T lo = sweep >= 0 ? start : start + sweep;
    T w = fabs(sweep);
    T d = fmod(a - lo, (T)((T)TAU));
    if (d < (T)0) d += ((T)TAU);
    return d <= w;
}

template <typename T> __device__ Prof<T> in_profile(const T* segs, int s0, int n, T qx, T qy, T graze) {
    Prof<T> r; r.ok = 1; r.in = 0; r.dist = (T)1e30;
    for (int i = 0; i < n; i++) {
        const T* s = segs + (size_t)(s0 + i) * SEG;
        if (s[0] == (T)0) {
            T ax = s[1], ay = s[2], bx = s[3], by = s[4];
            T ex = bx - ax, ey = by - ay;
            T t = ((qx - ax) * ex + (qy - ay) * ey) / (ex * ex + ey * ey);
            t = fmin(fmax(t, (T)0), (T)1);
            T dx = qx - (ax + ex * t), dy = qy - (ay + ey * t);
            r.dist = fmin(r.dist, sqrt(dx * dx + dy * dy));
            if ((ay > qy) != (by > qy) && qx < ax + (qy - ay) / (by - ay) * (bx - ax)) r.in = !r.in;
        } else {
            T cx = s[1], cy = s[2], rad = s[3], start = s[4], sweep = s[5];
            T dx = qx - cx, dy = qy - cy;
            T len = sqrt(dx * dx + dy * dy);
            T at = atan2(dy, dx);
            if (on_sweep(at, start, sweep)) {
                r.dist = fmin(r.dist, fabs(len - rad));
            } else {
                T p0x = cx + cos(start) * rad, p0y = cy + sin(start) * rad;
                T p1x = cx + cos(start + sweep) * rad, p1y = cy + sin(start + sweep) * rad;
                T d0 = sqrt((qx - p0x) * (qx - p0x) + (qy - p0y) * (qy - p0y));
                T d1 = sqrt((qx - p1x) * (qx - p1x) + (qy - p1y) * (qy - p1y));
                r.dist = fmin(r.dist, fmin(d0, d1));
            }
            if (fabs(dy) >= rad - graze) {
                if (fabs(dy) <= rad + graze) { r.ok = 0; return r; }
                continue;
            }
            if (fabs(sweep) < ((T)TAU) - (T)1e-12) {
                if (fabs(cy + rad * sin(start) - qy) <= graze || fabs(cy + rad * sin(start + sweep) - qy) <= graze) {
                    r.ok = 0; return r;
                }
            }
            T hx = sqrt(rad * rad - dy * dy);
            T xs[2] = {cx - hx, cx + hx};
            for (int k = 0; k < 2; k++) {
                if (xs[k] > qx && on_sweep(atan2(dy, xs[k] - cx), start, sweep)) r.in = !r.in;
            }
        }
    }
    return r;
}

// 0 out, 1 in, 2 near
template <typename T> __device__ int combine(Prof<T> p, T z, T lo, T hi, T m) {
    if (!p.ok) return 2;
    if (z < lo - m || z > hi + m || (!p.in && p.dist > m)) return 0;
    T dz = fmin(z - lo, hi - z);
    return (p.in && p.dist > m && dz > m) ? 1 : 2;
}

template <typename T> __device__ int member_plain(const T* t, const T* segs, T px, T py, T pz, T m, T graze) {
    int kind = (int)t[0];
    if (kind == 2) {
        T dx = px - t[4], dy = py - t[5], dz = pz - t[6];
        T d = sqrt(dx * dx + dy * dy + dz * dz);
        T rad = t[16];
        return d < rad - m ? 1 : (d > rad + m ? 0 : 2);
    }
    T vx = px - t[4], vy = py - t[5], vz = pz - t[6];
    T lx = vx * t[7] + vy * t[8] + vz * t[9];
    T ly = vx * t[10] + vy * t[11] + vz * t[12];
    T lz = vx * t[13] + vy * t[14] + vz * t[15];
    int s0 = (int)t[2], n = (int)t[3];
    if (kind == 0) {
        T lo = t[16], hi = t[17];
        if (lz < lo - m || lz > hi + m) return 0;
        return combine(in_profile(segs, s0, n, lx, ly, graze), lz, lo, hi, m);
    }
    T r = sqrt(lx * lx + ly * ly);
    T qx = t[18] + t[20] * lz + t[22] * r;
    T qy = t[19] + t[21] * lz + t[23] * r;
    Prof<T> p = in_profile(segs, s0, n, qx, qy, graze);
    T mm = fmin(m, (T)0.5);
    int c = combine(p, (T)0, (T)-1, (T)1, mm);
    T angle = t[16];
    if (angle < (T)0) return c;
    T a = fmod(atan2(ly, lx), ((T)TAU));
    if (a < (T)0) a += ((T)TAU);
    int in_sector = a <= angle;
    T d0, d1;
    if (in_sector) {
        d0 = fabs(a); d1 = fabs(a - angle);
    } else {
        d0 = fmin(fabs(a), ((T)TAU) - fabs(a));
        d1 = fmin(fabs(a - angle), ((T)TAU) - fabs(a - angle));
    }
    T s0d = d0 >= ((T)HALF_PI) ? r : r * sin(d0);
    T s1d = d1 >= ((T)HALF_PI) ? r : r * sin(d1);
    T sd = fmin(s0d, s1d);
    if (c == 0) return 0;
    if (!in_sector && sd > m) return 0;
    if (c == 1 && in_sector && sd > m) return 1;
    return 2;
}

// a tool's verdict: a plain solid's, or a combination's (kind 3: parts from t[2], t[3] of them,
// each folded in by its op), from its parts' (shape.rs Csg)
template <typename T>
__device__ int member(const T* tools, const T* t, const T* segs, T px, T py, T pz, T m, T graze) {
    // one call site, so the plain solid's test is inlined once
    int n = 1;
    if ((int)t[0] == 3) {
        n = (int)t[3];
        t = tools + (size_t)t[2] * TOOL;
    }
    int v = 0;
    for (int i = 0; i < n; i++) {
        const T* p = t + (size_t)i * TOOL;
        int op = i == 0 ? -1 : (int)p[1];
        if (v == 0 && op > 0) continue;
        int w = member_plain(p, segs, px, py, pz, m, graze);
        if (op == -1) v = w;
        else if (op == 0) v = (v == 1 || w == 1) ? 1 : ((v == 0 && w == 0) ? 0 : 2);
        else if (op == 1) v = (v == 0 || w == 1) ? 0 : ((v == 1 && w == 0) ? 1 : 2);
        else v = (v == 0 || w == 0) ? 0 : ((v == 1 && w == 1) ? 1 : 2);
    }
    return v;
}

// whether a point is in a tool's box (6 doubles, then 12 floats from `fast`): 0 out, 1 in, 2
// too near the box's faces for floats to tell (single precision then leaves the point
// undecided: it does no double arithmetic at all)
template <typename T>
__device__ int in_box(const double* b, const float* f, double px, double py, double pz, T qx, T qy, T qz) {
    if constexpr (sizeof(T) == 4) {
        if (qx < f[0] || qy < f[1] || qz < f[2] || qx > f[3] || qy > f[4] || qz > f[5]) return 0;
        return qx > f[6] && qy > f[7] && qz > f[8] && qx < f[9] && qy < f[10] && qz < f[11] ? 1 : 2;
    } else {
        return !(px < b[0] || py < b[1] || pz < b[2] || px > b[3] || py > b[4] || pz > b[5]);
    }
}

// a point's verdict against every tool, its own tool `own` taken as just entered (CpuBatch's
// verdict), worked by a group of `lanes` threads (1, 2, 4 .. 32, aligned in a warp) each taking
// every lanes-th tool. Each tool after the first either leaves the folded expression as it is or
// sets it (a union holding the point sets it true, a subtract holding it false, an intersect
// not holding it false), so the fold is the value set by the highest tool that sets one: a max
// across the lanes. Every lane of the warp must call this together (an `active` 0 lane takes
// no tools), as the lanes trade results.
template <typename T>
__device__ int verdict(const T* tools, int ntools, const T* segs, const double* boxes, double px, double py,
                       double pz, int own, T margin, T graze, int lane, int lanes, int active) {
    T qx = (T)px, qy = (T)py, qz = (T)pz;
    const float* fast = (const float*)(boxes + (size_t)ntools * 6);
    // highest tool setting the value just inside (kin) and just outside (kout) the own tool,
    // times two, plus the value
    int kin = -1, kout = -1, near = 0;
    for (int k = lane; active && k < ntools; k += lanes) {
        const T* t = tools + (size_t)k * TOOL;
        int bin, bout, inb;
        if (k == own) {
            bin = 1; bout = 0;
        } else if ((inb = in_box<T>(boxes + (size_t)k * 6, fast + (size_t)k * 12, px, py, pz, qx, qy, qz)) == 0) {
            bin = 0; bout = 0;
        } else if (inb == 2) {
            near = 1;
            break;
        } else {
            int mb = member(tools, t, segs, qx, qy, qz, margin, graze);
            if (mb == 2) {
                near = 1;
                break;
            }
            bin = mb; bout = mb;
        }
        int op = k == 0 ? -1 : (int)t[1];
        if (op == -1) { kin = bin; kout = bout; }
        else if (op == 0) { if (bin) kin = 2 * k + 1; if (bout) kout = 2 * k + 1; }
        else if (op == 1) { if (bin) kin = 2 * k; if (bout) kout = 2 * k; }
        else { if (!bin) kin = 2 * k; if (!bout) kout = 2 * k; }
    }
    for (int o = lanes / 2; o > 0; o >>= 1) {
        near = max(near, __shfl_xor_sync(0xffffffffu, near, o));
        kin = max(kin, __shfl_xor_sync(0xffffffffu, kin, o));
        kout = max(kout, __shfl_xor_sync(0xffffffffu, kout, o));
    }
    if (near) return 3;
    int vin = kin & 1, vout = kout & 1;
    return vin == vout ? 0 : (vin ? 1 : 2);
}

// Points 0..n, a group of `lanes` threads each: single precision answers the points clear of
// every boundary by `margin32`; double precision, in the same group, takes the rest.
extern "C" __global__ void verdicts(const float* tools32, const float* segs32, const double* tools,
                                    const double* segs, int ntools, const double* boxes, const double* points,
                                    const int* sides, int n, int lanes, float margin32, double margin,
                                    double graze, unsigned char* out) {
    int at = blockIdx.x * blockDim.x + threadIdx.x;
    int i = at / lanes, lane = at % lanes;
    // lanes past the end still take part in their warp's exchanges
    int active = i < n;
    if (!active) i = 0;
    double px = points[3 * i], py = points[3 * i + 1], pz = points[3 * i + 2];
    int v = verdict<float>(tools32, ntools, segs32, boxes, px, py, pz, sides[i], margin32, 1e-4f, lane, lanes,
                           active);
    int again = active && v == 3;
    if (__any_sync(0xffffffffu, again)) {
        int w = verdict<double>(tools, ntools, segs, boxes, px, py, pz, sides[i], margin, graze, lane, lanes, again);
        if (again) v = w;
    }
    if (active && lane == 0) out[i] = (unsigned char)v;
}

// does nothing: launched ahead of a batch so the GPU is awake (clocks up) when it comes
extern "C" __global__ void wake() {}
