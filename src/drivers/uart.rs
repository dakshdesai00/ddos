use super::super::utils::locked::SpinLock;
use crate::hardwareselect::{UART_CLOCK_HZ, UART0_BASE};
use core::fmt;
use core::ptr::{read_volatile, write_volatile};

const PL011_BASE: usize = UART0_BASE;
const DR: *mut u32 = (PL011_BASE + 0x00) as *mut u32;
const FR: *mut u32 = (PL011_BASE + 0x18) as *mut u32;
const IBRD: *mut u32 = (PL011_BASE + 0x24) as *mut u32;
const FBRD: *mut u32 = (PL011_BASE + 0x28) as *mut u32;
const LCRH: *mut u32 = (PL011_BASE + 0x2C) as *mut u32;
const CR: *mut u32 = (PL011_BASE + 0x30) as *mut u32;
const IMSC: *mut u32 = (PL011_BASE + 0x38) as *mut u32;
const ICR: *mut u32 = (PL011_BASE + 0x44) as *mut u32;

pub struct Uart;

pub static UART: SpinLock<Uart> = SpinLock::new(Uart::new());

impl Uart {
    const BAUD_RATE: u32 = 115_200;

    pub const fn new() -> Uart {
        Uart
    }

    pub fn init(&self) {
        #[cfg(feature = "rpi5")]
        {
            return;
        }

        #[cfg(not(feature = "rpi5"))]
        {
            let baud_divisor_times_64 =
                (UART_CLOCK_HZ * 4 + (Self::BAUD_RATE / 2)) / Self::BAUD_RATE;
            let integer_divisor = baud_divisor_times_64 / 64;
            let fractional_divisor = baud_divisor_times_64 % 64;

            unsafe {
                write_volatile(CR, 0);
                write_volatile(IMSC, 0);
                write_volatile(ICR, 0x7FF);
                write_volatile(IBRD, integer_divisor);
                write_volatile(FBRD, fractional_divisor);
                write_volatile(LCRH, (1 << 4) | (3 << 5));
                write_volatile(CR, (1 << 0) | (1 << 8) | (1 << 9));
            }
        }
    }

    pub fn send(&self, c: char) {
        unsafe {
            while (read_volatile(FR) & (1 << 5)) != 0 {}
            write_volatile(DR, c as u32);
        }
    }

    pub fn read_byte(&self) -> u8 {
        unsafe {
            while (read_volatile(FR) & (1 << 4)) != 0 {}
            (read_volatile(DR) & 0xFF) as u8
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                self.send('\r');
            }
            self.send(c);
        }
        Ok(())
    }
}

// MACROS

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let mut uart = crate::drivers::uart::UART.lock();
    let _ = uart.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::drivers::uart::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}
