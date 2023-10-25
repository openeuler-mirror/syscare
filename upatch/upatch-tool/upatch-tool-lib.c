#include "upatch-tool-lib.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <limits.h>
#include <errno.h>
#include <string.h>

#include <sys/stat.h>
#include <sys/types.h>

#include "log.h"
#include "list.h"
#include "upatch-meta.h"
#include "upatch-resolve.h"
#include "upatch-ioctl.h"

int upatch_check(const char *target_elf, const char *patch_file, char *err_msg, size_t max_len)
{
    struct list_head *patch_syms = patch_symbols_resolve(target_elf, patch_file);
    if (patch_syms == NULL) {
        return ENOENT;
    }

    struct list_head *collision_list = meta_get_symbol_collision(target_elf, patch_syms);
    if (collision_list == NULL) {
        return 0;
    }

    int offset = snprintf(err_msg, max_len, "Upatch: Patch is conflicted with ");
    symbol_collision *collision = NULL;
    list_for_each_entry(collision, collision_list, self) {
        err_msg += offset;
        max_len -= offset;
        offset = snprintf(err_msg, max_len, "\"%s\" ", collision->uuid);
    }

    patch_symbols_free(patch_syms);
    meta_put_symbol_collision(collision_list);

    return EEXIST;
}

int upatch_load(const char *uuid, const char *target, const char *patch)
{
    // Pointer check
    if (uuid == NULL || target == NULL || patch == NULL) {
        return EINVAL;
    }
    log_normal("Loading patch {%s} (\"%s\") for \"%s\"\n", uuid, patch, target);

    // Fails if patch is already exist
    if (meta_get_patch_status(uuid) != UPATCH_PATCH_STATUS_NOT_APPLIED) {
        log_warn("{%s}: Patch status is invalid\n", uuid);
        return EPERM;
    }

    // Resolve patch symbols
    struct list_head *patch_syms = patch_symbols_resolve(target, patch);
    if (patch_syms == NULL) {
        log_warn("{%s}: Patch symbol is empty\n", uuid);
        return ENOENT;
    }

    // Check patch symbol collision
    struct list_head *collision_syms = meta_get_symbol_collision(target, patch_syms);
    if (collision_syms != NULL) {
        log_warn("{%s}: Patch symbol conflicted\n", uuid);
        patch_symbols_free(patch_syms);
        meta_put_symbol_collision(collision_syms);
        return EEXIST;
    }

    // Alloc memory for patch
    patch_entity_t *patch_entity = calloc(1, sizeof(patch_entity_t));
    if (patch_entity == NULL) {
        log_warn("{%s}: Failed to alloc memory\n", uuid);
        patch_symbols_free(patch_syms);
        return ENOMEM;
    }

    strncpy(patch_entity->target_path, target, strnlen(target, PATH_MAX));
    strncpy(patch_entity->patch_path, patch, strnlen(patch, PATH_MAX));
    log_normal("target_path: %s, target: %s\n", target, patch_entity->target_path);
    log_normal("patch: %s, patch_path: %s\n", patch, patch_entity->patch_path);
    patch_entity->symbols = patch_syms;

    int ret = meta_create_patch(uuid, patch_entity);
    if (ret != 0) {
        log_warn("{%s}: Failed to create patch entity\n", uuid);
        free(patch_entity);
        patch_symbols_free(patch_syms);
        return ret;
    }

    free(patch_entity);
    meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_DEACTIVED);

    log_normal("Patch {%s} status changed to %d", uuid, UPATCH_PATCH_STATUS_DEACTIVED);
    return ret;
}

int upatch_remove(const char *uuid)
{
    log_normal("Removing patch {%s}\n", uuid);

    // Pointer check
    if (uuid == NULL) {
        return EINVAL;
    }

    // Fails if patch is not in 'DEACTIVED' state
    if (meta_get_patch_status(uuid) != UPATCH_PATCH_STATUS_DEACTIVED) {
        log_warn("{%s}: Patch status is invalid\n", uuid);
        return EPERM;
    }

    meta_remove_patch(uuid);
    meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_NOT_APPLIED);

    log_normal("Patch {%s} status changed to %d", uuid, UPATCH_PATCH_STATUS_DEACTIVED);
    return 0;
}

int upatch_active(const char *uuid)
{
    int ret = 0;

    // Pointer check
    if (uuid == NULL) {
        return EINVAL;
    }
    log_normal("Activing patch {%s}\n", uuid);

    // Fails if patch is not in 'DEACTIVED' state
    if (meta_get_patch_status(uuid) != UPATCH_PATCH_STATUS_DEACTIVED) {
        log_warn("{%s}: Patch status is invalid\n", uuid);
        return EPERM;
    }

    // Find patch entity
    patch_entity_t *patch_entity = calloc(1, sizeof(patch_entity_t));
    if (patch_entity == NULL) {
        log_warn("{%s}: Failed to alloc memory\n", uuid);

        return ENOMEM;
    }

    ret = meta_get_patch_entity(uuid, patch_entity);
    if (ret != 0) {
        log_warn("{%s}: Cannot find patch entity\n", uuid);
        free(patch_entity);
        return ENOENT;
    }

    // Find symbols in the patch
    if ((patch_entity->symbols == NULL) || list_empty(patch_entity->symbols)) {
        log_warn("{%s}: Patch symbol is empty\n", uuid);
        free(patch_entity);
        return ENOENT;
    }

    // Apply a patch
    ret = patch_ioctl_apply(
        patch_entity->target_path,
        patch_entity->patch_path,
        patch_entity->symbols
    );
    if (ret != 0) {
        log_warn("{%s}: ioctl failed\n", uuid);
        free(patch_entity);
        return ret;
    }

    meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_ACTIVED);
    free(patch_entity);

    log_normal("Patch {%s} status changed to %d", uuid, UPATCH_PATCH_STATUS_ACTIVED);
    return 0;
}

int upatch_deactive(const char *uuid)
{
    int ret = 0;

    log_normal("Dectiving patch {%s}\n", uuid);

    // Pointer check
    if (uuid == NULL) {
        return EINVAL;
    }

    // Fails if patch is not in 'ACTIVED' state
    if (meta_get_patch_status(uuid) != UPATCH_PATCH_STATUS_ACTIVED) {
        return EPERM;
    }

    // Find patch entity
    patch_entity_t *patch_entity = calloc(1, sizeof(patch_entity_t));
    if (patch_entity == NULL) {
        log_warn("{%s}: Failed to alloc memory\n", uuid);
        return ENOENT;
    }

    ret = meta_get_patch_entity(uuid, patch_entity);
    if (ret != 0) {
        log_warn("{%s}: Cannot find patch entity\n", uuid);
        free(patch_entity);
        return ENOENT;
    }

    // Find symbols in the patch
    if (list_empty(patch_entity->symbols)) {
        log_warn("{%s}: Patch symbol is empty\n", uuid);
        free(patch_entity);
        return ENOENT;
    }

    // Remove a patch
    ret = patch_ioctl_remove(
        patch_entity->target_path,
        patch_entity->patch_path,
        patch_entity->symbols
    );
    if (ret != 0) {
        log_warn("{%s}: ioctl failed\n", uuid);
        free(patch_entity);
        return ret;
    }

    meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_DEACTIVED);
    free(patch_entity);

    log_normal("Patch {%s} status changed to %d", uuid, UPATCH_PATCH_STATUS_ACTIVED);
    return 0;
}

patch_status_e upatch_status(const char *uuid)
{
    if (uuid == NULL) {
        return UPATCH_PATCH_STATUS_NOT_APPLIED;
    }
    return meta_get_patch_status(uuid);
}
