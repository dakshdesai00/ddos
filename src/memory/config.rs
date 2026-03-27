pub(crate) const KERNEL_START: usize = 0x80000;

pub(crate) const KERNEL_STACK_START: usize = 0x80000;

pub(crate) const HEAP_START: usize = KERNEL_STACK_START + 0x200000;

pub(crate) const HEAP_SIZE: usize = 0x200000;
