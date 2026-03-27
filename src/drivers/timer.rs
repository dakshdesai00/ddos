use crate::println;
use core::arch::asm;
pub(crate) fn init() {
    reset();
    println!("[TIMER] ARM64 Virtual Timer initialized.");
}

fn reset() {
    unsafe {
        let mut frequency: u64;
        asm!("mrs {}, cntfrq_el0", out(reg) frequency);

        let ticks = frequency / 10;

        asm!("msr cntv_tval_el0, {}", in(reg) ticks);

        let ctl: u64 = 1;
        asm!("msr cntv_ctl_el0, {}", in(reg) ctl);
    }
}

pub(crate) fn handle_tick() {
    reset();
    crate::scheduler::mlfq::handle_timer_tick();
}
