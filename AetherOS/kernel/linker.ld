/*
 * AetherOS x86_64 kernel linker script
 *
 * Layout goals:
 * - Place the `_start` entry stub first in the image.
 * - Keep executable / RO / RW / zeroed sections in canonical order.
 * - Export section boundary symbols used by low-level bring-up and diagnostics.
 * - Reserve explicit alignment for page-sized and ABI-sensitive regions.
 */
ENTRY(_start)

KERNEL_LOAD_ADDR = 0x00100000; /* 1 MiB physical load address */

MEMORY {
    /*
     * Single flat RAM window for early kernel image placement.
     * 64 MiB is intentionally conservative for early boot payloads.
     */
    RAM (rwx) : ORIGIN = KERNEL_LOAD_ADDR, LENGTH = 64M
}

SECTIONS
{
    . = ORIGIN(RAM);
    PROVIDE(__kernel_start = .);

    /*
     * Keep the entry trampoline in its own section at image start so `_start`
     * remains the first executable bytes regardless of compiler section naming.
     */
    .text.start : ALIGN(16)
    {
        PROVIDE(__text_start = .);
        KEEP(*(.text._start))
        KEEP(*(.text._start.*))
    } > RAM

    /* All remaining executable code follows the entry stub. */
    .text : ALIGN(4K)
    {
        *(.text .text.*)
        PROVIDE(__text_end = .);
    } > RAM

    /* Read-only constants and jump tables. */
    .rodata : ALIGN(4K)
    {
        PROVIDE(__rodata_start = .);
        *(.rodata .rodata.*)
        PROVIDE(__rodata_end = .);
    } > RAM

    /* Writable initialized data. */
    .data : ALIGN(4K)
    {
        PROVIDE(__data_start = .);
        *(.data .data.*)
        PROVIDE(__data_end = .);
    } > RAM

    /*
     * Uninitialized data is not stored in the file image.
     * Keep the bootstrap stack first so it gets a stable low-level address.
     */
    .bss (NOLOAD) : ALIGN(4K)
    {
        PROVIDE(__bss_start = .);

        . = ALIGN(16);
        PROVIDE(__stack_start = .);
        KEEP(*(.bss.stack))
        PROVIDE(__stack_end = .);

        . = ALIGN(4K);
        *(.bss .bss.*)
        *(COMMON)

        . = ALIGN(4K);
        PROVIDE(__bss_end = .);
    } > RAM

    /* Optional page-aligned custom sections used by architecture code. */
    .page_tables (NOLOAD) : ALIGN(4K)
    {
        PROVIDE(__page_tables_start = .);
        KEEP(*(.page_tables .page_tables.*))
        . = ALIGN(4K);
        PROVIDE(__page_tables_end = .);
    } > RAM

    .heap (NOLOAD) : ALIGN(4K)
    {
        PROVIDE(__heap_start = .);
        KEEP(*(.heap .heap.*))
        . = ALIGN(4K);
        PROVIDE(__heap_end = .);
    } > RAM

    . = ALIGN(4K);
    PROVIDE(__kernel_end = .);

    /* Compatibility aliases for low-level code expecting unprefixed symbols. */
    PROVIDE(kernel_start = __kernel_start);
    PROVIDE(kernel_end = __kernel_end);
    PROVIDE(text_start = __text_start);
    PROVIDE(text_end = __text_end);
    PROVIDE(rodata_start = __rodata_start);
    PROVIDE(rodata_end = __rodata_end);
    PROVIDE(data_start = __data_start);
    PROVIDE(data_end = __data_end);
    PROVIDE(bss_start = __bss_start);
    PROVIDE(bss_end = __bss_end);
    PROVIDE(stack_start = __stack_start);
    PROVIDE(stack_end = __stack_end);
    PROVIDE(page_tables_start = __page_tables_start);
    PROVIDE(page_tables_end = __page_tables_end);
    PROVIDE(heap_start = __heap_start);
    PROVIDE(heap_end = __heap_end);

    /DISCARD/ :
    {
        *(.eh_frame .eh_frame.*)
        *(.note .note.*)
        *(.comment)
    }

}

/* Fail the link if any section spills out of the RAM placement window. */
ASSERT(__kernel_end <= ORIGIN(RAM) + LENGTH(RAM), "Kernel image exceeds RAM region");
