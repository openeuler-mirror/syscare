module_name ?= upatch_helper
module_version ?= "1.0-dev"
kernel ?= /lib/modules/$(shell uname -r)/build

ccflags-y += -O2
ccflags-y += -DBUILD_VERSION=\"$(module_version)\"
ccflags-y += -I$(PWD)/..
ccflags-y += -Werror -Wall
ccflags-y += -fstack-protector-strong
ccflags-y += -Wl,-Bsymbolic -Wl,-no-undefined -Wl,-z,now -Wl,-z,noexecstack

obj-m += $(module_name).o

$(module_name)-objs := map.o records.o context.o
$(module_name)-objs += cache.o uprobe.o ioctl.o main.o

all:
	make -C $(kernel) M=$(shell pwd) modules

clean:
	make -C $(kernel) M=$(shell pwd) clean
