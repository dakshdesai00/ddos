/*
 * config.rs - Memory Layout Configuration for DDOS
 *
 * WRITTEN BY: User (not AI-generated)
 *
 * This file defines the memory addresses and sizes used for the OS kernel.
 * It's based on the Raspberry Pi 4 memory map and bootloader requirements.
 *
 * Raspberry Pi 4 Memory Layout:
 * - 0x00000000 - 0x80000: Firmware/Bootloader (128 KB)
 * - 0x80000    onwards:   Kernel code and data
 *
 * Book Reference:
 * These values follow the setup described in OSTEP (Operating Systems: Three Easy Pieces)
 * Chapter 13: Address Spaces, Chapter 14: Memory Virtualization
 */

/*
 * KERNEL_START - Starting address of kernel code
 *
 * Value: 0x80000 (512 KB)
 *
 * Why this address?
 * - RPi bootloader loads ARM kernel at 0x80000 by convention
 * - We're running bare-metal (no OS above us), so we own this address
 * - The first 0x80000 bytes are reserved for firmware/bootloader
 */
pub const KERNEL_START: usize = 0x80000;

/*
 * KERNEL_STACK_START - Starting address for kernel stack
 *
 * Value: 0x80000 (same as KERNEL_START)
 *
 * Why here?
 * - Stack grows downward (from high to low addresses)
 * - Starting point is at kernel start; stack will grow downward from here
 * - Each thread/CPU will have its own stack space
 * - Current setup is single-core, so one stack is sufficient
 */
pub const KERNEL_STACK_START: usize = 0x80000;

/*
 * HEAP_START - Starting address for dynamic memory (heap)
 *
 * Value: KERNEL_STACK_START + 0x200000 = 0x80000 + 2 MB = 0x280000
 *
 * Why this offset?
 * - We allocate 2 MB (0x200000) for the kernel stack
 * - Heap starts immediately after stack to avoid collision
 * - This leaves plenty of space for stack growth before hitting the heap
 *
 * Memory Usage:
 * - 0x80000 - 0x280000: Kernel stack (2 MB)
 * - 0x280000 - 0x480000: Heap (2 MB)
 */
pub const HEAP_START: usize = KERNEL_STACK_START + 0x200000;

/*
 * HEAP_SIZE - Total size of heap memory pool
 *
 * Value: 0x200000 (2 MB)
 *
 * What it's used for:
 * - Dynamic memory allocation via malloc/Box/Vec
 * - Kernel data structures that don't have fixed size at compile time
 * - FreeList allocator manages this entire region
 *
 * Allocation Method: Free List (Chapter 17, OSTEP)
 * - Maintained by heap.rs::FreeList
 * - Supports multiple allocation strategies: FirstFit, BestFit, WorstFit, NextFit
 * - Implements coalescing to reduce fragmentation
 */
pub const HEAP_SIZE: usize = 0x200000;
