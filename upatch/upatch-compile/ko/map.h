// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
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
