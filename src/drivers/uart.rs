/*
 * uart.rs - PL011 UART Driver for Raspberry Pi Serial Communication
 *
 * This module provides a driver for the ARM PL011 UART (Universal Asynchronous
 * Receiver/Transmitter) peripheral, which enables serial communication over
 * GPIO pins 14 (TX) and 15 (RX).
 *
 * The UART is used for:
 * - Debugging output (printf-style logging)
 * - Interactive console input
 * - Communication with host computer via serial cable/USB-to-serial
 *
 * Configuration:
 * - Baud rate: 115200 (standard for Raspberry Pi)
 * - Data bits: 8
 * - Stop bits: 1
 * - Parity: None
 * - Flow control: None
 */

use core::fmt;
use core::ptr::{read_volatile, write_volatile};

// PL011 UART register addresses (BCM2711/RPi4 peripheral base at 0xFE000000)
const PL011_BASE: usize = 0xFE201000; // UART0 base address
const DR: *mut u32 = (PL011_BASE + 0x00) as *mut u32; // Data Register (read/write data)
const FR: *mut u32 = (PL011_BASE + 0x18) as *mut u32; // Flag Register (status flags)
const IBRD: *mut u32 = (PL011_BASE + 0x24) as *mut u32; // Integer Baud Rate Divisor
const FBRD: *mut u32 = (PL011_BASE + 0x28) as *mut u32; // Fractional Baud Rate Divisor
const LCRH: *mut u32 = (PL011_BASE + 0x2C) as *mut u32; // Line Control Register
const CR: *mut u32 = (PL011_BASE + 0x30) as *mut u32; // Control Register (enable/disable)
const IMSC: *mut u32 = (PL011_BASE + 0x38) as *mut u32; // Interrupt Mask Set/Clear
const ICR: *mut u32 = (PL011_BASE + 0x44) as *mut u32; // Interrupt Clear Register

/*
 * UART controller (stateless)
 */
pub struct Uart;

impl Uart {
    /*
     * Creates and initializes a new UART instance
     *
     * Returns: Uart instance ready for communication
     *
     * How it works:
     * Creates a UART object and calls init() to configure hardware registers
     */
    pub fn new() -> Uart {
        let uart = Uart;
        uart.init(); // Configure UART hardware
        uart
    }

    /*
     * Initializes UART hardware registers
     *
     * How it works:
     * 1. Disable UART (write 0 to Control Register)
     * 2. Disable all interrupts (we'll use polling, not interrupts)
     * 3. Clear any pending interrupts
     * 4. Set baud rate to 115200 via divisor registers
     * 5. Configure line parameters (8 data bits, no parity)
     * 6. Enable UART, transmitter, and receiver
     *
     * Baud rate calculation:
     * UART clock = 48 MHz (default on RPi4)
     * Divisor = UART_CLK / (16 * baud_rate) = 48000000 / (16 * 115200) = 26.0416
     * IBRD = integer part = 26
     * FBRD = fractional part * 64 = 0.0416 * 64 ≈ 3
     */
    fn init(&self) {
        unsafe {
            // Disable UART during configuration
            write_volatile(CR, 0);

            // Disable all interrupts (bit mask 0 = all disabled)
            write_volatile(IMSC, 0);

            // Clear all interrupts (write 1 to clear, 0x7FF = all 11 interrupt bits)
            write_volatile(ICR, 0x7FF);

            // Set baud rate to 115200
            write_volatile(IBRD, 26); // Integer divisor
            write_volatile(FBRD, 3); // Fractional divisor

            // Configure line control:
            // Bit 4: Enable FIFOs
            // Bits 5-6 (WLEN): 8 data bits (value 3 = 8 bits)
            write_volatile(LCRH, (1 << 4) | (3 << 5));

            // Enable UART:
            // Bit 0 (UARTEN): Enable UART
            // Bit 8 (TXE): Enable transmitter
            // Bit 9 (RXE): Enable receiver
            write_volatile(CR, (1 << 0) | (1 << 8) | (1 << 9));
        }
    }

    /*
     * Sends a single character via UART
     *
     * Parameters:
     * - c: Character to transmit
     *
     * How it works:
     * 1. Poll Flag Register bit 5 (TXFF - Transmit FIFO Full)
     * 2. Wait until FIFO has space (TXFF = 0)
     * 3. Write character to Data Register, triggering transmission
     *
     * This is a blocking operation - waits until character can be sent
     */
    pub fn send(&self, c: char) {
        unsafe {
            // Wait until transmit FIFO is not full
            // Bit 5 of FR (Flag Register) = TXFF (Transmit FIFO Full)
            // Spin until bit 5 = 0 (FIFO has space)
            while (read_volatile(FR) & (1 << 5)) != 0 {}

            // Write character to Data Register (triggers transmission)
            write_volatile(DR, c as u32);
        }
    }

    /*
     * Receives a single byte from UART
     *
     * Returns: Received byte (0-255)
     *
     * How it works:
     * 1. Poll Flag Register bit 4 (RXFE - Receive FIFO Empty)
     * 2. Wait until data is available (RXFE = 0)
     * 3. Read byte from Data Register
     * 4. Mask to 8 bits (DR is 32-bit, but data is in lower 8 bits)
     *
     * This is a blocking operation - waits until a byte arrives
     */
    pub fn read_byte(&self) -> u8 {
        unsafe {
            // Wait until receive FIFO has data
            // Bit 4 of FR (Flag Register) = RXFE (Receive FIFO Empty)
            // Spin until bit 4 = 0 (FIFO has data)
            while (read_volatile(FR) & (1 << 4)) != 0 {}

            // Read byte from Data Register and mask to 8 bits
            (read_volatile(DR) & 0xFF) as u8
        }
    }
}

/*
 * Implement Rust's fmt::Write trait for UART
 *
 * This allows UART to be used with Rust's formatting macros like
 * write!(), writeln!(), and format_args!(). By implementing write_str(),
 * we enable statements like: writeln!(uart, "Value: {}", x)
 */
impl fmt::Write for Uart {
    /*
     * Writes a string to UART
     *
     * Parameters:
     * - s: String slice to transmit
     *
     * Returns: fmt::Result (always Ok for UART)
     *
     * How it works:
     * Iterates through each character in the string and calls send()
     * for each one. This handles all formatting automatically.
     */
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.send(c);
        }
        Ok(())
    }
}
