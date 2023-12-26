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

struct upatch_meta_symbol {
	struct list_head self;

	char name[UPATCH_SYMBOL_NAME_MAX];
	loff_t offset;
	struct list_head cover; // to record symbol cover list
	void *cover_head;
	void *patch;
};

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

// get elf who has patch of *uuid
static struct upatch_meta_elf *find_elf_by_uuid(const char *uuid)
{
	struct upatch_meta_elf *elf;
	list_for_each_entry(elf, &meta_head, self) {
		struct upatch_meta_patch *patch;
		list_for_each_entry(patch, &elf->patchs, self) {
			if (strcmp(patch->uuid, uuid) == 0)
				return elf;
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

static struct upatch_meta_patch *find_patch_by_symbol(struct upatch_meta_symbol *cover_sym)
{
	return cover_sym->patch;
}

static int symbol_add_cover(struct upatch_meta_symbol *finded, struct upatch_meta_symbol *newsym)
{
	struct list_head *cover_head;
	if (list_empty(&finded->cover)) {
		cover_head = (struct list_head *)malloc(sizeof(struct list_head));
		if (cover_head == NULL) {
			log_warn("malloc failed when add cover\n");
			return ENOMEM;
		}
		INIT_LIST_HEAD(cover_head);
		// list add is add as first node every time,
		// so cover list is from newer to older
		list_add(&finded->cover, cover_head);
		list_add(&newsym->cover, cover_head);
		finded->cover_head = cover_head;
		newsym->cover_head = cover_head;
		return 0;
	}

	cover_head = finded->cover_head;
	list_add(&newsym->cover, cover_head);
	newsym->cover_head = cover_head;
	return 0;
}

static void symbol_delete_from_cover(struct upatch_meta_symbol *symbol)
{
	struct list_head *head = symbol->cover_head;
	if (!list_empty(&symbol->cover)) {
		list_del(&symbol->cover);
		INIT_LIST_HEAD(&symbol->cover);
	}
	if (head != NULL) {
		if (head != &symbol->cover && list_empty(head)) {
			free(head);
		}
		symbol->cover_head = NULL;
	}
	return;
}

static int symbol_active_in_cover(struct list_head *patch_list, struct upatch_meta_symbol *symbol)
{
	struct upatch_meta_patch *patch;
	list_for_each_entry(patch, patch_list, self) {
		struct upatch_meta_symbol *sym;
		if (patch->status != UPATCH_PATCH_STATUS_ACTIVED)
			continue;
		list_for_each_entry(sym, &patch->syms, self) {
			if (sym->offset == symbol->offset) {
				// find cover
				return symbol_add_cover(sym, symbol);
			}
		}
	}
	// no cover
	return 0;
}

static int patch_active_in_cover(struct list_head *patch_list, struct upatch_meta_patch *patch)
{
	int ret;
	struct upatch_meta_symbol *sym;
	list_for_each_entry(sym, &patch->syms, self) {
		if ((ret = symbol_active_in_cover(patch_list, sym)) != 0) {
			log_warn("symbol offset:%ld active in cover failed!\n", sym->offset);
			return ret;
		}
	}
	return 0;
}

// 此处认为已经check过，不再进行cover check，直接从cover栈移除
static int patch_deactive_in_cover(struct upatch_meta_patch *patch)
{
	struct upatch_meta_symbol *sym;
	list_for_each_entry(sym, &patch->syms, self) {
		symbol_delete_from_cover(sym);
	}
	return 0;
}


static int list_add_symbol(struct list_head *head, struct upatch_meta_symbol *sym)
{
	struct upatch_meta_symbol *newsym = (struct upatch_meta_symbol *)malloc(sizeof(struct upatch_meta_symbol));
	if (newsym == NULL) {
		return ENOMEM;
	}
	memset(newsym, 0, sizeof(struct upatch_meta_symbol));

	strncpy(newsym->name, sym->name, sizeof(newsym->name));
	newsym->offset = sym->offset;
	INIT_LIST_HEAD(&newsym->self);
	INIT_LIST_HEAD(&newsym->cover);
	list_add(&newsym->self, head);
	// 如果有symbol覆盖，在此解决建链
	return 0;
}

static int list_add_symbol_for_patch(struct upatch_meta_patch *patch, struct list_head *head, struct upatch_meta_symbol *sym)
{
	struct upatch_meta_symbol *newsym = (struct upatch_meta_symbol *)malloc(sizeof(struct upatch_meta_symbol));
	if (newsym == NULL) {
		return ENOMEM;
	}
	memset(newsym, 0, sizeof(struct upatch_meta_symbol));

	strncpy(newsym->name, sym->name, sizeof(newsym->name));
	newsym->offset = sym->offset;
	INIT_LIST_HEAD(&newsym->self);
	INIT_LIST_HEAD(&newsym->cover);
	newsym->patch = patch;
	list_add(&newsym->self, head);
	// 如果有symbol覆盖，在此解决建链
	return 0;
}


// 作为create补丁出错时的处理，不需要考虑cover的处理，因为
// 新增补丁如果要回滚，它肯定是cover栈顶
static void list_remove_all_symbols(struct list_head *head)
{
	struct upatch_meta_symbol *sym, *symsafe;
	list_for_each_entry_safe(sym, symsafe, head, self) {
		list_del(&sym->self);
		symbol_delete_from_cover(sym);
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
		return ENOENT;
	}
	list_for_each_entry(sym, syms, self) {
		if (list_add_symbol_for_patch(patch, &patch->syms, (struct upatch_meta_symbol *)sym) != 0) {
			list_remove_all_symbols(&patch->syms);
			log_warn("malloc new symbol failed, name:%s offset:%ld\n", sym->name, sym->offset);
			return ENOMEM;
		}
		log_debug("+add sym:%s offset:%ld to patch:%s\n", sym->name, sym->offset, patch->name);
	}
	log_debug("successed to add symbols to patch:%s uuid:%s\n", patch->name, patch->uuid);
	return 0;
}

static int patch_check_symbol_cover(struct upatch_meta_patch *patch)
{
	struct upatch_meta_symbol *sym;
	list_for_each_entry(sym, &patch->syms, self) {
		if (list_empty(&sym->cover) || sym->cover.next == LIST_POISON1)
			continue;

		struct list_head *cover_head = sym->cover_head;
		if (cover_head->next != &sym->cover) {
			//log_warn("cover head:%lx next:%lx symcover:%lx", cover_head, cover_head->next, &sym->cover);
			// 任意一个symbol有cover列表，且不在栈顶则无法移除此补丁
			return EFAULT;
		}
	}
	return 0;
}

static int create_new_patch(const char *uuid, patch_entity_t *entity, struct list_head *patch_list)
{
	if (entity->status == UPATCH_PATCH_STATUS_ACTIVED) {
		log_warn("new patch:%s status is ACTIVED is not allowed!\n", uuid);
		return EFAULT;
	}
	struct upatch_meta_patch *patch = (struct upatch_meta_patch *)malloc(sizeof(struct upatch_meta_patch));
	if (patch == NULL) {
		log_warn("create new patch malloc failed, uuid:%s path:%s.\n",
				uuid, entity->patch_path);
		return ENOMEM;
	}
	memset(patch, 0, sizeof(struct upatch_meta_patch));
	INIT_LIST_HEAD(&patch->self);
	INIT_LIST_HEAD(&patch->syms);
	if (patch_add_all_symbols(patch, entity->symbols) != 0) {
		log_warn("create new patch failed, add symbols error.\n");
		free(patch);
		return ENOMEM;
	}
	patch->status = entity->status;
	strncpy(patch->name, entity->patch_path, sizeof(patch->name));
	strncpy(patch->uuid, uuid, sizeof(patch->uuid));
	// add to elf list
	list_add(&patch->self, patch_list);
	return 0;
}

static int create_new_elf(const char *uuid, patch_entity_t *entity, struct list_head *lst)
{
	int ret;
	struct upatch_meta_elf *elf = (struct upatch_meta_elf *)malloc(sizeof(struct upatch_meta_elf));
	if (elf == NULL) {
		log_warn("create new elf malloc failed, uuid:%s elf path:%s patch path:%s, status:%u.\n",
				uuid, entity->target_path, entity->patch_path, entity->status);
		return ENOMEM;
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

static int symbol_collision_add(struct list_head *head, struct upatch_meta_patch *patch)
{
	symbol_collision *sym_col = (symbol_collision *)malloc(sizeof(symbol_collision));
	if (sym_col == NULL) {
		log_warn("symbol collision add malloc failed.\n");
		return ENOMEM;
	}
	memset(sym_col, 0, sizeof(symbol_collision));
	INIT_LIST_HEAD(&sym_col->self);
	list_add(head, &sym_col->self);
	memcpy(sym_col->uuid, patch->uuid, sizeof(sym_col->uuid));
	return 0;
}

static struct list_head *symbol_get_collision_list(struct upatch_meta_elf *elf, struct list_head *syms)
{
	struct list_head *ret = NULL;
	struct upatch_meta_patch *patch;
	int finded_in_patch = 0;

	list_for_each_entry(patch, &elf->patchs, self) {
		struct upatch_meta_symbol *sym;
		finded_in_patch = 0;
		list_for_each_entry(sym, &patch->syms, self) {
			struct upatch_meta_symbol *add_sym;
			list_for_each_entry(add_sym, syms, self) {
				if (add_sym->offset == sym->offset && patch->status == UPATCH_PATCH_STATUS_ACTIVED) {
					if (ret == NULL) {
						ret = (struct list_head *)malloc(sizeof(struct list_head));
						if (ret == NULL) {
							log_warn("malloc failed\n");
							return NULL;
						}
						INIT_LIST_HEAD(ret);
					}
					symbol_collision_add(ret, patch);
					finded_in_patch = 1;
					log_warn("find conflict patch in elf:%s.\n"
							"   => exist sym:%s offset:%ld.\n"
							"   => new sym:%s offset:%ld\n"
							"   => exist patch status:%u path:%s uuid:%s.\n",
							elf->path, sym->name, sym->offset, add_sym->name, add_sym->offset, patch->status,
							patch->name, patch->uuid);
					break;
				}
			}
			// in patch loop
			if (finded_in_patch == 1)
				break;
		}
	}
	return ret;
}

#define FREE_WHOLE_LIST(sym, symsafe, lst, node) \
	do {\
		if (lst == NULL)\
			break;\
		list_for_each_entry_safe(sym, symsafe, lst, node) {\
			list_del(&sym->node);\
			free(sym);\
		}\
		free(lst);\
	} while (0);


// ===================================PUBLIC API=======================================
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
		return EINVAL;
	}
	if ((patch = find_patch_by_uuid(uuid)) != NULL) {
		log_warn("meta create patch failed, uuid:%s exist, patch:%s status:%u!\n", uuid, patch->name, patch->status);
		return EEXIST;
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
int meta_remove_patch(const char *uuid)
{
	struct upatch_meta_elf *elf, *elfsafe;
	list_for_each_entry_safe(elf, elfsafe, &meta_head, self) {
		struct upatch_meta_patch *patch = find_patch_in_elf(elf, uuid);
		if (patch == NULL)
			continue;
		// 摘除patch前先检查补丁覆盖情况是否符合
		if (patch_check_symbol_cover(patch) != 0) {
			log_warn("Can't remove patch because of symbol cover.\n");
			return EFAULT;
		}
		list_del(&patch->self);
		list_remove_all_symbols(&patch->syms);
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
	return 0;
}

// 查找patch
int meta_get_patch_entity(const char *uuid, patch_entity_t *entity)
{
	struct upatch_meta_elf *elf;
	if (uuid == NULL || entity == NULL) {
		log_warn("meta get patch entity uuid:%s or entity:%s invalid\n",
				(uuid == NULL) ? "NULL" : uuid,
				(entity == NULL) ? "NULL" : "VALID");
		return EINVAL;
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
	return ENOENT;
}

// 释放patch_symbols_t **类型返回内存
void meta_put_symbols(struct list_head *symbols)
{
	struct upatch_meta_symbol *sym, *symsafe;
	FREE_WHOLE_LIST(sym, symsafe, symbols, self);
	return;
}

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
		struct upatch_meta_symbol *sym;
		log_debug("Find patch:%s uuid:%s to add symbol.\n", patch->name, patch->uuid);
		list_for_each_entry(sym, &patch->syms, self) {
			if (list_add_symbol(syms, sym) != 0) {
				log_warn("add sym:%s offset:%ld to result failed!\n", sym->name, sym->offset);
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
	struct upatch_meta_symbol *sym;
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
			log_warn("add sym:%s offset:%ld to result failed!\n", sym->name, sym->offset);
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
		return UPATCH_PATCH_STATUS_NOT_APPLIED;
	}
	return patch->status;
}

static int patch_cover_status_machine(const char *uuid, struct upatch_meta_patch *patch, patch_status_e status)
{
	struct upatch_meta_elf *elf = find_elf_by_uuid(uuid);
	if (elf == NULL) {
		log_warn("get elf by uuid:%s failed when set patch status:%u\n", uuid, status);
		return EFAULT;
	}

	if (patch->status != status && status == UPATCH_PATCH_STATUS_ACTIVED) {
		if (patch_active_in_cover(&elf->patchs, patch) != 0) {
			log_warn("symbol resolve cover failed.\n");
			return EFAULT;
		}
		return 0;
	}
	if (patch->status == UPATCH_PATCH_STATUS_ACTIVED && status != patch->status) {
		struct list_head *check = meta_patch_deactive_check(uuid);
		if (check != NULL) {
			log_warn("cover check failed!");
			meta_put_symbol_collision(check);
			return EFAULT;
		}
		if (patch_deactive_in_cover(patch) != 0) {
			log_warn("patch:%s deactive in cover failed.!\n", uuid);
			return EFAULT;
		}
		return 0;
	}
	return 0;
}

// 设置补丁状态
int meta_set_patch_status(const char *uuid, patch_status_e status)
{
	struct upatch_meta_patch *patch;
	if (uuid == NULL || status >= UPATCH_PATCH_STATUS_INV) {
		log_warn("meta set patch status uuid:%s or status:%u invalid\n", (uuid == NULL) ? "NULL" : uuid, status);
		return EINVAL;
	}
	patch = find_patch_by_uuid(uuid);
	if (patch == NULL) {
		log_warn("can't find patch uuid:%s failed to set status:%u\n", uuid, status);
		return ENOENT;
	}
	if (patch_cover_status_machine(uuid, patch, status) != 0) {
		log_warn("set patch:%s status:%u failed.\n", uuid, status);
		return EFAULT;
	}

	log_debug("meta hit patch status:%u set to %u\n", patch->status, status);
	patch->status = status;

	return 0;
}

struct list_head *meta_get_symbol_collision(const char *elf_path, struct list_head *symbols)
{
	struct upatch_meta_elf *elf = find_elf_by_path(elf_path);
	if (!elf)
		return NULL;

	return symbol_get_collision_list(elf, symbols);
}

void meta_put_symbol_collision(struct list_head *lst)
{
	symbol_collision *sym, *symsafe;
	FREE_WHOLE_LIST(sym, symsafe, lst, self);
	return;
}

struct list_head *meta_patch_deactive_check(const char *uuid)
{
	struct list_head *res = NULL;
	struct upatch_meta_symbol *sym;
	struct upatch_meta_patch *patch = find_patch_by_uuid(uuid);
	if (patch == NULL) {
		log_warn("can't find patch by uuid:%s\n", uuid);
		return NULL;
	}
	if (patch->status != UPATCH_PATCH_STATUS_ACTIVED) {
		log_warn("uuid:%s patch status is:%u no need to check.\n", uuid, patch->status);
		return NULL;
	}
	list_for_each_entry(sym, &patch->syms, self) {
		if (list_empty(&sym->cover) || sym->cover.next == LIST_POISON1)
			continue;
		struct list_head *cover_head = sym->cover_head;
		if (cover_head->next == &sym->cover)
			continue;
		if (res == NULL) {
			res = (struct list_head *)malloc(sizeof(struct list_head));
			if (res == NULL) {
				log_warn("malloc failed when deactive check.\n");
				return NULL;
			}
		}

		struct upatch_meta_symbol *cover_sym;
		list_for_each_entry(cover_sym, cover_head, cover) {
			if (&cover_sym->cover == &sym->cover)
				break;
			struct upatch_meta_patch *patch_cover = find_patch_by_symbol(cover_sym);
			symbol_collision_add(res, patch_cover);
		}
	}
	return res;
}

int meta_print_all()
{
	struct upatch_meta_elf *elf;
	struct upatch_meta_patch *patch;
	struct upatch_meta_symbol *sym;
	log_debug("List all patch info:");
	list_for_each_entry(elf, &meta_head, self) {
		log_debug(" + elf:%s", elf->path);
		list_for_each_entry(patch, &elf->patchs, self) {
			log_debug("   + patch:%s uuid:%s status:%u", patch->name, patch->uuid, patch->status);
			list_for_each_entry(sym, &patch->syms, self) {
				log_debug("     + symbol name:%s offset:%ld cover:%lx prev:%lx", sym->name, sym->offset,
						(unsigned long)&sym->cover, (unsigned long)sym->cover.prev);
			}
		}
	}
	return 0;
}
