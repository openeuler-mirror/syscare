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
#include <linux/spinlock.h>
#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/slab.h>
#include <linux/namei.h>
#include <linux/file.h>
#include <linux/elf.h>
#include <linux/mm.h>
#include <linux/fs.h>
#include <linux/vmalloc.h>

#include "common.h"
#include "patch.h"
#include "patch-uprobe.h"
#include "upatch-ioctl.h"
#include "upatch-patch.h"

static DEFINE_MUTEX(upatch_entity_lock);
static LIST_HEAD(upatch_entity_list);

/* lock for all patch entity, since we need free its memory */
static DEFINE_SPINLOCK(patch_entity_lock);

static struct upatch_entity *__get_upatch_entity(struct inode *uinode)
{
    struct upatch_entity *entity;
    list_for_each_entry(entity, &upatch_entity_list, list)
        /* binary / patch all can be the master key */
        if (entity->binary == uinode || entity->set_patch == uinode)
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

void __remove_patch_entity(struct patch_entity *patch_entity)
{
    if (patch_entity->patch_buff)
        vm_munmap((unsigned long)patch_entity->patch_buff, patch_entity->patch_size);
    patch_entity->patch_buff = NULL;
    patch_entity->patch_size = 0;
    kfree(patch_entity);
}

/* After put, holder should never use this memory */
void upatch_put_patch_entity(struct patch_entity *patch_entity)
{
    if (patch_entity == NULL)
        return;

    spin_lock(&patch_entity_lock);
    patch_entity->ref --;
    if (patch_entity->ref == 0)
        __remove_patch_entity(patch_entity);
    spin_unlock(&patch_entity_lock);
}

void upatch_get_patch_entity(struct patch_entity *patch_entity)
{
    if (patch_entity == NULL)
        return;

    spin_lock(&patch_entity_lock);
    patch_entity->ref ++;
    spin_unlock(&patch_entity_lock);
}

int upatch_init_patch_entity(struct patch_entity *patch_entity, struct file *patch)
{
    int ret;

    if (patch == NULL)
        return 0;

    patch_entity->ref = 1;
    patch_entity->patch_size = i_size_read(file_inode(patch));
    patch_entity->patch_buff = vmalloc(patch_entity->patch_size);
    if (!patch_entity->patch_buff)
        return -ENOMEM;

    ret = kernel_read(patch, patch_entity->patch_buff, patch_entity->patch_size, 0);
    if (ret != patch_entity->patch_size) {
        pr_err("read patch file for entity failed. \n");
        vm_munmap((unsigned long)patch_entity->patch_buff, patch_entity->patch_size);
        return -ENOEXEC;
    }
    return 0;
}

static int __insert_upatch_entity(struct file *binary, struct file *patch)
{
    struct upatch_entity *entity = NULL;
    int err;

    entity = kzalloc(sizeof(*entity), GFP_KERNEL);
    if (!entity)
        return -ENOMEM;

    entity->patch_entity = kzalloc(sizeof(*entity->patch_entity), GFP_KERNEL);
    if (!entity->patch_entity) {
        err = -ENOMEM;
        goto err_out;
    }

    err = upatch_init_patch_entity(entity->patch_entity, patch);
    if (err) {
        err = -ENOMEM;
        goto err_out;
    }

    entity->set_patch = file_inode(patch);
    entity->set_status = UPATCH_STATE_ATTACHED;
    entity->binary = file_inode(binary);

    mutex_init(&entity->entity_status_lock);
    INIT_LIST_HEAD(&entity->offset_list);
    INIT_LIST_HEAD(&entity->module_list);

    /* when everything is ok, add it to the list */
    list_add(&entity->list, &upatch_entity_list);
    return 0;

err_out:
    if (entity && entity->patch_entity)
        __remove_patch_entity(entity->patch_entity);
    if (entity && entity->patch_entity)
        kfree(entity->patch_entity);
    if (entity)
        kfree(entity);
    return err;
}

static int __update_upatch_entity(struct upatch_entity *entity, struct file *patch)
{
    int ret;

    if (patch == NULL)
        return -EINVAL;

    /* upatch_entity_lock > entity_status_lock > patch_entity_lock */
    mutex_lock(&entity->entity_status_lock);

    upatch_put_patch_entity(entity->patch_entity);
    entity->patch_entity = NULL;

    entity->patch_entity = kzalloc(sizeof(*entity->patch_entity), GFP_KERNEL);
    ret = upatch_init_patch_entity(entity->patch_entity, patch);
    if (ret != 0) {
        kfree(entity->patch_entity);
        goto out;
    }

    entity->set_patch = file_inode(patch);
    entity->set_status = UPATCH_STATE_ATTACHED;
out:
    mutex_unlock(&entity->entity_status_lock);
    return ret;
}

static int update_upatch_entity(struct file *binary, struct file *patch)
{
    int ret;
    struct upatch_entity *entity;

    if (!binary || !patch)
        return -EINVAL;

    mutex_lock(&upatch_entity_lock);
    entity = __get_upatch_entity(file_inode(binary));
    if (entity)
        ret = __update_upatch_entity(entity, patch);
    else
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
    int ret = 0;
    struct uprobe_offset *uo;

    mutex_lock(&entity->entity_status_lock);
    list_for_each_entry(uo, &entity->offset_list, list) {
        if (uo->offset == offset) {
            ret = -EEXIST;
            break;
        }
    }

    if (ret == 0)
        ret = __insert_uprobe_offset(entity, offset);
    mutex_unlock(&entity->entity_status_lock);
    return ret;
}

static int read_build_id(struct file *file, struct elf_build_id *build_id)
{
    int ret = 0;
    int index;
    loff_t offset;
    int buf_len;
    Elf_Ehdr ehdr;
    Elf_Shdr *eshdrs = NULL;
    char* shstr = NULL;
    char* name = NULL;

    offset = 0;
    buf_len = sizeof(Elf_Ehdr);
    ret = kernel_read(file, &ehdr, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read file failed - %d \n", ret);
        ret = -EINVAL;
        goto out;
    }

    buf_len = sizeof(Elf_Shdr) * ehdr.e_shnum;
    eshdrs = kmalloc(buf_len, GFP_KERNEL);
    if (!eshdrs) {
        ret = -ENOMEM;
        goto out;
    }

    offset = ehdr.e_shoff;
    ret = kernel_read(file, eshdrs, buf_len, &offset);
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
    ret = kernel_read(file, shstr, buf_len, &offset);
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
        if (!strcmp(name, ".note.gnu.build-id") != 0)
            break;
    }

    if (index == ehdr.e_shnum) {
        ret = -EINVAL;
        goto out;
    }

    offset = eshdrs[index].sh_offset;
    buf_len = sizeof(build_id->head);
    if (buf_len >= eshdrs[index].sh_size) {
        pr_err(".note.gnu.build-id section is failed \n");
        ret = -EINVAL;
        goto out;
    }

    ret = kernel_read(file, &build_id->head, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read .note.gnu.build-id failed - %d \n", ret);
        ret = -EINVAL;
        goto out;
    }

    buf_len = build_id->head.nhdr.n_descsz;
    offset = eshdrs[index].sh_offset + sizeof(build_id->head);
    build_id->id = kmalloc(buf_len, GFP_KERNEL);
    if (!build_id->id) {
        ret = -ENOMEM;
        goto out;
    }

    ret = kernel_read(file, build_id->id, buf_len, &offset);
    if (ret != buf_len) {
        pr_err("read .build-id failed - %d \n", ret);
        ret = -EINVAL;
        goto out;
    }
    ret = 0;
out:
    if (shstr)
        kfree(shstr);
    if (eshdrs)
        kfree(eshdrs);
    return ret;
}

static int check_upatch(Elf_Ehdr *ehdr, struct file *patch_file, struct file *binary_file)
{
    int ret = -EINVAL;
    struct elf_build_id patch_id;
    struct elf_build_id binary_id;

    memset(&patch_id, 0, sizeof(patch_id));
    memset(&binary_id, 0, sizeof(binary_id));

    if (memcmp(ehdr->e_ident, ELFMAG, SELFMAG) != 0)
        goto out;

    if (ehdr->e_type != ET_REL)
        goto out;

    if (ehdr->e_shentsize != sizeof(Elf_Shdr))
        goto out;

    ret = read_build_id(patch_file, &patch_id);
    if (ret) {
        pr_err("read patch's build id failed - %d \n", ret);
        goto out;
    }

    ret = read_build_id(binary_file, &binary_id);
    if (ret) {
        pr_err("read binary's build id failed - %d \n", ret);
        goto out;
    }

    if (memcmp(patch_id.id, binary_id.id, binary_id.head.nhdr.n_descsz) != 0) {
        pr_err("build id is different.\n");
        goto out;
    }

    ret = 0;
out:
    if (patch_id.id)
        kfree(patch_id.id);
    if (binary_id.id)
        kfree(binary_id.id);
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
        if (pc == upatch_funs[i].old_addr + module->load_bias) {
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

/* TODO: check modules that doesn't live anymore */
static int do_module_remove(struct upatch_entity *entity,
    struct upatch_module *module)
{
    if (module->real_state == UPATCH_STATE_REMOVED)
        return 0;

    module->real_state = UPATCH_STATE_REMOVED;
    module->real_patch = NULL;
    /* TODO: when remove, check function stack!!!!! */
    upatch_module_deallocate(module);

    /* TODO: currently, remove is only a mark flag */
    // upatch_module_remove(entity, module);
    // upatch_entity_try_remove(entity);

    return 0;
}

static int uprobe_patch_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    unsigned long pc;
    int ret = 0;

    struct upatch_load_info info;
    struct upatch_entity *entity = NULL;
    struct upatch_module *upatch_mod = NULL;
    struct file *binary_file = NULL;

    bool need_resolve = false;
    bool need_active = false;

    enum upatch_module_state set_status;
    struct inode *set_patch;
    struct patch_entity *patch_entity = NULL;

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
        pr_err("No entity found in patch handler \n");
        goto out;
    }

    mutex_lock(&entity->entity_status_lock);
    set_status = entity->set_status;
    set_patch = entity->set_patch;
    patch_entity = entity->patch_entity;
    upatch_get_patch_entity(patch_entity);
    mutex_unlock(&entity->entity_status_lock);

    upatch_mod = upatch_module_get_or_create(entity, task_pid_nr(current));
    if (!upatch_mod) {
        pr_err("found module failed \n");
        goto out;
    }

    /* Now, all actions happens within module lock */
    mutex_lock(&upatch_mod->module_status_lock);

    /* if not set_patch, clear exist patch */
    if (upatch_mod->real_patch != set_patch)
        do_module_remove(entity, upatch_mod);
    
    pr_debug("status from %d to %d \n", upatch_mod->real_state, set_status);

    info.mod = upatch_mod;
    switch (set_status)
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
        goto out_unlock;
    default:
        pr_err("invalid upatch status \n");
        break;
    }

    /* we can be sure module->real_patch == set_patch/NULL  */
    if (need_resolve && upatch_load(binary_file, set_patch, patch_entity, &info)) {
        pr_err("load patch failed \n");
        goto out_unlock;
    }

    if (need_active) {
        ret = do_module_active(upatch_mod, regs);
        goto out_unlock;
    }

out_unlock:
    mutex_unlock(&upatch_mod->module_status_lock);
out:
    upatch_put_patch_entity(patch_entity);
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
        return ret;
    }

    pr_info("upatch register patch uprobe at 0x%llx\n", offset);
    return 0;
}

void upatch_entity_try_remove(struct upatch_entity *entity)
{
    bool has_mods = false;
    struct uprobe_offset *uprobe_offset, *tmp;

    if (!entity)
        return;

    mutex_lock(&entity->entity_status_lock);
    if (!list_empty(&entity->module_list))
        has_mods = true;
    mutex_unlock(&entity->entity_status_lock);

    if (has_mods) {
        pr_debug("entity still has modules \n");
        return;
    }

    pr_debug("start to remove entity \n");

    mutex_lock(&upatch_entity_lock);
    mutex_lock(&entity->entity_status_lock);
    /* unregister it in the handler will lead to deadlock */
    list_for_each_entry_safe(uprobe_offset, tmp, &entity->offset_list, list) {
        uprobe_unregister(entity->binary, uprobe_offset->offset, &patch_consumber);
        list_del(&uprobe_offset->list);
        kfree(uprobe_offset);
    }
    mutex_unlock(&entity->entity_status_lock);
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
    ret = update_upatch_entity(binary_file, patch_file);
    if (ret) {
        pr_err("update upatch entity failed - %d \n", ret);
        goto out;
    }

    pr_debug("load address is 0x%llx \n", min_addr);
    for (index = 0; index < upatch_shdr->sh_size / upatch_shdr->sh_entsize; index ++) {
        old_addr = upatch_funs[index].old_addr;
        ret = register_patch_uprobe(binary_file, old_addr - min_addr);
        if (ret && ret != -EEXIST)
            goto out;
    }

out:
    if (upatch_funs)
        kfree(upatch_funs);
    return 0;
}

int check_entity(struct upatch_entity *entity)
{
    int ret = 0;
    mutex_lock(&entity->entity_status_lock);
    if (entity->set_patch != NULL || entity->set_status != UPATCH_STATE_REMOVED)
        ret = -EPERM;
    mutex_unlock(&entity->entity_status_lock);
    return ret;
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

    patch_file = filp_open(patch, O_RDONLY, 0);
    if (IS_ERR(patch_file)) {
        ret = PTR_ERR(patch_file);
        pr_err("open patch failed - %d \n", ret);
        goto out;
    }

    entity = upatch_entity_get(file_inode(binary_file));
    /* not first time to handle this binary  */
    if (entity && check_entity(entity)) {
        pr_err("need to remove exist patch first \n");
        ret = -EPERM;
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

    ret = check_upatch(&ehdr, patch_file, binary_file);
    if (ret) {
        pr_err("check upatch failed - %d \n", ret);
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
        filp_close(patch_file, NULL);
    if (binary_file && !IS_ERR(binary_file))
        filp_close(binary_file, NULL);
    return ret;
}
