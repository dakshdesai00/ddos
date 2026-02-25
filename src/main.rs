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
use drivers::uart;

#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n[KERNEL] Booting DDOS...");

    memory::init();
    let _ = writeln!(uart, "[KERNEL] Heap Initialized.");
    let _ = writeln!(uart, "Welcome to DDOS Kernel v0.1");
    let _ = writeln!(uart, "Testing Heap Allocation...");

    let heap_val = Box::new(42);
    let _ = writeln!(
        uart,
        "- Box allocated at {:p}, value: {}",
        heap_val, *heap_val
    );

    let mut vec = Vec::new();
    for i in 0..5 {
        vec.push(i);
    }
    let _ = writeln!(uart, "- Vec allocated: {:?} (Success!)", vec);

    let _ = write!(uart, "\n> ");
    loop {
        let byte = uart.read_byte();
        match byte {
            b'\r' | b'\n' => {
                let _ = write!(uart, "\r\n> ");
            }
            127 | 8 => {
                let _ = write!(uart, "\x08 \x08");
            }
            _ => {
                let _ = write!(uart, "{}", byte as char);
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n!!! KERNEL PANIC !!!");
    let _ = writeln!(uart, "Details: {}", info);
    loop {}
}
