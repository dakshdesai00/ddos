/*
 * link.ld - Linker Script for DDOS Kernel
 * 
 * This linker script controls how the compiler organizes our kernel in memory.
 * It defines the memory layout and section placement for the ARM64 executable.
 * 
 * Raspberry Pi boot process:
 * 1. GPU firmware loads kernel8.img at address 0x80000
 * 2. CPU starts executing at _start (first instruction)
 * 3. Memory below 0x80000 is available for stack
 * 4. Memory at/above 0x80000 contains our kernel code and data
 */

/* Entry point - tells linker which symbol is the start of execution */
ENTRY(_start)

SECTIONS
{
    /* Set location counter to 0x80000 (where RPi loads kernel) */
    . = 0x80000;

    /* .text section: Executable code */
    .text : {
        /* Keep _start section even if appears unused (it's our entry point!) */
        KEEP(*(.text._start))
        /* Boot arguments (if any) */
        *(.text._start_arguments) 
        /* All other code sections */
        *(.text*)
    }

    /* .rodata section: Read-only data (const variables, string literals) */
    .rodata : { *(.rodata*) }
    
    /* .data section: Initialized global/static variables */
    .data : { *(.data*) }
    
    /* .bss section: Uninitialized global/static variables */
    /* Must be zeroed by boot code before main() runs */
    .bss : {
        . = ALIGN(16);        /* Align to 16-byte boundary */
        __bss_start = .;      /* Export start address for boot.s */
        *(.bss*)              /* Include all .bss sections */
        . = ALIGN(16);        /* Align end to 16-byte boundary */
        __bss_end = .;        /* Export end address for boot.s */
    }

    /* Discard unwanted sections to save space */
    /DISCARD/ : { 
        *(.eh_frame)   /* Exception handling frames (not needed in no_std) */
        *(.comment)    /* Compiler comments */
    }
}