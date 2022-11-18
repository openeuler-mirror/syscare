# SPDX-License-Identifier: GPL-2.0
kernel ?= /lib/modules/$(shell uname -r)/build

EXTRA_CFLAGS=-I$(PWD)/../upatch-diff

obj-m += upatch.o
upatch-objs := kmod.o common.o
upatch-objs += compiler.o compiler-args.o
upatch-objs += asm/$(ARCH)/hijack-code.o asm/$(ARCH)/hijack.o
upatch-objs += patch-uprobe.o patch.o patch-load.o

all:
	make -C $(kernel) M=$(shell pwd) modules

clean:
	make -C $(kernel) M=$(shell pwd) clean