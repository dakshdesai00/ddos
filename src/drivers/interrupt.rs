use crate::hardwareselect;

pub(crate) fn init() {
    #[cfg(any(feature = "rpi4", feature = "rpi5"))]
    unsafe {
        let gicd_ctlr = hardwareselect::GICD_BASE as *mut u32;
        core::ptr::write_volatile(gicd_ctlr, 3);

        let gicd_igroupr0 = (hardwareselect::GICD_BASE + 0x080) as *mut u32;
        core::ptr::write_volatile(gicd_igroupr0, 0xFFFF_FFFF);

        let gicd_isenabler0 = (hardwareselect::GICD_BASE + 0x100) as *mut u32;
        core::ptr::write_volatile(gicd_isenabler0, 1 << 27);

        let gicc_pmr = (hardwareselect::GICC_BASE + 0x004) as *mut u32;
        core::ptr::write_volatile(gicc_pmr, 0xFF);

        let gicc_ctlr = hardwareselect::GICC_BASE as *mut u32;
        core::ptr::write_volatile(gicc_ctlr, 3);

        crate::println!("[INTERRUPT] GIC Initialized. Timer unmasked.");
    }

    #[cfg(any(feature = "qemu", feature = "rpi3"))]
    unsafe {
        let core0_timer_irq_ctrl = (hardwareselect::LOCAL_INTC_BASE + 0x40) as *mut u32;
        core::ptr::write_volatile(core0_timer_irq_ctrl, 0xF);

        crate::println!("[INTERRUPT] Legacy Controller Initialized. Timers unmasked.");
    }
}

pub(crate) fn handle_irq() {
    #[cfg(any(feature = "rpi4", feature = "rpi5"))]
    unsafe {
        let gicc_iar = (hardwareselect::GICC_BASE + 0x00C) as *mut u32;
        let irq = core::ptr::read_volatile(gicc_iar) & 0x3FF;

        if irq == 27 {
            crate::drivers::timer::handle_tick();
        }

        let gicc_eoir = (hardwareselect::GICC_BASE + 0x010) as *mut u32;
        core::ptr::write_volatile(gicc_eoir, irq);
    }

    #[cfg(any(feature = "qemu", feature = "rpi3"))]
    {
        crate::drivers::timer::handle_tick();
    }
}
