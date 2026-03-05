# Locks — Phase 2 Notes

**OSTEP Chapters:** 26, 28, 29 (core: 28)
**What I built:** Three lock implementations — Test-And-Set SpinLock, Compare-And-Swap Lock, Ticket Lock (Fetch-And-Add)
**Files:**

- `src/utils/locked.rs` — mine, all three lock implementations + interrupt disable hack
- `src/memory/heap.rs` — updated, now uses `SpinLock` instead of the fake `Locked`
- `src/memory/mod.rs` — updated, `ALLOCATOR` type changed to `SpinLock<FreeList>`
- `src/drivers/uart.rs` — updated, now uses `SpinLock<Uart>` and has `print!`/`println!` macros

---

## Why We Even Need Locks — Chapter 26

Before writing a single line of lock code, you need to understand what problem we're solving. Chapter 26 of OSTEP is entirely about this.

### The Race Condition

The problem is called a **race condition**. It happens when two threads (or two CPU cores, or a thread and an interrupt handler) try to read and modify the same variable at the same time, and the final result depends on which one wins the race. The result is unpredictable, changes every run, and is almost impossible to debug.

Here's the textbook example. Suppose two threads both want to increment a counter:

```ddos/Notes/locks(phase 2).md#L1-1
counter = counter + 1;
```

That looks like one operation. In reality, the CPU breaks it into three separate instructions:

```ddos/Notes/locks(phase 2).md#L1-1
load  R1, counter    // Step 1: Read the value from memory into a register
add   R1, R1, 1      // Step 2: Add 1 to that register
store counter, R1    // Step 3: Write the result back to memory
```

Now imagine Thread 1 and Thread 2 both running this. Thread 1 executes step 1, reads `counter = 50`. The OS timer fires right after that, before Thread 1 can finish. Thread 2 wakes up, runs all three steps, writes `counter = 51`. Now Thread 1 wakes back up. It still has `50` in its register from step 1 — it has no idea Thread 2 did anything. It adds 1, gets 51, writes `counter = 51`.

Both threads ran. Counter was incremented twice. But the final value is 51, not 52. One increment was silently thrown away. OSTEP calls this a **lost update**.

The horror is that this bug only appears sometimes. Most of the time Thread 1 finishes all three steps uninterrupted and you get 52. Only when the OS happens to switch threads between those specific instructions do you get the bug. You can run your program a thousand times and it works fine. Then under load, or on a different machine, it breaks.

### Critical Sections

The solution the book describes is the concept of a **critical section** — a region of code that accesses shared data and must not be executed by more than one thread at a time.

What you want is something like this:

```ddos/Notes/locks(phase 2).md#L1-1
lock()
counter = counter + 1   // only one thread can be here at a time
unlock()
```

The `lock()` call before the critical section guarantees that if another thread already holds the lock, the current thread waits. The `unlock()` call at the end signals that the current thread is done and someone else can proceed.

Three properties OSTEP says a good lock must have:

**Mutual Exclusion** — only one thread can hold the lock at a time. This is the basic correctness guarantee. Without this, the whole thing is pointless.

**Fairness** — if multiple threads are waiting for a lock, do they all eventually get a turn? Or can one thread starve forever while others keep cutting in line? Basic spinlocks fail this. Ticket locks fix it.

**Performance** — how much overhead does locking add? On a single CPU with low contention, a lock should be nearly free. On multiple CPUs under heavy contention, the cost of waiting has to be managed carefully.

### The Hardware Requirement

You can't build a correct lock out of ordinary load and store instructions. The problem is that checking the lock and setting the lock are two separate operations, and a thread can be interrupted between them.

```ddos/Notes/locks(phase 2).md#L1-1
if (lock == 0) {        // ← interrupted here!
    lock = 1;           // another thread already set it, now two threads hold the lock
}
```

To build a real lock, you need the CPU hardware to provide a way to read a value and write a new value in a single, **atomic** operation — one that cannot be interrupted midway. No other CPU core can observe a half-finished state.

Modern CPUs provide exactly this in the form of special atomic instructions. That's what chapters 28 goes through.

---

## The Oldest Trick: Disabling Interrupts

Before hardware atomic instructions existed, the first operating systems used an even more primitive approach: just turn off hardware interrupts entirely before touching shared data, then turn them back on after.

```ddos/Notes/locks(phase 2).md#L1-1
disable_interrupts()
counter = counter + 1   // nothing can interrupt us now
enable_interrupts()
```

On a single CPU, a race condition can only happen if the CPU switches threads mid-instruction. Thread switching is triggered by a hardware timer interrupt. If you disable interrupts, the timer can't fire, the scheduler can't run, and you're the only thread executing. Mutual exclusion achieved, no special hardware needed.

OSTEP covers this in section 28.3 and is very direct about its problems:

**It only works on a single CPU.** Disabling interrupts stops the timer on _this_ core. Core 1 is still running and can reach into the same RAM and corrupt your data. On a multiprocessor machine, this is completely broken.

**It gives user programs too much power.** If you let user programs disable interrupts, a malicious or buggy program can just turn off interrupts and hold the CPU forever. The OS never gets control back.

**It can miss real hardware events.** Disk completion interrupts, network packets arriving — if you're inside a `disable_interrupts()` section when these fire, they're lost.

Despite all this, for a single-core OS that hasn't turned on the MMU yet, it's actually the right choice. And as you'll see in a moment, that's exactly what we have to do on real Pi 5 hardware. The textbook documents this as the historically first approach, and we get to live the history ourselves.

---

## Hardware Atomics — The Four Instructions

Chapter 28 goes through the hardware primitives CPUs provide to build proper locks. Each one does the read-modify-write in a single uninterruptible step.

### Test-And-Set

Test-And-Set is the simplest atomic primitive. It does two things atomically:

1. Read the current value
2. Write a new value into the same location

And it hands you back the **old** value. That's it. Here's the conceptual C:

```ddos/Notes/locks(phase 2).md#L1-1
int TestAndSet(int *ptr, int new) {
    int old = *ptr;   // both of these happen
    *ptr = new;       // atomically, as one instruction
    return old;
}
```

You build a lock with it like this: the lock variable is 0 when free and 1 when held. To acquire the lock, you call `TestAndSet(&lock, 1)`. If it returns 0, the lock was free and you just grabbed it by writing 1. If it returns 1, the lock was already held — you keep spinning and trying again.

```ddos/Notes/locks(phase 2).md#L1-1
while (TestAndSet(&lock, 1) == 1) {
    // spin
}
// critical section
lock = 0;  // release
```

This is a **spinlock** — a lock where the waiting thread does nothing but loop continuously testing the lock. It burns CPU cycles doing no useful work, but it's simple and fast when the wait time is very short.

In Rust, `TestAndSet` is the `.swap()` method on atomic types. `AtomicBool.swap(true, Ordering::Acquire)` atomically sets the bool to true and returns what it was before. This is our `SpinLock` in `locked.rs`.

### Compare-And-Swap

Compare-And-Swap (CAS) is more powerful. It does three things atomically:

1. Read the current value
2. **Compare** it against an expected value
3. Only write the new value **if** the current value matched the expected value

In conceptual C:

```ddos/Notes/locks(phase 2).md#L1-1
int CompareAndSwap(int *ptr, int expected, int new) {
    int actual = *ptr;
    if (actual == expected) {
        *ptr = new;
        return 1;  // success
    }
    return 0;  // failed, someone else changed it
}
```

For a simple lock this doesn't seem much different from Test-And-Set. The power of CAS shows up in lock-free data structures — situations where you want to update a node in a linked list only if nobody else has changed it since you last looked. CAS lets you build things that Test-And-Set can't.

For our purposes, we use it for `CasLock`. The lock variable starts `false`. To acquire: "if it's currently `false`, change it to `true`." If the compare fails (it was already `true`), we spin.

In Rust this is `.compare_exchange()` and `.compare_exchange_weak()`. The `_weak` variant is allowed to occasionally fail even when the values match — this is fine inside a loop because you just try again, and `_weak` is faster on ARM.

### Load-Linked / Store-Conditional

Here's where the ARM architecture gets interesting. ARM processors do not have a native Compare-And-Swap instruction. Instead, they implement LL/SC: two instructions that together achieve the same effect.

**Load-Linked (ldaxr):** Read the memory value normally, but also tell the CPU's **Exclusive Monitor** hardware to start watching that memory address. The CPU now has a flag set saying "I'm monitoring this address."

**Store-Conditional (stlxr):** Attempt to write the new value. Before writing, the hardware checks: did anyone else write to this monitored address since our `ldaxr`? If yes, the store **fails** and returns a non-zero status. If the address is still untouched, the store succeeds and clears the monitor.

```ddos/Notes/locks(phase 2).md#L1-1
retry:
    ldaxr  w1, [x0]        // Load current value, start monitoring
    cbnz   w1, retry       // If already locked (non-zero), spin
    mov    w2, #1
    stlxr  w3, w2, [x0]    // Try to store 1. w3 = 0 if success, 1 if failed
    cbnz   w3, retry       // If store failed (someone else interfered), retry
    // lock acquired
```

When you write `.swap()` or `.compare_exchange()` in Rust for an `AtomicBool` targeting ARM, the compiler generates exactly this `ldaxr`/`stlxr` pair. You don't write the assembly — the compiler does. This means CAS and Test-And-Set are the same underlying hardware mechanism on ARM. There is no separate CAS instruction. The `ldaxr`/`stlxr` pair emulates both.

**Why we didn't write LL/SC explicitly:** Because Rust's atomic types handle the translation automatically. When you write `.compare_exchange_weak()`, you get the correct `ldaxr`/`stlxr` assembly for ARM. Writing it manually in inline assembly would be the same thing but harder to read and easier to get wrong.

### Fetch-And-Add

Fetch-And-Add atomically increments a value and returns the old value:

```ddos/Notes/locks(phase 2).md#L1-1
int FetchAndAdd(int *ptr, int amount) {
    int old = *ptr;   // atomically:
    *ptr += amount;   // add amount, return old value
    return old;
}
```

This one is the key to building **fair** locks. Instead of a simple locked/unlocked bit, you use two counters: a ticket counter and a turn counter. When a thread wants the lock, it calls `FetchAndAdd` on the ticket counter to grab a ticket number. It then spins waiting for the turn counter to reach its ticket number. When the current lock holder releases, it increments the turn counter — which wakes up exactly the next thread in line.

This is the **Ticket Lock**, and it's the most interesting lock we implemented. We'll cover it in detail in the code section.

In Rust, Fetch-And-Add is the `.fetch_add()` method on atomic types.

---

## Instead of Spinning: Yield and Sleep

Chapter 28 also covers what to do instead of burning CPU cycles in a spin loop. There are two better alternatives.

### Yield

The simplest improvement: when you try to acquire a lock and it's already held, instead of spinning, call `yield()` — give up the CPU voluntarily and let the scheduler run someone else. The spinning thread goes back to the ready queue and tries again when it gets scheduled next.

```ddos/Notes/locks(phase 2).md#L1-1
while (TestAndSet(&lock, 1) == 1) {
    yield();  // give up the CPU instead of spinning
}
```

This is much better under high contention. Instead of burning an entire CPU core doing nothing, the waiting thread parks itself and lets useful work happen. But it still has overhead — the thread gets re-scheduled repeatedly, checks the lock, and parks again if it's still held. Under very high contention with many threads this is O(threads) wasted context switches per unlock.

### Sleep (Queue-Based Locks)

The real solution is for waiting threads to go to sleep entirely and only be woken up when the lock is actually available. The lock data structure maintains a queue of sleeping threads. When a thread tries to acquire a held lock, it adds itself to the queue and calls `sleep()` — it won't run at all until someone wakes it up. When the holder calls `unlock()`, it pops a thread from the queue and wakes it up.

This is what Linux's `mutex` does. Zero CPU wasted on spinning. The downside is that calling into the scheduler to sleep and wake costs more time than a few spin iterations — so for very short critical sections (a few nanoseconds), a spinlock actually beats a sleep-based lock.

### Why We Haven't Done Either Yet

**We cannot yield or sleep right now because we haven't built a Scheduler yet.**

`yield()` requires a scheduler that can park a thread, pick another one, and come back. We have no thread abstraction, no process table, no timer interrupt handler that switches context. There is nothing to yield to.

Sleep-based locks require all of the above plus a mechanism to add a thread to a wait queue, put it to sleep without losing it forever, and wake it up when the lock is free. That's even more infrastructure we don't have.

Right now, spinning is the only option. And for a single-core kernel where critical sections are extremely short (writing a character to UART, doing a heap allocation), spinning is completely fine. The scheduler and proper sleeping locks come later in the roadmap.

---

## The ARM Hardware Problem: Why AtomicBool Breaks on Real Pi 5

This is where theory meets bare-metal reality and reality wins.

The plan after reading Chapter 28 was simple: use Rust's `AtomicBool` with `.swap()` and `.compare_exchange()`. The Rust compiler translates these to `ldaxr`/`stlxr` ARM instructions. Textbook spinlock. Run it on the Pi 5.

It crashed. Or locked up forever. Or produced garbage output. Not on QEMU — QEMU worked perfectly. Only on real hardware.

### The Villain: The Exclusive Monitor Needs Cacheable Memory

Here's the hardware fact that nobody documents clearly. The ARM Exclusive Monitor — the hardware mechanism that makes `ldaxr`/`stlxr` work — only functions correctly on memory that is marked **Cacheable** in the MMU page tables.

Right now, DDOS doesn't have an MMU driver. The MMU is completely off. When the MMU is off, the ARM architecture treats all memory as **Device memory** or **Strongly Ordered memory** — both of which are non-cacheable.

When `stlxr` runs on non-cacheable memory, the Exclusive Monitor has no way to track whether another core touched the address between the `ldaxr` and the `stlxr`. The hardware essentially says "I can't guarantee the atomicity of this operation" and the instruction either silently fails (returning failure status forever, causing an infinite loop) or behaves unpredictably.

QEMU doesn't simulate this limitation. QEMU's emulated CPU is more permissive and lets `ldaxr`/`stlxr` work on non-cacheable memory. Real Cortex-A76 silicon does not.

### The Hero: Disabling Interrupts as a Lock Primitive

The workaround is to use the oldest trick in the book from OSTEP 28.3 — disable hardware interrupts.

On a single core, a race condition can only happen if a timer interrupt fires and causes a context switch mid-critical-section. If you disable interrupts, the timer can't fire. Nothing can preempt you. You have exclusive access without needing the Exclusive Monitor at all.

The ARM register that controls this is `daif`. The 'I' bit (bit 7) controls the IRQ (interrupt request) mask. Setting it disables interrupts. Clearing it re-enables them.

```ddos/Notes/locks(phase 2).md#L1-1
mrs  x0, daif       // read current interrupt state
msr  daifset, #2    // set the 'I' bit — disable IRQs
```

Our lock on Pi 5 does exactly this: disable interrupts, check and set the lock variable (a plain `bool` in an `UnsafeCell`, no atomics needed since we're the only thing running), then re-enable interrupts if we didn't get the lock.

**The critical caveat:** This only works because we're running on Core 0 alone. If Cores 1, 2, 3 were active, Core 0 disabling its own interrupts does nothing to stop Core 1 from reaching into the same `UnsafeCell<bool>` simultaneously. The interrupt-disable hack is single-core only.

### The Grand Plan

This is not a permanent hack. It's exactly the evolutionary path operating systems followed historically:

1. **Right now (no MMU, single core):** Disable interrupts to fake atomicity. Safe, correct, limited to one core.

2. **Phase 4 — Paging:** We write the MMU driver. This turns on the page tables and marks RAM as Normal Cacheable memory.

3. **The glorious return:** Once RAM is cacheable, the ARM Exclusive Monitor works. We delete all the `daif` assembly, uncomment the `AtomicBool` code path, and instantly have a multi-core safe OS. The `#[cfg(feature = "rpi5")]` conditional compilation blocks we wrote now become scrap.

You haven't broken anything. You wrote the Chapter 28.3 primitive to survive the bare-metal boot phase. The textbook literally documents this approach as the historical starting point.

---

## The Code — `src/utils/locked.rs`

This file replaced the old `Locked<T>` from Phase 1 entirely. Three lock types live here now, plus the interrupt-disable plumbing.

### The Interrupt Plumbing (Pi 5 Only)

```ddos/src/utils/locked.rs#L5-20
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
```

`#[inline(always)]` — force the compiler to paste this function's body directly at every call site. Never generate a real function call and stack frame. For something that runs on every lock acquisition, the call overhead matters.

`#[cfg(target_arch = "aarch64")]` — this entire block is only compiled on 64-bit ARM. On any other architecture it disappears completely. This is the same conditional compilation pattern used throughout — the lock works on x86 or RISC-V in simulation without needing ARM-specific instructions.

`core::arch::asm!` — inline assembly inside Rust. The exclamation mark means it's a macro. Inside the string is raw ARM assembly.

`"mrs {0}, daif"` — `mrs` is "Move from System Register." `{0}` is a Rust inline assembly placeholder that gets replaced with a real register name by the compiler. `daif` is the name of the ARM system register that holds interrupt flags. This reads the current interrupt state into a CPU register, and that register value gets stored in the Rust variable `daif`.

`out(reg) daif` — the `out(reg)` tells the compiler "this placeholder `{0}` is an output, pick any general-purpose register for it, and store the result in the Rust variable `daif`."

`"msr daifset, #2"` — `msr` is "Move to System Register." `daifset` is a write-only register that sets individual bits in `daif`. Writing `#2` sets bit 1, which corresponds to the 'I' (IRQ) bit. This is the master kill-switch for hardware interrupts.

`options(nomem, nostack, preserves_flags)` — these are hints to the compiler. `nomem` means "this asm doesn't read or write normal memory, so don't fence memory accesses around it." `nostack` means "this asm doesn't touch the stack." `preserves_flags` means "this asm doesn't change the CPU condition flags (NZCV)." These allow the compiler to schedule the surrounding code more freely.

`return (daif & (1 << 7)) == 0` — check bit 7 of the value we read from `daif`. On ARM, bit 7 of `daif` is the 'I' mask bit. If it's 0, interrupts were enabled when we entered. If it's 1, interrupts were already disabled. We return `true` if interrupts were enabled (we turned them off and need to restore) and `false` if they were already off (we should leave them off when done).

```ddos/src/utils/locked.rs#L22-35
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
```

`daifclr` is the opposite of `daifset` — writing `#2` to it clears bit 1 of `daif`, re-enabling IRQs. We only re-enable if `was_enabled` is true. If interrupts were already off when we entered the lock, we leave them off — we shouldn't change state we didn't create.

`let _ = was_enabled` on non-AArch64 — the variable isn't used on other architectures, and Rust warns about unused variables. Binding to `_` silences the warning without doing anything.

---

### Lock 1: SpinLock (Test-And-Set)

```ddos/src/utils/locked.rs#L40-55
pub struct SpinLock<T> {
    #[cfg(feature = "rpi5")]
    locked_state: UnsafeCell<bool>,

    #[cfg(not(feature = "rpi5"))]
    locked_state: AtomicBool,

    data_to_protect: UnsafeCell<T>,
}
```

`SpinLock<T>` is generic over `T` — it can protect any type. The angle brackets `<T>` declare a type parameter that gets filled in at the call site: `SpinLock<Uart>`, `SpinLock<FreeList>`, whatever you need.

The struct has two different fields depending on the compilation target — this is the hardware split:

`#[cfg(feature = "rpi5")]` — if we're building with the `rpi5` feature flag (which Cargo uses when compiling for actual Pi 5 hardware), use `UnsafeCell<bool>`. Plain bool, no hardware atomics.

`#[cfg(not(feature = "rpi5"))]` — on everything else (QEMU, other targets), use `AtomicBool`. This is the proper hardware-atomic version.

`data_to_protect: UnsafeCell<T>` — the actual data the lock guards. `UnsafeCell<T>` is Rust's fundamental interior mutability primitive. Without it, Rust's type system would never let you get a `&mut T` from a shared `&SpinLock<T>`. `UnsafeCell` tells the compiler "yes I know what I'm doing, mutation through shared reference is happening here, I take responsibility."

#### The Sync and Send Lines

```ddos/src/utils/locked.rs#L57-58
unsafe impl<T> Sync for SpinLock<T> {}
unsafe impl<T> Send for SpinLock<T> {}
```

These two lines are doing something very specific. By default, Rust is conservative about what types are safe to share across threads.

`Sync` is Rust's trait for "it is safe to share a reference to this type across multiple threads simultaneously." By default, any type containing `UnsafeCell<T>` is NOT `Sync` — Rust refuses to let you put it in a `static` variable because it can't verify you're handling concurrent access safely.

`Send` is Rust's trait for "it is safe to send ownership of this type from one thread to another." Also denied by default for `UnsafeCell`.

The `unsafe impl` overrides Rust's conservative default. We are manually promising: "Yes, this type can be shared across threads, and we the programmer have ensured mutual exclusion is correctly handled." This is marked `unsafe` because the programmer is making a guarantee the compiler cannot verify. If you lie — if you `unsafe impl Sync` for a type that actually has a data race — the compiler will not catch it. You own the bug.

For `SpinLock`, this promise is valid because the lock mechanism (either interrupts-off or atomic swap) genuinely prevents concurrent access to the inner data.

#### Locking: The Test-And-Set Implementation

```ddos/src/utils/locked.rs#L70-103
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
        while self.locked_state.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}
```

`lock()` takes `&self` — a shared immutable reference. This seems wrong at first: how can a function that provides exclusive mutable access take a shared reference? The answer is the `UnsafeCell` inside. `UnsafeCell` is specifically designed to allow mutation through shared references. The lock ensures only one thread ever gets to do that mutation at a time.

**On rpi5:** Disable interrupts. Check the raw bool. If false (lock is free), set it to true and return the guard. If true (lock is held), re-enable interrupts and spin with `spin_loop()`. The loop keeps trying until it gets in.

`self.locked_state.get()` — `UnsafeCell::get()` returns a raw pointer `*mut bool` to the inner data. This is the only way to get a pointer out of an `UnsafeCell`. Then `*self.locked_state.get()` dereferences it to read the bool. The `unsafe {}` block is required because raw pointer dereference is always unsafe in Rust.

**On QEMU/other:** `self.locked_state.swap(true, Ordering::Acquire)`. This is the Test-And-Set. `.swap(true, ...)` atomically writes `true` to the `AtomicBool` and returns whatever was there before. If the returned value was `false`, the lock was free and we just grabbed it. If it was `true`, the lock was held — we keep spinning.

`core::hint::spin_loop()` — emits a CPU hint instruction (on ARM it's `yield`, not to be confused with OS-level yield — it's a hardware power/performance hint). This tells the CPU "I'm in a spin loop, please manage power and pipeline resources accordingly." It doesn't actually yield to another thread — it's just an optimization hint.

The return type `SpinLockGuard<T>` is the lock guard — a value that holds the lock and automatically releases it when dropped. More on this in a moment.

#### The RAII Guard Pattern

```ddos/src/utils/locked.rs#L113-131
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
```

`'a` is a **lifetime parameter**. It ties the guard's lifetime to the lock's lifetime. The guard holds a reference `&'a SpinLock<T>` — Rust's borrow checker uses this to enforce that the guard cannot outlive the lock it came from. You can't drop the `SpinLock` while someone is holding a `SpinLockGuard` to it.

`Deref` and `DerefMut` are the traits that let `*guard` or `guard.method()` work as if `guard` were the inner `T` directly. When you write `uart.write_str("hello")` on a `SpinLockGuard<Uart>`, Rust automatically calls `deref_mut()` to get a `&mut Uart` and then calls `write_str` on that. You never have to manually unwrap the guard.

The `Drop` impl is the entire point of the guard. Rust calls `drop()` automatically when the guard goes out of scope — end of function, end of block, even if there's a panic. This implements **RAII** (Resource Acquisition Is Initialization) for locks. You physically cannot forget to release the lock because the compiler enforces the drop. In C you could write `lock()` and return early without `unlock()` and corrupt your system. In Rust, the guard pattern makes that impossible.

On Pi 5, `drop()` also calls `restore_irq_state(self.irq_was_enabled)` — it re-enables interrupts when the lock is released. The interrupt disable is scoped exactly to the duration of the lock hold.

---

### Lock 2: CasLock (Compare-And-Swap)

```ddos/src/utils/locked.rs#L140-185
pub struct CasLock<T> {
    locked_state: AtomicBool,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for CasLock<T> {}
unsafe impl<T> Send for CasLock<T> {}

impl<T> CasLock<T> {
    pub fn lock(&self) -> CasLockGuard<T> {
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
```

`CasLock` doesn't have a Pi 5 workaround. It's intentionally kept as the pure atomic version — it's here to demonstrate the Compare-And-Swap primitive and to have something that compiles and works correctly on QEMU. On real Pi 5 hardware before the MMU is on, this lock would encounter the same Exclusive Monitor issue as `AtomicBool` would. `SpinLock` is the one we actually use.

`.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)` — this is the CAS operation. Breaking down the arguments:

- `false` — the **expected** value. "I expect the lock to currently be false (unlocked)."
- `true` — the **new** value. "If it is false, swap it to true (lock it)."
- `Ordering::Acquire` — the success ordering. If the exchange succeeds, apply Acquire semantics.
- `Ordering::Relaxed` — the failure ordering. If the exchange fails (lock was already held), use Relaxed semantics — no ordering needed because we didn't get the lock.

`.is_err()` — `compare_exchange_weak` returns `Result<T, T>`. `Ok(false)` means "yes it was false, we changed it to true, you now own the lock." `Err(true)` means "it was true (already locked), we did nothing, keep trying."

**`_weak` vs regular:** `compare_exchange` (without `_weak`) guarantees success if the values match. `compare_exchange_weak` is allowed to spuriously fail — fail even when the values actually matched. On ARM, because CAS is implemented as `ldaxr`/`stlxr`, the store conditional can fail if something else touched the monitored address, even if it wasn't our lock. In a loop we don't care — we just retry. `_weak` is faster on ARM because it maps more naturally to the hardware instruction.

---

### Lock 3: TicketLock (Fetch-And-Add)

```ddos/src/utils/locked.rs#L195-215
pub struct TicketLock<T> {
    ticket_counter: AtomicUsize,
    turn_display: AtomicUsize,
    data_to_protect: UnsafeCell<T>,
}

unsafe impl<T> Sync for TicketLock<T> {}
unsafe impl<T> Send for TicketLock<T> {}
```

Two counters instead of one bool. Think of it exactly like a deli counter — the kind with a ticket dispenser and a "Now Serving" sign.

`ticket_counter` — the ticket dispenser. When a CPU core wants the lock, it pulls a ticket. This counter increments every time someone takes a ticket. It only ever goes up.

`turn_display` — the "Now Serving" sign. The current lock holder owns the turn corresponding to their ticket. When they're done, they increment this counter by 1, advancing the display to the next number in line.

```ddos/src/utils/locked.rs#L218-237
pub fn lock(&self) -> TicketLockGuard<T> {
    let my_ticket = self.ticket_counter.fetch_add(1, Ordering::Relaxed);

    while self.turn_display.load(Ordering::Acquire) != my_ticket {
        core::hint::spin_loop();
    }

    TicketLockGuard { lock: self }
}

pub fn unlock(&self) {
    self.turn_display.fetch_add(1, Ordering::Release);
}
```

`self.ticket_counter.fetch_add(1, Ordering::Relaxed)` — the Fetch-And-Add. Atomically add 1 to `ticket_counter` and return the old value. That old value is your ticket. If three cores call this simultaneously, they get tickets 0, 1, 2 — guaranteed, no duplicates, no races, because the add is atomic.

`Ordering::Relaxed` here — we just want the atomic increment, we don't need any memory ordering guarantees at this point. Having a ticket doesn't give you access to the protected data. The actual memory barrier comes from the `Acquire` in the next line.

`self.turn_display.load(Ordering::Acquire)` — spin until the "Now Serving" display matches our ticket. The `Acquire` here is the memory barrier that matters — it ensures that once we see our ticket number on the display, we are guaranteed to see all the memory writes made by the previous lock holder before their `Release` unlock.

`self.turn_display.fetch_add(1, Ordering::Release)` — when unlocking, increment the display. This wakes up exactly the next ticket holder. The `Release` ensures all our writes inside the critical section become visible to the next holder before they see the updated display counter.

**Why TicketLock is fairer than SpinLock:**

With a Test-And-Set spinlock, every waiting thread hammers the lock simultaneously. When it releases, whichever thread happens to win the atomic race gets it. There's no ordering. A thread that has been waiting for a second has no better chance than one that just started waiting. Under high contention, a thread can starve — wait forever while other threads keep winning.

With a TicketLock, tickets are handed out in order. Turn display advances in order. If you got ticket 5 and the display is at 3, you know you're waiting for exactly two more holders to finish. You will get the lock. Starvation is impossible. Every thread gets its turn.

The practical difference shows up in fairness under load. SpinLock has better raw throughput when there's no contention (just one compare and you're in). TicketLock has worse throughput under high contention because every CPU core is constantly reading `turn_display` (cache line contention), but it's strictly fair. For an OS that eventually has multiple cores and wants predictable behavior, fairness matters.

---

## The Ordering Syntax — Memory Barriers

This is the most conceptually difficult part of the lock code. The `Ordering` enum values that appear everywhere are not optional decoration — they are instructions to both the compiler and the CPU hardware.

Modern processors like the Cortex-A76 in the Pi 5 are deeply out-of-order. To maximize speed, the CPU looks ahead at upcoming instructions and executes them early if they don't depend on previous results. The Rust compiler does the same thing — it rearranges code to optimize register use, eliminate redundant loads, and reduce pipeline stalls.

This is completely fine for single-threaded code. The observable result is the same whether the CPU runs instructions in program order or rearranged order — within a single thread, data dependencies prevent anything externally visible from changing.

With multiple threads or concurrent hardware, it becomes catastrophic. Example:

```ddos/Notes/locks(phase 2).md#L1-1
// Thread 1 (lock, write, unlock):
lock.swap(true);          // acquire the lock
uart_register = 'H';      // write to UART
lock.store(false);        // release the lock
```

The CPU sees that writing to `lock` and writing to `uart_register` don't depend on each other. It might decide to release the lock first, then write to UART. Now Thread 2 grabs the lock between those two operations and also starts writing to UART. Your output is corrupted.

Memory ordering tells the CPU which reorderings are forbidden.

### Ordering::Acquire

Used when **acquiring** a lock (on the `.swap()` in `SpinLock.lock()` and the `.load()` in `TicketLock.lock()`).

Acquire creates a one-way barrier. It says: **nothing from below this point can move above this point.** All the work you do inside the critical section stays inside the critical section. You cannot start accessing the UART before you've confirmed you own the lock.

Visually: the barrier is a floor that code cannot fall through upward.

```ddos/Notes/locks(phase 2).md#L1-1
--- code before the lock (can be reordered freely up here) ---
[ACQUIRE barrier] ← nothing below can move above this line
--- critical section (all of this stays here) ---
```

### Ordering::Release

Used when **releasing** a lock (on the `.store()` in `SpinLock.unlock()` and the `.fetch_add()` in `TicketLock.unlock()`).

Release creates the opposite barrier. It says: **nothing from above this point can move below this point.** All the work done in the critical section must be fully committed to memory before the lock is released. The next thread that acquires the lock is guaranteed to see all your writes.

Visually: a ceiling that code cannot rise through downward.

```ddos/Notes/locks(phase 2).md#L1-1
--- critical section (all of this stays here) ---
[RELEASE barrier] ← nothing above can move below this line
--- code after the unlock (can be reordered freely down here) ---
```

### Ordering::Relaxed

Used in `TicketLock.lock()`'s `fetch_add` — pulling a ticket.

Relaxed says: **do the atomic operation, but impose no ordering constraints on surrounding code.** The compiler and CPU can freely move other instructions around the `fetch_add`.

We can use Relaxed here because taking a ticket doesn't give you access to the protected data. You haven't acquired the lock yet — you're just getting in line. The ordering guarantee that matters (the one that ensures you see the previous holder's writes) comes from the `Acquire` on the `turn_display.load()` — that's the actual moment you gain access. The ticket grab just needs to be atomic so no two cores get the same number.

### Acquire-Release Pairing

These two orderings are designed to work together. When Thread 1 releases a lock with `Release` and Thread 2 acquires it with `Acquire`, a **happens-before** relationship is established between them:

Every write Thread 1 made before its `Release` is visible to Thread 2 after its `Acquire`.

This is the guarantee that makes locks correct. The data Thread 1 wrote inside its critical section is visible to Thread 2 when Thread 2 enters its critical section. No data can be "left behind" in a CPU cache and invisible to the next holder.

---

## The Atomic Operation Methods

A quick reference for the three atomic methods used in the lock implementations.

### `.swap(new_value, ordering)`

Atomically writes `new_value` and returns the **old** value that was there before. This is Test-And-Set. Used in `SpinLock.lock()`.

```ddos/Notes/locks(phase 2).md#L1-1
let old = atomic_bool.swap(true, Ordering::Acquire);
// old == false: lock was free, we now hold it
// old == true: lock was already held, keep spinning
```

### `.compare_exchange_weak(expected, new, success_ord, fail_ord)`

Atomically: if current value equals `expected`, replace with `new` and return `Ok(expected)`. If current value does not equal `expected`, do nothing and return `Err(current)`. The `_weak` variant can spuriously return `Err` even if the values match — always use inside a loop. Used in `CasLock.lock()`.

```ddos/Notes/locks(phase 2).md#L1-1
// "if false, set to true"
let result = atomic_bool.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed);
// Ok(false): it was false, we changed it to true, lock acquired
// Err(true): it was true, we did nothing, retry
```

### `.fetch_add(amount, ordering)`

Atomically adds `amount` to the value and returns the **old** value. Used in `TicketLock.lock()` (take a ticket) and `TicketLock.unlock()` (advance the display).

```ddos/Notes/locks(phase 2).md#L1-1
let my_ticket = ticket_counter.fetch_add(1, Ordering::Relaxed);
// my_ticket = what the counter was before we incremented it
// ticket_counter is now my_ticket + 1 (for the next core)
```

---

## What Changed in the Rest of the Codebase

### `src/memory/heap.rs` — Swapping Out the Fake Lock

In Phase 1, `heap.rs` used `Locked<FreeList>` — the fake interior mutability wrapper that had no actual lock. The import at the top of the file and the `GlobalAlloc` implementation were the only things that changed.

**Before (Phase 1):**
```ddos/Notes/locks(phase 2).md#L1-1
use super::super::utils::locked::Locked;

unsafe impl GlobalAlloc for Locked<FreeList> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = self.lock();
        // ...
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let allocator = self.lock();
        // ...
    }
}
```

**After (Phase 2):**
```ddos/src/memory/heap.rs#L1-5
use super::super::utils::locked::SpinLock;

unsafe impl GlobalAlloc for SpinLock<FreeList> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
```

Two changes:

1. `Locked` becomes `SpinLock`. The import swaps. The `impl GlobalAlloc for ...` line changes the type.

2. `let allocator` becomes `let mut allocator`. In Phase 1, `Locked::lock()` returned `&mut FreeList` directly — already mutable, no need for `mut` on the binding. Now `SpinLock::lock()` returns a `SpinLockGuard<T>`. To call mutable methods on the inner `T`, Rust needs the guard binding itself to be `mut` so it can call `deref_mut()`. If you forget `mut`, the compiler tells you `.allocate()` needs `&mut self` but you only have `&self`. The `mut` on the binding is what enables `DerefMut`.

### `src/memory/mod.rs` — ALLOCATOR Gets a Real Lock

```ddos/src/memory/mod.rs#L1-20
pub mod config;
pub mod heap;

use core::alloc::Layout;

use super::utils::locked::SpinLock;
use config::{HEAP_SIZE, HEAP_START};
use heap::{FreeList, HeapType};

#[global_allocator]
static ALLOCATOR: SpinLock<FreeList> = SpinLock::new(FreeList {
    head: None,
    start_address: 0,
    capacity: 0,
    heap_type: HeapType::BestFit,
    next_fit_cursor: None,
});

pub fn init() {
    unsafe {
        let mut allocator = ALLOCATOR.lock();
        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }
}
```

`static ALLOCATOR: SpinLock<FreeList>` — same story as before. `ALLOCATOR` is a `static`, meaning it lives for the entire program lifetime and can be accessed from anywhere. The type changed from `Locked<FreeList>` to `SpinLock<FreeList>`.

`SpinLock::new()` is `const fn` — can be evaluated at compile time, which is required for `static` initialization. Everything inside still uses dummy values for the same reason as Phase 1: `FreeList::init()` does memory writes and can't run at compile time. Boot calls `memory::init()` to replace the dead struct with a real one.

`let mut allocator = ALLOCATOR.lock()` — the `mut` is required for the same reason as in `heap.rs`. The guard needs to be `mut` for `DerefMut` to kick in so we can call `*allocator = ...`.

### `src/drivers/uart.rs` — The UART Gets a Real Lock and Macros

```ddos/src/drivers/uart.rs#L1-10
use super::super::utils::locked::SpinLock;

pub static UART: SpinLock<Uart> = SpinLock::new(Uart::new());
```

`UART` is now a `SpinLock<Uart>` instead of a bare `Uart`. Before Phase 2, the UART had no protection at all — any code could call `uart.send()` directly and two things writing at the same time would produce garbled output. Now, all UART access must go through `.lock()`, which serializes access.

`const fn new()` on `Uart` — the `Uart` constructor is `const fn` because `SpinLock::new()` is `const fn`, and `const fn` can only call other `const fn`s. You can't call non-const functions in a context that must be evaluated at compile time.

The bigger change is the macros.

```ddos/src/drivers/uart.rs#L80-100
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let mut uart = crate::drivers::uart::UART.lock();
    let _ = uart.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::drivers::uart::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}
```

Before this, every print required: `uart.lock().write_str("something")` or similar — awkward and verbose. The macros give us Rust's standard `println!` interface.

`#[doc(hidden)]` on `_print` — the underscore prefix and this attribute mark `_print` as an implementation detail. It's `pub` because macros from other modules need to call it (macros expand at the call site, not in `uart.rs`), but it's not meant to be called directly. Hiding it from documentation keeps the public API clean.

`format_args!($($arg:tt)*)` — `format_args!` is a compiler built-in (not a normal macro) that takes a format string and arguments and produces a `core::fmt::Arguments` value. This is a lazy format — it doesn't allocate a `String`, it just holds references to the format string and the argument values. The actual formatting happens later when `write_fmt` is called. This means `format_args!` works in `no_std` where you have no heap and can't build a `String`.

`$($arg:tt)*` — macro syntax for "zero or more token trees." A token tree is roughly "any Rust token or any group of tokens in brackets." This matches anything: `"hello"`, `"value: {}", x`, `"{:?}", some_struct` — any valid format string and arguments.

`#[macro_export]` — makes the macro available at the crate root. After this, `print!` and `println!` work from anywhere in the codebase as `crate::print!("...")` or, once brought into scope, just `println!(...)`.

`$crate::print!("\n")` — `$crate` in a macro context refers to the crate that defined the macro. Using `$crate` instead of `crate` is the correct way to reference your own crate from inside a macro, because macros are expanded at the call site, not the definition site. Using just `crate` would refer to whatever crate is using the macro, which is wrong.

### `src/main.rs` — Using It All

```ddos/src/main.rs#L18-40
pub extern "C" fn _main() -> ! {
    drivers::uart::UART.lock().init();

    println!("\n[KERNEL] Booting DDOS...");

    memory::init();

    println!("[KERNEL] Heap Initialized.");
    println!("Welcome to DDOS Kernel v0.1");

    let heap_val = Box::new(42);
    println!("- Box allocated at {:p}, value: {}", heap_val, *heap_val);
```

`drivers::uart::UART.lock().init()` — the UART initialization is now done through the lock. We lock, get a guard (a `SpinLockGuard<Uart>`), and call `.init()` on it. The guard's `Deref` impl makes `uart_guard.init()` work as if we had a plain `&mut Uart`. When the temporary guard goes out of scope at the end of this statement, `Drop` releases the lock automatically.

`println!` everywhere — this is the macro from `uart.rs`. Each `println!` call goes to `_print()`, which locks the UART, formats the string, writes it, and unlocks. The lock and unlock are completely invisible to the call site — that's the point.

The heap test (`Box::new(42)`, `Vec::new()`) works now not just because the heap is initialized, but because the heap's global allocator is a real `SpinLock<FreeList>` instead of an unprotected `Locked<FreeList>`. Every `Box::new()` call locks the allocator, does the allocation, and releases the lock — all through the `GlobalAlloc` implementation in `heap.rs`.

---

## The Whole Picture

The call chain for `println!("hello")`:

```ddos/Notes/locks(phase 2).md#L1-1
println!("hello")
  └─ expands to: crate::drivers::uart::_print(format_args!("hello"))
       └─ _print(args):
            └─ UART.lock()
                 └─ [on rpi5] disable IRQs, check bool, set to true, return guard
                 └─ [on qemu] AtomicBool.swap(true, Acquire) spin until false→true
            └─ guard.write_fmt(args)
                 └─ DerefMut gives &mut Uart
                      └─ Uart::write_str("hello")
                           └─ for each char: poll FR register, write to DR register
            └─ guard goes out of scope → Drop runs → UART.unlock()
                 └─ [on rpi5] set bool to false, restore IRQ state
                 └─ [on qemu] AtomicBool.store(false, Release)
```

The call chain for `Box::new(42)`:

```ddos/Notes/locks(phase 2).md#L1-1
Box::new(42)
  └─ Rust runtime calls GlobalAlloc::alloc(&ALLOCATOR, layout)
       └─ ALLOCATOR.lock() → SpinLockGuard<FreeList>
            └─ [locks the heap - same spinlock mechanism as above]
       └─ guard.allocate(layout.size(), layout.align())
            └─ DerefMut gives &mut FreeList
                 └─ BestFit search, split, write headers/footers, return pointer
       └─ guard goes out of scope → Drop → ALLOCATOR.unlock()
```

---

## What's Still Missing

**No yield.** Threads waiting on a lock spin and burn CPU. The scheduler doesn't exist yet.

**No sleep.** No wait queues, no wakeup mechanism. Every lock is a spinlock.

**No multi-core safety on Pi 5.** The interrupt-disable trick works on Core 0 only. Cores 1–3 are not started and the heap/UART are only protected against single-core preemption.

**`CasLock` and `TicketLock` are QEMU-only.** They use `AtomicBool`/`AtomicUsize` without the interrupt workaround. On real hardware before the MMU is on, they would encounter the same Exclusive Monitor issue as the original `AtomicBool` implementation.

When the MMU comes online in Phase 4, the Pi 5 memory becomes cacheable. The Exclusive Monitor wakes up. The `#[cfg(feature = "rpi5")]` split in `SpinLock` becomes unnecessary — we delete the `UnsafeCell<bool>` path and the `daif` assembly entirely and go back to clean, uniform `AtomicBool` code that works correctly on all four cores simultaneously.

---

## OSTEP Reading Notes

---

### Chapter 26 — Concurrency: An Introduction

Chapter 26 opens with a question: why is concurrent programming hard? The answer is that multiple threads sharing state creates problems that simply don't exist in single-threaded code.

The chapter introduces threads as independent execution contexts inside the same process — they share code, heap, and global variables, but each has its own stack and registers (its own CPU state). This is different from processes, which have completely separate address spaces.

The book then demonstrates the race condition with a counter increment example, showing that the three-step assembly sequence (load, add, store) can be interrupted between any two steps by a context switch. This is the example I summarized at the top of these notes. It's worth dwelling on: the bug only happens in the narrow window between two specific instructions. That's milliseconds out of a program that might run for hours. These bugs are rare in testing and devastating in production.

**The key terminology:**

A **critical section** is code that accesses shared state and must not be executed concurrently. A **race condition** occurs when multiple threads enter a critical section simultaneously. **Indeterminate** execution means the output of a program depends on scheduling, which varies run to run.

The chapter introduces **mutual exclusion** — the property that only one thread is in a critical section at a time — as the solution, and sets up the need for a primitive to achieve it. That primitive is the lock.

One thing Chapter 26 emphasizes that I found useful: the problem isn't threads sharing data per se. The problem is threads sharing **mutable** data. If data never changes after initialization, any number of threads can read it simultaneously without any issues. Locks are only needed for mutation.

---

### Chapter 28 — Locks

Chapter 28 is the main chapter. It starts by defining what a lock is at the interface level: a variable with two states (locked, unlocked) and two operations (acquire, release). Simple. Then it asks: how do you actually implement this in hardware?

**28.3 — Controlling Interrupts**

The first approach: on a single processor, disable interrupts before entering the critical section, re-enable after. Chapter 28 is honest about the limitations — it only works on one CPU, it's dangerous to give to user programs, and it can drop hardware events. But it works for simple kernel code on a single core, and that's us, right now.

**28.5 — Test-And-Set**

The book formalizes Test-And-Set as the atomic primitive and shows it builds a working spinlock. The Rust `.swap()` method is this. Then it critiques spinlocks: they work on single CPUs only if a preemptive scheduler exists (which can swap out the spinning thread), and on multiprocessors they waste CPU cycles.

**28.6 — Compare-And-Swap**

Described as strictly more powerful than Test-And-Set. CAS can build lock-free data structures that TAS cannot. For simple locks they're equivalent. OSTEP mentions that on x86 this is one instruction (`cmpxchg`). On ARM it's the `ldaxr`/`stlxr` pair — the same instruction pair used for Test-And-Set. Both primitives map to the same hardware mechanism on ARM.

**28.7 — Load-Linked / Store-Conditional**

The book presents LL/SC as an alternative to CAS on architectures that don't have a native CAS instruction. ARM is the most prominent such architecture. OSTEP shows how you write a lock directly in LL/SC pseudocode. As established, Rust's atomics abstract this away — writing `.compare_exchange()` on ARM produces `ldaxr`/`stlxr` automatically.

**28.9 — Fetch-And-Add**

The book presents Fetch-And-Add as the primitive behind ticket locks and emphasizes the fairness argument. The deli counter analogy is theirs, not mine — it's in the book almost word for word. Fetch-And-Add eliminates starvation by making the waiting order explicit and deterministic.

**28.11 — A Simple Approach: Just Yield, Baby**

The first "stop spinning" approach. Instead of burning cycles, call `yield()` to give up the CPU. Chapter 28 shows that even this is not ideal: under high contention with many threads, you still get O(N) context switches per lock release as every waiting thread wakes up, checks the lock, and goes back to sleep. It's better than pure spinning, but it has cost.

**28.12 — Using Queues: Sleeping Instead of Spinning**

The real solution. The lock maintains a queue of sleeping threads. When you can't acquire the lock, you add yourself to the queue and sleep. When the holder unlocks, it picks the next thread from the queue and wakes it. Zero wasted CPU cycles. Zero context switch storms. The trade-off is complexity — you need a thread abstraction, a sleep system call, a wakeup system call, and a queue data structure all working correctly. 

This is where we can't go yet. We don't have threads. We don't have a scheduler. We don't have `sleep()`. All of this comes in later phases.

---

### Chapter 29 — Lock-Based Concurrent Data Structures

Chapter 29 applies locks to real data structures and discusses the trade-offs. A few things are worth noting for our kernel.

**Concurrent counters:** The naive approach is one lock for the counter. Under high contention on a multiprocessor, this becomes a bottleneck. All threads serialize through the single lock. The chapter presents **approximate counters** — each CPU has its own local counter that it increments without locking, and a global counter that gets updated periodically. Much faster. Slightly approximate.

**Concurrent linked lists:** Wrapping an entire list in one lock is simple but kills concurrency — only one thread can traverse or modify the list at a time. **Hand-over-hand locking** puts a separate lock on each node. Threads can traverse different parts of the list concurrently. Acquisition is slow (you lock every node as you pass through it) but concurrency is maximum.

**Concurrent hash tables:** One lock per bucket. Operations on different buckets proceed in parallel. This is the practical middle ground — better concurrency than one global lock, simpler than per-node locking.

What Chapter 29 is really teaching is that "add a lock" is not a performance strategy. The granularity of locking (one big lock vs. one lock per piece of data) has enormous performance implications. A global heap lock (which is what our allocator has) is fine for a simple kernel. A serious OS like Linux uses per-size-class slab locks so different CPU cores can allocate from different size classes simultaneously.

For now, one lock around the entire heap is correct and fast enough. When contention becomes a real problem — when we have multiple cores and real workloads — the data from Chapter 29 tells you exactly what to reach for.

---

_DDOS Kernel — Daksh Desai_
_Phase 2 complete. Locks are live. The heap is finally protected._