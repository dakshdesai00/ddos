use crate::println;
use core::arch::asm;

pub fn init() {
    reset();
    println!("[TIMER] ARM64 Virtual Timer initialized.");
}

pub fn reset() {
    unsafe {
        let mut frequency: u64;
        asm!("mrs {}, cntfrq_el0", out(reg) frequency);

        let ticks = frequency;

        asm!("msr cntv_tval_el0, {}", in(reg) ticks);

        let ctl: u64 = 1;
        asm!("msr cntv_ctl_el0, {}", in(reg) ctl);
    }
}

pub fn handle_tick() {
    println!("\n[KERNEL] TICK! Hardware Timer Fired!");
    reset();
}
