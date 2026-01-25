/*
 * main.rs - DDOS Operating System Entry Point
 *
 * This is the main Rust entry point for the DDOS (Daksh's Demo Operating System).
 * It's called from the assembly boot code after initial hardware setup.
 *
 * The OS provides:
 * - UART serial console for debugging
 * - HDMI framebuffer graphics output
 * - Simple text console with colored text support
 * - Interactive input/output via UART
 *
 * Control flow:
 * 1. Initialize UART for serial debugging
 * 2. Initialize GPU framebuffer for HDMI display
 * 3. Create text console on framebuffer
 * 4. Display welcome message
 * 5. Enter infinite loop reading UART and echoing to screen
 */

// Disable Rust standard library (no OS support available)
#![no_std]
// Disable standard main function (we define our own entry point)
#![no_main]

// Include assembly boot code from cpu module
use core::arch::global_asm;
global_asm!(include_str!("cpu/boot.s"));

// Module declarations
mod drivers; // Hardware drivers (UART, framebuffer, mailbox, console)
mod utils; // Utilities (font data)

use drivers::console;
use drivers::framebuffer;
use drivers::uart;

use core::fmt::Write;
use core::panic::PanicInfo;

/*
 * Main function - Entry point called from boot.s
 *
 * Attributes:
 * - no_mangle: Prevents Rust from changing function name (boot.s calls "_main")
 * - extern "C": Uses C calling convention for compatibility with assembly
 * - Returns '!': Never returns (infinite loop)
 *
 * How it works:
 * 1. Initialize UART for serial debugging output
 * 2. Attempt to initialize framebuffer via GPU mailbox
 * 3. On success: Create console, display UI, enter input loop
 * 4. On failure: Print error to UART and halt
 */
#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    // Initialize UART serial port for debugging
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\n[KERNEL] Starting...");

    // Attempt to initialize framebuffer
    // Returns Ok(fb) with framebuffer on success, or Err(code) on failure
    match framebuffer::FrameBuffer::new() {
        Ok(fb) => {
            let _ = writeln!(uart, "[KERNEL] HDMI Initialized.");

            // Create text console on framebuffer
            let mut console = console::Console::new(fb);

            // Display welcome banner with colored text
            console.set_color(0xFF00FF00); // Green
            let _ = writeln!(console, "DDOS Kernel v0.1");
            console.set_color(0xFFFFFFFF); // White
            let _ = write!(console, "\n> "); // Command prompt

            // Main input loop: read UART and echo to console
            // This creates an interactive terminal experience
            loop {
                // Block until a byte arrives on UART (keyboard input)
                let byte = uart.read_byte();

                match byte {
                    b'\r' => {
                        // Carriage return (Enter key): start new line with prompt
                        let _ = write!(console, "\n> ");
                    }
                    127 | 8 => {
                        // Backspace (ASCII 127 = DEL, 8 = BS)
                        console.backspace();
                    }
                    _ => {
                        // Regular printable character: echo to screen
                        let c = byte as char;
                        let _ = write!(console, "{}", c);
                    }
                }
            }
        }
        Err(e) => {
            // Framebuffer initialization failed - print error and halt
            let _ = writeln!(uart, "[PANIC] HDMI Failed. Error: 0x{:X}", e);
            loop {} // Infinite loop (system halted)
        }
    }
}

/*
 * Panic Handler - Required for no_std Rust
 *
 * This function is called when Rust code panics (unrecoverable error).
 * Since we have no OS or standard library, we simply halt the system.
 *
 * In a more sophisticated OS, this might:
 * - Print panic message to UART/screen
 * - Save crash dump to memory
 * - Reboot the system
 */
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {} // Infinite loop (system halted on panic)
}
