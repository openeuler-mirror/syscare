#ifndef __UPATCH_TOOL_H_
#define __UPATCH_TOOL_H_

#include <stdbool.h>
#include "upatch-meta.h"

int upatch_check(const char *target_elf, const char *patch_file, char *err_msg, size_t max_len);
int upatch_load(const char *uuid, const char *target_elf, const char *patch_file, bool force);
int upatch_remove(const char *uuid);
int upatch_active(const char *uuid, const pid_t *pid_list, size_t list_len);
int upatch_deactive(const char *uuid, const pid_t *pid_list, size_t list_len);

patch_status_e upatch_status(const char *uuid);

#endif
