
.global exception_vector_table

.macro ventry label
.align 7
    b \label
.endm

.macro kernel_entry
    stp x0, x1, [sp, #-16]!
    stp x2, x3, [sp, #-16]!
    stp x4, x5, [sp, #-16]!
    stp x6, x7, [sp, #-16]!
    stp x8, x9, [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x18, x30, [sp, #-16]!


    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #3
    b.eq 3f
    cmp x0, #2
    b.eq 2f


1:  mrs x1, elr_el1
    mrs x2, spsr_el1
    b 4f
2:  mrs x1, elr_el2
    mrs x2, spsr_el2
    b 4f
3:  mrs x1, elr_el3
    mrs x2, spsr_el3


4:  stp x1, x2, [sp, #-16]!
.endm


.macro kernel_exit

    ldp x1, x2, [sp], #16


    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #3
    b.eq 3f
    cmp x0, #2
    b.eq 2f

1:  msr elr_el1, x1
    msr spsr_el1, x2
    b 4f
2:  msr elr_el2, x1
    msr spsr_el2, x2
    b 4f
3:  msr elr_el3, x1
    msr spsr_el3, x2


4:  ldp x18, x30, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x10, x11, [sp], #16
    ldp x8, x9, [sp], #16
    ldp x6, x7, [sp], #16
    ldp x4, x5, [sp], #16
    ldp x2, x3, [sp], #16
    ldp x0, x1, [sp], #16
    eret
.endm

.align 11
exception_vector_table:
    ventry el1_sync_invalid
    ventry el1_irq_invalid
    ventry el1_fiq_invalid
    ventry el1_error_invalid

    ventry el1_sync
    ventry el1_irq
    ventry el1_fiq
    ventry el1_error

    ventry el0_sync_invalid
    ventry el0_irq_invalid
    ventry el0_fiq_invalid
    ventry el0_error_invalid

    ventry el0_sync_invalid_32
    ventry el0_irq_invalid_32
    ventry el0_fiq_invalid_32
    ventry el0_error_invalid_32


el1_irq:
    kernel_entry
    bl handle_irq
    kernel_exit

el1_fiq:
    kernel_entry
    bl handle_irq
    kernel_exit

el1_sync:
    kernel_entry
    bl handle_sync
    kernel_exit

el1_error:
    kernel_entry
    bl handle_sync
    kernel_exit



el1_sync_invalid:
    b el1_sync
el1_irq_invalid:
el1_fiq_invalid:
el1_error_invalid:
el0_sync_invalid:
el0_irq_invalid:
el0_fiq_invalid:
el0_error_invalid:
el0_sync_invalid_32:
el0_irq_invalid_32:
el0_fiq_invalid_32:
el0_error_invalid_32:
    b .
