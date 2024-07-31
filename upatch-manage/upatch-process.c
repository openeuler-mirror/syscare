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
#include "process.h"
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
	snprintf(path, sizeof(path), "/proc/%d/maps", pid);

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
	char *bn, *c;
	ssize_t ret;

	snprintf(path, sizeof(path), "/proc/%d/exe", proc->pid);
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
	INIT_LIST_HEAD(&proc->vmaholes);
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
	struct object_patch *opatch, *opatch_safe;
	struct obj_vm_area *ovma, *ovma_safe;

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
	struct upatch_ptrace_ctx *p, *p_safe;
	struct object_file *obj, *obj_safe;
	struct vm_hole *hole, *hole_safe;

	list_for_each_entry_safe(p, p_safe, &proc->ptrace.pctxs, list) {
		free(p);
	}

	list_for_each_entry_safe(hole, hole_safe, &proc->vmaholes, list) {
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
	ssize_t i, rv;

	snprintf(buf, PATH_MAX, "/proc/%d/cmdline", proc->pid);
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
				printf("%c", buf[i]);
			}
			else {
				printf(" ");
			}
		}
	}

err_close:
	close(fd);
}

void upatch_process_print_short(struct upatch_process *proc)
{
	printf("process %d, cmdline: ", proc->pid);
	process_print_cmdline(proc);
	printf("\n");
}

int upatch_process_mem_open(struct upatch_process *proc, int mode)
{
	char path[PATH_MAX];

	if (proc->memfd >= 0) {
		close(proc->memfd);
	}

	snprintf(path, sizeof(path), "/proc/%d/mem", proc->pid);
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

	if (perms[0] == 'r')
		prot |= PROT_READ;
	if (perms[1] == 'w')
		prot |= PROT_WRITE;
	if (perms[2] == 'x')
		prot |= PROT_EXEC;
	/* Ignore 'p'/'s' flag, we don't need it */
	return prot;
}

static struct vm_hole *process_add_vm_hole(struct upatch_process *proc,
					   unsigned long hole_start,
					   unsigned long hole_end)
{
	struct vm_hole *hole;

	hole = malloc(sizeof(*hole));
	if (hole == NULL)
		return NULL;

	memset(hole, 0, sizeof(*hole));
	hole->start = hole_start;
	hole->end = hole_end;

	list_add(&hole->list, &proc->vmaholes);

	return hole;
}

static int process_get_object_type(struct upatch_process *proc,
				   struct vm_area *vma, char *name,
				   unsigned char *buf, size_t bufsize)
{
	int ret, type = OBJECT_UNKNOWN;

	ret = upatch_process_mem_read(proc, vma->start, buf, bufsize);
	if (ret < 0)
		return -1;

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
	return ((a->start == b->start) && (a->end == b->end) &&
		(a->prot == b->prot));
}

static int object_add_vm_area(struct object_file *o, struct vm_area *vma,
			      struct vm_hole *hole)
{
	struct obj_vm_area *ovma;

	if (o->previous_hole == NULL)
		o->previous_hole = hole;
	list_for_each_entry(ovma, &o->vma, list) {
		if (vm_area_same(vma, &ovma->inmem))
			return 0;
	}
	ovma = malloc(sizeof(*ovma));
	if (!ovma)
		return -1;
	memset(ovma, 0, sizeof(*ovma));
	ovma->inmem = *vma;
	list_add(&ovma->list, &o->vma);
	return 0;
}

static struct object_file *
process_new_object(struct upatch_process *proc, dev_t dev, ino_t inode,
		   const char *name, struct vm_area *vma, struct vm_hole *hole)
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

	o->previous_hole = hole;
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

static int add_upatch_object(struct upatch_process *proc,
	struct object_file *o, unsigned long src, unsigned char *header_buf)
{
	struct object_patch *opatch;

	opatch = malloc(sizeof(struct object_patch));
	if (opatch == NULL) {
		log_error("malloc opatch failed\n");
		return -1;
	}

	opatch->uinfo = malloc(sizeof(struct upatch_info));
	if (opatch->uinfo == NULL) {
		log_error("malloc opatch->uinfo failed\n");
		free(opatch);
		return -1;
	}

	memcpy(opatch->uinfo, header_buf, sizeof(struct upatch_info));
	opatch->uinfo->funcs = malloc(opatch->uinfo->changed_func_num *
			       sizeof(struct upatch_info_func));
	if (upatch_process_mem_read(proc, src, opatch->uinfo->funcs,
		opatch->uinfo->changed_func_num * sizeof(struct upatch_info_func))) {
		log_error("can't read patch funcs at 0x%lx\n", src);
		free(opatch->uinfo->funcs);
		free(opatch->uinfo);
		free(opatch);
		return -1;
	}
	list_add(&opatch->list, &o->applied_patch);
	o->num_applied_patch++;
	o->is_patch = 1;

	return 0;
}
/**
 * Returns: 0 if everything is ok, -1 on error.
 */
static int process_add_object_vma(struct upatch_process *proc, dev_t dev,
				  ino_t inode, char *name, struct vm_area *vma,
				  struct vm_hole *hole)
{
	int object_type;
	unsigned char header_buf[1024];
	struct object_file *o;

	/* Event though process_get_object_type() return -1,
	 * we still need continue process. */
	object_type = process_get_object_type(proc, vma, name, header_buf,
					      sizeof(header_buf));

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

int upatch_process_parse_proc_maps(struct upatch_process *proc)
{
	FILE *f;
	int ret, is_libc_base_set = 0;
	unsigned long hole_start = 0;
	struct vm_hole *hole = NULL;

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
	f = fdopen(fd, "r");
	if (f == NULL) {
		log_error("unable to fdopen %d", fd);
		close(fd);
		return -1;
	}

	do {
		struct vm_area vma;
		char line[1024];
		unsigned long start, end, offset;
		unsigned int maj, min, inode;
		char perms[5], name_[256], *name = name_;
		int r;

		if (!fgets(line, sizeof(line), f)) {
			break;
		}

		r = sscanf(line, "%lx-%lx %s %lx %x:%x %d %255s", &start, &end,
			   perms, &offset, &maj, &min, &inode, name_);
		if (r == EOF) {
			log_error("Failed to read maps: unexpected EOF");
			goto error;
		}

		if (r != 8) {
			strcpy(name, "[anonymous]");
		}

		vma.start = start;
		vma.end = end;
		vma.offset = offset;
		vma.prot = perms2prot(perms);

		/* Hole must be at least 2 pages for guardians */
		if (start - hole_start > (unsigned long)(2 * PAGE_SIZE)) {
			hole = process_add_vm_hole(proc, hole_start + (unsigned long)PAGE_SIZE,
						   start - (unsigned long)PAGE_SIZE);
			if (hole == NULL) {
				log_error("Failed to add vma hole");
				goto error;
			}
		}
		hole_start = end;

		name = name[0] == '/' ? basename(name) : name;

		ret = process_add_object_vma(proc, makedev(maj, min), inode,
					     name, &vma, hole);
		if (ret < 0) {
			log_error("Failed to add object vma");
			goto error;
		}

		if (!is_libc_base_set && !strncmp(basename(name), "libc", 4) &&
		    vma.prot & PROT_EXEC) {
			proc->libc_base = start;
			is_libc_base_set = 1;
		}

	} while (1);
	fclose(f);
	close(fd);

	log_debug("Found %d object file(s)\n", proc->num_objs);

	if (!is_libc_base_set) {
		log_error("Can't find libc_base required for manipulations: %d",
			  proc->pid);
		return -1;
	}

	return 0;

error:
	fclose(f);
	close(fd);
	return -1;
}

int upatch_process_map_object_files(struct upatch_process *proc)
{
	// we can get plt/got table from mem's elf_segments
	// Now we read them from the running file
	return upatch_process_parse_proc_maps(proc);
}

// static int process_has_thread_pid(struct upatch_proces *proc, int pid)
// {
// 	struct upatch_ptrace_ctx *pctx;

// 	list_for_each_entry(pctx, &proc->ptrace.pctxs, list)
// 		if (pctx->pid == pid)
// 			return 1;

// 	return 0;
// }

static int process_list_threads(struct upatch_process *proc, int **ppids,
				size_t *npids, size_t *alloc)
{
	DIR *dir = NULL;
	struct dirent *de;
	char path[PATH_MAX];
	int *pids = *ppids;

	snprintf(path, sizeof(path), "/proc/%d/task", proc->pid);

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
	int *pids = NULL, ret;
	size_t i, npids = 0, alloc = 0, prevnpids = 0, nattempts;

	if (upatch_process_mem_open(proc, MEM_WRITE) < 0) {
		return -1;
	}

	for (nattempts = 0; nattempts < MAX_ATTACH_ATTEMPTS; nattempts++) {
		ret = process_list_threads(proc, &pids, &npids, &alloc);
		if (ret == -1)
			goto detach;

		if (nattempts == 0) {
			log_debug("Found %lu thread(s), attaching...\n", npids);
		} else {
			/*
			 * FIXME(pboldin): This is wrong, amount of threads can
			 * be the same because some new spawned and some old
			 * died
			 */
			if (prevnpids == npids)
				break;

			log_debug("Found %lu new thread(s), attaching...\n",
				  prevnpids - npids);
		}

		for (i = prevnpids; i < npids; i++) {
			int pid = pids[i];

			// if (process_has_thread_pid(proc, pid)) {
			// 	log_debug("already have pid %d\n", pid);
			// 	continue;
			// }

			ret = upatch_ptrace_attach_thread(proc, pid);
			if (ret < 0)
				goto detach;
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
	struct upatch_ptrace_ctx *p, *ptmp;
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
		// upatch_ptrace_ctx_destroy(p);
	}
	log_debug("Process detached\n");
}

static inline struct vm_hole *next_hole(struct vm_hole *hole,
					struct list_head *head)
{
	if (hole == NULL || hole->list.next == head)
		return NULL;

	return list_entry(hole->list.next, struct vm_hole, list);
}

static inline struct vm_hole *prev_hole(struct vm_hole *hole,
					struct list_head *head)
{
	if (hole == NULL || hole->list.prev == head)
		return NULL;

	return list_entry(hole->list.prev, struct vm_hole, list);
}

static inline unsigned long hole_size(struct vm_hole *hole)
{
	if (hole == NULL)
		return 0;
	return hole->end - hole->start;
}

int vm_hole_split(struct vm_hole *hole, unsigned long alloc_start,
		  unsigned long alloc_end)
{
	unsigned long page_size = (unsigned long)PAGE_SIZE;

	alloc_start = ROUND_DOWN(alloc_start, page_size) - page_size;
	alloc_end = ROUND_UP(alloc_end, page_size) + page_size;

	if (alloc_start > hole->start) {
		struct vm_hole *left = NULL;

		left = malloc(sizeof(*hole));
		if (left == NULL) {
			log_error("Failed to malloc for vm hole");
			return -1;
		}

		left->start = hole->start;
		left->end = alloc_start;

		list_add(&left->list, &hole->list);
	}

	/* Reuse hole pointer as the right hole since it is pointed to by
	 * the `previous_hole` of some `object_file`. */
	hole->start = alloc_end;
	hole->end = hole->end > alloc_end ? hole->end : alloc_end;

	return 0;
}

/*
 * Find region for a patch. Take object's `previous_hole` as a left candidate
 * and the next hole as a right candidate. Pace through them until there is
 * enough space in the hole for the patch.
 *
 * Since holes can be much larger than 2GiB take extra caution to allocate
 * patch region inside the (-2GiB, +2GiB) range from the original object.
 */
unsigned long object_find_patch_region(struct object_file *obj, size_t memsize,
				       struct vm_hole **hole)
{
	struct list_head *head = &obj->proc->vmaholes;
	struct vm_hole *left_hole = obj->previous_hole;
	struct vm_hole *right_hole = next_hole(left_hole, head);
	unsigned long max_distance = MAX_DISTANCE;
	struct obj_vm_area *sovma;

	unsigned long obj_start, obj_end;
	unsigned long region_start = 0, region_end = 0;

	log_debug("Looking for patch region for '%s'...\n", obj->name);

	sovma = list_first_entry(&obj->vma, struct obj_vm_area, list);
	obj_start = sovma->inmem.start;
	sovma = list_entry(obj->vma.prev, struct obj_vm_area, list);
	obj_end = sovma->inmem.end;

	max_distance -= memsize;

	/* TODO carefully check for the holes laying between obj_start and
	 * obj_end, i.e. just after the executable segment of an executable
	 */
	while (left_hole != NULL && right_hole != NULL) {
		if (right_hole != NULL &&
		    right_hole->start - obj_start > max_distance)
			right_hole = NULL;
		else if (hole_size(right_hole) > memsize) {
			region_start = right_hole->start;
			region_end = (right_hole->end - obj_start) <=
						     max_distance ?
					     right_hole->end - memsize :
					     obj_start + max_distance;
			*hole = right_hole;
			break;
		} else
			right_hole = next_hole(right_hole, head);

		if (left_hole != NULL &&
		    obj_end - left_hole->end > max_distance)
			left_hole = NULL;
		else if (hole_size(left_hole) > memsize) {
			region_start = (obj_end - left_hole->start) <=
						       max_distance ?
					       left_hole->start :
				       obj_end > max_distance ?
					       obj_end - max_distance :
					       0;
			region_end = left_hole->end - memsize;
			*hole = left_hole;
			break;
		} else
			left_hole = prev_hole(left_hole, head);
	}

	if (region_start == region_end) {
		log_error("Cannot find suitable region for patch '%s'\n", obj->name);
		return -1UL;
	}

	region_start = (region_start >> (unsigned long)PAGE_SHIFT) << (unsigned long)PAGE_SHIFT;
	log_debug("Found patch region for '%s' at 0x%lx\n", obj->name,
		  region_start);

	return region_start;
}
unsigned long object_find_patch_region_nolimit(struct object_file *obj, size_t memsize,
				       struct vm_hole **hole)
{
	struct list_head *head = &obj->proc->vmaholes;
	struct vm_hole *left_hole = obj->previous_hole;
	struct vm_hole *right_hole = next_hole(left_hole, head);
	unsigned long region_start = 0;

	log_debug("Looking for patch region for '%s'...\n", obj->name);

	while (right_hole != NULL) {
		if (hole_size(right_hole) > memsize) {
			*hole = right_hole;
			goto found;
		} else
			right_hole = next_hole(right_hole, head);

	while (left_hole != NULL)
		if (hole_size(left_hole) > memsize) {
			*hole = left_hole;
			goto found;
		} else
			left_hole = prev_hole(left_hole, head);
	}

	log_error("Cannot find suitable region for patch '%s'\n", obj->name);
	return -1UL;
found:
	region_start = ((*hole)->start >> PAGE_SHIFT) << PAGE_SHIFT;
	log_debug("Found patch region for '%s' 0xat %lx\n", obj->name,
		  region_start);

	return region_start;
}
