pub(crate) const KERNEL_START: usize = 0x80000;

pub(crate) const KERNEL_STACK_START: usize = 0x80000;

pub(crate) const HEAP_START: usize = KERNEL_STACK_START + 0x200000;

pub(crate) const HEAP_SIZE: usize = 0x200000;

// Top of the per-process stack virtual region (must be canonical for 48-bit VA and page aligned)
pub(crate) const PROCESS_STACK_TOP: usize = 0x0000_7FFF_FFFF_0000;
