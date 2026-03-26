# Processes + Clock Tick (Phase 3 Part 1)

**OSTEP Chapters:** 4, 5, 6 (core for this phase: 6)
**What I built in this phase:**

1. Process abstraction data structures (`Process`, `ProcessState`, `CpuContext`)
2. Real context switch in AArch64 assembly (`cpu_switch_to`)
3. Exception/interrupt vector setup in assembly (`boot.s`, `vectors.s`)
4. Timer interrupt pipeline (hardware timer -> interrupt controller -> kernel handler)
5. Kernel boot/EL setup logic that makes interrupt delivery possible

**Files covered (as requested):**

1. `src/cpu/boot.s`
2. `src/cpu/vectors.s`
3. `src/cpu/switch.s`
4. `src/cpu/exception.rs`
5. `src/cpu/process.rs`
6. `src/cpu/mod.rs`
7. `src/drivers/interrupt.rs`
8. `src/drivers/timer.rs`

---

## Big Picture First

This phase is where the kernel starts behaving like an OS instead of just a Rust program on bare metal.

Before this phase:

1. We could boot.
2. We could print.
3. We could allocate memory.
4. We could lock critical sections.

After this phase:

1. We have process-like execution contexts.
2. We can switch CPU state between tasks.
3. We can receive asynchronous hardware interrupts.
4. We can configure a periodic timer tick.
5. We have the mechanism required for preemption (full scheduler comes next).

This is exactly the bridge from OSTEP Chapters 4/5 (process abstraction and API) to Chapter 6 (timer interrupts and limited direct execution).

---

## OSTEP Chapter 4 Theory: The Process Abstraction

Chapter 4 introduces one of the most important ideas in OS design:

**A process is an abstraction.**

A process is not "just code". It is:

1. Program code + data.
2. Its own stack/heap view.
3. CPU register state (PC, SP, general registers, flags).
4. Execution state (ready/running/blocked/etc).

The key insight: the OS gives each program the illusion that it owns the CPU, while the CPU is actually multiplexed across many processes.

### Mapping to this codebase

1. `src/cpu/process.rs` defines `ProcessState`, `CpuContext`, and `Process`.
2. `CpuContext` stores callee-saved registers + `sp` so execution can be paused/resumed.
3. `ProcessState` mirrors textbook states (`Ready`, `Running`, `Blocked`, `Dead`).
4. `Process::new()` allocates a private stack (`Vec<u8>`), representing per-process memory separation at this stage.

### Why this matters

Without saved register context, a "process switch" is fake.
With `CpuContext` + `cpu_switch_to`, process switching becomes real CPU state switching.

---

## OSTEP Chapter 5 Theory: Process API

Chapter 5 explains what an OS typically exposes:

1. Create a process (`fork`-like).
2. Load a program (`exec`-like).
3. Wait/synchronize (`wait`-like).
4. Kill/exit (`exit`-like).
5. Query process metadata (`pid`, parent, state).

This phase does not implement full Unix syscalls yet, but it builds the kernel-side prerequisites.

### What is already implemented that aligns with Chapter 5

1. Process identity fields: `pid`, `parent_pid`.
2. Lifecycle tracking: `ProcessState` enum.
3. Memory ownership per process: `stack` buffer.
4. Execution context storage: `context: CpuContext`.
5. Context-switch primitive callable from Rust: `cpu_switch_to(...)`.

### What is not yet implemented (next phases)

1. True address-space isolation (MMU per process).
2. `fork/exec/wait` syscall layer.
3. Timer-driven scheduler with run queue.
4. User mode transitions with full trap return paths.

So Chapter 5 theory is represented structurally, but the API surface to user programs is not exposed yet.

---

## OSTEP Chapter 6 Theory: Limited Direct Execution + Timer Interrupts

Chapter 6 answers a critical question:

If programs run directly on CPU for performance, how does the OS regain control?

Answer:

1. Boot and privileged setup are done by OS.
2. Hardware timer is programmed to fire periodically.
3. On timer interrupt, CPU traps into kernel handler.
4. Kernel can account time, schedule, preempt, and return.

This is called **Limited Direct Execution (LDE)**:

1. "Direct" because normal code runs on hardware directly.
2. "Limited" because hardware traps transfer control back to kernel at safe boundaries (interrupts/exceptions/syscalls).

### Full clock-tick path in this project

1. `boot.s` sets exception vector base and unmasks interrupts.
2. `timer::init()` programs ARM virtual timer registers.
3. `interrupt::init()` unmasks timer IRQ in the interrupt controller.
4. Timer expires -> interrupt controller signals CPU.
5. CPU jumps to vector in `vectors.s`.
6. `vectors.s` saves register context and calls Rust `handle_irq`.
7. `exception.rs` forwards to `drivers::interrupt::handle_irq()`.
8. `drivers::interrupt` checks IRQ number and calls `timer::handle_tick()`.
9. `timer::handle_tick()` logs tick and rearms timer (`reset()`).
10. Control returns back through vector epilogue (`eret`).

That is exactly the Chapter 6 mechanism needed before implementing a real preemptive scheduler.

---

## Security Levels and Privilege Theory (EL3/EL2/EL1)

You mentioned "security levels" and assembly changes, so this is the core theory.

AArch64 has Exception Levels (EL):

1. `EL3` = highest privilege (secure monitor / TrustZone world control)
2. `EL2` = hypervisor level
3. `EL1` = kernel level
4. `EL0` = user level

Each EL can have its own vector base register (`VBAR_ELx`) and control registers.

### What your boot code does

In `boot.s`, code reads `CurrentEL` and configures vectors for the current level:

1. If running at EL1: writes `VBAR_EL1`.
2. If running at EL2: writes `VBAR_EL2`, then updates `HCR_EL2` bits for IRQ/FIQ routing behavior.
3. If running at EL3: writes `VBAR_EL3`, then updates `SCR_EL3` bits used for exception routing policy.

This is a practical boot-time compatibility step because firmware/hardware may start kernel at different ELs.

### Why this matters for clock ticks

Timer interrupts only work if:

1. Vector base for current EL is valid.
2. Interrupt masks are cleared (`daifclr`).
3. Interrupt controller route/unmask is configured.

If any one is wrong, timer hardware may fire but kernel never sees it.

---

## Why Changing the Last Loop Helped the Tick Work

You asked specifically about changing the final loop.

In `src/main.rs`, the final idle path is:

```rust
loop {
	unsafe {
		core::arch::asm!("nop");
	}
}
```

This is a **no-op loop** (not "no loop").

Why this can help versus an empty `loop {}` in bare-metal release builds:

1. It inserts a guaranteed instruction (`nop`) every iteration.
2. It avoids relying on compiler behavior for an empty non-returning loop.
3. It keeps the core in a predictable idle-spin state while interrupts remain enabled.

Important nuance:

1. `nop` does **not** enable interrupts; interrupts must already be unmasked.
2. The real interrupt-enabling step is in `boot.s`: `msr daifclr, #0b1111`.
3. Better long-term idle instruction is usually `wfi` (wait-for-interrupt), but `nop` is a valid bring-up step during debug.

So the clock tick "working" comes from the whole pipeline being correct, and the `nop` loop simply gives a stable idle point for observing recurring interrupts.

---

## CPU Folder Documentation

## `src/cpu/mod.rs`

Purpose:

1. Exposes CPU modules (`exception`, `process`).
2. Pulls in assembly files through `global_asm!`.
3. Declares `cpu_switch_to` as external symbol callable from Rust.

Why it exists:

Rust code needs to call assembly (`switch.s`) and assembly needs to call Rust handlers (`exception.rs`). This file is the bridge.

## `src/cpu/process.rs`

Purpose:

1. Defines process states.
2. Defines saved CPU context layout (`CpuContext`) in `#[repr(C)]` order.
3. Defines `Process` struct with metadata and owned stack.

Key design notes:

1. `#[repr(C)]` is essential because `switch.s` uses hardcoded field offsets (`#0`, `#16`, ...).
2. `x19..x30 + sp` are sufficient for cooperative context switching demo because caller-saved regs are not expected to survive call boundaries.
3. `x30` holds return address / entry function pointer in this setup.

## `src/cpu/switch.s`

Purpose:

1. Save callee-saved register set + stack pointer of current context.
2. Restore next context.
3. `ret` into the restored `x30` execution point.

Why this is "real" context switch:

After `sp` and register restore, CPU execution continues in a different logical thread of control with its own stack and return chain.

## `src/cpu/boot.s`

Purpose:

1. Core0 selection at boot.
2. BSS zeroing.
3. Exception vector base setup for current EL.
4. Interrupt unmasking (`daifclr`).
5. Jump into Rust `_main`.

Special details:

1. Secondary cores branch to `hang` loop.
2. `wfe` in `hang` is low-power wait, good for parked cores.

## `src/cpu/vectors.s`

Purpose:

1. Defines vector table with required alignment and 16 entry slots.
2. Implements EL1 IRQ/FIQ/SYNC handlers.
3. Saves/restores register frame around Rust handlers.
4. Returns with `eret`.

Why this is necessary:

Rust functions cannot be first-level vector table targets directly; assembly entry stubs are needed to preserve machine context correctly.

## `src/cpu/exception.rs`

Purpose:

1. Rust-side trap handler entry points (`handle_irq`, `handle_sync`).
2. Reads exception syndrome registers for sync faults (`esr_el1`, `far_el1`).
3. Delegates IRQ handling to interrupt driver.

Why panic path loops with `wfe`:

On unrecoverable synchronous exception, kernel halts safely instead of running corrupted state.

---

## Drivers Documentation (Interrupt + Timer)

## `src/drivers/interrupt.rs`

Purpose:

1. Program platform-specific interrupt controller.
2. Unmask timer IRQ source.
3. Dispatch IRQ to timer handler.

Platforms:

1. `rpi4/rpi5` path uses GIC Distributor/CPU interface registers.
2. `qemu/rpi3` path uses legacy local interrupt controller register.

Important behavior:

1. Reads active IRQ ID (`GICC_IAR`) and checks for IRQ 27 (timer in this setup).
2. Calls `timer::handle_tick()`.
3. Writes EOI (`GICC_EOIR`) to complete interrupt handling.

## `src/drivers/timer.rs`

Purpose:

1. Configure ARM virtual timer.
2. Program timeout interval with `cntv_tval_el0`.
3. Enable timer with `cntv_ctl_el0`.
4. Re-arm timer each tick.

Tick period logic now:

1. Reads `cntfrq_el0` (counter frequency ticks/sec).
2. Uses that value directly as next `cntv_tval_el0`.
3. Therefore period is roughly 1 second per interrupt.

---

## Line-by-Line Explanation (Chapter 6 Clock Tick Path)

You asked for line-by-line specifically for the clock tick mechanism and assembly/security changes. This section is exactly that.

## A) `src/cpu/boot.s` (interrupt enable + EL setup)

```asm
mrs     x1, CurrentEL
lsr     x1, x1, #2
```

1. Read current exception level encoding.
2. Shift right so `x1` becomes numeric EL value (`1`, `2`, or `3`).

```asm
cmp     x1, #3
b.eq    set_el3
cmp     x1, #2
b.eq    set_el2
```

1. Branch to EL-specific setup path.
2. Falls through to EL1 path if not EL2/EL3.

```asm
set_el1:
	msr     vbar_el1, x0
	b       continue
```

1. Write vector table base address into EL1 vector base register.
2. All EL1 exceptions/IRQs now jump into your `exception_vector_table`.

```asm
set_el2:
	msr     vbar_el2, x0
	mrs     x1, hcr_el2
	orr     x1, x1, #(1<<4)
	orr     x1, x1, #(1<<5)
	msr     hcr_el2, x1
	b       continue
```

1. Set EL2 vector table base.
2. Read `HCR_EL2`.
3. Set bit 4 and bit 5 in your current policy (IRQ/FIQ routing control under EL2 behavior).
4. Write back `HCR_EL2`.
5. Continue boot.

```asm
set_el3:
	msr     vbar_el3, x0
	mrs     x1, scr_el3
	orr     x1, x1, #(1<<1)
	orr     x1, x1, #(1<<2)
	msr     scr_el3, x1
	b       continue
```

1. Set EL3 vector table base.
2. Read `SCR_EL3` (secure configuration register).
3. Set bit 1 and bit 2 in your configuration (exception routing policy knobs at EL3).
4. Write back `SCR_EL3`.
5. Continue boot.

```asm
continue:
	msr     daifclr, #0b1111
	bl      _main
```

1. Clear all mask bits in `DAIF` (`D/A/I/F`) to unmask debug, SError, IRQ, FIQ.
2. Call Rust `_main`.

This `daifclr` is mandatory for timer tick visibility. If IRQ mask stays set, timer can expire forever and no handler runs.

## B) `src/cpu/vectors.s` (IRQ entry/exit frame)

```asm
.align 11
exception_vector_table:
```

1. Align vector table to architecture-required boundary.
2. Label start of table.

```asm
ventry el1_irq
```

1. Table entry branch target for EL1 IRQ case.

```asm
el1_irq:
	kernel_entry
	bl handle_irq
	kernel_exit
```

1. `kernel_entry` macro pushes x0-x18 and x30 onto current stack.
2. Calls Rust `handle_irq` symbol.
3. `kernel_exit` pops saved registers and returns with `eret`.

`eret` is important: it restores PC/PSTATE from exception link registers and resumes interrupted context cleanly.

## C) `src/cpu/exception.rs` (Rust trap bridge)

```rust
#[unsafe(no_mangle)]
pub extern "C" fn handle_irq() {
	crate::drivers::interrupt::handle_irq();
}
```

1. `no_mangle` ensures symbol name is exactly `handle_irq` for assembly `bl handle_irq`.
2. `extern "C"` ABI match with assembly call convention.
3. Body delegates to interrupt driver logic.

## D) `src/drivers/interrupt.rs` (controller setup + dispatch)

### Init path (`rpi4`/`rpi5`)

```rust
let gicd_ctlr = hardwareselect::GICD_BASE as *mut u32;
core::ptr::write_volatile(gicd_ctlr, 3);
```

1. Pointer to GIC Distributor control register.
2. Enable distributor paths according to current bitmask configuration.

```rust
let gicd_igroupr0 = (hardwareselect::GICD_BASE + 0x080) as *mut u32;
core::ptr::write_volatile(gicd_igroupr0, 0xFFFF_FFFF);
```

1. Configure interrupt group assignment bits for first 32 interrupts.
2. Current code places all in group mask used by your setup.

```rust
let gicd_isenabler0 = (hardwareselect::GICD_BASE + 0x100) as *mut u32;
core::ptr::write_volatile(gicd_isenabler0, 1 << 27);
```

1. Enable interrupt ID 27.
2. This is your timer IRQ source in current configuration.

```rust
let gicc_pmr = (hardwareselect::GICC_BASE + 0x004) as *mut u32;
core::ptr::write_volatile(gicc_pmr, 0xFF);
```

1. Set CPU interface priority mask threshold to allow normal priorities.

```rust
let gicc_ctlr = hardwareselect::GICC_BASE as *mut u32;
core::ptr::write_volatile(gicc_ctlr, 3);
```

1. Enable CPU interface so pending enabled IRQs are signaled to core.

### IRQ dispatch path

```rust
let gicc_iar = (hardwareselect::GICC_BASE + 0x00C) as *mut u32;
let irq = core::ptr::read_volatile(gicc_iar) & 0x3FF;
```

1. Read Interrupt Acknowledge Register (claims active interrupt).
2. Mask low 10 bits to extract interrupt ID.

```rust
if irq == 27 {
	crate::drivers::timer::handle_tick();
}
```

1. Route only timer IRQ in this phase.

```rust
let gicc_eoir = (hardwareselect::GICC_BASE + 0x010) as *mut u32;
core::ptr::write_volatile(gicc_eoir, irq);
```

1. Write End-Of-Interrupt with same ID.
2. Tells controller handling is complete and allows next interrupt.

Without EOI, same IRQ can remain active/pending and break periodic behavior.

## E) `src/drivers/timer.rs` (program + rearm virtual timer)

```rust
pub fn init() {
	reset();
	println!("[TIMER] ARM64 Virtual Timer initialized.");
}
```

1. Initialize timer by programming first expiry.
2. Log confirmation.

```rust
let mut frequency: u64;
asm!("mrs {}, cntfrq_el0", out(reg) frequency);
```

1. Read generic timer frequency register.
2. Example meaning: if value is 1,000,000,000, one second is 1e9 ticks.

```rust
let ticks = frequency;
asm!("msr cntv_tval_el0, {}", in(reg) ticks);
```

1. Set countdown compare value for virtual timer.
2. Here, next interrupt is after `frequency` ticks, about 1 second.

```rust
let ctl: u64 = 1;
asm!("msr cntv_ctl_el0, {}", in(reg) ctl);
```

1. Enable virtual timer (`ENABLE=1` in control register policy used here).

```rust
pub fn handle_tick() {
	println!("\n[KERNEL] TICK! Hardware Timer Fired!");
	reset();
}
```

1. Tick handler prints message.
2. Immediately rearms next tick by calling `reset()`.

This is why ticks keep repeating instead of firing once.

## F) `src/main.rs` final loop (why no-op loop)

```rust
loop {
	unsafe {
		core::arch::asm!("nop");
	}
}
```

1. Keep kernel alive forever after demo task switch returns.
2. Execute `nop` continuously as explicit idle spin.
3. Interrupts can still preempt this loop because IRQ mask was cleared in boot.

If this loop exits, `_main` would return into invalid flow for a `-> !` kernel entry.

---

## What This Phase Proves

1. You can boot and configure vectors at the active privilege level.
2. You can unmask interrupts and receive hardware timer IRQs.
3. You can traverse full IRQ path from assembly entry to Rust handler and back.
4. You can preserve/restore CPU context for task switching.
5. You now have the exact Chapter 6 foundation required for adding a preemptive scheduler next.

---

## Quick Revision Cheatsheet

1. Chapter 4: Process = running program + register context + memory + state.
2. Chapter 5: Process API needs create/exec/wait/exit; this phase builds kernel internals first.
3. Chapter 6: Timer interrupt is how OS regains control for preemption.
4. `boot.s`: setup VBAR + clear DAIF masks.
5. `vectors.s`: save regs -> call Rust handler -> `eret`.
6. `interrupt.rs`: unmask + dispatch + EOI.
7. `timer.rs`: set compare value + enable + rearm each tick.
8. Final no-op loop keeps CPU alive while interrupts continue firing.
