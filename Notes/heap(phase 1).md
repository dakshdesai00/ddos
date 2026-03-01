# Heap Allocator — Phase 1 Notes

**OSTEP Chapters:** 13, 14, 15, 17 (core: 17)
**What I built:** Free List allocator with splitting, coalescing, and four allocation strategies
**Files:**

- `src/memory/config.rs` — mine, memory addresses
- `src/memory/heap.rs` — mine, the actual allocator logic
- `src/memory/mod.rs` — integration glue (not the interesting part)
- `src/utils/locked.rs` — Rust-specific wrapper (not the interesting part)

---

## Code Sync Update (2026-03-02)

This section reflects the **current code** in `src/memory/*` and should be treated as canonical if anything below disagrees.

### Current `config.rs` constants

```rust
pub const KERNEL_START: usize = 0x80000;
pub const KERNEL_STACK_START: usize = 0x80000;
pub const HEAP_START: usize = KERNEL_STACK_START + 0x200000; // 0x280000
pub const HEAP_SIZE: usize = 0x200000;                        // 2 MiB
```

Effective heap range is still `0x280000..0x480000` (2 MiB total), but the kernel/stack base constants are now `0x80000`.

### Current allocator invariants in `heap.rs`

- `ALIGN` is **16** (not 8).
- `FreeListNode` uses `#[repr(C, align(16))]`.
- `align_up()` returns `Option<usize>` and uses checked arithmetic.
- `init()` now aligns the start address up, subtracts alignment loss from capacity, and asserts minimum usable space.
- `init()` writes both the initial head node and its footer.

### Current allocation API and behavior

```rust
pub fn allocate(&mut self, requested_size: usize, requested_align: usize) -> Option<*mut u8>
```

- Requests with `requested_align > ALIGN` are rejected (`None`).
- Zero-size requests are normalized via `requested_size.max(1)`.
- Total block size is computed with checked + aligned math:
  - aligned payload
  - plus header+footer overhead
  - then aligned again
- Strategy dispatch is unchanged (`BestFit`, `WorstFit`, `FirstFit`, `NextFit`).

### Current integration in `mod.rs`

```rust
pub fn init() {
        unsafe {
                let allocator = ALLOCATOR.lock();
                *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
        }
}
```

Global allocator bridge now passes both size and alignment:

```rust
allocator.allocate(layout.size(), layout.align())
```

So unlike the earlier phase note, `layout.align()` is no longer ignored.

---

## What I Actually Read and Why It Matters

The core of this phase is Chapter 17 of OSTEP — _Free-Space Management_. Before that, chapters 13 and 14 give you the mental model: programs see virtual address spaces, not real RAM, and `malloc`/`free` live in user space as a library on top of OS-provided memory. Chapter 15 explains the hardware mechanism behind virtual memory — base and bounds registers, the MMU, address translation. We're writing the OS, so we're writing the `malloc`. The virtual memory machinery in chapter 15 is coming later when we add user processes.

The question Chapter 17 poses is simple but brutal: **how do you manage a big flat region of memory when requests come in arbitrary sizes and in arbitrary order?** Arrays don't work — you don't know how many allocations will happen. A single pointer that just moves forward (bump allocator) doesn't work because you can't free anything. You need something smarter.

The answer is a **free list**: a linked list embedded _inside the free memory itself_ that tracks which chunks are available. When you allocate, you find a suitable chunk and hand it out. When you free, you put it back and try to merge it with neighbors.

---

## Memory Layout — `src/memory/config.rs`

```rust
pub const KERNEL_START: usize = 0x200000;
pub const KERNEL_STACK_START: usize = 0x200000;
pub const HEAP_START: usize = KERNEL_STACK_START + 0x200000;
pub const HEAP_SIZE: usize = 0x200000;
```

The Raspberry Pi bootloader always loads the kernel at physical address `0x200000`. That's not a choice we make — it's what the Broadcom firmware does. So `KERNEL_START` is fixed.

The stack grows downward from `0x200000`. We give it 2MB of space (`0x200000`), which means the heap starts at `0x200000 + 0x200000 = 0x400000`. The heap also gets 2MB. They're separated by the stack buffer so they can't crash into each other.

Visually:

```
0x00000000   Firmware / bootloader region (don't touch)
0x00080000   Kernel code starts here
             Stack lives between 0x200000 and 0x400000, grows downward
0x00280000   Heap starts here, grows upward
0x00480000   End of heap
```

Nothing fancy here. The constants are just a written-down contract for where things live so the heap init function knows where to point.

---

## The Free List — `src/memory/heap.rs`

This is the part I actually wrote from reading Chapter 17.

### The two structs

```rust
pub struct FreeListNode {
    size: usize,
    next: Option<*mut FreeListNode>,
}

pub struct FreeList {
    pub head: Option<*mut FreeListNode>,
    pub start_address: usize,
    pub capacity: usize,
    pub heap_type: HeapType,
    pub(crate) next_fit_cursor: Option<*mut FreeListNode>,
}
```

`FreeListNode` represents one free chunk. It lives _at the start of that chunk in memory_. There's no separate array of node metadata — the nodes are written directly into the free space they describe.

`FreeList` is the manager. It holds a pointer to the first free node (`head`), the base address of the heap, total capacity, and which allocation strategy to use.

`next_fit_cursor` stores the current scan position for Next Fit. After each Next Fit allocation, this cursor advances so the next search resumes from where the previous one left off.

At boot, the entire heap is one giant free chunk:

```
Heap memory (0x280000 to 0x480000):
┌──────────────────────────────────────┐
│ FreeListNode { size: 2MB, next: None }
│ (rest of the 2MB is just bytes)
└──────────────────────────────────────┘
```

`head` points to that single node. As we allocate, it splits. As we free, we merge pieces back.

### init

```rust
pub unsafe fn init(start: usize, capacity: usize, heap_type: HeapType) -> Self {
    let node_ptr = start as *mut FreeListNode;
    node_ptr.write(FreeListNode::new(capacity, None));

    FreeList {
        head: Some(node_ptr),
        start_address: start,
        capacity,
        heap_type,
        next_fit_cursor: Some(node_ptr),
    }
}
```

Cast the start address to a pointer and write one node there. That node's size is the entire heap capacity. Now the free list has exactly one entry covering all available memory.

### Alignment and overhead

```rust
const ALIGN: usize = 8;

fn align_up(size: usize) -> usize {
    (size + ALIGN - 1) & !(ALIGN - 1)
}

fn block_overhead() -> usize {
    size_of::<FreeListNode>() + size_of::<usize>()
}
```

**Alignment:** Every pointer we return needs to be 8-byte aligned. On 64-bit ARM, misaligned access is either a performance penalty or a fault depending on the instruction. The formula `(size + ALIGN - 1) & !(ALIGN - 1)` rounds up to the nearest multiple of 8. For example, 9 bytes becomes 16. 8 stays 8.

**Overhead:** Each allocated block needs a header (`FreeListNode` — `size` + `next`, so 16 bytes on a 64-bit system) and a footer (one `usize`, 8 bytes). Total overhead per allocation: 24 bytes.

The footer is the key to backward coalescing. When we free a block, we need to merge with the _previous_ block if it's free. We can't walk forward to find it efficiently — that would require a doubly linked list or scanning from `head`. Instead, the footer at the end of every block stores the block's own size. So we read 8 bytes before the current block's start to get the previous block's size, then jump backward by that amount to find the previous header. That's the boundary tag technique from OSTEP.

### The four strategies

Chapter 17 describes these — I implemented all four so the strategy is switchable.

#### First Fit

Scan from `head`, return the first node that's big enough. Stop immediately.

```rust
fn find_region_first_fit(&mut self, requested_size: usize)
    -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>)
{
    while let Some(node_ptr) = current {
        if node.size >= requested_size {
            return (Some(node_ptr), prev);
        }
        prev = current;
        current = node.next;
    }
    (None, None)
}
```

Returns `(found_node, previous_node)` because the caller needs `prev` to fix up linked list pointers after removing the found node.

Fast. Tends to fragment the start of the free list over time because small leftover pieces accumulate there.

#### Best Fit

Scan the _entire_ free list. Track the smallest node that still fits.

```rust
fn find_region_best_fit(&mut self, requested_size: usize) -> ... {
    while let Some(node_ptr) = current {
        if node.size >= requested_size {
            if best.is_none() || node.size < (*best.unwrap()).size {
                best = Some(node_ptr);
                best_prev = prev;
            }
        }
        prev = current;
        current = node.next;
    }
    (best, best_prev)
}
```

The comparison `node.size < (*best.unwrap()).size` keeps track of the tightest fit found so far. After the loop, `best` is the smallest block that's still big enough.

This is what I'm running (`HeapType::BestFit` in `mod.rs`). For a 2MB heap with low allocation pressure it's fine. BestFit leaves the least waste from a given allocation's perspective — you're not burning a 500KB block to satisfy a 10-byte request.

#### Worst Fit

Scan the entire list, return the _largest_ node. The idea is that after splitting, you leave a large leftover that will still be useful. In practice, research shows it performs worse than BestFit. Implemented for completeness.

```rust
if worst.is_none() || node.size > (*worst.unwrap()).size {
    worst = Some(node_ptr);
    worst_prev = prev;
}
```

Only change from BestFit is the comparison direction.

#### Next Fit

This implementation is now a real Next Fit. It stores a cursor in `FreeList` (`next_fit_cursor`) and starts searching from that node instead of always from `head`. If no fit is found to the end of the list, it wraps to `head` and scans until it reaches the original start position.

After allocation, the cursor advances to the next free region (or to the split remainder when a block is split). After deallocation/coalescing, the cursor is reset to a valid node (`head`) so it never points to removed nodes.

The practical effect is exactly what OSTEP describes: allocation search start points rotate through the list instead of repeatedly concentrating at the front.

### allocate

```rust
pub fn allocate(&mut self, requested_size: usize) -> Option<*mut u8> {
    let aligned_payload = Self::align_up(requested_size);
    let total_size = aligned_payload + Self::block_overhead();

    let (region, prev) = match self.heap_type { ... };

    let node_ptr = region?;
```

`region?` is Rust's `?` operator on an `Option`. If `region` is `None` (no suitable free block), it returns `None` from `allocate`. Clean.

```rust
    if node.size >= total_size + Self::block_overhead() {
        // Block is big enough to split
        let remaining_size = node.size - total_size;
        let new_node_ptr = (node_ptr as *mut u8).add(total_size) as *mut FreeListNode;
        new_node_ptr.write(FreeListNode::new(remaining_size, node.next));

        // Write footer for the new free node
        let new_footer = (new_node_ptr as usize + remaining_size - size_of::<usize>()) as *mut usize;
        new_footer.write(remaining_size);

        // Update list pointer to skip over the allocated block
        if let Some(prev_ptr) = prev {
            (*prev_ptr).next = Some(new_node_ptr);
        } else {
            self.head = Some(new_node_ptr);
        }
        node.size = total_size;
    } else {
        // Block is too small to split, take the whole thing
        if let Some(prev_ptr) = prev {
            (*prev_ptr).next = node.next;
        } else {
            self.head = node.next;
        }
    }
```

The split condition is `node.size >= total_size + block_overhead()`. You need not just `total_size` left over but also enough for another node's overhead. If the leftover after splitting would be 0 or just a few bytes, there's no point writing a new free node — you just take the whole block and eat the internal waste.

After the split (or whole-block take), write the footer for the allocated block:

```rust
    let footer_ptr = (node_ptr as usize + node.size - size_of::<usize>()) as *mut usize;
    footer_ptr.write(node.size);

    Some((node_ptr as *mut u8).add(size_of::<FreeListNode>()))
```

The returned pointer skips past the `FreeListNode` header. The user gets a pointer to their writable data, not the metadata.

Memory layout of an allocated block:

```
node_ptr →  [FreeListNode: size, next]   ← 16 bytes (header)
            [padding]                    ← 8 bytes (alignment)
user ptr →  [user data]                  ← aligned_payload bytes
            [footer: size]               ← 8 bytes
```

### deallocate

```rust
pub fn deallocate(&mut self, address: usize) {
    let node_ptr = (address - size_of::<FreeListNode>()) as *mut FreeListNode;
```

The user passes back the pointer they received. We subtract the header size to get back to the node. From there, `node.size` tells us the full block size including overhead.

**Step 1: find the right position in the free list.**

The free list has to be kept sorted by address for coalescing to work:

```rust
    while let Some(curr_ptr) = current {
        if curr_ptr as usize > node_ptr as usize {
            break;
        }
        prev = current;
        current = (*curr_ptr).next;
    }
    node.next = current;
    if let Some(prev_ptr) = prev {
        (*prev_ptr).next = Some(node_ptr);
    } else {
        self.head = Some(node_ptr);
    }
```

Walk the list until we find a node with a higher address than ours. Insert our node before it. This keeps the list in ascending address order.

**Step 2: forward coalescing — merge with the next block.**

```rust
    if let Some(next_ptr) = node.next {
        let node_end = node_ptr as usize + node.size;
        if node_end == next_ptr as usize {
            node.size += (*next_ptr).size;
            node.next = (*next_ptr).next;
            let footer = (node_ptr as usize + node.size - size_of::<usize>()) as *mut usize;
            footer.write(node.size);
        }
    }
```

If the end of our block exactly touches the start of the next free block, they're adjacent — merge them. Bump `node.size` up by the next block's size, skip over `next` in the linked list, and write a new footer with the combined size.

**Step 3: backward coalescing — merge with the previous block.**

```rust
    if node_ptr as usize > self.start_address {
        let prev_footer_ptr = (node_ptr as usize - size_of::<usize>()) as *mut usize;
        let prev_size = prev_footer_ptr.read();
        let prev_start = node_ptr as usize - prev_size;

        if prev_start >= self.start_address {
            let prev_node_ptr = prev_start as *mut FreeListNode;
            let prev_node = &mut *prev_node_ptr;

            if prev_start + prev_node.size == node_ptr as usize {
                prev_node.size += node.size;
                prev_node.next = node.next;
                let footer = (prev_start + prev_node.size - size_of::<usize>()) as *mut usize;
                footer.write(prev_node.size);
            }
        }
    }
```

Read the 8 bytes immediately before our block's start — that's the _previous block's footer_. It contains the previous block's total size. Jump backward by that size to get the previous block's header address. If the previous block ends exactly where we begin (`prev_start + prev_node.size == node_ptr as usize`), they're adjacent. Merge: absorb our size into the previous node, update the footer, remove us from the list by pointing `prev_node.next` over us.

This is the boundary tag approach. Without footers, backward coalescing would require scanning from `head` every time — O(n) per free. Footers make it O(1).

---

## The Plumbing I Didn't Write (But Still Need to Understand)

The free list logic is mine. Everything below is Rust-specific infrastructure that makes the free list usable as a real allocator in a `no_std` kernel. I didn't write it from OSTEP — it's the "make Rust happy" layer.

---

### `src/utils/locked.rs` — Line by Line

```rust
use core::cell::UnsafeCell;
```

Import `UnsafeCell` from Rust's core library. This is the fundamental building block for interior mutability in Rust — the ability to mutate data through a shared (immutable) reference.

```rust
pub struct Locked<A> {
    inner: UnsafeCell<A>,
}
```

A generic wrapper around any type `A`. The `UnsafeCell` wrapping is what makes mutation through `&self` possible. Normally, Rust forbids mutating through `&T` — it's a compiler guarantee. `UnsafeCell` is the escape hatch: the compiler allows mutation through it, but _you_ are responsible for proving it's not causing data races.

```rust
unsafe impl<A> Sync for Locked<A> {}
```

By default, types containing `UnsafeCell` are not `Sync` (not safe to share between threads). Rust conservatively refuses to let you put them in `static` variables. This line overrides that conservatism and says "trust me, it's fine to share this across threads." This is `unsafe` because the programmer is making a promise the compiler can't verify.

For our single-threaded kernel this is actually true — no races can happen. In a multi-core kernel you'd need a real spinlock here.

```rust
impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: UnsafeCell::new(inner),
        }
    }
```

Constructor. It's `const fn` — meaning it can be evaluated at compile time. This is required because `ALLOCATOR` in `mod.rs` is a `static`, and statics must be initialized with compile-time-constant expressions. Without `const fn`, you can't use this in a `static` context.

```rust
    pub fn lock(&self) -> &mut A {
        unsafe { &mut *self.inner.get() }
    }
}
```

`self` is `&self` — an immutable reference. But this returns `&mut A` — a mutable reference to the inner value. This is the whole point: interior mutability.

`self.inner.get()` returns a raw `*mut A` pointer to the data inside `UnsafeCell`. We dereference it with `*` and then take a mutable reference `&mut`. The `unsafe` is required because dereferencing a raw pointer is always unsafe in Rust.

The name `lock()` is a bit aspirational — there's no actual lock here, no mutex, no spinlock. It's just "give me a mutable reference." In Phase 2, this would be replaced with an actual spinlock implementation that spins until it can acquire exclusive access.

---

### `src/memory/mod.rs` — What and Why

```rust
pub mod config;
pub mod heap;
```

Expose the submodules so the rest of the kernel can reference `memory::config::HEAP_START` and `memory::heap::FreeList`.

```rust
use super::utils::locked::Locked;
use config::{HEAP_SIZE, HEAP_START};
use heap::{FreeList, HeapType};
```

Pull in what we need. `super::utils` because `utils` is a sibling module to `memory`, both under `src/`.

```rust
#[global_allocator]
static ALLOCATOR: Locked<FreeList> = Locked::new(FreeList {
    head: None,
    start_address: 0,
    capacity: 0,
    heap_type: HeapType::BestFit,
    next_fit_cursor: None,
});
```

`#[global_allocator]` is a Rust attribute that registers this static as the allocator for `Box`, `Vec`, `String`, and everything else that uses the heap. Rust's standard library checks for this registration and routes all allocation calls through it.

The values are dummies (`None`, `0`, `0`) because Rust statics require compile-time-constant initialization, and `HEAP_START` is a constant we know at compile time — but `FreeList::init` does a memory write (`node_ptr.write(...)`) which can't happen at compile time. So we initialize with a dead allocator and replace it during boot.

Why `Locked<FreeList>` instead of just `FreeList`? Because `ALLOCATOR` is a `static`, which means Rust sees it as potentially shared across multiple threads. `GlobalAlloc::alloc` takes `&self` (immutable reference), but `FreeList::allocate` needs `&mut self` (mutable). You can't get `&mut` from `&` without interior mutability. `Locked<T>` provides exactly that.

```rust
pub fn init() {
    unsafe {
        let allocator = ALLOCATOR.lock();
        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }
}
```

Called once from `main.rs` during kernel boot. `ALLOCATOR.lock()` gives a `&mut FreeList`. `*allocator = ...` replaces the entire dead struct with a real initialized one. The `unsafe` is for calling `FreeList::init`, which does raw memory writes.

After this returns, `Box::new()` and `Vec::new()` work. Before this, any heap allocation panics.

```rust
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
```

`#[alloc_error_handler]` is the counterpart to `#[global_allocator]`. When `allocate()` returns null (allocation failed), Rust calls this. The return type `!` means it never returns — it must panic, infinite loop, or halt. We can't propagate allocation failures as `Result` in Rust's allocator interface, so the only sane response is to crash loud.

---

### `GlobalAlloc` in `src/memory/heap.rs` — The Bridge

```rust
unsafe impl GlobalAlloc for Locked<FreeList> {
```

`GlobalAlloc` is a trait defined in Rust's `core` library. It's the interface a `#[global_allocator]` must satisfy. It has two required methods: `alloc` and `dealloc`. The `unsafe impl` is because implementing this trait carries safety obligations — if you return a bad pointer, all of Rust's memory safety goes out the window.

```rust
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = self.lock();
        match allocator.allocate(layout.size()) {
            Some(ptr) => ptr,
            None => null_mut(),
        }
    }
```

`layout` tells us the size and alignment requirements. We only use `layout.size()` here — we trust that `align_up(8)` inside `allocate` handles the alignment (8-byte alignment satisfies most requests; a more robust implementation would respect `layout.align()`).

`self.lock()` gets the `&mut FreeList` from the `UnsafeCell`. We call our `allocate` method. On success, return the pointer. On failure, return `null_mut()` — Rust's allocator contract says null means failure, which then triggers `alloc_error_handler`.

```rust
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let allocator = self.lock();
        allocator.deallocate(ptr as usize);
    }
```

`_layout` is ignored with an underscore prefix — we don't need it because the block size is stored in the block's own header. We cast the pointer back to `usize` (because `deallocate` takes an address, not a typed pointer) and call our free list's deallocation logic.

---

## The Whole Picture

When you write `let x = Box::new(42)` in `main.rs`, this is the full call chain:

```
Box::new(42)
  └─ Rust runtime calls GlobalAlloc::alloc(&ALLOCATOR, layout)
       └─ alloc() in heap.rs
            └─ ALLOCATOR.lock() → &mut FreeList from UnsafeCell
                 └─ FreeList::allocate(8)
                      └─ align_up(8) = 8
                      └─ total_size = 8 + 24 = 32
                      └─ find_region_best_fit(32)
                           └─ head points to 2MB node → fits
                           └─ returns (node_ptr, None)
                      └─ node.size >= 32 + 24? yes, split
                      └─ write new free node at node_ptr + 32
                      └─ write footer at node_ptr + 32 - 8
                      └─ write footer for allocated block
                      └─ return node_ptr + 16 (past header)
  └─ x contains a pointer to heap memory at 0x280010 or wherever
```

Freeing:

```
drop(x)
  └─ Rust runtime calls GlobalAlloc::dealloc(&ALLOCATOR, ptr, layout)
       └─ dealloc() in heap.rs
            └─ FreeList::deallocate(ptr as usize)
                 └─ node_ptr = ptr - 16
                 └─ walk list to find insertion point by address
                 └─ insert in sorted order
                 └─ check forward coalesce with next node
                 └─ check backward coalesce with prev node via footer
```

---

## What I Would Do Differently

**NextFit is now fixed.** It tracks a persistent cursor and does wrap-around scanning. The strategy now behaves differently from First Fit as intended.

**`layout.align()` is ignored.** If someone requests 16-byte aligned memory (e.g., SIMD types), we don't guarantee it. For a kernel that doesn't use SIMD this is fine, but it's technically incorrect.

**`Locked<T>` is not actually a lock.** The name is misleading. It's a thin wrapper for interior mutability. Phase 2 needs to replace this with an actual spinlock that has an `AtomicBool` to prevent simultaneous access on multiple cores.

---

## OSTEP Reading Notes

These are my notes from reading chapters 13, 14, 15, and 17 — written in the order I understood things, not necessarily the order the book presents them.

---

### Chapter 13 — The Abstraction: Address Spaces

The first thing this chapter does is explain why we need the abstraction at all.

In the very early days of computing, there was no OS. The program loaded at some fixed address and had all of RAM to itself. Simple. Then people wanted to run multiple programs — not just one at a time, but multiple _at the same time_ (time sharing). You'd run program A for a bit, stop it, run program B, stop it, come back to A. This required keeping all their state in memory simultaneously.

So now you have a problem: program A thinks it owns memory at address `0x0000`. Program B also thinks it owns `0x0000`. If both are loaded in RAM at the same time, one of them is lying about where it actually lives. Someone has to manage this.

The solution is **virtual memory**. Each program sees a private address space that starts at 0 and looks like it has the entire machine to itself. But none of these addresses are real — the OS and hardware translate every memory access to a real physical address under the hood. The program never sees the physical address, never even knows about it.

The address space has a fixed layout in the abstraction:

```
High addresses
┌───────────────┐
│     Stack     │  grows downward ↓
│               │
│   (gap)       │
│               │
│     Heap      │  grows upward ↑
├───────────────┤
│     Data      │  global variables, statics
├───────────────┤
│     Code      │  the actual program instructions
└───────────────┘
Low addresses (0x0000)
```

Stack is at the top and grows downward. Heap is above the code/data segments and grows upward. They grow toward each other. If they ever actually meet, you run out of memory. The gap between them is "virtual" slack space — the OS doesn't have to back it with real RAM until you use it.

The book emphasizes three goals of virtual memory:

**Transparency** — programs shouldn't know or care that virtualization is happening. Write a C program, compile it, it runs the same whether it's the only program or one of fifty.

**Efficiency** — the translation has to be fast and it shouldn't waste too much real memory. You can't just dedicate a full gigabyte of physical RAM to each program's virtual space.

**Protection** — one program shouldn't be able to read or corrupt another program's memory, or the kernel's memory. The isolation has to be enforced by hardware, not just by being polite.

The mechanism for achieving this is discussed in chapter 15. Chapter 13 is all about establishing _why_ we need it.

One thing I found clarifying: the book makes it clear that the heap and stack are `malloc`/`free`'s problem and the function call stack's problem respectively — not the OS's problem directly. The OS provides a big region of virtual address space. What lives inside that space (heap, stack, data) is managed by either the runtime or the programmer. The OS doesn't care about `malloc`. It only manages at the level of pages or segments.

---

### Chapter 14 — Interlude: Memory API

This chapter is about `malloc` and `free` from the programmer's side — what the interface is, what goes wrong, and where it actually lives.

The key facts up front:

- `malloc(size)` takes a number of bytes, returns a `void*` pointer to allocated memory (or NULL on failure)
- `free(ptr)` takes the pointer back. It does **not** take the size. The allocator tracks the size internally using metadata stored near the pointer.
- You must `free` exactly once per `malloc`. Not twice. Not zero times.

The reason `free` doesn't need the size is that the allocator hides metadata near the returned pointer — typically just before it. When you call `free(ptr)`, the allocator walks back a few bytes, reads its metadata, and knows exactly how big the block is. This is exactly what the footer mechanism in our `heap.rs` does.

**Where does `malloc` actually live?**

This confused me at first reading. `malloc` is not a syscall. It's a library function in libc, running entirely in user space. The OS doesn't know about individual `malloc` calls. What the OS _does_ provide are lower-level syscalls that let the heap region grow:

- `brk()` / `sbrk()` — move the "program break" (the top of the heap segment) up or down. Old, simple.
- `mmap()` — ask the OS for a new anonymous mapping of pages. More modern, more flexible. A malloc implementation typically uses mmap for large allocations.

So the OS gives you a region of virtual memory. malloc manages that region. When malloc runs out of space in its region, it calls `brk()` or `mmap()` to ask the OS for more.

We're writing a kernel, so we're writing the malloc itself, and we don't have an OS underneath us to call. The entire heap comes from those constants in `config.rs`. We can't grow it dynamically — it's a fixed 2MB and that's it.

**The bugs the book covers**

The book spends a lot of time on common memory bugs, which are worth writing down because they're the kind of thing that causes mysterious crashes hours into a run:

**Forgetting to allocate at all:**

```c
char *s;
strcpy(s, "hello");  // s is uninitialized, points to garbage
```

**Not allocating enough — off by one:**

```c
char *s = malloc(strlen("hello"));  // Should be strlen("hello") + 1 for the \0
strcpy(s, "hello");  // Writes past the end
```

**Forgetting to initialize:** `malloc` does not zero the memory. You get whatever bytes were there before. `calloc` zeros it. If you're writing a kernel, this can leak sensitive data from previous uses.

**Use after free:** Call `free(ptr)`, then access `ptr` again. The memory might have been reused for something else. Silent corruption.

**Double free:** Call `free(ptr)` twice. The allocator's metadata gets corrupted. Typically crashes or, worse, becomes a security vulnerability.

**Freeing the wrong pointer:** If you did `ptr++` after malloc, and then `free(ptr)`, you're freeing the middle of a block. The metadata lookup goes to the wrong place.

In Rust, the ownership system prevents all of these. You can't double free — the first `drop` consumes the value. You can't use after free — the borrow checker ensures the reference is dead. You can't free the wrong pointer — `Box<T>` and `Vec<T>` handle deallocation internally. The bugs in chapter 14 are exactly what Rust's type system was designed to eliminate.

But in our kernel's `heap.rs`, we're writing the allocator itself in `unsafe` Rust. The borrow checker can't help us here — we're the ones the borrow checker relies on.

---

### Chapter 15 — Mechanism: Address Translation

This chapter explains how the hardware turns virtual addresses into physical addresses. The mechanism covered is **base and bounds** (also called dynamic relocation).

The idea is simple. The CPU has two special registers: a **base register** and a **bounds register**. When the OS sets up a process to run, it loads:

- Base = the physical address where this process's address space starts
- Bounds = the size of that address space

Every memory access the program makes gets translated automatically by the hardware:

```
physical address = virtual address + base
```

And before it touches memory, the hardware checks:

```
if virtual address >= bounds:
    raise SegmentationFault
```

This is done completely in hardware — in the Memory Management Unit (MMU). From the program's perspective, it's accessing address `0x1000`. The hardware silently adds the base and accesses `0x1000 + base` in physical memory. Zero software overhead per access.

Example:

```
Process A: base = 0x10000, bounds = 0x4000
Process B: base = 0x50000, bounds = 0x4000

Process A virtual 0x0000  →  physical 0x10000
Process A virtual 0x1000  →  physical 0x11000
Process A virtual 0x4001  →  TRAP (out of bounds)

Process B virtual 0x0000  →  physical 0x50000
Process B virtual 0x1000  →  physical 0x51000
```

Both processes think they start at 0. Neither can see the other's memory. Protection comes for free.

**Context switching** is where the OS work happens. When the OS switches from process A to process B:

1. Save process A's base and bounds registers to A's PCB (process control block)
2. Load process B's base and bounds from B's PCB into the hardware registers
3. Resume B

That's it. The hardware does translation for B now using B's values.

**The problem with base and bounds**

It's too coarse. The entire address space is one block. You allocate physical RAM for the full address space size, even if most of it is the gap between heap and stack. If a process has a 4GB virtual address space but only uses 1MB of actual data, you've wasted nearly 4GB of physical RAM.

The book calls this **internal fragmentation** at the OS level — wasted space inside an allocated region.

**Segmentation** (chapter 16, which I skipped for now since ARM uses paging not segmentation) tries to fix this by having separate base/bounds pairs for code, stack, and heap. So you only allocate physical RAM for the regions that are actually used, not the gap.

**What this means for our kernel**

Right now DDOS doesn't implement virtual memory at all. The kernel runs in physical address space. The heap at `0x280000` is a real physical address, not a virtual one. When we eventually add user-space processes, we'll need to set up page tables and teach the MMU to do address translation. But for Phase 1, it's irrelevant — we're kernel-only and we work with real addresses directly.

Understanding chapter 15 anyway matters because it explains _why_ `HEAP_START = 0x280000` is a real address we can just write to, and why user programs in a real OS can't do that.

---

### Chapter 17 — Free-Space Management

This is the chapter the actual allocator code comes from.

The setup: assume you have a region of memory to manage. Requests come in for arbitrary sizes. You hand out pointers. Later, those pointers get freed. You need to track what's available.

The structure the book uses to track free space is a **free list** — a linked list where each node represents one contiguous free region. The key insight is that the nodes live _inside the free memory they describe_. You're not allocating metadata from somewhere else; you embed it in the unused space. When memory is free, you use those bytes as a `FreeListNode`. When it's allocated, those bytes become the user's data.

**Splitting**

When a request comes in, you find a suitable free region. If that region is bigger than needed, you split it. The left part becomes the allocated block. The right part becomes a new, smaller free region. A new `FreeListNode` is written at the start of that right part and inserted into the free list.

```
Before:  [         256 bytes free         ]
Request 40 bytes
After:   [40 used][        216 free       ]
                   ^ new FreeListNode written here
```

**Coalescing**

Freeing is the inverse. When you return a block to the free list, you check if the block immediately before it is also free (backward coalescing) and if the block immediately after it is also free (forward coalescing). If either is true, merge them into one bigger free region.

```
Before:  [free 100][used 50][free 200]
Free the middle:
         [free 100][free 50][free 200]
Coalesce left:
         [   free 150      ][free 200]
Coalesce right:
         [        free 350           ]
```

Without coalescing, the free list fills up with tiny disconnected fragments. A 1000-byte request fails even if there are thousands of free bytes, because none of them are contiguous.

**The boundary tag trick (Knuth)**

Forward coalescing is easy — when you free a block, `node.next` in the free list tells you what's to the right. But backward coalescing is harder. You need to find the block immediately before you in memory, and the free list is only linked forwards.

The solution: store a footer at the _end_ of every block. The footer is just the block's size. When you want to find the previous block, read the 8 bytes immediately before your block's start — that's the previous block's footer. It tells you how big the previous block was. Jump back by that amount to reach the previous block's header.

```
┌─────────────────────────────────────────┐
│ Block N-1:                              │
│  [Header: size, next] [data] [footer:size] │
├─────────────────────────────────────────┤
│ Block N (just freed):                   │
│  [Header: size, next] [data] [footer:size] │
│   ^ We're here. Read 8 bytes before us  │
│     → get Block N-1's footer → get N-1's size │
│     → jump back size bytes → get Block N-1's header │
└─────────────────────────────────────────┘
```

Every block, free or allocated, has a footer. Overhead doubles (header + footer vs just header), but backward coalescing goes from O(n) to O(1). That's worth it.

**Allocation strategies**

Given multiple free regions that are large enough, which one do you pick?

_First Fit_ — take the first one you find. Fast. Fragments the beginning of the list over time.

_Best Fit_ — scan all of them, take the smallest one that fits. Minimizes waste per allocation. Still O(n) scan.

_Worst Fit_ — scan all, take the largest. Theoretically leaves "useful" large remainders. In practice causes more fragmentation than Best Fit. The book says don't use this.

_Next Fit_ — like First Fit but start scanning from where you left off last time, not from the beginning. Reduces clustering at the front of the list. Requires tracking a "cursor" pointer in the allocator state.

The book doesn't give a definitive winner. It depends on workload. For a long-running server doing millions of small allocations, First Fit with periodic compaction might win. For a kernel with predictable allocation patterns and a fixed heap size, Best Fit is a reasonable default.

**Fragmentation — two kinds**

_Internal fragmentation_: waste inside an allocated block. You asked for 13 bytes, we gave you 16 (aligned), 3 bytes are wasted but in your block. Unavoidable with alignment requirements. Bounded: at most `ALIGN - 1` bytes per allocation.

_External fragmentation_: waste across the free list. Total free bytes is enough but no single contiguous region is. Grows over time with random alloc/free patterns. Coalescing reduces it; no allocation strategy eliminates it entirely.

**Things the book mentions that we haven't done yet**

_Slab allocator_: keep a separate free list for each common object size. Allocating a 64-byte inode? Pull from the 64-byte slab. No splitting, no external fragmentation within a size class. Linux uses this.

_Buddy system_: always allocate in power-of-two sizes. Split blocks by halving. Merge by finding the "buddy" (the other half). Very easy to find buddies since they're always at predictable offsets. Internal fragmentation can be up to 50% (if you ask for 65 bytes you get 128).

These are the "more advanced" alternatives the book teases at the end of chapter 17. Future phases.

**The overhead problem (not in the book)**

Something chapter 17 glosses over: the metadata overhead per block matters a lot for small allocations. Here every block costs 24 bytes (16 header + 8 footer) plus any alignment padding. If you allocate a `u8` (1 byte), you actually use 32 bytes: 24 overhead + 8 aligned payload. That's 24x overhead.

For kernel code this is usually fine — we don't allocate millions of 1-byte objects. But it's worth knowing. If you ever want to support tiny allocations efficiently, slab allocators solve this by amortizing the overhead across many objects of the same size.

---

_DDOS Kernel — Daksh Desai_
_Phase 1 complete. Heap is live._
