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

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    // 1. Initialize the global hardware UART ONCE at boot
    drivers::uart::UART.lock().init();

    println!("\n[KERNEL] Booting DDOS...");

    memory::init();

    println!("[KERNEL] Heap Initialized.");
    println!("Welcome to DDOS Kernel v0.1");
    println!("Testing Heap Allocation...");

    let heap_val = Box::new(42);

    println!("- Box allocated at {:p}, value: {}", heap_val, *heap_val);

    let mut vec = Vec::new();
    for i in 0..5 {
        vec.push(i);
    }

    println!("- Vec allocated: {:?} (Success!)", vec);
    println!("[KERNEL] UART console mode");
    print!("\n> ");

    loop {
        let byte = drivers::uart::UART.lock().read_byte();

        match byte {
            b'\r' | b'\n' => {
                print!("\r\n> ");
            }
            127 | 8 => {
                print!("\x08 \x08");
            }
            b' '..=b'~' => {
                print!("{}", byte as char);
            }
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Even the panic handler gets a massive clean up
    println!("\n!!! KERNEL PANIC !!!");
    println!("Details: {}", info);
    loop {}
}
