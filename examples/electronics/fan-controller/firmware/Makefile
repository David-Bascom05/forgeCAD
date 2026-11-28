CROSS ?= arm-none-eabi-
OUT ?= build
CFLAGS = -mcpu=cortex-m0plus -mthumb -std=c11 -Os -ffreestanding -fno-builtin -Wall -Wextra -Werror

all: $(OUT)/fan-node.bin

$(OUT)/fan-node.elf: main.c board.ld
	mkdir -p $(OUT)
	$(CROSS)gcc $(CFLAGS) -nostdlib -Wl,--build-id=none,-T,board.ld main.c -lgcc -o $@

$(OUT)/fan-node.bin: $(OUT)/fan-node.elf
	$(CROSS)objcopy -O binary $< $@
