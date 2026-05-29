.section .text._start
.global _start

.extern exception_vector_table

_start:
    mrs     x0, mpidr_el1
    and     x0, x0, #0x3
    cbz     x0, master
    b       hang

master:
    ldr     x1, =_start
    mov     sp, x1

    ldr     x1, =__bss_start
    ldr     x2, =__bss_end
    cmp     x1, x2
    b.hs    jump_main
    mov     x3, #0

loop_bss:
    str     x3, [x1], #8
    cmp     x1, x2
    b.lo    loop_bss

jump_main:
    ldr     x0, =exception_vector_table
    mrs     x1, CurrentEL
    lsr     x1, x1, #2

    cmp     x1, #1
    b.eq    el1_entry
    cmp     x1, #2
    b.eq    from_el2
    cmp     x1, #3
    b.eq    from_el3

from_el3:
    // Set EL3 vector (for completeness) and configure SCR_EL3
    msr     vbar_el3, x0
    mrs     x1, scr_el3
    orr     x1, x1, #(1<<0)   // NS = 1 (Non-secure)
    orr     x1, x1, #(1<<8)   // HCE = 1 (Enable HVC at lower ELs)
    orr     x1, x1, #(1<<10)  // RW = 1 (AArch64 at EL2/EL1)
    msr     scr_el3, x1

    // Return to EL2h
    ldr     x1, =from_el3_to_el2
    msr     elr_el3, x1
    mov     x1, #0b1001       // M[3:0] = EL2h
    orr     x1, x1, #0x3C0    // Mask D, A, I, F
    msr     spsr_el3, x1
    eret

from_el3_to_el2:
    b       from_el2

from_el2:
    // Set EL2 vector and configure HCR_EL2 for EL1 AArch64
    msr     vbar_el2, x0
    mov     x1, #(1<<31)      // HCR_EL2.RW = 1 (EL1 is AArch64)
    msr     hcr_el2, x1

    // Set up EL1 stack pointer
    ldr     x1, =_start
    msr     sp_el1, x1

    // Return to EL1h
    ldr     x1, =el1_entry
    msr     elr_el2, x1
    mov     x1, #0b0101       // M[3:0] = EL1h
    orr     x1, x1, #0x3C0    // Mask D, A, I, F
    msr     spsr_el2, x1
    eret

el1_entry:
    msr     vbar_el1, x0
    ldr     x1, =_start
    mov     sp, x1
    msr     daifclr, #0b1111
    bl      _main
    b       hang

hang:
    wfe
    b       hang
