#include <linux/types.h>
#include <linux/string.h>
#include <linux/errno.h>
#include <linux/printk.h>
#include <linux/spinlock.h> 

#include "upatch-entry.h"

/* Too many hijacker will damage the system performance */
#define UPATCH_MAX_HIJACK_ENTRY 16

/* search key: compiler_ino + driver_name / hijacker_ino */
struct upatch_hijack_entry {
    unsigned long compiler_ino;
    unsigned long hijacker_ino;
    const char driver_name[UPATCH_ENTRY_MAX_LEN];
    const char hijacker_name[UPATCH_ENTRY_MAX_LEN];
    unsigned int ref;
};

/* callers decide how to use locks */
static DEFINE_SPINLOCK(entry_lock);
static struct upatch_hijack_entry entry_list[UPATCH_MAX_HIJACK_ENTRY];

static inline bool upatch_streql(const char *src, const char *dst)
{
    /* NULL means skip this check */
    if (!dst)
        return true;
    return (strcmp(src, dst) == 0);
}

static struct upatch_hijack_entry *
find_entry_by_compiler(unsigned long compiler_ino, const char *dirver_name)
{
    unsigned int i;
    for (i = 0; i < UPATCH_MAX_HIJACK_ENTRY; i ++) {
        if (entry_list[i].compiler_ino == compiler_ino &&
            upatch_streql(entry_list[i].driver_name, dirver_name))
            return &entry_list[i]; 
    }
    return NULL;
}

static struct upatch_hijack_entry *
find_entry_by_hijacker(unsigned long hijacker_ino)
{
    unsigned int i;
    for (i = 0; i < UPATCH_MAX_HIJACK_ENTRY; i ++) {
        if (entry_list[i].hijacker_ino == hijacker_ino)
            return &entry_list[i];
    }
    return NULL;
}

static struct upatch_hijack_entry *
create_entry(unsigned long compiler_ino, const char *dirver_name,
    unsigned long hijacker_ino, const char *hijacker_name)
{
    struct upatch_hijack_entry *entry = find_entry_by_compiler(0, NULL);
    if (!entry)
        goto out;

    entry->compiler_ino = compiler_ino;
    entry->hijacker_ino = hijacker_ino;
    strncpy((char *)entry->driver_name, dirver_name, UPATCH_ENTRY_MAX_LEN - 1);
    strncpy((char *)entry->hijacker_name, hijacker_name, UPATCH_ENTRY_MAX_LEN - 1);
    entry->ref = 0;
out:
    return entry;
}

static void destroy_entry(unsigned long compiler_ino, const char *dirver_name)
{
    struct upatch_hijack_entry *entry = find_entry_by_compiler(compiler_ino, dirver_name);
    if (entry)
        memset(entry, 0, sizeof(*entry));
}

static int check_entry(struct upatch_hijack_entry *entry,
    unsigned long hijacker_ino, const char *hijacker_name)
{
    if (entry->hijacker_ino == hijacker_ino &&
        upatch_streql(entry->hijacker_name, hijacker_name))
        return 0;
    return -EINVAL;
}

int upatch_get_matched_entry_name(unsigned long prey_ino, const char *name, char *buff, unsigned int len)
{
    int matched_name_len, ret;
    const char *matched_name = NULL;
    struct upatch_hijack_entry *entry = NULL;

    spin_lock(&entry_lock);
    if (name) {
        entry = find_entry_by_compiler(prey_ino, name);
        if (entry)
            matched_name = entry->hijacker_name;
    } else {
        entry = find_entry_by_hijacker(prey_ino);
        if (entry)
            matched_name = entry->driver_name;
    }

    ret = -ENOENT;
    if (!matched_name)
        goto out;

    ret = -EOVERFLOW;
    matched_name_len = strlen(matched_name) + 1;
    if (matched_name_len > len)
        goto out;

    ret = 0;
    memcpy(buff, matched_name, matched_name_len);
out:
    spin_unlock(&entry_lock);
    return ret;
}

int upatch_register_entry(unsigned long compiler_ino, const char *dirver_name,
    unsigned long hijacker_ino, const char *hijacker_name)
{
    int ret;
    struct upatch_hijack_entry *entry = NULL;

    spin_lock(&entry_lock);
    entry = find_entry_by_compiler(compiler_ino, dirver_name);

    /* Not found, but hijacker alreay been registered, reject it. */
    ret = -EPERM;
    if (!entry && find_entry_by_hijacker(hijacker_ino))
        goto out;

    /* create a new solt */
    if (!entry)
        entry = create_entry(compiler_ino, dirver_name, hijacker_ino, hijacker_name);
    
    /* no solt left */
    ret = -EOVERFLOW;
    if (!entry)
        goto out;

    /* check entry, new slots should always pass */
    ret = check_entry(entry, hijacker_ino, hijacker_name);
    if (ret)
        goto out;

    entry->ref ++;
out:
    spin_unlock(&entry_lock);
    return ret;
}

int upatch_unregister_entry(unsigned long compiler_ino, const char *dirver_name)
{
    int ret;
    struct upatch_hijack_entry *entry = NULL;
    
    spin_lock(&entry_lock);
    ret = -EINVAL;
    entry = find_entry_by_compiler(compiler_ino, dirver_name);
    if (!entry)
        goto out;
    entry->ref --;
    if (!entry->ref)
        destroy_entry(compiler_ino, dirver_name);
    ret = 0;
out:
    spin_unlock(&entry_lock);
    return 0;
}


