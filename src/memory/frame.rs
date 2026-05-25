use super::super::utils::locked::TicketLock;

pub const PAGE_SIZE: usize = 4096;

pub(crate) static FRAME_ALLOCATOR: TicketLock<FrameAllocator> =
    TicketLock::new(FrameAllocator::new());

#[repr(C)]
pub struct FreeFrameNode {
    pub next: Option<*mut FreeFrameNode>,
}

pub struct FrameAllocator {
    pub current_address: usize,
    pub ram_end: usize,
    pub free_list_head: Option<*mut FreeFrameNode>,

    // Optional tracker to see how much memory your OS is consuming
    pub allocated_frames: usize,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        FrameAllocator {
            current_address: 0,
            ram_end: 0,
            free_list_head: None,
            allocated_frames: 0,
        }
    }

    pub fn init(&mut self, start: usize, end: usize) {
        self.current_address = (start + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        self.ram_end = end & !(PAGE_SIZE - 1);
    }

    pub fn alloc_frame(&mut self) -> Option<usize> {
        // 1. Check the Recycled Free List first
        if let Some(node_ptr) = self.free_list_head {
            unsafe {
                self.free_list_head = (*node_ptr).next;

                // CRITICAL: Zeroing the frame before handeling it to a process is a MUST to prevent data leaks and security vulnerabilities as
                // we are not doing this during the free_frame() call, we must do it here before handing it to a new process. This ensures that any sensitive data from the previous process is wiped clean.
                core::ptr::write_bytes(node_ptr as *mut u8, 0, PAGE_SIZE);

                self.allocated_frames += 1;
                return Some(node_ptr as usize);
            }
        }

        // 2. If the Free List is empty, fallback to the Bump Pointer
        if self.current_address + PAGE_SIZE <= self.ram_end {
            let frame_address = self.current_address;
            self.current_address += PAGE_SIZE;

            unsafe {
                core::ptr::write_bytes(frame_address as *mut u8, 0, PAGE_SIZE);
            }

            self.allocated_frames += 1;
            return Some(frame_address);
        }
        None
    }

    pub fn free_frame(&mut self, physical_address: usize) {
        // Safety Checks to prevent kernel panics later
        assert!(
            physical_address % PAGE_SIZE == 0,
            "Freed frame is not 4KB aligned!"
        );
        assert!(
            physical_address < self.ram_end,
            "Freed frame is outside RAM bounds!"
        );

        // Cast the raw address into a FreeFrameNode pointer
        let node_ptr = physical_address as *mut FreeFrameNode;

        unsafe {
            // Point this new node at the current head of the list
            (*node_ptr).next = self.free_list_head;
        }
        self.free_list_head = Some(node_ptr);
        self.allocated_frames -= 1;
    }
}
