// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *   Zongwu Li <lzw32321226@163.com>
 *
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/namei.h>
#include <linux/uprobes.h>
#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/proc_fs.h>
#include <linux/elf.h>
#include <linux/dcache.h>
#include <linux/file.h>
#include <linux/fs.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/string.h>
#include <linux/spinlock.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/ctype.h>

#include <asm/ptrace.h>

#include "kmod.h"
#include "compiler.h"
#include "common.h"
#include "patch-uprobe.h"
#include "patch-syscall.h"
#include "upatch-ioctl.h"

struct elf_path {
    char name[PATH_MAX];
    unsigned int count;
    loff_t entry_offset;
    struct list_head list;
};

static DEFINE_MUTEX(compiler_steps_lock);
static LIST_HEAD(compiler_steps_list);

static LIST_HEAD(compiler_paths_list);
static DEFINE_MUTEX(compiler_paths_lock);

static LIST_HEAD(assembler_paths_list);
static DEFINE_MUTEX(assembler_paths_lock);

static LIST_HEAD(link_paths_list);
static DEFINE_MUTEX(link_paths_lock);

static struct step *__get_compiler_step(char *name)
{
    struct step *step;
    list_for_each_entry(step, &compiler_steps_list, list)
        if (strncmp(step->name, name, STEP_MAX_LEN) == 0)
            return step;
    return NULL;
}

static struct step *get_compiler_step(char *name)
{
    struct step *step;
    mutex_lock(&compiler_steps_lock);
    step = __get_compiler_step(name);
    mutex_unlock(&compiler_steps_lock);
    return step;
}

static int __register_compiler_step(char *name, step_handler_t step_handler)
{
    struct step *step;

    if (!name || !step_handler)
        return -EINVAL;

    if (__get_compiler_step(name))
        return 0;

    step = kzalloc(sizeof(*step), GFP_KERNEL);
    if (!step)
        return -ENOMEM;

    strncpy(step->name, name, STEP_MAX_LEN);
    step->step_handler = step_handler;
    list_add(&step->list, &compiler_steps_list);
    return 0;
}

int register_compiler_step(char *name, step_handler_t step_handler)
{
    int ret;
    mutex_lock(&compiler_steps_lock);
    ret = __register_compiler_step(name, step_handler);
    mutex_unlock(&compiler_steps_lock);
    return ret;
}

static void __unregister_compiler_step(char *name)
{
    struct step *step;

    if (!name)
        return;

    step = __get_compiler_step(name);
    if (step) {
        list_del(&step->list);
        kfree(step);
    }
}

void unregister_compiler_step(char *name)
{
    mutex_lock(&compiler_steps_lock);
    __unregister_compiler_step(name);
    mutex_unlock(&compiler_steps_lock);
}

void clear_compiler_step(void)
{
    struct step *step, *tmp;
    mutex_lock(&compiler_steps_lock);
    list_for_each_entry_safe(step, tmp, &compiler_steps_list, list)
        __unregister_compiler_step(step->name);
    mutex_unlock(&compiler_steps_lock);
}

static struct step *check_env_for_step(char __user **envp, unsigned long *cmd_addr, char *name, int name_len)
{
    int ret;
    size_t len;
    char __env[CMD_MAX_LEN];
    unsigned long env_pos;

    ret = obtain_parameter_addr(envp, name, cmd_addr, NULL);
    if (ret || *cmd_addr == 0) {
        pr_debug("no valid env found for %s \n", name);
        return NULL;
    }

    env_pos = *cmd_addr;
    len = strnlen_user((void __user *)env_pos, MAX_ARG_STRLEN);

    if (len >= CMD_MAX_LEN)
        return NULL;

    if (copy_from_user(__env, (void __user *)env_pos, len))
        return NULL;

    return get_compiler_step(&__env[name_len + 1]);
}

static const char *get_real_path(const char *path_buff, unsigned int buff_size)
{
    int err;
    struct path path;
    const char* real_name = NULL;

    err = kern_path(path_buff, LOOKUP_FOLLOW, &path);
    if (!err)
        real_name = d_path(&path, (void *)path_buff, buff_size);
    return real_name;
}

/* check https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf for initial stack */
static int uprobe_compiler_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    unsigned long cmd_addr;
    struct step *step;
    char __user **envp = get_env_from_regs(regs);

    if (!envp)
        return 0;

    step = check_env_for_step(envp, &cmd_addr, COMPILER_CMD_ENV, COMPILER_CMD_ENV_LEN);
    if (!step) {
        pr_debug("no upatch cmd found \n");
        return 0;
    }

    return step->step_handler(step, regs, (char __user *)cmd_addr);
}

static struct uprobe_consumer uprobe_compiler_consumer = {
    .handler = uprobe_compiler_handler,
    .ret_handler = NULL,
    .filter = uprobe_default_filter,
};

static int uprobe_assembler_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    unsigned long cmd_addr;
    struct step *step;
    char __user **envp = get_env_from_regs(regs);

    if (!envp)
        return 0;

    step = check_env_for_step(envp, &cmd_addr, ASSEMBLER_CMD_ENV, ASSEMBLER_CMD_ENV_LEN);
    if (!step) {
        pr_debug("no upatch cmd found \n");
        return 0;
    }

    return step->step_handler(step, regs, (char __user *)cmd_addr);
}

static struct uprobe_consumer uprobe_assembler_consumer = {
    .handler = uprobe_assembler_handler,
    .ret_handler = NULL,
    .filter = uprobe_default_filter,
};

static int __init compiler_step_init(void)
{
    int ret;
    // for compiler
    ret = register_compiler_step(CMD_COMPILER_SOURCE_ENTER, args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_COMPILER_SOURCE_AFTER, args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_COMPILER_PATCHED_ENTER, args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_COMPILER_PATCHED_AFTER, args_handler);
    if (ret)
        goto out;

    //for assembler
    ret = register_compiler_step(CMD_ASSEMBLER_SOURCE_ENTER, args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_ASSEMBLER_SOURCE_AFTER, args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_ASSEMBLER_PATCHED_ENTER, args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_ASSEMBLER_PATCHED_AFTER, args_handler);
    if (ret)
        goto out;

out:
    return ret;
}

static int __unregister_uprobe(unsigned int cmd, struct elf_path *ep, struct uprobe_consumer *uc)
{
    int ret;
    struct inode *inode;
    struct path path;

    /* if path is null, return directly */
    if (strlen(ep->name) == 0)
        return 0;

    ret = kern_path(ep->name, LOOKUP_FOLLOW, &path);
    if (ret) {
        pr_err("kernel path failed - %d \n", ret);
        goto out;
    }
    inode = path.dentry->d_inode;

    pr_debug("unregister uprobe for %s \n", ep->name);
    uprobe_unregister(inode, ep->entry_offset, uc);
out:
    return ret;
}

static int __elf_check(struct file *file, loff_t *entry_offset)
{
    Elf_Ehdr elf_header;
    int ret;
    elf_addr_t min_addr;

    ret = kernel_read(file, &elf_header, sizeof(elf_header), 0);
    if (ret != sizeof(elf_header)) {
        pr_err("kernel read failed - %d \n", ret);
        ret = -ENOMEM;
        goto out;
    }

    min_addr = calculate_load_address(file, false);
    if (min_addr == -1) {
        pr_err("no valid segment found \n");
        ret = -EINVAL;
        goto out;
    }

    *entry_offset = elf_header.e_entry - min_addr;

    ret = 0;
out:
    return ret;
}

static int elf_check(char *elf_path, loff_t *entry_offset)
{
    struct file *file;
    int ret;
    char *p;

    file = filp_open(elf_path, O_RDONLY, 0);
    if (IS_ERR(file)) {
        ret = PTR_ERR(file);
        pr_err("open elf failed - %d \n", ret);
        goto out;
    }

    p = file_path(file, elf_path, PATH_MAX);
    if (IS_ERR(p)) {
        ret = PTR_ERR(p);
        pr_err("obtain path failed - %d \n", ret);
        goto put_file;
    }

    memmove(elf_path, p, strlen(p) + 1);
    pr_debug("elf path is %s len is %lu \n", elf_path, strlen(elf_path));

    ret = __elf_check(file, entry_offset);
put_file:
    filp_close(file, NULL);
out:
    return ret;
}

static struct list_head *get_elf_list(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &compiler_paths_list;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &assembler_paths_list;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static struct uprobe_consumer *get_uprobe_consumer(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &uprobe_compiler_consumer;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &uprobe_assembler_consumer;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static struct mutex *get_elf_lock(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &compiler_paths_lock;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &assembler_paths_lock;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static int __register_uprobe(unsigned int cmd, struct elf_path *ep, struct uprobe_consumer *uc)
{
    int ret;
    struct path path;
    struct inode *inode;

    ret = elf_check(ep->name, &ep->entry_offset);
    if (ret)
        goto out;

    ret = kern_path(ep->name, LOOKUP_FOLLOW, &path);
    if (ret) {
        pr_err("kernel path failed - %d \n", ret);
        goto out;
    }
    inode = path.dentry->d_inode;

    pr_debug("register uprobe for %s \n", ep->name);
    ret = uprobe_register(inode, ep->entry_offset, uc);
out:
    return ret;
}

static struct elf_path *__get_elf_path(unsigned int cmd, const char *name)
{
    struct elf_path *ep;
    struct list_head *elf_list;

    elf_list = get_elf_list(cmd);
    if (elf_list) {
        list_for_each_entry(ep, elf_list, list) {
            if (strncmp(ep->name, name, PATH_MAX) == 0) {
                return ep;
            }
        }
    }
    return NULL;
}

static int inline __delete_elf_path(unsigned int cmd, const char *name)
{
    struct elf_path *ep;
    int ret;
    struct uprobe_consumer  *uc = get_uprobe_consumer(cmd);

    if (!uc)
        return -ENOTTY;

    ep = __get_elf_path(cmd, name);
    if (ep) {
        ret = __unregister_uprobe(cmd, ep, uc);
        if (ret)
            return ret;
        list_del(&ep->list);
        kfree(ep);
    }
    return 0;
}

static int inline __add_elf_path(unsigned int cmd, const char *name)
{
    struct elf_path *ep;
    int ret;
    struct list_head *elf_list = get_elf_list(cmd);
    struct uprobe_consumer  *uc = get_uprobe_consumer(cmd);

    if (!uc)
        return -ENOTTY;

    if (!elf_list)
        return -ENOTTY;

    ep = kzalloc(sizeof(*ep), GFP_KERNEL);
    if (!ep)
        return -ENOTTY;

    strncpy(ep->name, name, PATH_MAX);
    ep->count = 1;
    ep->entry_offset = 0;

    ret = __register_uprobe(cmd, ep, uc);
    if (ret)
        return ret;

    list_add(&ep->list, elf_list);

    return 0;
}

struct elf_path *get_elf_path(unsigned int cmd, const char *name)
{
    struct mutex *lock;
    struct elf_path *ep = NULL;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        ep = __get_elf_path(cmd, name);
        mutex_unlock(lock);
    }
    return ep;
}

int delete_elf_path(unsigned int cmd, const char *name)
{
    struct mutex *lock;
    struct elf_path *ep = NULL;
    int ret = 0;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        ep = __get_elf_path(cmd, name);
        if (ep) {
            ep->count--;
            if (!ep->count)
                ret = __delete_elf_path(cmd, name);
        }
        mutex_unlock(lock);
    }
    return ret;
}

int add_elf_path(unsigned int cmd, const char *name)
{
    struct mutex *lock;
    struct elf_path *ep = NULL;
    int ret = 0;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        ep = __get_elf_path(cmd, name);
        if (!ep) {
            ret = __add_elf_path(cmd, name);
        } else {
            ep->count++;
        }
        mutex_unlock(lock);
    }
    return ret;
}

int handle_compiler_cmd(unsigned long user_addr, unsigned int cmd)
{
    int ret;
    char *path_buf = NULL;
    const char* real_path;

    path_buf = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!path_buf)
        return -ENOMEM;

    ret = copy_para_from_user(user_addr, path_buf, PATH_MAX);
    if (ret)
        goto out;

    real_path = get_real_path(path_buf, PATH_MAX);
    if (real_path == NULL || IS_ERR(real_path)) {
        pr_err("get real_path failed: %u \n", cmd);
        goto out;
    }

    switch (cmd)
    {
    case UPATCH_REGISTER_COMPILER:
    case UPATCH_REGISTER_ASSEMBLER:
        ret = add_elf_path(cmd, real_path);
        break;

    case UPATCH_UNREGISTER_COMPILER:
    case UPATCH_UNREGISTER_ASSEMBLER:
        ret = delete_elf_path(cmd, real_path);
        break;

    default:
        ret = -ENOTTY;
        break;
    }

out:
    if (path_buf)
        kfree(path_buf);
    return ret;
}

int __init compiler_hack_init(void)
{
    int ret;

    ret = compiler_step_init();
    if (ret) {
        pr_err("compiler step register failed - %d \n", ret);
        goto out;
    }

out:
    return ret;
}

void clear_compiler_path(void)
{
    struct elf_path *ep, *tmp;

    mutex_lock(&compiler_paths_lock);
    list_for_each_entry_safe(ep, tmp, &compiler_paths_list, list) {
        __delete_elf_path(UPATCH_UNREGISTER_COMPILER, ep->name);
    }
    mutex_unlock(&compiler_paths_lock);
}

void clear_assembler_path(void)
{
    struct elf_path *ep, *tmp;
    mutex_lock(&assembler_paths_lock);
    list_for_each_entry_safe(ep, tmp, &assembler_paths_list, list) {
        __delete_elf_path(UPATCH_UNREGISTER_ASSEMBLER, ep->name);
    }
    mutex_unlock(&assembler_paths_lock);
}

void __exit compiler_hack_exit(void)
{
    clear_compiler_path();
    clear_assembler_path();
    clear_compiler_step();
}
