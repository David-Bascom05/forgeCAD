//! The global allocator on single-threaded wasm (linked in through forge-math). A two-level
//! segregated fit heap: every block has a header (the previous block's start and its size), a
//! freed block merges with free neighbours, and free blocks sit on lists by size, four per
//! octave, found through a bitmap. Requests up to 1 KiB are rounded to size classes, carved off a
//! 64 KiB block in turn, and their freed blocks kept per class for reuse as they are, up to 64 KiB
//! in all; past that, and before memory grows, they go back to the heap and merge.
// elsewhere only the tests use the heap
#![cfg_attr(not(target_family = "wasm"), allow(dead_code))]

const W: usize = core::mem::size_of::<usize>();
/// Block header: the previous block's start (0 for a region's first) and the size, | FREE.
const H: usize = 2 * W;
const FREE: usize = 1;
/// Smallest block: a header and a free block's two links.
const MIN: usize = (H + 2 * W).next_multiple_of(16);
/// Largest request.
const MAX: usize = 1 << 30;
/// Largest request kept per size class.
const SMALL: usize = 1024;
const LISTS: usize = 16 + 4 * 24;
const CACHES: usize = 20;
/// Most bytes held in the caches: past it, freed small blocks merge back at once.
const CAP: usize = 64 << 10;
/// Small blocks are carved off a used block of this size, a cheap bump.
const WILD: usize = 64 << 10;

fn rd(a: usize) -> usize {
    unsafe { *(a as *const usize) }
}

fn wr(a: usize, v: usize) {
    unsafe { *(a as *mut usize) = v }
}

/// The class of a small request and its size: 16 to 128 bytes in steps of 16, then four per
/// octave.
fn class(size: usize) -> (usize, usize) {
    let s = size.max(1);
    if s <= 128 {
        let c = s.div_ceil(16);
        return (c - 1, c * 16);
    }
    let b = (usize::BITS - 1 - (s - 1).leading_zeros()) as usize;
    let sub = ((s - 1) >> (b - 2)) & 3;
    (8 + (b - 7) * 4 + sub, (5 + sub) << (b - 2))
}

/// The list a free block of `size` (a multiple of 16) goes on.
fn index(size: usize) -> usize {
    if size < 256 {
        return size / 16;
    }
    let b = (usize::BITS - 1 - size.leading_zeros()) as usize;
    16 + (b - 8) * 4 + ((size >> (b - 2)) & 3)
}

/// The first list whose every block holds `size`.
fn index_up(size: usize) -> usize {
    if size < 256 {
        return size / 16;
    }
    let b = (usize::BITS - 1 - size.leading_zeros()) as usize;
    index(size + (1 << (b - 2)) - 1)
}

/// The block size for a payload.
fn block(payload: usize) -> usize {
    (payload + H).next_multiple_of(16).max(MIN)
}

/// Memory comes from `grow(bytes)`: a new region of at least that many bytes, as (start, length).
pub struct Heap {
    lists: [usize; LISTS],
    bits: u128,
    cache: [usize; CACHES],
    cached: usize,
    /// The block small blocks are carved from (used, so its neighbours don't merge into it).
    wild: usize,
    /// The last region's end sentinel (a used block of size 0), and where the region stops.
    end: usize,
    top: usize,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub const fn new() -> Heap {
        Heap {
            lists: [0; LISTS],
            bits: 0,
            cache: [0; CACHES],
            cached: 0,
            wild: 0,
            end: 0,
            top: 0,
        }
    }

    fn size(b: usize) -> usize {
        rd(b + W) & !FREE
    }

    fn is_free(b: usize) -> bool {
        rd(b + W) & FREE != 0
    }

    fn insert(&mut self, b: usize, size: usize) {
        let i = index(size);
        let next = self.lists[i];
        wr(b + W, size | FREE);
        wr(b + H, next);
        wr(b + H + W, 0);
        if next != 0 {
            wr(next + H + W, b);
        }
        self.lists[i] = b;
        self.bits |= 1 << i;
    }

    fn remove(&mut self, b: usize) {
        let i = index(Self::size(b));
        let (next, prev) = (rd(b + H), rd(b + H + W));
        if prev != 0 {
            wr(prev + H, next);
        } else {
            self.lists[i] = next;
            if next == 0 {
                self.bits &= !(1 << i);
            }
        }
        if next != 0 {
            wr(next + H + W, prev);
        }
    }

    /// Frees block b, merged with its free neighbours.
    fn release(&mut self, mut b: usize) {
        let mut size = Self::size(b);
        let p = rd(b);
        if p != 0 && Self::is_free(p) {
            self.remove(p);
            size += Self::size(p);
            b = p;
        }
        let n = b + size;
        if Self::is_free(n) {
            self.remove(n);
            size += Self::size(n);
        }
        self.insert(b, size);
        wr(b + size, b);
    }

    /// Cuts used block b (of `have` bytes) to `need`, freeing the rest when it makes a block.
    fn trim(&mut self, b: usize, have: usize, need: usize) {
        if have - need >= MIN {
            wr(b + W, need);
            let r = b + need;
            wr(r, b);
            wr(r + W, have - need);
            self.release(r);
        } else {
            wr(b + W, have);
        }
    }

    /// A block of `need` off the lists: the first list whose blocks all hold it, or else (when
    /// `search`) any block that does on the list below.
    fn fit(&mut self, need: usize, search: bool) -> usize {
        let i = index_up(need);
        let m = if i < LISTS { self.bits & (!0 << i) } else { 0 };
        let mut b = if m == 0 {
            0
        } else {
            self.lists[m.trailing_zeros() as usize]
        };
        if b == 0 && search {
            b = self.lists[index(need)];
            while b != 0 && Self::size(b) < need {
                b = rd(b + H);
            }
        }
        if b == 0 {
            return 0;
        }
        self.remove(b);
        self.trim(b, Self::size(b), need);
        b + H
    }

    /// Every cached small block back to the heap.
    fn flush(&mut self) {
        self.cached = 0;
        let w = core::mem::take(&mut self.wild);
        if w != 0 {
            self.release(w);
        }
        for c in 0..CACHES {
            let mut p = core::mem::take(&mut self.cache[c]);
            while p != 0 {
                let next = rd(p);
                self.release(p - H);
                p = next;
            }
        }
    }

    /// Adds a region of memory: continuing the last one when it starts where that stopped.
    fn add(&mut self, start: usize, len: usize) {
        let stop = start + len;
        let b = if start == self.top && self.end != 0 {
            self.end
        } else {
            let b = (start + H).next_multiple_of(16) - H;
            wr(b, 0);
            b
        };
        let end = stop - H - (stop - H - b) % 16;
        wr(b + W, end - b);
        wr(end, b);
        wr(end + W, 0);
        (self.end, self.top) = (end, stop);
        self.release(b);
    }

    fn alloc_block(
        &mut self,
        payload: usize,
        grow: &mut impl FnMut(usize) -> Option<(usize, usize)>,
    ) -> usize {
        let need = block(payload);
        let p = self.fit(need, false);
        if p != 0 {
            return p;
        }
        // before growing: the cached blocks merged back, and a look through the nearest list
        self.flush();
        let p = self.fit(need, true);
        if p != 0 {
            return p;
        }
        match grow(need + 16 + 2 * H) {
            Some((start, len)) => {
                self.add(start, len);
                self.fit(need, true)
            }
            None => 0,
        }
    }

    /// A block of `need` bytes off the front of the wild block, taking a new one when it runs out.
    fn carve(&mut self, need: usize, grow: &mut impl FnMut(usize) -> Option<(usize, usize)>) -> usize {
        let w = self.wild;
        if w != 0 {
            let have = Self::size(w);
            if have >= need + MIN {
                let r = w + need;
                wr(w + W, need);
                wr(r, w);
                wr(r + W, have - need);
                wr(r + have - need, r);
                self.wild = r;
                return w + H;
            }
            self.wild = 0;
            self.release(w);
        }
        let p = self.alloc_block(WILD - H, grow);
        if p == 0 {
            return self.alloc_block(need - H, grow);
        }
        self.wild = p - H;
        self.carve(need, grow)
    }

    /// # Safety
    /// The regions `grow` returns are this heap's alone.
    pub unsafe fn alloc(
        &mut self,
        size: usize,
        align: usize,
        grow: &mut impl FnMut(usize) -> Option<(usize, usize)>,
    ) -> *mut u8 {
        if size > MAX {
            return core::ptr::null_mut();
        }
        let p = if align > 16 {
            // over-aligned blocks (none in the kernel) are never freed
            let p = self.alloc_block(size + align, grow);
            if p == 0 { 0 } else { p.next_multiple_of(align) }
        } else if size <= SMALL {
            let (c, s) = class(size);
            let p = self.cache[c];
            if p != 0 {
                self.cache[c] = rd(p);
                self.cached -= s;
                p
            } else {
                self.carve(block(s), grow)
            }
        } else {
            self.alloc_block(size, grow)
        };
        p as *mut u8
    }

    /// # Safety
    /// `p` came from this heap's `alloc` or `realloc` with this size and alignment.
    pub unsafe fn dealloc(&mut self, p: *mut u8, size: usize, align: usize) {
        let p = p as usize;
        let (c, s) = class(size);
        if align <= 16 && size <= SMALL && self.cached + s <= CAP {
            wr(p, self.cache[c]);
            self.cache[c] = p;
            self.cached += s;
        } else if align <= 16 {
            self.release(p - H);
        }
    }

    /// # Safety
    /// As `dealloc`.
    pub unsafe fn realloc(
        &mut self,
        p: *mut u8,
        size: usize,
        align: usize,
        new: usize,
        grow: &mut impl FnMut(usize) -> Option<(usize, usize)>,
    ) -> *mut u8 {
        if align <= 16 && new <= MAX {
            if size <= SMALL && new <= SMALL && class(size).0 == class(new).0 {
                return p;
            }
            if size > SMALL && new > SMALL {
                // in place: shrinking, or growing into a free block after it
                let b = p as usize - H;
                let (have, need) = (Self::size(b), block(new));
                if need <= have {
                    self.trim(b, have, need);
                    return p;
                }
                let n = b + have;
                if Self::is_free(n) && have + Self::size(n) >= need {
                    self.remove(n);
                    let all = have + Self::size(n);
                    wr(b + all, b);
                    self.trim(b, all, need);
                    return p;
                }
            }
        }
        let q = unsafe { self.alloc(new, align, grow) };
        if !q.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(p, q, size.min(new));
                self.dealloc(p, size, align);
            }
        }
        q
    }
}

#[cfg(all(target_family = "wasm", not(target_feature = "atomics")))]
mod global {
    use super::Heap;
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;

    struct Alloc(UnsafeCell<Heap>);

    // no atomics, so one thread
    unsafe impl Sync for Alloc {}

    #[global_allocator]
    static ALLOC: Alloc = Alloc(UnsafeCell::new(Heap::new()));

    fn grow(bytes: usize) -> Option<(usize, usize)> {
        const PAGE: usize = 65536;
        let pages = bytes.div_ceil(PAGE).max(16);
        let old = core::arch::wasm32::memory_grow(0, pages);
        (old != usize::MAX).then_some((old * PAGE, pages * PAGE))
    }

    unsafe impl GlobalAlloc for Alloc {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            unsafe { (*self.0.get()).alloc(l.size(), l.align(), &mut grow) }
        }

        // out of line: inlined at every drop it grows the module by a twentieth for 1%
        #[inline(never)]
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            unsafe { (*self.0.get()).dealloc(p, l.size(), l.align()) }
        }

        unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
            unsafe { (*self.0.get()).realloc(p, l.size(), l.align(), new, &mut grow) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn classes_cover_every_small_size() {
        let mut last = (0, 16);
        for s in 1..=SMALL {
            let (c, size) = class(s);
            assert!(size >= s && size % 16 == 0 && c < CACHES, "{s}");
            assert!(
                c == last.0 && size == last.1 || c == last.0 + 1 && size > last.1,
                "{s}"
            );
            last = (c, size);
        }
    }

    #[test]
    fn lists_hold_what_they_promise() {
        let mut s = MIN;
        while s <= 2 * MAX {
            let (i, up) = (index(s), index_up(s));
            assert!(i < LISTS && index(s + 16) >= i, "{s}");
            // every block on list `up` holds s, and `up` is at most one list too high
            assert!(index(s - 16) < up && up <= i + 1, "{s}");
            s += 16 + (s / 64).next_multiple_of(16);
        }
    }

    /// A heap over a fixed arena, handing out regions in order, with gaps when `gaps`.
    struct Arena {
        mem: Vec<u128>,
        used: usize,
        regions: Vec<(usize, usize)>,
        gaps: bool,
    }

    impl Arena {
        fn new(bytes: usize, gaps: bool) -> Arena {
            Arena {
                mem: vec![0; bytes / 16],
                used: 0,
                regions: Vec::new(),
                gaps,
            }
        }
        fn grow(&mut self, bytes: usize) -> Option<(usize, usize)> {
            let len = bytes.next_multiple_of(4096).max(16384) + if self.gaps { 8 } else { 0 };
            let start = self.used + if self.gaps { 24 } else { 0 };
            if start + len > self.mem.len() * 16 {
                return None;
            }
            self.used = start + len;
            let a = self.mem.as_mut_ptr() as usize + start;
            self.regions.push((a, len));
            Some((a, len))
        }
    }

    /// Every block of every region: headers chained, no two free side by side, the free ones
    /// exactly those on the lists (each on the right one). Returns the free blocks' sizes.
    fn check(h: &Heap, a: &Arena) -> Vec<usize> {
        let mut free = Vec::new();
        let mut starts = Vec::new();
        for (i, &(start, _)) in a.regions.iter().enumerate() {
            if i > 0 && start == a.regions[i - 1].0 + a.regions[i - 1].1 {
                continue;
            }
            starts.push((start + H).next_multiple_of(16) - H);
        }
        for s in starts {
            let (mut b, mut prev, mut prev_free) = (s, 0, false);
            assert_eq!(rd(b), 0);
            loop {
                assert_eq!(rd(b), prev, "prev link at {b:x}");
                let size = Heap::size(b);
                if size == 0 {
                    break;
                }
                assert!(size >= MIN && size.is_multiple_of(16) && (b + H).is_multiple_of(16));
                let f = Heap::is_free(b);
                assert!(!(f && prev_free), "two free blocks side by side");
                if f {
                    free.push((b, size));
                }
                (prev, prev_free, b) = (b, f, b + size);
            }
        }
        let mut listed = Vec::new();
        for i in 0..LISTS {
            assert_eq!(h.bits >> i & 1 == 1, h.lists[i] != 0, "bitmap {i}");
            let (mut b, mut back) = (h.lists[i], 0);
            while b != 0 {
                assert_eq!(index(Heap::size(b)), i);
                assert_eq!(rd(b + H + W), back);
                listed.push((b, Heap::size(b)));
                (back, b) = (b, rd(b + H));
            }
        }
        listed.sort();
        free.sort();
        assert_eq!(listed, free, "the lists hold exactly the free blocks");
        free.into_iter().map(|f| f.1).collect()
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        fn size(&mut self) -> usize {
            match self.below(10) {
                0..=5 => 1 + self.below(200) as usize,
                6 | 7 => 1 + self.below(2000) as usize,
                8 => 1 + self.below(70_000) as usize,
                _ => 1 + self.below(600_000) as usize,
            }
        }
    }

    fn fill(p: usize, n: usize, tag: u8) {
        unsafe { core::ptr::write_bytes(p as *mut u8, tag, n) }
    }

    fn holds(p: usize, n: usize, tag: u8) -> bool {
        unsafe { core::slice::from_raw_parts(p as *const u8, n) }
            .iter()
            .all(|&b| b == tag)
    }

    /// Random allocs, frees and reallocs against a model of the live blocks: aligned, disjoint,
    /// contents kept; with everything freed the heap is one free block per region.
    fn random(seed: u64, gaps: bool, aligned: bool) {
        let mut r = Rng(seed);
        let mut a = Arena::new(64 << 20, gaps);
        let ap = &raw mut a;
        let mut h = Heap::new();
        let mut live: BTreeMap<usize, (usize, usize, u8)> = BTreeMap::new();
        // the heap's structure is checked as it goes, while grow holds the arena
        let mut grow = |n: usize| unsafe { (*ap).grow(n) };
        for step in 0..20_000u32 {
            let tag = (step % 251) as u8 + 1;
            // live data held under 16 MiB
            let held: usize = live.values().map(|l| l.0).sum();
            let op = if held > 16 << 20 { 5 } else { r.below(10) };
            if op < 5 || live.is_empty() {
                let (mut size, mut align) = (r.size(), 1 << r.below(5));
                // over-aligned blocks are never freed: kept small
                if aligned && r.below(8) == 0 {
                    (size, align) = (size.min(2000), 32 << r.below(3));
                }
                let p = unsafe { h.alloc(size, align, &mut grow) } as usize;
                assert!(
                    p != 0 && p.is_multiple_of(align.max(16)),
                    "alloc {size} {align} holding {held}"
                );
                fill(p, size, tag);
                live.insert(p, (size, align, tag));
            } else {
                let k = r.below(live.len() as u64) as usize;
                let (&p, &(size, align, t)) = live.iter().nth(k).unwrap();
                assert!(holds(p, size, t), "contents of {p:x}");
                live.remove(&p);
                if op < 8 {
                    unsafe { h.dealloc(p as *mut u8, size, align) };
                } else {
                    let new = r.size();
                    let q = unsafe { h.realloc(p as *mut u8, size, align, new, &mut grow) } as usize;
                    assert!(q != 0 && q.is_multiple_of(align.max(16)));
                    assert!(holds(q, size.min(new), t), "realloc kept the contents");
                    fill(q, new, tag);
                    live.insert(q, (new, align, tag));
                }
            }
            if step % 1000 == 0 {
                check(&h, unsafe { &*ap });
            }
            // disjoint from its neighbours
            let mut end = 0;
            if step % 97 == 0 {
                for (&p, &(s, _, _)) in &live {
                    assert!(p >= end, "overlap at {p:x}");
                    end = p + s;
                }
            }
        }
        for (&p, &(size, align, t)) in &live {
            assert!(holds(p, size, t));
            unsafe { h.dealloc(p as *mut u8, size, align) };
        }
        h.flush();
        let free = check(&h, unsafe { &*ap });
        if !aligned {
            let regions = a
                .regions
                .iter()
                .enumerate()
                .filter(|&(i, &(s, _))| i == 0 || s != a.regions[i - 1].0 + a.regions[i - 1].1);
            assert_eq!(free.len(), regions.count(), "everything merged back");
        }
    }

    #[test]
    fn random_against_a_model() {
        for seed in 1..=6u64 {
            random(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15), seed % 2 == 0, false);
        }
        random(77, false, true);
    }

    #[test]
    fn growth_is_bounded_by_the_live_peak() {
        // the same churn repeated: memory barely grows after the first round
        let mut a = Arena::new(256 << 20, false);
        let base = a.mem.as_ptr() as usize;
        let mut h = Heap::new();
        let mut grow = |n: usize| a.grow(n);
        let mut tops = Vec::new();
        for _ in 0..40 {
            let mut r = Rng(5);
            let mut live = Vec::new();
            for _ in 0..3000 {
                let s = r.size();
                live.push((unsafe { h.alloc(s, 8, &mut grow) }, s));
                if r.below(3) == 0 {
                    let (p, s) = live.swap_remove(r.below(live.len() as u64) as usize);
                    unsafe { h.dealloc(p, s, 8) };
                }
            }
            for (p, s) in live {
                unsafe { h.dealloc(p, s, 8) };
            }
            tops.push(h.top - base);
        }
        // fragmentation costs a little over the rounds, not a round's worth
        assert!(tops[39] < tops[0] + tops[0] / 8, "{tops:?}");
    }
}
