// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
 * Copyright (C) 2024 Huawei Technologies Co., Ltd.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#include <ctype.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <libgen.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/sysmacros.h>
#include <sys/wait.h>
#include <unistd.h>

#include "list.h"
#include "log.h"
#include "upatch-common.h"
#include "upatch-elf.h"
#include "upatch-process.h"
#include "upatch-ptrace.h"

static const int MAX_ATTACH_ATTEMPTS = 3;

/*
 * Locks process by opening /proc/<pid>/maps
 * This ensures that task_struct will not be
 * deleted in the kernel while we are working with
 * the process
 */
static int lock_process(int pid)
{
    int fd;
    char path[128];

    log_debug("Locking PID %d...", pid);
    (void) snprintf(path, sizeof(path), "/proc/%d/maps", pid);

    fd = open(path, O_RDONLY);
    if (fd < 0) {
        log_error("Failed to open file '%s'\n", path);
        return -1;
    }
    log_debug("OK\n");

    return fd;
}

static void unlock_process(int fdmaps)
{
    int errsv = errno;
    close(fdmaps);
    errno = errsv;
}

// TODO: get addr_space
static int upatch_coroutines_init(struct upatch_process *proc)
{
    INIT_LIST_HEAD(&proc->coro.coros);

    return 0;
}

static int process_get_comm(struct upatch_process *proc)
{
    char path[128];
    char realpath[PATH_MAX];
    char *bn;
    char *c;
    ssize_t ret;

    (void) snprintf(path, sizeof(path), "/proc/%d/exe", proc->pid);
    log_debug("Reading from '%s'...", path);

    ret = readlink(path, realpath, sizeof(realpath));
    if (ret < 0) {
        return -1;
    }

    realpath[ret] = '\0';
    bn = basename(realpath);
    strncpy(path, bn, sizeof(path) - 1);
    if ((c = strstr(path, " (deleted)"))) {
        *c = '\0';
    }

    proc->comm[sizeof(proc->comm) - 1] = '\0';
    memcpy(proc->comm, path, sizeof(proc->comm) - 1);
    // TODO: the comm is ldxxx
    log_debug("OK\n");

    return 0;
}

int upatch_process_init(struct upatch_process *proc, int pid)
{
    int fdmaps;

    fdmaps = lock_process(pid);
    if (fdmaps < 0) {
        goto out_err;
    }

    memset(proc, 0, sizeof(*proc));

    proc->pid = pid;
    proc->fdmaps = fdmaps;
    proc->memfd = -1;

    INIT_LIST_HEAD(&proc->ptrace.pctxs);
    INIT_LIST_HEAD(&proc->objs);
    INIT_LIST_HEAD(&proc->vma_holes);
    proc->num_objs = 0;

    if (upatch_coroutines_init(proc)) {
        goto out_unlock;
    }

    if (process_get_comm(proc)) {
        goto out_unlock;
    }

    return 0;

out_unlock:
    unlock_process(fdmaps);
out_err:
    return -1;
}

static void upatch_object_memfree(struct object_file *obj)
{
    struct object_patch *opatch;
    struct object_patch *opatch_safe;
    struct obj_vm_area *ovma;
    struct obj_vm_area *ovma_safe;

    if (obj->name) {
        free(obj->name);
    }

    list_for_each_entry_safe(opatch, opatch_safe, &obj->applied_patch, list) {
        if (opatch->uinfo) {
            if (opatch->uinfo->funcs) {
                free(opatch->uinfo->funcs);
            }
            free(opatch->uinfo);
        }
        free(opatch);
    }

    list_for_each_entry_safe(ovma, ovma_safe, &obj->vma, list) {
        free(ovma);
    }
}

static void upatch_process_memfree(struct upatch_process *proc)
{
    struct upatch_ptrace_ctx *p;
    struct upatch_ptrace_ctx *p_safe;
    struct object_file *obj;
    struct object_file *obj_safe;
    struct vm_hole *hole;
    struct vm_hole *hole_safe;

    list_for_each_entry_safe(p, p_safe, &proc->ptrace.pctxs, list) {
        free(p);
    }

    list_for_each_entry_safe(hole, hole_safe, &proc->vma_holes, list) {
        free(hole);
    }

    list_for_each_entry_safe(obj, obj_safe, &proc->objs, list) {
        upatch_object_memfree(obj);
        free(obj);
    }
}

void upatch_process_destroy(struct upatch_process *proc)
{
    unlock_process(proc->fdmaps);
    upatch_process_memfree(proc);
}

static void process_print_cmdline(struct upatch_process *proc)
{
    char buf[PATH_MAX];
    ssize_t i;
    ssize_t rv;

    (void) snprintf(buf, PATH_MAX, "/proc/%d/cmdline", proc->pid);

    int fd = open(buf, O_RDONLY);
    if (fd == -1) {
        log_error("Failed to open file '%s'\n", buf);
        return;
    }

    while (1) {
        rv = read(fd, buf, sizeof(buf));
        if (rv == -1) {
            if (errno == EINTR) {
                continue;
            }
            log_error("Failed to read cmdline\n");
            goto err_close;
        }

        if (rv == 0) {
            break;
        }

        for (i = 0; i < rv; i++) {
            if (isprint(buf[i])) {
                log_debug("%c", buf[i]);
            } else {
                log_debug(" ");
            }
        }
    }

err_close:
    close(fd);
}

void upatch_process_print_short(struct upatch_process *proc)
{
    log_debug("process %d, cmdline: ", proc->pid);
    process_print_cmdline(proc);
    log_debug("\n");
}

int upatch_process_mem_open(struct upatch_process *proc, int mode)
{
    char path[PATH_MAX];

    if (proc->memfd >= 0) {
        close(proc->memfd);
    }

    (void) snprintf(path, sizeof(path), "/proc/%d/mem", proc->pid);
    proc->memfd = open(path, mode == MEM_WRITE ? O_RDWR : O_RDONLY);
    if (proc->memfd < 0) {
        log_error("Failed to open file '%s'\n", path);
        return -1;
    }

    return 0;
}

static unsigned int perms2prot(const char *perms)
{
    unsigned int prot = 0;

    if (perms[0] == 'r') {
        prot |= PROT_READ;
    }
    if (perms[1] == 'w') {
        prot |= PROT_WRITE;
    }
    if (perms[2] == 'x') {
        prot |= PROT_EXEC;
    }
    /* Ignore 'p'/'s' flag, we don't need it */
    return prot;
}

static struct vm_hole *process_add_vm_hole(struct upatch_process *proc,
    unsigned long start, unsigned long end)
{
    struct vm_hole *hole = malloc(sizeof(*hole));
    if (hole == NULL) {
        return NULL;
    }

    hole->start = start;
    hole->end = end;
    hole->len = end - start;
    list_init(&hole->list);

    list_add(&hole->list, &proc->vma_holes);
    return hole;
}

static int process_get_object_type(struct upatch_process *proc,
    struct vm_area *vma, char *name, unsigned char *buf, size_t bufsize)
{
    int ret;
    int type = OBJECT_UNKNOWN;

    ret = upatch_process_mem_read(proc, vma->start, buf, bufsize);
    if (ret < 0) {
        return -1;
    }

    if (vma->prot == PROT_READ &&
        !strncmp(name, "[anonymous]", strlen("[anonymous]")) &&
        !memcmp(buf, UPATCH_HEADER, UPATCH_HEADER_LEN)) {
        type = OBJECT_UPATCH;
    } else if (!memcmp(buf, ELFMAG, SELFMAG)) {
        type = OBJECT_ELF;
    } else {
        type = OBJECT_UNKNOWN;
    }

    return type;
}

static int vm_area_same(struct vm_area *a, struct vm_area *b)
{
    return ((a->start == b->start) &&
        (a->end == b->end) &&
        (a->prot == b->prot));
}

static int object_add_vm_area(struct object_file *o, struct vm_area *vma,
    struct vm_hole *hole)
{
    struct obj_vm_area *ovma;

    if (o->prev_hole == NULL) {
        o->prev_hole = hole;
    }

    list_for_each_entry(ovma, &o->vma, list) {
        if (vm_area_same(vma, &ovma->inmem)) {
            return 0;
        }
    }

    ovma = malloc(sizeof(*ovma));
    if (!ovma) {
        return -1;
    }

    memset(ovma, 0, sizeof(*ovma));
    ovma->inmem = *vma;

    list_add(&ovma->list, &o->vma);
    return 0;
}

static struct object_file *process_new_object(struct upatch_process *proc,
    dev_t dev, ino_t inode, const char *name,
    struct vm_area *vma, struct vm_hole *hole)
{
    struct object_file *o;

    log_debug("Creating object file '%s' for %lx:%lu...", name, dev, inode);

    o = malloc(sizeof(*o));
    if (!o) {
        log_error("FAILED\n");
        return NULL;
    }
    memset(o, 0, sizeof(struct object_file));

    INIT_LIST_HEAD(&o->list);
    INIT_LIST_HEAD(&o->vma);
    INIT_LIST_HEAD(&o->applied_patch);
    o->num_applied_patch = 0;
    o->proc = proc;
    o->dev = dev;
    o->inode = inode;
    o->is_patch = 0;

    o->prev_hole = hole;
    if (object_add_vm_area(o, vma, hole) < 0) {
        log_error("Cannot add vm area for %s\n", name);
        free(o);
        return NULL;
    }

    o->name = strdup(name);
    o->is_elf = 0;

    list_add(&o->list, &proc->objs);
    proc->num_objs++;

    log_debug("OK\n");
    return o;
}

static void link_funcs_name(struct upatch_info *uinfo)
{
    unsigned long idx = 0;

    for (unsigned long i = 0; i < uinfo->changed_func_num; i++) {
        char *name = (char *)uinfo->func_names + idx;

        uinfo->funcs[i].name = name;
        idx += strlen(name) + 1;
    }
}

static void free_object_patch(struct object_patch *opatch)
{
    if (opatch == NULL) {
        return;
    }

    if (opatch->uinfo != NULL) {
        if (opatch->uinfo->funcs != NULL) {
            free(opatch->uinfo->funcs);
        }
        if (opatch->uinfo->func_names != NULL) {
            free(opatch->uinfo->func_names);
        }
        free(opatch->uinfo);
    }

    free(opatch);
}

static int add_upatch_object(struct upatch_process *proc, struct object_file *o,
    unsigned long src, unsigned char *header_buf)
{
    struct object_patch *opatch;

    opatch = malloc(sizeof(struct object_patch));
    if (opatch == NULL) {
        log_error("malloc opatch failed\n");
        return -1;
    }

    opatch->obj = o;
    opatch->uinfo = malloc(sizeof(struct upatch_info));
    if (opatch->uinfo == NULL) {
        log_error("malloc opatch->uinfo failed\n");
        free(opatch);
        return -1;
    }

    memcpy(opatch->uinfo->magic, header_buf, sizeof(struct upatch_info));

    opatch->uinfo->func_names = malloc(opatch->uinfo->func_names_size);
    if (opatch->uinfo->func_names == NULL) {
        log_error("Failed to malloc funcs_names\n");
        free_object_patch(opatch);
        return -ENOMEM;
    }

    if (upatch_process_mem_read(proc, src,
        opatch->uinfo->func_names, opatch->uinfo->func_names_size)) {
        log_error("Cannot read patch func names at 0x%lx\n", src);
        free_object_patch(opatch);
        return -1;
    }

    src += opatch->uinfo->func_names_size;
    opatch->uinfo->funcs = malloc(opatch->uinfo->changed_func_num *
        sizeof(struct upatch_info_func));
    if (upatch_process_mem_read(proc, src, opatch->uinfo->funcs,
        opatch->uinfo->changed_func_num * sizeof(struct upatch_info_func))) {
        log_error("can't read patch funcs at 0x%lx\n", src);
        free_object_patch(opatch);
        return -1;
    }

    link_funcs_name(opatch->uinfo);
    list_add(&opatch->list, &o->applied_patch);
    o->num_applied_patch++;
    o->is_patch = 1;

    return 0;
}
/**
 * Returns: 0 if everything is ok, -1 on error.
 */
static int process_add_vma(struct upatch_process *proc,
    dev_t dev, ino_t inode, char *name,
    struct vm_area *vma, struct vm_hole *hole)
{
    int object_type;
    unsigned char header_buf[1024];
    struct object_file *o;

    /* Event though process_get_object_type() return -1,
     * we still need continue process. */
    object_type = process_get_object_type(proc, vma, name,
        header_buf, sizeof(header_buf));
    if (object_type != OBJECT_UPATCH) {
        /* Is not a upatch, look if this is a vm_area of an already
         * enlisted object.
         */
        list_for_each_entry_reverse(o, &proc->objs, list) {
            if ((dev && inode && o->dev == dev &&
                 o->inode == (ino_t)inode) ||
                (dev == 0 && !strcmp(o->name, name))) {
                return object_add_vm_area(o, vma, hole);
            }
        }
    }

    o = process_new_object(proc, dev, inode, name, vma, hole);
    if (o == NULL) {
        return -1;
    }

    if (object_type == OBJECT_UPATCH) {
        unsigned long src = vma->start + sizeof(struct upatch_info);
        if (add_upatch_object(proc, o, src, header_buf) != 0) {
            return -1;
        }
    }

    if (object_type == OBJECT_ELF) {
        o->is_elf = 1;
    }

    return 0;
}

int upatch_process_map_object_files(struct upatch_process *proc)
{
    int ret = 0;

    /*
     * 1. Create the list of all objects in the process
     * 2. Check whether we have patch for any of them
     * 3. If we have at least one patch, create files for all
     *    of the object (we might have references to them
     *    in the patch).
     */
    int fd = dup(proc->fdmaps);
    if (fd < 0) {
        log_error("unable to dup fd %d", proc->fdmaps);
        return -1;
    }

    lseek(fd, 0, SEEK_SET);
    FILE *file = fdopen(fd, "r");
    if (file == NULL) {
        log_error("unable to fdopen %d", fd);
        close(fd);
        return -1;
    }

    unsigned long hole_start = 0;

    char line[1024];
    while (fgets(line, sizeof(line), file) != NULL) {
        struct vm_area vma;
        unsigned long vma_start;
        unsigned long vma_end;
        unsigned long offset;
        unsigned int maj;
        unsigned int min;
        unsigned int inode;
        char perms[5];
        char name_buf[256];
        char *name = name_buf;

        ret = sscanf(line, "%lx-%lx %s %lx %x:%x %u %255s",
            &vma_start, &vma_end, perms, &offset,
            &maj, &min, &inode, name_buf);
        if (ret == EOF) {
            log_error("Failed to read maps: unexpected EOF");
            goto error;
        }
        if (ret != 8) {
            name = "[anonymous]";
        }

        vma.start = vma_start;
        vma.end = vma_end;
        vma.offset = offset;
        vma.prot = perms2prot(perms);

        /* Hole must be at least 2 pages for guardians */
        struct vm_hole *hole = NULL;
        if ((hole_start != 0) &&
            (vma_start - hole_start > 2 * (uintptr_t)PAGE_SIZE)) {
            uintptr_t start = hole_start + (uintptr_t)PAGE_SIZE;
            uintptr_t end = vma_start - (uintptr_t)PAGE_SIZE;

            hole = process_add_vm_hole(proc, start, end);
            if (hole == NULL) {
                log_error("Failed to add vma hole");
                goto error;
            }
            log_debug("vm_hole: start=0x%lx, end=0x%lx, len=0x%lx\n",
                hole->start, hole->end, hole->len);
        }
        hole_start = vma_end;

        name = name[0] == '/' ? basename(name) : name;
        ret = process_add_vma(proc, makedev(maj, min), inode, name, &vma, hole);
        if (ret < 0) {
            log_error("Failed to add object vma");
            goto error;
        }

        if ((proc->libc_base == 0) &&
            (vma.prot & PROT_EXEC) &&
            !strncmp(basename(name), "libc", 4)) {
            proc->libc_base = vma_start;
        }
    }

    (void)fclose(file);
    (void)close(fd);
    log_debug("Found %d object file(s)\n", proc->num_objs);

    if (proc->libc_base == 0) {
        log_error("Cannot find libc_base, pid=%d",
            proc->pid);
        return -1;
    }

    return 0;

error:
    (void)fclose(file);
    (void)close(fd);
    return -1;
}

static int process_list_threads(struct upatch_process *proc, int **ppids,
    size_t *npids, size_t *alloc)
{
    DIR *dir = NULL;
    struct dirent *de;
    char path[PATH_MAX];
    int *pids = *ppids;

    (void) snprintf(path, sizeof(path), "/proc/%d/task", proc->pid);
    dir = opendir(path);
    if (!dir) {
        log_error("Failed to open directory '%s'\n", path);
        goto dealloc;
    }

    *npids = 0;
    while ((de = readdir(dir))) {
        int *t;
        if (de->d_name[0] == '.') {
            continue;
        }
        if (*npids >= *alloc) {
            *alloc = *alloc ? *alloc * 2 : 1;
            t = realloc(pids, *alloc * sizeof(*pids));
            if (t == NULL) {
                log_error("Failed to (re)allocate memory for pids\n");
                goto dealloc;
            }
            pids = t;
        }

        pids[*npids] = atoi(de->d_name);
        (*npids)++;
    }

    closedir(dir);
    *ppids = pids;

    return (int)*npids;

dealloc:
    if (dir) {
        closedir(dir);
    }

    free(pids);
    *ppids = NULL;
    *alloc = *npids = 0;
    return -1;
}

int upatch_process_attach(struct upatch_process *proc)
{
    int *pids = NULL;
    int ret;

    size_t i;
    size_t npids = 0;
    size_t alloc = 0;
    size_t prevnpids = 0;
    size_t nattempts;

    if (upatch_process_mem_open(proc, MEM_WRITE) < 0) {
        return -1;
    }

    for (nattempts = 0; nattempts < MAX_ATTACH_ATTEMPTS; nattempts++) {
        ret = process_list_threads(proc, &pids, &npids, &alloc);
        if (ret == -1) {
            goto detach;
        }

        if (nattempts == 0) {
            log_debug("Found %lu thread(s), attaching...\n", npids);
        } else {
            /*
             * FIXME(pboldin): This is wrong, amount of threads can
             * be the same because some new spawned and some old
             * died
             */
            if (prevnpids == npids) {
                break;
            }
            log_debug("Found %lu new thread(s), attaching...\n",
                prevnpids - npids);
        }

        for (i = prevnpids; i < npids; i++) {
            int pid = pids[i];

            ret = upatch_ptrace_attach_thread(proc, pid);
            if ((ret != 0) && (ret != ESRCH)) {
                goto detach;
            }
        }

        prevnpids = npids;
    }

    if (nattempts == MAX_ATTACH_ATTEMPTS) {
        log_error("Unable to catch up with process, bailing\n");
        goto detach;
    }

    log_debug("Attached to %lu thread(s): %d", npids, pids[0]);
    for (i = 1; i < npids; i++) {
        log_debug(", %d", pids[i]);
    }
    log_debug("\n");

    free(pids);
    return 0;

detach:
    upatch_process_detach(proc);
    free(pids);
    return -1;
}

void upatch_process_detach(struct upatch_process *proc)
{
    struct upatch_ptrace_ctx *p;
    struct upatch_ptrace_ctx *ptmp;
    int status;
    pid_t pid;

    if (proc->memfd >= 0 && close(proc->memfd) < 0) {
        log_error("Failed to close memfd");
    }
    proc->memfd = -1;

    list_for_each_entry_safe(p, ptmp, &proc->ptrace.pctxs, list) {
        /**
         * If upatch_ptrace_detach(p) return -ESRCH, there are two situations,
         * as described below:
         * 1. the specified thread does not exist, it means the thread dead
         *    during the attach processing, so we need to wait for the thread
         *    to exit;
         * 2. the specified thread is not currently being traced by us,
         *    or is not stopped, so we just ignore it;
         *
         * We using the running variable of the struct upatch_ptrace_ctx to
         * distinguish them:
         * 1. if pctx->running = 0, it means the thread is traced by us, we
         *    will wait for the thread to exit;
         * 2. if pctx->running = 1, it means we can not sure about the status of
         *    the thread, we just ignore it;
         */
        if (upatch_ptrace_detach(p) == -ESRCH && !p->running) {
            do {
                pid = waitpid(p->pid, &status, __WALL);
            } while (pid > 0 && !WIFEXITED(status));
        }
        list_del(&p->list);
        free(p);
    }
    log_debug("Process detached\n");
}

static inline struct vm_hole *next_hole(struct vm_hole *hole,
    struct list_head *head)
{
    if (hole == NULL || hole->list.next == head) {
        return NULL;
    }

    return list_entry(hole->list.next, struct vm_hole, list);
}

static inline struct vm_hole *prev_hole(struct vm_hole *hole,
    struct list_head *head)
{
    if (hole == NULL || hole->list.prev == head) {
        return NULL;
    }

    return list_entry(hole->list.prev, struct vm_hole, list);
}

int vm_hole_split(struct vm_hole *hole, uintptr_t start, uintptr_t end)
{
    uintptr_t new_start = ROUND_DOWN(start, (uintptr_t)PAGE_SIZE) -
        (uintptr_t)PAGE_SIZE;
    uintptr_t new_end = ROUND_UP(end, (uintptr_t)PAGE_SIZE) +
        (uintptr_t)PAGE_SIZE;

    if (new_start > hole->start) {
        struct vm_hole *left = NULL;

        left = malloc(sizeof(*hole));
        if (left == NULL) {
            log_error("Failed to malloc for vm hole");
            return ENOMEM;
        }

        left->start = hole->start;
        left->end = new_start;

        list_add(&left->list, &hole->list);
    }

    /* Reuse hole pointer as the right hole since it is pointed to by
     * the `prev_hole` of some `object_file`. */
    hole->start = new_end;
    hole->end = hole->end > new_end ? hole->end : new_end;

    return 0;
}

static bool is_vm_hole_suitable(struct obj_vm_area *vma,
    struct vm_hole *hole, size_t len)
{
    uintptr_t vma_start = vma->inmem.start;
    uintptr_t vma_end = vma->inmem.end;
    uintptr_t hole_start = PAGE_ALIGN(hole->start);
    uintptr_t hole_end = PAGE_ALIGN(hole->start + len);

    log_debug("vma_start=0x%lx, vma_end=0x%lx, "
        "hole_start=0x%lx, hole_end=0x%lx, hole_len=0x%lx\n",
        vma_start, vma_end, hole->start, hole->end, hole->len);
    if (hole->len < len) {
        return false;
    }

    if (hole_end < vma_start) {
        // hole is on the left side of the vma
        if ((vma_start - hole_start) <= MAX_DISTANCE) {
            return true;
        }
    } else if (hole_start > vma_end) {
        // hole is on the right side of the vma
        if ((hole_end - vma_end) <= MAX_DISTANCE) {
            return true;
        }
    }

    return false;
}
/*
 * Take object's `prev_hole` as a left candidate
 * and the next hole as a right candidate. Pace through them until there is
 * enough space in the hole for the patch.
 *
 * Due to relocation constraints, the hole position should be whin 4GB range
 * from the obj.
 * eg: R_AARCH64_ADR_GOT_PAGE
 */
struct vm_hole *find_patch_region(struct object_file *obj, size_t len)
{
    struct list_head *vma_holes = &obj->proc->vma_holes;

    struct obj_vm_area *vma = NULL;
    list_for_each_entry(vma, &obj->vma, list) {
        struct vm_hole *left_hole = obj->prev_hole;
        struct vm_hole *right_hole = NULL;
        if (left_hole) {
            right_hole = next_hole(left_hole, vma_holes);
        } else {
            if (!list_empty(vma_holes)) {
                right_hole = list_first_entry(vma_holes, struct vm_hole, list);
            }
        }

        while ((left_hole != NULL) || (right_hole != NULL)) {
            if (left_hole != NULL) {
                if (is_vm_hole_suitable(vma, left_hole, len)) {
                    return left_hole;
                }
                left_hole = prev_hole(left_hole, vma_holes);
            }
            if (right_hole != NULL) {
                if (is_vm_hole_suitable(vma, right_hole, len)) {
                    return right_hole;
                }
                right_hole = next_hole(right_hole, vma_holes);
            }
        }
    }

    return NULL;
}
