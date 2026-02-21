/*
 * utils/mod.rs - Utility Module for DDOS
 *
 * NOTE: This is AI-generated boilerplate code for module organization.
 *
 * This module re-exports utility functionality used throughout the OS.
 *
 * Utility Components:
 * - font: 8x8 bitmap font data for console character rendering
 *   Used by: Console driver for drawing text on framebuffer
 *   Data: 95 ASCII printable characters (0x20-0x7E)
 *
 * - locked: Synchronization primitive for safe access to shared data
 *   Used by: Memory allocator and other components requiring interior mutability
 *   Purpose: Allows mutable access to shared static variables in single-threaded environment
 */

pub mod font;
pub mod locked;
