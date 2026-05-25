.global cpu_switch_to
cpu_switch_to:
    stp     x19, x20, [x0, #0]
    stp     x21, x22, [x0, #16]
    stp     x23, x24, [x0, #32]
    stp     x25, x26, [x0, #48]
    stp     x27, x28, [x0, #64]
    stp     x29, x30, [x0, #80]
    mov     x9, sp
    str     x9, [x0, #96]

    // (We do NOT save ttbr0 here, because a living process never changes its root Page Table)

    ldp     x19, x20, [x1, #0]
    ldp     x21, x22, [x1, #16]
    ldp     x23, x24, [x1, #32]
    ldp     x25, x26, [x1, #48]
    ldp     x27, x28, [x1, #64]
    ldp     x29, x30, [x1, #80]
    ldr     x9, [x1, #96]
    mov     sp, x9

    // Load ttbr0 (the new physical Page Table address) from offset 104
    ldr     x10, [x1, #104]
    msr     ttbr0_el1, x10

    // INLINE TLB FLUSH (Avoids overwriting x30)
    dsb     ish         // Data Synchronization Barrier (wait for memory writes)
    tlbi    vmalle1is   // TLB Invalidate All, EL1, Inner Shareable
    dsb     ish         // Wait for the invalidation to physically complete
    isb                 // Instruction Synchronization Barrier (flush CPU pipeline)

    // Return to the new process (jumps to the address now in x30)
    ret
