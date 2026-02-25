pub mod config;
pub mod heap;

use core::alloc::Layout;

use super::utils::locked::Locked;
use config::{HEAP_SIZE, HEAP_START};
use heap::{FreeList, HeapType};

#[global_allocator]
static ALLOCATOR: Locked<FreeList> = Locked::new(FreeList {
    head: None,
    start_address: 0,
    capacity: 0,

    heap_type: HeapType::BestFit,
    next_fit_cursor: None,
});

pub fn init() {
    unsafe {
        let allocator = ALLOCATOR.lock();

        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
