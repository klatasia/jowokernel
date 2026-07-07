ENTRY(_start)

SECTIONS
{
    . = 0x8000000000;
    
    .text : {
        *(.text*)
        *(.ltext*)
    }
    
    .rodata : {
        *(.rodata*)
        *(.lrodata*)
    }
    
    .data : {
        *(.data*)
        *(.ldata*)
    }

    .init_array : {
        PROVIDE_HIDDEN (__init_array_start = .);
        KEEP (*(SORT(.init_array.*)))
        KEEP (*(.init_array*))
        PROVIDE_HIDDEN (__init_array_end = .);
    }

    .fini_array : {
        PROVIDE_HIDDEN (__fini_array_start = .);
        KEEP (*(SORT(.fini_array.*)))
        KEEP (*(.fini_array*))
        PROVIDE_HIDDEN (__fini_array_end = .);
    }
    
    .bss : {
        __bss_start = .;
        *(.bss*)
        *(.lbss*)
        __bss_end = .;
    }
    
    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}
