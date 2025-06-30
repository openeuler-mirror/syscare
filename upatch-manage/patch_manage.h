// SPDX-License-Identifier: GPL-2.0
/*
 * provide kload kactive kdeactive kremove API to manage patch
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

#ifndef _UPATCH_MANAGE_PATCH_MANAGE_H
#define _UPATCH_MANAGE_PATCH_MANAGE_H

#include <linux/module.h>

struct inode;

enum upatch_status;
struct target_entity;

enum upatch_status upatch_status(const char *patch_file);

int upatch_load(const char *patch_file, const char *binary_file);

int upatch_remove(const char *patch_file);

int upatch_active(const char *patch_file);

int upatch_deactive(const char *patch_file);

void __exit report_global_table_populated(void);

#endif // _UPATCH_MANAGE_PATCH_MANAGE_H
