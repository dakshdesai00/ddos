use super::frame::FRAME_ALLOCATOR;
use super::mmu::KERNEL_L0_TABLE;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const VALID: u64 = 1 << 0;
    pub const TABLE_OR_PAGE: u64 = 1 << 1;
    pub const ATTR_NORMAL: u64 = 0 << 2;
    pub const ATTR_DEVICE: u64 = 1 << 2;
    pub const USER_ACCESS: u64 = 1 << 6;
    pub const READ_ONLY: u64 = 1 << 7;
    pub const ACCESS_FLAG: u64 = 1 << 10;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn set(&mut self, physical_address: usize, flags: u64) {
        let addr_masked = (physical_address as u64) & 0x0000_FFFF_FFFF_F000;
        self.0 = addr_masked | flags;
    }

    pub fn is_valid(&self) -> bool {
        (self.0 & Self::VALID) != 0
    }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::empty(); 512],
        }
    }

    pub fn new_process_table() -> usize {
        let frame = FRAME_ALLOCATOR
            .lock()
            .alloc_frame()
            .expect("Out of memory for Page Table");
        let l0_table = unsafe { &mut *(frame as *mut PageTable) };
        *l0_table = PageTable::new();
        unsafe {
            let kernel_l0_ptr = core::ptr::addr_of!(KERNEL_L0_TABLE);
            for i in 0..512 {
                l0_table.entries[i] = (*kernel_l0_ptr).entries[i];
            }
        }
        frame
    }

    pub fn map_page(&mut self, virtual_addr: usize, physical_addr: usize, flags: u64) {
        let l0_index = (virtual_addr >> 39) & 0x1FF;
        let l1_index = (virtual_addr >> 30) & 0x1FF;
        let l2_index = (virtual_addr >> 21) & 0x1FF;
        let l3_index = (virtual_addr >> 12) & 0x1FF;
        let l0_entry = &mut self.entries[l0_index];

        if !l0_entry.is_valid() {
            let new_frame = FRAME_ALLOCATOR
                .lock()
                .alloc_frame()
                .expect("Out of Physical Memory!");
            l0_entry.set(
                new_frame,
                PageTableEntry::VALID | PageTableEntry::TABLE_OR_PAGE,
            );
        }

        let l1_phys_addr = (l0_entry.0 & 0x0000_FFFF_FFFF_F000) as usize;
        let l1_table = unsafe { &mut *(l1_phys_addr as *mut PageTable) };

        let l1_entry = &mut l1_table.entries[l1_index];

        if !l1_entry.is_valid() {
            let new_frame = FRAME_ALLOCATOR
                .lock()
                .alloc_frame()
                .expect("Out of Physical Memory!");
            l1_entry.set(
                new_frame,
                PageTableEntry::VALID | PageTableEntry::TABLE_OR_PAGE,
            );
        }

        let l2_phys_addr = (l1_entry.0 & 0x0000_FFFF_FFFF_F000) as usize;
        let l2_table = unsafe { &mut *(l2_phys_addr as *mut PageTable) };
        let l2_entry = &mut l2_table.entries[l2_index];

        if !l2_entry.is_valid() {
            let new_frame = FRAME_ALLOCATOR
                .lock()
                .alloc_frame()
                .expect("Out of Physical Memory!");
            l2_entry.set(
                new_frame,
                PageTableEntry::VALID | PageTableEntry::TABLE_OR_PAGE,
            );
        }

        let l3_phys_addr = (l2_entry.0 & 0x0000_FFFF_FFFF_F000) as usize;
        let l3_table = unsafe { &mut *(l3_phys_addr as *mut PageTable) };
        let l3_entry = &mut l3_table.entries[l3_index];
        l3_entry.set(
            physical_addr,
            flags | PageTableEntry::VALID | PageTableEntry::TABLE_OR_PAGE,
        );
    }
}
