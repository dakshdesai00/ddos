# DDOS: Learning Operating System in Rust

A minimal operating system kernel built in Rust to understand how operating systems work. Following **"Operating Systems: Three Easy Pieces" (OSTEP)** by Remzi and Andrea Arpaci-Dusseau, this project implements OS concepts from first principles.

## 🎯 Project Goal

Build a learning OS from scratch that demonstrates:

- **Memory Management**: Virtual memory, address translation, heap allocation
- **Concurrency & Safety**: Locks, synchronization primitives, race condition prevention
- **CPU Scheduling**: Process creation, context switching, scheduling algorithms
- **Storage & Protection**: Paging, MMU, file systems, disk I/O
- **And more**: Threads, events, persistence, distributed concepts

## ✅ Current Status: Phase 1 Complete ✅

**Phase 1: Memory Management** is fully implemented and documented.

### What's Working Now:

- ✅ **Custom Heap Allocator** - Linked List Free Space Management
  - 4 allocation strategies: FirstFit, BestFit (active), WorstFit, NextFit
  - Block splitting & coalescing to prevent fragmentation
  - 8-byte alignment for efficient memory usage
  - Size: 2MB heap at address `0x280000`

- ✅ **Drivers** - Basic I/O functionality
  - UART driver (serial console)
  - Mailbox driver (ARM firmware communication)
  - Framebuffer driver (coming soon)

- ✅ **Multi-Platform Support** - Compile-time hardware selection
  - QEMU ARM emulator
  - Raspberry Pi 3
  - Raspberry Pi 4

- ✅ **Build System** - Automated workflows
  - Feature flags for hardware selection
  - Build scripts for QEMU and RPi4
  - Cargo integration

## 📚 Understanding Phase 1

For a **complete deep-dive** into Phase 1 memory management, including:

- Detailed explanation of each memory file (config.rs, heap.rs, mod.rs, locked.rs)
- All 4 allocation algorithms with step-by-step examples
- Why boilerplate code exists in mod.rs
- Understanding interior mutability and the Locked<T> wrapper
- Memory layout diagrams and fragmentation prevention

👉 **See [Phase1.md](./Documentation/Phase1.md)** ← Read this for comprehensive documentation

## 🚀 Quick Start

### Build for QEMU (default)

```bash
cd /Users/dakshdesai/Codes/rust-os/ddos
./scripts/build-qemu.sh
```

### Build for Raspberry Pi 4

```bash
./scripts/build-rpi4.sh
```

See [scripts/README.md](./scripts/README.md) for detailed build instructions.

## 📁 Project Structure

```
.
├── src/
│   ├── main.rs                 # Kernel entry point
│   ├── hardwareselect.rs       # Platform abstraction layer
│   ├── memory/
│   │   ├── config.rs           # Memory layout constants
│   │   ├── heap.rs             # FreeList allocator implementation
│   │   └── mod.rs              # GlobalAlloc integration & init
│   ├── drivers/
│   │   ├── uart.rs             # Serial console
│   │   ├── mailbox.rs          # ARM firmware interface
│   │   ├── framebuffer.rs      # GPU output (future)
│   │   └── mod.rs              # Driver module exports
│   └── utils/
│       ├── locked.rs           # Interior mutability wrapper
│       ├── font.rs             # Console font (future)
│       └── mod.rs              # Utils module exports
├── Documentation/
│   └── Phase1.md               # **READ THIS** - Complete Phase 1 explanation
├── scripts/
│   ├── build-qemu.sh           # QEMU build automation
│   ├── build-rpi4.sh           # RPi4 build automation
│   └── README.md               # Detailed build instructions
├── Cargo.toml                  # Rust project config
├── link.ld                     # ARM linker script
└── README.md                   # This file
```

## 🗺️ Learning Roadmap: OSTEP Chapters

Follow this roadmap as you progress through the OS implementation. Each phase builds on previous work.

| Order | Phase         | Book Part      | Chapter | Title                                  | What You Will Code                               | Difficulty | Status     |
| ----- | ------------- | -------------- | ------- | -------------------------------------- | ------------------------------------------------ | ---------- | ---------- |
| 1     | 0. Setup      | Intro          | 1       | A Dialogue on the Book                 | Setup Rust Project & QEMU                        | ⭐         | ✅ Done    |
| 2     | 0. Setup      | Intro          | 2       | Introduction to Operating Systems      | Hello World (UART Driver)                        | ⭐         | ✅ Done    |
| 3     | 1. Memory     | Virtualization | 13      | The Abstraction: Address Spaces        | Define MemoryMap (Kernel vs User constants)      | ⭐         | ✅ Done    |
| 4     | 1. Memory     | Virtualization | 14      | Interlude: Memory API                  | Understand Stack vs Heap (Theory)                | 📖         | ✅ Done    |
| 5     | 1. Memory     | Virtualization | 15      | Mechanism: Address Translation         | Theory: Virtual to Physical mapping              | 📖         | ✅ Done    |
| 6     | 1. Memory     | Virtualization | 17      | Free-Space Management                  | Implement Linked List Allocator (Enable Vec/Box) | ⭐⭐⭐     | ✅ Done    |
| 7     | 1. Memory     | Virtualization | 16      | Segmentation                           | Read Only (ARM uses Paging)                      | ❌         | Skipped    |
| 8     | 2. Safety     | Concurrency    | 26      | Concurrency: An Introduction           | Theory: Race Conditions                          | 📖         | ⏳ Pending |
| 9     | 2. Safety     | Concurrency    | 28      | Locks                                  | Implement SpinLock<T> (AtomicBool)               | ⭐⭐       | ⏳ Pending |
| 10    | 2. Safety     | Concurrency    | 29      | Lock-based Data Structures             | Wrap Console/UART in SpinLocks                   | ⭐⭐       | ⏳ Pending |
| 11    | 3. The CPU    | Virtualization | 4       | The Abstraction: The Process           | Define struct Process (Registers x0-x30)         | ⭐⭐       | ⏳ Pending |
| 12    | 3. The CPU    | Virtualization | 5       | Interlude: Process API                 | Implement Context Switch (Assembly)              | ⭐⭐⭐     | ⏳ Pending |
| 13    | 3. The CPU    | Virtualization | 6       | Mechanism: Limited Direct Execution    | Implement Exception Vector Table (Interrupts)    | ⭐⭐⭐⭐   | ⏳ Pending |
| 14    | 3. The CPU    | Virtualization | 7       | Scheduling: Introduction               | Implement Round Robin Scheduler                  | ⭐⭐       | ⏳ Pending |
| 15    | 3. The CPU    | Virtualization | 8       | Scheduling: MLFQ                       | Add Priority Queues to Scheduler                 | ⭐⭐⭐     | ⏳ Pending |
| 16    | 3. The CPU    | Virtualization | 9       | Scheduling: Proportional Share         | Implement Lottery Scheduling (RNG)               | ⭐⭐       | ⏳ Pending |
| 17    | 3. The CPU    | Virtualization | 10      | Multiprocessor Scheduling              | Enable Multi-Core (SMP) on RPi4                  | ⭐⭐⭐⭐⭐ | ⏳ Pending |
| 18    | 4. Protection | Virtualization | 18      | Paging: Introduction                   | Define PageTable structs                         | ⭐⭐       | ⏳ Pending |
| 19    | 4. Protection | Virtualization | 19      | Paging: Faster Translations (TLBs)     | Add TLB Flush helpers in Assembly                | ⭐⭐       | ⏳ Pending |
| 20    | 4. Protection | Virtualization | 20      | Paging: Smaller Tables                 | Implement MMU Driver (Multi-Level Tables)        | ⭐⭐⭐⭐⭐ | ⏳ Pending |
| 21    | 4. Protection | Virtualization | 21      | Mechanism: Beyond Physical Memory      | Swap to Disk (Requires Disk Driver)              | 🚀         | ⏳ Pending |
| 22    | 4. Protection | Virtualization | 22      | Policies: Beyond Physical Memory       | Page Replacement Algorithms (LRU)                | 🚀         | ⏳ Pending |
| 23    | 4. Protection | Virtualization | 23      | The VAX/VMS Operating System           | Case Study (History)                             | 📖         | ⏳ Pending |
| 24    | 5. Threads    | Concurrency    | 27      | Interlude: Thread API                  | Implement kthread_create()                       | ⭐⭐⭐     | ⏳ Pending |
| 25    | 5. Threads    | Concurrency    | 30      | Condition Variables                    | Implement wait() and signal()                    | ⭐⭐⭐     | ⏳ Pending |
| 26    | 5. Threads    | Concurrency    | 31      | Semaphores                             | Implement Semaphore struct                       | ⭐⭐       | ⏳ Pending |
| 27    | 5. Threads    | Concurrency    | 32      | Common Concurrency Problems            | Theory: Deadlock Avoidance                       | 📖         | ⏳ Pending |
| 28    | 5. Threads    | Concurrency    | 33      | Event-based Concurrency                | Async/Await (Rust Futures)                       | 🚀         | ⏳ Pending |
| 29    | 5. Threads    | Concurrency    | 34      | Summary Dialogue on Concurrency        | Review                                           | 📖         | ⏳ Pending |
| 30    | 6. Storage    | Persistence    | 36      | I/O Devices                            | Review MMIO (UART/HDMI)                          | 📖         | ⏳ Pending |
| 31    | 6. Storage    | Persistence    | 37      | Hard Disk Drives                       | Read SD Card Specs (Theory)                      | 📖         | ⏳ Pending |
| 32    | 6. Storage    | Persistence    | 39      | Interlude: Files and Directories       | Define Inode & DirEntry Structs                  | ⭐⭐       | ⏳ Pending |
| 33    | 6. Storage    | Persistence    | 40      | File System Implementation             | Write VirtIO-Block Driver & SimpleFS             | ⭐⭐⭐⭐⭐ | ⏳ Pending |
| 34    | 6. Storage    | Persistence    | 41      | Locality and The Fast File System      | Optimize Inode placement                         | ⭐⭐⭐     | ⏳ Pending |
| 35    | 6. Storage    | Persistence    | 42      | Crash Consistency: FSCK and Journaling | Implement Journaling (Write-Ahead Log)           | ⭐⭐⭐⭐   | ⏳ Pending |
| 36    | 6. Storage    | Persistence    | 43      | Log-structured File Systems            | Theory: LFS                                      | 📖         | ⏳ Pending |
| 37    | 6. Storage    | Persistence    | 44      | Data Integrity and Protection          | Implement Checksums (CRC32)                      | ⭐⭐       | ⏳ Pending |
| 38    | 6. Storage    | Persistence    | 45      | Summary Dialogue on Persistence        | Review                                           | 📖         | ⏳ Pending |
| 39    | 7. Network    | Distribution   | 46      | Distributed Systems                    | Theory: Network Basics                           | 📖         | ⏳ Pending |
| 40    | 7. Network    | Distribution   | 47      | Sun's Network File System (NFS)        | Requires Network Driver                          | 🚀         | ⏳ Pending |
| 41    | 7. Network    | Distribution   | 48      | The Andrew File System (AFS)           | Theory: Caching                                  | 📖         | ⏳ Pending |
| 42    | 7. Network    | Distribution   | 49      | Summary Dialogue on Distribution       | Review                                           | 📖         | ⏳ Pending |
| 43    | 8. Bonus      | Virtualization | 50      | Virtual Machines                       | Run an OS inside your OS                         | 🚀         | ⏳ Pending |

**Legend:**

- ✅ `Done` - Implemented and tested
- ⏳ `Pending` - Not started
- ❌ `Skipped` - Not applicable (ARM uses paging, not segmentation)
- ⭐ to ⭐⭐⭐⭐⭐ - Difficulty estimate
- 📖 - Theory only (reading/understanding, no coding)
- 🚀 - Advanced/complex feature

## 📖 Key Concepts Learned So Far

### Phase 1: Memory Management

1. **Address Spaces** - Kernel vs User memory separation
2. **Virtual Memory** - Abstraction between logical and physical addresses
3. **Stack vs Heap** - Static vs dynamic memory allocation
4. **Free Space Management** - Linked list allocator with 4 strategies
5. **Fragmentation** - External vs internal, coalescing solution
6. **Interior Mutability** - Safe mutation through immutable references in Rust

## 🔍 Key Files Overview

### `src/memory/config.rs`

Defines all memory layout constants that the kernel uses. Nothing runs until these addresses are correct.

### `src/memory/heap.rs`

The core allocator - a FreeList that tracks free memory regions and implements 4 different search strategies.

### `src/memory/mod.rs`

Initializes the heap and integrates with Rust's GlobalAlloc trait to enable `Box::new()` and `Vec::push()`.

### `src/utils/locked.rs`

A wrapper enabling safe mutable access to global state in a single-threaded context using interior mutability.

### `src/hardwareselect.rs`

Hardware abstraction layer allowing the same code to run on QEMU, RPi3, and RPi4.

## 🛠️ Building & Testing

```bash
# Clean build for QEMU
cargo clean && cargo build --features qemu --target aarch64-unknown-none-softfloat

# Build for RPi4
cargo clean && cargo build --no-default-features --features rpi4 --target aarch64-unknown-none-softfloat

# Run on QEMU (requires QEMU ARM installed)
./scripts/build-qemu.sh
```

## 📚 Resources

- **OSTEP Book**: [Operating Systems: Three Easy Pieces](https://pages.cs.wisc.edu/~remzi/OSTEP/)
- **Raspberry Pi Documentation**: [RPi Bare-Metal Tutorial](https://github.com/raspberrypi/documentation)
- **Rust Documentation**: [Rust Book](https://doc.rust-lang.org/book/)
- **ARM64 Assembly**: [ARM64 Instruction Set Reference](https://developer.arm.com/documentation/dui0801/latest/)

## 📝 License

Educational/Learning project. Use freely for learning purposes.

---

**Next Steps:** Read [Phase1.md](./Documentation/Phase1.md) to understand memory management in detail, then tackle Phase 2: Concurrency & Synchronization!
