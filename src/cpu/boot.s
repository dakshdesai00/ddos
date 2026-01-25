/*
 * boot.s - ARM64 Assembly Boot Code for DDOS Operating System
 * 
 * This file contains the initial boot code that runs when the Raspberry Pi starts.
 * It performs critical initialization before jumping to the Rust main function.
 * 
 * Key responsibilities:
 * 1. Ensure only one CPU core executes the boot code (multicore safety)
 * 2. Initialize the stack pointer for proper function call handling
 * 3. Zero out the BSS section (uninitialized global variables)
 * 4. Transfer control to the Rust main function
 */

.section .text._start
.global _start

_start:
    /* MULTICORE SAFETY: Only allow Core 0 to run
     * On Raspberry Pi, multiple CPU cores start simultaneously.
     * We read the Multiprocessor Affinity Register (MPIDR_EL1) to identify
     * which core is running. Bits [1:0] contain the core ID.
     * Core 0 continues to 'master', all others go to 'hang' and sleep forever.
     */
    mrs     x0, mpidr_el1      // Move MPIDR_EL1 system register to x0
    and     x0, x0, #0x3       // Mask to get only the core ID (bits 0-1)
    cbz     x0, master         // Compare x0 to zero; if zero (Core 0), branch to master
    b       hang               // Otherwise, branch to hang (infinite sleep)

master:
    /* STACK INITIALIZATION: Set Stack Pointer
     * The stack grows downward in memory. We set the stack pointer (sp)
     * to the address of _start (0x80000), which is where our code begins.
     * Since we want stack to grow down from our code, this gives us memory
     * from 0x00000000 to 0x80000 for stack usage (512KB).
     */
    ldr     x1, =_start        // Load address of _start symbol into x1
    mov     sp, x1             // Move that address to stack pointer register

    /* BSS SECTION ZEROING: Clean uninitialized global variables
     * The BSS section contains uninitialized global and static variables.
     * By C/Rust convention, these must be zeroed before program execution.
     * We calculate the size of BSS and write zeros to every 8-byte chunk.
     * 
     * Process:
     * 1. Load start and end addresses of BSS (from linker script)
     * 2. Calculate size in bytes
     * 3. Loop through, writing 8-byte chunks of zero
     */
    ldr     x1, =__bss_start   // Load BSS start address into x1
    ldr     w2, =__bss_end     // Load BSS end address into w2
    sub     w2, w2, w1         // Calculate BSS size: end - start
    cbz     w2, jump_main      // If size is 0 (no BSS), skip zeroing and jump to main
    mov     x3, #0             // Load value 0 into x3 (to write to memory)
    
loop_bss:
    /* Loop that zeros 8 bytes at a time:
     * str x3, [x1], #8 - Store x3 (zero) at address x1, then increment x1 by 8
     * This post-increment addressing mode efficiently zeros memory
     */
    str     x3, [x1], #8       // Store zero at current address, then add 8 to x1
    sub     w2, w2, #1         // Decrement counter (treating as 8-byte units)
    cbnz    w2, loop_bss       // If counter != 0, continue loop

jump_main:
    /* TRANSFER TO RUST: Call the Rust main function
     * bl (Branch with Link) calls _main and stores return address in link register.
     * After Rust initialization completes, execution continues to hang.
     */
    bl      _main              // Branch and link to _main (Rust entry point)
    b       hang               // After _main returns (shouldn't happen), go to hang

hang:
    /* INFINITE SLEEP LOOP: Low-power idle state
     * wfe (Wait For Event) puts the CPU in low-power mode until an event occurs.
     * Since we don't configure any events, this effectively halts the CPU.
     * The loop ensures even if wfe returns, we immediately sleep again.
     */
    wfe                        // Wait for event (low-power sleep)
    b       hang               // Loop back to wfe