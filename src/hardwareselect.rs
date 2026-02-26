#[cfg(feature = "qemu")]
pub const PERIPHERAL_BASE: usize = 0x3F000000;

#[cfg(feature = "rpi3")]
pub const PERIPHERAL_BASE: usize = 0x3F000000;

#[cfg(feature = "rpi4")]
pub const PERIPHERAL_BASE: usize = 0xFE000000;

#[cfg(feature = "rpi5")]
pub const PERIPHERAL_BASE: usize = 0x1F000000;

pub const UART0_BASE: usize = PERIPHERAL_BASE + 0x201000;

pub const GPIO_BASE: usize = PERIPHERAL_BASE + 0x200000;

pub const TIMER_BASE: usize = PERIPHERAL_BASE + 0x003000;

pub const WATCHDOG_BASE: usize = PERIPHERAL_BASE + 0x100000;

#[cfg(any(feature = "qemu", feature = "rpi3", feature = "rpi4"))]
pub const UART_CLOCK_HZ: u32 = 48_000_000;

#[cfg(feature = "rpi5")]
pub const UART_CLOCK_HZ: u32 = 24_000_000;

pub const SYSTEM_CLOCK_HZ: u32 = 1_000_000_000;

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
    #[cfg(feature = "qemu")]
    return "0x3F000000";

    #[cfg(feature = "rpi3")]
    return "0x3F000000";

    #[cfg(feature = "rpi4")]
    return "0xFE000000";

    #[cfg(feature = "rpi5")]
    return "0x1F000000";

    #[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4", feature = "rpi5")))]
    return "Unknown";
}

#[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4", feature = "rpi5")))]
compile_error!(
    "Must enable exactly one hardware feature: qemu, rpi3, rpi4, or rpi5\n\
     Example: cargo build --features qemu"
);

#[cfg(all(feature = "qemu", feature = "rpi3"))]
compile_error!("Cannot enable both 'qemu' and 'rpi3' features simultaneously");

#[cfg(all(feature = "qemu", feature = "rpi4"))]
compile_error!("Cannot enable both 'qemu' and 'rpi4' features simultaneously");

#[cfg(all(feature = "qemu", feature = "rpi5"))]
compile_error!("Cannot enable both 'qemu' and 'rpi5' features simultaneously");

#[cfg(all(feature = "rpi3", feature = "rpi4"))]
compile_error!("Cannot enable both 'rpi3' and 'rpi4' features simultaneously");

#[cfg(all(feature = "rpi3", feature = "rpi5"))]
compile_error!("Cannot enable both 'rpi3' and 'rpi5' features simultaneously");

#[cfg(all(feature = "rpi4", feature = "rpi5"))]
compile_error!("Cannot enable both 'rpi4' and 'rpi5' features simultaneously");
