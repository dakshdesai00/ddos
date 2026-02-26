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
