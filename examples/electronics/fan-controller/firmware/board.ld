ENTRY(reset)
MEMORY {
  flash (rx) : ORIGIN = 0x08000000, LENGTH = 128K
  ram (rwx) : ORIGIN = 0x20000000, LENGTH = 32K
}
_stack = ORIGIN(ram) + LENGTH(ram);
SECTIONS {
  .vectors : { KEEP(*(.vectors)) } > flash
  .text : { *(.text*) *(.rodata*) } > flash
  .data : { _data = .; *(.data*) . = ALIGN(4); _edata = .; } > ram AT>flash
  _idata = LOADADDR(.data);
  .bss (NOLOAD) : { _bss = .; *(.bss*) *(COMMON) . = ALIGN(4); _ebss = .; } > ram
}
