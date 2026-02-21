/*
 * drivers/mod.rs - Device Driver Module for DDOS
 *
 * NOTE: This is AI-generated boilerplate code for module organization.
 *
 * This module re-exports device drivers for various hardware peripherals.
 * Each driver provides a high-level interface to interact with specific hardware.
 *
 * Driver Components:
 * - uart: PL011 UART serial communication driver
 *   Used for: Serial/debug output, communication with host via USB-serial cable
 *
 * - mailbox: Mailbox interface for CPU-GPU communication
 *   Used for: Property tag messages to request GPU services (framebuffer allocation, etc.)
 *
 * - framebuffer: GPU framebuffer manager for video output
 *   Used for: Pixel-level drawing operations on screen (1920x1080 resolution)
 *
 * - console: Text console with character rendering
 *   Used for: Displaying text and debug output using 8x8 bitmap font
 */

pub mod console;
pub mod framebuffer;
pub mod mailbox;
pub mod uart;
