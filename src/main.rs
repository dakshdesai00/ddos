#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

pub mod cpu;
mod drivers;
mod hardwareselect;
mod memory;
mod utils;

use core::panic::PanicInfo;

use cpu::cpu_switch_to;
use cpu::process::CpuContext;

static mut CTX_MAIN: CpuContext = CpuContext {
    x19: 0,
    x20: 0,
    x21: 0,
    x22: 0,
    x23: 0,
    x24: 0,
    x25: 0,
    x26: 0,
    x27: 0,
    x28: 0,
    x29: 0,
    x30: 0,
    sp: 0,
};

static mut CTX_A: CpuContext = CpuContext {
    x19: 0,
    x20: 0,
    x21: 0,
    x22: 0,
    x23: 0,
    x24: 0,
    x25: 0,
    x26: 0,
    x27: 0,
    x28: 0,
    x29: 0,
    x30: 0,
    sp: 0,
};

static mut CTX_B: CpuContext = CpuContext {
    x19: 0,
    x20: 0,
    x21: 0,
    x22: 0,
    x23: 0,
    x24: 0,
    x25: 0,
    x26: 0,
    x27: 0,
    x28: 0,
    x29: 0,
    x30: 0,
    sp: 0,
};

extern "C" fn task_a() {
    println!("\n[TASK A] Task A started! The OS switched to me.");

    let mut task_a_vec: Vec<&str> = Vec::new();
    task_a_vec.push("Isolated Memory!");
    task_a_vec.push("No other task can touch this.");

    let stack_address = &task_a_vec as *const _ as u64;

    println!("[TASK A] I created a Vector!");
    println!("[TASK A] Vector contents: {:?}", task_a_vec);
    println!("[TASK A] Vector stack address: {:#X}", stack_address);

    println!("[TASK A] Yielding the CPU to Task B...");
    unsafe {
        cpu_switch_to(&raw mut CTX_A, &raw const CTX_B);
    }

    println!("\n[TASK A] I am back! Task B handed the CPU back to me.");
    println!("[TASK A] Checking if my stack memory survived...");

    println!("[TASK A] Vector contents are still: {:?}", task_a_vec);
    println!("[TASK A] My memory survived! Yielding back to Main OS...");

    unsafe {
        cpu_switch_to(&raw mut CTX_B, &raw const CTX_MAIN);
    }

    loop {}
}

extern "C" fn task_b() {
    println!("\n[TASK B] Task B Started! Task A handed me the CPU.");
    println!("[TASK B] My stack is completely separate from Task A.");
    println!("[TASK B] Yielding the CPU back to Task A...");
    unsafe {
        cpu_switch_to(&raw mut CTX_B, &raw const CTX_A);
    }
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    drivers::uart::UART.lock().init();

    println!("\n[KERNEL] Booting DDOS...");

    memory::init();

    drivers::interrupt::init();
    drivers::timer::init();

    println!("[KERNEL] Heap Initialized.");
    println!("Welcome to DDOS Kernel v0.1");
    println!("Testing Heap Allocation...");

    let heap_val = Box::new(42);
    println!("- Box allocated at {:p}, value: {}", heap_val, *heap_val);

    let mut vec = alloc::vec![];
    for i in 0..5 {
        vec.push(i);
    }
    println!("- Vec allocated: {:?} (Success!)", vec);

    println!("\n[KERNEL] Setting up things for Two Tasks Demo...");

    let stack_a = Box::leak(alloc::vec![0u8; 16 * 1024].into_boxed_slice());
    let stack_b = Box::leak(alloc::vec![0u8; 16 * 1024].into_boxed_slice());

    let sp_a = (stack_a.as_ptr() as u64 + stack_a.len() as u64) & !0xF;
    let sp_b = (stack_b.as_ptr() as u64 + stack_b.len() as u64) & !0xF;

    unsafe {
        CTX_A.sp = sp_a;
        CTX_A.x30 = task_a as *const () as u64;

        CTX_B.sp = sp_b;
        CTX_B.x30 = task_b as *const () as u64;

        println!("[KERNEL] Firing cpu_switch_to... Jumping to Task A");

        cpu_switch_to(&raw mut CTX_MAIN, &raw const CTX_A);
    }

    println!("\n[KERNEL] We are back in main. The OS is in full control again.");

    println!("[KERNEL] Dropping into busy loop. Waiting for Virtual Timer Heartbeat...");
    loop {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n!!! KERNEL PANIC !!!");
    println!("Details: {}", info);
    loop {}
}
