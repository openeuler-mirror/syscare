// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/printk.h>
#include <linux/uprobes.h>
#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/slab.h>
#include <linux/namei.h>
#include <linux/file.h>
#include <linux/elf.h>
#include <linux/mm.h>
#include <linux/fs.h>

#include "common.h"
#include "patch.h"
#include "patch-uprobe.h"
#include "upatch-ioctl.h"
#include "upatch-patch.h"

static DEFINE_MUTEX(upatch_entity_lock);
static LIST_HEAD(upatch_entity_list);

static struct upatch_entity *__get_upatch_entity(struct inode *uinode)
{
    struct upatch_entity *entity;
    list_for_each_entry(entity, &upatch_entity_list, list)
        /* binary / patch all can be the master key */
        if (entity->binary == uinode || entity->patch == uinode)
            return entity;
    return NULL;
}

struct upatch_entity *upatch_entity_get(struct inode *uinode)
{
    struct upatch_entity *entity;
    mutex_lock(&upatch_entity_lock);
    entity = __get_upatch_entity(uinode);
    mutex_unlock(&upatch_entity_lock);
    return entity;
}

static int __insert_upatch_entity(struct inode *binary, struct inode *patch)
{
    struct upatch_entity *entity;

    if (!binary || !patch)
        return -EINVAL;

    if (__get_upatch_entity(binary))
        return 0;

    entity = kzalloc(sizeof(*entity), GFP_KERNEL);
    if (!entity)
        return -ENOMEM;

    entity->binary = binary;
    entity->patch = patch;
    list_add(&entity->list, &upatch_entity_list);
    mutex_init(&entity->offset_list_lock);
    INIT_LIST_HEAD(&entity->offset_list);
    mutex_init(&entity->module_list_lock);
    INIT_LIST_HEAD(&entity->module_list);
    return 0;
}

static int insert_upatch_entity(struct inode *binary, struct inode *patch)
{
    int ret;
    mutex_lock(&upatch_entity_lock);
    ret = __insert_upatch_entity(binary, patch);
    mutex_unlock(&upatch_entity_lock);
    return ret;
}

/* no check for offset */
static int __insert_uprobe_offset(struct upatch_entity *entity, loff_t offset)
{
    struct uprobe_offset *uo;

    uo = kzalloc(sizeof(*uo), GFP_KERNEL);
    if (!uo)
        return -ENOMEM;

    uo->offset = offset;
    list_add(&uo->list, &entity->offset_list);
    return 0;
}

static int insert_uprobe_offset(struct upatch_entity *entity, loff_t offset)
{
    int ret;
    mutex_lock(&entity->offset_list_lock);
    ret = __insert_uprobe_offset(entity, offset);
    mutex_unlock(&entity->offset_list_lock);
    return ret;
}

static bool check_upatch(Elf_Ehdr *ehdr)
{
    if (memcmp(ehdr->e_ident, ELFMAG, SELFMAG) != 0)
        return false;

    if (ehdr->e_type != ET_REL)
        return false;

    if (ehdr->e_shentsize != sizeof(Elf_Shdr))
        return false;

    return true;
}

static int do_module_load(struct upatch_entity *entity, struct file *binary_file,
    struct upatch_load_info *info)
{
    int ret;
    struct file *patch_file = NULL;

    patch_file = d_open_inode(entity->patch);
    if (!patch_file || IS_ERR(patch_file)) {
        pr_err("open patch inode failed \n");
        ret = -ENOEXEC;
        goto out;
    }

    ret = upatch_load(binary_file, patch_file, info);
out:
    if (patch_file && !IS_ERR(patch_file))
        fput(patch_file);
    return ret;
}

static int do_module_active(struct upatch_module *module, struct pt_regs *regs)
{
    struct upatch_patch_func __user *upatch_funs;
    unsigned int nums;
    unsigned int i;
    unsigned long pc;
    bool set_pc = false;

    nums = module->num_upatch_funcs;
    upatch_funs = kzalloc(sizeof(struct upatch_patch_func) * module->num_upatch_funcs,
        GFP_KERNEL);
    if (!upatch_funs) {
        pr_err("malloc upatch funcs failed \n");
        return 0;
    }

    if (copy_from_user(upatch_funs, module->upatch_funs,
        sizeof(struct upatch_patch_func) * module->num_upatch_funcs)) {
        pr_err("copy from user failed \n");
        kfree(upatch_funs);
        return 0;
    }

    pc = instruction_pointer(regs);
    for (i = 0; i < nums; i ++) {
        if (pc == upatch_funs[i].old_addr) {
            pc = upatch_funs[i].new_addr;
            instruction_pointer_set(regs, pc);
            pr_debug("jmp to patched address 0x%lx \n", pc);
            set_pc = true;
            break;
        }
    }

    if (!set_pc) {
        pr_err("unable to activate the patch, no address found \n");
        return 0;
    }

    return UPROBE_ALTER_PC;
}

static int do_module_create(struct upatch_entity *entity)
{
    struct upatch_module *mod =
        upatch_module_new(task_pid_nr(current));
    if (!mod)
        return -ENOMEM;
    mod->set_state = entity->entity_status;
    return upatch_module_insert(entity, mod);
}

/* TODO: check modules that doesn't live anymore */
static int do_module_remove(struct upatch_entity *entity,
    struct upatch_module *module)
{
    module->real_state = UPATCH_STATE_REMOVED;
    upatch_module_deallocate(module);
    /* TODO: currently, remove is only a mark flag */
    // upatch_module_remove(entity, module);
    // upatch_entity_try_remove(entity);
    return 0;
}

static int uprobe_patch_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    unsigned long pc;
    struct upatch_load_info info;
    struct upatch_entity *entity;
    struct file *binary_file = NULL;
    bool need_resolve = false;
    bool need_active = false;
    struct upatch_module *upatch_mod = NULL;
    pid_t pid = task_pid_nr(current);
    int ret = 0;

    pc = instruction_pointer(regs);
    pr_debug("patch handler works in 0x%lx \n", pc);

    memset(&info, 0, sizeof(info));

    binary_file = get_binary_file_from_addr(current, pc);
    if (!binary_file) {
        pr_err("no exe file found for upatch \n");
        goto out;
    }

    entity = upatch_entity_get(file_inode(binary_file));
    if (!entity) {
        pr_err("How can you be here without attach ? \n");
        goto out;
    }

    /* TODO: sync between different threads */
    upatch_mod = upatch_module_get(entity, pid);
    if (!upatch_mod && do_module_create(entity)) {
        pr_err("create module failed \n");
        goto out;
    }

    upatch_mod = upatch_module_get(entity, pid);
    if (!upatch_mod) {
        pr_err("no patch module found \n");
        goto out;
    }

    pr_debug("status from %d to %d \n", upatch_mod->real_state, upatch_mod->set_state);

    info.mod = upatch_mod;
    switch (upatch_mod->set_state)
    {
    case UPATCH_STATE_ACTIVED:
        if (upatch_mod->real_state < UPATCH_STATE_ACTIVED)
            need_active = true;
        fallthrough;
    case UPATCH_STATE_RESOLVED:
    case UPATCH_STATE_ATTACHED:
        if (upatch_mod->real_state < UPATCH_STATE_RESOLVED)
            need_resolve = true;
        break;
    case UPATCH_STATE_REMOVED:
        do_module_remove(entity, upatch_mod);
        goto out;
    default:
        pr_err("invalid upatch status \n");
        break;
    }

    if (need_resolve && do_module_load(entity, binary_file, &info)) {
        pr_err("load patch failed \n");
        goto out;
    }

    if (need_active) {
        ret = do_module_active(upatch_mod, regs);
        goto out;
    }

out:
    if (binary_file)
        fput(binary_file);
    return ret;
}

static struct uprobe_consumer patch_consumber = {
    .handler = uprobe_patch_handler,
    .ret_handler = NULL,
    .filter = uprobe_default_filter,
};

/*
 * shoule we check if it is the entry of the function ?
 */
static int register_patch_uprobe(struct file *binary_file, loff_t offset)
{
    int ret;
    struct inode *inode;
    struct upatch_entity *entity;

    inode = file_inode(binary_file);
    entity = upatch_entity_get(inode);
    if (!entity)
        return -ENOENT;

    ret = insert_uprobe_offset(entity, offset);
    if (ret)
        return ret;

    ret = uprobe_register(inode, offset, &patch_consumber);
    if (ret) {
        pr_err("patch uprobe register failed - %d \n", ret);
        goto out;
    }

    pr_debug("register patch uprobe at 0x%llx\n", offset);

    ret = 0;
out:
    if (binary_file != NULL)
        fput(binary_file);
    return ret;
}

void upatch_entity_try_remove(struct upatch_entity *entity)
{
    bool has_mods = false;
    struct uprobe_offset *uprobe_offset, *tmp;

    if (!entity)
        return;

    mutex_lock(&entity->module_list_lock);
    if (!list_empty(&entity->module_list))
        has_mods = true;
    mutex_unlock(&entity->module_list_lock);

    if (has_mods) {
        pr_debug("entity still has modules \n");
        return;
    }

    pr_debug("start to remove entity \n");
    mutex_lock(&entity->offset_list_lock);
    list_for_each_entry_safe(uprobe_offset, tmp, &entity->offset_list, list) {
        pr_debug("unregister for offset 0x%llx\n", uprobe_offset->offset);
        uprobe_unregister(entity->binary, uprobe_offset->offset, &patch_consumber);
        list_del(&uprobe_offset->list);
        kfree(uprobe_offset);
    }
    mutex_unlock(&entity->offset_list_lock);

    mutex_lock(&upatch_entity_lock);
    list_del(&entity->list);
    kfree(entity);
    mutex_unlock(&upatch_entity_lock);
}

/*
 * find valid entry points for applying patch.
 * no matter which point hits, it will active the whole patch.
 */
static int handle_upatch_funcs(struct file *binary_file, struct file *patch_file,
    Elf_Shdr *upatch_shdr)
{
    int buf_len;
    int ret;
    int index;
    loff_t offset;
    unsigned long old_addr;
    elf_addr_t min_addr;
    struct upatch_patch_func *upatch_funs = NULL;

    /* TODO: sh_entsize becomes 0 after ld -r, skip this problem now */
    upatch_shdr->sh_entsize = sizeof(struct upatch_patch_func);

    if (upatch_shdr->sh_entsize != sizeof(struct upatch_patch_func)) {
        pr_err("invalid section size for upatch func section %llu - %lu \n",
            upatch_shdr->sh_entsize, sizeof(struct upatch_patch_func));
        return -EINVAL;
    }

    buf_len = upatch_shdr->sh_size;
    upatch_funs = kmalloc(buf_len, GFP_KERNEL);
    if (!upatch_funs)
        return -ENOMEM;

    offset = upatch_shdr->sh_offset;
    ret = kernel_read(patch_file, upatch_funs, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read upatch funcs failed- %d \n", ret);
        ret = -EINVAL;
        goto out;
    }

    min_addr = calculate_load_address(binary_file, false);
    if (min_addr == -1) {
        ret = -EINVAL;
        goto out;
    }

    /* TODO: if failed, we need clean this entity */
    /* TODO: check if other patch has taken effect */
    /* before uprobe works, we must set upatch entity first */
    ret = insert_upatch_entity(file_inode(binary_file), file_inode(patch_file));
    if (ret) {
        pr_err("insert upatch entity failed - %d \n", ret);
        goto out;
    }

    pr_debug("load address is 0x%llx \n", min_addr);
    for (index = 0; index < upatch_shdr->sh_size / upatch_shdr->sh_entsize; index ++) {
        old_addr = upatch_funs[index].old_addr;
        ret = register_patch_uprobe(binary_file, old_addr - min_addr);
        if (ret)
            goto out;
    }

out:
    if (upatch_funs)
        kfree(upatch_funs);
    return 0;
}

/*
 * TODO:
 * 1. handle SHN_LORESERVE
 * 2. check elf arch and abi
 */
int upatch_attach(const char *binary, const char *patch)
{
    int ret = 0;
    int index;
    loff_t offset;
    int buf_len;
    Elf_Ehdr ehdr;
    Elf_Shdr *eshdrs = NULL;
    char *shstr = NULL;
    char *name = NULL;
    struct file *binary_file = NULL;
    struct file *patch_file = NULL;
    struct upatch_entity *entity = NULL;

    binary_file = filp_open(binary, O_RDONLY, 0);
    if (IS_ERR(binary_file)) {
        ret = PTR_ERR(binary_file);
        pr_err("open binary failed - %d \n", ret);
        goto out;
    }

    /* TODO: update status if found */
    entity = upatch_entity_get(file_inode(binary_file));
    if (entity) {
        ret = 0;
        upatch_update_entity_status(entity, UPATCH_STATE_RESOLVED);
        goto out;
    }

    patch_file = filp_open(patch, O_RDONLY, 0);
    if (IS_ERR(patch_file)) {
        ret = PTR_ERR(patch_file);
        pr_err("open patch failed - %d \n", ret);
        goto out;
    }

    offset = 0;
    buf_len = sizeof(Elf_Ehdr);
    ret = kernel_read(patch_file, &ehdr, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read patch header failed - %d \n", ret);
        ret = -EINVAL;
        goto out;
    }

    if (!check_upatch(&ehdr)) {
        pr_err("check upatch failed \n");
        ret = -EINVAL;
        goto out;
    }

    pr_debug("patch has %d sections at %lld \n", ehdr.e_shnum, ehdr.e_shoff);
    /* read section header table */
    buf_len = sizeof(Elf_Shdr) * ehdr.e_shnum;
    eshdrs = kmalloc(buf_len, GFP_KERNEL);
    if (!eshdrs) {
        ret = -ENOMEM;
        goto out;
    }

    offset = ehdr.e_shoff;
    ret = kernel_read(patch_file, eshdrs, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read patch section header failed - %d \n", ret);
        ret = -EINVAL;
        goto out;
    }

    pr_debug("section string table index %d at %lld \n", ehdr.e_shstrndx, eshdrs[ehdr.e_shstrndx].sh_offset);

    /* read string table for section header table */
    buf_len = eshdrs[ehdr.e_shstrndx].sh_size;
    shstr = kmalloc(buf_len, GFP_KERNEL);
    if (!shstr) {
        ret = -ENOMEM;
        goto out;
    }

    offset = eshdrs[ehdr.e_shstrndx].sh_offset;
    ret = kernel_read(patch_file, shstr, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read string table failed - %d \n", ret);
        ret = -EINVAL;
        goto out;
    }

    pr_debug("total section number : %d \n", ehdr.e_shnum);
    for (index = 0; index < ehdr.e_shnum; index ++) {
        if (eshdrs[index].sh_name == 0)
            continue;

        name = shstr + eshdrs[index].sh_name;
        if (strncmp(name, ".upatch.funcs", 13) != 0)
            continue;

        pr_debug("upatch section index is %d \n", index);
        ret = handle_upatch_funcs(binary_file, patch_file, &eshdrs[index]);
        if (ret)
            pr_err("handle upatch failed - %d \n", ret);
        goto out;
    }

    ret = 0;
out:
    if (shstr)
        kfree(shstr);
    if (eshdrs)
        kfree(eshdrs);
    if (patch_file && !IS_ERR(patch_file))
        fput(patch_file);
    if (binary_file && !IS_ERR(binary_file))
        fput(binary_file);
    return ret;
}

void upatch_update_entity_status(struct upatch_entity *entity,
    enum upatch_module_state status)
{
    struct upatch_module *um;
    mutex_lock(&entity->module_list_lock);
    entity->entity_status = status;
    list_for_each_entry(um, &entity->module_list, list) {
        um->set_state = status;
    }
    mutex_unlock(&entity->module_list_lock);
}
