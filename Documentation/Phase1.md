# Phase 1: Memory Management & Heap Allocator Implementation

**Status:** ✅ Complete  
**Chapters:** OSTEP 13-17 (Memory Virtualization & Free-Space Management)  
**Last Updated:** February 21, 2026

---

## Overview

Phase 1 establishes the **foundational memory management system** for the DDOS kernel. This phase implements a **Free List Allocator** that enables dynamic memory allocation, allowing Rust features like `Box`, `Vec`, and other heap-based data structures to function. Without this allocator, the kernel would only have stack memory and compile-time fixed-size data structures.

This is a **complete deep-dive** into the memory subsystem, covering architecture, algorithms, boilerplate code, and Rust fundamentals for OS development.

---

## 📁 The Memory Folder Structure

```
src/memory/
├── config.rs       ← Memory layout constants (addresses & sizes)
├── heap.rs         ← Free List allocator implementation (300+ lines)
├── mod.rs          ← Initialization boilerplate & GlobalAlloc trait
└── README.md       ← This file (you can add one here for reference)
```

Each file has a specific, well-defined purpose. Let's look at each in detail.

---

## 1️⃣ File: `src/memory/config.rs`

### Purpose
Define **memory layout constants** - the "map" of where everything lives in RAM.

### What It Contains

```rust
pub const KERNEL_START: usize = 0x80000;           // Where kernel code starts
pub const KERNEL_STACK_START: usize = 0x80000;    // Where stack begins
pub const HEAP_START: usize = KERNEL_STACK_START + 0x200000;  // 0x280000
pub const HEAP_SIZE: usize = 0x200000;             // 2 MB
```

### Understanding the Memory Layout

On a Raspberry Pi, the physical memory looks like this:

```
Memory Map (Physical Addresses)
─────────────────────────────────────────────────

0x00000000  ┌──────────────────────────────┐
            │  FIRMWARE/BOOTLOADER         │  128 KB - 512 KB
            │  (Created by Broadcom chip)  │  We don't touch this!
            │  Loads kernel at 0x80000     │
0x00080000  ├──────────────────────────────┤
            │  KERNEL CODE + RODATA        │  From boot.s and main.rs
            │  (What we're running now)    │
0x00200000  │                              │  (depends on kernel size)
            ├──────────────────────────────┤
0x00280000  │  KERNEL STACK (2 MB)         │  Stack grows DOWNWARD ⬇️
            │  Currently using ~0.1 MB     │  Grows from 0x280000 down
            │  Room to grow to 0x80000      │  to 0x80000
            ├──────────────────────────────┤
0x00480000  │  HEAP (2 MB)                 │  FreeList manages all of this
            │  Currently: 1 region (free)  │  Used by Box, Vec, etc.
            │  As we allocate, splits into │  with coalescing during free()
            │  many smaller regions        │
0x00680000  ├──────────────────────────────┤
            │  (Future: Device memory)     │  Mailbox, HDMI framebuffer
            │  (Future: Available RAM)     │  SD card, NIC drivers
            │                              │
```

**Key Insight:** Stack and heap are separated by 2 MB, so they can't collide.

### Why These Specific Addresses?

1. **0x80000 (512 KB)**: Raspberry Pi bootloader convention
   - The ARM bootloader always loads kernels here
   - We don't have a choice - it's hardware/firmware defined
   
2. **Stack starts at 0x80000**: Grows downward
   - Stack grows from HIGH addresses to LOW addresses
   - Starting point: 0x80000
   - Growing downward: 0x7FFFF, 0x7FFFE, ...
   - Buffer: 2 MB of stack space = plenty of room

3. **Heap starts at 0x280000**: After stack buffer
   - 0x80000 + 0x200000 (2 MB) = 0x280000
   - Allocation goes UPWARD: 0x280000, 0x280001, 0x280002, ...
   - This way stack and heap can coexist

### The Formula

```
HEAP_START = KERNEL_STACK_START + KERNEL_STACK_SIZE
           = 0x80000 + 0x200000
           = 0x280000
```

This is not random - it's carefully designed to prevent stack/heap collision!

---

## 2️⃣ File: `src/memory/heap.rs`

### Purpose
Implement the **FreeList allocator** - the heart of dynamic memory allocation.

This is ~300 lines of user-written code (yours!) implementing the algorithm from OSTEP Chapter 17.

### The Big Picture: What Does It Do?

```
Application: let vec = vec![1, 2, 3];
             ↓ (needs 24 bytes)
Rust runtime: Box::new(...) or Vec::new(...)
             ↓ (calls the allocator)
BestFit Allocator: "I'll give you a chunk from heap starting at 0x280100"
             ↓ (marks as used, returns pointer)
Application: vec contains 3 integers at 0x280100
```

### The Data Structures Explained

#### `FreeListNode` - Represents a Free Memory Region

```rust
pub struct FreeListNode {
    size: usize,                     // Total size of this free block (bytes)
    next: Option<*mut FreeListNode>, // Pointer to next free block
}
```

**Visual representation:**

```
In memory, a node looks like:
┌────────────────────────┐
│ FreeListNode header    │  (24 bytes on 64-bit)
├────────────────────────┤
│ Integer 1: size = 256  │  (8 bytes) - how big is this free region?
│ Integer 2: next ptr    │  (8 bytes) - where's the next free region?
│ Padding                │  (8 bytes for alignment)
└────────────────────────┘
```

Why? Because we need to track which memory regions are FREE. When you deallocate, we need to know "was the previous block also free? can I merge?"

#### `FreeList` - Manages All Free Regions

```rust
pub struct FreeList {
    pub head: Option<*mut FreeListNode>,   // Pointer to first free region
    pub start_address: usize,              // Where heap begins (0x280000)
    pub capacity: usize,                   // Total heap size (2 MB)
    pub heap_type: HeapType,               // Which algorithm to use
}
```

**Think of it as:**
- `head`: First node in a linked list of free regions
- `start_address`: "The heap starts here"
- `capacity`: "The heap is this big"
- `heap_type`: "Use this strategy to find free space"

### Initial State (Boot Time)

At startup, the entire 2 MB heap is one giant free region:

```
FreeList {
    head: Points to one FreeListNode ──────────────┐
    start_address: 0x280000                        │
    capacity: 0x200000 (2 MB)                      │
    heap_type: BestFit                             │
}                                                  │
                                                   ↓
FreeListNode ─────────────────────────────────────┘
  size: 0x200000 (entire 2 MB is free!)
  next: None (there's only one node)
```

**This is the freest, most fragmented state possible** - but also the simplest!

### The 4 Free List Algorithms (OSTEP Chapter 17)

All solve the same problem: "Where in the free list should I allocate from?"

#### Algorithm #1: **FirstFit** ⚡ Fastest

```
Algorithm: Keep searching until you find ANY region big enough

    head ──→ [16 B] [8 B] [256 B] ← STOP HERE! 256 B is big enough
              skip   skip    use

Pros:
✅ Fast - stops at first suitable region (O(1) typical case)
✅ Simple to understand and implement
✅ Good for small heaps with lots of activity

Cons:
❌ Fragments at the START of list
❌ Leaves many unusable tiny regions at beginning
❌ Long-running systems degrade over time

Example of fragmentation:
After many allocs/deallocs:
[8B used][2B free][24B used][1B free][16B used]...
Request 100 B → FAIL (no contiguous 100B free!)
```

Implementation in heap.rs:
```rust
fn find_region_first_fit(&mut self, requested_size: usize) 
    -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) 
{
    let mut current = self.head;
    while let Some(node_ptr) = current {
        if (*node_ptr).size >= requested_size {
            return (Some(node_ptr), prev);  // FOUND IT! Return now
        }
        prev = current;
        current = (*node_ptr).next;
    }
    (None, None)  // Couldn't find any suitable region
}
```

#### Algorithm #2: **BestFit** 🎯 Balanced (Currently Selected)

```
Algorithm: Keep searching through ENTIRE list, find the SMALLEST region that fits

    head ──→ [256 B] [1000 B] [300 B]
               ↓        ↓        ↓
             OK       OK       ← BEST! (smallest that fits)
             
             (assume we request 250 B)

Pros:
✅ Low external fragmentation - tight fits
✅ Good memory utilization
✅ Reasonable performance O(n) where n = free regions

Cons:
❌ Slower than FirstFit
❌ Still scans entire list
❌ More splits = more nodes in free list later

Example of good behavior:
Request 100 B from: [256 B][1000 B][300 B]
Use 256 B region → leaves [156 B] (not great)
Don't use 1000 B → would leave [900 B] (wasteful!)
Better than FirstFit!
```

Implementation:
```rust
fn find_region_best_fit(&mut self, requested_size: usize) 
    -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) 
{
    let mut best: Option<*mut FreeListNode> = None;
    let mut best_prev = None;
    let mut current = self.head;
    
    // Search ENTIRE list
    while let Some(node_ptr) = current {
        if (*node_ptr).size >= requested_size {
            // This region is suitable
            // Is it smaller than what we found before? (BestFit criterion)
            if best.is_none() || (*node_ptr).size < (*best.unwrap()).size {
                best = Some(node_ptr);
                best_prev = prev;
            }
        }
        current = (*node_ptr).next;
    }
    (best, best_prev)
}
```

#### Algorithm #3: **WorstFit** ❌ Rarely Used

```
Algorithm: Find the LARGEST region >= requested_size

    head ──→ [256 B] [1000 B] [300 B]
               OK       ← USE THIS! (biggest)
             
             (assume we request 250 B)

Idea: "Use largest space, maybe this will create better divisions?"

Reality: Doesn't work well in practice.

Pros:
✅ Keeps medium-sized regions around (theory)

Cons:
❌ Actually causes MORE fragmentation (confirmed by research)
❌ Wastes large regions on small allocations
❌ Don't use this unless you have a specific reason

Example of why it's bad:
Free list: [256 B] [2000 B] [300 B]
Request 10 B
WorstFit chooses: 2000 B
Result: Leaves [1990 B free] but we just wasted a huge region!

BestFit would choose: 256 B
Result: Leaves [246 B free] and preserves the 2000 B for larger requests
```

#### Algorithm #4: **NextFit** 🔄 Hybrid Approach

```
Algorithm: Like FirstFit, but remember where we last allocated and start there

    Allocation 1:
    head ──→ [A][B][C][D]
                ← allocate from B
             Remember position

    Allocation 2:
    head ──→ [A][B][C][D]
                 ↓
                Skip B (used), continue from C ← allocate from C
             Remember position
                 
    Allocation 3:
    head ──→ [A][B][C][D]
                       ↓
                   Skip D, wrap to A ← allocate from A
             Remember position

Pros:
✅ Faster than BestFit (O(1) typical case)
✅ Reduces clustering at start of list (better than FirstFit)
✅ Good balance of speed and fragmentation

Cons:
❌ More complex to implement
❌ Requires maintaining state (last search position)

Use case: When you want speed like FirstFit but better fragmentation like BestFit
```

### Comparison Table

| Algorithm | Speed | Memory Use | Fragmentation | Notes |
|-----------|-------|-----------|---|---|
| FirstFit | O(1) avg | Poor | High | Fast but fragments at start |
| **BestFit** | O(n) | **Good** | **Low** | ⭐ Currently used - best for this OS |
| WorstFit | O(n) | Poor | High | Don't use (worse than BestFit) |
| NextFit | O(1) avg | Good | Low | Good hybrid (if you have time to implement) |

Here, **BestFit is selected** in `src/memory/mod.rs` because:
- The heap is only 2 MB
- We're not under extreme performance pressure
- Memory efficiency matters more than speed
- The kernel typically has low allocation churn

### Core Operations: Allocation

The allocate function is the main workhorse. Here's the step-by-step process:

```
User calls: let x = Box::new(42);
                    ↓
Rust calls: allocator.allocate(8)  // 8 bytes for a u32
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 1: Align the size               │
            │ requested = 8 bytes                  │
            │ aligned = (8 + 8 - 1) & ~(8-1)      │
            │        = 15 & ~7 = 8 bytes          │
            │ Why? Alignment helps CPU access     │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 2: Add metadata overhead        │
            │ total = aligned + header + footer    │
            │       = 8 + 24 + 8 = 40 bytes       │
            │ Header (24B): size, next pointer    │
            │ Footer (8B): size for backward walk │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 3: Find suitable free region    │
            │ Find in free list using BestFit     │
            │ We have 1 region: 2 MB free        │
            │ Use it! (big enough)                │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 4: Check if we should split     │
            │ We request 40B from 2 MB            │
            │ Remaining: 2097152 - 40 = 2097112  │
            │ Split it!                           │
            │ • Keep 40 B for user (marked used)  │
            │ • Create new node for 2097112 B     │
            │ • Update free list pointers         │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 5: Write metadata              │
            │ [Header][Data][Footer]              │
            │  ↓       ↓      ↓                    │
            │ size   user   size (for dealloc)    │
            │        data   lookup                │
            └─────────────────────────────────────┘
                    ↓
            Return pointer to user's data
            
Memory before:
┌──────────────────────────────────────────┐
│ One big free region: 2 MB                │
└──────────────────────────────────────────┘

Memory after:
┌────────────────┬──────────────────────────┐
│ Used (40 B)    │ Free (2 MB - 40 B)       │
└────────────────┴──────────────────────────┘
  ↑ pointer      ↑
  returned       new node in free list
```

### Core Operations: Deallocation

When user frees memory, we need to "unborrow" it and return it to the free list:

```
User calls: drop(vec);  // Vector goes out of scope
                    ↓
Rust calls: allocator.deallocate(pointer)
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 1: Find the header             │
            │ User gives us pointer to data       │
            │ Header is 24 bytes BEFORE that      │
            │ header_ptr = data_ptr - 24          │
            │ Now we know: size, next pointer     │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 2: Find insertion point        │
            │ Free list must stay SORTED by addr  │
            │ (important for coalescing!)         │
            │ Find where this block goes in order │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 3: Reinsert into free list     │
            │ Add this node back to the linked    │
            │ list in sorted order                │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 4: Forward Coalescing          │
            │ Is the NEXT block also free?        │
            │ If yes: merge them together         │
            │ "Join adjacent free regions"        │
            └─────────────────────────────────────┘
                    ↓
            ┌─────────────────────────────────────┐
            │ Step 5: Backward Coalescing         │
            │ Is the PREVIOUS block also free?    │
            │ Use the footer to find previous     │
            │ Read footer 8 bytes BEFORE us       │
            │ If yes: merge them together         │
            └─────────────────────────────────────┘

Memory before deallocation:
┌────────┬─────────────┬──────────┬──────────────┐
│ Used   │ Free (100B) │ Used     │ Free (500B)  │
└────────┴─────────────┴──────────┴──────────────┘

Free this middle block:
┌────────┬─────────────┬──────────┬──────────────┐
│ Used   │ Used (now)  │ Used     │ Free (500B)  │
└────────┴─────────────┴──────────┴──────────────┘
          ↑ pointer to this block

After dealloc with coalescing:
┌────────┬─────────────┬──────────┬──────────────┐
│ Used   │ Free (600B) │← Merged! │ (empty)      │
└────────┴─────────────┴──────────┴──────────────┘

Why merge? Prevents this nightmare scenario:
Without coalescing:
[1B used][1B free] [1B used][1B free] [1B used][1B free]...
Request 1000B → FAIL! (no contiguous block)

With coalescing:
[1B used][1000B free! Merged from fragments]
Request 1000B → SUCCESS!
```

### Memory Layout: How Data Actually Looks in RAM

When we allocate:

```
Allocate 16 bytes:
Requested:  16 B
Aligned:    16 B
Total block: 16 + 24 (header) + 8 (footer) = 48 B

Layout in memory:
0x280018 ┌─────────────────────────┐
         │ FreeListNode (24 B):    │
         │  - size: 48             │ ← Says "I'm a 48B block"
         │  - next: 0x280080       │ ← Points to next free region
0x280030 ├─────────────────────────┤
         │ User's data (16 B):     │
         │ [0x0000000000000042]    │ ← The actual integer 42 ← USER GETS POINTER HERE
         │                         │
0x280040 ├─────────────────────────┤
         │ Footer (8 B):           │
         │ [48]                    │ ← "I'm a 48B block" (for dealloc backward walk)
0x280048 └─────────────────────────┘

So when we:
1. Allocate: Return pointer to 0x280030 (the user's data start)
2. Deallocate: User passes 0x280030, we read back 24 bytes to get header at 0x280018
```

### Alignment Best Practices (Why ALIGN = 8?)

```rust
const ALIGN: usize = 8;  // 64-bit systems

Why 8 bytes?
- Pointers are 8 bytes, so align to 8
- A `usize` is 8 bytes, align to it
- Reduces fragmentation (every 8B boundary)
- Standard for 64-bit systems

Alignment formula:
align_up(size) = (size + ALIGN - 1) & !(ALIGN - 1)

Examples with ALIGN=8:
- align_up(1) = 8
- align_up(7) = 8
- align_up(8) = 8          (already aligned)
- align_up(9) = 16
- align_up(15) = 16
- align_up(16) = 16        (already aligned)

Binary view:
9 in binary:    1001
ALIGN-1 = 7:    0111
+ 7:            1 0000  = 16 in binary
```

### Fragmentation Explained

**External Fragmentation** - Free space is split into unusable pieces:

```
Without fragmentation:
[Used: 100B][Free: 1900B]
Request 1000B → SUCCESS

WITH fragmentation:
[U:40][F:50][U:60][F:30][U:80][F:40]...
Request 1000B → FAIL (no contiguous 1000B free!)

But total free = 50+30+40 = 120B
   (enough for 1000B? No!)
   
Coalescing fixes this:
[U:40][F:1000+][U:60]
Request 1000B → SUCCESS!
```

---

## 3️⃣ File: `src/memory/mod.rs`

### Purpose
"Boilerplate" initialization code that:
1. Creates the global allocator instance
2. Initializes it at boot time
3. Handles allocation failures

### Understanding the "Boilerplate"

**What does "boilerplate" mean?** Code that's necessary but mostly follows a pattern. You typically don't modify it much after initial setup.

### The Problem It Solves

In Rust, static variables must be **immutable**:

```rust
static MY_NUMBER: u32 = 42;
// All good, immutable, no problem

MY_NUMBER = 99;  // ❌ ERROR: Can't mutate a static!
```

But the allocator needs to **mutate** its internal state:

```rust
static ALLOCATOR: FreeList = ...;

ALLOCATOR.allocate(100);  // ❌ ERROR: allocate() takes &mut self
                          // but we can only provide &self (immutable)
```

**This is impossible in standard Rust!** That's where `Locked<T>` comes in.

### Solution: Interior Mutability with `Locked<T>`

See the next section for full details on `Locked<T>`.

### Global Allocator Registration

```rust
#[global_allocator]
static ALLOCATOR: Locked<FreeList> = Locked::new(FreeList {
    head: None,
    start_address: 0,
    capacity: 0,
    heap_type: HeapType::BestFit,
});
```

This attribute tells Rust:
- "This object is the allocator for Box, Vec, etc."
- ANY call to allocate memory goes through this object
- It must implement the `GlobalAlloc` trait (the allocate/deallocate interface)

### Why We Initialize with Dummy Values (0, 0, None)

We can't use real values (`HEAP_START`, `HEAP_SIZE`) in a `const` context. Const evaluation has a limited scope and can't access runtime values.

Solution: Initialize with dummy values, then "re-initialize" in the `init()` function:

```rust
static ALLOCATOR: Locked<FreeList> = Locked::new(FreeList {
    head: None,           // ← Dummy: "No regions"
    start_address: 0,     // ← Dummy: "Starts at 0"
    capacity: 0,          // ← Dummy: "Size is 0"
    heap_type: HeapType::BestFit,
});

// Later, during kernel boot:
pub fn init() {
    unsafe {
        let allocator = ALLOCATOR.lock();  // Get mutable access
        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
        // Now allocator has REAL values!
    }
}
```

Before `init()` is called: Allocator is "offline" (can't allocate anything)
After `init()` is called: Allocator is "online" (Box, Vec work!)

### The `init()` Function

```rust
pub fn init() {
    unsafe {
        let allocator = ALLOCATOR.lock();
        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }
}
```

**Step by step:**

1. `ALLOCATOR.lock()` - Get a mutable reference to the FreeList inside Locked<T>
2. `*allocator = ...` - Replace the entire FreeList struct with a real initialized one
3. `FreeList::init(...)` - Create a new FreeList pointing to real heap memory

**Why `unsafe`?**
- Modifying a global static requires `unsafe`
- It's safe because:
  - We only do it once (during boot)
  - No other code can call this simultaneously (single-threaded)
  - We're in kernel-only code (not reachable from userspace yet)

### The Allocation Error Handler

```rust
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
```

This function is called when `Box::new()` or `Vec::new()` tries to allocate memory but **fails** (returns null).

**Why might it fail?**
- Requested size > HEAP_SIZE (impossible request)
- Heap is completely fragmented (no contiguous block available)
- HEAP_SIZE is too small for your workload

**What happens?**
- We can't recover (can't return `Result`, must return never type `!`)
- We panic with the layout that failed
- Kernel crashes with error message

**Production systems would:**
- Trigger garbage collection (we don't have one)
- Swap to disk (too complex for now)
- Terminate less important processes
- Just crash (what we do now - acceptable for a learning OS)

### Putting It All Together: Boot Sequence

```
1. KERNEL STARTS (src/cpu/boot.s)
   ↓
2. _main() is called (Rust main)
   ↓
3. memory::init() is called
   ├─ ALLOCATOR.lock() - get mutable access
   ├─ FreeList::init(0x280000, 0x200000, BestFit)
   │  └─ Write FreeListNode at 0x280000
   └─ Now entire 2 MB heap is registered as ONE free region
   ↓
4. Console/UART drivers initialize
   ↓
5. Box::new() and Vec::new() NOW WORK!
   (Before this, any allocation would panic!)
```

---

## 4️⃣ File: `src/utils/locked.rs`

### Purpose
Provide **interior mutability** for global allocator - allow mutation through immutable references.

### The Core Problem: Rust's Borrowing Rules

Rust's fundamental rule:

```
You can have:
✅ Many immutable references (&T)
✅ One mutable reference (&mut T)
❌ NOT both at the same time
```

But the allocator is a **static** (always one reference):

```rust
// This is immutable!
static ALLOCATOR: FreeList = ...;

// Allocations need mutable access:
Box::new(42)  // ❌ Needs &mut ALLOCATOR but we only have &ALLOCATOR
```

### The Solution: `UnsafeCell`

`UnsafeCell` is a special Rust type that breaks this rule **safely** in certain cases:

```rust
pub struct Locked<A> {
    inner: UnsafeCell<A>,  // Can get mutable pointers from immutable refs
}
```

**How it works:**

```rust
impl<A> Locked<A> {
    pub fn lock(&self) -> &mut A {
        // self is immutable (&self)
        // But we return &mut A (mutable)!
        unsafe { &mut *self.inner.get() }
    }
}
```

This is "safe" (doesn't cause undefined behavior) IF:
- Only one thread accesses the data at a time
- We're careful not to hold multiple mutable references

### Why Is This Safe (For Our Kernel)?

In a **single-threaded** kernel:
- Only one CPU core runs code at any time
- Only one thread can call `lock()` simultaneously
- Therefore, no data races possible

**In a multi-threaded kernel**, this would be a race condition:

```
Thread 1: lock() → &mut allocator ─────→ [allocate]
Thread 2: lock() → &mut allocator ─────→ [allocate]
                     ↑
                Same mutable ref!
                Both threads modify at once
                → Data corruption!
```

That's why Chapter 28-29 (OSTEP) introduces **SpinLock**, which we'll implement in Phase 2:

```rust
// In Phase 2:
pub struct SpinLock<T> {
    inner: UnsafeCell<T>,
    locked: AtomicBool,  // ← Prevents simultaneous access!
}

impl<T> SpinLock<T> {
    pub fn lock(&self) -> &mut T {
        // Spin (busy wait) until locked bit is false
        while self.locked.compare_and_swap(false, true) {
            // Wait...
        }
        // Now we have exclusive access
        unsafe { &mut *self.inner.get() }
    }
}
```

### Memory Layout: UnsafeCell Under the Hood

```rust
UnsafeCell<T> is just a wrapper:

┌────────────────────────┐
│ UnsafeCell<A>          │
│ ┌──────────────────┐   │
│ │ inner: A         │   │ (contains the actual data)
│ └──────────────────┘   │
└────────────────────────┘

Example: UnsafeCell<FreeList>
┌────────────────────────┐
│ UnsafeCell             │
│ ┌──────────────────┐   │
│ │ FreeList {       │   │
│ │  head: Some(ptr) │   │
│ │  start: 0x280000 │   │
│ │  ...             │   │
│ │ }                │   │
│ └──────────────────┘   │
└────────────────────────┘

When we call .lock():
Returns: mutable reference to the FreeList inside

```rust
let allocator = ALLOCATOR.lock();
*allocator = ...  // Modify the FreeList!
```
```

### Why the `Sync` Trait?

```rust
unsafe impl<A> Sync for Locked<A> {}
```

This tells Rust: "It's safe to send `Locked<A>` between threads."

Normally, types containing `UnsafeCell` are NOT `Sync` because they could cause data races. But we explicitly say "It's OK, we handle it correctly" by:

1. Implementing `Sync` manually with `unsafe impl`
2. Being responsible for correct synchronization (which we do via SpinLock in Phase 2)

### The Workflow in Boot

```
static ALLOCATOR: Locked<FreeList> = Locked::new(dummy_freelist);
                   ↑
                   UnsafeCell inside

memory::init():
    let allocator = ALLOCATOR.lock();  // Get &mut FreeList
    *allocator = FreeList::init(...);  // Replace with real allocator
    // lock() returns, mutable reference dropped
    // Allocator now works!

Box::new(42):
    Rust calls: GlobalAlloc::alloc(&self, layout)
    Which calls: ALLOCATOR.lock().allocate(size)
    Which gets: &mut FreeList from UnsafeCell
    Which can: Mutate internal state safely (because single-threaded)
```

---

## 🔗 How It All Connects

### Initialization Flow

```
src/cpu/boot.s  (Assembly)
    ↓ Sets up stack, jumps to _main()
src/main.rs  (Rust entry point)
    ↓ Calls memory::init()
src/memory/mod.rs  (initialization)
    ↓ Calls FreeList::init() with values from config.rs
src/memory/config.rs  (HEAP_START, HEAP_SIZE)
    ↓ Returns initialized allocator
src/memory/heap.rs  (FreeList implementation)
    ↓ Now handles all allocations
src/utils/locked.rs  (Interior mutability)
    ↓ Wrapped the FreeList for static usage

Result: Box::new(), Vec::new() now work!
```

### When You Allocate Memory

```
Your code: let vec = vec![1, 2, 3];
    ↓
Rust macro: expands to Vec::new().push(1).push(2).push(3)
    ↓
Vec::new(): calls Box::new(capacity) for internal buffer
    ↓
Box::new(): calls allocator.alloc(Layout)
    ↓
GlobalAlloc impl: routes to ALLOCATOR
    ↓
Locked::lock(): gets &mut FreeList from UnsafeCell
    ↓
FreeList::allocate(): uses BestFit algorithm
    ↓
find_region_best_fit(): searches free list
    ↓
Returns pointer to free memory
    ↓
Vector's buffer is at that pointer!
```

### File Dependencies

```
mod.rs
    ├─→ depends on: config.rs, heap.rs, ../utils/locked.rs
    └─→ uses: HEAP_START, HEAP_SIZE from config
    └─→ uses: FreeList, HeapType from heap
    └─→ uses: Locked<T> from utils

heap.rs
    ├─→ depends on: (none - core algorithms)
    └─→ defines: FreeList, FreeListNode, HeapType, allocation algos

config.rs
    ├─→ depends on: (none - just constants)
    └─→ defines: KERNEL_START, HEAP_START, HEAP_SIZE

locked.rs (in utils/)
    ├─→ depends on: (none - core Rust)
    └─→ defines: Locked<T>, interior mutability wrapper
```

---

## 🧠 Rust Fundamentals for OS Development

### Concept 1: Raw Pointers vs Safe References

```rust
// Safe reference (Rust checks this)
let x = 42;
let r: &u32 = &x;  // Compiler verifies: x lives long enough

// Raw pointer (unsafe)
let ptr: *mut u32 = 0x280000 as *mut u32;  // Can point to anything!
// Compiler: "I don't know if this is valid!"

// Using raw pointer
unsafe {
    ptr.write(42);    // Write to raw address (anything could be there!)
    let value = ptr.read();  // Read from raw address
}
```

**Why kernel code needs unsafe:** Hardware registers are at fixed addresses, not allocated by Rust.

### Concept 2: Option<T> for Nullable Pointers

```rust
// In C:
void *node = NULL;  // Could be null
node->size = 100;   // Crash if null! ← undefined behavior

// In Rust:
let node: Option<*mut FreeListNode> = None;  // Explicitly nullable

if let Some(node_ptr) = node {
    // Safe: we know node_ptr is not null here
    unsafe { (*node_ptr).size = 100; }
} else {
    // Safe: handled the null case
}
// Can't forget to handle null! Compiler enforces it.
```

### Concept 3: `const fn` - Compile-Time Functions

```rust
pub const fn new(inner: A) -> Self {  // ← const fn
    Locked { inner: UnsafeCell::new(inner) }
}

// Can be used at compile time:
static ALLOCATOR: Locked<FreeList> = Locked::new(dummy);
                                     ↑
                                     Called at compile time!
                                     No runtime cost
```

### Concept 4: `!` (Never Type) - Functions That Don't Return

```rust
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error");  // ← Never returns!
}

// This means:
let x = alloc_error_handler(...);  // ← x can never be assigned
// Control flow exits (panics), x is never initialized
```

### Concept 5: Size and Alignment

```rust
size_of::<FreeListNode>() == 24  // On 64-bit systems
                                  // Field 1: usize (8)
                                  // Field 2: Option<*mut...> (8)
                                  // Padding: (8)

// Alignment:
// Every FreeListNode must start at an 8-byte boundary
// This is AUTOMATIC in Rust; compiler handles it
```

---

## 📊 Performance Characteristics

### Current Allocator (BestFit)

| Operation | Time Complexity | Best Case | Worst Case |
|-----------|-----------------|-----------|-----------|
| Allocate | O(n) | O(1) - free list sorted | O(n) - scan all regions |
| Deallocate | O(n) | O(1) - at end | O(n) - find insertion point |
| Coalesce | O(1) | Immediate | Immediate |

Where n = number of free regions

### Example Timeline

```
Time  Action               Free List
t0    Boot                 [2MB free]  (1 region)
t1    Alloc 100B           [100B used][2MB-100B free]  (2 regions)
t2    Alloc 50B            [100B][50B][2MB-150B]  (3 regions)
t3    Free first 100B      [100B][50B][2MB-150B]  (3)
      (coalesce)           [150B][2MB-150B]  (2)
t4    Alloc 200B           [200B][2MB-350B]  (2)
```

As the OS runs, n (number of free regions) typically grows from 1 to ~10-40, then stabilizes. Most allocations are O(1-2) after initial fragmentation.

---

## 🎓 Learning Outcomes

After Phase 1, you understand:

✅ **Memory Layout**: How addresses are organized on RPi  
✅ **Free List Algorithm**: The core of malloc/free  
✅ **Allocation Strategies**: First/Best/Worst/NextFit tradeoffs  
✅ **Fragmentation**: External fragmentation and coalescing  
✅ **Rust Pointers**: Raw pointers, UnsafeCell, interior mutability  
✅ **Global Allocator**: How Rust redirects all allocations  
✅ **Boilerplate Code**: Why it's there and what it does  
✅ **Metadata**: Headers and footers for allocation bookkeeping  

---

## 🚀 Building & Running

### Quick Start - Using Scripts

```bash
# Build and run in QEMU
./scripts/build-qemu.sh

# Build for real RPi4 hardware
./scripts/build-rpi4.sh
```

### Manual Build

```bash
# QEMU (default)
cargo build --target aarch64-unknown-none-softfloat

# Or explicit feature
cargo build --features qemu --target aarch64-unknown-none-softfloat

# RPi4
cargo build --no-default-features --features rpi4 \
  --target aarch64-unknown-none-softfloat

# Run in QEMU
qemu-system-aarch64 -M raspi3b -serial stdio \
  -kernel target/aarch64-unknown-none-softfloat/debug/ddos
```

### Expected Output

```
[KERNEL] Booting DDOS...
[KERNEL] Heap Initialized. (BestFit)
[KERNEL] Console initialized via HDMI
Testing Heap Allocation...
  ✓ Box allocated: 42
  ✓ Vec allocated: [0, 1, 2, 3, 4]
  ✓ String allocated: "Hello, Heap!"
>
```

---

## 📖 OSTEP References

| Chapter | Title | Relevance to Phase 1 |
|---------|-------|---|
| 13 | The Abstraction: Address Spaces | Memory layout, kernel vs user space |
| 14 | Interlude: Memory API | malloc/free interface design |
| 15 | Mechanism: Address Translation | Virtual to physical address mapping |
| 16 | Segmentation | Hardware memory protection (skipped - ARM uses paging) |
| 17 | Free-Space Management | ⭐ **Core of this phase** - all allocation algorithms |

### Key Concepts from Ch. 17

- Free list as linked list of free regions
- Allocation strategies (First/Best/Worst/NextFit)
- Block splitting (allocate part of a region)
- Coalescing (merge adjacent free regions)
- Headers and footers for metadata
- External vs internal fragmentation

---

## 🔮 What's Next: Phase 2

**Concurrency & Synchronization** (OSTEP Chapters 26-29)

Currently, each function can mutate the allocator. In Phase 2:

1. Implement **SpinLock<T>** - prevents simultaneous access
2. Wrap Console in SpinLock - multiple "threads" can't write simultaneously
3. Wrap UART in SpinLock - kernel and drivers won't fight over output
4. Theory: Race conditions, deadlocks, synchronization

This moves `Locked<T>` from "single-threaded hack" to proper "multi-threaded primitive."

---

## Summary

✅ **Complete Free List allocator** with 4 selectable strategies  
✅ **BestFit algorithm** selected for memory efficiency  
✅ **Block splitting & coalescing** prevents fragmentation  
✅ **GlobalAlloc integration** - Box and Vec work!  
✅ **Memory layout** carefully designed (stack ≠ heap)  
✅ **Hardware abstraction** - runs on QEMU, RPi3, RPi4  
✅ **Interior mutability** - static allocator can be mutated safely  
✅ **Boilerplate code** - necessary for Rust integration  

The kernel can now use any Rust heap collection!

---

**Author:** Daksh Desai (user-implemented: config.rs, heap.rs, hardwareselect.rs, build scripts)  
**Date:** February 21, 2026  
**Status:** ✅ Phase 1 Complete
