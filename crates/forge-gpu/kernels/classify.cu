// Region verdicts for the n-ary boolean: each thread takes one region's point, finds which
// tools hold it (from their analytic solids), and folds the CSG expression just inside and
// just outside the region's own tool. Mirrors forge-bool's shape.rs; see layout in lib.rs.
//
// tools: TOOL doubles each: kind, op, seg0, nseg, frame origin xyz, x xyz, y xyz, z xyz,
//        a, b (prism lo hi / revolve angle (<0 whole) / ball radius), origin2 xy, dir2 xy,
//        perp2 xy, box lo xyz, box hi xyz
// segs:  SEG doubles each: kind (0 line, 1 arc), then a.x a.y b.x b.y | c.x c.y r start sweep
// out:   0 dropped, 1 kept, 2 kept flipped, 3 undecided (near some tool's boundary)

#define TOOL 32
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

template <typename T> __device__ int member(const T* t, const T* segs, T px, T py, T pz, T m, T graze) {
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

template <typename T>
__device__ void verdicts_t(const T* tools, int ntools, const T* segs, const T* points, const int* sides,
                           int npoints, T margin, T graze, unsigned char* out) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= npoints) return;
    T px = points[3 * i], py = points[3 * i + 1], pz = points[3 * i + 2];
    int own = sides[i];
    // fold the expression for the point just inside (vin) and just outside (vout) its tool
    int vin = 0, vout = 0;
    for (int k = 0; k < ntools; k++) {
        const T* t = tools + (size_t)k * TOOL;
        int bin, bout;
        if (k == own) {
            bin = 1; bout = 0;
        } else if (px < t[24] || py < t[25] || pz < t[26] || px > t[27] || py > t[28] || pz > t[29]) {
            bin = 0; bout = 0;
        } else {
            int mb = member(t, segs, px, py, pz, margin, graze);
            if (mb == 2) { out[i] = 3; return; }
            bin = mb; bout = mb;
        }
        if (k == 0) {
            vin = bin; vout = bout;
        } else {
            int op = (int)t[1];
            if (op == 0) { vin = vin || bin; vout = vout || bout; }
            else if (op == 1) { vin = vin && !bin; vout = vout && !bout; }
            else { vin = vin && bin; vout = vout && bout; }
        }
    }
    out[i] = vin == vout ? 0 : (vin ? 1 : 2);
}

extern "C" __global__ void verdicts(const double* tools, int ntools, const double* segs, const double* points,
                                    const int* sides, int npoints, int any_intersect, double margin,
                                    unsigned char* out) {
    (void)any_intersect;
    verdicts_t<double>(tools, ntools, segs, points, sides, npoints, margin, 1e-9, out);
}

// single precision: answers only where clear by `margin` (set well above float error by the
// caller); everything nearer comes back undecided
extern "C" __global__ void verdicts32(const float* tools, int ntools, const float* segs, const float* points,
                                      const int* sides, int npoints, int any_intersect, float margin,
                                      unsigned char* out) {
    (void)any_intersect;
    verdicts_t<float>(tools, ntools, segs, points, sides, npoints, margin, 1e-4f, out);
}
