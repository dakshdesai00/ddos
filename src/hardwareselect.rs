/*
 * hardwareselect.rs - Hardware Abstraction and Configuration
 *
 * This module uses conditional compilation (cfg features) to set memory
 * addresses, clock speeds, and platform names at compile time.
 *
 * Build examples:
 * cargo build --features qemu
 * cargo build --features rpi3
 * cargo build --features rpi4
 * cargo build --features rpi5
 */

// ============================================================================
// 1. PERIPHERAL BASE ADDRESSES
// ============================================================================

#[cfg(any(feature = "qemu", feature = "rpi3"))]
pub const PERIPHERAL_BASE: usize = 0x3F00_0000;

#[cfg(feature = "rpi4")]
pub const PERIPHERAL_BASE: usize = 0xFE00_0000;

// RPi5 firmware maps the RP1 Southbridge peripherals here via PCIe
#[cfg(feature = "rpi5")]
pub const PERIPHERAL_BASE: usize = 0x1C_0000_0000;

// ============================================================================
// 2. PERIPHERAL OFFSETS
// ============================================================================

// --- UART0 BASE ---
#[cfg(feature = "rpi5")]
pub const UART0_BASE: usize = PERIPHERAL_BASE + 0x30000; // RP1 UART0 offset

#[cfg(not(feature = "rpi5"))]
pub const UART0_BASE: usize = PERIPHERAL_BASE + 0x201000; // Legacy Broadcom offset

// --- GPIO BASE ---
#[cfg(feature = "rpi5")]
pub const GPIO_BASE: usize = PERIPHERAL_BASE + 0xD0000; // RP1 SYS_RIO (GPIO) offset

#[cfg(not(feature = "rpi5"))]
pub const GPIO_BASE: usize = PERIPHERAL_BASE + 0x200000; // Legacy Broadcom offset

// --- TIMER BASE ---
// Note: RPi5 handles timers differently. These are legacy Broadcom System Timer offsets.
#[cfg(feature = "rpi5")]
pub const TIMER_BASE: usize = 0; // Placeholder to prevent compilation errors, do not use on RPi5

#[cfg(not(feature = "rpi5"))]
pub const TIMER_BASE: usize = PERIPHERAL_BASE + 0x003000;

// --- WATCHDOG BASE ---
#[cfg(feature = "rpi5")]
pub const WATCHDOG_BASE: usize = 0; // Placeholder to prevent compilation errors, do not use on RPi5

#[cfg(not(feature = "rpi5"))]
pub const WATCHDOG_BASE: usize = PERIPHERAL_BASE + 0x100000;

// ============================================================================
// 3. CLOCK SPEEDS
// ============================================================================

#[cfg(any(feature = "qemu", feature = "rpi3", feature = "rpi4"))]
pub const UART_CLOCK_HZ: u32 = 48_000_000;

// RPi5 RP1 PL011 UART reference clock used for divisor programming
#[cfg(feature = "rpi5")]
pub const UART_CLOCK_HZ: u32 = 48_000_000;

pub const SYSTEM_CLOCK_HZ: u32 = 1_000_000_000;

// ============================================================================
// 4. HELPER FUNCTIONS FOR LOGGING
// ============================================================================

pub fn get_platform_name() -> &'static str {
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

pub fn get_peripheral_base_display() -> &'static str {
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
// 5. FEATURE VALIDATION
// ============================================================================

#[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4", feature = "rpi5")))]
compile_error!(
    "Must enable exactly one hardware feature: qemu, rpi3, rpi4, or rpi5\n\
     Example: cargo build --features qemu"
);

// This compact block ensures the compiler panics if you accidentally pass multiple flags
#[cfg(any(
    all(feature = "qemu", feature = "rpi3"),
    all(feature = "qemu", feature = "rpi4"),
    all(feature = "qemu", feature = "rpi5"),
    all(feature = "rpi3", feature = "rpi4"),
    all(feature = "rpi3", feature = "rpi5"),
    all(feature = "rpi4", feature = "rpi5")
))]
compile_error!("Cannot enable multiple hardware features simultaneously. Pick one!");
