/// A list of up to `N` ids held inline, spilling to the heap past that: an edge's two uses, a
/// face's loops. Reads as a slice.
#[derive(Clone)]
pub enum Small<T: Copy + Default, const N: usize> {
    Inline(u8, [T; N]),
    Heap(Vec<T>),
}

impl<T: Copy + Default, const N: usize> Small<T, N> {
    pub fn push(&mut self, x: T) {
        match self {
            Small::Inline(n, a) if (*n as usize) < N => {
                a[*n as usize] = x;
                *n += 1;
            }
            Small::Inline(_, a) => {
                let mut v = a.to_vec();
                v.push(x);
                *self = Small::Heap(v);
            }
            Small::Heap(v) => v.push(x),
        }
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        match self {
            Small::Inline(n, a) => {
                let mut k = 0;
                for i in 0..*n as usize {
                    if keep(&a[i]) {
                        a[k] = a[i];
                        k += 1;
                    }
                }
                *n = k as u8;
            }
            Small::Heap(v) => v.retain(keep),
        }
    }
}

impl<T: Copy + Default, const N: usize> Default for Small<T, N> {
    fn default() -> Self {
        Small::Inline(0, [T::default(); N])
    }
}

impl<T: Copy + Default, const N: usize> std::ops::Deref for Small<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            Small::Inline(n, a) => &a[..*n as usize],
            Small::Heap(v) => v,
        }
    }
}

impl<T: Copy + Default, const N: usize> std::ops::DerefMut for Small<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            Small::Inline(n, a) => &mut a[..*n as usize],
            Small::Heap(v) => v,
        }
    }
}

impl<'a, T: Copy + Default, const N: usize> IntoIterator for &'a Small<T, N> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Copy + Default, const N: usize> FromIterator<T> for Small<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        let mut s = Self::default();
        for x in it {
            s.push(x);
        }
        s
    }
}

impl<T: Copy + Default + PartialEq, const N: usize> PartialEq for Small<T, N> {
    fn eq(&self, o: &Self) -> bool {
        **self == **o
    }
}

impl<T: Copy + Default + std::fmt::Debug, const N: usize> std::fmt::Debug for Small<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
