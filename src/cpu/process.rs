use alloc::vec::Vec;
use core::arch::global_asm;

// This invisible function wraps every new task.
global_asm!(
    ".global task_startup",
    "task_startup:",
    "    msr daifclr, #0b1111", // 1. Forcefully unmask interrupts!
    "    blr x19",              // 2. Jump to the actual user entry point stored in x19
    "1:  wfe", // 3. If the user task ever 'returns', trap it here so it doesn't crash the OS
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
    stack: Vec<u8>,
    parent_pid: u64,
    current_priority: usize,
    tick_consumed: usize,
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
    x29: u64, // Frame Pointer
    x30: u64, // Link Register (Return Address)
    sp: u64,  // Stack Pointer
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

impl Process {
    pub(crate) fn new(pid: u64, size_in_kb: u64, parent_pid: u64, entry_point: usize) -> Self {
        let stack_size = (size_in_kb * 1024) as usize;
        let stack = alloc::vec![0; stack_size];

        let sp_address = (stack.as_ptr() as u64 + stack.len() as u64) & !0xF;

        let context = CpuContext::for_entry(entry_point, sp_address);

        Self {
            pid,
            state: ProcessState::Ready,
            context,
            size_in_kb,
            stack,
            parent_pid,
            current_priority: 0,
            tick_consumed: 0,
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
