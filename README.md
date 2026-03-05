# DDOS (Daksh's Desai OS) — Learn OS by Building a Small OS in Rust

This project is a learning-focused operating system kernel written in Rust.
The goal is to understand core OS concepts by implementing them directly:

- boot flow and low-level startup
- memory layout and heap allocation
- hardware-specific builds (QEMU / Raspberry Pi targets)
- kernel-safe Rust patterns in a `no_std` environment

## Why this project exists

This repo is for learning operating systems hands-on instead of only reading theory.
The approach is:

1. Study concepts from OSTEP
2. Implement a minimal version in kernel code
3. Document what was learned in `Notes/`

## Repository guide

- `src/` — kernel source code
  - `src/main.rs` — kernel entry and init flow
  - `src/memory/` — memory config + allocator implementation
  - `src/drivers/` — basic device drivers (e.g., UART)
  - `src/cpu/` — architecture-specific boot/startup code
- `scripts/` — helper scripts to build/run for specific hardware
- `link.ld` — linker script
- `QUICKSTART.md` — quick build/run instructions
- `Notes/` — learning notes mapped to implemented phases

## Notes index

### Notes/heap(phase 1).md

Click here: [Click me to open this note](Notes/heap%28phase%201%29.md)

This note explains the Phase 1 heap allocator work and the OS concepts behind it.
It includes:

- why free-space management is needed
- heap memory layout used in this kernel
- allocator internals (`FreeList`, split, coalesce, boundary tags)
- allocation strategies (First Fit, Best Fit, Worst Fit, Next Fit)
- Rust integration details (`#[global_allocator]`, `GlobalAlloc`, `Locked<T>`)
- OSTEP chapter takeaways (13, 14, 15, 17)

### Notes/locks(phase 2).md

Click here: [Click me to open this note](Notes/locks%28phase%202%29.md)

This note explains the Phase 2 lock implementations and the OS concepts behind them.
It includes:

- why race conditions exist and what mutual exclusion means
- the oldest approach: disabling hardware interrupts (OSTEP 28.3)
- all four hardware atomic primitives (Test-And-Set, Compare-And-Swap, LL/SC, Fetch-And-Add)
- why yield and sleep are not implemented yet (no scheduler)
- why `AtomicBool` crashes on real Pi 5 hardware (MMU off, Exclusive Monitor limitation)
- the interrupt-disable workaround and when it gets replaced (Phase 4: Paging)
- all three lock implementations (`SpinLock`, `CasLock`, `TicketLock`) with full syntax breakdown
- memory ordering explained (`Acquire`, `Release`, `Relaxed`) and why it matters
- `unsafe impl Sync` and `unsafe impl Send` explained
- what changed in `heap.rs`, `mod.rs`, `uart.rs`, and `main.rs`
- `print!` and `println!` macro internals
- OSTEP chapter takeaways (26, 28, 29)

As more phases are added, new files in `Notes/` can follow the same format.

## OSTEP (book links)

Official OSTEP home page (free chapter PDFs):

- https://pages.cs.wisc.edu/~remzi/OSTEP/

Official downloadable all-in-one PDF page:

- https://pages.cs.wisc.edu/~remzi/OSTEP/book-electronic.html

## Hardware target features

Build with exactly one hardware feature:

- `qemu`
- `rpi3`
- `rpi4`
- `rpi5`
