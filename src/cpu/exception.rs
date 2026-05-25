use crate::println;
use core::arch::asm;

#[inline(always)]
fn read_esr() -> u64 {
    let mut esr: u64;
    unsafe { asm!("mrs {}, esr_el1", out(reg) esr, options(nomem, nostack, preserves_flags)) };
    esr
}

#[inline(always)]
fn read_far() -> u64 {
    let mut far: u64;
    unsafe { asm!("mrs {}, far_el1", out(reg) far, options(nomem, nostack, preserves_flags)) };
    far
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn handle_irq() {
    crate::drivers::interrupt::handle_irq();
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn handle_sync() {
    let esr = read_esr();
    let far = read_far();

    let exception_class = esr >> 26;

    println!("\n==================================================");
    println!(" SEGMENTATION FAULT ");
    println!("Process attempted an illegal memory access!");
    println!("==================================================");
    println!("Exception Class (EC): {:#04X}", exception_class);
    println!("Syndrome (ESR_EL1):   {:#018X}", esr);
    println!("Fault Addr (FAR_EL1): {:#018X}", far);
    println!("==================================================");
    println!("[KERNEL] Terminating malicious process...");

    crate::sys_exit();
}
