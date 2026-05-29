// ============================================================================
// QEMU (raspi3b)
// ============================================================================
#[cfg(feature = "qemu")]
pub const RAM_START: usize = 0x0000_0000;

#[cfg(feature = "qemu")]
pub const RAM_END: usize = 0x3F00_0000; // Safe boundary before peripherals/GPU

#[cfg(feature = "qemu")]
pub const PERIPHERAL_BASE: usize = 0x3F00_0000;

// ============================================================================
// RASPBERRY PI 4
// ============================================================================
#[cfg(feature = "rpi4")]
pub const RAM_START: usize = 0x0000_0000;

#[cfg(feature = "rpi4")]
pub const RAM_END: usize = 0x3F00_0000; // Safe boundary before GPU split

#[cfg(feature = "rpi4")]
pub const PERIPHERAL_BASE: usize = 0xFE00_0000;

// ============================================================================
// RASPBERRY PI 5
// ============================================================================
#[cfg(feature = "rpi5")]
pub const RAM_START: usize = 0x0000_0000;

#[cfg(feature = "rpi5")]
pub const RAM_END: usize = 0x3F00_0000; // Safe boundary before GPU split

#[cfg(feature = "rpi5")]
pub const PERIPHERAL_BASE_PREFETCH: usize = 0x1C_0000_0000;

#[cfg(feature = "rpi5")]
pub const PERIPHERAL_BASE_NON_PREFETCH: usize = 0x1F_0000_0000;
