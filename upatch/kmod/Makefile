# SPDX-License-Identifier: GPL-2.0
kernel ?= /lib/modules/$(shell uname -r)/build

obj-m += upatch.o
upatch-y += kmod.o
upatch-y += compiler.o compiler_args.o ./asm/$(ARCH)/hijack_code.o ./asm/$(ARCH)/hijack.o

all:
	make -C $(kernel) M=$(shell pwd) modules

clean:
	make -C $(kernel) M=$(shell pwd) clean