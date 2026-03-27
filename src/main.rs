#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;

pub(crate) mod cpu;
mod drivers;
mod hardwareselect;
mod memory;
pub(crate) mod scheduler;
mod utils;

use core::alloc::Layout;
use core::panic::PanicInfo;
use cpu::process::Process;

use crate::scheduler::mlfq::SCHEDULER;

pub fn sys_exit() -> ! {
    println!("[SYSCALL] Task requested termination. Moving to Graveyard...");
    SCHEDULER.lock().kill_current();
    loop {
        unsafe {
            core::arch::asm!("wfe");
        }
    }
}

extern "C" fn task_a() {
    for i in 1..=3 {
        println!("[TASK A] Sprinting... (Step {}/3)", i);
        for _ in 0..500_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
    println!("[TASK A] Finished! Exiting the system.");
    sys_exit();
}

extern "C" fn task_b() {
    let mut loops = 0;
    loop {
        loops += 1;
        if loops % 2 == 0 {
            println!("[TASK B] Still crunching numbers... (Infinite CPU Hog)");
        }
        for _ in 0..500_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
}

extern "C" fn task_c() {
    for i in 1..=6 {
        println!("[TASK C] Jogging... (Step {}/6)", i);
        for _ in 0..500_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
    println!("[TASK C] Finished! Exiting the system.");
    sys_exit();
}

#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    drivers::uart::UART.lock().init();
    println!("\n[KERNEL] Booting DDOS...");

    memory::init();
    println!("[KERNEL] Heap Initialized.");

    println!("\n[KERNEL] Initializing MLFQ Scheduler...");

    let task_main = Process::new(0, 16, 0, 0);
    let task_a = Process::new(1, 16, 0, task_a as usize);
    let task_b = Process::new(2, 16, 0, task_b as usize);
    let task_c = Process::new(3, 16, 0, task_c as usize);

    SCHEDULER.lock().add_task(task_main);
    SCHEDULER.lock().add_task(task_a);
    SCHEDULER.lock().add_task(task_b);
    SCHEDULER.lock().add_task(task_c);

    println!("[KERNEL] 4 Tasks loaded into Highest Priority Queue.");

    drivers::interrupt::init();
    drivers::timer::init();

    println!("[KERNEL] Dropping into idle loop. Waiting for the first timer interrupt...");

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

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
