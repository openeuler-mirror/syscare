// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-lib
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

#ifndef __UPATCH_META_H_
#define __UPATCH_META_H_

#include <limits.h>
#include <sys/types.h>

#include "list.h"

#define UPATCH_SYMBOL_NAME_MAX 64 // with \\0'
#define UPATCH_UUID_LEN 37

typedef enum _patch_status {
	UPATCH_PATCH_STATUS_NOT_APPLIED = 1,
	UPATCH_PATCH_STATUS_DEACTIVED,
	UPATCH_PATCH_STATUS_ACTIVED,
	UPATCH_PATCH_STATUS_INV,
} patch_status_e;

typedef struct _patch_symbols {
	struct list_head self;

	char name[UPATCH_SYMBOL_NAME_MAX];
	loff_t offset;
} patch_symbols_t;

typedef struct _symbol_collision {
	struct list_head self;

	char uuid[UPATCH_UUID_LEN];
} symbol_collision;

// use by create and get
typedef struct _patch_entity {
	char target_path[PATH_MAX];
	char patch_path[PATH_MAX];
	patch_status_e status;
	struct list_head *symbols;
} patch_entity_t;

// 创建补丁管理结构
int meta_create_patch(const char *uuid, patch_entity_t *entity);

// 删除补丁管理结构
int meta_remove_patch(const char *uuid);

// 查找patch
int meta_get_patch_entity(const char *uuid, patch_entity_t *entity);

// 查找elf函数列表
struct list_head *meta_get_elf_symbols(const char *elf_path);

// 查找补丁函数列表
struct list_head *meta_get_patch_symbols(const char *uuid);

// 释放patch_symbols_t **类型返回内存
void meta_put_symbols(struct list_head *symbols);

// 获取补丁状态
patch_status_e meta_get_patch_status(const char *uuid);

// 设置补丁状态
int meta_set_patch_status(const char *uuid, patch_status_e status);

struct list_head *meta_get_symbol_collision(const char *elf_path, struct list_head *symbols);
void meta_put_symbol_collision(struct list_head *lst);
struct list_head *meta_patch_deactive_check(const char *uuid);

int meta_patch_init();
void meta_patch_fini();

#endif
