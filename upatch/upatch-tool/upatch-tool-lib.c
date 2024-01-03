#include "upatch-tool-lib.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <unistd.h>
#include <limits.h>
#include <errno.h>
#include <string.h>

#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>

#include "log.h"
#include "list.h"
#include "upatch-elf.h"
#include "upatch-meta.h"
#include "upatch-resolve.h"
#include "upatch-ioctl.h"

int upatch_check(const char *target_elf, const char *patch_file, char *err_msg, size_t max_len)
{
    int ret = 0;
    struct list_head *patch_syms = NULL;
    struct list_head *collision_list = NULL;

    struct upatch_elf uelf;
    ret = upatch_init(&uelf, patch_file);
	if (ret < 0) {
        snprintf(err_msg, max_len, "Failed to read patch");
		goto out;
	}

    struct running_elf relf;
	ret = binary_init(&relf, target_elf);
	if (ret < 0) {
        snprintf(err_msg, max_len, "Failed to read target elf");
		goto out;
	}
	uelf.relf = &relf;

    patch_syms = patch_symbols_resolve(&uelf, &relf);
    if (patch_syms == NULL) {
        snprintf(err_msg, max_len, "Patch format error");
        ret = ENOEXEC;
        goto out;
    }

    collision_list = meta_get_symbol_collision(target_elf, patch_syms);
    if (collision_list == NULL) {
        ret = 0;
        goto out;
    }

    int offset = snprintf(err_msg, max_len, "Patch is conflicted with ");
    symbol_collision *collision = NULL;
    list_for_each_entry(collision, collision_list, self) {
        err_msg += offset;
        max_len -= offset;
        offset = snprintf(err_msg, max_len, "\"%s\" ", collision->uuid);
    }
    ret = EEXIST;

out:
    if (patch_syms != NULL) {
        patch_symbols_free(patch_syms);
    }
    if (collision_list != NULL) {
        meta_put_symbol_collision(collision_list);
    }
	binary_close(&relf);
	upatch_close(&uelf);

    return ret;
}

int upatch_load(const char *uuid, const char *target, const char *patch, bool force)
{
    int ret = 0;
    struct list_head *patch_syms = NULL;
    patch_entity_t *patch_entity = NULL;
    struct list_head *collision_syms = NULL;

    // Pointer check
    if (uuid == NULL || target == NULL || patch == NULL) {
        return EINVAL;
    }
    log_normal("Loading patch {%s} (\"%s\") for \"%s\"\n", uuid, patch, target);

    struct upatch_elf uelf;
    ret = upatch_init(&uelf, patch);
	if (ret < 0) {
        log_warn("Failed to read patch\n");
		goto out;
	}

    struct running_elf relf;
	ret = binary_init(&relf, target);
	if (ret < 0) {
        log_warn("Failed to read target elf\n");
		goto out;
	}
	uelf.relf = &relf;

    // Fails if patch is already exist
    if (meta_get_patch_status(uuid) != UPATCH_PATCH_STATUS_NOT_APPLIED) {
        log_warn("{%s}: Patch status is invalid\n", uuid);
        ret = EPERM;
        goto out;
    }

    // Resolve patch symbols
    patch_syms = patch_symbols_resolve(&uelf, &relf);
    if (patch_syms == NULL) {
        log_warn("{%s}: Patch format error\n", uuid);
        ret = ENOEXEC;
        goto out;
    }

    // Check patch symbol collision
    if (!force) {
        collision_syms = meta_get_symbol_collision(target, patch_syms);
        if (collision_syms != NULL) {
            log_warn("{%s}: Patch symbol conflicted\n", uuid);
            ret = EEXIST;
            goto out;
        }
    }

    // Alloc memory for patch
    patch_entity = calloc(1, sizeof(patch_entity_t));
    if (patch_entity == NULL) {
        log_warn("{%s}: Failed to alloc memory\n", uuid);
        ret = ENOMEM;
        goto out;
    }

    strncpy(patch_entity->target_path, target, strnlen(target, PATH_MAX));
    strncpy(patch_entity->patch_path, patch, strnlen(patch, PATH_MAX));
    log_normal("target_path: %s, target: %s\n", target, patch_entity->target_path);
    log_normal("patch: %s, patch_path: %s\n", patch, patch_entity->patch_path);
    patch_entity->symbols = patch_syms;

    ret = meta_create_patch(uuid, patch_entity);
    if (ret != 0) {
        log_warn("{%s}: Failed to create patch entity\n", uuid);
        goto out;
    }

    meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_DEACTIVED);

out:
    if (collision_syms != NULL) {
        meta_put_symbol_collision(collision_syms);
    }
    if (patch_syms != NULL) {
        patch_symbols_free(patch_syms);
    }
    if (patch_entity != NULL) {
        free(patch_entity);
    }
	binary_close(&relf);
	upatch_close(&uelf);

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
    patch_status_e cur_status = meta_get_patch_status(uuid);
    if (cur_status != UPATCH_PATCH_STATUS_DEACTIVED) {
        log_warn("{%s}: Patch status is invalid\n", uuid);
        return EPERM;
    }

    // Set up patch status at first to check possible errors
    int ret = meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_NOT_APPLIED);
    if (ret != 0) {
        meta_set_patch_status(uuid, cur_status);
        return ret;
    }

    ret = meta_remove_patch(uuid);
    if (ret != 0) {
        log_warn("{%s}: Failed to remove patch\n", uuid);
        meta_set_patch_status(uuid, cur_status);
        return ret;
    }

    return 0;
}

int upatch_active(const char *uuid, const pid_t *pid_list, size_t list_len)
{
    int ret = 0;

    // Pointer check
    if (uuid == NULL) {
        return EINVAL;
    }
    log_normal("Activing patch {%s}", uuid);

    patch_status_e cur_status = meta_get_patch_status(uuid);
    // if (cur_status != UPATCH_PATCH_STATUS_DEACTIVED) {
    //     log_warn("{%s}: Patch status is invalid\n", uuid);
    //     return EPERM;
    // }

    // Set up patch status at first to check possible errors
    ret = meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_ACTIVED);
    if (ret != 0) {
        meta_set_patch_status(uuid, cur_status);
        return ret;
    }

    // Find patch entity
    patch_entity_t *patch_entity = calloc(1, sizeof(patch_entity_t));
    if (patch_entity == NULL) {
        log_warn("{%s}: Failed to alloc memory", uuid);
        meta_set_patch_status(uuid, cur_status);
        return ENOMEM;
    }

    ret = meta_get_patch_entity(uuid, patch_entity);
    if (ret != 0) {
        log_warn("{%s}: Cannot find patch entity", uuid);
        meta_set_patch_status(uuid, cur_status);
        free(patch_entity);
        return ENOENT;
    }

    // Find symbols in the patch
    if ((patch_entity->symbols == NULL) || list_empty(patch_entity->symbols)) {
        log_warn("{%s}: Patch symbol is empty", uuid);
        meta_set_patch_status(uuid, cur_status);
        free(patch_entity);
        return ENOENT;
    }

    // Apply patch
    for (size_t i = 0; i < list_len; i++) {
        char pid[16];
        char *pid_str = (char *) &pid;
        sprintf(pid_str, "%d", *(pid_list + i));

        char *patch_file = patch_entity->patch_path;
        char *target_elf = patch_entity->target_path;
        log_normal("{%s}: Apply patch \"%s\" for \"%s\" (%s)", uuid, patch_file, target_elf, pid_str);

        char *argv[] = {
            "/usr/libexec/syscare/upatch-manage",
            "patch",
            "--uuid",(char *) uuid,
            "--pid", pid_str,
            "--binary",
            target_elf,
            "--upatch",
            patch_file,
            "-v",
            NULL
        };

        pid_t child_pid = fork();
        if (child_pid == 0) {
            ret = execve("/usr/libexec/syscare/upatch-manage", argv, NULL);
            if (ret != 0) {
                log_warn("{%s}: Execve failed", uuid);
                meta_set_patch_status(uuid, cur_status);
                free(patch_entity);
                return ret;
            }
        } else if (child_pid > 0) {
            int status = 0;
            waitpid(child_pid, &status, 0);

            int exit_code = WEXITSTATUS(status);
            if (exit_code == EEXIST) {
                log_warn("{%s}: Patch already exists", uuid);
                continue;
            }
            if (exit_code != 0) {
                log_warn("{%s}: Patch failed", uuid);
                meta_set_patch_status(uuid, cur_status);
                free(patch_entity);
                return exit_code;
            }
        } else {
            log_warn("{%s}: Fork failed", uuid);
            meta_set_patch_status(uuid, cur_status);
            free(patch_entity);
            return child_pid;
        }
    }

    free(patch_entity);
    return 0;
}

int upatch_deactive(const char *uuid, const pid_t *pid_list, size_t list_len)
{
    int ret = 0;

    log_normal("Dectiving patch {%s}", uuid);

    // Pointer check
    if (uuid == NULL) {
        return EINVAL;
    }

    // Fails if patch is not in 'DEACTIVED' state
    patch_status_e cur_status = meta_get_patch_status(uuid);
    if (cur_status != UPATCH_PATCH_STATUS_ACTIVED) {
        log_warn("{%s}: Patch status is invalid", uuid);
        return EPERM;
    }

    // Set up patch status at first to check possible errors
    ret = meta_set_patch_status(uuid, UPATCH_PATCH_STATUS_DEACTIVED);
    if (ret != 0) {
        // Rollback status
        meta_set_patch_status(uuid, cur_status);
        return ret;
    }

    // Find patch entity
    patch_entity_t *patch_entity = calloc(1, sizeof(patch_entity_t));
    if (patch_entity == NULL) {
        log_warn("{%s}: Failed to alloc memory", uuid);
        meta_set_patch_status(uuid, cur_status);
        return ENOENT;
    }

    ret = meta_get_patch_entity(uuid, patch_entity);
    if (ret != 0) {
        log_warn("{%s}: Cannot find patch entity", uuid);
        meta_set_patch_status(uuid, cur_status);
        free(patch_entity);
        return ENOENT;
    }

    // Find symbols in the patch
    if (list_empty(patch_entity->symbols)) {
        log_warn("{%s}: Patch symbol is empty", uuid);
        meta_set_patch_status(uuid, cur_status);
        free(patch_entity);
        return ENOENT;
    }

    // Remove patch
    for (size_t i = 0; i < list_len; i++) {
        char pid[16];
        char *pid_str = (char *) &pid;
        sprintf(pid_str, "%d", *(pid_list + i));

        char *patch_file = patch_entity->patch_path;
        char *target_elf = patch_entity->target_path;
        log_normal("{%s}: Remove patch \"%s\" for \"%s\" (%s)", uuid, patch_file, target_elf, pid_str);

        char *argv[] = {
            "/usr/libexec/syscare/upatch-manage",
            "unpatch",
            "--uuid",(char *) uuid,
            "--pid", pid_str,
            "--binary",
            target_elf,
            "--upatch",
            patch_file,
            "-v",
            NULL
        };

        pid_t child_pid = fork();
        if (child_pid == 0) {
            ret = execve("/usr/libexec/syscare/upatch-manage", argv, NULL);
            if (ret != 0) {
                log_warn("{%s}: Execve failed", uuid);
                meta_set_patch_status(uuid, cur_status);
                free(patch_entity);
                return ret;
            }
        } else if (child_pid > 0) {
            int status = 0;
            waitpid(child_pid, &status, 0);
            if (status != 0) {
                log_warn("{%s}: Unpatch failed", uuid);
                meta_set_patch_status(uuid, cur_status);
                free(patch_entity);
                return status;
            }
        } else {
            log_warn("{%s}: Fork failed", uuid);
            meta_set_patch_status(uuid, cur_status);
            free(patch_entity);
            return child_pid;
        }
    }

    free(patch_entity);
    return 0;
}

patch_status_e upatch_status(const char *uuid)
{
    if (uuid == NULL) {
        return UPATCH_PATCH_STATUS_NOT_APPLIED;
    }
    return meta_get_patch_status(uuid);
}
