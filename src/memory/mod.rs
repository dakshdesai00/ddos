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
// We create a static instance of your FreeList, wrapped in a Lock.
// We initialize it with dummy values because we can't use HEAP_START yet.
#[global_allocator]
static ALLOCATOR: Locked<FreeList> = Locked::new(FreeList {
    head: None,
    start_address: 0,
    capacity: 0,
    // You can choose BestFit, WorstFit, FirstFit, or NextFit here!
    heap_type: HeapType::BestFit,
});

// ============================================================================
// 2. INITIALIZATION
// ============================================================================
// Main.rs calls this to give the allocator real memory.
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
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
