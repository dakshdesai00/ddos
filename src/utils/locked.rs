/*
 * locked.rs - Simple Synchronization Wrapper for Interior Mutability
 *
 * NOTE: This is AI-generated boilerplate code for synchronization.
 *
 * Problem it solves:
 * In Rust, static variables must be immutable (&T), but the allocator needs a
 * mutable FreeList to track memory regions. We need to mutate data inside an
 * immutable static variable, which violates Rust's borrow checker rules by default.
 *
 * Solution: Interior Mutability
 * This Locked<A> wrapper provides a way to mutate data through an immutable reference.
 * It uses UnsafeCell to bypass Rust's borrow checker in a controlled way.
 *
 * Why it's safe:
 * - We're in a single-threaded kernel (only one CPU core running at a time)
 * - No two threads can call lock() simultaneously
 * - Therefore, no data races can occur
 * - In multi-threaded systems, this would need to be a SpinLock (Chapter 28 of OSTEP)
 *
 * Usage:
 * static MY_DATA: Locked<MyType> = Locked::new(initial_value);
 * ...
 * unsafe { MY_DATA.lock().do_something(); }
 */

use core::cell::UnsafeCell;

/*
 * Locked<A> - A simple wrapper providing interior mutability
 *
 * Fields:
 * - inner: UnsafeCell containing the actual data
 *          UnsafeCell allows us to get mutable pointers from immutable references
 */
pub struct Locked<A> {
    inner: UnsafeCell<A>,
}

/*
 * Sync Safety Marker
 *
 * By implementing Sync for Locked<A>, we tell the compiler:
 * "It's safe to share this across threads (even though it contains UnsafeCell)"
 *
 * This is only true for single-threaded code. In multi-threaded code,
 * you'd need proper locking (spinlock, mutex, etc.).
 */
unsafe impl<A> Sync for Locked<A> {}

impl<A> Locked<A> {
    /*
     * Creates a new Locked wrapper around data
     *
     * Parameters:
     * - inner: Initial value to wrap
     *
     * Returns: Locked<A> containing the wrapped data
     *
     * Why const?
     * This can be called in const contexts to initialize static variables
     * at compile time, without runtime overhead
     */
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: UnsafeCell::new(inner),
        }
    }

    /*
     * Acquires mutable access to the wrapped data
     *
     * Returns: Mutable reference to the inner data
     *
     * Why unsafe?
     * This bypasses Rust's borrow checker - you're responsible for ensuring:
     * 1. Only one thread accesses the data at a time (we're single-threaded, so OK)
     * 2. No borrowed references exist from previous lock() calls (caller's job)
     *
     * How it works:
     * - self.inner.get() returns a raw *mut pointer to the UnsafeCell's contents
     * - We dereference it to create a mutable reference (&mut A)
     *
     * Note: The reference lifetime is tied to self's lifetime, but in practice
     * each lock() call should be short and not held across function boundaries
     */
    pub fn lock(&self) -> &mut A {
        unsafe { &mut *self.inner.get() }
    }
}
