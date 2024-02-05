// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_IOCTL_H
#define _UPATCH_HIJACKER_KO_IOCTL_H

#include <linux/types.h>
#include <linux/limits.h>

#define UPATCH_HIJACKER_DEV_NAME "upatch-hijacker"
#define UPATCH_HIJACKER_DEV_MODE 0600

#define UPATCH_HIJACKER_IOC_MAGIC 0xE5
#define UPATCH_HIJACKER_ENABLE _IOW(UPATCH_HIJACKER_IOC_MAGIC, 0x1, \
    upatch_enable_request_t)
#define UPATCH_HIJACKER_DISABLE _IO(UPATCH_HIJACKER_IOC_MAGIC, 0x2)
#define UPATCH_HIJACKER_REGISTER _IOW(UPATCH_HIJACKER_IOC_MAGIC, 0x3, \
    upatch_register_request_t)
#define UPATCH_HIJACKER_UNREGISTER _IOW(UPATCH_HIJACKER_IOC_MAGIC, 0x4, \
    upatch_register_request_t)

typedef struct {
    char path[PATH_MAX];
    loff_t offset;
} upatch_enable_request_t;

typedef struct {
    char exec_path[PATH_MAX];
    char jump_path[PATH_MAX];
} upatch_register_request_t;

struct file;

int ioctl_init(void);
void ioctl_exit(void);
long handle_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

#endif /* _UPATCH_HIJACKER_KO_IOCTL_H */
