MODULE_NAME ?= upatch_hijacker
MODULE_VERSION ?= "1.0-dev"
KERNEL_DIR ?= /lib/modules/$(shell uname -r)/build

ccflags-y += -O2
ccflags-y += -DBUILD_VERSION=\"$(MODULE_VERSION)\"
ccflags-y += -Werror -Wall
ccflags-y += -fstack-protector-strong
ccflags-y += -Wl,-Bsymbolic -Wl,-no-undefined -Wl,-z,now -Wl,-z,noexecstack

obj-m += $(MODULE_NAME).o

$(MODULE_NAME)-objs := map.o records.o context.o
$(MODULE_NAME)-objs += cache.o uprobe.o ioctl.o main.o

all:
	make -C $(KERNEL_DIR) M=$(PWD) modules

clean:
	make -C $(KERNEL_DIR) M=$(PWD) clean
