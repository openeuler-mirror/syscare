#include "upatch-ioctl.h"

#include <sys/types.h>
#include <sys/ioctl.h>

#include "list.h"

static const char *UPATCH_DEV = "/dev/upatch";

int patch_ioctl_apply(const char *target_path, const char *patch_path,
    struct list_head *symbol_list)
{
    // TODO: Call ioctl to request kernel driver to load patch
    // ioctl -> ko -> register uprobe -> uprobe handler -> execute upatch-manage
    return 0;
}

int patch_ioctl_remove(const char *target_path, const char *patch_path,
    struct list_head *symbol_list)
{
    // TODO: Call ioctl to request kernel driver to remove patch
    // ioctl -> ko -> remove uprobe -> execute upatch-manage
    return 0;
}
