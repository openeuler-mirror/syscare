# SPDX-License-Identifier: GPL-2.0
kernel ?= /lib/modules/$(shell uname -r)/build

EXTRA_CFLAGS += -I$(PWD)/include

obj-m += upatch_manager.o
upatch_manager-objs := src/module.o src/ioctl.o src/uprobe.o src/utils.o
upatch_manager-objs += src/monitor_list.o src/uprobe_list.o src/pid_list.o

all:
	make -C $(kernel) M=$(shell pwd) modules

clean:
	make -C $(kernel) M=$(shell pwd) clean
install:
	insmod upatch_manager.ko
