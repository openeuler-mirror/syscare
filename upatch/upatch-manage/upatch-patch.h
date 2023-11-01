// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Zongwu Li <lzw32321226@gmail.com>
 *
 */

#ifndef __UPATCH_PATCH__
#define __UPATCH_PATCH__

#include "upatch-elf.h"
#include "upatch-process.h"
#include "list.h"

int process_patch(int, struct upatch_elf *, struct running_elf *, const char *uuid, const char *binary_path);

int process_unpatch(int, const char *uuid);

int process_info(int);

#endif
