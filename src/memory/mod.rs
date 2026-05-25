pub(crate) mod config;
pub(crate) mod frame;
pub(crate) mod heap;
pub(crate) mod layout;
pub(crate) mod mmu;
pub(crate) mod pagetable;
use super::utils::locked::TicketLock;
use config::{HEAP_SIZE, HEAP_START};
use frame::FRAME_ALLOCATOR;
use heap::{FreeList, HeapType};

#[global_allocator]
static ALLOCATOR: TicketLock<FreeList> = TicketLock::new(FreeList::empty(HeapType::BestFit));

pub(crate) fn init() {
    unsafe {
        let mut allocator = ALLOCATOR.lock();
        *allocator = FreeList::init(HEAP_START, HEAP_SIZE, HeapType::BestFit);
    }

    let physical_memory_start = HEAP_START + HEAP_SIZE;

    FRAME_ALLOCATOR
        .lock()
        .init(physical_memory_start, layout::RAM_END);

    unsafe {
        mmu::init();
    }
}
