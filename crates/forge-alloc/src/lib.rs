//! The global allocator on single-threaded wasm (linked in through forge-math). Blocks are
//! 16-byte aligned and come in size classes, 16 to 128 bytes in steps of 16, then four per
//! octave; a freed block goes on its class's list and is reused as is, and a realloc that stays
//! in its class keeps the block. Memory, as on any wasm heap, only grows.
// elsewhere only the tests use the classes
#![cfg_attr(not(target_family = "wasm"), allow(dead_code))]

/// Number of size classes: up to 2 GiB.
const CLASSES: usize = 8 + 4 * 24;
/// Largest block.
const MAX: usize = 1 << 31;

/// The class of a size and its block size.
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

#[cfg(all(target_family = "wasm", not(target_feature = "atomics")))]
mod heap {
    use super::{MAX, class};
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;
    use core::ptr::null_mut;

    const PAGE: usize = 65536;

    struct Heap {
        free: [usize; super::CLASSES],
        bump: usize,
        end: usize,
    }

    impl Heap {
        /// A new block off the bump region, growing memory when it runs out.
        #[cold]
        #[inline(never)]
        fn carve(&mut self, size: usize, align: usize) -> *mut u8 {
            loop {
                let a = (self.bump + align - 1) & !(align - 1);
                if let Some(e) = a.checked_add(size)
                    && e <= self.end
                {
                    self.bump = e;
                    return a as *mut u8;
                }
                let pages = (size + align).div_ceil(PAGE).max(16);
                let old = core::arch::wasm32::memory_grow(0, pages);
                if old == usize::MAX {
                    return null_mut();
                }
                if old * PAGE != self.end {
                    self.bump = old * PAGE;
                }
                self.end = (old + pages) * PAGE;
            }
        }
    }

    struct Alloc(UnsafeCell<Heap>);

    // no atomics, so one thread
    unsafe impl Sync for Alloc {}

    #[global_allocator]
    static ALLOC: Alloc = Alloc(UnsafeCell::new(Heap {
        free: [0; super::CLASSES],
        bump: 0,
        end: 0,
    }));

    unsafe impl GlobalAlloc for Alloc {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            let h = unsafe { &mut *self.0.get() };
            if l.size() > MAX {
                return null_mut();
            }
            let (c, size) = class(l.size());
            let align = l.align().max(16);
            if align == 16 && h.free[c] != 0 {
                let p = h.free[c];
                h.free[c] = unsafe { *(p as *const usize) };
                return p as *mut u8;
            }
            h.carve(size, align)
        }

        // over-aligned blocks (none in the kernel) are not reused; out of line, since inlined
        // at every drop it grows the module by a twentieth for a 1% gain
        #[inline(never)]
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            if l.align() <= 16 {
                let h = unsafe { &mut *self.0.get() };
                let c = class(l.size()).0;
                unsafe { *(p as *mut usize) = h.free[c] };
                h.free[c] = p as usize;
            }
        }

        unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
            if l.align() <= 16 && new <= MAX && class(l.size()).0 == class(new).0 {
                return p;
            }
            let n = unsafe { self.alloc(Layout::from_size_align_unchecked(new, l.align())) };
            if !n.is_null() {
                unsafe {
                    core::ptr::copy_nonoverlapping(p, n, l.size().min(new));
                    self.dealloc(p, l);
                }
            }
            n
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_cover_every_size() {
        let mut last = (0, 16);
        let mut s = 1;
        while s <= MAX {
            let (c, size) = class(s);
            assert!(size >= s && size % 16 == 0 && c < CLASSES, "{s}");
            assert!(
                c == last.0 && size == last.1 || c == last.0 + 1 && size > last.1,
                "{s}"
            );
            assert!(size - s < size / 4 + 16, "{s} wastes too much");
            last = (c, size);
            s += 1 + s / 64;
        }
        assert_eq!(class(MAX), (CLASSES - 1, MAX));
    }
}
