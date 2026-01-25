ENTRY(_start)

SECTIONS
{
    . = 0x80000;

    .text : {
        KEEP(*(.text._start))
        *(.text._start_arguments) 
        *(.text*)
    }

    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    
    .bss : {
        . = ALIGN(16);
        __bss_start = .;
        *(.bss*)
        . = ALIGN(16);
        __bss_end = .;
    }

    /DISCARD/ : { *(.eh_frame) *(.comment) }
}