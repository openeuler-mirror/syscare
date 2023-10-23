#ifndef __UPATCH_IOCTL_H_
#define __UPATCH_IOCTL_H_

#include "upatch-meta.h"

#define UPATCH_IOCTL_MAGIC 0xE5

typedef struct elf_request {
    char elf_path[PATH_MAX];
    loff_t offset;
    pid_t monitor_pid;
    char patch_path[PATH_MAX];
} elf_request_t;

struct upatch_pid {
    pid_t *buf;
    int size;
};

#define UPATCH_UPROBE_INIT		_IO(UPATCH_IOCTL_MAGIC, 0x5)
#define UPATCH_UPROBE_DESTROY		_IO(UPATCH_IOCTL_MAGIC, 0x6)
#define UPATCH_REGISTER_ELF		_IOW(UPATCH_IOCTL_MAGIC, 0x7, const elf_request_t *)
#define UPATCH_DEREGISTER_ELF		_IOW(UPATCH_IOCTL_MAGIC, 0x8, const elf_request_t *)
#define UPATCH_GET_PID			_IOW(UPATCH_IOCTL_MAGIC, 0x9, struct upatch_pid *)
#define UPATCH_REGISTER_MONITOR		_IO(UPATCH_IOCTL_MAGIC, 0x10)
#define UPATCH_DEREGISTER_MONITOR	_IO(UPATCH_IOCTL_MAGIC, 0x11)
#define UPATCH_ACTIVE_PATCH		_IOW(UPATCH_IOCTL_MAGIC, 0x12, const elf_request_t *)
#define UPATCH_REMOVE_PATCH		_IOW(UPATCH_IOCTL_MAGIC, 0x13, const elf_request_t *)



long handle_ioctl(struct file *filp, unsigned int cmd, unsigned long arg);

int patch_ioctl_apply(const char *target_path, const char *patch_path,
    struct list_head *symbol_list);

int patch_ioctl_remove(const char *target_path, const char *patch_path,
    struct list_head *symbol_list);

#endif
