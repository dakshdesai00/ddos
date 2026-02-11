use core::cell::UnsafeCell;

// A wrapper that allows us to modify data inside an immutable static variable.
// In a real OS, this would be a SpinLock (Chapter 28).
// Since we are single-threaded (for now), this is just a wrapper.
pub struct Locked<A> {
    inner: UnsafeCell<A>,
}

// We promise the compiler we will be careful sharing this across threads.
unsafe impl<A> Sync for Locked<A> {}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: UnsafeCell::new(inner),
        }
    }

    // This function gives you a mutable reference to the inner data.
    // It's your job to ensure no two threads call this at once (easy for now).
    pub fn lock(&self) -> &mut A {
        unsafe { &mut *self.inner.get() }
    }
}
