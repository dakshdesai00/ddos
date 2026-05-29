# Notes/paging.md

This note covers paging theory from OSTEP (paging, TLBs, page table size, and “beyond physical memory”), then walks through how paging is implemented in this kernel, including the issues encountered and the fixes that were applied.

---

## 1) Paging theory (OSTEP summary)

### 1.1 Why paging exists
Paging solves the core problems of early memory management:
- **Protection**: Each process has its own virtual address space (VAS) and cannot access other processes or the kernel unless permitted.
- **Relocation**: Programs can be loaded anywhere in physical memory without changing their virtual addresses.
- **Fragmentation control**: With fixed‑size pages, the OS avoids **external fragmentation** (holes between segments).

**Trade‑off**: paging introduces **internal fragmentation** because allocations must be rounded up to page size.

### 1.2 Basic translation model
A virtual address splits into:
- **VPN** (Virtual Page Number)
- **offset** (byte index within the page)

The page table maps VPN → PFN (Physical Frame Number). The final physical address is:

```
physical = (PFN << page_bits) | offset
```

### 1.3 Page Table Entries (PTEs)
PTEs typically include:
- **PFN**: the base of the physical frame
- **valid** bit: whether the mapping exists
- **protection bits**: R/W/X, user/supervisor, etc.
- **accessed/dirty** bits: used by the OS to track usage (replacement)

### 1.4 The cost of page tables
A single‑level page table for a large address space can be enormous.
For example, with 48‑bit VAs and 4KB pages:
- VPN bits = 48 − 12 = 36
- entries = 2^36
- Even if each PTE is 8 bytes → 512GB of page table memory (impossible)

### 1.5 TLB (Translation Lookaside Buffer)
The TLB caches recent translations and avoids page‑table walks.
- **TLB hit**: fast translation
- **TLB miss**: hardware walk (or software trap on some architectures)

**Context switches** require:
- switching page tables (TTBR or equivalent)
- invalidating or tagging TLB entries (ASIDs or TLB flush)

### 1.6 Multi‑level page tables
Multi‑level page tables reduce memory overhead by allocating page‑table pages **on demand**.
- Break VPN into multiple indexes (e.g., L0 → L1 → L2 → L3)
- Only allocate a lower‑level table when a translation is actually needed

This trades memory for a few extra memory reads (hidden by TLB on hits).

### 1.7 Other page table designs
OSTEP covers several alternatives:
- **Inverted page tables**: one entry per physical frame, with hashing
- **Hashed page tables**: reduce size using hash buckets
- **Segmented paging**: segmentation at the top level, then paging within each segment
- **Huge pages**: larger page sizes for TLB efficiency

### 1.8 Beyond physical memory (paging to disk)
When RAM is full, the OS can **swap** pages to disk:
- **Demand paging**: only load a page when it is actually accessed
- **Page fault**: trap when an unmapped page is accessed, load it from disk

#### Replacement policies (OSTEP)
- **OPT**: replace page used farthest in the future (ideal, not implementable)
- **FIFO**: remove oldest page (simple, not always good)
- **LRU**: remove least‑recently‑used (good, hard to implement exactly)
- **Clock / Second‑Chance**: LRU approximation using accessed bits

#### Thrashing and working sets
- If the working set of a process doesn’t fit in RAM, it **thrashes**.
- OS needs to balance how many pages or processes are active.

### 1.9 Common failure modes
- **Unmapped address** → page fault / data abort
- **Wrong permissions** → access fault
- **Missing device mappings** → faults on MMIO
- **MMU enabled for the wrong EL** → virtual address treated as physical

---

## 2) How paging is implemented in this kernel

### 2.1 Physical frame allocator
File: `src/memory/frame.rs`
- `FRAME_ALLOCATOR` provides 4KB frames (`PAGE_SIZE = 4096`).
- Frames are zeroed on allocation (prevents data leaks).
- Free frames are tracked via an in‑RAM linked list (`FreeFrameNode`).

### 2.2 Kernel heap
Files: `src/memory/heap.rs`, `src/memory/mod.rs`
- A free‑list heap is created at a fixed region (`HEAP_START/HEAP_SIZE`).
- Used for dynamic structures (`Vec`, scheduling queues, etc.).

### 2.3 Page tables and mapping
File: `src/memory/pagetable.rs`
- `PageTable` is a 4KB table with 512 entries (4‑level structure).
- `map_page()` allocates intermediate tables on demand and installs a final PTE.
- `PageTable::new_process_table()` clones the kernel L0 entries for each process.

### 2.4 MMU setup
File: `src/memory/mmu.rs`
- **Identity maps** all RAM so the kernel can keep using physical addresses as VAs.
- Maps peripheral MMIO as **device memory** (uncached, strongly ordered).
- Programs:
  - `MAIR_EL1` for memory types
  - `TCR_EL1` for 4KB pages, 48‑bit VA
  - `TTBR0_EL1` for the page table root
  - `SCTLR_EL1.M = 1` to enable MMU

### 2.5 Per‑process page tables
File: `src/cpu/process.rs`
- Each process gets its own root page table (`ttbr0` in `CpuContext`).
- Context switch (`src/cpu/switch.s`) updates `TTBR0_EL1` and flushes the TLB.

### 2.6 Per‑process stacks using page frames
File: `src/cpu/process.rs`
- Stacks are allocated **from physical frames**, not from the kernel heap.
- Mapped into the process address space at a fixed high virtual region:
  - `PROCESS_STACK_TOP` in `src/memory/config.rs`
- `ProcessStack` owns its frames and frees them on drop.

---

## 3) Issues encountered and fixes applied

### 3.1 Stacks were in the kernel heap
**Problem**: process stacks lived inside the kernel heap, which breaks isolation.

**Fix**:
- Implemented `ProcessStack` to allocate page frames and map them into the process address space.
- Stack VA is stable and canonical (`PROCESS_STACK_TOP`).

### 3.2 MMU enabled in EL1 but QEMU started in EL2
**Symptom**: system stalled after the first tick.

**Root cause**: QEMU (`-M raspi3b`) boots in **EL2**. The kernel enabled MMU in **EL1**, but execution stayed in EL2, so the MMU wasn’t active for the running EL. The stack virtual address was treated as a physical address → fault.

**Fix**:
- Updated `src/cpu/boot.s` to transition **EL3 → EL2 → EL1**, configure `HCR_EL2.RW`, set `SP_EL1`, and return into EL1h via `eret`.

### 3.3 Fault on local interrupt controller
**Symptom**: data abort with `FAR_EL1 = 0x4000_0040`, `EC = 0x25`.

**Root cause**: the local interrupt controller (base `0x4000_0000` on QEMU/RPi3) was not mapped once the MMU was enabled.

**Fix**:
- Mapped the local interrupt controller region as device memory in `src/memory/mmu.rs`.
- Also mapped GIC regions for rpi4/rpi5 (they sit outside the main peripheral window).

### 3.4 QEMU memory map mismatch
**Problem**: the `qemu` feature in `layout.rs` used virt‑machine addresses, but the project runs `-M raspi3b`.

**Fix**:
- Updated `layout.rs` for `qemu` to match the RPi3 memory map (RAM at `0x0000_0000`, peripherals at `0x3F00_0000`).

---

## 4) Key takeaways
- Paging is about **translation and protection**, not just allocation.
- Every memory‑mapped device the kernel touches must be mapped.
- The MMU must be enabled **in the EL you are actually running**.
- Per‑process stacks should be backed by page frames owned by the process address space.
- TLB flushes are required on address‑space switches.

---

## 5) Quick file map
- `src/memory/frame.rs` — physical frame allocator
- `src/memory/pagetable.rs` — page table definitions and mapping
- `src/memory/mmu.rs` — kernel mappings, MAIR/TCR/TTBR setup
- `src/memory/config.rs` — `PROCESS_STACK_TOP`
- `src/cpu/process.rs` — per‑process stacks using page frames
- `src/cpu/switch.s` — context switch + TTBR0 update + TLB flush
- `src/cpu/boot.s` — EL transition setup (EL3/EL2 → EL1)
