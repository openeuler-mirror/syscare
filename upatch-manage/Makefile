# SPDX-License-Identifier: GPL-2.0

MODULE_NAME = upatch_manage
ifndef MODULE_VERSION
	MODULE_VERSION := dev
endif

ifeq ($(ENABLE_GCOV), 1)
	GCOV_PROFILE := y
endif

obj-m += $(MODULE_NAME).o
$(MODULE_NAME)-objs := kernel_compat.o process_cache.o ioctl_dev.o main.o
$(MODULE_NAME)-objs += patch_entity.o target_entity.o process_entity.o
$(MODULE_NAME)-objs += patch_load.o arch/$(ARCH)/patch_load.o symbol_resolve.o stack_check.o
$(MODULE_NAME)-objs += patch_manage.o

ifeq ($(ARCH), arm64)
	${MODULE_NAME}-objs += arch/$(ARCH)/insn.o
endif

all:
	make -C $(KERNEL_SRC) M=$(MODULE_SRC) modules -j

clean:
	make -C $(KERNEL_SRC) M=$(MODULE_SRC) clean
