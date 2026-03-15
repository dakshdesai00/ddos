use alloc::vec::Vec;

#[derive(Debug, PartialEq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Dead,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct CpuContext {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64, // Frame Pointer
    pub x30: u64, // Link Register (The address to return to) or also pc
    pub sp: u64,  // Stack Pointer
}

pub struct Process {
    pub pid: u64,
    pub state: ProcessState,
    pub context: CpuContext,
    pub size_in_kb: u64,
    pub stack: Vec<u8>,
    pub parent_pid: u64,
}

impl Process {
    pub fn new(pid: u64, size_in_kb: u64, parent_pid: u64) -> Self {
        Self {
            pid,
            state: ProcessState::Ready,
            context: CpuContext::default(),
            size_in_kb,
            stack: alloc::vec![0; (size_in_kb * 1024) as usize],
            parent_pid,
        }
    }
}
