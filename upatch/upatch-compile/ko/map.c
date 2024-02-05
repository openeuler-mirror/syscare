// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#include "map.h"

#include <linux/kref.h>
#include <linux/mutex.h>
#include <linux/slab.h>

#include "log.h"

struct map_entry {
    struct map *parent;
    void *value;
    struct kref ref;
};

struct map {
    struct mutex lock;
    size_t length;
    size_t capacity;
    const struct map_ops *ops;
    struct map_entry entries[];
};

/* Map private interface */
static inline void insert_entry(struct map_entry *entry, struct map *parent,
    void *value)
{
    pr_debug("insert map entry, map=0x%lx, index=%lu, value=0x%lx\n",
        (unsigned long)parent, (entry - &parent->entries[0]),
        (unsigned long)value);
    entry->parent = parent;
    entry->value = value;
    kref_init(&entry->ref);
}

static inline void remove_entry(struct map_entry *entry)
{
    struct map *parent = entry->parent;
    void *value = entry->value;

    if (value == NULL) {
        return;
    }

    pr_debug("remove map entry, map=0x%lx, index=%lu\n", (unsigned long)parent,
        (entry - &parent->entries[0]));
    entry->parent = NULL;
    entry->value = NULL;
    parent->ops->free_value(value);
}

static inline void release_entry(struct kref *kref)
{
    remove_entry(container_of_safe(kref, struct map_entry, ref));
}

static inline struct map_entry *lookup_entry(struct map *map, const void *param)
{
    struct map_entry *entry = NULL;
    size_t i = 0;

    for (i = 0; i < map->capacity; i++) {
        entry = &map->entries[i];
        if (entry->value == NULL) {
            continue;
        }
        if (map->ops->find_value(entry->value, param)) {
            return entry;
        }
    }

    return NULL;
}

static inline struct map_entry *lookup_free_entry(struct map *map)
{
    size_t i = 0;

    for (i = 0; i < map->capacity; i++) {
        if (map->entries[i].value == NULL) {
            return &map->entries[i];
        }
    }

    return NULL;
}

/* Map public interface */
int new_map(struct map **map, size_t capacity, const struct map_ops *ops)
{
    struct map *new_map = NULL;
    size_t map_size = 0;

    if ((map == NULL) || (capacity == 0) || (ops == NULL)) {
        return -EINVAL;
    }

    map_size += sizeof(struct map);
    map_size += sizeof(struct map_entry) * capacity;

    new_map = kzalloc(map_size, GFP_KERNEL);
    if (new_map == NULL) {
        return -ENOMEM;
    }

    mutex_init(&new_map->lock);
    new_map->ops = ops;
    new_map->capacity = capacity;

    *map = new_map;
    return 0;
}

void free_map(struct map *map)
{
    size_t capacity = 0;
    size_t i = 0;

    if (map == NULL) {
        return;
    }

    mutex_lock(&map->lock);

    capacity = map->capacity;
    map->length = 0;
    map->capacity = 0;
    for (i = 0; i < capacity; i++) {
        remove_entry(&map->entries[i]);
    }

    mutex_unlock(&map->lock);
    mutex_destroy(&map->lock);

    kfree(map);
}

int map_insert(struct map *map, void *value)
{
    struct map_entry *entry = NULL;

    if ((map == NULL) || (value == NULL)) {
        return -EINVAL;
    }

    /*
     * try to find the record
     * if found, increase refence
     * if not found, create a new entry
     */
    mutex_lock(&map->lock);

    entry = lookup_entry(map, value);
    if (entry != NULL) {
        mutex_unlock(&map->lock);
        kref_get(&entry->ref);
        return 0;
    }

    entry = lookup_free_entry(map);
    if (entry == NULL) {
        mutex_unlock(&map->lock);
        return -ENOBUFS;
    }

    insert_entry(entry, map, value);
    map->length++;

    mutex_unlock(&map->lock);

    return 0;
}

void map_remove(struct map *map, const void *param)
{
    struct map_entry *entry = NULL;

    if ((map == NULL) || (param == NULL)) {
        return;
    }

    mutex_lock(&map->lock);

    entry = lookup_entry(map, param);
    if (entry == NULL) {
        mutex_unlock(&map->lock);
        return;
    }

    // decrease reference and try to free
    if (kref_put(&entry->ref, release_entry)) {
        map->length--;
    };

    mutex_unlock(&map->lock);
}

void *map_get(struct map *map, const void *param)
{
    struct map_entry *entry = NULL;

    if ((map == NULL) || (param == NULL)) {
        return NULL;
    }

    mutex_lock(&map->lock);

    entry = lookup_entry(map, param);

    mutex_unlock(&map->lock);

    return (entry != NULL) ? entry->value : NULL;
}

size_t map_size(const struct map *map)
{
    return (map != NULL) ? map->length : 0;
}