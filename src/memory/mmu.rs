use super::frame::PAGE_SIZE;
use super::layout;
use super::pagetable::{PageTable, PageTableEntry};
use crate::hardwareselect;

// We need a safe boundary for peripherals (16MB is plenty for UART, GPIO)
const PERIPHERAL_SIZE: usize = 0x0100_0000;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".data.page_tables")]
pub static mut KERNEL_L0_TABLE: PageTable = PageTable::new();

pub unsafe fn init() {
    unsafe {
        let l0_ptr = core::ptr::addr_of_mut!(KERNEL_L0_TABLE);

        // 1. Map the Kernel Code and entire physical RAM
        for addr in (layout::RAM_START..layout::RAM_END).step_by(PAGE_SIZE) {
            (*l0_ptr).map_page(
                addr,
                addr,
                PageTableEntry::VALID | PageTableEntry::TABLE_OR_PAGE | PageTableEntry::ACCESS_FLAG,
            );
        }

        // 2. Map Peripherals for QEMU or Raspberry Pi 4 (Single Bus)
        #[cfg(not(feature = "rpi5"))]
        {
            for addr in (layout::PERIPHERAL_BASE..layout::PERIPHERAL_BASE + PERIPHERAL_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
        }

        // 2b. Map Local Interrupt Controller for QEMU/RPi3
        #[cfg(any(feature = "qemu", feature = "rpi3"))]
        {
            const LOCAL_INTC_SIZE: usize = 0x0001_0000; // 64KB local peripheral region
            for addr in (hardwareselect::LOCAL_INTC_BASE
                ..hardwareselect::LOCAL_INTC_BASE + LOCAL_INTC_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
        }

        // 2c. Map GIC registers for RPi4/RPi5 (outside main peripheral window)
        #[cfg(feature = "rpi4")]
        {
            const GIC_REGION_SIZE: usize = 0x0001_0000; // 64KB
            for addr in (hardwareselect::GICD_BASE..hardwareselect::GICD_BASE + GIC_REGION_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
            for addr in (hardwareselect::GICC_BASE..hardwareselect::GICC_BASE + GIC_REGION_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
        }

        #[cfg(feature = "rpi5")]
        {
            const GIC_REGION_SIZE: usize = 0x0001_0000; // 64KB
            for addr in (hardwareselect::GICD_BASE..hardwareselect::GICD_BASE + GIC_REGION_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
            for addr in (hardwareselect::GICC_BASE..hardwareselect::GICC_BASE + GIC_REGION_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
        }

        // 3. Map Peripherals for Raspberry Pi 5 (Dual Bus Architecture)
        #[cfg(feature = "rpi5")]
        {
            // Map the Prefetchable Peripheral Bus
            for addr in (layout::PERIPHERAL_BASE_PREFETCH
                ..layout::PERIPHERAL_BASE_PREFETCH + PERIPHERAL_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }

            // Map the Non-Prefetchable Peripheral Bus
            for addr in (layout::PERIPHERAL_BASE_NON_PREFETCH
                ..layout::PERIPHERAL_BASE_NON_PREFETCH + PERIPHERAL_SIZE)
                .step_by(PAGE_SIZE)
            {
                (*l0_ptr).map_page(
                    addr,
                    addr,
                    PageTableEntry::VALID
                        | PageTableEntry::TABLE_OR_PAGE
                        | PageTableEntry::ATTR_DEVICE
                        | PageTableEntry::ACCESS_FLAG,
                );
            }
        }

        let ttbr0_value = core::ptr::addr_of!(KERNEL_L0_TABLE) as u64;

        // 4. Configure Hardware and Enable MMU
        core::arch::asm!(
            // Step A: Setup MAIR_EL1 (Define Memory Attributes)
            // Attr 0: 0xFF (Normal Memory, Cacheable)
            // Attr 1: 0x00 (Device Memory, nGnRnE - strictly no caching)
            "ldr x0, =0x00000000000000FF",
            "msr mair_el1, x0",

            // Step B: Setup TCR_EL1 (Translation Control)
            // Configures 4KB Granule, 48-bit address space, and caching policies.
            "ldr x0, =0x80803510",
            "msr tcr_el1, x0",

            // Step C: Load the Page Table Root
            "msr ttbr0_el1, {0}",

            // Step D: Force CPU to finish all previous instructions
            "isb",

            // Step E: Enable the MMU (Set M bit in SCTLR_EL1)
            "mrs x0, sctlr_el1",
            "orr x0, x0, #1",
            "msr sctlr_el1, x0",

            // Step F: Force CPU to fetch new instructions using the MMU
            "isb",

            in(reg) ttbr0_value,
            options(nostack, preserves_flags)
        );
    }
}
