/*
 * mailbox.rs - Raspberry Pi Mailbox Interface for GPU Communication
 *
 * The mailbox is a hardware communication mechanism between the ARM CPU and
 * the VideoCore GPU on Raspberry Pi. It allows the CPU to send property tag
 * messages to request GPU services like framebuffer allocation, clock settings, etc.
 *
 * How it works:
 * 1. CPU writes a message address + channel number to the WRITE register
 * 2. GPU processes the request and writes response data into the message buffer
 * 3. GPU writes the same value to the READ register to signal completion
 * 4. CPU reads from READ register to receive the response
 *
 * The mailbox has status flags (FULL/EMPTY) to prevent race conditions.
 */

use core::ptr::{read_volatile, write_volatile};
use crate::hardwareselect::MAILBOX_BASE;

// Mailbox hardware register addresses (derived from hardware-selected peripheral base)
const MBOX_READ: *mut u32 = (MAILBOX_BASE + 0x00) as *mut u32; // Read register (offset 0x00)
const MBOX_STATUS: *mut u32 = (MAILBOX_BASE + 0x18) as *mut u32; // Status register (offset 0x18)
const MBOX_WRITE: *mut u32 = (MAILBOX_BASE + 0x20) as *mut u32; // Write register (offset 0x20)

// Mailbox status register bit flags
const MBOX_FULL: u32 = 0x80000000; // Bit 31: Mailbox write queue is full
const MBOX_EMPTY: u32 = 0x40000000; // Bit 30: Mailbox read queue is empty

/*
 * Mailbox Message Structure
 *
 * repr(C): Use C memory layout (no Rust reordering)
 * align(16): GPU requires 16-byte alignment for mailbox messages
 *
 * The data array holds property tags structured as:
 * [0]: Total message size in bytes
 * [1]: Request code (0) / Response code (0x80000000 = success)
 * [2..n]: Property tags (each tag has ID, size, and value fields)
 * [n]: End tag (0)
 */
#[repr(C, align(16))]
pub struct MboxMessage {
    pub data: [u32; 36], // Fixed-size buffer for property tag messages
}

/*
 * Mailbox controller (stateless)
 */
pub struct Mailbox;

impl Mailbox {
    /*
     * Creates a new Mailbox instance
     *
     * Returns: Mailbox (stateless, just provides methods)
     *
     * The mailbox has no state; it's just an interface to hardware registers
     */
    pub fn new() -> Mailbox {
        Mailbox
    }

    /*
     * Sends a message to the GPU via the mailbox and waits for response
     *
     * Parameters:
     * - channel: Mailbox channel number (8 = property tags ARM to VC)
     * - msg: Mutable reference to message buffer (GPU writes response here)
     *
     * Returns: Ok(()) on success, Err(()) on failure
     *
     * How it works:
     * 1. Verify message buffer is 16-byte aligned (GPU requirement)
     * 2. Combine message address with channel number in lower 4 bits
     * 3. Wait until mailbox write queue is not full
     * 4. Write the combined value to trigger GPU processing
     * 5. Poll read queue until our response arrives (matching address+channel)
     * 6. Verify GPU's response code indicates success (0x80000000)
     *
     * Channel numbers:
     * - 0: Power management
     * - 1: Framebuffer
     * - 8: Property tags (used here for flexible GPU requests)
     */
    pub fn call(&self, channel: u32, msg: &mut MboxMessage) -> Result<(), ()> {
        // Get message buffer address
        let ptr = msg.data.as_ptr() as u32;

        // Verify 16-byte alignment (lower 4 bits must be 0)
        if ptr & 0xF != 0 {
            return Err(()); // Alignment error
        }

        // Construct mailbox value: upper 28 bits = address, lower 4 bits = channel
        // The address clearing (!0xF) ensures channel bits don't conflict
        let val = (ptr & !0xF) | (channel & 0xF);

        unsafe {
            // Wait until mailbox is not full (can accept writes)
            // Spin-wait checking FULL flag in status register
            while (read_volatile(MBOX_STATUS) & MBOX_FULL) != 0 {}

            // Write message address + channel to trigger GPU processing
            write_volatile(MBOX_WRITE, val);

            // Poll for response
            loop {
                // Wait until mailbox has data to read (not empty)
                while (read_volatile(MBOX_STATUS) & MBOX_EMPTY) != 0 {}

                // Read response value
                let response = read_volatile(MBOX_READ);

                // Check if this response matches our request (same address+channel)
                // Multiple mailbox transactions can be in flight, so we filter by value
                if response == val {
                    // Check GPU's response code in message data[1]
                    // 0x80000000 = success (high bit set)
                    return if msg.data[1] == 0x80000000 {
                        Ok(()) // GPU processed successfully
                    } else {
                        Err(()) // GPU returned error
                    };
                }
                // If response doesn't match, continue polling (it was for another request)
            }
        }
    }
}
