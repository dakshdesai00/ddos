use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ============================================================================
// 1. TEST-AND-SET LOCK (Standard SpinLock)
// Uses: atomic .swap()
// ============================================================================

pub(crate) struct SpinLock<T> {
    locked_state: AtomicBool,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for SpinLock<T> {}
unsafe impl<T> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub(crate) const fn new(data: T) -> Self {
        Self {
            locked_state: AtomicBool::new(false),
            data_to_protect: UnsafeCell::new(data),
        }
    }

    pub(crate) fn lock(&self) -> SpinLockGuard<T> {
        // Test-And-Set: Atomically swap in 'true'.
        // If the old value was already 'true', someone else has the lock, so we spin.
        while self.locked_state.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }

    pub(crate) fn unlock(&self) {
        self.locked_state.store(false, Ordering::Release);
    }
}

pub(crate) struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data_to_protect.get() }
    }
}
impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data_to_protect.get() }
    }
}
impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

// ============================================================================
// 2. COMPARE-AND-SWAP LOCK (CAS)
// Uses: atomic .compare_exchange_weak()
// ============================================================================

pub(crate) struct CasLock<T> {
    locked_state: AtomicBool,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for CasLock<T> {}
unsafe impl<T> Send for CasLock<T> {}

impl<T> CasLock<T> {
    pub(crate) const fn new(data: T) -> Self {
        Self {
            locked_state: AtomicBool::new(false),
            data_to_protect: UnsafeCell::new(data),
        }
    }

    pub(crate) fn lock(&self) -> CasLockGuard<T> {
        // Compare-And-Swap: "If the current state is exactly false, make it true. Otherwise, fail."
        // We use 'weak' in loops because it can occasionally fail on ARM due to interrupts.
        while self
            .locked_state
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        CasLockGuard { lock: self }
    }

    pub(crate) fn unlock(&self) {
        self.locked_state.store(false, Ordering::Release);
    }
}

pub(crate) struct CasLockGuard<'a, T> {
    lock: &'a CasLock<T>,
}

impl<T> Deref for CasLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data_to_protect.get() }
    }
}
impl<T> DerefMut for CasLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data_to_protect.get() }
    }
}
impl<T> Drop for CasLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

// ============================================================================
// 3. TICKET LOCK (Fetch-And-Add)
// Uses: atomic .fetch_add()
// Guarantees fairness so no CPU core starves!
// ============================================================================

pub(crate) struct TicketLock<T> {
    ticket_counter: AtomicUsize,
    turn_display: AtomicUsize,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for TicketLock<T> {}
unsafe impl<T> Send for TicketLock<T> {}

impl<T> TicketLock<T> {
    pub(crate) const fn new(data: T) -> Self {
        Self {
            ticket_counter: AtomicUsize::new(0),
            turn_display: AtomicUsize::new(0),
            data_to_protect: UnsafeCell::new(data),
        }
    }

    pub(crate) fn lock(&self) -> TicketLockGuard<T> {
        // Fetch-And-Add: Grab a ticket and increment the roll for the next CPU core.
        let my_ticket = self.ticket_counter.fetch_add(1, Ordering::Relaxed);

        // Spin until the "Now Serving" display matches our ticket number.
        while self.turn_display.load(Ordering::Acquire) != my_ticket {
            core::hint::spin_loop();
        }

        TicketLockGuard { lock: self }
    }

    pub(crate) fn unlock(&self) {
        // Increment the "Now Serving" display to wake up the next CPU core in line.
        self.turn_display.fetch_add(1, Ordering::Release);
    }
}

pub(crate) struct TicketLockGuard<'a, T> {
    lock: &'a TicketLock<T>,
}

impl<T> Deref for TicketLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data_to_protect.get() }
    }
}
impl<T> DerefMut for TicketLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data_to_protect.get() }
    }
}
impl<T> Drop for TicketLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
