use super::super::utils::locked::Locked;
use core::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;
use core::ptr::null_mut;

const ALIGN: usize = 16;

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
    pub(crate) next_fit_cursor: Option<*mut FreeListNode>,
}

#[repr(C, align(16))]
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
        let aligned_start = Self::align_up(start).expect("heap start alignment overflow");
        let alignment_loss = aligned_start - start;
        let usable_capacity = capacity.saturating_sub(alignment_loss) & !(ALIGN - 1);

        assert!(usable_capacity >= Self::block_overhead());

        let node_ptr = aligned_start as *mut FreeListNode;
        unsafe {
            node_ptr.write(FreeListNode::new(usable_capacity, None));

            let footer = (aligned_start + usable_capacity - size_of::<usize>()) as *mut usize;
            footer.write(usable_capacity);
        }

        FreeList {
            head: Some(node_ptr),
            start_address: aligned_start,
            capacity: usable_capacity,
            heap_type,
            next_fit_cursor: Some(node_ptr),
        }
    }

    fn contains_node(&self, target: *mut FreeListNode) -> bool {
        let mut current = self.head;

        while let Some(node_ptr) = current {
            if node_ptr == target {
                return true;
            }

            unsafe {
                current = (*node_ptr).next;
            }
        }

        false
    }

    fn find_prev_node(&self, target: *mut FreeListNode) -> Option<*mut FreeListNode> {
        let mut current = self.head;
        let mut prev = None;

        while let Some(node_ptr) = current {
            if node_ptr == target {
                return prev;
            }

            unsafe {
                prev = current;
                current = (*node_ptr).next;
            }
        }

        None
    }

    fn find_region_best_fit(
        &mut self,
        requested_size: usize,
    ) -> (Option<*mut FreeListNode>, Option<*mut FreeListNode>) {
        let mut current = self.head;
        let mut prev: Option<*mut FreeListNode> = None;

        let mut best: Option<*mut FreeListNode> = None;
        let mut best_prev: Option<*mut FreeListNode> = None;

        if requested_size > self.capacity {
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

        if requested_size > self.capacity {
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
        if requested_size > self.capacity {
            return (None, None);
        }

        let head = match self.head {
            Some(head) => head,
            None => return (None, None),
        };

        let start = match self.next_fit_cursor {
            Some(cursor) if self.contains_node(cursor) => cursor,
            _ => head,
        };

        self.next_fit_cursor = Some(start);

        let mut current = Some(start);
        let mut prev = self.find_prev_node(start);

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

        current = self.head;
        prev = None;

        while let Some(node_ptr) = current {
            if node_ptr == start {
                break;
            }

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

        if requested_size > self.capacity {
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

    fn align_up(size: usize) -> Option<usize> {
        size.checked_add(ALIGN - 1).map(|s| s & !(ALIGN - 1))
    }

    fn block_overhead() -> usize {
        size_of::<FreeListNode>() + size_of::<usize>()
    }

    pub fn allocate(&mut self, requested_size: usize, requested_align: usize) -> Option<*mut u8> {
        if requested_align > ALIGN {
            return None;
        }

        let request = requested_size.max(1);
        let aligned_payload = Self::align_up(request)?;
        let raw_total = aligned_payload.checked_add(Self::block_overhead())?;
        let total_size = Self::align_up(raw_total)?;

        let (region, prev) = match self.heap_type {
            HeapType::FirstFit => self.find_region_first_fit(total_size),
            HeapType::BestFit => self.find_region_best_fit(total_size),
            HeapType::WorstFit => self.find_region_worst_fit(total_size),
            HeapType::NextFit => self.find_region_next_fit(total_size),
        };

        let node_ptr = region?;

        unsafe {
            let node = &mut *node_ptr;
            let next_cursor;

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

                next_cursor = Some(new_node_ptr);

                node.size = total_size;
            } else {
                let next_node = node.next;

                if let Some(prev_ptr) = prev {
                    (*prev_ptr).next = next_node;
                } else {
                    self.head = next_node;
                }

                next_cursor = next_node.or(self.head);
            }

            self.next_fit_cursor = next_cursor.or(self.head);

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

            self.next_fit_cursor = self.head;
        }
    }
}

unsafe impl GlobalAlloc for Locked<FreeList> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = self.lock();

        match allocator.allocate(layout.size(), layout.align()) {
            Some(ptr) => ptr,
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let allocator = self.lock();
        allocator.deallocate(ptr as usize);
    }
}
