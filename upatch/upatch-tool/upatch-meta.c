#include "upatch-meta.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <error.h>
#include <stdbool.h>
#include <unistd.h>
#include <sys/types.h>
#include <limits.h>

#include "log.h"
#include "list.h"

/*
	meta_head-->elf1-->elf2-->elf3-->NULL
			   |	  |		 |__patch_head-->patch1-->patch2
			   |	  |__...				  |			|__symbol_head->symbol1-->symbol2...
			   |__...						  |__...
*/

typedef patch_symbols_t upatch_meta_symbol;

struct upatch_meta_patch {
	struct list_head self;
	char name[PATH_MAX];
	char uuid[UPATCH_UUID_LEN];
	patch_status_e status;
	// list
	struct list_head syms;
};

struct upatch_meta_elf {
	struct list_head self;
	char path[PATH_MAX]; // elf path name
	// list manage
	struct list_head patchs;
};

struct list_head meta_head = LIST_HEAD_INIT(meta_head);

int meta_patch_init()
{
	return 0;
}

void meta_patch_fini()
{
	return;
}

static struct upatch_meta_patch *find_patch_in_elf(struct upatch_meta_elf *elf, const char *uuid)
{
	struct upatch_meta_patch *patch;
	list_for_each_entry(patch, &elf->patchs, self) {
		if (strcmp(patch->uuid, uuid) == 0)
			return patch;
	}
	return NULL;
}

static struct upatch_meta_patch *find_patch_by_uuid(const char *uuid)
{
	struct upatch_meta_elf *elf;
	list_for_each_entry(elf, &meta_head, self) {
		struct upatch_meta_patch *patch;
		list_for_each_entry(patch, &elf->patchs, self) {
			if (strcmp(patch->uuid, uuid) == 0)
				return patch;
		}
	}
	return NULL;
}

static struct upatch_meta_elf *find_elf_by_path(const char *path)
{
	struct upatch_meta_elf *elf;
	list_for_each_entry(elf, &meta_head, self) {
		if (strcmp(elf->path, path) == 0)
			return elf;
	}
	return NULL;
}

static int list_add_symbol(struct list_head *head, patch_symbols_t *sym)
{
	upatch_meta_symbol *newsym = (upatch_meta_symbol *)malloc(sizeof(upatch_meta_symbol));
	if (newsym == NULL)
		return -ENOMEM;
	memset(newsym, 0, sizeof(upatch_meta_symbol));
	strncpy(newsym->name, sym->name, sizeof(newsym->name));
	newsym->offset = sym->offset;
	INIT_LIST_HEAD(&newsym->self);
	list_add(&newsym->self, head);
	return 0;
}

static void list_remove_all_symbols(struct list_head *head)
{
	upatch_meta_symbol *sym, *symsafe;
	list_for_each_entry_safe(sym, symsafe, head, self) {
		list_del(&sym->self);
		free(sym);
	}

	return;
}

static int patch_add_all_symbols(struct upatch_meta_patch *patch, struct list_head *syms)
{
	patch_symbols_t *sym;

	if (syms == NULL || list_empty(syms)) {
		log_warn("patch:%s symbols list is empty or NULL:%s, add patch failed.",
				patch->name, (syms == NULL) ? "NULL" : "valid");
		return -ENOENT;
	}
	list_for_each_entry(sym, syms, self) {
		if (list_add_symbol(&patch->syms, sym) != 0) {
			list_remove_all_symbols(&patch->syms);
			log_warn("malloc new symbol failed, name:%s offset:%d\n", sym->name, sym->offset);
			return -ENOMEM;
		}
		log_debug("+add sym:%s offset:%d to patch:%s\n", sym->name, sym->offset, patch->name);
	}
	log_debug("successed to add symbols to patch:%s uuid:%s\n", patch->name, patch->uuid);
	return 0;
}

static int create_new_patch(const char *uuid, patch_entity_t *entity, struct list_head *elf_lst)
{
	struct upatch_meta_patch *patch = (struct upatch_meta_patch *)malloc(sizeof(struct upatch_meta_patch));
	if (patch == NULL) {
		log_warn("create new patch malloc failed, uuid:%s path:%s.\n",
				uuid, entity->patch_path);
		return -ENOMEM;
	}
	memset(patch, 0, sizeof(struct upatch_meta_patch));
	INIT_LIST_HEAD(&patch->self);
	INIT_LIST_HEAD(&patch->syms);
	if (patch_add_all_symbols(patch, entity->symbols) != 0) {
		log_warn("create new patch failed, add symbols error.\n");
		free(patch);
		return -ENOMEM;
	}
	patch->status = entity->status;
	strncpy(patch->name, entity->patch_path, sizeof(patch->name));
	strncpy(patch->uuid, uuid, sizeof(patch->uuid));
	// add to elf list
	list_add(&patch->self, elf_lst);
	return 0;
}

static int create_new_elf(const char *uuid, patch_entity_t *entity, struct list_head *lst)
{
	int ret;
	struct upatch_meta_elf *elf = (struct upatch_meta_elf *)malloc(sizeof(struct upatch_meta_elf));
	if (elf == NULL) {
		log_warn("create new elf malloc failed, uuid:%s elf path:%s patch path:%s, status:%u.\n",
				uuid, entity->target_path, entity->patch_path, entity->status);
		return -ENOMEM;
	}
	memset(elf, 0, sizeof(struct upatch_meta_elf));
	INIT_LIST_HEAD(&elf->self);
	INIT_LIST_HEAD(&elf->patchs);
	// create new patch and add to elf->patchs
	if ((ret = create_new_patch(uuid, entity, &elf->patchs)) != 0) {
		log_warn("create new patch uuid:%s failed:%d...\n", uuid, ret);
		free(elf);
		return ret;
	}
	strncpy(elf->path, entity->target_path, sizeof(elf->path));
	// add elf to global list
	list_add(&elf->self, lst);
	return 0;
}

// 创建补丁管理结构
int meta_create_patch(const char *uuid, patch_entity_t *entity)
{
	int ret;
	struct upatch_meta_patch *patch;
	struct upatch_meta_elf *elf;
	if (uuid == NULL || entity == NULL) {
		log_warn("meta creat patch uuid:%s or entity:%s invalid\n",
			(uuid == NULL) ? "NULL" : uuid,
			(entity == NULL) ? "NULL" : "VALID");
		return -EINVAL;
	}
	if ((patch = find_patch_by_uuid(uuid)) != NULL) {
		log_warn("meta create patch failed, uuid:%s exist, patch:%s status:%s !\n", patch->name, uuid);
		return -EEXIST;
	}
	elf = find_elf_by_path(entity->target_path);
	// finded elf
	if (elf) {
		if ((ret = create_new_patch(uuid, entity, &elf->patchs)) != 0) {
			log_warn("create new patch uuid:%s failed:%d...\n", uuid, ret);
			return ret;
		}
		return 0;
	}
	// elf not exist, create new elf and create new patch and add to elf
	if ((ret = create_new_elf(uuid, entity, &meta_head)) != 0) {
		log_warn("create patch failed, uuid:%s elf path:%s patch path:%s, status:%u ret:%d.\n",
				uuid, entity->target_path, entity->patch_path, entity->status, ret);
		return ret;
	}
	log_debug("create patch successed, uuid:%s elf path:%s patch path:%s, status:%u.\n",
				uuid, entity->target_path, entity->patch_path, entity->status);
	return 0;
}

// 删除补丁管理结构
void meta_remove_patch(const char *uuid)
{
	struct upatch_meta_elf *elf, *elfsafe;
	list_for_each_entry_safe(elf, elfsafe, &meta_head, self) {
		struct upatch_meta_patch *patch = find_patch_in_elf(elf, uuid);
		if (patch == NULL)
			continue;
		// 摘除patch
		list_del(&patch->self);
		free(patch);
		// elf->patchs 非空
		if (!list_empty(&elf->patchs))
			break;
		// elf->patchs删空了，释放这个elf
		list_del(&elf->self);
		log_debug("elf path:%s patchs is empty, remove this elf meta\n",
				elf->path);
		free(elf);
		break;
	}
	return;
}

// 查找patch
int meta_get_patch_entity(const char *uuid, patch_entity_t *entity)
{
	struct upatch_meta_elf *elf;
	if (uuid == NULL || entity == NULL) {
		log_warn("meta get patch entity uuid:%s or entity:%s invalid\n",
				(uuid == NULL) ? "NULL" : uuid,
				(entity == NULL) ? "NULL" : "VALID");
		return -EINVAL;
	}
	list_for_each_entry(elf, &meta_head, self) {
		struct upatch_meta_patch *patch = find_patch_in_elf(elf, uuid);
		if (patch == NULL)
			continue;
		// finded patch
		entity->status = patch->status;
		strncpy(entity->target_path, elf->path, sizeof(entity->target_path));
		strncpy(entity->patch_path, patch->name, sizeof(entity->patch_path));
		entity->symbols = &patch->syms;
		return 0;
	}
	log_warn("uuid:%s cant find patch.\n", uuid);
	return -ENOENT;
}

// 释放patch_symbols_t **类型返回内存
void meta_put_symbols(struct list_head *symbols)
{
	upatch_meta_symbol *sym, *symsafe;
	list_for_each_entry_safe(sym, symsafe, symbols, self) {
		list_del(&sym->self);
		free(sym);
	}
	free(symbols);
	return;
}

#define LIST_HEAD_INIT_P(p) {p, p}
// 查找elf函数列表
struct list_head *meta_get_elf_symbols(const char *elf_path)
{
	struct upatch_meta_elf *elf;
	struct list_head *syms;
	struct upatch_meta_patch *patch;
	if (elf_path == NULL) {
		log_warn("elf path invalid\n");
		return NULL;
	}
	syms = (struct list_head *)malloc(sizeof(struct list_head));
	if (syms == NULL) {
		log_warn("failed to malloc list head\n");
		return NULL;
	}
	INIT_LIST_HEAD(syms);
	elf = find_elf_by_path(elf_path);
	if (elf == NULL) {
		log_warn("elf path:%s not exist.\n", elf_path);
		return NULL;
	}
	list_for_each_entry(patch, &elf->patchs, self) {
		upatch_meta_symbol *sym;
		log_debug("Find patch:%s uuid:%s to add symbol.\n", patch->name, patch->uuid);
		list_for_each_entry(sym, &patch->syms, self) {
			if (list_add_symbol(syms, sym) != 0) {
				log_warn("add sym:%s offset:%u to result failed!\n", sym->name, sym->offset);
				meta_put_symbols(syms);
				return NULL;
			}
			log_debug(" ++add sym:%s offset:%lu to result.\n", sym->name, sym->offset);
		}
	}
	return syms;
}

// 查找补丁函数列表
struct list_head *meta_get_patch_symbols(const char *uuid)
{
	struct list_head *syms;
	upatch_meta_symbol *sym;
	struct upatch_meta_patch *patch;
	if (uuid == NULL) {
		log_warn("get patch symbols uuid is invalid.\n");
		return NULL;
	}
	syms = (struct list_head *)malloc(sizeof(struct list_head));
	if (syms == NULL) {
		log_warn("failed to malloc list head\n");
		return NULL;
	}
	INIT_LIST_HEAD(syms);
	patch = find_patch_by_uuid(uuid);
	if (patch == NULL) {
		log_warn("can't find patch with uuid:%s\n", uuid);
		meta_put_symbols(syms);
		return NULL;
	}
	list_for_each_entry(sym, &patch->syms, self) {
		if (list_add_symbol(syms, sym) != 0) {
			log_warn("add sym:%s offset:%u to result failed!\n", sym->name, sym->offset);
			meta_put_symbols(syms);
			return NULL;
		}
	}
	return syms;
}


patch_status_e meta_get_patch_status(const char *uuid)
{
	struct upatch_meta_patch *patch;
	if (uuid == NULL) {
		log_warn("meta get patch status uuid:%s invalid\n", (uuid == NULL) ? "NULL" : uuid);
		return UPATCH_PATCH_STATUS_NOT_APPLIED;
	}
	patch = find_patch_by_uuid(uuid);
	if (patch == NULL) {
		log_warn("can't find patch uuid:%s failed to get status\n", uuid);
		return UPATCH_PATCH_STATUS_NOT_APPLIED;
	}
	return patch->status;
}

// 设置补丁状态
int meta_set_patch_status(const char *uuid, patch_status_e status)
{
	struct upatch_meta_patch *patch;
	if (uuid == NULL || status >= UPATCH_PATCH_STATUS_INV) {
		log_warn("meta set patch status uuid:%s or status:%u invalid\n", (uuid == NULL) ? "NULL" : uuid, status);
		return -EINVAL;
	}
	patch = find_patch_by_uuid(uuid);
	if (patch == NULL) {
		log_warn("can't find patch uuid:%s failed to set status:%u\n", uuid, status);
		return -ENOENT;
	}
	log_debug("meta hit patch status:%u set to %u\n", patch->status, status);
	patch->status = status;

	return 0;
}

int meta_print_all()
{
	struct upatch_meta_elf *elf;
	struct upatch_meta_patch *patch;
	upatch_meta_symbol *sym;
	log_debug("List all patch info:");
	list_for_each_entry(elf, &meta_head, self) {
		log_debug(" + elf:%s", elf->path);
		list_for_each_entry(patch, &elf->patchs, self) {
			log_debug("   + patch:%s uuid:%s", patch->name, patch->uuid);
			list_for_each_entry(sym, &patch->syms, self) {
				log_debug("     + symbol:%s offset:%u", sym->name, sym->offset);
			}
		}
	}
	return 0;
}
