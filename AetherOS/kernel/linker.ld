ENTRY(_start)

SECTIONS
{
    . = 0x100000; /* 1MB kernel load address */

    .text : ALIGN(4K) {
        *(.text*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata*)
    }

    .data : ALIGN(4K) {
        *(.data*)
    }

    .bss : ALIGN(4K) {
        *(.bss.stack)
        *(.bss*)
        *(COMMON)
    }
}
