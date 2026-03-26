pub mod exception;
pub mod process;

use core::arch::global_asm;

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("vectors.s"));
global_asm!(include_str!("switch.s"));

unsafe extern "C" {
    pub fn cpu_switch_to(prev: *mut process::CpuContext, next: *const process::CpuContext);
}
