#ifndef __UPATCH_IOCTL_H_
#define __UPATCH_IOCTL_H_

#include "upatch-meta.h"

int patch_ioctl_apply(const char *target_path, const char *patch_path,
    struct list_head *symbol_list);

int patch_ioctl_remove(const char *target_path, const char *patch_path,
    struct list_head *symbol_list);

#endif
