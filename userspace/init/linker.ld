ENTRY(_start)

SECTIONS
{
    . = 0x4000000000; /* 256 GB mark, jauh dari Identity Map Kernel */
    
    .text : {
        *(.text*)
    }
    
    .rodata : {
        *(.rodata*)
    }
    
    .data : {
        *(.data*)
    }
    
    .bss : {
        *(.bss*)
    }
    
    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}
