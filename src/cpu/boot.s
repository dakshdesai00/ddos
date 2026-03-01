.section .text._start
.global _start

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
    bl      _main
    b       hang

hang:
    wfe
    b       hang