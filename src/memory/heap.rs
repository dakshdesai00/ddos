/*
 * heap.rs - Free List Allocator Implementation for DDOS
 *
 * WRITTEN BY: User (user-implemented logic)
 * GlobalAlloc wrapper and basic boilerplate: AI-generated
 *
 * This module implements a free list memory allocator using multiple allocation strategies.
 * It's the core memory management system for dynamic allocation (malloc/Box/Vec/etc).
 *
 * Book References:
 * - OSTEP Chapter 13: Address Spaces - Virtual memory concepts
 * - OSTEP Chapter 14: Memory Virtualization - Memory management overview
 * - OSTEP Chapter 15: Virtual Memory - Paging and segmentation
 * - OSTEP Chapter 17: Free-Space Management - Free list algorithms (THIS IS THE CORE)
 *
 * Algorithm Overview (Chapter 17, OSTEP):
 * The free list maintains a linked list of free memory regions. When allocating:
 * 1. Search the free list for a suitable region using a strategy (First/Best/Worst/Next-Fit)
 * 2. Split the region if it's larger than needed
 * 3. Mark the allocated portion with a header containing size information
 *
 * When deallocating:
 * 1. Find the header before the pointer
 * 2. Reinsert into free list in address order
 * 3. Coalesce with adjacent free regions to prevent fragmentation
 */

use super::super::utils::locked::Locked;
use core::alloc::{GlobalAlloc, Layout}; // This line is AI GEN
use core::mem::size_of;
use core::ptr::null_mut; // This line is AI GEN

// ============================================================================
// ALIGNMENT AND CONSTANTS
// ============================================================================

/*
 * ALIGN - Minimum alignment for allocated blocks in bytes
 *
 * Value: 8 bytes (on 64-bit systems)
 *
 * Why 8 bytes?
 * - Provides good alignment for most data types (pointers, u64, etc.)
 * - Reduces fragmentation compared to smaller alignments
 * - Standard for many OS allocators
 * - OSTEP Chapter 17 discusses alignment tradeoffs
 */
const ALIGN: usize = 8;

// ============================================================================
// ENUMS AND CONFIGURATION
// ============================================================================

/*
 * HeapType - Allocation strategy selector
 *
 * Different strategies have different performance/fragmentation characteristics:
 *
 * BestFit - Allocate from the smallest suitable region
 * - Pros: Low external fragmentation (finds tight fits)
 * - Cons: Slower search, may split more regions
 * - Use case: System needs free memory efficiency
 *
 * WorstFit - Allocate from the largest suitable region
 * - Pros: May produce fewer small fragments
 * - Cons: Higher fragmentation, slower search
 * - Use case: Rarely used; generally performs worse
 *
 * FirstFit - Allocate from the first suitable region
 * - Pros: Fastest search (stops at first match)
 * - Cons: May leave fragments at start of list
 * - Use case: When search speed is critical
 *
 * NextFit - Like FirstFit but remembers last position
 * - Pros: Faster than BestFit, distributes allocations
 * - Cons: More complex to implement
 * - Use case: Good balance of speed and fragmentation
 */
pub enum HeapType {
    BestFit,
    WorstFit,
    FirstFit,
    NextFit,
}

// ============================================================================
// CORE DATA STRUCTURES
// ============================================================================

/*
 * FreeList - The main allocator data structure
 *
 * Fields:
 * - head: Pointer to first free region (linked list head)
 *         Option<*mut FreeListNode> allows None for empty heap
 *
 * - start_address: Base address of the heap region (from config.rs)
 *
 * - capacity: Total heap size in bytes (from config.rs)
 *
 * - heap_type: Which allocation strategy to use (FirstFit, BestFit, etc.)
 *
 * How it works:
 * The free list is a linked list where each node represents a contiguous
 * region of free memory. When memory is allocated, we remove a node (or part
 * of it) from the free list. When deallocated, we reconstruct nodes and
 * merge with adjacent regions.
 */
pub struct FreeList {
    pub head: Option<*mut FreeListNode>,
    pub start_address: usize,
    pub capacity: usize,
    pub heap_type: HeapType,
}

/*
 * FreeListNode - Metadata for a single free memory region
 *
 * Fields:
 * - size: Total size of this free region in bytes (includes this header)
 *
 * - next: Pointer to next free region in the linked list
 *         Maintained in address order (sorted) for coalescing
 *
 * Memory Layout:
 * When allocated, memory looks like:
 * [FreeListNode header (24 bytes)] [Allocated data...] [Footer with size]
 *
 * The footer allows us to find the previous block during deallocation
 * (walk backward from pointer to find the previous block's footer)
 */
pub struct FreeListNode {
    size: usize,                     // Size of entire block (header + data + footer)
    next: Option<*mut FreeListNode>, // Next free region in sorted order
}

impl FreeListNode {
    fn new(size: usize, next: Option<*mut FreeListNode>) -> Self {
        FreeListNode { size, next }
    }
}

// ============================================================================
// FREELIST IMPLEMENTATION
// ============================================================================

impl FreeList {
    /*
     * Initializes a new FreeList allocator
     *
     * Parameters:
     * - start: Base address of heap (from config::HEAP_START)
     * - capacity: Total heap size in bytes (from config::HEAP_SIZE)
     * - heap_type: Allocation strategy to use
     *
     * Returns: FreeList with entire heap as one free region
     *
     * How it works:
     * 1. Create the first FreeListNode at the start address
     * 2. Initialize it with capacity as size and no next pointer
     * 3. All of the heap is initially free
     *
     * Why unsafe?
     * We're writing to arbitrary memory addresses (the heap). This is only
     * safe because we've been given the heap_region from config.rs which is
     * a valid, writable memory region.
     */
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

    /*
     * Finds a suitable free region using Best-Fit strategy
     *
     * Best-Fit: Find the smallest region that fits the requested size
     *
     * Parameters:
     * - requested_size: Bytes needed (already aligned)
     *
     * Returns: Tuple of (found_node_ptr, previous_node_ptr)
     *          - found_node_ptr: The best-fitting region to allocate from
     *          - previous_node_ptr: Previous node in linked list (for removal)
     *          - Returns (None, None) if no suitable region exists
     *
     * Algorithm:
     * 1. Walk entire free list once
     * 2. Track the smallest region >= requested_size
     * 3. Return both the found node and its predecessor
     *
     * Time complexity: O(n) where n = number of free regions
     */
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

    /*
     * Finds a suitable free region using Worst-Fit strategy
     *
     * Worst-Fit: Find the largest region >= requested_size
     *
     * Theory: By using largest available space, we might avoid creating
     * many tiny unusable fragments. Reality: Doesn't work well in practice.
     *
     * Parameters & Returns: Same as find_region_best_fit
     */
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

    /*
     * Finds a suitable free region using Next-Fit strategy
     *
     * Next-Fit: Like FirstFit, but remembers the last allocation point
     * and starts searching from there next time.
     *
     * This reduces clustering of allocations at the start of the free list.
     *
     * OSTEP Note: In this implementation, we always search from head
     * (doesn't truly maintain state), so it behaves like FirstFit.
     * A full implementation would track the last search position.
     */
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

    /*
     * Finds a suitable free region using First-Fit strategy
     *
     * First-Fit: Return the first region that's large enough
     *
     * Pros:
     * - Fastest (stops at first match, O(1) typical case)
     * - Simple to understand
     *
     * Cons:
     * - Leaves fragments at the start of the free list
     * - Can reduce overall free memory utility
     *
     * Time complexity: O(1) to O(n) depending on fragmentation
     */
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

    /*
     * Aligns a size up to the nearest multiple of ALIGN
     *
     * Formula: (size + ALIGN - 1) & ~(ALIGN - 1)
     *
     * Example with ALIGN=8:
     * - align_up(1) = 8
     * - align_up(8) = 8
     * - align_up(9) = 16
     *
     * Why bitwise operations?
     * More efficient than division/modulo on most CPUs
     * The mask ~(ALIGN-1) zeros out the low bits
     */
    fn align_up(size: usize) -> usize {
        (size + ALIGN - 1) & !(ALIGN - 1)
    }

    /*
     * Calculates memory overhead per allocated block
     *
     * Returns: size_of::<FreeListNode>() + size_of::<usize>()
     *
     * Breakdown:
     * - FreeListNode (24 bytes on 64-bit): Header with size and next pointer
     * - usize (8 bytes): Footer containing size for backward coalescing
     *
     * Total: 32 bytes per allocation
     *
     * This overhead is included in total_size when allocating
     */
    fn block_overhead() -> usize {
        size_of::<FreeListNode>() + size_of::<usize>()
    }

    /*
     * Allocates memory for a given size using the selected strategy
     *
     * Parameters:
     * - requested_size: Bytes requested (before alignment)
     *
     * Returns:
     * - Some(ptr): Pointer to allocated memory (after header)
     * - None: Allocation failed (not enough contiguous memory)
     *
     * Algorithm (from OSTEP Chapter 17):
     * 1. Align the requested size
     * 2. Calculate total size needed (header + aligned payload + footer)
     * 3. Find a suitable free region using selected strategy
     * 4. If region is larger than needed:
     *    a. Split it: keep needed amount, create new free region for remainder
     *    b. Reinsert remainder into free list in sorted order
     * 5. Mark the allocated block with header and footer containing size
     * 6. Return pointer (after header) to caller
     *
     * Memory Layout after allocation:
     * [Header: FreeListNode] [Allocated data...] [Footer: size]
     *                       ^ pointer returned
     */
    pub fn allocate(&mut self, requested_size: usize) -> Option<*mut u8> {
        // Align payload size to ALIGN boundary
        let aligned_payload = Self::align_up(requested_size);

        // Total block size = header + aligned payload + footer
        let total_size = aligned_payload + Self::block_overhead();

        // Find suitable region using the configured strategy
        let (region, prev) = match self.heap_type {
            HeapType::FirstFit => self.find_region_first_fit(total_size),
            HeapType::BestFit => self.find_region_best_fit(total_size),
            HeapType::WorstFit => self.find_region_worst_fit(total_size),
            HeapType::NextFit => self.find_region_next_fit(total_size),
        };

        let node_ptr = region?;

        unsafe {
            let node = &mut *node_ptr;

            // Check if we should split this region
            if node.size >= total_size + Self::block_overhead() {
                // Enough space to split
                let remaining_size = node.size - total_size;

                // Create new node for the remaining free space
                let new_node_ptr = (node_ptr as *mut u8).add(total_size) as *mut FreeListNode;

                new_node_ptr.write(FreeListNode::new(remaining_size, node.next));

                // Write footer to remaining block
                let new_footer =
                    (new_node_ptr as usize + remaining_size - size_of::<usize>()) as *mut usize;
                new_footer.write(remaining_size);

                // Remove old node and insert new one into free list
                if let Some(prev_ptr) = prev {
                    (*prev_ptr).next = Some(new_node_ptr);
                } else {
                    self.head = Some(new_node_ptr);
                }

                // Mark current node as allocated
                node.size = total_size;
            } else {
                // Not enough to split, use entire block
                if let Some(prev_ptr) = prev {
                    (*prev_ptr).next = node.next;
                } else {
                    self.head = node.next;
                }
            }

            // Write footer to allocated block
            let footer_ptr = (node_ptr as usize + node.size - size_of::<usize>()) as *mut usize;
            footer_ptr.write(node.size);

            // Return pointer after header (where user's data starts)
            Some((node_ptr as *mut u8).add(size_of::<FreeListNode>()))
        }
    }

    /*
     * Deallocates memory and returns it to the free list
     *
     * Parameters:
     * - address: User's pointer (returned by allocate)
     *
     * Algorithm (from OSTEP Chapter 17):
     * 1. Find the header by subtracting header size from address
     * 2. Reinsert block into free list in sorted (address) order
     * 3. Coalesce with next block if adjacent
     * 4. Coalesce with previous block if adjacent (using footer)
     *
     * Time complexity: O(n) due to finding insertion point in sorted list
     *
     * Coalescing (combining adjacent blocks):
     * This prevents external fragmentation. Without coalescing, many small
     * free regions would accumulate and become unusable. OSTEP Chapter 17
     * discusses the importance of coalescing for long-running systems.
     */
    pub fn deallocate(&mut self, address: usize) {
        unsafe {
            // Find the header by subtracting header size from user's pointer
            let node_ptr = (address - size_of::<FreeListNode>()) as *mut FreeListNode;

            let node = &mut *node_ptr;

            // Find insertion point in free list (must be in sorted order)
            let mut current = self.head;
            let mut prev: Option<*mut FreeListNode> = None;

            while let Some(curr_ptr) = current {
                if curr_ptr as usize > node_ptr as usize {
                    break;
                }
                prev = current;
                current = (*curr_ptr).next;
            }

            // Insert this block into the free list at correct position
            node.next = current;

            if let Some(prev_ptr) = prev {
                (*prev_ptr).next = Some(node_ptr);
            } else {
                self.head = Some(node_ptr);
            }

            // Forward coalescing: merge with next block if adjacent
            if let Some(next_ptr) = node.next {
                let node_end = node_ptr as usize + node.size;

                if node_end == next_ptr as usize {
                    // Blocks are adjacent, merge them
                    node.size += (*next_ptr).size;
                    node.next = (*next_ptr).next;

                    // Update footer of merged block
                    let footer = (node_ptr as usize + node.size - size_of::<usize>()) as *mut usize;
                    footer.write(node.size);
                }
            }

            // Backward coalescing: merge with previous block if adjacent
            // We need to walk backward through previous blocks' footers
            if node_ptr as usize > self.start_address {
                // Read the footer of the block before this one
                let prev_footer_ptr = (node_ptr as usize - size_of::<usize>()) as *mut usize;
                let prev_size = prev_footer_ptr.read();
                let prev_start = node_ptr as usize - prev_size;

                if prev_start >= self.start_address {
                    let prev_node_ptr = prev_start as *mut FreeListNode;
                    let prev_node = &mut *prev_node_ptr;

                    // Check if previous block's end aligns with current block's start
                    if prev_start + prev_node.size == node_ptr as usize {
                        // Blocks are adjacent, merge them
                        prev_node.size += node.size;
                        prev_node.next = node.next;

                        // Update footer of merged block
                        let footer =
                            (prev_start + prev_node.size - size_of::<usize>()) as *mut usize;
                        footer.write(prev_node.size);
                    }
                }
            }
        }
    }
}

// ============================================================================
// GLOBALALLOC IMPLEMENTATION
// ============================================================================

/*
 * GlobalAlloc Implementation for Locked<FreeList>
 *
 * NOTE: This is AI-generated boilerplate code
 *
 * By implementing GlobalAlloc, we register FreeList as the system allocator.
 * Rust's Box, Vec, and all dynamic allocation will use this allocator.
 *
 * The #[global_allocator] attribute in mod.rs marks ALLOCATOR as the system allocator.
 */

unsafe impl GlobalAlloc for Locked<FreeList> {
    /*
     * Allocates memory for a given layout
     *
     * Parameters:
     * - layout: Size and alignment requirements
     *
     * Returns: Raw pointer to allocated memory, or null on failure
     *
     * How it works:
     * 1. Lock the allocator (get mutable access)
     * 2. Call our FreeList::allocate with requested size
     * 3. Return pointer or null
     *
     * Note: Ignores layout.align() and just uses ALIGN constant
     * A more robust implementation would respect layout.align()
     */
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = self.lock();

        match allocator.allocate(layout.size()) {
            Some(ptr) => ptr,
            None => null_mut(),
        }
    }

    /*
     * Deallocates previously allocated memory
     *
     * Parameters:
     * - ptr: Pointer to deallocate (from alloc())
     * - _layout: Original layout (unused in our simple implementation)
     *
     * How it works:
     * 1. Lock the allocator
     * 2. Call FreeList::deallocate with the address
     * 3. Memory is returned to free list
     *
     * Safety requirement:
     * - ptr must have been returned by alloc()
     * - ptr must not be used after dealloc()
     * - These are enforced by Rust's type system (& and &mut references)
     */
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let allocator = self.lock();
        allocator.deallocate(ptr as usize);
    }
}
