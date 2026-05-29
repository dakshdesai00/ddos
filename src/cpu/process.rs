use crate::memory::config::PROCESS_STACK_TOP;
use crate::memory::frame::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::pagetable::{PageTable, PageTableEntry};
use alloc::vec::Vec;
use core::arch::global_asm;

global_asm!(
    ".global task_startup",
    "task_startup:",
    "    msr daifclr, #0b1111", // Forcefully unmask interrupts
    "    blr x19",              // Jump to the actual user entry point
    "1:  wfe",                  // Trap if it returns
    "    b 1b"
);

unsafe extern "C" {
    fn task_startup();
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum ProcessState {
    Ready,
    Running,
    Blocked,
    Dead,
}

pub(crate) struct Process {
    pid: u64,
    state: ProcessState,
    context: CpuContext,
    size_in_kb: u64,
    parent_pid: u64,
    current_priority: usize,
    tick_consumed: usize,
    stack: ProcessStack,
}

#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct CpuContext {
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    x29: u64,
    x30: u64,
    sp: u64,
    ttbr0: u64,
}

impl CpuContext {
    fn for_entry(entry_point: usize, sp: u64) -> Self {
        let mut context = CpuContext::default();
        context.sp = sp;
        context.x19 = entry_point as u64;
        context.x30 = task_startup as *const () as u64;
        context
    }
}

struct ProcessStack {
    base: usize,
    size: usize,
    frames: Vec<usize>,
}

impl ProcessStack {
    fn allocate(page_table: &mut PageTable, size_in_kb: u64) -> Self {
        let stack_size_bytes = (size_in_kb * 1024) as usize;
        let stack_pages = (stack_size_bytes + PAGE_SIZE - 1) / PAGE_SIZE;
        assert!(stack_pages > 0, "Stack size must be at least one page");

        let stack_size = stack_pages * PAGE_SIZE;
        let stack_top = PROCESS_STACK_TOP;
        let stack_base = stack_top
            .checked_sub(stack_size)
            .expect("Stack size exceeds virtual address space");

        let mut frames = Vec::with_capacity(stack_pages);
        for _ in 0..stack_pages {
            let frame = FRAME_ALLOCATOR
                .lock()
                .alloc_frame()
                .expect("Out of physical memory for process stack");
            frames.push(frame);
        }

        for (i, frame) in frames.iter().enumerate() {
            let vaddr = stack_base + i * PAGE_SIZE;
            page_table.map_page(vaddr, *frame, PageTableEntry::ACCESS_FLAG);
        }

        ProcessStack {
            base: stack_base,
            size: stack_size,
            frames,
        }
    }

    fn sp(&self) -> u64 {
        // 16-byte alignment required by ARM64 hardware
        ((self.base + self.size) & !0xF) as u64
    }
}

impl Drop for ProcessStack {
    fn drop(&mut self) {
        let mut allocator = FRAME_ALLOCATOR.lock();
        for &frame in &self.frames {
            allocator.free_frame(frame);
        }
    }
}

impl Process {
    pub(crate) fn new(pid: u64, size_in_kb: u64, parent_pid: u64, entry_point: usize) -> Self {
        // Keep the isolated Page Table!
        let pt_phys_addr = PageTable::new_process_table();
        let page_table = unsafe { &mut *(pt_phys_addr as *mut PageTable) };

        // Allocate the stack from physical frames and map it into the process address space.
        let stack = ProcessStack::allocate(page_table, size_in_kb);
        let sp_address = stack.sp();

        let mut context = CpuContext::for_entry(entry_point, sp_address);
        context.ttbr0 = pt_phys_addr as u64;

        Self {
            pid,
            state: ProcessState::Ready,
            context,
            size_in_kb,
            parent_pid,
            current_priority: 0,
            tick_consumed: 0,
            stack,
        }
    }

    pub(crate) fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
    pub(crate) fn kill(&mut self) {
        self.state = ProcessState::Dead;
    }
    pub(crate) fn is_dead(&self) -> bool {
        self.state == ProcessState::Dead
    }
    pub(crate) fn current_priority(&self) -> usize {
        self.current_priority
    }
    pub(crate) fn set_current_priority(&mut self, value: usize) {
        self.current_priority = value;
    }
    pub(crate) fn tick_consumed(&self) -> usize {
        self.tick_consumed
    }
    pub(crate) fn increment_tick_consumed(&mut self) {
        self.tick_consumed = self.tick_consumed.saturating_add(1);
    }
    pub(crate) fn reset_tick_consumed(&mut self) {
        self.tick_consumed = 0;
    }
    pub(crate) fn context_ptr_mut(&mut self) -> *mut CpuContext {
        &mut self.context
    }
    pub(crate) fn context_ptr(&self) -> *const CpuContext {
        &self.context
    }
}
