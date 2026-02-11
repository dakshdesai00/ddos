use super::super::utils::locked::Locked;
use core::alloc::{GlobalAlloc, Layout}; // This line is AI GEN
use core::mem::size_of;
use core::ptr::null_mut; // This line is AI GEN

// HERE ALL THE CODE WHICH HAS LOGIC OF FREELIST AND FREELISTNODE IS WRITTEN BY ME BUT THE WRAPPER WITH GLOBAL ALLOC IS WRITTEN BY AI BECAUSE
// I DONT KNOW THIS PART I READ THEORY FROM OSTEP BOOK

const ALIGN: usize = 8;

pub enum HeapType {
    BestFit,
    WorstFit,
    FirstFit,
    NextFit,
}

pub struct FreeList {
    pub head: Option<*mut FreeListNode>,
    pub start_address: usize,
    pub capacity: usize,
    pub heap_type: HeapType,
}

pub struct FreeListNode {
    size: usize,
    next: Option<*mut FreeListNode>,
}

impl FreeListNode {
    fn new(size: usize, next: Option<*mut FreeListNode>) -> Self {
        FreeListNode { size, next }
    }
}

impl FreeList {
    pub unsafe fn init(start: usize, capacity: usize, heap_type: HeapType) -> Self {
        let node_ptr = start as *mut FreeListNode;
        unsafe {
            node_ptr.write(FreeListNode::new(capacity, None));
        }

        FreeList {
            head: Some(node_ptr),
            start_address: start,
            capacity,
            heap_type,
        }
    }

    fn find_region_best_fit(
        &mut self,
        requested_size: usize,
    ) -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) {
        let mut current = self.head;
        let mut prev: Option<*mut FreeListNode> = None;

        let mut best: Option<*mut FreeListNode> = None;
        let mut best_prev: Option<*mut FreeListNode> = None;

        if requested_size >= self.capacity {
            return (None, None);
        }

        while let Some(node_ptr) = current {
            unsafe {
                let node = &*node_ptr;

                if node.size >= requested_size {
                    if best.is_none() || node.size < (*best.unwrap()).size {
                        best = Some(node_ptr);
                        best_prev = prev;
                    }
                }

                prev = current;
                current = node.next;
            }
        }

        (best, best_prev)
    }

    fn find_region_worst_fit(
        &mut self,
        requested_size: usize,
    ) -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) {
        let mut current = self.head;
        let mut prev: Option<*mut FreeListNode> = None;

        let mut worst: Option<*mut FreeListNode> = None;
        let mut worst_prev: Option<*mut FreeListNode> = None;

        if requested_size >= self.capacity {
            return (None, None);
        }

        while let Some(node_ptr) = current {
            unsafe {
                let node = &*node_ptr;

                if node.size >= requested_size {
                    if worst.is_none() || node.size > (*worst.unwrap()).size {
                        worst = Some(node_ptr);
                        worst_prev = prev;
                    }
                }

                prev = current;
                current = node.next;
            }
        }

        (worst, worst_prev)
    }

    fn find_region_next_fit(
        &mut self,
        requested_size: usize,
    ) -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) {
        let mut current = self.head;
        let mut prev: Option<*mut FreeListNode> = None;

        if requested_size >= self.capacity {
            return (None, None);
        }

        while let Some(node_ptr) = current {
            unsafe {
                let node = &*node_ptr;

                if node.size >= requested_size {
                    return (Some(node_ptr), prev);
                }

                prev = current;
                current = node.next;
            }
        }

        (None, None)
    }

    fn find_region_first_fit(
        &mut self,
        requested_size: usize,
    ) -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) {
        let mut current = self.head;
        let mut prev = None;

        if requested_size >= self.capacity {
            return (None, None);
        }

        while let Some(node_ptr) = current {
            unsafe {
                let node = &*node_ptr;

                if node.size >= requested_size {
                    return (Some(node_ptr), prev);
                }

                prev = current;
                current = node.next;
            }
        }

        (None, None)
    }

    fn align_up(size: usize) -> usize {
        (size + ALIGN - 1) & !(ALIGN - 1)
    }

    fn block_overhead() -> usize {
        size_of::<FreeListNode>() + size_of::<usize>()
    }

    pub fn allocate(&mut self, requested_size: usize) -> Option<*mut u8> {
        // Align payload size
        let aligned_payload = Self::align_up(requested_size);

        // Total block size = header + payload + footer
        let total_size = aligned_payload + Self::block_overhead();

        let (region, prev) = match self.heap_type {
            HeapType::FirstFit => self.find_region_first_fit(total_size),
            HeapType::BestFit => self.find_region_best_fit(total_size),
            HeapType::WorstFit => self.find_region_worst_fit(total_size),
            HeapType::NextFit => self.find_region_next_fit(total_size),
        };

        let node_ptr = region?;

        unsafe {
            let node = &mut *node_ptr;

            if node.size >= total_size + Self::block_overhead() {
                let remaining_size = node.size - total_size;

                let new_node_ptr = (node_ptr as *mut u8).add(total_size) as *mut FreeListNode;

                new_node_ptr.write(FreeListNode::new(remaining_size, node.next));
                let new_footer =
                    (new_node_ptr as usize + remaining_size - size_of::<usize>()) as *mut usize;

                new_footer.write(remaining_size);

                if let Some(prev_ptr) = prev {
                    (*prev_ptr).next = Some(new_node_ptr);
                } else {
                    self.head = Some(new_node_ptr);
                }

                node.size = total_size;
            } else {
                if let Some(prev_ptr) = prev {
                    (*prev_ptr).next = node.next;
                } else {
                    self.head = node.next;
                }
            }

            let footer_ptr = (node_ptr as usize + node.size - size_of::<usize>()) as *mut usize;

            footer_ptr.write(node.size);

            Some((node_ptr as *mut u8).add(size_of::<FreeListNode>()))
        }
    }

    pub fn deallocate(&mut self, address: usize) {
        unsafe {
            let node_ptr = (address - size_of::<FreeListNode>()) as *mut FreeListNode;

            let node = &mut *node_ptr;

            let mut current = self.head;
            let mut prev: Option<*mut FreeListNode> = None;

            while let Some(curr_ptr) = current {
                if curr_ptr as usize > node_ptr as usize {
                    break;
                }
                prev = current;
                current = (*curr_ptr).next;
            }

            node.next = current;

            if let Some(prev_ptr) = prev {
                (*prev_ptr).next = Some(node_ptr);
            } else {
                self.head = Some(node_ptr);
            }

            if let Some(next_ptr) = node.next {
                let node_end = node_ptr as usize + node.size;

                if node_end == next_ptr as usize {
                    node.size += (*next_ptr).size;
                    node.next = (*next_ptr).next;

                    let footer = (node_ptr as usize + node.size - size_of::<usize>()) as *mut usize;

                    footer.write(node.size);
                }
            }

            if node_ptr as usize > self.start_address {
                let prev_footer_ptr = (node_ptr as usize - size_of::<usize>()) as *mut usize;

                let prev_size = prev_footer_ptr.read();
                let prev_start = node_ptr as usize - prev_size;

                if prev_start >= self.start_address {
                    let prev_node_ptr = prev_start as *mut FreeListNode;
                    let prev_node = &mut *prev_node_ptr;

                    if prev_start + prev_node.size == node_ptr as usize {
                        prev_node.size += node.size;
                        prev_node.next = node.next;

                        let footer =
                            (prev_start + prev_node.size - size_of::<usize>()) as *mut usize;

                        footer.write(prev_node.size);
                    }
                }
            }
        }
    }
}

// The code below is ai gem

unsafe impl GlobalAlloc for Locked<FreeList> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = self.lock();

        match allocator.allocate(layout.size()) {
            Some(ptr) => ptr,
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let allocator = self.lock();
        allocator.deallocate(ptr as usize);
    }
}
