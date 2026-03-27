/*
 * hardwareselect.rs - Hardware Abstraction and Configuration
 */

// ============================================================================
// 1. PERIPHERAL BASE ADDRESSES
// ============================================================================

#[cfg(any(feature = "qemu", feature = "rpi3"))]
pub(crate) const PERIPHERAL_BASE: usize = 0x3F00_0000;

#[cfg(feature = "rpi4")]
pub(crate) const PERIPHERAL_BASE: usize = 0xFE00_0000;

#[cfg(feature = "rpi5")]
pub(crate) const PERIPHERAL_BASE: usize = 0x1C_0000_0000;

// ============================================================================
// 2. PERIPHERAL OFFSETS (UART, GPIO, TIMERS)
// ============================================================================

#[cfg(feature = "rpi5")]
pub(crate) const UART0_BASE: usize = PERIPHERAL_BASE + 0x30000;
#[cfg(all(
    not(feature = "rpi5"),
    any(feature = "qemu", feature = "rpi3", feature = "rpi4")
))]
pub(crate) const UART0_BASE: usize = PERIPHERAL_BASE + 0x201000;

#[cfg(feature = "rpi5")]
pub(crate) const GPIO_BASE: usize = PERIPHERAL_BASE + 0xD0000;
#[cfg(all(
    not(feature = "rpi5"),
    any(feature = "qemu", feature = "rpi3", feature = "rpi4")
))]
pub(crate) const GPIO_BASE: usize = PERIPHERAL_BASE + 0x200000;

#[cfg(feature = "rpi5")]
pub(crate) const TIMER_BASE: usize = 0;
#[cfg(all(
    not(feature = "rpi5"),
    any(feature = "qemu", feature = "rpi3", feature = "rpi4")
))]
pub(crate) const TIMER_BASE: usize = PERIPHERAL_BASE + 0x003000;

#[cfg(feature = "rpi5")]
pub(crate) const WATCHDOG_BASE: usize = 0;
#[cfg(all(
    not(feature = "rpi5"),
    any(feature = "qemu", feature = "rpi3", feature = "rpi4")
))]
pub(crate) const WATCHDOG_BASE: usize = PERIPHERAL_BASE + 0x100000;

// ============================================================================
// 3. INTERRUPT CONTROLLER BASE ADDRESSES
// ============================================================================

#[cfg(feature = "rpi5")]
pub(crate) const GICD_BASE: usize = 0x10_7FFF_9000;
#[cfg(feature = "rpi5")]
pub(crate) const GICC_BASE: usize = 0x10_7FFF_A000;

#[cfg(feature = "rpi4")]
pub(crate) const GICD_BASE: usize = 0xFF84_1000;
#[cfg(feature = "rpi4")]
pub(crate) const GICC_BASE: usize = 0xFF84_2000;

#[cfg(any(feature = "qemu", feature = "rpi3"))]
pub(crate) const LOCAL_INTC_BASE: usize = 0x4000_0000;

// ============================================================================
// 4. CLOCK SPEEDS
// ============================================================================

// Simplified: 48MHz applies to QEMU, RPi3, RPi4, and RPi5's RP1 UART
pub(crate) const UART_CLOCK_HZ: u32 = 48_000_000;
pub(crate) const SYSTEM_CLOCK_HZ: u32 = 1_000_000_000;

// ============================================================================
// 5. HELPER FUNCTIONS FOR LOGGING
// ============================================================================

pub(crate) fn get_platform_name() -> &'static str {
    #[cfg(feature = "qemu")]
    return "QEMU (RPi3 Model)";
    #[cfg(feature = "rpi3")]
    return "Raspberry Pi 3";
    #[cfg(feature = "rpi4")]
    return "Raspberry Pi 4";
    #[cfg(feature = "rpi5")]
    return "Raspberry Pi 5";
    #[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4", feature = "rpi5")))]
    return "Unknown Platform";
}

pub(crate) fn get_peripheral_base_display() -> &'static str {
    #[cfg(any(feature = "qemu", feature = "rpi3"))]
    return "0x3F00_0000";
    #[cfg(feature = "rpi4")]
    return "0xFE00_0000";
    #[cfg(feature = "rpi5")]
    return "0x1C_0000_0000";
    #[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4", feature = "rpi5")))]
    return "Unknown";
}

// ============================================================================
// 6. FEATURE VALIDATION
// ============================================================================

#[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4", feature = "rpi5")))]
compile_error!(
    "Must enable exactly one hardware feature: qemu, rpi3, rpi4, or rpi5\n\
     Example: cargo build --features qemu"
);

#[cfg(any(
    all(feature = "qemu", feature = "rpi3"),
    all(feature = "qemu", feature = "rpi4"),
    all(feature = "qemu", feature = "rpi5"),
    all(feature = "rpi3", feature = "rpi4"),
    all(feature = "rpi3", feature = "rpi5"),
    all(feature = "rpi4", feature = "rpi5")
))]
compile_error!("Cannot enable multiple hardware features simultaneously. Pick one!");
