//! The build worker's messages as bytes: what crosses between the page and the default browser
//! build's build Worker, a Web Worker with its own wasm instance (no shared memory, so every
//! `fg::Msg` and job is copied over `postMessage`). Plain data only: a body, or anything else
//! holding the kernel's state, stays in the worker and is asked of it (`fg::Query`).
//!
//! Every value is written exactly as it is held (floats by their bits), so what the page gets is
//! bit for bit what the worker built. A struct is written field by field through `record!`, whose
//! decoder names every field: one added to a type and not here fails to compile, never drops
//! silently. Built on every target, so its round trips are tested natively.

use std::collections::HashMap;
use std::sync::Arc;

/// Bytes being written.
#[derive(Default)]
pub struct W(pub Vec<u8>);

/// Bytes being read: `None` from any read past the end or of a value that can't be.
pub struct R<'a>(pub &'a [u8]);

impl W {
    pub fn u8(&mut self, x: u8) {
        self.0.push(x);
    }
    /// A length or count: LEB128, as most are small.
    pub fn size(&mut self, mut x: usize) {
        loop {
            let b = (x & 0x7f) as u8;
            x >>= 7;
            if x == 0 {
                self.0.push(b);
                return;
            }
            self.0.push(b | 0x80);
        }
    }
    pub fn raw(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
}

impl<'a> R<'a> {
    pub fn u8(&mut self) -> Option<u8> {
        let (&b, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(b)
    }
    pub fn size(&mut self) -> Option<usize> {
        let mut x = 0usize;
        for shift in (0..64).step_by(7) {
            let b = self.u8()?;
            x |= ((b & 0x7f) as usize).checked_shl(shift)?;
            if b & 0x80 == 0 {
                return Some(x);
            }
        }
        None
    }
    pub fn raw(&mut self, n: usize) -> Option<&'a [u8]> {
        if n > self.0.len() {
            return None;
        }
        let (a, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(a)
    }
    /// A count of things each at least `min` bytes long: refused when the rest can't hold them
    /// (a bad length never makes a huge allocation).
    pub fn count(&mut self, min: usize) -> Option<usize> {
        let n = self.size()?;
        (n.checked_mul(min.max(1))? <= self.0.len() || min == 0).then_some(n)
    }
    pub fn done(&self) -> bool {
        self.0.is_empty()
    }
}

/// A value that crosses as bytes.
pub trait Wire: Sized {
    fn put(&self, o: &mut W);
    fn take(i: &mut R) -> Option<Self>;
    /// The fewest bytes one takes (bounds a count before reading).
    const MIN: usize = 1;
}

/// A value's bytes.
pub fn to_bytes<T: Wire>(x: &T) -> Vec<u8> {
    let mut o = W::default();
    x.put(&mut o);
    o.0
}

/// A value from its bytes, all of them.
pub fn from_bytes<T: Wire>(b: &[u8]) -> Option<T> {
    let mut i = R(b);
    let x = T::take(&mut i)?;
    i.done().then_some(x)
}

macro_rules! fixed {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            const MIN: usize = std::mem::size_of::<$t>();
            fn put(&self, o: &mut W) {
                o.raw(&self.to_le_bytes());
            }
            fn take(i: &mut R) -> Option<Self> {
                Some(<$t>::from_le_bytes(i.raw(std::mem::size_of::<$t>())?.try_into().ok()?))
            }
        }
    )*};
}
// floats by their bits (`to_le_bytes` is): NaNs and signed zeros cross as they are
fixed!(u16, u32, u64, u128, i32, i64, f32, f64);

impl Wire for u8 {
    fn put(&self, o: &mut W) {
        o.u8(*self);
    }
    fn take(i: &mut R) -> Option<Self> {
        i.u8()
    }
}

impl Wire for usize {
    fn put(&self, o: &mut W) {
        o.size(*self);
    }
    fn take(i: &mut R) -> Option<Self> {
        i.size()
    }
}

impl Wire for bool {
    fn put(&self, o: &mut W) {
        o.u8(*self as u8);
    }
    fn take(i: &mut R) -> Option<Self> {
        match i.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
}

impl Wire for String {
    fn put(&self, o: &mut W) {
        o.size(self.len());
        o.raw(self.as_bytes());
    }
    fn take(i: &mut R) -> Option<Self> {
        let n = i.count(1)?;
        Some(std::str::from_utf8(i.raw(n)?).ok()?.to_string())
    }
}

/// The names the kernel's reports hold as `&'static str` (a joint's kind, a property's key): each
/// distinct one read is kept once for the life of the page, so reading the same report again
/// costs nothing more.
fn interned(s: &str) -> &'static str {
    static SEEN: std::sync::Mutex<Option<std::collections::HashSet<&'static str>>> =
        std::sync::Mutex::new(None);
    let mut g = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    let set = g.get_or_insert_with(Default::default);
    if let Some(&k) = set.get(s) {
        return k;
    }
    let k: &'static str = Box::leak(s.to_string().into_boxed_str());
    set.insert(k);
    k
}

impl Wire for &'static str {
    fn put(&self, o: &mut W) {
        o.size(self.len());
        o.raw(self.as_bytes());
    }
    fn take(i: &mut R) -> Option<Self> {
        let n = i.count(1)?;
        Some(interned(std::str::from_utf8(i.raw(n)?).ok()?))
    }
}

impl Wire for std::path::PathBuf {
    fn put(&self, o: &mut W) {
        self.to_string_lossy().into_owned().put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        String::take(i).map(Into::into)
    }
}

impl<T: Wire> Wire for Vec<T> {
    fn put(&self, o: &mut W) {
        o.size(self.len());
        for x in self {
            x.put(o);
        }
    }
    fn take(i: &mut R) -> Option<Self> {
        let n = i.count(T::MIN)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(T::take(i)?);
        }
        Some(v)
    }
}

impl<T: Wire> Wire for Option<T> {
    fn put(&self, o: &mut W) {
        match self {
            None => o.u8(0),
            Some(x) => {
                o.u8(1);
                x.put(o);
            }
        }
    }
    fn take(i: &mut R) -> Option<Self> {
        match i.u8()? {
            0 => Some(None),
            1 => Some(Some(T::take(i)?)),
            _ => None,
        }
    }
}

impl<T: Wire, E: Wire> Wire for Result<T, E> {
    fn put(&self, o: &mut W) {
        match self {
            Ok(x) => {
                o.u8(0);
                x.put(o);
            }
            Err(e) => {
                o.u8(1);
                e.put(o);
            }
        }
    }
    fn take(i: &mut R) -> Option<Self> {
        match i.u8()? {
            0 => Some(Ok(T::take(i)?)),
            1 => Some(Err(E::take(i)?)),
            _ => None,
        }
    }
}

impl<T: Wire> Wire for Arc<T> {
    const MIN: usize = T::MIN;
    fn put(&self, o: &mut W) {
        (**self).put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        T::take(i).map(Arc::new)
    }
}

impl<T: Wire> Wire for Box<T> {
    const MIN: usize = T::MIN;
    fn put(&self, o: &mut W) {
        (**self).put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        T::take(i).map(Box::new)
    }
}

impl<T: Wire, const N: usize> Wire for [T; N] {
    const MIN: usize = T::MIN * N;
    fn put(&self, o: &mut W) {
        for x in self {
            x.put(o);
        }
    }
    fn take(i: &mut R) -> Option<Self> {
        let mut v = Vec::with_capacity(N);
        for _ in 0..N {
            v.push(T::take(i)?);
        }
        v.try_into().ok()
    }
}

impl<K: Wire + Eq + std::hash::Hash, V: Wire> Wire for HashMap<K, V> {
    // by the keys' bytes: equal maps write the same bytes, whatever their order in memory
    fn put(&self, o: &mut W) {
        let mut all: Vec<(Vec<u8>, &V)> = self.iter().map(|(k, v)| (to_bytes(k), v)).collect();
        all.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        o.size(all.len());
        for (k, v) in all {
            o.raw(&k);
            v.put(o);
        }
    }
    fn take(i: &mut R) -> Option<Self> {
        let n = i.count(K::MIN + V::MIN)?;
        let mut m = HashMap::with_capacity(n);
        for _ in 0..n {
            let k = K::take(i)?;
            m.insert(k, V::take(i)?);
        }
        Some(m)
    }
}

impl Wire for () {
    const MIN: usize = 0;
    fn put(&self, _: &mut W) {}
    fn take(_: &mut R) -> Option<Self> {
        Some(())
    }
}

macro_rules! tuple {
    ($($n:tt $t:ident),*) => {
        impl<$($t: Wire),*> Wire for ($($t,)*) {
            const MIN: usize = 0 $(+ $t::MIN)*;
            fn put(&self, o: &mut W) {
                $(self.$n.put(o);)*
            }
            fn take(i: &mut R) -> Option<Self> {
                Some(($($t::take(i)?,)*))
            }
        }
    };
}
tuple!(0 A);
tuple!(0 A, 1 B);
tuple!(0 A, 1 B, 2 C);
tuple!(0 A, 1 B, 2 C, 3 D);
tuple!(0 A, 1 B, 2 C, 3 D, 4 E);

/// A struct, field by field. Its decoder names every field, so a field added to the type and not
/// here doesn't compile.
macro_rules! record {
    ($t:path { $($f:ident),* $(,)? }) => {
        impl $crate::wire::Wire for $t {
            fn put(&self, o: &mut $crate::wire::W) {
                $($crate::wire::Wire::put(&self.$f, o);)*
            }
            fn take(i: &mut $crate::wire::R) -> Option<Self> {
                Some(Self { $($f: $crate::wire::Wire::take(i)?,)* })
            }
        }
    };
}

/// An enum, by a tag per variant, then its fields. The encoder's match is exhaustive: a variant
/// added to the type and not here doesn't compile.
macro_rules! choice {
    ($t:path { $($tag:literal => $v:ident $(($($a:ident),*))? $({$($n:ident),*})?),* $(,)? }) => {
        impl $crate::wire::Wire for $t {
            #[allow(unused_variables)]
            fn put(&self, o: &mut $crate::wire::W) {
                match self {
                    $(Self::$v $(($($a),*))? $({$($n),*})? => {
                        o.u8($tag);
                        $($($crate::wire::Wire::put($a, o);)*)?
                        $($($crate::wire::Wire::put($n, o);)*)?
                    })*
                }
            }
            fn take(i: &mut $crate::wire::R) -> Option<Self> {
                Some(match i.u8()? {
                    $($tag => Self::$v $(($({ let $a = $crate::wire::Wire::take(i)?; $a }),*))?
                        $({$($n: $crate::wire::Wire::take(i)?),*})?,)*
                    _ => return None,
                })
            }
        }
    };
}

/// A tuple struct, field by field.
macro_rules! record_tuple {
    ($t:path { $($n:tt),* $(,)? }) => {
        impl $crate::wire::Wire for $t {
            fn put(&self, o: &mut $crate::wire::W) {
                $($crate::wire::Wire::put(&self.$n, o);)*
            }
            fn take(i: &mut $crate::wire::R) -> Option<Self> {
                Some(Self($({ let _ = $n; $crate::wire::Wire::take(i)? }),*))
            }
        }
    };
}

pub(crate) use {choice, record};

mod types;
pub use types::crossable;

#[cfg(test)]
mod tests {
    use super::*;

    /// Written, read back, written again: the same bytes, every byte read.
    pub fn round<T: Wire>(x: &T) -> T {
        let a = to_bytes(x);
        let y: T = from_bytes(&a).expect("reads back");
        assert_eq!(to_bytes(&y), a, "written again the same");
        y
    }

    #[test]
    fn plain() {
        assert_eq!(
            round(&(3u64, -0.0f64, f32::NAN.to_bits())),
            (3, -0.0, f32::NAN.to_bits())
        );
        assert!(round(&-0.0f64).is_sign_negative());
        assert_eq!(
            round(&vec![Some("ab".to_string()), None]),
            vec![Some("ab".into()), None]
        );
        assert_eq!(round(&usize::MAX), usize::MAX);
        let r: Result<[f32; 2], String> = Err("no".into());
        assert_eq!(round(&r), r);
        assert_eq!(round(&"kind"), "kind");
        // a length past the end is refused, not allocated
        assert!(from_bytes::<Vec<u64>>(&[0xff, 0xff, 0xff, 0x7f]).is_none());
        // trailing bytes are refused
        assert!(from_bytes::<u8>(&[1, 2]).is_none());
    }
}
