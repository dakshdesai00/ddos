// ============================================================================
// MAIN.RS - KERNEL ENTRY POINT
// ============================================================================

// 1. Enable Allocator Error Handling (Required for our Heap)
#![feature(alloc_error_handler)]
// 2. Standard No-OS Setup
#![no_std] // Disable standard library (no OS support)
#![no_main] // Disable standard main() entry point

// --- 3. MEMORY MANAGEMENT IMPORTS ---
// We need this to use 'Box', 'Vec', and 'String'.
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

// --- 4. MODULES ---
mod drivers; // Hardware drivers (UART, Framebuffer, Console)
mod memory; // Memory management (Heap Allocator)
mod utils; // Utilities (Locked wrapper, Font)

// --- 5. ASSEMBLY BOOTLOADER ---
use core::arch::global_asm;
global_asm!(include_str!("cpu/boot.s"));

// --- 6. IMPORTS ---
use core::fmt::Write; // Allows usage of writeln! macro
use core::panic::PanicInfo; // Used for the panic handler
use drivers::{console, framebuffer, uart}; // Import drivers

// ============================================================================
// KERNEL MAIN FUNCTION
// ============================================================================
#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    // A. Init UART (Serial) First
    // We do this first so if anything else fails, we can print the error.
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n[KERNEL] Booting DDOS...");

    // B. Init Memory (The Heap)
    // CRITICAL: This calls the 'init' function in 'src/memory/mod.rs'
    // This MUST be done before using Box or Vec!
    memory::init();
    let _ = writeln!(uart, "[KERNEL] Heap Initialized.");

    // C. Init Framebuffer (HDMI)
    match framebuffer::FrameBuffer::new() {
        Ok(fb) => {
            let _ = writeln!(uart, "[KERNEL] HDMI Initialized.");

            // Wrap the framebuffer in our Console driver
            let mut console = console::Console::new(fb);

            // D. Visual Test
            console.set_color(0xFF00FF00); // Green
            let _ = writeln!(console, "Welcome to DDOS Kernel v0.1");
            console.set_color(0xFFFFFFFF); // White

            // E. HEAP ALLOCATION TEST
            let _ = writeln!(console, "Testing Heap Allocation...");

            // Test 1: Box (Single allocation)
            // If the heap works, this allocates memory and stores 42.
            let heap_val = Box::new(42);
            let _ = writeln!(
                console,
                "- Box allocated at {:p}, value: {}",
                heap_val, *heap_val
            );

            // Test 2: Vec (Dynamic resizing)
            // This tests finding new blocks and expanding.
            let mut vec = Vec::new();
            for i in 0..5 {
                vec.push(i);
            }
            let _ = writeln!(console, "- Vec allocated: {:?} (Success!)", vec);

            // F. The Infinite Loop (Shell)
            let _ = write!(console, "\n> ");
            loop {
                let byte = uart.read_byte();

                match byte {
                    b'\r' => {
                        // Enter Key
                        let _ = write!(console, "\n> ");
                    }
                    127 | 8 => {
                        // Backspace
                        console.backspace();
                    }
                    _ => {
                        // Regular Character
                        let c = byte as char;
                        let _ = write!(console, "{}", c);
                    }
                }
            }
        }
        Err(e) => {
            // If HDMI fails, print error to Serial and halt.
            let _ = writeln!(uart, "[PANIC] HDMI Init Failed: Error 0x{:X}", e);
            loop {}
        }
    }
}

// ============================================================================
// PANIC HANDLER
// ============================================================================
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n!!! KERNEL PANIC !!!");
    let _ = writeln!(uart, "Details: {}", info);
    loop {}
}
