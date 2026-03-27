pub(crate) mod config;
pub(crate) mod heap;

use super::utils::locked::SpinLock;
use config::{HEAP_SIZE, HEAP_START};
use heap::{FreeList, HeapType};

#[global_allocator]
static ALLOCATOR: SpinLock<FreeList> = SpinLock::new(FreeList::empty(HeapType::BestFit));

pub(crate) fn init() {
    unsafe {
        let mut allocator = ALLOCATOR.lock();

        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }
}
