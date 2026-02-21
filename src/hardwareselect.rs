/*
 * hardwareselect.rs - Hardware Abstraction & Selection Layer
 *
 * PURPOSE:
 * This module centralizes all hardware-specific constants and addresses.
 * Instead of hardcoding addresses throughout drivers, we define them here
 * and switch via Cargo features.
 *
 * Enables building the same kernel for multiple platforms:
 * - QEMU (RPi3 model)
 * - Real RPi3 hardware
 * - Real RPi4 hardware
 *
 * USAGE:
 * cargo build --features qemu    # Build for QEMU (RPi3 model)
 * cargo build --features rpi3    # Build for real RPi3
 * cargo build --features rpi4    # Build for real RPi4
 *
 * In drivers, use:
 * use crate::hardwareselect::UART0_BASE;
 * const UART_REG: *mut u32 = (UART0_BASE) as *mut u32;
 */

// ============================================================================
// HARDWARE SELECTION (Choose ONE via Cargo feature)
// ============================================================================

/*
 * Peripheral Base Address varies significantly between hardware:
 *
 * QEMU (emulating RPi3):   0x3F000000
 * Real RPi3:               0x3F000000
 * Real RPi4:               0xFE000000  ← Different! (BCM2711 vs BCM2835)
 *
 * All other peripherals are offsets from this base address.
 * By selecting one base, all derived addresses automatically work!
 */

#[cfg(feature = "qemu")]
pub const PERIPHERAL_BASE: usize = 0x3F000000;

#[cfg(feature = "rpi3")]
pub const PERIPHERAL_BASE: usize = 0x3F000000;

#[cfg(feature = "rpi4")]
pub const PERIPHERAL_BASE: usize = 0xFE000000;

// ============================================================================
// DERIVED PERIPHERAL ADDRESSES (Same offset for all platforms)
// ============================================================================

/*
 * All these addresses are computed as: PERIPHERAL_BASE + offset
 *
 * The offsets are the same across RPi3 and RPi4, only the base changes.
 * This is why we can use a single equation for all platforms!
 */

/// PL011 UART0 - Serial communication (debugging, console input)
/// Offset: 0x201000 from peripheral base
pub const UART0_BASE: usize = PERIPHERAL_BASE + 0x201000;

/// Mailbox Interface - Communication with GPU for framebuffer, memory, etc.
/// Offset: 0x00B880 from peripheral base
pub const MAILBOX_BASE: usize = PERIPHERAL_BASE + 0x00B880;

/// GPIO Controller - General Purpose Input/Output pins
/// Offset: 0x200000 from peripheral base
pub const GPIO_BASE: usize = PERIPHERAL_BASE + 0x200000;

/// System Timer - ARM Timer for scheduling and timeouts
/// Offset: 0x003000 from peripheral base
pub const TIMER_BASE: usize = PERIPHERAL_BASE + 0x003000;

/// Watchdog Timer - System reset on timeout
/// Offset: 0x100000 from peripheral base
pub const WATCHDOG_BASE: usize = PERIPHERAL_BASE + 0x100000;

// ============================================================================
// PLATFORM-SPECIFIC PROPERTIES
// ============================================================================

/*
 * Some constants are identical across platforms, but documented here for clarity
 */

/// UART Clock Speed (both RPi3 and RPi4)
pub const UART_CLOCK_HZ: u32 = 48_000_000;

/// System clock frequency
pub const SYSTEM_CLOCK_HZ: u32 = 1_000_000_000;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get a human-readable name for the current hardware platform
pub fn get_platform_name() -> &'static str {
    #[cfg(feature = "qemu")]
    return "QEMU (RPi3 Model)";

    #[cfg(feature = "rpi3")]
    return "Raspberry Pi 3";

    #[cfg(feature = "rpi4")]
    return "Raspberry Pi 4";

    #[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4")))]
    return "Unknown Platform";
}

/// Get the peripheral base address as a human-readable hex string
pub fn get_peripheral_base_display() -> &'static str {
    #[cfg(feature = "qemu")]
    return "0x3F000000";

    #[cfg(feature = "rpi3")]
    return "0x3F000000";

    #[cfg(feature = "rpi4")]
    return "0xFE000000";

    #[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4")))]
    return "Unknown";
}

// ============================================================================
// VALIDATION & COMPILE-TIME CHECKS
// ============================================================================

/// Compile-time assertion that exactly one feature is enabled
/// (This helps catch configuration errors early)
#[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4")))]
compile_error!(
    "Must enable exactly one hardware feature: qemu, rpi3, or rpi4\n\
     Example: cargo build --features qemu"
);

#[cfg(all(feature = "qemu", feature = "rpi3"))]
compile_error!("Cannot enable both 'qemu' and 'rpi3' features simultaneously");

#[cfg(all(feature = "qemu", feature = "rpi4"))]
compile_error!("Cannot enable both 'qemu' and 'rpi4' features simultaneously");

#[cfg(all(feature = "rpi3", feature = "rpi4"))]
compile_error!("Cannot enable both 'rpi3' and 'rpi4' features simultaneously");
