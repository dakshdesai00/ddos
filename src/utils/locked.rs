use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// this assembly code is temporary code we will change this when we have MMU cause atmoicbool,atomicusize needs cached ram which is on qemu but not on
// rpi so most code here does not work directly on the hardware so we go back to the basic just turn of the interupts so cpu doesnt change the thread
// BUT THIS WORKS ONLY ON SIGNLE CORE OR SINGLE CPU READ NOTES WHY
#[inline(always)]
fn disable_irq_and_save_state() -> bool {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let daif: u64;
        core::arch::asm!("mrs {0}, daif", out(reg) daif, options(nomem, nostack, preserves_flags));
        core::arch::asm!("msr daifset, #2", options(nomem, nostack, preserves_flags));
        return (daif & (1 << 7)) == 0;
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        false
    }
}

#[inline(always)]
fn restore_irq_state(was_enabled: bool) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        if was_enabled {
            core::arch::asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags));
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        let _ = was_enabled;
    }
}

// ============================================================================
// 1. TEST-AND-SET LOCK
// Uses: atomic .swap()
// ============================================================================

pub struct SpinLock<T> {
    #[cfg(feature = "rpi5")]
    locked_state: UnsafeCell<bool>,

    #[cfg(not(feature = "rpi5"))]
    locked_state: AtomicBool,

    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for SpinLock<T> {} // This tells rust that it is safe to share refrence between threads
unsafe impl<T> Send for SpinLock<T> {} // This tells rust that it is safe to transfer ownership between threads

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            #[cfg(feature = "rpi5")]
            locked_state: UnsafeCell::new(false),

            #[cfg(not(feature = "rpi5"))]
            locked_state: AtomicBool::new(false),

            data_to_protect: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        #[cfg(feature = "rpi5")]
        {
            loop {
                let irq_was_enabled = disable_irq_and_save_state();

                unsafe {
                    if !*self.locked_state.get() {
                        *self.locked_state.get() = true;
                        return SpinLockGuard {
                            lock: self,
                            irq_was_enabled,
                        };
                    }
                }

                restore_irq_state(irq_was_enabled);
                core::hint::spin_loop();
            }
        }

        #[cfg(not(feature = "rpi5"))]
        {
            // Test-And-Set: Atomically swap in 'true' and check what the old value was.
            while self.locked_state.swap(true, Ordering::Acquire) {
                core::hint::spin_loop();
            }
            SpinLockGuard { lock: self }
        }
    }

    pub fn unlock(&self) {
        #[cfg(feature = "rpi5")]
        unsafe {
            *self.locked_state.get() = false;
        }

        #[cfg(not(feature = "rpi5"))]
        self.locked_state.store(false, Ordering::Release);
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,

    #[cfg(feature = "rpi5")]
    irq_was_enabled: bool,
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

        #[cfg(feature = "rpi5")]
        restore_irq_state(self.irq_was_enabled);
    }
}

// ============================================================================
// 2. COMPARE-AND-SWAP LOCK (CAS)
// Uses: atomic .compare_exchange_weak()
// ============================================================================

pub struct CasLock<T> {
    locked_state: AtomicBool,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for CasLock<T> {}
unsafe impl<T> Send for CasLock<T> {}

impl<T> CasLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked_state: AtomicBool::new(false),
            data_to_protect: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> CasLockGuard<T> {
        // Compare-And-Swap: "If the current state is exactly false, make it true. Otherwise, fail."
        // We use 'weak' in loops because it can occasionally fail on ARM due to interrupts,
        // which is fine since we just spin and try again.
        while self
            .locked_state
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        CasLockGuard { lock: self }
    }

    pub fn unlock(&self) {
        self.locked_state.store(false, Ordering::Release);
    }
}

pub struct CasLockGuard<'a, T> {
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

pub struct TicketLock<T> {
    ticket_counter: AtomicUsize,
    turn_display: AtomicUsize,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for TicketLock<T> {}
unsafe impl<T> Send for TicketLock<T> {}

impl<T> TicketLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            ticket_counter: AtomicUsize::new(0),
            turn_display: AtomicUsize::new(0),
            data_to_protect: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> TicketLockGuard<T> {
        // Fetch-And-Add: Grab a ticket and increment the roll for the next CPU core.
        let my_ticket = self.ticket_counter.fetch_add(1, Ordering::Relaxed);

        // Spin until the "Now Serving" display matches our ticket number.
        while self.turn_display.load(Ordering::Acquire) != my_ticket {
            core::hint::spin_loop();
        }

        TicketLockGuard { lock: self }
    }

    pub fn unlock(&self) {
        // Increment the "Now Serving" display to wake up the next CPU core in line.
        self.turn_display.fetch_add(1, Ordering::Release);
    }
}

pub struct TicketLockGuard<'a, T> {
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
