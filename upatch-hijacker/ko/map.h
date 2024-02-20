// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-hijacker kernel module
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

#ifndef _UPATCH_HIJACKER_KO_MAP_H
#define _UPATCH_HIJACKER_KO_MAP_H

#include <linux/types.h>
#include <stdbool.h>

typedef bool (*find_value_fn)(const void *value, const void *param);
typedef void (*free_value_fn)(void *value);

struct map_ops {
    find_value_fn find_value;
    free_value_fn free_value;
};
struct map;

int new_map(struct map **map, size_t capacity, const struct map_ops *ops);
void free_map(struct map *map);

int map_insert(struct map *map, void *value);
void map_remove(struct map *map, const void *param);
void *map_get(struct map *map, const void *param);
size_t map_size(const struct map *map);

#endif /* _UPATCH_HIJACKER_KO_MAP_H */
