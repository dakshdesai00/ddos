.section .text._start
.global _start

_start:
    /* Only allow Core 0 to run */
    mrs     x0, mpidr_el1
    and     x0, x0, #0x3
    cbz     x0, master
    b       hang

master:
    /* Set Stack Pointer */
    ldr     x1, =_start
    mov     sp, x1 

    /* Zero out BSS section (Clean memory) */
    ldr     x1, =__bss_start
    ldr     w2, =__bss_end
    sub     w2, w2, w1
    cbz     w2, jump_main
    mov     x3, #0
loop_bss:
    str     x3, [x1], #8
    sub     w2, w2, #1
    cbnz    w2, loop_bss

jump_main:
    bl      _main
    b       hang

hang:
    wfe
    b       hang