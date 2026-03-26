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

    cmp     x1, #3
    b.eq    set_el3
    cmp     x1, #2
    b.eq    set_el2

set_el1:
    msr     vbar_el1, x0
    b       continue

set_el2:
    msr     vbar_el2, x0
    mrs     x1, hcr_el2
    orr     x1, x1, #(1<<4)
    orr     x1, x1, #(1<<5)
    msr     hcr_el2, x1
    b       continue

set_el3:
    msr     vbar_el3, x0
    mrs     x1, scr_el3
    orr     x1, x1, #(1<<1)
    orr     x1, x1, #(1<<2)
    msr     scr_el3, x1
    b       continue

continue:
    msr     daifclr, #0b1111
    bl      _main
    b       hang

hang:
    wfe
    b       hang
