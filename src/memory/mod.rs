/*
 * memory/mod.rs - Memory Management System for DDOS
 *
 * NOTE: This is AI-generated initialization boilerplate.
 * User-implemented logic is in: config.rs, heap.rs (FreeList algorithm)
 *
 * This module coordinates the heap memory management system. It:
 * 1. Defines the global allocator instance
 * 2. Initializes the FreeList with real heap memory
 * 3. Handles allocation errors with panic
 *
 * Book References:
 * - OSTEP Chapter 13: Address Spaces
 * - OSTEP Chapter 14: Memory Virtualization
 * - OSTEP Chapter 15: Virtual Memory
 * - OSTEP Chapter 17: Free-Space Management (Core allocator concepts)
 */

pub mod config;
pub mod heap;

use core::alloc::Layout;
// Import your FreeList and the HeapType enum (e.g. BestFit)
use super::utils::locked::Locked;
use config::{HEAP_SIZE, HEAP_START};
use heap::{FreeList, HeapType};

// ============================================================================
// 1. THE GLOBAL ALLOCATOR INSTANCE
// ============================================================================

/*
 * ALLOCATOR - The system-wide memory allocator
 *
 * Attributes:
 * - #[global_allocator]: Registers this as THE allocator for Box, Vec, etc.
 * - Locked<FreeList>: Wraps FreeList with interior mutability for safe mutation
 * - Static: Lives for entire program lifetime
 *
 * Initialization:
 * We initialize with dummy values (None, 0, 0) because we can't use HEAP_START
 * and HEAP_SIZE in const contexts. The real init happens in init() function below.
 *
 * Allocation Strategy:
 * Using BestFit - finds smallest suitable free region for each allocation.
 * This minimizes wasted space compared to FirstFit or WorstFit.
 * You can change to: FirstFit (faster), WorstFit (rarely used), NextFit (hybrid)
 */
#[global_allocator]
static ALLOCATOR: Locked<FreeList> = Locked::new(FreeList {
    head: None,
    start_address: 0,
    capacity: 0,
    // You can choose BestFit, WorstFit, FirstFit, or NextFit here!
    heap_type: HeapType::BestFit,
});

// ============================================================================
// 2. INITIALIZATION FUNCTION
// ============================================================================

/*
 * init() - Initializes the heap allocator with real memory
 *
 * This function MUST be called early in kernel startup (from main.rs).
 * It replaces the dummy allocator with the real one pointing to actual heap memory.
 *
 * How it works:
 * 1. Get mutable access to the global ALLOCATOR (via lock())
 * 2. Call FreeList::init() with real heap parameters:
 *    - HEAP_START: Starting physical address (0x280000)
 *    - HEAP_SIZE: Total heap size in bytes (2 MB)
 *    - HeapType::BestFit: Allocation strategy
 * 3. Replaces the dummy allocator structure with real initialized one
 *
 * Why unsafe?
 * We're modifying a global static variable, which is only safe if:
 * - Called exactly once during startup (before any allocations)
 * - No other threads access the allocator during reinitialization
 * - We're single-threaded during init, so this is safe
 *
 * Result:
 * After this call, Box::new(), Vec::new(), etc. all work and use our FreeList
 */
pub fn init() {
    unsafe {
        let allocator = ALLOCATOR.lock();
        // We re-initialize the allocator with the real heap memory
        // and your choice of algorithm (e.g. BestFit)
        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }
}

// ============================================================================
// 3. ERROR HANDLER
// ============================================================================

/*
 * alloc_error_handler() - Called when allocation fails
 *
 * When Box::new(), Vec::new(), or other allocation fails (returns null),
 * Rust calls this handler. We can't recover from allocation failure in a kernel,
 * so we panic with an error message.
 *
 * Parameters:
 * - layout: The Layout that failed to allocate
 *           Contains: size and alignment requirements
 *
 * Returns: Never (!) - this always panics
 *
 * Why debuggable:
 * Printing the layout helps identify what was being allocated when we ran out of memory.
 * In a production system, you might:
 * - Trigger garbage collection (we don't have one)
 * - Swap to disk (Linux does this)
 * - Terminate processes (beyond scope of basic OS)
 * - Just crash (what we do now)
 *
 * Preventing this error:
 * Make sure HEAP_SIZE in config.rs is large enough for your workload.
 * If you see this panic, increase HEAP_SIZE and rebuild.
 */
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
