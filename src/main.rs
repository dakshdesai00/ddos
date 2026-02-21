#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

mod drivers;
mod hardwareselect;
mod memory;
mod utils;

use core::arch::global_asm;
global_asm!(include_str!("cpu/boot.s"));

use core::fmt::Write;
use core::panic::PanicInfo;
use drivers::{console, framebuffer, uart};

// Kernel entry point (called from boot.s)
#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    // --- Early serial output (debug channel) ---
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n[KERNEL] Booting DDOS...");

    // --- Initialize heap allocator ---
    memory::init();
    let _ = writeln!(uart, "[KERNEL] Heap Initialized.");

    // --- Initialize framebuffer (graphics output) ---
    match framebuffer::FrameBuffer::new() {
        Ok(fb) => {
            let _ = writeln!(uart, "[KERNEL] HDMI Initialized.");

            let mut console = console::Console::new(fb);

            // Visual boot message
            console.set_color(0xFF00FF00);
            let _ = writeln!(console, "Welcome to DDOS Kernel v0.1");
            console.set_color(0xFFFFFFFF);

            // ================= TESTING FEATURES =================
            let _ = writeln!(console, "Testing Heap Allocation...");

            // Test: single heap allocation (Box)
            let heap_val = Box::new(42);
            let _ = writeln!(
                console,
                "- Box allocated at {:p}, value: {}",
                heap_val, *heap_val
            );

            // Test: dynamic heap growth (Vec)
            let mut vec = Vec::new();
            for i in 0..5 {
                vec.push(i);
            }
            let _ = writeln!(console, "- Vec allocated: {:?} (Success!)", vec);
            // ====================================================

            // Simple UART echo shell
            let _ = write!(console, "\n> ");
            loop {
                let byte = uart.read_byte();
                match byte {
                    b'\r' => {
                        let _ = write!(console, "\n> ");
                    }
                    127 | 8 => {
                        console.backspace();
                    }
                    _ => {
                        let c = byte as char;
                        let _ = write!(console, "{}", c);
                    }
                }
            }
        }
        Err(e) => {
            let _ = writeln!(uart, "[PANIC] HDMI Init Failed: Error 0x{:X}", e);
            loop {}
        }
    }
}

// Panic handler -> prints panic info to serial and halts
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n!!! KERNEL PANIC !!!");
    let _ = writeln!(uart, "Details: {}", info);
    loop {}
}
