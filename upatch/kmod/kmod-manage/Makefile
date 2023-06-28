# SPDX-License-Identifier: GPL-2.0
kernel ?= /lib/modules/$(shell uname -r)/build

EXTRA_CFLAGS += -I$(PWD)
EXTRA_CFLAGS += -I$(PWD)/../../upatch-diff

ifdef UPATCH_VERSION
	EXTRA_CFLAGS += -DUPATCH_VERSION=\"$(UPATCH_VERSION)\"
endif

obj-m += upatch.o
upatch-objs := kmod.o common.o
upatch-objs += compiler.o compiler-args.o
upatch-objs += patch-uprobe.o patch-syscall.o patch.o patch-load.o
upatch-objs += arch/$(ARCH)/patch-syscall-impl.o arch/$(ARCH)/patch-syscall-register.o arch/$(ARCH)/patch-load.o
ifeq ($(ARCH), arm64)
	upatch-objs += arch/$(ARCH)/insn.o
endif

all:
	make -C $(kernel) M=$(shell pwd) modules

clean:
	make -C $(kernel) M=$(shell pwd) clean
