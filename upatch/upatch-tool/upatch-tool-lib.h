#ifndef __UPATCH_TOOL_H_
#define __UPATCH_TOOL_H_

#include "upatch-meta.h"

int upatch_check(const char *target_elf, const char *patch_file, char *err_msg, size_t max_len);
int upatch_load(const char *uuid, const char *target_elf, const char *patch_file);
int upatch_remove(const char *uuid);
int upatch_active(const char *uuid);
int upatch_deactive(const char *uuid);

patch_status_e upatch_status(const char *uuid);

#endif
