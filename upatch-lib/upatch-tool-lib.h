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
