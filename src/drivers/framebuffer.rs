/*
 * framebuffer.rs - Video Framebuffer Manager for DDOS
 *
 * This module manages the hardware framebuffer for video output on Raspberry Pi.
 * It communicates with the GPU via the mailbox interface to allocate and configure
 * a framebuffer, then provides pixel-level drawing operations.
 *
 * The framebuffer is a region of memory shared between CPU and GPU where each
 * pixel's color is stored. Writing to this memory updates what appears on screen.
 *
 * Resolution: 1920x1080 (Full HD)
 * Color depth: 32-bit ARGB (8 bits each for Alpha, Red, Green, Blue)
 */

use super::mailbox::{Mailbox, MboxMessage};
use core::ptr::write_volatile;

/*
 * FrameBuffer Structure
 *
 * Fields:
 * - width: Screen width in pixels (1920)
 * - height: Screen height in pixels (1080)
 * - pitch: Bytes per row (width * 4, since each pixel is 4 bytes)
 * - base_addr: Physical memory address where framebuffer begins
 */
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub base_addr: usize,
}

impl FrameBuffer {
    /*
     * Initializes the framebuffer by communicating with the GPU
     *
     * Returns: Result containing FrameBuffer on success, or error code on failure
     *
     * How it works:
     * 1. Construct a mailbox message with multiple property tags:
     *    - Set physical display size (1920x1080)
     *    - Set virtual display size (1920x1080, no double buffering)
     *    - Set virtual offset (0,0) - critical for QEMU compatibility
     *    - Set color depth (32-bit)
     *    - Request framebuffer allocation from GPU
     * 2. Send message to GPU via mailbox channel 8 (property tags)
     * 3. Verify GPU successfully processed the request
     * 4. Extract framebuffer base address from GPU response
     * 5. Return initialized FrameBuffer structure
     *
     * Error codes:
     * - 1: Mailbox communication failure
     * - 2: GPU returned invalid address (0)
     * - Other: GPU-specific error codes
     */
    pub fn new() -> Result<FrameBuffer, u32> {
        // Create mailbox message structure
        let mut mbox = MboxMessage { data: [0; 36] };

        // Message header
        mbox.data[0] = 35 * 4; // Total size in bytes (35 words * 4 bytes/word)
        mbox.data[1] = 0; // Request code (0 = request, GPU will set to 0x80000000 on success)

        // Index for building property tags sequentially
        let mut i = 2;
        // Property Tag 1: Set Physical Display Size
        // This sets the actual display resolution
        mbox.data[i + 0] = 0x00048003; // Tag ID for "Set Physical Width/Height"
        mbox.data[i + 1] = 8; // Value buffer size (2 words)
        mbox.data[i + 2] = 8; // Request size (2 words)
        mbox.data[i + 3] = 1920; // Width in pixels
        mbox.data[i + 4] = 1080; // Height in pixels
        i += 5;

        // Property Tag 2: Set Virtual Display Size
        // Virtual size can be larger than physical for scrolling/double buffering
        // We set it equal to physical size (no scrolling/double buffering)
        mbox.data[i + 0] = 0x00048004; // Tag ID for "Set Virtual Width/Height"
        mbox.data[i + 1] = 8; // Value buffer size (2 words)
        mbox.data[i + 2] = 8; // Request size (2 words)
        mbox.data[i + 3] = 1920; // Virtual width in pixels
        mbox.data[i + 4] = 1080; // Virtual height in pixels
        i += 5;

        // Property Tag 3: Set Virtual Offset
        // Defines which part of virtual framebuffer is displayed (for panning/scrolling)
        // Set to (0,0) to display from top-left corner
        // CRITICAL: QEMU Raspberry Pi 4 emulation requires this tag
        mbox.data[i + 0] = 0x00048009; // Tag ID for "Set Virtual Offset"
        mbox.data[i + 1] = 8; // Value buffer size (2 words)
        mbox.data[i + 2] = 8; // Request size (2 words)
        mbox.data[i + 3] = 0; // X offset (0 = start at left)
        mbox.data[i + 4] = 0; // Y offset (0 = start at top)
        i += 5;

        // Property Tag 4: Set Color Depth
        // 32 bits per pixel = ARGB (8 bits alpha, 8 red, 8 green, 8 blue)
        mbox.data[i + 0] = 0x00048005; // Tag ID for "Set Depth"
        mbox.data[i + 1] = 4; // Value buffer size (1 word)
        mbox.data[i + 2] = 4; // Request size (1 word)
        mbox.data[i + 3] = 32; // Bits per pixel
        i += 4;

        // Property Tag 5: Allocate Framebuffer
        // Requests GPU to allocate memory for framebuffer with specified alignment
        mbox.data[i + 0] = 0x00040001; // Tag ID for "Allocate Buffer"
        mbox.data[i + 1] = 8; // Value buffer size (2 words)
        mbox.data[i + 2] = 8; // Request size (2 words)
        mbox.data[i + 3] = 4096; // Alignment requirement (4KB = page size)
        mbox.data[i + 4] = 0; // Placeholder for response (GPU writes address here)
        i += 5;

        // End tag (value 0 signals end of property tag list)
        mbox.data[i] = 0;

        // Send mailbox message to GPU and check for errors
        let mb = Mailbox::new();

        // Check 1: Verify mailbox communication succeeded
        // Channel 8 is used for property tag messages to GPU
        if mb.call(8, &mut mbox).is_err() {
            return Err(1); // Error Code 1: Mailbox hardware communication failed
        }

        // Check 2: Verify GPU successfully processed all property tags
        // GPU writes 0x80000000 to data[1] on success (high bit set = success)
        if mbox.data[1] != 0x80000000 {
            return Err(mbox.data[1]); // Return GPU's error code
        }

        // Extract framebuffer base address from GPU response
        //
        // Message layout calculation:
        // Index 0-1:   Header (size, request code)
        // Index 2-6:   Physical size tag (5 words)
        // Index 7-11:  Virtual size tag (5 words)
        // Index 12-16: Virtual offset tag (5 words)
        // Index 17-20: Depth tag (4 words)
        // Index 21-25: Allocate buffer tag (5 words)
        //   21: Tag ID
        //   22: Buffer size
        //   23: Request/Response size
        //   24: Alignment (request) / Base Address (response) <- GPU writes address here
        //   25: Size (response)
        //
        // The GPU writes the framebuffer base address to index 24
        let ptr_val = mbox.data[24];

        // Use address directly as returned by GPU
        // Note: On real RPi, addresses have specific bit patterns (e.g., 0x3E000000)
        // QEMU emulation may use different addresses; trust what GPU returns
        let base_addr = ptr_val as usize;

        // Validate that GPU returned a valid address
        if base_addr == 0 {
            return Err(2); // Error Code 2: GPU returned null address (allocation failed)
        }

        // Create and return FrameBuffer with GPU-allocated memory
        Ok(FrameBuffer {
            width: 1920,     // Display width in pixels
            height: 1080,    // Display height in pixels
            pitch: 1920 * 4, // Bytes per row (width * 4 bytes per pixel)
            base_addr,       // Physical memory address from GPU
        })
    }

    /*
     * Draws a single pixel at the specified coordinates
     *
     * Parameters:
     * - x: Horizontal position (0 = left edge)
     * - y: Vertical position (0 = top edge)
     * - color: 32-bit ARGB color value (0xAARRGGBB)
     *
     * How it works:
     * 1. Validate coordinates are within screen bounds (clip to prevent memory corruption)
     * 2. Calculate byte offset: each row is 'pitch' bytes, each pixel is 4 bytes
     *    Formula: offset = (row * bytes_per_row) + (column * bytes_per_pixel)
     * 3. Calculate absolute memory address by adding offset to base address
     * 4. Write color value using volatile write (prevents compiler optimization)
     *
     * Note: write_volatile ensures the write isn't optimized away by the compiler,
     * which is critical for memory-mapped hardware like framebuffers.
     */
    pub fn draw_pixel(&self, x: u32, y: u32, color: u32) {
        // Bounds checking: ignore pixels outside screen area
        if x >= self.width || y >= self.height {
            return;
        }

        // Calculate byte offset within framebuffer
        // pitch = bytes per row, each pixel = 4 bytes (32-bit color)
        let offset = (y * self.pitch) + (x * 4);

        // Write directly to framebuffer memory
        unsafe {
            let addr = (self.base_addr + offset as usize) as *mut u32;
            write_volatile(addr, color); // Volatile write prevents optimization
        }
    }
}
