# SPDX-License-Identifier: GPL-2.0

MODULE_NAME = upatch_manage
ifndef MODULE_VERSION
	MODULE_VERSION := dev
endif

ifeq ($(ENABLE_GCOV), 1)
	GCOV_PROFILE := y
endif

ifndef KERNEL_SRC
	KERNEL_SRC := /lib/modules/$(shell uname -r)/build
endif
MODULE_SRC := $(shell pwd)

ccflags-y += -D__KERNEL__ -DMODULE -DMODNAME=\"${MODULE_NAME}\" -DMODVER=\"${MODULE_VERSION}\"
ccflags-y += -D_FORTIFY_SOURCE=2 -DSECUREC_SUPPORT_FORMAT_WARNING=1
ccflags-y += -O2 -Wall -pipe

obj-m += ${MODULE_NAME}.o
${MODULE_NAME}-objs := main.o ioctl_dev.o
${MODULE_NAME}-objs += patch_entity.o target_entity.o process_entity.o
${MODULE_NAME}-objs += patch_load.o arch/$(ARCH)/patch_load.o symbol_resolve.o
${MODULE_NAME}-objs += patch_manage.o kernel_compat.o

ifeq ($(ARCH), arm64)
	${MODULE_NAME}-objs += arch/$(ARCH)/insn.o
endif

all:
	make -C $(KERNEL_SRC) M=$(MODULE_SRC) modules -j

clean:
	make -C $(KERNEL_SRC) M=$(MODULE_SRC) clean
