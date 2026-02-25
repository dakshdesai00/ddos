use core::cell::UnsafeCell;

pub struct Locked<A> {
    inner: UnsafeCell<A>,
}

unsafe impl<A> Sync for Locked<A> {}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: UnsafeCell::new(inner),
        }
    }

    pub fn lock(&self) -> &mut A {
        unsafe { &mut *self.inner.get() }
    }
}
